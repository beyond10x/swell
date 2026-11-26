//! The consumer of slice 4 (`story:slice-04-conformance-close`): the nine references of the
//! first consumer's harness to `Date` and `Math`, together in one script, after both stories
//! that own the halves have landed. The two `Date.now()` reads of its busy wait
//! (`date_clock::consumer_date_reads_of_surf_resolve_as_the_standard_says`,
//! `story:date-object-clock-and-time-value`) and the seven `Math` calls of its harness
//! (`builtins_math::consumer_calls_of_the_harness_answer_as_the_standard_says`,
//! `story:math-constants-and-exact-functions`). Run also with the feature
//! `collect-every-safe-point`.
//!
//! Each reference depends on a part slice 4 brought, so that it fails when that part is gone or
//! answers wrongly: `Math.ceil` gets an argument that is no integer, and the two calls of
//! `Math.max`, a function older than the slice, get a part of slice 4 inside their arguments
//! (`Math.trunc` and `Math.round`) that decides their result. Before slice 4 `Math` had `max`
//! and nothing else, and `Date` did not exist.
#![cfg(test)]

use crate::support;

use support::host::{agent, realm};
use support::script::{Outcome, eval_text};

/// The nine references, each with the text of its result that the standard gives: `Date.now()`
/// over a clock at 1.7 × 10^18 nanoseconds (`#sec-date.now`, the time value floored to
/// milliseconds), the count of the busy wait over a clock that advances by one millisecond per
/// read (one read before the loop, five in it), and the seven calls of `Math` with
/// Number::toString of each result (`#sec-numeric-types-number-tostring`; `Math.max(-3, -0)`
/// is -0, whose text is "0").
const REFERENCES: [(&str, &str); 9] = [
    ("Date.now() before the busy wait", "1700000000000"),
    ("Date.now() in the busy wait, its count", "4"),
    ("Math.floor(7 / 2)", "3"),
    ("Math.min(3 + 1, 2)", "2"),
    ("Math.max(2 - 5, Math.trunc(-0.5))", "0"),
    ("Math.abs(1 - 4.5)", "3.5"),
    ("Math.abs(0.25 - 0.75)", "0.5"),
    ("Math.ceil(250 / 100)", "3"),
    ("Math.max(3, Math.round(3.6))", "4"),
];

/// The script: the busy wait of the consumer's page as it is written there, then the seven
/// calls, and the nine results joined by commas.
const SCRIPT: &str = "var start = Date.now(); var end = start + 5; var n = 0; \
     while ((end - Date.now()) > 0) { n++; } \
     [start, n, Math.floor(7 / 2), Math.min(3 + 1, 2), Math.max(2 - 5, Math.trunc(-0.5)), \
      Math.abs(1 - 4.5), Math.abs(0.25 - 0.75), Math.ceil(250 / 100), \
      Math.max(3, Math.round(3.6))].join();";

/// The parts of slice 4 the script calls, each as the expression that names it.
const PARTS: [&str; 7] = [
    "Date.now",
    "Math.floor",
    "Math.min",
    "Math.trunc",
    "Math.abs",
    "Math.ceil",
    "Math.round",
];

/// The nine texts joined as the script joins them, quoted as `eval_text` shows a String.
fn expected_text() -> String {
    let texts: Vec<&str> = REFERENCES.iter().map(|(_, text)| *text).collect();
    format!("\"{}\"", texts.join(","))
}

/// `prefix` and then the script, on an agent whose clock starts at 1.7 × 10^18 nanoseconds and
/// advances by one millisecond per read.
fn run_script(prefix: &str) -> Outcome {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    agent.host_mut().now = 1_700_000_000_000_000_000;
    agent.host_mut().clock_step_ns = 1_000_000;
    eval_text(&mut agent, &realm, &format!("{prefix}{SCRIPT}"))
}

#[test]
fn consumer_nine_references_of_surf_resolve_as_the_standard_says() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    agent.host_mut().now = 1_700_000_000_000_000_000;
    agent.host_mut().clock_step_ns = 1_000_000;
    let text = match eval_text(&mut agent, &realm, SCRIPT) {
        Outcome::Normal(text) => text,
        other => panic!("the script answered {other:?}"),
    };
    let joined = text
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
        .unwrap_or_else(|| panic!("the script answered {text}, not a String"));
    let results: Vec<&str> = joined.split(',').collect();
    assert_eq!(results.len(), REFERENCES.len(), "{joined}");
    for ((reference, expected), result) in REFERENCES.iter().zip(&results) {
        println!("{reference}: {result}");
        assert_eq!(result, expected, "{reference}");
    }
    let clock_calls = agent
        .host()
        .calls
        .iter()
        .filter(|call| call.as_str() == "host_system_utc_epoch_nanoseconds")
        .count();
    assert_eq!(clock_calls, 6);
}

/// Without each part of slice 4 the script calls (its property deleted), the script throws:
/// no reference answers its text without the part it names.
#[test]
fn the_nine_references_fail_without_each_part_of_slice_4() {
    for part in PARTS {
        let outcome = run_script(&format!("delete {part};\n"));
        assert!(
            matches!(outcome, Outcome::Throw(_)),
            "without {part} the script answered {outcome:?}"
        );
    }
}

/// With each function of `Math` the script calls replaced by the identity on its first
/// argument, the nine texts differ from those of the standard: each call's argument is chosen
/// so that the function changes it (`Math.ceil(250 / 100)` rounds 2.5 up, `Math.trunc(-0.5)`
/// gives -0, `Math.round(3.6)` gives 4).
#[test]
fn the_nine_references_tell_each_math_function_from_the_identity() {
    let expected = expected_text();
    for part in PARTS.iter().filter(|part| part.starts_with("Math.")) {
        let outcome = run_script(&format!("{part} = function (x) {{ return x; }};\n"));
        assert_ne!(
            outcome,
            Outcome::Normal(expected.clone()),
            "with {part} as the identity the nine results are those of the standard"
        );
    }
}
