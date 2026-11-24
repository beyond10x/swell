//! The pattern of a regular expression literal checked at parse through the host interface
//! (`docs/design/engine.md`, decision 7, "No Rust recursion in the parser and the compiler of a
//! pattern"; `#sec-primary-expression-regular-expression-literals-static-semantics-early-errors`):
//! a pattern nested deeper than `parser_nesting` is the `SyntaxError` naming the limit, an early
//! error of the script through `parse_script` (HOST-142) and a `SyntaxError` the script catches
//! through `eval` and `Function`; a pattern that is not valid is an early error of the eval
//! code; and no part of the pattern grammar is a refused parse any more, so `refusals()`
//! (HOST-164) stays 0. Run also with the feature `collect-every-safe-point`.
#![cfg(test)]

use crate::support;

use support::host::{agent_with, limits, realm};
use support::script::{Outcome, eval_text, messages, run};
use swell::{Parsed, Source};

/// A pattern of `n` nested capturing groups around `a`.
fn groups(n: usize) -> String {
    format!("{}a{}", "(".repeat(n), ")".repeat(n))
}

/// The messages of `parse_script` of `source` with the test limits (`parser_nesting` 1,000),
/// `None` when it parses; and `refusals()` after it.
fn parse_errors(source: &str) -> (Option<Vec<String>>, u64) {
    let mut agent = agent_with(limits()).unwrap();
    let realm = realm(&mut agent);
    let errors = match agent
        .parse_script(&realm, Source::Utf8(source), None)
        .unwrap()
    {
        Parsed::Parsed(_) => None,
        Parsed::Errors(errors) => Some(messages(&mut agent, &realm, &errors)),
        Parsed::Terminated(termination) => panic!("{source:?} ended in {termination:?}"),
    };
    (errors, agent.refusals())
}

/// HOST-142: with `parser_nesting` 1,000, `parse_script` of a literal of 1,001 nested groups
/// answers `Parsed::Errors` with one `SyntaxError` whose message is `nesting deeper than
/// parser_nesting (1000)`, with the literal's line and column after it as every error of a parse
/// has them; 1,000 nested groups parse; neither is a refusal, so `refusals()` stays 0. The
/// pattern's nesting counts from zero at its start, whatever the script's own depth.
#[test]
fn host_142_a_literal_past_parser_nesting_is_a_parse_error_naming_it() {
    assert_eq!(
        parse_errors(&format!("x = /{}/;", groups(1_001))),
        (
            Some(vec![
                "nesting deeper than parser_nesting (1000) (1:5)".to_string()
            ]),
            0
        )
    );
    assert_eq!(
        parse_errors(&format!("x = /{}/;", groups(1_000))),
        (None, 0)
    );
    let deep_script = format!(
        "x = {}/{}/{};",
        "(".repeat(100),
        groups(1_000),
        ")".repeat(100)
    );
    assert_eq!(parse_errors(&deep_script), (None, 0));
}

/// HOST-142 through `eval` and `Function`, as HOST-139 and HOST-141 hold for the script's own
/// nesting: a literal of 1,001 nested groups in eval code, direct and indirect, and in the body
/// given to `Function`, throws a `SyntaxError` naming `parser_nesting` that the script catches;
/// one of 1,000 nested groups parses there. No refusal is counted.
#[test]
fn a_literal_past_parser_nesting_in_eval_code_throws_a_catchable_syntax_error() {
    let mut agent = agent_with(limits()).unwrap();
    let realm = realm(&mut agent);
    for call in ["eval", "(0, eval)", "Function"] {
        let source = format!(
            "try {{ {call}(\"x = /{}/;\"); }} catch (e) {{ e.name + \": \" + e.message; }}",
            groups(1_001)
        );
        let Outcome::Normal(caught) = eval_text(&mut agent, &realm, &source) else {
            panic!("{call}: the script did not complete normally");
        };
        assert!(
            caught.starts_with("\"SyntaxError: nesting deeper than parser_nesting (1000) ("),
            "{call}: {caught}"
        );
        let source = format!(
            "typeof {call}(\"(function () {{ return /{}/; }})\");",
            groups(1_000)
        );
        assert_eq!(
            eval_text(&mut agent, &realm, &source),
            Outcome::Normal("\"function\"".to_string()),
            "{call}"
        );
    }
    assert_eq!(agent.refusals(), 0);
}

/// A pattern that is not valid with its flags is an early error of the eval code and of the
/// body given to `Function` (`#sec-performeval`, `#sec-createdynamicfunction`): the call throws
/// a `SyntaxError` with the message of the pattern's error, which the script catches. A grammar
/// error is no refusal.
#[test]
fn an_invalid_pattern_in_eval_code_throws_a_syntax_error() {
    let mut agent = agent_with(limits()).unwrap();
    let realm = realm(&mut agent);
    for (call, pattern, message) in [
        (
            "eval",
            "(?<a>b)(?<a>c)",
            "duplicate capture group name (1:1)",
        ),
        (
            "(0, eval)",
            "a{2,1}",
            "numbers out of order in {} quantifier (1:1)",
        ),
        ("eval", "(a", "unterminated group (1:1)"),
        ("Function", "[z-a]", "range out of order in character class"),
    ] {
        let source = format!(
            "try {{ {call}(\"/{pattern}/\"); \"parsed\"; }} catch (e) {{ e.name + \": \" + \
             e.message; }}"
        );
        let Outcome::Normal(caught) = eval_text(&mut agent, &realm, &source) else {
            panic!("{call} /{pattern}/: the script did not complete normally");
        };
        assert!(
            caught.starts_with(&format!("\"SyntaxError: {message}")),
            "{call} /{pattern}/: {caught}"
        );
    }
    assert_eq!(
        run(limits(), "x = /(?<a>b)(?<a>c)/;"),
        Outcome::Errors(vec!["duplicate capture group name (1:5)".to_string()])
    );
    assert_eq!(agent.refusals(), 0, "a grammar error is no refusal");
}

/// No part of the pattern grammar is refused any more (HOST-164): a literal with the flag `v`,
/// a property escape under `u` and a group name outside ASCII parse and move nothing; an early
/// error of `v` is an early error of the script and of the eval code, no refusal; and classes of
/// `v` nested deeper than `parser_nesting` are the `SyntaxError` naming the limit.
#[test]
fn the_patterns_of_slice_5_in_a_literal_are_not_refused() {
    assert_eq!(parse_errors("x = /\\p{L}/u;"), (None, 0));
    assert_eq!(
        parse_errors("x = /\\P{Script=Greek}[\\p{Lu}]/u;"),
        (None, 0)
    );
    assert_eq!(parse_errors("x = /(?<\u{e9}>a)/;"), (None, 0));
    assert_eq!(parse_errors("x = /(?<\\u{e9}>a)\\k<\u{e9}>/u;"), (None, 0));
    assert_eq!(parse_errors("x = /[a]/v;"), (None, 0));
    assert_eq!(
        parse_errors("x = /[\\p{L}--[a-z]][\\q{ab|c}&&\\q{c}]\\p{RGI_Emoji}/gv;"),
        (None, 0)
    );
    assert_eq!(
        parse_errors("x = /[a&&&b]/v;"),
        (
            Some(vec![
                "a reserved double punctuator in a class (1:5)".to_string()
            ]),
            0
        )
    );
    let deep = format!("x = /{}a{}/v;", "[".repeat(1_001), "]".repeat(1_001));
    assert_eq!(
        parse_errors(&deep),
        (
            Some(vec![
                "nesting deeper than parser_nesting (1000) (1:5)".to_string()
            ]),
            0
        )
    );
    let deep = format!("x = /{}a{}/v;", "[".repeat(1_000), "]".repeat(1_000));
    assert_eq!(parse_errors(&deep), (None, 0));
    let mut agent = agent_with(limits()).unwrap();
    let realm = realm(&mut agent);
    let source = "/[\\q{ab|c}--c]/v.test(\"ab\") && !/^[\\q{ab|c}--c]$/v.test(\"c\")";
    assert_eq!(
        eval_text(&mut agent, &realm, source),
        Outcome::Normal("true".to_string())
    );
    let source = "try { eval(\"/[a--]/v\"); } catch (e) { e.name + \": \" + e.message; }";
    assert_eq!(
        eval_text(&mut agent, &realm, source),
        Outcome::Normal(
            "\"SyntaxError: a class set operation without its operand (1:1)\"".to_string()
        )
    );
    assert_eq!(agent.refusals(), 0);
}
