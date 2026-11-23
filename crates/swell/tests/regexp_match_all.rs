//! `RegExp.prototype [ %Symbol.matchAll% ]` (`#sec-regexp-prototype-%symbol.matchall%`) with
//! CreateRegExpStringIterator (`#sec-createregexpstringiterator`) and
//! `%RegExpStringIteratorPrototype%` (`#sec-%regexpstringiteratorprototype%-object`),
//! `String.prototype.matchAll` (`#sec-string.prototype.matchall`) and `RegExp.escape`
//! (`#sec-regexp.escape`, EncodeForRegExpEscape `#sec-encodeforregexpescape`), written by
//! `story:regexp-match-all-and-escape`: what each answers, the order of its reads, the steps of
//! each `next` and of each code unit `RegExp.escape` reads, the step budget, the iterator's hold
//! on its RegExp and its String across a collection, and the re-entries on the promised stack.
//! Run also with the feature `collect-every-safe-point`.
#![cfg(test)]

use crate::support;

use support::host::{TestHost, agent_with, global, key, limits, object, realm, stack_with_room};
use support::script::{Outcome, describe, eval, eval_text, run_both};
use support::stack::{BUILTINS_REENTRY, on_bounded_stack, reach_through, stack_limits, test_path};
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
        assert_eq!(sloppy, yes(), "sloppy: {script}");
        assert_eq!(strict, yes(), "strict: {script}");
    }
}

/// An agent with `limits` and a realm.
fn agent_and_realm(limits: Limits) -> (Agent<TestHost>, Realm) {
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    (agent, realm)
}

/// A count of a test whose claim shows at any count: `default` in the default mode and
/// `collect` with the feature `collect-every-safe-point`, where every instruction collects.
fn mode_count(default: usize, collect: usize) -> usize {
    if cfg!(feature = "collect-every-safe-point") {
        collect
    } else {
        default
    }
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

/// `#sec-%regexpstringiteratorprototype%.next`: the iterator of `String.prototype.matchAll`
/// answers each match of a global RegExp in order, as RegExpExec answers it (`index`, `input`,
/// `groups`), then done, and done from then on; a match whose String is empty moves `lastIndex`
/// by AdvanceStringIndex (`#sec-advancestringindex`), past a whole surrogate pair with `u` (or
/// `v`, in the String of `flags`) and by one code unit without. A pattern that is not an Object
/// is made a RegExp by RegExpCreate with the flags "g", unescaped.
#[test]
fn match_all_iterates_every_match_and_advances_past_an_empty_one() {
    all_true(&[
        r#"var it = "a1b22".matchAll(/\d/g);
           var a = it.next(), b = it.next(), c = it.next(), d = it.next(), e = it.next();
           a.done === false && Array.isArray(a.value) && a.value[0] === "1" &&
             a.value.index === 1 && a.value.input === "a1b22" && b.value[0] === "2" &&
             b.value.index === 3 && c.value.index === 4 && d.done === true &&
             d.value === undefined && e.done === true && e.value === undefined &&
             Object.keys(a).join() === "value,done""#,
        r#"var s = "a😀b";
           [...s.matchAll(/(?:)/gu)].map(m => m.index).join() === "0,1,3,4" &&
             [...s.matchAll(/(?:)/g)].map(m => m.index).join() === "0,1,2,3,4" &&
             [..."xx".matchAll(/(?<n>x)/g)].map(m => m.groups.n + m.index).join() === "x0,x1" &&
             [..."a.b".matchAll(".")].length === 3 && [..."aaa".matchAll(undefined)].length === 4 &&
             [..."abc".matchAll("z")].length === 0"#,
        r#"var log = [];
           var o = {
             constructor: { [Symbol.species]: Object },
             flags: "gv",
             i: 0,
             get lastIndex() { return this.i; },
             set lastIndex(v) { log.push(v); this.i = v; },
             exec: function () { return this.i < 4 ? { 0: "" } : null; }
           };
           var found = [...RegExp.prototype[Symbol.matchAll].call(o, "a😀b")];
           found.length === 3 && log.join() === "0,1,3,4""#,
    ]);
}

/// `#sec-string.prototype.matchall`, step 3.b: a RegExp-like argument (IsRegExp,
/// `#sec-isregexp`) whose `flags`, read by Get, is undefined or null, or whose ToString holds no
/// "g", throws a TypeError before its method keyed by `@@matchAll` is read or called;
/// `%RegExp.prototype%` is one, its `flags` is "". RequireObjectCoercible of the this value
/// comes first. `RegExp.prototype[@@matchAll]` called directly on a RegExp without `g` answers
/// an iterator of one match. An Object that is not RegExp-like is handed to its own method with
/// the this value unconverted.
#[test]
fn match_all_without_g_throws_from_string_match_all_and_answers_one_match_from_the_symbol_method() {
    all_true(&[
        r#"var thrown = [];
           function t(f) { try { f(); thrown.push("none"); } catch (e) { thrown.push(e.constructor.name); } }
           var called = false;
           var r = /a/;
           r[Symbol.matchAll] = function () { called = true; };
           t(() => "a".matchAll(/a/));
           t(() => "a".matchAll(RegExp.prototype));
           t(() => "a".matchAll({ [Symbol.match]: true, flags: undefined }));
           t(() => "a".matchAll({ [Symbol.match]: true, flags: null }));
           t(() => "a".matchAll(r));
           t(() => String.prototype.matchAll.call(undefined, /a/g));
           t(() => String.prototype.matchAll.call(null, /a/g));
           thrown.join() === "TypeError,TypeError,TypeError,TypeError,TypeError,TypeError,TypeError" &&
             !called"#,
        r#"var it = /a/[Symbol.matchAll]("aaa");
           var first = it.next(), second = it.next(), third = it.next();
           first.done === false && first.value[0] === "a" && first.value.index === 0 &&
             second.done === true && second.value === undefined && third.done === true &&
             [.../a/y[Symbol.matchAll]("aaa")].length === 1"#,
        r#"var log = [];
           var o = {
             [Symbol.match]: true,
             get flags() { log.push("flags"); return { toString: function () { log.push("toString"); return "g"; } }; },
             get [Symbol.matchAll]() { log.push("matchAll"); return function (s) { return [this === o, s].join(); }; }
           };
           var plain = { [Symbol.matchAll]: function (s) { return typeof s; } };
           "a".matchAll(o) === "true,a" && log.join() === "flags,toString,matchAll" &&
             String.prototype.matchAll.call(5, plain) === "number""#,
    ]);
}

/// `#sec-regexp-prototype-%symbol.matchall%`, steps 4 to 8: the matcher is constructed by
/// SpeciesConstructor (`#sec-speciesconstructor`) with the this value and the String of its
/// `flags`, its `lastIndex` set to ToLength of the this value's, read once when the iterator is
/// made; the iterator then reads and sets `lastIndex` of the matcher and matches through its
/// `exec`, and never touches the this value.
#[test]
fn the_iterator_reads_last_index_of_a_species_matcher() {
    all_true(&[
        r#"var r = /./g;
           r.lastIndex = { valueOf: function () { return 2; } };
           var it = r[Symbol.matchAll]("abcd");
           r.lastIndex = 0;
           [...it].map(m => m[0] + m.index).join() === "c2,d3""#,
        r#"var log = [];
           function S(pattern, flags) {
             log.push("construct " + flags + " " + (pattern === source));
             var inner = new RegExp(pattern, flags);
             return {
               get lastIndex() { log.push("get"); return inner.lastIndex; },
               set lastIndex(v) { log.push("set " + v); inner.lastIndex = v; },
               exec: function (s) { log.push("exec"); return inner.exec(s); }
             };
           }
           var source = /(?:)/g;
           source.lastIndex = 1;
           source.constructor = { [Symbol.species]: S };
           var found = [...source[Symbol.matchAll]("ab")].map(m => m.index).join();
           found === "1,2" && source.lastIndex === 1 &&
             log.join() === "construct g true,set 1,exec,get,set 2,exec,get,set 3,exec""#,
        r#"var log = [];
           var o = {
             get constructor() { log.push("constructor"); return undefined; },
             get flags() { log.push("flags"); return "gu"; },
             get lastIndex() { log.push("lastIndex"); return 0; },
             source: "b"
           };
           o[Symbol.match] = true;
           var it = RegExp.prototype[Symbol.matchAll].call(o,
             { toString: function () { log.push("string"); return "abcb"; } });
           log.join() === "string,constructor,flags,lastIndex" &&
             [...it].map(m => m.index).join() === "1,3""#,
    ]);
}

/// `#sec-%regexpstringiteratorprototype%-object`: the prototype of every RegExp String Iterator
/// is an ordinary object whose [[Prototype]] is `%Iterator.prototype%`, with `next` of "length"
/// 0 and `[Symbol.toStringTag]` "RegExp String Iterator" { [[Writable]]: false, [[Enumerable]]:
/// false, [[Configurable]]: true } and nothing else; `next` throws a TypeError for a this value
/// that is not an Object or lacks the iterator's slots. The two methods keyed by `matchAll` are
/// methods of "length" 1 as clause 18 makes them.
#[test]
fn the_iterator_prototype_has_next_and_its_tag() {
    all_true(&[
        r#"var it = /a/g[Symbol.matchAll]("a");
           var P = Object.getPrototypeOf(it);
           var IteratorPrototype = Object.getPrototypeOf(Object.getPrototypeOf([][Symbol.iterator]()));
           var d = Object.getOwnPropertyDescriptor(P, "next");
           var t = Object.getOwnPropertyDescriptor(P, Symbol.toStringTag);
           Object.getPrototypeOf(P) === IteratorPrototype && it[Symbol.iterator]() === it &&
             typeof d.value === "function" && d.writable && !d.enumerable && d.configurable &&
             d.value.length === 0 && d.value.name === "next" &&
             t.value === "RegExp String Iterator" && !t.writable && !t.enumerable && t.configurable &&
             Object.prototype.toString.call(it) === "[object RegExp String Iterator]" &&
             Object.getOwnPropertySymbols(P).length === 1 && Object.getOwnPropertyNames(P).join() === "next" &&
             Object.getPrototypeOf("a".matchAll("a")) === P && !Object.hasOwn(it, "next")"#,
        r#"var P = Object.getPrototypeOf(/a/g[Symbol.matchAll]("a"));
           var thrown = 0;
           [{}, Object.create(/a/g[Symbol.matchAll]("a")), 1, "s", undefined, [][Symbol.iterator](),
            "ab"[Symbol.iterator]()].forEach(function (v) {
             try { P.next.call(v); } catch (e) { if (e instanceof TypeError) thrown++; }
           });
           thrown === 7"#,
        r#"function method(d, name, length) {
             var constructed;
             try { new d.value(); constructed = true; } catch (e) { constructed = !(e instanceof TypeError); }
             return typeof d.value === "function" && d.writable && !d.enumerable && d.configurable &&
               d.value.length === length && d.value.name === name &&
               !d.value.hasOwnProperty("prototype") && !constructed;
           }
           method(Object.getOwnPropertyDescriptor(RegExp.prototype, Symbol.matchAll), "[Symbol.matchAll]", 1) &&
             method(Object.getOwnPropertyDescriptor(String.prototype, "matchAll"), "matchAll", 1) &&
             method(Object.getOwnPropertyDescriptor(RegExp, "escape"), "escape", 1)"#,
    ]);
}

/// `#sec-createregexpstringiterator`: the iterator holds its RegExp and its String
/// ([[IteratingRegExp]], [[IteratedString]]), and nothing else does: the matcher is made by
/// the species constructor and the String inside a function that returned. Two full collections
/// of the host run between two calls of `next`, and the script that calls them makes more than
/// the 1 MiB that starts one (`heap/mod.rs`, `collection_due`); the matches after them are
/// read from the same RegExp and String. The step budget is 2^24: the garbage costs a step per
/// code unit.
#[test]
fn the_iterator_survives_a_collection_between_two_next_calls() {
    let mut limits = limits();
    limits.steps_per_entry = 1 << 24;
    let (mut agent, realm) = agent_and_realm(limits);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var it = (function () {\n\
               var s = \"x1y\" + \"22z\".repeat(2);\n\
               return s.matchAll(new RegExp(\"\\\\d+\", \"g\"));\n\
             })();\n\
             it.next().value[0];"
        ),
        Outcome::Normal("\"1\"".to_string())
    );
    for _ in 0..2 {
        agent.collect().unwrap();
    }
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var junk = [];\n\
             for (var i = 0; i < 2; i++) junk.push(\"x\".repeat(1 << 19) + i);\n\
             junk = null;\n\
             var second = it.next().value;\n\
             var third = it.next().value;\n\
             [second[0], second.index, second.input, third[0], third.index, it.next().done].join();"
        ),
        Outcome::Normal("\"22,3,x1y22z22z,22,6,true\"".to_string())
    );
}

/// `#sec-regexp.escape` and `#sec-encodeforregexpescape`, a row of each class of code point:
/// an initial ASCII letter or digit as `\x` and two hex digits (not a later one), a syntax
/// character and `/` after a `\`, the five control escapes of Table 63, the other punctuators,
/// white space and line terminators as `\x` (up to U+00FF) or `\u` with four lowercase hex
/// digits, a lone surrogate of either kind as `\u`, and any other code point (`_`, a letter
/// outside ASCII, a surrogate pair) as itself.
#[test]
fn regexp_escape_escapes_each_class_of_code_point() {
    all_true(&[
        r#"RegExp.escape("1a") === "\\x31a" && RegExp.escape("x1") === "\\x781" &&
           RegExp.escape("Z") === "\\x5a" && RegExp.escape("") === "" &&
           RegExp.escape("_1") === "_1" && RegExp.escape("a.b") === "\\x61\\.b" &&
           RegExp.escape(".") === "\\." && RegExp.escape("/") === "\\/" &&
           RegExp.escape("^$\\.*+?()[]{}|/") === "\\^\\$\\\\\\.\\*\\+\\?\\(\\)\\[\\]\\{\\}\\|\\/""#,
        r#"RegExp.escape("\t\n\v\f\r") === "\\t\\n\\v\\f\\r" &&
           RegExp.escape(",") === "\\x2c" &&
           RegExp.escape(",-=<>#&!%:;@~'`\"") ===
             "\\x2c\\x2d\\x3d\\x3c\\x3e\\x23\\x26\\x21\\x25\\x3a\\x3b\\x40\\x7e\\x27\\x60\\x22" &&
           RegExp.escape(" ") === "\\x20" && RegExp.escape(" ") === "\\xa0" &&
           RegExp.escape("﻿") === "\\ufeff" && RegExp.escape(" ") === "\\u202f" &&
           RegExp.escape("　") === "\\u3000" && RegExp.escape(" ") === "\\u2028" &&
           RegExp.escape(" ") === "\\u2029""#,
        r#"RegExp.escape("\ud800") === "\\ud800" && RegExp.escape("\udfff") === "\\udfff" &&
           RegExp.escape("_\udc00\ud83d") === "_\\udc00\\ud83d" &&
           RegExp.escape("_😀") === "_😀" && RegExp.escape("_é") === "_é" &&
           RegExp.escape("_\u200b") === "_\u200b" &&
           new RegExp(RegExp.escape("1+1 = 2?")).test("x 1+1 = 2?") &&
           RegExp.escape("_@") === "_\\x40""#,
    ]);
}

/// `#sec-regexp.escape`, step 1: a value that is not a String throws a TypeError, a String
/// object and an object with a `toString` included, which is not called.
#[test]
fn regexp_escape_throws_for_a_non_string() {
    all_true(&[r#"var called = false;
           var thrown = 0;
           [undefined, null, 1, true, {}, [], new String("a"), Symbol("s"),
            { toString: function () { called = true; return "a"; } }].forEach(function (v) {
             try { RegExp.escape(v); } catch (e) { if (e instanceof TypeError) thrown++; }
           });
           try { RegExp.escape(); } catch (e) { if (e instanceof TypeError) thrown++; }
           thrown === 10 && !called && RegExp.escape.call(1, "a") === "\\x61""#]);
}

/// The steps of the entry of `source`, which must answer true, in a new agent with the test
/// limits and the global `s` of `units`, which the host makes before the entry.
fn steps_with(units: &[u16], source: &str) -> u64 {
    let (mut agent, realm) = agent_and_realm(limits());
    host_string(&mut agent, &realm, "s", units);
    let outcome = eval(&mut agent, &realm, source);
    let steps = agent.last_entry_steps().unwrap();
    assert_eq!(describe(&mut agent, &realm, outcome), yes(), "{source}");
    steps
}

/// The steps `source` over `s` of `count` code units `unit` costs per code unit beside the
/// same over `2 × count`: the difference of the two entries, divided by `count`, checked to
/// divide evenly.
fn steps_per_unit(unit: u16, count: usize, source: &str) -> u64 {
    let once = steps_with(&vec![unit; count], source);
    let twice = steps_with(&vec![unit; 2 * count], source);
    let difference = twice - once;
    let count = u64::try_from(count).unwrap();
    assert_eq!(difference % count, 0, "{difference} steps over {count}");
    difference / count
}

/// The steps of one match of `[...s.matchAll(/a/g)]`, a call of `next` with its RegExpExec,
/// the iterator's step, its result and the spread's element, measured with [`steps_per_unit`]
/// on this story's code, the same in both collection modes and at both counts (2026-11-23).
const NEXT_STEPS_PER_MATCH: u64 = 9;
/// The steps of `RegExp.escape(s)` per code unit "a" (written as itself after the first) and
/// per code unit "." (written as `\.`): one read and one per code unit written, measured as
/// [`NEXT_STEPS_PER_MATCH`].
const ESCAPE_STEPS_PER_LETTER: u64 = 2;
const ESCAPE_STEPS_PER_SYNTAX_CHARACTER: u64 = 3;

/// `docs/design/engine.md`, decision 3, "The step budget", and decision 7: each `next` of a
/// RegExp String Iterator pays a step of its own besides the steps of its RegExpExec (the call
/// of the builtin `exec`, the matcher, the match array), so the steps of `[...s.matchAll(/a/g)]`
/// over 2n code units "a" exceed those over n by n times [`NEXT_STEPS_PER_MATCH`], exactly, at
/// every n; `RegExp.escape(s)` a step per code unit read and one per code unit written. The
/// counts: 200 in the default mode, 20 with a collection at every safe point.
#[test]
fn each_next_and_each_code_unit_of_escape_are_charged() {
    let count = mode_count(200, 20);
    for (unit, source, expected) in [
        (
            b'a',
            "[...s.matchAll(/a/g)].length === s.length",
            NEXT_STEPS_PER_MATCH,
        ),
        (
            b'a',
            "RegExp.escape(s).length === s.length + 3",
            ESCAPE_STEPS_PER_LETTER,
        ),
        (
            b'.',
            "RegExp.escape(s).length === 2 * s.length",
            ESCAPE_STEPS_PER_SYNTAX_CHARACTER,
        ),
    ] {
        let unit = u16::from(unit);
        let once = steps_per_unit(unit, count, source);
        let twice = steps_per_unit(unit, 2 * count, source);
        println!("{source}: {once} steps per code unit at {count}, {twice} at twice");
        assert_eq!(twice, once, "{source} at twice the count");
        assert_eq!(once, expected, "{source}");
    }
}

/// The step budget ends a global iteration: with `steps_per_entry` 50,000, `[...s.matchAll(/a/g)]`
/// of 100 code units "a" answers its 100 matches, and that of 100,000 ends the entry in
/// `Terminated(StepBudget)` with no more than 50,000 steps spent; `RegExp.escape` of the long
/// String alike, which charges its reads before it writes. With a collection at every safe point
/// the budget is 2,500 and the long String 5,000 code units ([`mode_count`]): at 5,000 the test
/// took 1.34 CPU seconds alone in that mode (2026-11-23).
#[test]
fn a_global_match_all_over_a_long_string_ends_at_the_step_budget() {
    let budget = mode_count(50_000, 2_500);
    let mut limits = limits();
    limits.steps_per_entry = u64::try_from(budget).unwrap();
    let a = u16::from(b'a');
    let (mut agent, realm) = agent_and_realm(limits);
    host_string(&mut agent, &realm, "short", &[a; 100]);
    host_string(&mut agent, &realm, "long", &vec![a; 2 * budget]);
    assert_eq!(
        eval_text(&mut agent, &realm, "[...short.matchAll(/a/g)].length;"),
        Outcome::Normal("100".to_string())
    );
    for source in [
        "[...long.matchAll(/a/g)];",
        "var it = long.matchAll(/a/g); while (!it.next().done);",
        "RegExp.escape(long);",
    ] {
        assert_eq!(
            eval_text(&mut agent, &realm, source),
            Outcome::Terminated(Termination::StepBudget),
            "{source}"
        );
        assert!(
            agent.last_entry_steps().unwrap() <= limits.steps_per_entry,
            "{source}"
        );
    }
}

/// The re-entries of `RegExp.prototype[@@matchAll]`, of the iterator's `next` and of
/// `String.prototype.matchAll`, each called so that one builtin stands between two calls of `f`:
/// the species constructor called by Construct, the getter of `flags`, the `valueOf` of
/// `lastIndex` called by ToLength (of a RegExp-like object whose `source` and `flags` are data,
/// so that `RegExp` calls no `toString` of a builtin before it, which would be a call past
/// `reentry_depth` before `f` at the deepest re-entry), a user `exec` called by RegExpExec from
/// `next` (the species `Object` answers the this value itself as the matcher, with no call of a
/// script), the method keyed by `@@matchAll` of an Object given to `matchAll`, and the getter of
/// `flags` that `matchAll` reads of a RegExp-like argument.
const THROUGH_MATCH_ALL: [&str; 6] = [
    "({ constructor: { [Symbol.species]: f }, m: RegExp.prototype[Symbol.matchAll] }).m(\"x\")",
    "Object.defineProperty({ m: RegExp.prototype[Symbol.matchAll] }, \"flags\", { get: f }).m(\"x\")",
    "({ [Symbol.match]: true, source: \"x\", flags: \"\", lastIndex: { valueOf: f }, m: RegExp.prototype[Symbol.matchAll] }).m(\"x\")",
    "({ constructor: { [Symbol.species]: Object }, flags: \"g\", lastIndex: 0, exec: f, m: RegExp.prototype[Symbol.matchAll] }).m(\"x\").next()",
    "\"x\".matchAll({ [Symbol.matchAll]: f })",
    "\"x\".matchAll(Object.defineProperty({ [Symbol.match]: true }, \"flags\", { get: f }))",
];

/// The `reentry_depth` of the stack test: [`BUILTINS_REENTRY`] in the default mode, 25 with a
/// collection at every safe point, where each instruction marks the whole heap, as
/// `tests/regexp_symbol_methods.rs` takes it.
const STACK_REENTRY: u32 = if cfg!(feature = "collect-every-safe-point") {
    25
} else {
    BUILTINS_REENTRY
};

/// The small `reentry_depth`s each expression of the stack test reaches exactly: 0, 1 and 7 in
/// the default mode, none with a collection at every safe point, where the run at
/// [`STACK_REENTRY`] counts the re-entries exactly. The cost of the test in that mode is the
/// number of agents, not the depth: with the depth 7 besides, the test through the six
/// expressions of [`THROUGH_MATCH_ALL`] took 2.05 CPU seconds alone at the depth 25 and 2.09 at
/// 18 (2026-11-23).
const SMALL_DEPTHS: &[u32] = if cfg!(feature = "collect-every-safe-point") {
    &[]
} else {
    &[0, 1, 7]
};

/// The body of the stack test of [`THROUGH_MATCH_ALL`]: one agent per expression.
fn reach_through_match_all() {
    for through in THROUGH_MATCH_ALL {
        reach_through(through, STACK_REENTRY);
    }
}

/// On a thread with seven eighths of `min_stack_bytes()` for the stack limits, a species
/// constructor that calls `@@matchAll` again reaches the re-entry limit (a `RangeError`), as do
/// the other expressions of [`THROUGH_MATCH_ALL`], and parses 1,000 nested brackets at the
/// deepest, without a stack overflow (`tests/support/stack.rs`, [`reach_through`]), at the
/// `reentry_depth` [`STACK_REENTRY`]; each also reaches the small depths of [`SMALL_DEPTHS`]
/// exactly.
#[test]
fn a_species_constructor_re_enters_on_the_promised_stack() {
    let stack = stack_with_room(stack_limits(STACK_REENTRY));
    on_bounded_stack(
        test_path!("a_species_constructor_re_enters_on_the_promised_stack"),
        stack,
        reach_through_match_all,
    )
    .unwrap();
    for &depth in SMALL_DEPTHS {
        for through in THROUGH_MATCH_ALL {
            reach_through(through, depth);
        }
    }
}
