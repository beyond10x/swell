//! Derived classes (`#sec-class-definitions`): the heritage of ClassDefinitionEvaluation
//! (`#sec-runtime-semantics-classdefinitionevaluation`, steps 8 and 15 to 18), SuperCall
//! (`#sec-super-keyword-runtime-semantics-evaluation`, GetSuperConstructor
//! `#sec-getsuperconstructor`, BindThisValue `#sec-bindthisvalue`), the `this` of a derived
//! constructor before and after `super()`, the derived [[Construct]]
//! (`#sec-ecmascript-function-objects-construct-argumentslist-newtarget`, steps 10 to 12) and
//! its realm, the default derived constructor, `super` in an arrow function and in direct eval
//! code (PerformEval, `#sec-performeval`, steps 7 to 11), subclasses of builtins and the bounds
//! of a chain of `super()` calls. Written by `story:derived-classes-and-super-call`. Run also
//! with the feature `collect-every-safe-point`.
//!
//! Each script compares its results with `===` and answers `true`, as written and with a
//! `"use strict";` prologue.
#![cfg(test)]

use crate::support;

use support::host::{agent, agent_with, global, key, limits, object, realm};
use support::script::{Outcome, define_function, eval, eval_text, run, run_both};
use support::stack::{is_range_error_of, reenter};
use swell::{Agent, Outcome as HostOutcome, Termination};

/// What a script that checks its own results answers.
fn passed() -> Outcome {
    Outcome::Normal("true".to_string())
}

/// `source` answers `true` as written and as strict code.
fn both_pass(source: &str) {
    assert_eq!(run_both(source), (passed(), passed()), "{source}");
}

/// ClassDefinitionEvaluation, steps 8.e to 8.h: with a class as the heritage, the prototype
/// inherits from the parent's `prototype` and F from the parent; `new` of the derived class
/// runs the parent's constructor with NewTarget the derived class, so the instance inherits
/// from the derived prototype (OrdinaryCreateFromConstructor of NewTarget, through
/// `push_construct_frame`'s NewTarget), and the parent's code sees that NewTarget; a static
/// method is inherited; the heritage is evaluated in the class scope, where the class's own
/// name is uninitialized.
#[test]
fn extends_a_class_sets_both_prototype_chains() {
    both_pass(
        "var seen = [];\n\
         class A { constructor(x) { seen.push(new.target); this.x = x; } static s() { return 'A.s'; } m() { return 'A.m'; } }\n\
         class B extends A { constructor(x) { super(x + 1); this.y = x; } }\n\
         var C = class extends B {};\n\
         var b = new B(1), c = new C(5);\n\
         var tdz; try { (class D extends (D, A) {}); } catch (e) { tdz = e instanceof ReferenceError; }\n\
         Object.getPrototypeOf(B.prototype) === A.prototype && Object.getPrototypeOf(B) === A\n\
           && Object.getPrototypeOf(C.prototype) === B.prototype && Object.getPrototypeOf(C) === B\n\
           && Object.getPrototypeOf(b) === B.prototype && b.x === 2 && b.y === 1 && b.m() === 'A.m'\n\
           && Object.getPrototypeOf(c) === C.prototype && c.x === 6 && c.y === 5\n\
           && seen[0] === B && seen[1] === C && B.s() === 'A.s' && C.s() === 'A.s'\n\
           && b instanceof A && c instanceof B && tdz === true\n\
           && Object.getOwnPropertyNames(B.prototype).join() === 'constructor'\n\
           && B.prototype.constructor === B && C.name === 'C' && C.length === 0;",
    );
}

/// The heritage may be any constructor: an ordinary function runs as the parent with `this`
/// made from NewTarget, so it sees the derived class as `new.target` and the instance inherits
/// from the derived prototype.
#[test]
fn extends_a_function() {
    both_pass(
        "function F(a, b) { this.t = new.target; this.sum = a + b; }\n\
         F.prototype.f = function () { return this.sum; };\n\
         class B extends F { constructor() { super(1, 2); this.own = true; } }\n\
         class C extends F {}\n\
         var b = new B(), c = new C(3, 4);\n\
         Object.getPrototypeOf(B) === F && Object.getPrototypeOf(B.prototype) === F.prototype\n\
           && b.t === B && b.f() === 3 && b.own === true && Object.getPrototypeOf(b) === B.prototype\n\
           && c.t === C && c.sum === 7 && c instanceof F && Object.getPrototypeOf(c) === C.prototype;",
    );
}

/// ClassDefinitionEvaluation, step 8.e: `extends null` makes a prototype without parent and
/// F's parent `%Function.prototype%`; `new` of its default constructor throws a `TypeError`
/// (GetSuperConstructor answers `%Function.prototype%`, no constructor: SuperCall step 6), and
/// a constructor that returns an object of its own works.
#[test]
fn extends_null_makes_a_prototype_without_parent_and_construct_throws() {
    both_pass(
        "class N extends null {}\n\
         class M extends null { constructor() { return Object.create(M.prototype); } }\n\
         var thrown; try { new N(); } catch (e) { thrown = e instanceof TypeError; }\n\
         var explicit; try { new (class extends null { constructor() { super(); } })(); }\n\
           catch (e) { explicit = e instanceof TypeError; }\n\
         Object.getPrototypeOf(N.prototype) === null && Object.getPrototypeOf(N) === Function.prototype\n\
           && thrown === true && explicit === true && new M() instanceof M\n\
           && Object.getPrototypeOf(new M()) === M.prototype;",
    );
}

/// ClassDefinitionEvaluation, steps 8.f and 8.g: a heritage that is no constructor (a number,
/// `undefined`, an arrow, a generator, a method), or a constructor whose `prototype` is neither
/// an object nor `null`, throws a `TypeError`; `prototype` is read once, after the check.
#[test]
fn extends_a_non_constructor_or_a_bad_prototype_throws_a_type_error() {
    both_pass(
        "function bad(make) { try { make(); } catch (e) { return e instanceof TypeError; } return false; }\n\
         function F() {}\n\
         F.prototype = 3;\n\
         var reads = 0;\n\
         var G = function () {}.bind(null);\n\
         Object.defineProperty(G, 'prototype', { get: function () { reads++; return null; } });\n\
         class H extends G {}\n\
         bad(function () { class A extends 1 {} }) && bad(function () { class A extends undefined {} })\n\
           && bad(function () { class A extends (() => {}) {} })\n\
           && bad(function () { class A extends function* () {} {} })\n\
           && bad(function () { class A extends ({ m() {} }).m {} })\n\
           && bad(function () { class A extends F {} })\n\
           && bad(function () { F.prototype = 'p'; class A extends F {} })\n\
           && reads === 1 && Object.getPrototypeOf(H.prototype) === null;",
    );
}

/// SuperCall, step 10 (BindThisValue, `#sec-bindthisvalue`, step 2): a second `super()` throws
/// a `ReferenceError` after the parent's constructor ran again and its arguments were
/// evaluated; the first binding stays.
#[test]
fn super_called_twice_throws_a_reference_error_after_the_parent_ran() {
    both_pass(
        "var log = [];\n\
         class A { constructor(x) { log.push('A' + x); this.x = x; } }\n\
         class B extends A {\n\
           constructor() {\n\
             super(1);\n\
             var first = this;\n\
             try { super((log.push('arg'), 2)); } catch (e) { log.push(e instanceof ReferenceError); }\n\
             log.push(this === first, this.x);\n\
           }\n\
         }\n\
         var b = new B();\n\
         log.join() === 'A1,arg,A2,true,true,1' && b.x === 1;",
    );
}

/// The `this` of a derived constructor is uninitialized until `super()` returns
/// (`#sec-function-environment-records-getthisbinding`, step 2): reading it, in the
/// constructor, in an argument of `super()`, through `typeof` and through `super.x` (whose
/// reference reads `this` first), throws a `ReferenceError`.
#[test]
fn this_before_super_throws_a_reference_error() {
    both_pass(
        "class A { constructor(v) { this.v = v; } }\n\
         function tdz(make) { try { make(); } catch (e) { return e instanceof ReferenceError; } return false; }\n\
         tdz(function () { new (class extends A { constructor() { this.x = 1; super(); } })(); })\n\
           && tdz(function () { new (class extends A { constructor() { super(this); } })(); })\n\
           && tdz(function () { new (class extends A { constructor() { typeof this; super(); } })(); })\n\
           && tdz(function () { new (class extends A { constructor() { super.x; super(); } })(); })\n\
           && tdz(function () { new (class extends A { constructor() { var f = () => this; f(); super(); } })(); })\n\
           && new (class extends A { constructor() { super(2); this.w = this.v; } })().w === 2;",
    );
}

/// `super()` in an arrow function binds the `this` of the constructor the arrow is in
/// (`#sec-super-keyword-runtime-semantics-evaluation`, GetThisEnvironment): the constructor
/// then reads it, its NewTarget reaches the parent, and an arrow in an arrow does it too;
/// `super.m()` in an arrow reads the constructor's home object.
#[test]
fn super_in_an_arrow_binds_this_for_the_constructor() {
    both_pass(
        "class A { constructor(x) { this.x = x; this.t = new.target; } m() { return 'A.m'; } }\n\
         class B extends A { constructor() { var f = () => super(1); var r = f(); this.same = r === this; } }\n\
         class C extends A { constructor() { var g = () => () => super(2); g()(); this.m2 = (() => super.m())(); } }\n\
         var b = new B(), c = new C();\n\
         b.x === 1 && b.same === true && b.t === B && Object.getPrototypeOf(b) === B.prototype\n\
           && c.x === 2 && c.t === C && c.m2 === 'A.m';",
    );
}

/// An arrow made before `super()` holds the constructor's environment, not a copy of its
/// `this`: called after the constructor returned (with a collection in between), it reads the
/// object `super()` bound.
#[test]
fn an_arrow_made_before_super_reads_this_after_the_constructor_returned() {
    for prologue in ["", "\"use strict\";\n"] {
        let mut agent = agent().unwrap();
        let realm = realm(&mut agent);
        let made = format!(
            "{prologue}class A {{ constructor() {{ this.a = 1; }} }}\n\
             class B extends A {{ constructor() {{ B.read = () => this; B.early = (() => {{ try {{ this; }} catch (e) {{ return e instanceof ReferenceError; }} }})(); super(); }} }}\n\
             var b = new B(); B.early;"
        );
        assert_eq!(eval_text(&mut agent, &realm, &made), passed(), "{made}");
        assert!(matches!(agent.collect(), Ok(HostOutcome::Normal(_))));
        assert_eq!(
            eval_text(&mut agent, &realm, "B.read() === b && B.read().a === 1;"),
            passed(),
            "{prologue}"
        );
    }
}

/// [[Construct]] of a derived class, steps 10 to 12: a returned object is the result; a
/// returned `undefined` gives the bound `this`; any other value throws a `TypeError`, also
/// after `super()`; `undefined` with `this` never bound throws a `ReferenceError`; a `return`
/// through `finally` follows the same rule after the `finally` block ran.
#[test]
fn a_derived_constructor_returns_an_object_undefined_or_throws() {
    both_pass(
        "class A { constructor() { this.a = 1; } }\n\
         var o = {};\n\
         function thrown(make, type) { try { make(); } catch (e) { return e instanceof type; } return false; }\n\
         class Obj extends A { constructor() { return o; } }\n\
         class Und extends A { constructor() { super(); return; } }\n\
         class Num extends A { constructor() { super(); return 1; } }\n\
         class Nul extends A { constructor() { super(); return null; } }\n\
         class None extends A { constructor() {} }\n\
         class Fin extends A { constructor() { try { return; } finally { super(); } } }\n\
         class FinNum extends A { constructor() { try { super(); } finally { return 2; } } }\n\
         new Obj() === o && new Und().a === 1 && new Und() instanceof Und\n\
           && thrown(function () { new Num(); }, TypeError) && thrown(function () { new Nul(); }, TypeError)\n\
           && thrown(function () { new None(); }, ReferenceError)\n\
           && new Fin() instanceof Fin && new Fin().a === 1\n\
           && thrown(function () { new FinNum(); }, TypeError);",
    );
}

/// The realm `there` with the derived classes of the realm tests, and their names as globals of
/// the realm `here`, with `OtherTypeError` and `OtherReferenceError` of `there`: `Num` returns
/// a number after `super()`, `None` returns without it, `Early` reads `this` before it.
fn two_realms() -> (Agent<support::host::TestHost>, swell::Realm) {
    let mut agent = agent().unwrap();
    let here = realm(&mut agent);
    let there = realm(&mut agent);
    let mut roots = Vec::new();
    for (name, source) in [
        (
            "Num",
            "(class Num extends (class {}) { constructor() { super(); return 1; } });",
        ),
        (
            "None",
            "(class None extends (class {}) { constructor() {} });",
        ),
        (
            "Early",
            "(class Early extends (class {}) { constructor() { this.x = 1; super(); } });",
        ),
        ("OtherTypeError", "TypeError;"),
        ("OtherReferenceError", "ReferenceError;"),
    ] {
        let HostOutcome::Normal(root) = eval(&mut agent, &there, source) else {
            panic!("{source}");
        };
        roots.push((name, root));
    }
    let global_root = global(&mut agent, &here);
    agent
        .with_cx(&here, |cx| {
            let global = object(cx, &global_root);
            for (name, root) in &roots {
                let value = cx.get(root).unwrap();
                let name = key(cx, name);
                cx.set(global, name, value, true).unwrap().unwrap();
            }
        })
        .unwrap();
    (agent, here)
}

/// [[Construct]] of a derived class, steps 10 to 12, with the caller in another realm: the
/// `TypeError` of a returned number and the `ReferenceError` of an unbound `this` are made
/// after the callee's context is removed, so they are of the realm that called `new` (here the
/// frame of the constructor is not the base of its run: `New` pushed it).
#[test]
fn return_errors_are_of_the_callers_realm() {
    let (mut agent, here) = two_realms();
    assert_eq!(
        eval_text(
            &mut agent,
            &here,
            "var r = [];\n\
             try { new Num(); } catch (e) { r.push(e instanceof TypeError, e instanceof OtherTypeError); }\n\
             try { new None(); } catch (e) { r.push(e instanceof ReferenceError, e instanceof OtherReferenceError); }\n\
             try { new Early(); } catch (e) { r.push(e instanceof ReferenceError, e instanceof OtherReferenceError); }\n\
             r.join();"
        ),
        Outcome::Normal("\"true,false,true,false,false,true\"".to_string())
    );
}

/// The same two errors through a bound function's [[Construct]] (`#sec-bound-function-exotic-
/// objects-construct-argumentslist-newtarget`), which constructs the class from Rust, so the
/// constructor's frame is the base of its run: they are of the realm that called `new`; `this`
/// read before `super()` is the `ReferenceError` of the constructor's own realm (its code
/// throws it): no object was made for it.
#[test]
fn return_errors_through_a_bound_function_are_of_the_callers_realm() {
    let (mut agent, here) = two_realms();
    assert_eq!(
        eval_text(
            &mut agent,
            &here,
            "var r = [];\n\
             var BNum = Num.bind(null), BNone = None.bind(null), BEarly = Early.bind(null);\n\
             try { new BNum(); } catch (e) { r.push(e instanceof TypeError, e instanceof OtherTypeError); }\n\
             try { new BNone(); } catch (e) { r.push(e instanceof ReferenceError, e instanceof OtherReferenceError); }\n\
             try { new BEarly(); } catch (e) { r.push(e instanceof ReferenceError, e instanceof OtherReferenceError); }\n\
             r.join();"
        ),
        Outcome::Normal("\"true,false,true,false,false,true\"".to_string())
    );
}

/// `super.x` in static methods and accessors of a derived class reads past their home object:
/// the parent class for a static method or accessor, the parent's prototype for an instance
/// accessor; a `super.x = v` in a setter writes on `this`.
#[test]
fn super_property_in_static_methods_and_accessors() {
    both_pass(
        "class A { static s() { return 'A.s'; } static get sg() { return 'A.sg'; } get g() { return 'A.g'; } m() { return 'A.m'; } }\n\
         class B extends A {\n\
           static s() { return super.s() + '>B'; }\n\
           static get sg() { return super.sg + '>B'; }\n\
           static set ss(v) { super.marked = v; }\n\
           get g() { return super.g + '>B'; }\n\
           set v(x) { super.w = x; }\n\
           m() { return super.m() + '>B'; }\n\
         }\n\
         var b = new B();\n\
         b.v = 5; B.ss = 7;\n\
         B.s() === 'A.s>B' && B.sg === 'A.sg>B' && b.g === 'A.g>B' && b.m() === 'A.m>B'\n\
           && b.w === 5 && Object.getOwnPropertyNames(A.prototype).indexOf('w') === -1\n\
           && B.marked === 7 && A.marked === undefined;",
    );
}

/// PerformEval, steps 7 to 11: direct eval code in a method admits a SuperProperty
/// (inMethod), and in a derived constructor a SuperCall too (inDerivedConstructor); `this` in
/// that eval code reads the constructor's binding, uninitialized before `super()`, and an arrow
/// in the eval code calls `super()` for the constructor.
#[test]
fn super_in_eval_in_a_method_and_in_a_derived_constructor() {
    both_pass(
        "class A { constructor(x) { this.x = x; } m() { return 'A.m'; } }\n\
         class B extends A {\n\
           constructor() {\n\
             var early; try { eval('this'); } catch (e) { early = e instanceof ReferenceError; }\n\
             eval('super(1)');\n\
             this.early = early;\n\
             this.same = eval('this') === this;\n\
             this.m1 = eval('super.m()');\n\
           }\n\
           m() { return eval('super.m()') + '>B'; }\n\
           n() { return (() => eval('super.m()'))(); }\n\
         }\n\
         class C extends A { constructor() { eval('(() => super(2))()'); } }\n\
         class D extends A { constructor() { (() => eval('super(3)'))(); } }\n\
         var b = new B();\n\
         b.x === 1 && b.early === true && b.same === true && b.m1 === 'A.m' && b.m() === 'A.m>B'\n\
           && b.n() === 'A.m' && new C().x === 2 && new D().x === 3\n\
           && Object.getPrototypeOf(new C()) === C.prototype;",
    );
}

/// PerformEval, steps 7 to 11: where the flags are not set, `super` in eval code is the
/// SyntaxError a script answers: `super.x` outside a method (the top, a function, an indirect
/// eval in a method), `super()` outside a derived constructor (a method, a base constructor,
/// an arrow in a method), and in `Function` code.
#[test]
fn super_in_eval_elsewhere_is_a_syntax_error() {
    both_pass(
        "function syntax(make) { try { make(); } catch (e) { return e instanceof SyntaxError; } return false; }\n\
         var o = { m() { return eval('super()'); }, n() { return (0, eval)('super.x'); }, a() { return (() => eval('super()'))(); } };\n\
         class A { constructor() { eval('super()'); } }\n\
         class B extends A { m() { eval('super()'); } }\n\
         syntax(function () { eval('super.x'); }) && syntax(function () { eval('super()'); })\n\
           && syntax(function () { (function () { eval('super.x'); })(); })\n\
           && syntax(function () { o.m(); }) && syntax(function () { o.n(); }) && syntax(function () { o.a(); })\n\
           && syntax(function () { new A(); }) && syntax(function () { new B().m(); })\n\
           && syntax(function () { Function('super.x'); }) && syntax(function () { Function('super()'); });",
    );
}

/// Subclasses of builtins whose constructors make their object from NewTarget: a Promise
/// subclass as the receiver of `resolve` and `then`, an Array subclass with its own
/// `length`, an Error subclass with `message` and the stack of prototypes, a RegExp subclass
/// whose `exec` runs.
#[test]
fn subclasses_of_promise_array_error_and_regexp() {
    both_pass(
        "class P extends Promise {}\n\
         class Ar extends Array { sum() { return this.reduce(function (a, b) { return a + b; }, 0); } }\n\
         class E extends Error { constructor(m) { super(m); this.name = 'E'; } }\n\
         class R extends RegExp { exec(s) { this.seen = s; return super.exec(s); } }\n\
         var p = P.resolve(1), q = p.then(function () {});\n\
         var a = new Ar(1, 2, 3), e = new E('msg'), r = new R('b+');\n\
         p instanceof P && q instanceof P && Object.getPrototypeOf(p) === P.prototype\n\
           && a instanceof Ar && Array.isArray(a) && a.length === 3 && a.sum() === 6\n\
           && (a.push(4), a.length === 4) && a.map(function (x) { return x; }) instanceof Ar\n\
           && e instanceof E && e instanceof Error && e.message === 'msg' && String(e) === 'E: msg'\n\
           && r instanceof R && r.test('abb') && r.seen === 'abb' && r.source === 'b+';",
    );
}

/// The default derived constructor (ClassDefinitionEvaluation, step 14.a) passes its
/// arguments to the parent as they are: no iterator is read, so a replaced
/// `%Array.prototype%[@@iterator]` is not called; its NewTarget reaches the parent.
#[test]
fn the_default_derived_constructor_passes_arguments_without_an_iterator() {
    both_pass(
        "var calls = 0;\n\
         Array.prototype[Symbol.iterator] = function () { calls++; throw new Error('iterated'); };\n\
         class A { constructor(a, b, c) { this.args = [a, b, c]; this.n = arguments.length; this.t = new.target; } }\n\
         class B extends A {}\n\
         var b = new B(1, 2, 3);\n\
         var none = new B();\n\
         calls === 0 && b.args[0] === 1 && b.args[1] === 2 && b.args[2] === 3 && b.n === 3\n\
           && b.t === B && none.n === 0 && B.length === 0;",
    );
}

/// The default derived constructor passes its kept arguments on at a step per argument, as
/// `RestArguments` copies them (`SuperCallKept`, `docs/design/engine.md`, "Classes", item 7).
/// `Reflect.construct` with 100 arguments against none, read on `last_entry_steps()`:
/// - straight into the base class `P`, 200 steps more: 100 for the list CreateListFromArrayLike
///   makes of the array (`#sec-createlistfromarraylike`) and 100 for `P`'s `arguments` object
///   (CreateUnmappedArgumentsObject), the cost of the spread and of its reader;
/// - through the three default constructors of `D3`, `D2` and `D1` down to `P`, 500 steps
///   more: those 200 and one step per argument at each of the three levels, 300.
#[test]
fn the_default_derived_constructor_charges_a_step_per_argument_per_level() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "class P { constructor() { this.n = arguments.length; } }\n\
             class D1 extends P {} class D2 extends D1 {} class D3 extends D2 {}\n\
             var none = [], hundred = [];\n\
             for (var i = 0; i < 100; i++) hundred.push(i);\n\
             true;"
        ),
        passed()
    );
    let mut steps = |source: &str| {
        assert_eq!(eval_text(&mut agent, &realm, source), passed(), "{source}");
        agent.last_entry_steps().unwrap()
    };
    let direct = steps("Reflect.construct(P, hundred).n === 100;")
        - steps("Reflect.construct(P, none).n === 0;");
    let levels = steps("Reflect.construct(D3, hundred).n === 100;")
        - steps("Reflect.construct(D3, none).n === 0;");
    assert_eq!((direct, levels), (200, 200 + 3 * 100));
}

/// `KeepThisEnv` before each `return` of a derived constructor whose `this` lives in its
/// function environment counts its hops from the environment the `return` stands in: from a
/// block with a captured `let`, from such a block inside `try` with a `finally`, from a
/// `finally` block with one, from a `for (let ...)` body and from a `catch` block, after
/// `super()`, each returns the object (a hop count that left out the environments the
/// constructor's own code pushed reads another slot and throws a `ReferenceError`).
#[test]
fn a_return_from_an_inner_environment_keeps_the_bound_this() {
    both_pass(
        "class A { constructor(x) { this.x = x; } }\n\
         class B extends A { constructor() { var f = () => this; { let a = 1; var g = () => a; super(1); return; } } }\n\
         class T extends A { constructor() { var f = () => this; try { let a = 2; var g = () => a; super(2); return; } finally { f(); } } }\n\
         class F extends A { constructor() { var f = () => this; try { super(3); } finally { let b = 3; var h = () => b; return; } } }\n\
         class L extends A { constructor() { var f = () => this; for (let i = 4; ; ) { var g = () => i; super(i); return; } } }\n\
         class C extends A { constructor() { var f = () => this; try { throw 5; } catch (e) { var g = () => e; super(e); return; } } }\n\
         new B().x === 1 && new T().x === 2 && new F().x === 3 && new L().x === 4 && new C().x === 5;",
    );
}

/// How many levels [`a_chain_of_1000_derived_classes_ends_at_the_step_budget`] makes: 1,000,
/// and 100 with a collection at every safe point, which marks every frame at each instruction,
/// so the cost grows with the square of the depth; the smaller chain still runs `super()` at
/// every level, which is what the step budget is charged on.
const LEVELS: u64 = if cfg!(feature = "collect-every-safe-point") {
    100
} else {
    1_000
};

/// A chain of derived classes whose default constructors call `super()` down to the base runs
/// each level as one frame of the run loop, charged steps like a call: with a step budget of
/// 5 steps per level, `new` of the deepest class ends in `Terminated(StepBudget)` (no stack
/// overflow, no re-entry), while the tenth class of the chain constructs within it. The chain
/// is built one class per entry: setting a class's [[Prototype]] walks the parent's chain at a
/// step per link (OrdinarySetPrototypeOf, step 8), so ten classes at depth 470 would already
/// spend the budget.
#[test]
fn a_chain_of_1000_derived_classes_ends_at_the_step_budget() {
    let mut limits = limits();
    limits.steps_per_entry = 5 * LEVELS;
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var C = class { constructor() { this.base = true; } }; var C10, k = 0; true;"
        ),
        passed()
    );
    for level in 0..LEVELS {
        let source = "C = class extends C {}; if (++k === 10) C10 = C; true;";
        assert_eq!(
            eval_text(&mut agent, &realm, source),
            passed(),
            "level {level}"
        );
    }
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var c10 = new C10(); c10.base && c10 instanceof C10;"
        ),
        passed()
    );
    assert_eq!(
        eval_text(&mut agent, &realm, "new C();"),
        Outcome::Terminated(Termination::StepBudget)
    );
    let spent = agent.last_entry_steps().unwrap();
    assert!(spent <= 5 * LEVELS, "{spent}");
}

/// The `frame_depth` of [`nested_super_calls_stop_at_frame_depth`]: 50, and 10 with a
/// collection at every safe point, which marks every frame at each instruction.
const FRAME_DEPTH: u32 = if cfg!(feature = "collect-every-safe-point") {
    10
} else {
    50
};

/// A chain of `super()` calls twice as deep as `frame_depth` throws a `RangeError` the script
/// catches, and a class of the chain at a fifth of `frame_depth` constructs after it.
#[test]
fn nested_super_calls_stop_at_frame_depth() {
    let mut limits = limits();
    limits.frame_depth = FRAME_DEPTH;
    let source = format!(
        "var C = class {{ constructor() {{ this.base = true; }} }};\n\
         for (var i = 0; i < {chain}; i++) C = class extends C {{ constructor() {{ super(); }} }};\n\
         var D = C;\n\
         for (var j = 0; j < {up}; j++) D = Object.getPrototypeOf(D);\n\
         var range; try {{ new C(); }} catch (e) {{ range = e instanceof RangeError; }}\n\
         range === true && new D().base === true;",
        chain = 2 * FRAME_DEPTH,
        up = 2 * FRAME_DEPTH - FRAME_DEPTH / 5,
    );
    assert_eq!(run(limits, &source), passed(), "{source}");
    let strict = format!("\"use strict\";\n{source}");
    assert_eq!(run(limits, &strict), passed(), "{strict}");
}

/// `super()` into a host constructor constructs it from Rust as `new` does, one re-entry of a
/// host call: a host constructor that calls back into script, which constructs the derived
/// class again, ends at `reentry_depth` with a `RangeError` and no panic, and the agent runs on.
#[test]
fn super_into_a_host_constructor_is_bounded_by_reentry_depth() {
    let mut limits = limits();
    limits.reentry_depth = 16;
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    define_function(&mut agent, &realm, "Host", 1, reenter, true);
    let outcome = eval(
        &mut agent,
        &realm,
        "var n = 0, seen;\n\
         Host.prototype = Object.prototype;\n\
         class D extends Host { constructor() { n++; super(function () { return new D(); }); } }\n\
         try { new D(); } catch (e) { seen = e; }\n\
         throw seen;",
    );
    assert_eq!(
        eval_text(&mut agent, &realm, "String(seen) + ',' + n;"),
        Outcome::Normal("\"RangeError: reentry_depth exceeded,17\"".to_string())
    );
    assert!(is_range_error_of(&mut agent, &realm, outcome));
    assert_eq!(
        eval_text(&mut agent, &realm, "1 + 1;"),
        Outcome::Normal("2".to_string())
    );
}

/// What a construction or call answers, as text: `ok`, or the `name` of what it threw.
const ANSWER_PRELUDE: &str = "function t(f) { try { f(); return 'ok'; } catch (e) { return (e && e.name) || String(e); } }\n\
     class A { constructor(x) { this.x = x; } }\n\
     var log = [];\n";

/// GetThisBinding (`#sec-function-environment-records-getthisbinding`, step 2) before
/// `super()` from every place that reads `this`: `delete this`, a template, a destructuring
/// default, a parameter's initializer, an arrow in an arrow, direct eval with `typeof` and
/// `delete`; a SuperProperty reads `this` before its key and before the value of an assignment
/// (`#sec-super-keyword-runtime-semantics-evaluation`); a `finally` after a parent that threw
/// still finds `this` unbound, and a second `super()` after one that threw binds it.
#[test]
fn this_before_super_throws_in_every_position_that_reads_it() {
    both_pass(&[
        ANSWER_PRELUDE,
        "\
         var tdz = [\n\
           t(() => new (class extends A { constructor() { delete this; super(); } })()),\n\
           t(() => new (class extends A { constructor() { `${this}`; super(); } })()),\n\
           t(() => new (class extends A { constructor() { var { a = this } = {}; super(); } })()),\n\
           t(() => new (class extends A { constructor(a = this) { super(); } })()),\n\
           t(() => new (class extends A { constructor() { (() => () => this)()(); super(); } })()),\n\
           t(() => new (class extends A { constructor() { eval('typeof this'); super(); } })()),\n\
           t(() => new (class extends A { constructor() { eval('delete this'); super(); } })()),\n\
           t(() => new (class extends A { constructor() { super[log.push('key')]; super(); } })()),\n\
           t(() => new (class extends A { constructor() { super.x = log.push('value'); super(); } })()),\n\
           t(() => new (class extends A { constructor() { super[log.push('k')] = log.push('v'); super(); } })()),\n\
           t(() => new (class extends (class { constructor() { throw 1; } }) { constructor() { try { super(); } finally { this.q = 1; } } })())\n\
         ];\n\
         var n = 0;\n\
         class Retry extends (class { constructor() { if (n++ === 0) throw 1; this.ok = 1; } }) { constructor() { try { super(); } catch (e) {} super(); } }\n\
         class Params extends A { constructor(a = super(1), b = this, f = () => this) { this.same = b === this && f() === this; } }\n\
         tdz.join() === 'ReferenceError,ReferenceError,ReferenceError,ReferenceError,ReferenceError,ReferenceError,ReferenceError,ReferenceError,ReferenceError,ReferenceError,ReferenceError'\n\
           && log.length === 0 && new Retry().ok === 1 && new Params().same === true && new Params().x === 1;"
    ].concat());
}

/// The return rule of a derived [[Construct]] (steps 10 to 12) through the routes that
/// [`return_errors_are_of_the_callers_realm`] and
/// [`return_errors_through_a_bound_function_are_of_the_callers_realm`] do not take, with the
/// classes of the realm `there`: the errors are made once the
/// callee's context is removed, so they are of the realm of the context that is current then:
/// `Reflect.construct` of this realm and of `there` (a builtin function's own context), a
/// Proxy without a `construct` trap and a SuperCall from a class of this realm (the caller's),
/// and the host's own construct (the realm it names).
#[test]
fn return_errors_by_every_route_of_construct_are_of_the_current_realm() {
    let (mut agent, here) = two_realms();
    assert_eq!(
        eval_text(
            &mut agent,
            &here,
            "var r = [];\n\
             var OtherReflect = Object.getPrototypeOf(OtherTypeError).constructor('return Reflect')();\n\
             var HereNum = class extends (class {}) { constructor() { super(); return 1; } };\n\
             function kind(make, Here, There) { try { make(); r.push('none'); } catch (e) { r.push(e instanceof Here ? 'here' : e instanceof There ? 'there' : 'other'); } }\n\
             kind(function () { Reflect.construct(Num, []); }, TypeError, OtherTypeError);\n\
             kind(function () { Reflect.construct(None, []); }, ReferenceError, OtherReferenceError);\n\
             kind(function () { new (new Proxy(Num, {}))(); }, TypeError, OtherTypeError);\n\
             kind(function () { new (class extends Num { constructor() { super(); } })(); }, TypeError, OtherTypeError);\n\
             kind(function () { OtherReflect.construct(Num, []); }, TypeError, OtherTypeError);\n\
             kind(function () { OtherReflect.construct(None, []); }, ReferenceError, OtherReferenceError);\n\
             kind(function () { OtherReflect.construct(HereNum, []); }, TypeError, OtherTypeError);\n\
             r.join();"
        ),
        Outcome::Normal("\"here,here,here,here,there,there,there\"".to_string())
    );
    let HostOutcome::Normal(num) = eval(&mut agent, &here, "Num;") else {
        panic!("Num");
    };
    let elsewhere = realm(&mut agent);
    for current in [&here, &elsewhere] {
        let HostOutcome::Throw(thrown) = agent.construct(current, &num, &[], None).unwrap() else {
            panic!("the host's construct of Num did not throw");
        };
        let global_root = global(&mut agent, current);
        let of_current = agent
            .with_cx(current, |cx| {
                let global = object(cx, &global_root);
                let value = cx.get(&thrown).unwrap();
                support::host::is_error_of(cx, global, value, "TypeError")
            })
            .unwrap();
        assert!(
            of_current,
            "the host's construct makes its error in the realm it names"
        );
    }
}

/// The heritage after the class exists: SuperCall and the default constructor read the parent
/// at the call (a bound function, `null`, a non-constructor after `Object.setPrototypeOf`),
/// `prototype` of a Proxy heritage is read once and only at the definition, and the forms
/// with no usable `prototype` throw a `TypeError` at the definition (`Proxy`, a bound
/// function), while `extends Symbol` defines and its construction throws.
#[test]
fn super_reads_a_parent_changed_after_the_definition() {
    both_pass(&[
        ANSWER_PRELUDE,
        "\
         class B extends A { constructor() { super(5); } }\n\
         Object.setPrototypeOf(B, A.bind(null));\n\
         var b = new B();\n\
         class C extends A {}\n\
         Object.setPrototypeOf(C, A.bind(null, 6));\n\
         var c = new C();\n\
         class N extends A {} Object.setPrototypeOf(N, null);\n\
         var ev = 0; class X extends A { constructor() { super(ev++); } } Object.setPrototypeOf(X, {});\n\
         var H = new Proxy(function () {}, { get(target, k) { log.push(String(k)); return target[k]; } });\n\
         class P extends H {} var defined = log.join(); new P();\n\
         class S extends Symbol {}\n\
         b.x === 5 && Object.getPrototypeOf(b) === B.prototype && c.x === 6 && c instanceof C\n\
           && t(() => new N()) === 'TypeError' && t(() => new X()) === 'TypeError' && ev === 1\n\
           && defined === 'prototype' && log.join() === 'prototype'\n\
           && t(() => { class Q extends Proxy {} }) === 'TypeError'\n\
           && t(() => { class Q extends (function () {}).bind() {} }) === 'TypeError'\n\
           && t(() => new S()) === 'TypeError';"
    ].concat());
}

/// Builtin parents make the instance from NewTarget's `prototype`, and with one that is no
/// object from the realm of NewTarget (GetPrototypeFromConstructor, step 4): a derived class
/// of each, constructed by `Reflect.construct` with a NewTarget of the realm `there` whose
/// `prototype` is a number.
#[test]
fn builtin_parents_fall_back_to_the_realm_of_new_target() {
    let mut agent = agent().unwrap();
    let here = realm(&mut agent);
    let there = realm(&mut agent);
    let HostOutcome::Normal(other) = eval(
        &mut agent,
        &there,
        "var f = function () {}; f.prototype = 1;\n\
         ({ f: f, protos: [Object.prototype, Array.prototype, Map.prototype, Error.prototype, Promise.prototype, Function.prototype, Number.prototype] });",
    ) else {
        panic!("there");
    };
    let global_root = global(&mut agent, &here);
    agent
        .with_cx(&here, |cx| {
            let global = object(cx, &global_root);
            let value = cx.get(&other).unwrap();
            let name = key(cx, "other");
            cx.set(global, name, value, true).unwrap().unwrap();
        })
        .unwrap();
    assert_eq!(
        eval_text(
            &mut agent,
            &here,
            "var made = [\n\
               Reflect.construct(class extends (class {}) {}, [], other.f),\n\
               Reflect.construct(class extends Array {}, [], other.f),\n\
               Reflect.construct(class extends Map {}, [], other.f),\n\
               Reflect.construct(class extends Error {}, [], other.f),\n\
               Reflect.construct(class extends Promise {}, [function () {}], other.f),\n\
               Reflect.construct(class extends Function {}, ['return 1'], other.f),\n\
               Reflect.construct(class extends Number {}, [1], other.f)\n\
             ];\n\
             made.every(function (o, i) { return Object.getPrototypeOf(o) === other.protos[i]; });"
        ),
        passed()
    );
}

/// Each value a SuperCall holds while script runs survives a collection there (meaningful with
/// `collect-every-safe-point`): the old parent GetSuperConstructor read when the arguments
/// replace it, a NewTarget only the frame holds, the arguments the default constructor passes
/// on, the value `super()` answered, a spread's elements, a Proxy trap's run, the prototype a
/// heritage getter answered, and `this` an arrow made before `super()` reads after it.
#[test]
fn values_of_a_super_call_survive_garbage() {
    both_pass(&[
        ANSWER_PRELUDE,
        "\
         function garbage() { var a = []; for (var i = 0; i < 2; i++) a.push({ i: i, s: 'x' + i }); return 0; }\n\
         class Old extends A { constructor() { super((Object.setPrototypeOf(Old, A), garbage())); } }\n\
         Object.setPrototypeOf(Old, class { constructor() { garbage(); this.v = 'old'; } });\n\
         class Nt extends A { constructor() { garbage(); super(1); } }\n\
         var nt = Reflect.construct(Nt, [], (function () { function F() {} F.prototype = { tag: 1 }; return F; })());\n\
         class Kept extends (class { constructor(o) { garbage(); this.k = o.k; } }) {}\n\
         class Res extends (class { constructor() { this.v = 7; } }) { constructor() { var r = super(); garbage(); this.same = r === this; } }\n\
         class Spread extends A { constructor() { super(...{ [Symbol.iterator]() { var k = 0; return { next() { garbage(); return k++ ? { done: true } : { value: 3, done: false }; } }; } }); } }\n\
         var Trapped = new Proxy(class extends A { constructor() { super(9); } }, { construct(t, a, n) { garbage(); return Reflect.construct(t, a, n); } });\n\
         var G = function () {}.bind(null);\n\
         Object.defineProperty(G, 'prototype', { get() { garbage(); return { m() { return 'm'; } }; } });\n\
         class Got extends G {} garbage(); Object.setPrototypeOf(Got, A);\n\
         class Early extends A { constructor() { var f = () => this; garbage(); super(1); garbage(); this.ok = f() === this; } }\n\
         var res = new Res();\n\
         new Old().v === 'old' && nt.tag === 1 && new Kept({ k: 1 }).k === 1 && res.same && res.v === 7\n\
           && new Spread().x === 3 && new Trapped().x === 9 && new Got().m() === 'm' && new Early().ok;"
    ].concat());
}
