//! Template literals without a tag (`#sec-template-literals`): the ToString of each substitution
//! (`#sec-template-literals-runtime-semantics-evaluation`), the order in which the substitutions
//! run, the host's `string_length` reached by a template's concatenation, and `parser_nesting`
//! reached by substitutions nested in substitutions. Written by `story:template-literals`. Run
//! also with the feature `collect-every-safe-point`.
//!
//! Each script compares its results by `===` with string literals and answers `true`; none uses
//! a construct of slice 6 other than the template literal.
#![cfg(test)]

use crate::support;

use support::host::{agent_with, limits, realm};
use support::script::{Outcome, eval_text, run, run_both};

/// What a script that checks its own results answers.
fn passed() -> Outcome {
    Outcome::Normal("true".to_string())
}

/// `#sec-template-literals-runtime-semantics-evaluation`: a substitution goes through ToString
/// (`#sec-tostring`), so an object is asked for `toString` first (ToPrimitive with hint
/// string), where `+` calls ToPrimitive with no hint and so asks for `valueOf` first
/// (`#sec-applystringornumericbinaryoperator`).
#[test]
fn a_substitution_calls_to_string_before_value_of() {
    let source = "var log = \"\";\n\
                  var o = {\n\
                    toString: function () { log += \"t\"; return \"T\"; },\n\
                    valueOf: function () { log += \"v\"; return \"V\"; }\n\
                  };\n\
                  var template = `${o}`;\n\
                  var plus = \"\" + o;\n\
                  var only = { valueOf: function () { log += \"w\"; return 1; } };\n\
                  var fallback = `<${only}>`;\n\
                  template === \"T\" && plus === \"V\" && fallback === \"<[object Object]>\"\n\
                    && log === \"tv\";";
    assert_eq!(run_both(source), (passed(), passed()));
}

/// `#sec-template-literals-runtime-semantics-evaluation` and
/// `#sec-runtime-semantics-substitutionevaluation`: the substitutions run in source order, each
/// once, and each value is converted by ToString before the next substitution runs.
#[test]
fn substitutions_are_evaluated_left_to_right_once() {
    let source = "var log = \"\";\n\
                  function f(x, tag) { log += tag; return x; }\n\
                  function o(n) { return { toString: function () { log += \"t\" + n; return \"\" + n; } }; }\n\
                  var r = `a${f(1, \"f1\")}b${f(2, \"f2\")}c${f(3, \"f3\")}d`;\n\
                  var first = log;\n\
                  log = \"\";\n\
                  var s = `${f(o(1), \"f1\")}-${f(o(2), \"f2\")}`;\n\
                  r === \"a1b2c3d\" && first === \"f1f2f3\" && s === \"1-2\" && log === \"f1t1f2t2\";";
    assert_eq!(run_both(source), (passed(), passed()));
}

/// The concatenation of a template is bounded by `string_length` as `+` is (HOST-20): under a
/// limit of 1,000 code units `s` doubles to 512 by `+`, which fits; `` `${s}${s}` ``, whose
/// substitutions are each under the limit and whose concatenation is past it, answers the
/// `RangeError` that `s + s` answers, with the same message, and `s` is unchanged.
#[test]
fn a_template_longer_than_string_length_ends_as_concatenation_does() {
    let mut limits = limits();
    limits.string_length = 1_000;
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    let setup = "var s = \"x\";\n\
                 for (var i = 0; i < 9; i++) s = s + s;\n\
                 s.length;";
    assert_eq!(
        eval_text(&mut agent, &realm, setup),
        Outcome::Normal("512".to_string())
    );
    assert_eq!(
        eval_text(&mut agent, &realm, "s + s;"),
        Outcome::Throw("RangeError".to_string())
    );
    assert_eq!(
        eval_text(&mut agent, &realm, "`${s}${s}`;"),
        Outcome::Throw("RangeError".to_string())
    );
    let compare = "var plus, template;\n\
                   try { s + s; } catch (e) { plus = e; }\n\
                   try { `${s}${s}`; } catch (e) { template = e; }\n\
                   plus instanceof RangeError && template instanceof RangeError\n\
                     && template.message === plus.message && s.length === 512\n\
                     && `${s}`.length === 512;";
    assert_eq!(eval_text(&mut agent, &realm, compare), passed());
}

/// `#sec-template-literals`: each substitution is an Expression, counted against
/// `parser_nesting` as a parenthesized expression is (`docs/design/engine.md`, decision 3):
/// `parse_script` of 40 templates nested in substitutions under a `parser_nesting` of 32
/// answers the `SyntaxError` that names `parser_nesting`; 2 nested templates parse and run (4
/// do not: measured, the error then stands at the fourth template, column 14).
#[test]
fn substitutions_count_against_parser_nesting() {
    let nested = |n: usize| format!("x = {}0{};", "`${".repeat(n), "}`".repeat(n));
    let mut limits = limits();
    limits.parser_nesting = 32;
    match run(limits, &nested(40)) {
        // The message of a parse error ends with the position, `(<line>:<column>)`.
        Outcome::Errors(messages) => assert!(
            messages.len() == 1
                && messages
                    .first()
                    .is_some_and(|m| m.starts_with("nesting deeper than parser_nesting (32) (")),
            "{messages:?}"
        ),
        other => panic!("40 nested templates answered {other:?}"),
    }
    assert_eq!(
        run(limits, &format!("{} x === \"0\";", nested(2))),
        passed()
    );
}
