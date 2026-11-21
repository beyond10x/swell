//! The runner's host (design, decision 11; test262 `INTERPRETING.md`, "Host-Defined
//! Functions"): the [`Host`] of every agent the runner makes, and the two host-defined globals
//! a case may call, `print` and `$262` with `createRealm` and `global`. The other members of
//! `$262` come with their slices: `evalScript` (2), `detachArrayBuffer` (13), `gc` (16),
//! `IsHTMLDDA` (18), `AbstractModuleSource` (11); `agent` is excluded (decision 1).
//!
//! `$262.createRealm` creates "a new ECMAScript Realm, defines this API on the new realm's global
//! object, and returns the `$262` property of the new realm's global object". A host function
//! reaches the agent through `Cx`, and `Cx` creates no realm (`docs/design/host-interface.md`,
//! decision 7: `create_realm` is an entry point of `Agent`). So the run creates its realms before
//! the case runs, and `createRealm` hands out the next one: each call gets a realm no code has
//! touched, made in call order. A call that finds none left marks the run and sets the agent's
//! interrupt (HOST-96), and the run is made again with more realms (`run.rs`).
//!
//! This is a workaround for this slice, recorded for the design: a run that calls `createRealm`
//! more often than realms were made is run again from the start, and every run pays for the
//! realms made for it whether it calls `createRealm` or not. What replaces it is a way for a host
//! function to create a realm in its own agent, a `create_realm` on `Cx` (or a hook the engine
//! calls back), which the host interface does not have; then `createRealm` makes its realm when
//! it is called, and the realms made before the run and the runs made again go.

use std::collections::VecDeque;

use swell::{
    Abrupt, Agent, Brand, Completion, Cx, HookCx, Host, HostCall, HostError, HostObjectSpec,
    InterruptHandle, Job, LoadRequest, ObjectRef, Outcome, PropertyDescriptor, PropertyKey,
    Prototype, Realm, Root, Termination, Value, ValueKind,
};

/// The brand of the `$262` objects.
const BRAND_262: Brand = Brand(262);

/// What the runner answers when the global object or `Object.prototype` of a new realm is not an
/// object, which the engine guarantees it is.
const NOT_AN_OBJECT: HostError = HostError::WrongKind {
    expected: ValueKind::Object,
};

/// The host of one run.
#[derive(Default)]
pub(crate) struct RunnerHost {
    /// The `$262` objects of the realms made for `createRealm` that no call has taken yet, in
    /// the order they were made.
    spare: VecDeque<Root>,
    /// The `$262` objects `createRealm` handed out, and the realms made for it.
    kept: Vec<Root>,
    realms: Vec<Realm>,
    /// `createRealm` was called when no realm was left.
    pub(crate) wanted_realm: bool,
    /// The agent's interrupt handle.
    pub(crate) interrupt: Option<InterruptHandle>,
    /// What `print` received, each as its string value.
    pub(crate) printed: Vec<String>,
    /// The jobs and module loads the engine handed over. No case before slice 10 needs one run;
    /// they are kept, never performed.
    jobs: Vec<Job>,
    loads: Vec<LoadRequest>,
}

impl Host for RunnerHost {
    fn host_enqueue_generic_job(cx: &mut HookCx<'_, Self>, job: Job) {
        cx.host_mut().jobs.push(job);
    }

    fn host_enqueue_timeout_job(cx: &mut HookCx<'_, Self>, job: Job, _milliseconds: f64) {
        cx.host_mut().jobs.push(job);
    }

    fn host_enqueue_finalization_registry_cleanup_job(cx: &mut HookCx<'_, Self>, job: Job) {
        cx.host_mut().jobs.push(job);
    }

    fn host_load_imported_module(cx: &mut HookCx<'_, Self>, request: LoadRequest) {
        cx.host_mut().loads.push(request);
    }

    /// A fixed instant, the epoch: the lines the runner prints do not depend on when it runs.
    fn host_system_utc_epoch_nanoseconds(_cx: &mut HookCx<'_, Self>, _global: ObjectRef) -> i128 {
        0
    }
}

/// Why a realm could not be made and given its globals.
pub(crate) enum SetupError {
    /// The engine ended the call.
    Terminated(Termination),
    /// Anything else, as text.
    Other(String),
}

impl From<HostError> for SetupError {
    fn from(error: HostError) -> Self {
        SetupError::Other(format!("host error {error:?}"))
    }
}

/// A new realm with `print` and `$262` on its global object, and the root of its `$262`.
pub(crate) fn new_realm(agent: &mut Agent<RunnerHost>) -> Result<(Realm, Root), SetupError> {
    let realm = match agent.create_realm(None, None)? {
        Outcome::Normal(realm) => realm,
        Outcome::Throw(_) => return Err(SetupError::Other("create_realm threw".to_string())),
        Outcome::Terminated(termination) => return Err(SetupError::Terminated(termination)),
    };
    let global = agent.global_object(&realm)?;
    let installed = agent.with_cx(&realm, |cx| install(cx, &realm, &global))?;
    match installed {
        Ok(Ok(dollar)) => Ok((realm, dollar)),
        Ok(Err(Abrupt::Terminated(termination))) => Err(SetupError::Terminated(termination)),
        Ok(Err(Abrupt::Throw(_))) => Err(SetupError::Other(
            "defining print and $262 threw".to_string(),
        )),
        Err(error) => Err(error.into()),
    }
}

/// Keeps `realm` and its `$262` for a later `createRealm`.
pub(crate) fn keep_spare(agent: &mut Agent<RunnerHost>, realm: Realm, dollar: Root) {
    let host = agent.host_mut();
    host.realms.push(realm);
    host.spare.push_back(dollar);
}

/// Defines `print` and `$262` on the global object `global` holds, in `realm`.
fn install(
    cx: &mut Cx<'_, RunnerHost>,
    realm: &Realm,
    global: &Root,
) -> Result<Completion<Root>, HostError> {
    let realm_ref = cx.realm_ref(realm)?;
    let Value::Object(global) = cx.get(global)? else {
        return Err(NOT_AN_OBJECT);
    };
    let object_prototype = match object_prototype(cx, global)? {
        Ok(prototype) => prototype,
        Err(abrupt) => return Ok(Err(abrupt)),
    };
    let dollar = match cx.new_host_object(HostObjectSpec {
        prototype: Prototype::Object(object_prototype),
        brand: BRAND_262,
        data: None,
    })? {
        Ok(dollar) => dollar,
        Err(abrupt) => return Ok(Err(abrupt)),
    };
    let print = match cx.new_host_function(realm_ref, "print", 1, print_steps, false, None)? {
        Ok(print) => print,
        Err(abrupt) => return Ok(Err(abrupt)),
    };
    let create_realm =
        match cx.new_host_function(realm_ref, "createRealm", 0, create_realm_steps, false, None)? {
            Ok(function) => function,
            Err(abrupt) => return Ok(Err(abrupt)),
        };
    for (object, name, value) in [
        (dollar, "createRealm", Value::Object(create_realm)),
        (dollar, "global", Value::Object(global)),
        (global, "print", Value::Object(print)),
        (global, "$262", Value::Object(dollar)),
    ] {
        if let Err(abrupt) = define(cx, object, name, value)? {
            return Ok(Err(abrupt));
        }
    }
    cx.root(Value::Object(dollar)).map(Ok)
}

/// `%Object.prototype%` of the realm whose global object is `global`, read as `Object.prototype`
/// before any code ran.
fn object_prototype(
    cx: &mut Cx<'_, RunnerHost>,
    global: ObjectRef,
) -> Result<Completion<ObjectRef>, HostError> {
    let mut object = global;
    for name in ["Object", "prototype"] {
        let key = match cx.new_string(name) {
            Ok(key) => PropertyKey::String(key),
            Err(abrupt) => return Ok(Err(abrupt)),
        };
        match cx.get_property(object, key)? {
            Ok(Value::Object(found)) => object = found,
            Ok(_) => return Err(NOT_AN_OBJECT),
            Err(abrupt) => return Ok(Err(abrupt)),
        }
    }
    Ok(Ok(object))
}

/// Defines `name` on `object` as a writable, configurable, non-enumerable data property, as
/// the global functions of clause 19 are.
fn define(
    cx: &mut Cx<'_, RunnerHost>,
    object: ObjectRef,
    name: &str,
    value: Value,
) -> Result<Completion<()>, HostError> {
    let key = match cx.new_string(name) {
        Ok(key) => PropertyKey::String(key),
        Err(abrupt) => return Ok(Err(abrupt)),
    };
    cx.define_property_or_throw(
        object,
        key,
        PropertyDescriptor {
            value: Some(value),
            writable: Some(true),
            enumerable: Some(false),
            configurable: Some(true),
            ..PropertyDescriptor::default()
        },
    )
}

/// `print(value)`: keeps ToString(value) (`#sec-tostring`) for the runner.
fn print_steps(cx: &mut Cx<'_, RunnerHost>, call: HostCall<'_>) -> Completion<Value> {
    let value = call.args.first().copied().unwrap_or(Value::Undefined);
    let string = fault_on_host_error(cx.to_string(value))??;
    let text = fault_on_host_error(cx.to_rust_string_lossy(string))?;
    cx.host_mut().printed.push(text);
    Ok(Value::Undefined)
}

/// `$262.createRealm()`: the `$262` of the next realm made for it. With none left, the run is
/// marked to be made again with more and is interrupted.
fn create_realm_steps(cx: &mut Cx<'_, RunnerHost>, _call: HostCall<'_>) -> Completion<Value> {
    let host = cx.host_mut();
    let Some(dollar) = host.spare.pop_front() else {
        host.wanted_realm = true;
        if let Some(interrupt) = &host.interrupt {
            interrupt.interrupt();
        }
        return Ok(Value::Undefined);
    };
    let value = fault_on_host_error(cx.get(&dollar))?;
    cx.host_mut().kept.push(dollar);
    Ok(value)
}

/// A host error on a handle the engine gave this host is a broken engine invariant: the run
/// ends in `Terminated(EngineFault)` (`docs/design/host-interface.md`, decision 6).
fn fault_on_host_error<T>(result: Result<T, HostError>) -> Completion<T> {
    result.map_err(|_| Abrupt::Terminated(Termination::EngineFault))
}
