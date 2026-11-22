//! `call`, `construct` and host functions (`docs/design/host-interface.md`, decision 14;
//! ECMA-262 10.3, `#sec-built-in-function-objects`; 7.3.13 `#sec-call`, 7.3.14
//! `#sec-construct`), the result types of decision 5 and the handle the host returns of
//! decision 6. Run also with the feature `collect-every-safe-point`.
#![cfg(test)]

mod support;

use support::host::{
    TestHost, agent, agent_with, data, get, get_object, global, is_error_of, key, limits, object,
    realm, string, text, thrown,
};
use support::script::{Outcome as Shown, define_function, eval_text};
use swell::{
    Abrupt, AgentState, Brand, Completion, Cx, HostCall, HostError, HostObjectSpec, Outcome,
    Parsed, PropertyDescriptor, Prototype, Source, Termination, Value,
};

/// A host function that calls itself through `cx.call`, counting its calls in the host; when
/// its call throws, it records the thrown value in `count`'s place by answering the throw.
fn recurse(cx: &mut Cx<'_, TestHost>, call: HostCall<'_>) -> Completion<Value> {
    cx.host_mut().count += 1;
    cx.call(Value::Object(call.callee), Value::Undefined, &[])
        .unwrap()
}

/// HOST-15: with `reentry_depth` R, a host function that calls itself through `cx.call`
/// gets `Abrupt::Throw` of a `RangeError` from the call that would be re-entry R + 1, after R
/// calls through `cx.call` have run. The outermost call, made while no code runs, is the entry
/// and not a re-entry, so the function runs R + 1 times.
#[test]
fn host_15_reentry_depth_bounds_cx_call() {
    for depth in [0_u32, 1, 5, 40] {
        let mut limits = limits();
        limits.reentry_depth = depth;
        let mut agent = agent_with(limits).unwrap();
        let realm = realm(&mut agent);
        let global = global(&mut agent, &realm);
        let (thrown, is_range_error) = agent
            .with_cx(&realm, |cx| {
                let realm = cx.realm_ref(&realm).unwrap();
                let f = cx
                    .new_host_function(realm, "recurse", 0, recurse, false, None)
                    .unwrap()
                    .unwrap();
                let thrown = thrown(cx.call(Value::Object(f), Value::Undefined, &[]).unwrap());
                let global = object(cx, &global);
                let is_range_error = is_error_of(cx, global, thrown, "RangeError");
                (thrown, is_range_error)
            })
            .unwrap();
        assert!(is_range_error, "depth {depth}: {thrown:?}");
        assert_eq!(agent.host().count, depth + 1, "depth {depth}");
        assert_eq!(agent.state(), AgentState::Alive);
    }
}

/// A host function that is not a constructor.
fn plain(_cx: &mut Cx<'_, TestHost>, _call: HostCall<'_>) -> Completion<Value> {
    Ok(Value::Number(1.0))
}

/// HOST-47: `agent.call` of a root that reads a non-callable value answers `Throw` of a
/// `TypeError` (`#sec-call`, "If IsCallable(F) is false, throw a TypeError exception."), and
/// `agent.construct` of a non-constructor answers `Throw` of a `TypeError` (`#sec-construct`
/// requires a constructor; `#sec-createbuiltinfunction` makes a host function without
/// `constructor: true` one with no [[Construct]]).
#[test]
fn host_47_call_and_construct_of_the_wrong_kind_throw_type_error() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let global_root = global(&mut agent, &realm);
    let (number, undefined, ordinary, function) = agent
        .with_cx(&realm, |cx| {
            let realm = cx.realm_ref(&realm).unwrap();
            let f = cx
                .new_host_function(realm, "plain", 0, plain, false, None)
                .unwrap()
                .unwrap();
            let ordinary = cx.to_object(Value::Boolean(true)).unwrap().unwrap();
            (
                cx.root(Value::Number(1.0)).unwrap(),
                cx.root(Value::Undefined).unwrap(),
                cx.root(Value::Object(ordinary)).unwrap(),
                cx.root(Value::Object(f)).unwrap(),
            )
        })
        .unwrap();

    let mut throws = Vec::new();
    for callee in [&number, &ordinary] {
        match agent.call(&realm, callee, &undefined, &[]).unwrap() {
            Outcome::Throw(root) => throws.push(root),
            other => panic!("call answered {other:?}"),
        }
    }
    for callee in [&number, &ordinary, &function] {
        match agent.construct(&realm, callee, &[], None).unwrap() {
            Outcome::Throw(root) => throws.push(root),
            other => panic!("construct answered {other:?}"),
        }
    }
    // The function itself is callable.
    assert!(matches!(
        agent
            .call(&realm, &function, &undefined, &[&number])
            .unwrap(),
        Outcome::Normal(_)
    ));
    let all_type_errors = agent
        .with_cx(&realm, |cx| {
            let global = object(cx, &global_root);
            throws.iter().all(|root| {
                let value = cx.get(root).unwrap();
                is_error_of(cx, global, value, "TypeError")
            })
        })
        .unwrap();
    assert!(all_type_errors);
    assert_eq!(agent.state(), AgentState::Alive);
}

/// HOST-106: a host function made with name `"print"` and length 1 has the own properties
/// `name` `"print"` and `length` 1 with the attributes CreateBuiltinFunction gives them
/// (`#sec-createbuiltinfunction`: SetFunctionLength, `#sec-setfunctionlength`, and
/// SetFunctionName, `#sec-setfunctionname`: { [[Writable]]: false, [[Enumerable]]: false,
/// [[Configurable]]: true }).
///
/// The attributes are read through the interface: a non-writable property refuses `set`; a
/// property made non-configurable then accepts a redefinition of [[Enumerable]] only to its
/// current value and of [[Writable]] to true only when it is writable
/// (`#sec-validateandapplypropertydescriptor`, steps 5.b and 5.d); a configurable one can be
/// deleted.
#[test]
fn host_106_host_function_name_and_length() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    agent
        .with_cx(&realm, |cx| {
            let realm = cx.realm_ref(&realm).unwrap();
            let f = cx
                .new_host_function(realm, "print", 1, plain, false, None)
                .unwrap()
                .unwrap();
            let name = get(cx, f, "name");
            assert_eq!(text(cx, name), "print");
            assert_eq!(get(cx, f, "length"), Value::Number(1.0));

            for (property, value) in [("name", name), ("length", Value::Number(1.0))] {
                // [[Writable]]: false.
                let k = key(cx, property);
                let other = string(cx, "other");
                assert_eq!(cx.set(f, k, other, false).unwrap(), Ok(false));
                assert!(matches!(
                    cx.set(f, k, other, true).unwrap(),
                    Err(Abrupt::Throw(_))
                ));
                assert_eq!(get(cx, f, property), value);

                // [[Configurable]]: true, and [[Enumerable]]: false.
                let probe = cx.new_host_function(realm, property, 1, plain, false, None);
                let probe = probe.unwrap().unwrap();
                let k = key(cx, property);
                let fixed = PropertyDescriptor {
                    configurable: Some(false),
                    ..PropertyDescriptor::default()
                };
                assert_eq!(
                    cx.define_property_or_throw(probe, k, fixed).unwrap(),
                    Ok(())
                );
                let not_enumerable = PropertyDescriptor {
                    enumerable: Some(false),
                    ..PropertyDescriptor::default()
                };
                assert_eq!(
                    cx.define_property_or_throw(probe, k, not_enumerable)
                        .unwrap(),
                    Ok(())
                );
                let enumerable = PropertyDescriptor {
                    enumerable: Some(true),
                    ..PropertyDescriptor::default()
                };
                assert!(matches!(
                    cx.define_property_or_throw(probe, k, enumerable).unwrap(),
                    Err(Abrupt::Throw(_))
                ));
                let writable = PropertyDescriptor {
                    writable: Some(true),
                    ..PropertyDescriptor::default()
                };
                assert!(matches!(
                    cx.define_property_or_throw(probe, k, writable).unwrap(),
                    Err(Abrupt::Throw(_))
                ));

                // Configurable: the delete succeeds, and the property read is then the one
                // `%Function.prototype%` has ("length" 0 and "name" "", 20.2.3).
                let k = key(cx, property);
                assert_eq!(cx.delete_property_or_throw(f, k).unwrap(), Ok(()));
                let inherited = get(cx, f, property);
                if property == "name" {
                    assert_eq!(text(cx, inherited), "");
                } else {
                    assert_eq!(inherited, Value::Number(0.0));
                }
            }
        })
        .unwrap();
}

/// A constructor that records what it receives and answers a new object.
fn record_call(cx: &mut Cx<'_, TestHost>, call: HostCall<'_>) -> Completion<Value> {
    let entry = format!(
        "this {:?} args {:?} new_target {} callee {:?}",
        call.this,
        call.args,
        call.new_target.is_some_and(|target| target == call.callee),
        call.callee,
    );
    cx.host_mut().calls.push(entry);
    if call.new_target.is_some() {
        let object = cx
            .new_host_object(HostObjectSpec {
                prototype: Prototype::Null,
                brand: Brand(3),
                data: None,
                error_kind: None,
            })
            .unwrap()?;
        Ok(Value::Object(object))
    } else {
        Ok(Value::Number(call.args.len() as f64))
    }
}

/// `#sec-built-in-function-objects-call-thisargument-argumentslist` and
/// `#sec-built-in-function-objects-construct-argumentslist-newtarget`: a host function receives
/// `this`, the arguments and `new_target` `None` when called, and `new_target` `Some(F)` with
/// an uninitialized (undefined) `this` when constructed; its `Ok` value is the result.
#[test]
fn a_host_function_receives_this_arguments_and_new_target() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let (called, constructed, f) = agent
        .with_cx(&realm, |cx| {
            let realm = cx.realm_ref(&realm).unwrap();
            let f = cx
                .new_host_function(realm, "F", 2, record_call, true, None)
                .unwrap()
                .unwrap();
            let called = cx
                .call(
                    Value::Object(f),
                    Value::Boolean(false),
                    &[Value::Number(1.0), Value::Null],
                )
                .unwrap();
            let constructed = cx
                .construct(Value::Object(f), &[Value::Number(3.0)], None)
                .unwrap();
            let is_brand_3 = match constructed {
                Ok(Value::Object(object)) => cx.brand(object) == Ok(Some(Brand(3))),
                _ => false,
            };
            (called, is_brand_3, f)
        })
        .unwrap();
    assert_eq!(called, Ok(Value::Number(2.0)));
    assert!(constructed);
    assert_eq!(
        agent.host().calls,
        [
            format!("this Boolean(false) args [Number(1.0), Null] new_target false callee {f:?}"),
            format!("this Undefined args [Number(3.0)] new_target true callee {f:?}"),
        ]
    );
}

thread_local! {
    /// The value `answer` returns: set by a test before the call.
    static ANSWER: std::cell::Cell<Option<Completion<Value>>> = const { std::cell::Cell::new(None) };
}

/// A host function that returns what the test put in `ANSWER`.
fn answer(_cx: &mut Cx<'_, TestHost>, _call: HostCall<'_>) -> Completion<Value> {
    ANSWER
        .with(|answer| answer.take())
        .unwrap_or(Ok(Value::Undefined))
}

/// The handle the host returns (decision 6), first site of HOST-160, which
/// `story:host-interface-hardening` closes: a host function whose `Ok` value or
/// `Abrupt::Throw` value holds a stale handle or a handle of another agent ends the run with
/// `Terminated(EngineFault)`, `state()` then reads `Faulted`, and nothing panics.
#[test]
fn a_host_function_returning_a_stale_or_foreign_handle_faults_the_agent() {
    let mut other = agent().unwrap();
    let other_realm = realm(&mut other);
    let foreign = other
        .with_cx(&other_realm, |cx| cx.new_string("foreign").unwrap())
        .unwrap();

    let returns: [fn(Value) -> Completion<Value>; 2] = [Ok, |v| Err(Abrupt::Throw(v))];
    for (case, make) in returns.into_iter().enumerate() {
        for stale_or_foreign in [true, false] {
            let mut agent = agent().unwrap();
            let realm = realm(&mut agent);
            let stale = agent
                .with_cx(&realm, |cx| {
                    cx.new_host_object(HostObjectSpec {
                        prototype: Prototype::Null,
                        brand: Brand(1),
                        data: None,
                        error_kind: None,
                    })
                    .unwrap()
                    .unwrap()
                })
                .unwrap();
            assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
            let returned = if stale_or_foreign {
                Value::Object(stale)
            } else {
                Value::String(foreign)
            };
            let (outcome, later) = agent
                .with_cx(&realm, |cx| {
                    let realm = cx.realm_ref(&realm).unwrap();
                    let f = cx
                        .new_host_function(realm, "answer", 0, answer, false, None)
                        .unwrap()
                        .unwrap();
                    ANSWER.with(|answer| answer.set(Some(make(returned))));
                    let outcome = cx.call(Value::Object(f), Value::Undefined, &[]);
                    (outcome, cx.new_string("later").map(|_| ()))
                })
                .unwrap();
            assert_eq!(
                outcome,
                Ok(Err(Abrupt::Terminated(Termination::EngineFault))),
                "case {case}, stale {stale_or_foreign}"
            );
            assert_eq!(later, Err(Abrupt::Terminated(Termination::EngineFault)));
            assert_eq!(agent.state(), AgentState::Faulted);
            assert!(matches!(
                agent.with_cx(&realm, |_| ()),
                Err(HostError::Faulted)
            ));
            assert!(matches!(agent.collect(), Err(HostError::Faulted)));
        }
    }
}

/// Decision 5: each method of `Cx` that takes a handle checks every handle among its arguments
/// before anything else, before the sticky termination of decision 10 too, and answers
/// `Err(StaleHandle)` or `Err(ForeignHandle)` with nothing changed and the agent `Alive`;
/// `new_host_object` checks the prototype of its spec the same way (HOST-30, HOST-31).
#[test]
fn cx_methods_check_their_handles_before_the_sticky_termination() {
    let mut other = agent().unwrap();
    let other_realm = realm(&mut other);
    let foreign = other
        .with_cx(&other_realm, |cx| {
            cx.to_object(Value::Boolean(true)).unwrap().unwrap()
        })
        .unwrap();
    let foreign_realm = other
        .with_cx(&other_realm, |cx| cx.realm_ref(&other_realm).unwrap())
        .unwrap();

    let mut limits = limits();
    limits.steps_per_entry = 0;
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    let stale = agent
        .with_cx(&realm, |cx| {
            cx.to_object(Value::Boolean(true)).unwrap().unwrap()
        })
        .unwrap();
    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));

    agent
        .with_cx(&realm, |cx| {
            let realm = cx.realm_ref(&realm).unwrap();
            let live = cx.to_object(Value::Boolean(true)).unwrap().unwrap();
            let f = cx
                .new_host_function(realm, "plain", 0, plain, true, None)
                .unwrap()
                .unwrap();
            let k = key(cx, "k");
            // The step budget of 0 ends the first call, and the termination is sticky.
            assert_eq!(
                cx.call(Value::Object(f), Value::Undefined, &[]),
                Ok(Err(Abrupt::Terminated(Termination::StepBudget)))
            );

            for (bad, error) in [
                (stale, HostError::StaleHandle),
                (foreign, HostError::ForeignHandle),
            ] {
                let v = Value::Object(bad);
                assert_eq!(cx.call(v, Value::Undefined, &[]), Err(error));
                assert_eq!(cx.call(Value::Object(f), v, &[]), Err(error));
                assert_eq!(
                    cx.call(Value::Object(f), Value::Undefined, &[v]),
                    Err(error)
                );
                assert_eq!(cx.construct(v, &[], None), Err(error));
                assert_eq!(cx.construct(Value::Object(f), &[], Some(v)), Err(error));
                assert_eq!(cx.get_property(bad, k), Err(error));
                assert_eq!(cx.set(bad, k, Value::Null, true), Err(error));
                assert_eq!(cx.set(live, k, v, true), Err(error));
                assert_eq!(
                    cx.define_property_or_throw(bad, k, data(Value::Null, true, true, true)),
                    Err(error)
                );
                assert_eq!(
                    cx.define_property_or_throw(live, k, data(v, true, true, true)),
                    Err(error)
                );
                assert_eq!(cx.has_property(bad, k), Err(error));
                assert_eq!(cx.delete_property_or_throw(bad, k), Err(error));
                assert_eq!(cx.to_primitive(v, None), Err(error));
                assert_eq!(cx.to_number(v), Err(error));
                assert_eq!(cx.to_numeric(v), Err(error));
                assert_eq!(cx.to_string(v), Err(error));
                assert_eq!(cx.to_object(v), Err(error));
                assert_eq!(cx.to_property_key(v), Err(error));
                assert_eq!(cx.get_prototype_of(bad), Err(error));
                assert_eq!(cx.set_prototype_of(bad, None), Err(error));
                assert_eq!(cx.set_prototype_of(live, Some(bad)), Err(error));
                assert!(matches!(
                    cx.new_host_object(HostObjectSpec {
                        prototype: Prototype::Object(bad),
                        brand: Brand(1),
                        data: None,
                        error_kind: None,
                    }),
                    Err(e) if e == error
                ));
            }
            assert_eq!(
                cx.new_error(foreign_realm, swell::ErrorKind::TypeError, "x"),
                Err(HostError::ForeignHandle)
            );
            assert_eq!(
                cx.new_host_function(foreign_realm, "f", 0, plain, false, None),
                Err(HostError::ForeignHandle)
            );
            // The run stays terminated for the calls whose handles are good.
            assert_eq!(
                cx.get_property(live, k),
                Ok(Err(Abrupt::Terminated(Termination::StepBudget)))
            );
            assert_eq!(
                cx.set_prototype_of(live, None),
                Ok(Err(Abrupt::Terminated(Termination::StepBudget)))
            );
        })
        .unwrap();
    assert_eq!(agent.state(), AgentState::Alive);
}

/// A host function that calls itself through `cx.call` without end, counting its calls.
fn spin(cx: &mut Cx<'_, TestHost>, call: HostCall<'_>) -> Completion<Value> {
    cx.host_mut().count += 1;
    cx.call(Value::Object(call.callee), Value::Undefined, &[])
        .unwrap()
}

/// The step budget (decision 2, `steps_per_entry`): every call into a function charges one step
/// to the outermost entry, nested calls included; with N steps the call that would spend step
/// N + 1 ends the run in `Terminated(StepBudget)` before it runs, and the next entry has the
/// whole budget again.
#[test]
fn a_call_spends_the_step_budget_of_the_outermost_entry() {
    for steps in [0_u64, 1, 10] {
        let mut limits = limits();
        limits.steps_per_entry = steps;
        let mut agent = agent_with(limits).unwrap();
        let realm = realm(&mut agent);
        let outcome = agent
            .with_cx(&realm, |cx| {
                let realm = cx.realm_ref(&realm).unwrap();
                let f = cx
                    .new_host_function(realm, "spin", 0, spin, false, None)
                    .unwrap()
                    .unwrap();
                cx.call(Value::Object(f), Value::Undefined, &[]).unwrap()
            })
            .unwrap();
        assert_eq!(outcome, Err(Abrupt::Terminated(Termination::StepBudget)));
        assert_eq!(u64::from(agent.host().count), steps, "steps {steps}");
        assert_eq!(agent.state(), AgentState::Alive);
    }
}

/// A host function that sets the interrupt flag of its agent through the handle it was given.
fn interrupt_self(cx: &mut Cx<'_, TestHost>, _call: HostCall<'_>) -> Completion<Value> {
    cx.host_mut().count += 1;
    INTERRUPT.with(|handle| {
        if let Some(handle) = handle.take() {
            handle.interrupt();
        }
    });
    Ok(Value::Undefined)
}

thread_local! {
    static INTERRUPT: std::cell::Cell<Option<swell::InterruptHandle>> = const { std::cell::Cell::new(None) };
}

/// Decision 12: the step function checks the interrupt flag. A flag set while nothing runs ends
/// the next entry before it runs anything; a flag set inside a host function ends the run at
/// the next check, the next call. The flag is cleared when it ends a run.
#[test]
fn an_interrupt_ends_the_run_at_the_next_check() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let handle = agent.interrupt_handle();

    handle.interrupt();
    let outcome = agent
        .with_cx(&realm, |cx| {
            let realm = cx.realm_ref(&realm).unwrap();
            let f = cx
                .new_host_function(realm, "interrupt_self", 0, interrupt_self, false, None)
                .unwrap()
                .unwrap();
            cx.call(Value::Object(f), Value::Undefined, &[]).unwrap()
        })
        .unwrap();
    assert_eq!(outcome, Err(Abrupt::Terminated(Termination::Interrupted)));
    assert!(!handle.is_set());
    assert_eq!(agent.host().count, 0);

    INTERRUPT.with(|slot| slot.set(Some(handle.clone())));
    let outcome = agent
        .with_cx(&realm, |cx| {
            let realm = cx.realm_ref(&realm).unwrap();
            let f = cx
                .new_host_function(realm, "interrupt_self", 0, interrupt_self, false, None)
                .unwrap()
                .unwrap();
            let first = cx.call(Value::Object(f), Value::Undefined, &[]).unwrap();
            let second = cx.call(Value::Object(f), Value::Undefined, &[]).unwrap();
            (first, second)
        })
        .unwrap();
    assert_eq!(outcome.0, Ok(Value::Undefined));
    assert_eq!(outcome.1, Err(Abrupt::Terminated(Termination::Interrupted)));
    assert_eq!(agent.host().count, 1);
    assert!(!handle.is_set());
    assert_eq!(agent.state(), AgentState::Alive);
}

/// `Agent::call` and `Agent::construct` take roots and answer `Outcome` with roots; a stale or
/// foreign root answers `Err` (HOST-30, HOST-31).
#[test]
fn agent_call_reads_and_answers_roots() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let (f, this, one) = agent
        .with_cx(&realm, |cx| {
            let realm = cx.realm_ref(&realm).unwrap();
            let f = cx
                .new_host_function(realm, "F", 2, record_call, true, None)
                .unwrap()
                .unwrap();
            (
                cx.root(Value::Object(f)).unwrap(),
                cx.root(Value::Null).unwrap(),
                cx.root(Value::Number(1.0)).unwrap(),
            )
        })
        .unwrap();
    let Outcome::Normal(result) = agent.call(&realm, &f, &this, &[&one, &one]).unwrap() else {
        panic!("call did not answer Normal");
    };
    assert_eq!(
        agent.with_cx(&realm, |cx| cx.get(&result)).unwrap(),
        Ok(Value::Number(2.0))
    );
    let Outcome::Normal(made) = agent.construct(&realm, &f, &[&one], Some(&f)).unwrap() else {
        panic!("construct did not answer Normal");
    };
    let brand = agent
        .with_cx(&realm, |cx| match cx.get(&made).unwrap() {
            Value::Object(object) => cx.brand(object),
            other => panic!("{other:?}"),
        })
        .unwrap();
    assert_eq!(brand, Ok(Some(Brand(3))));

    let mut other = support::host::agent().unwrap();
    let other_realm = support::host::realm(&mut other);
    let foreign = other
        .with_cx(&other_realm, |cx| cx.root(Value::Null).unwrap())
        .unwrap();
    assert!(matches!(
        agent.call(&realm, &f, &foreign, &[]),
        Err(HostError::ForeignHandle)
    ));
    assert!(matches!(
        agent.construct(&realm, &f, &[&foreign], None),
        Err(HostError::ForeignHandle)
    ));
    assert_eq!(agent.state(), AgentState::Alive);
}

/// Decision 7: `with_cx`, `Agent::call` and `Agent::construct` check the `Realm` they are given
/// first, before every other argument, and a realm of another agent answers
/// `Err(ForeignHandle)` with nothing run, nothing changed and the agent `Alive`.
#[test]
fn entries_check_their_realm_first() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let (f, null) = agent
        .with_cx(&realm, |cx| {
            let realm = cx.realm_ref(&realm).unwrap();
            let f = cx
                .new_host_function(realm, "F", 0, record_call, true, None)
                .unwrap()
                .unwrap();
            (
                cx.root(Value::Object(f)).unwrap(),
                cx.root(Value::Null).unwrap(),
            )
        })
        .unwrap();
    let mut other = support::host::agent().unwrap();
    let other_realm = support::host::realm(&mut other);
    let before = agent.heap_bytes().unwrap();

    let mut ran = false;
    assert!(matches!(
        agent.with_cx(&other_realm, |_| ran = true),
        Err(HostError::ForeignHandle)
    ));
    assert!(!ran);
    assert!(matches!(
        agent.call(&other_realm, &f, &null, &[]),
        Err(HostError::ForeignHandle)
    ));
    // A `null` new target alone answers `WrongKind`; the realm is checked before it.
    assert!(matches!(
        agent.construct(&realm, &f, &[], Some(&null)),
        Err(HostError::WrongKind { .. })
    ));
    assert!(matches!(
        agent.construct(&other_realm, &f, &[], Some(&null)),
        Err(HostError::ForeignHandle)
    ));
    assert!(agent.host().calls.is_empty());
    assert_eq!(agent.heap_bytes().unwrap(), before);
    assert_eq!(agent.state(), AgentState::Alive);
}

/// A host function that records its arguments and whether it got a new target, and answers its
/// `this`.
fn receive(cx: &mut Cx<'_, TestHost>, call: HostCall<'_>) -> Completion<Value> {
    let entry = format!(
        "args {:?} new_target {:?}",
        call.args,
        call.new_target.map(|target| target == call.callee)
    );
    cx.host_mut().calls.push(entry);
    Ok(call.this)
}

/// HOST-103 (`#sec-built-in-function-objects-call-thisargument-argumentslist`): a host function
/// called from script as `o.f(1, 2)` receives `this` = `o`, the arguments `[1, 2]` and
/// `new_target` `None`, and its `Ok(v)` is the call's value.
#[test]
fn host_103_host_function_receives_this_and_arguments() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    define_function(&mut agent, &realm, "receive", 2, receive, false);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var o = { f: receive }; o.f(1, 2) === o;"
        ),
        Shown::Normal("true".to_string())
    );
    assert_eq!(
        agent.host().calls,
        ["args [Number(1.0), Number(2.0)] new_target None"]
    );
}

/// HOST-104 (`#sec-built-in-function-objects-construct-argumentslist-newtarget`): a host
/// function made with `constructor: true` and called as `new F()` receives `new_target`
/// `Some(F)`; one made with `constructor: false` makes `new G()` throw a `TypeError` without
/// being called.
#[test]
fn host_104_new_target_and_non_constructors() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    define_function(&mut agent, &realm, "F", 0, record_call, true);
    define_function(&mut agent, &realm, "G", 0, record_call, false);
    assert_eq!(
        eval_text(&mut agent, &realm, "typeof new F(7);"),
        Shown::Normal("\"object\"".to_string())
    );
    assert_eq!(agent.host().calls.len(), 1);
    assert!(
        agent.host().calls[0].contains("args [Number(7.0)] new_target true"),
        "{:?}",
        agent.host().calls
    );
    assert_eq!(
        eval_text(&mut agent, &realm, "new G();"),
        Shown::Throw("TypeError".to_string())
    );
    assert_eq!(agent.host().calls.len(), 1, "G was called");
}

/// A host function that throws its first argument.
fn throw_it(_cx: &mut Cx<'_, TestHost>, call: HostCall<'_>) -> Completion<Value> {
    Err(Abrupt::Throw(
        call.args.first().copied().unwrap_or(Value::Undefined),
    ))
}

/// HOST-105 (decision 14): a host function that returns `Err(Abrupt::Throw(v))` makes the
/// calling script's `catch (e)` receive `e === v`.
#[test]
fn host_105_host_throw_reaches_the_script_catch() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    define_function(&mut agent, &realm, "throwIt", 1, throw_it, false);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var o = {}; var same; try { throwIt(o); } catch (e) { same = e === o; } \
             var s; try { throwIt(\"s\"); } catch (e) { s = e; } same + s;"
        ),
        Shown::Normal("\"trues\"".to_string())
    );
    assert_eq!(
        eval_text(&mut agent, &realm, "throwIt(3);"),
        Shown::Throw("3".to_string())
    );
}

thread_local! {
    /// The realm `evaluate_inner` parses in.
    static INNER_REALM: std::cell::Cell<Option<swell::RealmRef>> = const { std::cell::Cell::new(None) };
}

/// A host function that parses and evaluates a second script through `cx` and answers its
/// completion value.
fn evaluate_inner(cx: &mut Cx<'_, TestHost>, _call: HostCall<'_>) -> Completion<Value> {
    let realm = INNER_REALM.with(std::cell::Cell::get).unwrap();
    let source = "var inner = 41; function made() { return inner + 1; } inner;";
    let Parsed::Parsed(script) = cx.parse_script(realm, Source::Utf8(source), None).unwrap() else {
        panic!("the inner script did not parse");
    };
    cx.evaluate_script(script).unwrap()
}

/// HOST-107 (`html:clean-up-after-running-script`): a host function that evaluates a second
/// parsed script through `cx.evaluate_script` gets that script's completion value, and the
/// outer script then continues with the bindings the inner one created.
#[test]
fn host_107_cx_evaluate_script_returns_its_completion_value() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let realm_ref = agent
        .with_cx(&realm, |cx| cx.realm_ref(&realm).unwrap())
        .unwrap();
    INNER_REALM.with(|cell| cell.set(Some(realm_ref)));
    define_function(
        &mut agent,
        &realm,
        "evaluateInner",
        0,
        evaluate_inner,
        false,
    );
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var got = evaluateInner(); got + \",\" + inner + \",\" + made();"
        ),
        Shown::Normal("\"41,41,42\"".to_string())
    );
}

/// HOST-180 (`webidl:interface-object`: "If I inherits from some other interface P, then set
/// constructorProto to the interface object of P in realm."): a host function `Element` given
/// the host function `Node` as [[Prototype]] by `cx.set_prototype_of` reads `Node` through
/// `Object.getPrototypeOf` and `cx.get_prototype_of`, and `Node` keeps `%Function.prototype%`.
/// The ordinary [[SetPrototypeOf]] (`#sec-ordinarysetprototypeof`) refuses a cycle (step 7.b.i)
/// and a non-extensible object given another prototype (step 4) with `Ok(Ok(false))`; a stale
/// handle answers `Err(StaleHandle)`. No call runs script code: getters on
/// `%Function.prototype%` that count their calls in a script variable stay at 0.
#[test]
fn host_180_an_interface_object_has_its_parent_as_prototype() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let global_root = global(&mut agent, &realm);
    define_function(&mut agent, &realm, "Node", 0, plain, true);
    define_function(&mut agent, &realm, "Element", 0, plain, true);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var count = 0; var sealed = Object.preventExtensions({}); \
             var names = ['constructor', 'prototype', '__proto__']; \
             for (var i = 0; i < names.length; i++) { \
               Object.defineProperty(Function.prototype, names[i], \
                 { get: function () { count += 1; }, configurable: true }); \
             } count;"
        ),
        Shown::Normal("0".to_string())
    );
    let stale = agent
        .with_cx(&realm, |cx| {
            cx.new_host_object(HostObjectSpec {
                prototype: Prototype::Null,
                brand: Brand(1),
                data: None,
                error_kind: None,
            })
            .unwrap()
            .unwrap()
        })
        .unwrap();
    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));

    let answers = agent
        .with_cx(&realm, |cx| {
            let global = object(cx, &global_root);
            let node = get_object(cx, global, "Node");
            let element = get_object(cx, global, "Element");
            let sealed = get_object(cx, global, "sealed");
            let set = cx.set_prototype_of(element, Some(node));
            let read = cx.get_prototype_of(element) == Ok(Ok(Some(node)));
            let cycle = cx.set_prototype_of(node, Some(element));
            let not_extensible = cx.set_prototype_of(sealed, Some(node));
            let stale = [
                cx.set_prototype_of(stale, Some(node)),
                cx.set_prototype_of(element, Some(stale)),
                cx.get_prototype_of(stale).map(|_| Ok(true)),
            ];
            (set, read, cycle, not_extensible, stale)
        })
        .unwrap();
    assert_eq!(answers.0, Ok(Ok(true)));
    assert!(answers.1, "get_prototype_of(Element) is not Node");
    assert_eq!(answers.2, Ok(Ok(false)));
    assert_eq!(answers.3, Ok(Ok(false)));
    assert!(
        answers
            .4
            .iter()
            .all(|answer| *answer == Err(HostError::StaleHandle)),
        "{:?}",
        answers.4
    );
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "Object.getPrototypeOf(Element) === Node && \
             Object.getPrototypeOf(Node) === Function.prototype && \
             Object.getPrototypeOf(sealed) === Object.prototype;"
        ),
        Shown::Normal("true".to_string())
    );
    assert_eq!(
        eval_text(&mut agent, &realm, "count;"),
        Shown::Normal("0".to_string())
    );
    assert_eq!(agent.state(), AgentState::Alive);
}

/// [[SetPrototypeOf]] through `cx.set_prototype_of` is the object's internal method: the
/// ordinary one (`#sec-ordinarysetprototypeof`) answers true for the prototype the object has
/// (step 2) and false for another on a non-extensible object (step 4);
/// SetImmutablePrototype of `%Object.prototype%`
/// (`#sec-immutable-prototype-exotic-objects-setprototypeof-v`) answers false for any value
/// but its own null. `cx.get_prototype_of` of a host object made with `Prototype::Null` answers
/// `None`. The walk of step 7 spends one step per object of the new prototype's chain, in the
/// running entry, and each `with_cx` call begins its entry afresh.
#[test]
fn set_prototype_of_follows_the_internal_method() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let global_root = global(&mut agent, &realm);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var fixed = Object.preventExtensions({}); var other = {}; Object.isExtensible(fixed);"
        ),
        Shown::Normal("false".to_string())
    );
    let answers = agent
        .with_cx(&realm, |cx| {
            let global = object(cx, &global_root);
            let fixed = get_object(cx, global, "fixed");
            let other = get_object(cx, global, "other");
            let object_constructor = get_object(cx, global, "Object");
            let object_prototype = get_object(cx, object_constructor, "prototype");
            let null_host = cx
                .new_host_object(HostObjectSpec {
                    prototype: Prototype::Null,
                    brand: Brand(2),
                    data: None,
                    error_kind: None,
                })
                .unwrap()
                .unwrap();
            let fixed_answers = [
                cx.set_prototype_of(fixed, Some(other)),
                cx.set_prototype_of(fixed, None),
                cx.set_prototype_of(fixed, Some(object_prototype)),
            ];
            let fixed_kept = cx.get_prototype_of(fixed) == Ok(Ok(Some(object_prototype)));
            let immutable_answers = [
                cx.set_prototype_of(object_prototype, Some(other)),
                cx.set_prototype_of(object_prototype, Some(null_host)),
                cx.set_prototype_of(object_prototype, None),
            ];
            let immutable_kept = cx.get_prototype_of(object_prototype) == Ok(Ok(None));
            let null_host_read = cx.get_prototype_of(null_host) == Ok(Ok(None));
            let null_host_set = cx.set_prototype_of(null_host, Some(other));
            let null_host_moved = cx.get_prototype_of(null_host) == Ok(Ok(Some(other)));
            (
                fixed_answers,
                fixed_kept,
                immutable_answers,
                immutable_kept,
                null_host_read,
                null_host_set,
                null_host_moved,
            )
        })
        .unwrap();
    assert_eq!(answers.0, [Ok(Ok(false)), Ok(Ok(false)), Ok(Ok(true))]);
    assert!(answers.1, "the non-extensible object's prototype changed");
    assert_eq!(answers.2, [Ok(Ok(false)), Ok(Ok(false)), Ok(Ok(true))]);
    assert!(answers.3, "%Object.prototype%'s prototype changed");
    assert!(
        answers.4,
        "a host object made with Prototype::Null has a prototype"
    );
    assert_eq!(answers.5, Ok(Ok(true)));
    assert!(answers.6, "the host object's prototype did not change");
    assert_eq!(agent.state(), AgentState::Alive);

    // With 2 steps an entry, a new prototype whose chain has 3 objects ends the run before the
    // prototype is set; one whose chain has 2 objects is set in the next entry.
    let mut limits = limits();
    limits.steps_per_entry = 2;
    let mut agent = agent_with(limits).unwrap();
    let realm = support::host::realm(&mut agent);
    // `made[n]` heads a chain of n + 1 objects.
    let (subject, chain_of_two, chain_of_three, rest) = agent
        .with_cx(&realm, |cx| {
            let mut made = Vec::new();
            let mut prototype = Prototype::Null;
            for brand in 0..3 {
                let object = cx
                    .new_host_object(HostObjectSpec {
                        prototype,
                        brand: Brand(brand),
                        data: None,
                        error_kind: None,
                    })
                    .unwrap()
                    .unwrap();
                made.push(cx.root(Value::Object(object)).unwrap());
                prototype = Prototype::Object(object);
            }
            let subject = cx
                .new_host_object(HostObjectSpec {
                    prototype: Prototype::Null,
                    brand: Brand(9),
                    data: None,
                    error_kind: None,
                })
                .unwrap()
                .unwrap();
            let subject = cx.root(Value::Object(subject)).unwrap();
            let chain_of_three = made.remove(2);
            let chain_of_two = made.remove(1);
            (subject, chain_of_two, chain_of_three, made)
        })
        .unwrap();
    let too_long = agent
        .with_cx(&realm, |cx| {
            let subject = object(cx, &subject);
            let three = object(cx, &chain_of_three);
            let answer = cx.set_prototype_of(subject, Some(three));
            (answer, cx.get_prototype_of(subject))
        })
        .unwrap();
    assert_eq!(
        too_long.0,
        Ok(Err(Abrupt::Terminated(Termination::StepBudget)))
    );
    assert_eq!(
        too_long.1,
        Ok(Err(Abrupt::Terminated(Termination::StepBudget)))
    );
    let within = agent
        .with_cx(&realm, |cx| {
            let subject = object(cx, &subject);
            let two = object(cx, &chain_of_two);
            let answer = cx.set_prototype_of(subject, Some(two));
            (answer, cx.get_prototype_of(subject) == Ok(Ok(Some(two))))
        })
        .unwrap();
    assert_eq!(within.0, Ok(Ok(true)));
    assert!(within.1, "the prototype was not set in the next entry");
    drop(rest);
    assert_eq!(agent.state(), AgentState::Alive);
}

/// A host function that answers a throw of 7.
fn fail(_cx: &mut Cx<'_, TestHost>, _call: HostCall<'_>) -> Completion<Value> {
    Err(Abrupt::Throw(Value::Number(7.0)))
}

/// A host function that calls its first argument through `cx.call`, which must throw, catches
/// the throw and answers `undefined`: the host's own catch.
fn swallow(cx: &mut Cx<'_, TestHost>, call: HostCall<'_>) -> Completion<Value> {
    let callee = call.args.first().copied().unwrap_or(Value::Undefined);
    match cx.call(callee, Value::Undefined, &[]).unwrap() {
        Err(Abrupt::Throw(_)) => Ok(Value::Undefined),
        other => panic!("the callee did not throw: {other:?}"),
    }
}

/// The line and column `last_entry_error_site` reads after the script `source` answered
/// `Throw`.
fn thrown_at(
    agent: &mut swell::Agent<TestHost>,
    realm: &swell::Realm,
    source: &str,
) -> Option<(u32, u32)> {
    let Parsed::Parsed(script) = agent
        .parse_script(realm, Source::Utf8(source), None)
        .unwrap()
    else {
        panic!("{source:?} did not parse");
    };
    assert!(
        matches!(agent.evaluate_script(&script), Ok(Outcome::Throw(_))),
        "{source:?} did not throw"
    );
    let site = agent.last_entry_error_site().unwrap()?;
    assert!(!site.dynamic && site.host_defined.is_none());
    Some((site.line, site.column))
}

/// HOST-173 (decision 13), a host function: a throw a host function answers is sited at the
/// call of it in the innermost ECMAScript frame; a throw the host function caught (`cx.call`
/// answered it `Abrupt::Throw` and the function answered `Ok`) is no longer the record, so a
/// later throw of the same value is sited at its own `throw` statement; and `Agent::call` of
/// the host function itself, with no ECMAScript frame, reads `None`.
#[test]
fn a_throw_from_a_host_function_is_sited_at_its_call() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    define_function(&mut agent, &realm, "fail", 0, fail, false);
    define_function(&mut agent, &realm, "swallow", 1, swallow, false);
    assert_eq!(thrown_at(&mut agent, &realm, "1;\n  fail();"), Some((2, 3)));
    assert_eq!(
        thrown_at(
            &mut agent,
            &realm,
            "function g() { throw 3; }\nswallow(g);\n1;\nthrow 3;"
        ),
        Some((4, 1))
    );
    let global_root = global(&mut agent, &realm);
    let (fail_root, undefined) = agent
        .with_cx(&realm, |cx| {
            let global = object(cx, &global_root);
            let fail = get(cx, global, "fail");
            (cx.root(fail).unwrap(), cx.root(Value::Undefined).unwrap())
        })
        .unwrap();
    assert!(thrown_at(&mut agent, &realm, "throw 1;").is_some());
    assert!(matches!(
        agent.call(&realm, &fail_root, &undefined, &[]),
        Ok(Outcome::Throw(_))
    ));
    assert!(agent.last_entry_error_site().unwrap().is_none());
}
