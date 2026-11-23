//! The refusals of RegExpInitialize until the stories of slice 5 that lift them
//! (`docs/design/engine.md`, decision 7, "Refusals until the later stories of slice 5"; section
//! 12, the third form; `docs/design/host-interface.md`, decision 13, "A refusal raised by
//! `RegExp`", HOST-191). Each story that lifts a refusal changes its line here to the answer
//! HOST-191 gives once slice 5 is delivered:
//! `story:regexp-v-flag-class-sets-and-properties-of-strings` for `v`;
//! `story:regexp-property-escapes-and-property-tables` lifted the refusal of a property escape,
//! `story:regexp-case-insensitive-matching-and-modifiers` that of `i` and
//! `story:identifiers-and-group-names-outside-ascii` that of a group name outside ASCII.
//! Run also with the feature `collect-every-safe-point`.
#![cfg(test)]

use crate::support;

use support::host::{agent_with, limits, realm};
use support::script::{Outcome, eval_text};

/// `true`, as an outcome.
fn yes() -> Outcome {
    Outcome::Normal("true".to_string())
}

/// The script that answers whether `construction` throws a `SyntaxError` whose message is
/// exactly `message`.
fn throws(construction: &str, message: &str) -> String {
    format!(
        "try {{ {construction}; false; }} catch (e) {{ e instanceof SyntaxError && e.message === \"{message}\"; }}"
    )
}

/// HOST-191: on a new agent, `new RegExp("a", "v")` throws a `SyntaxError` with the exact
/// message of the refusal and no position after it, caught by the script, and moves
/// `refusals()` by 1; `new RegExp("(")`, `new RegExp("a", "gg")`, `new RegExp("a",
/// "g").test("a")`, `new RegExp("\\p{L}", "u").test("a")`, `new RegExp("a", "i").test("A")`,
/// `/a/i.test("A")` and `new RegExp("(?<\\u{e9}>a)").exec("a").groups["é"]`, a property escape,
/// `i` and a group name outside ASCII answered once their stories landed, leave the count
/// unchanged, and so does an error of ToString of the pattern, which is thrown instead of the
/// refusal.
#[test]
fn host_191_regexp_counts_its_refusals_of_slice_5() {
    let mut agent = agent_with(limits()).unwrap();
    let realm = realm(&mut agent);
    assert_eq!(agent.refusals(), 0);
    let construction = r#"new RegExp("a", "v")"#;
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            &throws(
                construction,
                "class set expressions of the v flag is not supported before slice 5"
            )
        ),
        yes(),
        "{construction}"
    );
    assert_eq!(agent.refusals(), 1, "{construction}");
    for script in [
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
           try { new RegExp(t, "i"); false; } catch (e) { e === 7; }"#,
        r#"try { new RegExp("a", "ii"); false; } catch (e) { e instanceof SyntaxError &&
             e.message === "repeated regular expression flag"; }"#,
        r#"try { new RegExp("(", "i"); false; } catch (e) { e instanceof SyntaxError &&
             e.message === "unterminated group"; }"#,
    ] {
        assert_eq!(eval_text(&mut agent, &realm, script), yes(), "{script}");
        assert_eq!(agent.refusals(), 1, "{script}");
    }
    assert_eq!(
        eval_text(&mut agent, &realm, "new RegExp(\"a\", \"v\");"),
        Outcome::Throw("SyntaxError".to_string())
    );
    assert_eq!(agent.refusals(), 2, "uncaught");
}
