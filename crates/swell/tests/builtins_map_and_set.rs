//! `Map` and `Set` (`#sec-map-objects`, `#sec-set-objects`) with their iterators
//! (`#sec-map-iterator-objects`, `#sec-set-iterator-objects`), on the entry table of
//! `story:keyed-entry-table` (`story:map-and-set`): insertion order, SameValueZero keys, the
//! cursors of `forEach` and of the iterators, the constructors with AddEntriesFromIterable
//! (`#sec-add-entries-from-iterable`) and their adders, `@@species`, the depth bounds, the step
//! and memory budgets, the interrupt, and the release of the cursors of iterators that never
//! finish. Run also with the feature `collect-every-safe-point`.
//!
//! The counts of the heavy tests are smaller with that feature, where every instruction
//! collects a heap that holds the collection, in the gate and in the lane `slow` alike.
#![cfg(test)]

use crate::support;

use support::host::{TestHost, agent, agent_with, limits, realm, stack_with_room};
use support::script::{Outcome, define_function, eval, eval_text, run_both};
use support::stack::{BUILTINS_REENTRY, on_bounded_stack, reach_through, stack_limits, test_path};
use swell::{
    Agent, Completion, Cx, HostCall, InterruptHandle, Limits, Outcome as HostOutcome, Realm,
    Termination, Value,
};

/// `source` answers the String `expected` in sloppy and in strict code.
fn answers(source: &str, expected: &str) {
    let (sloppy, strict) = run_both(source);
    let expected = Outcome::Normal(format!("\"{expected}\""));
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

/// Whether the feature `collect-every-safe-point` is on: the heavy tests take their smaller
/// counts.
fn collect_mode() -> bool {
    cfg!(feature = "collect-every-safe-point")
}

/// The bytes of one place of the cursor list of an entry table (`story:keyed-entry-table`: "A
/// place of the cursor list is 32 bytes"), which a cursor left open keeps charged.
const CURSOR_PLACE_BYTES: u64 = 32;

/// The least bytes one entry of a table holds in its storage: its key and its value, each at
/// least a `Value`.
fn entry_bytes() -> u64 {
    2 * u64::try_from(size_of::<Value>()).unwrap()
}

/// The iterables the scripts write: `iterable(values, ret)` answers the elements of the Array
/// `values` in order, then done, with `ret` as its `return` when given; `closing(values)` does
/// the same and counts the calls of its `return` in `returns`; `caught(f)` answers what `f`
/// threw, "TypeError" for a `TypeError`, or "none".
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

/// `#sec-map.prototype.set`, `#sec-map.prototype.delete`, `#sec-set.prototype.add` and
/// `#sec-set.prototype.delete`: an entry keeps its place when its value is overwritten, a
/// deleted entry leaves the order, and one added again goes to the end; `forEach`, `keys`,
/// `values`, `entries` and the default iterator all visit in that order, a Set's entries as
/// `[v, v]`. The same holds past the size where the table keeps an index map (a Map of 100
/// keys, 20 in the collect mode, whose odd keys and two added again are left). `set` and `add`
/// answer their collection, `delete` whether it held the key.
#[test]
fn map_and_set_keep_insertion_order_through_deletes_and_overwrites() {
    let big: u32 = if collect_mode() { 20 } else { 100 };
    let half = big / 2;
    let source = "var r = [];\n\
         var m = new Map();\n\
         m.set(\"a\", 1); m.set(\"b\", 2); m.set(\"c\", 3); m.delete(\"b\"); m.set(\"a\", 10); m.set(\"b\", 20); m.set(\"d\", 4);\n\
         var k = []; m.forEach(function (v, key) { k.push(key + \"=\" + v); });\n\
         r.push(k.join(\" \"), m.size);\n\
         var s = new Set([3, 1, 2]); s.delete(1); s.add(3); s.add(1); s.add(5);\n\
         var e = []; s.forEach(function (v, v2, set) { e.push(v + (v === v2 && set === s ? \"\" : \"?\")); });\n\
         r.push(e.join(\" \"), s.size);\n\
         r.push([...m.keys()].join(), [...m.values()].join(), [...m.entries()].map(x => x.join(\":\")).join(),\n\
           [...m].map(x => x.join(\":\")).join(), [...s].join(), [...s.keys()].join(),\n\
           [...s.entries()].map(x => x.join(\":\")).join());\n\
         var big = new Map();\n\
         for (var i = 0; i < BIG; i++) big.set(i, i);\n\
         for (var i = 0; i < BIG; i += 2) big.delete(i);\n\
         big.set(0, \"z\"); big.set(1, \"one\");\n\
         var keys = []; for (var x of big.keys()) keys.push(x);\n\
         r.push(keys.length, keys[0], keys[HALF - 1], keys[HALF], big.get(1), big.get(2), big.size);\n\
         r.push(m.set(\"x\", 1) === m, s.add(9) === s, m.delete(\"nope\"), m.delete(\"x\"), m.has(\"x\"), m.get(\"a\"), m.get(\"zz\"));\n\
         r.join(\"|\");"
        .replace("BIG", &big.to_string())
        .replace("HALF", &half.to_string());
    let left = half + 1;
    let last = big - 1;
    answers(
        &source,
        &format!(
            "a=10 c=3 b=20 d=4|4|3 2 1 5|4|a,c,b,d|10,3,20,4|a:10,c:3,b:20,d:4|a:10,c:3,b:20,d:4|\
             3,2,1,5|3,2,1,5|3:3,2:2,1:1,5:5|{left}|1|{last}|0|one||{left}|true|true|false|true|false|10|"
        ),
    );
}

/// `#sec-canonicalizekeyedcollectionkey` and `#sec-samevaluezero`: `-0` and `+0` are one key,
/// kept as `+0`; every NaN is one key; two Strings made apart with the same code units are one
/// key; two objects are two keys; the String "1" and the Number 1 are two keys.
#[test]
fn keys_compare_by_same_value_zero() {
    answers(
        "var r = [];\n\
         var m = new Map();\n\
         m.set(0, \"zero\"); r.push(m.get(-0), m.has(-0), m.size);\n\
         m.set(-0, \"neg\"); r.push(m.size, m.get(0), Object.is([...m.keys()][0], 0), 1 / [...m.keys()][0]);\n\
         m.set(NaN, \"nan\"); r.push(m.get(NaN), m.get(0 / 0), m.has(Number(\"x\")));\n\
         var a = \"ab\" + \"c\", b = [\"a\", \"bc\"].join(\"\"); m.set(a, 1); m.set(b, 2); r.push(m.get(\"abc\"), m.size);\n\
         var o1 = {}, o2 = {}; m.set(o1, \"o1\"); r.push(m.get(o2), m.get(o1));\n\
         m.set(\"1\", \"s\"); m.set(1, \"n\"); r.push(m.get(\"1\"), m.get(1));\n\
         var s = new Set([-0, 0, NaN, NaN]); r.push(s.size, 1 / [...s][0], s.has(0), s.has(-0), s.delete(-0), s.size);\n\
         r.join();",
        "zero,true,1,1,neg,true,Infinity,nan,nan,true,2,3,,o1,s,n,2,Infinity,true,true,true,1",
    );
}

/// `#sec-map.prototype.foreach` and `#sec-set.prototype.foreach`, note 1: an entry added
/// during the loop is visited, one deleted before the loop reaches it is not, and one deleted
/// after its visit and added again is visited again; the callback gets the value, the key and
/// the collection, with `thisArg` as its this value. A callback that is not callable throws a
/// `TypeError`, as does a this value that is not a collection of the method's kind.
#[test]
fn for_each_visits_an_entry_added_during_the_loop_and_skips_one_deleted_before_it() {
    answers(
        "var r = [];\n\
         var m = new Map([[1, \"a\"], [2, \"b\"], [3, \"c\"]]);\n\
         var seen = [];\n\
         m.forEach(function (v, k, map) {\n\
           seen.push(k + v);\n\
           if (k === 1) { map.delete(2); map.set(4, \"d\"); }\n\
           if (k === 3 && v === \"c\") { map.delete(1); map.set(1, \"again\"); }\n\
         });\n\
         r.push(seen.join());\n\
         var s = new Set([\"x\", \"y\", \"z\"]), t = [];\n\
         s.forEach(function (v) { t.push(v); if (v === \"x\") { s.delete(\"y\"); s.add(\"w\"); } });\n\
         r.push(t.join());\n\
         var self = {}, th;\n\
         new Map([[1, 1]]).forEach(function () { \"use strict\"; th = this; }, self);\n\
         r.push(th === self, new Set().forEach(function () {}));\n\
         r.join(\"|\");",
        "1a,3c,4d,1again|x,z,w|true|",
    );
    for source in [
        "new Map().forEach(1);",
        "new Set().forEach();",
        "Map.prototype.forEach.call(new Set(), function () {});",
        "Set.prototype.forEach.call(new Map(), function () {});",
        "Map.prototype.forEach.call({}, function () {});",
    ] {
        throws(source, "TypeError");
    }
}

/// `#sec-map.prototype.clear`, note, and `#sec-%mapiteratorprototype%.next`: an iterator that
/// ran part of the way before `clear` sees only the entries added after it; one made before
/// `clear` and not yet run sees them too; an iterator that answered done answers done for ever,
/// also after an entry is added. `next` of one kind's iterator refuses the other kind's, and
/// every other this value, with a `TypeError`.
#[test]
fn an_iterator_after_clear_sees_only_entries_added_after() {
    answers(
        "var r = [];\n\
         var m = new Map([[1, 1], [2, 2], [3, 3]]);\n\
         var it = m.keys(); var first = it.next().value;\n\
         m.clear(); m.set(9, 9); m.set(8, 8);\n\
         var rest = [], n; while (!(n = it.next()).done) rest.push(n.value);\n\
         var after = it.next(); m.set(7, 7); var still = it.next();\n\
         r.push(first + \":\" + rest.join() + \":\" + after.done + after.value + \":\" + still.done + \":\" + m.size);\n\
         var s = new Set([\"a\", \"b\"]); var v = s.values(); s.clear(); s.add(\"n\");\n\
         r.push(v.next().value, v.next().done, s.size);\n\
         r.push(Object.prototype.toString.call(m.keys()), Object.prototype.toString.call(s.values()));\n\
         r.join(\"|\");",
        "1:9,8:trueundefined:true:3|n|true|1|[object Map Iterator]|[object Set Iterator]",
    );
    for source in [
        "Object.getPrototypeOf(new Map().keys()).next.call(new Set().values());",
        "Object.getPrototypeOf(new Set().values()).next.call(new Map().entries());",
        "Object.getPrototypeOf(new Map().keys()).next.call({});",
        "Object.getPrototypeOf(new Set().values()).next.call([][Symbol.iterator]());",
        "Map.prototype.keys.call(new Set());",
        "Set.prototype.values.call(new Map());",
    ] {
        throws(source, "TypeError");
    }
}

/// `#sec-map-iterable` and `#sec-set-iterable`: the constructor reads `set` or `add` of the new
/// object (through its prototype) once, after making it and only when an iterable is given,
/// and calls it for each entry; one that is not callable throws a `TypeError`. The prototype
/// comes from NewTarget. A call without `new`, an iterable that is no iterable, and an entry
/// of a Map that is no object throw a `TypeError`.
#[test]
fn the_constructor_calls_the_adder_of_the_prototype() {
    answers(
        "var r = [];\n\
         var calls = 0, original = Map.prototype.set;\n\
         Map.prototype.set = function (k, v) { calls++; return original.call(this, k, v); };\n\
         var m = new Map([[1, 2], [3, 4]]);\n\
         r.push(calls, m.size, m.get(3));\n\
         Map.prototype.set = 5;\n\
         try { new Map([]); r.push(\"none\"); } catch (e) { r.push(e.name); }\n\
         r.push(new Map().size, new Map(undefined).size, new Map(null).size);\n\
         Map.prototype.set = original;\n\
         var adds = 0, add = Set.prototype.add;\n\
         Set.prototype.add = function (v) { adds++; return add.call(this, v); };\n\
         var s = new Set(\"abca\"); r.push(adds, s.size);\n\
         Set.prototype.add = add;\n\
         function F() {} F.prototype = Object.create(Map.prototype);\n\
         var sub = Reflect.construct(Map, [[[1, 1]]], F);\n\
         r.push(Object.getPrototypeOf(sub) === F.prototype, sub.get(1));\n\
         r.join();",
        "2,2,4,TypeError,0,0,0,4,3,true,1",
    );
    for source in [
        "Map();",
        "Set();",
        "Map([]);",
        "new Map(1);",
        "new Map({});",
        "new Map([1]);",
        "new Set(1);",
    ] {
        throws(source, "TypeError");
    }
}

/// `#sec-add-entries-from-iterable`, step 2.c, and IfAbruptCloseIterator: an entry that is no
/// object throws a `TypeError` after `return` was called once; a throw of reading its `0`
/// closes the iterator too. The end of the iteration and a throw of `next` call no `return`.
#[test]
fn the_constructor_closes_the_iterator_when_an_entry_is_not_an_object() {
    answers(
        &[
            ITERABLES,
            "var r = [];\n\
             r.push(caught(function () { new Map(closing([1])); }), returns);\n\
             r.push(caught(function () { new Map(closing([[1, 1], \"s\"])); }), returns);\n\
             r.push(caught(function () { new Map(closing([null])); }), returns);\n\
             r.push(caught(function () { new Map(closing([{ get 0() { throw \"zero\"; } }])); }), returns);\n\
             r.push(caught(function () { new Map(closing([{ get 1() { throw \"one\"; } }])); }), returns);\n\
             r.push(caught(function () { new Map(closing([[1, 1]])); }), returns);\n\
             r.push(caught(function () { new Set(closing([1, 2])); }), returns);\n\
             var o = {}; o[Symbol.iterator] = function () { return { next: function () { throw \"next\"; }, \"return\": function () { returns++; return {}; } }; };\n\
             r.push(caught(function () { new Map(o); }), caught(function () { new Set(o); }), returns);\n\
             r.join();",
        ]
        .concat(),
        "TypeError,1,TypeError,2,TypeError,3,zero,4,one,5,none,5,none,5,next,next,5",
    );
}

/// `#sec-add-entries-from-iterable`, step 2.i, and `#sec-set-iterable`, step 8.c: a throw of
/// the adder closes the iterator, calling its `return` once, and the adder's throw is what the
/// constructor throws, also when `return` throws itself. The entries added before stay in the
/// object, which no one holds.
#[test]
fn a_throw_of_the_adder_closes_the_iterator_once() {
    answers(
        &[
            ITERABLES,
            "var r = [];\n\
             var set = Map.prototype.set, add = Set.prototype.add, n = 0;\n\
             Map.prototype.set = function (k, v) { if (++n === 2) throw \"adder\"; return set.call(this, k, v); };\n\
             r.push(caught(function () { new Map(closing([[1, 1], [2, 2], [3, 3]])); }), returns, n);\n\
             n = 0;\n\
             r.push(caught(function () { new Map(iterable([[1, 1], [2, 2]], function () { returns++; throw \"return\"; })); }), returns);\n\
             Map.prototype.set = set;\n\
             Set.prototype.add = function () { throw \"add\"; };\n\
             r.push(caught(function () { new Set(closing([1, 2])); }), returns);\n\
             Set.prototype.add = add;\n\
             r.join();",
        ]
        .concat(),
        "adder,1,2,adder,2,add,3",
    );
}

/// `#sec-get-map-%symbol.species%` and `#sec-get-set-%symbol.species%`: an accessor with no
/// setter, not enumerable, configurable, whose getter is named "get [Symbol.species]", has
/// "length" 0 and answers its this value, whatever it is.
#[test]
fn species_getters_answer_this() {
    answers(
        "var r = [];\n\
         for (var C of [Map, Set]) {\n\
           var d = Object.getOwnPropertyDescriptor(C, Symbol.species);\n\
           r.push(d.get.name, d.get.length, d.set, d.enumerable, d.configurable, C[Symbol.species] === C,\n\
             d.get.call(1), d.get.call(undefined), d.get.call(Math) === Math);\n\
         }\n\
         r.join();",
        "get [Symbol.species],0,,false,true,true,1,,true,\
         get [Symbol.species],0,,false,true,true,1,,true",
    );
}

/// `Limits::min_stack_bytes` through `Map.prototype.forEach` (decision 3, "Depth bounds"): the
/// callback is a call from Rust, a re-entry, so `f` re-enters itself through it until the
/// re-entry limit, on seven eighths of `min_stack_bytes()`, in a child process. In the collect
/// mode the depth is [`BUILTINS_REENTRY`].
#[test]
fn map_for_each_reaches_the_reentry_limit_on_the_promised_stack() {
    fn body() {
        reach_through("new Map([[0, 0]]).forEach(f)", BUILTINS_REENTRY);
    }
    let stack = stack_with_room(stack_limits(BUILTINS_REENTRY));
    on_bounded_stack(
        test_path!("map_for_each_reaches_the_reentry_limit_on_the_promised_stack"),
        stack,
        body,
    )
    .unwrap();
}

/// `Limits::min_stack_bytes` through the `Map` constructor: a user `next` of the iterable is a
/// call from Rust, a re-entry, so `f` re-enters itself through it until the re-entry limit, on
/// seven eighths of `min_stack_bytes()`, in a child process.
#[test]
fn the_map_constructor_reaches_the_reentry_limit_on_the_promised_stack() {
    fn body() {
        reach_through(
            "new Map({ [Symbol.iterator]() { return { next() { f(); return { done: true }; } }; } })",
            BUILTINS_REENTRY,
        );
    }
    let stack = stack_with_room(stack_limits(BUILTINS_REENTRY));
    on_bounded_stack(
        test_path!("the_map_constructor_reaches_the_reentry_limit_on_the_promised_stack"),
        stack,
        body,
    )
    .unwrap();
}

/// An agent with [`limits`] changed by `change` and its realm.
fn agent_and_realm(change: impl FnOnce(&mut Limits)) -> (Agent<TestHost>, Realm) {
    let mut limits = limits();
    change(&mut limits);
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    (agent, realm)
}

/// The step budget (`docs/design/engine.md`, "The step budget"): a loop that fills a Map
/// without end, with `steps_per_entry` N (100,000; 2,000 in the collect mode), ends in
/// `Terminated(StepBudget)`, and `last_entry_steps()` is at most N.
#[test]
fn a_map_filled_in_a_loop_ends_at_the_step_budget() {
    let budget = if collect_mode() { 2_000 } else { 100_000 };
    let (mut agent, realm) = agent_and_realm(|limits| limits.steps_per_entry = budget);
    let outcome = eval_text(
        &mut agent,
        &realm,
        "var m = new Map(); for (var i = 0; ; i++) m.set(i, i);",
    );
    let steps = agent.last_entry_steps().unwrap();
    println!("{steps} steps of {budget}");
    assert_eq!(outcome, Outcome::Terminated(Termination::StepBudget));
    assert!(steps <= budget, "{steps} steps of {budget}");
}

/// Collect and answer `heap_bytes()` after it.
fn collected(agent: &mut Agent<TestHost>) -> u64 {
    assert!(matches!(agent.collect().unwrap(), HostOutcome::Normal(_)));
    agent.heap_bytes().unwrap()
}

/// The memory budget (`docs/design/engine.md`, "Memory budget"): a Map filled without end in a
/// function's scope, with a budget of the realm's reading plus 256 KiB (16 KiB in the collect
/// mode), after a first script of the same shape that grows the arenas a script needs, ends in
/// `Terminated(MemoryBudget)`; once the host dropped the script, one `collect()` brings
/// `heap_bytes()` back within the reading before the script, the table and the Map credited.
/// The readings are printed.
#[test]
fn a_map_filled_in_a_loop_ends_at_the_memory_budget_and_collect_credits_it() {
    let room: u64 = if collect_mode() { 16 << 10 } else { 256 << 10 };
    // The realm's reading, read in a probe agent made alike: the agent of the test is made
    // with the budget.
    // A first script of the same shape grows the arenas a script needs (a script record,
    // environments), which stay charged as an arena's capacity does, before the reading.
    let warmed = |agent: &mut Agent<TestHost>, realm: &Realm| {
        assert_eq!(
            eval_text(
                agent,
                realm,
                "(function () { var m = new Map(); for (var i = 0; i < 8; i++) m.set(i, i); })();"
            ),
            Outcome::Normal("undefined".to_string())
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
    let script = support::script::parse(
        &mut agent,
        &realm,
        "(function () { var m = new Map(); for (var i = 0; ; i++) m.set(i, i); })();",
    );
    let outcome = agent.evaluate_script(&script).unwrap();
    let at_the_end = agent.heap_bytes().unwrap();
    assert!(
        matches!(outcome, HostOutcome::Terminated(Termination::MemoryBudget)),
        "{outcome:?}"
    );
    drop(script);
    let after = collected(&mut agent);
    println!(
        "heap_bytes(): {before} before the script, budget {}, {at_the_end} at its end, {after} \
         after collect()",
        before + room
    );
    assert!(at_the_end > before + room / 2, "{at_the_end}");
    assert!(after <= before, "{after} after collect(), {before} before");
}

/// The memory budget, both ways: `new Set` of 10,000 Numbers (100 in the collect mode) raises
/// `heap_bytes()` by at least that many times the bytes of an entry ([`entry_bytes`], a key and
/// a value: 32), and once the Set is unreachable one `collect()` brings `heap_bytes()` back
/// to the reading before it. The readings are printed.
#[test]
fn a_set_of_10000_keys_is_charged_and_credited() {
    let count: u64 = if collect_mode() { 100 } else { 10_000 };
    let (mut agent, realm) = agent_and_realm(|limits| limits.steps_per_entry = 1 << 40);
    assert_eq!(
        eval_text(&mut agent, &realm, "var s, i;"),
        Outcome::Normal("undefined".to_string())
    );
    let before = collected(&mut agent);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            &format!("s = new Set(); for (i = 0; i < {count}; i++) s.add(i); s.size;")
        ),
        Outcome::Normal(count.to_string())
    );
    let full = collected(&mut agent);
    assert_eq!(
        eval_text(&mut agent, &realm, "s = undefined;"),
        Outcome::Normal("undefined".to_string())
    );
    let after = collected(&mut agent);
    println!(
        "heap_bytes(): {before} before, {full} with the Set of {count} keys, {after} after; \
         {} bytes an entry at the least",
        entry_bytes()
    );
    assert!(
        full >= before + count * entry_bytes(),
        "{full} with the Set, {before} before"
    );
    assert!(after <= before, "{after} after, {before} before");
}

thread_local! {
    /// The handle `interrupt_now` sets.
    static HANDLE: std::cell::Cell<Option<InterruptHandle>> = const { std::cell::Cell::new(None) };
}

/// A host function that sets its agent's interrupt flag and returns `1`, counting its calls.
fn interrupt_now(cx: &mut Cx<'_, TestHost>, _call: HostCall<'_>) -> Completion<Value> {
    cx.host_mut().count += 1;
    HANDLE.with(|slot| {
        if let Some(handle) = slot.take() {
            handle.interrupt();
        }
    });
    Ok(Value::Number(1.0))
}

/// The interrupt (`docs/design/host-interface.md`, decision 12): set by a host function the
/// callback of `forEach` calls, it ends the run with `Terminated(Interrupted)` after the host
/// function returns; the callback ran once, and the Map keeps its entries.
#[test]
fn a_for_each_ends_at_an_interrupt() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    define_function(&mut agent, &realm, "interruptNow", 0, interrupt_now, false);
    HANDLE.with(|slot| slot.set(Some(agent.interrupt_handle())));
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var n = 0, m = new Map([[1, 1], [2, 2], [3, 3]]);\n\
             m.forEach(function () { n++; interruptNow(); n++; });"
        ),
        Outcome::Terminated(Termination::Interrupted)
    );
    assert_eq!(agent.host().count, 1);
    assert_eq!(
        eval_text(&mut agent, &realm, "n + \",\" + m.size;"),
        Outcome::Normal("\"1,3\"".to_string())
    );
}

/// Run `source` in `agent`, which must answer normally, and answer `last_entry_steps()`.
fn steps_of(agent: &mut Agent<TestHost>, realm: &Realm, source: &str) -> u64 {
    let outcome = eval(agent, realm, source);
    let steps = agent.last_entry_steps().unwrap();
    assert!(matches!(outcome, HostOutcome::Normal(_)), "{outcome:?}");
    steps
}

/// Two agents made alike, for the tests that run a script on both and a part of it on one.
fn twin_agents() -> [(Agent<TestHost>, Realm); 2] {
    std::array::from_fn(|_| agent_and_realm(|limits| limits.steps_per_entry = 1 << 40))
}

/// The keys of the Map the cursor tests compact: 1,000, 32 in the collect mode, where every
/// builtin call collects.
fn table_keys() -> u32 {
    if collect_mode() { 32 } else { 1_000 }
}

/// Deletes keys 0 to 599 of `m` and adds keys 1,000 to 1,024: with 1,000 keys before, the
/// storage of 1,024 places is full at the last addition with 600 holes, at least half of it,
/// so that addition compacts the table, walking every place of its cursor list
/// (`story:keyed-entry-table`). In the collect mode, with 32 keys before in a full storage of
/// 32 places, it deletes keys 0 to 19 and adds 32 and 33: the first addition compacts.
fn compacting() -> &'static str {
    if collect_mode() {
        "for (i = 0; i < 20; i++) m.delete(i); for (i = 32; i < 34; i++) m.set(i, i);"
    } else {
        "for (i = 0; i < 600; i++) m.delete(i); for (i = 1000; i < 1025; i++) m.set(i, i);"
    }
}

/// The cursors of iterators the script dropped (`story:keyed-entry-table`, "Who releases a
/// cursor"): two agents made alike run one script. Entry 1 makes `m`, a Map of [`table_keys`]
/// keys; entry 2 runs `for (k of m) break;` N times (2,000; 30 in the collect mode) in the first
/// agent and once in the second, so the engine's own storage grows alike in both, each loop
/// leaving an iterator unfinished with its cursor at position 1; then the host collects both agents and reads `live_bytes()`; entry 3
/// deletes and adds keys so that the table compacts ([`compacting`]). The `last_entry_steps()`
/// of entry 3 differ by less than N / 10, where N cursors left would add N steps of the
/// compaction, and the `live_bytes()` by less than N / 10 places of the cursor list
/// ([`CURSOR_PLACE_BYTES`]), where N cursors left would hold at least N.
#[test]
fn abandoned_iterators_release_their_cursors_at_collect() {
    let n: u64 = if collect_mode() { 30 } else { 2_000 };
    let mut agents = twin_agents();
    for (agent, realm) in &mut agents {
        steps_of(
            agent,
            realm,
            &format!(
                "var m = new Map(), i, j, k; for (i = 0; i < {}; i++) m.set(i, i);",
                table_keys()
            ),
        );
    }
    for ((agent, realm), loops) in agents.iter_mut().zip([n, 1]) {
        steps_of(
            agent,
            realm,
            &format!("for (j = 0; j < {loops}; j++) {{ for (k of m) break; }}"),
        );
    }
    let mut live = Vec::new();
    for (agent, _) in &mut agents {
        collected(agent);
        live.push(agent.live_bytes().unwrap());
    }
    let steps: Vec<u64> = agents
        .iter_mut()
        .map(|(agent, realm)| steps_of(agent, realm, compacting()))
        .collect();
    println!("abandoned loops {n}: live_bytes() {live:?}, steps of the compacting entry {steps:?}");
    assert!(
        steps[0].abs_diff(steps[1]) < n / 10,
        "steps {steps:?} differ by N / 10 or more"
    );
    assert!(
        live[0].abs_diff(live[1]) < n / 10 * CURSOR_PLACE_BYTES,
        "live_bytes() {live:?} differ by N / 10 places or more"
    );
}

/// `forEach` closes its cursor on an abrupt exit: on one Map of 3 entries, a callback that
/// throws a primitive is run by `forEach` N times (1,000; 40 in the collect mode), each throw
/// caught; `live_bytes()` read after a `collect()` before the throws and after them rises by
/// less than N / 10 places of the cursor list, where N cursors left would add N. A cursor of
/// `forEach` has no owner, so no collection releases one the throw left.
#[test]
fn a_throw_in_for_each_closes_its_cursor() {
    let n: u64 = if collect_mode() { 40 } else { 1_000 };
    let (mut agent, realm) = agent_and_realm(|limits| limits.steps_per_entry = 1 << 40);
    steps_of(
        &mut agent,
        &realm,
        "var m = new Map([[0, 0], [1, 1], [2, 2]]), j, thrown = 0;",
    );
    collected(&mut agent);
    let before = agent.live_bytes().unwrap();
    steps_of(
        &mut agent,
        &realm,
        &format!(
            "for (j = 0; j < {n}; j++) {{ try {{ m.forEach(function () {{ throw 0; }}); }} catch (e) {{ thrown++; }} }}"
        ),
    );
    collected(&mut agent);
    let after = agent.live_bytes().unwrap();
    assert_eq!(
        eval_text(&mut agent, &realm, "thrown;"),
        Outcome::Normal(n.to_string())
    );
    println!("live_bytes() {before} before {n} throws, {after} after");
    assert!(
        after < before + n / 10 * CURSOR_PLACE_BYTES,
        "live_bytes() {before} before, {after} after"
    );
}

/// An iterator closes its cursor when it answers done, and `forEach` on a normal exit, each
/// without a collection: two agents made alike run one script in four entries, the loops of
/// entry 2 in the first agent only. Entry 1 makes `m`, a Map of 2 entries, and `kept`; entry 2
/// runs N iterators `m.keys()` (1,000; 10 in the collect mode) each to its end by `for`-`of`,
/// each kept in `kept`, then N calls of `forEach` that return normally; entry 3 adds keys from 2 up to
/// [`table_keys`]; entry 4 compacts the table ([`compacting`]). The host collects nothing, and a
/// collection at a safe point releases none of these cursors: each iterator is kept and a
/// cursor of `forEach` has no owner. The `last_entry_steps()` of entry 4 differ by less than N
/// / 10, where a cursor left by each finished loop, 2N in all, would add 2N steps.
#[test]
fn finished_loops_leave_no_cursor() {
    let n: u64 = if collect_mode() { 10 } else { 1_000 };
    let mut agents = twin_agents();
    for (agent, realm) in &mut agents {
        steps_of(
            agent,
            realm,
            "var m = new Map([[0, 0], [1, 1]]), kept = [], i, j, k, it;",
        );
    }
    {
        let (agent, realm) = &mut agents[0];
        steps_of(
            agent,
            realm,
            &format!(
                "for (j = 0; j < {n}; j++) {{ it = m.keys(); for (k of it) {{}} kept.push(it); }}\n\
                 for (j = 0; j < {n}; j++) m.forEach(function () {{}});\n\
                 kept.length;"
            ),
        );
    }
    let steps: Vec<u64> = agents
        .iter_mut()
        .map(|(agent, realm)| {
            steps_of(
                agent,
                realm,
                &format!("for (i = 2; i < {}; i++) m.set(i, i);", table_keys()),
            );
            steps_of(agent, realm, compacting())
        })
        .collect();
    println!("finished loops {n}: steps of the compacting entry {steps:?}");
    assert!(
        steps[0].abs_diff(steps[1]) < n / 10,
        "steps {steps:?} differ by N / 10 or more"
    );
}
