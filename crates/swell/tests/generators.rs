//! Generators (`#sec-generator-objects`; `#sec-generator-function-definitions`): `function*`
//! declarations and expressions, `yield`, and `next`, `return` and `throw` of
//! `%GeneratorPrototype%`, on the suspended frame of `docs/design/engine.md`, decision 3,
//! "Suspended frames" (items 1 to 6): a `yield` moves the running frame, boxed, into its
//! generator object, and a resumption moves it back onto the frame stack. Written by
//! `story:generator-functions-and-suspended-frames`. Run also with the feature
//! `collect-every-safe-point`.
//!
//! Each script compares its results with `===` and answers `true`. A test that reads
//! `heap_bytes()` around an entry parses that entry's script before the first reading, so the
//! reading holds what the run charged and not the code of the script, and declares the global
//! names it assigns in an earlier script, so no global property is added between the readings.
#![cfg(test)]

use crate::support;

use support::host::{TestHost, agent, agent_with, limits, realm, stack_with_room};
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

/// The local slots of the generator functions the tests of the charge make: item 3 of
/// "Suspended frames" states the charge of a never resumed generator with L = 200.
const LOCALS: usize = 200;

/// The bytes item 3 states for [`LOCALS`] local slots: 16 bytes per slot.
const FRAME_FLOOR: u64 = 16 * LOCALS as u64;

/// A generator function `name` with [`LOCALS`] `var` names (each a local slot of its frame) and
/// the body `body` after them.
fn with_locals(name: &str, body: &str) -> String {
    let names: Vec<String> = (0..LOCALS).map(|i| format!("v{i}")).collect();
    format!("function* {name}() {{ var {}; {body} }}", names.join(", "))
}

/// Evaluate the parsed script `script` in `realm` and describe its outcome.
fn evaluate(agent: &mut Agent<TestHost>, realm: &Realm, script: &swell::Script) -> Outcome {
    let outcome = agent.evaluate_script(script).unwrap();
    describe(agent, realm, outcome)
}

/// `0` as an outcome: what a script that ends with `0;` answers.
fn zero() -> Outcome {
    Outcome::Normal("0".to_string())
}

/// GeneratorResume through `next` (`#sec-generator.prototype.next`): each `next` runs the body
/// to its next `yield` and answers `{ value, done: false }`; the end of the body answers
/// `{ value: undefined, done: true }`, and so does every `next` after it.
#[test]
fn next_runs_to_each_yield_and_answers_done_at_the_end() {
    both_pass(
        "function* g() { yield 1; yield 2; }\n\
         var it = g(), a = it.next(), b = it.next(), c = it.next(), d = it.next();\n\
         a.value === 1 && a.done === false && b.value === 2 && b.done === false &&\n\
         c.value === undefined && c.done === true && d.value === undefined && d.done === true;",
    );
}

/// The value given to `next` is the value of the `yield` the generator is suspended at
/// (`#sec-generator-function-definitions-runtime-semantics-evaluation`); the value given to the
/// first `next` is not seen; a `return` of the body answers `done: true` with its value.
#[test]
fn the_value_given_to_next_is_the_value_of_the_yield() {
    both_pass(
        "function* g() { var x = yield 1; var y = yield x + 1; return x * y; }\n\
         var it = g(), a = it.next(\"lost\"), b = it.next(10), c = it.next(3);\n\
         a.value === 1 && b.value === 11 && !b.done && c.value === 30 && c.done === true;",
    );
}

/// `return` given to a generator suspended at a `yield` inside two `try` statements runs both
/// `finally` blocks, innermost first, and completes it with the value given
/// (`#sec-generator.prototype.return`, GeneratorResumeAbrupt, `#sec-generatorresumeabrupt`);
/// the code after the `try` statements does not run.
#[test]
fn return_runs_the_finally_blocks_around_the_yield_and_completes() {
    both_pass(
        "var log = [];\n\
         function* g() {\n\
           try { try { yield 1; } finally { log.push(\"inner\"); } }\n\
           finally { log.push(\"outer\"); }\n\
           log.push(\"after\");\n\
         }\n\
         var it = g(); it.next();\n\
         var r = it.return(42), n = it.next();\n\
         r.value === 42 && r.done === true && log.join() === \"inner,outer\" &&\n\
         n.value === undefined && n.done === true;",
    );
}

/// `throw` given to a generator suspended at a `yield` inside a `try` with a `catch` is caught
/// there (`#sec-generator.prototype.throw`): the catch block runs and its `yield` answers.
#[test]
fn a_throw_given_to_a_suspended_generator_is_caught_by_a_catch_around_the_yield() {
    both_pass(
        "function* g() { try { yield 1; } catch (e) { yield \"caught \" + e; } return 9; }\n\
         var it = g(); it.next();\n\
         var r = it.throw(\"x\"), end = it.next();\n\
         r.value === \"caught x\" && r.done === false && end.value === 9 && end.done;",
    );
}

/// GeneratorResumeAbrupt, step 2 (`#sec-generatorresumeabrupt`): `return` and `throw` given to
/// a generator that has not started complete it without running its code, and its frame is
/// credited at once ("Suspended frames", item 3): with [`LOCALS`] local slots and no
/// `collect()`, `heap_bytes()` after a `throw` the caller catches is at least [`FRAME_FLOOR`]
/// below the reading before it, and so is the reading after the `return` once the result
/// object the `return` answers is counted back: its bytes are what the same `return` of a
/// completed generator, which credits nothing, adds to the reading.
#[test]
fn return_and_throw_complete_a_suspended_start_generator_without_running_it() {
    both_pass(
        "var ran = false;\n\
         function* g() { ran = true; yield 1; }\n\
         var a = g(), r = a.return(5), b = g(), caught;\n\
         try { b.throw(new Error(\"e\")); } catch (e) { caught = e.message; }\n\
         r.value === 5 && r.done === true && caught === \"e\" && !ran &&\n\
         a.next().done && b.next().done && !ran;",
    );

    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let setup = format!("{}\nvar g, h, done; 0;", with_locals("gen", "yield 1;"));
    assert_eq!(eval_text(&mut agent, &realm, &setup), zero());
    let make = parse(
        &mut agent,
        &realm,
        "g = gen(); h = gen(); done = gen(); done.return(0); 0;",
    );
    let returned = parse(&mut agent, &realm, "g.return(1); 0;");
    let answered = parse(&mut agent, &realm, "done.return(1); 0;");
    let thrown = parse(&mut agent, &realm, "try { h.throw(1); } catch (e) {} 0;");
    assert_eq!(evaluate(&mut agent, &realm, &make), zero());
    agent.collect().unwrap();
    let before = agent.heap_bytes().unwrap();
    assert_eq!(evaluate(&mut agent, &realm, &returned), zero());
    let after_return = agent.heap_bytes().unwrap();
    assert_eq!(evaluate(&mut agent, &realm, &answered), zero());
    let after_answer = agent.heap_bytes().unwrap();
    assert_eq!(evaluate(&mut agent, &realm, &thrown), zero());
    let after_throw = agent.heap_bytes().unwrap();
    let result_bytes = after_answer.saturating_sub(after_return);
    println!(
        "heap_bytes: {before} before, {after_return} after return, {after_answer} after the \
         return of a completed generator (+{result_bytes}), {after_throw} after throw"
    );
    assert!(
        before.saturating_sub(after_return) + result_bytes >= FRAME_FLOOR,
        "return: {before} -> {after_return}, result object {result_bytes}"
    );
    assert!(
        after_answer.saturating_sub(after_throw) >= FRAME_FLOOR,
        "throw: {after_answer} -> {after_throw}"
    );
}

/// GeneratorValidate, step 6 (`#sec-generatorvalidate`): `next`, `return` and `throw` of a
/// generator whose own code is running throw a `TypeError`.
#[test]
fn next_of_an_executing_generator_throws_a_type_error() {
    both_pass(
        "var it;\n\
         function* g() {\n\
           var seen = [];\n\
           for (var m of [\"next\", \"return\", \"throw\"]) {\n\
             try { it[m](1); seen.push(\"answered\"); }\n\
             catch (e) { seen.push(e instanceof TypeError); }\n\
           }\n\
           yield seen.join();\n\
         }\n\
         it = g();\n\
         it.next().value === \"true,true,true\" && it.next().done;",
    );
}

/// EvaluateGeneratorBody (`#sec-runtime-semantics-evaluategeneratorbody`): the parameters are
/// bound (FunctionDeclarationInstantiation) before OrdinaryCreateFromConstructor makes the
/// object, so a default that throws throws from the call itself, and a default that sets the
/// `prototype` of the function gives the object that prototype.
#[test]
fn the_parameters_are_bound_before_the_generator_object_is_made() {
    both_pass(
        "var answered = false, threw = false;\n\
         function* g(a = (function () { throw new RangeError(\"p\"); })()) { yield a; }\n\
         try { g(); answered = true; } catch (e) { threw = e instanceof RangeError; }\n\
         var proto = { mark: 1 };\n\
         function* h(a = (h.prototype = proto, 2)) { yield a; }\n\
         var it = h();\n\
         threw && !answered && Object.getPrototypeOf(it) === proto && h.prototype === proto;",
    );
}

/// InstantiateGeneratorFunctionObject and InstantiateGeneratorFunctionExpression
/// (`#sec-runtime-semantics-instantiategeneratorfunctionobject`,
/// `#sec-runtime-semantics-instantiategeneratorfunctionexpression`): [[Prototype]]
/// `%GeneratorFunction.prototype%`, no [[Construct]], and a `prototype` property, writable, not
/// enumerable, not configurable, whose object inherits from `%GeneratorPrototype%` and has no
/// `constructor`; `%GeneratorFunction.prototype%` and `%GeneratorPrototype%` with the
/// properties of `#sec-properties-of-the-generatorfunction-prototype-object` and
/// `#sec-properties-of-generator-prototype`.
#[test]
fn a_generator_function_has_its_prototype_object_and_no_construct() {
    both_pass(
        "function* g() {}\n\
         var e = function* () {};\n\
         var d = Object.getOwnPropertyDescriptor(g, \"prototype\");\n\
         var GFP = Object.getPrototypeOf(g), GP = Object.getPrototypeOf(g.prototype);\n\
         var IP = Object.getPrototypeOf(Object.getPrototypeOf([][Symbol.iterator]()));\n\
         var fp = Object.getOwnPropertyDescriptor(GFP, \"prototype\");\n\
         var gc = Object.getOwnPropertyDescriptor(GP, \"constructor\");\n\
         var constructed = false;\n\
         try { new g(); constructed = true; } catch (x) { constructed = !(x instanceof TypeError); }\n\
         d.writable && !d.enumerable && !d.configurable &&\n\
         !Object.prototype.hasOwnProperty.call(g.prototype, \"constructor\") &&\n\
         Object.getPrototypeOf(e) === GFP && Object.getPrototypeOf(e.prototype) === GP &&\n\
         g.prototype !== e.prototype && Object.getPrototypeOf(g()) === g.prototype &&\n\
         Object.getPrototypeOf(GFP) === Function.prototype && Object.getPrototypeOf(GP) === IP &&\n\
         fp.value === GP && !fp.writable && !fp.enumerable && fp.configurable &&\n\
         gc.value === GFP && !gc.writable && !gc.enumerable && gc.configurable &&\n\
         GFP[Symbol.toStringTag] === \"GeneratorFunction\" && GP[Symbol.toStringTag] === \"Generator\" &&\n\
         typeof GP.next === \"function\" && GP.next.length === 1 && GP[\"return\"].length === 1 &&\n\
         GP[\"throw\"].length === 1 && GP.next.name === \"next\" && !constructed;",
    );
}

/// A `break` out of a `for`-`of` over a generator calls its `return` (IteratorClose,
/// `#sec-iteratorclose`), which runs the `finally` around its `yield`.
#[test]
fn a_break_out_of_a_for_of_over_a_generator_runs_its_finally() {
    both_pass(
        "var log = [];\n\
         function* g() { try { yield 1; yield 2; } finally { log.push(\"finally\"); } }\n\
         for (var x of g()) { log.push(x); break; }\n\
         log.join() === \"1,finally\";",
    );
}

/// A `for`-`of` around a `yield` keeps its Iterator Record on the saved operand stack across
/// the suspension ("The iterator of `for`-`of`", item 1): the loop goes on after each
/// resumption, and a `return` given at the `yield` closes its iterator.
#[test]
fn a_for_of_around_a_yield_keeps_its_iterator_across_the_suspension() {
    both_pass(
        "function* g(list) { for (var x of list) yield x; }\n\
         var it = g([1, 2, 3]);\n\
         var got = [it.next().value, it.next().value, it.next().value, it.next().done].join();\n\
         var closed = 0, source = {};\n\
         source[Symbol.iterator] = function () {\n\
           var i = 0;\n\
           return { next: function () { i++; return { value: i, done: i > 5 }; },\n\
                    \"return\": function () { closed++; return {}; } };\n\
         };\n\
         var k = g(source); k.next(); k.next();\n\
         var r = k[\"return\"](9);\n\
         got === \"1,2,3,true\" && closed === 1 && r.value === 9 && r.done;",
    );
}

/// "Suspended frames", item 2: an object that only a local slot and the operand stack of a
/// suspended frame hold survives `collect()`, traced at its generator's kind site, and the
/// resumption reads both.
#[test]
fn a_suspended_frame_keeps_its_values_through_collect() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let setup = "function pair(a, b) { return a.mark + b; }\n\
                 var g = (function* () {\n\
                   var local = { mark: \"local\" };\n\
                   var r = pair({ mark: \"stack\" }, yield 1);\n\
                   yield local.mark + \",\" + r;\n\
                 })();\n\
                 g.next().value;";
    assert_eq!(
        eval_text(&mut agent, &realm, setup),
        Outcome::Normal("1".to_string())
    );
    agent.collect().unwrap();
    agent.collect().unwrap();
    assert_eq!(
        eval_text(&mut agent, &realm, "g.next(\"!\").value;"),
        Outcome::Normal("\"local,stack!\"".to_string())
    );
}

/// "Suspended frames", item 3: with [`LOCALS`] local slots, `heap_bytes()` after the call that
/// makes a generator exceeds the reading before it by at least [`FRAME_FLOOR`], and once the
/// script drops the generator one `collect()` brings the reading at least [`FRAME_FLOOR`] below
/// the reading while it was held: the sweep that frees the object credits its frame. With a
/// collection at every safe point the script that drops it frees it already, so the second
/// difference is read from the reading while it was held.
#[test]
fn a_suspended_frame_is_charged_until_its_generator_is_freed() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let setup = format!("{}\nvar g; 0;", with_locals("gen", "yield 1;"));
    assert_eq!(eval_text(&mut agent, &realm, &setup), zero());
    let make = parse(&mut agent, &realm, "g = gen(); 0;");
    let drop_it = parse(&mut agent, &realm, "g = null; 0;");
    agent.collect().unwrap();
    let before = agent.heap_bytes().unwrap();
    assert_eq!(evaluate(&mut agent, &realm, &make), zero());
    let held = agent.heap_bytes().unwrap();
    assert_eq!(evaluate(&mut agent, &realm, &drop_it), zero());
    agent.collect().unwrap();
    let freed = agent.heap_bytes().unwrap();
    println!(
        "heap_bytes: {before} before the call, {held} after it (+{}), {freed} after collect() \
         (-{})",
        held.saturating_sub(before),
        held.saturating_sub(freed)
    );
    assert!(
        held.saturating_sub(before) >= FRAME_FLOOR,
        "{before} -> {held}"
    );
    assert!(
        held.saturating_sub(freed) >= FRAME_FLOOR,
        "{held} -> {freed}"
    );
}

/// "Suspended frames", item 3: the frame of a generator that completes is credited by the
/// `next` that completes it, with no `collect()`, and the collection that frees the object
/// then gives back less than [`FRAME_FLOOR`]: the frame is credited once.
#[test]
fn a_completed_generator_holds_no_frame() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let setup = format!(
        "{}\nvar g = gen(); g.next(); 0;",
        with_locals("gen", "yield 1;")
    );
    assert_eq!(eval_text(&mut agent, &realm, &setup), zero());
    let complete = parse(&mut agent, &realm, "g.next(); 0;");
    let drop_it = parse(&mut agent, &realm, "g = null; 0;");
    agent.collect().unwrap();
    let suspended = agent.heap_bytes().unwrap();
    assert_eq!(evaluate(&mut agent, &realm, &complete), zero());
    let completed = agent.heap_bytes().unwrap();
    assert_eq!(evaluate(&mut agent, &realm, &drop_it), zero());
    agent.collect().unwrap();
    let freed = agent.heap_bytes().unwrap();
    println!("heap_bytes: {suspended} suspended, {completed} completed, {freed} freed");
    assert!(
        suspended.saturating_sub(completed) >= FRAME_FLOOR,
        "{suspended} -> {completed}"
    );
    assert!(
        completed.saturating_sub(freed) < FRAME_FLOOR,
        "{completed} -> {freed}"
    );
}

/// The iterations of the longer loop of [`the_entry_that_resumes_a_generator_spends_its_steps`]:
/// 1,000, the story's count; 100 with the feature `collect-every-safe-point`, where every safe
/// point of the loop collects the whole heap (1,000 took 2.72 CPU seconds alone there, over the
/// 2 a test may take).
const RESUMED_LOOP: u32 = if cfg!(feature = "collect-every-safe-point") {
    100
} else {
    1_000
};

/// "Suspended frames", item 5: the instructions a resumed generator runs are steps of the
/// entry that resumes it, whichever entry made it: an entry that resumes a generator whose body
/// loops [`RESUMED_LOOP`] times before its next `yield` spends at least `RESUMED_LOOP` - 10
/// steps more than one whose body loops 10 times.
#[test]
fn the_entry_that_resumes_a_generator_spends_its_steps() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let setup = format!(
        "function* gen(n) {{ yield 0; for (var i = 0; i < n; i++) {{}} yield 1; }}\n\
         var big = gen({RESUMED_LOOP}), few = gen(10); big.next(); few.next(); 0;"
    );
    assert_eq!(eval_text(&mut agent, &realm, &setup), zero());
    // The reading comes before `describe`, whose `with_cx` is an entry of its own.
    let mut steps_of = |source: &str| {
        let script = parse(&mut agent, &realm, source);
        let outcome = agent.evaluate_script(&script).unwrap();
        let steps = agent.last_entry_steps().unwrap();
        assert_eq!(describe(&mut agent, &realm, outcome), zero(), "{source}");
        steps
    };
    let many = steps_of("big.next(); 0;");
    let some = steps_of("few.next(); 0;");
    println!("last_entry_steps: {many} resuming the loop of {RESUMED_LOOP}, {some} the loop of 10");
    assert!(
        many.saturating_sub(some) >= u64::from(RESUMED_LOOP) - 10,
        "{many} against {some}"
    );
}

/// The step budget of the tests of `Terminated(StepBudget)`: room for their other scripts, and
/// few enough steps that the loop which spends it stays cheap with a collection at every safe
/// point.
const STEP_BUDGET: u64 = 1_000;

/// The script that checks, after a termination while the generator `g` ran, that `next`,
/// `return` and `throw` of it each throw the `TypeError` of GeneratorValidate.
const STAYS_EXECUTING: &str = "var seen = [];\n\
     for (var m of [\"next\", \"return\", \"throw\"]) {\n\
       try { g[m](1); seen.push(\"answered\"); }\n\
       catch (e) { seen.push(e instanceof TypeError); }\n\
     }\n\
     seen.join() === \"true,true,true\" && !ranFinally;";

/// HOST-73 (`docs/design/host-interface.md`, section 10): a generator running when its entry
/// ends in `Terminated(StepBudget)` stays `executing` with no frame, and `next`, `return` and
/// `throw` of it throw a `TypeError` in every later entry; no `finally` around its code ran
/// ("Suspended frames", item 5).
#[test]
fn host_73_a_generator_running_at_a_step_budget_termination_throws_a_type_error_from_its_next_next()
{
    let mut limits = limits();
    limits.steps_per_entry = STEP_BUDGET;
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    let setup = "var ranFinally = false;\n\
                 var g = (function* () { try { while (true) {} } finally { ranFinally = true; } })();\n\
                 0;";
    assert_eq!(eval_text(&mut agent, &realm, setup), zero());
    assert_eq!(
        eval_text(&mut agent, &realm, "g.next();"),
        Outcome::Terminated(Termination::StepBudget)
    );
    assert_eq!(eval_text(&mut agent, &realm, STAYS_EXECUTING), passed());
    assert_eq!(eval_text(&mut agent, &realm, STAYS_EXECUTING), passed());
}

/// The room of the memory budget tests above the reading of a fresh realm: the setup scripts
/// run in it (64 KiB is too little for the setup of [`resumes_after`]), [`FILL`] spends it, and
/// a collection after the termination gives it back for the scripts after it.
const FILL_ROOM: u64 = 1024 * 1024;

/// Statements that spend any room in a few steps and safe points: Strings that double, each
/// kept, until the charge of one is refused.
const FILL: &str = "var a = []; for (var n = 1024; ; n = n * 2) a.push(\"x\".repeat(n));";

/// An agent with a realm whose memory budget is the reading of `heap_bytes()` of a fresh
/// realm, measured in an agent of its own, plus [`FILL_ROOM`].
fn memory_agent() -> (Agent<TestHost>, Realm) {
    let realm_bytes = {
        let mut agent = agent().unwrap();
        let _realm = realm(&mut agent);
        agent.heap_bytes().unwrap()
    };
    let mut limits = limits();
    limits.memory_bytes = realm_bytes + FILL_ROOM;
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    (agent, realm)
}

/// "Suspended frames", item 5: a generator running when its entry ends in
/// `Terminated(MemoryBudget)` stays `executing` with no frame: `next`, `return` and `throw` of
/// it throw a `TypeError` in later entries, and its `finally` did not run.
#[test]
fn a_generator_running_at_a_memory_budget_termination_stays_executing() {
    let (mut agent, realm) = memory_agent();
    let setup = format!(
        "var ranFinally = false;\n\
         var g = (function* () {{ try {{ {FILL} }} finally {{ ranFinally = true; }} }})();\n\
         0;"
    );
    assert_eq!(eval_text(&mut agent, &realm, &setup), zero());
    assert_eq!(
        eval_text(&mut agent, &realm, "g.next();"),
        Outcome::Terminated(Termination::MemoryBudget)
    );
    agent.collect().unwrap();
    assert_eq!(eval_text(&mut agent, &realm, STAYS_EXECUTING), passed());
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

/// An agent with the test limits, a realm and the host function `interruptNow`.
fn interrupt_agent() -> (Agent<TestHost>, Realm) {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    define_function(&mut agent, &realm, "interruptNow", 0, interrupt_now, false);
    (agent, realm)
}

/// "Suspended frames", item 5: a generator running when its entry ends in
/// `Terminated(Interrupted)` stays `executing` with no frame.
#[test]
fn a_generator_running_at_an_interrupt_stays_executing() {
    let (mut agent, realm) = interrupt_agent();
    let setup = "var ranFinally = false;\n\
                 var g = (function* () { try { interruptNow(); yield 1; } finally { ranFinally = true; } })();\n\
                 0;";
    assert_eq!(eval_text(&mut agent, &realm, setup), zero());
    HANDLE.with(|slot| slot.set(Some(agent.interrupt_handle())));
    assert_eq!(
        eval_text(&mut agent, &realm, "g.next();"),
        Outcome::Terminated(Termination::Interrupted)
    );
    assert_eq!(eval_text(&mut agent, &realm, STAYS_EXECUTING), passed());
}

/// The first entry of [`resumes_after`]: the generator `g`, suspended at its first `yield`, and
/// the global names the later entries assign.
fn resume_setup() -> String {
    let generator = with_locals(
        "gen",
        "try { yield 1; yield 2; } finally { ranFinally = true; }",
    );
    format!(
        "{generator}\nvar ranFinally = false, before, result, second, secondDone, endDone;\n\
         var g = gen(); g.next(); 0;"
    )
}

/// A generator suspended at a `yield` inside a `try` with a `finally`, with [`LOCALS`] local
/// slots, made by the first entry; `ending` ends the second entry in `termination` while it is
/// suspended. Then (item 5): its `finally` did not run, `heap_bytes()` still holds its frame
/// (the `next` that completes it, in a later entry, lowers the reading by at least
/// [`FRAME_FLOOR`]; each reading follows a `collect()`, and `g` stays reachable, so no sweep
/// frees the generator or its frame), and the later entry gets its next value.
fn resumes_after(
    agent: &mut Agent<TestHost>,
    realm: &Realm,
    ending: &str,
    termination: Termination,
) {
    assert_eq!(eval_text(agent, realm, &resume_setup()), zero());
    assert_eq!(
        eval_text(agent, realm, ending),
        Outcome::Terminated(termination)
    );
    let resume = parse(
        agent,
        realm,
        "before = ranFinally; result = g.next(); second = result.value;\n\
         secondDone = result.done; endDone = g.next().done; result = null; 0;",
    );
    agent.collect().unwrap();
    let suspended = agent.heap_bytes().unwrap();
    assert_eq!(evaluate(agent, realm, &resume), zero());
    agent.collect().unwrap();
    let completed = agent.heap_bytes().unwrap();
    println!("heap_bytes after {termination:?}: {suspended} suspended, {completed} completed");
    assert!(
        suspended.saturating_sub(completed) >= FRAME_FLOOR,
        "{suspended} -> {completed}"
    );
    assert_eq!(
        eval_text(
            agent,
            realm,
            "before === false && second === 2 && secondDone === false &&\n\
             endDone === true && ranFinally === true;"
        ),
        passed()
    );
}

/// "Suspended frames", item 5: a generator suspended when an entry ends in
/// `Terminated(StepBudget)` keeps its state, its frame and its charge, and resumes in a later
/// entry.
#[test]
fn a_generator_suspended_at_a_step_budget_termination_resumes_in_a_later_entry() {
    let mut limits = limits();
    limits.steps_per_entry = STEP_BUDGET;
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    resumes_after(
        &mut agent,
        &realm,
        "while (true) {}",
        Termination::StepBudget,
    );
}

/// "Suspended frames", item 5: the same at `Terminated(MemoryBudget)`; the collection after
/// the termination frees what the fill held, not the suspended generator.
#[test]
fn a_generator_suspended_at_a_memory_budget_termination_resumes_in_a_later_entry() {
    let (mut agent, realm) = memory_agent();
    resumes_after(
        &mut agent,
        &realm,
        &format!("(function () {{ {FILL} }})();"),
        Termination::MemoryBudget,
    );
}

/// "Suspended frames", item 5: the same at `Terminated(Interrupted)`.
#[test]
fn a_generator_suspended_at_an_interrupt_resumes_in_a_later_entry() {
    let (mut agent, realm) = interrupt_agent();
    HANDLE.with(|slot| slot.set(Some(agent.interrupt_handle())));
    resumes_after(
        &mut agent,
        &realm,
        "interruptNow(); 0;",
        Termination::Interrupted,
    );
}

/// "Suspended frames", item 4: a suspended generator takes no place against `frame_depth`: a
/// script with `frame_depth` 10 holds 11 suspended generators and resumes each.
#[test]
fn a_suspended_generator_takes_no_frame_depth() {
    let mut limits = limits();
    limits.frame_depth = 10;
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "function* one(i) { yield i; return i * 2; }\n\
             var held = [];\n\
             for (var i = 0; i <= 10; i++) { var it = one(i); it.next(); held.push(it); }\n\
             var sum = 0;\n\
             for (var j = 0; j < held.length; j++) sum += held[j].next().value;\n\
             held.length === 11 && sum === 110;"
        ),
        passed()
    );
}

/// "Suspended frames", item 4: with `frame_depth` 20 and 20 frames on the stack (the script's
/// and 19 calls of `at`), `next` of a suspended-start and of a suspended-yield generator throws
/// the `RangeError` of "Depth bounds" and leaves the generator suspended: resumed once the
/// stack is lower, it answers its next value.
#[test]
fn a_resumption_at_frame_depth_throws_a_range_error_and_leaves_the_generator_suspended() {
    let mut limits = limits();
    limits.frame_depth = 20;
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var g = (function* () { yield 1; yield 2; })();\n\
             function at(n) {\n\
               if (n > 1) return at(n - 1);\n\
               try { return g.next().value; }\n\
               catch (e) { return e instanceof RangeError ? \"range\" : \"other\"; }\n\
             }\n\
             [at(19), at(18), at(19), at(18), at(18)].join();"
        ),
        Outcome::Normal("\"range,1,range,2,\"".to_string())
    );
}

/// "Suspended frames", item 6, "Re-entry": a generator whose body calls `next` of another
/// nests one re-entry per level and ends at `reentry_depth` with a `RangeError`: with
/// `reentry_depth` 20, 20 levels run.
#[test]
fn a_generator_resuming_a_generator_reaches_reentry_depth() {
    let mut limits = limits();
    limits.reentry_depth = 20;
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var depth = 0, range = false;\n\
             function* chain() { depth++; chain().next(); yield depth; }\n\
             try { chain().next(); } catch (e) { range = e instanceof RangeError; }\n\
             range + \",\" + depth;"
        ),
        Outcome::Normal("\"true,20\"".to_string())
    );
}

/// The body of the stack test: `f` re-enters itself through the `next` of a generator, called
/// from script and by the step of a `for`-`of`, one agent each.
fn reach_through_generators() {
    for through in [
        "(function* () { yield f(); })().next()",
        "for (var x of (function* () { yield f(); })()) {}",
    ] {
        reach_through(through, BUILTINS_REENTRY);
    }
}

/// "Suspended frames", item 6, "Re-entry", and `Limits::min_stack_bytes` (decision 3): on a
/// thread with seven eighths of `min_stack_bytes()` for `frame_depth` 10,000, `reentry_depth`
/// 1,000 and `parser_nesting` 1,000, a script reaches `reentry_depth` re-entries through the
/// resumption of a generator, and parses 1,000 nested brackets at the deepest, without a stack
/// overflow. In the collect mode the depth is [`BUILTINS_REENTRY`].
#[test]
fn a_resumption_re_enters_on_the_promised_stack() {
    let stack = stack_with_room(stack_limits(BUILTINS_REENTRY));
    on_bounded_stack(
        test_path!("a_resumption_re_enters_on_the_promised_stack"),
        stack,
        reach_through_generators,
    )
    .unwrap();
}

/// The sweep of every function with a Symbol (`builtins_symbol_values.rs`) walks from the global
/// object and reaches no function of `%GeneratorPrototype%`, so this test calls `next`,
/// `return` and `throw` with a Symbol: as the value it is yielded, returned or thrown as any
/// value is, and as `this` it throws the `TypeError` of GeneratorValidate.
#[test]
fn next_return_and_throw_take_a_symbol() {
    both_pass(
        "var s = Symbol(\"s\");\n\
         function* g() { var got = yield 1; yield got; }\n\
         var a = g(); a.next();\n\
         var b = g(); b.next();\n\
         var c = g(); c.next();\n\
         var thrown;\n\
         try { c[\"throw\"](s); } catch (e) { thrown = e; }\n\
         var types = [];\n\
         for (var m of [\"next\", \"return\", \"throw\"]) {\n\
           try { g.prototype[m].call(s, 1); types.push(\"answered\"); }\n\
           catch (e) { types.push(e instanceof TypeError); }\n\
         }\n\
         a.next(s).value === s && b[\"return\"](s).value === s && thrown === s &&\n\
         types.join() === \"true,true,true\";",
    );
}

/// The refusal `text` of a source `eval` parses: a `SyntaxError` with that message, which moves
/// `refusals()` by 1.
fn refusal_is_counted(source: &str, text: &str) {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let before = agent.refusals();
    let script = format!(
        "var message;\n\
         try {{ eval({source:?}); }} catch (e) {{ message = e instanceof SyntaxError && e.message; }}\n\
         message;"
    );
    assert_eq!(
        eval_text(&mut agent, &realm, &script),
        Outcome::Normal(format!("\"{text}\""))
    );
    assert_eq!(agent.refusals(), before + 1);
}

/// A generator method keeps a refusal of its own until the story of generator methods,
/// counted.
#[test]
fn the_refusal_of_a_generator_method_is_counted() {
    refusal_is_counted(
        "({ *m() { yield 1; } });",
        "generator method is not supported before slice 7 (1:4)",
    );
}
