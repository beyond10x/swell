//! Error objects (ECMA-262 20.5, `#sec-error-objects`): the `Error` constructor
//! (`#sec-error-constructor`), `Error.prototype.message`, `.name` and `.toString`
//! (`#sec-error.prototype.tostring`), InstallErrorCause (`#sec-installerrorcause`) and the six
//! NativeError constructors (`#sec-native-error-types-used-in-this-standard`,
//! `#sec-nativeerror-constructors`), read through the global object of a realm.
#![cfg(test)]

mod support;

use support::host::{
    TestHost, agent, data, get, get_object, global, key, object, realm, string, text, thrown,
};
use swell::{Abrupt, Completion, Cx, HostCall, ObjectRef, PropertyDescriptor, Value};

const ERRORS: [&str; 7] = [
    "Error",
    "EvalError",
    "RangeError",
    "ReferenceError",
    "SyntaxError",
    "TypeError",
    "URIError",
];

/// An options object `{ cause: cause }` (InstallErrorCause reads it with HasProperty and Get).
fn options(cx: &mut Cx<'_, TestHost>, cause: Value) -> ObjectRef {
    let options = cx.to_object(Value::Boolean(true)).unwrap().unwrap();
    let k = key(cx, "cause");
    cx.define_property_or_throw(options, k, data(cause, true, true, true))
        .unwrap()
        .unwrap();
    options
}

/// Whether `object` has an own property `name`: deleting it changes what `Get` reads.
fn own_value(cx: &mut Cx<'_, TestHost>, object: ObjectRef, name: &str) -> Option<Value> {
    let k = key(cx, name);
    if !cx.has_property(object, k).unwrap().unwrap() {
        return None;
    }
    let value = get(cx, object, name);
    cx.delete_property_or_throw(object, k).unwrap().unwrap();
    let after = get(cx, object, name);
    // Put it back as a non-enumerable data property, as CreateNonEnumerableDataPropertyOrThrow
    // made it.
    cx.define_property_or_throw(object, k, data(value, true, false, true))
        .unwrap()
        .unwrap();
    (after != value).then_some(value)
}

/// `#sec-error-constructor` and `#sec-nativeerror-constructors`: called or constructed, each
/// constructor makes an object whose prototype is its own `prototype`, with an own `message`
/// when a message is given (converted by ToString) and none when it is undefined, and with an
/// own `cause` exactly when the options object has a `cause` property.
#[test]
fn error_constructors_set_message_and_cause() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let global_root = global(&mut agent, &realm);
    agent
        .with_cx(&realm, |cx| {
            let global = object(cx, &global_root);
            for name in ERRORS {
                let constructor = get(cx, global, name);
                let Value::Object(constructor_object) = constructor else {
                    panic!("{name} is not an object");
                };
                let prototype = get_object(cx, constructor_object, "prototype");
                let message = string(cx, "went wrong");
                let with_cause = options(cx, Value::Number(42.0));

                let made = cx
                    .construct(constructor, &[message, Value::Object(with_cause)], None)
                    .unwrap()
                    .unwrap();
                let Value::Object(made) = made else {
                    panic!("{name} made {made:?}");
                };
                assert_eq!(get_object(cx, made, "constructor"), constructor_object);
                let found = own_value(cx, made, "message").unwrap();
                assert_eq!(text(cx, found), "went wrong", "{name}");
                assert_eq!(own_value(cx, made, "cause"), Some(Value::Number(42.0)));
                // The prototype is the constructor's "prototype": a property put there is seen.
                let probe = key(cx, "probe");
                cx.define_property_or_throw(
                    prototype,
                    probe,
                    data(Value::Number(1.0), true, true, true),
                )
                .unwrap()
                .unwrap();
                assert_eq!(get(cx, made, "probe"), Value::Number(1.0));
                cx.delete_property_or_throw(prototype, probe)
                    .unwrap()
                    .unwrap();

                // Called as a function, with a message that ToString converts, an undefined
                // cause that is still installed, and no options.
                let undefined_cause = options(cx, Value::Undefined);
                let called = cx
                    .call(
                        constructor,
                        Value::Undefined,
                        &[Value::Number(12.0), Value::Object(undefined_cause)],
                    )
                    .unwrap()
                    .unwrap();
                let Value::Object(called) = called else {
                    panic!("{name} called made {called:?}");
                };
                let found = own_value(cx, called, "message").unwrap();
                assert_eq!(text(cx, found), "12");
                let k = key(cx, "cause");
                assert_eq!(cx.has_property(called, k).unwrap(), Ok(true));
                assert_eq!(get(cx, called, "cause"), Value::Undefined);

                let bare = cx.construct(constructor, &[], None).unwrap().unwrap();
                let Value::Object(bare) = bare else {
                    panic!("{name} made {bare:?}");
                };
                assert_eq!(own_value(cx, bare, "message"), None);
                let inherited = get(cx, bare, "message");
                assert_eq!(text(cx, inherited), "");
                assert_eq!(cx.has_property(bare, k).unwrap(), Ok(false));

                // A primitive options value installs no cause.
                let primitive = cx
                    .construct(constructor, &[Value::Undefined, Value::Number(1.0)], None)
                    .unwrap()
                    .unwrap();
                let Value::Object(primitive) = primitive else {
                    panic!("{primitive:?}");
                };
                assert_eq!(cx.has_property(primitive, k).unwrap(), Ok(false));
            }
        })
        .unwrap();
}

/// `#sec-properties-of-the-nativeerror-prototype-objects` and
/// `#sec-properties-of-the-nativeerror-constructors`: each NativeError constructor has
/// `%Error%` as its prototype and a `name` of its own; each NativeError prototype has
/// `%Error.prototype%` as its prototype, its own `name`, an empty `message` and a `constructor`
/// that is its constructor, and inherits `toString`.
#[test]
fn native_errors_inherit_from_error() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let global_root = global(&mut agent, &realm);
    agent
        .with_cx(&realm, |cx| {
            let global = object(cx, &global_root);
            let error = get_object(cx, global, "Error");
            let error_prototype = get_object(cx, error, "prototype");
            let to_string = get(cx, error_prototype, "toString");
            assert!(matches!(to_string, Value::Object(_)));
            let probe = key(cx, "probe");
            cx.define_property_or_throw(error, probe, data(Value::Number(1.0), true, true, true))
                .unwrap()
                .unwrap();
            cx.define_property_or_throw(
                error_prototype,
                probe,
                data(Value::Number(2.0), true, true, true),
            )
            .unwrap()
            .unwrap();
            for name in &ERRORS[1..] {
                let constructor = get_object(cx, global, name);
                let prototype = get_object(cx, constructor, "prototype");
                assert_ne!(prototype, error_prototype);
                assert_eq!(get(cx, constructor, "probe"), Value::Number(1.0), "{name}");
                assert_eq!(get(cx, prototype, "probe"), Value::Number(2.0), "{name}");
                assert_eq!(get(cx, prototype, "toString"), to_string);
                assert_eq!(get_object(cx, prototype, "constructor"), constructor);
                let found = get(cx, prototype, "name");
                assert_eq!(text(cx, found), *name);
                let found = get(cx, constructor, "name");
                assert_eq!(text(cx, found), *name);
                assert_eq!(get(cx, constructor, "length"), Value::Number(1.0));
                let message = get(cx, prototype, "message");
                assert_eq!(text(cx, message), "");
                // NativeError.prototype is { [[Writable]]: false, [[Enumerable]]: false,
                // [[Configurable]]: false }.
                let k = key(cx, "prototype");
                assert_eq!(
                    cx.set(constructor, k, Value::Null, false).unwrap(),
                    Ok(false)
                );
                assert!(matches!(
                    cx.delete_property_or_throw(constructor, k).unwrap(),
                    Err(Abrupt::Throw(_))
                ));
            }
        })
        .unwrap();
}

/// A getter that throws its own `this`.
fn throwing_getter(_cx: &mut Cx<'_, TestHost>, call: HostCall<'_>) -> Completion<Value> {
    Err(Abrupt::Throw(call.this))
}

/// `#sec-error.prototype.tostring`: name and message, each defaulted and converted, joined by
/// ": " when both are non-empty; a this value that is not an object throws a `TypeError`; an
/// abrupt Get is passed on.
#[test]
fn error_prototype_to_string() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let global_root = global(&mut agent, &realm);
    agent
        .with_cx(&realm, |cx| {
            let realm_ref = cx.realm_ref(&realm).unwrap();
            let global = object(cx, &global_root);
            let error = get_object(cx, global, "Error");
            let error_prototype = get_object(cx, error, "prototype");
            let to_string = get(cx, error_prototype, "toString");

            let cases: [(Option<Value>, Option<Value>, &str); 7] = [
                (None, None, "Error"),
                (Some(Value::Undefined), Some(Value::Undefined), "Error"),
                (Some(Value::Number(1.0)), None, "1"),
                (None, Some(Value::Boolean(true)), "Error: true"),
                (Some(Value::Null), Some(Value::Null), "null: null"),
                (
                    Some(Value::Number(0.5)),
                    Some(Value::Number(-0.0)),
                    "0.5: 0",
                ),
                (Some(Value::Undefined), Some(Value::Number(2.0)), "Error: 2"),
            ];
            for (name, message, expected) in cases {
                let this = cx.to_object(Value::Boolean(true)).unwrap().unwrap();
                for (property, value) in [("name", name), ("message", message)] {
                    if let Some(value) = value {
                        let k = key(cx, property);
                        cx.define_property_or_throw(this, k, data(value, true, true, true))
                            .unwrap()
                            .unwrap();
                    }
                }
                let answer = cx
                    .call(to_string, Value::Object(this), &[])
                    .unwrap()
                    .unwrap();
                assert_eq!(text(cx, answer), expected);
            }

            // Empty name: the message alone; empty message: the name alone.
            for (name, message, expected) in [("", "m", "m"), ("n", "", "n"), ("", "", "")] {
                let this = cx.to_object(Value::Boolean(true)).unwrap().unwrap();
                let name = string(cx, name);
                let message = string(cx, message);
                let k = key(cx, "name");
                cx.define_property_or_throw(this, k, data(name, true, true, true))
                    .unwrap()
                    .unwrap();
                let k = key(cx, "message");
                cx.define_property_or_throw(this, k, data(message, true, true, true))
                    .unwrap()
                    .unwrap();
                let answer = cx
                    .call(to_string, Value::Object(this), &[])
                    .unwrap()
                    .unwrap();
                assert_eq!(text(cx, answer), expected);
            }

            // An error object made by a constructor.
            let range_error = get(cx, global, "RangeError");
            let message = string(cx, "too far");
            let made = cx
                .construct(range_error, &[message], None)
                .unwrap()
                .unwrap();
            let answer = cx.call(to_string, made, &[]).unwrap().unwrap();
            assert_eq!(text(cx, answer), "RangeError: too far");

            // A this value that is not an object.
            for this in [Value::Undefined, Value::Number(1.0)] {
                let error = thrown(cx.call(to_string, this, &[]).unwrap());
                assert!(support::host::is_error_of(cx, global, error, "TypeError"));
            }

            // An abrupt Get: a getter that throws.
            let this = cx.to_object(Value::Boolean(true)).unwrap().unwrap();
            let getter = cx
                .new_host_function(realm_ref, "get", 0, throwing_getter, false, None)
                .unwrap()
                .unwrap();
            let k = key(cx, "message");
            let accessor = PropertyDescriptor {
                get: Some(Value::Object(getter)),
                set: Some(Value::Undefined),
                enumerable: Some(false),
                configurable: Some(true),
                ..PropertyDescriptor::default()
            };
            cx.define_property_or_throw(this, k, accessor)
                .unwrap()
                .unwrap();
            assert_eq!(
                cx.call(to_string, Value::Object(this), &[]).unwrap(),
                Err(Abrupt::Throw(Value::Object(this)))
            );
        })
        .unwrap();
}
