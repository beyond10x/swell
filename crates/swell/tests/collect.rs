//! The collector and the root table (`docs/design/host-interface.md`, decisions 4 and 18;
//! `engine.md`, decision 4, "Collector"). Run also with the feature `collect-every-safe-point`.

use crate::support;

use swell::{Brand, HostData, HostError, HostObjectSpec, Outcome, Prototype, Tracer, Value};

const BRAND: Brand = Brand(1);

/// Host data of a known size, so that freeing it lowers the heap's bytes by a visible amount.
struct Payload {
    _bytes: [u8; 256],
}

impl HostData for Payload {
    fn trace(&self, _tracer: &mut Tracer<'_>) {}
}

fn spec(data: Option<Box<dyn HostData>>) -> HostObjectSpec {
    HostObjectSpec {
        prototype: Prototype::Null,
        brand: BRAND,
        data,
        error_kind: None,
    }
}

/// HOST-28: an object held only by a `Root` survives `collect`, and the root then reads the
/// same object; an object nothing holds is freed by the same collection.
#[test]
fn host_28_an_object_held_only_by_a_root_survives_collect() {
    let mut agent = support::host::agent().unwrap();
    let realm = support::host::realm(&mut agent);
    let (root, kept, dropped) = agent
        .with_cx(&realm, |cx| {
            let kept = cx.new_host_object(spec(None)).unwrap().unwrap();
            let dropped = cx.new_host_object(spec(None)).unwrap().unwrap();
            (cx.root(Value::Object(kept)).unwrap(), kept, dropped)
        })
        .unwrap();

    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));

    let (read, brand, gone) = agent
        .with_cx(&realm, |cx| {
            let read = cx.get(&root).unwrap();
            (read, cx.brand(kept), cx.brand(dropped))
        })
        .unwrap();
    assert_eq!(read, Value::Object(kept));
    assert_eq!(brand, Ok(Some(BRAND)));
    assert_eq!(gone, Err(HostError::StaleHandle));
}

/// HOST-29: after the host drops the last `Root` of an object nothing else reaches, the second
/// `collect` after the drop reports a lower `live_bytes()` than before the drop.
#[test]
fn host_29_dropping_the_last_root_lowers_heap_bytes() {
    let mut agent = support::host::agent().unwrap();
    let realm = support::host::realm(&mut agent);
    let root = agent
        .with_cx(&realm, |cx| {
            let object = cx
                .new_host_object(spec(Some(Box::new(Payload { _bytes: [0; 256] }))))
                .unwrap()
                .unwrap();
            cx.root(Value::Object(object)).unwrap()
        })
        .unwrap();
    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
    let before = agent.live_bytes().unwrap();

    drop(root);
    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
    let after = agent.live_bytes().unwrap();
    assert!(
        after < before,
        "live_bytes {after} after the drop, {before} before it"
    );
    assert!(
        before - after >= 256,
        "{before} - {after} does not cover the host data"
    );
}

/// HOST-132: `collect` answers a report whose `heap_bytes_after` equals `heap_bytes()` read
/// after it, and whose `heap_bytes_before` is the reading before it.
#[test]
fn host_132_report_matches_heap_bytes() {
    let mut agent = support::host::agent().unwrap();
    let realm = support::host::realm(&mut agent);
    let _root = agent
        .with_cx(&realm, |cx| {
            for _ in 0..10 {
                cx.new_string("garbage nothing holds").unwrap();
            }
            let kept = cx.new_string("kept").unwrap();
            cx.root(Value::String(kept)).unwrap()
        })
        .unwrap();
    let before = agent.heap_bytes().unwrap();
    let Ok(Outcome::Normal(report)) = agent.collect() else {
        panic!("collect did not answer Normal");
    };
    assert_eq!(report.heap_bytes_before, before);
    assert_eq!(report.heap_bytes_after, agent.heap_bytes().unwrap());
    assert!(
        report.heap_bytes_after > 0,
        "the rooted string is still charged"
    );
    assert!(report.heap_bytes_after <= report.heap_bytes_before);

    // A second collection with nothing to free reports the same number twice.
    let Ok(Outcome::Normal(again)) = agent.collect() else {
        panic!("collect did not answer Normal");
    };
    assert_eq!(again.heap_bytes_before, again.heap_bytes_after);
    assert_eq!(again.heap_bytes_after, agent.heap_bytes().unwrap());
}

/// HOST-171: at the return of every entry and of `collect`, `live_bytes()` reads at most
/// `heap_bytes()` and `heap_bytes()` at most `memory_bytes` M, the realm's bytes plus 4 MiB.
/// After `(function () { var a = []; while (true) a.push({}); })()` answers
/// `Terminated(MemoryBudget)`, two `collect`s lower `live_bytes()` by more than they lower
/// `heap_bytes()` (the freed objects' slots stay charged); then, with k taken from the readings
/// so that `live_bytes()` + 2 × k ≤ M < `heap_bytes()` + k, `"x".repeat(k).length` answers
/// `Terminated(MemoryBudget)`: the budget bounds the charged reading, not the live one. With
/// the feature `collect-every-safe-point` the room is 64 KiB: a collection at every safe point
/// traces the whole heap, so filling 4 MiB would cost time that grows with the square of the
/// heap (as HOST-10 measures), and the readings relate the same way at any size.
#[test]
fn host_171_live_bytes_reads_the_records_and_heap_bytes_the_budget() {
    let realm_bytes = {
        let mut agent = support::host::agent().unwrap();
        let _realm = support::host::realm(&mut agent);
        agent.heap_bytes().unwrap()
    };
    let room: u64 = if cfg!(feature = "collect-every-safe-point") {
        64 * 1024
    } else {
        4 * 1024 * 1024
    };
    let mut limits = support::host::limits();
    limits.memory_bytes = realm_bytes + room;
    limits.steps_per_entry = u64::MAX;
    let budget = limits.memory_bytes;
    let mut agent = support::host::agent_with(limits).unwrap();
    let realm = support::host::realm(&mut agent);
    let readings = |agent: &swell::Agent<support::host::TestHost>, after: &str| {
        let live = agent.live_bytes().unwrap();
        let heap = agent.heap_bytes().unwrap();
        assert!(
            live <= heap,
            "{after}: live_bytes {live} > heap_bytes {heap}"
        );
        assert!(
            heap <= budget,
            "{after}: heap_bytes {heap} > memory_bytes {budget}"
        );
        (live, heap)
    };
    readings(&agent, "create_realm");

    assert_eq!(
        support::script::eval_text(
            &mut agent,
            &realm,
            "(function () { var a = []; while (true) a.push({}); })()"
        ),
        support::script::Outcome::Terminated(swell::Termination::MemoryBudget)
    );
    let (live_ended, heap_ended) = readings(&agent, "the push loop");
    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
    readings(&agent, "the first collect");
    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
    let (live, heap) = readings(&agent, "the second collect");
    assert!(
        live_ended - live > heap_ended - heap,
        "live_bytes {live_ended} -> {live}, heap_bytes {heap_ended} -> {heap}"
    );

    let k = budget - heap + 1;
    assert!(
        live + 2 * k <= budget,
        "no k: live_bytes {live}, heap_bytes {heap}, memory_bytes {budget}"
    );
    assert_eq!(
        support::script::eval_text(&mut agent, &realm, &format!("\"x\".repeat({k}).length")),
        support::script::Outcome::Terminated(swell::Termination::MemoryBudget)
    );
    readings(&agent, "the repeat");
}

/// A root of another agent is `ForeignHandle`, and `duplicate` makes a second root that keeps
/// the value after the first is dropped.
#[test]
fn a_root_is_checked_and_can_be_duplicated() {
    let mut a = support::host::agent().unwrap();
    let realm_a = support::host::realm(&mut a);
    let mut b = support::host::agent().unwrap();
    let realm_b = support::host::realm(&mut b);
    let (root, s) = a
        .with_cx(&realm_a, |cx| {
            let s = cx.new_string("held").unwrap();
            (cx.root(Value::String(s)).unwrap(), s)
        })
        .unwrap();
    assert_eq!(
        b.with_cx(&realm_b, |cx| cx.get(&root)).unwrap(),
        Err(HostError::ForeignHandle)
    );
    assert!(matches!(b.duplicate(&root), Err(HostError::ForeignHandle)));

    let second = a.duplicate(&root).unwrap();
    drop(root);
    assert!(matches!(a.collect(), Ok(Outcome::Normal(_))));
    let units = a
        .with_cx(&realm_a, |cx| {
            assert_eq!(cx.get(&second), Ok(Value::String(s)));
            cx.to_utf16(s).unwrap()
        })
        .unwrap();
    assert_eq!(units, "held".encode_utf16().collect::<Vec<u16>>());
}

use swell::{
    Agent, AgentState, Completion, Cx, ErrorKind, HookCx, Host, HostCall, Job, LoadRequest,
    ObjectRef, Parsed, PropertyKey, Root, Source,
};

/// A host that keeps in its own value the root `take` reads and drops, the root of `churn`,
/// and the object `take` read.
#[derive(Default)]
struct Keeper {
    held: Option<Root>,
    churn: Option<Root>,
    read: Option<ObjectRef>,
}

impl Host for Keeper {
    fn host_enqueue_generic_job(_: &mut HookCx<'_, Self>, _: Job) {}
    fn host_enqueue_timeout_job(_: &mut HookCx<'_, Self>, _: Job, _: f64) {}
    fn host_enqueue_finalization_registry_cleanup_job(_: &mut HookCx<'_, Self>, _: Job) {}
    fn host_load_imported_module(_: &mut HookCx<'_, Self>, _: LoadRequest) {}
    fn host_system_utc_epoch_nanoseconds(_: &mut HookCx<'_, Self>, _: ObjectRef) -> i128 {
        0
    }
}

/// The script that defines `churn`, whose String of `n` code units (two bytes each) starts a
/// collection at the next safe point when `n` is at least twice `heap_bytes()` and at least
/// 2^20: a collection starts when the bytes since the last one pass the live size of that one,
/// and at least 1 MiB (`heap/mod.rs`, `collection_due`).
fn churn_source(n: u64) -> String {
    format!("function churn() {{ return \"x\".repeat({n}).length; }}")
}

/// The steps of `take`: read the object the host keeps a root of with `cx.get`, drop that root,
/// call `churn` through `cx.call`, and return the object. A step that does not answer as the
/// test needs makes it return `undefined` or `null`, which the test refuses.
fn take(cx: &mut Cx<'_, Keeper>, _call: HostCall<'_>) -> Completion<Value> {
    let held = cx.host_mut().held.take();
    let churn_root = cx.host_mut().churn.take();
    let (Some(held), Some(churn_root)) = (held, churn_root) else {
        return Ok(Value::Undefined);
    };
    let (Ok(Value::Object(object)), Ok(churn)) = (cx.get(&held), cx.get(&churn_root)) else {
        return Ok(Value::Undefined);
    };
    cx.host_mut().read = Some(object);
    drop(held);
    match cx.call(churn, Value::Undefined, &[]) {
        Ok(Ok(Value::Number(_))) => Ok(Value::Object(object)),
        _ => Ok(Value::Null),
    }
}

/// HOST-170: a host function that reads an object with `cx.get` from the only root that holds
/// it, drops that root, calls through `cx.call` a function whose allocation starts a
/// collection, and returns the object, answers `Normal` with that object, and the agent stays
/// `Alive`. The value read is handed out to the running scope, so once its scope has ended and
/// the root of the answer is dropped, a collection frees it.
#[test]
fn host_170_a_value_read_from_a_dropped_root_lives_until_its_scope_ends() {
    let mut limits = support::host::limits();
    limits.steps_per_entry = 1 << 26;
    let mut agent = Agent::new(Keeper::default(), limits, support::host::seeds()).unwrap();
    let Ok(Outcome::Normal(realm)) = agent.create_realm(None, None) else {
        panic!("create_realm did not answer Normal");
    };
    let n = (2 * agent.heap_bytes().unwrap()).max(1 << 20);
    let source = churn_source(n);
    let Ok(Parsed::Parsed(script)) = agent.parse_script(&realm, Source::Utf8(&source), None) else {
        panic!("the script of churn did not parse");
    };
    assert!(matches!(
        agent.evaluate_script(&script),
        Ok(Outcome::Normal(_))
    ));
    let global = agent.global_object(&realm).unwrap();
    agent
        .with_cx(&realm, |cx| {
            let Ok(Value::Object(global)) = cx.get(&global) else {
                panic!("the global object");
            };
            let realm_ref = cx.realm_ref(&realm).unwrap();
            let function = cx
                .new_host_function(realm_ref, "take", 0, take, false, None)
                .unwrap()
                .unwrap();
            let key = PropertyKey::String(cx.new_string("take").unwrap());
            cx.set(global, key, Value::Object(function), true)
                .unwrap()
                .unwrap();
            let key = PropertyKey::String(cx.new_string("churn").unwrap());
            let churn = cx.get_property(global, key).unwrap().unwrap();
            let object = cx.new_host_object(spec(None)).unwrap().unwrap();
            let held = cx.root(Value::Object(object)).unwrap();
            let churn = cx.root(churn).unwrap();
            let host = cx.host_mut();
            host.held = Some(held);
            host.churn = Some(churn);
        })
        .unwrap();

    let Ok(Parsed::Parsed(script)) = agent.parse_script(&realm, Source::Utf8("take();"), None)
    else {
        panic!("take(); did not parse");
    };
    let outcome = agent.evaluate_script(&script).unwrap();
    let Outcome::Normal(answer) = outcome else {
        panic!(
            "take() answered {outcome:?} with n = {n}; the agent is {:?}",
            agent.state()
        );
    };
    assert_eq!(agent.state(), AgentState::Alive);
    let read = agent.host().read.unwrap();
    let answered = agent.with_cx(&realm, |cx| cx.get(&answer)).unwrap();
    assert_eq!(answered, Ok(Value::Object(read)), "n = {n}");

    drop(answer);
    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
    let brand = agent.with_cx(&realm, |cx| cx.brand(read)).unwrap();
    assert_eq!(brand, Err(HostError::StaleHandle));
}

/// A `RealmRef` names its realm and holds nothing (section 4: a realm is held through a `Realm`
/// root): after the host drops the only root of the second realm and a collection frees it, a
/// `new_error` in that realm answers `Err(StaleHandle)` and the agent stays `Alive`.
#[test]
fn realm_ref_after_the_realm_root_is_dropped() {
    let mut limits = support::host::limits();
    limits.steps_per_entry = 1 << 26;
    let mut agent = support::host::agent_with(limits).unwrap();
    let first = support::host::realm(&mut agent);
    let second = support::host::realm(&mut agent);
    let n = (2 * agent.heap_bytes().unwrap()).max(1 << 20);
    assert!(matches!(
        support::script::eval(&mut agent, &first, &churn_source(n)),
        Outcome::Normal(_)
    ));
    let global = support::host::global(&mut agent, &first);
    let (ran, error) = agent
        .with_cx(&first, |cx| {
            let global = support::host::object(cx, &global);
            let churn = support::host::get(cx, global, "churn");
            let realm_ref = cx.realm_ref(&second).unwrap();
            drop(second);
            let ran = cx.call(churn, Value::Undefined, &[]).unwrap();
            let error = cx.new_error(realm_ref, ErrorKind::RangeError, "m");
            (matches!(ran, Ok(Value::Number(_))), error.err())
        })
        .unwrap();
    assert!(ran, "churn did not answer a Number, n = {n}");
    assert_eq!(error, Some(HostError::StaleHandle), "n = {n}");
    assert_eq!(agent.state(), AgentState::Alive);
}
