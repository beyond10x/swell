//! The String builtins the harness files call (ECMA-262 22.1, `#sec-string-objects`): the
//! `String` constructor (`#sec-string-constructor-string-value`) and
//! `String.prototype.indexOf` (`#sec-string.prototype.indexof`) with StringIndexOf
//! (`#sec-stringindexof`). No script runs. A Symbol argument needs a well-known symbol, which
//! the host interface does not hand out, so that case is a unit test of `builtins/string.rs`.
#![cfg(test)]

mod support;

use support::host::{
    TestHost, agent, get, get_object, global, is_error_of, key, object, realm, string, text, thrown,
};
use swell::{Completion, Cx, HostCall, ObjectRef, Value};

/// A constructor whose "prototype" the test sets; it is only used as a new target.
fn unused(_cx: &mut Cx<'_, TestHost>, call: HostCall<'_>) -> Completion<Value> {
    Ok(Value::Object(call.callee))
}

/// A toString that answers "custom".
fn custom(cx: &mut Cx<'_, TestHost>, _call: HostCall<'_>) -> Completion<Value> {
    Ok(Value::String(cx.new_string("custom").unwrap()))
}

/// `#sec-string-constructor-string-value`: called with no argument it answers the empty
/// String, else ToString of its argument; constructed it answers a String object
/// (`#sec-stringcreate`) of that String whose "length" is its length and whose [[Prototype]]
/// is `%String.prototype%`, or the new target's "prototype"; `String.prototype.constructor` is
/// `String`, whose length is 1.
#[test]
fn string_called_and_constructed() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let global_root = global(&mut agent, &realm);
    agent
        .with_cx(&realm, |cx| {
            let global = object(cx, &global_root);
            let string_constructor = get_object(cx, global, "String");
            let constructor = Value::Object(string_constructor);
            let called = |cx: &mut Cx<'_, TestHost>, args: &[Value]| {
                let value = cx
                    .call(constructor, Value::Undefined, args)
                    .unwrap()
                    .unwrap();
                text(cx, value)
            };
            assert_eq!(called(cx, &[]), "");
            assert_eq!(called(cx, &[Value::Undefined]), "undefined");
            assert_eq!(called(cx, &[Value::Null]), "null");
            assert_eq!(called(cx, &[Value::Number(-0.0)]), "0");
            assert_eq!(called(cx, &[Value::Number(1.5), Value::Null]), "1.5");
            assert_eq!(called(cx, &[Value::Boolean(true)]), "true");
            // An object goes through ToPrimitive with hint String: its own toString answers.
            let object_constructor = get(cx, global, "Object");
            let Value::Object(texted) = cx
                .call(object_constructor, Value::Undefined, &[])
                .unwrap()
                .unwrap()
            else {
                panic!("Object() made no object");
            };
            let realm_ref = cx.realm_ref(&realm).unwrap();
            let method = cx
                .new_host_function(realm_ref, "toString", 0, custom, false, None)
                .unwrap()
                .unwrap();
            let to_string_key = key(cx, "toString");
            cx.set(texted, to_string_key, Value::Object(method), true)
                .unwrap()
                .unwrap();
            assert_eq!(called(cx, &[Value::Object(texted)]), "custom");

            let ab = string(cx, "ab");
            let made = cx.construct(constructor, &[ab], None).unwrap().unwrap();
            let Value::Object(made) = made else {
                panic!("{made:?}");
            };
            assert_eq!(get(cx, made, "length"), Value::Number(2.0));
            let unit = get(cx, made, "1");
            assert_eq!(text(cx, unit), "b");
            let object_constructor = get_object(cx, global, "Object");
            let get_prototype_of = get(cx, object_constructor, "getPrototypeOf");
            let string_prototype = get(cx, string_constructor, "prototype");
            assert_eq!(
                cx.call(get_prototype_of, Value::Undefined, &[Value::Object(made)])
                    .unwrap()
                    .unwrap(),
                string_prototype
            );
            let object_prototype = get_object(cx, object_constructor, "prototype");
            let to_string = get(cx, object_prototype, "toString");
            let tag = cx
                .call(to_string, Value::Object(made), &[])
                .unwrap()
                .unwrap();
            assert_eq!(text(cx, tag), "[object String]");
            let empty = cx.construct(constructor, &[], None).unwrap().unwrap();
            let Value::Object(empty) = empty else {
                panic!("{empty:?}");
            };
            assert_eq!(get(cx, empty, "length"), Value::Number(0.0));

            // Another new target gives the prototype.
            let realm_ref = cx.realm_ref(&realm).unwrap();
            let new_target = cx
                .new_host_function(realm_ref, "N", 0, unused, true, None)
                .unwrap()
                .unwrap();
            let prototype = cx.to_object(Value::Boolean(true)).unwrap().unwrap();
            let prototype_key = key(cx, "prototype");
            cx.set(new_target, prototype_key, Value::Object(prototype), true)
                .unwrap()
                .unwrap();
            let derived = cx
                .construct(constructor, &[ab], Some(Value::Object(new_target)))
                .unwrap()
                .unwrap();
            assert_eq!(
                cx.call(get_prototype_of, Value::Undefined, &[derived])
                    .unwrap()
                    .unwrap(),
                Value::Object(prototype)
            );

            let Value::Object(string_prototype) = string_prototype else {
                panic!("{string_prototype:?}");
            };
            assert_eq!(get(cx, string_prototype, "constructor"), constructor);
            assert_eq!(get(cx, string_constructor, "length"), Value::Number(1.0));
            let name = get(cx, string_constructor, "name");
            assert_eq!(text(cx, name), "String");
        })
        .unwrap();
}

/// `String.prototype.indexOf` of `this` and `args`.
fn index_of(
    cx: &mut Cx<'_, TestHost>,
    string_prototype: ObjectRef,
    this: Value,
    args: &[Value],
) -> Completion<Value> {
    let method = get(cx, string_prototype, "indexOf");
    cx.call(method, this, args).unwrap()
}

/// `#sec-string.prototype.indexof` with `#sec-stringindexof`: the first index at or after the
/// position, which is clamped to 0 and the length (NaN, negative, +∞ and fractional ones); -1
/// when not found; the empty search string is found at the clamped position, the length
/// included; the search string and the this value go through ToString (undefined is
/// "undefined", a Number its digits); a this value of undefined or null throws a TypeError; code
/// units are compared, lone surrogates included.
#[test]
fn index_of_positions() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let global_root = global(&mut agent, &realm);
    agent
        .with_cx(&realm, |cx| {
            let global = object(cx, &global_root);
            let string_constructor = get_object(cx, global, "String");
            let prototype = get_object(cx, string_constructor, "prototype");
            let at = |cx: &mut Cx<'_, TestHost>, this: Value, args: &[Value]| {
                index_of(cx, prototype, this, args).unwrap()
            };
            let [abcabc, c, empty, missing] = ["abcabc", "c", "", "x"].map(|text| string(cx, text));
            let n = Value::Number;
            assert_eq!(at(cx, abcabc, &[c]), n(2.0));
            assert_eq!(at(cx, abcabc, &[c, n(3.0)]), n(5.0));
            assert_eq!(at(cx, abcabc, &[c, n(2.9)]), n(2.0));
            assert_eq!(at(cx, abcabc, &[c, n(6.0)]), n(-1.0));
            assert_eq!(at(cx, abcabc, &[c, n(-5.0)]), n(2.0));
            assert_eq!(at(cx, abcabc, &[c, n(f64::NAN)]), n(2.0));
            assert_eq!(at(cx, abcabc, &[c, n(f64::INFINITY)]), n(-1.0));
            assert_eq!(at(cx, abcabc, &[missing]), n(-1.0));
            assert_eq!(at(cx, abcabc, &[empty, n(4.0)]), n(4.0));
            assert_eq!(at(cx, abcabc, &[empty, n(6.0)]), n(6.0));
            assert_eq!(at(cx, abcabc, &[empty, n(7.0)]), n(6.0));
            assert_eq!(at(cx, abcabc, &[empty, n(f64::INFINITY)]), n(6.0));
            assert_eq!(at(cx, empty, &[empty]), n(0.0));
            let long = string(cx, "abcabcd");
            assert_eq!(at(cx, abcabc, &[long]), n(-1.0));
            let undefined_text = string(cx, "xundefined");
            assert_eq!(at(cx, undefined_text, &[]), n(1.0));
            assert_eq!(at(cx, n(12312.0), &[n(3.0)]), n(2.0));
            let digits = string(cx, "32");
            assert_eq!(at(cx, n(12312.0), &[digits]), n(-1.0));
            let units = Value::String(
                cx.new_string_utf16(&[0x61, 0xD800, 0xDC00, 0xD800])
                    .unwrap(),
            );
            let lone = Value::String(cx.new_string_utf16(&[0xD800]).unwrap());
            assert_eq!(at(cx, units, &[lone]), n(1.0));
            assert_eq!(at(cx, units, &[lone, n(2.0)]), n(3.0));
            for this in [Value::Undefined, Value::Null] {
                let error = thrown(index_of(cx, prototype, this, &[c]));
                assert!(is_error_of(cx, global, error, "TypeError"), "{this:?}");
            }
            let method = get_object(cx, prototype, "indexOf");
            assert_eq!(get(cx, method, "length"), Value::Number(1.0));
        })
        .unwrap();
}
