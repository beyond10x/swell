//! Names looked up at run time (`docs/design/engine.md`, decision 3, "Names", item 5): a
//! reference inside a `with` body is resolved through the Environment Records from the innermost
//! outward (`#sec-resolvebinding`, `#sec-getidentifierreference`), and each record the lookup
//! visits costs a step, so the walk is bounded by the step budget as every loop is; the records
//! of `with` statements and the arguments objects are charged to the memory budget. Run also
//! with the feature `collect-every-safe-point`.
#![cfg(test)]

use crate::support;

use support::host::{agent_with, limits, realm};
use support::script::{Outcome, describe, eval, eval_text};
use swell::{AgentState, Termination};

/// The number of nested `with` statements of the first test.
const DEPTH: usize = 100;

/// `source` run in a fresh agent with `limits`: its outcome and the steps of its evaluation,
/// read before the outcome is described (which is an entry of its own).
fn steps_with(limits: swell::Limits, source: &str) -> (Outcome, u64) {
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    let outcome = eval(&mut agent, &realm, source);
    let steps = agent.last_entry_steps().unwrap();
    (describe(&mut agent, &realm, outcome), steps)
}

/// `source` run with the test limits: its outcome and its steps.
fn steps_of(source: &str) -> (Outcome, u64) {
    steps_with(limits(), source)
}

/// `DEPTH` nested `with` statements over one object around `body`.
fn nested_with(body: &str) -> String {
    format!(
        "var o = {{}}; var x = 1; {}{body}",
        "with (o) ".repeat(DEPTH)
    )
}

/// `lookup_by_name_charges_a_step_per_record`: in the innermost body of 100 nested `with`
/// statements, a read of `x`, which is looked up by name through the 100 Object Environment
/// Records and the global one, costs at least 100 steps more than the read of a catch parameter
/// in the same place, which is a local slot of the frame; the two scripts have the same
/// instructions but that read.
#[test]
fn lookup_by_name_charges_a_step_per_record() {
    let (by_name, name_steps) = steps_of(&nested_with("try { throw 2; } catch (l) { l + x; }"));
    let (local, local_steps) = steps_of(&nested_with("try { throw 2; } catch (l) { l + l; }"));
    assert_eq!(by_name, Outcome::Normal("3".to_string()));
    assert_eq!(local, Outcome::Normal("4".to_string()));
    let records = u64::try_from(DEPTH).unwrap();
    assert!(
        name_steps >= local_steps + records,
        "{name_steps} steps by name, {local_steps} with a local read"
    );
}

/// `full` steps of a step budget, or a fiftieth of them with the collector at every safe point,
/// where every step collects, in the gate and in the lane `slow` (`SWELL_FULL_COUNTS=1`) alike.
/// The scripts given such a budget loop or recurse without end, so every budget ends them in
/// the same way. With a tenth, 10,000 of 100,000,
/// [`step_budget_ends_a_loop_of_lookups_by_name`] took 18.1 s in that mode at a load of 100 to
/// 110 on 17 cores (cargo-nextest), and with all 100,000 in the lane 12.9 CPU seconds alone
/// (measured on 2026-11-22); with 2,000 it still runs the loop for longer than its budget of
/// 1,000.
fn steps_in_the_collect_mode(full: u64) -> u64 {
    if cfg!(feature = "collect-every-safe-point") {
        full / 50
    } else {
        full
    }
}

/// `step_budget_ends_a_loop_of_lookups_by_name`: a loop that reads and writes a name through a
/// `with` statement forever ends in `Terminated(StepBudget)`, with `last_entry_steps()` at most
/// the budget; so does a single read through more records than the budget has steps.
#[test]
fn step_budget_ends_a_loop_of_lookups_by_name() {
    for steps in [10_u64, 1_000, steps_in_the_collect_mode(100_000)] {
        let mut limits = limits();
        limits.steps_per_entry = steps;
        let mut agent = agent_with(limits).unwrap();
        let realm = realm(&mut agent);
        assert_eq!(
            eval_text(
                &mut agent,
                &realm,
                "var o = { n: 0 }; with (o) { while (true) { n = n + 1; } }"
            ),
            Outcome::Terminated(Termination::StepBudget)
        );
        let spent = agent.last_entry_steps().unwrap();
        assert!(spent <= steps, "{spent} > {steps}");
        assert_eq!(agent.state(), AgentState::Alive);
    }
    // 300 records (each `with` takes two levels of the parser nesting limit of the test limits),
    // a budget the 300 `PushWith` instructions and their operands fit in, but not
    // the read of `x` through every record.
    let mut limits = limits();
    let source = format!("var o = {{}}; {}x;", "with (o) ".repeat(300));
    let (outcome, whole) = steps_with(limits, &source);
    assert_eq!(outcome, Outcome::Throw("ReferenceError".to_string()));
    limits.steps_per_entry = whole - 100;
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(&mut agent, &realm, &source),
        Outcome::Terminated(Termination::StepBudget)
    );
}

/// `memory_budget_ends_a_loop_that_keeps_arguments_objects_and_with_records`: a loop that keeps
/// every mapped and unmapped arguments object it makes, or a closure over every Object
/// Environment Record a `with` statement makes, ends in `Terminated(MemoryBudget)` with
/// `heap_bytes()` at most the budget, and the agent stays `Alive` (what the global `kept` holds
/// stays reachable, so a next script may find no room left).
#[test]
fn memory_budget_ends_a_loop_that_keeps_arguments_objects_and_with_records() {
    let base = {
        let mut agent = agent_with(limits()).unwrap();
        let _realm = realm(&mut agent);
        agent.collect().unwrap();
        agent.heap_bytes().unwrap()
    };
    for source in [
        "function f(a, b) { return arguments; } var kept = []; while (true) kept.push(f(1, 2, 3));",
        "function f(a, b) { 'use strict'; return arguments; } var kept = [];
         while (true) kept.push(f(1, 2, 3));",
        "var kept = []; while (true) { with ({}) { kept.push(function () { return x; }); } }",
    ] {
        let mut limits = limits();
        limits.memory_bytes = base + 65_536;
        limits.steps_per_entry = u64::MAX;
        let budget = limits.memory_bytes;
        let mut agent = agent_with(limits).unwrap();
        let realm = realm(&mut agent);
        assert_eq!(
            eval_text(&mut agent, &realm, source),
            Outcome::Terminated(Termination::MemoryBudget),
            "{source}"
        );
        let bytes = agent.heap_bytes().unwrap();
        assert!(bytes <= budget, "{bytes} > {budget}");
        assert_eq!(agent.state(), AgentState::Alive);
    }
}
