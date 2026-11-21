//! The test host and the limits and seeds the tests share.
//!
//! The helpers below stop the test with a panic when a call does not complete as the test
//! needs; `cfg(test)` (always set for an integration test) lets `clippy.toml` treat them as test
//! code.
#![cfg(test)]

use swell::{
    Abrupt, Agent, Completion, CreateError, Cx, HookCx, Host, Job, Limits, LoadRequest, ObjectRef,
    Outcome, PropertyDescriptor, PropertyKey, Realm, Root, Seeds, Value,
};

/// A host that records every call of a required hook, in order, as the hook's name and its
/// arguments, and keeps the jobs and load requests it is handed. The provided hooks keep their
/// defaults (`docs/design/host-interface.md`, decision 16), which `tests/hooks.rs` checks; a test
/// that reads `calls` shows which required hooks were called.
#[derive(Default)]
pub struct TestHost {
    pub calls: Vec<String>,
    pub jobs: Vec<Job>,
    pub loads: Vec<LoadRequest>,
    /// What `host_system_utc_epoch_nanoseconds` answers.
    pub now: i128,
    /// A counter host functions of the tests may use.
    pub count: u32,
}

impl Host for TestHost {
    fn host_enqueue_generic_job(cx: &mut HookCx<'_, Self>, job: Job) {
        let host = cx.host_mut();
        host.calls.push("host_enqueue_generic_job".to_string());
        host.jobs.push(job);
    }

    fn host_enqueue_timeout_job(cx: &mut HookCx<'_, Self>, job: Job, milliseconds: f64) {
        let host = cx.host_mut();
        host.calls
            .push(format!("host_enqueue_timeout_job {milliseconds}"));
        host.jobs.push(job);
    }

    fn host_enqueue_finalization_registry_cleanup_job(cx: &mut HookCx<'_, Self>, job: Job) {
        let host = cx.host_mut();
        host.calls
            .push("host_enqueue_finalization_registry_cleanup_job".to_string());
        host.jobs.push(job);
    }

    fn host_load_imported_module(cx: &mut HookCx<'_, Self>, request: LoadRequest) {
        let host = cx.host_mut();
        host.calls.push("host_load_imported_module".to_string());
        host.loads.push(request);
    }

    fn host_system_utc_epoch_nanoseconds(cx: &mut HookCx<'_, Self>, _global: ObjectRef) -> i128 {
        let host = cx.host_mut();
        host.calls
            .push("host_system_utc_epoch_nanoseconds".to_string());
        host.now
    }
}

/// The limits the tests use unless a test sets one of its own: large enough that no test of
/// this suite reaches one by accident.
pub fn limits() -> Limits {
    Limits {
        steps_per_entry: 1_000_000,
        memory_bytes: 64 * 1024 * 1024,
        frame_depth: 10_000,
        reentry_depth: 256,
        parser_nesting: 1_000,
        string_length: 1 << 30,
        bigint_bits: 1 << 20,
    }
}

/// The thread stack a test of `Limits::min_stack_bytes` runs its subject on: seven eighths of
/// what `limits` promise, so that the test also proves an eighth of room. The promise is the
/// measured need plus a fifth, rounded up to a whole KiB: through `toJSON`, the largest need,
/// 11,100 bytes per re-entry against 14,336 promised (77.4 %), measured on `247333c` with the
/// bodies of the instructions moved out of the run loop. A frame that grows by 1,560 bytes per
/// re-entry on that path (14 %) fails the first test (through builtins) while the promise still
/// holds, in the gate run here before the push, not first on the forge with the promise
/// broken. An eighth, not the whole fifth: the fifth is for another compiler's frames, and a
/// test that failed at the measured need would fail on the first byte of growth. This holds
/// because the build here is the forge's: the repository's `.cargo/config.toml` builds the dev
/// profile with full debug information whatever a user's own configuration says. A build with
/// line tables only had frames about 5 % smaller, measured at `4c921d3` (29,855 against 31,314
/// bytes per re-entry), and passed where the forge's build failed.
pub fn stack_with_room(limits: Limits) -> usize {
    limits.min_stack_bytes() / 8 * 7
}

/// The seeds the tests use: fixed, so every run is the same.
pub fn seeds() -> Seeds {
    Seeds {
        hash_key: [7; 16],
        random_seed: [11; 32],
    }
}

/// An agent with the test host, [`limits`] and [`seeds`].
pub fn agent() -> Result<Agent<TestHost>, CreateError> {
    Agent::new(TestHost::default(), limits(), seeds())
}

/// An agent with the test host, the given limits and [`seeds`].
pub fn agent_with(limits: Limits) -> Result<Agent<TestHost>, CreateError> {
    Agent::new(TestHost::default(), limits, seeds())
}

/// A realm with no setup closure and no host-defined value.
pub fn realm(agent: &mut Agent<TestHost>) -> Realm {
    match agent.create_realm(None, None).unwrap() {
        Outcome::Normal(realm) => realm,
        other => panic!("create_realm answered {other:?}"),
    }
}

/// A root of the global object of `realm`.
pub fn global(agent: &mut Agent<TestHost>, realm: &Realm) -> Root {
    agent.global_object(realm).unwrap()
}

/// The object a root holds.
pub fn object(cx: &Cx<'_, TestHost>, root: &Root) -> ObjectRef {
    match cx.get(root).unwrap() {
        Value::Object(object) => object,
        other => panic!("the root holds {other:?}"),
    }
}

/// The property key of `name`.
pub fn key(cx: &mut Cx<'_, TestHost>, name: &str) -> PropertyKey {
    PropertyKey::String(cx.new_string(name).unwrap())
}

/// A String value of `text`.
pub fn string(cx: &mut Cx<'_, TestHost>, text: &str) -> Value {
    Value::String(cx.new_string(text).unwrap())
}

/// `Get(object, name)`, which must complete normally.
pub fn get(cx: &mut Cx<'_, TestHost>, object: ObjectRef, name: &str) -> Value {
    let key = key(cx, name);
    cx.get_property(object, key).unwrap().unwrap()
}

/// `Get(object, name)` for a property that holds an object.
pub fn get_object(cx: &mut Cx<'_, TestHost>, object: ObjectRef, name: &str) -> ObjectRef {
    match get(cx, object, name) {
        Value::Object(found) => found,
        other => panic!("{name} is {other:?}"),
    }
}

/// The text of a String value, lossy.
pub fn text(cx: &Cx<'_, TestHost>, value: Value) -> String {
    match value {
        Value::String(string) => cx.to_rust_string_lossy(string).unwrap(),
        other => panic!("{other:?} is not a String"),
    }
}

/// A fully populated data property descriptor.
pub fn data(
    value: Value,
    writable: bool,
    enumerable: bool,
    configurable: bool,
) -> PropertyDescriptor {
    PropertyDescriptor {
        value: Some(value),
        writable: Some(writable),
        enumerable: Some(enumerable),
        configurable: Some(configurable),
        ..PropertyDescriptor::default()
    }
}

/// Whether `value` is an object whose `name`, read through its prototype chain, is `name`, and
/// whose `constructor` is the global property `name` of `global`: an error of that kind of
/// that realm.
pub fn is_error_of(cx: &mut Cx<'_, TestHost>, global: ObjectRef, value: Value, name: &str) -> bool {
    let Value::Object(error) = value else {
        return false;
    };
    let constructor = get(cx, global, name);
    let found_name = get(cx, error, "name");
    let found_constructor = get(cx, error, "constructor");
    matches!(found_name, Value::String(_))
        && text(cx, found_name) == name
        && found_constructor == constructor
}

/// The thrown value of a completion that must be a throw.
pub fn thrown<T: std::fmt::Debug>(completion: Completion<T>) -> Value {
    match completion {
        Err(Abrupt::Throw(value)) => value,
        other => panic!("expected a throw, got {other:?}"),
    }
}
