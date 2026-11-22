//! The JSON object (ECMA-262 25.5, `#sec-json-object`): `JSON.stringify`
//! (`#sec-json.stringify`) with SerializeJSONProperty (`#sec-serializejsonproperty`),
//! SerializeJSONObject (`#sec-serializejsonobject`), SerializeJSONArray
//! (`#sec-serializejsonarray`), QuoteJSONString (`#sec-quotejsonstring`) and UnicodeEscape
//! (`#sec-unicodeescape`); its walk over nested values on an explicit stack, `reentry_depth` of
//! its callbacks and `string_length` of its result (`docs/design/host-interface.md`, section 2).
//! No script runs: `toJSON` and the replacer are host functions. Run also with the feature
//! `collect-every-safe-point`.
#![cfg(test)]

use crate::support;

use support::host::{
    TestHost, agent, agent_with, data, get, get_object, global, is_error_of, key, limits, object,
    realm, string, text, thrown,
};
use support::stack::{TestPath, test_path};
use swell::{Completion, Cx, HostCall, ObjectRef, Value};

/// `JSON.stringify(args)`.
fn stringify(cx: &mut Cx<'_, TestHost>, global: ObjectRef, args: &[Value]) -> Completion<Value> {
    let json = get_object(cx, global, "JSON");
    let method = get(cx, json, "stringify");
    cx.call(method, Value::Object(json), args).unwrap()
}

/// The text `JSON.stringify(args)` answers.
fn json(cx: &mut Cx<'_, TestHost>, global: ObjectRef, args: &[Value]) -> String {
    let value = stringify(cx, global, args).unwrap();
    text(cx, value)
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

/// `Array(...args)`.
fn array(cx: &mut Cx<'_, TestHost>, global: ObjectRef, args: &[Value]) -> Value {
    let constructor = get(cx, global, "Array");
    cx.call(constructor, Value::Undefined, args)
        .unwrap()
        .unwrap()
}

/// Set `object[name]` to `value`.
fn put(cx: &mut Cx<'_, TestHost>, object: ObjectRef, name: &str, value: Value) {
    let name = key(cx, name);
    cx.set(object, name, value, true).unwrap().unwrap();
}

/// A toJSON that answers its key followed by "!".
fn to_json(cx: &mut Cx<'_, TestHost>, call: HostCall<'_>) -> Completion<Value> {
    let key = match call.args.first() {
        Some(Value::String(key)) => cx.to_rust_string_lossy(*key).unwrap(),
        other => panic!("toJSON got {other:?}"),
    };
    Ok(Value::String(cx.new_string(&format!("{key}!")).unwrap()))
}

/// A host function, which JSON does not render.
fn nothing(_cx: &mut Cx<'_, TestHost>, _call: HostCall<'_>) -> Completion<Value> {
    Ok(Value::Undefined)
}

/// A `toString` and `valueOf` that answers the "primitive" property of its this value. It
/// stands in for the methods of `%String.prototype%` and `%Number.prototype%`, which are slice 3:
/// SerializeJSONProperty and the gap unwrap a String or Number object through ToString or
/// ToNumber, which read them.
fn primitive(cx: &mut Cx<'_, TestHost>, call: HostCall<'_>) -> Completion<Value> {
    let Value::Object(this) = call.this else {
        panic!("this {:?}", call.this);
    };
    let name = key(cx, "primitive");
    cx.get_property(this, name).unwrap()
}

/// ToObject of `value` (a Boolean, Number or String object), with [`primitive`] as its own
/// `toString` and `valueOf`.
fn wrap(cx: &mut Cx<'_, TestHost>, method: ObjectRef, value: Value) -> ObjectRef {
    let wrapper = cx.to_object(value).unwrap().unwrap();
    put(cx, wrapper, "primitive", value);
    put(cx, wrapper, "toString", Value::Object(method));
    put(cx, wrapper, "valueOf", Value::Object(method));
    wrapper
}

/// The host function [`primitive`].
fn primitive_method(cx: &mut Cx<'_, TestHost>, realm: &swell::Realm) -> ObjectRef {
    let realm_ref = cx.realm_ref(realm).unwrap();
    cx.new_host_function(realm_ref, "primitive", 0, primitive, false, None)
        .unwrap()
        .unwrap()
}

/// `#sec-json.stringify` and `#sec-serializejsonproperty`: undefined and a function give
/// undefined; null, true, false; a finite Number is ToString of it, NaN and ±∞ are null; a
/// String is quoted with the escapes of Table 78 and \u00XX below U+0020; Boolean, Number and
/// String objects give their value; an object gives its enumerable own String keys in
/// OrdinaryOwnPropertyKeys order and skips undefined and functions; an array gives every index
/// below its length, undefined and functions as null; empty ones are {} and []; toJSON is
/// called with the key, "" at the top.
#[test]
fn stringify_values_and_nesting() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let global_root = global(&mut agent, &realm);
    agent
        .with_cx(&realm, |cx| {
            let global = object(cx, &global_root);
            let realm_ref = cx.realm_ref(&realm).unwrap();
            let function = cx
                .new_host_function(realm_ref, "nothing", 0, nothing, false, None)
                .unwrap()
                .unwrap();
            assert_eq!(stringify(cx, global, &[]).unwrap(), Value::Undefined);
            assert_eq!(
                stringify(cx, global, &[Value::Undefined]).unwrap(),
                Value::Undefined
            );
            assert_eq!(
                stringify(cx, global, &[Value::Object(function)]).unwrap(),
                Value::Undefined
            );
            let n = Value::Number;
            for (value, expected) in [
                (Value::Null, "null"),
                (Value::Boolean(true), "true"),
                (Value::Boolean(false), "false"),
                (n(1.5), "1.5"),
                (n(-0.0), "0"),
                (n(1e21), "1e+21"),
                (n(f64::NAN), "null"),
                (n(f64::INFINITY), "null"),
                (n(f64::NEG_INFINITY), "null"),
            ] {
                assert_eq!(json(cx, global, &[value]), expected, "{value:?}");
            }
            let escapes = string(cx, "a\"b\\c\u{8}\u{c}\n\r\t\u{1}\u{1f} \u{7f}é");
            assert_eq!(
                json(cx, global, &[escapes]),
                "\"a\\\"b\\\\c\\b\\f\\n\\r\\t\\u0001\\u001f \u{7f}é\""
            );

            // A Boolean object gives its [[BooleanData]] without a call: its own valueOf would
            // answer false. A Number and a String object go through ToNumber and ToString.
            let method = primitive_method(cx, &realm);
            let wrapped_true = wrap(cx, method, Value::Boolean(true));
            put(cx, wrapped_true, "primitive", Value::Boolean(false));
            let wrapped_two = wrap(cx, method, n(2.0));
            let x = string(cx, "x");
            let wrapped_x = wrap(cx, method, x);
            let wrappers = array(
                cx,
                global,
                &[
                    Value::Object(wrapped_true),
                    Value::Object(wrapped_two),
                    Value::Object(wrapped_x),
                ],
            );
            assert_eq!(json(cx, global, &[wrappers]), "[true,2,\"x\"]");

            let inner = new_object(cx, global);
            put(cx, inner, "b", Value::Null);
            let list = array(cx, global, &[n(1.0), Value::Object(inner)]);
            let outer = new_object(cx, global);
            let s = string(cx, "s");
            put(cx, outer, "a", list);
            put(cx, outer, "c", Value::Undefined);
            put(cx, outer, "d", Value::Object(function));
            put(cx, outer, "e", s);
            let hidden = key(cx, "hidden");
            cx.define_property_or_throw(outer, hidden, data(n(1.0), true, false, true))
                .unwrap()
                .unwrap();
            put(cx, outer, "1", n(1.0));
            assert_eq!(
                json(cx, global, &[Value::Object(outer)]),
                "{\"1\":1,\"a\":[1,{\"b\":null}],\"e\":\"s\"}"
            );
            let holes = array(cx, global, &[Value::Undefined, Value::Object(function)]);
            let Value::Object(holes_object) = holes else {
                panic!("{holes:?}");
            };
            put(cx, holes_object, "3", n(3.0));
            assert_eq!(json(cx, global, &[holes]), "[null,null,null,3]");
            let empty_object = new_object(cx, global);
            assert_eq!(json(cx, global, &[Value::Object(empty_object)]), "{}");
            let empty_array = array(cx, global, &[]);
            assert_eq!(json(cx, global, &[empty_array]), "[]");

            // toJSON, with the key; "" at the top.
            let with_to_json = new_object(cx, global);
            let to_json_function = cx
                .new_host_function(realm_ref, "toJSON", 1, to_json, false, None)
                .unwrap()
                .unwrap();
            put(cx, with_to_json, "toJSON", Value::Object(to_json_function));
            assert_eq!(json(cx, global, &[Value::Object(with_to_json)]), "\"!\"");
            let holder = new_object(cx, global);
            put(cx, holder, "k", Value::Object(with_to_json));
            let in_list = array(cx, global, &[Value::Object(with_to_json), Value::Null]);
            put(cx, holder, "list", in_list);
            assert_eq!(
                json(cx, global, &[Value::Object(holder)]),
                "{\"k\":\"k!\",\"list\":[\"0!\",null]}"
            );
        })
        .unwrap();
}

/// A replacer that records its `this` (whether it has the key as an own property) and its key,
/// drops the key "drop", doubles Numbers and keeps the rest.
fn replacer(cx: &mut Cx<'_, TestHost>, call: HostCall<'_>) -> Completion<Value> {
    let key = call.args.first().copied().unwrap_or(Value::Undefined);
    let value = call.args.get(1).copied().unwrap_or(Value::Undefined);
    let Value::String(key_string) = key else {
        panic!("key {key:?}");
    };
    let key_text = cx.to_rust_string_lossy(key_string).unwrap();
    let Value::Object(holder) = call.this else {
        panic!("this {:?}", call.this);
    };
    let property_key = swell::PropertyKey::String(key_string);
    let own = cx.has_property(holder, property_key).unwrap().unwrap();
    cx.host_mut().calls.push(format!("{key_text}:{own}"));
    Ok(match (key_text.as_str(), value) {
        ("drop", _) => Value::Undefined,
        (_, Value::Number(number)) => Value::Number(number * 2.0),
        _ => value,
    })
}

/// A replacer that stringifies its value again with itself as the replacer: a re-entry per
/// level, which `reentry_depth` bounds.
fn reenter(cx: &mut Cx<'_, TestHost>, call: HostCall<'_>) -> Completion<Value> {
    let value = call.args.get(1).copied().unwrap_or(Value::Undefined);
    let Value::Object(holder) = call.this else {
        panic!("this {:?}", call.this);
    };
    let json_key = key(cx, "json");
    let json = cx.get_property(holder, json_key).unwrap()?;
    let Value::Object(json_object) = json else {
        return Ok(value);
    };
    let stringify_key = key(cx, "stringify");
    let method = cx.get_property(json_object, stringify_key).unwrap()?;
    cx.call(method, json, &[call.this, Value::Object(call.callee)])
        .unwrap()
}

/// `#sec-json.stringify`, the replacer and the space: a replacer function is called with the
/// holder as `this`, the key and the value, "" first, and its answer is serialized, undefined
/// dropping the property; a replacer array gives the property list, its Strings, Numbers and
/// String and Number objects as Strings, each once, in order, other values ignored, and it does
/// not filter array elements; a space Number gives that many spaces up to 10, below 1 none; a
/// space String its first 10 code units; Number and String objects are unwrapped; another space
/// gives none. The indentation puts each member on its own line with ": ", and an empty object
/// stays {}. A replacer that stringifies again re-enters until `reentry_depth` throws a
/// RangeError. `JSON.stringify` has length 3.
#[test]
fn stringify_replacer_and_space() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let global_root = global(&mut agent, &realm);
    let calls = agent
        .with_cx(&realm, |cx| {
            let global = object(cx, &global_root);
            let realm_ref = cx.realm_ref(&realm).unwrap();
            let n = Value::Number;
            let replace = cx
                .new_host_function(realm_ref, "replacer", 2, replacer, false, None)
                .unwrap()
                .unwrap();
            let value = new_object(cx, global);
            put(cx, value, "keep", n(1.0));
            put(cx, value, "drop", n(2.0));
            // `Array(3)` would be a length: the element is put.
            let nested = array(cx, global, &[]);
            let Value::Object(nested_object) = nested else {
                panic!("{nested:?}");
            };
            put(cx, nested_object, "0", n(3.0));
            put(cx, value, "nested", nested);
            assert_eq!(
                json(cx, global, &[Value::Object(value), Value::Object(replace)]),
                "{\"keep\":2,\"nested\":[6]}"
            );

            // A replacer array.
            let [b, a, a_object] = ["b", "a", "a"].map(|text| string(cx, text));
            let method = primitive_method(cx, &realm);
            let a_wrapper = wrap(cx, method, a_object);
            let two_wrapper = wrap(cx, method, n(2.0));
            let ignored = new_object(cx, global);
            let list = array(
                cx,
                global,
                &[
                    b,
                    n(1.0),
                    b,
                    Value::Object(a_wrapper),
                    Value::Object(two_wrapper),
                    Value::Boolean(true),
                    Value::Object(ignored),
                    a,
                ],
            );
            let picked = new_object(cx, global);
            for name in ["c", "a", "2", "b", "1"] {
                put(cx, picked, name, n(0.0));
            }
            let all = array(cx, global, &[n(7.0), n(8.0)]);
            put(cx, picked, "b", all);
            assert_eq!(
                json(cx, global, &[Value::Object(picked), list]),
                "{\"b\":[7,8],\"1\":0,\"a\":0,\"2\":0}"
            );

            // Space.
            let spaced = new_object(cx, global);
            let pair = array(cx, global, &[n(1.0), n(2.0)]);
            put(cx, spaced, "a", pair);
            let empty = new_object(cx, global);
            put(cx, spaced, "b", Value::Object(empty));
            let spaced = Value::Object(spaced);
            assert_eq!(
                json(cx, global, &[spaced, Value::Undefined, n(2.0)]),
                "{\n  \"a\": [\n    1,\n    2\n  ],\n  \"b\": {}\n}"
            );
            let dashes = string(cx, "--");
            assert_eq!(
                json(cx, global, &[pair, Value::Null, dashes]),
                "[\n--1,\n--2\n]"
            );
            let ten = " ".repeat(10);
            assert_eq!(
                json(cx, global, &[pair, Value::Null, n(20.0)]),
                format!("[\n{ten}1,\n{ten}2\n]")
            );
            let long = string(cx, "abcdefghijklm");
            assert_eq!(
                json(cx, global, &[pair, Value::Null, long]),
                "[\nabcdefghij1,\nabcdefghij2\n]"
            );
            for none in [n(0.0), n(-1.0), n(0.9), Value::Boolean(true)] {
                assert_eq!(
                    json(cx, global, &[pair, Value::Null, none]),
                    "[1,2]",
                    "{none:?}"
                );
            }
            let three = wrap(cx, method, n(3.0));
            assert_eq!(
                json(cx, global, &[pair, Value::Null, Value::Object(three)]),
                "[\n   1,\n   2\n]"
            );
            let tab = string(cx, "\t");
            let tab_object = wrap(cx, method, tab);
            assert_eq!(
                json(cx, global, &[pair, Value::Null, Value::Object(tab_object)]),
                "[\n\t1,\n\t2\n]"
            );

            let json_object = get_object(cx, global, "JSON");
            let stringify_function = get_object(cx, json_object, "stringify");
            assert_eq!(get(cx, stringify_function, "length"), Value::Number(3.0));
            cx.host().calls.clone()
        })
        .unwrap();
    assert_eq!(
        calls,
        [":true", "keep:true", "drop:true", "nested:true", "0:true"]
    );

    // Re-entry through the replacer is bounded by reentry_depth.
    let mut limits = limits();
    limits.reentry_depth = 8;
    let mut agent = agent_with(limits).unwrap();
    let realm = support::host::realm(&mut agent);
    let global_root = global(&mut agent, &realm);
    agent
        .with_cx(&realm, |cx| {
            let global = object(cx, &global_root);
            let realm_ref = cx.realm_ref(&realm).unwrap();
            let again = cx
                .new_host_function(realm_ref, "reenter", 2, reenter, false, None)
                .unwrap()
                .unwrap();
            let value = new_object(cx, global);
            let json_object = get(cx, global, "JSON");
            put(cx, value, "json", json_object);
            let error = thrown(stringify(
                cx,
                global,
                &[Value::Object(value), Value::Object(again)],
            ));
            assert!(is_error_of(cx, global, error, "RangeError"));
        })
        .unwrap();
}

/// SerializeJSONObject and SerializeJSONArray throw a TypeError for a value already on the
/// stack: an array that holds itself, an object that holds itself, and a cycle through both. A
/// value reached twice but not inside itself is no cycle. After the TypeError the next
/// stringify starts from an empty stack.
#[test]
fn stringify_cycle_throws_type_error() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let global_root = global(&mut agent, &realm);
    agent
        .with_cx(&realm, |cx| {
            let global = object(cx, &global_root);
            let looped = array(cx, global, &[]);
            let Value::Object(looped_object) = looped else {
                panic!("{looped:?}");
            };
            put(cx, looped_object, "0", looped);
            let error = thrown(stringify(cx, global, &[looped]));
            assert!(is_error_of(cx, global, error, "TypeError"));

            let itself = new_object(cx, global);
            put(cx, itself, "self", Value::Object(itself));
            let error = thrown(stringify(cx, global, &[Value::Object(itself)]));
            assert!(is_error_of(cx, global, error, "TypeError"));

            let through = new_object(cx, global);
            let list = array(cx, global, &[Value::Object(through)]);
            put(cx, through, "x", list);
            let error = thrown(stringify(cx, global, &[list]));
            assert!(is_error_of(cx, global, error, "TypeError"));

            let shared = new_object(cx, global);
            let twice = array(cx, global, &[Value::Object(shared), Value::Object(shared)]);
            assert_eq!(json(cx, global, &[twice]), "[{},{}]");
            let holder = new_object(cx, global);
            put(cx, holder, "a", Value::Object(shared));
            put(cx, holder, "b", Value::Object(shared));
            assert_eq!(
                json(cx, global, &[Value::Object(holder)]),
                "{\"a\":{},\"b\":{}}"
            );
        })
        .unwrap();
}

/// `#sec-quotejsonstring` with `#sec-unicodeescape`: a lone leading or trailing surrogate, in a
/// value or in a key, is written as \u and four lowercase hex digits; a surrogate pair is kept
/// as it is; a trailing surrogate before a leading one is two lone ones.
#[test]
fn stringify_lone_surrogates_are_escaped() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let global_root = global(&mut agent, &realm);
    agent
        .with_cx(&realm, |cx| {
            let global = object(cx, &global_root);
            let units = |cx: &mut Cx<'_, TestHost>, value: Value| -> Vec<u16> {
                match stringify(cx, global, &[value]).unwrap() {
                    Value::String(string) => cx.to_utf16(string).unwrap(),
                    other => panic!("{other:?}"),
                }
            };
            let quoted =
                |inner: &str| -> Vec<u16> { format!("\"{inner}\"").encode_utf16().collect() };
            let lone_leading = Value::String(cx.new_string_utf16(&[0xD800]).unwrap());
            assert_eq!(units(cx, lone_leading), quoted("\\ud800"));
            let lone_trailing = Value::String(cx.new_string_utf16(&[0x61, 0xDC00]).unwrap());
            assert_eq!(units(cx, lone_trailing), quoted("a\\udc00"));
            let pair = Value::String(cx.new_string_utf16(&[0xD83D, 0xDE00]).unwrap());
            assert_eq!(units(cx, pair), vec![0x22, 0xD83D, 0xDE00, 0x22]);
            let reversed = Value::String(cx.new_string_utf16(&[0xDE00, 0xD83D]).unwrap());
            assert_eq!(units(cx, reversed), quoted("\\ude00\\ud83d"));
            let holder = new_object(cx, global);
            let key_string = cx.new_string_utf16(&[0xDFFF]).unwrap();
            cx.set(
                holder,
                swell::PropertyKey::String(key_string),
                Value::Number(1.0),
                true,
            )
            .unwrap()
            .unwrap();
            let expected: Vec<u16> = "{\"\\udfff\":1}".encode_utf16().collect();
            assert_eq!(units(cx, Value::Object(holder)), expected);
        })
        .unwrap();
}

/// Set in the environment of the child process a test starts.
const STACK_CHILD: &str = "SWELL_BUILTINS_JSON_STACK_CHILD";

/// The stack of the thread the child runs its body on: 256 KiB, far less than 100,000 levels
/// would take if each level were a Rust call.
const STACK_BYTES: usize = 256 * 1024;

/// Runs `body` on a thread of [`STACK_BYTES`] in a child process that runs only the test
/// `test`. A stack overflow aborts the process, which no test can catch in its own process; in
/// the child it fails this test alone, with the child's output.
fn on_small_stack(test: TestPath, body: fn()) {
    let done = format!("{STACK_CHILD}: {test} done");
    if std::env::var_os(STACK_CHILD).is_some() {
        std::thread::Builder::new()
            .stack_size(STACK_BYTES)
            .spawn(body)
            .unwrap()
            .join()
            .unwrap();
        println!("{done}");
        return;
    }
    let exe = std::env::current_exe().unwrap();
    let out = std::process::Command::new(exe)
        .args(["--exact", test.as_str(), "--nocapture", "--test-threads=1"])
        .env(STACK_CHILD, "1")
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success() && stdout.contains(&done),
        "{test} on a stack of {STACK_BYTES} bytes: {}\n{stdout}\n{}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
}

/// The nesting of the test: 100,000; 2,000 with the feature `collect-every-safe-point`, where
/// each `Array` call collects every array made so far, so the cost grows with the square of the
/// nesting (5,000 took 8.0 s there, so 100,000 would take about 53 minutes). The default mode
/// shows the stack bound; the collect mode shows that the nesting stays rooted across every call.
const DEPTH: usize = if cfg!(feature = "collect-every-safe-point") {
    2_000
} else {
    100_000
};

/// The nesting in this run: [`DEPTH`] in the collect mode and in the lane `slow`
/// (`SWELL_FULL_COUNTS=1`); 25,000 in the default mode's gate (100,000 took 16.2 s there at a
/// load of 100 to 110 on 17 cores, cargo-nextest). 25,000 still shows the stack bound: a Rust
/// call per level takes at least 16 bytes (a return address and a saved frame pointer), 400,000
/// bytes for the nesting, more than the [`STACK_BYTES`] (262,144) the walk runs on.
fn depth() -> usize {
    if cfg!(feature = "collect-every-safe-point")
        || std::env::var("SWELL_FULL_COUNTS").is_ok_and(|value| value == "1")
    {
        DEPTH
    } else {
        25_000
    }
}

/// JSON.stringify walks nested values on an explicit stack (the story's Change; decision 3,
/// "Depth bounds"): 100,000 nested arrays, each made by `Array` through `cx.call`, are
/// stringified on a 256 KiB stack to 100,000 "[" and 100,000 "]" ([`depth`] in this run).
#[test]
fn stringify_of_100000_nested_arrays_needs_no_rust_stack() {
    on_small_stack(
        test_path!("stringify_of_100000_nested_arrays_needs_no_rust_stack"),
        || {
            let depth = depth();
            let mut limits = limits();
            limits.memory_bytes = 512 * 1024 * 1024;
            let mut agent = agent_with(limits).unwrap();
            let realm = realm(&mut agent);
            let global_root = global(&mut agent, &realm);
            let units = agent
                .with_cx(&realm, |cx| {
                    let global = object(cx, &global_root);
                    let mut nested = array(cx, global, &[]);
                    for _ in 1..depth {
                        nested = array(cx, global, &[nested]);
                    }
                    match stringify(cx, global, &[nested]).unwrap() {
                        Value::String(string) => cx.to_utf16(string).unwrap(),
                        other => panic!("{other:?}"),
                    }
                })
                .unwrap();
            let mut expected = vec![u16::from(b'['); depth];
            expected.extend(std::iter::repeat_n(u16::from(b']'), depth));
            assert!(units == expected, "{} code units", units.len());
        },
    );
}

/// `JSON.stringify` builds a String at run time from other values, so it checks
/// `string_length` (`docs/design/host-interface.md`, section 2): with `string_length` 10, a
/// result of 10 code units is made and one of 11 throws a RangeError before it is allocated,
/// for a String and for an array.
#[test]
fn stringify_past_string_length_throws_range_error() {
    let mut limits = limits();
    limits.string_length = 10;
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    let global_root = global(&mut agent, &realm);
    agent
        .with_cx(&realm, |cx| {
            let global = object(cx, &global_root);
            let [fits, over, abc, def] =
                ["abcdefgh", "abcdefghi", "abc", "def"].map(|text| string(cx, text));
            assert_eq!(json(cx, global, &[fits]), "\"abcdefgh\"");
            let error = thrown(stringify(cx, global, &[over]));
            assert!(is_error_of(cx, global, error, "RangeError"));
            let pair = array(cx, global, &[abc, def]);
            let error = thrown(stringify(cx, global, &[pair]));
            assert!(is_error_of(cx, global, error, "RangeError"));
            let single = array(cx, global, &[abc]);
            assert_eq!(json(cx, global, &[single]), "[\"abc\"]");
        })
        .unwrap();
}

/// An item of a replacer list: a String the host makes of the code units, or the String of an
/// item before.
enum Item {
    Units(Vec<u16>),
    Same(usize),
}

/// The text `JSON.stringify(holder, list)` answers in `agent`, with `list` an array of the
/// Strings of `items` (made in an entry before) and `holder` an object whose keys are the
/// items at `holder_keys`, and the steps of that last entry.
fn stringify_with_list(
    agent: &mut swell::Agent<TestHost>,
    items: &[Item],
    holder_keys: &[usize],
) -> (Completion<String>, u64) {
    let realm = realm(agent);
    let global_root = global(agent, &realm);
    let answer = agent
        .with_cx(&realm, |cx| {
            let global = object(cx, &global_root);
            let mut values: Vec<Value> = Vec::new();
            for item in items {
                let value = match item {
                    Item::Units(units) => Value::String(cx.new_string_utf16(units).unwrap()),
                    Item::Same(index) => values[*index],
                };
                values.push(value);
            }
            let holder = new_object(cx, global);
            for &index in holder_keys {
                let Value::String(name) = values[index] else {
                    panic!("{index}");
                };
                cx.set(
                    holder,
                    swell::PropertyKey::String(name),
                    Value::Number(index as f64),
                    true,
                )
                .unwrap()
                .unwrap();
            }
            let list = array(cx, global, &values);
            stringify(cx, global, &[Value::Object(holder), list]).map(|value| text(cx, value))
        })
        .unwrap();
    (answer, agent.last_entry_steps().unwrap())
}

/// `#sec-json.stringify`, step 5.b.iv.7: whether propertyList contains an item is asked of
/// each item listed, and two Strings of the same length above 32 code units are compared at a
/// step per code unit past 32 (`docs/design/engine.md`, "The step budget", "per code unit for
/// string operations"). 200 Strings of 10,000 code units that differ only in their last unit
/// ask for about 19,900 comparisons of 9,968 steps each; with `steps_per_entry` 10,000,000 the
/// entry ends at the step budget.
#[test]
fn a_replacer_list_of_long_strings_is_charged_per_code_unit() {
    let mut limits = limits();
    limits.steps_per_entry = 10_000_000;
    let mut agent = agent_with(limits).unwrap();
    let items: Vec<Item> = (0..200_u16)
        .map(|index| {
            let mut units = vec![0x61; 10_000];
            units[9_999] = 0x100 + index;
            Item::Units(units)
        })
        .collect();
    let (answer, _) = stringify_with_list(&mut agent, &items, &[]);
    assert_eq!(
        answer,
        Err(swell::Abrupt::Terminated(swell::Termination::StepBudget))
    );
}

/// The answer and the steps of [`a_replacer_list_of_short_strings_spends_what_it_spends_today`],
/// recorded on the base of `story:string-builtins-charge-before-they-copy` (`6a89c27`).
const SHORT_LIST_ANSWER: &str = "{\"a\":0,\"bcdefghi\":7,\"vwxyzabcdefghijklmno\":21,\
    \"hijklmno\":33,\"xyzabcdefghijklmnopqrstu\":49}";
const SHORT_LIST_STEPS: u64 = 1_011;

/// A guard of `#sec-json.stringify`, step 5.b.iv.7: items of at most 32 code units are
/// compared at the one step per comparison they cost before the change. 50 items of 1 to 32
/// code units, some the same String twice and some equal Strings made apart: the answer and
/// `last_entry_steps()` equal those recorded on the base.
#[test]
fn a_replacer_list_of_short_strings_spends_what_it_spends_today() {
    let mut agent = agent().unwrap();
    let mut contents: Vec<Vec<u16>> = Vec::new();
    let mut items: Vec<Item> = Vec::new();
    for index in 0..50_usize {
        let units: Vec<u16> = if index % 4 == 3 {
            contents[index - 2].clone()
        } else if index % 6 == 5 {
            contents[index - 4].clone()
        } else {
            let length = 1 + (index * 7) % 32;
            (0..length)
                .map(|unit| 0x61 + ((index + unit) % 26) as u16)
                .collect()
        };
        items.push(if index % 4 == 3 {
            Item::Same(index - 2)
        } else {
            Item::Units(units.clone())
        });
        contents.push(units);
    }
    let (answer, steps) = stringify_with_list(&mut agent, &items, &[0, 7, 21, 33, 49]);
    let answer = answer.unwrap();
    assert_eq!(
        (answer.as_str(), steps),
        (SHORT_LIST_ANSWER, SHORT_LIST_STEPS)
    );
}
