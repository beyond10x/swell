//! The budgets of `Limits` that this part of the engine reaches (`docs/design/host-interface.md`,
//! decision 2; `engine.md`, decision 4, "Memory budget"). The step budget and the depth bounds
//! are `story:interpreter-and-script-entry-points`.
//!
//! Every entry names a realm (decision 7), so each agent here holds one, and its budget is the
//! bytes the realm takes plus the room the test gives.
#![cfg(test)]

mod support;

use swell::{Abrupt, AgentState, Brand, HostData, HostObjectSpec, Termination, Tracer};

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
                prototype: None,
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
