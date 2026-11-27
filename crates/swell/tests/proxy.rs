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
///
/// What the script between the readings makes besides its subject (the Array of the answers,
/// the TypeError, the Strings of the answer) is made once before the first reading: the same
/// script runs first on a spare revocable Proxy, and the collection before the reading frees
/// what it made, so the arenas have room for the second run. Measured on 2026-11-26 at
/// `SWELL_TEST_REALM_FUNCTIONS` 128, with the script run only between the readings: its
/// Strings grew the arenas, and `heap_bytes()` read 457,412 before and 528,696 after in the
/// default mode, 426,688 and 528,692 in the mode `collect-every-safe-point`.
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
                 var r = Proxy.revocable(filled(), filled()), spare = Proxy.revocable([], {{}});\n\
                 function revokeAndRead(s) {{\n\
                   var answers = [s.revoke(), s.revoke()].map(String).join(\"/\");\n\
                   var read;\n\
                   try {{ s.proxy.length; read = \"answered\"; }} catch (e) {{ read = e.name; }}\n\
                   return answers + \"/\" + read;\n\
                 }}\n\
                 r.proxy.length;"
            )
        ),
        Outcome::Normal(ELEMENTS.to_string())
    );
    let revoked = Outcome::Normal("\"undefined/undefined/TypeError\"".to_string());
    assert_eq!(
        eval_text(&mut agent, &realm, "revokeAndRead(spare);"),
        revoked
    );
    collect(&mut agent);
    let before = agent.heap_bytes().unwrap();
    assert_eq!(eval_text(&mut agent, &realm, "revokeAndRead(r);"), revoked);
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

/// A chain test's route: each Proxy of the chain has `handler`, the last one is over `base`, and
/// `op` runs on the first one, `p`; the base's code calls `parseDeep`, so the parser runs at the
/// deepest point. A chain of `n` proxies costs `per_level` re-entries per Proxy (2 for
/// `construct`, whose clause reads "prototype" of the chain again at its base) and `extra` more
/// at its deepest point, so the longest chain `reentry_depth` allows is
/// `(reentry_depth - extra) / per_level` ([`allowed_depth`]). The values are those the search
/// this computation replaced found in both modes: 1,000 or 999 at `reentry_depth` 1,000, 50 or
/// 49 at 50, and 500 and 25 for `construct`.
struct Route {
    handler: &'static str,
    base: &'static str,
    op: &'static str,
    per_level: u32,
    extra: u32,
}

/// The longest chain of `route` that `reentry_depth` allows.
fn allowed_depth(route: &Route, reentry_depth: u32) -> u32 {
    (reentry_depth - route.extra) / route.per_level
}

/// A chain of `route` at the depth `reentry_depth` allows answers `op`, and one Proxy more
/// throws the RangeError of `reentry_depth` (not a stack overflow): two attempts, the depth
/// computed, not searched. The agent's `reentry_depth` is `reentry_depth`.
fn chain_at_the_bound(
    agent: &mut swell::Agent<support::host::TestHost>,
    realm: &swell::Realm,
    route: &Route,
    reentry_depth: u32,
) {
    let depth = allowed_depth(route, reentry_depth);
    let Route {
        handler, base, op, ..
    } = route;
    let script = format!(
        "var h = {handler};\n\
         var base = {base};\n\
         function chain(n) {{ var p = base; for (var i = 0; i < n; i++) p = new Proxy(p, h); return p; }}\n\
         function attempt(n) {{ var p = chain(n); try {{ {op}; return \"ok\"; }} catch (e) {{ return e instanceof RangeError ? \"RangeError\" : String(e); }} }}\n\
         attempt({depth}) + \"/\" + attempt({depth} + 1);"
    );
    assert_eq!(
        eval_text(agent, realm, &script),
        Outcome::Normal("\"ok/RangeError\"".to_string()),
        "{op} with {handler} at {depth} of {reentry_depth}"
    );
}

/// The `reentry_depth` of the chain tests whose clause reads the chain below again after each
/// trap: the checks of `get`, `set` and `deleteProperty` read the target's descriptor, those of
/// `getPrototypeOf` and `ownKeys` its IsExtensible, through every Proxy below. A chain that
/// answers costs the square of its length there: 1.8 to 2.5 CPU seconds for one attempt at
/// 1,000 in the default mode, against a quarter of it at 500. `min_stack_bytes()` grows by the
/// same bytes per re-entry at either depth, so the chain at the bound proves the same promise
/// per level.
#[cfg(not(feature = "collect-every-safe-point"))]
const SQUARE_REENTRY: u32 = BUILTINS_REENTRY / 2;

/// One stack test per route in the default mode: [`chain_at_the_bound`] at `BUILTINS_REENTRY`
/// (or the depth given after the route) in a child process on seven eighths of
/// `min_stack_bytes()` of that depth. With the feature `collect-every-safe-point` three tests run
/// every route instead (`combined_chain_test`, a part of the routes each): the stack per level
/// does not depend on the collector, and a chain test per route there took 2.4 to 4.6 CPU
/// seconds.
macro_rules! chain_test {
    ($name:ident, $route:expr) => {
        chain_test!($name, $route, BUILTINS_REENTRY);
    };
    ($name:ident, $route:expr, $reentry:expr) => {
        #[cfg(not(feature = "collect-every-safe-point"))]
        #[test]
        fn $name() {
            fn body() {
                let (mut agent, realm) = stack_agent($reentry);
                chain_at_the_bound(&mut agent, &realm, &$route, $reentry);
            }
            let stack = stack_with_room(stack_limits($reentry));
            on_bounded_stack(test_path!(stringify!($name)), stack, body).unwrap();
        }
    };
}

const GET_BASE: &str = "{ get x() { parseDeep(); return 1; } }";
const SET_BASE: &str = "{ set x(v) { parseDeep(); } }";
const HAS_BASE: &str = "new Proxy({}, { has: function () { parseDeep(); return true; } })";
const DELETE_BASE: &str =
    "new Proxy({}, { deleteProperty: function () { parseDeep(); return true; } })";
const CALL_BASE: &str = "function () { parseDeep(); }";

/// The route of `op` with `handler` over `base`, 1 re-entry per Proxy and `extra` more.
const fn route(handler: &'static str, base: &'static str, op: &'static str, extra: u32) -> Route {
    Route {
        handler,
        base,
        op,
        per_level: 1,
        extra,
    }
}

/// The route of `new p()` with `handler`: 2 re-entries per Proxy.
const fn construct_route(handler: &'static str) -> Route {
    Route {
        handler,
        base: CALL_BASE,
        op: "new p()",
        per_level: 2,
        extra: 0,
    }
}

const BUILTIN_GET: Route = route("{ get: Reflect.get }", GET_BASE, "p.x", 0);
const BUILTIN_SET: Route = route("{ set: Reflect.set }", SET_BASE, "p.x = 1", 0);
const BUILTIN_HAS: Route = route("{ has: Reflect.has }", HAS_BASE, "\"x\" in p", 0);
const BUILTIN_DELETE: Route = route(
    "{ deleteProperty: Reflect.deleteProperty }",
    DELETE_BASE,
    "delete p.x",
    1,
);
const BUILTIN_APPLY: Route = route("{ apply: Reflect.apply }", CALL_BASE, "p()", 0);
const BUILTIN_CONSTRUCT: Route = construct_route("{ construct: Reflect.construct }");
const BOUND_BUILTIN_GET: Route = route("{ get: Reflect.get.bind(null) }", GET_BASE, "p.x", 0);
const TRAPLESS_SET: Route = route("{}", SET_BASE, "p.x = 1", 1);
const TRAPLESS_HAS: Route = route("{}", HAS_BASE, "\"x\" in p", 1);
const TRAPLESS_DELETE: Route = route("{}", DELETE_BASE, "delete p.x", 1);
const TRAPLESS_APPLY: Route = route("{}", CALL_BASE, "p()", 1);
const TRAPLESS_CONSTRUCT: Route = construct_route("{}");
const TRAPLESS_FORWARDING: Route = route(
    "{}",
    "{ x: 1 }",
    "Object.getPrototypeOf(p); Object.isExtensible(p); Object.getOwnPropertyDescriptor(p, \"x\"); \
     Object.defineProperty(p, \"y\", { value: 1, configurable: true }); Object.keys(p); \
     Object.setPrototypeOf(p, Object.prototype); for (var k in p) {}",
    0,
);
// The seven traps of shape (`story:proxy-descriptor-prototype-and-extensibility-traps`). A
// trap that forwards through `Reflect` re-enters through its call; where the clause reads the
// target again after the trap (`isExtensible`, `getOwnPropertyDescriptor`, `ownKeys`), a chain
// of forwarding traps would read the chain below twice at each level, so those chains answer
// in the trap and re-enter through the clause's read of the target.
const BUILTIN_GET_PROTOTYPE_OF: Route = route(
    "{ getPrototypeOf: Reflect.getPrototypeOf }",
    "new Proxy({}, { getPrototypeOf: function () { parseDeep(); return null; } })",
    "Object.getPrototypeOf(p)",
    1,
);
const BUILTIN_SET_PROTOTYPE_OF: Route = route(
    "{ setPrototypeOf: Reflect.setPrototypeOf }",
    "new Proxy({}, { setPrototypeOf: function () { parseDeep(); return false; } })",
    "Reflect.setPrototypeOf(p, null)",
    1,
);
const IS_EXTENSIBLE: Route = route(
    "{ isExtensible: function () { return true; } }",
    "new Proxy({}, { isExtensible: function () { parseDeep(); return true; } })",
    "Object.isExtensible(p)",
    1,
);
const BUILTIN_PREVENT_EXTENSIONS: Route = route(
    "{ preventExtensions: Reflect.preventExtensions }",
    "new Proxy({}, { preventExtensions: function () { parseDeep(); return false; } })",
    "Reflect.preventExtensions(p)",
    1,
);
const GET_OWN_PROPERTY_DESCRIPTOR: Route = route(
    "{ getOwnPropertyDescriptor: function () { return undefined; } }",
    "new Proxy({}, { getOwnPropertyDescriptor: function () { parseDeep(); return undefined; } })",
    "Object.getOwnPropertyDescriptor(p, \"x\")",
    1,
);
const BUILTIN_DEFINE_PROPERTY: Route = route(
    "{ defineProperty: Reflect.defineProperty }",
    "new Proxy({}, { defineProperty: function () { parseDeep(); return false; } })",
    "Reflect.defineProperty(p, \"x\", { value: 1 })",
    1,
);
const OWN_KEYS: Route = route(
    "{ ownKeys: function () { return []; } }",
    "new Proxy({}, { ownKeys: function () { parseDeep(); return []; } })",
    "Reflect.ownKeys(p)",
    1,
);
const PROXY_PROTOTYPE_CHAIN_WALK: Route = route(
    "{}",
    "{ get x() { parseDeep(); return 1; } }",
    "Object.create(p).x",
    1,
);

chain_test!(
    builtin_get_trap_chain_on_the_promised_stack,
    BUILTIN_GET,
    SQUARE_REENTRY
);
chain_test!(
    builtin_set_trap_chain_on_the_promised_stack,
    BUILTIN_SET,
    SQUARE_REENTRY
);
chain_test!(builtin_has_trap_chain_on_the_promised_stack, BUILTIN_HAS);
chain_test!(
    builtin_delete_trap_chain_on_the_promised_stack,
    BUILTIN_DELETE,
    SQUARE_REENTRY
);
chain_test!(
    builtin_apply_trap_chain_on_the_promised_stack,
    BUILTIN_APPLY
);
chain_test!(
    builtin_construct_trap_chain_on_the_promised_stack,
    BUILTIN_CONSTRUCT
);
chain_test!(
    bound_builtin_get_trap_chain_on_the_promised_stack,
    BOUND_BUILTIN_GET,
    SQUARE_REENTRY
);
chain_test!(trapless_set_chain_on_the_promised_stack, TRAPLESS_SET);
chain_test!(trapless_has_chain_on_the_promised_stack, TRAPLESS_HAS);
chain_test!(trapless_delete_chain_on_the_promised_stack, TRAPLESS_DELETE);
chain_test!(trapless_apply_chain_on_the_promised_stack, TRAPLESS_APPLY);
chain_test!(
    trapless_construct_chain_on_the_promised_stack,
    TRAPLESS_CONSTRUCT
);
chain_test!(
    trapless_forwarding_chains_on_the_promised_stack,
    TRAPLESS_FORWARDING
);
chain_test!(
    builtin_get_prototype_of_trap_chain_on_the_promised_stack,
    BUILTIN_GET_PROTOTYPE_OF,
    SQUARE_REENTRY
);
chain_test!(
    builtin_set_prototype_of_trap_chain_on_the_promised_stack,
    BUILTIN_SET_PROTOTYPE_OF
);
chain_test!(
    is_extensible_trap_chain_on_the_promised_stack,
    IS_EXTENSIBLE
);
chain_test!(
    builtin_prevent_extensions_trap_chain_on_the_promised_stack,
    BUILTIN_PREVENT_EXTENSIONS
);
chain_test!(
    get_own_property_descriptor_trap_chain_on_the_promised_stack,
    GET_OWN_PROPERTY_DESCRIPTOR
);
chain_test!(
    builtin_define_property_trap_chain_on_the_promised_stack,
    BUILTIN_DEFINE_PROPERTY
);
chain_test!(
    own_keys_trap_chain_on_the_promised_stack,
    OWN_KEYS,
    SQUARE_REENTRY
);
chain_test!(
    proxy_prototype_chain_walk_on_the_promised_stack,
    PROXY_PROTOTYPE_CHAIN_WALK
);
chain_test!(for_in_chain_on_the_promised_stack, FOR_IN);

/// The route of `for`-`in` over a trapless chain whose base's `getOwnPropertyDescriptor` trap
/// calls `parseDeep`: the walk reaches the base through every Proxy for the key's descriptor,
/// and so does the loop for that key's turn, which reads the chain the walk recorded; the
/// parser runs at the deepest point of both.
const FOR_IN: Route = route(
    "{}",
    "new Proxy({ x: 1 }, { getOwnPropertyDescriptor: function (t, k) { parseDeep(); return Reflect.getOwnPropertyDescriptor(t, k); } })",
    "for (var k in p) {}",
    1,
);

/// The routes of the chain tests.
#[cfg(feature = "collect-every-safe-point")]
const ROUTES: [&Route; 22] = [
    &BUILTIN_GET,
    &BUILTIN_SET,
    &BUILTIN_HAS,
    &BUILTIN_DELETE,
    &BUILTIN_APPLY,
    &BUILTIN_CONSTRUCT,
    &BOUND_BUILTIN_GET,
    &TRAPLESS_SET,
    &TRAPLESS_HAS,
    &TRAPLESS_DELETE,
    &TRAPLESS_APPLY,
    &TRAPLESS_CONSTRUCT,
    &TRAPLESS_FORWARDING,
    &BUILTIN_GET_PROTOTYPE_OF,
    &BUILTIN_SET_PROTOTYPE_OF,
    &IS_EXTENSIBLE,
    &BUILTIN_PREVENT_EXTENSIONS,
    &GET_OWN_PROPERTY_DESCRIPTOR,
    &BUILTIN_DEFINE_PROPERTY,
    &OWN_KEYS,
    &PROXY_PROTOTYPE_CHAIN_WALK,
    &FOR_IN,
];

/// The `reentry_depth` of the tests made by [`combined_chain_test`].
#[cfg(feature = "collect-every-safe-point")]
const COMBINED_REENTRY: u32 = 4;

/// With the feature `collect-every-safe-point`, three tests cover the routes of the default
/// mode's chain tests: each runs a part of [`ROUTES`] (the three parts together are all of it)
/// by [`chain_at_the_bound`] in one agent of `reentry_depth` `COMBINED_REENTRY`, on seven
/// eighths of the `min_stack_bytes()` of that depth. A value held without a root shows at any
/// depth, while the collections at every safe point cost with the length of each chain; the
/// formula of [`allowed_depth`] is checked at a third `reentry_depth` besides. Three parts, not
/// one test: all 22 routes in one took 1.83 to 2.51 CPU seconds alone, near the bound of 2.
macro_rules! combined_chain_test {
    ($name:ident, $routes:expr) => {
        #[cfg(feature = "collect-every-safe-point")]
        #[test]
        fn $name() {
            fn body() {
                let (mut agent, realm) = stack_agent(COMBINED_REENTRY);
                for route in $routes {
                    chain_at_the_bound(&mut agent, &realm, route, COMBINED_REENTRY);
                }
            }
            let stack = stack_with_room(stack_limits(COMBINED_REENTRY));
            on_bounded_stack(test_path!(stringify!($name)), stack, body).unwrap();
        }
    };
}

combined_chain_test!(chains_of_routes_1_to_8_on_the_promised_stack, &ROUTES[..8]);
combined_chain_test!(
    chains_of_routes_9_to_15_on_the_promised_stack,
    &ROUTES[8..15]
);
combined_chain_test!(
    chains_of_routes_16_to_22_on_the_promised_stack,
    &ROUTES[15..]
);

/// The `reentry_depth` of the tests that check that no Proxy count is left behind.
const EXACT_DEPTH: u32 = 20;

/// The check that no Proxy count is left behind: a trapless chain of exactly `EXACT_DEPTH` proxies
/// answers its value and one of `EXACT_DEPTH + 1` throws the RangeError, at the top of an entry.
/// The chain is written out as nested `new Proxy(…, h)` with one handler, and the longer chain
/// is one Proxy over the shorter one: several tests run the check after every attempt, and with
/// a collection at every safe point each step costs a collection. Measured on 2026-11-26: 543
/// steps with a loop that made the chain, a handler per Proxy.
fn full_bound() -> String {
    let depth = usize::try_from(EXACT_DEPTH).unwrap();
    format!(
        "(function () {{ var h = {{}};\n\
           var p = {}{{ x: 1 }}{};\n\
           var a = [];\n\
           try {{ a.push(p.x); }} catch (e) {{ a.push(e.name); }}\n\
           try {{ a.push(new Proxy(p, h).x); }} catch (e) {{ a.push(e.name); }}\n\
           return a.join(); }})();",
        "new Proxy(".repeat(depth),
        ", h)".repeat(depth)
    )
}

fn reaches_the_full_bound(
    agent: &mut swell::Agent<support::host::TestHost>,
    realm: &swell::Realm,
    after: &str,
) {
    assert_eq!(
        eval_text(agent, realm, &full_bound()),
        Outcome::Normal("\"1,RangeError\"".to_string()),
        "after {after}"
    );
}

/// Every abrupt way out of a Proxy method at depth (a throwing trap, a RangeError of the bound,
/// recursion through each trap, the step budget, an interrupt) leaves `proxy_nesting` at 0: the
/// next entry reaches the full bound exactly. The step budget of every entry is 1,200 with
/// `collect-every-safe-point`, where the loop that ends at it collects at each of its steps;
/// every other entry of the test fits in it.
#[test]
fn nesting_is_back_to_zero_after_every_abrupt_way_out() {
    let mut budget = limits();
    budget.reentry_depth = EXACT_DEPTH;
    budget.steps_per_entry = if cfg!(feature = "collect-every-safe-point") {
        1_200
    } else {
        200_000
    };
    let mut agent = agent_with(budget).unwrap();
    let realm = realm(&mut agent);
    define_function(&mut agent, &realm, "interruptNow", 0, interrupt_now, false);
    reaches_the_full_bound(&mut agent, &realm, "nothing");
    let setup = "var base = { x: 1 };\n\
         function trapped(n, h) { var p = base; for (var i = 0; i < n; i++) p = new Proxy(p, h); return p; }\n\
         function deepThen(f) { return trapped(8, { get: function (t, k, r) { if (t === base) return f(); return Reflect.get(t, k, r); } }).x; }";
    assert_eq!(
        eval_text(&mut agent, &realm, setup),
        Outcome::Normal("undefined".to_string())
    );
    let caught: [(&str, &str); 9] = [
        (
            "throwing trap",
            "try { deepThen(function () { throw new TypeError(\"t\"); }); } catch (e) { e.name; }",
        ),
        (
            "bound inside a trap",
            "try { deepThen(function () { return trapped(30, {}).x; }); } catch (e) { e.name; }",
        ),
        (
            "get recursion",
            "var pg = new Proxy({}, { get: function (t, k, r) { return r[k]; } }); try { pg.x; } catch (e) { e.name; }",
        ),
        (
            "apply recursion",
            "var pa = new Proxy(function () {}, { apply: function () { return pa(); } }); try { pa(); } catch (e) { e.name; }",
        ),
        (
            "construct recursion",
            "var pc = new Proxy(function () {}, { construct: function () { return new pc(); } }); try { new pc(); } catch (e) { e.name; }",
        ),
        (
            "has recursion in with",
            "var ph = new Proxy({}, { has: function (t, k) { with (ph) { return x; } } }); try { with (ph) { x; } } catch (e) { e.name; }",
        ),
        (
            "set recursion through the chain",
            "var o; var ps = new Proxy({}, { set: function (t, k, v, r) { o[k] = v; return true; } }); o = Object.create(ps); try { o.x = 1; } catch (e) { e.name; }",
        ),
        (
            "trapless recursion through a prototype cycle",
            "var t = {}; Object.setPrototypeOf(t, new Proxy(t, {})); try { t.missing; } catch (e) { e.name; }",
        ),
        (
            "deleteProperty recursion",
            "var pd = new Proxy({}, { deleteProperty: function (t, k) { return delete pd[k]; } }); try { delete pd.x; } catch (e) { e.name; }",
        ),
    ];
    for (what, source) in caught {
        let outcome = eval_text(&mut agent, &realm, source);
        assert!(
            matches!(&outcome, Outcome::Normal(name) if name == "\"RangeError\"" || name == "\"TypeError\""),
            "{what}: {outcome:?}"
        );
        reaches_the_full_bound(&mut agent, &realm, what);
    }
    assert_eq!(
        eval_text(&mut agent, &realm, "deepThen(function () { for (;;) {} });"),
        Outcome::Terminated(Termination::StepBudget)
    );
    reaches_the_full_bound(&mut agent, &realm, "the step budget in a trap at depth");
    HANDLE.with(|slot| slot.set(Some(agent.interrupt_handle())));
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "deepThen(function () { interruptNow(); return 1; });"
        ),
        Outcome::Terminated(Termination::Interrupted)
    );
    reaches_the_full_bound(&mut agent, &realm, "an interrupt in a trap at depth");
    assert_eq!(agent.state(), AgentState::Alive);
}

/// The memory budget reached inside a trap at depth: the run ends in
/// `Terminated(MemoryBudget)`, one collection brings `live_bytes()` back to the reading before,
/// and the next entry reaches the full bound exactly.
#[test]
fn memory_budget_in_a_trap_at_depth_leaves_no_count_and_no_bytes() {
    let mut budget = limits();
    budget.reentry_depth = EXACT_DEPTH;
    budget.steps_per_entry = 1 << 40;
    let mut probe = agent_with(budget).unwrap();
    let probe_realm = realm(&mut probe);
    reaches_the_full_bound(&mut probe, &probe_realm, "nothing");
    collect(&mut probe);
    budget.memory_bytes = probe.heap_bytes().unwrap() + MEMORY_ROOM;
    let mut agent = agent_with(budget).unwrap();
    let realm = realm(&mut agent);
    let script = |n: &str| {
        format!(
            "(function () {{ var base = {{ x: 1 }};\n\
               var h = {{ get: function (t, k, r) {{ if (t === base) {{ var a = []; for (var i = 0; i < {n}; i++) a.push({{}}); return a.length; }} return Reflect.get(t, k, r); }} }};\n\
               var p = base; for (var i = 0; i < 8; i++) p = new Proxy(p, h); return p.x; }})();"
        )
    };
    assert_eq!(
        eval_text(&mut agent, &realm, &script("4")),
        Outcome::Normal("4".to_string())
    );
    reaches_the_full_bound(&mut agent, &realm, "the warm-up");
    collect(&mut agent);
    let before = agent.live_bytes().unwrap();
    let outcome = eval(&mut agent, &realm, &script("Infinity"));
    assert!(
        matches!(outcome, HostOutcome::Terminated(Termination::MemoryBudget)),
        "{outcome:?}"
    );
    drop(outcome);
    collect(&mut agent);
    let after = agent.live_bytes().unwrap();
    println!("live_bytes {before} before, {after} after");
    assert!(after <= before, "{before} before, {after} after");
    reaches_the_full_bound(&mut agent, &realm, "the memory budget in a trap at depth");
}

/// The step budget ending at every step of `setup` followed by `call`, the Proxy operations,
/// with the limits of `EXACT_DEPTH`: every outcome answers `answer` or is
/// `Terminated(StepBudget)`, and after each the next entry reaches the full bound exactly. The
/// budget is fixed and the script runs after `m` padding statements `0;` (and after each shift
/// of one to three steps, until every offset modulo a padding's steps is reached), from the
/// last `m` with which it answers to the first with which it ends before it starts. With
/// `collect-every-safe-point`, where each step costs a collection, it runs at two `m` per
/// offset: the last that answers and one that ends the budget half way into `call` (the steps
/// of `setup` run no Proxy method). Measured on 2026-11-26 in that mode, opt-level 0, each run
/// alone: every 89th `m` took 3.92 and 2.66 CPU seconds for the two scripts.
fn step_budget_at_every_step(setup: &str, call: &str, answer: &str) {
    let op = format!("{setup}\n{call}");
    let mut measure = limits();
    measure.reentry_depth = EXACT_DEPTH;
    let mut probe = agent_with(measure).unwrap();
    let probe_realm = realm(&mut probe);
    let expected = Outcome::Normal(format!("\"{answer}\""));
    assert_eq!(eval_text(&mut probe, &probe_realm, &op), expected);
    let total = probe.last_entry_steps().unwrap();
    let with_one = format!("0;{op}");
    assert_eq!(eval_text(&mut probe, &probe_realm, &with_one), expected);
    let per = probe.last_entry_steps().unwrap() - total;
    assert!(per > 0, "a `0;` spends no step");
    let outcome = eval(&mut probe, &probe_realm, setup);
    let in_setup = probe.last_entry_steps().unwrap();
    drop(outcome);
    let in_call = total.saturating_sub(in_setup);
    reaches_the_full_bound(&mut probe, &probe_realm, "nothing");
    let check = probe.last_entry_steps().unwrap();
    println!("op {total} steps, {in_call} of them in the call, a `0;` {per}, the check {check}");
    let budget_steps = total.max(check) + 10;
    let mut budget = measure;
    budget.steps_per_entry = budget_steps;
    let mut agent = agent_with(budget).unwrap();
    let realm = realm(&mut agent);
    let (stride, runs) = if cfg!(feature = "collect-every-safe-point") {
        ((in_call / per / 2).max(1), 2)
    } else {
        (1, u64::MAX)
    };
    let mut terminated = 0;
    let mut normal = 0;
    let mut residues = Vec::new();
    for shift in ["", "!0;", "-0;", "void 0;"] {
        assert_eq!(
            eval_text(&mut probe, &probe_realm, &format!("{shift}{op}")),
            expected
        );
        let cost = probe.last_entry_steps().unwrap() - total;
        if residues.contains(&(cost % per)) {
            continue;
        }
        residues.push(cost % per);
        let mut m = budget_steps.saturating_sub(total + cost) / per;
        m = m.saturating_sub(1);
        let mut ran = 0;
        while m * per + cost <= budget_steps && ran < runs {
            let source = format!("{shift}{}{op}", "0;".repeat(usize::try_from(m).unwrap()));
            let outcome = eval_text(&mut agent, &realm, &source);
            match &outcome {
                Outcome::Terminated(Termination::StepBudget) => terminated += 1,
                o if *o == expected => normal += 1,
                other => panic!("with {shift} and {m} padding statements: {other:?}"),
            }
            reaches_the_full_bound(
                &mut agent,
                &realm,
                &format!("{shift} and {m} padding statements"),
            );
            m += stride;
            ran += 1;
        }
    }
    println!("{normal} answered, {terminated} terminated, residues {residues:?} of {per}");
    assert!(terminated > 0 && normal > 0);
    assert_eq!(
        residues.len() as u64,
        per,
        "not every step offset was reached"
    );
}

/// The step budget ending at every step of a script whose Proxy operations run traps and the
/// invariant reads through a forwarding Proxy target ([`step_budget_at_every_step`]).
#[test]
fn step_budget_at_every_step_of_proxy_operations_leaves_no_count() {
    step_budget_at_every_step(
        "var frozen = Object.freeze({ x: 1 });\n\
         var fwd = new Proxy(frozen, {});\n\
         var pg = new Proxy(fwd, { get: function (t, k, r) { return Reflect.get(t, k, r); }, has: function (t, k) { return Reflect.has(t, k); }, set: function () { return true; }, deleteProperty: function () { return false; } });\n\
         var deep = pg; for (var i = 0; i < 4; i++) deep = new Proxy(deep, {});\n\
         function F() {}\n\
         var pf = new Proxy(new Proxy(F, {}), { apply: function (t, th, a) { return Reflect.apply(t, th, a); }, construct: function (t, a, nt) { return Reflect.construct(t, a, nt); } });",
        "[deep.x, \"x\" in deep, Reflect.set(deep, \"x\", 1), delete deep.x, typeof pf(), typeof new pf()].join();",
        "1,true,true,false,undefined,object",
    );
}

/// A throw from a trap (`get`, `apply`, `construct`) and a RangeError of the bound, caught
/// inside a trap that runs under five trapless proxies, leave the count exact: the deepest
/// trapless chain the trap reaches is the same before and after, and at the top it is
/// `reentry_depth`. `deepest()` grows one chain a Proxy at a time. The chain past the bound
/// inside the trap has `PAST_THE_BOUND` proxies: 100, and 30 with `collect-every-safe-point`,
/// still past the 14 the trap reaches.
#[test]
fn a_throw_caught_inside_nested_proxies_leaves_the_count_exact() {
    const PAST_THE_BOUND: u32 = if cfg!(feature = "collect-every-safe-point") {
        30
    } else {
        100
    };
    let mut budget = limits();
    budget.reentry_depth = EXACT_DEPTH;
    let mut agent = agent_with(budget).unwrap();
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            &format!(
                "function c(n) {{ var p = {{ x: 1 }}; for (var i = 0; i < n; i++) p = new Proxy(p, {{}}); return p; }}\n\
             function deepest() {{ var p = {{ x: 1 }}; for (var n = 1; n < 40; n++) {{ p = new Proxy(p, {{}}); try {{ p.x; }} catch (e) {{ return n - 1; }} }} return -1; }}\n\
             var thrower = new Proxy({{}}, {{ get: function () {{ throw new Error(\"t\"); }} }});\n\
             var applyThrower = new Proxy(function () {{}}, {{ apply: function () {{ throw new Error(\"a\"); }} }});\n\
             var constructThrower = new Proxy(function () {{}}, {{ construct: function () {{ throw new Error(\"c\"); }} }});\n\
             var report;\n\
             var trapping = new Proxy({{}}, {{ get: function () {{\n\
               var before = deepest();\n\
               try {{ thrower.x; }} catch (e) {{}}\n\
               try {{ applyThrower(); }} catch (e) {{}}\n\
               try {{ new constructThrower(); }} catch (e) {{}}\n\
               try {{ c({PAST_THE_BOUND}).x; }} catch (e) {{}}\n\
               report = (before === deepest()) + \":\" + before;\n\
               return 0;\n\
             }} }});\n\
             var outer = trapping; for (var i = 0; i < 5; i++) outer = new Proxy(outer, {{}});\n\
             outer.x;\n\
             deepest() + \"|\" + report;"
            )
        ),
        Outcome::Normal("\"20|true:14\"".to_string())
    );
}

/// IsArray and GetFunctionRealm walk a chain of proxies in a loop with a step per Proxy:
/// `Array.isArray` of a chain of 5,000 spends at least 5,000 steps more than of its base, and
/// `Reflect.construct` with a chain of 200 ([`FUNCTION_REALM_CHAIN`]) as `newTarget` whose
/// "prototype" is no object spends, beyond the steps of reading "prototype" through that chain,
/// at least 200 more (GetFunctionRealm). With `collect-every-safe-point` both chains are 40 long:
/// each Proxy made costs a few collections there, and a walk without its step per Proxy still
/// falls 40 steps short. The read of "prototype" alone costs 2 steps per Proxy (measured on
/// 2026-11-26: 23 and 143 steps at 40 proxies, 3 per Proxy with GetFunctionRealm), so the bound
/// of 2 per Proxy this test had before passed with GetFunctionRealm's step taken out.
const IS_ARRAY_CHAIN: u32 = if cfg!(feature = "collect-every-safe-point") {
    40
} else {
    5_000
};

/// The chain that `Reflect.construct` takes as `newTarget` in
/// [`is_array_and_get_function_realm_charge_a_step_per_proxy`].
const FUNCTION_REALM_CHAIN: u32 = if cfg!(feature = "collect-every-safe-point") {
    40
} else {
    200
};

#[test]
fn is_array_and_get_function_realm_charge_a_step_per_proxy() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            &format!(
                "var base = []; var p = base; for (var i = 0; i < {IS_ARRAY_CHAIN}; i++) p = new Proxy(p, {{}});\n\
                 function g() {{}} g.prototype = 1; var pg = g; for (var i = 0; i < {FUNCTION_REALM_CHAIN}; i++) pg = new Proxy(pg, {{}});\n\
                 function h() {{}}\n\
                 0;"
            )
        ),
        Outcome::Normal("0".to_string())
    );
    let mut steps = |source: &str| {
        assert_eq!(
            eval_text(&mut agent, &realm, source),
            Outcome::Normal("true".to_string()),
            "{source}"
        );
        agent.last_entry_steps().unwrap()
    };
    let plain = steps("Array.isArray(base);");
    let chained = steps("Array.isArray(p);");
    let direct = steps("Object.getPrototypeOf(Reflect.construct(h, [], g)) === Object.prototype;");
    let through =
        steps("Object.getPrototypeOf(Reflect.construct(h, [], pg)) === Object.prototype;");
    let read_direct = steps("g.prototype === 1;");
    let read_through = steps("pg.prototype === 1;");
    println!(
        "IsArray {plain} and {chained}; GetFunctionRealm {direct} and {through}; the read \
         {read_direct} and {read_through}"
    );
    assert!(
        chained - plain >= u64::from(IS_ARRAY_CHAIN),
        "{plain} and {chained}"
    );
    assert!(
        through - direct >= read_through - read_direct + u64::from(FUNCTION_REALM_CHAIN),
        "{direct} and {through}, the read {read_direct} and {read_through}"
    );
}

/// The reads of each of the six traps: GetMethod once on the handler (a logging Proxy), the
/// arguments the trap receives (the key as a String or Symbol, the receiver as given, a fresh
/// Array of the arguments, newTarget), the conversion of its answer; a trap `undefined` or `null`
/// forwards with the receiver; a trap that is not callable throws a TypeError, for each value of
/// `NOT_CALLABLE` as each of the six traps (with `collect-every-safe-point` a Number and an
/// object, each run costing a few collections).
#[test]
fn each_trap_reads_the_handler_once_and_gets_the_clause_arguments() {
    const NOT_CALLABLE: (&str, usize) = if cfg!(feature = "collect-every-safe-point") {
        ("[1, {}]", 2)
    } else {
        ("[1, \"s\", {}, true]", 4)
    };
    let (values, count) = NOT_CALLABLE;
    let refused = "T".repeat(6 * count);
    answers(
        &format!(
            "var log = [];\n\
         function show(v) {{ return typeof v === \"symbol\" ? \"sym\" : typeof v === \"string\" ? \"s:\" + v : typeof v === \"number\" ? \"n:\" + v : typeof v; }}\n\
         function logged(traps) {{ return new Proxy(traps, {{ get: function (t, k) {{ log.push(\"h.\" + String(k)); return t[k]; }} }}); }}\n\
         var target = {{ x: 1 }}, other = {{}}, sym = Symbol(\"s\");\n\
         function who(r) {{ return r === p ? \"p\" : r === other ? \"other\" : typeof r; }}\n\
         var p = new Proxy(target, logged({{\n\
           get: function (t, k, r) {{ log.push(\"get(\" + (t === target) + \",\" + show(k) + \",\" + who(r) + \")\"); return 7; }},\n\
           set: function (t, k, v, r) {{ log.push(\"set(\" + (t === target) + \",\" + show(k) + \",\" + show(v) + \",\" + who(r) + \")\"); return \"yes\"; }},\n\
           has: function (t, k) {{ log.push(\"has(\" + (t === target) + \",\" + show(k) + \")\"); return 0; }},\n\
           deleteProperty: function (t, k) {{ log.push(\"del(\" + (t === target) + \",\" + show(k) + \")\"); return {{}}; }}\n\
         }}));\n\
         var r = [p.x, p[1], p[-0], p[1.5], p[4294967295], p[1e21], p[sym], p[true], p[null], Reflect.get(p, \"x\", other)];\n\
         r.push(p.x = 5, Reflect.set(p, 2, 6, other), \"x\" in p, 3 in p, sym in p, delete p.x, delete p[4]);\n\
         var log2 = [];\n\
         var t2 = new Proxy({{ x: 1 }}, {{ get: function (t, k, rc) {{ log2.push(\"tget:\" + String(k) + \":\" + (rc === q)); return Reflect.get(t, k, rc); }} }});\n\
         var q = new Proxy(t2, {{ get: null, set: undefined, has: null, deleteProperty: undefined }});\n\
         r.push(q.x, \"x\" in q, log2.join(\"+\"));\n\
         var bad = [];\n\
         {values}.forEach(function (trap) {{\n\
           [\"get\", \"set\", \"has\", \"deleteProperty\", \"apply\", \"construct\"].forEach(function (name) {{\n\
             var h = {{}}; h[name] = trap;\n\
             var b = new Proxy(function () {{}}, h);\n\
             try {{ if (name === \"get\") b.x; else if (name === \"set\") b.x = 1; else if (name === \"has\") \"x\" in b; else if (name === \"deleteProperty\") delete b.x; else if (name === \"apply\") b(); else new b(); bad.push(\"ok\"); }} catch (e) {{ bad.push(e.name === \"TypeError\" ? \"T\" : e.name); }}\n\
           }});\n\
         }});\n\
         r.push(bad.join(\"\"));\n\
         var args = [];\n\
         function T() {{ return \"called\"; }}\n\
         function Other() {{}}\n\
         var pf = new Proxy(T, logged({{\n\
           apply: function (t, self, a) {{ args.push(a); log.push(\"apply(\" + (t === T) + \",\" + show(self) + \",\" + Array.isArray(a) + \",\" + a.length + \",\" + (Object.getPrototypeOf(a) === Array.prototype) + \")\"); return \"applied\"; }},\n\
           construct: function (t, a, nt) {{ args.push(a); log.push(\"construct(\" + (t === T) + \",\" + a.length + \",\" + (nt === pf ? \"pf\" : nt === Other ? \"Other\" : typeof nt) + \")\"); return {{}}; }}\n\
         }}));\n\
         r.push(pf(1, 2), Function.prototype.call.call(pf, 5, 1), typeof new pf(1), typeof Reflect.construct(pf, [], Other), args[0] !== args[1] && args[2] !== args[3] && args[0] !== args[2]);\n\
         r.join() + \"|\" + log.join(\" \");"
        ),
        &format!(
            "7,7,7,7,7,7,7,7,7,7,5,true,false,false,false,true,true,1,true,tget:x:true,\
         {refused},applied,applied,object,object,true|\
         h.get get(true,s:x,p) h.get get(true,s:1,p) h.get get(true,s:0,p) h.get get(true,s:1.5,p) \
         h.get get(true,s:4294967295,p) h.get get(true,s:1e+21,p) h.get get(true,sym,p) \
         h.get get(true,s:true,p) h.get get(true,s:null,p) h.get get(true,s:x,other) \
         h.set set(true,s:x,n:5,p) h.set set(true,s:2,n:6,other) h.has has(true,s:x) h.has has(true,s:3) \
         h.has has(true,sym) h.deleteProperty del(true,s:x) h.deleteProperty del(true,s:4) \
         h.apply apply(true,undefined,true,2,true) h.apply apply(true,n:5,true,1,true) \
         h.construct construct(true,1,pf) h.construct construct(true,0,Other)"
        ),
    );
}

/// `with` over a Proxy (`#sec-object-environment-records-hasbinding-n`,
/// `#sec-object-environment-records-getbindingvalue-n-s`): HasProperty, Get of
/// @@unscopables, then HasProperty and Get again for the read; sloppy code only.
#[test]
fn with_over_a_proxy_reads_in_the_order_of_the_object_environment() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var log = [];\n\
             var p = new Proxy({ x: 1 }, {\n\
               has: function (t, k) { log.push(\"has:\" + String(k)); return k in t; },\n\
               get: function (t, k, r) { log.push(\"get:\" + String(k)); return t[k]; }\n\
             });\n\
             var v; with (p) { v = x; }\n\
             v + \"|\" + log.join(\" \");"
        ),
        Outcome::Normal("\"1|has:v has:x get:Symbol(Symbol.unscopables) has:x get:x\"".to_string())
    );
}

/// The helpers of the tests of every invariant: `attempt` answers a call's value (`-0` for
/// negative zero) or the name of what it threw, `fixed` an object with the property `k` defined
/// by a descriptor.
const INVARIANT_HELPERS: &str = "var r = [];\n\
     function attempt(f) { try { var v = f(); return typeof v === \"number\" && v === 0 && 1 / v < 0 ? \"-0\" : String(v); } catch (e) { return e.name; } }\n\
     function fixed(desc) { var t = {}; Object.defineProperty(t, \"k\", desc); return t; }\n";

/// Every invariant of the six traps, the case that must throw and its nearest case that must
/// not, SameValue for `+0`/`-0` and NaN; the same with a target that is itself a Proxy, whose
/// [[GetOwnProperty]] the invariant read forwards through. In three tests, each within the CPU
/// bound of the lane's collect build (one test took 1.95 CPU seconds alone): this one the `get`
/// and `set` traps, `each_invariant_both_sides_of_has_and_delete_property` the failed set and
/// delete in strict code, `has` and `deleteProperty`, and
/// `each_invariant_both_sides_of_construct_and_a_proxy_target` `construct` and a target that is
/// a Proxy.
#[test]
fn each_invariant_both_sides() {
    answers(
        &[
            INVARIANT_HELPERS,
            "function getter(desc, answer) { return attempt(function () { return new Proxy(fixed(desc), { get: function () { return answer; } }).k; }); }\n\
         var noGet = { set: function (v) {} };\n\
         r.push(getter({ value: 1 }, 2), getter({ value: 1 }, 1), getter({ value: 0 }, -0), getter({ value: -0 }, 0), getter({ value: -0 }, -0),\n\
           getter({ value: NaN }, NaN), getter({ value: \"ab\" }, \"a\" + \"b\"), getter({ value: 1, writable: true }, 2), getter({ value: 1, configurable: true }, 2),\n\
           getter(noGet, undefined), getter(noGet, null), getter(noGet, 0), getter({ get: function () { return 1; } }, 2),\n\
           getter({ set: function (v) {}, configurable: true }, 1));\n\
         function setter(desc, value, answer) { return attempt(function () { return Reflect.set(new Proxy(fixed(desc), { set: function () { return answer; } }), \"k\", value); }); }\n\
         r.push(\"|\", setter({ value: 1 }, 2, true), setter({ value: 1 }, 1, true), setter({ value: 0 }, -0, true), setter({ value: NaN }, NaN, true),\n\
           setter({ value: 1 }, 2, false), setter({ value: 1, writable: true }, 2, true), setter({ get: function () {} }, 1, true),\n\
           setter({ get: function () {} }, 1, false), setter({ set: function (v) {} }, 1, true), setter({ value: 1 }, 2, \"yes\"), setter({ value: 1 }, 2, 0));\n\
         r.join();",
        ]
        .concat(),
        "TypeError,1,TypeError,TypeError,-0,NaN,ab,2,2,undefined,TypeError,TypeError,2,1,\
         |,TypeError,true,TypeError,true,false,true,TypeError,false,true,TypeError,false",
    );
}

/// The second part of [`each_invariant_both_sides`]: a `set` and a `deleteProperty` trap that
/// answer false, in strict and in sloppy code; the invariants of `has` and `deleteProperty`.
#[test]
fn each_invariant_both_sides_of_has_and_delete_property() {
    answers(
        &[
            INVARIANT_HELPERS,
            "var sp =new Proxy({}, { set: function () { return false; } });\n\
         var dp = new Proxy({ x: 1 }, { deleteProperty: function () { return false; } });\n\
         r.push(attempt(function () { \"use strict\"; sp.x = 1; return \"assigned\"; }), attempt(function () { return Function(\"sp\", \"sp.x = 1; return 'assigned';\")(sp); }),\n\
           attempt(function () { \"use strict\"; return delete dp.x; }), attempt(function () { return Function(\"dp\", \"return delete dp.x;\")(dp); }));\n\
         function hasser(t, answer) { return attempt(function () { return \"k\" in new Proxy(t, { has: function () { return answer; } }); }); }\n\
         r.push(\"|\", hasser(fixed({ value: 1 }), false), hasser(fixed({ value: 1, configurable: true }), false), hasser(Object.preventExtensions({ k: 1 }), false),\n\
           hasser(Object.preventExtensions({}), false), hasser(fixed({ value: 1 }), true), hasser({}, false), hasser(Object.preventExtensions({ k: 1 }), \"\"));\n\
         function deleter(t, answer) { return attempt(function () { return Reflect.deleteProperty(new Proxy(t, { deleteProperty: function () { return answer; } }), \"k\"); }); }\n\
         r.push(\"|\", deleter(fixed({ value: 1 }), true), deleter(fixed({ value: 1 }), false), deleter(Object.preventExtensions({ k: 1 }), true),\n\
           deleter(Object.preventExtensions({}), true), deleter({ k: 1 }, true), deleter(fixed({ value: 1, configurable: true }), 1));\n\
         r.join();",
        ]
        .concat(),
        "TypeError,assigned,TypeError,false,\
         |,TypeError,false,TypeError,false,true,false,TypeError,\
         |,TypeError,false,TypeError,true,true,true",
    );
}

/// The third part of [`each_invariant_both_sides`]: the answer of a `construct` trap, and the
/// invariants of `get`, `has`, `deleteProperty` and `set` over a target that is itself a Proxy.
#[test]
fn each_invariant_both_sides_of_construct_and_a_proxy_target() {
    answers(
        &[
            INVARIANT_HELPERS,
            "function constructed(answer) { return attempt(function () { return typeof new (new Proxy(function () {}, { construct: function () { return answer; } }))(); }); }\n\
         r.push(constructed(1), constructed(null), constructed(undefined), constructed(Symbol()), constructed(\"s\"), constructed(function () {}), constructed([]));\n\
         var frozen = Object.freeze({ k: 1 });\n\
         function inner() { return new Proxy(frozen, { get: function () { return 2; }, has: function () { return false; } }); }\n\
         r.push(\"|\", attempt(function () { return new Proxy(inner(), { get: function () { return 2; } }).k; }),\n\
           attempt(function () { return \"k\" in new Proxy(inner(), { has: function () { return false; } }); }),\n\
           attempt(function () { return Reflect.deleteProperty(new Proxy(inner(), { deleteProperty: function () { return true; } }), \"k\"); }),\n\
           attempt(function () { return Reflect.set(new Proxy(inner(), { set: function () { return true; } }), \"k\", 2); }),\n\
           attempt(function () { return new Proxy(inner(), { get: function () { return 1; } }).k; }));\n\
         r.join();",
        ]
        .concat(),
        "TypeError,TypeError,TypeError,TypeError,TypeError,function,object,\
         |,TypeError,TypeError,TypeError,TypeError,1",
    );
}

/// Revocation in the middle: a trap that revokes its own Proxy, a handler getter that revokes
/// during GetMethod, a revoked Proxy as a target, `revoke` twice, a revoked Proxy as a
/// prototype, `typeof`, IsArray and IsCallable of a revoked Proxy.
#[test]
fn revocation_in_the_middle() {
    answers(
        "var r = [];\n\
         function attempt(f) { try { return String(f()); } catch (e) { return e.name; } }\n\
         var a = Proxy.revocable({ x: 1 }, { get: function (t, k) { a.revoke(); return t[k]; } });\n\
         r.push(attempt(function () { return a.proxy.x; }), attempt(function () { return a.proxy.x; }));\n\
         var b = Proxy.revocable({ x: 1 }, { get get() { b.revoke(); return function (t, k) { return t[k] + 1; }; } });\n\
         r.push(attempt(function () { return b.proxy.x; }), attempt(function () { return b.proxy.x; }));\n\
         var c = Proxy.revocable({}, {}); c.revoke();\n\
         var wrapped;\n\
         r.push(attempt(function () { wrapped = new Proxy(c.proxy, {}); return \"made\"; }), attempt(function () { return wrapped.x; }),\n\
           attempt(function () { return Proxy.revocable(c.proxy, c.proxy) !== null; }), String(c.revoke()), String(c.revoke()));\n\
         var d = Proxy.revocable({}, {}); var child = Object.create(d.proxy); child.own = 1; d.revoke();\n\
         r.push(attempt(function () { return \"z\" in child; }), attempt(function () { return child.z; }), attempt(function () { return child.own; }),\n\
           attempt(function () { child.z = 1; return \"set\"; }), attempt(function () { for (var k in child) {} return \"enumerated\"; }), attempt(function () { return \"own\" in child; }),\n\
           attempt(function () { child.own = 2; return child.own; }), attempt(function () { return Object.keys(child).join(); }));\n\
         var e = Proxy.revocable(function () {}, {}); e.revoke();\n\
         var f = Proxy.revocable([], {}); f.revoke();\n\
         r.push(typeof e.proxy, attempt(function () { return e.proxy(); }), attempt(function () { return new e.proxy(); }), attempt(function () { return Array.isArray(f.proxy); }),\n\
           typeof f.proxy, attempt(function () { return Object.prototype.toString.call(f.proxy); }), attempt(function () { return ({}) instanceof e.proxy; }),\n\
           attempt(function () { return e.proxy instanceof Object; }), attempt(function () { return [].concat(f.proxy).length; }));\n\
         r.join();",
        "1,TypeError,2,TypeError,made,TypeError,true,undefined,undefined,\
         TypeError,TypeError,1,TypeError,TypeError,true,2,own,\
         function,TypeError,TypeError,TypeError,object,TypeError,TypeError,TypeError,TypeError",
    );
}

/// An accessor reached through a trapless Proxy in the prototype chain runs with the receiver
/// (OrdinaryGet, step 2.c, then `#sec-proxy-object-internal-methods-and-internal-slots-get-p-receiver`
/// step 6): a getter and a setter of the Proxy's target reached through `Object.create(proxy)`
/// at depth 1 and 2. (`Object.prototype.__proto__` does not exist on this base.)
#[test]
fn an_accessor_behind_a_trapless_proxy_parent_gets_the_receiver() {
    answers(
        "var tgt = { get me() { return this; }, set me(v) { this.stored = v; } };\n\
         var P = new Proxy(tgt, {});\n\
         var o = Object.create(P), o2 = Object.create(o);\n\
         o2.me = 5;\n\
         [o.me === o, o2.me === o2, Reflect.get(P, \"me\", o) === o, o2.stored, tgt.stored, Object.prototype.hasOwnProperty.call(o2, \"stored\")].join();",
        "true,true,true,5,,true",
    );
}

/// The walks of the chain: a Proxy as the prototype at depth 1, 2 and 100 receives the
/// receiver. The other walks are in `walks_of_the_chain_through_a_proxy_with_other_receivers`
/// (one test in two, each within the CPU bound of the lane's collect build).
#[test]
fn walks_of_the_chain_through_a_proxy() {
    answers(
        "var r = [];\n\
         var seen = [];\n\
         var P = new Proxy({ inherited: 1 }, {\n\
           get: function (t, k, rc) { seen.push(\"get:\" + String(k) + \":\" + (rc === o)); return Reflect.get(t, k, rc); },\n\
           has: function (t, k) { seen.push(\"has:\" + String(k)); return Reflect.has(t, k); },\n\
           set: function (t, k, v, rc) { seen.push(\"set:\" + String(k) + \":\" + (rc === o)); return Reflect.set(t, k, v, rc); }\n\
         });\n\
         var o;\n\
         [1, 2, 100].forEach(function (depth) {\n\
           o = P; for (var i = 0; i < depth; i++) o = Object.create(o);\n\
           r.push(o.inherited, \"inherited\" in o, (o.added = depth, Object.getOwnPropertyDescriptor(o, \"added\").value));\n\
         });\n\
         r.push(seen.join(\" \"));\n\
         r.join();",
        "1,true,1,1,true,2,1,true,100,\
         get:inherited:true has:inherited set:added:true get:inherited:true has:inherited set:added:true \
         get:inherited:true has:inherited set:added:true",
    );
}

/// The other walks of the chain: `set` through a Proxy prototype with an Array and a String
/// wrapper as receiver; `__proto__`, `instanceof`, `for`-`in` and `isPrototypeOf` over a Proxy.
#[test]
fn walks_of_the_chain_through_a_proxy_with_other_receivers() {
    answers(
        "var r = [];\n\
         var T = new Proxy({}, {});\n\
         var arr = [1, 2, 3]; Object.setPrototypeOf(arr, T);\n\
         arr[5] = 6;\n\
         r.push(arr.length, Reflect.set(T, \"length\", 0, arr), arr.length);\n\
         var s = new String(\"ab\"); Object.setPrototypeOf(s, T);\n\
         s[5] = \"q\";\n\
         r.push(s[5], Reflect.set(T, \"0\", \"z\", s), s[0], Reflect.set(T, \"9\", \"n\", s), s[9]);\n\
         var withProto = Object.create(new Proxy(Array.prototype, {}));\n\
         r.push(typeof withProto.push === \"function\", Object.getPrototypeOf(withProto) !== Array.prototype);\n\
         function F() {}\n\
         var instance = new F();\n\
         var PF = new Proxy(F, {});\n\
         r.push(instance instanceof PF, Object.create(new Proxy(F.prototype, {})) instanceof F, new Proxy(instance, {}) instanceof F,\n\
           F.prototype.isPrototypeOf(Object.create(new Proxy(instance, {}))));\n\
         var keys = []; var enumerated = Object.create(new Proxy({ a: 1, b: 2 }, {})); enumerated.c = 3;\n\
         for (var k in enumerated) keys.push(k);\n\
         r.push(keys.join(\"\"));\n\
         r.join();",
        "6,true,0,q,false,a,true,n,true,true,true,false,true,true,cab",
    );
}

/// A prototype cycle made through a Proxy (`Object.setPrototypeOf(t, new Proxy(t, {}))`, which
/// the ordinary check does not see through): a read, `in` and a write end in the RangeError of
/// `reentry_depth`; `instanceof`, `for`-`in` and `isPrototypeOf` loop without recursion and end
/// at the step budget.
#[test]
fn a_prototype_cycle_through_a_proxy() {
    let mut budget = limits();
    budget.steps_per_entry = if cfg!(feature = "collect-every-safe-point") {
        3_000
    } else {
        100_000
    };
    budget.reentry_depth = EXACT_DEPTH;
    let mut agent = agent_with(budget).unwrap();
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var t = {}; var P = new Proxy(t, {}); Object.setPrototypeOf(t, P);\n\
             var r = [];\n\
             function attempt(f) { try { return String(f()); } catch (e) { return e.name; } }\n\
             r.push(Object.getPrototypeOf(t) === P, attempt(function () { return t.missing; }), attempt(function () { return \"m\" in t; }),\n\
               attempt(function () { t.m = 1; return \"set\"; }), attempt(function () { return t.__proto__ === P; }));\n\
             r.join();"
        ),
        Outcome::Normal("\"true,RangeError,RangeError,RangeError,RangeError\"".to_string())
    );
    for source in [
        "t instanceof function F() {};",
        "for (var k in t) {}",
        "Object.prototype.isPrototypeOf.call({}, t);",
    ] {
        let outcome = eval_text(&mut agent, &realm, source);
        println!("{source}: {outcome:?}");
        assert!(
            outcome == Outcome::Terminated(Termination::StepBudget)
                || (source.starts_with("for")
                    && outcome == Outcome::Normal("undefined".to_string())),
            "{source}: {outcome:?}"
        );
        reaches_the_full_bound(&mut agent, &realm, source);
    }
}

/// Call and construct through a Proxy of each kind of function: an arrow and a generator
/// function are not constructors (`new` throws before any read of the handler), an async
/// function, builtins, a bound function; `this` of every type to an `apply` trap and to a
/// strict and a sloppy target; `new.target` the Proxy and another constructor; the prototype
/// of the made object read through the Proxy's `get` trap; a Proxy as a callback, a getter
/// and a species constructor; `name`, `length` and `bind` through a Proxy.
#[test]
fn call_and_construct_through_each_kind_of_function() {
    answers(
        "var r = [];\n\
         function attempt(f) { try { var v = f(); return typeof v === \"object\" && v !== null ? \"object\" : String(v); } catch (e) { return e.name; } }\n\
         var log = [];\n\
         var handlerLog = new Proxy({}, { get: function (t, k) { log.push(String(k)); return undefined; } });\n\
         var arrow = new Proxy(() => 1, handlerLog);\n\
         r.push(attempt(function () { return new arrow(); }), log.length, attempt(function () { return arrow(); }), log.join(\"+\"));\n\
         var gen = new Proxy(function* () { yield 1; }, {});\n\
         r.push(attempt(function () { return new gen(); }), attempt(function () { return gen().next().value; }));\n\
         var asy = new Proxy(async function () { return 1; }, {});\n\
         r.push(attempt(function () { return asy() instanceof Promise; }), attempt(function () { return new asy(); }));\n\
         r.push(attempt(function () { return new Proxy(Math.max, {})(1, 3); }), attempt(function () { return new (new Proxy(Math.max, {}))(); }));\n\
         r.push(attempt(function () { return new Proxy(Array, {})(3).length; }), attempt(function () { return new (new Proxy(Array, {}))(2).length; }),\n\
           attempt(function () { return new (new Proxy(Date, {}))(0) instanceof Date; }));\n\
         function add(a, b) { return a + b; }\n\
         r.push(attempt(function () { return new Proxy(add.bind(null, 1), {})(2); }), attempt(function () { return new (new Proxy(add.bind(null, 1), {}))(2) instanceof add; }));\n\
         var seen = [];\n\
         var strictTarget = new Proxy(function () { \"use strict\"; return this; }, { apply: function (t, self, a) { seen.push(self === null ? \"null\" : typeof self); return Reflect.apply(t, self, a); } });\n\
         var sloppyTarget = new Proxy(Function(\"return this;\"), {});\n\
         [undefined, null, 1, \"s\", true, Symbol(), {}].forEach(function (v) { strictTarget.call(v); });\n\
         r.push(seen.join(\"/\"), typeof strictTarget.call(1), typeof sloppyTarget.call(1), sloppyTarget.call(undefined) === globalThis);\n\
         function F() {}\n\
         var PF = new Proxy(F, {});\n\
         function Other() {}\n\
         r.push(Object.getPrototypeOf(new PF()) === F.prototype, Object.getPrototypeOf(Reflect.construct(PF, [], Other)) === Other.prototype);\n\
         var getLog = [];\n\
         function G() {}\n\
         G.prototype = { tag: \"G\" };\n\
         var PG = new Proxy(G, { get: function (t, k, rc) { getLog.push(String(k)); return Reflect.get(t, k, rc); } });\n\
         r.push(new PG().tag, getLog.join(\"+\"));\n\
         getLog.length = 0;\n\
         r.push(Reflect.construct(G, [], PG).tag, getLog.join(\"+\"));\n\
         r.push([1, 2].map(new Proxy(function (x) { return x * 2; }, {})).join(\"-\"));\n\
         var withGetter = Object.defineProperty({}, \"g\", { get: new Proxy(function () { return \"got\"; }, {}) });\n\
         r.push(withGetter.g);\n\
         var sp = [1, 2]; sp.constructor = {}; sp.constructor[Symbol.species] = new Proxy(Array, {});\n\
         r.push(Array.isArray(sp.map(function (x) { return x; })));\n\
         var named = new Proxy(function abc(a, b) {}, { get: function (t, k, rc) { return k === \"name\" ? \"trapped\" : Reflect.get(t, k, rc); } });\n\
         r.push(named.name, named.length, Function.prototype.call.call(new Proxy(function (a) { \"use strict\"; return this === undefined ? \"u:\" + a : typeof this; }, {}), undefined, 5));\n\
         var bp = Function.prototype.bind.call(new Proxy(function abc(a, b) {}, {}), null);\n\
         r.push(bp.name, bp.length, typeof Function.prototype.bind.call(new Proxy(() => 1, {}), null));\n\
         r.join();",
        "TypeError,0,1,apply,TypeError,1,true,TypeError,3,TypeError,3,2,true,3,false,\
         undefined/null/number/string/boolean/symbol/object,number,object,true,true,true,G,prototype,G,prototype,\
         2-4,got,true,trapped,2,u:5,bound abc,2,function",
    );
}

/// Builtins given a Proxy of the kind they expect: the answer the standard gives, through the
/// traps or a TypeError for a missing internal slot.
#[test]
fn builtins_given_a_proxy_of_each_kind() {
    answers(
        "var r = [];\n\
         function attempt(f) { try { return String(f()); } catch (e) { return e.name; } }\n\
         var ots = Object.prototype.toString;\n\
         r.push(ots.call(new Proxy([], {})), ots.call(new Proxy(function () {}, {})), ots.call(new Proxy(new Error(\"m\"), {})), ots.call(new Proxy(new Date(0), {})),\n\
           ots.call(new Proxy(/a/, {})), ots.call(new Proxy(new Boolean(true), {})), ots.call(new Proxy(new String(\"s\"), {})), ots.call(new Proxy(new Number(1), {})),\n\
           ots.call((function () { return new Proxy(arguments, {}); })()), ots.call(new Proxy(Promise.resolve(1), {})));\n\
         r.push(\"|\", attempt(function () { return Date.prototype.getTime.call(new Proxy(new Date(0), {})); }),\n\
           attempt(function () { return RegExp.prototype.exec.call(new Proxy(/a/, {}), \"a\"); }),\n\
           attempt(function () { return \"abc\".replace(new Proxy(/b/, {}), \"x\"); }),\n\
           attempt(function () { return String(new Proxy(new String(\"ab\"), {})); }),\n\
           attempt(function () { return Boolean.prototype.valueOf.call(new Proxy(new Boolean(true), {})); }),\n\
           attempt(function () { return Number.prototype.toString.call(new Proxy(new Number(1), {})); }),\n\
           attempt(function () { return Symbol.prototype.toString.call(new Proxy(Object(Symbol(\"q\")), {})); }),\n\
           attempt(function () { return Error.prototype.toString.call(new Proxy(new Error(\"m\"), {})); }),\n\
           attempt(function () { return Error.isError(new Proxy(new Error(\"m\"), {})); }),\n\
           attempt(function () { var g = (function* () { yield 1; })(); return g.next.call(new Proxy(g, {})); }),\n\
           attempt(function () { return Promise.prototype.then.call(new Proxy(Promise.resolve(1), {}), function () {}); }),\n\
           attempt(function () { return Function.prototype.toString.call(new Proxy(function f() {}, {})).indexOf(\"native code\") > 0; }),\n\
           attempt(function () { return RegExp.prototype.toString.call(new Proxy(/a/g, {})); }),\n\
           attempt(function () { return RegExp.prototype.test.call(new Proxy(/a/, {}), \"a\"); }),\n\
           attempt(function () { return new Proxy(/a/, {}).source; }),\n\
           attempt(function () { return Array.prototype.concat.call(new Proxy([1], {}), [2]).length; }),\n\
           attempt(function () { return new Proxy(new String(\"ab\"), {}).length + new Proxy(new String(\"ab\"), {})[1]; }),\n\
           attempt(function () { return JSON.stringify(new Proxy(new Number(3), {})); }),\n\
           attempt(function () { return JSON.stringify(new Proxy(new String(\"s\"), {})); }),\n\
           attempt(function () { return JSON.stringify(new Proxy(new Boolean(false), {})); }),\n\
           attempt(function () { return JSON.stringify({ d: new Proxy(new Date(0), {}) }); }),\n\
           attempt(function () { return Array.prototype.join.call(new Proxy((function () { return arguments; })(1, 2), {}), \"-\"); }));\n\
         function logging(t, log) { return new Proxy(t, { get: function (o, k, rc) { log.push(String(k)); return Reflect.get(o, k, rc); }, has: function (o, k) { log.push(\"has:\" + String(k)); return Reflect.has(o, k); } }); }\n\
         var l1 = []; r.push(\"|\", JSON.stringify(logging({ a: 1, b: [2] }, l1)), l1.join(\"+\"));\n\
         var l2 = []; r.push(JSON.stringify(logging([1, 2], l2)), l2.join(\"+\"));\n\
         var l3 = [];\n\
         var revived = JSON.parse('{\"a\":1,\"c\":2}', function (k, v) { if (k === \"a\") this.c = logging([5], l3); return v; });\n\
         r.push(Array.isArray(revived.c), l3.join(\"+\"));\n\
         var l4 = []; r.push([0].concat(logging([1, 2], l4)).join(\"-\"), l4.join(\"+\"));\n\
         var l5 = []; r.push(Array.from(logging([1, 2], l5)).join(\"-\"), l5.join(\"+\"));\n\
         var l6 = []; r.push([...logging([1, 2], l6)].join(\"-\"), l6.join(\"+\"));\n\
         var l7 = []; var [d1, d2] = logging([1, 2, 3], l7); r.push(d1 + d2, l7.join(\"+\"));\n\
         var l8 = []; var { a: ra, ...rest } = logging({ a: 1, b: 2 }, l8); r.push(ra + rest.b, l8.join(\"+\"));\n\
         var l9 = []; r.push(JSON.stringify(Object.assign({}, logging({ a: 1, b: 2 }, l9))), Object.keys(logging({ a: 1, b: 2 }, l9)).join(), JSON.stringify(Object.entries(logging({ a: 1 }, l9))), l9.join(\"+\"));\n\
         var l10 = []; r.push((function (a, b) { return a + b; }).apply(null, logging({ length: 2, 0: \"x\", 1: \"y\" }, l10)), l10.join(\"+\"));\n\
         var sorted = new Proxy([3, 1, 2], {}); Array.prototype.sort.call(sorted);\n\
         r.push(Array.prototype.join.call(sorted, \"-\"), Array.prototype.splice.call(new Proxy([1, 2, 3], {}), 1, 1).join(\"-\"),\n\
           [[1, [2]], new Proxy([3, [4]], {})].flat(Infinity).join(\"-\"), Array.prototype.join.call(Array.prototype.reverse.call(new Proxy([1, 2], {})), \"-\"));\n\
         var l11 = []; var pushed = new Proxy([], { set: function (t, k, v, rc) { l11.push(String(k) + \"=\" + String(v)); return Reflect.set(t, k, v, rc); } });\n\
         Array.prototype.push.call(pushed, \"a\"); r.push(l11.join(\"+\"), pushed.length);\n\
         r.join();",
        "[object Array],[object Function],[object Object],[object Object],[object Object],[object Object],[object Object],[object Object],[object Object],[object Promise],\
         |,TypeError,TypeError,TypeError,TypeError,TypeError,TypeError,TypeError,Error: m,false,TypeError,TypeError,true,TypeError,TypeError,TypeError,2,2b,\
         {},{\"0\":\"s\"},{},TypeError,1-2,\
         |,{\"a\":1,\"b\":[2]},toJSON+a+b,[1,2],toJSON+length+0+1,true,length+0,\
         0-1-2,Symbol(Symbol.isConcatSpreadable)+length+has:0+0+has:1+1,\
         1-2,Symbol(Symbol.iterator)+length+0+length+1+length,1-2,Symbol(Symbol.iterator)+length+0+length+1+length,\
         3,Symbol(Symbol.iterator)+length+0+length+1,3,a+b,{\"a\":1,\"b\":2},a,b,[[\"a\",1]],a+b+a,\
         xy,length+0+1,1-2-3,2,1-2-3-4,2-1,0=a+length=1,1",
    );
}

/// Promise jobs with a Proxy: `Promise.resolve` of a Proxy of a promise (no [[PromiseState]]: a
/// thenable whose `then` throws a TypeError in its job), `await` of one, a revoke function as a
/// `then` handler, as an executor and as a thenable's `then`, a callable Proxy as a `then`.
#[test]
fn promise_jobs_with_proxies_and_revoke_functions() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var out = [];\n\
             var pp = new Proxy(Promise.resolve(1), {});\n\
             var res = Promise.resolve(pp);\n\
             out.push(String(res !== pp));\n\
             res.then(function (v) { out.push(\"fulfilled:\" + v); }, function (e) { out.push(\"rejected:\" + e.name); });\n\
             var rv = Proxy.revocable({}, {});\n\
             Promise.resolve(1).then(rv.revoke).then(function (v) { out.push(\"revoke-handler:\" + v); });\n\
             try { new Promise(rv.revoke); out.push(\"executor-ok\"); } catch (e) { out.push(\"executor:\" + e.name); }\n\
             Promise.resolve({ then: rv.revoke }).then(function (v) { out.push(\"thenable:\" + v); }, function (e) { out.push(\"thenable-rejected\"); });\n\
             (async function () { try { await new Proxy(Promise.resolve(2), {}); out.push(\"awaited\"); } catch (e) { out.push(\"await:\" + e.name); } })();\n\
             (async function () { out.push(\"proxy-then:\" + await { then: new Proxy(function (resolve) { resolve(7); }, {}) }); })();\n\
             Promise.reject(3).catch(new Proxy(function (e) { out.push(\"caught:\" + e); }, {}));\n\
             out.length;"
        ),
        Outcome::Normal("2".to_string())
    );
    assert!(matches!(
        agent.perform_microtask_checkpoint().unwrap(),
        HostOutcome::Normal(_)
    ));
    assert_eq!(
        eval_text(&mut agent, &realm, "out.sort().join();"),
        Outcome::Normal(
            "\"await:TypeError,caught:3,executor-ok,proxy-then:7,rejected:TypeError,revoke-handler:undefined,true\""
                .to_string()
        )
    );
}

/// The collector: a target reachable only through the Proxy, whose trap revokes the Proxy and
/// allocates before the invariant read of the target, for `get`, `has`, `deleteProperty`,
/// `set` and `apply`; a Proxy reachable only through its own handler's closure.
#[test]
fn collector_keeps_what_a_running_proxy_method_holds() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let churn = if cfg!(feature = "collect-every-safe-point") {
        8
    } else {
        300
    };
    assert_eq!(
        eval_text(&mut agent, &realm, &format!("var CHURN = {churn}; 0;")),
        Outcome::Normal("0".to_string())
    );
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var r = [];\n\
             function attempt(f) { try { return String(f()); } catch (e) { return e.name; } }\n\
             function churn() { var junk = []; for (var i = 0; i < CHURN; i++) junk.push({ i: i, s: \"x\" + i }); return junk.length; }\n\
             function plain(name, answer) { var h = {}; var rv; h[name] = function (t, k) { rv.revoke(); churn(); return answer; }; rv = Proxy.revocable(Object.freeze({ k: 1 }), h); return rv.proxy; }\n\
             r.push(attempt(function () { return plain(\"get\", 1).k; }), attempt(function () { return plain(\"get\", 2).k; }),\n\
               attempt(function () { return \"k\" in plain(\"has\", false); }), attempt(function () { return Reflect.deleteProperty(plain(\"deleteProperty\", true), \"k\"); }),\n\
               attempt(function () { return Reflect.set(plain(\"set\", true), \"k\", 2); }), attempt(function () { return Reflect.set(plain(\"set\", true), \"k\", 1); }));\n\
             var applied = Proxy.revocable(function (a, b) { return a + b; }, { apply: function (t, self, args) { applied.revoke(); churn(); return t.apply(self, args) + args.length; } });\n\
             r.push(attempt(function () { return applied.proxy(1, 2); }));\n\
             function make() { var h = {}; var p = new Proxy({ v: 1 }, h); h.get = function (t, k) { return k === \"self\" ? p : t[k]; }; return p; }\n\
             var held = make(); churn();\n\
             r.join();"
        ),
        Outcome::Normal("\"1,TypeError,TypeError,TypeError,TypeError,true,5\"".to_string())
    );
    collect(&mut agent);
    assert_eq!(
        eval_text(&mut agent, &realm, "churn(); held.self.self.v + held.v;"),
        Outcome::Normal("2".to_string())
    );
}

/// A revoke function outlives every other reference to its Proxy and its result object: after a
/// collection it still revokes (the Proxy it holds was kept), answers `undefined` twice, and a
/// Proxy kept apart from its result object throws once revoked.
#[test]
fn a_revoke_function_outlives_its_result_object() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var lone = Proxy.revocable({}, {}).revoke;\n\
             var kept = Proxy.revocable([1], {}); var rv2 = kept.revoke; var p2 = kept.proxy; kept = null;\n\
             0;"
        ),
        Outcome::Normal("0".to_string())
    );
    collect(&mut agent);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var r = [String(lone()), String(lone()), p2.length, String(rv2())];\n\
             try { p2.length; r.push(\"read\"); } catch (e) { r.push(e.name); }\n\
             r.join();"
        ),
        Outcome::Normal("\"undefined,undefined,1,undefined,TypeError\"".to_string())
    );
    collect(&mut agent);
    assert_eq!(
        eval_text(&mut agent, &realm, "String(rv2()) + typeof p2;"),
        Outcome::Normal("\"undefinedobject\"".to_string())
    );
}

/// More builtins given a Proxy: own-key reads, `Object.freeze` through the forwarding methods, a
/// Proxy as a replacer, a replace function, a comparator, a `split` separator and a RegExp
/// pattern (no [[RegExpMatcher]]: a TypeError), Array methods on a Proxy of an Array, `for`-`of`,
/// `yield*`, `Object.fromEntries`, a tag, compound assignment, an optional call,
/// `Promise.prototype.finally` on a Proxy of a promise, `Object.defineProperties`.
#[test]
fn more_builtins_given_a_proxy() {
    answers(
        "var r = [];\n\
         function attempt(f) { try { return String(f()); } catch (e) { return e.name; } }\n\
         var target = { a: 1 }; Object.defineProperty(target, \"hidden\", { value: 2, enumerable: false, configurable: true });\n\
         var p = new Proxy(target, {});\n\
         r.push(Object.getOwnPropertyNames(p).join(\"+\"), Reflect.ownKeys(p).join(\"+\"), Object.prototype.hasOwnProperty.call(p, \"a\"),\n\
           Object.prototype.hasOwnProperty.call(p, \"hidden\"), Object.prototype.propertyIsEnumerable.call(p, \"hidden\"));\n\
         Object.freeze(p);\n\
         r.push(Object.isFrozen(p), Object.isFrozen(target), Object.isExtensible(p));\n\
         r.push(attempt(function () { return JSON.stringify({ a: 1, b: 2 }, new Proxy([\"a\"], {})); }),\n\
           attempt(function () { return JSON.stringify({ a: 1 }, new Proxy(function (k, v) { return typeof v === \"number\" ? v + 1 : v; }, {})); }),\n\
           attempt(function () { return \"abc\".replace(\"b\", new Proxy(function () { return \"X\"; }, {})); }),\n\
           attempt(function () { return [3, 1, 2].sort(new Proxy(function (x, y) { return x - y; }, {})).join(\"\"); }),\n\
           attempt(function () { return \"a,b\".split(new Proxy(/,/, {})).length; }),\n\
           attempt(function () { return new RegExp(new Proxy(/a/, {})).source; }),\n\
           attempt(function () { return Array.prototype.includes.call(new Proxy([1, NaN], {}), NaN); }),\n\
           attempt(function () { return Array.prototype.indexOf.call(new Proxy([1, 2], {}), 2); }),\n\
           attempt(function () { return Array.prototype.slice.call(new Proxy([1, 2, 3], {}), 1).join(\"\"); }),\n\
           attempt(function () { var s = 0; for (var v of new Proxy([1, 2, 3], {})) s += v; return s; }),\n\
           attempt(function () { function* g() { yield* new Proxy([1, 2], {}); } return [...g()].join(\"\"); }),\n\
           attempt(function () { return Object.fromEntries(new Proxy([[\"k\", 1]], {})).k; }),\n\
           attempt(function () { var t = new Proxy(function (s) { return s.raw[0]; }, {}); return t`x`; }),\n\
           attempt(function () { var q = new Proxy({ x: 1 }, {}); q.x += 2; q.x++; return q.x; }),\n\
           attempt(function () { var f = new Proxy(function () { return 9; }, {}); return f?.(); }),\n\
           attempt(function () { return Promise.prototype.finally.call(new Proxy(Promise.resolve(1), {}), function () {}); }),\n\
           attempt(function () { return Object.defineProperties(new Proxy({}, {}), { z: { value: 3, enumerable: true } }).z; }),\n\
           attempt(function () { return Object.keys(Object.getOwnPropertyDescriptors(new Proxy({ m: 1 }, {}))).length; }));\n\
         r.join();",
        "a+hidden,a+hidden,true,true,false,true,true,false,{\"a\":1},{\"a\":2},aXc,123,TypeError,TypeError,true,1,23,6,12,1,x,4,9,TypeError,3,1",
    );
}

/// A `super` property read in a method whose home object's prototype is a Proxy reaches its
/// `get` trap with the method's `this` as the receiver (`#sec-super-keyword-runtime-semantics-evaluation`,
/// MakeSuperPropertyReference). The adversary's case of `story:proxy-objects-and-access-traps`,
/// added once `super` and Proxy are both on the branch.
#[test]
fn super_property_through_a_proxy_prototype() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let outcome = eval_text(
        &mut agent,
        &realm,
        "var log = [], o;\n\
         var P = new Proxy({ x: \"px\" }, { get: function (t, k, r) { log.push(String(k) + \":\" + (r === o)); return Reflect.get(t, k, r); } });\n\
         o = { m() { return super.x; } };\n\
         Object.setPrototypeOf(o, P);\n\
         o.m() + \"|\" + log.join();",
    );
    assert_eq!(outcome, Outcome::Normal("\"px|x:true\"".to_string()));
}

/// The seven traps of shape (`story:proxy-descriptor-prototype-and-extensibility-traps`): a
/// throw from each trap, and a broken invariant of each, caught at the top and inside a `get`
/// trap six proxies deep, each under two trapless proxies, leave the count exact: inside the trap a
/// trapless chain of 14 answers and one of 15 throws the RangeError before and after them, and at
/// the top one of `reentry_depth` answers and one more throws.
#[test]
fn a_throw_of_each_shape_trap_caught_inside_nested_proxies_leaves_the_count_exact() {
    let mut budget = limits();
    budget.reentry_depth = EXACT_DEPTH;
    let mut agent = agent_with(budget).unwrap();
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "function c(n) { var p = { x: 1 }; for (var i = 0; i < n; i++) p = new Proxy(p, {}); return p; }\n\
             function bound(k) { var a = []; try { a.push(c(k).x); } catch (e) { a.push(e.name); } try { a.push(c(k + 1).x); } catch (e) { a.push(e.name); } return a.join(\"+\"); }\n\
             function nest(t, h) { return new Proxy(new Proxy(new Proxy(t, h), {}), {}); }\n\
             function thrower() { throw new Error(\"t\"); }\n\
             var fixed = Object.preventExtensions({ a: 1 });\n\
             var cases = [\n\
               function () { Object.getPrototypeOf(nest({}, { getPrototypeOf: thrower })); },\n\
               function () { Object.getPrototypeOf(nest({}, { getPrototypeOf: function () { return 1; } })); },\n\
               function () { Object.setPrototypeOf(nest({}, { setPrototypeOf: thrower }), null); },\n\
               function () { Object.setPrototypeOf(nest(fixed, { setPrototypeOf: function () { return true; } }), null); },\n\
               function () { Object.isExtensible(nest({}, { isExtensible: thrower })); },\n\
               function () { Object.isExtensible(nest({}, { isExtensible: function () { return false; } })); },\n\
               function () { Object.preventExtensions(nest({}, { preventExtensions: thrower })); },\n\
               function () { Object.preventExtensions(nest({}, { preventExtensions: function () { return true; } })); },\n\
               function () { Object.getOwnPropertyDescriptor(nest({}, { getOwnPropertyDescriptor: thrower }), \"x\"); },\n\
               function () { Object.getOwnPropertyDescriptor(nest(fixed, { getOwnPropertyDescriptor: function () { return undefined; } }), \"a\"); },\n\
               function () { Object.defineProperty(nest({}, { defineProperty: thrower }), \"x\", { value: 1 }); },\n\
               function () { Object.defineProperty(nest(fixed, { defineProperty: function () { return true; } }), \"x\", { value: 1 }); },\n\
               function () { Object.keys(nest({}, { ownKeys: thrower })); },\n\
               function () { Object.keys(nest(fixed, { ownKeys: function () { return [\"a\", \"b\"]; } })); }\n\
             ];\n\
             function all() { var names = []; for (var i = 0; i < cases.length; i++) { try { cases[i](); names.push(\"none\"); } catch (e) { names.push(e.name); } } return names.join(\"\"); }\n\
             var before = bound(20), top = all(), report;\n\
             var trapping = new Proxy({}, { get: function () {\n\
               var inner = bound(14), names = all();\n\
               report = [inner, bound(14), names].join(\"/\");\n\
               return 0;\n\
             } });\n\
             var outer = trapping; for (var i = 0; i < 5; i++) outer = new Proxy(outer, {});\n\
             outer.x;\n\
             [before, bound(20), report, top].join(\"/\");"
        ),
        Outcome::Normal(format!(
            "\"1+RangeError/1+RangeError/1+RangeError/1+RangeError/{0}/{0}\"",
            ["Error", "TypeError"].repeat(7).concat()
        ))
    );
}

/// The steps of one `ownKeys` call through the checks of its clause (10.5.11) per key of its
/// answer, read from `last_entry_steps()` as the difference between an answer of 20 keys and
/// one of 10 over targets that are not extensible: CreateListFromArrayLike's step per element,
/// the check of repeated keys (I18), the target's [[OwnPropertyKeys]] (a step per key of an
/// ordinary object), the loop over the target's keys that reads each descriptor, the check I19
/// (every key non-configurable, a frozen target) or I20 (every key configurable), and
/// `Reflect.ownKeys`'s CreateArrayFromList: 6 steps per key. Each charge of the clause is held:
/// without one of them the difference is 50, not 60.
#[test]
fn own_keys_spends_a_step_per_key_at_each_check() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "function made(n, freeze) { var t = {}, keys = []; for (var i = 0; i < n; i++) { t[\"k\" + i] = i; keys.push(\"k\" + i); }\n\
               if (freeze) Object.freeze(t); else Object.preventExtensions(t);\n\
               return new Proxy(t, { ownKeys: function () { return keys; } }); }\n\
             var open10 = made(10, false), open20 = made(20, false), frozen10 = made(10, true), frozen20 = made(20, true);\n\
             0;"
        ),
        Outcome::Normal("0".to_string())
    );
    let mut steps = |source: &str| {
        assert_eq!(
            eval_text(&mut agent, &realm, source),
            Outcome::Normal("true".to_string()),
            "{source}"
        );
        agent.last_entry_steps().unwrap()
    };
    let open = [
        steps("Reflect.ownKeys(open10).length === 10;"),
        steps("Reflect.ownKeys(open20).length === 20;"),
    ];
    let frozen = [
        steps("Reflect.ownKeys(frozen10).length === 10;"),
        steps("Reflect.ownKeys(frozen20).length === 20;"),
    ];
    let configurable = open[1] - open[0];
    let fixed = frozen[1] - frozen[0];
    println!(
        "open {open:?}, frozen {frozen:?}: 10 keys more, {configurable} steps more (I20), {fixed} (I19)"
    );
    assert_eq!(configurable, 60);
    assert_eq!(fixed, 60);
}

/// The step budget ending at every step of one `ownKeys` call with its invariant checks,
/// through a forwarding Proxy over a target that is not extensible with a non-configurable key
/// (I17, I18, IsExtensible, the target's keys, a descriptor per key, I19, I20, I21)
/// ([`step_budget_at_every_step`]).
#[test]
fn step_budget_at_every_step_of_an_own_keys_call_leaves_no_count() {
    step_budget_at_every_step(
        "var t = { a: 1, b: 2 }; Object.defineProperty(t, \"n\", { value: 3 }); Object.preventExtensions(t);\n\
         var fwd = new Proxy(t, {});\n\
         var p = new Proxy(fwd, { ownKeys: function (target) { return [\"n\", \"a\", \"b\"]; } });\n\
         var deep = p; for (var i = 0; i < 3; i++) deep = new Proxy(deep, {});",
        "Reflect.ownKeys(deep).join();",
        "n,a,b",
    );
}
