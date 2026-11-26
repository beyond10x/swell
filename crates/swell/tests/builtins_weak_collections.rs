//! `WeakMap` and `WeakSet` (`#sec-weakmap-objects`, `#sec-weakset-objects`) with
//! `getOrInsert` and `getOrInsertComputed`, Symbols as keys (CanBeHeldWeakly,
//! `#sec-canbeheldweakly`) and the collector's clearing of their entries
//! (`story:weak-map-and-weak-set`): the methods by their clauses, the constructors with their
//! adders, the brand checks against `Map` and `Set`, an entry freed with its key, and the step,
//! memory and depth bounds. Run also with the feature `collect-every-safe-point`, where the
//! loops of the budget tests take smaller counts, as those of `builtins_map_and_set` do.
#![cfg(test)]

use crate::support;

use support::host::{TestHost, agent_with, limits, realm, stack_with_room};
use support::script::{Outcome, eval, eval_text, run_both};
use support::stack::{BUILTINS_REENTRY, on_bounded_stack, reach_through, stack_limits, test_path};
use swell::{Agent, Limits, Outcome as HostOutcome, Realm, Termination, Value};

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

/// Whether the feature `collect-every-safe-point` is on: the loops of the budget tests take
/// their smaller counts.
fn collect_mode() -> bool {
    cfg!(feature = "collect-every-safe-point")
}

/// `caught(f)` answers the `name` of the error `f` threw, the thrown value if it is no error,
/// or "none".
const CAUGHT: &str = "function caught(f) { try { f(); return \"none\"; } catch (e) { return e instanceof Error ? e.name : e; } }\n";

/// `#sec-weakmap.prototype.set`, `get`, `has`, `delete`, `getOrInsert` and the WeakSet's
/// `add`, `has` and `delete`: an overwrite keeps one entry, `getOrInsert` answers the entry it
/// finds and appends only when there is none, `delete` answers whether it removed one; a key
/// that cannot be held weakly is a `TypeError` in `set`, `add` and `getOrInsert*`, and
/// `false` or `undefined` in `delete`, `has` and `get`. The constructors
/// (`#sec-weakmap-iterable`, `#sec-weakset-iterable`) need `new`, read their adder from the
/// new object once and call it per entry, and take their prototype from NewTarget; the
/// prototypes have their methods in the order of the clauses with their lengths, and
/// `@@toStringTag` "WeakMap" and "WeakSet" (`#sec-weakmap.prototype-%symbol.tostringtag%`).
#[test]
fn weak_map_and_weak_set_methods_follow_their_clauses() {
    answers(
        &[
            CAUGHT,
            "var r = [], g;\n\
             var wm = new WeakMap(), a = {}, b = function () {}, c = [];\n\
             g = [wm.set(a, 1) === wm, wm.get(a), wm.has(a), wm.has(b), wm.get(b)];\n\
             wm.set(a, 2); g.push(wm.get(a)); r.push(g.join());\n\
             r.push([wm.getOrInsert(b, \"b\"), wm.getOrInsert(b, \"x\"), wm.get(b)].join());\n\
             r.push([wm.delete(a), wm.delete(a), wm.has(a), wm.get(a)].join());\n\
             r.push([wm.get(1), wm.has(\"s\"), wm.delete(null), wm.get(undefined)].join());\n\
             r.push([caught(function () { wm.set(1, 1); }), caught(function () { wm.getOrInsert(\"s\", 1); }),\n\
               caught(function () { wm.getOrInsertComputed(true, function () {}); })].join());\n\
             var ws = new WeakSet([a, c]);\n\
             r.push([ws.has(a), ws.has(c), ws.has(b), ws.add(b) === ws, ws.add(b) === ws, ws.has(b),\n\
               ws.delete(b), ws.delete(b), ws.has(b), ws.has(1), ws.delete(1), caught(function () { ws.add(1); })].join());\n\
             var m2 = new WeakMap([[a, \"a\"], [c, \"c\"]]); r.push([m2.get(a), m2.get(c)].join());\n\
             r.push([caught(function () { new WeakMap([[1, 2]]); }), caught(function () { new WeakSet([1]); }),\n\
               caught(function () { WeakMap(); }), caught(function () { WeakSet(); }),\n\
               caught(function () { new WeakMap(1); }), new WeakMap(null) instanceof WeakMap, new WeakSet(undefined) instanceof WeakSet].join());\n\
             r.push([Object.prototype.toString.call(wm), Object.prototype.toString.call(ws),\n\
               WeakMap.prototype[Symbol.toStringTag], WeakSet.prototype[Symbol.toStringTag]].join());\n\
             r.push([WeakMap.length, WeakSet.length, WeakMap.name, WeakSet.name,\n\
               WeakMap.prototype.constructor === WeakMap, WeakSet.prototype.constructor === WeakSet].join());\n\
             r.push([\"delete\", \"get\", \"getOrInsert\", \"getOrInsertComputed\", \"has\", \"set\"].map(function (n) { return n + WeakMap.prototype[n].length; }).join(\" \"));\n\
             r.push([\"add\", \"delete\", \"has\"].map(function (n) { return n + WeakSet.prototype[n].length; }).join(\" \"));\n\
             r.push(Object.getOwnPropertyNames(WeakMap.prototype).join(\" \"), Object.getOwnPropertyNames(WeakSet.prototype).join(\" \"));\n\
             var d = Object.getOwnPropertyDescriptor(WeakMap, \"prototype\");\n\
             var t = Object.getOwnPropertyDescriptor(WeakSet.prototype, Symbol.toStringTag);\n\
             r.push([d.writable, d.enumerable, d.configurable, t.writable, t.enumerable, t.configurable,\n\
               Object.getPrototypeOf(WeakMap.prototype) === Object.prototype, Object.getPrototypeOf(wm) === WeakMap.prototype].join());\n\
             var calls = 0, set = WeakMap.prototype.set;\n\
             WeakMap.prototype.set = function (k, v) { calls++; return set.call(this, k, v); };\n\
             new WeakMap([[a, 1], [b, 2]]); WeakMap.prototype.set = set;\n\
             var adds = 0, add = WeakSet.prototype.add;\n\
             WeakSet.prototype.add = function (v) { adds++; return add.call(this, v); };\n\
             new WeakSet([a, b, c]); WeakSet.prototype.add = add;\n\
             function F() {} F.prototype = Object.create(WeakSet.prototype);\n\
             var sub = Reflect.construct(WeakSet, [[a]], F);\n\
             r.push([calls, adds, Object.getPrototypeOf(sub) === F.prototype, sub.has(a)].join());\n\
             r.join(\"|\");",
        ]
        .concat(),
        "true,1,true,false,,2|b,b,b|true,false,false,|,false,false,|TypeError,TypeError,TypeError|\
         true,true,false,true,true,true,true,false,false,false,false,TypeError|a,c|\
         TypeError,TypeError,TypeError,TypeError,TypeError,true,true|\
         [object WeakMap],[object WeakSet],WeakMap,WeakSet|0,0,WeakMap,WeakSet,true,true|\
         delete1 get1 getOrInsert2 getOrInsertComputed2 has1 set2|add1 delete1 has1|\
         constructor delete get getOrInsert getOrInsertComputed has set|add constructor delete has|\
         false,false,false,false,false,true,true,true|2,3,true,true",
    );
}

/// CanBeHeldWeakly (`#sec-canbeheldweakly`): a Symbol that `Symbol.for` registered
/// (KeyForSymbol is not undefined) is no key of a WeakMap or a WeakSet, a `TypeError` in
/// `set`, `add`, `getOrInsert` and `getOrInsertComputed` and a miss in `has`, `get` and
/// `delete`; a Symbol of `Symbol()` and a well-known Symbol are keys.
#[test]
fn a_registered_symbol_cannot_be_a_key_and_a_well_known_one_can() {
    answers(
        &[
            CAUGHT,
            "var r = [];\n\
             var reg = Symbol.for(\"reg\"), own = Symbol(\"own\"), known = Symbol.iterator;\n\
             var wm = new WeakMap(), ws = new WeakSet();\n\
             r.push([caught(function () { wm.set(reg, 1); }), caught(function () { ws.add(reg); }),\n\
               caught(function () { wm.getOrInsert(reg, 1); }), caught(function () { wm.getOrInsertComputed(reg, function () { return 1; }); })].join());\n\
             r.push([wm.has(reg), wm.get(reg), wm.delete(reg), ws.has(reg), ws.delete(reg)].join());\n\
             wm.set(own, \"own\"); wm.set(known, \"known\"); ws.add(own); ws.add(known);\n\
             r.push([wm.get(own), wm.get(known), ws.has(own), ws.has(known), wm.getOrInsert(own, \"x\"),\n\
               wm.delete(known), wm.has(known), ws.delete(own), ws.has(own), ws.has(known)].join());\n\
             r.push([new WeakSet([own, known, {}]).has(known), new WeakMap([[known, 1]]).get(known)].join());\n\
             r.join(\"|\");",
        ]
        .concat(),
        "TypeError,TypeError,TypeError,TypeError|false,,false,false,false|\
         own,known,true,true,own,true,false,true,false,true|true,1",
    );
}

/// `#sec-weakmap.prototype.getorinsertcomputed`: CanBeHeldWeakly comes first, then IsCallable
/// (step 4), then the lookup; the callback is called only on a miss, once, with the key as its
/// one argument and `undefined` as its this value; after it the entry is looked up again, so a
/// `set` of the same key in the callback is overwritten by its answer and no second entry is
/// made; a throw of the callback adds nothing.
#[test]
fn get_or_insert_computed_calls_its_callback_with_the_key_and_reads_the_entry_again() {
    answers(
        &[
            CAUGHT,
            "var r = [], wm = new WeakMap(), k = {}, seen, calls = 0;\n\
             var first = wm.getOrInsertComputed(k, function (key) { \"use strict\"; calls++; seen = [key === k, arguments.length, this === undefined]; return \"computed\"; });\n\
             r.push([first, calls, seen.join(\" \"), wm.get(k)].join());\n\
             r.push([wm.getOrInsertComputed(k, function () { calls++; return \"again\"; }), calls].join());\n\
             var k2 = {};\n\
             r.push([wm.getOrInsertComputed(k2, function () { wm.set(k2, \"inner\"); return \"outer\"; }), wm.get(k2)].join());\n\
             var k3 = {};\n\
             r.push([caught(function () { wm.getOrInsertComputed(k3, function () { throw new RangeError(\"x\"); }); }), wm.has(k3)].join());\n\
             var counted = 0, count = function () { counted++; return 1; };\n\
             r.push([caught(function () { wm.getOrInsertComputed(1, count); }), caught(function () { wm.getOrInsertComputed(1, 1); }),\n\
               caught(function () { wm.getOrInsertComputed(k, 1); }), counted].join());\n\
             var k4 = {};\n\
             r.push([wm.getOrInsertComputed(k4, function () { wm.delete(k4); return \"after delete\"; }), wm.get(k4)].join());\n\
             r.join(\"|\");",
        ]
        .concat(),
        "computed,1,true 1 true,computed|computed,1|outer,outer|RangeError,false|\
         TypeError,TypeError,TypeError,0|after delete,after delete",
    );
}

/// RequireInternalSlot (`#sec-requireinternalslot`): a method of `Map.prototype` or
/// `Set.prototype` called on a WeakMap or a WeakSet, a method of `WeakMap.prototype` or
/// `WeakSet.prototype` called on a Map, a Set, the other weak kind, a prototype or a plain
/// object, throws a `TypeError`.
#[test]
fn a_map_method_on_a_weak_map_throws_a_type_error() {
    for source in [
        "Map.prototype.get.call(new WeakMap(), {});",
        "Map.prototype.set.call(new WeakMap(), {}, 1);",
        "Map.prototype.has.call(new WeakMap(), {});",
        "Map.prototype.delete.call(new WeakMap(), {});",
        "Object.getOwnPropertyDescriptor(Map.prototype, \"size\").get.call(new WeakMap());",
        "Map.prototype.forEach.call(new WeakMap(), function () {});",
        "Map.prototype.clear.call(new WeakMap());",
        "Set.prototype.add.call(new WeakSet(), {});",
        "Set.prototype.has.call(new WeakSet(), {});",
        "Set.prototype.values.call(new WeakSet());",
        "WeakMap.prototype.get.call(new Map(), {});",
        "WeakMap.prototype.set.call(new Map(), {}, 1);",
        "WeakMap.prototype.has.call(new WeakSet(), {});",
        "WeakMap.prototype.delete.call(WeakMap.prototype, {});",
        "WeakMap.prototype.getOrInsert.call({}, {}, 1);",
        "WeakMap.prototype.getOrInsertComputed.call(new Map(), {}, function () {});",
        "WeakSet.prototype.add.call(new Set(), {});",
        "WeakSet.prototype.has.call(new WeakMap(), {});",
        "WeakSet.prototype.delete.call(WeakSet.prototype, {});",
    ] {
        throws(source, "TypeError");
    }
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

/// The elements of the Array the test of a freed key makes.
const ARRAY_ELEMENTS: u64 = 1_000;

/// The weak entry (`#sec-weakmap-objects`, the note on keys that become inaccessible): a script
/// keeps a WeakMap and sets, under a key only the host's root holds, an Array of 1,000
/// elements, which nothing else holds. With the root held, `collect()` keeps both; once the host
/// drops the root, one `collect()` frees the key, its entry and the Array: `heap_bytes()` falls
/// by at least the Array's dense elements, 1,000 × `size_of::<Value>()` (16,000 bytes). Nothing
/// is made between the two readings. The readings are printed.
#[test]
fn a_key_freed_by_collect_takes_its_entry_and_its_value() {
    let (mut agent, realm) = agent_and_realm(|limits| limits.steps_per_entry = 1 << 40);
    let HostOutcome::Normal(key) = eval(
        &mut agent,
        &realm,
        &format!(
            "var wm = new WeakMap();\n\
             (function () {{ var k = {{}}; wm.set(k, new Array({ARRAY_ELEMENTS}).fill(7)); return k; }})();"
        ),
    ) else {
        panic!("the script did not complete normally");
    };
    let held = collected(&mut agent);
    drop(key);
    let freed = collected(&mut agent);
    let array = ARRAY_ELEMENTS * u64::try_from(size_of::<Value>()).unwrap();
    println!(
        "heap_bytes(): {held} with the key held, {freed} after the root was dropped and collect(); \
         the Array's elements {array} bytes"
    );
    assert!(
        held >= freed + array,
        "{held} with the key held, {freed} after; the Array alone is {array} bytes"
    );
    assert_eq!(
        eval_text(&mut agent, &realm, "wm instanceof WeakMap;"),
        Outcome::Normal("true".to_string())
    );
}

/// The objects the first script of the memory test makes at once in the default mode. The
/// collect mode makes none: its room is small, the realm's objects arena has free slots for what
/// its loop makes, and each object made there costs collections of the whole heap.
const WARM_OBJECTS: usize = 4_096;

/// The memory budget (`docs/design/engine.md`, "Memory budget"): WeakMaps filled without end in
/// a function's scope, a new map per turn or one map with a new key per turn (the latter in the
/// default mode only), the keys and the maps kept by an Array (a key nothing keeps is cleared by
/// a collection, and with a collection at every safe point such a loop runs in bounded memory for
/// ever), with a budget of the
/// realm's reading plus 256 KiB (16 KiB in the collect mode: the first script needs more than
/// 8 KiB above the reading while it runs), end in
/// `Terminated(MemoryBudget)`; once the host dropped the script, one `collect()` brings
/// `heap_bytes()` back within the reading before the script. Before the reading a first script
/// of the same shape grows the arenas a script needs; in the default mode it also makes
/// [`WARM_OBJECTS`] objects at once, so the objects arena has room for every object the loop
/// makes within the budget: the slots of an arena stay charged with its capacity once it grew
/// (`heap/arena.rs`), and a growth during the loop would stay in the reading after it. The
/// readings are printed.
#[test]
fn weak_maps_filled_in_a_loop_end_at_the_memory_budget() {
    let room: u64 = if collect_mode() { 16 << 10 } else { 256 << 10 };
    let shapes = [
        (
            "(function () { var all = []; for (var i = 0; i < 8; i++) { var m = new WeakMap(), k = {}; m.set(k, i); all.push(m, k); } })();",
            "(function () { var all = []; for (var i = 0; ; i++) { var m = new WeakMap(), k = {}; m.set(k, i); all.push(m, k); } })();",
        ),
        (
            "(function () { var m = new WeakMap(), keys = []; for (var i = 0; i < 8; i++) { var k = {}; keys.push(k); m.set(k, i); } })();",
            "(function () { var m = new WeakMap(), keys = []; for (var i = 0; ; i++) { var k = {}; keys.push(k); m.set(k, i); } })();",
        ),
    ];
    // The collect mode runs the first shape only: with the second the test took 2.4 CPU seconds
    // alone there, past its bound of 2 (the one map's loop, a collection of the whole heap at
    // each of its safe points, 1.0 s of it; measured on 2026-11-26).
    let shapes = if collect_mode() {
        &shapes[..1]
    } else {
        &shapes[..]
    };
    for &(warm, endless) in shapes {
        let warm = if collect_mode() {
            warm.to_string()
        } else {
            format!(
                "(function () {{ var a = []; for (var i = 0; i < {WARM_OBJECTS}; i++) a.push({{}}); }})(); {warm}"
            )
        };
        let warmed = |agent: &mut Agent<TestHost>, realm: &Realm| {
            assert_eq!(
                eval_text(agent, realm, &warm),
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
        let script = support::script::parse(&mut agent, &realm, endless);
        let outcome = agent.evaluate_script(&script).unwrap();
        let at_the_end = agent.heap_bytes().unwrap();
        assert!(
            matches!(outcome, HostOutcome::Terminated(Termination::MemoryBudget)),
            "{endless}: {outcome:?}"
        );
        drop(script);
        let after = collected(&mut agent);
        println!(
            "{endless}: heap_bytes(): {before} before the script, budget {}, {at_the_end} at its \
             end, {after} after collect()",
            before + room
        );
        assert!(at_the_end > before + room / 2, "{at_the_end}");
        assert!(after <= before, "{after} after collect(), {before} before");
    }
}

/// The step budget (`docs/design/engine.md`, "The step budget"): a loop that fills a WeakMap
/// without end, with `steps_per_entry` N (100,000; 2,000 in the collect mode), ends in
/// `Terminated(StepBudget)`, and `last_entry_steps()` is at most N.
#[test]
fn a_weak_map_filled_in_a_loop_ends_at_the_step_budget() {
    let budget = if collect_mode() { 2_000 } else { 100_000 };
    let (mut agent, realm) = agent_and_realm(|limits| limits.steps_per_entry = budget);
    let outcome = eval_text(
        &mut agent,
        &realm,
        "var m = new WeakMap(), keys = []; for (var i = 0; ; i++) { var k = {}; keys.push(k); m.set(k, i); }",
    );
    let steps = agent.last_entry_steps().unwrap();
    println!("{steps} steps of {budget}");
    assert_eq!(outcome, Outcome::Terminated(Termination::StepBudget));
    assert!(steps <= budget, "{steps} steps of {budget}");
}

/// `Limits::min_stack_bytes` through `WeakMap.prototype.getOrInsertComputed` (decision 3,
/// "Depth bounds"): the callback is a call from Rust, a re-entry, so `f` re-enters itself
/// through it until the re-entry limit, on seven eighths of `min_stack_bytes()`, in a child
/// process. In the collect mode the depth is [`BUILTINS_REENTRY`].
#[test]
fn get_or_insert_computed_reaches_the_reentry_limit_on_the_promised_stack() {
    fn body() {
        reach_through("new WeakMap().getOrInsertComputed({}, f)", BUILTINS_REENTRY);
    }
    let stack = stack_with_room(stack_limits(BUILTINS_REENTRY));
    on_bounded_stack(
        test_path!("get_or_insert_computed_reaches_the_reentry_limit_on_the_promised_stack"),
        stack,
        body,
    )
    .unwrap();
}
