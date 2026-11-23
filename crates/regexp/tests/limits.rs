//! The bounds of the parser (`docs/design/engine.md`, decision 7, "How `swell-regexp` charges
//! its work", "What one step of the matcher is" and "No Rust recursion in the parser and the
//! compiler of a pattern"): the nesting limit counted from zero, no recursion on a small
//! stack, every growth charged and credited, no step asked for, a refused charge and an
//! interrupt that stop the parse with everything credited, and work linear in the pattern.

use swell_regexp::{
    Budget, Flags, MAX_GRANT, POLL_INTERVAL, ParseError, Pattern, Stop, Unlimited,
    parse_name_visits, parse_pattern,
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

/// The work of one parse as the crate shows it: the polls of a counting budget (one before the
/// pattern is read, then one per 4,096 code units read and per 4,096 nodes made or visited),
/// the bytes it charged, and the visits of the group-name check (`parse_name_visits`: a name
/// declared or looked up, a level its union-find walked). Nothing is held after it.
#[cfg(test)]
fn work_of(family: &str, pattern: &[u16], f: &str) -> [usize; 3] {
    let mut budget = Recording::default();
    let visits = parse_name_visits(pattern, flags(f), u32::MAX, &mut budget)
        .unwrap_or_else(|e| panic!("{family}: {e:?}"));
    assert_eq!(budget.held(), 0, "{family}");
    let visits = usize::try_from(visits).expect("a count");
    [budget.polls, budget.charged, visits]
}

/// A family of patterns: what it repeats, its flags, whether it has group names, and its
/// pattern for a count k, which grows in proportion to k.
#[cfg(test)]
type Family = (&'static str, &'static str, bool, fn(usize) -> String);

/// The families of the constructs where a pattern parser is commonly quadratic.
#[cfg(test)]
fn families() -> Vec<Family> {
    vec![
        ("a name declared in two alternatives", "u", true, |k| {
            (0..k)
                .map(|i| format!("(?:(?<g{i:06}>a)|(?<g{i:06}>b))"))
                .collect()
        }),
        (
            "names declared again past a deep closed nesting",
            "u",
            true,
            |k| {
                let names: String = (0..k).map(|i| format!("(?<g{i:06}>a)")).collect();
                format!("(?:{}{names}{}|{names})", "(".repeat(k), ")".repeat(k))
            },
        ),
        ("a group and a name of its own referenced", "u", true, |k| {
            (0..k)
                .map(|i| format!("(?<g{i:06}>a)\\k<g{i:06}>"))
                .collect()
        }),
        ("named backreferences", "u", true, |k| {
            format!("(?<g>a){}", "\\k<g>".repeat(k))
        }),
        (
            "named backreferences before the group, without u",
            "",
            true,
            |k| format!("{}(?<g>a)", "\\k<g>".repeat(k)),
        ),
        ("a long name and its backreferences", "u", true, |k| {
            let name = "n".repeat(500);
            format!("(?<{name}>a){}", format!("\\k<{name}>").repeat(k))
        }),
        ("groups and decimal escapes, without u", "", false, |k| {
            format!("{}{}", "(a)".repeat(k), "\\1".repeat(k))
        }),
        (
            "decimal escapes past the groups, without u",
            "",
            false,
            |k| "\\8\\18\\400".repeat(k),
        ),
        ("decimal escapes and a group, with u", "u", false, |k| {
            format!("{}(a)", "\\1".repeat(k))
        }),
        ("the digits after a backslash, without u", "", false, |k| {
            format!("\\{}", "1".repeat(k))
        }),
        ("the ranges of one class", "u", false, |k| {
            format!("[{}]", "a-z".repeat(k))
        }),
        ("classes", "u", false, |k| "[^a-z\\d_-]".repeat(k)),
        (
            "property escapes, outside a class and in one",
            "u",
            false,
            |k| "\\p{L}\\P{sc=Greek}[\\p{scx=Latin}\\P{Changes_When_NFKC_Casefolded}a-z]".repeat(k),
        ),
        ("braced quantifiers", "u", false, |k| "a{1,2}".repeat(k)),
        (
            "the digits of a braced quantifier left open, without u",
            "",
            false,
            |k| format!("a{{1,{}", "2".repeat(k)),
        ),
        (
            "braces that start no quantifier, without u",
            "",
            false,
            |k| "a{1,2".repeat(k),
        ),
        ("nesting", "u", false, |k| {
            format!("{}a{}", "(".repeat(k), ")".repeat(k))
        }),
        (
            "nesting of every kind of level, without u",
            "",
            false,
            |k| {
                let open = ["(", "(?:", "(?=", "(?<!", "(?i:", "(?-s:"];
                let opened: String = open.iter().cycle().take(k).copied().collect();
                format!("{opened}a{}", ")".repeat(k))
            },
        ),
        ("alternatives", "", false, |k| "a|".repeat(k)),
        ("lookarounds and modifier groups", "", false, |k| {
            "(?i-ms:a)(?<=b)(?<!c)(?=d)".repeat(k)
        }),
        (
            "classes of wide ranges, characters and backreferences under i",
            "iu",
            false,
            |k| "(?-i:[\\x80-\\uffff]k)[^a-z\\w\\p{Lu}](s)\\1".repeat(k),
        ),
    ]
}

#[test]
fn the_parse_is_linear_in_the_pattern() {
    // Each family at a count k of about 12,000 code units and at 4k: the pattern grows 4 times,
    // and each measure of the work grows between 3 and 5 times, within 2 for the polls, which
    // are whole (the vectors grow by doubling). A parse that does work per earlier name, group,
    // reference, digit or level for one of them grows a measure about 16 times.
    const UNITS: usize = 12_000;
    for (family, f, named, pattern) in families() {
        let len = |k: usize| pattern(k).encode_utf16().count();
        let growth = len(2) - len(1);
        let k = UNITS.saturating_sub(len(1)) / growth + 1;
        let (small, large) = (units(&pattern(k)), units(&pattern(4 * k)));
        let sizes = (small.len(), large.len());
        assert!(
            3 * sizes.0 <= sizes.1 && sizes.1 <= 5 * sizes.0,
            "{family}: {sizes:?}"
        );
        let (a, b) = (work_of(family, &small, f), work_of(family, &large, f));
        for (what, (a, b)) in ["polls", "bytes charged", "name visits"]
            .into_iter()
            .zip(a.into_iter().zip(b))
        {
            assert!(
                3 * a <= b + 2 && b <= 5 * a + 2,
                "{family}: {what} {a} at {} code units and {b} at {}",
                sizes.0,
                sizes.1
            );
        }
        assert_eq!(a[2] > 0, named, "{family}: name visits {}", a[2]);
    }
}
