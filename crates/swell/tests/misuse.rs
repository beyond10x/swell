//! Misuse ends in a defined answer (`docs/design/host-interface.md`, decisions 4, 5, 6, 7, 13
//! and 18): for each entry point and read of slice 1, the answer to a stale handle, to a handle
//! of another agent and of a faulted agent, each checked with a host that records every hook
//! call (HOST-30, HOST-31, HOST-45, HOST-48, HOST-102, HOST-129, HOST-160).
//!
//! What a host can hold that is stale: a scoped handle (`ObjectRef`, `StringRef`, `RealmRef`,
//! `ScriptRef`) kept past the scope it was handed in, whose record a collection then freed, and
//! the scoped handle of a value whose `Root` was released. A `Root`, `Realm` or `Script` itself
//! cannot be held stale or released: none is `Clone` or `Copy`, it is released only by its own
//! drop, and while it is held it keeps its record alive. So the entry points of `Agent`, which
//! take only those, have the foreign answer and the faulted one; the stale answer is that of the
//! methods of `Cx` and `HookCx`. Run also with the feature `collect-every-safe-point`.
#![cfg(test)]

use crate::support;

use std::cell::Cell;
use std::rc::Rc;

use support::host::{limits, seeds};
use swell::{
    Abrupt, Agent, AgentState, Brand, CompileStrings, Completion, Cx, ErrorKind, GlobalObjects,
    Handled, HookCx, Host, HostCall, HostData, HostError, HostObjectSpec, IsoDateTime, Job,
    LoadRequest, ModuleRef, ObjectRef, Outcome, Parsed, PropertyDescriptor, PropertyKey, Prototype,
    Realm, RealmRef, RejectionOperation, Root, Script, ScriptRef, Source, StringRef, Termination,
    TimeZoneIdentifier, Tracer, Value,
};

/// A host that records every hook call, provided hooks included, by name, and answers each
/// with the draft's default (decision 16's table). `host_fn` counts the calls of the host
/// functions of these tests.
#[derive(Default)]
struct Recorder {
    calls: Vec<&'static str>,
    host_fn: u32,
    /// What the host function `answer` returns next.
    answer: Option<Completion<Value>>,
}

impl Host for Recorder {
    fn host_ensure_can_compile_strings(
        cx: &mut Cx<'_, Self>,
        _request: &CompileStrings<'_>,
    ) -> Completion<()> {
        cx.host_mut().calls.push("host_ensure_can_compile_strings");
        Ok(())
    }
    fn host_get_code_for_eval(cx: &mut HookCx<'_, Self>, _arg: ObjectRef) -> Option<StringRef> {
        cx.host_mut().calls.push("host_get_code_for_eval");
        None
    }
    fn host_ensure_can_add_private_element(
        cx: &mut HookCx<'_, Self>,
        _obj: ObjectRef,
    ) -> Completion<()> {
        cx.host_mut()
            .calls
            .push("host_ensure_can_add_private_element");
        Ok(())
    }
    fn host_has_source_text_available(cx: &mut HookCx<'_, Self>, _func: ObjectRef) -> bool {
        cx.host_mut().calls.push("host_has_source_text_available");
        true
    }
    fn host_make_job_callback(
        cx: &mut HookCx<'_, Self>,
        _callback: ObjectRef,
    ) -> Option<swell::HostDefined> {
        cx.host_mut().calls.push("host_make_job_callback");
        None
    }
    fn host_call_job_callback(
        cx: &mut Cx<'_, Self>,
        callback: ObjectRef,
        _host_defined: Option<&swell::HostDefined>,
        this: Value,
        args: &[Value],
    ) -> Completion<Value> {
        cx.host_mut().calls.push("host_call_job_callback");
        cx.call(Value::Object(callback), this, args)
            .unwrap_or(Err(Abrupt::Terminated(Termination::EngineFault)))
    }
    fn host_enqueue_promise_job(cx: &mut HookCx<'_, Self>, _realm: Option<RealmRef>) {
        cx.host_mut().calls.push("host_enqueue_promise_job");
    }
    fn host_prepare_to_run_job(cx: &mut HookCx<'_, Self>, _realm: RealmRef) {
        cx.host_mut().calls.push("host_prepare_to_run_job");
    }
    fn host_clean_up_after_job(
        cx: &mut HookCx<'_, Self>,
        _realm: Option<RealmRef>,
        _result: Result<(), Value>,
    ) {
        cx.host_mut().calls.push("host_clean_up_after_job");
    }
    fn host_enqueue_generic_job(cx: &mut HookCx<'_, Self>, _job: Job) {
        cx.host_mut().calls.push("host_enqueue_generic_job");
    }
    fn host_enqueue_timeout_job(cx: &mut HookCx<'_, Self>, _job: Job, _milliseconds: f64) {
        cx.host_mut().calls.push("host_enqueue_timeout_job");
    }
    fn host_enqueue_finalization_registry_cleanup_job(cx: &mut HookCx<'_, Self>, _job: Job) {
        cx.host_mut()
            .calls
            .push("host_enqueue_finalization_registry_cleanup_job");
    }
    fn host_promise_rejection_tracker(
        cx: &mut HookCx<'_, Self>,
        _promise: ObjectRef,
        _operation: RejectionOperation,
    ) {
        cx.host_mut().calls.push("host_promise_rejection_tracker");
    }
    fn host_load_imported_module(cx: &mut HookCx<'_, Self>, _request: LoadRequest) {
        cx.host_mut().calls.push("host_load_imported_module");
    }
    fn host_get_import_meta_properties(
        cx: &mut HookCx<'_, Self>,
        _module: ModuleRef,
    ) -> Completion<Vec<(PropertyKey, Value)>> {
        cx.host_mut().calls.push("host_get_import_meta_properties");
        Ok(Vec::new())
    }
    fn host_finalize_import_meta(
        cx: &mut HookCx<'_, Self>,
        _import_meta: ObjectRef,
        _module: ModuleRef,
    ) {
        cx.host_mut().calls.push("host_finalize_import_meta");
    }
    fn host_resize_array_buffer(
        cx: &mut HookCx<'_, Self>,
        _buffer: ObjectRef,
        _new_byte_length: u64,
    ) -> Completion<Handled> {
        cx.host_mut().calls.push("host_resize_array_buffer");
        Ok(Handled::Unhandled)
    }
    fn host_grow_shared_array_buffer(
        cx: &mut HookCx<'_, Self>,
        _buffer: ObjectRef,
        _new_byte_length: u64,
    ) -> Completion<Handled> {
        cx.host_mut().calls.push("host_grow_shared_array_buffer");
        Ok(Handled::Unhandled)
    }
    fn host_system_utc_epoch_nanoseconds(cx: &mut HookCx<'_, Self>, _global: ObjectRef) -> i128 {
        cx.host_mut()
            .calls
            .push("host_system_utc_epoch_nanoseconds");
        0
    }
    fn system_time_zone_identifier(cx: &mut HookCx<'_, Self>) -> String {
        cx.host_mut().calls.push("system_time_zone_identifier");
        "UTC".to_string()
    }
    fn get_named_time_zone_offset_nanoseconds(
        cx: &mut HookCx<'_, Self>,
        _id: &str,
        _epoch_ns: i128,
    ) -> i128 {
        cx.host_mut()
            .calls
            .push("get_named_time_zone_offset_nanoseconds");
        0
    }
    fn get_named_time_zone_epoch_nanoseconds(
        cx: &mut HookCx<'_, Self>,
        _id: &str,
        _local: IsoDateTime,
    ) -> Vec<i128> {
        cx.host_mut()
            .calls
            .push("get_named_time_zone_epoch_nanoseconds");
        Vec::new()
    }
    fn available_named_time_zone_identifiers(cx: &mut HookCx<'_, Self>) -> Vec<TimeZoneIdentifier> {
        cx.host_mut()
            .calls
            .push("available_named_time_zone_identifiers");
        Vec::new()
    }
}

/// A host function that counts its calls and answers `undefined`.
fn counted(cx: &mut Cx<'_, Recorder>, _call: HostCall<'_>) -> Completion<Value> {
    cx.host_mut().host_fn += 1;
    Ok(Value::Undefined)
}

/// A host function that counts its calls and returns what the host's `answer` holds.
fn answer(cx: &mut Cx<'_, Recorder>, _call: HostCall<'_>) -> Completion<Value> {
    cx.host_mut().host_fn += 1;
    cx.host_mut().answer.take().unwrap_or(Ok(Value::Undefined))
}

/// Host data that holds nothing.
struct Nothing;

impl HostData for Nothing {
    fn trace(&self, _tracer: &mut Tracer<'_>) {}
}

/// An agent with the recording host and the test limits.
fn agent() -> Agent<Recorder> {
    Agent::new(Recorder::default(), limits(), seeds()).unwrap()
}

/// A realm with no setup closure.
fn realm(agent: &mut Agent<Recorder>) -> Realm {
    match support::host::create_realm(agent, None, None).unwrap() {
        Outcome::Normal(realm) => realm,
        other => panic!("create_realm answered {other:?}"),
    }
}

/// `parse_script` of `source`, which must parse.
fn parse(agent: &mut Agent<Recorder>, realm: &Realm, source: &str) -> Script {
    match agent
        .parse_script(realm, Source::Utf8(source), None)
        .unwrap()
    {
        Parsed::Parsed(script) => script,
        other => panic!("{source:?} answered {other:?}"),
    }
}

/// Define the host function `steps` as the global `name` of `realm`.
fn define(agent: &mut Agent<Recorder>, realm: &Realm, name: &str, steps: swell::HostFn<Recorder>) {
    let global = agent.global_object(realm).unwrap();
    agent
        .with_cx(realm, |cx| {
            let realm = cx.realm_ref(realm).unwrap();
            let f = cx
                .new_host_function(realm, name, 0, steps, true, None)
                .unwrap()
                .unwrap();
            let Value::Object(global) = cx.get(&global).unwrap() else {
                panic!("the global is not an object");
            };
            let key = PropertyKey::String(cx.new_string(name).unwrap());
            cx.set(global, key, Value::Object(f), true)
                .unwrap()
                .unwrap();
        })
        .unwrap();
}

/// Handles of one kind each that the misuse calls are given: all stale, or all of another
/// agent.
#[derive(Clone, Copy, Debug)]
struct Bad {
    object: ObjectRef,
    string: StringRef,
    realm: RealmRef,
    script: ScriptRef,
}

/// Live handles of the agent under test, rooted so that they stay live: a plain object, a host
/// function, the key `k` and the realm.
struct Good {
    object: Root,
    function: Root,
    key: Root,
}

/// The live values of `good`, read in `cx`.
fn good_values(cx: &mut Cx<'_, Recorder>, good: &Good) -> (ObjectRef, Value, PropertyKey) {
    let Ok(Value::Object(object)) = cx.get(&good.object) else {
        panic!("the good object");
    };
    let function = cx.get(&good.function).unwrap();
    let Ok(Value::String(key)) = cx.get(&good.key) else {
        panic!("the good key");
    };
    (object, function, PropertyKey::String(key))
}

/// Make the live handles of `agent`.
fn good(agent: &mut Agent<Recorder>, realm: &Realm) -> Good {
    agent
        .with_cx(realm, |cx| {
            let realm_ref = cx.realm_ref(realm).unwrap();
            let object = cx.to_object(Value::Boolean(true)).unwrap().unwrap();
            let function = cx
                .new_host_function(realm_ref, "counted", 0, counted, true, None)
                .unwrap()
                .unwrap();
            let key = cx.new_string("k").unwrap();
            Good {
                object: cx.root(Value::Object(object)).unwrap(),
                function: cx.root(Value::Object(function)).unwrap(),
                key: cx.root(Value::String(key)).unwrap(),
            }
        })
        .unwrap()
}

/// Stale handles of `agent`: an object whose root was released, a String, a realm and a script
/// whose scopes ended, each freed by a collection, which each is checked to be.
fn stale(agent: &mut Agent<Recorder>, realm: &Realm) -> Bad {
    let doomed = match support::host::create_realm(agent, None, None).unwrap() {
        Outcome::Normal(doomed) => doomed,
        other => panic!("{other:?}"),
    };
    let realm_ref = agent
        .with_cx(&doomed, |cx| cx.realm_ref(&doomed).unwrap())
        .unwrap();
    drop(doomed);
    let (root, string, script) = agent
        .with_cx(realm, |cx| {
            let object = cx.to_object(Value::Number(1.0)).unwrap().unwrap();
            let root = cx.root(Value::Object(object)).unwrap();
            let string = cx.new_string("stale").unwrap();
            let own = cx.realm_ref(realm).unwrap();
            let Parsed::Parsed(script) = cx.parse_script(own, Source::Utf8("1;"), None).unwrap()
            else {
                panic!("the script did not parse");
            };
            (root, string, script)
        })
        .unwrap();
    let object = agent
        .with_cx(realm, |cx| match cx.get(&root) {
            Ok(Value::Object(object)) => object,
            other => panic!("{other:?}"),
        })
        .unwrap();
    drop(root);
    for _ in 0..2 {
        assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
    }
    let bad = Bad {
        object,
        string,
        realm: realm_ref,
        script,
    };
    // Each is stale now: the reads that take it say so.
    agent
        .with_cx(realm, |cx| {
            assert_eq!(cx.brand(bad.object), Err(HostError::StaleHandle));
            assert_eq!(cx.to_utf16(bad.string), Err(HostError::StaleHandle));
            assert!(matches!(
                cx.new_error(bad.realm, ErrorKind::Error, ""),
                Err(HostError::StaleHandle)
            ));
            assert_eq!(cx.evaluate_script(bad.script), Err(HostError::StaleHandle));
        })
        .unwrap();
    bad
}

/// Handles of `other`, an agent that is not the one under test.
fn foreign(other: &mut Agent<Recorder>) -> (Bad, Realm, Root, Script) {
    let realm = realm(other);
    let script = parse(other, &realm, "1;");
    let (bad, root) = other
        .with_cx(&realm, |cx| {
            let own = cx.realm_ref(&realm).unwrap();
            let object = cx.to_object(Value::Boolean(false)).unwrap().unwrap();
            let string = cx.new_string("foreign").unwrap();
            let Parsed::Parsed(script) = cx.parse_script(own, Source::Utf8("2;"), None).unwrap()
            else {
                panic!("the script did not parse");
            };
            let root = cx.root(Value::Object(object)).unwrap();
            (
                Bad {
                    object,
                    string,
                    realm: own,
                    script,
                },
                root,
            )
        })
        .unwrap();
    (bad, realm, root, script)
}

/// A data property descriptor of `value`.
fn data(value: Value) -> PropertyDescriptor {
    PropertyDescriptor {
        value: Some(value),
        writable: Some(true),
        enumerable: Some(true),
        configurable: Some(true),
        ..PropertyDescriptor::default()
    }
}

/// `Ok(())` for any `Ok`.
fn shape<T>(answer: Result<T, HostError>) -> Result<(), HostError> {
    answer.map(|_| ())
}

/// The methods `Cx` and `HookCx` share, each given one bad handle in one place, as (name,
/// answer) pairs.
macro_rules! scoped_answers {
    ($cx:expr, $bad:expr, $out:expr) => {{
        let bad: Bad = $bad;
        let o = Value::Object(bad.object);
        let s = Value::String(bad.string);
        $out.push(("root(object)", shape($cx.root(o))));
        $out.push(("root(string)", shape($cx.root(s))));
        $out.push(("to_utf16", shape($cx.to_utf16(bad.string))));
        $out.push((
            "to_rust_string_lossy",
            shape($cx.to_rust_string_lossy(bad.string)),
        ));
        $out.push((
            "new_host_object(prototype)",
            shape($cx.new_host_object(HostObjectSpec {
                prototype: Prototype::Object(bad.object),
                brand: Brand(3),
                data: Some(Box::new(Nothing)),
                error_kind: None,
            })),
        ));
        $out.push((
            "new_host_object(ObjectPrototypeOf)",
            shape($cx.new_host_object(HostObjectSpec {
                prototype: Prototype::ObjectPrototypeOf(bad.realm),
                brand: Brand(3),
                data: Some(Box::new(Nothing)),
                error_kind: None,
            })),
        ));
        $out.push(("brand", shape($cx.brand(bad.object))));
        $out.push((
            "host_data",
            shape($cx.host_data::<Nothing>(bad.object, Brand(3))),
        ));
        $out.push((
            "host_data_mut",
            shape($cx.host_data_mut::<Nothing>(bad.object, Brand(3))),
        ));
        $out.push((
            "new_error",
            shape($cx.new_error(bad.realm, ErrorKind::TypeError, "m")),
        ));
    }};
}

/// Every method of `Cx` that takes a handle, each given one bad handle in one place while the
/// others are `good`'s, as (name, answer) pairs.
fn cx_answers(
    cx: &mut Cx<'_, Recorder>,
    bad: Bad,
    good: &Good,
) -> Vec<(&'static str, Result<(), HostError>)> {
    let mut out = Vec::new();
    scoped_answers!(cx, bad, out);
    let (object, function, key) = good_values(cx, good);
    let o = Value::Object(bad.object);
    let s = Value::String(bad.string);
    let bad_key = PropertyKey::String(bad.string);
    out.push((
        "new_host_function(realm)",
        shape(cx.new_host_function(bad.realm, "f", 0, counted, false, None)),
    ));
    out.push(("call(function)", shape(cx.call(o, Value::Undefined, &[]))));
    out.push((
        "call(string function)",
        shape(cx.call(s, Value::Undefined, &[])),
    ));
    out.push(("call(this)", shape(cx.call(function, o, &[]))));
    out.push((
        "call(argument)",
        shape(cx.call(function, Value::Undefined, &[Value::Null, s])),
    ));
    out.push(("construct(function)", shape(cx.construct(o, &[], None))));
    out.push((
        "construct(argument)",
        shape(cx.construct(function, &[o], None)),
    ));
    out.push((
        "construct(new_target)",
        shape(cx.construct(function, &[], Some(o))),
    ));
    out.push(("get(object)", shape(cx.get_property(bad.object, key))));
    out.push(("get(key)", shape(cx.get_property(object, bad_key))));
    out.push((
        "set(object)",
        shape(cx.set(bad.object, key, Value::Null, true)),
    ));
    out.push((
        "set(key)",
        shape(cx.set(object, bad_key, Value::Null, true)),
    ));
    out.push(("set(value)", shape(cx.set(object, key, s, true))));
    out.push((
        "define_property_or_throw(object)",
        shape(cx.define_property_or_throw(bad.object, key, data(Value::Null))),
    ));
    out.push((
        "define_property_or_throw(key)",
        shape(cx.define_property_or_throw(object, bad_key, data(Value::Null))),
    ));
    out.push((
        "define_property_or_throw(value)",
        shape(cx.define_property_or_throw(object, key, data(o))),
    ));
    for (name, descriptor) in [
        (
            "define_property_or_throw(get)",
            PropertyDescriptor {
                get: Some(o),
                ..PropertyDescriptor::default()
            },
        ),
        (
            "define_property_or_throw(set)",
            PropertyDescriptor {
                set: Some(o),
                ..PropertyDescriptor::default()
            },
        ),
    ] {
        out.push((
            name,
            shape(cx.define_property_or_throw(object, key, descriptor)),
        ));
    }
    out.push((
        "has_property(object)",
        shape(cx.has_property(bad.object, key)),
    ));
    out.push(("has_property(key)", shape(cx.has_property(object, bad_key))));
    out.push((
        "delete_property_or_throw(object)",
        shape(cx.delete_property_or_throw(bad.object, key)),
    ));
    out.push((
        "delete_property_or_throw(key)",
        shape(cx.delete_property_or_throw(object, bad_key)),
    ));
    for (name, value) in [("object", o), ("string", s)] {
        out.push((name, shape(cx.to_primitive(value, None))));
        out.push((name, shape(cx.to_number(value))));
        out.push((name, shape(cx.to_numeric(value))));
        out.push((name, shape(cx.to_string(value))));
        out.push((name, shape(cx.to_object(value))));
        out.push((name, shape(cx.to_property_key(value))));
    }
    out.push((
        "parse_script(realm)",
        shape(cx.parse_script(bad.realm, Source::Utf8("var made = 1;"), None)),
    ));
    out.push(("evaluate_script", shape(cx.evaluate_script(bad.script))));
    out
}

/// Assert that every answer is `expected`.
fn all_answer(answers: &[(&'static str, Result<(), HostError>)], expected: HostError) {
    assert!(answers.len() > 40, "{} answers", answers.len());
    for (name, answer) in answers {
        assert_eq!(*answer, Err(expected), "{name}");
    }
}

/// The misuse calls of `Cx` and `HookCx` with `bad`, which must each answer `expected`, in an
/// agent whose live handles are `good`: once in a run that goes on, once in a run already ended
/// by the interrupt, which stays ended, and in a setup closure. Nothing changes: the heap holds
/// the same bytes, the last entry the same steps, the host function never ran, no property
/// was made, no hook was called and the agent stays `Alive`.
fn misuse_changes_nothing(
    agent: &mut Agent<Recorder>,
    realm: &Realm,
    good: &Good,
    bad: Bad,
    expected: HostError,
) {
    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
    let bytes = agent.heap_bytes().unwrap();
    let steps = agent.last_entry_steps().unwrap();
    let answers = agent
        .with_cx(realm, |cx| cx_answers(cx, bad, good))
        .unwrap();
    all_answer(&answers, expected);
    assert_eq!(agent.heap_bytes(), Ok(bytes));
    assert_eq!(agent.last_entry_steps(), Ok(steps));

    // In a run already terminated: the handles are checked first, and the run stays ended.
    let interrupt = agent.interrupt_handle();
    let (answers, after) = agent
        .with_cx(realm, |cx| {
            interrupt.interrupt();
            let ended = cx.to_number(Value::Number(1.0)).unwrap();
            assert_eq!(ended, Err(Abrupt::Terminated(Termination::Interrupted)));
            let answers = cx_answers(cx, bad, good);
            (answers, cx.to_number(Value::Number(2.0)).unwrap())
        })
        .unwrap();
    all_answer(&answers, expected);
    assert_eq!(after, Err(Abrupt::Terminated(Termination::Interrupted)));

    // The scoped methods of `HookCx`, in a setup closure.
    let mut answers = Vec::new();
    let mut setup =
        |cx: &mut HookCx<'_, Recorder>, _realm: RealmRef| -> Completion<GlobalObjects> {
            scoped_answers!(cx, bad, answers);
            let global = cx
                .new_host_object(HostObjectSpec {
                    prototype: Prototype::Null,
                    brand: Brand(1),
                    data: None,
                    error_kind: None,
                })
                .unwrap()?;
            Ok(GlobalObjects { global, this: None })
        };
    assert!(matches!(
        support::host::create_realm(agent, Some(&mut setup), None),
        Ok(Outcome::Normal(_))
    ));
    assert_eq!(answers.len(), 10);
    for (name, answer) in &answers {
        assert_eq!(*answer, Err(expected), "HookCx {name}");
    }

    let (object, made) = agent
        .with_cx(realm, |cx| {
            let (object, _, key) = good_values(cx, good);
            let made = cx.new_string("made").unwrap();
            (
                cx.has_property(object, key).unwrap().unwrap(),
                cx.has_property(object, PropertyKey::String(made))
                    .unwrap()
                    .unwrap(),
            )
        })
        .unwrap();
    assert!(!object, "no property `k` was made");
    assert!(!made, "no property `made` was made");
    let global = agent.global_object(realm).unwrap();
    let defined = agent
        .with_cx(realm, |cx| {
            let Ok(Value::Object(global)) = cx.get(&global) else {
                panic!("the global");
            };
            let made = cx.new_string("made").unwrap();
            cx.has_property(global, PropertyKey::String(made))
                .unwrap()
                .unwrap()
        })
        .unwrap();
    assert!(!defined, "no script ran");
    assert_eq!(agent.host().host_fn, 0, "no host function ran");
    assert!(agent.host().calls.is_empty(), "{:?}", agent.host().calls);
    assert_eq!(agent.state(), AgentState::Alive);
}

/// HOST-30: a handle whose record was freed, or the handle of a value whose root was released,
/// given to every method of `Cx` and `HookCx` that takes one, in every place it takes one,
/// answers `Err(StaleHandle)`, changes nothing and leaves the agent `Alive`; in a run already
/// terminated the same, and the run stays terminated.
#[test]
fn host_30_stale_handles_and_released_roots() {
    let mut agent = agent();

    let realm = realm(&mut agent);
    let good = good(&mut agent, &realm);
    let bad = stale(&mut agent, &realm);
    misuse_changes_nothing(&mut agent, &realm, &good, bad, HostError::StaleHandle);
}

/// HOST-30 for a realm `create_realm` did not make: the setup closure is handed the `RealmRef`
/// of the realm being made; when the closure throws or ends the run, `create_realm` returns no
/// `Realm`, and the kept `RealmRef` given later to `parse_script`, `new_host_function`,
/// `new_error` or as `Prototype::ObjectPrototypeOf` to `new_host_object` answers
/// `Err(StaleHandle)` and leaves the agent `Alive`, as for a realm that was
/// freed. Found by the fuzz test (`swell::fuzz`, seed 0x5eed0212): the script parsed in such a
/// realm faulted the agent when it ran.
#[test]
fn host_30_a_realm_create_realm_did_not_make_is_stale() {
    let endings: [fn() -> Completion<GlobalObjects>; 2] = [
        || Err(Abrupt::Throw(Value::Number(1.0))),
        || Err(Abrupt::Terminated(Termination::StepBudget)),
    ];
    for ending in endings {
        let mut agent = agent();
        let realm = realm(&mut agent);
        let mut kept: Option<RealmRef> = None;
        let mut setup = |_cx: &mut HookCx<'_, Recorder>, made: RealmRef| {
            kept = Some(made);
            ending()
        };
        let created = agent.create_realm(Some(&mut setup), None).unwrap();
        assert!(
            matches!(
                created,
                Outcome::Throw(_) | Outcome::Terminated(Termination::StepBudget)
            ),
            "create_realm answered {created:?}"
        );
        let unmade = kept.unwrap();
        let answers = agent
            .with_cx(&realm, |cx| {
                let parsed = shape(cx.parse_script(unmade, Source::Utf8("var x = 1; x;"), None));
                let function = shape(cx.new_host_function(unmade, "f", 0, counted, true, None));
                let error = shape(cx.new_error(unmade, ErrorKind::TypeError, "e"));
                let object = shape(cx.new_host_object(HostObjectSpec {
                    prototype: Prototype::ObjectPrototypeOf(unmade),
                    brand: Brand(1),
                    data: None,
                    error_kind: None,
                }));
                [parsed, function, error, object]
            })
            .unwrap();
        assert_eq!(
            answers,
            [Err(HostError::StaleHandle); 4],
            "after {created:?}"
        );
        assert_eq!(agent.state(), AgentState::Alive);
    }
}

/// HOST-31: a handle or root of another agent given to every entry point, read and method of
/// `Cx` and `HookCx` that takes one answers `Err(ForeignHandle)` and changes nothing; so does
/// a `Realm` of another agent given to `with_cx`, `Agent::call` and `Agent::construct`, whose
/// closure or function then does not run.
#[test]
fn host_31_handles_of_another_agent() {
    let mut other = agent();
    let (bad, other_realm, other_root, other_script) = foreign(&mut other);
    let mut agent = agent();

    let realm = realm(&mut agent);
    let good = good(&mut agent, &realm);
    misuse_changes_nothing(&mut agent, &realm, &good, bad, HostError::ForeignHandle);
    agent_entries_refuse_foreign(
        &mut agent,
        &realm,
        &good,
        &other_realm,
        &other_root,
        &other_script,
    );
    // The other agent is untouched.
    assert_eq!(other.state(), AgentState::Alive);
    assert!(other.host().calls.is_empty());
}

/// The entry points and reads of `Agent` given a `Realm`, `Root` or `Script` of another agent:
/// each answers `Err(ForeignHandle)`, runs no closure and no function, and changes nothing.
fn agent_entries_refuse_foreign(
    agent: &mut Agent<Recorder>,
    realm: &Realm,
    good: &Good,
    other_realm: &Realm,
    other_root: &Root,
    other_script: &Script,
) {
    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
    let bytes = agent.heap_bytes().unwrap();
    let steps = agent.last_entry_steps().unwrap();
    let ran = Rc::new(Cell::new(false));
    let flag = Rc::clone(&ran);
    let queued = Rc::clone(&ran);
    let foreign = Err(HostError::ForeignHandle);
    let undefined = agent
        .with_cx(realm, |cx| cx.root(Value::Undefined).unwrap())
        .unwrap();
    let answers: Vec<(&str, Result<(), HostError>)> = vec![
        (
            "with_cx",
            shape(agent.with_cx(other_realm, move |_| flag.set(true))),
        ),
        ("duplicate", shape(agent.duplicate(other_root))),
        ("global_object", shape(agent.global_object(other_realm))),
        (
            "realm_host_defined",
            shape(agent.realm_host_defined(other_realm)),
        ),
        (
            "call(realm)",
            shape(agent.call(other_realm, &good.function, &undefined, &[])),
        ),
        (
            "call(function)",
            shape(agent.call(realm, other_root, &undefined, &[])),
        ),
        (
            "call(this)",
            shape(agent.call(realm, &good.function, other_root, &[])),
        ),
        (
            "call(argument)",
            shape(agent.call(realm, &good.function, &undefined, &[&undefined, other_root])),
        ),
        (
            "construct(realm)",
            shape(agent.construct(other_realm, &good.function, &[], None)),
        ),
        (
            "construct(function)",
            shape(agent.construct(realm, other_root, &[], None)),
        ),
        (
            "construct(argument)",
            shape(agent.construct(realm, &good.function, &[other_root], None)),
        ),
        (
            "construct(new_target)",
            shape(agent.construct(realm, &good.function, &[], Some(other_root))),
        ),
        (
            "parse_script(realm)",
            shape(agent.parse_script(other_realm, Source::Utf8("var made = 1;"), None)),
        ),
        (
            "evaluate_script",
            shape(agent.evaluate_script(other_script)),
        ),
        (
            "script_host_defined",
            shape(agent.script_host_defined(other_script)),
        ),
        (
            "queue_microtask",
            shape(agent.queue_microtask(
                other_realm,
                Box::new(move |_cx: &mut Cx<'_, Recorder>| queued.set(true)),
            )),
        ),
        ("promise_state", shape(agent.promise_state(other_root))),
        (
            "promise_is_handled",
            shape(agent.promise_is_handled(other_root)),
        ),
        (
            "mark_promise_as_handled",
            shape(agent.mark_promise_as_handled(other_root)),
        ),
    ];
    for (name, answer) in &answers {
        assert_eq!(*answer, foreign, "Agent::{name}");
    }
    assert_eq!(agent.pending_microtasks(), Ok(0), "nothing was queued");
    let scoped = agent
        .with_cx(realm, |cx| {
            vec![
                ("get", shape(cx.get(other_root))),
                ("duplicate", shape(cx.duplicate(other_root))),
                ("realm_ref", shape(cx.realm_ref(other_realm))),
                ("promise_state", shape(cx.promise_state(other_root))),
                (
                    "promise_is_handled",
                    shape(cx.promise_is_handled(other_root)),
                ),
                (
                    "mark_promise_as_handled",
                    shape(cx.mark_promise_as_handled(other_root)),
                ),
            ]
        })
        .unwrap();
    for (name, answer) in &scoped {
        assert_eq!(*answer, foreign, "Cx::{name}");
    }
    drop(undefined);
    assert!(
        !ran.get(),
        "the closure of with_cx and the microtask did not run"
    );
    assert_eq!(agent.host().host_fn, 0, "no function ran");
    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
    assert_eq!(agent.heap_bytes(), Ok(bytes));
    assert_eq!(agent.last_entry_steps(), Ok(steps));
    assert!(agent.host().calls.is_empty(), "{:?}", agent.host().calls);
    assert_eq!(agent.state(), AgentState::Alive);
}

/// HOST-45: a `Realm` of agent A given to `parse_script` of agent B answers
/// `Err(ForeignHandle)`; so does a `Root` or `Script` of A given to any call of B. Both ways,
/// with the agents' own handles still answering in their own agent.
#[test]
fn host_45_realm_root_and_script_of_another_agent() {
    let mut a = agent();
    let mut b = agent();
    let realm_a = realm(&mut a);
    let realm_b = realm(&mut b);
    let script_a = parse(&mut a, &realm_a, "40 + 2;");
    let script_b = parse(&mut b, &realm_b, "40 + 2;");
    let good_a = good(&mut a, &realm_a);
    let good_b = good(&mut b, &realm_b);
    assert!(matches!(
        b.parse_script(&realm_a, Source::Utf8("1;"), None),
        Err(HostError::ForeignHandle)
    ));
    assert!(matches!(
        a.parse_script(&realm_b, Source::Utf8("1;"), None),
        Err(HostError::ForeignHandle)
    ));
    agent_entries_refuse_foreign(
        &mut b,
        &realm_b,
        &good_b,
        &realm_a,
        &good_a.object,
        &script_a,
    );
    agent_entries_refuse_foreign(&mut a, &realm_a, &good_a, &realm_b, &good_b.key, &script_b);
    for (agent, script) in [(&mut a, &script_a), (&mut b, &script_b)] {
        assert!(matches!(
            agent.evaluate_script(script),
            Ok(Outcome::Normal(_))
        ));
    }
}

/// Make `agent` fault: a script calls a host function that returns a stale handle (decision 6,
/// HOST-160).
fn fault(agent: &mut Agent<Recorder>, realm: &Realm) {
    let stale = stale(agent, realm);
    define(agent, realm, "answer", answer);
    agent.host_mut().answer = Some(Ok(Value::Object(stale.object)));
    let script = parse(agent, realm, "answer();");
    assert!(matches!(
        agent.evaluate_script(&script),
        Ok(Outcome::Terminated(Termination::EngineFault))
    ));
}

/// HOST-48: after an entry answered `Terminated(EngineFault)`, `state()` reads `Faulted` and
/// every other entry point and read answers `Err(HostError::Faulted)` without calling a hook:
/// no closure of the host runs, no host function, no setup closure.
#[test]
fn host_48_a_faulted_agent_answers_faulted_everywhere() {
    let mut agent = agent();
    let realm = realm(&mut agent);
    let good = good(&mut agent, &realm);
    let script = parse(&mut agent, &realm, "counted();");
    define(&mut agent, &realm, "counted", counted);
    // A microtask queued before the fault: the queue of a faulted agent never runs.
    let queued_before = Rc::new(Cell::new(false));
    let early = Rc::clone(&queued_before);
    agent
        .queue_microtask(
            &realm,
            Box::new(move |_cx: &mut Cx<'_, Recorder>| early.set(true)),
        )
        .unwrap();
    fault(&mut agent, &realm);
    assert_eq!(agent.state(), AgentState::Faulted);
    let calls_before = agent.host().host_fn;

    let ran = Rc::new(Cell::new(0));
    let (flag, setup_flag, queued) = (Rc::clone(&ran), Rc::clone(&ran), Rc::clone(&ran));
    let mut setup =
        move |_cx: &mut HookCx<'_, Recorder>, _realm: RealmRef| -> Completion<GlobalObjects> {
            setup_flag.set(setup_flag.get() + 1);
            Err(Abrupt::Throw(Value::Undefined))
        };
    let faulted = Err(HostError::Faulted);
    let answers: Vec<(&str, Result<(), HostError>)> = vec![
        (
            "with_cx",
            shape(agent.with_cx(&realm, move |_| flag.set(1))),
        ),
        ("duplicate", shape(agent.duplicate(&good.object))),
        ("create_realm", shape(agent.create_realm(None, None))),
        (
            "create_realm(setup)",
            shape(agent.create_realm(Some(&mut setup), None)),
        ),
        ("global_object", shape(agent.global_object(&realm))),
        (
            "realm_host_defined",
            shape(agent.realm_host_defined(&realm)),
        ),
        (
            "call",
            shape(agent.call(&realm, &good.function, &good.object, &[&good.key])),
        ),
        (
            "construct",
            shape(agent.construct(&realm, &good.function, &[&good.key], None)),
        ),
        (
            "parse_script",
            shape(agent.parse_script(&realm, Source::Utf8("1;"), None)),
        ),
        ("evaluate_script", shape(agent.evaluate_script(&script))),
        (
            "script_host_defined",
            shape(agent.script_host_defined(&script)),
        ),
        ("last_entry_steps", shape(agent.last_entry_steps())),
        ("heap_bytes", shape(agent.heap_bytes())),
        ("collect", shape(agent.collect())),
        (
            "queue_microtask",
            shape(agent.queue_microtask(
                &realm,
                Box::new(move |_cx: &mut Cx<'_, Recorder>| queued.set(1)),
            )),
        ),
        ("pending_microtasks", shape(agent.pending_microtasks())),
        ("clear_microtasks", shape(agent.clear_microtasks())),
        (
            "perform_microtask_checkpoint",
            shape(agent.perform_microtask_checkpoint()),
        ),
        ("promise_state", shape(agent.promise_state(&good.object))),
        (
            "promise_is_handled",
            shape(agent.promise_is_handled(&good.object)),
        ),
        (
            "mark_promise_as_handled",
            shape(agent.mark_promise_as_handled(&good.object)),
        ),
    ];
    for (name, answer) in &answers {
        assert_eq!(*answer, faulted, "{name}");
    }
    assert_eq!(ran.get(), 0, "no closure of the host ran");
    assert!(
        !queued_before.get(),
        "the microtask queued before the fault did not run"
    );
    assert_eq!(agent.host().host_fn, calls_before, "no host function ran");
    assert!(agent.host().calls.is_empty(), "{:?}", agent.host().calls);
    assert_eq!(agent.state(), AgentState::Faulted);
    // The interrupt handle of a faulted agent is a store nobody reads.
    agent.interrupt_handle().interrupt();
    assert_eq!(agent.state(), AgentState::Faulted);
}

/// What the reads of HOST-102 answer, as text. `name` roots the String the reads of a String
/// read, made by the test before its first reading of `heap_bytes()`: the reads make no record
/// on the heap, so that a String of the test's own cannot grow the arena of Strings between two
/// readings. Measured on 2026-11-26 with `Math.random` in the realm: at
/// `SWELL_TEST_REALM_FUNCTIONS` 200 the padded realm leaves that arena two free slots after a
/// collection, the third String grows it by 53,248 bytes, and a String made here moved
/// `heap_bytes()` from 550,495 to 603,743.
fn reads(
    agent: &mut Agent<Recorder>,
    realm: &Realm,
    script: &Script,
    root: &Root,
    name: &Root,
) -> String {
    let global = agent.global_object(realm).unwrap();
    let duplicate = agent.duplicate(root).unwrap();
    let scoped = agent
        .with_cx(realm, |cx| {
            let value = cx.get(&duplicate).unwrap();
            let Value::Object(object) = cx.get(root).unwrap() else {
                panic!("the root holds an object");
            };
            let Value::Object(global) = cx.get(&global).unwrap() else {
                panic!("the global");
            };
            let Value::String(name) = cx.get(name).unwrap() else {
                panic!("the root of \"name\" holds no String");
            };
            (
                value == cx.get(root).unwrap(),
                cx.brand(object),
                cx.brand(global),
                cx.host_data::<Nothing>(object, Brand(3))
                    .map(|d| d.is_some()),
                cx.host_data_mut::<Nothing>(object, Brand(3))
                    .map(|d| d.is_some()),
                cx.to_utf16(name),
                cx.to_rust_string_lossy(name),
                cx.realm_ref(realm).is_ok(),
                cx.promise_state(root).map(|_| ()),
                cx.promise_is_handled(root),
                cx.mark_promise_as_handled(root),
            )
        })
        .unwrap();
    format!(
        "{:?} {:?} {:?} {:?} {:?} {:?} {scoped:?} {:?} {:?} {:?}",
        agent.state(),
        agent.heap_bytes(),
        agent.last_entry_steps(),
        agent.realm_host_defined(realm).map(|d| d.is_some()),
        agent.script_host_defined(script).map(|d| d.is_some()),
        agent.pending_microtasks(),
        agent.promise_state(root).map(|_| ()),
        agent.promise_is_handled(root),
        agent.mark_promise_as_handled(root),
    )
}

/// HOST-102: a read changes no value `last_entry_steps()`, `heap_bytes()` and `live_bytes()`
/// report, and calls no hook: after a script spent steps and a collection, every read of
/// `Agent` and the reads of `Cx` answer the same each time, `heap_bytes()`, `live_bytes()` and
/// `last_entry_steps()` read what they read before them, and no hook was called.
#[test]
fn host_102_reads_change_nothing_and_call_no_hook() {
    let mut agent = agent();
    let realm = realm(&mut agent);
    let script = parse(&mut agent, &realm, "var o = { a: 1 }; o.a + 1;");
    assert!(matches!(
        agent.evaluate_script(&script),
        Ok(Outcome::Normal(_))
    ));
    let (root, name) = agent
        .with_cx(&realm, |cx| {
            let object = cx
                .new_host_object(HostObjectSpec {
                    prototype: Prototype::Null,
                    brand: Brand(3),
                    data: Some(Box::new(Nothing)),
                    error_kind: None,
                })
                .unwrap()
                .unwrap();
            let name = cx.new_string("name").unwrap();
            (
                cx.root(Value::Object(object)).unwrap(),
                cx.root(Value::String(name)).unwrap(),
            )
        })
        .unwrap();
    assert!(matches!(
        agent.evaluate_script(&script),
        Ok(Outcome::Normal(_))
    ));
    // A microtask queued, so that `pending_microtasks` reads 1 and must keep reading it.
    agent
        .queue_microtask(&realm, Box::new(|_cx: &mut Cx<'_, Recorder>| {}))
        .unwrap();
    let steps = agent.last_entry_steps().unwrap();
    assert!(steps > 0);
    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
    let bytes = agent.heap_bytes().unwrap();
    let live = agent.live_bytes().unwrap();

    let first = reads(&mut agent, &realm, &script, &root, &name);
    assert!(first.contains(" Ok(1) "), "{first}");
    for _ in 0..3 {
        assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
        assert_eq!(reads(&mut agent, &realm, &script, &root, &name), first);
    }
    assert_eq!(agent.last_entry_steps(), Ok(steps));
    assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
    assert_eq!(agent.heap_bytes(), Ok(bytes));
    assert_eq!(agent.live_bytes(), Ok(live));
    let _ = agent.interrupt_handle();
    let _ = agent.host();
    assert_eq!(agent.last_entry_steps(), Ok(steps));
    assert_eq!(agent.heap_bytes(), Ok(bytes));
    assert_eq!(agent.live_bytes(), Ok(live));
    assert!(agent.host().calls.is_empty(), "{:?}", agent.host().calls);
}

/// Host data that counts how often a collection traced it.
struct Traced(Rc<Cell<u32>>);

impl HostData for Traced {
    fn trace(&self, _tracer: &mut Tracer<'_>) {
        self.0.set(self.0.get() + 1);
    }
}

/// HOST-129: `collect` runs no ECMAScript code and calls no hook: with live and unreachable
/// objects whose getters, `valueOf`, `toString` and `toJSON` would count, a host function that
/// would count, and host data that the collection traces, `collect` leaves every counter and
/// `last_entry_steps()` as they were, and no hook was called.
#[test]
fn host_129_collect_runs_no_code_and_calls_no_hook() {
    let mut agent = agent();
    let realm = realm(&mut agent);
    define(&mut agent, &realm, "counted", counted);
    let source = "var ran = 0;\n\
        function make() {\n\
          return { get x() { ran++; return 1; }, valueOf: function () { ran++; return 1; },\n\
                   toString: function () { ran++; return ''; }, toJSON: function () { ran++; },\n\
                   f: counted };\n\
        }\n\
        var kept = make();\n\
        for (var i = 0; i < 50; i++) { make(); [make(), make()].map(function (o) { return o; }); }\n\
        ran;";
    let script = parse(&mut agent, &realm, source);
    let Ok(Outcome::Normal(ran)) = agent.evaluate_script(&script) else {
        panic!("the script did not complete");
    };
    let traced = Rc::new(Cell::new(0));
    let data = Rc::clone(&traced);
    let held = agent
        .with_cx(&realm, |cx| {
            assert_eq!(cx.get(&ran), Ok(Value::Number(0.0)));
            let object = cx
                .new_host_object(HostObjectSpec {
                    prototype: Prototype::Null,
                    brand: Brand(4),
                    data: Some(Box::new(Traced(data))),
                    error_kind: None,
                })
                .unwrap()
                .unwrap();
            cx.root(Value::Object(object)).unwrap()
        })
        .unwrap();
    let steps = agent.last_entry_steps().unwrap();
    let host_fn = agent.host().host_fn;
    for _ in 0..3 {
        let Ok(Outcome::Normal(report)) = agent.collect() else {
            panic!("collect did not answer Normal");
        };
        assert_eq!(agent.heap_bytes(), Ok(report.heap_bytes_after));
    }
    assert!(traced.get() >= 3, "each collection traced the host data");
    assert_eq!(agent.last_entry_steps(), Ok(steps), "no step was spent");
    assert_eq!(agent.host().host_fn, host_fn, "no host function ran");
    assert!(agent.host().calls.is_empty(), "{:?}", agent.host().calls);
    let global = agent.global_object(&realm).unwrap();
    let after = agent
        .with_cx(&realm, |cx| {
            let Value::Object(global) = cx.get(&global).unwrap() else {
                panic!("the global");
            };
            let key = PropertyKey::String(cx.new_string("ran").unwrap());
            cx.get_property(global, key).unwrap().unwrap()
        })
        .unwrap();
    assert_eq!(
        after,
        Value::Number(0.0),
        "no getter or method of script ran"
    );
    drop(held);
}

/// The entry points that answer `Err(Faulted)` after HOST-160's fault, with valid arguments.
fn refuses_after_fault(agent: &mut Agent<Recorder>, realm: &Realm) {
    assert_eq!(agent.state(), AgentState::Faulted);
    assert!(matches!(
        agent.with_cx(realm, |_| ()),
        Err(HostError::Faulted)
    ));
    assert!(matches!(
        agent.parse_script(realm, Source::Utf8("1;"), None),
        Err(HostError::Faulted)
    ));
    assert!(matches!(agent.collect(), Err(HostError::Faulted)));
    assert!(matches!(agent.heap_bytes(), Err(HostError::Faulted)));
    assert!(agent.host().calls.is_empty(), "{:?}", agent.host().calls);
}

/// HOST-160: a host function whose `Ok` value holds a stale handle, one whose `Abrupt::Throw`
/// value holds a handle of another agent, and a `GlobalSetup` closure whose `GlobalObjects`
/// hold a stale one: each ends its run in `Terminated(EngineFault)`, `state()` then reads
/// `Faulted`, another entry point answers `Err(HostError::Faulted)`, and nothing panics. The
/// host function is called from a script, from `Agent::call` and from `cx.call`; the other two
/// cases of each site (a foreign `Ok`, a stale throw) are run as well.
#[test]
fn host_160_a_handle_returned_by_the_host_faults_the_agent() {
    let mut other = agent();
    let (foreign, _other_realm, _root, _script) = foreign(&mut other);

    for case in 0..4 {
        for site in 0..3 {
            let mut agent = agent();
            let realm = realm(&mut agent);
            let stale = stale(&mut agent, &realm);
            define(&mut agent, &realm, "answer", answer);
            let (stale_value, foreign_value) =
                (Value::Object(stale.object), Value::String(foreign.string));
            agent.host_mut().answer = Some(match case {
                0 => Ok(stale_value),
                1 => Err(Abrupt::Throw(foreign_value)),
                2 => Ok(foreign_value),
                _ => Err(Abrupt::Throw(stale_value)),
            });
            let script = parse(
                &mut agent,
                &realm,
                "var after = 0; try { answer(); } finally { after = 1; }",
            );
            let ended = match site {
                0 => matches!(
                    agent.evaluate_script(&script),
                    Ok(Outcome::Terminated(Termination::EngineFault))
                ),
                1 => {
                    let global = agent.global_object(&realm).unwrap();
                    let function = agent
                        .with_cx(&realm, |cx| {
                            let Value::Object(global) = cx.get(&global).unwrap() else {
                                panic!("the global");
                            };
                            let key = PropertyKey::String(cx.new_string("answer").unwrap());
                            let f = cx.get_property(global, key).unwrap().unwrap();
                            cx.root(f).unwrap()
                        })
                        .unwrap();
                    matches!(
                        agent.call(&realm, &function, &global, &[]),
                        Ok(Outcome::Terminated(Termination::EngineFault))
                    )
                }
                _ => {
                    let global = agent.global_object(&realm).unwrap();
                    let (first, later, strings) = agent
                        .with_cx(&realm, |cx| {
                            let Value::Object(global) = cx.get(&global).unwrap() else {
                                panic!("the global");
                            };
                            let key = PropertyKey::String(cx.new_string("answer").unwrap());
                            let f = cx.get_property(global, key).unwrap().unwrap();
                            let first = cx.call(f, Value::Undefined, &[]);
                            // Later calls in the scope still on the stack (the text beside
                            // HOST-48): a method whose type carries `HostError` answers
                            // `Err(Faulted)` before it checks its handles.
                            let later = [
                                ("to_number", shape(cx.to_number(Value::Null))),
                                ("root", shape(cx.root(Value::Object(global)))),
                                ("brand", shape(cx.brand(global))),
                                ("realm_ref", shape(cx.realm_ref(&realm))),
                                ("get_property", shape(cx.get_property(global, key))),
                                ("call", shape(cx.call(f, Value::Undefined, &[]))),
                                (
                                    "new_host_object",
                                    shape(cx.new_host_object(HostObjectSpec {
                                        prototype: Prototype::Null,
                                        brand: Brand(1),
                                        data: None,
                                        error_kind: None,
                                    })),
                                ),
                            ];
                            // A method whose type is `Completion<T>` alone answers the sticky
                            // termination (decision 10).
                            let strings = [
                                cx.new_string("s").map(|_| ()),
                                cx.new_string_utf16(&[0x73]).map(|_| ()),
                            ];
                            (first, later, strings)
                        })
                        .unwrap();
                    for (name, answer) in later {
                        assert_eq!(answer, Err(HostError::Faulted), "{name}, case {case}");
                    }
                    assert_eq!(
                        strings,
                        [Err(Abrupt::Terminated(Termination::EngineFault)); 2],
                        "new_string and new_string_utf16, case {case}"
                    );
                    first == Ok(Err(Abrupt::Terminated(Termination::EngineFault)))
                }
            };
            assert!(ended, "case {case}, site {site}");
            assert_eq!(agent.host().host_fn, 1, "case {case}, site {site}");
            refuses_after_fault(&mut agent, &realm);
        }
    }

    // The setup closure: a stale global, a foreign `this`, a foreign global, a stale throw.
    for case in 0..4 {
        let mut agent = agent();
        let realm = realm(&mut agent);
        let stale = stale(&mut agent, &realm);
        let mut calls = 0;
        let mut setup =
            |cx: &mut HookCx<'_, Recorder>, _realm: RealmRef| -> Completion<GlobalObjects> {
                calls += 1;
                let good = cx
                    .new_host_object(HostObjectSpec {
                        prototype: Prototype::Null,
                        brand: Brand(2),
                        data: None,
                        error_kind: None,
                    })
                    .unwrap()?;
                match case {
                    0 => Ok(GlobalObjects {
                        global: stale.object,
                        this: None,
                    }),
                    1 => Ok(GlobalObjects {
                        global: good,
                        this: Some(foreign.object),
                    }),
                    2 => Ok(GlobalObjects {
                        global: foreign.object,
                        this: None,
                    }),
                    _ => Err(Abrupt::Throw(Value::Object(stale.object))),
                }
            };
        let outcome = agent.create_realm(Some(&mut setup), None).unwrap();
        assert!(
            matches!(outcome, Outcome::Terminated(Termination::EngineFault)),
            "setup case {case}: {outcome:?}"
        );
        assert_eq!(calls, 1);
        refuses_after_fault(&mut agent, &realm);
    }
}
