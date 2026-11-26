//! `async *` methods of object literals (MethodDefinition : AsyncGeneratorMethod,
//! `#sec-async-generator-function-definitions`,
//! `#sec-asyncgenerator-definitions-propertydefinitionevaluation`, `#sec-definemethodproperty`):
//! the template of a method whose `generator` and `asynchronous` are both set is an async
//! generator of kind method, made by the instantiation every async generator template takes, given
//! its object as [[HomeObject]] by MakeMethod and defined on its object by DefineMethod; it runs
//! as the async generators of `story:async-generators-and-their-request-queue` do, on the
//! suspended frames of `docs/design/engine.md`, decision 3, "Async generators on suspended
//! frames". Written by `story:async-generator-methods-in-object-literals`. Run also with the
//! feature `collect-every-safe-point`.
#![cfg(test)]

use crate::support;

use std::cell::RefCell;

use support::host::{TestHost, agent_with, limits, realm};
use support::script::{Outcome as Shown, define_function, eval, eval_text};
use swell::{
    Agent, Completion, Cx, HostCall, InterruptHandle, Limits, Outcome, PromiseState, Realm, Root,
    Termination, Value,
};

/// A checkpoint that must answer `Normal`.
fn normal(agent: &mut Agent<TestHost>) {
    match agent.perform_microtask_checkpoint().unwrap() {
        Outcome::Normal(_) => {}
        other => panic!("the checkpoint answered {other:?}"),
    }
}

/// A checkpoint that must answer `Terminated`: its reason.
fn terminated(agent: &mut Agent<TestHost>) -> Termination {
    match agent.perform_microtask_checkpoint().unwrap() {
        Outcome::Terminated(termination) => termination,
        other => panic!("the checkpoint answered {other:?}"),
    }
}

/// The script `source`, which must complete normally: its value, rooted.
fn run(agent: &mut Agent<TestHost>, realm: &Realm, source: &str) -> Root {
    match eval(agent, realm, source) {
        Outcome::Normal(root) => root,
        other => panic!("{source:?} answered {other:?}"),
    }
}

/// The text of `String(expression)`, read by a script.
fn text(agent: &mut Agent<TestHost>, realm: &Realm, expression: &str) -> String {
    match eval_text(agent, realm, &format!("String({expression})")) {
        Shown::Normal(shown) => shown.trim_matches('"').to_string(),
        other => panic!("{expression}: {other:?}"),
    }
}

thread_local! {
    /// The interrupt handle of the agent of the test that runs on this thread, for the host
    /// function `interruptNow`.
    static INTERRUPT: RefCell<Option<InterruptHandle>> = const { RefCell::new(None) };
}

/// The host function `interruptNow`: it sets the interrupt flag of the test's agent.
fn interrupt_now(_cx: &mut Cx<'_, TestHost>, _call: HostCall<'_>) -> Completion<Value> {
    INTERRUPT.with(|cell| {
        if let Some(handle) = cell.borrow().as_ref() {
            handle.interrupt();
        }
    });
    Ok(Value::Undefined)
}

/// An agent of the test host with `limits` and a realm whose global `interruptNow` is the host
/// function of this file.
fn new_agent(limits: Limits) -> (Agent<TestHost>, Realm) {
    let mut agent = agent_with(limits).unwrap();
    INTERRUPT.with(|cell| *cell.borrow_mut() = Some(agent.interrupt_handle()));
    let realm = realm(&mut agent);
    define_function(&mut agent, &realm, "interruptNow", 0, interrupt_now, false);
    (agent, realm)
}

/// `source` run in a fresh agent, sloppy and with a `"use strict";` prologue: `String(probe)`
/// reads `before`, and after one checkpoint that answers `Normal`, `after`.
fn both_settle(source: &str, probe: &str, before: &str, after: &str) {
    for text_of in [source.to_string(), format!("\"use strict\";\n{source}")] {
        let (mut agent, realm) = new_agent(limits());
        run(&mut agent, &realm, &text_of);
        assert_eq!(text(&mut agent, &realm, probe), before, "{text_of}");
        normal(&mut agent);
        assert_eq!(text(&mut agent, &realm, probe), after, "{text_of}");
    }
}

/// AsyncGeneratorMethod's MethodDefinitionEvaluation
/// (`#sec-asyncgenerator-definitions-propertydefinitionevaluation`) and AsyncGeneratorStart
/// (`#sec-asyncgeneratorstart`): a call of the method runs nothing of its body and answers an
/// `[object AsyncGenerator]`; each `next` answers a promise at once and runs the body to its next
/// `yield` with the `this` and the arguments of the call, and a later job fulfils the promise with
/// an iterator result: `{ value, done: false }` for a `yield` (its operand awaited),
/// `{ value, done: true }` for the end of the body, `{ value: undefined, done: true }` after it.
#[test]
fn an_async_generator_method_yields_promises_of_iterator_results() {
    both_settle(
        "var log = [];\n\
         function show(r) { return JSON.stringify(r); }\n\
         var o = { base: 10, async *g(a) { log.push('start ' + a); var x = yield a + this.base;\n\
           log.push('got ' + x); yield Promise.resolve(x + arguments.length); return 'end'; } };\n\
         var it = o.g(1);\n\
         log.push('made ' + Object.prototype.toString.call(it));\n\
         var p1 = it.next('ignored');\n\
         log.push('answered ' + (p1 instanceof Promise));\n\
         p1.then(function (r) { log.push(show(r));\n\
           return it.next(5); }).then(function (r) { log.push(show(r));\n\
           return it.next(); }).then(function (r) { log.push(show(r));\n\
           return it.next(); }).then(function (r) { log.push(show(r)); });",
        "log.join(' | ')",
        "made [object AsyncGenerator] | start 1 | answered true",
        "made [object AsyncGenerator] | start 1 | answered true | {\"value\":11,\"done\":false} | \
         got 5 | {\"value\":6,\"done\":false} | {\"value\":\"end\",\"done\":true} | {\"done\":true}",
    );
}

/// The `steps_per_entry` of the step budget case of
/// [`an_async_generator_method_ends_at_each_budget_and_at_an_interrupt`].
const STEPS: u64 = 1_000;

/// The room of the memory budget case of
/// [`an_async_generator_method_ends_at_each_budget_and_at_an_interrupt`] above the reading of a
/// fresh realm.
const FILL_ROOM: u64 = 1024 * 1024;

/// A method `o.g` whose body, resumed by a job after its `await`, runs `ending`; the script calls
/// `next` once and keeps its promise in `p`, then the checkpoint runs the job and answers
/// `Terminated(termination)`: its `last_entry_steps()`, read at once. After a `collect()`,
/// `resumed` reads 1, and the promise of the `next` reads `Pending` through `promise_state`, as
/// it does after a later checkpoint.
fn ends_in(
    agent: &mut Agent<TestHost>,
    realm: &Realm,
    ending: &str,
    termination: Termination,
) -> u64 {
    run(
        agent,
        realm,
        &format!(
            "var resumed = 0;\n\
             var o = {{ async *g() {{ await null; resumed++; {ending} }} }};\n\
             var p = o.g().next();"
        ),
    );
    assert_eq!(terminated(agent), termination);
    let steps = agent.last_entry_steps().unwrap();
    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
    assert_eq!(text(agent, realm, "resumed"), "1");
    let promise = run(agent, realm, "p");
    assert!(matches!(
        agent.promise_state(&promise),
        Ok(PromiseState::Pending)
    ));
    normal(agent);
    assert!(matches!(
        agent.promise_state(&promise),
        Ok(PromiseState::Pending)
    ));
    steps
}

/// "Async generators on suspended frames", item 5, for a method: a body that loops without end
/// after an `await` ends the checkpoint that resumes it in `Terminated(StepBudget)` with
/// `steps_per_entry` [`STEPS`], and `last_entry_steps()` reads at most [`STEPS`]; a body that
/// grows an array without end, Strings that double each kept, in `Terminated(MemoryBudget)` with
/// a memory budget of [`FILL_ROOM`] over a realm; and a body that sets the interrupt flag through
/// the interrupt handle and then loops, in `Terminated(Interrupted)`. After each the promise of
/// the `next` reads `Pending` ([`ends_in`]).
#[test]
fn an_async_generator_method_ends_at_each_budget_and_at_an_interrupt() {
    let mut steps = limits();
    steps.steps_per_entry = STEPS;
    let (mut agent, realm) = new_agent(steps);
    let spent = ends_in(&mut agent, &realm, "for (;;) {}", Termination::StepBudget);
    assert!(spent <= STEPS, "{spent}");

    let realm_bytes = {
        let (agent, _realm) = new_agent(limits());
        agent.heap_bytes().unwrap()
    };
    let mut memory = limits();
    memory.memory_bytes = realm_bytes + FILL_ROOM;
    let (mut agent, realm) = new_agent(memory);
    ends_in(
        &mut agent,
        &realm,
        "var a = []; for (var n = 1024; ; n = n * 2) a.push('x'.repeat(n));",
        Termination::MemoryBudget,
    );

    let (mut agent, realm) = new_agent(limits());
    ends_in(
        &mut agent,
        &realm,
        "interruptNow(); for (;;) {}",
        Termination::Interrupted,
    );
}

/// MakeMethod (`#sec-makemethod`) gives the method its object as [[HomeObject]], which `super`
/// reads (`#sec-super-keyword-runtime-semantics-evaluation`): in an `async *` method, also when
/// the literal is eval code, `super.x` in a `return` and in a `yield` reads the property of the
/// prototype of that object, and nothing is refused. DefineMethodProperty
/// (`#sec-definemethodproperty`) with enumerable true defines the method on that object: a data
/// property, writable, enumerable and configurable, in the order of the literal among the
/// others.
#[test]
fn an_async_generator_method_has_its_object_as_home_object() {
    let (mut agent, realm) = new_agent(limits());
    let before = agent.refusals();
    for source in [
        "({ __proto__: { x: 'p' }, async *m() { return super.x; } })",
        "({ __proto__: { x: 'p' }, async *m() { yield super.x; } })",
    ] {
        assert_eq!(
            eval_text(
                &mut agent,
                &realm,
                &format!(
                    "var read = 'pending';\n\
                     (function () {{ eval(\"{source}\").m().next()\n\
                       .then(function (r) {{ read = r.value; }}); }})();\n\
                     read;"
                ),
            ),
            Shown::Normal("\"pending\"".to_string()),
            "{source}"
        );
        normal(&mut agent);
        assert_eq!(
            eval_text(&mut agent, &realm, "read"),
            Shown::Normal("\"p\"".to_string()),
            "{source}"
        );
    }
    assert_eq!(agent.refusals(), before);
    both_settle(
        "var o = { a: 1, async *g() { yield this === o; }, b: 2 }, seen = 'pending';\n\
         var d = Object.getOwnPropertyDescriptor(o, 'g');\n\
         var shape = d.value === o.g && d.writable && d.enumerable && d.configurable &&\n\
           Object.keys(o).join() === 'a,g,b';\n\
         o.g().next().then(function (r) { seen = r.value; });",
        "shape + ' ' + seen",
        "true pending",
        "true true",
    );
}

/// The MethodDefinitionEvaluation of an AsyncGeneratorMethod
/// (`#sec-asyncgenerator-definitions-propertydefinitionevaluation`): the closure's [[Prototype]]
/// is `%AsyncGeneratorFunction.prototype%`, it has a `prototype` { [[Writable]]: true,
/// [[Enumerable]]: false, [[Configurable]]: false } whose [[Prototype]] is
/// `%AsyncGeneratorPrototype%` and which has no `constructor`, each method its own; the
/// generator the method makes inherits from that `prototype`; it has no [[Construct]], so `new`
/// throws a `TypeError` (`#sec-evaluatenew`, step 7); `length` counts its parameters, and its
/// source text starts at `async` (`#sec-function.prototype.tostring`).
#[test]
fn an_async_generator_method_has_a_prototype_and_no_construct() {
    both_settle(
        "var agfp = Object.getPrototypeOf(async function* () {});\n\
         var o = { async *m(x, y) { yield x; }, async *n() {} }, f = o.m, constructed = 'none';\n\
         try { new o.m(); constructed = 'constructed'; }\n\
         catch (e) { constructed = e instanceof TypeError; }\n\
         var d = Object.getOwnPropertyDescriptor(f, 'prototype');\n\
         var shape = Object.getPrototypeOf(f) === agfp &&\n\
           d.writable === true && d.enumerable === false && d.configurable === false &&\n\
           Object.getPrototypeOf(d.value) === agfp.prototype &&\n\
           !Object.prototype.hasOwnProperty.call(d.value, 'constructor') &&\n\
           o.n.prototype !== d.value && Object.getPrototypeOf(o.m()) === d.value &&\n\
           Object.prototype.toString.call(f) === '[object AsyncGeneratorFunction]' &&\n\
           f.length === 2 && f.name === 'm' && f.toString() === 'async *m(x, y) { yield x; }';",
        "shape + ' ' + constructed",
        "true true",
        "true true",
    );
}

/// SetFunctionName (`#sec-setfunctionname`) through DefineMethod: an `async *` method is named by
/// its key, a computed one by the key computed when the literal is evaluated, a Symbol key as
/// `[description]`; `get`, `set`, `async` and `await` are names like any other. The
/// ComputedPropertyName is read with the [Yield] and [Await] of the code around the literal
/// (`ClassElementName[?Yield, ?Await]`): in an `async` function an `await` there suspends that
/// function, and in a generator a `yield` there suspends that generator; each value is the key.
#[test]
fn a_computed_async_generator_method_gets_its_name() {
    both_settle(
        "var k = 'dyn', s = Symbol('sym'), names = 'pending';\n\
         var o = { async *[k + 1]() {}, async *[s](a, b = 0) {}, async *plain(a, b) {},\n\
           async *0() {}, async *'str'() {}, async *get() {}, async *set() {},\n\
           async *async() {}, async *await() {} };\n\
         async function outer() { return { async *[await 'awaited']() { yield 7; } }; }\n\
         outer().then(function (made) {\n\
           return made.awaited().next().then(function (r) {\n\
             names = Object.keys(made).join() + ' ' + made.awaited.name + ' ' + r.value; });\n\
         });\n\
         function* gen() { return { async *[yield 'key?']() {} }; }\n\
         var it = gen(), asked = it.next().value, made = it.next('yielded').value;\n\
         var direct = o.dyn1.name === 'dyn1' && o[s].name === '[sym]' && o.plain.name === 'plain' &&\n\
           o[0].name === '0' && o.str.name === 'str' && o.get.name === 'get' &&\n\
           o.set.name === 'set' && o.async.name === 'async' && o.await.name === 'await' &&\n\
           o.plain.length === 2 && o[s].length === 1 && asked === 'key?' &&\n\
           Object.keys(made).join() === 'yielded' && made.yielded.name === 'yielded';",
        "direct + ' ' + names",
        "true pending",
        "true awaited awaited 7",
    );
}
