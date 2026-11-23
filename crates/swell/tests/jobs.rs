//! The microtask queue of the agent and its checkpoint (`docs/design/host-interface.md`,
//! decision 11, "A host microtask's realm", "The checkpoint is one entry" and "Steps, the
//! interrupt and what a checkpoint leaves"; section 10, "The microtask checkpoint";
//! `docs/design/engine.md`, decision 3, "The microtask checkpoint is one entry", decision 4,
//! "Queued jobs", and decision 9, "Jobs"), shown with host microtasks, the only entries the queue
//! holds before `Promise`: HOST-77, HOST-79, HOST-81, HOST-192, HOST-193, HOST-196 and HOST-197,
//! the interrupt at the three places it ends a checkpoint, the memory budget, what dropping the
//! agent and `Agent::clear_microtasks` do with the entries queued, and the bounds a checkpoint
//! reaches: its step budget, `reentry_depth` and `frame_depth`. Run also with the feature
//! `collect-every-safe-point`.
#![cfg(test)]

use crate::support;

use std::cell::Cell;
use std::rc::Rc;

use support::host::{TestHost, limits};
use support::script::{define_function, show, show_thrown};
use support::stack::stack_agent;
use swell::{
    Abrupt, Agent, AgentState, Brand, CheckpointReport, CompileStrings, Completion, Cx, Handled,
    HookCx, Host, HostCall, HostError, HostObjectSpec, InterruptHandle, IsoDateTime, Job, Limits,
    LoadRequest, Microtask, ModuleRef, ObjectRef, Outcome, Parsed, PropertyKey, Prototype, Realm,
    RealmRef, RejectionOperation, Root, Source, StringRef, Termination, TimeZoneIdentifier, Value,
};

/// The host of these tests: it counts every hook call, required and provided, in `hooks` (an
/// `Rc`, so that a test reads it after the agent is dropped), and keeps what the host
/// microtasks and host functions of the tests record.
#[derive(Default)]
struct JobsHost {
    hooks: Rc<Cell<u64>>,
    /// The numbers the microtasks of a test recorded, in the order they ran.
    records: Vec<u32>,
    /// What a microtask or host function saw, as text.
    seen: Vec<String>,
    /// What `cx.perform_microtask_checkpoint` answered inside a microtask or host function.
    inner: Vec<Completion<CheckpointReport>>,
    /// The roots a microtask reads: the `%TypeError.prototype%` of two realms (HOST-192).
    roots: Vec<Root>,
    /// A host object kept with no root (HOST-196).
    kept: Option<ObjectRef>,
    /// The agent's interrupt handle, for the host function `interruptNow`.
    interrupt: Option<InterruptHandle>,
    /// How often the microtask `requeue` ran.
    count: u64,
}

/// One more hook call.
fn hook(cx: &HookCx<'_, JobsHost>) {
    let hooks = &cx.host().hooks;
    hooks.set(hooks.get() + 1);
}

impl Host for JobsHost {
    fn host_ensure_can_compile_strings(
        cx: &mut Cx<'_, Self>,
        _request: &CompileStrings<'_>,
    ) -> Completion<()> {
        let hooks = &cx.host().hooks;
        hooks.set(hooks.get() + 1);
        Ok(())
    }
    fn host_get_code_for_eval(cx: &mut HookCx<'_, Self>, _arg: ObjectRef) -> Option<StringRef> {
        hook(cx);
        None
    }
    fn host_ensure_can_add_private_element(
        cx: &mut HookCx<'_, Self>,
        _obj: ObjectRef,
    ) -> Completion<()> {
        hook(cx);
        Ok(())
    }
    fn host_has_source_text_available(cx: &mut HookCx<'_, Self>, _func: ObjectRef) -> bool {
        hook(cx);
        true
    }
    fn host_make_job_callback(
        cx: &mut HookCx<'_, Self>,
        _callback: ObjectRef,
    ) -> Option<swell::HostDefined> {
        hook(cx);
        None
    }
    fn host_call_job_callback(
        cx: &mut Cx<'_, Self>,
        callback: ObjectRef,
        _host_defined: Option<&swell::HostDefined>,
        this: Value,
        args: &[Value],
    ) -> Completion<Value> {
        let hooks = &cx.host().hooks;
        hooks.set(hooks.get() + 1);
        cx.call(Value::Object(callback), this, args)
            .unwrap_or(Err(Abrupt::Terminated(Termination::EngineFault)))
    }
    fn host_enqueue_promise_job(cx: &mut HookCx<'_, Self>, _realm: Option<RealmRef>) {
        hook(cx);
    }
    fn host_prepare_to_run_job(cx: &mut HookCx<'_, Self>, _realm: RealmRef) {
        hook(cx);
    }
    fn host_clean_up_after_job(
        cx: &mut HookCx<'_, Self>,
        _realm: Option<RealmRef>,
        _result: Result<(), Value>,
    ) {
        hook(cx);
    }
    fn host_enqueue_generic_job(cx: &mut HookCx<'_, Self>, _job: Job) {
        hook(cx);
    }
    fn host_enqueue_timeout_job(cx: &mut HookCx<'_, Self>, _job: Job, _milliseconds: f64) {
        hook(cx);
    }
    fn host_enqueue_finalization_registry_cleanup_job(cx: &mut HookCx<'_, Self>, _job: Job) {
        hook(cx);
    }
    fn host_promise_rejection_tracker(
        cx: &mut HookCx<'_, Self>,
        _promise: ObjectRef,
        _operation: RejectionOperation,
    ) {
        hook(cx);
    }
    fn host_load_imported_module(cx: &mut HookCx<'_, Self>, _request: LoadRequest) {
        hook(cx);
    }
    fn host_get_import_meta_properties(
        cx: &mut HookCx<'_, Self>,
        _module: ModuleRef,
    ) -> Completion<Vec<(PropertyKey, Value)>> {
        hook(cx);
        Ok(Vec::new())
    }
    fn host_finalize_import_meta(
        cx: &mut HookCx<'_, Self>,
        _import_meta: ObjectRef,
        _module: ModuleRef,
    ) {
        hook(cx);
    }
    fn host_resize_array_buffer(
        cx: &mut HookCx<'_, Self>,
        _buffer: ObjectRef,
        _new_byte_length: u64,
    ) -> Completion<Handled> {
        hook(cx);
        Ok(Handled::Unhandled)
    }
    fn host_grow_shared_array_buffer(
        cx: &mut HookCx<'_, Self>,
        _buffer: ObjectRef,
        _new_byte_length: u64,
    ) -> Completion<Handled> {
        hook(cx);
        Ok(Handled::Unhandled)
    }
    fn host_system_utc_epoch_nanoseconds(cx: &mut HookCx<'_, Self>, _global: ObjectRef) -> i128 {
        hook(cx);
        0
    }
    fn system_time_zone_identifier(cx: &mut HookCx<'_, Self>) -> String {
        hook(cx);
        "UTC".to_string()
    }
    fn get_named_time_zone_offset_nanoseconds(
        cx: &mut HookCx<'_, Self>,
        _id: &str,
        _epoch_ns: i128,
    ) -> i128 {
        hook(cx);
        0
    }
    fn get_named_time_zone_epoch_nanoseconds(
        cx: &mut HookCx<'_, Self>,
        _id: &str,
        _local: IsoDateTime,
    ) -> Vec<i128> {
        hook(cx);
        Vec::new()
    }
    fn available_named_time_zone_identifiers(cx: &mut HookCx<'_, Self>) -> Vec<TimeZoneIdentifier> {
        hook(cx);
        Vec::new()
    }
}

/// The step budget of the tests whose microtasks call a script function that loops without
/// end: small, so that the loop ends soon in both collection modes.
const LOOP_STEPS: u64 = 1_000;

/// The test limits with `steps` as the step budget.
fn limits_with(steps: u64) -> Limits {
    let mut limits = limits();
    limits.steps_per_entry = steps;
    limits
}

/// An agent of the jobs host with `limits` and a realm.
fn new_agent(limits: Limits) -> (Agent<JobsHost>, Realm) {
    let mut agent = Agent::new(JobsHost::default(), limits, support::host::seeds()).unwrap();
    let realm = new_realm(&mut agent);
    (agent, realm)
}

/// A realm with no setup closure.
fn new_realm(agent: &mut Agent<JobsHost>) -> Realm {
    match agent.create_realm(None, None).unwrap() {
        Outcome::Normal(realm) => realm,
        other => panic!("create_realm answered {other:?}"),
    }
}

/// The script `source`, which must complete normally.
fn run(agent: &mut Agent<JobsHost>, realm: &Realm, source: &str) {
    let script = match agent
        .parse_script(realm, Source::Utf8(source), None)
        .unwrap()
    {
        Parsed::Parsed(script) => script,
        other => panic!("{source:?} answered {other:?}"),
    };
    let outcome = agent.evaluate_script(&script).unwrap();
    assert!(
        matches!(outcome, Outcome::Normal(_)),
        "{source:?} answered {outcome:?}"
    );
}

/// A root of the global property `name` of `realm`.
fn global(agent: &mut Agent<JobsHost>, realm: &Realm, name: &str) -> Root {
    let global = agent.global_object(realm).unwrap();
    agent
        .with_cx(realm, |cx| {
            let Value::Object(global) = cx.get(&global).unwrap() else {
                panic!("the global object");
            };
            let key = PropertyKey::String(cx.new_string(name).unwrap());
            let value = cx.get_property(global, key).unwrap().unwrap();
            cx.root(value).unwrap()
        })
        .unwrap()
}

/// Define the host function `steps` as the global `name` of `realm`, made in `home`.
fn define(
    agent: &mut Agent<JobsHost>,
    realm: &Realm,
    home: &Realm,
    name: &str,
    steps: swell::HostFn<JobsHost>,
) {
    let global = agent.global_object(realm).unwrap();
    agent
        .with_cx(realm, |cx| {
            let home = cx.realm_ref(home).unwrap();
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

/// A host microtask that records `n` and makes no call.
fn record(n: u32) -> Microtask<JobsHost> {
    Box::new(move |cx: &mut Cx<'_, JobsHost>| cx.host_mut().records.push(n))
}

/// A host microtask that records `n`, then calls the function `function` holds with no
/// arguments and keeps what the call answered in `seen`.
fn record_and_call(n: u32, function: Root) -> Microtask<JobsHost> {
    Box::new(move |cx: &mut Cx<'_, JobsHost>| {
        cx.host_mut().records.push(n);
        let f = cx.get(&function).unwrap();
        let answer = cx.call(f, Value::Undefined, &[]).unwrap();
        cx.host_mut()
            .seen
            .push(format!("{n}: {:?}", answer.map(|_| ())));
    })
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

/// A report of `host_microtasks` host microtasks and no promise job.
fn host_microtasks(count: u64) -> CheckpointReport {
    CheckpointReport {
        promise_jobs: 0,
        host_microtasks: count,
    }
}

/// HOST-77: of five queued microtasks, the third calls a script function that loops without
/// end: the checkpoint answers `Terminated(StepBudget)`, the first two ran, two entries stay
/// queued, and the next checkpoint runs the fourth and fifth in order, not the third again.
#[test]
fn host_77_a_termination_leaves_the_jobs_behind_it_queued() {
    let (mut agent, realm) = new_agent(limits_with(LOOP_STEPS));
    run(&mut agent, &realm, "function endless() { for (;;) {} }");
    let endless = global(&mut agent, &realm, "endless");
    let mut endless = Some(endless);
    for n in 1..=5 {
        let task = match (n, endless.take()) {
            (3, Some(function)) => record_and_call(3, function),
            (_, function) => {
                endless = function;
                record(n)
            }
        };
        agent.queue_microtask(&realm, task).unwrap();
    }
    let outcome = agent.perform_microtask_checkpoint().unwrap();
    assert_eq!(terminated(outcome), Termination::StepBudget);
    assert_eq!(agent.host().records, [1, 2, 3]);
    assert_eq!(agent.host().seen, ["3: Err(Terminated(StepBudget))"]);
    assert_eq!(agent.pending_microtasks(), Ok(2));
    let report = normal(agent.perform_microtask_checkpoint().unwrap());
    assert_eq!(report, host_microtasks(2));
    assert_eq!(agent.host().records, [1, 2, 3, 4, 5]);
    assert_eq!(agent.pending_microtasks(), Ok(0));
    assert_eq!(agent.state(), AgentState::Alive);
}

/// HOST-79: a microtask that queues another through `cx.queue_microtask`: the checkpoint runs
/// both before it returns, answers `Normal` with `host_microtasks` 2, and leaves nothing queued.
#[test]
fn host_79_a_checkpoint_runs_the_microtasks_queued_while_it_runs() {
    let (mut agent, realm) = new_agent(limits());
    agent
        .queue_microtask(
            &realm,
            Box::new(|cx: &mut Cx<'_, JobsHost>| {
                cx.host_mut().records.push(1);
                cx.queue_microtask(record(2)).unwrap();
            }),
        )
        .unwrap();
    let report = normal(agent.perform_microtask_checkpoint().unwrap());
    assert_eq!(report, host_microtasks(2));
    assert_eq!(agent.pending_microtasks(), Ok(0));
    assert_eq!(agent.host().records, [1, 2]);
}

/// HOST-81: `cx.perform_microtask_checkpoint` called by a microtask during a checkpoint answers
/// both counts 0 at once and runs nothing; the entry the microtask queued before that call runs
/// after it, in the outer checkpoint.
#[test]
fn host_81_a_checkpoint_inside_a_checkpoint_returns_at_once() {
    let (mut agent, realm) = new_agent(limits());
    agent
        .queue_microtask(
            &realm,
            Box::new(|cx: &mut Cx<'_, JobsHost>| {
                cx.host_mut().records.push(1);
                cx.queue_microtask(record(3)).unwrap();
                let inner = cx.perform_microtask_checkpoint();
                cx.host_mut().inner.push(inner);
                cx.host_mut().records.push(2);
            }),
        )
        .unwrap();
    let report = normal(agent.perform_microtask_checkpoint().unwrap());
    assert_eq!(report, host_microtasks(2));
    assert_eq!(agent.host().inner, [Ok(host_microtasks(0))]);
    assert_eq!(agent.host().records, [1, 2, 3]);
    assert_eq!(agent.pending_microtasks(), Ok(0));
}

/// The script function of [`a_checkpoint_has_one_step_budget`]: a loop of 20 turns.
const SPEND: &str = "function spend() { var i = 0; while (i < 20) { i = i + 1; } return i; }";

/// One step budget for the whole checkpoint, not one per entry: with `steps_per_entry` N, 20
/// microtasks each calling a script function that spends about N / 8 steps end the checkpoint in
/// `Terminated(StepBudget)` with `last_entry_steps()` at most N and at least 10 entries still
/// queued. N is eight times the steps one call of the function spends, read first in an agent
/// of its own.
#[test]
fn a_checkpoint_has_one_step_budget() {
    let (mut probe, probe_realm) = new_agent(limits());
    run(&mut probe, &probe_realm, SPEND);
    let spend = global(&mut probe, &probe_realm, "spend");
    let this = probe
        .with_cx(&probe_realm, |cx| cx.root(Value::Undefined).unwrap())
        .unwrap();
    assert!(matches!(
        probe.call(&probe_realm, &spend, &this, &[]),
        Ok(Outcome::Normal(_))
    ));
    let one = probe.last_entry_steps().unwrap();
    assert!(one > 20, "a call of spend spends {one} steps");
    let budget = one * 8;

    let (mut agent, realm) = new_agent(limits_with(budget));
    run(&mut agent, &realm, SPEND);
    let spend = global(&mut agent, &realm, "spend");
    for n in 1..=20 {
        let function = agent.duplicate(&spend).unwrap();
        agent
            .queue_microtask(&realm, record_and_call(n, function))
            .unwrap();
    }
    let outcome = agent.perform_microtask_checkpoint().unwrap();
    assert_eq!(terminated(outcome), Termination::StepBudget);
    let steps = agent.last_entry_steps().unwrap();
    assert!(steps <= budget, "{steps} steps of {budget}");
    let pending = agent.pending_microtasks().unwrap();
    assert!(pending >= 10, "{pending} entries left");
    assert_eq!(
        agent.host().records.len() + pending,
        20,
        "each entry ran once or is queued"
    );
}

/// A host function that queues three microtasks that make no call, then runs a checkpoint
/// through `cx`, inside the running entry.
fn queue_three(cx: &mut Cx<'_, JobsHost>, _call: HostCall<'_>) -> Completion<Value> {
    for n in 1..=3 {
        cx.queue_microtask(record(n)).unwrap();
    }
    let report = cx.perform_microtask_checkpoint();
    cx.host_mut().inner.push(report);
    report.map(|_| Value::Undefined)
}

/// A host function that queues nothing and runs a checkpoint through `cx`.
fn queue_none(cx: &mut Cx<'_, JobsHost>, _call: HostCall<'_>) -> Completion<Value> {
    let report = cx.perform_microtask_checkpoint();
    cx.host_mut().inner.push(report);
    report.map(|_| Value::Undefined)
}

/// HOST-193: a checkpoint is one outermost entry and spends one step for each entry it takes
/// off: k microtasks that make no call answer `Normal` with `host_microtasks` k and
/// `last_entry_steps()` k; an empty queue answers both counts 0 and 0 steps; and a script whose
/// host function queues three such microtasks and runs `cx.perform_microtask_checkpoint` spends
/// at least 3 steps more than the same script whose host function queues none.
#[test]
fn host_193_a_checkpoint_spends_one_step_for_each_entry_it_takes_off() {
    let (mut agent, realm) = new_agent(limits());
    let k = 5;
    for n in 1..=k {
        agent.queue_microtask(&realm, record(n)).unwrap();
    }
    let report = normal(agent.perform_microtask_checkpoint().unwrap());
    assert_eq!(report, host_microtasks(u64::from(k)));
    assert_eq!(agent.last_entry_steps(), Ok(u64::from(k)));

    let report = normal(agent.perform_microtask_checkpoint().unwrap());
    assert_eq!(report, host_microtasks(0));
    assert_eq!(agent.last_entry_steps(), Ok(0));

    define(&mut agent, &realm, &realm, "queueThree", queue_three);
    define(&mut agent, &realm, &realm, "queueNone", queue_none);
    run(&mut agent, &realm, "queueNone();");
    let none = agent.last_entry_steps().unwrap();
    run(&mut agent, &realm, "queueThree();");
    let three = agent.last_entry_steps().unwrap();
    assert!(three >= none + 3, "{three} steps against {none}");
    assert_eq!(
        agent.host().inner,
        [Ok(host_microtasks(0)), Ok(host_microtasks(3))]
    );
    assert_eq!(agent.pending_microtasks(), Ok(0));
}

/// HOST-197: `last_entry_steps()` after a checkpoint reads the steps of the whole checkpoint,
/// also after a termination: with `steps_per_entry` 3 and five microtasks that make no call, the
/// checkpoint answers `Terminated(StepBudget)`, 3 steps, two entries queued; a checkpoint of two
/// such microtasks, the first of which runs `cx.perform_microtask_checkpoint` (both counts 0),
/// answers `Normal` with `host_microtasks` 2 and 2 steps.
#[test]
fn host_197_last_entry_steps_reads_the_whole_checkpoint() {
    let (mut agent, realm) = new_agent(limits_with(3));
    for n in 1..=5 {
        agent.queue_microtask(&realm, record(n)).unwrap();
    }
    let outcome = agent.perform_microtask_checkpoint().unwrap();
    assert_eq!(terminated(outcome), Termination::StepBudget);
    assert_eq!(agent.last_entry_steps(), Ok(3));
    assert_eq!(agent.pending_microtasks(), Ok(2));

    let (mut agent, realm) = new_agent(limits_with(3));
    agent
        .queue_microtask(
            &realm,
            Box::new(|cx: &mut Cx<'_, JobsHost>| {
                let inner = cx.perform_microtask_checkpoint();
                cx.host_mut().inner.push(inner);
            }),
        )
        .unwrap();
    agent.queue_microtask(&realm, record(2)).unwrap();
    let report = normal(agent.perform_microtask_checkpoint().unwrap());
    assert_eq!(report, host_microtasks(2));
    assert_eq!(agent.last_entry_steps(), Ok(2));
    assert_eq!(agent.host().inner, [Ok(host_microtasks(0))]);
}

/// The step of taking an entry off is charged while the entry is still first: with
/// `steps_per_entry` 3, of five microtasks that record their number the first checkpoint runs
/// three; the fourth, whose step was refused, is still first, and the next checkpoint answers
/// `Normal` with `host_microtasks` 2: each number recorded once, in order.
#[test]
fn a_refused_step_of_taking_an_entry_off_leaves_it_first() {
    let (mut agent, realm) = new_agent(limits_with(3));
    for n in 1..=5 {
        agent.queue_microtask(&realm, record(n)).unwrap();
    }
    let outcome = agent.perform_microtask_checkpoint().unwrap();
    assert_eq!(terminated(outcome), Termination::StepBudget);
    assert_eq!(agent.host().records, [1, 2, 3]);
    let report = normal(agent.perform_microtask_checkpoint().unwrap());
    assert_eq!(report, host_microtasks(2));
    assert_eq!(agent.host().records, [1, 2, 3, 4, 5]);
    assert_eq!(agent.last_entry_steps(), Ok(2));
}

/// The microtask of HOST-192: it calls a value that is not callable and records whether the
/// `TypeError` it gets has the `%TypeError.prototype%` of the second realm (the host's second
/// root) and not that of the first.
fn calls_a_number(cx: &mut Cx<'_, JobsHost>) {
    let answer = cx.call(Value::Number(1.0), Value::Undefined, &[]).unwrap();
    let Err(Abrupt::Throw(Value::Object(error))) = answer else {
        panic!("the call answered {answer:?}");
    };
    let prototype = cx.get_prototype_of(error).unwrap().unwrap();
    let roots = std::mem::take(&mut cx.host_mut().roots);
    let first = cx.get(&roots[0]).unwrap();
    let second = cx.get(&roots[1]).unwrap();
    cx.host_mut().roots = roots;
    let realm = match prototype.map(Value::Object) {
        Some(found) if found == second => "second",
        Some(found) if found == first => "first",
        _ => "neither",
    };
    cx.host_mut()
        .seen
        .push(format!("TypeError of the {realm} realm"));
}

/// A host function that queues [`calls_a_number`] through `cx.queue_microtask`.
fn queue_call(cx: &mut Cx<'_, JobsHost>, _call: HostCall<'_>) -> Completion<Value> {
    cx.queue_microtask(Box::new(calls_a_number)).unwrap();
    Ok(Value::Undefined)
}

/// HOST-192: a host microtask runs in the realm it was queued with. Queued by
/// `Agent::queue_microtask` with the second of two realms, a call of a value that is not
/// callable throws a `TypeError` whose prototype is the second realm's `%TypeError.prototype%`;
/// queued by `cx.queue_microtask` in a host function of the second realm called from a script
/// of the first, the same. A realm of another agent answers `Err(ForeignHandle)` and queues
/// nothing. (A released `Realm` cannot be made through the interface, which releases a `Realm`
/// only by its drop: the unit test `host::jobs::tests` of the crate gives `queue_microtask` one
/// and reads `Err(StaleHandle)`.)
#[test]
fn host_192_a_host_microtask_runs_in_the_realm_it_was_queued_with() {
    let (mut agent, first) = new_agent(limits());
    let second = new_realm(&mut agent);
    for realm in [&first, &second] {
        let error = global(&mut agent, realm, "TypeError");
        let prototype = agent
            .with_cx(realm, |cx| {
                let Value::Object(error) = cx.get(&error).unwrap() else {
                    panic!("TypeError");
                };
                let key = PropertyKey::String(cx.new_string("prototype").unwrap());
                let prototype = cx.get_property(error, key).unwrap().unwrap();
                cx.root(prototype).unwrap()
            })
            .unwrap();
        agent.host_mut().roots.push(prototype);
    }

    agent
        .queue_microtask(&second, Box::new(calls_a_number))
        .unwrap();
    assert_eq!(
        normal(agent.perform_microtask_checkpoint().unwrap()),
        host_microtasks(1)
    );

    define(&mut agent, &first, &second, "queueCall", queue_call);
    run(&mut agent, &first, "queueCall();");
    assert_eq!(agent.pending_microtasks(), Ok(1));
    assert_eq!(
        normal(agent.perform_microtask_checkpoint().unwrap()),
        host_microtasks(1)
    );
    assert_eq!(
        agent.host().seen,
        [
            "TypeError of the second realm",
            "TypeError of the second realm"
        ]
    );

    let (_other, other_realm) = new_agent(limits());
    let ran = Rc::new(Cell::new(false));
    let flag = Rc::clone(&ran);
    let answer = agent.queue_microtask(
        &other_realm,
        Box::new(move |_cx: &mut Cx<'_, JobsHost>| flag.set(true)),
    );
    assert_eq!(answer, Err(HostError::ForeignHandle));
    assert_eq!(agent.pending_microtasks(), Ok(0));
    assert_eq!(
        normal(agent.perform_microtask_checkpoint().unwrap()),
        host_microtasks(0)
    );
    assert!(!ran.get());
}

/// After a checkpoint the interrupt ended: the flag is clear, and the next checkpoint answers
/// `Normal` with `host_microtasks` `left`.
fn next_checkpoint_runs(agent: &mut Agent<JobsHost>, left: u64) {
    assert!(!agent.interrupt_handle().is_set(), "the flag is clear");
    let report = normal(agent.perform_microtask_checkpoint().unwrap());
    assert_eq!(report, host_microtasks(left));
}

/// The interrupt at the start: two microtasks queued and the flag set through the agent's
/// `InterruptHandle` before the call: `Terminated(Interrupted)` with nothing taken off,
/// `last_entry_steps()` 0 and two entries queued; the next checkpoint runs both.
#[test]
fn an_interrupt_set_before_the_checkpoint_ends_it_with_nothing_taken_off() {
    let (mut agent, realm) = new_agent(limits());
    agent.queue_microtask(&realm, record(1)).unwrap();
    agent.queue_microtask(&realm, record(2)).unwrap();
    agent.interrupt_handle().interrupt();
    let outcome = agent.perform_microtask_checkpoint().unwrap();
    assert_eq!(terminated(outcome), Termination::Interrupted);
    assert_eq!(agent.last_entry_steps(), Ok(0));
    assert_eq!(agent.pending_microtasks(), Ok(2));
    assert!(agent.host().records.is_empty());
    next_checkpoint_runs(&mut agent, 2);
    assert_eq!(agent.host().records, [1, 2]);
}

/// The interrupt between two microtasks: the first sets the flag and returns: the checkpoint
/// answers `Terminated(Interrupted)` with the second still queued, and the next checkpoint
/// runs it.
#[test]
fn an_interrupt_set_between_two_microtasks_leaves_the_next_first() {
    let (mut agent, realm) = new_agent(limits());
    let interrupt = agent.interrupt_handle();
    agent
        .queue_microtask(
            &realm,
            Box::new(move |cx: &mut Cx<'_, JobsHost>| {
                cx.host_mut().records.push(1);
                interrupt.interrupt();
            }),
        )
        .unwrap();
    agent.queue_microtask(&realm, record(2)).unwrap();
    let outcome = agent.perform_microtask_checkpoint().unwrap();
    assert_eq!(terminated(outcome), Termination::Interrupted);
    assert_eq!(agent.pending_microtasks(), Ok(1));
    assert_eq!(agent.host().records, [1]);
    next_checkpoint_runs(&mut agent, 1);
    assert_eq!(agent.host().records, [1, 2]);
}

/// A host function that sets the agent's interrupt flag.
fn interrupt_now(cx: &mut Cx<'_, JobsHost>, _call: HostCall<'_>) -> Completion<Value> {
    if let Some(interrupt) = &cx.host().interrupt {
        interrupt.interrupt();
    }
    Ok(Value::Undefined)
}

/// The interrupt while a microtask runs: the first of two calls a script function whose loop
/// without end calls, in its first turn, a host function that sets the flag: the checkpoint
/// answers `Terminated(Interrupted)` with the second still queued; the next checkpoint runs the
/// second only.
#[test]
fn an_interrupt_while_a_microtask_runs_ends_that_microtask() {
    let (mut agent, realm) = new_agent(limits());
    agent.host_mut().interrupt = Some(agent.interrupt_handle());
    define(&mut agent, &realm, &realm, "interruptNow", interrupt_now);
    run(
        &mut agent,
        &realm,
        "var first = true;\n\
         function spin() { for (;;) { if (first) { first = false; interruptNow(); } } }",
    );
    let spin = global(&mut agent, &realm, "spin");
    agent
        .queue_microtask(&realm, record_and_call(1, spin))
        .unwrap();
    agent.queue_microtask(&realm, record(2)).unwrap();
    let outcome = agent.perform_microtask_checkpoint().unwrap();
    assert_eq!(terminated(outcome), Termination::Interrupted);
    assert_eq!(agent.pending_microtasks(), Ok(1));
    assert_eq!(agent.host().seen, ["1: Err(Terminated(Interrupted))"]);
    next_checkpoint_runs(&mut agent, 1);
    assert_eq!(agent.host().records, [1, 2]);
}

/// The script function of the memory budget test: it allocates without end.
const GROW: &str = "function grow() { var a = []; for (;;) { a.push({}); } }";

/// The room the memory budget test leaves above what the agent holds before its checkpoint.
const MEMORY_ROOM: u64 = 16 * 1024;

/// A termination by the memory budget ends the checkpoint like any other: the second of three
/// microtasks calls a script function that allocates past the budget: `Terminated(MemoryBudget)`,
/// that microtask is not run again, one entry is queued, and the next checkpoint runs the third.
/// The budget is the bytes an agent holds after the same setup, read first in an agent of its
/// own, and [`MEMORY_ROOM`] more.
#[test]
fn a_memory_budget_termination_ends_the_checkpoint_and_keeps_the_rest_queued() {
    let (mut probe, probe_realm) = new_agent(limits());
    run(&mut probe, &probe_realm, GROW);
    let _grow = global(&mut probe, &probe_realm, "grow");
    assert!(matches!(probe.collect(), Ok(Outcome::Normal(_))));
    let held = probe.heap_bytes().unwrap();

    let mut limits = limits();
    limits.memory_bytes = held + MEMORY_ROOM;
    let (mut agent, realm) = new_agent(limits);
    run(&mut agent, &realm, GROW);
    let grow = global(&mut agent, &realm, "grow");
    agent.queue_microtask(&realm, record(1)).unwrap();
    agent
        .queue_microtask(&realm, record_and_call(2, grow))
        .unwrap();
    agent.queue_microtask(&realm, record(3)).unwrap();
    let outcome = agent.perform_microtask_checkpoint().unwrap();
    assert_eq!(terminated(outcome), Termination::MemoryBudget);
    assert_eq!(agent.host().records, [1, 2]);
    assert_eq!(agent.host().seen, ["2: Err(Terminated(MemoryBudget))"]);
    assert_eq!(agent.pending_microtasks(), Ok(1));
    assert_eq!(agent.state(), AgentState::Alive);
    let report = normal(agent.perform_microtask_checkpoint().unwrap());
    assert_eq!(report, host_microtasks(1));
    assert_eq!(agent.host().records, [1, 2, 3]);
}

/// The script function of HOST-196: a String of `n` code units, so that the bytes allocated
/// since the last collection pass the collector's threshold.
const BIG: &str = "function big(n) { return 'x'.repeat(n).length; }";

/// HOST-196: a checkpoint holds what a job was handed only until that job ends. Of three
/// microtasks, the first makes a host object, which the host keeps as an `ObjectRef` with no
/// root; the second calls a script function whose allocation starts a collection; the third
/// gets `Err(StaleHandle)` from `brand` of the kept handle. The checkpoint answers `Normal` with
/// `host_microtasks` 3 and the agent stays `Alive`. In the default mode the collection starts at
/// the safe point after an allocation that passes the collector's threshold, the live bytes
/// after the last collection and at least `MIN_TRIGGER_BYTES` (1 MiB, `heap/mod.rs`): the
/// String is made larger than both (the limits of the tests leave it the room); with a
/// collection at every safe point it starts at the first.
#[test]
fn host_196_a_checkpoint_holds_what_a_job_was_handed_only_until_it_ends() {
    // `repeat` spends steps by the code units it writes: the budget leaves room for the String.
    let (mut agent, realm) = new_agent(limits_with(1 << 24));
    run(&mut agent, &realm, BIG);
    let big = global(&mut agent, &realm, "big");
    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
    let length = agent.live_bytes().unwrap().max(1 << 20) + 64 * 1024;
    agent
        .queue_microtask(
            &realm,
            Box::new(|cx: &mut Cx<'_, JobsHost>| {
                let object = cx
                    .new_host_object(HostObjectSpec {
                        prototype: Prototype::Null,
                        brand: Brand(196),
                        data: None,
                        error_kind: None,
                    })
                    .unwrap()
                    .unwrap();
                cx.host_mut().kept = Some(object);
                let brand = cx.brand(object);
                cx.host_mut().seen.push(format!("1: {brand:?}"));
            }),
        )
        .unwrap();
    agent
        .queue_microtask(
            &realm,
            Box::new(move |cx: &mut Cx<'_, JobsHost>| {
                let f = cx.get(&big).unwrap();
                let answer = cx.call(f, Value::Undefined, &[Value::Number(length as f64)]);
                let made = answer.unwrap().unwrap() == Value::Number(length as f64);
                cx.host_mut().seen.push(format!("2: {made}"));
            }),
        )
        .unwrap();
    agent
        .queue_microtask(
            &realm,
            Box::new(|cx: &mut Cx<'_, JobsHost>| {
                let kept = cx.host().kept.unwrap();
                let brand = cx.brand(kept);
                cx.host_mut().seen.push(format!("3: {brand:?}"));
            }),
        )
        .unwrap();
    let report = normal(agent.perform_microtask_checkpoint().unwrap());
    assert_eq!(report, host_microtasks(3));
    assert_eq!(
        agent.host().seen,
        ["1: Ok(Some(Brand(196)))", "2: true", "3: Err(StaleHandle)"]
    );
    assert_eq!(agent.state(), AgentState::Alive);
}

/// Three closures that hold `shared` and would set `ran`, queued in `realm`.
fn queue_holders(
    agent: &mut Agent<JobsHost>,
    realm: &Realm,
    shared: &Rc<()>,
    ran: &Rc<Cell<bool>>,
) {
    for _ in 0..3 {
        let (held, flag) = (Rc::clone(shared), Rc::clone(ran));
        agent
            .queue_microtask(
                realm,
                Box::new(move |_cx: &mut Cx<'_, JobsHost>| {
                    let _held = &held;
                    flag.set(true);
                }),
            )
            .unwrap();
    }
    assert_eq!(Rc::strong_count(shared), 4);
}

/// Dropping the agent drops every queued closure unrun and calls nothing of the host: the
/// closures' `Rc` is back to one strong count, none of them ran, and the count of hook calls is
/// what it was before the drop.
#[test]
fn the_queue_is_dropped_with_the_agent() {
    let (mut agent, realm) = new_agent(limits());
    run(
        &mut agent,
        &realm,
        "var a = [1, 2, 3].map(function (x) { return x; });",
    );
    let shared = Rc::new(());
    let ran = Rc::new(Cell::new(false));
    queue_holders(&mut agent, &realm, &shared, &ran);
    let hooks = Rc::clone(&agent.host().hooks);
    let before = hooks.get();
    drop(realm);
    drop(agent);
    assert_eq!(Rc::strong_count(&shared), 1);
    assert!(!ran.get(), "no closure ran");
    assert_eq!(hooks.get(), before, "no hook was called");
}

/// `Agent::clear_microtasks` drops every queued host microtask unrun: three closures queued, it
/// answers `Ok(3)`, their `Rc` is back to one strong count, none ran, no hook was called,
/// `last_entry_steps()` is unchanged and nothing is queued; the next checkpoint answers
/// `Normal` with both counts 0. With nothing queued it answers `Ok(0)`.
#[test]
fn clear_microtasks_drops_every_host_microtask_unrun() {
    let (mut agent, realm) = new_agent(limits());
    run(&mut agent, &realm, "var a = 1 + 1;");
    let steps = agent.last_entry_steps().unwrap();
    assert!(steps > 0);
    let shared = Rc::new(());
    let ran = Rc::new(Cell::new(false));
    queue_holders(&mut agent, &realm, &shared, &ran);
    let before = agent.host().hooks.get();
    assert_eq!(agent.clear_microtasks(), Ok(3));
    assert_eq!(Rc::strong_count(&shared), 1);
    assert!(!ran.get(), "no closure ran");
    assert_eq!(agent.host().hooks.get(), before, "no hook was called");
    assert_eq!(agent.last_entry_steps(), Ok(steps));
    assert_eq!(agent.pending_microtasks(), Ok(0));
    assert_eq!(
        normal(agent.perform_microtask_checkpoint().unwrap()),
        host_microtasks(0)
    );
    assert_eq!(agent.clear_microtasks(), Ok(0));
}

/// A host microtask that queues a copy of itself and calls no code.
fn requeue(cx: &mut Cx<'_, JobsHost>) {
    cx.host_mut().count += 1;
    cx.queue_microtask(Box::new(requeue)).unwrap();
}

/// A checkpoint takes at most `steps_per_entry` entries off: a microtask that queues a copy of
/// itself and runs no code ends it in `Terminated(StepBudget)` with `last_entry_steps()` at
/// most N and one entry queued.
#[test]
fn a_microtask_that_queues_itself_ends_the_checkpoint_at_the_step_budget() {
    let budget = 200;
    let (mut agent, realm) = new_agent(limits_with(budget));
    agent.queue_microtask(&realm, Box::new(requeue)).unwrap();
    let outcome = agent.perform_microtask_checkpoint().unwrap();
    assert_eq!(terminated(outcome), Termination::StepBudget);
    let steps = agent.last_entry_steps().unwrap();
    assert!(steps <= budget, "{steps} steps of {budget}");
    assert_eq!(agent.host().count, steps, "one entry per step");
    assert_eq!(agent.pending_microtasks(), Ok(1));
}

/// A host function of the test host that queues a microtask calling its first argument with
/// no arguments, then runs a checkpoint through `cx` inside the running entry. The microtask
/// records what the call answered in the host's `calls`, and the host function then records
/// what the checkpoint answered.
fn checkpoint_call(cx: &mut Cx<'_, TestHost>, call: HostCall<'_>) -> Completion<Value> {
    let function = call.args.first().copied().unwrap_or(Value::Undefined);
    cx.queue_microtask(Box::new(move |cx: &mut Cx<'_, TestHost>| {
        let line = match cx.call(function, Value::Undefined, &[]).unwrap() {
            Ok(value) => format!("value {}", show(cx, value)),
            Err(Abrupt::Throw(thrown)) => format!("threw {}", show_thrown(cx, thrown)),
            Err(abrupt) => format!("{abrupt:?}"),
        };
        cx.host_mut().calls.push(line);
    }))
    .unwrap();
    let report = cx.perform_microtask_checkpoint();
    cx.host_mut().calls.push(format!("checkpoint {report:?}"));
    report.map(|_| Value::Undefined)
}

/// What the test host's `calls` hold after a nested checkpoint whose microtask's call answered
/// `first`.
fn nested(first: &str) -> Vec<String> {
    vec![
        first.to_string(),
        "checkpoint Ok(CheckpointReport { promise_jobs: 0, host_microtasks: 1 })".to_string(),
    ]
}

/// The script of the bound tests: `f(k)` reaches `checkpointCall(g)` through `k` levels of
/// `next`, which is `reenter` (a re-entry each) or a plain call (a frame each).
fn through(next: &str, k: u32) -> String {
    format!(
        "function g() {{ return 7; }}\n\
         function f(k) {{\n\
           if (k === 0) {{ return checkpointCall(g); }}\n\
           return {next};\n\
         }}\n\
         f({k});"
    )
}

/// The re-entry limit of [`a_microtask_of_a_nested_checkpoint_counts_against_reentry_depth`].
const REENTRY: u32 = 16;

/// A microtask that a nested checkpoint runs calls into code above the frames of its caller,
/// counted against `reentry_depth` as every call from a host function is: a script recurses
/// through `reenter` (of `support/stack.rs`) to the deepest level the limit lets it reach, then
/// calls a host function that queues a microtask calling a script function and runs
/// `cx.perform_microtask_checkpoint`: the microtask's call throws a `RangeError`; one level
/// shallower it answers the function's value; the checkpoint answers `Ok` in both. One level
/// deeper the recursion itself throws, so the first level is the deepest.
#[test]
fn a_microtask_of_a_nested_checkpoint_counts_against_reentry_depth() {
    let reenter = "reenter(function () { return f(k - 1); })";
    for (k, expected) in [
        (REENTRY, nested("threw RangeError")),
        (REENTRY - 1, nested("value 7")),
        (REENTRY + 1, Vec::new()),
    ] {
        let (mut agent, realm) = stack_agent(REENTRY);
        define_function(
            &mut agent,
            &realm,
            "checkpointCall",
            1,
            checkpoint_call,
            false,
        );
        let outcome = support::script::eval_text(&mut agent, &realm, &through(reenter, k));
        let shown = if expected.is_empty() {
            support::script::Outcome::Throw("RangeError".to_string())
        } else {
            support::script::Outcome::Normal("undefined".to_string())
        };
        assert_eq!(outcome, shown, "at level {k}");
        assert_eq!(agent.host().calls, expected, "at level {k}");
    }
}

/// The frame limit of [`a_microtask_of_a_nested_checkpoint_counts_against_frame_depth`].
const FRAMES: u32 = 32;

/// The same against `frame_depth`: a script recurses to the deepest frame the limit lets it
/// push (the script's own frame and `k + 1` of `f` fill it), then the microtask's call throws a
/// `RangeError`; one frame shallower it answers the function's value. One frame deeper the
/// recursion itself throws.
#[test]
fn a_microtask_of_a_nested_checkpoint_counts_against_frame_depth() {
    let mut limits = limits();
    limits.frame_depth = FRAMES;
    for (k, expected) in [
        (FRAMES - 2, nested("threw RangeError")),
        (FRAMES - 3, nested("value 7")),
        (FRAMES - 1, Vec::new()),
    ] {
        let mut agent = support::host::agent_with(limits).unwrap();
        let realm = support::host::realm(&mut agent);
        define_function(
            &mut agent,
            &realm,
            "checkpointCall",
            1,
            checkpoint_call,
            false,
        );
        let outcome = support::script::eval_text(&mut agent, &realm, &through("f(k - 1)", k));
        let shown = if expected.is_empty() {
            support::script::Outcome::Throw("RangeError".to_string())
        } else {
            support::script::Outcome::Normal("undefined".to_string())
        };
        assert_eq!(outcome, shown, "at level {k}");
        assert_eq!(agent.host().calls, expected, "at level {k}");
    }
}
