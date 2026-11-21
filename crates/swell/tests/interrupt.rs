//! The interrupt handle (`docs/design/host-interface.md`, decision 12). The checks that read the
//! flag during a run are `story:object-model-and-realms` and
//! `story:interpreter-and-script-entry-points`.

mod support;

use std::thread;
use std::time::Duration;

use support::script::{Outcome as Shown, define_function, describe, eval_text, parse};
use swell::{
    AgentState, Brand, Completion, Cx, HostCall, HostData, HostError, HostObjectSpec,
    InterruptHandle, ObjectRef, Outcome, Prototype, Termination, Tracer, Value,
};

fn send_and_sync<T: Send + Sync + 'static>(_: &T) {}

/// HOST-94: `InterruptHandle` is `Send` and `Sync`: a clone moves into a spawned thread, and
/// what it sets there the agent's own handle reads.
#[test]
fn host_94_interrupt_handle_moves_into_a_thread() {
    let agent = support::host::agent().unwrap();
    let handle: InterruptHandle = agent.interrupt_handle();
    send_and_sync(&handle);
    assert!(!handle.is_set());

    let clone = handle.clone();
    thread::spawn(move || clone.interrupt()).join().unwrap();
    assert!(handle.is_set());
    assert!(
        agent.interrupt_handle().is_set(),
        "every handle of one agent shares the flag"
    );

    let clone = handle.clone();
    thread::spawn(move || clone.clear()).join().unwrap();
    assert!(!handle.is_set());

    // A handle of another agent is another flag.
    let other = support::host::agent().unwrap();
    handle.interrupt();
    assert!(!other.interrupt_handle().is_set());
}

struct Stale(ObjectRef);

impl HostData for Stale {
    fn trace(&self, tracer: &mut Tracer<'_>) {
        tracer.object(self.0);
    }
}

/// HOST-95: `interrupt()` on the handle of a faulted agent returns, and the agent's calls still
/// answer `Err(HostError::Faulted)`.
#[test]
fn host_95_interrupt_on_a_faulted_agent_returns() {
    let mut agent = support::host::agent().unwrap();
    let realm = support::host::realm(&mut agent);
    let handle = agent.interrupt_handle();

    // Fault the agent: a host object whose trace lists a freed object.
    let freed = agent
        .with_cx(&realm, |cx| {
            cx.new_host_object(HostObjectSpec {
                prototype: Prototype::Null,
                brand: Brand(1),
                data: None,
            })
            .unwrap()
            .unwrap()
        })
        .unwrap();
    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
    let _root = agent
        .with_cx(&realm, |cx| {
            let holder = cx
                .new_host_object(HostObjectSpec {
                    prototype: Prototype::Null,
                    brand: Brand(2),
                    data: Some(Box::new(Stale(freed))),
                })
                .unwrap()
                .unwrap();
            cx.root(Value::Object(holder)).unwrap()
        })
        .unwrap();
    assert!(matches!(
        agent.collect(),
        Ok(Outcome::Terminated(Termination::EngineFault))
    ));
    assert_eq!(agent.state(), AgentState::Faulted);

    handle.interrupt();
    let from_thread = handle.clone();
    thread::spawn(move || from_thread.interrupt())
        .join()
        .unwrap();
    assert!(handle.is_set());

    assert_eq!(agent.state(), AgentState::Faulted);
    assert!(matches!(
        agent.with_cx(&realm, |_| ()),
        Err(HostError::Faulted)
    ));
    assert!(matches!(agent.collect(), Err(HostError::Faulted)));
    assert_eq!(agent.heap_bytes(), Err(HostError::Faulted));
    handle.clear();
    assert!(matches!(
        agent.with_cx(&realm, |_| ()),
        Err(HostError::Faulted)
    ));
}

/// HOST-90: `interrupt()` called from another thread while the script `while (true) {}` runs
/// with a step budget of 2^62 makes `evaluate_script` answer `Terminated(Interrupted)`.
#[test]
fn host_90_interrupt_from_another_thread_ends_the_loop() {
    let mut limits = support::host::limits();
    limits.steps_per_entry = 1 << 62;
    let mut agent = support::host::agent_with(limits).unwrap();
    let realm = support::host::realm(&mut agent);
    let script = parse(&mut agent, &realm, "while (true) {}");
    let handle = agent.interrupt_handle();
    let interrupter = thread::spawn(move || {
        thread::sleep(Duration::from_millis(50));
        handle.interrupt();
    });
    let outcome = agent.evaluate_script(&script).unwrap();
    interrupter.join().unwrap();
    assert!(matches!(
        outcome,
        Outcome::Terminated(Termination::Interrupted)
    ));
    assert!(agent.last_entry_steps().unwrap() > 0);
    assert_eq!(agent.state(), AgentState::Alive);
}

/// HOST-91: after an entry answered `Terminated(Interrupted)`, `is_set()` reads false and the
/// next script `1` answers `Normal(1)`.
#[test]
fn host_91_interrupted_entry_clears_the_flag() {
    let mut agent = support::host::agent().unwrap();
    let realm = support::host::realm(&mut agent);
    let handle = agent.interrupt_handle();
    let script = parse(&mut agent, &realm, "while (true) {}");
    handle.interrupt();
    assert!(matches!(
        agent.evaluate_script(&script),
        Ok(Outcome::Terminated(Termination::Interrupted))
    ));
    assert!(!handle.is_set());
    assert_eq!(
        eval_text(&mut agent, &realm, "1;"),
        Shown::Normal("1".to_string())
    );
}

/// HOST-92: `interrupt()` called while no entry runs makes the next entry answer
/// `Terminated(Interrupted)` with `last_entry_steps()` 0.
#[test]
fn host_92_interrupt_before_an_entry_ends_it_at_zero_steps() {
    let mut agent = support::host::agent().unwrap();
    let realm = support::host::realm(&mut agent);
    let script = parse(&mut agent, &realm, "var ran = 1;");
    agent.interrupt_handle().interrupt();
    assert!(matches!(
        agent.evaluate_script(&script),
        Ok(Outcome::Terminated(Termination::Interrupted))
    ));
    assert_eq!(agent.last_entry_steps(), Ok(0));
    assert_eq!(
        eval_text(&mut agent, &realm, "typeof ran;"),
        Shown::Normal("\"undefined\"".to_string())
    );
}

/// HOST-93: `clear()` after `interrupt()` and before the next entry lets that entry run to
/// `Normal`.
#[test]
fn host_93_clear_before_the_entry_lets_it_run() {
    let mut agent = support::host::agent().unwrap();
    let realm = support::host::realm(&mut agent);
    let script = parse(&mut agent, &realm, "var i = 0; while (i < 100) i++; i;");
    let handle = agent.interrupt_handle();
    handle.interrupt();
    handle.clear();
    let outcome = agent.evaluate_script(&script).unwrap();
    assert_eq!(
        describe(&mut agent, &realm, outcome),
        Shown::Normal("100".to_string())
    );
}

thread_local! {
    /// The handle `interrupt_now` sets.
    static HANDLE: std::cell::Cell<Option<InterruptHandle>> = const { std::cell::Cell::new(None) };
}

/// A host function that sets its agent's interrupt flag and returns `1`, counting its calls.
fn interrupt_now(
    cx: &mut Cx<'_, support::host::TestHost>,
    _call: HostCall<'_>,
) -> Completion<Value> {
    cx.host_mut().count += 1;
    HANDLE.with(|slot| {
        if let Some(handle) = slot.take() {
            handle.interrupt();
        }
    });
    Ok(Value::Number(1.0))
}

/// HOST-96: an interrupt set while a host function runs ends the run with
/// `Terminated(Interrupted)` at the first check after the host function returns to the engine:
/// the statement after the call does not run, and the host function's value is not used.
#[test]
fn host_96_interrupt_set_in_a_host_function_ends_the_run_after_it() {
    let mut agent = support::host::agent().unwrap();
    let realm = support::host::realm(&mut agent);
    define_function(&mut agent, &realm, "interruptNow", 0, interrupt_now, false);
    HANDLE.with(|slot| slot.set(Some(agent.interrupt_handle())));
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var after = 0; var got = interruptNow(); after = 1;"
        ),
        Shown::Terminated(Termination::Interrupted)
    );
    assert_eq!(agent.host().count, 1);
    assert!(!agent.interrupt_handle().is_set());
    assert_eq!(
        eval_text(&mut agent, &realm, "after + \",\" + got;"),
        Shown::Normal("\"0,undefined\"".to_string())
    );
}
