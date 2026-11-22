//! Host objects: brand, host data and `trace` (`docs/design/host-interface.md`, decision 15),
//! and a handle the host keeps without a root (decision 4, HOST-178). Run also with the feature
//! `collect-every-safe-point`.
#![cfg(test)]

mod support;

use std::cell::Cell;
use std::rc::Rc;

use support::host::TestHost;
use support::script::{Outcome as Shown, define_function, eval_text};
use swell::{
    Agent, AgentState, Brand, Completion, Cx, HostCall, HostData, HostError, HostObjectSpec,
    ObjectRef, Outcome, Parsed, PropertyKey, Prototype, Source, Termination, Tracer, Value,
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
        prototype: Prototype::Null,
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
                    prototype: Prototype::Object(prototype),
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

/// The new host objects of HOST-178, made after the kept one was freed.
const NEW_OBJECTS: u32 = 1_000;

/// The value of the script `source`, parsed and evaluated inside the running `with_cx` of
/// `realm`.
fn evaluate(cx: &mut Cx<'_, TestHost>, realm: &swell::Realm, source: &str) -> Value {
    let realm = cx.realm_ref(realm).unwrap();
    match cx.parse_script(realm, Source::Utf8(source), None).unwrap() {
        Parsed::Parsed(script) => cx.evaluate_script(script).unwrap().unwrap(),
        other => panic!("{source:?} answered {other:?}"),
    }
}

/// The slot index of `object`, read from its `Debug` text (`index: <n>,`), so that a test tells
/// that a new object took the slot of a freed one.
fn slot_of(object: ObjectRef) -> Option<String> {
    let text = format!("{object:?}");
    let (_, rest) = text.split_once("index: ")?;
    rest.split_once(',').map(|(index, _)| index.to_string())
}

/// HOST-178: a host object made in one `with_cx` and kept by the host as an `ObjectRef` with no
/// root, stored by a script in a global, answers its brand in a later `with_cx` and after
/// `collect`; once the script deleted the global and `collect` ran, `brand` of the kept handle
/// answers `Err(HostError::StaleHandle)`, also after 1,000 new host objects were made in the
/// freed slots, and `state()` reads `Alive`. In the collect mode the object is freed at the
/// first safe point after the global is deleted. A guard: the generation check of the arenas
/// holds it.
#[test]
fn host_178_a_kept_handle_is_weak() {
    let mut agent = support::host::agent().unwrap();
    let realm = support::host::realm(&mut agent);
    agent
        .with_cx(&realm, |cx| {
            let node = cx
                .new_host_object(spec(NODE, Some(Box::new(Node { id: 0 }))))
                .unwrap()
                .unwrap();
            cx.host_mut().kept.push(Some(node));
            let store = evaluate(cx, &realm, "(function (o) { globalThis.g = o; })");
            cx.call(store, Value::Undefined, &[Value::Object(node)])
                .unwrap()
                .unwrap();
        })
        .unwrap();
    let kept = agent.host().kept[0].unwrap();
    let brand = |agent: &mut Agent<TestHost>| agent.with_cx(&realm, |cx| cx.brand(kept)).unwrap();

    assert_eq!(brand(&mut agent), Ok(Some(NODE)), "in a later with_cx");
    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
    assert_eq!(
        brand(&mut agent),
        Ok(Some(NODE)),
        "after collect, held by the global"
    );

    assert_eq!(
        eval_text(&mut agent, &realm, "delete globalThis.g"),
        Shown::Normal("true".to_string())
    );
    if cfg!(feature = "collect-every-safe-point") {
        assert_eq!(
            brand(&mut agent),
            Err(HostError::StaleHandle),
            "freed at the first safe point after the delete"
        );
    }
    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
    assert_eq!(brand(&mut agent), Err(HostError::StaleHandle));

    let freed_slot = slot_of(kept);
    assert!(freed_slot.is_some(), "the slot of {kept:?}");
    let reused = agent
        .with_cx(&realm, |cx| {
            let mut reused = 0;
            for id in 1..=NEW_OBJECTS {
                let made = cx
                    .new_host_object(spec(NODE, Some(Box::new(Node { id }))))
                    .unwrap()
                    .unwrap();
                if slot_of(made) == freed_slot {
                    reused += 1;
                }
            }
            assert_eq!(cx.brand(kept), Err(HostError::StaleHandle));
            assert!(matches!(
                cx.host_data::<Node>(kept, NODE),
                Err(HostError::StaleHandle)
            ));
            reused
        })
        .unwrap();
    assert_eq!(reused, 1, "one new object took the freed slot");
    assert_eq!(brand(&mut agent), Err(HostError::StaleHandle));
    assert_eq!(agent.state(), AgentState::Alive);
}

/// The steps of the host function `node(n)`: the host object the host keeps for node `n` when
/// `brand` finds it live, else a new one, which the host keeps in its place and counts in
/// `TestHost::count`. A host that keeps a handle without a root checks it before it returns it
/// (HOST-178, HOST-160).
fn node(cx: &mut Cx<'_, TestHost>, call: HostCall<'_>) -> Completion<Value> {
    let number = match call.args.first() {
        Some(Value::Number(number)) => *number as usize,
        _ => 0,
    };
    if let Some(&Some(kept)) = cx.host().kept.get(number) {
        match cx.brand(kept) {
            Ok(Some(NODE)) => return Ok(Value::Object(kept)),
            Err(HostError::StaleHandle) => {}
            other => panic!("node {number}: brand answered {other:?}"),
        }
    }
    let id = u32::try_from(number).unwrap();
    let made = cx
        .new_host_object(spec(NODE, Some(Box::new(Node { id }))))
        .unwrap()?;
    let host = cx.host_mut();
    if host.kept.len() <= number {
        host.kept.resize(number + 1, None);
    }
    host.kept[number] = Some(made);
    host.count += 1;
    Ok(Value::Object(made))
}

/// A guard of HOST-178: a kept handle of a live object is that object. A script stores what the
/// host function `node(0)` made in a global; after `collect` the host's `with_cx` gets the kept
/// handle back through `node(0)`, and the script reads it as the object of its global
/// (`o === g`). Once the global is deleted and `collect` ran, `node(0)` finds the kept handle
/// stale and makes a new object.
#[test]
fn a_kept_handle_of_a_live_object_is_the_same_object() {
    let mut agent = support::host::agent().unwrap();
    let realm = support::host::realm(&mut agent);
    define_function(&mut agent, &realm, "node", 1, node, false);
    let global = support::host::global(&mut agent, &realm);

    assert_eq!(
        eval_text(&mut agent, &realm, "globalThis.g = node(0); typeof g"),
        Shown::Normal("\"object\"".to_string())
    );
    assert_eq!(agent.host().count, 1);
    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));

    let same = agent
        .with_cx(&realm, |cx| {
            let global = support::host::object(cx, &global);
            let key = PropertyKey::String(cx.new_string("node").unwrap());
            let function = cx.get_property(global, key).unwrap().unwrap();
            let o = cx
                .call(function, Value::Undefined, &[Value::Number(0.0)])
                .unwrap()
                .unwrap();
            let compare = evaluate(cx, &realm, "(function (o) { return o === g; })");
            cx.call(compare, Value::Undefined, &[o]).unwrap().unwrap()
        })
        .unwrap();
    assert_eq!(same, Value::Boolean(true));
    assert_eq!(
        eval_text(&mut agent, &realm, "node(0) === g"),
        Shown::Normal("true".to_string())
    );
    assert_eq!(agent.host().count, 1, "node(0) returned the kept handle");

    assert_eq!(
        eval_text(&mut agent, &realm, "delete globalThis.g"),
        Shown::Normal("true".to_string())
    );
    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
    assert_eq!(
        eval_text(&mut agent, &realm, "globalThis.g = node(0); node(0) === g"),
        Shown::Normal("true".to_string())
    );
    assert_eq!(agent.host().count, 2, "node(0) replaced the stale handle");
    assert_eq!(agent.state(), AgentState::Alive);
}
