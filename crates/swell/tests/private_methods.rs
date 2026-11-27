//! Private methods and accessors (`#sec-private-names`, `#sec-privateelement-specification-type`):
//! the Private Name each evaluation of a class makes per private identifier, [[PrivateElements]]
//! of an object, PrivateMethodOrAccessorAdd (`#sec-privatemethodoraccessoradd`) of an instance's
//! methods by its class's initializer and of the static ones on F, PrivateElementFind, PrivateGet
//! and PrivateSet (`#sec-privateelementfind`, `#sec-privateget`, `#sec-privateset`), and every
//! form of a private reference as an assignment target. Written by
//! `story:private-names-and-private-methods`. Run also with the feature
//! `collect-every-safe-point`.
//!
//! Each script answers `true`, or the empty string when it lists what went wrong, as written and
//! with a `"use strict";` prologue (class code is strict either way).
#![cfg(test)]

use crate::support;

use support::host::{TestHost, agent, agent_with, get_object, global, key, limits, object, realm};
use support::script::{Outcome, describe, eval, eval_text, parse, run, run_both};
use swell::{Agent, Outcome as HostOutcome, Termination, Value};

/// What a script that checks its own results answers.
fn passed() -> Outcome {
    Outcome::Normal("true".to_string())
}

/// `source` answers `true` as written and as strict code.
fn both_pass(source: &str) {
    assert_eq!(run_both(source), (passed(), passed()), "{source}");
}

/// `source` answers the empty string, the list of what went wrong, as written and as strict
/// code.
fn both_list_nothing(source: &str) {
    let nothing = Outcome::Normal("\"\"".to_string());
    assert_eq!(run_both(source), (nothing.clone(), nothing), "{source}");
}

/// The script helper `te(f)`: whether `f` throws a `TypeError`.
const TE: &str =
    "function te(f) { try { f(); } catch (e) { return e instanceof TypeError; } return false; }\n";

/// ClassDefinitionEvaluation and PrivateMethodOrAccessorAdd: a method, a getter, a setter and an
/// accessor pair of an instance are reached through `this`, each with its own function: the
/// method is not a constructor, has no `prototype`, is named `#m`
/// (`#sec-definemethodproperty`, SetFunctionName with the Private Name's description) and its
/// `toString` is its source text from its first token.
#[test]
fn a_private_method_getter_setter_and_accessor_pair() {
    both_pass(&format!(
        "{TE}\
         class C {{\n\
           #m(a, b) {{ return 1; }}\n\
           get #g() {{ return 2; }}\n\
           set #s(v) {{ this.sv = v; }}\n\
           get #p() {{ return this.pv; }}\n\
           set #p(v) {{ this.pv = v * 2; }}\n\
           run() {{ this.#s = 5; this.#p = 3; return this.#m() + this.#g + this.sv + this.#p; }}\n\
           method() {{ return this.#m; }}\n\
         }}\n\
         var c = new C(), m = c.method();\n\
         c.run() === 14 && typeof m === \"function\" && m.name === \"#m\" && m.length === 2\n\
           && !m.hasOwnProperty(\"prototype\") && te(function () {{ new m(); }})\n\
           && m.toString() === \"#m(a, b) {{ return 1; }}\" && c.pv === 6\n\
           && Object.getOwnPropertyNames(C.prototype).join() === \"constructor,run,method\";"
    ));
}

/// ClassDefinitionEvaluation, step 4: the static private methods and accessors are added to F,
/// with F as their `this` and their [[HomeObject]]; an instance has none of them; the source
/// text of a static private method is its MethodDefinition, without `static`.
#[test]
fn static_private_methods_and_accessors() {
    both_list_nothing(
        "class C {\n\
           static #m() { return this; }\n\
           static get #g() { return \"g\"; }\n\
           static set #g(v) { C.log = v; }\n\
           static #h() { return super.toString === Function.prototype.toString; }\n\
           static run() { C.#g = 4; return [C.#m() === C, C.#g === \"g\", C.log === 4, C.#h()]; }\n\
           static has(o) { try { o.#m; return true; } catch (e) { return e instanceof TypeError ? false : \"other\"; } }\n\
           static source() { return C.#m.toString(); }\n\
         }\n\
         var checks = C.run().concat([C.has(C) === true, C.has(new C()) === false,\n\
           C.source() === \"#m() { return this; }\",\n\
           Object.getOwnPropertyNames(C).join() === \"length,name,prototype,run,has,source,log\"]);\n\
         var names = [\"this\", \"get\", \"set\", \"super\", \"F has\", \"instance lacks\", \"source\", \"own names\"];\n\
         names.filter(function (n, i) { return checks[i] !== true; }).join();",
    );
}

/// `obj.#m()` calls the method with `obj` as `this` (`#sec-function-calls-runtime-semantics-evaluation`,
/// EvaluateCall with a Reference whose base is `obj`): through parentheses, an optional chain,
/// a tagged template and with arguments; a call of the bare value has `this` undefined; an
/// optional chain on `null` short-circuits.
#[test]
fn this_private_method_call_gets_this() {
    both_pass(
        "class C {\n\
           #m() { return this; }\n\
           #n(a, b) { return [this, a, b]; }\n\
           #t(strings) { return [this, strings[0]]; }\n\
           run() {\n\
             var o = this, none = null, r = this.#n(1, 2), t = this.#t`x`;\n\
             return this.#m() === this && r[0] === this && r[1] === 1 && r[2] === 2\n\
               && (this.#m)() === this && (0, this.#m)() === undefined\n\
               && o?.#m() === o && none?.#m() === undefined && none?.#m.x.y === undefined\n\
               && this.#m.call(5) === 5 && t[0] === this && t[1] === \"x\"\n\
               && { o }.o.#m() === o;\n\
           }\n\
         }\n\
         new C().run();",
    );
}

/// PrivateElementFind finds nothing on an object that the class did not initialize: PrivateGet
/// and PrivateSet throw a `TypeError` (`#sec-privateget`, step 2; `#sec-privateset`, step 2),
/// for a plain object, an object whose prototype is an instance, F itself, and a `Proxy` of an
/// instance (no trap sees a private name).
#[test]
fn access_without_the_brand_throws_a_type_error() {
    both_pass(&format!(
        "{TE}\
         class C {{\n\
           #m() {{ return 1; }}\n\
           get #g() {{ return 1; }}\n\
           set #s(v) {{}}\n\
           static get(o) {{ return o.#m; }}\n\
           static getg(o) {{ return o.#g; }}\n\
           static set(o) {{ o.#s = 1; }}\n\
           static call(o) {{ return o.#m(); }}\n\
         }}\n\
         var c = new C(), traps = [];\n\
         var p = new Proxy(c, {{ get() {{ traps.push(\"get\"); }}, set() {{ traps.push(\"set\"); }} }});\n\
         te(function () {{ C.get({{}}); }}) && te(function () {{ C.getg({{}}); }})\n\
           && te(function () {{ C.set({{}}); }}) && te(function () {{ C.call(C); }})\n\
           && te(function () {{ C.get(Object.create(c)); }}) && te(function () {{ C.call(p); }})\n\
           && te(function () {{ C.set(p); }}) && traps.length === 0 && C.call(c) === 1;"
    ));
}

/// GetValue and PutValue of a private reference convert the base with ToObject (`#sec-getvalue`,
/// step 3.a; `#sec-putvalue`, step 3.a): `undefined` and `null` throw there, and the wrapper of
/// any other primitive has no private element, so PrivateGet or PrivateSet throws; all of them
/// are `TypeError`s.
#[test]
fn access_on_a_primitive_throws_a_type_error() {
    both_pass(&format!(
        "{TE}\
         class C {{\n\
           #m() {{ return 1; }}\n\
           set #s(v) {{}}\n\
           static get(o) {{ return o.#m; }}\n\
           static set(o) {{ o.#s = 1; }}\n\
           static call(o) {{ return o.#m(); }}\n\
         }}\n\
         [1, \"s\", true, Symbol(), undefined, null].every(function (v) {{\n\
           return te(function () {{ C.get(v); }}) && te(function () {{ C.set(v); }})\n\
             && te(function () {{ C.call(v); }});\n\
         }});"
    ));
}

/// PrivateSet, step 5: a private method is not writable; setting it throws a `TypeError` and
/// leaves it as it was; so does setting an accessor that has no setter (step 6.b).
#[test]
fn setting_a_private_method_throws_a_type_error() {
    both_pass(&format!(
        "{TE}\
         class C {{\n\
           #m() {{ return 1; }}\n\
           get #g() {{ return 2; }}\n\
           run() {{\n\
             var o = this, m = this.#m;\n\
             return te(function () {{ o.#m = 5; }}) && o.#m === m && o.#m() === 1\n\
               && te(function () {{ o.#g = 5; }}) && o.#g === 2;\n\
           }}\n\
         }}\n\
         new C().run();"
    ));
}

/// PrivateGet, step 5: an accessor with a setter and no getter throws a `TypeError` when read;
/// a compound assignment reads first, so its setter is not called.
#[test]
fn getting_a_setter_only_accessor_throws_a_type_error() {
    both_pass(&format!(
        "{TE}\
         var calls = 0;\n\
         class C {{\n\
           set #s(v) {{ calls++; }}\n\
           run() {{\n\
             var o = this;\n\
             return te(function () {{ return o.#s; }}) && te(function () {{ o.#s += 1; }})\n\
               && te(function () {{ o.#s++; }}) && calls === 0 && (o.#s = 3) === 3 && calls === 1;\n\
           }}\n\
         }}\n\
         new C().run();"
    ));
}

/// ClassDefinitionEvaluation makes one function per private method per evaluation, and
/// PrivateMethodOrAccessorAdd adds that one to every instance: two instances read the same
/// function, and a property set on it is seen through both.
#[test]
fn two_instances_share_one_method_function() {
    both_pass(
        "class C {\n\
           #m() { return 1; }\n\
           static #s() {}\n\
           get() { return this.#m; }\n\
           static s() { return C.#s; }\n\
         }\n\
         var a = new C(), b = new C();\n\
         a.get().mark = 7;\n\
         a.get() === b.get() && b.get().mark === 7 && a.get() !== C.s() && C.s() === C.s();",
    );
}

/// `#sec-private-names`: each evaluation of a class makes new Private Names, so an instance of
/// one evaluation has no element of the other's, and the two make different functions.
#[test]
fn two_evaluations_of_a_class_make_two_brands() {
    both_pass(
        "function make() {\n\
           return class {\n\
             #m() { return 1; }\n\
             static has(o) { try { o.#m; return true; } catch (e) { return false; } }\n\
             static fn(o) { return o.#m; }\n\
           };\n\
         }\n\
         var A = make(), B = make(), a = new A(), b = new B();\n\
         A.has(a) && B.has(b) && !A.has(b) && !B.has(a) && A.fn(a) !== B.fn(b)\n\
           && A.fn(a) === A.fn(new A());",
    );
}

/// ResolvePrivateIdentifier (`#sec-resolve-private-identifier`): an inner class that declares
/// `#m` hides the outer class's `#m` in its body; a name only the outer class declares is
/// reached from the inner class's methods and computed keys.
#[test]
fn an_inner_class_name_shadows_an_outer_one() {
    both_pass(
        "class Outer {\n\
           #m() { return \"outer\"; }\n\
           #only() { return \"only\"; }\n\
           static test(o) {\n\
             class Inner {\n\
               #m() { return \"inner\"; }\n\
               [o.#only()]() { return o.#only(); }\n\
               static call(x) { return x.#m(); }\n\
             }\n\
             var i = new Inner(), threw = false;\n\
             try { Inner.call(o); } catch (e) { threw = e instanceof TypeError; }\n\
             return Inner.call(i) === \"inner\" && threw && o.#m() === \"outer\"\n\
               && i.only() === \"only\";\n\
           }\n\
         }\n\
         Outer.test(new Outer());",
    );
}

/// "Private elements are no properties": no listing of a script sees them
/// (`Object.getOwnPropertyNames`, `Object.getOwnPropertySymbols`, `Reflect.ownKeys`,
/// `Object.keys`, `JSON.stringify`, `for`-`in`), `Object.assign` copies none, and
/// `Object.freeze` leaves them working. The host interface has no listing of an object's keys:
/// the host's view is `has_property` and `get_property` of the name `#m`, which find nothing.
#[test]
fn private_elements_are_invisible_to_listings() {
    let source = "class C {\n\
           #m() { return 1; }\n\
           get #a() { return 2; }\n\
           set #a(v) {}\n\
           static #s() {}\n\
           static has(o) { try { o.#m; return true; } catch (e) { return false; } }\n\
           static read(o) { return o.#m() + o.#a; }\n\
         }\n\
         var c = new C(); c.x = 1;\n\
         var copy = Object.assign({}, c), keys = [];\n\
         for (var k in c) keys.push(k);\n\
         Object.freeze(c);\n\
         Object.getOwnPropertyNames(c).join() === \"x\" && Object.getOwnPropertySymbols(c).length === 0\n\
           && Reflect.ownKeys(c).join() === \"x\" && Object.keys(c).join() === \"x\"\n\
           && JSON.stringify(c) === '{\"x\":1}' && keys.join() === \"x\"\n\
           && !C.has(copy) && Object.keys(copy).join() === \"x\"\n\
           && Object.isFrozen(c) && C.has(c) && C.read(c) === 3\n\
           && Object.getOwnPropertyNames(C).join() === \"length,name,prototype,has,read\";";
    both_pass(source);
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    assert_eq!(eval_text(&mut agent, &realm, source), passed());
    let global_root = global(&mut agent, &realm);
    let (has, value) = agent
        .with_cx(&realm, |cx| {
            let global = object(cx, &global_root);
            let c = get_object(cx, global, "c");
            let name = key(cx, "#m");
            let has = cx.has_property(c, name).unwrap().unwrap();
            let value = cx.get_property(c, name).unwrap().unwrap();
            (has, value)
        })
        .unwrap();
    assert!(!has);
    assert_eq!(value, Value::Undefined);
}

/// How many class evaluations each of the two rounds of
/// [`private_names_are_freed_when_nothing_holds_them`] makes: 5,000 with `SWELL_FULL_COUNTS=1`,
/// the story's 10,000 over the two rounds (two rounds of 10,000 took 2.4 to 3.1 CPU s alone on
/// the build node, over the 2.5 s bound), 1,000 in the gate (for the time bound, as
/// `classes::classes`), and 20 with a collection at every safe point, which marks the whole heap
/// at each instruction (the story's 1,000 ran far over the 2 s bound there, as 1,000 classes did
/// in `classes`). Each iteration runs the same instructions, so a Private Name or an element list
/// that leaks one byte fails at every count.
fn evaluations() -> u32 {
    if cfg!(feature = "collect-every-safe-point") {
        20
    } else if std::env::var("SWELL_FULL_COUNTS").is_ok_and(|value| value == "1") {
        5_000
    } else {
        1_000
    }
}

/// A Private Name lives while a record, a frame or an object's elements hold it (design,
/// "Private names", item 6): a loop of [`evaluations`] classes with private methods, accessors and
/// a static method, each with an instance that calls them, leaves, once the script dropped what
/// it held, the reading of `heap_bytes()` after a `collect()` it had before the loop (taken after
/// a first round, which grows the arenas).
#[test]
fn private_names_are_freed_when_nothing_holds_them() {
    // About 100 steps an iteration or more: the full count's iterations come near the suite's
    // limit of 1,000,000 steps an entry.
    let mut limits = limits();
    limits.steps_per_entry = 4_000_000;
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(&mut agent, &realm, "var i = 0, k = null, o = null; 0;"),
        Outcome::Normal("0".to_string())
    );
    let count = evaluations();
    let looped = parse(
        &mut agent,
        &realm,
        &format!(
            "for (i = 0; i < {count}; i++) {{\n\
               k = class {{ #m() {{ return 1; }} get #a() {{ return 2; }} set #a(v) {{}}\n\
                 static #s() {{ return 3; }} run() {{ this.#a = 0; return this.#m() + this.#a + k.#s(); }} }};\n\
               o = new k(); if (o.run() !== 6) throw 0;\n\
             }} i;"
        ),
    );
    let dropped = parse(&mut agent, &realm, "k = null; o = null; 0;");
    let round = |agent: &mut Agent<TestHost>| {
        let outcome = agent.evaluate_script(&looped).unwrap();
        assert_eq!(
            describe(agent, &realm, outcome),
            Outcome::Normal(count.to_string())
        );
        let outcome = agent.evaluate_script(&dropped).unwrap();
        assert_eq!(
            describe(agent, &realm, outcome),
            Outcome::Normal("0".to_string())
        );
        assert!(matches!(agent.collect(), Ok(HostOutcome::Normal(_))));
        agent.heap_bytes().unwrap()
    };
    let before = round(&mut agent);
    let after = round(&mut agent);
    println!("heap_bytes: {before} before the loop, {after} after it and a collection");
    assert_eq!(after, before);
}

/// How many classes each round of [`an_instance_keeps_its_private_elements_across_collections`]
/// makes and drops: 50, and 5 with a collection at every safe point (for the time): garbage
/// whose records a collection that freed what the instance holds would hand out again.
const GARBAGE: u32 = if cfg!(feature = "collect-every-safe-point") {
    5
} else {
    50
};

/// An instance held by a global keeps working across collections while its class is no longer
/// reachable by name. Its methods hold the class's private record, which holds the same Private
/// Names and functions as the instance's elements, so this test stays green when the object
/// record does not trace its elements; the mark from the object alone is held by the library
/// test `object::private_names::tests::a_mark_from_the_object_reaches_every_element`.
#[test]
fn an_instance_keeps_its_private_elements_across_collections() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var o = (function () {\n\
               class C { #m() { return 7; } get #a() { return this.v; } set #a(v) { this.v = v; }\n\
                 run() { this.#a = 8; return this.#m() + this.#a; } }\n\
               return new C();\n\
             })(); 0;"
        ),
        Outcome::Normal("0".to_string())
    );
    for _ in 0..3 {
        assert!(matches!(agent.collect(), Ok(HostOutcome::Normal(_))));
        assert_eq!(
            eval_text(
                &mut agent,
                &realm,
                &format!(
                    "var garbage = []; for (var j = 0; j < {GARBAGE}; j++) garbage.push(class {{ #x() {{}} }});\n\
                     garbage = null; o.run();"
                )
            ),
            Outcome::Normal("15".to_string())
        );
    }
}

/// How many private methods [`a_class_with_1000_private_methods`] defines: 1,000, and 40 with a
/// collection at every safe point (for the time, as `classes::METHODS`).
const METHODS: u32 = if cfg!(feature = "collect-every-safe-point") {
    40
} else {
    1_000
};

/// A class with [`METHODS`] private methods: each made once, added to the instance in source
/// order and callable. Each add and each access finds its entry by comparing in order, a step per
/// comparison past `SMALL` (8), so the adds and the calls each spend about `METHODS`² / 2 steps:
/// the test gives the entry 4,000,000 steps, where the suite's limit is 1,000,000.
#[test]
fn a_class_with_1000_private_methods() {
    let n = METHODS;
    let mut source = String::from("class C {\n");
    for i in 0..n {
        source.push_str(&format!("  #m{i}() {{ return {i}; }}\n"));
    }
    source.push_str("  sum() { var s = 0;\n");
    for i in 0..n {
        source.push_str(&format!("    s += this.#m{i}();\n"));
    }
    let last = n - 1;
    let sum = n * last / 2;
    source.push_str(&format!(
        "    return s; }}\n\
           last() {{ return this.#m{last}; }}\n\
         }}\n\
         var c = new C();\n\
         c.sum() === {sum} && c.last().name === \"#m{last}\"\n\
           && Object.getOwnPropertyNames(C.prototype).join() === \"constructor,sum,last\";"
    ));
    let mut limits = limits();
    limits.steps_per_entry = 4_000_000;
    assert_eq!(run(limits, &source), passed(), "{source}");
    let strict = format!("\"use strict\";\n{source}");
    assert_eq!(run(limits, &strict), passed(), "{strict}");
}

/// How many instances [`instances_are_charged_to_the_memory_budget`] holds: the story's 10,000
/// with `SWELL_FULL_COUNTS=1`, 2,000 in the gate (10,000 took 1.6 to 2.8 CPU s alone on the build
/// node, over the 1.5 s bound), and 3 with a collection at every safe point, which marks the
/// whole heap at each of the loop's instructions (the story's 1,000 ran over 60 s there, 50 took
/// 4.4 CPU s; with the three passes the test reads, 10 took 2.1 CPU s and 5 took 1.9). Each
/// instance adds the same two entries, so a list that is not charged fails at every count.
fn instances() -> u64 {
    if cfg!(feature = "collect-every-safe-point") {
        3
    } else if std::env::var("SWELL_FULL_COUNTS").is_ok_and(|value| value == "1") {
        10_000
    } else {
        2_000
    }
}

/// The room above a fresh agent and realm that the memory budget of
/// [`instances_are_charged_to_the_memory_budget`] leaves: 256 KiB, a few thousand instances, and
/// 20 KiB with a collection at every safe point (for the time: 32 KiB took 0.8 s a round there;
/// 16 KiB, `classes::ROOM`, does not hold the parse of the script).
const ROOM: u64 = if cfg!(feature = "collect-every-safe-point") {
    20 * 1024
} else {
    256 * 1024
};

/// "Every allocation is charged before it exists" (decision 4, "Memory budget"): an instance with
/// private methods holds its element list, charged with the object, so [`instances`] instances
/// with two private methods take at least 16 bytes each more than as many instances of a class
/// without (the list's charge: without it the two readings are equal); a loop that keeps such
/// instances under a small budget, four per run of the loop (for the time), ends in
/// `Terminated(MemoryBudget)`, and leaks nothing.
#[test]
fn instances_are_charged_to_the_memory_budget() {
    let count = instances();
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var keep = null, i = 0;\n\
             class P { #m() {} #n() {} }\n\
             class Q { m() {} n() {} }\n\
             0;"
        ),
        Outcome::Normal("0".to_string())
    );
    let private = parse(
        &mut agent,
        &realm,
        &format!("keep = []; for (i = 0; i < {count}; i++) keep.push(new P()); 0;"),
    );
    let plain = parse(
        &mut agent,
        &realm,
        &format!("keep = []; for (i = 0; i < {count}; i++) keep.push(new Q()); 0;"),
    );
    let dropped = parse(&mut agent, &realm, "keep = null; 0;");
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
    // A first pass grows the arenas to what either script holds: the instances are as many.
    // The plain instances are read first: were the lists not charged, the sweep of the first
    // pass would credit bytes that were never charged, and the two readings after it would be
    // equal, which the assertion below refuses.
    held(&mut agent, &private);
    let with_plain = held(&mut agent, &plain);
    let with_private = held(&mut agent, &private);
    println!(
        "heap_bytes: {with_private} with {count} instances with private methods, {with_plain} without"
    );
    assert!(
        with_private >= with_plain + 16 * count,
        "{with_private} with private methods, {with_plain} without"
    );
    // The memory budget: [`ROOM`] above what a fresh agent and realm hold.
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
        "(function () { class P { #m() {} get #a() { return 1; } }\n\
           var keep = []; for (;;) keep.push(new P(), new P(), new P(), new P()); })();",
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

/// A private method, a static private method, a private getter and a private setter that recurse
/// without end stop at the frame depth or the re-entry depth with a `RangeError` (decision 4),
/// which a `catch` sees; the class works after it. The limits with a collection at every safe
/// point are those of `classes::a_method_recursion_stops_at_frame_depth`, for the time.
#[test]
fn a_private_method_recursion_stops_at_frame_depth() {
    let mut limits = limits();
    if cfg!(feature = "collect-every-safe-point") {
        limits.frame_depth = 100;
        limits.reentry_depth = 8;
        limits.steps_per_entry = 5_000;
    }
    let source = "class A {\n\
           #m(n) { return this.#m(n + 1); }\n\
           static #s(n) { return A.#s(n + 1); }\n\
           get #g() { return this.#g; }\n\
           set #t(v) { this.#t = v; }\n\
           m() { return this.#m(0); } static s() { return A.#s(0); }\n\
           g() { return this.#g; } t() { this.#t = 1; }\n\
         }\n\
         function range(f) { try { f(); } catch (e) { return e instanceof RangeError; } return false; }\n\
         range(function () { new A().m(); }) && range(function () { A.s(); })\n\
           && range(function () { new A().g(); }) && range(function () { new A().t(); })\n\
           && new A() instanceof A;";
    assert_eq!(run(limits, source), passed(), "{source}");
    let strict = format!("\"use strict\";\n{source}");
    assert_eq!(run(limits, &strict), passed(), "{strict}");
}

/// How many accesses each script of [`a_lookup_past_small_spends_a_step_per_comparison`] makes:
/// 100, and 10 with a collection at every safe point (for the time); a find that spends no step
/// per comparison, or one per entry, fails at both counts.
const ACCESSES: u64 = if cfg!(feature = "collect-every-safe-point") {
    10
} else {
    100
};

/// PrivateElementFind compares the entries in order and spends a step per comparison past
/// `SMALL` (8): reading the ninth entry spends one step more than reading the eighth, and the
/// fortieth 32 more than the first, per access, as `last_entry_steps()` reads it.
#[test]
fn a_lookup_past_small_spends_a_step_per_comparison() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let mut class = String::from("var o = new (class {\n");
    for i in 0..40 {
        class.push_str(&format!("  #m{i}() {{}}\n"));
    }
    for i in [0, 7, 8, 39] {
        class.push_str(&format!(
            "  read{i}() {{ for (var j = 0; j < {ACCESSES}; j++) this.#m{i}; return j; }}\n"
        ));
    }
    class.push_str("})(); 0;");
    assert_eq!(
        eval_text(&mut agent, &realm, &class),
        Outcome::Normal("0".to_string())
    );
    let mut steps = |i: u32| {
        let source = format!("if (o.read{i}() !== {ACCESSES}) throw 0;");
        let outcome = eval(&mut agent, &realm, &source);
        assert!(matches!(outcome, HostOutcome::Normal(_)), "{source}");
        agent.last_entry_steps().unwrap()
    };
    let (first, eighth, ninth, fortieth) = (steps(0), steps(7), steps(8), steps(39));
    println!("last_entry_steps: {first}, {eighth}, {ninth}, {fortieth} for entries 1, 8, 9, 40");
    assert_eq!(eighth, first);
    assert_eq!(ninth, eighth + ACCESSES);
    assert_eq!(fortieth, first + 32 * ACCESSES);
}

/// The test of `Target::Private`: every form of assignment on a private reference, on an
/// accessor with a getter and a setter (the setter gets the value, the getter is read where the
/// form reads), on a private method, and on an accessor with a getter and no setter, where
/// PrivateSet throws a `TypeError` (`#sec-privateset`, steps 5 and 6.b) at the point the clause
/// of the form puts PutValue, read by the log of the side effects of the reference
/// (`ref`), the right-hand side (`rhs`), the getter (`get`), the setter (`set v`), ToPrimitive of
/// the method (`valueOf`), and the iterator or the source object (`next`, `return`, `x`):
/// `=`, `+=`, `&&=`, `||=`, `??=` (`#sec-assignment-operators-runtime-semantics-evaluation`; on
/// a method, whose value is a function, `||=` and `??=` short-circuit and throw nothing),
/// prefix and postfix `++` and `--` (`#sec-prefix-increment-operator-runtime-semantics-evaluation`
/// and its twins), a target of an array and of an object pattern
/// (`#sec-runtime-semantics-iteratordestructuringassignmentevaluation`,
/// `#sec-runtime-semantics-keyeddestructuringassignmentevaluation`; the array form closes its
/// iterator), and a `for`-`of` and a `for`-`in` target
/// (`#sec-runtime-semantics-forin-div-ofbodyevaluation-lhs-stmt-iterator-lhskind-labelset`;
/// `for`-`of` closes its iterator). The script answers the list of the forms whose log, value or
/// throw differs from the expected one.
#[test]
fn every_assignment_form_on_a_private_accessor_and_a_private_method() {
    // Every form stands in class code, which is strict either way; with a collection at every
    // safe point the script runs as written only (both forms took 3.4 CPU s there, over the 2 s
    // bound).
    let source = "var log = [], failures = [];\n\
         function rhs(v) { log.push(\"rhs\"); return v; }\n\
         function iter(v) {\n\
           var it = { next() { log.push(\"next\"); return { value: v, done: false }; },\n\
                      return() { log.push(\"return\"); return {}; } };\n\
           return { [Symbol.iterator]() { return it; } };\n\
         }\n\
         function src(v) { return { get x() { log.push(\"x\"); return v; } }; }\n\
         function show(r) { return typeof r === \"function\" ? \"fn\" : String(r); }\n\
         function check(name, f, expected) {\n\
           log = [];\n\
           try { log.push(\"=\" + show(f())); } catch (e) { log.push(e instanceof TypeError ? \"TypeError\" : \"other \" + e); }\n\
           if (log.join() !== expected) failures.push(name + \": \" + log.join());\n\
         }\n\
         class C {\n\
           get #a() { log.push(\"get\"); return this.store; }\n\
           set #a(v) { log.push(\"set \" + v); this.store = v; }\n\
           #m() { return \"m\"; }\n\
           get #g() { log.push(\"get\"); return this.gv; }\n\
           static run(o) {\n\
             function ref() { log.push(\"ref\"); return o; }\n\
             function a(v) { o.store = v; return true; }\n\
             function g(v) { o.gv = v; return true; }\n\
             o.#m.valueOf = function () { log.push(\"valueOf\"); return 1; };\n\
             a(1); check(\"a =\", () => ref().#a = rhs(5), \"ref,rhs,set 5,=5\");\n\
             a(1); check(\"a +=\", () => ref().#a += rhs(2), \"ref,get,rhs,set 3,=3\");\n\
             a(1); check(\"a &&=\", () => ref().#a &&= rhs(4), \"ref,get,rhs,set 4,=4\");\n\
             a(0); check(\"a ||=\", () => ref().#a ||= rhs(4), \"ref,get,rhs,set 4,=4\");\n\
             a(1); check(\"a ||= short\", () => ref().#a ||= rhs(4), \"ref,get,=1\");\n\
             a(null); check(\"a ??=\", () => ref().#a ??= rhs(4), \"ref,get,rhs,set 4,=4\");\n\
             a(1); check(\"a ++x\", () => ++ref().#a, \"ref,get,set 2,=2\");\n\
             a(1); check(\"a x++\", () => ref().#a++, \"ref,get,set 2,=1\");\n\
             a(1); check(\"a --x\", () => --ref().#a, \"ref,get,set 0,=0\");\n\
             a(1); check(\"a x--\", () => ref().#a--, \"ref,get,set 0,=1\");\n\
             check(\"a [] =\", () => { [ref().#a] = iter(7); return o.store; }, \"ref,next,set 7,return,=7\");\n\
             check(\"a {} =\", () => { ({ x: ref().#a } = src(8)); return o.store; }, \"ref,x,set 8,=8\");\n\
             check(\"a for of\", () => { for (ref().#a of iter(9)) break; return o.store; }, \"next,ref,set 9,return,=9\");\n\
             check(\"a for in\", () => { for (ref().#a in { k: 1 }); return o.store; }, \"ref,set k,=k\");\n\
             check(\"m =\", () => ref().#m = rhs(5), \"ref,rhs,TypeError\");\n\
             check(\"m +=\", () => ref().#m += rhs(2), \"ref,rhs,valueOf,TypeError\");\n\
             check(\"m &&=\", () => ref().#m &&= rhs(4), \"ref,rhs,TypeError\");\n\
             check(\"m ||=\", () => ref().#m ||= rhs(4), \"ref,=fn\");\n\
             check(\"m ??=\", () => ref().#m ??= rhs(4), \"ref,=fn\");\n\
             check(\"m ++x\", () => ++ref().#m, \"ref,valueOf,TypeError\");\n\
             check(\"m x++\", () => ref().#m++, \"ref,valueOf,TypeError\");\n\
             check(\"m --x\", () => --ref().#m, \"ref,valueOf,TypeError\");\n\
             check(\"m x--\", () => ref().#m--, \"ref,valueOf,TypeError\");\n\
             check(\"m [] =\", () => { [ref().#m] = iter(7); }, \"ref,next,return,TypeError\");\n\
             check(\"m {} =\", () => { ({ x: ref().#m } = src(8)); }, \"ref,x,TypeError\");\n\
             check(\"m for of\", () => { for (ref().#m of iter(9)) break; }, \"next,ref,return,TypeError\");\n\
             check(\"m for in\", () => { for (ref().#m in { k: 1 }); }, \"ref,TypeError\");\n\
             g(1); check(\"g =\", () => ref().#g = rhs(5), \"ref,rhs,TypeError\");\n\
             g(1); check(\"g +=\", () => ref().#g += rhs(2), \"ref,get,rhs,TypeError\");\n\
             g(1); check(\"g &&=\", () => ref().#g &&= rhs(4), \"ref,get,rhs,TypeError\");\n\
             g(0); check(\"g ||=\", () => ref().#g ||= rhs(4), \"ref,get,rhs,TypeError\");\n\
             g(null); check(\"g ??=\", () => ref().#g ??= rhs(4), \"ref,get,rhs,TypeError\");\n\
             g(1); check(\"g ++x\", () => ++ref().#g, \"ref,get,TypeError\");\n\
             g(1); check(\"g x++\", () => ref().#g++, \"ref,get,TypeError\");\n\
             g(1); check(\"g --x\", () => --ref().#g, \"ref,get,TypeError\");\n\
             g(1); check(\"g x--\", () => ref().#g--, \"ref,get,TypeError\");\n\
             check(\"g [] =\", () => { [ref().#g] = iter(7); }, \"ref,next,return,TypeError\");\n\
             check(\"g {} =\", () => { ({ x: ref().#g } = src(8)); }, \"ref,x,TypeError\");\n\
             check(\"g for of\", () => { for (ref().#g of iter(9)) break; }, \"next,ref,return,TypeError\");\n\
             check(\"g for in\", () => { for (ref().#g in { k: 1 }); }, \"ref,TypeError\");\n\
             return o.#m() === \"m\";\n\
           }\n\
         }\n\
         if (!C.run(new C())) failures.push(\"the method changed\");\n\
         failures.join(\"; \");";
    if cfg!(feature = "collect-every-safe-point") {
        let nothing = Outcome::Normal("\"\"".to_string());
        assert_eq!(run(limits(), source), nothing, "{source}");
    } else {
        both_list_nothing(source);
    }
}

/// GetValue and PutValue of a private reference (`#sec-getvalue`,
/// `#sec-putvalue`, step 3) on every kind of base, with the order of the side effects: `o.#s = v`
/// evaluates `v` before ToObject or PrivateSet throws; `o.#g += v` and `o.#m(arg)` throw at
/// GetValue, before `v` or `arg`; `o?.a.#m` short-circuits only on `o`; a static element is on F
/// alone, not on a function whose prototype is F, a `Proxy` of F (no trap runs), the prototype
/// or an instance. The script answers the indexes of the checks that failed.
#[test]
fn brand_check_order_on_every_base() {
    both_list_nothing(
        r#"var log = [];
class C {
  #m() { return this; }
  get #g() { return 1; }
  set #s(v) { log.push("set"); }
  static #st() { return "st"; }
  static put(o) { try { o.#s = (log.push("rhs"), 1); } catch (e) { log.push(e.constructor.name); } }
  static add(o) { try { o.#g += (log.push("rhs"), 1); } catch (e) { log.push(e.constructor.name); } }
  static call(o) { try { o.#m(log.push("arg")); } catch (e) { log.push(e.constructor.name); } }
  static chain(o) { try { return o?.a.#m; } catch (e) { return e.constructor.name; } }
  static has(o) { try { o.#st; return true; } catch (e) { return false; } }
}
var bases = [null, undefined, 1, "s", {}];
function each(f) { log = []; bases.forEach(f); return log.join(); }
var put = each(function (b) { C.put(b); });
var add = each(function (b) { C.add(b); });
var call = each(function (b) { C.call(b); });
log = []; C.call(new C()); var branded = log.join();
var traps = [];
var p = new Proxy(C, { get() { traps.push("get"); }, getPrototypeOf() { traps.push("proto"); return null; } });
function D() {}
Object.setPrototypeOf(D, C);
var checks = [
  put === "rhs,TypeError,rhs,TypeError,rhs,TypeError,rhs,TypeError,rhs,TypeError",
  add === "TypeError,TypeError,TypeError,TypeError,TypeError",
  call === "TypeError,TypeError,TypeError,TypeError,TypeError",
  branded === "arg",
  C.chain(null) === undefined, C.chain({ a: {} }) === "TypeError",
  typeof C.chain({ a: new C() }) === "function",
  C.has(C), !C.has(D), !C.has(p), !C.has(C.prototype), !C.has(new C()), traps.length === 0
];
checks.reduce(function (a, c, i) { return c === true ? a : a + i + " "; }, "");"#,
    );
}

/// `#sec-ecmascript-function-objects-construct-argumentslist-newtarget`, step 6:
/// a base class's [[Construct]] runs InitializeInstanceElements before the parameters are
/// bound, so a default parameter reads a private method; the object made from a NewTarget that
/// is another constructor is branded by the class whose constructor ran; an object the
/// constructor returns in place of `this` is not branded, and an instance returned again is not
/// initialized twice.
#[test]
fn the_brand_lands_before_the_parameters_and_on_the_object_made() {
    both_list_nothing(
        r#"class A {
  #m() { return 7; }
  constructor(a = this.#m(), ret) { this.a = a; if (ret) return ret; }
  static has(o) { try { o.#m; return true; } catch (e) { return false; } }
}
function B() {}
var made = Reflect.construct(A, [], B);
var plain = {};
var first = new A();
var again = new A(undefined, first);
var other = new A(undefined, plain);
var checks = [
  first.a === 7,
  A.has(made), Object.getPrototypeOf(made) === B.prototype, made.a === 7,
  again === first, A.has(first),
  other === plain, !A.has(plain), plain.a === undefined
];
checks.reduce(function (a, c, i) { return c === true ? a : a + i + " "; }, "");"#,
    );
}

/// The early errors of private names
/// (`#sec-static-semantics-allprivateidentifiersvalid`, `#sec-class-definitions-static-semantics-early-errors`,
/// `#sec-delete-operator-static-semantics-early-errors`) and the sources next to them that are
/// legal: a getter and a setter of one name, `.#m` after a line break, Unicode escapes in a
/// private name; `Function` and indirect `eval` throw the `SyntaxError` at run time.
#[test]
fn early_errors_of_private_names() {
    let errors: &[(&str, &str)] = &[
        ("class C { f() { this.#x; } }", "#x is not declared"),
        (
            "class C { #m() {} f() { function g() { return this.#n; } } }",
            "#n is not declared",
        ),
        ("this.#x;", "#x is not declared"),
        (
            "class O { #x() {} static t() { class I { #y() {} f(o) { return o.#z; } } } }",
            "#z is not declared",
        ),
        ("class C { #m() {} #m() {} }", "duplicate private name #m"),
        (
            "class C { get #m() {} get #m() {} }",
            "duplicate private name #m",
        ),
        (
            "class C { static get #m() {} set #m(v) {} }",
            "duplicate private name #m",
        ),
        (
            "class C { static #m() {} #m() {} }",
            "duplicate private name #m",
        ),
        (
            "class C { #m() {} get #m() {} }",
            "duplicate private name #m",
        ),
        ("class C { #constructor() {} }", "#constructor"),
        ("class C { static #constructor() {} }", "#constructor"),
        ("class C { # m() {} }", "unexpected character #"),
        (
            "class C { #m() {} f() { delete this.#m; } }",
            "delete of a private reference",
        ),
        (
            "class C { #m() {} f() { delete (this.#m); } }",
            "delete of a private reference",
        ),
        (
            "class C { #m() {} f() { delete this?.#m; } }",
            "delete of a private reference",
        ),
        (
            "class C { #m() {} f() { return super.#m; } }",
            "unexpected private name #m",
        ),
    ];
    for (source, message) in errors {
        for text in [source.to_string(), format!("\"use strict\";\n{source}")] {
            match run(limits(), &text) {
                Outcome::Errors(found) => assert!(
                    found.len() == 1 && found[0].contains(message),
                    "{text}: {found:?}"
                ),
                other => panic!("{text}: {other:?}"),
            }
        }
    }
    both_pass(
        r#"class C {
  get #p() { return this.v; } set #p(v) { this.v = v; }
  #\u{6d}() { return 1; }
  f() { this.#p = 2; return this
    .#m() + this.#m() + this.#p; }
}
function syntax(f) { try { f(); } catch (e) { return e instanceof SyntaxError; } return false; }
new C().f() === 4 && syntax(function () { Function("o", "return o.#x;"); })
  && syntax(function () { (0, eval)("o.#x"); });"#,
    );
}

/// What a direct `eval` with a private name gives until `story:private-names-in-eval-code`
/// passes the class's names to the parse (`#sec-performeval`, step 12). A name that a class
/// enclosing the call declares, which the standard would run, is the stated refusal `private
/// name #p in direct eval code is not supported before slice 8` (a `Refusal` by
/// `swell_syntax::message_kind`, counted once per eval): in a method, a private method, a
/// static method, an arrow in a method and a method of an inner class reading the outer class's
/// name. A name no enclosing class declares is the early `SyntaxError` the standard gives,
/// counted as no refusal: in a method, beside a declared name (whose error is dropped), outside
/// every class, in a direct `eval` in the heritage (which runs before the class's private record
/// is entered), by indirect `eval` and by `Function`. Code with an early error of the grammar
/// throws that error, and no refusal is counted for its declared name.
#[test]
fn a_direct_eval_is_refused_only_for_a_name_an_enclosing_class_declares() {
    let script = r#"function said(e) { return (e instanceof SyntaxError ? "SyntaxError: " : "other: ") + e.message; }
function tried(f) { try { f(); } catch (e) { return said(e); } return "none"; }
class C {
  #p() { return 1; }
  static #s() { return 3; }
  #q() { return tried(() => eval("this.#p()")); }
  method() { try { eval("this.#p()"); } catch (e) { return said(e); } return "none"; }
  static st() { try { eval("C.#s()"); } catch (e) { return said(e); } return "none"; }
  arrow() { return (() => { try { eval("this.#p"); } catch (e) { return said(e); } return "none"; })(); }
  undeclared() { try { eval("this.#x"); } catch (e) { return said(e); } return "none"; }
  mixed() { try { eval("this.#p; this.#x"); } catch (e) { return said(e); } return "none"; }
  grammar() { try { eval("let a; let a; this.#p;"); } catch (e) { return said(e); } return "none"; }
  priv() { return this.#q(); }
}
class O { #o() {} static make() { return class { m() { try { eval("this.#o"); } catch (e) { return said(e); } return "none"; } }; } }
var heritage;
class D extends (function () { try { eval("this.#d"); } catch (e) { heritage = said(e); } return Object; })() { #d() {} }
var c = new C(), Inner = O.make();
[c.method(), c.priv(), C.st(), c.arrow(), new Inner().m(),
 c.undeclared(), c.mixed(), (function () { try { eval("this.#x"); } catch (e) { return said(e); } return "none"; })(),
 heritage, tried(() => (0, eval)("this.#x")), tried(() => Function("return this.#x;")),
 c.grammar()].join("|");"#;
    let refused = |name: &str, at: &str| {
        format!(
            "SyntaxError: private name {name} in direct eval code is not supported before slice 8 ({at})"
        )
    };
    let early = |name: &str, at: &str| {
        format!("SyntaxError: private name {name} is not declared in an enclosing class ({at})")
    };
    let wanted = [
        refused("#p", "1:6"),
        refused("#p", "1:6"),
        refused("#s", "1:3"),
        refused("#p", "1:6"),
        refused("#o", "1:6"),
        early("#x", "1:6"),
        early("#x", "1:15"),
        early("#x", "1:6"),
        early("#d", "1:6"),
        early("#x", "1:6"),
        early("#x", "3:13"),
    ];
    for text in [script.to_string(), format!("\"use strict\";\n{script}")] {
        let mut agent = agent().unwrap();
        let r = realm(&mut agent);
        let before = agent.refusals();
        let answer = match eval_text(&mut agent, &r, &text) {
            Outcome::Normal(answer) => answer,
            other => panic!("{other:?}"),
        };
        let counted = agent.refusals() - before;
        let answers: Vec<&str> = answer.trim_matches('"').split('|').collect();
        assert_eq!(answers.len(), wanted.len() + 1, "{answer}");
        assert_eq!(answers[..wanted.len()], wanted[..], "{answer}");
        let grammar = answers[wanted.len()];
        assert!(
            grammar.starts_with("SyntaxError: ") && !grammar.contains("#p"),
            "{grammar}"
        );
        assert_eq!(counted, 5, "{answer}");
    }
    let message = "private name #p in direct eval code is not supported before slice 8";
    assert_eq!(
        swell_syntax::message_kind(message),
        swell_syntax::MessageKind::Refusal { slice: 8 }
    );
    for (text, at) in [("this.#x;", "1:6"), ("\"use strict\";\nthis.#x;", "2:6")] {
        assert_eq!(
            run(limits(), text),
            Outcome::Errors(vec![format!(
                "private name #x is not declared in an enclosing class ({at})"
            )]),
            "{text}"
        );
    }
}

/// A private method's [[HomeObject]] is the prototype, a static one's is F
/// (`#sec-runtime-semantics-classelementevaluation`, MethodDefinitionEvaluation with `object`),
/// so `super.x` inside an instance private method, getter or setter reads the prototype's
/// prototype and inside a static one F's prototype.
#[test]
fn super_in_a_private_method_reads_its_home_object() {
    both_pass(
        r#"class C {
  #m() { return super.x; }
  get #g() { return super.x; }
  set #s(v) { this.seen = super.x + v; }
  static #t() { return super.y; }
  f() { this.#s = "!"; return this.#m() + this.#g + this.seen; }
  static g() { return C.#t(); }
}
Object.setPrototypeOf(C.prototype, { x: "p" });
Object.setPrototypeOf(C, { y: "s" });
new C().f() === "ppp!" && C.g() === "s";"#,
    );
}

/// A private generator method suspended across collections, and a private async
/// method and async generator method across collections and a checkpoint, each call
/// `this.#o()` after the resume, while the class that made them is reachable only through
/// their frames.
#[test]
fn suspended_private_methods_across_collections() {
    let mut agent = support::host::agent().unwrap();
    let r = realm(&mut agent);
    let garbage = "var g = []; for (var j = 0; j < 5; j++) g.push(class { #x() {} }); g = null; 0;";
    assert_eq!(
        eval_text(
            &mut agent,
            &r,
            "var it = (function () { class C { *#g() { var x = yield 1; yield this.#o() + x; } #o() { return 2; }\n\
               static make() { return new C().#g(); } } return C.make(); })(); it.next().value;"
        ),
        Outcome::Normal("1".to_string())
    );
    assert_eq!(
        eval_text(
            &mut agent,
            &r,
            "var r = 'pending', r2 = 'pending'; (function () { class C { async #a() { await null; return this.#o(); }\n\
               async *#ag() { await null; yield this.#o() + 1; } #o() { return 3; }\n\
               static go() { var c = new C(); c.#a().then(function (v) { r = v; });\n\
                 c.#ag().next().then(function (v) { r2 = v.value; }); } } C.go(); })(); 0;"
        ),
        Outcome::Normal("0".to_string())
    );
    for _ in 0..2 {
        assert!(matches!(agent.collect(), Ok(HostOutcome::Normal(_))));
        assert_eq!(
            eval_text(&mut agent, &r, garbage),
            Outcome::Normal("0".to_string())
        );
    }
    assert!(matches!(agent.collect(), Ok(HostOutcome::Normal(_))));
    assert!(matches!(
        agent.perform_microtask_checkpoint(),
        Ok(HostOutcome::Normal(_))
    ));
    assert!(matches!(agent.collect(), Ok(HostOutcome::Normal(_))));
    assert_eq!(
        eval_text(&mut agent, &r, "it.next(5).value + ',' + [r, r2].join();"),
        Outcome::Normal("\"7,3,4\"".to_string())
    );
}

/// The class of [`budgets_never_leave_a_half_branded_instance`]: three private methods
/// and an accessor pair, and a check that reads all of them.
#[cfg(not(feature = "collect-every-safe-point"))]
const HALF_BRANDED_SWEEP: &str = "class C { #a(){ return 1 } #b(){ return 2 } #c(){ return 3 }\n\
   get #p(){ return this.v } set #p(x){ this.v = x }\n\
   static check(o){ try { o.#p = 4; return o.#a() + o.#b() + o.#c() + o.#p === 10 } catch (e) { return 'partial ' + e } } }\n\
 var o = new C(); C.check(o)";

/// The answers of [`HALF_BRANDED_SWEEP`] in a fresh agent with `limits`, and of a second script
/// that checks `o` if the first one stored it.
#[cfg(not(feature = "collect-every-safe-point"))]
fn half_branded_outcome(limits: swell::Limits) -> (Outcome, Outcome) {
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    let first = eval_text(&mut agent, &realm, HALF_BRANDED_SWEEP);
    assert!(matches!(agent.collect(), Ok(HostOutcome::Normal(_))));
    let second = eval_text(
        &mut agent,
        &realm,
        "typeof o === 'undefined' ? 'none' : C.check(o)",
    );
    (first, second)
}

/// The memory budget 64 bytes at a time over the last 4 KiB below what
/// [`HALF_BRANDED_SWEEP`] needs, and the step budget one step at a time over all its steps: each
/// run ends in the completed object or in the termination, and a later script finds no
/// instance or a whole one, never one that lacks an element. Default mode only (a fresh agent
/// per budget; in the collect mode a realm alone takes about 0.1 s).
#[cfg(not(feature = "collect-every-safe-point"))]
#[test]
fn budgets_never_leave_a_half_branded_instance() {
    let done = Outcome::Normal("true".to_string());
    let none = Outcome::Normal("\"none\"".to_string());
    let fresh = {
        let mut agent = support::host::agent().unwrap();
        let _realm = realm(&mut agent);
        agent.heap_bytes().unwrap()
    };
    let (mut lo, mut hi) = (fresh, fresh + (1 << 20));
    while hi - lo > 64 {
        let mid = lo + (hi - lo) / 2;
        let mut l = limits();
        l.memory_bytes = mid;
        if half_branded_outcome(l).0 == done {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    for budget in (hi.saturating_sub(4096)..hi).step_by(64) {
        let mut l = limits();
        l.memory_bytes = budget;
        let (first, second) = half_branded_outcome(l);
        assert!(
            first == done || first == Outcome::Terminated(Termination::MemoryBudget),
            "{budget}: {first:?}"
        );
        assert!(
            second == done
                || second == none
                || second == Outcome::Terminated(Termination::MemoryBudget),
            "{budget}: {second:?}"
        );
    }
    let mut agent = support::host::agent().unwrap();
    let r = realm(&mut agent);
    let outcome = eval(&mut agent, &r, HALF_BRANDED_SWEEP);
    assert!(matches!(outcome, HostOutcome::Normal(_)));
    let steps = agent.last_entry_steps().unwrap();
    for budget in 1..=steps {
        let mut l = limits();
        l.steps_per_entry = budget;
        let (first, second) = half_branded_outcome(l);
        assert!(
            first == done || first == Outcome::Terminated(Termination::StepBudget),
            "{budget}: {first:?}"
        );
        assert!(
            second == done
                || second == none
                || second == Outcome::Terminated(Termination::StepBudget),
            "{budget}: {second:?}"
        );
    }
}

/// How many instances [`an_instance_pays_for_the_room_of_its_list`] holds: 200, and 4 with a
/// collection at every safe point (for the time: 10 took 2.4 CPU s alone there).
const ROOM_INSTANCES: u64 = if cfg!(feature = "collect-every-safe-point") {
    4
} else {
    200
};

/// The list of an instance holds the room of its class's elements and no more
/// (`object/private_names.rs`): the first add of a construction reserves room for exactly the
/// class's private methods and accessors, a getter and a setter of one name counted once, and
/// charges it with the box. Per instance, over a plain one, a class with `k` private methods
/// costs the box and `k` entries: the bytes of `k` = 1, 4, 5, 9 and 17 lie on one line whose
/// step is an entry (at least 16 bytes) and whose start is the box, and an accessor pair costs
/// what one method does. Each reading is the rise of `heap_bytes()` from the agent without the
/// instances to the agent holding them, both after a collection, so the class's own functions
/// and the array that holds the instances cancel out.
#[test]
fn an_instance_pays_for_the_room_of_its_list() {
    let n = ROOM_INSTANCES;
    let mut agent = support::host::agent().unwrap();
    let r = realm(&mut agent);
    let methods = |k: u32| (0..k).map(|i| format!(" #m{i}() {{}}")).collect::<String>();
    let classes: Vec<(&str, String)> = vec![
        ("0", String::new()),
        ("1", methods(1)),
        ("pair", " get #a() { return 1; } set #a(v) {}".to_string()),
        ("4", methods(4)),
        ("5", methods(5)),
        ("9", methods(9)),
        ("17", methods(17)),
    ];
    let mut rises = Vec::new();
    for (_, body) in &classes {
        assert_eq!(
            eval_text(
                &mut agent,
                &r,
                &format!("var K = class {{{body} }}; var keep = null; 0;")
            ),
            Outcome::Normal("0".to_string())
        );
        let fill = format!("keep = []; for (var i = 0; i < {n}; i++) keep.push(new K()); 0;");
        let mut rise = 0;
        // The first round grows the arenas.
        for _ in 0..2 {
            assert!(matches!(agent.collect(), Ok(HostOutcome::Normal(_))));
            let empty = agent.heap_bytes().unwrap();
            assert_eq!(
                eval_text(&mut agent, &r, &fill),
                Outcome::Normal("0".to_string())
            );
            assert!(matches!(agent.collect(), Ok(HostOutcome::Normal(_))));
            rise = agent.heap_bytes().unwrap() - empty;
            assert_eq!(
                eval_text(&mut agent, &r, "keep = null; 0;"),
                Outcome::Normal("0".to_string())
            );
        }
        rises.push(rise);
    }
    let per: Vec<u64> = rises
        .iter()
        .map(|&rise| {
            let over = rise - rises[0];
            assert_eq!(over % n, 0, "{rises:?}");
            over / n
        })
        .collect();
    let table: Vec<String> = classes
        .iter()
        .zip(&per)
        .map(|((name, _), bytes)| format!("{name}: {bytes}"))
        .collect();
    println!("bytes per instance over a plain one, by private methods: {table:?}");
    let entry = per[4] - per[3];
    let start = per[1] - entry;
    assert!(entry >= 16 && start > 0, "{table:?}");
    for (k, bytes) in [
        (1u64, per[1]),
        (4, per[3]),
        (5, per[4]),
        (9, per[5]),
        (17, per[6]),
    ] {
        assert_eq!(bytes, start + k * entry, "{k} methods: {table:?}");
    }
    assert_eq!(per[2], per[1], "an accessor pair is one entry: {table:?}");
}

/// A lookup by name that passes the private record: a class inside a `with`
/// body (non-strict code) whose private method reads, writes and `typeof`s a name the `with`
/// object holds, and a direct `eval` in a private method that reads the class's own name and a
/// name of the `with` object, while a property named `#m` on the `with` object is no private
/// name. The private record is named only when a direct `eval` stands in the class body.
#[test]
fn a_lookup_by_name_passes_the_private_record() {
    let source = r##"var scope = { x: 1, "#m": "property" }, r;
with (scope) {
  class C {
    #m() { x = x + 1; return typeof x + x; }
    #e() { return eval("C === K && x"); }
    f() { return this.#m() + "," + this.#e(); }
  }
  var K = C;
  r = new C().f();
}
r === "number2,2" && scope.x === 2;"##;
    assert_eq!(run(limits(), source), passed(), "{source}");
}

/// Private methods of a derived class (SuperCall, step 11: InitializeInstanceElements after
/// BindThisValue): before `super()` the constructor's `this` is uninitialized, a
/// `ReferenceError`; after it the methods are on the object. A base constructor that returns
/// another object makes that object the one the derived class's methods are added to (the
/// object is "stamped"): a frozen object and a Proxy get them (the add does not read
/// [[Extensible]] and no trap runs), and a second stamp of the same object is the `TypeError`
/// of PrivateMethodOrAccessorAdd, step 3, after the parent's constructor ran.
#[test]
fn private_methods_of_a_derived_class_meet_super_and_return_override() {
    both_list_nothing(
        "var wrong = [];\n\
         class P { constructor() { this.p = 1; } }\n\
         class C extends P {\n\
           #m() { return 'm'; }\n\
           constructor() {\n\
             try { this.#m(); wrong.push('before'); } catch (e) { if (!(e instanceof ReferenceError)) wrong.push('before kind'); }\n\
             super();\n\
             this.after = this.#m();\n\
           }\n\
         }\n\
         if (new C().after !== 'm') wrong.push('after');\n\
         class Base { constructor(o) { return o; } }\n\
         class Stamp extends Base { #m() { return 'stamped'; } static read(o) { return o.#m(); } }\n\
         var plain = {}, frozen = Object.freeze({}), traps = 0;\n\
         var proxy = new Proxy({}, { defineProperty() { traps++; return true; }, getOwnPropertyDescriptor() { traps++; } });\n\
         if (new Stamp(plain) !== plain || Stamp.read(plain) !== 'stamped') wrong.push('plain');\n\
         new Stamp(frozen); if (Stamp.read(frozen) !== 'stamped') wrong.push('frozen');\n\
         new Stamp(proxy); if (Stamp.read(proxy) !== 'stamped' || traps !== 0) wrong.push('proxy');\n\
         try { new Stamp(plain); wrong.push('twice'); } catch (e) { if (!(e instanceof TypeError)) wrong.push('twice kind'); }\n\
         try { Stamp.read({}); wrong.push('unstamped'); } catch (e) { if (!(e instanceof TypeError)) wrong.push('unstamped kind'); }\n\
         wrong.join();",
    );
}

/// ClassDefinitionEvaluation, step 8: the heritage is evaluated with the PrivateEnvironment of
/// the code around the class and in the class scope, so the class's private record is entered
/// after it. A heritage calls an outer class's private method of the same name as one the
/// inner class declares; a heritage reads a `let` of the enclosing function, and an arrow made
/// in the heritage reads it later; the class's methods still reach their own private record.
/// A heritage naming a private name only the inner class declares is the early `SyntaxError`
/// (`#sec-class-definitions-static-semantics-early-errors`, ClassTail).
#[test]
fn the_private_record_is_entered_after_the_heritage() {
    both_pass(
        "class Outer {\n\
           #m() { return 'outer'; }\n\
           static f(o) {\n\
             return class extends (o.#m() === 'outer' ? Object : null) {\n\
               #m() { return 'inner'; }\n\
               run() { return this.#m(); }\n\
             };\n\
           }\n\
         }\n\
         var K = Outer.f(new Outer());\n\
         Object.getPrototypeOf(K) === Object && new K().run() === 'inner';",
    );
    both_pass(
        "function make() {\n\
           let base = Object, seen = 'first', read;\n\
           const K = class extends (read = () => seen, base) {\n\
             #p() { return seen; }\n\
             get() { return this.#p(); }\n\
           };\n\
           seen = 'later';\n\
           return [K, read];\n\
         }\n\
         var made = make(), K = made[0], read = made[1];\n\
         Object.getPrototypeOf(K) === Object && read() === 'later' && new K().get() === 'later';",
    );
    let source = "class C extends (function (o) { return o.#x; }, Object) { #x() {} }";
    for text in [source.to_string(), format!("\"use strict\";\n{source}")] {
        match run(limits(), &text) {
            Outcome::Errors(found) => assert!(
                found.len() == 1
                    && found[0].contains("private name #x is not declared in an enclosing class"),
                "{text}: {found:?}"
            ),
            other => panic!("{text}: {other:?}"),
        }
    }
    both_pass(
        "class O {\n\
           #x() { return 'o'; }\n\
           static f(o) { return class extends (o.#x() === 'o' ? Object : null) { #x() {} }; }\n\
         }\n\
         Object.getPrototypeOf(O.f(new O())) === Object;",
    );
}

/// Each `(name, source)` answers `true` as written and as strict code; the
/// assertion lists every case that does not, with what it answered.
fn all_pass(cases: &[(&str, String)]) {
    let wrong: Vec<String> = cases
        .iter()
        .filter_map(|(name, source)| {
            let got = run_both(source);
            (got != (passed(), passed())).then(|| format!("{name}: {got:?}"))
        })
        .collect();
    assert!(wrong.is_empty(), "{wrong:#?}");
}

/// The parent of the derived classes of the scripts below.
const PARENT: &str = "class P { constructor() { this.p = 1; } }\n";

/// The `this` binding of a derived constructor and the private record of a
/// class declared inside that constructor live in one function. `super()` in a computed key, a
/// field key and the heritage of the inner class (each `Contains` SuperCall of the outer
/// constructor, `#sec-static-semantics-contains`, ClassTail), an arrow made there and called
/// later, a direct `eval` in a computed key (the inner record named), `super()` from an arrow
/// and from eval code beside the outer class's own private methods, a second `super()` by eval,
/// and the return without `super()`: none may reach a slot of a private record as `this`.
#[test]
fn this_and_a_private_record_in_one_derived_constructor() {
    let cases: Vec<(&str, String)> = vec![
        ("super() in a computed method key", format!("{PARENT}class D extends P {{
  constructor() {{
    var self = () => this;
    class I {{ #m() {{ return 'i'; }} [super().p === 1 ? 'k' : 'j']() {{ return this.#m(); }} }}
    this.r = new I().k() + (self() === this);
  }}
}}
new D().r === 'itrue';")),
        ("super() in a static field key", format!("{PARENT}class D extends P {{
  constructor() {{
    var self = () => this;
    class I {{ #m() {{ return 'f'; }} static [(super(), 'k')] = 1; get() {{ return this.#m(); }} }}
    this.r = new I().get() + I.k + (self() === this);
  }}
}}
new D().r === 'f1true';")),
        ("super() in an instance field key", format!("{PARENT}class D extends P {{
  constructor() {{
    class I {{ #m() {{ return 'g'; }} [(super(), 'k')] = 2; get() {{ return this.#m() + this.k; }} }}
    this.r = new I().get() + this.p;
  }}
}}
new D().r === 'g21';")),
        ("super() in the heritage of the inner class", format!("{PARENT}class D extends P {{
  constructor() {{
    var self = () => this;
    class I extends (super(), Object) {{ #m() {{ return 'h'; }} get() {{ return this.#m(); }} }}
    this.r = new I().get() + (self() === this) + this.p;
  }}
}}
new D().r === 'htrue1';")),
        ("an arrow made in a computed key calls super() later", format!("{PARENT}class D extends P {{
  constructor() {{
    var go;
    class I {{ #m() {{ return 'late'; }} [(go = () => super(), 'k')]() {{ return this.#m(); }} }}
    go();
    this.r = new I().k() + this.p;
  }}
}}
new D().r === 'late1';")),
        ("eval super() in a computed key, blocks around", format!("{PARENT}class D extends P {{
  constructor() {{
    let a = 1;
    {{
      let b = 2;
      class I {{ #m() {{ return a + b; }} g() {{ return this.#m(); }} [eval(\"super(); 'k'\")]() {{ return 'k'; }} }}
      this.r = new I().g() + this.p + new I().k();
    }}
  }}
}}
new D().r === '4k';")),
        ("this before super() in an inner computed key", format!("{PARENT}class D extends P {{
  constructor() {{
    try {{ class I {{ #m() {{}} [this.x]() {{}} }} }} catch (e) {{ super(); this.r = e instanceof ReferenceError; return; }}
    super(); this.r = false;
  }}
}}
new D().r === true;")),
        ("super() from an arrow, then the own private method", format!("{PARENT}class D extends P {{
  #m() {{ return 'm4'; }}
  constructor() {{ var go = () => {{ super(); return this.#m(); }}; var v = go(); this.r = v; }}
}}
new D().r === 'm4';")),
        ("eval super(), then the own private method", format!("{PARENT}class D extends P {{
  #m() {{ return 'm5'; }}
  constructor() {{ eval('super()'); this.r = this.#m(); }}
}}
new D().r === 'm5';")),
        ("an arrow's eval super()", format!("{PARENT}class D extends P {{
  #m() {{ return 'm6'; }}
  constructor() {{ (() => eval('super()'))(); this.r = this.#m(); }}
}}
new D().r === 'm6';")),
        ("a second super() by eval", format!("{PARENT}class D extends P {{
  #m() {{ return 'm'; }}
  constructor() {{ super(); try {{ eval('super()'); this.r = 'no'; }} catch (e) {{ this.r = (e instanceof ReferenceError) + this.#m(); }} }}
}}
new D().r === 'truem';")),
        ("a return override without super()", format!("{PARENT}class D extends P {{
  #m() {{}}
  constructor() {{ return {{ o: 1 }}; }}
  static has(o) {{ try {{ o.#m; return true; }} catch (e) {{ return false; }} }}
}}
var d = new D(); d.o === 1 && !D.has(d);")),
        ("no super() at all", format!("{PARENT}class D extends P {{
  #m() {{}}
  constructor() {{ var f = () => this; }}
}}
var kind; try {{ new D(); }} catch (e) {{ kind = e instanceof ReferenceError; }} kind === true;")),
        ("return after super() from an arrow", format!("{PARENT}class D extends P {{
  #m() {{ return 'r'; }}
  constructor() {{ (() => super())(); return; }}
  static read(o) {{ return o.#m(); }}
}}
D.read(new D()) === 'r';")),
    ];
    all_pass(&cases);
}

/// The resolver's order where fields, private methods and `super()` meet. Each
/// class shape (private methods only, fields only, both) as a base and as a derived class with
/// an explicit constructor, a default one, `super()` from an arrow and from an arrow's eval; a
/// private method that reads a field; a computed key that calls an outer class's private method;
/// a computed key whose arrow keeps the class's own private name; four classes nested through
/// field initializers, the innermost reading every outer private method.
#[test]
fn fields_private_methods_and_super_in_one_class() {
    let shapes = [
        (
            "methods",
            "#m() { return 'm'; } get() { return this.#m(); }",
            "m",
        ),
        ("fields", "f = 'f'; get() { return this.f; }", "f"),
        (
            "both",
            "#m() { return 'm'; } f = this.#m() + 'f'; get() { return this.f + this.#m(); }",
            "mfm",
        ),
    ];
    let kinds = [
        ("base", "", ""),
        ("base explicit", "", "constructor() { this.c = 1; }"),
        (
            "derived explicit",
            " extends P",
            "constructor() { super(); }",
        ),
        ("derived default", " extends P", ""),
        (
            "derived arrow",
            " extends P",
            "constructor() { var go = () => super(); go(); }",
        ),
        (
            "derived arrow eval",
            " extends P",
            "constructor() { (() => eval('super()'))(); }",
        ),
    ];
    let mut cases: Vec<(String, String)> = Vec::new();
    for (shape, body, expected) in shapes {
        for (kind, heritage, ctor) in kinds {
            cases.push((
                format!("{shape} / {kind}"),
                format!(
                    "{PARENT}class K{heritage} {{ {ctor} {body} }}\n\
                     var k = new K(); k.get() === '{expected}';"
                ),
            ));
        }
    }
    cases.push((
        "a private method reads a field".to_string(),
        "class K { #m() { return this.f; } f = 1; g = this.#m() + 1; }\n\
         new K().g === 2;"
            .to_string(),
    ));
    cases.push((
        "a computed key calls an outer class's private method".to_string(),
        "class O { static #k() { return 'key'; }\n\
           static make() { return class { [O.#k()] = 1; #m() { return this.key; } get() { return this.#m(); } }; } }\n\
         new (O.make())().get() === 1;"
            .to_string(),
    ));
    cases.push((
        "a computed key's arrow keeps the own private name".to_string(),
        "var f, g;\n\
         class C { #m() { return 'own'; } static #s() { return 'static'; }\n\
           [(f = o => o.#m(), 'k')] = 1; static [(g = () => C.#s(), 's')] = 2; }\n\
         var c = new C(); f(c) === 'own' && g() === 'static' && c.k === 1 && C.s === 2;"
            .to_string(),
    ));
    cases.push((
        "four classes nested through field initializers".to_string(),
        "class A { #a() { return 'A'; }\n\
           inner = new (class B { #b() { return 'B'; }\n\
             inner = new (class C { #c() { return 'C'; }\n\
               inner = new (class D { #d() { return 'D'; }\n\
                 inner = null;\n\
                 all(a, b, c) { return a.#a() + b.#b() + c.#c() + this.#d(); } })();\n\
               r(a, b) { return this.inner.all(a, b, this); } })();\n\
             r(a) { return this.inner.r(a, this); } })();\n\
           r() { return this.inner.r(this); } }\n\
         new A().r() === 'ABCD';"
            .to_string(),
    ));
    cases.push((
        "four derived classes nested through field initializers".to_string(),
        format!(
            "{PARENT}class A extends P {{ #a() {{ return 'A'; }}\n\
               inner = new (class B extends P {{ #b() {{ return 'B'; }}\n\
                 inner = new (class C extends P {{ #c() {{ return 'C'; }}\n\
                   inner = new (class D extends P {{ #d() {{ return 'D'; }}\n\
                     constructor() {{ (() => super())(); }}\n\
                     all(a, b, c) {{ return a.#a() + b.#b() + c.#c() + this.#d() + this.p; }} }})();\n\
                   r(a, b) {{ return this.inner.all(a, b, this); }} }})();\n\
                 r(a) {{ return this.inner.r(a, this); }} }})();\n\
               r() {{ return this.inner.r(this); }} }}\n\
             new A().r() === 'ABCD1';"
        ),
    ));
    let cases: Vec<(&str, String)> = cases
        .iter()
        .map(|(name, source)| (name.as_str(), source.clone()))
        .collect();
    all_pass(&cases);
}

/// `super.#m` is an early error (MemberExpression: `super` . PrivateIdentifier
/// is no production); `super.m()` and a static `super.s()` from private methods of a derived
/// class; a private method, getter and setter named on `this` before `super()` throw the
/// `ReferenceError` of `this` (the base is evaluated first); a private getter read on the
/// value `super()` returns.
#[test]
fn super_meets_private_names() {
    for source in [
        "class C { #m() {} f() { return super.#m; } }",
        "class C extends Object { #m() {} f() { return super.#m(); } }",
    ] {
        for text in [source.to_string(), format!("\"use strict\";\n{source}")] {
            assert!(
                matches!(run(limits(), &text), Outcome::Errors(ref e) if !e.is_empty()),
                "{text}: {:?}",
                run(limits(), &text)
            );
        }
    }
    let cases: Vec<(&str, String)> = vec![
        (
            "super.m() from a private method",
            "class P { m() { return 'pm'; } static s() { return 'ps'; } }\n\
             class D extends P { #x() { return super.m(); } static #y() { return super.s(); }\n\
               get #g() { return super.m() + '!'; } g() { return this.#x() + this.#g + D.#y(); } }\n\
             new D().g() === 'pmpm!ps';"
                .to_string(),
        ),
        (
            "each private form on this before super()",
            format!(
                "{PARENT}var kinds = [];\n\
                 class D extends P {{ #m() {{}} get #g() {{ return 1; }} set #s(v) {{}}\n\
                   constructor() {{\n\
                     try {{ this.#m(); }} catch (e) {{ kinds.push(e.constructor.name); }}\n\
                     try {{ this.#g; }} catch (e) {{ kinds.push(e.constructor.name); }}\n\
                     try {{ this.#s = 1; }} catch (e) {{ kinds.push(e.constructor.name); }}\n\
                     try {{ this.#g++; }} catch (e) {{ kinds.push(e.constructor.name); }}\n\
                     super(); }} }}\n\
                 new D(); kinds.join() === 'ReferenceError,ReferenceError,ReferenceError,ReferenceError';"
            ),
        ),
        (
            "a private getter on the value super() returns",
            format!(
                "{PARENT}class D extends P {{ get #g() {{ return 'g' + this.p; }}\n\
                   constructor() {{ const r = super(); this.r = r.#g + (r === this); }} }}\n\
                 new D().r === 'g1true';"
            ),
        ),
    ];
    all_pass(&cases);
}

/// The heritage runs before the private record is entered. A heritage that reads the
/// class's own name is in its TDZ (a `ReferenceError`), for a declaration and for a named
/// class expression; an arrow made in the heritage reads the class later; a class expression in
/// the heritage with its own `#m` of the same name is a different brand; a direct `eval` in the
/// heritage; a closure made in the heritage reads a `let` before the class and a `let` of a
/// block around it, in a function whose bindings live in records.
#[test]
fn the_heritage_runs_before_the_private_record() {
    let cases: Vec<(&str, String)> = vec![
        (
            "the class name in the heritage is in its TDZ",
            "var kind; try { class C extends (C, Object) { #m() {} } } catch (e) { kind = e instanceof ReferenceError; }\n\
             var kind2; try { (class C extends (C, Object) { #m() {} }); } catch (e) { kind2 = e instanceof ReferenceError; }\n\
             kind === true && kind2 === true;"
                .to_string(),
        ),
        (
            "an arrow made in the heritage reads the class later",
            "var f; class C extends (f = () => C, Object) { #m() { return 1; } r() { return this.#m(); } }\n\
             var g; var K = class N extends (g = () => N, Object) { #m() { return 2; } static t(o) { return o.#m(); } };\n\
             f() === C && new C().r() === 1 && g() === K && K.t(new K()) === 2;"
                .to_string(),
        ),
        (
            "a class expression with its own #m in the heritage",
            "class C extends (class { #m() { return 'h'; } static t(o) { return o.#m(); } }) { #m() { return 'c'; } r() { return this.#m(); } }\n\
             var c = new C(); c.r() === 'c' && Object.getPrototypeOf(C).t(c) === 'h';"
                .to_string(),
        ),
        (
            "a direct eval in the heritage",
            "class C extends (eval('Object')) { #m() { return 1; } r() { return this.#m(); } }\n\
             new C().r() === 1;"
                .to_string(),
        ),
        (
            "closures of the heritage read a let before and a let around",
            "function make() {\n\
               let a = 'a', g, K;\n\
               { let b = 'b';\n\
                 K = class extends (g = () => a + b, Object) { #m() { return a + b + (() => a)(); } r() { return this.#m(); } };\n\
                 b = 'B'; }\n\
               a = 'A';\n\
               return [K, g];\n\
             }\n\
             var made = make();\n\
             made[1]() === 'AB' && new made[0]().r() === 'ABA';"
                .to_string(),
        ),
    ];
    all_pass(&cases);
}

/// A second `super()` with a Proxy as the object the parent returns. SuperCall runs
/// the parent again (its counter is 2), then BindThisValue throws the `ReferenceError` before
/// InitializeInstanceElements, so no field initializer runs again and no private method is
/// added again (which would be the `TypeError` of PrivateMethodOrAccessorAdd). On the first
/// `super()` the private method is on the Proxy before the field's `defineProperty` trap runs
/// (InitializeInstanceElements, steps 3 and 4).
#[test]
fn a_second_super_with_a_proxy_adds_nothing_again() {
    both_pass(
        "var ctor = 0, init = 0, traps = [], proxy;\n\
         proxy = new Proxy({}, { defineProperty(t, k, d) { traps.push(String(k) + ':' + D.has(proxy)); return Reflect.defineProperty(t, k, d); } });\n\
         class P { constructor() { ctor++; return proxy; } }\n\
         class D extends P {\n\
           #m() { return 'm'; }\n\
           f = ++init;\n\
           constructor() { super(); try { super(); this.kind = 'none'; } catch (e) { this.kind = e.constructor.name; } }\n\
           static has(o) { try { o.#m(); return true; } catch (e) { return false; } }\n\
         }\n\
         var d = new D();\n\
         d === proxy && ctor === 2 && init === 1 && d.kind === 'ReferenceError'\n\
           && traps.join() === 'f:true,kind:true' && D.has(proxy);",
    );
}

/// A direct eval whose source has an early error of the grammar before a private name, one the
/// class does not declare and one it declares, throws that early error; the engine must not
/// count a refusal for it (`swell_syntax`, `only_the_refusal`: a refusal is a parse's only error), or a host
/// that reads `refusals()` (the test262 runner) takes the right answer for a refusal.
#[test]
fn an_early_error_before_a_private_name_counts_no_refusal() {
    for declared in ["", "#x() {}"] {
        let mut agent = agent().unwrap();
        let r = realm(&mut agent);
        let before = agent.refusals();
        let answer = eval_text(
            &mut agent,
            &r,
            &format!(
                "class C {{ {declared} m() {{ try {{ eval('let a; let a; this.#x;'); }} catch (e) {{ return e.message; }} }} }}\n\
                 new C().m();"
            ),
        );
        let counted = agent.refusals() - before;
        let thrown = match &answer {
            Outcome::Normal(text) => text.clone(),
            other => panic!("{other:?}"),
        };
        assert!(
            !thrown.contains("not supported before slice") && !thrown.contains("#x"),
            "{thrown}"
        );
        assert_eq!(counted, 0, "thrown {thrown}, refusals counted {counted}");
    }
}

/// A direct eval outside every class with an undeclared private name is the early
/// `SyntaxError` the standard gives (`#sec-performeval`, step 12: privateIdentifiers is empty
/// when the running PrivateEnvironment is null), counted as no refusal: the cases
/// `built-ins/eval/private-identifiers-not-empty.js` and
/// `language/statements/class/elements/privatename-not-valid-eval-earlyerr-7.js` and `-8.js` of
/// `story-element-syntax.cases.txt`, which counted a refusal at the head before this rule.
#[test]
fn a_direct_eval_outside_every_class_is_the_early_error() {
    let mut agent = agent().unwrap();
    let r = realm(&mut agent);
    let before = agent.refusals();
    let answer = eval_text(
        &mut agent,
        &r,
        "var o = {}; (function () { try { eval('o.#f'); } catch (e) { return e.message; } })();",
    );
    let counted = agent.refusals() - before;
    assert_eq!(
        (answer, counted),
        (
            Outcome::Normal(
                "\"private name #f is not declared in an enclosing class (1:3)\"".to_string()
            ),
            0
        )
    );
}

/// How many throws each round of [`a_throw_in_the_class_leaks_no_private_name`]
/// makes: 1,000, and 8 with a collection at every safe point (for the time: 20 took 2.23 CPU s).
const THROWS: u32 = if cfg!(feature = "collect-every-safe-point") {
    8
} else {
    1_000
};

/// A heritage that throws, a heritage that is no constructor, a heritage whose
/// `prototype` getter throws, and a computed key that throws after the private record was
/// entered: [`THROWS`] of each, then a collection, leave `heap_bytes()` at the reading of
/// the round before (the first round grows the arenas).
#[test]
fn a_throw_in_the_class_leaks_no_private_name() {
    let mut agent = agent().unwrap();
    let r = realm(&mut agent);
    assert_eq!(
        eval_text(
            &mut agent,
            &r,
            "var i = 0, n = 0; function bad() { throw 1; }\n\
             var proto = function () {}.bind(); Object.defineProperty(proto, 'prototype', { get: bad }); 0;"
        ),
        Outcome::Normal("0".to_string())
    );
    let looped = parse(
        &mut agent,
        &r,
        &format!(
            "n = 0; for (i = 0; i < {THROWS}; i++) {{\n\
               try {{ class C extends (bad()) {{ #m() {{}} static #s() {{}} }} }} catch (e) {{ n++; }}\n\
               try {{ class C extends 5 {{ #m() {{}} }} }} catch (e) {{ n++; }}\n\
               try {{ class C extends proto {{ #m() {{}} }} }} catch (e) {{ n++; }}\n\
               try {{ class C extends Object {{ #m() {{}} static #t() {{}} [bad()]() {{}} }} }} catch (e) {{ n++; }}\n\
             }} n;"
        ),
    );
    let round = |agent: &mut Agent<TestHost>| {
        let outcome = agent.evaluate_script(&looped).unwrap();
        assert_eq!(
            describe(agent, &r, outcome),
            Outcome::Normal((4 * THROWS).to_string())
        );
        assert!(matches!(agent.collect(), Ok(HostOutcome::Normal(_))));
        agent.heap_bytes().unwrap()
    };
    let first = round(&mut agent);
    let second = round(&mut agent);
    let third = round(&mut agent);
    println!("heap_bytes after each round: {first}, {second}, {third}");
    assert_eq!((second, third), (first, first));
}

/// The setups of [`the_room_of_each_shape_and_level`]: each defines `make`, which
/// makes one object the round keeps.
#[cfg(not(feature = "collect-every-safe-point"))]
fn room_setups() -> Vec<(String, String)> {
    let methods = |k: u32| (0..k).map(|i| format!(" #m{i}() {{}}")).collect::<String>();
    let class = |name: &str, body: String| {
        (
            name.to_string(),
            format!("var K = class {{{body} }}; var make = function () {{ return new K(); }}; 0;"),
        )
    };
    let mut setups = vec![class("plain", String::new())];
    for k in [1, 2, 4, 5, 6, 9, 15, 17] {
        setups.push(class(&k.to_string(), methods(k)));
    }
    setups.push(class(
        "static only",
        " static #s() {} static get #t() { return 1; } static set #t(v) {}".to_string(),
    ));
    setups.push(class(
        "pair and method",
        " get #a() { return 1; } set #a(v) {} #m() {}".to_string(),
    ));
    setups.push(class(
        "static and instance accessor",
        " static get #s() { return 1; } get #a() { return 1; }".to_string(),
    ));
    setups.push((
        "three levels".to_string(),
        "var make = (function () { class A3 { #a() {} } class B3 extends A3 { #b() {} #c() {} } class C3 extends B3 { #d() {} #e() {} #f() {} }\n\
         return function () { return new C3(); }; })(); 0;"
            .to_string(),
    ));
    setups.push((
        "three plain levels".to_string(),
        "var make = (function () { class A3 {} class B3 extends A3 {} class C3 extends B3 {}\n\
         return function () { return new C3(); }; })(); 0;"
            .to_string(),
    ));
    let stamps = |name: &str, private: bool| {
        let mut source =
            String::from("var make = (function () { class Base { constructor(o) { return o; } }\n");
        for k in 1..=5 {
            let body = if private { methods(k) } else { String::new() };
            source.push_str(&format!("class S{k} extends Base {{{body} }}\n"));
        }
        source.push_str(
            "return function () { var o = {}; new S1(o); new S2(o); new S3(o); new S4(o); new S5(o); return o; }; })(); 0;",
        );
        (name.to_string(), source)
    };
    setups.push(stamps("five stamps", true));
    setups.push(stamps("five plain stamps", false));
    setups
}

/// The exact room per class level. The rise of `heap_bytes()` per object (200
/// objects, read as in `an_instance_pays_for_the_room_of_its_list`) for each setup of
/// [`room_setups`]: a class with only static private elements costs an instance
/// nothing; a getter and setter pair beside a method costs what two methods do; a static and an
/// instance accessor cost an instance what one method does; an instance of three levels of 1, 2
/// and 3 private methods costs what one level of 6 does (each level reserves exactly its own
/// room, no spare entry after level 3); an object stamped by five classes of 1 to 5 methods
/// through return overrides costs what one level of 15 does. Default mode only (16 setups).
#[cfg(not(feature = "collect-every-safe-point"))]
#[test]
fn the_room_of_each_shape_and_level() {
    let n = 200u64;
    let mut agent = support::host::agent().unwrap();
    let r = realm(&mut agent);
    let mut per = std::collections::BTreeMap::new();
    for (name, setup) in room_setups() {
        assert_eq!(
            eval_text(&mut agent, &r, &format!("{setup} var keep = null; 0;")),
            Outcome::Normal("0".to_string()),
            "{name}"
        );
        let fill = format!("keep = []; for (var i = 0; i < {n}; i++) keep.push(make()); 0;");
        let mut rise = 0;
        for _ in 0..2 {
            assert!(matches!(agent.collect(), Ok(HostOutcome::Normal(_))));
            let empty = agent.heap_bytes().unwrap();
            assert_eq!(
                eval_text(&mut agent, &r, &fill),
                Outcome::Normal("0".to_string())
            );
            assert!(matches!(agent.collect(), Ok(HostOutcome::Normal(_))));
            rise = agent.heap_bytes().unwrap() - empty;
            assert_eq!(
                eval_text(&mut agent, &r, "keep = null; 0;"),
                Outcome::Normal("0".to_string())
            );
        }
        per.insert(name, rise);
    }
    let over = |name: &str, base: &str| {
        let rise = per[name] - per[base];
        assert_eq!(rise % n, 0, "{name} over {base}: {per:?}");
        rise / n
    };
    let table: Vec<String> = ["1", "4", "5", "9", "17"]
        .iter()
        .map(|k| format!("{k}: {}", over(k, "plain")))
        .collect();
    println!("bytes per instance over a plain one: {table:?}; all readings {per:?}");
    let found = [
        over("static only", "plain"),
        over("pair and method", "plain"),
        over("static and instance accessor", "plain"),
        over("three levels", "three plain levels"),
        over("five stamps", "five plain stamps"),
    ];
    let wanted = [
        0,
        over("2", "plain"),
        over("1", "plain"),
        over("6", "plain"),
        over("15", "plain"),
    ];
    assert_eq!(found, wanted, "{per:?}");
}

/// The steps of one construction by its count of private methods. The adds of a
/// construction with n methods spend sum over k of max(0, k - 1 - 8) steps past the
/// instructions, 0 for n <= 9 and (n - 9)(n - 8) / 2 above (`for-coordinator.md`): measured at
/// 7, 8, 9, 10, 16, 17 and 1,000 methods against the step of one more method below `SMALL`.
#[cfg(not(feature = "collect-every-safe-point"))]
#[test]
fn the_steps_of_a_construction_by_private_methods() {
    let mut limits = limits();
    limits.steps_per_entry = 4_000_000;
    let mut agent = agent_with(limits).unwrap();
    let r = realm(&mut agent);
    let mut steps = std::collections::BTreeMap::new();
    for n in [1u64, 7, 8, 9, 10, 16, 17, 1_000] {
        let body = (0..n).map(|i| format!(" #m{i}() {{}}")).collect::<String>();
        assert_eq!(
            eval_text(&mut agent, &r, &format!("var K = class {{{body} }}; 0;")),
            Outcome::Normal("0".to_string())
        );
        let outcome = eval(&mut agent, &r, "new K(); 0;");
        assert!(matches!(outcome, HostOutcome::Normal(_)), "{n}");
        steps.insert(n, agent.last_entry_steps().unwrap());
    }
    println!("steps of `new K(); 0;` by private methods: {steps:?}");
    let per = steps[&8] - steps[&7];
    let extra = |n: u64| steps[&n] - steps[&1] - (n - 1) * per;
    let formula = |n: u64| if n <= 9 { 0 } else { (n - 9) * (n - 8) / 2 };
    let found: Vec<u64> = [9, 10, 16, 17, 1_000].iter().map(|&n| extra(n)).collect();
    let wanted: Vec<u64> = [9, 10, 16, 17, 1_000].iter().map(|&n| formula(n)).collect();
    assert_eq!(found, wanted, "{steps:?}");
}

/// The setup of [`memory_budget_at_every_8_bytes_across_two_levels`]: a level of one
/// private method under a level of five, and a check of every element.
#[cfg(not(feature = "collect-every-safe-point"))]
const LEVELS: &str = "class A { #a() { return 1; } static check(o) { return o.#a() === 1; } }\n\
   class B extends A { #b() {} #c() {} #d() {} #e() {} #f() {}\n\
     static check(o) { try { o.#b(); o.#c(); o.#d(); o.#e(); o.#f(); return A.check(o); } catch (e) { return 'partial'; } } }\n\
   var keep = null; 0;";

/// The length of `h` in [`memory_budget_at_every_8_bytes_across_two_levels`], read
/// by the host function `padLength`, so that the entry that makes it is parsed once.
#[cfg(not(feature = "collect-every-safe-point"))]
static PAD_LENGTH: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// `padLength()`: [`PAD_LENGTH`].
#[cfg(not(feature = "collect-every-safe-point"))]
fn pad_length(
    _cx: &mut swell::Cx<'_, TestHost>,
    _call: swell::HostCall<'_>,
) -> swell::Completion<Value> {
    Ok(Value::Number(
        PAD_LENGTH.load(std::sync::atomic::Ordering::Relaxed) as f64,
    ))
}

/// The memory budget at every 8 bytes across the reservations of the two levels of
/// `new B()` ([`LEVELS`]), in the form of `proxy_shape_traps::memory_sweep`: one agent with
/// 64 KiB above its reading after the setup and a warm-up of three constructions; Strings in
/// `pads` leave a target room (from 2 KiB, 512 more each round), and each entry, parsed once,
/// makes `h`, a String of `u` code units, then `keep = new B()`, for `u` from 3/8 of the room down
/// by 4 (the room the construction has grows by at most 8 bytes per run) until 8 constructions
/// in a row answered. Every run answers (and `B.check(keep)` reads every element) or ends in the
/// memory budget (and `keep` stays `null`); after each, `keep = null` and a collection give
/// `heap_bytes()` back its reading after the round's pads. At the last `u` whose construction
/// started and ended in the budget, 1,000 failed constructions, each with a collection, leave
/// the reading of that round too.
#[cfg(not(feature = "collect-every-safe-point"))]
#[test]
fn memory_budget_at_every_8_bytes_across_two_levels() {
    use std::sync::atomic::Ordering;
    let zero = Outcome::Normal("0".to_string());
    let done = Outcome::Normal("true".to_string());
    let base = {
        let mut probe = support::host::agent().unwrap();
        let r = realm(&mut probe);
        assert_eq!(eval_text(&mut probe, &r, LEVELS), zero);
        assert!(matches!(probe.collect(), Ok(HostOutcome::Normal(_))));
        probe.heap_bytes().unwrap()
    };
    let mut l = limits();
    l.memory_bytes = base + (64 << 10);
    let budget = l.memory_bytes;
    let mut agent = agent_with(l).unwrap();
    let r = realm(&mut agent);
    support::script::define_function(&mut agent, &r, "padLength", 0, pad_length, false);
    assert_eq!(
        eval_text(
            &mut agent,
            &r,
            &format!("var started = 0, pads = []; {LEVELS}")
        ),
        zero
    );
    let scripts = [
        "started = 0; (function () { var h = \"x\".repeat(padLength()); started = 1; keep = new B(); })(); 0;",
        "started;",
        "B.check(keep);",
        "keep === null;",
        "keep = null; 0;",
        "pads.pop(); 0;",
        "pads.push(\"x\".repeat(padLength())); 0;",
        "keep = new B(); 0;",
    ]
    .map(|source| parse(&mut agent, &r, source));
    let [run, started, check, unset, clear, pop, push, make] = &scripts;
    let go = |agent: &mut Agent<TestHost>, script: &swell::Script| {
        let outcome = agent.evaluate_script(script).unwrap();
        describe(agent, &r, outcome)
    };
    for _ in 0..3 {
        assert_eq!(go(&mut agent, make), zero);
        assert_eq!(go(&mut agent, clear), zero);
        assert!(matches!(agent.collect(), Ok(HostOutcome::Normal(_))));
    }
    let room_now = |agent: &Agent<TestHost>| budget.saturating_sub(agent.heap_bytes().unwrap());
    let (mut target, mut streak, mut ended, mut before, mut answered) =
        (2048u64, 0u32, 0u32, 0u32, 0u32);
    let mut last = None;
    while streak < 8 && target < (32 << 10) {
        while room_now(&agent) < target {
            assert_eq!(go(&mut agent, pop), zero);
            assert!(matches!(agent.collect(), Ok(HostOutcome::Normal(_))));
        }
        let mut shift = 0;
        loop {
            let room = room_now(&agent);
            let units = (room.saturating_sub(target) / 2) >> shift;
            if room <= target + 64 || units == 0 {
                break;
            }
            PAD_LENGTH.store(units, Ordering::Relaxed);
            if go(&mut agent, push) == zero {
                shift = 0;
            } else {
                shift += 1;
            }
            assert!(matches!(agent.collect(), Ok(HostOutcome::Normal(_))));
        }
        let reading = agent.heap_bytes().unwrap();
        let mut u = room_now(&agent) * 3 / 8;
        loop {
            PAD_LENGTH.store(u, Ordering::Relaxed);
            let outcome = go(&mut agent, run);
            // What the run made, `h` with it, is garbage.
            assert!(matches!(agent.collect(), Ok(HostOutcome::Normal(_))));
            let began = go(&mut agent, started) == Outcome::Normal("1".to_string());
            if outcome == zero {
                streak += 1;
                answered += 1;
                assert_eq!(go(&mut agent, check), done, "{target} {u}");
            } else {
                assert_eq!(
                    outcome,
                    Outcome::Terminated(Termination::MemoryBudget),
                    "{target} {u}"
                );
                assert_eq!(go(&mut agent, unset), done, "{target} {u}");
                streak = 0;
                if began {
                    ended += 1;
                    last = Some((u, reading));
                } else {
                    before += 1;
                }
            }
            assert_eq!(go(&mut agent, clear), zero);
            assert!(matches!(agent.collect(), Ok(HostOutcome::Normal(_))));
            assert_eq!(
                agent.heap_bytes().unwrap(),
                reading,
                "{target} {u}: {outcome:?}"
            );
            if streak >= 8 || u < 4 {
                break;
            }
            u -= 4;
        }
        if streak < 8 {
            target += 512;
        }
    }
    println!(
        "{ended} ended in the construction, {before} before it, {answered} answered, last target {target}"
    );
    let (u, reading) = last.expect("no construction ended in the budget");
    assert!(answered > 0, "no construction answered");
    assert_eq!(
        agent.heap_bytes().unwrap(),
        reading,
        "the last failure was in an earlier round"
    );
    PAD_LENGTH.store(u, Ordering::Relaxed);
    for _ in 0..1_000 {
        assert_eq!(
            go(&mut agent, run),
            Outcome::Terminated(Termination::MemoryBudget)
        );
        assert!(matches!(agent.collect(), Ok(HostOutcome::Normal(_))));
    }
    assert_eq!(agent.heap_bytes().unwrap(), reading);
}

/// `this` read in the same arrow after that arrow's `super()`, in a derived
/// class with and without private methods and fields. The arrow's answer is kept in a local
/// first: `this.r = go()` evaluates `this` before the call, a `ReferenceError`.
#[test]
fn this_after_super_in_the_same_arrow() {
    let cases: Vec<(&str, String)> = vec![
        (
            "no private, this",
            format!(
                "{PARENT}class D extends P {{ constructor() {{ var go = () => {{ super(); return this; }}; var v = go(); this.r = v === this; }} }}\nnew D().r === true;"
            ),
        ),
        (
            "no private, this.p",
            format!(
                "{PARENT}class D extends P {{ constructor() {{ var go = () => {{ super(); return this.p; }}; var v = go(); this.r = v; }} }}\nnew D().r === 1;"
            ),
        ),
        (
            "no private, a method",
            format!(
                "{PARENT}class D extends P {{ m() {{ return 1; }} constructor() {{ var go = () => {{ super(); return this.m(); }}; var v = go(); this.r = v; }} }}\nnew D().r === 1;"
            ),
        ),
        (
            "private declared, this",
            format!(
                "{PARENT}class D extends P {{ #m() {{}} constructor() {{ var go = () => {{ super(); return this; }}; var v = go(); this.r = v === this; }} }}\nnew D().r === true;"
            ),
        ),
        (
            "private called after the arrow",
            format!(
                "{PARENT}class D extends P {{ #m() {{ return 1; }} constructor() {{ var go = () => {{ super(); }}; go(); this.r = this.#m(); }} }}\nnew D().r === 1;"
            ),
        ),
        (
            "private through a local",
            format!(
                "{PARENT}class D extends P {{ #m() {{ return 1; }} constructor() {{ var go = () => {{ super(); var t = this; return t.#m(); }}; var v = go(); this.r = v; }} }}\nnew D().r === 1;"
            ),
        ),
        (
            "private in the arrow",
            format!(
                "{PARENT}class D extends P {{ #m() {{ return 1; }} constructor() {{ var go = () => {{ super(); return this.#m(); }}; var v = go(); this.r = v; }} }}\nnew D().r === 1;"
            ),
        ),
        (
            "field, this in the arrow",
            format!(
                "{PARENT}class D extends P {{ f = 2; constructor() {{ var go = () => {{ super(); return this.f; }}; var v = go(); this.r = v; }} }}\nnew D().r === 2;"
            ),
        ),
    ];
    all_pass(&cases);
}

/// A direct `eval` in a field initializer of a class with private methods (`#sec-performeval`
/// with inClassFieldInitializer; the initializer's environment is the class's private record):
/// a name the class declares is the refusal of a direct eval, counted once; a name it does not
/// declare is the early `SyntaxError`; `arguments` is the error of ContainsArguments, also
/// beside a declared name in either order, so the declared name's refusal is no error of the
/// standard there and is neither thrown nor counted. In an instance field and a static one.
#[test]
fn a_direct_eval_in_a_field_initializer_meets_private_names() {
    let script = r#"function said(e) { return (e instanceof SyntaxError ? "SyntaxError: " : "other: ") + e.message; }
function tried(f) { try { f(); } catch (e) { return said(e); } return "none"; }
[tried(() => new (class { #m() { return 1; } x = eval("this.#m()"); })()),
 tried(() => new (class { #m() { return 1; } x = eval("this.#q"); })()),
 tried(() => new (class { #m() { return 1; } x = eval("arguments"); })()),
 tried(() => new (class { #m() {} x = eval("arguments; this.#m()"); })()),
 tried(() => new (class { #m() {} x = eval("this.#m(); arguments"); })()),
 tried(() => class { static #s() { return 1; } static x = eval("this.#s()"); }),
 tried(() => class { static #s() {} static x = eval("this.#t"); })].join("|");"#;
    let refused = |name: &str, at: &str| {
        format!(
            "SyntaxError: private name {name} in direct eval code is not supported before slice 8 ({at})"
        )
    };
    let arguments = |at: &str| {
        format!("SyntaxError: arguments in eval code in a class field initializer ({at})")
    };
    let wanted = [
        refused("#m", "1:6"),
        "SyntaxError: private name #q is not declared in an enclosing class (1:6)".to_string(),
        arguments("1:1"),
        arguments("1:1"),
        arguments("1:12"),
        refused("#s", "1:6"),
        "SyntaxError: private name #t is not declared in an enclosing class (1:6)".to_string(),
    ];
    let expected = Outcome::Normal(format!("\"{}\"", wanted.join("|")));
    for text in [script.to_string(), format!("\"use strict\";\n{script}")] {
        let mut agent = agent().unwrap();
        let r = realm(&mut agent);
        let before = agent.refusals();
        assert_eq!(eval_text(&mut agent, &r, &text), expected, "{text}");
        assert_eq!(agent.refusals() - before, 2, "{text}");
    }
}
