//! Fault injection (`docs/design/host-interface.md`, decision 10, HOST-71): a broken engine
//! invariant found inside `parse_script`, `create_realm` or `collect` makes that call answer
//! `Terminated(EngineFault)`, `state()` then reads `Faulted`, and every other entry point and
//! read answers `Err(HostError::Faulted)` without calling a hook (HOST-48).
//!
//! No input of the interface reaches a broken invariant on purpose, so the test build has a
//! switch: with the feature `fault-injection` of `crates/swell/Cargo.toml` (off by default),
//! `Agent::inject_fault` makes the next call of one of the three find one. The test is listed
//! in every build and runs with the feature:
//! `cargo test -p swell --features fault-injection --test faults`. Run also with the feature
//! `collect-every-safe-point`.
#![cfg(test)]

#[cfg(feature = "fault-injection")]
mod support;

#[cfg(feature = "fault-injection")]
mod injected {
    use std::cell::Cell;
    use std::rc::Rc;

    use super::support::host::{TestHost, agent, realm};
    use super::support::script::parse;
    use swell::{
        Agent, AgentState, FaultPoint, HostError, Outcome, Parsed, Realm, Source, Termination,
    };

    /// Every entry point and read of slice 1 but `state()`, on a faulted agent: each answers
    /// `Err(HostError::Faulted)`, a `with_cx` closure and a setup closure do not run, and
    /// `credit_host_bytes` of `charge`, made before the fault, answers `Err(HostError::Faulted)`
    /// while the handle, dropped in the call, gives its bytes to its own queue without a panic.
    fn answers_faulted_everywhere(
        agent: &mut Agent<TestHost>,
        realm: &Realm,
        charge: swell::HostBytes,
    ) {
        let ran = Rc::new(Cell::new(false));
        assert_eq!(agent.state(), AgentState::Faulted);
        assert!(matches!(
            agent.credit_host_bytes(charge),
            Err(HostError::Faulted)
        ));
        assert!(matches!(agent.collect(), Err(HostError::Faulted)));
        assert!(matches!(agent.heap_bytes(), Err(HostError::Faulted)));
        assert!(matches!(agent.live_bytes(), Err(HostError::Faulted)));
        assert!(matches!(agent.last_entry_steps(), Err(HostError::Faulted)));
        assert!(matches!(
            agent.parse_script(realm, Source::Utf8("1;"), None),
            Err(HostError::Faulted)
        ));
        let flag = Rc::clone(&ran);
        let mut setup = move |_cx: &mut swell::HookCx<'_, TestHost>, _realm: swell::RealmRef| {
            flag.set(true);
            Err(swell::Abrupt::Throw(swell::Value::Undefined))
        };
        assert!(matches!(
            agent.create_realm(Some(&mut setup), None),
            Err(HostError::Faulted)
        ));
        assert!(matches!(
            agent.global_object(realm),
            Err(HostError::Faulted)
        ));
        assert!(matches!(
            agent.realm_host_defined(realm),
            Err(HostError::Faulted)
        ));
        let flag = Rc::clone(&ran);
        assert!(matches!(
            agent.with_cx(realm, move |_cx| flag.set(true)),
            Err(HostError::Faulted)
        ));
        assert!(!ran.get(), "no closure of the host ran");
        assert!(
            agent.host().calls.is_empty(),
            "no hook was called: {:?}",
            agent.host().calls
        );
    }

    /// HOST-71: with the switch at each of `parse_script`, `create_realm` and `collect`, the
    /// call answers `Terminated(EngineFault)`, `state()` reads `Faulted`, and the agent answers
    /// every other call with `Err(HostError::Faulted)` (HOST-48). The switch fires once and at
    /// its call only: the calls before it answer as they would without it.
    pub(super) fn host_71_injected_fault_at_each_call_faults_the_agent() {
        for point in [
            FaultPoint::ParseScript,
            FaultPoint::CreateRealm,
            FaultPoint::Collect,
        ] {
            let mut agent = agent().unwrap();
            let realm = realm(&mut agent);
            let charge = agent
                .with_cx(&realm, |cx| cx.charge_host_bytes(8))
                .unwrap()
                .unwrap();
            let script = parse(&mut agent, &realm, "var kept = 1;");
            agent.inject_fault(point);
            // The calls that are not the switch's are not touched by it.
            if point != FaultPoint::ParseScript {
                let _ = parse(&mut agent, &realm, "2;");
            }
            if point != FaultPoint::CreateRealm {
                let _second = realm_or_panic(&mut agent);
            }
            if point != FaultPoint::Collect {
                assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
            }
            assert!(matches!(
                agent.evaluate_script(&script),
                Ok(Outcome::Normal(_))
            ));
            assert_eq!(agent.state(), AgentState::Alive, "{point:?}");

            let faulted = match point {
                FaultPoint::ParseScript => matches!(
                    agent.parse_script(&realm, Source::Utf8("var x = 1;"), None),
                    Ok(Parsed::Terminated(Termination::EngineFault))
                ),
                FaultPoint::CreateRealm => matches!(
                    agent.create_realm(None, None),
                    Ok(Outcome::Terminated(Termination::EngineFault))
                ),
                FaultPoint::Collect => matches!(
                    agent.collect(),
                    Ok(Outcome::Terminated(Termination::EngineFault))
                ),
            };
            assert!(faulted, "{point:?} did not answer Terminated(EngineFault)");
            answers_faulted_everywhere(&mut agent, &realm, charge);
            assert!(matches!(
                agent.evaluate_script(&script),
                Err(HostError::Faulted)
            ));
            assert!(matches!(
                agent.script_host_defined(&script),
                Err(HostError::Faulted)
            ));
        }
    }

    /// A second realm, which must be made.
    fn realm_or_panic(agent: &mut Agent<TestHost>) -> Realm {
        realm(agent)
    }
}

/// HOST-71, listed in every build. Without the feature `fault-injection` the switch does not
/// exist and the test is ignored; the gate's run with the feature runs it.
#[test]
#[cfg_attr(
    not(feature = "fault-injection"),
    ignore = "needs the feature fault-injection: cargo test -p swell --features fault-injection --test faults"
)]
fn host_71_injected_fault_at_each_call_faults_the_agent() {
    #[cfg(feature = "fault-injection")]
    injected::host_71_injected_fault_at_each_call_faults_the_agent();
}
