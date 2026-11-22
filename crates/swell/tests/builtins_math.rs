//! The Math object (ECMA-262 21.3, `#sec-math-object`): `Math.max`
//! (`#sec-math.max`) and `Math[@@toStringTag]` (`#sec-math-%symbol.tostringtag%`). No script
//! runs.
#![cfg(test)]

use crate::support;

use support::host::{
    TestHost, agent, get, get_object, global, is_error_of, key, object, realm, string, text, thrown,
};
use swell::{Completion, Cx, HostCall, Value};

/// A valueOf that records its call and answers 1.
fn value_of(cx: &mut Cx<'_, TestHost>, _call: HostCall<'_>) -> Completion<Value> {
    cx.host_mut().calls.push("valueOf".to_string());
    Ok(Value::Number(1.0))
}

/// `#sec-math.max`: every argument goes through ToNumber first, in order, even after a NaN;
/// any NaN gives NaN; +0 is larger than -0 whatever their order; no argument gives -∞; a
/// String argument is converted. `Math.max` has length 2; `Math` is an ordinary object, not a
/// function, whose @@toStringTag is "Math".
#[test]
fn max_of_nan_and_signed_zeros() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let global_root = global(&mut agent, &realm);
    let calls = agent
        .with_cx(&realm, |cx| {
            let global = object(cx, &global_root);
            let math = get_object(cx, global, "Math");
            let max = get(cx, math, "max");
            let n = Value::Number;
            let of = |cx: &mut Cx<'_, TestHost>, args: &[Value]| -> f64 {
                match cx.call(max, Value::Object(math), args).unwrap().unwrap() {
                    Value::Number(number) => number,
                    other => panic!("{other:?}"),
                }
            };
            assert_eq!(of(cx, &[]), f64::NEG_INFINITY);
            let zero = of(cx, &[n(-0.0), n(0.0)]);
            assert!(zero == 0.0 && zero.is_sign_positive());
            let zero = of(cx, &[n(0.0), n(-0.0)]);
            assert!(zero == 0.0 && zero.is_sign_positive());
            let zero = of(cx, &[n(-0.0), n(-0.0)]);
            assert!(zero == 0.0 && zero.is_sign_negative());
            let zero = of(cx, &[n(-0.0)]);
            assert!(zero == 0.0 && zero.is_sign_negative());
            assert!(of(cx, &[n(1.0), n(f64::NAN), n(3.0)]).is_nan());
            assert!(of(cx, &[n(f64::NAN)]).is_nan());
            assert!(of(cx, &[Value::Undefined]).is_nan());
            assert_eq!(of(cx, &[n(-5.0), n(f64::INFINITY), n(2.0)]), f64::INFINITY);
            let three = string(cx, "3");
            assert_eq!(of(cx, &[three, n(2.0)]), 3.0);
            assert_eq!(of(cx, &[Value::Null, n(-1.0)]), 0.0);

            // ToNumber of every argument, the one after a NaN included.
            let realm_ref = cx.realm_ref(&realm).unwrap();
            let recorder = cx
                .new_host_function(realm_ref, "valueOf", 0, value_of, false, None)
                .unwrap()
                .unwrap();
            let object_constructor = get(cx, global, "Object");
            let Value::Object(valued) = cx
                .call(object_constructor, Value::Undefined, &[])
                .unwrap()
                .unwrap()
            else {
                panic!("Object() made no object");
            };
            let value_of_key = key(cx, "valueOf");
            cx.set(valued, value_of_key, Value::Object(recorder), true)
                .unwrap()
                .unwrap();
            assert!(of(cx, &[n(f64::NAN), Value::Object(valued)]).is_nan());
            assert_eq!(of(cx, &[Value::Object(valued), n(0.5)]), 1.0);

            let Value::Object(max_object) = max else {
                panic!("{max:?}");
            };
            assert_eq!(get(cx, max_object, "length"), Value::Number(2.0));
            let name = get(cx, max_object, "name");
            assert_eq!(text(cx, name), "max");
            let error = thrown(cx.call(Value::Object(math), Value::Undefined, &[]).unwrap());
            assert!(is_error_of(cx, global, error, "TypeError"));
            let Value::Object(object_constructor) = object_constructor else {
                unreachable!()
            };
            let object_prototype = get_object(cx, object_constructor, "prototype");
            let to_string = get(cx, object_prototype, "toString");
            let tag = cx
                .call(to_string, Value::Object(math), &[])
                .unwrap()
                .unwrap();
            assert_eq!(text(cx, tag), "[object Math]");
            let get_prototype_of = get(cx, object_constructor, "getPrototypeOf");
            assert_eq!(
                cx.call(get_prototype_of, Value::Undefined, &[Value::Object(math)])
                    .unwrap()
                    .unwrap(),
                Value::Object(object_prototype)
            );
            cx.host().calls.clone()
        })
        .unwrap();
    assert_eq!(calls, ["valueOf", "valueOf"]);
}
