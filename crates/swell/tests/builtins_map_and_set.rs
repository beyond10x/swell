//! `Map` and `Set` (`#sec-map-objects`, `#sec-set-objects`) with their iterators
//! (`#sec-map-iterator-objects`, `#sec-set-iterator-objects`), on the entry table of
//! `story:keyed-entry-table` (`story:map-and-set`): insertion order, SameValueZero keys, the
//! cursors of `forEach` and of the iterators, the constructors with AddEntriesFromIterable
//! (`#sec-add-entries-from-iterable`) and their adders, `@@species`, the depth bounds, the step
//! and memory budgets, the interrupt, and the release of the cursors of iterators that never
//! finish. Run also with the feature `collect-every-safe-point`.
//!
//! The counts of the heavy tests are smaller with that feature, where every instruction
//! collects a heap that holds the collection. A test whose count the story states runs that
//! count in the lane `slow` (`SWELL_FULL_COUNTS=1`, [`counted`]) and a part of it in the gate,
//! and says how the part reaches what the whole does; the other counts are the same in the gate
//! and in the lane.
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

/// `full` in the lane `slow` (`SWELL_FULL_COUNTS=1`), `gate` otherwise. Where the story's count
/// in the collect mode took more than 20 s of wall time on the build node, the lane takes the
/// largest count measured under it, named at the test.
fn counted<T>(gate: T, full: T) -> T {
    if std::env::var("SWELL_FULL_COUNTS").is_ok_and(|value| value == "1") {
        full
    } else {
        gate
    }
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

/// The memory budget, both ways: `new Set` of 10,000 Numbers raises `heap_bytes()` by at least
/// that many times the bytes of an entry ([`entry_bytes`], a key and a value: 32), and once the
/// Set is unreachable one `collect()` brings `heap_bytes()` back to the reading before it. The
/// readings are printed. In the collect mode the lane `slow` adds 700 ([`counted`]; on the
/// build node 10,000 ran past 60 s, 2,000 took 34 s, 1,000 22 s in the lane and 700 10 s) and
/// the gate 100: an entry not charged leaves the reading short by 32 bytes per entry, and a
/// table not credited keeps its bytes after the collection, at any count.
#[test]
fn a_set_of_10000_keys_is_charged_and_credited() {
    let count: u64 = if collect_mode() {
        counted(100, 700)
    } else {
        10_000
    };
    let (mut agent, realm) = agent_and_realm(|limits| limits.steps_per_entry = 1 << 40);
    // A Set made and dropped before the first reading: at some sizes of the realm the first
    // object made after it grows the arena of objects, whose room stays charged.
    assert_eq!(
        eval_text(&mut agent, &realm, "var s = new Set(), i; s = undefined;"),
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
/// builtin call collects, in the gate and in the lane `slow` alike: with 1,000 keys in the
/// collect mode and the lane's N the two tests that compact took 27.6 s and 28.3 s on the build
/// node. The compaction walks every place of the cursor list at either size.
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
/// keys; entry 2 runs `for (k of m) break;` N times in the first agent and once in the second,
/// so the engine's own storage grows alike in both, each loop leaving an iterator unfinished
/// with its cursor at position 1; then the host collects both agents and reads `live_bytes()`;
/// entry 3 deletes and adds keys so that the table compacts ([`compacting`]). The
/// `last_entry_steps()` of entry 3 differ by less than N / 10, where N cursors left would add N
/// steps of the compaction, and the `live_bytes()` by less than N / 10 places of the cursor
/// list ([`CURSOR_PLACE_BYTES`]), where N cursors left would hold at least N.
///
/// N is the story's 10,000 (200 in the collect mode) in the lane `slow` ([`counted`]) and
/// 2,000 (30) in the gate. The bound is a tenth of N at each count, so cursors that are never
/// released break it at the gate's N as at the story's: with the release planted out, the
/// steps read [16847, 14848] at 2,000 and [573, 544] at 30.
#[test]
fn abandoned_iterators_release_their_cursors_at_collect() {
    let n: u64 = if collect_mode() {
        counted(30, 200)
    } else {
        counted(2_000, 10_000)
    };
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
/// throws a primitive is run by `forEach` N times, each throw caught; `live_bytes()` read after
/// a `collect()` before the throws and after them rises by less than N / 10 places of the
/// cursor list, where N cursors left would add N. A cursor of `forEach` has no owner, so no
/// collection releases one the throw left. N is 1,000; in the collect mode the story's 100 in
/// the lane `slow` ([`counted`]) and 40 in the gate, where the places a throw that skips the
/// close leaves still break the bound (planted: 264744 before, 266792 after at 40, bound 128).
#[test]
fn a_throw_in_for_each_closes_its_cursor() {
    let n: u64 = if collect_mode() {
        counted(40, 100)
    } else {
        1_000
    };
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
/// runs N iterators `m.keys()` each to its end by `for`-`of`, each kept in `kept`, then N calls
/// of `forEach` that return normally; entry 3 adds keys from 2 up to [`table_keys`]; entry 4
/// compacts the table ([`compacting`]). The host collects nothing, and a collection at a safe
/// point releases none of these cursors: each iterator is kept and a cursor of `forEach` has no
/// owner. The `last_entry_steps()` of entry 4 differ by less than N / 10, where a cursor left by
/// each finished loop, 2N in all, would add 2N steps. N is 1,000; in the collect mode the
/// story's 100 in the lane `slow` ([`counted`]) and 10 in the gate, where the steps of
/// cursors left still break the bound (planted: [554, 543] at 10, bound 1).
#[test]
fn finished_loops_leave_no_cursor() {
    let n: u64 = if collect_mode() {
        counted(10, 100)
    } else {
        1_000
    };
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

/// The test limits with a step budget no run of these cases reaches.
fn unbounded_steps() -> Limits {
    let mut limits = limits();
    limits.steps_per_entry = 1 << 40;
    limits
}

/// `source`, run once in sloppy code with [`unbounded_steps`], answers the String `expected`.
fn answers_once(source: &str, expected: &str) {
    let outcome = support::script::run(unbounded_steps(), source);
    assert_eq!(
        outcome,
        Outcome::Normal(format!("\"{expected}\"")),
        "{source}"
    );
}

/// A property test with a fixed seed against the clauses as written (`#sec-map-objects`,
/// `#sec-set-objects`, `#sec-%mapiteratorprototype%.next`, `#sec-map.prototype.foreach`): a
/// model List that never shrinks, a deleted entry `empty`, `clear` empties every entry, and an
/// iterator is an index into the List. A Map and a Set get the same random operations (`set`
/// and `add` of 40 keys: Strings made anew each time, Numbers, Symbols, objects, `+0` and `-0`,
/// two NaNs, `14` and `"14"`, `undefined`, `null`, the Booleans; `delete`; `clear`; up to five
/// live iterators of every kind, one replaced and abandoned at random; `next` of one; and a
/// `forEach` whose callback mutates both). Every answer is compared with the model: the
/// order, the canonical key (`+0`), the value, `done` for ever, `size` and `delete`'s answer.
/// The script runs `ops` operations from `seed` and answers `mismatches <n>[ first: <what>],
/// specials <r>`: `r` of the eight special keys were set at least once.
fn random_operations(seed: u64, ops: u32) -> Outcome {
    let source = r#"
var OPS = __OPS__, K = 40, seed = __SEED__, i, reached = 0, hit = [];
function rnd(n) { seed = (seed * 48271) % 2147483647; return seed % n; }
var syms = [], objs = [];
for (i = 0; i < K; i++) { syms.push(Symbol("y" + i)); objs.push({ n: i }); }
var specials = ["zero", "nan", 14, "14", undefined, null, true, false];
function keyOf(id) {
  var kind = id % 5;
  if (kind === 0) return "s" + id;
  if (kind === 1) return id * 1.5;
  if (kind === 2) return syms[id];
  if (kind === 3) return objs[id];
  var special = specials[(id - 4) / 5];
  if (special === "zero") return rnd(2) === 1 ? -0 : 0;
  if (special === "nan") return rnd(2) === 1 ? NaN : 0 / 0;
  return special;
}
function shown(k) {
  if (typeof k === "symbol") return String(k);
  if (typeof k === "object" && k !== null) return "o" + k.n;
  if (k === 0) return 1 / k === Infinity ? "+0" : "-0";
  if (typeof k === "string") return "'" + k + "'";
  return String(k);
}
function canon(id) { if (id === 4) return "+0"; if (id === 9) return "NaN"; return shown(keyOf(id)); }
var list = [], at = [], size = 0;
for (i = 0; i < K; i++) at.push(-1);
var m = new Map(), s = new Set(), its = [];
var bad = 0, first = "", compares = 0;
function check(ok, what) { compares++; if (!ok) { bad++; if (first === "") first = what; } }
function mset(id, v) { if (at[id] >= 0) list[at[id]].v = v; else { at[id] = list.length; list.push({ id: id, v: v, live: true }); size++; } }
function mdel(id) { if (at[id] < 0) return false; list[at[id]].live = false; at[id] = -1; size--; return true; }
function mclear() { var j; for (j = 0; j < list.length; j++) list[j].live = false; for (j = 0; j < K; j++) at[j] = -1; size = 0; }
function mnext(c) { if (c.done) return null; while (c.idx < list.length) { var e = list[c.idx++]; if (e.live) return e; } c.done = true; return null; }
function doSet(turn) { var id = rnd(K); m.set(keyOf(id), turn); s.add(keyOf(id)); mset(id, turn); if (id % 5 === 4 && !hit[id]) { hit[id] = true; reached++; } }
function doDelete(turn) {
  var id = rnd(K), a = m.delete(keyOf(id)), b = s.delete(keyOf(id)), e = mdel(id);
  check(a === e && b === e, "delete " + canon(id) + " at " + turn + ": " + a + b + e);
}
function doClear() { m.clear(); s.clear(); mclear(); }
function shownResult(c, r) {
  if (r.done) return "done";
  if (c.kind === "entries") return shown(r.value[0]) + "=" + (c.set ? shown(r.value[1]) : String(r.value[1]));
  if (c.kind === "values" && !c.set) return String(r.value);
  return shown(r.value);
}
function compareStep(c, turn) {
  var r = c.it.next(), e = mnext(c);
  if (e === null) { check(r.done === true && r.value === undefined, "turn " + turn + ": done expected, got " + shownResult(c, r)); return; }
  var want = c.kind === "keys" ? canon(e.id) : c.kind === "values" ? (c.set ? canon(e.id) : String(e.v)) : canon(e.id) + "=" + (c.set ? canon(e.id) : String(e.v));
  check(r.done === false && shownResult(c, r) === want, "turn " + turn + " " + c.kind + (c.set ? " of s" : " of m") + ": want " + want + " got " + shownResult(c, r));
}
function newIterator() {
  var set = rnd(2) === 1, kind = ["keys", "values", "entries"][rnd(3)], coll = set ? s : m, it;
  if (kind === "keys") it = coll.keys();
  else if (kind === "values") it = set && rnd(2) === 1 ? s[Symbol.iterator]() : coll.values();
  else it = !set && rnd(2) === 1 ? m[Symbol.iterator]() : coll.entries();
  var c = { it: it, idx: 0, done: false, kind: kind, set: set };
  if (its.length < 5) its.push(c); else its[rnd(5)] = c;
}
function forEachTurn(turn) {
  var set = rnd(2) === 1, c = { idx: 0, done: false }, visits = 0;
  (set ? s : m).forEach(function (v, k, coll) {
    var e = mnext(c);
    var got = set ? shown(k) + (shown(v) === shown(k) ? "" : "!") : shown(k) + "=" + String(v);
    var want = e === null ? "nothing" : set ? canon(e.id) : canon(e.id) + "=" + String(e.v);
    check(coll === (set ? s : m) && got === want, "forEach at " + turn + " visit " + visits + ": want " + want + " got " + got);
    if (visits++ < 30) { var op = rnd(20); if (op < 10) doSet(turn); else if (op < 19) doDelete(turn); else doClear(); }
  });
  check(mnext(c) === null, "forEach at " + turn + " ended before the model");
}
for (var turn = 0; turn < OPS; turn++) {
  var op = rnd(100);
  if (op < 38) doSet(turn);
  else if (op < 62) doDelete(turn);
  else if (op < 63) doClear();
  else if (op < 70) newIterator();
  else if (op < 97) { if (its.length > 0) compareStep(its[rnd(its.length)], turn); }
  else forEachTurn(turn);
  check(m.size === size && s.size === size, "size at " + turn + ": " + m.size + "/" + s.size + " want " + size);
}
var all = []; m.forEach(function (v, k) { all.push(shown(k) + "=" + v); });
var want = []; for (i = 0; i < list.length; i++) if (list[i].live) want.push(canon(list[i].id) + "=" + list[i].v);
check(all.join() === want.join(), "final order " + all.join() + " want " + want.join());
check([...s].map(shown).join() === [...m.keys()].map(shown).join(), "the Set's order differs from the Map's");
"mismatches " + bad + (bad > 0 ? " first: " + first : "") + (compares >= OPS ? "" : " compares " + compares) + ", specials " + reached;
"#
    .replace("__OPS__", &ops.to_string())
    .replace("__SEED__", &seed.to_string());
    support::script::run(unbounded_steps(), &source)
}

/// The operations of each test of [`random_operations`] and the special keys it sets: 1,000
/// and all eight in the default mode; 6 in the collect mode, which reach none (as the 12 of one
/// test did before). One test of 2,000 operations took 2.1 s alone in the default build, and
/// one of 12 took 2.15 s in the lane's collect build (test profile, opt-level 0), of which the
/// script's setup with no operation took 0.88 s and each operation about 0.1 s (2026-11-27), so
/// the story's 2,000 are two tests over two seeds.
fn random_operations_count() -> (u32, u32) {
    if collect_mode() { (6, 0) } else { (1_000, 8) }
}

/// [`random_operations`] from the seed 20261126, the story's seed: the first half of its 2,000
/// operations ([`random_operations_count`]).
#[test]
fn random_operations_match_the_model_list_of_the_clauses() {
    let (ops, specials) = random_operations_count();
    assert_eq!(
        random_operations(20_261_126, ops),
        Outcome::Normal(format!("\"mismatches 0, specials {specials}\""))
    );
}

/// [`random_operations`] from a second seed, 20261127, in place of the second half of the
/// story's 2,000 operations ([`random_operations_count`]).
#[test]
fn random_operations_from_a_second_seed_match_the_model_list_of_the_clauses() {
    let (ops, specials) = random_operations_count();
    assert_eq!(
        random_operations(20_261_127, ops),
        Outcome::Normal(format!("\"mismatches 0, specials {specials}\""))
    );
}

/// `story:map-and-set` and `story:keyed-entry-table`: a compaction in the middle of a
/// `forEach` (the callback deletes ten keys and adds one, which fills the storage with at least
/// half of it holes) leaves every live cursor on the entry it named: four iterators stopped at
/// the first hole, inside the deleted run, on a live entry past it and at the end, and the
/// cursor of the `forEach` itself; once with an index map (16 keys) and once without (8 keys).
#[test]
fn a_compaction_inside_for_each_keeps_five_cursors_on_their_entries() {
    answers_once(
        "function take(it, n) { for (var j = 0; j < n; j++) it.next(); return it; }\n\
         function rest(it) { var out = [], n; while (!(n = it.next()).done) out.push(n.value); return out.join(); }\n\
         function scenario(count, lo, hi, takes) {\n\
           var m = new Map(), i;\n\
           for (i = 0; i < count; i++) m.set(i, i);\n\
           var its = takes.map(function (t) { return take(m.keys(), t); });\n\
           var seen = [];\n\
           m.forEach(function (v, k) {\n\
             seen.push(k);\n\
             if (k === 1) { for (var d = lo; d < hi; d++) m.delete(d); m.set(100, 100); }\n\
           });\n\
           return its.map(rest).join(\"/\") + \"|\" + seen.join() + \"|\" + [...m.keys()].join() + \"|\" + m.size;\n\
         }\n\
         scenario(16, 3, 13, [3, 10, 14, 16]) + \"#\" + scenario(8, 2, 7, [2, 5, 7, 8]);",
        "13,14,15,100/13,14,15,100/14,15,100/100|0,1,2,13,14,15,100|0,1,2,13,14,15,100|7\
         #7,100/7,100/7,100/100|0,1,7,100|0,1,7,100|4",
    );
}

/// `#sec-map-iterable`, `#sec-set-iterable` and `#sec-add-entries-from-iterable`: every
/// observable read and call of the constructors, in order, through accessors on the
/// prototype of NewTarget, the iterable, the iterator, each result and each entry: the adder
/// once and before the iterator is opened; a non-callable adder a `TypeError` before any read
/// of the iterable; an entry that is not an Object closes the iterator; a throw of the adder
/// closes it, a throw of `return` and of its getter swallowed.
#[test]
fn the_constructors_read_and_call_in_the_order_of_the_clauses() {
    answers_once(
        r#"var log = [];
function logged(name, value) { return { get: function () { log.push("get " + name); return value; }, configurable: true }; }
function entry(k, v) { var e = {}; Object.defineProperty(e, "0", logged("0", k)); Object.defineProperty(e, "1", logged("1", v)); return e; }
function iterable(items, ret) {
  var o = {}, i = 0;
  Object.defineProperty(o, Symbol.iterator, logged("@@iterator", function () {
    log.push("call @@iterator");
    var it = {};
    Object.defineProperty(it, "next", logged("next", function () {
      log.push("call next");
      var done = i >= items.length, value = items[i++], res = {};
      Object.defineProperty(res, "done", logged("done", done));
      Object.defineProperty(res, "value", logged("value", value));
      return res;
    }));
    if (ret === "getter throws") Object.defineProperty(it, "return", { get: function () { log.push("get return"); throw "getter"; } });
    else Object.defineProperty(it, "return", logged("return", ret));
    return it;
  }));
  return o;
}
function adderOn(proto, name, adder) { var F = function () {}; F.prototype = Object.create(proto); Object.defineProperty(F.prototype, name, logged(name, adder)); return F; }
function run(C, F, items, ret) {
  log = [];
  try { var o = Reflect.construct(C, [iterable(items, ret)], F); log.push("size " + o.size); }
  catch (e) { log.push("threw " + (e instanceof TypeError ? "TypeError" : e)); }
  return log.join(", ");
}
var mapSet = function (k, v) { log.push("call set " + k + " " + v); return Map.prototype.set.call(this, k, v); };
var setAdd = function (v) { log.push("call add " + v); return Set.prototype.add.call(this, v); };
var closer = function () { log.push("call return"); return {}; };
var thrower = function () { log.push("call return"); throw "return"; };
[
  run(Map, adderOn(Map.prototype, "set", mapSet), [entry("a", 1), entry("b", 2)]),
  run(Set, adderOn(Set.prototype, "add", setAdd), ["x", "y"]),
  run(Map, adderOn(Map.prototype, "set", 1), [entry("a", 1)], closer),
  run(Set, adderOn(Set.prototype, "add", {}), ["x"], closer),
  run(Map, adderOn(Map.prototype, "set", mapSet), [entry("a", 1), 2], closer),
  run(Map, adderOn(Map.prototype, "set", mapSet), [2], thrower),
  run(Map, adderOn(Map.prototype, "set", function () { log.push("call set"); throw "adder"; }), [entry("a", 1)], thrower),
  run(Map, adderOn(Map.prototype, "set", function () { log.push("call set"); throw "adder"; }), [entry("a", 1)], "getter throws"),
  run(Set, adderOn(Set.prototype, "add", function () { log.push("call add"); throw "adder"; }), ["x"], undefined)
].join(" / ");"#,
        "get set, get @@iterator, call @@iterator, get next, call next, get done, get value, get 0, get 1, call set a 1, \
         call next, get done, get value, get 0, get 1, call set b 2, call next, get done, size 2 / \
         get add, get @@iterator, call @@iterator, get next, call next, get done, get value, call add x, \
         call next, get done, get value, call add y, call next, get done, size 2 / \
         get set, threw TypeError / \
         get add, threw TypeError / \
         get set, get @@iterator, call @@iterator, get next, call next, get done, get value, get 0, get 1, call set a 1, \
         call next, get done, get value, get return, call return, threw TypeError / \
         get set, get @@iterator, call @@iterator, get next, call next, get done, get value, get return, call return, threw TypeError / \
         get set, get @@iterator, call @@iterator, get next, call next, get done, get value, get 0, get 1, call set, \
         get return, call return, threw adder / \
         get set, get @@iterator, call @@iterator, get next, call next, get done, get value, get 0, get 1, call set, \
         get return, threw adder / \
         get add, get @@iterator, call @@iterator, get next, call next, get done, get value, call add, get return, threw adder",
    );
}

/// The functions the tests of the surface of `%Map%` and `%Set%` share: `d(o, k)`, the
/// descriptor of `o[k]` written out, `all(o)`, every own key of `o` with its descriptor, `MIP`,
/// `SIP` and `IP`, the iterator prototypes, and `typeErrors(f, own)`, how many of the receivers
/// but `own` make `f` throw a `TypeError`.
const SURFACE: &str = r#"function d(o, k) {
  var x = Object.getOwnPropertyDescriptor(o, k);
  if (x === undefined) return "none";
  var flags = (x.writable ? "w" : "") + (x.enumerable ? "e" : "") + (x.configurable ? "c" : "");
  if ("value" in x) return (typeof x.value === "function" ? x.value.name + "/" + x.value.length : String(x.value)) + ":" + flags;
  return "get " + (x.get ? x.get.name + "/" + x.get.length : "none") + " set " + (x.set === undefined ? "none" : "some") + ":" + flags;
}
function all(o) { return Reflect.ownKeys(o).map(function (k) { return String(k) + " " + d(o, k); }).join("; "); }
var MIP = Object.getPrototypeOf(new Map().keys()), SIP = Object.getPrototypeOf(new Set().values());
var IP = Object.getPrototypeOf(Object.getPrototypeOf([][Symbol.iterator]()));
var receivers = [new Set(), new Map(), {}, 1, "s", undefined, null, Symbol(), new Map().keys(), new Set().values(), Map.prototype, Set.prototype];
function typeErrors(f, own) {
  var n = 0;
  receivers.forEach(function (x) { if (x === own) return; try { f(x); } catch (e) { if (e instanceof TypeError) n++; } });
  return n;
}
"#;

/// The surface of `%Map%`, `%Set%`, their prototypes and both iterator prototypes against
/// clauses 24.1 and 24.2: the own keys in the order of the clauses, every descriptor (`length`,
/// `name`, attributes), the shared function objects and the prototype chains. The brand checks
/// are [`the_methods_of_map_check_their_brand`] and [`the_methods_of_set_and_both_nexts_check_their_brand`]:
/// one script of all three took 6.3 s alone with the feature `collect-every-safe-point` in the
/// test profile (opt-level 0) on 2026-11-27.
#[test]
fn the_surface_of_map_set_and_their_iterators() {
    answers_once(
        &[
            SURFACE,
            r#"var r = [all(Map.prototype), all(Set.prototype), all(MIP), all(SIP),
  [d(Map, "length"), d(Map, "name"), d(Map, "prototype"), d(Map, Symbol.species)].join(";"),
  [d(Set, "length"), d(Set, "name"), d(Set, "prototype"), d(Set, Symbol.species)].join(";")];
r.push([Map.prototype[Symbol.iterator] === Map.prototype.entries, Set.prototype.keys === Set.prototype.values,
  Set.prototype[Symbol.iterator] === Set.prototype.values, Map.prototype.constructor === Map, Set.prototype.constructor === Set,
  Object.getPrototypeOf(Map.prototype) === Object.prototype, Object.getPrototypeOf(Map) === Function.prototype,
  Object.getPrototypeOf(MIP) === IP, Object.getPrototypeOf(SIP) === IP, MIP !== SIP, MIP.next !== SIP.next,
  Object.getPrototypeOf(new Map().entries()) === MIP, Object.getPrototypeOf(new Map().values()) === MIP,
  Object.getPrototypeOf(new Set().entries()) === SIP, (function () { var it = new Map().keys(); return it[Symbol.iterator]() === it; })(),
  Object.prototype.toString.call(new Map()), Object.prototype.toString.call(new Set())].join());
r.join(" | ");"#,
        ]
        .concat(),
        "clear clear/0:wc; constructor Map/0:wc; delete delete/1:wc; entries entries/0:wc; forEach forEach/1:wc; \
         get get/1:wc; getOrInsert getOrInsert/2:wc; getOrInsertComputed getOrInsertComputed/2:wc; \
         has has/1:wc; keys keys/0:wc; set set/2:wc; size get get size/0 set none:c; values values/0:wc; \
         Symbol(Symbol.iterator) entries/0:wc; Symbol(Symbol.toStringTag) Map:c | \
         add add/1:wc; clear clear/0:wc; constructor Set/0:wc; delete delete/1:wc; difference difference/1:wc; \
         entries entries/0:wc; forEach forEach/1:wc; has has/1:wc; intersection intersection/1:wc; \
         isDisjointFrom isDisjointFrom/1:wc; isSubsetOf isSubsetOf/1:wc; isSupersetOf isSupersetOf/1:wc; \
         keys values/0:wc; size get get size/0 set none:c; symmetricDifference symmetricDifference/1:wc; \
         union union/1:wc; values values/0:wc; \
         Symbol(Symbol.iterator) values/0:wc; Symbol(Symbol.toStringTag) Set:c | \
         next next/0:wc; Symbol(Symbol.toStringTag) Map Iterator:c | \
         next next/0:wc; Symbol(Symbol.toStringTag) Set Iterator:c | \
         0:c;Map:c;[object Map]:;get get [Symbol.species]/0 set none:c | \
         0:c;Set:c;[object Set]:;get get [Symbol.species]/0 set none:c | \
         true,true,true,true,true,true,true,true,true,true,true,true,true,true,true,[object Map],[object Set]",
    );
}

/// The brand check of every method and getter of `%Map.prototype%` against eleven other this
/// values (`#sec-map.prototype.clear` and the clauses after it, each step "RequireInternalSlot").
#[test]
fn the_methods_of_map_check_their_brand() {
    answers_once(
        &[
            SURFACE,
            r#"var mapSize = Object.getOwnPropertyDescriptor(Map.prototype, "size").get;
var counts = [];
["clear", "delete", "entries", "forEach", "get", "has", "keys", "set", "values"].forEach(function (name) {
  counts.push(name + typeErrors(function (x) { Map.prototype[name].call(x, function () {}); }, receivers[1]));
});
counts.push("size" + typeErrors(function (x) { mapSize.call(x); }, receivers[1]));
counts.join();"#,
        ]
        .concat(),
        "clear11,delete11,entries11,forEach11,get11,has11,keys11,set11,values11,size11",
    );
}

/// The brand check of every method and getter of `%Set.prototype%` and of `next` of both
/// iterator prototypes against eleven other this values, and `new` of a method, of `next` and a
/// call of `Set` without it, each a `TypeError`.
#[test]
fn the_methods_of_set_and_both_nexts_check_their_brand() {
    answers_once(
        &[
            SURFACE,
            r#"var setSize = Object.getOwnPropertyDescriptor(Set.prototype, "size").get;
var counts = [];
["add", "clear", "delete", "entries", "forEach", "has", "values"].forEach(function (name) {
  counts.push(name + typeErrors(function (x) { Set.prototype[name].call(x, function () {}); }, receivers[0]));
});
counts.push("size" + typeErrors(function (x) { setSize.call(x); }, receivers[0]));
counts.push("mapnext" + typeErrors(function (x) { MIP.next.call(x); }, receivers[8]));
counts.push("setnext" + typeErrors(function (x) { SIP.next.call(x); }, receivers[9]));
counts.push("new" + typeErrors(function (x) { new Map.prototype.set(); }, {}) + typeErrors(function (x) { new MIP.next(); }, {}) + typeErrors(function (x) { Set(); }, {}));
counts.join();"#,
        ]
        .concat(),
        "add11,clear11,delete11,entries11,forEach11,has11,values11,size11,mapnext11,setnext11,new121212",
    );
}

/// Map values keep `-0` (only keys are canonicalized), every way out of the table answers the
/// key `+0` for a key stored as `-0`, and a table past its index map finds `-0`, NaN, a long
/// String made two ways, and keeps `"0"`, `0`, `"-0"` and two Symbols of one description apart.
#[test]
fn minus_zero_values_and_keys_past_the_index_map() {
    answers_once(
        "var m = new Map([[1, -0]]), r = [], i, fe;\n\
         m.forEach(function (v) { fe = Object.is(v, -0); });\n\
         r.push(Object.is(m.get(1), -0), Object.is([...m.values()][0], -0), Object.is([...m][0][1], -0), fe,\n\
           Object.is(new Set([-0]).values().next().value, 0), Object.is(new Map([[-0, 1]]).keys().next().value, 0),\n\
           Object.is([...new Map([[-0, 1]]).entries()][0][0], 0), Object.is([...new Set([-0]).entries()][0][1], 0));\n\
         var b = new Map(); for (i = 0; i < 20; i++) b.set(i, i);\n\
         var long1 = Array(201).join(\"x\"), long2 = Array(101).join(\"x\") + Array(101).join(\"x\");\n\
         b.set(-0, \"z\"); b.set(NaN, \"n\"); b.set(long1, \"l\"); b.set(\"0\", \"s\"); b.set(\"-0\", \"m\");\n\
         var y1 = Symbol(\"y\"), y2 = Symbol(\"y\"); b.set(y1, 1); b.set(y2, 2);\n\
         r.push(b.get(0), b.get(-0), b.get(0 / 0), b.get(long2), b.get(\"0\"), b.get(\"-0\"), b.get(y1), b.get(y2), b.size,\n\
           b.delete(-0), b.has(0), b.delete(long2), b.has(long1), b.size);\n\
         r.join();",
        "true,true,true,true,true,true,true,true,z,z,n,l,s,m,1,2,26,true,false,true,false,24",
    );
}

/// The first value an entry of `realm` completes with, normal or thrown.
fn completion_root(agent: &mut Agent<TestHost>, realm: &Realm, source: &str) -> swell::Root {
    match eval(agent, realm, source) {
        HostOutcome::Normal(root) | HostOutcome::Throw(root) => root,
        HostOutcome::Terminated(termination) => panic!("{source} ended in {termination:?}"),
    }
}

/// A Map of another realm: the methods of this realm work on it (the brand is the slot, not
/// the realm), an iterator made by this realm's `entries` has this realm's
/// `%MapIteratorPrototype%`, and `Reflect.construct(Map, [], F)` with a constructor `F` of the
/// other realm whose `prototype` is not an object takes the other realm's `%Map.prototype%`
/// (GetPrototypeFromConstructor, GetFunctionRealm).
#[test]
fn a_map_of_another_realm() {
    let mut agent = support::host::agent_with(unbounded_steps()).unwrap();
    let a = realm(&mut agent);
    let b = realm(&mut agent);
    let map_b = completion_root(&mut agent, &b, "new Map([[1, \"one\"], [2, \"two\"]]);");
    let f_b = completion_root(
        &mut agent,
        &b,
        "(function () { var F = function () {}; F.prototype = null; return F; })();",
    );
    let probe = completion_root(
        &mut agent,
        &a,
        "(function (m, F) {\n\
           var it = Map.prototype.entries.call(m), size = Object.getOwnPropertyDescriptor(Map.prototype, \"size\").get;\n\
           var out = [Map.prototype.get.call(m, 1), size.call(m), Map.prototype.has.call(m, 2),\n\
             Object.getPrototypeOf(it) === Object.getPrototypeOf(new Map().keys()),\n\
             Object.getPrototypeOf(m.entries()) === Object.getPrototypeOf(new Map().keys()), m instanceof Map,\n\
             Object.getPrototypeOf(Reflect.construct(Map, [], F)) === Object.getPrototypeOf(m),\n\
             Object.getPrototypeOf(Reflect.construct(Set, [], F)) === Object.getPrototypeOf(new Set())];\n\
           var e = [], n; while (!(n = it.next()).done) e.push(n.value.join(\":\"));\n\
           Map.prototype.set.call(m, 3, \"three\"); Map.prototype.delete.call(m, 1);\n\
           return out.join() + \"|\" + e.join() + \"|\" + [...m.keys()].join();\n\
         });",
    );
    let undefined = agent
        .with_cx(&a, |cx| cx.root(Value::Undefined).unwrap())
        .unwrap();
    let outcome = agent.call(&a, &probe, &undefined, &[&map_b, &f_b]).unwrap();
    assert_eq!(
        support::script::describe(&mut agent, &a, outcome),
        Outcome::Normal("\"one,2,true,true,false,false,true,false|1:one,2:two|2,3\"".to_string())
    );
}

/// The collector (`heap/collect.rs`, `story:map-and-set`): keys and values are traced; a Map
/// that holds itself as key and value survives; a Map reachable only through an iterator a
/// suspended generator holds, and a Set reachable only through its iterator, survive two
/// host collections with a heap churned between them and go on from where they were; a Map
/// whose five iterators were dropped unfinished keeps its entries. The churn makes 200 objects;
/// 50 in the collect mode, still more than the objects the first script made, so they take every
/// slot a missing trace would have freed (200 took 2.6 s alone in that mode in the test profile,
/// opt-level 0, on 2026-11-27).
#[test]
fn the_collector_keeps_what_iterators_and_generators_hold() {
    let junk = if collect_mode() { 50 } else { 200 };
    let (mut agent, realm) = agent_and_realm(|limits| limits.steps_per_entry = 1 << 40);
    steps_of(
        &mut agent,
        &realm,
        "var self = new Map(); self.set(self, self);\n\
         var g = (function* () {\n\
           var it = new Map([[1, { a: 1 }], [2, { a: 2 }], [3, { a: 3 }]]).entries();\n\
           var first = it.next().value;\n\
           yield first[0] + \":\" + first[1].a;\n\
           var rest = []; for (var e of it) rest.push(e[0] + \":\" + e[1].a);\n\
           yield rest.join();\n\
         })();\n\
         var kept = (function () { var s = new Set([\"x\", { y: 1 }, \"z\"]); return s.values(); })();\n\
         var w = (function () { var m = new Map(), i; for (i = 0; i < 20; i++) m.set(\"k\" + i, { v: i }); for (i = 0; i < 5; i++) m.keys().next(); return m; })();\n\
         var first = g.next().value + \",\" + kept.next().value;",
    );
    collected(&mut agent);
    steps_of(
        &mut agent,
        &realm,
        &format!(
            "var junk = [], i; for (i = 0; i < {junk}; i++) junk.push({{ i: i }}); junk = null;"
        ),
    );
    collected(&mut agent);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "[first, g.next().value, typeof kept.next().value, kept.next().value, kept.next().done,\n\
              self.get(self) === self && self.has(self), self.size, w.size, w.get(\"k7\").v,\n\
              [...w.values()].map(function (o) { return o.v; }).join(\"\")].join(\"|\");"
        ),
        Outcome::Normal(
            "\"1:1,x|2:2,3:3|object|z|true|true|1|20|7|012345678910111213141516171819\""
                .to_string()
        )
    );
}

/// The table's growth with no live entries (`story:keyed-entry-table`, compaction): a fresh
/// key added and deleted N times (10,000; 40 in the collect mode), then the same with
/// `clear` every 1,000 turns (20), then on a Map of 1,000 live keys (16) twice: the bytes
/// charged right after each loop and after a collection stay bounded, and the second churn of
/// the big Map leaves the reading where the first left it. The readings are printed.
///
/// The collect mode's counts: 40 keys pass 32, so a table that never compacts holds storage
/// for 64 entries and their index map after the churn, and 16 live keys and 40 churned pass
/// the 32 places of the big Map's storage. With 100, 25 and 32 the test took 4.8 s alone in
/// the test profile (opt-level 0) on 2026-11-27.
#[test]
fn churn_without_live_entries_stays_bounded() {
    let (n, every, live) = if collect_mode() {
        (40, 20, 16)
    } else {
        (10_000, 1_000, 1_000)
    };
    let (mut agent, realm) = agent_and_realm(|limits| limits.steps_per_entry = 1 << 40);
    steps_of(&mut agent, &realm, "var m = new Map(), big = new Map(), i;");
    let start = collected(&mut agent);
    steps_of(
        &mut agent,
        &realm,
        &format!("for (i = 0; i < {n}; i++) {{ m.set(i, i); m.delete(i); }}"),
    );
    let churned = agent.heap_bytes().unwrap();
    let churned_collected = collected(&mut agent);
    steps_of(
        &mut agent,
        &realm,
        &format!(
            "for (i = 0; i < {n}; i++) {{ m.set(i, i); m.delete(i); if (i % {every} === {every} - 1) m.clear(); }}"
        ),
    );
    let cleared = agent.heap_bytes().unwrap();
    let cleared_collected = collected(&mut agent);
    steps_of(
        &mut agent,
        &realm,
        &format!("for (i = 0; i < {live}; i++) big.set(i, i);"),
    );
    let filled = collected(&mut agent);
    let churn_big = format!(
        "for (i = 0; i < {n}; i++) {{ big.set(1e6 + i, i); big.delete(1e6 + i); }} big.size;"
    );
    steps_of(&mut agent, &realm, &churn_big);
    let big_once = collected(&mut agent);
    steps_of(&mut agent, &realm, &churn_big);
    let big_twice = collected(&mut agent);
    println!(
        "heap_bytes(): start {start}; churn {churned} ({churned_collected} collected); with clear \
         {cleared} ({cleared_collected}); big {filled}, after one churn {big_once}, after two {big_twice}"
    );
    assert!(
        churned <= start + 8192,
        "{churned} after the churn, {start} before"
    );
    assert!(churned_collected <= start + 2048, "{churned_collected}");
    assert!(cleared <= start + 8192, "{cleared}");
    assert!(cleared_collected <= start + 2048, "{cleared_collected}");
    assert_eq!(
        big_twice, big_once,
        "the second churn of the big Map moved the reading"
    );
}

/// The memory budget twice in one agent (`docs/design/engine.md`, "Memory budget";
/// `docs/design/host-interface.md`, section 10): a Map filled in a function's scope until the
/// run ends in `Terminated(MemoryBudget)`, a `collect()`, the same script again, which ends the
/// same way, and a `collect()`: both readings after a collection are the reading before the
/// first run, and a third run of a small script answers normally. The fill calls `forEach` at
/// every 64th entry; at every 256th in the collect mode, where each of its visits is a call whose
/// every step marks the heap (a room of 16 KiB above the realm in that mode, 256 KiB in the
/// default one; the test took 3.0 s alone with every 64th in the test profile, opt-level 0, on
/// 2026-11-27, and with a room of 4 or 8 KiB the parse of the first script was refused).
#[test]
fn the_memory_budget_twice_in_one_agent_credits_both_runs() {
    let room: u64 = if collect_mode() { 16 << 10 } else { 256 << 10 };
    let every = if collect_mode() { 256 } else { 64 };
    let warm = "(function () { var m = new Map(); for (var i = 0; i < 8; i++) m.set(i, i); var it = m.keys(); it.next(); m.forEach(function () {}); })();";
    let realm_reading = {
        let (mut probe, realm) = agent_and_realm(|limits| limits.steps_per_entry = 1 << 40);
        steps_of(&mut probe, &realm, warm);
        collected(&mut probe)
    };
    let (mut agent, realm) = agent_and_realm(|limits| {
        limits.steps_per_entry = 1 << 40;
        limits.memory_bytes = realm_reading + room;
    });
    steps_of(&mut agent, &realm, warm);
    let before = collected(&mut agent);
    let mut readings = Vec::new();
    for _ in 0..2 {
        let outcome = eval_text(
            &mut agent,
            &realm,
            &format!(
                "(function () {{ var m = new Map(), it = m.keys(), f = function () {{}}; for (var i = 0; ; i++) {{ m.set(i, i); if (i % {every} === 0) m.forEach(f); }} }})();"
            ),
        );
        assert_eq!(outcome, Outcome::Terminated(Termination::MemoryBudget));
        readings.push(collected(&mut agent));
    }
    println!("heap_bytes(): {before} before, {readings:?} after each run and collect()");
    assert!(
        readings.iter().all(|&r| r <= before),
        "{readings:?}, {before} before"
    );
    assert_eq!(
        eval_text(&mut agent, &realm, "new Map([[1, 2]]).get(1);"),
        Outcome::Normal("2".to_string())
    );
}

/// The interrupt between two `next` calls of one iterator ends the run in
/// `Terminated(Interrupted)`, and the iterator goes on from its entry in the next run; an
/// interrupt in the adder of the constructor ends it with no call of `return` (a termination
/// closes nothing), and the entries added before stay.
#[test]
fn an_interrupt_between_next_calls_and_in_the_adder() {
    let mut agent = agent_with(unbounded_steps()).unwrap();
    let realm = realm(&mut agent);
    define_function(&mut agent, &realm, "interruptNow", 0, interrupt_now, false);
    HANDLE.with(|slot| slot.set(Some(agent.interrupt_handle())));
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var m = new Map([[1, 1], [2, 2], [3, 3]]), it = m.keys(), a = it.next().value; interruptNow(); a = 9;"
        ),
        Outcome::Terminated(Termination::Interrupted)
    );
    HANDLE.with(|slot| slot.set(Some(agent.interrupt_handle())));
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            &[
                ITERABLES,
                "var target, set = Map.prototype.set;\n\
                 Map.prototype.set = function (k, v) { target = this; set.call(this, k, v); if (k === 2) interruptNow(); return this; };\n\
                 new Map(closing([[1, 1], [2, 2], [3, 3]]));"
            ]
            .concat()
        ),
        Outcome::Terminated(Termination::Interrupted)
    );
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "Map.prototype.set = set; [a, it.next().value, it.next().value, it.next().done, returns, target.size].join();"
        ),
        Outcome::Normal("\"1,2,3,true,0,2\"".to_string())
    );
}

/// The step budget (`docs/design/engine.md`, "The step budget"): a `forEach` whose callback
/// adds one entry per visit, a `for`-`of` whose body does the same, and `new Set` of an
/// iterable without end each end in `Terminated(StepBudget)` with `last_entry_steps()` at most
/// the budget (100,000; 500 in the collect mode, where 2,000 took 2.9 s alone in the test
/// profile, opt-level 0, on 2026-11-27).
#[test]
fn loops_that_add_an_entry_per_turn_end_at_the_step_budget() {
    let budget = if collect_mode() { 500 } else { 100_000 };
    for source in [
        "var m = new Map([[0, 0]]); m.forEach(function (v, k) { m.set(k + 1, 0); });",
        "var s = new Set([0]); for (var k of s) s.add(k + 1);",
        "var m = new Map([[0, 0]]); var it = m.keys(); while (true) { var n = it.next(); m.set(n.value + 1, 0); }",
        "new Set({ [Symbol.iterator]() { var i = 0; return { next() { return { value: i++, done: false }; } }; } });",
    ] {
        let (mut agent, realm) = agent_and_realm(|limits| limits.steps_per_entry = budget);
        let outcome = eval_text(&mut agent, &realm, source);
        let steps = agent.last_entry_steps().unwrap();
        assert_eq!(
            outcome,
            Outcome::Terminated(Termination::StepBudget),
            "{source}"
        );
        assert!(steps <= budget, "{steps} steps of {budget}: {source}");
    }
}

/// The steps per entry (`story:map-and-set`, Bounds: "each entry visited by `forEach` and by
/// `next` and each entry the constructor adds is one step, charged before the work"):
/// N `next` calls on an iterator over N entries spend exactly N steps more than N calls on a
/// done iterator; `forEach` and the `Set` constructor over N entries spend a fixed number of
/// steps per entry more than over none, read here and pinned. The readings are printed.
#[test]
fn each_entry_of_next_for_each_and_the_constructor_is_a_step() {
    let n: u64 = if collect_mode() { 20 } else { 1_000 };
    let (mut agent, realm) = agent_and_realm(|limits| limits.steps_per_entry = 1 << 40);
    steps_of(
        &mut agent,
        &realm,
        &format!(
            "var m = new Map(), e = new Map(), arr = [], none = [], i;\n\
             for (i = 0; i < {n}; i++) {{ m.set(i, i); arr.push(i); }}\n\
             var live = m.keys(), done = e.keys(); done.next();"
        ),
    );
    let next_live = steps_of(
        &mut agent,
        &realm,
        &format!("for (i = 0; i < {n}; i++) live.next();"),
    );
    let next_done = steps_of(
        &mut agent,
        &realm,
        &format!("for (i = 0; i < {n}; i++) done.next();"),
    );
    let for_each_full = steps_of(&mut agent, &realm, "m.forEach(Math.abs);");
    let for_each_empty = steps_of(&mut agent, &realm, "e.forEach(Math.abs);");
    let set_full = steps_of(&mut agent, &realm, "new Set(arr);");
    let set_empty = steps_of(&mut agent, &realm, "new Set(none);");
    println!(
        "N {n}: next {next_live} against {next_done}; forEach {for_each_full} against \
         {for_each_empty}; new Set {set_full} against {set_empty}"
    );
    // Pinned from the reading at `9cae087a`: forEach with a builtin callback, the entry's step
    // and the call's; new Set of an Array, the loop's step, the call of the Array Iterator's
    // `next`, the call of `add` and the table's step of `insert`.
    assert_eq!(
        [
            next_live - next_done,
            for_each_full - for_each_empty,
            set_full - set_empty
        ],
        [n, 2 * n, 4 * n],
        "steps per entry of next, forEach and new Set"
    );
}
