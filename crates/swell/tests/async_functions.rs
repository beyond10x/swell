//! `async` functions on the suspended frames of slice 7 (`story:async-functions-and-await`;
//! `docs/design/engine.md`, decision 3, "`async` functions on suspended frames", items 1 to 5;
//! `docs/design/host-interface.md`, section 10 and decision 11, "The hooks of an `await`"): the
//! shape of the first consumer's `fetch_json`, the call that runs to its first `await`, the job
//! that resumes it, the rejection of the promise of the call, the charge of a suspended frame, the
//! steps and depth bounds of a resumption, the terminations, and HOST-194 and HOST-195. Run also
//! with the feature `collect-every-safe-point`.
//!
//! The public interface has no read of a promise's state (`promise_state` comes with
//! `story:promise-rejection-tracker-and-promise-reads`), so a test reads it through a `then` of a
//! later script ([`state_of`]): a pending promise queues no job and leaves the probe at
//! `pending`. A test that reads `heap_bytes()` around an entry parses that entry's script before
//! the first reading and declares the global names it assigns in an earlier script, as
//! `tests/generators.rs` does.
#![cfg(test)]

use crate::support;

use std::any::Any;
use std::rc::Rc;

use support::host::{TestHost, limits, seeds, stack_with_room};
use support::script::{Outcome as ScriptOutcome, define_function, eval, eval_text};
use support::stack::{BUILTINS_REENTRY, on_bounded_stack, stack_agent, stack_limits, test_path};
use swell::{
    Agent, Brand, CheckpointReport, Completion, Cx, HookCx, Host, HostCall, HostData, HostDefined,
    HostObjectSpec, InterruptHandle, Job, Limits, LoadRequest, ObjectRef, Outcome, Parsed,
    PropertyKey, Prototype, Realm, RealmRef, Root, Script, Source, Termination, Tracer, Value,
};

/// A [[HostDefined]] that holds the number of the `host_make_job_callback` call that made it.
struct Marker(u32);

impl HostData for Marker {
    fn trace(&self, _tracer: &mut Tracer<'_>) {}
}

/// The host of these tests: the embedder of the first consumer. Its host functions make
/// promises with `Cx::new_promise_capability` and root their resolve functions; its job hooks
/// record what they receive, in order.
#[derive(Default)]
struct AsyncHost {
    /// The resolve functions of the capabilities the host functions made, rooted, in order.
    resolves: Vec<Root>,
    /// The promises of those capabilities, rooted when `root_promises` is set.
    promises: Vec<Root>,
    root_promises: bool,
    /// Whether `host_make_job_callback` roots each function it receives in `kept`.
    keep: bool,
    kept: Vec<Root>,
    /// How often `host_make_job_callback` was called.
    made: u32,
    /// What the job hooks received, in order: `prepare`, `call <k> marker <m>` (the function
    /// the k-th `host_make_job_callback` kept, 0 for one it did not keep, and the marker made by
    /// the m-th call), and `clean-up ok` or `clean-up <kind> <message>`.
    events: Vec<String>,
    /// The realm of each `host_prepare_to_run_job`.
    prepares: Vec<RealmRef>,
    /// What the host functions `note`, `probe` and `checkpointHere` recorded.
    notes: Vec<String>,
    /// The agent's interrupt handle, for the host function `interruptNow`.
    interrupt: Option<InterruptHandle>,
}

impl Host for AsyncHost {
    fn host_enqueue_generic_job(_cx: &mut HookCx<'_, Self>, _job: Job) {}
    fn host_enqueue_timeout_job(_cx: &mut HookCx<'_, Self>, _job: Job, _milliseconds: f64) {}
    fn host_enqueue_finalization_registry_cleanup_job(_cx: &mut HookCx<'_, Self>, _job: Job) {}
    fn host_load_imported_module(_cx: &mut HookCx<'_, Self>, _request: LoadRequest) {}
    fn host_system_utc_epoch_nanoseconds(_cx: &mut HookCx<'_, Self>, _global: ObjectRef) -> i128 {
        0
    }
    fn host_make_job_callback(
        cx: &mut HookCx<'_, Self>,
        callback: ObjectRef,
    ) -> Option<HostDefined> {
        cx.host_mut().made += 1;
        if cx.host().keep {
            let root = cx.root(Value::Object(callback)).unwrap();
            cx.host_mut().kept.push(root);
        }
        Some(Rc::new(Marker(cx.host().made)))
    }
    fn host_call_job_callback(
        cx: &mut Cx<'_, Self>,
        callback: ObjectRef,
        host_defined: Option<&HostDefined>,
        this: Value,
        args: &[Value],
    ) -> Completion<Value> {
        let marker = host_defined
            .and_then(|data| (&**data as &dyn Any).downcast_ref::<Marker>())
            .map_or(0, |Marker(n)| *n);
        let kept = std::mem::take(&mut cx.host_mut().kept);
        let which = kept
            .iter()
            .position(|root| cx.get(root).unwrap() == Value::Object(callback))
            .map_or(0, |k| k + 1);
        cx.host_mut().kept = kept;
        cx.host_mut()
            .events
            .push(format!("call {which} marker {marker}"));
        cx.call(Value::Object(callback), this, args).unwrap()
    }
    fn host_prepare_to_run_job(cx: &mut HookCx<'_, Self>, realm: RealmRef) {
        cx.host_mut().events.push("prepare".to_string());
        cx.host_mut().prepares.push(realm);
    }
    fn host_clean_up_after_job(
        cx: &mut HookCx<'_, Self>,
        _realm: Option<RealmRef>,
        result: Result<(), Value>,
    ) {
        let event = match result {
            Ok(()) => "clean-up ok".to_string(),
            Err(value) => match cx.error_summary(value).unwrap() {
                Some(summary) => format!(
                    "clean-up {} {}",
                    summary.kind.name(),
                    String::from_utf16_lossy(&summary.message.unwrap_or_default())
                ),
                None => "clean-up other".to_string(),
            },
        };
        cx.host_mut().events.push(event);
    }
}

/// A capability of `Cx::new_promise_capability`, its resolve function rooted by the host (and
/// its promise, when the host roots promises): the promise.
fn host_promise(cx: &mut Cx<'_, AsyncHost>) -> Completion<Value> {
    let capability = cx.new_promise_capability()?;
    let resolve = cx.root(capability.resolve).unwrap();
    cx.host_mut().resolves.push(resolve);
    if cx.host().root_promises {
        let promise = cx.root(Value::Object(capability.promise)).unwrap();
        cx.host_mut().promises.push(promise);
    }
    Ok(Value::Object(capability.promise))
}

/// The host function `fetch` of the consumer: a promise the host resolves later.
fn fetch(cx: &mut Cx<'_, AsyncHost>, _call: HostCall<'_>) -> Completion<Value> {
    host_promise(cx)
}

/// The method `json` of the consumer's response object: a promise the host resolves later.
fn json(cx: &mut Cx<'_, AsyncHost>, _call: HostCall<'_>) -> Completion<Value> {
    host_promise(cx)
}

/// The host function `note`: it records its first argument, a String as its text.
fn note(cx: &mut Cx<'_, AsyncHost>, call: HostCall<'_>) -> Completion<Value> {
    let line = match call.args.first().copied() {
        Some(Value::String(string)) => cx.to_rust_string_lossy(string).unwrap(),
        other => format!("{other:?}"),
    };
    cx.host_mut().notes.push(line);
    Ok(Value::Undefined)
}

/// The host function `probe`: it records that it ran.
fn probe(cx: &mut Cx<'_, AsyncHost>, _call: HostCall<'_>) -> Completion<Value> {
    cx.host_mut().notes.push("probe".to_string());
    Ok(Value::Undefined)
}

/// The host function `checkpointHere`: a checkpoint through `cx`, inside the running entry.
fn checkpoint_here(cx: &mut Cx<'_, AsyncHost>, _call: HostCall<'_>) -> Completion<Value> {
    let report = cx.perform_microtask_checkpoint()?;
    cx.host_mut()
        .notes
        .push(format!("checkpoint {}", report.promise_jobs));
    Ok(Value::Undefined)
}

/// The host function `interruptNow`: it sets the agent's interrupt flag.
fn interrupt_now(cx: &mut Cx<'_, AsyncHost>, _call: HostCall<'_>) -> Completion<Value> {
    if let Some(interrupt) = &cx.host().interrupt {
        interrupt.interrupt();
    }
    Ok(Value::Undefined)
}

/// An agent of the host of these tests with `limits`, and a realm whose globals `fetch`,
/// `note`, `probe`, `checkpointHere` and `interruptNow` are the host functions of this file.
fn new_agent(limits: Limits) -> (Agent<AsyncHost>, Realm) {
    let mut agent = Agent::new(AsyncHost::default(), limits, seeds()).unwrap();
    agent.host_mut().interrupt = Some(agent.interrupt_handle());
    let realm = match agent.create_realm(None, None).unwrap() {
        Outcome::Normal(realm) => realm,
        other => panic!("create_realm answered {other:?}"),
    };
    define(&mut agent, &realm, "fetch", fetch);
    define(&mut agent, &realm, "note", note);
    define(&mut agent, &realm, "probe", probe);
    define(&mut agent, &realm, "checkpointHere", checkpoint_here);
    define(&mut agent, &realm, "interruptNow", interrupt_now);
    (agent, realm)
}

/// The test limits with `steps_per_entry` `steps`.
fn limits_with(steps: u64) -> Limits {
    let mut limits = limits();
    limits.steps_per_entry = steps;
    limits
}

/// Define the host function `steps` as the global `name` of `realm`.
fn define(
    agent: &mut Agent<AsyncHost>,
    realm: &Realm,
    name: &str,
    steps: swell::HostFn<AsyncHost>,
) {
    let global = agent.global_object(realm).unwrap();
    agent
        .with_cx(realm, |cx| {
            let home = cx.realm_ref(realm).unwrap();
            let function = cx
                .new_host_function(home, name, 0, steps, false, None)
                .unwrap()
                .unwrap();
            let Value::Object(global) = cx.get(&global).unwrap() else {
                panic!("the global object");
            };
            let key = PropertyKey::String(cx.new_string(name).unwrap());
            cx.set(global, key, Value::Object(function), true)
                .unwrap()
                .unwrap();
        })
        .unwrap();
}

/// The script `source` parsed, which must parse.
fn parse(agent: &mut Agent<AsyncHost>, realm: &Realm, source: &str) -> Script {
    match agent
        .parse_script(realm, Source::Utf8(source), None)
        .unwrap()
    {
        Parsed::Parsed(script) => script,
        Parsed::Errors(errors) => {
            let messages: Vec<String> = errors
                .iter()
                .map(|error| {
                    agent
                        .with_cx(realm, |cx| match cx.get(error).unwrap() {
                            Value::Object(object) => {
                                let key = PropertyKey::String(cx.new_string("message").unwrap());
                                match cx.get_property(object, key).unwrap().unwrap() {
                                    Value::String(s) => cx.to_rust_string_lossy(s).unwrap(),
                                    other => format!("{other:?}"),
                                }
                            }
                            other => format!("{other:?}"),
                        })
                        .unwrap()
                })
                .collect();
            panic!("{source:?} did not parse: {messages:?}");
        }
        other => panic!("{source:?} answered {other:?}"),
    }
}

/// The outcome of the script `source`, which must parse.
fn evaluate(agent: &mut Agent<AsyncHost>, realm: &Realm, source: &str) -> Outcome<Root> {
    let script = parse(agent, realm, source);
    agent.evaluate_script(&script).unwrap()
}

/// The script `source`, which must complete normally: its completion value.
fn run(agent: &mut Agent<AsyncHost>, realm: &Realm, source: &str) -> Root {
    match evaluate(agent, realm, source) {
        Outcome::Normal(value) => value,
        other => panic!("{source:?} answered {other:?}"),
    }
}

/// The parsed script `script`, which must complete normally.
fn run_parsed(agent: &mut Agent<AsyncHost>, script: &Script) {
    match agent.evaluate_script(script).unwrap() {
        Outcome::Normal(_) => {}
        other => panic!("the script answered {other:?}"),
    }
}

/// The text of `String(expression)`.
fn text(agent: &mut Agent<AsyncHost>, realm: &Realm, expression: &str) -> String {
    let root = run(agent, realm, &format!("String({expression})"));
    agent
        .with_cx(realm, |cx| match cx.get(&root).unwrap() {
            Value::String(string) => cx.to_rust_string_lossy(string).unwrap(),
            other => panic!("{other:?}"),
        })
        .unwrap()
}

/// A checkpoint that must answer `Normal`: its report.
fn normal(agent: &mut Agent<AsyncHost>) -> CheckpointReport {
    match agent.perform_microtask_checkpoint().unwrap() {
        Outcome::Normal(report) => report,
        other => panic!("the checkpoint answered {other:?}"),
    }
}

/// A checkpoint that must answer `Terminated`: its reason.
fn terminated(agent: &mut Agent<AsyncHost>) -> Termination {
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

/// The state of the promise `expression` as a later script reads it: a `then` of it, then a
/// checkpoint. `pending` (the checkpoint ran no job), `fulfilled <value>` or `rejected <name or
/// value>` (it ran the one job of that `then`).
fn state_of(agent: &mut Agent<AsyncHost>, realm: &Realm, expression: &str) -> String {
    run(
        agent,
        realm,
        &format!(
            "var stateRead = 'pending';\n\
             ({expression}).then(function (v) {{ stateRead = 'fulfilled ' + v; }},\n\
               function (e) {{ stateRead = 'rejected ' + (e instanceof Error ? e.name + ' ' + e.message : e); }});"
        ),
    );
    let checkpoint = normal(agent);
    let state = text(agent, realm, "stateRead");
    let expected = if state == "pending" {
        report(0, 0)
    } else {
        report(1, 0)
    };
    assert_eq!(
        checkpoint, expected,
        "the probe of {expression} read {state}"
    );
    state
}

/// `Agent::call` of `function` with `undefined` as `this` and the one argument `value`.
fn call(
    agent: &mut Agent<AsyncHost>,
    realm: &Realm,
    function: &Root,
    value: Value,
) -> Outcome<Root> {
    let (undefined, argument) = agent
        .with_cx(realm, |cx| {
            (cx.root(Value::Undefined).unwrap(), cx.root(value).unwrap())
        })
        .unwrap();
    agent
        .call(realm, function, &undefined, &[&argument])
        .unwrap()
}

/// Whether `outcome` is `Normal(undefined)`.
fn answers_undefined(agent: &mut Agent<AsyncHost>, realm: &Realm, outcome: Outcome<Root>) -> bool {
    match outcome {
        Outcome::Normal(value) => agent
            .with_cx(realm, |cx| cx.get(&value).unwrap() == Value::Undefined)
            .unwrap(),
        _ => false,
    }
}

/// The function `host_make_job_callback` kept `index`-th (from 0), duplicated.
fn kept(agent: &mut Agent<AsyncHost>, index: usize) -> Root {
    let root = agent.host_mut().kept.remove(index);
    let function = agent.duplicate(&root).unwrap();
    agent.host_mut().kept.insert(index, root);
    function
}

/// `Agent::call` of the resolve function the host rooted `index`-th, with `value`, which must
/// answer `Normal`.
fn resolve(agent: &mut Agent<AsyncHost>, realm: &Realm, index: usize, value: &Root) {
    let root = agent.host_mut().resolves.remove(index);
    let function = agent.duplicate(&root).unwrap();
    agent.host_mut().resolves.insert(index, root);
    let undefined = agent
        .with_cx(realm, |cx| cx.root(Value::Undefined).unwrap())
        .unwrap();
    match agent.call(realm, &function, &undefined, &[value]).unwrap() {
        Outcome::Normal(_) => {}
        other => panic!("the resolve function answered {other:?}"),
    }
}

/// `heap_bytes()` after a `collect()`.
fn collected(agent: &mut Agent<AsyncHost>) -> u64 {
    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
    agent.heap_bytes().unwrap()
}

/// The local slots of the `async` functions the tests of the charge make: item 2 of "`async`
/// functions on suspended frames" states the charge of an `await` that never ends with L = 200.
const LOCALS: usize = 200;

/// The bytes item 2 states for [`LOCALS`] local slots beyond `frame_bytes()`: 16 bytes per slot.
const FRAME_FLOOR: u64 = 16 * LOCALS as u64;

/// `var v0, ..., v199;`: [`LOCALS`] local slots of the function whose body starts with it.
fn locals() -> String {
    let names: Vec<String> = (0..LOCALS).map(|i| format!("v{i}")).collect();
    format!("var {};", names.join(", "))
}

/// The test of the first consumer's shape (`docs/plan/waves.md`, "The order after slice 6":
/// `fetch_json` of its harness): `fetch` makes a capability with `cx.new_promise_capability`,
/// roots its resolve function and returns the promise, and the response's `json` does the same.
/// The script calls `fetch_json`, which runs to its first `await` and returns its promise; the
/// host resolves the first promise with the response by `Agent::call` and runs a checkpoint,
/// which resumes the function to its second `await`; it resolves the second with 42 and runs a
/// checkpoint, which resumes the function to its end and runs the `then` callback. Every
/// checkpoint answers `Normal`, `result` reads 42 and nothing is left queued.
#[test]
fn the_consumer_shape_runs_to_its_end() {
    let (mut agent, realm) = new_agent(limits());
    let response = agent
        .with_cx(&realm, |cx| {
            let home = cx.realm_ref(&realm).unwrap();
            let object = cx
                .new_host_object(HostObjectSpec {
                    prototype: Prototype::ObjectPrototypeOf(home),
                    brand: Brand(1),
                    data: None,
                    error_kind: None,
                })
                .unwrap()
                .unwrap();
            let method = cx
                .new_host_function(home, "json", 0, json, false, None)
                .unwrap()
                .unwrap();
            let key = PropertyKey::String(cx.new_string("json").unwrap());
            cx.set(object, key, Value::Object(method), true)
                .unwrap()
                .unwrap();
            cx.root(Value::Object(object)).unwrap()
        })
        .unwrap();
    run(
        &mut agent,
        &realm,
        "var result;\n\
         async function fetch_json(resource) {\n\
           const response = await fetch(resource);\n\
           return await response.json();\n\
         }\n\
         fetch_json(\"a\").then(function (v) { result = v; });",
    );
    assert_eq!(agent.host().resolves.len(), 1, "fetch was called once");
    assert_eq!(agent.pending_microtasks(), Ok(0));
    resolve(&mut agent, &realm, 0, &response);
    assert_eq!(normal(&mut agent), report(1, 0));
    assert_eq!(agent.host().resolves.len(), 2, "json was called once");
    let answer = agent
        .with_cx(&realm, |cx| cx.root(Value::Number(42.0)).unwrap())
        .unwrap();
    resolve(&mut agent, &realm, 1, &answer);
    assert_eq!(normal(&mut agent), report(2, 0));
    assert_eq!(text(&mut agent, &realm, "result"), "42");
    assert_eq!(agent.pending_microtasks(), Ok(0));
}

/// `#sec-async-function-definitions-runtime-semantics-evaluation` and AsyncFunctionStart
/// (`#sec-async-functions-abstract-operations-async-function-start`): the call runs the body to
/// its first `await` and returns a promise; the code after the `await` has not run when the
/// call returns.
#[test]
fn an_async_function_runs_to_its_first_await_in_the_call() {
    let (mut agent, realm) = new_agent(limits());
    run(
        &mut agent,
        &realm,
        "var log = [];\n\
         async function f() { log.push(1); await null; log.push(3); }\n\
         var p = f(); log.push(2);",
    );
    assert_eq!(
        text(
            &mut agent,
            &realm,
            "p instanceof Promise && Object.getPrototypeOf(p) === Promise.prototype && log.join()"
        ),
        "1,2"
    );
    assert_eq!(normal(&mut agent), report(1, 0));
    assert_eq!(text(&mut agent, &realm, "log.join()"), "1,2,3");
    assert_eq!(state_of(&mut agent, &realm, "p"), "fulfilled undefined");
}

/// Await (`#sec-await`): the code after an `await` runs in a promise job, after a `then`
/// callback queued before it and before one queued after it.
#[test]
fn the_code_after_an_await_runs_in_a_later_job() {
    let (mut agent, realm) = new_agent(limits());
    run(
        &mut agent,
        &realm,
        "var log = [];\n\
         Promise.resolve().then(function () { log.push('before'); });\n\
         async function f() { log.push('call'); var v = await 5; log.push('after ' + v); }\n\
         f();\n\
         Promise.resolve().then(function () { log.push('later'); });\n\
         log.push('script');",
    );
    assert_eq!(text(&mut agent, &realm, "log.join()"), "call,script");
    assert_eq!(normal(&mut agent), report(3, 0));
    assert_eq!(
        text(&mut agent, &realm, "log.join()"),
        "call,script,before,after 5,later"
    );
}

/// AsyncBlockStart (`#sec-asyncblockstart`), step 2.i: a throw that leaves the body rejects the
/// promise of the call with the thrown value; the call itself returns normally.
#[test]
fn a_throw_in_the_body_rejects_the_promise() {
    let (mut agent, realm) = new_agent(limits());
    run(
        &mut agent,
        &realm,
        "var returned = false;\n\
         async function f() { throw new TypeError('in the body'); }\n\
         async function g() { await null; throw 7; }\n\
         var p = f(), q = g(); returned = true;",
    );
    assert_eq!(text(&mut agent, &realm, "returned"), "true");
    assert_eq!(normal(&mut agent), report(1, 0));
    assert_eq!(
        state_of(&mut agent, &realm, "p"),
        "rejected TypeError in the body"
    );
    assert_eq!(state_of(&mut agent, &realm, "q"), "rejected 7");
}

/// Await, steps 5 and 6 (`#sec-await`): a rejected awaited promise resumes the body with a throw
/// completion at the `await`, which a `catch` around it catches; without one it rejects the
/// promise of the call. A `finally` around it runs.
#[test]
fn a_rejected_await_throws_at_the_await_and_a_catch_around_it_catches_it() {
    let (mut agent, realm) = new_agent(limits());
    run(
        &mut agent,
        &realm,
        "var log = [];\n\
         async function f() {\n\
           try { await Promise.reject(5); log.push('not here'); }\n\
           catch (e) { log.push('caught ' + e); return e + 1; }\n\
           finally { log.push('finally'); }\n\
         }\n\
         async function g() { await Promise.reject(new RangeError('away')); }\n\
         var p = f(), q = g();",
    );
    assert_eq!(normal(&mut agent), report(2, 0));
    assert_eq!(text(&mut agent, &realm, "log.join()"), "caught 5,finally");
    assert_eq!(state_of(&mut agent, &realm, "p"), "fulfilled 6");
    assert_eq!(
        state_of(&mut agent, &realm, "q"),
        "rejected RangeError away"
    );
}

/// EvaluateAsyncFunctionBody (`#sec-runtime-semantics-evaluateasyncfunctionbody`), step 3: a
/// default of a parameter that throws rejects the promise of the call, and the call returns it
/// instead of throwing; the body does not run.
#[test]
fn a_default_that_throws_rejects_the_promise_and_the_call_returns() {
    let (mut agent, realm) = new_agent(limits());
    run(
        &mut agent,
        &realm,
        "var ran = false, returned = false;\n\
         function boom() { throw new RangeError('default'); }\n\
         async function f(a = boom()) { ran = true; }\n\
         var arrow = async (a = boom()) => { ran = true; };\n\
         var p = f(), q = arrow(); returned = p instanceof Promise && q instanceof Promise;",
    );
    assert_eq!(text(&mut agent, &realm, "returned && !ran"), "true");
    assert_eq!(
        state_of(&mut agent, &realm, "p"),
        "rejected RangeError default"
    );
    assert_eq!(
        state_of(&mut agent, &realm, "q"),
        "rejected RangeError default"
    );
}

/// `#sec-async-arrow-function-definitions-runtime-semantics-evaluation`: an `async` arrow takes
/// `this` and `arguments` from the code that made it, also after an `await` (`new.target` is
/// refused before slice 8).
#[test]
fn an_async_arrow_takes_this_and_arguments_from_its_scope() {
    let (mut agent, realm) = new_agent(limits());
    run(
        &mut agent,
        &realm,
        "var o = {}, seen = 'none';\n\
         function outer() {\n\
           var a = async () => { var before = this === o && arguments[0] === 9;\n\
             await null; return before && this === o && arguments[0] === 9; };\n\
           return a('not mine');\n\
         }\n\
         outer.call(o, 9).then(function (v) { seen = v; });",
    );
    assert_eq!(normal(&mut agent), report(2, 0));
    assert_eq!(text(&mut agent, &realm, "seen"), "true");
}

/// InstantiateAsyncFunctionObject, InstantiateAsyncFunctionExpression and the evaluation of an
/// `async` arrow (`#sec-runtime-semantics-instantiateasyncfunctionobject`,
/// `#sec-runtime-semantics-instantiateasyncfunctionexpression`): the [[Prototype]] is
/// `%AsyncFunction.prototype%`, whose [[Prototype]] is `%Function.prototype%` and whose
/// `[%Symbol.toStringTag%]` is "AsyncFunction"; there is no `prototype` property and no
/// [[Construct]], so `new` throws a `TypeError`.
#[test]
fn an_async_function_has_no_prototype_property_and_no_construct() {
    let (mut agent, realm) = new_agent(limits());
    run(
        &mut agent,
        &realm,
        "async function d() {}\n\
         var e = async function () {}, a = async () => {};\n\
         var proto = Object.getPrototypeOf(d), news = [];\n\
         for (var f of [d, e, a]) {\n\
           try { new f(); news.push('constructed'); }\n\
           catch (x) { news.push(x instanceof TypeError); }\n\
         }\n\
         var tag = Object.getOwnPropertyDescriptor(proto, Symbol.toStringTag);",
    );
    assert_eq!(
        text(
            &mut agent,
            &realm,
            "[d, e, a].every(function (f) {\n\
               return !Object.prototype.hasOwnProperty.call(f, 'prototype') &&\n\
                 Object.getPrototypeOf(f) === proto; }) &&\n\
             proto !== Function.prototype &&\n\
             Object.getPrototypeOf(proto) === Function.prototype &&\n\
             proto.constructor !== Function &&\n\
             tag.value === 'AsyncFunction' && !tag.writable && !tag.enumerable && tag.configurable &&\n\
             Object.prototype.toString.call(d) === '[object AsyncFunction]' &&\n\
             news.join()"
        ),
        "true,true,true"
    );
}

/// The script of the charge tests: the function `f`, whose frame has [`LOCALS`] local slots
/// more than `g`'s, each waiting on a promise of its own that the script makes and never
/// settles; `h` completes without an `await`; and the global names the later scripts assign.
fn charge_setup() -> String {
    format!(
        "var never = new Promise(function () {{}}), never0 = new Promise(function () {{}});\n\
         async function f() {{ {} await never; }}\n\
         async function g() {{ await never0; }}\n\
         async function h() {{}}\n\
         var p, pg, ph; 0;",
        locals()
    )
}

/// Item 2 of "`async` functions on suspended frames": with [`LOCALS`] local slots, an `await`
/// on a promise that never settles costs the sum of these terms, named here (the engine's own
/// sizes are not public; each term is read with `heap_bytes()` after a `collect()`):
///
/// - the frame's locals, [`FRAME_FLOOR`] (16 bytes per slot, beyond `frame_bytes()`);
/// - the rest of the `await`, read as the bytes of the same `await` in `g`, which has no locals
///   of its own (its reading after the call), less the bytes of the promise of a call that
///   completes at once (`h`): the activation's record, `frame_bytes()` and the frame's few
///   slots and entries, the resolving functions of the call's promise, the records and boxes of
///   the two functions of the `await`, and two reactions in the awaited promise's lists.
///
/// `heap_bytes()` after the call of `f` exceeds the reading before it by at least that sum; once
/// the script drops the only promise the `await` waits on, one `collect()` gives at least that
/// sum back, both when the script still holds the promise of the call and when it does not.
///
/// The readings compared are those of a second round. An arena is charged its capacity and
/// never shrinks, so a call that makes the arena of objects (or another) double is charged the
/// doubling, and the calls after it take slots already charged; where a doubling falls moves
/// with every object a realm gains. The first round (the three calls and the drop, then
/// `renew`, which makes the two promises again and lets everything of the round go) leaves each
/// arena at the capacity the round needs, so the second round's calls, which make the same
/// records, are charged only what they hold beyond the arenas. For example, on 2026-11-24 with
/// the collector at every safe point, a realm of four functions more than `76aa4fc`'s read g's
/// `await` as 56,184 bytes, a doubling, and the call of `f` as 5,112 bytes, less than the sum
/// 59,384 that reading gave, when the test read one round only. The second round read, at
/// three realm sizes and in both modes, the promise of a call as 0 bytes (its records take
/// slots already charged), g's `await` as 1,912 and the call of `f` as 5,112, the sum.
#[test]
fn a_frame_suspended_at_an_await_is_charged_until_its_activation_is_freed() {
    for keep_call_promise in [true, false] {
        let (mut agent, realm) = new_agent(limits());
        run(&mut agent, &realm, &charge_setup());
        let call_h = parse(&mut agent, &realm, "ph = h(); 0;");
        let call_g = parse(&mut agent, &realm, "pg = g(); 0;");
        let call_f = parse(&mut agent, &realm, "p = f(); 0;");
        let drop_it = parse(
            &mut agent,
            &realm,
            if keep_call_promise {
                "never = null; 0;"
            } else {
                "never = null; p = null; 0;"
            },
        );
        let renew = parse(
            &mut agent,
            &realm,
            "never = new Promise(function () {}); never0 = new Promise(function () {});\n\
             p = pg = ph = undefined; 0;",
        );
        let mut rounds = Vec::new();
        for _ in 0..2 {
            run_parsed(&mut agent, &renew);
            let start = collected(&mut agent);
            run_parsed(&mut agent, &call_h);
            let after_h = collected(&mut agent);
            run_parsed(&mut agent, &call_g);
            let after_g = collected(&mut agent);
            run_parsed(&mut agent, &call_f);
            let held = collected(&mut agent);
            run_parsed(&mut agent, &drop_it);
            let freed = collected(&mut agent);
            rounds.push((start, after_h, after_g, held, freed));
        }
        let (start, after_h, after_g, held, freed) = rounds[1];
        let promise = after_h.saturating_sub(start);
        let rest = after_g.saturating_sub(after_h).saturating_sub(promise);
        let sum = FRAME_FLOOR + rest;
        println!(
            "keep the call's promise {keep_call_promise}: sum {sum} = frame locals {FRAME_FLOOR} \
             + rest of the await {rest} (g's await {} less the promise of a call {promise}); \
             heap_bytes {after_g} before the call, {held} after it (+{}), {freed} after \
             collect() (-{}); (start, after h, after g, after f, after the drop) per round: \
             {rounds:?}",
            after_g.saturating_sub(after_h),
            held.saturating_sub(after_g),
            held.saturating_sub(freed)
        );
        assert!(rest > 0, "the rest of the await: {after_h} -> {after_g}");
        assert!(
            held.saturating_sub(after_g) >= sum,
            "{after_g} -> {held}, sum {sum}"
        );
        assert!(
            held.saturating_sub(freed) >= sum,
            "{held} -> {freed}, sum {sum}"
        );
    }
}

/// The same `await` on a promise the host roots (a capability of `cx.new_promise_capability`,
/// with its resolve function): a `collect()` after the script dropped everything it held gives
/// no byte back while the root holds the promise; once the host releases the roots, one gives
/// at least [`FRAME_FLOOR`] back.
#[test]
fn a_frame_waiting_on_a_promise_the_host_roots_stays_charged() {
    let (mut agent, realm) = new_agent(limits());
    agent.host_mut().root_promises = true;
    run(
        &mut agent,
        &realm,
        &format!(
            "async function f() {{ {} await fetch(); }}\nvar p; 0;",
            locals()
        ),
    );
    let call_f = parse(&mut agent, &realm, "p = f(); 0;");
    let drop_it = parse(&mut agent, &realm, "p = null; 0;");
    let before = collected(&mut agent);
    run_parsed(&mut agent, &call_f);
    run_parsed(&mut agent, &drop_it);
    let held = collected(&mut agent);
    let again = collected(&mut agent);
    agent.host_mut().promises.clear();
    agent.host_mut().resolves.clear();
    let freed = collected(&mut agent);
    println!(
        "heap_bytes: {before} before the call, {held} after it with the root (+{}), {again} \
         after another collect(), {freed} after the root is released (-{})",
        held.saturating_sub(before),
        held.saturating_sub(freed)
    );
    assert!(
        held.saturating_sub(before) >= FRAME_FLOOR,
        "{before} -> {held}"
    );
    assert_eq!(again, held, "a collect() under the root gave bytes back");
    assert!(
        held.saturating_sub(freed) >= FRAME_FLOOR,
        "{held} -> {freed}"
    );
}

/// Item 2: the frame of an activation whose call completes is credited by the return of its
/// frame (`return_from_frame`), with no `collect()`: `heap_bytes()` after the checkpoint whose
/// job completes the call is at least [`FRAME_FLOOR`] below the reading while it was
/// suspended, and the `collect()` that then frees the activation gives back less than
/// [`FRAME_FLOOR`]: the frame is credited once.
#[test]
fn a_completed_activation_holds_no_frame() {
    let (mut agent, realm) = new_agent(limits());
    run(
        &mut agent,
        &realm,
        &format!(
            "async function f() {{ {} await null; }}\nvar p = f(); 0;",
            locals()
        ),
    );
    let drop_it = parse(&mut agent, &realm, "p = null; 0;");
    let suspended = collected(&mut agent);
    assert_eq!(normal(&mut agent), report(1, 0));
    let completed = agent.heap_bytes().unwrap();
    run_parsed(&mut agent, &drop_it);
    let freed = collected(&mut agent);
    println!("heap_bytes: {suspended} suspended, {completed} completed, {freed} freed");
    assert!(
        suspended.saturating_sub(completed) >= FRAME_FLOOR,
        "{suspended} -> {completed}"
    );
    assert!(
        completed.saturating_sub(freed) < FRAME_FLOOR,
        "{completed} -> {freed}"
    );
}

/// The iterations of the longer loop of [`the_job_that_resumes_an_activation_spends_its_steps`]:
/// 1,000, the story's count; 100 with the feature `collect-every-safe-point`, where every safe
/// point of the loop collects the whole heap (as in `tests/generators.rs`).
const RESUMED_LOOP: u32 = if cfg!(feature = "collect-every-safe-point") {
    100
} else {
    1_000
};

/// Item 4: the instructions a resumed frame runs are steps of the checkpoint that runs its job,
/// whichever entry made the activation: a checkpoint that resumes a body which loops
/// [`RESUMED_LOOP`] times spends at least `RESUMED_LOOP` - 10 steps more than one that resumes a
/// body which loops 10 times.
#[test]
fn the_job_that_resumes_an_activation_spends_its_steps() {
    let (mut agent, realm) = new_agent(limits());
    run(
        &mut agent,
        &realm,
        "async function f(n) { await null; for (var i = 0; i < n; i++) {} }",
    );
    run(&mut agent, &realm, &format!("f({RESUMED_LOOP});"));
    assert_eq!(normal(&mut agent), report(1, 0));
    let many = agent.last_entry_steps().unwrap();
    run(&mut agent, &realm, "f(10);");
    assert_eq!(normal(&mut agent), report(1, 0));
    let some = agent.last_entry_steps().unwrap();
    println!("last_entry_steps: {many} resuming the loop of {RESUMED_LOOP}, {some} the loop of 10");
    assert!(
        many.saturating_sub(some) >= u64::from(RESUMED_LOOP) - 10,
        "{many} against {some}"
    );
}

/// The `frame_depth` of the depth tests.
const FRAME_DEPTH: u32 = 16;

/// Item 4, "a chain of `await` takes no depth": `f(n)` awaits, then awaits `f(n - 1)`, with `n`
/// above `frame_depth`; every frame of the chain is suspended off the frame stack, so the chain
/// runs to its end.
#[test]
fn a_chain_of_awaits_longer_than_frame_depth_runs_to_its_end() {
    let mut limits = limits();
    limits.frame_depth = FRAME_DEPTH;
    let (mut agent, realm) = new_agent(limits);
    let n = FRAME_DEPTH * 2;
    run(
        &mut agent,
        &realm,
        &format!(
            "var count = 0, state = 'pending';\n\
             async function f(n) {{ await null; count++; if (n > 0) await f(n - 1); }}\n\
             f({n}).then(function () {{ state = 'fulfilled'; }}, function (e) {{ state = String(e); }});"
        ),
    );
    let ran = normal(&mut agent);
    assert!(ran.promise_jobs > u64::from(n), "{ran:?}");
    assert_eq!(text(&mut agent, &realm, "count"), (n + 1).to_string());
    assert_eq!(text(&mut agent, &realm, "state"), "fulfilled");
    assert_eq!(agent.pending_microtasks(), Ok(0));
}

/// The depth script: `a()` waits on `pa` and `b()` on `pb`, each with [`LOCALS`] local slots;
/// `down(k, resolveIt)` recurses `k` calls deep, then resolves its promise and runs a
/// checkpoint through the host function `checkpointHere`.
fn depth_setup() -> String {
    let locals = locals();
    format!(
        "var ranA = false, ranB = false, resolveA, resolveB;\n\
         var pa = new Promise(function (r) {{ resolveA = r; }});\n\
         var pb = new Promise(function (r) {{ resolveB = r; }});\n\
         async function a() {{ {locals} await pa; ranA = true; }}\n\
         async function b() {{ {locals} await pb; ranB = true; return 'b'; }}\n\
         var callA = a(), callB = b();\n\
         function down(k, resolveIt) {{ if (k === 0) {{ resolveIt(); checkpointHere(); return; }} down(k - 1, resolveIt); }}\n\
         0;"
    )
}

/// Item 4: the resumption checks `frame_depth` first. The script resolves the promise `a`
/// waits on and then, with the frames of the script and of `down` filling the frame stack to
/// `frame_depth`, runs a checkpoint: the resumption throws the `RangeError` of "Depth bounds"
/// from the function the job called, `host_clean_up_after_job` receives it as `Err`, the
/// activation stays suspended, and `heap_bytes()` still holds its frame (the host roots the two
/// functions of the `await`, so the activation stays reachable; once it releases them a
/// `collect()` gives the frame back). That reaction is spent: a later checkpoint resumes
/// nothing and the promise of the call stays pending. With `b`, whose checkpoint runs one
/// frame shallower, the resumption runs it to its end.
#[test]
fn a_resumption_at_frame_depth_throws_a_range_error_and_leaves_the_activation_suspended() {
    let mut limits = limits();
    limits.frame_depth = FRAME_DEPTH;
    let (mut agent, realm) = new_agent(limits);
    agent.host_mut().keep = true;
    run(&mut agent, &realm, &depth_setup());
    // The script's frame and `down`'s k + 1 frames: frame_depth frames at k = FRAME_DEPTH - 2.
    let deepest = FRAME_DEPTH - 2;
    let resume_a = parse(
        &mut agent,
        &realm,
        &format!("down({deepest}, resolveA); 0;"),
    );
    let suspended = collected(&mut agent);
    run_parsed(&mut agent, &resume_a);
    assert_eq!(agent.host().notes, ["checkpoint 1"]);
    assert_eq!(
        agent.host().events,
        [
            "prepare",
            "call 1 marker 1",
            "clean-up RangeError frame_depth exceeded"
        ]
    );
    let after = collected(&mut agent);
    assert_eq!(normal(&mut agent), report(0, 0));
    assert_eq!(text(&mut agent, &realm, "ranA"), "false");
    let kept_a: Vec<Root> = agent.host_mut().kept.drain(..2).collect();
    drop(kept_a);
    let freed = collected(&mut agent);
    assert_eq!(state_of(&mut agent, &realm, "callA"), "pending");
    println!(
        "heap_bytes: {suspended} suspended, {after} after the refused resumption, {freed} once \
         the functions of the await are released"
    );
    assert!(
        after.saturating_add(FRAME_FLOOR) > suspended,
        "{suspended} -> {after}"
    );
    assert!(
        after.saturating_sub(freed) >= FRAME_FLOOR,
        "{after} -> {freed}"
    );

    agent.host_mut().events.clear();
    agent.host_mut().notes.clear();
    run(
        &mut agent,
        &realm,
        &format!("down({}, resolveB); 0;", deepest - 1),
    );
    assert_eq!(agent.host().notes, ["checkpoint 1"]);
    assert_eq!(
        agent.host().events,
        ["prepare", "call 1 marker 3", "clean-up ok"]
    );
    assert_eq!(text(&mut agent, &realm, "ranB"), "true");
    assert_eq!(state_of(&mut agent, &realm, "callB"), "fulfilled b");
}

/// The host function `checkpointHere` of the stack test: a checkpoint through `cx`.
fn checkpoint_here_test_host(cx: &mut Cx<'_, TestHost>, _call: HostCall<'_>) -> Completion<Value> {
    cx.perform_microtask_checkpoint()?;
    Ok(Value::Undefined)
}

/// Whether a checkpoint of the test host's agent answers `Normal` with both counts 0.
fn runs_nothing(agent: &mut Agent<TestHost>) -> bool {
    matches!(
        agent.perform_microtask_checkpoint(),
        Ok(Outcome::Normal(CheckpointReport {
            promise_jobs: 0,
            host_microtasks: 0
        }))
    )
}

/// The script of the stack test: `a()` waits on `p`, and its body calls through `reenter` once
/// more when it resumes; `f(k)` recurses through `reenter` `k` times, then resolves `p` and
/// runs a checkpoint through `checkpointHere`.
const REENTER_SCRIPT: &str = "var ran = false, inner = false, resolveP;\n\
     var p = new Promise(function (r) { resolveP = r; });\n\
     async function a() { await p; ran = true; reenter(function () { inner = true; }); }\n\
     var call = a();\n\
     function f(k) {\n\
       if (k === 0) { resolveP(); return checkpointHere(); }\n\
       return reenter(function () { return f(k - 1); });\n\
     }\n";

/// The body of [`a_resumption_re_enters_on_the_promised_stack`], on the bounded stack. With
/// `reentry_depth` R, the job runs inside `f(k)`'s checkpoint with k + 1 activations below it
/// (k of `reenter` and `checkpointHere`); the call of the function of the `await` is the
/// (k + 2)-th and the resumption one re-entry more. At k = R - 1 the resumption would be the
/// (R + 1)-th re-entry: it ends in a `RangeError` (the test host does not record
/// `host_clean_up_after_job`; the `RangeError` is read through `ran`, which stays false, and
/// in the frame-depth test, which records it), the activation stays suspended, that reaction
/// is spent and the promise of the call stays pending. In a second run one level shallower,
/// whose deepest re-entry is the R-th (k = R - 3: the resumption the (R - 1)-th, and the
/// body's own call through `reenter` the R-th), the resumption runs the activation to its end.
fn resume_through_reenter() {
    let reentry = BUILTINS_REENTRY;
    let (mut agent, realm) = stack_agent(reentry);
    define_function(
        &mut agent,
        &realm,
        "checkpointHere",
        0,
        checkpoint_here_test_host,
        false,
    );
    let refused = format!("{REENTER_SCRIPT}f({}); ran;", reentry - 1);
    assert_eq!(
        eval_text(&mut agent, &realm, &refused),
        ScriptOutcome::Normal("false".to_string())
    );
    assert!(runs_nothing(&mut agent));
    let probe = "var state = 'pending';\n\
                 call.then(function () { state = 'settled'; }, function () { state = 'settled'; });";
    assert!(matches!(
        eval(&mut agent, &realm, probe),
        Outcome::Normal(_)
    ));
    assert!(runs_nothing(&mut agent));
    assert_eq!(
        eval_text(&mut agent, &realm, "ran + ' ' + state;"),
        ScriptOutcome::Normal("\"false pending\"".to_string())
    );

    let (mut agent, realm) = stack_agent(reentry);
    define_function(
        &mut agent,
        &realm,
        "checkpointHere",
        0,
        checkpoint_here_test_host,
        false,
    );
    let runs = format!("{REENTER_SCRIPT}f({}); ran && inner;", reentry - 3);
    assert_eq!(
        eval_text(&mut agent, &realm, &runs),
        ScriptOutcome::Normal("true".to_string())
    );
}

/// Item 6 of "Suspended frames" and the re-entry bound: a resumption is one re-entry, refused
/// past `reentry_depth`, and the deepest resumption the bound lets run, with one more
/// re-entry from its body, does not overflow seven eighths of `min_stack_bytes()`
/// ([`resume_through_reenter`]).
#[test]
fn a_resumption_re_enters_on_the_promised_stack() {
    let stack = stack_with_room(stack_limits(BUILTINS_REENTRY));
    on_bounded_stack(
        test_path!("a_resumption_re_enters_on_the_promised_stack"),
        stack,
        resume_through_reenter,
    )
    .unwrap();
}

/// HOST-194 (`docs/design/host-interface.md`, decision 11, "The hooks of an `await`"):
/// `(async function () { await 1; })()` calls `host_make_job_callback` twice, with two distinct
/// functions; the checkpoint after it runs one promise job, for which the engine calls
/// `host_prepare_to_run_job` with the script's realm, then `host_call_job_callback` once, with
/// the first of those two functions and the value `host_make_job_callback` returned for it,
/// then `host_clean_up_after_job` with `Ok(())`.
#[test]
fn host_194_an_await_makes_two_job_callbacks_and_its_job_calls_one() {
    let (mut agent, realm) = new_agent(limits());
    agent.host_mut().keep = true;
    run(&mut agent, &realm, "(async function () { await 1; })();");
    assert_eq!(agent.host().made, 2);
    let first = kept(&mut agent, 0);
    let second = kept(&mut agent, 1);
    let distinct = agent
        .with_cx(&realm, |cx| {
            cx.get(&first).unwrap() != cx.get(&second).unwrap()
        })
        .unwrap();
    assert!(distinct, "the two functions of the await are one");
    assert!(agent.host().events.is_empty());
    assert_eq!(normal(&mut agent), report(1, 0));
    assert_eq!(
        agent.host().events,
        ["prepare", "call 1 marker 1", "clean-up ok"]
    );
    let here = agent
        .with_cx(&realm, |cx| cx.realm_ref(&realm).unwrap())
        .unwrap();
    assert_eq!(agent.host().prepares, [here]);
}

/// The step budget of the termination tests: room for their other scripts, and few enough
/// steps that the loop which spends it stays cheap with a collection at every safe point.
const STEP_BUDGET: u64 = 300;

/// HOST-195 (`docs/design/host-interface.md`, section 10; item 5):
/// `(async function () { await null; for (;;) {} })()` answers `Normal` with a promise; the next
/// checkpoint answers `Terminated(StepBudget)`, and the promise reads pending after it and after
/// a later checkpoint, which answers `Normal` with both counts 0. The frame (with [`LOCALS`]
/// local slots) is credited by the termination: `heap_bytes()` right after it, with no
/// `collect()`, is at least [`FRAME_FLOOR`] below the reading while it was suspended.
#[test]
fn host_195_a_step_budget_termination_in_a_resumed_activation_leaves_its_promise_pending_for_good()
{
    let (mut agent, realm) = new_agent(limits_with(STEP_BUDGET));
    let promise = run(
        &mut agent,
        &realm,
        &format!(
            "var p = (async function () {{ {} await null; for (;;) {{}} }})(); p;",
            locals()
        ),
    );
    let is_promise = agent
        .with_cx(&realm, |cx| {
            matches!(cx.get(&promise).unwrap(), Value::Object(_))
        })
        .unwrap();
    assert!(is_promise);
    let suspended = collected(&mut agent);
    assert_eq!(terminated(&mut agent), Termination::StepBudget);
    let after = agent.heap_bytes().unwrap();
    println!("heap_bytes: {suspended} suspended, {after} after the termination");
    assert!(
        suspended.saturating_sub(after) >= FRAME_FLOOR,
        "{suspended} -> {after}"
    );
    assert_eq!(agent.pending_microtasks(), Ok(0));
    assert_eq!(state_of(&mut agent, &realm, "p"), "pending");
    assert_eq!(normal(&mut agent), report(0, 0));
    assert_eq!(state_of(&mut agent, &realm, "p"), "pending");
}

/// The room of the memory budget tests above the reading of a fresh realm.
const FILL_ROOM: u64 = 1024 * 1024;

/// Statements that spend any room in a few steps and safe points: Strings that double, each
/// kept, until the charge of one is refused.
const FILL: &str = "var a = []; for (var n = 1024; ; n = n * 2) a.push(\"x\".repeat(n));";

/// An agent of this file's host whose memory budget is the reading of `heap_bytes()` of a fresh
/// realm, measured in an agent of its own, plus [`FILL_ROOM`].
fn memory_agent() -> (Agent<AsyncHost>, Realm) {
    let realm_bytes = {
        let (agent, _realm) = new_agent(limits());
        agent.heap_bytes().unwrap()
    };
    let mut limits = limits();
    limits.memory_bytes = realm_bytes + FILL_ROOM;
    new_agent(limits)
}

/// Item 5 for an activation running in the job that resumed it when the checkpoint ends in
/// `termination` (`ending` is the code after its `await`, inside a `try` with a `finally`): the
/// `finally` does not run, the promise of the call stays pending for good, and the function of
/// the `await` the host kept from `host_make_job_callback`, called afterwards, returns
/// `undefined` and runs nothing.
fn pending_for_good(
    agent: &mut Agent<AsyncHost>,
    realm: &Realm,
    ending: &str,
    termination: Termination,
) {
    agent.host_mut().keep = true;
    run(
        agent,
        realm,
        &format!(
            "var ranFinally = false, resumed = 0;\n\
             var p = (async function () {{ try {{ await null; resumed++; {ending} }} finally {{ ranFinally = true; }} }})();"
        ),
    );
    assert_eq!(terminated(agent), termination);
    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
    assert_eq!(text(agent, realm, "resumed + ' ' + ranFinally"), "1 false");
    assert_eq!(state_of(agent, realm, "p"), "pending");
    let on_fulfilled = kept(agent, 0);
    let outcome = call(agent, realm, &on_fulfilled, Value::Null);
    assert!(answers_undefined(agent, realm, outcome));
    assert_eq!(text(agent, realm, "resumed + ' ' + ranFinally"), "1 false");
    assert_eq!(normal(agent), report(0, 0));
    assert_eq!(state_of(agent, realm, "p"), "pending");
}

/// Item 5: `Terminated(MemoryBudget)` in a resumed activation leaves its promise pending for
/// good ([`pending_for_good`]).
#[test]
fn a_memory_budget_termination_in_a_resumed_activation_leaves_its_promise_pending_for_good() {
    let (mut agent, realm) = memory_agent();
    pending_for_good(&mut agent, &realm, FILL, Termination::MemoryBudget);
}

/// Item 5: `Terminated(Interrupted)` in a resumed activation leaves its promise pending for
/// good ([`pending_for_good`]).
#[test]
fn an_interrupt_in_a_resumed_activation_leaves_its_promise_pending_for_good() {
    let (mut agent, realm) = new_agent(limits());
    pending_for_good(
        &mut agent,
        &realm,
        "interruptNow(); for (;;) {}",
        Termination::Interrupted,
    );
}

/// The `then` callback a checkpoint runs before the job of an activation: it loops 10 turns,
/// more steps than any other script of the tests that use it.
const BURN: &str = "function burn() { for (var i = 0; i < 10; i++) {} }\n\
     Promise.resolve().then(burn);";

/// The steps of a checkpoint that runs only the job of [`BURN`], read in an agent of its own.
fn burn_steps() -> u64 {
    let (mut probe, probe_realm) = new_agent(limits());
    run(&mut probe, &probe_realm, BURN);
    assert_eq!(normal(&mut probe), report(1, 0));
    probe.last_entry_steps().unwrap()
}

/// The steps the resumed body of `f` in
/// [`a_resumed_await_has_spent_three_steps_when_its_next_instruction_runs`] spends after its
/// three: counted from its disassembly, `JumpIfTag`, `Pop` (the `await` statement), `Undefined`,
/// `GetGlobal "probe"`, `Call 0` and the step of the call of the host function, `Pop`,
/// `Undefined`, `AsyncReturn`.
const AFTER_THE_AWAIT: u64 = 9;

/// Item 4: a resumed `await` has spent 3 steps when its next instruction runs (taking the job
/// off, the call of the function of the `await`, the resumption). The checkpoint's budget is the
/// steps of a job queued before the activation's ([`BURN`]; a budget of 3 alone would not let
/// the script that makes the activation run) plus 3: it answers `Terminated(StepBudget)` and
/// `probe` is not called. With an ample budget, `last_entry_steps()` of the checkpoint that
/// resumes it alone reads 3 plus [`AFTER_THE_AWAIT`].
#[test]
fn a_resumed_await_has_spent_three_steps_when_its_next_instruction_runs() {
    const F: &str = "async function f() { await null; probe(); }";
    let burn = burn_steps();
    let (mut agent, realm) = new_agent(limits_with(burn + 3));
    run(&mut agent, &realm, &format!("{F}\n{BURN}\nf();"));
    assert_eq!(terminated(&mut agent), Termination::StepBudget);
    assert!(agent.host().notes.is_empty(), "{:?}", agent.host().notes);
    assert_eq!(agent.last_entry_steps(), Ok(burn + 3));

    let (mut agent, realm) = new_agent(limits());
    run(&mut agent, &realm, &format!("{F}\nf();"));
    assert_eq!(normal(&mut agent), report(1, 0));
    assert_eq!(agent.host().notes, ["probe"]);
    println!(
        "last_entry_steps: {} (3 + {AFTER_THE_AWAIT}); the job before it {burn}",
        agent.last_entry_steps().unwrap()
    );
    assert_eq!(agent.last_entry_steps(), Ok(3 + AFTER_THE_AWAIT));
}

/// Item 3: a function of an `await` called at any other time than while its activation is
/// suspended at its suspension does nothing and returns `undefined`. The host keeps the
/// functions from `host_make_job_callback`; after the job ran the first (onFulfilled), the body
/// is suspended at its second `await`: a call of the second function of the first `await`
/// (onRejected), and a second call of the first, each return `undefined` and leave the
/// activation as it was, which a call of onFulfilled of the second `await` then shows by
/// resuming it.
#[test]
fn a_function_of_an_await_called_at_another_time_does_nothing() {
    let (mut agent, realm) = new_agent(limits());
    agent.host_mut().keep = true;
    run(
        &mut agent,
        &realm,
        "var log = [];\n\
         async function f() { var x = await null; log.push('first ' + x);\n\
           var y = await new Promise(function () {}); log.push('second ' + y); }\n\
         var p = f();",
    );
    assert_eq!(normal(&mut agent), report(1, 0));
    assert_eq!(agent.host().made, 4);
    assert_eq!(text(&mut agent, &realm, "log.join()"), "first null");
    let on_fulfilled = kept(&mut agent, 0);
    let on_rejected = kept(&mut agent, 1);
    for function in [&on_rejected, &on_fulfilled] {
        let outcome = call(&mut agent, &realm, function, Value::Number(1.0));
        assert!(answers_undefined(&mut agent, &realm, outcome));
        assert_eq!(text(&mut agent, &realm, "log.join()"), "first null");
    }
    assert_eq!(agent.pending_microtasks(), Ok(0));
    let second = kept(&mut agent, 2);
    let outcome = call(&mut agent, &realm, &second, Value::Number(2.0));
    assert!(answers_undefined(&mut agent, &realm, outcome));
    assert_eq!(
        text(&mut agent, &realm, "log.join()"),
        "first null,second 2"
    );
    let outcome = call(&mut agent, &realm, &second, Value::Number(3.0));
    assert!(answers_undefined(&mut agent, &realm, outcome));
    assert_eq!(
        text(&mut agent, &realm, "log.join()"),
        "first null,second 2"
    );
    assert_eq!(state_of(&mut agent, &realm, "p"), "fulfilled undefined");
}

/// Item 5 for an activation suspended at an `await` when another job, queued before its own,
/// ends the checkpoint in `termination` (`ends` is that job's function): the activation keeps
/// its frame and its charge (a `collect()` after the termination leaves `heap_bytes()` at least
/// [`FRAME_FLOOR`] above the reading after the next checkpoint completes it and a `collect()`),
/// its job stays queued, and the next checkpoint resumes it and settles the promise of the
/// call.
fn resumes_at_a_later_checkpoint(
    agent: &mut Agent<AsyncHost>,
    realm: &Realm,
    ends: &str,
    termination: Termination,
) {
    run(
        agent,
        realm,
        &format!(
            "function ends() {{ {ends} }}\n\
             async function f() {{ {} await null; return 7; }}\n\
             Promise.resolve().then(ends);\n\
             var p = f();",
            locals()
        ),
    );
    assert_eq!(terminated(agent), termination);
    assert_eq!(agent.pending_microtasks(), Ok(1));
    let suspended = collected(agent);
    assert_eq!(normal(agent), report(1, 0));
    let completed = collected(agent);
    println!("heap_bytes after {termination:?}: {suspended} suspended, {completed} completed");
    assert!(
        suspended.saturating_sub(completed) >= FRAME_FLOOR,
        "{suspended} -> {completed}"
    );
    assert_eq!(state_of(agent, realm, "p"), "fulfilled 7");
}

/// Item 5: [`resumes_at_a_later_checkpoint`] at `Terminated(StepBudget)`.
#[test]
fn an_activation_suspended_at_a_step_budget_termination_resumes_at_a_later_checkpoint() {
    let (mut agent, realm) = new_agent(limits_with(STEP_BUDGET));
    resumes_at_a_later_checkpoint(&mut agent, &realm, "for (;;) {}", Termination::StepBudget);
}

/// Item 5: [`resumes_at_a_later_checkpoint`] at `Terminated(MemoryBudget)`; the collection after
/// the termination frees what the fill held, not the suspended activation.
#[test]
fn an_activation_suspended_at_a_memory_budget_termination_resumes_at_a_later_checkpoint() {
    let (mut agent, realm) = memory_agent();
    resumes_at_a_later_checkpoint(&mut agent, &realm, FILL, Termination::MemoryBudget);
}

/// Item 5: [`resumes_at_a_later_checkpoint`] at `Terminated(Interrupted)`.
#[test]
fn an_activation_suspended_at_an_interrupt_resumes_at_a_later_checkpoint() {
    let (mut agent, realm) = new_agent(limits());
    resumes_at_a_later_checkpoint(
        &mut agent,
        &realm,
        "interruptNow(); for (;;) {}",
        Termination::Interrupted,
    );
}

/// The script of the tests of a refused take-off, call or resumption: [`BURN`], then an
/// activation that waits on `null`, its job queued after the job of `burn`.
fn refused_script() -> String {
    format!("{BURN}\nvar p = (async function () {{ await null; return 'ran'; }})();")
}

/// Item 5: a job queued before the activation's spends the checkpoint's budget, so that the
/// step of taking the activation's job off is the one refused: `Terminated(StepBudget)` with
/// `pending_microtasks()` 1, and the next checkpoint resumes the activation and settles the
/// promise of the call.
#[test]
fn an_activation_whose_job_could_not_be_taken_off_resumes_at_a_later_checkpoint() {
    let burn = burn_steps();
    let (mut agent, realm) = new_agent(limits_with(burn));
    run(&mut agent, &realm, &refused_script());
    assert_eq!(terminated(&mut agent), Termination::StepBudget);
    assert_eq!(agent.pending_microtasks(), Ok(1));
    assert_eq!(normal(&mut agent), report(1, 0));
    assert_eq!(state_of(&mut agent, &realm, "p"), "fulfilled ran");
}

/// Item 5: with the checkpoint's budget at the steps of the job before the activation's
/// ([`BURN`]) plus 1, the step of the call of the function of the `await` is refused after its
/// job was taken off; plus 2, the step of the resumption. Each answers `Terminated(StepBudget)`
/// with `pending_microtasks()` 0; a later checkpoint answers `Normal` with both counts 0, and
/// the promise of the call reads pending. (The story states budgets of 1 and 2 with the job
/// alone in the queue; the script that makes the activation does not run in so few steps.)
#[test]
fn an_activation_whose_resumption_was_refused_is_never_resumed() {
    let burn = burn_steps();
    for extra in [1, 2] {
        let (mut agent, realm) = new_agent(limits_with(burn + extra));
        run(&mut agent, &realm, &refused_script());
        assert_eq!(terminated(&mut agent), Termination::StepBudget, "{extra}");
        assert_eq!(agent.last_entry_steps(), Ok(burn + extra), "{extra}");
        assert_eq!(agent.pending_microtasks(), Ok(0), "{extra}");
        assert_eq!(normal(&mut agent), report(0, 0), "{extra}");
        assert_eq!(state_of(&mut agent, &realm, "p"), "pending", "{extra}");
    }
}

/// Item 5: `Agent::clear_microtasks` drops the activation's job: it answers `Ok(1)`, the next
/// checkpoint answers `Normal` with both counts 0, the promise of the call reads pending, and
/// once the script's values are released a `collect()` brings `heap_bytes()` back to within the
/// bytes of the promise of a call (read as in
/// [`a_frame_suspended_at_an_await_is_charged_until_its_activation_is_freed`]) of the reading
/// before the script. A first round grows the arenas first.
#[test]
fn an_activation_whose_job_was_cleared_is_never_resumed() {
    let (mut agent, realm) = new_agent(limits());
    run(
        &mut agent,
        &realm,
        "async function h() {}\nvar p, q, stateRead; 0;",
    );
    let make = parse(
        &mut agent,
        &realm,
        &format!(
            "p = (async function () {{ {} await null; }})(); 0;",
            locals()
        ),
    );
    let call_h = parse(&mut agent, &realm, "q = h(); 0;");
    let release = parse(
        &mut agent,
        &realm,
        "p = null; q = null; stateRead = undefined; 0;",
    );
    // A first round grows the arenas to what the round needs: their capacity stays in
    // `heap_bytes()`.
    run_parsed(&mut agent, &call_h);
    run_parsed(&mut agent, &make);
    assert_eq!(agent.clear_microtasks(), Ok(1));
    assert_eq!(normal(&mut agent), report(0, 0));
    assert_eq!(state_of(&mut agent, &realm, "p"), "pending");
    run_parsed(&mut agent, &release);
    let before = collected(&mut agent);
    run_parsed(&mut agent, &call_h);
    let promise = collected(&mut agent).saturating_sub(before);
    run_parsed(&mut agent, &make);
    assert_eq!(agent.clear_microtasks(), Ok(1));
    assert_eq!(normal(&mut agent), report(0, 0));
    assert_eq!(state_of(&mut agent, &realm, "p"), "pending");
    run_parsed(&mut agent, &release);
    let after = collected(&mut agent);
    println!(
        "heap_bytes: {before} before the script, {after} after it is released; the promise of \
         a call {promise}"
    );
    assert!(
        after <= before + promise,
        "{before} -> {after}, promise {promise}"
    );
}
