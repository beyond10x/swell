//! Type conversion (ECMA-262 7.1, `#sec-type-conversion`) through the operations of `Cx`:
//! ToNumber of Strings (`#sec-tonumber-applied-to-the-string-type`, StringToNumber
//! `#sec-stringtonumber`), ToString of Numbers (`#sec-tostring`,
//! `#sec-numeric-types-number-tostring`), ToPrimitive (`#sec-toprimitive`,
//! `#sec-ordinarytoprimitive`) and ToObject (`#sec-toobject`).
#![cfg(test)]

mod support;

use support::host::{
    TestHost, agent, data, get, global, is_error_of, key, object, realm, string, text, thrown,
};
use swell::{Abrupt, Completion, Cx, HostCall, PreferredType, Value};

/// `#sec-tonumber-applied-to-the-string-type`: StringToNumber trims every `StrWhiteSpaceChar`
/// (WhiteSpace, `#sec-white-space`, with every code point of General_Category `Zs`, and
/// LineTerminator) from both ends and reads the rest as a `StrNumericLiteral`; anything else
/// is NaN.
#[test]
fn to_number_of_strings() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let mut padding: Vec<u16> = vec![0x09, 0x0B, 0x0C, 0xFEFF, 0x0A, 0x0D, 0x2028, 0x2029];
    let zs: Vec<u16> = (0..=0xFFFF_u32)
        .filter(|&c| swell_unicode::is_space_separator(c))
        .filter_map(|c| u16::try_from(c).ok())
        .collect();
    // Unicode 17.0.0 has 17 Zs code points, all in the BMP (`data/unicode/17.0.0`).
    assert_eq!(zs.len(), 17, "{zs:x?}");
    padding.extend(&zs);

    agent
        .with_cx(&realm, |cx| {
            let mut number = |units: &[u16]| -> f64 {
                let s = cx.new_string_utf16(units).unwrap();
                cx.to_number(Value::String(s)).unwrap().unwrap()
            };
            for &pad in &padding {
                let units: Vec<u16> = [pad, pad]
                    .into_iter()
                    .chain("42".encode_utf16())
                    .chain([pad])
                    .collect();
                assert_eq!(number(&units), 42.0, "padding {pad:#x}");
                // White space inside the literal makes it NaN.
                let inside: Vec<u16> = "4"
                    .encode_utf16()
                    .chain([pad])
                    .chain("2".encode_utf16())
                    .collect();
                assert!(number(&inside).is_nan(), "inside {pad:#x}");
                // Only white space is +0.
                assert_eq!(number(&[pad, pad]), 0.0);
            }
            let utf16 = |text: &str| text.encode_utf16().collect::<Vec<u16>>();
            for (source, expected) in [
                ("", 0.0),
                ("0", 0.0),
                ("  12.5e1 ", 125.0),
                ("-Infinity", f64::NEG_INFINITY),
                ("+Infinity", f64::INFINITY),
                ("0x1F", 31.0),
                ("0X1f", 31.0),
                ("0b101", 5.0),
                ("0o17", 15.0),
                (".5", 0.5),
                ("5.", 5.0),
                ("-0.0", -0.0),
                ("1e1000", f64::INFINITY),
            ] {
                let found = number(&utf16(source));
                assert_eq!(found, expected, "{source:?}");
                assert_eq!(
                    found.is_sign_negative(),
                    expected.is_sign_negative(),
                    "{source:?}"
                );
            }
            for source in [
                "abc",
                "1_000",
                "0x",
                "-0x1",
                "infinity",
                "1e",
                "\u{180E}1",
                "12px",
                "0x1g",
            ] {
                assert!(number(&utf16(source)).is_nan(), "{source:?}");
            }
        })
        .unwrap();
}

/// `#sec-tostring` with `#sec-numeric-types-number-tostring` (radix 10): NaN, the zeros, the
/// infinities, the exponent forms on both sides of the range -6 < n ≤ 21.
#[test]
fn to_string_of_numbers() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    agent
        .with_cx(&realm, |cx| {
            for (number, expected) in [
                (f64::NAN, "NaN"),
                (0.0, "0"),
                (-0.0, "0"),
                (f64::INFINITY, "Infinity"),
                (f64::NEG_INFINITY, "-Infinity"),
                (1.0, "1"),
                (-42.5, "-42.5"),
                (123.456, "123.456"),
                (1e21, "1e+21"),
                (1e20, "100000000000000000000"),
                (0.000001, "0.000001"),
                (1e-7, "1e-7"),
                (-1.5e-9, "-1.5e-9"),
                (0.1 + 0.2, "0.30000000000000004"),
                (5e-324, "5e-324"),
                (1.7976931348623157e308, "1.7976931348623157e+308"),
            ] {
                let s = cx.to_string(Value::Number(number)).unwrap().unwrap();
                assert_eq!(cx.to_rust_string_lossy(s).unwrap(), expected, "{number:e}");
            }
            for (value, expected) in [
                (Value::Undefined, "undefined"),
                (Value::Null, "null"),
                (Value::Boolean(true), "true"),
                (Value::Boolean(false), "false"),
            ] {
                let s = cx.to_string(value).unwrap().unwrap();
                assert_eq!(cx.to_rust_string_lossy(s).unwrap(), expected);
            }
            let s = cx.new_string("same").unwrap();
            assert_eq!(cx.to_string(Value::String(s)).unwrap(), Ok(s));
        })
        .unwrap();
}

/// `valueOf` of the test objects: records its call and answers the number 1, or an object when
/// the object's `answer` property says so.
fn value_of(cx: &mut Cx<'_, TestHost>, call: HostCall<'_>) -> Completion<Value> {
    cx.host_mut().calls.push("valueOf".to_string());
    answer(cx, call.this, Value::Number(1.0))
}

/// `toString` of the test objects: records its call and answers the String "s", or an object
/// when the object's `answer` property says so.
fn to_string(cx: &mut Cx<'_, TestHost>, call: HostCall<'_>) -> Completion<Value> {
    cx.host_mut().calls.push("toString".to_string());
    let s = cx.new_string("s")?;
    answer(cx, call.this, Value::String(s))
}

fn answer(cx: &mut Cx<'_, TestHost>, this: Value, primitive: Value) -> Completion<Value> {
    let Value::Object(this) = this else {
        return Ok(primitive);
    };
    let k = cx.new_string("objects")?;
    match cx
        .get_property(this, swell::PropertyKey::String(k))
        .unwrap()?
    {
        Value::Boolean(true) => Ok(Value::Object(this)),
        _ => Ok(primitive),
    }
}

/// `#sec-toprimitive` and `#sec-ordinarytoprimitive`: without a hint and with the hint number,
/// `valueOf` is called first and `toString` only when `valueOf` answers an object; with the
/// hint string, the other order; when both answer objects, a `TypeError`.
#[test]
fn to_primitive_calls_valueof_and_tostring_in_hint_order() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let global_root = global(&mut agent, &realm);
    agent
        .with_cx(&realm, |cx| {
            let realm_ref = cx.realm_ref(&realm).unwrap();
            let global = object(cx, &global_root);
            let make = |cx: &mut Cx<'_, TestHost>, objects: bool| {
                let object = cx.to_object(Value::Boolean(true)).unwrap().unwrap();
                let v = cx
                    .new_host_function(realm_ref, "valueOf", 0, value_of, false, None)
                    .unwrap()
                    .unwrap();
                let t = cx
                    .new_host_function(realm_ref, "toString", 0, to_string, false, None)
                    .unwrap()
                    .unwrap();
                for (name, value) in [
                    ("valueOf", Value::Object(v)),
                    ("toString", Value::Object(t)),
                    ("objects", Value::Boolean(objects)),
                ] {
                    let k = key(cx, name);
                    cx.define_property_or_throw(object, k, data(value, true, false, true))
                        .unwrap()
                        .unwrap();
                }
                object
            };

            let plain = make(cx, false);
            let take = |cx: &mut Cx<'_, TestHost>| std::mem::take(&mut cx.host_mut().calls);
            assert_eq!(
                cx.to_primitive(Value::Object(plain), None).unwrap(),
                Ok(Value::Number(1.0))
            );
            assert_eq!(take(cx), ["valueOf"]);
            assert_eq!(
                cx.to_primitive(Value::Object(plain), Some(PreferredType::Number))
                    .unwrap(),
                Ok(Value::Number(1.0))
            );
            assert_eq!(take(cx), ["valueOf"]);
            let s = cx
                .to_primitive(Value::Object(plain), Some(PreferredType::String))
                .unwrap()
                .unwrap();
            assert_eq!(text(cx, s), "s");
            assert_eq!(take(cx), ["toString"]);
            // ToNumber and ToString go through ToPrimitive with their hints.
            assert_eq!(cx.to_number(Value::Object(plain)).unwrap(), Ok(1.0));
            assert_eq!(take(cx), ["valueOf"]);
            let s = cx.to_string(Value::Object(plain)).unwrap().unwrap();
            assert_eq!(cx.to_rust_string_lossy(s).unwrap(), "s");
            assert_eq!(take(cx), ["toString"]);

            // Both answer objects: each is called once, then a TypeError.
            let stubborn = make(cx, true);
            let error = thrown(cx.to_primitive(Value::Object(stubborn), None).unwrap());
            assert!(is_error_of(cx, global, error, "TypeError"));
            assert_eq!(take(cx), ["valueOf", "toString"]);
            let error = thrown(
                cx.to_primitive(Value::Object(stubborn), Some(PreferredType::String))
                    .unwrap(),
            );
            assert!(is_error_of(cx, global, error, "TypeError"));
            assert_eq!(take(cx), ["toString", "valueOf"]);

            // A primitive is its own ToPrimitive, and no method runs.
            let s = string(cx, "x");
            assert_eq!(cx.to_primitive(s, None).unwrap(), Ok(s));
            assert!(take(cx).is_empty());

            // An object with neither method throws a TypeError.
            let bare = cx.to_object(Value::Number(3.0)).unwrap().unwrap();
            let error = thrown(cx.to_primitive(Value::Object(bare), None).unwrap());
            assert!(is_error_of(cx, global, error, "TypeError"));
        })
        .unwrap();
}

/// `#sec-toobject`: undefined and null throw a `TypeError`; a Boolean, a Number and a String
/// give a new wrapper object each time (a String one a String exotic object with its
/// `length` and its indices); an object is its own ToObject.
#[test]
fn to_object_of_primitives() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let global_root = global(&mut agent, &realm);
    agent
        .with_cx(&realm, |cx| {
            let global = object(cx, &global_root);
            for value in [Value::Undefined, Value::Null] {
                let error = thrown(cx.to_object(value).unwrap());
                assert!(is_error_of(cx, global, error, "TypeError"));
            }
            for value in [Value::Boolean(false), Value::Number(-0.0)] {
                let first = cx.to_object(value).unwrap().unwrap();
                let second = cx.to_object(value).unwrap().unwrap();
                assert_ne!(first, second);
                assert_eq!(cx.to_object(Value::Object(first)).unwrap(), Ok(first));
                assert_eq!(cx.brand(first), Ok(None));
            }
            let s = string(cx, "ab");
            let wrapper = cx.to_object(s).unwrap().unwrap();
            assert_eq!(get(cx, wrapper, "length"), Value::Number(2.0));
            let first = get(cx, wrapper, "0");
            assert_eq!(text(cx, first), "a");
            let second = get(cx, wrapper, "1");
            assert_eq!(text(cx, second), "b");
            assert_eq!(get(cx, wrapper, "2"), Value::Undefined);
            let k = key(cx, "1");
            assert_eq!(cx.has_property(wrapper, k).unwrap(), Ok(true));
            assert!(matches!(
                cx.delete_property_or_throw(wrapper, k).unwrap(),
                Err(Abrupt::Throw(_))
            ));
        })
        .unwrap();
}
