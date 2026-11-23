//! The bounds of the parser (`docs/design/engine.md`, decision 7, "How `swell-regexp` charges
//! its work", "What one step of the matcher is" and "No Rust recursion in the parser and the
//! compiler of a pattern"): the nesting limit counted from zero, no recursion on a small
//! stack, every growth charged and credited, no step asked for, a refused charge and an
//! interrupt that stop the parse with everything credited.

use swell_regexp::{
    Budget, Flags, MAX_GRANT, POLL_INTERVAL, ParseError, Pattern, Stop, Unlimited, parse_pattern,
};

#[cfg(test)]
fn units(text: &str) -> Vec<u16> {
    text.encode_utf16().collect()
}

#[cfg(test)]
fn flags(text: &str) -> Flags {
    Flags::parse(&units(text)).expect("the flags")
}

/// A budget that records what it is asked: it refuses a charge that would hold more than
/// `limit`, and answers `Interrupted` from the poll numbered `interrupt_from` (from 1) on.
#[derive(Debug, Default)]
struct Recording {
    charged: usize,
    credited: usize,
    high: usize,
    charges: usize,
    limit: Option<usize>,
    polls: usize,
    interrupt_from: Option<usize>,
    grants: usize,
    refunds: usize,
}

impl Recording {
    fn held(&self) -> usize {
        self.charged - self.credited
    }
}

impl Budget for Recording {
    fn charge(&mut self, bytes: usize) -> Result<(), Stop> {
        assert!(bytes > 0, "a charge of nothing");
        let held = self.held() + bytes;
        if self.limit.is_some_and(|limit| held > limit) {
            return Err(Stop::MemoryBudget);
        }
        self.charged += bytes;
        self.charges += 1;
        self.high = self.high.max(held);
        Ok(())
    }

    fn credit(&mut self, bytes: usize) {
        self.credited += bytes;
        assert!(
            self.credited <= self.charged,
            "{bytes} credited past the charges"
        );
    }

    fn grant(&mut self, asked: u32) -> Result<u32, Stop> {
        self.grants += 1;
        Ok(asked.clamp(1, MAX_GRANT))
    }

    fn refund(&mut self, _steps: u32) {
        self.refunds += 1;
    }

    fn poll(&mut self) -> Result<(), Stop> {
        self.polls += 1;
        if self.interrupt_from.is_some_and(|from| self.polls >= from) {
            return Err(Stop::Interrupted);
        }
        Ok(())
    }
}

/// `depth` groups, each of one of the kinds that open a level, around `a`.
#[cfg(test)]
fn nested(depth: usize) -> Vec<u16> {
    const OPEN: [&str; 6] = ["(", "(?:", "(?=", "(?<!", "(?i:", "(?-s:"];
    let mut text = String::new();
    for level in 0..depth {
        text.push_str(OPEN[level % OPEN.len()]);
    }
    text.push('a');
    for _ in 0..depth {
        text.push(')');
    }
    units(&text)
}

/// Patterns that make every kind of node and list of the tree.
const PATTERNS: &[(&str, &str)] = &[
    ("", ""),
    ("a", ""),
    ("^(?<year>\\d{4})-(?<month>\\d\\d)|(?<year>x)$", ""),
    (
        "(?:a|b|c)*?[^a-z\\d\\-_]+(?=x)(?!y)(?<=z)(?<!w)\\b\\B.",
        "u",
    ),
    ("(a)(b)\\2\\1\\k<n>(?<n>c){2,5}(?i-m:d)(?s:e)", ""),
    ("{}]\\c\\1\\08[\\c_\\d-a]\\k\\u{3}x{1,}", ""),
    ("\\u{1F600}\\uD83D\\uDE00[\u{1F600}-\u{1F602}]", "u"),
    ("[0-9A-Fa-f]{1,8}(?:[.,][0-9]+)?|x+y*z?", ""),
];

#[test]
fn a_pattern_one_level_past_the_nesting_limit_is_the_nesting_error() {
    for f in ["", "u"] {
        let at = parse_pattern(&nested(1000), flags(f), 1000, &mut Unlimited);
        assert!(
            at.is_ok(),
            "1,000 levels at a limit of 1,000, flags {f:?}: {at:?}"
        );
        match parse_pattern(&nested(1001), flags(f), 1000, &mut Unlimited) {
            Err(ParseError::Pattern(e)) => {
                assert_eq!(e.message(), "nesting deeper than parser_nesting (1000)");
                assert!(!e.is_refusal());
                let before: usize = (0..1000).map(|level| [1, 3, 3, 4, 4, 5][level % 6]).sum();
                assert_eq!(e.offset, before, "the offset of the 1,001st level");
            }
            other => panic!("1,001 levels at a limit of 1,000, flags {f:?}: {other:?}"),
        }
    }
    // Counted from zero at the start of the pattern: one level at a limit of 0 is too deep.
    assert!(parse_pattern(&units("a"), flags(""), 0, &mut Unlimited).is_ok());
    assert!(parse_pattern(&units("[(]"), flags(""), 0, &mut Unlimited).is_ok());
    match parse_pattern(&units("(a)"), flags(""), 0, &mut Unlimited) {
        Err(ParseError::Pattern(e)) => {
            assert_eq!(e.message(), "nesting deeper than parser_nesting (0)");
        }
        other => panic!("{other:?}"),
    }
    // Siblings are no nesting.
    let siblings = units(&"(a)".repeat(2000));
    assert!(parse_pattern(&siblings, flags(""), 1, &mut Unlimited).is_ok());
}

/// Runs `body` on a thread of 64 KiB.
#[cfg(test)]
fn on_a_small_stack(body: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(64 * 1024)
        .spawn(body)
        .expect("a thread")
        .join()
        .expect("the thread ends");
}

#[test]
fn a_pattern_at_the_nesting_limit_parses_on_a_small_stack() {
    on_a_small_stack(|| {
        for f in ["", "u"] {
            let tree = parse_pattern(&nested(10_000), flags(f), 10_000, &mut Unlimited);
            let tree = tree.expect("10,000 levels at a limit of 10,000");
            assert_eq!(tree.group_count(), 1667);
        }
    });
}

#[test]
fn dropping_a_deep_tree_does_not_recurse() {
    on_a_small_stack(|| {
        let mut budget = Recording::default();
        let tree: Pattern =
            parse_pattern(&nested(10_000), flags(""), 10_000, &mut budget).expect("10,000 levels");
        assert!(tree.nodes().len() > 20_000);
        tree.release(&mut budget);
        assert_eq!(budget.held(), 0);
        let tree = parse_pattern(&nested(10_000), flags("u"), 10_000, &mut Unlimited)
            .expect("10,000 levels");
        drop(tree);
    });
}

#[test]
fn every_node_is_charged_before_it_grows_and_credited_when_dropped() {
    for &(pattern, f) in PATTERNS {
        let mut budget = Recording::default();
        let tree = parse_pattern(&units(pattern), flags(f), 1000, &mut budget)
            .unwrap_or_else(|e| panic!("/{pattern}/{f}: {e:?}"));
        assert_eq!(
            budget.held(),
            tree.charged(),
            "/{pattern}/{f}: held against the tree"
        );
        assert_eq!(
            tree.charged(),
            tree.capacity_bytes(),
            "/{pattern}/{f}: the charge against the capacities"
        );
        assert!(tree.charged() > 0 || pattern.is_empty(), "/{pattern}/{f}");
        tree.release(&mut budget);
        assert_eq!(budget.held(), 0, "/{pattern}/{f}: after the release");
    }
    // A long pattern: the vectors grew many times, each growth charged first.
    let long = units(&"(?:ab|[c-e]+)".repeat(500));
    let mut budget = Recording::default();
    let tree = parse_pattern(&long, flags(""), 1000, &mut budget).expect("a tree");
    assert_eq!(budget.held(), tree.capacity_bytes());
    assert!(budget.charges > 20, "{} charges", budget.charges);
    tree.release(&mut budget);
    assert_eq!(budget.held(), 0);
}

#[test]
fn the_parser_asks_for_no_step() {
    for &(pattern, f) in PATTERNS {
        let mut budget = Recording::default();
        let tree = parse_pattern(&units(pattern), flags(f), 1000, &mut budget).expect("a tree");
        tree.release(&mut budget);
        let _ = parse_pattern(&units("(a"), flags(f), 1000, &mut budget);
        assert_eq!((budget.grants, budget.refunds), (0, 0), "/{pattern}/{f}");
    }
}

#[test]
fn a_refused_charge_stops_the_parse_and_credits_what_it_holds() {
    for &(pattern, f) in PATTERNS {
        let source = units(pattern);
        let mut budget = Recording::default();
        let tree = parse_pattern(&source, flags(f), 1000, &mut budget).expect("a tree");
        tree.release(&mut budget);
        let high = budget.high;
        // Every limit below the high mark stops the parse at some charge.
        for limit in (0..high).step_by(8) {
            let mut budget = Recording {
                limit: Some(limit),
                ..Recording::default()
            };
            let parsed = parse_pattern(&source, flags(f), 1000, &mut budget);
            assert_eq!(
                parsed.err(),
                Some(ParseError::Stopped(Stop::MemoryBudget)),
                "/{pattern}/{f} at {limit} of {high}"
            );
            assert_eq!(budget.held(), 0, "/{pattern}/{f} at {limit}");
        }
    }
    // A pattern whose error lies at its end answers the stop, never the error.
    let source = units(&format!("{}(", "a".repeat(5000)));
    let mut budget = Recording::default();
    assert!(matches!(
        parse_pattern(&source, flags(""), 1000, &mut budget),
        Err(ParseError::Pattern(_))
    ));
    assert_eq!(budget.held(), 0);
    let mut budget = Recording {
        limit: Some(budget.high / 2),
        ..Recording::default()
    };
    assert_eq!(
        parse_pattern(&source, flags(""), 1000, &mut budget).err(),
        Some(ParseError::Stopped(Stop::MemoryBudget))
    );
    assert!(budget.charged > 0);
    assert_eq!(budget.held(), 0);
}

#[test]
fn an_interrupt_stops_the_parse_within_4096_code_units() {
    // Polled at least once per 4,096 code units read: a pattern of e code units whose error
    // lies at its end is answered after e / 4,096 polls or more.
    // A class of e code units is one node, so its polls are those of the code units read.
    for e in [1, 4095, 4096, 4097, 8192, 12_289, 300_000] {
        for f in ["", "u"] {
            let terms = units(&format!("{})", "a".repeat(e)));
            let class = units(&format!("[{}])", "a".repeat(e)));
            for source in [terms, class] {
                let mut budget = Recording::default();
                let parsed = parse_pattern(&source, flags(f), 1000, &mut budget);
                assert!(matches!(parsed, Err(ParseError::Pattern(_))), "{e}");
                assert!(
                    budget.polls > e / POLL_INTERVAL,
                    "{} polls for {e} code units",
                    budget.polls
                );
            }
        }
    }
    // An interrupt read at the first poll, before a code unit is read, at the second, after
    // 4,096 code units at most, and at the twentieth: the parse answers the stop, not the error
    // at the end, polls no more, and holds nothing.
    let source = units(&format!("{})", "a".repeat(300_000)));
    for from in [1, 2, 20] {
        for f in ["", "u"] {
            let mut budget = Recording {
                interrupt_from: Some(from),
                ..Recording::default()
            };
            let parsed = parse_pattern(&source, flags(f), 1000, &mut budget);
            assert_eq!(parsed.err(), Some(ParseError::Stopped(Stop::Interrupted)));
            assert_eq!(budget.polls, from);
            assert_eq!(budget.held(), 0);
        }
    }
    // An interrupt set from the second poll on stops a pattern whose error lies at code unit
    // 4,096: the second poll came before the parser read past it.
    let source = units(&format!("{})", "a".repeat(POLL_INTERVAL)));
    let mut budget = Recording {
        interrupt_from: Some(2),
        ..Recording::default()
    };
    assert_eq!(
        parse_pattern(&source, flags(""), 1000, &mut budget).err(),
        Some(ParseError::Stopped(Stop::Interrupted))
    );
}
