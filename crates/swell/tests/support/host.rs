//! The test host and the limits and seeds the tests share.
//!
//! The helpers below stop the test with a panic when a call does not complete as the test
//! needs; `cfg(test)` (always set for an integration test) lets `clippy.toml` treat them as test
//! code.
#![cfg(test)]

use swell::{
    Abrupt, Agent, AgentState, Brand, Completion, CreateError, Cx, GlobalObjects, HookCx, Host,
    HostObjectSpec, IsoDateTime, Job, Limits, LoadRequest, ObjectRef, Outcome, PropertyDescriptor,
    PropertyKey, Prototype, Realm, Root, Seeds, Value,
};

/// A host that records every call of a required hook, in order, as the hook's name and its
/// arguments, and keeps the jobs and load requests it is handed. The provided hooks keep their
/// defaults (`docs/design/host-interface.md`, decision 16), which `tests/hooks.rs` checks; a test
/// that reads `calls` shows which required hooks were called. The three time zone hooks it
/// answers from its zone fields record their calls too.
pub struct TestHost {
    pub calls: Vec<String>,
    pub jobs: Vec<Job>,
    pub loads: Vec<LoadRequest>,
    /// What `host_system_utc_epoch_nanoseconds` answers.
    pub now: i128,
    /// What the clock adds to `now` after each answer: 0, a clock that does not advance, by
    /// default.
    pub clock_step_ns: i128,
    /// The global object `host_system_utc_epoch_nanoseconds` was given, one per call.
    pub clock_globals: Vec<ObjectRef>,
    /// A counter host functions of the tests may use.
    pub count: u32,
    /// The host's map from a node number to the handle it keeps for that node, with no root: a
    /// weak reference (HOST-178).
    pub kept: Vec<Option<ObjectRef>>,
    /// The charges of the host's own bytes a test's host functions keep.
    pub charges: Vec<swell::HostBytes>,
    /// What `system_time_zone_identifier` answers: "UTC" by default.
    pub zone_identifier: String,
    /// What `get_named_time_zone_offset_nanoseconds` answers: 0 by default.
    pub zone_offset_ns: i128,
    /// What `get_named_time_zone_epoch_nanoseconds` answers for a local time; `None`, the
    /// default, answers as the trait's default does.
    pub zone_epochs: Option<fn(&IsoDateTime) -> Vec<i128>>,
}

impl Default for TestHost {
    fn default() -> TestHost {
        TestHost {
            calls: Vec::new(),
            jobs: Vec::new(),
            loads: Vec::new(),
            now: 0,
            clock_step_ns: 0,
            clock_globals: Vec::new(),
            count: 0,
            kept: Vec::new(),
            charges: Vec::new(),
            zone_identifier: "UTC".to_string(),
            zone_offset_ns: 0,
            zone_epochs: None,
        }
    }
}

/// The name `TestHost` records for a call of `system_time_zone_identifier`.
pub const ZONE_IDENTIFIER: &str = "system_time_zone_identifier";

/// The first word `TestHost` records for a call of `get_named_time_zone_offset_nanoseconds`,
/// followed by the identifier and the epoch nanoseconds it was given.
pub const ZONE_OFFSET: &str = "get_named_time_zone_offset_nanoseconds";

/// The first word `TestHost` records for a call of `get_named_time_zone_epoch_nanoseconds`,
/// followed by the identifier and the local time it was given.
pub const ZONE_EPOCHS: &str = "get_named_time_zone_epoch_nanoseconds";

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

    fn host_system_utc_epoch_nanoseconds(cx: &mut HookCx<'_, Self>, global: ObjectRef) -> i128 {
        let host = cx.host_mut();
        host.calls
            .push("host_system_utc_epoch_nanoseconds".to_string());
        host.clock_globals.push(global);
        let now = host.now;
        host.now = now.saturating_add(host.clock_step_ns);
        now
    }

    fn system_time_zone_identifier(cx: &mut HookCx<'_, Self>) -> String {
        let host = cx.host_mut();
        host.calls.push(ZONE_IDENTIFIER.to_string());
        host.zone_identifier.clone()
    }

    fn get_named_time_zone_offset_nanoseconds(
        cx: &mut HookCx<'_, Self>,
        id: &str,
        epoch_ns: i128,
    ) -> i128 {
        let host = cx.host_mut();
        host.calls.push(format!("{ZONE_OFFSET} {id} {epoch_ns}"));
        host.zone_offset_ns
    }

    fn get_named_time_zone_epoch_nanoseconds(
        cx: &mut HookCx<'_, Self>,
        id: &str,
        local: IsoDateTime,
    ) -> Vec<i128> {
        let host = cx.host_mut();
        host.calls.push(format!(
            "{ZONE_EPOCHS} {id} {}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}",
            local.year,
            local.month,
            local.day,
            local.hour,
            local.minute,
            local.second,
            local.millisecond
        ));
        if let Some(epochs) = host.zone_epochs {
            return epochs(&local);
        }
        let date = swell_num::date::IsoDateTime {
            year: i64::from(local.year),
            month: local.month,
            day: local.day,
            hour: local.hour,
            minute: local.minute,
            second: local.second,
            millisecond: local.millisecond,
            microsecond: local.microsecond,
            nanosecond: local.nanosecond,
        };
        swell_num::date::get_utc_epoch_nanoseconds(&date)
            .into_iter()
            .collect()
    }
}

/// A host that answers only the required hooks and records nothing: every provided hook, the
/// four time zone hooks among them, keeps the trait's own body (`tests/hooks.rs` checks those
/// defaults, `tests/date_zone.rs` runs `Date` over them).
pub struct ProvidedHost;

impl Host for ProvidedHost {
    fn host_enqueue_generic_job(_cx: &mut HookCx<'_, Self>, _job: Job) {}
    fn host_enqueue_timeout_job(_cx: &mut HookCx<'_, Self>, _job: Job, _milliseconds: f64) {}
    fn host_enqueue_finalization_registry_cleanup_job(_cx: &mut HookCx<'_, Self>, _job: Job) {}
    fn host_load_imported_module(_cx: &mut HookCx<'_, Self>, _request: LoadRequest) {}
    fn host_system_utc_epoch_nanoseconds(_cx: &mut HookCx<'_, Self>, _global: ObjectRef) -> i128 {
        0
    }
}

/// Runs `f` inside the setup closure of a `create_realm` of an agent of [`ProvidedHost`], the
/// place a host meets a `HookCx` outside a hook, and answers what `f` returned. The realm's
/// global is a host object.
pub fn in_provided_hook<R>(f: impl FnOnce(&mut HookCx<'_, ProvidedHost>) -> R) -> R {
    let mut agent = Agent::new(ProvidedHost, limits(), seeds()).unwrap();
    let mut f = Some(f);
    let mut out = None;
    let mut setup = |cx: &mut HookCx<'_, ProvidedHost>, _realm| -> Completion<GlobalObjects> {
        let global = cx
            .new_host_object(HostObjectSpec {
                prototype: Prototype::Null,
                brand: Brand(1),
                data: None,
                error_kind: None,
            })
            .unwrap()?;
        if let Some(f) = f.take() {
            out = Some(f(cx));
        }
        Ok(GlobalObjects { global, this: None })
    };
    let created = agent.create_realm(Some(&mut setup), None).unwrap();
    assert!(matches!(created, Outcome::Normal(_)), "{created:?}");
    assert_eq!(agent.state(), AgentState::Alive);
    out.unwrap()
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
/// test that failed at the measured need would fail on the first byte of growth. The build here
/// is the forge's because both read the repository's `.cargo/config.toml`, which builds the dev
/// profile without debug information (`debug = false`) whatever a user's own configuration
/// says; a build with line tables only had frames about 5 % smaller, measured at `4c921d3`
/// (29,855 against 31,314 bytes per re-entry), and passed where the forge's build failed. The
/// frames without debug information are smaller than with it, so the 1,560 bytes of growth
/// above are the margin of the build with full debug information, cargo's default for a dev
/// build: the profile `stack-debug` runs the stack tests in it (`taskfiles/stack-profiles.yml`).
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
pub fn object(cx: &mut Cx<'_, TestHost>, root: &Root) -> ObjectRef {
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
