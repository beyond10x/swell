//! A host of the emulated kind (`docs/design/host-interface.md`, "Adapters";
//! `story:core-builds-without-std`): the engine core has no access to the system, and an adapter
//! is whatever implements `Host`. This one is a test host inside the integration-test binary of
//! `swell`, which links `std` and its system allocator; it calls no system API. It answers the
//! clock from a fixed value, gives the agent fixed seeds and keeps the jobs it is handed in
//! memory. Its source names no `std` path and no output macro of the `std` prelude, which
//! `the_module_names_no_system_api` reads with the scanners of HOST-2. Its hooks are given as
//! values to the scripted host of `support/scripted.rs`, so that the test binary compiles the
//! engine for no host type of its own (`host_types.rs`).
//!
//! Module loading from memory, the third part of such a host, is slice 11 and not delivered: the
//! load hook keeps nothing.
#![cfg(test)]

use crate::source_scan::scan::{ambient_authority, code_of, has_word, std_paths};
use crate::support::scripted::{Hooks, Scripted};
use swell::{
    Agent, HookCx, Job, Limits, LoadRequest, ObjectRef, Outcome, Parsed, Realm, Seeds, Source,
    Value,
};

/// The clock's one answer, nanoseconds since the epoch: 2026-11-18T11:06:40.123456789Z.
const FIXED_NS: i128 = 1_795_000_000_123_456_789;

/// What `Date.now()` answers over [`FIXED_NS`]: the nanoseconds divided by 10^6, rounded toward
/// negative infinity (HOST-125).
const FIXED_MS: f64 = 1_795_000_000_123.0;

/// A host whose clock stands still at [`FIXED_NS`], with its jobs kept in memory.
#[derive(Default)]
struct EmulatedHost {
    /// The jobs the engine handed over, in order; this host runs none.
    jobs: Vec<Job>,
    /// How often the engine read the clock.
    clock_reads: u32,
}

fn host_enqueue_generic_job(cx: &mut HookCx<'_, Scripted>, job: Job) {
    cx.host_mut().state_mut::<EmulatedHost>().jobs.push(job);
}

fn host_enqueue_timeout_job(cx: &mut HookCx<'_, Scripted>, job: Job, _milliseconds: f64) {
    cx.host_mut().state_mut::<EmulatedHost>().jobs.push(job);
}

fn host_enqueue_finalization_registry_cleanup_job(cx: &mut HookCx<'_, Scripted>, job: Job) {
    cx.host_mut().state_mut::<EmulatedHost>().jobs.push(job);
}

/// Slice 11 delivers module loading; until then this host keeps no request.
fn host_load_imported_module(_cx: &mut HookCx<'_, Scripted>, _request: LoadRequest) {}

fn host_system_utc_epoch_nanoseconds(cx: &mut HookCx<'_, Scripted>, _global: ObjectRef) -> i128 {
    let host = cx.host_mut().state_mut::<EmulatedHost>();
    host.clock_reads += 1;
    FIXED_NS
}

/// The hooks of the tests of this module, given to the scripted host with the state
/// [`EmulatedHost`] (`support/scripted.rs`).
const HOOKS: Hooks = Hooks {
    host_enqueue_generic_job,
    host_enqueue_timeout_job,
    host_enqueue_finalization_registry_cleanup_job,
    host_load_imported_module,
    host_system_utc_epoch_nanoseconds,
    ..Hooks::PROVIDED
};

/// Fixed seeds: every agent of this host hashes and draws the same.
fn seeds() -> Seeds {
    Seeds {
        hash_key: [3; 16],
        random_seed: [5; 32],
    }
}

/// Limits no script of this module comes near.
fn limits() -> Limits {
    Limits {
        steps_per_entry: 100_000,
        memory_bytes: 16 * 1024 * 1024,
        frame_depth: 1_000,
        reentry_depth: 64,
        parser_nesting: 256,
        string_length: 1 << 20,
        bigint_bits: 1 << 16,
    }
}

/// An agent of the emulated host with `seeds`, and a realm of it.
fn agent_and_realm(seeds: Seeds) -> (Agent<Scripted>, Realm) {
    let mut agent = Agent::new(
        Scripted::new(EmulatedHost::default(), HOOKS),
        limits(),
        seeds,
    )
    .unwrap();
    let realm = match agent.create_realm(None, None).unwrap() {
        Outcome::Normal(realm) => realm,
        other => panic!("create_realm answered {other:?}"),
    };
    (agent, realm)
}

/// The Number a script completes with.
fn number(agent: &mut Agent<Scripted>, realm: &Realm, source: &str) -> f64 {
    let script = match agent
        .parse_script(realm, Source::Utf8(source), None)
        .unwrap()
    {
        Parsed::Parsed(script) => script,
        other => panic!("{source:?} did not parse: {other:?}"),
    };
    let root = match agent.evaluate_script(&script).unwrap() {
        Outcome::Normal(root) => root,
        other => panic!("{source:?} answered {other:?}"),
    };
    match agent.with_cx(realm, |cx| cx.get(&root)).unwrap().unwrap() {
        Value::Number(n) => n,
        other => panic!("{source:?} completed with {other:?}"),
    }
}

/// What of the system a Rust source names, read with the scanners of HOST-2
/// (`source_scan.rs`): the clock, the environment, files, the network, processes and the
/// random state of `std` (also through a grouped import), any other `std` path, and the macros
/// of the `std` prelude that write to the process's standard streams.
fn system_api(source: &str) -> Vec<String> {
    let code = code_of(source);
    let mut named = ambient_authority(&code);
    named.extend(std_paths(&code));
    for name in ["print!", "println!", "eprint!", "eprintln!", "dbg!"] {
        if has_word(&code, name) {
            named.push(name.to_string());
        }
    }
    named
}

/// A script that reads `Date.now()` twice gets the fixed time both times, and two agents of the
/// host answer the same: the clock is the host's, and this host's does not move.
#[test]
fn date_now_reads_the_fixed_clock_twice_with_the_same_answer() {
    let source = "var first = Date.now(); var second = Date.now(); first === second ? first : -1;";
    let (mut agent, realm) = agent_and_realm(seeds());
    assert_eq!(number(&mut agent, &realm, source), FIXED_MS);
    assert_eq!(agent.host().state::<EmulatedHost>().clock_reads, 2);
    assert!(agent.host().state::<EmulatedHost>().jobs.is_empty());

    let (mut again, realm) = agent_and_realm(seeds());
    assert_eq!(number(&mut again, &realm, source), FIXED_MS);
    assert_eq!(again.host().state::<EmulatedHost>().clock_reads, 2);
}

/// The seeds are the host's too: two agents of the host with its fixed seeds draw the same
/// three values of `Math.random`, bit for bit, and an agent whose random seed differs in one
/// bit draws others. No call reads the clock.
#[test]
fn math_random_draws_the_same_from_the_fixed_seeds() {
    let draws = |seeds: Seeds| -> [u64; 3] {
        let (mut agent, realm) = agent_and_realm(seeds);
        let values = [0, 1, 2].map(|_| number(&mut agent, &realm, "Math.random();"));
        assert_eq!(agent.host().state::<EmulatedHost>().clock_reads, 0);
        values.map(f64::to_bits)
    };
    let first = draws(seeds());
    assert!(
        first
            .iter()
            .all(|&bits| (0.0..1.0).contains(&f64::from_bits(bits))),
        "{first:?}"
    );
    assert_eq!(draws(seeds()), first);
    let mut other = seeds();
    other.random_seed[0] ^= 1;
    assert_ne!(draws(other), first);
}

/// The module reaches nothing of the system: its source names no `std` path, and none of the
/// items of HOST-2's scan.
#[test]
fn the_module_names_no_system_api() {
    let named = system_api(include_str!("emulated_host.rs"));
    assert!(named.is_empty(), "{named:?}");
}

/// The check of `the_module_names_no_system_api` sees a grouped import of `std::env` and of
/// `std::time::SystemTime` (adversary pass 1 on unit gate: a search for the text `std::time`
/// did not), and passes text that names `core` and `alloc` only.
#[test]
fn the_filter_sees_a_grouped_std_import() {
    let planted =
        "use std::{env, time::SystemTime};\nfn now() -> SystemTime { SystemTime::now() }\n";
    assert_eq!(
        system_api(planted),
        ["SystemTime", "std::{env}", "line 1: std"]
    );
    assert!(system_api("use core::cell::Cell;\nuse alloc::vec::Vec;\n// std::fs\n").is_empty());
}

/// The module is part of a `std` test binary, whose prelude brings `eprintln!` and `print!`
/// (writes to the process's standard streams) without a `std` path; the check names both
/// (adversary pass 2 on unit gate).
#[test]
fn the_filter_sees_an_output_macro_of_the_std_prelude() {
    let planted = "fn host_enqueue_generic_job() { eprintln!(\"job\"); print!(\"x\"); }\n";
    assert_eq!(system_api(planted), ["print!", "eprintln!"], "{planted}");
}
