//! Class static blocks (`#sec-class-definitions`, ClassStaticBlock): each block is a function of
//! its own, the body function of ClassStaticBlockDefinitionEvaluation
//! (`#sec-runtime-semantics-classstaticblockdefinitionevaluation`), made as a method with home
//! object F and called with `this` F in the static elements of ClassDefinitionEvaluation
//! (`#sec-runtime-semantics-classdefinitionevaluation`, step 31), in source order with the runs of
//! static fields (`docs/design/engine.md`, "Classes", item 4). The block runs as a frame of the
//! run loop (`Call(0)`), so the step budget, the interrupt and the frame depth reach it as they
//! reach a call. And the cases where the parts of order 3 meet: fields of a derived class, a field
//! initializer that calls a private method, private methods in a derived class. Written by
//! `story:class-static-blocks`. Run also with the feature `collect-every-safe-point`.
//!
//! Each script compares its results with `===` and answers `true`, as written and with a
//! `"use strict";` prologue.
#![cfg(test)]

use std::cell::Cell;

use crate::support;

use support::host::{TestHost, agent, agent_with, limits, realm};
use support::script::{Outcome, define_function, eval_text, parse, run, run_both};
use swell::{
    Completion, Cx, HostCall, InterruptHandle, Limits, Outcome as HostOutcome, Termination, Value,
};

/// What a script that checks its own results answers.
fn passed() -> Outcome {
    Outcome::Normal("true".to_string())
}

/// `source` answers `true` as written and as strict code.
fn both_pass(source: &str) {
    assert_eq!(run_both(source), (passed(), passed()), "{source}");
}

/// `source` answers `true` with `limits`, as written and as strict code.
fn both_pass_with(limits: Limits, source: &str) {
    assert_eq!(run(limits, source), passed(), "{source}");
    let strict = format!("\"use strict\";\n{source}");
    assert_eq!(run(limits, &strict), passed(), "{strict}");
}

/// ClassStaticBlockDefinitionEvaluation: the block's function is called with `this` F, once, at
/// the class's definition, after every method is defined (step 31 of ClassDefinitionEvaluation),
/// an arrow function in it keeping that `this`; the class's own inner name is initialized before
/// it runs (step 27); the block defines no property of F, and an empty block runs too
/// (`static-init-statement-list-optional.js`); a class expression's blocks run alike.
#[test]
fn a_block_runs_with_this_the_constructor() {
    both_pass(
        "var runs = 0, seen = [];\n\
         class C {\n\
           static { runs++; seen.push(this, (() => this)(), C, typeof this.later, typeof C.prototype.m); }\n\
           static {}\n\
           static later() {}\n\
           m() {}\n\
         }\n\
         new C(); new C();\n\
         var E = class Named { static { seen.push(this === Named); } };\n\
         runs === 1 && seen[0] === C && seen[1] === C && seen[2] === C\n\
           && seen[3] === \"function\" && seen[4] === \"function\" && seen[5] === true\n\
           && Object.getOwnPropertyNames(C).join() === \"length,name,prototype,later\";",
    );
}

/// A static block has a `var` scope of its own (ClassStaticBlockBody's VarScopedDeclarations
/// bind in the block function's environment): a `var`, a `let`, a `const` and a function
/// declared in a block are not visible after the class, nor in the next block, and an outer
/// binding of the same name keeps its value; a `var` of the block shadows the outer one inside
/// the block and is hoisted there; a block's function declaration is callable before its
/// declaration.
#[test]
fn a_blocks_var_is_not_visible_outside() {
    both_pass(
        "var x = \"outer\", r = [];\n\
         class C {\n\
           static { r.push(x, f()); var x = 1; let y = 2; const z = 3; function f() { return \"f\"; } r.push(x + y + z); }\n\
           static { r.push(typeof y, typeof z, typeof f, x); }\n\
         }\n\
         x === \"outer\" && typeof y === \"undefined\" && typeof z === \"undefined\" && typeof f === \"undefined\"\n\
           && r.join() === \",f,6,undefined,undefined,undefined,outer\"\n\
           && !Object.prototype.hasOwnProperty.call(globalThis, \"y\");",
    );
}

/// `super.x` in a block reads through its home object F (`static-init-super-property.js`): F's
/// [[Prototype]], the parent class in a derived class, `%Function.prototype%` in a base one,
/// with `this` F as the receiver; in an arrow function in the block too; and in direct eval code
/// in the block, whose PerformEval reads inMethod from the block's function (HasSuperBinding).
#[test]
fn super_property_in_a_block() {
    both_pass(
        "class B { static who() { return \"B \" + this.name; } }\n\
         var r = [];\n\
         class C extends B {\n\
           static {\n\
             r.push(super.who(), (() => super.who())(), super[\"wh\" + \"o\"] === B.who, eval(\"super.who()\"));\n\
           }\n\
         }\n\
         class D { static { r.push(super.call === Function.prototype.call); } }\n\
         r.join() === \"B C,B C,true,B C,true\";",
    );
}

/// `new.target` in a block is `undefined` (`static-init-expr-new-target.js`): the block's function
/// is called, never constructed; also in an arrow function in it, and in direct eval code in it,
/// which PerformEval admits (inFunction is true: GetThisEnvironment is the block's function
/// environment).
#[test]
fn new_target_in_a_block_is_undefined() {
    both_pass(
        "var r = [];\n\
         class C { static { r.push(new.target, (() => new.target)(), eval(\"new.target\")); } }\n\
         function F() { class G { static { r.push(new.target); } } }\n\
         new F();\n\
         r.length === 4 && r.every(function (v) { return v === undefined; });",
    );
}

/// ClassDefinitionEvaluation, step 31: the static fields and blocks run in source order, after
/// every computed key and every method (`static-init-sequence.js`: field, block, field, block);
/// a block reads the static fields defined before it and not those after it; a block between
/// two fields splits them into two runs; a construction runs no block.
#[test]
fn blocks_and_static_fields_run_in_source_order() {
    both_pass(
        "var log = [];\n\
         class C {\n\
           static a = log.push(\"a\");\n\
           static { log.push(\"block 1\", typeof this.a, typeof this.b); }\n\
           [(log.push(\"key\"), \"k\")]() {}\n\
           x = log.push(\"instance\");\n\
           static b = log.push(\"b\");\n\
           static { log.push(\"block 2\", typeof this.b, typeof this.c); }\n\
           static { log.push(\"block 3\"); }\n\
           static c = log.push(\"c\");\n\
         }\n\
         var defined = log.join();\n\
         new C();\n\
         defined === \"key,a,block 1,number,undefined,b,block 2,number,undefined,block 3,c\"\n\
           && log.length === 12 && log[11] === \"instance\";",
    );
}

/// A method or an accessor between static fields ends their run (`docs/design/engine.md`,
/// "Classes", item 4: one function per run of consecutive static fields), so the static elements
/// after it are called too, in source order: a static field, a method, a static field, a block that
/// reads both, a static accessor, a static field; the answer names every value in order.
#[test]
fn a_method_between_static_fields_ends_their_run() {
    let source = "var log = [];\n\
         class C {\n\
           static a = log.push(\"a\");\n\
           m() {}\n\
           static b = log.push(\"b\");\n\
           static { log.push(\"block \" + this.a + \" \" + this.b); }\n\
           static get g() { return \"g\"; }\n\
           static c = log.push(\"c\");\n\
         }\n\
         log.join() + \"|\" + [C.a, C.b, C.c, C.g].join();";
    let answer = Outcome::Normal("\"a,b,block 1 2,c|1,2,4,g\"".to_string());
    assert_eq!(run_both(source), (answer.clone(), answer), "{source}");
}

/// ClassDefinitionEvaluation, step 31.b: an abrupt completion of a block ends the class's
/// definition (`static-init-abrupt.js`): the rest of the block, the fields and blocks after it do
/// not run; the class scope's own name is initialized (step 27), so an arrow made in the block
/// reads F; the binding of the class declaration in the code around it is never initialized
/// (BindingClassDeclarationEvaluation returns the throw first), so a read of it throws a
/// `ReferenceError`; a class expression assigns nothing.
#[test]
fn a_throw_from_a_block_leaves_the_class_binding_uninitialized() {
    both_pass(
        "var read, inner, thrown, after = [], E = \"unset\";\n\
         try {\n\
           read = function () { return C; };\n\
           class C {\n\
             static a = after.push(\"field before\");\n\
             static { inner = () => C; throw new Error(\"block\"); after.push(\"same block\"); }\n\
             static b = after.push(\"field after\");\n\
             static { after.push(\"block after\"); }\n\
           }\n\
         } catch (e) { thrown = e.message; }\n\
         var outer;\n\
         try { read(); outer = \"read\"; } catch (e) { outer = e instanceof ReferenceError; }\n\
         try { E = class { static { throw 1; } }; } catch (e) {}\n\
         thrown === \"block\" && after.join() === \"field before\" && outer === true\n\
           && typeof inner() === \"function\" && inner().a === 1 && !inner().hasOwnProperty(\"b\")\n\
           && E === \"unset\";",
    );
}

/// `await` in a block is reserved (ClassStaticBlockStatementList is `[+Await]`), but a function
/// in the block has [Await] of its own: there `await` is a binding name of a `var`, a `let`, a
/// `const`, a parameter, a function's name, a `catch` parameter, a generator's and a method's
/// parameter (the `static-init-await-binding-valid.js` cases).
#[test]
fn await_as_a_binding_name_in_a_nested_function() {
    both_pass(
        "var r = [];\n\
         class C {\n\
           static {\n\
             (function () { var await = 1; r.push(await); })();\n\
             (function () { let await = 2; r.push(await); })();\n\
             (function () { const await = 3; r.push(await); })();\n\
             (function (await) { r.push(await); })(4);\n\
             r.push((function await() { return typeof await; })());\n\
             (function () { try { throw 6; } catch (await) { r.push(await); } })();\n\
             r.push((function* (await) { yield await; })(7).next().value);\n\
             r.push(({ m(await) { return await; } }).m(8));\n\
           }\n\
         }\n\
         r.join() === \"1,2,3,4,function,6,7,8\";",
    );
}

/// A static block makes an arguments object when a direct `eval` in its code can observe one
/// ("Classes", item 4; "Names", item 4): PerformEval reads inClassFieldInitializer false from the
/// block's function, which has no [[ClassFieldInitializerName]], so `eval("arguments")` is the
/// block's own arguments object, of no argument (`Call(0)`), not the one of a function around
/// the class; an arrow function in the block hands its eval the same. The other flags of that
/// PerformEval: `super()` in the eval code is a `SyntaxError` (inDerivedConstructor is false, in
/// a derived class too), and a `var` of the eval code stays in its own record (class code is
/// strict). A private name the class declares in that eval code answers the refusal of
/// `Agent::private_names_of_direct_eval` until `story:private-names-in-eval-code`.
#[test]
fn a_direct_eval_of_arguments_in_a_block_sees_its_own_arguments_object() {
    both_pass(
        "var r = [], outerArgs;\n\
         function syntax(f) { try { f(); } catch (e) { return e instanceof SyntaxError; } return false; }\n\
         class B {}\n\
         function outer(a, b) {\n\
           outerArgs = arguments;\n\
           class C extends B {\n\
             static {\n\
               var own = eval(\"arguments\");\n\
               r.push(own.length, own !== outerArgs, Object.prototype.toString.call(own));\n\
               r.push((() => eval(\"arguments\"))() === own);\n\
               r.push(syntax(() => eval(\"super()\")));\n\
               eval(\"var hidden = 1;\");\n\
               r.push(typeof hidden);\n\
             }\n\
           }\n\
         }\n\
         outer(1, 2);\n\
         var refusal = \"none\";\n\
         class P { #m() {} static { try { eval(\"this.#m\"); } catch (e) { refusal = e.message; } } }\n\
         r.join() === \"0,true,[object Arguments],true,true,undefined\"\n\
           && refusal.indexOf(\"private name #m in direct eval code is not supported before slice 8 (\") === 0;",
    );
}

/// The step budget of the termination tests: enough for the class definition, small enough
/// that a loop in a block reaches it at once in either mode; 500 with a collection at every safe
/// point, where each step marks the whole heap.
const STEPS: u64 = if cfg!(feature = "collect-every-safe-point") {
    500
} else {
    2_000
};

/// "Hostile input must never hang": a block whose code loops for ever ends the entry in
/// `Terminated(StepBudget)` (the block is a frame of the run loop, whose instructions are steps),
/// a block after a static field and a block of a class in a function alike; the agent runs the
/// next entry.
#[test]
fn a_block_loop_ends_at_the_step_budget() {
    let mut limits = limits();
    limits.steps_per_entry = STEPS;
    for source in [
        "class C { static { for (;;) {} } } 0;",
        "class S { static a = 1; static { while (true) {} } } 0;",
        "(function () { class D { static { (() => { for (;;) {} })(); } } })(); 0;",
    ] {
        let mut agent = agent_with(limits).unwrap();
        let realm = realm(&mut agent);
        assert_eq!(
            eval_text(&mut agent, &realm, source),
            Outcome::Terminated(Termination::StepBudget),
            "{source}"
        );
        assert_eq!(
            eval_text(
                &mut agent,
                &realm,
                "class E { static { this.y = 2; } } E.y;"
            ),
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

/// Decision 12, the interrupt: a block whose loop sets the interrupt flag on its hundredth turn
/// ends the entry in `Terminated(Interrupted)`; the flag is cleared and the agent runs the next
/// entry.
#[test]
fn a_block_loop_ends_at_an_interrupt() {
    for source in [
        "class C { static { for (var i = 0; ; i++) { if (i === 100) interruptNow(); } } } 0;",
        "class S { static a = 1; static { (() => { for (var i = 0; ; i++) { if (i === 100) interruptNow(); } })(); } } 0;",
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
            eval_text(
                &mut agent,
                &realm,
                "class E { static { this.y = 2; } } E.y;"
            ),
            Outcome::Normal("2".to_string()),
            "{source}"
        );
    }
}

/// The limits of the frame-depth tests: a frame depth of 1,000; with a collection at every safe
/// point, which marks every frame at each instruction, 100 and a re-entry depth of 8.
fn depth_limits() -> Limits {
    let mut limits = limits();
    if cfg!(feature = "collect-every-safe-point") {
        limits.frame_depth = 100;
        limits.reentry_depth = 8;
    } else {
        limits.frame_depth = 1_000;
    }
    limits
}

/// Decision 4, the frame depth: a function whose class's static block calls the function again
/// recurses through the blocks' `Call(0)`, whose frames are checked as any call's
/// (`push_call_frame`), and stops with a `RangeError` that a `catch` sees, a block of a class
/// expression and a block that constructs a class whose block recurses alike; the agent defines
/// classes with blocks after it.
#[test]
fn a_recursion_of_classes_with_blocks_stops_at_frame_depth() {
    both_pass_with(
        depth_limits(),
        "function f() { class C { static { f(); } } }\n\
         function g() { return class { static { g(); } }; }\n\
         function h() { new (class { constructor() { class D { static { h(); } } } })(); }\n\
         function range(fn) { try { fn(); } catch (e) { return e instanceof RangeError; } return false; }\n\
         range(f) && range(g) && range(h) && (class { static { this.ok = 1; } }).ok === 1;",
    );
}

/// The parts of order 3 meet: the instance initializer adds every private method of the class
/// before its first field runs (`#sec-initializeinstanceelements`: methods, then fields), so a
/// field initializer calls a private method, getter or setter declared after it in the source;
/// also in a derived class, after `super()`, and a static field calls a static private method.
#[test]
fn a_field_initializer_sees_the_private_methods() {
    both_pass(
        "class C {\n\
           a = this.#m(1);\n\
           b = this.#g;\n\
           c = (this.#s = 5, this.stored);\n\
           #m(n) { return n + 1; }\n\
           get #g() { return \"g\"; }\n\
           set #s(v) { this.stored = v; }\n\
           static d = C.#t();\n\
           static #t() { return \"t\"; }\n\
         }\n\
         class D extends C {\n\
           e = this.#n() + this.a;\n\
           #n() { return 10; }\n\
         }\n\
         var c = new C(), d = new D();\n\
         c.a === 2 && c.b === \"g\" && c.c === 5 && C.d === \"t\" && d.e === 12 && d.b === \"g\"\n\
           && Object.keys(d).join() === \"a,b,stored,c,e\";",
    );
}

/// The initializer that runs after `super()` is the one of the class whose constructor called it
/// (`#sec-super-keyword-runtime-semantics-evaluation`, SuperCall step 9: InitializeInstanceElements
/// of the active function): a parent's fields run inside the parent's construction, the child's
/// after `super()` returns and before the code after it, level by level through three classes;
/// the parent sees none of the child's fields; a default derived constructor runs them alike.
#[test]
fn fields_run_after_super_for_the_class_whose_constructor_called_it() {
    both_pass(
        "var log = [];\n\
         class A { a = log.push(\"A field\"); constructor() { log.push(\"A body\", typeof this.b); } }\n\
         class B extends A {\n\
           b = log.push(\"B field\");\n\
           constructor() { log.push(\"B before\"); super(); log.push(\"B after\", typeof this.b, typeof this.c); }\n\
         }\n\
         class C extends B { c = log.push(\"C field\"); }\n\
         var o = new C();\n\
         log.join() === \"B before,A field,A body,undefined,B field,B after,number,undefined,C field\"\n\
           && Object.keys(o).join() === \"a,b,c\";",
    );
}

/// `super()` in an arrow function runs the fields of the class whose constructor made the arrow
/// (the arrow's function stands for the constructor: SuperCall's GetThisEnvironment and active
/// function), also when the arrow is called later, after other code of the constructor ran, and
/// from a nested arrow; the fields run once, before the arrow's `super()` returns.
#[test]
fn fields_run_after_super_in_an_arrow_called_later() {
    both_pass(
        "var log = [];\n\
         class A { a = log.push(\"A field\"); }\n\
         class B extends A {\n\
           x = log.push(\"B field\");\n\
           constructor() {\n\
             var call = () => { var r = super(); log.push(\"returned\", r === this); };\n\
             var later = () => () => call();\n\
             log.push(\"made\");\n\
             later()();\n\
             log.push(\"after\", this.x);\n\
           }\n\
         }\n\
         var b = new B();\n\
         log.join() === \"made,A field,B field,returned,true,after,3\" && b.x === 3 && b.a === 2;",
    );
}

/// SuperCall binds `this` before it initializes the instance elements (BindThisValue, step 7,
/// then InitializeInstanceElements, step 9): an initializer that throws ends the `super()` with
/// its throw, the fields after it not defined, and `this` is bound: the constructor reads it,
/// returns it, and a second `super()` throws a `ReferenceError` (the binding is initialized)
/// after the parent ran again.
#[test]
fn an_initializer_that_throws_after_super_leaves_this_bound() {
    both_pass(
        "var parents = 0;\n\
         class A { constructor() { parents++; } }\n\
         class B extends A {\n\
           a = 1;\n\
           b = (() => { throw new Error(\"init\"); })();\n\
           c = 3;\n\
           constructor() {\n\
             var first, second;\n\
             try { super(); } catch (e) { first = e.message; }\n\
             this.first = first;\n\
             try { super(); } catch (e) { second = e instanceof ReferenceError; }\n\
             this.second = second;\n\
           }\n\
         }\n\
         var o = new B();\n\
         o instanceof B && o.first === \"init\" && o.second === true && o.a === 1\n\
           && !o.hasOwnProperty(\"c\") && parents === 2;",
    );
}

/// A derived constructor that returns an object before `super()` (`#sec-ecmascript-function-objects-construct-argumentslist-newtarget`,
/// step 10: the result is the object) runs no parent and initializes nothing: the object has
/// none of the class's fields and none of its private methods (a private method call on it
/// throws a `TypeError`), and the parent's fields are not on it either.
#[test]
fn a_derived_constructor_returning_an_object_before_super_adds_nothing() {
    both_pass(
        "var parents = 0, back = { own: 1 };\n\
         class A { p = 1; constructor() { parents++; } }\n\
         class B extends A {\n\
           x = 2;\n\
           #m() { return \"m\"; }\n\
           static branded(o) { try { o.#m(); return true; } catch (e) { return e instanceof TypeError ? false : \"other\"; } }\n\
           constructor() { return back; }\n\
         }\n\
         var o = new B();\n\
         o === back && parents === 0 && Object.keys(o).join() === \"own\" && B.branded(o) === false\n\
           && Object.getPrototypeOf(o) === Object.prototype;",
    );
}

/// `super.x` in a field initializer of a derived class reads through the initializer's home
/// object ("Classes", item 5): the prototype for an instance field, so the parent's prototype,
/// with `this` the instance as receiver; F for a static field, so the parent class; an arrow
/// function in a field keeps both.
#[test]
fn super_property_in_an_initializer_of_a_derived_class() {
    both_pass(
        "class A { get v() { return \"A \" + this.tag; } static s() { return \"As \" + this.name; } }\n\
         class B extends A {\n\
           tag = \"b\";\n\
           get v() { return \"own\"; }\n\
           x = super.v;\n\
           y = () => super.v;\n\
           static z = super.s();\n\
           static w = (() => super.s())();\n\
         }\n\
         var b = new B();\n\
         b.x === \"A b\" && b.y() === \"A b\" && B.z === \"As B\" && B.w === \"As B\" && b.v === \"own\";",
    );
}

/// How many instances [`a_class_with_100_private_methods_and_100_fields_constructed_1000_times`]
/// constructs in its loop: the story's 1,000 with `SWELL_FULL_COUNTS=1` (1.44 CPU s alone) and
/// 500 in the gate (0.80 CPU s); with a collection at every safe point, which marks the whole
/// heap at each instruction of a construction (about 0.45 CPU s a construction), 2 with
/// `SWELL_FULL_COUNTS=1` and 1 in the gate, where the story's 100 took 53 CPU s and 3 took 2.75.
/// Every construction spends the same steps, so a construction whose scans of the private
/// elements are not charged fails the reading of [`SCANNED_STEPS`] at every count, 1 included.
fn constructions() -> u64 {
    let full = std::env::var("SWELL_FULL_COUNTS").is_ok_and(|value| value == "1");
    match (cfg!(feature = "collect-every-safe-point"), full) {
        (false, true) => 1_000,
        (false, false) => 500,
        (true, true) => 2,
        (true, false) => 1,
    }
}

/// The source of a class expression with 100 private methods `#m0` to `#m99` and 100 fields
/// `f0` to `f99`, each field calling the private method of its number.
fn class_with_methods_and_fields() -> String {
    let methods: String = (0..100)
        .map(|i| format!("#m{i}() {{ return {i}; }} "))
        .collect();
    let fields: String = (0..100).map(|i| format!("f{i} = this.#m{i}(); ")).collect();
    format!("class {{ {methods}{fields}}}")
}

/// The steps a construction of [`class_with_methods_and_fields`] spends at the least on the
/// comparisons of PrivateElementFind past `SMALL` (8), a step each
/// (`object/private_names.rs`): the k-th add of a private method compares with the k − 1
/// elements before it, 4,186 steps over the 100 adds, and the field that reads `#m<i>` finds it
/// after i + 1 comparisons, 4,278 steps over the 100 fields.
const SCANNED_STEPS: u64 = 4_186 + 4_278;

/// The steps a construction may spend, for the step budget of the loop: 9,604 measured (500
/// constructions in 4,802,331 steps; the loop's checks after it included).
const STEPS_PER_CONSTRUCTION: u64 = 20_000;

/// The room above a fresh agent and realm that the memory budget of
/// [`a_class_with_100_private_methods_and_100_fields_constructed_1000_times`] leaves: 1 MiB, some
/// hundred instances (256 KiB does not hold the parse of the class's source).
const ROOM: u64 = 1024 * 1024;

/// A class with 100 private methods and 100 fields whose initializers call them, constructed
/// [`constructions`] times in one entry: every instance holds its 100 fields with the values of
/// the methods, and the loop spends at least [`SCANNED_STEPS`] a construction; the same loop under
/// a step budget of half what it spent ends in `Terminated(StepBudget)` in the loop, with the
/// constructions before the last whole; a loop that keeps instances under a memory budget of
/// [`ROOM`] ends in `Terminated(MemoryBudget)` and leaks nothing (not with a collection at every
/// safe point, where that loop marks the heap at each of its instructions until it is full); the
/// agent runs the next entry after each.
#[test]
fn a_class_with_100_private_methods_and_100_fields_constructed_1000_times() {
    let count = constructions();
    let class = class_with_methods_and_fields();
    let define = format!("var C = {class}, o = null, sum = 0, i = 0; 0;");
    let looped = format!(
        "for (i = 0; i < {count}; i++) {{ o = new C(); sum += o.f99 + o.f1; }}\n\
         sum === {count} * 100 && Object.keys(o).length === 100;"
    );
    let mut enough = limits();
    enough.steps_per_entry = STEPS_PER_CONSTRUCTION * count;
    let mut agent = agent_with(enough).unwrap();
    let realm_enough = realm(&mut agent);
    assert_eq!(
        eval_text(&mut agent, &realm_enough, &define),
        Outcome::Normal("0".to_string())
    );
    assert_eq!(eval_text(&mut agent, &realm_enough, &looped), passed());
    let spent = agent.last_entry_steps().unwrap();
    println!(
        "{count} constructions: {spent} steps, {} a construction",
        spent / count
    );
    assert!(
        spent >= SCANNED_STEPS * count,
        "{spent} steps for {count} constructions"
    );
    let mut short = limits();
    short.steps_per_entry = spent / 2;
    let mut agent = agent_with(short).unwrap();
    let realm_short = realm(&mut agent);
    assert_eq!(
        eval_text(&mut agent, &realm_short, &define),
        Outcome::Normal("0".to_string())
    );
    assert_eq!(
        eval_text(&mut agent, &realm_short, &looped),
        Outcome::Terminated(Termination::StepBudget)
    );
    assert_eq!(
        eval_text(
            &mut agent,
            &realm_short,
            &format!("i < {count} && sum === i * 100;")
        ),
        passed()
    );
    assert_eq!(
        eval_text(
            &mut agent,
            &realm_short,
            "class E { #m() { return 1; } x = this.#m(); } new E().x;"
        ),
        Outcome::Normal("1".to_string())
    );
    if cfg!(feature = "collect-every-safe-point") {
        return;
    }
    let fresh = {
        let mut fresh_agent = support::host::agent().unwrap();
        let _realm = support::host::realm(&mut fresh_agent);
        fresh_agent.heap_bytes().unwrap()
    };
    let mut small = limits();
    small.memory_bytes = fresh + ROOM;
    small.steps_per_entry = 100_000_000;
    let mut agent = agent_with(small).unwrap();
    let small_realm = realm(&mut agent);
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
    assert_eq!(
        eval_text(&mut agent, &small_realm, "class E { x = 1; } new E().x;"),
        Outcome::Normal("1".to_string())
    );
}

/// Decision 4, the frame depth: a field initializer of a derived class that constructs its own
/// class recurses through `super()` and the initializer that runs after it (each a frame checked
/// as a call's), and stops with a `RangeError` that a `catch` sees; through `super()` in an arrow
/// function and through a default derived constructor alike; the classes work after it.
#[test]
fn a_recursion_of_new_through_super_in_an_initializer_stops_at_frame_depth() {
    both_pass_with(
        depth_limits(),
        "class A {}\n\
         class B extends A { x = new B(); constructor() { super(); } }\n\
         class C extends A { x = new C(); constructor() { (() => super())(); } }\n\
         class D extends A { x = new D(); }\n\
         function range(fn) { try { fn(); } catch (e) { return e instanceof RangeError; } return false; }\n\
         range(function () { new B(); }) && range(function () { new C(); }) && range(function () { new D(); })\n\
           && new (class extends A { y = 1; })().y === 1;",
    );
}
