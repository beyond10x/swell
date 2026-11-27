//! Host objects with an override table (`docs/design/host-interface.md`, decision 15, "Host
//! objects: brand, host data, `trace`, overrides"; `story:host-object-overrides`): each of the 11
//! internal methods of a host object runs its override where the table has one and the ordinary
//! method where it has none (HOST-114); every handle of an override's answer is checked first
//! (HOST-222), then the answer against the facts the engine recorded of the object's earlier
//! answers with the checks of the Proxy internal methods (HOST-112, HOST-113); a broken
//! invariant ends the run in `Terminated(EngineFault)`, never in a `TypeError`. An override call
//! is one re-entry and spends one step before it runs; the box of facts is charged before it
//! grows and credited when the object is freed. HOST-41: a global object whose
//! `define_own_property` refuses a property of clause 19 makes `create_realm` throw. Run also
//! with the feature `collect-every-safe-point`.
#![cfg(test)]

use crate::support;

use std::cell::Cell;

use support::host::{
    TestHost, agent, agent_with, create_realm, data, global, key, limits, object as object_of,
    realm, seeds, stack_with_room, text,
};
use support::script::{Outcome, define_function, eval_text, parse};
use support::stack::{BUILTINS_REENTRY, on_bounded_stack, stack_agent, stack_limits, test_path};
use swell::{
    Abrupt, Agent, AgentState, Brand, Completion, Cx, GlobalObjects, HookCx, HostCall, HostData,
    HostError, HostObjectSpec, ObjectRef, Outcome as HostOutcome, Overrides, PropertyDescriptor,
    PropertyKey, Prototype, Realm, RealmRef, SymbolRef, Termination, Tracer, Value,
};

/// The brand of the host objects of these tests.
const BRAND: Brand = Brand(15);

/// A table without any override.
const NONE: Overrides<TestHost> = Overrides {
    get_prototype_of: None,
    set_prototype_of: None,
    is_extensible: None,
    prevent_extensions: None,
    get_own_property: None,
    define_own_property: None,
    has_property: None,
    get: None,
    set: None,
    delete: None,
    own_property_keys: None,
};

/// The table that has no override, as a host passes it.
static EMPTY: Overrides<TestHost> = NONE;

/// What `new_host_object` makes in these tests: brand [`BRAND`], no [[ErrorData]].
fn spec(
    prototype: Prototype,
    data: Option<Box<dyn HostData>>,
    overrides: Option<&'static Overrides<TestHost>>,
) -> HostObjectSpec<TestHost> {
    HostObjectSpec {
        prototype,
        brand: BRAND,
        data,
        error_kind: None,
        overrides,
    }
}

/// A new host object of `%Object.prototype%` of `realm`, with `overrides` and `data`, bound as
/// the global property `name`.
fn bind_host(
    agent: &mut Agent<TestHost>,
    realm: &Realm,
    name: &str,
    overrides: Option<&'static Overrides<TestHost>>,
    data: Option<Box<dyn HostData>>,
) {
    let global_root = global(agent, realm);
    agent
        .with_cx(realm, |cx| {
            let realm_ref = cx.realm_ref(realm).unwrap();
            let made = cx
                .new_host_object(spec(
                    Prototype::ObjectPrototypeOf(realm_ref),
                    data,
                    overrides,
                ))
                .unwrap()
                .unwrap();
            let global = object_of(cx, &global_root);
            let name = key(cx, name);
            cx.set(global, name, Value::Object(made), true)
                .unwrap()
                .unwrap();
        })
        .unwrap();
}

/// The text of a property key an override was given; a Symbol reads `@@`.
fn key_text(cx: &mut Cx<'_, TestHost>, key: PropertyKey) -> String {
    match key {
        PropertyKey::String(string) => cx.to_rust_string_lossy(string).unwrap(),
        PropertyKey::Symbol(_) => "@@".to_string(),
    }
}

/// Record `call` in the host's list of calls.
fn log(cx: &mut Cx<'_, TestHost>, call: String) {
    cx.host_mut().calls.push(call);
}

/// The calls the host recorded since the last read, which empties the list.
fn take_calls(agent: &mut Agent<TestHost>) -> Vec<String> {
    std::mem::take(&mut agent.host_mut().calls)
}

/// After a run ended in `Terminated(EngineFault)` (HOST-48): `state()` reads `Faulted`,
/// `refusals()` answers, and every other entry point and read answers `Err(Faulted)`; `script`
/// was parsed before the fault.
fn assert_faulted(agent: &mut Agent<TestHost>, realm: &Realm, script: &swell::Script) {
    assert_eq!(agent.state(), AgentState::Faulted);
    let _ = agent.refusals();
    let faulted = |answer: Result<(), HostError>| answer == Err(HostError::Faulted);
    let answers = [
        ("with_cx", faulted(agent.with_cx(realm, |_| ()))),
        (
            "parse_script",
            faulted(
                agent
                    .parse_script(realm, swell::Source::Utf8("1;"), None)
                    .map(|_| ()),
            ),
        ),
        (
            "evaluate_script",
            faulted(agent.evaluate_script(script).map(|_| ())),
        ),
        (
            "global_object",
            faulted(agent.global_object(realm).map(|_| ())),
        ),
        ("collect", faulted(agent.collect().map(|_| ()))),
        ("heap_bytes", faulted(agent.heap_bytes().map(|_| ()))),
        ("live_bytes", faulted(agent.live_bytes().map(|_| ()))),
        (
            "last_entry_steps",
            faulted(agent.last_entry_steps().map(|_| ())),
        ),
        (
            "create_realm",
            faulted(create_realm(agent, None, None).map(|_| ())),
        ),
    ];
    for (name, answered) in answers {
        assert!(answered, "{name} of a faulted agent");
    }
}

/// The override of `define_own_property` of a global object that refuses every property, each
/// call recorded with its key.
fn refuse_define(
    cx: &mut Cx<'_, TestHost>,
    _object: ObjectRef,
    key: PropertyKey,
    _descriptor: PropertyDescriptor,
) -> Completion<bool> {
    let name = key_text(cx, key);
    log(cx, format!("define_own_property {name}"));
    Ok(false)
}

static REFUSING: Overrides<TestHost> = Overrides {
    define_own_property: Some(refuse_define),
    ..NONE
};

/// HOST-41 (`#sec-initializehostdefinedrealm`): `create_realm` whose setup gives a global object
/// whose `define_own_property` override answers `false` answers `Throw` of a `TypeError`:
/// SetDefaultGlobalBindings's DefinePropertyOrThrow (`#sec-setdefaultglobalbindings`) of the
/// first property of clause 19 throws it, after one call of the override.
#[test]
fn host_41_a_global_object_that_refuses_a_clause_19_property_fails_create_realm() {
    let mut agent = agent().unwrap();
    let mut setup = |cx: &mut HookCx<'_, TestHost>, realm: RealmRef| -> Completion<GlobalObjects> {
        let global = cx
            .new_host_object(spec(
                Prototype::ObjectPrototypeOf(realm),
                None,
                Some(&REFUSING),
            ))
            .unwrap()?;
        Ok(GlobalObjects { global, this: None })
    };
    let created = create_realm(&mut agent, Some(&mut setup), None).unwrap();
    let HostOutcome::Throw(thrown) = created else {
        panic!("create_realm answered {created:?}");
    };
    let calls = take_calls(&mut agent);
    println!("override calls: {calls:?}");
    assert_eq!(calls.len(), 1, "{calls:?}");
    assert!(calls[0].starts_with("define_own_property "), "{calls:?}");
    let other = realm(&mut agent);
    let name = agent
        .with_cx(&other, |cx| {
            let Value::Object(error) = cx.get(&thrown).unwrap() else {
                panic!("the thrown value is no object");
            };
            let name = key(cx, "name");
            let name = cx.get_property(error, name).unwrap().unwrap();
            text(cx, name)
        })
        .unwrap();
    assert_eq!(name, "TypeError");
    assert_eq!(agent.state(), AgentState::Alive);
}

/// `get_own_property` answering a non-configurable data property of 1 on its first call and
/// absent on every later one.
fn fixed_then_absent(
    cx: &mut Cx<'_, TestHost>,
    _object: ObjectRef,
    _key: PropertyKey,
) -> Completion<Option<PropertyDescriptor>> {
    let host = cx.host_mut();
    host.count += 1;
    Ok((host.count == 1).then(|| data(Value::Number(1.0), false, true, false)))
}

static FIXED_THEN_ABSENT: Overrides<TestHost> = Overrides {
    get_own_property: Some(fixed_then_absent),
    ..NONE
};

/// HOST-112 (`#sec-invariants-of-the-essential-internal-methods`): a `get_own_property`
/// override that reports `x` non-configurable and then absent ends the run that reads the
/// second answer in `Terminated(EngineFault)`, not in a `TypeError`; `state()` then reads
/// `Faulted` and every entry point but `state()` and `refusals()` answers `Err(Faulted)`.
#[test]
fn host_112_a_non_configurable_property_reported_absent_later_faults_the_agent() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    bind_host(&mut agent, &realm, "o", Some(&FIXED_THEN_ABSENT), None);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "Object.getOwnPropertyDescriptor(o, \"x\").configurable;"
        ),
        Outcome::Normal("false".to_string())
    );
    assert_eq!(agent.host().count, 1);
    let later = parse(&mut agent, &realm, "1;");
    let second = parse(
        &mut agent,
        &realm,
        "try { Object.getOwnPropertyDescriptor(o, \"x\"); } catch (e) { 0; }",
    );
    assert!(matches!(
        agent.evaluate_script(&second),
        Ok(HostOutcome::Terminated(Termination::EngineFault))
    ));
    assert_eq!(agent.host().count, 2);
    assert_faulted(&mut agent, &realm, &later);
}

thread_local! {
    /// Which descriptor [`partial_descriptor`] answers.
    static SHAPE: Cell<usize> = const { Cell::new(0) };
}

/// `get_own_property` answering a descriptor that is not a complete one: shape [`SHAPE`].
fn partial_descriptor(
    _cx: &mut Cx<'_, TestHost>,
    _object: ObjectRef,
    _key: PropertyKey,
) -> Completion<Option<PropertyDescriptor>> {
    let mut answer = data(Value::Number(1.0), true, true, true);
    match SHAPE.with(Cell::get) {
        0 => answer.enumerable = None,
        1 => answer.configurable = None,
        2 => answer.writable = None,
        3 => answer.value = None,
        4 => answer = PropertyDescriptor::default(),
        5 => {
            answer.value = None;
            answer.writable = None;
            answer.get = Some(Value::Undefined);
        }
        _ => answer.get = Some(Value::Undefined),
    }
    Ok(Some(answer))
}

static PARTIAL: Overrides<TestHost> = Overrides {
    get_own_property: Some(partial_descriptor),
    ..NONE
};

/// The shapes [`partial_descriptor`] answers.
const SHAPES: usize = 7;

/// HOST-113 (`#sec-invariants-of-the-essential-internal-methods`): a `get_own_property`
/// override that answers a descriptor that is not fully populated (no `enumerable`, no
/// `configurable`, no `writable`, no `value`, no field, an accessor without `set`, or a data
/// descriptor with a getter) ends the run in `Terminated(EngineFault)`, read through `in`, which
/// calls no other override, and faults the agent; nothing is thrown to the script.
#[test]
fn host_113_a_descriptor_that_is_not_fully_populated_faults_the_agent() {
    for shape in 0..SHAPES {
        SHAPE.with(|cell| cell.set(shape));
        let mut agent = agent().unwrap();
        let realm = realm(&mut agent);
        bind_host(&mut agent, &realm, "o", Some(&PARTIAL), None);
        let script = parse(
            &mut agent,
            &realm,
            "var r; try { r = \"x\" in o; } catch (e) { r = e; } r;",
        );
        assert!(
            matches!(
                agent.evaluate_script(&script),
                Ok(HostOutcome::Terminated(Termination::EngineFault))
            ),
            "shape {shape}"
        );
        assert_eq!(agent.state(), AgentState::Faulted, "shape {shape}");
    }
}

/// The overrides of [`LOGGING`]: each records its name (and its key) and answers.
fn log_get_prototype_of(
    cx: &mut Cx<'_, TestHost>,
    _object: ObjectRef,
) -> Completion<Option<ObjectRef>> {
    log(cx, "get_prototype_of".to_string());
    Ok(None)
}

fn log_set_prototype_of(
    cx: &mut Cx<'_, TestHost>,
    _object: ObjectRef,
    prototype: Option<ObjectRef>,
) -> Completion<bool> {
    log(cx, format!("set_prototype_of {}", prototype.is_some()));
    Ok(true)
}

fn log_is_extensible(cx: &mut Cx<'_, TestHost>, _object: ObjectRef) -> Completion<bool> {
    log(cx, "is_extensible".to_string());
    Ok(true)
}

fn log_prevent_extensions(cx: &mut Cx<'_, TestHost>, _object: ObjectRef) -> Completion<bool> {
    log(cx, "prevent_extensions".to_string());
    Ok(false)
}

fn log_get_own_property(
    cx: &mut Cx<'_, TestHost>,
    _object: ObjectRef,
    key: PropertyKey,
) -> Completion<Option<PropertyDescriptor>> {
    let name = key_text(cx, key);
    log(cx, format!("get_own_property {name}"));
    Ok(Some(data(Value::Number(7.0), true, true, true)))
}

fn log_define_own_property(
    cx: &mut Cx<'_, TestHost>,
    _object: ObjectRef,
    key: PropertyKey,
    descriptor: PropertyDescriptor,
) -> Completion<bool> {
    let name = key_text(cx, key);
    log(
        cx,
        format!(
            "define_own_property {name} {:?}",
            descriptor.value.map(|value| value == Value::Number(5.0))
        ),
    );
    Ok(true)
}

fn log_has_property(
    cx: &mut Cx<'_, TestHost>,
    _object: ObjectRef,
    key: PropertyKey,
) -> Completion<bool> {
    let name = key_text(cx, key);
    log(cx, format!("has_property {name}"));
    Ok(true)
}

fn log_get(
    cx: &mut Cx<'_, TestHost>,
    object: ObjectRef,
    key: PropertyKey,
    receiver: Value,
) -> Completion<Value> {
    let name = key_text(cx, key);
    log(
        cx,
        format!("get {name} {}", receiver == Value::Object(object)),
    );
    Ok(Value::Number(42.0))
}

fn log_set(
    cx: &mut Cx<'_, TestHost>,
    object: ObjectRef,
    key: PropertyKey,
    value: Value,
    receiver: Value,
) -> Completion<bool> {
    let name = key_text(cx, key);
    log(
        cx,
        format!(
            "set {name} {} {}",
            value == Value::Number(3.0),
            receiver == Value::Object(object)
        ),
    );
    Ok(true)
}

fn log_delete(cx: &mut Cx<'_, TestHost>, _object: ObjectRef, key: PropertyKey) -> Completion<bool> {
    let name = key_text(cx, key);
    log(cx, format!("delete {name}"));
    Ok(true)
}

fn log_own_property_keys(
    cx: &mut Cx<'_, TestHost>,
    _object: ObjectRef,
) -> Completion<Vec<PropertyKey>> {
    log(cx, "own_property_keys".to_string());
    Ok(vec![
        PropertyKey::String(cx.new_string("a")?),
        PropertyKey::String(cx.new_string("b")?),
    ])
}

/// One override per internal method, each recording its call.
static LOGGING: Overrides<TestHost> = Overrides {
    get_prototype_of: Some(log_get_prototype_of),
    set_prototype_of: Some(log_set_prototype_of),
    is_extensible: Some(log_is_extensible),
    prevent_extensions: Some(log_prevent_extensions),
    get_own_property: Some(log_get_own_property),
    define_own_property: Some(log_define_own_property),
    has_property: Some(log_has_property),
    get: Some(log_get),
    set: Some(log_set),
    delete: Some(log_delete),
    own_property_keys: Some(log_own_property_keys),
};

/// The script that reads each internal method of `t` and what it answers, in order: the
/// operations with ordinary answers of [`HOST_114_ORDINARY`].
const PROBE: &str = "function probe(t) {\n\
       var r = [];\n\
       r.push(Object.getPrototypeOf(t) === Object.prototype);\n\
       r.push(Object.isExtensible(t));\n\
       r.push(Object.defineProperty(t, \"x\", { value: 1, writable: true, enumerable: true, configurable: true }) === t);\n\
       r.push(Object.getOwnPropertyDescriptor(t, \"x\").value);\n\
       r.push(\"x\" in t);\n\
       r.push(t.x);\n\
       t.x = 2;\n\
       r.push(t.x);\n\
       t[Symbol.iterator] = 3;\n\
       r.push(Reflect.ownKeys(t).length);\n\
       r.push(delete t.x);\n\
       r.push(\"x\" in t);\n\
       r.push(Object.setPrototypeOf(t, null) === t);\n\
       r.push(Object.getPrototypeOf(t));\n\
       r.push(Object.preventExtensions(t) === t);\n\
       r.push(Object.isExtensible(t));\n\
       return r.join();\n\
     }\n";

/// What [`PROBE`] answers for an ordinary object.
const HOST_114_ORDINARY: &str = "true,true,true,1,true,1,2,2,true,false,true,,true,false";

/// HOST-114 (`#sec-invariants-of-the-essential-internal-methods`): with a table of one override
/// per internal method, each script below reaches its override, and only that one, with the
/// key and the receiver of the operation, and answers what the override answered; with a table
/// of none, every internal method answers what it answers for an ordinary object, and no
/// override runs.
#[test]
fn host_114_each_override_runs_and_a_missing_one_runs_the_ordinary_method() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    bind_host(&mut agent, &realm, "o", Some(&LOGGING), None);
    let cases: [(&str, &str, &str); 11] = [
        ("Object.getPrototypeOf(o);", "null", "get_prototype_of"),
        (
            "Object.setPrototypeOf(o, {}) === o;",
            "true",
            "set_prototype_of true",
        ),
        ("Object.isExtensible(o);", "true", "is_extensible"),
        (
            "var r; try { Object.preventExtensions(o); r = 0; } catch (e) { r = e.name; } r;",
            "\"TypeError\"",
            "prevent_extensions",
        ),
        (
            "Object.getOwnPropertyDescriptor(o, \"x\").value;",
            "7",
            "get_own_property x",
        ),
        (
            "Object.defineProperty(o, \"x\", { value: 5 }) === o;",
            "true",
            "define_own_property x Some(true)",
        ),
        ("\"x\" in o;", "true", "has_property x"),
        ("o.x;", "42", "get x true"),
        ("o.x = 3;", "3", "set x true true"),
        ("delete o.x;", "true", "delete x"),
        ("Reflect.ownKeys(o).join();", "\"a,b\"", "own_property_keys"),
    ];
    take_calls(&mut agent);
    for (source, answer, call) in cases {
        assert_eq!(
            eval_text(&mut agent, &realm, source),
            Outcome::Normal(answer.to_string()),
            "{source}"
        );
        assert_eq!(take_calls(&mut agent), [call], "{source}");
    }
    bind_host(&mut agent, &realm, "n", Some(&EMPTY), None);
    let source = format!("{PROBE}probe(n) + \"/\" + probe({{}});");
    assert_eq!(
        eval_text(&mut agent, &realm, &source),
        Outcome::Normal(format!("\"{HOST_114_ORDINARY}/{HOST_114_ORDINARY}\""))
    );
    assert_eq!(take_calls(&mut agent), Vec::<String>::new());
    assert_eq!(agent.state(), AgentState::Alive);
}

thread_local! {
    /// What [`answer_set`] answers.
    static ANSWER: Cell<Option<Completion<Value>>> = const { Cell::new(None) };
    /// The handle [`stale_prototype`], [`stale_descriptor`] and [`foreign_keys`] answer.
    static HANDLE: Cell<Value> = const { Cell::new(Value::Undefined) };
}

/// `get` answering [`ANSWER`].
fn answer_set(
    _cx: &mut Cx<'_, TestHost>,
    _object: ObjectRef,
    _key: PropertyKey,
    _receiver: Value,
) -> Completion<Value> {
    ANSWER.with(Cell::get).unwrap_or(Ok(Value::Undefined))
}

/// `get_prototype_of` answering the object of [`HANDLE`].
fn stale_prototype(
    _cx: &mut Cx<'_, TestHost>,
    _object: ObjectRef,
) -> Completion<Option<ObjectRef>> {
    match HANDLE.with(Cell::get) {
        Value::Object(object) => Ok(Some(object)),
        _ => Ok(None),
    }
}

/// `get_own_property` answering a data property whose value is [`HANDLE`].
fn stale_descriptor(
    _cx: &mut Cx<'_, TestHost>,
    _object: ObjectRef,
    _key: PropertyKey,
) -> Completion<Option<PropertyDescriptor>> {
    Ok(Some(data(HANDLE.with(Cell::get), true, true, true)))
}

/// `own_property_keys` answering the String of [`HANDLE`] as a key.
fn foreign_keys(_cx: &mut Cx<'_, TestHost>, _object: ObjectRef) -> Completion<Vec<PropertyKey>> {
    match HANDLE.with(Cell::get) {
        Value::String(string) => Ok(vec![PropertyKey::String(string)]),
        _ => Ok(Vec::new()),
    }
}

static ANSWERING: Overrides<TestHost> = Overrides {
    get_prototype_of: Some(stale_prototype),
    get_own_property: Some(stale_descriptor),
    get: Some(answer_set),
    own_property_keys: Some(foreign_keys),
    ..NONE
};

/// HOST-222 (swell:D6): an override whose `Ok` value or `Abrupt::Throw` value holds a stale
/// handle or a handle of another agent ends the run in `Terminated(EngineFault)`, before any
/// invariant is checked; `state()` then reads `Faulted` and nothing panics. The `get` override
/// answers each of the four; then a stale prototype, a stale value in a descriptor and a key of
/// another agent, through `get_prototype_of`, `get_own_property` and `own_property_keys`.
#[test]
fn host_222_an_override_result_with_a_stale_handle_faults_the_agent() {
    let mut other = agent().unwrap();
    let other_realm = realm(&mut other);
    let foreign = other
        .with_cx(&other_realm, |cx| {
            Value::String(cx.new_string("other").unwrap())
        })
        .unwrap();
    let scripts = [
        "o.x;",
        "o.x;",
        "o.x;",
        "o.x;",
        "Object.getPrototypeOf(o);",
        "Object.getOwnPropertyDescriptor(o, \"x\");",
        "Reflect.ownKeys(o);",
    ];
    for (case, source) in scripts.iter().enumerate() {
        let mut agent = agent().unwrap();
        let realm = realm(&mut agent);
        let stale = agent
            .with_cx(&realm, |cx| {
                cx.new_host_object(spec(Prototype::Null, None, None))
                    .unwrap()
                    .unwrap()
            })
            .unwrap();
        assert!(matches!(agent.collect(), Ok(HostOutcome::Normal(_))));
        assert_eq!(
            agent.with_cx(&realm, |cx| cx.brand(stale)).unwrap(),
            Err(HostError::StaleHandle)
        );
        let stale = Value::Object(stale);
        let (answer, handle) = match case {
            0 => (Ok(stale), stale),
            1 => (Ok(foreign), stale),
            2 => (Err(Abrupt::Throw(stale)), stale),
            3 => (Err(Abrupt::Throw(foreign)), stale),
            4 | 5 => (Ok(Value::Undefined), stale),
            _ => (Ok(Value::Undefined), foreign),
        };
        ANSWER.with(|cell| cell.set(Some(answer)));
        HANDLE.with(|cell| cell.set(handle));
        bind_host(&mut agent, &realm, "o", Some(&ANSWERING), None);
        let script = parse(
            &mut agent,
            &realm,
            &format!("var after = 0; try {{ {source} }} catch (e) {{ after = 1; }} after;"),
        );
        let later = parse(&mut agent, &realm, "after;");
        assert!(
            matches!(
                agent.evaluate_script(&script),
                Ok(HostOutcome::Terminated(Termination::EngineFault))
            ),
            "case {case}: {source}"
        );
        assert_faulted(&mut agent, &realm, &later);
    }
}

/// `get` throwing the String "boom".
fn throw_boom(
    cx: &mut Cx<'_, TestHost>,
    _object: ObjectRef,
    _key: PropertyKey,
    _receiver: Value,
) -> Completion<Value> {
    Err(Abrupt::Throw(Value::String(cx.new_string("boom")?)))
}

static THROWING: Overrides<TestHost> = Overrides {
    get: Some(throw_boom),
    ..NONE
};

/// An override that answers `Abrupt::Throw(v)` throws `v` to the script, after the check of its
/// handles: `catch (e)` receives the String the override made, and the agent goes on.
#[test]
fn an_override_throw_reaches_the_script_as_its_value() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    bind_host(&mut agent, &realm, "o", Some(&THROWING), None);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var r; try { o.x; r = \"none\"; } catch (e) { r = typeof e + \" \" + e; } r;"
        ),
        Outcome::Normal("\"string boom\"".to_string())
    );
    assert_eq!(agent.state(), AgentState::Alive);
}

/// `get` answering its receiver, the object it is a method of recorded as the host's `kept`.
fn get_receiver(
    cx: &mut Cx<'_, TestHost>,
    object: ObjectRef,
    _key: PropertyKey,
    receiver: Value,
) -> Completion<Value> {
    cx.host_mut().kept.push(Some(object));
    Ok(receiver)
}

static RECEIVING: Overrides<TestHost> = Overrides {
    get: Some(get_receiver),
    ..NONE
};

/// OrdinaryGet (`#sec-ordinaryget`, step 2.c, "Return ? parent.[[Get]](P, Receiver)"): a read of
/// an object whose prototype chain holds a host object with a `get` override, one or two
/// ordinary objects below it, hands over to the override with the object read as the receiver,
/// and answers what the override answers; the override is given the host object.
#[test]
fn a_host_object_with_a_get_override_in_a_prototype_chain_gets_the_receiver() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    bind_host(&mut agent, &realm, "h", Some(&RECEIVING), None);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var c = Object.create(h), d = Object.create(c);\n\
             var own = Object.prototype.hasOwnProperty.call(c, \"x\");\n\
             [c.x === c, d.y === d, h.z === h, own].join();"
        ),
        Outcome::Normal("\"true,true,true,false\"".to_string())
    );
    let global_root = global(&mut agent, &realm);
    let h = agent
        .with_cx(&realm, |cx| {
            let global = object_of(cx, &global_root);
            let name = key(cx, "h");
            cx.get_property(global, name).unwrap().unwrap()
        })
        .unwrap();
    let kept = std::mem::take(&mut agent.host_mut().kept);
    assert_eq!(kept.len(), 3);
    for object in kept {
        assert_eq!(object.map(Value::Object), Some(h));
    }
}

/// Host data: the next host object of a chain.
struct Next(Option<ObjectRef>);

impl HostData for Next {
    fn trace(&self, tracer: &mut Tracer<'_>) {
        if let Some(next) = self.0 {
            tracer.object(next);
        }
    }
}

/// `get` reading the same key of the next object of the chain, or answering 1 at its end.
fn get_next(
    cx: &mut Cx<'_, TestHost>,
    object: ObjectRef,
    key: PropertyKey,
    _receiver: Value,
) -> Completion<Value> {
    let next = cx
        .host_data::<Next>(object, BRAND)
        .unwrap()
        .and_then(|next| next.0);
    match next {
        Some(next) => cx.get_property(next, key).unwrap(),
        None => Ok(Value::Number(1.0)),
    }
}

static CHAINED: Overrides<TestHost> = Overrides {
    get: Some(get_next),
    ..NONE
};

/// A chain of `length` host objects whose `get` override reads the next one, bound as the
/// global `name`.
fn bind_chain(agent: &mut Agent<TestHost>, realm: &Realm, name: &str, length: u32) {
    let global_root = global(agent, realm);
    agent
        .with_cx(realm, |cx| {
            let mut next = None;
            for _ in 0..length {
                let made = cx
                    .new_host_object(spec(
                        Prototype::Null,
                        Some(Box::new(Next(next))),
                        Some(&CHAINED),
                    ))
                    .unwrap()
                    .unwrap();
                next = Some(made);
            }
            let global = object_of(cx, &global_root);
            let name = key(cx, name);
            let first = next.map_or(Value::Undefined, Value::Object);
            cx.set(global, name, first, true).unwrap().unwrap();
        })
        .unwrap();
}

/// The body of [`overrides_recursing_through_each_other_end_at_the_reentry_limit`].
fn recursing_overrides() {
    let (mut agent, realm) = stack_agent(BUILTINS_REENTRY);
    bind_chain(&mut agent, &realm, "long", BUILTINS_REENTRY + 10);
    bind_chain(&mut agent, &realm, "short", BUILTINS_REENTRY - 1);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var caught;\n\
             try { long.x; caught = \"none\"; }\n\
             catch (e) { caught = e instanceof RangeError ? \"RangeError\" : String(e); }\n\
             caught + \"/\" + short.x;"
        ),
        Outcome::Normal("\"RangeError/1\"".to_string())
    );
    assert_eq!(agent.state(), AgentState::Alive);
}

/// `docs/design/engine.md`, decision 3, "Depth bounds": each override call is one re-entry, so a
/// chain of `BUILTINS_REENTRY + 10` host objects whose `get` override reads the next one through
/// `cx.get_property`, with no script between, ends in the RangeError of "Depth bounds" the
/// script catches, not in a Rust stack overflow, on seven eighths of `min_stack_bytes()`
/// (`stack_agent` of `tests/support/stack.rs`); a chain one shorter than `reentry_depth` answers
/// its value after it.
#[test]
fn overrides_recursing_through_each_other_end_at_the_reentry_limit() {
    let stack = stack_with_room(stack_limits(BUILTINS_REENTRY));
    on_bounded_stack(
        test_path!("overrides_recursing_through_each_other_end_at_the_reentry_limit"),
        stack,
        recursing_overrides,
    )
    .unwrap();
}

/// The step budget of the step test.
const STEPS: u64 = 5_000;

/// `get` answering 1, with no step of its own.
fn get_one(
    _cx: &mut Cx<'_, TestHost>,
    _object: ObjectRef,
    _key: PropertyKey,
    _receiver: Value,
) -> Completion<Value> {
    Ok(Value::Number(1.0))
}

static ONE: Overrides<TestHost> = Overrides {
    get: Some(get_one),
    ..NONE
};

/// A host function that reads `x` of the global `o` through `cx.get_property` up to twice
/// [`STEPS`] times, counting each read in the host's `count`, and answers how the reads ended.
fn spin(cx: &mut Cx<'_, TestHost>, call: HostCall<'_>) -> Completion<Value> {
    let Some(&Value::Object(target)) = call.args.first() else {
        return Ok(Value::Undefined);
    };
    let name = key(cx, "x");
    for _ in 0..2 * STEPS {
        cx.get_property(target, name).unwrap()?;
        cx.host_mut().count += 1;
    }
    Ok(Value::Undefined)
}

/// Bounds, "Steps: one per override call, before the call": a host function whose loop reads a
/// key of a host object through its `get` override, which spends no step of its own, ends in
/// `Terminated(StepBudget)` with `steps_per_entry` 5,000, after at most that many reads and
/// with `last_entry_steps()` at most that; and the agent runs the next entry.
#[test]
fn an_override_loop_ends_at_the_step_budget() {
    let mut budget = limits();
    budget.steps_per_entry = STEPS;
    let mut agent = agent_with(budget).unwrap();
    let realm = realm(&mut agent);
    bind_host(&mut agent, &realm, "o", Some(&ONE), None);
    define_function(&mut agent, &realm, "spin", 1, spin, false);
    assert_eq!(
        eval_text(&mut agent, &realm, "spin(o);"),
        Outcome::Terminated(Termination::StepBudget)
    );
    let spent = agent.last_entry_steps().unwrap();
    let reads = u64::from(agent.host().count);
    println!("{reads} reads, {spent} steps of {STEPS}");
    assert!(spent <= STEPS, "{spent} steps");
    assert!(reads < STEPS, "{reads} reads");
    assert!(reads > STEPS - 100, "{reads} reads");
    assert_eq!(agent.state(), AgentState::Alive);
    assert_eq!(
        eval_text(&mut agent, &realm, "o.x;"),
        Outcome::Normal("1".to_string())
    );
}

/// `get_own_property` answering a non-configurable, non-writable data property of 1 for every
/// key.
fn fixed_one(
    _cx: &mut Cx<'_, TestHost>,
    _object: ObjectRef,
    _key: PropertyKey,
) -> Completion<Option<PropertyDescriptor>> {
    Ok(Some(data(Value::Number(1.0), false, true, false)))
}

static FIXED: Overrides<TestHost> = Overrides {
    get_own_property: Some(fixed_one),
    ..NONE
};

/// The properties the facts test reports.
const FACTS: usize = 1_000;

/// The bytes of one fact in the box of facts: an entry of its table, a key and the descriptor
/// reported (`(Key, (Key, Property))`, 72 bytes), and its control byte, 73; the table holds at
/// most seven eighths of its buckets, so a fact costs at least that. The unit test
/// `object::host_overrides::tests::a_fact_has_the_bytes_the_tests_name` holds the 72.
const FACT_BYTES: u64 = 73;

/// The object, rooted, and the [`FACTS`] keys `k0` to `k999` as atoms, rooted, made before the
/// first reading, with every handle once handed out, so that the readings see the facts alone.
fn facts_setup(agent: &mut Agent<TestHost>, realm: &Realm) -> (swell::Root, Vec<swell::Root>) {
    facts_setup_with(agent, realm, &FIXED)
}

/// As [`facts_setup`], an object of `table`.
fn facts_setup_with(
    agent: &mut Agent<TestHost>,
    realm: &Realm,
    table: &'static Overrides<TestHost>,
) -> (swell::Root, Vec<swell::Root>) {
    let made = agent
        .with_cx(realm, |cx| {
            let made = cx
                .new_host_object(spec(Prototype::Null, None, Some(table)))
                .unwrap()
                .unwrap();
            let object = cx.root(Value::Object(made)).unwrap();
            let keys = (0..FACTS)
                .map(|i| {
                    let name = Value::String(cx.new_string(&format!("k{i}")).unwrap());
                    let PropertyKey::String(atom) = cx.to_property_key(name).unwrap().unwrap()
                    else {
                        panic!("k{i} is no String key");
                    };
                    cx.root(Value::String(atom)).unwrap()
                })
                .collect::<Vec<_>>();
            (object, keys)
        })
        .unwrap();
    agent
        .with_cx(realm, |cx| {
            for root in &made.1 {
                for _ in 0..3 {
                    cx.get(root).unwrap();
                }
            }
            cx.get(&made.0).unwrap();
        })
        .unwrap();
    assert!(matches!(agent.collect(), Ok(HostOutcome::Normal(_))));
    made
}

/// Read every key of `keys` through the object of `object` (OrdinaryGet calls its
/// `get_own_property` override, which reports each non-configurable): the reads made before
/// the run ended, and how it ended.
fn report_facts(
    agent: &mut Agent<TestHost>,
    realm: &Realm,
    object: &swell::Root,
    keys: &[swell::Root],
) -> (usize, Completion<()>) {
    report_values(agent, realm, object, keys, Value::Number(1.0))
}

/// As [`report_facts`], each read answering `expected`.
fn report_values(
    agent: &mut Agent<TestHost>,
    realm: &Realm,
    object: &swell::Root,
    keys: &[swell::Root],
    expected: Value,
) -> (usize, Completion<()>) {
    agent
        .with_cx(realm, |cx| {
            let Value::Object(target) = cx.get(object).unwrap() else {
                panic!("the root holds no object");
            };
            for (read, root) in keys.iter().enumerate() {
                let Value::String(atom) = cx.get(root).unwrap() else {
                    panic!("the root holds no String");
                };
                match cx.get_property(target, PropertyKey::String(atom)).unwrap() {
                    Ok(value) => assert_eq!(value, expected),
                    Err(abrupt) => return (read, Err(abrupt)),
                }
            }
            (keys.len(), Ok(()))
        })
        .unwrap()
}

/// Bounds, "Memory": 1,000 properties a host object's `get_own_property` override reports
/// non-configurable are recorded as facts in a box of the object charged before it grows:
/// `heap_bytes()` rises by at least 1,000 times [`FACT_BYTES`], and falls back to its reading
/// before them once the object is freed by `collect()`; with a budget of that reading and half
/// the bytes of the facts, the reports end in `Terminated(MemoryBudget)` before the last one.
/// The readings are printed.
#[test]
fn the_facts_of_a_host_object_are_charged_and_credited() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let (object, keys) = facts_setup(&mut agent, &realm);
    let before = agent.heap_bytes().unwrap();
    let (read, ended) = report_facts(&mut agent, &realm, &object, &keys);
    assert_eq!((read, ended), (FACTS, Ok(())));
    let with_facts = agent.heap_bytes().unwrap();
    drop(object);
    assert!(matches!(agent.collect(), Ok(HostOutcome::Normal(_))));
    let after = agent.heap_bytes().unwrap();
    let facts = u64::try_from(FACTS).unwrap();
    println!(
        "heap_bytes: {before} before, {with_facts} with {FACTS} facts (+{}, at least {}), {after} after the object is freed",
        with_facts - before,
        facts * FACT_BYTES
    );
    assert!(with_facts - before >= facts * FACT_BYTES);
    assert!(after <= before, "{after} after, {before} before");

    let mut budget = limits();
    budget.memory_bytes = before + facts * FACT_BYTES / 2;
    let mut tight = Agent::new(TestHost::default(), budget, seeds()).unwrap();
    let tight_realm = realm_of(&mut tight);
    let (object, keys) = facts_setup(&mut tight, &tight_realm);
    let (read, ended) = report_facts(&mut tight, &tight_realm, &object, &keys);
    println!(
        "budget {}: {read} facts reported, then {ended:?}",
        budget.memory_bytes
    );
    assert_eq!(ended, Err(Abrupt::Terminated(Termination::MemoryBudget)));
    assert!(read < FACTS, "{read}");
}

/// The realm of an agent of its own limits, made as every realm of these tests is.
fn realm_of(agent: &mut Agent<TestHost>) -> Realm {
    match create_realm(agent, None, None).unwrap() {
        HostOutcome::Normal(realm) => realm,
        other => panic!("create_realm answered {other:?}"),
    }
}

/// Host data that holds a value and a Symbol it does not list in its trace, and whether the
/// override changes the value.
struct Unlisted {
    value: Cell<Option<ObjectRef>>,
    symbol: Cell<Option<SymbolRef>>,
    change: Cell<bool>,
}

impl HostData for Unlisted {
    fn trace(&self, _tracer: &mut Tracer<'_>) {}
}

/// `get_own_property` answering, for the Symbol of the host data, a non-configurable,
/// non-writable data property of its value, or of a new object once `change` is set; absent
/// for every other key.
fn unlisted_fact(
    cx: &mut Cx<'_, TestHost>,
    object: ObjectRef,
    key: PropertyKey,
) -> Completion<Option<PropertyDescriptor>> {
    let (value, symbol, change) = {
        let held = cx.host_data::<Unlisted>(object, BRAND).unwrap().unwrap();
        (held.value.get(), held.symbol.get(), held.change.get())
    };
    if PropertyKey::Symbol(symbol.unwrap()) != key {
        return Ok(None);
    }
    let value = if change {
        cx.new_host_object(spec(Prototype::Null, None, None))
            .unwrap()?
    } else {
        value.unwrap()
    };
    Ok(Some(data(Value::Object(value), false, true, false)))
}

static UNLISTED: Overrides<TestHost> = Overrides {
    get_own_property: Some(unlisted_fact),
    ..NONE
};

/// The value and the Symbol key of the object `root` holds, from its host data.
fn unlisted(cx: &mut Cx<'_, TestHost>, root: &swell::Root) -> (ObjectRef, ObjectRef, SymbolRef) {
    let Value::Object(holder) = cx.get(root).unwrap() else {
        panic!("the root holds no object");
    };
    let held = cx.host_data::<Unlisted>(holder, BRAND).unwrap().unwrap();
    (
        holder,
        held.value.get().unwrap(),
        held.symbol.get().unwrap(),
    )
}

/// The box of facts holds values, keys and a prototype, and the kind site of host objects
/// traces it (`HostObject::trace`): an override reports a non-configurable, non-writable
/// property whose key is a Symbol and whose value is an object made for it, both held only by
/// the host data, which does not list them; after the host's scope ends and `collect()` runs,
/// both handles still reach their records, an answer that repeats the fact passes the check and
/// the value read through the object is the object made before the collection, and an answer
/// that changes the value ends the run in `Terminated(EngineFault)`.
#[test]
fn the_facts_of_a_host_object_survive_a_collection() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let global_root = global(&mut agent, &realm);
    let holder = agent
        .with_cx(&realm, |cx| {
            let made = cx
                .new_host_object(spec(
                    Prototype::Null,
                    Some(Box::new(Unlisted {
                        value: Cell::new(None),
                        symbol: Cell::new(None),
                        change: Cell::new(false),
                    })),
                    Some(&UNLISTED),
                ))
                .unwrap()
                .unwrap();
            let value = cx
                .new_host_object(spec(Prototype::Null, None, None))
                .unwrap()
                .unwrap();
            let global = object_of(cx, &global_root);
            let symbol_key = key(cx, "Symbol");
            let constructor = cx.get_property(global, symbol_key).unwrap().unwrap();
            let description = Value::String(cx.new_string("fact").unwrap());
            let Value::Symbol(symbol) = cx
                .call(constructor, Value::Undefined, &[description])
                .unwrap()
                .unwrap()
            else {
                panic!("Symbol() made no Symbol");
            };
            let held = cx.host_data::<Unlisted>(made, BRAND).unwrap().unwrap();
            held.value.set(Some(value));
            held.symbol.set(Some(symbol));
            let read = cx
                .get_property(made, PropertyKey::Symbol(symbol))
                .unwrap()
                .unwrap();
            assert_eq!(read, Value::Object(value));
            cx.root(Value::Object(made)).unwrap()
        })
        .unwrap();
    assert!(matches!(agent.collect(), Ok(HostOutcome::Normal(_))));
    agent
        .with_cx(&realm, |cx| {
            let (made, value, symbol) = unlisted(cx, &holder);
            assert_eq!(cx.brand(value), Ok(Some(BRAND)));
            let read = cx
                .get_property(made, PropertyKey::Symbol(symbol))
                .unwrap()
                .unwrap();
            assert_eq!(read, Value::Object(value));
            cx.host_data::<Unlisted>(made, BRAND)
                .unwrap()
                .unwrap()
                .change
                .set(true);
            let changed = cx.get_property(made, PropertyKey::Symbol(symbol)).unwrap();
            assert_eq!(changed, Err(Abrupt::Terminated(Termination::EngineFault)));
        })
        .unwrap();
    assert_eq!(agent.state(), AgentState::Faulted);
}

/// Host data: the prototype the override answers.
struct Proto(ObjectRef);

impl HostData for Proto {
    fn trace(&self, tracer: &mut Tracer<'_>) {
        tracer.object(self.0);
    }
}

/// `get_prototype_of` answering the prototype of the host data.
fn data_prototype(cx: &mut Cx<'_, TestHost>, object: ObjectRef) -> Completion<Option<ObjectRef>> {
    log(cx, "get_prototype_of".to_string());
    Ok(cx
        .host_data::<Proto>(object, BRAND)
        .unwrap()
        .map(|proto| proto.0))
}

static PROTOTYPED: Overrides<TestHost> = Overrides {
    get_prototype_of: Some(data_prototype),
    set_prototype_of: Some(log_set_prototype_of),
    ..NONE
};

/// `Cx::get_prototype_of` and `Cx::set_prototype_of` (`docs/design/host-interface.md`, section
/// 14, "Prototypes": "once a host object has an override table (slice 9) they run host code")
/// run the overrides of a host object with a table: the prototype answered is the override's,
/// not the record's, and each call runs its override once.
#[test]
fn cx_get_prototype_of_runs_the_override() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    agent
        .with_cx(&realm, |cx| {
            let proto = cx
                .new_host_object(spec(Prototype::Null, None, None))
                .unwrap()
                .unwrap();
            let made = cx
                .new_host_object(spec(
                    Prototype::Null,
                    Some(Box::new(Proto(proto))),
                    Some(&PROTOTYPED),
                ))
                .unwrap()
                .unwrap();
            assert_eq!(cx.get_prototype_of(made).unwrap(), Ok(Some(proto)));
            assert_eq!(cx.set_prototype_of(made, None).unwrap(), Ok(true));
        })
        .unwrap();
    assert_eq!(
        take_calls(&mut agent),
        ["get_prototype_of", "set_prototype_of false"]
    );
}

/// `set` recording its key, whether the value is 1, and its receiver as the host's `kept`;
/// it answers true without defining anything.
fn set_recorded(
    cx: &mut Cx<'_, TestHost>,
    _object: ObjectRef,
    key: PropertyKey,
    value: Value,
    receiver: Value,
) -> Completion<bool> {
    let name = key_text(cx, key);
    log(cx, format!("set {name} {}", value == Value::Number(1.0)));
    if let Value::Object(receiver) = receiver {
        cx.host_mut().kept.push(Some(receiver));
    }
    Ok(true)
}

static SETTING: Overrides<TestHost> = Overrides {
    set: Some(set_recorded),
    ..NONE
};

/// The fast path of an Array write (`object/array.rs`, `array_set_index`) treats a host object
/// with an override table on the chain as not plain: `a[0] = 1` on an empty Array whose
/// prototype is a host object with a `set` override takes the full [[Set]]
/// (`#sec-ordinarysetwithowndescriptor`, step 1.c) and calls the override once, with the key
/// `"0"` and the Array as the receiver, and `a.length` stays 0 when it answers true without
/// defining; with a host prototype without a table the write appends as before.
#[test]
fn an_array_write_reaches_the_set_override_of_a_host_prototype() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    bind_host(&mut agent, &realm, "h", Some(&SETTING), None);
    bind_host(&mut agent, &realm, "plain", None, None);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var a = []; Object.setPrototypeOf(a, h); a[0] = 1;\n\
             var b = []; Object.setPrototypeOf(b, plain); b[0] = 1;\n\
             [a.length, b.length, b[0], plain.hasOwnProperty(\"0\")].join();"
        ),
        Outcome::Normal("\"0,1,1,false\"".to_string())
    );
    assert_eq!(take_calls(&mut agent), ["set 0 true"]);
    let global_root = global(&mut agent, &realm);
    let a = agent
        .with_cx(&realm, |cx| {
            let global = object_of(cx, &global_root);
            let name = key(cx, "a");
            cx.get_property(global, name).unwrap().unwrap()
        })
        .unwrap();
    let kept = std::mem::take(&mut agent.host_mut().kept);
    assert_eq!(
        kept.into_iter()
            .map(|o| o.map(Value::Object))
            .collect::<Vec<_>>(),
        [Some(a)]
    );
}

// The facts as the clause `#sec-invariants-of-the-essential-internal-methods` words them, the
// loop of `#sec-ordinarysetprototypeof`, and what the ordinary methods of a host object hold
// across an override (the module documentation of `object/host_overrides.rs`).

/// The value of the global property `name` of `realm`.
fn global_value(agent: &mut Agent<TestHost>, realm: &Realm, name: &str) -> Value {
    let global_root = global(agent, realm);
    agent
        .with_cx(realm, |cx| {
            let global = object_of(cx, &global_root);
            let name = key(cx, name);
            cx.get_property(global, name).unwrap().unwrap()
        })
        .unwrap()
}

/// `own_property_keys` answering no key.
fn no_keys(_cx: &mut Cx<'_, TestHost>, _object: ObjectRef) -> Completion<Vec<PropertyKey>> {
    Ok(Vec::new())
}

static NO_KEYS: Overrides<TestHost> = Overrides {
    own_property_keys: Some(no_keys),
    ..NONE
};

/// `#sec-invariants-of-the-essential-internal-methods`, [[OwnPropertyKeys]]: "The returned List
/// must contain at least the keys of all non-configurable own properties that have previously
/// been observed." The table has no `get_own_property`, so the module documentation of
/// `object/host_overrides.rs` makes the record's own properties the facts ("What a method
/// without an override answers is a fact as it stands"). `x` is defined non-configurable on the
/// record and observed so through [[GetOwnProperty]]; an `own_property_keys` answer without it
/// must fault the agent.
#[test]
fn own_keys_must_list_a_non_configurable_property_of_the_record() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    bind_host(&mut agent, &realm, "o", Some(&NO_KEYS), None);
    let outcome = eval_text(
        &mut agent,
        &realm,
        "Object.defineProperty(o, \"x\", { value: 1 });\n\
         var observed = Object.getOwnPropertyDescriptor(o, \"x\").configurable;\n\
         var r; try { r = observed + \"/\" + Reflect.ownKeys(o).length; } catch (e) { r = e.name; } r;",
    );
    assert_eq!(outcome, Outcome::Terminated(Termination::EngineFault));
    assert_eq!(agent.state(), AgentState::Faulted);
}

/// `own_property_keys` answering the key `ghost`.
fn ghost_key(cx: &mut Cx<'_, TestHost>, _object: ObjectRef) -> Completion<Vec<PropertyKey>> {
    Ok(vec![PropertyKey::String(cx.new_string("ghost")?)])
}

static GHOST: Overrides<TestHost> = Overrides {
    own_property_keys: Some(ghost_key),
    ..NONE
};

/// [[OwnPropertyKeys]]: "If the target is non-extensible, the returned List must contain only
/// the keys of all own properties of the target that are observable using [[GetOwnProperty]]."
/// The object is made non-extensible by the ordinary [[PreventExtensions]] (no override), its
/// ordinary [[GetOwnProperty]] answers `ghost` absent, and `own_property_keys` lists `ghost`:
/// the answer must fault the agent.
#[test]
fn own_keys_of_a_non_extensible_record_list_only_its_own_properties() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    bind_host(&mut agent, &realm, "o", Some(&GHOST), None);
    let outcome = eval_text(
        &mut agent,
        &realm,
        "Object.preventExtensions(o);\n\
         var absent = Object.getOwnPropertyDescriptor(o, \"ghost\") === undefined;\n\
         var r; try { r = absent + \"/\" + Reflect.ownKeys(o).join(); } catch (e) { r = e.name; } r;",
    );
    assert_eq!(outcome, Outcome::Terminated(Termination::EngineFault));
    assert_eq!(agent.state(), AgentState::Faulted);
}

/// `is_extensible` answering false.
fn not_extensible(_cx: &mut Cx<'_, TestHost>, _object: ObjectRef) -> Completion<bool> {
    Ok(false)
}

/// `get_own_property` answering absent on its first call and a configurable data property
/// after it.
fn absent_then_present(
    cx: &mut Cx<'_, TestHost>,
    _object: ObjectRef,
    _key: PropertyKey,
) -> Completion<Option<PropertyDescriptor>> {
    let host = cx.host_mut();
    host.count += 1;
    Ok((host.count > 1).then(|| data(Value::Number(1.0), true, true, true)))
}

static APPEARING: Overrides<TestHost> = Overrides {
    is_extensible: Some(not_extensible),
    get_own_property: Some(absent_then_present),
    ..NONE
};

/// [[GetOwnProperty]]: "If the target is non-extensible and propertyKey is non-existent, then
/// all future calls to [[GetOwnProperty]] (propertyKey) on the target must describe propertyKey
/// as non-existent". `is_extensible` answered false, then `get_own_property` answered `x`
/// absent, then present: the second answer must fault the agent.
#[test]
fn a_property_absent_from_a_non_extensible_host_object_stays_absent() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    bind_host(&mut agent, &realm, "o", Some(&APPEARING), None);
    let outcome = eval_text(
        &mut agent,
        &realm,
        "var e = Object.isExtensible(o);\n\
         var first = Object.getOwnPropertyDescriptor(o, \"x\");\n\
         var second; try { second = typeof Object.getOwnPropertyDescriptor(o, \"x\"); } catch (err) { second = err.name; }\n\
         e + \"/\" + first + \"/\" + second;",
    );
    assert_eq!(agent.host().count, 2);
    assert_eq!(outcome, Outcome::Terminated(Termination::EngineFault));
    assert_eq!(agent.state(), AgentState::Faulted);
}

/// `get_own_property` answering absent.
fn always_absent(
    _cx: &mut Cx<'_, TestHost>,
    _object: ObjectRef,
    _key: PropertyKey,
) -> Completion<Option<PropertyDescriptor>> {
    Ok(None)
}

/// `define_own_property` answering true.
fn define_true(
    _cx: &mut Cx<'_, TestHost>,
    _object: ObjectRef,
    _key: PropertyKey,
    _descriptor: PropertyDescriptor,
) -> Completion<bool> {
    Ok(true)
}

static GROWING: Overrides<TestHost> = Overrides {
    is_extensible: Some(not_extensible),
    get_own_property: Some(always_absent),
    define_own_property: Some(define_true),
    ..NONE
};

/// [[DefineOwnProperty]]: "must return false if target is non-extensible and propertyKey is a
/// non-existent own property". `is_extensible` answered false and `get_own_property` answered
/// `y` absent, so `y` is non-existent; `define_own_property` answering true for it must fault
/// the agent.
#[test]
fn define_true_for_a_key_absent_from_a_non_extensible_object_faults_the_agent() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    bind_host(&mut agent, &realm, "o", Some(&GROWING), None);
    let outcome = eval_text(
        &mut agent,
        &realm,
        "var e = Object.isExtensible(o);\n\
         var before = Object.getOwnPropertyDescriptor(o, \"y\");\n\
         var r; try { r = Reflect.defineProperty(o, \"y\", { value: 1, writable: true, enumerable: true, configurable: true }); } catch (err) { r = err.name; }\n\
         e + \"/\" + before + \"/\" + r;",
    );
    assert_eq!(outcome, Outcome::Terminated(Termination::EngineFault));
    assert_eq!(agent.state(), AgentState::Faulted);
}

/// `get_own_property` of an object that has one configurable property `x` until it is deleted
/// (the host's `count` set to 1).
fn deletable_x(
    cx: &mut Cx<'_, TestHost>,
    _object: ObjectRef,
    key: PropertyKey,
) -> Completion<Option<PropertyDescriptor>> {
    let name = key_text(cx, key);
    let present = name == "x" && cx.host_mut().count == 0;
    Ok(present.then(|| data(Value::Number(1.0), true, true, true)))
}

/// `own_property_keys` of that object.
fn deletable_keys(cx: &mut Cx<'_, TestHost>, _object: ObjectRef) -> Completion<Vec<PropertyKey>> {
    if cx.host_mut().count == 0 {
        Ok(vec![PropertyKey::String(cx.new_string("x")?)])
    } else {
        Ok(Vec::new())
    }
}

/// `delete` of that object: `x` is deleted.
fn delete_x(cx: &mut Cx<'_, TestHost>, _object: ObjectRef, key: PropertyKey) -> Completion<bool> {
    let name = key_text(cx, key);
    if name == "x" {
        cx.host_mut().count = 1;
    }
    Ok(true)
}

static SHRINKING: Overrides<TestHost> = Overrides {
    is_extensible: Some(not_extensible),
    get_own_property: Some(deletable_x),
    delete: Some(delete_x),
    own_property_keys: Some(deletable_keys),
    ..NONE
};

/// The invariants let a non-extensible object lose a configurable property: [[Delete]] must
/// answer false only for a property "previously observed as a non-configurable own data or
/// accessor property", and an ordinary object does it (`Object.preventExtensions(p); delete
/// p.x` is true). A host object that behaves exactly as the ordinary object `p` must not fault
/// the agent.
#[test]
fn a_non_extensible_host_object_may_delete_a_configurable_property() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    bind_host(&mut agent, &realm, "o", Some(&SHRINKING), None);
    let outcome = eval_text(
        &mut agent,
        &realm,
        "var p = { x: 1 }; Object.preventExtensions(p); Reflect.ownKeys(p);\n\
         var ordinary = [Object.isExtensible(p), delete p.x, Reflect.ownKeys(p).length, \"x\" in p].join();\n\
         var host = [Object.isExtensible(o), Reflect.ownKeys(o).join()].join();\n\
         host += \",\" + [delete o.x, Reflect.ownKeys(o).length, \"x\" in o].join();\n\
         ordinary + \"/\" + host;",
    );
    assert_eq!(
        outcome,
        Outcome::Normal("\"false,true,0,false/false,x,true,0,false\"".to_string())
    );
    assert_eq!(agent.state(), AgentState::Alive);
}

/// `get_prototype_of` answering null.
fn null_prototype(_cx: &mut Cx<'_, TestHost>, _object: ObjectRef) -> Completion<Option<ObjectRef>> {
    Ok(None)
}

static NULL_PROTOTYPE: Overrides<TestHost> = Overrides {
    get_prototype_of: Some(null_prototype),
    ..NONE
};

/// OrdinarySetPrototypeOf (`#sec-ordinarysetprototypeof`), step 7.c.i: "If p.[[GetPrototypeOf]]
/// is not the ordinary object internal method defined in 10.1.1, set done to true." `h` has a
/// `get_prototype_of` override (answering null) and its record's [[Prototype]] is `a`, so the
/// loop of `Reflect.setPrototypeOf(a, h)` stops at `h` and the answer is true, as it is for
/// `b`; reading `h`'s record past it finds `a` and answers false.
#[test]
fn set_prototype_of_stops_its_loop_at_an_overridden_get_prototype_of() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    assert_eq!(
        eval_text(&mut agent, &realm, "var a = {}; 0;"),
        Outcome::Normal("0".to_string())
    );
    let Value::Object(a) = global_value(&mut agent, &realm, "a") else {
        panic!("a is no object");
    };
    let global_root = global(&mut agent, &realm);
    agent
        .with_cx(&realm, |cx| {
            let made = cx
                .new_host_object(spec(Prototype::Object(a), None, Some(&NULL_PROTOTYPE)))
                .unwrap()
                .unwrap();
            let global = object_of(cx, &global_root);
            let name = key(cx, "h");
            cx.set(global, name, Value::Object(made), true)
                .unwrap()
                .unwrap();
        })
        .unwrap();
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var b = {};\n\
             var other = Reflect.setPrototypeOf(b, h);\n\
             var own = Reflect.setPrototypeOf(a, h);\n\
             [other, own, Object.getPrototypeOf(a) === h].join();"
        ),
        Outcome::Normal("\"true,true,true\"".to_string())
    );
}

thread_local! {
    /// The script function the overrides below call before they answer.
    static HOOK: Cell<Option<ObjectRef>> = const { Cell::new(None) };
}

/// Call [`HOOK`], if set: script runs (and, with `collect-every-safe-point`, collects at each
/// of its instructions).
fn run_hook(cx: &mut Cx<'_, TestHost>) -> Completion<()> {
    if let Some(function) = HOOK.with(Cell::get) {
        cx.call(Value::Object(function), Value::Undefined, &[])
            .unwrap()?;
    }
    Ok(())
}

/// Set [`HOOK`] to the global function `name` of `realm`.
fn set_hook(agent: &mut Agent<TestHost>, realm: &Realm, name: &str) {
    let Value::Object(function) = global_value(agent, realm, name) else {
        panic!("{name} is no object");
    };
    HOOK.with(|cell| cell.set(Some(function)));
}

/// `is_extensible` answering true after [`run_hook`].
fn extensible_after_hook(cx: &mut Cx<'_, TestHost>, _object: ObjectRef) -> Completion<bool> {
    run_hook(cx)?;
    Ok(true)
}

static HOOKED_EXTENSIBLE: Overrides<TestHost> = Overrides {
    is_extensible: Some(extensible_after_hook),
    ..NONE
};

/// OrdinaryDefineOwnProperty (`#sec-ordinarydefineownproperty`) of a host object whose table
/// has `is_extensible` and no `define_own_property`: step 2 calls the override while the engine
/// holds the descriptor read from `d` and the key made from `"k" + n`. The override runs a
/// script that deletes `d.value` and allocates; with `collect-every-safe-point` each of its
/// instructions collects. The value defined must still be the object `{ tag: 7 }`.
#[test]
fn a_descriptor_held_across_an_is_extensible_override_survives_a_collection() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    bind_host(&mut agent, &realm, "o", Some(&HOOKED_EXTENSIBLE), None);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var d = { value: { tag: 7 }, writable: true, enumerable: true, configurable: true };\n\
             function hook() { delete d.value; var junk = []; for (var i = 0; i < 50; i++) junk.push({ i: i }); junk = null; }\n\
             0;"
        ),
        Outcome::Normal("0".to_string())
    );
    set_hook(&mut agent, &realm, "hook");
    let outcome = eval_text(
        &mut agent,
        &realm,
        "var n = 3; var ok = Reflect.defineProperty(o, \"k\" + n, d);\n\
         for (var j = 0; j < 20; j++) ({});\n\
         var names = Object.getOwnPropertyNames(o).join();\n\
         [ok, names, typeof o.k3, o.k3 && o.k3.tag].join();",
    );
    HOOK.with(|cell| cell.set(None));
    assert_eq!(outcome, Outcome::Normal("\"true,k3,object,7\"".to_string()));
    assert_eq!(agent.state(), AgentState::Alive);
}

/// `Reflect.defineProperty(o, <key>, d)` on a host object of [`HOOKED_EXTENSIBLE`] whose hook
/// runs `hook_body`, then the names of `o` and `o.k3.tag`.
fn define_across_hook(hook_body: &str, key_source: &str) -> (Outcome, AgentState) {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    bind_host(&mut agent, &realm, "o", Some(&HOOKED_EXTENSIBLE), None);
    let setup = format!(
        "var d = {{ value: {{ tag: 7 }}, writable: true, enumerable: true, configurable: true }};\n\
         function hook() {{ {hook_body} var junk = []; for (var i = 0; i < 50; i++) junk.push({{ i: i }}); junk = null; }}\n\
         0;"
    );
    assert_eq!(
        eval_text(&mut agent, &realm, &setup),
        Outcome::Normal("0".to_string())
    );
    set_hook(&mut agent, &realm, "hook");
    let outcome = eval_text(
        &mut agent,
        &realm,
        &format!(
            "var n = 3; var ok = Reflect.defineProperty(o, {key_source}, d);\n\
             for (var j = 0; j < 20; j++) ({{}});\n\
             [ok, Object.getOwnPropertyNames(o).join(), o.k3 && o.k3.tag].join();"
        ),
    );
    HOOK.with(|cell| cell.set(None));
    (outcome, agent.state())
}

/// As [`a_descriptor_held_across_an_is_extensible_override_survives_a_collection`],
/// the descriptor's value alone: the key is the literal `"k3"`, the hook deletes `d.value`.
#[test]
fn a_descriptor_value_held_across_an_is_extensible_override_survives_a_collection() {
    assert_eq!(
        define_across_hook("delete d.value;", "\"k3\""),
        (
            Outcome::Normal("\"true,k3,7\"".to_string()),
            AgentState::Alive
        )
    );
}

/// `get` reading `x` of the next object of its chain (host data [`Next`]) through
/// `cx.get_property` up to twice [`STEPS`] times, counting each read in the host's `count`.
fn get_through_cx(
    cx: &mut Cx<'_, TestHost>,
    object: ObjectRef,
    key: PropertyKey,
    _receiver: Value,
) -> Completion<Value> {
    let next = cx
        .host_data::<Next>(object, BRAND)
        .unwrap()
        .and_then(|next| next.0);
    let Some(next) = next else {
        return Ok(Value::Undefined);
    };
    for _ in 0..2 * STEPS {
        cx.get_property(next, key).unwrap()?;
        cx.host_mut().count += 1;
    }
    Ok(Value::Number(2.0))
}

static LOOPING: Overrides<TestHost> = Overrides {
    get: Some(get_through_cx),
    ..NONE
};

/// `docs/design/host-interface.md`, "An entry made by host code that runs inside another entry
/// ... is nested" (`Agent::begin`): a `Cx` call made inside an override spends from the entry
/// that called the override. `with_cx` reads `x` of a host object whose `get` override reads a
/// key of another one through `cx.get_property` twice [`STEPS`] times, with `steps_per_entry`
/// [`STEPS`]: the read ends in `Terminated(StepBudget)` before the loop does, and the agent
/// stays usable.
#[test]
fn a_cx_call_inside_an_override_spends_from_the_entry_that_called_it() {
    let mut budget = limits();
    budget.steps_per_entry = STEPS;
    let mut agent = agent_with(budget).unwrap();
    let realm = realm(&mut agent);
    let answer = agent
        .with_cx(&realm, |cx| {
            let one = cx
                .new_host_object(spec(Prototype::Null, None, Some(&ONE)))
                .unwrap()
                .unwrap();
            let looping = cx
                .new_host_object(spec(
                    Prototype::Null,
                    Some(Box::new(Next(Some(one)))),
                    Some(&LOOPING),
                ))
                .unwrap()
                .unwrap();
            let name = key(cx, "x");
            cx.get_property(looping, name).unwrap()
        })
        .unwrap();
    let reads = u64::from(agent.host().count);
    println!("{reads} reads inside the override, then {answer:?}");
    assert_eq!(answer, Err(Abrupt::Terminated(Termination::StepBudget)));
    assert!(reads < STEPS, "{reads} reads");
    assert_eq!(agent.state(), AgentState::Alive);
}

/// OrdinaryHasProperty (`#sec-ordinaryhasproperty`, step 4.a, "Return ? parent.[[HasProperty]]
/// (P)"): `"x" in Object.create(o)` hands over to the `has_property` override of the host
/// prototype `o` and calls no other override of it.
#[test]
fn an_in_walk_hands_over_to_the_has_property_override_of_a_host_prototype() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    bind_host(&mut agent, &realm, "o", Some(&LOGGING), None);
    take_calls(&mut agent);
    assert_eq!(
        eval_text(&mut agent, &realm, "\"x\" in Object.create(o);"),
        Outcome::Normal("true".to_string())
    );
    assert_eq!(take_calls(&mut agent), ["has_property x"]);
}

/// `own_property_keys` answering the key `a` twice.
fn duplicate_keys(cx: &mut Cx<'_, TestHost>, _object: ObjectRef) -> Completion<Vec<PropertyKey>> {
    let a = PropertyKey::String(cx.new_string("a")?);
    Ok(vec![a, a])
}

static DUPLICATE: Overrides<TestHost> = Overrides {
    own_property_keys: Some(duplicate_keys),
    ..NONE
};

/// [[OwnPropertyKeys]]: "The returned List must not contain any duplicate entries." A key list
/// that holds `a` twice faults the agent.
#[test]
fn a_key_list_with_a_key_twice_faults_the_agent() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    bind_host(&mut agent, &realm, "o", Some(&DUPLICATE), None);
    let script = parse(
        &mut agent,
        &realm,
        "var r; try { r = Reflect.ownKeys(o).join(); } catch (e) { r = e.name; } r;",
    );
    let later = parse(&mut agent, &realm, "r;");
    assert!(matches!(
        agent.evaluate_script(&script),
        Ok(HostOutcome::Terminated(Termination::EngineFault))
    ));
    assert_faulted(&mut agent, &realm, &later);
}

/// HOST-222 names a handle that reaches no record or one of another agent: a handle of another
/// realm of the same agent is a handle of the agent. A `get` override answering an object made
/// in another realm is answered to the script, and the agent stays usable.
#[test]
fn an_override_may_answer_an_object_of_another_realm_of_the_agent() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let other = realm_of(&mut agent);
    assert_eq!(
        eval_text(&mut agent, &other, "var made = { tag: 7 }; 0;"),
        Outcome::Normal("0".to_string())
    );
    let made = global_value(&mut agent, &other, "made");
    ANSWER.with(|cell| cell.set(Some(Ok(made))));
    bind_host(&mut agent, &realm, "o", Some(&ANSWERING), None);
    let outcome = eval_text(
        &mut agent,
        &realm,
        "[o.x.tag, o.x === o.x, Object.getPrototypeOf(o.x) === Object.prototype].join();",
    );
    ANSWER.with(|cell| cell.set(None));
    assert_eq!(outcome, Outcome::Normal("\"7,true,false\"".to_string()));
    assert_eq!(agent.state(), AgentState::Alive);
}

thread_local! {
    /// The Symbol [`symbol_keys`] answers.
    static SYMBOL: Cell<Option<SymbolRef>> = const { Cell::new(None) };
}

/// `own_property_keys` answering the Symbol of [`SYMBOL`].
fn symbol_keys(_cx: &mut Cx<'_, TestHost>, _object: ObjectRef) -> Completion<Vec<PropertyKey>> {
    Ok(SYMBOL
        .with(Cell::get)
        .map(PropertyKey::Symbol)
        .into_iter()
        .collect())
}

static SYMBOL_KEYS: Overrides<TestHost> = Overrides {
    own_property_keys: Some(symbol_keys),
    ..NONE
};

/// HOST-222: a key list whose Symbol was collected (no root held it) faults the agent before
/// any key is made of it.
#[test]
fn a_key_list_with_a_stale_symbol_faults_the_agent() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let global_root = global(&mut agent, &realm);
    let symbol = agent
        .with_cx(&realm, |cx| {
            let global = object_of(cx, &global_root);
            let symbol_key = key(cx, "Symbol");
            let constructor = cx.get_property(global, symbol_key).unwrap().unwrap();
            let description = Value::String(cx.new_string("gone").unwrap());
            let Value::Symbol(symbol) = cx
                .call(constructor, Value::Undefined, &[description])
                .unwrap()
                .unwrap()
            else {
                panic!("Symbol() made no Symbol");
            };
            symbol
        })
        .unwrap();
    assert!(matches!(agent.collect(), Ok(HostOutcome::Normal(_))));
    SYMBOL.with(|cell| cell.set(Some(symbol)));
    bind_host(&mut agent, &realm, "o", Some(&SYMBOL_KEYS), None);
    let script = parse(
        &mut agent,
        &realm,
        "var r; try { r = Reflect.ownKeys(o).length; } catch (e) { r = e.name; } r;",
    );
    let later = parse(&mut agent, &realm, "r;");
    let outcome = agent.evaluate_script(&script);
    SYMBOL.with(|cell| cell.set(None));
    assert!(
        matches!(
            outcome,
            Ok(HostOutcome::Terminated(Termination::EngineFault))
        ),
        "{outcome:?}"
    );
    assert_faulted(&mut agent, &realm, &later);
}

/// `get` calling [`HOOK`] (a script function that makes an object) and answering, for the key
/// `ok`, what it returns, and for every other key throwing it.
fn nested_run(
    cx: &mut Cx<'_, TestHost>,
    _object: ObjectRef,
    key: PropertyKey,
    _receiver: Value,
) -> Completion<Value> {
    let name = key_text(cx, key);
    let Some(function) = HOOK.with(Cell::get) else {
        return Ok(Value::Undefined);
    };
    let made = cx
        .call(Value::Object(function), Value::Undefined, &[])
        .unwrap()?;
    if name == "ok" {
        Ok(made)
    } else {
        Err(Abrupt::Throw(made))
    }
}

static NESTED: Overrides<TestHost> = Overrides {
    get: Some(nested_run),
    ..NONE
};

/// HOST-222 and decision 6: a value made by a run nested in the override (a script function it
/// calls), answered as its `Ok` value or thrown as its `Abrupt::Throw` value, is a live handle
/// of the agent; it reaches the script, also when each instruction of the nested run collects.
#[test]
fn a_value_made_in_a_nested_run_is_answered_and_thrown() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    bind_host(&mut agent, &realm, "o", Some(&NESTED), None);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "function make() { var junk = []; for (var i = 0; i < 20; i++) junk.push({ i: i }); return { tag: 7 }; }\n\
             0;"
        ),
        Outcome::Normal("0".to_string())
    );
    set_hook(&mut agent, &realm, "make");
    let outcome = eval_text(
        &mut agent,
        &realm,
        "var r; try { o.thrown; r = \"none\"; } catch (e) { r = e.tag; }\n\
         [o.ok.tag, r].join();",
    );
    HOOK.with(|cell| cell.set(None));
    assert_eq!(outcome, Outcome::Normal("\"7,7\"".to_string()));
    assert_eq!(agent.state(), AgentState::Alive);
}

/// The fixed properties of the budget sweep.
const SWEPT: usize = 300;

/// The room under which the sweep lowers the budget by [`FINE`] bytes; above it by [`COARSE`].
/// A run costs about 1.7 ms in the default build, so a step of 64 bytes over the whole range
/// (591 runs) would go over the CPU bound of a test in the build that collects at every safe
/// point; below this room the object, the box of facts and the first growths of its table are
/// made.
const FINE_ROOM: u64 = 4096;

/// The steps of the sweep below and above [`FINE_ROOM`].
const FINE: u64 = 64;
const COARSE: u64 = 512;

/// The keys of the sweep, as atoms, rooted.
fn swept_keys(agent: &mut Agent<TestHost>, realm: &Realm) -> Vec<swell::Root> {
    agent
        .with_cx(realm, |cx| {
            (0..SWEPT)
                .map(|i| {
                    let name = Value::String(cx.new_string(&format!("s{i}")).unwrap());
                    let PropertyKey::String(atom) = cx.to_property_key(name).unwrap().unwrap()
                    else {
                        panic!("s{i} is no String key");
                    };
                    cx.root(Value::String(atom)).unwrap()
                })
                .collect()
        })
        .unwrap()
}

/// One run of the sweep with `room` bytes left under the budget: a new host object of
/// [`FIXED`], its [`SWEPT`] properties reported; then, in the next entry, a property reported
/// before answers as reported; then the object is freed and the agent collects. Answers the
/// reports made, whether the run ended in `Terminated(MemoryBudget)`, and `heap_bytes()` after.
fn sweep_run(
    agent: &mut Agent<TestHost>,
    realm: &Realm,
    keys: &[swell::Root],
    room: u64,
) -> (usize, bool, u64) {
    let used = agent.heap_bytes().unwrap();
    let limit = limits().memory_bytes;
    let (object, ballast, read, ended) = agent
        .with_cx(realm, |cx| {
            let ballast = cx
                .charge_host_bytes(limit.saturating_sub(used).saturating_sub(room))
                .unwrap();
            let made = match cx
                .new_host_object(spec(Prototype::Null, None, Some(&FIXED)))
                .unwrap()
            {
                Ok(made) => made,
                Err(abrupt) => return (None, ballast, 0, Err(abrupt)),
            };
            let object = cx.root(Value::Object(made)).unwrap();
            let mut read = 0;
            let mut ended = Ok(());
            for root in keys {
                let Value::String(atom) = cx.get(root).unwrap() else {
                    panic!("the root holds no String");
                };
                match cx.get_property(made, PropertyKey::String(atom)).unwrap() {
                    Ok(value) => assert_eq!(value, Value::Number(1.0)),
                    Err(abrupt) => {
                        ended = Err(abrupt);
                        break;
                    }
                }
                read += 1;
            }
            (Some(object), ballast, read, ended)
        })
        .unwrap();
    let terminated = match ended {
        Ok(()) => false,
        Err(Abrupt::Terminated(Termination::MemoryBudget)) => true,
        Err(other) => panic!("room {room}: {other:?}"),
    };
    if let Some(object) = object.as_ref().filter(|_| read > 0) {
        agent
            .with_cx(realm, |cx| {
                let Value::Object(made) = cx.get(object).unwrap() else {
                    panic!("the root holds no object");
                };
                let Value::String(atom) = cx.get(&keys[0]).unwrap() else {
                    panic!("the root holds no String");
                };
                assert_eq!(
                    cx.get_property(made, PropertyKey::String(atom)).unwrap(),
                    Ok(Value::Number(1.0)),
                    "room {room}"
                );
            })
            .unwrap();
    }
    drop(ballast);
    drop(object);
    assert!(matches!(agent.collect(), Ok(HostOutcome::Normal(_))));
    (read, terminated, agent.heap_bytes().unwrap())
}

/// Bounds, "Memory": the room left under the memory budget lowered from the bytes of a host
/// object and its [`SWEPT`] facts down to none, in steps of 512 bytes, then of 64 bytes below
/// 4 KiB (`FINE_ROOM`). Each run either reports
/// every property or ends in `Terminated(MemoryBudget)` with the object as it was before the
/// report that did not fit (a property reported before still answers); after the object is
/// freed and a collection, `heap_bytes()` reads what it read before the run.
#[test]
fn the_facts_end_at_the_memory_budget_at_every_size_and_leave_no_byte() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let keys = swept_keys(&mut agent, &realm);
    let (_, _, settled) = sweep_run(&mut agent, &realm, &keys, u64::MAX / 2);
    let (read, terminated, before) = sweep_run(&mut agent, &realm, &keys, u64::MAX / 2);
    assert_eq!((read, terminated, before), (SWEPT, false, settled));
    let start = agent.heap_bytes().unwrap();
    let full = agent
        .with_cx(&realm, |cx| {
            let made = cx
                .new_host_object(spec(Prototype::Null, None, Some(&FIXED)))
                .unwrap()
                .unwrap();
            for root in &keys {
                let Value::String(atom) = cx.get(root).unwrap() else {
                    panic!("the root holds no String");
                };
                cx.get_property(made, PropertyKey::String(atom))
                    .unwrap()
                    .unwrap();
            }
            cx.root(Value::Object(made)).unwrap()
        })
        .unwrap();
    let needed = agent.heap_bytes().unwrap() - start;
    drop(full);
    assert!(matches!(agent.collect(), Ok(HostOutcome::Normal(_))));
    let mut room = needed + 64;
    let mut runs = 0;
    let mut ended = 0;
    let mut reports = Vec::new();
    loop {
        let (read, terminated, after) = sweep_run(&mut agent, &realm, &keys, room);
        assert_eq!(after, before, "room {room}: {read} reports");
        assert!(terminated || read == SWEPT, "room {room}: {read} reports");
        runs += 1;
        ended += usize::from(terminated);
        if reports.last() != Some(&read) {
            reports.push(read);
        }
        if room < FINE {
            break;
        }
        room -= if room <= FINE_ROOM { FINE } else { COARSE };
    }
    println!(
        "{runs} runs, {needed} bytes for the object and {SWEPT} facts, {ended} ended at the budget; reports made as the room fell: {reports:?}"
    );
    assert_eq!(reports.first(), Some(&SWEPT));
    assert_eq!(reports.last(), Some(&0));
    assert_eq!(agent.state(), AgentState::Alive);
}

/// Host data: the keys of an object whose overrides read and change them, and the change the
/// read of `a` makes.
struct Changing(std::cell::RefCell<Vec<&'static str>>);

impl HostData for Changing {
    fn trace(&self, _tracer: &mut Tracer<'_>) {}
}

/// `get_own_property` of [`Changing`]: an enumerable, configurable data property of 1 for each
/// listed key; reading `a` deletes `b` and adds `d`.
fn changing_property(
    cx: &mut Cx<'_, TestHost>,
    object: ObjectRef,
    key: PropertyKey,
) -> Completion<Option<PropertyDescriptor>> {
    let name = key_text(cx, key);
    let keys = &cx.host_data::<Changing>(object, BRAND).unwrap().unwrap().0;
    let present = keys.borrow().iter().any(|listed| *listed == name);
    if name == "a" {
        let mut keys = keys.borrow_mut();
        keys.retain(|listed| *listed != "b");
        if !keys.contains(&"d") {
            keys.push("d");
        }
    }
    Ok(present.then(|| data(Value::Number(1.0), true, true, true)))
}

/// `own_property_keys` of [`Changing`]: the keys listed now.
fn changing_keys(cx: &mut Cx<'_, TestHost>, object: ObjectRef) -> Completion<Vec<PropertyKey>> {
    let names = cx
        .host_data::<Changing>(object, BRAND)
        .unwrap()
        .unwrap()
        .0
        .borrow()
        .clone();
    names
        .into_iter()
        .map(|name| Ok(PropertyKey::String(cx.new_string(name)?)))
        .collect()
}

static CHANGING: Overrides<TestHost> = Overrides {
    get_own_property: Some(changing_property),
    own_property_keys: Some(changing_keys),
    ..NONE
};

/// EnumerateObjectProperties (`#sec-enumerate-object-properties`): "A property that is deleted
/// before it is processed is ignored. If new properties are added to the target object during
/// enumeration, the newly added properties are not guaranteed to be processed". A for-in over a
/// host object whose `get_own_property` override deletes `b` and adds `d` when `a` is read
/// visits `a` and `c`; the object then lists `a,c,d`.
#[test]
fn a_for_in_over_keys_an_override_deletes_and_adds_skips_the_deleted() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    bind_host(
        &mut agent,
        &realm,
        "o",
        Some(&CHANGING),
        Some(Box::new(Changing(std::cell::RefCell::new(vec![
            "a", "b", "c",
        ])))),
    );
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var r = []; for (var k in o) r.push(k); r.join() + \"/\" + Reflect.ownKeys(o).join();"
        ),
        Outcome::Normal("\"a,c/a,c,d\"".to_string())
    );
    assert_eq!(agent.state(), AgentState::Alive);
}

/// Host data: the object `get_prototype_of` answers.
struct Other(Cell<Option<ObjectRef>>);

impl HostData for Other {
    fn trace(&self, tracer: &mut Tracer<'_>) {
        if let Some(other) = self.0.get() {
            tracer.object(other);
        }
    }
}

/// `get_prototype_of` answering the object of the host data [`Other`].
fn other_prototype(cx: &mut Cx<'_, TestHost>, object: ObjectRef) -> Completion<Option<ObjectRef>> {
    Ok(cx
        .host_data::<Other>(object, BRAND)
        .unwrap()
        .and_then(|other| other.0.get()))
}

static CYCLING: Overrides<TestHost> = Overrides {
    get_prototype_of: Some(other_prototype),
    ..NONE
};

/// The step budget of the prototype cycle, small so that each walk ends soon.
const CYCLE_STEPS: u64 = 20_000;

/// The body of [`a_prototype_cycle_of_two_overrides_ends_each_walk_at_the_step_budget`].
fn prototype_cycle() {
    let mut budget = stack_limits(BUILTINS_REENTRY);
    budget.steps_per_entry = CYCLE_STEPS;
    let mut agent = Agent::new(TestHost::default(), budget, seeds()).unwrap();
    let realm = realm_of(&mut agent);
    let global_root = global(&mut agent, &realm);
    agent
        .with_cx(&realm, |cx| {
            let made = [0, 1].map(|_| {
                cx.new_host_object(spec(
                    Prototype::Null,
                    Some(Box::new(Other(Cell::new(None)))),
                    Some(&CYCLING),
                ))
                .unwrap()
                .unwrap()
            });
            for (index, name) in ["a", "b"].into_iter().enumerate() {
                let other = made[1 - index];
                cx.host_data::<Other>(made[index], BRAND)
                    .unwrap()
                    .unwrap()
                    .0
                    .set(Some(other));
                let global = object_of(cx, &global_root);
                let name = key(cx, name);
                cx.set(global, name, Value::Object(made[index]), true)
                    .unwrap()
                    .unwrap();
            }
        })
        .unwrap();
    let walks = [
        "for (var k in a) {} 0;",
        "\"x\" in a;",
        "a.x;",
        "a instanceof Object;",
        "Object.prototype.isPrototypeOf.call(Object.prototype, a);",
    ];
    for source in walks {
        assert_eq!(
            eval_text(&mut agent, &realm, source),
            Outcome::Terminated(Termination::StepBudget),
            "{source}"
        );
        assert_eq!(agent.state(), AgentState::Alive, "{source}");
    }
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var p = {};\n\
             [Object.setPrototypeOf(p, a) === p, Object.getPrototypeOf(p) === a,\n\
              Reflect.setPrototypeOf(a, p), Reflect.setPrototypeOf(a, {}), Object.getPrototypeOf(a) === b].join();"
        ),
        Outcome::Normal("\"true,true,false,true,true\"".to_string())
    );
}

/// Note 1 of [[GetPrototypeOf]] (`#sec-invariants-of-the-essential-internal-methods`): a chain
/// through objects whose [[GetPrototypeOf]] is not the ordinary one may be circular. Two host
/// objects whose `get_prototype_of` overrides answer each other: a for-in, `in`, a property
/// read, `instanceof` and `isPrototypeOf` each walk the cycle a step per object and end in
/// `Terminated(StepBudget)`, with no Rust stack growth, on seven eighths of `min_stack_bytes()`;
/// `Object.setPrototypeOf` of an ordinary object `p` to one of them stops its loop there (step
/// 7.c.i) and answers true; `Reflect.setPrototypeOf` of that one, which has no
/// `set_prototype_of` override, runs OrdinarySetPrototypeOf on its record: to `p`, whose record
/// leads back to it, false; to a new object, true, while its override still answers the other.
#[test]
fn a_prototype_cycle_of_two_overrides_ends_each_walk_at_the_step_budget() {
    let stack = stack_with_room(stack_limits(BUILTINS_REENTRY));
    on_bounded_stack(
        test_path!("a_prototype_cycle_of_two_overrides_ends_each_walk_at_the_step_budget"),
        stack,
        prototype_cycle,
    )
    .unwrap();
}

/// Host data: the own properties of a host object whose overrides keep them, in the order
/// they were defined.
struct Stored(std::cell::RefCell<Vec<(PropertyKey, PropertyDescriptor)>>);

impl HostData for Stored {
    fn trace(&self, tracer: &mut Tracer<'_>) {
        for (key, descriptor) in self.0.borrow().iter() {
            match *key {
                PropertyKey::String(string) => tracer.value(Value::String(string)),
                PropertyKey::Symbol(symbol) => tracer.value(Value::Symbol(symbol)),
            }
            for value in [descriptor.value, descriptor.get, descriptor.set]
                .into_iter()
                .flatten()
            {
                tracer.value(value);
            }
        }
    }
}

/// The override of `define_own_property` of a global object that refuses `x`, `f` and `z`,
/// recording each refusal, and keeps every other property in its host data [`Stored`]: a new
/// one completed with the defaults, a known one with the fields given replaced.
fn refuse_some(
    cx: &mut Cx<'_, TestHost>,
    object: ObjectRef,
    key: PropertyKey,
    given: PropertyDescriptor,
) -> Completion<bool> {
    let name = key_text(cx, key);
    if matches!(name.as_str(), "x" | "f" | "z") {
        log(cx, format!("refused {name}"));
        return Ok(false);
    }
    let stored = &cx.host_data::<Stored>(object, BRAND).unwrap().unwrap().0;
    let mut stored = stored.borrow_mut();
    match stored.iter_mut().find(|(listed, _)| *listed == key) {
        Some((_, current)) => {
            let fields = [
                (&mut current.value, given.value),
                (&mut current.get, given.get),
                (&mut current.set, given.set),
            ];
            for (field, new) in fields {
                if new.is_some() {
                    *field = new;
                }
            }
            for (field, new) in [
                (&mut current.writable, given.writable),
                (&mut current.enumerable, given.enumerable),
                (&mut current.configurable, given.configurable),
            ] {
                if new.is_some() {
                    *field = new;
                }
            }
        }
        None => {
            let accessor = given.get.is_some() || given.set.is_some();
            let complete = if accessor {
                PropertyDescriptor {
                    get: given.get.or(Some(Value::Undefined)),
                    set: given.set.or(Some(Value::Undefined)),
                    ..given
                }
            } else {
                PropertyDescriptor {
                    value: given.value.or(Some(Value::Undefined)),
                    writable: given.writable.or(Some(false)),
                    ..given
                }
            };
            stored.push((
                key,
                PropertyDescriptor {
                    enumerable: complete.enumerable.or(Some(false)),
                    configurable: complete.configurable.or(Some(false)),
                    ..complete
                },
            ));
        }
    }
    Ok(true)
}

/// `get_own_property` of [`Stored`].
fn stored_property(
    cx: &mut Cx<'_, TestHost>,
    object: ObjectRef,
    key: PropertyKey,
) -> Completion<Option<PropertyDescriptor>> {
    let stored = &cx.host_data::<Stored>(object, BRAND).unwrap().unwrap().0;
    Ok(stored
        .borrow()
        .iter()
        .find(|(listed, _)| *listed == key)
        .map(|(_, descriptor)| *descriptor))
}

static REFUSING_SOME: Overrides<TestHost> = Overrides {
    get_own_property: Some(stored_property),
    define_own_property: Some(refuse_some),
    ..NONE
};

/// A realm whose global object is a host object of [`REFUSING_SOME`].
fn refusing_realm(agent: &mut Agent<TestHost>) -> Realm {
    let mut setup = |cx: &mut HookCx<'_, TestHost>, realm: RealmRef| -> Completion<GlobalObjects> {
        let global = cx
            .new_host_object(spec(
                Prototype::ObjectPrototypeOf(realm),
                Some(Box::new(Stored(std::cell::RefCell::new(Vec::new())))),
                Some(&REFUSING_SOME),
            ))
            .unwrap()?;
        Ok(GlobalObjects { global, this: None })
    };
    match create_realm(agent, Some(&mut setup), None).unwrap() {
        HostOutcome::Normal(realm) => realm,
        other => panic!("create_realm answered {other:?}"),
    }
}

/// GlobalDeclarationInstantiation (`#sec-globaldeclarationinstantiation`) with a global object
/// whose `define_own_property` override refuses `x`, `f` and `z` and keeps every other
/// property, which its `get_own_property` answers: CanDeclareGlobalFunction and
/// CanDeclareGlobalVar find no such property on an extensible object and pass; the lexical `y` is created at
/// step 15; CreateGlobalFunctionBinding of `f` (step 17) throws the TypeError of
/// DefinePropertyOrThrow before `x` (step 18) is reached, so `y` stays uninitialized and `x`
/// unbound. A script of `var x` alone throws at step 18. An undeclared assignment `z = 1`
/// reaches the override through OrdinarySet's CreateDataProperty, whose false a sloppy script
/// ignores; a strict one throws a ReferenceError before any define.
#[test]
fn a_host_global_object_that_refuses_declarations_throws_in_the_order_of_the_steps() {
    let mut agent = agent().unwrap();
    let realm = refusing_realm(&mut agent);
    take_calls(&mut agent);
    let cases: [(&str, &str, &[&str]); 5] = [
        (
            "let y = 1; function f() {} var x;",
            "TypeError",
            &["refused f"],
        ),
        (
            "var r = []; try { y; } catch (e) { r.push(e.name); } r.push(typeof x); r.join();",
            "\"ReferenceError,undefined\"",
            &[],
        ),
        ("var x;", "TypeError", &["refused x"]),
        ("z = 1; typeof z;", "\"undefined\"", &["refused z"]),
        ("\"use strict\"; z = 1;", "ReferenceError", &[]),
    ];
    for (source, answer, calls) in cases {
        let outcome = eval_text(&mut agent, &realm, source);
        let shown = match &outcome {
            Outcome::Normal(text) => text.clone(),
            Outcome::Throw(text) => text.split(':').next().unwrap_or_default().to_string(),
            other => format!("{other:?}"),
        };
        assert_eq!(shown, answer, "{source}: {outcome:?}");
        assert_eq!(take_calls(&mut agent), calls, "{source}");
    }
    assert_eq!(agent.state(), AgentState::Alive);
}

/// `get_prototype_of` running [`run_hook`], then answering the object of the host data
/// [`Other`].
fn hooked_prototype(cx: &mut Cx<'_, TestHost>, object: ObjectRef) -> Completion<Option<ObjectRef>> {
    run_hook(cx)?;
    Ok(cx
        .host_data::<Other>(object, BRAND)
        .unwrap()
        .and_then(|other| other.0.get()))
}

static HOOKED_PROTOTYPE: Overrides<TestHost> = Overrides {
    get_prototype_of: Some(hooked_prototype),
    ..NONE
};

/// `get_own_property` answering `"v"` for the key `4294967295`, absent for every other key.
fn far_element(
    cx: &mut Cx<'_, TestHost>,
    _object: ObjectRef,
    key: PropertyKey,
) -> Completion<Option<PropertyDescriptor>> {
    if key_text(cx, key) != "4294967295" {
        return Ok(None);
    }
    let v = Value::String(cx.new_string("v")?);
    Ok(Some(data(v, true, true, true)))
}

static FAR_ELEMENT: Overrides<TestHost> = Overrides {
    get_own_property: Some(far_element),
    ..NONE
};

/// Bind as the global `h` a host object of [`HOOKED_PROTOTYPE`] whose `get_prototype_of`
/// answers `next`, with `hook_body` as the body of the global function `hook` it runs.
fn bind_hooked_prototype(
    agent: &mut Agent<TestHost>,
    realm: &Realm,
    next: Option<&'static Overrides<TestHost>>,
    hook_body: &str,
) {
    let global_root = global(agent, realm);
    agent
        .with_cx(realm, |cx| {
            let next = next.map(|table| {
                cx.new_host_object(spec(Prototype::Null, None, Some(table)))
                    .unwrap()
                    .unwrap()
            });
            let made = cx
                .new_host_object(spec(
                    Prototype::Null,
                    Some(Box::new(Other(Cell::new(next)))),
                    Some(&HOOKED_PROTOTYPE),
                ))
                .unwrap()
                .unwrap();
            let global = object_of(cx, &global_root);
            let name = key(cx, "h");
            cx.set(global, name, Value::Object(made), true)
                .unwrap()
                .unwrap();
        })
        .unwrap();
    assert_eq!(
        eval_text(
            agent,
            realm,
            &format!(
                "function hook() {{ {hook_body} var junk = []; for (var i = 0; i < 20; i++) junk.push({{ i: i }}); junk = null; }}\n\
                 0;"
            )
        ),
        Outcome::Normal("0".to_string())
    );
}

/// OrdinaryHasProperty and OrdinaryGet (`object/ordinary.rs`) walk on past a host object
/// without a `has_property` or `get` override and call its `get_prototype_of` override, which
/// runs a script that allocates (with `collect-every-safe-point`, collecting at each
/// instruction). `indexOf` and `lastIndexOf` at the index 2^32 - 1 hold the key of that index,
/// a String made for the step and held by nothing else, across that call: the walk keeps it, so
/// the next object of the chain, a host object whose `get_own_property` answers `"v"` for it, is
/// asked for that key and the element is found.
#[test]
fn a_key_held_by_a_walk_across_a_get_prototype_of_override_survives_a_collection() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    bind_hooked_prototype(&mut agent, &realm, Some(&FAR_ELEMENT), "");
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var al = Object.create(h); al.length = 4294967297; 0;"
        ),
        Outcome::Normal("0".to_string())
    );
    set_hook(&mut agent, &realm, "hook");
    let outcome = eval_text(
        &mut agent,
        &realm,
        "var at = 4294967295;\n\
         [Array.prototype.indexOf.call(al, \"v\", at), Array.prototype.lastIndexOf.call(al, \"v\", at)].join();",
    );
    HOOK.with(|cell| cell.set(None));
    assert_eq!(
        outcome,
        Outcome::Normal("\"4294967295,4294967295\"".to_string())
    );
    assert_eq!(agent.state(), AgentState::Alive);
}

/// OrdinarySet (`object/ordinary.rs`) walks on past a host object without a `set` override and
/// calls its `get_prototype_of` override, which runs a script that deletes the only other
/// reference to the value being set and allocates (with `collect-every-safe-point`, collecting
/// at each instruction). `copyWithin` reads element 0 of `o` and sets it as element 1 at once:
/// the walk keeps the value, so element 1 is the object element 0 held.
#[test]
fn a_value_held_by_a_set_walk_across_a_get_prototype_of_override_survives_a_collection() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    bind_hooked_prototype(&mut agent, &realm, None, "if (o) delete o[0];");
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var o = null; var made = Object.create(h); made[0] = { tag: 7 }; made.length = 2; o = made; made = null; 0;"
        ),
        Outcome::Normal("0".to_string())
    );
    set_hook(&mut agent, &realm, "hook");
    let outcome = eval_text(
        &mut agent,
        &realm,
        "Array.prototype.copyWithin.call(o, 1, 0);\n\
         for (var j = 0; j < 20; j++) ({});\n\
         [0 in o, typeof o[1], o[1] && o[1].tag].join();",
    );
    HOOK.with(|cell| cell.set(None));
    assert_eq!(outcome, Outcome::Normal("\"false,object,7\"".to_string()));
    assert_eq!(agent.state(), AgentState::Alive);
}

static SEALED_ABSENT: Overrides<TestHost> = Overrides {
    is_extensible: Some(not_extensible),
    get_own_property: Some(always_absent),
    ..NONE
};

/// The bytes of one key non-existent in the box of facts: an entry of its table, the key and
/// itself (32 bytes), and its control byte. The unit test
/// `object::host_overrides::tests::a_fact_has_the_bytes_the_tests_name` holds the 32.
const ABSENT_BYTES: u64 = 33;

/// `Object.isExtensible(target)` through the global `Object` of the realm of `global_root`.
fn is_extensible_of(
    cx: &mut Cx<'_, TestHost>,
    global_root: &swell::Root,
    target: ObjectRef,
) -> Value {
    let global = object_of(cx, global_root);
    let name = key(cx, "Object");
    let Value::Object(constructor) = cx.get_property(global, name).unwrap().unwrap() else {
        panic!("Object is no object");
    };
    let name = key(cx, "isExtensible");
    let function = cx.get_property(constructor, name).unwrap().unwrap();
    cx.call(function, Value::Undefined, &[Value::Object(target)])
        .unwrap()
        .unwrap()
}

/// Bounds, "Memory", for the keys non-existent: once a host object reported itself not
/// extensible, each of [`FACTS`] keys its `get_own_property` override reports absent is recorded
/// in its box of facts, charged before it grows: `heap_bytes()` rises by at least [`FACTS`]
/// times [`ABSENT_BYTES`], and falls back once the object is freed; asking the same keys again
/// adds nothing.
#[test]
fn the_keys_absent_from_a_host_object_are_charged_and_credited() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let global_root = global(&mut agent, &realm);
    let (object, keys) = facts_setup_with(&mut agent, &realm, &SEALED_ABSENT);
    agent
        .with_cx(&realm, |cx| {
            let target = object_of(cx, &object);
            assert_eq!(
                is_extensible_of(cx, &global_root, target),
                Value::Boolean(false)
            );
        })
        .unwrap();
    assert!(matches!(agent.collect(), Ok(HostOutcome::Normal(_))));
    let before = agent.heap_bytes().unwrap();
    let (read, ended) = report_values(&mut agent, &realm, &object, &keys, Value::Undefined);
    assert_eq!((read, ended), (FACTS, Ok(())));
    let with_absent = agent.heap_bytes().unwrap();
    let (read, ended) = report_values(&mut agent, &realm, &object, &keys, Value::Undefined);
    assert_eq!((read, ended), (FACTS, Ok(())));
    let again = agent.heap_bytes().unwrap();
    drop(object);
    assert!(matches!(agent.collect(), Ok(HostOutcome::Normal(_))));
    let after = agent.heap_bytes().unwrap();
    let facts = u64::try_from(FACTS).unwrap();
    println!(
        "heap_bytes: {before} before, {with_absent} with {FACTS} keys non-existent (+{}, at least {}), {again} after asking them again, {after} after the object is freed",
        with_absent - before,
        facts * ABSENT_BYTES
    );
    assert!(with_absent - before >= facts * ABSENT_BYTES);
    assert_eq!(again, with_absent);
    assert!(after < before, "{after} after, {before} before");
}

/// `get_own_property` answering, for the Symbol of the host data [`Unlisted`], absent, or a
/// data property once `change` is set; absent for every other key.
fn unlisted_absent(
    cx: &mut Cx<'_, TestHost>,
    object: ObjectRef,
    key: PropertyKey,
) -> Completion<Option<PropertyDescriptor>> {
    let (symbol, change) = {
        let held = cx.host_data::<Unlisted>(object, BRAND).unwrap().unwrap();
        (held.symbol.get(), held.change.get())
    };
    let present = change && PropertyKey::Symbol(symbol.unwrap()) == key;
    Ok(present.then(|| data(Value::Number(1.0), true, true, true)))
}

static UNLISTED_ABSENT: Overrides<TestHost> = Overrides {
    is_extensible: Some(not_extensible),
    get_own_property: Some(unlisted_absent),
    ..NONE
};

/// The kind site traces the keys non-existent of the box of facts: a host object that reported
/// itself not extensible reports a Symbol key absent, the Symbol held only by its host data,
/// which does not list it; after `collect()` the Symbol still reaches its record (a read of it
/// answers undefined, not `StaleHandle`), and a later answer that reports it present ends the
/// run in `Terminated(EngineFault)`.
#[test]
fn the_keys_absent_from_a_host_object_survive_a_collection() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let global_root = global(&mut agent, &realm);
    let holder = agent
        .with_cx(&realm, |cx| {
            let made = cx
                .new_host_object(spec(
                    Prototype::Null,
                    Some(Box::new(Unlisted {
                        value: Cell::new(None),
                        symbol: Cell::new(None),
                        change: Cell::new(false),
                    })),
                    Some(&UNLISTED_ABSENT),
                ))
                .unwrap()
                .unwrap();
            let global = object_of(cx, &global_root);
            let symbol_key = key(cx, "Symbol");
            let constructor = cx.get_property(global, symbol_key).unwrap().unwrap();
            let description = Value::String(cx.new_string("absent").unwrap());
            let Value::Symbol(symbol) = cx
                .call(constructor, Value::Undefined, &[description])
                .unwrap()
                .unwrap()
            else {
                panic!("Symbol() made no Symbol");
            };
            cx.host_data::<Unlisted>(made, BRAND)
                .unwrap()
                .unwrap()
                .symbol
                .set(Some(symbol));
            assert_eq!(
                is_extensible_of(cx, &global_root, made),
                Value::Boolean(false)
            );
            let read = cx
                .get_property(made, PropertyKey::Symbol(symbol))
                .unwrap()
                .unwrap();
            assert_eq!(read, Value::Undefined);
            cx.root(Value::Object(made)).unwrap()
        })
        .unwrap();
    assert!(matches!(agent.collect(), Ok(HostOutcome::Normal(_))));
    agent
        .with_cx(&realm, |cx| {
            let Value::Object(made) = cx.get(&holder).unwrap() else {
                panic!("the root holds no object");
            };
            let held = cx.host_data::<Unlisted>(made, BRAND).unwrap().unwrap();
            let symbol = held.symbol.get().unwrap();
            let read = cx.get_property(made, PropertyKey::Symbol(symbol)).unwrap();
            assert_eq!(read, Ok(Value::Undefined));
            cx.host_data::<Unlisted>(made, BRAND)
                .unwrap()
                .unwrap()
                .change
                .set(true);
            let changed = cx.get_property(made, PropertyKey::Symbol(symbol)).unwrap();
            assert_eq!(changed, Err(Abrupt::Terminated(Termination::EngineFault)));
        })
        .unwrap();
    assert_eq!(agent.state(), AgentState::Faulted);
}
thread_local! {
    /// The address of a local of each call of [`get_calls_back`], innermost last.
    static DEPTHS: std::cell::RefCell<Vec<usize>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// `get` calling the function its host data holds through `cx.call`, after recording the
/// address of one of its locals.
fn get_calls_back(
    cx: &mut Cx<'_, TestHost>,
    object: ObjectRef,
    _key: PropertyKey,
    _receiver: Value,
) -> Completion<Value> {
    let marker = 0_u8;
    let address = std::hint::black_box(&marker) as *const u8 as usize;
    DEPTHS.with(|depths| depths.borrow_mut().push(address));
    let function = cx
        .host_data::<Next>(object, BRAND)
        .unwrap()
        .and_then(|next| next.0);
    match function {
        Some(function) => cx
            .call(Value::Object(function), Value::Undefined, &[])
            .unwrap(),
        None => Ok(Value::Undefined),
    }
}

static CALLING_BACK: Overrides<TestHost> = Overrides {
    get: Some(get_calls_back),
    ..NONE
};

/// Bind as the global `o` a host object whose `get` calls the global function `name`.
fn bind_calling_back(agent: &mut Agent<TestHost>, realm: &Realm, name: &str) {
    let Value::Object(function) = global_value(agent, realm, name) else {
        panic!("{name} is no object");
    };
    let global_root = global(agent, realm);
    agent
        .with_cx(realm, |cx| {
            let made = cx
                .new_host_object(spec(
                    Prototype::Null,
                    Some(Box::new(Next(Some(function)))),
                    Some(&CALLING_BACK),
                ))
                .unwrap()
                .unwrap();
            let global = object_of(cx, &global_root);
            let name = key(cx, "o");
            cx.set(global, name, Value::Object(made), true)
                .unwrap()
                .unwrap();
        })
        .unwrap();
}

/// The bytes of the Rust stack between consecutive calls of [`get_calls_back`]: the median
/// of the differences, and how many re-entries one cycle makes.
fn bytes_per_cycle() -> usize {
    let mut steps = DEPTHS.with(|depths| {
        depths
            .borrow()
            .windows(2)
            .map(|pair| pair[0].abs_diff(pair[1]))
            .collect::<Vec<_>>()
    });
    steps.sort_unstable();
    steps.get(steps.len() / 2).copied().unwrap_or(0)
}

/// The body of [`an_override_script_host_function_chain_ends_at_the_reentry_limit`].
fn override_script_host_function_chain() {
    let (mut agent, realm) = stack_agent(BUILTINS_REENTRY);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var n = 0; function g() { n++; return o.x; } function f() { return reenter(g); }\n\
             function direct() { n++; return o.x; }\n\
             0;"
        ),
        Outcome::Normal("0".to_string())
    );
    bind_calling_back(&mut agent, &realm, "f");
    DEPTHS.with(|depths| depths.borrow_mut().clear());
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var caught;\n\
             try { o.x; caught = \"none\"; }\n\
             catch (e) { caught = e instanceof RangeError ? \"RangeError\" : String(e); }\n\
             caught;"
        ),
        Outcome::Normal("\"RangeError\"".to_string())
    );
    let through_host_function = bytes_per_cycle();
    let cycles = DEPTHS.with(|depths| depths.borrow().len());
    bind_calling_back(&mut agent, &realm, "direct");
    DEPTHS.with(|depths| depths.borrow_mut().clear());
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var caught;\n\
             try { o.x; caught = \"none\"; }\n\
             catch (e) { caught = e instanceof RangeError ? \"RangeError\" : String(e); }\n\
             caught;"
        ),
        Outcome::Normal("\"RangeError\"".to_string())
    );
    let through_script = bytes_per_cycle();
    let direct_cycles = DEPTHS.with(|depths| depths.borrow().len());
    println!(
        "stack per cycle at reentry_depth {BUILTINS_REENTRY}: override -> script -> host function -> script -> override {through_host_function} bytes over {cycles} cycles; override -> script -> override {through_script} bytes over {direct_cycles} cycles; STACK_BYTES_PER_REENTRY 14336"
    );
    assert_eq!(agent.state(), AgentState::Alive);
}

/// `docs/design/engine.md`, decision 3, "Depth bounds", and the module documentation of
/// `object/host_overrides.rs`: a chain override → script → host function → script → override
/// … and a chain override → script → override … each end in the RangeError of the re-entry
/// limit, caught by the script, on seven eighths of `min_stack_bytes()`, and the agent stays
/// usable. The bytes of a cycle are printed.
#[test]
fn an_override_script_host_function_chain_ends_at_the_reentry_limit() {
    let stack = stack_with_room(stack_limits(BUILTINS_REENTRY));
    on_bounded_stack(
        test_path!("an_override_script_host_function_chain_ends_at_the_reentry_limit"),
        stack,
        override_script_host_function_chain,
    )
    .unwrap();
}

// A host object whose overrides ask a script handler, as a Proxy asks its handler; a table that
// leaves one internal method out answers that method from the object's record, and that answer
// is held to the facts as an override's is.

thread_local! {
    /// The script handler of [`JS_ALL`] and its variants.
    static JS_HANDLER: Cell<Option<ObjectRef>> = const { Cell::new(None) };
}

/// The handle a key holds, as a value.
fn js_key_value(key: PropertyKey) -> Value {
    match key {
        PropertyKey::String(string) => Value::String(string),
        PropertyKey::Symbol(symbol) => Value::Symbol(symbol),
    }
}

/// `undefined` or the Boolean of a field.
fn js_flag(field: Option<bool>) -> Value {
    field.map_or(Value::Undefined, Value::Boolean)
}

/// Call the method `name` of the script handler with `args`.
fn js_trap(cx: &mut Cx<'_, TestHost>, name: &str, args: &[Value]) -> Completion<Value> {
    let handler = JS_HANDLER.with(Cell::get).unwrap();
    let name = key(cx, name);
    let method = cx.get_property(handler, name).unwrap()?;
    cx.call(method, Value::Object(handler), args).unwrap()
}

/// A descriptor object of a script as the host's descriptor; `undefined` is absent.
fn js_descriptor(
    cx: &mut Cx<'_, TestHost>,
    value: Value,
) -> Completion<Option<PropertyDescriptor>> {
    let Value::Object(descriptor) = value else {
        return Ok(None);
    };
    let mut out = PropertyDescriptor::default();
    for name in [
        "value",
        "writable",
        "get",
        "set",
        "enumerable",
        "configurable",
    ] {
        let field = key(cx, name);
        if !cx.has_property(descriptor, field).unwrap()? {
            continue;
        }
        let read = cx.get_property(descriptor, field).unwrap()?;
        let flag = Some(read == Value::Boolean(true));
        match name {
            "value" => out.value = Some(read),
            "get" => out.get = Some(read),
            "set" => out.set = Some(read),
            "writable" => out.writable = flag,
            "enumerable" => out.enumerable = flag,
            _ => out.configurable = flag,
        }
    }
    Ok(Some(out))
}

fn js_get_prototype_of(
    cx: &mut Cx<'_, TestHost>,
    _object: ObjectRef,
) -> Completion<Option<ObjectRef>> {
    match js_trap(cx, "getPrototypeOf", &[])? {
        Value::Object(prototype) => Ok(Some(prototype)),
        _ => Ok(None),
    }
}

fn js_set_prototype_of(
    cx: &mut Cx<'_, TestHost>,
    _object: ObjectRef,
    prototype: Option<ObjectRef>,
) -> Completion<bool> {
    let given = prototype.map_or(Value::Null, Value::Object);
    Ok(js_trap(cx, "setPrototypeOf", &[given])? == Value::Boolean(true))
}

fn js_is_extensible(cx: &mut Cx<'_, TestHost>, _object: ObjectRef) -> Completion<bool> {
    Ok(js_trap(cx, "isExtensible", &[])? == Value::Boolean(true))
}

fn js_prevent_extensions(cx: &mut Cx<'_, TestHost>, _object: ObjectRef) -> Completion<bool> {
    Ok(js_trap(cx, "preventExtensions", &[])? == Value::Boolean(true))
}

fn js_get_own_property(
    cx: &mut Cx<'_, TestHost>,
    _object: ObjectRef,
    key: PropertyKey,
) -> Completion<Option<PropertyDescriptor>> {
    let answer = js_trap(cx, "getOwnPropertyDescriptor", &[js_key_value(key)])?;
    js_descriptor(cx, answer)
}

fn js_define_own_property(
    cx: &mut Cx<'_, TestHost>,
    _object: ObjectRef,
    key: PropertyKey,
    descriptor: PropertyDescriptor,
) -> Completion<bool> {
    let args = [
        js_key_value(key),
        descriptor.value.unwrap_or(Value::Undefined),
        js_flag(descriptor.configurable),
        js_flag(descriptor.writable),
    ];
    Ok(js_trap(cx, "defineProperty", &args)? == Value::Boolean(true))
}

fn js_has_property(
    cx: &mut Cx<'_, TestHost>,
    _object: ObjectRef,
    key: PropertyKey,
) -> Completion<bool> {
    Ok(js_trap(cx, "has", &[js_key_value(key)])? == Value::Boolean(true))
}

fn js_get(
    cx: &mut Cx<'_, TestHost>,
    _object: ObjectRef,
    key: PropertyKey,
    receiver: Value,
) -> Completion<Value> {
    js_trap(cx, "get", &[js_key_value(key), receiver])
}

fn js_set(
    cx: &mut Cx<'_, TestHost>,
    _object: ObjectRef,
    key: PropertyKey,
    value: Value,
    receiver: Value,
) -> Completion<bool> {
    Ok(js_trap(cx, "set", &[js_key_value(key), value, receiver])? == Value::Boolean(true))
}

fn js_delete(cx: &mut Cx<'_, TestHost>, _object: ObjectRef, key: PropertyKey) -> Completion<bool> {
    Ok(js_trap(cx, "deleteProperty", &[js_key_value(key)])? == Value::Boolean(true))
}

fn js_own_property_keys(
    cx: &mut Cx<'_, TestHost>,
    _object: ObjectRef,
) -> Completion<Vec<PropertyKey>> {
    let Value::Object(list) = js_trap(cx, "ownKeys", &[])? else {
        return Ok(Vec::new());
    };
    let length = key(cx, "length");
    let Value::Number(length) = cx.get_property(list, length).unwrap()? else {
        return Ok(Vec::new());
    };
    let mut keys = Vec::new();
    for index in 0..(length as usize) {
        let element = key(cx, &index.to_string());
        let element = cx.get_property(list, element).unwrap()?;
        keys.push(cx.to_property_key(element).unwrap()?);
    }
    Ok(keys)
}

/// Every override, each asking the script handler.
const JS_ALL: Overrides<TestHost> = Overrides {
    get_prototype_of: Some(js_get_prototype_of),
    set_prototype_of: Some(js_set_prototype_of),
    is_extensible: Some(js_is_extensible),
    prevent_extensions: Some(js_prevent_extensions),
    get_own_property: Some(js_get_own_property),
    define_own_property: Some(js_define_own_property),
    has_property: Some(js_has_property),
    get: Some(js_get),
    set: Some(js_set),
    delete: Some(js_delete),
    own_property_keys: Some(js_own_property_keys),
};

static JS_EVERY: Overrides<TestHost> = JS_ALL;
static JS_NO_SET_PROTOTYPE_OF: Overrides<TestHost> = Overrides {
    set_prototype_of: None,
    ..JS_ALL
};
static JS_NO_PREVENT_EXTENSIONS: Overrides<TestHost> = Overrides {
    prevent_extensions: None,
    ..JS_ALL
};
static JS_NO_OWN_PROPERTY_KEYS: Overrides<TestHost> = Overrides {
    own_property_keys: None,
    ..JS_ALL
};
static JS_NO_DELETE: Overrides<TestHost> = Overrides {
    delete: None,
    ..JS_ALL
};

/// The functions every handler script below uses: `base(over)` is a handler with an answer
/// for every trap (an empty extensible object with a null prototype) and the methods of
/// `over`; `seq(list)` answers the next element of `list` on each call, its last one after;
/// `D` and `A` make data and accessor descriptors.
const JS_PRELUDE: &str = "var P1 = {}, P2 = {}, G1 = function () {}, G2 = function () {}, S1 = Symbol(\"a\");\n\
     function seq(list) { var i = 0; return function () { var v = list[Math.min(i, list.length - 1)]; i++; return v; }; }\n\
     function D(v, w, e, c) { return { value: v, writable: w, enumerable: e, configurable: c }; }\n\
     function A(g, s, e, c) { return { get: g, set: s, enumerable: e, configurable: c }; }\n\
     function base(over) {\n\
       var h = { getPrototypeOf: function () { return null; }, setPrototypeOf: function () { return true; },\n\
         isExtensible: function () { return true; }, preventExtensions: function () { return false; },\n\
         getOwnPropertyDescriptor: function () { return undefined; }, defineProperty: function () { return true; },\n\
         has: function () { return false; }, get: function () { return undefined; }, set: function () { return true; },\n\
         deleteProperty: function () { return true; }, ownKeys: function () { return []; } };\n\
       for (var k in over) h[k] = over[k];\n\
       return h;\n\
     }\n";

/// A fresh agent with `o`, a host object of `table` whose handler is the script expression
/// `handler`; then `script`: its outcome and the agent's state.
fn js_run(
    table: &'static Overrides<TestHost>,
    handler: &str,
    script: &str,
) -> (Outcome, AgentState) {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let setup = format!("{JS_PRELUDE}var H = {handler};\n0;");
    assert_eq!(
        eval_text(&mut agent, &realm, &setup),
        Outcome::Normal("0".to_string()),
        "{handler}"
    );
    let Value::Object(handler_object) = global_value(&mut agent, &realm, "H") else {
        panic!("H is no object");
    };
    JS_HANDLER.with(|cell| cell.set(Some(handler_object)));
    bind_host(&mut agent, &realm, "o", Some(table), None);
    let outcome = eval_text(&mut agent, &realm, script);
    JS_HANDLER.with(|cell| cell.set(None));
    (outcome, agent.state())
}

/// [[SetPrototypeOf]]: "If target is non-extensible, [[SetPrototypeOf]] must return false,
/// unless proto is the SameValue as the target's observed [[GetPrototypeOf]] value." The table
/// has `is_extensible` (false) and `get_prototype_of` (`P1`) but no `set_prototype_of`, so
/// OrdinarySetPrototypeOf answers for the object from its record, which is still extensible:
/// `Reflect.setPrototypeOf(o, P2)` answers true after the object reported itself not extensible
/// and its prototype `P1`. That answer must fault the agent.
#[test]
fn set_prototype_of_from_the_record_after_a_non_extensible_report_faults() {
    let (outcome, state) = js_run(
        &JS_NO_SET_PROTOTYPE_OF,
        "base({ isExtensible: function () { return false; }, getPrototypeOf: function () { return P1; } })",
        "[Object.isExtensible(o), Object.getPrototypeOf(o) === P1, Reflect.setPrototypeOf(o, P2)].join();",
    );
    assert_eq!(
        (outcome, state),
        (
            Outcome::Terminated(Termination::EngineFault),
            AgentState::Faulted
        )
    );
}

/// [[PreventExtensions]]: "If [[PreventExtensions]] returns true, all future calls to
/// [[IsExtensible]] on the target must return false and the target is now considered
/// non-extensible." The table has `is_extensible` but no `prevent_extensions`, so
/// OrdinaryPreventExtensions answers true for the object; `is_extensible` answering true after
/// it must fault the agent, and so must a key reported absent after it and then present.
#[test]
fn prevent_extensions_true_from_the_record_makes_the_object_non_extensible() {
    let extensible_after = js_run(
        &JS_NO_PREVENT_EXTENSIONS,
        "base({ isExtensible: function () { return true; } })",
        "[Reflect.preventExtensions(o), Object.isExtensible(o)].join();",
    );
    let absent_then_present = js_run(
        &JS_NO_PREVENT_EXTENSIONS,
        "base({ isExtensible: function () { return false; }, getOwnPropertyDescriptor: seq([undefined, D(1, true, true, true)]) })",
        "[Reflect.preventExtensions(o), Object.getOwnPropertyDescriptor(o, \"x\"), Object.getOwnPropertyDescriptor(o, \"x\").value].join();",
    );
    let faulted = (
        Outcome::Terminated(Termination::EngineFault),
        AgentState::Faulted,
    );
    assert_eq!(
        (extensible_after, absent_then_present),
        (faulted.clone(), faulted)
    );
}

/// [[OwnPropertyKeys]]: "The returned List must contain at least the keys of all
/// non-configurable own properties that have previously been observed." The table has
/// `get_own_property` (`x` non-configurable) but no `own_property_keys`, so
/// OrdinaryOwnPropertyKeys answers the record's keys, which do not hold `x`. That list must
/// fault the agent, as the same list from an override does (the module documentation: "every key
/// list holds it").
#[test]
fn the_record_key_list_must_hold_an_observed_non_configurable_key() {
    let (outcome, state) = js_run(
        &JS_NO_OWN_PROPERTY_KEYS,
        "base({ getOwnPropertyDescriptor: function () { return D(1, true, true, false); } })",
        "[Object.getOwnPropertyDescriptor(o, \"x\").configurable, Reflect.ownKeys(o).length].join();",
    );
    assert_eq!(
        (outcome, state),
        (
            Outcome::Terminated(Termination::EngineFault),
            AgentState::Faulted
        )
    );
}

/// [[Delete]]: the `delete` override answering true for a configurable key of an object that
/// reported itself not extensible makes the key non-existent, so a later `get_own_property`
/// that describes it faults (the module's rule, as the Proxy's deleteProperty check). The table
/// has no `delete`, so OrdinaryDelete answers true for the object from its record; the same
/// later answer must fault the agent.
#[test]
fn delete_true_from_the_record_on_a_non_extensible_object_makes_the_key_absent() {
    let handler = "base({ isExtensible: function () { return false; }, getOwnPropertyDescriptor: function () { return D(1, true, true, true); } })";
    let script = "[Object.isExtensible(o), Reflect.deleteProperty(o, \"x\"), Object.getOwnPropertyDescriptor(o, \"x\").value].join();";
    let faulted = (
        Outcome::Terminated(Termination::EngineFault),
        AgentState::Faulted,
    );
    assert_eq!(
        (
            js_run(&JS_EVERY, handler, script),
            js_run(&JS_NO_DELETE, handler, script)
        ),
        (faulted.clone(), faulted)
    );
}

/// [[HasProperty]] answering false is not one of the clause's observations of a non-existent
/// property ("A non-existent property is a property that does not exist as an own property on
/// a non-extensible target"; the one bullet of [[HasProperty]] binds only a non-configurable
/// key). A host object that reported itself not extensible, hides a configurable `x` from
/// `in`, and describes it through [[GetOwnProperty]] breaks no bullet.
#[test]
fn has_property_false_is_not_an_observation_of_a_non_existent_key() {
    let (outcome, state) = js_run(
        &JS_EVERY,
        "base({ isExtensible: function () { return false; }, getOwnPropertyDescriptor: function () { return D(1, true, true, true); } })",
        "[Object.isExtensible(o), \"x\" in o, Object.getOwnPropertyDescriptor(o, \"x\").value].join();",
    );
    assert_eq!(
        (outcome, state),
        (
            Outcome::Normal("\"false,false,1\"".to_string()),
            AgentState::Alive
        )
    );
}

/// [[HasProperty]]: "If P was previously observed as a non-configurable own data or accessor
/// property of the target, [[HasProperty]] must return true." The object reports `x`
/// non-configurable, then `in` answers false for it: the agent faults (the false of any other
/// key records nothing).
#[test]
fn has_property_false_for_a_key_observed_non_configurable_faults_the_agent() {
    let (outcome, state) = js_run(
        &JS_EVERY,
        "base({ getOwnPropertyDescriptor: function () { return D(1, true, true, false); } })",
        "[Object.getOwnPropertyDescriptor(o, \"x\").configurable, \"x\" in o].join();",
    );
    assert_eq!(
        (outcome, state),
        (
            Outcome::Terminated(Termination::EngineFault),
            AgentState::Faulted
        )
    );
}

/// [[OwnPropertyKeys]] of an object that reported itself not extensible: the first list holds
/// `a` and `b`, the second leaves `b` out, so `b` is non-existent; the third, which holds `b`
/// again, faults the agent.
#[test]
fn a_key_a_later_list_leaves_out_does_not_come_back() {
    let (outcome, state) = js_run(
        &JS_EVERY,
        "base({ isExtensible: function () { return false; }, ownKeys: seq([[\"a\", \"b\"], [\"a\"], [\"a\", \"b\"]]) })",
        "Object.isExtensible(o); Reflect.ownKeys(o); Reflect.ownKeys(o); Reflect.ownKeys(o).length;",
    );
    assert_eq!(
        (outcome, state),
        (
            Outcome::Terminated(Termination::EngineFault),
            AgentState::Faulted
        )
    );
}

/// [[GetOwnProperty]] of an object that reported itself not extensible and listed `a`: `a`
/// reported absent is non-existent from then on, so a later descriptor for it faults the agent.
#[test]
fn a_listed_key_reported_absent_does_not_come_back() {
    let (outcome, state) = js_run(
        &JS_EVERY,
        "base({ isExtensible: function () { return false; }, ownKeys: function () { return [\"a\"]; }, getOwnPropertyDescriptor: seq([undefined, D(1, true, true, true)]) })",
        "Object.isExtensible(o); Reflect.ownKeys(o); Object.getOwnPropertyDescriptor(o, \"a\"); Object.getOwnPropertyDescriptor(o, \"a\");",
    );
    assert_eq!(
        (outcome, state),
        (
            Outcome::Terminated(Termination::EngineFault),
            AgentState::Faulted
        )
    );
}

thread_local! {
    /// The script function [`fresh_current`] calls for the value it answers.
    static MAKE: Cell<Option<ObjectRef>> = const { Cell::new(None) };
}

/// `get_own_property` answering a configurable data property `x` whose value is a new object
/// made by the script function [`MAKE`] for this answer, once the host's `count` is set; absent
/// before.
fn fresh_current(
    cx: &mut Cx<'_, TestHost>,
    _object: ObjectRef,
    key: PropertyKey,
) -> Completion<Option<PropertyDescriptor>> {
    if cx.host_mut().count == 0 || key_text(cx, key) != "x" {
        return Ok(None);
    }
    let make = MAKE.with(Cell::get).unwrap();
    let value = cx
        .call(Value::Object(make), Value::Undefined, &[])
        .unwrap()?;
    Ok(Some(data(value, true, true, true)))
}

static FRESH_CURRENT: Overrides<TestHost> = Overrides {
    is_extensible: Some(extensible_after_hook),
    get_own_property: Some(fresh_current),
    ..NONE
};

/// OrdinaryDefineOwnProperty of a host object without `define_own_property`
/// (`held_ordinary_define_own_property`) reads `current` from a `get_own_property` override,
/// which answers a new object held by nothing else, then calls the `is_extensible` override,
/// whose script allocates (with `collect-every-safe-point`, collecting at each instruction).
/// `{ enumerable: false }` keeps the value of `current` (ValidateAndApplyPropertyDescriptor,
/// step 6), which is written into the record; the next collection must find it alive.
#[test]
fn the_current_value_held_across_an_is_extensible_override_survives_a_collection() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    bind_host(&mut agent, &realm, "o", Some(&FRESH_CURRENT), None);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "function hook() { var junk = []; for (var i = 0; i < 20; i++) junk.push({ i: i }); junk = null; }\n\
             function mk() { return { tag: 7 }; }\n\
             Reflect.defineProperty(o, \"x\", { value: 1, writable: true, enumerable: true, configurable: true });"
        ),
        Outcome::Normal("true".to_string())
    );
    let Value::Object(make) = global_value(&mut agent, &realm, "mk") else {
        panic!("mk is no object");
    };
    MAKE.with(|cell| cell.set(Some(make)));
    set_hook(&mut agent, &realm, "hook");
    agent.host_mut().count = 1;
    let outcome = eval_text(
        &mut agent,
        &realm,
        "var ok = Reflect.defineProperty(o, \"x\", { enumerable: false }); for (var j = 0; j < 20; j++) ({}); ok;",
    );
    HOOK.with(|cell| cell.set(None));
    MAKE.with(|cell| cell.set(None));
    assert_eq!(outcome, Outcome::Normal("true".to_string()));
    assert!(matches!(agent.collect(), Ok(HostOutcome::Normal(_))));
    assert_eq!(agent.state(), AgentState::Alive);
}
