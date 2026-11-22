//! The Array builtins the harness files call (ECMA-262 23.1, `#sec-array-objects`): the `Array`
//! constructor (`#sec-array`), `Array.prototype.join` (`#sec-array.prototype.join`) with
//! JoinIndexedProperties (`#sec-joinindexedproperties`), `.push` (`#sec-array.prototype.push`)
//! and `.map` (`#sec-array.prototype.map`) with ArraySpeciesCreate (`#sec-arrayspeciescreate`),
//! the step budget and `string_length` (`docs/design/host-interface.md`, section 2). No script
//! runs: the callbacks are host functions. A constructor with its own @@species needs a
//! well-known symbol, which the host interface does not hand out, so that part of `map` is a
//! unit test of `builtins/array/mod.rs`. `Array.prototype.toString`
//! (`#sec-array.prototype.tostring`), which slice 2 takes early (`docs/design/engine.md`,
//! section 12, "A part before its slice"), is tested with scripts, in sloppy and in strict
//! code. Run also with the feature `collect-every-safe-point`.
#![cfg(test)]

use crate::support;

use support::host::{
    TestHost, agent, agent_with, get, get_object, global, is_error_of, key, limits, object, realm,
    string, text, thrown,
};
use support::script::{Outcome, eval_text, run, run_both};
use swell::{Abrupt, Completion, Cx, HostCall, ObjectRef, Termination, Value};

/// Both modes of `source` answer `expected`, a normal completion as `show` writes it.
fn both_answer(source: &str, expected: &str) {
    let (sloppy, strict) = run_both(source);
    let expected = Outcome::Normal(expected.to_string());
    assert_eq!(sloppy, expected, "sloppy: {source}");
    assert_eq!(strict, expected, "strict: {source}");
}

/// Both modes of `source` throw `name`: the name of an error, or a thrown value as `show`
/// writes it.
fn both_throw(source: &str, name: &str) {
    let (sloppy, strict) = run_both(source);
    let expected = Outcome::Throw(name.to_string());
    assert_eq!(sloppy, expected, "sloppy: {source}");
    assert_eq!(strict, expected, "strict: {source}");
}

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

/// `#sec-array.prototype.tostring`: an Array answers the result of its `join`, so the string
/// conversion of an Array (ToPrimitive, `#sec-toprimitive`, through OrdinaryToPrimitive) is its
/// elements joined with ",": `+` with a String (`#sec-applystringornumericbinaryoperator`),
/// `String` called (`#sec-string-constructor-string-value`), nested Arrays, and ToNumber in
/// `isNaN` (`#sec-isnan-number`), where `[]` is "" and so +0, and `[1, 2]` is "1,2" and so NaN.
#[test]
fn to_string_of_an_array_joins_its_elements() {
    both_answer("[1, 2, 3].toString();", "\"1,2,3\"");
    both_answer("[] + \"\";", "\"\"");
    both_answer("new Array(2, 4, 8, 16, 32) + \"\";", "\"2,4,8,16,32\"");
    both_answer("String([1, [2, 3]]);", "\"1,2,3\"");
    both_answer("[1, , null, undefined, \"x\"].toString();", "\"1,,,,x\"");
    both_answer("isNaN([]);", "false");
    both_answer("isNaN([1, 2]);", "true");
    both_answer("[7] * 2;", "14");
}

/// `#sec-array.prototype.tostring` and clause 18 (`#sec-ecmascript-standard-built-in-objects`):
/// `Array.prototype.toString` is a function with "length" 0 and "name" "toString", a property
/// { [[Writable]]: true, [[Enumerable]]: false, [[Configurable]]: true } of
/// `%Array.prototype%`, and not a constructor (`new` throws a `TypeError`).
#[test]
fn to_string_is_a_method_of_array_prototype() {
    both_answer(
        "var d = Object.getOwnPropertyDescriptor(Array.prototype, \"toString\"); \
         var f = Array.prototype.toString; \
         [typeof f, d.value === f, d.writable, d.enumerable, d.configurable, f.length, f.name, \
          Object.getPrototypeOf(f) === Function.prototype].join();",
        "\"function,true,true,false,true,0,toString,true\"",
    );
    both_throw("new Array.prototype.toString();", "TypeError");
    both_throw("new [].toString();", "TypeError");
}

/// `#sec-array.prototype.tostring`, steps 2 and 4: the method reads "join" from the object and
/// calls it with the object as `this` and no argument, so an Array whose own `join` is replaced
/// answers through the replacement, and so does a replaced `Array.prototype.join`.
#[test]
fn to_string_calls_the_join_of_the_object() {
    both_answer(
        "var a = [1, 2]; a.join = function () { return \"j\" + this.length + arguments.length; }; \
         a.toString();",
        "\"j20\"",
    );
    both_answer(
        "Array.prototype.join = function () { return \"replaced\"; }; [1, 2] + \"\";",
        "\"replaced\"",
    );
}

/// `#sec-array.prototype.tostring`, step 3: when "join" is not callable the method calls the
/// intrinsic `%Object.prototype.toString%` (`#sec-object.prototype.tostring`), not what
/// `Object.prototype.toString` holds now: an Array with a `join` that is not a function, an
/// Array after `Array.prototype.join` is deleted, and a plain object, which has no `join`.
#[test]
fn to_string_without_a_callable_join_is_object_prototype_to_string() {
    both_answer(
        "var a = [1, 2]; a.join = 1; a.toString();",
        "\"[object Array]\"",
    );
    both_answer(
        "var a = [1, 2]; a.join = {}; a + \"\";",
        "\"[object Array]\"",
    );
    both_answer(
        "delete Array.prototype.join; [1, 2].toString();",
        "\"[object Array]\"",
    );
    both_answer("Array.prototype.toString.call({});", "\"[object Object]\"");
    both_answer(
        "Object.prototype.toString = function () { return \"replaced\"; }; \
         [Array.prototype.toString.call({}), Array.prototype.toString.call({ join: null })].join();",
        "\"[object Object],[object Object]\"",
    );
    both_answer(
        "Array.prototype.toString.call(function () {});",
        "\"[object Function]\"",
    );
}

/// `#sec-array.prototype.tostring`: the method is generic (its note). Step 1, ToObject
/// (`#sec-toobject`): `undefined` and `null` throw a `TypeError`; a primitive is wrapped, and
/// the wrapper is the `this` of its `join`, in strict code too. A plain object with a callable
/// `join` answers through it, called on the object.
#[test]
fn to_string_is_generic() {
    both_answer(
        "var o = { join: function () { return this === o ? \"self\" : \"other\"; } }; \
         Array.prototype.toString.call(o);",
        "\"self\"",
    );
    both_answer(
        "var o = { length: 2, 0: \"a\", 1: \"b\", join: Array.prototype.join }; \
         Array.prototype.toString.call(o);",
        "\"a,b\"",
    );
    both_throw("Array.prototype.toString.call(undefined);", "TypeError");
    both_throw("Array.prototype.toString.call(null);", "TypeError");
    both_throw("Array.prototype.toString.call();", "TypeError");
    both_answer("Array.prototype.toString.call(5);", "\"[object Number]\"");
    both_answer(
        "Array.prototype.toString.call(\"ab\");",
        "\"[object String]\"",
    );
    both_answer(
        "Number.prototype.join = function () { return typeof this + this.valueOf(); }; \
         Array.prototype.toString.call(5);",
        "\"object5\"",
    );
}

/// `#sec-array.prototype.tostring`, steps 2 and 4 (`?`): an abrupt completion of the Get of
/// "join" (a getter that throws) and of the call of `join` is passed on, a thrown value that
/// is not an error included; a throw from step 1 comes before any Get.
#[test]
fn to_string_passes_on_abrupt_completions() {
    both_throw(
        "var o = {}; \
         Object.defineProperty(o, \"join\", { get: function () { throw new RangeError(\"g\"); } }); \
         Array.prototype.toString.call(o);",
        "RangeError",
    );
    both_throw(
        "var a = [1]; \
         Object.defineProperty(a, \"join\", { get: function () { throw 7; } }); \
         a + \"\";",
        "7",
    );
    both_throw(
        "Array.prototype.toString.call({ join: function () { throw new SyntaxError(\"c\"); } });",
        "SyntaxError",
    );
    both_throw(
        "var a = [1]; a.join = function () { throw \"x\"; }; String(a);",
        "\"x\"",
    );
}

/// The steps an entry of `source` spends; it must answer `expected`.
fn steps_of(source: &str, expected: &str) -> u64 {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(&mut agent, &realm, source),
        Outcome::Normal(expected.to_string()),
        "{source}"
    );
    agent.last_entry_steps().unwrap()
}

/// The budgets (`docs/design/engine.md`, decision 2): `Array.prototype.toString` is charged as
/// a call, and the `join` or `%Object.prototype.toString%` it calls each as a call of its own,
/// and its Get of "join" as every Get is, a step for each prototype it goes up to
/// (`#sec-ordinaryget`). So a script that calls `toString` spends what the same script calling
/// `join` or `Object.prototype.toString` itself spends, one step more for the call of
/// `toString`, and one more again when "join" is found on `%Array.prototype%`, not on the
/// object. The loop of `join` is charged too: the `toString` of an Array of length 2^32 - 1
/// ends in `Terminated(StepBudget)`, in sloppy and in strict code.
#[test]
fn to_string_is_charged_as_its_calls() {
    // Each pair differs only in the function `f` names: `t`, or what `t` calls.
    let pair = |setup: &str, expected: &str| {
        let through = steps_of(&format!("{setup} x.f = t; x.f();"), expected);
        let direct = steps_of(&format!("{setup} x.f = d; x.f();"), expected);
        (through, direct)
    };
    let t = "var t = Array.prototype.toString;";
    let (through, direct) = pair(
        &format!("{t} var d = Array.prototype.join; var x = [1, 2, 3];"),
        "\"1,2,3\"",
    );
    assert_eq!(through, direct + 2, "join found on %Array.prototype%");
    let (through, direct) = pair(
        &format!("{t} var d = function () {{ return \"0\"; }}; var x = [1, 2, 3]; x.join = d;"),
        "\"0\"",
    );
    assert_eq!(through, direct + 1, "an own join");
    let (through, direct) = pair(
        &format!("{t} var d = Object.prototype.toString; var x = {{ join: 1 }};"),
        "\"[object Object]\"",
    );
    assert_eq!(through, direct + 1, "%Object.prototype.toString%");

    let mut limits = limits();
    limits.steps_per_entry = 100_000;
    for prologue in ["", "\"use strict\";\n"] {
        assert_eq!(
            run(
                limits,
                &format!("{prologue}var a = []; a.length = 4294967295; a.toString();")
            ),
            Outcome::Terminated(Termination::StepBudget),
            "{prologue}"
        );
    }
}

/// The object `toString` works on is held across the Get of "join" and the call (decision 4):
/// the wrapper ToObject makes of a primitive is referenced by nothing else, and a getter of
/// "join" and the `join` it answers both make garbage; with `collect-every-safe-point` every
/// call collects, and `join` still reads the wrapper. An Array whose `join` makes garbage
/// still joins its elements.
#[test]
fn to_string_holds_its_object_across_the_call() {
    let n = garbage_objects();
    both_answer(
        &format!(
            "Object.defineProperty(Number.prototype, \"join\", {{ get: function () {{ \
               var g = []; for (var i = 0; i < {n}; i++) g.push({{ i: i }}); \
               return function () {{ \
                 var h = []; for (var i = 0; i < {n}; i++) h.push([i]); \
                 return typeof this + this.valueOf(); \
               }}; \
             }} }}); \
             Array.prototype.toString.call(5) + Array.prototype.toString.call(6);"
        ),
        "\"object5object6\"",
    );
    both_answer(
        &format!(
            "var a = [1, [2, 3], \"x\"]; \
             a.join = function () {{ \
               var g = []; for (var i = 0; i < {n}; i++) g.push({{ i: i }}); \
               return Array.prototype.join.call(this, \"-\"); \
             }}; \
             a.toString();"
        ),
        "\"1-2,3-x\"",
    );
}

/// The objects each garbage loop of [`to_string_holds_its_object_across_the_call`] makes: 50,
/// the full count, in the default mode and in the lane `slow` (`SWELL_FULL_COUNTS=1`); 5 in the
/// gate with the feature `collect-every-safe-point`, where every safe point collects (with 50
/// the test took 19.9 s there at a load of 100 to 110 on 17 cores, cargo-nextest). Five objects
/// a loop are still garbage made between the Get of "join" and the call, and between the call
/// and the read of the wrapper, with a collection at every one of their safe points.
fn garbage_objects() -> u32 {
    if cfg!(feature = "collect-every-safe-point")
        && !std::env::var("SWELL_FULL_COUNTS").is_ok_and(|value| value == "1")
    {
        5
    } else {
        50
    }
}

/// `%Object.prototype.toString%` is an intrinsic of the realm, so the realm holds it (decision
/// 4): after a script deletes `Object.prototype.toString` nothing else references that
/// function, and after two collections between entries (and one at every safe point with
/// `collect-every-safe-point`) the fallback of `Array.prototype.toString` still calls it.
#[test]
fn the_intrinsic_survives_collections_when_only_the_realm_holds_it() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "delete Object.prototype.toString; var a = [1, 2]; a.join = null; \
             typeof Object.prototype.toString;"
        ),
        Outcome::Normal("\"undefined\"".to_string())
    );
    for _ in 0..2 {
        assert!(matches!(agent.collect(), Ok(swell::Outcome::Normal(_))));
    }
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "a.toString() + \" \" + Array.prototype.toString.call({}) + \" \" + String(a);"
        ),
        Outcome::Normal("\"[object Array] [object Object] [object Array]\"".to_string())
    );
}

/// `#sec-array.prototype.tostring`, step 3, with two realms: each realm's
/// `Array.prototype.toString` falls back to the intrinsic of its own realm, whatever the
/// scripts of either realm did to their `Object.prototype.toString` (realm A replaces it, realm
/// B deletes it), and on an Array of the other realm too.
#[test]
fn the_fallback_is_the_intrinsic_in_each_realm() {
    let mut agent = agent().unwrap();
    let realm_a = realm(&mut agent);
    let realm_b = support::host::realm(&mut agent);
    let global_a = global(&mut agent, &realm_a);
    let global_b = global(&mut agent, &realm_b);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm_a,
            "Object.prototype.toString = function () { return \"A\"; }; \
             var a = [1]; a.join = null; a.toString() + ({}).toString();"
        ),
        Outcome::Normal("\"[object Array]A\"".to_string())
    );
    assert_eq!(
        eval_text(
            &mut agent,
            &realm_b,
            "delete Object.prototype.toString; var b = [2]; b.join = 0; b + \"\";"
        ),
        Outcome::Normal("\"[object Array]\"".to_string())
    );
    agent
        .with_cx(&realm_a, |cx| {
            let global_a = object(cx, &global_a);
            let global_b = object(cx, &global_b);
            let a = get(cx, global_a, "a");
            let b = get(cx, global_b, "b");
            for (global, this) in [(global_a, a), (global_a, b), (global_b, a), (global_b, b)] {
                let result = array_method(cx, global, "toString", this, &[]);
                assert_eq!(joined(cx, result), "[object Array]");
            }
        })
        .unwrap();
}
