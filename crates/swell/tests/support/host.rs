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
                overrides: None,
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
    no_stack_test_here();
    limits.min_stack_bytes() / 8 * 7
}

/// The variable the lane's run of the collect mode at opt-level 1 sets for its test processes
/// (task `slow-tests` of `Taskfile.yml`): that run holds no test that runs on a bounded stack,
/// since such a test proves the stack with frames whose size depends on the optimization
/// level and runs at opt-level 0 there.
pub const NO_STACK_TESTS: &str = "SWELL_NO_STACK_TESTS";

/// The thread a test runs its body on with a stack of `bytes`: the one way of the tests to a
/// bounded stack besides [`stack_with_room`], and like it the guard of [`NO_STACK_TESTS`].
pub fn stack_thread(bytes: usize) -> std::thread::Builder {
    no_stack_test_here();
    std::thread::Builder::new().stack_size(bytes)
}

/// The line a stack test fails with when it runs where [`NO_STACK_TESTS`] is set.
pub const NO_STACK_TESTS_LINE: &str = "this test runs on a bounded stack, but SWELL_NO_STACK_TESTS is set: it runs in the lane's run at opt-level 1 (task slow-tests), because taskfiles/realm-room-tests.awk -v rule=stack did not find it; it must run at opt-level 0: call stack_with_room or stack_thread in the test's own text, or in a function the rule reads it through";

/// Panics with [`NO_STACK_TESTS_LINE`] when [`NO_STACK_TESTS`] is set: a test the rule of
/// `slow-tests` did not find fails in that run, by itself, instead of passing on frames of
/// another size.
fn no_stack_test_here() {
    if std::env::var_os(NO_STACK_TESTS).is_some() {
        panic!("{NO_STACK_TESTS_LINE}");
    }
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
    match create_realm(agent, None, None).unwrap() {
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

/// The variable that pads every realm of a test by that many host functions.
pub const REALM_FUNCTIONS_VARIABLE: &str = "SWELL_TEST_REALM_FUNCTIONS";

/// The number of host functions [`create_realm`] pads a realm by: the value of
/// `SWELL_TEST_REALM_FUNCTIONS`, 0 when it is absent or empty. A value that is not an integer
/// stops the test with a panic naming the variable. The task `test-realm-sizes` sets it, so that
/// every test runs at several sizes of the realm (story `tests-hold-at-any-size-of-the-realm`).
pub fn realm_functions() -> usize {
    match std::env::var(REALM_FUNCTIONS_VARIABLE) {
        Err(std::env::VarError::NotPresent) => 0,
        Ok(value) if value.is_empty() => 0,
        Ok(value) => value
            .parse()
            .unwrap_or_else(|_| panic!("{REALM_FUNCTIONS_VARIABLE} is {value:?}, not an integer")),
        Err(error) => panic!("{REALM_FUNCTIONS_VARIABLE} cannot be read: {error}"),
    }
}

/// The steps of a padding function: `undefined`.
fn padding_steps<H: Host>(_cx: &mut Cx<'_, H>, _call: swell::HostCall<'_>) -> Completion<Value> {
    Ok(Value::Undefined)
}

/// Pad `realm` by `k` host functions, as a builtin grows a realm. At k = 0 it makes nothing,
/// no object and no Symbol, so a run without the variable is the run of the realm alone. At
/// k > 0 it makes an ordinary object (`new Object()` of the realm) and `k` host functions
/// `padding0` to `padding<k-1>` of the realm, of length 0 and steps that answer `undefined`,
/// each a non-enumerable property of that object under its name; then a Symbol, and binds the
/// object, non-enumerable, on the realm's global object under it. The padding is reached from
/// the realm only and lives and dies with it, as a builtin does, and no String key of the global
/// object moves.
///
/// It takes two entries so that, as with `create_realm`, the most the padding is ever charged is
/// what it is charged once made: a test whose budget is the padded realm's reading plus a few
/// bytes, or none, makes its realm under it. The first entry makes the object and the Symbol
/// (each a call, which holds a few bytes while it runs) and reads `Object`, `Symbol` and the
/// atom "name", which the realm has, by Strings it makes for them; a key is interned as an atom
/// of its own (`Heap::intern_string`), so these three Strings are garbage, and a collection
/// frees them before the second entry. The second calls nothing and makes no garbage: each
/// property's key is the atom of the function's own name, read from its "name" property. A
/// collection ends it, so that the padded realm leaves nothing for a collection to free. The
/// calls spend steps, so an agent whose `steps_per_entry` is 0 cannot be padded.
///
/// Each function grows the arena of objects (its record), that of Strings (its name and its
/// key's atom) and the atom table, and the holder's property list and index map; a builtin grows
/// the same, and its prototype or its data properties grow them further.
pub fn pad_realm<H: Host>(agent: &mut Agent<H>, realm: &Realm, k: usize) {
    if k == 0 {
        return;
    }
    let global_root = agent.global_object(realm).unwrap();
    let global_of = |cx: &mut Cx<'_, H>| match cx.get(&global_root).unwrap() {
        Value::Object(global) => global,
        other => panic!("the global object of the realm is {other:?}"),
    };
    let [holder_root, symbol_root, name_root] = agent
        .with_cx(realm, |cx| {
            let global = global_of(cx);
            let read = |cx: &mut Cx<'_, H>, name: &str| {
                let key = PropertyKey::String(cx.new_string(name).unwrap());
                cx.get_property(global, key).unwrap().unwrap()
            };
            let object_constructor = read(cx, "Object");
            let holder = cx
                .construct(object_constructor, &[], None)
                .unwrap()
                .unwrap();
            let symbol_constructor = read(cx, "Symbol");
            let description = Value::String(cx.new_string("realm padding").unwrap());
            let symbol = cx
                .call(symbol_constructor, Value::Undefined, &[description])
                .unwrap()
                .unwrap();
            let name = Value::String(cx.new_string("name").unwrap());
            let PropertyKey::String(atom) = cx.to_property_key(name).unwrap().unwrap() else {
                panic!("the key of \"name\" is no String");
            };
            [holder, symbol, Value::String(atom)].map(|value| cx.root(value).unwrap())
        })
        .unwrap();
    assert!(
        matches!(agent.collect(), Ok(Outcome::Normal(_))),
        "the collection before the padding"
    );
    agent
        .with_cx(realm, |cx| {
            let global = global_of(cx);
            let realm_ref = cx.realm_ref(realm).unwrap();
            let Value::String(name) = cx.get(&name_root).unwrap() else {
                panic!("the root of the atom \"name\" holds no String");
            };
            let Value::Object(holder) = cx.get(&holder_root).unwrap() else {
                panic!("new Object() made no object");
            };
            let Value::Symbol(symbol) = cx.get(&symbol_root).unwrap() else {
                panic!("Symbol() made no Symbol");
            };
            for index in 0..k {
                let function = cx
                    .new_host_function(
                        realm_ref,
                        &format!("padding{index}"),
                        0,
                        padding_steps::<H>,
                        false,
                        None,
                    )
                    .unwrap()
                    .unwrap();
                let own_name = cx
                    .get_property(function, PropertyKey::String(name))
                    .unwrap()
                    .unwrap();
                let key = cx.to_property_key(own_name).unwrap().unwrap();
                cx.define_property_or_throw(
                    holder,
                    key,
                    data(Value::Object(function), true, false, true),
                )
                .unwrap()
                .unwrap();
            }
            cx.define_property_or_throw(
                global,
                PropertyKey::Symbol(symbol),
                data(Value::Object(holder), true, false, true),
            )
            .unwrap()
            .unwrap();
        })
        .unwrap();
    drop((holder_root, symbol_root, name_root, global_root));
    assert!(
        matches!(agent.collect(), Ok(Outcome::Normal(_))),
        "the collection after the padding"
    );
}

/// What `Agent::create_realm` answers, with a `Normal` realm padded by [`realm_functions`]
/// host functions ([`pad_realm`]). Every realm of a test is made through it or through
/// [`realm`], so that the task `test-realm-sizes` runs every test at several sizes of the
/// realm. The calls of `Agent::create_realm` left in the tests, one line per call, with the
/// reason (`git grep -n '\.create_realm(' -- 'crates/swell/tests/*.rs'
/// ':!crates/swell/tests/support'` prints these calls and no other):
///
/// - `budgets.rs::host_9_zero_steps_ends_before_the_first`: `steps_per_entry` 0, which the
///   padding's entry needs steps of.
/// - `builtins_symbol_values.rs::in_a_bare_realm`: the parts of the walk over the functions of
///   the realm and the test that holds the file of their places equal to the walk make their
///   realm through it, because the file holds the places of the bare realm.
/// - `calls.rs::cx_methods_check_their_handles_before_the_sticky_termination`: `steps_per_entry`
///   0.
/// - `calls.rs::a_call_spends_the_step_budget_of_the_outermost_entry`: `steps_per_entry` 0 among
///   its budgets.
/// - `eval.rs::assert_faulted`: the answer of a faulted agent.
/// - `faults.rs::answers_faulted_everywhere`: the answer of a faulted agent.
/// - `faults.rs::host_71_injected_fault_at_each_call_faults_the_agent`: the injected fault of
///   `create_realm` itself.
/// - `fuzz.rs::op_create_realm`: the shape of the answer, with a setup, the transcript records.
/// - `fuzz.rs::op_create_realm`: the shape of the answer, without one.
/// - `host_objects.rs::host_183_the_host_names_three_intrinsics`: a setup that throws.
/// - `misuse.rs::host_30_a_realm_create_realm_did_not_make_is_stale`: a setup that throws or ends.
/// - `misuse.rs::host_48_a_faulted_agent_answers_faulted_everywhere`: a faulted agent, no setup.
/// - `misuse.rs::host_48_a_faulted_agent_answers_faulted_everywhere`: a faulted agent, a setup.
/// - `misuse.rs::host_160_a_handle_returned_by_the_host_faults_the_agent`: a setup's handle
///   faults the agent.
/// - `realm_padding.rs::padded`: pads the bare realm by the k it is given, not the variable's.
/// - `realms.rs::host_12_create_realm_with_zero_memory_is_memory_budget`: the budget refuses the
///   bare realm.
/// - `realms.rs::a_setup_returning_a_stale_or_foreign_global_faults_the_agent`: a setup's handle
///   faults the agent.
/// - `realms.rs::a_setup_returning_a_stale_or_foreign_global_faults_the_agent`: then the faulted
///   agent's answer.
/// - `realms.rs::a_setup_that_throws_makes_create_realm_throw`: a setup that throws.
pub fn create_realm<H: Host>(
    agent: &mut Agent<H>,
    setup: Option<swell::GlobalSetup<'_, H>>,
    host_defined: Option<swell::HostDefined>,
) -> Result<Outcome<Realm>, swell::HostError> {
    let created = agent.create_realm(setup, host_defined)?;
    if let Outcome::Normal(realm) = &created {
        pad_realm(agent, realm, realm_functions());
    }
    Ok(created)
}

/// The Number a completion value is, read with no allocation: `Ok` of the Number for a normal
/// completion that holds one, otherwise `Err` of the outcome as text. A test reads the answer of
/// a script so where the agent's room is the subject (after `Terminated(MemoryBudget)`):
/// `support::script::describe` writes a Number as a String the engine makes, and that String
/// can need a growth of the arena of Strings, whose size moves with every String the realm
/// holds, so the test's own reading would spend the room it gives the script.
pub fn number_of(
    agent: &mut Agent<TestHost>,
    realm: &Realm,
    outcome: Outcome<Root>,
) -> Result<f64, String> {
    match outcome {
        Outcome::Normal(root) => match agent.with_cx(realm, |cx| cx.get(&root)).unwrap() {
            Ok(Value::Number(number)) => Ok(number),
            other => Err(format!("Normal({other:?})")),
        },
        other => Err(format!("{other:?}")),
    }
}

/// The Number the script `source` answers in `realm`, parsed and evaluated, read by
/// [`number_of`]; a parse that does not give a script answers `Err` with what it gave.
pub fn number_answer(
    agent: &mut Agent<TestHost>,
    realm: &Realm,
    source: &str,
) -> Result<f64, String> {
    match agent
        .parse_script(realm, swell::Source::Utf8(source), None)
        .unwrap()
    {
        swell::Parsed::Parsed(script) => {
            let outcome = agent.evaluate_script(&script).unwrap();
            number_of(agent, realm, outcome)
        }
        swell::Parsed::Errors(errors) => Err(format!("{} parse error(s)", errors.len())),
        swell::Parsed::Terminated(termination) => {
            Err(format!("Parsed::Terminated({termination:?})"))
        }
    }
}
