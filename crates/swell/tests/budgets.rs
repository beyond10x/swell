//! The budgets of `Limits` (`docs/design/host-interface.md`, decision 2; `engine.md`, decisions
//! 3 and 4): the memory budget of `with_cx`, and for scripts the step budget, `frame_depth`,
//! `reentry_depth`, `string_length` and the Rust stack `Limits::min_stack_bytes` promises.
//!
//! Every entry names a realm (decision 7), so each agent here holds one, and its budget is the
//! bytes the realm takes plus the room the test gives. The host's own charges, bytes with
//! `charge_host_bytes` and `resize_host_bytes` and steps with `charge_steps` (section 14, "The
//! host's own work and bytes"; HOST-175 to HOST-177), go to the same budgets. Run also with the
//! feature `collect-every-safe-point`.
#![cfg(test)]

use crate::support;

use std::cell::{Cell, RefCell};

use support::host::{
    TestHost, agent_with, global, is_error_of, limits, object, realm, stack_with_room,
};
use support::script::{Outcome, define_function, eval, eval_text, parse, run};
use support::stack::{TestPath, test_path};
use swell::{
    Abrupt, Agent, AgentState, Brand, Completion, Cx, GlobalObjects, HookCx, HostBytes, HostCall,
    HostData, HostError, HostObjectSpec, Limits, Outcome as HostOutcome, Parsed, Prototype, Realm,
    RealmRef, Script, Source, Termination, Tracer, Value,
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

/// The bytes the String "small" adds to an agent with the test limits right after
/// `create_realm`: its owned bytes, and the growth of the arena of Strings when its slot needs
/// one.
fn small_string_bytes() -> u64 {
    let mut agent = support::host::agent().unwrap();
    let realm = support::host::realm(&mut agent);
    let before = agent.heap_bytes().unwrap();
    let made = agent
        .with_cx(&realm, |cx| cx.new_string("small").map(|_| ()))
        .unwrap();
    assert_eq!(made, Ok(()));
    agent.heap_bytes().unwrap() - before
}

/// The epic's item 9: an allocation that takes the agent over its memory budget ends the run
/// with `Terminated(MemoryBudget)`. A `with_cx` whose `new_string` of at least 2,000 code units
/// passes a budget of 1,024 bytes beyond the realm and the String "small".
///
/// The room is the bytes a twin agent reads for the String "small" after its realm
/// ([`small_string_bytes`]) plus 1,024, and the long String is longer than the room, so it
/// passes the budget whatever the slot of a String costs. Before, the room was 1,024 bytes: on
/// 2026-11-24, at a realm of 232 functions (the base of
/// `story:tests-hold-at-any-size-of-the-realm`), "small" needed a growth of the arena of Strings
/// that the 1,024 bytes did not cover and was refused in both modes, and at that realm padded
/// by 2 to 200 functions it was not. The test prints its readings.
#[test]
fn memory_budget_ends_with_cx_in_memory_budget() {
    let small = small_string_bytes();
    let room = small + 1_024;
    let mut limits = support::host::limits();
    limits.memory_bytes = realm_bytes() + room;
    let budget = limits.memory_bytes;
    let mut agent = support::host::agent_with(limits).unwrap();
    let realm = support::host::realm(&mut agent);
    println!("\"small\": {small} bytes, room {room} bytes");

    let text = "x".repeat(usize::try_from(room + 1).unwrap().max(2_000));
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
                error_kind: None,
            })
            .unwrap()
            .map(|_| ())
        })
        .unwrap();
    assert_eq!(outcome, Err(Abrupt::Terminated(Termination::MemoryBudget)));
    assert_eq!(agent.state(), AgentState::Alive);
}

/// With no room beyond the realm every allocation that charges a byte ends the run, nothing
/// more is charged, and the agent stays usable. (`memory_bytes` 0 refuses the realm itself:
/// HOST-12.)
#[test]
fn memory_budget_of_zero_refuses_every_allocation() {
    let mut limits = support::host::limits();
    limits.memory_bytes = realm_bytes();
    let mut agent = support::host::agent_with(limits).unwrap();
    let realm = support::host::realm(&mut agent);
    let outcome = agent
        .with_cx(&realm, |cx| cx.new_string("x").map(|_| ()))
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
    // The bare realm (an exception of `support::host::create_realm`): with no step the
    // padding's entry cannot run.
    let HostOutcome::Normal(realm) = agent.create_realm(None, None).unwrap() else {
        panic!("create_realm did not answer Normal");
    };
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

/// `full` steps of a step budget, or a fiftieth of them with the collector at every safe
/// point, where every step collects, in the gate and in the lane `slow`
/// (`SWELL_FULL_COUNTS=1`) alike. The scripts given such a budget loop or recurse without end,
/// so every budget ends them in the same way. With a tenth, 10,000 of 100,000,
/// [`catching_the_depth_error_and_recursing_again_ends_in_step_budget`] took 22.5 s in that
/// mode at a load of 100 to 110 on 17 cores (cargo-nextest), and with all 100,000 in the lane
/// 18.5 CPU seconds alone (2,000: 0.38, measured on 2026-11-22); its 2,000 steps still reach
/// the `RangeError` at `frame_depth` 20, which takes a few hundred.
fn steps_in_the_collect_mode(full: u64) -> u64 {
    if cfg!(feature = "collect-every-safe-point") {
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
    limits.steps_per_entry = steps_in_the_collect_mode(100_000);
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
fn on_bounded_stack(test: TestPath, stack: usize, body: fn()) -> Result<(), String> {
    let done = format!("{STACK_CHILD}: {test} on {stack} bytes done");
    if std::env::var_os(STACK_CHILD).is_some() {
        crate::support::host::stack_thread(stack)
            .spawn(body)
            .map_err(|e| e.to_string())?
            .join()
            .map_err(|_| format!("{test}: the body failed"))?;
        println!("{done}");
        return Ok(());
    }
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let out = std::process::Command::new(exe)
        .args(["--exact", test.as_str(), "--nocapture", "--test-threads=1"])
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

/// The `reentry_depth` of the stack test: the full depth, 1,000, in the default mode; with the
/// collector at every safe point, 100, in the gate and in the lane `slow` alike. The stack
/// bound is a property of the default mode's frames (the collector marks from a worklist, so
/// its own stack does not grow with the depth). What the collect mode looks for, a value held
/// without a root, shows at any depth: at 100 every safe point of the deepest levels still
/// collects while 100 re-entries, each with its Rust frame and its activation, are live. A
/// collection traces every activation, so the cost grows with the square of the depth: at
/// 1,000 the test took 30.4 s in that mode (21.2 CPU s) against 0.10 s in the default one
/// (measured), and 12.2 CPU seconds alone in the lane on 2026-11-22.
fn stack_reentry() -> u32 {
    if cfg!(feature = "collect-every-safe-point") {
        100
    } else {
        1_000
    }
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
/// `min_stack_bytes()` is enough, with an eighth of it to spare. In the collect mode the
/// depth is [`stack_reentry`], and the thread has seven eighths of `min_stack_bytes()` of that
/// depth.
#[test]
fn min_stack_bytes_is_enough() {
    let stack = stack_with_room(stack_limits());
    on_bounded_stack(
        test_path!("min_stack_bytes_is_enough"),
        stack,
        reach_every_bound,
    )
    .unwrap();
}

/// The room beyond the realm that the tests of the host's own bytes give an agent.
const HOST_ROOM: u64 = 64 * 1024;

/// HOST-175's room beyond the realm: [`HOST_ROOM`], or an eighth of it in the gate with the
/// collector at every safe point ([`in_the_collect_gate`]), where every step of the loop
/// collects. With 64 KiB the loop calls `make` about 990 times at about 8 steps a call; an
/// eighth still leaves room for about 90 calls after the host function and the script.
fn host_175_room() -> u64 {
    if in_the_collect_gate() {
        HOST_ROOM / 8
    } else {
        HOST_ROOM
    }
}

/// The two readings of the heap, `heap_bytes()` and `live_bytes()`.
fn readings(agent: &Agent<TestHost>) -> (u64, u64) {
    (agent.heap_bytes().unwrap(), agent.live_bytes().unwrap())
}

/// An agent with the test limits and `memory_bytes` `budget`, its realm, and after a collection
/// the room the budget leaves.
fn agent_with_budget(budget: u64) -> (Agent<TestHost>, Realm, u64) {
    let mut limits = limits();
    limits.memory_bytes = budget;
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    agent.collect().unwrap();
    let room = budget - agent.heap_bytes().unwrap();
    (agent, realm, room)
}

thread_local! {
    /// The 64-byte buffers the host function `make` keeps, each beside its `HostBytes` in
    /// `TestHost::charges`.
    static BUFFERS: RefCell<Vec<[u8; 64]>> = const { RefCell::new(Vec::new()) };
}

/// HOST-175's host function: it charges 64 bytes and keeps the answer with a 64-byte buffer.
fn make(cx: &mut Cx<'_, TestHost>, _call: HostCall<'_>) -> Completion<Value> {
    let charge = cx.charge_host_bytes(64)?;
    BUFFERS.with(|buffers| buffers.borrow_mut().push([0; 64]));
    cx.host_mut().charges.push(charge);
    Ok(Value::Undefined)
}

/// HOST-175: a host function that charges 64 bytes with `cx.charge_host_bytes` and keeps the
/// answer with a 64-byte buffer in the host on each call, run by the script
/// `while (true) make();` with `memory_bytes` M, the bytes after the realm and 64 KiB, ends in
/// `Terminated(MemoryBudget)`, and `heap_bytes()` then reads at most M; after the host drops the
/// buffers with their `HostBytes`, the script `1` answers `Normal(1)`.
///
/// M is the realm's bytes, the bytes a twin agent reads for defining `make` after its realm
/// ([`make_bytes`]) and the room ([`host_175_room`]), and the answer of `1` is read with no
/// allocation (`support::host::number_answer`). Before, M had no share for `make`: on
/// 2026-11-24 with the collector at every safe point, at a realm of 232 functions (the base of
/// `story:tests-hold-at-any-size-of-the-realm`), defining `make` needed a growth of an arena that
/// the room did not cover and was refused, and at that realm padded by 2 to 200 functions it was
/// not. The test prints its readings.
#[test]
fn host_175_host_bytes_without_end_end_in_memory_budget() {
    let defined = make_bytes();
    let budget = realm_bytes() + defined + host_175_room();
    let (mut agent, realm, room) = agent_with_budget(budget);
    println!("make: {defined} bytes, room {room} bytes before it");
    define_function(&mut agent, &realm, "make", 0, make, false);
    assert_eq!(
        eval_text(&mut agent, &realm, "while (true) make();"),
        Outcome::Terminated(Termination::MemoryBudget)
    );
    let kept = agent.host().charges.len();
    assert!(kept > 0, "make kept no charge");
    assert_eq!(BUFFERS.with(|buffers| buffers.borrow().len()), kept);
    assert!(agent.heap_bytes().unwrap() <= budget);

    agent.host_mut().charges.clear();
    BUFFERS.with(|buffers| buffers.borrow_mut().clear());
    assert_eq!(
        support::host::number_answer(&mut agent, &realm, "1;"),
        Ok(1.0)
    );
    assert_eq!(agent.state(), AgentState::Alive);
}

/// The bytes defining the host function `make` adds to an agent with the test limits after its
/// realm and a collection: the function, its name and key, and any arena growth they need.
fn make_bytes() -> u64 {
    let mut agent = support::host::agent().unwrap();
    let realm = realm(&mut agent);
    agent.collect().unwrap();
    let before = agent.heap_bytes().unwrap();
    define_function(&mut agent, &realm, "make", 0, make, false);
    agent.heap_bytes().unwrap() - before
}

/// Host data that holds a charge of the host and nothing else.
struct HeldCharge {
    _charge: HostBytes,
}

impl HostData for HeldCharge {
    fn trace(&self, _tracer: &mut Tracer<'_>) {}
}

/// HOST-176: a `HostBytes` gives back exactly what it charged, once. A charge of 100 kept in the
/// host data of a host object with no root: after `collect()` frees the object, `live_bytes()`
/// reads what it read before the charge and `heap_bytes()` falls by the 100 bytes and the size
/// of the host data, and a second `collect()` changes neither. `Agent::credit_host_bytes` of a
/// second charge answers its 100 bytes, both readings fall by them at the call, and a later
/// `collect()` changes neither. A `HostBytes` of another agent answers
/// `Err(HostError::ForeignHandle)` and changes nothing here; the other agent's `live_bytes()`
/// falls by its bytes at its next `collect()`.
#[test]
fn host_176_a_host_charge_gives_back_what_it_charged() {
    let mut agent = support::host::agent().unwrap();
    let realm = realm(&mut agent);
    agent.collect().unwrap();
    let (_, live_before) = readings(&agent);
    agent
        .with_cx(&realm, |cx| {
            let charge = cx.charge_host_bytes(100).unwrap();
            cx.new_host_object(HostObjectSpec {
                prototype: Prototype::Null,
                brand: Brand(1),
                data: Some(Box::new(HeldCharge { _charge: charge })),
                error_kind: None,
            })
            .unwrap()
            .unwrap();
        })
        .unwrap();
    let (heap_made, live_made) = readings(&agent);
    assert!(live_made > live_before + 100, "{live_made} {live_before}");
    let data = u64::try_from(size_of::<HeldCharge>()).unwrap();
    let freed = (heap_made - 100 - data, live_before);
    agent.collect().unwrap();
    assert_eq!(readings(&agent), freed);
    agent.collect().unwrap();
    assert_eq!(readings(&agent), freed);

    let second = agent
        .with_cx(&realm, |cx| cx.charge_host_bytes(100))
        .unwrap()
        .unwrap();
    let (heap, live) = readings(&agent);
    assert_eq!(agent.credit_host_bytes(second), Ok(100));
    assert_eq!(readings(&agent), (heap - 100, live - 100));
    agent.collect().unwrap();
    assert_eq!(readings(&agent), (heap - 100, live - 100));

    let mut other = support::host::agent().unwrap();
    let other_realm = support::host::realm(&mut other);
    let foreign = other
        .with_cx(&other_realm, |cx| cx.charge_host_bytes(100))
        .unwrap()
        .unwrap();
    other.collect().unwrap();
    let (_, other_live) = readings(&other);
    let here = readings(&agent);
    assert_eq!(
        agent.credit_host_bytes(foreign),
        Err(HostError::ForeignHandle)
    );
    assert_eq!(readings(&agent), here);
    assert_eq!(readings(&other).1, other_live);
    other.collect().unwrap();
    assert_eq!(readings(&other).1, other_live - 100);
}

/// A dropped charge gives its room to the next charge in the same `with_cx`, with no collection
/// between: `charge_host_bytes` credits the dropped charges before it charges.
#[test]
fn a_dropped_charge_gives_its_room_to_the_next_charge() {
    let (mut agent, realm, room) = agent_with_budget(realm_bytes() + HOST_ROOM);
    let answers = agent
        .with_cx(&realm, |cx| {
            let first = cx.charge_host_bytes(room).map(drop);
            let second = cx.charge_host_bytes(room).map(drop);
            (first, second)
        })
        .unwrap();
    assert_eq!(answers, (Ok(()), Ok(())));
}

/// The room the test of a charge dropped inside a host function leaves, and what its host
/// function charges: the room less 4,096 bytes for the call's own.
const RECHARGE_ROOM: u64 = 1 << 20;
const RECHARGED: u64 = RECHARGE_ROOM - 4_096;

/// A host function that takes the host's charge out, drops it and charges as much again.
fn recharge(cx: &mut Cx<'_, TestHost>, _call: HostCall<'_>) -> Completion<Value> {
    let held = cx.host_mut().charges.pop();
    assert!(held.is_some(), "the host holds no charge");
    drop(held);
    let charge = cx.charge_host_bytes(RECHARGED)?;
    cx.host_mut().charges.push(charge);
    Ok(Value::Undefined)
}

/// An agent with `memory_bytes` `budget`, its realm, the host function `f` ([`recharge`]) and
/// the script `f()` parsed.
fn recharge_agent(budget: u64) -> (Agent<TestHost>, Realm, Script) {
    let mut limits = limits();
    limits.memory_bytes = budget;
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    define_function(&mut agent, &realm, "f", 0, recharge, false);
    let script = parse(&mut agent, &realm, "f();");
    agent.collect().unwrap();
    (agent, realm, script)
}

/// A charge the host drops inside a host function gives its room back to the host function's
/// next charge: with 1 MiB of room and a charge of 1 MiB less 4,096 bytes held, the host
/// function `f` drops it and charges as much again, and the script `f()` answers `Normal`.
#[test]
fn a_charge_dropped_inside_a_host_function_gives_its_room_back() {
    let (probe, _, _) = recharge_agent(limits().memory_bytes);
    let made = probe.heap_bytes().unwrap();
    drop(probe);
    let (mut agent, realm, script) = recharge_agent(made + RECHARGE_ROOM);
    assert_eq!(agent.heap_bytes(), Ok(made));
    let charge = agent
        .with_cx(&realm, |cx| cx.charge_host_bytes(RECHARGED))
        .unwrap()
        .unwrap();
    agent.host_mut().charges.push(charge);
    let outcome = agent.evaluate_script(&script).unwrap();
    assert!(
        matches!(outcome, HostOutcome::Normal(_)),
        "f() answered {outcome:?}"
    );
    assert_eq!(agent.host().charges.len(), 1);
}

/// A `HostBytes` dropped after its agent touches only its own queue: no panic.
#[test]
fn a_charge_dropped_after_its_agent_is_harmless() {
    let mut agent = support::host::agent().unwrap();
    let realm = realm(&mut agent);
    let charge = agent
        .with_cx(&realm, |cx| cx.charge_host_bytes(100))
        .unwrap()
        .unwrap();
    agent.host_mut().charges.push(charge);
    let taken = agent.host_mut().charges.pop().unwrap();
    drop(agent);
    drop(taken);
}

/// `resize_host_bytes` changes a charge in place (decision D3): with room R, a charge of
/// R / 2 + 1 resized to R / 2 and back answers `Ok(Ok(()))` each, `live_bytes()` following the
/// size, and the agent goes on; a resize to R + 1 ends in `Terminated(MemoryBudget)` with the
/// handle's charge and `live_bytes()` unchanged. In a fresh agent a second charge of R / 2 + 1
/// beside the first, a replacement, ends in `Terminated(MemoryBudget)`. A handle of another agent
/// answers `Err(HostError::ForeignHandle)` and changes nothing.
#[test]
fn a_resized_charge_fits_where_a_replacement_does_not() {
    let budget = realm_bytes() + HOST_ROOM;
    let (mut agent, realm, room) = agent_with_budget(budget);
    let half = room / 2 + 1;
    let mut charge = agent
        .with_cx(&realm, |cx| cx.charge_host_bytes(half))
        .unwrap()
        .unwrap();
    let (_, live) = readings(&agent);
    let shrunk = agent
        .with_cx(&realm, |cx| cx.resize_host_bytes(&mut charge, half - 1))
        .unwrap();
    assert_eq!(shrunk, Ok(Ok(())));
    assert_eq!(readings(&agent).1, live - 1);
    let grown = agent
        .with_cx(&realm, |cx| cx.resize_host_bytes(&mut charge, half))
        .unwrap();
    assert_eq!(grown, Ok(Ok(())));
    assert_eq!(readings(&agent).1, live);
    assert_eq!(
        eval_text(&mut agent, &realm, "1;"),
        Outcome::Normal("1".to_string())
    );

    // The readings are compared across a `with_cx`, a safe point that may collect: a collection
    // first frees what the script left.
    agent.collect().unwrap();
    let (_, live) = readings(&agent);
    let past = agent
        .with_cx(&realm, |cx| {
            let past = cx.resize_host_bytes(&mut charge, room + 1);
            let after = cx.new_string("a").map(|_| ());
            (past, after)
        })
        .unwrap();
    assert_eq!(
        past,
        (
            Ok(Err(Abrupt::Terminated(Termination::MemoryBudget))),
            Err(Abrupt::Terminated(Termination::MemoryBudget))
        )
    );
    assert_eq!(readings(&agent).1, live);

    let (mut fresh, fresh_realm, fresh_room) = agent_with_budget(budget);
    assert_eq!(fresh_room, room);
    let (first, replacement) = fresh
        .with_cx(&fresh_realm, |cx| {
            let first = cx.charge_host_bytes(half).unwrap();
            (first, cx.charge_host_bytes(half).map(drop))
        })
        .unwrap();
    assert_eq!(
        replacement,
        Err(Abrupt::Terminated(Termination::MemoryBudget))
    );

    let mut foreign = first;
    agent.collect().unwrap();
    fresh.collect().unwrap();
    let (here, there) = (readings(&agent), readings(&fresh));
    let answer = agent
        .with_cx(&realm, |cx| cx.resize_host_bytes(&mut foreign, 1))
        .unwrap();
    assert_eq!(answer, Err(HostError::ForeignHandle));
    assert_eq!((readings(&agent), readings(&fresh)), (here, there));
    assert_eq!(fresh.credit_host_bytes(foreign), Ok(half));
    assert_eq!(agent.credit_host_bytes(charge), Ok(half));
}

/// The steps of a call of [`walk`].
const WALK_STEPS: u64 = 1_000;

/// The step budget of HOST-177.
const HOST_177_STEPS: u64 = 100_000;

/// HOST-177's host function: it charges 1,000 steps of its own work.
fn walk(cx: &mut Cx<'_, TestHost>, _call: HostCall<'_>) -> Completion<Value> {
    cx.charge_steps(WALK_STEPS)?;
    Ok(Value::Undefined)
}

/// A host function that counts its calls and charges a step more than the budget.
fn walk_past(cx: &mut Cx<'_, TestHost>, _call: HostCall<'_>) -> Completion<Value> {
    cx.host_mut().count += 1;
    cx.charge_steps(HOST_177_STEPS + 1)?;
    Ok(Value::Undefined)
}

/// HOST-177: a host function that calls `cx.charge_steps(1_000)` on each call, run by the
/// script `while (true) walk();` with `steps_per_entry` N, ends in `Terminated(StepBudget)` with
/// `last_entry_steps()` at most N; one that calls `cx.charge_steps(N + 1)` ends the entry at its
/// first call with `last_entry_steps()` below N; an interrupt set before the charge ends it in
/// `Terminated(Interrupted)`.
#[test]
fn host_177_host_steps_end_in_step_budget() {
    let mut agent = agent_with(with_steps(HOST_177_STEPS)).unwrap();
    let realm = realm(&mut agent);
    define_function(&mut agent, &realm, "walk", 0, walk, false);
    define_function(&mut agent, &realm, "walkPast", 0, walk_past, false);
    assert_eq!(
        eval_text(&mut agent, &realm, "while (true) walk();"),
        Outcome::Terminated(Termination::StepBudget)
    );
    let spent = agent.last_entry_steps().unwrap();
    assert!(spent <= HOST_177_STEPS, "{spent}");
    assert!(spent > HOST_177_STEPS - WALK_STEPS - 100, "{spent}");

    assert_eq!(
        eval_text(&mut agent, &realm, "while (true) walkPast();"),
        Outcome::Terminated(Termination::StepBudget)
    );
    assert_eq!(agent.host().count, 1);
    let spent = agent.last_entry_steps().unwrap();
    assert!(spent < HOST_177_STEPS, "{spent}");

    let handle = agent.interrupt_handle();
    let answers = agent
        .with_cx(&realm, |cx| {
            handle.interrupt();
            (cx.charge_steps(1), cx.charge_steps(0))
        })
        .unwrap();
    let interrupted = Err(Abrupt::Terminated(Termination::Interrupted));
    assert_eq!(answers, (interrupted, interrupted));
    assert!(!handle.is_set());
    assert_eq!(
        eval_text(&mut agent, &realm, "1;"),
        Outcome::Normal("1".to_string())
    );
}

/// A realm whose setup closure makes a host object the global object, after it charged
/// `charge` bytes with the `HookCx`, when given, and kept the `HostBytes` in
/// `TestHost::charges`.
fn realm_charging(agent: &mut Agent<TestHost>, charge: Option<u64>) -> HostOutcome<Realm> {
    let mut setup = |cx: &mut HookCx<'_, TestHost>, realm: RealmRef| -> Completion<GlobalObjects> {
        if let Some(bytes) = charge {
            let held = cx.charge_host_bytes(bytes)?;
            cx.host_mut().charges.push(held);
        }
        let global = cx
            .new_host_object(HostObjectSpec {
                prototype: Prototype::ObjectPrototypeOf(realm),
                brand: Brand(1),
                data: None,
                error_kind: None,
            })
            .unwrap()?;
        Ok(GlobalObjects { global, this: None })
    };
    support::host::create_realm(agent, Some(&mut setup), None).unwrap()
}

/// A charge made with the `HookCx` of a setup closure counts as one made with `Cx`: an agent
/// whose setup charged 1,000 bytes reads 1,000 more in both readings than one whose setup did
/// not; a charge past `memory_bytes` ends `create_realm` in `Terminated(MemoryBudget)`.
#[test]
fn host_charges_from_a_hook_count_as_from_cx() {
    let mut charged = support::host::agent().unwrap();
    let mut plain = support::host::agent().unwrap();
    assert!(matches!(
        realm_charging(&mut charged, Some(1_000)),
        HostOutcome::Normal(_)
    ));
    assert!(matches!(
        realm_charging(&mut plain, None),
        HostOutcome::Normal(_)
    ));
    assert_eq!(charged.host().charges.len(), 1);
    let (heap, live) = readings(&plain);
    assert_eq!(readings(&charged), (heap + 1_000, live + 1_000));

    let mut refused = support::host::agent().unwrap();
    assert!(matches!(
        realm_charging(&mut refused, Some(limits().memory_bytes + 1)),
        HostOutcome::Terminated(Termination::MemoryBudget)
    ));
    assert!(refused.host().charges.is_empty());
}

/// A charge refused inside `with_cx` is sticky: with 10 bytes of room a charge of 11 answers
/// `Terminated(MemoryBudget)`, and so does the next call; the next `with_cx` runs afresh.
#[test]
fn host_charges_inside_with_cx_are_sticky() {
    let (mut agent, realm, room) = agent_with_budget(realm_bytes() + 10);
    assert_eq!(room, 10);
    let answers = agent
        .with_cx(&realm, |cx| {
            let charge = cx.charge_host_bytes(11).map(drop);
            let string = cx.new_string("a").map(|_| ());
            (charge, string)
        })
        .unwrap();
    let refused = Err(Abrupt::Terminated(Termination::MemoryBudget));
    assert_eq!(answers, (refused, refused));
    let next = agent
        .with_cx(&realm, |cx| cx.charge_host_bytes(10).map(drop))
        .unwrap();
    assert_eq!(next, Ok(()));
}

/// HOST-215's room: the bytes the host leaves free with its charge.
const HOST_215_ROOM: u64 = 8_388_608;

/// HOST-215: with `memory_bytes` M = 64 MiB, `string_length` 2^24, `reentry_depth` 256 and
/// 10,000,000 steps, a global `s` the host made of 1,048,576 code units `x`, and a host charge
/// of all of the room but 8 MiB held until the entry ends, `join` nested through ToString holds
/// a builder of 2 MiB per level, charged while the engine calls into script, and the script
/// answers `Terminated(MemoryBudget)`; `heap_bytes()` reads at most M before the entry, after
/// it and after a collection; once the host drops its charge and collects, `1` answers 1.
#[test]
fn host_215_a_buffer_held_across_a_call_into_script_is_charged() {
    let mut limits = limits();
    limits.string_length = 1 << 24;
    limits.steps_per_entry = 10_000_000;
    let budget = limits.memory_bytes;
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    let global = global(&mut agent, &realm);
    agent
        .with_cx(&realm, |cx| {
            let s = cx.new_string_utf16(&vec![0x78; 1 << 20]).unwrap();
            let Value::Object(global) = cx.get(&global).unwrap() else {
                panic!("the global object is no object");
            };
            let key = swell::PropertyKey::String(cx.new_string("s").unwrap());
            assert_eq!(
                cx.set(global, key, Value::String(s), true).unwrap(),
                Ok(true)
            );
        })
        .unwrap();
    let script = parse(
        &mut agent,
        &realm,
        "var o = { toString: function () { return [s, o].join(\"\"); } }; [s, o].join(\"\")",
    );
    agent.collect().unwrap();
    let room = budget - agent.heap_bytes().unwrap() - HOST_215_ROOM;
    let charge = agent
        .with_cx(&realm, |cx| cx.charge_host_bytes(room))
        .unwrap()
        .unwrap();
    assert!(agent.heap_bytes().unwrap() <= budget);
    let outcome = agent.evaluate_script(&script).unwrap();
    assert!(
        matches!(outcome, HostOutcome::Terminated(Termination::MemoryBudget)),
        "{outcome:?}"
    );
    assert!(agent.heap_bytes().unwrap() <= budget);
    agent.collect().unwrap();
    assert!(agent.heap_bytes().unwrap() <= budget);
    drop(charge);
    agent.collect().unwrap();
    assert_eq!(
        eval_text(&mut agent, &realm, "1"),
        Outcome::Normal("1".to_string())
    );
}
