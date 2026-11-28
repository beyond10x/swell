//! Promise runs (`story:promise-constructor-then-and-jobs`): the constructor, its resolving
//! functions, `then`, `catch`, `resolve` and `reject`, with promise jobs in the one microtask
//! queue of the agent and the host's hooks around each (`docs/design/host-interface.md`,
//! decision 11, "The checkpoint is one entry", "Steps, the interrupt and what a checkpoint
//! leaves" and "What a queued job costs"; section 10, "The microtask checkpoint";
//! `docs/design/engine.md`, decision 3, "The microtask checkpoint is one entry", and decision 4,
//! "Queued jobs" and "A built-in function with slots"): HOST-78, HOST-80, HOST-82, HOST-83,
//! HOST-84, HOST-148, HOST-198, HOST-199 and HOST-200, the root and the charge of a queued job,
//! the charge of the reactions of a pending promise and of a builtin closure's slots, and the
//! bounds a job reaches. Run also with the feature `collect-every-safe-point`.
#![cfg(test)]

use crate::support;

use std::any::Any;
use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use support::host::limits;
use support::scripted::{Hooks, Scripted};
use support::stack::stack_limits;
use swell::{
    Abrupt, Agent, AgentState, CheckpointReport, CompileStrings, Completion, Cx, Handled, HookCx,
    HostCall, HostData, HostDefined, InterruptHandle, IsoDateTime, Job, Limits, LoadRequest,
    Microtask, ModuleRef, ObjectRef, Outcome, Parsed, PropertyKey, Realm, RealmRef,
    RejectionOperation, Root, Source, StringRef, Termination, TimeZoneIdentifier, Tracer, Value,
};

/// The bytes one promise job's entry is charged while it is queued: the size of an entry of the
/// queue (`host/jobs.rs`, `promise_job_bytes`, which the library's test
/// `a_promise_job_entry_is_charged_its_size` reads), on this target.
const ENTRY_BYTES: u64 = 176;

/// The bytes one reaction takes in the lists of a pending promise (`object/promise.rs`,
/// `REACTION_BYTES`, read by the library's test `a_reaction_is_charged_its_size`).
const REACTION_BYTES: u64 = 112;

/// The bytes of the slots of a resolving function: its box and its share of [[AlreadyResolved]]
/// (`object/builtin_function.rs`, read by the library's test
/// `the_slots_of_a_closure_are_charged_their_box`).
const RESOLVING_BOX_BYTES: u64 = 44;

/// The bytes of the slots of the executor NewPromiseCapability makes (the same test).
const EXECUTOR_BOX_BYTES: u64 = 32;

/// A [[HostDefined]] that holds a number and no engine value.
struct Marker(u32);

impl HostData for Marker {
    fn trace(&self, _tracer: &mut Tracer<'_>) {}
}

/// A [[HostDefined]] that holds an engine value, which its `trace` lists.
struct Holder(Value);

impl HostData for Holder {
    fn trace(&self, tracer: &mut Tracer<'_>) {
        tracer.value(self.0);
    }
}

/// What `host_make_job_callback` of the host answers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum MakeAnswer {
    /// No [[HostDefined]].
    #[default]
    Empty,
    /// A [`Marker`] of the count of the calls so far.
    Marker,
    /// A [`Holder`] of a new String, held by nothing else.
    Holder,
}

/// The host of these tests: it counts every hook call, required and provided, in `hooks` (an
/// `Rc`, so that a test reads it after the agent is dropped) and the calls of
/// `host_prepare_to_run_job` in `prepared`, and records what the hooks of a promise job and the
/// host functions of the tests receive.
#[derive(Default)]
struct PromisesHost {
    hooks: Rc<Cell<u64>>,
    prepared: Rc<Cell<u64>>,
    /// What the host function `note` received, in order.
    notes: Vec<String>,
    /// The realm of each `host_enqueue_promise_job`.
    enqueued: Vec<Option<RealmRef>>,
    /// The realm of each `host_prepare_to_run_job`.
    prepares: Vec<RealmRef>,
    /// The realm and the result of each `host_clean_up_after_job`.
    cleanups: Vec<(Option<RealmRef>, Result<(), Value>)>,
    /// What `host_make_job_callback` answers, and how often it was called.
    make: MakeAnswer,
    made: u32,
    /// What `host_call_job_callback` read of the [[HostDefined]] it received.
    received: Vec<String>,
    /// The resolve function of the capability `makeCapability` made, rooted by the host.
    resolve: Option<Root>,
    /// The agent's interrupt handle, for the host function `interruptNow`.
    interrupt: Option<InterruptHandle>,
    /// The flag the host function `signal` sets for a thread of the test.
    signal: Option<Arc<AtomicBool>>,
}

/// One more hook call.
fn hook(cx: &HookCx<'_, Scripted>) {
    let hooks = &cx.host().state::<PromisesHost>().hooks;
    hooks.set(hooks.get() + 1);
}

fn host_ensure_can_compile_strings(
    cx: &mut Cx<'_, Scripted>,
    _request: &CompileStrings<'_>,
) -> Completion<()> {
    let hooks = &cx.host().state::<PromisesHost>().hooks;
    hooks.set(hooks.get() + 1);
    Ok(())
}

fn host_get_code_for_eval(cx: &mut HookCx<'_, Scripted>, _arg: ObjectRef) -> Option<StringRef> {
    hook(cx);
    None
}

fn host_ensure_can_add_private_element(
    cx: &mut HookCx<'_, Scripted>,
    _obj: ObjectRef,
) -> Completion<()> {
    hook(cx);
    Ok(())
}

fn host_has_source_text_available(cx: &mut HookCx<'_, Scripted>, _func: ObjectRef) -> bool {
    hook(cx);
    true
}

fn host_make_job_callback(
    cx: &mut HookCx<'_, Scripted>,
    _callback: ObjectRef,
) -> Option<HostDefined> {
    hook(cx);
    cx.host_mut().state_mut::<PromisesHost>().made += 1;
    match cx.host().state::<PromisesHost>().make {
        MakeAnswer::Empty => None,
        MakeAnswer::Marker => Some(Rc::new(Marker(cx.host().state::<PromisesHost>().made))),
        MakeAnswer::Holder => {
            let string = cx.new_string("held by the host-defined").unwrap();
            Some(Rc::new(Holder(Value::String(string))))
        }
    }
}

fn host_call_job_callback(
    cx: &mut Cx<'_, Scripted>,
    callback: ObjectRef,
    host_defined: Option<&HostDefined>,
    this: Value,
    args: &[Value],
) -> Completion<Value> {
    let hooks = &cx.host().state::<PromisesHost>().hooks;
    hooks.set(hooks.get() + 1);
    if let Some(host_defined) = host_defined {
        let data: &dyn Any = &**host_defined;
        let read = if let Some(Marker(n)) = data.downcast_ref::<Marker>() {
            format!("marker {n}")
        } else if let Some(Holder(Value::String(string))) = data.downcast_ref::<Holder>() {
            format!("holder {:?}", cx.to_rust_string_lossy(*string))
        } else {
            "other".to_string()
        };
        cx.host_mut()
            .state_mut::<PromisesHost>()
            .received
            .push(read);
    }
    cx.call(Value::Object(callback), this, args)
        .unwrap_or(Err(Abrupt::Terminated(Termination::EngineFault)))
}

fn host_enqueue_promise_job(cx: &mut HookCx<'_, Scripted>, realm: Option<RealmRef>) {
    hook(cx);
    cx.host_mut()
        .state_mut::<PromisesHost>()
        .enqueued
        .push(realm);
}

fn host_prepare_to_run_job(cx: &mut HookCx<'_, Scripted>, realm: RealmRef) {
    hook(cx);
    let prepared = &cx.host().state::<PromisesHost>().prepared;
    prepared.set(prepared.get() + 1);
    cx.host_mut()
        .state_mut::<PromisesHost>()
        .prepares
        .push(realm);
}

fn host_clean_up_after_job(
    cx: &mut HookCx<'_, Scripted>,
    realm: Option<RealmRef>,
    result: Result<(), Value>,
) {
    hook(cx);
    cx.host_mut()
        .state_mut::<PromisesHost>()
        .cleanups
        .push((realm, result));
}

fn host_enqueue_generic_job(cx: &mut HookCx<'_, Scripted>, _job: Job) {
    hook(cx);
}

fn host_enqueue_timeout_job(cx: &mut HookCx<'_, Scripted>, _job: Job, _milliseconds: f64) {
    hook(cx);
}

fn host_enqueue_finalization_registry_cleanup_job(cx: &mut HookCx<'_, Scripted>, _job: Job) {
    hook(cx);
}

fn host_promise_rejection_tracker(
    cx: &mut HookCx<'_, Scripted>,
    _promise: ObjectRef,
    _operation: RejectionOperation,
) {
    hook(cx);
}

fn host_load_imported_module(cx: &mut HookCx<'_, Scripted>, _request: LoadRequest) {
    hook(cx);
}

fn host_get_import_meta_properties(
    cx: &mut HookCx<'_, Scripted>,
    _module: ModuleRef,
) -> Completion<Vec<(PropertyKey, Value)>> {
    hook(cx);
    Ok(Vec::new())
}

fn host_finalize_import_meta(
    cx: &mut HookCx<'_, Scripted>,
    _import_meta: ObjectRef,
    _module: ModuleRef,
) {
    hook(cx);
}

fn host_resize_array_buffer(
    cx: &mut HookCx<'_, Scripted>,
    _buffer: ObjectRef,
    _new_byte_length: u64,
) -> Completion<Handled> {
    hook(cx);
    Ok(Handled::Unhandled)
}

fn host_grow_shared_array_buffer(
    cx: &mut HookCx<'_, Scripted>,
    _buffer: ObjectRef,
    _new_byte_length: u64,
) -> Completion<Handled> {
    hook(cx);
    Ok(Handled::Unhandled)
}

fn host_system_utc_epoch_nanoseconds(cx: &mut HookCx<'_, Scripted>, _global: ObjectRef) -> i128 {
    hook(cx);
    0
}

fn system_time_zone_identifier(cx: &mut HookCx<'_, Scripted>) -> String {
    hook(cx);
    "UTC".to_string()
}

fn get_named_time_zone_offset_nanoseconds(
    cx: &mut HookCx<'_, Scripted>,
    _id: &str,
    _epoch_ns: i128,
) -> i128 {
    hook(cx);
    0
}

fn get_named_time_zone_epoch_nanoseconds(
    cx: &mut HookCx<'_, Scripted>,
    _id: &str,
    _local: IsoDateTime,
) -> Vec<i128> {
    hook(cx);
    Vec::new()
}

fn available_named_time_zone_identifiers(cx: &mut HookCx<'_, Scripted>) -> Vec<TimeZoneIdentifier> {
    hook(cx);
    Vec::new()
}

/// The hooks of the tests of this module, given to the scripted host with the state
/// [`PromisesHost`] (`support/scripted.rs`).
const HOOKS: Hooks = Hooks {
    host_ensure_can_compile_strings,
    host_get_code_for_eval,
    host_ensure_can_add_private_element,
    host_has_source_text_available,
    host_make_job_callback,
    host_call_job_callback,
    host_enqueue_promise_job,
    host_prepare_to_run_job,
    host_clean_up_after_job,
    host_enqueue_generic_job,
    host_enqueue_timeout_job,
    host_enqueue_finalization_registry_cleanup_job,
    host_promise_rejection_tracker,
    host_load_imported_module,
    host_get_import_meta_properties,
    host_finalize_import_meta,
    host_resize_array_buffer,
    host_grow_shared_array_buffer,
    host_system_utc_epoch_nanoseconds,
    system_time_zone_identifier,
    get_named_time_zone_offset_nanoseconds,
    get_named_time_zone_epoch_nanoseconds,
    available_named_time_zone_identifiers,
};

/// The step budget of the tests whose jobs loop without end or queue without end: small, so that
/// they end soon in both collection modes.
const LOOP_STEPS: u64 = 1_000;

/// The test limits with `steps` as the step budget.
fn limits_with(steps: u64) -> Limits {
    let mut limits = limits();
    limits.steps_per_entry = steps;
    limits
}

/// The host function `note`: it records its first argument, a String as its text.
fn note(cx: &mut Cx<'_, Scripted>, call: HostCall<'_>) -> Completion<Value> {
    let line = match call.args.first().copied() {
        Some(Value::String(string)) => cx.to_rust_string_lossy(string).unwrap(),
        other => format!("{other:?}"),
    };
    cx.host_mut().state_mut::<PromisesHost>().notes.push(line);
    Ok(Value::Undefined)
}

/// An agent of the promises host with `limits` and a realm whose global `note` is [`note`].
fn new_agent(limits: Limits) -> (Agent<Scripted>, Realm) {
    let mut agent = Agent::new(
        Scripted::new(PromisesHost::default(), HOOKS),
        limits,
        support::host::seeds(),
    )
    .unwrap();
    let realm = match support::host::create_realm(&mut agent, None, None).unwrap() {
        Outcome::Normal(realm) => realm,
        other => panic!("create_realm answered {other:?}"),
    };
    define(&mut agent, &realm, "note", note);
    (agent, realm)
}

/// The outcome of the script `source`.
fn evaluate(agent: &mut Agent<Scripted>, realm: &Realm, source: &str) -> Outcome<Root> {
    let script = match agent
        .parse_script(realm, Source::Utf8(source), None)
        .unwrap()
    {
        Parsed::Parsed(script) => script,
        other => panic!("{source:?} answered {other:?}"),
    };
    agent.evaluate_script(&script).unwrap()
}

/// The script `source`, which must complete normally: its completion value.
fn run(agent: &mut Agent<Scripted>, realm: &Realm, source: &str) -> Root {
    match evaluate(agent, realm, source) {
        Outcome::Normal(value) => value,
        other => panic!("{source:?} answered {other:?}"),
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

/// Define the host function `steps` as the global `name` of `realm`.
fn define(agent: &mut Agent<Scripted>, realm: &Realm, name: &str, steps: swell::HostFn<Scripted>) {
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

/// The `RealmRef` of `realm`.
fn realm_ref(agent: &mut Agent<Scripted>, realm: &Realm) -> RealmRef {
    agent
        .with_cx(realm, |cx| cx.realm_ref(realm).unwrap())
        .unwrap()
}

/// A checkpoint.
fn checkpoint(agent: &mut Agent<Scripted>) -> Outcome<CheckpointReport> {
    agent.perform_microtask_checkpoint().unwrap()
}

/// The report of a checkpoint that must answer `Normal`.
fn normal(outcome: Outcome<CheckpointReport>) -> CheckpointReport {
    match outcome {
        Outcome::Normal(report) => report,
        other => panic!("the checkpoint answered {other:?}"),
    }
}

/// The reason of a checkpoint that must answer `Terminated`.
fn terminated(outcome: Outcome<CheckpointReport>) -> Termination {
    match outcome {
        Outcome::Terminated(termination) => termination,
        other => panic!("the checkpoint answered {other:?}"),
    }
}

/// A report of `promise_jobs` promise jobs and `host_microtasks` host microtasks.
fn report(promise_jobs: u64, host_microtasks: u64) -> CheckpointReport {
    CheckpointReport {
        promise_jobs,
        host_microtasks,
    }
}

/// The host function `queueB`: it queues a host microtask that records "b".
fn queue_b(cx: &mut Cx<'_, Scripted>, _call: HostCall<'_>) -> Completion<Value> {
    cx.queue_microtask(Box::new(|cx: &mut Cx<'_, Scripted>| {
        cx.host_mut()
            .state_mut::<PromisesHost>()
            .notes
            .push("b".to_string());
    }))
    .unwrap();
    Ok(Value::Undefined)
}

/// HOST-78: `then` callbacks a and c with a host microtask b queued between them through a host
/// function run in the order they were queued: a, b, c, in one checkpoint.
#[test]
fn host_78_promise_jobs_and_host_microtasks_run_in_one_order() {
    let (mut agent, realm) = new_agent(limits());
    define(&mut agent, &realm, "queueB", queue_b);
    run(
        &mut agent,
        &realm,
        "Promise.resolve().then(function () { note('a'); });\n\
         queueB();\n\
         Promise.resolve().then(function () { note('c'); });",
    );
    assert!(
        agent.host().state::<PromisesHost>().notes.is_empty(),
        "nothing ran in the script"
    );
    assert_eq!(normal(checkpoint(&mut agent)), report(2, 1));
    assert_eq!(agent.host().state::<PromisesHost>().notes, ["a", "b", "c"]);
    assert_eq!(agent.pending_microtasks(), Ok(0));
}

/// The script of HOST-80 and HOST-148: a job that queues a job like itself.
const ENDLESS: &str = "function f() { Promise.resolve().then(f); } f();";

/// HOST-80: after the script `function f() { Promise.resolve().then(f); } f();` a checkpoint with
/// `steps_per_entry` N answers `Terminated(StepBudget)` with `last_entry_steps()` at most N, and
/// a job is still queued.
#[test]
fn host_80_a_job_that_queues_itself_ends_the_checkpoint_at_the_step_budget() {
    let (mut agent, realm) = new_agent(limits_with(LOOP_STEPS));
    run(&mut agent, &realm, ENDLESS);
    assert_eq!(terminated(checkpoint(&mut agent)), Termination::StepBudget);
    let steps = agent.last_entry_steps().unwrap();
    assert!(steps <= LOOP_STEPS, "{steps} steps of {LOOP_STEPS}");
    assert!(agent.pending_microtasks().unwrap() >= 1);
    assert_eq!(agent.state(), AgentState::Alive);
}

/// HOST-82: `host_prepare_to_run_job` gets the realm of each promise job whose realm is not null
/// before it runs, and `host_clean_up_after_job` that realm after it, with `Err(value)` when the
/// job's completion was a throw: the resolve function of a capability made by a constructor of
/// the script throws, so the job that calls it completes with that throw. A job of a `then` with
/// no callable handler has no realm, and neither hook is called for it.
#[test]
fn host_82_the_hooks_around_a_promise_job_get_its_realm_and_its_throw() {
    let (mut agent, realm) = new_agent(limits());
    let here = realm_ref(&mut agent, &realm);
    run(
        &mut agent,
        &realm,
        "var thrown = new Error('from resolve');\n\
         function C(executor) {\n\
           executor(function () { throw thrown; }, function () { throw thrown; });\n\
         }\n\
         var p = Promise.resolve(1);\n\
         p.then(function (v) { note('first ' + v); });\n\
         p.constructor = { [Symbol.species]: C };\n\
         p.then(function (v) { return v; });\n\
         Promise.resolve(2).then();",
    );
    let thrown = run(&mut agent, &realm, "thrown");
    assert_eq!(normal(checkpoint(&mut agent)), report(3, 0));
    assert_eq!(agent.host().state::<PromisesHost>().notes, ["first 1"]);
    assert_eq!(agent.host().state::<PromisesHost>().prepares, [here, here]);
    let cleanups = std::mem::take(&mut agent.host_mut().state_mut::<PromisesHost>().cleanups);
    let thrown = agent
        .with_cx(&realm, |cx| cx.get(&thrown).unwrap())
        .unwrap();
    assert_eq!(cleanups, [(Some(here), Ok(())), (Some(here), Err(thrown))]);
    assert_eq!(agent.host().state::<PromisesHost>().prepared.get(), 2);
}

/// HOST-83: `host_enqueue_promise_job` is called once for each promise job, with its realm or
/// `None`, when it is queued, before it can run: a `then` with a callable handler on a settled
/// promise (the handler's realm), one with none (`None`), and a thenable resolved by `new
/// Promise` (the realm of its `then`): three calls after the script, as many as the entries
/// queued; the reaction of the thenable is announced when the thenable job settles its promise.
#[test]
fn host_83_each_promise_job_is_announced_to_the_host_before_it_can_run() {
    let (mut agent, realm) = new_agent(limits());
    let here = realm_ref(&mut agent, &realm);
    run(
        &mut agent,
        &realm,
        "var p = Promise.resolve(1);\n\
         p.then(function () { note('handler'); });\n\
         p.then();\n\
         var q = new Promise(function (resolve) {\n\
           resolve({ then: function (r) { note('then'); r(2); } });\n\
         });\n\
         q.then(function (v) { note('q ' + v); });",
    );
    assert_eq!(
        agent.host().state::<PromisesHost>().enqueued,
        [Some(here), None, Some(here)]
    );
    assert_eq!(agent.pending_microtasks(), Ok(3));
    assert!(agent.host().state::<PromisesHost>().notes.is_empty());
    assert_eq!(normal(checkpoint(&mut agent)), report(4, 0));
    assert_eq!(
        agent.host().state::<PromisesHost>().notes,
        ["handler", "then", "q 2"]
    );
    assert_eq!(
        agent.host().state::<PromisesHost>().enqueued,
        [Some(here), None, Some(here), Some(here)]
    );
}

/// HOST-84: `then` with two callables calls `host_make_job_callback` for each, and the job of the
/// one that runs calls `host_call_job_callback` with the [[HostDefined]] that was made for it; a
/// handler that is not callable makes no job callback.
#[test]
fn host_84_then_makes_a_job_callback_and_the_job_calls_it_with_its_host_defined() {
    let (mut agent, realm) = new_agent(limits());
    agent.host_mut().state_mut::<PromisesHost>().make = MakeAnswer::Marker;
    run(
        &mut agent,
        &realm,
        "var p = Promise.resolve(1);\n\
         p.then(function (v) { note('fulfilled ' + v); }, function () { note('rejected'); });",
    );
    assert_eq!(agent.host().state::<PromisesHost>().made, 2);
    run(&mut agent, &realm, "p.then(5, null);");
    assert_eq!(
        agent.host().state::<PromisesHost>().made,
        2,
        "no job callback for a value that is not callable"
    );
    assert_eq!(normal(checkpoint(&mut agent)), report(2, 0));
    assert_eq!(agent.host().state::<PromisesHost>().notes, ["fulfilled 1"]);
    assert_eq!(agent.host().state::<PromisesHost>().received, ["marker 1"]);
}

/// HOST-148: with `steps_per_entry` N, a checkpoint after a script that queued three `then`
/// callbacks answers `Normal` with `promise_jobs` 3; the script of HOST-80 then answers `Normal`,
/// and the next checkpoint answers `Terminated(StepBudget)` with `last_entry_steps()` at most N.
#[test]
fn host_148_the_host_can_tell_which_checkpoint_spent_its_budget() {
    let (mut agent, realm) = new_agent(limits_with(LOOP_STEPS));
    run(
        &mut agent,
        &realm,
        "for (var i = 0; i < 3; i++) Promise.resolve(i).then(function (v) { return v; });",
    );
    assert_eq!(normal(checkpoint(&mut agent)), report(3, 0));
    run(&mut agent, &realm, ENDLESS);
    assert_eq!(terminated(checkpoint(&mut agent)), Termination::StepBudget);
    let steps = agent.last_entry_steps().unwrap();
    assert!(steps <= LOOP_STEPS, "{steps} steps of {LOOP_STEPS}");
}

/// "Queued jobs": a handler and an argument that only the queue holds survive `collect()`, and
/// the job reads them when it runs.
#[test]
fn a_queued_job_keeps_its_values_through_collect() {
    let (mut agent, realm) = new_agent(limits());
    run(
        &mut agent,
        &realm,
        "(function () {\n\
           var o = { v: 'kept ' + 42 };\n\
           Promise.resolve(o).then(function (x) { note(x.v); });\n\
         })();",
    );
    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
    assert_eq!(normal(checkpoint(&mut agent)), report(1, 0));
    assert_eq!(agent.host().state::<PromisesHost>().notes, ["kept 42"]);
}

/// A script of `statement` written 100 times, one after another. Each instruction is a safe point
/// that collects in the mode `collect-every-safe-point`, so the tests of 100 calls write them out
/// rather than pay a loop's counter, test and jump for each.
fn hundred(statement: &str) -> String {
    statement.repeat(100)
}

/// The statement that queues one reaction job on the settled promise `settled`, written 100 times
/// by [`hundred`].
const QUEUE_ONE: &str = "settled.then(h);\n";

/// "Queued jobs": a promise job is charged from its enqueue until the checkpoint takes it off.
/// After a first round that grows the arenas to what the round needs, 100 reaction jobs queued
/// make `heap_bytes()`, after a collection that keeps them, exceed the reading before by at
/// least 100 times [`ENTRY_BYTES`]; after the checkpoint and one `collect()` it is back within
/// that much of the reading before. The test prints the readings.
#[test]
fn a_queued_job_is_charged_until_it_is_dequeued() {
    let (mut agent, realm) = new_agent(limits());
    run(
        &mut agent,
        &realm,
        "var settled = Promise.resolve(0); function h() {}",
    );
    run(&mut agent, &realm, &hundred(QUEUE_ONE));
    assert_eq!(normal(checkpoint(&mut agent)), report(100, 0));
    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
    let before = agent.heap_bytes().unwrap();
    run(&mut agent, &realm, &hundred(QUEUE_ONE));
    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
    let queued = agent.heap_bytes().unwrap();
    assert_eq!(normal(checkpoint(&mut agent)), report(100, 0));
    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
    let after = agent.heap_bytes().unwrap();
    println!("heap_bytes: before {before}, 100 jobs queued {queued}, after {after}");
    assert!(
        queued >= before + 100 * ENTRY_BYTES,
        "{queued} against {before}"
    );
    assert!(
        after.abs_diff(before) < 100 * ENTRY_BYTES,
        "{after} against {before}"
    );
}

/// The statement that adds one reaction to the pending promise `pending`, written 100 times by
/// [`hundred`].
const THEN_ONE: &str = "pending.then(h);\n";

/// The bytes a promise of the script `new Promise(...)` adds to a heap whose arenas have room
/// for it: its resolving functions' slots, which the collection that frees them gives back.
const PROMISE_BYTES: u64 = 2 * RESOLVING_BOX_BYTES;

/// "Queued jobs": the reactions of a pending promise are charged, in a box of its kind, while it
/// is pending: 100 `then` on a promise that stays pending make `heap_bytes()` exceed the reading
/// before by at least 100 times [`REACTION_BYTES`]; once it is resolved and the checkpoint has run
/// the 100 jobs, one `collect()` brings it back within the bytes of a promise of the reading
/// before. A first round grows the arenas first. The test prints the three readings.
#[test]
fn the_reactions_of_a_pending_promise_are_charged_until_it_settles() {
    let (mut agent, realm) = new_agent(limits());
    let round = "var resolvePending;\n\
                 var pending = new Promise(function (r) { resolvePending = r; });\n\
                 function h() {}";
    run(&mut agent, &realm, round);
    run(&mut agent, &realm, &hundred(THEN_ONE));
    run(&mut agent, &realm, "resolvePending(1);");
    assert_eq!(normal(checkpoint(&mut agent)), report(100, 0));
    run(&mut agent, &realm, round);
    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
    let before = agent.heap_bytes().unwrap();
    run(&mut agent, &realm, &hundred(THEN_ONE));
    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
    let pending = agent.heap_bytes().unwrap();
    run(&mut agent, &realm, "resolvePending(1);");
    assert_eq!(normal(checkpoint(&mut agent)), report(100, 0));
    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
    let after = agent.heap_bytes().unwrap();
    println!("heap_bytes: before {before}, 100 reactions {pending}, after {after}");
    assert!(
        pending >= before + 100 * REACTION_BYTES,
        "{pending} against {before}"
    );
    assert!(after <= before + PROMISE_BYTES, "{after} against {before}");
}

/// A chain of `then` longer than `frame_depth` (32) runs to its end: each job is a call of its
/// own that returns before the next job runs, so the chain takes no frame of another.
#[test]
fn a_chain_of_then_longer_than_frame_depth_runs_to_its_end() {
    let mut limits = limits();
    limits.frame_depth = 32;
    let (mut agent, realm) = new_agent(limits);
    run(
        &mut agent,
        &realm,
        "var p = Promise.resolve(0);\n\
         for (var i = 0; i < 100; i++) p = p.then(function (v) { return v + 1; });\n\
         p.then(function (v) { note('end ' + v); });",
    );
    assert_eq!(normal(checkpoint(&mut agent)), report(101, 0));
    assert_eq!(agent.host().state::<PromisesHost>().notes, ["end 100"]);
}

/// The host function `reenter` of `support/stack.rs`, for the promises host: it calls its first
/// argument through `cx.call`, one re-entry.
fn reenter(cx: &mut Cx<'_, Scripted>, call: HostCall<'_>) -> Completion<Value> {
    let function = call.args.first().copied().unwrap_or(Value::Undefined);
    cx.call(function, Value::Undefined, &[]).unwrap()
}

/// The host function `checkpointHere`: a checkpoint through `cx`, inside the running entry.
fn checkpoint_here(cx: &mut Cx<'_, Scripted>, _call: HostCall<'_>) -> Completion<Value> {
    let report = cx.perform_microtask_checkpoint()?;
    cx.host_mut()
        .state_mut::<PromisesHost>()
        .notes
        .push(format!("checkpoint {}", report.promise_jobs));
    Ok(Value::Undefined)
}

/// The re-entry limit of
/// [`a_then_handler_of_a_nested_checkpoint_counts_against_reentry_depth`].
const REENTRY: u32 = 16;

/// The call of a handler from Rust is one re-entry: a script queues a `then` handler `g` that
/// would set a global, recurses through `reenter` to the deepest level `reentry_depth` lets it
/// reach, and calls a host function that runs `cx.perform_microtask_checkpoint`: `g` does not
/// run, the job rejects its derived promise with a `RangeError`, and `host_clean_up_after_job`
/// receives `Ok`; one level shallower `g` runs and the derived promise is fulfilled with its
/// value. The derived promise is read by a `then` of a later script and checkpoint. (At the
/// deepest level `then` itself could not run: its NewPromiseCapability constructs the derived
/// promise through one more builtin call.)
#[test]
fn a_then_handler_of_a_nested_checkpoint_counts_against_reentry_depth() {
    for (k, ran, settled) in [
        (REENTRY, "false", "rejected RangeError"),
        (REENTRY - 1, "true", "fulfilled 7"),
    ] {
        let mut agent = Agent::new(
            Scripted::new(PromisesHost::default(), HOOKS),
            stack_limits(REENTRY),
            support::host::seeds(),
        )
        .unwrap();
        let realm = match support::host::create_realm(&mut agent, None, None).unwrap() {
            Outcome::Normal(realm) => realm,
            other => panic!("create_realm answered {other:?}"),
        };
        define(&mut agent, &realm, "note", note);
        define(&mut agent, &realm, "reenter", reenter);
        define(&mut agent, &realm, "checkpointHere", checkpoint_here);
        let here = realm_ref(&mut agent, &realm);
        run(
            &mut agent,
            &realm,
            &format!(
                "var ran = false, settled = 'pending';\n\
                 function g() {{ ran = true; return 7; }}\n\
                 function f(k) {{\n\
                   if (k === 0) {{ return checkpointHere(); }}\n\
                   return reenter(function () {{ return f(k - 1); }});\n\
                 }}\n\
                 var derived = Promise.resolve().then(g);\n\
                 f({k});"
            ),
        );
        assert_eq!(
            agent.host().state::<PromisesHost>().notes,
            ["checkpoint 1"],
            "at level {k}"
        );
        assert_eq!(text(&mut agent, &realm, "ran"), ran, "at level {k}");
        assert_eq!(
            agent.host().state::<PromisesHost>().cleanups,
            [(Some(here), Ok(()))],
            "at level {k}"
        );
        run(
            &mut agent,
            &realm,
            "derived.then(function (v) { settled = 'fulfilled ' + v; },\n\
                          function (e) { settled = 'rejected ' + e.name; });",
        );
        assert_eq!(normal(checkpoint(&mut agent)), report(1, 0));
        assert_eq!(text(&mut agent, &realm, "settled"), settled, "at level {k}");
    }
}

/// The host function `interruptNow`: it sets the agent's interrupt flag.
fn interrupt_now(cx: &mut Cx<'_, Scripted>, _call: HostCall<'_>) -> Completion<Value> {
    if let Some(interrupt) = &cx.host().state::<PromisesHost>().interrupt {
        interrupt.interrupt();
    }
    Ok(Value::Undefined)
}

/// The script function that allocates without end.
const GROW: &str = "function grow() { var a = []; for (;;) { a.push({}); } }";

/// The script of [`a_termination_in_a_job_leaves_its_promise_pending_and_the_rest_queued`]: jobs
/// a, b and c, where b's handler is `ends`, and the derived promise of b kept in `d`.
const THREE_JOBS: &str = "var state = 'none';\n\
     Promise.resolve().then(function () { note('a'); });\n\
     var d = Promise.resolve().then(ends);\n\
     Promise.resolve().then(function () { note('c'); });";

/// The room the memory budget case leaves above what the agent holds after its setup.
const MEMORY_ROOM: u64 = 64 * 1024;

/// Section 10, "The microtask checkpoint": a termination of one job, by the step budget, the
/// memory budget or the interrupt, ends the checkpoint; that job is not run again and the promise
/// it would have settled stays pending; the job behind it stays queued and the next checkpoint
/// runs it.
#[test]
fn a_termination_in_a_job_leaves_its_promise_pending_and_the_rest_queued() {
    let cases = [
        (
            Termination::StepBudget,
            "function ends() { note('b'); for (;;) {} }",
        ),
        (
            Termination::MemoryBudget,
            "function ends() { note('b'); grow(); }",
        ),
        (
            Termination::Interrupted,
            "function ends() { note('b'); interruptNow(); for (;;) {} }",
        ),
    ];
    for (reason, ends) in cases {
        let setup = format!("{GROW}\n{ends}\n{THREE_JOBS}");
        let mut limits = limits_with(1 << 20);
        if reason == Termination::MemoryBudget {
            let (mut probe, probe_realm) = new_agent(limits);
            define(&mut probe, &probe_realm, "interruptNow", interrupt_now);
            run(&mut probe, &probe_realm, &setup);
            assert!(matches!(probe.collect(), Ok(Outcome::Normal(_))));
            limits.memory_bytes = probe.heap_bytes().unwrap() + MEMORY_ROOM;
        }
        if reason == Termination::StepBudget {
            limits.steps_per_entry = LOOP_STEPS;
        }
        let (mut agent, realm) = new_agent(limits);
        agent.host_mut().state_mut::<PromisesHost>().interrupt = Some(agent.interrupt_handle());
        define(&mut agent, &realm, "interruptNow", interrupt_now);
        run(&mut agent, &realm, &setup);
        assert_eq!(terminated(checkpoint(&mut agent)), reason);
        assert_eq!(
            agent.host().state::<PromisesHost>().notes,
            ["a", "b"],
            "{reason:?}"
        );
        assert_eq!(agent.pending_microtasks(), Ok(1), "{reason:?}");
        assert!(!agent.interrupt_handle().is_set());
        assert_eq!(normal(checkpoint(&mut agent)), report(1, 0), "{reason:?}");
        assert_eq!(
            agent.host().state::<PromisesHost>().notes,
            ["a", "b", "c"],
            "{reason:?}"
        );
        run(
            &mut agent,
            &realm,
            "d.then(function () { state = 'fulfilled'; }, function () { state = 'rejected'; });",
        );
        assert_eq!(normal(checkpoint(&mut agent)), report(0, 0), "{reason:?}");
        assert_eq!(text(&mut agent, &realm, "state"), "none", "{reason:?}");
        assert_eq!(agent.state(), AgentState::Alive);
    }
}

/// A host microtask that records `n`.
fn record(n: u32) -> Microtask<Scripted> {
    Box::new(move |cx: &mut Cx<'_, Scripted>| {
        cx.host_mut()
            .state_mut::<PromisesHost>()
            .notes
            .push(n.to_string())
    })
}

/// The host function `signal`: it sets the flag a thread of the test waits for.
fn signal(cx: &mut Cx<'_, Scripted>, _call: HostCall<'_>) -> Completion<Value> {
    if let Some(flag) = &cx.host().state::<PromisesHost>().signal {
        flag.store(true, Ordering::SeqCst);
    }
    Ok(Value::Undefined)
}

/// HOST-198: the interrupt ends a checkpoint with `Terminated(Interrupted)` at each of three
/// places, and the flag is clear after each: set before the checkpoint with two host microtasks
/// queued (`last_entry_steps()` 0, both still queued); set through the agent's `InterruptHandle`
/// by the first of two host microtasks, which then returns (one queued, which the next
/// checkpoint runs); and set from another thread while a `then` callback loops without end (the
/// callback calls, before its loop, a host function that signals the thread through an atomic,
/// and the thread then sets the flag through a clone of the handle): the next checkpoint answers
/// `Normal` without running that callback again.
#[test]
fn host_198_the_interrupt_ends_a_checkpoint_at_three_places() {
    // At the start.
    let (mut agent, realm) = new_agent(limits());
    agent.queue_microtask(&realm, record(1)).unwrap();
    agent.queue_microtask(&realm, record(2)).unwrap();
    agent.interrupt_handle().interrupt();
    assert_eq!(terminated(checkpoint(&mut agent)), Termination::Interrupted);
    assert_eq!(agent.last_entry_steps(), Ok(0));
    assert_eq!(agent.pending_microtasks(), Ok(2));
    assert!(!agent.interrupt_handle().is_set());
    assert_eq!(normal(checkpoint(&mut agent)), report(0, 2));
    assert_eq!(agent.host().state::<PromisesHost>().notes, ["1", "2"]);

    // Between two entries.
    let (mut agent, realm) = new_agent(limits());
    let interrupt = agent.interrupt_handle();
    agent
        .queue_microtask(
            &realm,
            Box::new(move |cx: &mut Cx<'_, Scripted>| {
                cx.host_mut()
                    .state_mut::<PromisesHost>()
                    .notes
                    .push("1".to_string());
                interrupt.interrupt();
            }),
        )
        .unwrap();
    agent.queue_microtask(&realm, record(2)).unwrap();
    assert_eq!(terminated(checkpoint(&mut agent)), Termination::Interrupted);
    assert_eq!(agent.pending_microtasks(), Ok(1));
    assert!(!agent.interrupt_handle().is_set());
    assert_eq!(normal(checkpoint(&mut agent)), report(0, 1));
    assert_eq!(agent.host().state::<PromisesHost>().notes, ["1", "2"]);

    // While a job runs, from another thread.
    let (mut agent, realm) = new_agent(limits_with(1 << 26));
    let flag = Arc::new(AtomicBool::new(false));
    let stop = Arc::new(AtomicBool::new(false));
    agent.host_mut().state_mut::<PromisesHost>().signal = Some(Arc::clone(&flag));
    define(&mut agent, &realm, "signal", signal);
    run(
        &mut agent,
        &realm,
        "var runs = 0;\n\
         Promise.resolve().then(function () { runs++; signal(); for (;;) {} });",
    );
    let handle = agent.interrupt_handle();
    let (seen, stopped) = (Arc::clone(&flag), Arc::clone(&stop));
    let thread = std::thread::spawn(move || {
        while !seen.load(Ordering::SeqCst) && !stopped.load(Ordering::SeqCst) {
            std::thread::yield_now();
        }
        if seen.load(Ordering::SeqCst) {
            handle.interrupt();
        }
    });
    let outcome = checkpoint(&mut agent);
    stop.store(true, Ordering::SeqCst);
    thread.join().unwrap();
    assert_eq!(terminated(outcome), Termination::Interrupted);
    assert!(!agent.interrupt_handle().is_set());
    assert_eq!(agent.pending_microtasks(), Ok(0));
    assert_eq!(normal(checkpoint(&mut agent)), report(0, 0));
    assert_eq!(text(&mut agent, &realm, "runs"), "1");
}

/// The script functions of HOST-199: `spend` loops 20 turns, and `make(k)` answers a callback
/// that appends k to `log` and then calls `spend`. Each step is a safe point that collects in the
/// mode `collect-every-safe-point`, so the turns are few.
const SPEND: &str = "var log = [];\n\
     function spend() { var i = 0; while (i < 20) { i = i + 1; } return i; }\n\
     function make(k) { return function () { log.push(k); spend(); }; }";

/// HOST-199: with `steps_per_entry` N, four times the steps of a checkpoint that runs one such
/// callback (read first in an agent of its own), after a script that queues 20 `then` callbacks,
/// each of which first appends its number to a global array and then spends about N / 4 steps,
/// every checkpoint answers `Terminated(StepBudget)` with `last_entry_steps()` at most N until
/// one answers `Normal`, at most 30 of them, and the array then holds 1 to 20, each once and in
/// order.
#[test]
fn host_199_a_checkpoint_ended_by_its_budget_leaves_the_rest_in_order() {
    let (mut probe, probe_realm) = new_agent(limits());
    run(&mut probe, &probe_realm, SPEND);
    run(&mut probe, &probe_realm, "Promise.resolve().then(make(0));");
    assert_eq!(normal(checkpoint(&mut probe)), report(1, 0));
    let one = probe.last_entry_steps().unwrap();
    assert!(one > 20, "one callback spends {one} steps");
    let budget = one * 4;

    let (mut agent, realm) = new_agent(limits_with(budget));
    run(&mut agent, &realm, SPEND);
    run(
        &mut agent,
        &realm,
        "for (var k = 1; k <= 20; k++) Promise.resolve().then(make(k));",
    );
    let mut ended = false;
    let mut terminations = 0;
    for _ in 0..30 {
        match checkpoint(&mut agent) {
            Outcome::Normal(_) => {
                ended = true;
                break;
            }
            other => {
                assert_eq!(terminated(other), Termination::StepBudget);
                let steps = agent.last_entry_steps().unwrap();
                assert!(steps <= budget, "{steps} steps of {budget}");
                terminations += 1;
            }
        }
    }
    assert!(ended, "no checkpoint answered Normal");
    assert!(
        terminations >= 1,
        "the budget N = {budget} ended no checkpoint"
    );
    let expected: Vec<String> = (1..=20).map(|k| k.to_string()).collect();
    assert_eq!(text(&mut agent, &realm, "log"), expected.join(","));
}

/// HOST-200: `Agent::clear_microtasks` empties the queue without running anything: after the
/// script `var p = Promise.resolve().then(function () { return 1; }); p`, which answers `Normal`
/// with p, it answers `Ok(1)` and calls no hook; `pending_microtasks()` then reads 0 and
/// `heap_bytes()` less than before the call; the next checkpoint answers `Normal` with both
/// counts 0; and p stays pending: a `then` of a later script on it queues no job, and its
/// callbacks never run (`promise_state`, which HOST-46 adds with a later story, is not there
/// yet). The `compile_fail` half of HOST-200 is the doc test
/// `host_200_cx_and_hook_cx_have_no_clear_microtasks` of `src/compile_fail.rs`.
#[test]
fn host_200_clear_microtasks_empties_the_queue_without_running_anything() {
    let (mut agent, realm) = new_agent(limits());
    let p = run(
        &mut agent,
        &realm,
        "var state = 'pending';\n\
         var p = Promise.resolve().then(function () { state = 'ran'; return 1; }); p",
    );
    let is_object = agent
        .with_cx(&realm, |cx| matches!(cx.get(&p).unwrap(), Value::Object(_)))
        .unwrap();
    assert!(is_object, "the script answers the promise p");
    let hooks = agent.host().state::<PromisesHost>().hooks.get();
    let bytes = agent.heap_bytes().unwrap();
    assert_eq!(agent.clear_microtasks(), Ok(1));
    assert_eq!(
        agent.host().state::<PromisesHost>().hooks.get(),
        hooks,
        "no hook was called"
    );
    assert_eq!(agent.pending_microtasks(), Ok(0));
    let after = agent.heap_bytes().unwrap();
    assert!(after < bytes, "{after} against {bytes}");
    assert_eq!(normal(checkpoint(&mut agent)), report(0, 0));
    run(
        &mut agent,
        &realm,
        "p.then(function () { state = 'fulfilled'; }, function () { state = 'rejected'; });",
    );
    assert_eq!(agent.pending_microtasks(), Ok(0), "p is pending");
    assert_eq!(normal(checkpoint(&mut agent)), report(0, 0));
    assert_eq!(text(&mut agent, &realm, "state"), "pending");
}

/// The host function `makeCapability`: `Cx::new_promise_capability`, its resolve function rooted
/// by the host; it answers the promise.
fn make_capability(cx: &mut Cx<'_, Scripted>, _call: HostCall<'_>) -> Completion<Value> {
    let capability = cx.new_promise_capability()?;
    let resolve = cx.root(capability.resolve).unwrap();
    cx.host_mut().state_mut::<PromisesHost>().resolve = Some(resolve);
    Ok(Value::Object(capability.promise))
}

/// The host function `settle`: it calls the resolve function the host rooted with 42.
fn settle(cx: &mut Cx<'_, Scripted>, _call: HostCall<'_>) -> Completion<Value> {
    let resolve = cx
        .host_mut()
        .state_mut::<PromisesHost>()
        .resolve
        .take()
        .unwrap();
    let function = cx.get(&resolve).unwrap();
    cx.call(function, Value::Undefined, &[Value::Number(42.0)])
        .unwrap()
}

/// "Emptying the queue is the host's call", last sentence: a host function called from a job
/// settles a promise through a resolve function it holds; the reaction job that queues runs in
/// the same checkpoint behind every entry already queued: a, b, c.
#[test]
fn a_reaction_settled_from_a_job_runs_behind_the_entries_already_queued() {
    let (mut agent, realm) = new_agent(limits());
    define(&mut agent, &realm, "makeCapability", make_capability);
    define(&mut agent, &realm, "settle", settle);
    run(
        &mut agent,
        &realm,
        "makeCapability().then(function (v) { note('c ' + v); });\n\
         Promise.resolve().then(function () { note('a'); settle(); });\n\
         Promise.resolve().then(function () { note('b'); });",
    );
    assert_eq!(normal(checkpoint(&mut agent)), report(3, 0));
    assert_eq!(
        agent.host().state::<PromisesHost>().notes,
        ["a", "b", "c 42"]
    );
}

/// "Emptying the queue is the host's call": dropping the agent with three `then` callbacks
/// queued calls nothing of the host: the count of hook calls, shared with the host, reads the
/// same before and after the drop, and no `host_prepare_to_run_job` was called.
#[test]
fn dropping_the_agent_with_promise_jobs_queued_calls_nothing_of_the_host() {
    let (mut agent, realm) = new_agent(limits());
    run(
        &mut agent,
        &realm,
        "var ran = false;\n\
         for (var i = 0; i < 3; i++) Promise.resolve(i).then(function () { ran = true; });",
    );
    assert_eq!(agent.pending_microtasks(), Ok(3));
    let hooks = Rc::clone(&agent.host().state::<PromisesHost>().hooks);
    let prepared = Rc::clone(&agent.host().state::<PromisesHost>().prepared);
    let before = hooks.get();
    drop(realm);
    drop(agent);
    assert_eq!(hooks.get(), before, "no hook was called");
    assert_eq!(prepared.get(), 0, "no job was prepared");
}

/// `Cx::new_promise_capability` gives the host a promise it settles later: a host function makes
/// a capability and roots its resolve function, a script adds a `then`, and `Agent::call` of the
/// resolve function with 42 and a checkpoint hand 42 to the callback.
#[test]
fn new_promise_capability_gives_the_host_a_promise_it_settles_later() {
    let (mut agent, realm) = new_agent(limits());
    define(&mut agent, &realm, "makeCapability", make_capability);
    run(
        &mut agent,
        &realm,
        "var got = 'nothing';\n\
         makeCapability().then(function (v) { got = v; });",
    );
    assert_eq!(normal(checkpoint(&mut agent)), report(0, 0));
    assert_eq!(text(&mut agent, &realm, "got"), "nothing");
    let resolve = agent
        .host_mut()
        .state_mut::<PromisesHost>()
        .resolve
        .take()
        .unwrap();
    let (this, value) = agent
        .with_cx(&realm, |cx| {
            (
                cx.root(Value::Undefined).unwrap(),
                cx.root(Value::Number(42.0)).unwrap(),
            )
        })
        .unwrap();
    assert!(matches!(
        agent.call(&realm, &resolve, &this, &[&value]),
        Ok(Outcome::Normal(_))
    ));
    assert_eq!(normal(checkpoint(&mut agent)), report(1, 0));
    assert_eq!(text(&mut agent, &realm, "got"), "42");
}

/// NewPromiseResolveThenableJob (`#sec-newpromiseresolvethenablejob`): a thenable is resolved in
/// a job of its own: its `then` is not called by the script that resolves with it, but by the
/// checkpoint, before the reactions of the promise it settles.
#[test]
fn a_thenable_is_resolved_in_a_job_of_its_own() {
    let (mut agent, realm) = new_agent(limits());
    run(
        &mut agent,
        &realm,
        "var thenable = { then: function (r) { note('then'); r(5); } };\n\
         var p = Promise.resolve(thenable);\n\
         var q = new Promise(function (r) { r(thenable); });\n\
         note('sync');\n\
         p.then(function (v) { note('p ' + v); });\n\
         q.then(function (v) { note('q ' + v); });",
    );
    assert_eq!(agent.host().state::<PromisesHost>().notes, ["sync"]);
    assert_eq!(normal(checkpoint(&mut agent)), report(4, 0));
    assert_eq!(
        agent.host().state::<PromisesHost>().notes,
        ["sync", "then", "then", "p 5", "q 5"]
    );
}

/// The resolving functions (`#sec-createresolvingfunctions`): the first call of either settles the
/// promise and every later call of both does nothing; a throw of the executor after it resolved
/// does nothing; each has `length` 1 and the empty `name`.
#[test]
fn the_resolving_functions_resolve_once() {
    let (mut agent, realm) = new_agent(limits());
    run(
        &mut agent,
        &realm,
        "var res, rej;\n\
         var p = new Promise(function (a, b) { res = a; rej = b; });\n\
         res(1); res(2); rej(3);\n\
         p.then(function (v) { note('p ' + v); }, function (e) { note('p rejected ' + e); });\n\
         var q = new Promise(function (a) { a(4); throw 5; });\n\
         q.then(function (v) { note('q ' + v); }, function (e) { note('q rejected ' + e); });\n\
         var r = new Promise(function (a, b) { b(6); a(7); });\n\
         r.then(function (v) { note('r ' + v); }, function (e) { note('r rejected ' + e); });",
    );
    assert_eq!(normal(checkpoint(&mut agent)), report(3, 0));
    assert_eq!(
        agent.host().state::<PromisesHost>().notes,
        ["p 1", "q 4", "r rejected 6"]
    );
    assert_eq!(
        text(
            &mut agent,
            &realm,
            "[res.length, rej.length, res.name === '', rej.name === '']"
        ),
        "1,1,true,true"
    );
}

/// "The microtask checkpoint is one entry": each promise job queued spends one step, charged
/// before its record is made: a script that settles a promise with 100 reactions spends at
/// least 100 steps more than one that settles a promise with none, and the checkpoint that runs
/// those k = 100 jobs, whose handlers make no call, reads at least 2k steps.
#[test]
fn each_promise_job_queued_spends_one_step() {
    let (mut agent, realm) = new_agent(limits());
    run(
        &mut agent,
        &realm,
        "var r0, r100;\n\
         new Promise(function (r) { r0 = r; });\n\
         var p = new Promise(function (r) { r100 = r; });\n\
         function h() {}\n\
         for (var i = 0; i < 100; i++) p.then(h);",
    );
    run(&mut agent, &realm, "r0(1);");
    let none = agent.last_entry_steps().unwrap();
    run(&mut agent, &realm, "r100(1);");
    let hundred = agent.last_entry_steps().unwrap();
    assert!(hundred >= none + 100, "{hundred} steps against {none}");
    assert_eq!(normal(checkpoint(&mut agent)), report(100, 0));
    let steps = agent.last_entry_steps().unwrap();
    assert!(steps >= 200, "{steps} steps for 100 jobs");
}

/// "The checkpoint is one entry": a job whose `host_prepare_to_run_job` was called gets its
/// `host_clean_up_after_job`, with `Ok(())` after a termination: a handler that loops without end
/// ends the checkpoint in `Terminated(StepBudget)`, and the host saw the two hooks in order.
#[test]
fn a_termination_in_a_job_still_calls_clean_up_with_ok() {
    let (mut agent, realm) = new_agent(limits_with(LOOP_STEPS));
    let here = realm_ref(&mut agent, &realm);
    run(
        &mut agent,
        &realm,
        "Promise.resolve().then(function () { for (;;) {} });",
    );
    assert_eq!(terminated(checkpoint(&mut agent)), Termination::StepBudget);
    assert_eq!(agent.host().state::<PromisesHost>().prepares, [here]);
    assert_eq!(
        agent.host().state::<PromisesHost>().cleanups,
        [(Some(here), Ok(()))]
    );
}

/// "Queued jobs": the [[HostDefined]] of a job callback is traced while its job is queued: a
/// host whose `host_make_job_callback` answers a [[HostDefined]] holding a new String, which only
/// its `trace` lists: after `collect()` with the job queued, `host_call_job_callback` receives it
/// and the String reads as it was made.
#[test]
fn a_job_callback_keeps_its_host_defined_through_collect() {
    let (mut agent, realm) = new_agent(limits());
    agent.host_mut().state_mut::<PromisesHost>().make = MakeAnswer::Holder;
    run(
        &mut agent,
        &realm,
        "Promise.resolve().then(function () { note('ran'); });",
    );
    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
    assert_eq!(normal(checkpoint(&mut agent)), report(1, 0));
    assert_eq!(agent.host().state::<PromisesHost>().notes, ["ran"]);
    assert_eq!(
        agent.host().state::<PromisesHost>().received,
        ["holder Ok(\"held by the host-defined\")"]
    );
}

/// "A built-in function with slots": a builtin closure is charged with the box of its slots.
/// `heap_bytes()` across 100 calls of `new Promise(function () {})` whose executor keeps the two
/// resolving functions, and 100 of `Promise.resolve` with a constructor of the script that keeps
/// the executor NewPromiseCapability makes, all kept in an array, rises by at least 100 times the
/// boxes of their slots (two resolving functions and an executor), and one `collect()` after the
/// array is dropped gives at least that back. The functions the script passes are declared once
/// and the 100 pairs of calls are written out by [`hundred`].
#[test]
fn a_builtin_closure_is_charged_with_its_box() {
    let (mut agent, realm) = new_agent(limits());
    run(
        &mut agent,
        &realm,
        "var kept = [], resolve = Promise.resolve;\n\
         function nop() {}\n\
         function keep(a, b) { kept.push(a, b); }\n\
         function C(executor) { kept.push(executor); executor(nop, nop); }",
    );
    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
    let before = agent.heap_bytes().unwrap();
    run(
        &mut agent,
        &realm,
        &hundred("kept.push(new Promise(keep));\nresolve.call(C, 0);\n"),
    );
    let boxes = 100 * (2 * RESOLVING_BOX_BYTES + EXECUTOR_BOX_BYTES);
    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
    let held = agent.heap_bytes().unwrap();
    run(&mut agent, &realm, "kept = null;");
    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
    let after = agent.heap_bytes().unwrap();
    println!("heap_bytes: before {before}, 200 promises {held}, after {after}");
    assert!(held >= before + boxes, "{held} against {before}");
    assert!(held >= after + boxes, "{held} against {after}");
}
