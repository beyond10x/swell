//! The Reflect object (`#sec-reflect-object`), `story:reflect-namespace`: the global `Reflect`,
//! an ordinary object with its 13 functions (`#sec-reflect.apply` to `#sec-reflect.setprototypeof`)
//! and `Reflect[%Symbol.toStringTag%]` (`#sec-reflect-%symbol.tostringtag%`). Each function is a
//! call of one internal method of its target, with the checks of its clause; `apply` and
//! `construct` copy their argument list by CreateListFromArrayLike
//! (`#sec-createlistfromarraylike`), a step per element, each charged to the memory budget, and
//! re-enter the engine once per call (`docs/design/engine.md`, decision 3, "Depth bounds"). Run
//! also with the feature `collect-every-safe-point`.
//!
//! The scripts read prototypes with `Object.getPrototypeOf`: the accessor `__proto__` of Annex B
//! is not in the engine yet.
#![cfg(test)]

use crate::support;

use std::cell::Cell;

use support::host::{agent_with, limits, realm, stack_with_room};
use support::script::{Outcome, define_function, eval, eval_text, run, run_both};
use support::stack::{BUILTINS_REENTRY, on_bounded_stack, reach_through, stack_limits, test_path};
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

/// Whether a test takes its gate count: with the feature `collect-every-safe-point`, where
/// every instruction and every call of a builtin collects, unless `SWELL_FULL_COUNTS=1` (the
/// lane `slow`) asks for the full one.
fn gate_count() -> bool {
    cfg!(feature = "collect-every-safe-point")
        && !std::env::var("SWELL_FULL_COUNTS").is_ok_and(|value| value == "1")
}

/// `source` answers the String `expected` in sloppy code: for a script none of whose steps
/// depends on strictness, run once, since with the feature `collect-every-safe-point` every
/// instruction collects (each of the two tests that use it took 1.6 CPU seconds there run in
/// both modes, measured).
fn answers_once(source: &str, expected: &str) {
    assert_eq!(
        run(limits(), source),
        Outcome::Normal(format!("\"{expected}\"")),
        "{source}"
    );
}

/// The 13 functions of `Reflect` in the order of their clauses (28.1.1 to 28.1.13), each with
/// its "length", the count of its parameters before the first optional one.
const FUNCTIONS: [(&str, u32); 13] = [
    ("apply", 3),
    ("construct", 2),
    ("defineProperty", 3),
    ("deleteProperty", 2),
    ("get", 2),
    ("getOwnPropertyDescriptor", 2),
    ("getPrototypeOf", 1),
    ("has", 2),
    ("isExtensible", 1),
    ("ownKeys", 1),
    ("preventExtensions", 1),
    ("set", 3),
    ("setPrototypeOf", 2),
];

/// `#sec-reflect-object`: `Reflect` is an ordinary, extensible object whose [[Prototype]] is
/// `%Object.prototype%`, neither callable nor a constructor, the global property `Reflect` {
/// [[Writable]]: true, [[Enumerable]]: false, [[Configurable]]: true } (clause 18). Its own
/// String keys are the 13 functions in the order of their clauses, each a method (clause 18)
/// whose "length" and "name" are { [[Writable]]: false, [[Enumerable]]: false,
/// [[Configurable]]: true } with the length its clause gives, its [[Prototype]]
/// `%Function.prototype%`, no "prototype" and no [[Construct]]; its one Symbol key is
/// `%Symbol.toStringTag%`, "Reflect", { [[Writable]]: false, [[Enumerable]]: false,
/// [[Configurable]]: true } (`#sec-reflect-%symbol.tostringtag%`), so `Object.prototype.toString`
/// answers "[object Reflect]".
#[test]
fn reflect_is_an_ordinary_object_with_its_tag_and_13_functions() {
    let source = "var r = [];\n\
         r.push(typeof Reflect, Object.getPrototypeOf(Reflect) === Object.prototype,\n\
           Object.isExtensible(Reflect), Object.prototype.toString.call(Reflect));\n\
         var g = Object.getOwnPropertyDescriptor(this, \"Reflect\");\n\
         r.push(g.value === Reflect, g.writable, g.enumerable, g.configurable);\n\
         try { Reflect(); r.push(\"called\"); } catch (e) { r.push(e.name); }\n\
         try { new Reflect(); r.push(\"constructed\"); } catch (e) { r.push(e.name); }\n\
         var symbols = Object.getOwnPropertySymbols(Reflect);\n\
         var t = Object.getOwnPropertyDescriptor(Reflect, Symbol.toStringTag);\n\
         r.push(symbols.length, symbols[0] === Symbol.toStringTag, t.value, t.writable,\n\
           t.enumerable, t.configurable);\n\
         var names = Object.getOwnPropertyNames(Reflect);\n\
         r.push(names.join(\"+\"));\n\
         for (var i = 0; i < names.length; i++) {\n\
           var p = Object.getOwnPropertyDescriptor(Reflect, names[i]), f = p.value;\n\
           var l = Object.getOwnPropertyDescriptor(f, \"length\");\n\
           var n = Object.getOwnPropertyDescriptor(f, \"name\");\n\
           var made;\n\
           try { new f(); made = \"constructed\"; } catch (e) { made = e.name; }\n\
           r.push([typeof f, p.writable, p.enumerable, p.configurable, l.value, l.writable,\n\
             l.enumerable, l.configurable, n.value, n.writable, n.enumerable, n.configurable,\n\
             Object.getPrototypeOf(f) === Function.prototype, f.hasOwnProperty(\"prototype\"),\n\
             made].join(\"/\"));\n\
         }\n\
         r.join();";
    let names: Vec<&str> = FUNCTIONS.iter().map(|&(name, _)| name).collect();
    let mut expected = vec![
        "object,true,true,[object Reflect],true,true,false,true,TypeError,TypeError".to_string(),
        "1,true,Reflect,false,false,true".to_string(),
        names.join("+"),
    ];
    for (name, length) in FUNCTIONS {
        expected.push(format!(
            "function/true/false/true/{length}/false/false/true/{name}/false/false/true/true/false/TypeError"
        ));
    }
    answers_once(source, &expected.join(","));
}

/// The first step of each clause: `apply` throws a TypeError when its target is not callable
/// (`#sec-reflect.apply`, step 1), `construct` when its target or a `newTarget` that is present
/// is not a constructor (`#sec-reflect.construct`, steps 1 to 3), and each of the other eleven
/// when its target is not an Object (step 1 of each), so `setPrototypeOf` also when its `proto`
/// is neither an Object nor null (step 2). Each throws before any other argument is read: one
/// object given as every other argument, whose `toString` and `valueOf` (a key), `length` (an
/// argument list) and `value` and `enumerable` getters (a descriptor) count, is read 0 times
/// over the 13 functions with each of the 6 kinds of primitive target and 8 calls more. At the
/// gate count of the feature `collect-every-safe-point` the kinds are 2, `undefined` and a
/// Symbol: with all 6 the test took 2.06 CPU seconds there, measured, over the bound of 2.
#[test]
fn each_function_throws_a_type_error_on_a_non_object_target_before_reading_its_other_arguments() {
    let targets = if gate_count() {
        "undefined, Symbol(\"t\")"
    } else {
        "undefined, null, 1, \"s\", true, Symbol(\"t\")"
    };
    let calls = FUNCTIONS.len() * targets.split(", ").count() + 8;
    let source = "var reads = 0;\n\
         var probe = { toString: function () { reads++; return \"k\"; },\n\
           valueOf: function () { reads++; return 1; },\n\
           get length() { reads++; return 0; },\n\
           get value() { reads++; return 1; }, get enumerable() { reads++; return true; } };\n\
         var names = Object.getOwnPropertyNames(Reflect);\n\
         var targets = [TARGETS];\n\
         var count = 0, other = [];\n\
         for (var i = 0; i < targets.length; i++) {\n\
           for (var j = 0; j < names.length; j++) {\n\
             try { Reflect[names[j]](targets[i], probe, probe, probe); other.push(names[j]); }\n\
             catch (e) { if (e instanceof TypeError) count++; else other.push(names[j]); }\n\
           }\n\
         }\n\
         function F() {}\n\
         var more = [[Reflect.construct, F, probe, {}], [Reflect.construct, F, probe, undefined],\n\
           [Reflect.construct, F, probe, Math.max], [Reflect.construct, Math.max, probe],\n\
           [Reflect.apply, {}, null, probe], [Reflect.setPrototypeOf, {}, 1],\n\
           [Reflect.setPrototypeOf, {}, undefined], [Reflect.setPrototypeOf, {}, \"p\"]];\n\
         for (var m = 0; m < more.length; m++) {\n\
           var c = more[m];\n\
           try { if (c.length === 4) c[0](c[1], c[2], c[3]); else c[0](c[1], c[2]); other.push(m); }\n\
           catch (e) { if (e instanceof TypeError) count++; else other.push(m); }\n\
         }\n\
         [count, other.join(\"+\"), reads].join();";
    answers_once(&source.replace("TARGETS", targets), &format!("{calls},,0"));
}

/// `#sec-reflect.apply` and `#sec-reflect.construct` with CreateListFromArrayLike
/// (`#sec-createlistfromarraylike`): the argument list is read by its "length", through a
/// getter, then by Get of each index in order, a hole read as `undefined` and a getter of an
/// element run once in its turn; the callee gets the list and, for `apply`, the given this
/// value. A list that is not an object, an absent list included, throws a TypeError, unlike
/// `Function.prototype.apply`. A built-in function is a target too.
#[test]
fn reflect_apply_and_construct_copy_an_array_like() {
    answers(
        "var log = [], t = {};\n\
         function show() {\n\
           var s = [];\n\
           for (var i = 0; i < arguments.length; i++) s.push(String(arguments[i]));\n\
           return s.join(\"/\");\n\
         }\n\
         var like = { get length() { log.push(\"length\"); return 3; }, 0: \"a\",\n\
           get 2() { log.push(\"2\"); return \"c\"; } };\n\
         var r = [Reflect.apply(function () {\n\
           return (this === t) + \":\" + arguments.length + \":\" + show.apply(null, arguments);\n\
         }, t, like), log.join(\"+\")];\n\
         log = [];\n\
         function C() { this.args = show.apply(null, arguments); this.n = arguments.length; }\n\
         var c = Reflect.construct(C, like);\n\
         r.push(c.args, c.n, c instanceof C, log.join(\"+\"));\n\
         r.push(Reflect.apply(Math.max, undefined, [1, 3, 2]));\n\
         r.push(Reflect.apply(show, null, [, \"b\"]));\n\
         r.push(Reflect.apply(show, null, { length: \"2.5\", 0: \"x\", 1: \"y\", 2: \"z\" }));\n\
         r.push(Reflect.apply(show, null, { length: -1, 0: \"x\" }).length);\n\
         r.push(Reflect.apply(show, null, (function () { return arguments; })(7, 8)));\n\
         r.push(Reflect.construct(Array, [1, 2]).join(\"/\"));\n\
         r.join();",
        "true:3:a/undefined/c,length+2,a/undefined/c,3,true,length+2,3,undefined/b,x/y,0,7/8,1/2",
    );
    for source in [
        "Reflect.apply(function () {});",
        "Reflect.apply(function () {}, null);",
        "Reflect.apply(function () {}, null, 1);",
        "Reflect.apply(function () {}, null, \"ab\");",
        "Reflect.construct(function () {});",
        "Reflect.construct(function () {}, null);",
        "Reflect.construct(function () {}, true);",
    ] {
        throws(source, "TypeError");
    }
}

/// `#sec-reflect.construct`, steps 2 and 5: with a `newTarget` the object is made from
/// `newTarget`'s "prototype" (`#sec-getprototypefromconstructor`), for a builtin constructor
/// (`Date`, `Error`, `Array`) as for an ECMAScript function, and when that "prototype" is no
/// object from the intrinsic of the realm of `newTarget`; without one `newTarget` is the target.
/// A `newTarget` that is not a constructor (an object, an arrow function, a built-in function
/// that is no constructor, `undefined` given) throws a TypeError.
#[test]
fn reflect_construct_uses_new_target_for_the_prototype() {
    answers(
        "function F() {}\n\
         F.prototype = { tag: \"F\" };\n\
         var r = [];\n\
         var d = Reflect.construct(Date, [0], F);\n\
         r.push(Object.getPrototypeOf(d) === F.prototype, Date.prototype.getTime.call(d));\n\
         var e = Reflect.construct(Error, [\"m\"], F);\n\
         r.push(Object.getPrototypeOf(e) === F.prototype, e.message);\n\
         var a = Reflect.construct(Array, [2], F);\n\
         r.push(Object.getPrototypeOf(a) === F.prototype, Array.isArray(a), a.length);\n\
         function G() { this.k = 1; }\n\
         var o = Reflect.construct(G, [], F);\n\
         r.push(Object.getPrototypeOf(o) === F.prototype, o.k, o.tag);\n\
         var p = Reflect.construct(G, []);\n\
         r.push(Object.getPrototypeOf(p) === G.prototype);\n\
         function H() {}\n\
         H.prototype = null;\n\
         r.push(Object.getPrototypeOf(Reflect.construct(Date, [], H)) === Date.prototype);\n\
         r.push(Object.getPrototypeOf(Reflect.construct(G, [], H)) === Object.prototype);\n\
         r.join();",
        "true,0,true,m,true,true,2,true,1,F,true,true,true",
    );
    for source in [
        "Reflect.construct(Date, [], {});",
        "Reflect.construct(Date, [], () => {});",
        "Reflect.construct(Date, [], Math.max);",
        "Reflect.construct(Date, [], undefined);",
        "Reflect.construct(function () {}, [], Reflect.apply);",
    ] {
        throws(source, "TypeError");
    }
}

/// `#sec-reflect.defineproperty`: the answer of [[DefineOwnProperty]] itself, so `false` where
/// `Object.defineProperty` throws a TypeError (a change of a property that is not configurable,
/// a new property of an object that is not extensible, an element of a frozen Array), `true`
/// where it defines; the key is made by ToPropertyKey and the descriptor by
/// ToPropertyDescriptor, which throws a TypeError for one that is no object, and an Array's
/// `length` that is no valid length throws the RangeError of ArraySetLength.
#[test]
fn reflect_define_property_answers_false_where_define_throws() {
    answers(
        "var o = {}, r = [];\n\
         Object.defineProperty(o, \"x\", { value: 1 });\n\
         r.push(Reflect.defineProperty(o, \"x\", { value: 2 }), o.x);\n\
         try { Object.defineProperty(o, \"x\", { value: 2 }); r.push(\"defined\"); } catch (e) { r.push(e.name); }\n\
         r.push(Reflect.defineProperty(o, \"x\", { value: 1 }));\n\
         r.push(Reflect.defineProperty(o, \"y\", { value: 3, enumerable: true }));\n\
         var d = Object.getOwnPropertyDescriptor(o, \"y\");\n\
         r.push(d.value, d.writable, d.enumerable, d.configurable);\n\
         var sealed = Object.preventExtensions({});\n\
         r.push(Reflect.defineProperty(sealed, \"z\", { value: 1 }), sealed.hasOwnProperty(\"z\"));\n\
         var frozen = Object.freeze([1]);\n\
         r.push(Reflect.defineProperty(frozen, \"0\", { value: 2 }), frozen[0]);\n\
         r.push(Reflect.defineProperty(o, { toString: function () { return \"w\"; } }, { value: 5 }), o.w);\n\
         r.push(Reflect.defineProperty(o, 7, { get: function () { return \"g\"; } }), o[7]);\n\
         r.join();",
        "false,1,TypeError,true,true,3,false,true,false,false,false,false,1,true,5,true,g",
    );
    throws("Reflect.defineProperty({}, \"x\", 1);", "TypeError");
    throws("Reflect.defineProperty({}, \"x\");", "TypeError");
    throws(
        "Reflect.defineProperty({}, \"x\", { get: 1 });",
        "TypeError",
    );
    throws(
        "Reflect.defineProperty([], \"length\", { value: -1 });",
        "RangeError",
    );
}

/// `#sec-reflect.set` and `#sec-reflect.get` with a `receiver`: a data property of the target is
/// written on the receiver (OrdinarySetWithOwnDescriptor, `#sec-ordinarysetwithowndescriptor`),
/// the target is left as it is, and a setter or getter runs with the receiver as its this
/// value; the answer is `false` when the receiver is no object (`undefined` given included),
/// when the receiver's own property is not writable or is an accessor, and when the target's
/// property is not writable. Without a receiver the target is the receiver.
#[test]
fn reflect_set_with_a_receiver_writes_the_receiver() {
    answers(
        "var r = [], target = { x: 1 }, receiver = {};\n\
         r.push(Reflect.set(target, \"x\", 2, receiver), target.x, receiver.x);\n\
         var s = { set v(a) { this.got = a; } };\n\
         r.push(Reflect.set(s, \"v\", 3, receiver), receiver.got, s.got);\n\
         r.push(Reflect.set({ x: 1 }, \"x\", 2, 1), Reflect.set({ x: 1 }, \"x\", 2, undefined));\n\
         var fixed = {};\n\
         Object.defineProperty(fixed, \"x\", { value: 0, writable: false, configurable: true });\n\
         r.push(Reflect.set(target, \"x\", 4, fixed), fixed.x);\n\
         var accessor = { get x() { return \"a\"; }, set x(v) {} };\n\
         r.push(Reflect.set(target, \"x\", 5, accessor));\n\
         r.push(Reflect.set(Object.freeze({ x: 1 }), \"x\", 6, receiver), receiver.x);\n\
         var plain = {};\n\
         r.push(Reflect.set(plain, \"y\", 7), plain.y);\n\
         var g = { k: \"own\", get v() { return this.k; } };\n\
         r.push(Reflect.get(g, \"v\", { k: \"other\" }), Reflect.get(g, \"v\"));\n\
         r.push(Reflect.get([5, 6], 1), Reflect.get({}, \"missing\"));\n\
         r.join();",
        "true,1,2,true,3,,false,false,false,0,false,false,2,true,7,other,own,6,",
    );
}

/// `#sec-reflect.ownkeys`: CreateArrayFromList of [[OwnPropertyKeys]]
/// (`#sec-ordinaryownpropertykeys`): the array indices ascending, then the String keys in the
/// order they were made ("-1" and "4294967295", which are no array index, among them, and a
/// key that is not enumerable), then the Symbol keys in the order they were made; an Array and
/// a String object answer their indices and "length". The answer is a new Array whose
/// elements are Strings and Symbols.
#[test]
fn reflect_own_keys_answers_integer_keys_then_strings_then_symbols() {
    answers(
        "var s1 = Symbol(\"a\"), s2 = Symbol(\"b\"), o = {};\n\
         o.b = 1; o[s2] = 1; o[2] = 1; o.a = 1; o[s1] = 1; o[0] = 1; o[\"-1\"] = 1;\n\
         o[\"4294967295\"] = 1; o[1] = 1;\n\
         Object.defineProperty(o, \"hidden\", { value: 1 });\n\
         var keys = Reflect.ownKeys(o);\n\
         var text = function (list) {\n\
           return list.map(function (k) { return typeof k === \"symbol\" ? String(k) : k; }).join(\"+\");\n\
         };\n\
         [text(keys), Array.isArray(keys), typeof keys[0], keys[8] === s2, keys[9] === s1,\n\
          text(Reflect.ownKeys([1, 2])), text(Reflect.ownKeys(new String(\"ab\"))),\n\
          Reflect.ownKeys({}).length, Reflect.ownKeys(o) !== keys].join();",
        "0+1+2+b+a+-1+4294967295+hidden+Symbol(b)+Symbol(a),true,string,true,true,\
         0+1+length,0+1+length,0,true",
    );
}

/// `#sec-reflect.setprototypeof` with OrdinarySetPrototypeOf
/// (`#sec-ordinarysetprototypeof`): a prototype that would make a cycle is refused with `false`
/// and the prototype stays; so is a new prototype of an object that is not extensible (the same
/// one is `true`) and of `%Object.prototype%` (SetImmutablePrototype,
/// `#sec-set-immutable-prototype`; null, its own, is `true`); null is a prototype. Beside it
/// `getPrototypeOf`, `preventExtensions` and `isExtensible` answer what their internal method
/// answers.
#[test]
fn reflect_set_prototype_of_refuses_a_cycle_with_false() {
    answers(
        "var r = [], a = {}, b = Object.create(a), c = Object.create(b);\n\
         r.push(Reflect.setPrototypeOf(a, c), Object.getPrototypeOf(a) === Object.prototype);\n\
         r.push(Reflect.setPrototypeOf(a, a), Reflect.getPrototypeOf(c) === b);\n\
         r.push(Reflect.setPrototypeOf(b, null), Reflect.getPrototypeOf(b));\n\
         r.push(Reflect.setPrototypeOf(a, c), Reflect.getPrototypeOf(a) === c);\n\
         var fixed = {};\n\
         r.push(Reflect.isExtensible(fixed), Reflect.preventExtensions(fixed), Reflect.isExtensible(fixed));\n\
         r.push(Reflect.setPrototypeOf(fixed, {}), Reflect.setPrototypeOf(fixed, Object.prototype));\n\
         r.push(Reflect.setPrototypeOf(Object.prototype, {}), Reflect.setPrototypeOf(Object.prototype, null));\n\
         r.push(Reflect.preventExtensions(fixed));\n\
         r.join();",
        "false,true,false,true,true,,true,true,true,true,false,false,true,false,true,true",
    );
}

/// The body of the stack test through `Reflect.apply`.
fn reach_through_apply() {
    reach_through("Reflect.apply(f, null, [])", BUILTINS_REENTRY);
}

/// `Limits::min_stack_bytes` through `Reflect.apply`, which calls back into script
/// (`tests/support/stack.rs`): on seven eighths of `min_stack_bytes()` a script reaches
/// `reentry_depth` re-entries through `Reflect.apply(f, null, [])` and parses 1,000 nested
/// brackets at the deepest, without a stack overflow. In the collect mode the depth is
/// [`BUILTINS_REENTRY`].
#[test]
fn reflect_apply_reaches_the_reentry_limit_on_the_promised_stack() {
    let stack = stack_with_room(stack_limits(BUILTINS_REENTRY));
    on_bounded_stack(
        test_path!("reflect_apply_reaches_the_reentry_limit_on_the_promised_stack"),
        stack,
        reach_through_apply,
    )
    .unwrap();
}

/// The body of the stack test through `Reflect.construct`.
fn reach_through_construct() {
    reach_through("Reflect.construct(f, [])", BUILTINS_REENTRY);
}

/// As [`reflect_apply_reaches_the_reentry_limit_on_the_promised_stack`], through
/// `Reflect.construct(f, [])`: `f`, a plain function, is a constructor.
#[test]
fn reflect_construct_reaches_the_reentry_limit_on_the_promised_stack() {
    let stack = stack_with_room(stack_limits(BUILTINS_REENTRY));
    on_bounded_stack(
        test_path!("reflect_construct_reaches_the_reentry_limit_on_the_promised_stack"),
        stack,
        reach_through_construct,
    )
    .unwrap();
}

/// The step budget of the step test: 10,000 steps per entry.
const STEPS: u64 = 10_000;

/// CreateListFromArrayLike is a loop a script runs, a step per element (`AGENTS.md`, "Rules"):
/// `Reflect.apply` over an array-like of length 10^9 ends in `Terminated(StepBudget)` with
/// `steps_per_entry` 10,000, having spent at most those steps, and the agent runs the next
/// entry.
#[test]
fn reflect_apply_over_a_long_array_like_ends_at_the_step_budget() {
    let mut budget = limits();
    budget.steps_per_entry = STEPS;
    let mut agent = agent_with(budget).unwrap();
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "Reflect.apply(function () {}, null, { length: 1e9 });"
        ),
        Outcome::Terminated(Termination::StepBudget)
    );
    let spent = agent.last_entry_steps().unwrap();
    println!("{spent} steps of {STEPS}");
    assert!(spent <= STEPS, "{spent} steps");
    assert_eq!(agent.state(), AgentState::Alive);
    assert_eq!(
        eval_text(&mut agent, &realm, "Reflect.apply(Math.max, null, [1, 2]);"),
        Outcome::Normal("2".to_string())
    );
}

/// The memory a run of the memory test has beyond the realm's reading: 1 MiB.
const MEMORY_ROOM: u64 = 1 << 20;

/// A collection, which must complete.
fn collect(agent: &mut swell::Agent<support::host::TestHost>) {
    assert!(matches!(agent.collect().unwrap(), HostOutcome::Normal(_)));
}

/// The argument list of `Reflect.apply` is charged to the memory budget as it grows, a slot
/// per element, before the element is kept: over an array-like of length 2^32 - 1 whose first
/// eight elements are getters that each make a new object (kept on the list across the next
/// Get) and whose others are holes, with steps to spare and 1 MiB of memory over the realm's
/// reading, the run ends in `Terminated(MemoryBudget)`. The script holds nothing in a global;
/// once the host released the script, one collection brings `heap_bytes()` back within the
/// reading taken before the script (the test prints the readings). That reading is taken after
/// a first entry of the same script over a length of 8, which completes, and a collection: the
/// arenas keep the room that entry grew them by (1,600 bytes, measured without it), which no
/// collection gives back.
#[test]
fn reflect_apply_over_a_long_array_like_ends_at_the_memory_budget() {
    let script = |length: &str| {
        format!(
            "(function () {{\n\
               var like = {{ length: {length} }};\n\
               for (var i = 0; i < 8; i++) {{\n\
                 Object.defineProperty(like, i, {{ get: function () {{ return {{}}; }} }});\n\
               }}\n\
               return Reflect.apply(function () {{ return arguments.length; }}, null, like);\n\
             }})();"
        )
    };
    let mut budget = limits();
    budget.steps_per_entry = 1 << 40;
    let mut probe = agent_with(budget).unwrap();
    realm(&mut probe);
    collect(&mut probe);
    budget.memory_bytes = probe.heap_bytes().unwrap() + MEMORY_ROOM;
    let mut agent = agent_with(budget).unwrap();
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(&mut agent, &realm, &script("8")),
        Outcome::Normal("8".to_string())
    );
    collect(&mut agent);
    let before = agent.heap_bytes().unwrap();
    let outcome = eval(&mut agent, &realm, &script("4294967295"));
    assert!(
        matches!(outcome, HostOutcome::Terminated(Termination::MemoryBudget)),
        "{outcome:?}"
    );
    let terminated = agent.heap_bytes().unwrap();
    drop(outcome);
    collect(&mut agent);
    let after = agent.heap_bytes().unwrap();
    println!(
        "heap_bytes: {before} before the script, {terminated} at the termination, {after} after \
         one collection; budget {}",
        budget.memory_bytes
    );
    assert!(after <= before, "{before} before, {after} after");
    assert_eq!(agent.state(), AgentState::Alive);
    assert_eq!(
        eval_text(&mut agent, &realm, "Reflect.ownKeys([0]).length;"),
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

/// The keys of the object of the interrupt test.
const KEYS: u32 = 1_000;

/// The place of the getter among them: the eleventh key. Before it the loop reads 10 data
/// properties; with the feature `collect-every-safe-point`, where every instruction collects a
/// heap that holds the object, a getter in the middle of the 1,000 cost 12.4 CPU seconds,
/// measured.
const GETTER: u32 = 10;

/// An interrupt set by a host function that is the getter of one property, read in a loop over
/// `Reflect.ownKeys` of an object of 1,000 keys, ends the run in `Terminated(Interrupted)` at
/// that getter: the loop read the keys before it and no other, and the agent runs the next
/// entry.
#[test]
fn reflect_own_keys_ends_at_an_interrupt() {
    let n = KEYS;
    let middle = GETTER;
    let mut agent = agent_with(limits()).unwrap();
    let realm = realm(&mut agent);
    define_function(&mut agent, &realm, "interruptNow", 0, interrupt_now, false);
    // The object is made by one call of `JSON.parse`, so that no instruction runs per key.
    let members: Vec<String> = (0..n).map(|i| format!("\\\"k{i}\\\": {i}")).collect();
    let text = members.join(", ");
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            &format!(
                "var o = JSON.parse(\"{{{text}}}\"), seen = 0;\n\
                 Object.defineProperty(o, \"k{middle}\", {{ get: interruptNow }});\n\
                 Reflect.ownKeys(o).length;"
            )
        ),
        Outcome::Normal(n.to_string())
    );
    HANDLE.with(|slot| slot.set(Some(agent.interrupt_handle())));
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var keys = Reflect.ownKeys(o);\n\
             for (var j = 0; j < keys.length; j++) { o[keys[j]]; seen++; }"
        ),
        Outcome::Terminated(Termination::Interrupted)
    );
    assert_eq!(agent.state(), AgentState::Alive);
    assert_eq!(
        eval_text(&mut agent, &realm, "seen;"),
        Outcome::Normal(middle.to_string())
    );
}
