//! The state a terminated run leaves (`docs/design/host-interface.md`, decision 10;
//! `docs/design/engine.md`, decision 3): no `catch` or `finally` block runs, the frames are
//! dropped, the agent stays `Alive` after every termination but `EngineFault`, a host function
//! cannot end a termination, and the same script run twice spends the same steps. Run also with
//! the feature `collect-every-safe-point`.
#![cfg(test)]

use crate::support;

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

/// The room the host of [`an_agent_that_ended_in_memory_budget_runs_again_after_a_collection`]
/// holds charged during the fill and gives back after the collection: 16 KiB.
///
/// What it rests on, measured on `7a7bf0a` in both modes: since the parse and compile step is
/// charged to the budget (the copy of the source, the tree, the scope analysis, the compile
/// step's scratch and the code), the second script needs 11,563 bytes of room above
/// `heap_bytes()` to answer `Normal(10)` (its parse peaks there: the tree 3,879 bytes, the
/// compile step's scratch with the bound of one node's expansion, 2,240 bytes; it keeps 2,078
/// in its record, and its run peaks at 1,632 more), and `n;` before it keeps 645 bytes in its
/// record: 12,208 bytes, 4,176 below this room. Without it the room after the collection was
/// what the collection credited, the elements of the fill's `a` and its frame (9,291 bytes in
/// the default mode, 12,363 at 16 KiB), which depends on where the fill's last doubling
/// stopped: the second script then ended in `Terminated(MemoryBudget)` in the default mode and
/// passed by 155 bytes in the collect gate.
const AFTER_FILL_ROOM: u64 = 16 * 1024;

/// An agent that ended in `Terminated(MemoryBudget)` runs again after a collection: with
/// `memory_bytes` the realm's bytes plus 64 KiB, as HOST-70 sizes it, plus [`AFTER_FILL_ROOM`],
/// which the host holds charged (`charge_host_bytes`) before the fill, the fill below ends by
/// memory with 64 KiB above the realm (each iteration adds an element to `a`, whose capacity is
/// charged as it doubles, so the room lasts far fewer iterations than the million steps);
/// `collect()` answers `Normal`, the host gives its charge back (`credit_host_bytes`), `n` reads
/// the objects the fill made, and a second script that makes ten objects and an Array answers
/// `Normal(10)`: its records take the slots the fill's records left free, at what they own. The
/// room for it is bounded from below whatever the fill's doublings did: `heap_bytes()` is at
/// most `memory_bytes` after the termination (HOST-171), the collection lowers it, and the
/// credit lowers it by [`AFTER_FILL_ROOM`], so at least that much is free. The test prints `n`,
/// the room `memory_bytes - heap_bytes()` and `live_bytes()` after the collection and the
/// credit. In the gate with the collector at every safe point, where each safe point walks the
/// whole heap, the fill has the realm's bytes plus 16 KiB: it still ends in
/// `Terminated(MemoryBudget)`, after fewer objects, and the second script runs as at 64 KiB;
/// the lane `slow` (`SWELL_FULL_COUNTS=1`) gives 64 KiB there too.
#[test]
fn an_agent_that_ended_in_memory_budget_runs_again_after_a_collection() {
    let realm_bytes = {
        let mut agent = support::host::agent().unwrap();
        let _realm = realm(&mut agent);
        agent.heap_bytes().unwrap()
    };
    let kib = if in_the_collect_gate() { 16 } else { 64 };
    let mut limits = limits();
    limits.memory_bytes = realm_bytes + kib * 1024 + AFTER_FILL_ROOM;
    let budget = limits.memory_bytes;
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    let held = agent
        .with_cx(&realm, |cx| cx.charge_host_bytes(AFTER_FILL_ROOM))
        .unwrap()
        .unwrap();

    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var n = 0; function fill() { var a = []; while (true) { a.push({}); n++; } } fill();"
        ),
        Outcome::Terminated(Termination::MemoryBudget)
    );
    assert!(matches!(agent.collect(), Ok(swell::Outcome::Normal(_))));
    agent.credit_host_bytes(held).unwrap();
    let room = budget - agent.heap_bytes().unwrap();
    assert!(room >= AFTER_FILL_ROOM, "room {room}");
    let live = agent.live_bytes().unwrap();
    let Outcome::Normal(made) = eval_text(&mut agent, &realm, "n;") else {
        panic!("n did not answer");
    };
    println!(
        "objects the fill made: {made}; after the collection and the credit: room {room} bytes, \
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

/// The room HOST-190 states for the script `1`: 3 KiB. Measured once the compile step charged
/// what a node's expansion holds as it is made, in both modes: `1` needs 2,228 bytes of room
/// above `heap_bytes()` to answer and `1 + 1` needs 2,322 (3,440 and 3,895 before, when the
/// step charged a bound of 2,240 bytes for one node's expansion that takes 224).
const ROOM_FOR_ONE: u64 = 3 * 1024;

/// What the host of [`after_a_fill`] holds charged during the fill and gives back after it.
const HELD_DURING_FILL: u64 = 16 * 1024;

/// An agent after `var a = []; while (true) a.push({});`, a loop that keeps every object it
/// makes, ended in `Terminated(MemoryBudget)`, and a `collect()`: `memory_bytes` is the realm's
/// bytes, 64 KiB and [`HELD_DURING_FILL`], which the host charged (`charge_host_bytes`) before
/// the loop. Before the loop the host kept a root of the String `"kept"`. The agent, its realm,
/// the host's charge, the root and `memory_bytes`.
fn after_a_fill() -> (
    swell::Agent<TestHost>,
    swell::Realm,
    swell::HostBytes,
    swell::Root,
    u64,
) {
    let realm_bytes = {
        let mut agent = support::host::agent().unwrap();
        let _realm = realm(&mut agent);
        agent.heap_bytes().unwrap()
    };
    let mut limits = limits();
    limits.memory_bytes = realm_bytes + 64 * 1024 + HELD_DURING_FILL;
    let budget = limits.memory_bytes;
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    let swell::Outcome::Normal(kept) = support::script::eval(&mut agent, &realm, "'kept'") else {
        panic!("'kept' did not answer");
    };
    let held = agent
        .with_cx(&realm, |cx| cx.charge_host_bytes(HELD_DURING_FILL))
        .unwrap()
        .unwrap();
    assert_eq!(
        eval_text(&mut agent, &realm, "var a = []; while (true) a.push({});"),
        Outcome::Terminated(Termination::MemoryBudget)
    );
    assert!(agent.heap_bytes().unwrap() <= budget);
    assert!(matches!(agent.collect(), Ok(swell::Outcome::Normal(_))));
    (agent, realm, held, kept, budget)
}

/// The host charges what is free of `budget` past `room`, so that `memory_bytes - heap_bytes()`
/// reads exactly `room`; the charge.
fn leave_room(
    agent: &mut swell::Agent<TestHost>,
    realm: &swell::Realm,
    budget: u64,
    room: u64,
) -> swell::HostBytes {
    let free = budget - agent.heap_bytes().unwrap();
    let charge = agent
        .with_cx(realm, |cx| cx.charge_host_bytes(free - room))
        .unwrap()
        .unwrap();
    assert_eq!(budget - agent.heap_bytes().unwrap(), room, "the room left");
    charge
}

/// HOST-188: after the fill of [`after_a_fill`] ended in `Terminated(MemoryBudget)` and the host
/// charged all the room left, so that `memory_bytes - heap_bytes()` reads 0, the calls that take
/// no memory answer: `state()` reads `Alive`, `collect()` answers `Normal`, the root the host
/// kept reads `"kept"`, the script `1` answers `Terminated(MemoryBudget)` (HOST-189), and
/// dropping the root, `credit_host_bytes` of both charges and dropping the agent end without a
/// fault or a panic.
#[test]
fn host_188_calls_that_take_no_memory_answer_with_a_room_of_0() {
    let (mut agent, realm, held, kept, budget) = after_a_fill();
    let free = budget - agent.heap_bytes().unwrap();
    let rest = leave_room(&mut agent, &realm, budget, 0);
    assert_eq!(agent.state(), AgentState::Alive);
    assert!(matches!(agent.collect(), Ok(swell::Outcome::Normal(_))));
    assert_eq!(
        budget - agent.heap_bytes().unwrap(),
        0,
        "the room after collect"
    );
    let read = agent
        .with_cx(&realm, |cx| {
            let value = cx.get(&kept).unwrap();
            support::script::show(cx, value)
        })
        .unwrap();
    assert_eq!(read, "\"kept\"");
    assert_eq!(
        eval_text(&mut agent, &realm, "1;"),
        Outcome::Terminated(Termination::MemoryBudget)
    );
    assert_eq!(agent.state(), AgentState::Alive);
    drop(kept);
    assert_eq!(agent.credit_host_bytes(rest).unwrap(), free);
    assert_eq!(agent.credit_host_bytes(held).unwrap(), HELD_DURING_FILL);
    assert_eq!(agent.state(), AgentState::Alive);
    drop(agent);
}

/// The script `1` in `realm`, parsed and evaluated: what it answered and the steps it spent,
/// read before the answer is described (a description that converts a value begins an entry).
fn one(agent: &mut swell::Agent<TestHost>, realm: &swell::Realm) -> (Outcome, u64) {
    let outcome = support::script::eval(agent, realm, "1;");
    let steps = agent.last_entry_steps().unwrap();
    (support::script::describe(agent, realm, outcome), steps)
}

/// HOST-189 and HOST-190 after a loop that keeps every object it makes ended in
/// `Terminated(MemoryBudget)` ([`after_a_fill`]): with 1,000 bytes of room, read as
/// `memory_bytes - heap_bytes()` with no trial entry, the script `1` answers
/// `Terminated(MemoryBudget)` twice in a row with the same `heap_bytes()` after each, and
/// `parse_script` of it answers `Parsed::Terminated(MemoryBudget)` with that reading; `state()`
/// reads `Alive` after each and `collect()` answers `Normal`. After the host gave back what it
/// charged during the fill and the room it took (`credit_host_bytes`), the room is at least
/// those 16 KiB whatever the loop's last refused growth left; with the room set to
/// [`ROOM_FOR_ONE`], `1` answers `Normal(1)` with the steps it spends in an agent that never
/// ended in the memory budget and has the same room.
#[test]
fn host_189_190_one_runs_after_a_memory_budget_termination_once_the_host_made_room() {
    let (mut agent, realm, held, _kept, budget) = after_a_fill();
    let small = leave_room(&mut agent, &realm, budget, 1_000);
    let mut after = Vec::new();
    for _ in 0..2 {
        assert_eq!(
            eval_text(&mut agent, &realm, "1;"),
            Outcome::Terminated(Termination::MemoryBudget)
        );
        assert_eq!(agent.state(), AgentState::Alive);
        after.push(agent.heap_bytes().unwrap());
    }
    assert_eq!(after[0], after[1], "heap_bytes() after each");
    assert!(matches!(
        agent
            .parse_script(&realm, swell::Source::Utf8("1;"), None)
            .unwrap(),
        swell::Parsed::Terminated(Termination::MemoryBudget)
    ));
    assert_eq!(agent.heap_bytes().unwrap(), after[0]);
    assert_eq!(agent.state(), AgentState::Alive);
    assert!(matches!(agent.collect(), Ok(swell::Outcome::Normal(_))));

    agent.credit_host_bytes(small).unwrap();
    agent.credit_host_bytes(held).unwrap();
    let room = budget - agent.heap_bytes().unwrap();
    assert!(room >= HELD_DURING_FILL, "room {room}");
    let _rest = leave_room(&mut agent, &realm, budget, ROOM_FOR_ONE);
    let (answer, steps) = one(&mut agent, &realm);
    assert_eq!(answer, Outcome::Normal("1".to_string()));
    assert!(steps > 0);

    let fresh_budget = limits().memory_bytes;
    let mut fresh = agent_with(limits()).unwrap();
    let fresh_realm = support::host::realm(&mut fresh);
    assert!(fresh.collect().is_ok());
    let _fresh_rest = leave_room(&mut fresh, &fresh_realm, fresh_budget, ROOM_FOR_ONE);
    assert_eq!(one(&mut fresh, &fresh_realm), (answer, steps));
    println!(
        "after the fill: room {room} bytes once the host gave its charges back; 1 spent {steps} steps"
    );
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
