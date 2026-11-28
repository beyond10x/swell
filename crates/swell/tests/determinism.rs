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

use crate::support;

use support::host::{limits, seeds};
use support::scripted::{Hooks, Scripted};
use support::stack::{TestPath, test_path};
use swell::{
    Abrupt, Agent, Brand, CompileStrings, Completion, Cx, Handled, HookCx, HostCall,
    HostObjectSpec, IsoDateTime, Job, Limits, LoadRequest, ModuleRef, ObjectRef, Outcome, Parsed,
    PropertyKey, Prototype, Realm, RealmRef, RejectionOperation, Root, Source, StringRef,
    Termination, TimeZoneIdentifier, Value,
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

fn host_ensure_can_compile_strings(
    cx: &mut Cx<'_, Scripted>,
    request: &CompileStrings<'_>,
) -> Completion<()> {
    let kind = request.compilation_type;
    Transcript::hook(
        cx.host_mut().state_mut::<Transcript>(),
        format!("host_ensure_can_compile_strings {kind:?}"),
    );
    Ok(())
}

fn host_get_code_for_eval(cx: &mut HookCx<'_, Scripted>, _arg: ObjectRef) -> Option<StringRef> {
    Transcript::hook(
        cx.host_mut().state_mut::<Transcript>(),
        "host_get_code_for_eval".to_string(),
    );
    None
}

fn host_ensure_can_add_private_element(
    cx: &mut HookCx<'_, Scripted>,
    _obj: ObjectRef,
) -> Completion<()> {
    Transcript::hook(
        cx.host_mut().state_mut::<Transcript>(),
        "host_ensure_can_add_private_element".to_string(),
    );
    Ok(())
}

fn host_has_source_text_available(cx: &mut HookCx<'_, Scripted>, _func: ObjectRef) -> bool {
    Transcript::hook(
        cx.host_mut().state_mut::<Transcript>(),
        "host_has_source_text_available".to_string(),
    );
    true
}

fn host_make_job_callback(
    cx: &mut HookCx<'_, Scripted>,
    _callback: ObjectRef,
) -> Option<swell::HostDefined> {
    Transcript::hook(
        cx.host_mut().state_mut::<Transcript>(),
        "host_make_job_callback".to_string(),
    );
    None
}

fn host_call_job_callback(
    cx: &mut Cx<'_, Scripted>,
    callback: ObjectRef,
    _host_defined: Option<&swell::HostDefined>,
    this: Value,
    args: &[Value],
) -> Completion<Value> {
    Transcript::hook(
        cx.host_mut().state_mut::<Transcript>(),
        format!("host_call_job_callback {}", args.len()),
    );
    cx.call(Value::Object(callback), this, args)
        .unwrap_or(Err(Abrupt::Terminated(Termination::EngineFault)))
}

fn host_enqueue_promise_job(cx: &mut HookCx<'_, Scripted>, realm: Option<RealmRef>) {
    Transcript::hook(
        cx.host_mut().state_mut::<Transcript>(),
        format!("host_enqueue_promise_job {}", realm.is_some()),
    );
}

fn host_prepare_to_run_job(cx: &mut HookCx<'_, Scripted>, _realm: RealmRef) {
    Transcript::hook(
        cx.host_mut().state_mut::<Transcript>(),
        "host_prepare_to_run_job".to_string(),
    );
}

fn host_clean_up_after_job(
    cx: &mut HookCx<'_, Scripted>,
    _realm: Option<RealmRef>,
    result: Result<(), Value>,
) {
    Transcript::hook(
        cx.host_mut().state_mut::<Transcript>(),
        format!("host_clean_up_after_job {}", result.is_ok()),
    );
}

fn host_enqueue_generic_job(cx: &mut HookCx<'_, Scripted>, job: Job) {
    Transcript::hook(
        cx.host_mut().state_mut::<Transcript>(),
        format!("host_enqueue_generic_job {:?}", job.kind()),
    );
}

fn host_enqueue_timeout_job(cx: &mut HookCx<'_, Scripted>, job: Job, milliseconds: f64) {
    Transcript::hook(
        cx.host_mut().state_mut::<Transcript>(),
        format!("host_enqueue_timeout_job {:?} {milliseconds}", job.kind()),
    );
}

fn host_enqueue_finalization_registry_cleanup_job(cx: &mut HookCx<'_, Scripted>, job: Job) {
    Transcript::hook(
        cx.host_mut().state_mut::<Transcript>(),
        format!(
            "host_enqueue_finalization_registry_cleanup_job {:?}",
            job.kind()
        ),
    );
}

fn host_promise_rejection_tracker(
    cx: &mut HookCx<'_, Scripted>,
    _promise: ObjectRef,
    operation: RejectionOperation,
) {
    Transcript::hook(
        cx.host_mut().state_mut::<Transcript>(),
        format!("host_promise_rejection_tracker {operation:?}"),
    );
}

fn host_load_imported_module(cx: &mut HookCx<'_, Scripted>, request: LoadRequest) {
    Transcript::hook(
        cx.host_mut().state_mut::<Transcript>(),
        format!("host_load_imported_module {:?}", request.specifier()),
    );
}

fn host_get_import_meta_properties(
    cx: &mut HookCx<'_, Scripted>,
    _module: ModuleRef,
) -> Completion<Vec<(PropertyKey, Value)>> {
    Transcript::hook(
        cx.host_mut().state_mut::<Transcript>(),
        "host_get_import_meta_properties".to_string(),
    );
    Ok(Vec::new())
}

fn host_finalize_import_meta(
    cx: &mut HookCx<'_, Scripted>,
    _import_meta: ObjectRef,
    _module: ModuleRef,
) {
    Transcript::hook(
        cx.host_mut().state_mut::<Transcript>(),
        "host_finalize_import_meta".to_string(),
    );
}

fn host_resize_array_buffer(
    cx: &mut HookCx<'_, Scripted>,
    _buffer: ObjectRef,
    new_byte_length: u64,
) -> Completion<Handled> {
    Transcript::hook(
        cx.host_mut().state_mut::<Transcript>(),
        format!("host_resize_array_buffer {new_byte_length}"),
    );
    Ok(Handled::Unhandled)
}

fn host_grow_shared_array_buffer(
    cx: &mut HookCx<'_, Scripted>,
    _buffer: ObjectRef,
    new_byte_length: u64,
) -> Completion<Handled> {
    Transcript::hook(
        cx.host_mut().state_mut::<Transcript>(),
        format!("host_grow_shared_array_buffer {new_byte_length}"),
    );
    Ok(Handled::Unhandled)
}

fn host_system_utc_epoch_nanoseconds(cx: &mut HookCx<'_, Scripted>, _global: ObjectRef) -> i128 {
    Transcript::hook(
        cx.host_mut().state_mut::<Transcript>(),
        "host_system_utc_epoch_nanoseconds".to_string(),
    );
    1_700_000_000_000_000_000
}

fn system_time_zone_identifier(cx: &mut HookCx<'_, Scripted>) -> String {
    Transcript::hook(
        cx.host_mut().state_mut::<Transcript>(),
        "system_time_zone_identifier".to_string(),
    );
    "UTC".to_string()
}

fn get_named_time_zone_offset_nanoseconds(
    cx: &mut HookCx<'_, Scripted>,
    id: &str,
    epoch_ns: i128,
) -> i128 {
    Transcript::hook(
        cx.host_mut().state_mut::<Transcript>(),
        format!("get_named_time_zone_offset_nanoseconds {id} {epoch_ns}"),
    );
    0
}

fn get_named_time_zone_epoch_nanoseconds(
    cx: &mut HookCx<'_, Scripted>,
    id: &str,
    local: IsoDateTime,
) -> Vec<i128> {
    Transcript::hook(
        cx.host_mut().state_mut::<Transcript>(),
        format!("get_named_time_zone_epoch_nanoseconds {id} {local:?}"),
    );
    Vec::new()
}

fn available_named_time_zone_identifiers(cx: &mut HookCx<'_, Scripted>) -> Vec<TimeZoneIdentifier> {
    Transcript::hook(
        cx.host_mut().state_mut::<Transcript>(),
        "available_named_time_zone_identifiers".to_string(),
    );
    Vec::new()
}

/// The hooks of the tests of this module, given to the scripted host with the state
/// [`Transcript`] (`support/scripted.rs`).
const HOOKS: Hooks = Hooks {
    host_ensure_can_compile_strings,
    host_get_code_for_eval,
    host_ensure_can_add_private_element,
    host_has_source_text_available,
    host_make_job_callback,
    host_call_job_callback,
    host_enqueue_promise_job,
    host_prepare_to_run_job,
    host_clean_up_after_job,
    host_enqueue_generic_job,
    host_enqueue_timeout_job,
    host_enqueue_finalization_registry_cleanup_job,
    host_promise_rejection_tracker,
    host_load_imported_module,
    host_get_import_meta_properties,
    host_finalize_import_meta,
    host_resize_array_buffer,
    host_grow_shared_array_buffer,
    host_system_utc_epoch_nanoseconds,
    system_time_zone_identifier,
    get_named_time_zone_offset_nanoseconds,
    get_named_time_zone_epoch_nanoseconds,
    available_named_time_zone_identifiers,
};

/// A value as text, through ToString for a primitive (Number to String conversions included)
/// and by `typeof`-like kind for an object; a String in quotes.
fn text(cx: &mut Cx<'_, Scripted>, value: Value) -> String {
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
fn print(cx: &mut Cx<'_, Scripted>, call: HostCall<'_>) -> Completion<Value> {
    let mut line = String::from("print");
    for &arg in call.args {
        let string = cx.to_string(arg).unwrap()?;
        let shown = cx.to_rust_string_lossy(string).unwrap();
        line.push(' ');
        line.push_str(&shown);
    }
    cx.host_mut().state_mut::<Transcript>().lines.push(line);
    Ok(Value::Number(call.args.len() as f64))
}

/// The host function `callBack`: calls its first argument with the rest, from Rust (a
/// re-entry), and writes what it answered.
fn call_back(cx: &mut Cx<'_, Scripted>, call: HostCall<'_>) -> Completion<Value> {
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
    cx.host_mut().state_mut::<Transcript>().lines.push(shown);
    answer
}

/// The script of the corpus that fills an Array in a loop and maps it: 50 elements (1,794
/// steps); 5 with the feature `collect-every-safe-point`, where 50 would not end within that
/// mode's step budget ([`corpus_limits`]).
const SQUARES: &str = if cfg!(feature = "collect-every-safe-point") {
    "var a = []; for (var i = 0; i < 5; i++) a.push(i * i); print(a.map(function (x) { return x % 7; }).join(''));"
} else {
    "var a = []; for (var i = 0; i < 50; i++) a.push(i * i); print(a.map(function (x) { return x % 7; }).join(''));"
};

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
    SQUARES,
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

/// The limits of the corpus but its memory budget, which [`corpus_setup`] finds: small enough
/// that the endless scripts reach their budgets fast (`fib(15)` takes about 27,000 steps). In
/// the default mode they are 40,000 steps per entry, a `frame_depth` of 200 and a
/// `string_length` of 4,096, in the gate and in the lane `slow` (`SWELL_FULL_COUNTS=1`) alike.
/// With the feature `collect-every-safe-point` they are 300 steps per entry, a `frame_depth` of
/// 30 and a `string_length` of 100, and [`SQUARES`] fills 5 elements: every step collects and
/// this process runs the corpus twice and most of it once more in the probe of
/// [`keeping_loop_budget`], while a child runs it beside. With 10,000 steps the test was
/// terminated at 60 s in that mode, at a load of 100 to 110 on 17 cores (cargo-nextest); with
/// 2,000 steps, a `string_length` of 256 and 50 elements it took 6.2 s alone in the test
/// profile (opt-level 0) on 2026-11-27, the corpus 8,100 steps a run, of them 5,794 in
/// `fib(15)`, `while (true) {}` and [`SQUARES`] and 598 in the doubling String. Every script
/// that ended normally there still does, the longest the 148 steps of the `for`-`in`, and each
/// endless one still ends at its budget. The memory budget here is the test limits' own, wide
/// enough that nothing the corpus does is refused; the probes of [`corpus_setup`] run under it.
fn corpus_limits() -> Limits {
    let mut limits = limits();
    limits.steps_per_entry = 40_000;
    limits.frame_depth = 200;
    limits.reentry_depth = 20;
    limits.string_length = 1 << 12;
    if cfg!(feature = "collect-every-safe-point") {
        limits.steps_per_entry = 300;
        limits.frame_depth = 30;
        limits.string_length = 100;
    }
    limits
}

/// The slots of the arena of objects a corpus agent has free when the corpus starts: 48; 22
/// with the feature `collect-every-safe-point`. More than the scripts before the loop that
/// keeps its objects hold at once (with the collector at every safe point they hold the dozen
/// objects they keep in globals and the few a statement makes, 15 on 2026-11-27, when 48 left
/// the loop 33 iterations to the growth; were it fewer, they would take the growth, and
/// [`keeping_loop_budget`] would find none in the loop and fail), and few enough that the loop,
/// which keeps one object per iteration at about 17 steps, fills them within half of the steps
/// it has (40,000; 300 with that collector, where 22 leave it about 7 iterations).
const FREE_OBJECT_SLOTS: u64 = if cfg!(feature = "collect-every-safe-point") {
    22
} else {
    48
};

/// The least rise of `heap_bytes()` that one kept object cannot make by itself: 2 KiB. An
/// object of [`pin_objects`] or `{ a: h.length }` is charged its record's property table, and an
/// element of `h` 16 bytes; an arena of objects that grows is charged its new slots, at least 16
/// of 212 bytes, and from a realm on 54,272 bytes or more.
const DOUBLING_FLOOR: u64 = 2 * 1024;

/// The room the budget leaves above the charge the corpus returns to between its scripts, at the
/// least: 128 KiB in the default mode, where `deep()` reaches `frame_depth` 200 only with about
/// 84 KiB (a budget of 256,924 bytes over a realm's charge of 172,603 on `cc41ae6`); 40 KiB with
/// the collector at every safe point, where its 30 frames take less than that and the nested
/// `join` needs more than 20,000 bytes.
const LEAST_ROOM: u64 = if cfg!(feature = "collect-every-safe-point") {
    40 * 1024
} else {
    128 * 1024
};

/// The room the script after the loop that keeps its objects, `h = undefined; print(typeof
/// h);`, needs while `h` still holds them: 8 KiB, more than twice what a parse and run of a
/// script of its size takes (the script `1` needs 2,228 bytes, `tests/termination.rs`).
const ROOM_AFTER_THE_LOOP: u64 = 8 * 1024;

/// The variable that hands the setup of this process to a child: `<memory_bytes> <pins>`.
const SETUP: &str = "SWELL_DETERMINISM_SETUP";

/// What every corpus agent of a run is made with: [`corpus_limits`] with the memory budget
/// [`corpus_setup`] found, and the number of objects its host keeps before the corpus starts.
#[derive(Clone, Copy)]
struct CorpusSetup {
    limits: Limits,
    pins: u64,
}

/// The setup of the corpus agents of this run, from readings the test makes itself. A child
/// takes its parent's (the variable [`SETUP`]), so that every agent of the run has the same.
///
/// Why it is measured: the outcome `MemoryBudget` comes from the loop that keeps its objects
/// (`var h = []; while (true) h.push({ a: h.length });`), and the budget must refuse a growth of
/// the arena of objects in that loop, large enough that the refusal leaves room for the next
/// script, while every other script runs as it would with no budget. Where that growth falls
/// moves with every object a realm gains: measured on 2026-11-24 with the collector at every
/// safe point and the budget as the realm's charge plus 64 KiB, the realm charged 214,160 bytes
/// with 256 slots of objects, and the loop's growth to 512 slots (54,272 bytes, from 234,764 to
/// 236,197 bytes) was refused 8,811 to 10,244 bytes short; with the 11 functions of
/// `story:math-constants-and-exact-functions` the realm charged 219,907, the fifth script took
/// that growth at 229,241 with 2,459 bytes to spare, and every script after it was refused its
/// parse: the loop never reached the budget, and the corpus lost `StepBudget`, `frame_depth`
/// and `errors 1`.
///
/// How it is measured, in two probe agents under [`corpus_limits`]:
/// 1. The free slots ([`pins_leaving_free_slots`]): the host keeps one object at a time until
///    the arena grows, and once more if fewer than [`FREE_OBJECT_SLOTS`] were free; `pins` is
///    the number of objects that leaves exactly that many free. Every corpus agent keeps that
///    many, so the loop meets a growth of the arena after about the same number of objects
///    whatever the realm holds. On 2026-11-24, in both modes, `pins` was 224 at the realm of
///    `76aa4fc`, 213 with the 11 functions of that story, and 201 with its `Math` table installed
///    a second time.
/// 2. The budget ([`keeping_loop_budget`]): the scripts before the loop, then the loop one
///    iteration per entry for at most half of `steps_per_entry`; the largest rise of
///    `heap_bytes()` in one iteration is a growth of an arena, and the budget is the reading
///    before it plus half the rise. Until a charge is refused a budget changes nothing an agent
///    does, so the loop of the corpus reaches the same reading, give or take the few hundred
///    bytes of the probe's scripts, and is refused that growth with about half of it left. On
///    2026-11-24 with the collector at every safe point and the realm of that story: 213 pins,
///    the arena's growth from 512 to 1,024 slots (109,280 bytes) at 377,284, a budget of
///    431,924.
///
/// What it costs: the pins are made by the host, with no safe point ([`pin_objects`]), and
/// marking them adds little (one run of the corpus in the collect mode took 1.6 CPU seconds
/// with 213 pins and without); the second probe runs 19 of the 27 scripts of the corpus, the
/// costly ones among them. On 2026-11-24 the test took 6.1 CPU seconds in the collect mode at
/// the realm of `76aa4fc` and 6.4 to 6.9 at that story's, against 3.4 to 4.0 for the test of
/// `76aa4fc` at its realm (a budget of the realm's charge plus a constant, and no probe).
fn corpus_setup() -> CorpusSetup {
    let mut limits = corpus_limits();
    if let Some(passed) = std::env::var_os(SETUP) {
        let passed = passed.to_string_lossy().into_owned();
        let mut words = passed.split(' ').map(|word| word.parse::<u64>().unwrap());
        limits.memory_bytes = words.next().unwrap();
        let pins = words.next().unwrap();
        return CorpusSetup { limits, pins };
    }
    let pins = pins_leaving_free_slots(limits);
    limits.memory_bytes = keeping_loop_budget(CorpusSetup { limits, pins });
    CorpusSetup { limits, pins }
}

/// How many objects the host of a corpus agent keeps so that [`FREE_OBJECT_SLOTS`] slots of the
/// arena of objects are free when the corpus starts, read in a probe agent: [`pin_objects`]
/// keeps one object per call; the call that raises `heap_bytes()` by more than
/// [`DOUBLING_FLOOR`] grew the arena, so the objects before it filled the free slots.
fn pins_leaving_free_slots(limits: Limits) -> u64 {
    let (mut agent, realm) = corpus_agent_with(CorpusSetup { limits, pins: 0 });
    let roots = PinRoots::new(&mut agent, &realm);
    let mut before = agent.heap_bytes().unwrap();
    let mut growths = Vec::new();
    for kept in 1..=1_u64 << 17 {
        pin_objects(&mut agent, &realm, &roots, 1);
        let after = agent.heap_bytes().unwrap();
        if after.saturating_sub(before) > DOUBLING_FLOOR {
            // The kept objects before this one filled the arena.
            growths.push(kept - 1);
            match growths[..] {
                [free] if free >= FREE_OBJECT_SLOTS => return free - FREE_OBJECT_SLOTS,
                // Fewer were free: count on to the next growth, where every slot is taken.
                [_] => {}
                [_, full] => {
                    return full
                        .checked_sub(FREE_OBJECT_SLOTS)
                        .expect("the arena grew twice within FREE_OBJECT_SLOTS objects");
                }
                _ => unreachable!(),
            }
        }
        before = after;
    }
    panic!("no growth of the arena of objects in 131,072 objects");
}

/// The brand of the objects [`pin_objects`] makes.
const PIN_BRAND: Brand = Brand(133);

/// The global object and the keys `pin` and `next`, rooted once, so that a call of
/// [`pin_objects`] makes the objects it keeps and nothing else: no root, no string.
struct PinRoots {
    global: Root,
    pin: Root,
    next: Root,
}

impl PinRoots {
    fn new(agent: &mut Agent<Scripted>, realm: &Realm) -> PinRoots {
        let global = agent.global_object(realm).unwrap();
        let (pin, next) = agent
            .with_cx(realm, |cx| {
                let pin = cx.new_string("pin").unwrap();
                let next = cx.new_string("next").unwrap();
                (
                    cx.root(Value::String(pin)).unwrap(),
                    cx.root(Value::String(next)).unwrap(),
                )
            })
            .unwrap();
        PinRoots { global, pin, next }
    }
}

/// Keeps `count` more objects in the global `pin`, as `count` runs of the script `pin = { next:
/// pin };` would: each a host object with no prototype whose property `next` holds the one
/// before it. The host makes them in one call, with no safe point, so that no collection runs
/// while they are made (with the collector at every safe point the script took a collection
/// per instruction for each object).
fn pin_objects(agent: &mut Agent<Scripted>, realm: &Realm, roots: &PinRoots, count: u64) {
    agent
        .with_cx(realm, |cx| {
            let (Value::Object(global), Value::String(pin), Value::String(next)) = (
                cx.get(&roots.global).unwrap(),
                cx.get(&roots.pin).unwrap(),
                cx.get(&roots.next).unwrap(),
            ) else {
                panic!("the roots of the pins");
            };
            let (pin, next) = (PropertyKey::String(pin), PropertyKey::String(next));
            for _ in 0..count {
                let previous = cx.get_property(global, pin).unwrap().unwrap();
                let object = cx
                    .new_host_object(HostObjectSpec {
                        prototype: Prototype::Null,
                        brand: PIN_BRAND,
                        data: None,
                        error_kind: None,
                        overrides: None,
                    })
                    .unwrap()
                    .unwrap();
                cx.set(object, next, previous, true).unwrap().unwrap();
                cx.set(global, pin, Value::Object(object), true)
                    .unwrap()
                    .unwrap();
            }
        })
        .unwrap();
}

/// The memory budget of the corpus, read in a probe agent of `setup` with the budget of
/// [`corpus_limits`]: the scripts before the loop that keeps its objects, then that loop one
/// iteration per entry for at most half of `steps_per_entry`, the budget the reading before the
/// largest rise of `heap_bytes()` so far plus half of that rise. The loop stops at the first
/// rise that gives a budget both checks below pass: the iterations after it would run in every
/// probe and change nothing.
fn keeping_loop_budget(setup: CorpusSetup) -> u64 {
    let (mut agent, realm) = corpus_agent(setup);
    let keeping = CORPUS
        .iter()
        .position(|source| source.starts_with("var h = [];"))
        .unwrap();
    for index in 0..keeping {
        run_item(&mut agent, &realm, index);
    }
    let between = agent.heap_bytes().unwrap();
    let start = parse_corpus(&mut agent, &realm, "var h = [];");
    assert!(matches!(
        agent.evaluate_script(&start).unwrap(),
        Outcome::Normal(_)
    ));
    let push = parse_corpus(&mut agent, &realm, "h.push({ a: h.length });");
    let reach = setup.limits.steps_per_entry / 2;
    let (mut spent, mut iterations) = (0, 0);
    let mut before = agent.heap_bytes().unwrap();
    let (mut rise, mut at) = (0, before);
    while spent < reach {
        let outcome = agent.evaluate_script(&push).unwrap();
        assert!(matches!(outcome, Outcome::Normal(_)), "{outcome:?}");
        spent += agent.last_entry_steps().unwrap();
        iterations += 1;
        let after = agent.heap_bytes().unwrap();
        if after.saturating_sub(before) > rise {
            (rise, at) = (after - before, before);
        }
        before = after;
        if rise > DOUBLING_FLOOR
            && rise / 2 >= ROOM_AFTER_THE_LOOP
            && (at + rise / 2).saturating_sub(between) >= LEAST_ROOM
        {
            break;
        }
    }
    let budget = at + rise / 2;
    println!(
        "corpus setup: {} kept objects; {between} bytes before the loop; in {iterations} \
         iterations ({spent} steps) the largest rise {rise} at {at}; budget {budget}",
        setup.pins
    );
    assert!(
        rise > DOUBLING_FLOOR && rise / 2 >= ROOM_AFTER_THE_LOOP,
        "no growth of an arena of {} bytes or more in the loop's reach: the loop would not end \
         by memory with room for the next script; it needs more steps or fewer free slots",
        2 * ROOM_AFTER_THE_LOOP
    );
    assert!(
        budget - between >= LEAST_ROOM,
        "the budget {budget} leaves {} bytes above {between}, less than {LEAST_ROOM}: the loop's \
         growth comes too early; it needs a later one in its reach",
        budget - between
    );
    budget
}

/// `source` parsed in a corpus agent, which must parse.
fn parse_corpus(agent: &mut Agent<Scripted>, realm: &Realm, source: &str) -> swell::Script {
    match agent
        .parse_script(realm, Source::Utf8(source), None)
        .unwrap()
    {
        Parsed::Parsed(script) => script,
        _ => panic!("{source} did not parse"),
    }
}

/// An agent of the corpus run under `setup`, its realm, its host functions defined and its
/// objects kept.
fn corpus_agent(setup: CorpusSetup) -> (Agent<Scripted>, Realm) {
    let (mut agent, realm) = corpus_agent_with(setup);
    let roots = PinRoots::new(&mut agent, &realm);
    pin_objects(&mut agent, &realm, &roots, setup.pins);
    (agent, realm)
}

/// An agent under the limits of `setup`, its realm, the host functions of the corpus run
/// defined and the global `pin` declared (`null`).
fn corpus_agent_with(setup: CorpusSetup) -> (Agent<Scripted>, Realm) {
    let mut agent = Agent::new(
        Scripted::new(Transcript::default(), HOOKS),
        setup.limits,
        seeds(),
    )
    .unwrap();
    let realm = match support::host::create_realm(&mut agent, None, None).unwrap() {
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
                ("print", print as swell::HostFn<Scripted>),
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
            let key = PropertyKey::String(cx.new_string("pin").unwrap());
            cx.set(global, key, Value::Null, true).unwrap().unwrap();
        })
        .unwrap();
    (agent, realm)
}

/// What a root of an outcome holds, as text: an object through ToString (an error as
/// `Error.prototype.toString` writes it), which may run script code.
fn root_text(agent: &mut Agent<Scripted>, realm: &Realm, root: &Root) -> String {
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
fn run_item(agent: &mut Agent<Scripted>, realm: &Realm, index: usize) {
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
    agent
        .host_mut()
        .state_mut::<Transcript>()
        .lines
        .push(format!(
            "{index}: {line}; steps {steps}; heap {bytes}; collect {report}"
        ));
}

/// The transcripts of two agents with equal limits and seeds that run the corpus in one
/// process, each call of one followed by the same call of the other.
fn two_agents_interleaved(setup: CorpusSetup) -> (String, String) {
    let (mut a, realm_a) = corpus_agent(setup);
    let (mut b, realm_b) = corpus_agent(setup);
    for index in 0..CORPUS.len() {
        run_item(&mut a, &realm_a, index);
        run_item(&mut b, &realm_b, index);
    }
    (
        a.host().state::<Transcript>().lines.join("\n"),
        b.host().state::<Transcript>().lines.join("\n"),
    )
}

/// The variable that makes this test binary a child that prints the corpus transcript.
const CHILD: &str = "SWELL_DETERMINISM_CHILD";
/// The lines around the transcript a child prints.
const BEGIN: &str = "--- determinism transcript begin ---";
const END: &str = "--- determinism transcript end ---";

/// A child process that runs only the test `test`, which runs the corpus and prints its
/// transcript, started and not waited for: the two children of the test run beside each other
/// and beside the test's own two agents, each in a process of its own, so that the test takes
/// the time of one of the three runs rather than of all of them.
fn spawn_child(test: TestPath, setup: CorpusSetup) -> std::process::Child {
    let exe = std::env::current_exe().unwrap();
    std::process::Command::new(exe)
        .args(["--exact", test.as_str(), "--nocapture", "--test-threads=1"])
        .env(CHILD, "1")
        .env(
            SETUP,
            format!("{} {}", setup.limits.memory_bytes, setup.pins),
        )
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap()
}

/// The transcript a child process of [`spawn_child`] printed, once it ended.
fn child_transcript(child: std::process::Child) -> String {
    let out = child.wait_with_output().unwrap();
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
/// child processes ([`host_133_children`]), gives the same transcript of hook calls, host
/// function calls, outcomes, `last_entry_steps()`, `heap_bytes()` and collection reports, byte
/// for byte. The corpus
/// reaches every outcome: normal, throw, a parse error, and the step and memory budgets, the
/// frame depth and the string length. The memory budget and the objects a corpus agent keeps
/// are measured once, in this process ([`corpus_setup`]), and handed to the children.
#[test]
fn host_133_same_calls_same_hooks_same_outcomes() {
    if std::env::var_os(CHILD).is_some() {
        let (mut agent, realm) = corpus_agent(corpus_setup());
        for index in 0..CORPUS.len() {
            run_item(&mut agent, &realm, index);
        }
        let a = agent.host().state::<Transcript>().lines.join("\n");
        println!("{BEGIN}\n{a}\n{END}");
        return;
    }
    let setup = corpus_setup();
    let test = test_path!("host_133_same_calls_same_hooks_same_outcomes");
    let children: Vec<_> = (0..host_133_children())
        .map(|_| spawn_child(test.clone(), setup))
        .collect();
    let (a, b) = two_agents_interleaved(setup);
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
    for (child, transcript) in children.into_iter().map(child_transcript).enumerate() {
        assert_eq!(transcript, a, "child process {child} and this one");
    }
}

/// The child processes of HOST-133, each compared with this process (and so with each other):
/// two in the default mode, one with the feature `collect-every-safe-point`, in the gate and in
/// the lane `slow` (`SWELL_FULL_COUNTS=1`) alike, still two processes against each other. The
/// corpus runs four times with two children, and with them the test took 5.3 CPU seconds in
/// that mode, run alone, the most of any test there (5.7 in the lane, against 4.3 with one,
/// measured on 2026-11-22: 1.4 a run of the corpus, every step of it marking the heap).
fn host_133_children() -> usize {
    if cfg!(feature = "collect-every-safe-point") {
        1
    } else {
        2
    }
}

/// The keys of the key-order tests. HOST-25 and HOST-214 state 10,000 keys: the lane `slow`
/// (`SWELL_FULL_COUNTS=1`) runs 10,000 in the default mode, and 100 with the feature
/// `collect-every-safe-point`, where every builtin call collects a heap that holds them all
/// (the largest count measured under 20 s of wall time on the build node: HOST-25 took 16.2 s
/// at 100 keys in a run of the whole lane, 21.8 s at 200 run alone); the gate runs 1,000 and
/// 12. Each of the four counts is past
/// the size above which a property list and an entry table keep an index map keyed by
/// `hash_key` (8 entries), so an order that follows the index map, and so `hash_key`, differs
/// between the two agents of HOST-25 at each of them: an iteration planted to follow the index
/// map made HOST-25 red at 1,000 and at 12 keys. HOST-214 compares two processes with the same
/// `Seeds`, so it catches an order that depends on the process, at any count past the index
/// map. What only the full count adds is a table through more growths of its storage.
fn ordered_keys() -> u32 {
    let full = std::env::var("SWELL_FULL_COUNTS").is_ok_and(|value| value == "1");
    match (cfg!(feature = "collect-every-safe-point"), full) {
        (false, true) => 10_000,
        (false, false) => 1_000,
        (true, true) => 100,
        (true, false) => 12,
    }
}

/// A script that inserts `count` keys into an object, a `Map` and a `Set` (each third key a
/// String, a Number or an object), deletes every third and answers the three in iteration
/// order as one String: the object's own keys by `for`-`in`, then the Map's keys with their
/// values by `forEach`, then the Set's values by `for`-`of`, an object key by its number.
fn key_order_script(count: u32) -> String {
    format!(
        "var o = {{}}, m = new Map(), s = new Set(), keys = [], out = [];\n\
         for (var i = 0; i < {count}; i++) {{\n\
           var key = i % 3 === 0 ? \"k\" + i : i % 3 === 1 ? i * 1.5 : {{ n: i }};\n\
           keys.push(key); o[\"p\" + i] = i; m.set(key, i); s.add(key);\n\
         }}\n\
         for (var i = 0; i < {count}; i += 3) {{ delete o[\"p\" + i]; m.delete(keys[i]); s.delete(keys[i]); }}\n\
         function shown(key) {{ return typeof key === \"object\" ? \"o\" + key.n : String(key); }}\n\
         for (var p in o) out.push(p);\n\
         m.forEach(function (v, k) {{ out.push(shown(k) + \"=\" + v); }});\n\
         for (var v of s) out.push(shown(v));\n\
         out.length + \":\" + out.join();"
    )
}

/// The limits of the key-order tests: the test limits with a step budget no run reaches.
fn key_order_limits() -> Limits {
    let mut limits = limits();
    limits.steps_per_entry = 1 << 40;
    limits
}

/// The text the script of [`key_order_script`] answers in a fresh agent of `seeds`.
fn key_order(seeds: swell::Seeds) -> String {
    let mut agent = Agent::new(
        Scripted::new(Transcript::default(), HOOKS),
        key_order_limits(),
        seeds,
    )
    .unwrap();
    let realm = match support::host::create_realm(&mut agent, None, None).unwrap() {
        Outcome::Normal(realm) => realm,
        other => panic!("create_realm answered {other:?}"),
    };
    let script = parse_corpus(&mut agent, &realm, &key_order_script(ordered_keys()));
    let Outcome::Normal(root) = agent.evaluate_script(&script).unwrap() else {
        panic!("the key-order script did not answer normally");
    };
    agent
        .with_cx(&realm, |cx| match cx.get(&root).unwrap() {
            Value::String(string) => cx.to_rust_string_lossy(string).unwrap(),
            other => panic!("the key-order script answered {other:?}"),
        })
        .unwrap()
}

/// HOST-25 (`docs/design/host-interface.md`, decision 3): the key order of an object, a `Map`
/// and a `Set` does not depend on `hash_key`. Two agents with equal limits and seeds but
/// `hash_key` run one script that inserts 10,000 keys into each ([`ordered_keys`]: 1,000 in the
/// gate), deletes every third and writes them in iteration order: the two transcripts are equal
/// byte for byte, and each lists the two thirds of the keys left in each of the three.
#[test]
fn host_25_key_order_does_not_depend_on_hash_key() {
    let first = key_order(seeds());
    let mut other = seeds();
    other.hash_key = [0x5a; 16];
    assert_ne!(other.hash_key, seeds().hash_key);
    let second = key_order(other);
    let left = ordered_keys() - ordered_keys().div_ceil(3);
    assert!(
        first.starts_with(&format!("{}:", 3 * left)),
        "{}",
        &first[..first.len().min(200)]
    );
    assert!(first == second, "the two transcripts differ");
}

/// HOST-214 (`docs/design/host-interface.md`, decision 3): two child processes ([`spawn_child`])
/// whose agents have the same `Seeds` print the key order of a `Map` of 10,000 keys
/// ([`ordered_keys`]: 1,000 in the gate; with an object and a `Set` beside it), and the two
/// prints are equal.
#[test]
fn host_214_two_processes_print_the_same_map_order() {
    if std::env::var_os(CHILD).is_some() {
        let order = key_order(seeds());
        println!("{BEGIN}\n{order}\n{END}");
        return;
    }
    let test = test_path!("host_214_two_processes_print_the_same_map_order");
    let setup = CorpusSetup {
        limits: key_order_limits(),
        pins: 0,
    };
    let children: Vec<_> = (0..2).map(|_| spawn_child(test.clone(), setup)).collect();
    let prints: Vec<String> = children.into_iter().map(child_transcript).collect();
    let left = ordered_keys() - ordered_keys().div_ceil(3);
    assert!(
        prints[0].starts_with(&format!("{}:", 3 * left)),
        "{}",
        &prints[0][..prints[0].len().min(200)]
    );
    assert!(prints[0] == prints[1], "the two prints differ");
}
