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
