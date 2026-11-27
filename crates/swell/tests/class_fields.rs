//! Public class fields (`#sec-class-definitions`), instance and static: ClassFieldDefinitionEvaluation
//! (`#sec-runtime-semantics-classfielddefinitionevaluation`, a computed key evaluated once at the
//! class's definition, in source order with the other elements, and kept in a hidden binding of
//! the class scope), DefineField (`#sec-definefield`) in InitializeInstanceElements
//! (`#sec-initializeinstanceelements`) for the instance fields, and the static fields defined in
//! source order after every method (ClassDefinitionEvaluation,
//! `#sec-runtime-semantics-classdefinitionevaluation`, step 31). The initializers run as frames of
//! the run loop: the instance elements' initializer pushed by `InitInstance`, a run of static
//! fields called by `Call(0)` (`docs/design/engine.md`, "Classes", item 4), so the step budget,
//! the interrupt and the frame depth reach them as they reach a call. Written by
//! `story:public-class-fields`. Run also with the feature `collect-every-safe-point`.
//!
//! Each script compares its results with `===` and answers `true`, as written and with a
//! `"use strict";` prologue.
#![cfg(test)]

use std::cell::Cell;

use crate::support;

use support::host::{TestHost, agent, agent_with, limits, realm};
use support::script::{Outcome, define_function, describe, eval_text, parse, run, run_both};
use swell::{
    Agent, Completion, Cx, HostCall, InterruptHandle, Outcome as HostOutcome, Termination, Value,
};

/// What a script that checks its own results answers.
fn passed() -> Outcome {
    Outcome::Normal("true".to_string())
}

/// `source` answers `true` as written and as strict code.
fn both_pass(source: &str) {
    assert_eq!(run_both(source), (passed(), passed()), "{source}");
}

/// DefineField (`#sec-definefield`) in InitializeInstanceElements: every field that is not
/// static becomes an own data property of each new instance, writable, enumerable and
/// configurable, in source order, its initializer evaluated at each construction (or
/// `undefined` without one), before the constructor's body runs; a literal key of every kind
/// (an identifier, a string, a number) and a computed one; an initializer reads the fields
/// defined before it through `this`.
#[test]
fn fields_with_and_without_initializers_in_order() {
    both_pass(
        "var count = 0;\n\
         class C {\n\
           a = ++count;\n\
           b;\n\
           [\"c\" + \"\"] = this.a + 1;\n\
           'd' = this.c * 2;\n\
           2 = \"two\";\n\
           e = this.b;\n\
           constructor() { this.seen = Object.keys(this).join(); }\n\
         }\n\
         var x = new C(), y = new C();\n\
         var d = Object.getOwnPropertyDescriptor(x, \"b\");\n\
         x.seen === \"2,a,b,c,d,e\" && Object.keys(x).join() === \"2,a,b,c,d,e,seen\"\n\
           && x.a === 1 && y.a === 2 && count === 2 && x.c === 2 && x.d === 4 && y.d === 6\n\
           && x[2] === \"two\" && x.hasOwnProperty(\"b\") && x.b === undefined && x.e === undefined\n\
           && d.writable && d.enumerable && d.configurable && d.value === undefined\n\
           && !C.prototype.hasOwnProperty(\"a\") && !C.hasOwnProperty(\"a\")\n\
           && Object.getOwnPropertyNames(C.prototype).join() === \"constructor\";",
    );
}

/// ClassFieldDefinitionEvaluation, step 1, and ClassDefinitionEvaluation, steps 20 to 26: a
/// computed field key runs once, when the class is defined, in source order with the keys of
/// the methods, instance and static mixed, and is converted by ToPropertyKey there, once; the
/// constructions read the kept key and run no key again; a Symbol is a key; a key that throws
/// stops the class definition with the keys after it not run.
#[test]
fn computed_keys_are_evaluated_once_at_definition_with_their_side_effects() {
    both_pass(
        "var log = [];\n\
         function k(label, v) { log.push(label); return v; }\n\
         var conversions = 0;\n\
         var obj = { toString() { conversions++; log.push(\"to string\"); return \"o\"; } };\n\
         var sym = Symbol(\"s\");\n\
         class C {\n\
           [k(\"a\", \"a\")] = log.push(\"init a\");\n\
           [k(\"m\", \"m\")]() {}\n\
           static [k(\"b\", \"b\")] = log.push(\"init b\");\n\
           [k(\"obj\", obj)] = 3;\n\
           [k(\"sym\", sym)] = 4;\n\
           [k(\"c\", \"c\")];\n\
         }\n\
         var defined = log.join();\n\
         var x = new C(), y = new C();\n\
         var constructed = log.join();\n\
         log = [];\n\
         var thrown = \"none\";\n\
         try { class D { [k(\"d\", \"d\")] = 1; [k(\"t\", { toString() { throw new Error(\"key\"); } })] = 2; [k(\"never\", \"n\")] = 3; } }\n\
         catch (e) { thrown = e.message; }\n\
         defined === \"a,m,b,obj,to string,sym,c,init b\" && C.b === 8\n\
           && constructed === defined + \",init a,init a\" && conversions === 1\n\
           && x.a === 9 && y.a === 10 && x.o === 3 && y.o === 3 && x[sym] === 4\n\
           && x.hasOwnProperty(\"c\") && x.c === undefined\n\
           && Object.keys(x).join() === \"a,o,c\" && Object.getOwnPropertySymbols(x)[0] === sym\n\
           && thrown === \"key\" && log.join() === \"d,t\" && typeof D === \"undefined\";",
    );
}

/// The initializer's `this` is the instance (InitializeInstanceElements calls it with the
/// object as receiver), and `new.target` in it is `undefined`
/// (`#sec-runtime-semantics-classfielddefinitionevaluation`: the initializer is a method whose
/// [[Construct]] is never used), also in an arrow function made in it, which keeps both; a
/// static field's `this` is F.
#[test]
fn this_and_new_target_in_an_initializer() {
    both_pass(
        "class C {\n\
           me = this;\n\
           target = new.target;\n\
           arrow = () => [this, new.target];\n\
           static self = this;\n\
           static target = new.target;\n\
         }\n\
         var c = new C(), pair = c.arrow();\n\
         c.me === c && c.target === undefined && pair[0] === c && pair[1] === undefined\n\
           && C.self === C && C.target === undefined && Reflect.construct(C, [], Object).me instanceof Object;",
    );
}

/// `super.x` in an initializer reads through its home object (`docs/design/engine.md`,
/// "Classes", item 5): the prototype for an instance field, so it reads the prototype's
/// [[Prototype]] with `this` the instance as receiver; F for a static field, so it reads F's
/// [[Prototype]], `%Function.prototype%`; `super[k]` with a computed key alike.
#[test]
fn super_property_in_an_initializer() {
    both_pass(
        "var base = { who() { return \"base \" + this.tag; }, plain: 7 };\n\
         class C {\n\
           tag = \"c\";\n\
           who() { return \"own\"; }\n\
           read = super.who();\n\
           plain = super.plain;\n\
           keyed = super[\"pla\" + \"in\"];\n\
           static call = super.call === Function.prototype.call;\n\
           static own = super.hasOwnProperty === Object.prototype.hasOwnProperty;\n\
         }\n\
         Object.setPrototypeOf(C.prototype, base);\n\
         var c = new C();\n\
         c.read === \"base c\" && c.plain === 7 && c.keyed === 7 && C.call === true && C.own === true;",
    );
}

/// NamedEvaluation in ClassFieldDefinitionEvaluation: an anonymous function, arrow function or
/// class as a field's initializer takes the field's name, a literal key's at compile time and a
/// computed key's (a Symbol's description in brackets) at run time; an anonymous class takes the
/// name before its own static elements run, so its static `name` method wins; a named function
/// keeps its own name, and a function in parentheses of a comma expression takes none.
#[test]
fn an_anonymous_function_or_class_takes_the_field_name() {
    both_pass(
        "var sym = Symbol(\"desc\");\n\
         var early = [];\n\
         class C {\n\
           a = function () {};\n\
           b = () => {};\n\
           c = class {};\n\
           [\"d\"] = function () {};\n\
           [sym] = () => {};\n\
           [\"e\" + \"\"] = class { static seen = early.push(this.name); };\n\
           f = function named() {};\n\
           g = (0, function () {});\n\
           [\"h\"] = class { static name() { return \"own\"; } };\n\
           1 = function () {};\n\
           static s = function () {};\n\
           static [\"t\"] = class {};\n\
         }\n\
         var c = new C();\n\
         c.a.name === \"a\" && c.b.name === \"b\" && c.c.name === \"c\" && c.d.name === \"d\"\n\
           && c[sym].name === \"[desc]\" && c.e.name === \"e\" && early.join() === \"e\"\n\
           && c.f.name === \"named\" && c.g.name === \"\" && typeof c.h.name === \"function\"\n\
           && c[1].name === \"1\" && C.s.name === \"s\" && C.t.name === \"t\";",
    );
}

/// DefineField uses CreateDataPropertyOrThrow (`#sec-definefield`, step 6), not [[Set]]: a
/// field whose name has an accessor on the prototype becomes an own data property of the
/// instance and calls no setter; a setter on `Object.prototype` is not called either; a static
/// field over a setter of F's [[Prototype]], `%Function.prototype%`, is an own data property of
/// F.
#[test]
fn a_field_over_a_prototype_getter_is_an_own_data_property() {
    both_pass(
        "var calls = [];\n\
         Object.defineProperty(Object.prototype, \"viaObject\", { set(v) { calls.push(\"object\"); }, configurable: true });\n\
         Object.defineProperty(Function.prototype, \"viaFunction\", { set(v) { calls.push(\"function\"); }, configurable: true });\n\
         class C {\n\
           get x() { return \"getter\"; }\n\
           set x(v) { calls.push(\"setter\"); }\n\
           x = 1;\n\
           viaObject = 2;\n\
           static viaFunction = 3;\n\
         }\n\
         var c = new C();\n\
         var d = Object.getOwnPropertyDescriptor(c, \"x\");\n\
         var s = Object.getOwnPropertyDescriptor(C, \"viaFunction\");\n\
         delete Object.prototype.viaObject;\n\
         delete Function.prototype.viaFunction;\n\
         d.value === 1 && d.writable && d.enumerable && d.configurable && c.x === 1\n\
           && Object.getOwnPropertyDescriptor(c, \"viaObject\").value === 2\n\
           && s.value === 3 && s.writable && s.enumerable && s.configurable\n\
           && calls.length === 0 && C.prototype.hasOwnProperty(\"x\")\n\
           && Object.getOwnPropertyDescriptor(C.prototype, \"x\").get !== undefined;",
    );
}

/// CreateDataPropertyOrThrow fails, and DefineField throws a `TypeError`, where the object
/// refuses the property (`#sec-createdatapropertyorthrow`): a static run that froze F
/// (`static a = Object.freeze(this); static b = 1;`, with a literal and with a computed key)
/// stops the class definition, and a class declaration in a block leaves no binding outside it;
/// an instance made non-extensible in a field's initializer, and an instance whose earlier
/// field defined the name as non-configurable, throw at construction, with the fields before
/// it defined and the initializers after it not run.
#[test]
fn define_field_on_a_frozen_constructor_throws_a_type_error() {
    both_pass(
        "var r = [];\n\
         try { class A { static a = Object.freeze(this); static b = 1; } r.push(\"defined\"); }\n\
         catch (e) { r.push(e instanceof TypeError); }\n\
         var seen = null, late = false;\n\
         class B { first = 1; stop = (seen = this, Object.preventExtensions(this)); late = (late = true); }\n\
         try { new B(); r.push(\"constructed\"); } catch (e) { r.push(e instanceof TypeError); }\n\
         class C { fixed = Object.defineProperty(this, \"later\", { value: 0 }); later = 1; }\n\
         try { new C(); r.push(\"constructed\"); } catch (e) { r.push(e instanceof TypeError); }\n\
         var frozenKey = null;\n\
         try { class F { static [\"a\"] = Object.freeze(this); static b = 1; } } catch (e) { frozenKey = e instanceof TypeError; }\n\
         r.join() === \"true,true,true\" && frozenKey === true && seen.first === 1 && !late\n\
           && Object.keys(seen).join() === \"first\" && typeof A === \"undefined\";",
    );
}

/// The static fields: each is defined on F, writable, enumerable and configurable, in source
/// order, after `length`, `name` and `prototype`; a run's initializers run with `this` F, once,
/// at the definition, and read the fields of the run before them; F's own name is initialized
/// before they run; a construction runs none of them.
#[test]
fn static_fields_and_their_this() {
    both_pass(
        "var runs = 0;\n\
         class C {\n\
           static a = this;\n\
           static b = (runs++, this.a === this);\n\
           static [\"c\"] = C;\n\
           static d;\n\
           x = 1;\n\
         }\n\
         new C(); new C();\n\
         var d = Object.getOwnPropertyDescriptor(C, \"b\");\n\
         C.a === C && C.b === true && C.c === C && runs === 1 && C.hasOwnProperty(\"d\")\n\
           && C.d === undefined && d.writable && d.enumerable && d.configurable\n\
           && Object.getOwnPropertyNames(C).join() === \"length,name,prototype,a,b,c,d\"\n\
           && Object.keys(C).join() === \"a,b,c,d\" && !(\"a\" in new C());",
    );
}

/// ClassDefinitionEvaluation, steps 27 to 31: the static fields run after every element is
/// defined, the methods and accessors written after them included, and after every computed
/// key; static runs split by a method or an instance field run in source order, each a call of
/// its own; a static field may call a static method and read the prototype's methods.
#[test]
fn static_fields_run_after_every_method_is_defined() {
    both_pass(
        "var log = [];\n\
         class C {\n\
           static a = (log.push(\"a\"), this.later());\n\
           static b = (log.push(\"b\"), typeof C.prototype.method);\n\
           [(log.push(\"key 1\"), \"k\")]() {}\n\
           static c = (log.push(\"c\"), this.d);\n\
           x = log.push(\"instance\");\n\
           static d = (log.push(\"d\"), \"d\");\n\
           static later() { return \"later\"; }\n\
           method() {}\n\
           static [(log.push(\"key 2\"), \"e\")] = log.push(\"e\");\n\
           static get g() { return \"g\"; }\n\
           static h = this.g;\n\
         }\n\
         log.join() === \"key 1,key 2,a,b,c,d,e\" && C.a === \"later\" && C.b === \"function\"\n\
           && C.c === undefined && C.d === \"d\" && C.h === \"g\";",
    );
}

/// The step budget of the test: enough for the class definition and a construction, small
/// enough that a loop in the initializer reaches it at once in either mode; 500 with a
/// collection at every safe point, where each step marks the whole heap.
const STEPS: u64 = if cfg!(feature = "collect-every-safe-point") {
    500
} else {
    2_000
};

/// "Hostile input must never hang": an initializer whose code loops for ever ends the entry in
/// `Terminated(StepBudget)`, an instance initializer pushed by `InitInstance` and a static run
/// called by the class definition alike (each is a frame of the run loop, whose instructions
/// are steps); the agent runs the next entry.
#[test]
fn an_initializer_loop_ends_at_the_step_budget() {
    let mut limits = limits();
    limits.steps_per_entry = STEPS;
    for source in [
        "class C { x = (function () { for (;;) {} })(); } new C(); 0;",
        "class C { x = 1; y = (() => { while (true) {} })(); } new C(); 0;",
        "class S { static a = 1; static b = (function () { for (;;) {} })(); } 0;",
    ] {
        let mut agent = agent_with(limits).unwrap();
        let realm = realm(&mut agent);
        assert_eq!(
            eval_text(&mut agent, &realm, source),
            Outcome::Terminated(Termination::StepBudget),
            "{source}"
        );
        assert_eq!(
            eval_text(&mut agent, &realm, "class D { y = 2; } new D().y;"),
            Outcome::Normal("2".to_string()),
            "{source}"
        );
    }
}

thread_local! {
    /// The interrupt handle [`interrupt_now`] sets.
    static HANDLE: Cell<Option<InterruptHandle>> = const { Cell::new(None) };
}

/// A host function that sets its agent's interrupt flag and answers `undefined`.
fn interrupt_now(_cx: &mut Cx<'_, TestHost>, _call: HostCall<'_>) -> Completion<Value> {
    HANDLE.with(|slot| {
        if let Some(handle) = slot.take() {
            handle.interrupt();
        }
    });
    Ok(Value::Undefined)
}

/// Decision 12, the interrupt: an initializer whose loop sets the interrupt flag on its
/// hundredth turn ends the entry in `Terminated(Interrupted)`, an instance initializer and a
/// static run alike; the flag is cleared and the agent runs the next entry.
#[test]
fn an_initializer_loop_ends_at_an_interrupt() {
    for source in [
        "class C { x = (function () { for (var i = 0; ; i++) { if (i === 100) interruptNow(); } })(); }\n\
         new C(); 0;",
        "class S { static a = 1; static b = (() => { for (var i = 0; ; i++) { if (i === 100) interruptNow(); } })(); }\n\
         0;",
    ] {
        let mut agent = agent().unwrap();
        let realm = realm(&mut agent);
        define_function(&mut agent, &realm, "interruptNow", 0, interrupt_now, false);
        HANDLE.with(|slot| slot.set(Some(agent.interrupt_handle())));
        assert_eq!(
            eval_text(&mut agent, &realm, source),
            Outcome::Terminated(Termination::Interrupted),
            "{source}"
        );
        assert_eq!(
            eval_text(&mut agent, &realm, "class D { y = 2; } new D().y;"),
            Outcome::Normal("2".to_string()),
            "{source}"
        );
    }
}

/// The source of a class expression with `n` fields `f0 = 0` to `f<n-1> = <n-1>`.
fn class_with_fields(n: u32) -> String {
    let fields: String = (0..n).map(|i| format!("f{i} = {i}; ")).collect();
    format!("class {{ {fields}}}")
}

/// How many instances each round of [`constructions_of_a_class_with_100_fields_are_charged_and_freed`]
/// constructs: 3,000 under `SWELL_FULL_COUNTS=1` (the story's 10,000 took 6.5 CPU s, over the
/// 2.5 s of a test there; about 0.32 ms a construction) and 500 in the gate; with a
/// collection at every safe point, where each of the about 205 instructions of a construction
/// marks the whole heap (the realm's, about 400 KB), [`COLLECT_FULL`] under
/// `SWELL_FULL_COUNTS=1` (the story's 1,000 take far over the 2.5 s of a test there) and 4 in
/// the gate. Each construction runs the same instructions and the reading after the round must
/// equal the one before it to the byte, so a construction that leaks one byte fails at every
/// count, 4 included.
fn constructions() -> u32 {
    let full = std::env::var("SWELL_FULL_COUNTS").is_ok_and(|value| value == "1");
    match (cfg!(feature = "collect-every-safe-point"), full) {
        (false, true) => 3_000,
        (false, false) => 500,
        (true, true) => COLLECT_FULL,
        (true, false) => 4,
    }
}

/// The constructions of a round under `SWELL_FULL_COUNTS=1` with a collection at every safe
/// point: what fits the 2.5 s of a test there (about 84 ms a construction; 12 took 2.8 s).
const COLLECT_FULL: u32 = 8;

/// The step budget of the rounds: about 210 steps a construction, for the most a round makes.
const ROUND_STEPS: u64 = 5_000_000;

/// How many instances the reading of the fields' charge holds: 100, and 4 with a collection at
/// every safe point (for the time, as [`constructions`]); 4 instances whose fields are not
/// charged read 3,200 bytes short.
const HELD: u64 = if cfg!(feature = "collect-every-safe-point") {
    4
} else {
    100
};

/// The room above a fresh agent and realm that the memory budget leaves: 256 KiB, about a
/// hundred instances of 100 fields.
const ROOM: u64 = 256 * 1024;

/// "Every allocation is charged before it exists" (decision 4, "Memory budget"): a loop of
/// [`constructions`] constructions of a class with 100 fields, each instance read and dropped,
/// leaves, once the script dropped what it held, the reading of `heap_bytes()` after a
/// `collect()` it had before the loop (taken after a first round, which grows the arenas): the
/// initializer's frame, the instance and its 100 properties are credited. [`HELD`] instances
/// held take at least 8 bytes a field more than as many empty objects: the properties the
/// fields define are charged. A loop that keeps instances under a small budget ends in
/// `Terminated(MemoryBudget)`, and leaks nothing (in the default build).
#[test]
fn constructions_of_a_class_with_100_fields_are_charged_and_freed() {
    let mut rounds = limits();
    rounds.steps_per_entry = ROUND_STEPS;
    let mut agent = agent_with(rounds).unwrap();
    let realm = realm(&mut agent);
    let class = class_with_fields(100);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            &format!("var i = 0, o = null, keep = null, C = {class}; 0;")
        ),
        Outcome::Normal("0".to_string())
    );
    let count = constructions();
    let looped = parse(
        &mut agent,
        &realm,
        &format!("for (i = 0; i < {count}; i++) {{ o = new C(); if (o.f99 !== 99) throw 0; }} i;"),
    );
    let dropped = parse(&mut agent, &realm, "o = null; keep = null; 0;");
    let round = |agent: &mut Agent<TestHost>, script: &swell::Script, expected: Outcome| {
        let outcome = agent.evaluate_script(script).unwrap();
        assert_eq!(describe(agent, &realm, outcome), expected);
        let outcome = agent.evaluate_script(&dropped).unwrap();
        assert_eq!(
            describe(agent, &realm, outcome),
            Outcome::Normal("0".to_string())
        );
        assert!(matches!(agent.collect(), Ok(HostOutcome::Normal(_))));
        agent.heap_bytes().unwrap()
    };
    let made = Outcome::Normal(count.to_string());
    let before = round(&mut agent, &looped, made.clone());
    let after = round(&mut agent, &looped, made);
    println!("heap_bytes: {before} before the loop, {after} after it and a collection");
    assert_eq!(after, before);
    // The properties: instances held against empty objects held.
    let instances = parse(
        &mut agent,
        &realm,
        &format!("keep = []; for (i = 0; i < {HELD}; i++) keep.push(new C()); 0;"),
    );
    let objects = parse(
        &mut agent,
        &realm,
        &format!("keep = []; for (i = 0; i < {HELD}; i++) keep.push({{}}); 0;"),
    );
    let held = |agent: &mut Agent<TestHost>, script: &swell::Script| {
        let outcome = agent.evaluate_script(script).unwrap();
        assert!(matches!(outcome, HostOutcome::Normal(_)), "{outcome:?}");
        assert!(matches!(agent.collect(), Ok(HostOutcome::Normal(_))));
        let bytes = agent.heap_bytes().unwrap();
        let outcome = agent.evaluate_script(&dropped).unwrap();
        assert!(matches!(outcome, HostOutcome::Normal(_)), "{outcome:?}");
        assert!(matches!(agent.collect(), Ok(HostOutcome::Normal(_))));
        bytes
    };
    // A first pass of each grows the arenas.
    held(&mut agent, &instances);
    held(&mut agent, &objects);
    let with_instances = held(&mut agent, &instances);
    let with_objects = held(&mut agent, &objects);
    println!(
        "heap_bytes: {with_instances} with {HELD} instances held, {with_objects} with objects"
    );
    assert!(
        with_instances >= with_objects + 100 * 8 * HELD,
        "{with_instances} with instances, {with_objects} with objects"
    );
    // The memory budget: [`ROOM`] above what a fresh agent and realm hold. Not with a
    // collection at every safe point: the loop runs until the heap is full, about a hundred
    // constructions, each marking the heap some 205 times (as `new_target`'s budget test).
    if cfg!(feature = "collect-every-safe-point") {
        return;
    }
    let fresh = {
        let mut agent = support::host::agent().unwrap();
        let _realm = support::host::realm(&mut agent);
        agent.heap_bytes().unwrap()
    };
    let mut small = limits();
    small.memory_bytes = fresh + ROOM;
    let mut agent = agent_with(small).unwrap();
    let small_realm = support::host::realm(&mut agent);
    let kept = parse(
        &mut agent,
        &small_realm,
        &format!("(function () {{ var C = {class}, keep = []; for (;;) keep.push(new C()); }})();"),
    );
    let mut readings = Vec::new();
    for _ in 0..3 {
        let outcome = agent.evaluate_script(&kept).unwrap();
        assert!(
            matches!(outcome, HostOutcome::Terminated(Termination::MemoryBudget)),
            "{outcome:?}"
        );
        assert!(matches!(agent.collect(), Ok(HostOutcome::Normal(_))));
        readings.push(agent.heap_bytes().unwrap());
    }
    println!("heap_bytes after each memory budget and a collection: {readings:?}");
    assert_eq!(readings[1], readings[2]);
}

/// Decision 4, the frame depth: a field initializer that constructs its own class recurses
/// through `InitInstance`, whose frame is checked as a call's (`push_call_frame`), and stops with
/// a `RangeError` that a `catch` sees, an instance field and a static field whose initializer
/// recurses alike; the class works after it. The test's limits; with a collection at every safe
/// point, which marks every frame at each instruction, a frame depth of 100 and a re-entry depth
/// of 8.
#[test]
fn new_in_an_initializer_stops_at_frame_depth() {
    let mut limits = limits();
    if cfg!(feature = "collect-every-safe-point") {
        limits.frame_depth = 100;
        limits.reentry_depth = 8;
    }
    let source = "class A { x = new A(); }\n\
         var B = class { static f() { return new B(); } y = B.f(); };\n\
         function range(f) { try { f(); } catch (e) { return e instanceof RangeError; } return false; }\n\
         var s = range(function () { class S { static a = (function g() { return g(); })(); } });\n\
         range(function () { new A(); }) && range(function () { new B(); }) && s\n\
           && new (class { z = 3; })().z === 3;";
    assert_eq!(run(limits, source), passed(), "{source}");
    let strict = format!("\"use strict\";\n{source}");
    assert_eq!(run(limits, &strict), passed(), "{strict}");
}

/// A computed key made at run time (a String or a Symbol made by the key's expression) lives in
/// its hidden binding from the class's definition to every construction: three classes made in
/// a loop, garbage made between, then each constructed; with a collection at every safe point a
/// key the collector does not reach from its binding is freed before the construction reads it.
#[test]
fn a_computed_key_made_at_run_time_lives_until_the_constructions() {
    both_pass(
        "var cs = [];\n\
         for (let i = 0; i < 3; i++)\n\
           cs.push(class { ['k' + i] = i; [Symbol('s' + i)] = i; static ['s' + i] = i; });\n\
         var junk = [];\n\
         for (var j = 0; j < 20; j++) junk.push({ j: j });\n\
         cs.map(function (C, i) {\n\
           var o = new C();\n\
           return Object.keys(o).join() + Object.getOwnPropertySymbols(o)[0].description + C['s' + i];\n\
         }).join() === 'k0s00,k1s11,k2s22';",
    );
}

/// The error site of a field (`Agent::last_entry_error_site`): the `TypeError` of a
/// DefineField that the instance refuses is at the field (line 3, column 3), and the
/// `TypeError` of a computed key's ToPropertyKey is at the key (line 3, column 4, after `[`).
#[test]
fn a_field_error_is_reported_at_the_field_and_at_its_key() {
    for (source, site) in [
        (
            "class C {\n  a = 1;\n  b = Object.preventExtensions(this) && 0;\n  c = 2;\n}\nnew C();",
            (3, 3),
        ),
        (
            "class C {\n  a = 1;\n  [Object.create(null)] = 2;\n}\n0;",
            (3, 4),
        ),
    ] {
        let mut agent = agent().unwrap();
        let realm = realm(&mut agent);
        let script = parse(&mut agent, &realm, source);
        let outcome = agent.evaluate_script(&script).unwrap();
        let found = agent
            .last_entry_error_site()
            .unwrap()
            .map(|s| (s.line, s.column));
        assert_eq!(
            describe(&mut agent, &realm, outcome),
            Outcome::Throw("TypeError".to_string()),
            "{source}"
        );
        assert_eq!(found, Some(site), "{source}");
    }
}

/// A field named `__proto__` by a literal key is DefineField like any other: an own data
/// property of the instance (or of the class, for a static field), the prototype unchanged; not
/// the `__proto__: v` of an object literal.
#[test]
fn a_literal_proto_field_is_an_own_property() {
    both_pass(
        "class C { __proto__ = 5; }\n\
         class S { static __proto__ = Array.prototype; }\n\
         var c = new C();\n\
         Object.getPrototypeOf(c) === C.prototype && Object.getOwnPropertyDescriptor(c, '__proto__').value === 5\n\
           && Object.getPrototypeOf(S) === Function.prototype\n\
           && Object.getOwnPropertyDescriptor(S, '__proto__').value === Array.prototype;",
    );
}
