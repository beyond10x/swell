//! The `for`-`of` statement (`#sec-for-in-and-for-of-statements`): ForIn/OfHeadEvaluation
//! (`#sec-runtime-semantics-forinofheadevaluation`) gets the iterator of the head's value by
//! GetIterator (`#sec-getiterator`), and ForIn/OfBodyEvaluation
//! (`#sec-runtime-semantics-forin-div-ofbodyevaluation-lhs-stmt-iterator-lhskind-labelset`) runs
//! the body once per value IteratorStepValue (`#sec-iteratorstepvalue`) answers, with a new
//! binding per iteration for `let` and `const`, and closes the iterator by IteratorClose
//! (`#sec-iteratorclose`) on every exit before it is done: a `break` of the loop, a `break` or
//! `continue` to a statement around it, a `return` and a throw of the body or of the head's
//! binding (`docs/design/engine.md`, decision 3, "The iterator of `for`-`of`, of an array
//! pattern and of a spread"). Written by `story:for-of-statement`. Run also with the feature
//! `collect-every-safe-point`.
//!
//! No script uses an array pattern or a spread (`story:array-destructuring-and-spread`).
#![cfg(test)]

use crate::support;

use std::cell::Cell;

use support::host::{agent, agent_with, limits, realm, stack_with_room};
use support::script::{Outcome, define_function, eval, eval_text, run_both};
use support::stack::{BUILTINS_REENTRY, on_bounded_stack, reach_through, stack_limits, test_path};
use swell::{
    Completion, Cx, HostCall, InterruptHandle, Outcome as HostOutcome, Parsed, Source, Termination,
    Value,
};

/// `source` answers `expected` (as `support::script::show` writes it) in sloppy and in strict
/// code.
fn answers(source: &str, expected: &str) {
    let (sloppy, strict) = run_both(source);
    let expected = Outcome::Normal(expected.to_string());
    assert_eq!(sloppy, expected, "sloppy: {source}");
    assert_eq!(strict, expected, "strict: {source}");
}

/// `source` after the declarations of [`ITERABLE`].
fn with_iterable(source: &str) -> String {
    [ITERABLE, source].concat()
}

/// An iterable whose iterator answers 1 to `n`, then done, and has `ret` as its `return` when
/// it is not undefined; `counting()` is a `return` that writes "return" to `log` and answers an
/// object; `throwing(v)` one that writes "return" and throws `v`.
const ITERABLE: &str = "var log = [];\n\
     function iterable(n, ret) {\n\
       var o = {};\n\
       o[Symbol.iterator] = function () {\n\
         var i = 0;\n\
         var it = { next: function () { i++; return { value: i, done: i > n }; } };\n\
         if (ret !== undefined) it[\"return\"] = ret;\n\
         return it;\n\
       };\n\
       return o;\n\
     }\n\
     function counting() { return function () { log.push(\"return\"); return {}; }; }\n\
     function throwing(v) { return function () { log.push(\"return\"); throw v; }; }\n";

/// `#sec-runtime-semantics-forinofheadevaluation` and the iterators of slice 6: a `for`-`of`
/// over an Array steps its elements through `Array.prototype[Symbol.iterator]`, over a String
/// its code points (a surrogate pair is one value), over an arguments object its arguments;
/// the head binds with `var`, `let` and `const`, puts to a member or an index expression, and
/// destructures with an object pattern; a script's completion value is the last value of the
/// body, and `undefined` for a loop that runs no iteration (`#sec-loopcontinues`, UpdateEmpty).
#[test]
fn for_of_iterates_an_array_a_string_and_arguments() {
    for (source, expected) in [
        (
            "var out = []; for (var x of [1, 2, 3]) out.push(x); out.join()",
            "\"1,2,3\"",
        ),
        (
            "var s = \"\"; for (var c of \"a\\u{1F600}b\") s += c.length; s",
            "\"121\"",
        ),
        (
            "function f() { var r = []; for (var a of arguments) r.push(a); return r.join(); } \
             f(4, 5)",
            "\"4,5\"",
        ),
        (
            "var r = []; for (let x of [1, 2]) r.push(x); for (const y of \"ab\") r.push(y); \
             r.join()",
            "\"1,2,a,b\"",
        ),
        (
            "var o = {}, a = []; for (o.p of [6, 7]) ; for (a[1] of [8]) ; o.p + \",\" + a[1]",
            "\"7,8\"",
        ),
        (
            "var r = [], k; for (var { k: v } of [{ k: 1 }, { k: 2 }]) r.push(v); \
             for ({ k } of [{ k: 3 }]) r.push(k); for (const { k: c } of [{ k: 4 }]) r.push(c); \
             r.join()",
            "\"1,2,3,4\"",
        ),
        ("1; for (var z of [1, 2]) z * 10;", "20"),
        ("2; for (var z of []) z;", "undefined"),
        (
            "var it = [5, 6][Symbol.iterator](); it.next(); var r = []; \
             for (var x of it) r.push(x); r.join()",
            "\"6\"",
        ),
    ] {
        answers(source, expected);
    }
}

/// `#sec-runtime-semantics-forin-div-ofbodyevaluation-lhs-stmt-iterator-lhskind-labelset`,
/// steps 6.j to 6.m: IteratorClose calls `return` once on each exit before the iterator is
/// done: a `break` of the loop, a `continue` to an outer loop, a `break` to a label around it,
/// a `return` (after the `finally` block inside the loop), a throw from the body; a `break` of
/// an outer `for`-`of` from an inner one closes both iterators, the inner first.
#[test]
fn for_of_calls_return_once_on_break_continue_outer_return_and_a_throw_from_the_body() {
    for (source, expected) in [
        (
            "for (var x of iterable(3, counting())) { if (x === 2) break; } \
             log.join() + \";\" + x",
            "\"return;2\"",
        ),
        (
            "outer: for (var k = 0; k < 2; k++) { for (var x of iterable(3, counting())) \
             { continue outer; } } log.join()",
            "\"return,return\"",
        ),
        (
            "block: { for (var x of iterable(3, counting())) break block; } log.join()",
            "\"return\"",
        ),
        (
            "function f() { for (var x of iterable(3, counting())) { \
             try { return x * 10; } finally { log.push(\"finally\"); } } } \
             var r = f(); log.join() + \";\" + r",
            "\"finally,return;10\"",
        ),
        (
            "var caught; try { for (var x of iterable(3, counting())) throw x; } \
             catch (e) { caught = e; } log.join() + \";\" + caught",
            "\"return;1\"",
        ),
        (
            "outer: for (var a of iterable(3, function () { log.push(\"outer\"); return {}; })) \
             for (var b of iterable(3, function () { log.push(\"inner\"); return {}; })) \
             break outer; log.join()",
            "\"inner,outer\"",
        ),
    ] {
        answers(&with_iterable(source), expected);
    }
}

/// `#sec-iteratorstepvalue` and
/// `#sec-runtime-semantics-forin-div-ofbodyevaluation-lhs-stmt-iterator-lhskind-labelset`,
/// step 6.c: the iterator is not closed when it is done, nor when `next` throws, answers a
/// value that is not an object (a `TypeError`), or a result whose `done` or `value` getter
/// throws; each of those throws reaches the `catch` around the loop.
#[test]
fn for_of_does_not_call_return_when_next_throws_or_the_iterator_is_done() {
    for (source, expected) in [
        (
            "for (var x of iterable(2, counting())) ; log.join() + x",
            "\"2\"",
        ),
        (
            "function bad(next) { var o = {}; o[Symbol.iterator] = function () { \
             return { next: next, \"return\": counting() }; }; return o; }\n\
             var r = [];\n\
             for (var next of [function () { throw \"next\"; }, function () { return 1; }, \
             function () { return { get done() { throw \"done\"; } }; }, \
             function () { return { done: false, get value() { throw \"value\"; } }; }]) {\n\
               try { for (var x of bad(next)) r.push(\"body\"); }\n\
               catch (e) { r.push(e instanceof TypeError ? \"TypeError\" : e); }\n\
             }\n\
             r.join() + \";\" + log.length",
            "\"next,TypeError,done,value;0\"",
        ),
    ] {
        answers(&with_iterable(source), expected);
    }
}

/// `#sec-iteratorclose`, step 5: on a throw completion IteratorClose calls `return` and answers
/// the throw it was given, whatever `return` throws or answers: a `return` that throws, one
/// that answers a value that is not an object, and a `return` getter that throws each leave the
/// throw of the body.
#[test]
fn a_throw_of_return_does_not_hide_the_throw_from_the_body() {
    for (source, expected) in [
        (
            "var caught; try { for (var x of iterable(3, throwing(\"from return\"))) \
             throw \"from body\"; } catch (e) { caught = e; } caught + \";\" + log.join()",
            "\"from body;return\"",
        ),
        (
            "var caught; try { for (var x of iterable(3, function () { log.push(\"return\"); \
             return 1; })) throw \"from body\"; } catch (e) { caught = e; } \
             caught + \";\" + log.join()",
            "\"from body;return\"",
        ),
        (
            "var o = {}; o[Symbol.iterator] = function () { return { next: function () { \
             return { done: false }; }, get return() { log.push(\"get\"); throw \"getter\"; } }; };\n\
             var caught; try { for (var x of o) throw \"from body\"; } catch (e) { caught = e; } \
             caught + \";\" + log.join()",
            "\"from body;get\"",
        ),
    ] {
        answers(&with_iterable(source), expected);
    }
}

/// `#sec-runtime-semantics-forin-div-ofbodyevaluation-lhs-stmt-iterator-lhskind-labelset`,
/// step 6.g, and `#sec-forin-div-ofheadevaluation-tdznames-expr-iterationkind`
/// (ForIn/OfHeadEvaluation, step 2): `let` and `const` in the head give each iteration a binding of its own, which the
/// closures of that iteration keep, where `var` shares one binding; the head's expression sees
/// the head's names uninitialized (a `ReferenceError`), a closure made there too, and a `const`
/// of the head cannot be assigned in the body.
#[test]
fn let_in_the_head_gives_each_iteration_its_binding() {
    for (source, expected) in [
        (
            "var fs = []; for (let x of [1, 2, 3]) fs.push(function () { return x; }); \
             fs.map(function (f) { return f(); }).join()",
            "\"1,2,3\"",
        ),
        (
            "var fs = []; for (const x of [1, 2, 3]) fs.push(() => x); \
             fs.map(function (f) { return f(); }).join()",
            "\"1,2,3\"",
        ),
        (
            "var fs = []; for (var x of [1, 2, 3]) fs.push(function () { return x; }); \
             fs.map(function (f) { return f(); }).join()",
            "\"3,3,3\"",
        ),
        (
            "var fs = []; for (let { k } of [{ k: \"a\" }, { k: \"b\" }]) \
             fs.push(function () { return k; }); fs[0]() + fs[1]()",
            "\"ab\"",
        ),
        (
            "let x = \"outside\"; var probe; \
             for (let x of (probe = function () { return typeof x; }, [])) ; \
             var r; try { probe(); r = \"no throw\"; } catch (e) { r = e.name; } r + \",\" + x",
            "\"ReferenceError,outside\"",
        ),
    ] {
        answers(source, expected);
    }
    for (source, error) in [
        ("for (let x of [x]) ;", "ReferenceError"),
        ("for (const x of [1]) { x = 2; }", "TypeError"),
    ] {
        let (sloppy, strict) = run_both(source);
        assert_eq!(
            sloppy,
            Outcome::Throw(error.to_string()),
            "sloppy: {source}"
        );
        assert_eq!(
            strict,
            Outcome::Throw(error.to_string()),
            "strict: {source}"
        );
    }
}

/// Whether a test takes its gate count: with the feature `collect-every-safe-point`, where
/// every instruction collects, unless `SWELL_FULL_COUNTS=1` (the lane `slow`) asks for the full
/// one.
fn gate_count() -> bool {
    cfg!(feature = "collect-every-safe-point")
        && !std::env::var("SWELL_FULL_COUNTS").is_ok_and(|value| value == "1")
}

/// The steps of the entry that runs a `for`-`of` over the first `n` elements of an array of
/// 1,000 elements, made by an earlier entry, with an empty body.
fn steps_of_loop(n: u32) -> u64 {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let setup = format!(
        "var all = new Array(1000).fill(0), part = all.slice(0, {n}), sum = 0; part.length"
    );
    assert_eq!(
        eval_text(&mut agent, &realm, &setup),
        Outcome::Normal(n.to_string())
    );
    // Read right after `evaluate_script`: reading the answer is an entry of its own.
    let outcome = eval(&mut agent, &realm, "for (var x of part) ;");
    assert!(matches!(outcome, HostOutcome::Normal(_)), "{outcome:?}");
    agent.last_entry_steps().unwrap()
}

/// `docs/design/engine.md`, decision 3, "The iterator of `for`-`of`", item 2: each iteration
/// charges the steps of its instructions and its `LoopJump`, and the call of `next` its own.
/// `last_entry_steps()` of a loop over 1,000 elements exceeds that of a loop over 10 by at
/// least 990; the gate count with the feature `collect-every-safe-point` is 100 elements
/// against 10, at least 90 steps more.
#[test]
fn for_of_charges_each_iteration() {
    let n = if gate_count() { 100 } else { 1_000 };
    let many = steps_of_loop(n);
    let ten = steps_of_loop(10);
    println!("{many} steps for {n} elements, {ten} for 10");
    assert!(
        many >= ten + u64::from(n - 10),
        "{many} steps for {n} elements, {ten} for 10"
    );
}

/// `#sec-continue-statement-runtime-semantics-evaluation` with LoopContinues
/// (`#sec-loopcontinues`): a `continue` of the loop itself, labelled or not, goes on with the
/// next step and does not close the iterator; a `break` of it does, once.
#[test]
fn a_continue_of_the_loop_does_not_close_and_a_break_of_it_does() {
    for (source, expected) in [
        (
            "var n = 0; for (var x of iterable(3, counting())) { n++; continue; } \
             n + \";\" + log.join()",
            "\"3;\"",
        ),
        (
            "var n = 0; l: for (var x of iterable(3, counting())) { n++; continue l; } \
             n + \";\" + log.join()",
            "\"3;\"",
        ),
        (
            "var n = 0; for (var x of iterable(3, counting())) { n++; if (x < 2) continue; \
             break; } n + \";\" + log.join()",
            "\"2;return\"",
        ),
    ] {
        answers(&with_iterable(source), expected);
    }
}

/// `#sec-iteratorclose`, steps 6 and 7: on a `break`, a `continue` to an outer loop and a
/// `return`, a throw of `return` replaces the exit, which does not happen, and a `return` that
/// answers a value that is not an object throws a `TypeError`; the throw goes to the handlers
/// around the loop, never to a `catch` inside its body.
#[test]
fn a_throw_of_return_replaces_a_break_and_goes_to_the_handler_around_the_loop() {
    for (source, expected) in [
        (
            "var after = false, caught; try { for (var x of iterable(3, throwing(\"R\"))) \
             break; after = true; } catch (e) { caught = e; } caught + \",\" + after",
            "\"R,false\"",
        ),
        (
            "var inside = false, caught; try { for (var x of iterable(3, throwing(\"R\"))) \
             { try { break; } catch (e) { inside = true; } } } catch (e) { caught = e; } \
             caught + \",\" + inside",
            "\"R,false\"",
        ),
        (
            "var caught, turns = 0; try { outer: for (var k = 0; k < 2; k++) { turns++; \
             for (var x of iterable(3, throwing(\"R\"))) continue outer; } } \
             catch (e) { caught = e; } caught + \",\" + turns",
            "\"R,1\"",
        ),
        (
            "function f() { for (var x of iterable(3, throwing(\"R\"))) return \"returned\"; } \
             var caught; try { caught = f(); } catch (e) { caught = e; } caught",
            "\"R\"",
        ),
        (
            "var caught; try { for (var x of iterable(3, function () { return 0; })) break; } \
             catch (e) { caught = e instanceof TypeError; } caught",
            "true",
        ),
    ] {
        answers(&with_iterable(source), expected);
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

/// `docs/design/engine.md`, decision 3, "The iterator of `for`-`of`", item 4: a termination
/// enters no handler and runs no exit code, so an interrupt the body sets ends the run in
/// `Terminated(Interrupted)` and `return` of the iterator is not called; the agent runs the
/// next entry (HOST-70).
#[test]
fn a_termination_in_the_body_calls_no_return() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    define_function(&mut agent, &realm, "interruptNow", 0, interrupt_now, false);
    HANDLE.with(|slot| slot.set(Some(agent.interrupt_handle())));
    let source = with_iterable(
        "var turns = 0; for (var x of iterable(5, counting())) { turns++; interruptNow(); }",
    );
    assert_eq!(
        eval_text(&mut agent, &realm, &source),
        Outcome::Terminated(Termination::Interrupted)
    );
    assert_eq!(
        eval_text(&mut agent, &realm, "turns + \";\" + log.length"),
        Outcome::Normal("\"1;0\"".to_string())
    );
}

/// The iterable of [`a_loop_ends_at_the_step_and_the_memory_budget_in_the_body_and_in_next`]:
/// `endless(n, work)` answers `n` values (`Infinity` for no end), each after `work()`, and its
/// `return` counts its calls in `returns`.
const ENDLESS: &str = "var returns = 0;\n\
     function endless(n, work) {\n\
       var o = {};\n\
       o[Symbol.iterator] = function () {\n\
         var i = 0;\n\
         return {\n\
           next: function () { work(); i++; return { value: i, done: i > n }; },\n\
           \"return\": function () { returns++; return {}; }\n\
         };\n\
       };\n\
       return o;\n\
     }\n\
     function nothing() {}\n\
     true;";

/// The String the loops of [`endless_loop`] that grow copy, a flat String of 1,024 code units,
/// made by an entry of its own: its 1,024 steps of copying would not fit the step budget of the
/// loops that end at the step budget.
const CHUNK: &str = "var chunk = \"x\".repeat(1024); true;";

/// A loop over `endless(n, next)` with `body`, in a function so that what it made is garbage
/// once the run has ended; it answers the length of its Array `a`, 0 when nothing grew it.
fn budget_loop(n: &str, next: &str, body: &str) -> String {
    format!(
        "(function () {{ var a = []; for (var x of endless({n}, {next})) {{ {body} }} \
         return a.length; }})();"
    )
}

/// The four loops that end at a budget: the work in the body or in `next`, an empty loop that
/// steps forever or a new flat String of 65,536 code units (128 KiB) kept each time, over an
/// iterator that is never done. A String made by `+` is a rope of its two parts and would not
/// grow the heap by its length.
fn endless_loop(work_in_next: bool, grows: bool) -> String {
    let work = if grows {
        "a.push(chunk.repeat(64));"
    } else {
        "while (true) {}"
    };
    if work_in_next {
        budget_loop("Infinity", &format!("function () {{ {work} }}"), "")
    } else {
        budget_loop("Infinity", "nothing", work)
    }
}

/// The step room of [`a_loop_ends_at_the_step_and_the_memory_budget_in_the_body_and_in_next`]
/// above the control loop's steps: 1,000; 100 with the feature `collect-every-safe-point`,
/// where every instruction collects.
const STEP_ROOM: u64 = if cfg!(feature = "collect-every-safe-point") {
    100
} else {
    1_000
};

/// The memory room above the reading of an agent that ran the control loops and collected:
/// 1 MiB, which the Strings of [`endless_loop`] pass after about 8 iterations, about 524,000
/// steps of copying, within the step budget of `limits()`.
const MEMORY_ROOM: u64 = 1 << 20;

/// `docs/design/engine.md`, decision 3, "Iterators held by a builtin", item 2, and "The iterator
/// of `for`-`of`", items 2 and 4: a loop over an iterator that is never done ends in
/// `Terminated(StepBudget)` when the body or `next` runs forever, and in
/// `Terminated(MemoryBudget)` when the body or `next` keeps a new String each time, never in a
/// hang; no termination calls `return`. The step budget is the reading of the control loop (3
/// iterations of an empty body) plus [`STEP_ROOM`], the memory budget the heap after the control
/// loops and a collection plus [`MEMORY_ROOM`]; the control loop completes under each. After each
/// termination the agent runs again: after the memory budget a collection frees what the loop
/// made, and the script `1` answers `Normal(1)` (HOST-70, HOST-190).
#[test]
fn a_loop_ends_at_the_step_and_the_memory_budget_in_the_body_and_in_next() {
    let control = budget_loop("3", "nothing", "");
    let mut measure = agent().unwrap();
    let measured = realm(&mut measure);
    for setup in [ENDLESS, CHUNK] {
        assert_eq!(
            eval_text(&mut measure, &measured, setup),
            Outcome::Normal("true".to_string())
        );
    }
    let outcome = eval(&mut measure, &measured, &control);
    assert!(matches!(outcome, HostOutcome::Normal(_)), "{outcome:?}");
    let steps = measure.last_entry_steps().unwrap();
    measure.collect().unwrap();
    let heap = measure.heap_bytes().unwrap();
    println!("control: {steps} steps, heap {heap} bytes after a collection");

    for (grows, termination) in [
        (false, Termination::StepBudget),
        (true, Termination::MemoryBudget),
    ] {
        let mut budgeted = limits();
        if grows {
            budgeted.memory_bytes = heap + MEMORY_ROOM;
        } else {
            budgeted.steps_per_entry = steps + STEP_ROOM;
        }
        let mut agent = agent_with(budgeted).unwrap();
        let realm = realm(&mut agent);
        let setups: &[&str] = if grows { &[ENDLESS, CHUNK] } else { &[ENDLESS] };
        for setup in setups {
            assert_eq!(
                eval_text(&mut agent, &realm, setup),
                Outcome::Normal("true".to_string())
            );
        }
        for work_in_next in [false, true] {
            let source = endless_loop(work_in_next, grows);
            assert_eq!(
                eval_text(&mut agent, &realm, &source),
                Outcome::Terminated(termination),
                "{source}"
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
                eval_text(&mut agent, &realm, &control),
                Outcome::Normal("0".to_string()),
                "the control after {source}"
            );
        }
    }
}

/// The step jumps past the loop once the iterator is done
/// (`#sec-runtime-semantics-forin-div-ofbodyevaluation-lhs-stmt-iterator-lhskind-labelset`,
/// step 6.c): the statement after a loop over `[1, 2]` runs once, after both iterations, at
/// the top level and in a function, so the target of the step is the instruction after the
/// loop and not the label the compiler gave it.
#[test]
fn the_step_jumps_past_the_loop_when_the_iterator_is_done() {
    for (source, expected) in [
        (
            "var r = 0; for (var x of [1, 2]) r += x; r = r * 10; r",
            "30",
        ),
        (
            "function f() { var r = \"\"; for (let x of [1, 2]) { r += x; } r += \"!\"; \
             return r; } f() + f()",
            "\"12!12!\"",
        ),
    ] {
        answers(source, expected);
    }
}

/// HOST-173 (decision 13): the `TypeError` of a value that is not iterable is sited at the
/// head's expression, line 3, column 3; a throw of `next` in `next`, line 2, column 32, where
/// its `throw` starts; a throw of the body keeps its own site, line 4, column 20, also when
/// `return` throws while the iterator is closed.
#[test]
fn a_throw_of_get_iterator_and_of_next_is_sited_there() {
    let cases = [
        ("var a = 1;\nfor (var x of\n  a) ;", (3, 3)),
        (
            "var o = {}; o[Symbol.iterator] = function () {\n  return { next: function () { throw 1; } }; };\n\
             for (var x of o) ;",
            (2, 32),
        ),
        (
            "var o = {}; o[Symbol.iterator] = function () { return {\n  next: function () { \
             return { done: false }; },\n  \"return\": function () { throw 2; } }; };\n  \
             for (var x of o) throw 3;",
            (4, 20),
        ),
    ];
    for (source, at) in cases {
        let mut agent = agent().unwrap();
        let realm = realm(&mut agent);
        let Parsed::Parsed(script) = agent
            .parse_script(&realm, Source::Utf8(source), None)
            .unwrap()
        else {
            panic!("{source:?} did not parse");
        };
        assert!(
            matches!(agent.evaluate_script(&script), Ok(HostOutcome::Throw(_))),
            "{source:?}"
        );
        let site = agent.last_entry_error_site().unwrap().unwrap();
        assert_eq!((site.line, site.column), at, "{source:?}");
    }
}

/// `Limits::min_stack_bytes` through a `for`-`of` (decision 3, "Depth bounds"): the step calls
/// `next` from the run loop's instruction, a re-entry, so `f` re-enters itself through a
/// user iterator's `next` until the re-entry limit, on seven eighths of `min_stack_bytes()`
/// (`AGENTS.md`, "Commands and the gate"), in a child process. In the collect mode the depth is
/// [`BUILTINS_REENTRY`].
#[test]
fn a_user_iterator_re_enters_on_the_promised_stack() {
    fn body() {
        reach_through(
            "var o = {}; o[Symbol.iterator] = function () { return { next: function () { \
             f(); return { done: true }; } }; }; for (var x of o) ;",
            BUILTINS_REENTRY,
        );
    }
    let stack = stack_with_room(stack_limits(BUILTINS_REENTRY));
    on_bounded_stack(
        test_path!("a_user_iterator_re_enters_on_the_promised_stack"),
        stack,
        body,
    )
    .unwrap();
}

/// A ledger with no bound that reads what it holds and its high-water mark.
#[derive(Default)]
struct HighWater {
    held: usize,
    high: usize,
}

impl swell_syntax::Ledger for HighWater {
    fn charge(&mut self, bytes: usize) -> Result<(), swell_syntax::Stop> {
        self.held += bytes;
        self.high = self.high.max(self.held);
        Ok(())
    }

    fn credit(&mut self, bytes: usize) {
        self.held -= bytes;
    }

    fn poll(&mut self) -> Result<(), swell_syntax::Stop> {
        Ok(())
    }
}

/// The frontend's ledger over the parse and compile of `text`: when the parse returns the
/// ledger holds exactly what the tree holds by capacity, and while the compile step runs its
/// high-water mark covers the tree and the code, after which it holds exactly both.
fn charged_as_read(text: &str) {
    let units: Vec<u16> = text.encode_utf16().collect();
    let mut ledger = HighWater::default();
    let options = swell_syntax::ParseOptions { max_nesting: 100 };
    let Ok(tree) = swell_syntax::parse_script_with(&units, options, &mut ledger) else {
        panic!("the source did not parse");
    };
    assert_eq!(ledger.held, tree.capacity_bytes(), "held after the parse");
    assert_eq!(tree.charged, tree.capacity_bytes());
    ledger.high = ledger.held;
    let Ok(code) = swell_compiler::compile_script_with(&tree, &mut ledger) else {
        panic!("the source did not compile");
    };
    let held = tree.charged + code.charged;
    println!(
        "tree {} code {} high-water {}",
        tree.charged, code.charged, ledger.high
    );
    assert_eq!(code.charged, code.capacity_bytes());
    assert!(
        ledger.high >= held,
        "high-water {} below {held}",
        ledger.high
    );
    assert_eq!(ledger.held, held, "the compile step credited its scratch");
}

/// The statements of [`many_for_of_statements_are_charged_as_they_are_read`]: 10,000, the full
/// count, in the default mode and in the lane `slow` (`SWELL_FULL_COUNTS=1`); 1,000 in the gate
/// with the feature `collect-every-safe-point`, whose build is slower. The test runs no script,
/// so the feature changes nothing it checks.
fn statements() -> usize {
    if gate_count() { 1_000 } else { 10_000 }
}

/// The frontend's ledger over a script of [`statements`] `for`-`of` statements in a row inside
/// a labelled loop, a third each: with `var`; with `let` and a closure that captures its
/// binding (a record per iteration); with `const` and a body that does `break` and `continue`
/// to the label (the exits through each close). The tree, the early errors' frames, the scope
/// records of the resolver, the control entries and the handlers of the closes are charged as
/// they grow. One script of the three kinds: three scripts of 10,000 each took 2.4 s of
/// processor time.
#[test]
fn many_for_of_statements_are_charged_as_they_are_read() {
    let statements: Vec<String> = (0..statements())
        .map(|i| match i % 3 {
            0 => format!("for (var x{i} of a) ;"),
            1 => format!("for (let x{i} of a) f(() => x{i});"),
            _ => format!("for (const x{i} of a) {{ if (x{i}) break l; continue l; }}"),
        })
        .collect();
    charged_as_read(&format!("l: for (;;) {{\n{}\n}}", statements.join("\n")));
}
