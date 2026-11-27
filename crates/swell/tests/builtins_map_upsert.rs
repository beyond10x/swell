//! `Map.prototype.getOrInsert` (`#sec-map.prototype.getorinsert`),
//! `Map.prototype.getOrInsertComputed` (`#sec-map.prototype.getorinsertcomputed`) and
//! `Map.groupBy` (`#sec-map.groupby`) through GroupBy (`#sec-groupby`) with the key coercion
//! collection, beside `Object.groupBy` (`#sec-object.groupby`) with the coercion property
//! (`story:map-upsert-and-group-by`): the methods by their clauses, the one loop of GroupBy
//! with its IteratorClose, and the step, memory and depth bounds. Run also with the feature
//! `collect-every-safe-point`, where the loops of the budget tests take smaller counts, as
//! those of `builtins_weak_collections` do.
//!
//! No script uses `for`-`of`, a spread or an array pattern: an iterable is an Array, a String or
//! an object whose `[Symbol.iterator]` the script writes, and a Map is read with `forEach`.
#![cfg(test)]

use crate::support;

use support::host::{TestHost, agent_with, limits, realm, stack_with_room};
use support::script::{Outcome, eval, eval_text, run_both};
use support::stack::{BUILTINS_REENTRY, on_bounded_stack, reach_through, stack_limits, test_path};
use swell::{Agent, Limits, Outcome as HostOutcome, Realm, Termination};

/// `source` answers the String `expected` in sloppy and in strict code.
fn answers(source: &str, expected: &str) {
    let (sloppy, strict) = run_both(source);
    let expected = Outcome::Normal(format!("\"{expected}\""));
    assert_eq!(sloppy, expected, "sloppy: {source}");
    assert_eq!(strict, expected, "strict: {source}");
}

/// Whether the feature `collect-every-safe-point` is on: the loops of the budget tests take
/// their smaller counts.
fn collect_mode() -> bool {
    cfg!(feature = "collect-every-safe-point")
}

/// What the scripts share: `caught(f)` answers the `name` of the error `f` threw, the thrown
/// value if it is no error, or "none"; `keys(map)` the keys of `map` in its order, each as
/// `String` writes it (a Symbol as "sym", `-0` and `+0` as "+0"); `closing(values)` an iterable
/// of the elements of `values` that counts the calls of its `return` in `returns`; `endless(f)`
/// an iterable whose `next` answers `f(i)` for i = 0, 1, ... and is never done.
const HELPERS: &str = "function caught(f) { try { f(); return \"none\"; } catch (e) { return e instanceof Error ? e.name : e; } }\n\
     function keys(map) { var k = []; map.forEach(function (v, key) { k.push(typeof key === \"symbol\" ? \"sym\" : key === 0 ? (1 / key > 0 ? \"+0\" : \"-0\") : String(key)); }); return k.join(\" \"); }\n\
     var returns = 0;\n\
     function closing(values) { var o = {}; o[Symbol.iterator] = function () { var i = 0; return { next: function () { i++; return { value: values[i - 1], done: i > values.length }; }, \"return\": function () { returns++; return {}; } }; }; return o; }\n\
     function endless(f) { var o = {}; o[Symbol.iterator] = function () { var i = 0; return { next: function () { var v = f(i); i++; return { done: false, value: v }; } }; }; return o; }\n";

/// `#sec-map.prototype.getorinsert`: RequireInternalSlot first (a Set, a WeakMap, a plain
/// object, `Map.prototype` and a primitive throw a `TypeError`), then
/// CanonicalizeKeyedCollectionKey (`-0` is stored as `+0`); a key found by SameValueZero
/// answers its entry's value and leaves the map as it was, a missing one is appended with the
/// value, which is answered as it is (the same object). The function is
/// `Map.prototype.getOrInsert`, "length" 2, a method, and no constructor.
#[test]
fn get_or_insert_answers_the_present_value_or_inserts() {
    answers(
        &[
            HELPERS,
            "var r = [], m = new Map([[1, \"a\"]]);\n\
             r.push([m.getOrInsert(1, \"x\"), m.size, m.get(1)].join());\n\
             r.push([m.getOrInsert(2, \"b\"), m.size, m.get(2)].join());\n\
             var o = {}, s = Symbol(\"s\"), v = {};\n\
             r.push([m.getOrInsert(o, \"o\"), m.getOrInsert(o, \"p\"), m.getOrInsert(s, \"s\"), m.getOrInsert(\"1\", \"str\"),\n\
               m.getOrInsert(NaN, \"nan\"), m.getOrInsert(0 / 0, \"x\"), m.getOrInsert(undefined), m.has(undefined), m.size].join());\n\
             r.push(keys(m), new Map().getOrInsert(1, v) === v);\n\
             var z = new Map();\n\
             r.push([z.getOrInsert(-0, \"zero\"), z.getOrInsert(0, \"x\"), z.get(-0), keys(z), z.size].join());\n\
             var f = Map.prototype.getOrInsert, d = Object.getOwnPropertyDescriptor(Map.prototype, \"getOrInsert\");\n\
             r.push([f.name, f.length, d.writable, d.enumerable, d.configurable, caught(function () { new f(1, 2); })].join());\n\
             var receivers = [new Set(), new WeakMap(), {}, Map.prototype, 1, undefined];\n\
             r.push(receivers.map(function (x) { return caught(function () { f.call(x, {}, 1); }); }).join());\n\
             r.join(\"|\");",
        ]
        .concat(),
        "a,1,a|b,2,b|o,o,s,str,nan,nan,,true,7|1 2 [object Object] sym 1 NaN undefined|true|\
         zero,zero,zero,+0,1|getOrInsert,2,true,false,true,TypeError|\
         TypeError,TypeError,TypeError,TypeError,TypeError,TypeError",
    );
}

/// `#sec-map.prototype.getorinsertcomputed`: RequireInternalSlot, then IsCallable of the
/// callback (a `TypeError` also when the key is present), then CanonicalizeKeyedCollectionKey;
/// a present key answers its value and calls nothing; a missing one calls the callback once,
/// with the canonical key (`+0` for `-0`) as its one argument and `undefined` as its this
/// value, and stores and answers its result; a callback that throws adds nothing. The function
/// is `Map.prototype.getOrInsertComputed`, "length" 2, a method, and no constructor.
#[test]
fn get_or_insert_computed_calls_its_callback_only_for_a_missing_key() {
    answers(
        &[
            HELPERS,
            "var r = [], m = new Map([[1, \"a\"]]), calls = 0, seen;\n\
             function cb(key) { \"use strict\"; calls++; seen = [arguments.length, this === undefined, 1 / key].join(\" \"); return \"c\" + calls; }\n\
             r.push([m.getOrInsertComputed(1, cb), calls].join());\n\
             r.push([m.getOrInsertComputed(-0, cb), calls, seen, m.get(0), keys(m)].join());\n\
             r.push([m.getOrInsertComputed(0, cb), calls].join());\n\
             r.push([caught(function () { m.getOrInsertComputed(1, 1); }), caught(function () { m.getOrInsertComputed(9, {}); }),\n\
               caught(function () { m.getOrInsertComputed(9); }), m.has(9), calls].join());\n\
             r.push([caught(function () { m.getOrInsertComputed(5, function () { throw new RangeError(\"cb\"); }); }), m.has(5), m.size].join());\n\
             var f = Map.prototype.getOrInsertComputed, d = Object.getOwnPropertyDescriptor(Map.prototype, \"getOrInsertComputed\");\n\
             r.push([f.name, f.length, d.writable, d.enumerable, d.configurable, caught(function () { new f(1, cb); })].join());\n\
             var receivers = [new Set(), new WeakMap(), {}, Map.prototype, 1, undefined];\n\
             r.push(receivers.map(function (x) { return caught(function () { f.call(x, {}, cb); }); }).join(), calls);\n\
             r.join(\"|\");",
        ]
        .concat(),
        "a,0|c1,1,1 true Infinity,c1,1 +0|c1,1|TypeError,TypeError,TypeError,false,1|\
         RangeError,false,2|getOrInsertComputed,2,true,false,true,TypeError|\
         TypeError,TypeError,TypeError,TypeError,TypeError,TypeError|1",
    );
}

/// `#sec-map.prototype.getorinsertcomputed`, steps 7 and 8: the map is searched again after the
/// callback, so a key the callback set keeps the place the callback gave it and takes the
/// callback's answer, with no second entry; a key the callback set and deleted again, or a map
/// it cleared, is appended at the end. A callback that grows the map to 25 entries, past the 8
/// where the table starts to keep an index map and through two growths of its room, with the key
/// set as the 13th, leaves the key at its place. (With 201 entries the test took 5.6 CPU seconds
/// alone with the feature `collect-every-safe-point`, measured on 2026-11-27.)
#[test]
fn a_callback_that_sets_the_key_is_overwritten_not_appended() {
    answers(
        &[
            HELPERS,
            "var r = [], m = new Map([[1, \"a\"]]);\n\
             var got = m.getOrInsertComputed(2, function () { m.set(2, \"inner\"); m.set(3, \"c\"); return \"outer\"; });\n\
             r.push([got, m.size, keys(m), m.get(2)].join());\n\
             got = m.getOrInsertComputed(4, function () { m.set(4, \"in\"); m.set(5, \"e\"); m.delete(4); return \"out\"; });\n\
             r.push([got, m.size, keys(m), m.get(4)].join());\n\
             got = m.getOrInsertComputed(6, function () { m.clear(); m.set(7, \"g\"); return \"six\"; });\n\
             r.push([got, m.size, keys(m), m.get(6)].join());\n\
             var big = new Map(), i;\n\
             got = big.getOrInsertComputed(\"k\", function () { for (i = 0; i < 12; i++) big.set(i, i); big.set(\"k\", \"inner\"); for (i = 12; i < 24; i++) big.set(i, i); return \"outer\"; });\n\
             var at = -1, n = 0; big.forEach(function (v, key) { if (key === \"k\") at = n; n++; });\n\
             r.push([got, big.size, at, big.get(\"k\"), big.get(23)].join());\n\
             r.join(\"|\");",
        ]
        .concat(),
        "outer,3,1 2 3,outer|out,5,1 2 3 5 4,out|six,2,7 6,six|outer,25,12,outer,23",
    );
}

/// `#sec-map.groupby` with GroupBy (`#sec-groupby`, keyCoercion collection) and
/// AddValueToKeyedGroup: the callback's answer is the key after CanonicalizeKeyedCollectionKey,
/// compared by SameValueZero, so `-0` and `+0` are one group whose key is `+0`, every NaN is
/// one group, `"0"` is not `0`, and objects are keys by identity; the values are kept as they
/// are (`-0` stays `-0` in its group). The result is a new Map of `%Map.prototype%` (whatever
/// the this value) whose entries are in the order of each key's first appearance, each an Array
/// of its values in order. The callback is called with the value and its index, two arguments,
/// with undefined as its this value; a String is grouped by code point; an empty iterable gives
/// an empty Map. `items` undefined or null throws a `TypeError` (RequireObjectCoercible), as
/// does a callback that is not callable, before `items` is read, and a value that is not
/// iterable. The function is `Map.groupBy`, "length" 2, a method of `Map`.
#[test]
fn map_group_by_groups_by_same_value_zero_in_order() {
    answers(
        &[
            HELPERS,
            "var r = [], o1 = {}, o2 = {};\n\
             var items = [1, -0, 0, NaN, \"0\", o1, 2, 0 / 0, o2, o1, -0];\n\
             var g = Map.groupBy(items, function (v) { return v; });\n\
             var parts = [];\n\
             g.forEach(function (values, key) { parts.push((key === o1 ? \"o1\" : key === o2 ? \"o2\" : typeof key === \"string\" ? \"s\" + key : key === 0 ? (1 / key > 0 ? \"+0\" : \"-0\") : String(key)) + \":\" + values.length); });\n\
             r.push(g.size, parts.join(\" \"), 1 / g.get(0)[0], 1 / g.get(0)[1], g.get(o1)[1] === o1, Array.isArray(g.get(NaN)));\n\
             r.push(g instanceof Map, Object.getPrototypeOf(g) === Map.prototype, Object.getPrototypeOf(Map.groupBy.call({}, [1], String)) === Map.prototype);\n\
             var seen = [];\n\
             Map.groupBy([\"a\", \"b\"], function (v, k) { \"use strict\"; seen.push(arguments.length + v + k + (this === undefined)); });\n\
             r.push(seen.join(\"/\"), keys(Map.groupBy([\"b\", \"a\", \"b\", \"c\"], function (v) { return v; })));\n\
             r.push(keys(Map.groupBy(\"a\\uD83D\\uDE00a\", function (v) { return v.length; })), Map.groupBy([], function () {}).size);\n\
             var f = Map.groupBy, fd = Object.getOwnPropertyDescriptor(Map, \"groupBy\");\n\
             r.push(f.name, f.length, fd.writable, fd.enumerable, fd.configurable, caught(function () { new f([], String); }));\n\
             var read = false, o = {};\n\
             Object.defineProperty(o, Symbol.iterator, { get: function () { read = true; return undefined; } });\n\
             r.push(caught(function () { Map.groupBy(o, 1); }), read);\n\
             r.push([caught(function () { Map.groupBy(); }), caught(function () { Map.groupBy(null, String); }),\n\
               caught(function () { Map.groupBy(undefined, String); }), caught(function () { Map.groupBy([]); }),\n\
               caught(function () { Map.groupBy([], {}); }), caught(function () { Map.groupBy({}, String); }),\n\
               caught(function () { Map.groupBy(1, String); })].join(\" \"));\n\
             r.join();",
        ]
        .concat(),
        "7,1:1 +0:3 NaN:2 s0:1 o1:2 2:1 o2:1,-Infinity,Infinity,true,true,true,true,true,\
         2a0true/2b1true,b a c,1 2,0,groupBy,2,true,false,true,TypeError,TypeError,false,\
         TypeError TypeError TypeError TypeError TypeError TypeError TypeError",
    );
}

/// `#sec-object.groupby` after GroupBy took the coercion: keyCoercion property still makes the
/// key with ToPropertyKey, so `-0`, `0` and `"0"` are the one group `"0"`, `1` and `"1"` the
/// group `"1"`, the values are kept as they are, and the answer's prototype is null.
#[test]
fn object_group_by_still_groups_by_property_key() {
    answers(
        "var g = Object.groupBy([-0, \"0\", 0, 1, \"1\"], function (v) { return v; });\n\
         [Object.keys(g).join(\"+\"), g[0].length, 1 / g[0][0], g[\"0\"][1], g[1].length, Object.getPrototypeOf(g) === null].join();",
        "0+1,3,-Infinity,0,2,true",
    );
}

/// GroupBy, step 6.e (IfAbruptCloseIterator of the callback's completion), in `Map.groupBy`
/// and in `Object.groupBy`, which share the loop: a callback that throws closes the iterator
/// once (its `return` is called) and the throw is the call's; a `next` that throws ends the
/// call with no close (step 6.b); a callback whose answer has a `toString` that throws closes
/// the iterator in `Object.groupBy` (step 6.g.ii) and is no throw in `Map.groupBy`, whose
/// coercion calls no code.
#[test]
fn group_by_closes_the_iterator_when_the_callback_throws() {
    answers(
        &[
            HELPERS,
            "var r = [], calls = 0;\n\
             function cb(v) { calls++; if (v === 2) throw new RangeError(\"cb\"); return v; }\n\
             r.push([caught(function () { Map.groupBy(closing([1, 2, 3]), cb); }), calls, returns].join());\n\
             r.push([caught(function () { Object.groupBy(closing([1, 2, 3]), cb); }), calls, returns].join());\n\
             var bad = { toString: function () { throw new SyntaxError(\"key\"); } };\n\
             r.push([caught(function () { Object.groupBy(closing([1, 2]), function () { return bad; }); }), returns].join());\n\
             r.push([Map.groupBy(closing([1, 2]), function () { return bad; }).get(bad).length, returns].join());\n\
             var it = {}; it[Symbol.iterator] = function () { return { next: function () { throw new EvalError(\"next\"); }, \"return\": function () { returns++; return {}; } }; };\n\
             r.push([caught(function () { Map.groupBy(it, cb); }), caught(function () { Object.groupBy(it, cb); }), returns].join());\n\
             r.join(\"|\");",
        ]
        .concat(),
        "RangeError,2,1|RangeError,4,2|SyntaxError,3|2,3|EvalError,EvalError,3",
    );
}

/// An agent with [`limits`] changed by `change` and its realm.
fn agent_and_realm(change: impl FnOnce(&mut Limits)) -> (Agent<TestHost>, Realm) {
    let mut limits = limits();
    change(&mut limits);
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    (agent, realm)
}

/// Collect and answer `heap_bytes()` after it.
fn collected(agent: &mut Agent<TestHost>) -> u64 {
    assert!(matches!(agent.collect().unwrap(), HostOutcome::Normal(_)));
    agent.heap_bytes().unwrap()
}

/// `last_entry_steps()` of `Map.groupBy(ones, Boolean)` over `ones`, an Array of `n` 1s made by
/// an earlier script; every element goes to the one group `true`.
fn group_by_steps(n: u32) -> u64 {
    let (mut agent, realm) = agent_and_realm(|_| {});
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            &format!("var ones = new Array({n}).fill(1); ones.length;")
        ),
        Outcome::Normal(n.to_string())
    );
    // Read right after `evaluate_script`: reading the answer is an entry of its own.
    let outcome = eval(
        &mut agent,
        &realm,
        "Map.groupBy(ones, Boolean).get(true).length;",
    );
    assert!(matches!(outcome, HostOutcome::Normal(_)), "{outcome:?}");
    agent.last_entry_steps().unwrap()
}

/// The steps of one more element of `Map.groupBy(ones, Boolean)`, measured: the loop's own
/// step (GroupBy, step 6, charged before `next`), the call of `next`
/// (`%ArrayIteratorPrototype%.next`), the call of the callback (`Boolean`), the lookup of the
/// key in the result's table and the append to the group's Array. Without the loop's own step
/// one element costs one step less, and the comparison of [`group_by_steps`] goes red.
const STEPS_PER_ELEMENT: u64 = 4;

/// The step budget (`docs/design/engine.md`, "The step budget"): `Map.groupBy` over an iterable
/// that is never done, with `steps_per_entry` N (100,000; 2,000 in the collect mode), ends in
/// `Terminated(StepBudget)`, and `last_entry_steps()` is at most N. Beside it, the loop charges
/// its own step per element: `Map.groupBy` of 1,000 elements (100 in the collect mode) takes at
/// least [`STEPS_PER_ELEMENT`] steps per element more than of 10.
#[test]
fn group_by_over_an_endless_iterable_ends_at_the_step_budget() {
    let budget = if collect_mode() { 2_000 } else { 100_000 };
    let (mut agent, realm) = agent_and_realm(|limits| limits.steps_per_entry = budget);
    let outcome = eval_text(
        &mut agent,
        &realm,
        &format!(
            "{HELPERS}Map.groupBy(endless(function (i) {{ return i; }}), function (v) {{ return v % 4; }});"
        ),
    );
    let steps = agent.last_entry_steps().unwrap();
    println!("{steps} steps of {budget}");
    assert_eq!(outcome, Outcome::Terminated(Termination::StepBudget));
    assert!(steps <= budget, "{steps} steps of {budget}");
    let n: u32 = if collect_mode() { 100 } else { 1_000 };
    let many = group_by_steps(n);
    let ten = group_by_steps(10);
    println!("Map.groupBy: {many} steps for {n} elements, {ten} for 10");
    assert!(
        many >= ten + STEPS_PER_ELEMENT * u64::from(n - 10),
        "{many} steps for {n} elements, {ten} for 10"
    );
}

/// The objects the first script of the memory test makes at once in the default mode, so the
/// objects arena has room for every object the endless script makes within the budget (as in
/// `builtins_weak_collections`). The collect mode makes none: each object made there costs
/// collections of the whole heap.
const WARM_OBJECTS: usize = 4_096;

/// The elements the memory test groups at the least before the budget ends it, each its own
/// group (an Array and an entry of the table). Measured at the eight sizes of the realm of
/// `task test-realm-sizes` and with no padding: 419 in the default mode at each; in the collect
/// mode 29 at 64 padding functions, where a growth of the objects arena comes first, 55 at 38,
/// and 76 to 134 at the others.
const MEMORY_GROUPS: u64 = 16;

/// The memory budget (`docs/design/engine.md`, "Memory budget"): `Map.groupBy` over an iterable
/// that is never done, each element its own key, so the result's table and a new group grow
/// with every element, with a budget of the realm's reading plus 256 KiB (64 KiB in the collect
/// mode: at 16 KiB the first script itself ended at the budget there), ends in
/// `Terminated(MemoryBudget)` after grouping at least [`MEMORY_GROUPS`] elements, with
/// `heap_bytes()` above the reading before it; once the host dropped the script, one `collect()`
/// brings `heap_bytes()` back within the reading before the script. Before the reading a first
/// script defines the helpers the second calls and runs the same shape over 8 elements, which
/// grows the arenas a script needs (in the default mode with [`WARM_OBJECTS`] objects made at
/// once); nothing is made between the readings. The readings are printed. How far into the room
/// the reading at the end lies is not held: the run ends at the first growth the room cannot
/// take, and where an arena of the heap doubles depends on the size of the realm (in the collect
/// mode, at 38 and 64 padding functions, the end lay 25,544 and 15,432 bytes above the reading,
/// under half the room). Objects made first in the collect mode, by one `JSON.parse`, did not
/// make that hold: with 256 of them the first script ended at the budget at 128 padding
/// functions in a room of 32 and of 48 KiB, and with 512 in a room of 64 KiB the test took 2.5 to
/// 3.3 s of wall time.
#[test]
fn group_by_over_an_endless_iterable_ends_at_the_memory_budget() {
    let room: u64 = if collect_mode() { 64 << 10 } else { 256 << 10 };
    let warm = format!(
        "{HELPERS}var grouped = 0;\n\
         (function () {{ var a = []; for (var i = 0; i < {objects}; i++) a.push({{}}); }})();\n\
         Map.groupBy(closing([0, 1, 2, 3, 4, 5, 6, 7]), function (v) {{ return v; }}).size;",
        objects = if collect_mode() { 0 } else { WARM_OBJECTS }
    );
    // The helpers are the first script's globals: a script that defines them again does not
    // parse within the room of the collect mode. `grouped` counts the elements the callback saw,
    // a Number stored in a global the first script made, so the count makes nothing.
    let endless = "Map.groupBy(endless(function (i) { return i; }), function (v) { grouped = v + 1; return v; });";
    let warmed = |agent: &mut Agent<TestHost>, realm: &Realm| {
        assert_eq!(
            eval_text(agent, realm, &warm),
            Outcome::Normal("8".to_string())
        );
        collected(agent)
    };
    let realm_reading = {
        let (mut probe, realm) = agent_and_realm(|limits| limits.steps_per_entry = 1 << 40);
        warmed(&mut probe, &realm)
    };
    let (mut agent, realm) = agent_and_realm(|limits| {
        limits.steps_per_entry = 1 << 40;
        limits.memory_bytes = realm_reading + room;
    });
    let before = warmed(&mut agent, &realm);
    assert_eq!(before, realm_reading, "two agents made alike");
    let script = support::script::parse(&mut agent, &realm, endless);
    let outcome = agent.evaluate_script(&script).unwrap();
    let at_the_end = agent.heap_bytes().unwrap();
    assert!(
        matches!(outcome, HostOutcome::Terminated(Termination::MemoryBudget)),
        "{outcome:?}"
    );
    drop(script);
    let after = collected(&mut agent);
    // Read after the last reading of the heap: the answer is a String the host reads.
    let Outcome::Normal(grouped) = eval_text(&mut agent, &realm, "grouped;") else {
        panic!("grouped is not readable");
    };
    let grouped: u64 = grouped.parse().unwrap();
    println!(
        "Map.groupBy over an endless iterable: heap_bytes(): {before} before the script, budget \
         {}, {at_the_end} at its end, {after} after collect(); {grouped} elements grouped",
        before + room
    );
    assert!(
        at_the_end > before,
        "{at_the_end} at the end, {before} before"
    );
    assert!(
        grouped >= MEMORY_GROUPS,
        "{grouped} elements grouped before the budget"
    );
    assert!(after <= before, "{after} after collect(), {before} before");
}

/// `Limits::min_stack_bytes` through `Map.prototype.getOrInsertComputed` (decision 3, "Depth
/// bounds"): the callback is a call from Rust, a re-entry, so `f` re-enters itself through it
/// until the re-entry limit, on seven eighths of `min_stack_bytes()`, in a child process. In
/// the collect mode the depth is [`BUILTINS_REENTRY`].
#[test]
fn get_or_insert_computed_reaches_the_reentry_limit_on_the_promised_stack() {
    fn body() {
        reach_through("new Map().getOrInsertComputed(0, f)", BUILTINS_REENTRY);
    }
    let stack = stack_with_room(stack_limits(BUILTINS_REENTRY));
    on_bounded_stack(
        test_path!("get_or_insert_computed_reaches_the_reentry_limit_on_the_promised_stack"),
        stack,
        body,
    )
    .unwrap();
}

/// `Limits::min_stack_bytes` through `Map.groupBy` (decision 3, "Depth bounds"): the callback
/// is a call from Rust, a re-entry, so `f` re-enters itself through it until the re-entry
/// limit, on seven eighths of `min_stack_bytes()`, in a child process. In the collect mode the
/// depth is [`BUILTINS_REENTRY`].
#[test]
fn map_group_by_reaches_the_reentry_limit_on_the_promised_stack() {
    fn body() {
        reach_through("Map.groupBy([0], f)", BUILTINS_REENTRY);
    }
    let stack = stack_with_room(stack_limits(BUILTINS_REENTRY));
    on_bounded_stack(
        test_path!("map_group_by_reaches_the_reentry_limit_on_the_promised_stack"),
        stack,
        body,
    )
    .unwrap();
}

/// The elements of the test of the groups across a collection.
const KEPT_ELEMENTS: u32 = 50;

/// The result of `Map.groupBy` is reachable from a root while its loop runs
/// (`story:map-upsert-and-group-by`, Change): over 50 elements, each value an object that `next`
/// makes (`{ i: i, pad: [i, i + 1] }`) and each key made by the callback (a new object
/// `{ tag: i }` for every fifth element, otherwise a new String `"s" + i % 3`), with nothing else
/// holding a key or a group, the result read once in its order is the text this test computes:
/// each group's key (an object key by its `tag`), then each value of the group in order with its
/// `pad`, so every key is the one the callback made and every value is whole. With the feature
/// `collect-every-safe-point`, which collects at every safe point of the loop, of `next` and of
/// the callback, the answer is the same as in the default mode. The script runs once, in sloppy
/// code: in that mode a second run in strict code doubled the test's time, to 3.8 CPU seconds
/// alone with the check of each value written in the script, measured on 2026-11-27.
#[test]
fn map_group_by_keeps_its_groups_across_a_collection() {
    let mut groups: Vec<(String, Vec<u32>)> = Vec::new();
    for i in 0..KEPT_ELEMENTS {
        let key = if i % 5 == 0 {
            format!("o{i}")
        } else {
            format!("s{}", i % 3)
        };
        match groups.iter_mut().find(|(k, _)| *k == key) {
            Some((_, values)) => values.push(i),
            None => groups.push((key, vec![i])),
        }
    }
    let expected = groups
        .iter()
        .map(|(key, values)| {
            let values: Vec<String> = values
                .iter()
                .map(|i| format!("{i}/{i}/{}", i + 1))
                .collect();
            format!("{key}:{}", values.join("."))
        })
        .collect::<Vec<_>>()
        .join(" ");
    let outcome = support::script::run(
        limits(),
        &format!(
            "var it = {{}};\n\
             it[Symbol.iterator] = function () {{ var i = 0; return {{ next: function () {{\n\
               if (i >= {KEPT_ELEMENTS}) return {{ done: true }};\n\
               var v = {{ i: i, pad: [i, i + 1] }}; i++; return {{ done: false, value: v }}; }} }}; }};\n\
             var g = Map.groupBy(it, function (v) {{ return v.i % 5 === 0 ? {{ tag: v.i }} : \"s\" + v.i % 3; }});\n\
             var parts = [];\n\
             g.forEach(function (values, key) {{\n\
               parts.push((typeof key === \"string\" ? key : \"o\" + key.tag) + \":\" + values.map(function (v) {{ return v.i + \"/\" + v.pad[0] + \"/\" + v.pad[1]; }}).join(\".\"));\n\
             }});\n\
             parts.join(\" \");"
        ),
    );
    assert_eq!(outcome, Outcome::Normal(format!("\"{expected}\"")));
}
