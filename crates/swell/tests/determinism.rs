//! Determinism (`docs/design/host-interface.md`, decision 19, HOST-133): two agents created
//! with the same limits and seeds, given the same calls with the same arguments by hosts whose
//! hooks answer the same values, make the same hook calls with the same arguments and return
//! the same outcomes. A fixed corpus of slice 1 scripts is run by two agents in one process,
//! their calls interleaved, and by two child processes (this test binary started again with
//! `SWELL_DETERMINISM_CHILD` set); the transcripts of hook calls, host function calls, outcomes,
//! `last_entry_steps()`, `heap_bytes()` and collection reports are compared byte for byte.
//!
//! Epic acceptance item 13 (the same script run twice gives the same steps and value) is held
//! by `scripts::same_script_twice_gives_the_same_steps_and_value`; this extends it to hook
//! calls and to two processes. Run also with the feature `collect-every-safe-point`.
#![cfg(test)]

mod support;

use support::host::{limits, seeds};
use swell::{
    Abrupt, Agent, CompileStrings, Completion, Cx, Handled, HookCx, Host, HostCall, IsoDateTime,
    Job, Limits, LoadRequest, ModuleRef, ObjectRef, Outcome, Parsed, PropertyKey, Realm, RealmRef,
    RejectionOperation, Root, Source, StringRef, Termination, TimeZoneIdentifier, Value,
};

/// A host that writes every hook call, provided hooks included, and every call of its host
/// functions into one transcript, and answers each hook with the draft's default.
#[derive(Default)]
struct Transcript {
    lines: Vec<String>,
}

impl Transcript {
    fn hook(cx_host: &mut Transcript, line: String) {
        cx_host.lines.push(format!("hook {line}"));
    }
}

impl Host for Transcript {
    fn host_ensure_can_compile_strings(
        cx: &mut Cx<'_, Self>,
        request: &CompileStrings<'_>,
    ) -> Completion<()> {
        let kind = request.compilation_type;
        Self::hook(
            cx.host_mut(),
            format!("host_ensure_can_compile_strings {kind:?}"),
        );
        Ok(())
    }
    fn host_get_code_for_eval(cx: &mut HookCx<'_, Self>, _arg: ObjectRef) -> Option<StringRef> {
        Self::hook(cx.host_mut(), "host_get_code_for_eval".to_string());
        None
    }
    fn host_ensure_can_add_private_element(
        cx: &mut HookCx<'_, Self>,
        _obj: ObjectRef,
    ) -> Completion<()> {
        Self::hook(
            cx.host_mut(),
            "host_ensure_can_add_private_element".to_string(),
        );
        Ok(())
    }
    fn host_has_source_text_available(cx: &mut HookCx<'_, Self>, _func: ObjectRef) -> bool {
        Self::hook(cx.host_mut(), "host_has_source_text_available".to_string());
        true
    }
    fn host_make_job_callback(
        cx: &mut HookCx<'_, Self>,
        _callback: ObjectRef,
    ) -> Option<swell::HostDefined> {
        Self::hook(cx.host_mut(), "host_make_job_callback".to_string());
        None
    }
    fn host_call_job_callback(
        cx: &mut Cx<'_, Self>,
        callback: ObjectRef,
        _host_defined: Option<&swell::HostDefined>,
        this: Value,
        args: &[Value],
    ) -> Completion<Value> {
        Self::hook(
            cx.host_mut(),
            format!("host_call_job_callback {}", args.len()),
        );
        cx.call(Value::Object(callback), this, args)
            .unwrap_or(Err(Abrupt::Terminated(Termination::EngineFault)))
    }
    fn host_enqueue_promise_job(cx: &mut HookCx<'_, Self>, realm: Option<RealmRef>) {
        Self::hook(
            cx.host_mut(),
            format!("host_enqueue_promise_job {}", realm.is_some()),
        );
    }
    fn host_prepare_to_run_job(cx: &mut HookCx<'_, Self>, _realm: RealmRef) {
        Self::hook(cx.host_mut(), "host_prepare_to_run_job".to_string());
    }
    fn host_clean_up_after_job(
        cx: &mut HookCx<'_, Self>,
        _realm: Option<RealmRef>,
        result: Result<(), Value>,
    ) {
        Self::hook(
            cx.host_mut(),
            format!("host_clean_up_after_job {}", result.is_ok()),
        );
    }
    fn host_enqueue_generic_job(cx: &mut HookCx<'_, Self>, job: Job) {
        Self::hook(
            cx.host_mut(),
            format!("host_enqueue_generic_job {:?}", job.kind()),
        );
    }
    fn host_enqueue_timeout_job(cx: &mut HookCx<'_, Self>, job: Job, milliseconds: f64) {
        Self::hook(
            cx.host_mut(),
            format!("host_enqueue_timeout_job {:?} {milliseconds}", job.kind()),
        );
    }
    fn host_enqueue_finalization_registry_cleanup_job(cx: &mut HookCx<'_, Self>, job: Job) {
        Self::hook(
            cx.host_mut(),
            format!(
                "host_enqueue_finalization_registry_cleanup_job {:?}",
                job.kind()
            ),
        );
    }
    fn host_promise_rejection_tracker(
        cx: &mut HookCx<'_, Self>,
        _promise: ObjectRef,
        operation: RejectionOperation,
    ) {
        Self::hook(
            cx.host_mut(),
            format!("host_promise_rejection_tracker {operation:?}"),
        );
    }
    fn host_load_imported_module(cx: &mut HookCx<'_, Self>, request: LoadRequest) {
        Self::hook(
            cx.host_mut(),
            format!("host_load_imported_module {:?}", request.specifier()),
        );
    }
    fn host_get_import_meta_properties(
        cx: &mut HookCx<'_, Self>,
        _module: ModuleRef,
    ) -> Completion<Vec<(PropertyKey, Value)>> {
        Self::hook(cx.host_mut(), "host_get_import_meta_properties".to_string());
        Ok(Vec::new())
    }
    fn host_finalize_import_meta(
        cx: &mut HookCx<'_, Self>,
        _import_meta: ObjectRef,
        _module: ModuleRef,
    ) {
        Self::hook(cx.host_mut(), "host_finalize_import_meta".to_string());
    }
    fn host_resize_array_buffer(
        cx: &mut HookCx<'_, Self>,
        _buffer: ObjectRef,
        new_byte_length: u64,
    ) -> Completion<Handled> {
        Self::hook(
            cx.host_mut(),
            format!("host_resize_array_buffer {new_byte_length}"),
        );
        Ok(Handled::Unhandled)
    }
    fn host_grow_shared_array_buffer(
        cx: &mut HookCx<'_, Self>,
        _buffer: ObjectRef,
        new_byte_length: u64,
    ) -> Completion<Handled> {
        Self::hook(
            cx.host_mut(),
            format!("host_grow_shared_array_buffer {new_byte_length}"),
        );
        Ok(Handled::Unhandled)
    }
    fn host_system_utc_epoch_nanoseconds(cx: &mut HookCx<'_, Self>, _global: ObjectRef) -> i128 {
        Self::hook(
            cx.host_mut(),
            "host_system_utc_epoch_nanoseconds".to_string(),
        );
        1_700_000_000_000_000_000
    }
    fn system_time_zone_identifier(cx: &mut HookCx<'_, Self>) -> String {
        Self::hook(cx.host_mut(), "system_time_zone_identifier".to_string());
        "UTC".to_string()
    }
    fn get_named_time_zone_offset_nanoseconds(
        cx: &mut HookCx<'_, Self>,
        id: &str,
        epoch_ns: i128,
    ) -> i128 {
        Self::hook(
            cx.host_mut(),
            format!("get_named_time_zone_offset_nanoseconds {id} {epoch_ns}"),
        );
        0
    }
    fn get_named_time_zone_epoch_nanoseconds(
        cx: &mut HookCx<'_, Self>,
        id: &str,
        local: IsoDateTime,
    ) -> Vec<i128> {
        Self::hook(
            cx.host_mut(),
            format!("get_named_time_zone_epoch_nanoseconds {id} {local:?}"),
        );
        Vec::new()
    }
    fn available_named_time_zone_identifiers(cx: &mut HookCx<'_, Self>) -> Vec<TimeZoneIdentifier> {
        Self::hook(
            cx.host_mut(),
            "available_named_time_zone_identifiers".to_string(),
        );
        Vec::new()
    }
}

/// A value as text, through ToString for a primitive (Number to String conversions included)
/// and by `typeof`-like kind for an object; a String in quotes.
fn text(cx: &mut Cx<'_, Transcript>, value: Value) -> String {
    match value {
        Value::Object(_) => "object".to_string(),
        Value::String(string) => format!("{:?}", cx.to_utf16(string).unwrap()),
        Value::Symbol(_) => "symbol".to_string(),
        other => match cx.to_string(other).unwrap() {
            Ok(string) => cx.to_rust_string_lossy(string).unwrap(),
            Err(abrupt) => format!("{abrupt:?}"),
        },
    }
}

/// The host function `print`: writes its arguments, each as ToString gives it (a throw of
/// ToString written as such), and answers its argument count.
fn print(cx: &mut Cx<'_, Transcript>, call: HostCall<'_>) -> Completion<Value> {
    let mut line = String::from("print");
    for &arg in call.args {
        let string = cx.to_string(arg).unwrap()?;
        let shown = cx.to_rust_string_lossy(string).unwrap();
        line.push(' ');
        line.push_str(&shown);
    }
    cx.host_mut().lines.push(line);
    Ok(Value::Number(call.args.len() as f64))
}

/// The host function `callBack`: calls its first argument with the rest, from Rust (a
/// re-entry), and writes what it answered.
fn call_back(cx: &mut Cx<'_, Transcript>, call: HostCall<'_>) -> Completion<Value> {
    let (function, rest) = match call.args.split_first() {
        Some((&function, rest)) => (function, rest),
        None => (Value::Undefined, &[][..]),
    };
    let answer = cx.call(function, Value::Undefined, rest).unwrap();
    let shown = match answer {
        Ok(value) => format!("callBack normal {}", text(cx, value)),
        Err(Abrupt::Throw(value)) => format!("callBack throw {}", text(cx, value)),
        Err(Abrupt::Terminated(termination)) => format!("callBack terminated {termination:?}"),
    };
    cx.host_mut().lines.push(shown);
    answer
}

/// The corpus: slice 1 scripts over the interpreter, the builtins, the budgets and the bounds,
/// and the host functions above; each is run in this order in one agent, so that later scripts
/// see what earlier ones left.
const CORPUS: &[&str] = &[
    "1 + 1;",
    "var o = { b: 1, a: 2, 10: 3, 2: 4 }; o.c = 5; delete o.b; var keys = []; for (var k in o) keys.push(k); print(keys.join());",
    "print(0.1 + 0.2, 1e21, 1 / 3, -0, 0 / 0, 1 / 0, 123456789012345680000, 5e-7, 2 / 3 * 1e300 * 1e10);",
    "print(Math.max(1, 3, 2), Math.max(), Math.max(NaN, 1), Math.max(-0, 0));",
    "print(Object.getOwnPropertyNames({ z: 1, y: 2, 1: 0, 0: 0 }).join(), Object.getOwnPropertyNames([1, 2]).join());",
    "print(JSON.stringify({ a: [1, 'x', null, true], b: { c: undefined, d: 1.5 } }, null, 2));",
    "print(JSON.stringify({ a: 1, b: 2 }, function (k, v) { return typeof v === 'number' ? v + 1 : v; }));",
    "var fib = function (n) { return n < 2 ? n : fib(n - 1) + fib(n - 2); }; print(fib(15));",
    "function counter() { var c = 0; return function () { return ++c; }; } var next = counter(); next(); next(); print(next());",
    "var a = []; for (var i = 0; i < 50; i++) a.push(i * i); print(a.map(function (x) { return x % 7; }).join(''));",
    "print('hello world'.indexOf('o'), 'hello'.indexOf('z'), String(12.5), String(true));",
    "function P(x) { this.x = x; } var B = P.bind(null, 7); var p = new B(); print(p.x, p instanceof P, p instanceof B);",
    "print(callBack(function (a, b) { return a + b; }, 2, 3));",
    "callBack(function () { throw new TypeError('from the callback'); });",
    "try { null.x; } catch (e) { print(e.name, e.message); }",
    "var f; try { while (true) {} } finally { f = 1; }",
    "print(f);",
    "function deep() { deep(); } deep();",
    "var s = 'x'; while (true) s = s + s;",
    "var h = []; while (true) h.push({ a: h.length });",
    "h = undefined; print(typeof h);",
    "var 1;",
    "throw { toString: function () { return 'thrown'; } };",
    "Object.defineProperty(this, 'fixed', { value: 1, writable: false }); fixed = 2; print(fixed);",
    "'use strict'; var r; try { undefined_variable = 1; } catch (e) { r = e.name; } print(r);",
    "print([1, [2, [3, [4]]]].join(';'), Array.isArray([]), Array.isArray({}));",
    "var e = new RangeError('r'); print(e.toString(), Object.prototype.toString.call(e), Object.prototype.toString.call([]));",
];

/// The limits of the corpus: small enough that the endless scripts reach their budgets fast,
/// also with the collector run at every safe point (`fib(15)` takes about 27,000 steps).
fn corpus_limits() -> Limits {
    let mut limits = limits();
    limits.steps_per_entry = 40_000;
    limits.memory_bytes = 256 * 1024;
    limits.frame_depth = 200;
    limits.reentry_depth = 20;
    limits.string_length = 1 << 12;
    limits
}

/// An agent of the corpus run, its realm, and its host functions defined.
fn corpus_agent() -> (Agent<Transcript>, Realm) {
    let mut agent = Agent::new(Transcript::default(), corpus_limits(), seeds()).unwrap();
    let realm = match agent.create_realm(None, None).unwrap() {
        Outcome::Normal(realm) => realm,
        other => panic!("create_realm answered {other:?}"),
    };
    let global = agent.global_object(&realm).unwrap();
    agent
        .with_cx(&realm, |cx| {
            let Value::Object(global) = cx.get(&global).unwrap() else {
                panic!("the global");
            };
            let realm_ref = cx.realm_ref(&realm).unwrap();
            for (name, steps) in [
                ("print", print as swell::HostFn<Transcript>),
                ("callBack", call_back),
            ] {
                let f = cx
                    .new_host_function(realm_ref, name, 1, steps, false, None)
                    .unwrap()
                    .unwrap();
                let key = PropertyKey::String(cx.new_string(name).unwrap());
                cx.set(global, key, Value::Object(f), true)
                    .unwrap()
                    .unwrap();
            }
        })
        .unwrap();
    (agent, realm)
}

/// What a root of an outcome holds, as text: an object through ToString (an error as
/// `Error.prototype.toString` writes it), which may run script code.
fn root_text(agent: &mut Agent<Transcript>, realm: &Realm, root: &Root) -> String {
    agent
        .with_cx(realm, |cx| {
            let value = cx.get(root).unwrap();
            match value {
                Value::Object(_) => match cx.to_string(value).unwrap() {
                    Ok(string) => format!("object {}", cx.to_rust_string_lossy(string).unwrap()),
                    Err(abrupt) => format!("object {abrupt:?}"),
                },
                other => text(cx, other),
            }
        })
        .unwrap()
}

/// Run the corpus item `index` in `agent` and write what it did: the parse, the outcome, the
/// steps and heap bytes the entry left, and a collection's report. The steps are read before
/// the outcome is: reading it is an entry of its own.
fn run_item(agent: &mut Agent<Transcript>, realm: &Realm, index: usize) {
    let source = CORPUS.get(index).copied().unwrap_or("");
    let (outcome, steps, bytes) = match agent
        .parse_script(realm, Source::Utf8(source), None)
        .unwrap()
    {
        Parsed::Parsed(script) => {
            let outcome = agent.evaluate_script(&script).unwrap();
            let read = (
                agent.last_entry_steps().unwrap(),
                agent.heap_bytes().unwrap(),
            );
            (Ok(outcome), read.0, read.1)
        }
        Parsed::Errors(errors) => {
            let read = (
                agent.last_entry_steps().unwrap(),
                agent.heap_bytes().unwrap(),
            );
            (Err(errors), read.0, read.1)
        }
        Parsed::Terminated(termination) => (Ok(Outcome::Terminated(termination)), 0, 0),
    };
    let line = match outcome {
        Ok(Outcome::Normal(root)) => format!("normal {}", root_text(agent, realm, &root)),
        Ok(Outcome::Throw(root)) => format!("throw {}", root_text(agent, realm, &root)),
        Ok(Outcome::Terminated(termination)) => format!("terminated {termination:?}"),
        Err(errors) => {
            let messages: Vec<String> = errors
                .iter()
                .map(|error| root_text(agent, realm, error))
                .collect();
            format!("errors {}: {}", messages.len(), messages.join(" / "))
        }
    };
    let report = match agent.collect().unwrap() {
        Outcome::Normal(report) => {
            format!("{} {}", report.heap_bytes_before, report.heap_bytes_after)
        }
        other => format!("{other:?}"),
    };
    agent.host_mut().lines.push(format!(
        "{index}: {line}; steps {steps}; heap {bytes}; collect {report}"
    ));
}

/// The transcripts of two agents with equal limits and seeds that run the corpus in one
/// process, each call of one followed by the same call of the other.
fn two_agents_interleaved() -> (String, String) {
    let (mut a, realm_a) = corpus_agent();
    let (mut b, realm_b) = corpus_agent();
    for index in 0..CORPUS.len() {
        run_item(&mut a, &realm_a, index);
        run_item(&mut b, &realm_b, index);
    }
    (a.host().lines.join("\n"), b.host().lines.join("\n"))
}

/// The variable that makes this test binary a child that prints the corpus transcript.
const CHILD: &str = "SWELL_DETERMINISM_CHILD";
/// The lines around the transcript a child prints.
const BEGIN: &str = "--- determinism transcript begin ---";
const END: &str = "--- determinism transcript end ---";

/// The transcript a child process printed.
fn child_transcript() -> String {
    let exe = std::env::current_exe().unwrap();
    let out = std::process::Command::new(exe)
        .args([
            "host_133_same_calls_same_hooks_same_outcomes",
            "--exact",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(CHILD, "1")
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(
        out.status.success(),
        "the child failed: {}\n{stdout}\n{}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
    let start = stdout.find(BEGIN).map(|i| i + BEGIN.len() + 1);
    let end = stdout.find(END);
    match (start, end) {
        (Some(start), Some(end)) if start <= end => stdout
            .get(start..end)
            .unwrap_or_default()
            .trim_end()
            .to_string(),
        _ => panic!("the child printed no transcript:\n{stdout}"),
    }
}

/// HOST-133: the corpus run by two agents with equal limits and seeds in one process, and by
/// two child processes, gives the same transcript of hook calls, host function calls, outcomes,
/// `last_entry_steps()`, `heap_bytes()` and collection reports, byte for byte. The corpus
/// reaches every outcome: normal, throw, a parse error, and the step and memory budgets, the
/// frame depth and the string length.
#[test]
fn host_133_same_calls_same_hooks_same_outcomes() {
    if std::env::var_os(CHILD).is_some() {
        let (mut agent, realm) = corpus_agent();
        for index in 0..CORPUS.len() {
            run_item(&mut agent, &realm, index);
        }
        let a = agent.host().lines.join("\n");
        println!("{BEGIN}\n{a}\n{END}");
        return;
    }
    let (a, b) = two_agents_interleaved();
    assert_eq!(a, b, "two agents in one process");
    let lines: Vec<&str> = a.lines().collect();
    for expected in [
        "normal",
        "throw",
        "errors 1",
        "terminated StepBudget",
        "terminated MemoryBudget",
        "frame_depth",
        "string_length",
        "callBack normal",
        "callBack throw",
        "print ",
    ] {
        assert!(
            lines.iter().any(|line| line.contains(expected)),
            "the corpus reaches {expected:?}:\n{a}"
        );
    }
    let first = child_transcript();
    let second = child_transcript();
    assert_eq!(first, second, "two processes");
    assert_eq!(first, a, "a process and this one");
}
