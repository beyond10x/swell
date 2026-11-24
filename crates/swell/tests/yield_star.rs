//! `yield*` (YieldExpression : `yield *` AssignmentExpression,
//! `#sec-generator-function-definitions-runtime-semantics-evaluation`): the delegation runs in
//! the loop of the delegating frame (`docs/design/engine.md`, decision 3, "Suspended frames",
//! item 6): a generator delegated to is resumed by an instruction of that frame and runs above
//! it on the same run loop, so a chain of n generators holds n frames, counted against
//! `frame_depth`, and takes no Rust frame and no re-entry per level; any other iterator is
//! called as the step of a `for`-`of` calls `next`, a re-entry. Written by
//! `story:yield-star-delegation`. Run also with the feature `collect-every-safe-point`.
//!
//! Each script compares its results with `===` and answers `true`.
#![cfg(test)]

use crate::support;

use support::host::{
    TestHost, agent, agent_with, global, key, limits, object, realm, stack_with_room,
};
use support::script::{Outcome, define_function, describe, eval_text, parse, run_both};
use support::stack::{BUILTINS_REENTRY, on_bounded_stack, reach_through, stack_limits, test_path};
use swell::{Agent, Completion, Cx, HostCall, InterruptHandle, Realm, Termination, Value};

/// The outcome of a script that answers `true`.
fn passed() -> Outcome {
    Outcome::Normal("true".to_string())
}

/// `source` answers `true` in sloppy and in strict mode.
fn both_pass(source: &str) {
    assert_eq!(run_both(source), (passed(), passed()), "{source}");
}

/// `0` as an outcome: what a script that ends with `0;` answers.
fn zero() -> Outcome {
    Outcome::Normal("0".to_string())
}

/// The generator functions of a chain: `leaf` yields 1 for ever, `level` delegates to the
/// iterator it is given, and `chain(n)` is `n` levels over a leaf.
const CHAIN: &str = "function* leaf() { while (true) yield 1; }\n\
     function* level(inner) { yield* inner; }\n\
     function chain(n) { var it = leaf(); for (var i = 0; i < n; i++) it = level(it); return it; }\n";

/// Item 6: per `next`, each level of a chain spends at least 2 and at most 10 steps, so a `next`
/// through 101 levels spends between 200 and 1,000 steps more than one through 1 level. Both
/// chains have run to their first `yield` before, and each `next` is an entry of its own, read
/// with `last_entry_steps()`.
#[test]
fn a_next_through_101_levels_spends_200_to_1000_steps_more_than_through_1_level() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let setup =
        format!("{CHAIN}var one = chain(1), many = chain(101); one.next(); many.next(); 0;");
    assert_eq!(eval_text(&mut agent, &realm, &setup), zero());
    // The reading comes before `describe`, whose `with_cx` is an entry of its own.
    let mut steps_of = |source: &str| {
        let script = parse(&mut agent, &realm, source);
        let outcome = agent.evaluate_script(&script).unwrap();
        let steps = agent.last_entry_steps().unwrap();
        assert_eq!(describe(&mut agent, &realm, outcome), zero(), "{source}");
        steps
    };
    let through_one = steps_of("one.next(); 0;");
    let through_many = steps_of("many.next(); 0;");
    let more = through_many.saturating_sub(through_one);
    println!(
        "last_entry_steps: {through_one} for a next through 1 level, {through_many} through 101 \
         levels, {more} more"
    );
    assert!(
        (200..=1_000).contains(&more),
        "{through_many} against {through_one}"
    );
}

/// Item 4 and item 6: with `frame_depth` 20 a chain of 30 levels made by the script's own
/// frame reaches `frame_depth` at the `yield*` that would push the 20th frame (the script's,
/// and the levels 0 to 18): the `RangeError` is thrown there, in level 18, and leaves through
/// the levels below it, which complete; level 19, whose resumption was refused, stays
/// suspended at its start, and once the stack is lower its `next` runs the rest of the chain.
#[test]
fn a_chain_reaches_frame_depth_at_the_yield_star_that_would_push() {
    let mut limits = limits();
    limits.frame_depth = 20;
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var gens = [], thrownAt = [];\n\
             function* level(i) { try { yield* gens[i + 1]; } catch (e) { thrownAt.push(i); throw e; } }\n\
             function* leaf() { yield \"leaf\"; }\n\
             for (var i = 0; i < 30; i++) gens.push(level(i));\n\
             gens.push(leaf());\n\
             var caught;\n\
             try { gens[0].next(); } catch (e) { caught = e; }\n\
             var rest = gens[19].next();\n\
             caught instanceof RangeError && thrownAt[0] === 18 && thrownAt.length === 19 &&\n\
             gens[18].next().done && gens[0].next().done &&\n\
             rest.value === \"leaf\" && rest.done === false;"
        ),
        passed()
    );
}

/// Item 6: a chain of `reentry_depth` + 10 levels, below `frame_depth`, answers its values and
/// the value its leaf returns: no level is a re-entry. With `reentry_depth` 20, 30 levels.
#[test]
fn a_chain_deeper_than_reentry_depth_takes_no_re_entry_per_level() {
    let mut limits = limits();
    limits.reentry_depth = 20;
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "function* leaf() { yield 1; yield 2; return 3; }\n\
             function* level(inner) { return yield* inner; }\n\
             var it = leaf();\n\
             for (var i = 0; i < 30; i++) it = level(it);\n\
             var a = it.next(), b = it.next(), c = it.next();\n\
             [a.value, a.done, b.value, b.done, c.value, c.done].join();"
        ),
        Outcome::Normal("\"1,false,2,false,3,true\"".to_string())
    );
}

/// The body of the stack test: `f` re-enters itself through the `next` of a chain of three
/// generators, the innermost of which calls it.
fn reach_through_a_chain() {
    reach_through(
        "(function* () { yield* (function* () { yield* (function* () { yield f(); })(); })(); })().next()",
        BUILTINS_REENTRY,
    );
}

/// Item 6 and `Limits::min_stack_bytes` (decision 3): on a thread with seven eighths of
/// `min_stack_bytes()` for `frame_depth` 10,000, `reentry_depth` 1,000 and `parser_nesting`
/// 1,000, a script reaches `reentry_depth` re-entries, each the `next` of a chain of three
/// generators, and parses 1,000 nested brackets at the deepest, without a stack overflow: a
/// chain adds no Rust frame to a re-entry. In the collect mode the depth is
/// [`BUILTINS_REENTRY`].
#[test]
fn a_chain_runs_on_the_promised_stack() {
    let stack = stack_with_room(stack_limits(BUILTINS_REENTRY));
    on_bounded_stack(
        test_path!("a_chain_runs_on_the_promised_stack"),
        stack,
        reach_through_a_chain,
    )
    .unwrap();
}

/// An iterator that is no generator is called as the step of a `for`-`of` calls `next`
/// (step 8.a.i): its [[NextMethod]], read once, is called with the iterator as the this value
/// and the received value (first `undefined`), and the value it is done with is the value of
/// the `yield*`. Each call is a re-entry: with `reentry_depth` 20, 5 levels that each delegate
/// to such an iterator over the next level answer, and 30 throw a `RangeError`.
#[test]
fn yield_star_over_a_plain_iterator_calls_its_next_as_a_re_entry() {
    both_pass(
        "var log = [], n = 0, reads = 0, it = {}, step = function (v) {\n\
           log.push(this === it, v); n++; return { value: n, done: n > 2 };\n\
         };\n\
         it[Symbol.iterator] = function () { return it; };\n\
         Object.defineProperty(it, \"next\", { get: function () { reads++; return step; } });\n\
         function* g() { var r = yield* it; log.push(\"r\" + r); }\n\
         var gen = g(), a = gen.next(\"a\"), b = gen.next(\"b\"), c = gen.next(\"c\");\n\
         a.value === 1 && b.value === 2 && c.done && c.value === undefined && reads === 1 &&\n\
         log.join() === \"true,,true,b,true,c,r3\";",
    );
    let mut limits = limits();
    limits.reentry_depth = 20;
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    let setup = "function wrap(inner) {\n\
                   var o = {};\n\
                   o[Symbol.iterator] = function () { return o; };\n\
                   o.next = function (v) { return inner.next(v); };\n\
                   return o;\n\
                 }\n\
                 function* leaf() { yield 1; }\n\
                 function* level(inner) { yield* wrap(inner); }\n\
                 function levels(n) { var it = leaf(); for (var i = 0; i < n; i++) it = level(it); return it; }\n\
                 function answer(n) {\n\
                   try { return levels(n).next().value; }\n\
                   catch (e) { return e instanceof RangeError ? \"range\" : \"other\"; }\n\
                 }\n\
                 answer(5) + \",\" + answer(30);";
    assert_eq!(
        eval_text(&mut agent, &realm, setup),
        Outcome::Normal("\"1,range\"".to_string())
    );
}

/// GeneratorYield(innerResult) (step 8.a.vii): the result object of the iterator delegated to
/// is yielded as it is, through a plain iterator and through a generator that delegates to it:
/// the object itself, its "done" read once per level and its "value" not read.
#[test]
fn yield_star_yields_the_inner_result_object_as_it_is() {
    both_pass(
        "var reads = { done: 0, value: 0 };\n\
         var res = { get done() { reads.done++; return false; }, get value() { reads.value++; return 7; } };\n\
         var it = {};\n\
         it[Symbol.iterator] = function () { return it; };\n\
         it.next = function () { return res; };\n\
         function* g() { yield* it; }\n\
         function* outer() { yield* g(); }\n\
         var one = g().next(), done1 = reads.done, value1 = reads.value;\n\
         var two = outer().next();\n\
         one === res && two === res && done1 === 1 && value1 === 0 && reads.done === 3 &&\n\
         reads.value === 0;",
    );
}

/// A return given to the outermost level of a chain (step 8.c): each level gets the `return`
/// of the next by GetMethod and resumes it with the return, so the `finally` blocks run from
/// the innermost out, and every level completes with the value given.
#[test]
fn return_through_a_chain_runs_every_finally_from_the_innermost_out() {
    both_pass(
        "var log = [];\n\
         function* leaf() { try { yield 1; } finally { log.push(\"leaf\"); } }\n\
         function* level(name, inner) { try { yield* inner; } finally { log.push(name); } }\n\
         var l = leaf(), c = level(\"c\", l), it = level(\"a\", level(\"b\", c));\n\
         it.next();\n\
         var r = it[\"return\"](42), n = it.next();\n\
         log.join() === \"leaf,c,b,a\" && r.value === 42 && r.done === true &&\n\
         n.value === undefined && n.done === true && c.next().done && l.next().done;",
    );
}

/// A throw given to the outermost level of a chain (step 8.b): it reaches the generator the
/// innermost level delegates to, whose `catch` catches it and yields; a throw that generator
/// does not catch leaves it and the `yield*` of the level above, whose `catch` catches it.
#[test]
fn throw_through_a_chain_is_caught_by_the_innermost_catch() {
    both_pass(
        "var log = [];\n\
         function* leaf() { try { yield 1; } catch (e) { log.push(\"leaf \" + e); yield 2; } }\n\
         function* bare() { yield 1; }\n\
         function* level(name, inner) { try { yield* inner; } catch (e) { log.push(name + \" \" + e); } }\n\
         var it = level(\"a\", level(\"b\", leaf()));\n\
         it.next();\n\
         var r = it[\"throw\"](\"x\");\n\
         var caught = log.join();\n\
         log = [];\n\
         var other = level(\"a\", level(\"b\", bare()));\n\
         other.next();\n\
         var s = other[\"throw\"](\"y\");\n\
         r.value === 2 && r.done === false && caught === \"leaf x\" &&\n\
         s.done === true && log.join() === \"b y\";",
    );
}

/// Step 8.b.iii: a throw given while the iterator delegated to has no `throw` closes it
/// (IteratorClose with a normal completion: its `return` is called once) and throws a
/// `TypeError`, which completes the delegating generator.
#[test]
fn a_throw_into_a_delegate_without_a_throw_method_closes_it_and_throws_a_type_error() {
    both_pass(
        "var closed = 0, it = {};\n\
         it[Symbol.iterator] = function () { return it; };\n\
         it.next = function () { return { value: 1, done: false }; };\n\
         it[\"return\"] = function () { closed++; return {}; };\n\
         function* g() { yield* it; }\n\
         var gen = g(); gen.next();\n\
         var caught;\n\
         try { gen[\"throw\"](\"x\"); } catch (e) { caught = e; }\n\
         caught instanceof TypeError && closed === 1 && gen.next().done;",
    );
}

/// Item 6: a generator of another realm, whose `next` is the `%GeneratorPrototype%.next` of
/// that realm, is resumed in the loop as one of the same realm is: with `reentry_depth` 20 a
/// chain of 30 levels whose functions come from both realms in turn answers its leaf's value.
/// GeneratorValidate there throws the `TypeError` of that function's realm: a generator of the
/// first realm that delegates to a generator of the other one while that one runs gets the
/// other realm's `TypeError`.
#[test]
fn a_generator_of_another_realm_is_resumed_in_the_loop() {
    let mut limits = limits();
    limits.reentry_depth = 20;
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    let other_realm = support::host::realm(&mut agent);
    assert_eq!(
        eval_text(
            &mut agent,
            &other_realm,
            "function* level(inner) { yield* inner; }\n\
             function* leaf() { yield \"leaf\"; }\n\
             var agen;\n\
             function* bg() { try { agen.next(); } catch (e) { yield e; } }\n\
             0;"
        ),
        zero()
    );
    let global_root = global(&mut agent, &realm);
    let other_root = global(&mut agent, &other_realm);
    agent
        .with_cx(&realm, |cx| {
            let global = object(cx, &global_root);
            let other = object(cx, &other_root);
            let name = key(cx, "other");
            cx.set(global, name, Value::Object(other), true)
                .unwrap()
                .unwrap();
        })
        .unwrap();
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "function* level(inner) { yield* inner; }\n\
             var it = other.leaf();\n\
             for (var i = 0; i < 30; i++) it = (i % 2 ? other.level : level)(it);\n\
             var first = it.next();\n\
             var bgen = other.bg();\n\
             function* ag() { yield* bgen; }\n\
             other.agen = ag();\n\
             var e = bgen.next().value;\n\
             first.value === \"leaf\" && first.done === false &&\n\
             e instanceof other.TypeError && !(e instanceof TypeError);"
        ),
        passed()
    );
}

/// The step budget of the test of `Terminated(StepBudget)`: room for its other scripts, and few
/// enough steps that the loop which spends it stays cheap with a collection at every safe point.
const STEP_BUDGET: u64 = 1_000;

/// The chain the termination tests end inside: `a` delegates to `b`, `b` to `l`, whose body is
/// `body`; each has a `finally` that marks `ranFinally`.
fn chain_setup(body: &str) -> String {
    format!(
        "var ranFinally = false;\n\
         function* leaf() {{ try {{ {body} }} finally {{ ranFinally = true; }} }}\n\
         function* level(inner) {{ try {{ yield* inner; }} finally {{ ranFinally = true; }} }}\n\
         var l = leaf(), b = level(l), a = level(b);\n\
         0;"
    )
}

/// The script that checks, after a termination inside the chain of [`chain_setup`], that
/// `next`, `return` and `throw` of each of its three levels throw the `TypeError` of
/// GeneratorValidate, and that no `finally` ran.
const EVERY_LEVEL_EXECUTING: &str = "var seen = [];\n\
     for (var g of [a, b, l]) {\n\
       for (var m of [\"next\", \"return\", \"throw\"]) {\n\
         try { g[m](1); seen.push(\"answered\"); }\n\
         catch (e) { seen.push(e instanceof TypeError); }\n\
       }\n\
     }\n\
     seen.join() === \"true,true,true,true,true,true,true,true,true\" && !ranFinally;";

/// Item 5: a chain running when its entry ends in `termination` leaves every level whose frame
/// was on the stack `executing`: in this entry and the next, `next`, `return` and `throw` of
/// each throw a `TypeError`, and no `finally` ran.
fn every_level_stays_executing(
    agent: &mut Agent<TestHost>,
    realm: &Realm,
    body: &str,
    termination: Termination,
) {
    assert_eq!(eval_text(agent, realm, &chain_setup(body)), zero());
    assert_eq!(
        eval_text(agent, realm, "a.next();"),
        Outcome::Terminated(termination)
    );
    agent.collect().unwrap();
    assert_eq!(eval_text(agent, realm, EVERY_LEVEL_EXECUTING), passed());
    assert_eq!(eval_text(agent, realm, EVERY_LEVEL_EXECUTING), passed());
}

/// [`every_level_stays_executing`] at `Terminated(StepBudget)`.
#[test]
fn a_termination_inside_a_chain_leaves_every_running_level_executing_step_budget() {
    let mut limits = limits();
    limits.steps_per_entry = STEP_BUDGET;
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    every_level_stays_executing(
        &mut agent,
        &realm,
        "while (true) {}",
        Termination::StepBudget,
    );
}

/// The room of the memory budget test above the reading of a fresh realm: its setup runs in
/// it, [`FILL`] spends it, and a collection after the termination gives it back.
const FILL_ROOM: u64 = 1024 * 1024;

/// Statements that spend any room in a few steps and safe points: Strings that double, each
/// kept, until the charge of one is refused.
const FILL: &str = "var a = []; for (var n = 1024; ; n = n * 2) a.push(\"x\".repeat(n));";

/// [`every_level_stays_executing`] at `Terminated(MemoryBudget)`, with a memory budget of the
/// reading of `heap_bytes()` of a fresh realm, measured in an agent of its own, plus
/// [`FILL_ROOM`].
#[test]
fn a_termination_inside_a_chain_leaves_every_running_level_executing_memory_budget() {
    let realm_bytes = {
        let mut agent = agent().unwrap();
        let _realm = realm(&mut agent);
        agent.heap_bytes().unwrap()
    };
    let mut limits = limits();
    limits.memory_bytes = realm_bytes + FILL_ROOM;
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    every_level_stays_executing(&mut agent, &realm, FILL, Termination::MemoryBudget);
}

thread_local! {
    /// The interrupt handle [`interrupt_now`] sets.
    static HANDLE: std::cell::Cell<Option<InterruptHandle>> = const { std::cell::Cell::new(None) };
}

/// A host function that sets its agent's interrupt flag and answers `undefined`.
fn interrupt_now(_cx: &mut Cx<'_, TestHost>, _call: HostCall<'_>) -> Completion<Value> {
    HANDLE.with(|slot| {
        if let Some(handle) = slot.take() {
            handle.interrupt();
        }
    });
    Ok(Value::Undefined)
}

/// [`every_level_stays_executing`] at `Terminated(Interrupted)`.
#[test]
fn a_termination_inside_a_chain_leaves_every_running_level_executing_interrupted() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    define_function(&mut agent, &realm, "interruptNow", 0, interrupt_now, false);
    HANDLE.with(|slot| slot.set(Some(agent.interrupt_handle())));
    every_level_stays_executing(
        &mut agent,
        &realm,
        "interruptNow(); yield 1;",
        Termination::Interrupted,
    );
}
