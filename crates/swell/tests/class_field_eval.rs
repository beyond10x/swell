//! Direct and indirect `eval` in a class field initializer (PerformEval, `#sec-performeval`): a
//! direct eval whose GetThisEnvironment is a field initializer's (an instance or a static field,
//! and an arrow function in one, which keeps the initializer's) has inFunction and inMethod set,
//! inDerivedConstructor clear and inClassFieldInitializer set, read from the template kind
//! `FieldInitializer` (`docs/design/engine.md`, decision 3, "`this` and `arguments` in an arrow
//! function", item 5; "Classes", item 4). So `new.target` in it is `undefined`, a SuperProperty
//! reads through the initializer's home object, a SuperCall is a `SyntaxError` (in a derived
//! class too), and ContainsArguments of its code is a `SyntaxError`
//! (`#sec-static-semantics-containsarguments`); its `this` is the instance (F in a static field)
//! and its `var` names stay in its own record, since class code is strict. An indirect eval has
//! every flag clear. Written by `story:eval-in-class-field-initializers`. Run also with the
//! feature `collect-every-safe-point`.
//!
//! Each script compares its results with `===` and answers `true`, as written and with a
//! `"use strict";` prologue.
#![cfg(test)]

use std::cell::Cell;

use crate::support;

use support::host::{TestHost, agent, agent_with, limits, realm};
use support::script::{Outcome, define_function, eval_text, run, run_both};
use swell::{Completion, Cx, HostCall, InterruptHandle, Termination, Value};
use swell_syntax::{MessageKind, message_kind};

/// What a script that checks its own results answers.
fn passed() -> Outcome {
    Outcome::Normal("true".to_string())
}

/// `source` answers `true` as written and as strict code.
fn both_pass(source: &str) {
    assert_eq!(run_both(source), (passed(), passed()), "{source}");
}

/// The function `name(f)` the scripts share: the `name` of what `f()` throws, or `"none"`.
const NAME: &str =
    "function name(f) { try { f(); return \"none\"; } catch (e) { return e.name; } }\n";

/// The entry a test runs after a termination: an eval in an initializer answers 2.
const NEXT: &str = "class D { y = eval(\"2\"); } new D().y;";

/// PerformEval, step 12.h (`#sec-performeval`): in eval code of a direct eval in a field
/// initializer, ContainsArguments of the body is a `SyntaxError`, whatever reaches it (a read,
/// `typeof`, an assignment, an arrow function, a computed key of a class or an object literal,
/// a direct eval nested in the eval code, whose function is still the initializer's), for an
/// instance field and a static one; the code before it does not run. The message is the
/// grammar's (`swell_syntax::message_kind` reads `Grammar`, no refusal is counted), at the
/// `arguments` of the eval code.
#[test]
fn eval_of_arguments_in_an_initializer_is_a_syntax_error() {
    both_pass(&format!(
        "{NAME}var executed = false;\n\
         var r = [\n\
           name(function () {{ new (class {{ x = eval(\"executed = true; arguments\"); }})(); }}),\n\
           name(function () {{ new (class {{ x = eval(\"typeof arguments\"); }})(); }}),\n\
           name(function () {{ new (class {{ x = eval(\"arguments = 1\"); }})(); }}),\n\
           name(function () {{ new (class {{ x = eval(\"() => arguments\"); }})(); }}),\n\
           name(function () {{ new (class {{ x = eval(\"(class {{ [arguments] = 1; }})\"); }})(); }}),\n\
           name(function () {{ new (class {{ x = eval(\"({{ [arguments]: 1 }})\"); }})(); }}),\n\
           name(function () {{ new (class {{ x = eval(\"eval('arguments')\"); }})(); }}),\n\
           name(function () {{ class S {{ static x = eval(\"arguments\"); }} }})\n\
         ];\n\
         r.join() === \"SyntaxError,SyntaxError,SyntaxError,SyntaxError,SyntaxError,SyntaxError,SyntaxError,SyntaxError\"\n\
           && executed === false;"
    ));
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let before = agent.refusals();
    let answer = eval_text(
        &mut agent,
        &realm,
        "var m; try { new (class { x = eval(\"0;\\n  arguments\"); })(); } catch (e) { m = e.message; } m;",
    );
    let Outcome::Normal(quoted) = answer else {
        panic!("{answer:?}");
    };
    let message = quoted.trim_matches('"');
    assert_eq!(
        message,
        "arguments in eval code in a class field initializer (2:3)"
    );
    let (text, _) = message.rsplit_once(" (").unwrap();
    assert_eq!(message_kind(text), MessageKind::Grammar);
    assert_eq!(agent.refusals(), before);
}

/// An indirect eval in an initializer has every flag of PerformEval clear: `arguments` in its
/// code is an IdentifierReference of the global code, the global property when there is one and
/// otherwise unresolvable; `new.target` and `super.x` in it are `SyntaxError`s; its `this` is the
/// global object. In an instance field and a static one, and through an arrow function in one.
#[test]
fn indirect_eval_of_arguments_reads_the_global_binding() {
    both_pass(&format!(
        "{NAME}globalThis.arguments = \"global\";\n\
         class C {{\n\
           a = (0, eval)(\"arguments\");\n\
           b = (() => (0, eval)(\"arguments\"))();\n\
           c = (0, eval)(\"this\");\n\
           static d = (0, eval)(\"arguments\");\n\
         }}\n\
         var c = new C();\n\
         delete globalThis.arguments;\n\
         class D {{ a = (0, eval)(\"typeof arguments\"); }}\n\
         var r = [\n\
           name(function () {{ new (class {{ x = (0, eval)(\"new.target\"); }})(); }}),\n\
           name(function () {{ new (class {{ x = (0, eval)(\"super.x\"); }})(); }}),\n\
           name(function () {{ new (class {{ x = (0, eval)(\"arguments\"); }})(); }})\n\
         ];\n\
         c.a === \"global\" && c.b === \"global\" && c.c === globalThis && C.d === \"global\"\n\
           && new D().a === \"undefined\" && r.join() === \"SyntaxError,SyntaxError,ReferenceError\";"
    ));
}

/// ContainsArguments looks into arrow functions and no other function: in the eval code of an
/// initializer a function declaration and a function expression, a generator declaration and
/// expression, an `async` function and an `async` method (each reads `arguments` before its
/// first `await`), a method, a getter and a setter, and a class's constructor and method have
/// `arguments` of their own; a property name, a string and a member name `arguments` are no
/// IdentifierReference; a direct eval in a function made in the initializer is that function's,
/// with its own `arguments`.
#[test]
fn arguments_in_a_function_inside_the_eval_is_allowed() {
    both_pass(
        "class C {\n\
           declaration = eval(\"function f() { return arguments.length; } f(1, 2)\");\n\
           generator = eval(\"function* g() { yield arguments.length; } g(1, 2, 3, 4).next().value\");\n\
           asyncFunction = eval(\"var n; (async function () { n = arguments.length; })(1, 2, 3); n\");\n\
           asyncMethod = eval(\"var n; ({ async m() { n = arguments.length; } }).m(1); n\");\n\
           setter = eval(\"var n, o = { set s(v) { n = arguments.length; } }; o.s = 1; n\");\n\
           a = eval(\"(function () { return arguments.length; })(7, 8)\");\n\
           b = eval(\"({ m() { return arguments[0]; } }).m(3)\");\n\
           c = eval(\"({ get g() { return typeof arguments; } }).g\");\n\
           d = eval(\"(function* () { yield arguments.length; })(1, 2, 3).next().value\");\n\
           e = eval(\"new (class { constructor() { this.n = arguments.length; } })(1).n\");\n\
           f = eval(\"({ arguments: 4 }).arguments + 'arguments'.length\");\n\
           g = (function () { return eval(\"arguments.length\"); })(1, 2, 3, 4, 5);\n\
           h = eval(\"(class { m() { return arguments.length; } })\");\n\
           static s = eval(\"(function () { return arguments.length; })(1)\");\n\
         }\n\
         var c = new C();\n\
         c.declaration === 2 && c.generator === 4 && c.asyncFunction === 3 && c.asyncMethod === 1\n\
           && c.setter === 1\n\
           && c.a === 2 && c.b === 3 && c.c === \"object\" && c.d === 3 && c.e === 1 && c.f === 13\n\
           && c.g === 5 && new c.h().m(1, 2) === 2 && C.s === 1;",
    );
}

/// inClassFieldInitializer is set only where GetThisEnvironment is a field initializer's: a
/// direct eval in a method, a static method, a getter and the constructor body of a class with
/// fields, and in a method of an object literal, is in a function with a [[HomeObject]] (so
/// inMethod is set there) but no [[ClassFieldInitializerName]], and reads that function's
/// `arguments`.
#[test]
fn eval_of_arguments_in_a_method_of_a_class_with_fields_reads_its_own() {
    both_pass(
        "class C {\n\
           f = 1;\n\
           constructor() { this.c = eval(\"arguments.length\"); }\n\
           m() { return eval(\"arguments.length\"); }\n\
           static n() { return eval(\"arguments.length\"); }\n\
           get g() { return eval(\"typeof arguments + arguments.length\"); }\n\
         }\n\
         var o = { m() { return eval(\"arguments.length\"); } };\n\
         var c = new C(1, 2, 3, 4);\n\
         c.c === 4 && c.m(1, 2) === 2 && C.n(1) === 1 && c.g === \"object0\" && o.m(1, 2, 3) === 3;",
    );
}

/// PerformEval's inFunction is set in an initializer: `new.target` in the eval code is no
/// `SyntaxError` and reads the initializer's NewTarget, `undefined`, for a construction by
/// `new` and by `Reflect.construct` with another NewTarget, in a static field, in a derived
/// class and through an arrow function; the code before it runs.
#[test]
fn eval_new_target_is_undefined() {
    both_pass(
        "var executed = false;\n\
         class B {}\n\
         class C {\n\
           a = eval(\"executed = true; new.target\");\n\
           b = (() => eval(\"new.target\"))();\n\
           c = eval(\"typeof new.target\");\n\
           static d = eval(\"new.target\");\n\
         }\n\
         class D extends B { a = eval(\"new.target\"); constructor() { super(); this.t = new.target; } }\n\
         var c = new C(), r = Reflect.construct(C, [], Object), d = new D();\n\
         executed && c.a === undefined && c.b === undefined && c.c === \"undefined\"\n\
           && C.hasOwnProperty(\"d\") && C.d === undefined && r.a === undefined\n\
           && d.a === undefined && d.t === D;",
    );
}

/// PerformEval's inMethod is set in an initializer: a SuperProperty in the eval code reads
/// through the initializer's home object, the prototype for an instance field (its
/// [[Prototype]], with the instance as receiver) and F for a static one, with a literal key and
/// a computed one; in a derived class the parent's prototype and, statically, the parent itself.
#[test]
fn eval_super_property_reads_the_home_objects_prototype() {
    both_pass(
        "var base = { who() { return \"base \" + this.tag; }, plain: 7 };\n\
         class C {\n\
           tag = \"c\";\n\
           who() { return \"own\"; }\n\
           read = eval(\"super.who()\");\n\
           keyed = eval(\"super['pla' + 'in']\");\n\
           static call = eval(\"super.call\") === Function.prototype.call;\n\
         }\n\
         Object.setPrototypeOf(C.prototype, base);\n\
         class H { get v() { return 5; } static k() { return \"static \" + this.name; } }\n\
         class D extends H {\n\
           v = 1;\n\
           x = eval(\"super.v\");\n\
           y = eval(\"super['v'] + this.v\");\n\
           static s = eval(\"super.k()\");\n\
         }\n\
         var c = new C(), d = new D();\n\
         c.read === \"base c\" && c.keyed === 7 && C.call === true\n\
           && d.x === 5 && d.y === 6 && D.s === \"static D\";",
    );
}

/// PerformEval's inDerivedConstructor is clear in an initializer, a derived class's too (the
/// initializer is a method, not the constructor, and its fields run after `super()` returned):
/// a SuperCall in the eval code is a `SyntaxError` before any of the code runs, as written, in
/// an arrow function, behind a member access, through an arrow function in the initializer, in
/// a static field and in a base class; the parent's constructor runs once. The error is the
/// grammar's SuperCall error, and eval code with no SuperCall runs in the same initializer.
#[test]
fn eval_super_call_is_a_syntax_error_also_in_a_derived_class() {
    both_pass(&format!(
        "{NAME}var executed = false, parents = 0;\n\
         class H {{ constructor() {{ parents++; }} }}\n\
         var message;\n\
         try {{ new (class extends H {{ x = eval(\"super()\"); }})(); }} catch (e) {{ message = e.message; }}\n\
         class E extends H {{ x = eval(\"typeof (() => super.constructor)\"); }}\n\
         var ran = new E().x === \"function\" && message.indexOf(\"super call outside\") === 0;\n\
         parents = 0;\n\
         var r = [\n\
           name(function () {{ new (class extends H {{ x = eval(\"executed = true; super();\"); }})(); }}),\n\
           name(function () {{ new (class extends H {{ x = eval(\"executed = true; super()['x'];\"); }})(); }}),\n\
           name(function () {{ new (class extends H {{ x = eval(\"executed = true; () => super();\"); }})(); }}),\n\
           name(function () {{ new (class extends H {{ x = (() => eval(\"executed = true; super();\"))(); }})(); }}),\n\
           name(function () {{ class S extends H {{ static x = eval(\"executed = true; super();\"); }} }}),\n\
           name(function () {{ new (class {{ x = eval(\"executed = true; super();\"); }})(); }})\n\
         ];\n\
         r.join() === \"SyntaxError,SyntaxError,SyntaxError,SyntaxError,SyntaxError,SyntaxError\"\n\
           && executed === false && parents === 4 && ran;"
    ));
}

/// The eval code's `this` is the initializer's: the instance for an instance field, in a base
/// and a derived class, through an arrow function and through an eval nested in the eval code;
/// `this.f` reads the fields defined before.
#[test]
fn eval_this_is_the_instance() {
    both_pass(
        "class H {}\n\
         class C {\n\
           f = 1;\n\
           me = eval(\"this\");\n\
           arrow = (() => eval(\"this\"))();\n\
           nested = eval(\"eval('this')\");\n\
           g = eval(\"this.f + 1\");\n\
         }\n\
         class D extends H { me = eval(\"this\"); }\n\
         var c = new C(), d = new D();\n\
         c.me === c && c.arrow === c && c.nested === c && c.g === 2 && d.me === d;",
    );
}

/// Class code is strict, so eval code of a direct eval in an initializer is strict (strictEval,
/// `#sec-performeval`, steps 16 to 18): its `var` and function declarations are bindings of a
/// variable environment of that eval alone. A later eval in the same initializer, one in the
/// next field's, the initializer itself and the global code see none of them.
#[test]
fn an_evals_var_stays_in_that_eval() {
    both_pass(
        "class C {\n\
           x = eval(\"var v = 1; v\");\n\
           later = (eval(\"var u = 2; function g() {}\"), eval(\"typeof u + typeof g\"));\n\
           next = eval(\"typeof v\");\n\
           own = typeof v;\n\
           f = eval(\"function h() { return 3; } h()\");\n\
           static s = eval(\"var w = 4; w\");\n\
           static t = eval(\"typeof w\");\n\
         }\n\
         var c = new C();\n\
         c.x === 1 && c.later === \"undefinedundefined\" && c.next === \"undefined\"\n\
           && c.own === \"undefined\" && c.f === 3 && C.s === 4 && C.t === \"undefined\"\n\
           && typeof v === \"undefined\" && typeof u === \"undefined\" && typeof h === \"undefined\"\n\
           && typeof w === \"undefined\";",
    );
}

/// A static field's initializer is a method of F: a direct eval in it sees F as `this` (the
/// test262 case `static-field-init-with-this`), reads the static fields before it, and refuses
/// `arguments` (ContainsArguments) with the code before it not run; an indirect one there has
/// the global object as `this`.
#[test]
fn eval_in_a_static_field_sees_the_constructor_as_this_and_refuses_arguments() {
    both_pass(&format!(
        "{NAME}var executed = false;\n\
         class C {{\n\
           static f = \"test\";\n\
           static g = this.f + \"262\";\n\
           static h = eval(\"this.g\") + \"test\";\n\
           static me = eval(\"this\");\n\
           static arrow = (() => eval(\"this\"))();\n\
           static global = (0, eval)(\"this\");\n\
         }}\n\
         var r = [\n\
           name(function () {{ class S {{ static x = eval(\"executed = true; arguments\"); }} }}),\n\
           name(function () {{ class S {{ static x = (() => eval(\"executed = true; arguments\"))(); }} }}),\n\
           name(function () {{ class S {{ static x = eval(\"() => arguments\"); }} }})\n\
         ];\n\
         C.h === \"test262test\" && C.me === C && C.arrow === C && C.global === globalThis\n\
           && r.join() === \"SyntaxError,SyntaxError,SyntaxError\" && executed === false;"
    ));
}

/// An arrow function in an initializer keeps its GetThisEnvironment, the initializer's, so a
/// direct eval in it has the initializer's four flags: `arguments` is a `SyntaxError` (also two
/// arrow functions deep, and when the arrow is called after the construction), `super.x` reads
/// the home object's prototype, `new.target` is `undefined` and `this` the instance. A function
/// expression in the initializer is a function of its own: an eval in it has its `arguments`.
#[test]
fn an_arrow_in_an_initializer_passes_the_flags_to_its_eval() {
    both_pass(&format!(
        "{NAME}var base = {{ x: \"base\" }};\n\
         class C {{\n\
           args = () => eval(\"arguments\");\n\
           deep = () => () => eval(\"arguments\");\n\
           sup = () => eval(\"super.x\");\n\
           target = () => eval(\"new.target\");\n\
           me = () => eval(\"this\");\n\
           plain = function () {{ return eval(\"arguments.length\"); }};\n\
         }}\n\
         Object.setPrototypeOf(C.prototype, base);\n\
         var c = new C();\n\
         name(c.args) === \"SyntaxError\" && name(function () {{ c.deep()(); }}) === \"SyntaxError\"\n\
           && name(function () {{ new (class {{ x = (() => eval(\"arguments\"))(); }})(); }}) === \"SyntaxError\"\n\
           && c.sup() === \"base\" && c.target() === undefined && c.me() === c\n\
           && c.plain(1, 2) === 2;"
    ));
}

/// The step budget of the tests that loop: enough for the class definition and a
/// construction, small enough that a loop reaches it at once in either mode; 500 with a
/// collection at every safe point, where each step marks the whole heap.
const STEPS: u64 = if cfg!(feature = "collect-every-safe-point") {
    500
} else {
    2_000
};

/// "Hostile input must never hang": eval code that loops for ever in an initializer (an
/// instance one, through an arrow function in one, a static one) ends the entry in
/// `Terminated(StepBudget)`, since eval code runs as a frame of the run loop whose instructions
/// are steps; the agent runs the next entry.
#[test]
fn an_eval_loop_in_an_initializer_ends_at_the_step_budget() {
    let mut limits = limits();
    limits.steps_per_entry = STEPS;
    for source in [
        "class C { x = eval(\"for (;;) {}\"); } new C(); 0;",
        "class C { x = (() => eval(\"while (true) {}\"))(); } new C(); 0;",
        "class S { static a = eval(\"for (;;) {}\"); } 0;",
    ] {
        let mut agent = agent_with(limits).unwrap();
        let realm = realm(&mut agent);
        assert_eq!(
            eval_text(&mut agent, &realm, source),
            Outcome::Terminated(Termination::StepBudget),
            "{source}"
        );
        assert_eq!(
            eval_text(&mut agent, &realm, NEXT),
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

/// Decision 12, the interrupt: eval code in an initializer whose loop sets the interrupt flag on
/// its hundredth turn ends the entry in `Terminated(Interrupted)`, in an instance field and a
/// static one; the flag is cleared and the agent runs the next entry.
#[test]
fn an_eval_loop_in_an_initializer_ends_at_an_interrupt() {
    for source in [
        "class C { x = eval(\"for (var i = 0; ; i++) { if (i === 100) interruptNow(); }\"); }\n\
         new C(); 0;",
        "class S { static a = eval(\"for (var i = 0; ; i++) { if (i === 100) interruptNow(); }\"); }\n\
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
            eval_text(&mut agent, &realm, NEXT),
            Outcome::Normal("2".to_string()),
            "{source}"
        );
    }
}

/// The `frame_depth` of [`nested_eval_in_an_initializer_is_bounded`]: 1,000, the story's count,
/// in the default mode; 100 with a collection at every safe point, where every collection
/// traces every live frame, so the cost grows with the square of the depth (the story's count
/// there; `eval::eval_recursion_ends_in_a_range_error` gives the same reason).
fn nested_frame_depth() -> u32 {
    if cfg!(feature = "collect-every-safe-point") {
        100
    } else {
        1_000
    }
}

/// Eval code in an initializer that evals itself, directly and through an arrow function,
/// pushes a frame of the run loop per level, with no Rust recursion and every level still in
/// the initializer (an eval of `arguments` made at the fiftieth level is refused there), until
/// `frame_depth` throws a `RangeError` the script catches: below the first level are the
/// frames of the script, the constructor and the initializer, so `frame_depth - 3` levels run
/// directly, and `frame_depth / 2 - 1` with an arrow function's frame in each. No panic, and
/// the agent runs the next entry. Each level resolves `n`, `s` and `eval` through the records of every level below it,
/// a step per record (`Agent::resolve_name`), so the steps grow with the square of the depth:
/// the step budget is raised to let `frame_depth` end the run.
#[test]
fn nested_eval_in_an_initializer_is_bounded() {
    let mut limits = limits();
    limits.steps_per_entry = 50_000_000;
    limits.frame_depth = nested_frame_depth();
    let depth = limits.frame_depth;
    for (source, levels) in [
        ("var n = 0, s = \"n++; eval(s)\";", depth - 3),
        ("var n = 0, s = \"n++; (() => eval(s))()\";", depth / 2 - 1),
    ] {
        let script = format!(
            "{source}\n\
             var r;\n\
             try {{ new (class {{ x = eval(s); }})(); r = \"none\"; }}\n\
             catch (e) {{ r = e instanceof RangeError ? \"RangeError\" : String(e); }}\n\
             r + \" \" + n;"
        );
        assert_eq!(
            run(limits, &script),
            Outcome::Normal(format!("\"RangeError {levels}\"")),
            "{script}"
        );
    }
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var n = 0, s = \"if (++n < 50) eval(s); else eval('argu' + 'ments')\";\n\
             var r; try { new (class { x = eval(s); })(); } catch (e) { r = e.name; } r + \" \" + n;"
        ),
        Outcome::Normal("\"SyntaxError 50\"".to_string())
    );
    assert_eq!(
        eval_text(&mut agent, &realm, NEXT),
        Outcome::Normal("2".to_string())
    );
}
