//! `String.prototype.replace` (`#sec-string.prototype.replace`), `replaceAll`
//! (`#sec-string.prototype.replaceall`) and `split` (`#sec-string.prototype.split`) for a
//! String pattern, with GetSubstitution (`#sec-getsubstitution`) on the String paths
//! (`story:string-replace-replace-all-and-split`): the hand-over of an Object argument to its
//! method keyed by `%Symbol.replace%` or `%Symbol.split%`, the check of `flags` of an object
//! IsRegExp takes for a RegExp in `replaceAll`, the order of the conversions, and the bounds:
//! `string_length` and the memory budget for a result of `replaceAll`, the step budget and a
//! step per element for `split`, and the re-entry of a functional replacer and of a method
//! keyed by a well-known symbol on the promised stack. GetSubstitution with captures and
//! named captures has its own unit test in `src/builtins/string/replace.rs`.
#![cfg(test)]

use crate::support;

use support::host::{TestHost, agent_with, global, key, limits, object, realm, stack_with_room};
use support::script::{Outcome, describe, eval, eval_text, run_both};
use support::stack::{
    BUILTINS_REENTRY, is_range_error_of, on_bounded_stack, reach_through, stack_limits, test_path,
};
use swell::{Agent, Limits, Realm, Termination, Value};

/// `true`, as an outcome.
fn yes() -> Outcome {
    Outcome::Normal("true".to_string())
}

/// Each script answers `true`, as written and with a `"use strict";` prologue, each run in an
/// agent of its own.
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
/// "prototype" and no [[Construct]].
fn method_script(name: &str, length: u32) -> String {
    format!(
        "var d = Object.getOwnPropertyDescriptor(String.prototype, \"{name}\");\n\
         var m = d.value;\n\
         var constructed;\n\
         try {{ new m(); constructed = true; }} catch (e) {{ constructed = !(e instanceof TypeError); }}\n\
         typeof m === \"function\" && d.writable === true && d.enumerable === false &&\n\
           d.configurable === true && m.length === {length} && m.name === \"{name}\" &&\n\
           !m.hasOwnProperty(\"prototype\") && !constructed"
    )
}

/// An agent with `limits` and a realm.
fn agent_and_realm(limits: Limits) -> (Agent<TestHost>, Realm) {
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    (agent, realm)
}

/// Set the global `name` of `realm` to a String of `units` the host makes, outside any entry
/// of a script.
fn host_string(agent: &mut Agent<TestHost>, realm: &Realm, name: &str, units: &[u16]) {
    let global_root = global(agent, realm);
    agent
        .with_cx(realm, |cx| {
            let global = object(cx, &global_root);
            let string = cx.new_string_utf16(units).unwrap();
            let name = key(cx, name);
            cx.set(global, name, Value::String(string), true)
                .unwrap()
                .unwrap();
        })
        .unwrap();
}

/// `#sec-string.prototype.replace`: a method of length 2. The first occurrence of the search
/// String is replaced; a search String that does not occur answers the String; the empty
/// search String occurs at 0. A String replacement goes through GetSubstitution with no
/// captures (`$&` the match, `` $` `` before it, `$'` after it, `$$` a dollar sign, `$1` and
/// `$<n>` themselves); a functional replacer is called once with undefined as its this value
/// and (match, position, String), and its result is converted by ToString. The this value,
/// the search value and a replacement that is not callable are converted in that order, the
/// replacement also when the search String does not occur.
#[test]
fn replace_replaces_the_first_match_with_a_string_or_a_function() {
    all_true(&[
        &method_script("replace", 2),
        "\"abcabc\".replace(\"b\", \"X\") === \"aXcabc\" && \"abc\".replace(\"z\", \"X\") === \"abc\" &&\n\
         \"abc\".replace(\"\", \"X\") === \"Xabc\" && \"\".replace(\"\", \"X\") === \"X\" &&\n\
         \"abc\".replace(\"abc\", \"\") === \"\" && \"a1b\".replace(1, 2) === \"a2b\"",
        "\"abc\".replace(\"b\", \"[$&|$`|$'|$$|$1|$<n>|$]\") === \"a[b|a|c|$|$1|$<n>|$]c\" &&\n\
         \"abc\".replace(\"b\", \"$0$00$10$\") === \"a$0$00$10$c\"",
        "\"abcb\".replace(\"b\", function (m, p, s) { return m + p + s; }) === \"ab1abcbcb\" &&\n\
         \"abc\".replace(\"b\", function () { return 5; }) === \"a5c\" &&\n\
         \"abc\".replace(\"b\", function () { \"use strict\"; return typeof this + arguments.length; }) === \"aundefined3c\"",
        "var n = 0;\n\
         var r = \"abc\".replace(\"z\", function () { n++; return \"x\"; });\n\
         r === \"abc\" && n === 0",
        "var log = \"\";\n\
         var r = String.prototype.replace.call(\n\
           { toString: function () { log += \"this\"; return \"abc\"; } },\n\
           { toString: function () { log += \",search\"; return \"z\"; } },\n\
           { toString: function () { log += \",replace\"; return \"x\"; } });\n\
         log === \"this,search,replace\" && r === \"abc\"",
        "var log = \"\";\n\
         try {\n\
           \"a\".replace({ toString: function () { throw 7; } },\n\
             { toString: function () { log += \"replace\"; return \"\"; } });\n\
         } catch (e) { log += e; }\n\
         log === \"7\"",
    ]);
}

/// `#sec-string.prototype.replaceall`: a method of length 2. Every occurrence is replaced, the
/// search starting again after each match (`advanceBy` = max(1, searchLength)), so occurrences
/// do not overlap and the empty search String matches before every code unit and at the end;
/// a functional replacer is called once per match, in order, with each match's position;
/// GetSubstitution reads each match's position.
#[test]
fn replace_all_replaces_every_match_and_advances_past_an_empty_search() {
    all_true(&[
        &method_script("replaceAll", 2),
        "\"aXbXc\".replaceAll(\"X\", \"-\") === \"a-b-c\" && \"aaa\".replaceAll(\"aa\", \"b\") === \"ba\" &&\n\
         \"abc\".replaceAll(\"z\", \"-\") === \"abc\" && \"aaaa\".replaceAll(\"aa\", \"\") === \"\"",
        "\"abc\".replaceAll(\"\", \"-\") === \"-a-b-c-\" && \"\".replaceAll(\"\", \"x\") === \"x\" &&\n\
         \"\".replaceAll(\"a\", \"x\") === \"\"",
        "var p = [];\n\
         var r = \"xaxbx\".replaceAll(\"x\", function (m, i, s) { p.push(m + i + s.length); return \"\"; });\n\
         r === \"ab\" && p.join() === \"x05,x25,x45\"",
        "var p = [];\n\
         var r = \"ab\".replaceAll(\"\", function (m, i) { p.push(i); return \"|\"; });\n\
         r === \"|a|b|\" && p.join() === \"0,1,2\"",
        "\"abab\".replaceAll(\"b\", \"[$`$&$']\") === \"a[abab]a[abab]\" &&\n\
         \"ab\".replaceAll(\"b\", \"$$$\") === \"a$$\"",
        "var log = \"\";\n\
         var r = String.prototype.replaceAll.call(\n\
           { toString: function () { log += \"this\"; return \"abc\"; } },\n\
           { toString: function () { log += \",search\"; return \"z\"; } },\n\
           { toString: function () { log += \",replace\"; return \"x\"; } });\n\
         log === \"this,search,replace\" && r === \"abc\"",
    ]);
}

/// `#sec-string.prototype.split`: a method of length 2. The String is cut at each occurrence of
/// the separator, at most `limit` parts (ToUint32, so -1 is 2^32 - 1 and 2^32 is 0); the empty
/// separator splits into code units, a surrogate pair into its two units; no separator answers
/// the String alone; the empty String split by a non-empty separator is one empty part, by the
/// empty separator none.
#[test]
fn split_splits_on_a_string_up_to_the_limit_and_into_code_units_on_the_empty_string() {
    all_true(&[
        &method_script("split", 2),
        "var a = \"a,b,c\".split(\",\");\n\
         Array.isArray(a) && a.length === 3 && a.join(\"|\") === \"a|b|c\" &&\n\
         Object.getPrototypeOf(a) === Array.prototype",
        "\"a,b,c\".split(\",\", 2).join(\"|\") === \"a|b\" && \"a,b,c\".split(\",\", 0).length === 0 &&\n\
         \"a,b\".split(\",\", -1).length === 2 && \"a,b\".split(\",\", 4294967296).length === 0 &&\n\
         \"a,b\".split(\",\", 4294967297).length === 1",
        "\"abc\".split(\"\").join(\"|\") === \"a|b|c\" && \"abc\".split(\"\", 2).join(\"|\") === \"a|b\" &&\n\
         \"\\ud83d\\ude00\".split(\"\").length === 2 && \"\\ud83d\\ude00\".split(\"\")[0] === \"\\ud83d\" &&\n\
         \"\".split(\"\").length === 0",
        "var a = \"abc\".split();\n\
         a.length === 1 && a[0] === \"abc\" && \"abc\".split(undefined, 0).length === 0 &&\n\
         \"\".split(\",\").length === 1 && \"\".split(\",\")[0] === \"\"",
        "\",a,,\".split(\",\").join(\"|\") === \"|a||\" && \"a--b--c\".split(\"--\").join(\"|\") === \"a|b|c\" &&\n\
         \"a1b\".split(1).join(\"|\") === \"a|b\" && \"anullb\".split(null).join(\"|\") === \"a|b\" &&\n\
         \"abc\".split(\"abcd\").join(\"|\") === \"abc\" && \"abc\".split(\"abc\").join(\"|\") === \"|\"",
    ]);
}

/// `#sec-string.prototype.split`, steps 5 to 7: the limit is converted by ToUint32, then the
/// separator by ToString, and only then is a limit of 0 read, so the separator is converted
/// (and an error of its conversion thrown) also when no part is made.
#[test]
fn split_converts_the_separator_before_it_reads_a_limit_of_zero() {
    all_true(&[
        "var log = \"\";\n\
         var r = \"a,b\".split({ toString: function () { log += \"separator\"; return \",\"; } },\n\
           { valueOf: function () { log += \"limit,\"; return 0; } });\n\
         log === \"limit,separator\" && r.length === 0",
        "try { \"a\".split({ toString: function () { throw 1; } }, 0); false } catch (e) { e === 1 }",
        "var log = \"\";\n\
         try {\n\
           \"a\".split({ toString: function () { log += \"separator\"; return \"\"; } },\n\
             { valueOf: function () { throw 2; } });\n\
         } catch (e) { log += e; }\n\
         log === \"2\"",
        "var log = \"\";\n\
         var r = String.prototype.split.call({ toString: function () { log += \"this,\"; return \"ab\"; } },\n\
           { toString: function () { log += \"separator\"; return \"\"; } },\n\
           { valueOf: function () { log += \"limit,\"; return 1; } });\n\
         log === \"this,limit,separator\" && r.length === 1 && r[0] === \"a\"",
    ]);
}

/// The first steps of the three clauses: an Object argument whose method keyed by
/// `%Symbol.replace%` or `%Symbol.split%` is not undefined or null is called with the Object as
/// its this value and (the this value, unconverted, and the second argument), and its result
/// answered as it is; a method that is not callable is a TypeError (GetMethod,
/// `#sec-getmethod`); a null method is none. The method is looked up only on an Object: not on
/// undefined or null, and not on a primitive, whose prototype's method is never read.
#[test]
fn an_argument_with_a_symbol_method_is_handed_to_it() {
    all_true(&[
        "var o = { [Symbol.replace]: function (s, r) { return [this === o, typeof s, s, r, arguments.length].join(); } };\n\
         \"abc\".replace(o, \"x\") === \"true,string,abc,x,2\" &&\n\
         \"abc\".replaceAll(o, 1) === \"true,string,abc,1,2\" &&\n\
         String.prototype.replace.call(5, o) === \"true,number,5,,2\"",
        "var o = { [Symbol.split]: function (s, l) { return [this === o, typeof s, s, l, arguments.length]; } };\n\
         var r = String.prototype.split.call(true, o, 3);\n\
         r.join() === \"true,boolean,true,3,2\" && \"a\".split(o).join() === \"true,string,a,,2\"",
        "var answer = {};\n\
         \"a\".split({ [Symbol.split]: function () { return answer; } }) === answer &&\n\
         \"a\".replace({ [Symbol.replace]: function () { return answer; } }) === answer",
        "var r = [];\n\
         try { \"a\".replace({ [Symbol.replace]: 1 }); } catch (e) { r.push(e instanceof TypeError); }\n\
         try { \"a\".replaceAll({ [Symbol.replace]: {} }); } catch (e) { r.push(e instanceof TypeError); }\n\
         try { \"a\".split({ [Symbol.split]: \"f\" }); } catch (e) { r.push(e instanceof TypeError); }\n\
         r.join() === \"true,true,true\"",
        "var sep = { [Symbol.split]: null, toString: function () { return \",\"; } };\n\
         var search = { [Symbol.replace]: undefined, toString: function () { return \"b\"; } };\n\
         \"a,b\".split(sep).join(\"|\") === \"a|b\" && \"abc\".replace(search, \"-\") === \"a-c\" &&\n\
         \"abb\".replaceAll(search, \"-\") === \"a--\"",
        "\"aundefinedb\".replace(undefined, \"-\") === \"a-b\" && \"anullb\".replace(null, \"-\") === \"a-b\" &&\n\
         \"anullnull\".replaceAll(null, \"-\") === \"a--\" && \"aundefinedb\".split(undefined).length === 1 &&\n\
         \"anullb\".split(null).join(\"|\") === \"a|b\"",
        "var get = function () { throw new Error(\"read\"); };\n\
         Object.defineProperty(String.prototype, Symbol.replace, { get: get });\n\
         Object.defineProperty(String.prototype, Symbol.split, { get: get });\n\
         Object.defineProperty(Number.prototype, Symbol.replace, { get: get });\n\
         Object.defineProperty(Boolean.prototype, Symbol.split, { get: get });\n\
         \"a,b\".replace(\",\", \"-\") === \"a-b\" && \"a,b,\".replaceAll(\",\", \"-\") === \"a-b-\" &&\n\
         \"a,b\".split(\",\").length === 2 && \"a1b\".replace(1, \"-\") === \"a-b\" &&\n\
         \"atrueb\".split(true).length === 2",
    ]);
}

/// `#sec-string.prototype.replaceall`, step 2.a and 2.b: an Object that IsRegExp
/// (`#sec-isregexp`) takes for a RegExp, here by a true `@@match`, has its `flags` read; flags
/// undefined or null are a TypeError (RequireObjectCoercible), and flags whose String has no
/// "g" a TypeError, before the method keyed by `%Symbol.replace%` is read; with a "g" that
/// method answers. An Object whose `@@match` is false has no `flags` read.
#[test]
fn replace_all_throws_for_a_regexp_like_object_whose_flags_lack_g() {
    all_true(&[
        "var r = [];\n\
         var log = \"\";\n\
         var replacer = function () { log += \"called\"; return \"r\"; };\n\
         try { \"a\".replaceAll({ [Symbol.match]: true, flags: \"iy\", [Symbol.replace]: replacer }, \"b\"); } catch (e) { r.push(e instanceof TypeError); }\n\
         try { \"a\".replaceAll({ [Symbol.match]: 1, [Symbol.replace]: replacer }, \"b\"); } catch (e) { r.push(e instanceof TypeError); }\n\
         try { \"a\".replaceAll({ [Symbol.match]: \"x\", flags: null, [Symbol.replace]: replacer }, \"b\"); } catch (e) { r.push(e instanceof TypeError); }\n\
         r.join() === \"true,true,true\" && log === \"\"",
        "var o = { [Symbol.match]: true, flags: \"gy\", [Symbol.replace]: function (s, v) { return s + v; } };\n\
         var f = { [Symbol.match]: true, flags: { toString: function () { return \"g\"; } },\n\
           [Symbol.replace]: function () { return \"f\"; } };\n\
         \"a\".replaceAll(o, \"b\") === \"ab\" && \"a\".replaceAll(f, \"b\") === \"f\"",
        "var log = [];\n\
         var o = {};\n\
         Object.defineProperty(o, Symbol.match, { get: function () { log.push(\"match\"); return true; } });\n\
         Object.defineProperty(o, \"flags\", { get: function () { log.push(\"flags\"); return \"g\"; } });\n\
         Object.defineProperty(o, Symbol.replace, { get: function () { log.push(\"replace\"); return function () { return \"r\"; }; } });\n\
         \"a\".replaceAll(o, \"b\") === \"r\" && log.join() === \"match,flags,replace\"",
        "var o = { [Symbol.match]: false, toString: function () { return \"a\"; } };\n\
         Object.defineProperty(o, \"flags\", { get: function () { throw 1; } });\n\
         \"aba\".replaceAll(o, \"-\") === \"-b-\"",
    ]);
}

/// RequireObjectCoercible of the this value (`#sec-requireobjectcoercible`), the first step of
/// each clause: undefined and null are a TypeError before an argument is looked at, so neither
/// a method keyed by a well-known symbol nor a conversion of an argument runs. The method
/// exists, so the TypeError is its own and not that of a call of undefined.
#[test]
fn the_methods_throw_on_undefined_and_null_this() {
    for name in ["replace", "replaceAll", "split"] {
        let source = format!(
            "var log = \"\";\n\
             var arg = {{ toString: function () {{ log += \"toString\"; return \"a\"; }} }};\n\
             Object.defineProperty(arg, Symbol.replace, {{ get: function () {{ log += \"replace\"; }} }});\n\
             Object.defineProperty(arg, Symbol.split, {{ get: function () {{ log += \"split\"; }} }});\n\
             Object.defineProperty(arg, Symbol.match, {{ get: function () {{ log += \"match\"; }} }});\n\
             var r = [];\n\
             for (var thisValue of [undefined, null]) {{\n\
               try {{ String.prototype.{name}.call(thisValue, arg, arg); r.push(false); }}\n\
               catch (e) {{ r.push(e instanceof TypeError); }}\n\
             }}\n\
             typeof String.prototype.{name} === \"function\" && r.join() === \"true,true\" && log === \"\""
        );
        all_true(&[&source]);
    }
}

/// The steps of the entry of `source`, which must answer true, in a new agent with the test
/// limits and the global `s` of `units`, which the host makes before the entry. The steps are
/// read before the answer is: a conversion through the host interface is an entry of its own.
fn steps_with(units: &[u16], source: &str) -> u64 {
    let (mut agent, realm) = agent_and_realm(limits());
    host_string(&mut agent, &realm, "s", units);
    let outcome = eval(&mut agent, &realm, source);
    let steps = agent.last_entry_steps().unwrap();
    assert_eq!(describe(&mut agent, &realm, outcome), yes(), "{source}");
    steps
}

/// The steps of `split` grow with its elements: `s.split(",")` of a String of 999 commas (1,000
/// empty parts) spends more than that of 9 commas (10 parts) by the 990 more code units read,
/// the 990 more starts of the search, each a match of one code unit, and at least a step for
/// each of the 990 more elements: 2,970 at least.
#[test]
fn split_charges_each_element() {
    let comma = u16::from(b',');
    let thousand = steps_with(&[comma; 999], "s.split(\",\").length === 1000");
    let ten = steps_with(&[comma; 9], "s.split(\",\").length === 10");
    println!("split: {thousand} steps for 1,000 parts, {ten} for 10");
    assert!(thousand >= ten + 990, "{thousand} against {ten}");
    assert!(thousand >= ten + 3 * 990, "{thousand} against {ten}");
}

/// The step budget ends a `split`: with `steps_per_entry` 5,000, the 1,000 parts of a String
/// of 999 commas are made, and the 4,000 parts of one of 3,999 commas end the entry in
/// `Terminated(StepBudget)`, with no more than 5,000 steps spent.
#[test]
fn split_of_a_long_string_ends_at_the_step_budget() {
    let mut limits = limits();
    limits.steps_per_entry = 5_000;
    let comma = u16::from(b',');
    let (mut agent, realm) = agent_and_realm(limits);
    host_string(&mut agent, &realm, "short", &[comma; 999]);
    host_string(&mut agent, &realm, "long", &[comma; 3_999]);
    assert_eq!(
        eval_text(&mut agent, &realm, "short.split(\",\").length;"),
        Outcome::Normal("1000".to_string())
    );
    assert_eq!(
        eval_text(&mut agent, &realm, "long.split(\",\");"),
        Outcome::Terminated(Termination::StepBudget)
    );
    assert!(agent.last_entry_steps().unwrap() <= 5_000);
}

/// `HOST-21` for `replaceAll`: with `string_length` 1,000, a result of 1,000 code units is
/// made and one of 1,200 is a `RangeError` of the realm, which a script catches; `replace`
/// is bounded the same way.
#[test]
fn replace_all_past_string_length_throws_a_range_error() {
    let mut limits = limits();
    limits.string_length = 1_000;
    let (mut agent, realm) = agent_and_realm(limits);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "\"a\".repeat(500).replaceAll(\"a\", \"bb\").length;"
        ),
        Outcome::Normal("1000".to_string())
    );
    let past = eval(
        &mut agent,
        &realm,
        "\"a\".repeat(600).replaceAll(\"a\", \"bb\");",
    );
    assert!(is_range_error_of(&mut agent, &realm, past));
    let past = eval(
        &mut agent,
        &realm,
        "\"a\".repeat(600).replace(\"a\", \"b\".repeat(500));",
    );
    assert!(is_range_error_of(&mut agent, &realm, past));
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "try { \"ab\".repeat(300).replaceAll(\"b\", \"$&$&$&\"); false } catch (e) { e instanceof RangeError }"
        ),
        yes()
    );
}

/// The room of the memory budget above the reading of [`replace_all_past_the_memory_budget_terminates`]:
/// 1 MiB, a quarter of the 4 MB the result would take.
const ROOM: u64 = 1 << 20;

/// The result of `replaceAll` is charged before it grows: with the memory budget at the bytes
/// a realm and two host Strings take plus [`ROOM`], `s.replaceAll("a", r)` with `s` 1,000 code
/// units "a" and `r` 2,000 code units, a result of 2,000,000 code units (4 MB), ends the entry
/// in `Terminated(MemoryBudget)`; with a replacement of 200 code units (400 KB) it is made.
#[test]
fn replace_all_past_the_memory_budget_terminates() {
    let a = u16::from(b'a');
    let b = u16::from(b'b');
    let setup = |limits: Limits| {
        let (mut agent, realm) = agent_and_realm(limits);
        host_string(&mut agent, &realm, "s", &[a; 1_000]);
        host_string(&mut agent, &realm, "r", &[b; 2_000]);
        (agent, realm)
    };
    let (agent, _realm) = setup(limits());
    let reading = agent.heap_bytes().unwrap();
    let mut bounded = limits();
    bounded.memory_bytes = reading + ROOM;
    bounded.steps_per_entry = 1 << 24;
    let (mut agent, realm) = setup(bounded);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "s.replaceAll(\"a\", r.slice(0, 200)).length;"
        ),
        Outcome::Normal("200000".to_string())
    );
    assert_eq!(
        eval_text(&mut agent, &realm, "s.replaceAll(\"a\", r);"),
        Outcome::Terminated(Termination::MemoryBudget)
    );
}

/// The re-entries of this story's methods: a functional replacer of `replace` and of
/// `replaceAll`, and a method keyed by `%Symbol.replace%` or `%Symbol.split%`.
const THROUGH_REPLACE: [&str; 4] = [
    "\"x\".replace(\"x\", f)",
    "\"x\".replaceAll(\"x\", f)",
    "\"x\".replace({ [Symbol.replace]: f })",
    "\"x\".split({ [Symbol.split]: f })",
];

/// The body of the stack test: one agent per expression.
fn reach_through_replace() {
    for through in THROUGH_REPLACE {
        reach_through(through, BUILTINS_REENTRY);
    }
}

/// On a thread with seven eighths of `min_stack_bytes()` for the stack limits, a replacer that
/// calls `replace` again reaches the re-entry limit (a `RangeError`), as do the other
/// expressions of [`THROUGH_REPLACE`], and parses 1,000 nested brackets at the deepest, without
/// a stack overflow (`tests/support/stack.rs`, [`reach_through`]).
#[test]
fn a_replacer_re_enters_on_the_promised_stack() {
    let stack = stack_with_room(stack_limits(BUILTINS_REENTRY));
    on_bounded_stack(
        test_path!("a_replacer_re_enters_on_the_promised_stack"),
        stack,
        reach_through_replace,
    )
    .unwrap();
}

/// Each expression of [`THROUGH_REPLACE`] with a small `reentry_depth`: the run reaches the
/// bound exactly, so the stack test measures what it claims.
#[test]
fn each_replacer_reentry_counts_against_reentry_depth() {
    for depth in [0_u32, 1, 7] {
        for through in THROUGH_REPLACE {
            reach_through(through, depth);
        }
    }
}
