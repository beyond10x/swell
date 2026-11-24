//! `Array.from` (`#sec-array.from`, `story:array-from-and-object-from-entries`): the iterable
//! path through GetIteratorFromMethod (`#sec-getiteratorfrommethod`) and IteratorStepValue
//! (`#sec-iteratorstepvalue`), closed by IteratorClose (`#sec-iteratorclose`) when the mapping
//! function or the definition of an element ends abruptly, and the array-like path; a `this`
//! that is a constructor is constructed. The design is `docs/design/engine.md`, decision 3,
//! "Iterators held by a builtin". Two tests hold all three builtins of the story,
//! `Array.from`, `Object.fromEntries` (`#sec-object.fromentries`) and `Object.groupBy`
//! (`#sec-object.groupby`): over an iterator that is never done each ends at the step and at
//! the memory budget, and an abrupt completion of its own work closes the iterator once where
//! a termination calls no `return`. Run also with the feature `collect-every-safe-point`.
//!
//! No script uses `for`-`of`, a spread or an array pattern: an iterable is an Array, a String or
//! an object whose `[Symbol.iterator]` the script writes.
#![cfg(test)]

use crate::support;

use std::cell::Cell;

use support::host::{agent, agent_with, limits, realm, stack_with_room};
use support::script::{Outcome, define_function, eval, eval_text, run_both};
use support::stack::{BUILTINS_REENTRY, on_bounded_stack, reach_through, stack_limits, test_path};
use swell::{
    Completion, Cx, HostCall, InterruptHandle, Outcome as HostOutcome, Termination, Value,
};

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

/// The iterable the scripts write: `iterable(values, ret)` answers the elements of the Array
/// `values` in order, then done, and has `ret` as its `return` when it is not undefined.
const ITERABLE: &str = "function iterable(values, ret) {\n\
       var o = {};\n\
       o[Symbol.iterator] = function () {\n\
         var i = 0;\n\
         var it = { next: function () { i++; return { value: values[i - 1], done: i > values.length }; } };\n\
         if (ret !== undefined) it[\"return\"] = ret;\n\
         return it;\n\
       };\n\
       return o;\n\
     }\n";

/// `#sec-array.from`: an Array, a String (by code point, a surrogate pair one value), an Array
/// Iterator and an iterable the script writes are read through their iterator, whose `length`
/// is not read; an object without `[Symbol.iterator]` is array-like: its `length` is read once
/// and each index below it by Get, a hole read as undefined, and a primitive is made an object
/// by ToObject. The answer is an Array of the realm with its `length` set. `items` undefined or
/// null throws a `TypeError` (GetMethod, step 4), as does a `[Symbol.iterator]` that is not
/// callable, or a mapper that is neither undefined nor callable, before `items` is read. The
/// function is `Array.from`, "length" 1, a method of `Array`.
#[test]
fn from_an_iterable_and_from_an_array_like() {
    answers(
        &[
            ITERABLE,
            "var r = [];\n\
             var a = Array.from([1, 2, 3]);\n\
             r.push(a.join(\"\"), Array.isArray(a), Object.getPrototypeOf(a) === Array.prototype, a.length);\n\
             var s = Array.from(\"a\\uD83D\\uDE00b\");\n\
             r.push(s.length, s[1].length, s[2]);\n\
             r.push(Array.from([7, 8].values()).join(\"\"), Array.from(iterable([4, 5])).join(\"\"));\n\
             var reads = 0, withLength = iterable([6]);\n\
             Object.defineProperty(withLength, \"length\", { get: function () { reads++; return 9; } });\n\
             r.push(Array.from(withLength).length, reads);\n\
             var like = { length: 3, 0: \"x\", 2: \"z\" }, lengthReads = 0;\n\
             var counted = { get length() { lengthReads++; return 2; }, 0: \"p\", 1: \"q\" };\n\
             var l = Array.from(like);\n\
             r.push(l.length, l[0], l[1], 1 in l, l[2], Array.from(counted).join(\"\"), lengthReads);\n\
             r.push(Array.from({}).length, Array.from(5).length, Array.from({ length: \"2\" }).length);\n\
             var f = Array.from, d = Object.getOwnPropertyDescriptor(Array, \"from\");\n\
             r.push(f.name, f.length, d.writable, d.enumerable, d.configurable);\n\
             r.join();",
        ]
        .concat(),
        "\"123,true,true,3,3,2,b,78,45,1,0,3,x,,true,z,pq,1,0,0,2,from,1,true,false,true\"",
    );
    for source in [
        "Array.from(undefined);",
        "Array.from(null);",
        "Array.from();",
        "var o = {}; o[Symbol.iterator] = 1; Array.from(o);",
        "Array.from([], null);",
        "Array.from([], {});",
        "var read = false; Array.from({ get length() { read = true; return 0; } }, 1);",
    ] {
        throws(source, "TypeError");
    }
    answers(
        "var read = false;\n\
         try { Array.from({ get length() { read = true; return 0; } }, 1); } catch (e) {}\n\
         read;",
        "false",
    );
}

/// The steps of the entry `Array.from(it)`, with `it` an Array Iterator over the first `n`
/// elements of an array of 1,000 made by an earlier entry.
fn steps_of_from_an_array_iterator(n: u32) -> u64 {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let setup = format!("var a = new Array(1000).fill(0).slice(0, {n}), it = a.values(); a.length");
    assert_eq!(
        eval_text(&mut agent, &realm, &setup),
        Outcome::Normal(n.to_string())
    );
    // Read right after `evaluate_script`: reading the answer is an entry of its own.
    let outcome = eval(&mut agent, &realm, "Array.from(it);");
    assert!(matches!(outcome, HostOutcome::Normal(_)), "{outcome:?}");
    agent.last_entry_steps().unwrap()
}

/// `docs/design/engine.md`, decision 3, "Iterators held by a builtin", item 3: the loop of
/// `Array.from` charges a step per iteration through `charge_steps`, beside the step the call
/// path charges for each call of `next`, also when `next` is `%ArrayIteratorPrototype%.next`,
/// a builtin that runs no instruction. `last_entry_steps()` of `Array.from` of an Array
/// Iterator over 1,000 elements exceeds that over 10 by at least 2 × 990. The earlier script
/// runs no loop of its own, so the count is the same with the feature
/// `collect-every-safe-point`.
#[test]
fn from_charges_its_loop_when_next_is_a_builtin() {
    let n = 1_000;
    let many = steps_of_from_an_array_iterator(n);
    let ten = steps_of_from_an_array_iterator(10);
    println!("{many} steps for {n} elements, {ten} for 10");
    assert!(
        many >= ten + 2 * u64::from(n - 10),
        "{many} steps for {n} elements, {ten} for 10"
    );
}

/// `docs/design/engine.md`, decision 3, "Iterators held by a builtin", item 1: what `Array.from`
/// holds across a call is kept where the collector finds it. Each `next` makes a new object
/// that only the result of `next` holds, the mapper makes a new object from it and from its
/// this value, a new object only the call passes, and the constructor `this` answers a new
/// object only `Array.from` holds; the array-like path reads values a getter makes. With the
/// feature `collect-every-safe-point` every instruction and every call collects, so a value
/// held without a root is freed and read back wrong.
#[test]
fn from_keeps_its_values_across_a_collection_in_next() {
    answers(
        "var o = {};\n\
         o[Symbol.iterator] = function () {\n\
           var i = 0;\n\
           return { next: function () { i++; var made = { v: i }; return { value: made, done: i > 4 }; } };\n\
         };\n\
         function C() { return { made: true }; }\n\
         var mapper = function (x, k) { return { v: x.v * this.by, k: k }; };\n\
         var r = Array.from.call(C, o, mapper, { by: 10 });\n\
         var out = [r.made, r.length];\n\
         for (var i = 0; i < 4; i++) out.push(r[i].v + \":\" + r[i].k);\n\
         var like = { length: 3, get 0() { return { v: 1 }; }, get 1() { return { v: 2 }; }, get 2() { return { v: 3 }; } };\n\
         var l = Array.from(like, mapper, { by: 100 });\n\
         for (var j = 0; j < 3; j++) out.push(l[j].v);\n\
         var plain = Array.from(o);\n\
         out.push(plain.length, plain[3].v);\n\
         out.join();",
        "\"true,4,10:0,20:1,30:2,40:3,100,200,300,4,4\"",
    );
}

/// `#sec-array.from`, steps 5.e.v and 5.e.vii: the mapper is called with the value and its
/// index, two arguments, with `thisArg` as its this value (undefined when absent, which
/// sloppy code reads as the global object); when it throws, IteratorClose calls `return` once
/// and the mapper's throw is what `Array.from` throws, also when `return` throws; when
/// CreateDataPropertyOrThrow fails (a constructor that answers a non-extensible object) the
/// iterator is closed once and the `TypeError` thrown. A throw of `next` closes nothing. The
/// array-like path calls the mapper the same way.
#[test]
fn from_calls_mapfn_with_value_and_index_and_closes_the_iterator_when_it_throws() {
    answers(
        &[
            ITERABLE,
            "var seen = [], self = { name: \"self\" };\n\
             var r = Array.from(iterable([\"a\", \"b\"]), function (v, k) {\n\
               seen.push(arguments.length + \":\" + v + k + \":\" + (this === self));\n\
               return v + v;\n\
             }, self);\n\
             seen.push(r.join(\"\"));\n\
             Array.from({ length: 1, 0: \"c\" }, function (v, k) { seen.push(arguments.length + v + k + (this === self)); }, self);\n\
             var global = this;\n\
             Array.from([1], function () { \"use strict\"; seen.push(this === undefined); });\n\
             Array.from([1], function () { seen.push(this === global || this === undefined); });\n\
             var returns = 0;\n\
             function counting() { return function () { returns++; return {}; }; }\n\
             function caught(f) { try { f(); return \"none\"; } catch (e) { return e instanceof TypeError ? \"TypeError\" : e; } }\n\
             seen.push(caught(function () { Array.from(iterable([1, 2, 3], counting()), function (v) { if (v === 2) throw \"map\"; }); }), returns);\n\
             seen.push(caught(function () { Array.from(iterable([1], function () { returns++; throw \"return\"; }), function () { throw \"map\"; }); }), returns);\n\
             function Frozen() { return Object.preventExtensions({}); }\n\
             seen.push(caught(function () { Array.from.call(Frozen, iterable([1], counting())); }), returns);\n\
             var throwing = {};\n\
             throwing[Symbol.iterator] = function () { return { next: function () { throw \"next\"; }, \"return\": counting() }; };\n\
             seen.push(caught(function () { Array.from(throwing, function () {}); }), returns);\n\
             seen.push(caught(function () { Array.from(iterable([1, 2], counting())); }), returns);\n\
             seen.join();",
        ]
        .concat(),
        "\"2:a0:true,2:b1:true,aabb,2c0true,true,true,map,1,map,2,TypeError,3,next,3,none,3\"",
    );
}

/// `#sec-array.from`, steps 5.a, 5.b, 5.e.iv, 9 and 12: a `this` that is a constructor is constructed,
/// with no argument on the iterable path and with the length on the array-like path; its
/// object gets the elements and then `length` by Set, which runs a setter of its prototype. A
/// `this` that is no constructor (undefined, an arrow function, a plain object, a method) makes
/// an Array.
#[test]
fn from_with_a_constructor_this_constructs_it() {
    answers(
        "var log = [];\n\
         function C() { log.push(\"C\" + arguments.length + (arguments.length ? arguments[0] : \"\")); }\n\
         Object.defineProperty(C.prototype, \"length\", { set: function (v) { log.push(\"length=\" + v); }, configurable: true });\n\
         var a = Array.from.call(C, [\"x\", \"y\"]);\n\
         var b = Array.from.call(C, { length: 1, 0: \"z\" });\n\
         log.push(a instanceof C, Array.isArray(a), a[0] + a[1], b instanceof C, b[0]);\n\
         var others = [undefined, () => 1, {}, Math.max];\n\
         for (var i = 0; i < others.length; i++) {\n\
           var made = Array.from.call(others[i], [1, 2]);\n\
           log.push(Array.isArray(made) && Object.getPrototypeOf(made) === Array.prototype && made.length === 2);\n\
         }\n\
         log.join();",
        "\"C0,length=2,C11,length=1,true,false,xy,true,z,true,true,true,true\"",
    );
}

/// The steps of the entry `Array.from(<source>)` after an earlier entry made `small`, an Array
/// of 10 elements, `big`, one of 1,000, and `likeSmall` and `likeBig`, array-like objects of
/// those lengths.
fn steps_of_from(source: &str) -> u64 {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var small = new Array(10).fill(1), big = new Array(1000).fill(1);\n\
             var likeSmall = { length: 10 }, likeBig = { length: 1000 }; big.length"
        ),
        Outcome::Normal("1000".to_string())
    );
    // Read right after `evaluate_script`: reading the answer is an entry of its own.
    let outcome = eval(&mut agent, &realm, &format!("Array.from({source});"));
    assert!(matches!(outcome, HostOutcome::Normal(_)), "{outcome:?}");
    agent.last_entry_steps().unwrap()
}

/// `docs/design/engine.md`, decision 2: `Array.from` charges a step per element in both paths.
/// `last_entry_steps()` after `Array.from(big)` exceeds that after `Array.from(small)` by at
/// least 990, and so does `Array.from(likeBig)` over `Array.from(likeSmall)`, whose loop calls
/// nothing.
#[test]
fn from_charges_a_step_per_element() {
    for (big, small) in [("big", "small"), ("likeBig", "likeSmall")] {
        let many = steps_of_from(big);
        let ten = steps_of_from(small);
        println!("{big}: {many} steps, {small}: {ten}");
        assert!(many >= ten + 990, "{big}: {many} steps, {small}: {ten}");
    }
}

/// `Limits::min_stack_bytes` through `Array.from` (decision 3, "Depth bounds"): the mapper is a
/// call from Rust, a re-entry, so `f` re-enters itself through it until the re-entry limit, on
/// seven eighths of `min_stack_bytes()` (`AGENTS.md`, "Commands and the gate"), in a child
/// process. In the collect mode the depth is [`BUILTINS_REENTRY`].
#[test]
fn from_re_enters_through_mapfn_on_the_promised_stack() {
    fn body() {
        reach_through("Array.from([0], f)", BUILTINS_REENTRY);
    }
    let stack = stack_with_room(stack_limits(BUILTINS_REENTRY));
    on_bounded_stack(
        test_path!("from_re_enters_through_mapfn_on_the_promised_stack"),
        stack,
        body,
    )
    .unwrap();
}

/// The iterable of [`the_three_end_at_the_step_and_the_memory_budget_over_an_endless_iterator`]:
/// `endless(n, work)` answers `work(i)` for `i` from 1 to `n` (`Infinity` for no end), and its
/// `return` counts its calls in `returns`; `id` answers its argument, `pair(i)` the entry
/// `[i, i]`.
const ENDLESS: &str = "var returns = 0;\n\
     function endless(n, work) {\n\
       var o = {};\n\
       o[Symbol.iterator] = function () {\n\
         var i = 0;\n\
         return {\n\
           next: function () { i++; return i > n ? { done: true } : { value: work(i), done: false }; },\n\
           \"return\": function () { returns++; return {}; }\n\
         };\n\
       };\n\
       return o;\n\
     }\n\
     function id(i) { return i; }\n\
     function pair(i) { return [i, i]; }\n\
     true;";

/// The String the memory runs keep a copy of per value, a flat String of 1,024 code units, made
/// by an entry of its own; `big(i)` answers a new flat String of 65,536 code units (128 KiB),
/// `bigPair(i)` the entry `[i, big(i)]`. A String made by `+` is a rope of its parts and would
/// not grow the heap by its length.
const CHUNK: &str = "var chunk = \"x\".repeat(1024);\n\
     function big(i) { return chunk.repeat(64); }\n\
     function bigPair(i) { return [i, big(i)]; }\n\
     true;";

/// The script that runs the three builtins over `endless(n, ...)`: `Array.from` with no mapper,
/// `Object.fromEntries` and `Object.groupBy` with `id` as its callback, a group per value; the
/// values are `id(i)` or, when `grows`, `big(i)`.
fn three_over(n: &str, grows: bool) -> [String; 3] {
    let (value, entry) = if grows {
        ("big", "bigPair")
    } else {
        ("id", "pair")
    };
    [
        format!("Array.from(endless({n}, {value})).length;"),
        format!("Object.keys(Object.fromEntries(endless({n}, {entry}))).length;"),
        format!(
            "Object.keys(Object.groupBy(endless({n}, {value}), function (v, k) {{ return k; }})).length;"
        ),
    ]
}

/// The step room of [`the_three_end_at_the_step_and_the_memory_budget_over_an_endless_iterator`]
/// above the steps of the control: 1,000; 100 with the feature `collect-every-safe-point`, where
/// every instruction collects.
const STEP_ROOM: u64 = if cfg!(feature = "collect-every-safe-point") {
    100
} else {
    1_000
};

/// The memory room above the reading of an agent that ran the setups of the memory runs and
/// collected: 1 MiB, eight Strings of `big` (128 KiB each), so the eighth does not fit beside
/// what the run makes besides them; the seven before it take 917,999 steps of copying (at every
/// size of the realm the task `test-realm-sizes` pads), within the 1,000,000 steps of
/// `limits()`.
const MEMORY_ROOM: u64 = 1 << 20;

/// `docs/design/engine.md`, decision 3, "Iterators held by a builtin", items 2 and 3: each of
/// `Array.from`, `Object.fromEntries` and `Object.groupBy` over an iterator that is never done
/// ends in `Terminated(StepBudget)` when its values are small and in
/// `Terminated(MemoryBudget)` when it keeps a new String of 128 KiB per value (in the Array,
/// the object or the groups it builds), never in a hang, and no termination calls `return`.
/// The step budget is the largest reading of the three controls (each over 3 values) plus
/// [`STEP_ROOM`]; the memory budget is the reading of a twin agent after the setups of the
/// memory runs and a collection plus [`MEMORY_ROOM`], and the budgeted agent reads the same
/// after its own setups and a collection, so no arena grows between the reading and the run;
/// each control completes under them. After each termination the agent runs
/// again: after a collection the script `1` answers `Normal(1)` and the control its count
/// (HOST-70, HOST-190).
#[test]
fn the_three_end_at_the_step_and_the_memory_budget_over_an_endless_iterator() {
    let controls = three_over("3", false);
    let mut measure = agent().unwrap();
    let measured = realm(&mut measure);
    for setup in [ENDLESS, CHUNK] {
        assert_eq!(
            eval_text(&mut measure, &measured, setup),
            Outcome::Normal("true".to_string())
        );
    }
    let mut steps = 0;
    for control in &controls {
        let outcome = eval(&mut measure, &measured, control);
        assert!(matches!(outcome, HostOutcome::Normal(_)), "{outcome:?}");
        steps = steps.max(measure.last_entry_steps().unwrap());
    }
    println!("controls: at most {steps} steps");

    for (grows, termination) in [
        (false, Termination::StepBudget),
        (true, Termination::MemoryBudget),
    ] {
        // The 1,024 steps of copying `chunk` would not fit the step budget of the step runs,
        // which need no `big`.
        let setups: &[&str] = if grows { &[ENDLESS, CHUNK] } else { &[ENDLESS] };
        let mut budgeted = limits();
        let mut twin_heap = None;
        if grows {
            // A twin agent runs the setups of the budgeted agent and collects: its reading is
            // the budgeted agent's before its first run, whatever the size of the realm.
            let mut twin = agent().unwrap();
            let twin_realm = realm(&mut twin);
            for setup in setups {
                assert_eq!(
                    eval_text(&mut twin, &twin_realm, setup),
                    Outcome::Normal("true".to_string())
                );
            }
            twin.collect().unwrap();
            let heap = twin.heap_bytes().unwrap();
            budgeted.memory_bytes = heap + MEMORY_ROOM;
            twin_heap = Some(heap);
        } else {
            budgeted.steps_per_entry = steps + STEP_ROOM;
        }
        let mut agent = agent_with(budgeted).unwrap();
        let realm = realm(&mut agent);
        for setup in setups {
            assert_eq!(
                eval_text(&mut agent, &realm, setup),
                Outcome::Normal("true".to_string())
            );
        }
        if let Some(heap) = twin_heap {
            agent.collect().unwrap();
            assert_eq!(
                agent.heap_bytes().unwrap(),
                heap,
                "the twin's reading after the setups and a collection"
            );
        }
        for (source, control) in three_over("Infinity", grows).iter().zip(&controls) {
            let before = agent.heap_bytes().unwrap();
            assert_eq!(
                eval_text(&mut agent, &realm, source),
                Outcome::Terminated(termination),
                "{source}"
            );
            println!(
                "{source}: heap {before} before, {} at the termination, {} steps",
                agent.heap_bytes().unwrap(),
                agent.last_entry_steps().unwrap()
            );
            agent.collect().unwrap();
            assert_eq!(
                eval_text(&mut agent, &realm, "returns"),
                Outcome::Normal("0".to_string()),
                "{source}"
            );
            assert_eq!(
                eval_text(&mut agent, &realm, "1"),
                Outcome::Normal("1".to_string()),
                "after {source}"
            );
            assert_eq!(
                eval_text(&mut agent, &realm, control),
                Outcome::Normal("3".to_string()),
                "the control after {source}"
            );
        }
    }
}

thread_local! {
    /// The handle `interruptNow` sets.
    static HANDLE: Cell<Option<InterruptHandle>> = const { Cell::new(None) };
}

/// A host function that sets its agent's interrupt flag once.
fn interrupt_now(
    _cx: &mut Cx<'_, support::host::TestHost>,
    _call: HostCall<'_>,
) -> Completion<Value> {
    HANDLE.with(|slot| {
        if let Some(handle) = slot.take() {
            handle.interrupt();
        }
    });
    Ok(Value::Undefined)
}

/// The iterable of [`the_three_close_once_on_an_abrupt_completion_and_a_termination_calls_no_return`]:
/// `closing(values)` answers the elements of `values` and counts the calls of its `return` in
/// `returns`; `caught(f)` answers what `f` threw, "TypeError" for a `TypeError`, or "none".
const CLOSING: &str = "var returns = 0;\n\
     function closing(values) {\n\
       var o = {};\n\
       o[Symbol.iterator] = function () {\n\
         var i = 0;\n\
         return {\n\
           next: function () { i++; return { value: values[i - 1], done: i > values.length }; },\n\
           \"return\": function () { returns++; return {}; }\n\
         };\n\
       };\n\
       return o;\n\
     }\n\
     function caught(f) { try { f(); return \"none\"; } catch (e) { return e instanceof TypeError ? \"TypeError\" : e; } }\n\
     function badKey(thrown) { return { toString: function () { throw thrown; } }; }\n\
     function Frozen() { return Object.preventExtensions({}); }\n\
     true;";

/// `docs/design/engine.md`, decision 3, "Iterators held by a builtin", item 4, with
/// IfAbruptCloseIterator (`#sec-ifabruptcloseiterator`): an abrupt completion of a builtin's own
/// work between two steps calls `return` once and throws what the work threw: the mapper and
/// the definition of an element of `Array.from` (`#sec-array.from`, steps 5.e.v and 5.e.vii),
/// an entry that is no object, a throw of reading its `0` or `1` and of ToPropertyKey of its key
/// in `Object.fromEntries` (`#sec-add-entries-from-iterable`), the callback and ToPropertyKey of
/// its answer in `Object.groupBy` (`#sec-groupby`, steps 6.f and 6.g.ii). An interrupt set in the
/// mapper, in the callback and in the `toString` of a key ends the run in
/// `Terminated(Interrupted)` and calls no `return`; the agent runs the next entry (HOST-70).
#[test]
fn the_three_close_once_on_an_abrupt_completion_and_a_termination_calls_no_return() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    define_function(&mut agent, &realm, "interruptNow", 0, interrupt_now, false);
    assert_eq!(
        eval_text(&mut agent, &realm, CLOSING),
        Outcome::Normal("true".to_string())
    );
    for (source, thrown) in [
        (
            "Array.from(closing([1, 2]), function (v) { if (v === 2) throw \"map\"; });",
            "map",
        ),
        ("Array.from.call(Frozen, closing([1]));", "TypeError"),
        ("Object.fromEntries(closing([1]));", "TypeError"),
        (
            "Object.fromEntries(closing([{ get 0() { throw \"zero\"; } }]));",
            "zero",
        ),
        (
            "Object.fromEntries(closing([{ 0: \"k\", get 1() { throw \"one\"; } }]));",
            "one",
        ),
        (
            "Object.fromEntries(closing([[badKey(\"key\"), 1]]));",
            "key",
        ),
        (
            "Object.groupBy(closing([1]), function () { throw \"callback\"; });",
            "callback",
        ),
        (
            "Object.groupBy(closing([1]), function () { return badKey(\"group\"); });",
            "group",
        ),
    ] {
        let before = eval_text(&mut agent, &realm, "returns");
        let Outcome::Normal(before) = before else {
            panic!("returns: {before:?}");
        };
        let before: u32 = before.parse().unwrap();
        assert_eq!(
            eval_text(
                &mut agent,
                &realm,
                &format!("caught(function () {{ {source} }}) + \";\" + returns")
            ),
            Outcome::Normal(format!("\"{thrown};{}\"", before + 1)),
            "{source}"
        );
    }
    let Outcome::Normal(closed) = eval_text(&mut agent, &realm, "returns") else {
        panic!("returns did not answer");
    };
    for source in [
        "Array.from(closing([1, 2]), function () { interruptNow(); });",
        "Object.groupBy(closing([1, 2]), function () { interruptNow(); return 1; });",
        "Object.fromEntries(closing([[{ toString: function () { interruptNow(); return \"k\"; } }, 1]]));",
    ] {
        HANDLE.with(|slot| slot.set(Some(agent.interrupt_handle())));
        assert_eq!(
            eval_text(&mut agent, &realm, source),
            Outcome::Terminated(Termination::Interrupted),
            "{source}"
        );
        assert_eq!(
            eval_text(&mut agent, &realm, "returns"),
            Outcome::Normal(closed.clone()),
            "{source}"
        );
        assert_eq!(
            eval_text(&mut agent, &realm, "1"),
            Outcome::Normal("1".to_string()),
            "after {source}"
        );
    }
}
