//! The budgets of `Limits` (`docs/design/host-interface.md`, decision 2; `engine.md`, decisions
//! 3 and 4): the memory budget of `with_cx`, and for scripts the step budget, `frame_depth`,
//! `reentry_depth`, `string_length` and the Rust stack `Limits::min_stack_bytes` promises.
//!
//! Every entry names a realm (decision 7), so each agent here holds one, and its budget is the
//! bytes the realm takes plus the room the test gives. Run also with the feature
//! `collect-every-safe-point`.
#![cfg(test)]

mod support;

use std::cell::Cell;

use support::host::{
    TestHost, agent_with, global, is_error_of, limits, object, realm, stack_with_room,
};
use support::script::{Outcome, define_function, eval, eval_text, run};
use swell::{
    Abrupt, Agent, AgentState, Brand, Completion, Cx, HostCall, HostData, HostObjectSpec, Limits,
    Outcome as HostOutcome, Parsed, Prototype, Realm, RealmRef, Source, Termination, Tracer, Value,
};

/// The bytes an agent with the test limits holds after `create_realm`, which leaves nothing
/// for a collection to free.
fn realm_bytes() -> u64 {
    let mut agent = support::host::agent().unwrap();
    let _realm = support::host::realm(&mut agent);
    let made = agent.heap_bytes().unwrap();
    agent.collect().unwrap();
    assert_eq!(agent.heap_bytes().unwrap(), made);
    made
}

/// The epic's item 9: an allocation that takes the agent over its memory budget ends the run
/// with `Terminated(MemoryBudget)`. A `with_cx` whose `new_string` of 2,000 code units passes
/// a budget of 1,024 bytes beyond the realm.
#[test]
fn memory_budget_ends_with_cx_in_memory_budget() {
    let mut limits = support::host::limits();
    limits.memory_bytes = realm_bytes() + 1_024;
    let budget = limits.memory_bytes;
    let mut agent = support::host::agent_with(limits).unwrap();
    let realm = support::host::realm(&mut agent);

    let text = "x".repeat(2_000);
    let outcome = agent
        .with_cx(&realm, |cx| {
            let first = cx.new_string(&text).map(|_| ());
            // Termination is sticky for the run: a later allocation answers the same.
            let second = cx.new_string("y").map(|_| ());
            (first, second)
        })
        .unwrap();
    assert_eq!(
        outcome.0,
        Err(Abrupt::Terminated(Termination::MemoryBudget))
    );
    assert_eq!(
        outcome.1,
        Err(Abrupt::Terminated(Termination::MemoryBudget))
    );
    assert!(agent.heap_bytes().unwrap() <= budget);

    // The agent stays usable, and the next entry has the budget again.
    assert_eq!(agent.state(), AgentState::Alive);
    let small = agent
        .with_cx(&realm, |cx| cx.new_string("small").map(|_| ()))
        .unwrap();
    assert_eq!(small, Ok(()));
    assert!(agent.heap_bytes().unwrap() <= budget);
}

/// The UTF-16 form and host objects are charged the same way.
#[test]
fn memory_budget_charges_utf16_strings_and_host_data() {
    let mut limits = support::host::limits();
    limits.memory_bytes = realm_bytes() + 1_024;

    let mut agent = support::host::agent_with(limits).unwrap();
    let realm = support::host::realm(&mut agent);
    let units = vec![0x3042_u16; 600];
    let outcome = agent
        .with_cx(&realm, |cx| cx.new_string_utf16(&units).map(|_| ()))
        .unwrap();
    assert_eq!(outcome, Err(Abrupt::Terminated(Termination::MemoryBudget)));

    struct Big {
        _bytes: [u8; 2_048],
    }
    impl HostData for Big {
        fn trace(&self, _tracer: &mut Tracer<'_>) {}
    }
    let outcome = agent
        .with_cx(&realm, |cx| {
            cx.new_host_object(HostObjectSpec {
                prototype: Prototype::Null,
                brand: Brand(1),
                data: Some(Box::new(Big { _bytes: [0; 2_048] })),
            })
            .unwrap()
            .map(|_| ())
        })
        .unwrap();
    assert_eq!(outcome, Err(Abrupt::Terminated(Termination::MemoryBudget)));
    assert_eq!(agent.state(), AgentState::Alive);
}

/// With no room beyond the realm every allocation ends the run, nothing more is charged, and
/// the agent stays usable. (`memory_bytes` 0 refuses the realm itself: HOST-12.)
#[test]
fn memory_budget_of_zero_refuses_every_allocation() {
    let mut limits = support::host::limits();
    limits.memory_bytes = realm_bytes();
    let mut agent = support::host::agent_with(limits).unwrap();
    let realm = support::host::realm(&mut agent);
    let outcome = agent
        .with_cx(&realm, |cx| cx.new_string("").map(|_| ()))
        .unwrap();
    assert_eq!(outcome, Err(Abrupt::Terminated(Termination::MemoryBudget)));
    assert_eq!(agent.heap_bytes().unwrap(), realm_bytes());
    assert_eq!(agent.state(), AgentState::Alive);
}

/// The test limits with `steps_per_entry` `steps`.
fn with_steps(steps: u64) -> Limits {
    let mut limits = limits();
    limits.steps_per_entry = steps;
    limits
}

/// HOST-6: the script `while (true) {}` with `steps_per_entry` N ends in
/// `Terminated(StepBudget)`, and `last_entry_steps()` then reads at most N.
#[test]
fn host_6_endless_loop_ends_in_step_budget() {
    for steps in [1_u64, 10, 1_000] {
        let mut agent = agent_with(with_steps(steps)).unwrap();
        let realm = realm(&mut agent);
        assert_eq!(
            eval_text(&mut agent, &realm, "while (true) {}"),
            Outcome::Terminated(Termination::StepBudget)
        );
        let spent = agent.last_entry_steps().unwrap();
        assert!(spent <= steps, "{spent} > {steps}");
        assert_eq!(agent.state(), AgentState::Alive);
    }
}

/// HOST-7: after an entry ended in `Terminated(StepBudget)`, the next outermost entry has the
/// whole budget again: a script of fewer than N steps answers `Normal`.
#[test]
fn host_7_next_entry_gets_the_whole_budget() {
    let mut agent = agent_with(with_steps(100)).unwrap();
    let realm = realm(&mut agent);
    let counting = "var i = 0; while (i < 5) i++; i;";
    assert_eq!(
        eval_text(&mut agent, &realm, counting),
        Outcome::Normal("5".to_string())
    );
    let alone = agent.last_entry_steps().unwrap();
    assert!(alone < 100);
    assert_eq!(
        eval_text(&mut agent, &realm, "while (true) {}"),
        Outcome::Terminated(Termination::StepBudget)
    );
    assert_eq!(
        eval_text(&mut agent, &realm, counting),
        Outcome::Normal("5".to_string())
    );
    assert_eq!(agent.last_entry_steps(), Ok(alone));
}

thread_local! {
    /// The realm a host function of these tests parses in.
    static REALM: Cell<Option<RealmRef>> = const { Cell::new(None) };
    /// Whether a host function of these tests has parsed its deep source yet.
    static PARSED: Cell<bool> = const { Cell::new(false) };
}

/// Make `realm` the realm the host functions of these tests parse in.
fn remember(agent: &mut Agent<TestHost>, realm: &Realm) {
    let realm_ref = agent
        .with_cx(realm, |cx| cx.realm_ref(realm).unwrap())
        .unwrap();
    REALM.with(|cell| cell.set(Some(realm_ref)));
}

/// A host function that parses `while (true) {}` and evaluates it through `cx`.
fn nested_loop(cx: &mut Cx<'_, TestHost>, _call: HostCall<'_>) -> Completion<Value> {
    let realm = REALM.with(Cell::get).unwrap();
    let Parsed::Parsed(script) = cx
        .parse_script(realm, Source::Utf8("while (true) {}"), None)
        .unwrap()
    else {
        panic!("the nested script did not parse");
    };
    cx.evaluate_script(script).unwrap()
}

/// HOST-8: a host function that evaluates `while (true) {}` through `cx.evaluate_script` inside
/// an outermost entry makes that entry answer `Terminated(StepBudget)` with `last_entry_steps()`
/// at most N: a nested entry gets no budget of its own.
#[test]
fn host_8_nested_entry_spends_the_outer_budget() {
    let steps = 500;
    let mut agent = agent_with(with_steps(steps)).unwrap();
    let realm = realm(&mut agent);
    remember(&mut agent, &realm);
    define_function(&mut agent, &realm, "nestedLoop", 0, nested_loop, false);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var before = 1; nestedLoop(); before = 2;"
        ),
        Outcome::Terminated(Termination::StepBudget)
    );
    assert!(agent.last_entry_steps().unwrap() <= steps);
    assert_eq!(
        eval_text(&mut agent, &realm, "before;"),
        Outcome::Normal("1".to_string())
    );
}

/// HOST-9: with `steps_per_entry` 0, the script `1` answers `Terminated(StepBudget)` and
/// `last_entry_steps()` reads 0: an entry that would spend a step ends before spending it.
#[test]
fn host_9_zero_steps_ends_before_the_first() {
    let mut agent = agent_with(with_steps(0)).unwrap();
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(&mut agent, &realm, "1;"),
        Outcome::Terminated(Termination::StepBudget)
    );
    assert_eq!(agent.last_entry_steps(), Ok(0));
}

/// Whether the thrown value of `outcome` is a `RangeError` of `realm`.
fn is_range_error_of(
    agent: &mut Agent<TestHost>,
    realm: &Realm,
    outcome: HostOutcome<swell::Root>,
) -> bool {
    let HostOutcome::Throw(thrown) = outcome else {
        panic!("expected a throw, got {outcome:?}");
    };
    let global_root = global(agent, realm);
    agent
        .with_cx(realm, |cx| {
            let global = object(cx, &global_root);
            let thrown = cx.get(&thrown).unwrap();
            is_error_of(cx, global, thrown, "RangeError")
        })
        .unwrap()
}

/// HOST-13: with `frame_depth` D, `var n = 0; function f() { n++; f(); } f();` answers `Throw`
/// of a `RangeError` of its realm; `n` then answers `D - 1`, the script's own context being the
/// first; and `try { f(); } catch (e) {}` answers `Normal`.
#[test]
fn host_13_frame_depth_counts_the_script_context() {
    for depth in [1_u32, 2, 50] {
        let mut limits = limits();
        limits.frame_depth = depth;
        let mut agent = agent_with(limits).unwrap();
        let realm = realm(&mut agent);
        let outcome = eval(
            &mut agent,
            &realm,
            "var n = 0; function f() { n++; f(); } f();",
        );
        assert!(
            is_range_error_of(&mut agent, &realm, outcome),
            "depth {depth}"
        );
        assert_eq!(
            eval_text(&mut agent, &realm, "n;"),
            Outcome::Normal((depth - 1).to_string()),
            "depth {depth}"
        );
        assert_eq!(
            eval_text(&mut agent, &realm, "try { f(); } catch (e) {}"),
            Outcome::Normal("undefined".to_string())
        );
    }
}

/// HOST-14: with `frame_depth` 0, the script `1` answers `Throw` of a `RangeError` of its
/// realm.
#[test]
fn host_14_zero_frame_depth_throws_range_error() {
    let mut limits = limits();
    limits.frame_depth = 0;
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    let outcome = eval(&mut agent, &realm, "1;");
    assert!(is_range_error_of(&mut agent, &realm, outcome));
}

/// HOST-20: with `string_length` L and `memory_bytes` at least 8 × L, `var s = "x"; while
/// (true) s = s + s;` answers `Throw` of a `RangeError` raised by the concatenation whose
/// result would exceed L code units: `s` then holds the longest String it made.
#[test]
fn host_20_doubling_a_string_past_string_length_throws() {
    let mut limits = limits();
    limits.string_length = 1_000;
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    let outcome = eval(&mut agent, &realm, "var s = \"x\"; while (true) s = s + s;");
    assert!(is_range_error_of(&mut agent, &realm, outcome));
    assert_eq!(
        eval_text(&mut agent, &realm, "s.length;"),
        Outcome::Normal("512".to_string())
    );
}

/// HOST-22: with `string_length` 0, the script `"a" + "b"` answers `Throw` of a `RangeError`.
#[test]
fn host_22_zero_string_length_throws_on_concatenation() {
    let mut limits = limits();
    limits.string_length = 0;
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    let outcome = eval(&mut agent, &realm, "\"a\" + \"b\";");
    assert!(is_range_error_of(&mut agent, &realm, outcome));
}

/// `full` steps of a step budget, or a fiftieth of them in the gate with the collector at every
/// safe point, where every step collects; the lane `slow` (`SWELL_FULL_COUNTS=1`) runs `full`
/// there too. The scripts given such a budget loop or recurse without end, so every budget
/// ends them in the same way. With a tenth, 10,000 of 100,000,
/// [`catching_the_depth_error_and_recursing_again_ends_in_step_budget`] took 22.5 s in that
/// mode at a load of 100 to 110 on 17 cores (cargo-nextest); its 2,000 steps still reach the
/// `RangeError` at `frame_depth` 20, which takes a few hundred.
fn steps_in_the_collect_gate(full: u64) -> u64 {
    if in_the_collect_gate() {
        full / 50
    } else {
        full
    }
}

/// Whether this run is the gate's with the collector at every safe point: the feature
/// `collect-every-safe-point`, without `SWELL_FULL_COUNTS=1` (which the lane `slow` sets).
fn in_the_collect_gate() -> bool {
    cfg!(feature = "collect-every-safe-point")
        && !std::env::var("SWELL_FULL_COUNTS").is_ok_and(|value| value == "1")
}

/// The epic, item 7 (`engine.md`, decision 3): a function that calls itself without end throws
/// a `RangeError` at `frame_depth`; a script that catches it and recurses again ends in
/// `Terminated(StepBudget)`, which it cannot catch.
#[test]
fn catching_the_depth_error_and_recursing_again_ends_in_step_budget() {
    let mut limits = limits();
    limits.frame_depth = 20;
    limits.steps_per_entry = steps_in_the_collect_gate(100_000);
    assert_eq!(
        run(
            limits,
            "var caught = 0; function f() { try { f(); } catch (e) { caught++; f(); } } f();"
        ),
        Outcome::Terminated(Termination::StepBudget)
    );
}

/// A host function that calls its first argument through `cx.call`: a re-entry. At the
/// deepest one, the call that would pass `reentry_depth` throws; the first time that happens
/// the host function parses a source nested `parser_nesting` levels through `cx.parse_script`.
fn reenter(cx: &mut Cx<'_, TestHost>, call: HostCall<'_>) -> Completion<Value> {
    let function = call.args.first().copied().unwrap_or(Value::Undefined);
    let answer = cx.call(function, Value::Undefined, &[]).unwrap();
    if answer.is_err() && !PARSED.with(Cell::get) {
        PARSED.with(|cell| cell.set(true));
        parse_deep(cx);
    }
    answer
}

/// A host function that parses a source nested `parser_nesting` levels.
fn parse_deep_function(cx: &mut Cx<'_, TestHost>, _call: HostCall<'_>) -> Completion<Value> {
    parse_deep(cx);
    Ok(Value::Undefined)
}

/// Parse 1,000 nested brackets through `cx.parse_script`.
fn parse_deep(cx: &mut Cx<'_, TestHost>) {
    let realm = REALM.with(Cell::get).unwrap();
    let source = format!("{}0{};", "(".repeat(1_000), ")".repeat(1_000));
    let parsed = cx.parse_script(realm, Source::Utf8(&source), None).unwrap();
    assert!(!matches!(parsed, Parsed::Terminated(_)));
    cx.host_mut().count += 1;
}

/// The script that reaches `reentry_depth` through a host function: each level the script calls
/// `reenter` directly (no re-entry) and `reenter` calls back through `cx.call` (a re-entry).
const THROUGH_HOST: &str = "var n = 0; function f() { n++; reenter(f); } reenter(f);";

/// The script that reaches it through a getter: each getter reads the property again from its
/// own code, a call from Rust into code nested in the one before.
const THROUGH_GETTER: &str = "var g = 0, parsed = false; \
    var o = { get x() { g++; try { return o.x; } catch (e) { if (!parsed) { parsed = true; parseDeep(); } throw e; } } }; \
    o.x;";

/// `reentry_depth` (decision 2; decision 14): what counts is a call from Rust into code made
/// while code runs. A host function that script calls directly is not one, so it does not use
/// up a level by itself; its `cx.call` back into script is one. With `reentry_depth` R the
/// R + 1st re-entry throws a `RangeError`, so `f` runs R times through `reenter`: had `reenter`
/// used up a level by itself, `f` would run R / 2 times. A getter is called from Rust by the
/// property read, a run loop nested in the running one: each getter is a re-entry, R getters
/// run, and the deepest one still calls the host function `parseDeep` directly (with R 0 no
/// getter runs and `parseDeep` is not reached).
#[test]
fn reentry_depth_counts_calls_from_rust_not_host_functions_called_by_script() {
    for depth in [0_u32, 1, 5, 40] {
        let mut limits = limits();
        limits.reentry_depth = depth;
        let mut agent = agent_with(limits).unwrap();
        let realm = realm(&mut agent);
        remember(&mut agent, &realm);
        PARSED.with(|cell| cell.set(true));
        define_function(&mut agent, &realm, "reenter", 1, reenter, false);
        define_function(
            &mut agent,
            &realm,
            "parseDeep",
            0,
            parse_deep_function,
            false,
        );
        let outcome = eval(&mut agent, &realm, THROUGH_HOST);
        assert!(
            is_range_error_of(&mut agent, &realm, outcome),
            "depth {depth}"
        );
        assert_eq!(
            eval_text(&mut agent, &realm, "n;"),
            Outcome::Normal(depth.to_string()),
            "depth {depth}"
        );
        let outcome = eval(&mut agent, &realm, THROUGH_GETTER);
        assert!(
            is_range_error_of(&mut agent, &realm, outcome),
            "depth {depth}"
        );
        assert_eq!(
            eval_text(&mut agent, &realm, "g;"),
            Outcome::Normal(depth.to_string()),
            "depth {depth}"
        );
        assert_eq!(agent.host().count, u32::from(depth > 0), "depth {depth}");
        assert_eq!(agent.state(), AgentState::Alive);
    }
}

/// The variable that tells a child process which stack test to run.
const STACK_CHILD: &str = "SWELL_STACK_CHILD";

/// Run `body` on a thread of `stack` bytes in a child process that runs only the test `test`. A
/// stack overflow aborts the process, which no test can catch in its own process; in the child
/// it fails this test alone, with the child's output.
fn on_bounded_stack(test: &str, stack: usize, body: fn()) -> Result<(), String> {
    let done = format!("{STACK_CHILD}: {test} on {stack} bytes done");
    if std::env::var_os(STACK_CHILD).is_some() {
        std::thread::Builder::new()
            .stack_size(stack)
            .spawn(body)
            .map_err(|e| e.to_string())?
            .join()
            .map_err(|_| format!("{test}: the body failed"))?;
        println!("{done}");
        return Ok(());
    }
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let out = std::process::Command::new(exe)
        .args([test, "--exact", "--nocapture", "--test-threads=1"])
        .env(STACK_CHILD, "1")
        .output()
        .map_err(|e| e.to_string())?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    if out.status.success() && stdout.contains(&done) {
        return Ok(());
    }
    Err(format!(
        "{test} on a stack of {stack} bytes: {}\n{stdout}\n{}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    ))
}

/// The `reentry_depth` of the stack test: the full depth, 1,000, in the default mode and in the
/// lane `slow`; in the gate with the collector at every safe point ([`in_the_collect_gate`]),
/// 100. The stack bound is a property of the default mode's frames (the collector marks from a
/// worklist, so its own stack does not grow with the depth). What the collect mode looks for, a
/// value held without a root, shows at any depth: at 100 every safe point of the deepest levels
/// still collects while 100 re-entries, each with its Rust frame and its activation, are live.
/// A collection traces every activation, so the cost grows with the square of the depth: at
/// 1,000 the test took 30.4 s in that mode (21.2 CPU s) against 0.10 s in the default one
/// (measured).
fn stack_reentry() -> u32 {
    if in_the_collect_gate() { 100 } else { 1_000 }
}

/// The limits of the stack test: `frame_depth` 10,000, `reentry_depth` [`stack_reentry`] and
/// `parser_nesting` 1,000, with step and memory budgets no run of it reaches.
fn stack_limits() -> Limits {
    let mut limits = limits();
    limits.frame_depth = 10_000;
    limits.reentry_depth = stack_reentry();
    limits.parser_nesting = 1_000;
    limits.steps_per_entry = 1 << 40;
    limits.memory_bytes = 1 << 30;
    limits
}

/// The body of the stack test: both scripts reach `reentry_depth` and parse a source nested
/// `parser_nesting` levels at the deepest re-entry.
fn reach_every_bound() {
    let mut agent = agent_with(stack_limits()).unwrap();
    let realm = realm(&mut agent);
    remember(&mut agent, &realm);
    define_function(&mut agent, &realm, "reenter", 1, reenter, false);
    define_function(
        &mut agent,
        &realm,
        "parseDeep",
        0,
        parse_deep_function,
        false,
    );
    PARSED.with(|cell| cell.set(false));
    let outcome = eval(&mut agent, &realm, THROUGH_HOST);
    assert!(is_range_error_of(&mut agent, &realm, outcome));
    assert_eq!(
        eval_text(&mut agent, &realm, "n;"),
        Outcome::Normal(stack_reentry().to_string())
    );
    let outcome = eval(&mut agent, &realm, THROUGH_GETTER);
    assert!(is_range_error_of(&mut agent, &realm, outcome));
    assert_eq!(
        eval_text(&mut agent, &realm, "g;"),
        Outcome::Normal(stack_reentry().to_string())
    );
    assert_eq!(
        agent.host().count,
        2,
        "both scripts parsed at their deepest"
    );
}

/// HOST-17's bound, held here for the limits of the story (`Limits::min_stack_bytes`, decision
/// 2): a thread of seven eighths of `min_stack_bytes()` ([`stack_with_room`]) for
/// `frame_depth` 10,000, `reentry_depth` 1,000 and `parser_nesting` 1,000 runs a script that
/// reaches `reentry_depth` through host functions, and one that reaches it through getters,
/// each parsing 1,000 nested brackets at the deepest, and ends without a stack overflow:
/// `min_stack_bytes()` is enough, with an eighth of it to spare. In the gate's collect mode the
/// depth is [`stack_reentry`], and the thread has seven eighths of `min_stack_bytes()` of that
/// depth.
#[test]
fn min_stack_bytes_is_enough() {
    let stack = stack_with_room(stack_limits());
    on_bounded_stack("min_stack_bytes_is_enough", stack, reach_every_bound).unwrap();
}
