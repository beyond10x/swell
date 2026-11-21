//! Boolean objects (ECMA-262 20.3, `#sec-boolean-objects`), the part slice 2 takes early
//! (`docs/design/engine.md`, section 12, "A part before its slice"), which is all of them: the
//! `Boolean` constructor (`#sec-boolean-constructor-boolean-value`), `Boolean.prototype`
//! (`#sec-boolean.prototype`), `Boolean.prototype.constructor`
//! (`#sec-boolean.prototype.constructor`), `toString` (`#sec-boolean.prototype.tostring`) and
//! `valueOf` (`#sec-boolean.prototype.valueof`) with ThisBooleanValue (`#sec-thisbooleanvalue`).
//! Scripts run in sloppy and in strict code; the new target is given through the host
//! interface. Run also with the feature `collect-every-safe-point`.
#![cfg(test)]

mod support;

use support::host::{TestHost, agent, get, get_object, global, key, object, realm};
use support::script::{Outcome, run_both};
use swell::{Completion, Cx, HostCall, Value};

/// Both modes of `source` answer `expected`, a normal completion as `show` writes it.
fn both_answer(source: &str, expected: &str) {
    let (sloppy, strict) = run_both(source);
    let expected = Outcome::Normal(expected.to_string());
    assert_eq!(sloppy, expected, "sloppy: {source}");
    assert_eq!(strict, expected, "strict: {source}");
}

/// Both modes of `source` throw an error named `name`.
fn both_throw(source: &str, name: &str) {
    let (sloppy, strict) = run_both(source);
    let expected = Outcome::Throw(name.to_string());
    assert_eq!(sloppy, expected, "sloppy: {source}");
    assert_eq!(strict, expected, "strict: {source}");
}

/// A constructor whose "prototype" the test sets; it is only used as a new target.
fn unused(_cx: &mut Cx<'_, TestHost>, call: HostCall<'_>) -> Completion<Value> {
    Ok(Value::Object(call.callee))
}

/// `#sec-boolean-constructor-boolean-value`: called, `Boolean` answers ToBoolean of its
/// argument, which runs no code; constructed, a Boolean object whose [[BooleanData]] is that
/// value and whose [[Prototype]] is `%Boolean.prototype%`, or the new target's "prototype"
/// (OrdinaryCreateFromConstructor). A Boolean object is an object, so ToBoolean of
/// `new Boolean(false)` is true. `Boolean` has length 1, name "Boolean" and the [[Prototype]]
/// `%Function.prototype%` (`#sec-properties-of-the-boolean-constructor`); `Boolean.prototype` is
/// { [[Writable]]: false, [[Enumerable]]: false, [[Configurable]]: false } and a Boolean object
/// whose [[BooleanData]] is false (`#sec-properties-of-the-boolean-prototype-object`).
#[test]
fn boolean_called_and_constructed() {
    both_answer(
        "[Boolean(), Boolean(undefined), Boolean(null), Boolean(0), Boolean(-0), Boolean(NaN), \
          Boolean('')].join();",
        "\"false,false,false,false,false,false,false\"",
    );
    both_answer(
        "[Boolean(1), Boolean('0'), Boolean(' '), Boolean({}), Boolean([]), Boolean(Infinity), \
          Boolean(new Boolean(false)), Boolean(true, false)].join();",
        "\"true,true,true,true,true,true,true,true\"",
    );
    both_answer(
        "var called = false; Boolean({ valueOf: function () { called = true; return 0; } }) + \
         '|' + called;",
        "\"true|false\"",
    );
    both_answer(
        "var b = new Boolean(false); typeof b + '|' + b.valueOf() + '|' + \
         (Object.getPrototypeOf(b) === Boolean.prototype) + '|' + \
         Object.prototype.toString.call(b) + '|' + (b ? 'taken' : 'not taken');",
        "\"object|false|true|[object Boolean]|taken\"",
    );
    both_answer(
        "new Boolean().valueOf() + '|' + new Boolean('x').valueOf() + '|' + \
         new Boolean(0).valueOf();",
        "\"false|true|false\"",
    );
    both_answer(
        "Boolean.length + '|' + Boolean.name + '|' + \
         (Object.getPrototypeOf(Boolean) === Object.getPrototypeOf(Object)) + '|' + \
         (Boolean.prototype.constructor === Boolean) + '|' + typeof Boolean;",
        "\"1|Boolean|true|true|function\"",
    );
    both_answer(
        "var d = Object.getOwnPropertyDescriptor(Boolean, 'prototype'); \
         d.writable + '|' + d.enumerable + '|' + d.configurable;",
        "\"false|false|false\"",
    );
    both_answer(
        "var d = Object.getOwnPropertyDescriptor(Boolean.prototype, 'constructor'); \
         d.writable + '|' + d.enumerable + '|' + d.configurable;",
        "\"true|false|true\"",
    );
    both_answer(
        "Boolean.prototype.valueOf() + '|' + Object.prototype.toString.call(Boolean.prototype) + \
         '|' + (Object.getPrototypeOf(Boolean.prototype) === Object.prototype);",
        "\"false|[object Boolean]|true\"",
    );

    // OrdinaryCreateFromConstructor takes the new target's "prototype".
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let global_root = global(&mut agent, &realm);
    agent
        .with_cx(&realm, |cx| {
            let global = object(cx, &global_root);
            let boolean = get(cx, global, "Boolean");
            let realm_ref = cx.realm_ref(&realm).unwrap();
            let new_target = cx
                .new_host_function(realm_ref, "N", 0, unused, true, None)
                .unwrap()
                .unwrap();
            let prototype = cx.to_object(Value::Number(1.0)).unwrap().unwrap();
            let prototype_key = key(cx, "prototype");
            cx.set(new_target, prototype_key, Value::Object(prototype), true)
                .unwrap()
                .unwrap();
            let made = cx
                .construct(
                    boolean,
                    &[Value::Number(1.0)],
                    Some(Value::Object(new_target)),
                )
                .unwrap()
                .unwrap();
            let object_constructor = get_object(cx, global, "Object");
            let get_prototype_of = get(cx, object_constructor, "getPrototypeOf");
            assert_eq!(
                cx.call(get_prototype_of, Value::Undefined, &[made])
                    .unwrap()
                    .unwrap(),
                Value::Object(prototype)
            );
            let boolean_object = get_object(cx, global, "Boolean");
            let boolean_prototype = get_object(cx, boolean_object, "prototype");
            let value_of = get(cx, boolean_prototype, "valueOf");
            assert_eq!(
                cx.call(value_of, made, &[]).unwrap().unwrap(),
                Value::Boolean(true)
            );
        })
        .unwrap();
}

/// `#sec-boolean.prototype.tostring` and `#sec-boolean.prototype.valueof` with
/// `#sec-thisbooleanvalue`: a Boolean this value or the [[BooleanData]] of a Boolean object;
/// anything else, a Number or String object and an object that only inherits from
/// `Boolean.prototype` included, throws a TypeError. `toString` answers "true" or "false". Both
/// have length 0. ToPrimitive of a Boolean object finds them.
#[test]
fn boolean_prototype_methods_need_a_boolean() {
    both_answer(
        "true.toString() + '|' + false.toString() + '|' + new Boolean(true).toString() + '|' + \
         Boolean.prototype.toString();",
        "\"true|false|true|false\"",
    );
    both_answer(
        "Boolean.prototype.valueOf.call(true) + '|' + Boolean.prototype.toString.call(false) + \
         '|' + Boolean.prototype.valueOf.call(new Boolean(true));",
        "\"true|false|true\"",
    );
    for method in ["toString", "valueOf"] {
        for this in [
            "1",
            "'true'",
            "undefined",
            "null",
            "{}",
            "new Number(1)",
            "new String('true')",
            "Boolean",
        ] {
            both_throw(
                &format!("Boolean.prototype.{method}.call({this});"),
                "TypeError",
            );
        }
        both_throw(
            &format!("var o = {{}}; o.m = Boolean.prototype.{method}; o.m();"),
            "TypeError",
        );
        both_answer(
            &format!(
                "var f = Boolean.prototype.{method}; f.length + '|' + f.name + '|' + \
                 Object.getOwnPropertyDescriptor(Boolean.prototype, '{method}').enumerable;"
            ),
            &format!("\"0|{method}|false\""),
        );
    }
    both_answer(
        "String(new Boolean(true)) + '|' + (new Boolean(false) + '') + '|' + \
         (new Boolean(true) + 1);",
        "\"true|false|2\"",
    );
}
