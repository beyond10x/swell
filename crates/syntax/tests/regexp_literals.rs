//! The pattern of a regular expression literal checked at parse
//! (`#sec-primary-expression-regular-expression-literals-static-semantics-early-errors`;
//! `docs/design/engine.md`, decision 7, "How `swell-regexp` charges its work", item 2, and "A
//! regular expression literal: checked at parse time, compiled at each evaluation", rule 1):
//! the early errors run ParsePattern of `swell-regexp` with the meter of the parse, so the tree
//! of a pattern is charged to the ledger and credited when the check ends, the interrupt is read
//! inside a long pattern, a grant of steps is answered in full and counted nowhere, and an
//! early error of a pattern comes before the compile step's refusal of `i`.
#![cfg(test)]

use swell_regexp::{Budget, Flags, MAX_GRANT, Unlimited as NoBudget, parse_pattern};
use swell_syntax::ledger::{Ledger, Meter, POLL_INTERVAL, Stop};
use swell_syntax::regexp::PatternBudget;
use swell_syntax::{
    Failure, MessageKind, ParseOptions, message_kind, parse_script, parse_script_with,
};

const OPTIONS: ParseOptions = ParseOptions { max_nesting: 1_000 };

fn units(text: &str) -> Vec<u16> {
    text.encode_utf16().collect()
}

/// The script `x = /<pattern>/<flags>;`.
fn literal(pattern: &str, flags: &str) -> Vec<u16> {
    units(&format!("x = /{pattern}/{flags};"))
}

/// A ledger that records what it is asked and refuses nothing, but for an interrupt at one poll.
#[derive(Debug, Default)]
struct Recording {
    charged: usize,
    credited: usize,
    polls: usize,
    /// The poll, counted from 1, that answers `Interrupted`.
    interrupt_at: Option<usize>,
}

impl Recording {
    fn held(&self) -> usize {
        self.charged.saturating_sub(self.credited)
    }
}

impl Ledger for Recording {
    fn charge(&mut self, bytes: usize) -> Result<(), Stop> {
        self.charged += bytes;
        Ok(())
    }

    fn credit(&mut self, bytes: usize) {
        self.credited += bytes;
    }

    fn poll(&mut self) -> Result<(), Stop> {
        self.polls += 1;
        if self.interrupt_at == Some(self.polls) {
            return Err(Stop::Interrupted);
        }
        Ok(())
    }
}

/// A pattern of 6,800 code units: 400 times a non-capturing group of two alternatives, a class,
/// an escape and a capturing group, about seven nodes per 17 code units.
fn long_pattern() -> String {
    "(?:a|b)[c-e]\\d(f)g".repeat(400)
}

/// The bytes `swell-regexp` holds for the tree of `pattern` without flags: what the check of a
/// literal charges for its tree, besides the scratch of its early errors.
fn tree_bytes(pattern: &str) -> usize {
    parse_pattern(&units(pattern), Flags::default(), 1_000, &mut NoBudget)
        .unwrap()
        .charged()
}

/// The parse of a literal of a long pattern charges more than one of the empty pattern (written
/// `(?:)`, as `source` writes it), by at least the bytes of the long pattern's tree; after the
/// parse the ledger holds exactly the script's tree, as a walk of it sums its bytes, so nothing
/// of the pattern's tree stays charged.
#[test]
fn the_pattern_of_a_literal_is_charged_and_credited() {
    let charged = |pattern: &str| {
        let mut ledger = Recording::default();
        let script = parse_script_with(&literal(pattern, ""), OPTIONS, &mut ledger)
            .unwrap_or_else(|failure| panic!("{failure:?}"));
        assert_eq!(ledger.held(), script.charged, "{ledger:?}");
        assert_eq!(script.charged, script.capacity_bytes(), "{ledger:?}");
        ledger.charged
    };
    let long = long_pattern();
    let (long_tree, empty_tree) = (tree_bytes(&long), tree_bytes("(?:)"));
    let (long_charged, empty_charged) = (charged(&long), charged("(?:)"));
    println!(
        "tree bytes {long_tree} and {empty_tree}; charged by the parse {long_charged} and \
         {empty_charged}"
    );
    assert!(long_tree > empty_tree);
    assert!(
        long_charged.saturating_sub(empty_charged) >= long_tree - empty_tree,
        "the parse charged {long_charged} and {empty_charged}, the trees are {long_tree} and \
         {empty_tree} bytes"
    );
}

/// A literal of 100,000 code units makes the parse poll at least once more per 4,096 of them
/// than a string literal as long; an interrupt at the last poll of that parse, which
/// ParsePattern makes (the lexer is done by then, and the walks visit fewer than 4,096 nodes),
/// ends the parse in `Stopped(Interrupted)` with everything credited.
#[test]
fn an_interrupt_stops_the_parse_inside_a_long_pattern() {
    let length = 100_000;
    let body = "a".repeat(length);
    let regexp = literal(&body, "");
    let string = units(&format!("x = \"{body}\";"));
    let polls = |source: &[u16]| {
        let mut ledger = Recording::default();
        let parsed = parse_script_with(source, OPTIONS, &mut ledger);
        assert!(parsed.is_ok(), "{:?}", parsed.err());
        ledger.polls
    };
    let (of_regexp, of_string) = (polls(&regexp), polls(&string));
    println!("polls: {of_regexp} for the literal, {of_string} for the string");
    assert!(
        of_regexp >= of_string + length / POLL_INTERVAL,
        "{of_regexp} polls for the literal, {of_string} for the string"
    );
    let mut ledger = Recording {
        interrupt_at: Some(of_regexp),
        ..Recording::default()
    };
    let parsed = parse_script_with(&regexp, OPTIONS, &mut ledger);
    assert_eq!(parsed.err(), Some(Failure::Stopped(Stop::Interrupted)));
    assert_eq!(ledger.polls, of_regexp);
    assert!(ledger.charged > 0);
    assert_eq!(ledger.held(), 0, "{ledger:?}");
}

/// A parse spends no step (decision 3, "The step budget"), and the budget a literal's pattern is
/// parsed with counts none: every grant is answered in full, from 1 to `MAX_GRANT`, and neither
/// a grant nor a return reaches the ledger, which has no operation for steps; once the parse has
/// stopped a grant answers the stop. The parse of a literal reaches the ledger only through
/// charges, credits and polls.
#[test]
fn a_literal_spends_no_step_at_parse() {
    let mut ledger = Recording::default();
    {
        let meter = Meter::new(&mut ledger);
        let mut budget = PatternBudget::new(&meter);
        for asked in [1, 2, 1_000, MAX_GRANT] {
            assert_eq!(budget.grant(asked), Ok(asked));
            budget.refund(asked);
        }
        assert_eq!(meter.held(), 0);
        assert_eq!(meter.stopped(), None);
    }
    assert_eq!((ledger.charged, ledger.credited, ledger.polls), (0, 0, 0));

    let mut ledger = Recording {
        interrupt_at: Some(1),
        ..Recording::default()
    };
    let meter = Meter::new(&mut ledger);
    let mut budget = PatternBudget::new(&meter);
    assert_eq!(budget.poll(), Err(swell_regexp::Stop::Interrupted));
    assert_eq!(budget.grant(1), Err(swell_regexp::Stop::Interrupted));
}

/// `/(?i-i:a)/` adds and removes `i`: an early error of the pattern
/// (`#sec-patterns-static-semantics-early-errors`), which the parse answers before the compile
/// step runs. A literal with the flag `i` or a modifier `i` and no error parses.
#[test]
fn a_literal_with_i_and_an_early_error_is_the_early_error() {
    let errors = parse_script(&literal("(?i-i:a)", ""), OPTIONS).unwrap_err();
    let messages: Vec<&str> = errors.iter().map(|e| e.message.as_str()).collect();
    assert_eq!(messages, vec!["a modifier both added and removed"]);
    assert_eq!(message_kind(&errors[0].message), MessageKind::Grammar);
    assert_eq!((errors[0].line, errors[0].column), (1, 5));
    for (pattern, flags) in [
        ("(?i:a)", ""),
        ("a", "i"),
        ("(?-i:a)", "i"),
        ("(?i:a)b", "u"),
    ] {
        let parsed = parse_script(&literal(pattern, flags), OPTIONS);
        assert!(parsed.is_ok(), "/{pattern}/{flags}: {:?}", parsed.err());
    }
}

/// One literal for each row of `docs/plan/slice-05/consumer-regexps.tsv`, written for this
/// test with the constructs that row names (nothing of the consumer's files is copied); each
/// parses. Two of them have `{` and `}` as pattern characters, by the grammar of B.1.2.
#[test]
fn the_patterns_of_the_consumer_table_parse() {
    let rows = [
        // testharness.js 581: anchors, identity escapes, \s, a non-capturing group, { and }
        // as pattern characters (Annex B), captures, alternation.
        (r"^\s*(?:{\s*)?(\w+)\s*}?\s*$|^\-\/(a|b)$", ""),
        // 584: a class of \u escapes.
        (r"[\u0000-\u001f\u007f-\u009f]", ""),
        // 587: anchors, captures, a negated class, a quantified group.
        (r"^([^:]+)(:[^:]*)+$", ""),
        // 3651: \n.
        (r"\n", "g"),
        // 4013: captures, classes of surrogate ranges, a negative lookahead, alternation, ^ in
        // a group.
        (
            r"([\ud800-\udbff](?![\udc00-\udfff]))|(^|[^\ud800-\udbff])([\udc00-\udfff])",
            "g",
        ),
        // 4433: \s.
        (r"\s", "g"),
        // 4558: \w, \d, identity escapes, a negated class, an optional non-capturing group.
        (r"\w\d\.\-[^\w](?:\:\d)?", "g"),
        // 4660: identity escapes of $ { }, a capture, a negated class.
        (r"\$\{([^\}]*)\}", "g"),
        // 4816: a class of escaped syntax characters.
        (r"[\\^$.*+?()[\]{}|]", "g"),
        // 5004: anchors, a capture, an empty alternative, identity escapes.
        (r"^(\-|)\.$", ""),
        // template.js 68: { and } as pattern characters (Annex B), a capture, a negated class.
        (r"{([^}]*)}", "g"),
        // test.js 251: a class of \x escapes.
        (r"[\x00-\x1f\x7f]", "g"),
    ];
    assert_eq!(rows.len(), 12, "one literal per row of the table");
    for (pattern, flags) in rows {
        let parsed = parse_script(&literal(pattern, flags), OPTIONS);
        assert!(parsed.is_ok(), "/{pattern}/{flags}: {:?}", parsed.err());
    }
}
