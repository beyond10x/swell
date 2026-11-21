//! The collector and the root table (`docs/design/host-interface.md`, decisions 4 and 18;
//! `engine.md`, decision 4, "Collector"). Run also with the feature `collect-every-safe-point`.

mod support;

use swell::{Brand, HostData, HostError, HostObjectSpec, Outcome, Tracer, Value};

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
        prototype: None,
        brand: BRAND,
        data,
    }
}

/// HOST-28: an object held only by a `Root` survives `collect`, and the root then reads the
/// same object; an object nothing holds is freed by the same collection.
#[test]
fn host_28_an_object_held_only_by_a_root_survives_collect() {
    let mut agent = support::host::agent().unwrap();
    let (root, kept, dropped) = agent
        .with_cx(|cx| {
            let kept = cx.new_host_object(spec(None)).unwrap().unwrap();
            let dropped = cx.new_host_object(spec(None)).unwrap().unwrap();
            (cx.root(Value::Object(kept)).unwrap(), kept, dropped)
        })
        .unwrap();

    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));

    let (read, brand, gone) = agent
        .with_cx(|cx| {
            let read = cx.get(&root).unwrap();
            (read, cx.brand(kept), cx.brand(dropped))
        })
        .unwrap();
    assert_eq!(read, Value::Object(kept));
    assert_eq!(brand, Ok(Some(BRAND)));
    assert_eq!(gone, Err(HostError::StaleHandle));
}

/// HOST-29: after the host drops the last `Root` of an object nothing else reaches, the second
/// `collect` after the drop reports a lower `heap_bytes()` than before the drop.
#[test]
fn host_29_dropping_the_last_root_lowers_heap_bytes() {
    let mut agent = support::host::agent().unwrap();
    let root = agent
        .with_cx(|cx| {
            let object = cx
                .new_host_object(spec(Some(Box::new(Payload { _bytes: [0; 256] }))))
                .unwrap()
                .unwrap();
            cx.root(Value::Object(object)).unwrap()
        })
        .unwrap();
    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
    let before = agent.heap_bytes().unwrap();

    drop(root);
    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
    let after = agent.heap_bytes().unwrap();
    assert!(
        after < before,
        "heap_bytes {after} after the drop, {before} before it"
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
    let _root = agent
        .with_cx(|cx| {
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

/// A root of another agent is `ForeignHandle`, and `duplicate` makes a second root that keeps
/// the value after the first is dropped.
#[test]
fn a_root_is_checked_and_can_be_duplicated() {
    let mut a = support::host::agent().unwrap();
    let mut b = support::host::agent().unwrap();
    let (root, s) = a
        .with_cx(|cx| {
            let s = cx.new_string("held").unwrap();
            (cx.root(Value::String(s)).unwrap(), s)
        })
        .unwrap();
    assert_eq!(
        b.with_cx(|cx| cx.get(&root)).unwrap(),
        Err(HostError::ForeignHandle)
    );
    assert!(matches!(b.duplicate(&root), Err(HostError::ForeignHandle)));

    let second = a.duplicate(&root).unwrap();
    drop(root);
    assert!(matches!(a.collect(), Ok(Outcome::Normal(_))));
    let units = a
        .with_cx(|cx| {
            assert_eq!(cx.get(&second), Ok(Value::String(s)));
            cx.to_utf16(s).unwrap()
        })
        .unwrap();
    assert_eq!(units, "held".encode_utf16().collect::<Vec<u16>>());
}
