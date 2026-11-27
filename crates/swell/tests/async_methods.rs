//! `async` methods of object literals (MethodDefinition : AsyncMethod,
//! `#sec-async-function-definitions`, `#sec-runtime-semantics-methoddefinitionevaluation`,
//! `#sec-definemethodproperty`): the template of a method whose `asynchronous` is set is an
//! `async` function of kind method, made by the instantiation every `async` template takes, and
//! defined on its object by DefineMethod; it runs on the suspended frames of
//! `story:async-functions-and-await` (`docs/design/engine.md`, decision 3, "`async` functions on
//! suspended frames"). `async *m() {}` is tested in `tests/async_generator_methods.rs`. Written by
//! `story:async-methods-in-object-literals`. Run also with the feature
//! `collect-every-safe-point`.
//!
//! A script sets up its calls and its probe; the test reads `String(probe)` before and after one
//! microtask checkpoint, in sloppy and in strict mode.
#![cfg(test)]

use crate::support;

use support::host::{agent, realm};
use support::script::{Outcome, eval_text};
use swell::Outcome as HostOutcome;

/// `source` run in a fresh agent, then `String(probe)` read before and after one microtask
/// checkpoint, which must answer `Normal`.
fn around_a_checkpoint(source: &str, probe: &str) -> (Outcome, Outcome) {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let ran = eval_text(&mut agent, &realm, source);
    assert!(matches!(ran, Outcome::Normal(_)), "{source}: {ran:?}");
    let read = format!("String({probe})");
    let before = eval_text(&mut agent, &realm, &read);
    let checkpoint = agent.perform_microtask_checkpoint().unwrap();
    assert!(
        matches!(checkpoint, HostOutcome::Normal(_)),
        "{checkpoint:?}"
    );
    let after = eval_text(&mut agent, &realm, &read);
    (before, after)
}

/// [`around_a_checkpoint`] as written and with a `"use strict";` prologue: each reads `before`
/// and then `after`.
fn both_settle(source: &str, probe: &str, before: &str, after: &str) {
    let expected = (
        Outcome::Normal(format!("\"{before}\"")),
        Outcome::Normal(format!("\"{after}\"")),
    );
    assert_eq!(around_a_checkpoint(source, probe), expected, "{source}");
    let strict = format!("\"use strict\";\n{source}");
    assert_eq!(around_a_checkpoint(&strict, probe), expected, "{strict}");
}

/// AsyncMethod's MethodDefinitionEvaluation and EvaluateAsyncFunctionBody
/// (`#sec-runtime-semantics-evaluateasyncfunctionbody`): a call of an `async` method runs its body
/// to the first `await` with the `this` and the arguments of the call and returns a promise; a
/// job resumes it, and its return value fulfils the promise. A throw in the body and a default
/// that throws reject it, and the call returns normally.
#[test]
fn an_async_method_returns_a_promise_and_awaits() {
    both_settle(
        "var log = [], result = 'pending', thrown = 'pending', dflt = 'pending';\n\
         function boom() { throw new RangeError('default'); }\n\
         var o = { base: 1,\n\
           async m(a) { log.push('call'); var v = await a; log.push('after');\n\
             return v + this.base + arguments.length; },\n\
           async t() { await null; throw new TypeError('body'); },\n\
           async d(x = boom()) { log.push('not here'); } };\n\
         var p = o.m(40);\n\
         log.push('script');\n\
         p.then(function (v) { result = 'fulfilled ' + v; });\n\
         o.t().then(null, function (e) { thrown = e.name + ' ' + e.message; });\n\
         o.d().then(null, function (e) { dflt = e.name + ' ' + e.message; });\n\
         var shape = p instanceof Promise && Object.getPrototypeOf(p) === Promise.prototype;",
        "shape + ' ' + log.join() + ' ' + result + ' ' + thrown + ' ' + dflt",
        "true call,script pending pending pending",
        "true call,script,after fulfilled 42 TypeError body RangeError default",
    );
}

/// MakeMethod (`#sec-makemethod`) gives the method its object as [[HomeObject]], which `super`
/// reads (`#sec-super-keyword-runtime-semantics-evaluation`): in an `async` method as in a plain
/// one, also when the literal is eval code, `super.x` reads the property of the prototype of
/// that object, and nothing is refused.
#[test]
fn super_in_an_async_method_reads_its_home_object() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let before = agent.refusals();
    for source in [
        "({ __proto__: { x: 'p' }, async m() { return super.x; } })",
        "({ __proto__: { x: 'p' }, m() { return super.x; } })",
    ] {
        assert_eq!(
            eval_text(
                &mut agent,
                &realm,
                &format!(
                    "var read = 'pending';\n\
                     (function () {{ var r = eval(\"{source}\").m();\n\
                       if (r instanceof Promise) r.then(function (v) {{ read = v; }});\n\
                       else read = r; }})();\n\
                     read;"
                ),
            ),
            Outcome::Normal(
                if source.contains("async") {
                    "\"pending\""
                } else {
                    "\"p\""
                }
                .to_string()
            ),
            "{source}"
        );
        let checkpoint = agent.perform_microtask_checkpoint().unwrap();
        assert!(
            matches!(checkpoint, HostOutcome::Normal(_)),
            "{checkpoint:?}"
        );
        assert_eq!(
            eval_text(&mut agent, &realm, "read"),
            Outcome::Normal("\"p\"".to_string()),
            "{source}"
        );
    }
    assert_eq!(agent.refusals(), before);
}

/// InstantiateAsyncFunctionObject's shape through DefineMethod
/// (`#sec-runtime-semantics-methoddefinitionevaluation`, AsyncMethod): the [[Prototype]] is
/// `%AsyncFunction.prototype%`, there is no `prototype` property and no [[Construct]], so `new`
/// throws a `TypeError`; the method is a data property of its object, writable, enumerable and
/// configurable (`#sec-definemethodproperty`), and its source text starts at `async`
/// (`#sec-function.prototype.tostring`).
#[test]
fn an_async_method_has_no_prototype_and_no_construct() {
    both_settle(
        "var o = { a: 1, async m(x, y) { await x; }, b: 2 }, f = o.m, constructed = 'none';\n\
         try { new o.m(); constructed = 'constructed'; }\n\
         catch (e) { constructed = e instanceof TypeError; }\n\
         var d = Object.getOwnPropertyDescriptor(o, 'm');\n\
         var shape = Object.getPrototypeOf(f) === Object.getPrototypeOf(async function () {}) &&\n\
           !Object.prototype.hasOwnProperty.call(f, 'prototype') &&\n\
           Object.prototype.toString.call(f) === '[object AsyncFunction]' &&\n\
           d.value === f && d.writable && d.enumerable && d.configurable &&\n\
           Object.keys(o).join() === 'a,m,b' && f.length === 2 && f.name === 'm' &&\n\
           f.toString() === 'async m(x, y) { await x; }';",
        "shape + ' ' + constructed",
        "true true",
        "true true",
    );
}

/// SetFunctionName (`#sec-setfunctionname`) through DefineMethod: an `async` method is named by
/// its key, a computed one by the key computed when the literal is evaluated, a Symbol key as
/// `[description]`; `get`, `set`, `async` and `await` are names like any other. The
/// ComputedPropertyName is read with the [Await] of the code around the literal
/// (`ClassElementName[?Yield, ?Await]`), so in an `async` function an `await` there suspends that
/// function, and the awaited value is the key.
#[test]
fn a_computed_async_method_gets_its_name() {
    both_settle(
        "var k = 'dyn', s = Symbol('sym'), names = 'pending';\n\
         var o = { async [k + 1]() {}, async [s](a, b = 0) {}, async plain(a, b) {},\n\
           async 0() {}, async 'str'() {}, async get() {}, async set() {}, async async() {},\n\
           async await() {} };\n\
         async function outer() { return { async [await 'awaited']() { return 7; } }; }\n\
         outer().then(function (made) {\n\
           return made.awaited().then(function (v) { names = Object.keys(made).join() + ' ' + v; });\n\
         });\n\
         var direct = o.dyn1.name === 'dyn1' && o[s].name === '[sym]' && o.plain.name === 'plain' &&\n\
           o[0].name === '0' && o.str.name === 'str' && o.get.name === 'get' &&\n\
           o.set.name === 'set' && o.async.name === 'async' && o.await.name === 'await' &&\n\
           o.plain.length === 2 && o[s].length === 1;",
        "direct + ' ' + names",
        "true pending",
        "true awaited 7",
    );
}
