//! Async generators (`story:async-generators-and-their-request-queue`;
//! `#sec-asyncgenerator-objects`): AsyncGeneratorStart (`#sec-asyncgeneratorstart`), `yield`
//! with the `await` of its operand (`#sec-asyncgeneratoryield`), `await`, `next`, `return` and
//! `throw` of `%AsyncGeneratorPrototype%` with the request queue and its drain
//! (`#sec-asyncgeneratorenqueue`, `#sec-asyncgeneratorcompletestep`,
//! `#sec-asyncgeneratordrainqueue`, `#sec-asyncgeneratorawaitreturn`), on the suspended frames of
//! `docs/design/engine.md`, decision 3, "Async generators on suspended frames", items 1 to 5, and
//! HOST-204 to HOST-206 (`docs/design/host-interface.md`, section 10). Run also with the feature
//! `collect-every-safe-point`.
//!
//! A test that reads `heap_bytes()` around an entry parses that entry's script before the first
//! reading and declares the global names it assigns in an earlier script, as
//! `tests/async_functions.rs` does.
#![cfg(test)]

use crate::support;

use std::cell::RefCell;

use support::host::{TestHost, agent_with, limits, realm, stack_with_room};
use support::script::{Outcome as Shown, define_function, eval, eval_text, parse};
use support::stack::{BUILTINS_REENTRY, on_bounded_stack, stack_agent, stack_limits, test_path};
use swell::{
    Agent, CheckpointReport, Completion, Cx, HostCall, InterruptHandle, Limits, Outcome,
    PromiseState, PropertyKey, Realm, Root, Script, Termination, Value,
};

/// A checkpoint that must answer `Normal`: its report.
fn normal(agent: &mut Agent<TestHost>) -> CheckpointReport {
    match agent.perform_microtask_checkpoint().unwrap() {
        Outcome::Normal(report) => report,
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

/// A checkpoint report with the counts `promise_jobs` and `host_microtasks`.
fn report(promise_jobs: u64, host_microtasks: u64) -> CheckpointReport {
    CheckpointReport {
        promise_jobs,
        host_microtasks,
    }
}

/// The script `source`, which must complete normally: its value, rooted.
fn run(agent: &mut Agent<TestHost>, realm: &Realm, source: &str) -> Root {
    match eval(agent, realm, source) {
        Outcome::Normal(root) => root,
        other => panic!("{source:?} answered {other:?}"),
    }
}

/// The parsed script `script`, which must complete normally.
fn run_parsed(agent: &mut Agent<TestHost>, script: &Script) {
    match agent.evaluate_script(script).unwrap() {
        Outcome::Normal(_) => {}
        other => panic!("the script answered {other:?}"),
    }
}

/// The text of `String(expression)`, read by a script.
fn text(agent: &mut Agent<TestHost>, realm: &Realm, expression: &str) -> String {
    match eval_text(agent, realm, &format!("String({expression})")) {
        Shown::Normal(shown) => shown.trim_matches('"').to_string(),
        other => panic!("{expression}: {other:?}"),
    }
}

/// The state of the promise `promise` holds, read with `promise_state`: `pending`, `fulfilled`
/// with an iterator result read through `cx.get` as `{value} {done}` (another value as
/// [`support::script::show`] writes it), or `rejected` with the reason as
/// [`support::script::show_thrown`] writes it.
fn state_of_root(agent: &mut Agent<TestHost>, realm: &Realm, promise: &Root) -> String {
    match agent.promise_state(promise).unwrap() {
        PromiseState::Pending => "pending".to_string(),
        PromiseState::Fulfilled(value) => agent
            .with_cx(realm, |cx| {
                let value = cx.get(&value).unwrap();
                match value {
                    Value::Object(result) => {
                        let value_key = PropertyKey::String(cx.new_string("value").unwrap());
                        let done_key = PropertyKey::String(cx.new_string("done").unwrap());
                        let inner = cx.get_property(result, value_key).unwrap().unwrap();
                        let done = cx.get_property(result, done_key).unwrap().unwrap();
                        format!(
                            "fulfilled {} {}",
                            support::script::show(cx, inner),
                            support::script::show(cx, done)
                        )
                    }
                    other => format!("fulfilled {}", support::script::show(cx, other)),
                }
            })
            .unwrap(),
        PromiseState::Rejected(value) => agent
            .with_cx(realm, |cx| {
                let value = cx.get(&value).unwrap();
                format!("rejected {}", support::script::show_thrown(cx, value))
            })
            .unwrap(),
    }
}

/// The state of the promise the expression `name` reads in `realm` ([`state_of_root`]).
fn state(agent: &mut Agent<TestHost>, realm: &Realm, name: &str) -> String {
    let promise = run(agent, realm, name);
    state_of_root(agent, realm, &promise)
}

/// `heap_bytes()` after a `collect()`.
fn collected(agent: &mut Agent<TestHost>) -> u64 {
    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
    agent.heap_bytes().unwrap()
}

/// The host function `checkpointHere`: a checkpoint through `cx`, inside the running entry.
fn checkpoint_here(cx: &mut Cx<'_, TestHost>, _call: HostCall<'_>) -> Completion<Value> {
    cx.perform_microtask_checkpoint()?;
    Ok(Value::Undefined)
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

/// An agent of the test host with `limits` and a realm whose globals `checkpointHere` and
/// `interruptNow` are the host functions of this file.
fn new_agent(limits: Limits) -> (Agent<TestHost>, Realm) {
    let mut agent = agent_with(limits).unwrap();
    INTERRUPT.with(|cell| *cell.borrow_mut() = Some(agent.interrupt_handle()));
    let realm = realm(&mut agent);
    define_function(
        &mut agent,
        &realm,
        "checkpointHere",
        0,
        checkpoint_here,
        false,
    );
    define_function(&mut agent, &realm, "interruptNow", 0, interrupt_now, false);
    (agent, realm)
}

/// The test limits with `steps_per_entry` `steps`.
fn limits_with(steps: u64) -> Limits {
    let mut limits = limits();
    limits.steps_per_entry = steps;
    limits
}

/// The local slots of the async generators the tests of the charge make (item 3 of "Suspended
/// frames" states the charge of a frame with L = 200).
const LOCALS: usize = 200;

/// The bytes item 3 states for [`LOCALS`] local slots: 16 bytes per slot.
const FRAME_FLOOR: u64 = 16 * LOCALS as u64;

/// A `var` statement of [`LOCALS`] names, each a local slot of the frame it is written in.
fn locals() -> String {
    let names: Vec<String> = (0..LOCALS).map(|i| format!("v{i}")).collect();
    format!("var {};", names.join(", "))
}

/// The bytes of one entry of the request queue of an async generator: the completion's tag and
/// value, the capability's promise and its two functions (`object/async_generator.rs`,
/// `Request`, which the library's test `a_request_keeps_its_size` reads), on this target.
const REQUEST_BYTES: u64 = 56;

/// The room of the memory budget tests above the reading of a fresh realm.
const FILL_ROOM: u64 = 1024 * 1024;

/// Statements that spend any room in a few steps and safe points: Strings that double, each
/// kept, until the charge of one is refused.
const FILL: &str = "var a = []; for (var n = 1024; ; n = n * 2) a.push(\"x\".repeat(n));";

/// An agent of this file whose memory budget is the reading of `heap_bytes()` of a fresh realm,
/// measured in an agent of its own, plus `room`, with `steps_per_entry` `steps`.
fn memory_agent(room: u64, steps: u64) -> (Agent<TestHost>, Realm) {
    let realm_bytes = {
        let (agent, _realm) = new_agent(limits());
        agent.heap_bytes().unwrap()
    };
    let mut limits = limits_with(steps);
    limits.memory_bytes = realm_bytes + room;
    new_agent(limits)
}

/// AsyncGeneratorStart and `next` (`#sec-asyncgenerator-prototype-next`): the call of an async
/// generator function runs nothing of its body; each `next` answers a promise at once, runs the
/// body to its next `yield` in the call when the generator is suspended, and the promise is
/// fulfilled with an iterator result in a later job: `{ value, done: false }` for a `yield`,
/// `{ value, done: true }` for the end of the body, `{ value: undefined, done: true }` after it.
/// The value given to `next` is the value of the `yield`. The object is an
/// `[object AsyncGenerator]`.
#[test]
fn an_async_generator_yields_promises_of_iterator_results() {
    let (mut agent, realm) = new_agent(limits());
    run(
        &mut agent,
        &realm,
        "var log = [];\n\
         function show(r) { return JSON.stringify(r); }\n\
         async function* g(a) { log.push('start ' + a); var x = yield 1; log.push('got ' + x);\n\
           yield x + 1; return 'end'; }\n\
         var it = g('a');\n\
         log.push('made ' + Object.prototype.toString.call(it));\n\
         var p1 = it.next('ignored');\n\
         log.push('answered ' + (p1 instanceof Promise));\n\
         p1.then(function (r) { log.push(show(r));\n\
           return it.next(5); }).then(function (r) { log.push(show(r));\n\
           return it.next(); }).then(function (r) { log.push(show(r));\n\
           return it.next(); }).then(function (r) { log.push(show(r)); });",
    );
    assert_eq!(
        text(&mut agent, &realm, "log.join(' | ')"),
        "made [object AsyncGenerator] | start a | answered true"
    );
    normal(&mut agent);
    assert_eq!(
        text(&mut agent, &realm, "log.join(' | ')"),
        "made [object AsyncGenerator] | start a | answered true | {\"value\":1,\"done\":false} | \
         got 5 | {\"value\":6,\"done\":false} | {\"value\":\"end\",\"done\":true} | {\"done\":true}"
    );
}

/// AsyncGeneratorEnqueue and AsyncGeneratorYield, steps 9 and 10: requests queued while the
/// generator is executing are answered in the order of the calls, each `yield` settling the
/// request at the head and taking the next one without suspending; three `next` calls in one
/// turn and a fourth after the end.
#[test]
fn requests_queued_before_the_first_settles_are_answered_in_order() {
    let (mut agent, realm) = new_agent(limits());
    run(
        &mut agent,
        &realm,
        "var log = [], results = [];\n\
         async function* g() { log.push('run'); var a = yield 'one'; log.push('a ' + a);\n\
           var b = yield 'two'; log.push('b ' + b); return 'three'; }\n\
         var it = g();\n\
         var ps = [it.next('x'), it.next('y'), it.next('z'), it.next('w')];\n\
         ps.forEach(function (p, i) { p.then(function (r) { results.push(i + ' ' + r.value + ' ' + r.done); }); });\n\
         log.push('queued');",
    );
    assert_eq!(text(&mut agent, &realm, "log.join()"), "run,queued");
    normal(&mut agent);
    assert_eq!(text(&mut agent, &realm, "log.join()"), "run,queued,a y,b z");
    assert_eq!(
        text(&mut agent, &realm, "results.join()"),
        "0 one false,1 two false,2 three true,3 undefined true"
    );
}

/// `return` of a generator suspended at its start (`#sec-asyncgenerator-prototype-return`,
/// AsyncGeneratorAwaitReturn): the body never runs; the value is awaited, so the promise of the
/// call settles only once the awaited promise does, with `{ value, done: true }`, and a `next`
/// queued behind it answers `{ value: undefined, done: true }` after it; a rejected value
/// rejects the promise of the `return`.
#[test]
fn return_on_a_suspended_start_generator_awaits_its_value_and_completes() {
    let (mut agent, realm) = new_agent(limits());
    run(
        &mut agent,
        &realm,
        "var log = [], resolveV;\n\
         var v = new Promise(function (r) { resolveV = r; });\n\
         async function* g() { log.push('ran'); yield 1; }\n\
         var it = g();\n\
         var p = it['return'](v), q = it.next();\n\
         p.then(function (r) { log.push('return ' + r.value + ' ' + r.done); });\n\
         q.then(function (r) { log.push('next ' + r.value + ' ' + r.done); });\n\
         var rejected = g()['return'](Promise.reject('no'));\n\
         var plain = g()['return'](3);",
    );
    normal(&mut agent);
    assert_eq!(text(&mut agent, &realm, "log.join()"), "");
    assert_eq!(state(&mut agent, &realm, "p"), "pending");
    assert_eq!(state(&mut agent, &realm, "rejected"), "rejected \"no\"");
    assert_eq!(state(&mut agent, &realm, "plain"), "fulfilled 3 true");
    run(&mut agent, &realm, "resolveV('v');");
    normal(&mut agent);
    assert_eq!(
        text(&mut agent, &realm, "log.join()"),
        "return v true,next undefined true"
    );
    run(&mut agent, &realm, "var after = it.next();");
    normal(&mut agent);
    assert_eq!(
        state(&mut agent, &realm, "after"),
        "fulfilled undefined true"
    );
}

/// `throw` (`#sec-asyncgenerator-prototype-throw`): a generator suspended at its start becomes
/// completed and the promise is rejected with the value, without running the body; a completed
/// generator rejects at once and queues nothing; a `throw` queued behind the request that
/// completes the body is rejected by the drain. `next` after each answers
/// `{ value: undefined, done: true }`.
#[test]
fn throw_on_a_completed_generator_rejects() {
    let (mut agent, realm) = new_agent(limits());
    run(
        &mut agent,
        &realm,
        "var ran = 0;\n\
         async function* g() { ran++; yield 1; }\n\
         var started = g();\n\
         var atStart = started['throw'](new TypeError('start'));\n\
         var afterStart = started.next();\n\
         var it = g();\n\
         var first = it.next(), second = it.next();\n\
         var drained = it['throw'](new RangeError('queued'));\n",
    );
    normal(&mut agent);
    run(
        &mut agent,
        &realm,
        "var completed = it['throw'](new SyntaxError('done')), later = it.next();",
    );
    normal(&mut agent);
    assert_eq!(text(&mut agent, &realm, "ran"), "1");
    assert_eq!(state(&mut agent, &realm, "atStart"), "rejected TypeError");
    assert_eq!(
        state(&mut agent, &realm, "afterStart"),
        "fulfilled undefined true"
    );
    assert_eq!(state(&mut agent, &realm, "first"), "fulfilled 1 false");
    assert_eq!(
        state(&mut agent, &realm, "second"),
        "fulfilled undefined true"
    );
    assert_eq!(state(&mut agent, &realm, "drained"), "rejected RangeError");
    assert_eq!(
        state(&mut agent, &realm, "completed"),
        "rejected SyntaxError"
    );
    assert_eq!(
        state(&mut agent, &realm, "later"),
        "fulfilled undefined true"
    );
}

/// AsyncGeneratorYield awaits its operand (`#sec-yield`, `? Await(value)` for an async
/// generator): a rejected promise yielded throws at the `yield`, where a `catch` around it
/// catches it; a fulfilled one is yielded as its value, not as a promise.
#[test]
fn a_yield_awaits_its_operand() {
    let (mut agent, realm) = new_agent(limits());
    run(
        &mut agent,
        &realm,
        "var log = [];\n\
         async function* g() { try { yield Promise.reject('no'); log.push('after'); }\n\
           catch (e) { log.push('caught ' + e); yield 'c'; } }\n\
         g().next().then(function (r) { log.push('r ' + r.value + ' ' + r.done); });\n\
         async function* h() { yield Promise.resolve(3); }\n\
         h().next().then(function (r) { log.push(typeof r.value + ' ' + r.value); });",
    );
    normal(&mut agent);
    assert_eq!(
        text(&mut agent, &realm, "log.join()"),
        "caught no,number 3,r c false"
    );
}

/// `return` of a generator suspended at a `yield` (AsyncGeneratorUnwrapYieldResumption): the
/// value is awaited, then the `finally` around the `yield` runs, and its own `await` is awaited
/// before the promise of the `return` settles with `{ value, done: true }`.
#[test]
fn a_finally_runs_on_return_and_its_await_is_awaited() {
    let (mut agent, realm) = new_agent(limits());
    run(
        &mut agent,
        &realm,
        "var log = [], resolveF;\n\
         var f = new Promise(function (r) { resolveF = r; });\n\
         async function* g() { try { yield 1; yield 2; }\n\
           finally { log.push('finally'); await f; log.push('after await'); } }\n\
         var it = g();\n\
         it.next().then(function () {\n\
           it['return']('r').then(function (r) { log.push('return ' + r.value + ' ' + r.done); }); });",
    );
    normal(&mut agent);
    assert_eq!(text(&mut agent, &realm, "log.join()"), "finally");
    run(&mut agent, &realm, "resolveF();");
    normal(&mut agent);
    assert_eq!(
        text(&mut agent, &realm, "log.join()"),
        "finally,after await,return r true"
    );
}

/// A `for await` over an async generator (`#sec-asynciteratorclose`): a `break` calls its
/// `return`, which runs its `finally`, before the code after the loop; and a `for await` inside
/// an async generator awaits each value before it yields it.
#[test]
fn for_await_over_an_async_generator_closes_it_on_break() {
    let (mut agent, realm) = new_agent(limits());
    run(
        &mut agent,
        &realm,
        "var log = [], got = [];\n\
         async function* g() { try { for (var i = 0; ; i++) { log.push('yield ' + i); yield i; } }\n\
           finally { log.push('closed'); } }\n\
         (async function () { for await (var x of g()) { log.push('body ' + x); if (x === 2) break; }\n\
           log.push('after'); })();\n\
         async function* outer() { for await (var x of [Promise.resolve('a'), 'b']) yield x + '!'; }\n\
         (async function () { for await (var y of outer()) got.push(y); got.push('end'); })();",
    );
    normal(&mut agent);
    assert_eq!(
        text(&mut agent, &realm, "log.join()"),
        "yield 0,body 0,yield 1,body 1,yield 2,body 2,closed,after"
    );
    assert_eq!(text(&mut agent, &realm, "got.join()"), "a!,b!,end");
}

/// InstantiateAsyncGeneratorFunctionObject
/// (`#sec-runtime-semantics-instantiateasyncgeneratorfunctionobject`) and the prototypes
/// (`#sec-properties-of-asyncgeneratorfunction-prototype`,
/// `#sec-properties-of-asyncgenerator-prototype`): an async generator function has
/// `%AsyncGeneratorFunction.prototype%` as [[Prototype]], a `prototype` object whose
/// [[Prototype]] is `%AsyncGeneratorPrototype%`, and no [[Construct]]; the generator it makes
/// inherits from its `prototype`, or from `%AsyncGeneratorPrototype%` when that is no object;
/// FunctionDeclarationInstantiation runs in the call, so a default that throws throws there.
#[test]
fn an_async_generator_has_its_prototype_and_no_construct() {
    let (mut agent, realm) = new_agent(limits());
    run(
        &mut agent,
        &realm,
        "async function* g() {}\n\
         var AGFP = Object.getPrototypeOf(g), AGP = Object.getPrototypeOf(g.prototype);\n\
         var AIP = Object.getPrototypeOf(AGP);\n\
         function d(o, k) { var x = Object.getOwnPropertyDescriptor(o, k);\n\
           return '' + x.writable + x.enumerable + x.configurable; }\n\
         var checks = [\n\
           Object.getPrototypeOf(AGFP) === Function.prototype, typeof AGFP,\n\
           AGFP[Symbol.toStringTag], d(AGFP, Symbol.toStringTag),\n\
           AGFP.prototype === AGP, d(AGFP, 'prototype'),\n\
           AGP.constructor === AGFP, d(AGP, 'constructor'),\n\
           AGP[Symbol.toStringTag], d(AGP, Symbol.toStringTag),\n\
           typeof AIP[Symbol.asyncIterator], AIP === Object.getPrototypeOf(Object.getPrototypeOf((async function* () {}).prototype)) ? 'one' : 'two',\n\
           ['next', 'return', 'throw'].map(function (m) { return AGP[m].name + AGP[m].length + d(AGP, m); }).join(' '),\n\
           Object.getPrototypeOf(g()) === g.prototype, d(g, 'prototype'),\n\
           Object.getOwnPropertyNames(g.prototype).length,\n\
           g.hasOwnProperty('caller') || g.hasOwnProperty('arguments'),\n\
         ];\n\
         try { new g(); checks.push('constructed'); } catch (e) { checks.push(e instanceof TypeError); }\n\
         g.prototype = null; checks.push(Object.getPrototypeOf(g()) === AGP);\n\
         function boom() { throw 'parameter'; }\n\
         async function* h(a = boom()) { yield a; }\n\
         try { h(); checks.push('made'); } catch (e) { checks.push(e); }\n\
         var other = (async function* () {}).prototype;\n\
         checks.push(Object.getPrototypeOf(other) === AGP);",
    );
    assert_eq!(
        text(&mut agent, &realm, "checks.join()"),
        "true,object,AsyncGeneratorFunction,falsefalsetrue,true,falsefalsetrue,true,falsefalsetrue,\
         AsyncGenerator,falsefalsetrue,function,one,\
         next1truefalsetrue return1truefalsetrue throw1truefalsetrue,true,truefalsefalse,0,false,\
         true,true,parameter,true"
    );
}

/// "Async generators on suspended frames", item 1: an object that only a local slot and the
/// operand stack of a suspended async generator hold survives `collect()`, at a `yield` (the
/// generator held by the script) and at an `await` (held only by the functions of its `await`,
/// through the reaction of the awaited promise).
#[test]
fn a_suspended_async_generator_keeps_its_values_through_collect() {
    let (mut agent, realm) = new_agent(limits());
    run(
        &mut agent,
        &realm,
        "var out, kept, resolveP;\n\
         function pair(a, b) { return a.mark + b; }\n\
         var it = (async function* () { var local = { mark: 'local' };\n\
           var r = pair({ mark: 'stack' }, yield 1); yield local.mark + ',' + r; })();\n\
         it.next();\n\
         (async function* () { var local = { mark: 'kept' };\n\
           var r = pair({ mark: 'stack' }, await new Promise(function (r) { resolveP = r; }));\n\
           yield local.mark + ',' + r; })().next().then(function (r) { kept = r.value; });",
    );
    normal(&mut agent);
    agent.collect().unwrap();
    agent.collect().unwrap();
    run(
        &mut agent,
        &realm,
        "it.next('!').then(function (r) { out = r.value; }); resolveP('?');",
    );
    agent.collect().unwrap();
    normal(&mut agent);
    assert_eq!(text(&mut agent, &realm, "out"), "local,stack!");
    assert_eq!(text(&mut agent, &realm, "kept"), "kept,stack?");
}

/// "Async generators on suspended frames", item 1, and item 3 of "Suspended frames": with
/// [`LOCALS`] local slots, `heap_bytes()` after the first `next` (a checkpoint after it brings
/// the generator to its `yield`) exceeds the reading before the call by at least
/// [`FRAME_FLOOR`], and one `collect()` after the script drops the generator gives at least
/// [`FRAME_FLOOR`] back. Each reading is `heap_bytes()` after a `collect()`, in the second of
/// two rounds: the first grows the arenas, whose capacity stays in `heap_bytes()`, so the
/// second reads only what the generator holds. The test prints both rounds.
#[test]
fn a_frame_suspended_in_an_async_generator_is_charged_until_its_object_is_freed() {
    let (mut agent, realm) = new_agent(limits());
    run(
        &mut agent,
        &realm,
        &format!(
            "async function* gen() {{ {} yield 1; }}\nvar it; 0;",
            locals()
        ),
    );
    let make = parse(&mut agent, &realm, "it = gen(); it.next(); 0;");
    let drop_it = parse(&mut agent, &realm, "it = null; 0;");
    let mut readings = Vec::new();
    for _round in 0..2 {
        let before = collected(&mut agent);
        run_parsed(&mut agent, &make);
        normal(&mut agent);
        let held = collected(&mut agent);
        run_parsed(&mut agent, &drop_it);
        let freed = collected(&mut agent);
        readings.push((before, held, freed));
    }
    println!(
        "heap_bytes (before the call, after the first next, after collect) per round: {readings:?}"
    );
    let (before, held, freed) = readings[1];
    assert!(held.saturating_sub(before) >= FRAME_FLOOR, "{readings:?}");
    assert!(held.saturating_sub(freed) >= FRAME_FLOOR, "{readings:?}");
}

/// The requests of [`host_206_requests_are_charged_until_the_generator_is_freed_and_bounded_by_the_memory_budget`]
/// and [`settling_a_request_credits_nothing_until_the_generator_is_freed`]: 1,000, HOST-206's
/// count; 100 with the feature `collect-every-safe-point`, where every safe point collects the
/// whole heap, which grows with each request (at 1,000 the second test passed 60 s there).
const REQUESTS: u64 = if cfg!(feature = "collect-every-safe-point") {
    100
} else {
    1_000
};

/// The bytes HOST-206 states per request.
const PER_REQUEST: u64 = 64;

/// HOST-206 (`docs/design/host-interface.md`, section 10; "Async generators on suspended
/// frames", item 2): after the script that makes `g`, suspended at an `await` that never
/// settles, `heap_bytes()` after the script that calls `g.next()` [`REQUESTS`] times exceeds
/// the reading before it by at least [`REQUESTS`] times [`PER_REQUEST`]; after the script
/// `g = null` one `collect()` gives at least that back. The readings are those of the second of
/// two rounds (the first grows the arenas); the test prints both. With `steps_per_entry` 10^9
/// and a memory budget of [`QUEUE_ROOM`] over a realm the script `for (;;) h.next();` ends in
/// `Terminated(MemoryBudget)`.
#[test]
fn host_206_requests_are_charged_until_the_generator_is_freed_and_bounded_by_the_memory_budget() {
    let (mut agent, realm) = new_agent(limits());
    run(&mut agent, &realm, "var g; 0;");
    let make = parse(
        &mut agent,
        &realm,
        "g = (async function* () { await new Promise(function () {}); })(); g.next(); 0;",
    );
    let queue = parse(
        &mut agent,
        &realm,
        &format!("for (var i = 0; i < {REQUESTS}; i++) g.next(); 0;"),
    );
    let drop_it = parse(&mut agent, &realm, "g = null; 0;");
    let mut readings = Vec::new();
    for _round in 0..2 {
        run_parsed(&mut agent, &make);
        let before = agent.heap_bytes().unwrap();
        run_parsed(&mut agent, &queue);
        let held = agent.heap_bytes().unwrap();
        run_parsed(&mut agent, &drop_it);
        let freed = collected(&mut agent);
        readings.push((before, held, freed));
    }
    println!(
        "heap_bytes (before the loop, after it, after g = null and collect()) per round: \
         {readings:?}; {REQUESTS} x {PER_REQUEST}"
    );
    let (before, held, freed) = readings[1];
    assert!(
        held.saturating_sub(before) >= REQUESTS * PER_REQUEST,
        "{readings:?}"
    );
    assert!(
        held.saturating_sub(freed) >= REQUESTS * PER_REQUEST,
        "{readings:?}"
    );

    let (mut agent, realm) = memory_agent(QUEUE_ROOM, 1_000_000_000);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var h = (async function* () { await new Promise(function () {}); })(); h.next();\n\
             for (;;) h.next();"
        ),
        Shown::Terminated(Termination::MemoryBudget)
    );
}

/// "Async generators on suspended frames", item 2: settling a request credits nothing. A
/// generator with [`REQUESTS`] requests queued at an `await` completes and drains them all; with
/// the promises released and the generator still held, a `collect()` keeps the queue's capacity
/// charged, which the `collect()` after the script drops the generator gives back: at least
/// [`REQUESTS`] times [`REQUEST_BYTES`]. Both readings are `heap_bytes()` after a `collect()`,
/// whose arenas keep their capacity, so their difference is what the freed records owned.
#[test]
fn settling_a_request_credits_nothing_until_the_generator_is_freed() {
    let (mut agent, realm) = new_agent(limits());
    run(
        &mut agent,
        &realm,
        &format!(
            "var resolveP, settled = 0;\n\
             var it = (async function* () {{ await new Promise(function (r) {{ resolveP = r; }}); }})();\n\
             for (var i = 0; i < {REQUESTS}; i++) it.next().then(function () {{ settled++; }});\n\
             0;"
        ),
    );
    run(&mut agent, &realm, "resolveP(); resolveP = null; 0;");
    normal(&mut agent);
    assert_eq!(text(&mut agent, &realm, "settled"), REQUESTS.to_string());
    let drop_it = parse(&mut agent, &realm, "it = null; 0;");
    let held = collected(&mut agent);
    run_parsed(&mut agent, &drop_it);
    let freed = collected(&mut agent);
    println!(
        "heap_bytes: {held} with the requests settled, {freed} once the generator is freed \
         (-{}); {REQUESTS} x {REQUEST_BYTES}",
        held.saturating_sub(freed)
    );
    assert!(
        held.saturating_sub(freed) >= REQUESTS * REQUEST_BYTES,
        "{held} -> {freed}"
    );
}

/// The requests queued in [`the_drain_spends_one_step_for_each_request_it_settles`] behind the
/// one that ends the body: 99 and 0.
fn drain_steps(requests: u32) -> u64 {
    let (mut agent, realm) = new_agent(limits());
    run(
        &mut agent,
        &realm,
        &format!(
            "var resolveP;\n\
             var it = (async function* () {{ await new Promise(function (r) {{ resolveP = r; }}); }})();\n\
             for (var i = 0; i < {requests}; i++) it.next();\n\
             0;"
        ),
    );
    run(&mut agent, &realm, "resolveP(); 0;");
    normal(&mut agent);
    agent.last_entry_steps().unwrap()
}

/// "Async generators on suspended frames", item 3: the drain spends one step for each request
/// it settles. A generator that completes with r requests queued: `last_entry_steps()` of the
/// checkpoint that completes it with r = 100 exceeds that with r = 1 by at least 99. The test
/// prints both.
#[test]
fn the_drain_spends_one_step_for_each_request_it_settles() {
    let many = drain_steps(100);
    let one = drain_steps(1);
    println!("last_entry_steps: {many} with 100 requests, {one} with 1");
    assert!(many.saturating_sub(one) >= 99, "{many} against {one}");
}

/// The `then` callback a checkpoint runs before the job of a generator: it loops 10 turns.
const BURN: &str = "function burn() { for (var i = 0; i < 10; i++) {} }\n\
     Promise.resolve().then(burn);";

/// The steps of a checkpoint that runs only the job of [`BURN`], read in an agent of its own.
fn burn_steps() -> u64 {
    let (mut probe, probe_realm) = new_agent(limits());
    run(&mut probe, &probe_realm, BURN);
    assert_eq!(normal(&mut probe), report(1, 0));
    probe.last_entry_steps().unwrap()
}

/// The steps the resumed body of `g` in
/// [`a_resumed_await_in_an_async_generator_has_spent_three_steps`] spends after its three:
/// counted from its disassembly (`crates/compiler/tests/sources/async-generator-await.disasm`):
/// `JumpIfTag`, `Pop` (the `await` statement), `ResolveGlobal "probed"`, `True`,
/// `PutGlobalRef "probed"`, `Pop`, `Undefined` and `AsyncGeneratorReturn`.
const AFTER_THE_AWAIT: u64 = 8;

/// The steps of settling the request at the head with an iterator result: none of its own, and
/// one of the read of `then` of the result by its resolve function (`#sec-promise-resolve-functions`,
/// step 9), for the one hop of its prototype chain, to `%Object.prototype%`.
const SETTLE: u64 = 1;

/// Item 4 of "`async` functions on suspended frames", for an `await` in an async generator: a
/// resumed `await` has spent 3 steps when its next instruction runs (taking the job off, the
/// call of the function of the `await`, the resumption). The checkpoint's budget is the steps of
/// a job queued before the generator's ([`BURN`]) plus 3: it answers `Terminated(StepBudget)`
/// and `probed` stays false. With an ample budget, `last_entry_steps()` of the checkpoint that
/// resumes it alone reads 3 plus [`AFTER_THE_AWAIT`] plus [`SETTLE`].
#[test]
fn a_resumed_await_in_an_async_generator_has_spent_three_steps() {
    const G: &str = "var probed = false;\n\
                     async function* g() { await null; probed = true; }";
    let burn = burn_steps();
    let (mut agent, realm) = new_agent(limits_with(burn + 3));
    run(&mut agent, &realm, &format!("{G}\n{BURN}\ng().next();"));
    assert_eq!(terminated(&mut agent), Termination::StepBudget);
    assert_eq!(agent.last_entry_steps(), Ok(burn + 3));
    assert_eq!(text(&mut agent, &realm, "probed"), "false");

    let (mut agent, realm) = new_agent(limits());
    run(&mut agent, &realm, &format!("{G}\ng().next();"));
    assert_eq!(normal(&mut agent), report(1, 0));
    let steps = agent.last_entry_steps().unwrap();
    println!(
        "last_entry_steps: {steps} (3 + {AFTER_THE_AWAIT} + {SETTLE}); the job before it {burn}"
    );
    assert_eq!(text(&mut agent, &realm, "probed"), "true");
    assert_eq!(steps, 3 + AFTER_THE_AWAIT + SETTLE);
}

/// The iterations of [`a_chain_of_yields_longer_than_frame_depth_runs_to_its_end`]: 1,000; 40
/// with the feature `collect-every-safe-point`, where every safe point collects the whole heap
/// (at 100 the test ran 2.1 s there).
const LOOP: u32 = if cfg!(feature = "collect-every-safe-point") {
    40
} else {
    1_000
};

/// The `frame_depth` of the depth tests.
const FRAME_DEPTH: u32 = 16;

/// Item 4: a generator suspended at a `yield` takes no `frame_depth`, and each resumption by
/// `next` pops what it pushed: with `frame_depth` [`FRAME_DEPTH`], a `for await` over a
/// generator that yields [`LOOP`] times, and [`LOOP`] requests queued in one turn and answered
/// by one run of the body without suspending, both run to their end.
#[test]
fn a_chain_of_yields_longer_than_frame_depth_runs_to_its_end() {
    let mut limits = limits();
    limits.frame_depth = FRAME_DEPTH;
    let (mut agent, realm) = new_agent(limits);
    run(
        &mut agent,
        &realm,
        &format!(
            "var out = 'pending', last = 'pending';\n\
             async function* g() {{ for (var i = 0; i < {LOOP}; i++) yield i; }}\n\
             (async function () {{ var c = 0; for await (var x of g()) c++; return c; }})()\n\
               .then(function (v) {{ out = v; }}, function (e) {{ out = String(e); }});\n\
             var it = g(), p;\n\
             for (var i = 0; i <= {LOOP}; i++) p = it.next();\n\
             p.then(function (r) {{ last = r.done; }}, function (e) {{ last = String(e); }});"
        ),
    );
    normal(&mut agent);
    assert_eq!(text(&mut agent, &realm, "out"), LOOP.to_string());
    assert_eq!(text(&mut agent, &realm, "last"), "true");
    assert_eq!(agent.pending_microtasks(), Ok(0));
}

/// Item 4: with `frame_depth` 20 and 20 frames on the stack (the script's and 19 calls of
/// `at`), `next` of a suspended-start and of a suspended-yield generator answers a promise
/// rejected with the `RangeError` of "Depth bounds", queues nothing and leaves the generator
/// suspended: `next` from a shallower frame resumes it and gets its next value, which it would
/// not if the refused request had been queued. The deepest `at` only keeps the promise (a
/// call of a function there would pass `frame_depth` itself); a later script reads them.
#[test]
fn a_resumption_by_next_at_frame_depth_throws_a_range_error_and_leaves_the_generator_suspended() {
    let mut limits = limits();
    limits.frame_depth = 20;
    let (mut agent, realm) = new_agent(limits);
    run(
        &mut agent,
        &realm,
        "var results = [], ps = [];\n\
         var g = (async function* () { yield 1; yield 2; })();\n\
         function at(n) { if (n > 1) return at(n - 1); ps.push(g.next()); }",
    );
    for depth in [19, 18, 19, 18, 18] {
        run(&mut agent, &realm, &format!("at({depth});"));
        normal(&mut agent);
    }
    run(
        &mut agent,
        &realm,
        "ps.forEach(function (p, i) { p.then(function (r) { results[i] = r.value + ' ' + r.done; },\n\
           function (e) { results[i] = e instanceof RangeError ? 'range' : 'other ' + e; }); });",
    );
    normal(&mut agent);
    assert_eq!(
        text(&mut agent, &realm, "results.join()"),
        "range,1 false,range,2 false,undefined true"
    );
}

/// The body of [`an_async_generator_resuming_an_async_generator_reaches_reentry_depth`], on the
/// bounded stack. Through `reenter` (one re-entry), a chain of async generators each of which
/// resumes a new one with `next` in its body, one re-entry per level: with `reentry_depth` R the
/// levels 1 to R - 1 run, and the `next` of level R answers a promise rejected with a
/// `RangeError` and leaves that generator suspended at its start, with nothing queued: a later
/// `next` from the script runs it and answers its `yield`. Each level keeps the promise of its
/// `next` in `ps` once that call returned (an Array's `push` calls no function; `then` at the
/// deepest level would, for the executor of its capability, past the bound), so the deepest
/// level's comes first, and a later script reads it.
fn chain_through_reenter() {
    let reentry = BUILTINS_REENTRY;
    let (mut agent, realm) = stack_agent(reentry);
    let script = "var depth = 0, refused = 'none', last, deeper = true, got, ps = [];\n\
         async function* chain() { depth++; if (deeper) { var inner = chain(); last = inner;\n\
           ps.push(inner.next()); }\n\
           yield depth; }\n\
         reenter(function () { chain().next(); });\n\
         depth;";
    assert_eq!(
        eval_text(&mut agent, &realm, script),
        Shown::Normal((reentry - 1).to_string())
    );
    assert!(matches!(
        eval(
            &mut agent,
            &realm,
            "ps[0].then(null, function (e) {\n\
               refused = (e instanceof RangeError ? 'RangeError' : String(e)) + ' at ' + ps.length; });"
        ),
        Outcome::Normal(_)
    ));
    assert!(matches!(
        agent.perform_microtask_checkpoint(),
        Ok(Outcome::Normal(_))
    ));
    assert_eq!(
        eval_text(&mut agent, &realm, "refused;"),
        Shown::Normal(format!("\"RangeError at {}\"", reentry - 1))
    );
    assert!(matches!(
        eval(
            &mut agent,
            &realm,
            "deeper = false; last.next().then(function (r) { got = r.value + ' ' + r.done; });"
        ),
        Outcome::Normal(_)
    ));
    assert!(matches!(
        agent.perform_microtask_checkpoint(),
        Ok(Outcome::Normal(_))
    ));
    assert_eq!(
        eval_text(&mut agent, &realm, "got;"),
        Shown::Normal(format!("\"{reentry} false\""))
    );
}

/// Item 4 and item 6 of "Suspended frames", "Re-entry": a resumption by `next` is one re-entry,
/// refused past `reentry_depth`, and the deepest chain of async generators the bound lets run
/// does not overflow seven eighths of `min_stack_bytes()` ([`chain_through_reenter`]).
#[test]
fn an_async_generator_resuming_an_async_generator_reaches_reentry_depth() {
    let stack = stack_with_room(stack_limits(BUILTINS_REENTRY));
    on_bounded_stack(
        test_path!("an_async_generator_resuming_an_async_generator_reaches_reentry_depth"),
        stack,
        chain_through_reenter,
    )
    .unwrap();
}

/// The `steps_per_entry` of the step-budget round of
/// [`a_script_that_queues_requests_without_end_ends_at_a_budget`]: 10,000; 2,000 with the
/// feature `collect-every-safe-point`, where every safe point collects the whole heap, which
/// grows with each request.
const QUEUE_BUDGET: u64 = if cfg!(feature = "collect-every-safe-point") {
    2_000
} else {
    10_000
};

/// The room of the memory rounds of [`a_script_that_queues_requests_without_end_ends_at_a_budget`]
/// and of HOST-206 above the reading of a fresh realm: 256 KiB; 64 KiB with the feature
/// `collect-every-safe-point`.
const QUEUE_ROOM: u64 = if cfg!(feature = "collect-every-safe-point") {
    64 * 1024
} else {
    256 * 1024
};

/// "Async generators on suspended frames", item 2: `for (;;) g.next();` on a generator suspended
/// at an `await` that never settles ends at a budget. With `steps_per_entry` [`QUEUE_BUDGET`]
/// the script answers `Terminated(StepBudget)` and `last_entry_steps()` reads at most the
/// budget; once the script releases the generator, one `collect()` brings `heap_bytes()` back to
/// the reading before the script (the second of two rounds, so that the arenas grew in the
/// first). With a memory budget of [`QUEUE_ROOM`] over a realm and steps to spare the requests
/// reach the memory budget first: `Terminated(MemoryBudget)`. The test prints the readings.
#[test]
fn a_script_that_queues_requests_without_end_ends_at_a_budget() {
    let (mut agent, realm) = new_agent(limits_with(QUEUE_BUDGET));
    run(&mut agent, &realm, "var g; 0;");
    let endless = parse(
        &mut agent,
        &realm,
        "g = (async function* () { await new Promise(function () {}); })(); g.next();\n\
         for (;;) g.next();",
    );
    let release = parse(&mut agent, &realm, "g = undefined; 0;");
    let mut readings = Vec::new();
    for _round in 0..2 {
        let before = collected(&mut agent);
        assert!(matches!(
            agent.evaluate_script(&endless),
            Ok(Outcome::Terminated(Termination::StepBudget))
        ));
        let steps = agent.last_entry_steps().unwrap();
        assert!(steps <= QUEUE_BUDGET, "{steps}");
        let held = agent.heap_bytes().unwrap();
        run_parsed(&mut agent, &release);
        let after = collected(&mut agent);
        readings.push((before, held, after, steps));
    }
    println!("heap_bytes (before, at the termination, released) and steps per round: {readings:?}");
    let (before, held, after, _) = readings[1];
    assert!(held > before, "{readings:?}");
    assert!(after <= before, "{readings:?}");

    let (mut agent, realm) = memory_agent(QUEUE_ROOM, 1_000_000_000);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var h = (async function* () { await new Promise(function () {}); })(); h.next();\n\
             for (;;) h.next();"
        ),
        Shown::Terminated(Termination::MemoryBudget)
    );
}

/// HOST-204 (`docs/design/host-interface.md`, section 10; item 5): the script answers `Normal`
/// with an array of two promises; the next checkpoint answers `Terminated(StepBudget)`;
/// `promise_state` of both reads `Pending` after it and after a later checkpoint, which answers
/// `Normal` with both counts 0; `typeof f` then answers `"undefined"` (no `finally` ran); and
/// `g.next()` answers a promise that reads `Pending` after a further checkpoint.
#[test]
fn host_204_a_termination_in_a_running_async_generator_leaves_every_request_pending() {
    let (mut agent, realm) = new_agent(limits_with(1_000));
    let both = run(
        &mut agent,
        &realm,
        "var g = (async function* () { try { await null; for (;;) {} } finally { globalThis.f = 1; } })();\n\
         [g.next(), g.next()]",
    );
    let promises = agent
        .with_cx(&realm, |cx| {
            let Value::Object(array) = cx.get(&both).unwrap() else {
                panic!("no array");
            };
            let mut promises = Vec::new();
            for index in ["0", "1"] {
                let key = PropertyKey::String(cx.new_string(index).unwrap());
                let promise = cx.get_property(array, key).unwrap().unwrap();
                assert!(matches!(promise, Value::Object(_)));
                promises.push(cx.root(promise).unwrap());
            }
            promises
        })
        .unwrap();
    assert_eq!(terminated(&mut agent), Termination::StepBudget);
    for promise in &promises {
        assert_eq!(state_of_root(&mut agent, &realm, promise), "pending");
    }
    assert_eq!(normal(&mut agent), report(0, 0));
    for promise in &promises {
        assert_eq!(state_of_root(&mut agent, &realm, promise), "pending");
    }
    assert_eq!(
        eval_text(&mut agent, &realm, "typeof f"),
        Shown::Normal("\"undefined\"".to_string())
    );
    let later = run(&mut agent, &realm, "g.next()");
    normal(&mut agent);
    assert_eq!(state_of_root(&mut agent, &realm, &later), "pending");
}

/// HOST-205 (`docs/design/host-interface.md`, section 10; item 5): an async generator suspended
/// at an `await` when an entry ends in `Terminated(StepBudget)` keeps its frame and its queue:
/// `res(5)` and the next checkpoint fulfil `p` with `{ value: 5, done: false }` and `q` with
/// `{ value: 7, done: false }`, read through `promise_state` and `cx.get`.
#[test]
fn host_205_an_async_generator_suspended_when_an_entry_ends_keeps_its_frame_and_queue() {
    let (mut agent, realm) = new_agent(limits_with(1_000));
    run(
        &mut agent,
        &realm,
        "var res; var g = (async function* () { var x = yield await new Promise(function (r) { res = r; }); yield x; })();\n\
         var p = g.next(), q = g.next(7); 0",
    );
    normal(&mut agent);
    assert_eq!(
        eval_text(&mut agent, &realm, "for (;;) {}"),
        Shown::Terminated(Termination::StepBudget)
    );
    run(&mut agent, &realm, "res(5)");
    normal(&mut agent);
    assert_eq!(state(&mut agent, &realm, "p"), "fulfilled 5 false");
    assert_eq!(state(&mut agent, &realm, "q"), "fulfilled 7 false");
}

/// The script of [`drain_until`]: a generator suspended at an `await`, with `requests` requests
/// queued, whose body loops 100 turns once resumed, so that the checkpoint that ends it spends
/// more steps than the scripts before it.
fn drain_until(budget: u64, requests: u32) -> (Agent<TestHost>, Realm, u64) {
    let (mut agent, realm) = new_agent(limits_with(budget));
    run(
        &mut agent,
        &realm,
        &format!(
            "var resolveP, ps = [];\n\
             var it = (async function* () {{ await new Promise(function (r) {{ resolveP = r; }});\n\
               for (var i = 0; i < 100; i++) {{}} }})();\n\
             for (var i = 0; i < {requests}; i++) ps.push(it.next());\n\
             0;"
        ),
    );
    run(&mut agent, &realm, "resolveP(); 0;");
    let outcome = agent.perform_microtask_checkpoint().unwrap();
    let steps = agent.last_entry_steps().unwrap();
    match outcome {
        Outcome::Normal(_) | Outcome::Terminated(Termination::StepBudget) => {}
        other => panic!("{other:?}"),
    }
    (agent, realm, steps)
}

/// Item 5: a drain whose step for its second request is refused by the step budget. With one
/// request the checkpoint that ends the body spends `s1` steps, with two `s2`: more, by the
/// drain's step for the second request and the steps of its settling (the read of `then` of
/// its iterator result). With four requests and a budget of `s2`, the body's end settles the
/// head, the drain settles the next request and is refused its step for the one after: the
/// checkpoint answers `Terminated(StepBudget)` at `s2`, the first two promises are fulfilled and
/// the other two stay pending through a later checkpoint; a later `next` queues and stays
/// pending.
#[test]
fn a_termination_in_a_drain_leaves_the_rest_of_the_queue_pending() {
    let (_agent, _realm, one) = drain_until(1_000_000, 1);
    let (_agent, _realm, two) = drain_until(1_000_000, 2);
    let (mut agent, realm, steps) = drain_until(two, 4);
    println!(
        "last_entry_steps: {one} with one request, {two} with two, {steps} at the termination"
    );
    assert!(two > one, "{one} -> {two}");
    assert_eq!(steps, two);
    for (index, expected) in [
        "fulfilled undefined true",
        "fulfilled undefined true",
        "pending",
        "pending",
    ]
    .iter()
    .enumerate()
    {
        assert_eq!(
            state(&mut agent, &realm, &format!("ps[{index}]")),
            *expected,
            "{index}"
        );
    }
    assert_eq!(normal(&mut agent), report(0, 0));
    run(&mut agent, &realm, "ps.push(it.next()); 0;");
    normal(&mut agent);
    for index in 2..5 {
        assert_eq!(
            state(&mut agent, &realm, &format!("ps[{index}]")),
            "pending",
            "{index}"
        );
    }
}

/// Item 5 for a generator running in the job that resumed it when the checkpoint ends in
/// `termination` (`ending` is the code after its `await`, inside a `try` with a `finally`): the
/// `finally` does not run; its frame (with [`LOCALS`] local slots) is credited, so `live_bytes()`
/// after the termination and a `collect()` reads at least [`FRAME_FLOOR`] below the reading
/// while it was suspended (also after a `collect()`; the scripts that read and assign after it
/// were parsed before the first reading). `live_bytes()`, not `heap_bytes()`: the fill of the
/// memory budget grows the arenas, whose free slots `heap_bytes()` keeps after the collection
/// and `live_bytes()` leaves out. The generator stays executing: the promise of its request and
/// that of a later `next` stay pending through later checkpoints.
fn leaves_its_request_pending(
    agent: &mut Agent<TestHost>,
    realm: &Realm,
    ending: &str,
    termination: Termination,
) {
    run(
        agent,
        realm,
        &format!(
            "var ranFinally = false, resumed = 0, later;\n\
             var g = (async function* () {{ {} try {{ await null; resumed++; {ending} }}\n\
               finally {{ ranFinally = true; }} }})();\n\
             var p = g.next();",
            locals()
        ),
    );
    let call_next = parse(agent, realm, "later = g.next(); 0;");
    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
    let suspended = agent.live_bytes().unwrap();
    assert_eq!(terminated(agent), termination);
    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
    let after = agent.live_bytes().unwrap();
    println!("live_bytes after {termination:?}: {suspended} suspended, {after} after it");
    assert!(
        suspended.saturating_sub(after) >= FRAME_FLOOR,
        "{suspended} -> {after}"
    );
    assert_eq!(text(agent, realm, "resumed + ' ' + ranFinally"), "1 false");
    assert_eq!(state(agent, realm, "p"), "pending");
    run_parsed(agent, &call_next);
    assert_eq!(normal(agent), report(0, 0));
    assert_eq!(state(agent, realm, "p"), "pending");
    assert_eq!(state(agent, realm, "later"), "pending");
    assert_eq!(text(agent, realm, "resumed + ' ' + ranFinally"), "1 false");
}

/// Item 5: `Terminated(StepBudget)` in a running async generator ([`leaves_its_request_pending`]).
#[test]
fn a_step_budget_termination_in_a_running_async_generator_leaves_its_request_pending() {
    let (mut agent, realm) = new_agent(limits_with(1_000));
    leaves_its_request_pending(&mut agent, &realm, "for (;;) {}", Termination::StepBudget);
}

/// Item 5: `Terminated(MemoryBudget)` in a running async generator
/// ([`leaves_its_request_pending`]).
#[test]
fn a_memory_budget_termination_in_a_running_async_generator_leaves_its_request_pending() {
    let (mut agent, realm) = memory_agent(FILL_ROOM, limits().steps_per_entry);
    leaves_its_request_pending(&mut agent, &realm, FILL, Termination::MemoryBudget);
}

/// Item 5: `Terminated(Interrupted)` in a running async generator
/// ([`leaves_its_request_pending`]).
#[test]
fn an_interrupt_in_a_running_async_generator_leaves_its_request_pending() {
    let (mut agent, realm) = new_agent(limits());
    leaves_its_request_pending(
        &mut agent,
        &realm,
        "interruptNow(); for (;;) {}",
        Termination::Interrupted,
    );
}

/// Item 5 for a generator suspended at an `await` when another job, queued before its own, ends
/// the checkpoint in `termination` (`ends` is that job's function): the generator keeps its
/// frame and its charge (a `collect()` after the termination reads at least [`FRAME_FLOOR`]
/// above the reading after the next checkpoint completes it and a `collect()`), its job stays
/// queued, and the next checkpoint resumes it and settles its requests in order: two jobs, that
/// of its `await` and that of the `await` of the value of its `return`. Each reading is
/// `heap_bytes()` after a `collect()`, in the second of two rounds: the first grows the arenas,
/// whose capacity stays in `heap_bytes()`, so the second reads only what the suspended frame
/// holds (the termination ends the checkpoint, not the agent, so a round can follow it). The
/// script of a round is parsed before the first reading; the test prints both rounds.
fn resumes_at_a_later_checkpoint(
    agent: &mut Agent<TestHost>,
    realm: &Realm,
    ends: &str,
    termination: Termination,
) {
    run(
        agent,
        realm,
        &format!(
            "function ends() {{ {ends} }}\n\
             async function* f() {{ {} await null; return 7; }}\n\
             var it, p, q; 0;",
            locals()
        ),
    );
    let round = parse(
        agent,
        realm,
        "Promise.resolve().then(ends);\n\
         it = f(); p = it.next(); q = it.next(); 0;",
    );
    let mut readings = Vec::new();
    for _round in 0..2 {
        run_parsed(agent, &round);
        assert_eq!(terminated(agent), termination);
        assert_eq!(agent.pending_microtasks(), Ok(1));
        let suspended = collected(agent);
        assert_eq!(normal(agent), report(2, 0));
        let completed = collected(agent);
        readings.push((suspended, completed));
        assert_eq!(state(agent, realm, "p"), "fulfilled 7 true");
        assert_eq!(state(agent, realm, "q"), "fulfilled undefined true");
    }
    println!("heap_bytes after {termination:?} (suspended, completed) per round: {readings:?}");
    let (suspended, completed) = readings[1];
    assert!(
        suspended.saturating_sub(completed) >= FRAME_FLOOR,
        "{readings:?}"
    );
}

/// Item 5: [`resumes_at_a_later_checkpoint`] at `Terminated(StepBudget)`,
/// `Terminated(MemoryBudget)` (the collection after the termination frees what the fill held,
/// not the suspended generator) and `Terminated(Interrupted)`, each reading of the second round.
#[test]
fn an_async_generator_suspended_at_a_termination_resumes_at_a_later_checkpoint() {
    let (mut agent, realm) = new_agent(limits_with(1_000));
    resumes_at_a_later_checkpoint(&mut agent, &realm, "for (;;) {}", Termination::StepBudget);

    let (mut agent, realm) = memory_agent(FILL_ROOM, limits().steps_per_entry);
    resumes_at_a_later_checkpoint(&mut agent, &realm, FILL, Termination::MemoryBudget);

    let (mut agent, realm) = new_agent(limits());
    resumes_at_a_later_checkpoint(
        &mut agent,
        &realm,
        "interruptNow(); for (;;) {}",
        Termination::Interrupted,
    );
}

/// The functions the tests of `yield*` share (`story:yield-star-in-async-generators`):
/// `show(r)` writes an iterator result as `<value> <done>`, `answer(p, name)` records the
/// settling of the promise `p` in `answers[name]` (`<value> <done>`, or the name of the error a
/// rejection gives, or `rejected <reason>`), and `asyncIterable(next, more)` is an object of the
/// script whose `[Symbol.asyncIterator]` answers itself, with `next` and the properties of `more`.
const DELEGATION: &str = "var log = [], answers = {};\n\
     function show(r) { return r.value + ' ' + r.done; }\n\
     function answer(p, name) {\n\
       p.then(function (r) { answers[name] = show(r); },\n\
         function (e) { answers[name] = e instanceof Error ? e.name : 'rejected ' + e; });\n\
     }\n\
     function asyncIterable(next, more) {\n\
       var o = { next: next };\n\
       o[Symbol.asyncIterator] = function () { return o; };\n\
       for (var k in more) o[k] = more[k];\n\
       return o;\n\
     }\n";

/// The `answers` of [`DELEGATION`] in the order of `names`, joined with `,`.
fn answers(agent: &mut Agent<TestHost>, realm: &Realm, names: &[&str]) -> String {
    let list: Vec<String> = names
        .iter()
        .map(|name| format!("answers['{name}']"))
        .collect();
    text(agent, realm, &format!("[{}].join()", list.join(", ")))
}

/// YieldExpression : `yield *` AssignmentExpression with generatorKind async, step 8.a: the
/// inner iterator's `next` is called with the received value and its result awaited; a value
/// not done is yielded by AsyncGeneratorYield without a second `await` (a promise as the value
/// of an inner result is the value of the outer one), and the value it is done with is the value
/// of the `yield*`. An inner async generator runs in the call of the outer `next`, before the
/// code after that call ("Async generators on suspended frames", item 6); the [[NextMethod]] of
/// an iterator of the script is read once and called with the iterator as the this value, also
/// for requests queued while the delegating generator runs.
#[test]
fn yield_star_in_an_async_generator_delegates_to_an_async_iterator() {
    let (mut agent, realm) = new_agent(limits());
    run(
        &mut agent,
        &realm,
        &format!(
            "{DELEGATION}\
             async function* inner(a) {{ log.push('inner ' + a); var x = yield 'i1';\n\
               log.push('inner got ' + x); var y = yield 'i2'; log.push('inner got ' + y); return 'ret'; }}\n\
             async function* outer() {{ log.push('outer'); var r = yield* inner('a'); log.push('r ' + r);\n\
               yield 'after'; }}\n\
             var it = outer();\n\
             it.next('ignored').then(function (r) {{ answers.a = show(r); return it.next('x'); }})\n\
               .then(function (r) {{ answers.b = show(r); return it.next('y'); }})\n\
               .then(function (r) {{ answers.c = show(r); return it.next(); }})\n\
               .then(function (r) {{ answers.d = show(r); }});\n\
             log.push('called');\n\
             var reads = 0, n = 0, calls = [], held = Promise.resolve(5), plain;\n\
             plain = asyncIterable(null, {{}});\n\
             Object.defineProperty(plain, 'next', {{ get: function () {{ reads++; return function (v) {{\n\
               calls.push((this === plain) + ' ' + v); n++;\n\
               return n === 1 ? Promise.resolve({{ value: held, done: false }}) : {{ value: n, done: n > 2 }}; }}; }} }});\n\
             async function* g() {{ var r = yield* plain; calls.push('r ' + r); }}\n\
             var it2 = g(), first = it2.next('a');\n\
             answer(it2.next('b'), 'f'); answer(it2.next('c'), 'g');\n\
             first.then(function (r) {{ answers.e = r.value === held; }});"
        ),
    );
    assert_eq!(
        text(&mut agent, &realm, "log.join()"),
        "outer,inner a,called"
    );
    normal(&mut agent);
    assert_eq!(
        text(&mut agent, &realm, "log.join()"),
        "outer,inner a,called,inner got x,inner got y,r ret"
    );
    assert_eq!(
        answers(&mut agent, &realm, &["a", "b", "c", "d"]),
        "i1 false,i2 false,after false,undefined true"
    );
    assert_eq!(
        text(&mut agent, &realm, "calls.join() + ' ' + reads"),
        "true undefined,true b,true c,r 3 1"
    );
    assert_eq!(
        answers(&mut agent, &realm, &["e", "f", "g"]),
        "true,2 false,undefined true"
    );
}

/// GetIterator(value, async) of a value with no `[Symbol.asyncIterator]`: the Async-from-Sync
/// Iterator of its sync iterator (`#sec-createasyncfromsynciterator`), whose `next` passes the
/// received value on and awaits the value of each result; the value a sync generator returns is
/// the value of the `yield*`, and an Array delegates its elements.
#[test]
fn yield_star_in_an_async_generator_delegates_to_a_sync_iterator_through_async_from_sync() {
    let (mut agent, realm) = new_agent(limits());
    run(
        &mut agent,
        &realm,
        &format!(
            "{DELEGATION}\
             function* s() {{ log.push('s'); var x = yield 1; log.push('s got ' + x);\n\
               yield Promise.resolve(2); return 'end'; }}\n\
             async function* g() {{ var r = yield* s(); log.push('r ' + r); var a = yield* [3, 4];\n\
               log.push('a ' + a); }}\n\
             var it = g();\n\
             it.next().then(function (r) {{ answers.a = show(r); return it.next('x'); }})\n\
               .then(function (r) {{ answers.b = show(r); return it.next(); }})\n\
               .then(function (r) {{ answers.c = show(r); return it.next(); }})\n\
               .then(function (r) {{ answers.d = show(r); return it.next(); }})\n\
               .then(function (r) {{ answers.e = show(r); }});"
        ),
    );
    normal(&mut agent);
    assert_eq!(
        answers(&mut agent, &realm, &["a", "b", "c", "d", "e"]),
        "1 false,2 false,3 false,4 false,undefined true"
    );
    assert_eq!(
        text(&mut agent, &realm, "log.join()"),
        "s,s got x,r end,a undefined"
    );
}

/// Steps 8.b and 8.c: a throw given to the delegating generator is passed to the inner
/// iterator's `throw` (an async generator's `catch` catches it and yields), a return to its
/// `return`; each result is awaited, and a return's value is awaited before it is passed on
/// (AsyncGeneratorUnwrapYieldResumption). A `return` whose result is done completes the
/// delegating generator with its value, not awaited (8.c.viii), through its `finally`; with no
/// `return` method the received value is awaited and returned (8.c.iii).
#[test]
fn yield_star_passes_throw_and_return_on_and_awaits_them() {
    let (mut agent, realm) = new_agent(limits());
    run(
        &mut agent,
        &realm,
        &format!(
            "{DELEGATION}\
             async function* inner() {{ try {{ yield 1; }} catch (e) {{ log.push('inner caught ' + e); yield 2; }} }}\n\
             async function* outer(i) {{ try {{ yield* i; }} finally {{ log.push('outer finally'); }} }}\n\
             var a = outer(inner());\n\
             a.next().then(function () {{ answer(a['throw']('x'), 'throw'); }});\n\
             var got = [], held = Promise.resolve('p');\n\
             var withBoth = asyncIterable(function () {{ return {{ value: 1, done: false }}; }}, {{\n\
               'throw': function (e) {{ got.push('throw ' + e); return Promise.resolve({{ value: 't', done: false }}); }},\n\
               'return': function (v) {{ got.push('return ' + (v instanceof Promise ? 'a promise' : v));\n\
                 return {{ value: held, done: true }}; }} }});\n\
             var b = outer(withBoth);\n\
             b.next().then(function () {{ answer(b['throw']('e'), 'thrown');\n\
               b['return'](Promise.resolve('v')).then(function (r) {{ answers.returned = r.value === held; }}); }});\n\
             var c = outer(asyncIterable(function () {{ return {{ value: 1, done: false }}; }}, {{}}));\n\
             c.next().then(function () {{ answer(c['return'](Promise.resolve('w')), 'bare'); }});"
        ),
    );
    normal(&mut agent);
    assert_eq!(
        answers(&mut agent, &realm, &["throw", "thrown", "returned", "bare"]),
        "2 false,t false,true,w true"
    );
    assert_eq!(text(&mut agent, &realm, "got.join()"), "throw e,return v");
    assert_eq!(
        text(&mut agent, &realm, "log.join()"),
        "inner caught x,outer finally,outer finally"
    );
}

/// Step 8.b.iii: a throw given while the inner iterator has no `throw` closes it with
/// AsyncIteratorClose and a normal completion (`#sec-asynciteratorclose`): its `return` is
/// called with no argument and its result awaited, and then a `TypeError` rejects the request;
/// a rejected result of `return` rejects it with that reason instead, and a result that is no
/// Object with a `TypeError`; with no `return` either nothing is called. The delegating
/// generator is completed after each.
#[test]
fn yield_star_closes_an_inner_iterator_without_throw() {
    let (mut agent, realm) = new_agent(limits());
    run(
        &mut agent,
        &realm,
        &format!(
            "{DELEGATION}\
             var closed = [];\n\
             function step() {{ return {{ value: 1, done: false }}; }}\n\
             function closing(name, result) {{\n\
               return asyncIterable(step, {{ 'return': function () {{\n\
                 closed.push(name + ' ' + arguments.length); return result(); }} }});\n\
             }}\n\
             async function* g(i) {{ yield* i; }}\n\
             function throwInto(i, name) {{\n\
               var it = g(i);\n\
               it.next().then(function () {{ answer(it['throw']('x'), name);\n\
                 answer(it.next(), name + ' after'); }});\n\
             }}\n\
             throwInto(closing('a', function () {{ return Promise.resolve({{}}); }}), 'a');\n\
             throwInto(closing('b', function () {{ return Promise.reject('no'); }}), 'b');\n\
             throwInto(closing('c', function () {{ return 1; }}), 'c');\n\
             throwInto(asyncIterable(step, {{}}), 'd');"
        ),
    );
    normal(&mut agent);
    assert_eq!(
        answers(
            &mut agent,
            &realm,
            &[
                "a", "a after", "b", "b after", "c", "c after", "d", "d after"
            ]
        ),
        "TypeError,undefined true,rejected no,undefined true,TypeError,undefined true,\
         TypeError,undefined true"
    );
    assert_eq!(text(&mut agent, &realm, "closed.join()"), "a 0,b 0,c 0");
}

/// Item 6 of "Async generators on suspended frames": a chain of 30 delegating async generators
/// over a leaf, with `reentry_depth` 20 (so `frame_depth` is 10,000, above the 32 frames of the
/// script, the levels and the leaf), answers the three `next` calls of the outermost with the
/// leaf's values and the value it returns: no level is a re-entry.
#[test]
fn a_chain_of_delegating_async_generators_deeper_than_reentry_depth_takes_no_re_entry_per_level() {
    let mut limits = limits();
    limits.reentry_depth = 20;
    let (mut agent, realm) = new_agent(limits);
    run(
        &mut agent,
        &realm,
        &format!(
            "{DELEGATION}\
             async function* leaf() {{ yield 1; yield 2; return 3; }}\n\
             async function* level(inner) {{ return yield* inner; }}\n\
             var it = leaf();\n\
             for (var i = 0; i < 30; i++) it = level(it);\n\
             answer(it.next(), 'a'); answer(it.next(), 'b'); answer(it.next(), 'c');"
        ),
    );
    normal(&mut agent);
    assert_eq!(
        answers(&mut agent, &realm, &["a", "b", "c"]),
        "1 false,2 false,3 true"
    );
}

/// Item 6 with item 4: `frame_depth` counts the script's frame and the levels a `next` pushes
/// in the loop, and a push is refused at a count of `frame_depth` or more. With `frame_depth` 20
/// and a chain of 30 levels over a leaf, each catching at its `yield*`, recording its index and
/// throwing again: the push of level 19 is refused at the `yield*` of level 18 (20 frames: the
/// script's and those of levels 0 to 18), its request is rejected with the `RangeError` and it
/// stays suspended at its start; level 18's `await` throws it, the throw leaves through levels
/// 18 to 0, and after the checkpoint the outermost promise is rejected with it. A later `next`
/// of level 19 runs the rest of the chain and is fulfilled with the leaf's value. 18 levels over
/// a leaf (20 frames) are fulfilled, 19 are rejected.
#[test]
fn a_chain_of_delegating_async_generators_reaches_frame_depth_at_the_yield_star_that_would_push() {
    let mut limits = limits();
    limits.frame_depth = 20;
    let (mut agent, realm) = new_agent(limits);
    run(
        &mut agent,
        &realm,
        &format!(
            "{DELEGATION}\
             var gens = [], thrownAt = [];\n\
             async function* level(i) {{ try {{ yield* gens[i + 1]; }} catch (e) {{ thrownAt.push(i); throw e; }} }}\n\
             async function* leaf() {{ yield 'leaf'; }}\n\
             for (var i = 0; i < 30; i++) gens.push(level(i));\n\
             gens.push(leaf());\n\
             async function* plain(gs, i) {{ yield* gs[i + 1]; }}\n\
             function chain(n) {{ var gs = []; for (var i = 0; i < n; i++) gs.push(plain(gs, i));\n\
               gs.push(leaf()); return gs[0]; }}\n\
             var c18 = chain(18), c19 = chain(19);\n\
             answer(gens[0].next(), 'outermost'); answer(c18.next(), '18'); answer(c19.next(), '19');"
        ),
    );
    normal(&mut agent);
    assert_eq!(
        answers(&mut agent, &realm, &["outermost", "18", "19"]),
        "RangeError,leaf false,RangeError"
    );
    let indices: Vec<String> = (0..=18).rev().map(|i| i.to_string()).collect();
    assert_eq!(
        text(&mut agent, &realm, "thrownAt.join()"),
        indices.join(",")
    );
    run(&mut agent, &realm, "answer(gens[19].next(), 'rest');");
    normal(&mut agent);
    assert_eq!(answers(&mut agent, &realm, &["rest"]), "leaf false");
}

/// Item 6: a `next` through n levels of delegating async generators spends, per level, at least
/// 4 and at most 20 steps more than through one, so through 101 levels between 400 and 2,000
/// steps more than through 1 level, read with `last_entry_steps()` summed over the script of one
/// `next` and the checkpoint after it. Both chains have run to their first `yield` before.
/// `frame_depth` 10,000 is above the 103 frames of the script, 101 levels and the leaf. The test
/// prints both sums.
#[test]
fn a_next_through_101_levels_spends_400_to_2000_steps_more_than_through_one() {
    let (mut agent, realm) = new_agent(limits());
    run(
        &mut agent,
        &realm,
        "async function* leaf() { while (true) yield 1; }\n\
         async function* level(inner) { yield* inner; }\n\
         function chain(n) { var it = leaf(); for (var i = 0; i < n; i++) it = level(it); return it; }\n\
         var one = chain(1), many = chain(101); one.next(); many.next();",
    );
    normal(&mut agent);
    let mut steps_of = |source: &str| {
        let script = parse(&mut agent, &realm, source);
        run_parsed(&mut agent, &script);
        let called = agent.last_entry_steps().unwrap();
        normal(&mut agent);
        called + agent.last_entry_steps().unwrap()
    };
    let through_one = steps_of("one.next(); 0;");
    let through_many = steps_of("many.next(); 0;");
    let more = through_many.saturating_sub(through_one);
    println!(
        "last_entry_steps: {through_one} for a next through 1 level, {through_many} through 101 \
         levels, {more} more"
    );
    assert!(
        (400..=2_000).contains(&more),
        "{through_many} against {through_one}"
    );
}

/// The `reentry_depth` of [`yield_star_over_a_script_iterator_at_reentry_depth_throws_a_range_error_at_the_yield_star`].
const SCRIPT_ITERATOR_REENTRY: u32 = 10;

/// The script of [`deep_delegation`]: `target(name)` is an async iterator of the script
/// (`%AsyncIteratorPrototype%`'s `[Symbol.asyncIterator]`, a builtin, answers it) whose `next`,
/// `throw` and `return` are script functions that record their call in `called`; `g` delegates
/// to it; `at(n, f)` calls `f` under `n` re-entries through `reenter`.
const DEEP_DELEGATION: &str = "var called = [], gen, promise;\n\
     var AsyncIteratorPrototype = Object.getPrototypeOf(Object.getPrototypeOf(\n\
       (async function* () {}).prototype));\n\
     function target(name) {\n\
       var o = Object.create(AsyncIteratorPrototype);\n\
       o.next = function () { called.push(name + ' next'); return { value: name, done: false }; };\n\
       o['throw'] = function () { called.push(name + ' throw'); return { value: name, done: false }; };\n\
       o['return'] = function () { called.push(name + ' return'); return { value: name, done: true }; };\n\
       return o;\n\
     }\n\
     async function* g(o) { yield* o; }\n\
     function at(n, f) { if (n > 0) return reenter(function () { return at(n - 1, f); }); return f(); }\n\
     0;";

/// `method` (`next` of a generator not started, `throw` or `return` of one first run to its
/// `yield*` by a `next` at the top) called under `depth` re-entries; for `return`, whose value
/// the delegating generator awaits before it calls the inner `return`
/// (AsyncGeneratorUnwrapYieldResumption), the checkpoint that resumes it runs there too
/// (`checkpointHere`). Answers the state of the promise of the call after the checkpoint at the
/// top, and the calls of the script's functions.
fn deep_delegation(agent: &mut Agent<TestHost>, realm: &Realm, method: &str, depth: u32) -> String {
    run(
        agent,
        realm,
        &format!(
            "gen = g(target('{method}')); {}",
            if method == "next" {
                "0;"
            } else {
                "gen.next();"
            }
        ),
    );
    normal(agent);
    let deep = if method == "return" {
        " checkpointHere();"
    } else {
        ""
    };
    run(
        agent,
        realm,
        &format!(
            "called = []; at({depth}, function () {{ promise = gen['{method}']('v');{deep} }}); 0;"
        ),
    );
    normal(agent);
    let state = state(agent, realm, "promise");
    format!("{state} [{}]", text(agent, realm, "called.join()"))
}

/// Item 6 and "Depth bounds": a delegation to an iterator of the script calls its `next`,
/// `throw` or `return` as one re-entry, the resumption of the delegating generator being one
/// before it. With `reentry_depth` [`SCRIPT_ITERATOR_REENTRY`], under the deepest stack where
/// the resumption still fits and the call does not, the `RangeError` is thrown at the `yield*`
/// and rejects the request: `next` of a generator not started, `throw` of one at its `yield*`,
/// and `return` of one at its `yield*` (in the checkpoint at that depth that resumes it after
/// the `await` of its value) each answer a promise rejected with a `RangeError`, and the
/// script's function was not called; one level shallower each is fulfilled. Measured: under
/// `reentry_depth` - 1 re-entries of `reenter` for `next` and `throw`, and under
/// `reentry_depth` - 2 for `return`, whose resumption is made by the function of its `await`,
/// called by the job inside the host function `checkpointHere`, one re-entry more.
#[test]
fn yield_star_over_a_script_iterator_at_reentry_depth_throws_a_range_error_at_the_yield_star() {
    let (mut agent, realm) = stack_agent(SCRIPT_ITERATOR_REENTRY);
    define_function(
        &mut agent,
        &realm,
        "checkpointHere",
        0,
        checkpoint_here,
        false,
    );
    run(&mut agent, &realm, DEEP_DELEGATION);
    for (method, deepest, done) in [
        ("next", SCRIPT_ITERATOR_REENTRY - 2, false),
        ("throw", SCRIPT_ITERATOR_REENTRY - 2, false),
        ("return", SCRIPT_ITERATOR_REENTRY - 3, true),
    ] {
        assert_eq!(
            deep_delegation(&mut agent, &realm, method, deepest + 1),
            "rejected RangeError []",
            "{method}"
        );
        assert_eq!(
            deep_delegation(&mut agent, &realm, method, deepest),
            format!("fulfilled \"{method}\" {done} [{method} {method}]"),
            "{method}"
        );
    }
}

/// Item 5 of "Async generators on suspended frames" for each level of a chain of delegating
/// async generators: `a` delegates to `b`, `b` to the leaf `l`, whose body runs `body` in the
/// call of `a.next()`, so the three frames are on the stack when the entry ends in
/// `termination`. No `finally` runs; every level stays executing, so a later `next`, `return` or
/// `throw` of any of them only queues, and its promise stays pending through later checkpoints.
fn a_delegating_level_leaves_its_requests_pending(
    agent: &mut Agent<TestHost>,
    realm: &Realm,
    body: &str,
    termination: Termination,
) {
    run(
        agent,
        realm,
        &format!(
            "var ranFinally = false, ps;\n\
             async function* leaf() {{ try {{ {body} }} finally {{ ranFinally = true; }} }}\n\
             async function* level(inner) {{ try {{ yield* inner; }} finally {{ ranFinally = true; }} }}\n\
             var l = leaf(), b = level(l), a = level(b);"
        ),
    );
    assert_eq!(
        eval_text(agent, realm, "a.next();"),
        Shown::Terminated(termination)
    );
    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
    run(
        agent,
        realm,
        "ps = [a.next(), b.next(), l.next(), a['return'](1), b['throw'](2), l['return'](3)];",
    );
    normal(agent);
    normal(agent);
    for index in 0..6 {
        assert_eq!(
            state(agent, realm, &format!("ps[{index}]")),
            "pending",
            "{index}"
        );
    }
    assert_eq!(text(agent, realm, "ranFinally"), "false");
}

/// [`a_delegating_level_leaves_its_requests_pending`] at `Terminated(StepBudget)`,
/// `Terminated(MemoryBudget)` and `Terminated(Interrupted)`.
#[test]
fn a_termination_inside_a_delegating_async_generator_leaves_its_request_pending() {
    let (mut agent, realm) = new_agent(limits_with(1_000));
    a_delegating_level_leaves_its_requests_pending(
        &mut agent,
        &realm,
        "for (;;) {}",
        Termination::StepBudget,
    );

    let (mut agent, realm) = memory_agent(FILL_ROOM, limits().steps_per_entry);
    a_delegating_level_leaves_its_requests_pending(
        &mut agent,
        &realm,
        FILL,
        Termination::MemoryBudget,
    );

    let (mut agent, realm) = new_agent(limits());
    a_delegating_level_leaves_its_requests_pending(
        &mut agent,
        &realm,
        "interruptNow(); for (;;) {}",
        Termination::Interrupted,
    );
}
