//! The Array builtins the harness files call (ECMA-262 23.1, `#sec-array-objects`): the `Array`
//! constructor (`#sec-array`), `Array.prototype.join` (`#sec-array.prototype.join`) with
//! JoinIndexedProperties (`#sec-joinindexedproperties`), `.push` (`#sec-array.prototype.push`)
//! and `.map` (`#sec-array.prototype.map`) with ArraySpeciesCreate (`#sec-arrayspeciescreate`),
//! the step budget and `string_length` (`docs/design/host-interface.md`, section 2). No script
//! runs: the callbacks are host functions. A constructor with its own @@species needs a
//! well-known symbol, which the host interface does not hand out, so that part of `map` is a
//! unit test of `builtins/array.rs`. Run also with the feature `collect-every-safe-point`.
#![cfg(test)]

mod support;

use support::host::{
    TestHost, agent, agent_with, get, get_object, global, is_error_of, key, limits, object, realm,
    string, text, thrown,
};
use swell::{Abrupt, Completion, Cx, HostCall, ObjectRef, Termination, Value};

/// `Array(...args)`.
fn array(cx: &mut Cx<'_, TestHost>, global: ObjectRef, args: &[Value]) -> ObjectRef {
    let constructor = get(cx, global, "Array");
    match cx
        .call(constructor, Value::Undefined, args)
        .unwrap()
        .unwrap()
    {
        Value::Object(made) => made,
        other => panic!("Array answered {other:?}"),
    }
}

/// `Array.prototype[name]` called with `this` and `args`.
fn array_method(
    cx: &mut Cx<'_, TestHost>,
    global: ObjectRef,
    name: &str,
    this: Value,
    args: &[Value],
) -> Completion<Value> {
    let constructor = get_object(cx, global, "Array");
    let prototype = get_object(cx, constructor, "prototype");
    let method = get(cx, prototype, name);
    cx.call(method, this, args).unwrap()
}

/// A new ordinary object.
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

/// The text of a completion that must be a String.
fn joined(cx: &mut Cx<'_, TestHost>, completion: Completion<Value>) -> String {
    let value = completion.unwrap();
    text(cx, value)
}

/// A toString that answers "T".
fn to_string_t(cx: &mut Cx<'_, TestHost>, _call: HostCall<'_>) -> Completion<Value> {
    Ok(Value::String(cx.new_string("T").unwrap()))
}

/// `#sec-array.prototype.join` with `#sec-joinindexedproperties`: holes, undefined and null
/// are the empty String; the separator is "," when undefined and ToString of itself otherwise;
/// each element goes through ToString, an object's own toString included; an array-like object
/// and a String this value (ToObject) join their indices below "length"; no element is "".
/// The arrays are made by the `Array` constructor (`#sec-array`): no argument, one Number (the
/// length; one that is not a uint32 throws a RangeError), one other value (the element), and
/// several (the elements).
#[test]
fn join_of_holes_and_nullish() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let global_root = global(&mut agent, &realm);
    agent
        .with_cx(&realm, |cx| {
            let global = object(cx, &global_root);
            let realm_ref = cx.realm_ref(&realm).unwrap();
            let x = string(cx, "x");
            let mixed = array(
                cx,
                global,
                &[Value::Number(1.0), Value::Null, Value::Undefined, x],
            );
            // A hole at 4, then "y" at 5.
            let five = key(cx, "5");
            let y = string(cx, "y");
            cx.set(mixed, five, y, true).unwrap().unwrap();
            assert_eq!(get(cx, mixed, "length"), Value::Number(6.0));
            let mixed_value = Value::Object(mixed);
            let result = array_method(cx, global, "join", mixed_value, &[]);
            assert_eq!(joined(cx, result), "1,,,x,,y");
            let result = array_method(cx, global, "join", mixed_value, &[Value::Undefined]);
            assert_eq!(joined(cx, result), "1,,,x,,y");
            let dash = string(cx, " - ");
            let result = array_method(cx, global, "join", mixed_value, &[dash]);
            assert_eq!(joined(cx, result), "1 -  -  - x -  - y");
            let result = array_method(cx, global, "join", mixed_value, &[Value::Number(0.5)]);
            assert_eq!(joined(cx, result), "10.50.50.5x0.50.5y");

            // An element with its own toString.
            let element = new_object(cx, global);
            let to_string = cx
                .new_host_function(realm_ref, "toString", 0, to_string_t, false, None)
                .unwrap()
                .unwrap();
            let to_string_key = key(cx, "toString");
            cx.set(element, to_string_key, Value::Object(to_string), true)
                .unwrap()
                .unwrap();
            let with_object = array(cx, global, &[Value::Object(element), Value::Boolean(true)]);
            let result = array_method(cx, global, "join", Value::Object(with_object), &[]);
            assert_eq!(joined(cx, result), "T,true");

            // Array-likes.
            let like = new_object(cx, global);
            let [zero, one, length] = ["0", "1", "length"].map(|name| key(cx, name));
            let a = string(cx, "a");
            let b = string(cx, "b");
            cx.set(like, zero, a, true).unwrap().unwrap();
            cx.set(like, one, b, true).unwrap().unwrap();
            cx.set(like, length, Value::Number(1.9), true)
                .unwrap()
                .unwrap();
            let result = array_method(cx, global, "join", Value::Object(like), &[]);
            assert_eq!(joined(cx, result), "a");
            let abc = string(cx, "abc");
            let result = array_method(cx, global, "join", abc, &[]);
            assert_eq!(joined(cx, result), "a,b,c");
            let empty = array(cx, global, &[]);
            let result = array_method(cx, global, "join", Value::Object(empty), &[]);
            assert_eq!(joined(cx, result), "");
            let error = thrown(array_method(cx, global, "join", Value::Undefined, &[]));
            assert!(is_error_of(cx, global, error, "TypeError"));

            // The constructor's other cases.
            let sized = array(cx, global, &[Value::Number(3.0)]);
            assert_eq!(get(cx, sized, "length"), Value::Number(3.0));
            let result = array_method(cx, global, "join", Value::Object(sized), &[]);
            assert_eq!(joined(cx, result), ",,");
            let single = array(cx, global, &[a]);
            assert_eq!(get(cx, single, "length"), Value::Number(1.0));
            assert_eq!(get(cx, single, "0"), a);
            let constructor = get(cx, global, "Array");
            for bad in [1.5, -1.0, 4_294_967_296.0, f64::NAN] {
                let error = thrown(
                    cx.call(constructor, Value::Undefined, &[Value::Number(bad)])
                        .unwrap(),
                );
                assert!(is_error_of(cx, global, error, "RangeError"), "{bad}");
            }
            let constructed = cx
                .construct(constructor, &[Value::Number(2.0)], None)
                .unwrap()
                .unwrap();
            let Value::Object(constructed) = constructed else {
                panic!("{constructed:?}");
            };
            assert_eq!(get(cx, constructed, "length"), Value::Number(2.0));
            let Value::Object(constructor_object) = constructor else {
                unreachable!()
            };
            assert_eq!(get(cx, constructor_object, "length"), Value::Number(1.0));
        })
        .unwrap();
}

/// `#sec-array.prototype.push`: an array-like whose "length" is 2^53 - 1 throws a TypeError
/// when one more element would pass it, and sets nothing; push with no item sets "length" to
/// ToLength of it; push appends in order and answers the new length.
#[test]
fn push_past_2_53_minus_1_throws() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let global_root = global(&mut agent, &realm);
    agent
        .with_cx(&realm, |cx| {
            let global = object(cx, &global_root);
            let object_constructor = get_object(cx, global, "Object");
            let names = get(cx, object_constructor, "getOwnPropertyNames");
            let max = 9_007_199_254_740_991.0;
            let like = new_object(cx, global);
            let length = key(cx, "length");
            cx.set(like, length, Value::Number(max), true)
                .unwrap()
                .unwrap();
            let error = thrown(array_method(
                cx,
                global,
                "push",
                Value::Object(like),
                &[Value::Null],
            ));
            assert!(is_error_of(cx, global, error, "TypeError"));
            let Value::Object(keys) = cx
                .call(names, Value::Undefined, &[Value::Object(like)])
                .unwrap()
                .unwrap()
            else {
                panic!("no keys");
            };
            assert_eq!(
                get(cx, keys, "length"),
                Value::Number(1.0),
                "only \"length\""
            );
            assert_eq!(
                array_method(cx, global, "push", Value::Object(like), &[]).unwrap(),
                Value::Number(max)
            );
            cx.set(like, length, Value::Number(max + 2.0), true)
                .unwrap()
                .unwrap();
            assert_eq!(
                array_method(cx, global, "push", Value::Object(like), &[]).unwrap(),
                Value::Number(max)
            );
            assert_eq!(get(cx, like, "length"), Value::Number(max));
            cx.set(like, length, Value::Number(max - 1.0), true)
                .unwrap()
                .unwrap();
            assert_eq!(
                array_method(cx, global, "push", Value::Object(like), &[Value::Null]).unwrap(),
                Value::Number(max)
            );
            assert_eq!(get(cx, like, &(max - 1.0).to_string()), Value::Null);

            let list = array(cx, global, &[]);
            let a = string(cx, "a");
            assert_eq!(
                array_method(
                    cx,
                    global,
                    "push",
                    Value::Object(list),
                    &[a, Value::Number(2.0)]
                )
                .unwrap(),
                Value::Number(2.0)
            );
            assert_eq!(
                array_method(cx, global, "push", Value::Object(list), &[Value::Null]).unwrap(),
                Value::Number(3.0)
            );
            let result = array_method(cx, global, "join", Value::Object(list), &[]);
            assert_eq!(joined(cx, result), "a,2,");
            let error = thrown(array_method(cx, global, "push", Value::Null, &[]));
            assert!(is_error_of(cx, global, error, "TypeError"));
        })
        .unwrap();
}

/// A callback that records (value, index, this) and answers the value's text doubled.
fn double(cx: &mut Cx<'_, TestHost>, call: HostCall<'_>) -> Completion<Value> {
    let value = call.args.first().copied().unwrap_or(Value::Undefined);
    let index = call.args.get(1).copied().unwrap_or(Value::Undefined);
    let value_text = match value {
        Value::String(string) => cx.to_rust_string_lossy(string).unwrap(),
        other => format!("{other:?}"),
    };
    let this_text = match call.this {
        Value::String(string) => cx.to_rust_string_lossy(string).unwrap(),
        other => format!("{other:?}"),
    };
    let Value::Number(index) = index else {
        panic!("index {index:?}");
    };
    cx.host_mut()
        .calls
        .push(format!("{value_text}@{index} this={this_text}"));
    Ok(Value::String(
        cx.new_string(&format!("{value_text}{value_text}")).unwrap(),
    ))
}

/// A constructor with no @@species of its own and none inherited.
fn plain_constructor(_cx: &mut Cx<'_, TestHost>, call: HostCall<'_>) -> Completion<Value> {
    Ok(Value::Object(call.callee))
}

/// `#sec-array.prototype.map` with `#sec-arrayspeciescreate`, what the host interface reaches:
/// the callback runs for each element present, not for holes, with (value, index, object) and
/// thisArg; the result has the source's length and its holes; the result of an Array whose
/// constructor is the `%Array%` of another realm is an Array of the current realm (step 5.c);
/// a constructor without @@species, an undefined constructor and an array-like this make an
/// Array; a constructor that is neither an object nor undefined throws a TypeError, and so
/// does a callback that is not callable. `Array.prototype.constructor` is `%Array%`.
#[test]
fn map_uses_species_and_skips_holes() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let other_realm = support::host::realm(&mut agent);
    let global_root = global(&mut agent, &realm);
    let other_global_root = global(&mut agent, &other_realm);
    let calls = agent
        .with_cx(&realm, |cx| {
            let global = object(cx, &global_root);
            let other_global = object(cx, &other_global_root);
            let realm_ref = cx.realm_ref(&realm).unwrap();
            let callback = cx
                .new_host_function(realm_ref, "double", 1, double, false, None)
                .unwrap()
                .unwrap();
            let [a, c, this_arg] = ["a", "c", "self"].map(|text| string(cx, text));
            let source = array(cx, global, &[a, Value::Undefined, c]);
            let one = key(cx, "1");
            cx.delete_property_or_throw(source, one).unwrap().unwrap();
            let mapped = array_method(
                cx,
                global,
                "map",
                Value::Object(source),
                &[Value::Object(callback), this_arg],
            )
            .unwrap();
            let Value::Object(mapped) = mapped else {
                panic!("{mapped:?}");
            };
            assert_eq!(get(cx, mapped, "length"), Value::Number(3.0));
            let result = array_method(cx, global, "join", Value::Object(mapped), &[]);
            assert_eq!(joined(cx, result), "aa,,cc");
            let object_constructor = get_object(cx, global, "Object");
            let object_prototype = get_object(cx, object_constructor, "prototype");
            let has_own = get(cx, object_prototype, "hasOwnProperty");
            assert_eq!(
                cx.call(has_own, Value::Object(mapped), &[Value::Number(1.0)])
                    .unwrap()
                    .unwrap(),
                Value::Boolean(false)
            );

            let array_constructor = get_object(cx, global, "Array");
            let array_prototype = get(cx, array_constructor, "prototype");
            let Value::Object(array_prototype_object) = array_prototype else {
                panic!("{array_prototype:?}");
            };
            assert_eq!(
                get(cx, array_prototype_object, "constructor"),
                Value::Object(array_constructor)
            );
            let get_prototype_of = get(cx, object_constructor, "getPrototypeOf");
            let prototype_of = |cx: &mut Cx<'_, TestHost>, value: Value| {
                cx.call(get_prototype_of, Value::Undefined, &[value])
                    .unwrap()
                    .unwrap()
            };
            assert_eq!(prototype_of(cx, Value::Object(mapped)), array_prototype);

            // An Array of the other realm: its constructor is that realm's %Array%, so this
            // realm's map makes an Array of this realm, and that realm's map one of that realm.
            let foreign = array(cx, other_global, &[a]);
            let other_array_constructor = get_object(cx, other_global, "Array");
            let other_array_prototype = get_object(cx, other_array_constructor, "prototype");
            let other_map = get(cx, other_array_prototype, "map");
            let mapped = array_method(
                cx,
                global,
                "map",
                Value::Object(foreign),
                &[Value::Object(callback)],
            )
            .unwrap();
            assert_eq!(prototype_of(cx, mapped), array_prototype);
            let mapped = cx
                .call(
                    other_map,
                    Value::Object(foreign),
                    &[Value::Object(callback)],
                )
                .unwrap()
                .unwrap();
            assert_eq!(
                prototype_of(cx, mapped),
                Value::Object(other_array_prototype)
            );

            // Constructors without @@species, undefined, and not an object.
            let constructor_key = key(cx, "constructor");
            let constructor = cx
                .new_host_function(realm_ref, "Plain", 0, plain_constructor, true, None)
                .unwrap()
                .unwrap();
            for value in [Value::Object(constructor), Value::Undefined] {
                let source = array(cx, global, &[a]);
                cx.set(source, constructor_key, value, true)
                    .unwrap()
                    .unwrap();
                let mapped = array_method(
                    cx,
                    global,
                    "map",
                    Value::Object(source),
                    &[Value::Object(callback)],
                )
                .unwrap();
                assert_eq!(prototype_of(cx, mapped), array_prototype, "{value:?}");
            }
            let source = array(cx, global, &[a]);
            cx.set(source, constructor_key, Value::Number(1.0), true)
                .unwrap()
                .unwrap();
            let error = thrown(array_method(
                cx,
                global,
                "map",
                Value::Object(source),
                &[Value::Object(callback)],
            ));
            assert!(is_error_of(cx, global, error, "TypeError"));

            // An array-like this makes an Array.
            let like = new_object(cx, global);
            let [zero, length] = ["0", "length"].map(|name| key(cx, name));
            cx.set(like, zero, c, true).unwrap().unwrap();
            cx.set(like, length, Value::Number(1.0), true)
                .unwrap()
                .unwrap();
            let mapped = array_method(
                cx,
                global,
                "map",
                Value::Object(like),
                &[Value::Object(callback)],
            )
            .unwrap();
            assert_eq!(prototype_of(cx, mapped), array_prototype);
            let is_array = get(cx, array_constructor, "isArray");
            assert_eq!(
                cx.call(is_array, Value::Undefined, &[mapped])
                    .unwrap()
                    .unwrap(),
                Value::Boolean(true)
            );

            let error = thrown(array_method(
                cx,
                global,
                "map",
                Value::Object(source),
                &[Value::Null],
            ));
            assert!(is_error_of(cx, global, error, "TypeError"));
            cx.host().calls.clone()
        })
        .unwrap();
    assert_eq!(
        calls,
        [
            "a@0 this=self",
            "c@2 this=self",
            "a@0 this=Undefined",
            "a@0 this=Undefined",
            "a@0 this=Undefined",
            "a@0 this=Undefined",
            "c@0 this=Undefined",
        ]
    );
}

/// A callback that makes garbage, then answers a new String "v<index>".
fn fresh(cx: &mut Cx<'_, TestHost>, call: HostCall<'_>) -> Completion<Value> {
    for _ in 0..8 {
        cx.new_string("garbage").unwrap();
    }
    let index = match call.args.get(1) {
        Some(Value::Number(index)) => *index,
        other => panic!("index {other:?}"),
    };
    Ok(Value::String(cx.new_string(&format!("v{index}")).unwrap()))
}

/// The array `map` builds is held in the activation of `map` (`object/builtin_function.rs`,
/// decision 4): with `collect-every-safe-point` every callback call is a re-entry that collects,
/// and the result still reads every element the callbacks answered.
#[test]
fn map_result_survives_a_collection_at_each_callback() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let global_root = global(&mut agent, &realm);
    agent
        .with_cx(&realm, |cx| {
            let global = object(cx, &global_root);
            let realm_ref = cx.realm_ref(&realm).unwrap();
            let callback = cx
                .new_host_function(realm_ref, "fresh", 1, fresh, false, None)
                .unwrap()
                .unwrap();
            let source = array(cx, global, &[Value::Number(20.0)]);
            for index in 0..20 {
                let index_key = key(cx, &index.to_string());
                cx.set(source, index_key, Value::Null, true)
                    .unwrap()
                    .unwrap();
            }
            let mapped = array_method(
                cx,
                global,
                "map",
                Value::Object(source),
                &[Value::Object(callback)],
            )
            .unwrap();
            let Value::Object(mapped) = mapped else {
                panic!("{mapped:?}");
            };
            for index in 0..20 {
                let element = get(cx, mapped, &index.to_string());
                assert_eq!(text(cx, element), format!("v{index}"));
            }
        })
        .unwrap();
}

/// JoinIndexedProperties loops over every index below the length, a step each (decision 2): an
/// Array of length 2^32 - 1, all holes, ends in `Terminated(StepBudget)`, not in a hang, and its
/// partial result is never allocated as a String. The next entry runs afresh.
#[test]
fn join_of_length_2_32_minus_1_ends_in_step_budget() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let global_root = global(&mut agent, &realm);
    agent
        .with_cx(&realm, |cx| {
            let global = object(cx, &global_root);
            let huge = array(cx, global, &[Value::Number(4_294_967_295.0)]);
            assert_eq!(get(cx, huge, "length"), Value::Number(4_294_967_295.0));
            let result = array_method(cx, global, "join", Value::Object(huge), &[]);
            assert_eq!(result, Err(Abrupt::Terminated(Termination::StepBudget)));
        })
        .unwrap();
    agent
        .with_cx(&realm, |cx| {
            let global = object(cx, &global_root);
            let small = array(cx, global, &[Value::Number(2.0)]);
            let result = array_method(cx, global, "join", Value::Object(small), &[]);
            assert_eq!(joined(cx, result), ",");
        })
        .unwrap();
}

/// `join` builds a String at run time from other values, so it checks `string_length`
/// (`docs/design/host-interface.md`, section 2): with `string_length` 10, a result of 10 code
/// units is made and one of 11 throws a RangeError before it is allocated.
#[test]
fn join_past_string_length_throws_range_error() {
    let mut limits = limits();
    limits.string_length = 10;
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    let global_root = global(&mut agent, &realm);
    agent
        .with_cx(&realm, |cx| {
            let global = object(cx, &global_root);
            let [abcd, efghi, efghij] = ["abcd", "efghi", "efghij"].map(|text| string(cx, text));
            let fits = array(cx, global, &[abcd, efghi]);
            let result = array_method(cx, global, "join", Value::Object(fits), &[]);
            assert_eq!(joined(cx, result), "abcd,efghi");
            let over = array(cx, global, &[abcd, efghij]);
            let error = thrown(array_method(cx, global, "join", Value::Object(over), &[]));
            assert!(is_error_of(cx, global, error, "RangeError"));
            let long_separator = string(cx, "-----");
            let over = array(cx, global, &[abcd, abcd]);
            let error = thrown(array_method(
                cx,
                global,
                "join",
                Value::Object(over),
                &[long_separator],
            ));
            assert!(is_error_of(cx, global, error, "RangeError"));
        })
        .unwrap();
}
