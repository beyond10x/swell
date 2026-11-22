//! The enumeration and copy methods of `Object` (ECMA-262 20.1.2,
//! `#sec-properties-of-the-object-constructor`): `Object.assign` (`#sec-object.assign`),
//! `Object.entries` (`#sec-object.entries`), `Object.getOwnPropertyDescriptors`
//! (`#sec-object.getownpropertydescriptors`), `Object.hasOwn` (`#sec-object.hasown`),
//! `Object.keys` (`#sec-object.keys`) and `Object.values` (`#sec-object.values`), with
//! EnumerableOwnProperties (`#sec-enumerableownproperties`), the step and memory budgets, the
//! re-entry limit through the getters and setters `Object.assign` calls, and the invariant that
//! no builtin of slice 3 hands a Symbol value to a script (the twelve well-known symbols on
//! `Symbol` and `Object.getOwnPropertySymbols` are slice 6's,
//! `story:symbol-constructor-and-well-known-symbols`). `story:object-enumeration-and-copy-methods`.
//!
//! No script calls a builtin another story of the wave delivers: a result array is compared
//! element by element or through `join`, a target that refuses a Set has a property that is not
//! writable, an object with a chosen prototype is made by a constructor function. A script has
//! no Symbol value; a Symbol-keyed property is reached under its key by `@@species`, which
//! ArraySpeciesCreate (`#sec-arrayspeciescreate`) of `Array.prototype.map` reads from the
//! `constructor` of an Array. Run also with the feature `collect-every-safe-point`.
#![cfg(test)]

use crate::support;

use support::host::{
    TestHost, agent, agent_with, get, get_object, global, key, limits, object, realm,
    stack_with_room, text,
};
use support::script::{Outcome, eval_text, run_both};
use support::stack::{FULL_REENTRY, on_bounded_stack, reach_through, stack_limits, test_path};
use swell::{Agent, AgentState, Cx, ObjectRef, Realm, Termination, Value};

/// Both modes of `source` answer `expected`, a normal completion as `show` writes it.
fn both_answer(source: &str, expected: &str) {
    let (sloppy, strict) = run_both(source);
    let expected = Outcome::Normal(expected.to_string());
    assert_eq!(sloppy, expected, "sloppy: {source}");
    assert_eq!(strict, expected, "strict: {source}");
}

/// Both modes of `source` throw `name`, the name of an error.
fn both_throw(source: &str, name: &str) {
    let (sloppy, strict) = run_both(source);
    let expected = Outcome::Throw(name.to_string());
    assert_eq!(sloppy, expected, "sloppy: {source}");
    assert_eq!(strict, expected, "strict: {source}");
}

/// The six builtins, in the order of their clauses in 20.1.2, with their "length".
const BUILTINS: [(&str, u32); 6] = [
    ("assign", 2),
    ("entries", 1),
    ("getOwnPropertyDescriptors", 1),
    ("hasOwn", 2),
    ("keys", 1),
    ("values", 1),
];

/// Each builtin is a property of `Object` { [[Writable]]: true, [[Enumerable]]: false,
/// [[Configurable]]: true } (clause 18), a function whose "length" is the clause's and whose
/// "name" is its key, and the six keys come in the order of their clauses among the own keys
/// of `Object`.
#[test]
fn each_builtin_is_a_method_of_object_in_the_order_of_its_clause() {
    for (name, length) in BUILTINS {
        both_answer(
            &format!(
                "var f = Object.{name}; var d = Object.getOwnPropertyDescriptor(Object, \"{name}\");\n\
                 [typeof f, f.length, f.name, d.writable, d.enumerable, d.configurable].join();"
            ),
            &format!("\"function,{length},{name},true,false,true\""),
        );
    }
    let names: Vec<&str> = BUILTINS.iter().map(|(name, _)| *name).collect();
    both_answer(
        "var mine = { assign: 1, entries: 1, getOwnPropertyDescriptors: 1,\n\
           hasOwn: 1, keys: 1, values: 1 };\n\
         var all = Object.getOwnPropertyNames(Object), found = [];\n\
         for (var i = 0; i < all.length; i++) {\n\
           if (Object.prototype.hasOwnProperty.call(mine, all[i])) found.push(all[i]);\n\
         }\n\
         found.join();",
        &format!("\"{}\"", names.join(",")),
    );
}

/// `#sec-object.keys`: the enumerable String keys of ToObject(O) in the order of
/// [[OwnPropertyKeys]]: of a String object its indices, of an Array its elements (a hole is no
/// key) and then its other String keys, of an arguments object its indices; "length" is not
/// enumerable on any of them. A primitive without own properties answers an empty array, and
/// undefined and null throw a TypeError.
#[test]
fn keys_of_a_string_an_array_and_arguments() {
    both_answer("Object.keys(\"abc\").join();", "\"0,1,2\"");
    both_answer(
        "var a = [1, , 3]; a.x = 4; Object.keys(a).join();",
        "\"0,2,x\"",
    );
    both_answer(
        "(function () { return Object.keys(arguments).join(); })(5, 6);",
        "\"0,1\"",
    );
    both_answer(
        "var o = { b: 1, a: 2, 1: 3, 0: 4 };\n\
         Object.defineProperty(o, \"c\", { value: 5, enumerable: false });\n\
         Object.keys(o).join();",
        "\"0,1,b,a\"",
    );
    both_answer("Object.keys(1).length + Object.keys(true).length;", "0");
    both_throw("Object.keys(undefined);", "TypeError");
    both_throw("Object.keys(null);", "TypeError");
    both_throw("Object.keys();", "TypeError");
}

/// `#sec-object.values`: the value of each enumerable String key, read by Get with the object
/// as receiver, so a getter runs with it as its this value.
#[test]
fn values_read_each_value_by_get() {
    both_answer(
        "var o = { a: 1, get b() { return this.a + 1; }, c: 3 };\n\
         Object.defineProperty(o, \"d\", { value: 4, enumerable: false });\n\
         Object.values(o).join();",
        "\"1,2,3\"",
    );
    both_answer("Object.values(\"ab\").join();", "\"a,b\"");
    both_throw("Object.values(null);", "TypeError");
}

/// `#sec-enumerableownproperties`, step 3.a.i: each key is read again by [[GetOwnProperty]]
/// when its turn comes, so a property that a getter of an earlier one deleted is no entry; one
/// it added after [[OwnPropertyKeys]] is none either. Each entry is an Array of the key and
/// the value.
#[test]
fn entries_skip_a_property_a_getter_deleted() {
    both_answer(
        "var o = { get a() { delete this.b; this.z = 9; return 1; }, b: 2, c: 3 };\n\
         var e = Object.entries(o);\n\
         e.length + \":\" + e[0].join(\"=\") + \";\" + e[1].join(\"=\") + \":\" + Array.isArray(e[0]);",
        "\"2:a=1;c=3:true\"",
    );
    both_answer(
        "var e = Object.entries([7, 8]); e[1][0] + typeof e[1][0] + e[1][1];",
        "\"1string8\"",
    );
    both_throw("Object.entries(undefined);", "TypeError");
}

/// What `[1].map` answers for an Array whose `constructor` is `c`: "none" when it completes, or
/// the name of what it threw. ArraySpeciesCreate (`#sec-arrayspeciescreate`) reads
/// `c[@@species]` when `c` is an object, and throws a TypeError when that is neither undefined,
/// null nor a constructor: so a script sees whether an object has an `@@species` property
/// without holding the Symbol.
const SPECIES_OF: &str = "function speciesOf(c) {\n\
       var a = [1]; a.constructor = c;\n\
       try { a.map(function (x) { return x; }); return \"none\"; } catch (e) { return e.name; }\n\
     }\n";

/// `#sec-object.assign`: the String keys and then the Symbol keys of each source, in the order
/// of [[OwnPropertyKeys]], read by Get and written by Set with `true`. A source with an
/// enumerable `@@species` (a copy of the descriptors of `Array`, its String keys deleted) and a
/// String key made after it: on a target whose prototype is `Array`, whose `@@species` has no
/// setter, the String key is set first and the Set of `@@species` throws; on `{}` the copy
/// holds `@@species`. A setter of the target runs; undefined and null sources are skipped, a
/// String source gives its indices, and the target is answered, ToObject of a primitive
/// included. With only a target it is answered unread.
#[test]
fn assign_copies_symbol_keys_and_calls_setters() {
    both_answer(
        &format!(
            "{SPECIES_OF}function F() {{}} F.prototype = Array;\n\
             var source = Object.getOwnPropertyDescriptors(Array);\n\
             var names = Object.getOwnPropertyNames(source);\n\
             for (var i = 0; i < names.length; i++) delete source[names[i]];\n\
             source.a = 1;\n\
             var target = new F(), out;\n\
             try {{ Object.assign(target, source); out = \"none\"; }} catch (e) {{ out = e.name; }}\n\
             var copy = Object.assign({{}}, source);\n\
             [out, Object.hasOwn(target, \"a\"), Object.keys(copy).join(), speciesOf({{}}),\n\
              speciesOf(copy)].join();"
        ),
        "\"TypeError,true,a,none,TypeError\"",
    );
    both_answer(
        "var log = [];\n\
         var target = { set a(v) { log.push(\"a\" + v); } };\n\
         var result = Object.assign(target, { a: 2, b: 3 });\n\
         [result === target, log.join(\"/\"), target.b].join();",
        "\"true,a2,3\"",
    );
    both_answer(
        "var t = Object.assign({ a: 0 }, null, { a: 1 }, undefined, \"xy\");\n\
         [t.a, t[0], t[1]].join();",
        "\"1,x,y\"",
    );
    both_answer(
        "var n = Object.assign(1); typeof n + (n + 1);",
        "\"object2\"",
    );
    both_answer(
        "var source = { get p() { return this === source; } };\n\
         var o = Object.assign({}, source);\n\
         Object.getOwnPropertyDescriptor(o, \"p\").value;",
        "true",
    );
    both_throw("Object.assign(null, {});", "TypeError");
}

/// `#sec-object.assign`, the `?` of Get and of Set: the first throw ends the copy, a property
/// copied before it stays on the target, and no later key or source is read.
#[test]
fn assign_stops_at_the_first_throw_and_keeps_what_it_copied() {
    both_answer(
        "var t = {};\n\
         Object.defineProperty(t, \"b\", { value: 0, writable: false, enumerable: true, configurable: true });\n\
         var read = false, out;\n\
         try { Object.assign(t, { a: 1, b: 2, c: 3 }, { get d() { read = true; } }); out = \"none\"; }\n\
         catch (e) { out = e.name; }\n\
         [out, t.a, t.b, t.c, read].join();",
        "\"TypeError,1,0,,false\"",
    );
    both_answer(
        "var t = {}, out;\n\
         try { Object.assign(t, { a: 1, get b() { throw 7; }, c: 3 }); } catch (e) { out = e; }\n\
         [out, t.a, \"c\" in t].join();",
        "\"7,1,false\"",
    );
}

/// `#sec-object.hasown`: ToObject(O) comes before ToPropertyKey(P), so a null object throws
/// its TypeError before the key's toString runs; an inherited property is not own, an index of
/// a String is.
#[test]
fn has_own_converts_the_object_before_the_key() {
    both_answer(
        "var log = [];\n\
         try { Object.hasOwn(null, { toString: function () { log.push(\"key\"); return \"a\"; } }); }\n\
         catch (e) { log.push(e.name); }\n\
         log.join();",
        "\"TypeError\"",
    );
    both_answer(
        "function C() {} C.prototype.p = 1;\n\
         var o = { a: 1 };\n\
         var k = { toString: function () { return \"a\"; } };\n\
         [Object.hasOwn(o, \"a\"), Object.hasOwn(o, k), Object.hasOwn(o, \"b\"),\n\
          Object.hasOwn(new C(), \"p\"), Object.hasOwn(\"ab\", 1), Object.hasOwn(\"ab\", 2)].join();",
        "\"true,true,false,false,true,false\"",
    );
}

/// `#sec-object.getownpropertydescriptors`: an ordinary object of `%Object.prototype%` with,
/// for every own key, the String and the Symbol ones, FromPropertyDescriptor of its property,
/// created as an enumerable data property in the order of [[OwnPropertyKeys]].
#[test]
fn get_own_property_descriptors_of_every_own_key() {
    both_answer(
        "var o = { a: 1 };\n\
         Object.defineProperty(o, \"b\", { get: function () { return 2; }, configurable: true });\n\
         var d = Object.getOwnPropertyDescriptors(o);\n\
         [[d.a.value, d.a.writable, d.a.enumerable, d.a.configurable].join(\"/\"),\n\
          [typeof d.b.get, typeof d.b.set, d.b.enumerable, d.b.configurable].join(\"/\"),\n\
          Object.getPrototypeOf(d) === Object.prototype, Object.keys(d).join(\"/\")].join();",
        "\"1/true/true/true,function/undefined/false/true,true,a/b\"",
    );
    // The Symbol key of `Array`, `@@species`, is described too: its descriptor object, which
    // is no constructor, makes ArraySpeciesCreate throw.
    both_answer(
        &format!(
            "{SPECIES_OF}var d = Object.getOwnPropertyDescriptors(Array);\n\
             [Object.keys(d).join() === Object.getOwnPropertyNames(Array).join(),\n\
              speciesOf({{}}), speciesOf(d)].join();"
        ),
        "\"true,none,TypeError\"",
    );
    both_answer(
        "var d = Object.getOwnPropertyDescriptors(\"ab\"); [d[0].value, d.length.value].join();",
        "\"a,2\"",
    );
    both_throw("Object.getOwnPropertyDescriptors(null);", "TypeError");
}

/// The steps the script `source` spends in `realm`, which must complete normally.
fn steps_of(agent: &mut Agent<TestHost>, realm: &Realm, source: &str) -> u64 {
    let outcome = eval_text(agent, realm, source);
    assert!(
        matches!(outcome, Outcome::Normal(_)),
        "{source}: {outcome:?}"
    );
    agent.last_entry_steps().unwrap()
}

/// Every loop of these builtins runs for the script and is charged a step per key
/// (`docs/design/engine.md`, decision 4). Two objects of 10 and 100 String keys, each past the
/// small size of the property map, so that a lookup is as cheap in both: the same call on the
/// larger one spends `90 * per_key` steps more. Per key: [[OwnPropertyKeys]] 1 and the loop of
/// EnumerableOwnProperties 1, then for `keys` and `values` CreateArrayFromList 1, for `entries`
/// CreateArrayFromList of each entry 2 and of the list 1; the loop of `assign` 1 and its Set,
/// which goes up from the new target to `%Object.prototype%` (a step per prototype, as Get),
/// 1; the loop of `getOwnPropertyDescriptors` 1 after [[OwnPropertyKeys]].
#[test]
fn keys_charge_a_step_per_key() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var small = {}, large = {};\n\
             for (var i = 0; i < 10; i++) small[\"k\" + i] = i;\n\
             for (i = 0; i < 100; i++) large[\"k\" + i] = i;"
        ),
        Outcome::Normal("99".to_string())
    );
    for (call, per_key) in [
        ("Object.keys(X);", 3),
        ("Object.values(X);", 3),
        ("Object.entries(X);", 5),
        ("Object.assign({}, X);", 3),
        ("Object.getOwnPropertyDescriptors(X);", 2),
    ] {
        let small = steps_of(&mut agent, &realm, &call.replace('X', "small"));
        let large = steps_of(&mut agent, &realm, &call.replace('X', "large"));
        assert_eq!(large - small, 90 * per_key, "{call}: {small} and {large}");
    }
}

/// An object of `count` String keys `k0`, `k1`, ... as the global `big`, made through the host
/// interface, whose calls the step budget of a script does not count.
fn global_with_keys(agent: &mut Agent<TestHost>, realm: &Realm, count: u32) {
    let global_root = global(agent, realm);
    agent
        .with_cx(realm, |cx| {
            let global = object(cx, &global_root);
            let constructor = support::host::get(cx, global, "Object");
            let Value::Object(big) = cx
                .call(constructor, Value::Undefined, &[])
                .unwrap()
                .unwrap()
            else {
                panic!("Object() answered no object");
            };
            let big_key = key(cx, "big");
            cx.set(global, big_key, Value::Object(big), true)
                .unwrap()
                .unwrap();
            for index in 0..count {
                let name = key(cx, &format!("k{index}"));
                cx.set(big, name, Value::Number(f64::from(index)), true)
                    .unwrap()
                    .unwrap();
            }
        })
        .unwrap();
}

/// The step budget ends each of these builtins on an object of many keys in
/// `Terminated(StepBudget)`, never in a hang: under 1,000 steps an object of 600 keys passes
/// [[OwnPropertyKeys]] and is ended by the builtin's own loop, one of 5,000 keys by
/// [[OwnPropertyKeys]]; the same call on an object of one key completes.
#[test]
fn step_budget_ends_keys_of_many_properties() {
    let mut limits = limits();
    limits.steps_per_entry = 1_000;
    for count in [600, 5_000] {
        let mut agent = agent_with(limits).unwrap();
        let realm = realm(&mut agent);
        global_with_keys(&mut agent, &realm, count);
        for call in [
            "Object.keys(big);",
            "Object.values(big);",
            "Object.entries(big);",
            "Object.assign({}, big);",
            "Object.getOwnPropertyDescriptors(big);",
        ] {
            assert_eq!(
                eval_text(&mut agent, &realm, call),
                Outcome::Terminated(Termination::StepBudget),
                "{call} of {count} keys"
            );
            let one = call.replace("big", "{ a: 1 }");
            assert!(
                matches!(eval_text(&mut agent, &realm, &one), Outcome::Normal(_)),
                "{one}"
            );
        }
    }
}

/// The bytes an agent with the test limits holds after `create_realm`.
fn realm_bytes() -> u64 {
    let mut agent = agent().unwrap();
    let _realm = realm(&mut agent);
    agent.collect().unwrap();
    agent.heap_bytes().unwrap()
}

/// The memory budget (decision 4): with 300,000 bytes beyond the realm, a String of 50,000 code
/// units fits, but its keys do not: `Object.keys` makes a String and an element for each, and
/// `Object.assign` charges its list of 50,000 keys before it copies one. Each ends in
/// `Terminated(MemoryBudget)`, and the agent answers the next script.
#[test]
fn memory_budget_ends_the_keys_of_a_long_string() {
    let mut limits = limits();
    limits.memory_bytes = realm_bytes() + 300_000;
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    let global_root = global(&mut agent, &realm);
    agent
        .with_cx(&realm, |cx| {
            let global = object(cx, &global_root);
            let long = Value::String(cx.new_string(&"a".repeat(50_000)).unwrap());
            let long_key = key(cx, "long");
            cx.set(global, long_key, long, true).unwrap().unwrap();
        })
        .unwrap();
    for call in ["Object.keys(long);", "Object.assign({}, long);"] {
        assert_eq!(
            eval_text(&mut agent, &realm, call),
            Outcome::Terminated(Termination::MemoryBudget),
            "{call}"
        );
        assert_eq!(
            eval_text(&mut agent, &realm, "Object.keys(\"abc\").join();"),
            Outcome::Normal("\"0,1,2\"".to_string()),
            "after {call}"
        );
    }
}

/// The global `s`, a String of 1,000,000 code units, defined through the host interface.
fn global_long_string(agent: &mut Agent<TestHost>, realm: &Realm) {
    let global_root = global(agent, realm);
    agent
        .with_cx(realm, |cx| {
            let global = object(cx, &global_root);
            let long = Value::String(cx.new_string(&"a".repeat(1_000_000)).unwrap());
            let s = key(cx, "s");
            cx.set(global, s, long, true).unwrap().unwrap();
        })
        .unwrap();
}

/// `docs/design/engine.md`, "The step budget" and "Memory budget": EnumerateObjectProperties
/// asks [[OwnPropertyKeys]] of the String object, which charges the list of its 1,000,001 keys
/// (`size_of::<Key>()` bytes each) before it builds it. With 1 MiB left beyond the String,
/// `for (var k in new String(s)) break;` ends in `Terminated(MemoryBudget)` at that list:
/// nothing the walk over the keys would make is charged (the heap holds less than 64 KiB more
/// than before the script when the run ends), and the agent answers the next script.
#[test]
fn for_in_over_a_long_string_object_is_charged_before_its_keys_exist() {
    let mut limits = limits();
    limits.steps_per_entry = 10_000_000;
    let mut measure = agent_with(limits).unwrap();
    let measured = realm(&mut measure);
    global_long_string(&mut measure, &measured);
    let base = measure.heap_bytes().unwrap();
    limits.memory_bytes = base + (1 << 20);
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    global_long_string(&mut agent, &realm);
    let before = agent.heap_bytes().unwrap();
    assert_eq!(
        eval_text(&mut agent, &realm, "for (var k in new String(s)) break; 1"),
        Outcome::Terminated(Termination::MemoryBudget)
    );
    let held = agent.heap_bytes().unwrap().saturating_sub(before);
    assert!(held < 64 * 1024, "{held} bytes held when the run ended");
    assert_eq!(
        eval_text(&mut agent, &realm, "1"),
        Outcome::Normal("1".to_string())
    );
}

/// The steps and the heap after `Object.keys` of an object of 5,000 index keys added in the
/// order i × 3,001 mod 5,000 (a permutation: 3,001 is prime to 5,000), made through the host
/// interface.
fn steps_and_bytes_of_unordered_keys() -> (u64, u64) {
    const N: u32 = 5_000;
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let global_root = global(&mut agent, &realm);
    agent
        .with_cx(&realm, |cx| {
            let global = object(cx, &global_root);
            let constructor = get(cx, global, "Object");
            let Value::Object(big) = cx
                .call(constructor, Value::Undefined, &[])
                .unwrap()
                .unwrap()
            else {
                panic!("Object() answered no object");
            };
            let big_key = key(cx, "big");
            cx.set(global, big_key, Value::Object(big), true)
                .unwrap()
                .unwrap();
            for i in 0..N {
                let index = key(cx, &(i * 3_001 % N).to_string());
                cx.set(big, index, Value::Number(f64::from(i)), true)
                    .unwrap()
                    .unwrap();
            }
        })
        .unwrap();
    assert_eq!(
        eval_text(&mut agent, &realm, "Object.keys(big).length;"),
        Outcome::Normal("5000".to_string())
    );
    (
        agent.last_entry_steps().unwrap(),
        agent.heap_bytes().unwrap(),
    )
}

/// Determinism (`docs/design/engine.md`, "The step budget"): the same enumeration of keys
/// added out of order, sorted pass by pass, spends the same steps and leaves the same heap in
/// two agents.
#[test]
fn the_same_enumeration_twice_spends_the_same_steps() {
    assert_eq!(
        steps_and_bytes_of_unordered_keys(),
        steps_and_bytes_of_unordered_keys()
    );
}

/// `Object.assign` calls a getter of a source (Get) and a setter of the target (Set) from Rust:
/// each is a re-entry. `f` recurses through each until the re-entry limit throws a
/// `RangeError`, on seven eighths of `min_stack_bytes()` of that depth, without a stack
/// overflow (`AGENTS.md`, "Commands and the gate"). The depth is [`ASSIGN_REENTRY`].
#[test]
fn assign_getter_and_setter_reach_the_reentry_limit_on_the_promised_stack() {
    let stack = stack_with_room(stack_limits(ASSIGN_REENTRY));
    on_bounded_stack(
        test_path!("assign_getter_and_setter_reach_the_reentry_limit_on_the_promised_stack"),
        stack,
        || {
            for through in [
                "Object.assign({}, { get p() { return f(); } })",
                "Object.assign({ set p(v) { f(); } }, { p: 1 })",
            ] {
                reach_through(through, ASSIGN_REENTRY);
            }
        },
    )
    .unwrap();
}

/// The `reentry_depth` of the re-entry test through `Object.assign`: the full depth in the
/// default mode, where the stack bound is measured, since only there the bytes of the
/// re-entries outweigh the fixed part and the parser's of `min_stack_bytes()`; with the feature
/// `collect-every-safe-point`, 7. That mode looks for a value held without a root across a
/// safe point, which a getter and a setter nested seven deep through the builtin show as one
/// at the limit of 100 does: a collection at every safe point makes the cost grow with the
/// square of the depth (5.9 s at 100, measured).
const ASSIGN_REENTRY: u32 = if cfg!(feature = "collect-every-safe-point") {
    7
} else {
    FULL_REENTRY
};

/// The only Symbol values a builtin of the realm hands to a script through a String-keyed data
/// property are the twelve well-known symbols of slice 6 on `Symbol`
/// (`#sec-properties-of-the-symbol-constructor`,
/// `story:symbol-constructor-and-well-known-symbols`; before it, none: design, section 12, "A
/// part before its slice"): a walk from the global object over every own String-keyed data
/// property, and the prototype, of each object it reaches, calling no getter and no method of
/// the objects it walks, finds a Symbol value at exactly those twelve places. The walk reads
/// what a script reads, through the host interface: `Object.getOwnPropertyNames`,
/// `Object.getOwnPropertyDescriptors` and `Object.getPrototypeOf` of each object once, then the
/// "value" of each data descriptor; it runs no script, so that with the feature
/// `collect-every-safe-point` it pays a collection per call of those three, not one per
/// instruction of a walk in script (26.6 s in that mode, measured). A Symbol-keyed property is
/// out of the walk's reach; a script reaches one through `Object.getOwnPropertySymbols`.
#[test]
fn no_builtin_hands_a_symbol_value_to_a_script() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let global_root = global(&mut agent, &realm);
    let (walked, found) = agent
        .with_cx(&realm, |cx| {
            let global = object(cx, &global_root);
            let object_constructor = get_object(cx, global, "Object");
            let names_of = get(cx, object_constructor, "getOwnPropertyNames");
            let descriptors_of = get(cx, object_constructor, "getOwnPropertyDescriptors");
            let prototype_of = get(cx, object_constructor, "getPrototypeOf");
            let call = |cx: &mut Cx<'_, TestHost>, function: Value, o: ObjectRef| {
                cx.call(function, Value::Undefined, &[Value::Object(o)])
                    .unwrap()
                    .unwrap()
            };
            let mut queue = vec![(global, "global".to_string())];
            let mut seen = vec![global];
            let mut found = Vec::new();
            let mut next = 0;
            while let Some((o, path)) = queue.get(next).cloned() {
                next += 1;
                let Value::Object(names) = call(cx, names_of, o) else {
                    panic!("{path}: no names");
                };
                let Value::Object(descriptors) = call(cx, descriptors_of, o) else {
                    panic!("{path}: no descriptors");
                };
                let Value::Number(length) = get(cx, names, "length") else {
                    panic!("{path}: no length");
                };
                let mut values = Vec::new();
                for index in 0..length as u32 {
                    let name = get(cx, names, &index.to_string());
                    let name = text(cx, name);
                    let descriptor = get_object(cx, descriptors, &name);
                    let value_key = key(cx, "value");
                    if cx.has_property(descriptor, value_key).unwrap().unwrap() {
                        values.push((format!("{path}.{name}"), get(cx, descriptor, "value")));
                    }
                }
                let prototype = call(cx, prototype_of, o);
                values.push((format!("{path}.[[Prototype]]"), prototype));
                for (place, value) in values {
                    match value {
                        Value::Symbol(_) => found.push(place),
                        Value::Object(reached) if !seen.contains(&reached) => {
                            seen.push(reached);
                            queue.push((reached, place));
                        }
                        _ => {}
                    }
                }
            }
            (seen.len(), found)
        })
        .unwrap();
    let mut found = found;
    found.sort();
    let mut expected: Vec<String> = [
        "hasInstance",
        "isConcatSpreadable",
        "iterator",
        "match",
        "matchAll",
        "replace",
        "search",
        "species",
        "split",
        "toPrimitive",
        "toStringTag",
        "unscopables",
    ]
    .iter()
    .map(|name| format!("global.Symbol.{name}"))
    .collect();
    expected.sort();
    assert_eq!(found, expected, "Symbol values");
    assert!(walked > 50, "only {walked} objects walked");
}

/// No script faults the agent through a Symbol value (design, section 12, "A part before its
/// slice"): a script that asks `Object.getOwnPropertySymbols` for the first Symbol key of
/// `Array` (`@@species`) and hands it to ToObject (`Object(s)`, `Object.keys(s)`,
/// `Object.hasOwn(s, "x")`) gets a Symbol object (`#sec-toobject`, slice 6,
/// `story:symbol-constructor-and-well-known-symbols`) and its answers, and the agent stays
/// alive. Before slice 6 the method was left out: with it, each of the three ended in
/// `Terminated(EngineFault)` and a faulted agent (ToObject of a Symbol, `ops/conversion.rs`),
/// measured before it was removed.
#[test]
fn no_script_faults_the_agent_through_a_symbol() {
    let mut found = Vec::new();
    for call in [
        "[typeof Object(s), Object(s).valueOf() === s].join();",
        "[Object.keys(s).length].join();",
        "[Object.hasOwn(s, \"x\")].join();",
    ] {
        let mut agent = agent().unwrap();
        let realm = realm(&mut agent);
        let source = format!(
            "var s = Object.getOwnPropertySymbols(Array)[0];\n\
             if (s !== Symbol.species) throw new Error(\"not @@species\");\n{call}"
        );
        found.push((eval_text(&mut agent, &realm, &source), agent.state()));
    }
    let normal = |text: &str| (Outcome::Normal(format!("\"{text}\"")), AgentState::Alive);
    assert_eq!(found, [normal("object,true"), normal("0"), normal("false")]);
}
