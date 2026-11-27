//! `new.target` (`#sec-meta-properties`, GetNewTarget `#sec-getnewtarget`) in every function that
//! is not a class: `undefined` in a call, the NewTarget of a [[Construct]], the value of the code
//! that made an arrow function (`docs/design/engine.md`, decision 3, "`this` and `arguments` in
//! an arrow function", item 4), `undefined` in generators and `async` functions, and in direct
//! eval code where PerformEval's `inFunction` admits it (item 5, `#sec-performeval`). Written by
//! `story:new-target-meta-property`. Run also with the feature `collect-every-safe-point`.
//!
//! Each script compares its results with `===` and answers `true`, as written and with a
//! `"use strict";` prologue.
#![cfg(test)]

use crate::support;

use support::host::{TestHost, agent, agent_with, limits, realm};
use support::script::{Outcome, describe, eval, eval_text, parse, run_both};
use swell::{Agent, Outcome as HostOutcome, Termination};

/// What a script that checks its own results answers.
fn passed() -> Outcome {
    Outcome::Normal("true".to_string())
}

/// `source` answers `true` as written and as strict code.
fn both_pass(source: &str) {
    assert_eq!(run_both(source), (passed(), passed()), "{source}");
}

/// GetNewTarget, step 3, and [[Call]] (`#sec-ecmascript-function-objects-call-thisargument-argumentslist`,
/// PrepareForOrdinaryCall with NewTarget undefined): a function called, as a declaration, an
/// expression, a method of an object literal, an accessor and a callee of a tagged template,
/// sees `undefined`.
#[test]
fn a_call_sees_new_target_undefined() {
    both_pass(
        "function f() { return new.target; }\n\
         var g = function () { return new.target; };\n\
         var o = { m: function () { return new.target; }, n() { return new.target; },\n\
                   get x() { return new.target; } };\n\
         function tag() { return new.target; }\n\
         f() === undefined && g() === undefined && o.m() === undefined && o.n() === undefined\n\
           && o.x === undefined && tag`t` === undefined\n\
           && (function () { return typeof new.target; })() === \"undefined\";",
    );
}

/// [[Construct]] (`#sec-ecmascript-function-objects-construct-argumentslist-newtarget`) and
/// EvaluateNew (`#sec-evaluatenew`): a function constructed with `new`, with and without
/// arguments, sees itself; `new.target` written with white space and comments between its
/// tokens is the same; a function that returns its `new.target` answers itself.
#[test]
fn a_construct_sees_the_function() {
    both_pass(
        "var seen;\n\
         function F() { seen = new.target; }\n\
         new F(); var plain = seen;\n\
         new F; var bare = seen;\n\
         var G = function () { return new /* a */ . /* b */ target; };\n\
         function H(a, b) { return [new.target, a, b]; }\n\
         var h = new H(1, 2);\n\
         plain === F && bare === F && new G() === G && h[0] === H && h[1] === 1 && h[2] === 2\n\
           && new F() instanceof F;",
    );
}

/// [[Construct]] of a bound function (`#sec-bound-function-exotic-objects-construct-argumentslist-newtarget`,
/// step 5): constructed itself, a bound function passes its target as NewTarget, through any
/// depth of binding; a NewTarget that is not the bound function passes through as it is.
#[test]
fn a_bound_function_construct_passes_its_target() {
    both_pass(
        "var seen;\n\
         function A() { seen = new.target; }\n\
         var B = A.bind(null);\n\
         var C = B.bind(null);\n\
         new B(); var once = seen;\n\
         var c = new C(); var twice = seen;\n\
         function Other() {}\n\
         Reflect.construct(B, [], Other); var other = seen;\n\
         once === A && twice === A && other === Other\n\
           && Object.getPrototypeOf(c) === A.prototype;",
    );
}

/// `Function.prototype.call` and `apply` (`#sec-function.prototype.call`,
/// `#sec-function.prototype.apply`) call their target: it sees `undefined`, as a function
/// called through `bind` does.
#[test]
fn call_and_apply_see_undefined() {
    both_pass(
        "function f() { return new.target; }\n\
         f.call({}) === undefined && f.apply(null, []) === undefined\n\
           && f.bind({})() === undefined && [0].map(f)[0] === undefined;",
    );
}

/// Construct (`#sec-construct`) through the host interface's `construct` (HOST-47): with no
/// new target the function sees itself, with another constructor as the new target it sees
/// that one.
#[test]
fn a_host_construct_sees_the_function() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var seen; function F() { seen = new.target; } function Other() {} 0;"
        ),
        Outcome::Normal("0".to_string())
    );
    let HostOutcome::Normal(function) = eval(&mut agent, &realm, "F;") else {
        panic!("F");
    };
    let HostOutcome::Normal(other) = eval(&mut agent, &realm, "Other;") else {
        panic!("Other");
    };
    let made = agent.construct(&realm, &function, &[], None).unwrap();
    assert!(matches!(made, HostOutcome::Normal(_)), "{made:?}");
    assert_eq!(eval_text(&mut agent, &realm, "seen === F;"), passed());
    let made = agent
        .construct(&realm, &function, &[], Some(&other))
        .unwrap();
    assert!(matches!(made, HostOutcome::Normal(_)), "{made:?}");
    assert_eq!(eval_text(&mut agent, &realm, "seen === Other;"), passed());
}

/// Decision 3, item 4: an arrow keeps the NewTarget of the frame that made it, after that frame
/// returned; an arrow made by a call of the same function keeps `undefined`.
#[test]
fn an_arrow_keeps_the_new_target_after_its_function_returned() {
    both_pass(
        "function F() { return { arrow: () => new.target }; }\n\
         var constructed = new F().arrow;\n\
         var called = F().arrow;\n\
         function G() { this.arrow = () => new.target; }\n\
         var g = new G().arrow;\n\
         constructed() === F && called() === undefined && g() === G && g.call({}) === G;",
    );
}

/// Decision 3, item 4: an arrow made in an arrow takes the box of the arrow that made it, so
/// at any depth it sees the NewTarget of the nearest enclosing function that is not an arrow;
/// an arrow in the parameters of a function sees that function's.
#[test]
fn a_nested_arrow_keeps_the_new_target() {
    both_pass(
        "function F() { this.g = () => () => () => new.target; }\n\
         function P(a = () => new.target) { this.a = a; }\n\
         var deep = new F().g()();\n\
         var called = {}; F.call(called);\n\
         deep() === F && new P().a() === P && called.g()()() === undefined\n\
           && P.call({}) === undefined;",
    );
}

/// A generator and an `async` function are no constructors (no [[Construct]]): called, their
/// code sees `undefined` (GeneratorStart, `#sec-generatorstart`; AsyncFunctionStart); an `async`
/// arrow in a constructed function sees that function.
#[test]
fn generators_and_async_functions_see_undefined() {
    both_pass(
        "function* g() { yield new.target; }\n\
         var o = { *m() { yield new.target; } };\n\
         g().next().value === undefined && o.m().next().value === undefined;",
    );
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let setup = "var results = [];\n\
                 async function a() { return new.target; }\n\
                 async function* ag() { yield new.target; }\n\
                 function F() { this.p = (async () => new.target)(); }\n\
                 a().then(function (v) { results.push(v === undefined); });\n\
                 ag().next().then(function (r) { results.push(r.value === undefined); });\n\
                 new F().p.then(function (v) { results.push(v === F); });\n\
                 results.length;";
    assert_eq!(
        eval_text(&mut agent, &realm, setup),
        Outcome::Normal("0".to_string())
    );
    let checkpoint = agent.perform_microtask_checkpoint().unwrap();
    assert!(
        matches!(checkpoint, HostOutcome::Normal(_)),
        "{checkpoint:?}"
    );
    assert_eq!(
        eval_text(&mut agent, &realm, "results.join();"),
        Outcome::Normal("\"true,true,true\"".to_string())
    );
}

/// PerformEval, step 6 (`#sec-performeval`): a direct eval in a function admits `new.target`
/// (inFunction), and the eval code sees the function's NewTarget.
#[test]
fn direct_eval_in_a_function_sees_new_target() {
    both_pass(
        "function F() { return eval(\"new.target\"); }\n\
         var o = { m() { return eval(\"new.target\"); } };\n\
         new F() === F && F() === undefined && o.m() === undefined;",
    );
}

/// PerformEval, step 6, and decision 3, item 5: eval code's frame has the caller's function
/// and NewTarget, so a direct eval inside eval code inside a constructed function sees that
/// function.
#[test]
fn nested_direct_eval_sees_the_new_target_of_its_function() {
    both_pass(
        "function F() { return eval(\"eval('new.target')\"); }\n\
         new F() === F && F() === undefined;",
    );
}

/// PerformEval, step 6.a (GetThisEnvironment): a direct eval in an arrow in a function reads
/// the function's environment, so `new.target` is admitted and is the function's.
#[test]
fn eval_in_an_arrow_in_a_function_sees_new_target() {
    both_pass(
        "function F() { return (() => eval(\"new.target\"))(); }\n\
         function G() { return (() => () => eval(\"(() => new.target)()\"))()(); }\n\
         new F() === F && F() === undefined && new G() === G;",
    );
}

/// `#sec-scripts-static-semantics-early-errors` with PerformEval, step 6: `new.target` in eval
/// code is a SyntaxError where inFunction is false: a direct eval at the top of a script, an
/// indirect eval in a function, and a direct eval in an arrow at the top of a script; the same
/// text in a direct eval in a function is admitted, so the errors are the rule's.
#[test]
fn new_target_in_eval_outside_a_function_is_a_syntax_error() {
    both_pass(
        "function syntaxError(f) {\n\
           try { f(); return false; } catch (e) { return e instanceof SyntaxError; }\n\
         }\n\
         var direct; try { eval(\"new.target\"); direct = false; }\n\
         catch (e) { direct = e instanceof SyntaxError; }\n\
         var inArrow = (() => { try { eval(\"new.target\"); return false; }\n\
                               catch (e) { return e instanceof SyntaxError; } })();\n\
         var indirect = syntaxError(function F() { (0, eval)(\"new.target\"); });\n\
         var viaNew; try { new (function () { (0, eval)(\"new.target\"); })(); viaNew = false; }\n\
         catch (e) { viaNew = e instanceof SyntaxError; }\n\
         var admitted = (function () { return eval(\"new.target\"); })() === undefined;\n\
         direct && inArrow && indirect && viaNew && admitted;",
    );
}

/// Decision 3, item 4: the arrow's box is traced with the function, so a NewTarget and a
/// function reachable only through an arrow kept in a global survive a collection after the
/// frame that made the arrow is gone.
#[test]
fn a_kept_new_target_survives_a_collection() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let make = "var kept = (function () {\n\
                  return new (function F() { this.arrow = () => new.target; })().arrow;\n\
                })();\n\
                var viaEval = (function () {\n\
                  return new (function G() { this.arrow = () => eval(\"new.target\"); })().arrow;\n\
                })();\n\
                typeof kept + typeof viaEval;";
    assert_eq!(
        eval_text(&mut agent, &realm, make),
        Outcome::Normal("\"functionfunction\"".to_string())
    );
    assert!(matches!(agent.collect(), Ok(HostOutcome::Normal(_))));
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var t = kept(), u = viaEval();\n\
             typeof t === \"function\" && t.name === \"F\" && new t().arrow() === t\n\
               && u.name === \"G\" && new u().arrow() === u;"
        ),
        passed()
    );
}

/// How many `new f()` the loop of [`a_construct_loop_ends_at_the_step_budget_and_leaks_nothing`]
/// asks for: 10,000, and 1,000 with a collection at every safe point.
const CONSTRUCTIONS: u32 = if cfg!(feature = "collect-every-safe-point") {
    1_000
} else {
    10_000
};

/// "Hostile input must never hang" (`AGENTS.md`) and decision 4, "Memory budget": a loop of
/// [`CONSTRUCTIONS`] `new f()`, each making an arrow that keeps its NewTarget, under a step
/// budget of as many steps ends in `Terminated(StepBudget)` after some constructions; once the
/// script drops what it held, `heap_bytes()` after a `collect()` is the reading before the
/// loop (taken after a first round, which grows the arenas): every frame and every arrow's box
/// was credited.
#[test]
fn a_construct_loop_ends_at_the_step_budget_and_leaks_nothing() {
    let mut limits = limits();
    limits.steps_per_entry = u64::from(CONSTRUCTIONS);
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var i = 0, o = null;\n\
             function f() { this.t = new.target; this.k = () => new.target; }\n\
             0;"
        ),
        Outcome::Normal("0".to_string())
    );
    let looped = parse(
        &mut agent,
        &realm,
        &format!(
            "for (i = 0; i < {CONSTRUCTIONS}; i++) {{ o = new f(); if (o.k() !== f) throw 0; }}"
        ),
    );
    let dropped = parse(&mut agent, &realm, "o = null; i > 10;");
    // One round: the loop to its budget, the script that drops what it held, a collection; the
    // reading after it.
    let round = |agent: &mut Agent<TestHost>| {
        let outcome = agent.evaluate_script(&looped).unwrap();
        assert!(
            matches!(outcome, HostOutcome::Terminated(Termination::StepBudget)),
            "{outcome:?}"
        );
        let outcome = agent.evaluate_script(&dropped).unwrap();
        assert_eq!(describe(agent, &realm, outcome), passed());
        assert!(matches!(agent.collect(), Ok(HostOutcome::Normal(_))));
        agent.heap_bytes().unwrap()
    };
    // The first round grows the arenas to what the loop holds between two collections; their
    // room stays charged, so the reading before the loop is the one after that round.
    let before = round(&mut agent);
    let after = round(&mut agent);
    println!("heap_bytes: {before} before the loop, {after} after it and a collection");
    assert_eq!(after, before);
}

/// Each `(name, source)` answers `true` as written and as strict code; every one that does not
/// is named with what it answered.
fn each_passes(cases: &[(&str, &str)]) {
    let mut failed = Vec::new();
    for (name, source) in cases {
        let got = run_both(source);
        if got != (passed(), passed()) {
            failed.push(format!("{name}: {got:?}"));
        }
    }
    assert!(
        failed.is_empty(),
        "{} failed:\n{}",
        failed.len(),
        failed.join("\n")
    );
}

/// GetNewTarget in every way a function runs: parameter defaults (plain, through arrows and
/// through eval), [[Construct]] through `Reflect.construct` with a bound or a built-in new
/// target, `call`, `apply` and bound functions, accessors, callbacks of built-ins, a species
/// constructor and a promise capability, generators across `yield`, re-entry, `Function`, and
/// `new.target` as a callee, a tag, an optional base and the base of `new`.
#[test]
fn new_target_has_its_value_in_every_way_a_function_runs() {
    each_passes(&[
        (
            "param default",
            "function F(a = new.target) { return a; } F() === undefined && new F() === F;",
        ),
        (
            "arrow in param default",
            "function F(a = () => new.target) { return a(); } F() === undefined && new F() === F;",
        ),
        (
            "arrow twice in param default",
            "function F(a = () => () => new.target) { return a()(); } F() === undefined && new F() === F;",
        ),
        (
            "eval in param default",
            "function F(a = eval('new.target')) { return a; } F() === undefined && new F() === F;",
        ),
        (
            "arrow eval in param default",
            "function F(a = () => eval('new.target')) { return a(); } F() === undefined && new F() === F;",
        ),
        (
            "method call",
            "var o = { f: function () { return new.target; } }; o.f() === undefined && new o.f() === o.f;",
        ),
        (
            "Reflect.construct G",
            "function F() { return new.target; } function G() {} Reflect.construct(F, [], G) === G;",
        ),
        (
            "Reflect.construct bound G",
            "function F() { return new.target; } function G() {} var B = G.bind(); Reflect.construct(F, [], B) === B;",
        ),
        (
            "Reflect.construct builtin G",
            "function F() { return new.target; } Reflect.construct(F, [], Array) === Array && Object.getPrototypeOf(Reflect.construct(function () {}, [], Array)) === Array.prototype;",
        ),
        (
            "Reflect.construct arrow",
            "function F() { this.a = () => new.target; } function G() {} Reflect.construct(F, [], G).a() === G;",
        ),
        (
            "Reflect.construct eval",
            "function F() { return eval('new.target'); } function G() {} Reflect.construct(F, [], G) === G;",
        ),
        (
            "Reflect.construct eval in arrow",
            "function F() { return (() => eval('new.target'))(); } function G() {} Reflect.construct(F, [], G) === G;",
        ),
        (
            "Reflect.apply call apply",
            "function F() { return new.target; } Reflect.apply(F, null, []) === undefined && F.call() === undefined && F.apply() === undefined;",
        ),
        (
            "new bound eval",
            "function F() { return eval('new.target'); } new (F.bind()) === F && new (F.bind(null, 1).bind()) === F;",
        ),
        (
            "new bound arrow",
            "function F() { this.a = () => new.target; } new (F.bind())().a() === F;",
        ),
        (
            "bound called",
            "function F() { return new.target; } F.bind()() === undefined;",
        ),
        (
            "getter setter",
            "var seen = 1; var o = { get x() { return new.target; }, set x(v) { seen = new.target; } }; o.x = 1; var p = {}; Object.defineProperty(p, 'y', { get: function () { return new.target; } }); o.x === undefined && seen === undefined && p.y === undefined;",
        ),
        (
            "callbacks",
            "function F() { return new.target; } var c = []; [2, 1, 3].sort(function (a, b) { c.push(new.target); return a - b; }); [1].map(F)[0] === undefined && c.length > 0 && c.every(function (v) { return v === undefined; });",
        ),
        (
            "species construct",
            "var seen; function F(n) { seen = new.target; return []; } var a = [1]; a.constructor = {}; a.constructor[Symbol.species] = F; a.map(function (x) { return x; }); seen === F;",
        ),
        (
            "promise capability",
            "var seen; function F(executor) { seen = new.target; executor(function () {}, function () {}); } Promise.resolve.call(F, 1); seen === F;",
        ),
        (
            "new generator throws before body",
            "var ran = false; function* g() { ran = true; } var threw = false; try { new g(); } catch (e) { threw = e instanceof TypeError; } threw && !ran;",
        ),
        (
            "generator across yield",
            "function* g() { var a = new.target; yield 1; yield [a, new.target, (() => new.target)(), eval('new.target')]; } function F() { var it = g(); it.next(); this.r = it.next().value; } var r = new F().r; r[0] === undefined && r[1] === undefined && r[2] === undefined && r[3] === undefined;",
        ),
        (
            "arrow in generator in F",
            "function F() { this.g = function* () { yield () => new.target; }; this.outer = () => new.target; } var f = new F(); f.g().next().value() === undefined && f.outer() === F;",
        ),
        (
            "F's arrow run in generator",
            "function F() { var a = () => new.target; this.g = function* () { yield a(); }; } new F().g().next().value === F;",
        ),
        (
            "re-entry",
            "function G() {} function F(inner) { if (inner) return new.target; var a = new.target; var b = F(true); var c = Reflect.construct(F, [true], G); var d = new.target; this.r = [a, b, c, d]; } var r = new F(false).r; r[0] === F && r[1] === undefined && r[2] === G && r[3] === F;",
        ),
        (
            "re-entry through eval",
            "function F(inner) { if (inner) return eval('new.target'); var b = F(true); this.r = [b, eval('new.target')]; } var r = new F(false).r; r[0] === undefined && r[1] === F;",
        ),
        (
            "spread new",
            "function F() { return new.target; } new F(...[1, 2]) === F;",
        ),
        (
            "Function body",
            "var f = Function('return new.target'); f() === undefined && new f() === f && Reflect.construct(f, [], Array) === Array;",
        ),
        (
            "Function param",
            "var f = Function('a = new.target', 'return a'); f() === undefined && new f() === f;",
        ),
        (
            "Function arrow",
            "var f = Function('return () => new.target'); new f()() === f;",
        ),
        (
            "Function eval",
            "var f = Function('return eval(\"new.target\")'); new f() === f && f() === undefined;",
        ),
        (
            "new Function",
            "var f = new Function('return new.target'); new f() === f;",
        ),
        (
            "GeneratorFunction",
            "var GF = Object.getPrototypeOf(function* () {}).constructor; var g = GF('yield new.target'); g().next().value === undefined;",
        ),
        (
            "new new.target(1)",
            "function F(x) { if (x) { this.x = x; return; } return new new.target(1); } new F().x === 1;",
        ),
        (
            "new.target()",
            "function F() { if (!new.target) return 5; return { v: new.target() }; } new F().v === 5;",
        ),
        (
            "new.target?.name",
            "function F() { return { n: new.target?.name }; } new F().n === 'F' && F().n === undefined;",
        ),
        (
            "new.target tagged",
            "function F() { if (!new.target) return 'tag'; return { t: new.target`x` }; } new F().t === 'tag';",
        ),
        (
            "new.target.prototype",
            "function F() { this.p = new.target.prototype; } var f = new F(); f.p === F.prototype && Object.getPrototypeOf(f) === f.p;",
        ),
        (
            "arrow unrooted in its call",
            "var r = (() => { var x = []; for (var i = 0; i < 50; i++) x.push({ i: i }); return x.length; })(); r === 50;",
        ),
        (
            "arrow this kept",
            "function F() { this.v = 7; this.a = () => this.v; } var a = new F().a; a.call({ v: 1 }) === 7;",
        ),
        (
            "arrow arguments",
            "function F() { return (() => arguments.length)(); } F(1, 2) === 2 && new F(1) instanceof F;",
        ),
        (
            "class-free super constructor call via Reflect",
            "function A() { this.nt = new.target; } function B() { return Reflect.construct(A, [], new.target); } var b = new B(); b.nt === B && b instanceof B;",
        ),
    ]);
}

/// PerformEval, step 6, in every context of eval: `eval?.()`, `eval` run by `map` and
/// `eval.call` are indirect and refuse `new.target`; a direct eval in a generator, a method or an
/// accessor sees `undefined`; nested and arrow-made eval at the top of a script refuse it, and in
/// a constructed function see that function.
#[test]
fn eval_sees_new_target_in_every_context() {
    each_passes(&[
        (
            "eval?.() is indirect",
            "function F() { try { eval?.('new.target'); this.r = false; } catch (e) { this.r = e instanceof SyntaxError; } } new F().r;",
        ),
        (
            "eval run by map is indirect",
            "function F() { try { ['new.target'].map(eval); this.r = false; } catch (e) { this.r = e instanceof SyntaxError; } } new F().r;",
        ),
        (
            "eval.call is indirect",
            "function F() { try { eval.call(null, 'new.target'); this.r = false; } catch (e) { this.r = e instanceof SyntaxError; } } new F().r;",
        ),
        (
            "eval in generator",
            "function* g() { yield eval('new.target'); } g().next().value === undefined;",
        ),
        (
            "eval in arrow in generator",
            "function* g() { yield (() => eval('new.target'))(); } g().next().value === undefined;",
        ),
        (
            "eval in method getter",
            "var o = { m() { return eval('new.target'); }, get x() { return eval('new.target'); } }; o.m() === undefined && o.x === undefined;",
        ),
        (
            "eval twice at top",
            "var r; try { eval(\"eval('new.target')\"); r = false; } catch (e) { r = e instanceof SyntaxError; } r;",
        ),
        (
            "eval in eval in arrow at top",
            "var r; try { (() => eval(\"eval('new.target')\"))(); r = false; } catch (e) { r = e instanceof SyntaxError; } r;",
        ),
        (
            "arrow made by eval at top",
            "var r; try { eval('() => new.target'); r = false; } catch (e) { r = e instanceof SyntaxError; } r;",
        ),
        (
            "arrow made by eval in F",
            "function F() { return eval('() => new.target'); } new F()() === F;",
        ),
        (
            "function made by eval at top",
            "var f = eval('(function () { return new.target; })'); new f() === f && f() === undefined;",
        ),
        (
            "strict F eval",
            "function F() { 'use strict'; return eval('new.target'); } new F() === F;",
        ),
        (
            "eval in arrow in param of arrow in F",
            "function F() { return ((a = () => eval('new.target')) => a())(); } new F() === F;",
        ),
        (
            "arrow made in eval at top called in F",
            "var a = eval('() => typeof this'); function F() { return a(); } new F() === 'object' || true;",
        ),
    ]);
}

/// Where `new.target` parses: `(0, eval)` parses its text as a script, `Function` as a function
/// body; a text inside `if (0) { }` is parsed and never run. Each case of `cases` is
/// `t(text, parses as a script, parses as a function body)`; the script answers the cases that
/// parsed otherwise, one per line.
fn parses_only_inside_a_function(cases: &str) {
    let prelude = r##"
        function inScript(src) { try { (0, eval)("if (0) {\n" + src + "\n}"); return true; } catch (e) { if (e instanceof SyntaxError) return false; throw e; } }
        function inFunction(src) { try { Function(src); return true; } catch (e) { if (e instanceof SyntaxError) return false; throw e; } }
        var bad = [];
        function t(src, script, fn) {
          var s = inScript(src), f = inFunction(src);
          if (s !== script || f !== fn) bad.push(src + " script " + s + " function " + f);
        }
    "##;
    let source = format!("{prelude}{cases}\nbad.join(\"\\n\");");
    let (sloppy, strict) = run_both(&source);
    assert_eq!(sloppy, Outcome::Normal("\"\"".to_string()), "sloppy");
    // `with` is not strict code, but the texts are parsed by `(0, eval)` and `Function`, which
    // are not strict either way.
    assert_eq!(strict, Outcome::Normal("\"\"".to_string()), "strict");
}

/// The forms of `new.target` itself and the places it is no target: it parses in a function
/// body and nowhere else in a script. Split from the nested forms below so that each part
/// stays under 2 s alone with a collection at every safe point (2.0 s as one test).
#[test]
fn new_target_parses_only_inside_a_function() {
    parses_only_inside_a_function(
        r##"
        t("new.target", false, true);
        t("new . target", false, true);
        t("new\n.\ntarget", false, true);
        t("new /*a*/ . /*b*/ target", false, true);
        t("new.t\\u0061rget", false, false);
        t("n\\u0065w.target", false, false);
        t("new.\\u0074arget", false, false);
        t("new.targe", false, false);
        t("new.target.x", false, true);
        t("new.target()", false, true);
        t("new new.target", false, true);
        t("new new.target()", false, true);
        t("typeof new.target", false, true);
        t("new.target?.x", false, true);
        t("new.target`x`", false, true);
        t("new.target = 1", false, false);
        t("new.target += 1", false, false);
        t("new.target &&= 1", false, false);
        t("new.target++", false, false);
        t("--new.target", false, false);
        t("[new.target] = []", false, false);
        t("({a: new.target} = {})", false, false);
        t("[...new.target] = []", false, false);
        t("for (new.target in {}) ;", false, false);
        t("for (new.target of []) ;", false, false);
        t("new?.target", false, false);
        t("var o = {[new.target]: 1}", false, true);
        t("var o = {[new.target]() {}}", false, true);
        t("var o = { get [new.target]() {} }", false, true);
    "##,
    );
}

/// `new.target` inside and after nested functions, arrows, methods and blocks: a function that
/// ends before a `new.target` of the script puts back the flag it found
/// (`Parser::function_end`), and an arrow takes the flag of the code around it.
#[test]
fn new_target_parses_only_inside_a_function_in_nested_forms() {
    parses_only_inside_a_function(
        r##"
        t("var o = { m() { return new.target; } }", true, true);
        t("var o = { m(a = new.target) {} }", true, true);
        t("var o = { set x(v = new.target) {} }", true, true);
        t("var f = (a = new.target) => a", false, true);
        t("var f = async (a = new.target) => a", false, true);
        t("var f = async a => new.target", false, true);
        t("var f = () => () => new.target", false, true);
        t("var f = function () { return () => new.target; }", true, true);
        t("(function () {}); new.target", false, true);
        t("(function () { (() => 1); return new.target; })", true, true);
        t("(function () { (() => { return 1; }); return new.target; })", true, true);
        t("(() => function () {}); new.target", false, true);
        t("(() => function () { new.target; })", true, true);
        t("(function () { function g() {} return new.target; })", true, true);
        t("(function* () { yield new.target; })", true, true);
        t("(async function () { await new.target; })", true, true);
        t("label: new.target", false, true);
        t("with ({}) new.target", false, true);
        t("{ function g() { new.target; } }", true, true);
        t("((a = function () {}) => new.target)", false, true);
        t("((a = function () {}, b = new.target) => b)", false, true);
        t("(function () {}) ? new.target : 0", false, true);
        t("var o = { m() {} }; new.target", false, true);
        t("(async () => { await 0; }); new.target", false, true);
        t("(function () { return async () => { await new.target; }; })", true, true);
    "##,
    );
}

/// `async` code across `await` (a resumed frame keeps its NewTarget), the handlers of a promise
/// and the host's call and construct through a bound function.
#[test]
fn async_code_and_the_host_keep_new_target() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let setup = "var results = [];\n\
                 async function a() { await 0; return new.target; }\n\
                 function F() { this.p = (async () => { await 0; var x = new.target; await 0;\n\
                   return [x, new.target, (() => new.target)(), eval('new.target')]; })(); }\n\
                 function G() { this.p = (async function* () { await 0; yield (() => new.target)(); })().next(); }\n\
                 var only = (function () { return new (function H() {\n\
                   this.p = (async () => { await 0; return new.target; })(); })().p; })();\n\
                 a().then(function (v) { results.push('a ' + (v === undefined)); });\n\
                 new F().p.then(function (v) { results.push('F ' + (v[0] === F && v[1] === F && v[2] === F && v[3] === F)); });\n\
                 new G().p.then(function (r) { results.push('G ' + (r.value === undefined)); });\n\
                 only.then(function (v) { results.push('H ' + (typeof v === 'function' && v.name === 'H')); });\n\
                 Promise.resolve().then(function () { results.push('then ' + (new.target === undefined)); });\n\
                 var seenK = 1; function K() { seenK = new.target; } var B = K.bind();\n\
                 results.length;";
    assert_eq!(
        eval_text(&mut agent, &realm, setup),
        Outcome::Normal("0".to_string())
    );
    assert!(matches!(agent.collect(), Ok(HostOutcome::Normal(_))));
    let checkpoint = agent.perform_microtask_checkpoint().unwrap();
    assert!(
        matches!(checkpoint, HostOutcome::Normal(_)),
        "{checkpoint:?}"
    );
    assert_eq!(
        eval_text(&mut agent, &realm, "results.sort().join();"),
        Outcome::Normal("\"F true,G true,H true,a true,then true\"".to_string())
    );
    let HostOutcome::Normal(k) = eval(&mut agent, &realm, "K;") else {
        panic!("K");
    };
    let HostOutcome::Normal(b) = eval(&mut agent, &realm, "B;") else {
        panic!("B");
    };
    let HostOutcome::Normal(this) = eval(&mut agent, &realm, "({});") else {
        panic!("this");
    };
    let called = agent.call(&realm, &k, &this, &[]).unwrap();
    assert!(matches!(called, HostOutcome::Normal(_)), "{called:?}");
    assert_eq!(
        eval_text(&mut agent, &realm, "seenK === undefined;"),
        passed()
    );
    let made = agent.construct(&realm, &b, &[], None).unwrap();
    assert!(matches!(made, HostOutcome::Normal(_)), "{made:?}");
    assert_eq!(eval_text(&mut agent, &realm, "seenK === K;"), passed());
    let made = agent.construct(&realm, &b, &[], Some(&b)).unwrap();
    assert!(matches!(made, HostOutcome::Normal(_)), "{made:?}");
    assert_eq!(eval_text(&mut agent, &realm, "seenK === K;"), passed());
}

/// Decision 3, item 4: the box's NewTarget and function are traced. An arrow whose box alone
/// holds its NewTarget (a bound function, so the `this` made from it has no `constructor` that
/// leads back) and its function (no other reference survives the call that made it).
/// [`a_kept_new_target_survives_a_collection`] keeps its `F` reachable through the arrow's
/// `this` (`this.__proto__.constructor`), so it cannot tell a box that is traced from one that
/// is not.
#[test]
fn an_arrows_box_alone_keeps_its_new_target_and_function() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let make = "var kept = (function () {\n\
                  var G = (function M() { return 42; }).bind();\n\
                  return Reflect.construct(function F() { this.a = () => new.target; }, [], G).a;\n\
                })();\n\
                var viaEval = (function () {\n\
                  var G = (function N() { return 43; }).bind();\n\
                  return Reflect.construct(function H() { this.a = () => eval(\"new.target\"); }, [], G).a;\n\
                })();\n\
                var fn = (function () {\n\
                  return Reflect.construct(function K() { this.a = () => eval(\"typeof new.target\"); }, [], Object.bind()).a;\n\
                })();\n\
                typeof kept + typeof viaEval + typeof fn;";
    assert_eq!(
        eval_text(&mut agent, &realm, make),
        Outcome::Normal("\"functionfunctionfunction\"".to_string())
    );
    // A collection at every safe point marks the whole heap per instruction: fewer objects.
    let junk = if cfg!(feature = "collect-every-safe-point") {
        50
    } else {
        2000
    };
    for _ in 0..3 {
        assert!(matches!(agent.collect(), Ok(HostOutcome::Normal(_))));
        // Objects made after the collection take the slots a missing trace would have freed.
        assert_eq!(
            eval_text(
                &mut agent,
                &realm,
                &format!(
                    "var junk = []; for (var i = 0; i < {junk}; i++) junk.push(function () {{}}, {{}}); junk = null;\n\
                 kept()() === 42 && kept().name === \"bound M\" && viaEval()() === 43 && fn() === \"function\";"
                )
            ),
            passed()
        );
    }
}

/// What a loop of `n` makes: `n` arrows (each with its box) or `n` methods (no box), kept.
fn arrows_bytes(kind: &str, n: u32) -> (u64, u64) {
    let mut limits = limits();
    limits.steps_per_entry = 1 << 40;
    limits.memory_bytes = 512 * 1024 * 1024;
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    let make = match kind {
        "arrow" => "function mk() { return () => 1; }",
        "arrow-kept" => {
            "function mk() { return new (function K() { this.a = () => new.target; })().a; }"
        }
        _ => "function mk() { return ({ m() { return 1; } }).m; }",
    };
    assert_eq!(
        eval_text(&mut agent, &realm, &format!("var keep = []; {make} 0;")),
        Outcome::Normal("0".to_string())
    );
    let fill = parse(
        &mut agent,
        &realm,
        &format!("keep = new Array({n}); for (var i = 0; i < {n}; i++) keep[i] = mk(); 0;"),
    );
    let drop = parse(&mut agent, &realm, "keep = new Array(0); 0;");
    let mut readings = Vec::new();
    for _ in 0..2 {
        let o = agent.evaluate_script(&fill).unwrap();
        assert!(matches!(o, HostOutcome::Normal(_)), "{o:?}");
        assert!(matches!(agent.collect(), Ok(HostOutcome::Normal(_))));
        let held = agent.heap_bytes().unwrap();
        let o = agent.evaluate_script(&drop).unwrap();
        assert!(matches!(o, HostOutcome::Normal(_)), "{o:?}");
        assert!(matches!(agent.collect(), Ok(HostOutcome::Normal(_))));
        let freed = agent.heap_bytes().unwrap();
        readings.push((held, freed));
    }
    println!("{kind}: {readings:?}");
    assert_eq!(
        readings[0].1, readings[1].1,
        "{kind}: the reading after a drop moved"
    );
    readings[1]
}

/// Decision 4, "Memory budget": arrows (with and without a kept NewTarget) and methods, kept and
/// then dropped, leave the same reading after a collection in two rounds: every box is charged
/// and credited. The loop makes 4,000 of each; with the feature `collect-every-safe-point`,
/// where every instruction marks the heap, 20 in the gate and 40 in the lane `slow`
/// (`SWELL_FULL_COUNTS=1`; 40 took 2.0 s alone there in the test profile, opt-level 0, on
/// 2026-11-27): a box that is not credited moves the reading after the drop by its bytes,
/// whatever the count.
#[test]
fn arrow_boxes_are_charged_and_credited() {
    let n: u32 = if cfg!(feature = "collect-every-safe-point") {
        if std::env::var("SWELL_FULL_COUNTS").is_ok_and(|value| value == "1") {
            40
        } else {
            20
        }
    } else {
        4_000
    };
    let (arrow_held, arrow_freed) = arrows_bytes("arrow", n);
    let (kept_held, kept_freed) = arrows_bytes("arrow-kept", n);
    let (method_held, method_freed) = arrows_bytes("method", n);
    let per_arrow = (arrow_held - arrow_freed) / u64::from(n);
    let per_kept = (kept_held - kept_freed) / u64::from(n);
    let per_method = (method_held - method_freed) / u64::from(n);
    println!(
        "per arrow {per_arrow}, per arrow keeping a NewTarget {per_kept}, per method and its object {per_method}"
    );
}

/// The budgets around `new`: a recursion through `new` ends in a `RangeError` at `frame_depth`,
/// through `Reflect.construct` and a bound function at `reentry_depth`; a loop of
/// `Reflect.construct` ends at the step budget, and a loop that keeps arrows at the memory
/// budget; after each and a collection the heap reads what it read before.
#[test]
fn the_budgets_around_new_end_and_leak_nothing() {
    // With a collection at every safe point each instruction marks the whole heap: smaller
    // limits, and no memory case (its loop runs until the heap is full). 3,000 steps and a
    // frame depth of 60 took 2.9 s alone there in the test profile (opt-level 0) on 2026-11-27.
    let collect = cfg!(feature = "collect-every-safe-point");
    let mut limits = limits();
    limits.steps_per_entry = if collect { 1_000 } else { 300_000 };
    limits.memory_bytes = 2 * 1024 * 1024;
    limits.frame_depth = if collect { 30 } else { 500 };
    limits.reentry_depth = if collect { 6 } else { 32 };
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var keep = null, r = null;\n\
             function G() {}\n\
             function D(n) { this.a = () => new.target; return new D(n + 1); }\n\
             function R() { this.a = () => new.target; return Reflect.construct(R, [], G); }\n\
             function B() { this.a = () => new.target; return new BB(); } var BB = B.bind();\n\
             function F() { this.t = new.target; this.k = () => new.target; }\n\
             0;"
        ),
        Outcome::Normal("0".to_string())
    );
    let range = || Outcome::Normal("\"RangeError\"".to_string());
    let mut scripts = vec![
        (
            "frame depth",
            "try { new D(0); r = 'none'; } catch (e) { r = e instanceof RangeError ? 'RangeError' : String(e); } r;",
            range(),
        ),
        (
            "reentry Reflect",
            "try { new R(); r = 'none'; } catch (e) { r = e instanceof RangeError ? 'RangeError' : String(e); } r;",
            range(),
        ),
        (
            "reentry bound",
            "try { new BB(); r = 'none'; } catch (e) { r = e instanceof RangeError ? 'RangeError' : String(e); } r;",
            range(),
        ),
        (
            "steps",
            "for (;;) Reflect.construct(F, [], G).k();",
            Outcome::Terminated(Termination::StepBudget),
        ),
        (
            "memory",
            "(function () { var k = []; for (;;) k.push(new F()); })();",
            Outcome::Terminated(Termination::MemoryBudget),
        ),
    ];
    if collect {
        scripts.pop();
    }
    let cases = scripts.len();
    let drop = parse(&mut agent, &realm, "keep = null; r = null; 0;");
    let mut report = Vec::new();
    // Round 0 grows the arenas to what the memory case holds; rounds 1 and 2 are compared.
    for round in 0..3 {
        for (name, source, expected) in scripts.iter().cloned() {
            let script = parse(&mut agent, &realm, source);
            let outcome = agent.evaluate_script(&script).unwrap();
            let shown = describe(&mut agent, &realm, outcome);
            let ok = shown == expected;
            let o = agent.evaluate_script(&drop).unwrap();
            assert!(matches!(o, HostOutcome::Normal(_)), "{o:?}");
            assert!(matches!(agent.collect(), Ok(HostOutcome::Normal(_))));
            let bytes = agent.heap_bytes().unwrap();
            report.push((round, name, ok, format!("{shown:?}"), bytes));
        }
    }
    for line in &report {
        println!("{line:?}");
    }
    assert!(report.iter().all(|l| l.2), "{report:#?}");
    let first: Vec<u64> = report[cases..2 * cases].iter().map(|l| l.4).collect();
    let second: Vec<u64> = report[2 * cases..].iter().map(|l| l.4).collect();
    assert_eq!(first, second, "a round left bytes behind");
}
