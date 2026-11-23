//! The String builtins the harness files call (ECMA-262 22.1, `#sec-string-objects`): the
//! `String` constructor (`#sec-string-constructor-string-value`) and
//! `String.prototype.indexOf` (`#sec-string.prototype.indexof`) with StringIndexOf
//! (`#sec-stringindexof`). No script runs. A Symbol argument needs a well-known symbol, which
//! the host interface does not hand out, so that case is a unit test of
//! `builtins/string/mod.rs`. A guard holds the step totals of the builtins that read a String
//! and copy it (`story:string-builtins-charge-before-they-copy`), each called by the host.
#![cfg(test)]

use crate::support;

use support::host::{
    TestHost, agent, data, get, get_object, global, is_error_of, key, object, realm, string, text,
    thrown,
};
use swell::{Completion, Cx, HostCall, ObjectRef, PropertyKey, Value};

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

/// A valueOf that answers 7, to show that ToPrimitive of a String object never reaches an
/// object's own valueOf through `String.prototype.toString`.
fn seven(_cx: &mut Cx<'_, TestHost>, _call: HostCall<'_>) -> Completion<Value> {
    Ok(Value::Number(7.0))
}

/// `#sec-string.prototype.tostring` and `#sec-string.prototype.valueof` with
/// `#sec-thisstringvalue`: a String this value is answered as it is, a String object answers
/// its [[StringData]]; anything else, a Number or Boolean object and an object that only
/// inherits from `String.prototype` included, throws a TypeError. Both have length 0 and their
/// names, and are { [[Writable]]: true, [[Enumerable]]: false, [[Configurable]]: true }. With
/// them, ToPrimitive of a String object finds its String (`#sec-ordinarytoprimitive`): a String
/// object multiplied by "1" is the Number of its String.
#[test]
fn string_prototype_to_string_and_value_of_need_a_string() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let global_root = global(&mut agent, &realm);
    agent
        .with_cx(&realm, |cx| {
            let global = object(cx, &global_root);
            let string_constructor = get_object(cx, global, "String");
            let prototype = get_object(cx, string_constructor, "prototype");
            let object_constructor = get_object(cx, global, "Object");
            let get_own = get(cx, object_constructor, "getOwnPropertyDescriptor");
            let one = string(cx, "1");
            let wrapped = cx
                .construct(Value::Object(string_constructor), &[one], None)
                .unwrap()
                .unwrap();
            let Value::Object(wrapped_object) = wrapped else {
                panic!("{wrapped:?}");
            };
            // An ordinary object whose [[Prototype]] is %String.prototype% (`new Object` with a
            // new target whose "prototype" it is) has no [[StringData]].
            let realm_ref = cx.realm_ref(&realm).unwrap();
            let new_target = cx
                .new_host_function(realm_ref, "N", 0, unused, true, None)
                .unwrap()
                .unwrap();
            let prototype_key = key(cx, "prototype");
            cx.set(new_target, prototype_key, Value::Object(prototype), true)
                .unwrap()
                .unwrap();
            let inheriting = cx
                .construct(
                    Value::Object(object_constructor),
                    &[],
                    Some(Value::Object(new_target)),
                )
                .unwrap()
                .unwrap();
            let Value::Object(inheriting) = inheriting else {
                panic!("{inheriting:?}");
            };
            let get_prototype_of = get(cx, object_constructor, "getPrototypeOf");
            assert_eq!(
                cx.call(
                    get_prototype_of,
                    Value::Undefined,
                    &[Value::Object(inheriting)]
                )
                .unwrap()
                .unwrap(),
                Value::Object(prototype)
            );
            let number_object = cx.to_object(Value::Number(1.0)).unwrap().unwrap();
            let boolean_object = cx.to_object(Value::Boolean(true)).unwrap().unwrap();
            for name in ["toString", "valueOf"] {
                let method = get(cx, prototype, name);
                let Value::Object(method_object) = method else {
                    panic!("String.prototype.{name} is {method:?}");
                };
                let answer = cx.call(method, one, &[]).unwrap().unwrap();
                assert_eq!(answer, one, "{name} of a String");
                let answer = cx.call(method, wrapped, &[]).unwrap().unwrap();
                assert_eq!(text(cx, answer), "1", "{name} of a String object");
                let answer = cx
                    .call(method, Value::Object(prototype), &[])
                    .unwrap()
                    .unwrap();
                assert_eq!(text(cx, answer), "", "{name} of String.prototype");
                for this in [
                    Value::Undefined,
                    Value::Null,
                    Value::Number(1.0),
                    Value::Boolean(false),
                    Value::Object(number_object),
                    Value::Object(boolean_object),
                    Value::Object(inheriting),
                    Value::Object(string_constructor),
                ] {
                    let error = thrown(cx.call(method, this, &[]).unwrap());
                    assert!(
                        is_error_of(cx, global, error, "TypeError"),
                        "{name} of {this:?}"
                    );
                }
                assert_eq!(get(cx, method_object, "length"), Value::Number(0.0));
                let found = get(cx, method_object, "name");
                assert_eq!(text(cx, found), name);
                let name_value = string(cx, name);
                let descriptor = cx
                    .call(
                        get_own,
                        Value::Undefined,
                        &[Value::Object(prototype), name_value],
                    )
                    .unwrap()
                    .unwrap();
                let Value::Object(descriptor) = descriptor else {
                    panic!("{descriptor:?}");
                };
                assert_eq!(get(cx, descriptor, "writable"), Value::Boolean(true));
                assert_eq!(get(cx, descriptor, "enumerable"), Value::Boolean(false));
                assert_eq!(get(cx, descriptor, "configurable"), Value::Boolean(true));
            }

            // ToPrimitive: valueOf of %String.prototype% answers the [[StringData]], so
            // ToNumber of new String("1") is 1, and ToString of it "1".
            assert_eq!(cx.to_number(wrapped).unwrap(), Ok(1.0));
            let as_string = cx.to_string(wrapped).unwrap().unwrap();
            assert_eq!(text(cx, Value::String(as_string)), "1");
            // An own valueOf of the object comes first with the hint Number.
            let realm_ref = cx.realm_ref(&realm).unwrap();
            let own = cx
                .new_host_function(realm_ref, "valueOf", 0, seven, false, None)
                .unwrap()
                .unwrap();
            let value_of_key = key(cx, "valueOf");
            cx.set(wrapped_object, value_of_key, Value::Object(own), true)
                .unwrap()
                .unwrap();
            assert_eq!(cx.to_number(wrapped).unwrap(), Ok(7.0));
            let as_string = cx.to_string(wrapped).unwrap().unwrap();
            assert_eq!(text(cx, Value::String(as_string)), "1");
        })
        .unwrap();
}

/// The steps of one case of [`the_step_totals_are_unchanged`]: what it needs is made first,
/// each an entry of its own, and its last entry, the one measured, is the call of the builtin.
/// It is given the global object and the String the case runs on.
type Step = fn(&mut Cx<'_, TestHost>, ObjectRef, Value) -> Completion<Value>;

/// The method at `path` from the global object, called with `this` and `args`.
fn call_path(
    cx: &mut Cx<'_, TestHost>,
    global: ObjectRef,
    path: &[&str],
    this: Value,
    args: &[Value],
) -> Completion<Value> {
    let Some((last, holders)) = path.split_last() else {
        panic!("an empty path");
    };
    let mut holder = global;
    for name in holders {
        holder = get_object(cx, holder, name);
    }
    let method = get(cx, holder, last);
    cx.call(method, this, args).unwrap()
}

/// `<constructor>.prototype.<method>` called with `this` and `args`.
fn call_proto(
    cx: &mut Cx<'_, TestHost>,
    global: ObjectRef,
    constructor: &str,
    method: &str,
    this: Value,
    args: &[Value],
) -> Completion<Value> {
    call_path(cx, global, &[constructor, "prototype", method], this, args)
}

/// The length of the String `value` plus `more`, as a Number.
fn length_plus(cx: &Cx<'_, TestHost>, value: Value, more: usize) -> Value {
    let Value::String(string) = value else {
        panic!("{value:?} is not a String");
    };
    Value::Number((cx.to_utf16(string).unwrap().len() + more) as f64)
}

/// A new value a case needs, which must be made normally.
fn made(completion: Completion<Value>) -> Value {
    completion.unwrap()
}

/// The object a case needs, which must be made normally.
fn made_object(completion: Completion<Value>) -> ObjectRef {
    match completion.unwrap() {
        Value::Object(object) => object,
        other => panic!("{other:?} is no object"),
    }
}

/// A host function that answers its second argument: a reviver that keeps every value.
fn keep_value(_cx: &mut Cx<'_, TestHost>, call: HostCall<'_>) -> Completion<Value> {
    Ok(call.args.get(1).copied().unwrap_or(Value::Undefined))
}

/// The cases that run on each String: every method of points 3 to 5 of
/// `story:string-builtins-charge-before-they-copy`, and the users of the builder's append of a
/// String (`concat`, `String.raw`, `join`, `toLocaleString`, the name of `bind`).
fn string_cases() -> Vec<(&'static str, Step)> {
    vec![
        ("trim", |cx, g, s| {
            call_proto(cx, g, "String", "trim", s, &[])
        }),
        ("trimStart", |cx, g, s| {
            call_proto(cx, g, "String", "trimStart", s, &[])
        }),
        ("trimEnd", |cx, g, s| {
            call_proto(cx, g, "String", "trimEnd", s, &[])
        }),
        ("toLowerCase", |cx, g, s| {
            call_proto(cx, g, "String", "toLowerCase", s, &[])
        }),
        ("toUpperCase", |cx, g, s| {
            call_proto(cx, g, "String", "toUpperCase", s, &[])
        }),
        ("toLocaleLowerCase", |cx, g, s| {
            call_proto(cx, g, "String", "toLocaleLowerCase", s, &[])
        }),
        ("toLocaleUpperCase", |cx, g, s| {
            call_proto(cx, g, "String", "toLocaleUpperCase", s, &[])
        }),
        ("localeCompare", |cx, g, s| {
            let other = string(cx, "Ab");
            call_proto(cx, g, "String", "localeCompare", s, &[other])
        }),
        ("localeCompare this", |cx, g, s| {
            let other = string(cx, "Ab");
            call_proto(cx, g, "String", "localeCompare", other, &[s])
        }),
        ("Error name", |cx, g, s| {
            let constructor = get(cx, g, "Error");
            let m = string(cx, "m");
            let error = made_object(cx.construct(constructor, &[m], None).unwrap());
            let name = key(cx, "name");
            cx.set(error, name, s, true).unwrap().unwrap();
            call_proto(cx, g, "Error", "toString", Value::Object(error), &[])
        }),
        ("Error message", |cx, g, s| {
            let constructor = get(cx, g, "Error");
            let error = made(cx.construct(constructor, &[s], None).unwrap());
            call_proto(cx, g, "Error", "toString", error, &[])
        }),
        ("concat", |cx, g, s| {
            let a = string(cx, "a");
            call_proto(cx, g, "String", "concat", a, &[s, s])
        }),
        ("String.raw", |cx, g, s| {
            let x = string(cx, "x");
            let constructor = get(cx, g, "Array");
            let raw = made(cx.call(constructor, Value::Undefined, &[s, x, s]).unwrap());
            let constructor = get(cx, g, "Object");
            let template = made_object(cx.call(constructor, Value::Undefined, &[]).unwrap());
            let raw_key = key(cx, "raw");
            cx.set(template, raw_key, raw, true).unwrap().unwrap();
            call_path(
                cx,
                g,
                &["String", "raw"],
                Value::Undefined,
                &[Value::Object(template), s, s],
            )
        }),
        ("join", |cx, g, s| {
            let constructor = get(cx, g, "Array");
            let array = made(cx.call(constructor, Value::Undefined, &[s, s]).unwrap());
            call_proto(cx, g, "Array", "join", array, &[s])
        }),
        ("join of one", |cx, g, s| {
            let one = string(cx, "1");
            let constructor = get(cx, g, "Array");
            let array = made(cx.call(constructor, Value::Undefined, &[one]).unwrap());
            call_proto(cx, g, "Array", "join", array, &[s])
        }),
        ("join default", |cx, g, s| {
            let constructor = get(cx, g, "Array");
            let array = made(cx.call(constructor, Value::Undefined, &[s]).unwrap());
            call_proto(cx, g, "Array", "join", array, &[])
        }),
        ("toLocaleString", |cx, g, s| {
            let constructor = get(cx, g, "Array");
            let array = made(cx.call(constructor, Value::Undefined, &[s, s]).unwrap());
            call_proto(cx, g, "Array", "toLocaleString", array, &[])
        }),
        ("bind name", |cx, g, s| {
            let constructor = get(cx, g, "Function");
            let function = made_object(cx.call(constructor, Value::Undefined, &[]).unwrap());
            let name = key(cx, "name");
            cx.define_property_or_throw(function, name, data(s, false, false, true))
                .unwrap()
                .unwrap();
            call_proto(cx, g, "Function", "bind", Value::Object(function), &[])
        }),
        ("repeat 0", |cx, g, s| {
            call_proto(cx, g, "String", "repeat", s, &[Value::Number(0.0)])
        }),
        ("repeat 1", |cx, g, s| {
            call_proto(cx, g, "String", "repeat", s, &[Value::Number(1.0)])
        }),
        ("repeat 3", |cx, g, s| {
            call_proto(cx, g, "String", "repeat", s, &[Value::Number(3.0)])
        }),
        ("padStart", |cx, g, s| {
            let length = length_plus(cx, s, 5);
            call_proto(cx, g, "String", "padStart", s, &[length])
        }),
        ("padEnd", |cx, g, s| {
            let length = length_plus(cx, s, 5);
            let fill = string(cx, "xy");
            call_proto(cx, g, "String", "padEnd", s, &[length, fill])
        }),
        ("padEnd to its length", |cx, g, s| {
            let length = length_plus(cx, s, 0);
            call_proto(cx, g, "String", "padEnd", s, &[length])
        }),
        ("padStart fill", |cx, g, s| {
            let x = string(cx, "x");
            call_proto(cx, g, "String", "padStart", x, &[Value::Number(40.0), s])
        }),
        ("padEnd fill", |cx, g, s| {
            let x = string(cx, "x");
            call_proto(cx, g, "String", "padEnd", x, &[Value::Number(40.0), s])
        }),
        ("JSON.stringify", |cx, g, s| {
            call_path(cx, g, &["JSON", "stringify"], Value::Undefined, &[s])
        }),
        ("JSON.stringify key", |cx, g, s| {
            let constructor = get(cx, g, "Object");
            let object = made_object(cx.call(constructor, Value::Undefined, &[]).unwrap());
            let Value::String(name) = s else {
                panic!("{s:?}");
            };
            cx.set(object, PropertyKey::String(name), s, true)
                .unwrap()
                .unwrap();
            call_path(
                cx,
                g,
                &["JSON", "stringify"],
                Value::Undefined,
                &[Value::Object(object)],
            )
        }),
        ("JSON.stringify gap", |cx, g, s| {
            let constructor = get(cx, g, "Object");
            let object = made_object(cx.call(constructor, Value::Undefined, &[]).unwrap());
            let a = key(cx, "a");
            cx.set(object, a, Value::Number(1.0), true)
                .unwrap()
                .unwrap();
            let constructor = get(cx, g, "Array");
            let array = made(
                cx.call(
                    constructor,
                    Value::Undefined,
                    &[Value::Number(1.0), Value::Object(object)],
                )
                .unwrap(),
            );
            call_path(
                cx,
                g,
                &["JSON", "stringify"],
                Value::Undefined,
                &[array, Value::Null, s],
            )
        }),
        ("JSON.stringify raw", |cx, g, s| {
            let quoted = made(call_path(
                cx,
                g,
                &["JSON", "stringify"],
                Value::Undefined,
                &[s],
            ));
            let raw = made(call_path(
                cx,
                g,
                &["JSON", "rawJSON"],
                Value::Undefined,
                &[quoted],
            ));
            call_path(cx, g, &["JSON", "stringify"], Value::Undefined, &[raw])
        }),
    ]
}

/// The code units of a String of `length` for the cases: a space at each end (white space
/// for the trims), and between them letters of both cases, digits, a quotation mark, a reverse
/// solidus, a line feed and U+0001 (escapes of QuoteJSONString), all below 0x80.
fn ascii_units(length: usize) -> Vec<u16> {
    const MIDDLE: &[u8] = b"Ab\"\\\nz9\x01 _Q";
    (0..length)
        .map(|index| {
            if index == 0 || index + 1 == length {
                0x20
            } else {
                u16::from(MIDDLE[index % MIDDLE.len()])
            }
        })
        .collect()
}

/// A String of 1,000 code units with a surrogate pair at 255 and 256, where a range of 256
/// units of QuoteJSONString would end between its units, a lone leading surrogate at 511, a
/// lone trailing one at 767 and U+00E9 between: `localeCompare` refuses it before its story of
/// slice 5.
fn other_units() -> Vec<u16> {
    let mut units = ascii_units(1_000);
    units[255] = 0xD83D;
    units[256] = 0xDE00;
    units[511] = 0xD800;
    units[767] = 0xDC00;
    units[600] = 0xE9;
    units
}

/// A JSON text of exactly `length` code units (at least 1): one number, or an array of nested
/// arrays and objects, Strings with escapes, numbers, literals and white space, padded with
/// spaces before its closing bracket.
fn json_text(length: usize) -> Vec<u16> {
    if length == 1 {
        return vec![u16::from(b'7')];
    }
    const PIECES: &[&str] = &[
        r#"{"k\nA": [1.5e3, -2, true, null, "a\"b\\c"], "x": {}}"#,
        r#""0123456789abcdef0123456789abcdef0123456789abcdef0123456789\t😀""#,
        r#"[[[]], -1234567.875E-2, false]"#,
        r#""a long String of text with no escape in it at all, read unit by unit""#,
    ];
    let mut text = String::from("[");
    let mut index = 0;
    loop {
        let piece = PIECES[index % PIECES.len()];
        let separator = if index == 0 { "" } else { ",\n " };
        if text.len() + separator.len() + piece.len() + 1 > length {
            break;
        }
        text.push_str(separator);
        text.push_str(piece);
        index += 1;
    }
    while text.len() + 1 < length {
        text.push(' ');
    }
    text.push(']');
    assert_eq!(text.len(), length);
    text.encode_utf16().collect()
}

/// FNV-1a of `units`, with their count: a String the test compares without writing it out.
fn digest(units: &[u16]) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &unit in units {
        hash ^= u64::from(unit);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{}:{hash:016x}", units.len())
}

/// What a case answered, as text: a String by [`digest`], an object by the digest of
/// `JSON.stringify` of it or else its `name`, a thrown value by its `name`, a termination by
/// its debug form. Read after the steps, in an entry of its own.
fn answer_text(
    agent: &mut swell::Agent<TestHost>,
    realm: &swell::Realm,
    global_root: &swell::Root,
    answer: Result<swell::Root, Result<swell::Root, swell::Termination>>,
) -> String {
    let (root, thrown) = match answer {
        Ok(root) => (root, false),
        Err(Ok(root)) => (root, true),
        Err(Err(termination)) => return format!("{termination:?}"),
    };
    agent
        .with_cx(realm, |cx| {
            let global = object(cx, global_root);
            let value = cx.get(&root).unwrap();
            if thrown {
                let Value::Object(error) = value else {
                    return format!("throw {value:?}");
                };
                let name = get(cx, error, "name");
                return format!("throw {}", text(cx, name));
            }
            match value {
                Value::String(string) => digest(&cx.to_utf16(string).unwrap()),
                Value::Object(object) => {
                    let stringify = ["JSON", "stringify"];
                    let json = call_path(cx, global, &stringify, Value::Undefined, &[value]);
                    let (label, string) = match json.unwrap() {
                        Value::String(string) => ("json", string),
                        _ => match get(cx, object, "name") {
                            Value::String(name) => ("name", name),
                            other => panic!("{other:?}"),
                        },
                    };
                    format!("{label} {}", digest(&cx.to_utf16(string).unwrap()))
                }
                other => format!("{other:?}"),
            }
        })
        .unwrap()
}

/// The answer and the steps of each case, recorded on the base of
/// `story:string-builtins-charge-before-they-copy` (`6a89c27`), before the change.
const RECORDED: &[(&str, &str, u64)] = &[
    ("trim s0", "0:cbf29ce484222325", 1),
    ("trimStart s0", "0:cbf29ce484222325", 1),
    ("trimEnd s0", "0:cbf29ce484222325", 1),
    ("toLowerCase s0", "0:cbf29ce484222325", 1),
    ("toUpperCase s0", "0:cbf29ce484222325", 1),
    ("toLocaleLowerCase s0", "0:cbf29ce484222325", 1),
    ("toLocaleUpperCase s0", "0:cbf29ce484222325", 1),
    ("localeCompare s0", "Number(-1.0)", 3),
    ("localeCompare this s0", "Number(1.0)", 3),
    ("Error name s0", "1:af63e04c8601f358", 1),
    ("Error message s0", "5:5f343a43e7ea9f91", 2),
    ("concat s0", "1:af63dc4c8601ec8c", 2),
    ("String.raw s0", "1:af63f54c86021707", 5),
    ("join s0", "0:cbf29ce484222325", 3),
    ("join of one s0", "1:af63ac4c86019afc", 3),
    ("join default s0", "0:cbf29ce484222325", 2),
    ("toLocaleString s0", "1:af63a14c8601884b", 14),
    ("bind name s0", "name 6:52f5bf4caef03491", 7),
    ("repeat 0 s0", "0:cbf29ce484222325", 1),
    ("repeat 1 s0", "0:cbf29ce484222325", 1),
    ("repeat 3 s0", "0:cbf29ce484222325", 1),
    ("padStart s0", "5:3ac7a97e17e3642f", 6),
    ("padEnd s0", "5:79c9af060422ac0f", 6),
    ("padEnd to its length s0", "0:cbf29ce484222325", 1),
    ("padStart fill s0", "1:af63f54c86021707", 1),
    ("padEnd fill s0", "1:af63f54c86021707", 1),
    ("JSON.stringify s0", "2:07cc7607b4949e25", 3),
    ("JSON.stringify key s0", "7:869317f36b8a733f", 13),
    ("JSON.stringify gap s0", "11:16d4bcce10e1767e", 22),
    ("JSON.stringify raw s0", "2:07cc7607b4949e25", 3),
    ("trim s1", "0:cbf29ce484222325", 2),
    ("trimStart s1", "0:cbf29ce484222325", 2),
    ("trimEnd s1", "0:cbf29ce484222325", 2),
    ("toLowerCase s1", "1:af639d4c8601817f", 3),
    ("toUpperCase s1", "1:af639d4c8601817f", 3),
    ("toLocaleLowerCase s1", "1:af639d4c8601817f", 3),
    ("toLocaleUpperCase s1", "1:af639d4c8601817f", 3),
    ("localeCompare s1", "Number(-1.0)", 4),
    ("localeCompare this s1", "Number(1.0)", 4),
    ("Error name s1", "4:56fd6a73ba2a8a40", 5),
    ("Error message s1", "8:879f43ab73cdecb9", 10),
    ("concat s1", "3:e63fd7190483a1ec", 4),
    ("String.raw s1", "5:54db417d046ae487", 9),
    ("join s1", "3:c3b0d217ceb2bed7", 6),
    ("join of one s1", "1:af63ac4c86019afc", 3),
    ("join default s1", "1:af639d4c8601817f", 3),
    ("toLocaleString s1", "3:c3884a17ce9085ab", 16),
    ("bind name s1", "name 7:e7c8c04d422988c3", 8),
    ("repeat 0 s1", "0:cbf29ce484222325", 1),
    ("repeat 1 s1", "1:af639d4c8601817f", 2),
    ("repeat 3 s1", "3:c3b0d217ceb2bed7", 4),
    ("padStart s1", "6:c4a910429763057d", 7),
    ("padEnd s1", "6:b0802252c9f4752d", 7),
    ("padEnd to its length s1", "1:af639d4c8601817f", 1),
    ("padStart fill s1", "40:40dce0a315b6214d", 41),
    ("padEnd fill s1", "40:67fb14fd3f73c69d", 41),
    ("JSON.stringify s1", "3:d503c617d882b8c7", 4),
    ("JSON.stringify key s1", "9:4d4c55e3adaed4fb", 15),
    ("JSON.stringify gap s1", "22:79275999ee859c70", 33),
    ("JSON.stringify raw s1", "3:d503c617d882b8c7", 4),
    ("trim s31", "29:93321ce7b55667df", 61),
    ("trimStart s31", "30:748f1cb921d2b64d", 62),
    ("trimEnd s31", "30:43a7ae0e976788ed", 62),
    ("toLowerCase s31", "31:8fe8865311922117", 63),
    ("toUpperCase s31", "31:c23a7bb7bc92b457", 63),
    ("toLocaleLowerCase s31", "31:8fe8865311922117", 63),
    ("toLocaleUpperCase s31", "31:c23a7bb7bc92b457", 63),
    ("localeCompare s31", "Number(-1.0)", 34),
    ("localeCompare this s31", "Number(1.0)", 34),
    ("Error name s31", "34:5a2aa54066202de8", 35),
    ("Error message s31", "38:688ea8267f7b1f99", 40),
    ("concat s31", "63:7181aa45390d2bd4", 64),
    ("String.raw s31", "125:fc8f1c2cc0303047", 129),
    ("join s31", "93:690b614263cc2f7f", 96),
    ("join of one s31", "1:af63ac4c86019afc", 3),
    ("join default s31", "31:5d758fcb44ed7457", 33),
    ("toLocaleString s31", "63:a376d5036601f0e3", 76),
    ("bind name s31", "name 37:0e0d376d5ed0c0f3", 38),
    ("repeat 0 s31", "0:cbf29ce484222325", 1),
    ("repeat 1 s31", "31:5d758fcb44ed7457", 32),
    ("repeat 3 s31", "93:690b614263cc2f7f", 94),
    ("padStart s31", "36:cfa3d8ea679cefc5", 37),
    ("padEnd s31", "36:3719420359d33ca5", 37),
    ("padEnd to its length s31", "31:5d758fcb44ed7457", 1),
    ("padStart fill s31", "40:e68d649d49969e37", 41),
    ("padEnd fill s31", "40:dc9d3bdbae6fe78b", 41),
    ("JSON.stringify s31", "57:303dea42d3e85874", 58),
    ("JSON.stringify key s31", "117:54759d9ab455479f", 123),
    ("JSON.stringify gap s31", "67:e8d655a577996ad5", 78),
    ("JSON.stringify raw s31", "57:303dea42d3e85874", 58),
    ("trim s32", "29:93321ce7b55667df", 62),
    ("trimStart s32", "31:e1e43b94790bfb37", 64),
    ("trimEnd s32", "30:43a7ae0e976788ed", 63),
    ("toLowerCase s32", "32:1a3d7626db4e7075", 65),
    ("toUpperCase s32", "32:9c14b0356d48a635", 65),
    ("toLocaleLowerCase s32", "32:1a3d7626db4e7075", 65),
    ("toLocaleUpperCase s32", "32:9c14b0356d48a635", 65),
    ("localeCompare s32", "Number(-1.0)", 35),
    ("localeCompare this s32", "Number(1.0)", 35),
    ("Error name s32", "35:60b9bbc0c7ddd11e", 36),
    ("Error message s32", "39:2587726a9e36e75b", 41),
    ("concat s32", "65:a6c8f015ad88622c", 66),
    ("String.raw s32", "129:563a4857b0a879c7", 133),
    ("join s32", "96:c3d59ab50539fcb5", 99),
    ("join of one s32", "1:af63ac4c86019afc", 3),
    ("join default s32", "32:bc37cd661f7ce635", 34),
    ("toLocaleString s32", "65:91119d2c9cfadc33", 78),
    ("bind name s32", "name 38:b13601d81cb7a689", 39),
    ("repeat 0 s32", "0:cbf29ce484222325", 1),
    ("repeat 1 s32", "32:bc37cd661f7ce635", 33),
    ("repeat 3 s32", "96:c3d59ab50539fcb5", 97),
    ("padStart s32", "37:70597b4e0faba21f", 38),
    ("padEnd s32", "37:ee5f63d3849f8edf", 38),
    ("padEnd to its length s32", "32:bc37cd661f7ce635", 1),
    ("padStart fill s32", "40:e8cef19dd4eaf6a8", 41),
    ("padEnd fill s32", "40:4b8717e589d679d6", 41),
    ("JSON.stringify s32", "58:e1946d8e13d44668", 59),
    ("JSON.stringify key s32", "119:de4003ee27bc0009", 125),
    ("JSON.stringify gap s32", "67:e8d655a577996ad5", 78),
    ("JSON.stringify raw s32", "58:e1946d8e13d44668", 59),
    ("trim s33", "31:e1e3e094790b6096", 65),
    ("trimStart s33", "32:e1975249ae555542", 66),
    ("trimEnd s33", "32:bc375e661f7c2998", 66),
    ("toLowerCase s33", "33:e51a1f06a67a62e8", 67),
    ("toUpperCase s33", "33:7e5485c8b13223a8", 67),
    ("toLocaleLowerCase s33", "33:e51a1f06a67a62e8", 67),
    ("toLocaleUpperCase s33", "33:7e5485c8b13223a8", 67),
    ("localeCompare s33", "Number(-1.0)", 36),
    ("localeCompare this s33", "Number(1.0)", 36),
    ("Error name s33", "36:cddcba6f0b7b235d", 37),
    ("Error message s33", "40:fc3ad42ad7707256", 42),
    ("concat s33", "67:3f82654323fe86ea", 68),
    ("String.raw s33", "133:6062138997c0be9f", 137),
    ("join s33", "99:bbcb655c8ef0c016", 102),
    ("join of one s33", "1:af63ac4c86019afc", 3),
    ("join default s33", "33:4e3f1f877ffae3a8", 35),
    ("toLocaleString s33", "67:0bbe4a9d20c3208f", 80),
    ("bind name s33", "name 39:d5e74138cb9f95ec", 40),
    ("repeat 0 s33", "0:cbf29ce484222325", 1),
    ("repeat 1 s33", "33:4e3f1f877ffae3a8", 34),
    ("repeat 3 s33", "99:bbcb655c8ef0c016", 100),
    ("padStart s33", "38:92afa2a49fcbb74a", 39),
    ("padEnd s33", "38:2b157ed2189b6598", 39),
    ("padEnd to its length s33", "33:4e3f1f877ffae3a8", 1),
    ("padStart fill s33", "40:dd886433cc976204", 41),
    ("padEnd fill s33", "40:0f1f3424c0ba9986", 41),
    ("JSON.stringify s33", "59:3ca66f6bbfb8d491", 60),
    ("JSON.stringify key s33", "121:9fb8da5056fa621b", 127),
    ("JSON.stringify gap s33", "67:e8d655a577996ad5", 78),
    ("JSON.stringify raw s33", "59:3ca66f6bbfb8d491", 60),
    ("trim s1000", "997:5df7272d016a8d97", 1998),
    ("trimStart s1000", "999:a047fb4bd127c0ef", 2000),
    ("trimEnd s1000", "998:1ef25fdf627e0625", 1999),
    ("toLowerCase s1000", "1000:b7a5dd44d4de2dad", 2001),
    ("toUpperCase s1000", "1000:e66f9bd46157802d", 2001),
    ("toLocaleLowerCase s1000", "1000:b7a5dd44d4de2dad", 2001),
    ("toLocaleUpperCase s1000", "1000:e66f9bd46157802d", 2001),
    ("localeCompare s1000", "Number(-1.0)", 1003),
    ("localeCompare this s1000", "Number(1.0)", 1003),
    ("Error name s1000", "1003:7a646bc65f7a09c6", 1004),
    ("Error message s1000", "1007:0ce473a449c23c13", 1009),
    ("concat s1000", "2001:0eedc710bd9581dc", 2002),
    ("String.raw s1000", "4001:1ca92cbb50b33287", 4005),
    ("join s1000", "3000:eacc4f03ecda353d", 3003),
    ("join of one s1000", "1:af63ac4c86019afc", 3),
    ("join default s1000", "1000:e8081218918f2f6d", 1002),
    ("toLocaleString s1000", "2001:cd05789f3d64c4e3", 2014),
    ("bind name s1000", "name 1006:8c5bb5f61e9384e1", 1007),
    ("repeat 0 s1000", "0:cbf29ce484222325", 1),
    ("repeat 1 s1000", "1000:e8081218918f2f6d", 1001),
    ("repeat 3 s1000", "3000:eacc4f03ecda353d", 3001),
    ("padStart s1000", "1005:6ba01769eb022737", 1006),
    ("padEnd s1000", "1005:bee424aa904db937", 1006),
    ("padEnd to its length s1000", "1000:e8081218918f2f6d", 1),
    ("padStart fill s1000", "40:9ff11f874502e4d2", 41),
    ("padEnd fill s1000", "40:cfcfc2de43280918", 41),
    ("JSON.stringify s1000", "1730:93564e92a8fd34f8", 1731),
    ("JSON.stringify key s1000", "3463:4978ab29b3b2c7f9", 3469),
    ("JSON.stringify gap s1000", "67:e8d655a577996ad5", 78),
    ("JSON.stringify raw s1000", "1730:93564e92a8fd34f8", 1731),
    ("trim u1000", "997:37c7f971ef012894", 1998),
    ("trimStart u1000", "999:246eb02f9faf5734", 2000),
    ("trimEnd u1000", "998:b957afce36728d0a", 1999),
    ("toLowerCase u1000", "1000:096689fc19a3a81a", 2001),
    ("toUpperCase u1000", "1000:8efc88564c07005a", 2001),
    ("toLocaleLowerCase u1000", "1000:096689fc19a3a81a", 2001),
    ("toLocaleUpperCase u1000", "1000:8efc88564c07005a", 2001),
    ("localeCompare u1000", "throw SyntaxError", 1003),
    ("localeCompare this u1000", "throw SyntaxError", 1003),
    ("Error name u1000", "1003:8c5212807280fff7", 1004),
    ("Error message u1000", "1007:a6e61755e3287b98", 1009),
    ("concat u1000", "2001:d4b02baa5398c6c0", 2002),
    ("String.raw u1000", "4001:8a7d1a5d38f496d7", 4005),
    ("join u1000", "3000:e0e5b203a72a4d4e", 3003),
    ("join of one u1000", "1:af63ac4c86019afc", 3),
    ("join default u1000", "1000:1b484b3365d9101a", 1002),
    ("toLocaleString u1000", "2001:b9527bf532ab91d9", 2014),
    ("bind name u1000", "name 1006:6a1daaadc34601fe", 1007),
    ("repeat 0 u1000", "0:cbf29ce484222325", 1),
    ("repeat 1 u1000", "1000:1b484b3365d9101a", 1001),
    ("repeat 3 u1000", "3000:e0e5b203a72a4d4e", 3001),
    ("padStart u1000", "1005:fba7ddcc121dc06c", 1006),
    ("padEnd u1000", "1005:8265a80c76c85bce", 1006),
    ("padEnd to its length u1000", "1000:1b484b3365d9101a", 1),
    ("padStart fill u1000", "40:9ff11f874502e4d2", 41),
    ("padEnd fill u1000", "40:cfcfc2de43280918", 41),
    ("JSON.stringify u1000", "1738:be0624eb7ee85580", 1739),
    ("JSON.stringify key u1000", "3479:a728641bc397cf41", 3485),
    ("JSON.stringify gap u1000", "67:e8d655a577996ad5", 78),
    ("JSON.stringify raw u1000", "1738:be0624eb7ee85580", 1739),
    ("Object.prototype.toString", "13:d2ce4e3d150a20a4", 14),
    ("JSON.parse 1", "Number(7.0)", 3),
    ("JSON.parse reviver 1", "Number(7.0)", 5),
    ("JSON.parse 4095", "json 3674:c88338e2b46f326c", 4330),
    (
        "JSON.parse reviver 4095",
        "json 3674:c88338e2b46f326c",
        4942,
    ),
    ("JSON.parse 4096", "json 3674:c88338e2b46f326c", 4331),
    (
        "JSON.parse reviver 4096",
        "json 3674:c88338e2b46f326c",
        4943,
    ),
    ("JSON.parse 4097", "json 3674:c88338e2b46f326c", 4332),
    (
        "JSON.parse reviver 4097",
        "json 3674:c88338e2b46f326c",
        4944,
    ),
    ("JSON.parse 20000", "json 18000:df4369503b519867", 21127),
    (
        "JSON.parse reviver 20000",
        "json 18000:df4369503b519867",
        24075,
    ),
    ("JSON.rawJSON String", "json 13:7b1697706f2a1519", 17),
    ("JSON.rawJSON number", "json 7:687688913ae12c66", 11),
    (
        "JSON.rawJSON long String",
        "json 1000:e3c9389840aadc2b",
        1004,
    ),
];

/// A guard of `story:string-builtins-charge-before-they-copy`: a charge moves before the copy
/// it pays for and never grows or shrinks. Each method of the story's points 3 to 5 on Strings
/// of 0, 1, 31, 32, 33 and 1,000 code units below 0x80 and on one of 1,000 with surrogates;
/// `JSON.parse` of valid texts of 1, 4,095, 4,096, 4,097 and 20,000 code units, each without and
/// with a reviver; `JSON.rawJSON` of a String literal, a number and a String literal of 1,000
/// code units: the answer and `last_entry_steps()` of each equal those recorded on the base.
/// Run also with the feature `collect-every-safe-point`.
#[test]
fn the_step_totals_are_unchanged() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let global_root = global(&mut agent, &realm);
    agent
        .with_cx(&realm, |cx| {
            let global = object(cx, &global_root);
            let realm_ref = cx.realm_ref(&realm).unwrap();
            let reviver = cx
                .new_host_function(realm_ref, "keepValue", 2, keep_value, false, None)
                .unwrap()
                .unwrap();
            let name = key(cx, "keepValue");
            cx.set(global, name, Value::Object(reviver), true)
                .unwrap()
                .unwrap();
        })
        .unwrap();
    let mut strings: Vec<(String, Vec<u16>)> = [0, 1, 31, 32, 33, 1_000]
        .into_iter()
        .map(|length| (format!("s{length}"), ascii_units(length)))
        .collect();
    strings.push(("u1000".to_string(), other_units()));
    let mut seen: Vec<(String, String, u64)> = Vec::new();
    let mut run = |agent: &mut swell::Agent<TestHost>, name: String, units: &[u16], step: Step| {
        let answer = agent
            .with_cx(&realm, |cx| {
                let global = object(cx, &global_root);
                let value = Value::String(cx.new_string_utf16(units).unwrap());
                match step(cx, global, value) {
                    Ok(value) => Ok(cx.root(value).unwrap()),
                    Err(swell::Abrupt::Throw(value)) => Err(Ok(cx.root(value).unwrap())),
                    Err(swell::Abrupt::Terminated(termination)) => Err(Err(termination)),
                }
            })
            .unwrap();
        let steps = agent.last_entry_steps().unwrap();
        let answer = answer_text(agent, &realm, &global_root, answer);
        seen.push((name, answer, steps));
    };
    for (string_name, units) in &strings {
        for (case, step) in string_cases() {
            run(&mut agent, format!("{case} {string_name}"), units, step);
        }
    }
    run(
        &mut agent,
        "Object.prototype.toString".to_string(),
        &[],
        |cx, g, _| {
            let json = get(cx, g, "JSON");
            call_proto(cx, g, "Object", "toString", json, &[])
        },
    );
    for length in [1, 4_095, 4_096, 4_097, 20_000] {
        let text = json_text(length);
        run(
            &mut agent,
            format!("JSON.parse {length}"),
            &text,
            |cx, g, t| call_path(cx, g, &["JSON", "parse"], Value::Undefined, &[t]),
        );
        run(
            &mut agent,
            format!("JSON.parse reviver {length}"),
            &text,
            |cx, g, t| {
                let reviver = get(cx, g, "keepValue");
                call_path(cx, g, &["JSON", "parse"], Value::Undefined, &[t, reviver])
            },
        );
    }
    let raw_texts = [
        (
            "JSON.rawJSON String",
            "\"ab\\nc\\u0041\"".encode_utf16().collect(),
        ),
        ("JSON.rawJSON number", "-12.5e3".encode_utf16().collect()),
        ("JSON.rawJSON long String", {
            let mut units = vec![u16::from(b'q'); 1_000];
            units[0] = u16::from(b'"');
            units[999] = u16::from(b'"');
            units
        }),
    ];
    for (name, units) in raw_texts {
        run(&mut agent, name.to_string(), &units, |cx, g, t| {
            call_path(cx, g, &["JSON", "rawJSON"], Value::Undefined, &[t])
        });
    }
    drop(run);
    if seen.as_slice() != recorded().as_slice() {
        for (name, answer, steps) in &seen {
            println!("    ({name:?}, {answer:?}, {steps}),");
        }
    }
    assert_eq!(seen, recorded());
}

/// [`RECORDED`] as owned values.
fn recorded() -> Vec<(String, String, u64)> {
    RECORDED
        .iter()
        .map(|&(name, answer, steps)| (name.to_string(), answer.to_string(), steps))
        .collect()
}
