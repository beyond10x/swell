//! Type conversion (ECMA-262 7.1, `#sec-type-conversion`) through the operations of `Cx`:
//! ToNumber of Strings (`#sec-tonumber-applied-to-the-string-type`, StringToNumber
//! `#sec-stringtonumber`), ToString of Numbers (`#sec-tostring`,
//! `#sec-numeric-types-number-tostring`), ToPrimitive (`#sec-toprimitive`,
//! `#sec-ordinarytoprimitive`) and ToObject (`#sec-toobject`). And the step charge of a
//! comparison of two Strings (`docs/design/engine.md`, "The step budget": a step per code unit
//! past 32, charged before the units are read; HOST-181) with its answers. Run also with the
//! feature `collect-every-safe-point`.
#![cfg(test)]

mod support;

use support::host::{
    TestHost, agent, agent_with, data, get, global, is_error_of, key, limits, object, realm,
    string, text, thrown,
};
use support::script::{Outcome, describe, eval_text, parse};
use swell::{
    Abrupt, Agent, Brand, Completion, Cx, HostCall, HostObjectSpec, Limits, PreferredType,
    Prototype, Realm, Termination, Value,
};

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

            // An object with neither method throws a TypeError: one whose [[Prototype]] is null,
            // since every ordinary object inherits `Object.prototype.toString` (20.1.3.6).
            let bare = cx
                .new_host_object(HostObjectSpec {
                    prototype: Prototype::Null,
                    brand: Brand(1),
                    data: None,
                })
                .unwrap()
                .unwrap();
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

/// The code units of the long Strings of HOST-181.
const LONG: u64 = 1_000_000;

/// The code units a comparison of two Strings reads without a step of its own
/// (`docs/design/engine.md`, "The step budget").
const FREE: u64 = 32;

/// An agent with `limits` and a realm whose global object has a String property per entry of
/// `strings`, made from code units through the host interface.
fn agent_with_strings(limits: Limits, strings: &[(String, Vec<u16>)]) -> (Agent<TestHost>, Realm) {
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    let global_root = global(&mut agent, &realm);
    agent
        .with_cx(&realm, |cx| {
            let global = object(cx, &global_root);
            for (name, units) in strings {
                let value = Value::String(cx.new_string_utf16(units).unwrap());
                let name = key(cx, name);
                cx.set(global, name, value, true).unwrap().unwrap();
            }
        })
        .unwrap();
    (agent, realm)
}

/// The outcome of the script `source` in `realm` and the steps its entry spent, read before
/// the outcome is described (a Number is described through `Cx::to_string`, an entry of its
/// own).
fn counted(agent: &mut Agent<TestHost>, realm: &Realm, source: &str) -> (Outcome, u64) {
    let script = parse(agent, realm, source);
    let outcome = agent.evaluate_script(&script).unwrap();
    let steps = agent.last_entry_steps().unwrap();
    (describe(agent, realm, outcome), steps)
}

/// `a` and `b`, Strings of [`LONG`] code units that differ only in the last, and `x` and `y`,
/// Strings of one code unit.
fn long_and_short_strings() -> Vec<(String, Vec<u16>)> {
    let a = vec![0x61; LONG as usize];
    let mut b = a.clone();
    *b.last_mut().unwrap() = 0x62;
    vec![
        ("a".to_string(), a),
        ("b".to_string(), b),
        ("x".to_string(), vec![0x78]),
        ("y".to_string(), vec![0x79]),
    ]
}

/// HOST-181: with `a` and `b` two Strings of 1,000,000 code units that differ only in the last,
/// defined as globals by the host, `a === b` spends at least 999,968 steps more than the same
/// script over the one-unit Strings `x` and `y`, and so do `a == b` and `[a].indexOf(b)`
/// (IsStrictlyEqual, `#sec-isstrictlyequal`, a step per code unit past 32); `a < b` charges
/// the shorter length, 1,000,000 steps, where `x < y` charges 1 (IsLessThan,
/// `#sec-islessthan`). With `steps_per_entry` 10,000 each of the four scripts ends in
/// `Terminated(StepBudget)`.
#[test]
fn host_181_comparing_long_strings_is_charged() {
    let cases = [
        ("a === b", "x === y", "false", LONG - FREE),
        ("a < b", "x < y", "true", LONG - 1),
        ("a == b", "x == y", "false", LONG - FREE),
        ("[a].indexOf(b)", "[x].indexOf(y)", "-1", LONG - FREE),
    ];
    let mut roomy = limits();
    roomy.steps_per_entry = 10_000_000;
    let (mut agent, realm) = agent_with_strings(roomy, &long_and_short_strings());
    for (long, short, answer, extra) in cases {
        let answer = Outcome::Normal(answer.to_string());
        let (outcome, short_steps) = counted(&mut agent, &realm, short);
        assert_eq!(outcome, answer, "{short}");
        let (outcome, long_steps) = counted(&mut agent, &realm, long);
        assert_eq!(outcome, answer, "{long}");
        assert!(
            long_steps >= short_steps + extra,
            "{long} spent {long_steps} steps, {short} {short_steps}: fewer than {extra} more"
        );
    }

    let mut tight = limits();
    tight.steps_per_entry = 10_000;
    let (mut agent, realm) = agent_with_strings(tight, &long_and_short_strings());
    for (long, ..) in cases {
        assert_eq!(
            eval_text(&mut agent, &realm, long),
            Outcome::Terminated(Termination::StepBudget),
            "{long}"
        );
    }
}

/// 40 pairs of Strings of 0 to 32 code units: equal, a prefix, apart in the last or in the
/// first unit, or the same text; Latin-1 on both sides, or a unit above 0xFF on one side or
/// on both.
fn short_pairs() -> Vec<(Vec<u16>, Vec<u16>)> {
    (0..40usize)
        .map(|i| {
            let len = i * 7 % 33;
            let text: Vec<u16> = (0..len)
                .map(|j| 0x61 + u16::try_from(j % 26).unwrap())
                .collect();
            let widened = |units: &[u16]| {
                let mut units = units.to_vec();
                if let Some(unit) = units.get_mut(len / 2) {
                    *unit += 0x300;
                }
                units
            };
            let a = if i % 4 >= 2 {
                widened(&text)
            } else {
                text.clone()
            };
            let mut b = match i % 5 {
                0 => a.clone(),
                1 => a[..len.saturating_sub(1)].to_vec(),
                2 => {
                    let mut b = a.clone();
                    if let Some(unit) = b.last_mut() {
                        *unit += 1;
                    }
                    b
                }
                3 => {
                    let mut b = a.clone();
                    if let Some(unit) = b.first_mut() {
                        *unit -= 1;
                    }
                    b
                }
                _ => text.clone(),
            };
            if i % 4 == 1 {
                b = widened(&b);
            }
            (a, b)
        })
        .collect()
}

/// The steps of each script of [`comparisons_of_short_strings_spend_what_they_spend_today`],
/// in the order of its operators, measured on the base of
/// `story:string-comparison-and-property-keys-charge-per-code-unit` (`wave/budget-1` at
/// `cc41ae6`), the same in both collection modes.
const SHORT_STEPS: [u64; 6] = [163, 163, 764, 355, 323, 1_524];

/// `docs/design/engine.md`, "The step budget": a comparison of Strings of up to 32 code units
/// costs what it did before comparisons were charged per code unit past 32. Over the 40
/// [`short_pairs`], defined as globals by the host, a script per operator (`===`, `==`, `<`,
/// a `switch` case, `Object.is` and the default compare of `sort`) answers what the code units
/// of the pairs give and spends the steps it spent on the base ([`SHORT_STEPS`]). Each script
/// is straight-line code, one term per pair: the feature `collect-every-safe-point` collects
/// before every instruction.
#[test]
fn comparisons_of_short_strings_spend_what_they_spend_today() {
    let pairs = short_pairs();
    assert!(
        pairs
            .iter()
            .all(|(a, b)| a.len() <= FREE as usize && b.len() <= FREE as usize)
    );
    assert!(pairs.iter().any(|(a, _)| a.len() == FREE as usize));
    assert!(pairs.iter().any(|(a, _)| a.is_empty()));
    let equal = pairs.iter().filter(|(a, b)| a == b).count();
    let less = pairs.iter().filter(|(a, b)| a < b).count();
    let sorted_first: usize = pairs.iter().map(|(a, b)| a.min(b).len()).sum();
    let terms = |term: &dyn Fn(usize) -> String, join: &str| {
        (0..pairs.len())
            .map(term)
            .collect::<Vec<String>>()
            .join(join)
    };
    let operators = [
        ("===", terms(&|i| format!("(a{i} === b{i})"), " + "), equal),
        ("==", terms(&|i| format!("(a{i} == b{i})"), " + "), equal),
        ("<", terms(&|i| format!("(a{i} < b{i})"), " + "), less),
        (
            "switch",
            format!(
                "var n = 0; {} n",
                terms(&|i| format!("switch (a{i}) {{ case b{i}: n++; }}"), " ")
            ),
            equal,
        ),
        (
            "Object.is",
            terms(&|i| format!("Object.is(a{i}, b{i})"), " + "),
            equal,
        ),
        (
            "sort",
            terms(&|i| format!("[a{i}, b{i}].sort()[0].length"), " + "),
            sorted_first,
        ),
    ];
    let mut strings = Vec::new();
    for (i, (a, b)) in pairs.iter().enumerate() {
        strings.push((format!("a{i}"), a.clone()));
        strings.push((format!("b{i}"), b.clone()));
    }
    let mut steps = Vec::new();
    for (name, script, answer) in operators {
        let (mut agent, realm) = agent_with_strings(limits(), &strings);
        let (outcome, spent) = counted(&mut agent, &realm, &script);
        assert_eq!(outcome, Outcome::Normal(answer.to_string()), "{name}");
        steps.push(spent);
    }
    assert_eq!(
        steps, SHORT_STEPS,
        "the steps of ===, ==, <, switch, Object.is, sort"
    );
}

/// A generator of the tests, seeded in the test: xorshift64.
struct Generator(u64);

impl Generator {
    fn below(&mut self, bound: usize) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 % bound as u64) as usize
    }
}

/// The pairs of [`equality_answers_are_unchanged`]: `count` pairs of Strings of 0 to 80 code
/// units over a small alphabet, so that long prefixes repeat; in turn equal, a prefix, one unit
/// apart, and the same text with one unit above 0xFF on one side (one record Latin-1, the other
/// UTF-16); either side may come first, and an equal pair may hold a unit above 0xFF on both.
fn generated_pairs(count: usize) -> Vec<(Vec<u16>, Vec<u16>)> {
    let mut generator = Generator(0x5157_454c_4c31_3831);
    (0..count)
        .map(|i| {
            let len = generator.below(81);
            let mut a: Vec<u16> = (0..len)
                .map(|_| 0x61 + u16::try_from(generator.below(3)).unwrap())
                .collect();
            if i % 8 == 4 && len > 0 {
                let at = generator.below(len);
                a[at] = 0x3B1;
            }
            let mut b = a.clone();
            match i % 4 {
                0 => {}
                1 => b.truncate(generator.below(len + 1)),
                2 if len > 0 => {
                    let at = generator.below(len);
                    b[at] = if generator.below(2) == 0 {
                        b[at] + 1
                    } else {
                        b[at] - 1
                    };
                }
                3 if len > 0 => {
                    let at = generator.below(len);
                    b[at] += 0x100;
                }
                _ => {}
            }
            if generator.below(2) == 0 {
                (b, a)
            } else {
                (a, b)
            }
        })
        .collect()
}

/// IsStrictlyEqual (`#sec-isstrictlyequal`) and IsLessThan (`#sec-islessthan`) of two Strings
/// answer what their code units give: for 2,000 [`generated_pairs`], stored by the host in the
/// Arrays `A` and `B`, every answer of `A[i] === B[i]` and `A[i] < B[i]` equals `==`,
/// respectively `<`, of the two vectors `to_utf16` reads. With the feature
/// `collect-every-safe-point` the first 100 pairs: that mode collects before every
/// instruction, and 2,000 pairs ran over 60 s there (measured on the base, `cc41ae6`, against
/// 1.0 s in the default mode).
#[test]
fn equality_answers_are_unchanged() {
    let count = if cfg!(feature = "collect-every-safe-point") {
        100
    } else {
        2_000
    };
    let pairs = generated_pairs(count);
    let mut agent = agent_with(limits()).unwrap();
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(&mut agent, &realm, "var A = [], B = []; A.length"),
        Outcome::Normal("0".to_string())
    );
    let global_root = global(&mut agent, &realm);
    let expected: String = agent
        .with_cx(&realm, |cx| {
            let global = object(cx, &global_root);
            let arrays = [get(cx, global, "A"), get(cx, global, "B")];
            let mut expected = String::new();
            for (i, (a, b)) in pairs.iter().enumerate() {
                let mut read = Vec::new();
                for (array, units) in arrays.iter().zip([a, b]) {
                    let Value::Object(array) = *array else {
                        panic!("{array:?} is no Array");
                    };
                    let string = cx.new_string_utf16(units).unwrap();
                    read.push(cx.to_utf16(string).unwrap());
                    let index = key(cx, &i.to_string());
                    cx.set(array, index, Value::String(string), true)
                        .unwrap()
                        .unwrap();
                }
                let answer = u8::from(read[0] == read[1]) + 2 * u8::from(read[0] < read[1]);
                expected.push(char::from(b'0' + answer));
            }
            expected
        })
        .unwrap();
    assert!(expected.contains('1') && expected.contains('2') && expected.contains('0'));
    let script = "var r = [];
        for (var i = 0; i < A.length; i++) r.push((A[i] === B[i] ? 1 : 0) + (A[i] < B[i] ? 2 : 0));
        r.join('')";
    assert_eq!(
        eval_text(&mut agent, &realm, script),
        Outcome::Normal(format!("\"{expected}\""))
    );
}
