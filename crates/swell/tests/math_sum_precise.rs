//! `Math.sumPrecise ( items )` (ECMA-262 21.3.2.34, `#sec-math.sumprecise`,
//! `story:math-sum-precise`): the function's properties, the iteration of decision 3, "Iterators
//! held by a builtin" (the record kept from GetIterator to every way out, a step per item beside
//! the step of the call of `next`, IteratorClose for an item that is no Number and nothing closed
//! for a throw of `next` or a termination), and the design's charge (decision 6,
//! "`Math.sumPrecise`": 34 steps once at the end, nothing on the heap). The exact sum itself is
//! held by `crates/num/tests/sum.rs`. Run also with the feature `collect-every-safe-point`,
//! where a record held without a root is freed by the collection in `next` and read back wrong.
//!
//! No script uses `for`-`of`, a spread or an array pattern: an iterable is an Array, a String, a
//! generator or an object whose `[Symbol.iterator]` the script writes.
#![cfg(test)]

use crate::support;

use std::cell::Cell;

use support::host::{agent, agent_with, limits, realm};
use support::script::{Outcome, define_function, describe, eval, eval_text, parse, run_both};
use swell::{
    Completion, Cx, HostCall, InterruptHandle, Outcome as HostOutcome, Termination, Value,
};

/// Whether a test takes its gate count: with the feature `collect-every-safe-point`, where every
/// instruction and every call of a builtin collects, unless `SWELL_FULL_COUNTS=1` (the lane
/// `slow`) asks for the full one.
fn gate_count() -> bool {
    cfg!(feature = "collect-every-safe-point")
        && !std::env::var("SWELL_FULL_COUNTS").is_ok_and(|value| value == "1")
}

/// `full` in the default mode; with a collection at every safe point `collect_full` at the full
/// count and `collect_gate` at the gate count.
fn count(full: u32, collect_full: u32, collect_gate: u32) -> u32 {
    if gate_count() {
        collect_gate
    } else if cfg!(feature = "collect-every-safe-point") {
        collect_full
    } else {
        full
    }
}

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

/// The iterables the scripts write: `make(next, ret)` is an object whose `[Symbol.iterator]`
/// answers an iterator with `next` and, when `ret` is given, `return`; `list(values)` is a
/// `next` that answers the elements of `values`, then done, and counts its calls in `nexts`;
/// `count` is a `return` that counts its calls in `returns`; `caught(f)` answers what `f` threw,
/// the name of an error or the value, or "none".
const ITERABLES: &str = "var nexts = 0, returns = 0;\n\
     function make(next, ret) {\n\
       var o = {};\n\
       o[Symbol.iterator] = function () {\n\
         var it = { next: next };\n\
         if (ret !== undefined) it[\"return\"] = ret;\n\
         return it;\n\
       };\n\
       return o;\n\
     }\n\
     function list(values) {\n\
       var i = 0;\n\
       return function () { nexts++; i++; return { value: values[i - 1], done: i > values.length }; };\n\
     }\n\
     function count() { returns++; return {}; }\n\
     function caught(f) {\n\
       try { f(); return \"none\"; } catch (e) { return e instanceof Error ? e.name : e; }\n\
     }\n";

/// `#sec-function-properties-of-the-math-object` and 21.3.2.34: `Math.sumPrecise` is
/// { [[Writable]]: true, [[Enumerable]]: false, [[Configurable]]: true }, its "length" is 1 and
/// its "name" "sumPrecise", its prototype `%Function.prototype%`, and it is no constructor.
#[test]
fn the_function_has_its_length_name_and_descriptor() {
    answers(
        "var d = Object.getOwnPropertyDescriptor(Math, \"sumPrecise\");\n\
         [typeof d.value, d.writable, d.enumerable, d.configurable, d.value.length, d.value.name,\n\
          Object.getPrototypeOf(d.value) === Function.prototype].join();",
        "\"function,true,false,true,1,sumPrecise,true\"",
    );
    throws("new Math.sumPrecise([]);", "TypeError");
}

/// An Array, a generator and an iterator object the script writes give the same sum of the same
/// items, the design's example whose exact sum passes 2^1,025 on the way and is
/// 0.30000000000000004; the iterator object's `next` is called once per item and once more for
/// done, and `return` is not called. No item and a String of no code unit give -0, a +0 item
/// gives +0.
#[test]
fn an_array_a_generator_and_an_iterator_give_the_same_sum() {
    answers(
        &[
            ITERABLES,
            "var a = [1e308, 1e308, 0.1, 0.1, 0.1, -1e308, -1e308];\n\
             function* gen(values) { for (var i = 0; i < values.length; i++) yield values[i]; }\n\
             var r = [Math.sumPrecise(a), Math.sumPrecise(gen(a)), Math.sumPrecise(make(list(a), count)), nexts, returns];\n\
             r.push(Math.sumPrecise([1, 2, 3]), Object.is(Math.sumPrecise([]), -0), Object.is(Math.sumPrecise(\"\"), -0));\n\
             r.push(Object.is(Math.sumPrecise([-0, 0]), 0), Object.is(Math.sumPrecise([-0, -0]), -0));\n\
             r.join();",
        ]
        .concat(),
        "\"0.30000000000000004,0.30000000000000004,0.30000000000000004,8,0,6,true,true,true,true\"",
    );
}

/// Steps 1 and 2: undefined and null fail RequireObjectCoercible, and a Number, a plain object and
/// an object whose `[Symbol.iterator]` answers no object fail GetIterator, each with a `TypeError`.
#[test]
fn a_value_that_is_not_iterable_throws_a_type_error() {
    for source in [
        "Math.sumPrecise();",
        "Math.sumPrecise(undefined);",
        "Math.sumPrecise(null);",
        "Math.sumPrecise(1, 2);",
        "Math.sumPrecise({});",
        "var o = {}; o[Symbol.iterator] = function () { return 1; }; Math.sumPrecise(o);",
    ] {
        throws(source, "TypeError");
    }
}

/// Step 7.b.ii: an item that is no Number (a String, a Symbol, a Number object) is a `TypeError`
/// after IteratorClose, which calls `return` once and reads no item after it; a `return` that
/// throws does not replace the `TypeError` (`#sec-iteratorclose`, step 5). A throw of `next`, of
/// reading `done` or `value`, and a result of `next` that is no object close nothing
/// (IteratorStepValue, `#sec-iteratorstepvalue`).
#[test]
fn an_item_that_is_no_number_closes_once_and_a_throw_of_next_closes_nothing() {
    answers(
        &[
            ITERABLES,
            "var r = [];\n\
             r.push(caught(function () { Math.sumPrecise(make(list([1, \"2\", 3]), count)); }), returns, nexts);\n\
             r.push(caught(function () { Math.sumPrecise(make(list([Symbol()]), count)); }), returns, nexts);\n\
             r.push(caught(function () { Math.sumPrecise(make(list([1, new Number(2)]), count)); }), returns, nexts);\n\
             r.push(caught(function () { Math.sumPrecise(make(list([\"x\"]), function () { returns++; throw \"return\"; })); }), returns);\n\
             r.push(caught(function () { Math.sumPrecise(make(function () { throw \"next\"; }, count)); }), returns);\n\
             r.push(caught(function () { Math.sumPrecise(make(function () { return { get done() { throw \"done\"; } }; }, count)); }), returns);\n\
             r.push(caught(function () { Math.sumPrecise(make(function () { return { done: false, get value() { throw \"value\"; } }; }, count)); }), returns);\n\
             r.push(caught(function () { Math.sumPrecise(make(function () { return 1; }, count)); }), returns);\n\
             r.join();",
        ]
        .concat(),
        "\"TypeError,1,2,TypeError,2,3,TypeError,3,5,TypeError,4,next,4,done,4,value,4,TypeError,4\"",
    );
}

/// The endless iterable of [`an_endless_iterator_ends_at_the_step_budget_and_calls_no_return`]:
/// `endless(n)` answers 1 `n` times (`Infinity` for no end), and its `return` counts its calls in
/// `returns`.
const ENDLESS: &str = "var returns = 0;\n\
     function endless(n) {\n\
       var o = {};\n\
       o[Symbol.iterator] = function () {\n\
         var i = 0;\n\
         return {\n\
           next: function () { i++; return i > n ? { done: true } : { value: 1, done: false }; },\n\
           \"return\": function () { returns++; return {}; }\n\
         };\n\
       };\n\
       return o;\n\
     }\n\
     true;";

/// The step room of [`an_endless_iterator_ends_at_the_step_budget_and_calls_no_return`] above the
/// steps of the control: 1,000; 100 with the feature `collect-every-safe-point`, where every
/// instruction collects.
const STEP_ROOM: u64 = if cfg!(feature = "collect-every-safe-point") {
    100
} else {
    1_000
};

/// `docs/design/engine.md`, decision 6, "`Math.sumPrecise`", "An iterable that ends in a
/// budget": over an iterator that is never done `Math.sumPrecise` ends in
/// `Terminated(StepBudget)`, with no sum and no call of `return`. The budget is the steps of the
/// control (`Math.sumPrecise(endless(3))`) plus [`STEP_ROOM`]. After the termination the agent
/// runs the next entry (HOST-70), and the control answers 3 again.
#[test]
fn an_endless_iterator_ends_at_the_step_budget_and_calls_no_return() {
    let control = "Math.sumPrecise(endless(3));";
    let mut measure = agent().unwrap();
    let measured = realm(&mut measure);
    assert_eq!(
        eval_text(&mut measure, &measured, ENDLESS),
        Outcome::Normal("true".to_string())
    );
    let outcome = eval(&mut measure, &measured, control);
    let steps = measure.last_entry_steps().unwrap();
    assert_eq!(
        describe(&mut measure, &measured, outcome),
        Outcome::Normal("3".to_string())
    );
    println!("control: {steps} steps");

    let mut budgeted = limits();
    budgeted.steps_per_entry = steps + STEP_ROOM;
    let mut agent = agent_with(budgeted).unwrap();
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(&mut agent, &realm, ENDLESS),
        Outcome::Normal("true".to_string())
    );
    assert_eq!(
        eval_text(&mut agent, &realm, "Math.sumPrecise(endless(Infinity));"),
        Outcome::Terminated(Termination::StepBudget)
    );
    assert_eq!(
        eval_text(&mut agent, &realm, "returns"),
        Outcome::Normal("0".to_string())
    );
    assert_eq!(
        eval_text(&mut agent, &realm, "1"),
        Outcome::Normal("1".to_string())
    );
    assert_eq!(
        eval_text(&mut agent, &realm, control),
        Outcome::Normal("3".to_string())
    );
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

/// An interrupt that a host function raises from inside `next`, at the second item, ends the
/// entry in `Terminated(Interrupted)` with no call of `return` (decision 3, item 4); the agent
/// runs the next entry.
#[test]
fn an_interrupt_raised_in_next_ends_the_call_and_calls_no_return() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    define_function(&mut agent, &realm, "interruptNow", 0, interrupt_now, false);
    assert_eq!(
        eval_text(&mut agent, &realm, &[ITERABLES, "true;"].concat()),
        Outcome::Normal("true".to_string())
    );
    HANDLE.with(|slot| slot.set(Some(agent.interrupt_handle())));
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var i = 0;\n\
             Math.sumPrecise(make(function () { i++; if (i === 2) interruptNow(); return { value: i, done: i > 5 }; }, count));"
        ),
        Outcome::Terminated(Termination::Interrupted)
    );
    assert_eq!(
        eval_text(&mut agent, &realm, "[i, returns].join();"),
        Outcome::Normal("\"2,0\"".to_string())
    );
    assert_eq!(
        eval_text(&mut agent, &realm, "1"),
        Outcome::Normal("1".to_string())
    );
}

/// The steps of the entry `Math.sumPrecise(it)`, with `it` an Array Iterator over an Array of
/// `n` ones made by an earlier entry, and the sum it answers.
fn steps_of_a_sum_over_an_array_iterator(n: u32) -> (u64, Outcome) {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let setup = format!("var a = new Array({n}).fill(1), it = a.values(); a.length");
    assert_eq!(
        eval_text(&mut agent, &realm, &setup),
        Outcome::Normal(n.to_string())
    );
    // Read right after `evaluate_script`: reading the answer is an entry of its own.
    let outcome = eval(&mut agent, &realm, "Math.sumPrecise(it);");
    let steps = agent.last_entry_steps().unwrap();
    (steps, describe(&mut agent, &realm, outcome))
}

/// Decision 3, items 2 and 3, and decision 6, "`Math.sumPrecise`": each item costs the step of
/// the call of `next` (here `%ArrayIteratorPrototype%.next`, a builtin that runs no instruction)
/// and the loop's own step, so 40,000 items cost 2 × 40,000 steps more than no item; the 34 steps
/// of the end and the call of `next` that answers done are in both. At the gate count of the
/// feature `collect-every-safe-point` the items are 100 (1,000 at its full count), since each
/// call of `next` collects a heap that holds the Array.
#[test]
fn each_item_costs_a_step_beside_the_call_of_next() {
    let n = count(40_000, 1_000, 100);
    let (many, sum) = steps_of_a_sum_over_an_array_iterator(n);
    assert_eq!(sum, Outcome::Normal(n.to_string()));
    let (none, sum) = steps_of_a_sum_over_an_array_iterator(0);
    // -0, which `show` writes as ToString does.
    assert_eq!(sum, Outcome::Normal("0".to_string()));
    println!("{many} steps for {n} items, {none} for none");
    assert_eq!(
        many - none,
        2 * u64::from(n),
        "{many} steps for {n} items, {none} for none"
    );
}

/// The iterables of [`a_call_over_many_items_leaves_the_heap_where_it_was`]: the `next` of each
/// iterator is `Object.prototype.valueOf`, a builtin that runs no instruction and answers its
/// this value, the iterator itself, so a call of `next` makes nothing; `endless` answers the
/// Number 1 without end, `empty` is done at once.
const VALUE_OF_ITERABLES: &str = "function iterable(iterator) {\n\
       var o = {};\n\
       o[Symbol.iterator] = function () { return iterator; };\n\
       return o;\n\
     }\n\
     var endless = iterable({ value: 1, done: false, next: Object.prototype.valueOf });\n\
     var empty = iterable({ done: true, next: Object.prototype.valueOf });\n\
     true;";

/// Decision 6, "`Math.sumPrecise`": the accumulators are on the frame of the call, so a call over
/// 100,000 items leaves `heap_bytes()` where it was. The call sums `endless` under a step budget
/// of the steps of `Math.sumPrecise(empty)` plus 2 × 100,000: two steps per item (as
/// [`each_item_costs_a_step_beside_the_call_of_next`] shows), so it ends in
/// `Terminated(StepBudget)` after 100,017 items (the call of `next` that answers done and the 34
/// steps of the end of the empty call are 35 steps more). Read on a second round: the call, a
/// collection, the reading, the same call again and the reading after it. At the gate count of
/// the feature `collect-every-safe-point` the items are 100 (1,000 at its full count).
#[test]
fn a_call_over_many_items_leaves_the_heap_where_it_was() {
    let n = count(100_000, 1_000, 100);
    let mut measure = agent().unwrap();
    let measured = realm(&mut measure);
    assert_eq!(
        eval_text(&mut measure, &measured, VALUE_OF_ITERABLES),
        Outcome::Normal("true".to_string())
    );
    let outcome = eval(&mut measure, &measured, "Math.sumPrecise(empty);");
    let none = measure.last_entry_steps().unwrap();
    // -0, which `show` writes as ToString does.
    assert_eq!(
        describe(&mut measure, &measured, outcome),
        Outcome::Normal("0".to_string())
    );

    let mut budgeted = limits();
    budgeted.steps_per_entry = none + 2 * u64::from(n);
    let mut agent = agent_with(budgeted).unwrap();
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(&mut agent, &realm, VALUE_OF_ITERABLES),
        Outcome::Normal("true".to_string())
    );
    // One script, evaluated in both rounds, so that the second round makes no script record.
    let call = parse(&mut agent, &realm, "Math.sumPrecise(endless);");
    let outcome = agent.evaluate_script(&call).unwrap();
    assert!(
        matches!(outcome, HostOutcome::Terminated(Termination::StepBudget)),
        "{outcome:?}"
    );
    agent.collect().unwrap();
    let before = agent.heap_bytes().unwrap();
    let outcome = agent.evaluate_script(&call).unwrap();
    let after = agent.heap_bytes().unwrap();
    assert!(
        matches!(outcome, HostOutcome::Terminated(Termination::StepBudget)),
        "{outcome:?}"
    );
    println!(
        "heap_bytes() before {before}, after {after}; a step budget of {} steps, {none} for no item",
        none + 2 * u64::from(n)
    );
    assert_eq!(after, before);
    assert_eq!(
        eval_text(&mut agent, &realm, "1"),
        Outcome::Normal("1".to_string())
    );
}
