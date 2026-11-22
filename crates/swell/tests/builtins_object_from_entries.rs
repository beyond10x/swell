//! `Object.fromEntries` (`#sec-object.fromentries`) through AddEntriesFromIterable
//! (`#sec-add-entries-from-iterable`) and `Object.groupBy` (`#sec-object.groupby`) through
//! GroupBy (`#sec-groupby`) with AddValueToKeyedGroup (`#sec-addvaluetokeyedgroup`)
//! (`story:array-from-and-object-from-entries`). The design is `docs/design/engine.md`,
//! decision 3, "Iterators held by a builtin". The tests that hold all three builtins of the
//! story at the budgets and at IteratorClose are in `tests/builtins_array_from.rs`. Run also
//! with the feature `collect-every-safe-point`.
//!
//! No script uses `for`-`of`, a spread or an array pattern: an iterable is an Array, a String or
//! an object whose `[Symbol.iterator]` the script writes.
#![cfg(test)]

use crate::support;

use support::host::{agent, realm, stack_with_room};
use support::script::{Outcome, eval, eval_text, run_both};
use support::stack::{BUILTINS_REENTRY, on_bounded_stack, reach_through, stack_limits, test_path};
use swell::Outcome as HostOutcome;

/// `source` answers `expected` (as `support::script::show` writes it) in sloppy and in strict
/// code.
fn answers(source: &str, expected: &str) {
    let (sloppy, strict) = run_both(source);
    let expected = Outcome::Normal(expected.to_string());
    assert_eq!(sloppy, expected, "sloppy: {source}");
    assert_eq!(strict, expected, "strict: {source}");
}

/// `source` throws an error named `name` in sloppy and in strict code.
fn throws(source: &str, name: &str) {
    let (sloppy, strict) = run_both(source);
    let expected = Outcome::Throw(name.to_string());
    assert_eq!(sloppy, expected, "sloppy: {source}");
    assert_eq!(strict, expected, "strict: {source}");
}

/// Whether a test takes its gate count: with the feature `collect-every-safe-point`, where
/// every instruction and every call of a builtin collects, unless `SWELL_FULL_COUNTS=1` (the
/// lane `slow`) asks for the full one.
fn gate_count() -> bool {
    cfg!(feature = "collect-every-safe-point")
        && !std::env::var("SWELL_FULL_COUNTS").is_ok_and(|value| value == "1")
}

/// The elements of the large input of the step tests: 1,000, the full count; 100 at the gate
/// count. With the feature `collect-every-safe-point` each call of `next` and of a builtin
/// callback collects a heap that holds the input: at 1,000 the two tests took 1.44 and 0.52 CPU
/// seconds there, each run alone, the first close to the bound of 2; 0.11 and 0.09 at 100.
fn elements() -> u32 {
    if gate_count() { 100 } else { 1_000 }
}

/// The iterables the scripts write: `iterable(values)` answers the elements of the Array
/// `values` in order, then done; `closing(values)` does the same and counts the calls of its
/// `return` in `returns`; `caught(f)` answers what `f` threw, "TypeError" for a `TypeError`, or
/// "none".
const ITERABLES: &str = "var returns = 0;\n\
     function iterable(values, ret) {\n\
       var o = {};\n\
       o[Symbol.iterator] = function () {\n\
         var i = 0;\n\
         var it = { next: function () { i++; return { value: values[i - 1], done: i > values.length }; } };\n\
         if (ret !== undefined) it[\"return\"] = ret;\n\
         return it;\n\
       };\n\
       return o;\n\
     }\n\
     function closing(values) { return iterable(values, function () { returns++; return {}; }); }\n\
     function caught(f) { try { f(); return \"none\"; } catch (e) { return e instanceof TypeError ? \"TypeError\" : e; } }\n";

/// `#sec-object.fromentries` and `#sec-add-entries-from-iterable`: each entry is read by Get of
/// its `0` and then its `1`, never its `length`; the key is ToPropertyKey of the first (a
/// Number, a Symbol, an object by its `toString`, undefined as "undefined"), a later entry of
/// the same key replaces the value and keeps the place, and a key `__proto__` is an own
/// property. The answer is an ordinary object whose prototype is `Object.prototype`, each
/// property writable, enumerable and configurable; an entry with no `1` gives undefined. An
/// Array, a String, an Array Iterator and an iterable the script writes are each read through
/// their iterator. `iterable` undefined or null throws a `TypeError` (RequireObjectCoercible),
/// as does a value that is not iterable and an entry that is no object. The function is
/// `Object.fromEntries`, "length" 1, a method of `Object`.
#[test]
fn from_entries_reads_0_and_1_of_each_entry() {
    answers(
        &[
            ITERABLES,
            "var log = [], r = [];\n\
             var o = Object.fromEntries([[\"a\", 1], [\"b\", 2]]);\n\
             r.push(Object.keys(o).join(\"\"), o.a + o.b, Object.getPrototypeOf(o) === Object.prototype);\n\
             var entry = { get 0() { log.push(\"0\"); return \"k\"; }, get 1() { log.push(\"1\"); return \"v\"; },\n\
               get length() { log.push(\"length\"); return 2; } };\n\
             r.push(Object.fromEntries([entry]).k, log.join(\"\"));\n\
             var s = Symbol(\"s\"), key = { toString: function () { return \"t\"; } };\n\
             var c = Object.fromEntries([[1, \"one\"], [s, \"sym\"], [key, \"obj\"], [\"1\", \"again\"], [undefined, \"u\"]]);\n\
             r.push(c[1], c[s], c.t, c.undefined, Object.keys(c).join(\"+\"));\n\
             var d = Object.getOwnPropertyDescriptor(c, \"t\");\n\
             r.push(d.writable, d.enumerable, d.configurable);\n\
             var p = Object.fromEntries([[\"__proto__\", 5]]);\n\
             r.push(Object.getPrototypeOf(p) === Object.prototype, Object.getOwnPropertyNames(p).join(), \"a\" in Object.fromEntries([[\"a\"]]));\n\
             r.push(Object.keys(Object.fromEntries([])).length, Object.keys(Object.fromEntries(\"\")).length);\n\
             r.push(Object.fromEntries([[\"i\", 1]].values()).i, Object.fromEntries(iterable([[\"x\", 9]])).x);\n\
             var f = Object.fromEntries, fd = Object.getOwnPropertyDescriptor(Object, \"fromEntries\");\n\
             r.push(f.name, f.length, fd.writable, fd.enumerable, fd.configurable);\n\
             r.join();",
        ]
        .concat(),
        "\"ab,3,true,v,01,again,sym,obj,u,1+t+undefined,true,true,true,true,__proto__,true,0,0,1,9,\
         fromEntries,1,true,false,true\"",
    );
    for source in [
        "Object.fromEntries();",
        "Object.fromEntries(undefined);",
        "Object.fromEntries(null);",
        "Object.fromEntries({});",
        "Object.fromEntries(1);",
        "Object.fromEntries([1]);",
        "Object.fromEntries([\"ab\"]);",
    ] {
        throws(source, "TypeError");
    }
}

/// `#sec-add-entries-from-iterable`, step 2.c: an entry that is no object (a Number, a String,
/// null) throws a `TypeError` after IteratorClose has called `return` once, also when `return`
/// throws or answers no object, which do not replace that throw; so does a throw of reading its
/// `0`. A throw of `next`, of reading `done` or `value`, and the end of the iteration call no
/// `return`.
#[test]
fn from_entries_closes_the_iterator_when_an_entry_is_not_an_object() {
    answers(
        &[
            ITERABLES,
            "var r = [];\n\
             var entries = [[1], [\"s\"], [null], [{ get 0() { throw \"zero\"; } }]];\n\
             for (var i = 0; i < entries.length; i++) {\n\
               r.push(caught(function () { Object.fromEntries(closing(entries[i])); }), returns);\n\
             }\n\
             r.push(caught(function () { Object.fromEntries(iterable([1], function () { returns++; throw \"return\"; })); }), returns);\n\
             r.push(caught(function () { Object.fromEntries(iterable([1], function () { returns++; return 1; })); }), returns);\n\
             function bad(next) { var o = {}; o[Symbol.iterator] = function () { return { next: next, \"return\": function () { returns++; return {}; } }; }; return o; }\n\
             var nexts = [function () { throw \"next\"; }, function () { return 1; },\n\
               function () { return { get done() { throw \"done\"; } }; },\n\
               function () { return { done: false, get value() { throw \"value\"; } }; }];\n\
             for (var j = 0; j < nexts.length; j++) {\n\
               r.push(caught(function () { Object.fromEntries(bad(nexts[j])); }));\n\
             }\n\
             r.push(caught(function () { Object.fromEntries(closing([[\"a\", 1]])); }), returns);\n\
             r.join();",
        ]
        .concat(),
        "\"TypeError,1,TypeError,2,TypeError,3,zero,4,TypeError,5,TypeError,6,next,TypeError,done,value,none,6\"",
    );
}

/// The steps of the entry `<source>` after an earlier entry made `ones`, an Array of `n` 1s,
/// and, when `pairs`, `entries`, an Array of the `n` entries `[i, 1]` that
/// `Array.prototype.entries` answers, collected by `Array.from`. The earlier entry runs no
/// loop of its own.
fn steps_of(n: u32, pairs: bool, source: &str) -> u64 {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let setup = if pairs {
        format!(
            "var entries = Array.from(new Array({n}).fill(1).entries());\n\
             entries.length + \":\" + entries[{last}].join()",
            last = n - 1
        )
    } else {
        format!("var ones = new Array({n}).fill(1); ones.length + \":\" + ones[{n} - 1]")
    };
    let expected = if pairs {
        format!("\"{n}:{},1\"", n - 1)
    } else {
        format!("\"{n}:1\"")
    };
    assert_eq!(
        eval_text(&mut agent, &realm, &setup),
        Outcome::Normal(expected)
    );
    // Read right after `evaluate_script`: reading the answer is an entry of its own.
    let outcome = eval(&mut agent, &realm, source);
    assert!(matches!(outcome, HostOutcome::Normal(_)), "{outcome:?}");
    agent.last_entry_steps().unwrap()
}

/// `docs/design/engine.md`, decision 3, "Iterators held by a builtin", item 3: the loop of
/// `Object.fromEntries` charges a step per entry through `charge_steps`, beside the step of each
/// call of `next`, here `%ArrayIteratorPrototype%.next`, which runs no instruction.
/// `last_entry_steps()` after `Object.fromEntries` of an Array of 1,000 entries, made by an
/// earlier script, exceeds that after `Object.fromEntries` of one of 10 by at least 2 × 990;
/// the gate count with the feature `collect-every-safe-point` is 100 entries, at least 2 × 90.
#[test]
fn from_entries_charges_a_step_per_entry() {
    let n = elements();
    let many = steps_of(n, true, "Object.fromEntries(entries);");
    let ten = steps_of(10, true, "Object.fromEntries(entries);");
    println!("{many} steps for {n} entries, {ten} for 10");
    assert!(
        many >= ten + 2 * u64::from(n - 10),
        "{many} steps for {n} entries, {ten} for 10"
    );
}

/// `Limits::min_stack_bytes` through `Object.fromEntries` (decision 3, "Depth bounds"): a user
/// `next` is a call from Rust, a re-entry, so `f` re-enters itself through it until the
/// re-entry limit, on seven eighths of `min_stack_bytes()` (`AGENTS.md`, "Commands and the
/// gate"), in a child process. In the collect mode the depth is [`BUILTINS_REENTRY`].
#[test]
fn from_entries_re_enters_through_a_user_next_on_the_promised_stack() {
    fn body() {
        reach_through(
            "var o = {}; o[Symbol.iterator] = function () { return { next: function () { \
             f(); return { done: true }; } }; }; Object.fromEntries(o)",
            BUILTINS_REENTRY,
        );
    }
    let stack = stack_with_room(stack_limits(BUILTINS_REENTRY));
    on_bounded_stack(
        test_path!("from_entries_re_enters_through_a_user_next_on_the_promised_stack"),
        stack,
        body,
    )
    .unwrap();
}

/// `#sec-object.groupby` with GroupBy (`#sec-groupby`, keyCoercion property) and
/// AddValueToKeyedGroup (`#sec-addvaluetokeyedgroup`): the callback is called with the value
/// and its index, two arguments, with undefined as its this value; its answer is made a key by
/// ToPropertyKey (1 and "1" are one group, a Symbol, an object by its `toString`, undefined as
/// "undefined"), and each group is an Array of its values in order. The answer's prototype is
/// null, its groups are its properties in the order each key was first seen (integer keys
/// first, as for every ordinary object), each writable, enumerable and configurable, a key
/// `__proto__` included. A String is grouped by code point. `items` undefined or null throws a
/// `TypeError` (RequireObjectCoercible), as does a callback that is not callable, before
/// `items` is read, and a value that is not iterable. The function is `Object.groupBy`,
/// "length" 2, a method of `Object`.
#[test]
fn group_by_groups_in_order_and_keys_by_to_property_key() {
    answers(
        "var r = [];\n\
         var g = Object.groupBy([1, 2, 3, 4, 5], function (v) { return v % 2 ? \"odd\" : \"even\"; });\n\
         r.push(Object.keys(g).join(\"+\"), g.odd.join(\"\"), g.even.join(\"\"), Array.isArray(g.odd), Object.getPrototypeOf(g) === null);\n\
         var seen = [];\n\
         Object.groupBy([\"a\", \"b\"], function (v, k) { \"use strict\"; seen.push(arguments.length + v + k + (this === undefined)); });\n\
         r.push(seen.join(\"/\"));\n\
         var s = Symbol(\"s\"), keyOf = [1, \"1\", s, { toString: function () { return \"t\"; } }, undefined, \"z\"];\n\
         var keys = Object.groupBy([0, 1, 2, 3, 4, 5], function (v) { return keyOf[v]; });\n\
         r.push(Object.keys(keys).join(\"+\"), keys[1].join(\"\"), keys[s].join(\"\"), keys.t[0], keys.undefined[0], keys.z[0]);\n\
         r.push(Object.keys(Object.groupBy([\"b\", \"a\", \"b\", \"c\"], function (v) { return v; })).join(\"\"));\n\
         r.push(Object.keys(Object.groupBy([1, 2, 3], function (v) { return 3 - v; })).join(\"+\"));\n\
         r.push(Object.keys(Object.groupBy(\"a\\uD83D\\uDE00\", function (v) { return v.length; })).join(\"+\"));\n\
         var d = Object.getOwnPropertyDescriptor(g, \"odd\");\n\
         r.push(d.writable, d.enumerable, d.configurable, Object.keys(Object.groupBy([], function () {})).length);\n\
         var p = Object.groupBy([1], function () { return \"__proto__\"; });\n\
         r.push(Object.getOwnPropertyNames(p).join(), Array.isArray(p.__proto__));\n\
         var f = Object.groupBy, fd = Object.getOwnPropertyDescriptor(Object, \"groupBy\");\n\
         r.push(f.name, f.length, fd.writable, fd.enumerable, fd.configurable);\n\
         var read = false, o = {};\n\
         Object.defineProperty(o, Symbol.iterator, { get: function () { read = true; return undefined; } });\n\
         try { Object.groupBy(o, 1); } catch (e) { r.push(e.name, read); }\n\
         r.join();",
        "\"odd+even,135,24,true,true,2a0true/2b1true,1+t+undefined+z,01,2,3,4,5,bac,0+1+2,1+2,\
         true,true,true,0,__proto__,true,groupBy,2,true,false,true,TypeError,false\"",
    );
    for source in [
        "Object.groupBy();",
        "Object.groupBy(null, function () {});",
        "Object.groupBy(undefined, function () {});",
        "Object.groupBy([]);",
        "Object.groupBy([], {});",
        "Object.groupBy({}, function () {});",
        "Object.groupBy(1, function () {});",
    ] {
        throws(source, "TypeError");
    }
}

/// `docs/design/engine.md`, decision 3, "Iterators held by a builtin", item 3: the loop of
/// `Object.groupBy` charges a step per element through `charge_steps`, beside the step of each
/// call of `next` (`%ArrayIteratorPrototype%.next`) and of the callback (`Boolean`, a builtin
/// too, which puts every element in the group "true"). `last_entry_steps()` after grouping an
/// Array of 1,000 elements, made by an earlier script, exceeds that after grouping one of 10
/// by at least 3 × 990; the gate count with the feature `collect-every-safe-point` is 100
/// elements, at least 3 × 90.
#[test]
fn group_by_charges_a_step_per_element() {
    let n = elements();
    let many = steps_of(n, false, "Object.groupBy(ones, Boolean);");
    let ten = steps_of(10, false, "Object.groupBy(ones, Boolean);");
    println!("{many} steps for {n} elements, {ten} for 10");
    assert!(
        many >= ten + 3 * u64::from(n - 10),
        "{many} steps for {n} elements, {ten} for 10"
    );
}

/// `Limits::min_stack_bytes` through `Object.groupBy` (decision 3, "Depth bounds"): the
/// callback is a call from Rust, a re-entry, so `f` re-enters itself through it until the
/// re-entry limit, on seven eighths of `min_stack_bytes()` (`AGENTS.md`, "Commands and the
/// gate"), in a child process. In the collect mode the depth is [`BUILTINS_REENTRY`].
#[test]
fn group_by_re_enters_through_its_callback_on_the_promised_stack() {
    fn body() {
        reach_through("Object.groupBy([0], f)", BUILTINS_REENTRY);
    }
    let stack = stack_with_room(stack_limits(BUILTINS_REENTRY));
    on_bounded_stack(
        test_path!("group_by_re_enters_through_its_callback_on_the_promised_stack"),
        stack,
        body,
    )
    .unwrap();
}
