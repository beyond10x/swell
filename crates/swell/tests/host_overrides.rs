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
    let made = agent
        .with_cx(realm, |cx| {
            let made = cx
                .new_host_object(spec(Prototype::Null, None, Some(&FIXED)))
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
                    Ok(value) => assert_eq!(value, Value::Number(1.0)),
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
