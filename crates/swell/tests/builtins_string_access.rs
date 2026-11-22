//! The access and search methods of `String` and `String.prototype`
//! (`story:string-access-and-search-methods`; ECMA-262 22.1.2 and 22.1.3): `String.fromCharCode`
//! (`#sec-string.fromcharcode`), `String.fromCodePoint` (`#sec-string.fromcodepoint`),
//! `String.raw` (`#sec-string.raw`), and `at`, `charAt`, `charCodeAt`, `codePointAt`, `concat`,
//! `endsWith`, `includes`, `isWellFormed`, `lastIndexOf`, `slice`, `startsWith`, `substring`
//! and `toWellFormed` of `String.prototype` (`#sec-string.prototype.at` and the others). One
//! test per builtin for its property and its steps, the bounds of each (the `string_length`
//! limit, a step per code unit, the step budget) and the stack test through the conversions of
//! an argument. A property keyed by `@@match` needs a well-known symbol, which no script obtains
//! before `Symbol` (slice 6) and the host interface does not hand out, so the case of IsRegExp
//! answering true is a unit test of `builtins/string/access.rs`.
//!
//! The scripts call no builtin another story of the wave delivers: a long String comes from the
//! host interface, a result is compared by `===` with a literal or read through `join`. Run also
//! with the feature `collect-every-safe-point`.
#![cfg(test)]

mod support;

use support::host::{TestHost, agent_with, global, key, limits, object, realm, stack_with_room};
use support::script::{Outcome, eval, eval_text, run_both};
use support::stack::{BUILTINS_REENTRY, on_bounded_stack, reach_through, stack_limits};
use swell::{Agent, AgentState, Limits, Outcome as HostOutcome, Realm, Termination, Value};

/// The script `source` answers the String `expected` in sloppy and in strict mode.
fn answers(source: &str, expected: &str) {
    let want = Outcome::Normal(format!("\"{expected}\""));
    let (sloppy, strict) = run_both(source);
    assert_eq!(sloppy, want, "sloppy");
    assert_eq!(strict, want, "strict");
}

/// An agent with `limits` and a realm whose global object has a String property per entry of
/// `strings`, made from code units through the host interface.
fn agent_with_strings(limits: Limits, strings: &[(&str, Vec<u16>)]) -> (Agent<TestHost>, Realm) {
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    let global_root = global(&mut agent, &realm);
    agent
        .with_cx(&realm, |cx| {
            let global = object(cx, &global_root);
            for (name, units) in strings {
                let value = Value::String(cx.new_string_utf16(units).unwrap());
                let name = key(cx, name);
                cx.set(global, name, value, true).unwrap().unwrap();
            }
        })
        .unwrap();
    (agent, realm)
}

/// `n` code units "a".
fn a_units(n: usize) -> Vec<u16> {
    vec![0x61; n]
}

/// Clause 18 and the "length" each clause states: the sixteen builtins are own data properties
/// of `String` and `String.prototype`, writable, not enumerable, configurable; each a function
/// with its "length" and its "name", with no "prototype" and no [[Construct]].
#[test]
fn every_builtin_is_a_method_with_its_length_and_name() {
    answers(
        r#"
        var bad = [];
        function check(object, name, length) {
          var d = Object.getOwnPropertyDescriptor(object, name);
          if (!d) { bad.push(name + " missing"); return; }
          if (typeof d.value !== "function" || !d.writable || d.enumerable || !d.configurable) {
            bad.push(name + " attributes");
            return;
          }
          if (d.value.length !== length) bad.push(name + " length " + d.value.length);
          if (d.value.name !== name) bad.push(name + " name " + d.value.name);
          if (d.value.hasOwnProperty("prototype")) bad.push(name + " prototype");
          try { new d.value(); bad.push(name + " constructs"); }
          catch (e) { if (!(e instanceof TypeError)) bad.push(name + " threw " + e); }
        }
        check(String, "fromCharCode", 1);
        check(String, "fromCodePoint", 1);
        check(String, "raw", 1);
        var p = String.prototype;
        check(p, "at", 1);
        check(p, "charAt", 1);
        check(p, "charCodeAt", 1);
        check(p, "codePointAt", 1);
        check(p, "concat", 1);
        check(p, "endsWith", 1);
        check(p, "includes", 1);
        check(p, "isWellFormed", 0);
        check(p, "lastIndexOf", 1);
        check(p, "slice", 2);
        check(p, "startsWith", 1);
        check(p, "substring", 2);
        check(p, "toWellFormed", 0);
        bad.join("; ");
        "#,
        "",
    );
}

/// `#sec-string.fromcharcode` with ToUint16 (`#sec-touint16`): each argument is converted in
/// order and taken modulo 2^16 (NaN and the infinities are 0); `#sec-string.fromcodepoint`: a
/// value that is not an integral Number from 0 to 0x10FFFF throws a `RangeError` and stops the
/// conversions; a code point above 0xFFFF is two code units (UTF16EncodeCodePoint), a lone
/// surrogate one.
#[test]
fn from_char_code_wraps_at_2_16_and_from_code_point_refuses_what_is_not_a_code_point() {
    answers(
        r#"
        var r = [];
        r.push(String.fromCharCode() === "");
        r.push(String.fromCharCode(65, 66.9, 0x10043) === "ABC");
        r.push(String.fromCharCode(-1) === "\uFFFF", String.fromCharCode(-65535) === "\u0001");
        r.push(String.fromCharCode(65536, NaN, Infinity, -Infinity, -0) === "\u0000\u0000\u0000\u0000\u0000");
        r.push(String.fromCharCode("97", true, null, undefined) === "a\u0001\u0000\u0000");
        r.push(String.fromCharCode(4294967361, { valueOf: function () { return 0x20042; } }) === "AB");
        r.push(String.fromCharCode(0xD83D, 0xDE00) === "\uD83D\uDE00");
        r.push(String.fromCodePoint() === "");
        r.push(String.fromCodePoint(0x41, 0x1F600, 0x10FFFF, 0) === "A\uD83D\uDE00\uDBFF\uDFFF\u0000");
        r.push(String.fromCodePoint(-0, "66", 0xD800, 0xDC00) === "\u0000B\uD800\uDC00");
        function refuses(v) {
          try { String.fromCodePoint(65, v); return "none"; }
          catch (e) { return e instanceof RangeError; }
        }
        r.push(refuses(-1), refuses(0x110000), refuses(1.5), refuses(NaN), refuses(Infinity),
               refuses(-Infinity), refuses(undefined), refuses("x"), refuses({}), refuses(-0.5));
        var log = "";
        try {
          String.fromCodePoint({ valueOf: function () { log += "a"; return 65; } }, -1,
                               { valueOf: function () { log += "c"; return 67; } });
        } catch (e) { log += e.name; }
        r.push(log);
        r.join();
        "#,
        "true,true,true,true,true,true,true,true,true,true,true,true,true,true,true,\
         true,true,true,true,true,true,aRangeError",
    );
}

/// `#sec-string.raw`: the literals of `template.raw` (ToObject, LengthOfArrayLike) interleaved
/// with the substitutions converted by ToString, in the order literal, substitution; it stops
/// after the literal at the raw length, so a substitution past it is never converted; a raw
/// length of 0 or less answers the empty String; `template` and its "raw" go through ToObject.
#[test]
fn raw_interleaves_substitutions_and_stops_at_the_raw_length() {
    answers(
        r#"
        var r = [];
        r.push(String.raw({ raw: ["a", "b", "c"] }, 1, 2, 3, 4));
        r.push(String.raw({ raw: ["a", "b", "c"] }, 1));
        r.push(String.raw({ raw: { length: 2, 0: "x", 1: "y" } }));
        r.push(String.raw({ raw: "abc" }, "-", "+"));
        r.push(String.raw({ raw: { length: 1.9, 0: "only", 1: "never" } }, "s"));
        r.push(String.raw({ raw: { length: 3 } }, 0, 1));
        r.push(String.raw({ raw: { length: 0 } }, 1) === "", String.raw({ raw: { length: -5 } }) === "",
               String.raw({ raw: {} }) === "");
        r.push(String.raw({ raw: ["a"] }, { toString: function () { throw new Error("read"); } }));
        function throwsType(f) {
          try { f(); return "none"; } catch (e) { return e instanceof TypeError; }
        }
        r.push(throwsType(function () { String.raw(); }), throwsType(function () { String.raw({}); }),
               throwsType(function () { String.raw({ raw: null }); }));
        var log = [], raw = { length: 2 };
        Object.defineProperty(raw, "0", { get: function () {
          log.push("get0");
          return { toString: function () { log.push("literal0"); return "L"; } };
        } });
        Object.defineProperty(raw, "1", { get: function () { log.push("get1"); return "M"; } });
        r.push(String.raw({ raw: raw }, { toString: function () { log.push("sub0"); return "S"; } }));
        r.push(log.join(" "));
        r.join("|");
        "#,
        "a1b2c|a1bc|xy|a-b+c|only|undefined0undefined1undefined|true|true|true|a|true|true|true|\
         LSM|get0 literal0 sub0 get1",
    );
}

/// `#sec-string.prototype.at` with ToAbsoluteIndex (`#sec-toabsoluteindex`) and
/// `#sec-string.prototype.slice` with ToClampedIndex (`#sec-toclampedindex`): a negative index
/// counts from the end; `at` answers undefined outside the String, `slice` clamps to it, and an
/// end at or before the start is the empty String; code units, not code points.
#[test]
fn at_and_slice_count_from_the_end() {
    answers(
        r#"
        var s = "abc", r = [];
        r.push(s.at(0), s.at(-1), s.at(1.7), s.at(-1.7), s.at(), s.at(NaN), s.at("2"));
        r.push(s.at(3) === undefined, s.at(-4) === undefined, s.at(Infinity) === undefined,
               s.at(-Infinity) === undefined, "".at(0) === undefined);
        r.push(s.slice(-2), s.slice(1, -1), s.slice(2, 1) === "", s.slice(-Infinity, Infinity),
               s.slice(), s.slice(undefined, undefined), s.slice(0, NaN) === "", s.slice(-10, 2),
               s.slice(1, 10), s.slice(3) === "", s.slice(1.5, 2.5));
        r.push("\uD83D\uDE00".at(0) === "\uD83D", "\uD83D\uDE00".slice(1) === "\uDE00");
        r.push(String.prototype.at.call(12345, -2), String.prototype.slice.call(true, 1, 3));
        r.join("|");
        "#,
        "a|c|b|c|a|a|c|true|true|true|true|true|bc|b|true|abc|abc|abc|true|ab|bc|true|b|true|\
         true|4|ru",
    );
}

/// `#sec-string.prototype.charat` and `#sec-string.prototype.charcodeat`: the code unit at the
/// position after ToIntegerOrInfinity, as a String or as a Number; outside the String the empty
/// String or NaN.
#[test]
fn char_at_and_char_code_at_read_one_code_unit() {
    answers(
        r#"
        var s = "abc", r = [];
        r.push(s.charAt(1), s.charAt(3) === "", s.charAt(-1) === "", s.charAt(), s.charAt(1.9),
               s.charAt(NaN), s.charAt(Infinity) === "", s.charAt("2"), s.charAt(-0.5));
        r.push(s.charCodeAt(0), s.charCodeAt(), String(s.charCodeAt(3)), String(s.charCodeAt(-1)),
               "\uD800".charCodeAt(0), "\uFFFF".charCodeAt(0), s.charCodeAt(2.9));
        r.push(String.prototype.charAt.call(12345, 2), String.prototype.charCodeAt.call(true, 0));
        r.push("\uD83D\uDE00".charAt(1) === "\uDE00");
        r.join("|");
        "#,
        "b|true|true|a|b|a|true|c|a|97|97|NaN|NaN|55296|65535|99|3|116|true",
    );
}

/// `#sec-string.prototype.codepointat` with CodePointAt (`#sec-codepointat`): a leading
/// surrogate followed by a trailing one is the pair's code point; a lone surrogate, or the
/// trailing half read on its own, is its own value; outside the String undefined.
#[test]
fn code_point_at_reads_a_pair_and_a_lone_surrogate() {
    answers(
        r#"
        var s = "a\uD83D\uDE00\uDC00\uD800b\uDBFF", r = [];
        for (var i = -1; i <= s.length; i++) r.push(String(s.codePointAt(i)));
        r.push(s.codePointAt(), s.codePointAt(NaN), s.codePointAt(1.9),
               String(s.codePointAt(Infinity)), "\uD800\uD800".codePointAt(0),
               String.prototype.codePointAt.call(42, 1));
        r.join("|");
        "#,
        "undefined|97|128512|56832|56320|55296|98|56319|undefined|97|97|128512|undefined|55296|50",
    );
}

/// `#sec-string.prototype.concat`: the this value, then each argument converted by ToString in
/// order (ToPrimitive with hint String: `toString` first); an abrupt conversion stops the rest.
#[test]
fn concat_converts_each_argument_in_order() {
    answers(
        r#"
        var log = [], r = [];
        r.push("a".concat(), "a".concat("b", 1, null, undefined, true), "".concat() === "",
               "a".concat([1, 2]), String.prototype.concat.call(1, 2));
        var x = { toString: function () { log.push("x"); return "X"; } };
        var y = { toString: function () { log.push("y"); return "Y"; },
                  valueOf: function () { log.push("yv"); return "no"; } };
        r.push("a".concat(x, y), log.join());
        try { "a".concat(x, { toString: function () { throw new Error("stop"); } }, y); }
        catch (e) { r.push(e.message); }
        r.push(log.join());
        r.join("|");
        "#,
        "a|ab1nullundefinedtrue|true|a1,2|12|aXY|x,y|stop|x,y,x",
    );
}

/// `#sec-string.prototype.substring`: each argument clamped between 0 and the length (NaN and
/// negative ones are 0), an undefined end is the length, and the smaller is the start.
#[test]
fn substring_swaps_and_clamps() {
    answers(
        r#"
        var s = "abcdef", r = [];
        r.push(s.substring(1, 3), s.substring(3, 1), s.substring(-2, 2), s.substring(4, 100),
               s.substring(NaN, 2), s.substring(2, NaN), s.substring(2), s.substring(),
               s.substring(Infinity, -Infinity), s.substring(2, 2) === "", s.substring(1.9, 3.9),
               s.substring(undefined, 2), s.substring(10) === "");
        r.push(String.prototype.substring.call(12345, 1, 3));
        r.join("|");
        "#,
        "bc|bc|ab|ef|ab|ab|cdef|abcdef|abcdef|true|bc|ab|true|23",
    );
}

/// `#sec-string.prototype.lastindexof` with StringLastIndexOf (`#sec-stringlastindexof`): the
/// last index at or before the position; a position of NaN (undefined included) searches from
/// the last possible start, another is clamped between 0 and that start; the empty search
/// String is found at the clamped position; the search String is converted before the
/// position.
#[test]
fn last_index_of_with_nan_and_with_a_position() {
    answers(
        r#"
        var s = "abcabc", r = [];
        r.push(s.lastIndexOf("c"), s.lastIndexOf("c", 4), s.lastIndexOf("c", 5),
               s.lastIndexOf("c", 1), s.lastIndexOf("a", 0), s.lastIndexOf("a", -5),
               s.lastIndexOf("c", NaN), s.lastIndexOf("c", undefined), s.lastIndexOf("c", Infinity),
               s.lastIndexOf("c", -Infinity), s.lastIndexOf("abc", 2), s.lastIndexOf("x"),
               s.lastIndexOf("", 2), s.lastIndexOf(""), s.lastIndexOf("", 100),
               s.lastIndexOf("abcabcd"), "".lastIndexOf(""), s.lastIndexOf("bc", 4.9));
        r.push(String.prototype.lastIndexOf.call(12312, 12), "xundefined".lastIndexOf(),
               s.lastIndexOf("c", "4"), s.lastIndexOf("c", { valueOf: function () { return 3; } }));
        var log = [];
        "ab".lastIndexOf({ toString: function () { log.push("search"); return "b"; } },
                         { valueOf: function () { log.push("position"); return 1; } });
        r.push(log.join());
        r.join("|");
        "#,
        "5|2|5|-1|0|0|5|5|5|-1|0|-1|2|6|6|-1|0|4|3|1|2|2|search,position",
    );
}

/// `#sec-string.prototype.includes`, `#sec-string.prototype.startswith` and
/// `#sec-string.prototype.endswith`: the search from a position clamped between 0 and the
/// length (`endsWith` ends at it, undefined is the length); the empty search String is always
/// found.
#[test]
fn includes_starts_with_and_ends_with_search_from_a_position() {
    answers(
        r#"
        var s = "abcabc", r = [];
        r.push(s.includes("ca"), s.includes("ca", 3), s.includes("abc", 3), s.includes("abc", 4),
               s.includes(""), s.includes("", 100), s.includes("x"), s.includes("abc", -Infinity),
               s.includes("c", Infinity));
        r.push(s.startsWith("ab"), s.startsWith("bc"), s.startsWith("bc", 1), s.startsWith("", 6),
               s.startsWith("", 100), s.startsWith("abc", 3), s.startsWith("abc", 4),
               s.startsWith("a", -1), s.startsWith("c", Infinity), s.startsWith("abcabcd"));
        r.push(s.endsWith("bc"), s.endsWith("ab"), s.endsWith("ab", 2), s.endsWith("", 0),
               s.endsWith("abc", 3), s.endsWith("abc", 2), s.endsWith("c", 100), s.endsWith("a", NaN),
               s.endsWith("a", undefined), s.endsWith("abcabcd"), s.endsWith("", -1));
        r.push(String.prototype.includes.call(12345, 34), "undefined".startsWith(),
               "xundefined".endsWith(undefined));
        r.join("|");
        "#,
        &["tftfttftf", "tfttttftff", "tftttftffft", "ttt"]
            .concat()
            .chars()
            .map(|c| if c == 't' { "true" } else { "false" })
            .collect::<Vec<_>>()
            .join("|"),
    );
}

/// `#sec-isregexp` in `includes`, `startsWith` and `endsWith`: an object whose `@@match` is
/// undefined (every object a script can make before slice 6) is no RegExp, so it is converted
/// by ToString; the this value is converted first, then the search String, then the position.
/// IsRegExp answering true (a `TypeError`) is the unit test of the same name in
/// `builtins/string/access.rs`.
#[test]
fn includes_starts_with_and_ends_with_refuse_what_is_reg_exp_by_match() {
    answers(
        r#"
        var r = [];
        r.push("a[object Object]".includes({}), "abc".startsWith({ toString: function () { return "ab"; } }),
               "x1,2".endsWith([1, 2]));
        var names = ["includes", "startsWith", "endsWith"];
        for (var i = 0; i < names.length; i++) {
          var log = [];
          String.prototype[names[i]].call(
            { toString: function () { log.push("this"); return "ab"; } },
            { toString: function () { log.push("search"); return "b"; } },
            { valueOf: function () { log.push("position"); return 0; } });
          r.push(log.join());
        }
        r.join("|");
        "#,
        "true|true|true|this,search,position|this,search,position|this,search,position",
    );
}

/// `#sec-string.prototype.iswellformed` with IsStringWellFormedUnicode
/// (`#sec-isstringwellformedunicode`) and `#sec-string.prototype.towellformed`: a lone leading
/// or trailing surrogate makes a String not well formed, and `toWellFormed` replaces each by
/// U+FFFD and keeps the pairs.
#[test]
fn is_well_formed_and_to_well_formed_on_lone_surrogates() {
    answers(
        r#"
        var r = [];
        var cases = ["", "abc", "\uD83D\uDE00", "a\uD800", "\uDC00b", "\uDC00\uD800",
                     "\uD800\uD800\uDC00", "\uDBFF"];
        var fixed = ["", "abc", "\uD83D\uDE00", "a\uFFFD", "\uFFFDb", "\uFFFD\uFFFD",
                     "\uFFFD\uD800\uDC00", "\uFFFD"];
        for (var i = 0; i < cases.length; i++) {
          r.push(cases[i].isWellFormed() + ":" + (cases[i].toWellFormed() === fixed[i]));
        }
        r.push(String.prototype.isWellFormed.call(12), String.prototype.toWellFormed.call(12) === "12");
        r.join("|");
        "#,
        "true:true|true:true|true:true|false:true|false:true|false:true|false:true|false:true|\
         true|true",
    );
}

/// The first steps of each of the thirteen methods of `String.prototype` (RequireObjectCoercible,
/// `#sec-requireobjectcoercible`): a this value of undefined or null throws a `TypeError` before
/// any argument is converted.
#[test]
fn a_method_called_on_undefined_or_null_throws_type_error() {
    answers(
        r#"
        var names = ["at", "charAt", "charCodeAt", "codePointAt", "concat", "endsWith", "includes",
                     "isWellFormed", "lastIndexOf", "slice", "startsWith", "substring",
                     "toWellFormed"];
        var bad = [], ran = false;
        var arg = { valueOf: function () { ran = true; return 0; },
                    toString: function () { ran = true; return ""; } };
        var thisValues = [undefined, null];
        for (var i = 0; i < names.length; i++) {
          var method = String.prototype[names[i]];
          if (typeof method !== "function") { bad.push(names[i] + " missing"); continue; }
          for (var j = 0; j < thisValues.length; j++) {
            try { method.call(thisValues[j], arg, arg); bad.push(names[i] + " returned"); }
            catch (e) { if (!(e instanceof TypeError)) bad.push(names[i] + " threw " + e); }
          }
        }
        if (ran) bad.push("an argument was converted");
        bad.join("; ");
        "#,
        "",
    );
}

/// The `string_length` limit (`docs/design/host-interface.md`, section 2) in `concat`: with a
/// limit of 1,000 code units, a result of 1,280 throws a `RangeError` the script catches, and
/// one of exactly 1,000 is made.
#[test]
fn concat_past_string_length_throws_range_error() {
    let mut limits = limits();
    limits.string_length = 1_000;
    let source = r#"
        var s = "aaaaaaaaaa";
        for (var i = 0; i < 6; i++) s = s + s;
        var r = [s.length];
        try { s.concat(s); r.push("made"); } catch (e) { r.push(e instanceof RangeError); }
        try { "".concat(s, "x", s); r.push("made"); } catch (e) { r.push(e instanceof RangeError); }
        r.push(s.concat(s.slice(0, 360)).length);
        r.join();
    "#;
    for text in [source.to_string(), format!("\"use strict\";\n{source}")] {
        let mut agent = agent_with(limits).unwrap();
        let realm = realm(&mut agent);
        assert_eq!(
            eval_text(&mut agent, &realm, &text),
            Outcome::Normal("\"640,true,true,1000\"".to_string())
        );
    }
}

/// The steps of `source` on Strings `s` of `n` code units "a" and `t` of ten code units that
/// occur nowhere in `s` ("aaaaaaaaab"); the script must complete normally. The steps are read
/// before the value is: writing a Number as text is an entry of its own, which starts the count
/// again.
fn steps_on(n: usize, source: &str) -> u64 {
    let mut t = a_units(9);
    t.push(0x62);
    let (mut agent, realm) = agent_with_strings(limits(), &[("s", a_units(n)), ("t", t)]);
    let outcome = eval(&mut agent, &realm, source);
    let steps = agent.last_entry_steps().unwrap();
    assert!(
        matches!(outcome, HostOutcome::Normal(_)),
        "{source}: {outcome:?}"
    );
    steps
}

/// Every search charges a step per code unit it compares, and every String made a step per
/// code unit copied (decision 3 of the design): doubling the length of `s` from 1,000 to 2,000
/// code units raises the steps of a search of `t` (ten units, at each of 1,000 more starts) by
/// at least 10,000, of a comparison of `s` with itself or a read of each of its code units by
/// at least 1,000, and of a copy of `s` by at least the units copied.
#[test]
fn a_search_charges_a_step_per_code_unit() {
    for (source, more) in [
        ("s.includes(t);", 10_000),
        ("s.lastIndexOf(t);", 10_000),
        ("s.startsWith(s);", 1_000),
        ("s.endsWith(s);", 1_000),
        ("s.isWellFormed();", 1_000),
        ("s.toWellFormed();", 1_000),
        ("s.slice(0);", 1_000),
        ("s.substring(0);", 1_000),
        ("s.concat(s);", 2_000),
    ] {
        let short = steps_on(1_000, source);
        let long = steps_on(2_000, source);
        assert!(
            long >= short + more,
            "{source}: {short} steps on 1,000 units, {long} on 2,000"
        );
    }
}

/// A search or a copy of a String of 100,000 code units under a step budget of 10,000 ends in
/// `Terminated(StepBudget)`, with no more steps spent than the budget, and the agent answers the
/// next script.
#[test]
fn step_budget_ends_a_search_of_a_long_string() {
    for source in [
        "s.includes(\"b\");",
        "s.lastIndexOf(\"b\");",
        "s.lastIndexOf(\"ab\");",
        "s.startsWith(s);",
        "s.endsWith(s);",
        "s.isWellFormed();",
        "s.toWellFormed();",
        "s.slice(1);",
        "s.substring(1);",
        "s.concat(\"\");",
        "String.raw({ raw: s }, 0);",
    ] {
        let mut limits = limits();
        limits.steps_per_entry = 10_000;
        let (mut agent, realm) = agent_with_strings(limits, &[("s", a_units(100_000))]);
        assert_eq!(
            eval_text(&mut agent, &realm, source),
            Outcome::Terminated(Termination::StepBudget),
            "{source}"
        );
        let spent = agent.last_entry_steps().unwrap();
        assert!(spent <= 10_000, "{source}: {spent} steps");
        assert_eq!(agent.state(), AgentState::Alive, "{source}");
        assert_eq!(
            eval_text(&mut agent, &realm, "s.length;"),
            Outcome::Normal("100000".to_string()),
            "{source}"
        );
    }
}

/// `String.prototype.indexOf` (`#sec-string.prototype.indexof`, in `builtins/string/mod.rs`)
/// charges a step per code unit it reads before it copies them: a search of a String of
/// 100,000 code units from position 0 under a step budget of 10,000 ends in
/// `Terminated(StepBudget)`, even when the search String is found at the first start, with no
/// more steps spent than the budget; from position 99,990 it reads 10 code units and answers.
#[test]
fn step_budget_ends_index_of_a_long_string() {
    for source in [
        "s.indexOf(\"a\");",
        "s.indexOf(\"b\");",
        "s.indexOf(\"\", 0);",
    ] {
        let mut limits = limits();
        limits.steps_per_entry = 10_000;
        let (mut agent, realm) = agent_with_strings(limits, &[("s", a_units(100_000))]);
        assert_eq!(
            eval_text(&mut agent, &realm, source),
            Outcome::Terminated(Termination::StepBudget),
            "{source}"
        );
        let spent = agent.last_entry_steps().unwrap();
        assert!(spent <= 10_000, "{source}: {spent} steps");
        assert_eq!(agent.state(), AgentState::Alive, "{source}");
        assert_eq!(
            eval_text(&mut agent, &realm, "s.indexOf(\"a\", 99990);"),
            Outcome::Normal("99990".to_string()),
            "{source}"
        );
    }
}

/// The two conversions of this group that call back into script, through ToString of an
/// argument (`"a".concat({ toString: f })`) and through ToNumber of a position
/// (`"a".charAt({ valueOf: f })`).
fn reach_through_conversions() {
    for through in [
        "\"a\".concat({ toString: f })",
        "\"a\".charAt({ valueOf: f })",
    ] {
        reach_through(through, BUILTINS_REENTRY);
    }
}

/// `Limits::min_stack_bytes` through the conversions of an argument (`AGENTS.md`, "Commands and
/// the gate"): on a thread with seven eighths of `min_stack_bytes()` for the stack limits, a
/// script reaches `reentry_depth` re-entries through `concat` and through `charAt`, parses
/// 1,000 nested brackets at the deepest and throws a `RangeError`, without a stack overflow.
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
