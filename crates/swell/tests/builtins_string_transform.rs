//! The methods of `String.prototype` that make a String from a String or compare two (ECMA-262
//! 22.1.3, `#sec-properties-of-the-string-prototype-object`; `story:string-transform-and-compare-methods`):
//! `localeCompare` (`#sec-string.prototype.localecompare`), `padEnd`
//! (`#sec-string.prototype.padend`), `padStart` (`#sec-string.prototype.padstart`) with
//! StringPaddingBuiltinsImpl and StringPad (`#sec-stringpad`), `repeat`
//! (`#sec-string.prototype.repeat`), the four case methods (`#sec-string.prototype.tolowercase`,
//! `#sec-string.prototype.touppercase`, `#sec-string.prototype.tolocalelowercase`,
//! `#sec-string.prototype.tolocaleuppercase`) and the trims (`#sec-string.prototype.trim`,
//! `#sec-string.prototype.trimend`, `#sec-string.prototype.trimstart`) with TrimString
//! (`#sec-trimstring`), and `normalize` (`#sec-string.prototype.normalize`). The case methods
//! map every code point by the case tables of `swell-unicode`
//! (`story:case-mapping-tables-and-string-case-methods`); `normalize` makes the four forms of
//! UAX #15 by its normalization tables, and `localeCompare` orders every String by the code
//! units of its NFC form (`story:normalization-tables-normalize-and-locale-compare`;
//! `docs/design/engine.md`, decision 5, "The order `localeCompare` gives"): no method of the
//! group refuses an input (`HOST-165`). A String made longer than `string_length` is a
//! `RangeError` (`HOST-21`).
//!
//! The scripts call no builtin another story of the wave delivers: a code unit is written as a
//! `\u` escape in a literal, a result is compared by `===` with a literal and by `length`, and a
//! long String is made with `repeat`.
#![cfg(test)]

use crate::support;

use support::host::{agent_with, get_object, global, key, limits, object, realm, stack_with_room};
use support::script::{Outcome, describe, eval, eval_text, parse, run, run_both};
use support::stack::{
    BUILTINS_REENTRY, is_range_error_of, on_bounded_stack, reach_through, stack_limits, test_path,
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

/// The four case methods outside ASCII (`#sec-string.prototype.tolowercase`,
/// `#sec-string.prototype.touppercase`): the String read as code points (StringToCodePoints,
/// `#sec-stringtocodepoints`), each mapped by the full mappings of `SpecialCasing.txt` that need
/// no language, else by `UnicodeData.txt`; U+03A3 lowercased to U+03C2 where `Final_Sigma`
/// holds (Unicode chapter 3, table 3-17: a cased letter before it after any case-ignorable
/// code points, and no cased letter after it after any case-ignorable ones), else to U+03C3; a
/// supplementary-plane letter mapped as one code point; a lone surrogate kept. The locale
/// methods answer the same.
#[test]
fn case_methods_map_outside_ascii() {
    all_true(&[
        // U+0130 to i and U+0307; U+00DF to SS; the ligature U+FB03 to FFI.
        "\"\\u0130\".toLowerCase() === \"i\\u0307\" && \"\\u0130\".toLowerCase().length === 2 &&\n\
         \"\\u00df\".toUpperCase() === \"SS\" && \"\\u00df\".toLowerCase() === \"\\u00df\" &&\n\
         \"\\ufb03\".toUpperCase() === \"FFI\" && \"\\u0390\".toUpperCase() === \"\\u0399\\u0308\\u0301\" &&\n\
         \"\\u00e9\\u00c9\".toUpperCase() === \"\\u00c9\\u00c9\" && \"\\u00e9\\u00c9\".toLowerCase() === \"\\u00e9\\u00e9\"",
        // Final_Sigma.
        "\"\\u0391\\u03a3\".toLowerCase() === \"\\u03b1\\u03c2\" && \"\\u03a3\".toLowerCase() === \"\\u03c3\" &&\n\
         \"\\u0391\\u03a3\\u0391\".toLowerCase() === \"\\u03b1\\u03c3\\u03b1\" &&\n\
         \"\\u0391.\\u03a3\".toLowerCase() === \"\\u03b1.\\u03c2\" &&\n\
         \"\\u0391\\u03a3.\\u0391\".toLowerCase() === \"\\u03b1\\u03c3.\\u03b1\" &&\n\
         \"\\u0391\\u03a3\\u180e\".toLowerCase() === \"\\u03b1\\u03c2\\u180e\" &&\n\
         \"\\u0345\\u03a3\".toLowerCase() === \"\\u0345\\u03c3\" &&\n\
         \"\\u0391\\u0345\\u03a3\".toLowerCase() === \"\\u03b1\\u0345\\u03c2\" &&\n\
         \"\\ud835\\udca2\\u03a3\".toLowerCase() === \"\\ud835\\udca2\\u03c2\" &&\n\
         \"\\u0391\\u03a3\\ud835\\udca2\".toLowerCase() === \"\\u03b1\\u03c3\\ud835\\udca2\" &&\n\
         \"\\u0391\\u03a3 \\u03a3\".toLowerCase() === \"\\u03b1\\u03c2 \\u03c3\" &&\n\
         \"\\u0391\\u03a3\".toUpperCase() === \"\\u0391\\u03a3\" && \"\\u03c2\".toUpperCase() === \"\\u03a3\"",
        // A supplementary-plane letter, and lone surrogates of both kinds.
        "\"\\ud801\\udc00\".toLowerCase() === \"\\ud801\\udc28\" &&\n\
         \"\\ud801\\udc28\".toUpperCase() === \"\\ud801\\udc00\" &&\n\
         \"\\ud800a\\udc00b\\ud800\".toUpperCase() === \"\\ud800A\\udc00B\\ud800\" &&\n\
         \"\\udc00\\ud801\".toLowerCase() === \"\\udc00\\ud801\" &&\n\
         \"\\ud801\\udc00\".toLowerCase().length === 2",
        // The locale methods, whose arguments have no meaning.
        "\"\\u0130\".toLocaleLowerCase() === \"i\\u0307\" && \"\\u00df\".toLocaleUpperCase() === \"SS\" &&\n\
         \"\\u0391\\u03a3\".toLocaleLowerCase(\"el\") === \"\\u03b1\\u03c2\" &&\n\
         \"I\".toLocaleLowerCase(\"tr\") === \"i\" && \"i\".toLocaleUpperCase(\"tr\") === \"I\"",
    ]);
}

/// A case method whose result would be longer than `string_length` throws a `RangeError` of
/// the realm (`HOST-21`): with `string_length` 20, `"ß".repeat(11)` maps to 22 code units in
/// upper case and `"İ".repeat(11)` to 22 in lower case; 20 units are made.
#[test]
fn a_case_method_past_string_length_throws_a_range_error() {
    let mut limits = limits();
    limits.string_length = 20;
    assert_eq!(
        run(
            limits,
            "var r = [];\n\
             try { \"\\u00df\".repeat(11).toUpperCase(); } catch (e) { r.push(Object.getPrototypeOf(e) === RangeError.prototype); }\n\
             try { \"\\u0130\".repeat(11).toLowerCase(); } catch (e) { r.push(e instanceof RangeError); }\n\
             try { \"\\u00df\".repeat(11).toLocaleUpperCase(); } catch (e) { r.push(e instanceof RangeError); }\n\
             try { \"\\u0130\".repeat(11).toLocaleLowerCase(); } catch (e) { r.push(e instanceof RangeError); }\n\
             r.join() === \"true,true,true,true\" && \"\\u00df\".repeat(10).toUpperCase().length === 20 &&\n\
               \"\\u00df\".repeat(20).toLowerCase().length === 20"
        ),
        yes()
    );
}

/// A case method charges a step per code unit it reads and a step per code unit it writes:
/// `ß` is read as one unit and written as two, a supplementary-plane letter as two and two;
/// the step budget ends a case method that cannot pay them.
#[test]
fn a_case_method_charges_each_code_unit() {
    // The extra steps of `n` = 2,000 over `n` = 1,000, the repeat's copy among them.
    let extra = |template: &str| {
        steps_of(&template.replace("N", "2000")) - steps_of(&template.replace("N", "1000"))
    };
    // Copy 1 unit, read 1, write 2.
    assert_eq!(extra("\"\\u00df\".repeat(N).toUpperCase();"), 4_000);
    assert_eq!(extra("\"\\u0130\".repeat(N).toLocaleLowerCase();"), 4_000);
    // Copy 1, read 1, write 1.
    assert_eq!(extra("\"\\u00e9\".repeat(N).toUpperCase();"), 3_000);
    // Copy 2, read 2, write 2: a surrogate pair, and Final_Sigma.
    assert_eq!(extra("\"\\ud801\\udc00\".repeat(N).toLowerCase();"), 6_000);
    assert_eq!(extra("\"\\u0391\\u03a3\".repeat(N).toLowerCase();"), 6_000);
    // With 1,000 steps: 150 + 150 + 300 are paid, 400 + 400 + 800 are not.
    let mut limits = limits();
    limits.steps_per_entry = 1_000;
    let (mut agent, realm) = agent_and_realm(limits);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "\"\\u00df\".repeat(150).toUpperCase().length;"
        ),
        Outcome::Normal("300".to_string())
    );
    for method in ["toUpperCase", "toLocaleUpperCase"] {
        assert_eq!(
            eval_text(
                &mut agent,
                &realm,
                &format!("\"\\u00df\".repeat(400).{method}();")
            ),
            Outcome::Terminated(Termination::StepBudget),
            "{method}"
        );
        assert!(agent.last_entry_steps().unwrap() <= 1_000);
    }
}

/// `HOST-165` as slice 5 delivers it: on a new agent, each of the four case methods on `"é"`
/// answers the standard's String (`"é"` or `"É"`) and `refusals()` stays unchanged, as for a
/// code unit of 0x80, a surrogate pair, a `toString` that answers U+00FF, an uncaught call and
/// the method of another realm; `localeCompare` with `"é"` as `that` and as the this value
/// answers -1 and 1 (`docs/design/engine.md`, decision 5, "The order `localeCompare` gives":
/// 0x61 before 0xE9), as for a `toString` that answers U+0100 and the method of another realm,
/// and the count stays unchanged; the ASCII calls, a code unit of 0x7F, and a `toString` that
/// throws (whose value is thrown) leave it unchanged too.
#[test]
fn host_165_case_methods_and_locale_compare_refuse_a_unit_outside_ascii() {
    let (mut agent, realm) = agent_and_realm(limits());
    let other_realm = support::host::realm(&mut agent);
    assert_eq!(agent.refusals(), 0);
    // The case methods answer, and the count stays.
    for (method, mapped) in [
        ("toLowerCase", "é"),
        ("toUpperCase", "É"),
        ("toLocaleLowerCase", "é"),
        ("toLocaleUpperCase", "É"),
    ] {
        let source = format!("\"é\".{method}() === \"{mapped}\"");
        assert_eq!(eval_text(&mut agent, &realm, &source), yes(), "{source}");
        assert_eq!(agent.refusals(), 0, "{source}");
    }
    for (source, expected) in [
        ("\"\\u0080\".toLowerCase();", "\"\u{80}\""),
        ("\"a\\ud83d\\ude00\".toUpperCase();", "\"A\u{1F600}\""),
        (
            "String.prototype.toLocaleUpperCase.call({ toString: function () { return \"\\u00ff\"; } });",
            "\"\u{178}\"",
        ),
        ("\"\\u00c9\".toLocaleLowerCase();", "\"é\""),
    ] {
        assert_eq!(
            eval_text(&mut agent, &realm, source),
            Outcome::Normal(expected.to_string()),
            "{source}"
        );
        assert_eq!(agent.refusals(), 0, "{source}");
    }
    let answers = |agent: &mut Agent<_>, source: &str, expected: Outcome| {
        assert_eq!(eval_text(agent, &realm, source), expected, "{source}");
        assert_eq!(agent.refusals(), 0, "{source}");
    };
    // `localeCompare` answers a Number, and the count stays.
    for (call, answer) in [
        ("\"a\".localeCompare(\"é\")", "-1"),
        ("\"é\".localeCompare(\"a\")", "1"),
        (
            "\"a\".localeCompare({ toString: function () { return \"\\u0100\"; } })",
            "-1",
        ),
    ] {
        answers(
            &mut agent,
            &format!("{call};"),
            Outcome::Normal(answer.to_string()),
        );
        answers(
            &mut agent,
            &format!("try {{ typeof {call} === \"number\" }} catch (e) {{ false }}"),
            yes(),
        );
    }
    // The methods of another realm: `localeCompare` and `toLowerCase` answer.
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
    answers(
        &mut agent,
        "OtherSyntaxError.prototype !== SyntaxError.prototype &&\n\
         otherStringPrototype.localeCompare.call(\"a\", \"\\u00e9\") === -1 &&\n\
         otherStringPrototype.localeCompare.call(\"\\u00e9\", \"a\") === 1",
        yes(),
    );
    answers(
        &mut agent,
        "otherStringPrototype.toUpperCase.call(\"\\u00e9\") === \"\\u00c9\"",
        yes(),
    );
    // The ASCII calls of the requirement, a unit of 0x7F, and a ToString that throws.
    for source in [
        "\"AbC\".toLowerCase() === \"abc\" && \"AbC\".toUpperCase() === \"ABC\" && \"a\".localeCompare(\"b\") < 0",
        "\"\\u007f\".toLowerCase() === \"\\u007f\" && \"\\u007f\".localeCompare(\"\\u007f\") === 0",
        "try { String.prototype.toLowerCase.call({ toString: function () { throw 1; } }); } catch (e) { e === 1 }",
        "try { \"\\u00e9\".localeCompare({ toString: function () { throw 1; } }); } catch (e) { e === 1 }",
    ] {
        answers(&mut agent, source, yes());
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
/// result as well, and a trim that removes nothing copies nothing; `localeCompare` a step per
/// code unit it writes into the decomposition and the composed form of each String
/// (`docs/design/engine.md`, decision 5).
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
    // Read the units of both Strings, and write each into its decomposition and its composed
    // form: 1,000 copied, 2,000 read, 2,000 decomposed, 2,000 composed.
    assert_eq!(extra("var s = \"a\".repeat(N); s.localeCompare(s);"), 7_000);
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
        test_path!("conversions_reach_the_reentry_limit_on_the_promised_stack"),
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

/// `#sec-string.prototype.normalize`: a method of length 0; the String in the form ValidateStringOption
/// (`#sec-validatestringoption`) gives, `"NFC"` when `form` is undefined, else ToString of
/// `form`, which must be `"NFC"`, `"NFD"`, `"NFKC"` or `"NFKD"` (a `RangeError` of the realm
/// for any other String); ToString of the this value before ToString of `form`, and an error of
/// the first thrown before the second runs.
#[test]
fn normalize_answers_the_four_forms_and_throws_a_range_error_for_another() {
    check_method("normalize", 0);
    all_true(&[
        // UAX #15, figure 6: the four forms of U+1E9B U+0323.
        "var s = \"\\u1e9b\\u0323\";\n\
         s.normalize() === \"\\u1e9b\\u0323\" && s.normalize(\"NFC\") === \"\\u1e9b\\u0323\" &&\n\
         s.normalize(undefined) === \"\\u1e9b\\u0323\" && s.normalize(\"NFD\") === \"\\u017f\\u0323\\u0307\" &&\n\
         s.normalize(\"NFKC\") === \"\\u1e69\" && s.normalize(\"NFKD\") === \"s\\u0323\\u0307\"",
        // Composed and decomposed Latin, Hangul, a singleton, an exclusion, a ligature.
        "\"\\u00c5\".normalize(\"NFD\") === \"A\\u030a\" && \"A\\u030a\".normalize() === \"\\u00c5\" &&\n\
         \"\\u212b\".normalize() === \"\\u00c5\" && \"\\uac01\".normalize(\"NFD\") === \"\\u1100\\u1161\\u11a8\" &&\n\
         \"\\u1100\\u1161\\u11a8\".normalize() === \"\\uac01\" && \"\\u0958\".normalize() === \"\\u0915\\u093c\" &&\n\
         \"\\ufb01\".normalize() === \"\\ufb01\" && \"\\ufb01\".normalize(\"NFKD\") === \"fi\" &&\n\
         \"\\ufb01\".normalize(\"NFKC\") === \"fi\" && \"abc\".normalize(\"NFKD\") === \"abc\" && \"\".normalize() === \"\"",
        // A supplementary-plane composite that is excluded, and lone surrogates in place.
        "\"\\ud834\\udd5e\".normalize() === \"\\ud834\\udd57\\ud834\\udd65\" &&\n\
         \"\\ud800\\u0301\\udc00\".normalize(\"NFD\") === \"\\ud800\\u0301\\udc00\" &&\n\
         \"e\\ud800\\u0301\".normalize() === \"e\\ud800\\u0301\" && \"\\ud800e\\u0301\".normalize() === \"\\ud800\\u00e9\"",
        // ToString of the this value and of form, in that order; a form of another String.
        "var log = \"\";\n\
         var r = String.prototype.normalize.call(\n\
           { toString: function () { log += \"this\"; return \"\\u00e9\"; } },\n\
           { toString: function () { log += \",form\"; return \"NFD\"; } });\n\
         log === \"this,form\" && r === \"e\\u0301\" && String.prototype.normalize.call(12) === \"12\"",
        "var log = \"\";\n\
         try {\n\
           String.prototype.normalize.call({ toString: function () { throw 7; } },\n\
             { toString: function () { log += \"form\"; return \"NFC\"; } });\n\
         } catch (e) { log += e; }\n\
         log === \"7\"",
        "var refused = function (form) {\n\
           try { \"a\".normalize(form); return false; }\n\
           catch (e) { return Object.getPrototypeOf(e) === RangeError.prototype; }\n\
         };\n\
         refused(\"nfc\") && refused(\"NFC \") && refused(\"\") && refused(\"NFX\") && refused(null) &&\n\
         refused(\"NFKCD\") && refused(\"N\") && refused(1) && !refused(\"NFKD\") && !refused(undefined)",
    ]);
}

/// `#sec-string.prototype.localecompare`, which must answer +0 for canonically equivalent
/// Strings: the five pairs of the clause's Note 2, in both orders, `"\u00e9"` and `"e\u0301"`,
/// the pairs of `15.5.4.9_CE.js` of test262 in its spirit (marks out of canonical order, a
/// Hangul syllable and its jamo), and a String with itself, all answer +0
/// (`docs/design/engine.md`, decision 5); compatibility equivalents stay apart: `"\ufb01"` and
/// `"fi"` do not.
#[test]
fn locale_compare_answers_zero_exactly_for_canonically_equivalent_strings() {
    let pairs = [
        ("\\u212b", "A\\u030a"),
        ("\\u2126", "\\u03a9"),
        ("\\u1e69", "s\\u0307\\u0323"),
        ("\\u1e0b\\u0323", "\\u1e0d\\u0307"),
        ("\\u1100\\u1161", "\\uac00"),
        ("\\u00e9", "e\\u0301"),
        ("a\\u0308\\u0323", "a\\u0323\\u0308"),
        ("\\u1111\\u1171\\u11b6", "\\ud4db"),
        ("u\\u031b\\u0323", "\\u1ef1"),
        ("\\ud834\\udd5e", "\\ud834\\udd57\\ud834\\udd65"),
    ];
    for (a, b) in pairs {
        all_true(&[&format!(
            "1 / \"{a}\".localeCompare(\"{b}\") === Infinity && 1 / \"{b}\".localeCompare(\"{a}\") === Infinity &&\n\
             1 / \"{a}\".localeCompare(\"{a}\") === Infinity && 1 / \"{b}\".localeCompare(\"{b}\") === Infinity"
        )]);
    }
    all_true(&[
        "\"\\ufb01\".localeCompare(\"fi\") !== 0 && \"fi\".localeCompare(\"\\ufb01\") !== 0 &&\n\
         \"\\u2460\".localeCompare(\"1\") !== 0 && \"\\u00e9\".localeCompare(\"e\") !== 0",
    ]);
}

/// `#sec-string.prototype.localecompare` by decision 5: -1, +0 or 1 as the NFC form of the this
/// value is ordered before, equal to or after that of `that` by code units (not code points: a
/// supplementary code point, whose first unit is below 0xDC00, comes before U+FFFF), a String
/// before each longer String that starts with it, and the sign flips when the two are swapped.
#[test]
fn locale_compare_orders_the_nfc_forms_by_code_units() {
    all_true(&[
        "\"a\".localeCompare(\"B\") === 1 && \"B\".localeCompare(\"a\") === -1 &&\n\
         \"\\u00e9\".localeCompare(\"f\") === 1 && \"f\".localeCompare(\"\\u00e9\") === -1",
        "\"e\\u0301\".localeCompare(\"\\u00e9a\") === -1 && \"\\u00e9a\".localeCompare(\"e\\u0301\") === 1 &&\n\
         \"\\u00e9\".localeCompare(\"e\\u0301\\u0301\") === -1 && \"e\\u0301\\u0301\".localeCompare(\"\\u00e9\") === 1",
        "\"\\ud800\\udc00\".localeCompare(\"\\uffff\") === -1 && \"\\uffff\".localeCompare(\"\\ud800\\udc00\") === 1 &&\n\
         \"\\u00e9\".localeCompare(\"\\u0100\") === -1 && \"\\u0100\".localeCompare(\"\\u00e9\") === 1",
        "\"A\\u030a\".localeCompare(\"\\u00c5b\") === -1 && \"\\u00c5b\".localeCompare(\"A\\u030a\") === 1 &&\n\
         \"\".localeCompare(\"\\u0301\") === -1 && \"\\u0301\".localeCompare(\"\") === 1",
    ]);
}

/// A lone surrogate is a code point with no decomposition and class 0 (decision 5): it stays in
/// place in the NFC form, a mark after it stays after it and is not reordered across it, and a
/// starter after it composes with the marks that follow.
#[test]
fn locale_compare_keeps_a_lone_surrogate_in_place() {
    all_true(&[
        "1 / \"\\ud800\".localeCompare(\"\\ud800\") === Infinity && 1 / \"\\udc00\\u0301\".localeCompare(\"\\udc00\\u0301\") === Infinity &&\n\
         1 / \"\\ud800e\\u0301\".localeCompare(\"\\ud800\\u00e9\") === Infinity",
        "\"e\\ud800\\u0301\".localeCompare(\"\\u00e9\\ud800\") === -1 && \"\\u00e9\\ud800\".localeCompare(\"e\\ud800\\u0301\") === 1 &&\n\
         \"\\u0301\\ud800\\u0316\".localeCompare(\"\\u0316\\ud800\\u0301\") === -1 &&\n\
         \"\\udc00\".localeCompare(\"\\ud800\") === 1 && \"\\ud800\".localeCompare(\"\\udc00\") === -1 &&\n\
         \"\\ud800\".localeCompare(\"\\ud800\\udc00\") === -1",
    ]);
}

/// The steps of `localeCompare` (decision 5): a step per code unit of each String read, and a
/// step per code unit written into the buffers, the full canonical decomposition and the
/// composed form of each String; the comparison costs nothing more. `"\u00e9"` and `"e\u0301"`:
/// 1 + 2 read, 2 + 2 decomposed, 1 + 1 composed, 9 in all over `"".localeCompare("")`; U+1D15E,
/// excluded from composition, is read as 2 units and written as 4 and 4; `"Ab"` with itself 4,
/// 4 and 4. With `n` = 2,000 over 1,000 copies of each, the repeats copy 1,000 and 2,000 more.
#[test]
fn locale_compare_charges_a_step_per_unit_read_and_written() {
    let base = steps_of("\"\".localeCompare(\"\");");
    for (source, steps) in [
        ("\"\\u00e9\".localeCompare(\"e\\u0301\");", 9),
        ("\"e\\u0301\".localeCompare(\"\\u00e9\");", 9),
        ("\"\\ud834\\udd5e\".localeCompare(\"\");", 10),
        ("\"Ab\".localeCompare(\"Ab\");", 12),
    ] {
        assert_eq!(steps_of(source) - base, steps, "{source}");
    }
    let extra = |template: &str| {
        steps_of(&template.replace("N", "2000")) - steps_of(&template.replace("N", "1000"))
    };
    // Copy 3,000; read 3,000; decompose 4,000; compose 2,000.
    assert_eq!(
        extra("var s = \"\\u00e9\".repeat(N), t = \"e\\u0301\".repeat(N); s.localeCompare(t);"),
        12_000
    );
}

/// The bytes the agent holds after a collection, with nothing of the last entry kept.
fn settled(agent: &mut Agent<support::host::TestHost>) -> u64 {
    agent.collect().unwrap();
    agent.heap_bytes().unwrap()
}

/// The buffers of `localeCompare` are not Strings of the heap; they are charged to the memory
/// budget before they grow and credited when the method returns, on every way out (decision 5):
/// after an answer, a throw of ToString of `that`, a stop at the step budget and a stop at the
/// memory budget, each run three times on Strings of 5,000 and 10,000 code units, the heap holds
/// what it held before the call, once collected; the script is parsed once, before the first
/// reading. The budgets: 38,000 steps, which the units read (15,000) and the two forms of the
/// this value (15,000) fit and the decomposition of `that` (10,000) does not; and the bytes
/// held before the call plus 40,000, which the decomposition of the this value (10,000 code
/// points in a room of 16,384, 65,536 bytes) does not fit.
#[test]
fn locale_compare_credits_its_buffers_on_every_way_out() {
    const SETUP: &str = "var a = \"\\u00e9\".repeat(5000), b = \"e\\u0301\".repeat(5000);";
    let mut measure = support::host::agent().unwrap();
    let measure_realm = support::host::realm(&mut measure);
    assert!(matches!(
        eval(&mut measure, &measure_realm, SETUP),
        HostOutcome::Normal(_)
    ));
    let held = settled(&mut measure);
    let mut steps = limits();
    steps.steps_per_entry = 38_000;
    let mut memory = limits();
    memory.memory_bytes = held + 40_000;
    for (limits, source, expected) in [
        (
            limits(),
            "a.localeCompare(b);",
            Outcome::Normal("0".to_string()),
        ),
        (
            limits(),
            "a.localeCompare({ toString: function () { throw 1; } });",
            Outcome::Throw("1".to_string()),
        ),
        (
            steps,
            "a.localeCompare(b);",
            Outcome::Terminated(Termination::StepBudget),
        ),
        (
            memory,
            "a.localeCompare(b);",
            Outcome::Terminated(Termination::MemoryBudget),
        ),
    ] {
        let (mut agent, realm) = agent_and_realm(limits);
        assert!(matches!(
            eval(&mut agent, &realm, SETUP),
            HostOutcome::Normal(_)
        ));
        let script = parse(&mut agent, &realm, source);
        let before = settled(&mut agent);
        for run in 0..3 {
            let outcome = agent.evaluate_script(&script).unwrap();
            assert_eq!(
                describe(&mut agent, &realm, outcome),
                expected,
                "{source} {run}"
            );
            assert_eq!(settled(&mut agent), before, "{source} {run}: {expected:?}");
        }
    }
}

/// The forms of `localeCompare` are not Strings, so `string_length` does not bound them: with
/// `string_length` 20, a String of 20 code units of U+0344, whose NFC form is 40 code units
/// (U+0308 U+0301 each), compares as a Number, and `normalize` of it throws the `RangeError`.
#[test]
fn locale_compare_past_string_length_answers() {
    let mut limits = limits();
    limits.string_length = 20;
    assert_eq!(
        run(
            limits,
            "var s = \"\\u0344\".repeat(20);\n\
             var r;\n\
             try { s.normalize(); r = false; } catch (e) { r = e instanceof RangeError; }\n\
             r && 1 / s.localeCompare(s) === Infinity && s.localeCompare(\"\\u0308\\u0301\") === 1 &&\n\
               \"\\u0308\\u0301\".localeCompare(s) === -1 && s.localeCompare(\"\\u0344\") === 1"
        ),
        yes()
    );
}

/// `normalize` writes its result as a String of the heap, bounded by `string_length`
/// (`HOST-21`): with `string_length` 20, a form of more than 20 code units is a `RangeError` of
/// the realm, one of 20 is made; the decomposition it makes on the way is not a String, so the
/// NFC of 20 units of U+00E9, decomposed to 40, is made.
#[test]
fn normalize_past_string_length_throws_a_range_error() {
    let mut limits = limits();
    limits.string_length = 20;
    assert_eq!(
        run(
            limits,
            "var r = [];\n\
             var refused = function (s, form) {\n\
               try { s.normalize(form); return false; }\n\
               catch (e) { return Object.getPrototypeOf(e) === RangeError.prototype; }\n\
             };\n\
             r.push(refused(\"\\u0344\".repeat(11), \"NFD\"), refused(\"\\u0344\".repeat(11), \"NFC\"));\n\
             r.push(refused(\"\\u00e9\".repeat(11), \"NFD\"), refused(\"\\ufdfa\\ufdfa\", \"NFKD\"));\n\
             r.push(refused(\"\\u00e9\".repeat(20), \"NFC\"), refused(\"\\ufdfa\", \"NFKC\"));\n\
             r[0] && r[1] && r[2] && r[3] && r[4] === false && r[5] === false &&\n\
               \"\\u0344\".repeat(10).normalize().length === 20 &&\n\
               \"\\u00e9\".repeat(20).normalize().length === 20 &&\n\
               \"\\u00e9\".repeat(10).normalize(\"NFD\").length === 20 && \"\\ufdfa\".normalize(\"NFKD\").length === 18"
        ),
        yes()
    );
}

/// `normalize` charges as `localeCompare` does, and writes its result through the builder of
/// Strings: a step per code unit of the String read, a step per code unit written into the
/// decomposition, and a step per code unit of the result; `form` is compared with the four
/// names and charges nothing. `"\u00e9"` in NFD: 1, 2 and 2, in NFC 1, 2 and 1; `"e\u0301"` in
/// NFC: 2, 2 and 1; U+FB01 in NFKD: 1, 2 and 2. With 1,000 steps, the 900 of 150 copies of
/// `"\u00e9"` in NFD are paid and the 1,200 of 200 are not.
#[test]
fn normalize_charges_each_code_unit() {
    for (source, steps) in [
        ("\"\\u00e9\".normalize(\"NFD\");", 5),
        ("\"\\u00e9\".normalize(\"NFC\");", 4),
        ("\"e\\u0301\".normalize(\"NFC\");", 5),
        ("\"\\ufb01\".normalize(\"NFKD\");", 5),
        ("\"\\ufb01\".normalize(\"NFKC\");", 5),
        ("\"Ab\".normalize(\"NFKC\");", 6),
    ] {
        let form = if source.contains("NFK") {
            "\"NFKD\""
        } else {
            "\"NFD\""
        };
        let base = steps_of(&format!("\"\".normalize({form});"));
        assert_eq!(steps_of(source) - base, steps, "{source}");
    }
    let mut limits = limits();
    limits.steps_per_entry = 1_000;
    let (mut agent, realm) = agent_and_realm(limits);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "\"\\u00e9\".repeat(150).normalize(\"NFD\").length;"
        ),
        Outcome::Normal("300".to_string())
    );
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "\"\\u00e9\".repeat(200).normalize(\"NFD\");"
        ),
        Outcome::Terminated(Termination::StepBudget)
    );
    assert!(agent.last_entry_steps().unwrap() <= 1_000);
}
