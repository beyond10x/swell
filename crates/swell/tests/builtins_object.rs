//! The Object builtins the harness files call (ECMA-262 20.1, `#sec-object-objects`):
//! `Object.defineProperty` (`#sec-object.defineproperty`) with ToPropertyDescriptor
//! (`#sec-topropertydescriptor`), `Object.getOwnPropertyDescriptor`
//! (`#sec-object.getownpropertydescriptor`) with FromPropertyDescriptor
//! (`#sec-frompropertydescriptor`), `Object.getOwnPropertyNames`
//! (`#sec-object.getownpropertynames`), `Object.getPrototypeOf` (`#sec-object.getprototypeof`),
//! the `Object` constructor (`#sec-object-value`) and `Object.prototype.hasOwnProperty`,
//! `.propertyIsEnumerable` and `.toString` (`#sec-object.prototype.hasownproperty`,
//! `#sec-object.prototype.propertyisenumerable`, `#sec-object.prototype.tostring`). No script
//! runs: the tests call the builtins through `Cx` with host functions as callbacks.
#![cfg(test)]

use crate::support;

use support::host::{
    TestHost, agent, data, get, get_object, global, is_error_of, key, object, realm, string, text,
    thrown,
};
use swell::{Completion, Cx, ErrorKind, HostCall, ObjectRef, PropertyDescriptor, Value};

/// `object[name](args)`, the method read from `object`.
fn invoke(
    cx: &mut Cx<'_, TestHost>,
    object: ObjectRef,
    name: &str,
    args: &[Value],
) -> Completion<Value> {
    let method = get(cx, object, name);
    cx.call(method, Value::Object(object), args).unwrap()
}

/// `function.call(this, args)` of the method `name` of `holder`: the method called with `this`.
fn call_with(
    cx: &mut Cx<'_, TestHost>,
    holder: ObjectRef,
    name: &str,
    this: Value,
    args: &[Value],
) -> Completion<Value> {
    let method = get(cx, holder, name);
    cx.call(method, this, args).unwrap()
}

/// A new ordinary object, made by `Object()`.
fn new_object(cx: &mut Cx<'_, TestHost>, global: ObjectRef) -> ObjectRef {
    let constructor = get(cx, global, "Object");
    match cx
        .call(constructor, Value::Undefined, &[])
        .unwrap()
        .unwrap()
    {
        Value::Object(made) => made,
        other => panic!("Object() answered {other:?}"),
    }
}

/// A getter that records the name it was made with (its "name") in the host and answers it.
fn named_getter(cx: &mut Cx<'_, TestHost>, call: HostCall<'_>) -> Completion<Value> {
    let name = get(cx, call.callee, "name");
    let name_text = text(cx, name);
    cx.host_mut().calls.push(name_text.clone());
    match name_text.as_str() {
        "enumerable" | "configurable" | "writable" => Ok(Value::Boolean(true)),
        _ => Ok(Value::Number(7.0)),
    }
}

/// A host function that answers 1.
fn one(_cx: &mut Cx<'_, TestHost>, _call: HostCall<'_>) -> Completion<Value> {
    Ok(Value::Number(1.0))
}

/// A host function that records "toString" and answers the String "k".
fn key_to_string(cx: &mut Cx<'_, TestHost>, _call: HostCall<'_>) -> Completion<Value> {
    cx.host_mut().calls.push("toString".to_string());
    Ok(Value::String(cx.new_string("k").unwrap()))
}

/// The field `name` of a descriptor object, `None` when it has no such own property.
fn field(
    cx: &mut Cx<'_, TestHost>,
    object_constructor: ObjectRef,
    descriptor: ObjectRef,
    name: &str,
) -> Option<Value> {
    let name_value = string(cx, name);
    let own = call_with(
        cx,
        object_constructor,
        "getOwnPropertyDescriptor",
        Value::Undefined,
        &[Value::Object(descriptor), name_value],
    )
    .unwrap();
    match own {
        Value::Undefined => None,
        Value::Object(_) => Some(get(cx, descriptor, name)),
        other => panic!("{other:?}"),
    }
}

/// The names of `object`'s own String keys, as `Object.getOwnPropertyNames` lists them.
fn names(cx: &mut Cx<'_, TestHost>, object_constructor: ObjectRef, value: Value) -> Vec<String> {
    let list = call_with(
        cx,
        object_constructor,
        "getOwnPropertyNames",
        Value::Undefined,
        &[value],
    )
    .unwrap();
    let Value::Object(list) = list else {
        panic!("{list:?}");
    };
    let Value::Number(length) = get(cx, list, "length") else {
        panic!("no length");
    };
    (0..length as u32)
        .map(|index| {
            let element = get(cx, list, &index.to_string());
            text(cx, element)
        })
        .collect()
}

/// `#sec-object.defineproperty` with `#sec-topropertydescriptor` and the cases of
/// ValidateAndApplyPropertyDescriptor (`#sec-validateandapplypropertydescriptor`): a target or
/// attributes that are not objects, a getter that is not callable and a descriptor with accessor
/// and data fields throw a TypeError; the fields are read in the clause's order
/// (enumerable, configurable, value, writable, get, set), each by HasProperty then Get, through
/// accessors too; a generic descriptor makes a data property of undefined with every attribute
/// false; a data and an accessor descriptor; a non-configurable property refuses a change of
/// kind or of value and accepts the same value; an Array index past the length moves the length
/// and a length that is not a uint32 throws a RangeError; the key goes through ToPropertyKey;
/// the result is the target.
#[test]
fn define_property_every_descriptor_case() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let global_root = global(&mut agent, &realm);
    let calls = agent
        .with_cx(&realm, |cx| {
            let global = object(cx, &global_root);
            let object_constructor = get_object(cx, global, "Object");
            let realm_ref = cx.realm_ref(&realm).unwrap();
            let target = new_object(cx, global);
            let define =
                |cx: &mut Cx<'_, TestHost>, target: Value, name: Value, attributes: Value| {
                    call_with(
                        cx,
                        object_constructor,
                        "defineProperty",
                        Value::Undefined,
                        &[target, name, attributes],
                    )
                };
            let is_type_error = |cx: &mut Cx<'_, TestHost>, completion: Completion<Value>| {
                let error = thrown(completion);
                is_error_of(cx, global, error, "TypeError")
            };

            // Not an object, as target or as attributes.
            let a = string(cx, "a");
            let attributes = Value::Object(new_object(cx, global));
            let defined = define(cx, Value::Number(1.0), a, attributes);
            assert!(is_type_error(cx, defined));
            let defined = define(cx, Value::Object(target), a, Value::Number(1.0));
            assert!(is_type_error(cx, defined));
            let defined = define(cx, Value::Object(target), a, Value::Undefined);
            assert!(is_type_error(cx, defined));

            // A getter that is neither callable nor undefined; get with value; set with writable.
            let bad_getter = new_object(cx, global);
            let get_key = key(cx, "get");
            cx.set(bad_getter, get_key, Value::Number(1.0), true)
                .unwrap()
                .unwrap();
            let defined = define(cx, Value::Object(target), a, Value::Object(bad_getter));
            assert!(is_type_error(cx, defined));
            let getter = cx
                .new_host_function(realm_ref, "one", 0, one, false, None)
                .unwrap()
                .unwrap();
            let mixed = new_object(cx, global);
            cx.set(mixed, get_key, Value::Object(getter), true)
                .unwrap()
                .unwrap();
            let value_key = key(cx, "value");
            cx.set(mixed, value_key, Value::Number(2.0), true)
                .unwrap()
                .unwrap();
            let defined = define(cx, Value::Object(target), a, Value::Object(mixed));
            assert!(is_type_error(cx, defined));
            let mixed = new_object(cx, global);
            let set_key = key(cx, "set");
            cx.set(mixed, set_key, Value::Undefined, true)
                .unwrap()
                .unwrap();
            let writable_key = key(cx, "writable");
            cx.set(mixed, writable_key, Value::Boolean(false), true)
                .unwrap()
                .unwrap();
            let defined = define(cx, Value::Object(target), a, Value::Object(mixed));
            assert!(is_type_error(cx, defined));
            assert_eq!(
                names(cx, object_constructor, Value::Object(target)),
                Vec::<String>::new()
            );

            // The fields are read in the clause's order, each through its accessor, whatever
            // the order the properties were made in.
            let ordered = new_object(cx, global);
            for name in ["writable", "value", "configurable", "enumerable"] {
                let accessor = cx
                    .new_host_function(realm_ref, name, 0, named_getter, false, None)
                    .unwrap()
                    .unwrap();
                let name_key = key(cx, name);
                cx.define_property_or_throw(
                    ordered,
                    name_key,
                    PropertyDescriptor {
                        get: Some(Value::Object(accessor)),
                        enumerable: Some(true),
                        configurable: Some(true),
                        ..PropertyDescriptor::default()
                    },
                )
                .unwrap()
                .unwrap();
            }
            let b = string(cx, "b");
            let answer = define(cx, Value::Object(target), b, Value::Object(ordered)).unwrap();
            assert_eq!(
                answer,
                Value::Object(target),
                "defineProperty answers its target"
            );
            let read = get(cx, target, "b");
            assert_eq!(read, Value::Number(7.0));
            let descriptor = call_with(
                cx,
                object_constructor,
                "getOwnPropertyDescriptor",
                Value::Undefined,
                &[Value::Object(target), b],
            )
            .unwrap();
            let Value::Object(descriptor) = descriptor else {
                panic!("{descriptor:?}");
            };
            for name in ["writable", "enumerable", "configurable"] {
                assert_eq!(get(cx, descriptor, name), Value::Boolean(true), "{name}");
            }

            // A generic descriptor makes a data property of undefined, every attribute false.
            let c = string(cx, "c");
            let empty = Value::Object(new_object(cx, global));
            define(cx, Value::Object(target), c, empty).unwrap();
            let descriptor = call_with(
                cx,
                object_constructor,
                "getOwnPropertyDescriptor",
                Value::Undefined,
                &[Value::Object(target), c],
            )
            .unwrap();
            let Value::Object(descriptor) = descriptor else {
                panic!("{descriptor:?}");
            };
            assert_eq!(
                field(cx, object_constructor, descriptor, "value"),
                Some(Value::Undefined)
            );
            for name in ["writable", "enumerable", "configurable"] {
                assert_eq!(
                    field(cx, object_constructor, descriptor, name),
                    Some(Value::Boolean(false)),
                    "{name}"
                );
            }

            // An accessor property: Get calls its getter.
            let accessor = new_object(cx, global);
            cx.set(accessor, get_key, Value::Object(getter), true)
                .unwrap()
                .unwrap();
            let d = string(cx, "d");
            define(cx, Value::Object(target), d, Value::Object(accessor)).unwrap();
            assert_eq!(get(cx, target, "d"), Value::Number(1.0));

            // Non-configurable "c": a change of value or of kind is refused, the same value is
            // accepted, and an accessor descriptor on it throws.
            let same = new_object(cx, global);
            cx.set(same, value_key, Value::Undefined, true)
                .unwrap()
                .unwrap();
            define(cx, Value::Object(target), c, Value::Object(same)).unwrap();
            let other = new_object(cx, global);
            cx.set(other, value_key, Value::Number(3.0), true)
                .unwrap()
                .unwrap();
            let defined = define(cx, Value::Object(target), c, Value::Object(other));
            assert!(is_type_error(cx, defined));
            let defined = define(cx, Value::Object(target), c, Value::Object(accessor));
            assert!(is_type_error(cx, defined));
            assert_eq!(get(cx, target, "c"), Value::Undefined);

            // The key goes through ToPropertyKey: the Number 5 names "5".
            let five = new_object(cx, global);
            cx.set(five, value_key, Value::Number(5.0), true)
                .unwrap()
                .unwrap();
            define(
                cx,
                Value::Object(target),
                Value::Number(5.0),
                Value::Object(five),
            )
            .unwrap();
            assert_eq!(get(cx, target, "5"), Value::Number(5.0));

            // An Array: an index past the length moves it; a length that is not a uint32 is a
            // RangeError.
            let array_constructor = get(cx, global, "Array");
            let Value::Object(array) = cx
                .call(array_constructor, Value::Undefined, &[])
                .unwrap()
                .unwrap()
            else {
                panic!("Array() is not an object");
            };
            define(
                cx,
                Value::Object(array),
                Value::Number(3.0),
                Value::Object(five),
            )
            .unwrap();
            assert_eq!(get(cx, array, "length"), Value::Number(4.0));
            let bad_length = new_object(cx, global);
            cx.set(bad_length, value_key, Value::Number(1.5), true)
                .unwrap()
                .unwrap();
            let length = string(cx, "length");
            let error = thrown(define(
                cx,
                Value::Object(array),
                length,
                Value::Object(bad_length),
            ));
            assert!(is_error_of(cx, global, error, "RangeError"));

            // Object.defineProperty is a builtin of length 3 named "defineProperty".
            let define_property = get_object(cx, object_constructor, "defineProperty");
            assert_eq!(get(cx, define_property, "length"), Value::Number(3.0));
            let name = get(cx, define_property, "name");
            assert_eq!(text(cx, name), "defineProperty");
            cx.host().calls.clone()
        })
        .unwrap();
    assert_eq!(calls, ["enumerable", "configurable", "value", "writable"]);
}

/// `#sec-object.getownpropertydescriptor` with `#sec-frompropertydescriptor`: a data property
/// answers "value", "writable", "enumerable", "configurable" in that order, an accessor "get",
/// "set", "enumerable", "configurable"; an absent property answers undefined; a primitive goes
/// through ToObject (a String's index and "length"); undefined throws a TypeError; a descriptor
/// read back and defined on another object makes the same property. With
/// `Object.getPrototypeOf` (`#sec-object.getprototypeof`) and the `Object` constructor
/// (`#sec-object-value`).
#[test]
fn get_own_property_descriptor_round_trips() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let global_root = global(&mut agent, &realm);
    agent
        .with_cx(&realm, |cx| {
            let global = object(cx, &global_root);
            let object_constructor = get_object(cx, global, "Object");
            let realm_ref = cx.realm_ref(&realm).unwrap();
            let describe = |cx: &mut Cx<'_, TestHost>, target: Value, name: Value| {
                call_with(
                    cx,
                    object_constructor,
                    "getOwnPropertyDescriptor",
                    Value::Undefined,
                    &[target, name],
                )
            };
            let source = new_object(cx, global);
            let x = key(cx, "x");
            cx.define_property_or_throw(source, x, data(Value::Number(1.0), false, true, false))
                .unwrap()
                .unwrap();
            let getter = cx
                .new_host_function(realm_ref, "one", 0, one, false, None)
                .unwrap()
                .unwrap();
            let y = key(cx, "y");
            cx.define_property_or_throw(
                source,
                y,
                PropertyDescriptor {
                    get: Some(Value::Object(getter)),
                    set: Some(Value::Undefined),
                    enumerable: Some(false),
                    configurable: Some(true),
                    ..PropertyDescriptor::default()
                },
            )
            .unwrap()
            .unwrap();

            let x_name = string(cx, "x");
            let Value::Object(x_descriptor) = describe(cx, Value::Object(source), x_name).unwrap()
            else {
                panic!("no descriptor of x");
            };
            assert_eq!(
                names(cx, object_constructor, Value::Object(x_descriptor)),
                ["value", "writable", "enumerable", "configurable"]
            );
            assert_eq!(get(cx, x_descriptor, "value"), Value::Number(1.0));
            assert_eq!(get(cx, x_descriptor, "writable"), Value::Boolean(false));
            assert_eq!(get(cx, x_descriptor, "enumerable"), Value::Boolean(true));
            assert_eq!(get(cx, x_descriptor, "configurable"), Value::Boolean(false));
            let object_prototype = get(cx, object_constructor, "prototype");
            let prototype = call_with(
                cx,
                object_constructor,
                "getPrototypeOf",
                Value::Undefined,
                &[Value::Object(x_descriptor)],
            )
            .unwrap();
            assert_eq!(prototype, object_prototype);

            let y_name = string(cx, "y");
            let Value::Object(y_descriptor) = describe(cx, Value::Object(source), y_name).unwrap()
            else {
                panic!("no descriptor of y");
            };
            assert_eq!(
                names(cx, object_constructor, Value::Object(y_descriptor)),
                ["get", "set", "enumerable", "configurable"]
            );
            assert_eq!(get(cx, y_descriptor, "get"), Value::Object(getter));
            assert_eq!(get(cx, y_descriptor, "set"), Value::Undefined);

            let absent = string(cx, "absent");
            assert_eq!(
                describe(cx, Value::Object(source), absent).unwrap(),
                Value::Undefined
            );

            // A primitive goes through ToObject.
            let ab = string(cx, "ab");
            let Value::Object(unit) = describe(cx, ab, Value::Number(1.0)).unwrap() else {
                panic!("no descriptor of \"ab\"[1]");
            };
            let unit_value = get(cx, unit, "value");
            assert_eq!(text(cx, unit_value), "b");
            assert_eq!(get(cx, unit, "writable"), Value::Boolean(false));
            assert_eq!(get(cx, unit, "enumerable"), Value::Boolean(true));
            assert_eq!(get(cx, unit, "configurable"), Value::Boolean(false));
            let length = string(cx, "length");
            let Value::Object(length_descriptor) = describe(cx, ab, length).unwrap() else {
                panic!("no descriptor of \"ab\".length");
            };
            assert_eq!(get(cx, length_descriptor, "value"), Value::Number(2.0));
            let error = thrown(describe(cx, Value::Undefined, length));
            assert!(is_error_of(cx, global, error, "TypeError"));
            let error = thrown(call_with(
                cx,
                object_constructor,
                "getPrototypeOf",
                Value::Undefined,
                &[Value::Null],
            ));
            assert!(is_error_of(cx, global, error, "TypeError"));

            // Round trip: each descriptor defines the same property on another object.
            let copy = new_object(cx, global);
            for (name, descriptor) in [("x", x_descriptor), ("y", y_descriptor)] {
                let name_value = string(cx, name);
                call_with(
                    cx,
                    object_constructor,
                    "defineProperty",
                    Value::Undefined,
                    &[Value::Object(copy), name_value, Value::Object(descriptor)],
                )
                .unwrap();
                let Value::Object(again) = describe(cx, Value::Object(copy), name_value).unwrap()
                else {
                    panic!("no descriptor of the copy's {name}");
                };
                let fields = names(cx, object_constructor, Value::Object(descriptor));
                assert_eq!(names(cx, object_constructor, Value::Object(again)), fields);
                for field_name in fields {
                    assert_eq!(
                        get(cx, again, &field_name),
                        get(cx, descriptor, &field_name),
                        "{name}.{field_name}"
                    );
                }
            }

            // Object(value): undefined and null make a new object, an object is itself, a
            // primitive is wrapped; new Object() with another new target takes its prototype.
            let made = cx
                .call(
                    Value::Object(object_constructor),
                    Value::Undefined,
                    &[Value::Null],
                )
                .unwrap()
                .unwrap();
            let Value::Object(made) = made else {
                panic!("{made:?}");
            };
            let prototype = call_with(
                cx,
                object_constructor,
                "getPrototypeOf",
                Value::Undefined,
                &[Value::Object(made)],
            )
            .unwrap();
            assert_eq!(prototype, object_prototype);
            let itself = cx
                .call(
                    Value::Object(object_constructor),
                    Value::Undefined,
                    &[Value::Object(source)],
                )
                .unwrap()
                .unwrap();
            assert_eq!(itself, Value::Object(source));
            let wrapped = cx
                .call(
                    Value::Object(object_constructor),
                    Value::Undefined,
                    &[Value::Boolean(true)],
                )
                .unwrap()
                .unwrap();
            let to_string = get_object(cx, object_constructor, "prototype");
            let tag = call_with(cx, to_string, "toString", wrapped, &[]).unwrap();
            assert_eq!(text(cx, tag), "[object Boolean]");
            let array_constructor = get(cx, global, "Array");
            let constructed = cx
                .construct(
                    Value::Object(object_constructor),
                    &[Value::Number(1.0)],
                    Some(array_constructor),
                )
                .unwrap()
                .unwrap();
            let Value::Object(constructed) = constructed else {
                panic!("{constructed:?}");
            };
            let Value::Object(array_constructor) = array_constructor else {
                unreachable!()
            };
            let array_prototype = get(cx, array_constructor, "prototype");
            let prototype = call_with(
                cx,
                object_constructor,
                "getPrototypeOf",
                Value::Undefined,
                &[Value::Object(constructed)],
            )
            .unwrap();
            assert_eq!(prototype, array_prototype);
            assert_eq!(get(cx, object_constructor, "length"), Value::Number(1.0));
        })
        .unwrap();
}

/// `#sec-object.getownpropertynames` with GetOwnPropertyKeys (`#sec-getownpropertykeys`) and
/// OrdinaryOwnPropertyKeys (`#sec-ordinaryownpropertykeys`): the array indices ascending, then
/// the Strings in creation order, non-enumerable ones too; a String's indices then "length"; an
/// Array's indices then "length"; a Number has none; undefined throws a TypeError. The answer
/// is an Array (`Array.isArray`, `#sec-array.isarray`).
#[test]
fn own_property_names_order() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let global_root = global(&mut agent, &realm);
    agent
        .with_cx(&realm, |cx| {
            let global = object(cx, &global_root);
            let object_constructor = get_object(cx, global, "Object");
            let target = new_object(cx, global);
            for name in ["b", "2", "a", "0", "1"] {
                let name_key = key(cx, name);
                cx.define_property_or_throw(
                    target,
                    name_key,
                    data(Value::Null, true, name != "a", true),
                )
                .unwrap()
                .unwrap();
            }
            assert_eq!(
                names(cx, object_constructor, Value::Object(target)),
                ["0", "1", "2", "b", "a"]
            );
            let xy = string(cx, "xy");
            assert_eq!(names(cx, object_constructor, xy), ["0", "1", "length"]);
            assert_eq!(
                names(cx, object_constructor, Value::Number(1.0)),
                Vec::<String>::new()
            );
            let array_constructor = get_object(cx, global, "Array");
            let array = cx
                .call(
                    Value::Object(array_constructor),
                    Value::Undefined,
                    &[Value::Number(1.0), Value::Number(2.0)],
                )
                .unwrap()
                .unwrap();
            assert_eq!(names(cx, object_constructor, array), ["0", "1", "length"]);
            let error = thrown(call_with(
                cx,
                object_constructor,
                "getOwnPropertyNames",
                Value::Undefined,
                &[Value::Undefined],
            ));
            assert!(is_error_of(cx, global, error, "TypeError"));
            let list = call_with(
                cx,
                object_constructor,
                "getOwnPropertyNames",
                Value::Undefined,
                &[Value::Object(target)],
            )
            .unwrap();
            assert_eq!(
                call_with(cx, array_constructor, "isArray", Value::Undefined, &[list]).unwrap(),
                Value::Boolean(true)
            );
            assert_eq!(
                call_with(
                    cx,
                    array_constructor,
                    "isArray",
                    Value::Undefined,
                    &[Value::Object(target)]
                )
                .unwrap(),
                Value::Boolean(false)
            );
        })
        .unwrap();
}

/// `#sec-object.prototype.tostring`: "[object Undefined]" and "[object Null]", and the builtin
/// tag of each kind this slice has (Array, Function for a host function and a bound function,
/// Error, Boolean, Number, String, Object), and `Math` and `JSON`, whose @@toStringTag
/// (`#sec-math-%symbol.tostringtag%`, `#sec-json-%symbol.tostringtag%`) gives their tag.
#[test]
fn to_string_tags_of_each_kind() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let global_root = global(&mut agent, &realm);
    agent
        .with_cx(&realm, |cx| {
            let global = object(cx, &global_root);
            let realm_ref = cx.realm_ref(&realm).unwrap();
            let object_constructor = get_object(cx, global, "Object");
            let object_prototype = get_object(cx, object_constructor, "prototype");
            let array_constructor = get(cx, global, "Array");
            let array = cx
                .call(array_constructor, Value::Undefined, &[])
                .unwrap()
                .unwrap();
            let function = cx
                .new_host_function(realm_ref, "one", 0, one, false, None)
                .unwrap()
                .unwrap();
            let bind = {
                let function_prototype = cx.to_object(Value::Object(function)).unwrap().unwrap();
                get(cx, function_prototype, "bind")
            };
            let bound = cx
                .call(bind, Value::Object(function), &[])
                .unwrap()
                .unwrap();
            let error = cx
                .new_error(realm_ref, ErrorKind::TypeError, "e")
                .unwrap()
                .unwrap();
            let boolean = cx.to_object(Value::Boolean(false)).unwrap().unwrap();
            let number = cx.to_object(Value::Number(2.0)).unwrap().unwrap();
            let ab = string(cx, "ab");
            let plain = new_object(cx, global);
            let math = get(cx, global, "Math");
            let json = get(cx, global, "JSON");
            let cases = [
                (Value::Undefined, "[object Undefined]"),
                (Value::Null, "[object Null]"),
                (array, "[object Array]"),
                (Value::Object(function), "[object Function]"),
                (bound, "[object Function]"),
                (Value::Object(error), "[object Error]"),
                (Value::Object(boolean), "[object Boolean]"),
                (Value::Boolean(true), "[object Boolean]"),
                (Value::Object(number), "[object Number]"),
                (Value::Number(1.0), "[object Number]"),
                (ab, "[object String]"),
                (Value::Object(plain), "[object Object]"),
                (Value::Object(global), "[object Object]"),
                (math, "[object Math]"),
                (json, "[object JSON]"),
            ];
            for (value, expected) in cases {
                let tag = call_with(cx, object_prototype, "toString", value, &[]).unwrap();
                assert_eq!(text(cx, tag), expected, "{value:?}");
            }
            let to_string = get_object(cx, object_prototype, "toString");
            assert_eq!(get(cx, to_string, "length"), Value::Number(0.0));
            assert_eq!(
                get(cx, object_prototype, "constructor"),
                Value::Object(object_constructor)
            );
        })
        .unwrap();
}

/// `#sec-object.prototype.hasownproperty` and `#sec-object.prototype.propertyisenumerable`: an
/// own property, not an inherited one; enumerable or not; a String's index (enumerable) and its
/// "length" (not); an Array's "length" (not enumerable); the key goes through ToPropertyKey
/// before the this value through ToObject, so a key's toString runs before the TypeError of an
/// undefined this.
#[test]
fn has_own_and_property_is_enumerable() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let global_root = global(&mut agent, &realm);
    let calls = agent
        .with_cx(&realm, |cx| {
            let global = object(cx, &global_root);
            let realm_ref = cx.realm_ref(&realm).unwrap();
            let object_constructor = get_object(cx, global, "Object");
            let object_prototype = get_object(cx, object_constructor, "prototype");
            let parent = new_object(cx, global);
            let inherited = key(cx, "inherited");
            cx.set(parent, inherited, Value::Null, true)
                .unwrap()
                .unwrap();
            let child = cx
                .call(Value::Object(object_constructor), Value::Undefined, &[])
                .unwrap()
                .unwrap();
            let Value::Object(child) = child else {
                panic!("{child:?}");
            };
            // An object that inherits from parent: `new Object()` with a new target whose
            // "prototype" is parent (`#sec-object-value`, step 1).
            let realm_function = cx
                .new_host_function(realm_ref, "F", 0, one, true, None)
                .unwrap()
                .unwrap();
            let prototype_key = key(cx, "prototype");
            cx.set(realm_function, prototype_key, Value::Object(parent), true)
                .unwrap()
                .unwrap();
            let derived = cx
                .construct(
                    Value::Object(object_constructor),
                    &[],
                    Some(Value::Object(realm_function)),
                )
                .unwrap()
                .unwrap();
            let own = key(cx, "own");
            cx.set(child, own, Value::Null, true).unwrap().unwrap();
            let hidden = key(cx, "hidden");
            cx.define_property_or_throw(child, hidden, data(Value::Null, true, false, true))
                .unwrap()
                .unwrap();
            let has_own = |cx: &mut Cx<'_, TestHost>, this: Value, name: Value| {
                call_with(cx, object_prototype, "hasOwnProperty", this, &[name]).unwrap()
            };
            let is_enumerable = |cx: &mut Cx<'_, TestHost>, this: Value, name: Value| {
                call_with(cx, object_prototype, "propertyIsEnumerable", this, &[name]).unwrap()
            };
            let [
                own_name,
                hidden_name,
                inherited_name,
                absent_name,
                length_name,
            ] = ["own", "hidden", "inherited", "absent", "length"].map(|name| string(cx, name));
            let child_value = Value::Object(child);
            assert_eq!(has_own(cx, child_value, own_name), Value::Boolean(true));
            assert_eq!(has_own(cx, child_value, hidden_name), Value::Boolean(true));
            assert_eq!(has_own(cx, child_value, absent_name), Value::Boolean(false));
            assert_eq!(has_own(cx, derived, inherited_name), Value::Boolean(false));
            assert_eq!(get(cx, parent, "inherited"), Value::Null);
            let Value::Object(derived_object) = derived else {
                unreachable!()
            };
            assert_eq!(get(cx, derived_object, "inherited"), Value::Null);
            assert_eq!(
                is_enumerable(cx, child_value, own_name),
                Value::Boolean(true)
            );
            assert_eq!(
                is_enumerable(cx, child_value, hidden_name),
                Value::Boolean(false)
            );
            assert_eq!(
                is_enumerable(cx, child_value, absent_name),
                Value::Boolean(false)
            );
            assert_eq!(
                is_enumerable(cx, derived, inherited_name),
                Value::Boolean(false)
            );

            let abc = string(cx, "abc");
            assert_eq!(has_own(cx, abc, Value::Number(1.0)), Value::Boolean(true));
            assert_eq!(has_own(cx, abc, Value::Number(3.0)), Value::Boolean(false));
            assert_eq!(has_own(cx, abc, length_name), Value::Boolean(true));
            assert_eq!(
                is_enumerable(cx, abc, Value::Number(1.0)),
                Value::Boolean(true)
            );
            assert_eq!(is_enumerable(cx, abc, length_name), Value::Boolean(false));
            let array_constructor = get(cx, global, "Array");
            let array = cx
                .call(array_constructor, Value::Undefined, &[])
                .unwrap()
                .unwrap();
            assert_eq!(has_own(cx, array, length_name), Value::Boolean(true));
            assert_eq!(is_enumerable(cx, array, length_name), Value::Boolean(false));

            // ToPropertyKey of the key comes before ToObject of an undefined this.
            let key_object = new_object(cx, global);
            let converter = cx
                .new_host_function(realm_ref, "toString", 0, key_to_string, false, None)
                .unwrap()
                .unwrap();
            let to_string_key = key(cx, "toString");
            cx.set(key_object, to_string_key, Value::Object(converter), true)
                .unwrap()
                .unwrap();
            for name in ["hasOwnProperty", "propertyIsEnumerable"] {
                let error = thrown(call_with(
                    cx,
                    object_prototype,
                    name,
                    Value::Undefined,
                    &[Value::Object(key_object)],
                ));
                assert!(is_error_of(cx, global, error, "TypeError"), "{name}");
            }
            let k = key(cx, "k");
            cx.set(child, k, Value::Null, true).unwrap().unwrap();
            assert_eq!(
                invoke(cx, child, "hasOwnProperty", &[Value::Object(key_object)]).unwrap(),
                Value::Boolean(true)
            );
            cx.host().calls.clone()
        })
        .unwrap();
    assert_eq!(calls, ["toString", "toString", "toString"]);
}
