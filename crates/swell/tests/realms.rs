//! Realms (`docs/design/host-interface.md`, decision 7; ECMA-262 9.3, `#sec-code-realms`):
//! `create_realm` with InitializeHostDefinedRealm (`#sec-initializehostdefinedrealm`), the
//! global object, SetDefaultGlobalBindings (`#sec-setdefaultglobalbindings`) and the value
//! properties of the global object (19.1, `#sec-value-properties-of-the-global-object`). Run
//! also with the feature `collect-every-safe-point`.
#![cfg(test)]

mod support;

use std::rc::Rc;

use support::host::{
    TestHost, agent, agent_with, data, get, get_object, global, key, limits, object, realm, text,
};
use swell::{
    Abrupt, AgentState, Brand, Completion, GlobalObjects, HookCx, HostData, HostError,
    HostObjectSpec, ObjectRef, Outcome, RealmRef, Termination, Tracer, Value,
};

/// HOST-12: with `memory_bytes` 0, `create_realm` answers `Terminated(MemoryBudget)` and the
/// agent stays `Alive`; a realm that does not fit a small budget answers the same.
#[test]
fn host_12_create_realm_with_zero_memory_is_memory_budget() {
    for memory_bytes in [0, 1_000] {
        let mut limits = limits();
        limits.memory_bytes = memory_bytes;
        let mut agent = agent_with(limits).unwrap();
        let outcome = agent.create_realm(None, None).unwrap();
        assert!(
            matches!(outcome, Outcome::Terminated(Termination::MemoryBudget)),
            "{memory_bytes}: {outcome:?}"
        );
        assert_eq!(agent.state(), AgentState::Alive);
        assert!(agent.heap_bytes().unwrap() <= memory_bytes);
        // The agent answers the next call.
        assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
    }
}

/// HOST-52: after the host drops every root of a realm and of objects of that realm, and no
/// other realm reaches them, the second `collect` returns `heap_bytes()` to the value read
/// before `create_realm`.
#[test]
fn host_52_dropped_realm_returns_heap_bytes() {
    let mut agent = agent().unwrap();
    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
    let before = agent.heap_bytes().unwrap();

    let realm = realm(&mut agent);
    let global = global(&mut agent, &realm);
    let error = agent
        .with_cx(&realm, |cx| {
            let realm = cx.realm_ref(&realm).unwrap();
            let error = cx
                .new_error(realm, swell::ErrorKind::RangeError, "out of range")
                .unwrap()
                .unwrap();
            cx.root(Value::Object(error)).unwrap()
        })
        .unwrap();
    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
    assert!(agent.heap_bytes().unwrap() > before);

    drop(error);
    drop(global);
    drop(realm);
    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
    assert_eq!(agent.heap_bytes().unwrap(), before);
}

/// 19.1 (`#sec-value-properties-of-the-global-object`): `Infinity`, `NaN` and `undefined` are
/// properties of the global object with the attributes { [[Writable]]: false, [[Enumerable]]:
/// false, [[Configurable]]: false }.
#[test]
fn global_value_properties() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let global = global(&mut agent, &realm);
    agent
        .with_cx(&realm, |cx| {
            let global = object(cx, &global);
            assert_eq!(get(cx, global, "Infinity"), Value::Number(f64::INFINITY));
            assert!(matches!(get(cx, global, "NaN"), Value::Number(n) if n.is_nan()));
            assert_eq!(get(cx, global, "undefined"), Value::Undefined);
            for (name, value) in [
                ("Infinity", Value::Number(f64::INFINITY)),
                ("NaN", Value::Number(f64::NAN)),
                ("undefined", Value::Undefined),
            ] {
                let k = key(cx, name);
                assert_eq!(cx.has_property(global, k).unwrap(), Ok(true));
                // Not writable.
                assert_eq!(
                    cx.set(global, k, Value::Number(1.0), false).unwrap(),
                    Ok(false)
                );
                assert!(matches!(
                    cx.set(global, k, Value::Number(1.0), true).unwrap(),
                    Err(Abrupt::Throw(_))
                ));
                // Not configurable.
                assert!(matches!(
                    cx.delete_property_or_throw(global, k).unwrap(),
                    Err(Abrupt::Throw(_))
                ));
                // Not enumerable: the same descriptor is accepted, an enumerable one is not.
                assert_eq!(
                    cx.define_property_or_throw(global, k, data(value, false, false, false))
                        .unwrap(),
                    Ok(())
                );
                assert!(matches!(
                    cx.define_property_or_throw(global, k, data(value, false, true, false))
                        .unwrap(),
                    Err(Abrupt::Throw(_))
                ));
            }
        })
        .unwrap();
}

/// What an Error prototype reads as, for the comparison before and after a collection.
fn read_prototype(
    cx: &mut swell::Cx<'_, TestHost>,
    global: ObjectRef,
    name: &str,
) -> (ObjectRef, ObjectRef, String, String, Value) {
    let constructor = get_object(cx, global, name);
    let prototype = get_object(cx, constructor, "prototype");
    let found_name = get(cx, prototype, "name");
    let message = get(cx, prototype, "message");
    let to_string = get(cx, prototype, "toString");
    (
        constructor,
        prototype,
        text(cx, found_name),
        text(cx, message),
        to_string,
    )
}

/// With only a `Realm` root kept, a collection keeps every intrinsic of the realm: the global
/// object and `%Error.prototype%` and `%RangeError.prototype%`, read through the global `Error`
/// and `RangeError`, are the same objects with their properties after it. The global object's
/// prototype, `%Object.prototype%`, has no public read in this interface; the library test
/// `realm::tests::object_prototype_survives_collect_through_the_realm` holds that part.
#[test]
fn intrinsic_reachable_only_through_the_realm_survives_collect() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let global_root = global(&mut agent, &realm);
    let (global, before) = agent
        .with_cx(&realm, |cx| {
            let global = object(cx, &global_root);
            let error = read_prototype(cx, global, "Error");
            let range_error = read_prototype(cx, global, "RangeError");
            (global, (error, range_error))
        })
        .unwrap();
    drop(global_root);
    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));

    let global_root = agent.global_object(&realm).unwrap();
    let after = agent
        .with_cx(&realm, |cx| {
            let found = object(cx, &global_root);
            assert_eq!(found, global, "the same global object");
            (
                read_prototype(cx, found, "Error"),
                read_prototype(cx, found, "RangeError"),
            )
        })
        .unwrap();
    assert_eq!(after, before);
    assert_eq!(before.0.2, "Error");
    assert_eq!(before.1.2, "RangeError");
    assert_eq!(before.0.3, "");
    assert_eq!(
        before.0.4, before.1.4,
        "RangeError.prototype inherits toString"
    );
}

/// Host-defined data of a realm.
struct Settings;

impl HostData for Settings {
    fn trace(&self, _tracer: &mut Tracer<'_>) {}
}

/// `#sec-initializehostdefinedrealm`: the setup closure runs once, after the intrinsics exist;
/// the global it returns is the realm's global object and gets the clause 19 properties; the
/// host-defined value is the one `realm_host_defined` answers.
#[test]
fn the_setup_closure_gives_the_global_object() {
    let mut agent = agent().unwrap();
    let mut calls = 0;
    let mut made: Option<ObjectRef> = None;
    let mut setup =
        |cx: &mut HookCx<'_, TestHost>, _realm: RealmRef| -> Completion<GlobalObjects> {
            calls += 1;
            let global = cx
                .new_host_object(HostObjectSpec {
                    prototype: None,
                    brand: Brand(9),
                    data: None,
                })
                .unwrap()?;
            made = Some(global);
            Ok(GlobalObjects { global, this: None })
        };
    let settings: Rc<dyn HostData> = Rc::new(Settings);
    let Outcome::Normal(realm) = agent
        .create_realm(Some(&mut setup), Some(Rc::clone(&settings)))
        .unwrap()
    else {
        panic!("create_realm did not answer Normal");
    };
    assert_eq!(calls, 1);
    let held = agent.realm_host_defined(&realm).unwrap().unwrap();
    assert!(Rc::ptr_eq(&held, &settings));

    let global_root = global(&mut agent, &realm);
    agent
        .with_cx(&realm, |cx| {
            let global = object(cx, &global_root);
            assert_eq!(cx.brand(global), Ok(Some(Brand(9))));
            assert_eq!(get(cx, global, "undefined"), Value::Undefined);
            assert!(matches!(get(cx, global, "TypeError"), Value::Object(_)));
        })
        .unwrap();
    // The global made inside the closure is not the handle of another realm.
    assert!(made.is_some());
}

/// The handle the host returns (decision 6), second site of HOST-160, which
/// `story:host-interface-hardening` closes: a `GlobalSetup` closure whose `GlobalObjects` hold a
/// stale handle or a handle of another agent ends its `create_realm` call with
/// `Terminated(EngineFault)`, and `state()` then reads `Faulted`.
#[test]
fn a_setup_returning_a_stale_or_foreign_global_faults_the_agent() {
    let mut other = agent().unwrap();
    let other_realm = realm(&mut other);
    let foreign = other
        .with_cx(&other_realm, |cx| {
            cx.new_host_object(HostObjectSpec {
                prototype: None,
                brand: Brand(1),
                data: None,
            })
            .unwrap()
            .unwrap()
        })
        .unwrap();

    for case in 0..3 {
        let mut agent = agent().unwrap();
        let realm = realm(&mut agent);
        let stale = agent
            .with_cx(&realm, |cx| {
                cx.new_host_object(HostObjectSpec {
                    prototype: None,
                    brand: Brand(1),
                    data: None,
                })
                .unwrap()
                .unwrap()
            })
            .unwrap();
        assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
        let mut setup =
            |cx: &mut HookCx<'_, TestHost>, _realm: RealmRef| -> Completion<GlobalObjects> {
                let good = cx
                    .new_host_object(HostObjectSpec {
                        prototype: None,
                        brand: Brand(2),
                        data: None,
                    })
                    .unwrap()?;
                match case {
                    0 => Ok(GlobalObjects {
                        global: stale,
                        this: None,
                    }),
                    1 => Ok(GlobalObjects {
                        global: good,
                        this: Some(foreign),
                    }),
                    _ => Err(Abrupt::Throw(Value::Object(stale))),
                }
            };
        let outcome = agent.create_realm(Some(&mut setup), None).unwrap();
        assert!(
            matches!(outcome, Outcome::Terminated(Termination::EngineFault)),
            "case {case}: {outcome:?}"
        );
        assert_eq!(agent.state(), AgentState::Faulted);
        assert!(matches!(agent.collect(), Err(HostError::Faulted)));
        assert!(matches!(
            agent.create_realm(None, None),
            Err(HostError::Faulted)
        ));
    }
}

/// A setup closure that throws makes `create_realm` answer `Throw` with that value
/// (InitializeHostDefinedRealm "returns either a normal completion containing unused or a throw
/// completion").
#[test]
fn a_setup_that_throws_makes_create_realm_throw() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let mut setup = |_cx: &mut HookCx<'_, TestHost>,
                     _realm: RealmRef|
     -> Completion<GlobalObjects> { Err(Abrupt::Throw(Value::Number(7.0))) };
    let Outcome::Throw(root) = agent.create_realm(Some(&mut setup), None).unwrap() else {
        panic!("create_realm did not throw");
    };
    assert_eq!(
        agent.with_cx(&realm, |cx| cx.get(&root)).unwrap(),
        Ok(Value::Number(7.0))
    );
    assert_eq!(agent.state(), AgentState::Alive);
}

/// A `Realm` of another agent is `ForeignHandle` to `global_object` and `realm_ref`.
#[test]
fn a_realm_of_another_agent_is_foreign() {
    let mut a = agent().unwrap();
    let mut b = agent().unwrap();
    let realm_b = realm(&mut b);
    let realm = realm(&mut a);
    assert!(matches!(
        b.global_object(&realm),
        Err(HostError::ForeignHandle)
    ));
    assert!(matches!(
        b.realm_host_defined(&realm),
        Err(HostError::ForeignHandle)
    ));
    assert_eq!(
        b.with_cx(&realm_b, |cx| cx.realm_ref(&realm).map(|_| ()))
            .unwrap(),
        Err(HostError::ForeignHandle)
    );
}
