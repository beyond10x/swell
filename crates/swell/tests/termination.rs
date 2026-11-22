//! The state a terminated run leaves (`docs/design/host-interface.md`, decision 10;
//! `docs/design/engine.md`, decision 3): no `catch` or `finally` block runs, the frames are
//! dropped, the agent stays `Alive` after every termination but `EngineFault`, a host function
//! cannot end a termination, and the same script run twice spends the same steps. Run also with
//! the feature `collect-every-safe-point`.
#![cfg(test)]

mod support;

use support::host::{TestHost, agent_with, limits, realm};
use support::script::{Outcome, define_function, eval_text};
use swell::{Abrupt, AgentState, Completion, Cx, HostCall, Termination, Value};

/// HOST-69 (`html:killing-scripts`): `var f; try { while (true) {} } finally { f = 1; }` ends
/// in `Terminated(StepBudget)`, and the script `f` then answers `Normal(undefined)`: the
/// `finally` block did not run.
#[test]
fn host_69_finally_does_not_run_after_step_budget() {
    let mut limits = limits();
    limits.steps_per_entry = 1_000;
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var f; try { while (true) {} } finally { f = 1; }"
        ),
        Outcome::Terminated(Termination::StepBudget)
    );
    assert_eq!(
        eval_text(&mut agent, &realm, "f;"),
        Outcome::Normal("undefined".to_string())
    );
    // Nor a `catch` block, nor one of a calling frame.
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var c = 0; function g() { try { while (true) {} } catch (e) { c = 1; } } \
             try { g(); } catch (e) { c = 2; } finally { c = 3; }"
        ),
        Outcome::Terminated(Termination::StepBudget)
    );
    assert_eq!(
        eval_text(&mut agent, &realm, "c;"),
        Outcome::Normal("0".to_string())
    );
}

/// Whether this run is the gate's with the collector at every safe point: the feature
/// `collect-every-safe-point`, without `SWELL_FULL_COUNTS=1` (which the lane `slow` sets).
fn in_the_collect_gate() -> bool {
    cfg!(feature = "collect-every-safe-point")
        && !std::env::var("SWELL_FULL_COUNTS").is_ok_and(|value| value == "1")
}

/// The step budget and the memory room above the realm of
/// [`host_70_agent_stays_alive_after_each_budget`]: 10,000 steps and 64 KiB, the full counts,
/// which the lane `slow` (`SWELL_FULL_COUNTS=1`) runs in both modes; a quarter of each, 2,500
/// steps and 16 KiB, in the gate with the collector at every safe point, where every step
/// collects (the test took 29.0 s there with the full counts at a load of 100 to 110 on 17
/// cores, cargo-nextest). The memory script charges the same bytes per step whatever the
/// budget, so a quarter of the room is filled in a quarter of the steps, and it still ends in
/// the memory budget before the step budget, as with the full counts.
fn host_70_budgets() -> (u64, u64) {
    if in_the_collect_gate() {
        (2_500, 16 * 1024)
    } else {
        (10_000, 64 * 1024)
    }
}

/// HOST-70: after `Terminated(StepBudget)`, `Terminated(MemoryBudget)` and
/// `Terminated(Interrupted)`, `state()` reads `Alive` and the script `1` answers `Normal(1)`.
/// The memory budget is the realm's bytes plus 64 KiB, which the memory script fills with
/// Arrays long before its 10,000 steps (a quarter of each in the collect gate,
/// [`host_70_budgets`]); they are held by its function's frame only, so the collection after
/// the termination frees them for the next script.
#[test]
fn host_70_agent_stays_alive_after_each_budget() {
    let realm_bytes = {
        let mut agent = support::host::agent().unwrap();
        let _realm = realm(&mut agent);
        agent.heap_bytes().unwrap()
    };
    let (steps, room) = host_70_budgets();
    let mut limits = limits();
    limits.steps_per_entry = steps;
    limits.memory_bytes = realm_bytes + room;
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    let one = Outcome::Normal("1".to_string());

    assert_eq!(
        eval_text(&mut agent, &realm, "while (true) {}"),
        Outcome::Terminated(Termination::StepBudget)
    );
    assert_eq!(agent.state(), AgentState::Alive);
    assert_eq!(eval_text(&mut agent, &realm, "1;"), one);

    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "function grow() { var a = []; while (true) a = [a, a]; } grow();"
        ),
        Outcome::Terminated(Termination::MemoryBudget)
    );
    assert_eq!(agent.state(), AgentState::Alive);
    assert_eq!(eval_text(&mut agent, &realm, "1;"), one);

    let script = support::script::parse(&mut agent, &realm, "while (true) {}");
    agent.interrupt_handle().interrupt();
    let outcome = agent.evaluate_script(&script).unwrap();
    assert_eq!(
        support::script::describe(&mut agent, &realm, outcome),
        Outcome::Terminated(Termination::Interrupted)
    );
    assert_eq!(agent.state(), AgentState::Alive);
    assert_eq!(eval_text(&mut agent, &realm, "1;"), one);
}

/// An agent that ended in `Terminated(MemoryBudget)` runs again after a collection: with
/// `memory_bytes` the realm's bytes plus 64 KiB, as HOST-70 sizes it, the fill below ends by
/// memory (each iteration adds an element to `a`, whose capacity is charged as it doubles, so the
/// room lasts far fewer iterations than the million steps); `collect()` answers `Normal`, `n`
/// reads the objects the fill made, and a second script that makes ten objects and an Array
/// answers `Normal(10)`: its records take the slots the fill's records left free, at what they
/// own, and the collection credited the elements of the fill's `a` and its frame. The test prints
/// `n`, the room `memory_bytes - heap_bytes()` and `live_bytes()` after the collection. In the
/// gate with the collector at every safe point, where each safe point walks the whole heap, the
/// budget is the realm's bytes plus 16 KiB: the fill still ends in `Terminated(MemoryBudget)`,
/// after fewer objects, and the second script runs as at 64 KiB; the lane `slow`
/// (`SWELL_FULL_COUNTS=1`) gives 64 KiB there too.
#[test]
fn an_agent_that_ended_in_memory_budget_runs_again_after_a_collection() {
    let realm_bytes = {
        let mut agent = support::host::agent().unwrap();
        let _realm = realm(&mut agent);
        agent.heap_bytes().unwrap()
    };
    let kib = if in_the_collect_gate() { 16 } else { 64 };
    let mut limits = limits();
    limits.memory_bytes = realm_bytes + kib * 1024;
    let budget = limits.memory_bytes;
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);

    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var n = 0; function fill() { var a = []; while (true) { a.push({}); n++; } } fill();"
        ),
        Outcome::Terminated(Termination::MemoryBudget)
    );
    assert!(matches!(agent.collect(), Ok(swell::Outcome::Normal(_))));
    let room = budget - agent.heap_bytes().unwrap();
    let live = agent.live_bytes().unwrap();
    let Outcome::Normal(made) = eval_text(&mut agent, &realm, "n;") else {
        panic!("n did not answer");
    };
    println!(
        "objects the fill made: {made}; after the collection: room {room} bytes, \
         live_bytes {live}"
    );
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "(function () { var a = []; for (var i = 0; i < 10; i++) a.push({}); return a.length; })()"
        ),
        Outcome::Normal("10".to_string())
    );
    assert_eq!(agent.state(), AgentState::Alive);
}

/// A host function that calls its argument, catches the termination it answers and returns
/// `undefined`, counting its calls.
fn catcher(cx: &mut Cx<'_, TestHost>, call: HostCall<'_>) -> Completion<Value> {
    cx.host_mut().count += 1;
    let function = call.args.first().copied().unwrap_or(Value::Undefined);
    let answer = cx.call(function, Value::Undefined, &[]).unwrap();
    assert_eq!(answer, Err(Abrupt::Terminated(Termination::StepBudget)));
    // Every later call of the run answers the same termination (decision 10).
    assert_eq!(
        cx.call(function, Value::Undefined, &[]).unwrap(),
        Err(Abrupt::Terminated(Termination::StepBudget))
    );
    Ok(Value::Undefined)
}

/// HOST-72: a host function that catches `Abrupt::Terminated` from `cx.call` and returns
/// `Ok(Value::Undefined)` does not end the termination: the outermost entry answers
/// `Terminated` with the same reason and no further script statement runs.
#[test]
fn host_72_a_host_function_cannot_end_a_termination() {
    let mut limits = limits();
    limits.steps_per_entry = 1_000;
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    define_function(&mut agent, &realm, "catcher", 1, catcher, false);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var after = 0; function spin() { while (true) {} } \
             try { catcher(spin); } finally { after = 2; } after = 1;"
        ),
        Outcome::Terminated(Termination::StepBudget)
    );
    assert_eq!(agent.host().count, 1);
    assert_eq!(
        eval_text(&mut agent, &realm, "after;"),
        Outcome::Normal("0".to_string())
    );
}

/// The argument of `fib` in [`same_script_twice_gives_the_same_steps_and_value`] and the value
/// it answers: `fib(12)`, 144, the full count, which the lane `slow` (`SWELL_FULL_COUNTS=1`)
/// runs in both modes; `fib(7)`, 13, in the gate with the collector at every safe point, where
/// every step collects (the test took 43.3 s there with `fib(12)`, 465 calls a run and four
/// runs, at a load of 100 to 110 on 17 cores, cargo-nextest). `fib(7)` makes 41 calls a run,
/// each with its frames live across the collections of the calls below it, which is what the
/// collect mode looks at, and its steps are compared run for run in the same way.
fn fib_argument_and_value() -> (u32, u32) {
    if in_the_collect_gate() {
        (7, 13)
    } else {
        (12, 144)
    }
}

/// The epic, item 13 (decision 13, non-determinism): the same script run twice, in two fresh
/// agents and twice in one, answers the same value and spends the same steps.
#[test]
fn same_script_twice_gives_the_same_steps_and_value() {
    let (n, fib) = fib_argument_and_value();
    let source = format!(
        "function fib(n) {{ return n < 2 ? n : fib(n - 1) + fib(n - 2); }} \
         var o = {{ a: 1, b: 2 }}; var s = \"\"; for (var k in o) s += k + o[k]; \
         var caught; try {{ null.x; }} catch (e) {{ caught = e.name; }} \
         s + fib({n}) + caught;"
    );
    let source = source.as_str();
    let mut answers = Vec::new();
    for _ in 0..2 {
        let mut agent = agent_with(limits()).unwrap();
        let realm = realm(&mut agent);
        for _ in 0..2 {
            let outcome = eval_text(&mut agent, &realm, source);
            answers.push((outcome, agent.last_entry_steps().unwrap()));
        }
    }
    assert_eq!(
        answers[0].0,
        Outcome::Normal(format!("\"a1b2{fib}TypeError\""))
    );
    assert!(answers[0].1 > 0);
    for answer in &answers {
        assert_eq!(answer, &answers[0]);
    }
}
