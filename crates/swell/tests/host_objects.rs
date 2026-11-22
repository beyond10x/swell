//! Host objects: brand, host data and `trace` (`docs/design/host-interface.md`, decision 15),
//! and a handle the host keeps without a root (decision 4, HOST-178). Run also with the feature
//! `collect-every-safe-point`.
#![cfg(test)]

use crate::support;

use std::cell::Cell;
use std::rc::Rc;

use support::host::TestHost;
use support::script::{Outcome as Shown, define_function, eval_text};
use swell::{
    Abrupt, Agent, AgentState, Brand, Completion, Cx, GlobalObjects, HookCx, HostCall, HostData,
    HostError, HostIntrinsic, HostObjectSpec, ObjectRef, Outcome, Parsed, PropertyKey, Prototype,
    RealmRef, Root, Source, SymbolRef, Termination, Tracer, Value, WellKnownSymbol,
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
        error_kind: None,
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
                    error_kind: None,
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

/// The three intrinsics of the closed list (`docs/design/host-interface.md`, decision 7), each
/// with the script that reads it in a realm whose globals are the originals.
const INTRINSICS: [(HostIntrinsic, &str); 3] = [
    (HostIntrinsic::FunctionPrototype, "Function.prototype"),
    (HostIntrinsic::ErrorPrototype, "Error.prototype"),
    (HostIntrinsic::ObjectPrototype, "Object.prototype"),
];

/// Set the global property `name` of the global object `global` to `value`.
fn bind(cx: &mut Cx<'_, TestHost>, global: ObjectRef, name: &str, value: Value) {
    let key = support::host::key(cx, name);
    cx.set(global, key, value, true).unwrap().unwrap();
}

/// Define the own data property `key` of `object` as `value`: writable, configurable, not
/// enumerable.
fn define(cx: &mut Cx<'_, TestHost>, object: ObjectRef, key: PropertyKey, value: Value) {
    let descriptor = support::host::data(value, true, false, true);
    cx.define_property_or_throw(object, key, descriptor)
        .unwrap()
        .unwrap();
}

/// A host object of `brand` whose prototype is `prototype`, with no data.
fn on(prototype: Prototype, brand: Brand) -> HostObjectSpec {
    HostObjectSpec {
        prototype,
        brand,
        data: None,
        error_kind: None,
    }
}

/// HOST-183 (`webidl:interface-object`, `webidl:interface-prototype-object`): in the setup
/// closure and in `with_cx`, `intrinsic(realm, FunctionPrototype)`, `ErrorPrototype` and
/// `ObjectPrototype` answer the objects the scripts `Function.prototype`, `Error.prototype` and
/// `Object.prototype` read in that realm, also after a script set the globals `Function`,
/// `Error` and `Object` to `null`; a second realm's answers differ from the first's; a realm
/// `create_realm` did not make, and one a collection freed, answer `Err(StaleHandle)`. The
/// reads are reads: after a `collect`, `heap_bytes()` and `live_bytes()` read the same before
/// and after a `with_cx` of 3,000 of them, and no hook is called.
#[test]
fn host_183_the_host_names_three_intrinsics() {
    let mut agent = support::host::agent().unwrap();
    // The setup closure roots the three, and makes the global on the `%Object.prototype%` it
    // names.
    let mut in_setup: Vec<Root> = Vec::new();
    let mut setup = |cx: &mut HookCx<'_, TestHost>, realm: RealmRef| -> Completion<GlobalObjects> {
        for (which, _) in INTRINSICS {
            let named = cx.intrinsic(realm, which).unwrap();
            in_setup.push(cx.root(Value::Object(named)).unwrap());
        }
        let object_prototype = cx.intrinsic(realm, HostIntrinsic::ObjectPrototype).unwrap();
        let global = cx
            .new_host_object(on(Prototype::Object(object_prototype), OTHER))
            .unwrap()?;
        Ok(GlobalObjects { global, this: None })
    };
    let Outcome::Normal(realm) = agent.create_realm(Some(&mut setup), None).unwrap() else {
        panic!("create_realm did not answer Normal");
    };
    assert_eq!(in_setup.len(), INTRINSICS.len());
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "Object.getPrototypeOf(this) === Object.prototype"
        ),
        Shown::Normal("true".to_string()),
        "the global is on the named %Object.prototype%"
    );

    let first = agent
        .with_cx(&realm, |cx| {
            let realm_ref = cx.realm_ref(&realm).unwrap();
            let mut first = Vec::new();
            for ((which, source), from_setup) in INTRINSICS.into_iter().zip(&in_setup) {
                let read = evaluate(cx, &realm, source);
                let named = Value::Object(cx.intrinsic(realm_ref, which).unwrap());
                assert_eq!(named, read, "{which:?} is what {source} reads");
                assert_eq!(
                    cx.get(from_setup).unwrap(),
                    read,
                    "{which:?} in the setup closure"
                );
                first.push(cx.root(read).unwrap());
            }
            first
        })
        .unwrap();

    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "Function = null; Error = null; Object = null; \
             Function === null && Error === null && Object === null"
        ),
        Shown::Normal("true".to_string())
    );
    agent
        .with_cx(&realm, |cx| {
            let realm_ref = cx.realm_ref(&realm).unwrap();
            for ((which, source), first) in INTRINSICS.into_iter().zip(&first) {
                let named = Value::Object(cx.intrinsic(realm_ref, which).unwrap());
                assert_eq!(
                    named,
                    cx.get(first).unwrap(),
                    "{which:?} after the script set the global of {source} to null"
                );
            }
        })
        .unwrap();

    let second = support::host::realm(&mut agent);
    let second_ref = agent
        .with_cx(&second, |cx| {
            let second_ref = cx.realm_ref(&second).unwrap();
            for ((which, source), first) in INTRINSICS.into_iter().zip(&first) {
                let named = Value::Object(cx.intrinsic(second_ref, which).unwrap());
                assert_eq!(named, evaluate(cx, &second, source), "{which:?}");
                assert_ne!(
                    named,
                    cx.get(first).unwrap(),
                    "{which:?} of the second realm is its own"
                );
            }
            second_ref
        })
        .unwrap();

    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
    let before = (agent.heap_bytes().unwrap(), agent.live_bytes().unwrap());
    agent
        .with_cx(&realm, |cx| {
            let realm_ref = cx.realm_ref(&realm).unwrap();
            for _ in 0..1_000 {
                for (which, _) in INTRINSICS {
                    cx.intrinsic(realm_ref, which).unwrap();
                }
            }
        })
        .unwrap();
    assert_eq!(
        (agent.heap_bytes().unwrap(), agent.live_bytes().unwrap()),
        before,
        "(heap_bytes, live_bytes) after 3,000 reads"
    );

    // A realm a collection freed, and one `create_realm` did not make.
    drop(second);
    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
    let mut unmade = None;
    let mut throws =
        |_cx: &mut HookCx<'_, TestHost>, made: RealmRef| -> Completion<GlobalObjects> {
            unmade = Some(made);
            Err(Abrupt::Throw(Value::Number(1.0)))
        };
    let created = agent.create_realm(Some(&mut throws), None).unwrap();
    assert!(matches!(created, Outcome::Throw(_)), "{created:?}");
    let unmade = unmade.unwrap();
    agent
        .with_cx(&realm, |cx| {
            for (which, _) in INTRINSICS {
                assert_eq!(
                    cx.intrinsic(second_ref, which),
                    Err(HostError::StaleHandle),
                    "{which:?} of a freed realm"
                );
                assert_eq!(
                    cx.intrinsic(unmade, which),
                    Err(HostError::StaleHandle),
                    "{which:?} of a realm create_realm did not make"
                );
            }
        })
        .unwrap();
    assert_eq!(
        agent.host().calls,
        Vec::<String>::new(),
        "no hook was called"
    );
    assert_eq!(agent.state(), AgentState::Alive);
}

/// HOST-184 (`webidl:dfn-class-string`, `webidl:interface-prototype-object`): a host object `n`
/// whose own `@@toStringTag` data property, defined with `well_known_symbol(ToStringTag)`, is
/// `"Node"` makes the script `Object.prototype.toString.call(n)` answer `"[object Node]"`; with
/// an own `@@unscopables` object whose `before` is `true`, `with (n) { before }` reads the outer
/// binding and not `n`'s own `before`, which a host object without it gives. The symbols are
/// the same in two realms of the agent: those `with_cx` of the first realm and the setup closure
/// of the second name are equal, and a script of the second realm reads the first realm's `n`
/// as the first realm's script does.
#[test]
fn host_184_the_host_names_two_symbols() {
    let mut agent = support::host::agent().unwrap();
    let realm = support::host::realm(&mut agent);
    let global_root = support::host::global(&mut agent, &realm);
    let (n_root, symbols) = agent
        .with_cx(&realm, |cx| {
            let realm_ref = cx.realm_ref(&realm).unwrap();
            let tag = cx.well_known_symbol(WellKnownSymbol::ToStringTag).unwrap();
            let unscopables = cx.well_known_symbol(WellKnownSymbol::Unscopables).unwrap();
            let on_object_prototype = || on(Prototype::ObjectPrototypeOf(realm_ref), NODE);
            let blocked = cx.new_host_object(on_object_prototype()).unwrap().unwrap();
            let before = support::host::key(cx, "before");
            define(cx, blocked, before, Value::Boolean(true));

            let n = cx.new_host_object(on_object_prototype()).unwrap().unwrap();
            let node = support::host::string(cx, "Node");
            define(cx, n, PropertyKey::Symbol(tag), node);
            define(
                cx,
                n,
                PropertyKey::Symbol(unscopables),
                Value::Object(blocked),
            );
            let inner = support::host::string(cx, "inner");
            define(cx, n, before, inner);

            let m = cx.new_host_object(on_object_prototype()).unwrap().unwrap();
            define(cx, m, before, inner);

            let global = support::host::object(cx, &global_root);
            bind(cx, global, "n", Value::Object(n));
            bind(cx, global, "m", Value::Object(m));
            (cx.root(Value::Object(n)).unwrap(), [tag, unscopables])
        })
        .unwrap();

    let shown = |text: &str| Shown::Normal(format!("\"{text}\""));
    assert_eq!(
        eval_text(&mut agent, &realm, "Object.prototype.toString.call(n)"),
        shown("[object Node]")
    );
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var before = \"outer\"; with (n) { before }"
        ),
        shown("outer")
    );
    assert_eq!(eval_text(&mut agent, &realm, "n.before"), shown("inner"));
    assert_eq!(
        eval_text(&mut agent, &realm, "with (m) { before }"),
        shown("inner"),
        "without @@unscopables the with statement reads the object's own property"
    );

    let mut in_setup: Vec<SymbolRef> = Vec::new();
    let mut setup = |cx: &mut HookCx<'_, TestHost>, realm: RealmRef| -> Completion<GlobalObjects> {
        in_setup.push(cx.well_known_symbol(WellKnownSymbol::ToStringTag).unwrap());
        in_setup.push(cx.well_known_symbol(WellKnownSymbol::Unscopables).unwrap());
        let global = cx
            .new_host_object(on(Prototype::ObjectPrototypeOf(realm), OTHER))
            .unwrap()?;
        Ok(GlobalObjects { global, this: None })
    };
    let Outcome::Normal(second) = agent.create_realm(Some(&mut setup), None).unwrap() else {
        panic!("create_realm did not answer Normal");
    };
    assert_eq!(in_setup, symbols, "the setup closure of the second realm");

    let (named, read) = agent
        .with_cx(&second, |cx| {
            let named = [
                cx.well_known_symbol(WellKnownSymbol::ToStringTag).unwrap(),
                cx.well_known_symbol(WellKnownSymbol::Unscopables).unwrap(),
            ];
            let n = support::host::object(cx, &n_root);
            let probe = evaluate(
                cx,
                &second,
                "(function (n) { var before = \"outer\"; \
                 with (n) { return Object.prototype.toString.call(n) + \" \" + before; } })",
            );
            let answer = cx
                .call(probe, Value::Undefined, &[Value::Object(n)])
                .unwrap()
                .unwrap();
            (named, support::host::text(cx, answer))
        })
        .unwrap();
    assert_eq!(named, symbols, "with_cx of the second realm");
    assert_eq!(read, "[object Node] outer", "the second realm's script");
    assert_eq!(agent.state(), AgentState::Alive);
}

/// A `DOMException` reaches `%Error.prototype%` (`webidl:interface-prototype-object`:
/// "Otherwise, if interface is the DOMException interface, then set proto to
/// realm.[[Intrinsics]].[[%Error.prototype%]]."): a host object `e` whose prototype is a host
/// object whose prototype is `intrinsic(realm, ErrorPrototype)` makes `e instanceof Error`
/// answer `true`. Whether `e` has [[ErrorData]] is not asserted here.
#[test]
fn a_dom_exception_reaches_error_prototype() {
    let mut agent = support::host::agent().unwrap();
    let realm = support::host::realm(&mut agent);
    let global_root = support::host::global(&mut agent, &realm);
    agent
        .with_cx(&realm, |cx| {
            let realm_ref = cx.realm_ref(&realm).unwrap();
            let error_prototype = cx
                .intrinsic(realm_ref, HostIntrinsic::ErrorPrototype)
                .unwrap();
            let interface_prototype = cx
                .new_host_object(on(Prototype::Object(error_prototype), OTHER))
                .unwrap()
                .unwrap();
            let e = cx
                .new_host_object(on(Prototype::Object(interface_prototype), NODE))
                .unwrap()
                .unwrap();
            let global = support::host::object(cx, &global_root);
            bind(cx, global, "e", Value::Object(e));
        })
        .unwrap();
    assert_eq!(
        eval_text(&mut agent, &realm, "e instanceof Error"),
        Shown::Normal("true".to_string())
    );
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "Object.getPrototypeOf(Object.getPrototypeOf(e)) === Error.prototype \
             && !(e instanceof TypeError)"
        ),
        Shown::Normal("true".to_string())
    );
}
