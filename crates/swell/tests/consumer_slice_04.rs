//! The consumer of slice 4 (`story:slice-04-conformance-close`), in two scripts.
//!
//! The consumer's script holds the nine references of the first consumer's harness to `Date` and
//! `Math`, together in one script, after both stories that own the halves have landed: the two
//! `Date.now()` reads of its busy wait
//! (`date_clock::consumer_date_reads_of_surf_resolve_as_the_standard_says`,
//! `story:date-object-clock-and-time-value`) and the seven `Math` calls of its harness with the
//! argument values the script gives them
//! (`builtins_math::consumer_calls_of_the_harness_answer_as_the_standard_says`,
//! `story:math-constants-and-exact-functions`). Its test holds one thing: the consumer's calls
//! answer what the standard gives. The two calls of `Math.max` cannot fail when slice 4 is taken
//! out or answers wrongly, since the function is older than the slice. `Math.ceil(300 / 100)`
//! fails when `ceil` is absent, and cannot fail when `ceil` answers as the identity or as
//! `floor`, `round` or `trunc`, since its argument is an integer.
//!
//! The parts script holds what the consumer's script cannot: wrong answers of the parts of slice
//! 4 the consumer calls (`Date.now`, `Math.floor`, `Math.min`, `Math.abs` and `Math.ceil`), each
//! with arguments that tell it from its absence, from the identity, from each rounding neighbour
//! (`floor`, `ceil`, `round`, `trunc`) and that show the sign of a zero.
//!
//! Run also with the feature `collect-every-safe-point`.
#![cfg(test)]

use crate::support;

use support::host::{agent, realm};
use support::script::{Outcome, eval_text};

/// The nine references, each with the text of its result that the standard gives: `Date.now()`
/// over a clock at 1.7 × 10^18 nanoseconds (`#sec-date.now`, the time value floored to
/// milliseconds), the count of the busy wait over a clock that advances by one millisecond per
/// read (one read before the loop, five in it), and the seven calls of `Math` with
/// Number::toString of each result (`#sec-numeric-types-number-tostring`).
const REFERENCES: [(&str, &str); 9] = [
    ("Date.now() before the busy wait", "1700000000000"),
    ("Date.now() in the busy wait, its count", "4"),
    ("Math.floor(7 / 2)", "3"),
    ("Math.min(3 + 1, 2)", "2"),
    ("Math.max(2 - 5, 0)", "0"),
    ("Math.abs(1 - 4.5)", "3.5"),
    ("Math.abs(0.25 - 0.75)", "0.5"),
    ("Math.ceil(300 / 100)", "3"),
    ("Math.max(3, 4)", "4"),
];

/// The consumer's script: the busy wait of the consumer's page as it is written there, then the
/// seven calls, and the nine results joined by commas.
const SCRIPT: &str = "var start = Date.now(); var end = start + 5; var n = 0; \
     while ((end - Date.now()) > 0) { n++; } \
     [start, n, Math.floor(7 / 2), Math.min(3 + 1, 2), Math.max(2 - 5, 0), \
      Math.abs(1 - 4.5), Math.abs(0.25 - 0.75), Math.ceil(300 / 100), Math.max(3, 4)].join();";

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

/// The calls of the parts script, each with the text of its result that the standard gives,
/// a zero shown with its sign. `Date.now()` reads a clock at 1.7 × 10^18 + 600,000 nanoseconds:
/// the time value is floored to milliseconds (`#sec-date.now`), not rounded. `floor`
/// (`#sec-math.floor`) and `ceil` (`#sec-math.ceil`) on 2.7, -2.7, 2.5 and -2.5 give rows that
/// differ from each other and from those of `round` (3, -3, 3, -2) and `trunc` (2, -2, 2, -2);
/// `Math.floor(-0)` and `Math.ceil(-0.5)` are -0. `min` (`#sec-math.min`) answers its lowest
/// argument in either place, -0 below +0 in either order, and NaN when one argument is NaN;
/// `abs` (`#sec-math.abs`) answers +0 for -0.
const PART_CALLS: [(&str, &str); 22] = [
    ("Date.now()", "1700000000000"),
    ("Math.floor(2.7)", "2"),
    ("Math.floor(-2.7)", "-3"),
    ("Math.floor(2.5)", "2"),
    ("Math.floor(-2.5)", "-3"),
    ("Math.floor(-0)", "-0"),
    ("Math.floor(-0.5)", "-1"),
    ("Math.ceil(2.7)", "3"),
    ("Math.ceil(-2.7)", "-2"),
    ("Math.ceil(2.5)", "3"),
    ("Math.ceil(-2.5)", "-2"),
    ("Math.ceil(-0.5)", "-0"),
    ("Math.min(4, 2)", "2"),
    ("Math.min(2, 4)", "2"),
    ("Math.min(0, -0)", "-0"),
    ("Math.min(-0, 0)", "-0"),
    ("Math.min(1, NaN)", "NaN"),
    ("Math.abs(-3.5)", "3.5"),
    ("Math.abs(2)", "2"),
    ("Math.abs(-0)", "0"),
    ("Math.abs(0)", "0"),
    ("Math.abs(-Infinity)", "Infinity"),
];

/// The parts of slice 4 the consumer calls, each as the expression that names it.
const PARTS: [&str; 5] = [
    "Date.now",
    "Math.floor",
    "Math.min",
    "Math.abs",
    "Math.ceil",
];

/// The rounding functions of `Math` the consumer calls, each with its three neighbours.
const ROUNDING_SWAPS: [(&str, &str); 6] = [
    ("Math.floor", "Math.ceil"),
    ("Math.floor", "Math.round"),
    ("Math.floor", "Math.trunc"),
    ("Math.ceil", "Math.floor"),
    ("Math.ceil", "Math.round"),
    ("Math.ceil", "Math.trunc"),
];

/// The parts script: each call of `PART_CALLS`, its result shown with the sign of a zero, joined
/// by commas.
fn parts_script() -> String {
    let calls: Vec<String> = PART_CALLS
        .iter()
        .map(|(call, _)| format!("show({call})"))
        .collect();
    format!(
        "var show = function (x) {{ return (x === 0 && 1 / x < 0) ? '-0' : String(x); }}; \
         [{}].join();",
        calls.join(", ")
    )
}

/// `prefix` and then the parts script, on an agent whose clock stands at 1.7 × 10^18 + 600,000
/// nanoseconds.
fn run_parts(prefix: &str) -> Outcome {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    agent.host_mut().now = 1_700_000_000_000_600_000;
    agent.host_mut().clock_step_ns = 0;
    eval_text(&mut agent, &realm, &format!("{prefix}{}", parts_script()))
}

/// The texts of the standard joined as the parts script joins them, quoted as `eval_text` shows
/// a String.
fn parts_expected() -> Outcome {
    let texts: Vec<&str> = PART_CALLS.iter().map(|(_, text)| *text).collect();
    Outcome::Normal(format!("\"{}\"", texts.join(",")))
}

/// Every call of the parts script answers its text of the standard; a failure names every call
/// that differs.
#[test]
fn parts_script_answers_as_the_standard_says() {
    let outcome = run_parts("");
    let text = match &outcome {
        Outcome::Normal(text) => text.clone(),
        other => panic!("the parts script answered {other:?}"),
    };
    let joined = text
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
        .unwrap_or_else(|| panic!("the parts script answered {text}, not a String"));
    let results: Vec<&str> = joined.split(',').collect();
    assert_eq!(results.len(), PART_CALLS.len(), "{joined}");
    let wrong: Vec<String> = PART_CALLS
        .iter()
        .zip(&results)
        .filter(|((_, expected), result)| expected != *result)
        .map(|((call, expected), result)| format!("{call} answered {result}, not {expected}"))
        .collect();
    assert!(wrong.is_empty(), "{}", wrong.join("; "));
    assert_eq!(outcome, parts_expected());
}

/// Without each part of slice 4 the consumer calls (its property deleted), the parts script
/// throws.
#[test]
fn parts_script_fails_without_each_part_of_slice_4() {
    for part in PARTS {
        let outcome = run_parts(&format!("delete {part};\n"));
        assert!(
            matches!(outcome, Outcome::Throw(_)),
            "without {part} the parts script answered {outcome:?}"
        );
    }
}

/// With each function of `Math` of the parts script replaced by the identity on its first
/// argument, the texts differ from those of the standard.
#[test]
fn parts_script_tells_each_math_function_from_the_identity() {
    for part in PARTS.iter().filter(|part| part.starts_with("Math.")) {
        let outcome = run_parts(&format!("{part} = function (x) {{ return x; }};\n"));
        assert_ne!(
            outcome,
            parts_expected(),
            "with {part} as the identity the texts are those of the standard"
        );
    }
}

/// With each rounding function the consumer calls answering as one of its neighbours, the texts
/// differ from those of the standard. Lists every replacement under which they do not.
#[test]
fn parts_script_tells_each_rounding_function_from_its_neighbours() {
    let mut survivors = Vec::new();
    for (part, neighbour) in ROUNDING_SWAPS {
        let prefix = format!(
            "var neighbour = {neighbour}; \
             {part} = function (x) {{ return neighbour(x); }};\n"
        );
        if run_parts(&prefix) == parts_expected() {
            survivors.push(format!("{part} answering as {neighbour}"));
        }
    }
    assert!(
        survivors.is_empty(),
        "the texts are those of the standard with: {}",
        survivors.join("; ")
    );
}

/// With each function of `Math` of the parts script answering a zero of the other sign
/// (`floor`, `ceil` and `min` +0 for every zero, `abs` -0), the texts differ from those of the
/// standard.
#[test]
fn parts_script_sees_the_sign_of_each_zero() {
    let mutants = [
        ("Math.floor", "0"),
        ("Math.ceil", "0"),
        ("Math.min", "0"),
        ("Math.abs", "-0"),
    ];
    for (part, zero) in mutants {
        let prefix = format!(
            "var original = {part}; \
             {part} = function () {{ \
               var r = original.apply(undefined, arguments); return r === 0 ? {zero} : r; }};\n"
        );
        assert_ne!(
            run_parts(&prefix),
            parts_expected(),
            "{part} answering {zero} for a zero gives the texts of the standard"
        );
    }
}
