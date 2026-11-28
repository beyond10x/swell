//! `Array.fromAsync` (`#sec-array.fromasync`, `story:array-from-async`): a built-in async
//! function (`docs/design/engine.md`, decision 3, "A built-in async function", items 1 to 3)
//! whose steps await each result of an async iterator, each value of a sync iterable (through
//! the Async-from-Sync Iterator), each element of an array-like and each result of the mapper,
//! and close the async iterator (`#sec-asynciteratorclose`) when the mapper or the definition of
//! an element ends abruptly; its charge, its steps and depth bounds, the terminations, and
//! HOST-207 and HOST-208 (`docs/design/host-interface.md`). Run also with the feature
//! `collect-every-safe-point`.
//!
//! The promise of a call is read with `promise_state`; a value it was settled with is put on the
//! global object as `settledValue` and read by a script ([`state`]). A test that reads
//! `heap_bytes()` around an entry parses that entry's script before the first reading and
//! declares the global names it assigns in an earlier script, as `tests/async_functions.rs`
//! does. `2 ** 53 - 1` is written 9007199254740991: the exponentiation operator is slice 4's.
#![cfg(test)]

use crate::support;

use std::any::Any;
use std::rc::Rc;

use support::host::{limits, seeds};
use support::script::{Outcome as Shown, eval, eval_text};
use support::scripted::{Hooks, Scripted};
use support::stack::stack_agent;
use swell::{
    Agent, CheckpointReport, Completion, Cx, HookCx, HostCall, HostData, HostDefined,
    InterruptHandle, Job, Limits, LoadRequest, ObjectRef, Outcome, Parsed, PromiseState,
    PropertyKey, Realm, RealmRef, Root, Script, Source, Termination, Tracer, Value,
};

/// A [[HostDefined]] that holds the number of the `host_make_job_callback` call that made it.
struct Marker(u32);

impl HostData for Marker {
    fn trace(&self, _tracer: &mut Tracer<'_>) {}
}

/// The host of these tests: its job hooks record what they receive, in order, and
/// `host_make_job_callback` roots every function it receives in `kept`.
#[derive(Default)]
struct FromAsyncHost {
    /// The functions `host_make_job_callback` received, rooted, in order.
    kept: Vec<Root>,
    /// How often `host_make_job_callback` was called.
    made: u32,
    /// What the job hooks received, in order: `prepare`, `call <k> marker <m>` (the function
    /// the k-th `host_make_job_callback` kept, 0 for none, and the marker made by the m-th
    /// call), and `clean-up ok` or `clean-up err`.
    events: Vec<String>,
    /// The realm of each `host_prepare_to_run_job`.
    prepares: Vec<RealmRef>,
    /// The agent's interrupt handle, for the host function `interruptNow`.
    interrupt: Option<InterruptHandle>,
}

fn host_enqueue_generic_job(_cx: &mut HookCx<'_, Scripted>, _job: Job) {}
fn host_enqueue_timeout_job(_cx: &mut HookCx<'_, Scripted>, _job: Job, _milliseconds: f64) {}
fn host_enqueue_finalization_registry_cleanup_job(_cx: &mut HookCx<'_, Scripted>, _job: Job) {}
fn host_load_imported_module(_cx: &mut HookCx<'_, Scripted>, _request: LoadRequest) {}
fn host_system_utc_epoch_nanoseconds(_cx: &mut HookCx<'_, Scripted>, _global: ObjectRef) -> i128 {
    0
}

fn host_make_job_callback(
    cx: &mut HookCx<'_, Scripted>,
    callback: ObjectRef,
) -> Option<HostDefined> {
    cx.host_mut().state_mut::<FromAsyncHost>().made += 1;
    let root = cx.root(Value::Object(callback)).unwrap();
    cx.host_mut().state_mut::<FromAsyncHost>().kept.push(root);
    Some(Rc::new(Marker(cx.host().state::<FromAsyncHost>().made)))
}

fn host_call_job_callback(
    cx: &mut Cx<'_, Scripted>,
    callback: ObjectRef,
    host_defined: Option<&HostDefined>,
    this: Value,
    args: &[Value],
) -> Completion<Value> {
    let marker = host_defined
        .and_then(|data| (&**data as &dyn Any).downcast_ref::<Marker>())
        .map_or(0, |Marker(n)| *n);
    let kept = std::mem::take(&mut cx.host_mut().state_mut::<FromAsyncHost>().kept);
    // The function the marker names first, so a long run looks up each in one read.
    let named = usize::try_from(marker)
        .ok()
        .and_then(|m| m.checked_sub(1))
        .filter(|&k| {
            kept.get(k)
                .is_some_and(|root| cx.get(root).unwrap() == Value::Object(callback))
        });
    let which = named
        .or_else(|| {
            kept.iter()
                .position(|root| cx.get(root).unwrap() == Value::Object(callback))
        })
        .map_or(0, |k| k + 1);
    cx.host_mut().state_mut::<FromAsyncHost>().kept = kept;
    cx.host_mut()
        .state_mut::<FromAsyncHost>()
        .events
        .push(format!("call {which} marker {marker}"));
    cx.call(Value::Object(callback), this, args).unwrap()
}

fn host_prepare_to_run_job(cx: &mut HookCx<'_, Scripted>, realm: RealmRef) {
    cx.host_mut()
        .state_mut::<FromAsyncHost>()
        .events
        .push("prepare".to_string());
    cx.host_mut()
        .state_mut::<FromAsyncHost>()
        .prepares
        .push(realm);
}

fn host_clean_up_after_job(
    cx: &mut HookCx<'_, Scripted>,
    _realm: Option<RealmRef>,
    result: Result<(), Value>,
) {
    let event = match result {
        Ok(()) => "clean-up ok",
        Err(_) => "clean-up err",
    };
    cx.host_mut()
        .state_mut::<FromAsyncHost>()
        .events
        .push(event.to_string());
}

/// The hooks of the tests of this module, given to the scripted host with the state
/// [`FromAsyncHost`] (`support/scripted.rs`).
const HOOKS: Hooks = Hooks {
    host_enqueue_generic_job,
    host_enqueue_timeout_job,
    host_enqueue_finalization_registry_cleanup_job,
    host_load_imported_module,
    host_system_utc_epoch_nanoseconds,
    host_make_job_callback,
    host_call_job_callback,
    host_prepare_to_run_job,
    host_clean_up_after_job,
    ..Hooks::PROVIDED
};

/// The host function `interruptNow`: it sets the agent's interrupt flag.
fn interrupt_now(cx: &mut Cx<'_, Scripted>, _call: HostCall<'_>) -> Completion<Value> {
    if let Some(interrupt) = &cx.host().state::<FromAsyncHost>().interrupt {
        interrupt.interrupt();
    }
    Ok(Value::Undefined)
}

/// A realm of `agent` whose global `interruptNow` is the host function of this file.
fn new_realm(agent: &mut Agent<Scripted>) -> Realm {
    let realm = match support::host::create_realm(agent, None, None).unwrap() {
        Outcome::Normal(realm) => realm,
        other => panic!("create_realm answered {other:?}"),
    };
    let global = agent.global_object(&realm).unwrap();
    agent
        .with_cx(&realm, |cx| {
            let home = cx.realm_ref(&realm).unwrap();
            let function = cx
                .new_host_function(home, "interruptNow", 0, interrupt_now, false, None)
                .unwrap()
                .unwrap();
            let Value::Object(global) = cx.get(&global).unwrap() else {
                panic!("the global object");
            };
            let key = PropertyKey::String(cx.new_string("interruptNow").unwrap());
            cx.set(global, key, Value::Object(function), true)
                .unwrap()
                .unwrap();
        })
        .unwrap();
    realm
}

/// An agent of the host of these tests with `limits`, and a realm of [`new_realm`].
fn new_agent(limits: Limits) -> (Agent<Scripted>, Realm) {
    let mut agent = Agent::new(
        Scripted::new(FromAsyncHost::default(), HOOKS),
        limits,
        seeds(),
    )
    .unwrap();
    agent.host_mut().state_mut::<FromAsyncHost>().interrupt = Some(agent.interrupt_handle());
    let realm = new_realm(&mut agent);
    (agent, realm)
}

/// The test limits with `steps_per_entry` `steps`.
fn limits_with(steps: u64) -> Limits {
    let mut limits = limits();
    limits.steps_per_entry = steps;
    limits
}

/// The script `source` parsed, which must parse.
fn parse(agent: &mut Agent<Scripted>, realm: &Realm, source: &str) -> Script {
    match agent
        .parse_script(realm, Source::Utf8(source), None)
        .unwrap()
    {
        Parsed::Parsed(script) => script,
        other => panic!("{source:?} answered {other:?}"),
    }
}

/// The outcome of the script `source`, which must parse.
fn evaluate(agent: &mut Agent<Scripted>, realm: &Realm, source: &str) -> Outcome<Root> {
    let script = parse(agent, realm, source);
    agent.evaluate_script(&script).unwrap()
}

/// The script `source`, which must complete normally: its completion value.
fn run(agent: &mut Agent<Scripted>, realm: &Realm, source: &str) -> Root {
    match evaluate(agent, realm, source) {
        Outcome::Normal(value) => value,
        other => panic!("{source:?} answered {other:?}"),
    }
}

/// The parsed script `script`, which must complete normally.
fn run_parsed(agent: &mut Agent<Scripted>, script: &Script) {
    match agent.evaluate_script(script).unwrap() {
        Outcome::Normal(_) => {}
        other => panic!("the script answered {other:?}"),
    }
}

/// The text of `String(expression)`.
fn text(agent: &mut Agent<Scripted>, realm: &Realm, expression: &str) -> String {
    let root = run(agent, realm, &format!("String({expression})"));
    agent
        .with_cx(realm, |cx| match cx.get(&root).unwrap() {
            Value::String(string) => cx.to_rust_string_lossy(string).unwrap(),
            other => panic!("{other:?}"),
        })
        .unwrap()
}

/// A checkpoint that must answer `Normal`: its report.
fn normal(agent: &mut Agent<Scripted>) -> CheckpointReport {
    match agent.perform_microtask_checkpoint().unwrap() {
        Outcome::Normal(report) => report,
        other => panic!("the checkpoint answered {other:?}"),
    }
}

/// A checkpoint that must answer `Terminated`: its reason.
fn terminated(agent: &mut Agent<Scripted>) -> Termination {
    match agent.perform_microtask_checkpoint().unwrap() {
        Outcome::Terminated(termination) => termination,
        other => panic!("the checkpoint answered {other:?}"),
    }
}

/// A checkpoint report.
fn report(promise_jobs: u64, host_microtasks: u64) -> CheckpointReport {
    CheckpointReport {
        promise_jobs,
        host_microtasks,
    }
}

/// The global `name` of `realm` set to `value`.
fn set_global(agent: &mut Agent<Scripted>, realm: &Realm, name: &str, value: &Root) {
    let global = agent.global_object(realm).unwrap();
    agent
        .with_cx(realm, |cx| {
            let Value::Object(global) = cx.get(&global).unwrap() else {
                panic!("the global object");
            };
            let value = cx.get(value).unwrap();
            let key = PropertyKey::String(cx.new_string(name).unwrap());
            cx.set(global, key, value, true).unwrap().unwrap();
        })
        .unwrap();
}

/// The state of the promise `promise`, read with `promise_state`: `pending`, or `fulfilled` or
/// `rejected` with the value as a script of `realm` reads it once it is the global
/// `settledValue`: an error by its `name`, an Array as `[` its `join()` `]`, any other value as
/// `String` writes it.
fn state_of(agent: &mut Agent<Scripted>, realm: &Realm, promise: &Root) -> String {
    let (label, value) = match agent.promise_state(promise).unwrap() {
        PromiseState::Pending => return "pending".to_string(),
        PromiseState::Fulfilled(value) => ("fulfilled", value),
        PromiseState::Rejected(value) => ("rejected", value),
    };
    set_global(agent, realm, "settledValue", &value);
    let shown = text(
        agent,
        realm,
        "settledValue instanceof Error ? settledValue.name : Array.isArray(settledValue) ? \
         '[' + settledValue.join() + ']' : settledValue",
    );
    format!("{label} {shown}")
}

/// [`state_of`] the promise the expression `expression` reads.
fn state(agent: &mut Agent<Scripted>, realm: &Realm, expression: &str) -> String {
    let promise = run(agent, realm, expression);
    state_of(agent, realm, &promise)
}

/// `heap_bytes()` after a `collect()`.
fn collected(agent: &mut Agent<Scripted>) -> u64 {
    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
    agent.heap_bytes().unwrap()
}

/// The function `host_make_job_callback` kept `index`-th (from 0), duplicated.
fn kept(agent: &mut Agent<Scripted>, index: usize) -> Root {
    let root = agent
        .host_mut()
        .state_mut::<FromAsyncHost>()
        .kept
        .remove(index);
    let function = agent.duplicate(&root).unwrap();
    agent
        .host_mut()
        .state_mut::<FromAsyncHost>()
        .kept
        .insert(index, root);
    function
}

/// `Agent::call` of `function` with `undefined` as `this` and the one argument `value`: whether
/// it answered `Normal(undefined)`.
fn call_answers_undefined(
    agent: &mut Agent<Scripted>,
    realm: &Realm,
    function: &Root,
    value: Value,
) -> bool {
    let (undefined, argument) = agent
        .with_cx(realm, |cx| {
            (cx.root(Value::Undefined).unwrap(), cx.root(value).unwrap())
        })
        .unwrap();
    match agent
        .call(realm, function, &undefined, &[&argument])
        .unwrap()
    {
        Outcome::Normal(value) => agent
            .with_cx(realm, |cx| cx.get(&value).unwrap() == Value::Undefined)
            .unwrap(),
        _ => false,
    }
}

/// An async iterable whose `next` answers a promise of `{ value: i, done: false }` for i = 0,
/// 1, ... and of `{ done: true }` once i reaches `limit` (no end when `limit` is `Infinity`), and
/// whose `return` records `return` in `log`; `log` records `next <i>` too.
const COUNTING: &str = "var log = [];\n\
     function counting(limit) {\n\
       var source = {};\n\
       source[Symbol.asyncIterator] = function () {\n\
         return { i: 0,\n\
           next: function () { var i = this.i++; log.push('next ' + i);\n\
             return Promise.resolve(i < limit ? { value: i, done: false } : { value: 'end', done: true }); },\n\
           'return': function () { log.push('return'); return Promise.resolve({}); } };\n\
       };\n\
       return source;\n\
     }\n";

/// `#sec-array.fromasync`, the branch of an async iterable: the call reads
/// `Symbol.asyncIterator` and not `Symbol.iterator`, calls `next` once and answers a pending
/// promise; each later `next` is called by a job after the result before it was awaited, and a
/// value that is a promise is defined as it is (no Await of the value of an async iterator). The
/// checkpoint runs one job per Await (four results), and the promise is fulfilled with the
/// array.
#[test]
fn from_async_of_an_async_iterable_awaits_each_result() {
    let (mut agent, realm) = new_agent(limits());
    let promise = run(
        &mut agent,
        &realm,
        "var log = [], values = [0, 10, Promise.resolve(5)];\n\
         var source = {};\n\
         source[Symbol.asyncIterator] = function () { log.push('iter');\n\
           return { i: 0, next: function () { var i = this.i++; log.push('next ' + i);\n\
             return Promise.resolve(i < 3 ? { value: values[i], done: false } : { done: true }); } }; };\n\
         Object.defineProperty(source, Symbol.iterator, { get: function () { log.push('sync'); } });\n\
         var p = Array.fromAsync(source);\n\
         log.push('after the call');\n\
         p;",
    );
    assert_eq!(
        text(&mut agent, &realm, "log.join()"),
        "iter,next 0,after the call"
    );
    assert_eq!(state_of(&mut agent, &realm, &promise), "pending");
    assert_eq!(normal(&mut agent), report(4, 0));
    assert_eq!(
        text(&mut agent, &realm, "log.join()"),
        "iter,next 0,after the call,next 1,next 2,next 3"
    );
    assert_eq!(
        state_of(&mut agent, &realm, &promise),
        "fulfilled [0,10,[object Promise]]"
    );
    assert_eq!(
        text(&mut agent, &realm, "settledValue[2] === values[2]"),
        "true"
    );
}

/// The branch of a sync iterable: each value is awaited through the Async-from-Sync Iterator, so
/// a promise in the list that is not settled holds the call pending until it is, and the array
/// holds the values the promises were fulfilled with.
#[test]
fn from_async_of_a_sync_iterable_awaits_each_value() {
    let (mut agent, realm) = new_agent(limits());
    let promise = run(
        &mut agent,
        &realm,
        "var resolveB, b = new Promise(function (r) { resolveB = r; });\n\
         Array.fromAsync([1, Promise.resolve(2), b, 4]);",
    );
    normal(&mut agent);
    assert_eq!(state_of(&mut agent, &realm, &promise), "pending");
    run(&mut agent, &realm, "resolveB(3);");
    normal(&mut agent);
    assert_eq!(
        state_of(&mut agent, &realm, &promise),
        "fulfilled [1,2,3,4]"
    );
    let string = run(&mut agent, &realm, "Array.fromAsync('ab');");
    normal(&mut agent);
    assert_eq!(state_of(&mut agent, &realm, &string), "fulfilled [a,b]");
}

/// The branch of an array-like: element k is read only after element k - 1 was awaited, a
/// promise among them holds the call until it settles, and the array holds the awaited values in
/// order; `length` is set at the end.
#[test]
fn from_async_of_an_array_like_awaits_each_element_in_order() {
    let (mut agent, realm) = new_agent(limits());
    let promise = run(
        &mut agent,
        &realm,
        "var log = [], resolve1;\n\
         var p = Array.fromAsync({ length: 3,\n\
           get 0() { log.push('get 0'); return 'a'; },\n\
           get 1() { log.push('get 1'); return new Promise(function (r) { resolve1 = r; }); },\n\
           get 2() { log.push('get 2'); return 'c'; } });\n\
         log.push('after the call');\n\
         p;",
    );
    assert_eq!(
        text(&mut agent, &realm, "log.join()"),
        "get 0,after the call"
    );
    normal(&mut agent);
    assert_eq!(
        text(&mut agent, &realm, "log.join()"),
        "get 0,after the call,get 1"
    );
    assert_eq!(state_of(&mut agent, &realm, &promise), "pending");
    run(&mut agent, &realm, "resolve1('b');");
    normal(&mut agent);
    assert_eq!(
        text(&mut agent, &realm, "log.join()"),
        "get 0,after the call,get 1,get 2"
    );
    assert_eq!(state_of(&mut agent, &realm, &promise), "fulfilled [a,b,c]");
    assert_eq!(text(&mut agent, &realm, "settledValue.length"), "3");
}

/// The mapper is called with the value and k and `thisArg` as its this value, and its result
/// is awaited before the next value is read: in the branch of an iterable and in the branch of
/// an array-like.
#[test]
fn from_async_awaits_the_result_of_its_mapper() {
    let (mut agent, realm) = new_agent(limits());
    let promise = run(
        &mut agent,
        &realm,
        "var resolveM, calls = [];\n\
         Array.fromAsync([1, 2], function (v, k) { calls.push(v + ':' + k);\n\
           return k === 0 ? new Promise(function (r) { resolveM = r; }) : Promise.resolve(this.base + v * 10 + k); },\n\
           { base: 100 });",
    );
    normal(&mut agent);
    assert_eq!(text(&mut agent, &realm, "calls.join()"), "1:0");
    assert_eq!(state_of(&mut agent, &realm, &promise), "pending");
    run(&mut agent, &realm, "resolveM('first');");
    normal(&mut agent);
    assert_eq!(text(&mut agent, &realm, "calls.join()"), "1:0,2:1");
    assert_eq!(
        state_of(&mut agent, &realm, &promise),
        "fulfilled [first,121]"
    );
    let array_like = run(
        &mut agent,
        &realm,
        "Array.fromAsync({ length: 2, 0: 5, 1: Promise.resolve(6) },\n\
           function (v, k) { return Promise.resolve(v + k); });",
    );
    normal(&mut agent);
    assert_eq!(state_of(&mut agent, &realm, &array_like), "fulfilled [5,7]");
}

/// IfAbruptCloseAsyncIterator (`#sec-ifabruptcloseasynciterator`): a mapper that throws, and one
/// whose result is a rejected promise, reject the promise of the call with that reason after the
/// iterator's `return` was called once; a throw of `next` rejects it with no call of `return`.
#[test]
fn from_async_closes_the_iterator_when_the_mapper_throws() {
    let (mut agent, realm) = new_agent(limits());
    run(&mut agent, &realm, COUNTING);
    for (mapper, reason) in [
        (
            "function (v) { if (v === 1) throw new RangeError('mapper'); return v; }",
            "RangeError",
        ),
        (
            "function (v) { return v === 1 ? Promise.reject(new SyntaxError('mapper')) : v; }",
            "SyntaxError",
        ),
    ] {
        let promise = run(
            &mut agent,
            &realm,
            &format!("log = []; Array.fromAsync(counting(Infinity), {mapper});"),
        );
        normal(&mut agent);
        assert_eq!(
            state_of(&mut agent, &realm, &promise),
            format!("rejected {reason}")
        );
        assert_eq!(
            text(&mut agent, &realm, "log.join()"),
            "next 0,next 1,return"
        );
    }
    let promise = run(
        &mut agent,
        &realm,
        "log = []; var source = counting(Infinity);\n\
         var iterator = source[Symbol.asyncIterator]();\n\
         iterator.next = function () { log.push('next'); throw new EvalError('next'); };\n\
         source[Symbol.asyncIterator] = function () { return iterator; };\n\
         Array.fromAsync(source);",
    );
    assert_eq!(state_of(&mut agent, &realm, &promise), "rejected EvalError");
    normal(&mut agent);
    assert_eq!(text(&mut agent, &realm, "log.join()"), "next");
}

/// Steps 3.a and 3.b run in the call, before any read of the input: a mapper that is not callable
/// makes the call answer a promise rejected with a TypeError at once, and no getter of the input
/// runs.
#[test]
fn a_mapper_that_is_not_callable_rejects_without_reading_the_input() {
    let (mut agent, realm) = new_agent(limits());
    let promise = run(
        &mut agent,
        &realm,
        "var read = [], source = {};\n\
         Object.defineProperty(source, Symbol.asyncIterator, { get: function () { read.push('async'); } });\n\
         Object.defineProperty(source, Symbol.iterator, { get: function () { read.push('sync'); } });\n\
         Object.defineProperty(source, 'length', { get: function () { read.push('length'); return 0; } });\n\
         Array.fromAsync(source, {});",
    );
    assert_eq!(state_of(&mut agent, &realm, &promise), "rejected TypeError");
    assert_eq!(normal(&mut agent), report(0, 0));
    assert_eq!(text(&mut agent, &realm, "read.length"), "0");
}

/// The values of [`from_async_over_1000_values_takes_no_depth`]: 1,000, the story's count; 20
/// with the feature `collect-every-safe-point`, where every safe point collects the whole heap
/// (100 took 1.6 CPU seconds there), still more turns than both depth bounds of the test.
const VALUES: u32 = if cfg!(feature = "collect-every-safe-point") {
    20
} else {
    1_000
};

/// "A built-in async function", item 2: a continuation takes no frame and no re-entry of its
/// own, so a list of [`VALUES`] values, each awaited with its mapped result, runs to its end
/// under a `frame_depth` of 16 and a `reentry_depth` of 8.
#[test]
fn from_async_over_1000_values_takes_no_depth() {
    let mut limits = limits();
    limits.frame_depth = 16;
    limits.reentry_depth = 8;
    let (mut agent, realm) = new_agent(limits);
    let promise = run(
        &mut agent,
        &realm,
        &format!(
            "var list = [];\n\
             for (var i = 0; i < {VALUES}; i++) list.push(i);\n\
             Array.fromAsync(list, function (v) {{ return Promise.resolve(v * 2); }});"
        ),
    );
    let ran = normal(&mut agent);
    println!("the checkpoint over {VALUES} values: {ran:?}");
    assert!(ran.promise_jobs >= u64::from(VALUES) * 2, "{ran:?}");
    let shown = state_of(&mut agent, &realm, &promise);
    assert!(shown.starts_with("fulfilled ["), "{shown}");
    assert_eq!(
        text(
            &mut agent,
            &realm,
            "settledValue.length + ' ' + settledValue[settledValue.length - 1]"
        ),
        format!("{VALUES} {}", (VALUES - 1) * 2)
    );
}

/// The bytes of the box of an activation's state (`object/async_builtin.rs`, the box of
/// `AsyncBuiltinSteps`, which the library's test `the_box_of_an_activation_keeps_its_size`
/// reads), on this target.
const ACTIVATION_BOX_BYTES: u64 = 112;

/// The bytes the slots of a function of an Await are charged (`Slots::Await`, read by the
/// library's test `the_slots_of_a_closure_are_charged_their_box`).
const AWAIT_SLOTS_BYTES: u64 = 32;

/// The bytes of one reaction in the lists of a pending promise (`object/promise.rs`,
/// `REACTION_BYTES`, read by the library's test `a_reaction_is_charged_its_size`).
const REACTION_BYTES: u64 = 112;

/// "A built-in async function", item 1: while `Array.fromAsync` waits on a `next` that never
/// settles, `heap_bytes()` holds at least the sum of these terms, each named by its constant:
///
/// - the box of the activation's state, [`ACTIVATION_BOX_BYTES`];
/// - the slots of the two functions of the Await, 2 × [`AWAIT_SLOTS_BYTES`];
/// - the two reactions in the awaited promise's lists, 2 × [`REACTION_BYTES`].
///
/// The records of the activation, of the array, of the two functions and of the promise and its
/// resolving functions take slots of the object arena, which `heap_bytes()` counts by capacity:
/// the readings are those of the second of two rounds, whose records reuse the slots the first
/// round freed, so the floor holds only what the records own, and the test reads
/// `heap_bytes()`, the number `memory_bytes` bounds (`live_bytes()`, which counts each slot, is
/// printed beside it). One `collect()` after the script drops the input (the source, its
/// iterator and the promise `next` answered) gives at least that sum back, while the script
/// still holds the promise of the call, which stays pending.
#[test]
fn a_pending_from_async_is_charged_until_its_activation_is_freed() {
    let (mut agent, realm) = new_agent(limits());
    run(&mut agent, &realm, "var never, source, p; 0;");
    let make = parse(
        &mut agent,
        &realm,
        "never = new Promise(function () {});\n\
         source = {};\n\
         source[Symbol.asyncIterator] = function () { return { next: function () { return never; } }; };\n\
         p = Array.fromAsync(source); 0;",
    );
    let drop_input = parse(&mut agent, &realm, "never = null; source = null; 0;");
    let floor = ACTIVATION_BOX_BYTES + 2 * AWAIT_SLOTS_BYTES + 2 * REACTION_BYTES;
    let mut readings = Vec::new();
    for _round in 0..2 {
        let before = collected(&mut agent);
        let live_before = agent.live_bytes().unwrap();
        run_parsed(&mut agent, &make);
        let held = collected(&mut agent);
        let live_held = agent.live_bytes().unwrap();
        run_parsed(&mut agent, &drop_input);
        let freed = collected(&mut agent);
        let live_freed = agent.live_bytes().unwrap();
        readings.push((before, held, freed, live_before, live_held, live_freed));
    }
    println!(
        "floor {floor} = box {ACTIVATION_BOX_BYTES} + 2 x await slots {AWAIT_SLOTS_BYTES} + \
         2 x reaction {REACTION_BYTES}; per round (heap_bytes before the call, while it waits, \
         after collect; live_bytes the same): {readings:?}"
    );
    let (before, held, freed, ..) = readings[1];
    assert!(held.saturating_sub(before) >= floor, "{readings:?}");
    assert!(held.saturating_sub(freed) >= floor, "{readings:?}");
    assert_eq!(state(&mut agent, &realm, "p"), "pending");
}

/// The step budget of the termination tests: room for their other scripts, and few enough
/// steps that the loop which spends it stays cheap with a collection at every safe point
/// (1,000 took 0.17 s there; 300 with the feature `collect-every-safe-point`).
const STEP_BUDGET: u64 = if cfg!(feature = "collect-every-safe-point") {
    300
} else {
    1_000
};

/// The room of the memory budget tests above the reading of a fresh realm: room for their other
/// scripts, and little enough that the Strings which fill it are made quickly (1 MiB took 65 ms).
const FILL_ROOM: u64 = 256 * 1024;

/// Statements that spend any room in a few steps and safe points: Strings that double, each
/// kept in a local of the function they run in, until the charge of one is refused.
const FILL: &str = "var a = []; for (var n = 1024; ; n = n * 2) a.push('x'.repeat(n));";

/// An agent of this file whose memory budget is the reading of `heap_bytes()` of a fresh realm,
/// measured in an agent of its own, plus `room`, with `steps_per_entry` `steps`.
fn memory_agent(room: u64, steps: u64) -> (Agent<Scripted>, Realm) {
    let realm_bytes = {
        let (agent, _realm) = new_agent(limits());
        agent.heap_bytes().unwrap()
    };
    let mut limits = limits_with(steps);
    limits.memory_bytes = realm_bytes + room;
    new_agent(limits)
}

/// An agent for each termination, and the statements that end in it.
fn ending_agents() -> [(Agent<Scripted>, Realm, &'static str, Termination); 3] {
    let (step_agent, step_realm) = new_agent(limits_with(STEP_BUDGET));
    let (memory_agent, memory_realm) = memory_agent(FILL_ROOM, limits().steps_per_entry);
    let (interrupt_agent, interrupt_realm) = new_agent(limits());
    [
        (
            step_agent,
            step_realm,
            "for (;;) {}",
            Termination::StepBudget,
        ),
        (memory_agent, memory_realm, FILL, Termination::MemoryBudget),
        (
            interrupt_agent,
            interrupt_realm,
            "interruptNow(); for (;;) {}",
            Termination::Interrupted,
        ),
    ]
}

/// "A built-in async function", item 3, for a termination below a call its steps made into code
/// in a continuation (the mapper, at its second value): the checkpoint answers the termination,
/// the iterator is not closed, the array keeps the element defined before, the promise of the
/// call stays pending, the function of the last Await the host kept does nothing when called,
/// and a later checkpoint answers `Normal` with both counts 0.
#[test]
fn a_termination_in_the_steps_of_from_async_leaves_its_promise_pending() {
    for (mut agent, realm, ending, termination) in ending_agents() {
        run(&mut agent, &realm, COUNTING);
        let promise = run(
            &mut agent,
            &realm,
            &format!(
                "var made;\n\
                 function C() {{ made = this; }}\n\
                 Array.fromAsync.call(C, counting(Infinity), function (v) {{ if (v === 1) {{ {ending} }} return v; }});"
            ),
        );
        assert_eq!(terminated(&mut agent), termination);
        assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
        assert_eq!(
            text(
                &mut agent,
                &realm,
                "log.join() + ' ' + made[0] + ' ' + ('1' in made)"
            ),
            "next 0,next 1 0 false",
            "{termination:?}"
        );
        assert_eq!(state_of(&mut agent, &realm, &promise), "pending");
        let last = agent.host().state::<FromAsyncHost>().kept.len() - 2;
        let on_fulfilled = kept(&mut agent, last);
        assert!(call_answers_undefined(
            &mut agent,
            &realm,
            &on_fulfilled,
            Value::Number(1.0)
        ));
        assert_eq!(normal(&mut agent), report(0, 0), "{termination:?}");
        assert_eq!(text(&mut agent, &realm, "log.join()"), "next 0,next 1");
        assert_eq!(state_of(&mut agent, &realm, &promise), "pending");
    }
}

/// Item 3, for a call suspended at an Await when an entry ends: the element is a promise of the
/// script; a script that ends in the termination leaves the call waiting; once the script
/// resolves that promise, a checkpoint continues it and fulfils its promise with the array.
#[test]
fn a_termination_while_from_async_waits_keeps_it_and_a_later_checkpoint_continues_it() {
    for (mut agent, realm, ending, termination) in ending_agents() {
        let promise = run(
            &mut agent,
            &realm,
            "var res;\n\
             Array.fromAsync({ length: 1, 0: new Promise(function (r) { res = r; }) });",
        );
        assert_eq!(normal(&mut agent), report(0, 0));
        let outcome = evaluate(
            &mut agent,
            &realm,
            &format!("(function () {{ {ending} }})();"),
        );
        assert!(
            matches!(outcome, Outcome::Terminated(t) if t == termination),
            "{outcome:?}"
        );
        assert_eq!(state_of(&mut agent, &realm, &promise), "pending");
        run(&mut agent, &realm, "res(3);");
        assert_eq!(normal(&mut agent), report(1, 0), "{termination:?}");
        assert_eq!(state_of(&mut agent, &realm, &promise), "fulfilled [3]");
    }
}

/// The `reentry_depth` of the depth test of re-entries.
const REENTRY: u32 = 16;

/// The async iterable of the depth tests, `source`, whose `next` answers `0`, `1`, then done.
const SOURCE: &str = "var source = {}, p;\n\
     source[Symbol.asyncIterator] = function () { return { i: 0, next: function () {\n\
       return { value: this.i++, done: this.i > 2 }; } }; };";

/// The script of [`from_async_at_reentry_depth_rejects_with_a_range_error`]: `f(k)` recurses
/// through the host function `reenter` `k` times, then calls `Array.fromAsync` of an async
/// iterable whose `Symbol.asyncIterator` method and `next` are functions of the script. `next`
/// answers a plain result, which the Await wraps: a call of a builtin there (`Promise.resolve`,
/// which constructs a promise and calls its executor) would add activations of its own and move
/// the level at which the bound is reached.
fn reentry_script(levels: u32) -> String {
    format!(
        "{SOURCE}\n\
         function f(k) {{ if (k === 0) {{ p = Array.fromAsync(source); return; }}\n\
           reenter(function () {{ f(k - 1); }}); }}\n\
         f({levels}); p;"
    )
}

/// The state of the promise of the call of [`reentry_script`] after a checkpoint, in an agent of
/// the stack tests' host (`support::stack::stack_agent`).
fn reentry_state(levels: u32) -> String {
    let (mut agent, realm) = stack_agent(REENTRY);
    assert!(matches!(
        eval(&mut agent, &realm, &reentry_script(levels)),
        Outcome::Normal(_)
    ));
    assert!(matches!(
        agent.perform_microtask_checkpoint(),
        Ok(Outcome::Normal(_))
    ));
    match eval_text(
        &mut agent,
        &realm,
        "var shown = 'pending';\n\
         p.then(function (a) { shown = 'fulfilled ' + a.join(); }, function (e) { shown = 'rejected ' + e.name; });\n\
         0;",
    ) {
        Shown::Normal(_) => {}
        other => panic!("{other:?}"),
    }
    assert!(matches!(
        agent.perform_microtask_checkpoint(),
        Ok(Outcome::Normal(_))
    ));
    match eval_text(&mut agent, &realm, "shown;") {
        Shown::Normal(shown) => shown.trim_matches('"').to_string(),
        other => panic!("{other:?}"),
    }
}

/// Item 2: a call the steps make into code is a re-entry counted against `reentry_depth`. With
/// the script [`REENTRY`] re-entries deep, the call of `Array.fromAsync` is the last the bound
/// lets run and its call of the `Symbol.asyncIterator` method one more: that `RangeError` is a
/// throw completion of the steps, which reject the promise of the call with it. One level
/// shallower the promise is fulfilled after the checkpoint.
#[test]
fn from_async_at_reentry_depth_rejects_with_a_range_error() {
    assert_eq!(reentry_state(REENTRY), "rejected RangeError");
    assert_eq!(reentry_state(REENTRY - 1), "fulfilled 0,1");
}

/// The `frame_depth` of the depth test of frames.
const FRAME_DEPTH: u32 = 16;

/// Item 2 for `frame_depth`: `g(k)` recurses `k` calls deep (with the script's own frame, k + 2
/// frames at the deepest), then calls `Array.fromAsync` of the async iterable [`SOURCE`]; at k = `FRAME_DEPTH` - 2 the frame of the `Symbol.asyncIterator` method
/// would be one past `frame_depth`, and the promise of the call is rejected with that
/// `RangeError`; one level shallower it is fulfilled after the checkpoint.
#[test]
fn from_async_at_frame_depth_rejects_with_a_range_error() {
    for (levels, expected) in [
        (FRAME_DEPTH - 2, "rejected RangeError"),
        (FRAME_DEPTH - 3, "fulfilled [0,1]"),
    ] {
        let mut limits = limits();
        limits.frame_depth = FRAME_DEPTH;
        let (mut agent, realm) = new_agent(limits);
        let promise = run(
            &mut agent,
            &realm,
            &format!(
                "{SOURCE}\n\
                 function g(k) {{ if (k === 0) {{ p = Array.fromAsync(source); return; }} g(k - 1); }}\n\
                 g({levels}); p;"
            ),
        );
        normal(&mut agent);
        assert_eq!(
            state_of(&mut agent, &realm, &promise),
            expected,
            "{levels} levels"
        );
    }
}

/// The step budget of the endless array-like: 3,000; 300 with the feature
/// `collect-every-safe-point`.
const ENDLESS_STEPS: u64 = if cfg!(feature = "collect-every-safe-point") {
    300
} else {
    3_000
};

/// The memory room of the endless array-like above the reading its script leaves: 64 KiB; 32
/// KiB with the feature `collect-every-safe-point`.
const ENDLESS_ROOM: u64 = if cfg!(feature = "collect-every-safe-point") {
    32 * 1024
} else {
    64 * 1024
};

/// The script of the endless array-like: `{ length: 2 ** 53 - 1 }`, built by a constructor of
/// the script (`Construct(%Array%, « 2 ** 53 - 1 »)` and `ArrayCreate` throw a `RangeError`
/// for that length), which keeps the object it builds in `made`.
const ENDLESS: &str = "var made;\n\
     function C() { made = this; }\n\
     Array.fromAsync.call(C, { length: 9007199254740991 });";

/// Item 2: every turn of the loop over an array-like awaits, so one of length 2^53 - 1 ends at a
/// budget: with `steps_per_entry` [`ENDLESS_STEPS`] the checkpoint answers
/// `Terminated(StepBudget)` and `last_entry_steps()` reads at most that budget; with a memory
/// budget [`ENDLESS_ROOM`] above what the script leaves and no step budget to speak of, the
/// object it fills reaches the memory budget first: the script answers, then the checkpoint
/// answers `Terminated(MemoryBudget)`, with `heap_bytes()` above its reading before the
/// checkpoint. The test prints the readings. (After the memory budget is reached no script can
/// count the elements: its parse is refused its charge.)
///
/// How the memory budget is found, from a reading the test makes itself: a twin agent with the
/// test limits and the same step budget runs the script, and its `heap_bytes()` after the
/// script is the reading; the agent under test has `memory_bytes` that reading plus
/// [`ENDLESS_ROOM`]. Until a charge is refused, a budget changes nothing an agent does (a
/// collection is due by the bytes since the last one, not by the budget), so the agent under
/// test leaves the same reading, which the test asserts, and the room is the room of the
/// checkpoint whatever the script needed. The budget is not the realm's reading plus the room:
/// the script's first objects may grow the arena of objects, by 54,272 bytes at 256 slots, and
/// whether they do moves with every object a realm gains. Measured on 2026-11-24 with the
/// collector at every safe point: on the realm's charge of 214,160 bytes plus 32 KiB the script
/// answered at 223,135 and the checkpoint was refused that growth at its fourteenth step; with
/// the 11 functions of `story:math-constants-and-exact-functions` (219,907) the script itself
/// needed the growth, was refused it with 24,501 bytes left, and answered
/// `Terminated(MemoryBudget)` before the checkpoint.
#[test]
fn from_async_of_an_endless_array_like_ends_at_a_budget() {
    let (mut agent, realm) = new_agent(limits_with(ENDLESS_STEPS));
    let promise = run(&mut agent, &realm, ENDLESS);
    assert_eq!(terminated(&mut agent), Termination::StepBudget);
    let steps = agent.last_entry_steps().unwrap();
    let defined = text(&mut agent, &realm, "Object.keys(made).length");
    println!("steps_per_entry {ENDLESS_STEPS}: last_entry_steps {steps}, {defined} elements");
    assert!(steps <= ENDLESS_STEPS, "{steps}");
    assert!(defined.parse::<u64>().unwrap() > 0, "{defined}");
    assert_eq!(state_of(&mut agent, &realm, &promise), "pending");

    let reading = {
        let (mut twin, realm) = new_agent(limits_with(1 << 40));
        run(&mut twin, &realm, ENDLESS);
        twin.heap_bytes().unwrap()
    };
    let mut memory = limits_with(1 << 40);
    memory.memory_bytes = reading + ENDLESS_ROOM;
    let (mut agent, realm) = new_agent(memory);
    run(&mut agent, &realm, ENDLESS);
    let before = agent.heap_bytes().unwrap();
    assert_eq!(before, reading, "the twin's reading after the script");
    assert_eq!(terminated(&mut agent), Termination::MemoryBudget);
    let after = agent.heap_bytes().unwrap();
    let steps = agent.last_entry_steps().unwrap();
    println!(
        "memory room {ENDLESS_ROOM}: heap_bytes {before} before the checkpoint, {after} at the \
         termination, last_entry_steps {steps}"
    );
    assert!(after > before, "{before} {after}");
}

/// HOST-207 (`docs/design/host-interface.md`): an Await of a built-in async function goes
/// through the hooks of an `await`: the script `A.fromAsync({ length: 1, 0: 1 })`, run in the
/// second of two realms with the global `A` holding the first realm's `Array`, calls
/// `host_make_job_callback` twice, with two distinct functions, and the checkpoint after it runs
/// one promise job, for which the engine calls `host_prepare_to_run_job` with the first realm
/// (the builtin's), then `host_call_job_callback` once, with the first of those two functions
/// and the value `host_make_job_callback` returned for it, then `host_clean_up_after_job` with
/// `Ok(())`; the promise is then fulfilled with an array of the first realm whose element 0 is
/// 1, and `last_entry_steps()` of the checkpoint reads at least 3.
#[test]
fn host_207_an_await_of_from_async_goes_through_the_hooks_of_an_await() {
    let (mut agent, first) = new_agent(limits());
    let second = new_realm(&mut agent);
    let array = run(&mut agent, &first, "Array;");
    set_global(&mut agent, &second, "A", &array);
    let promise = run(&mut agent, &second, "A.fromAsync({ length: 1, 0: 1 })");
    assert_eq!(agent.host().state::<FromAsyncHost>().made, 2);
    let one = kept(&mut agent, 0);
    let two = kept(&mut agent, 1);
    let distinct = agent
        .with_cx(&first, |cx| cx.get(&one).unwrap() != cx.get(&two).unwrap())
        .unwrap();
    assert!(distinct, "the two functions of the Await are one");
    assert!(agent.host().state::<FromAsyncHost>().events.is_empty());
    assert_eq!(normal(&mut agent), report(1, 0));
    assert_eq!(
        agent.host().state::<FromAsyncHost>().events,
        ["prepare", "call 1 marker 1", "clean-up ok"]
    );
    let first_ref = agent
        .with_cx(&first, |cx| cx.realm_ref(&first).unwrap())
        .unwrap();
    assert_eq!(agent.host().state::<FromAsyncHost>().prepares, [first_ref]);
    let steps = agent.last_entry_steps().unwrap();
    println!("last_entry_steps of the checkpoint: {steps}");
    assert!(steps >= 3, "{steps}");
    assert_eq!(state_of(&mut agent, &first, &promise), "fulfilled [1]");
    assert_eq!(
        text(
            &mut agent,
            &first,
            "Object.getPrototypeOf(settledValue) === Array.prototype"
        ),
        "true"
    );
    set_global(&mut agent, &second, "settledValue", &promise);
    assert_eq!(
        text(
            &mut agent,
            &second,
            "Object.getPrototypeOf(settledValue) === Promise.prototype"
        ),
        "false"
    );
}

/// HOST-208 (`docs/design/host-interface.md`): a built-in async function cut by a termination
/// leaves its promise pending for good: the script `var p = Array.fromAsync({ length: 2, 0: 0,
/// get 1() { for (;;) {} } }); p` answers `Normal` with the promise p; the next checkpoint
/// answers `Terminated(StepBudget)`; `promise_state` of p reads `Pending` after it and after a
/// later checkpoint, which answers `Normal` with both counts 0. And the script `var q =
/// Array.fromAsync({ length: 1, 0: new Promise(function (r) { globalThis.res = r; }) }); q`,
/// then the script `for (;;) {}`, which ends in `Terminated(StepBudget)`, then the script
/// `res(3)` and a checkpoint fulfil q with an array whose element 0 is 3.
#[test]
fn host_208_a_from_async_cut_by_a_termination_leaves_its_promise_pending_for_good() {
    let (mut agent, realm) = new_agent(limits_with(STEP_BUDGET));
    let p = run(
        &mut agent,
        &realm,
        "var p = Array.fromAsync({ length: 2, 0: 0, get 1() { for (;;) {} } }); p",
    );
    assert_eq!(terminated(&mut agent), Termination::StepBudget);
    assert!(matches!(agent.promise_state(&p), Ok(PromiseState::Pending)));
    assert_eq!(normal(&mut agent), report(0, 0));
    assert!(matches!(agent.promise_state(&p), Ok(PromiseState::Pending)));

    let q = run(
        &mut agent,
        &realm,
        "var q = Array.fromAsync({ length: 1, 0: new Promise(function (r) { globalThis.res = r; }) }); q",
    );
    assert!(matches!(
        evaluate(&mut agent, &realm, "for (;;) {}"),
        Outcome::Terminated(Termination::StepBudget)
    ));
    run(&mut agent, &realm, "res(3)");
    normal(&mut agent);
    assert_eq!(state_of(&mut agent, &realm, &q), "fulfilled [3]");
}
