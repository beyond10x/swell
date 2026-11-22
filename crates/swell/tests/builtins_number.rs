//! Number objects (ECMA-262 21.1, `#sec-number-objects`), the part slice 2 takes early
//! (`docs/design/engine.md`, section 12, "A part before its slice"): the `Number` constructor
//! (`#sec-number-constructor-number-value`), its value properties
//! (`#sec-properties-of-the-number-constructor`), `Number.prototype.constructor`
//! (`#sec-number.prototype.constructor`), `valueOf` (`#sec-number.prototype.valueof`) with
//! ThisNumberValue (`#sec-thisnumbervalue`) and `toString` (`#sec-number.prototype.tostring`)
//! with SnapToInteger (`#sec-snaptointeger`) and Number::toString in every radix
//! (`#sec-numeric-types-number-tostring`). Scripts run in sloppy and in strict code; the new
//! target is given through the host interface. Run also with the feature
//! `collect-every-safe-point`.
#![cfg(test)]

use crate::support;

use support::host::{TestHost, agent, get, get_object, global, key, object, realm};
use support::script::{Outcome, run, run_both};
use swell::{Completion, Cx, HostCall, Termination, Value};

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

/// A constructor whose "prototype" the test sets; it is only used as a new target.
fn unused(_cx: &mut Cx<'_, TestHost>, call: HostCall<'_>) -> Completion<Value> {
    Ok(Value::Object(call.callee))
}

/// `#sec-number-constructor-number-value`: called, `Number` answers ToNumeric of its argument
/// (+0 without one, NaN for undefined, the StringToNumber of a String, an object through
/// ToPrimitive with hint Number, whose throw is returned); constructed, a Number object whose
/// [[NumberData]] is that value and whose [[Prototype]] is `%Number.prototype%`, or the new
/// target's "prototype" (OrdinaryCreateFromConstructor). `Number` has length 1, name "Number"
/// and the [[Prototype]] `%Function.prototype%` (`#sec-properties-of-the-number-constructor`);
/// `Number.prototype` is a Number object whose [[NumberData]] is +0
/// (`#sec-properties-of-the-number-prototype-object`), its "constructor" `Number`.
#[test]
fn number_called_and_constructed() {
    both_answer("Number();", "0");
    both_answer("Number(undefined);", "NaN");
    both_answer(
        "Number(null) + '|' + Number(true) + '|' + Number(false);",
        "\"0|1|0\"",
    );
    both_answer("Number('  12  ') + Number('0x10') + Number('');", "28");
    both_answer(
        "Number('1e1000') + '|' + Number('abc');",
        "\"Infinity|NaN\"",
    );
    both_answer("1 / Number(-0);", "-Infinity");
    both_answer("typeof Number(1) + '|' + Number(1, 2);", "\"number|1\"");
    both_answer(
        "Number({ valueOf: function () { return '7'; }, toString: function () { return 8; } });",
        "7",
    );
    both_answer("Number({ toString: function () { return '9'; } });", "9");
    both_answer("Number(new Number(3)) + Number(new String('4'));", "7");
    both_throw(
        "Number({ valueOf: function () { throw 'value'; } });",
        "\"value\"",
    );
    both_throw(
        "new Number({ valueOf: function () { throw 'value'; } });",
        "\"value\"",
    );
    both_throw(
        "Number({ valueOf: function () { return {}; }, toString: function () { return {}; } });",
        "TypeError",
    );
    both_answer(
        "var n = new Number(5); typeof n + '|' + n.valueOf() + '|' + \
         (Object.getPrototypeOf(n) === Number.prototype) + '|' + \
         Object.prototype.toString.call(n);",
        "\"object|5|true|[object Number]\"",
    );
    both_answer("new Number().valueOf();", "0");
    both_answer("1 / new Number(-0).valueOf();", "-Infinity");
    both_answer("new Number('0x1f').valueOf();", "31");
    both_answer("new Number(1) == 1 && new Number(1) !== 1;", "true");
    both_answer(
        "Number.length + '|' + Number.name + '|' + \
         (Object.getPrototypeOf(Number) === Object.getPrototypeOf(Object)) + '|' + \
         (Number.prototype.constructor === Number) + '|' + typeof Number;",
        "\"1|Number|true|true|function\"",
    );
    both_answer(
        "Number.prototype.valueOf() + '|' + Object.prototype.toString.call(Number.prototype) + \
         '|' + (Object.getPrototypeOf(Number.prototype) === Object.prototype);",
        "\"0|[object Number]|true\"",
    );
    both_answer(
        "var d = Object.getOwnPropertyDescriptor(Number.prototype, 'constructor'); \
         d.writable + '|' + d.enumerable + '|' + d.configurable;",
        "\"true|false|true\"",
    );

    // OrdinaryCreateFromConstructor takes the new target's "prototype".
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let global_root = global(&mut agent, &realm);
    agent
        .with_cx(&realm, |cx| {
            let global = object(cx, &global_root);
            let number = get(cx, global, "Number");
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
            let made = cx
                .construct(
                    number,
                    &[Value::Number(2.5)],
                    Some(Value::Object(new_target)),
                )
                .unwrap()
                .unwrap();
            let Value::Object(made) = made else {
                panic!("{made:?}");
            };
            let object_constructor = get_object(cx, global, "Object");
            let get_prototype_of = get(cx, object_constructor, "getPrototypeOf");
            assert_eq!(
                cx.call(get_prototype_of, Value::Undefined, &[Value::Object(made)])
                    .unwrap()
                    .unwrap(),
                Value::Object(prototype)
            );
            let number_object = get_object(cx, global, "Number");
            let number_prototype = get_object(cx, number_object, "prototype");
            let value_of = get(cx, number_prototype, "valueOf");
            assert_eq!(
                cx.call(value_of, Value::Object(made), &[])
                    .unwrap()
                    .unwrap(),
                Value::Number(2.5)
            );
        })
        .unwrap();
}

/// `#sec-properties-of-the-number-constructor`: `EPSILON` (2^-52), `MAX_SAFE_INTEGER`
/// (2^53 - 1), `MAX_VALUE`, `MIN_SAFE_INTEGER`, `MIN_VALUE`, `NaN`, `NEGATIVE_INFINITY`,
/// `POSITIVE_INFINITY` and `prototype` are data properties with { [[Writable]]: false,
/// [[Enumerable]]: false, [[Configurable]]: false }: an assignment is ignored in sloppy code and
/// throws a TypeError in strict code, and so does `delete`.
#[test]
fn number_value_properties_are_frozen_data() {
    both_answer(
        "var names = ['EPSILON', 'MAX_SAFE_INTEGER', 'MAX_VALUE', 'MIN_SAFE_INTEGER', \
         'MIN_VALUE', 'NaN', 'NEGATIVE_INFINITY', 'POSITIVE_INFINITY', 'prototype']; \
         var out = []; \
         for (var i = 0; i < names.length; i++) { \
           var d = Object.getOwnPropertyDescriptor(Number, names[i]); \
           out.push(names[i] + ':' + d.writable + d.enumerable + d.configurable + \
             ('value' in d) + ('get' in d)); \
         } \
         out.join();",
        "\"EPSILON:falsefalsefalsetruefalse,MAX_SAFE_INTEGER:falsefalsefalsetruefalse,\
         MAX_VALUE:falsefalsefalsetruefalse,MIN_SAFE_INTEGER:falsefalsefalsetruefalse,\
         MIN_VALUE:falsefalsefalsetruefalse,NaN:falsefalsefalsetruefalse,\
         NEGATIVE_INFINITY:falsefalsefalsetruefalse,POSITIVE_INFINITY:falsefalsefalsetruefalse,\
         prototype:falsefalsefalsetruefalse\"",
    );
    both_answer(
        "(1 + Number.EPSILON !== 1) + '|' + (1 + Number.EPSILON / 2 === 1) + '|' + \
         (Number.EPSILON === 2.220446049250313e-16);",
        "\"true|true|true\"",
    );
    both_answer(
        "(Number.MAX_SAFE_INTEGER === 9007199254740991) + '|' + \
         (Number.MIN_SAFE_INTEGER === -9007199254740991) + '|' + \
         (Number.MAX_SAFE_INTEGER + 2 === 9007199254740992);",
        "\"true|true|true\"",
    );
    both_answer(
        "(Number.MAX_VALUE === 1.7976931348623157e308) + '|' + (Number.MAX_VALUE * 2) + '|' + \
         (Number.MIN_VALUE === 5e-324) + '|' + (Number.MIN_VALUE / 2);",
        "\"true|Infinity|true|0\"",
    );
    both_answer(
        "(Number.NaN !== Number.NaN) + '|' + Number.NEGATIVE_INFINITY + '|' + \
         Number.POSITIVE_INFINITY;",
        "\"true|-Infinity|Infinity\"",
    );
    let (sloppy, strict) = run_both("Number.MAX_VALUE = 1; Number.MAX_VALUE === 1;");
    assert_eq!(sloppy, Outcome::Normal("false".to_string()));
    assert_eq!(strict, Outcome::Throw("TypeError".to_string()));
    let (sloppy, strict) = run_both("var deleted = delete Number.NaN; deleted + '|' + Number.NaN;");
    assert_eq!(sloppy, Outcome::Normal("\"false|NaN\"".to_string()));
    assert_eq!(strict, Outcome::Throw("TypeError".to_string()));
    let (sloppy, strict) = run_both("Number.prototype = {}; typeof Number.prototype.valueOf;");
    assert_eq!(sloppy, Outcome::Normal("\"function\"".to_string()));
    assert_eq!(strict, Outcome::Throw("TypeError".to_string()));
}

/// `#sec-number.prototype.valueof` with `#sec-thisnumbervalue`: a Number this value or the
/// [[NumberData]] of a Number object; anything else, a String or Boolean object and an object
/// that only inherits from `Number.prototype` included, throws a TypeError. Length 0, name
/// "valueOf".
#[test]
fn number_prototype_value_of_needs_a_number() {
    both_answer("(3).valueOf() + Number.prototype.valueOf.call(4);", "7");
    both_answer("Number.prototype.valueOf.call(new Number(7));", "7");
    both_answer("1 / Number.prototype.valueOf.call(-0);", "-Infinity");
    both_answer("Number.prototype.valueOf.call(NaN);", "NaN");
    for this in [
        "'5'",
        "true",
        "undefined",
        "null",
        "{}",
        "[]",
        "new String('1')",
        "new Boolean(true)",
        "{ valueOf: function () { return 1; } }",
        "Number",
    ] {
        both_throw(
            &format!("Number.prototype.valueOf.call({this});"),
            "TypeError",
        );
    }
    both_throw(
        "var o = {}; o.valueOf = Number.prototype.valueOf; o.valueOf();",
        "TypeError",
    );
    both_answer(
        "var v = Number.prototype.valueOf; v.length + '|' + v.name;",
        "\"0|valueOf\"",
    );
    // ToPrimitive of a Number object finds it: arithmetic on the wrapper.
    both_answer("new Number(2) * new Number(3) + 1;", "7");
}

/// `#sec-number.prototype.tostring`: ThisNumberValue first (a TypeError before the radix is
/// read), then the radix: undefined is 10, otherwise SnapToInteger(radix, truncate, 2, 36)
/// (`#sec-snaptointeger`), whose ToNumber may throw, and which throws a RangeError for NaN, ±∞
/// and an integer outside 2 to 36 after truncation; then Number::toString(x, radix)
/// (`#sec-numeric-types-number-tostring`). Length 1, name "toString".
#[test]
fn number_prototype_to_string_radix() {
    both_answer("(255).toString(16);", "\"ff\"");
    both_answer(
        "(255).toString() + '|' + (255).toString(undefined);",
        "\"255|255\"",
    );
    both_answer(
        "(255).toString(10.9) + '|' + (255).toString('16');",
        "\"255|ff\"",
    );
    both_answer(
        "(255).toString(2) + '|' + (255).toString(36.99);",
        "\"11111111|73\"",
    );
    both_answer(
        "(-255).toString(36) + '|' + (0.5).toString(2);",
        "\"-73|0.1\"",
    );
    both_answer(
        "(-0.25).toString(2) + '|' + (35).toString(36);",
        "\"-0.01|z\"",
    );
    both_answer(
        "NaN.toString(2) + '|' + (-0).toString(2) + '|' + Infinity.toString(36) + '|' + \
         (-Infinity).toString(3);",
        "\"NaN|0|Infinity|-Infinity\"",
    );
    both_answer(
        "new Number(255).toString(16) + new Number(1).toString();",
        "\"ff1\"",
    );
    both_answer(
        "(1e21).toString() + '|' + (1e21).toString(10);",
        "\"1e+21|1e+21\"",
    );
    both_answer(
        "var t = (1e21).toString(36); t.length + '|' + t.indexOf('+') + '|' + t.indexOf('.');",
        "\"14|-1|-1\"",
    );
    both_answer(
        "(0.000001).toString(10) + '|' + (1e-7).toString();",
        "\"0.000001|1e-7\"",
    );
    both_answer(
        "(Number.MAX_VALUE).toString(2).length + '|' + (Number.MIN_VALUE).toString(2).length;",
        "\"1024|1076\"",
    );
    both_answer(
        "Number.prototype.toString.call(new Number(10), 3) + '|' + \
         Number.prototype.toString.call(-10, 4);",
        "\"101|-22\"",
    );
    for radix in [
        "1",
        "37",
        "0",
        "-1",
        "1.9",
        "36.5 + 1",
        "Infinity",
        "-Infinity",
        "NaN",
        "'abc'",
    ] {
        both_throw(&format!("(10).toString({radix});"), "RangeError");
    }
    both_throw("(10).toString(null);", "RangeError");
    both_throw(
        "(1).toString({ valueOf: function () { throw 'radix'; } });",
        "\"radix\"",
    );
    both_answer(
        "(31).toString({ valueOf: function () { return 2; } });",
        "\"11111\"",
    );
    both_throw(
        "Number.prototype.toString.call('1', { valueOf: function () { throw 'radix'; } });",
        "TypeError",
    );
    for this in [
        "'1'",
        "{}",
        "undefined",
        "new Boolean(false)",
        "new String('2')",
    ] {
        both_throw(
            &format!("Number.prototype.toString.call({this}, 10);"),
            "TypeError",
        );
    }
    both_answer(
        "var f = Number.prototype.toString; f.length + '|' + f.name;",
        "\"1|toString\"",
    );
    // A Number object converted with the string hint finds it, not Object.prototype.toString.
    both_answer(
        "String(new Number(255)) + '|' + new Number(-1.5);",
        "\"255|-1.5\"",
    );
    both_answer(
        "var o = {}; o[new Number(12)] = 1; Object.getOwnPropertyNames(o).join();",
        "\"12\"",
    );
}

/// The digits of `Number.prototype.toString` are made by a loop that runs for the script, so
/// they are charged to the step budget (`docs/design/engine.md`, decision 4): a step per code
/// unit of the result. Under a budget of 600 steps the 8 digits of `(255).toString(2)` fit and
/// the 1,076 code units of `Number.MIN_VALUE.toString(2)` end the run in
/// `Terminated(StepBudget)`, in sloppy and in strict code.
#[test]
fn number_prototype_to_string_charges_its_digits() {
    let mut limits = support::host::limits();
    limits.steps_per_entry = 600;
    for prologue in ["", "\"use strict\";\n"] {
        assert_eq!(
            run(limits, &format!("{prologue}(255).toString(2);")),
            Outcome::Normal("\"11111111\"".to_string()),
            "{prologue}"
        );
        assert_eq!(
            run(limits, &format!("{prologue}Number.MIN_VALUE.toString(2);")),
            Outcome::Terminated(Termination::StepBudget),
            "{prologue}"
        );
    }
}
