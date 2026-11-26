//! The Math object (ECMA-262 21.3, `#sec-math-object`): its value properties
//! (`#sec-value-properties-of-the-math-object`), `Math[@@toStringTag]`
//! (`#sec-math-%symbol.tostringtag%`), `Math.max` (`#sec-math.max`) and the functions whose
//! answer IEEE 754 or integer arithmetic makes exact: `abs`, `ceil`, `clz32`, `floor`,
//! `fround`, `imul`, `min`, `round`, `sign`, `sqrt` and `trunc`; and the 22 functions whose
//! answer IEEE 754 does not fix, `acos` to `tanh`, `hypot` and `pow`, which answer what
//! `swell_num::math` answers. The tables below give the order in which `%Math%` defines its own
//! properties; a story that adds a member adds its row at its place.
#![cfg(test)]

use crate::support;

use std::cell::Cell;

use support::host::{
    TestHost, agent, agent_with, get, get_object, global, is_error_of, key, limits, object, realm,
    string, text, thrown,
};
use support::script::{Outcome as Shown, define_function, eval, eval_text};
use swell::{
    Abrupt, Agent, AgentState, Completion, Cx, HostCall, InterruptHandle, ObjectRef, Outcome,
    Realm, Termination, Value,
};

/// The value properties of `Math` (21.3.1.1 to 21.3.1.8), in the order `%Math%` defines them,
/// each with the digits the standard gives for it.
const VALUE_PROPERTIES: [(&str, &str); 8] = [
    ("E", "2.7182818284590452354"),
    ("LN10", "2.302585092994046"),
    ("LN2", "0.6931471805599453"),
    ("LOG10E", "0.4342944819032518"),
    ("LOG2E", "1.4426950408889634"),
    ("PI", "3.1415926535897932"),
    ("SQRT1_2", "0.7071067811865476"),
    ("SQRT2", "1.4142135623730951"),
];

/// The functions of `Math` after `max` and the value properties, in the order `%Math%` defines
/// them, each with the "length" the standard gives it.
const FUNCTIONS: [(&str, u32); 11] = [
    ("abs", 1),
    ("ceil", 1),
    ("clz32", 1),
    ("floor", 1),
    ("fround", 1),
    ("imul", 2),
    ("min", 2),
    ("round", 1),
    ("sign", 1),
    ("sqrt", 1),
    ("trunc", 1),
];

/// The functions of `Math` whose answer IEEE 754 does not fix, after those of `FUNCTIONS`, in
/// the order `%Math%` defines them, each with the "length" the standard gives it (21.3.2.8,
/// 21.3.2.19 and 21.3.2.27: 2 for `atan2`, `hypot` and `pow`).
const APPROXIMATED: [(&str, u32); 22] = [
    ("acos", 1),
    ("acosh", 1),
    ("asin", 1),
    ("asinh", 1),
    ("atan", 1),
    ("atan2", 2),
    ("atanh", 1),
    ("cbrt", 1),
    ("cos", 1),
    ("cosh", 1),
    ("exp", 1),
    ("expm1", 1),
    ("hypot", 2),
    ("log", 1),
    ("log10", 1),
    ("log1p", 1),
    ("log2", 1),
    ("pow", 2),
    ("sin", 1),
    ("sinh", 1),
    ("tan", 1),
    ("tanh", 1),
];

/// The functions of `Math` after those of `APPROXIMATED`, in the order `%Math%` defines them:
/// those whose argument is not a Number, which the tables of this file do not call
/// (`Math.sumPrecise`, `story:math-sum-precise`, held by `tests/math_sum_precise.rs`; and
/// `Math.random`, `story:math-random`, held by `tests/math_random.rs`).
const LATER_FUNCTIONS: [&str; 2] = ["sumPrecise", "random"];

/// A valueOf that records its call and answers 1.
fn value_of(cx: &mut Cx<'_, TestHost>, _call: HostCall<'_>) -> Completion<Value> {
    cx.host_mut().calls.push("valueOf".to_string());
    Ok(Value::Number(1.0))
}

/// `steps` with the `Math` object of a fresh agent's realm.
fn with_math<T>(steps: impl FnOnce(&mut Cx<'_, TestHost>, ObjectRef, ObjectRef) -> T) -> T {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let global_root = global(&mut agent, &realm);
    agent
        .with_cx(&realm, |cx| {
            let global = object(cx, &global_root);
            let math = get_object(cx, global, "Math");
            steps(cx, global, math)
        })
        .unwrap()
}

/// `Math[name](...args)` with `Math` as `this`, which must answer a Number.
fn call(cx: &mut Cx<'_, TestHost>, math: ObjectRef, name: &str, args: &[Value]) -> f64 {
    let function = get(cx, math, name);
    match cx.call(function, Value::Object(math), args).unwrap() {
        Ok(Value::Number(number)) => number,
        other => panic!("Math.{name}({args:?}) answered {other:?}"),
    }
}

/// `got` is `want` to the bit, or both are NaN (a NaN's payload is not observable).
fn assert_same(got: f64, want: f64, what: &str) {
    if want.is_nan() {
        assert!(got.is_nan(), "{what}: {got:?}, not NaN");
    } else {
        assert_eq!(
            got.to_bits(),
            want.to_bits(),
            "{what}: {got:?} where {want:?}"
        );
    }
}

/// A unary function of `Math` over a table of (input, answer) pairs, and with no argument.
fn unary_table(name: &str, absent: f64, table: &[(f64, f64)]) {
    with_math(|cx, _, math| {
        assert_same(call(cx, math, name, &[]), absent, &format!("{name}()"));
        for &(input, answer) in table {
            let got = call(cx, math, name, &[Value::Number(input)]);
            assert_same(got, answer, &format!("{name}({input:?})"));
        }
    });
}

/// The Number of the script `source`, which must complete normally with a Number.
fn number_of(agent: &mut Agent<TestHost>, realm: &Realm, source: &str) -> f64 {
    let Outcome::Normal(root) = eval(agent, realm, source) else {
        panic!("{source} did not complete normally");
    };
    match agent.with_cx(realm, |cx| cx.get(&root).unwrap()).unwrap() {
        Value::Number(number) => number,
        other => panic!("{source} answered {other:?}"),
    }
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

/// `#sec-value-properties-of-the-math-object`: each constant, read by a script, is the Number
/// the digits of the standard name (which is also `std`'s constant of the same value), and is
/// { [[Writable]]: false, [[Enumerable]]: false, [[Configurable]]: false }.
#[test]
fn the_constants_are_the_numbers_of_the_standard_and_read_only() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let oracles = [
        std::f64::consts::E,
        std::f64::consts::LN_10,
        std::f64::consts::LN_2,
        std::f64::consts::LOG10_E,
        std::f64::consts::LOG2_E,
        std::f64::consts::PI,
        std::f64::consts::FRAC_1_SQRT_2,
        std::f64::consts::SQRT_2,
    ];
    for ((name, digits), oracle) in VALUE_PROPERTIES.into_iter().zip(oracles) {
        let standard: f64 = digits.parse().unwrap();
        assert_same(standard, oracle, name);
        let got = number_of(&mut agent, &realm, &format!("Math.{name}"));
        assert_same(got, standard, name);
        let source = format!(
            "var d = Object.getOwnPropertyDescriptor(Math, \"{name}\"); \
             [typeof d.value, d.writable, d.enumerable, d.configurable, 'get' in d].join()"
        );
        assert_eq!(
            eval_text(&mut agent, &realm, &source),
            Shown::Normal("\"number,false,false,false,false\"".to_string()),
            "{name}"
        );
        let source = format!("\"use strict\"; Math.{name} = 1;");
        assert_eq!(
            eval_text(&mut agent, &realm, &source),
            Shown::Throw("TypeError".to_string()),
            "{name}"
        );
    }
}

/// `#sec-function-properties-of-the-math-object`: each function has the "length" and "name"
/// the standard gives it and is { [[Writable]]: true, [[Enumerable]]: false,
/// [[Configurable]]: true } (clause 18), and none is a constructor.
#[test]
fn the_functions_have_their_lengths_and_names() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    for (name, length) in FUNCTIONS.into_iter().chain(APPROXIMATED) {
        let source = format!(
            "var d = Object.getOwnPropertyDescriptor(Math, \"{name}\"); \
             [typeof d.value, d.writable, d.enumerable, d.configurable, \
              d.value.length, d.value.name].join()"
        );
        assert_eq!(
            eval_text(&mut agent, &realm, &source),
            Shown::Normal(format!("\"function,true,false,true,{length},{name}\"")),
            "{name}"
        );
        let source = format!("new Math.{name}(1, 2);");
        assert_eq!(
            eval_text(&mut agent, &realm, &source),
            Shown::Throw("TypeError".to_string()),
            "{name}"
        );
    }
}

/// `Math` has exactly the own keys of the tables of this module, in the order of definition
/// (OrdinaryOwnPropertyKeys, `#sec-ordinaryownpropertykeys`: Strings in the order of creation,
/// then Symbols): `max`, the value properties, the functions, then @@toStringTag.
#[test]
fn math_has_exactly_the_own_keys_of_its_tables_in_order() {
    let names: Vec<&str> = std::iter::once("max")
        .chain(VALUE_PROPERTIES.iter().map(|&(name, _)| name))
        .chain(FUNCTIONS.iter().map(|&(name, _)| name))
        .chain(APPROXIMATED.iter().map(|&(name, _)| name))
        .chain(LATER_FUNCTIONS)
        .collect();
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "Object.getOwnPropertyNames(Math).join()"
        ),
        Shown::Normal(format!("\"{}\"", names.join(",")))
    );
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var s = Object.getOwnPropertySymbols(Math); \
             s.length === 1 && s[0] === Symbol.toStringTag"
        ),
        Shown::Normal("true".to_string())
    );
}

/// The calls of `Math` that the first consumer's harness makes (`resources/testharness.js`
/// lines 1708, 1709, 1710, 1783, 1812 and 3018, `html/syntax/parsing/resources/common.js` line
/// 5), with the values the script gives their arguments.
#[test]
fn consumer_calls_of_the_harness_answer_as_the_standard_says() {
    let n = Value::Number;
    with_math(|cx, _, math| {
        // Math.floor(max_array_length / 2)
        assert_same(call(cx, math, "floor", &[n(7.0 / 2.0)]), 3.0, "floor");
        // Math.min(length_after_offset + offset, arr.length)
        assert_same(call(cx, math, "min", &[n(3.0 + 1.0), n(2.0)]), 2.0, "min");
        // Math.max(upper_bound - max_array_length, 0)
        assert_same(call(cx, math, "max", &[n(2.0 - 5.0), n(0.0)]), 0.0, "max");
        // Math.abs(actual[i] - expected[i])
        assert_same(call(cx, math, "abs", &[n(1.0 - 4.5)]), 3.5, "abs");
        // Math.abs(actual - expected)
        assert_same(call(cx, math, "abs", &[n(0.25 - 0.75)]), 0.5, "abs");
        // Math.ceil(timeout_full / interval)
        assert_same(call(cx, math, "ceil", &[n(300.0 / 100.0)]), 3.0, "ceil");
        // Math.max(expected_lines.length, actual_lines.length)
        assert_same(call(cx, math, "max", &[n(3.0), n(4.0)]), 4.0, "max");
    });
}

/// `#sec-math.abs`.
#[test]
fn abs_of_its_table() {
    unary_table(
        "abs",
        f64::NAN,
        &[
            (f64::NAN, f64::NAN),
            (-0.0, 0.0),
            (0.0, 0.0),
            (f64::NEG_INFINITY, f64::INFINITY),
            (f64::INFINITY, f64::INFINITY),
            (-5e-324, 5e-324),
            (-1.5, 1.5),
            (f64::MIN, f64::MAX),
            (3.0, 3.0),
        ],
    );
}

/// `#sec-math.ceil`: a value in (-1, -0) gives -0.
#[test]
fn ceil_of_its_table() {
    unary_table(
        "ceil",
        f64::NAN,
        &[
            (f64::NAN, f64::NAN),
            (-0.0, -0.0),
            (0.0, 0.0),
            (f64::NEG_INFINITY, f64::NEG_INFINITY),
            (f64::INFINITY, f64::INFINITY),
            (-0.5, -0.0),
            (-0.9999999999999999, -0.0),
            (-5e-324, -0.0),
            (5e-324, 1.0),
            (0.5, 1.0),
            (1.1, 2.0),
            (-1.1, -1.0),
            (4_503_599_627_370_495.5, 4_503_599_627_370_496.0),
            (-4_503_599_627_370_495.5, -4_503_599_627_370_495.0),
            (1e300, 1e300),
        ],
    );
}

/// `#sec-math.floor`: a value in (+0, 1) gives +0.
#[test]
fn floor_of_its_table() {
    unary_table(
        "floor",
        f64::NAN,
        &[
            (f64::NAN, f64::NAN),
            (-0.0, -0.0),
            (0.0, 0.0),
            (f64::NEG_INFINITY, f64::NEG_INFINITY),
            (f64::INFINITY, f64::INFINITY),
            (0.5, 0.0),
            (0.9999999999999999, 0.0),
            (5e-324, 0.0),
            (-5e-324, -1.0),
            (-0.5, -1.0),
            (1.9, 1.0),
            (-1.1, -2.0),
            (4_503_599_627_370_495.5, 4_503_599_627_370_495.0),
            (-4_503_599_627_370_495.5, -4_503_599_627_370_496.0),
            (-1e300, -1e300),
        ],
    );
}

/// `#sec-math.trunc`: toward zero, with the sign of the zeros kept.
#[test]
fn trunc_of_its_table() {
    unary_table(
        "trunc",
        f64::NAN,
        &[
            (f64::NAN, f64::NAN),
            (-0.0, -0.0),
            (0.0, 0.0),
            (f64::NEG_INFINITY, f64::NEG_INFINITY),
            (f64::INFINITY, f64::INFINITY),
            (0.5, 0.0),
            (-0.5, -0.0),
            (5e-324, 0.0),
            (-5e-324, -0.0),
            (1.9, 1.0),
            (-1.9, -1.0),
            (4_503_599_627_370_495.5, 4_503_599_627_370_495.0),
            (-4_503_599_627_370_495.5, -4_503_599_627_370_495.0),
            (f64::MAX, f64::MAX),
        ],
    );
}

/// `#sec-math.round`: ties toward +∞; a zero answer has the sign of the argument;
/// 0.49999999999999994 gives +0 and 2^52 + 1 itself, where `floor(x + 0.5)` gives 1 and
/// 2^52 + 2 (`docs/design/engine.md`, decision 6, "`Math.round`").
#[test]
fn round_of_its_table() {
    unary_table(
        "round",
        f64::NAN,
        &[
            (f64::NAN, f64::NAN),
            (-0.0, -0.0),
            (0.0, 0.0),
            (f64::NEG_INFINITY, f64::NEG_INFINITY),
            (f64::INFINITY, f64::INFINITY),
            (0.49999999999999994, 0.0),
            (-0.49999999999999994, -0.0),
            (0.5, 1.0),
            (-0.5, -0.0),
            (5e-324, 0.0),
            (-5e-324, -0.0),
            (1.5, 2.0),
            (-1.5, -1.0),
            (2.5, 3.0),
            (-2.5, -2.0),
            (3.5, 4.0),
            (-3.5, -3.0),
            (-0.5000000000000001, -1.0),
            (1.4999999999999998, 1.0),
            (4_503_599_627_370_495.5, 4_503_599_627_370_496.0),
            (-4_503_599_627_370_495.5, -4_503_599_627_370_495.0),
            (4_503_599_627_370_497.0, 4_503_599_627_370_497.0),
            (-4_503_599_627_370_497.0, -4_503_599_627_370_497.0),
            (f64::MAX, f64::MAX),
        ],
    );
}

/// `#sec-math.sign`: NaN and the zeros give themselves.
#[test]
fn sign_of_its_table() {
    unary_table(
        "sign",
        f64::NAN,
        &[
            (f64::NAN, f64::NAN),
            (-0.0, -0.0),
            (0.0, 0.0),
            (f64::NEG_INFINITY, -1.0),
            (f64::INFINITY, 1.0),
            (5e-324, 1.0),
            (-5e-324, -1.0),
            (-7.5, -1.0),
            (f64::MAX, 1.0),
        ],
    );
}

/// `#sec-math.fround`: to binary32 with ties to even, then back. 16777217 (2^24 + 1) is a tie
/// that goes to the even 2^24; 3.4028235677973366e38 is the first Number that goes to +∞, and
/// the Number before it goes to the largest binary32; 5e-324 goes to +0 and -5e-324 to -0.
#[test]
fn fround_of_its_table() {
    let overflow: f64 = 3.402_823_567_797_336_6e38;
    let before = f64::from_bits(overflow.to_bits() - 1);
    unary_table(
        "fround",
        f64::NAN,
        &[
            (f64::NAN, f64::NAN),
            (-0.0, -0.0),
            (0.0, 0.0),
            (f64::NEG_INFINITY, f64::NEG_INFINITY),
            (f64::INFINITY, f64::INFINITY),
            (16_777_217.0, 16_777_216.0),
            (16_777_219.0, 16_777_220.0),
            (overflow, f64::INFINITY),
            (-overflow, f64::NEG_INFINITY),
            (before, f64::from(f32::MAX)),
            (5e-324, 0.0),
            (-5e-324, -0.0),
            (0.1, f64::from(0.1_f32)),
            (1.5, 1.5),
        ],
    );
}

/// `#sec-math.sqrt`: -0 gives -0, a negative Number NaN, 2 the Number nearest √2.
#[test]
fn sqrt_of_its_table() {
    unary_table(
        "sqrt",
        f64::NAN,
        &[
            (f64::NAN, f64::NAN),
            (-0.0, -0.0),
            (0.0, 0.0),
            (f64::NEG_INFINITY, f64::NAN),
            (f64::INFINITY, f64::INFINITY),
            (-1.0, f64::NAN),
            (-5e-324, f64::NAN),
            (2.0, std::f64::consts::SQRT_2),
            (4.0, 2.0),
            (0.25, 0.5),
            (5e-324, 2.2227587494850775e-162),
        ],
    );
}

/// `#sec-math.clz32`: over ToUint32, so -1 is 0xffffffff, 4294967296 and 0.5 are 0, and no
/// argument is 0.
#[test]
fn clz32_of_its_table() {
    unary_table(
        "clz32",
        32.0,
        &[
            (f64::NAN, 32.0),
            (-0.0, 32.0),
            (0.0, 32.0),
            (f64::INFINITY, 32.0),
            (1.0, 31.0),
            (-1.0, 0.0),
            (4_294_967_296.0, 32.0),
            (4_294_967_297.0, 31.0),
            (0.5, 32.0),
            (2_147_483_648.0, 0.0),
            (2_147_483_647.0, 1.0),
            (65_535.9, 16.0),
        ],
    );
}

/// `#sec-math.imul`: ToUint32 of each argument, in order, then the product modulo 2^32 read as
/// a signed 32-bit integer.
#[test]
fn imul_of_its_table() {
    let n = Value::Number;
    with_math(|cx, _, math| {
        for (a, b, answer) in [
            (4_294_967_295.0, 5.0, -5.0),
            (2.0, 3.0, 6.0),
            (-1.0, 8.0, -8.0),
            (-0.0, 3.0, 0.0),
            (f64::NAN, 3.0, 0.0),
            (65_536.0, 65_536.0, 0.0),
            (2_147_483_647.0, 2.0, -2.0),
            (2_147_483_648.0, 1.0, -2_147_483_648.0),
            (4_294_967_296.5, 7.0, 0.0),
            (3.9, -2.9, -6.0),
        ] {
            let got = call(cx, math, "imul", &[n(a), n(b)]);
            assert_same(got, answer, &format!("imul({a:?}, {b:?})"));
        }
        assert_same(call(cx, math, "imul", &[]), 0.0, "imul()");
        assert_same(call(cx, math, "imul", &[n(3.0)]), 0.0, "imul(3)");
    });
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var log = []; \
             var x = { valueOf: function () { log.push('x'); return 3; } }; \
             var y = { valueOf: function () { log.push('y'); return 4; } }; \
             Math.imul(x, y) + ':' + log.join()"
        ),
        Shown::Normal("\"12:x,y\"".to_string())
    );
}

/// `#sec-math.min`: every argument goes through ToNumber first, in order, even after a NaN;
/// any NaN gives NaN; -0 is smaller than +0 whatever their order; no argument gives +∞.
#[test]
fn min_of_nan_and_signed_zeros() {
    let n = Value::Number;
    with_math(|cx, _, math| {
        let of = |cx: &mut Cx<'_, TestHost>, args: &[Value], answer: f64| {
            let got = call(cx, math, "min", args);
            assert_same(got, answer, &format!("min({args:?})"));
        };
        of(cx, &[], f64::INFINITY);
        of(cx, &[n(-0.0), n(0.0)], -0.0);
        of(cx, &[n(0.0), n(-0.0)], -0.0);
        of(cx, &[n(0.0), n(0.0)], 0.0);
        of(cx, &[n(-0.0)], -0.0);
        of(cx, &[n(1.0), n(f64::NAN), n(-3.0)], f64::NAN);
        of(cx, &[Value::Undefined], f64::NAN);
        of(
            cx,
            &[n(5.0), n(f64::NEG_INFINITY), n(2.0)],
            f64::NEG_INFINITY,
        );
        of(cx, &[Value::Null, n(1.0)], 0.0);
        let three = string(cx, "-3");
        of(cx, &[three, n(2.0)], -3.0);
    });
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var log = []; \
             var a = { valueOf: function () { log.push('a'); return 0; } }; \
             var b = { valueOf: function () { log.push('b'); return -1; } }; \
             Math.min(NaN, a, b) + ':' + log.join()"
        ),
        Shown::Normal("\"NaN:a,b\"".to_string())
    );
}

/// 64 inputs, each Number kind and the boundaries the functions turn on.
const INPUTS: [f64; 64] = [
    f64::NAN,
    0.0,
    -0.0,
    f64::INFINITY,
    f64::NEG_INFINITY,
    0.5,
    -0.5,
    0.49999999999999994,
    -0.49999999999999994,
    0.5000000000000001,
    -0.5000000000000001,
    1.0,
    -1.0,
    1.5,
    -1.5,
    2.5,
    -2.5,
    3.5,
    -3.5,
    0.1,
    -0.1,
    0.9999999999999999,
    -0.9999999999999999,
    5e-324,
    -5e-324,
    2.2250738585072014e-308,
    1e-300,
    -1e-300,
    1e-7,
    123_456.789,
    -123_456.789,
    16_777_217.0,
    16_777_216.0,
    3.402_823_466_385_288_6e38,
    3.402_823_567_797_336_6e38,
    -3.402_823_567_797_336_6e38,
    2_147_483_647.0,
    2_147_483_648.0,
    -2_147_483_648.0,
    -2_147_483_649.0,
    4_294_967_295.0,
    4_294_967_296.0,
    4_294_967_297.0,
    -4_294_967_297.0,
    4_503_599_627_370_495.5,
    -4_503_599_627_370_495.5,
    4_503_599_627_370_496.0,
    4_503_599_627_370_497.0,
    -4_503_599_627_370_497.0,
    9_007_199_254_740_992.0,
    1e21,
    -1e21,
    1e300,
    -1e300,
    f64::MAX,
    f64::MIN,
    7.0,
    -7.0,
    2.0,
    4.0,
    0.25,
    65_535.9,
    -65_535.9,
    1.0 / 3.0,
];

/// ToUint32 (`#sec-touint32`), written here as the standard states it.
fn to_uint32(x: f64) -> u32 {
    if !x.is_finite() {
        return 0;
    }
    let modulo = x.trunc().rem_euclid(4_294_967_296.0);
    modulo as u32
}

/// What each function of `FUNCTIONS` answers for `x` (and, for `imul` and `min`, `y`), written
/// from the standard's steps with `std` as the oracle, by other means than the engine where the
/// engine's means are the ones under test.
fn oracle(name: &str, x: f64, y: f64) -> f64 {
    match name {
        "abs" => x.abs(),
        "ceil" => x.ceil(),
        "clz32" => f64::from(to_uint32(x).leading_zeros()),
        "floor" => x.floor(),
        "fround" => f64::from(x as f32),
        "imul" => {
            let product = u64::from(to_uint32(x)) * u64::from(to_uint32(y)) % (1 << 32);
            let product = product as f64;
            if product >= 2_147_483_648.0 {
                product - 4_294_967_296.0
            } else {
                product
            }
        }
        "min" => {
            if x.is_nan() || y.is_nan() {
                f64::NAN
            } else if x == 0.0 && y == 0.0 {
                if x.is_sign_negative() { x } else { y }
            } else if x < y {
                x
            } else {
                y
            }
        }
        "round" => {
            // `std`'s round has its ties away from zero; a negative tie goes toward +∞ instead.
            // `fract` is exact.
            let answer = if x.fract() == -0.5 {
                x.ceil()
            } else {
                x.round()
            };
            if answer == 0.0 {
                0.0_f64.copysign(x)
            } else {
                answer
            }
        }
        "sign" => {
            if x.is_nan() || x == 0.0 {
                x
            } else if x < 0.0 {
                -1.0
            } else {
                1.0
            }
        }
        "sqrt" => x.sqrt(),
        "trunc" => x.trunc(),
        other => panic!("no oracle for {other}"),
    }
}

/// Each of the 11 functions over the 64 inputs of `INPUTS` (`imul` and `min` with a second
/// argument taken 23 rows further on): 704 calls, each answer to the bit as the oracle gives.
#[test]
fn the_exact_functions_over_a_table_of_64_inputs() {
    let n = Value::Number;
    let calls = with_math(|cx, _, math| {
        let mut calls = 0;
        for (name, length) in FUNCTIONS {
            for (row, &x) in INPUTS.iter().enumerate() {
                let y = INPUTS[(row + 23) % INPUTS.len()];
                let args = [n(x), n(y)];
                let args = &args[..usize::try_from(length).unwrap()];
                let got = call(cx, math, name, args);
                assert_same(got, oracle(name, x, y), &format!("{name}{args:?}"));
                calls += 1;
            }
        }
        calls
    });
    assert_eq!(calls, 704);
}

/// The step budget (decision 3): `Math.min` and `Math.max` charge a step per argument in each
/// of their two loops, so a call with 5,000 arguments under a budget of 2,000 steps ends in
/// `Terminated(StepBudget)`, where 900 arguments answer; the agent stays alive and its next
/// entry runs.
#[test]
fn min_and_max_over_many_arguments_end_at_the_step_budget() {
    let mut limits = limits();
    limits.steps_per_entry = 2_000;
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    let global_root = global(&mut agent, &realm);
    let many: Vec<Value> = (0..5_000).map(|i| Value::Number(f64::from(i))).collect();
    let entry = |agent: &mut Agent<TestHost>, name: &str, args: &[Value]| {
        agent
            .with_cx(&realm, |cx| {
                let global = object(cx, &global_root);
                let math = get_object(cx, global, "Math");
                let function = get(cx, math, name);
                cx.call(function, Value::Object(math), args).unwrap()
            })
            .unwrap()
    };
    for (name, answer) in [("min", 0.0), ("max", 899.0)] {
        assert_eq!(
            entry(&mut agent, name, &many[..900]),
            Ok(Value::Number(answer)),
            "{name} of 900"
        );
        assert_eq!(
            entry(&mut agent, name, &many),
            Err(Abrupt::Terminated(Termination::StepBudget)),
            "{name} of 5000"
        );
        assert_eq!(agent.state(), AgentState::Alive);
        assert_eq!(
            eval_text(&mut agent, &realm, "Math.min(3, 1, 2) + Math.max(3, 1, 2)"),
            Shown::Normal("4".to_string())
        );
    }
}

/// The memory budget (`docs/design/engine.md`, "Memory budget"): `Math.min` and `Math.max`
/// charge their list of coerced Numbers, 8 bytes per argument, before they make it. A call with
/// 100,000 arguments under a budget that holds the copy of the arguments into the call (as a
/// call of `Function.prototype`, which makes no list, shows by answering) and half of the list
/// ends in `Terminated(MemoryBudget)`; the agent stays alive and its next entry runs. With the
/// whole list and room the same call answers.
#[test]
fn min_and_max_over_many_arguments_end_at_the_memory_budget() {
    const ARGUMENTS: u32 = 100_000;
    let copy = u64::from(ARGUMENTS) * size_of::<Value>() as u64;
    let list = u64::from(ARGUMENTS) * size_of::<f64>() as u64;
    let mut agent = agent().unwrap();
    let _realm = realm(&mut agent);
    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
    let base = agent.heap_bytes().unwrap();
    let many: Vec<Value> = (0..ARGUMENTS)
        .map(|i| Value::Number(f64::from(i)))
        .collect();
    let entry = |agent: &mut Agent<TestHost>, realm: &Realm, path: &[&str]| {
        let global_root = global(agent, realm);
        agent
            .with_cx(realm, |cx| {
                let global = object(cx, &global_root);
                let (holder, name) = match path {
                    [name] => (global, *name),
                    [first, name] => (get_object(cx, global, first), *name),
                    _ => unreachable!(),
                };
                let function = get(cx, holder, name);
                cx.call(function, Value::Object(holder), &many).unwrap()
            })
            .unwrap()
    };
    for (name, answer) in [("min", 0.0), ("max", 99_999.0)] {
        for (memory, fits) in [
            (base + copy + list / 2, false),
            (base + copy + 2 * list, true),
        ] {
            let mut limits = limits();
            limits.memory_bytes = memory;
            let mut agent = agent_with(limits).unwrap();
            let realm = realm(&mut agent);
            assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
            assert_eq!(
                entry(&mut agent, &realm, &["Function", "prototype"]),
                Ok(Value::Undefined),
                "the copy of the arguments fits {memory}"
            );
            let got = entry(&mut agent, &realm, &["Math", name]);
            if fits {
                assert_eq!(got, Ok(Value::Number(answer)), "{name} under {memory}");
            } else {
                assert_eq!(
                    got,
                    Err(Abrupt::Terminated(Termination::MemoryBudget)),
                    "{name} under {memory}"
                );
            }
            assert_eq!(agent.state(), AgentState::Alive);
            assert_eq!(
                eval_text(&mut agent, &realm, "Math.min(3, 1, 2) + Math.max(3, 1, 2)"),
                Shown::Normal("4".to_string())
            );
        }
    }
}

thread_local! {
    /// The handle `interrupt_now` sets.
    static HANDLE: Cell<Option<InterruptHandle>> = const { Cell::new(None) };
}

/// A host function that sets its agent's interrupt flag and answers 1, counting its calls.
fn interrupt_now(cx: &mut Cx<'_, TestHost>, _call: HostCall<'_>) -> Completion<Value> {
    cx.host_mut().count += 1;
    HANDLE.with(|slot| {
        if let Some(handle) = slot.take() {
            handle.interrupt();
        }
    });
    Ok(Value::Number(1.0))
}

/// The interrupt (HOST-96, as `tests/interrupt.rs` holds it): a valueOf of the second argument
/// that raises the interrupt ends `Math.min` and `Math.max` in `Terminated(Interrupted)` before
/// the third argument is converted; the statement after the call does not run.
#[test]
fn min_and_max_over_many_arguments_end_at_an_interrupt() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    define_function(&mut agent, &realm, "interruptNow", 0, interrupt_now, false);
    for name in ["min", "max"] {
        HANDLE.with(|slot| slot.set(Some(agent.interrupt_handle())));
        let source = format!(
            "var seen = 0, after = 0; \
             var o = {{ valueOf: interruptNow }}; \
             var p = {{ valueOf: function () {{ seen = seen + 1; return 0; }} }}; \
             Math.{name}(1, o, p, 2); after = 1;"
        );
        assert_eq!(
            eval_text(&mut agent, &realm, &source),
            Shown::Terminated(Termination::Interrupted),
            "{name}"
        );
        assert!(!agent.interrupt_handle().is_set());
        assert_eq!(
            eval_text(&mut agent, &realm, "seen + ',' + after"),
            Shown::Normal("\"0,0\"".to_string()),
            "{name}"
        );
    }
    assert_eq!(agent.host().count, 2);
}

/// A Symbol argument throws a TypeError from ToNumber and ToUint32 (`#sec-tonumber`), for each
/// function and each of the two arguments of `imul` and `min`.
#[test]
fn a_symbol_argument_throws_a_type_error() {
    with_math(|cx, global, math| {
        for (name, length) in FUNCTIONS.into_iter().chain(APPROXIMATED) {
            let function = get(cx, math, name);
            assert!(matches!(function, Value::Object(_)), "Math.{name}");
            let symbol_constructor = get(cx, global, "Symbol");
            let symbol = cx
                .call(symbol_constructor, Value::Undefined, &[])
                .unwrap()
                .unwrap();
            for position in 0..length {
                let mut args = vec![Value::Number(1.0); usize::try_from(length).unwrap()];
                args[usize::try_from(position).unwrap()] = symbol;
                let error = thrown(cx.call(function, Value::Object(math), &args).unwrap());
                assert!(is_error_of(cx, global, error, "TypeError"), "{name}");
            }
        }
    });
}

/// 1 + 2^-52, the Number after 1.
const ABOVE_ONE: f64 = 1.000_000_000_000_000_2;
/// 1 - 2^-53, the Number before 1.
const BELOW_ONE: f64 = 0.999_999_999_999_999_9;
const NAN: f64 = f64::NAN;
const INF: f64 = f64::INFINITY;

/// An argument of the tables below: a Number, or a value that ToNumber (`#sec-tonumber`)
/// converts before the function reads it.
#[derive(Clone, Copy, Debug)]
enum Arg {
    N(f64),
    S(&'static str),
    Undefined,
    Null,
    Bool(bool),
}

use Arg::{Bool, N, Null, S, Undefined};

/// `arg` as a value of the agent of `cx`.
fn arg_value(cx: &mut Cx<'_, TestHost>, arg: Arg) -> Value {
    match arg {
        N(number) => Value::Number(number),
        S(source) => string(cx, source),
        Undefined => Value::Undefined,
        Null => Value::Null,
        Bool(flag) => Value::Boolean(flag),
    }
}

/// The one-argument function `Math[name]` over a table of 12 arguments, each with the answer the
/// draft fixes for it, to the bit; and with no argument, which converts `undefined` to NaN.
fn fixed_unary(name: &str, table: &[(Arg, f64); 12]) {
    with_math(|cx, _, math| {
        assert_same(call(cx, math, name, &[]), NAN, &format!("{name}()"));
        for &(arg, answer) in table {
            let value = arg_value(cx, arg);
            let got = call(cx, math, name, &[value]);
            assert_same(got, answer, &format!("{name}({arg:?})"));
        }
    });
}

/// `#sec-math.acos`: NaN, and a Number above 1 or below -1 (`acos(2)`), give NaN; 1 gives +0.
#[test]
fn acos_of_its_table() {
    fixed_unary(
        "acos",
        &[
            (N(NAN), NAN),
            (N(2.0), NAN),
            (N(-2.0), NAN),
            (N(ABOVE_ONE), NAN),
            (N(-ABOVE_ONE), NAN),
            (N(INF), NAN),
            (N(-INF), NAN),
            (N(1.0), 0.0),
            (S("1"), 0.0),
            (Bool(true), 0.0),
            (Undefined, NAN),
            (S("x"), NAN),
        ],
    );
}

/// `#sec-math.acosh`: NaN and +∞ give themselves, 1 gives +0, below 1 NaN (`acosh(0.5)`).
#[test]
fn acosh_of_its_table() {
    fixed_unary(
        "acosh",
        &[
            (N(NAN), NAN),
            (N(INF), INF),
            (N(1.0), 0.0),
            (N(0.5), NAN),
            (N(BELOW_ONE), NAN),
            (N(0.0), NAN),
            (N(-0.0), NAN),
            (N(-1.0), NAN),
            (N(-INF), NAN),
            (Bool(true), 0.0),
            (S("Infinity"), INF),
            (Null, NAN),
        ],
    );
}

/// `#sec-math.asin`: NaN and the zeros give themselves; above 1 or below -1 NaN
/// (`asin(-1.1)`).
#[test]
fn asin_of_its_table() {
    fixed_unary(
        "asin",
        &[
            (N(NAN), NAN),
            (N(0.0), 0.0),
            (N(-0.0), -0.0),
            (N(-1.1), NAN),
            (N(1.1), NAN),
            (N(ABOVE_ONE), NAN),
            (N(INF), NAN),
            (N(-INF), NAN),
            (S("-0"), -0.0),
            (Null, 0.0),
            (Bool(false), 0.0),
            (Undefined, NAN),
        ],
    );
}

/// The table of a function that gives back every argument that is not a non-zero finite
/// Number: `asinh`, `cbrt` and `sinh`.
const NOT_NON_ZERO_FINITE: [(Arg, f64); 12] = [
    (N(NAN), NAN),
    (N(0.0), 0.0),
    (N(-0.0), -0.0),
    (N(INF), INF),
    (N(-INF), -INF),
    (S("-0"), -0.0),
    (Null, 0.0),
    (S(""), 0.0),
    (Bool(false), 0.0),
    (S("Infinity"), INF),
    (S("-Infinity"), -INF),
    (Undefined, NAN),
];

/// `#sec-math.asinh`: an argument that is not a non-zero finite Number gives itself.
#[test]
fn asinh_of_its_table() {
    fixed_unary("asinh", &NOT_NON_ZERO_FINITE);
}

/// `#sec-math.atan`: NaN and the zeros give themselves (the infinities answer an approximation
/// of ±π/2, which the draft does not fix).
#[test]
fn atan_of_its_table() {
    fixed_unary(
        "atan",
        &[
            (N(NAN), NAN),
            (N(0.0), 0.0),
            (N(-0.0), -0.0),
            (S("-0"), -0.0),
            (S(" -0 "), -0.0),
            (Null, 0.0),
            (Bool(false), 0.0),
            (S(""), 0.0),
            (S("\t"), 0.0),
            (Undefined, NAN),
            (S("x"), NAN),
            (S("NaN"), NAN),
        ],
    );
}

/// `#sec-math.atanh`: NaN and the zeros give themselves; above 1 or below -1 NaN; 1 gives +∞
/// (`atanh(1)`) and -1 gives -∞.
#[test]
fn atanh_of_its_table() {
    fixed_unary(
        "atanh",
        &[
            (N(NAN), NAN),
            (N(0.0), 0.0),
            (N(-0.0), -0.0),
            (N(1.0), INF),
            (N(-1.0), -INF),
            (N(ABOVE_ONE), NAN),
            (N(-ABOVE_ONE), NAN),
            (N(2.0), NAN),
            (N(INF), NAN),
            (N(-INF), NAN),
            (Bool(true), INF),
            (S("-1"), -INF),
        ],
    );
}

/// `#sec-math.cbrt`: an argument that is not a non-zero finite Number gives itself.
#[test]
fn cbrt_of_its_table() {
    fixed_unary("cbrt", &NOT_NON_ZERO_FINITE);
}

/// `#sec-math.cos`: an argument that is not finite gives NaN; the zeros give 1 (`cos(±0)`).
#[test]
fn cos_of_its_table() {
    fixed_unary(
        "cos",
        &[
            (N(NAN), NAN),
            (N(INF), NAN),
            (N(-INF), NAN),
            (N(0.0), 1.0),
            (N(-0.0), 1.0),
            (S("-0"), 1.0),
            (Null, 1.0),
            (Bool(false), 1.0),
            (S(""), 1.0),
            (Undefined, NAN),
            (S("x"), NAN),
            (S("Infinity"), NAN),
        ],
    );
}

/// `#sec-math.cosh`: NaN gives NaN, the infinities +∞, the zeros 1 (`cosh(±0)`).
#[test]
fn cosh_of_its_table() {
    fixed_unary(
        "cosh",
        &[
            (N(NAN), NAN),
            (N(INF), INF),
            (N(-INF), INF),
            (N(0.0), 1.0),
            (N(-0.0), 1.0),
            (S("-0"), 1.0),
            (Null, 1.0),
            (Bool(false), 1.0),
            (S(""), 1.0),
            (S("-Infinity"), INF),
            (Undefined, NAN),
            (S("x"), NAN),
        ],
    );
}

/// `#sec-math.exp`: NaN and +∞ give themselves, the zeros 1 (`exp(±0)`), -∞ gives +0.
#[test]
fn exp_of_its_table() {
    fixed_unary(
        "exp",
        &[
            (N(NAN), NAN),
            (N(INF), INF),
            (N(-INF), 0.0),
            (N(0.0), 1.0),
            (N(-0.0), 1.0),
            (S("-0"), 1.0),
            (Null, 1.0),
            (Bool(false), 1.0),
            (S(""), 1.0),
            (S("-Infinity"), 0.0),
            (Undefined, NAN),
            (S("Infinity"), INF),
        ],
    );
}

/// `#sec-math.expm1`: NaN, the zeros and +∞ give themselves, -∞ gives -1.
#[test]
fn expm1_of_its_table() {
    fixed_unary(
        "expm1",
        &[
            (N(NAN), NAN),
            (N(0.0), 0.0),
            (N(-0.0), -0.0),
            (N(INF), INF),
            (N(-INF), -1.0),
            (S("-0"), -0.0),
            (Null, 0.0),
            (Bool(false), 0.0),
            (S(""), 0.0),
            (S("-Infinity"), -1.0),
            (Undefined, NAN),
            (S("Infinity"), INF),
        ],
    );
}

/// The table of a logarithm (`log`, `log10`, `log2`): NaN and +∞ give themselves, 1 gives +0,
/// the zeros -∞, below -0 NaN (`log(-1)`); then the exact cases of the function.
fn logarithm_table(exact: [(Arg, f64); 4]) -> [(Arg, f64); 12] {
    let [a, b, c, d] = exact;
    [
        (N(NAN), NAN),
        (N(INF), INF),
        (N(1.0), 0.0),
        (N(0.0), -INF),
        (N(-0.0), -INF),
        (N(-1.0), NAN),
        (N(-INF), NAN),
        (N(-5e-324), NAN),
        a,
        b,
        c,
        d,
    ]
}

/// `#sec-math.log`: the table of a logarithm, and conversions.
#[test]
fn log_of_its_table() {
    fixed_unary(
        "log",
        &logarithm_table([
            (Bool(true), 0.0),
            (Null, -INF),
            (S("-0"), -INF),
            (Undefined, NAN),
        ]),
    );
}

/// `#sec-math.log10`: the table of a logarithm, and `log10(10)`, `log10(100)` and
/// `log10(1000)` of test262, exact.
#[test]
fn log10_of_its_table() {
    fixed_unary(
        "log10",
        &logarithm_table([
            (N(10.0), 1.0),
            (N(100.0), 2.0),
            (N(1000.0), 3.0),
            (S("1000"), 3.0),
        ]),
    );
}

/// `#sec-math.log1p`: NaN, the zeros and +∞ give themselves, -1 gives -∞, below -1 NaN.
#[test]
fn log1p_of_its_table() {
    fixed_unary(
        "log1p",
        &[
            (N(NAN), NAN),
            (N(0.0), 0.0),
            (N(-0.0), -0.0),
            (N(INF), INF),
            (N(-1.0), -INF),
            (N(-ABOVE_ONE), NAN),
            (N(-2.0), NAN),
            (N(-INF), NAN),
            (S("-1"), -INF),
            (Null, 0.0),
            (S("-0"), -0.0),
            (Undefined, NAN),
        ],
    );
}

/// `#sec-math.log2`: the table of a logarithm, and `log2(2)`, `log2(4)` and `log2(8)` of
/// test262, exact.
#[test]
fn log2_of_its_table() {
    fixed_unary(
        "log2",
        &logarithm_table([(N(2.0), 1.0), (N(4.0), 2.0), (N(8.0), 3.0), (S("8"), 3.0)]),
    );
}

/// The table of `sin` and `tan`: NaN and the zeros give themselves, the infinities NaN.
const SIN_AND_TAN: [(Arg, f64); 12] = [
    (N(NAN), NAN),
    (N(0.0), 0.0),
    (N(-0.0), -0.0),
    (N(INF), NAN),
    (N(-INF), NAN),
    (S("-0"), -0.0),
    (Null, 0.0),
    (Bool(false), 0.0),
    (S(""), 0.0),
    (Undefined, NAN),
    (S("Infinity"), NAN),
    (S("-Infinity"), NAN),
];

/// `#sec-math.sin`.
#[test]
fn sin_of_its_table() {
    fixed_unary("sin", &SIN_AND_TAN);
}

/// `#sec-math.sinh`: an argument that is not a non-zero finite Number gives itself.
#[test]
fn sinh_of_its_table() {
    fixed_unary("sinh", &NOT_NON_ZERO_FINITE);
}

/// `#sec-math.tan`.
#[test]
fn tan_of_its_table() {
    fixed_unary("tan", &SIN_AND_TAN);
}

/// `#sec-math.tanh`: NaN and the zeros give themselves, +∞ gives 1 (`tanh(+∞)`), -∞ gives -1.
#[test]
fn tanh_of_its_table() {
    fixed_unary(
        "tanh",
        &[
            (N(NAN), NAN),
            (N(0.0), 0.0),
            (N(-0.0), -0.0),
            (N(INF), 1.0),
            (N(-INF), -1.0),
            (S("-0"), -0.0),
            (Null, 0.0),
            (Bool(false), 0.0),
            (S("Infinity"), 1.0),
            (S("-Infinity"), -1.0),
            (Undefined, NAN),
            (S("x"), NAN),
        ],
    );
}

/// `#sec-math.atan2`: a NaN on either side gives NaN; y +0 with x above +0 or +0 gives +0, y -0
/// so gives -0; y above +0 with x +∞ gives +0, below -0 gives -0. The other cases of the
/// clause answer approximations of multiples of π, which the draft does not fix.
#[test]
fn atan2_of_its_table() {
    with_math(|cx, _, math| {
        assert_same(call(cx, math, "atan2", &[]), NAN, "atan2()");
        assert_same(
            call(cx, math, "atan2", &[Value::Number(0.0)]),
            NAN,
            "atan2(0)",
        );
        for (y, x, answer) in [
            (NAN, 1.0, NAN),
            (1.0, NAN, NAN),
            (NAN, NAN, NAN),
            (0.0, 0.0, 0.0),
            (0.0, 1.0, 0.0),
            (0.0, INF, 0.0),
            (-0.0, 0.0, -0.0),
            (-0.0, 5.0, -0.0),
            (-0.0, INF, -0.0),
            (1.0, INF, 0.0),
            (-1.0, INF, -0.0),
            (-5e-324, INF, -0.0),
        ] {
            let got = call(cx, math, "atan2", &[Value::Number(y), Value::Number(x)]);
            assert_same(got, answer, &format!("atan2({y:?}, {x:?})"));
        }
    });
}

/// `#sec-math.pow` is Number::exponentiate (`#sec-numeric-types-number-exponentiate`): every
/// line of its table before the last, `pow(-8, 1/3)` NaN (a negative base with an exponent that
/// is not integral), `pow(1, ∞)` NaN, and `pow(1, -2^31)` 1 of test262.
#[test]
fn pow_of_its_table() {
    with_math(|cx, _, math| {
        assert_same(call(cx, math, "pow", &[]), NAN, "pow()");
        assert_same(call(cx, math, "pow", &[Value::Number(1.0)]), NAN, "pow(1)");
        for (base, exponent, answer) in [
            (1.0, NAN, NAN),
            (NAN, 0.0, 1.0),
            (NAN, -0.0, 1.0),
            (NAN, 1.0, NAN),
            (INF, 1.0, INF),
            (INF, -1.0, 0.0),
            (-INF, 3.0, -INF),
            (-INF, 2.0, INF),
            (-INF, -3.0, -0.0),
            (-INF, -2.0, 0.0),
            (0.0, 1.0, 0.0),
            (0.0, -1.0, INF),
            (-0.0, 3.0, -0.0),
            (-0.0, 2.0, 0.0),
            (-0.0, -3.0, -INF),
            (-0.0, -2.0, INF),
            (2.0, INF, INF),
            (1.0, INF, NAN),
            (-1.0, INF, NAN),
            (0.5, INF, 0.0),
            (2.0, -INF, 0.0),
            (1.0, -INF, NAN),
            (0.5, -INF, INF),
            (-8.0, 1.0 / 3.0, NAN),
            (-1.5, 0.5, NAN),
            (1.0, -2_147_483_648.0, 1.0),
        ] {
            let got = call(
                cx,
                math,
                "pow",
                &[Value::Number(base), Value::Number(exponent)],
            );
            assert_same(got, answer, &format!("pow({base:?}, {exponent:?})"));
        }
    });
}

/// `#sec-math.hypot`: an infinite argument gives +∞ even beside a NaN, then a NaN gives NaN,
/// then arguments that are all zero (or none) give +0; `hypot(3, 4)` of test262 is 5.
#[test]
fn hypot_of_its_table() {
    with_math(|cx, _, math| {
        let tables: [(&[f64], f64); 13] = [
            (&[], 0.0),
            (&[INF], INF),
            (&[-INF], INF),
            (&[NAN, -INF], INF),
            (&[INF, NAN], INF),
            (&[1.0, NAN, -INF], INF),
            (&[NAN], NAN),
            (&[1.0, NAN], NAN),
            (&[NAN, 0.0], NAN),
            (&[0.0], 0.0),
            (&[-0.0], 0.0),
            (&[-0.0, 0.0, -0.0], 0.0),
            (&[3.0, 4.0], 5.0),
        ];
        for (args, answer) in tables {
            let values: Vec<Value> = args.iter().map(|&n| Value::Number(n)).collect();
            let got = call(cx, math, "hypot", &values);
            assert_same(got, answer, &format!("hypot{args:?}"));
        }
        assert_same(
            call(cx, math, "hypot", &[Value::Undefined]),
            NAN,
            "hypot(undefined)",
        );
        let zero = string(cx, "-0");
        assert_same(call(cx, math, "hypot", &[zero]), 0.0, "hypot(\"-0\")");
    });
}

/// `#sec-math.atan2`, `#sec-math.pow` and `#sec-math.hypot`: ToNumber of each argument in
/// order (y before x, base before exponent) before the answer, and of no argument after those
/// the function has; `hypot(a, b, c)` converts all three even when the first is NaN or
/// infinite; a conversion that throws ends the call before the next.
#[test]
fn atan2_pow_and_hypot_convert_their_arguments_in_order() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    for (expression, shown) in [
        ("Math.atan2(o('y', 0), o('x', 1), o('z', 2))", "0:y,x"),
        (
            "Math.pow(o('base', 2), o('exponent', 10), o('z', 2))",
            "1024:base,exponent",
        ),
        ("Math.hypot(o('a', 3), o('b', 4))", "5:a,b"),
        ("Math.hypot(o('a', NaN), o('b', 3), o('c', 4))", "NaN:a,b,c"),
        (
            "Math.hypot(o('a', Infinity), o('b', NaN), o('c', 4))",
            "Infinity:a,b,c",
        ),
        (
            "Math.hypot(o('a', -Infinity), o('b', 1), o('c', 2))",
            "Infinity:a,b,c",
        ),
        (
            "(function () { try { Math.hypot(o('a', 1), thrower('b'), o('c', 2)); } \
             catch (e) { return e; } }())",
            "thrown:a,b",
        ),
        (
            "(function () { try { Math.pow(thrower('base'), o('exponent', 2)); } \
             catch (e) { return e; } }())",
            "thrown:base",
        ),
        (
            "(function () { try { Math.atan2(o('y', 1), thrower('x')); } \
             catch (e) { return e; } }())",
            "thrown:y,x",
        ),
    ] {
        let source = format!(
            "var log = []; \
             function o(name, value) {{ \
               return {{ valueOf: function () {{ log.push(name); return value; }} }}; }} \
             function thrower(name) {{ \
               return {{ valueOf: function () {{ log.push(name); throw 'thrown'; }} }}; }} \
             {expression} + ':' + log.join()"
        );
        assert_eq!(
            eval_text(&mut agent, &realm, &source),
            Shown::Normal(format!("\"{shown}\"")),
            "{expression}"
        );
    }
}

/// The step budget (decision 3): `Math.hypot` charges a step per argument while it converts
/// them and a step per element of its list before `swell_num::math::hypot` runs over it, so a
/// call with 5,000 arguments under a budget of 2,000 steps ends in `Terminated(StepBudget)`,
/// where 900 arguments answer; the agent stays alive and its next entry runs.
#[test]
fn hypot_over_many_arguments_ends_at_the_step_budget() {
    let mut limits = limits();
    limits.steps_per_entry = 2_000;
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    let global_root = global(&mut agent, &realm);
    let numbers: Vec<f64> = (0..5_000).map(f64::from).collect();
    let many: Vec<Value> = numbers.iter().map(|&n| Value::Number(n)).collect();
    let entry = |agent: &mut Agent<TestHost>, args: &[Value]| {
        agent
            .with_cx(&realm, |cx| {
                let global = object(cx, &global_root);
                let math = get_object(cx, global, "Math");
                let function = get(cx, math, "hypot");
                cx.call(function, Value::Object(math), args).unwrap()
            })
            .unwrap()
    };
    let answer = swell_num::math::hypot(&numbers[..900]);
    assert_eq!(
        entry(&mut agent, &many[..900]),
        Ok(Value::Number(answer)),
        "hypot of 900"
    );
    assert_eq!(
        entry(&mut agent, &many),
        Err(Abrupt::Terminated(Termination::StepBudget)),
        "hypot of 5000"
    );
    assert_eq!(agent.state(), AgentState::Alive);
    assert_eq!(
        eval_text(&mut agent, &realm, "Math.hypot(3, 4)"),
        Shown::Normal("5".to_string())
    );
}

/// The memory budget (`docs/design/engine.md`, "Memory budget"): `Math.hypot` charges its list
/// of coerced Numbers, 8 bytes per argument, before it makes it. A call with 100,000 arguments
/// under a budget that holds the copy of the arguments into the call (as a call of
/// `Function.prototype`, which makes no list, shows by answering) and half of the list ends in
/// `Terminated(MemoryBudget)`; the agent stays alive and its next entry runs. With the whole
/// list and room the same call answers what `swell_num::math::hypot` answers.
#[test]
fn hypot_over_many_arguments_ends_at_the_memory_budget() {
    const ARGUMENTS: u32 = 100_000;
    let copy = u64::from(ARGUMENTS) * size_of::<Value>() as u64;
    let list = u64::from(ARGUMENTS) * size_of::<f64>() as u64;
    let mut agent = agent().unwrap();
    let _realm = realm(&mut agent);
    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
    let base = agent.heap_bytes().unwrap();
    let numbers: Vec<f64> = (0..ARGUMENTS).map(f64::from).collect();
    let many: Vec<Value> = numbers.iter().map(|&n| Value::Number(n)).collect();
    let entry = |agent: &mut Agent<TestHost>, realm: &Realm, holder: &str, name: &str| {
        let global_root = global(agent, realm);
        agent
            .with_cx(realm, |cx| {
                let global = object(cx, &global_root);
                let holder = get_object(cx, global, holder);
                let function = get(cx, holder, name);
                cx.call(function, Value::Object(holder), &many).unwrap()
            })
            .unwrap()
    };
    let answer = swell_num::math::hypot(&numbers);
    for (memory, fits) in [
        (base + copy + list / 2, false),
        (base + copy + 2 * list, true),
    ] {
        let mut limits = limits();
        limits.memory_bytes = memory;
        let mut agent = agent_with(limits).unwrap();
        let realm = realm(&mut agent);
        assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
        assert_eq!(
            entry(&mut agent, &realm, "Function", "prototype"),
            Ok(Value::Undefined),
            "the copy of the arguments fits {memory}"
        );
        let got = entry(&mut agent, &realm, "Math", "hypot");
        if fits {
            assert_eq!(got, Ok(Value::Number(answer)), "hypot under {memory}");
        } else {
            assert_eq!(
                got,
                Err(Abrupt::Terminated(Termination::MemoryBudget)),
                "hypot under {memory}"
            );
        }
        assert_eq!(agent.state(), AgentState::Alive);
        assert_eq!(
            eval_text(&mut agent, &realm, "Math.hypot(3, 4)"),
            Shown::Normal("5".to_string())
        );
    }
}

/// The interrupt (HOST-96, as `tests/interrupt.rs` holds it): a valueOf of the first argument
/// that raises the interrupt ends `Math.hypot`, `Math.atan2` and `Math.pow` in
/// `Terminated(Interrupted)` before the second argument is converted; the statement after the
/// call does not run.
#[test]
fn hypot_atan2_and_pow_end_at_an_interrupt_between_two_arguments() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    define_function(&mut agent, &realm, "interruptNow", 0, interrupt_now, false);
    for name in ["hypot", "atan2", "pow"] {
        HANDLE.with(|slot| slot.set(Some(agent.interrupt_handle())));
        let source = format!(
            "var seen = 0, after = 0; \
             var o = {{ valueOf: interruptNow }}; \
             var p = {{ valueOf: function () {{ seen = seen + 1; return 0; }} }}; \
             Math.{name}(o, p); after = 1;"
        );
        assert_eq!(
            eval_text(&mut agent, &realm, &source),
            Shown::Terminated(Termination::Interrupted),
            "{name}"
        );
        assert!(!agent.interrupt_handle().is_set());
        assert_eq!(
            eval_text(&mut agent, &realm, "seen + ',' + after"),
            Shown::Normal("\"0,0\"".to_string()),
            "{name}"
        );
    }
    assert_eq!(agent.host().count, 3);
}

/// What `swell_num::math` answers for the function `name` of `APPROXIMATED` at `x`, and `y` for
/// the second argument of `atan2`, `hypot` and `pow`: the function the steps of each end in.
fn swell_num_answer(name: &str, x: f64, y: f64) -> f64 {
    use swell_num::math;
    match name {
        "acos" => math::acos(x),
        "acosh" => math::acosh(x),
        "asin" => math::asin(x),
        "asinh" => math::asinh(x),
        "atan" => math::atan(x),
        "atan2" => math::atan2(x, y),
        "atanh" => math::atanh(x),
        "cbrt" => math::cbrt(x),
        "cos" => math::cos(x),
        "cosh" => math::cosh(x),
        "exp" => math::exp(x),
        "expm1" => math::expm1(x),
        "hypot" => math::hypot(&[x, y]),
        "log" => math::log(x),
        "log10" => math::log10(x),
        "log1p" => math::log1p(x),
        "log2" => math::log2(x),
        "pow" => math::exponentiate(x, y),
        "sin" => math::sin(x),
        "sinh" => math::sinh(x),
        "tan" => math::tan(x),
        "tanh" => math::tanh(x),
        other => panic!("no function of swell_num for {other}"),
    }
}

/// Each of the 22 functions of `APPROXIMATED` over the 64 inputs of `INPUTS` (`atan2`, `hypot`
/// and `pow` with a second argument taken 23 rows further on): 1,408 calls, each answer to the
/// bit what `swell_num::math` answers for the same Numbers, so that each function is bound to
/// its own. No answer is held to an accuracy here; the data of `swell-num` holds that.
#[test]
fn the_approximated_functions_over_a_table_of_64_inputs() {
    let n = Value::Number;
    let calls = with_math(|cx, _, math| {
        let mut calls = 0;
        for (name, length) in APPROXIMATED {
            for (row, &x) in INPUTS.iter().enumerate() {
                let y = INPUTS[(row + 23) % INPUTS.len()];
                let args = [n(x), n(y)];
                let args = &args[..usize::try_from(length).unwrap()];
                let got = call(cx, math, name, args);
                assert_same(
                    got,
                    swell_num_answer(name, x, y),
                    &format!("{name}{args:?}"),
                );
                calls += 1;
            }
        }
        calls
    });
    assert_eq!(calls, 1_408);
}
