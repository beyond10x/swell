//! The methods of `String.prototype` that make a String from a String or compare two (ECMA-262
//! 22.1.3, `#sec-properties-of-the-string-prototype-object`; `story:string-transform-and-compare-methods`):
//! `localeCompare` (`#sec-string.prototype.localecompare`), `padEnd`
//! (`#sec-string.prototype.padend`), `padStart` (`#sec-string.prototype.padstart`) with
//! StringPaddingBuiltinsImpl and StringPad (`#sec-stringpad`), `repeat`
//! (`#sec-string.prototype.repeat`), the four case methods (`#sec-string.prototype.tolowercase`,
//! `#sec-string.prototype.touppercase`, `#sec-string.prototype.tolocalelowercase`,
//! `#sec-string.prototype.tolocaleuppercase`) and the trims (`#sec-string.prototype.trim`,
//! `#sec-string.prototype.trimend`, `#sec-string.prototype.trimstart`) with TrimString
//! (`#sec-trimstring`). Before slice 5 the case methods and `localeCompare` refuse a String with a
//! code unit of 0x80 or above (`docs/design/engine.md`, section 12, "A part before its slice";
//! `docs/design/host-interface.md`, section 13, "A refusal raised by a builtin method", `HOST-165`).
//! A String made longer than `string_length` is a `RangeError` (`HOST-21`).
//!
//! The scripts call no builtin another story of the wave delivers: a code unit is written as a
//! `\u` escape in a literal, a result is compared by `===` with a literal and by `length`, and a
//! long String is made with `repeat`.
#![cfg(test)]

mod support;

use support::host::{agent_with, get_object, global, key, limits, object, realm, stack_with_room};
use support::script::{Outcome, eval, eval_text, run, run_both};
use support::stack::{
    BUILTINS_REENTRY, is_range_error_of, on_bounded_stack, reach_through, stack_limits,
};
use swell::{Agent, Limits, Outcome as HostOutcome, Termination, Value};

/// `true`, as an outcome.
fn yes() -> Outcome {
    Outcome::Normal("true".to_string())
}

/// Each script answers `true`, as written and with a `"use strict";` prologue.
fn all_true(scripts: &[&str]) {
    for script in scripts {
        let (sloppy, strict) = run_both(script);
        assert_eq!(sloppy, yes(), "{script}");
        assert_eq!(strict, yes(), "strict: {script}");
    }
}

/// The script that answers whether `String.prototype[name]` is a method as clause 18 makes
/// one: a function in a data property { [[Writable]]: true, [[Enumerable]]: false,
/// [[Configurable]]: true }, whose "length" is `length` and "name" is `name`, with no
/// "prototype" and no [[Construct]]; and whether a call of it with an undefined or null this
/// value throws a TypeError (RequireObjectCoercible, `#sec-requireobjectcoercible`).
fn method_script(name: &str, length: u32) -> String {
    format!(
        "var d = Object.getOwnPropertyDescriptor(String.prototype, \"{name}\");\n\
         var m = d.value;\n\
         var refused = function (thisValue) {{\n\
           try {{ m.call(thisValue); return false; }} catch (e) {{ return e instanceof TypeError; }}\n\
         }};\n\
         var constructed;\n\
         try {{ new m(); constructed = true; }} catch (e) {{ constructed = !(e instanceof TypeError); }}\n\
         typeof m === \"function\" && d.writable === true && d.enumerable === false &&\n\
           d.configurable === true && m.length === {length} && m.name === \"{name}\" &&\n\
           !m.hasOwnProperty(\"prototype\") && !constructed && refused(undefined) && refused(null);"
    )
}

/// The property of the method `name` and its first steps, in both modes.
fn check_method(name: &str, length: u32) {
    all_true(&[&method_script(name, length)]);
}

/// An agent with `limits` and a realm.
fn agent_and_realm(limits: Limits) -> (Agent<support::host::TestHost>, swell::Realm) {
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    (agent, realm)
}

/// The steps `source` spends on a new agent with the test limits, read before anything else
/// runs: a conversion through the host interface (as [`eval_text`] makes of a Number) is an
/// entry of its own.
fn steps_of(source: &str) -> u64 {
    let (mut agent, realm) = agent_and_realm(limits());
    let outcome = eval(&mut agent, &realm, source);
    let steps = agent.last_entry_steps().unwrap();
    assert!(
        matches!(outcome, HostOutcome::Normal(_)),
        "{source}: {outcome:?}"
    );
    steps
}

/// The code units 0 to 0x7F, each as a `\u` escape.
fn ascii_escapes(map: fn(u8) -> u8) -> String {
    (0_u8..0x80)
        .map(|unit| format!("\\u{:04x}", map(unit)))
        .collect()
}

/// `#sec-string.prototype.localecompare`: a method of length 1; the this value and `that` are
/// converted in that order, and an error of the first conversion is thrown before the second.
#[test]
fn locale_compare_property_and_steps() {
    check_method("localeCompare", 1);
    all_true(&[
        "\"a\".localeCompare(\"b\") === -1 && String.prototype.localeCompare.call(2, 1) === 1",
        "var log = \"\";\n\
         var r = String.prototype.localeCompare.call(\n\
           { toString: function () { log += \"this\"; return \"a\"; } },\n\
           { toString: function () { log += \",that\"; return \"a\"; } });\n\
         log === \"this,that\" && r === 0",
        "var log = \"\";\n\
         try {\n\
           String.prototype.localeCompare.call({ toString: function () { throw 7; } },\n\
             { toString: function () { log += \"that\"; return \"a\"; } });\n\
         } catch (e) { log += e; }\n\
         log === \"7\"",
    ]);
}

/// `#sec-string.prototype.padend` with StringPaddingBuiltinsImpl and StringPad: the filler
/// after the String, a space by default, truncated to the length asked; the String itself when
/// the length asked is not more than its own or the filler is empty; ToString of the this
/// value, ToLength of maxLength and ToString of fillString in that order, the last only when
/// the String is padded.
#[test]
fn pad_end_property_and_steps() {
    check_method("padEnd", 1);
    all_true(&[
        "\"abc\".padEnd(10, \"123\") === \"abc1231231\" && \"abc\".padEnd(5) === \"abc  \" &&\n\
         \"abc\".padEnd(5, undefined) === \"abc  \" && \"abc\".padEnd(6, null) === \"abcnul\"",
        "\"abc\".padEnd(5, \"\") === \"abc\" && \"abc\".padEnd(3, \"x\") === \"abc\" &&\n\
         \"abc\".padEnd(-1, \"x\") === \"abc\" && \"abc\".padEnd(NaN, \"x\") === \"abc\" &&\n\
         \"abc\".padEnd(4.9, \"x\") === \"abcx\" && \"abc\".padEnd(\"5\", \"x\") === \"abcxx\" &&\n\
         \"\".padEnd(2, \"ab\") === \"ab\" && String.prototype.padEnd.call(1, 3, 0) === \"100\"",
        "var log = \"\";\n\
         var r = String.prototype.padEnd.call(\n\
           { toString: function () { log += \"this\"; return \"a\"; } },\n\
           { valueOf: function () { log += \",max\"; return 3; } },\n\
           { toString: function () { log += \",fill\"; return \"-\"; } });\n\
         log === \"this,max,fill\" && r === \"a--\"",
        "var log = \"\";\n\
         var r = \"abc\".padEnd(3, { toString: function () { log += \"fill\"; return \"-\"; } });\n\
         log === \"\" && r === \"abc\"",
    ]);
}

/// `#sec-string.prototype.padstart`: as `padEnd`, with the filler before the String.
#[test]
fn pad_start_property_and_steps() {
    check_method("padStart", 1);
    all_true(&[
        "\"abc\".padStart(10, \"123\") === \"1231231abc\" && \"abc\".padStart(5) === \"  abc\" &&\n\
         \"abc\".padStart(5, undefined) === \"  abc\" && \"abc\".padStart(6, null) === \"nulabc\"",
        "\"abc\".padStart(5, \"\") === \"abc\" && \"abc\".padStart(2, \"x\") === \"abc\" &&\n\
         \"abc\".padStart(-Infinity, \"x\") === \"abc\" && \"abc\".padStart(4.9, \"x\") === \"xabc\" &&\n\
         String.prototype.padStart.call(1, 3, 0) === \"001\"",
        "var log = \"\";\n\
         var r = String.prototype.padStart.call(\n\
           { toString: function () { log += \"this\"; return \"a\"; } },\n\
           { valueOf: function () { log += \",max\"; return 3; } },\n\
           { toString: function () { log += \",fill\"; return \"-\"; } });\n\
         log === \"this,max,fill\" && r === \"--a\"",
    ]);
}

/// `#sec-string.prototype.repeat`: n copies of the String; a count that is negative or +∞ is a
/// RangeError, also for the empty String; a count of 0 (NaN, undefined, a fraction below 1)
/// answers the empty String; ToString of the this value comes before ToIntegerOrInfinity of the
/// count.
#[test]
fn repeat_property_and_steps() {
    check_method("repeat", 1);
    all_true(&[
        "\"ab\".repeat(3) === \"ababab\" && \"ab\".repeat(1) === \"ab\" && \"ab\".repeat(0) === \"\" &&\n\
         \"ab\".repeat(-0.5) === \"\" && \"ab\".repeat(NaN) === \"\" && \"ab\".repeat() === \"\" &&\n\
         \"ab\".repeat(2.9) === \"abab\" && \"ab\".repeat(\"2\") === \"abab\" && \"\".repeat(3) === \"\" &&\n\
         String.prototype.repeat.call(12, 2) === \"1212\"",
        "var refused = function (count) {\n\
           try { \"ab\".repeat(count); return false; } catch (e) { return e instanceof RangeError; }\n\
         };\n\
         refused(-1) && refused(Infinity) && refused(-Infinity) && !refused(0)",
        "var log = \"\";\n\
         var r = String.prototype.repeat.call(\n\
           { toString: function () { log += \"this\"; return \"x\"; } },\n\
           { valueOf: function () { log += \",count\"; return 2; } });\n\
         log === \"this,count\" && r === \"xx\"",
    ]);
}

/// `#sec-string.prototype.tolocalelowercase`: as `toLowerCase` without ECMA-402; its arguments
/// have no meaning.
#[test]
fn to_locale_lower_case_property_and_steps() {
    check_method("toLocaleLowerCase", 0);
    all_true(&[
        "\"AbC1-z\".toLocaleLowerCase() === \"abc1-z\" && \"I\".toLocaleLowerCase(\"tr\") === \"i\" &&\n\
         String.prototype.toLocaleLowerCase.call(true) === \"true\" && \"\".toLocaleLowerCase() === \"\"",
    ]);
}

/// `#sec-string.prototype.tolocaleuppercase`: as `toUpperCase` without ECMA-402; its arguments
/// have no meaning.
#[test]
fn to_locale_upper_case_property_and_steps() {
    check_method("toLocaleUpperCase", 0);
    all_true(&[
        "\"AbC1-z\".toLocaleUpperCase() === \"ABC1-Z\" && \"i\".toLocaleUpperCase(\"tr\") === \"I\" &&\n\
         String.prototype.toLocaleUpperCase.call(false) === \"FALSE\"",
    ]);
}

/// `#sec-string.prototype.tolowercase`: the String with every upper-case letter mapped.
#[test]
fn to_lower_case_property_and_steps() {
    check_method("toLowerCase", 0);
    all_true(&[
        "\"AbC1-z\".toLowerCase() === \"abc1-z\" && String.prototype.toLowerCase.call(1e21) === \"1e+21\" &&\n\
         String.prototype.toLowerCase.call({ toString: function () { return \"XY\"; } }) === \"xy\"",
    ]);
}

/// `#sec-string.prototype.touppercase`: the String with every lower-case letter mapped.
#[test]
fn to_upper_case_property_and_steps() {
    check_method("toUpperCase", 0);
    all_true(&[
        "\"AbC1-z\".toUpperCase() === \"ABC1-Z\" && String.prototype.toUpperCase.call(NaN) === \"NAN\"",
    ]);
}

/// `#sec-string.prototype.trim` with TrimString: white space removed at both ends.
#[test]
fn trim_property_and_steps() {
    check_method("trim", 0);
    all_true(&[
        "\"  a b  \".trim() === \"a b\" && \"\".trim() === \"\" && \"   \".trim() === \"\" &&\n\
         String.prototype.trim.call(12) === \"12\" &&\n\
         String.prototype.trim.call({ toString: function () { return \" x \"; } }) === \"x\"",
    ]);
}

/// `#sec-string.prototype.trimend` with TrimString: white space removed at the end.
#[test]
fn trim_end_property_and_steps() {
    check_method("trimEnd", 0);
    all_true(&[
        "\"  a b  \".trimEnd() === \"  a b\" && \"   \".trimEnd() === \"\" &&\n\
         String.prototype.trimEnd.call(true) === \"true\"",
    ]);
}

/// `#sec-string.prototype.trimstart` with TrimString: white space removed at the start.
#[test]
fn trim_start_property_and_steps() {
    check_method("trimStart", 0);
    all_true(&[
        "\"  a b  \".trimStart() === \"a b  \" && \"   \".trimStart() === \"\" &&\n\
         String.prototype.trimStart.call(null === null) === \"true\"",
    ]);
}

/// `HOST-21`: with `string_length` 1,000, `"x".repeat(1000)` is a String of length 1,000 and
/// `"x".repeat(1001)` throws a `RangeError` of the realm; `"".repeat(Infinity)` throws a
/// `RangeError` by the clause (`#sec-string.prototype.repeat`, step 5).
#[test]
fn host_21_repeat_at_and_past_string_length() {
    let mut limits = limits();
    limits.string_length = 1_000;
    let (mut agent, realm) = agent_and_realm(limits);
    assert_eq!(
        eval_text(&mut agent, &realm, "\"x\".repeat(1000).length;"),
        Outcome::Normal("1000".to_string())
    );
    let past = eval(&mut agent, &realm, "\"x\".repeat(1001);");
    assert!(is_range_error_of(&mut agent, &realm, past));
    let infinite = eval(&mut agent, &realm, "\"\".repeat(Infinity);");
    assert!(is_range_error_of(&mut agent, &realm, infinite));
    assert_eq!(
        eval_text(&mut agent, &realm, "\"\".repeat(1001).length;"),
        Outcome::Normal("0".to_string())
    );
}

/// The steps a run that refuses a long String before a copy may spend: the instructions of the
/// script and nothing per code unit (measured: 8 in each of the four runs).
const STEPS_BEFORE_THE_COPY: u64 = 20;

/// The epic's acceptance 5: `"x".repeat(1073741824)` (2^30) under `string_length` 2^30 and a
/// memory budget of 64 MiB ends in `Terminated(MemoryBudget)`, under `string_length` 2^20 in a
/// `RangeError`; neither copies a code unit, so each spends no more than
/// [`STEPS_BEFORE_THE_COPY`]. `padEnd` to the same length asks the memory budget the same way.
#[test]
fn repeat_of_2_30_ends_in_range_error_or_memory_budget() {
    let mut limits = limits();
    limits.string_length = 1 << 30;
    limits.memory_bytes = 64 * 1024 * 1024;
    for source in ["\"x\".repeat(1073741824);", "\"x\".padEnd(1073741824);"] {
        let (mut agent, realm) = agent_and_realm(limits);
        assert_eq!(
            eval_text(&mut agent, &realm, source),
            Outcome::Terminated(Termination::MemoryBudget),
            "{source}"
        );
        let steps = agent.last_entry_steps().unwrap();
        println!("{source}: {steps} steps");
        assert!(steps <= STEPS_BEFORE_THE_COPY, "{source}: {steps} steps");
    }
    limits.string_length = 1 << 20;
    for source in ["\"x\".repeat(1073741824);", "\"x\".padStart(1073741824);"] {
        let (mut agent, realm) = agent_and_realm(limits);
        let outcome = eval(&mut agent, &realm, source);
        // Read before the error is looked at: a conversion through the host interface is an
        // entry of its own.
        let steps = agent.last_entry_steps().unwrap();
        assert!(is_range_error_of(&mut agent, &realm, outcome), "{source}");
        println!("{source}: {steps} steps");
        assert!(steps <= STEPS_BEFORE_THE_COPY, "{source}: {steps} steps");
    }
}

/// StringPad (`#sec-stringpad`): with `string_length` 1,000 a result of 1,000 code units is
/// made and one of 1,001 is a `RangeError` of the realm, for both placements; the filler is
/// repeated and cut to the length asked; an empty filler answers the String, however long the
/// length asked.
#[test]
fn pad_refuses_a_result_past_string_length_and_truncates_the_filler() {
    let mut limits = limits();
    limits.string_length = 1_000;
    let (mut agent, realm) = agent_and_realm(limits);
    for source in [
        "\"a\".padStart(1000, \"xy\").length === 1000 && \"a\".padEnd(1000).length === 1000",
        "\"abc\".padEnd(6, \"12345\") === \"abc123\" && \"abc\".padStart(6, \"12345\") === \"123abc\"",
        "\"abc\".padEnd(8, \"12\") === \"abc12121\" && \"abc\".padStart(8, \"12\") === \"12121abc\"",
        "\"abc\".padEnd(9007199254740991, \"\") === \"abc\" && \"abc\".padStart(1001, \"\") === \"abc\"",
    ] {
        assert_eq!(eval_text(&mut agent, &realm, source), yes(), "{source}");
    }
    for source in [
        "\"a\".padStart(1001);",
        "\"a\".padEnd(1001, \"xy\");",
        "\"a\".padEnd(Infinity);",
    ] {
        let outcome = eval(&mut agent, &realm, source);
        assert!(is_range_error_of(&mut agent, &realm, outcome), "{source}");
    }
}

/// The code points of WhiteSpace (`#sec-white-space`: TAB, VT, FF, ZWNBSP and every code point
/// of `Zs`, the 17 that `UnicodeData.txt` of `data/unicode/17.0.0/ucd/` lists) and of
/// LineTerminator (`#sec-line-terminators`: LF, CR, LS, PS).
const WHITE_SPACE: [u16; 25] = [
    0x0009, 0x000B, 0x000C, 0xFEFF, 0x0020, 0x00A0, 0x1680, 0x2000, 0x2001, 0x2002, 0x2003, 0x2004,
    0x2005, 0x2006, 0x2007, 0x2008, 0x2009, 0x200A, 0x202F, 0x205F, 0x3000, 0x000A, 0x000D, 0x2028,
    0x2029,
];

/// Code points next to white space that are not: U+180E MONGOLIAN VOWEL SEPARATOR (`Cf` since
/// Unicode 6.3), U+200B ZERO WIDTH SPACE (`Cf`), U+0085 NEXT LINE (`Cc`, no LineTerminator in
/// ECMA-262), U+001F, U+0021, and a lone surrogate.
const NOT_WHITE_SPACE: [u16; 6] = [0x180E, 0x200B, 0x0085, 0x001F, 0x0021, 0xD800];

/// TrimString (`#sec-trimstring`): each code point of WhiteSpace and LineTerminator is removed
/// at the start (`trimStart`), at the end (`trimEnd`) or at both (`trim`), and kept inside;
/// all of them at once are removed; U+180E and the others of [`NOT_WHITE_SPACE`] are kept.
#[test]
fn trim_removes_every_white_space_and_line_terminator_and_nothing_else() {
    let (mut agent, realm) = agent_and_realm(limits());
    for unit in WHITE_SPACE {
        let w = format!("\\u{unit:04x}");
        let source = format!(
            "var s = \"{w}{w}a{w}b{w}\";\n\
             s.trim() === \"a{w}b\" && s.trimStart() === \"a{w}b{w}\" && s.trimEnd() === \"{w}{w}a{w}b\""
        );
        assert_eq!(
            eval_text(&mut agent, &realm, &source),
            yes(),
            "U+{unit:04X}"
        );
    }
    let all: String = WHITE_SPACE
        .iter()
        .map(|unit| format!("\\u{unit:04x}"))
        .collect();
    let source = format!(
        "var s = \"{all}a{all}b{all}\";\n\
         s.trim() === \"a{all}b\" && s.trim().length === {} && \"{all}\".trim() === \"\"",
        WHITE_SPACE.len() + 2
    );
    assert_eq!(eval_text(&mut agent, &realm, &source), yes(), "all");
    for unit in NOT_WHITE_SPACE {
        let n = format!("\\u{unit:04x}");
        let source = format!(
            "var s = \"{n}a{n}\";\n\
             s.trim() === s && s.trimStart() === s && s.trimEnd() === s && s.trim().length === 3"
        );
        assert_eq!(
            eval_text(&mut agent, &realm, &source),
            yes(),
            "U+{unit:04X}"
        );
    }
}

/// The four case methods on every code unit below 0x80: `toLowerCase` and `toLocaleLowerCase`
/// map the 26 letters A to Z to a to z, `toUpperCase` and `toLocaleUpperCase` a to z to A to
/// Z, and every other unit is kept (`#sec-string.prototype.tolowercase`: the case mappings of
/// the Unicode Character Database, which below 0x80 map the 26 letters and nothing else).
#[test]
fn case_methods_map_the_26_letters_and_keep_every_other_ascii_unit() {
    let every = ascii_escapes(|unit| unit);
    let lower = ascii_escapes(|unit| unit.to_ascii_lowercase());
    let upper = ascii_escapes(|unit| unit.to_ascii_uppercase());
    let source = format!(
        "var s = \"{every}\";\n\
         s.toLowerCase() === \"{lower}\" && s.toLocaleLowerCase() === \"{lower}\" &&\n\
         s.toUpperCase() === \"{upper}\" && s.toLocaleUpperCase() === \"{upper}\" &&\n\
         s.toLowerCase().length === 128 && s.toUpperCase().length === 128 &&\n\
         \"{lower}\".toUpperCase() === \"{upper}\" && \"{upper}\".toLowerCase() === \"{lower}\""
    );
    let (sloppy, strict) = run_both(&source);
    assert_eq!(sloppy, yes());
    assert_eq!(strict, yes());
    let (mut agent, realm) = agent_and_realm(limits());
    assert_eq!(eval_text(&mut agent, &realm, &source), yes());
    assert_eq!(agent.refusals(), 0);
}

/// The message of the refusal of the case methods (`host-interface.md`, section 13).
const CASE_REFUSAL: &str = "case mapping outside ASCII is not supported before slice 5";

/// The message of the refusal of `localeCompare`.
const COMPARE_REFUSAL: &str = "comparison outside ASCII is not supported before slice 5";

/// `HOST-165`: on a new agent, each of the four case methods on `"é"` and `localeCompare` with
/// `"é"` as the this value and as `that` throws a `SyntaxError` the script catches, with the
/// exact message, and `refusals()` moves by 1 each time; the prototype of the error is
/// `%SyntaxError.prototype%` of the method's realm, also when the method of another realm is
/// called; `swell_syntax::message_kind` reads the message as a refusal of slice 5; an uncaught
/// refusal is counted too; the ASCII calls, a code unit of 0x7F, and a `toString` that throws
/// (whose value is thrown instead) leave the count unchanged.
#[test]
fn host_165_case_methods_and_locale_compare_refuse_a_unit_outside_ascii() {
    let (mut agent, realm) = agent_and_realm(limits());
    let other_realm = support::host::realm(&mut agent);
    assert_eq!(agent.refusals(), 0);
    let mut count = 0;
    let mut refused = |agent: &mut Agent<_>, source: &str, expected: Outcome| {
        assert_eq!(eval_text(agent, &realm, source), expected, "{source}");
        count += 1;
        assert_eq!(agent.refusals(), count, "{source}");
    };
    for method in [
        "toLowerCase",
        "toUpperCase",
        "toLocaleLowerCase",
        "toLocaleUpperCase",
    ] {
        refused(
            &mut agent,
            &format!(
                "try {{ \"é\".{method}(); false }} catch (e) {{ e instanceof SyntaxError && e.message === \"{CASE_REFUSAL}\" }}"
            ),
            yes(),
        );
    }
    for call in ["\"a\".localeCompare(\"é\")", "\"é\".localeCompare(\"a\")"] {
        refused(
            &mut agent,
            &format!(
                "try {{ {call}; false }} catch (e) {{ e instanceof SyntaxError && e.message === \"{COMPARE_REFUSAL}\" }}"
            ),
            yes(),
        );
    }
    // The prototype of the error, the message with nothing after it, and its kind.
    let case_message = Outcome::Normal(format!("\"{CASE_REFUSAL}\""));
    for source in [
        "try { \"\\u0080\".toLowerCase(); } catch (e) { Object.getPrototypeOf(e) === SyntaxError.prototype && e.message; }",
        "try { \"a\\ud83d\\ude00\".toUpperCase(); } catch (e) { Object.getPrototypeOf(e) === SyntaxError.prototype && e.message; }",
        "try { String.prototype.toLocaleUpperCase.call({ toString: function () { return \"\\u00ff\"; } }); } catch (e) { e.message; }",
    ] {
        refused(&mut agent, source, case_message.clone());
    }
    refused(
        &mut agent,
        "try { \"a\".localeCompare({ toString: function () { return \"\\u0100\"; } }); } catch (e) { e.message; }",
        Outcome::Normal(format!("\"{COMPARE_REFUSAL}\"")),
    );
    for message in [CASE_REFUSAL, COMPARE_REFUSAL] {
        assert_eq!(
            swell_syntax::message_kind(message),
            swell_syntax::MessageKind::Refusal { slice: 5 },
            "{message}"
        );
    }
    // Uncaught: the run throws the SyntaxError, counted all the same.
    refused(
        &mut agent,
        "\"\\u00e9\".toLocaleLowerCase();",
        Outcome::Throw("SyntaxError".to_string()),
    );
    // The methods of another realm throw an error of their own realm.
    let global_root = global(&mut agent, &realm);
    let other_global_root = global(&mut agent, &other_realm);
    agent
        .with_cx(&realm, |cx| {
            let global = object(cx, &global_root);
            let other_global = object(cx, &other_global_root);
            for (name, path) in [
                ("otherStringPrototype", "String"),
                ("OtherSyntaxError", "SyntaxError"),
            ] {
                let constructor = get_object(cx, other_global, path);
                let value = if path == "String" {
                    Value::Object(get_object(cx, constructor, "prototype"))
                } else {
                    Value::Object(constructor)
                };
                let name = key(cx, name);
                cx.set(global, name, value, true).unwrap().unwrap();
            }
        })
        .unwrap();
    for call in [
        "otherStringPrototype.toLowerCase.call(\"\\u00e9\")",
        "otherStringPrototype.localeCompare.call(\"a\", \"\\u00e9\")",
    ] {
        refused(
            &mut agent,
            &format!(
                "try {{ {call}; false }} catch (e) {{\n\
                   Object.getPrototypeOf(e) === OtherSyntaxError.prototype &&\n\
                   OtherSyntaxError.prototype !== SyntaxError.prototype && !(e instanceof SyntaxError)\n\
                 }}"
            ),
            yes(),
        );
    }
    // Not refused: the ASCII calls of the requirement, a unit of 0x7F, and a ToString that
    // throws before the refusal would be reached.
    for source in [
        "\"AbC\".toLowerCase() === \"abc\" && \"AbC\".toUpperCase() === \"ABC\" && \"a\".localeCompare(\"b\") < 0",
        "\"\\u007f\".toLowerCase() === \"\\u007f\" && \"\\u007f\".localeCompare(\"\\u007f\") === 0",
        "try { String.prototype.toLowerCase.call({ toString: function () { throw 1; } }); } catch (e) { e === 1 }",
        "try { \"\\u00e9\".localeCompare({ toString: function () { throw 1; } }); } catch (e) { e === 1 }",
    ] {
        assert_eq!(eval_text(&mut agent, &realm, source), yes(), "{source}");
        assert_eq!(agent.refusals(), count, "{source}");
    }
}

/// `#sec-string.prototype.localecompare` for Strings of code units below 0x80: their order of
/// code units, -1, +0 or 1; a String before every longer String it starts; the second and
/// third arguments have no meaning; `that` undefined is the String "undefined".
#[test]
fn locale_compare_orders_ascii_strings_by_code_unit() {
    all_true(&[
        "\"a\".localeCompare(\"b\") === -1 && \"b\".localeCompare(\"a\") === 1 &&\n\
         1 / \"a\".localeCompare(\"a\") === Infinity && 1 / \"\".localeCompare(\"\") === Infinity",
        "\"A\".localeCompare(\"a\") === -1 && \"Z\".localeCompare(\"a\") === -1 &&\n\
         \"a\".localeCompare(\"ab\") === -1 && \"ab\".localeCompare(\"a\") === 1 &&\n\
         \"\".localeCompare(\"\\u0000\") === -1 && \"abd\".localeCompare(\"abc\") === 1",
        "\"10\".localeCompare(\"9\") === -1 && \"\\u007f\".localeCompare(\"~\") === 1 &&\n\
         \"a\".localeCompare(\"b\", \"tr\", { sensitivity: \"base\" }) === -1",
        "\"undefined\".localeCompare() === 0 && \"null\".localeCompare(null) === 0 &&\n\
         \"a\".localeCompare() === -1",
    ]);
}

/// Steps per code unit: `repeat` and `padStart` charge a step per code unit they copy; the
/// trims, the case methods and `localeCompare` a step per code unit they read, every unit of
/// each String; the trims and the case methods a step per code unit they copy into the
/// result as well, and a trim that removes nothing copies nothing.
#[test]
fn repeat_and_trim_charge_a_step_per_code_unit() {
    // The extra steps of `n` = 2,000 over `n` = 1,000.
    let extra = |template: &str| {
        steps_of(&template.replace("N", "2000")) - steps_of(&template.replace("N", "1000"))
    };
    // 2 units per copy.
    assert_eq!(extra("\"ab\".repeat(N);"), 2_000);
    assert_eq!(extra("\"\".padStart(N, \"ab\");"), 1_000);
    // On top of the repeat: read every unit, copy nothing.
    assert_eq!(extra("\" \".repeat(N).trim();"), 2_000);
    assert_eq!(extra("\"x\".repeat(N).trim();"), 2_000);
    // Read 2 units per copy, copy all but the leading space.
    assert_eq!(extra("\" x\".repeat(N).trimStart();"), 6_000);
    assert_eq!(extra("\"x \".repeat(N).trimEnd();"), 6_000);
    // Read and copy each unit.
    assert_eq!(extra("\"Ab\".repeat(N).toLowerCase();"), 6_000);
    // Read the units of both Strings.
    assert_eq!(extra("var s = \"a\".repeat(N); s.localeCompare(s);"), 3_000);
}

/// A `repeat` is ended by the step budget: with `steps_per_entry` 1,000, `"x".repeat(900)` is
/// made and `"x".repeat(5000)` ends in `Terminated(StepBudget)` with no more than 1,000 steps
/// spent.
#[test]
fn step_budget_ends_a_repeat() {
    let mut limits = limits();
    limits.steps_per_entry = 1_000;
    let (mut agent, realm) = agent_and_realm(limits);
    assert_eq!(
        eval_text(&mut agent, &realm, "\"x\".repeat(900).length;"),
        Outcome::Normal("900".to_string())
    );
    assert_eq!(
        eval_text(&mut agent, &realm, "\"x\".repeat(5000);"),
        Outcome::Terminated(Termination::StepBudget)
    );
    assert!(agent.last_entry_steps().unwrap() <= 1_000);
}

/// The two re-entries through a conversion of an argument of this story's methods: ToString of
/// the fillString of `padStart` (StringPaddingBuiltinsImpl, step 5) and of `that` of
/// `localeCompare`.
const THROUGH_CONVERSIONS: [&str; 2] = [
    "\"a\".padStart(2, { toString: f })",
    "\"a\".localeCompare({ toString: f })",
];

/// The body of the stack test: one agent per expression.
fn reach_through_conversions() {
    for through in THROUGH_CONVERSIONS {
        reach_through(through, BUILTINS_REENTRY);
    }
}

/// On a thread with seven eighths of `min_stack_bytes()` for the stack limits, a script reaches
/// the re-entry limit through ToString of the filler of `padStart` and of `that` of
/// `localeCompare`, and parses 1,000 nested brackets at the deepest, without a stack overflow
/// (`tests/support/stack.rs`, [`reach_through`]).
#[test]
fn conversions_reach_the_reentry_limit_on_the_promised_stack() {
    let stack = stack_with_room(stack_limits(BUILTINS_REENTRY));
    on_bounded_stack(
        "conversions_reach_the_reentry_limit_on_the_promised_stack",
        stack,
        reach_through_conversions,
    )
    .unwrap();
}

/// Each expression of [`THROUGH_CONVERSIONS`] with a small `reentry_depth`: the run reaches the
/// bound exactly, so the stack test measures what it claims.
#[test]
fn each_conversion_reentry_counts_against_reentry_depth() {
    for depth in [0_u32, 1, 7] {
        for through in THROUGH_CONVERSIONS {
            reach_through(through, depth);
        }
    }
}

/// A run that ends in a `RangeError` of the realm, as a script reads it: also caught.
#[test]
fn range_errors_are_of_the_realm() {
    let mut limits = limits();
    limits.string_length = 20;
    assert_eq!(
        run(
            limits,
            "var r = [];\n\
             try { \"ab\".repeat(11); } catch (e) { r.push(Object.getPrototypeOf(e) === RangeError.prototype); }\n\
             try { \"ab\".padEnd(21); } catch (e) { r.push(e instanceof RangeError); }\n\
             try { \"ab\".repeat(-1); } catch (e) { r.push(e instanceof RangeError); }\n\
             r.join() === \"true,true,true\" && \"ab\".repeat(10).length === 20"
        ),
        yes()
    );
}
