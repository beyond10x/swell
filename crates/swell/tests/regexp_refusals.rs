//! The refusals of RegExpInitialize until the stories of slice 5 that lift them
//! (`docs/design/engine.md`, decision 7, "Refusals until the later stories of slice 5"; section
//! 12, the third form; `docs/design/host-interface.md`, decision 13, "A refusal raised by
//! `RegExp`", HOST-191). Each story that lifted a refusal changed its line here to the answer
//! HOST-191 gives once slice 5 is delivered:
//! `story:regexp-v-flag-class-sets-and-properties-of-strings` that of `v`,
//! `story:regexp-property-escapes-and-property-tables` that of a property escape,
//! `story:regexp-case-insensitive-matching-and-modifiers` that of `i` and
//! `story:identifiers-and-group-names-outside-ascii` that of a group name outside ASCII. None is
//! left, and `refusals()` stays 0.
//! Run also with the feature `collect-every-safe-point`.
#![cfg(test)]

use crate::support;

use support::host::{agent_with, limits, realm};
use support::script::{Outcome, eval_text};

/// `true`, as an outcome.
fn yes() -> Outcome {
    Outcome::Normal("true".to_string())
}

/// HOST-191: on a new agent, `new RegExp("a", "v").test("a")` answers `true` and leaves
/// `refusals()` at 0, as do `new RegExp("(")`, `new RegExp("a", "gg")`, `new RegExp("a",
/// "g").test("a")`, `new RegExp("\\p{L}", "u").test("a")`, `new RegExp("a", "i").test("A")`,
/// `/a/i.test("A")` and `new RegExp("(?<\\u{e9}>a)").exec("a").groups["é"]`, `v`, a property
/// escape, `i` and a group name outside ASCII answered once their stories landed, and an error
/// of ToString of the pattern, which is thrown.
#[test]
fn host_191_regexp_counts_its_refusals_of_slice_5() {
    let mut agent = agent_with(limits()).unwrap();
    let realm = realm(&mut agent);
    assert_eq!(agent.refusals(), 0);
    for script in [
        r#"new RegExp("a", "v").test("a")"#,
        r#"/[\p{L}--[a-z]]/v.test("A") && !/[\p{L}--[a-z]]/v.test("a")"#,
        r#"new RegExp("^[\\q{abc|ab}]$", "v").test("ab")"#,
        r#"/^\p{RGI_Emoji_Flag_Sequence}$/v.test("\u{1F1E9}\u{1F1EA}")"#,
        r#"try { new RegExp("[a&&&b]", "v"); false; } catch (e) { e instanceof SyntaxError &&
             e.message === "a reserved double punctuator in a class"; }"#,
        r#"try { new RegExp("("); false; } catch (e) { e instanceof SyntaxError; }"#,
        r#"try { new RegExp("a", "gg"); false; } catch (e) { e instanceof SyntaxError; }"#,
        r#"new RegExp("a", "g").test("a")"#,
        r#"new RegExp("\\p{L}", "u").test("a")"#,
        r#"new RegExp("a", "i").test("A")"#,
        r#"/a/i.test("A")"#,
        r#"new RegExp("(?i:a)").test("A") && !new RegExp("(?-i:a)", "i").test("A")"#,
        r#"new RegExp("(?<\\u{e9}>a)").exec("a").groups["é"] === "a""#,
        r#"/(?<é>a)\k<é>/i.exec("aA").groups["é"] === "a""#,
        r#"var t = { toString: function () { throw 7; } };
           try { new RegExp(t, "v"); false; } catch (e) { e === 7; }"#,
        r#"try { new RegExp("a", "vv"); false; } catch (e) { e instanceof SyntaxError &&
             e.message === "repeated regular expression flag"; }"#,
        r#"try { new RegExp("(", "v"); false; } catch (e) { e instanceof SyntaxError &&
             e.message === "unterminated group"; }"#,
    ] {
        assert_eq!(eval_text(&mut agent, &realm, script), yes(), "{script}");
        assert_eq!(agent.refusals(), 0, "{script}");
    }
}
