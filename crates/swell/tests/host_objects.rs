//! Host objects: brand, host data and `trace` (`docs/design/host-interface.md`, decision 15).
//! Run also with the feature `collect-every-safe-point`.

mod support;

use std::cell::Cell;
use std::rc::Rc;

use swell::{
    AgentState, Brand, HostData, HostError, HostObjectSpec, ObjectRef, Outcome, Termination,
    Tracer, Value,
};

const NODE: Brand = Brand(1);
const OTHER: Brand = Brand(2);

struct Node {
    id: u32,
}

impl HostData for Node {
    fn trace(&self, _tracer: &mut Tracer<'_>) {}
}

struct Text;

impl HostData for Text {
    fn trace(&self, _tracer: &mut Tracer<'_>) {}
}

/// Host data that holds two objects and lists only the first in its trace.
struct Holder {
    listed: Option<ObjectRef>,
    _unlisted: Option<ObjectRef>,
}

impl HostData for Holder {
    fn trace(&self, tracer: &mut Tracer<'_>) {
        if let Some(object) = self.listed {
            tracer.value(Value::Object(object));
        }
    }
}

/// Host data that counts its drops.
struct Counted {
    drops: Rc<Cell<u32>>,
}

impl HostData for Counted {
    fn trace(&self, _tracer: &mut Tracer<'_>) {}
}

impl Drop for Counted {
    fn drop(&mut self) {
        self.drops.set(self.drops.get() + 1);
    }
}

fn spec(brand: Brand, data: Option<Box<dyn HostData>>) -> HostObjectSpec {
    HostObjectSpec {
        prototype: None,
        brand,
        data,
    }
}

/// HOST-108: `host_data::<T>` with the object's own brand and type returns `Some`, and with
/// another brand, another type, or an object that carries no such data, `Ok(None)`.
#[test]
fn host_108_host_data_checks_brand_and_type() {
    let mut agent = support::host::agent().unwrap();
    let realm = support::host::realm(&mut agent);
    agent
        .with_cx(&realm, |cx| {
            let node = cx
                .new_host_object(spec(NODE, Some(Box::new(Node { id: 7 }))))
                .unwrap()
                .unwrap();
            let bare = cx.new_host_object(spec(NODE, None)).unwrap().unwrap();

            assert_eq!(cx.brand(node), Ok(Some(NODE)));
            assert_eq!(
                cx.host_data::<Node>(node, NODE).unwrap().map(|n| n.id),
                Some(7)
            );
            assert!(cx.host_data::<Node>(node, OTHER).unwrap().is_none());
            assert!(cx.host_data::<Text>(node, NODE).unwrap().is_none());
            assert!(cx.host_data::<Node>(bare, NODE).unwrap().is_none());

            cx.host_data_mut::<Node>(node, NODE).unwrap().unwrap().id = 8;
            assert!(cx.host_data_mut::<Node>(node, OTHER).unwrap().is_none());
            assert!(cx.host_data_mut::<Text>(node, NODE).unwrap().is_none());
            assert_eq!(
                cx.host_data::<Node>(node, NODE).unwrap().map(|n| n.id),
                Some(8)
            );
        })
        .unwrap();

    // A handle of another agent is refused, not read.
    let mut other = support::host::agent().unwrap();
    let other_realm = support::host::realm(&mut other);
    let node = agent
        .with_cx(&realm, |cx| {
            cx.new_host_object(spec(NODE, Some(Box::new(Node { id: 1 }))))
                .unwrap()
                .unwrap()
        })
        .unwrap();
    other
        .with_cx(&other_realm, |cx| {
            assert_eq!(cx.brand(node), Err(HostError::ForeignHandle));
            assert!(matches!(
                cx.host_data::<Node>(node, NODE),
                Err(HostError::ForeignHandle)
            ));
        })
        .unwrap();
}

/// HOST-109: an object reachable only through a value that a host object's `trace` lists
/// survives `collect`; the same kind of object held but not listed is freed by it.
#[test]
fn host_109_trace_keeps_listed_values() {
    let mut agent = support::host::agent().unwrap();
    let realm = support::host::realm(&mut agent);
    let (_root, listed, unlisted) = agent
        .with_cx(&realm, |cx| {
            let listed = cx.new_host_object(spec(NODE, None)).unwrap().unwrap();
            let unlisted = cx.new_host_object(spec(NODE, None)).unwrap().unwrap();
            let holder = cx
                .new_host_object(spec(
                    OTHER,
                    Some(Box::new(Holder {
                        listed: Some(listed),
                        _unlisted: Some(unlisted),
                    })),
                ))
                .unwrap()
                .unwrap();
            (cx.root(Value::Object(holder)).unwrap(), listed, unlisted)
        })
        .unwrap();

    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));

    agent
        .with_cx(&realm, |cx| {
            assert_eq!(cx.brand(listed), Ok(Some(NODE)));
            assert_eq!(cx.brand(unlisted), Err(HostError::StaleHandle));
        })
        .unwrap();
    assert_eq!(agent.state(), AgentState::Alive);
}

/// HOST-110: a `trace` that lists a stale handle makes `collect` answer
/// `Terminated(EngineFault)`, and the agent is faulted after it.
#[test]
fn host_110_trace_of_a_stale_handle_faults_collect() {
    let mut agent = support::host::agent().unwrap();
    let realm = support::host::realm(&mut agent);
    let freed = agent
        .with_cx(&realm, |cx| {
            cx.new_host_object(spec(NODE, None)).unwrap().unwrap()
        })
        .unwrap();
    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
    assert_eq!(
        agent.with_cx(&realm, |cx| cx.brand(freed)).unwrap(),
        Err(HostError::StaleHandle)
    );

    let _root = agent
        .with_cx(&realm, |cx| {
            let holder = cx
                .new_host_object(spec(
                    OTHER,
                    Some(Box::new(Holder {
                        listed: Some(freed),
                        _unlisted: None,
                    })),
                ))
                .unwrap()
                .unwrap();
            cx.root(Value::Object(holder)).unwrap()
        })
        .unwrap();

    assert!(matches!(
        agent.collect(),
        Ok(Outcome::Terminated(Termination::EngineFault))
    ));
    assert_eq!(agent.state(), AgentState::Faulted);
    assert!(matches!(agent.collect(), Err(HostError::Faulted)));
    assert!(matches!(
        agent.with_cx(&realm, |_| ()),
        Err(HostError::Faulted)
    ));
    assert_eq!(agent.heap_bytes(), Err(HostError::Faulted));
}

/// HOST-111: the host data of a freed host object is dropped by the `collect` that frees the
/// object, and not before.
#[test]
fn host_111_host_data_is_dropped_with_its_object() {
    let drops = Rc::new(Cell::new(0));
    let mut agent = support::host::agent().unwrap();
    let realm = support::host::realm(&mut agent);
    let root = agent
        .with_cx(&realm, |cx| {
            let kept = cx
                .new_host_object(spec(
                    NODE,
                    Some(Box::new(Counted {
                        drops: drops.clone(),
                    })),
                ))
                .unwrap()
                .unwrap();
            cx.new_host_object(spec(
                NODE,
                Some(Box::new(Counted {
                    drops: drops.clone(),
                })),
            ))
            .unwrap()
            .unwrap();
            cx.root(Value::Object(kept)).unwrap()
        })
        .unwrap();
    assert_eq!(
        drops.get(),
        0,
        "no host data is dropped before a collection"
    );

    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
    assert_eq!(
        drops.get(),
        1,
        "the collection drops the data of the object it frees"
    );

    drop(root);
    assert_eq!(drops.get(), 1, "dropping a root frees nothing by itself");
    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
    assert_eq!(drops.get(), 2);
    assert_eq!(
        agent.host().calls,
        Vec::<String>::new(),
        "collect called no hook"
    );
}

/// A host object's prototype is traced: an object reachable only as the prototype of a rooted
/// host object survives `collect`.
#[test]
fn a_prototype_is_kept_by_its_object() {
    let mut agent = support::host::agent().unwrap();
    let realm = support::host::realm(&mut agent);
    let (_root, prototype) = agent
        .with_cx(&realm, |cx| {
            let prototype = cx.new_host_object(spec(OTHER, None)).unwrap().unwrap();
            let object = cx
                .new_host_object(HostObjectSpec {
                    prototype: Some(prototype),
                    brand: NODE,
                    data: None,
                })
                .unwrap()
                .unwrap();
            (cx.root(Value::Object(object)).unwrap(), prototype)
        })
        .unwrap();
    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
    assert_eq!(
        agent.with_cx(&realm, |cx| cx.brand(prototype)).unwrap(),
        Ok(Some(OTHER))
    );
}
