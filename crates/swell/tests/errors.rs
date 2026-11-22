//! Error objects (ECMA-262 20.5, `#sec-error-objects`): the `Error` constructor
//! (`#sec-error-constructor`), `Error.prototype.message`, `.name` and `.toString`
//! (`#sec-error.prototype.tostring`), InstallErrorCause (`#sec-installerrorcause`) and the six
//! NativeError constructors (`#sec-native-error-types-used-in-this-standard`,
//! `#sec-nativeerror-constructors`), read through the global object of a realm. And the host's
//! read of an error object (`docs/design/host-interface.md`, decision 13, HOST-179): the kind
//! [[ErrorData]] records and the own `message` data property, read without running code, of an
//! engine error and of a host object made with an `error_kind` (decision 15).
#![cfg(test)]

mod support;

use support::host::{
    TestHost, agent, agent_with, data, get, get_object, global, key, limits, object, realm, string,
    text, thrown,
};
use support::script::{Outcome, eval, run};
use swell::{
    Abrupt, Agent, Brand, Completion, Cx, ErrorKind, ErrorSummary, GlobalObjects, HookCx, HostCall,
    HostObjectSpec, ObjectRef, Outcome as HostOutcome, PropertyDescriptor, Prototype, Realm,
    RealmRef, Root, Value,
};

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

/// The root of the value `source` throws in `realm`.
fn thrown_root(agent: &mut Agent<TestHost>, realm: &Realm, source: &str) -> Root {
    match eval(agent, realm, source) {
        HostOutcome::Throw(root) => root,
        HostOutcome::Normal(_) => panic!("{source} completed normally"),
        HostOutcome::Terminated(termination) => panic!("{source} ended in {termination:?}"),
    }
}

/// The root of the value `source` completes with normally in `realm`.
fn normal_root(agent: &mut Agent<TestHost>, realm: &Realm, source: &str) -> Root {
    match eval(agent, realm, source) {
        HostOutcome::Normal(root) => root,
        HostOutcome::Throw(_) => panic!("{source} threw"),
        HostOutcome::Terminated(termination) => panic!("{source} ended in {termination:?}"),
    }
}

/// `Agent::error_summary` of `root`, with `last_entry_steps()`, `heap_bytes()`, `live_bytes()`
/// and the count of hook calls read before and after it: the read changes none of them.
fn read_unchanged(agent: &mut Agent<TestHost>, root: &Root) -> Option<ErrorSummary> {
    let before = (
        agent.last_entry_steps().unwrap(),
        agent.heap_bytes().unwrap(),
        agent.live_bytes().unwrap(),
        agent.host().calls.len(),
    );
    let summary = agent.error_summary(root).unwrap();
    let after = (
        agent.last_entry_steps().unwrap(),
        agent.heap_bytes().unwrap(),
        agent.live_bytes().unwrap(),
        agent.host().calls.len(),
    );
    assert_eq!(after, before, "steps, heap bytes, live bytes, hook calls");
    summary
}

/// A host object of `realm` with the error kind `error_kind` and an own data property
/// `message` holding `"m"`, defined by the host.
fn host_error_object(
    cx: &mut Cx<'_, TestHost>,
    realm: RealmRef,
    error_kind: Option<ErrorKind>,
) -> ObjectRef {
    let object = cx
        .new_host_object(HostObjectSpec {
            prototype: Prototype::ObjectPrototypeOf(realm),
            brand: Brand(179),
            data: None,
            error_kind,
        })
        .unwrap()
        .unwrap();
    let k = key(cx, "message");
    let m = string(cx, "m");
    cx.define_property_or_throw(object, k, data(m, true, false, true))
        .unwrap()
        .unwrap();
    object
}

/// HOST-179 (decision 13): `error_summary` answers the kind [[ErrorData]] records and the own
/// `message` data property's code units, read from the property list alone: the same after the
/// error's prototype is set to `null` and an inherited getter `message` is defined; `message:
/// None` for an own accessor (its getter never runs), for an inherited `message`; a host object
/// made with `error_kind: Some(TypeError)` answers its kind and own message, and is an error
/// for `Error.isError` and `Object.prototype.toString`; `None` for a thrown primitive, a plain
/// object and a host object made with `error_kind: None`. Through `Agent`, `Cx` and the
/// `HookCx` of a setup closure, with no step, no heap byte and no hook call.
#[test]
fn host_179_an_error_is_read_without_running_code() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let global_root = global(&mut agent, &realm);
    let type_error = ErrorSummary {
        kind: ErrorKind::TypeError,
        message: Some(vec![0x61, 0xD800]),
    };

    let thrown = thrown_root(&mut agent, &realm, "throw new TypeError(\"a\\uD800\");");
    assert_eq!(
        read_unchanged(&mut agent, &thrown),
        Some(type_error.clone())
    );

    // The script sets the error's prototype to null and defines a getter `message` on
    // `Object.prototype`; another error's own `message` is an accessor whose getter throws.
    agent
        .with_cx(&realm, |cx| {
            let global = object(cx, &global_root);
            let error = cx.get(&thrown).unwrap();
            let k = key(cx, "e");
            cx.set(global, k, error, true).unwrap().unwrap();
        })
        .unwrap();
    let accessor = normal_root(
        &mut agent,
        &realm,
        "var count = 0; \
         Object.setPrototypeOf(e, null); \
         Object.defineProperty(Object.prototype, 'message', \
           { get: function () { count++; throw 0; }, configurable: true }); \
         var a = new RangeError('x'); \
         Object.defineProperty(a, 'message', { get: function () { count++; throw 1; } }); \
         a;",
    );
    let bare = normal_root(&mut agent, &realm, "new RangeError();");
    assert_eq!(
        read_unchanged(&mut agent, &thrown),
        Some(type_error.clone())
    );
    let range_error = ErrorSummary {
        kind: ErrorKind::RangeError,
        message: None,
    };
    assert_eq!(
        read_unchanged(&mut agent, &accessor),
        Some(range_error.clone())
    );
    // `bare` inherits `message` ("" of `RangeError.prototype`): it is not read.
    assert_eq!(read_unchanged(&mut agent, &bare), Some(range_error.clone()));

    // A thrown primitive and a plain object with an own `message`.
    let one = thrown_root(&mut agent, &realm, "throw 1;");
    assert_eq!(read_unchanged(&mut agent, &one), None);
    let plain = normal_root(&mut agent, &realm, "({ message: 'm' });");
    assert_eq!(read_unchanged(&mut agent, &plain), None);

    // Host objects, with and without an error kind, each with an own `message` "m".
    let (with_kind, without_kind) = agent
        .with_cx(&realm, |cx| {
            let realm_ref = cx.realm_ref(&realm).unwrap();
            let global = object(cx, &global_root);
            let with_kind = host_error_object(cx, realm_ref, Some(ErrorKind::TypeError));
            let without_kind = host_error_object(cx, realm_ref, None);
            for (name, made) in [("h", with_kind), ("p", without_kind)] {
                let k = key(cx, name);
                cx.set(global, k, Value::Object(made), true)
                    .unwrap()
                    .unwrap();
            }
            (
                cx.root(Value::Object(with_kind)).unwrap(),
                cx.root(Value::Object(without_kind)).unwrap(),
            )
        })
        .unwrap();
    let host_type_error = ErrorSummary {
        kind: ErrorKind::TypeError,
        message: Some(vec![0x6D]),
    };
    assert_eq!(
        read_unchanged(&mut agent, &with_kind),
        Some(host_type_error.clone())
    );
    assert_eq!(read_unchanged(&mut agent, &without_kind), None);

    // Through `Cx`: the same answers, no hook called.
    agent
        .with_cx(&realm, |cx| {
            let calls = cx.host().calls.len();
            let cases = [
                (&thrown, Some(type_error.clone())),
                (&accessor, Some(range_error.clone())),
                (&bare, Some(range_error.clone())),
                (&one, None),
                (&plain, None),
                (&with_kind, Some(host_type_error.clone())),
                (&without_kind, None),
            ];
            for (root, expected) in cases {
                let value = cx.get(root).unwrap();
                assert_eq!(cx.error_summary(value).unwrap(), expected);
            }
            assert_eq!(cx.error_summary(Value::Undefined).unwrap(), None);
            assert_eq!(cx.host().calls.len(), calls);
        })
        .unwrap();

    // Through the `HookCx` of a second realm's setup closure, from a root it captured.
    let mut seen = Vec::new();
    let mut setup = |cx: &mut HookCx<'_, TestHost>, realm: RealmRef| -> Completion<GlobalObjects> {
        let value = cx.get(&thrown).unwrap();
        seen.push(cx.error_summary(value).unwrap());
        let value = cx.get(&with_kind).unwrap();
        seen.push(cx.error_summary(value).unwrap());
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
    let second = agent.create_realm(Some(&mut setup), None).unwrap();
    assert!(matches!(second, HostOutcome::Normal(_)), "{second:?}");
    assert_eq!(seen, vec![Some(type_error), Some(host_type_error)]);

    // No getter ran; the host object is an error for the engine, the other one is not.
    assert_eq!(
        support::script::eval_text(
            &mut agent,
            &realm,
            "[count, Error.isError(h), Object.prototype.toString.call(h), \
              Error.isError(p), Object.prototype.toString.call(p)].join();",
        ),
        Outcome::Normal("\"0,true,[object Error],false,[object Object]\"".to_string())
    );
}

/// Each `ErrorKind` is recorded by the error that has it: the errors the engine throws (a
/// `TypeError` of `null.x`, a `RangeError` of `frame_depth`, a `SyntaxError` of `eval`, a
/// `ReferenceError` of an unbound name, a `URIError` of `decodeURI`), an `EvalError` and an
/// `Error` made by their constructors, an error the host makes with `cx.new_error`, and a
/// `TypeError` constructed with `RangeError` as the new target (the kind is the constructor's,
/// whatever prototype the new target gives).
#[test]
fn every_engine_error_records_its_kind() {
    let mut limits = limits();
    limits.frame_depth = 64;
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    let global_root = global(&mut agent, &realm);
    let cases = [
        (ErrorKind::Error, "throw new Error('e');"),
        (ErrorKind::EvalError, "throw new EvalError('e');"),
        (ErrorKind::RangeError, "function f() { return f(); } f();"),
        (ErrorKind::ReferenceError, "unboundName;"),
        (ErrorKind::SyntaxError, "eval('var 1');"),
        (ErrorKind::TypeError, "null.x;"),
        (ErrorKind::UriError, "decodeURI('%');"),
    ];
    assert_eq!(cases.map(|(kind, _)| kind), ErrorKind::ALL);
    for (kind, source) in cases {
        let root = thrown_root(&mut agent, &realm, source);
        let summary = agent.error_summary(&root).unwrap();
        let Some(summary) = summary else {
            panic!("{source}: no [[ErrorData]]");
        };
        assert_eq!(summary.kind, kind, "{source}");
        assert!(summary.message.is_some(), "{source}");
    }

    agent
        .with_cx(&realm, |cx| {
            let realm_ref = cx.realm_ref(&realm).unwrap();
            for kind in ErrorKind::ALL {
                let made = cx.new_error(realm_ref, kind, "m").unwrap().unwrap();
                assert_eq!(
                    cx.error_summary(Value::Object(made)).unwrap(),
                    Some(ErrorSummary {
                        kind,
                        message: Some(vec![0x6D]),
                    })
                );
            }
            let global = object(cx, &global_root);
            let type_error = get(cx, global, "TypeError");
            let range_error = get(cx, global, "RangeError");
            let message = string(cx, "m");
            let made = cx
                .construct(type_error, &[message], Some(range_error))
                .unwrap()
                .unwrap();
            let Value::Object(made) = made else {
                panic!("{made:?}");
            };
            let Value::Object(range_object) = range_error else {
                panic!("{range_error:?}");
            };
            let range_prototype = get_object(cx, range_object, "prototype");
            assert_eq!(
                cx.get_prototype_of(made).unwrap().unwrap(),
                Some(range_prototype)
            );
            assert_eq!(
                cx.error_summary(Value::Object(made)).unwrap(),
                Some(ErrorSummary {
                    kind: ErrorKind::TypeError,
                    message: Some(vec![0x6D]),
                })
            );
        })
        .unwrap();
}

/// A guard of what stays as it is: `Error.isError` and `Object.prototype.toString` of each kind
/// of engine error answer true and "[object Error]", and of an error's prototype and an object
/// that inherits from one false and "[object Object]".
#[test]
fn error_is_error_and_to_string_are_unchanged() {
    let mut limits = limits();
    limits.frame_depth = 64;
    let source = "var errors = []; \
         errors.push(new Error('e')); \
         errors.push(EvalError('e')); \
         function f() { return f(); } try { f(); } catch (x) { errors.push(x); } \
         try { unboundName; } catch (x) { errors.push(x); } \
         try { eval('var 1'); } catch (x) { errors.push(x); } \
         try { null.x; } catch (x) { errors.push(x); } \
         try { decodeURI('%'); } catch (x) { errors.push(x); } \
         errors.push(TypeError.prototype); \
         errors.push(Object.create(RangeError.prototype)); \
         var out = []; \
         for (var i = 0; i < errors.length; i++) { \
           out.push(Error.isError(errors[i]) + ' ' + \
             Object.prototype.toString.call(errors[i]) + ' ' + errors[i].name); \
         } \
         out.join();";
    assert_eq!(
        run(limits, source),
        Outcome::Normal(
            "\"true [object Error] Error,true [object Error] EvalError,\
             true [object Error] RangeError,true [object Error] ReferenceError,\
             true [object Error] SyntaxError,true [object Error] TypeError,\
             true [object Error] URIError,false [object Object] TypeError,\
             false [object Object] RangeError\""
                .to_string()
        )
    );
}
