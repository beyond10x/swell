//! The function properties of the global object that slice 2 takes early
//! (`docs/design/engine.md`, section 12, "A part before its slice"): `isNaN`
//! (`#sec-isnan-number`) and `isFinite` (`#sec-isfinite-number`); and the global properties
//! `Number`, `Boolean`, `isNaN` and `isFinite` of SetDefaultGlobalBindings
//! (`#sec-setdefaultglobalbindings`). Scripts run in sloppy and in strict code. Run also with the
//! feature `collect-every-safe-point`.
#![cfg(test)]

mod support;

use support::script::{Outcome, run_both};

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

/// `#sec-isnan-number` and `#sec-isfinite-number`: ToNumber of the argument first (undefined
/// without one; a String through StringToNumber; an object through ToPrimitive with hint
/// Number, whose throw is returned, and whose valueOf runs once), then whether it is NaN, or a
/// finite Number. Each has length 1 and its name, is no constructor, and is the global property
/// of its name with { [[Writable]]: true, [[Enumerable]]: false, [[Configurable]]: true }, as
/// `Number` and `Boolean` are (clause 19).
#[test]
fn is_nan_and_is_finite_convert_first() {
    both_answer(
        "[isNaN(NaN), isNaN(), isNaN(undefined), isNaN('abc'), isNaN({}), isNaN('0x1g'), \
          isNaN(Number.NaN)].join();",
        "\"true,true,true,true,true,true,true\"",
    );
    both_answer(
        "[isNaN(1), isNaN('12'), isNaN(' 12 '), isNaN(''), isNaN(null), isNaN(true), \
          isNaN(Infinity), isNaN(new Number(3)), isNaN(new String(' 7 '))].join();",
        "\"false,false,false,false,false,false,false,false,false\"",
    );
    both_answer(
        "[isFinite(1), isFinite(-0), isFinite('1e308'), isFinite(null), isFinite(''), \
          isFinite(Number.MAX_VALUE), isFinite(new Number(5)), isFinite(false)].join();",
        "\"true,true,true,true,true,true,true,true\"",
    );
    both_answer(
        "[isFinite(), isFinite(NaN), isFinite(Infinity), isFinite(-Infinity), \
          isFinite('1e309'), isFinite('abc'), isFinite({}), isFinite(undefined)].join();",
        "\"false,false,false,false,false,false,false,false\"",
    );
    both_answer(
        "var calls = 0; var o = { valueOf: function () { calls++; return NaN; } }; \
         isNaN(o) + '|' + isFinite(o) + '|' + calls;",
        "\"true|false|2\"",
    );
    both_answer(
        "isNaN({ valueOf: function () { return {}; }, toString: function () { return '5'; } });",
        "false",
    );
    both_throw(
        "isNaN({ valueOf: function () { throw 'value'; } });",
        "\"value\"",
    );
    both_throw(
        "isFinite({ valueOf: function () { throw 'value'; } });",
        "\"value\"",
    );
    both_throw(
        "isFinite({ valueOf: function () { return {}; }, toString: function () { return {}; } });",
        "TypeError",
    );
    both_answer(
        "isNaN.length + '|' + isNaN.name + '|' + isFinite.length + '|' + isFinite.name + '|' + \
         typeof isNaN + '|' + (Object.getPrototypeOf(isNaN) === Object.getPrototypeOf(Object));",
        "\"1|isNaN|1|isFinite|function|true\"",
    );
    both_throw("new isNaN(1);", "TypeError");
    both_throw("new isFinite(1);", "TypeError");
    both_answer(
        "var global = this; var names = ['Number', 'Boolean', 'isNaN', 'isFinite']; var out = []; \
         for (var i = 0; i < names.length; i++) { \
           var d = Object.getOwnPropertyDescriptor(global, names[i]); \
           out.push(names[i] + ':' + d.writable + d.enumerable + d.configurable + \
             (d.value === global[names[i]])); \
         } \
         out.join();",
        "\"Number:truefalsetruetrue,Boolean:truefalsetruetrue,isNaN:truefalsetruetrue,\
         isFinite:truefalsetruetrue\"",
    );
    both_answer(
        "var global = this; var saved = isNaN; isNaN = 1; var replaced = global.isNaN; \
         delete global.isNaN; replaced + '|' + typeof global.isNaN + '|' + saved(NaN);",
        "\"1|undefined|true\"",
    );
}
