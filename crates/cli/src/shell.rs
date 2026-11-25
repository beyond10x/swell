//! One run of the shell: its host, its two globals and the exit status of each outcome.

use std::io::Write;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use swell::{
    Abrupt, Agent, Brand, Completion, Cx, ErrorKind, HookCx, Host, HostCall, HostError,
    HostIntrinsic, HostObjectSpec, Job, Limits, LoadRequest, ObjectRef, Outcome, Parsed,
    PropertyDescriptor, PropertyKey, Prototype, Realm, Root, Seeds, Source, Termination, Value,
};
use swell_syntax::{MessageKind, message_kind};

pub const NORMAL: u8 = 0;
pub const THROW: u8 = 1;
pub const SYNTAX_ERROR: u8 = 2;
pub const STEP_BUDGET: u8 = 3;
pub const MEMORY_BUDGET: u8 = 4;
pub const DEPTH_LIMIT: u8 = 5;
pub const INTERRUPTED: u8 = 6;
/// An engine fault, or a call the engine refused with a `HostError` (`sysexits.h`,
/// `EX_SOFTWARE`).
pub const FAULT: u8 = 70;

/// The seeds of every run: fixed, so that a run is the same run each time (design decision 13).
const SEEDS: Seeds = Seeds {
    hash_key: [0x5e; 16],
    random_seed: [0x11; 32],
};

/// The brand of the `console` object.
const BRAND_CONSOLE: Brand = Brand(1);

/// The messages of the two depth limits that end in a `RangeError` (`interp/budget.rs`,
/// `interp/mod.rs`).
const DEPTH_MESSAGES: [&str; 2] = ["frame_depth exceeded", "reentry_depth exceeded"];

/// The host of a run. The generic, timeout and cleanup jobs and the module loads the engine hands
/// over are kept and never performed: the shell has no event loop and no module loader.
#[derive(Default)]
pub struct ShellHost {
    jobs: Vec<Job>,
    loads: Vec<LoadRequest>,
}

impl Host for ShellHost {
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

    /// The system clock, in nanoseconds since the epoch; before it, negative.
    fn host_system_utc_epoch_nanoseconds(_cx: &mut HookCx<'_, Self>, _global: ObjectRef) -> i128 {
        match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(after) => i128::try_from(after.as_nanos()).unwrap_or(i128::MAX),
            Err(before) => i128::try_from(before.duration().as_nanos()).map_or(i128::MIN, |n| -n),
        }
    }
}

/// Runs `text` as a script with `limits`, then the promise jobs it queued; interrupts the run
/// after `timeout`. Answers the exit status and writes what ended the run to standard error.
pub fn run(text: &str, limits: Limits, timeout: Option<Duration>) -> u8 {
    let mut agent = match Agent::new(ShellHost::default(), limits, SEEDS) {
        Ok(agent) => agent,
        Err(error) => {
            eprintln!("swell: no agent: {error:?}");
            return FAULT;
        }
    };
    if let Some(timeout) = timeout {
        let interrupt = agent.interrupt_handle();
        thread::spawn(move || {
            thread::sleep(timeout);
            interrupt.interrupt();
        });
    }
    let code = match evaluate(&mut agent, text, &limits) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("swell: the engine refused a call of the shell: {error:?}");
            FAULT
        }
    };
    let _ = std::io::stdout().flush();
    code
}

fn evaluate(agent: &mut Agent<ShellHost>, text: &str, limits: &Limits) -> Result<u8, HostError> {
    let realm = match agent.create_realm(None, None)? {
        Outcome::Normal(realm) => realm,
        Outcome::Throw(_) => {
            eprintln!("swell: creating the realm threw");
            return Ok(FAULT);
        }
        Outcome::Terminated(termination) => return Ok(terminated(termination, limits)),
    };
    let global = agent.global_object(&realm)?;
    match agent.with_cx(&realm, |cx| install(cx, &realm, &global))?? {
        Ok(()) => {}
        Err(Abrupt::Terminated(termination)) => return Ok(terminated(termination, limits)),
        Err(Abrupt::Throw(_)) => {
            eprintln!("swell: defining print and console threw");
            return Ok(FAULT);
        }
    }
    let script = match agent.parse_script(&realm, Source::Utf8(text), None)? {
        Parsed::Parsed(script) => script,
        Parsed::Errors(errors) => return parse_errors(agent, &errors),
        Parsed::Terminated(termination) => return Ok(terminated(termination, limits)),
    };
    match agent.evaluate_script(&script)? {
        Outcome::Normal(_) => {}
        Outcome::Throw(thrown) => return uncaught(agent, &realm, &thrown),
        Outcome::Terminated(termination) => return Ok(terminated(termination, limits)),
    }
    Ok(match agent.perform_microtask_checkpoint()? {
        Outcome::Normal(_) => NORMAL,
        Outcome::Throw(_) => FAULT,
        Outcome::Terminated(termination) => terminated(termination, limits),
    })
}

/// Writes the parse errors; a parse ended by `parser_nesting` is a depth limit, any other a
/// syntax error.
fn parse_errors(agent: &Agent<ShellHost>, errors: &[Root]) -> Result<u8, HostError> {
    let mut code = SYNTAX_ERROR;
    for error in errors {
        let message = match agent.error_summary(error)? {
            Some(summary) => summary.message.map(|m| String::from_utf16_lossy(&m)),
            None => None,
        }
        .unwrap_or_default();
        if matches!(
            message_kind(without_position(&message)),
            MessageKind::Nesting { .. }
        ) {
            code = DEPTH_LIMIT;
        }
        eprintln!("swell: SyntaxError: {message}");
    }
    Ok(code)
}

/// `text` without the ` (<line>:<column>)` the engine writes after the message of a parse error
/// (`crates/swell/src/script.rs`).
fn without_position(text: &str) -> &str {
    let Some((message, position)) = text.rsplit_once(" (") else {
        return text;
    };
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    let is_position = position
        .strip_suffix(')')
        .and_then(|p| p.split_once(':'))
        .is_some_and(|(line, column)| digits(line) && digits(column));
    if is_position { message } else { text }
}

/// Writes the uncaught throw `thrown`, with where it arose; a `RangeError` of `frame_depth` or
/// `reentry_depth` is a depth limit, any other throw a throw.
fn uncaught(agent: &mut Agent<ShellHost>, realm: &Realm, thrown: &Root) -> Result<u8, HostError> {
    let site = agent
        .last_entry_error_site()?
        .map(|site| format!(" at {}:{}", site.line, site.column))
        .unwrap_or_default();
    if let Some(summary) = agent.error_summary(thrown)? {
        let message = summary
            .message
            .map(|m| String::from_utf16_lossy(&m))
            .unwrap_or_default();
        eprintln!("swell: uncaught {}: {message}{site}", summary.kind.name());
        let depth =
            summary.kind == ErrorKind::RangeError && DEPTH_MESSAGES.contains(&message.as_str());
        return Ok(if depth { DEPTH_LIMIT } else { THROW });
    }
    let text = agent.with_cx(realm, |cx| -> Option<String> {
        let value = cx.get(thrown).ok()?;
        let string = cx.to_string(value).ok()?.ok()?;
        cx.to_rust_string_lossy(string).ok()
    })?;
    match text {
        Some(text) => eprintln!("swell: uncaught {text}{site}"),
        None => eprintln!("swell: uncaught value that has no string{site}"),
    }
    Ok(THROW)
}

/// Writes what ended the run and answers its status.
fn terminated(termination: Termination, limits: &Limits) -> u8 {
    match termination {
        Termination::StepBudget => {
            eprintln!(
                "swell: step budget of {} steps spent",
                limits.steps_per_entry
            );
            STEP_BUDGET
        }
        Termination::MemoryBudget => {
            eprintln!(
                "swell: memory budget of {} bytes reached",
                limits.memory_bytes
            );
            MEMORY_BUDGET
        }
        Termination::Interrupted => {
            eprintln!("swell: interrupted");
            INTERRUPTED
        }
        Termination::EngineFault => {
            eprintln!("swell: engine fault");
            FAULT
        }
    }
}

/// Defines `print` and `console` with `console.log`, the same function, on the global object
/// `global` holds.
fn install(
    cx: &mut Cx<'_, ShellHost>,
    realm: &Realm,
    global: &Root,
) -> Result<Completion<()>, HostError> {
    let realm_ref = cx.realm_ref(realm)?;
    let Value::Object(global) = cx.get(global)? else {
        return Err(HostError::Faulted);
    };
    let object_prototype = cx.intrinsic(realm_ref, HostIntrinsic::ObjectPrototype)?;
    let console = match cx.new_host_object(HostObjectSpec {
        prototype: Prototype::Object(object_prototype),
        brand: BRAND_CONSOLE,
        data: None,
        error_kind: None,
    })? {
        Ok(console) => console,
        Err(abrupt) => return Ok(Err(abrupt)),
    };
    let print = match cx.new_host_function(realm_ref, "print", 0, print_steps, false, None)? {
        Ok(print) => print,
        Err(abrupt) => return Ok(Err(abrupt)),
    };
    for (object, name, value) in [
        (console, "log", Value::Object(print)),
        (global, "print", Value::Object(print)),
        (global, "console", Value::Object(console)),
    ] {
        if let Err(abrupt) = define(cx, object, name, value)? {
            return Ok(Err(abrupt));
        }
    }
    Ok(Ok(()))
}

/// Defines `name` on `object` as a writable, configurable, non-enumerable data property, as the
/// global functions of clause 19 are.
fn define(
    cx: &mut Cx<'_, ShellHost>,
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

/// `print(...args)`: ToString (`#sec-tostring`) of each argument, joined by a space, a newline
/// after them, written to standard output. A write that fails (a closed pipe) is not an error of
/// the script.
fn print_steps(cx: &mut Cx<'_, ShellHost>, call: HostCall<'_>) -> Completion<Value> {
    let mut line = String::new();
    for (index, value) in call.args.iter().enumerate() {
        if index > 0 {
            line.push(' ');
        }
        let string = fault_on_host_error(cx.to_string(*value))??;
        line.push_str(&fault_on_host_error(cx.to_rust_string_lossy(string))?);
    }
    line.push('\n');
    let _ = std::io::stdout().lock().write_all(line.as_bytes());
    Ok(Value::Undefined)
}

/// A host error on a handle the engine gave this host is a broken engine invariant: the run ends
/// in `Terminated(EngineFault)` (`docs/design/host-interface.md`, decision 6).
fn fault_on_host_error<T>(result: Result<T, HostError>) -> Completion<T> {
    result.map_err(|_| Abrupt::Terminated(Termination::EngineFault))
}
