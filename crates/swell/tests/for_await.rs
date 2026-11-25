//! `for await` in an `async` function, the async iterator protocol and the thirteenth well-known
//! symbol (`story:for-await-and-async-from-sync-iterator`): ForIn/OfBodyEvaluation with
//! iteratorKind async
//! (`#sec-runtime-semantics-forin-div-ofbodyevaluation-lhs-stmt-iterator-lhskind-labelset`),
//! GetIterator with kind async (`#sec-getiterator`), AsyncIteratorClose
//! (`#sec-asynciteratorclose`), CreateAsyncFromSyncIterator and
//! `%AsyncFromSyncIteratorPrototype%` (`#sec-%asyncfromsynciteratorprototype%-object`),
//! `%Symbol.asyncIterator%` and HOST-203 (`docs/design/host-interface.md`, decision 7); the
//! bounds of `docs/design/engine.md`, decision 3, "`async` functions on suspended frames" and
//! "The Async-from-Sync Iterator". Run also with the feature `collect-every-safe-point`.
//!
//! A test that reads `heap_bytes()` around an entry parses that entry's script before the first
//! reading and declares the global names it assigns in an earlier script, as
//! `tests/async_functions.rs` does.
#![cfg(test)]

use crate::support;

use std::cell::RefCell;

use support::host::{
    TestHost, agent_with, data, global, key, limits, object, realm, stack_with_room,
};
use support::script::{Outcome as Shown, define_function, eval, eval_text, parse};
use support::stack::{BUILTINS_REENTRY, on_bounded_stack, stack_agent, stack_limits, test_path};
use swell::{
    Agent, CheckpointReport, Completion, Cx, GlobalObjects, HookCx, HostCall, InterruptHandle,
    Limits, ObjectRef, Outcome, PromiseState, PropertyKey, Prototype, Realm, RealmRef, Root,
    SymbolRef, Termination, Value, WellKnownSymbol,
};

/// A checkpoint that must answer `Normal`: its report.
fn normal(agent: &mut Agent<TestHost>) -> CheckpointReport {
    match agent.perform_microtask_checkpoint().unwrap() {
        Outcome::Normal(report) => report,
        other => panic!("the checkpoint answered {other:?}"),
    }
}

/// The checkpoint's outcome, which must not be a throw.
fn checkpoint(agent: &mut Agent<TestHost>) -> Outcome<CheckpointReport> {
    agent.perform_microtask_checkpoint().unwrap()
}

/// A checkpoint that must answer `Terminated`: its reason.
fn terminated(agent: &mut Agent<TestHost>) -> Termination {
    match checkpoint(agent) {
        Outcome::Terminated(termination) => termination,
        other => panic!("the checkpoint answered {other:?}"),
    }
}

/// The script `source`, which must complete normally.
fn run(agent: &mut Agent<TestHost>, realm: &Realm, source: &str) {
    match eval(agent, realm, source) {
        Outcome::Normal(_) => {}
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

/// The value of the global `name` of `realm`, rooted.
fn root_of(agent: &mut Agent<TestHost>, realm: &Realm, name: &str) -> Root {
    match eval(agent, realm, name) {
        Outcome::Normal(root) => root,
        other => panic!("{name}: {other:?}"),
    }
}

/// The state of the promise the global `name` of `realm` holds, read with `promise_state`:
/// `pending`, or `fulfilled` or `rejected` with the value as [`support::script::show`] writes
/// it.
fn state(agent: &mut Agent<TestHost>, realm: &Realm, name: &str) -> String {
    let promise = root_of(agent, realm, name);
    match agent.promise_state(&promise).unwrap() {
        PromiseState::Pending => "pending".to_string(),
        PromiseState::Fulfilled(value) => agent
            .with_cx(realm, |cx| {
                let value = cx.get(&value).unwrap();
                format!("fulfilled {}", support::script::show(cx, value))
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

/// The iterations of [`a_for_await_longer_than_frame_depth_runs_to_its_end`]: 1,000, the story's
/// count; 100 with the feature `collect-every-safe-point`, where every safe point of the loop
/// collects the whole heap (as `RESUMED_LOOP` of `tests/async_functions.rs`).
const LOOP: u32 = if cfg!(feature = "collect-every-safe-point") {
    100
} else {
    1_000
};

/// The `frame_depth` of the depth tests.
const FRAME_DEPTH: u32 = 16;

/// ForIn/OfBodyEvaluation with iteratorKind async, steps 6.a to 6.f: GetIterator with kind async
/// takes the method of `%Symbol.asyncIterator%` (not the one of `%Symbol.iterator%`), and each
/// step calls `next`, awaits its result (a promise, a thenable or a plain object alike), then
/// reads `done` and, when it is false, `value`; the body runs in a later job than the call that
/// started the loop. GetIterator throws a `TypeError` for a method that is not callable, for a
/// method that answers no object, and when neither method exists; a result that is not an object
/// throws a `TypeError` at the step.
#[test]
fn for_await_over_an_async_iterable_awaits_each_result() {
    let (mut agent, realm) = new_agent(limits());
    run(
        &mut agent,
        &realm,
        "var log = [], out = 'pending';\n\
         function result(v, d) { return { get done() { log.push('done ' + v); return d; },\n\
           get value() { log.push('value ' + v); return v; } }; }\n\
         var iterable = {\n\
           [Symbol.iterator]() { log.push('sync'); return [][Symbol.iterator](); },\n\
           [Symbol.asyncIterator]() { log.push('get'); var i = 0; return { next() {\n\
             i++; log.push('next ' + i + ' ' + arguments.length);\n\
             if (i === 1) return Promise.resolve(result(1, false));\n\
             if (i === 2) return { then(r) { log.push('then 2'); r(result(2, false)); } };\n\
             return result(3, true); } }; } };\n\
         (async function () { for await (var x of iterable) log.push('body ' + x); return 'end'; })()\n\
           .then(function (v) { out = v; }, function (e) { out = 'rejected ' + e; });\n\
         log.push('call returned');",
    );
    assert_eq!(
        text(&mut agent, &realm, "log.join()"),
        "get,next 1 0,call returned"
    );
    normal(&mut agent);
    assert_eq!(
        text(&mut agent, &realm, "log.join()"),
        "get,next 1 0,call returned,done 1,value 1,body 1,next 2 0,then 2,done 2,value 2,body 2,\
         next 3 0,done 3"
    );
    assert_eq!(text(&mut agent, &realm, "out"), "end");

    run(
        &mut agent,
        &realm,
        "var errors = [];\n\
         async function probe(name, iterable) {\n\
           try { for await (var x of iterable) {} errors.push(name + ' none'); }\n\
           catch (e) { errors.push(name + ' ' + (e instanceof TypeError ? 'TypeError' : e)); } }\n\
         probe('not-callable', { [Symbol.asyncIterator]: 1 });\n\
         probe('not-object', { [Symbol.asyncIterator]() { return 1; } });\n\
         probe('result', { [Symbol.asyncIterator]() { return { next() { return 1; } }; } });\n\
         probe('neither', {});",
    );
    normal(&mut agent);
    assert_eq!(
        text(&mut agent, &realm, "errors.join()"),
        "not-callable TypeError,not-object TypeError,neither TypeError,result TypeError"
    );
}

/// The host function `next` of HOST-203's host object: `{ value: 1, done: false }` at its first
/// call, `{ done: true }` after, each a host object with a null [[Prototype]].
fn host_next(cx: &mut Cx<'_, TestHost>, _call: HostCall<'_>) -> Completion<Value> {
    let first = cx.host().count == 0;
    cx.host_mut().count += 1;
    let result = cx
        .new_host_object(swell::HostObjectSpec {
            prototype: Prototype::Null,
            brand: swell::Brand(3),
            data: None,
            error_kind: None,
        })
        .unwrap()?;
    if first {
        let value = key(cx, "value");
        cx.define_property_or_throw(result, value, data(Value::Number(1.0), true, true, true))
            .unwrap()?;
    }
    let done = key(cx, "done");
    cx.define_property_or_throw(result, done, data(Value::Boolean(!first), true, true, true))
        .unwrap()?;
    Ok(Value::Object(result))
}

/// The host function of HOST-203's own `@@asyncIterator` method: its `this`, the host object,
/// whose own `next` is [`host_next`].
fn host_async_iterator(_cx: &mut Cx<'_, TestHost>, call: HostCall<'_>) -> Completion<Value> {
    Ok(call.this)
}

/// Define the own method `key` of `object` as Web IDL's DefineMethodProperty does
/// ({ [[Writable]]: true, [[Enumerable]]: false, [[Configurable]]: true }).
fn define_method(
    cx: &mut Cx<'_, TestHost>,
    object: ObjectRef,
    key: PropertyKey,
    method: ObjectRef,
) {
    cx.define_property_or_throw(object, key, data(Value::Object(method), true, false, true))
        .unwrap()
        .unwrap();
}

/// HOST-203 (`webidl:define-the-asynchronous-iteration-methods`): in the setup closure and in
/// `with_cx` of each of two realms of the agent, `well_known_symbol(AsyncIterator)` answers the
/// same Symbol, for which the script of the requirement answers `true` in either realm with `s`
/// a global holding it; a host object whose own `@@asyncIterator` method, defined with it,
/// answers an object with a `next` method that answers `{ value: 1, done: false }` and then
/// `{ done: true }` makes the `for await` of the requirement answer a promise fulfilled with
/// `"1"` after a checkpoint, read with `promise_state`; the read spends no step
/// (`last_entry_steps()` unchanged).
#[test]
fn host_203_a_host_reads_symbol_async_iterator_and_a_host_object_is_async_iterable() {
    let mut agent = support::host::agent().unwrap();
    let first = realm(&mut agent);
    run(&mut agent, &first, "0;");
    let steps = agent.last_entry_steps().unwrap();
    let symbol = agent
        .with_cx(&first, |cx| {
            cx.well_known_symbol(WellKnownSymbol::AsyncIterator)
                .unwrap()
        })
        .unwrap();
    assert_eq!(
        agent.last_entry_steps(),
        Ok(steps),
        "the read spends no step"
    );

    let mut in_setup: Vec<SymbolRef> = Vec::new();
    let mut setup = |cx: &mut HookCx<'_, TestHost>, realm: RealmRef| -> Completion<GlobalObjects> {
        in_setup.push(
            cx.well_known_symbol(WellKnownSymbol::AsyncIterator)
                .unwrap(),
        );
        let global = cx
            .new_host_object(swell::HostObjectSpec {
                prototype: Prototype::ObjectPrototypeOf(realm),
                brand: swell::Brand(2),
                data: None,
                error_kind: None,
            })
            .unwrap()?;
        Ok(GlobalObjects { global, this: None })
    };
    let Outcome::Normal(second) =
        support::host::create_realm(&mut agent, Some(&mut setup), None).unwrap()
    else {
        panic!("create_realm did not answer Normal");
    };
    assert_eq!(in_setup, [symbol], "the setup closure of the second realm");

    let requirement = "s === Symbol.asyncIterator && s.description === \"Symbol.asyncIterator\" \
                       && Symbol.keyFor(s) === undefined";
    for realm in [&first, &second] {
        let global_root = global(&mut agent, realm);
        let named = agent
            .with_cx(realm, |cx| {
                let named = cx
                    .well_known_symbol(WellKnownSymbol::AsyncIterator)
                    .unwrap();
                let global = object(cx, &global_root);
                let s = key(cx, "s");
                cx.set(global, s, Value::Symbol(named), true)
                    .unwrap()
                    .unwrap();
                named
            })
            .unwrap();
        assert_eq!(named, symbol, "with_cx of a realm");
        assert_eq!(
            eval_text(&mut agent, realm, requirement),
            Shown::Normal("true".to_string())
        );
    }

    let global_root = global(&mut agent, &first);
    agent
        .with_cx(&first, |cx| {
            let realm_ref = cx.realm_ref(&first).unwrap();
            let o = cx
                .new_host_object(swell::HostObjectSpec {
                    prototype: Prototype::ObjectPrototypeOf(realm_ref),
                    brand: swell::Brand(1),
                    data: None,
                    error_kind: None,
                })
                .unwrap()
                .unwrap();
            let method = cx
                .new_host_function(
                    realm_ref,
                    "[Symbol.asyncIterator]",
                    0,
                    host_async_iterator,
                    false,
                    None,
                )
                .unwrap()
                .unwrap();
            let async_iterator = cx
                .well_known_symbol(WellKnownSymbol::AsyncIterator)
                .unwrap();
            define_method(cx, o, PropertyKey::Symbol(async_iterator), method);
            let next = cx
                .new_host_function(realm_ref, "next", 0, host_next, false, None)
                .unwrap()
                .unwrap();
            let next_key = key(cx, "next");
            define_method(cx, o, next_key, next);
            let global = object(cx, &global_root);
            let o_key = key(cx, "o");
            cx.set(global, o_key, Value::Object(o), true)
                .unwrap()
                .unwrap();
        })
        .unwrap();
    run(
        &mut agent,
        &first,
        "var p = (async function () { var r = []; for await (var x of o) r.push(x); return r.join(); })();",
    );
    assert_eq!(state(&mut agent, &first, "p"), "pending");
    normal(&mut agent);
    assert_eq!(state(&mut agent, &first, "p"), "fulfilled \"1\"");
    assert_eq!(agent.host().count, 2, "next was called twice");
}

/// CreateAsyncFromSyncIterator (`#sec-createasyncfromsynciterator`) and
/// AsyncFromSyncIteratorContinuation (`#sec-asyncfromsynciteratorcontinuation`): over a sync
/// iterable with no `%Symbol.asyncIterator%` method, each value is awaited, so a promise and a
/// thenable are unwrapped and a plain value passes as it is; a generator's values alike.
#[test]
fn for_await_over_a_sync_iterable_awaits_each_value() {
    let (mut agent, realm) = new_agent(limits());
    run(
        &mut agent,
        &realm,
        "var out = 'pending', fromGenerator = 'pending';\n\
         var thenable = { then(r) { r(3); } };\n\
         (async function () { var r = []; for await (var x of [Promise.resolve(1), 2, thenable]) r.push(x);\n\
           return r.join(); })().then(function (v) { out = v; });\n\
         function* g() { yield Promise.resolve('a'); yield 'b'; }\n\
         (async function () { var r = []; for await (let x of g()) r.push(x); return r.join(); })()\n\
           .then(function (v) { fromGenerator = v; });",
    );
    assert_eq!(text(&mut agent, &realm, "out"), "pending");
    normal(&mut agent);
    assert_eq!(text(&mut agent, &realm, "out"), "1,2,3");
    assert_eq!(text(&mut agent, &realm, "fromGenerator"), "a,b");
}

/// AsyncIteratorClose with a normal completion (`#sec-asynciteratorclose`): a `break` (and a
/// `return`) out of the loop calls `return` of the iterator with no argument, awaits its result,
/// and goes on only once that settles; a result that is not an object throws a `TypeError`.
/// Over a sync iterable, `%AsyncFromSyncIteratorPrototype%.return` calls the sync iterator's
/// `return`; a sync iterator with none closes nothing.
#[test]
fn a_break_in_for_await_closes_the_iterator_and_awaits_its_return() {
    let (mut agent, realm) = new_agent(limits());
    run(
        &mut agent,
        &realm,
        "var log = [], out = 'pending', resolveReturn;\n\
         var iterable = { [Symbol.asyncIterator]() { return {\n\
           next() { return Promise.resolve({ value: 1, done: false }); },\n\
           return() { log.push('return ' + arguments.length);\n\
             return new Promise(function (r) { resolveReturn = r; }); } }; } };\n\
         (async function () { for await (var x of iterable) { log.push('body'); break; }\n\
           log.push('after'); return 'end'; })().then(function (v) { out = v; });",
    );
    normal(&mut agent);
    assert_eq!(text(&mut agent, &realm, "log.join()"), "body,return 0");
    assert_eq!(text(&mut agent, &realm, "out"), "pending");
    run(&mut agent, &realm, "resolveReturn({});");
    normal(&mut agent);
    assert_eq!(
        text(&mut agent, &realm, "log.join()"),
        "body,return 0,after"
    );
    assert_eq!(text(&mut agent, &realm, "out"), "end");

    run(
        &mut agent,
        &realm,
        "var results = [];\n\
         function closing(ret) { return { [Symbol.asyncIterator]() { return {\n\
           next() { return { value: 1, done: false }; }, return: ret }; } }; }\n\
         (async function () { try { for await (var x of closing(function () { return 5; })) break;\n\
           results.push('non-object none'); } catch (e) { results.push('non-object ' + (e instanceof TypeError)); } })();\n\
         (async function () { for await (var x of closing(function () { results.push('return called'); return {}; }))\n\
           return 'r'; })().then(function (v) { results.push('returned ' + v); });\n\
         var sync = { [Symbol.iterator]() { return { next() { return { value: 1, done: false }; },\n\
           return() { results.push('sync return ' + arguments.length); return {}; } }; } };\n\
         (async function () { for await (var x of sync) break; results.push('sync after'); })();\n\
         (async function () { for await (var x of [1, 2]) break; results.push('array after'); })();",
    );
    normal(&mut agent);
    assert_eq!(
        text(&mut agent, &realm, "results.slice().sort().join()"),
        "array after,non-object true,return called,returned r,sync after,sync return 0"
    );
}

/// AsyncIteratorClose with a throw completion (`#sec-asynciteratorclose`, steps 4 to 6): a throw
/// in the body calls `return`, awaits its result, and then throws the body's value, whatever
/// `return` did: completed, threw, answered a non-object, answered a rejected promise, or could
/// not be read; while the result of `return` is pending, the throw waits for it.
#[test]
fn a_throw_in_for_await_closes_the_iterator_and_keeps_the_throw() {
    let (mut agent, realm) = new_agent(limits());
    run(
        &mut agent,
        &realm,
        "var caught = {}, closes = {}, resolveLater;\n\
         function iterable(name, ret) { var it = { next() { return Promise.resolve({ value: 1, done: false }); } };\n\
           if (ret) it.return = function () { closes[name] = (closes[name] || 0) + 1; return ret(); };\n\
           else Object.defineProperty(it, 'return', { get() { closes[name] = 1; throw new RangeError('getter'); } });\n\
           return { [Symbol.asyncIterator]() { return it; } }; }\n\
         async function run(name, ret) { try { for await (var x of iterable(name, ret)) throw new Error(name); }\n\
           catch (e) { caught[name] = e.message; } }\n\
         run('plain', function () { return {}; });\n\
         run('throws', function () { throw new TypeError('from return'); });\n\
         run('non-object', function () { return 1; });\n\
         run('rejects', function () { return Promise.reject(new TypeError('rejected')); });\n\
         run('getter', null);\n\
         run('awaited', function () { return new Promise(function (r) { resolveLater = r; }); });\n\
         var names = ['plain', 'throws', 'non-object', 'rejects', 'getter', 'awaited'];\n\
         function read() { return names.map(function (n) { return n + ':' + caught[n] + ':' + closes[n]; }).join(); }",
    );
    normal(&mut agent);
    assert_eq!(
        text(&mut agent, &realm, "read()"),
        "plain:plain:1,throws:throws:1,non-object:non-object:1,rejects:rejects:1,getter:getter:1,\
         awaited:undefined:1"
    );
    run(&mut agent, &realm, "resolveLater(1);");
    normal(&mut agent);
    assert_eq!(
        text(&mut agent, &realm, "read()"),
        "plain:plain:1,throws:throws:1,non-object:non-object:1,rejects:rejects:1,getter:getter:1,\
         awaited:awaited:1"
    );
}

/// AsyncFromSyncIteratorContinuation, steps 9 to 13: a value of a sync iterator that is a
/// rejected promise, with `done` false, closes the sync iterator (its `return` called once) and
/// rejects with the reason, which the loop throws without closing again; with `done` true it
/// closes nothing.
#[test]
fn a_rejected_value_of_a_sync_iterator_closes_it() {
    let (mut agent, realm) = new_agent(limits());
    run(
        &mut agent,
        &realm,
        "var log = [];\n\
         function sync(name, done) { return { [Symbol.iterator]() { var i = 0; return {\n\
           next() { i++; log.push(name + ' next ' + i);\n\
             return i === 1 ? { value: 'a', done: false } : { value: Promise.reject(new RangeError(name)), done: done }; },\n\
           return() { log.push(name + ' return'); return {}; } }; } }; }\n\
         async function loop(name, done) { try { for await (var x of sync(name, done)) log.push(name + ' body ' + x); }\n\
           catch (e) { log.push(name + ' caught ' + (e instanceof RangeError) + ' ' + e.message); } }\n\
         loop('open', false);",
    );
    normal(&mut agent);
    assert_eq!(
        text(&mut agent, &realm, "log.join()"),
        "open next 1,open body a,open next 2,open return,open caught true open"
    );
    run(&mut agent, &realm, "log = []; loop('done', true);");
    normal(&mut agent);
    assert_eq!(
        text(&mut agent, &realm, "log.join()"),
        "done next 1,done body a,done next 2,done caught true done"
    );
}

/// "`async` functions on suspended frames", item 4: each iteration of a `for await` is resumed
/// by a job of its own, so a loop of [`LOOP`] iterations over a sync iterable, with
/// `frame_depth` [`FRAME_DEPTH`], runs to its end and takes no depth.
#[test]
fn a_for_await_longer_than_frame_depth_runs_to_its_end() {
    let mut limits = limits();
    limits.frame_depth = FRAME_DEPTH;
    let (mut agent, realm) = new_agent(limits);
    run(
        &mut agent,
        &realm,
        &format!(
            "var count = 0, out = 'pending', list = [];\n\
             for (var i = 0; i < {LOOP}; i++) list.push(i);\n\
             (async function () {{ for await (var x of list) count++; return count; }})()\n\
               .then(function (v) {{ out = v; }}, function (e) {{ out = String(e); }});"
        ),
    );
    let ran = normal(&mut agent);
    assert!(ran.promise_jobs > u64::from(LOOP), "{ran:?}");
    assert_eq!(text(&mut agent, &realm, "out"), LOOP.to_string());
    assert_eq!(agent.pending_microtasks(), Ok(0));
}

/// The script of the depth tests: `a()` runs a `for await` over a sync iterable whose first
/// value is the promise `p` and whose second `next` answers `done`; its body records, and a
/// throw from the loop is caught and recorded by its kind. `p` is resolved deep, right before a
/// checkpoint there, so the job that resumes the loop and its second call of `next` run at that
/// depth.
const DEPTH_SCRIPT: &str = "var log = [], resolveP;\n\
     var p = new Promise(function (r) { resolveP = r; });\n\
     var sync = { [Symbol.iterator]() { var i = 0; return { next() { i++;\n\
       return i === 1 ? { value: p, done: false } : { done: true }; } }; } };\n\
     async function a() { try { for await (var x of sync) log.push('body ' + x); log.push('end'); }\n\
       catch (e) { log.push(e instanceof RangeError ? 'RangeError' : String(e)); } }\n\
     var call = a();\n";

/// The body of [`a_sync_next_called_through_async_from_sync_at_reentry_depth_throws_a_range_error`],
/// on the bounded stack. `f(k)` recurses through `reenter` `k` times, then resolves `p` and runs
/// a checkpoint through `checkpointHere`; the job of the `await` of the loop resumes `a` there
/// (one re-entry for the call of the function of the `await`, one for the resumption, as
/// `tests/async_functions.rs` counts them), and its second step calls
/// `%AsyncFromSyncIteratorPrototype%.next`, which calls the sync `next` from Rust: one re-entry
/// more, and the deepest: the capability of `next`, PromiseResolve of the value and the
/// settling of the capability make no call. With `reentry_depth` R, at k = R - 2 that call
/// would pass R: the promise of `next` is rejected with the `RangeError` of "Depth bounds",
/// which the `await` of the loop throws and `a` catches. At k = R - 3 the loop runs to its end.
fn next_through_reenter() {
    let reentry = BUILTINS_REENTRY;
    for (k, expected) in [
        (reentry - 2, "body 1,RangeError"),
        (reentry - 3, "body 1,end"),
    ] {
        let (mut agent, realm) = stack_agent(reentry);
        define_function(
            &mut agent,
            &realm,
            "checkpointHere",
            0,
            checkpoint_here,
            false,
        );
        let script = format!(
            "{DEPTH_SCRIPT}function f(k) {{\n\
               if (k === 0) {{ resolveP(1); return checkpointHere(); }}\n\
               return reenter(function () {{ return f(k - 1); }}); }}\n\
             f({k}); log.join();"
        );
        assert_eq!(
            eval_text(&mut agent, &realm, &script),
            Shown::Normal(format!("\"{expected}\"")),
            "k = {k}, reentry_depth {reentry}"
        );
    }
}

/// "Depth bounds" and "The Async-from-Sync Iterator": the call of the sync iterator's method by
/// `%AsyncFromSyncIteratorPrototype%.next` is a re-entry, refused past `reentry_depth` with a
/// `RangeError` that the loop throws, and the deepest call the bound lets run does not overflow
/// seven eighths of `min_stack_bytes()` ([`next_through_reenter`]).
#[test]
fn a_sync_next_called_through_async_from_sync_at_reentry_depth_throws_a_range_error() {
    let stack = stack_with_room(stack_limits(BUILTINS_REENTRY));
    on_bounded_stack(
        test_path!(
            "a_sync_next_called_through_async_from_sync_at_reentry_depth_throws_a_range_error"
        ),
        stack,
        next_through_reenter,
    )
    .unwrap();
}

/// "Depth bounds": the call of `next` from the loop is a call checked against `frame_depth`.
/// `down(k)` recurses `k` calls deep, then resolves `p` and runs a checkpoint through
/// `checkpointHere`: with the script's frame and `down`'s k + 1 frames, the resumption of `a`
/// takes one frame more and the sync `next` one more again. At k = [`FRAME_DEPTH`] - 3 the
/// resumption fits and the call of `next` throws the `RangeError`, which `a` catches; at
/// k = [`FRAME_DEPTH`] - 4 the loop runs to its end.
#[test]
fn a_for_await_at_frame_depth_throws_a_range_error() {
    for (k, expected) in [
        (FRAME_DEPTH - 3, "body 1,RangeError"),
        (FRAME_DEPTH - 4, "body 1,end"),
    ] {
        let mut limits = limits();
        limits.frame_depth = FRAME_DEPTH;
        let (mut agent, realm) = new_agent(limits);
        run(&mut agent, &realm, DEPTH_SCRIPT);
        run(
            &mut agent,
            &realm,
            &format!(
                "function down(k) {{ if (k === 0) {{ resolveP(1); checkpointHere(); return; }} down(k - 1); }}\n\
                 down({k});"
            ),
        );
        assert_eq!(text(&mut agent, &realm, "log.join()"), expected, "k = {k}");
        assert_eq!(
            state(&mut agent, &realm, "call"),
            "fulfilled undefined",
            "k = {k}"
        );
    }
}

/// The `steps_per_entry` of the step-budget test.
const LOOP_BUDGET: u64 = 10_000;

/// "`async` functions on suspended frames", items 4 and 5, and HOST-199: a `for await` over an
/// async iterable whose `next` answers a resolved promise of `{ done: false }` for ever runs
/// in jobs that each queue the next; with `steps_per_entry` [`LOOP_BUDGET`] the checkpoint
/// answers `Terminated(StepBudget)` with `last_entry_steps()` at most the budget. Each later
/// checkpoint goes on from the queue: it answers `Terminated(StepBudget)` within the budget
/// while an entry is queued, and `Normal` with nothing run once none is; the promise of the
/// call stays pending.
#[test]
fn a_for_await_over_an_iterator_without_end_ends_at_the_step_budget() {
    let mut limits = limits();
    limits.steps_per_entry = LOOP_BUDGET;
    let (mut agent, realm) = new_agent(limits);
    run(
        &mut agent,
        &realm,
        "var turns = 0;\n\
         var endless = { [Symbol.asyncIterator]() { return {\n\
           next() { return Promise.resolve({ value: 0, done: false }); } }; } };\n\
         var p = (async function () { for await (var x of endless) turns++; })();",
    );
    assert_eq!(terminated(&mut agent), Termination::StepBudget);
    let steps = agent.last_entry_steps().unwrap();
    assert!(steps <= LOOP_BUDGET, "{steps}");
    let mut outcomes = vec![format!("StepBudget {steps}")];
    loop {
        let queued = agent.pending_microtasks().unwrap();
        match checkpoint(&mut agent) {
            Outcome::Terminated(Termination::StepBudget) => {
                assert!(queued > 0, "{outcomes:?}");
                let steps = agent.last_entry_steps().unwrap();
                assert!(steps <= LOOP_BUDGET, "{steps}");
                outcomes.push(format!("StepBudget {steps}"));
                assert!(outcomes.len() < 10, "{outcomes:?}");
            }
            Outcome::Normal(report) => {
                assert_eq!(queued, 0, "{outcomes:?}");
                assert_eq!(
                    report,
                    CheckpointReport {
                        promise_jobs: 0,
                        host_microtasks: 0
                    }
                );
                break;
            }
            other => panic!("{other:?} after {outcomes:?}"),
        }
    }
    println!("checkpoints: {outcomes:?}");
    let turns: u32 = text(&mut agent, &realm, "turns").parse().unwrap();
    assert!(turns > 10, "{turns}");
    assert_eq!(state(&mut agent, &realm, "p"), "pending");
}

/// The bytes of [[SyncIteratorRecord]] in the box of an async-from-sync iterator: the size of an
/// Iterator Record (`ops/iteration.rs`, `IteratorRecord`, which the library's test
/// `an_async_from_sync_iterator_is_charged_its_record` reads), on this target.
const SYNC_RECORD_BYTES: u64 = 32;

/// The bytes of the slots of the unwrap function and of the function that closes the sync
/// iterator, each (`object/builtin_function.rs`, read by the library's test
/// `the_slots_of_a_closure_are_charged_their_box`).
const CONTINUATION_BOX_BYTES: u64 = 32;

/// The bytes one reaction takes in the lists of a pending promise (`object/promise.rs`,
/// `REACTION_BYTES`, read by the library's test `a_reaction_is_charged_its_size`).
const REACTION_BYTES: u64 = 112;

/// What one `for await` over a sync iterable whose first value never settles holds at least:
/// the async-from-sync iterator's box, the unwrap function and the closing function with their
/// boxes, and the reaction of those two in the lists of the value.
const PER_LOOP: u64 = SYNC_RECORD_BYTES + 2 * CONTINUATION_BOX_BYTES + REACTION_BYTES;

/// The `async` functions of [`an_async_from_sync_iterator_is_charged_until_it_is_freed`]: 100;
/// 30 with the feature `collect-every-safe-point`, where every safe point collects the whole
/// heap, which grows with each function (at 100 the test took 3.5 CPU seconds there).
const HELD: u64 = if cfg!(feature = "collect-every-safe-point") {
    30
} else {
    100
};

/// "The Async-from-Sync Iterator": [`HELD`] `async` functions, each in a `for await` over a sync
/// iterable whose first value is a promise that never settles, make `heap_bytes()` exceed the
/// reading before by at least [`HELD`] times [`PER_LOOP`]; once the script releases its values,
/// one `collect()` gives at least that back. A first round grows the arenas first. The test
/// prints the readings.
#[test]
fn an_async_from_sync_iterator_is_charged_until_it_is_freed() {
    let (mut agent, realm) = new_agent(limits());
    run(&mut agent, &realm, "var never, sync, calls;");
    let start = format!(
        "never = new Promise(function () {{}});\n\
         sync = {{ [Symbol.iterator]() {{ return {{ next() {{ return {{ value: never, done: false }}; }} }}; }} }};\n\
         calls = [];\n\
         for (var i = 0; i < {HELD}; i++) calls.push((async function () {{ for await (var x of sync) {{}} }})());"
    );
    let release = "never = sync = calls = undefined;";
    let mut readings = Vec::new();
    for _round in 0..2 {
        let started = parse(&mut agent, &realm, &start);
        let released = parse(&mut agent, &realm, release);
        let before = collected(&mut agent);
        assert!(matches!(
            agent.evaluate_script(&started),
            Ok(Outcome::Normal(_))
        ));
        let held = collected(&mut agent);
        assert!(matches!(
            agent.evaluate_script(&released),
            Ok(Outcome::Normal(_))
        ));
        let freed = collected(&mut agent);
        readings.push((before, held, freed));
    }
    println!("heap_bytes (before, held, freed) per round: {readings:?}; {HELD} x {PER_LOOP}");
    let (before, held, freed) = readings[1];
    assert!(held >= before + HELD * PER_LOOP, "{readings:?}");
    assert!(
        held.saturating_sub(freed) >= HELD * PER_LOOP,
        "{readings:?}"
    );
}

/// The room of the memory budget test above the reading of a fresh realm.
const FILL_ROOM: u64 = 1024 * 1024;

/// Statements that spend any room in a few steps and safe points: Strings that double, each
/// kept, until the charge of one is refused.
const FILL: &str = "var a = []; for (var n = 1024; ; n = n * 2) a.push(\"x\".repeat(n));";

/// Item 5 of "`async` functions on suspended frames" for a `for await` whose body runs when the
/// checkpoint ends in `termination` (`ending` is the body after its first statement): the
/// iterator is not closed (its `return` is never called), a `finally` around the loop does not
/// run, and the promise of the call stays pending after the termination and after a later
/// checkpoint, which answers `Normal` with both counts 0.
fn closes_nothing(
    agent: &mut Agent<TestHost>,
    realm: &Realm,
    ending: &str,
    termination: Termination,
) {
    run(
        agent,
        realm,
        &format!(
            "var closed = 0, ranFinally = false, bodies = 0;\n\
             var it = {{ [Symbol.asyncIterator]() {{ return {{\n\
               next() {{ return Promise.resolve({{ value: 1, done: false }}); }},\n\
               return() {{ closed++; return {{}}; }} }}; }} }};\n\
             var p = (async function () {{ try {{ for await (var x of it) {{ bodies++; {ending} }} }}\n\
               finally {{ ranFinally = true; }} }})();"
        ),
    );
    assert_eq!(terminated(agent), termination);
    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
    assert_eq!(
        text(agent, realm, "bodies + ' ' + closed + ' ' + ranFinally"),
        "1 0 false"
    );
    assert_eq!(state(agent, realm, "p"), "pending");
    assert_eq!(
        normal(agent),
        CheckpointReport {
            promise_jobs: 0,
            host_microtasks: 0
        }
    );
    assert_eq!(
        text(agent, realm, "bodies + ' ' + closed + ' ' + ranFinally"),
        "1 0 false"
    );
    assert_eq!(state(agent, realm, "p"), "pending");
}

/// Item 5: a termination while the body of a `for await` runs, `Terminated(StepBudget)`,
/// `Terminated(MemoryBudget)` and `Terminated(Interrupted)` alike, closes no iterator and leaves
/// the promise of the call pending for good ([`closes_nothing`]).
#[test]
fn a_termination_in_the_body_of_for_await_closes_nothing_and_leaves_the_promise_pending() {
    let mut step_limits = limits();
    step_limits.steps_per_entry = 300;
    let (mut agent, realm) = new_agent(step_limits);
    closes_nothing(&mut agent, &realm, "for (;;) {}", Termination::StepBudget);

    let realm_bytes = {
        let (agent, _realm) = new_agent(limits());
        agent.heap_bytes().unwrap()
    };
    let mut memory_limits = limits();
    memory_limits.memory_bytes = realm_bytes + FILL_ROOM;
    let (mut agent, realm) = new_agent(memory_limits);
    closes_nothing(&mut agent, &realm, FILL, Termination::MemoryBudget);

    let (mut agent, realm) = new_agent(limits());
    closes_nothing(
        &mut agent,
        &realm,
        "interruptNow(); for (;;) {}",
        Termination::Interrupted,
    );
}

/// Decision 4, "Symbols": `%Symbol.asyncIterator%` is one Symbol per agent, the same in two
/// realms, with the description `Symbol.asyncIterator`, in no registry (`Symbol.for` of its
/// description gives another Symbol), held by `Symbol.asyncIterator`, which is neither writable
/// nor enumerable nor configurable (`#sec-symbol.asynciterator`).
#[test]
fn symbol_async_iterator_is_one_well_known_symbol_per_agent() {
    let mut agent = support::host::agent().unwrap();
    let first = realm(&mut agent);
    let second = realm(&mut agent);
    let from_first = root_of(&mut agent, &first, "Symbol.asyncIterator");
    let from_second = root_of(&mut agent, &second, "Symbol.asyncIterator");
    let same = agent
        .with_cx(&first, |cx| {
            let a = cx.get(&from_first).unwrap();
            let b = cx.get(&from_second).unwrap();
            matches!(a, Value::Symbol(_)) && a == b
        })
        .unwrap();
    assert!(same, "the same Symbol in both realms");
    assert_eq!(
        text(
            &mut agent,
            &second,
            "[typeof Symbol.asyncIterator, Symbol.asyncIterator.description,\n\
              Symbol.asyncIterator.toString(), Symbol.keyFor(Symbol.asyncIterator),\n\
              Symbol.for('Symbol.asyncIterator') === Symbol.asyncIterator,\n\
              JSON.stringify(Object.getOwnPropertyDescriptor(Symbol, 'asyncIterator'))]"
        ),
        "symbol,Symbol.asyncIterator,Symbol(Symbol.asyncIterator),,false,\
         {\"writable\":false,\"enumerable\":false,\"configurable\":false}"
    );
}
