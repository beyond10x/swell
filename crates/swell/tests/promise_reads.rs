//! The rejection tracker and the host's reads of a promise (`docs/design/host-interface.md`,
//! section 13, "The read surface", and section 16, "The hook trait"; ECMA-262
//! `#sec-rejectpromise`, step 7, and `#sec-performpromisethen`, step 11.c): HOST-46, HOST-100,
//! HOST-101 and HOST-121, with HOST-48 and HOST-102 for the three reads. Written by
//! `story:promise-rejection-tracker-and-promise-reads`.
//!
//! The host of these tests records each call of `host_promise_rejection_tracker` with its
//! operation, a root of its promise and what the reads of its `HookCx` answered then, and counts
//! the calls of every other hook it is handed. No script uses `async` or `await`: a rejection is
//! reached through `Promise.reject`, a rejecting or throwing executor, a `then` whose handler
//! throws, or the reject function of `Cx::new_promise_capability`. Run also with the feature
//! `collect-every-safe-point`.
#![cfg(test)]

use crate::support;

use support::host::{limits, seeds};
use swell::{
    Agent, HookCx, Host, HostError, Job, LoadRequest, ObjectRef, Outcome, Parsed, PromiseState,
    Realm, RealmRef, RejectionOperation, Root, Script, Source, Value, ValueKind,
};

/// One call of `host_promise_rejection_tracker`: the operation, a root of the promise, and what
/// `promise_state` and `promise_is_handled` of the hook's `HookCx` answered for it.
struct Tracked {
    operation: RejectionOperation,
    promise: Root,
    read: String,
}

/// The host of these tests.
#[derive(Default)]
struct ReadsHost {
    tracked: Vec<Tracked>,
    /// The calls of every other hook.
    hooks: u32,
}

/// What `state` of a promise shows: `Pending`, or the settlement with the number its result is.
fn shown(state: Result<PromiseState, HostError>, read: impl Fn(&Root) -> Value) -> String {
    match state {
        Ok(PromiseState::Pending) => "Pending".to_string(),
        Ok(PromiseState::Fulfilled(result)) => format!("Fulfilled({:?})", read(&result)),
        Ok(PromiseState::Rejected(result)) => format!("Rejected({:?})", read(&result)),
        Err(error) => format!("Err({error:?})"),
    }
}

impl Host for ReadsHost {
    fn host_make_job_callback(
        cx: &mut HookCx<'_, Self>,
        _callback: ObjectRef,
    ) -> Option<swell::HostDefined> {
        cx.host_mut().hooks += 1;
        None
    }
    fn host_enqueue_promise_job(cx: &mut HookCx<'_, Self>, _realm: Option<RealmRef>) {
        cx.host_mut().hooks += 1;
    }
    fn host_prepare_to_run_job(cx: &mut HookCx<'_, Self>, _realm: RealmRef) {
        cx.host_mut().hooks += 1;
    }
    fn host_clean_up_after_job(
        cx: &mut HookCx<'_, Self>,
        _realm: Option<RealmRef>,
        _result: Result<(), Value>,
    ) {
        cx.host_mut().hooks += 1;
    }
    fn host_enqueue_generic_job(cx: &mut HookCx<'_, Self>, _job: Job) {
        cx.host_mut().hooks += 1;
    }
    fn host_enqueue_timeout_job(cx: &mut HookCx<'_, Self>, _job: Job, _milliseconds: f64) {
        cx.host_mut().hooks += 1;
    }
    fn host_enqueue_finalization_registry_cleanup_job(cx: &mut HookCx<'_, Self>, _job: Job) {
        cx.host_mut().hooks += 1;
    }
    fn host_promise_rejection_tracker(
        cx: &mut HookCx<'_, Self>,
        promise: ObjectRef,
        operation: RejectionOperation,
    ) {
        let promise = cx.root(Value::Object(promise)).unwrap();
        let state = cx.promise_state(&promise);
        let state = match state {
            Ok(PromiseState::Rejected(result)) => format!("Rejected({:?})", cx.get(&result)),
            other => format!("{other:?}"),
        };
        let read = format!("{state} {:?}", cx.promise_is_handled(&promise));
        cx.host_mut().tracked.push(Tracked {
            operation,
            promise,
            read,
        });
    }
    fn host_load_imported_module(cx: &mut HookCx<'_, Self>, _request: LoadRequest) {
        cx.host_mut().hooks += 1;
    }
    fn host_system_utc_epoch_nanoseconds(cx: &mut HookCx<'_, Self>, _global: ObjectRef) -> i128 {
        cx.host_mut().hooks += 1;
        0
    }
}

/// An agent of the test host with a realm.
fn agent() -> (Agent<ReadsHost>, Realm) {
    let mut agent = Agent::new(ReadsHost::default(), limits(), seeds()).unwrap();
    let realm = match support::host::create_realm(&mut agent, None, None).unwrap() {
        Outcome::Normal(realm) => realm,
        other => panic!("create_realm answered {other:?}"),
    };
    (agent, realm)
}

/// `parse_script` of `source`, which must parse.
fn parse(agent: &mut Agent<ReadsHost>, realm: &Realm, source: &str) -> Script {
    match agent
        .parse_script(realm, Source::Utf8(source), None)
        .unwrap()
    {
        Parsed::Parsed(script) => script,
        other => panic!("{source:?} answered {other:?}"),
    }
}

/// The completion value of the script `source`, which must complete normally.
fn run(agent: &mut Agent<ReadsHost>, realm: &Realm, source: &str) -> Root {
    let script = parse(agent, realm, source);
    match agent.evaluate_script(&script).unwrap() {
        Outcome::Normal(root) => root,
        other => panic!("{source:?} answered {other:?}"),
    }
}

/// A root of `value`.
fn root(agent: &mut Agent<ReadsHost>, realm: &Realm, value: Value) -> Root {
    agent.with_cx(realm, |cx| cx.root(value).unwrap()).unwrap()
}

/// The value `root` holds.
fn value(agent: &mut Agent<ReadsHost>, realm: &Realm, root: &Root) -> Value {
    agent.with_cx(realm, |cx| cx.get(root).unwrap()).unwrap()
}

/// Whether two roots hold the same value.
fn same(agent: &mut Agent<ReadsHost>, realm: &Realm, a: &Root, b: &Root) -> bool {
    agent
        .with_cx(realm, |cx| cx.get(a).unwrap() == cx.get(b).unwrap())
        .unwrap()
}

/// What `Agent::promise_state` of `promise` answers, shown.
fn state(agent: &mut Agent<ReadsHost>, realm: &Realm, promise: &Root) -> String {
    let answer = agent.promise_state(promise);
    let results = match &answer {
        Ok(PromiseState::Fulfilled(result) | PromiseState::Rejected(result)) => {
            Some(value(agent, realm, result))
        }
        _ => None,
    };
    shown(answer, |_| results.unwrap_or(Value::Undefined))
}

/// The calls of the tracker, as (operation, what the hook read) pairs.
fn tracked(agent: &Agent<ReadsHost>) -> Vec<(RejectionOperation, String)> {
    agent
        .host()
        .tracked
        .iter()
        .map(|call| (call.operation, call.read.clone()))
        .collect()
}

/// Whether call `i` of the tracker was given the promise `promise` holds.
fn tracked_promise(agent: &mut Agent<ReadsHost>, realm: &Realm, i: usize, promise: &Root) -> bool {
    let mut held = std::mem::take(&mut agent.host_mut().tracked);
    let answer = same(agent, realm, &held[i].promise, promise);
    held.append(&mut agent.host_mut().tracked);
    agent.host_mut().tracked = held;
    answer
}

/// A checkpoint that must answer `Normal`.
fn checkpoint(agent: &mut Agent<ReadsHost>) {
    match agent.perform_microtask_checkpoint().unwrap() {
        Outcome::Normal(_) => {}
        other => panic!("the checkpoint answered {other:?}"),
    }
}

/// The answer `Err(WrongKind { expected: Promise })`.
const WRONG_KIND: HostError = HostError::WrongKind {
    expected: ValueKind::Promise,
};

/// HOST-46: `promise_state` of a root that reads a value that is no promise answers
/// `Err(WrongKind { expected: Promise })`, and so do `promise_is_handled` and
/// `mark_promise_as_handled`, on `Agent` and on `Cx`: a primitive, a plain object, a function, a
/// thenable, `%Promise%` and `%Promise.prototype%`, and an object whose prototype is a promise.
/// No hook is called.
#[test]
fn host_46_promise_state_of_a_value_that_is_no_promise_is_wrong_kind() {
    let (mut agent, realm) = agent();
    let sources = [
        "undefined",
        "1",
        "'p'",
        "({})",
        "(function () {})",
        "({ then: function (resolve) { resolve(1); } })",
        "Promise",
        "Promise.prototype",
        "Object.create(Promise.resolve(1))",
    ];
    let values: Vec<Root> = sources
        .iter()
        .map(|source| run(&mut agent, &realm, source))
        .collect();
    let hooks = agent.host().hooks;
    for (source, value) in sources.iter().zip(&values) {
        assert!(
            matches!(agent.promise_state(value), Err(WRONG_KIND)),
            "promise_state of {source}"
        );
        assert_eq!(
            agent.promise_is_handled(value),
            Err(WRONG_KIND),
            "promise_is_handled of {source}"
        );
        assert_eq!(
            agent.mark_promise_as_handled(value),
            Err(WRONG_KIND),
            "mark_promise_as_handled of {source}"
        );
        let scoped = agent
            .with_cx(&realm, |cx| {
                (
                    matches!(cx.promise_state(value), Err(WRONG_KIND)),
                    cx.promise_is_handled(value),
                    cx.mark_promise_as_handled(value),
                )
            })
            .unwrap();
        assert_eq!(
            scoped,
            (true, Err(WRONG_KIND), Err(WRONG_KIND)),
            "the reads of Cx of {source}"
        );
    }
    assert_eq!(agent.host().hooks, hooks, "no hook was called");
    assert!(agent.host().tracked.is_empty(), "nothing was tracked");
}

/// HOST-100: a promise of `Cx::new_promise_capability` reads `Pending` and not handled; after
/// `Agent::call` of its reject function with 7 it reads `Rejected(7)` and still not handled, and
/// the tracker was called once with `Reject`; after a script added a `catch` to it, it reads
/// handled, on `Agent` and on `Cx`, and the tracker was called with `Handle` while the flag was
/// still false (`#sec-performpromisethen`, step 11.c, before step 12). A promise fulfilled with 5
/// reads `Fulfilled(5)`, is not tracked, and reads handled after a `then`.
#[test]
fn host_100_promise_state_and_is_handled_follow_the_promise() {
    let (mut agent, realm) = agent();
    let (promise, resolve, reject, settled, settle) = agent
        .with_cx(&realm, |cx| {
            let made = cx.new_promise_capability().unwrap();
            let other = cx.new_promise_capability().unwrap();
            let promise = cx.root(Value::Object(made.promise)).unwrap();
            let pending = (
                format!("{:?}", cx.promise_state(&promise)),
                cx.promise_is_handled(&promise),
            );
            assert_eq!(pending, ("Ok(Pending)".to_string(), Ok(false)));
            (
                promise,
                cx.root(made.resolve).unwrap(),
                cx.root(made.reject).unwrap(),
                cx.root(Value::Object(other.promise)).unwrap(),
                cx.root(other.resolve).unwrap(),
            )
        })
        .unwrap();
    drop(resolve);
    assert_eq!(state(&mut agent, &realm, &promise), "Pending");
    assert_eq!(agent.promise_is_handled(&promise), Ok(false));

    let undefined = root(&mut agent, &realm, Value::Undefined);
    let seven = root(&mut agent, &realm, Value::Number(7.0));
    assert!(matches!(
        agent.call(&realm, &reject, &undefined, &[&seven]),
        Ok(Outcome::Normal(_))
    ));
    assert_eq!(state(&mut agent, &realm, &promise), "Rejected(Number(7.0))");
    assert_eq!(agent.promise_is_handled(&promise), Ok(false));
    assert_eq!(
        tracked(&agent),
        [(
            RejectionOperation::Reject,
            "Rejected(Ok(Number(7.0))) Ok(false)".to_string()
        )]
    );
    assert!(tracked_promise(&mut agent, &realm, 0, &promise));

    let add_catch = run(
        &mut agent,
        &realm,
        "(function (p) { p.catch(function () {}); })",
    );
    assert!(matches!(
        agent.call(&realm, &add_catch, &undefined, &[&promise]),
        Ok(Outcome::Normal(_))
    ));
    assert_eq!(agent.promise_is_handled(&promise), Ok(true));
    let scoped = agent
        .with_cx(&realm, |cx| cx.promise_is_handled(&promise))
        .unwrap();
    assert_eq!(scoped, Ok(true));
    assert_eq!(state(&mut agent, &realm, &promise), "Rejected(Number(7.0))");
    let calls = tracked(&agent);
    assert_eq!(calls.len(), 2, "{calls:?}");
    assert_eq!(
        calls[1],
        (
            RejectionOperation::Handle,
            "Rejected(Ok(Number(7.0))) Ok(false)".to_string()
        )
    );
    assert!(tracked_promise(&mut agent, &realm, 1, &promise));

    let five = root(&mut agent, &realm, Value::Number(5.0));
    assert!(matches!(
        agent.call(&realm, &settle, &undefined, &[&five]),
        Ok(Outcome::Normal(_))
    ));
    assert_eq!(
        state(&mut agent, &realm, &settled),
        "Fulfilled(Number(5.0))"
    );
    assert_eq!(agent.promise_is_handled(&settled), Ok(false));
    let add_then = run(
        &mut agent,
        &realm,
        "(function (p) { p.then(function () {}); })",
    );
    assert!(matches!(
        agent.call(&realm, &add_then, &undefined, &[&settled]),
        Ok(Outcome::Normal(_))
    ));
    assert_eq!(agent.promise_is_handled(&settled), Ok(true));
    checkpoint(&mut agent);
    assert_eq!(
        tracked(&agent).len(),
        2,
        "the fulfilled promise was not tracked"
    );
}

/// HOST-101: after `mark_promise_as_handled` of a rejected promise with no handler,
/// `promise_is_handled` reads true, the mark called no hook, and a handler added to it in a
/// later entry causes no call of the tracker with `Handle`. A promise marked through `Cx` before
/// it is rejected is not tracked at all (`#sec-rejectpromise`, step 7).
#[test]
fn host_101_a_promise_marked_as_handled_is_not_tracked_again() {
    let (mut agent, realm) = agent();
    let p = run(&mut agent, &realm, "var p = Promise.reject(2); p");
    assert_eq!(tracked(&agent).len(), 1);
    assert_eq!(tracked(&agent)[0].0, RejectionOperation::Reject);
    assert_eq!(agent.promise_is_handled(&p), Ok(false));
    let hooks = agent.host().hooks;
    assert_eq!(agent.mark_promise_as_handled(&p), Ok(()));
    assert_eq!(agent.promise_is_handled(&p), Ok(true));
    assert_eq!(agent.host().hooks, hooks, "the mark called no hook");
    assert_eq!(tracked(&agent).len(), 1, "the mark called no tracker");
    run(&mut agent, &realm, "p.then(undefined, function () {}); 0");
    run(&mut agent, &realm, "p.catch(function () {}); 0");
    checkpoint(&mut agent);
    assert_eq!(state(&mut agent, &realm, &p), "Rejected(Number(2.0))");
    assert_eq!(
        tracked(&agent).len(),
        1,
        "no call with Handle: {:?}",
        tracked(&agent)
    );

    // Marked through `Cx` while pending, then rejected: RejectPromise does not call the tracker.
    let q = run(
        &mut agent,
        &realm,
        "var rejectQ; var q = new Promise(function (_, reject) { rejectQ = reject; }); q",
    );
    let marked = agent
        .with_cx(&realm, |cx| {
            (cx.mark_promise_as_handled(&q), cx.promise_is_handled(&q))
        })
        .unwrap();
    assert_eq!(marked, (Ok(()), Ok(true)));
    run(&mut agent, &realm, "rejectQ(3); 0");
    assert_eq!(state(&mut agent, &realm, &q), "Rejected(Number(3.0))");
    assert_eq!(
        tracked(&agent).len(),
        1,
        "the marked promise was not tracked"
    );
}

/// HOST-121: the script `Promise.reject(1)` calls the tracker once, with `Reject` and that
/// promise; adding a handler to it in a later entry calls it once with `Handle` and that
/// promise, and a second handler calls nothing. A rejecting executor and a throwing one each
/// make one call with `Reject`; a promise that has a handler when it is rejected is not tracked.
#[test]
fn host_121_a_rejection_with_no_handler_is_tracked_and_a_later_handler_withdraws_it() {
    let (mut agent, realm) = agent();
    let p = run(&mut agent, &realm, "var p = Promise.reject(1); p");
    assert_eq!(
        tracked(&agent),
        [(
            RejectionOperation::Reject,
            "Rejected(Ok(Number(1.0))) Ok(false)".to_string()
        )]
    );
    assert!(tracked_promise(&mut agent, &realm, 0, &p));

    run(&mut agent, &realm, "p.catch(function () {}); 0");
    let calls = tracked(&agent);
    assert_eq!(calls.len(), 2, "{calls:?}");
    assert_eq!(
        calls[1],
        (
            RejectionOperation::Handle,
            "Rejected(Ok(Number(1.0))) Ok(false)".to_string()
        )
    );
    assert!(tracked_promise(&mut agent, &realm, 1, &p));
    run(&mut agent, &realm, "p.catch(function () {}); 0");
    checkpoint(&mut agent);
    assert_eq!(tracked(&agent).len(), 2, "a second handler is not tracked");

    let executors = run(
        &mut agent,
        &realm,
        "var a = new Promise(function (_, reject) { reject(4); });\n\
         var b = new Promise(function () { throw 5; });\n\
         var resolveC, rejectC;\n\
         var c = new Promise(function (resolve, reject) { rejectC = reject; });\n\
         c.then(undefined, function () {});\n\
         rejectC(6); [a, b]",
    );
    let calls = tracked(&agent);
    assert_eq!(
        calls.get(2..),
        Some(
            &[
                (
                    RejectionOperation::Reject,
                    "Rejected(Ok(Number(4.0))) Ok(false)".to_string()
                ),
                (
                    RejectionOperation::Reject,
                    "Rejected(Ok(Number(5.0))) Ok(false)".to_string()
                ),
            ][..]
        ),
        "a rejecting and a throwing executor; c had a handler"
    );
    drop(executors);
    checkpoint(&mut agent);
    assert_eq!(tracked(&agent).len(), 4);
}

/// A rejection inside a job is tracked once: the promise a `then` derives, whose handler throws,
/// is rejected by the job of the reaction, which calls the tracker with `Reject` for it, once,
/// and no later checkpoint calls it again. A rejected promise whose `then` has no reject
/// handler is tracked with `Reject`, then `Handle` by the `then`, and its derived promise with
/// `Reject` in the job. A call of the tracker adds no step: the same entry `reject(1)` spends
/// the same steps for a promise the host marked as handled (no call) and for one it did not
/// (one call).
#[test]
fn a_rejection_inside_a_job_is_tracked_once() {
    let (mut agent, realm) = agent();
    let d = run(
        &mut agent,
        &realm,
        "var d = Promise.resolve(1).then(function () { throw 6; }); d",
    );
    assert!(
        tracked(&agent).is_empty(),
        "nothing is rejected before the job"
    );
    checkpoint(&mut agent);
    assert_eq!(
        tracked(&agent),
        [(
            RejectionOperation::Reject,
            "Rejected(Ok(Number(6.0))) Ok(false)".to_string()
        )]
    );
    assert!(tracked_promise(&mut agent, &realm, 0, &d));
    assert_eq!(state(&mut agent, &realm, &d), "Rejected(Number(6.0))");
    checkpoint(&mut agent);
    assert_eq!(tracked(&agent).len(), 1, "tracked once");

    let e = run(
        &mut agent,
        &realm,
        "var e = Promise.reject(7).then(function () {}); e",
    );
    let operations: Vec<RejectionOperation> = tracked(&agent).iter().map(|call| call.0).collect();
    assert_eq!(
        operations,
        [
            RejectionOperation::Reject,
            RejectionOperation::Reject,
            RejectionOperation::Handle
        ]
    );
    checkpoint(&mut agent);
    assert_eq!(tracked(&agent).len(), 4);
    assert_eq!(
        tracked(&agent)[3],
        (
            RejectionOperation::Reject,
            "Rejected(Ok(Number(7.0))) Ok(false)".to_string()
        )
    );
    assert!(tracked_promise(&mut agent, &realm, 3, &e));

    let pair = run(
        &mut agent,
        &realm,
        "var rejectF, rejectG;\n\
         var f = new Promise(function (_, reject) { rejectF = reject; });\n\
         var g = new Promise(function (_, reject) { rejectG = reject; }); [f, g]",
    );
    drop(pair);
    let f = run(&mut agent, &realm, "f");
    assert_eq!(agent.mark_promise_as_handled(&f), Ok(()));
    run(&mut agent, &realm, "rejectF(1)");
    let unmarked = agent.last_entry_steps().unwrap();
    assert_eq!(
        tracked(&agent).len(),
        4,
        "the marked promise was not tracked"
    );
    run(&mut agent, &realm, "rejectG(1)");
    let tracked_steps = agent.last_entry_steps().unwrap();
    assert_eq!(tracked(&agent).len(), 5, "the other one was");
    assert_eq!(tracked_steps, unmarked, "the tracker's call spent no step");
}

/// The reads call no hook and change no reading (with HOST-102): after a collection,
/// `promise_state`, `promise_is_handled` and `mark_promise_as_handled` of a pending, a fulfilled
/// and a rejected promise, each twice, leave `last_entry_steps()`, `heap_bytes()` and
/// `live_bytes()` as they read before them and call no hook; the root `promise_state` makes for a
/// settled promise's result is the host's storage, charged to neither budget. The reads answer
/// the same each time.
#[test]
fn the_reads_call_no_hook_and_change_no_reading() {
    let (mut agent, realm) = agent();
    let promises = [
        run(&mut agent, &realm, "new Promise(function () {})"),
        run(&mut agent, &realm, "Promise.resolve(5)"),
        run(&mut agent, &realm, "Promise.reject(6)"),
    ];
    run(
        &mut agent,
        &realm,
        "var n = 0; for (var i = 0; i < 10; i++) { n += i; } n",
    );
    let steps = agent.last_entry_steps().unwrap();
    assert!(steps > 0);
    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
    let bytes = agent.heap_bytes().unwrap();
    let live = agent.live_bytes().unwrap();
    let hooks = agent.host().hooks;
    let tracked_before = tracked(&agent).len();

    let mut first = Vec::new();
    for round in 0..2 {
        let mut answers = Vec::new();
        for promise in &promises {
            let state = match agent.promise_state(promise) {
                Ok(PromiseState::Pending) => "Pending".to_string(),
                Ok(PromiseState::Fulfilled(result)) => {
                    format!("Fulfilled({})", agent.duplicate(&result).is_ok())
                }
                Ok(PromiseState::Rejected(result)) => {
                    format!("Rejected({})", agent.duplicate(&result).is_ok())
                }
                Err(error) => format!("{error:?}"),
            };
            answers.push(format!(
                "{state} {:?} {:?} {:?}",
                agent.promise_is_handled(promise),
                agent.mark_promise_as_handled(promise),
                agent.promise_is_handled(promise),
            ));
        }
        assert_eq!(agent.last_entry_steps(), Ok(steps), "round {round}");
        assert_eq!(agent.heap_bytes(), Ok(bytes), "round {round}");
        assert_eq!(agent.live_bytes(), Ok(live), "round {round}");
        if round == 0 {
            assert_eq!(
                answers,
                [
                    "Pending Ok(false) Ok(()) Ok(true)",
                    "Fulfilled(true) Ok(false) Ok(()) Ok(true)",
                    "Rejected(true) Ok(false) Ok(()) Ok(true)",
                ]
            );
            first = answers;
        } else {
            let again: Vec<String> = first
                .iter()
                .map(|answer| answer.replacen("Ok(false)", "Ok(true)", 1))
                .collect();
            assert_eq!(answers, again, "the reads answer the same");
        }
    }
    assert_eq!(agent.host().hooks, hooks, "no hook was called");
    assert_eq!(tracked(&agent).len(), tracked_before, "no tracker call");
    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
    assert_eq!(agent.heap_bytes(), Ok(bytes));
    assert_eq!(agent.live_bytes(), Ok(live));
    assert_eq!(
        state(&mut agent, &realm, &promises[2]),
        "Rejected(Number(6.0))"
    );
}

/// A read of a root of another agent answers `Err(ForeignHandle)` on `Agent` and on `Cx`, for
/// each of the three reads, and changes nothing: the promise of the other agent is still not
/// handled, and no hook of either agent was called.
#[test]
fn a_read_of_a_foreign_root_is_foreign_handle() {
    let (mut a, realm_a) = agent();
    let (mut b, realm_b) = agent();
    let p = run(&mut a, &realm_a, "new Promise(function () {})");
    let hooks = (a.host().hooks, b.host().hooks);
    let foreign = HostError::ForeignHandle;
    assert!(matches!(b.promise_state(&p), Err(HostError::ForeignHandle)));
    assert_eq!(b.promise_is_handled(&p), Err(foreign));
    assert_eq!(b.mark_promise_as_handled(&p), Err(foreign));
    let scoped = b
        .with_cx(&realm_b, |cx| {
            (
                matches!(cx.promise_state(&p), Err(HostError::ForeignHandle)),
                cx.promise_is_handled(&p),
                cx.mark_promise_as_handled(&p),
            )
        })
        .unwrap();
    assert_eq!(scoped, (true, Err(foreign), Err(foreign)));
    assert_eq!(
        a.promise_is_handled(&p),
        Ok(false),
        "the mark changed nothing"
    );
    assert_eq!(state(&mut a, &realm_a, &p), "Pending");
    assert_eq!(
        (a.host().hooks, b.host().hooks),
        hooks,
        "no hook was called"
    );
    assert!(a.host().tracked.is_empty() && b.host().tracked.is_empty());
}
