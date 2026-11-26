//! Base classes (`#sec-class-definitions`): ClassDefinitionEvaluation
//! (`#sec-runtime-semantics-classdefinitionevaluation`) of a class without heritage, fields,
//! private names or static blocks: the class scope with its own name, the prototype, the
//! constructor F with `prototype` and `constructor`, every kind of method, static or not, with
//! literal or computed keys; the class binding in a block, in eval code and at the top of a
//! script (GlobalDeclarationInstantiation, `#sec-globaldeclarationinstantiation`); F's [[Call]]
//! and [[Construct]] (`#sec-ecmascript-function-objects-call-thisargument-argumentslist`,
//! `#sec-ecmascript-function-objects-construct-argumentslist-newtarget`); and
//! `Function.prototype.toString` of a class and of its methods. Written by
//! `story:class-definition-evaluation`. Run also with the feature `collect-every-safe-point`.
//!
//! Each script compares its results with `===` and answers `true`, as written and with a
//! `"use strict";` prologue; a script that checks that the code around a class stays
//! non-strict runs as written only.
#![cfg(test)]

use crate::support;

use support::host::{TestHost, agent, agent_with, global, key, limits, object, realm};
use support::script::{Outcome, describe, eval, eval_text, parse, run, run_both};
use swell::{Agent, Outcome as HostOutcome, Termination};

/// What a script that checks its own results answers.
fn passed() -> Outcome {
    Outcome::Normal("true".to_string())
}

/// `source` answers `true` as written and as strict code.
fn both_pass(source: &str) {
    assert_eq!(run_both(source), (passed(), passed()), "{source}");
}

/// ClassDefinitionEvaluation, steps 1 to 16 and 30: a declaration and an expression make a
/// constructor F whose `prototype` (not writable, not enumerable, not configurable) inherits
/// from `%Object.prototype%` and has `constructor` (writable, not enumerable, configurable); F
/// inherits from `%Function.prototype%`, has `length` from its constructor's parameters and its
/// name; the default constructor makes an instance; a named class expression's name is bound
/// inside it only.
#[test]
fn a_class_declaration_and_expression_make_a_constructor_and_prototype() {
    both_pass(
        "class A { constructor(x, y) { this.x = x; } m() { return this.x; } }\n\
         var B = class {};\n\
         var C = class Named { who() { return Named; } };\n\
         var a = new A(3);\n\
         var p = Object.getOwnPropertyDescriptor(A, \"prototype\");\n\
         var c = Object.getOwnPropertyDescriptor(A.prototype, \"constructor\");\n\
         typeof A === \"function\" && a.m() === 3 && a instanceof A\n\
           && Object.getPrototypeOf(A.prototype) === Object.prototype\n\
           && Object.getPrototypeOf(A) === Function.prototype\n\
           && p.value === A.prototype && !p.writable && !p.enumerable && !p.configurable\n\
           && c.value === A && c.writable && !c.enumerable && c.configurable\n\
           && A.name === \"A\" && A.length === 2 && B.name === \"B\" && B.length === 0\n\
           && C.name === \"Named\" && new C().who() === C && typeof Named === \"undefined\"\n\
           && new B() instanceof B && Object.getPrototypeOf(new B()) === B.prototype\n\
           && Object.getOwnPropertyNames(A).join() === \"length,name,prototype\"\n\
           && Object.getOwnPropertyNames(A.prototype).join() === \"constructor,m\"\n\
           && (class {}).name === \"\" && typeof class {} === \"function\";",
    );
}

/// ClassElementEvaluation and MethodDefinitionEvaluation (`#sec-runtime-semantics-classelementevaluation`,
/// `#sec-runtime-semantics-methoddefinitionevaluation`): a method, a getter, a setter, a
/// generator, an `async` and an `async` generator method are defined on the prototype, none
/// enumerable, each with the prototype as its [[HomeObject]], so `super` reads past it.
#[test]
fn every_method_kind_is_non_enumerable_with_its_home_object() {
    both_pass(
        "var seen = [];\n\
         class A {\n\
           m() { return super.who; }\n\
           get g() { return super.who; }\n\
           set s(v) { seen.push(super.who); }\n\
           *gen() { yield super.who; }\n\
           async am() { seen.push(super.who); }\n\
           async *ag() { seen.push(super.who); }\n\
         }\n\
         var base = { who: \"base\" };\n\
         Object.setPrototypeOf(A.prototype, base);\n\
         var a = new A();\n\
         a.s = 1; a.am(); a.ag().next();\n\
         var all = true;\n\
         [\"m\", \"g\", \"s\", \"gen\", \"am\", \"ag\"].forEach(function (n) {\n\
           var d = Object.getOwnPropertyDescriptor(A.prototype, n);\n\
           all = all && d.enumerable === false && d.configurable === true;\n\
         });\n\
         var g = Object.getOwnPropertyDescriptor(A.prototype, \"g\");\n\
         var s = Object.getOwnPropertyDescriptor(A.prototype, \"s\");\n\
         all && Object.keys(A.prototype).length === 0 && a.m() === \"base\" && a.g === \"base\"\n\
           && a.gen().next().value === \"base\" && seen.join() === \"base,base,base\"\n\
           && g.set === undefined && s.get === undefined && g.get.name === \"get g\"\n\
           && s.set.name === \"set s\" && A.prototype.gen.name === \"gen\"\n\
           && Object.getOwnPropertyDescriptor(A.prototype, \"m\").writable === true\n\
           && Object.getPrototypeOf(A.prototype.gen) !== Function.prototype;",
    );
}

/// ClassElementEvaluation with `static`: the methods and accessors are defined on F, not
/// enumerable, with F as their [[HomeObject]], so `super` reads F's [[Prototype]]; an instance
/// method of the same name stays on the prototype.
#[test]
fn static_methods_and_accessors_go_on_the_constructor() {
    both_pass(
        "var log = [];\n\
         class A {\n\
           static m() { return this; }\n\
           static get g() { return \"g\"; }\n\
           static set g(v) { log.push(v); }\n\
           static *gen() { yield super.x; }\n\
           static async am() { log.push(super.x); }\n\
           static async *ag() { log.push(super.x); }\n\
           static h() { return super.x; }\n\
           m() { return \"proto\"; }\n\
         }\n\
         Object.setPrototypeOf(A, { x: 7 });\n\
         A.g = 5; A.am(); A.ag().next();\n\
         var d = Object.getOwnPropertyDescriptor(A, \"g\");\n\
         A.m() === A && A.g === \"g\" && A.prototype.m() === \"proto\" && A.h() === 7\n\
           && A.gen().next().value === 7 && log.join() === \"5,7,7\"\n\
           && !Object.getOwnPropertyDescriptor(A, \"m\").enumerable && !d.enumerable\n\
           && d.configurable && d.get.name === \"get g\" && d.set.name === \"set g\"\n\
           && !A.prototype.hasOwnProperty(\"gen\") && !A.prototype.hasOwnProperty(\"h\")\n\
           && Object.keys(A).length === 0 && A.m.name === \"m\";",
    );
}

/// ClassDefinitionEvaluation, step 27, and ClassElementEvaluation: the computed keys run once
/// each, in source order, instance and static mixed, each converted by ToPropertyKey before the
/// next one runs; the method takes its name from the key; a key that throws stops the class
/// with the keys after it not run.
#[test]
fn computed_keys_run_in_order_with_their_side_effects() {
    both_pass(
        "var log = [];\n\
         function k(v) { log.push(v); return v; }\n\
         var obj = { toString() { log.push(\"ts\"); return \"o\"; } };\n\
         class A {\n\
           [k(\"a\")]() { return \"a\"; }\n\
           static [k(\"b\")]() { return \"b\"; }\n\
           get [k(\"c\")]() { return \"c\"; }\n\
           [obj]() { return \"o\"; }\n\
           [Symbol.iterator]() {}\n\
           static [k(1)]() { return 1; }\n\
         }\n\
         var a = new A();\n\
         var thrown = false;\n\
         try {\n\
           class B { [k(\"x\")]() {} [(function () { throw 9; })()]() {} [k(\"y\")]() {} }\n\
         } catch (e) { thrown = e === 9; }\n\
         log.join() === \"a,b,c,ts,1,x\" && thrown && a.a() === \"a\" && A.b() === \"b\"\n\
           && a.c === \"c\" && a.o() === \"o\" && A[1]() === 1 && A.prototype.a.name === \"a\"\n\
           && A.prototype[Symbol.iterator].name === \"[Symbol.iterator]\"\n\
           && Object.getOwnPropertyDescriptor(A.prototype, \"c\").get.name === \"get c\"\n\
           && A[\"1\"].name === \"1\" && !Object.getOwnPropertyDescriptor(A.prototype, \"o\").enumerable;",
    );
}

/// ClassDefinitionEvaluation, steps 3 and 31: the class scope binds the class's own name
/// immutable and uninitialized while the computed keys run (a read or a write there throws a
/// `ReferenceError`); after the class it is F and stays F, a write throwing a `TypeError` in
/// the class's strict code; the outer binding of a declaration is mutable.
#[test]
fn the_own_name_is_immutable_and_uninitialized_during_computed_keys() {
    both_pass(
        "function reference(f) { try { f(); } catch (e) { return e instanceof ReferenceError; } return false; }\n\
         var r1 = reference(function () { class C { [C]() {} } });\n\
         var r2 = reference(function () { var X = class C { [C = 1]() {} }; });\n\
         var r3 = reference(function () { var X = class C { static [typeof C]() {} }; });\n\
         class D { m() { D = 1; } }\n\
         var r4 = false;\n\
         try { new D().m(); } catch (e) { r4 = e instanceof TypeError; }\n\
         class H { m() { return H; } }\n\
         var h = H; H = null;\n\
         var F = class G { m() { try { G = 1; } catch (e) { return e instanceof TypeError && G === F; } } };\n\
         class E {} E = 2;\n\
         r1 && r2 && r3 && r4 && E === 2 && new F().m() === true && new h().m() === h\n\
           && H === null && typeof D === \"function\";",
    );
}

/// ClassElementEvaluation of a static method with a computed key `prototype`
/// (DefinePropertyOrThrow on F, whose `prototype` is not configurable): a `TypeError`, the
/// class not bound; an instance method of that key is defined on the prototype.
#[test]
fn a_computed_static_prototype_key_throws_a_type_error() {
    both_pass(
        "var r = [];\n\
         try { class A { static [\"prototype\"]() {} } } catch (e) { r.push(e instanceof TypeError); }\n\
         try { class B { static get [\"proto\" + \"type\"]() {} } } catch (e) { r.push(e instanceof TypeError); }\n\
         try { class C { static set [\"prototype\"](v) {} } } catch (e) { r.push(e instanceof TypeError); }\n\
         try { var X = class { static *[\"prototype\"]() {} }; } catch (e) { r.push(e instanceof TypeError); }\n\
         class D { [\"prototype\"]() { return 1; } }\n\
         r.join() === \"true,true,true,true\" && new D().prototype() === 1 && X === undefined;",
    );
}

/// [[Call]] of a class constructor (`#sec-ecmascript-function-objects-call-thisargument-argumentslist`,
/// step 2): a `TypeError` of F's realm, made before the constructor's code runs; through a
/// call, `call`, `apply`, a bound function and a builtin that calls it, and from a script of
/// another realm.
#[test]
fn calling_a_class_throws_a_type_error_of_its_realm() {
    both_pass(
        "var log = [];\n\
         class A { constructor() { log.push(1); } }\n\
         var B = class {};\n\
         function thrown(f) {\n\
           try { f(); } catch (e) { return e instanceof TypeError && e.constructor === TypeError; }\n\
           return false;\n\
         }\n\
         thrown(function () { A(); }) && thrown(function () { B(); })\n\
           && thrown(function () { A.call({}); }) && thrown(function () { A.apply(null, []); })\n\
           && thrown(function () { A.bind(null)(); }) && thrown(function () { [0].map(A); })\n\
           && log.length === 0 && typeof new A() === \"object\" && log.length === 1;",
    );
    let mut agent = agent().unwrap();
    let here = realm(&mut agent);
    let there = realm(&mut agent);
    let HostOutcome::Normal(class) = eval(&mut agent, &there, "(class C {});") else {
        panic!("the class of the other realm");
    };
    let HostOutcome::Normal(type_error) = eval(&mut agent, &there, "TypeError;") else {
        panic!("TypeError of the other realm");
    };
    let global_root = global(&mut agent, &here);
    agent
        .with_cx(&here, |cx| {
            let global = object(cx, &global_root);
            for (name, root) in [("C", &class), ("OtherTypeError", &type_error)] {
                let value = cx.get(root).unwrap();
                let name = key(cx, name);
                cx.set(global, name, value, true).unwrap().unwrap();
            }
        })
        .unwrap();
    assert_eq!(
        eval_text(
            &mut agent,
            &here,
            "var r = []; try { C(); } catch (e) { r.push(e instanceof OtherTypeError, e instanceof TypeError); }\n\
             try { [0].forEach(C); } catch (e) { r.push(e instanceof OtherTypeError, e instanceof TypeError); }\n\
             r.join() === \"true,false,true,false\";"
        ),
        passed()
    );
}

/// [[Construct]] of a base class constructor
/// (`#sec-ecmascript-function-objects-construct-argumentslist-newtarget`): `this` is made by
/// OrdinaryCreateFromConstructor from NewTarget (the class itself, the target of a bound
/// function, another constructor through `Reflect.construct`, `%Object.prototype%` when its
/// `prototype` is no object); a returned object replaces `this`, another value does not.
#[test]
fn new_makes_this_from_new_target() {
    both_pass(
        "class A { constructor(x) { this.t = new.target; this.x = x; } }\n\
         class D {}\n\
         function Other() {}\n\
         Other.prototype = { o: 1 };\n\
         function P() {}\n\
         P.prototype = 1;\n\
         var a = new A(1), b = new (A.bind(null, 2))(), r = Reflect.construct(A, [3], Other);\n\
         var p = Reflect.construct(A, [], P), d = new D(), s = new A(...[4]);\n\
         var C2 = A.bind(null).bind(null);\n\
         class R { constructor() { return { z: 1 }; } }\n\
         class S { constructor() { this.y = 1; return 1; } }\n\
         a.t === A && a.x === 1 && Object.getPrototypeOf(a) === A.prototype\n\
           && b.t === A && b.x === 2 && Object.getPrototypeOf(b) === A.prototype\n\
           && r.t === Other && r.x === 3 && r.o === 1\n\
           && Object.getPrototypeOf(p) === Object.prototype && p.t === P\n\
           && Object.getPrototypeOf(d) === D.prototype && s.x === 4 && s.t === A\n\
           && new C2().t === A && Object.getPrototypeOf(new C2()) === A.prototype\n\
           && new R().z === 1 && !(new R() instanceof R) && new S().y === 1 && new S() instanceof S;",
    );
}

/// A class declaration in a block, a function, a loop body and a `switch` binds lexically
/// there (BlockDeclarationInstantiation, `#sec-blockdeclarationinstantiation`), uninitialized
/// before it runs; in eval code it binds in the eval's own lexical environment
/// (EvalDeclarationInstantiation, `#sec-evaldeclarationinstantiation`), direct or indirect;
/// the declaration's completion is empty.
#[test]
fn a_class_binds_in_a_block_and_in_eval_code() {
    both_pass(
        "var r = [];\n\
         { class A { m() { return 1; } } r.push(new A().m() === 1); }\n\
         r.push(typeof A === \"undefined\");\n\
         r.push((function () { try { B; } catch (e) { return e instanceof ReferenceError; } class B {} return false; })());\n\
         r.push((function () { try { typeof B; } catch (e) { return e instanceof ReferenceError; } class B {} return false; })());\n\
         var made = [];\n\
         for (var i = 0; i < 2; i++) { class L {} made.push(L); }\n\
         r.push(made[0] !== made[1]);\n\
         switch (1) { case 1: class S { m() { return 2; } } r.push(new S().m() === 2); }\n\
         eval(\"class E { m() { return 3; } } r.push(new E().m() === 3);\");\n\
         r.push(typeof E === \"undefined\");\n\
         r.push(eval(\"1; class G {}\") === 1);\n\
         r.push(eval(\"class G {}\") === undefined);\n\
         r.push(typeof eval(\"(class {})\") === \"function\");\n\
         r.push((function () { eval(\"class H {}\"); return typeof H; })() === \"undefined\");\n\
         r.push((function () { var f = eval(\"class K { m() { return K; } } K\"); return new f().m() === f; })());\n\
         r.push((0, eval)(\"class I {} typeof I\") === \"function\");\n\
         r.push(typeof I === \"undefined\");\n\
         r.length === 15 && r.every(function (x) { return x === true; });",
    );
}

/// GlobalDeclarationInstantiation: a class at the top of a script binds in the global
/// environment's declarative record, not on the global object, uninitialized until the
/// declaration runs and seen by later scripts; a second script that declares the name again
/// (as a class or a `var`), a class whose name is a `let` of an earlier script or a `var`, or
/// a restricted global property, throws a `SyntaxError` before any of its code runs.
#[test]
fn a_top_level_class_binds_in_the_declarative_record_across_scripts() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let zero = || Outcome::Normal("0".to_string());
    let syntax = || Outcome::Throw("SyntaxError".to_string());
    let reference = || Outcome::Throw("ReferenceError".to_string());
    for (source, expected) in [
        (
            "var ran = 0; function f() { return Z; } class A { m() { return 1; } } 0;",
            zero(),
        ),
        (
            "new A().m() === 1 && !(\"A\" in globalThis) && typeof A === \"function\"\n\
               && Object.getOwnPropertyDescriptor(globalThis, \"A\") === undefined;",
            passed(),
        ),
        ("ran = 1; class A {}", syntax()),
        ("ran = 1; var A;", syntax()),
        ("let B = 1; var V; 0;", zero()),
        ("ran = 1; class B {}", syntax()),
        ("ran = 1; class V {}", syntax()),
        ("ran = 1; class undefined {}", syntax()),
        ("ran === 0;", passed()),
        ("X; class X {}", reference()),
        ("typeof X;", reference()),
        ("class Y {} Y = 1; Y;", Outcome::Normal("1".to_string())),
        ("Y;", Outcome::Normal("1".to_string())),
        ("class Z {} f() === Z;", passed()),
    ] {
        assert_eq!(eval_text(&mut agent, &realm, source), expected, "{source}");
    }
}

/// `Function.prototype.toString` (`#sec-function.prototype.tostring`): a class's [[SourceText]]
/// is its source from `class` to its `}`, the constructor's own `toString` the same, and a
/// method's is its MethodDefinition: from its first token, `get`, `set`, `async` and `*`
/// included, `static` not (ClassElement : `static` MethodDefinition).
#[test]
fn to_string_gives_the_source_text_of_a_class_and_its_methods() {
    both_pass(
        r#"class /* a */ A /* b */ { /* c */ m /* d */ () {} static /* e */ s() {} get g() {} static async *ag() {} static static() {} static() {} }
var src = "class /* a */ A /* b */ { /* c */ m /* d */ () {} static /* e */ s() {} get g() {} static async *ag() {} static static() {} static() {} }";
var B = class {};
var T = class   T2   { constructor ( a ) { } };
var K = class { static [ "k" ] () {} static
  *gen() {} };
A.toString() === src && A.prototype.constructor.toString() === src
  && Function.prototype.toString.call(A) === src
  && A.prototype.m.toString() === "m /* d */ () {}"
  && A.s.toString() === "s() {}"
  && Object.getOwnPropertyDescriptor(A.prototype, "g").get.toString() === "get g() {}"
  && A.ag.toString() === "async *ag() {}"
  && A.static.toString() === "static() {}"
  && A.prototype.static.toString() === "static() {}"
  && B.toString() === "class {}"
  && T.toString() === "class   T2   { constructor ( a ) { } }"
  && K.k.toString() === '[ "k" ] () {}' && K.gen.toString() === "*gen() {}";"#,
    );
}

/// SuperProperty in the methods of a class (`#sec-super-keyword-runtime-semantics-evaluation`):
/// read, written and called through the [[Prototype]] of the home object, with the method's
/// `this` as the receiver; the constructor's home object is the prototype.
#[test]
fn super_property_in_a_class_method_reads_the_prototype_chain() {
    both_pass(
        "class A {\n\
           constructor() { this.own = super.hasOwnProperty === Object.prototype.hasOwnProperty; }\n\
           m() { return super.toString === Object.prototype.toString; }\n\
           static s() { return super.call === Function.prototype.call; }\n\
         }\n\
         var base = { x: 1, get y() { return this.z; }, f() { return this.z + 1; } };\n\
         class B { x() { return super.x; } y() { return super.y; } set(v) { super.w = v; }\n\
                   f() { return super.f(); } k() { return super[\"x\"]; } }\n\
         Object.setPrototypeOf(B.prototype, base);\n\
         var b = new B(); b.z = 9; b.set(4);\n\
         new A().own && new A().m() && A.s() && b.x() === 1 && b.y() === 9 && b.w === 4\n\
           && !base.hasOwnProperty(\"w\") && b.f() === 10 && b.k() === 1;",
    );
}

/// How many classes each round of the loop of [`a_class_made_in_a_loop_is_charged_and_freed`]
/// makes: the story's 10,000 with `SWELL_FULL_COUNTS=1` and 2,000 in the gate (two rounds of
/// 10,000 took 1.5 to 1.8 CPU s alone on the build node, over the 1.5 s bound); 20 with a
/// collection at every safe point, which marks the whole heap (about 380 KB, the realm's) at
/// each of the loop's instructions: 1,000 ran 45 s there. Each iteration runs the same
/// instructions and the reading after the loop must equal the one before it to the byte, so a
/// class that leaks one byte fails at every count.
fn classes() -> u32 {
    if cfg!(feature = "collect-every-safe-point") {
        20
    } else if std::env::var("SWELL_FULL_COUNTS").is_ok_and(|value| value == "1") {
        10_000
    } else {
        2_000
    }
}

/// How many classes and functions the reading of the box's charge holds: 100, and 10 with a
/// collection at every safe point (for the time, as [`classes`]).
const HELD: u64 = if cfg!(feature = "collect-every-safe-point") {
    10
} else {
    100
};

/// The room above a fresh agent and realm that the memory budget of
/// [`a_class_made_in_a_loop_is_charged_and_freed`] leaves: 256 KiB, a few hundred classes, and
/// 16 KiB with a collection at every safe point (for the time, as [`classes`]).
const ROOM: u64 = if cfg!(feature = "collect-every-safe-point") {
    16 * 1024
} else {
    256 * 1024
};

/// "Every allocation of a class is charged before it exists" and decision 4, "Memory budget":
/// a loop of [`classes`] classes, each with an instance made and its methods called, leaves,
/// once the script dropped what it held, the reading of `heap_bytes()` after a `collect()` it
/// had before the loop (taken after a first round, which grows the arenas): F, its box, its
/// prototype and its methods are credited. [`HELD`] classes held take at least 8 bytes each
/// more than as many functions with their prototypes: the box is charged with F. A loop that
/// keeps classes under a small budget ends in `Terminated(MemoryBudget)`, and leaks nothing.
#[test]
fn a_class_made_in_a_loop_is_charged_and_freed() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var i = 0, k = null, o = null, keep = null; 0;"
        ),
        Outcome::Normal("0".to_string())
    );
    let count = classes();
    let looped = parse(
        &mut agent,
        &realm,
        &format!(
            "for (i = 0; i < {count}; i++) {{\n\
               k = class {{ m() {{ return 1; }} static s() {{}} get g() {{ return 2; }} }};\n\
               o = new k(); if (o.m() + o.g !== 3) throw 0;\n\
             }} i;"
        ),
    );
    let dropped = parse(&mut agent, &realm, "k = null; o = null; keep = null; 0;");
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
    // The box: classes held against functions held, each with its prototype and its name.
    let classes = parse(
        &mut agent,
        &realm,
        &format!("keep = []; for (i = 0; i < {HELD}; i++) keep.push(class {{}}); 0;"),
    );
    let functions = parse(
        &mut agent,
        &realm,
        &format!("keep = []; for (i = 0; i < {HELD}; i++) keep.push(function () {{}}); 0;"),
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
    held(&mut agent, &classes);
    held(&mut agent, &functions);
    let with_classes = held(&mut agent, &classes);
    let with_functions = held(&mut agent, &functions);
    println!(
        "heap_bytes: {with_classes} with {HELD} classes held, {with_functions} with functions"
    );
    assert!(
        with_classes >= with_functions + 8 * HELD,
        "{with_classes} with classes, {with_functions} with functions"
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
        "(function () { var keep = []; for (;;) keep.push(class { m() {} static s() {} }); })();",
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

/// How many methods [`a_class_with_1000_methods`] defines: 1,000, and 40 with a collection at
/// every safe point, which marks the whole heap, every method of the class in it, at each of
/// the five instructions a method's definition takes (1,000 ran over 60 s there, 100 took
/// 2.4 CPU s, over the 2 s bound). Each method is defined by the same instructions, so 40
/// still holds every definition, its order and its call; the default mode runs the 1,000.
const METHODS: u32 = if cfg!(feature = "collect-every-safe-point") {
    40
} else {
    1_000
};

/// A class with [`METHODS`] methods: each defined on the prototype in source order and
/// callable.
#[test]
fn a_class_with_1000_methods() {
    let n = METHODS;
    let mut source = String::from("class C {\n");
    for i in 0..n {
        source.push_str(&format!("  m{i}() {{ return {i}; }}\n"));
    }
    let last = n - 1;
    let sum = n * last / 2;
    source.push_str(&format!(
        "}}\n\
         var c = new C(), sum = 0, names = Object.getOwnPropertyNames(C.prototype);\n\
         for (var i = 0; i < {n}; i++) sum += c[\"m\" + i]();\n\
         names.length === {n} + 1 && names[1] === \"m0\" && names[{n}] === \"m{last}\"\n\
           && sum === {sum} && c.m{last}() === {last};"
    ));
    both_pass(&source);
}

/// A method, a static method, a getter and a constructor that recurse without end stop at the
/// frame depth or the re-entry depth with a `RangeError` (decision 4), which a `catch` sees;
/// the class works after it. The test's limits; with a collection at every safe point, which
/// marks every frame at each instruction, a frame depth of 100, a re-entry depth of 8 and
/// 5,000 steps, twice what the script takes there (2,500 pass, 2,000 do not): a construction
/// that misses the frame-depth check then ends on the step budget in about 3 s, where it ran
/// into the 60 s bound before (in the default mode it ends on the memory budget).
#[test]
fn a_method_recursion_stops_at_frame_depth() {
    let mut limits = limits();
    if cfg!(feature = "collect-every-safe-point") {
        limits.frame_depth = 100;
        limits.reentry_depth = 8;
        limits.steps_per_entry = 5_000;
    }
    let source = "class A { m(n) { return this.m(n + 1); } static s(n) { return A.s(n + 1); }\n\
                   get g() { return this.g; } }\n\
         class B { constructor() { new B(); } }\n\
         function range(f) { try { f(); } catch (e) { return e instanceof RangeError; } return false; }\n\
         range(function () { new A().m(0); }) && range(function () { A.s(0); })\n\
           && range(function () { new B(); }) && range(function () { return new A().g; })\n\
           && new A() instanceof A;";
    assert_eq!(run(limits, source), passed(), "{source}");
    let strict = format!("\"use strict\";\n{source}");
    assert_eq!(run(limits, &strict), passed(), "{strict}");
}

/// `#sec-strict-mode-code`: "All parts of a ClassDeclaration or a ClassExpression are strict
/// mode code", the computed keys included. In a class written in non-strict code, a key that
/// assigns an undeclared name throws a `ReferenceError` (PutValue, `#sec-putvalue`, step 3.a),
/// a failed [[Set]] throws a `TypeError` (step 5.d), and a direct `eval` in a key is strict
/// (PerformEval, `#sec-performeval`, step 10), so its `var` stays in the eval's own
/// environment. Found by the adversary's first pass.
#[test]
fn computed_keys_are_strict_code() {
    both_pass(
        "var r = [];\n\
         try { class A { [(undeclaredInKey = 1, \"k\")]() {} } r.push(\"no\"); } catch (e) { r.push(e instanceof ReferenceError); }\n\
         var frozen = Object.freeze({ x: 1 });\n\
         try { class B { [(frozen.x = 2, \"k\")]() {} } r.push(\"no\"); } catch (e) { r.push(e instanceof TypeError); }\n\
         class C { [eval(\"var leaked = 1; 'k'\")]() {} }\n\
         r.push(typeof leaked === \"undefined\", typeof undeclaredInKey === \"undefined\");\n\
         r.join() === \"true,true,true,true\";",
    );
}

/// NamedEvaluation of an anonymous class under a computed key of an object literal
/// (`#sec-runtime-semantics-propertydefinitionevaluation`, step 5 of `PropertyDefinition :
/// PropertyName : AssignmentExpression`) passes the key into ClassDefinitionEvaluation, whose
/// SetFunctionName(F, className) runs before the static elements; a static `name` method or
/// accessor of the class then replaces it, as it does under a literal key. Found by the
/// adversary's first pass.
#[test]
fn a_static_name_wins_over_a_computed_key_name() {
    both_pass(
        "var o = {\n\
           [\"a\"]: class { static name() { return 1; } },\n\
           [\"b\"]: class { static get name() { return \"getter\"; } },\n\
           [\"c\"]: class { static [\"na\" + \"me\"]() {} },\n\
           [\"d\"]: class {}\n\
         };\n\
         var p = { e: class { static name() { return 1; } } };\n\
         typeof o.a.name === \"function\" && o.b.name === \"getter\" && typeof o.c.name === \"function\"\n\
           && o.d.name === \"d\" && typeof p.e.name === \"function\"\n\
           && Object.getOwnPropertyNames(o.a).join() === \"length,name,prototype\";",
    );
}

/// `source` answers `true` as written, in non-strict code: what it checks holds there only
/// (the code around a class stays non-strict).
fn sloppy_passes(source: &str) {
    assert_eq!(run(limits(), source), passed(), "{source}");
}

/// `source` run as written in a fresh agent, then `probe` read after one microtask checkpoint:
/// `true`.
fn sloppy_settles(source: &str, probe: &str) {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let ran = eval_text(&mut agent, &realm, source);
    assert!(matches!(ran, Outcome::Normal(_)), "{source}: {ran:?}");
    let checkpoint = agent.perform_microtask_checkpoint().unwrap();
    assert!(
        matches!(checkpoint, HostOutcome::Normal(_)),
        "{checkpoint:?}"
    );
    assert_eq!(eval_text(&mut agent, &realm, probe), passed(), "{source}");
}

/// The head of the scripts of the strictness tests: the list of results, a frozen object and
/// a key that throws.
const STRICT_HEAD: &str = "var r = [], frozen = Object.freeze({ x: 1 });\n\
     function thrower() { throw new Error(\"key\"); }\n";

/// The answer of the strictness tests: `true` when every result is `true`, the results
/// otherwise.
const STRICT_ANSWER: &str = "r.every(function (x) { return x === true; }) || r.join();";

/// Statements that run without an error only in non-strict code, numbered `n`: an assignment
/// to an undeclared name, a write to a frozen object, `delete` of a non-configurable property,
/// a direct `eval` that declares a `var` and one that assigns an undeclared name; then the
/// push of whether each did what it does in non-strict code.
fn sloppy_statements(n: u32) -> String {
    "a@ = 1; frozen.x = 2; delete Object.prototype; eval(\"var b@ = 1\"); eval(\"c@ = 1\");\n\
     r.push(typeof a@ === \"number\" && frozen.x === 1 && typeof b@ === \"number\"\n\
       && typeof c@ === \"number\" || \"strict at @\");\n"
        .replace('@', &n.to_string())
}

/// `#sec-strict-mode-code`, the computed key of every kind of class element in non-strict
/// code, at the top of a script and in a non-strict function: an assignment to an undeclared
/// name throws a `ReferenceError` (PutValue, step 3.a), a write to a frozen object and
/// `delete` of a non-configurable property a `TypeError` (step 5.d,
/// `#sec-delete-operator-runtime-semantics-evaluation`, step 5.d), an assignment to the name of
/// a named function expression a `TypeError` (SetMutableBinding of an immutable binding,
/// `#sec-declarative-environment-records-setmutablebinding-n-v-s`, step 4), a direct `eval`
/// is strict eval code (PerformEval, step 10): its `var` stays in its own environment and its
/// assignment to an undeclared name throws a `ReferenceError`.
#[test]
fn every_strict_operation_in_a_computed_key_is_strict() {
    let positions = [
        ("", "() {}"),
        ("get ", "() {}"),
        ("set ", "(v) {}"),
        ("static ", "() {}"),
        ("static get ", "() {}"),
        ("static set ", "(v) {}"),
    ];
    let mut n = 0;
    let mut source = String::from(STRICT_HEAD);
    for context in ["script", "function"] {
        let mut body = String::new();
        for (before, after) in positions {
            let class = |key: &str| format!("class C {{ {before}[{key}]{after} }}");
            let threw = |n: u32, code: String, error: &str| {
                format!(
                    "try {{ {code} r.push(\"none {n}\"); }} catch (e) {{ r.push(e instanceof {error} || \"wrong {n} \" + e); }}\n"
                )
            };
            n += 1;
            body.push_str(&threw(
                n,
                class(&format!("(u{n} = 1, \"k\")")),
                "ReferenceError",
            ));
            n += 1;
            body.push_str(&threw(n, class("(frozen.x = 2, \"k\")"), "TypeError"));
            n += 1;
            body.push_str(&threw(
                n,
                class("(delete Object.prototype, \"k\")"),
                "TypeError",
            ));
            n += 1;
            let nfe = format!(
                "(function nfe{n}() {{ {} }})();",
                class(&format!("(nfe{n} = 1, \"k\")"))
            );
            body.push_str(&threw(n, nfe, "TypeError"));
            n += 1;
            body.push_str(&threw(
                n,
                class(&format!("eval(\"ue{n} = 1; 'k'\")")),
                "ReferenceError",
            ));
            n += 1;
            let leak = class(&format!("eval(\"var leaked{n} = 1; 'k'\")"));
            body.push_str(&format!(
                "{{ {leak} }}\nr.push(typeof leaked{n} === \"undefined\" || \"leaked {n}\");\n"
            ));
        }
        if context == "script" {
            source.push_str(&body);
        } else {
            source.push_str(&format!("(function () {{\n{body}}})();\n"));
        }
    }
    source.push_str(&format!("r.length === {n} && {STRICT_ANSWER}"));
    both_pass(&source);
}

/// The code around a class in non-strict code stays non-strict (`#sec-strict-mode-code`: only
/// the parts of the class are strict): after a class that completed, in the `catch` and the
/// `finally` that a throw of a key enters, in a `finally` that returns after a key threw, in
/// the `finally` of a `return` of a class, and after the call of such a function; at the top
/// of a script and in a non-strict function, whose own name stays immutable without an error.
#[test]
fn the_code_around_a_class_stays_non_strict() {
    let mut source = String::from(STRICT_HEAD);
    for context in ["script", "function"] {
        let base = if context == "script" { 0 } else { 10 };
        let s = |k: u32| sloppy_statements(base + k);
        let mut body = String::new();
        body.push_str(&format!(
            "class A {{ [(\"m\")]() {{}} static [\"s\"]() {{}} }}\n{}",
            s(1)
        ));
        body.push_str(&format!(
            "try {{ class B {{ [thrower()]() {{}} }} }} catch (e) {{ {} }}\n",
            s(2)
        ));
        body.push_str(&format!(
            "try {{ try {{ class B {{ [thrower()]() {{}} }} }} finally {{ {} }} }} catch (e) {{}}\n",
            s(3)
        ));
        if context == "function" {
            body.push_str(&format!(
                "function f() {{ try {{ class D {{ [thrower()]() {{}} }} }} finally {{ {} return 1; }} }}\n\
                 r.push(f() === 1);\n{}",
                s(4),
                s(5)
            ));
            body.push_str(&format!(
                "function g() {{ try {{ return class {{ [\"k\"]() {{}} }}; }} finally {{ {} }} }}\n\
                 r.push(typeof g() === \"function\");\n{}",
                s(6),
                s(7)
            ));
            body.push_str(
                "outer = 1; r.push(typeof outer === \"function\" || \"outer assigned\");\n",
            );
            source.push_str(&format!("(function outer() {{\n{body}}})();\n"));
        } else {
            source.push_str(&body);
        }
    }
    source.push_str(&format!("r.length === 13 && {STRICT_ANSWER}"));
    sloppy_passes(&source);
}

/// A handler inside class code keeps the class's strictness, and one outside it the code's:
/// in a non-strict generator, `throw()` sent into `yield*` in a key is forwarded to the inner
/// generator, which answers, and the rest of the key and a second key are strict; the inner
/// generator that does not answer makes the handler of the `yield*` throw again into a `catch`
/// outside the class, which is non-strict; a destructuring assignment in a key completes and
/// the rest of the key is strict, and one whose default throws enters its handler, which closes
/// the iterator and throws again into a non-strict `catch`.
#[test]
fn a_handler_keeps_the_strictness_of_its_place() {
    let source = format!(
        "{STRICT_HEAD}\
         function* inner() {{ try {{ yield 1; }} catch (e) {{ return \"k\"; }} }}\n\
         function* g1() {{ class C {{ [(yield* inner(), u1 = 1, \"a\")]() {{}} }} }}\n\
         var it = g1(); it.next();\n\
         try {{ it.throw(new Error(\"in\")); r.push(\"none 1\"); }} catch (e) {{ r.push(e instanceof ReferenceError || \"wrong 1 \" + e); }}\n\
         function* g2() {{ class C {{ [yield* inner()]() {{}} [(u2 = 1, \"b\")]() {{}} }} }}\n\
         it = g2(); it.next();\n\
         try {{ it.throw(new Error(\"in\")); r.push(\"none 2\"); }} catch (e) {{ r.push(e instanceof ReferenceError || \"wrong 2 \" + e); }}\n\
         function* inner2() {{ yield 1; }}\n\
         function* g3() {{ try {{ class C {{ [yield* inner2()]() {{}} }} }} catch (e) {{ {} yield \"caught\"; }} }}\n\
         it = g3(); it.next(); r.push(it.throw(new Error(\"x\")).value === \"caught\");\n\
         var a, b, closed = 0;\n\
         try {{ class D {{ [([a, b] = [1, 2], u4 = 1, \"d\")]() {{}} }} r.push(\"none 4\"); }} catch (e) {{ r.push(e instanceof ReferenceError && a === 1 || \"wrong 4 \" + e); }}\n\
         var iterable = {{ [Symbol.iterator]() {{ return {{ next() {{ return {{ value: undefined, done: false }}; }}, return() {{ closed++; return {{}}; }} }}; }} }};\n\
         try {{ class E {{ [([a = thrower()] = iterable, \"e\")]() {{}} }} }} catch (e) {{ {} }}\n\
         r.push(closed === 1);\n\
         r.length === 7 && {STRICT_ANSWER}",
        sloppy_statements(3),
        sloppy_statements(5),
    );
    sloppy_passes(&source);
}

/// A generator, an `async` function and an async generator suspended in a computed key resume
/// in strict code: by `next` (or a fulfilled `await`), the rest of the key is strict; by
/// `throw` (or a rejected `await`) caught outside the class, the `catch` is non-strict; by
/// `return`, the `finally` outside the class is non-strict.
#[test]
fn a_frame_suspended_in_a_key_resumes_strict() {
    let source = format!(
        "{STRICT_HEAD}\
         function* g1() {{ class C {{ [(yield 1, u1 = 1, \"a\")]() {{}} }} }}\n\
         var it = g1(); it.next();\n\
         try {{ it.next(); r.push(\"none 1\"); }} catch (e) {{ r.push(e instanceof ReferenceError || \"wrong 1 \" + e); }}\n\
         function* g2() {{ try {{ class C {{ [(yield 1, \"a\")]() {{}} }} }} catch (e) {{ {} }} }}\n\
         it = g2(); it.next(); it.throw(new Error(\"t\"));\n\
         function* g3() {{ try {{ class C {{ [(yield 1, \"a\")]() {{}} }} }} finally {{ {} }} }}\n\
         it = g3(); it.next(); r.push(it.return(7).value === 7);\n\
         async function af1() {{ class C {{ [(await 0, u4 = 1, \"a\")]() {{}} }} }}\n\
         af1().then(function () {{ r.push(\"none 4\"); }}, function (e) {{ r.push(e instanceof ReferenceError || \"wrong 4 \" + e); }});\n\
         async function af2() {{ try {{ class C {{ [(await Promise.reject(new Error(\"r\")), \"a\")]() {{}} }} }} catch (e) {{ {} }} }}\n\
         af2();\n\
         async function* ag() {{ try {{ class C {{ [(yield 1, \"a\")]() {{}} }} }} finally {{ {} }} }}\n\
         var ai = ag(); ai.next().then(function () {{ return ai.return(8); }}).then(function (x) {{ r.push(x.value === 8); }});\n\
         0;",
        sloppy_statements(2),
        sloppy_statements(3),
        sloppy_statements(5),
        sloppy_statements(6),
    );
    sloppy_settles(&source, &format!("r.length === 8 && {STRICT_ANSWER}"));
}

/// A class in a computed key of a class, and a function there, are class code, strict: the key
/// of the inner class and the rest of the outer key after the inner class are strict, and the
/// code after the outer class is not; a function in a key is strict, so `this` in its call is
/// `undefined` (OrdinaryCallBindThis, `#sec-ordinarycallbindthis`, step 5); at the top of a
/// script and in a non-strict function.
#[test]
fn a_class_or_function_in_a_key_is_strict() {
    let mut source = String::from(STRICT_HEAD);
    for context in ["script", "function"] {
        let n = if context == "script" { 0 } else { 10 };
        let body = format!(
            "try {{ class O {{ [(class I {{ [(u{a} = 1, \"i\")]() {{}} }}, \"o\")]() {{}} }} r.push(\"none {a}\"); }}\n\
               catch (e) {{ r.push(e instanceof ReferenceError || \"wrong {a} \" + e); }}\n\
             try {{ class O {{ [(class I {{ [\"i\"]() {{}} }}, u{b} = 1, \"o\")]() {{}} }} r.push(\"none {b}\"); }}\n\
               catch (e) {{ r.push(e instanceof ReferenceError || \"wrong {b} \" + e); }}\n\
             class O3 {{ [(class I {{ [\"i\"]() {{}} }}, \"o\")]() {{}} }}\n\
             {}\
             class F {{ [(function () {{ return this; }})() === undefined ? \"yes\" : \"no\"]() {{}} }}\n\
             r.push(typeof F.prototype.yes === \"function\" || \"a non-strict function in a key\");\n",
            sloppy_statements(n + 3),
            a = n + 1,
            b = n + 2,
        );
        if context == "script" {
            source.push_str(&body);
        } else {
            source.push_str(&format!("(function () {{\n{body}}})();\n"));
        }
    }
    source.push_str(&format!("r.length === 8 && {STRICT_ANSWER}"));
    sloppy_passes(&source);
}

/// NamedEvaluation under a computed key of an object literal: ClassDefinitionEvaluation gives F
/// its name before any element is defined (SetFunctionName(F, className) after
/// MakeClassConstructor, `#sec-runtime-semantics-classdefinitionevaluation`), so F's own keys
/// are `length`, `name`, `prototype`, then its static elements: a Symbol key gives
/// `[description]`, a Symbol without one the empty String, a number its String; a static `name`
/// replaces it. An anonymous function expression and an arrow function there keep their name
/// from the key (`#sec-setfunctionname`) and their own keys.
#[test]
fn a_computed_key_names_its_anonymous_class_before_the_elements() {
    both_pass(
        "var s1 = Symbol(\"desc\"), s2 = Symbol();\n\
         var o = { [s1]: class { static m() {} }, [s2]: class {}, [1]: class { static name() {} },\n\
                   [2]: class { static get name() { return \"g\"; } }, [3]: class { static z() {} } };\n\
         var f = { [s1]: function () {}, [s2]: () => {}, [\"g\"]: function () {}, [4]: () => {} };\n\
         var keys = function (x) { return Reflect.ownKeys(x).map(String).join(); };\n\
         o[s1].name === \"[desc]\" && keys(o[s1]) === \"length,name,prototype,m\"\n\
           && o[s2].name === \"\" && keys(o[s2]) === \"length,name,prototype\"\n\
           && typeof o[1].name === \"function\" && keys(o[1]) === \"length,name,prototype\"\n\
           && o[2].name === \"g\" && keys(o[2]) === \"length,name,prototype\"\n\
           && o[3].name === \"3\" && keys(o[3]) === \"length,name,prototype,z\"\n\
           && f[s1].name === \"[desc]\" && keys(f[s1]) === \"length,name,prototype\"\n\
           && f[s2].name === \"\" && keys(f[s2]) === \"length,name\"\n\
           && f.g.name === \"g\" && f[4].name === \"4\" && keys(f[4]) === \"length,name\";",
    );
}

/// `Function.prototype.toString` of a static method (`#sec-function.prototype.tostring`, the
/// MethodDefinition after `static`): `static` and the trivia after it are left out, a line
/// comment and a white space outside ASCII (U+00A0) among them; `static` followed by an escape
/// is the start of a longer name, and that method's text keeps it. Holds three changes of
/// `method_definition_start` that the adversary's first pass found no test for.
#[test]
fn to_string_of_a_static_method_skips_static_and_its_trivia() {
    both_pass(
        "class A { static // c\n m() {} static\u{A0}n() {} static\\u0061b() {} }\n\
         A.m.toString() === \"m() {}\" && A.n.toString() === \"n() {}\"\n\
           && A.prototype.staticab.toString() === \"static\\\\u0061b() {}\";",
    );
}
