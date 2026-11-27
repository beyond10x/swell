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

/// `Reflect.apply` over an array-like of length 2^32 - 1 whose first eight elements are
/// getters that each make a new object (kept on the list across the next Get) and whose others
/// are holes, with steps to spare and 1 MiB of memory over the realm's reading, ends in
/// `Terminated(MemoryBudget)`. The script holds nothing in a global; once the host released the
/// script, one collection brings `heap_bytes()` back within the reading taken before the script
/// (the test prints the readings). That reading is taken after a first entry of the same script
/// over a length of 8, which completes, and a collection: the arenas keep the room that entry
/// grew them by (1,600 bytes, measured without it), which no collection gives back.
///
/// What this test holds: the copy of a long list ends at the memory budget, and what that run
/// was charged is given back. It does not hold that CreateListFromArrayLike charges each
/// element (`builtins/function.rs`): the growth of the kept stack is charged too, so the run
/// ends at the memory budget without the per-element charge as well. The charge test,
/// [`reflect_charges_are_given_back_on_every_way_out`], holds that the charges of `apply`,
/// `construct` and `ownKeys` are given back on each abrupt way out (equal readings after every
/// entry); it does not hold that a charge is made either.
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
///
/// What this test holds: an interrupt raised by a getter during a loop over the answer of
/// `ownKeys` ends the run. It does not hold a check of the flag inside `ownKeys`: the getter
/// runs after `Reflect.ownKeys` returned. The flag is read by `charge_steps`
/// (`host/agent.rs`), which the entry of every builtin call spends a step through
/// (`enter_function`) and CreateArrayFromList calls for each element; on one thread, between
/// that entry step and the return of an `ownKeys` without a Proxy trap no code of the host or
/// of the script runs, so the host of these tests cannot raise the flag there (a second thread
/// could, but not at a point a test chooses). The `ownKeys` trap of a Proxy could
/// (`story:proxy-descriptor-prototype-and-extensibility-traps`).
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

/// `#sec-reflect.get`, step 3: a `receiver` given as `undefined` is present (clause 18), so the
/// getter runs with `undefined` as its this value, not with the target; without a receiver the
/// target is the this value, and a primitive receiver is the this value as it is in strict code.
#[test]
fn reflect_get_with_an_undefined_receiver_runs_the_getter_with_undefined() {
    answers(
        r#"var g = { get x() { "use strict"; return this; } };
        [Reflect.get(g, "x", undefined) === undefined, Reflect.get(g, "x") === g,
         typeof Reflect.get(g, "x", 5)].join();"#,
        "true,true,number",
    );
}

/// `#sec-reflect.set`, step 3: a `receiver` given as `undefined` is present, so a setter runs
/// with `undefined` as its this value and the answer is `true`; without a receiver the target is
/// the this value, and a primitive receiver is the this value as it is in strict code.
#[test]
fn reflect_set_with_an_undefined_receiver_runs_the_setter_with_undefined() {
    answers(
        r#"var seen = "none", s = { set x(v) { "use strict"; seen = this; } };
        var r = [Reflect.set(s, "x", 1, undefined), seen === undefined];
        r.push(Reflect.set(s, "x", 1), seen === s);
        r.push(Reflect.set(s, "x", 1, 5), typeof seen);
        r.join();"#,
        "true,true,true,true,true,number",
    );
}

/// `#sec-reflect.deleteproperty`, step 3: the target's own [[Delete]], for an Array the one
/// that reaches its elements: an element of a dense Array is deleted (`true`, then `0 in a` is
/// `false` and "length" stays), and an element that is not configurable answers `false` and
/// stays.
#[test]
fn reflect_delete_property_runs_the_delete_of_an_array() {
    answers(
        r#"var a = [1, 2, 3], r = [];
        r.push(Reflect.deleteProperty(a, 0), 0 in a, a[0], a.length);
        var b = [1, 2, 3];
        Object.defineProperty(b, 1, { configurable: false });
        r.push(Reflect.deleteProperty(b, 1), 1 in b, b[1], Reflect.deleteProperty(b, 2), 2 in b);
        r.join();"#,
        "true,false,,3,false,true,2,true,false",
    );
}

/// The order of every observable step of each clause with arguments (28.1.1 to 28.1.12):
/// ToPropertyKey (by `toString`, or `Symbol.toPrimitive` with the hint "string") before
/// ToPropertyDescriptor and its six reads in the order of `#sec-topropertydescriptor`; the
/// value of `set` never converted; the getter and setter run with the receiver; the argument
/// list read by "length" and each index before the call, and the `newTarget`'s "prototype"
/// read by [[Construct]] after the list.
#[test]
fn reflect_reads_its_arguments_in_the_order_of_each_clause() {
    answers(
        r#"var log = [], r = [];
        function key(name) {
          return { toString: function () { log.push("key:" + name); return name; },
                   valueOf: function () { log.push("valueOf"); return 0; } };
        }
        function desc(fields) {
          var d = {};
          fields.forEach(function (f) {
            Object.defineProperty(d, f, { get: function () { log.push(f); return undefined; }, enumerable: true });
          });
          return d;
        }
        function likeLog(n) {
          var like = {};
          Object.defineProperty(like, "length", { get: function () { log.push("length"); return n; } });
          for (var i = 0; i < n; i++) {
            Object.defineProperty(like, i, { get: (function (i) { return function () { log.push(String(i)); return "e" + i; }; })(i) });
          }
          return like;
        }
        function step(label, f) {
          log = [];
          var answer;
          try { var v = f(); answer = typeof v === "object" && v !== null ? "obj" : String(v); }
          catch (e) { answer = e.name; }
          r.push(label + "=" + answer + "[" + log.join(" ") + "]");
        }
        var o = { k: 1 }, recv = {}, recv2 = {};
        var o2 = { get g() { log.push("getter:" + (this === recv)); return 5; } };
        var v = { toString: function () { log.push("value"); return "v"; } };
        var o3 = { set s(x) { log.push("setter:" + (this === recv2) + ":" + (x === v)); } };
        var pk = {};
        pk[Symbol.toPrimitive] = function (hint) { log.push("hint:" + hint); return "p"; };
        step("defineProperty", function () { return Reflect.defineProperty(o, key("a"), desc(["enumerable", "configurable", "value", "writable", "get", "set"])); });
        step("defineProperty2", function () { return Reflect.defineProperty(o, key("b"), desc(["value"])); });
        step("deleteProperty", function () { return Reflect.deleteProperty(o, key("k")); });
        step("get", function () { return Reflect.get(o2, key("g"), recv); });
        step("getOwnPropertyDescriptor", function () { return Reflect.getOwnPropertyDescriptor(o2, key("g")); });
        step("has", function () { return Reflect.has(o2, key("g")); });
        step("hasPrim", function () { return Reflect.has({ p: 1 }, pk); });
        step("set", function () { return Reflect.set(o3, key("s"), v, recv2); });
        step("apply", function () { return Reflect.apply(function () { log.push("call:" + arguments.length); return arguments[1]; }, null, likeLog(2)); });
        Object.defineProperty(Function.prototype, "prototype", { get: function () { log.push("proto"); return undefined; }, configurable: true });
        var NT = (function () {}).bind(null);
        step("construct", function () { return Reflect.construct(function () { log.push("construct:" + arguments.length); }, likeLog(2), NT); });
        r.join("|");"#,
        "defineProperty=TypeError[key:a enumerable configurable value writable get set]|\
         defineProperty2=true[key:b value]|deleteProperty=true[key:k]|get=5[key:g getter:true]|\
         getOwnPropertyDescriptor=obj[key:g]|has=true[key:g]|hasPrim=true[hint:string]|\
         set=true[key:s setter:true:true]|apply=e1[length 0 1 call:2]|\
         construct=obj[length 0 1 proto construct:2]",
    );
}

/// An argument that is missing is `undefined` (clause 18): a missing key is the key
/// "undefined" and a missing value is `undefined`; a missing descriptor, prototype or target
/// throws the TypeError of its step, and so does an argument list that is no object or whose
/// "length" is a Symbol; a "length" getter that throws ends the call with its error.
#[test]
fn reflect_reads_a_missing_argument_as_undefined() {
    answers(
        r#"var r = [];
        r.push(Reflect.get({ undefined: "u" }));
        r.push(Reflect.has({ undefined: 1 }));
        var o = {};
        r.push(Reflect.set(o), o.hasOwnProperty("undefined"), o.undefined === undefined);
        r.push(Reflect.deleteProperty({ undefined: 1 }));
        r.push(Reflect.getOwnPropertyDescriptor({ undefined: 7 }).value);
        r.join();"#,
        "u,true,true,true,true,true,7",
    );
    for source in [
        "Reflect.defineProperty({});",
        "Reflect.setPrototypeOf({});",
        "Reflect.getPrototypeOf();",
        "Reflect.apply(function () {}, null, { length: Symbol() });",
        "Reflect.apply(function () {}, null, Symbol());",
    ] {
        throws(source, "TypeError");
    }
    throws(
        "Reflect.apply(function () {}, null, { get length() { throw new RangeError(\"x\"); } });",
        "RangeError",
    );
}

/// Array targets (`#sec-array-exotic-objects-defineownproperty-p-desc`, ArraySetLength):
/// `false`, never a throw, where an element is not deletable, the "length" is not writable or
/// a receiver's "length" refuses; the key order of both storages with an index-like key over
/// 2^32 - 2; the RangeError of ArraySetLength stays a throw.
#[test]
fn reflect_on_array_targets_answers_their_internal_methods() {
    answers(
        r#"var r = [];
        var a = [1, 2, 3];
        Object.defineProperty(a, 1, { value: 2, configurable: false });
        r.push(Reflect.defineProperty(a, "length", { value: 0 }), a.length);
        r.push(Reflect.deleteProperty(a, 1), Reflect.deleteProperty(a, 0), a.length, 0 in a);
        var b = [1, 2, 3];
        Object.defineProperty(b, 1, { configurable: false });
        r.push(Reflect.set(b, "length", 0), b.length);
        var c = [];
        Object.defineProperty(c, "length", { writable: false });
        r.push(Reflect.set(c, 0, 1), Reflect.defineProperty(c, 0, { value: 1 }), c.length, c.hasOwnProperty(0));
        r.push(Reflect.defineProperty(a, "length", { value: 2 }));
        var n = 0, d = [1, 2, 3];
        r.push(Reflect.defineProperty(d, "length", { value: { valueOf: function () { n++; return 1; } } }), d.length, n);
        var e = [1, 2];
        r.push(Reflect.defineProperty(e, 0, { get: function () { return 9; } }), e[0], Reflect.ownKeys(e).join("+"));
        var f = [];
        f[4294967294] = 1; f["4294967295"] = 1; f.x = 1; f[0] = 1; f[5] = 1;
        r.push(Reflect.ownKeys(f).join("+"), f.length);
        var recv = [1, 2, 3];
        r.push(Reflect.set([], "length", 1, recv), recv.length);
        r.push(Reflect.set({}, 0, 5, c));
        var gd = Reflect.getOwnPropertyDescriptor([7], "length");
        r.push(gd.value, gd.writable, gd.enumerable, gd.configurable);
        r.push(Reflect.has([, 1], 0), Reflect.has([, 1], 1));
        r.join();"#,
        "false,2,false,true,2,false,false,2,false,false,0,false,true,true,1,2,true,9,0+1+length,\
         0+5+4294967294+length+4294967295+x,4294967295,true,1,false,1,true,false,false,false,true",
    );
    throws(
        "Reflect.defineProperty([], \"length\", { value: 4294967296 });",
        "RangeError",
    );
    throws("Reflect.set([], \"length\", -1);", "RangeError");
}

/// String objects (`#sec-string-exotic-objects`): the indices of the String, then the other
/// indices, then "length" and the String keys; an index of the String is read-only and not
/// configurable, so `defineProperty` of another value, `set` and `deleteProperty` answer
/// `false`.
#[test]
fn reflect_on_string_object_targets_answers_their_internal_methods() {
    answers(
        r#"var r = [];
        var s = new String("ab");
        s.x = 1; s[5] = 1;
        r.push(Reflect.ownKeys(s).join("+"));
        r.push(Reflect.defineProperty(s, "0", { value: "a" }), Reflect.defineProperty(s, "0", { value: "z" }),
          Reflect.defineProperty(s, "2", { value: "c" }), s[2]);
        r.push(Reflect.set(s, "0", "z"), Reflect.set(s, "length", 5), Reflect.deleteProperty(s, "0"),
          Reflect.deleteProperty(s, "length"), Reflect.deleteProperty(s, "x"));
        var d = Reflect.getOwnPropertyDescriptor(s, "1");
        r.push(d.value, d.writable, d.enumerable, d.configurable, Reflect.has(s, "1"), Reflect.get(s, "1"), Reflect.has(s, "3"));
        r.push(Reflect.preventExtensions(s), Reflect.defineProperty(s, "y", { value: 1 }), Reflect.isExtensible(s));
        r.join();"#,
        "0+1+5+length+x,true,false,true,c,false,false,false,false,true,b,false,true,false,true,b,false,true,false,false",
    );
}

/// A mapped arguments object (`#sec-arguments-exotic-objects`, sloppy code): its key order,
/// `defineProperty` and `set` reach the parameter, `set` with another receiver leaves the
/// parameter and writes the receiver (10.4.4.4, step 1), a non-writable definition and a
/// delete unmap it.
#[test]
fn reflect_on_mapped_arguments_targets_answers_their_internal_methods() {
    answers_once(
        r#"function f(a, b) {
          var r = [];
          r.push(Reflect.ownKeys(arguments).map(String).join("+"));
          r.push(Reflect.defineProperty(arguments, "0", { value: 5 }), a);
          r.push(Reflect.set(arguments, 1, 6), b);
          var recv = {};
          r.push(Reflect.set(arguments, 0, 9, recv), a, recv[0]);
          r.push(Reflect.defineProperty(arguments, "0", { value: 7, writable: false }), a);
          a = 8;
          r.push(arguments[0]);
          r.push(Reflect.deleteProperty(arguments, 1), b, Reflect.has(arguments, 1));
          b = 10;
          r.push(arguments[1]);
          r.push(Reflect.getOwnPropertyDescriptor(arguments, "callee").value === f);
          return r.join();
        }
        f(1, 2);"#,
        "0+1+length+callee+Symbol(Symbol.iterator),true,5,true,6,true,5,9,true,7,7,true,6,false,,true",
    );
}

/// `construct` over a bound function
/// (`#sec-bound-function-exotic-objects-construct-argumentslist-newtarget`, step 5: a
/// `newTarget` that is the bound function becomes its target), a bound function as `newTarget`
/// (no "prototype": the realm's intrinsic), a String with a `newTarget`, and the targets
/// IsConstructor refuses.
#[test]
fn reflect_construct_over_bound_and_other_targets() {
    answers(
        r#"var r = [];
        function F() { this.made = true; }
        var B = F.bind(null);
        var o = Reflect.construct(B, []);
        r.push(Object.getPrototypeOf(o) === F.prototype, o.made);
        function N() {}
        var p = Reflect.construct(B, [], N);
        r.push(Object.getPrototypeOf(p) === N.prototype, p.made);
        var q = Reflect.construct(F, [], B);
        r.push(Object.getPrototypeOf(q) === Object.prototype, q.made);
        var s = Reflect.construct(String, ["ab"], N);
        r.push(Object.getPrototypeOf(s) === N.prototype, s.length, Reflect.ownKeys(s).join("+"));
        var errs = [];
        var targets = [function* () {}, Symbol, Reflect.construct, ({ m() {} }).m, function () {}.bind, () => 1];
        for (var i = 0; i < targets.length; i++) {
          try { Reflect.construct(targets[i], []); errs.push("made"); } catch (e) { errs.push(e.name); }
        }
        r.push(errs.join("+"));
        r.join();"#,
        "true,true,true,true,true,true,true,2,0+1+length,\
         TypeError+TypeError+TypeError+TypeError+TypeError+TypeError",
    );
}

/// The global object as target: its non-writable, non-configurable values answer `false`, a
/// `var` is not deletable, and `Reflect` itself is { [[Writable]]: true, [[Enumerable]]:
/// false, [[Configurable]]: true } and deletable.
#[test]
fn reflect_on_the_global_object_answers_its_internal_methods() {
    answers(
        r#"var R = Reflect, g = this, r = [];
        var v = 1;
        r.push(R.deleteProperty(g, "NaN"), R.defineProperty(g, "undefined", { value: 1 }), R.set(g, "Infinity", 0), Infinity);
        var found = false, ks = R.ownKeys(g);
        for (var i = 0; i < ks.length; i++) { if (ks[i] === "Reflect") found = true; }
        r.push(R.has(g, "Reflect"), found, R.deleteProperty(g, "v"));
        var gd = R.getOwnPropertyDescriptor(g, "Reflect");
        r.push(gd.writable, gd.enumerable, gd.configurable);
        r.push(R.deleteProperty(g, "Reflect"), typeof Reflect, R.has(g, "Reflect"));
        r.join();"#,
        "false,false,false,Infinity,true,true,false,true,false,true,true,undefined,false",
    );
}

/// A host object of the test host (`docs/design/host-interface.md`, decision 15), with a null
/// prototype: each function answers what the ordinary internal method answers, and `apply`
/// and `construct` refuse it.
#[test]
fn reflect_on_a_host_object_answers_its_internal_methods() {
    let mut agent = agent_with(limits()).unwrap();
    let realm = realm(&mut agent);
    let global_root = support::host::global(&mut agent, &realm);
    agent
        .with_cx(&realm, |cx| {
            let host = cx
                .new_host_object(swell::HostObjectSpec {
                    prototype: swell::Prototype::Null,
                    brand: swell::Brand(9),
                    data: None,
                    error_kind: None,
                    overrides: None,
                })
                .unwrap()
                .unwrap();
            let global = support::host::object(cx, &global_root);
            let key = support::host::key(cx, "h");
            cx.set(global, key, Value::Object(host), true)
                .unwrap()
                .unwrap();
        })
        .unwrap();
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            r#"var r = [];
            r.push(Reflect.getPrototypeOf(h) === null, Reflect.isExtensible(h), Reflect.ownKeys(h).length);
            r.push(Reflect.defineProperty(h, "x", { value: 1 }), Reflect.set(h, "y", 2), Reflect.get(h, "y"), Reflect.has(h, "x"));
            r.push(Reflect.ownKeys(h).join("+"), Reflect.deleteProperty(h, "x"), Reflect.deleteProperty(h, "y"));
            r.push(Reflect.setPrototypeOf(h, Object.prototype), Reflect.getPrototypeOf(h) === Object.prototype);
            r.push(Reflect.preventExtensions(h), Reflect.isExtensible(h), Reflect.set(h, "z", 1));
            try { Reflect.apply(h, null, []); r.push("called"); } catch (e) { r.push(e.name); }
            try { Reflect.construct(h, []); r.push("made"); } catch (e) { r.push(e.name); }
            r.join();"#
        ),
        Outcome::Normal(
            "\"true,true,0,true,true,2,true,x+y,false,true,true,true,true,false,false,TypeError,TypeError\""
                .to_string()
        )
    );
}

/// The body of [`reflect_builtin_only_reentry_cycles_end_in_a_range_error`].
fn builtin_only_cycles() {
    let (mut agent, realm) = support::stack::stack_agent(BUILTINS_REENTRY);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            r#"var r = [];
            var a = [Reflect.apply, null]; a.push(a);
            try { Reflect.apply(Reflect.apply, null, a); r.push("none"); } catch (e) { r.push(e.name); }
            var o = {};
            Object.defineProperty(o, "x", { get: Reflect.get.bind(null, o, "x") });
            try { o.x; r.push("none"); } catch (e) { r.push(e.name); }
            function C() { return Reflect.construct(C, []); }
            try { Reflect.construct(C, []); r.push("none"); } catch (e) { r.push(e.name); }
            r.push(Reflect.apply(Math.max, null, [1, 2]));
            r.join();"#
        ),
        Outcome::Normal("\"RangeError,RangeError,RangeError,2\"".to_string())
    );
}

/// Re-entries with no script frame between them, on seven eighths of `min_stack_bytes()`:
/// `Reflect.apply` applying itself to an array that holds itself, a getter that is
/// `Reflect.get` bound to its own property, and a constructor that constructs itself through
/// `Reflect.construct`, each ends in the RangeError of "Depth bounds".
#[test]
fn reflect_builtin_only_reentry_cycles_end_in_a_range_error() {
    let stack = stack_with_room(stack_limits(BUILTINS_REENTRY));
    on_bounded_stack(
        test_path!("reflect_builtin_only_reentry_cycles_end_in_a_range_error"),
        stack,
        builtin_only_cycles,
    )
    .unwrap();
}

/// The scripts of the charge test that end normally: a getter that throws in the middle of
/// CreateListFromArrayLike, a callee of `apply` and of `construct` that throws after the list
/// was made, a "length" that is a Symbol, `ownKeys` of an object and of an Array, a key whose
/// `toString` throws and a getter that throws inside `Reflect.get`.
const CHARGE_SCRIPTS: [(&str, &str); 7] = [
    (
        "throwing getter in the list",
        "(function () { try { Reflect.apply(function () {}, null, { length: 200, get 100() { throw 1; } }); } catch (e) {} })();",
    ),
    (
        "throwing callee of apply",
        "(function () { try { Reflect.apply(function () { throw 2; }, null, { length: 200 }); } catch (e) {} })();",
    ),
    (
        "throwing callee of construct",
        "(function () { try { Reflect.construct(function () { throw 3; }, { length: 200 }); } catch (e) {} })();",
    ),
    (
        "Symbol length",
        "(function () { try { Reflect.apply(function () {}, null, { length: Symbol() }); } catch (e) {} })();",
    ),
    (
        "ownKeys",
        "(function () { var o = {}; for (var i = 0; i < 30; i++) o[\"k\" + i] = i; Reflect.ownKeys(o); Reflect.ownKeys([1, 2, 3]); })();",
    ),
    (
        "throwing key",
        "(function () { try { Reflect.defineProperty({}, { toString: function () { throw 4; } }, {}); } catch (e) {} })();",
    ),
    (
        "throwing getter of get",
        "(function () { try { Reflect.get({ get x() { throw 5; } }, \"x\"); } catch (e) {} })();",
    ),
];

/// The script of the charge test that ends at the step budget inside a callee of
/// `Reflect.apply`.
const CHARGE_TERMINATING: &str =
    "Reflect.apply(Reflect.apply, null, [function () {}, null, { length: 1e9 }]);";

/// The controls of the charge test, which `reflect.rs` does not reach: the same calls through
/// `Function.prototype.apply`, and a plain throw.
const CHARGE_CONTROLS: [(&str, &str); 3] = [
    (
        "control: throwing getter, Function.prototype.apply",
        "(function () { try { (function () {}).apply(null, { length: 200, get 100() { throw 1; } }); } catch (e) {} })();",
    ),
    (
        "control: throwing callee, Function.prototype.apply",
        "(function () { try { (function () { throw 2; }).apply(null, { length: 200 }); } catch (e) {} })();",
    ),
    (
        "control: plain throw",
        "(function () { try { throw 1; } catch (e) {} })();",
    ),
];

/// The counts of the charge test: the entries of each script alone, the rounds in one agent and
/// the steps per entry; 4, 2 and 3,000, and at the gate count of the feature
/// `collect-every-safe-point` 3, 1 and 2,000. There, with 5 entries, 5 rounds and 20,000 steps
/// the test took 4.08 s, and with 5, 2 and 5,000 at the full count 2.57 CPU seconds, measured,
/// over the bounds of 2 and 2.5. Three entries give two readings after the first to compare;
/// 2,000 steps are several times what each script that ends normally spends, and the
/// terminating one spends them all.
fn charge_counts() -> (usize, usize, u64) {
    if gate_count() {
        (3, 1, 2_000)
    } else {
        (4, 2, 3_000)
    }
}

/// Every charge of `reflect.rs` is given back on every way out: each script of
/// [`CHARGE_SCRIPTS`], [`CHARGE_TERMINATING`] and each of [`CHARGE_CONTROLS`] runs alone in a
/// fresh agent for several entries, with a collection and a `heap_bytes()` reading after each. The readings from the second on are equal and none is
/// above the first (the first entry grows the arenas); a charge that is not given back raises
/// each reading by its bytes. Before that, all the scripts run as rounds in one agent, each
/// round answering what it answers (the readings printed).
#[test]
fn reflect_charges_are_given_back_on_every_way_out() {
    let (entries, rounds, steps) = charge_counts();
    charge_rounds_in_one_agent(rounds, steps);
    let alone = CHARGE_SCRIPTS
        .iter()
        .copied()
        .chain([("termination in the callee", CHARGE_TERMINATING)])
        .chain(CHARGE_CONTROLS);
    let mut grew = Vec::new();
    for (name, script) in alone {
        let mut budget = limits();
        budget.steps_per_entry = steps;
        let mut agent = agent_with(budget).unwrap();
        let realm = realm(&mut agent);
        let mut readings = Vec::new();
        for _ in 0..entries {
            let _ = eval_text(&mut agent, &realm, script);
            collect(&mut agent);
            readings.push(agent.heap_bytes().unwrap());
        }
        println!("{name}: {readings:?}");
        let first = readings.first().copied().unwrap_or(0);
        let second = readings.get(1).copied().unwrap_or(first);
        if second > first || readings.iter().skip(1).any(|&reading| reading != second) {
            grew.push(format!("{name}: {readings:?}"));
        }
    }
    assert!(grew.is_empty(), "{grew:#?}");
}

/// The rounds of [`reflect_charges_are_given_back_on_every_way_out`] in one agent: each script
/// that ends normally answers `undefined` and [`CHARGE_TERMINATING`] ends at the step budget,
/// round after round, with a collection and a printed reading after each round.
fn charge_rounds_in_one_agent(rounds: usize, steps: u64) {
    let mut budget = limits();
    budget.steps_per_entry = steps;
    let mut agent = agent_with(budget).unwrap();
    let realm = realm(&mut agent);
    let mut readings = Vec::new();
    for _ in 0..rounds {
        for (name, script) in CHARGE_SCRIPTS {
            assert_eq!(
                eval_text(&mut agent, &realm, script),
                Outcome::Normal("undefined".to_string()),
                "{name}"
            );
        }
        assert_eq!(
            eval_text(&mut agent, &realm, CHARGE_TERMINATING),
            Outcome::Terminated(Termination::StepBudget)
        );
        collect(&mut agent);
        readings.push(agent.heap_bytes().unwrap());
    }
    println!("heap_bytes after each round in one agent: {readings:?}");
    assert_eq!(agent.state(), AgentState::Alive);
}

/// For the feature `collect-every-safe-point`: values made by getters during
/// CreateListFromArrayLike reach the callee intact, a key made by `toString` survives a getter
/// that allocates, a descriptor's value made by a getter is the one defined, and the index
/// Strings of `ownKeys` and the character of `getOwnPropertyDescriptor` of a String object are
/// whole.
#[test]
fn reflect_keeps_its_values_across_collections() {
    answers(
        r#"var like = { length: 6 };
        for (var i = 0; i < 6; i++) {
          Object.defineProperty(like, i, { get: (function (i) { return function () {
            var junk = []; for (var j = 0; j < 3; j++) junk.push({ j: j });
            return { v: i, s: "x" + i }; }; })(i) });
        }
        function sum() {
          var t = 0, s = "";
          for (var i = 0; i < arguments.length; i++) { t += arguments[i].v; s += arguments[i].s; }
          return t + ":" + s;
        }
        var r = [Reflect.apply(sum, null, like)];
        function K() { this.out = sum.apply(null, arguments); }
        r.push(Reflect.construct(K, like).out);
        var o = { get freshkey() { var junk = []; for (var j = 0; j < 3; j++) junk.push({}); return "got"; } };
        r.push(Reflect.get(o, { toString: function () { return "fresh" + "key"; } }));
        var o2 = {};
        r.push(Reflect.defineProperty(o2, { toString: function () { return "made" + "here"; } },
          { get value() { var junk = []; for (var j = 0; j < 3; j++) junk.push({}); return { w: 1 }; } }));
        r.push(Reflect.ownKeys(o2).join(), o2.madehere.w);
        r.push(Reflect.ownKeys([10, 20, 30, 40]).join("+"));
        r.push(Reflect.getOwnPropertyDescriptor(new String("xyz"), "2").value);
        r.join();"#,
        "15:x0x1x2x3x4x5,15:x0x1x2x3x4x5,got,true,madehere,1,0+1+2+3+length,z",
    );
}

/// Hostile argument lists of `apply` (`#sec-createlistfromarraylike`): "length" is read once,
/// before any index, so a getter that deletes a later element and shrinks "length" leaves the
/// list at its first length with `undefined`; ToLength of a String, of -Infinity, of a fraction
/// and of an object; a String object and a function are array-likes.
#[test]
fn reflect_apply_reads_a_hostile_array_like_by_its_first_length() {
    answers(
        r#"var r = [];
        function show() {
          var s = [];
          for (var i = 0; i < arguments.length; i++) s.push(String(arguments[i]));
          return arguments.length + ":" + s.join("/");
        }
        var like = { length: 3, get 0() { delete like[2]; like.length = 1; return "a"; }, 1: "b", 2: "c" };
        r.push(Reflect.apply(show, null, like));
        r.push(Reflect.apply(show, null, { length: "1e0", 0: "q" }));
        r.push(Reflect.apply(show, null, { length: -Infinity, 0: "q" }));
        r.push(Reflect.apply(show, null, { length: 1.9, 0: "q" }));
        r.push(Reflect.apply(show, null, { length: { valueOf: function () { return 2; } }, 0: "m", 1: "n" }));
        r.push(Reflect.apply(show, null, new String("ab")));
        r.push(Reflect.apply(show, null, function (p, q) {}));
        r.join();"#,
        "3:a/b/undefined,1:q,0:,1:q,2:m/n,2:a/b,2:undefined/undefined",
    );
}
