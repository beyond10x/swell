//! The consumer of slice 4 (`story:slice-04-conformance-close`): the nine references of the
//! first consumer's harness to `Date` and `Math`, together in one script, after both stories
//! that own the halves have landed. The two `Date.now()` reads of its busy wait
//! (`date_clock::consumer_date_reads_of_surf_resolve_as_the_standard_says`,
//! `story:date-object-clock-and-time-value`) and the seven `Math` calls of its harness with the
//! argument values the script gives them
//! (`builtins_math::consumer_calls_of_the_harness_answer_as_the_standard_says`,
//! `story:math-constants-and-exact-functions`). Run also with the feature
//! `collect-every-safe-point`.
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

/// The script: the busy wait of the consumer's page as it is written there, then the seven
/// calls, and the nine results joined by commas.
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
