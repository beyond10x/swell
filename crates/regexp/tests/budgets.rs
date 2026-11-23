//! The budgets of the compile step and the matcher (`docs/design/engine.md`, decision 7, "How
//! `swell-regexp` charges its work", "What one step of the matcher is" and "No Rust recursion in
//! the parser and the compiler of a pattern"; decision 3, "The step budget" and "Depth
//! bounds"): a match spends exactly its units of work, by grants of at most 4,096 returned when
//! not run; a refused grant, a refused charge and an interrupt stop it; every byte it charged
//! is credited on every way out; the compile step is linear and neither step recurses.

use swell_regexp::{
    Budget, CompileError, Flags, MAX_GRANT, Pattern, Program, Stop, Subject, Unlimited,
    compile_pattern, match_at, parse_pattern, search, search_slot_visits,
};

#[cfg(test)]
fn units(text: &str) -> Vec<u16> {
    text.encode_utf16().collect()
}

#[cfg(test)]
fn tree(pattern: &[u16], flags: &str, nesting: u32) -> Pattern {
    let flags = Flags::parse(&units(flags)).expect("the flags");
    parse_pattern(pattern, flags, nesting, &mut Unlimited).expect("the tree")
}

#[cfg(test)]
fn program(pattern: &str, flags: &str) -> Program {
    compile_pattern(&tree(&units(pattern), flags, 1000), &mut Unlimited).expect("the program")
}

/// A budget that records what it is asked. It refuses a charge that would hold more than
/// `limit`, grants at most `steps` steps in all (then refuses), and answers `Interrupted` from
/// the grant numbered `interrupt_from` (from 1) on, and from the poll numbered
/// `poll_interrupt_from` on.
#[derive(Debug, Default)]
struct Recording {
    polls: usize,
    poll_interrupt_from: Option<usize>,
    charged: usize,
    credited: usize,
    high: usize,
    charges: usize,
    limit: Option<usize>,
    steps: Option<u64>,
    granted: u64,
    refunded: u64,
    grants: usize,
    largest: u32,
    refused: bool,
    asked_after_refusal: usize,
    interrupt_from: Option<usize>,
}

impl Recording {
    fn held(&self) -> usize {
        self.charged - self.credited
    }

    /// The steps spent: granted, net of the returns.
    fn spent(&self) -> u64 {
        self.granted - self.refunded
    }

    fn with_steps(steps: u64) -> Recording {
        Recording {
            steps: Some(steps),
            ..Recording::default()
        }
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
        assert!((1..=MAX_GRANT).contains(&asked), "a grant of {asked}");
        if self.refused {
            self.asked_after_refusal += 1;
        }
        self.grants += 1;
        self.largest = self.largest.max(asked);
        if self.interrupt_from.is_some_and(|from| self.grants >= from) {
            self.refused = true;
            return Err(Stop::Interrupted);
        }
        let given = match &mut self.steps {
            None => asked,
            Some(0) => {
                self.refused = true;
                return Err(Stop::StepBudget);
            }
            Some(left) => {
                let given = u64::from(asked).min(*left);
                *left -= given;
                given as u32
            }
        };
        self.granted += u64::from(given);
        Ok(given)
    }

    fn refund(&mut self, steps: u32) {
        assert!(steps > 0, "a return of nothing");
        self.refunded += u64::from(steps);
        assert!(
            self.refunded <= self.granted,
            "{steps} returned past the grants"
        );
        if let Some(left) = &mut self.steps {
            *left += u64::from(steps);
        }
    }

    fn poll(&mut self) -> Result<(), Stop> {
        self.polls += 1;
        if self
            .poll_interrupt_from
            .is_some_and(|from| self.polls >= from)
        {
            return Err(Stop::Interrupted);
        }
        Ok(())
    }
}

#[test]
fn catastrophic_backtracking_ends_at_the_step_budget() {
    let program = program("(a+)+$", "");
    let subject = units(&format!("{}!", "a".repeat(10_000)));
    let mut budget = Recording::with_steps(1_000_000);
    let got = search(&program, Subject::Utf16(&subject), 0, &mut budget);
    assert_eq!(got.err(), Some(Stop::StepBudget));
    // The refused grant is the step past the budget: every step granted was run, none
    // returned, and none asked after the refusal.
    assert_eq!(budget.spent(), 1_000_000);
    assert_eq!(budget.refunded, 0);
    assert_eq!(budget.asked_after_refusal, 0);
    assert_eq!(budget.held(), 0);
    assert!(budget.charges > 0);
}

#[test]
fn the_steps_of_a_match_are_exactly_its_units() {
    // /a/ in "ba": two instructions, the code unit and the end of the match; index 0 tried (1),
    // the code unit fails (1), index 1 tried (1), the code unit (1), the end (1).
    let mut budget = Recording::default();
    let found = search(
        &program("a", ""),
        Subject::Utf16(&units("ba")),
        0,
        &mut budget,
    )
    .expect("no stop")
    .expect("a match");
    assert_eq!(found.index, 1);
    assert_eq!(budget.spent(), 5);
    assert_eq!(budget.grants, 1);
    assert_eq!(budget.refunded, u64::from(MAX_GRANT) - 5);
    assert_eq!(program("a", "").len(), 2);
    // No match spends its units too and returns the rest: three tries (at 0, 1 and the end),
    // each one step and its code unit one.
    let mut budget = Recording::default();
    let found = search(
        &program("a", ""),
        Subject::Utf16(&units("bb")),
        0,
        &mut budget,
    );
    assert!(found.expect("no stop").is_none());
    assert_eq!(budget.spent(), 6);
    // A backreference of 1,000 code units is charged 1,000 steps for its comparison, beside
    // the one step of its instruction.
    let subject = units(&"a".repeat(2000));
    let spent = |pattern: &str| {
        let mut budget = Recording::default();
        let found = search(
            &program(pattern, ""),
            Subject::Utf16(&subject),
            0,
            &mut budget,
        )
        .expect("no stop")
        .expect("a match");
        assert!(budget.largest <= MAX_GRANT);
        assert_eq!(budget.held(), 0);
        (budget.spent(), found.captures.get(0))
    };
    let (without, whole) = spent("(a{1000})");
    assert_eq!(whole, Some(0..1000));
    let (with, whole) = spent("(a{1000})\\1");
    assert_eq!(whole, Some(0..2000));
    assert_eq!(with - without, 1 + 1000);
    // The try (1), the start of group 1 (1), the counter set (1), 1,000 iterations of three
    // instructions (the loop's head, the code unit, the loop's tail), the head that leaves
    // at the maximum (1), the end of group 1 (1), the end of the match (1).
    assert_eq!(without, 1 + 1 + 1 + 3 * 1000 + 1 + 1 + 1);
    // A backreference that needs more than one grant holds takes further grants first.
    let subject = units(&"a".repeat(20_000));
    let mut budget = Recording::default();
    let found = search(
        &program("(a{10000})\\1", ""),
        Subject::Utf16(&subject),
        0,
        &mut budget,
    )
    .expect("no stop")
    .expect("a match");
    assert_eq!(found.captures.get(0), Some(0..20_000));
    assert_eq!(
        budget.spent(),
        1 + 1 + 1 + 3 * 10_000 + 1 + 1 + 1 + 1 + 10_000
    );
    assert!(budget.largest <= MAX_GRANT);
    assert!(budget.refunded < u64::from(MAX_GRANT));
    assert_eq!(budget.asked_after_refusal, 0);
}

#[test]
fn a_refused_grant_stops_the_match_before_the_next_unit() {
    let program = program("a", "");
    let ba = units("ba");
    // Four steps run the tries at 0 and 1 and their code units; the end of the match is the
    // next unit, and its grant is refused.
    for (steps, answer) in [
        (0, Err(Stop::StepBudget)),
        (1, Err(Stop::StepBudget)),
        (4, Err(Stop::StepBudget)),
        (5, Ok(Some(1))),
    ] {
        let mut budget = Recording::with_steps(steps);
        let got = search(&program, Subject::Utf16(&ba), 0, &mut budget).map(|f| f.map(|f| f.index));
        assert_eq!(got, answer, "{steps} steps");
        assert_eq!(budget.spent(), steps, "{steps} steps");
        assert_eq!(budget.asked_after_refusal, 0);
        assert_eq!(budget.held(), 0);
    }
    // A long match stops at exactly its budget, at every size of the last grant.
    let program = self::program("(?:a|b)*c", "");
    let subject = units(&"ab".repeat(5000));
    for steps in [1, 4095, 4096, 4097, 10_000, 12_289] {
        let mut budget = Recording::with_steps(steps);
        let got = search(&program, Subject::Utf16(&subject), 0, &mut budget);
        assert_eq!(got.err(), Some(Stop::StepBudget), "{steps} steps");
        assert_eq!(budget.spent(), steps);
        assert_eq!(budget.refunded, 0);
        assert_eq!(budget.asked_after_refusal, 0);
        assert_eq!(budget.held(), 0);
    }
    // A backreference whose comparison passes the steps left stops before it compares.
    let program = self::program("(a{100})\\1", "");
    let subject = units(&"a".repeat(200));
    let mut budget = Recording::default();
    search(&program, Subject::Utf16(&subject), 0, &mut budget).expect("no stop");
    let all = budget.spent();
    for steps in [all - 100, all - 1] {
        let mut budget = Recording::with_steps(steps);
        let got = search(&program, Subject::Utf16(&subject), 0, &mut budget);
        assert_eq!(got.err(), Some(Stop::StepBudget), "{steps} of {all}");
        assert_eq!(budget.spent(), steps);
    }
    let mut budget = Recording::with_steps(all);
    assert!(search(&program, Subject::Utf16(&subject), 0, &mut budget).is_ok());
}

#[test]
fn a_backtrack_stack_past_the_memory_budget_stops_the_match() {
    let program = program("(?:a|b)*c", "");
    let subject = units(&"a".repeat(100_000));
    // Without a limit the try at index 0 holds more than 64 KiB at its high mark: measured
    // 12,582,944 bytes on this checkout, three entries of the backtrack stack per iteration
    // (the loop's exit, the second alternative, the counter's restore) for 100,000 iterations,
    // the stack's capacity doubled to 524,288 entries. 64 KiB is under 1 % of that.
    let mut budget = Recording::default();
    let one = match_at(&program, Subject::Utf16(&subject), 0, &mut budget);
    assert!(one.expect("no stop").is_none());
    assert!(budget.high > 64 * 1024, "{} bytes", budget.high);
    assert_eq!(budget.held(), 0);
    // With 64 KiB the backtrack stack is refused a growth and the match stops.
    let mut budget = Recording {
        limit: Some(64 * 1024),
        ..Recording::default()
    };
    let got = search(&program, Subject::Utf16(&subject), 0, &mut budget);
    assert_eq!(got.err(), Some(Stop::MemoryBudget));
    assert!(budget.high <= 64 * 1024);
    assert_eq!(budget.held(), 0);
    // The steps it ran are what it spent: what it was granted and did not run came back.
    assert!(budget.spent() > 0 && budget.spent() < 1_000_000);
}

#[test]
fn the_stack_and_the_captures_are_credited_on_every_way_out() {
    let program = program("(a)(?:(b)|c)*(?=(d))d\\1", "");
    // The high mark of the match, measured here (856 bytes on this checkout); one byte less
    // stops it at its last growth, and exactly that much lets it match.
    let mut probe = Recording::default();
    let found = search(&program, Subject::Utf16(&units("abcbcda")), 0, &mut probe);
    assert_eq!(found.expect("no stop").map(|f| f.index), Some(0));
    let high = probe.high;
    let cases: [(&str, Recording, Result<bool, Stop>); 6] = [
        ("abcbcda", Recording::default(), Ok(true)),
        ("abcbcdb", Recording::default(), Ok(false)),
        ("abcbcda", Recording::with_steps(10), Err(Stop::StepBudget)),
        (
            "abcbcda",
            Recording {
                limit: Some(high - 1),
                ..Recording::default()
            },
            Err(Stop::MemoryBudget),
        ),
        (
            "abcbcda",
            Recording {
                limit: Some(high),
                ..Recording::default()
            },
            Ok(true),
        ),
        (
            "abcbcda",
            Recording {
                interrupt_from: Some(1),
                ..Recording::default()
            },
            Err(Stop::Interrupted),
        ),
    ];
    for (subject, mut budget, answer) in cases {
        let got = search(&program, Subject::Utf16(&units(subject)), 0, &mut budget);
        assert_eq!(got.map(|f| f.is_some()), answer, "{subject}");
        assert_eq!(budget.held(), 0, "{subject}: {answer:?}");
        if answer != Err(Stop::Interrupted) {
            assert!(budget.charges > 0, "{subject}: {answer:?}");
        }
    }
    // The compile step: the program holds what the budget holds for it, by its capacities,
    // and its release credits it; a compile that stops credits what it charged.
    let tree = tree(&units("(a)(?:(b)|[c-e\\d])*(?=(d))\\1|x{2,5}"), "", 1000);
    let mut budget = Recording::default();
    let program = compile_pattern(&tree, &mut budget).expect("the program");
    assert_eq!(budget.held(), program.heap_bytes());
    assert_eq!(program.heap_bytes(), program.capacity_bytes());
    assert!(program.heap_bytes() > 0);
    let high = budget.high;
    program.release(&mut budget);
    assert_eq!(budget.held(), 0);
    for limit in (0..high).step_by(8) {
        let mut budget = Recording {
            limit: Some(limit),
            ..Recording::default()
        };
        let got = compile_pattern(&tree, &mut budget);
        assert_eq!(got.err(), Some(CompileError::Stopped(Stop::MemoryBudget)));
        assert_eq!(budget.held(), 0, "{limit}");
    }
    // The compile step charges no step.
    assert_eq!(budget.grants, 0);
}

#[test]
fn an_interrupt_stops_a_long_match() {
    let program = program("(a+)+$", "");
    let subject = units(&format!("{}!", "a".repeat(10_000)));
    // The interrupt is read at every grant: at most 4,096 units run between two readings.
    for from in [1, 2, 3, 20] {
        let mut budget = Recording {
            interrupt_from: Some(from),
            ..Recording::default()
        };
        let got = search(&program, Subject::Utf16(&subject), 0, &mut budget);
        assert_eq!(got.err(), Some(Stop::Interrupted));
        assert_eq!(budget.grants, from);
        assert_eq!(budget.spent(), (from as u64 - 1) * u64::from(MAX_GRANT));
        assert_eq!(budget.asked_after_refusal, 0);
        assert_eq!(budget.held(), 0);
    }
    // The compile step reads it at least once per 4,096 nodes it visits, and stops there with
    // everything credited.
    let tree = tree(&units(&"a".repeat(20_000)), "", 1000);
    let mut budget = Recording::default();
    let program = compile_pattern(&tree, &mut budget).expect("the program");
    assert!(budget.polls > 20_000 / 4096, "{} polls", budget.polls);
    program.release(&mut budget);
    for from in [1, 2, 5] {
        let mut budget = Recording {
            poll_interrupt_from: Some(from),
            ..Recording::default()
        };
        let got = compile_pattern(&tree, &mut budget);
        assert_eq!(got.err(), Some(CompileError::Stopped(Stop::Interrupted)));
        assert_eq!(budget.polls, from);
        assert_eq!(budget.held(), 0);
    }
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
fn the_compiler_and_the_matcher_use_no_rust_recursion() {
    on_a_small_stack(|| {
        // 10,000 nested groups of four kinds around a quantified class.
        const OPEN: [&str; 4] = ["(?=", "(?:", "(?-s:", "("];
        let mut text = String::new();
        for level in 0..10_000 {
            text.push_str(OPEN[level % OPEN.len()]);
        }
        text.push_str("[ab]+");
        text.push_str(&")".repeat(10_000));
        let tree = tree(&units(&text), "", 10_000);
        assert_eq!(tree.group_count(), 2500);
        let mut budget = Recording::default();
        let program = compile_pattern(&tree, &mut budget).expect("the program");
        let subject = units("xab");
        let found = search(&program, Subject::Utf16(&subject), 0, &mut budget)
            .expect("no stop")
            .expect("a match");
        assert_eq!(found.captures.get(0), Some(1..1));
        assert_eq!(found.captures.get(1), Some(1..1));
        assert_eq!(found.captures.get(2500), Some(1..3));
        program.release(&mut budget);
        assert_eq!(budget.held(), 0);
        // 10,000 nested quantified groups.
        let text = format!("{}a{}", "(?:".repeat(10_000), ")?".repeat(10_000));
        let program = compile_pattern(&self::tree(&units(&text), "", 10_000), &mut Unlimited)
            .expect("the program");
        let found = search(&program, Subject::Utf16(&units("a")), 0, &mut Unlimited)
            .expect("no stop")
            .expect("a match");
        assert_eq!(found.captures.get(0), Some(0..1));
        // `a*` against 1,000,000 code units.
        let subject = vec![u16::from(b'a'); 1_000_000];
        let found = search(
            &self::program("a*", ""),
            Subject::Utf16(&subject),
            0,
            &mut Unlimited,
        )
        .expect("no stop")
        .expect("a match");
        assert_eq!(found.captures.get(0), Some(0..1_000_000));
    });
}

/// The steps a search of `program` in `subject` from 0 spends, the slots of the captures it
/// visits in a scan or writes (`search_slot_visits`), and the range of group 0 it found.
#[cfg(test)]
fn work_of(program: &Program, subject: &[u16]) -> (u64, u64, Option<std::ops::Range<usize>>) {
    let mut budget = Recording::default();
    let found = search(program, Subject::Utf16(subject), 0, &mut budget).expect("no stop");
    assert_eq!(budget.held(), 0);
    let visits =
        search_slot_visits(program, Subject::Utf16(subject), 0, &mut Unlimited).expect("no stop");
    (
        budget.spent(),
        visits,
        found.and_then(|f| f.captures.get(0)),
    )
}

#[test]
fn a_try_does_no_work_per_group_it_does_not_charge() {
    // `x` then n groups that never take part, searched over 1,000 start indices that fail at
    // their first instruction: each try is its step and the code unit's, whatever n is, and
    // writes one slot (the start of group 0); the slots a try changed are put back by the
    // entries the backtrack stack takes, one step each.
    let subject = units(&"b".repeat(1000));
    let one = work_of(&program("x(a)", ""), &subject);
    let many = work_of(&program(&format!("x{}", "(a)".repeat(2000)), ""), &subject);
    assert_eq!(one, (2 * 1001, 1001, None));
    assert_eq!(many, one);
    // Groups that take part in a try that fails are unset in the next: by a lookahead that
    // keeps its captures, by a negative one that undoes them, and by an alternative.
    for (pattern, subject) in [
        ("(a)x|b", "ab"),
        ("(?=(a))x|b", "ab"),
        ("(?!(a)c)(a)x|b", "aab"),
        ("(?:(a)|(c))+x|b", "acab"),
    ] {
        let program = program(pattern, "");
        let found = search(&program, Subject::Utf16(&units(subject)), 0, &mut Unlimited)
            .expect("no stop")
            .expect("a match");
        let unset = found.captures.iter().skip(1).all(|group| group.is_none());
        assert!(unset, "/{pattern}/ in {subject}: {:?}", found.captures);
        let (steps, visits, _) = work_of(&program, &units(subject));
        assert!(
            visits <= 4 * steps,
            "/{pattern}/: {visits} visits, {steps} steps"
        );
    }
}

#[test]
fn the_clearing_of_a_quantified_atom_is_charged_per_group_of_its_range() {
    // An atom with n groups in an alternative that is never taken, repeated over 1,000 code
    // units: its clearing is one step per group of its range (its instruction's step the
    // first of them) at each of the 1,001 times it runs, the last where the atom fails.
    let subject = units(&"a".repeat(1000));
    let atom = |n: usize| program(&format!("(?:a|x{})*", "(y)".repeat(n)), "");
    let (one, one_visits, whole) = work_of(&atom(1), &subject);
    assert_eq!(whole, Some(0..1000));
    let (many, many_visits, whole) = work_of(&atom(2000), &subject);
    assert_eq!(whole, Some(0..1000));
    assert_eq!(many - one, 1999 * 1001, "{one} and {many} steps");
    assert!(one_visits <= 4 * one, "{one_visits} visits, {one} steps");
    assert!(
        many_visits <= 4 * many,
        "{many_visits} visits, {many} steps"
    );
    // The example of the design: `/(?:a|(b)(c))*/` in "a" spends 20 steps, 18 of them as
    // before and 2 for the second group of each of its two clearings.
    assert_eq!(work_of(&program("(?:a|(b)(c))*", ""), &units("a")).0, 20);
    // The steps are charged before the scan: a budget that ends inside a clearing stops the
    // match with exactly its steps spent.
    let program = atom(2000);
    for steps in [10, 2000, 4097, many - 1] {
        let mut budget = Recording::with_steps(steps);
        let got = search(&program, Subject::Utf16(&subject), 0, &mut budget);
        assert_eq!(got.err(), Some(Stop::StepBudget), "{steps} steps");
        assert_eq!(budget.spent(), steps);
        assert_eq!(budget.asked_after_refusal, 0);
        assert_eq!(budget.held(), 0);
    }
}

#[test]
fn a_named_backreference_is_charged_per_group_of_its_name() {
    // `\k<n>` of a name n groups have, none of them set: one step per group of its name (its
    // instruction's step the first of them), beside the same pattern without it.
    let subject = units("b");
    let cost = |n: usize| {
        let groups = vec!["(?<n>a)"; n].join("|");
        let without = work_of(&program(&format!("(?:{groups})?b"), ""), &subject);
        let with = work_of(&program(&format!("(?:{groups})?\\k<n>b"), ""), &subject);
        assert_eq!(
            (without.2.clone(), with.2.clone()),
            (Some(0..1), Some(0..1))
        );
        assert!(with.1 <= 4 * with.0, "{} visits, {} steps", with.1, with.0);
        with.0 - without.0
    };
    assert_eq!(cost(2), 2);
    assert_eq!(cost(2000), 2000);
}

#[test]
fn the_compile_step_is_linear() {
    // A counter, not n copies: `a{1000000}` is the counter set, the loop's head, the code
    // unit, the loop's tail and the end of the match.
    assert_eq!(program("a{1000000}", "").len(), 5);
    assert_eq!(program("(?:ab|c){5,1000000}?", "").len(), 9);
    // A bounded number of instructions per node.
    for pattern in [
        "(a|b)*".repeat(1000),
        "(?<=x(?!y)[a-z\\d]{2,})\\1".repeat(1000),
        format!("({})", "a|".repeat(5000)),
        "[a-zA-Z0-9_$\\s]+?".repeat(1000),
    ] {
        let tree = tree(&units(&pattern), "", 1000);
        let program = compile_pattern(&tree, &mut Unlimited).expect("the program");
        assert!(
            program.len() <= 4 * tree.nodes().len() + 1,
            "{} instructions for {} nodes",
            program.len(),
            tree.nodes().len()
        );
    }
}
