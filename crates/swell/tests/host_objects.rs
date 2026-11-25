//! Host objects: brand, host data and `trace` (`docs/design/host-interface.md`, decision 15),
//! and a handle the host keeps without a root (decision 4, HOST-178). Run also with the feature
//! `collect-every-safe-point`.
#![cfg(test)]

use crate::support;

use std::cell::Cell;
use std::rc::Rc;
use std::time::Instant;

use support::host::TestHost;
use support::script::{Outcome as Shown, define_function, eval_text, nothing};
use swell::{
    Abrupt, Agent, AgentState, Brand, Completion, Cx, GlobalObjects, HookCx, HostCall, HostData,
    HostError, HostIntrinsic, HostObjectSpec, ObjectRef, Outcome, Parsed, PropertyDescriptor,
    PropertyKey, Prototype, Realm, RealmRef, Root, Source, SymbolRef, Termination, Tracer, Value,
    WellKnownSymbol,
};
use swell::{
    BINDINGS_REALM_HEAP_BYTES, HOST_FUNCTION_LIVE_BYTES, HOST_OBJECT_LIVE_BYTES,
    PROPERTY_LIVE_BYTES,
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

/// Host data that holds a String and a Symbol it lists in its trace, and a String it does not.
struct Keeper {
    string: Value,
    symbol: Value,
    _unlisted: Value,
}

impl HostData for Keeper {
    fn trace(&self, tracer: &mut Tracer<'_>) {
        tracer.value(self.string);
        tracer.value(self.symbol);
    }
}

/// The host function `get`: the Symbol the `Keeper` of the host's first kept object holds.
fn kept_symbol(cx: &mut Cx<'_, TestHost>, _call: HostCall<'_>) -> Completion<Value> {
    let holder = cx.host().kept.first().copied().flatten().unwrap();
    Ok(cx
        .host_data::<Keeper>(holder, OTHER)
        .unwrap()
        .unwrap()
        .symbol)
}

/// HOST-218: a String with a lone surrogate and a Symbol, each kept only in a host object's
/// data and listed by its `trace` with `Tracer::value`, survive `collect`; a String kept the
/// same way but not listed is freed by it.
#[test]
fn host_218_trace_keeps_a_listed_string_and_symbol() {
    let mut agent = support::host::agent().unwrap();
    let realm = support::host::realm(&mut agent);
    define_function(&mut agent, &realm, "get", 0, kept_symbol, false);
    let made: Vec<Root> = [
        "String.fromCharCode(0x61, 0xD800)",
        "Symbol(\"d\")",
        "String.fromCharCode(0x62, 0xD800)",
    ]
    .iter()
    .map(
        |source| match support::script::eval(&mut agent, &realm, source) {
            Outcome::Normal(root) => root,
            other => panic!("{source:?} answered {other:?}"),
        },
    )
    .collect();
    let (_root, string, unlisted) = agent
        .with_cx(&realm, |cx| {
            let values: Vec<Value> = made.iter().map(|root| cx.get(root).unwrap()).collect();
            let (Value::String(string), Value::Symbol(_), Value::String(unlisted)) =
                (values[0], values[1], values[2])
            else {
                panic!("the scripts made {values:?}");
            };
            let holder = cx
                .new_host_object(spec(
                    OTHER,
                    Some(Box::new(Keeper {
                        string: values[0],
                        symbol: values[1],
                        _unlisted: values[2],
                    })),
                ))
                .unwrap()
                .unwrap();
            cx.host_mut().kept = vec![Some(holder)];
            (cx.root(Value::Object(holder)).unwrap(), string, unlisted)
        })
        .unwrap();
    drop(made);

    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));

    agent
        .with_cx(&realm, |cx| {
            assert_eq!(cx.to_utf16(string), Ok(vec![0x61, 0xD800]));
            assert_eq!(cx.to_utf16(unlisted), Err(HostError::StaleHandle));
        })
        .unwrap();
    assert_eq!(
        eval_text(&mut agent, &realm, "get().description"),
        Shown::Normal("\"d\"".to_string())
    );
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
    let Outcome::Normal(realm) =
        support::host::create_realm(&mut agent, Some(&mut setup), None).unwrap()
    else {
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
    let Outcome::Normal(second) =
        support::host::create_realm(&mut agent, Some(&mut setup), None).unwrap()
    else {
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

/// The bytes of the host data of HOST-186.
const PAGE: usize = 4_096;

/// Host data of 4,096 bytes.
struct Page {
    _bytes: [u8; PAGE],
}

impl HostData for Page {
    fn trace(&self, _tracer: &mut Tracer<'_>) {}
}

/// The names of the members of a realm of bindings, of 4 to 32 code units: the name at index `i`
/// has `4 + i`.
const NAMES: [&str; 29] = [
    "item",
    "focus",
    "append",
    "replace",
    "contains",
    "normalize",
    "appendData",
    "insertAfter",
    "getAttribute",
    "querySelector",
    "hasAttributeNS",
    "removeAttribute",
    "addEventListener",
    "getAttributeNames",
    "isDefaultNamespace",
    "removeEventListener",
    "getElementsByTagName",
    "getBoundingClientRect",
    "getElementsByClassName",
    "compareDocumentPosition",
    "requestFullscreenOptions",
    "dispatchEventToListeners2",
    "createDocumentFragmentNode",
    "insertAdjacentElementBefore",
    "insertAdjacentElementsBefore",
    "requestPointerLockWithOptions",
    "createProcessingInstructionNow",
    "lookupNamespaceURIForPrefixNode",
    "requestAnimationFrameCallbackIds",
];

/// The definitions of new properties on one host object that HOST-186 amortizes a definition
/// over.
const DEFINITIONS: u64 = 300;

/// A name as long as `NAMES[i % 29]`, its last two code units the decimal of `i / 29`: a
/// different name for every `i` below 2,900.
fn numbered_name(i: usize) -> String {
    let base = NAMES[i % NAMES.len()];
    format!("{}{:02}", &base[..base.len() - 2], i / NAMES.len())
}

/// A full collection, which must complete normally.
fn collect(agent: &mut Agent<TestHost>) {
    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
}

/// What `live_bytes()` rose by over `f`, run in a `with_cx` of `realm` after a `collect` (so that
/// the collection at the safe point of the `with_cx` frees nothing in the collect mode), with
/// what `f` answered.
fn live_rise<R>(
    agent: &mut Agent<TestHost>,
    realm: &Realm,
    f: impl FnOnce(&mut Cx<'_, TestHost>) -> R,
) -> (u64, R) {
    collect(agent);
    let before = agent.live_bytes().unwrap();
    let answer = agent.with_cx(realm, f).unwrap();
    let after = agent.live_bytes().unwrap();
    let rise = after
        .checked_sub(before)
        .unwrap_or_else(|| panic!("live_bytes() fell from {before} to {after}"));
    (rise, answer)
}

/// A root of a new host function of `realm` named `name`, of length 0, answering `undefined`,
/// holding `data`.
fn host_function(
    cx: &mut Cx<'_, TestHost>,
    realm: &Realm,
    name: &str,
    data: Option<Box<dyn HostData>>,
) -> Root {
    let realm = cx.realm_ref(realm).unwrap();
    let function = cx
        .new_host_function(realm, name, 0, nothing, false, data)
        .unwrap()
        .unwrap();
    cx.root(Value::Object(function)).unwrap()
}

/// A root of a new host object of brand `NODE` holding `data`, on no prototype.
fn host_object(cx: &mut Cx<'_, TestHost>, data: Option<Box<dyn HostData>>) -> Root {
    let object = cx.new_host_object(spec(NODE, data)).unwrap().unwrap();
    cx.root(Value::Object(object)).unwrap()
}

/// Define 300 new properties on `object` by the keys `numbered_name(0)` to
/// `numbered_name(299)`, of 4 to 32 code units: each even one a data property holding its index,
/// each odd one an accessor whose getter is `getter`.
fn define_300(cx: &mut Cx<'_, TestHost>, object: ObjectRef, getter: ObjectRef) {
    for i in 0..300 {
        let key = support::host::key(cx, &numbered_name(i));
        let descriptor = if i % 2 == 0 {
            support::host::data(Value::Number(i as f64), true, true, true)
        } else {
            PropertyDescriptor {
                get: Some(Value::Object(getter)),
                enumerable: Some(true),
                configurable: Some(true),
                ..PropertyDescriptor::default()
            }
        };
        cx.define_property_or_throw(object, key, descriptor)
            .unwrap()
            .unwrap();
    }
}

/// The rises of `live_bytes()` the per-item ceilings are stated on, each in a `with_cx` of its
/// own in `realm`: a host function with a name of 32 code units and no data, a host object with
/// no data, and 300 definitions of new properties on that host object (`define_300`, the getter
/// that host function); with the roots that keep them.
struct ItemRises {
    function: u64,
    object: u64,
    definitions: u64,
    roots: Vec<Root>,
}

fn item_rises(agent: &mut Agent<TestHost>, realm: &Realm) -> ItemRises {
    let name = numbered_name(NAMES.len() - 1);
    assert_eq!(name.len(), 32, "{name}");
    let (function, function_root) =
        live_rise(agent, realm, |cx| host_function(cx, realm, &name, None));
    let (object, object_root) = live_rise(agent, realm, |cx| host_object(cx, None));
    let (definitions, ()) = live_rise(agent, realm, |cx| {
        let object = support::host::object(cx, &object_root);
        let getter = support::host::object(cx, &function_root);
        define_300(cx, object, getter);
    });
    ItemRises {
        function,
        object,
        definitions,
        roots: vec![function_root, object_root],
    }
}

/// HOST-186: after a realm's setup, a host function with a name of 32 code units and data of
/// 4,096 bytes raises `live_bytes()` by at least 4,096 (its data's box is charged with it, and
/// the box of its steps) and at most `HOST_FUNCTION_LIVE_BYTES` + 4,096, and each further one of
/// the same name length and type by the same amount; a host object with that data by at most
/// `HOST_OBJECT_LIVE_BYTES` + 4,096; a host function and a host object with no data by at most
/// their ceilings, and 300 definitions of new properties on one host object by at most 300 ×
/// `PROPERTY_LIVE_BYTES`. Once the host dropped them and `collect` ran twice, `live_bytes()`
/// reads its value from before them.
#[test]
fn host_186_a_host_function_charges_every_byte() {
    let started = Instant::now();
    let mut agent = support::host::agent().unwrap();
    let realm = support::host::realm(&mut agent);
    collect(&mut agent);
    let before = agent.live_bytes().unwrap();
    let page = u64::try_from(size_of::<Page>()).unwrap();

    let mut rises = Vec::new();
    let mut roots = Vec::new();
    for j in 0..3 {
        let name = numbered_name(NAMES.len() - 1 + NAMES.len() * j);
        assert_eq!(name.len(), 32, "{name}");
        let (rise, root) = live_rise(&mut agent, &realm, |cx| {
            host_function(
                cx,
                &realm,
                &name,
                Some(Box::new(Page { _bytes: [0; PAGE] })),
            )
        });
        rises.push(rise);
        roots.push(root);
    }
    assert!(
        rises[0] >= page,
        "a host function with {page} bytes of data raised live_bytes() by {}",
        rises[0]
    );
    assert!(
        rises[0] <= HOST_FUNCTION_LIVE_BYTES + page,
        "a host function with {page} bytes of data raised live_bytes() by {}, over \
         HOST_FUNCTION_LIVE_BYTES ({HOST_FUNCTION_LIVE_BYTES}) + {page}",
        rises[0]
    );
    assert_eq!(
        rises,
        vec![rises[0]; 3],
        "each further function of the same name length and type"
    );

    let (paged_object, root) = live_rise(&mut agent, &realm, |cx| {
        host_object(cx, Some(Box::new(Page { _bytes: [0; PAGE] })))
    });
    roots.push(root);
    assert!(paged_object >= page, "{paged_object}");
    assert!(
        paged_object <= HOST_OBJECT_LIVE_BYTES + page,
        "a host object with {page} bytes of data raised live_bytes() by {paged_object}, over \
         HOST_OBJECT_LIVE_BYTES ({HOST_OBJECT_LIVE_BYTES}) + {page}"
    );

    let items = item_rises(&mut agent, &realm);
    assert!(
        items.function <= HOST_FUNCTION_LIVE_BYTES,
        "a host function: {} over {HOST_FUNCTION_LIVE_BYTES}",
        items.function
    );
    assert!(
        items.object <= HOST_OBJECT_LIVE_BYTES,
        "a host object: {} over {HOST_OBJECT_LIVE_BYTES}",
        items.object
    );
    assert!(
        items.definitions <= DEFINITIONS * PROPERTY_LIVE_BYTES,
        "{DEFINITIONS} definitions: {} over {DEFINITIONS} × {PROPERTY_LIVE_BYTES}",
        items.definitions
    );
    println!(
        "host_186: a host function with {page} bytes of data {}, a host object with them \
         {paged_object}, a host function {}, a host object {}, {DEFINITIONS} definitions {}",
        rises[0], items.function, items.object, items.definitions
    );

    drop(roots);
    drop(items.roots);
    collect(&mut agent);
    collect(&mut agent);
    assert_eq!(
        agent.live_bytes().unwrap(),
        before,
        "live_bytes() once the host dropped them and collect ran twice"
    );
    println!("host_186: {} ms", started.elapsed().as_millis());
}

/// A guard of the steps sentence of decision 14: for k of 1, 2 and 10, a chain of k host objects
/// (the first on no prototype, each next given the one before by `set_prototype_of`) and a fresh
/// host object `t`; a `with_cx` whose last call is `set_prototype_of(t, Some(last))` answering
/// `Ok(Ok(true))` leaves `last_entry_steps()` at k, and setting the same prototype again leaves
/// it at 0.
#[test]
fn set_prototype_of_spends_a_step_per_object_of_the_new_chain() {
    let mut agent = support::host::agent().unwrap();
    let realm = support::host::realm(&mut agent);
    for k in [1u64, 2, 10] {
        let (t, last) = agent
            .with_cx(&realm, |cx| {
                let mut last = cx.new_host_object(spec(NODE, None)).unwrap().unwrap();
                for _ in 1..k {
                    let next = cx.new_host_object(spec(NODE, None)).unwrap().unwrap();
                    assert!(matches!(
                        cx.set_prototype_of(next, Some(last)).unwrap(),
                        Ok(true)
                    ));
                    last = next;
                }
                let t = cx.new_host_object(spec(OTHER, None)).unwrap().unwrap();
                let roots = (
                    cx.root(Value::Object(t)).unwrap(),
                    cx.root(Value::Object(last)).unwrap(),
                );
                assert!(matches!(
                    cx.set_prototype_of(t, Some(last)).unwrap(),
                    Ok(true)
                ));
                roots
            })
            .unwrap();
        assert_eq!(agent.last_entry_steps().unwrap(), k, "a chain of {k}");

        agent
            .with_cx(&realm, |cx| {
                let t = support::host::object(cx, &t);
                let last = support::host::object(cx, &last);
                assert!(matches!(
                    cx.set_prototype_of(t, Some(last)).unwrap(),
                    Ok(true)
                ));
            })
            .unwrap();
        assert_eq!(
            agent.last_entry_steps().unwrap(),
            0,
            "the same prototype again, a chain of {k}"
        );
    }
}

/// The interface objects of a realm of bindings, and the index of each one's parent interface.
const INTERFACES: [&str; 13] = [
    "EventTarget",
    "Node",
    "CharacterData",
    "Text",
    "Comment",
    "Element",
    "HTMLElement",
    "HTMLDivElement",
    "HTMLSpanElement",
    "Document",
    "HTMLDocument",
    "DocumentFragment",
    "ShadowRoot",
];
const PARENTS: [Option<usize>; 13] = [
    None,
    Some(0),
    Some(1),
    Some(2),
    Some(2),
    Some(1),
    Some(5),
    Some(6),
    Some(6),
    Some(1),
    Some(9),
    Some(1),
    Some(11),
];

/// The member functions, the other host objects, and the definitions on the other host objects
/// of a realm of bindings.
const MEMBERS: usize = 287;
const OTHERS: usize = 27;
const OTHER_DEFINITIONS: usize = 273;

/// The indexes of the host functions of a realm of bindings that HOST-187 binds as globals.
const BOUND: [usize; 3] = [0, 150, 299];

/// The interface objects on the chain that starts at interface `i`: it and its ancestors.
fn interfaces_on_chain(i: usize) -> u64 {
    let mut count = 1;
    let mut parent = PARENTS[i];
    while let Some(next) = parent {
        count += 1;
        parent = PARENTS[next];
    }
    count
}

/// A realm of bindings and what its build read.
struct Bindings {
    realm: Realm,
    global: Root,
    /// The 300 host functions: the 13 interface objects, then the 287 members.
    functions: Vec<Root>,
    /// The 40 host objects: the 13 interface prototype objects, then the 27 others.
    _objects: Vec<Root>,
    /// `heap_bytes()` after `create_realm`, before the first `with_cx`.
    heap_before: u64,
    /// `heap_bytes()` once the last definition was made.
    heap_after: u64,
}

/// Build a realm of 300 host functions and 40 host objects with 600 definitions, the size of a
/// browser's generated bindings, asserting the steps of HOST-187 on the way. The setup closure
/// makes the global, a host object on `%Object.prototype%`. A first `with_cx` defines the
/// global's `@@toStringTag` (the baseline of `last_entry_steps()`, 0 after it); then one
/// `with_cx` per kind of call: the 40 host objects (each interface prototype object on its
/// parent's, the root one and the 27 others on `%Object.prototype%`), the 300 host functions,
/// the 12 `set_prototype_of` that give each interface object but the first its parent, and the
/// other 599 definitions (each member on an interface prototype object as a data property or the
/// getter of an accessor, each interface's `prototype`, each prototype's `constructor`, each
/// interface as a global, and 273 data properties on the others).
fn build_bindings(agent: &mut Agent<TestHost>) -> Bindings {
    let mut setup = |cx: &mut HookCx<'_, TestHost>, realm: RealmRef| -> Completion<GlobalObjects> {
        let global = cx
            .new_host_object(on(Prototype::ObjectPrototypeOf(realm), OTHER))
            .unwrap()?;
        Ok(GlobalObjects { global, this: None })
    };
    let Outcome::Normal(realm) =
        support::host::create_realm(agent, Some(&mut setup), None).unwrap()
    else {
        panic!("create_realm did not answer Normal");
    };
    let heap_before = agent.heap_bytes().unwrap();
    let global_root = support::host::global(agent, &realm);
    let steps = |agent: &Agent<TestHost>| agent.last_entry_steps().unwrap();

    agent
        .with_cx(&realm, |cx| {
            let global = support::host::object(cx, &global_root);
            let tag = cx.well_known_symbol(WellKnownSymbol::ToStringTag).unwrap();
            let window = support::host::string(cx, "Window");
            let descriptor = support::host::data(window, false, false, true);
            cx.define_property_or_throw(global, PropertyKey::Symbol(tag), descriptor)
                .unwrap()
                .unwrap();
        })
        .unwrap();
    let baseline = steps(agent);
    assert_eq!(baseline, 0, "after a define_property_or_throw");

    let objects = agent
        .with_cx(&realm, |cx| {
            let realm_ref = cx.realm_ref(&realm).unwrap();
            let mut made: Vec<ObjectRef> = Vec::new();
            for parent in PARENTS {
                let prototype = match parent {
                    Some(parent) => Prototype::Object(made[parent]),
                    None => Prototype::ObjectPrototypeOf(realm_ref),
                };
                made.push(cx.new_host_object(on(prototype, NODE)).unwrap().unwrap());
            }
            for _ in 0..OTHERS {
                let prototype = Prototype::ObjectPrototypeOf(realm_ref);
                made.push(cx.new_host_object(on(prototype, OTHER)).unwrap().unwrap());
            }
            made.into_iter()
                .map(|object| cx.root(Value::Object(object)).unwrap())
                .collect::<Vec<_>>()
        })
        .unwrap();
    assert_eq!(objects.len(), 40);
    assert_eq!(steps(agent), baseline, "a with_cx of only new_host_object");

    let functions = agent
        .with_cx(&realm, |cx| {
            let realm_ref = cx.realm_ref(&realm).unwrap();
            let mut roots = Vec::new();
            for name in INTERFACES {
                let made = cx
                    .new_host_function(realm_ref, name, 0, nothing, true, None)
                    .unwrap()
                    .unwrap();
                roots.push(cx.root(Value::Object(made)).unwrap());
            }
            for member in 0..MEMBERS {
                let name = NAMES[member % NAMES.len()];
                let length = u32::try_from(member % 3).unwrap();
                let made = cx
                    .new_host_function(realm_ref, name, length, nothing, false, None)
                    .unwrap()
                    .unwrap();
                roots.push(cx.root(Value::Object(made)).unwrap());
            }
            roots
        })
        .unwrap();
    assert_eq!(functions.len(), 300);
    assert_eq!(
        steps(agent),
        baseline,
        "a with_cx of only new_host_function"
    );

    agent
        .with_cx(&realm, |cx| {
            for (interface, parent) in PARENTS.into_iter().enumerate() {
                let Some(parent) = parent else { continue };
                let object = support::host::object(cx, &functions[interface]);
                let parent = support::host::object(cx, &functions[parent]);
                assert!(matches!(
                    cx.set_prototype_of(object, Some(parent)).unwrap(),
                    Ok(true)
                ));
            }
        })
        .unwrap();
    // The new prototype of the last call, `DocumentFragment`: its interfaces, then
    // `%Function.prototype%` and `%Object.prototype%`.
    let last_parent = PARENTS[INTERFACES.len() - 1].unwrap();
    assert_eq!(
        steps(agent),
        interfaces_on_chain(last_parent) + 2,
        "set_prototype_of spends one step per object of the new prototype's chain"
    );

    agent
        .with_cx(&realm, |cx| {
            let global = support::host::object(cx, &global_root);
            let names: Vec<PropertyKey> = NAMES
                .iter()
                .map(|name| support::host::key(cx, name))
                .collect();
            let prototype_key = support::host::key(cx, "prototype");
            let constructor_key = support::host::key(cx, "constructor");
            for member in 0..MEMBERS {
                let holder = support::host::object(cx, &objects[member % INTERFACES.len()]);
                let function = Value::Object(support::host::object(
                    cx,
                    &functions[INTERFACES.len() + member],
                ));
                let descriptor = if member % 2 == 0 {
                    support::host::data(function, true, true, true)
                } else {
                    PropertyDescriptor {
                        get: Some(function),
                        enumerable: Some(true),
                        configurable: Some(true),
                        ..PropertyDescriptor::default()
                    }
                };
                cx.define_property_or_throw(holder, names[member % NAMES.len()], descriptor)
                    .unwrap()
                    .unwrap();
            }
            for (i, name) in INTERFACES.into_iter().enumerate() {
                let interface = support::host::object(cx, &functions[i]);
                let prototype = support::host::object(cx, &objects[i]);
                let descriptor = support::host::data(Value::Object(prototype), false, false, false);
                cx.define_property_or_throw(interface, prototype_key, descriptor)
                    .unwrap()
                    .unwrap();
                define(cx, prototype, constructor_key, Value::Object(interface));
                let key = support::host::key(cx, name);
                define(cx, global, key, Value::Object(interface));
            }
            for j in 0..OTHER_DEFINITIONS {
                let holder = support::host::object(cx, &objects[INTERFACES.len() + j % OTHERS]);
                let key = support::host::key(cx, &numbered_name(j));
                let descriptor = support::host::data(Value::Number(j as f64), false, true, false);
                cx.define_property_or_throw(holder, key, descriptor)
                    .unwrap()
                    .unwrap();
            }
        })
        .unwrap();
    assert_eq!(
        steps(agent),
        0,
        "definitions that do not compact, by keys of at most 32 code units"
    );
    let heap_after = agent.heap_bytes().unwrap();

    Bindings {
        realm,
        global: global_root,
        functions,
        _objects: objects,
        heap_before,
        heap_after,
    }
}

/// The steps the script `source` spent in `realm`; its value must be `undefined`.
fn steps_of(agent: &mut Agent<TestHost>, realm: &Realm, source: &str) -> u64 {
    match support::script::eval(agent, realm, source) {
        Outcome::Normal(value) => {
            let shown = agent.with_cx(realm, |cx| cx.get(&value).unwrap()).unwrap();
            assert_eq!(shown, Value::Undefined, "{source}");
        }
        other => panic!("{source} answered {other:?}"),
    }
    agent.last_entry_steps().unwrap()
}

/// What the scripts of HOST-187 spent in a realm of bindings and where they left `heap_bytes()`.
struct ScriptSteps {
    /// The steps of `f0()`, `f150()` and `f299()`.
    host: Vec<u64>,
    /// The steps of `g()`.
    script: u64,
    /// `heap_bytes()` after the scripts.
    heap_after: u64,
}

/// Bind three host functions of `bindings` as the globals `f0`, `f150` and `f299`, declare
/// `function g() {}` by a script, and run the scripts `f0()`, `f150()`, `f299()` and `g()`.
fn run_scripts(agent: &mut Agent<TestHost>, bindings: &Bindings) -> ScriptSteps {
    agent
        .with_cx(&bindings.realm, |cx| {
            let global = support::host::object(cx, &bindings.global);
            for index in BOUND {
                let function = support::host::object(cx, &bindings.functions[index]);
                let key = support::host::key(cx, &format!("f{index}"));
                define(cx, global, key, Value::Object(function));
            }
        })
        .unwrap();
    steps_of(agent, &bindings.realm, "function g() {}");
    let host = BOUND
        .iter()
        .map(|index| steps_of(agent, &bindings.realm, &format!("f{index}()")))
        .collect();
    let script = steps_of(agent, &bindings.realm, "g()");
    ScriptSteps {
        host,
        script,
        heap_after: agent.heap_bytes().unwrap(),
    }
}

/// HOST-187: a realm of 300 host functions and 40 host objects with 600 definitions, built in
/// this test's own body (`build_bindings`, which asserts the steps of the `with_cx` calls),
/// raises `heap_bytes()` from its reading after `create_realm` by at most
/// `BINDINGS_REALM_HEAP_BYTES`. Bound after that reading as the globals `f0`, `f150` and `f299`,
/// three of its host functions make the scripts `f0()`, `f150()` and `f299()` spend the same
/// steps, 2 fewer than the script `g()` of `function g() {}`. Once its roots are dropped and
/// `collect` ran twice, `live_bytes()` reads its value from before `create_realm`, and the same
/// realm built again, with the same bindings and scripts, leaves `heap_bytes()` at most where
/// the first build left it. They are compared after the scripts because the first build's
/// scripts make the agent's first script record, which grows the empty arena of script records
/// to 16 slots of 76 bytes, 1,216 bytes that stay charged as an arena's capacity does, so the
/// second build's reading before its scripts is 1,216 above the first's. It prints the measured rises of the four ceilings of
/// `host/limits.rs` in one line, the per-item ones measured in an agent of their own.
#[test]
fn host_187_a_realm_of_300_host_functions_and_40_host_objects() {
    let started = Instant::now();
    let mut items_agent = support::host::agent().unwrap();
    let items_realm = support::host::realm(&mut items_agent);
    let items = item_rises(&mut items_agent, &items_realm);

    let mut agent = support::host::agent().unwrap();
    collect(&mut agent);
    let live_before = agent.live_bytes().unwrap();

    let first = build_bindings(&mut agent);
    let realm_rise = first.heap_after - first.heap_before;
    println!(
        "host-ceilings measured: HOST_FUNCTION_LIVE_BYTES {} HOST_OBJECT_LIVE_BYTES {} \
         PROPERTY_LIVE_BYTES {} BINDINGS_REALM_HEAP_BYTES {realm_rise}",
        items.function,
        items.object,
        items.definitions.div_ceil(DEFINITIONS)
    );
    assert!(
        realm_rise <= BINDINGS_REALM_HEAP_BYTES,
        "the realm raised heap_bytes() by {realm_rise}, over {BINDINGS_REALM_HEAP_BYTES}"
    );

    let steps = run_scripts(&mut agent, &first);
    assert_eq!(steps.host, vec![steps.host[0]; 3], "f0(), f150(), f299()");
    assert_eq!(
        steps.script,
        steps.host[0] + 2,
        "g() against f0(): the call of a script function adds a step and runs the two \
         instructions of its empty body; the call of a host function adds one step"
    );

    let first_heap_after = first.heap_after;
    drop(first);
    collect(&mut agent);
    collect(&mut agent);
    assert_eq!(
        agent.live_bytes().unwrap(),
        live_before,
        "live_bytes() once the realm's roots were dropped and collect ran twice"
    );

    let second = build_bindings(&mut agent);
    let second_steps = run_scripts(&mut agent, &second);
    assert!(
        second_steps.heap_after <= steps.heap_after,
        "the second build left heap_bytes() at {}, the first at {}",
        second_steps.heap_after,
        steps.heap_after
    );
    println!(
        "host_187: f0() {} steps, g() {}; heap_bytes() after the build {} and {}, after the \
         scripts {} and {}; {} ms",
        steps.host[0],
        steps.script,
        first_heap_after,
        second.heap_after,
        steps.heap_after,
        second_steps.heap_after,
        started.elapsed().as_millis()
    );
}

/// The values `text` writes as "`name` = <value>", each read as the digits after it
/// (a comma between them allowed); `None` where no digit follows.
fn quoted_values(text: &str, name: &str) -> Vec<Option<u64>> {
    let marker = format!("`{name}` = ");
    text.match_indices(&marker)
        .map(|(at, _)| {
            let digits: String = text[at + marker.len()..]
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == ',')
                .filter(char::is_ascii_digit)
                .collect();
            digits.parse().ok()
        })
        .collect()
}

/// The design quotes each ceiling of `host/limits.rs` as "`NAME` = <value>" with the constant's
/// value, so that the document cannot state another number: the file `HOST_INTERFACE_DOC` names,
/// by default `docs/design/host-interface.md` of the workspace.
#[test]
fn the_design_quotes_the_ceilings_of_the_engine() {
    let path = match std::env::var_os("HOST_INTERFACE_DOC") {
        Some(path) => std::path::PathBuf::from(path),
        None => std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/design/host-interface.md"),
    };
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    for (name, value) in [
        ("HOST_FUNCTION_LIVE_BYTES", HOST_FUNCTION_LIVE_BYTES),
        ("HOST_OBJECT_LIVE_BYTES", HOST_OBJECT_LIVE_BYTES),
        ("PROPERTY_LIVE_BYTES", PROPERTY_LIVE_BYTES),
        ("BINDINGS_REALM_HEAP_BYTES", BINDINGS_REALM_HEAP_BYTES),
    ] {
        let quoted = quoted_values(&text, name);
        assert!(
            !quoted.is_empty(),
            "{} quotes no \"`{name}` = <value>\"",
            path.display()
        );
        assert_eq!(
            quoted,
            vec![Some(value); quoted.len()],
            "{} quotes `{name}` as another value than {value}",
            path.display()
        );
    }
}

/// The four intrinsics of HOST-185 (`webidl:define-the-iteration-methods`), each with the name
/// a script reads it by on `Array.prototype`.
const ITERATION_INTRINSICS: [(HostIntrinsic, &str); 4] = [
    (HostIntrinsic::ArrayPrototypeValues, "values"),
    (HostIntrinsic::ArrayPrototypeEntries, "entries"),
    (HostIntrinsic::ArrayPrototypeKeys, "keys"),
    (HostIntrinsic::ArrayPrototypeForEach, "forEach"),
];

/// The script of HOST-185, run with the globals `list`, `a` and `b`.
const NODE_LIST_SCRIPT: &str = "var r = [], it = list[Symbol.iterator](); \
     r.push(it.next().value === a, it.next().value === b, it.next().done); \
     var e = list.entries().next().value; \
     r.push(e[0] === 0 && e[1] === a, list.keys().next().value === 0, \
     list.values().next().value === a); \
     list.forEach(function (n, i) { r.push(n === (i ? b : a)); }); r.join()";

/// Make the host objects `a` and `b` and the host object `list` of HOST-185 in `realm`, and bind
/// the three as the globals of those names. `list` has the own data properties `0` and `1`
/// holding `a` and `b` and a `length` of 2, `@@iterator` as Web IDL's DefineMethodProperty
/// defines it ({ [[Writable]]: true, [[Enumerable]]: false, [[Configurable]]: true }), and
/// `entries`, `keys`, `values` and `forEach` as its CreateDataPropertyOrThrow does (all three
/// true), each from `well_known_symbol(Iterator)` and the intrinsics the host names.
fn define_node_list(agent: &mut Agent<TestHost>, realm: &Realm, global: &Root) {
    agent
        .with_cx(realm, |cx| {
            let realm_ref = cx.realm_ref(realm).unwrap();
            let on_object_prototype = |brand| on(Prototype::ObjectPrototypeOf(realm_ref), brand);
            let a = cx
                .new_host_object(on_object_prototype(NODE))
                .unwrap()
                .unwrap();
            let b = cx
                .new_host_object(on_object_prototype(NODE))
                .unwrap()
                .unwrap();
            let list = cx
                .new_host_object(on_object_prototype(OTHER))
                .unwrap()
                .unwrap();
            for (index, node) in [a, b].into_iter().enumerate() {
                let key = support::host::key(cx, &index.to_string());
                let descriptor = support::host::data(Value::Object(node), false, true, true);
                cx.define_property_or_throw(list, key, descriptor)
                    .unwrap()
                    .unwrap();
            }
            let length = support::host::key(cx, "length");
            let descriptor = support::host::data(Value::Number(2.0), false, false, true);
            cx.define_property_or_throw(list, length, descriptor)
                .unwrap()
                .unwrap();
            let iterator = cx.well_known_symbol(WellKnownSymbol::Iterator).unwrap();
            let values = cx
                .intrinsic(realm_ref, HostIntrinsic::ArrayPrototypeValues)
                .unwrap();
            define(
                cx,
                list,
                PropertyKey::Symbol(iterator),
                Value::Object(values),
            );
            for (which, name) in [
                (HostIntrinsic::ArrayPrototypeEntries, "entries"),
                (HostIntrinsic::ArrayPrototypeKeys, "keys"),
                (HostIntrinsic::ArrayPrototypeValues, "values"),
                (HostIntrinsic::ArrayPrototypeForEach, "forEach"),
            ] {
                let method = cx.intrinsic(realm_ref, which).unwrap();
                let key = support::host::key(cx, name);
                let descriptor = support::host::data(Value::Object(method), true, true, true);
                cx.define_property_or_throw(list, key, descriptor)
                    .unwrap()
                    .unwrap();
            }
            let global = support::host::object(cx, global);
            for (name, object) in [("a", a), ("b", b), ("list", list)] {
                bind(cx, global, name, Value::Object(object));
            }
        })
        .unwrap();
}

/// HOST-185 (`webidl:define-the-iteration-methods`): a host object `list` with indexed own data
/// properties `0` and `1` holding the host objects `a` and `b`, a `length` of 2, and own data
/// properties `@@iterator`, `entries`, `keys`, `values` and `forEach` defined from
/// `well_known_symbol(Iterator)` and the four intrinsics makes the script of HOST-185 answer
/// `"true,true,true,true,true,true,true,true"`, also after a script replaced
/// `Array.prototype.values`, `entries`, `keys`, `forEach` and `[Symbol.iterator]` before the host
/// defined them; `intrinsic(realm, ArrayPrototypeValues)` answers the same object as
/// `Array.prototype[Symbol.iterator]` read before that script.
#[test]
fn host_185_a_node_list_iterates_with_the_array_methods() {
    let mut agent = support::host::agent().unwrap();
    let realm = support::host::realm(&mut agent);
    let global = support::host::global(&mut agent, &realm);
    let original = agent
        .with_cx(&realm, |cx| {
            let read = evaluate(cx, &realm, "Array.prototype[Symbol.iterator]");
            cx.root(read).unwrap()
        })
        .unwrap();
    let answer = Shown::Normal("\"true,true,true,true,true,true,true,true\"".to_string());

    define_node_list(&mut agent, &realm, &global);
    assert_eq!(
        eval_text(&mut agent, &realm, NODE_LIST_SCRIPT),
        answer,
        "with the original methods on Array.prototype"
    );

    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "function replaced() { return \"replaced\"; } \
             Array.prototype.values = Array.prototype.entries = Array.prototype.keys = \
             Array.prototype.forEach = Array.prototype[Symbol.iterator] = replaced; \
             [].values() + [].entries() + [].keys() + [].forEach() + [][Symbol.iterator]()"
        ),
        Shown::Normal("\"replacedreplacedreplacedreplacedreplaced\"".to_string())
    );
    define_node_list(&mut agent, &realm, &global);
    assert_eq!(
        eval_text(&mut agent, &realm, NODE_LIST_SCRIPT),
        answer,
        "after a script replaced the methods on Array.prototype"
    );

    agent
        .with_cx(&realm, |cx| {
            let realm_ref = cx.realm_ref(&realm).unwrap();
            let named = cx
                .intrinsic(realm_ref, HostIntrinsic::ArrayPrototypeValues)
                .unwrap();
            assert_eq!(Value::Object(named), cx.get(&original).unwrap());
        })
        .unwrap();
    assert_eq!(agent.state(), AgentState::Alive);
}

/// The four intrinsics of HOST-185 are the realm's originals: in a fresh realm each answers the
/// function the script reads from `Array.prototype` by its name (and `ArrayPrototypeValues` also
/// the one at `[Symbol.iterator]`), and once a script deleted all five properties and `collect`
/// ran twice, with no root the host holds, each still answers that function, live, with its
/// `name`.
#[test]
fn the_iteration_intrinsics_are_the_originals() {
    let mut agent = support::host::agent().unwrap();
    let realm = support::host::realm(&mut agent);
    let originals = agent
        .with_cx(&realm, |cx| {
            let realm_ref = cx.realm_ref(&realm).unwrap();
            let mut originals = Vec::new();
            for (which, name) in ITERATION_INTRINSICS {
                let read = evaluate(cx, &realm, &format!("Array.prototype.{name}"));
                let named = cx.intrinsic(realm_ref, which).unwrap();
                assert_eq!(
                    Value::Object(named),
                    read,
                    "{which:?} is Array.prototype.{name}"
                );
                originals.push(named);
            }
            let iterator = evaluate(cx, &realm, "Array.prototype[Symbol.iterator]");
            assert_eq!(Value::Object(originals[0]), iterator, "@@iterator");
            originals
        })
        .unwrap();

    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "delete Array.prototype.values && delete Array.prototype.entries && \
             delete Array.prototype.keys && delete Array.prototype.forEach && \
             delete Array.prototype[Symbol.iterator] && \
             [].values === undefined && [].entries === undefined && [].keys === undefined && \
             [].forEach === undefined && [][Symbol.iterator] === undefined"
        ),
        Shown::Normal("true".to_string())
    );
    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));

    agent
        .with_cx(&realm, |cx| {
            let realm_ref = cx.realm_ref(&realm).unwrap();
            for ((which, name), original) in ITERATION_INTRINSICS.into_iter().zip(originals) {
                let named = cx.intrinsic(realm_ref, which).unwrap();
                assert_eq!(named, original, "{which:?} after the script deleted {name}");
                let shown = support::host::get(cx, named, "name");
                assert_eq!(support::host::text(cx, shown), name, "{which:?} is live");
            }
        })
        .unwrap();
    assert_eq!(agent.state(), AgentState::Alive);
}
