//! The builtins of `story:harness-builtins` with ECMAScript functions as their callbacks: the
//! tests of that story give them host functions; here script functions are called back from
//! Rust, in sloppy and in strict code, each test citing its clause. Run also with the feature
//! `collect-every-safe-point`.
#![cfg(test)]

use crate::support;

use support::script::{Outcome, run_both};

/// Both modes of `source` answer `expected`, a normal completion as `show` writes it.
fn both_answer(source: &str, expected: &str) {
    let (sloppy, strict) = run_both(source);
    let expected = Outcome::Normal(expected.to_string());
    assert_eq!(sloppy, expected, "sloppy: {source}");
    assert_eq!(strict, expected, "strict: {source}");
}

/// Both modes of `source` throw an error named `name`.
fn both_throw(source: &str, name: &str) {
    let (sloppy, strict) = run_both(source);
    let expected = Outcome::Throw(name.to_string());
    assert_eq!(sloppy, expected, "sloppy: {source}");
    assert_eq!(strict, expected, "strict: {source}");
}

/// `Array.prototype.map` (`#sec-array.prototype.map`) with a script function as callbackfn:
/// it is called with (kValue, k, O) and thisArg as its `this` (step 7.c.ii, "Call(callbackfn,
/// thisArg, « kValue, 𝔽(k), O »)"), only for present indices (step 7.b, HasProperty), up to the
/// length read before the first call (step 2, "Let len be ? LengthOfArrayLike(O)"), and an abrupt
/// completion of the callback is returned (step 7.c.ii, `?`). A callback that is not callable
/// throws a `TypeError` before any call (step 3).
#[test]
fn map_with_a_script_callback() {
    both_answer(
        "[1, 2, 3].map(function (x, i, a) { return x * 10 + i + (a.length * 100); }).join();",
        "\"310,321,332\"",
    );
    both_answer(
        "var t = { k: 5 }; [1, 2].map(function (x) { return this.k + x; }, t).join();",
        "\"6,7\"",
    );
    both_answer(
        "var seen = []; var r = [1, , 3].map(function (x, i) { seen.push(i); return x; }); \
         seen.join() + '|' + r.length + '|' + (1 in r);",
        "\"0,2|3|false\"",
    );
    both_answer(
        "var a = [1, 2]; var calls = 0; \
         a.map(function (x) { calls++; a.push(x); return x; }); calls + ':' + a.length;",
        "\"2:4\"",
    );
    both_answer(
        "var caught; try { [1, 2, 3].map(function (x) { if (x === 2) throw 'two'; return x; }); } \
         catch (e) { caught = e; } caught;",
        "\"two\"",
    );
    both_throw("[1].map(1);", "TypeError");
    both_answer(
        "var calls = 0; try { [].map({}); } catch (e) { calls = e instanceof TypeError; } calls;",
        "true",
    );
    // The `this` of a callback with no thisArg: undefined in strict code, the global in sloppy
    // code (`#sec-ordinarycallbindthis`).
    let (sloppy, strict) = run_both("[0].map(function () { return this === undefined; })[0];");
    assert_eq!(sloppy, Outcome::Normal("false".to_string()));
    assert_eq!(strict, Outcome::Normal("true".to_string()));
}

/// `Function.prototype.call` (`#sec-function.prototype.call`, step 4: "Return ? Call(func,
/// thisArg, args)") and `Function.prototype.bind` (`#sec-function.prototype.bind`, with
/// BoundFunctionCreate, `#sec-boundfunctioncreate`) of a script function: the `this` and
/// arguments it is called with, the bound arguments first, the bound `this` kept, `length` and
/// `name` as steps 5 to 10 of `bind` set them, and a target that is not callable throwing a
/// `TypeError`.
#[test]
fn call_and_bind_of_a_script_function() {
    both_answer(
        "function f(a, b) { return this.v + a + b; } f.call({ v: 1 }, 2, 3);",
        "6",
    );
    both_answer(
        "function f(a, b) { return [this.v, a, b].join(); } \
         var g = f.bind({ v: 'x' }, 'a'); g('b') + '|' + g.call({ v: 'y' }, 'c');",
        "\"x,a,b|x,a,c\"",
    );
    both_answer(
        "function f(a, b, c) {} var g = f.bind(null, 1); g.length + ':' + g.name;",
        "\"2:bound f\"",
    );
    both_answer("function f(a) {} f.bind(null, 1, 2, 3).length;", "0");
    both_answer(
        "var f = function (a, b, c, d, e) { return [a, b, c, d, e].join(); }; \
         f.bind(null, 1, 2)(3, 4) + '|' + f.call(null);",
        "\"1,2,3,4,|,,,,\"",
    );
    // A bound function of a bound function: the arguments of both, in order.
    both_answer(
        "function f(a, b, c) { return a + b + c; } f.bind(null, 'a').bind(null, 'b')('c');",
        "\"abc\"",
    );
    both_throw("(function () {}).bind.call({}, null);", "TypeError");
    both_throw("(function () {}).call.call(1);", "TypeError");
    // `call` with a null `this`: the strict callee gets null, the sloppy one the global object
    // (`#sec-ordinarycallbindthis`, step 6).
    both_answer(
        "function s() { 'use strict'; return this; } s.call(null) === null;",
        "true",
    );
    let (sloppy, strict) =
        run_both("var g = this; function l() { return this; } l.call(null) === g;");
    assert_eq!(sloppy, Outcome::Normal("true".to_string()));
    assert_eq!(
        strict,
        Outcome::Normal("false".to_string()),
        "a strict callee gets null"
    );
    // An exception of the callee comes back through `call`.
    both_answer(
        "var r; try { (function () { throw 7; }).call(null); } catch (e) { r = e; } r;",
        "7",
    );
}

/// A bound script function as a constructor and as the right side of `instanceof`: its
/// [[Construct]] constructs the target with the bound arguments first and the target as
/// newTarget when newTarget is the bound function
/// (`#sec-bound-function-exotic-objects-construct-argumentslist-newtarget`, steps 5 and 6), the
/// object's prototype is the target's `prototype`, and InstanceofOperator falls through to
/// OrdinaryHasInstance, whose step 2 uses the bound target (`#sec-ordinaryhasinstance`).
#[test]
fn bound_script_function_constructs_and_has_instance() {
    both_answer(
        "function P(a, b) { this.a = a; this.b = b; } var B = P.bind({ ignored: true }, 1); \
         var o = new B(2); [o.a, o.b, o.ignored].join() + '|' + (o instanceof P) + '|' + \
         (o instanceof B) + '|' + (Object.getPrototypeOf(o) === P.prototype);",
        "\"1,2,|true|true|true\"",
    );
    both_answer(
        "function P() {} var B = P.bind(null).bind(null); \
         (new B() instanceof P) + ',' + ({} instanceof B) + ',' + (new P() instanceof B);",
        "\"true,false,true\"",
    );
    // A constructor that returns an object gives that object.
    both_answer(
        "function P() { return { made: 1 }; } var B = P.bind(null); new B().made;",
        "1",
    );
    // A bound function of a function that is not a constructor is not one either.
    both_throw("var B = Math.max.bind(null); new B();", "TypeError");
    // `instanceof` of a bound function whose target's prototype is not an object throws.
    both_throw(
        "function P() {} P.prototype = 1; var B = P.bind(null); ({}) instanceof B;",
        "TypeError",
    );
}

/// `toJSON` and the replacer of `JSON.stringify` as script functions (`#sec-json.stringify`,
/// `#sec-serializejsonproperty`): `toJSON` is called with the key as its argument and the value
/// as `this` (step 2.b.i, "Set value to ? Call(toJSON, value, « key »)"), the replacer with
/// the holder as `this` and (key, value) (step 3.a, "Set value to ? Call(state.[[ReplacerFunction]],
/// holder, « key, value »)"), first with the key "" and the wrapper; an `undefined` from the
/// replacer leaves the property out, and an exception of either is thrown by `stringify`.
#[test]
fn to_json_and_replacer_as_script_functions() {
    both_answer(
        "JSON.stringify({ a: { toJSON: function (k) { return k + '!' + (this.x || 0); }, x: 2 }, \
         b: [ { toJSON: function (k) { return typeof k + k; } } ] });",
        "\"{\"a\":\"a!2\",\"b\":[\"string0\"]}\"",
    );
    both_answer(
        "var keys = []; JSON.stringify({ a: 1, b: [2, 3], c: 'x' }, function (k, v) { \
           keys.push(k === '' ? '<>' : k); return k === 'c' ? undefined : v; }) + '|' + keys.join();",
        "\"{\"a\":1,\"b\":[2,3]}|<>,a,b,0,1,c\"",
    );
    both_answer(
        "var holders = []; var o = { p: 1 }; JSON.stringify(o, function (k, v) { \
           holders.push(k === '' ? this[''] === o : this === o); return v; }); holders.join();",
        "\"true,true\"",
    );
    both_answer(
        "JSON.stringify({ a: 1, b: 2 }, function (k, v) { return typeof v === 'number' ? v * 2 : v; });",
        "\"{\"a\":2,\"b\":4}\"",
    );
    // toJSON first, then the replacer sees what toJSON returned.
    both_answer(
        "JSON.stringify({ d: { toJSON: function () { return 5; } } }, function (k, v) { \
           return k === 'd' ? v + 1 : v; });",
        "\"{\"d\":6}\"",
    );
    both_answer(
        "var r; try { JSON.stringify({ toJSON: function () { throw 'j'; } }); } catch (e) { r = e; } \
         try { JSON.stringify(1, function () { throw r + 'r'; }); } catch (e) { r = e; } r;",
        "\"jr\"",
    );
    // A toJSON that is not callable is ignored.
    both_answer("JSON.stringify({ toJSON: 1 });", "\"{\"toJSON\":1}\"");
}
