//! Proxy objects (`#sec-proxy-object-internal-methods-and-internal-slots`),
//! `story:proxy-objects-and-access-traps`: the constructor `Proxy` (`#sec-proxy-target-handler`)
//! and `Proxy.revocable` (`#sec-proxy.revocable`), the traps `get`, `set`, `has`,
//! `deleteProperty`, `apply` and `construct` with the checks of their clauses (10.5.7 to 10.5.10,
//! 10.5.12 and 10.5.13), the seven other internal methods forwarding to the target after the
//! check of revocation, a Proxy in the prototype chain, IsArray (`#sec-isarray`) and
//! GetFunctionRealm (`#sec-getfunctionrealm`) through a Proxy, and the bounds: every internal
//! method of a Proxy is one re-entry (`docs/design/engine.md`, decision 3, "Depth bounds"), a step
//! before it reads the handler, and its records charged before they exist. Run also with the
//! feature `collect-every-safe-point`.
#![cfg(test)]

use crate::support;

use std::cell::Cell;

use support::host::{agent, agent_with, global, key, limits, object, realm, stack_with_room};
use support::script::{Outcome, define_function, eval, eval_text, run_both};
use support::stack::{
    BUILTINS_REENTRY, on_bounded_stack, reach_through, stack_agent, stack_limits, test_path,
};
use swell::{
    AgentState, Completion, Cx, HostCall, InterruptHandle, Outcome as HostOutcome, Termination,
    Value,
};

/// `source` answers the String `expected` in sloppy and in strict code.
fn answers(source: &str, expected: &str) {
    let (sloppy, strict) = run_both(source);
    let expected = Outcome::Normal(format!("\"{expected}\""));
    assert_eq!(sloppy, expected, "sloppy: {source}");
    assert_eq!(strict, expected, "strict: {source}");
}

/// `source` throws an error named `name` in sloppy and in strict code.
fn throws(source: &str, name: &str) {
    let (sloppy, strict) = run_both(source);
    let expected = Outcome::Throw(name.to_string());
    assert_eq!(sloppy, expected, "sloppy: {source}");
    assert_eq!(strict, expected, "strict: {source}");
}

/// A collection, which must complete.
fn collect(agent: &mut swell::Agent<support::host::TestHost>) {
    assert!(matches!(agent.collect().unwrap(), HostOutcome::Normal(_)));
}

/// `#sec-proxy-target-handler`: a call without `new` throws a TypeError, and so does a target
/// or a handler that is not an Object, for `new Proxy` and for `Proxy.revocable`
/// (`#sec-proxycreate`, steps 1 and 2); a Proxy, a revoked one included, is a target and a
/// handler. `Proxy` has "length" 2, "name" "Proxy", the [[Prototype]] `%Function.prototype%` and
/// no "prototype" (`#sec-properties-of-the-proxy-constructor`); the global property is {
/// [[Writable]]: true, [[Enumerable]]: false, [[Configurable]]: true }. `Proxy.revocable`
/// answers an ordinary object with "proxy" and "revoke", in that order, whose `revoke` has
/// "length" 0, "name" "", no "prototype" and no [[Construct]].
#[test]
fn proxy_requires_new_and_object_arguments() {
    answers(
        "var r = [];\n\
         try { Proxy({}, {}); r.push(\"called\"); } catch (e) { r.push(e.name); }\n\
         var bad = [[1, {}], [{}, 1], [null, {}], [{}, null], [undefined, undefined], [\"s\", {}],\n\
           [{}, Symbol()]];\n\
         for (var i = 0; i < bad.length; i++) {\n\
           try { new Proxy(bad[i][0], bad[i][1]); r.push(\"made\"); } catch (e) { r.push(e.name); }\n\
           try { Proxy.revocable(bad[i][0], bad[i][1]); r.push(\"made\"); } catch (e) { r.push(e.name); }\n\
         }\n\
         var p = new Proxy({}, {});\n\
         r.push(typeof p, Proxy.hasOwnProperty(\"prototype\"), Proxy.length, Proxy.name,\n\
           typeof Proxy.revocable, Proxy.revocable.length, Proxy.revocable.name,\n\
           Object.getPrototypeOf(Proxy) === Function.prototype);\n\
         var g = Object.getOwnPropertyDescriptor(this, \"Proxy\");\n\
         r.push(g.writable, g.enumerable, g.configurable);\n\
         var rv = Proxy.revocable({}, {});\n\
         r.push(Object.keys(rv).join(\"+\"), typeof rv.revoke, rv.revoke.length, rv.revoke.name === \"\",\n\
           Object.getPrototypeOf(rv) === Object.prototype, rv.revoke.hasOwnProperty(\"prototype\"));\n\
         try { new rv.revoke(); r.push(\"made\"); } catch (e) { r.push(e.name); }\n\
         rv.revoke();\n\
         r.push(typeof new Proxy(rv.proxy, rv.proxy), typeof new Proxy(p, p));\n\
         r.join();",
        &format!(
            "TypeError,{},object,false,2,Proxy,function,2,revocable,true,true,false,true,\
             proxy+revoke,function,0,true,true,false,TypeError,object,object",
            vec!["TypeError"; 14].join(",")
        ),
    );
}

/// A handler with no trap: each of the 13 internal methods answers what the target's answers
/// (each clause's "If trap is undefined, then Return ? target.[[...]]"), through the syntax and
/// through `Reflect`, `for`-`in` and `instanceof` included.
#[test]
fn a_proxy_without_traps_forwards_every_operation() {
    answers(
        "var t = { a: 1 }, p = new Proxy(t, {}), r = [];\n\
         r.push(p.a, \"a\" in p, Reflect.has(p, \"b\"));\n\
         p.b = 2; r.push(t.b);\n\
         r.push(delete p.a, \"a\" in t);\n\
         r.push(Object.getPrototypeOf(p) === Object.prototype);\n\
         var q = {}; r.push(Reflect.setPrototypeOf(p, q), Object.getPrototypeOf(t) === q);\n\
         r.push(Object.isExtensible(p));\n\
         Object.defineProperty(p, \"c\", { value: 3, enumerable: true });\n\
         r.push(t.c, JSON.stringify(Object.getOwnPropertyDescriptor(p, \"c\")));\n\
         r.push(Reflect.ownKeys(p).join(\"+\"), Object.keys(p).join(\"+\"));\n\
         r.push(Reflect.preventExtensions(p), Object.isExtensible(t));\n\
         var f = new Proxy(function (x) { return this.k + x; }, {});\n\
         r.push(f.call({ k: 1 }, 2), typeof f);\n\
         function C(v) { this.v = v; }\n\
         var PC = new Proxy(C, {});\n\
         var o = new PC(5);\n\
         r.push(o.v, o instanceof C, o instanceof PC);\n\
         for (var k in p) r.push(\"in:\" + k);\n\
         r.join();",
        "1,true,false,2,true,false,true,true,true,true,3,\
         {\"value\":3,\"writable\":false,\"enumerable\":true,\"configurable\":false},\
         b+c,b+c,true,false,3,function,5,true,true,in:b,in:c",
    );
}

/// The traps `get` (10.5.8), `set` (10.5.9), `has` (10.5.7) and `deleteProperty` (10.5.10) are
/// called with the handler as their this value and « target, key, receiver », « target, key,
/// value, receiver », « target, key » and « target, key »: an index key as a String, a Symbol
/// key as itself, the receiver the Proxy or the one `Reflect` gives; their answers are what the
/// operations answer, and the target is not touched.
#[test]
fn get_set_has_and_delete_traps_receive_target_key_and_receiver() {
    answers(
        "var log = [], t = {}, h, p, other = {};\n\
         h = {\n\
           get: function (target, key, receiver) {\n\
             log.push([\"get\", this === h, target === t, typeof key, String(key), receiver === p,\n\
               receiver === other, arguments.length].join(\"/\"));\n\
             return \"g\";\n\
           },\n\
           set: function (target, key, value, receiver) {\n\
             log.push([\"set\", this === h, target === t, String(key), value, receiver === p,\n\
               receiver === other, arguments.length].join(\"/\"));\n\
             return true;\n\
           },\n\
           has: function (target, key) {\n\
             log.push([\"has\", this === h, target === t, String(key), arguments.length].join(\"/\"));\n\
             return true;\n\
           },\n\
           deleteProperty: function (target, key) {\n\
             log.push([\"deleteProperty\", this === h, target === t, String(key),\n\
               arguments.length].join(\"/\"));\n\
             return true;\n\
           }\n\
         };\n\
         p = new Proxy(t, h);\n\
         var s = Symbol(\"s\");\n\
         var r = [p.x, p[1], p[s], Reflect.get(p, \"y\", other)];\n\
         p.x = 5;\n\
         r.push(Reflect.set(p, \"z\", 6, other));\n\
         r.push(\"x\" in p, Reflect.has(p, 2));\n\
         r.push(delete p.x, Reflect.deleteProperty(p, \"w\"));\n\
         r.push(Object.keys(t).length);\n\
         r.concat(log).join(\",\");",
        "g,g,g,g,true,true,true,true,true,0,\
         get/true/true/string/x/true/false/3,get/true/true/string/1/true/false/3,\
         get/true/true/symbol/Symbol(s)/true/false/3,get/true/true/string/y/false/true/3,\
         set/true/true/x/5/true/false/4,set/true/true/z/6/false/true/4,\
         has/true/true/x/2,has/true/true/2/2,\
         deleteProperty/true/true/x/2,deleteProperty/true/true/w/2",
    );
}

/// The invariants of the six traps (`#sec-invariants-of-the-essential-internal-methods`), each
/// broken once and each a TypeError: `get` reporting another value for a non-configurable,
/// non-writable data property (10.5.8, step 9.a.i) or a value for a non-configurable accessor
/// without a getter (9.b.i); `set` answering true for a non-configurable, non-writable data
/// property with another value (10.5.9, step 10.a.i) or for a non-configurable accessor without
/// a setter (10.b.i); `has` hiding a non-configurable property (10.5.7, step 8.b.i) or a
/// property of a non-extensible target (8.b.iii); `deleteProperty` answering true for a
/// non-configurable property (10.5.10, step 11) or a property of a non-extensible target (step
/// 13); `construct` answering no Object (10.5.13, step 10). The honest answers beside them pass,
/// and the message names the trap and the key.
#[test]
fn the_get_trap_cannot_lie_about_a_non_configurable_non_writable_property() {
    let fixed = "var t = {};\n\
         Object.defineProperty(t, \"v\", { value: 1 });\n\
         Object.defineProperty(t, \"a\", { set: function () {} });\n\
         Object.defineProperty(t, \"w\", { get: function () { return 1; } });\n";
    for broken in [
        "new Proxy(t, { get: function () { return 2; } }).v;",
        "new Proxy(t, { get: function () { return 1; } }).a;",
        "Reflect.set(new Proxy(t, { set: function () { return true; } }), \"v\", 2);",
        "Reflect.set(new Proxy(t, { set: function () { return true; } }), \"w\", 1);",
        "\"v\" in new Proxy(t, { has: function () { return false; } });",
        "\"k\" in new Proxy(Object.preventExtensions({ k: 1 }), { has: function () { return false; } });",
        "Reflect.deleteProperty(new Proxy(t, { deleteProperty: function () { return true; } }), \"v\");",
        "Reflect.deleteProperty(new Proxy(Object.preventExtensions({ k: 1 }),\n\
           { deleteProperty: function () { return true; } }), \"k\");",
        "new (new Proxy(function () {}, { construct: function () { return 1; } }))();",
    ] {
        throws(&format!("{fixed}{broken}"), "TypeError");
    }
    answers(
        &format!(
            "{fixed}var r = [];\n\
             r.push(new Proxy(t, {{ get: function () {{ return 1; }} }}).v);\n\
             r.push(new Proxy(t, {{ get: function () {{ return undefined; }} }}).a);\n\
             r.push(Reflect.set(new Proxy(t, {{ set: function () {{ return true; }} }}), \"v\", 1));\n\
             r.push(Reflect.set(new Proxy(t, {{ set: function () {{ return false; }} }}), \"w\", 1));\n\
             r.push(Reflect.has(new Proxy(t, {{ has: function () {{ return false; }} }}), \"missing\"));\n\
             var c = {{ k: 1 }};\n\
             r.push(Reflect.has(new Proxy(c, {{ has: function () {{ return false; }} }}), \"k\"));\n\
             r.push(Reflect.deleteProperty(new Proxy(c, {{ deleteProperty: function () {{ return true; }} }}), \"k\"));\n\
             r.push(Reflect.deleteProperty(new Proxy(Object.preventExtensions({{}}),\n\
               {{ deleteProperty: function () {{ return true; }} }}), \"none\"));\n\
             try {{ new Proxy(t, {{ get: function () {{ return 2; }} }}).v; }}\n\
             catch (e) {{ r.push(e.message.indexOf(\"get\") >= 0 && e.message.indexOf(\"v\") >= 0); }}\n\
             r.join();"
        ),
        "1,,true,false,false,false,true,true,true",
    );
}

/// `apply` (10.5.12) gets « target, thisArg, argArray » and `construct` (10.5.13) « target,
/// argArray, newTarget », the handler as this value, `argArray` a new Array of the arguments
/// (CreateArrayFromList); their answers are the call's and the construction's. A Proxy has
/// [[Call]] and [[Construct]] only when its target has them (`#sec-proxycreate`, steps 7 and 8),
/// whatever its handler holds.
#[test]
fn apply_and_construct_traps_receive_the_argument_array_and_new_target() {
    answers(
        "var log = [], target = function () { return \"t\"; }, p, o;\n\
         var h = {\n\
           apply: function (t, thisArg, args) {\n\
             log.push([this === h, t === target, thisArg === o, Array.isArray(args), args.length,\n\
               args.join(\"+\"), arguments.length].join(\"/\"));\n\
             return \"applied\";\n\
           },\n\
           construct: function (t, args, newTarget) {\n\
             log.push([this === h, t === target, Array.isArray(args), args.join(\"+\"),\n\
               newTarget === p, newTarget === N, arguments.length].join(\"/\"));\n\
             return { made: true };\n\
           }\n\
         };\n\
         p = new Proxy(target, h);\n\
         o = { m: p };\n\
         function N() {}\n\
         var r = [p(1, 2), o.m(3), p.call(o, 4, 5, 6), Reflect.apply(p, o, []), new p(7).made,\n\
           Reflect.construct(p, [8, 9], N).made];\n\
         r.concat(log).join(\",\");",
        "applied,applied,applied,applied,true,true,\
         true/true/false/true/2/1+2/3,true/true/true/true/1/3/3,true/true/true/true/3/4+5+6/3,\
         true/true/true/true/0//3,true/true/true/7/true/false/3,true/true/true/8+9/false/true/3",
    );
    throws(
        "new (new Proxy(() => 1, { construct: function () { return {}; } }))();",
        "TypeError",
    );
    throws(
        "new Proxy({}, { apply: function () { return 1; } })();",
        "TypeError",
    );
}

/// ValidateNonRevokedProxy (`#sec-validatenonrevokedproxy`): after `revoke()` each of the 13
/// internal methods, reached through the 13 functions of `Reflect`, throws a TypeError; the
/// revoked Proxy keeps its [[Call]], so `typeof` still answers "function".
#[test]
fn a_revoked_proxy_throws_a_type_error_for_every_internal_method() {
    answers(
        "var rv = Proxy.revocable(function () {}, {});\n\
         rv.revoke();\n\
         var p = rv.proxy;\n\
         var calls = [\n\
           function () { return Reflect.apply(p, null, []); },\n\
           function () { return Reflect.construct(p, []); },\n\
           function () { return Reflect.defineProperty(p, \"x\", { value: 1 }); },\n\
           function () { return Reflect.deleteProperty(p, \"x\"); },\n\
           function () { return Reflect.get(p, \"x\"); },\n\
           function () { return Reflect.getOwnPropertyDescriptor(p, \"x\"); },\n\
           function () { return Reflect.getPrototypeOf(p); },\n\
           function () { return Reflect.has(p, \"x\"); },\n\
           function () { return Reflect.isExtensible(p); },\n\
           function () { return Reflect.ownKeys(p); },\n\
           function () { return Reflect.preventExtensions(p); },\n\
           function () { return Reflect.set(p, \"x\", 1); },\n\
           function () { return Reflect.setPrototypeOf(p, null); }\n\
         ];\n\
         var r = [];\n\
         for (var i = 0; i < calls.length; i++) {\n\
           try { calls[i](); r.push(\"answered\"); } catch (e) { r.push(e.name); }\n\
         }\n\
         r.push(typeof p);\n\
         r.join();",
        &format!("{},function", vec!["TypeError"; 13].join(",")),
    );
}

/// The elements of each Array of the revocation test: 1,000, and 50 with the feature
/// `collect-every-safe-point`, where every iteration of the loop that fills them collects.
const ELEMENTS: u64 = if cfg!(feature = "collect-every-safe-point") {
    50
} else {
    1_000
};

/// The bytes of the dense elements of an Array of [`ELEMENTS`] elements: at least a slot of
/// `Option<Value>` per element, what `object/array.rs` stores them in and charges.
fn array_bytes() -> u64 {
    ELEMENTS * size_of::<Option<Value>>() as u64
}

/// `#sec-proxy.revocable`: the revoke function answers `undefined`, a second call too, and
/// after the first the Proxy holds neither target nor handler: a target and a handler reachable
/// only through the Proxy, each an Array of [`ELEMENTS`] elements, are freed by one `collect()`,
/// so `heap_bytes()` falls by at least the bytes of their dense elements (twice
/// [`array_bytes`]); the readings are printed.
#[test]
fn revoke_is_idempotent_and_drops_target_and_handler() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            &format!(
                "function filled() {{ var a = []; for (var i = 0; i < {ELEMENTS}; i++) a.push(i); return a; }}\n\
                 var r = Proxy.revocable(filled(), filled());\n\
                 r.proxy.length;"
            )
        ),
        Outcome::Normal(ELEMENTS.to_string())
    );
    collect(&mut agent);
    let before = agent.heap_bytes().unwrap();
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var answers = [r.revoke(), r.revoke()].map(String).join(\"/\");\n\
             var read;\n\
             try { r.proxy.length; read = \"answered\"; } catch (e) { read = e.name; }\n\
             answers + \"/\" + read;"
        ),
        Outcome::Normal("\"undefined/undefined/TypeError\"".to_string())
    );
    collect(&mut agent);
    let after = agent.heap_bytes().unwrap();
    println!(
        "heap_bytes: {before} with the target and handler alive, {after} after revoke() and one \
         collection; the dense elements of each Array {} bytes",
        array_bytes()
    );
    assert!(
        before.saturating_sub(after) >= 2 * array_bytes(),
        "{before} before, {after} after"
    );
}

/// OrdinaryGet, OrdinaryHasProperty and OrdinarySet (`#sec-ordinaryget`, step 2.c;
/// `#sec-ordinaryhasproperty`, step 4.b; `#sec-ordinarysetwithowndescriptor`, step 2.c) hand a
/// key the object does not have to a Proxy parent's own [[Get]], [[HasProperty]] or [[Set]]
/// with the same receiver: its traps run with the child as the receiver, and the `set` that
/// forwards through `Reflect.set` defines the property on the child.
#[test]
fn a_proxy_in_the_prototype_chain_gets_its_get_trap_called_with_the_receiver() {
    answers(
        "var log = [], child;\n\
         var proto = new Proxy({ inherited: \"i\" }, {\n\
           get: function (t, k, r) { log.push(\"get:\" + String(k) + \":\" + (r === child)); return Reflect.get(t, k, r); },\n\
           has: function (t, k) { log.push(\"has:\" + String(k)); return Reflect.has(t, k); },\n\
           set: function (t, k, v, r) { log.push(\"set:\" + String(k) + \":\" + (r === child)); return Reflect.set(t, k, v, r); }\n\
         });\n\
         child = Object.create(proto);\n\
         child.own = \"o\";\n\
         var r = [child.own, child.inherited, child.missing, \"inherited\" in child, \"own\" in child];\n\
         child.added = 1;\n\
         r.push(Reflect.getOwnPropertyDescriptor(child, \"added\") !== undefined,\n\
           Reflect.getOwnPropertyDescriptor(proto, \"added\") === undefined);\n\
         r.concat(log).join(\",\");",
        "o,i,,true,true,true,true,set:own:true,get:inherited:true,get:missing:true,has:inherited,\
         set:added:true",
    );
}

/// IsArray (`#sec-isarray`, step 3) answers IsArray of a Proxy's target, through a chain of
/// proxies, and throws a TypeError for a revoked Proxy anywhere in the chain; `Array.isArray`,
/// `Object.prototype.toString`, `JSON.stringify` and `concat` (IsConcatSpreadable) read it.
#[test]
fn is_array_sees_through_a_chain_of_proxies_and_throws_on_a_revoked_one() {
    answers(
        "var p = [];\n\
         for (var i = 0; i < 5; i++) p = new Proxy(p, {});\n\
         var rv = Proxy.revocable([], {});\n\
         rv.revoke();\n\
         var r = [Array.isArray(p), Array.isArray(new Proxy({}, {})), Object.prototype.toString.call(p),\n\
           JSON.stringify(new Proxy([1, 2], {})), [0].concat(new Proxy([1, 2], {})).length];\n\
         try { Array.isArray(rv.proxy); r.push(\"answered\"); } catch (e) { r.push(e.name); }\n\
         try { Array.isArray(new Proxy(rv.proxy, {})); r.push(\"answered\"); } catch (e) { r.push(e.name); }\n\
         r.join();",
        "true,false,[object Array],[1,2],3,TypeError,TypeError",
    );
}

/// `typeof` (`#sec-typeof-operator`) of a Proxy with [[Call]] is "function", also when revoked;
/// `Function.prototype.toString` of it is a NativeFunction (`#sec-function.prototype.tostring`,
/// step 4) and of a Proxy without [[Call]] a TypeError; `Object.prototype.toString` gives a
/// callable Proxy the builtinTag "Function".
#[test]
fn typeof_and_to_string_of_a_callable_proxy() {
    answers(
        "var f = new Proxy(function () {}, {}), o = new Proxy({}, {}), g = new Proxy(f, {});\n\
         var rv = Proxy.revocable(function () {}, {});\n\
         rv.revoke();\n\
         var r = [typeof f, typeof o, typeof g, typeof rv.proxy, Function.prototype.toString.call(f),\n\
           Function.prototype.toString.call(g), Object.prototype.toString.call(f),\n\
           Object.prototype.toString.call(o), Function.prototype.toString.call(rv.proxy)];\n\
         try { Function.prototype.toString.call(o); r.push(\"answered\"); } catch (e) { r.push(e.name); }\n\
         r.join(\"|\");",
        "function|object|function|function|function () { [native code] }|\
         function () { [native code] }|[object Function]|[object Object]|\
         function () { [native code] }|TypeError",
    );
}

/// GetFunctionRealm (`#sec-getfunctionrealm`, step 3): a Proxy of a function of another realm,
/// as the `newTarget` of `Reflect.construct`, whose "prototype" is no object, gives the new
/// object the `%Object.prototype%` of that realm, through one Proxy and through two; a Proxy
/// revoked by its own `get` trap before GetFunctionRealm reads it throws a TypeError.
#[test]
fn get_function_realm_follows_the_proxy_target() {
    let mut agent = agent().unwrap();
    let here = realm(&mut agent);
    let there = realm(&mut agent);
    let HostOutcome::Normal(function_root) = eval(
        &mut agent,
        &there,
        "(function () { function F() {} F.prototype = null; return F; })();",
    ) else {
        panic!("no F");
    };
    let HostOutcome::Normal(prototype_root) = eval(&mut agent, &there, "Object.prototype;") else {
        panic!("no Object.prototype");
    };
    let global_root = global(&mut agent, &here);
    agent
        .with_cx(&here, |cx| {
            let global = object(cx, &global_root);
            for (name, root) in [
                ("F", &function_root),
                ("otherObjectPrototype", &prototype_root),
            ] {
                let value = cx.get(root).unwrap();
                let name = key(cx, name);
                assert_eq!(cx.set(global, name, value, true).unwrap(), Ok(true));
            }
        })
        .unwrap();
    assert_eq!(
        eval_text(
            &mut agent,
            &here,
            "var r = [];\n\
             var made = Reflect.construct(function () {}, [], new Proxy(F, {}));\n\
             r.push(Object.getPrototypeOf(made) === otherObjectPrototype,\n\
               Object.getPrototypeOf(made) === Object.prototype);\n\
             var twice = Reflect.construct(function () {}, [], new Proxy(new Proxy(F, {}), {}));\n\
             r.push(Object.getPrototypeOf(twice) === otherObjectPrototype);\n\
             var rv = Proxy.revocable(F, { get: function () { rv.revoke(); return undefined; } });\n\
             try { Reflect.construct(function () {}, [], rv.proxy); r.push(\"made\"); }\n\
             catch (e) { r.push(e.name); }\n\
             r.join();"
        ),
        Outcome::Normal("\"true,false,true,TypeError\"".to_string())
    );
}

/// The body of [`a_chain_of_proxies_with_trapping_get_deeper_than_the_reentry_limit_throws_a_catchable_range_error`].
fn trapping_chain() {
    let (mut agent, realm) = stack_agent(BUILTINS_REENTRY);
    let length = BUILTINS_REENTRY + 10;
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            &format!(
                "var h = {{ get: function (t, k, r) {{ return Reflect.get(t, k, r); }} }};\n\
                 function chain(n) {{ var p = {{ x: 1 }}; for (var i = 0; i < n; i++) p = new Proxy(p, h); return p; }}\n\
                 var caught;\n\
                 try {{ chain({length}).x; caught = \"none\"; }}\n\
                 catch (e) {{ caught = e instanceof RangeError ? \"RangeError\" : String(e); }}\n\
                 caught + \"/\" + chain(10).x;"
            )
        ),
        Outcome::Normal("\"RangeError/1\"".to_string())
    );
}

/// The epic's acceptance 5 (`docs/design/engine.md`, decision 3, "Depth bounds": "`Proxy` trap
/// chains: each step that calls script is a re-entry"): `BUILTINS_REENTRY + 10` proxies, each
/// with a `get` trap that calls `Reflect.get(target, key, receiver)`, read through the last one
/// inside `try`, end in a RangeError the script catches, on seven eighths of
/// `min_stack_bytes()`; the script answers normally and a chain of 10 reads its value after.
#[test]
fn a_chain_of_proxies_with_trapping_get_deeper_than_the_reentry_limit_throws_a_catchable_range_error()
 {
    let stack = stack_with_room(stack_limits(BUILTINS_REENTRY));
    on_bounded_stack(
        test_path!(
            "a_chain_of_proxies_with_trapping_get_deeper_than_the_reentry_limit_throws_a_catchable_range_error"
        ),
        stack,
        trapping_chain,
    )
    .unwrap();
}

/// The body of [`a_chain_of_trapless_proxies_longer_than_the_reentry_limit_throws_a_range_error`].
fn trapless_chain() {
    let (mut agent, realm) = stack_agent(BUILTINS_REENTRY);
    let long = BUILTINS_REENTRY + 10;
    let short = BUILTINS_REENTRY - 1;
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            &format!(
                "function chain(n) {{ var p = {{ x: 1 }}; for (var i = 0; i < n; i++) p = new Proxy(p, {{}}); return p; }}\n\
                 var caught;\n\
                 try {{ chain({long}).x; caught = \"none\"; }}\n\
                 catch (e) {{ caught = e instanceof RangeError ? \"RangeError\" : String(e); }}\n\
                 caught + \"/\" + chain({short}).x;"
            )
        ),
        Outcome::Normal("\"RangeError/1\"".to_string())
    );
}

/// A chain of proxies that calls no script (every handler `{}`), each the target of the next,
/// is a re-entry per Proxy (decision D3 of the decomposition, default A): `BUILTINS_REENTRY +
/// 10` of them end in the RangeError of "Depth bounds", not in a Rust stack overflow, on seven
/// eighths of `min_stack_bytes()`; one level shorter than `reentry_depth` the read answers the
/// value.
#[test]
fn a_chain_of_trapless_proxies_longer_than_the_reentry_limit_throws_a_range_error() {
    let stack = stack_with_room(stack_limits(BUILTINS_REENTRY));
    on_bounded_stack(
        test_path!(
            "a_chain_of_trapless_proxies_longer_than_the_reentry_limit_throws_a_range_error"
        ),
        stack,
        trapless_chain,
    )
    .unwrap();
}

/// The body of the stack test through a `get` trap.
fn reach_through_get_trap() {
    reach_through("new Proxy({}, { get: f }).x", BUILTINS_REENTRY);
}

/// `Limits::min_stack_bytes` through the `get` trap of a Proxy (`tests/support/stack.rs`): on
/// seven eighths of `min_stack_bytes()` a script reaches `reentry_depth` re-entries through
/// `new Proxy({}, { get: f }).x`, the trap's call being the re-entry of the [[Get]] that calls
/// it, and parses 1,000 nested brackets at the deepest, without a stack overflow.
#[test]
fn the_get_trap_reaches_the_reentry_limit_on_the_promised_stack() {
    let stack = stack_with_room(stack_limits(BUILTINS_REENTRY));
    on_bounded_stack(
        test_path!("the_get_trap_reaches_the_reentry_limit_on_the_promised_stack"),
        stack,
        reach_through_get_trap,
    )
    .unwrap();
}

/// The step budget of the step test: 10,000 steps per entry.
const STEPS: u64 = 10_000;

/// Every internal method of a Proxy spends a step before it reads the handler: a loop that
/// reads a key through a chain of 100 trapless proxies ends in `Terminated(StepBudget)` with
/// `steps_per_entry` 10,000, having spent at most those steps, and the agent runs the next
/// entry.
#[test]
fn a_proxy_chain_walk_ends_at_the_step_budget() {
    let mut budget = limits();
    budget.steps_per_entry = STEPS;
    let mut agent = agent_with(budget).unwrap();
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var p = { x: 1 };\n\
             for (var i = 0; i < 100; i++) p = new Proxy(p, {});\n\
             for (;;) p.x;"
        ),
        Outcome::Terminated(Termination::StepBudget)
    );
    let spent = agent.last_entry_steps().unwrap();
    println!("{spent} steps of {STEPS}");
    assert!(spent <= STEPS, "{spent} steps");
    assert_eq!(agent.state(), AgentState::Alive);
    assert_eq!(
        eval_text(&mut agent, &realm, "p.x;"),
        Outcome::Normal("1".to_string())
    );
}

/// The memory a run of the memory test has beyond the reading before it: 1 MiB, and 64 KiB with
/// the feature `collect-every-safe-point`, where every iteration of its loop collects.
const MEMORY_ROOM: u64 = if cfg!(feature = "collect-every-safe-point") {
    1 << 16
} else {
    1 << 20
};

/// Proxies made in a loop and kept in an Array, with steps to spare and [`MEMORY_ROOM`] of
/// memory over the reading of a realm after a first entry of the same script over 8 proxies
/// (which grows the arenas), end in `Terminated(MemoryBudget)`: each record is charged before
/// it exists. The script holds nothing in a global; once the host released the script's values,
/// one collection brings `live_bytes()` back within the reading taken before the script;
/// `heap_bytes()` keeps the capacity the arenas grew to, which stays charged while the agent
/// lives (decision 2, "Two readings of the heap"). The readings are printed.
#[test]
fn proxies_made_in_a_loop_end_at_the_memory_budget() {
    let script = |count: &str| {
        format!(
            "(function (n) {{ var a = []; for (var i = 0; i < n; i++) a.push(new Proxy({{}}, {{}}));\n\
               return a.length; }})({count});"
        )
    };
    let mut budget = limits();
    budget.steps_per_entry = 1 << 40;
    let mut probe = agent_with(budget).unwrap();
    let probe_realm = realm(&mut probe);
    assert_eq!(
        eval_text(&mut probe, &probe_realm, &script("8")),
        Outcome::Normal("8".to_string())
    );
    collect(&mut probe);
    budget.memory_bytes = probe.heap_bytes().unwrap() + MEMORY_ROOM;
    let mut agent = agent_with(budget).unwrap();
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(&mut agent, &realm, &script("8")),
        Outcome::Normal("8".to_string())
    );
    collect(&mut agent);
    let before = agent.live_bytes().unwrap();
    let heap_before = agent.heap_bytes().unwrap();
    let outcome = eval(&mut agent, &realm, &script("Infinity"));
    assert!(
        matches!(outcome, HostOutcome::Terminated(Termination::MemoryBudget)),
        "{outcome:?}"
    );
    let terminated = agent.heap_bytes().unwrap();
    drop(outcome);
    collect(&mut agent);
    let after = agent.live_bytes().unwrap();
    let heap_after = agent.heap_bytes().unwrap();
    println!(
        "live_bytes: {before} before the script, {after} after one collection; heap_bytes: \
         {heap_before} before, {terminated} at the termination, {heap_after} after; budget {}",
        budget.memory_bytes
    );
    assert!(
        terminated <= budget.memory_bytes,
        "{terminated} at the termination"
    );
    assert!(after <= before, "{before} before, {after} after");
    assert_eq!(agent.state(), AgentState::Alive);
    assert_eq!(
        eval_text(&mut agent, &realm, &script("2")),
        Outcome::Normal("2".to_string())
    );
}

thread_local! {
    /// The handle `interrupt_now` sets.
    static HANDLE: Cell<Option<InterruptHandle>> = const { Cell::new(None) };
}

/// A host function that sets its agent's interrupt flag.
fn interrupt_now(
    _cx: &mut Cx<'_, support::host::TestHost>,
    _call: HostCall<'_>,
) -> Completion<Value> {
    HANDLE.with(|slot| {
        if let Some(handle) = slot.take() {
            handle.interrupt();
        }
    });
    Ok(Value::Undefined)
}

/// An interrupt set by a host function that a `get` trap calls ends the run in
/// `Terminated(Interrupted)` (`docs/design/host-interface.md`, section 10): the read does not
/// answer, the statement after it does not run, and the agent runs the next entry.
#[test]
fn a_get_trap_ends_at_an_interrupt() {
    let mut agent = agent_with(limits()).unwrap();
    let realm = realm(&mut agent);
    define_function(&mut agent, &realm, "interruptNow", 0, interrupt_now, false);
    HANDLE.with(|slot| slot.set(Some(agent.interrupt_handle())));
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var after = 0;\n\
             var p = new Proxy({}, { get: function () { interruptNow(); return 1; } });\n\
             p.x;\n\
             after = 1;"
        ),
        Outcome::Terminated(Termination::Interrupted)
    );
    assert_eq!(agent.state(), AgentState::Alive);
    assert_eq!(
        eval_text(&mut agent, &realm, "after;"),
        Outcome::Normal("0".to_string())
    );
}

/// The `reentry_depth` of [`proxy_nesting_and_activations_count_together`], and the levels its
/// script recurses through `reenter` before it reads.
const NESTING_DEPTH: u32 = 40;
const NESTING_LEVELS: u32 = 10;

/// The re-entries of the proxies of a chain and the activations below it count together: a
/// script recurses through the host function `reenter` to k levels (k activations), then reads
/// a key through a chain of n trapless proxies whose last target has a getter. With k + n + 1
/// (the getter's call) passing `reentry_depth` the read throws the RangeError; with one Proxy
/// fewer it answers the getter's value, and again after both, so no count of the proxies is
/// left behind.
#[test]
fn proxy_nesting_and_activations_count_together() {
    let (mut agent, realm) = stack_agent(NESTING_DEPTH);
    let chain = NESTING_DEPTH - NESTING_LEVELS;
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            &format!(
                "var k = {NESTING_LEVELS};\n\
                 function readAt(n) {{\n\
                   var t = {{}};\n\
                   Object.defineProperty(t, \"x\", {{ get: function () {{ return \"got\"; }} }});\n\
                   var p = t;\n\
                   for (var i = 0; i < n; i++) p = new Proxy(p, {{}});\n\
                   var level = 0, answer = \"none\";\n\
                   function down() {{\n\
                     level++;\n\
                     if (level < k) {{ reenter(down); return; }}\n\
                     try {{ answer = p.x; }} catch (e) {{ answer = e.name; }}\n\
                   }}\n\
                   reenter(down);\n\
                   return answer;\n\
                 }}\n\
                 [readAt({chain}), readAt({chain} - 1), readAt({chain} - 1)].join();"
            )
        ),
        Outcome::Normal("\"RangeError,got,got\"".to_string())
    );
}

/// Each of the six traps reads the target's facts only at the steps of its clause and under
/// their conditions (10.5.7 to 10.5.10, 10.5.12, 10.5.13). The target `t` is itself a Proxy made
/// by `Proxy.revocable(plain, logger)`, whose six traps log their names and forward through
/// `Reflect`. The outer handler's trap revokes `t` before it answers, so a read of the target
/// after the trap throws the TypeError of a revoked Proxy, and an answer that reads nothing
/// comes back: `get` throws (step 8 reads), `has` true answers, false throws; `set` and
/// `deleteProperty` false answer, true throw; `apply` and `construct` answer the trap's result;
/// each log stays empty. With an outer handler `{}` each operation forwards to `t` and the log
/// is exactly that trap's name. The seven other methods of `t` forward without reading its
/// handler, so the reads of IsExtensible by `has` and `deleteProperty` are not logged here.
#[test]
fn the_target_is_read_only_at_the_steps_of_each_clause() {
    answers(
        "function setup() {\n\
           var log = [];\n\
           var logger = {\n\
             get: function (t, k, r) { log.push(\"get\"); return Reflect.get(t, k, r); },\n\
             set: function (t, k, v, r) { log.push(\"set\"); return Reflect.set(t, k, v, r); },\n\
             has: function (t, k) { log.push(\"has\"); return Reflect.has(t, k); },\n\
             deleteProperty: function (t, k) { log.push(\"deleteProperty\"); return Reflect.deleteProperty(t, k); },\n\
             apply: function (t, self, args) { log.push(\"apply\"); return Reflect.apply(t, self, args); },\n\
             construct: function (t, args) { log.push(\"construct\"); return Reflect.construct(t, args); }\n\
           };\n\
           var inner = Proxy.revocable(function () {}, logger);\n\
           return { t: inner.proxy, revoke: inner.revoke, log: log };\n\
         }\n\
         function outcome(f) {\n\
           try { var v = f(); return typeof v === \"object\" ? \"object\" : String(v); }\n\
           catch (e) { return e.name; }\n\
         }\n\
         var r = [];\n\
         function revoking(name, answer, op) {\n\
           var s = setup(), h = {};\n\
           h[name] = function () { s.revoke(); return answer; };\n\
           var o = new Proxy(s.t, h);\n\
           r.push(name + \":\" + outcome(function () { return op(o); }) + \":[\" + s.log.join(\"+\") + \"]\");\n\
         }\n\
         var made = {};\n\
         revoking(\"get\", 1, function (o) { return o.x; });\n\
         revoking(\"has\", true, function (o) { return \"x\" in o; });\n\
         revoking(\"has\", false, function (o) { return \"x\" in o; });\n\
         revoking(\"set\", false, function (o) { return Reflect.set(o, \"x\", 1); });\n\
         revoking(\"set\", true, function (o) { return Reflect.set(o, \"x\", 1); });\n\
         revoking(\"deleteProperty\", false, function (o) { return Reflect.deleteProperty(o, \"x\"); });\n\
         revoking(\"deleteProperty\", true, function (o) { return Reflect.deleteProperty(o, \"x\"); });\n\
         revoking(\"apply\", \"a\", function (o) { return o(); });\n\
         revoking(\"construct\", made, function (o) { return new o() === made; });\n\
         function forwarding(name, op) {\n\
           var s = setup();\n\
           var o = new Proxy(s.t, {});\n\
           r.push(name + \":\" + outcome(function () { return op(o); }) + \":[\" + s.log.join(\"+\") + \"]\");\n\
         }\n\
         forwarding(\"get\", function (o) { return o.x; });\n\
         forwarding(\"has\", function (o) { return \"x\" in o; });\n\
         forwarding(\"set\", function (o) { o.x = 1; return true; });\n\
         forwarding(\"deleteProperty\", function (o) { return delete o.x; });\n\
         forwarding(\"apply\", function (o) { return o(); });\n\
         forwarding(\"construct\", function (o) { return typeof new o(); });\n\
         r.join(\" \");",
        "get:TypeError:[] has:true:[] has:TypeError:[] set:false:[] set:TypeError:[] \
         deleteProperty:false:[] deleteProperty:TypeError:[] apply:a:[] construct:true:[] \
         get:undefined:[get] has:false:[has] set:true:[set] deleteProperty:true:[deleteProperty] \
         apply:undefined:[apply] construct:object:[construct]",
    );
}
