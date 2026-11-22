//! The collector and the root table (`docs/design/host-interface.md`, decisions 4 and 18;
//! `engine.md`, decision 4, "Collector"). Run also with the feature `collect-every-safe-point`.

mod support;

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
