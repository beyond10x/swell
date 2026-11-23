//! The fuzz test of the slice 1 host interface (`docs/design/host-interface.md`, decision 1,
//! HOST-3; `docs/design/engine.md`, decision 13, "Fuzzing is tooling"): a generator written
//! with `std` only issues, from a seed, sequences of interface calls on two agents, every
//! `Agent` entry point and read, `with_cx` with every `Cx` operation, setup closures with the
//! `HookCx` operations, and host functions that call back into the engine, with the microtask
//! queue among them (host microtasks queued through `Agent` and `Cx`, checkpoints of both, and
//! `clear_microtasks`; `story:job-queue-and-microtask-checkpoint`), with arbitrary
//! arguments: stale, released, foreign and wrong-kind handles among them, and small scripts of
//! the slice 1 grammar and broken sources, under small and random limits.
//!
//! 400 sequences from fixed seeds per run in the lane `slow`, 15 with the feature
//! `collect-every-safe-point`; the gate runs the first 160 and 15 ([`sequences`]). They are
//! divided among eight tests ([`PARTS`]) that cargo-nextest runs side by side. A sequence fails
//! on a panic, on an abort (a stack overflow; the sequences run in a child process, which
//! prints each seed before it runs it, so the parent names the seed the child died in), on a
//! run past its budget (`last_entry_steps()` over `steps_per_entry` or `heap_bytes()` over
//! `memory_bytes` after an entry), on an answer the design excludes (a faulted agent that
//! answers anything but `Err(Faulted)`, a foreign handle that is not answered
//! `Err(ForeignHandle)`, an entry that faults a live agent though no host function or setup
//! closure returned what decision 6 refuses and no host data lists a stale or foreign handle,
//! or one that does not fault it though one did), and on a difference between two runs of one
//! seed: each sequence runs twice and its transcripts are compared byte for byte, with the agent
//! keys written as `A` and `B`. A failing sequence is run again alone with
//! `SWELL_FUZZ_SEED=<seed>`. Each sequence runs on a thread of the agents'
//! `Limits::min_stack_bytes()` plus a fixed allowance for the frames of this test's own host
//! functions. Run also with the feature `collect-every-safe-point`.
#![cfg(test)]

use crate::support;

use support::stack::{TestPath, test_path};
use swell::{
    Abrupt, Agent, AgentState, Brand, CompileStrings, Completion, Cx, ErrorKind, GlobalObjects,
    Handled, HookCx, Host, HostCall, HostData, HostError, HostObjectSpec, IsoDateTime, Job, Limits,
    LoadRequest, ModuleRef, ObjectRef, Outcome, Parsed, PreferredType, PropertyDescriptor,
    PropertyKey, Prototype, Realm, RealmRef, RejectionOperation, Root, Script, ScriptRef, Seeds,
    Source, StringRef, Termination, TimeZoneIdentifier, Tracer, Value,
};

/// The sequences of one run: 400 in the default mode; with the feature
/// `collect-every-safe-point`, the first 15 of them, in the gate and in the lane `slow` alike.
/// That mode's job is to find a value held without a root, which any sequence that keeps a
/// value across a safe point shows; a sequence costs many times more in it (2,000 took 170.9 s
/// there against 24.6 s in the default mode in one pool job, a factor of 7: 11.7 against 81
/// sequences per second, measured).
///
/// 400 is the full count, which the lane `slow` runs (`SWELL_FULL_COUNTS=1`). The counts were
/// 2,000 and 150, counts the story (`story:host-interface-hardening`) and the time of one
/// pool job gave, no defect: a part of 250 sequences took 7.7 to 8.1 CPU seconds alone in the
/// default mode and 46.6 to 54.9 s in a loaded lane, near the kill; then 1,000 and 50, whose
/// parts of 125 took 3.8 to 4.3 CPU seconds alone, and the lane's tests are to stay under 2.5.
/// 400 sequences, 50 a part, draw each kind of the generator about 280 times (9,600 entry
/// calls). In the collect mode two sequences cost most of the 150: the seeds 55 and 58 (from
/// [`FIRST_SEED`]) run 20,000 steps an entry with a heap of up to 0.5 and 2 MB that every step
/// marks, about 13 and 16 CPU seconds each, so the parts that held them took 14.7 and 16.8 s;
/// the first 50 take 4.2 CPU seconds together, the most in a part 2.6 (seed 23, 2.2 s), all
/// measured alone on 2026-11-22, and the sequences from 15 on are the ones that cost (below).
/// The gate runs
/// the first 160 and, in the collect mode, the first 15: every choice of the generator is drawn
/// uniformly among at most 36 kinds, so the 3,840 entry calls of 160 sequences (360 of 15) draw
/// each kind about a hundred times (about ten), and a kind the full count reaches is not left
/// out. With 400 and 40 the parts took 15.1 to 22.6 s each in the default mode, and in the
/// collect mode the fourth and the fifth 36.3 and 31.8 s (the sequences 15 to 24, the others
/// 1.0 to 3.6 s a part), at a load of 100 to 110 on 17 cores (cargo-nextest). The first 15 took
/// 4.7 s together there.
fn sequences() -> u64 {
    let full = std::env::var("SWELL_FULL_COUNTS").is_ok_and(|value| value == "1");
    match (cfg!(feature = "collect-every-safe-point"), full) {
        (true, _) => 15,
        (false, true) => 400,
        (false, false) => 160,
    }
}
/// The seed of the first sequence; the others follow it.
const FIRST_SEED: u64 = 0x5EED_0000;
/// The calls of one sequence at the top level.
const CALLS: u64 = 24;
/// The `Cx` operations a `with_cx` closure or a host function makes at most.
const CX_CALLS: u64 = 6;
/// How deep host functions of this test nest their own operations; deeper ones only answer.
const HOST_DEPTH: u32 = 3;
/// The values, roots, realms and scripts a sequence keeps at most.
const KEEP: usize = 48;
/// The stack this test's own frames take beyond what the engine states.
const HARNESS_STACK: usize = 4 << 20;

/// splitmix64: a generator of `std` only.
#[derive(Clone, Copy, Debug, Default)]
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A number below `n`, 0 for `n` 0.
    fn below(&mut self, n: u64) -> u64 {
        if n == 0 { 0 } else { self.next() % n }
    }

    /// An index into a list of `len`.
    fn index(&mut self, len: usize) -> usize {
        usize::try_from(self.below(u64::try_from(len).unwrap_or(0))).unwrap_or(0)
    }

    /// True `percent` times in a hundred.
    fn chance(&mut self, percent: u64) -> bool {
        self.below(100) < percent
    }

    /// One of `items`.
    fn pick<T: Copy>(&mut self, items: &[T]) -> T {
        items[self.index(items.len())]
    }
}

/// What a sequence holds: the generator, the handles it has seen (each with the agent that
/// made it, 0 or 1), the roots, realms and scripts it keeps, and its transcript. It lives in
/// the host of the agent that runs, so that host functions and setup closures reach it.
#[derive(Default)]
struct Pool {
    rng: Rng,
    values: Vec<(usize, Value)>,
    realm_refs: Vec<(usize, RealmRef)>,
    script_refs: Vec<(usize, ScriptRef)>,
    roots: Vec<(usize, Root)>,
    realms: Vec<(usize, Realm)>,
    scripts: Vec<(usize, Script)>,
    log: Vec<String>,
}

/// Keep `item` in `list`, replacing a random one when the list is full.
fn keep<T>(rng: &mut Rng, list: &mut Vec<T>, item: T) {
    if list.len() < KEEP {
        list.push(item);
    } else {
        let i = rng.index(list.len());
        list[i] = item;
    }
}

/// The host of a fuzzed agent: its index, the pool while it runs, how deep its host functions
/// are, and what the sequence checks it against.
#[derive(Default)]
struct FuzzHost {
    index: usize,
    pool: Pool,
    depth: u32,
    /// A host function or a setup closure of the running entry returned a stale or foreign
    /// handle, or a constructor answer that is not an object, while the run was live: the entry
    /// must fault the agent (decision 6).
    expect_fault: bool,
    /// The agent holds host data that lists a stale or foreign handle: a later collection may
    /// fault it (`HostData::trace`).
    poisoned: bool,
}

/// Whether `$v` is a handle that reaches no record of the agent of `$cx`: stale or foreign.
macro_rules! bad {
    ($cx:expr, $v:expr) => {
        match $v {
            Value::Object(o) => $cx.brand(o).is_err(),
            Value::String(s) => $cx.to_utf16(s).is_err(),
            Value::Symbol(_) | Value::BigInt(_) => $cx.root($v).is_err(),
            _ => false,
        }
    };
}

impl FuzzHost {
    /// Write a hook call into the transcript.
    fn hook(&mut self, name: &str) {
        self.pool.log.push(format!("hook {name}"));
    }
}

impl Host for FuzzHost {
    fn host_ensure_can_compile_strings(
        cx: &mut Cx<'_, Self>,
        _request: &CompileStrings<'_>,
    ) -> Completion<()> {
        cx.host_mut().hook("host_ensure_can_compile_strings");
        Ok(())
    }
    fn host_get_code_for_eval(cx: &mut HookCx<'_, Self>, _arg: ObjectRef) -> Option<StringRef> {
        cx.host_mut().hook("host_get_code_for_eval");
        None
    }
    fn host_ensure_can_add_private_element(
        cx: &mut HookCx<'_, Self>,
        _obj: ObjectRef,
    ) -> Completion<()> {
        cx.host_mut().hook("host_ensure_can_add_private_element");
        Ok(())
    }
    fn host_has_source_text_available(cx: &mut HookCx<'_, Self>, _func: ObjectRef) -> bool {
        cx.host_mut().hook("host_has_source_text_available");
        true
    }
    fn host_make_job_callback(
        cx: &mut HookCx<'_, Self>,
        _callback: ObjectRef,
    ) -> Option<swell::HostDefined> {
        cx.host_mut().hook("host_make_job_callback");
        None
    }
    fn host_call_job_callback(
        cx: &mut Cx<'_, Self>,
        callback: ObjectRef,
        _host_defined: Option<&swell::HostDefined>,
        this: Value,
        args: &[Value],
    ) -> Completion<Value> {
        cx.host_mut().hook("host_call_job_callback");
        cx.call(Value::Object(callback), this, args)
            .unwrap_or(Err(Abrupt::Terminated(Termination::EngineFault)))
    }
    fn host_enqueue_promise_job(cx: &mut HookCx<'_, Self>, _realm: Option<RealmRef>) {
        cx.host_mut().hook("host_enqueue_promise_job");
    }
    fn host_prepare_to_run_job(cx: &mut HookCx<'_, Self>, _realm: RealmRef) {
        cx.host_mut().hook("host_prepare_to_run_job");
    }
    fn host_clean_up_after_job(
        cx: &mut HookCx<'_, Self>,
        _realm: Option<RealmRef>,
        _result: Result<(), Value>,
    ) {
        cx.host_mut().hook("host_clean_up_after_job");
    }
    fn host_enqueue_generic_job(cx: &mut HookCx<'_, Self>, _job: Job) {
        cx.host_mut().hook("host_enqueue_generic_job");
    }
    fn host_enqueue_timeout_job(cx: &mut HookCx<'_, Self>, _job: Job, _milliseconds: f64) {
        cx.host_mut().hook("host_enqueue_timeout_job");
    }
    fn host_enqueue_finalization_registry_cleanup_job(cx: &mut HookCx<'_, Self>, _job: Job) {
        cx.host_mut()
            .hook("host_enqueue_finalization_registry_cleanup_job");
    }
    fn host_promise_rejection_tracker(
        cx: &mut HookCx<'_, Self>,
        _promise: ObjectRef,
        _operation: RejectionOperation,
    ) {
        cx.host_mut().hook("host_promise_rejection_tracker");
    }
    fn host_load_imported_module(cx: &mut HookCx<'_, Self>, _request: LoadRequest) {
        cx.host_mut().hook("host_load_imported_module");
    }
    fn host_get_import_meta_properties(
        cx: &mut HookCx<'_, Self>,
        _module: ModuleRef,
    ) -> Completion<Vec<(PropertyKey, Value)>> {
        cx.host_mut().hook("host_get_import_meta_properties");
        Ok(Vec::new())
    }
    fn host_finalize_import_meta(
        cx: &mut HookCx<'_, Self>,
        _import_meta: ObjectRef,
        _module: ModuleRef,
    ) {
        cx.host_mut().hook("host_finalize_import_meta");
    }
    fn host_resize_array_buffer(
        cx: &mut HookCx<'_, Self>,
        _buffer: ObjectRef,
        _new_byte_length: u64,
    ) -> Completion<Handled> {
        cx.host_mut().hook("host_resize_array_buffer");
        Ok(Handled::Unhandled)
    }
    fn host_grow_shared_array_buffer(
        cx: &mut HookCx<'_, Self>,
        _buffer: ObjectRef,
        _new_byte_length: u64,
    ) -> Completion<Handled> {
        cx.host_mut().hook("host_grow_shared_array_buffer");
        Ok(Handled::Unhandled)
    }
    fn host_system_utc_epoch_nanoseconds(cx: &mut HookCx<'_, Self>, _global: ObjectRef) -> i128 {
        cx.host_mut().hook("host_system_utc_epoch_nanoseconds");
        0
    }
    fn system_time_zone_identifier(cx: &mut HookCx<'_, Self>) -> String {
        cx.host_mut().hook("system_time_zone_identifier");
        "UTC".to_string()
    }
    fn get_named_time_zone_offset_nanoseconds(
        cx: &mut HookCx<'_, Self>,
        _id: &str,
        _epoch_ns: i128,
    ) -> i128 {
        cx.host_mut().hook("get_named_time_zone_offset_nanoseconds");
        0
    }
    fn get_named_time_zone_epoch_nanoseconds(
        cx: &mut HookCx<'_, Self>,
        _id: &str,
        _local: IsoDateTime,
    ) -> Vec<i128> {
        cx.host_mut().hook("get_named_time_zone_epoch_nanoseconds");
        Vec::new()
    }
    fn available_named_time_zone_identifiers(cx: &mut HookCx<'_, Self>) -> Vec<TimeZoneIdentifier> {
        cx.host_mut().hook("available_named_time_zone_identifiers");
        Vec::new()
    }
}

/// Host data that holds the values it was made with and lists them to the collector.
struct Holds(Vec<Value>);

impl HostData for Holds {
    fn trace(&self, tracer: &mut Tracer<'_>) {
        for &value in &self.0 {
            tracer.value(value);
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Arguments.
// ---------------------------------------------------------------------------------------------

/// A number of the kinds conversions treat apart.
fn number(rng: &mut Rng) -> f64 {
    rng.pick(&[
        0.0,
        -0.0,
        1.0,
        -1.0,
        0.5,
        2.0,
        7.0,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        1e21,
        4_294_967_295.0,
        4_294_967_296.0,
        9_007_199_254_740_993.0,
        f64::MIN_POSITIVE,
        f64::MAX,
    ])
}

/// A value: one the sequence has seen (stale, released or of the other agent among them), or a
/// primitive.
fn value(pool: &mut Pool) -> (usize, Value) {
    let seen = pool.values.len();
    match pool.rng.below(10) {
        0..=4 if seen > 0 => {
            let i = pool.rng.index(seen);
            pool.values[i]
        }
        5 => (2, Value::Undefined),
        6 => (2, Value::Null),
        7 => (2, Value::Boolean(pool.rng.chance(50))),
        _ => (2, Value::Number(number(&mut pool.rng))),
    }
}

/// A seen value that is an object, if any.
fn object(pool: &mut Pool) -> Option<(usize, ObjectRef)> {
    let objects: Vec<(usize, ObjectRef)> = pool
        .values
        .iter()
        .filter_map(|&(origin, value)| match value {
            Value::Object(object) => Some((origin, object)),
            _ => None,
        })
        .collect();
    (!objects.is_empty()).then(|| objects[pool.rng.index(objects.len())])
}

/// A seen value that is a String, if any.
fn string(pool: &mut Pool) -> Option<(usize, StringRef)> {
    let strings: Vec<(usize, StringRef)> = pool
        .values
        .iter()
        .filter_map(|&(origin, value)| match value {
            Value::String(string) => Some((origin, string)),
            _ => None,
        })
        .collect();
    (!strings.is_empty()).then(|| strings[pool.rng.index(strings.len())])
}

/// A property key: a seen String or Symbol, if any.
fn key(pool: &mut Pool) -> Option<(usize, PropertyKey)> {
    let keys: Vec<(usize, PropertyKey)> = pool
        .values
        .iter()
        .filter_map(|&(origin, value)| match value {
            Value::String(string) => Some((origin, PropertyKey::String(string))),
            Value::Symbol(symbol) => Some((origin, PropertyKey::Symbol(symbol))),
            _ => None,
        })
        .collect();
    (!keys.is_empty()).then(|| keys[pool.rng.index(keys.len())])
}

/// Text for a String or a key: short names, digits that are array indices, and odd code points.
fn text(rng: &mut Rng) -> String {
    rng.pick(&[
        "",
        "a",
        "b",
        "x",
        "length",
        "0",
        "1",
        "4294967295",
        "4294967294",
        "-0",
        "__proto__",
        "constructor",
        "prototype",
        "toString",
        "valueOf",
        "toJSON",
        "name",
        "message",
        "\u{0}",
        "\u{FFFF}",
        "\u{1F600}",
        "caf\u{E9}",
    ])
    .to_string()
}

/// UTF-16 code units with lone surrogates among them.
fn units(rng: &mut Rng) -> Vec<u16> {
    let length = rng.below(6);
    (0..length)
        .map(|_| rng.pick(&[0x61_u16, 0xD800, 0xDC00, 0xD83D, 0xDE00, 0x0, 0xFFFF, 0x30]))
        .collect()
}

/// A property descriptor: data, accessor, generic, empty, or both kinds at once.
fn descriptor(pool: &mut Pool) -> PropertyDescriptor {
    let flag = |rng: &mut Rng| match rng.below(3) {
        0 => None,
        1 => Some(false),
        _ => Some(true),
    };
    let field = |pool: &mut Pool| {
        if pool.rng.chance(50) {
            Some(value(pool).1)
        } else {
            None
        }
    };
    PropertyDescriptor {
        value: field(pool),
        writable: flag(&mut pool.rng),
        get: field(pool),
        set: field(pool),
        enumerable: flag(&mut pool.rng),
        configurable: flag(&mut pool.rng),
    }
}

// ---------------------------------------------------------------------------------------------
// Scripts.
// ---------------------------------------------------------------------------------------------

/// What a script of the grammar starts with: the variables of [`NAMES`], so that a script
/// reaches more than its first ReferenceError.
const PROLOGUE: &str = "var a = 1, b = 'b', c = {}, o = { x: 1 }, arr = [1, 2], s = 's', i = 0, \
    g = function (x) { return x; }; ";

/// The names scripts use: their own variables and functions, the host functions of each realm,
/// and the globals of slice 1.
const NAMES: &[&str] = &[
    "a",
    "b",
    "c",
    "o",
    "arr",
    "f",
    "g",
    "i",
    "s",
    "h0",
    "h1",
    "h2",
    "this",
    "Object",
    "Array",
    "JSON",
    "Math",
    "String",
    "Error",
    "TypeError",
    "RangeError",
    "undefined",
    "NaN",
    "Infinity",
];

/// A literal.
fn literal(rng: &mut Rng) -> String {
    rng.pick(&[
        "0",
        "1",
        "-1",
        "0.5",
        "1e21",
        "0x10",
        "077",
        "\"\"",
        "\"str\"",
        "'\\u0041\\x42'",
        "\"\\uD800\"",
        "true",
        "false",
        "null",
        "undefined",
        "NaN",
        "/a+/g",
    ])
    .to_string()
}

/// An expression nested at most `depth` levels.
fn expression(rng: &mut Rng, depth: u32) -> String {
    if depth == 0 || rng.chance(25) {
        return if rng.chance(50) {
            literal(rng)
        } else {
            rng.pick(NAMES).to_string()
        };
    }
    let d = depth - 1;
    match rng.below(26) {
        0 => format!(
            "({} {} {})",
            expression(rng, d),
            rng.pick(&[
                "+",
                "-",
                "*",
                "/",
                "%",
                "<",
                ">=",
                "==",
                "===",
                "!=",
                "&&",
                "||",
                "&",
                "|",
                "^",
                "<<",
                ">>",
                ">>>",
                "in",
                "instanceof"
            ]),
            expression(rng, d)
        ),
        1 => format!(
            "{}{}",
            rng.pick(&["!", "-", "+", "~", "typeof ", "void ", "delete "]),
            expression(rng, d)
        ),
        2 => format!(
            "({} ? {} : {})",
            expression(rng, d),
            expression(rng, d),
            expression(rng, d)
        ),
        3 => format!(
            "{}.{}",
            expression(rng, d),
            rng.pick(&[
                "x",
                "length",
                "prototype",
                "constructor",
                "toString",
                "call",
                "bind",
                "map",
                "push",
                "join",
                "a"
            ])
        ),
        4 => format!("{}[{}]", expression(rng, d), expression(rng, d)),
        5 => format!("{}({})", expression(rng, d), arguments(rng, d)),
        6 => format!(
            "new {}({})",
            rng.pick(&[
                "Object",
                "Array",
                "Error",
                "TypeError",
                "f",
                "g",
                "o",
                "String"
            ]),
            arguments(rng, d)
        ),
        7 => format!("[{}]", arguments(rng, d)),
        8 => format!(
            "{{ x: {}, \"y\": {}, 1: {}, get z() {{ return {}; }} }}",
            expression(rng, d),
            expression(rng, d),
            expression(rng, d),
            expression(rng, d)
        ),
        9 => format!(
            "function ({}) {{ {} }}",
            rng.pick(&["", "a", "a, b"]),
            statements(rng, d, 2)
        ),
        10 => format!(
            "({} = {})",
            rng.pick(&["a", "b", "c", "o.x", "arr[0]", "s"]),
            expression(rng, d)
        ),
        11 => format!(
            "{} {} {}",
            rng.pick(&["a", "b", "i"]),
            rng.pick(&["+=", "-=", "*=", "|=", "<<="]),
            expression(rng, d)
        ),
        12 => format!(
            "{}{}",
            rng.pick(&["a", "i", "o.x"]),
            rng.pick(&["++", "--"])
        ),
        13 => format!("({}, {})", expression(rng, d), expression(rng, d)),
        14 => format!(
            "Object.defineProperty({}, {}, {{ value: {}, writable: {}, get: {} }})",
            expression(rng, d),
            expression(rng, d),
            expression(rng, d),
            rng.pick(&["true", "false"]),
            expression(rng, d)
        ),
        15 => format!(
            "Object.getOwnPropertyDescriptor({}, {})",
            expression(rng, d),
            expression(rng, d)
        ),
        16 => format!("Object.getOwnPropertyNames({})", expression(rng, d)),
        17 => format!("Object.getPrototypeOf({})", expression(rng, d)),
        18 => format!(
            "{}.hasOwnProperty({})",
            expression(rng, d),
            expression(rng, d)
        ),
        19 => format!(
            "JSON.stringify({}, {}, {})",
            expression(rng, d),
            expression(rng, d),
            expression(rng, d)
        ),
        20 => format!("Math.max({})", arguments(rng, d)),
        21 => format!("[{}].map({})", arguments(rng, d), expression(rng, d)),
        22 => format!("{}.call({})", expression(rng, d), arguments(rng, d)),
        23 => format!("{}.bind({})", expression(rng, d), arguments(rng, d)),
        24 => format!(
            "String({}).indexOf({})",
            expression(rng, d),
            expression(rng, d)
        ),
        _ => format!("{}({})", rng.pick(&["h0", "h1", "h2"]), arguments(rng, d)),
    }
}

/// Up to three arguments.
fn arguments(rng: &mut Rng, depth: u32) -> String {
    let count = rng.below(4);
    (0..count)
        .map(|_| expression(rng, depth))
        .collect::<Vec<_>>()
        .join(", ")
}

/// A statement nested at most `depth` levels.
fn statement(rng: &mut Rng, depth: u32) -> String {
    let d = depth.saturating_sub(1);
    if depth == 0 {
        return format!("({});", expression(rng, 1));
    }
    match rng.below(21) {
        0 => format!(
            "var {} = {};",
            rng.pick(&["a", "b", "c", "o", "arr", "s", "i"]),
            expression(rng, d)
        ),
        1 => format!(
            "if ({}) {{ {} }} else {{ {} }}",
            expression(rng, d),
            statements(rng, d, 2),
            statements(rng, d, 2)
        ),
        2 => format!(
            "for (var i = 0; i < {}; i++) {{ {} }}",
            rng.pick(&["3", "10", "100", "a", "Infinity"]),
            statements(rng, d, 2)
        ),
        3 => format!(
            "while ({}) {{ {} }}",
            rng.pick(&["true", "i++ < 5", "a", "false"]),
            statements(rng, d, 2)
        ),
        4 => format!(
            "do {{ {} }} while ({});",
            statements(rng, d, 2),
            rng.pick(&["false", "i-- > 0", "true"])
        ),
        5 => format!(
            "for (var k in {}) {{ {} }}",
            expression(rng, d),
            statements(rng, d, 2)
        ),
        6 => format!(
            "try {{ {} }} catch (e) {{ {} }} finally {{ {} }}",
            statements(rng, d, 2),
            statements(rng, d, 1),
            statements(rng, d, 1)
        ),
        7 => format!("throw {};", expression(rng, d)),
        8 => format!(
            "var {} = function ({}) {{ {} return {}; }};",
            rng.pick(&["f", "g"]),
            rng.pick(&["", "a", "a, b"]),
            statements(rng, d, 2),
            expression(rng, d)
        ),
        9 => format!(
            "switch ({}) {{ case 1: {} break; case {}: {} default: {} }}",
            expression(rng, d),
            statements(rng, d, 1),
            expression(rng, d),
            statements(rng, d, 1),
            statements(rng, d, 1)
        ),
        10 => format!("for (;;) {{ {} break; }}", statements(rng, d, 2)),
        11 => format!("{{ {} }}", statements(rng, d, 3)),
        12 => format!(
            "arr = []; for (var i = 0; i < {}; i++) arr.push({});",
            rng.pick(&["5", "1000", "100000"]),
            expression(rng, d)
        ),
        13 => "s = 'x'; for (var i = 0; i < 40; i++) s = s + s;".to_string(),
        14 => "function r(n) { return n <= 0 ? 0 : 1 + r(n - 1); } r(100);".to_string(),
        15 => format!(
            "{} = {};",
            rng.pick(&["undeclared", "NaN", "undefined", "this.q"]),
            expression(rng, d)
        ),
        16 => format!(
            "var o = {{ get x() {{ {} return 1; }}, valueOf: function () {{ {} return 2; }}, toJSON: g }};",
            statements(rng, d, 1),
            statements(rng, d, 1)
        ),
        17 | 18 => format!("{}({});", rng.pick(&["h0", "h1", "h2"]), arguments(rng, d)),
        19 => format!(
            "h1({}, {});",
            rng.pick(&["f", "g", "function (a) { return h0(a); }", "h1", "h2"]),
            arguments(rng, d)
        ),
        _ => format!("({});", expression(rng, d)),
    }
}

/// Up to `most` statements.
fn statements(rng: &mut Rng, depth: u32, most: u64) -> String {
    let count = rng.below(most + 1);
    (0..count)
        .map(|_| statement(rng, depth))
        .collect::<Vec<_>>()
        .join(" ")
}

/// The source of a script: one of slice 1's grammar, often strict, or a broken one (random
/// tokens, deep nesting, odd bytes).
fn source(rng: &mut Rng) -> String {
    match rng.below(12) {
        0 => {
            let tokens = [
                "(", ")", "{", "}", "[", "]", ";", "var", "function", "=", "+", "1", "a", ",", ".",
                "\"", "'", "/", "*", "if", "return", "new",
            ];
            (0..rng.below(20))
                .map(|_| rng.pick(&tokens))
                .collect::<Vec<_>>()
                .join(" ")
        }
        1 => {
            let depth = usize::try_from(rng.below(80)).unwrap_or(0);
            let (open, close) = rng.pick(&[
                ("(", ")"),
                ("[", "]"),
                ("{", "}"),
                ("!", ""),
                ("-", ""),
                ("function(){", "}"),
            ]);
            format!("{}0{}", open.repeat(depth), close.repeat(depth))
        }
        2 => "\"use strict\"; var eval = 1; with (a) {} 08; delete a;".to_string(),
        _ => {
            let strict = if rng.chance(30) { "'use strict'; " } else { "" };
            let declared = if rng.chance(50) {
                format!(
                    "function f(a, b) {{ {} return a; }} ",
                    statements(rng, 2, 2)
                )
            } else {
                String::new()
            };
            // Half the scripts call a host function first, so that their answers are checked
            // often (decision 6).
            let lead = if rng.chance(50) {
                let new = if rng.chance(30) { "new " } else { "" };
                format!(
                    "{new}{}({}); ",
                    rng.pick(&["h0", "h1", "h2"]),
                    arguments(rng, 1)
                )
            } else {
                String::new()
            };
            format!(
                "{strict}{PROLOGUE}{declared}{lead}{}",
                statements(rng, 3, 4)
            )
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Answers as text, and the checks on them.
// ---------------------------------------------------------------------------------------------

/// The shape of an answer that takes a handle.
fn shape<T: std::fmt::Debug>(answer: &Result<T, HostError>) -> String {
    match answer {
        Ok(value) => format!("Ok({value:?})"),
        Err(error) => format!("Err({error:?})"),
    }
}

/// An answer of a call whose one handle came from the other agent: it must be
/// `Err(ForeignHandle)`, or `Err(Faulted)` for a faulted agent.
fn foreign_answer<T: std::fmt::Debug>(name: &str, answer: &Result<T, HostError>) {
    assert!(
        matches!(answer, Err(HostError::ForeignHandle | HostError::Faulted)),
        "{name} of a handle of the other agent answered {answer:?}"
    );
}

/// Whether a value made by agent `origin` is foreign to agent `me`: primitives (origin 2) are
/// nobody's.
fn is_foreign(origin: usize, me: usize) -> bool {
    origin < 2 && origin != me
}

/// Keep the value of an answer, made by agent `me`.
fn keep_value(pool: &mut Pool, me: usize, value: Value) {
    if matches!(
        value,
        Value::Object(_) | Value::String(_) | Value::Symbol(_) | Value::BigInt(_)
    ) {
        keep(&mut pool.rng, &mut pool.values, (me, value));
    }
}

/// The value of a completion, kept.
fn keep_completion(pool: &mut Pool, me: usize, completion: &Completion<Value>) {
    if let Ok(value) | Err(Abrupt::Throw(value)) = completion {
        keep_value(pool, me, *value);
    }
}

// ---------------------------------------------------------------------------------------------
// The operations of `Cx` and `HookCx`.
// ---------------------------------------------------------------------------------------------

/// The pool of the running host.
macro_rules! pool {
    ($cx:expr) => {
        $cx.host_mut().pool
    };
}

/// One of the operations `Cx` and `HookCx` share, chosen by `$choice` (0 to 12), as text.
macro_rules! scoped_op {
    ($cx:expr, $choice:expr) => {{
        let me = $cx.host().index;
        match $choice {
            0 => {
                let (origin, v) = value(&mut pool!($cx));
                let answer = $cx.root(v);
                let line = format!("root({v:?}) -> {}", shape(&answer));
                if is_foreign(origin, me) {
                    foreign_answer("root", &answer);
                }
                if let Ok(root) = answer {
                    let pool = &mut pool!($cx);
                    keep(&mut pool.rng, &mut pool.roots, (me, root));
                }
                line
            }
            1 | 2 => {
                let count = pool!($cx).roots.len();
                if count == 0 {
                    return "get(no root)".to_string();
                }
                let i = pool!($cx).rng.index(count);
                let (origin, root) = pool!($cx).roots.swap_remove(i);
                let line = if $choice == 1 {
                    let answer = $cx.get(&root);
                    if is_foreign(origin, me) {
                        foreign_answer("get", &answer);
                    }
                    if let Ok(v) = answer {
                        keep_value(&mut pool!($cx), me, v);
                    }
                    format!("get({root:?}) -> {}", shape(&answer))
                } else {
                    let answer = $cx.duplicate(&root);
                    if is_foreign(origin, me) {
                        foreign_answer("duplicate", &answer);
                    }
                    let line = format!("duplicate({root:?}) -> {}", shape(&answer));
                    if let Ok(second) = answer {
                        let pool = &mut pool!($cx);
                        keep(&mut pool.rng, &mut pool.roots, (me, second));
                    }
                    line
                };
                pool!($cx).roots.push((origin, root));
                line
            }
            3 => {
                let t = text(&mut pool!($cx).rng);
                let answer = $cx.new_string(&t);
                if let Ok(s) = answer {
                    keep_value(&mut pool!($cx), me, Value::String(s));
                }
                format!("new_string({t:?}) -> {answer:?}")
            }
            4 => {
                let u = units(&mut pool!($cx).rng);
                let answer = $cx.new_string_utf16(&u);
                if let Ok(s) = answer {
                    keep_value(&mut pool!($cx), me, Value::String(s));
                }
                format!("new_string_utf16({u:?}) -> {answer:?}")
            }
            5 => match string(&mut pool!($cx)) {
                Some((origin, s)) => {
                    let utf16 = $cx.to_utf16(s);
                    let lossy = $cx.to_rust_string_lossy(s);
                    if is_foreign(origin, me) {
                        foreign_answer("to_utf16", &utf16);
                        foreign_answer("to_rust_string_lossy", &lossy);
                    }
                    if let (Ok(units), Ok(text)) = (&utf16, &lossy) {
                        assert_eq!(
                            text,
                            &String::from_utf16_lossy(units),
                            "to_rust_string_lossy agrees with to_utf16"
                        );
                    }
                    format!("to_utf16({s:?}) -> {} {}", shape(&utf16), shape(&lossy))
                }
                None => "to_utf16(no string)".to_string(),
            },
            6 => {
                // An object, the `%Object.prototype%` of a realm seen (of the other agent, or
                // one `create_realm` did not make, among them), or null.
                let (origin, prototype) = match pool!($cx).rng.below(5) {
                    0..=2 => match object(&mut pool!($cx)) {
                        Some((origin, p)) => (origin, Prototype::Object(p)),
                        None => (2, Prototype::Null),
                    },
                    3 => {
                        let count = pool!($cx).realm_refs.len();
                        if count == 0 {
                            (2, Prototype::Null)
                        } else {
                            let i = pool!($cx).rng.index(count);
                            let (origin, r) = pool!($cx).realm_refs[i];
                            (origin, Prototype::ObjectPrototypeOf(r))
                        }
                    }
                    _ => (2, Prototype::Null),
                };
                let held: Vec<Value> = (0..pool!($cx).rng.below(3))
                    .map(|_| value(&mut pool!($cx)).1)
                    .collect();
                let brand = Brand(u32::try_from(pool!($cx).rng.below(3)).unwrap_or(0));
                let data: Option<Box<dyn HostData>> = if pool!($cx).rng.chance(50) {
                    if held.iter().any(|&v| bad!($cx, v)) {
                        $cx.host_mut().poisoned = true;
                    }
                    Some(Box::new(Holds(held.clone())))
                } else {
                    None
                };
                let answer = $cx.new_host_object(HostObjectSpec {
                    prototype,
                    brand,
                    data,
                    error_kind: None,
                });
                if is_foreign(origin, me) {
                    foreign_answer("new_host_object", &answer);
                }
                if let Ok(Ok(made)) = answer {
                    keep_value(&mut pool!($cx), me, Value::Object(made));
                }
                format!(
                    "new_host_object({prototype:?}, {brand:?}, {held:?}) -> {}",
                    shape(&answer)
                )
            }
            7 => match object(&mut pool!($cx)) {
                Some((origin, o)) => {
                    let brand = $cx.brand(o);
                    let data = $cx
                        .host_data::<Holds>(o, Brand(1))
                        .map(|d| d.map(|d| d.0.len()));
                    let data_mut = $cx
                        .host_data_mut::<Holds>(o, Brand(0))
                        .map(|d| d.map(|d| d.0.len()));
                    if is_foreign(origin, me) {
                        foreign_answer("brand", &brand);
                        foreign_answer("host_data", &data);
                        foreign_answer("host_data_mut", &data_mut);
                    }
                    format!(
                        "brand({o:?}) -> {} {} {}",
                        shape(&brand),
                        shape(&data),
                        shape(&data_mut)
                    )
                }
                None => "brand(no object)".to_string(),
            },
            8 => {
                let count = pool!($cx).realms.len();
                if count == 0 {
                    return "realm_ref(no realm)".to_string();
                }
                let i = pool!($cx).rng.index(count);
                let (origin, realm) = pool!($cx).realms.swap_remove(i);
                let answer = $cx.realm_ref(&realm);
                if is_foreign(origin, me) {
                    foreign_answer("realm_ref", &answer);
                }
                if let Ok(r) = answer {
                    let pool = &mut pool!($cx);
                    keep(&mut pool.rng, &mut pool.realm_refs, (me, r));
                }
                pool!($cx).realms.push((origin, realm));
                format!("realm_ref -> {}", shape(&answer))
            }
            9 => {
                let count = pool!($cx).realm_refs.len();
                if count == 0 {
                    return "new_error(no realm)".to_string();
                }
                let i = pool!($cx).rng.index(count);
                let (origin, r) = pool!($cx).realm_refs[i];
                let kind = pool!($cx).rng.pick(&ErrorKind::ALL);
                let message = text(&mut pool!($cx).rng);
                let answer = $cx.new_error(r, kind, &message);
                if is_foreign(origin, me) {
                    foreign_answer("new_error", &answer);
                }
                if let Ok(Ok(error)) = answer {
                    keep_value(&mut pool!($cx), me, Value::Object(error));
                }
                format!(
                    "new_error({r:?}, {kind:?}, {message:?}) -> {}",
                    shape(&answer)
                )
            }
            10 => {
                // Release a root the sequence holds: its handle goes stale at the next safe point.
                let count = pool!($cx).roots.len();
                if count == 0 {
                    return "release(no root)".to_string();
                }
                let i = pool!($cx).rng.index(count);
                let (origin, root) = pool!($cx).roots.swap_remove(i);
                let line = format!("release({root:?}) of {origin}");
                drop(root);
                line
            }
            _ => {
                let (origin, v) = value(&mut pool!($cx));
                let answer = $cx.root(v);
                if is_foreign(origin, me) {
                    foreign_answer("root", &answer);
                }
                format!("root-and-drop({v:?}) -> {}", shape(&answer))
            }
        }
    }};
}

/// One operation of a setup closure's `HookCx`.
fn hook_op(cx: &mut HookCx<'_, FuzzHost>) -> String {
    let choice = cx.host_mut().pool.rng.below(12);
    scoped_op!(cx, choice)
}

/// One operation of a `Cx`: the shared ones, and those that run code.
fn cx_op(cx: &mut Cx<'_, FuzzHost>) -> String {
    let me = cx.host().index;
    let choice = pool!(cx).rng.below(36);
    if choice < 12 {
        return scoped_op!(cx, choice);
    }
    match choice {
        34 => {
            // A host microtask that makes one operation of its own `Cx` when a checkpoint
            // runs it, in the current realm of this `Cx`.
            let answer = cx.queue_microtask(Box::new(|cx: &mut Cx<'_, FuzzHost>| {
                let line = cx_op(cx);
                pool!(cx).log.push(format!("  microtask {line}"));
            }));
            format!("cx.queue_microtask -> {}", shape(&answer))
        }
        35 => {
            let answer = cx.perform_microtask_checkpoint();
            format!("cx.perform_microtask_checkpoint -> {answer:?}")
        }
        12 => {
            let count = pool!(cx).realm_refs.len();
            if count == 0 {
                return "new_host_function(no realm)".to_string();
            }
            let i = pool!(cx).rng.index(count);
            let (origin, r) = pool!(cx).realm_refs[i];
            let which = pool!(cx).rng.below(3);
            let steps: swell::HostFn<FuzzHost> = [h0, h1, h2][usize::try_from(which).unwrap_or(0)];
            let constructor = pool!(cx).rng.chance(50);
            let answer = cx.new_host_function(r, "h", 1, steps, constructor, None);
            if is_foreign(origin, me) {
                foreign_answer("new_host_function", &answer);
            }
            if let Ok(Ok(f)) = answer {
                keep_value(&mut pool!(cx), me, Value::Object(f));
            }
            format!(
                "new_host_function({r:?}, h{which}, {constructor}) -> {}",
                shape(&answer)
            )
        }
        13..=15 => {
            let (fo, f) = value(&mut pool!(cx));
            let (to, this) = value(&mut pool!(cx));
            let args: Vec<(usize, Value)> = (0..pool!(cx).rng.below(3))
                .map(|_| value(&mut pool!(cx)))
                .collect();
            let values: Vec<Value> = args.iter().map(|&(_, v)| v).collect();
            let answer = cx.call(f, this, &values);
            if let Ok(completion) = &answer {
                keep_completion(&mut pool!(cx), me, completion);
            }
            // The function the one handle, of the other agent: `Err(ForeignHandle)`.
            if is_foreign(fo, me) && to == 2 && args.iter().all(|&(o, _)| o == 2) {
                foreign_answer("call", &answer);
            }
            format!("call({f:?}, {this:?}, {values:?}) -> {}", shape(&answer))
        }
        16 | 17 => {
            let (_, f) = value(&mut pool!(cx));
            let args: Vec<Value> = (0..pool!(cx).rng.below(3))
                .map(|_| value(&mut pool!(cx)).1)
                .collect();
            let new_target = if pool!(cx).rng.chance(30) {
                Some(value(&mut pool!(cx)).1)
            } else {
                None
            };
            let answer = cx.construct(f, &args, new_target);
            if let Ok(completion) = &answer {
                keep_completion(&mut pool!(cx), me, completion);
            }
            format!(
                "construct({f:?}, {args:?}, {new_target:?}) -> {}",
                shape(&answer)
            )
        }
        18 | 19 => {
            let (Some((_, o)), Some((_, k))) = (object(&mut pool!(cx)), key(&mut pool!(cx))) else {
                return "get_property(nothing)".to_string();
            };
            let answer = cx.get_property(o, k);
            if let Ok(completion) = &answer {
                keep_completion(&mut pool!(cx), me, completion);
            }
            format!("get_property({o:?}, {k:?}) -> {}", shape(&answer))
        }
        20 => {
            let (Some((_, o)), Some((_, k))) = (object(&mut pool!(cx)), key(&mut pool!(cx))) else {
                return "set(nothing)".to_string();
            };
            let (_, v) = value(&mut pool!(cx));
            let throw = pool!(cx).rng.chance(50);
            let answer = cx.set(o, k, v, throw);
            if let Ok(Err(Abrupt::Throw(thrown))) = &answer {
                keep_value(&mut pool!(cx), me, *thrown);
            }
            format!("set({o:?}, {k:?}, {v:?}, {throw}) -> {}", shape(&answer))
        }
        21 => {
            let (Some((_, o)), Some((_, k))) = (object(&mut pool!(cx)), key(&mut pool!(cx))) else {
                return "define(nothing)".to_string();
            };
            let d = descriptor(&mut pool!(cx));
            let answer = cx.define_property_or_throw(o, k, d);
            if let Ok(Err(Abrupt::Throw(thrown))) = &answer {
                keep_value(&mut pool!(cx), me, *thrown);
            }
            format!(
                "define_property_or_throw({o:?}, {k:?}, {d:?}) -> {}",
                shape(&answer)
            )
        }
        22 => {
            let (Some((_, o)), Some((_, k))) = (object(&mut pool!(cx)), key(&mut pool!(cx))) else {
                return "has(nothing)".to_string();
            };
            let has = cx.has_property(o, k);
            let delete = cx.delete_property_or_throw(o, k);
            format!(
                "has_property/delete({o:?}, {k:?}) -> {} {}",
                shape(&has),
                shape(&delete)
            )
        }
        23..=28 => {
            let (origin, v) = value(&mut pool!(cx));
            match choice {
                23 => {
                    let hint = pool!(cx).rng.pick(&[
                        None,
                        Some(PreferredType::String),
                        Some(PreferredType::Number),
                    ]);
                    let answer = cx.to_primitive(v, hint);
                    if is_foreign(origin, me) {
                        foreign_answer("to_primitive", &answer);
                    }
                    if let Ok(completion) = &answer {
                        keep_completion(&mut pool!(cx), me, completion);
                    }
                    format!("to_primitive({v:?}, {hint:?}) -> {}", shape(&answer))
                }
                24 => {
                    let answer = cx.to_number(v);
                    if is_foreign(origin, me) {
                        foreign_answer("to_number", &answer);
                    }
                    format!("to_number({v:?}) -> {}", shape(&answer))
                }
                25 => {
                    let answer = cx.to_numeric(v);
                    if is_foreign(origin, me) {
                        foreign_answer("to_numeric", &answer);
                    }
                    format!("to_numeric({v:?}) -> {}", shape(&answer))
                }
                26 => {
                    let answer = cx.to_string(v);
                    if is_foreign(origin, me) {
                        foreign_answer("to_string", &answer);
                    }
                    if let Ok(Ok(s)) = answer {
                        keep_value(&mut pool!(cx), me, Value::String(s));
                    }
                    format!("to_string({v:?}) -> {}", shape(&answer))
                }
                27 => {
                    let answer = cx.to_object(v);
                    if is_foreign(origin, me) {
                        foreign_answer("to_object", &answer);
                    }
                    if let Ok(Ok(o)) = answer {
                        keep_value(&mut pool!(cx), me, Value::Object(o));
                    }
                    format!("to_object({v:?}) -> {}", shape(&answer))
                }
                _ => {
                    let answer = cx.to_property_key(v);
                    if is_foreign(origin, me) {
                        foreign_answer("to_property_key", &answer);
                    }
                    if let Ok(Ok(PropertyKey::String(s))) = answer {
                        keep_value(&mut pool!(cx), me, Value::String(s));
                    }
                    format!("to_property_key({v:?}) -> {}", shape(&answer))
                }
            }
        }
        29 | 30 => {
            let count = pool!(cx).realm_refs.len();
            if count == 0 {
                return "cx.parse_script(no realm)".to_string();
            }
            let i = pool!(cx).rng.index(count);
            let (origin, r) = pool!(cx).realm_refs[i];
            let text = source(&mut pool!(cx).rng);
            let answer = cx.parse_script(r, Source::Utf8(&text), None);
            if is_foreign(origin, me) {
                foreign_answer("cx.parse_script", &answer);
            }
            let line = format!("cx.parse_script({r:?}, {text:?}) -> {}", shape(&answer));
            match answer {
                Ok(Parsed::Parsed(script)) => {
                    let pool = &mut pool!(cx);
                    keep(&mut pool.rng, &mut pool.script_refs, (me, script));
                }
                Ok(Parsed::Errors(errors)) => {
                    let pool = &mut pool!(cx);
                    for error in errors {
                        keep(&mut pool.rng, &mut pool.roots, (me, error));
                    }
                }
                _ => {}
            }
            line
        }
        _ => {
            let count = pool!(cx).script_refs.len();
            if count == 0 {
                return "cx.evaluate_script(no script)".to_string();
            }
            let i = pool!(cx).rng.index(count);
            let (origin, script) = pool!(cx).script_refs[i];
            let answer = cx.evaluate_script(script);
            if is_foreign(origin, me) {
                foreign_answer("cx.evaluate_script", &answer);
            }
            if let Ok(completion) = &answer {
                keep_completion(&mut pool!(cx), me, completion);
            }
            format!("cx.evaluate_script({script:?}) -> {}", shape(&answer))
        }
    }
}

/// Up to [`CX_CALLS`] operations of `cx`, written to the transcript.
fn cx_ops(cx: &mut Cx<'_, FuzzHost>) {
    let count = pool!(cx).rng.below(CX_CALLS + 1);
    for _ in 0..count {
        let line = cx_op(cx);
        pool!(cx).log.push(format!("  {line}"));
    }
}

/// What a host function of this test returns: mostly a primitive or an argument; sometimes a
/// value it has seen (stale or of the other agent ones among them, which fault the agent,
/// decision 6), a throw of one, or a termination.
fn host_answer(cx: &mut Cx<'_, FuzzHost>, call: &HostCall<'_>) -> Completion<Value> {
    match pool!(cx).rng.below(20) {
        0 => Err(Abrupt::Throw(value(&mut pool!(cx)).1)),
        1 => Ok(value(&mut pool!(cx)).1),
        2 => Err(Abrupt::Terminated(Termination::Interrupted)),
        3..=8 => Ok(call.args.first().copied().unwrap_or(Value::Undefined)),
        9..=10 => Ok(Value::Object(call.callee)),
        _ => Ok(Value::Number(number(&mut pool!(cx).rng))),
    }
}

/// Enter a host function: its call written down, and whether it may make operations.
fn enter_host(cx: &mut Cx<'_, FuzzHost>, name: &str, call: &HostCall<'_>) -> bool {
    let me = cx.host().index;
    for &arg in call.args {
        keep_value(&mut pool!(cx), me, arg);
    }
    keep_value(&mut pool!(cx), me, call.this);
    let line = format!(
        "  {name}(this {:?}, args {:?}, new_target {:?})",
        call.this, call.args, call.new_target
    );
    pool!(cx).log.push(line);
    cx.host().depth < HOST_DEPTH
}

/// What a host function returns, checked: an answer the engine must refuse in a live run is
/// written down as an expected fault.
fn checked(
    cx: &mut Cx<'_, FuzzHost>,
    call: &HostCall<'_>,
    answer: Completion<Value>,
) -> Completion<Value> {
    let live = matches!(cx.to_number(Value::Undefined), Ok(Ok(_)));
    let refused = match answer {
        Ok(value) => {
            bad!(cx, value) || (call.new_target.is_some() && !matches!(value, Value::Object(_)))
        }
        Err(Abrupt::Throw(value)) => bad!(cx, value),
        Err(Abrupt::Terminated(_)) => false,
    };
    if live && refused {
        cx.host_mut().expect_fault = true;
        pool!(cx).log.push("  expect fault".to_string());
    }
    answer
}

/// The host function `h0`.
fn h0(cx: &mut Cx<'_, FuzzHost>, call: HostCall<'_>) -> Completion<Value> {
    let answer = h0_steps(cx, &call);
    checked(cx, &call, answer)
}

/// The host function `h1`.
fn h1(cx: &mut Cx<'_, FuzzHost>, call: HostCall<'_>) -> Completion<Value> {
    let answer = h1_steps(cx, &call);
    checked(cx, &call, answer)
}

/// The host function `h2`.
fn h2(cx: &mut Cx<'_, FuzzHost>, call: HostCall<'_>) -> Completion<Value> {
    let answer = h2_steps(cx, &call);
    checked(cx, &call, answer)
}

/// The steps of `h0`: operations of `cx`, then an answer.
fn h0_steps(cx: &mut Cx<'_, FuzzHost>, call: &HostCall<'_>) -> Completion<Value> {
    if enter_host(cx, "h0", call) {
        cx.host_mut().depth += 1;
        cx_ops(cx);
        cx.host_mut().depth -= 1;
    }
    host_answer(cx, call)
}

/// The host function `h1`: calls its first argument with the rest through `cx.call`.
fn h1_steps(cx: &mut Cx<'_, FuzzHost>, call: &HostCall<'_>) -> Completion<Value> {
    if !enter_host(cx, "h1", call) {
        return host_answer(cx, call);
    }
    let (function, rest) = match call.args.split_first() {
        Some((&function, rest)) => (function, rest),
        None => (Value::Undefined, &[][..]),
    };
    cx.host_mut().depth += 1;
    let answer = cx.call(function, call.this, rest);
    cx.host_mut().depth -= 1;
    pool!(cx)
        .log
        .push(format!("  h1 call -> {}", shape(&answer)));
    match answer {
        Ok(completion) => completion,
        Err(_) => host_answer(cx, call),
    }
}

/// The host function `h2`: parses a source in a realm it has seen and evaluates it.
fn h2_steps(cx: &mut Cx<'_, FuzzHost>, call: &HostCall<'_>) -> Completion<Value> {
    if !enter_host(cx, "h2", call) {
        return host_answer(cx, call);
    }
    let count = pool!(cx).realm_refs.len();
    if count == 0 {
        return host_answer(cx, call);
    }
    let i = pool!(cx).rng.index(count);
    let (_, realm) = pool!(cx).realm_refs[i];
    let text = source(&mut pool!(cx).rng);
    cx.host_mut().depth += 1;
    let parsed = cx.parse_script(realm, Source::Utf8(&text), None);
    let answer = match parsed {
        Ok(Parsed::Parsed(script)) => cx.evaluate_script(script),
        Ok(Parsed::Terminated(termination)) => Ok(Err(Abrupt::Terminated(termination))),
        Ok(Parsed::Errors(_)) => Ok(Ok(Value::Null)),
        Err(error) => Err(error),
    };
    cx.host_mut().depth -= 1;
    pool!(cx)
        .log
        .push(format!("  h2 {text:?} -> {}", shape(&answer)));
    match answer {
        Ok(completion) => completion,
        Err(_) => host_answer(cx, call),
    }
}

// ---------------------------------------------------------------------------------------------
// The sequence.
// ---------------------------------------------------------------------------------------------

/// Two agents with one pool, which moves into the host of the agent that runs.
struct World {
    agents: [Agent<FuzzHost>; 2],
    limits: Limits,
    pool: Pool,
}

impl World {
    /// Run `f` on agent `a` with the pool in its host.
    fn on<R>(&mut self, a: usize, f: impl FnOnce(&mut Agent<FuzzHost>) -> R) -> R {
        let agent = &mut self.agents[a];
        agent.host_mut().pool = std::mem::take(&mut self.pool);
        let result = f(agent);
        self.pool = std::mem::take(&mut agent.host_mut().pool);
        result
    }

    /// Take out a realm, a root or a script, to give to an entry point of `Agent`.
    fn take_realm(&mut self) -> Option<(usize, Realm)> {
        let count = self.pool.realms.len();
        (count > 0).then(|| {
            let i = self.pool.rng.index(count);
            self.pool.realms.swap_remove(i)
        })
    }

    fn take_root(&mut self) -> Option<(usize, Root)> {
        let count = self.pool.roots.len();
        (count > 0).then(|| {
            let i = self.pool.rng.index(count);
            self.pool.roots.swap_remove(i)
        })
    }

    fn take_script(&mut self) -> Option<(usize, Script)> {
        let count = self.pool.scripts.len();
        (count > 0).then(|| {
            let i = self.pool.rng.index(count);
            self.pool.scripts.swap_remove(i)
        })
    }

    fn log(&mut self, line: String) {
        self.pool.log.push(line);
    }
}

/// Random limits, small, each sometimes 0.
fn random_limits(rng: &mut Rng) -> Limits {
    /// One of `small` a quarter of the time, else one of `room`.
    fn either<T: Copy>(rng: &mut Rng, small: &[T], room: &[T]) -> T {
        if rng.chance(25) {
            rng.pick(small)
        } else {
            rng.pick(room)
        }
    }
    Limits {
        steps_per_entry: either(rng, &[0, 1, 10, 100], &[1_000, 5_000, 20_000]),
        memory_bytes: either(
            rng,
            &[0, 1_000, 40_000, 70_000],
            &[200_000, 500_000, 2_000_000],
        ),
        frame_depth: either(rng, &[0, 1, 3], &[16, 64]),
        reentry_depth: either(rng, &[0, 1], &[2, 4, 8]),
        parser_nesting: either(rng, &[0, 1, 4, 16], &[64, 128]),
        string_length: either(rng, &[0, 1, 16], &[1_024, 1 << 20]),
        bigint_bits: rng.pick(&[0, 64]),
    }
}

/// Define the host functions `h0`, `h1` and `h2` as globals of `realm`.
fn define_host_functions(agent: &mut Agent<FuzzHost>, realm: &Realm) -> String {
    let global = match agent.global_object(realm) {
        Ok(global) => global,
        Err(error) => return format!("global_object -> Err({error:?})"),
    };
    let answer = agent.with_cx(realm, |cx| -> Completion<()> {
        let Ok(Value::Object(global)) = cx.get(&global) else {
            return Ok(());
        };
        let Ok(realm_ref) = cx.realm_ref(realm) else {
            return Ok(());
        };
        let me = cx.host().index;
        let pool = &mut pool!(cx);
        keep(&mut pool.rng, &mut pool.realm_refs, (me, realm_ref));
        keep_value(&mut pool!(cx), me, Value::Object(global));
        for (name, steps) in [
            ("h0", h0 as swell::HostFn<FuzzHost>),
            ("h1", h1),
            ("h2", h2),
        ] {
            let Ok(f) = cx.new_host_function(realm_ref, name, 1, steps, true, None) else {
                return Ok(());
            };
            let f = f?;
            let key = PropertyKey::String(cx.new_string(name)?);
            if let Ok(set) = cx.set(global, key, Value::Object(f), false) {
                set?;
            }
        }
        Ok(())
    });
    format!("define h0 h1 h2 -> {answer:?}")
}

/// `create_realm`, with a setup closure or without, and the host functions defined in it.
fn op_create_realm(world: &mut World, a: usize) -> String {
    let with_setup = world.pool.rng.chance(50);
    let host_defined = world.pool.rng.chance(30);
    let (line, realm) = world.on(a, |agent| {
        let me = agent.host().index;
        let mut setup =
            |cx: &mut HookCx<'_, FuzzHost>, realm: RealmRef| -> Completion<GlobalObjects> {
                let pool = &mut pool!(cx);
                keep(&mut pool.rng, &mut pool.realm_refs, (me, realm));
                for _ in 0..pool!(cx).rng.below(4) {
                    let line = hook_op(cx);
                    pool!(cx).log.push(format!("  setup {line}"));
                }
                let prototype = if pool!(cx).rng.chance(70) {
                    Prototype::ObjectPrototypeOf(realm)
                } else {
                    Prototype::Null
                };
                let made = cx.new_host_object(HostObjectSpec {
                    prototype,
                    brand: Brand(7),
                    data: None,
                    error_kind: None,
                });
                let made = match made {
                    Ok(made) => made?,
                    Err(error) => {
                        pool!(cx)
                            .log
                            .push(format!("  setup new_host_object -> Err({error:?})"));
                        return Err(Abrupt::Throw(Value::Undefined));
                    }
                };
                let this = if pool!(cx).rng.chance(30) {
                    object(&mut pool!(cx)).map(|(_, o)| o)
                } else {
                    None
                };
                let answer = match pool!(cx).rng.below(12) {
                    0 => Err(Abrupt::Throw(value(&mut pool!(cx)).1)),
                    1 => Err(Abrupt::Terminated(Termination::StepBudget)),
                    2 => Ok(GlobalObjects {
                        global: object(&mut pool!(cx)).map_or(made, |(_, o)| o),
                        this,
                    }),
                    _ => Ok(GlobalObjects { global: made, this }),
                };
                // `made` was made, so the run is live: a stale or foreign handle in the answer
                // faults the agent (decision 6).
                let refused = match answer {
                    Ok(GlobalObjects { global, this }) => {
                        bad!(cx, Value::Object(global))
                            || this.is_some_and(|this| bad!(cx, Value::Object(this)))
                    }
                    Err(Abrupt::Throw(value)) => bad!(cx, value),
                    Err(Abrupt::Terminated(_)) => false,
                };
                if refused {
                    cx.host_mut().expect_fault = true;
                    pool!(cx).log.push("  setup expect fault".to_string());
                }
                answer
            };
        let data: Option<swell::HostDefined> =
            host_defined.then(|| std::rc::Rc::new(Holds(Vec::new())) as swell::HostDefined);
        let answer = if with_setup {
            agent.create_realm(Some(&mut setup), data)
        } else {
            agent.create_realm(None, data)
        };
        let line = format!("create_realm(setup {with_setup}) -> {}", shape(&answer));
        let realm = match answer {
            Ok(Outcome::Normal(realm)) => Some(realm),
            Ok(Outcome::Throw(root)) => {
                let pool = &mut agent.host_mut().pool;
                keep(&mut pool.rng, &mut pool.roots, (me, root));
                None
            }
            _ => None,
        };
        let line = match &realm {
            Some(realm) => format!("{line}; {}", define_host_functions(agent, realm)),
            None => line,
        };
        (line, realm)
    });
    if let Some(realm) = realm {
        keep(&mut world.pool.rng, &mut world.pool.realms, (a, realm));
    }
    line
}

/// `with_cx` of a realm the sequence holds (its own, or the other agent's).
fn op_with_cx(world: &mut World, a: usize) -> String {
    let Some((origin, realm)) = world.take_realm() else {
        return "with_cx(no realm)".to_string();
    };
    let answer = world.on(a, |agent| {
        agent.with_cx(&realm, |cx| {
            cx_ops(cx);
        })
    });
    if is_foreign(origin, a) {
        foreign_answer("with_cx", &answer);
    }
    world.pool.realms.push((origin, realm));
    format!("with_cx -> {}", shape(&answer))
}

/// `parse_script` of a broken or a slice 1 source, as UTF-8 or as UTF-16.
fn op_parse_script(world: &mut World, a: usize) -> String {
    let Some((origin, realm)) = world.take_realm() else {
        return "parse_script(no realm)".to_string();
    };
    let text = source(&mut world.pool.rng);
    let mut utf16: Vec<u16> = text.encode_utf16().collect();
    let as_utf16 = world.pool.rng.chance(30);
    if as_utf16 && world.pool.rng.chance(50) {
        let i = world.pool.rng.index(utf16.len() + 1);
        utf16.insert(
            i,
            world.pool.rng.pick(&[0xD800, 0xDFFF, 0x0, 0x2028, 0xFEFF]),
        );
    }
    let answer = world.on(a, |agent| {
        let source = if as_utf16 {
            Source::Utf16(&utf16)
        } else {
            Source::Utf8(&text)
        };
        agent.parse_script(&realm, source, None)
    });
    if is_foreign(origin, a) {
        foreign_answer("parse_script", &answer);
    }
    let mut line = format!(
        "parse_script({text:?}, utf16 {as_utf16}) -> {}",
        shape(&answer)
    );
    match answer {
        Ok(Parsed::Parsed(script)) => {
            // Most scripts run at once, the others wait in the pool.
            if world.pool.rng.chance(70) {
                let outcome = world.on(a, |agent| agent.evaluate_script(&script));
                let _ = std::fmt::Write::write_fmt(
                    &mut line,
                    format_args!("; evaluate_script -> {}", shape(&outcome)),
                );
                if let Ok(Outcome::Normal(root) | Outcome::Throw(root)) = outcome {
                    keep(&mut world.pool.rng, &mut world.pool.roots, (a, root));
                }
            }
            keep(&mut world.pool.rng, &mut world.pool.scripts, (a, script));
        }
        Ok(Parsed::Errors(errors)) => {
            // The first error's message, read in the realm parsed in.
            if let Some(first) = errors.first() {
                let message = world.on(a, |agent| {
                    agent.with_cx(&realm, |cx| {
                        let Ok(Value::Object(error)) = cx.get(first) else {
                            return String::new();
                        };
                        let Ok(name) = cx.new_string("message") else {
                            return String::new();
                        };
                        match cx.get_property(error, PropertyKey::String(name)) {
                            Ok(Ok(Value::String(message))) => {
                                cx.to_rust_string_lossy(message).unwrap_or_default()
                            }
                            other => format!("{other:?}"),
                        }
                    })
                });
                let _ = std::fmt::Write::write_fmt(
                    &mut line,
                    format_args!("; message {}", shape(&message)),
                );
            }
            for error in errors {
                keep(&mut world.pool.rng, &mut world.pool.roots, (a, error));
            }
        }
        _ => {}
    }
    world.pool.realms.push((origin, realm));
    line
}

/// `evaluate_script` of a script the sequence holds.
fn op_evaluate_script(world: &mut World, a: usize) -> String {
    let Some((origin, script)) = world.take_script() else {
        return "evaluate_script(no script)".to_string();
    };
    let answer = world.on(a, |agent| agent.evaluate_script(&script));
    if is_foreign(origin, a) {
        foreign_answer("evaluate_script", &answer);
    }
    let line = format!("evaluate_script -> {}", shape(&answer));
    if let Ok(Outcome::Normal(root) | Outcome::Throw(root)) = answer {
        keep(&mut world.pool.rng, &mut world.pool.roots, (a, root));
    }
    world.pool.scripts.push((origin, script));
    line
}

/// `Agent::call` or `Agent::construct` of roots the sequence holds.
fn op_call(world: &mut World, a: usize, construct: bool) -> String {
    let Some((ro, realm)) = world.take_realm() else {
        return "call(no realm)".to_string();
    };
    let mut taken: Vec<(usize, Root)> = Vec::new();
    let wanted = 2 + world.pool.rng.below(3);
    for _ in 0..wanted {
        if let Some(root) = world.take_root() {
            taken.push(root);
        }
    }
    if taken.len() < 2 {
        world.pool.roots.extend(taken);
        world.pool.realms.push((ro, realm));
        return "call(no roots)".to_string();
    }
    let with_target = world.pool.rng.chance(30);
    let answer = world.on(a, |agent| {
        let function = &taken[0].1;
        let second = &taken[1].1;
        let args: Vec<&Root> = taken.iter().skip(2).map(|(_, r)| r).collect();
        if construct {
            let new_target = with_target.then_some(second);
            agent.construct(&realm, function, &args, new_target)
        } else {
            agent.call(&realm, function, second, &args)
        }
    });
    if is_foreign(ro, a) {
        foreign_answer("call", &answer);
    }
    let line = format!(
        "{}(roots {}) -> {}",
        if construct { "construct" } else { "call" },
        taken.len(),
        shape(&answer)
    );
    if let Ok(Outcome::Normal(root) | Outcome::Throw(root)) = answer {
        keep(&mut world.pool.rng, &mut world.pool.roots, (a, root));
    }
    world.pool.roots.extend(taken);
    world.pool.realms.push((ro, realm));
    line
}

/// One of the reads and the other entry points that take a realm, a root or a script.
fn op_read(world: &mut World, a: usize) -> String {
    match world.pool.rng.below(10) {
        0 => {
            let Some((origin, realm)) = world.take_realm() else {
                return "global_object(no realm)".to_string();
            };
            let answer = world.on(a, |agent| agent.global_object(&realm));
            if is_foreign(origin, a) {
                foreign_answer("global_object", &answer);
            }
            let line = format!("global_object -> {}", shape(&answer));
            if let Ok(root) = answer {
                keep(&mut world.pool.rng, &mut world.pool.roots, (a, root));
            }
            world.pool.realms.push((origin, realm));
            line
        }
        1 => {
            let Some((origin, realm)) = world.take_realm() else {
                return "realm_host_defined(no realm)".to_string();
            };
            let answer = world.on(a, |agent| {
                agent.realm_host_defined(&realm).map(|d| d.is_some())
            });
            if is_foreign(origin, a) {
                foreign_answer("realm_host_defined", &answer);
            }
            world.pool.realms.push((origin, realm));
            format!("realm_host_defined -> {}", shape(&answer))
        }
        2 => {
            let Some((origin, script)) = world.take_script() else {
                return "script_host_defined(no script)".to_string();
            };
            let answer = world.on(a, |agent| {
                agent.script_host_defined(&script).map(|d| d.is_some())
            });
            if is_foreign(origin, a) {
                foreign_answer("script_host_defined", &answer);
            }
            world.pool.scripts.push((origin, script));
            format!("script_host_defined -> {}", shape(&answer))
        }
        3 => {
            let Some((origin, root)) = world.take_root() else {
                return "duplicate(no root)".to_string();
            };
            let answer = world.on(a, |agent| agent.duplicate(&root));
            if is_foreign(origin, a) {
                foreign_answer("duplicate", &answer);
            }
            let line = format!("duplicate -> {}", shape(&answer));
            if let Ok(second) = answer {
                keep(&mut world.pool.rng, &mut world.pool.roots, (a, second));
            }
            world.pool.roots.push((origin, root));
            line
        }
        4 => {
            let answer = world.on(a, |agent| agent.collect());
            format!("collect -> {}", shape(&answer))
        }
        5 => {
            let agent = &world.agents[a];
            format!(
                "reads -> {:?} {:?} {:?} {:?}",
                agent.state(),
                agent.heap_bytes(),
                agent.last_entry_steps(),
                agent.pending_microtasks()
            )
        }
        7 => {
            let Some((origin, realm)) = world.take_realm() else {
                return "queue_microtask(no realm)".to_string();
            };
            let answer = world.on(a, |agent| agent.queue_microtask(&realm, Box::new(cx_ops)));
            if is_foreign(origin, a) {
                foreign_answer("queue_microtask", &answer);
            }
            world.pool.realms.push((origin, realm));
            format!("queue_microtask -> {}", shape(&answer))
        }
        8 => {
            let answer = world.on(a, |agent| agent.perform_microtask_checkpoint());
            format!("perform_microtask_checkpoint -> {}", shape(&answer))
        }
        9 => {
            let answer = world.on(a, |agent| agent.clear_microtasks());
            format!("clear_microtasks -> {}", shape(&answer))
        }
        _ => {
            world.agents[a].interrupt_handle().interrupt();
            "interrupt".to_string()
        }
    }
}

/// Drop one realm, root or script: it is released at the agent's next safe point.
fn op_release(world: &mut World) -> String {
    match world.pool.rng.below(3) {
        0 => world
            .take_realm()
            .map_or("release(no realm)".to_string(), |(o, _)| {
                format!("release realm of {o}")
            }),
        1 => world
            .take_script()
            .map_or("release(no script)".to_string(), |(o, _)| {
                format!("release script of {o}")
            }),
        _ => world
            .take_root()
            .map_or("release(no root)".to_string(), |(o, r)| {
                format!("release {r:?} of {o}")
            }),
    }
}

/// The answer of an entry of a faulted agent is `Err(Faulted)`; after an entry of a live one,
/// the run spent no more than its budgets.
fn check_agent(world: &World, a: usize, was_faulted: bool, line: &str) {
    let agent = &world.agents[a];
    if was_faulted {
        assert_eq!(
            agent.state(),
            AgentState::Faulted,
            "a faulted agent stays faulted"
        );
        let refused = line.contains("Err(Faulted)")
            || line.contains("(no ")
            || line.starts_with("release")
            || line.starts_with("interrupt")
            || line.starts_with("reads");
        assert!(refused, "a faulted agent answered: {line}");
        assert_eq!(agent.heap_bytes(), Err(HostError::Faulted));
        return;
    }
    if agent.state() == AgentState::Alive {
        let steps = agent.last_entry_steps().unwrap();
        let bytes = agent.heap_bytes().unwrap();
        assert!(
            steps <= world.limits.steps_per_entry,
            "{steps} steps over the budget of {}: {line}",
            world.limits.steps_per_entry
        );
        assert!(
            bytes <= world.limits.memory_bytes,
            "{bytes} bytes over the budget of {}: {line}",
            world.limits.memory_bytes
        );
    }
}

/// An entry of a live agent faulted it exactly when a host function or a setup closure returned
/// what decision 6 refuses, or, beside that, when host data listed a stale or foreign handle.
fn check_fault(world: &World, a: usize, was_faulted: bool, line: &str) {
    if was_faulted {
        return;
    }
    let agent = &world.agents[a];
    let faulted = agent.state() == AgentState::Faulted;
    let host = agent.host();
    let log = &world.pool.log;
    let tail = log
        .get(log.len().saturating_sub(60)..)
        .unwrap_or_default()
        .join("\n");
    assert!(
        !host.expect_fault || faulted,
        "agent {a} returned a handle decision 6 refuses and was not faulted: {line}\n{tail}"
    );
    assert!(
        !faulted || host.expect_fault || host.poisoned,
        "agent {a} faulted with no refused answer and no stale host data: {line}\n{tail}"
    );
}

/// The key of an agent, as its roots print it.
fn agent_key(agent: &mut Agent<FuzzHost>) -> Option<String> {
    let realm = match agent.create_realm(None, None) {
        Ok(Outcome::Normal(realm)) => realm,
        _ => return None,
    };
    let text = format!("{realm:?}");
    let start = text.find("key: ")? + 5;
    let end = start + text.get(start..)?.find(',')?;
    text.get(start..end).map(str::to_string)
}

/// The transcript of the sequence of `seed`, with the agent keys written as `A` and `B`.
fn run_sequence(seed: u64) -> String {
    let mut rng = Rng(seed);
    let limits = random_limits(&mut rng);
    let seeds = Seeds {
        hash_key: [3; 16],
        random_seed: [5; 32],
    };
    let mut keys = Vec::new();
    let mut agents = Vec::new();
    for index in 0..2 {
        // The key is read with the largest limits on an agent of its own made just before,
        // so that it is the agent's key minus one; the fuzzed agents get `limits`.
        let mut probe = Agent::new(FuzzHost::default(), support::host::limits(), seeds).unwrap();
        let probe_key: u32 = agent_key(&mut probe).and_then(|k| k.parse().ok()).unwrap();
        let agent = Agent::new(
            FuzzHost {
                index,
                ..FuzzHost::default()
            },
            limits,
            seeds,
        )
        .unwrap();
        keys.push(probe_key + 1);
        agents.push(agent);
    }
    let Ok(agents) = <[Agent<FuzzHost>; 2]>::try_from(agents) else {
        panic!("two agents");
    };
    let mut world = World {
        agents,
        limits,
        pool: Pool {
            rng,
            ..Pool::default()
        },
    };
    world.log(format!("seed {seed:#x} limits {limits:?}"));
    for a in 0..2 {
        world.agents[a].host_mut().expect_fault = false;
        let line = op_create_realm(&mut world, a);
        check_fault(&world, a, false, &line);
        world.log(format!("{a}: {line}"));
    }
    for _ in 0..CALLS {
        let a = usize::from(world.pool.rng.chance(20));
        let was_faulted = world.agents[a].state() == AgentState::Faulted;
        world.agents[a].host_mut().expect_fault = false;
        let line = match world.pool.rng.below(24) {
            0..=2 => op_create_realm(&mut world, a),
            3..=7 => op_with_cx(&mut world, a),
            8..=11 => op_parse_script(&mut world, a),
            12..=15 => op_evaluate_script(&mut world, a),
            16 => op_call(&mut world, a, false),
            17 => op_call(&mut world, a, true),
            18..=21 => op_read(&mut world, a),
            _ => op_release(&mut world),
        };
        check_fault(&world, a, was_faulted, &line);
        check_agent(&world, a, was_faulted, &line);
        world.log(format!("{a}: {line}"));
    }
    // The end: every agent answers its reads, and a last collection after every root was
    // dropped finds nothing stale.
    let World { agents, pool, .. } = world;
    let mut log = pool.log;
    drop((pool.roots, pool.realms, pool.scripts));
    for (a, mut agent) in agents.into_iter().enumerate() {
        let collect = agent.collect();
        log.push(format!("{a}: end {:?} {}", agent.state(), shape(&collect)));
        if agent.state() == AgentState::Alive {
            assert!(
                matches!(collect, Ok(Outcome::Normal(_))),
                "the last collection of agent {a}: {collect:?}"
            );
        }
    }
    let mut text = log.join("\n");
    for (name, key) in ["A", "B"].iter().zip(keys) {
        text = text.replace(&format!("key: {key},"), &format!("key: {name},"));
    }
    text
}

/// The variable that runs the sequences in this process, as the child the test starts.
const CHILD: &str = "SWELL_FUZZ_CHILD";
/// The variable that runs one sequence alone.
const ONE_SEED: &str = "SWELL_FUZZ_SEED";

/// Run the sequence of `seed` twice on a thread of the stack its limits state, and compare.
fn run_twice(seed: u64) {
    let limits = random_limits(&mut Rng(seed));
    let stack = limits.min_stack_bytes().saturating_add(HARNESS_STACK);
    let joined = std::thread::Builder::new()
        .stack_size(stack)
        .spawn(move || {
            let first = run_sequence(seed);
            let second = run_sequence(seed);
            if first != second {
                let at = first
                    .lines()
                    .zip(second.lines())
                    .position(|(x, y)| x != y)
                    .unwrap_or(0);
                panic!(
                    "seed {seed:#x}: two runs differ at line {at}:\n{:?}\n{:?}",
                    first.lines().nth(at),
                    second.lines().nth(at)
                );
            }
        })
        .unwrap()
        .join();
    if joined.is_err() {
        panic!("seed {seed:#x} failed; run it alone with {ONE_SEED}={seed}");
    }
}

/// The tests the sequences of a run are divided among, each in a process of its own under
/// cargo-nextest: a test that runs all of them takes 19 s alone in the default mode and was
/// killed at 60 s on a loaded machine. Together they run every one of the [`sequences`].
const PARTS: u64 = 8;

/// The seeds of part `part` of the [`PARTS`].
fn part_seeds(part: u64) -> std::ops::Range<u64> {
    FIRST_SEED + sequences() * part / PARTS..FIRST_SEED + sequences() * (part + 1) / PARTS
}

/// HOST-3: no entry point, read or hook dispatch panics for any argument: the seeded sequences
/// of [`sequences`] (400 in the lane `slow`) of interface calls with arbitrary arguments
/// (stale, released, foreign and
/// wrong-kind handles included), run in a child process, end with no panic, no abort, no run
/// past its budget, no answer the design excludes, and the same transcript in two runs of each
/// seed. The seed of a failing sequence is printed. This test runs the first of the [`PARTS`]
/// of the sequences, and runs the one sequence of `SWELL_FUZZ_SEED` when it is set; the tests
/// `host_3_no_entry_point_read_or_hook_panics_part_<n>` run the others.
#[test]
fn host_3_no_entry_point_read_or_hook_panics() {
    if let Some(seed) = std::env::var_os(ONE_SEED) {
        let seed: u64 = seed.to_string_lossy().parse().unwrap();
        println!("{}", run_sequence(seed));
        run_twice(seed);
        return;
    }
    run_part(test_path!("host_3_no_entry_point_read_or_hook_panics"), 0);
}

/// Part 2 of the sequences of [`host_3_no_entry_point_read_or_hook_panics`].
#[test]
fn host_3_no_entry_point_read_or_hook_panics_part_2() {
    run_part(
        test_path!("host_3_no_entry_point_read_or_hook_panics_part_2"),
        1,
    );
}

/// Part 3 of the sequences of [`host_3_no_entry_point_read_or_hook_panics`].
#[test]
fn host_3_no_entry_point_read_or_hook_panics_part_3() {
    run_part(
        test_path!("host_3_no_entry_point_read_or_hook_panics_part_3"),
        2,
    );
}

/// Part 4 of the sequences of [`host_3_no_entry_point_read_or_hook_panics`].
#[test]
fn host_3_no_entry_point_read_or_hook_panics_part_4() {
    run_part(
        test_path!("host_3_no_entry_point_read_or_hook_panics_part_4"),
        3,
    );
}

/// Part 5 of the sequences of [`host_3_no_entry_point_read_or_hook_panics`].
#[test]
fn host_3_no_entry_point_read_or_hook_panics_part_5() {
    run_part(
        test_path!("host_3_no_entry_point_read_or_hook_panics_part_5"),
        4,
    );
}

/// Part 6 of the sequences of [`host_3_no_entry_point_read_or_hook_panics`].
#[test]
fn host_3_no_entry_point_read_or_hook_panics_part_6() {
    run_part(
        test_path!("host_3_no_entry_point_read_or_hook_panics_part_6"),
        5,
    );
}

/// Part 7 of the sequences of [`host_3_no_entry_point_read_or_hook_panics`].
#[test]
fn host_3_no_entry_point_read_or_hook_panics_part_7() {
    run_part(
        test_path!("host_3_no_entry_point_read_or_hook_panics_part_7"),
        6,
    );
}

/// Part 8 of the sequences of [`host_3_no_entry_point_read_or_hook_panics`].
#[test]
fn host_3_no_entry_point_read_or_hook_panics_part_8() {
    run_part(
        test_path!("host_3_no_entry_point_read_or_hook_panics_part_8"),
        7,
    );
}

/// The sequences of part `part`, run in a child process that runs only the test `test`; the
/// parent names the seed the child died in.
fn run_part(test: TestPath, part: u64) {
    if std::env::var_os(ONE_SEED).is_some() {
        return;
    }
    let seeds = part_seeds(part);
    let count = seeds.end - seeds.start;
    if std::env::var_os(CHILD).is_some() {
        for seed in seeds {
            eprintln!("fuzz: seed {seed}");
            run_twice(seed);
        }
        println!("fuzz: {count} sequences done");
        return;
    }
    let exe = std::env::current_exe().unwrap();
    let out = std::process::Command::new(exe)
        .args(["--exact", test.as_str(), "--nocapture", "--test-threads=1"])
        .env(CHILD, "1")
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    let done = format!("fuzz: {count} sequences done");
    if out.status.success() && stdout.contains(&done) {
        return;
    }
    let last = stderr
        .lines()
        .rev()
        .find_map(|line| line.strip_prefix("fuzz: seed "))
        .unwrap_or("none");
    let tail: Vec<&str> = stderr.lines().rev().take(30).collect();
    let tail: Vec<&str> = tail.into_iter().rev().collect();
    panic!(
        "the fuzz child ended with {} in the sequence of seed {last} (run it alone with \
         {ONE_SEED}={last}):\n{stdout}\n{}",
        out.status,
        tail.join("\n")
    );
}

/// The parts cover every sequence once, in order.
#[test]
fn the_parts_cover_every_sequence() {
    let mut next = FIRST_SEED;
    for part in 0..PARTS {
        let seeds = part_seeds(part);
        assert_eq!(seeds.start, next, "part {part}");
        assert!(seeds.end > seeds.start, "part {part} is empty");
        next = seeds.end;
    }
    assert_eq!(next, FIRST_SEED + sequences());
}
