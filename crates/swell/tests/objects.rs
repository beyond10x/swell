//! Objects through the interface (ECMA-262 10.1, `#sec-ordinary-object-internal-methods-and-internal-slots`;
//! 10.4.3, `#sec-string-exotic-objects`), and the trace of their slots and properties
//! (`docs/design/engine.md`, decision 4, "Collector"). Run also with the feature
//! `collect-every-safe-point`. The cases that need an Array or the list of own keys, which the
//! interface does not give, are unit tests of the library (`object::array::tests`,
//! `object::ordinary::tests`).
#![cfg(test)]

mod support;

use support::host::{TestHost, agent, data, get, key, realm, string, text};
use swell::{Abrupt, Completion, Cx, HostCall, ObjectRef, Outcome, PropertyDescriptor, Value};

/// `define_property_or_throw` as a Boolean: `Ok` true, a `TypeError` false.
fn define(
    cx: &mut Cx<'_, TestHost>,
    object: ObjectRef,
    name: &str,
    desc: PropertyDescriptor,
) -> bool {
    let k = key(cx, name);
    match cx.define_property_or_throw(object, k, desc).unwrap() {
        Ok(()) => true,
        Err(Abrupt::Throw(_)) => false,
        Err(other) => panic!("{other:?}"),
    }
}

/// `set` with `throw` false.
fn set(cx: &mut Cx<'_, TestHost>, object: ObjectRef, name: &str, value: Value) -> bool {
    let k = key(cx, name);
    cx.set(object, k, value, false).unwrap().unwrap()
}

/// `delete_property_or_throw` as a Boolean.
fn delete(cx: &mut Cx<'_, TestHost>, object: ObjectRef, name: &str) -> bool {
    let k = key(cx, name);
    match cx.delete_property_or_throw(object, k).unwrap() {
        Ok(()) => true,
        Err(Abrupt::Throw(_)) => false,
        Err(other) => panic!("{other:?}"),
    }
}

fn desc() -> PropertyDescriptor {
    PropertyDescriptor::default()
}

/// A getter that answers 7.
fn seven(_cx: &mut Cx<'_, TestHost>, _call: HostCall<'_>) -> Completion<Value> {
    Ok(Value::Number(7.0))
}

/// A getter that answers 8.
fn eight(_cx: &mut Cx<'_, TestHost>, _call: HostCall<'_>) -> Completion<Value> {
    Ok(Value::Number(8.0))
}

/// 10.4.3 (`#sec-string-exotic-objects-getownproperty-p`, `#sec-stringgetownproperty`,
/// `#sec-string-exotic-objects-defineownproperty-p-desc`): the integer-indexed properties of a
/// String object are { [[Writable]]: false, [[Enumerable]]: true, [[Configurable]]: false }
/// data properties holding one code unit each; `length` is not writable; other keys are
/// ordinary.
#[test]
fn string_exotic_indices_are_read_only() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    agent
        .with_cx(&realm, |cx| {
            let s = string(cx, "abc");
            let o = cx.to_object(s).unwrap().unwrap();
            let b = get(cx, o, "1");
            assert_eq!(text(cx, b), "b");
            assert_eq!(get(cx, o, "length"), Value::Number(3.0));
            for (name, present) in [
                ("0", true),
                ("2", true),
                ("3", false),
                ("-0", false),
                ("1.0", false),
            ] {
                let k = key(cx, name);
                assert_eq!(cx.has_property(o, k).unwrap(), Ok(present), "{name}");
            }
            assert_eq!(get(cx, o, "-0"), Value::Undefined);

            let x = string(cx, "x");
            assert!(!set(cx, o, "1", x));
            let k = key(cx, "1");
            assert!(matches!(
                cx.set(o, k, x, true).unwrap(),
                Err(Abrupt::Throw(_))
            ));
            let b = get(cx, o, "1");
            assert_eq!(text(cx, b), "b");
            assert!(!delete(cx, o, "1"));

            // The same value, the same attributes: compatible (SameValue compares the code units).
            let other_b = string(cx, "b");
            assert!(define(cx, o, "1", data(other_b, false, true, false)));
            assert!(define(
                cx,
                o,
                "1",
                PropertyDescriptor {
                    value: Some(other_b),
                    ..desc()
                }
            ));
            assert!(!define(
                cx,
                o,
                "1",
                PropertyDescriptor {
                    value: Some(x),
                    ..desc()
                }
            ));
            assert!(!define(
                cx,
                o,
                "1",
                PropertyDescriptor {
                    writable: Some(true),
                    ..desc()
                }
            ));
            assert!(!define(
                cx,
                o,
                "1",
                PropertyDescriptor {
                    enumerable: Some(false),
                    ..desc()
                }
            ));
            assert!(!define(
                cx,
                o,
                "1",
                PropertyDescriptor {
                    configurable: Some(true),
                    ..desc()
                }
            ));
            assert!(!define(
                cx,
                o,
                "1",
                PropertyDescriptor {
                    get: Some(Value::Undefined),
                    ..desc()
                }
            ));

            // length.
            assert!(!set(cx, o, "length", Value::Number(1.0)));
            assert!(!delete(cx, o, "length"));
            assert_eq!(get(cx, o, "length"), Value::Number(3.0));

            // Past the end, and other keys, are ordinary properties of an extensible object.
            let d = string(cx, "d");
            assert!(define(cx, o, "3", data(d, true, true, true)));
            let found = get(cx, o, "3");
            assert_eq!(text(cx, found), "d");
            assert!(set(cx, o, "1.0", x));
            assert!(delete(cx, o, "3"));
        })
        .unwrap();
}

/// `#sec-validateandapplypropertydescriptor` (10.1.6.3), every case on an ordinary object:
/// a new property (step 2: from a generic, a data or an accessor descriptor, with the default
/// attributes); a configurable property may change anything, data to accessor and back with
/// [[Enumerable]] and [[Configurable]] kept (step 6); a non-configurable one refuses
/// [[Configurable]] true, another [[Enumerable]], a change of kind, a new getter or setter, and
/// on a non-writable data property [[Writable]] true and another value by SameValue (step 5);
/// a field the descriptor leaves out keeps its value (step 7). The case of a non-extensible
/// object (step 2.a) is the unit test `object::ordinary::tests::define_on_a_non_extensible_object_fails`.
#[test]
fn define_own_property_validates_every_case_of_10_1_6_3() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    agent
        .with_cx(&realm, |cx| {
            let realm = cx.realm_ref(&realm).unwrap();
            let g = cx
                .new_host_function(realm, "g", 0, seven, false, None)
                .unwrap()
                .unwrap();
            let h = cx
                .new_host_function(realm, "h", 0, eight, false, None)
                .unwrap()
                .unwrap();
            let o = cx.to_object(Value::Boolean(true)).unwrap().unwrap();

            // Step 2: a generic descriptor makes a data property with every default.
            assert!(define(
                cx,
                o,
                "generic",
                PropertyDescriptor {
                    enumerable: Some(true),
                    ..desc()
                }
            ));
            assert_eq!(get(cx, o, "generic"), Value::Undefined);
            assert!(
                !set(cx, o, "generic", Value::Number(1.0)),
                "[[Writable]] false"
            );
            assert!(!delete(cx, o, "generic"), "[[Configurable]] false");
            // Step 2: an accessor descriptor with only [[Get]].
            assert!(define(
                cx,
                o,
                "accessor",
                PropertyDescriptor {
                    get: Some(Value::Object(g)),
                    ..desc()
                }
            ));
            assert_eq!(get(cx, o, "accessor"), Value::Number(7.0));
            assert!(!set(cx, o, "accessor", Value::Number(1.0)), "no setter");
            assert!(!delete(cx, o, "accessor"));

            // Step 6: a configurable data property becomes an accessor and back.
            assert!(define(
                cx,
                o,
                "flexible",
                data(Value::Number(1.0), false, false, true)
            ));
            assert!(define(
                cx,
                o,
                "flexible",
                PropertyDescriptor {
                    get: Some(Value::Object(g)),
                    ..desc()
                }
            ));
            assert_eq!(get(cx, o, "flexible"), Value::Number(7.0));
            // [[Configurable]] true and [[Enumerable]] false were kept: a redefinition of
            // [[Enumerable]] to false is accepted after it is made non-configurable.
            assert!(define(
                cx,
                o,
                "flexible",
                PropertyDescriptor {
                    value: Some(Value::Number(2.0)),
                    ..desc()
                }
            ));
            assert_eq!(get(cx, o, "flexible"), Value::Number(2.0));
            assert!(
                !set(cx, o, "flexible", Value::Number(3.0)),
                "data from accessor: [[Writable]] false"
            );
            assert!(define(
                cx,
                o,
                "flexible",
                PropertyDescriptor {
                    configurable: Some(false),
                    ..desc()
                }
            ));
            assert!(define(
                cx,
                o,
                "flexible",
                PropertyDescriptor {
                    enumerable: Some(false),
                    ..desc()
                }
            ));
            assert!(!define(
                cx,
                o,
                "flexible",
                PropertyDescriptor {
                    enumerable: Some(true),
                    ..desc()
                }
            ));

            // Step 5 on a non-configurable, non-writable data property.
            assert!(define(
                cx,
                o,
                "fixed",
                data(Value::Number(f64::NAN), false, true, false)
            ));
            assert!(
                define(
                    cx,
                    o,
                    "fixed",
                    PropertyDescriptor {
                        value: Some(Value::Number(f64::NAN)),
                        ..desc()
                    }
                ),
                "SameValue(NaN, NaN)"
            );
            assert!(define(cx, o, "fixed", desc()), "an empty descriptor");
            assert!(define(
                cx,
                o,
                "fixed",
                PropertyDescriptor {
                    writable: Some(false),
                    ..desc()
                }
            ));
            assert!(!define(
                cx,
                o,
                "fixed",
                PropertyDescriptor {
                    configurable: Some(true),
                    ..desc()
                }
            ));
            assert!(!define(
                cx,
                o,
                "fixed",
                PropertyDescriptor {
                    enumerable: Some(false),
                    ..desc()
                }
            ));
            assert!(!define(
                cx,
                o,
                "fixed",
                PropertyDescriptor {
                    writable: Some(true),
                    ..desc()
                }
            ));
            assert!(!define(
                cx,
                o,
                "fixed",
                PropertyDescriptor {
                    value: Some(Value::Number(1.0)),
                    ..desc()
                }
            ));
            assert!(!define(
                cx,
                o,
                "fixed",
                PropertyDescriptor {
                    get: Some(Value::Object(g)),
                    ..desc()
                }
            ));
            assert!(define(
                cx,
                o,
                "zero",
                data(Value::Number(0.0), false, true, false)
            ));
            assert!(
                !define(
                    cx,
                    o,
                    "zero",
                    PropertyDescriptor {
                        value: Some(Value::Number(-0.0)),
                        ..desc()
                    }
                ),
                "SameValue(+0, -0) is false"
            );

            // A non-configurable but writable data property: the value may change, and
            // [[Writable]] may go to false, not back.
            assert!(define(
                cx,
                o,
                "writable",
                data(Value::Number(1.0), true, true, false)
            ));
            assert!(define(
                cx,
                o,
                "writable",
                PropertyDescriptor {
                    value: Some(Value::Number(5.0)),
                    ..desc()
                }
            ));
            assert_eq!(get(cx, o, "writable"), Value::Number(5.0));
            assert!(set(cx, o, "writable", Value::Number(6.0)));
            assert!(define(
                cx,
                o,
                "writable",
                PropertyDescriptor {
                    writable: Some(false),
                    ..desc()
                }
            ));
            assert!(!define(
                cx,
                o,
                "writable",
                PropertyDescriptor {
                    writable: Some(true),
                    ..desc()
                }
            ));
            assert!(!define(
                cx,
                o,
                "writable",
                PropertyDescriptor {
                    value: Some(Value::Number(7.0)),
                    ..desc()
                }
            ));
            assert_eq!(get(cx, o, "writable"), Value::Number(6.0));

            // Step 5 on a non-configurable accessor property.
            assert!(define(
                cx,
                o,
                "getter",
                PropertyDescriptor {
                    get: Some(Value::Object(g)),
                    set: Some(Value::Undefined),
                    enumerable: Some(false),
                    configurable: Some(false),
                    ..desc()
                }
            ));
            assert!(define(
                cx,
                o,
                "getter",
                PropertyDescriptor {
                    get: Some(Value::Object(g)),
                    ..desc()
                }
            ));
            assert!(define(
                cx,
                o,
                "getter",
                PropertyDescriptor {
                    set: Some(Value::Undefined),
                    ..desc()
                }
            ));
            assert!(!define(
                cx,
                o,
                "getter",
                PropertyDescriptor {
                    get: Some(Value::Object(h)),
                    ..desc()
                }
            ));
            assert!(!define(
                cx,
                o,
                "getter",
                PropertyDescriptor {
                    set: Some(Value::Object(h)),
                    ..desc()
                }
            ));
            assert!(!define(
                cx,
                o,
                "getter",
                PropertyDescriptor {
                    value: Some(Value::Undefined),
                    ..desc()
                }
            ));
            assert!(!define(
                cx,
                o,
                "getter",
                PropertyDescriptor {
                    enumerable: Some(true),
                    ..desc()
                }
            ));
            assert_eq!(get(cx, o, "getter"), Value::Number(7.0));

            // Step 7: a configurable accessor gets a new getter and keeps its setter-less state.
            assert!(define(
                cx,
                o,
                "swap",
                PropertyDescriptor {
                    get: Some(Value::Object(g)),
                    configurable: Some(true),
                    ..desc()
                }
            ));
            assert!(define(
                cx,
                o,
                "swap",
                PropertyDescriptor {
                    get: Some(Value::Object(h)),
                    ..desc()
                }
            ));
            assert_eq!(get(cx, o, "swap"), Value::Number(8.0));
            assert!(delete(cx, o, "swap"));
            assert_eq!(get(cx, o, "swap"), Value::Undefined);

            // A descriptor that is both data and accessor, or whose getter is not callable, is
            // refused the way ToPropertyDescriptor refuses it.
            assert!(!define(
                cx,
                o,
                "invalid",
                PropertyDescriptor {
                    value: Some(Value::Null),
                    get: Some(Value::Object(g)),
                    ..desc()
                }
            ));
            assert!(!define(
                cx,
                o,
                "invalid",
                PropertyDescriptor {
                    get: Some(Value::Number(1.0)),
                    ..desc()
                }
            ));
        })
        .unwrap();
}

/// The trace of this story's object kinds and of the common record (`docs/design/engine.md`,
/// decision 4): a String wrapper whose `[[StringData]]` is the only reference to its String, and
/// an object whose only reference to a String is a data property and to a function an accessor,
/// each rooted only through the object, read the same after a collection.
#[test]
fn string_data_and_property_values_survive_collect() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let (wrapper, holder) = agent
        .with_cx(&realm, |cx| {
            let realm = cx.realm_ref(&realm).unwrap();
            let units: Vec<u16> = "kept \u{2603} string".encode_utf16().collect();
            let s = cx.new_string_utf16(&units).unwrap();
            let wrapper = cx.to_object(Value::String(s)).unwrap().unwrap();
            let holder = cx.to_object(Value::Boolean(true)).unwrap().unwrap();
            let value = string(cx, "a value held by a property");
            let k = key(cx, "data");
            cx.define_property_or_throw(holder, k, data(value, true, true, true))
                .unwrap()
                .unwrap();
            let getter = cx
                .new_host_function(realm, "seven", 0, seven, false, None)
                .unwrap()
                .unwrap();
            let k = key(cx, "accessor");
            let accessor = PropertyDescriptor {
                get: Some(Value::Object(getter)),
                set: Some(Value::Undefined),
                enumerable: Some(true),
                configurable: Some(true),
                ..desc()
            };
            cx.define_property_or_throw(holder, k, accessor)
                .unwrap()
                .unwrap();
            (
                cx.root(Value::Object(wrapper)).unwrap(),
                cx.root(Value::Object(holder)).unwrap(),
            )
        })
        .unwrap();
    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));

    let (read, value, accessor) = agent
        .with_cx(&realm, |cx| {
            let wrapper = support::host::object(cx, &wrapper);
            let holder = support::host::object(cx, &holder);
            let Value::Number(length) = get(cx, wrapper, "length") else {
                panic!("no length");
            };
            let mut read = String::new();
            for i in 0..length as u32 {
                let unit = get(cx, wrapper, &i.to_string());
                read.push_str(&text(cx, unit));
            }
            let value = get(cx, holder, "data");
            (read, text(cx, value), get(cx, holder, "accessor"))
        })
        .unwrap();
    assert_eq!(read, "kept \u{2603} string");
    assert_eq!(value, "a value held by a property");
    assert_eq!(accessor, Value::Number(7.0));
}
