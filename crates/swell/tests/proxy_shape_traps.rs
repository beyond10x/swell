//! The Proxy traps of shape (`#sec-proxy-object-internal-methods-and-internal-slots`),
//! `story:proxy-descriptor-prototype-and-extensibility-traps`: `getPrototypeOf` (10.5.1),
//! `setPrototypeOf` (10.5.2), `isExtensible` (10.5.3), `preventExtensions` (10.5.4),
//! `getOwnPropertyDescriptor` (10.5.5), `defineProperty` (10.5.6) and `ownKeys` (10.5.11), with
//! the 21 invariants their clauses check, numbered I1 to I21 as the story numbers them; each
//! broken invariant is a TypeError whose message names the trap, the key where there is one, and
//! `I<n>:`. Each invariant is tested on both of its sides, with `+0` and `-0`, `NaN` and a String
//! built at run time where its comparison reads a value. Then the walk of `for`-`in` over a
//! Proxy, the bounds of `ownKeys` (the step and the memory budget of a huge array-like, the
//! re-entry limit on the promised stack, the interrupt), and the reads of the target at the steps
//! of each clause. Run also with the feature `collect-every-safe-point`.
#![cfg(test)]

use crate::support;

use std::cell::Cell;

use support::host::{agent_with, limits, realm, stack_with_room};
use support::script::{Outcome, define_function, describe, eval, eval_text, run, run_both};
use support::stack::{BUILTINS_REENTRY, on_bounded_stack, reach_through, stack_limits, test_path};
use swell::{
    AgentState, Completion, Cx, HostCall, InterruptHandle, Outcome as HostOutcome, Termination,
    Value,
};

/// The functions every script of this file starts with. `outcome(f, n, trap, key)` runs `f` and
/// answers its value as text (`object`, `symbol`, `-0` for minus zero, `String(v)` else), or for a
/// TypeError whose message holds `I<n>:`, the name of the trap and, when `key` is given,
/// `"<key>"`, the text `I<n>`; any other throw answers its name and message.
const PRELUDE: &str = r#"
function show(v) {
  if (typeof v === "object" && v !== null) return "object";
  if (typeof v === "function") return "function";
  if (typeof v === "symbol") return "symbol";
  if (v === 0 && 1 / v < 0) return "-0";
  return String(v);
}
function outcome(f, n, trap, key) {
  try { return show(f()); }
  catch (e) {
    var m = e.message;
    if (e instanceof TypeError && m.indexOf("I" + n + ":") >= 0 && m.indexOf(trap) >= 0 &&
        (key === undefined || m.indexOf("\"" + key + "\"") >= 0)) return "I" + n;
    return e.name + "(" + m + ")";
  }
}
"#;

/// `source`, after [`PRELUDE`], answers the String `expected` in sloppy and in strict code.
fn answers(source: &str, expected: &str) {
    let source = format!("{PRELUDE}{source}");
    let (sloppy, strict) = run_both(&source);
    let expected = Outcome::Normal(format!("\"{expected}\""));
    assert_eq!(sloppy, expected, "sloppy: {source}");
    assert_eq!(strict, expected, "strict: {source}");
}

/// `source`, after [`PRELUDE`], answers the String `expected` in sloppy code: for a script whose
/// steps do not depend on the code's strictness, run once.
fn answers_once(source: &str, expected: &str) {
    let source = format!("{PRELUDE}{source}");
    let expected = Outcome::Normal(format!("\"{expected}\""));
    assert_eq!(run(limits(), &source), expected, "{source}");
}

/// `getPrototypeOf` (10.5.1) is called with the handler as its this value and « target », and
/// its answer is what `Object.getPrototypeOf`, `Reflect.getPrototypeOf`, `isPrototypeOf` and
/// `instanceof` (OrdinaryHasInstance) read; the target's own prototype is not read for an
/// extensible target.
#[test]
fn the_get_prototype_of_trap_answers_object_get_prototype_of() {
    answers(
        r#"var log = [], t = {}, proto = Object.create(null), h;
        function C() {}
        h = { getPrototypeOf: function (target) {
          log.push(this === h, target === t, arguments.length); return proto; } };
        var p = new Proxy(t, h);
        var r = [Object.getPrototypeOf(p) === proto, Reflect.getPrototypeOf(p) === proto,
          Object.prototype.isPrototypeOf.call(proto, p), Object.prototype.isPrototypeOf(p)];
        proto = C.prototype;
        r.push(p instanceof C, log.length, log.slice(0, 3).join("+"));
        r.join();"#,
        "true,true,true,false,true,15,true+true+1",
    );
}

/// `setPrototypeOf` (10.5.2) is called with « target, V »; ToBoolean of its answer is what
/// `Reflect.setPrototypeOf` answers (`NaN`, `-0` and an empty String built at run time are false,
/// a String built at run time is true), `Object.setPrototypeOf` throws a TypeError on false, and
/// the target is not changed by the call.
#[test]
fn the_set_prototype_of_trap_answers_object_set_prototype_of() {
    answers(
        r#"var log = [], t = {}, v = {}, answer, h;
        h = { setPrototypeOf: function (target, value) {
          log.push(this === h, target === t, value === v, arguments.length); return answer; } };
        var p = new Proxy(t, h), r = [];
        var answers = [false, true, 1, NaN, -0, "", "a".slice(1), "a" + "b"];
        for (var i = 0; i < answers.length; i++) {
          answer = answers[i];
          r.push(Reflect.setPrototypeOf(p, v));
        }
        answer = false;
        try { Object.setPrototypeOf(p, v); r.push("set"); } catch (e) { r.push(e.name); }
        answer = true;
        r.push(Object.setPrototypeOf(p, v) === p, Object.getPrototypeOf(t) === Object.prototype);
        r.push(log.slice(0, 4).join("+"));
        r.join();"#,
        "false,true,true,false,false,false,false,true,TypeError,true,true,true+true+true+2",
    );
}

/// `isExtensible` (10.5.3) is called with « target »; ToBoolean of its answer is what
/// `Object.isExtensible` and `Reflect.isExtensible` answer when it matches the target.
#[test]
fn the_is_extensible_trap_answers_object_is_extensible() {
    answers(
        r#"var log = [], t = {}, answer, h;
        h = { isExtensible: function (target) {
          log.push(this === h, target === t, arguments.length); return answer; } };
        var p = new Proxy(t, h), r = [];
        answer = 1; r.push(Object.isExtensible(p));
        answer = "a" + "b"; r.push(Reflect.isExtensible(p));
        Object.preventExtensions(t);
        answer = 0; r.push(Object.isExtensible(p));
        answer = ""; r.push(Reflect.isExtensible(p), Object.isFrozen(p));
        r.push(log.slice(0, 3).join("+"));
        r.join();"#,
        "true,true,false,false,true,true+true+1",
    );
}

/// `preventExtensions` (10.5.4) is called with « target »; ToBoolean of its answer is what
/// `Reflect.preventExtensions` answers, and `Object.preventExtensions` throws a TypeError on
/// false.
#[test]
fn the_prevent_extensions_trap_answers_object_prevent_extensions() {
    answers(
        r#"var log = [], t = {}, answer, h;
        h = { preventExtensions: function (target) {
          log.push(this === h, target === t, arguments.length);
          if (answer) Object.preventExtensions(target);
          return answer; } };
        var p = new Proxy(t, h), r = [];
        answer = false; r.push(Reflect.preventExtensions(p), Object.isExtensible(t));
        answer = NaN; r.push(Reflect.preventExtensions(p));
        answer = -0;
        try { Object.preventExtensions(p); r.push("prevented"); } catch (e) { r.push(e.name); }
        answer = "x" + "y"; r.push(Reflect.preventExtensions(p), Object.isExtensible(t));
        answer = 1; r.push(Object.preventExtensions(p) === p);
        r.push(log.slice(0, 3).join("+"));
        r.join();"#,
        "false,true,false,TypeError,true,false,true,true+true+1",
    );
}

/// `getOwnPropertyDescriptor` (10.5.5) is called with « target, key », an index key as a String
/// and a Symbol as itself; its answer, completed (CompletePropertyDescriptor), is what
/// `Object.getOwnPropertyDescriptor` answers, and `undefined` answers `undefined`.
#[test]
fn the_get_own_property_descriptor_trap_answers_object_get_own_property_descriptor() {
    answers(
        r#"var log = [], t = {}, s = Symbol("s"), answer, h;
        h = { getOwnPropertyDescriptor: function (target, key) {
          log.push(this === h, target === t, typeof key, String(key), arguments.length);
          return answer; } };
        var p = new Proxy(t, h), r = [];
        answer = { value: 5, configurable: true };
        var d = Object.getOwnPropertyDescriptor(p, "x");
        r.push(d.value, d.writable, d.enumerable, d.configurable, Object.keys(d).join("+"));
        answer = { get: undefined, configurable: true };
        d = Object.getOwnPropertyDescriptor(p, 1);
        r.push("get" in d, d.get, d.set, d.enumerable, Object.keys(d).join("+"));
        answer = undefined;
        r.push(Object.getOwnPropertyDescriptor(p, s), Object.prototype.hasOwnProperty.call(p, "x"));
        r.push(log.slice(0, 5).join("+"), log.slice(5, 10).join("+"), log[12], log[13] === "Symbol(s)");
        r.join();"#,
        "5,false,false,true,value+writable+enumerable+configurable,\
         true,,,false,get+set+enumerable+configurable,,false,\
         true+true+string+x+2,true+true+string+1+2,symbol,true",
    );
}

/// `defineProperty` (10.5.6) is called with « target, key, FromPropertyDescriptor(Desc) », a new
/// ordinary object holding exactly the fields Desc has; ToBoolean of its answer is what
/// `Reflect.defineProperty` answers, `Object.defineProperty` throws on false, and the target is
/// not changed by the call.
#[test]
fn the_define_property_trap_answers_object_define_property() {
    answers(
        r#"var log = [], t = {}, answer, h, seen;
        h = { defineProperty: function (target, key, desc) {
          log.push(this === h, target === t, typeof key, String(key), arguments.length);
          seen = desc; return answer; } };
        var p = new Proxy(t, h), r = [];
        answer = true;
        r.push(Reflect.defineProperty(p, "x", { value: -0, enumerable: true }));
        r.push(Object.keys(seen).join("+"), 1 / seen.value, seen.enumerable,
          Object.getPrototypeOf(seen) === Object.prototype, Object.isExtensible(seen));
        var g = function () {};
        r.push(Reflect.defineProperty(p, 2, { get: g }), Object.keys(seen).join("+"), seen.get === g);
        r.push(Reflect.defineProperty(p, "y", {}), Object.keys(seen).length);
        answer = NaN; r.push(Reflect.defineProperty(p, "x", { value: 1 }));
        answer = "";
        try { Object.defineProperty(p, "x", { value: 1 }); r.push("defined"); } catch (e) { r.push(e.name); }
        answer = "o" + "k"; r.push(Object.defineProperty(p, "x", { value: 1 }) === p);
        r.push(Object.getOwnPropertyNames(t).length, log.slice(0, 5).join("+"), log[8]);
        r.join();"#,
        "true,value+enumerable,-Infinity,true,true,true,true,get,true,true,0,false,TypeError,true,\
         0,true+true+string+x+3,2",
    );
}

/// `ownKeys` (10.5.11) is called with « target »; its answer, an Array or any array-like of
/// Strings and Symbols, is the list `Reflect.ownKeys` answers in its order, and the enumerations
/// read their keys from it: `Object.keys`, `getOwnPropertyNames`, `getOwnPropertySymbols`,
/// `Object.assign` and the spread of an object literal.
#[test]
fn the_own_keys_trap_answers_reflect_own_keys() {
    answers(
        r#"var log = [], s = Symbol("s"), t = { a: 1, b: 2, 1: 3 }, answer, h;
        t[s] = 4;
        h = { ownKeys: function (target) {
          log.push(this === h, target === t, arguments.length); return answer; } };
        var p = new Proxy(t, h), r = [];
        answer = ["b", s, "a", "1"];
        r.push(Reflect.ownKeys(p).map(String).join("+"), Object.keys(p).join("+"),
          Object.getOwnPropertyNames(p).join("+"), Object.getOwnPropertySymbols(p).length);
        var copy = Object.assign({}, p), spread = { ...p };
        r.push(Object.keys(copy).join("+"), copy[s], Object.keys(spread).join("+"));
        answer = { length: 2, 0: "a", 1: "b", 2: "c" };
        r.push(Reflect.ownKeys(p).join("+"));
        r.push(log.slice(0, 3).join("+"));
        r.join();"#,
        "b+Symbol(s)+a+1,b+a+1,b+a+1,1,1+b+a,4,1+b+a,a+b,true+true+1",
    );
}

/// I1 (10.5.1, step 8): the answer of `getPrototypeOf` is neither an Object nor `null`. A
/// number (`+0`, `-0`, `NaN`), a String built at run time, `undefined`, a Boolean and a Symbol
/// break it; `null` and an object do not.
#[test]
fn i01_get_prototype_of_must_answer_an_object_or_null() {
    answers(
        r#"var answer, p = new Proxy({}, { getPrototypeOf: function () { return answer; } });
        var cases = [0, -0, NaN, "pro" + "to", undefined, true, Symbol(), null, {}], r = [];
        for (var i = 0; i < cases.length; i++) {
          answer = cases[i];
          r.push(outcome(function () { return Object.getPrototypeOf(p); }, 1, "getPrototypeOf"));
        }
        r.join();"#,
        "I1,I1,I1,I1,I1,I1,I1,null,object",
    );
}

/// I2 (10.5.1, step 12): for a target that is not extensible the answer must be SameValue with
/// the target's prototype; an extensible target answers any object.
#[test]
fn i02_get_prototype_of_of_a_non_extensible_target_must_answer_its_prototype() {
    answers(
        r#"var proto = {}, other = {}, answer;
        var t = Object.preventExtensions(Object.create(proto));
        var p = new Proxy(t, { getPrototypeOf: function () { return answer; } });
        var e = new Proxy({}, { getPrototypeOf: function () { return answer; } });
        var n = new Proxy(Object.preventExtensions(Object.create(null)),
          { getPrototypeOf: function () { return answer; } });
        function read(o) { return outcome(function () { return Object.getPrototypeOf(o) === answer; }, 2, "getPrototypeOf"); }
        var r = [];
        answer = proto; r.push(read(p));
        answer = other; r.push(read(p), read(e));
        answer = null; r.push(read(p), read(n));
        answer = proto; r.push(read(n));
        r.join();"#,
        "true,I2,true,I2,true,I2",
    );
}

/// I3 (10.5.2, step 12): a true answer for a target that is not extensible requires the value
/// set to be SameValue with the target's prototype; false answers false with no check, and an
/// extensible target answers true.
#[test]
fn i03_set_prototype_of_of_a_non_extensible_target_must_not_change_it() {
    answers(
        r#"var proto = {}, other = {}, answer = true;
        var t = Object.preventExtensions(Object.create(proto));
        var h = { setPrototypeOf: function () { return answer; } };
        var p = new Proxy(t, h), e = new Proxy({}, h);
        function set(o, v) { return outcome(function () { return Reflect.setPrototypeOf(o, v); }, 3, "setPrototypeOf"); }
        var r = [set(p, proto), set(p, other), set(p, null), set(e, other)];
        answer = "a".slice(1); r.push(set(p, other));
        answer = "a" + "b"; r.push(set(p, other));
        r.join();"#,
        "true,I3,I3,true,false,I3",
    );
}

/// I4 (10.5.3, step 9): ToBoolean of the answer of `isExtensible` must be the target's
/// IsExtensible.
#[test]
fn i04_is_extensible_must_match_the_target() {
    answers(
        r#"var answer, t = {};
        var p = new Proxy(t, { isExtensible: function () { return answer; } });
        function read() { return outcome(function () { return Object.isExtensible(p); }, 4, "isExtensible"); }
        var r = [], extensible = [false, 1, NaN, -0, "a" + "b", ""], fixed = [false, 0, NaN, "x" + "", true];
        for (var i = 0; i < extensible.length; i++) { answer = extensible[i]; r.push(read()); }
        Object.preventExtensions(t);
        for (var i = 0; i < fixed.length; i++) { answer = fixed[i]; r.push(read()); }
        r.join();"#,
        "I4,true,I4,I4,true,I4,false,false,false,I4,I4",
    );
}

/// I5 (10.5.4, step 8.b): `preventExtensions` cannot answer true while the target is still
/// extensible; false (`NaN`, `-0` among them) answers false.
#[test]
fn i05_prevent_extensions_cannot_answer_true_for_an_extensible_target() {
    answers(
        r#"var answer, prevent = false, t = {};
        var p = new Proxy(t, { preventExtensions: function (target) {
          if (prevent) Object.preventExtensions(target); return answer; } });
        function run() { return outcome(function () { return Reflect.preventExtensions(p); }, 5, "preventExtensions"); }
        var r = [];
        answer = true; r.push(run());
        answer = "s" + "t"; r.push(run());
        answer = NaN; r.push(run());
        answer = -0; r.push(run());
        prevent = true; answer = 1; r.push(run(), Object.isExtensible(t));
        r.join();"#,
        "I5,I5,false,false,true,false",
    );
}

/// I6 (10.5.5, step 8): the answer of `getOwnPropertyDescriptor` is neither an Object nor
/// `undefined`.
#[test]
fn i06_get_own_property_descriptor_must_answer_an_object_or_undefined() {
    answers(
        r#"var answer, p = new Proxy({}, { getOwnPropertyDescriptor: function () { return answer; } });
        var cases = [0, -0, NaN, "a" + "b", null, true, Symbol(), undefined, { configurable: true }], r = [];
        for (var i = 0; i < cases.length; i++) {
          answer = cases[i];
          r.push(outcome(function () { return Object.getOwnPropertyDescriptor(p, "x"); }, 6, "getOwnPropertyDescriptor", "x"));
        }
        r.join();"#,
        "I6,I6,I6,I6,I6,I6,I6,undefined,object",
    );
}

/// I7 (10.5.5, step 10.b): `undefined` cannot hide a non-configurable own property of the
/// target; it answers `undefined` for a configurable or an absent one.
#[test]
fn i07_get_own_property_descriptor_cannot_hide_a_non_configurable_property() {
    answers(
        r#"var t = { c: 1 };
        Object.defineProperty(t, "n", { value: 1 });
        Object.defineProperty(t, "a" + "b", { value: 2 });
        var p = new Proxy(t, { getOwnPropertyDescriptor: function () { return undefined; } });
        function read(k) { return outcome(function () { return Object.getOwnPropertyDescriptor(p, k); }, 7, "getOwnPropertyDescriptor", k); }
        [read("n"), read("ab"), read("c"), read("absent")].join();"#,
        "I7,I7,undefined,undefined",
    );
}

/// I8 (10.5.5, step 10.d): `undefined` cannot hide an own property of a target that is not
/// extensible; for an extensible target it can.
#[test]
fn i08_get_own_property_descriptor_cannot_hide_a_property_of_a_non_extensible_target() {
    answers(
        r#"var h = { getOwnPropertyDescriptor: function () { return undefined; } };
        var open = new Proxy({ x: 1 }, h), closed = new Proxy(Object.preventExtensions({ x: 1 }), h);
        function read(o, k) { return outcome(function () { return Object.getOwnPropertyDescriptor(o, k); }, 8, "getOwnPropertyDescriptor", k); }
        [read(closed, "x"), read(open, "x"), read(closed, "absent")].join();"#,
        "I8,undefined,undefined",
    );
}

/// I9 (10.5.5, steps 14 and 15): the completed answer must be compatible with the target's
/// property by IsCompatiblePropertyDescriptor: a property the target lacks cannot be reported for
/// a target that is not extensible, and a non-configurable, non-writable property's value is
/// compared by SameValue (`+0` and `-0` differ, `NaN` is itself, a String built at run time equals
/// its literal).
#[test]
fn i09_get_own_property_descriptor_must_be_compatible_with_the_target() {
    answers(
        r#"var answer, t = {};
        Object.defineProperty(t, "z", { value: -0 });
        Object.defineProperty(t, "nan", { value: NaN });
        Object.defineProperty(t, "s", { value: "ab" });
        Object.defineProperty(t, "e", { value: 1, enumerable: true });
        var p = new Proxy(t, { getOwnPropertyDescriptor: function () { return answer; } });
        var closed = new Proxy(Object.preventExtensions({}), { getOwnPropertyDescriptor: function () { return answer; } });
        function read(o, k, a) { answer = a; return outcome(function () { return show(Object.getOwnPropertyDescriptor(o, k).value); }, 9, "getOwnPropertyDescriptor", k); }
        function fixed(v) { return { value: v, writable: false, enumerable: false, configurable: false }; }
        [read(p, "z", fixed(0)), read(p, "z", fixed(-0)), read(p, "nan", fixed(NaN)),
         read(p, "s", fixed("a" + "b")), read(p, "s", fixed("a" + "c")),
         read(p, "e", fixed(1)), read(p, "e", { value: 1, enumerable: true }),
         read(closed, "x", { value: 1, configurable: true })].join();"#,
        "I9,-0,NaN,ab,I9,I9,1,I9",
    );
}

/// I10 (10.5.5, step 16.a): a non-configurable answer requires a non-configurable own property
/// of the target; an absent or a configurable one breaks it.
#[test]
fn i10_get_own_property_descriptor_cannot_report_non_configurable_for_an_absent_or_configurable_property()
 {
    answers(
        r#"var answer, t = { c: 1 };
        Object.defineProperty(t, "n", { value: 1, writable: true });
        var p = new Proxy(t, { getOwnPropertyDescriptor: function () { return answer; } });
        function read(k, a) { answer = a; return outcome(function () { return Object.getOwnPropertyDescriptor(p, k).configurable; }, 10, "getOwnPropertyDescriptor", k); }
        [read("absent", { value: 1 }), read("c", { value: 1, writable: true, enumerable: true }),
         read("n", { value: 1, writable: true }), read("c", { value: 1, configurable: true })].join();"#,
        "I10,I10,false,true",
    );
}

/// I11 (10.5.5, step 16.b): a non-configurable, non-writable answer requires the target's
/// property to be non-writable too; a writable answer of any value is compatible with a
/// writable property (`-0` for `+0`).
#[test]
fn i11_get_own_property_descriptor_cannot_report_non_writable_for_a_writable_property() {
    answers(
        r#"var answer, t = {};
        Object.defineProperty(t, "w", { value: 0, writable: true });
        Object.defineProperty(t, "r", { value: 0 });
        var p = new Proxy(t, { getOwnPropertyDescriptor: function () { return answer; } });
        function read(k, a) { answer = a; return outcome(function () { return show(Object.getOwnPropertyDescriptor(p, k).value); }, 11, "getOwnPropertyDescriptor", k); }
        [read("w", { value: 0 }), read("w", { value: -0, writable: true }),
         read("r", { value: 0 })].join();"#,
        "I11,-0,0",
    );
}

/// I12 (10.5.6, step 14.a): after a true answer, a target that lacks the property and is not
/// extensible breaks it; an extensible target does not, and a false answer (`NaN`) checks
/// nothing.
#[test]
fn i12_define_property_cannot_add_to_a_non_extensible_target() {
    answers(
        r#"var answer = true, h = { defineProperty: function () { return answer; } };
        var closed = new Proxy(Object.preventExtensions({}), h), open = new Proxy({}, h);
        function define(o, k) { return outcome(function () { return Reflect.defineProperty(o, k, { value: 1, configurable: true }); }, 12, "defineProperty", k); }
        var r = [define(closed, "x"), define(open, "x")];
        answer = "a" + "b"; r.push(define(closed, "y"));
        answer = NaN; r.push(define(closed, "x"));
        r.join();"#,
        "I12,true,I12,false",
    );
}

/// I13 (10.5.6, step 14.b): a descriptor with `configurable: false` cannot be reported as
/// defined for a property the target lacks; `configurable: true` or no such field can.
#[test]
fn i13_define_property_cannot_report_a_non_configurable_definition_the_target_lacks() {
    answers(
        r#"var p = new Proxy({}, { defineProperty: function () { return true; } });
        function define(k, d) { return outcome(function () { return Reflect.defineProperty(p, k, d); }, 13, "defineProperty", k); }
        [define("x", { value: 1, configurable: false }), define("x", { value: 1, configurable: true }),
         define("x", { value: 1 })].join();"#,
        "I13,true,true",
    );
}

/// I14 (10.5.6, step 15.a): the descriptor must be compatible with the target's property by
/// IsCompatiblePropertyDescriptor; the value of a non-configurable, non-writable property is
/// compared by SameValue (`-0` against `+0`, `NaN`, a String built at run time).
#[test]
fn i14_define_property_must_be_compatible_with_the_target() {
    answers(
        r#"var t = {};
        Object.defineProperty(t, "z", { value: -0 });
        Object.defineProperty(t, "nan", { value: NaN });
        Object.defineProperty(t, "s", { value: "ab" });
        var p = new Proxy(t, { defineProperty: function () { return true; } });
        function define(k, d) { return outcome(function () { return Reflect.defineProperty(p, k, d); }, 14, "defineProperty", k); }
        [define("z", { value: 0 }), define("z", { value: -0 }), define("nan", { value: NaN }),
         define("s", { value: "a" + "b" }), define("s", { value: "a" + "c" }),
         define("s", { enumerable: true }), define("s", { writable: true })].join();"#,
        "I14,true,true,true,I14,I14,I14",
    );
}

/// I15 (10.5.6, step 15.b): `configurable: false` cannot be reported for a configurable property
/// of the target; for a non-configurable one it can.
#[test]
fn i15_define_property_cannot_report_non_configurable_for_a_configurable_property() {
    answers(
        r#"var t = { c: 1 };
        Object.defineProperty(t, "n", { value: 1 });
        var p = new Proxy(t, { defineProperty: function () { return true; } });
        function define(k, d) { return outcome(function () { return Reflect.defineProperty(p, k, d); }, 15, "defineProperty", k); }
        [define("c", { value: 1, configurable: false }), define("c", { configurable: false }),
         define("n", { configurable: false })].join();"#,
        "I15,I15,true",
    );
}

/// I16 (10.5.6, step 15.c): `writable: false` cannot be reported for a non-configurable,
/// writable data property of the target; `writable: true` or a value alone can.
#[test]
fn i16_define_property_cannot_report_non_writable_for_a_writable_non_configurable_property() {
    answers(
        r#"var t = {};
        Object.defineProperty(t, "w", { value: 0, writable: true });
        var p = new Proxy(t, { defineProperty: function () { return true; } });
        function define(k, d) { return outcome(function () { return Reflect.defineProperty(p, k, d); }, 16, "defineProperty", k); }
        [define("w", { writable: false }), define("w", { value: -0, writable: false }),
         define("w", { writable: true }), define("w", { value: -0 })].join();"#,
        "I16,I16,true,true",
    );
}

/// I17 (10.5.11, step 8, CreateListFromArrayLike with property-key): an element of the answer
/// that is neither a String nor a Symbol (`+0`, `-0`, `NaN`, `null`, `undefined`, an object, a
/// Boolean) breaks it; a String built at run time and a Symbol do not. A non-object answer is a
/// TypeError of CreateListFromArrayLike.
#[test]
fn i17_own_keys_rejects_an_element_that_is_not_a_string_or_symbol() {
    answers(
        r#"var answer, p = new Proxy({}, { ownKeys: function () { return answer; } });
        var cases = [[0], [-0], [NaN], ["a", null], [undefined], [{}], [true]], r = [];
        for (var i = 0; i < cases.length; i++) {
          answer = cases[i];
          r.push(outcome(function () { return Reflect.ownKeys(p); }, 17, "ownKeys"));
        }
        answer = ["a" + "b", Symbol()];
        r.push(Reflect.ownKeys(p).length);
        answer = "ab";
        try { Reflect.ownKeys(p); r.push("listed"); } catch (e) { r.push(e.name); }
        r.join();"#,
        "I17,I17,I17,I17,I17,I17,I17,2,TypeError",
    );
}

/// I18 (10.5.11, step 9): a repeated key breaks it, a String built at run time and its literal
/// being one key; distinct keys do not.
#[test]
fn i18_own_keys_rejects_a_repeated_key() {
    answers(
        r#"var answer, s = Symbol(), p = new Proxy({}, { ownKeys: function () { return answer; } });
        var cases = [["a", "a"], ["a" + "b", "ab"], [s, s], ["0", "1", String(0)], ["a", "b", s, Symbol()]], r = [];
        var keys = ["a", "ab", undefined, "0", undefined];
        for (var i = 0; i < cases.length; i++) {
          answer = cases[i];
          r.push(outcome(function () { return Reflect.ownKeys(p).length; }, 18, "ownKeys", keys[i]));
        }
        r.join();"#,
        "I18,I18,I18,I18,4",
    );
}

/// I19 (10.5.11, step 19): every non-configurable own key of the target must be in the answer,
/// as a literal or a String built at run time; an extensible target allows other keys.
#[test]
fn i19_own_keys_must_list_every_non_configurable_key() {
    answers(
        r#"var answer, t = { c: 1 };
        Object.defineProperty(t, "n", { value: 1 });
        var p = new Proxy(t, { ownKeys: function () { return answer; } });
        function keys(a) { answer = a; return outcome(function () { return Reflect.ownKeys(p).join("+"); }, 19, "ownKeys", "n"); }
        [keys([]), keys(["c"]), keys(["n"]), keys([String.fromCharCode(110), "extra"])].join();"#,
        "I19,I19,n,n+extra",
    );
}

/// I20 (10.5.11, step 21): for a target that is not extensible every configurable own key must
/// be in the answer.
#[test]
fn i20_own_keys_of_a_non_extensible_target_must_list_every_key() {
    answers(
        r#"var answer, t = Object.preventExtensions({ a: 1, b: 2 });
        var p = new Proxy(t, { ownKeys: function () { return answer; } });
        function keys(a, k) { answer = a; return outcome(function () { return Reflect.ownKeys(p).join("+"); }, 20, "ownKeys", k); }
        [keys(["a"], "b"), keys([], "a"), keys(["b", "a"]), keys(["a", "b".concat("")])].join();"#,
        "I20,I20,b+a,a+b",
    );
}

/// I21 (10.5.11, step 22): for a target that is not extensible the answer may hold no key the
/// target lacks.
#[test]
fn i21_own_keys_of_a_non_extensible_target_must_list_no_other_key() {
    answers(
        r#"var answer, s = Symbol(), t = Object.preventExtensions({ a: 1 });
        var p = new Proxy(t, { ownKeys: function () { return answer; } });
        function keys(a, k) { answer = a; return outcome(function () { return Reflect.ownKeys(p).length; }, 21, "ownKeys", k); }
        [keys(["a", "x" + "y"], "xy"), keys([s, "a"]), keys(["a"])].join();"#,
        "I21,I21,1",
    );
}

/// `for`-`in` over a Proxy (EnumerateObjectProperties, `#sec-enumerate-object-properties`, as
/// test262's `for-in` cases read it): `ownKeys` of the Proxy, then `getOwnPropertyDescriptor`
/// for each String key in the order of the answer, then `getPrototypeOf`, and the walk goes on
/// with the prototype the trap answered, whose `c` is visited though the target has no such
/// prototype. Before each key's turn [[GetOwnProperty]] of each object of the chain the walk
/// recorded, in order, reads whether it is still a property ("A property that is deleted before
/// it is processed is ignored"); no `has` trap is called, and `getPrototypeOf` only by the walk.
#[test]
fn for_in_over_a_proxy_calls_own_keys_and_get_own_property_descriptor_in_order() {
    answers(
        r#"var log = [], s = Symbol();
        var proto = { c: 5, a: 6 };
        var t = {}; t.a = 1; t.b = 2; t.hidden = 3;
        t[s] = 4;
        Object.defineProperty(t, "hidden", { enumerable: false });
        var p = new Proxy(t, {
          ownKeys: function (target) { log.push("ownKeys"); return ["b", s, "hidden", "a"]; },
          getOwnPropertyDescriptor: function (target, k) { log.push("gopd:" + String(k)); return Reflect.getOwnPropertyDescriptor(target, k); },
          getPrototypeOf: function () { log.push("getPrototypeOf"); return proto; }
        });
        for (var k in p) log.push("body:" + k);
        log.join();"#,
        "ownKeys,gopd:b,gopd:hidden,gopd:a,getPrototypeOf,\
         gopd:b,body:b,gopd:a,body:a,gopd:c,body:c",
    );
}

/// The step budget of the huge array-like test: 10,000 steps per entry, 2,000 with the feature
/// `collect-every-safe-point`.
const HUGE_STEPS: u64 = if cfg!(feature = "collect-every-safe-point") {
    2_000
} else {
    10_000
};

/// The array-like of length 2^32 - 1 whose index getter (a `get` trap) answers a fresh String
/// per index.
const HUGE: &str = "var huge = new Proxy({ length: 4294967295 }, { get: function (t, k) {\n\
     return k === \"length\" ? t.length : \"k\" + k; } });\n\
     var p = new Proxy({}, { ownKeys: function () { return huge; } });\n";

/// An `ownKeys` trap that answers an array-like of length 2^32 - 1 with a getter per index ends
/// in `Terminated(StepBudget)` (CreateListFromArrayLike spends a step per element before it reads
/// it), having spent at most `steps_per_entry`, and the agent runs the next entry.
#[test]
fn an_own_keys_trap_answering_a_huge_array_like_ends_at_the_step_budget() {
    let mut budget = limits();
    budget.steps_per_entry = HUGE_STEPS;
    let mut agent = agent_with(budget).unwrap();
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(&mut agent, &realm, &format!("{HUGE}Reflect.ownKeys(p);")),
        Outcome::Terminated(Termination::StepBudget)
    );
    let spent = agent.last_entry_steps().unwrap();
    println!("{spent} steps of {HUGE_STEPS}");
    assert!(spent <= HUGE_STEPS, "{spent} steps");
    assert_eq!(agent.state(), AgentState::Alive);
    assert_eq!(
        eval_text(&mut agent, &realm, "1 + 1;"),
        Outcome::Normal("2".to_string())
    );
}

/// The memory a run of the memory test has beyond the reading before it: 256 KiB.
const HUGE_ROOM: u64 = 1 << 18;

/// A collection, which must complete.
fn collect(agent: &mut swell::Agent<support::host::TestHost>) {
    assert!(matches!(agent.collect().unwrap(), HostOutcome::Normal(_)));
}

/// An `ownKeys` trap answering a String object of `HUGE_ROOM / 8` code units `"k"` (an
/// array-like whose elements are read with no call, so no safe point falls between them), with
/// steps to spare and [`HUGE_ROOM`] of memory over the reading of a realm that ran the script
/// once over a short one: the String fits the budget, the list of the answer, a `Value` per
/// element charged before it grows, does not, and the run ends in `Terminated(MemoryBudget)`
/// with `heap_bytes()` within the budget, before the check of repeated keys (I18) that a String
/// of two units reaches. Once the host released the script's values, one `collect()` brings
/// `heap_bytes()` back within the realm's reading before the script, read after two rounds
/// grew the arenas (the third of three rounds against the second), and `live_bytes()` is the
/// same after each round. The readings are printed.
#[test]
fn an_own_keys_trap_answering_a_huge_array_like_ends_at_the_memory_budget() {
    let script = |length: u64| {
        format!(
            "(function () {{ var like = new String(\"k\".repeat({length}));\n\
             var p = new Proxy({{}}, {{ ownKeys: function () {{ return like; }} }});\n\
             try {{ return Reflect.ownKeys(p).length; }} catch (e) {{ return e.name; }} }})();"
        )
    };
    let mut budget = limits();
    budget.steps_per_entry = 1 << 40;
    let mut probe = agent_with(budget).unwrap();
    let probe_realm = realm(&mut probe);
    assert_eq!(
        eval_text(&mut probe, &probe_realm, &script(1)),
        Outcome::Normal("1".to_string())
    );
    collect(&mut probe);
    budget.memory_bytes = probe.heap_bytes().unwrap() + HUGE_ROOM;
    let mut agent = agent_with(budget).unwrap();
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(&mut agent, &realm, &script(2)),
        Outcome::Normal("\"TypeError\"".to_string())
    );
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            &format!("new String(\"k\".repeat({})).length;", HUGE_ROOM / 8)
        ),
        Outcome::Normal((HUGE_ROOM / 8).to_string())
    );
    // Rounds of the script, each read at its termination and after one collection: the first
    // two grow the arenas to what a round needs (their capacity stays in `heap_bytes()`), the
    // third is compared with the second; `live_bytes()` after each is the same.
    let mut huge = || {
        let outcome = eval(&mut agent, &realm, &script(HUGE_ROOM / 8));
        assert!(
            matches!(outcome, HostOutcome::Terminated(Termination::MemoryBudget)),
            "{outcome:?}"
        );
        let terminated = agent.heap_bytes().unwrap();
        drop(outcome);
        collect(&mut agent);
        [
            terminated,
            agent.heap_bytes().unwrap(),
            agent.live_bytes().unwrap(),
        ]
    };
    let rounds = [huge(), huge(), huge()];
    println!(
        "heap_bytes at the termination, after one collection, live_bytes: {rounds:?}; budget {}",
        budget.memory_bytes
    );
    for [terminated, _, _] in rounds {
        assert!(
            terminated <= budget.memory_bytes,
            "{terminated} at the termination"
        );
    }
    let [_, before, live] = rounds[1];
    let [_, after, _] = rounds[2];
    assert!(after <= before, "{before} before, {after} after");
    assert!(rounds.iter().all(|round| round[2] == live), "{rounds:?}");
    assert_eq!(agent.state(), AgentState::Alive);
    assert_eq!(
        eval_text(&mut agent, &realm, &script(1)),
        Outcome::Normal("1".to_string())
    );
}

/// The body of [`the_own_keys_trap_reaches_the_reentry_limit_on_the_promised_stack`].
fn reach_through_own_keys_trap() {
    reach_through(
        "Object.keys(new Proxy({}, { ownKeys() { f(); return []; } }))",
        BUILTINS_REENTRY,
    );
}

/// `Limits::min_stack_bytes` through the `ownKeys` trap (`tests/support/stack.rs`): on seven
/// eighths of `min_stack_bytes()` a script reaches `reentry_depth` re-entries through
/// `Object.keys` of a Proxy whose `ownKeys` trap calls `f`, and parses 1,000 nested brackets at
/// the deepest, without a stack overflow.
#[test]
fn the_own_keys_trap_reaches_the_reentry_limit_on_the_promised_stack() {
    let stack = stack_with_room(stack_limits(BUILTINS_REENTRY));
    on_bounded_stack(
        test_path!("the_own_keys_trap_reaches_the_reentry_limit_on_the_promised_stack"),
        stack,
        reach_through_own_keys_trap,
    )
    .unwrap();
}

thread_local! {
    /// The handle `interruptAnswer` sets.
    static HANDLE: Cell<Option<InterruptHandle>> = const { Cell::new(None) };
}

/// A host function that sets its agent's interrupt flag and answers its first argument.
fn interrupt_answer(
    _cx: &mut Cx<'_, support::host::TestHost>,
    call: HostCall<'_>,
) -> Completion<Value> {
    HANDLE.with(|slot| {
        if let Some(handle) = slot.take() {
            handle.interrupt();
        }
    });
    Ok(call.args.first().copied().unwrap_or(Value::Undefined))
}

/// An interrupt set by the `ownKeys` trap itself, a host function that answers its target (an
/// array-like of two keys), lands between the trap's answer and the checks of the clause: the
/// run ends in `Terminated(Interrupted)` (`docs/design/host-interface.md`, section 10), the
/// statement after it does not run, the agent runs the next entry, and no Proxy count is left
/// behind (a trapless chain of `reentry_depth` proxies answers, one of `reentry_depth + 1`
/// throws the RangeError, as `full_bound` of `tests/proxy.rs` reads it).
#[test]
fn an_own_keys_trap_ends_at_an_interrupt() {
    let mut budget = limits();
    budget.reentry_depth = 20;
    let mut agent = agent_with(budget).unwrap();
    let realm = realm(&mut agent);
    define_function(
        &mut agent,
        &realm,
        "interruptAnswer",
        1,
        interrupt_answer,
        false,
    );
    HANDLE.with(|slot| slot.set(Some(agent.interrupt_handle())));
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var after = 0;\n\
             var p = new Proxy({ length: 2, 0: \"a\", 1: \"b\" }, { ownKeys: interruptAnswer });\n\
             var inner = new Proxy(p, {}); var outer = new Proxy(inner, {});\n\
             Reflect.ownKeys(outer);\n\
             after = 1;"
        ),
        Outcome::Terminated(Termination::Interrupted)
    );
    assert_eq!(agent.state(), AgentState::Alive);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "function c(n) { var q = { x: 1 }; for (var i = 0; i < n; i++) q = new Proxy(q, {}); return q; }\n\
             var a = [after];\n\
             try { a.push(c(20).x); } catch (e) { a.push(e.name); }\n\
             try { a.push(c(21).x); } catch (e) { a.push(e.name); }\n\
             a.join();"
        ),
        Outcome::Normal("\"0,1,RangeError\"".to_string())
    );
}

/// Each of the 13 internal methods reads the target's facts only at the steps of its clause and
/// under their conditions (10.5.1 to 10.5.11). The target `t` is itself a Proxy whose handler
/// defines all 13 traps, each pushing its name to `log` and forwarding through `Reflect`; the
/// outer Proxy's handler defines the trap under test; `log` is emptied before each operation
/// and compared whole after it, so a read too many or too early fails. No step of these
/// clauses reads the strictness of the code, so the script runs once, in sloppy code.
#[test]
fn the_target_is_read_only_at_the_steps_of_each_clause() {
    answers_once(
        r#"var log = [];
        var logger = {};
        ["getPrototypeOf", "setPrototypeOf", "isExtensible", "preventExtensions",
         "getOwnPropertyDescriptor", "defineProperty", "has", "get", "set", "deleteProperty",
         "ownKeys", "apply", "construct"].forEach(function (name) {
          logger[name] = function () { log.push(name); return Reflect[name].apply(null, arguments); };
        });
        function target(closed) {
          var plain = { a: 1, b: 2 };
          if (closed) Object.preventExtensions(plain);
          return new Proxy(plain, logger);
        }
        function fixedTarget() {
          var plain = {};
          Object.defineProperty(plain, "n", { value: 1 });
          return new Proxy(plain, logger);
        }
        var r = [];
        function op(name, t, trap, f) {
          var h = {}; h[name] = trap;
          var o = new Proxy(t, h);
          log.length = 0;
          var v = outcome(function () { return f(o); });
          r.push(name + ":" + (v.indexOf("TypeError") === 0 ? "TypeError" : v) + ":[" + log.join("+") + "]");
        }
        function answer(v) { return function () { return v; }; }
        var gp = function (o) { return Reflect.getPrototypeOf(o); };
        op("getPrototypeOf", target(), answer(1), gp);
        op("getPrototypeOf", target(), answer(Object.prototype), gp);
        op("getPrototypeOf", target(true), answer(Object.prototype), gp);
        var sp = function (o) { return Reflect.setPrototypeOf(o, Object.prototype); };
        op("setPrototypeOf", target(), answer(false), sp);
        op("setPrototypeOf", target(), answer(true), sp);
        op("setPrototypeOf", target(true), answer(true), sp);
        op("isExtensible", target(), answer(true), function (o) { return Reflect.isExtensible(o); });
        var pe = function (o) { return Reflect.preventExtensions(o); };
        op("preventExtensions", target(), answer(false), pe);
        op("preventExtensions", target(true), answer(true), pe);
        var gd = function (k) { return function (o) { return Reflect.getOwnPropertyDescriptor(o, k); }; };
        op("getOwnPropertyDescriptor", target(), answer(1), gd("a"));
        op("getOwnPropertyDescriptor", target(), answer(undefined), gd("absent"));
        op("getOwnPropertyDescriptor", fixedTarget(), answer(undefined), gd("n"));
        op("getOwnPropertyDescriptor", target(), answer(undefined), gd("a"));
        op("getOwnPropertyDescriptor", target(), answer({ get configurable() { log.push("configurable"); return true; }, value: 1 }), gd("a"));
        var dp = function (o) { return Reflect.defineProperty(o, "a", { value: 1 }); };
        op("defineProperty", target(), answer(false), dp);
        op("defineProperty", target(), answer(true), dp);
        var ok = function (o) { return Reflect.ownKeys(o).join("+"); };
        op("ownKeys", target(), answer([1]), ok);
        op("ownKeys", target(), answer(["a", "a"]), ok);
        op("ownKeys", target(), answer(["a", "b"]), ok);
        var has = function (k) { return function (o) { return Reflect.has(o, k); }; };
        op("has", target(), answer(true), has("a"));
        op("has", target(), answer(false), has("absent"));
        op("has", fixedTarget(), answer(false), has("n"));
        op("has", target(), answer(false), has("a"));
        op("get", target(), answer(1), function (o) { return o.a; });
        var set = function (o) { return Reflect.set(o, "a", 1); };
        op("set", target(), answer(false), set);
        op("set", target(), answer(true), set);
        var del = function (k) { return function (o) { return Reflect.deleteProperty(o, k); }; };
        op("deleteProperty", target(), answer(false), del("a"));
        op("deleteProperty", target(), answer(true), del("absent"));
        op("deleteProperty", fixedTarget(), answer(true), del("n"));
        op("deleteProperty", target(), answer(true), del("a"));
        r.join(" ");"#,
        "getPrototypeOf:TypeError:[] \
         getPrototypeOf:object:[isExtensible] \
         getPrototypeOf:object:[isExtensible+getPrototypeOf] \
         setPrototypeOf:false:[] \
         setPrototypeOf:true:[isExtensible] \
         setPrototypeOf:true:[isExtensible+getPrototypeOf] \
         isExtensible:true:[isExtensible] \
         preventExtensions:false:[] \
         preventExtensions:true:[isExtensible] \
         getOwnPropertyDescriptor:TypeError:[] \
         getOwnPropertyDescriptor:undefined:[getOwnPropertyDescriptor] \
         getOwnPropertyDescriptor:TypeError:[getOwnPropertyDescriptor] \
         getOwnPropertyDescriptor:undefined:[getOwnPropertyDescriptor+isExtensible] \
         getOwnPropertyDescriptor:object:[getOwnPropertyDescriptor+isExtensible+configurable] \
         defineProperty:false:[] \
         defineProperty:true:[getOwnPropertyDescriptor+isExtensible] \
         ownKeys:TypeError:[] \
         ownKeys:TypeError:[] \
         ownKeys:a+b:[isExtensible+ownKeys+getOwnPropertyDescriptor+getOwnPropertyDescriptor] \
         has:true:[] \
         has:false:[getOwnPropertyDescriptor] \
         has:TypeError:[getOwnPropertyDescriptor] \
         has:false:[getOwnPropertyDescriptor+isExtensible] \
         get:1:[getOwnPropertyDescriptor] \
         set:false:[] \
         set:true:[getOwnPropertyDescriptor] \
         deleteProperty:false:[] \
         deleteProperty:true:[getOwnPropertyDescriptor] \
         deleteProperty:TypeError:[getOwnPropertyDescriptor] \
         deleteProperty:true:[getOwnPropertyDescriptor+isExtensible]",
    );
}

/// [`answers_once`] with steps for [`JUNK`], whose String in the default mode spends a step per
/// code unit it copies.
fn answers_with_junk(source: &str, expected: &str) {
    let mut budget = limits();
    budget.steps_per_entry = 1 << 40;
    let source = format!("{PRELUDE}{JUNK}{FRESH}{source}");
    let expected = Outcome::Normal(format!("\"{expected}\""));
    assert_eq!(run(budget, &source), expected, "{source}");
}

/// `junk()`, called first in a trap, makes a collection run inside that trap in both modes, so
/// that a value the engine holds only in a Rust local across the trap is freed and its next use
/// ends in `Terminated(EngineFault)` or a wrong answer. With `collect-every-safe-point` every safe
/// point collects, and the loop passes a few. In the default mode it makes and drops Strings of
/// up to 2^19 code units by doubling (2 MiB in all, past the 1 MiB threshold `MIN_TRIGGER_BYTES`,
/// which is above the live reading of a test realm), passing a safe point after each, at which
/// the collection that falls due runs.
const JUNK: &str = if cfg!(feature = "collect-every-safe-point") {
    "function junk() { var n = 0; for (var j = 0; j < 3; j++) n = n + j; return n; }\n"
} else {
    "function junk() {\n\
       var s = \"xy\";\n\
       for (var i = 0; i < 18; i++) s = s + s;\n\
       s = null;\n\
       var n = 0;\n\
       for (var j = 0; j < 3; j++) n = n + j;\n\
       return n;\n\
     }\n"
};

/// `fresh(depth)` is a Proxy over an empty extensible target whose every trap runs `junk()` and
/// answers what it makes there, held by nothing else: `ownKeys` new Strings `d<depth>a` and
/// `d<depth>b` and a new Symbol; `getOwnPropertyDescriptor` a new descriptor object, enumerable,
/// whose value is a new String, for those keys (`mine`) and for any Symbol; `getPrototypeOf` a
/// new `fresh(depth - 1)`, `null` at depth 0; `has` and `get` answer for those keys and ask a new
/// `fresh(depth - 1)` for others; `get` answers `g<key>`, or with `descs` a new descriptor object
/// `{ value: "w<key>", enumerable: true }`; `defineProperty`, `deleteProperty` and
/// `setPrototypeOf` answer true, `isExtensible` true and `preventExtensions` false. Every answer
/// keeps the invariants of an extensible target that has no property.
const FRESH: &str = r#"
function fresh(depth, descs) {
  function mine(k) { return typeof k === "symbol" || String(k).slice(0, 2) === "d" + depth; }
  return new Proxy({}, {
    ownKeys: function () { junk(); return ["d" + depth + "a", "d" + depth + "b", Symbol("s" + depth)]; },
    getOwnPropertyDescriptor: function (t, k) {
      junk();
      return mine(k) ? { value: "v" + String(k), writable: true, enumerable: true, configurable: true } : undefined;
    },
    getPrototypeOf: function () { junk(); return depth > 0 ? fresh(depth - 1, descs) : null; },
    has: function (t, k) { junk(); return mine(k) || (depth > 0 && k in fresh(depth - 1, descs)); },
    get: function (t, k, r) {
      junk();
      if (!mine(k)) return depth > 0 ? Reflect.get(fresh(depth - 1, descs), k, r) : undefined;
      return descs ? { value: "w" + String(k), enumerable: true } : "g" + String(k);
    },
    defineProperty: function () { junk(); return true; },
    deleteProperty: function () { junk(); return true; },
    isExtensible: function () { junk(); return true; },
    preventExtensions: function () { junk(); return false; },
    setPrototypeOf: function () { junk(); return true; }
  });
}
"#;

/// `JSON.stringify` of a Proxy (SerializeJSONObject, EnumerableOwnProperties of kind key) holds
/// the keys an `ownKeys` trap made at run time while the `getOwnPropertyDescriptor` trap of each
/// key runs code that collects (`ops/objects.rs`, `enumerable_own_keys`).
#[test]
fn json_stringify_holds_the_keys_an_own_keys_trap_made() {
    answers_with_junk(
        r#"var p = new Proxy({}, {
          ownKeys: function () { return ["k" + 1, "k" + 2, "k" + 3]; },
          getOwnPropertyDescriptor: function () {
            junk(); return { value: 1, writable: true, enumerable: true, configurable: true }; },
          get: function (t, k) { return "v" + k; }
        });
        [JSON.stringify(p).split("\"").join("|"), JSON.stringify(fresh(0)).split("\"").join("|")].join(" ");"#,
        "{|k1|:|vk1|,|k2|:|vk2|,|k3|:|vk3|} {|d0a|:|gd0a|,|d0b|:|gd0b|}",
    );
}

/// The reviver walk of `JSON.parse` (InternalizeJSONProperty, EnumerableOwnProperties of kind
/// key) over a Proxy the reviver put in place holds the keys its `ownKeys` trap made while its
/// `getOwnPropertyDescriptor` trap collects; the reviver's answers are defined on the Proxy.
#[test]
fn json_parse_reviver_walk_holds_the_keys_an_own_keys_trap_made() {
    answers_with_junk(
        r#"var p = new Proxy({}, {
          ownKeys: function () { return ["k" + 1, "k" + 2]; },
          getOwnPropertyDescriptor: function () {
            junk(); return { value: 1, writable: true, enumerable: true, configurable: true }; },
          get: function () { return 7; }
        });
        var log = [];
        JSON.parse('{"a":1,"b":2,"c":3}', function (k, v) {
          if (k === "a") this.b = p;
          if (k === "b") this.c = fresh(0);
          log.push(k);
          return v;
        });
        log.join();"#,
        "a,k1,k2,b,d0a,d0b,c,",
    );
}

/// `for`-`in` over a Proxy (EnumerateObjectProperties) holds the keys its `ownKeys` trap made at
/// run time while its `getOwnPropertyDescriptor` trap collects.
#[test]
fn for_in_holds_the_keys_an_own_keys_trap_made() {
    answers_with_junk(
        r#"var p = new Proxy({}, {
          ownKeys: function () { return ["k" + 1, "k" + 2, "k" + 3]; },
          getOwnPropertyDescriptor: function () {
            junk(); return { value: 1, enumerable: true, configurable: true }; },
          has: function () { return true; }
        });
        var r = [];
        for (var k in p) r.push(k);
        r.join();"#,
        "k1,k2,k3",
    );
}

/// `for`-`in` holds the prototype a `getPrototypeOf` trap made (a new Proxy, held by nothing
/// else) while that prototype's own traps collect; its keys are literals, so only the object is
/// at stake.
#[test]
fn for_in_holds_a_prototype_a_get_prototype_of_trap_made() {
    answers_with_junk(
        r#"function made() {
          return new Proxy({}, {
            ownKeys: function () { junk(); return ["z1", "z2"]; },
            getOwnPropertyDescriptor: function () {
              junk(); return { value: 1, enumerable: true, configurable: true }; },
            getPrototypeOf: function () { junk(); return null; }
          });
        }
        var p = new Proxy({}, {
          ownKeys: function () { return []; },
          getPrototypeOf: function () { return made(); },
          has: function () { return true; }
        });
        var r = [];
        for (var k in p) r.push(k);
        r.join();"#,
        "z1,z2",
    );
}

/// `for`-`in` over an ordinary object whose chain runs through [`FRESH`] Proxies: the keys of
/// the object itself (made at run time, one of them deleted by the body before it is reached),
/// then those of each Proxy, each name once, a name of the object shadowing the same name
/// further up.
#[test]
fn fresh_answers_are_held_by_for_in() {
    answers_with_junk(
        r#"var o = Object.create(fresh(1));
        o["o" + 1] = 1; o["o" + 2] = 2; o["d0" + "a"] = 3;
        var r = [];
        for (var k in o) { if (k === "o1") delete o.o2; r.push(k); }
        var q = [];
        for (var k in fresh(2)) q.push(k);
        r.join("+") + " " + q.join("+");"#,
        "o1+d0a+d1a+d1b+d0b d2a+d2b+d1a+d1b+d0a+d0b",
    );
}

/// EnumerableOwnProperties of kind key, value and key+value (`Object.keys`, `values`,
/// `entries`) over a [`FRESH`] Proxy.
#[test]
fn fresh_answers_are_held_by_enumerable_own_properties() {
    answers_with_junk(
        r#"[Object.keys(fresh(0)).join("+"), Object.values(fresh(0)).join("+"),
         Object.entries(fresh(0)).join("+")].join(" ");"#,
        "d0a+d0b gd0a+gd0b d0a,gd0a+d0b,gd0b",
    );
}

/// CopyDataProperties and `Object.assign` from a [`FRESH`] Proxy: the String keys and the Symbol
/// it answers are copied with the values its `get` trap made.
#[test]
fn fresh_answers_are_held_by_copy_data_properties() {
    answers_with_junk(
        r#"function shown(o) {
          return Object.keys(o).join("+") + "/" + Object.getOwnPropertySymbols(o).length + "/" + o.d0b;
        }
        var assigned = Object.assign({}, fresh(0));
        var spread = { ...fresh(0) };
        var { d0a, ...rest } = fresh(0);
        [shown(assigned), shown(spread), d0a, shown(rest)].join(" ");"#,
        "d0a+d0b/1/gd0b d0a+d0b/1/gd0b gd0a d0b/1/gd0b",
    );
}

/// SetIntegrityLevel and TestIntegrityLevel over a Proxy of a non-extensible target whose traps
/// collect and answer new descriptor objects with values made at run time.
#[test]
fn fresh_answers_are_held_by_the_integrity_levels() {
    answers_with_junk(
        r#"function closed() {
          var t = {};
          t["c" + 1] = "x" + 1;
          Object.defineProperty(t, "c" + 2, { get: function () { return 1; }, configurable: true, enumerable: true });
          Object.preventExtensions(t);
          return new Proxy(t, {
            ownKeys: function (t) { junk(); return Reflect.ownKeys(t); },
            getOwnPropertyDescriptor: function (t, k) {
              junk();
              var d = Reflect.getOwnPropertyDescriptor(t, k);
              if (d && "value" in d) {
                d = { value: d.value + "", writable: d.writable, enumerable: d.enumerable, configurable: d.configurable };
              }
              return d;
            },
            defineProperty: function (t, k, d) { junk(); return Reflect.defineProperty(t, k, d); },
            isExtensible: function (t) { junk(); return Reflect.isExtensible(t); },
            preventExtensions: function (t) { junk(); return Reflect.preventExtensions(t); }
          });
        }
        [Object.isFrozen(closed()), Object.isSealed(closed()),
         Object.isFrozen(Object.freeze(closed())), Object.isSealed(Object.seal(closed())),
         outcome(function () { return Object.freeze(fresh(0)); })].join();"#,
        "false,false,true,true,TypeError(the object cannot be made non-extensible)",
    );
}

/// The lists of own keys and what is read for them: `Reflect.ownKeys`,
/// `Object.getOwnPropertyNames` and `getOwnPropertySymbols`, `getOwnPropertyDescriptors`,
/// `Object.defineProperties` and `Object.create` with a [`FRESH`] Proxy of descriptor objects.
#[test]
fn fresh_answers_are_held_by_the_lists_of_own_keys() {
    answers_with_junk(
        r#"var defined = Object.defineProperties({}, fresh(0, true));
        var created = Object.create(null, fresh(1, true));
        var descriptors = Object.getOwnPropertyDescriptors(fresh(0));
        [Reflect.ownKeys(fresh(0)).length, Object.getOwnPropertyNames(fresh(0)).join("+"),
         Object.getOwnPropertySymbols(fresh(0)).length,
         Object.keys(descriptors).join("+") + "=" + descriptors.d0b.value,
         Object.keys(defined).join("+") + "=" + defined.d0b,
         Object.keys(created).join("+") + "=" + created.d1a].join(" ");"#,
        "3 d0a+d0b 1 d0a+d0b=vd0b d0a+d0b=wd0b d1a+d1b=wd1a",
    );
}

/// The walks of a prototype chain through Proxies whose `getPrototypeOf` traps answer new
/// Proxies: `instanceof` (OrdinaryHasInstance, the constructor's `prototype` made by a `get`
/// trap), `isPrototypeOf` with a primitive this value, [[Get]] and [[HasProperty]] of an
/// ordinary object whose parent is a [`FRESH`] Proxy, and `bind` of a callable Proxy.
#[test]
fn fresh_answers_are_held_by_the_walks_of_a_prototype_chain() {
    answers_with_junk(
        r#"var C = new Proxy(function () {}, {
          get: function (t, k) { junk(); return k === "prototype" ? {} : Reflect.get(t, k); }
        });
        function F() {}
        var v = new Proxy({}, { getPrototypeOf: function () {
          junk();
          return new Proxy({}, { getPrototypeOf: function () { junk(); return F.prototype; } });
        } });
        var g = new Proxy(function () { return 7; }, {
          getPrototypeOf: function () { junk(); return fresh(0); }
        });
        var b = g.bind(null);
        var o = Object.create(fresh(1));
        [fresh(2) instanceof C, v instanceof F, Object.prototype.isPrototypeOf.call("s", fresh(2)),
         F.prototype.isPrototypeOf(v), Object.getPrototypeOf(b) !== null, o.d0b, "d0a" in o,
         b()].join();"#,
        "false,true,false,true,true,gd0b,true,7",
    );
}

/// CreateDataProperty and the ordinary [[Set]] on a Proxy whose `defineProperty` trap collects:
/// the array `map` makes through `@@species`, the object `Array.of` and `Array.from` construct,
/// and a Proxy receiver of `Reflect.set`; keys and values made at run time.
#[test]
fn fresh_answers_are_held_by_the_definitions() {
    answers_with_junk(
        r#"var log = [];
        function logged() {
          return new Proxy({}, {
            getOwnPropertyDescriptor: function () { junk(); return undefined; },
            defineProperty: function (t, k, d) { junk(); log.push(k + "=" + d.value); return true; }
          });
        }
        var a = [1, 2];
        a.constructor = {};
        a.constructor[Symbol.species] = function () { return logged(); };
        a.map(function (x) { return "m" + x; });
        Array.of.call(function () { return logged(); }, "a" + 1, "b" + 2);
        Array.from.call(function () { return logged(); }, ["x" + 1]);
        Reflect.set({}, "r" + 1, "v" + 1, logged());
        log.join();"#,
        "0=m1,1=m2,0=a1,1=b2,length=2,0=x1,length=1,r1=v1",
    );
}

/// `for`-`in` over a Proxy visits the enumerable keys of the prototype its `getPrototypeOf` trap
/// answers (`#sec-enumerate-object-properties`: "Enumerating the properties of the target object
/// includes enumerating properties of its prototype"; only a property deleted before it is
/// processed is ignored, and `c` is never deleted).
#[test]
fn for_in_visits_the_keys_of_the_prototype_the_trap_answers() {
    answers_once(
        r#"var proto = { c: 5, a: 6 };
        var p = new Proxy({ a: 1, b: 2 }, { getPrototypeOf: function () { return proto; } });
        var r = [];
        for (var k in p) r.push(k);
        r.join();"#,
        "a,b,c",
    );
}

/// Whether a key of a `for`-`in` over a Proxy is still a property when its turn comes is read by
/// [[GetOwnProperty]] of each object of the chain the enumeration walked, never by a `has`
/// trap: a key of the prototype the trap answers is visited, one deleted from it before
/// its turn is not, and a key of the target shadows the same name of the prototype.
#[test]
fn for_in_over_a_proxy_calls_no_has_trap() {
    answers_once(
        r#"var proto = { c: 1, d: 2, a: 9 };
        var p = new Proxy({ a: 1, b: 2 }, {
          has: function () { throw new Error("has called"); },
          getPrototypeOf: function () { return proto; }
        });
        var r = [];
        for (var k in p) { if (k === "a") delete proto.d; r.push(k); }
        r.join();"#,
        "a,b,c",
    );
}

/// `for`-`in` over a Proxy of `{ a: 1, b: 2 }` whose target's prototype is a Proxy of
/// `{ c: 3 }`, both with handlers that log each trap and forward it: the walk asks each object
/// for its own keys, the descriptor of each String key and its prototype, once; then before
/// each key's turn the loop asks [[GetOwnProperty]] of the objects of the chain the walk
/// recorded, in order, until one has the key, and no object for its prototype again (D66).
/// 11 trap calls; at `61b14ed2` there were 12 (the filter asked `p` for its prototype again
/// before `c`), and the clause's informative definition makes 7 (it reads each object's keys
/// and descriptors once, when it reaches them, and filters nothing).
#[test]
fn for_in_asks_each_object_of_the_chain_for_its_prototype_once() {
    answers(
        r#"var log = [];
        function logAll(name) {
          return {
            ownKeys: function (t) { log.push(name + ".ownKeys"); return Reflect.ownKeys(t); },
            getOwnPropertyDescriptor: function (t, k) { log.push(name + ".gopd:" + String(k)); return Reflect.getOwnPropertyDescriptor(t, k); },
            getPrototypeOf: function (t) { log.push(name + ".getPrototypeOf"); return Reflect.getPrototypeOf(t); },
            has: function (t, k) { log.push(name + ".has:" + String(k)); return Reflect.has(t, k); },
            get: function (t, k, r) { log.push(name + ".get:" + String(k)); return Reflect.get(t, k, r); }
          };
        }
        var q = new Proxy({ c: 3 }, logAll("q"));
        var p = new Proxy(Object.setPrototypeOf({ a: 1, b: 2 }, q), logAll("p"));
        for (var k in p) log.push("body:" + k);
        log.join();"#,
        "p.ownKeys,p.gopd:a,p.gopd:b,p.getPrototypeOf,q.ownKeys,q.gopd:c,q.getPrototypeOf,\
         p.gopd:a,body:a,p.gopd:b,body:b,p.gopd:c,q.gopd:c,body:c",
    );
}

/// A key deleted from the object that listed it is still visited while a later object of the
/// chain the walk recorded has it (the name is still a property of the chain, found on the
/// later object, though that one's own `b` was shadowed during the walk), and a key deleted
/// from every object of the chain before its turn is not ("A property that is deleted before
/// it is processed is ignored", `#sec-enumerate-object-properties`).
#[test]
fn a_key_deleted_where_it_was_listed_is_visited_while_a_later_object_has_it() {
    answers_once(
        r#"var proto = { b: 3, c: 4 };
        var t = { a: 1, b: 2 };
        var p = new Proxy(t, { getPrototypeOf: function () { return proto; } });
        var r = [];
        for (var k in p) { if (k === "a") { delete t.b; delete proto.c; } r.push(k); }
        r.join();"#,
        "a,b",
    );
}

/// A key that an earlier object of the recorded chain gains after the walk, not enumerable,
/// does not hide the same key of a later object the walk found: the clause lets the loop
/// ignore a key only when it was deleted before its turn, and `c` never was; the clause's
/// informative definition reads each object's keys when it reaches that object, so `c` is in
/// no visited set when it is yielded. No case of the pinned test262 asks otherwise
/// (`order-enumerable-shadowed.js` and `12.6.4-2.js` shadow by a property that exists before
/// the loop starts). The loop finds `c` on the earlier object and visits it.
#[test]
fn a_key_an_earlier_object_gains_after_the_walk_is_visited() {
    answers_once(
        r#"var proto = { c: 4 };
        var t = { a: 1 };
        var p = new Proxy(t, { getPrototypeOf: function () { return proto; } });
        var r = [];
        for (var k in p) {
          if (k === "a") Object.defineProperty(t, "c", { value: 0, enumerable: false, configurable: true });
          r.push(k);
        }
        r.join();"#,
        "a,c",
    );
}

/// The loop keeps the chain the walk found (D66): when the body cuts an ordinary object off it
/// (`Object.setPrototypeOf` of the object before it to `null`), a key the walk found on the
/// object cut off is still read there and visited, and that object, held by nothing but the
/// loop, stays held while each key's [[GetOwnProperty]] of the Proxy runs a trap that collects.
/// At `61b14ed2` the loop asked [[GetPrototypeOf]] again, found `null` and dropped `c1`.
#[test]
fn the_loop_keeps_the_chain_it_found_when_a_prototype_changes() {
    answers_with_junk(
        r#"function cut() {
          var y = {}; y["c" + 1] = 2;
          return Object.create(y, { b: { value: 1, enumerable: true, configurable: true } });
        }
        var x = cut();
        var p = new Proxy({ a: 1 }, {
          getPrototypeOf: function () { return x; },
          getOwnPropertyDescriptor: function (t, k) { junk(); return Reflect.getOwnPropertyDescriptor(t, k); }
        });
        var r = [];
        for (var k in p) { if (k === "a") Object.setPrototypeOf(x, null); r.push(k); }
        r.join();"#,
        "a,b,c1",
    );
}

/// The walk records the ordinary objects before the first Proxy of the chain too, and holds
/// them from that Proxy on: here the Proxy's `ownKeys` trap cuts the object between the
/// receiver and the Proxy off the chain (held by nothing else) and collects, and the loop still
/// reads it at the turn of its key `m1`, which no other object of the chain has.
#[test]
fn the_walk_holds_the_ordinary_objects_before_the_first_proxy() {
    answers_with_junk(
        r#"var o = { o1: 1 };
        var q = new Proxy({}, { ownKeys: function () { Object.setPrototypeOf(o, null); junk(); return []; } });
        function mid() { var m = Object.create(q); m["m" + 1] = 1; return m; }
        Object.setPrototypeOf(o, mid());
        var r = [];
        for (var k in o) r.push(k);
        r.join();"#,
        "o1,m1",
    );
}

/// The setup of [`every_way_out_of_a_loop_over_a_proxy_gives_back_its_chain`]: `chained()` is a
/// Proxy of three keys whose `getPrototypeOf` trap answers a new object of one key, so the walk
/// records a chain of three objects; `trapped(mode)` is a Proxy whose `getOwnPropertyDescriptor`
/// trap, once the walk is over (its `getPrototypeOf` trap ran), loops until the step budget
/// (`"steps"`) or makes Strings until the memory budget (`"memory"`); `g` is a generator whose
/// body yields inside a loop over `chained()`.
const EXITS: &str = "function chained() { return new Proxy({ a: 1, b: 2, c: 3 }, { getPrototypeOf: function () { return { d: 4 }; } }); }\n\
     function trapped(mode) { var walked = false; return new Proxy({ a: 1, b: 2 }, {\n\
       getOwnPropertyDescriptor: function (t, k) {\n\
         if (walked && mode === \"steps\") for (;;) {}\n\
         if (walked && mode === \"memory\") { var a = []; for (;;) a.push(\"x\".repeat(4096) + a.length); }\n\
         return Reflect.getOwnPropertyDescriptor(t, k); },\n\
       getPrototypeOf: function () { walked = true; return null; } }); }\n\
     function* g() { for (var k in chained()) yield k; }\n\
     var it = null; 0;";

/// Each entry of `exits` with its outcome, run in rounds on `agent` after [`EXITS`]: before
/// and after each, one collection and a reading of `heap_bytes()`. The first two rounds grow
/// the arenas to what a round needs; in the third every reading after an entry is the reading
/// before it. The readings are printed.
fn gives_back(
    agent: &mut swell::Agent<support::host::TestHost>,
    realm: &swell::Realm,
    exits: &[(&str, Outcome)],
) {
    assert_eq!(
        eval_text(agent, realm, EXITS),
        Outcome::Normal("0".to_string())
    );
    let mut rounds = Vec::new();
    for _ in 0..3 {
        let mut readings = Vec::new();
        for (source, outcome) in exits {
            collect(agent);
            let before = agent.heap_bytes().unwrap();
            assert_eq!(&eval_text(agent, realm, source), outcome, "{source}");
            assert_eq!(agent.state(), AgentState::Alive);
            collect(agent);
            readings.push((before, agent.heap_bytes().unwrap()));
        }
        rounds.push(readings);
    }
    println!("heap_bytes before and after each way out, per round: {rounds:?}");
    for ((source, _), (before, after)) in exits.iter().zip(&rounds[2]) {
        assert_eq!(before, after, "{source}");
    }
}

/// The room of the memory exit of [`every_way_out_of_a_loop_over_a_proxy_gives_back_its_chain`]:
/// 1 MiB, 64 KiB with the feature `collect-every-safe-point`, where each String the trap makes
/// passes safe points that collect the realm.
const EXIT_ROOM: u64 = if cfg!(feature = "collect-every-safe-point") {
    1 << 16
} else {
    1 << 20
};

/// The chain a `for`-`in` over a Proxy recorded is given back on every way out of the loop:
/// its end, a `break`, a throw in the body caught outside, a termination at the step budget
/// and one at the memory budget inside the `getOwnPropertyDescriptor` trap of a key's turn,
/// and a generator suspended in the body and then collected; after a collection
/// `heap_bytes()` reads what it read before each ([`gives_back`]). The memory budget is the
/// realm's reading after the setup plus [`EXIT_ROOM`], on an agent of its own with steps to
/// spare.
#[test]
fn every_way_out_of_a_loop_over_a_proxy_gives_back_its_chain() {
    let zero = || Outcome::Normal("0".to_string());
    let mut budget = limits();
    budget.steps_per_entry = if cfg!(feature = "collect-every-safe-point") {
        300
    } else {
        20_000
    };
    let mut agent = agent_with(budget).unwrap();
    let stepped = realm(&mut agent);
    gives_back(
        &mut agent,
        &stepped,
        &[
            ("for (var k in chained()) {} 0;", zero()),
            ("for (var k in chained()) break; 0;", zero()),
            (
                "try { for (var k in chained()) throw 1; } catch (e) {} 0;",
                zero(),
            ),
            (
                "for (var k in trapped(\"steps\")) {}",
                Outcome::Terminated(Termination::StepBudget),
            ),
            (
                "it = g(); var v = it.next().value; it = null; v;",
                Outcome::Normal("\"a\"".to_string()),
            ),
        ],
    );
    let mut budget = limits();
    budget.steps_per_entry = 1 << 40;
    let mut probe = agent_with(budget).unwrap();
    let probe_realm = realm(&mut probe);
    assert_eq!(eval_text(&mut probe, &probe_realm, EXITS), zero());
    collect(&mut probe);
    budget.memory_bytes = probe.heap_bytes().unwrap() + EXIT_ROOM;
    let mut agent = agent_with(budget).unwrap();
    let bounded = realm(&mut agent);
    gives_back(
        &mut agent,
        &bounded,
        &[(
            "for (var k in trapped(\"memory\")) {}",
            Outcome::Terminated(Termination::MemoryBudget),
        )],
    );
}

/// A Proxy whose `getPrototypeOf` trap answers the Proxy itself makes the walk of `for`-`in` go
/// round without end: each round charges a step, the room of the kept stack for the object and
/// its key, and the growth of the chain it records, so with steps to spare the memory budget
/// ends it, in the walk, before the loop's first key (the step budget ends it with less steps:
/// [`for_in_ends_at_a_revoked_proxy_and_at_a_cycle_of_trap_answers_and_gives_back_its_bytes`]).
/// The budget is the realm's reading after the setup plus 128 KiB (4 KiB with the feature
/// `collect-every-safe-point`, where each round's trap call collects the realm and what
/// the walk kept); after a collection `heap_bytes()` reads what it read after the second of
/// three rounds.
#[test]
fn a_cycle_of_get_prototype_of_answers_ends_at_the_memory_budget() {
    let setup =
        "var cyc = new Proxy({ a: 1 }, { getPrototypeOf: function () { return cyc; } }); 0;";
    let room: u64 = if cfg!(feature = "collect-every-safe-point") {
        1 << 12
    } else {
        1 << 17
    };
    let mut budget = limits();
    budget.steps_per_entry = 1 << 40;
    let mut probe = agent_with(budget).unwrap();
    let probe_realm = realm(&mut probe);
    assert_eq!(
        eval_text(&mut probe, &probe_realm, setup),
        Outcome::Normal("0".to_string())
    );
    collect(&mut probe);
    budget.memory_bytes = probe.heap_bytes().unwrap() + room;
    let mut agent = agent_with(budget).unwrap();
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(&mut agent, &realm, setup),
        Outcome::Normal("0".to_string())
    );
    let mut readings = Vec::new();
    for _ in 0..3 {
        assert_eq!(
            eval_text(&mut agent, &realm, "var n = 0; for (var k in cyc) n++; n;"),
            Outcome::Terminated(Termination::MemoryBudget)
        );
        let spent = agent.last_entry_steps().unwrap();
        assert_eq!(agent.state(), AgentState::Alive);
        collect(&mut agent);
        readings.push((spent, agent.heap_bytes().unwrap()));
    }
    println!("steps and heap_bytes after each round: {readings:?}");
    assert_eq!(readings[1].1, readings[2].1, "{readings:?}");
}

/// `L` `for`-`in` loops in one entry over an ordinary object whose chain has no Proxy when the
/// walk runs, so the loop reads it along [[GetPrototypeOf]] (`enumeration_has_property`): the
/// body of the first key deletes the second and puts a Proxy `P` into the chain, whose
/// `getPrototypeOf` trap answers a fresh Proxy of `{ b: 2 }` whose handler holds a String of
/// 262,144 code units. The second key's turn holds each Proxy it meets while its traps run and
/// releases it when it ends; the loops answer `2 * L` within `ROOM` over the reading of a realm
/// that ran the setup, where `L` fresh Proxies held to the end of the entry need more (as in
/// [`for_in_in_a_loop_gives_back_what_each_loop_held`]).
#[test]
fn for_in_in_a_loop_gives_back_what_each_filter_held() {
    let (loops, room): (u32, u64) = if cfg!(feature = "collect-every-safe-point") {
        (12, 3 << 19)
    } else {
        (20, 3 << 20)
    };
    let setup = "var T = { b: 2 }, kept = [];\n\
                 var P = new Proxy({}, { getPrototypeOf: function () { return new Proxy(T, { pad: \"x\".repeat(262144) }); } });\n\
                 function loop() { var o = { a: 1, b: 2 }, n = 0; for (var k in o) { if (k === \"a\") { delete o.b; Object.setPrototypeOf(o, P); } n++; } return n; }\n\
                 0;";
    let mut budget = limits();
    budget.steps_per_entry = 1 << 40;
    let mut probe = agent_with(budget).unwrap();
    let probe_realm = realm(&mut probe);
    assert_eq!(
        eval_text(&mut probe, &probe_realm, setup),
        Outcome::Normal("0".to_string())
    );
    collect(&mut probe);
    budget.memory_bytes = probe.heap_bytes().unwrap() + room;
    let mut agent = agent_with(budget).unwrap();
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(&mut agent, &realm, setup),
        Outcome::Normal("0".to_string())
    );
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            &format!("var n = 0; for (var i = 0; i < {loops}; i++) n += loop(); n;")
        ),
        Outcome::Normal(format!("{}", 2 * loops))
    );
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            &format!(
                "for (var i = 0; i < {loops}; i++) kept.push(Object.getPrototypeOf(P)); kept.length;"
            )
        ),
        Outcome::Terminated(Termination::MemoryBudget)
    );
}

/// The steps of a `for`-`in` over an object with three own keys, and over a chain of three
/// ordinary objects in which the body deletes a key before its turn: the walk that reads
/// whether a key is still a property costs, for ordinary objects, what HasProperty cost (91 and
/// 126, the numbers the unit printed with HasProperty as the filter, the same in both collection
/// modes).
#[test]
fn for_in_over_ordinary_objects_spends_the_steps_it_spent() {
    let loops = [
        (
            "var o = { a: 1, b: 2, c: 3 }; var n = 0; for (var k in o) n++; n;",
            "3",
            FOR_IN_STEPS[0],
        ),
        (
            "var p = { x: 1, y: 2 }; var q = Object.create(Object.create(p)); q.z = 3; q.x = 4;\n\
             var n = 0; for (var k in q) { if (k === \"z\") delete p.y; n++; } n;",
            "2",
            FOR_IN_STEPS[1],
        ),
    ];
    let mut read = Vec::new();
    for (source, answer, steps) in loops {
        let mut agent = agent_with(limits()).unwrap();
        let realm = realm(&mut agent);
        let outcome = eval(&mut agent, &realm, source);
        // Read before `describe`, whose own entry spends steps of its own.
        let spent = agent.last_entry_steps().unwrap();
        assert_eq!(
            describe(&mut agent, &realm, outcome),
            Outcome::Normal(answer.to_string())
        );
        println!("for-in steps {spent}: {source}");
        read.push((spent, steps));
    }
    for (spent, steps) in read {
        assert_eq!(spent, steps);
    }
}

/// The steps of the two loops of [`for_in_over_ordinary_objects_spends_the_steps_it_spent`].
const FOR_IN_STEPS: [u64; 2] = [91, 126];

/// The target's descriptor, read from a target that is a Proxy whose answer holds a String made
/// at run time, stays held while IsExtensible of that target runs a trap that collects, and
/// SameValue of I9 and I14 then reads it. It catches `keep_property(current)` dropped in
/// [[GetOwnProperty]] or in [[DefineOwnProperty]] (mutants 2 and 3 of the adversary's report).
#[test]
fn the_target_descriptor_is_held_across_is_extensible() {
    answers_with_junk(
        r#"var t = {};
        Object.defineProperty(t, "s", { value: "ab" });
        var inner = new Proxy(t, {
          getOwnPropertyDescriptor: function () {
            return { value: "a" + "b", writable: false, enumerable: false, configurable: false }; },
          isExtensible: function (target) { junk(); return Reflect.isExtensible(target); }
        });
        var p = new Proxy(inner, { getOwnPropertyDescriptor: function () {
          return { value: "a".concat("b"), writable: false, enumerable: false, configurable: false }; } });
        var d = Object.getOwnPropertyDescriptor(p, "s");
        var q = new Proxy(inner, { defineProperty: function () { return true; } });
        [d.value, d.configurable, Reflect.defineProperty(q, "s", { value: "ab".slice(0) + "" }),
         outcome(function () { return Reflect.defineProperty(q, "s", { value: "a" + "c" }); }, 14, "defineProperty", "s")].join();"#,
        "ab,false,true,I14",
    );
}

/// IsCompatiblePropertyDescriptor (`#sec-iscompatiblepropertydescriptor`) through I14
/// for the inputs the invariant tests do not give it: a generic descriptor against a
/// non-configurable accessor (only `enumerable` decides), a descriptor of no fields, another
/// getter, a data descriptor against an accessor, and an accessor against a configurable data
/// property.
#[test]
fn is_compatible_for_generic_and_accessor_descriptors() {
    answers_once(
        r#"var t = {}, g2 = function () {};
        Object.defineProperty(t, "acc", { get: function () {}, enumerable: false, configurable: false });
        Object.defineProperty(t, "conf", { value: 1, writable: true, enumerable: true, configurable: true });
        var p = new Proxy(t, { defineProperty: function () { return true; } });
        function define(k, d) { return outcome(function () { return Reflect.defineProperty(p, k, d); }, 14, "defineProperty", k); }
        [define("acc", { enumerable: true }), define("acc", { enumerable: false }), define("acc", {}),
         define("acc", { get: g2 }), define("acc", { value: 1 }),
         define("conf", { get: g2, configurable: true })].join();"#,
        "I14,true,true,I14,I14,true",
    );
}

/// The count of `enter_proxy` after the unit's change, read through each builtin of
/// the seven methods and through two instructions: a trapless chain of exactly `reentry_depth`
/// proxies answers and one more throws the RangeError, the same bound whether a builtin or the
/// running loop reaches the first Proxy.
#[test]
fn a_trapless_chain_reaches_the_same_bound_through_each_builtin() {
    let mut budget = limits();
    budget.reentry_depth = 20;
    let mut agent = agent_with(budget).unwrap();
    let realm = realm(&mut agent);
    let script = r#"function c(n) { var q = { x: 1 }; for (var i = 0; i < n; i++) q = new Proxy(q, {}); return q; }
        var ops = [
          function (p) { return Object.getPrototypeOf(p); },
          function (p) { return Object.keys(p); },
          function (p) { return Reflect.ownKeys(p); },
          function (p) { return Object.isExtensible(p); },
          function (p) { return Reflect.getOwnPropertyDescriptor(p, "x"); },
          function (p) { return Reflect.defineProperty(p, "y", { value: 1, configurable: true }); },
          function (p) { return Reflect.setPrototypeOf(p, Object.prototype); },
          function (p) { for (var k in p) {} return 1; },
          function (p) { return p.x; },
          function (p) { return Reflect.preventExtensions(p); }
        ];
        var r = [], twenty = c(20), more = c(21);
        for (var i = 0; i < ops.length; i++) {
          var a, b;
          try { ops[i](twenty); a = "ok"; } catch (e) { a = e.name; }
          try { ops[i](more); b = "ok"; } catch (e) { b = e.name; }
          r.push(a + "/" + b);
        }
        r.join();"#;
    let expected = ["ok/RangeError"; 10].join(",");
    assert_eq!(
        eval_text(&mut agent, &realm, script),
        Outcome::Normal(format!("\"{expected}\""))
    );
}

/// `super[k] = v` with a Proxy as `this` (OrdinarySetWithOwnDescriptor, steps 2.c and
/// 2.e: Receiver.[[GetOwnProperty]], then CreateDataProperty on the receiver) holds a key made at
/// run time while the receiver's `getOwnPropertyDescriptor` trap collects.
#[test]
fn super_set_holds_its_key_across_the_receivers_trap() {
    answers_with_junk(
        r#"var log = [];
        var obj = { m(k, v) { super[k] = v; } };
        var p = new Proxy({}, {
          getOwnPropertyDescriptor: function () { junk(); return undefined; },
          defineProperty: function (t, k, d) { log.push(k + "=" + d.value); return Reflect.defineProperty(t, k, d); }
        });
        obj.m.call(p, "k" + 1, "v" + 2);
        log.join();"#,
        "k1=v2",
    );
}

/// A key the target lists whose [[GetOwnProperty]] answers undefined goes to
/// targetConfigurableKeys (step 16.c), so an extensible target needs no I19. It catches such a
/// key counted as non-configurable in step 16 of [[OwnPropertyKeys]] (mutant 1 of the
/// adversary's report).
#[test]
fn a_target_key_without_a_descriptor_is_configurable() {
    answers_once(
        r#"var inner = new Proxy({}, { ownKeys: function () { return ["ghost"]; } });
        var p = new Proxy(inner, { ownKeys: function () { return []; } });
        [Reflect.ownKeys(inner).join(), Reflect.ownKeys(p).length, Object.keys(p).length].join();"#,
        "ghost,0,0",
    );
}

/// The order of the trap calls of the walks of builtins over a Proxy whose handler
/// logs every trap and forwards through `Reflect`, each against its clause.
#[test]
fn each_builtin_walk_calls_the_traps_in_the_order_of_its_clause() {
    answers_once(
        r#"var log = [], h = {};
        ["getPrototypeOf", "setPrototypeOf", "isExtensible", "preventExtensions",
         "getOwnPropertyDescriptor", "defineProperty", "has", "get", "set", "deleteProperty",
         "ownKeys"].forEach(function (n) {
          h[n] = function (target, k) {
            log.push(n + (typeof k === "string" ? ":" + k : ""));
            return Reflect[n].apply(null, arguments);
          };
        });
        function logged(t) { return new Proxy(t, h); }
        var r = [];
        function op(name, t, f) {
          var p = logged(t);
          log.length = 0;
          var v;
          try { v = f(p); } catch (e) { v = e.name; }
          r.push(name + "=" + String(v) + "[" + log.join("+") + "]");
        }
        op("keys", { a: 1 }, function (p) { return Object.keys(p).join(); });
        op("values", { a: 1 }, function (p) { return Object.values(p).join(); });
        op("entries", { a: 1 }, function (p) { return Object.entries(p).join(); });
        op("assign", { a: 1 }, function (p) { return Object.assign({}, p).a; });
        op("spread", { a: 1 }, function (p) { return ({ ...p }).a; });
        op("rest", { a: 1, b: 2 }, function (p) { var { a, ...rest } = p; return Object.keys(rest).join(); });
        op("names", { a: 1 }, function (p) { return Object.getOwnPropertyNames(p).join(); });
        op("descriptors", { a: 1 }, function (p) { return Object.keys(Object.getOwnPropertyDescriptors(p)).join(); });
        op("freeze", { a: 1 }, function (p) { return Object.freeze(p) === p; });
        op("isFrozen", Object.freeze({ a: 1 }), function (p) { return Object.isFrozen(p); });
        op("seal", { a: 1 }, function (p) { return Object.seal(p) === p; });
        op("isSealed", Object.seal({ a: 1 }), function (p) { return Object.isSealed(p); });
        op("json", { a: 1 }, function (p) { return JSON.stringify(p).length; });
        op("instanceof", {}, function (p) { return p instanceof function F() {}; });
        op("isPrototypeOf", {}, function (p) { return Object.prototype.isPrototypeOf(p); });
        op("setPrototypeOf", {}, function (p) { return Object.setPrototypeOf(p, null) === p; });
        op("push", [1, 2], function (p) { return Array.prototype.push.call(p, 3); });
        r.join(" ");"#,
        "keys=a[ownKeys+getOwnPropertyDescriptor:a] \
         values=1[ownKeys+getOwnPropertyDescriptor:a+get:a] \
         entries=a,1[ownKeys+getOwnPropertyDescriptor:a+get:a] \
         assign=1[ownKeys+getOwnPropertyDescriptor:a+get:a] \
         spread=1[ownKeys+getOwnPropertyDescriptor:a+get:a] \
         rest=b[get:a+ownKeys+getOwnPropertyDescriptor:b+get:b] \
         names=a[ownKeys] \
         descriptors=a[ownKeys+getOwnPropertyDescriptor:a] \
         freeze=true[preventExtensions+ownKeys+getOwnPropertyDescriptor:a+defineProperty:a] \
         isFrozen=true[isExtensible+ownKeys+getOwnPropertyDescriptor:a] \
         seal=true[preventExtensions+ownKeys+defineProperty:a] \
         isSealed=true[isExtensible+ownKeys+getOwnPropertyDescriptor:a] \
         json=7[get:toJSON+ownKeys+getOwnPropertyDescriptor:a+get:a] \
         instanceof=false[getPrototypeOf] \
         isPrototypeOf=true[getPrototypeOf] \
         setPrototypeOf=true[setPrototypeOf] \
         push=3[get:length+set:2+getOwnPropertyDescriptor:2+defineProperty:2+set:length+getOwnPropertyDescriptor:length+defineProperty:length]",
    );
}

/// The reads of a trap's answer, in the order of their clauses: ToPropertyDescriptor
/// of a `getOwnPropertyDescriptor` answer that is a Proxy (HasProperty, then Get, of each field
/// in the order enumerable, configurable, value, writable, get, set); CreateListFromArrayLike of
/// an `ownKeys` answer that is a Proxy (`length` once, then each index); a String object as the
/// answer; a `length` getter that grows the array-like.
#[test]
fn the_answer_of_a_trap_is_read_in_the_order_of_its_clause() {
    answers_once(
        r#"var log = [];
        var desc = new Proxy({ value: 1, configurable: true }, {
          has: function (t, k) { log.push("has:" + k); return k in t; },
          get: function (t, k) { log.push("get:" + k); return t[k]; }
        });
        var p = new Proxy({}, { getOwnPropertyDescriptor: function () { return desc; } });
        var d = Object.getOwnPropertyDescriptor(p, "x");
        var r = [log.join("+"), Object.keys(d).join("+")];
        log.length = 0;
        var like = new Proxy(["a", "b"], { get: function (t, k) { log.push("get:" + String(k)); return t[k]; } });
        r.push(Reflect.ownKeys(new Proxy({}, { ownKeys: function () { return like; } })).join("+"), log.join("+"));
        r.push(Reflect.ownKeys(new Proxy({}, { ownKeys: function () { return new String("ab"); } })).join("+"));
        var grows = { 0: "a", 1: "b", get length() { this[2] = "c"; this[3] = "d"; return 2; } };
        r.push(Reflect.ownKeys(new Proxy({}, { ownKeys: function () { return grows; } })).join("+"));
        r.join(" ");"#,
        "has:enumerable+has:configurable+get:configurable+has:value+get:value+has:writable+has:get+has:set \
         value+writable+enumerable+configurable \
         a+b get:length+get:0+get:1 \
         a+b \
         a+b",
    );
}

/// Keys that are Strings of numbers keep their text and their order through
/// `ownKeys` (an index, a String that is not one), and a non-extensible target's index key is
/// the String of its number for I20 and I21.
#[test]
fn own_keys_keeps_strings_of_numbers_as_listed() {
    answers_once(
        r#"var q = new Proxy({}, { ownKeys: function () {
          return ["01", "1", "4294967295", "-0", "1.5", "4294967294", "0"]; } });
        var t = Object.preventExtensions({ 1: "a", "01": "b" });
        var answer, c = new Proxy(t, { ownKeys: function () { return answer; } });
        function keys(a, k) { answer = a; return outcome(function () { return Reflect.ownKeys(c).join("+"); }, k === "01" ? 20 : 21, "ownKeys", k); }
        [Reflect.ownKeys(q).join("+"), keys(["01", String(1)]), keys(["1", "1.0"], "01"),
         keys(["1", "01", "1.0"], "1.0")].join(" ");"#,
        "01+1+4294967295+-0+1.5+4294967294+0 01+1 I20 I21",
    );
}

/// The order of the checks of [[DefineOwnProperty]] for a property the target lacks:
/// a target that is not extensible breaks I12 before `configurable: false` breaks I13 (step 14.a
/// before 14.b); a descriptor of only `undefined` fields reaches the trap with those fields.
/// It catches the setter read before the getter in FromPropertyDescriptor (mutant 4 of the
/// adversary's report).
#[test]
fn define_property_checks_i12_before_i13() {
    answers_once(
        r#"var seen, closed = new Proxy(Object.preventExtensions({}), { defineProperty: function () { return true; } });
        var open = new Proxy({}, { defineProperty: function (t, k, d) { seen = d; return true; } });
        var r = [outcome(function () { return Reflect.defineProperty(closed, "x", { value: 1, configurable: false }); }, 12, "defineProperty", "x")];
        Reflect.defineProperty(open, "y", { value: undefined });
        r.push(Object.keys(seen).join("+"), "value" in seen);
        Reflect.defineProperty(open, "y", { get: undefined, set: undefined });
        r.push(Object.keys(seen).join("+"));
        r.join();"#,
        "I12,value,true,get+set",
    );
}

// The cases of the adversary's second pass: the walk of `for`-`in`, what it holds and gives
// back, and the memory budget through it.

/// `for`-`in` over a Proxy whose `getPrototypeOf` trap answers `{ a: 1 }` at its first
/// call and `{ b: 2 }` after: the enumeration walked `{ a: 1 }`, whose `a` is enumerable and is
/// never deleted, so the loop visits `a` (`#sec-enumerate-object-properties`: the iterator
/// iterates over all the String keys of enumerable properties, its prototype's included; only
/// a property deleted before it is processed is ignored). The filter of `ForInNext` reads the
/// chain the walk recorded (D66) and does not ask the trap for the prototype a second time; at
/// `61b14ed2` it walked from the receiver again, got `{ b: 2 }` and dropped `a`.
#[test]
fn for_in_visits_the_keys_of_the_prototype_the_trap_answered_first() {
    answers_once(
        r#"var n = 0;
        var p = new Proxy({}, { getPrototypeOf: function () { n++; return n === 1 ? { a: 1 } : { b: 2 }; } });
        var r = [];
        for (var k in p) r.push(k);
        r.join();"#,
        "a",
    );
}

/// The keys of an ordinary object before the first Proxy of the chain (found and
/// visited), deleted by that Proxy's `ownKeys` trap, which then collects: the walk holds them
/// (`keep_visited`), and the filter hands each to the Proxy's `getOwnPropertyDescriptor` trap,
/// which reads it as a String and collects again. Both keys were deleted before their turn.
/// The keys are made at run time and named by no literal of the script, so no code holds
/// their atoms.
#[test]
fn for_in_holds_the_visited_keys_a_trap_deletes_from_an_object_before_it() {
    answers_with_junk(
        r#"var o = {}; o["o" + 1] = 1; o["o" + 2] = 2;
        var log = [];
        var q = new Proxy({}, {
          ownKeys: function () { delete o["o" + 1]; delete o["o" + 2]; junk(); return []; },
          getOwnPropertyDescriptor: function (t, k) { log.push(String(k)); junk(); return undefined; }
        });
        Object.setPrototypeOf(o, q);
        var r = [];
        for (var k in o) r.push(k);
        r.join() + "|" + log.join() + "|" + Object.keys(o).length;"#,
        "|o1,o2|0",
    );
}

/// The keys of an ordinary object after the first Proxy of the chain (the prototype a
/// `getPrototypeOf` trap answers), deleted by the `ownKeys` trap of the next Proxy, which then
/// collects: the walk holds them (`held_own_keys` for every object from the first Proxy on).
#[test]
fn for_in_holds_the_keys_of_an_ordinary_object_after_the_first_proxy() {
    answers_with_junk(
        r#"var x = {}; x["x" + 1] = 1; x["x" + 2] = 2;
        var log = [];
        var q = new Proxy({}, {
          ownKeys: function () { delete x["x" + 1]; delete x["x" + 2]; junk(); return []; },
          getOwnPropertyDescriptor: function (t, k) { log.push(String(k)); junk(); return undefined; }
        });
        Object.setPrototypeOf(x, q);
        var p = new Proxy({}, { getPrototypeOf: function () { return x; } });
        var r = [];
        for (var k in p) r.push(k);
        r.join() + "|" + log.join() + "|" + Object.keys(x).length;"#,
        "|x1,x2|0",
    );
}

/// `c`, a chain of three Proxies of 20 keys each (`k<level>_<i>`), whose traps answer
/// arrays and descriptor objects made once, so that a run makes little garbage; `pads` holds
/// the Strings that take the room of the memory sweep.
#[cfg(not(feature = "collect-every-safe-point"))]
const THREE: &str = "var c = null;\n\
  for (var d = 2; d >= 0; d--) (function (d, proto) {\n\
    var t = {}, keys = [], descs = {};\n\
    for (var i = 0; i < 20; i++) { var k = \"k\" + d + \"_\" + i; t[k] = i; keys.push(k); descs[k] = { value: i, writable: true, enumerable: true, configurable: true }; }\n\
    c = new Proxy(t, { ownKeys: function () { return keys; }, getOwnPropertyDescriptor: function (t, k) { return descs[k]; }, getPrototypeOf: function () { return proto; } });\n\
  })(d, c);\n\
  var pads = [], started = 0; 0;";

/// The room above the reading after [`THREE`] and one run that the sweep's agent has.
#[cfg(not(feature = "collect-every-safe-point"))]
const SWEEP_ROOM: u64 = 1 << 17;

/// The least room the sweep leaves above its pads: a String of 512 code units, which
/// needs twice its 1,024 bytes while it is made, and the short entries between runs fit in it.
#[cfg(not(feature = "collect-every-safe-point"))]
const FLOOR: u64 = 3_072;

/// The memory budget cut at every 64 bytes of what `op` needs over [`THREE`]. Entries
/// push Strings to `pads`, each half of the room left above a target (a String costs twice its
/// length while it is made), until the room left, `memory_bytes` less `heap_bytes()` after a
/// collection, is near the target. Then `op` (an expression) runs in a function of its own
/// after `var h = "x".repeat(n)` in it, for `n` down to 0 by 32 (from a quarter of the first
/// target's room, then from at least 512), so that the room `op` has grows by 64 bytes per
/// run with no gap between targets; `h` is garbage once the entry ends, and the short
/// entry after it that reads `started` has the target's room. The target starts at [`FLOOR`]
/// and grows by 512 bytes (the last pads are popped first) until `op` has answered 8 runs in
/// a row. Every outcome is the answer or
/// `Terminated(MemoryBudget)`; after each, one collection gives `live_bytes()` and
/// `heap_bytes()` back their readings before the run, and at the end `pads = []` and one
/// collection give them back their readings after the warm-up. No two rooms `op` started with,
/// sorted, are more than 64 bytes apart below the last that ended.
#[cfg(not(feature = "collect-every-safe-point"))]
fn memory_sweep(op: &str) {
    let mut budget = limits();
    budget.steps_per_entry = 1 << 40;
    let source = |units: u64| {
        format!(
            "started = 0; (function () {{ var h = \"x\".repeat({units}); started = 1; \
             return {op}; }})();"
        )
    };
    let mut probe = agent_with(budget).unwrap();
    let probe_realm = realm(&mut probe);
    let zero = Outcome::Normal("0".to_string());
    assert_eq!(eval_text(&mut probe, &probe_realm, THREE), zero);
    let answer = eval_text(&mut probe, &probe_realm, &source(0));
    assert!(matches!(answer, Outcome::Normal(_)), "{op}: {answer:?}");
    collect(&mut probe);
    let base = probe.heap_bytes().unwrap();
    budget.memory_bytes = base + SWEEP_ROOM;
    let mut agent = agent_with(budget).unwrap();
    let realm = realm(&mut agent);
    assert_eq!(eval_text(&mut agent, &realm, THREE), zero);
    let room_now = |agent: &swell::Agent<support::host::TestHost>| {
        budget
            .memory_bytes
            .saturating_sub(agent.heap_bytes().unwrap())
    };
    // The room left above `target` taken by pads.
    let fill = |agent: &mut swell::Agent<support::host::TestHost>, target: u64| {
        let mut shift = 0;
        loop {
            let units = (room_now(agent).saturating_sub(target) / 2) >> shift;
            if room_now(agent) <= target + 64 || units == 0 {
                return;
            }
            let pushed = eval_text(
                agent,
                &realm,
                &format!("pads.push(\"x\".repeat({units})); 0;"),
            );
            if pushed == zero {
                shift = 0;
            } else {
                shift += 1;
                // What the refused entry made is garbage.
                collect(agent);
            }
        }
    };
    let run = |agent: &mut swell::Agent<support::host::TestHost>, units: u64| {
        let outcome = eval_text(agent, &realm, &source(units));
        // What the run made, `h` with it, is garbage.
        collect(agent);
        let started = match eval_text(agent, &realm, "started;") {
            Outcome::Normal(s) if s == "1" => true,
            Outcome::Normal(s) if s == "0" => false,
            other => panic!("reading `started` after {outcome:?}: {other:?}"),
        };
        (outcome, started)
    };
    // The warm-up: `op` twice with pads, then alone.
    for target in [FLOOR, FLOOR + 4_096] {
        fill(&mut agent, target);
        run(&mut agent, 0);
        assert_eq!(eval_text(&mut agent, &realm, "pads = []; 0;"), zero);
        collect(&mut agent);
    }
    assert_eq!(run(&mut agent, 0).0, answer);
    collect(&mut agent);
    let live = agent.live_bytes().unwrap();
    let heap = agent.heap_bytes().unwrap();
    let (mut in_op, mut before_op, mut answered, mut streak) = (0u32, 0u32, 0u32, 0u32);
    let mut rooms: Vec<u64> = Vec::new();
    let mut last_ended = 0;
    let mut target = FLOOR;
    let mut last_room: Option<u64> = None;
    while streak < 8 && target < SWEEP_ROOM / 2 {
        while room_now(&agent) < target {
            assert_eq!(eval_text(&mut agent, &realm, "pads.pop(); 0;"), zero);
            collect(&mut agent);
        }
        fill(&mut agent, target);
        collect(&mut agent);
        let readings = (agent.live_bytes().unwrap(), agent.heap_bytes().unwrap());
        let room = room_now(&agent);
        // The first target's runs start at half its room (`h` needs twice its bytes while it is
        // made); a later one's one run below the room the last target had, the pads leaving
        // the room anywhere up to 64 bytes over the target.
        let mut units = match last_room {
            None => room / 4,
            Some(top) => (room.saturating_sub(top) / 2 + 32).max(512).min(room / 4),
        };
        last_room = Some(room);
        loop {
            let (outcome, started) = run(&mut agent, units);
            let left = room.saturating_sub(2 * units);
            match &outcome {
                Outcome::Terminated(Termination::MemoryBudget) if started => {
                    in_op += 1;
                    streak = 0;
                    last_ended = last_ended.max(left);
                }
                Outcome::Terminated(Termination::MemoryBudget) => before_op += 1,
                o if *o == answer => {
                    answered += 1;
                    streak += 1;
                }
                other => panic!("{op} with {left} bytes of room: {other:?}"),
            }
            if started {
                rooms.push(left);
            }
            assert_eq!(agent.state(), AgentState::Alive);
            collect(&mut agent);
            assert_eq!(
                (agent.live_bytes().unwrap(), agent.heap_bytes().unwrap()),
                readings,
                "{op} with {left} bytes of room: {outcome:?}"
            );
            if units < 32 {
                break;
            }
            units -= 32;
        }
        target += 512;
    }
    assert_eq!(eval_text(&mut agent, &realm, "pads = []; 0;"), zero);
    collect(&mut agent);
    assert_eq!(agent.live_bytes().unwrap(), live);
    assert_eq!(agent.heap_bytes().unwrap(), heap);
    let mut sorted: Vec<u64> = rooms
        .into_iter()
        .filter(|&r| r <= last_ended + 64)
        .collect();
    sorted.sort_unstable();
    sorted.dedup();
    let gap = sorted
        .windows(2)
        .map(|pair| pair[1] - pair[0])
        .max()
        .unwrap_or(0);
    println!(
        "{op}: base {base}, heap {heap}; {in_op} ended in it, {before_op} before it, \
         {answered} answered; rooms {:?} to {:?} below the last that ended, the widest step \
         {gap} bytes",
        sorted.first(),
        sorted.last()
    );
    assert!(
        in_op > 0 && answered > 0,
        "{in_op} ended, {answered} answered"
    );
    assert!(gap <= 64, "a step of {gap} bytes: {sorted:?}");
}

/// The memory budget at every 64 bytes through a `for`-`in` over [`THREE`]
/// ([`memory_sweep`]). Default mode only: with `collect-every-safe-point` each step of the
/// loop collects, and a run costs about a second.
#[cfg(not(feature = "collect-every-safe-point"))]
#[test]
fn memory_budget_at_every_64_bytes_through_for_in_over_three_proxies() {
    memory_sweep("(function () { var n = 0; for (var k in c) n++; return n; })()");
}

/// The same through `JSON.stringify` of [`THREE`].
#[cfg(not(feature = "collect-every-safe-point"))]
#[test]
fn memory_budget_at_every_64_bytes_through_json_stringify_of_three_proxies() {
    memory_sweep("JSON.stringify(c).length");
}

/// `L` `for`-`in` loops in one entry over a Proxy whose `getPrototypeOf` trap answers a
/// fresh Proxy of `{ a: 1, b: 2 }` at each call, whose handler holds a String of 262,144 code
/// units. The walk of each loop holds the fresh Proxy it met and records it in the chain the
/// loop reads (D66); when the walk ends it releases what it kept, and when the loop ends its
/// chain goes with its enumerator, so the Proxy is garbage. The
/// loops answer `2 * L` within `ROOM` over the reading of a realm that ran the setup, where
/// `L` of them held to the end of the entry need more: an entry that keeps each fresh Proxy in
/// an array ends in `Terminated(MemoryBudget)`. Its loop over an ordinary object, where each
/// key's turn holds what it meets: [`for_in_in_a_loop_gives_back_what_each_filter_held`].
#[test]
fn for_in_in_a_loop_gives_back_what_each_loop_held() {
    let (loops, room): (u32, u64) = if cfg!(feature = "collect-every-safe-point") {
        (12, 3 << 19)
    } else {
        (20, 3 << 20)
    };
    let setup = "var T = { a: 1, b: 2 }, kept = [];\n\
                 var P = new Proxy({}, { getPrototypeOf: function () { return new Proxy(T, { pad: \"x\".repeat(262144) }); } });\n\
                 0;";
    let mut budget = limits();
    budget.steps_per_entry = 1 << 40;
    let mut probe = agent_with(budget).unwrap();
    let probe_realm = realm(&mut probe);
    assert_eq!(
        eval_text(&mut probe, &probe_realm, setup),
        Outcome::Normal("0".to_string())
    );
    collect(&mut probe);
    budget.memory_bytes = probe.heap_bytes().unwrap() + room;
    let mut agent = agent_with(budget).unwrap();
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(&mut agent, &realm, setup),
        Outcome::Normal("0".to_string())
    );
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            &format!("var n = 0; for (var i = 0; i < {loops}; i++) for (var k in P) n++; n;")
        ),
        Outcome::Normal(format!("{}", 2 * loops))
    );
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            &format!(
                "for (var i = 0; i < {loops}; i++) kept.push(Object.getPrototypeOf(P)); kept.length;"
            )
        ),
        Outcome::Terminated(Termination::MemoryBudget)
    );
}

/// A `for`-`in` that meets a Proxy revoked by its body ends in the TypeError of the next
/// key's turn, and one over a Proxy whose `getPrototypeOf` trap answers the Proxy itself ends
/// at the step budget in the walk (with steps to spare, at the memory budget:
/// [`a_cycle_of_get_prototype_of_answers_ends_at_the_memory_budget`]); after three rounds of
/// both and a collection, `heap_bytes()` is what it was after the second.
#[test]
fn for_in_ends_at_a_revoked_proxy_and_at_a_cycle_of_trap_answers_and_gives_back_its_bytes() {
    let mut budget = limits();
    budget.steps_per_entry = if cfg!(feature = "collect-every-safe-point") {
        2_000
    } else {
        100_000
    };
    let mut agent = agent_with(budget).unwrap();
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "function revoked() { var r = Proxy.revocable({ a: 1, b: 2 }, {}); var o = Object.create(r.proxy); o.z = 1; var seen = [];\n\
               try { for (var k in o) { seen.push(k); if (k === \"z\") r.revoke(); } } catch (e) { seen.push(e.name); }\n\
               return seen.join(); }\n\
             var cyc = new Proxy({ a: 1 }, { getPrototypeOf: function () { return cyc; } });\n\
             0;"
        ),
        Outcome::Normal("0".to_string())
    );
    let mut readings = Vec::new();
    for _ in 0..3 {
        assert_eq!(
            eval_text(&mut agent, &realm, "revoked();"),
            Outcome::Normal("\"z,TypeError\"".to_string())
        );
        assert_eq!(
            eval_text(&mut agent, &realm, "for (var k in cyc) {}"),
            Outcome::Terminated(Termination::StepBudget)
        );
        assert_eq!(agent.state(), AgentState::Alive);
        collect(&mut agent);
        readings.push(agent.heap_bytes().unwrap());
    }
    println!("heap_bytes after each round: {readings:?}");
    assert_eq!(readings[1], readings[2], "{readings:?}");
}

/// The steps of `for`-`in` over ordinary shapes, the same as with the filter of
/// `c8e54d7f` (HasProperty, planted back at the head of the correction in a copy): a chain of
/// ten objects, the same with a key of the third deleted in the loop, an Array with a named
/// key, `arguments`, a String object with a named key, a String, a function with a named key.
#[test]
fn for_in_over_ordinary_shapes_spends_the_steps_it_spent() {
    for (source, steps, answer) in [
        ("var n = 0; for (var k in o) n++; n;", 244, "10"),
        (
            "var n = 0; for (var k in o) { if (k === \"k0\") delete Object.getPrototypeOf(Object.getPrototypeOf(o)).k3; n++; } n;",
            317,
            "10",
        ),
        (
            "var a = [1, 2, 3]; a.x = 4; var n = 0; for (var k in a) n++; n;",
            157,
            "4",
        ),
        (
            "(function () { var n = 0; for (var k in arguments) n++; return n; })(1, 2, 3);",
            71,
            "3",
        ),
        (
            "var s = new String(\"abc\"); s.x = 1; var n = 0; for (var k in s) n++; n;",
            149,
            "4",
        ),
        ("var n = 0; for (var k in \"abc\") n++; n;", 117, "3"),
        (
            "function f() {} f.x = 1; var n = 0; for (var k in f) n++; n;",
            69,
            "1",
        ),
    ] {
        let mut agent = agent_with(limits()).unwrap();
        let realm = realm(&mut agent);
        let setup = "var o = { k9: 9 }; for (var i = 8; i >= 0; i--) { o = Object.create(o); o[\"k\" + i] = i; } 0;";
        assert_eq!(
            eval_text(&mut agent, &realm, setup),
            Outcome::Normal("0".to_string())
        );
        let outcome = eval(&mut agent, &realm, source);
        let spent = agent.last_entry_steps().unwrap();
        let shown = describe(&mut agent, &realm, outcome);
        assert_eq!(
            (spent, shown),
            (steps, Outcome::Normal(answer.to_string())),
            "{source}"
        );
    }
}
