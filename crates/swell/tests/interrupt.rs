//! The interrupt handle (`docs/design/host-interface.md`, decision 12). The checks that read the
//! flag during a run are `story:object-model-and-realms` and
//! `story:interpreter-and-script-entry-points`.

mod support;

use std::thread;

use swell::{
    AgentState, Brand, HostData, HostError, HostObjectSpec, InterruptHandle, ObjectRef, Outcome,
    Termination, Tracer, Value,
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
    let handle = agent.interrupt_handle();

    // Fault the agent: a host object whose trace lists a freed object.
    let freed = agent
        .with_cx(|cx| {
            cx.new_host_object(HostObjectSpec {
                prototype: None,
                brand: Brand(1),
                data: None,
            })
            .unwrap()
        })
        .unwrap();
    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
    let _root = agent
        .with_cx(|cx| {
            let holder = cx
                .new_host_object(HostObjectSpec {
                    prototype: None,
                    brand: Brand(2),
                    data: Some(Box::new(Stale(freed))),
                })
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
    assert!(matches!(agent.with_cx(|_| ()), Err(HostError::Faulted)));
    assert!(matches!(agent.collect(), Err(HostError::Faulted)));
    assert_eq!(agent.heap_bytes(), Err(HostError::Faulted));
    handle.clear();
    assert!(matches!(agent.with_cx(|_| ()), Err(HostError::Faulted)));
}
