//! The runner's host (design, decision 11; test262 `INTERPRETING.md`, "Host-Defined
//! Functions"): the [`Host`] of every agent the runner makes, and the two host-defined globals
//! a case may call, `print` and `$262` with `createRealm`, `evalScript` and `global`. The other
//! members of `$262` come with their slices: `detachArrayBuffer` (13), `gc` (16), `IsHTMLDDA`
//! (18), `AbstractModuleSource` (11); `agent` is excluded (decision 1).
//!
//! `$262.evalScript(sourceText)` follows the steps `INTERPRETING.md` gives: ParseScript
//! (`#sec-parse-script`) of the string in the current realm; if that answers a list of errors,
//! a throw completion of the first; otherwise ScriptEvaluation
//! (`#sec-runtime-semantics-scriptevaluation`) of the script. The current realm while a host
//! function runs is its [[Realm]], so each realm has its own `evalScript`, and the host keeps
//! every realm of the run, the first one included, beside the `evalScript` made in it: a call
//! finds its realm by its callee. A refusal of the parser or its nesting limit inside the string
//! is a `SyntaxError` like an error of the grammar and is thrown as one, but it is not an error
//! a case can ask for (`run.rs`): the first such error marks the run, as `createRealm` marks a
//! run that wants another realm, and `run.rs` fails a marked run.
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
use std::rc::Rc;

use swell::{
    Abrupt, Agent, Brand, Completion, Cx, HookCx, Host, HostCall, HostError, HostObjectSpec,
    InterruptHandle, Job, LoadRequest, ObjectRef, Outcome, Parsed, PropertyDescriptor, PropertyKey,
    Prototype, Realm, RealmRef, Root, Source, Termination, Value, ValueKind,
};
use swell_syntax::{MessageKind, message_kind};

use crate::run::without_position;

/// The brand of the `$262` objects.
const BRAND_262: Brand = Brand(262);

/// What the runner answers when the global object or `Object.prototype` of a new realm is not an
/// object, which the engine guarantees it is.
const NOT_AN_OBJECT: HostError = HostError::WrongKind {
    expected: ValueKind::Object,
};

/// A refusal of the parser or its nesting limit that `$262.evalScript` met: the kind and the
/// `message` of the `SyntaxError` it threw.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct EvalScriptMark {
    pub(crate) kind: MessageKind,
    pub(crate) message: String,
}

/// The host of one run.
#[derive(Default)]
pub(crate) struct RunnerHost {
    /// The `$262` objects of the realms made for `createRealm` that no call has taken yet, in
    /// the order they were made.
    spare: VecDeque<Root>,
    /// The `$262` objects `createRealm` handed out.
    kept: Vec<Root>,
    /// Every realm of the run in the order it was made, the first one included, and beside each
    /// the `evalScript` function made in it, at the same index.
    realms: Vec<Rc<Realm>>,
    eval_scripts: Vec<Root>,
    /// `createRealm` was called when no realm was left.
    pub(crate) wanted_realm: bool,
    /// The first refusal or nesting limit `evalScript` met.
    pub(crate) eval_script_mark: Option<EvalScriptMark>,
    /// The agent's interrupt handle.
    pub(crate) interrupt: Option<InterruptHandle>,
    /// What `print` received, each as its string value.
    pub(crate) printed: Vec<String>,
    /// The generic and timeout jobs and the module loads the engine handed over: kept, never
    /// performed. The jobs come with `Atomics.waitAsync` (slice 14), the loads with slice 11.
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

/// A new realm with `print` and `$262` on its global object, kept by the host with its
/// `evalScript`, and the root of its `$262`.
pub(crate) fn new_realm(agent: &mut Agent<RunnerHost>) -> Result<(Rc<Realm>, Root), SetupError> {
    let realm = match agent.create_realm(None, None)? {
        Outcome::Normal(realm) => Rc::new(realm),
        Outcome::Throw(_) => return Err(SetupError::Other("create_realm threw".to_string())),
        Outcome::Terminated(termination) => return Err(SetupError::Terminated(termination)),
    };
    let global = agent.global_object(&realm)?;
    let installed = agent.with_cx(&realm, |cx| install(cx, &realm, &global))?;
    match installed {
        Ok(Ok((dollar, eval_script))) => {
            let host = agent.host_mut();
            host.realms.push(Rc::clone(&realm));
            host.eval_scripts.push(eval_script);
            Ok((realm, dollar))
        }
        Ok(Err(Abrupt::Terminated(termination))) => Err(SetupError::Terminated(termination)),
        Ok(Err(Abrupt::Throw(_))) => Err(SetupError::Other(
            "defining print and $262 threw".to_string(),
        )),
        Err(error) => Err(error.into()),
    }
}

/// Keeps the `$262` of a realm made for a later `createRealm`.
pub(crate) fn keep_spare(agent: &mut Agent<RunnerHost>, dollar: Root) {
    agent.host_mut().spare.push_back(dollar);
}

/// Defines `print` and `$262` on the global object `global` holds, in `realm`; answers the roots
/// of `$262` and of its `evalScript`.
fn install(
    cx: &mut Cx<'_, RunnerHost>,
    realm: &Realm,
    global: &Root,
) -> Result<Completion<(Root, Root)>, HostError> {
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
        error_kind: None,
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
    let eval_script =
        match cx.new_host_function(realm_ref, "evalScript", 1, eval_script_steps, false, None)? {
            Ok(function) => function,
            Err(abrupt) => return Ok(Err(abrupt)),
        };
    for (object, name, value) in [
        (dollar, "createRealm", Value::Object(create_realm)),
        (dollar, "evalScript", Value::Object(eval_script)),
        (dollar, "global", Value::Object(global)),
        (global, "print", Value::Object(print)),
        (global, "$262", Value::Object(dollar)),
    ] {
        if let Err(abrupt) = define(cx, object, name, value)? {
            return Ok(Err(abrupt));
        }
    }
    let dollar = cx.root(Value::Object(dollar))?;
    let eval_script = cx.root(Value::Object(eval_script))?;
    Ok(Ok((dollar, eval_script)))
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

/// `$262.evalScript(sourceText)` (`INTERPRETING.md`, "Host-Defined Functions"): ToString
/// (`#sec-tostring`) of the argument, ParseScript (`#sec-parse-script`) of its code units in the
/// realm of the function; a list of errors throws the first, and marks the run when it is a
/// refusal or the nesting limit; otherwise ScriptEvaluation
/// (`#sec-runtime-semantics-scriptevaluation`) of the script and its completion.
fn eval_script_steps(cx: &mut Cx<'_, RunnerHost>, call: HostCall<'_>) -> Completion<Value> {
    let source = call.args.first().copied().unwrap_or(Value::Undefined);
    let string = fault_on_host_error(cx.to_string(source))??;
    let units = fault_on_host_error(cx.to_utf16(string))?;
    let Some(realm) = fault_on_host_error(realm_of(cx, call.callee))? else {
        return Err(Abrupt::Terminated(Termination::EngineFault));
    };
    let parsed = fault_on_host_error(cx.parse_script(realm, Source::Utf16(&units), None))?;
    let script = match parsed {
        Parsed::Parsed(script) => script,
        Parsed::Errors(errors) => {
            let Some(first) = errors.first() else {
                return Err(Abrupt::Terminated(Termination::EngineFault));
            };
            let error = fault_on_host_error(cx.get(first))?;
            mark(cx, error)?;
            return Err(Abrupt::Throw(error));
        }
        Parsed::Terminated(termination) => return Err(Abrupt::Terminated(termination)),
    };
    fault_on_host_error(cx.evaluate_script(script))?
}

/// The realm of the `evalScript` function `callee`: the realm the host keeps at the index of
/// that function. The host data of a host function cannot be read back through `Cx`
/// (`host_data` reads host objects only), so the function is found among those the host keeps.
/// `get` takes the context mutably, so the roots are taken out of the host for the reads and
/// put back before the answer: the reads run no code and reach no hook.
fn realm_of(cx: &mut Cx<'_, RunnerHost>, callee: ObjectRef) -> Result<Option<RealmRef>, HostError> {
    let eval_scripts = std::mem::take(&mut cx.host_mut().eval_scripts);
    let found = index_of(cx, &eval_scripts, callee);
    cx.host_mut().eval_scripts = eval_scripts;
    let Some(realm) = found?.and_then(|index| cx.host().realms.get(index).map(Rc::clone)) else {
        return Ok(None);
    };
    cx.realm_ref(&realm).map(Some)
}

/// The index of the root among `eval_scripts` that holds `callee`.
fn index_of(
    cx: &mut Cx<'_, RunnerHost>,
    eval_scripts: &[Root],
    callee: ObjectRef,
) -> Result<Option<usize>, HostError> {
    for (index, eval_script) in eval_scripts.iter().enumerate() {
        if cx.get(eval_script)? == Value::Object(callee) {
            return Ok(Some(index));
        }
    }
    Ok(None)
}

/// Marks the run when `error`, the first error ParseScript gave, is a refusal or the nesting
/// limit, read from its `message` as `run.rs` reads one; the first mark stays.
fn mark(cx: &mut Cx<'_, RunnerHost>, error: Value) -> Completion<()> {
    let Value::Object(object) = error else {
        return Ok(());
    };
    let key = PropertyKey::String(cx.new_string("message")?);
    let Value::String(message) = fault_on_host_error(cx.get_property(object, key))?? else {
        return Ok(());
    };
    let message = fault_on_host_error(cx.to_rust_string_lossy(message))?;
    let kind = message_kind(without_position(&message));
    let host = cx.host_mut();
    if matches!(
        kind,
        MessageKind::Refusal { .. } | MessageKind::Nesting { .. }
    ) && host.eval_script_mark.is_none()
    {
        host.eval_script_mark = Some(EvalScriptMark { kind, message });
    }
    Ok(())
}

/// A host error on a handle the engine gave this host is a broken engine invariant: the run
/// ends in `Terminated(EngineFault)` (`docs/design/host-interface.md`, decision 6).
fn fault_on_host_error<T>(result: Result<T, HostError>) -> Completion<T> {
    result.map_err(|_| Abrupt::Terminated(Termination::EngineFault))
}
