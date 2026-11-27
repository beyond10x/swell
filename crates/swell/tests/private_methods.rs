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
/// makes and drops: 50, and 5 with a collection at every safe point (for the time). A collection
/// that frees the instance's Private Names lets the classes made after it reuse their records,
/// which one class already does.
const GARBAGE: u32 = if cfg!(feature = "collect-every-safe-point") {
    5
} else {
    50
};

/// The collector marks an object's private elements at the object record's site: an instance
/// held by a global keeps its Private Names, its methods and its accessors' functions across
/// collections while the class itself is no longer reachable by name.
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
/// node, over the 1.5 s bound), and 10 with a collection at every safe point, which marks the
/// whole heap at each of the loop's instructions (the story's 1,000 ran over 60 s there, 50 took
/// 4.4 CPU s). Each instance adds the same two entries, so a list that is not charged fails at
/// every count.
fn instances() -> u64 {
    if cfg!(feature = "collect-every-safe-point") {
        10
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
/// without; a loop that keeps such instances under a small budget ends in
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
    held(&mut agent, &private);
    let with_private = held(&mut agent, &private);
    let with_plain = held(&mut agent, &plain);
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
           var keep = []; for (;;) keep.push(new P()); })();",
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
