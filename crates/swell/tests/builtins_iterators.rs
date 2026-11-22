//! The iterators of slice 6 (`story:iterator-protocol-and-array-string-iterators`):
//! `%Iterator.prototype%` with its `[Symbol.iterator]` (`#sec-%iteratorprototype%-%symbol.iterator%`),
//! the Array Iterators (`#sec-%arrayiteratorprototype%-object`, CreateArrayIterator,
//! `#sec-createarrayiterator`) that `Array.prototype.values`, `keys`, `entries` and
//! `[Symbol.iterator]` answer (`#sec-array.prototype.values`, `#sec-array.prototype.keys`,
//! `#sec-array.prototype.entries`, `#sec-array.prototype-%symbol.iterator%`), the String
//! Iterators (`#sec-%stringiteratorprototype%-object`) that `String.prototype[Symbol.iterator]`
//! answers (`#sec-string.prototype-%symbol.iterator%`), and the own `[Symbol.iterator]` of an
//! arguments object (`#sec-createmappedargumentsobject`, `#sec-createunmappedargumentsobject`).
//! The design is `docs/design/engine.md`, decision 3, "Iterators held by a builtin": an
//! iterator holds what it iterates until it is done, and every call of `next` is charged by the
//! call path.
//!
//! No test uses `for`-`of`, a spread or an array pattern (later stories), nor a computed property
//! name: a script steps an iterator by calling `next`. Scripts run in sloppy and in strict code
//! where no step differs. Run also with the feature `collect-every-safe-point`.
#![cfg(test)]

use crate::support;

use support::host::{agent, agent_with, limits, realm, stack_with_room};
use support::script::{Outcome, eval, eval_text, run, run_both};
use support::stack::{BUILTINS_REENTRY, on_bounded_stack, reach_through, stack_limits, test_path};
use swell::{Outcome as HostOutcome, Termination};

/// `source` answers the String `expected` in sloppy and in strict code.
fn answers(source: &str, expected: &str) {
    let (sloppy, strict) = run_both(source);
    let expected = Outcome::Normal(format!("\"{expected}\""));
    assert_eq!(sloppy, expected, "sloppy: {source}");
    assert_eq!(strict, expected, "strict: {source}");
}

/// The function that steps an iterator to its end by calling `next`, and answers the values it
/// gave, each as `String` writes it, joined by "/".
const ALL: &str = "function all(it) {\n\
     var out = [], step;\n\
     while (!(step = it.next()).done) out.push(String(step.value));\n\
     return out.join(\"/\");\n\
   }\n";

/// `source` after the declaration of [`ALL`].
fn with_all(source: &str) -> String {
    [ALL, source].concat()
}

/// `#sec-array.prototype.values`, `#sec-array.prototype.keys`, `#sec-array.prototype.entries`
/// and `#sec-array.prototype-%symbol.iterator%` with CreateArrayIterator
/// (`#sec-createarrayiterator`) and `%ArrayIteratorPrototype%.next`
/// (`#sec-%arrayiteratorprototype%.next`): the values, the indices and the pairs
/// `[index, value]` in index order; `Array.prototype[Symbol.iterator]` is the function object
/// `Array.prototype.values`. The length is read on every `next`: an element pushed after the
/// iteration started is seen, a length cut short ends it, and once `next` answered done it
/// answers done again, also after the array grew (its [[IteratedArrayLike]] is undefined). The
/// iterator of a String object or of an array-like object iterates its indices. The methods
/// are named, have "length" 0, and the iterator's class string is "Array Iterator"
/// (`#sec-%arrayiteratorprototype%-%symbol.tostringtag%`); `next` on an object without the
/// slots of an Array Iterator throws a `TypeError`, and so does `values` on undefined.
#[test]
fn array_values_keys_and_entries_iterate_in_order_and_see_a_length_change() {
    answers(
        &with_all(
            "var a = [\"x\", \"y\"], r = [];\n\
             r.push(all(a.values()), all(a.keys()), all(a.entries()), all(a[Symbol.iterator]()));\n\
             r.push(Array.prototype[Symbol.iterator] === Array.prototype.values);\n\
             var b = [1, 2, 3], it = b.values(), seen = [it.next().value];\n\
             b.push(4);\n\
             seen.push(it.next().value, it.next().value, it.next().value);\n\
             var end = it.next();\n\
             b.push(5);\n\
             seen.push(end.done, end.value === undefined, it.next().done);\n\
             r.push(seen.join(\"/\"));\n\
             var c = [1, 2, 3], k = c.keys(), first = k.next().value;\n\
             c.length = 1;\n\
             r.push(first, k.next().done);\n\
             r.push(all(Array.prototype.values.call(\"ab\")),\n\
                    all(Array.prototype.entries.call({ length: 2, 0: \"p\", 1: \"q\" })));\n\
             var names = [Array.prototype.values, Array.prototype.keys, Array.prototype.entries];\n\
             for (var i = 0; i < names.length; i++) r.push(names[i].name + \":\" + names[i].length);\n\
             var step = [7].values().next();\n\
             r.push(Object.keys(step).join(\"+\"), Object.prototype.toString.call(a.values()));\n\
             try { a.values().next.call({}); r.push(\"answered\"); } catch (e) { r.push(e.name); }\n\
             try { Array.prototype.values.call(undefined); r.push(\"answered\"); } catch (e) { r.push(e.name); }\n\
             r.join();",
        ),
        "x/y,0/1,0,x/1,y,x/y,true,1/2/3/4/true/true/true,0,true,a/b,0,p/1,q,\
         values:0,keys:0,entries:0,value+done,[object Array Iterator],TypeError,TypeError",
    );
}

/// `docs/design/engine.md`, decision 3, the domain relation "Array Iterator → the iterated
/// object": an Array Iterator keeps its array alive until `next` finds it done, then drops it
/// (`#sec-%arrayiteratorprototype%.next`: "Set iteratorObj.[[IteratedArrayLike]] to
/// undefined"). The array holds a String of 1,000,000 code units. With the iterator held by a
/// global variable, `heap_bytes()` after `collect()` with the iterator not yet done (it gave
/// the element and has not answered done) exceeds the reading with the iterator done by at
/// least those 1,000,000 bytes. `repeat` charges a step per code unit it copies, so the step
/// budget is 10,000,000.
#[test]
fn an_array_iterator_drops_its_array_when_done() {
    const UNITS: u64 = 1_000_000;
    let mut roomy = limits();
    roomy.steps_per_entry = 10_000_000;
    let mut agent = agent_with(roomy).unwrap();
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var it = [\"x\".repeat(1000000)].values(); it.next().value.length"
        ),
        Outcome::Normal("1000000".to_string())
    );
    agent.collect().unwrap();
    let not_done = agent.heap_bytes().unwrap();
    assert_eq!(
        eval_text(&mut agent, &realm, "it.next().done"),
        Outcome::Normal("true".to_string())
    );
    agent.collect().unwrap();
    let done = agent.heap_bytes().unwrap();
    assert!(
        not_done >= done + UNITS,
        "heap_bytes {not_done} with the iterator not yet done, {done} with it done"
    );
}

/// `#sec-%stringiteratorprototype%.next` with `String.prototype[Symbol.iterator]`
/// (`#sec-string.prototype-%symbol.iterator%`): the iterator answers each code point as a
/// String (CodePointAt, `#sec-codepointat`): a surrogate pair is one value of two code units, a
/// lone leading and a lone trailing surrogate are each a value of one; once done it answers
/// done again. The this value is RequireObjectCoercible and then ToString: undefined throws a
/// `TypeError`, an object iterates the String its `toString` answers. The method is named
/// "[Symbol.iterator]" with "length" 0; the class string of the iterator is "String Iterator"
/// (`#sec-%stringiteratorprototype%-%symbol.tostringtag%`); its `next` throws a `TypeError`
/// on an Array Iterator, and an Array Iterator's `next` on it.
#[test]
fn a_string_iterator_steps_by_code_point() {
    answers(
        &with_all(
            "var s =\"a\\uD83D\\uDE00b\\uD800c\\uDC00\", it = s[Symbol.iterator](), out = [], step;\n\
             while (!(step = it.next()).done) out.push(step.value.length + \":\" + step.value.charCodeAt(0));\n\
             var r = [out.join(\"/\"), it.next().done, it.next().value === undefined];\n\
             r.push(all(String.prototype[Symbol.iterator].call({ toString: function () { return \"hi\"; } })));\n\
             r.push(all(\"\"[Symbol.iterator]()) === \"\", all(Object(\"ok\")[Symbol.iterator]()));\n\
             var f = String.prototype[Symbol.iterator];\n\
             r.push(f.name, f.length, Object.prototype.toString.call(it));\n\
             try { f.call(undefined); r.push(\"answered\"); } catch (e) { r.push(e.name); }\n\
             try { it.next.call([].values()); r.push(\"answered\"); } catch (e) { r.push(e.name); }\n\
             try { [].values().next.call(it); r.push(\"answered\"); } catch (e) { r.push(e.name); }\n\
             r.join();",
        ),
        "1:97/2:55357/1:98/1:55296/1:99/1:56320,true,true,h/i,true,o/k,\
         [Symbol.iterator],0,[object String Iterator],TypeError,TypeError,TypeError",
    );
}

/// `#sec-createmappedargumentsobject` and `#sec-createunmappedargumentsobject`: both kinds of
/// arguments object have an own `[Symbol.iterator]` whose value is `%Array.prototype.values%`,
/// writable, not enumerable and configurable, defined after the indices and "length" and
/// before "callee". Its iterator steps the arguments; over a mapped object it reads a parameter
/// changed after the iterator was made (the index is mapped), over an unmapped one the
/// argument as it was. The String keys are as before. Sloppy code only: the strict run would
/// make the mapped function unmapped.
#[test]
fn arguments_objects_have_array_prototype_values_as_iterator() {
    let outcome = run(
        limits(),
        &format!(
            "{ALL}function mapped(a, b) {{ var it = arguments[Symbol.iterator](); a = 9; return [arguments, all(it)]; }}\n\
             function unmapped(a, b) {{ \"use strict\"; var it = arguments[Symbol.iterator](); a = 9; return [arguments, all(it)]; }}\n\
             var r = [];\n\
             var made = [mapped(1, 2), unmapped(1, 2)];\n\
             for (var i = 0; i < made.length; i++) {{\n\
               var args = made[i][0], d = Object.getOwnPropertyDescriptor(args, Symbol.iterator);\n\
               var symbols = Object.getOwnPropertySymbols(args);\n\
               r.push(made[i][1], d.value === Array.prototype.values, d.writable, d.enumerable, d.configurable,\n\
                      symbols.length, symbols[0] === Symbol.iterator, Object.getOwnPropertyNames(args).join(\"+\"));\n\
             }}\n\
             r.join();"
        ),
    );
    assert_eq!(
        outcome,
        Outcome::Normal(
            "\"9/2,true,true,false,true,1,true,0+1+length+callee,\
             1/2,true,true,false,true,1,true,0+1+length+callee\""
                .to_string()
        )
    );
}

/// `#sec-%iteratorprototype%-%symbol.iterator%`: `%Iterator.prototype%[Symbol.iterator]`
/// answers its this value as it is, an object or a primitive; an Array Iterator and a String
/// Iterator answer themselves through it. `%Iterator.prototype%` is the [[Prototype]] of
/// `%ArrayIteratorPrototype%` and of `%StringIteratorPrototype%`
/// (`#sec-%arrayiteratorprototype%-object`, `#sec-%stringiteratorprototype%-object`), and its own
/// [[Prototype]] is `%Object.prototype%`. The method is named "[Symbol.iterator]" with "length"
/// 0, writable, not enumerable and configurable. The String keys of the three prototypes are
/// none, "next" and "next".
#[test]
fn iterator_prototype_iterator_answers_this() {
    answers(
        "var arrayIt = [].values(), stringIt = \"\"[Symbol.iterator]();\n\
         var arrayProto = Object.getPrototypeOf(arrayIt), stringProto = Object.getPrototypeOf(stringIt);\n\
         var proto = Object.getPrototypeOf(arrayProto), f = proto[Symbol.iterator], o = {};\n\
         var d = Object.getOwnPropertyDescriptor(proto, Symbol.iterator);\n\
         [f.call(o) === o, f.call(1) === 1, f.call(\"s\") === \"s\", f.call(undefined) === undefined,\n\
          arrayIt[Symbol.iterator]() === arrayIt, stringIt[Symbol.iterator]() === stringIt,\n\
          Object.getPrototypeOf(stringProto) === proto, Object.getPrototypeOf(proto) === Object.prototype,\n\
          f.name, f.length, d.writable, d.enumerable, d.configurable,\n\
          Object.getOwnPropertyNames(proto).length, Object.getOwnPropertyNames(arrayProto).join(\"+\"),\n\
          Object.getOwnPropertyNames(stringProto).join(\"+\")].join();",
        "true,true,true,true,true,true,true,true,[Symbol.iterator],0,true,false,true,0,next,next",
    );
}

/// The entry that runs `calls` calls of `next` on an Array Iterator over an array of 1,000
/// elements made by an earlier entry, and the steps it spent.
fn steps_of_next_calls(calls: u32) -> u64 {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var a = new Array(1000).fill(0), it = a.values(); 1"
        ),
        Outcome::Normal("1".to_string())
    );
    // Read right after `evaluate_script`: reading the answer is an entry of its own, after
    // which `last_entry_steps()` reads that entry's steps.
    let source = format!("for (var i = 0; i < {calls}; i++) it.next(); i");
    let outcome = eval(&mut agent, &realm, &source);
    assert!(matches!(outcome, HostOutcome::Normal(_)), "{outcome:?}");
    agent.last_entry_steps().unwrap()
}

/// Whether a test takes its gate count: with the feature `collect-every-safe-point`, where
/// every instruction collects, unless `SWELL_FULL_COUNTS=1` (the lane `slow`) asks for the full
/// one.
fn gate_count() -> bool {
    cfg!(feature = "collect-every-safe-point")
        && !std::env::var("SWELL_FULL_COUNTS").is_ok_and(|value| value == "1")
}

/// `docs/design/engine.md`, decision 3, item 2: every call of `next` goes through the agent's
/// call path, which charges its step. `last_entry_steps()` of the entry that calls `next` 1,000
/// times exceeds that of the entry that calls it 10 times by at least 990. The gate count with
/// the feature `collect-every-safe-point` is 100 calls against 10, at least 90 steps more.
#[test]
fn each_next_is_charged() {
    let calls = if gate_count() { 100 } else { 1_000 };
    let many = steps_of_next_calls(calls);
    let ten = steps_of_next_calls(10);
    assert!(
        many >= ten + u64::from(calls - 10),
        "{many} steps for {calls} calls of next, {ten} for 10"
    );
}

/// The steps a run of [`an_array_iterator_over_a_growing_array_ends_at_the_step_budget`] may
/// spend: 200,000 in the default mode; with the feature `collect-every-safe-point`, where
/// every instruction collects, 20,000 at the full count and 4,000 at the gate's.
fn growing_steps() -> u64 {
    if !cfg!(feature = "collect-every-safe-point") {
        200_000
    } else if gate_count() {
        4_000
    } else {
        20_000
    }
}

/// The memory a run of [`an_array_iterator_over_a_growing_array_ends_at_the_step_budget`] is
/// given above the reading of its agent before the loop: 16 MiB, far above what the loop can
/// grow in [`growing_steps`] steps (an element of 16 bytes per iteration of at least ten steps).
const GROWING_ROOM: u64 = 16 << 20;

/// `#sec-%arrayiteratorprototype%.next` reads the length on every call, so an iterator over an
/// array that grows by an element per step never ends by itself; it ends at the step budget,
/// not the memory budget (`docs/design/engine.md`, "The step budget"). The memory budget is the
/// reading of the agent after its realm and a collection plus [`GROWING_ROOM`]; the step budget
/// is [`growing_steps`]. The loop pushes the value it read; the run ends in
/// `Terminated(StepBudget)`, and the array has grown by an element per hundred steps or more.
#[test]
fn an_array_iterator_over_a_growing_array_ends_at_the_step_budget() {
    let steps = growing_steps();
    let mut measured = agent().unwrap();
    let measured_realm = realm(&mut measured);
    measured.collect().unwrap();
    let reading = measured.heap_bytes().unwrap();
    drop(measured_realm);
    drop(measured);

    let mut budgeted = limits();
    budgeted.steps_per_entry = steps;
    budgeted.memory_bytes = reading + GROWING_ROOM;
    let mut agent = agent_with(budgeted).unwrap();
    let realm = realm(&mut agent);
    let outcome = eval(
        &mut agent,
        &realm,
        "var a = [0], it = a.values(), step;\n\
         while (!(step = it.next()).done) a.push(step.value + 1);",
    );
    assert!(
        matches!(outcome, HostOutcome::Terminated(Termination::StepBudget)),
        "{outcome:?}"
    );
    let Outcome::Normal(length) = eval_text(&mut agent, &realm, "a.length") else {
        panic!("a.length did not answer");
    };
    let length: u64 = length.parse().unwrap();
    assert!(
        length >= steps / 100,
        "the array has {length} elements after {steps} steps"
    );
}

/// `docs/design/engine.md`, "Depth bounds": `%ArrayIteratorPrototype%.next` over an array-like
/// whose element is a getter re-enters the interpreter from Rust (`Get(array, elementKey)`).
/// `f` re-enters itself through it until the re-entry limit, on seven eighths of
/// `min_stack_bytes()` (`AGENTS.md`, "Commands and the gate"), in a child process
/// (`support::stack::on_bounded_stack`).
#[test]
fn next_of_an_array_like_with_a_getter_re_enters_on_the_promised_stack() {
    fn body() {
        reach_through(
            "Array.prototype.values.call({ length: 1, get 0() { return f(); } }).next()",
            BUILTINS_REENTRY,
        );
    }
    let stack = stack_with_room(stack_limits(BUILTINS_REENTRY));
    on_bounded_stack(
        test_path!("next_of_an_array_like_with_a_getter_re_enters_on_the_promised_stack"),
        stack,
        body,
    )
    .unwrap();
}
