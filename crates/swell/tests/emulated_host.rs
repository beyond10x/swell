//! A host of the emulated kind (`docs/design/host-interface.md`, "Adapters";
//! `story:core-builds-without-std`): the engine core has no access to the system, and an adapter
//! is whatever implements `Host`. This one answers the clock from a fixed value, gives the agent
//! fixed seeds and keeps the jobs it is handed in memory. It reaches nothing of the system: the
//! module names none of the clock, environment, file, network and process modules of the
//! standard library, which `the_module_names_no_system_api` reads from its source.
//!
//! Module loading from memory, the third part of such a host, is slice 11 and not delivered: the
//! load hook keeps nothing.
#![cfg(test)]

use swell::{
    Agent, HookCx, Host, Job, Limits, LoadRequest, ObjectRef, Outcome, Parsed, Realm, Seeds,
    Source, Value,
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

impl Host for EmulatedHost {
    fn host_enqueue_generic_job(cx: &mut HookCx<'_, Self>, job: Job) {
        cx.host_mut().jobs.push(job);
    }

    fn host_enqueue_timeout_job(cx: &mut HookCx<'_, Self>, job: Job, _milliseconds: f64) {
        cx.host_mut().jobs.push(job);
    }

    fn host_enqueue_finalization_registry_cleanup_job(cx: &mut HookCx<'_, Self>, job: Job) {
        cx.host_mut().jobs.push(job);
    }

    /// Slice 11 delivers module loading; until then this host keeps no request.
    fn host_load_imported_module(_cx: &mut HookCx<'_, Self>, _request: LoadRequest) {}

    fn host_system_utc_epoch_nanoseconds(cx: &mut HookCx<'_, Self>, _global: ObjectRef) -> i128 {
        let host = cx.host_mut();
        host.clock_reads += 1;
        FIXED_NS
    }
}

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

/// An agent of the emulated host and a realm of it.
fn agent_and_realm() -> (Agent<EmulatedHost>, Realm) {
    let mut agent = Agent::new(EmulatedHost::default(), limits(), seeds()).unwrap();
    let realm = match agent.create_realm(None, None).unwrap() {
        Outcome::Normal(realm) => realm,
        other => panic!("create_realm answered {other:?}"),
    };
    (agent, realm)
}

/// The Number a script completes with.
fn number(agent: &mut Agent<EmulatedHost>, realm: &Realm, source: &str) -> f64 {
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

/// A script that reads `Date.now()` twice gets the fixed time both times, and two agents of the
/// host answer the same: the clock is the host's, and this host's does not move.
#[test]
fn date_now_reads_the_fixed_clock_twice_with_the_same_answer() {
    let source = "var first = Date.now(); var second = Date.now(); first === second ? first : -1;";
    let (mut agent, realm) = agent_and_realm();
    assert_eq!(number(&mut agent, &realm, source), FIXED_MS);
    assert_eq!(agent.host().clock_reads, 2);
    assert!(agent.host().jobs.is_empty());

    let (mut again, realm) = agent_and_realm();
    assert_eq!(number(&mut again, &realm, source), FIXED_MS);
    assert_eq!(again.host().clock_reads, 2);
}

/// The module reaches nothing of the system: its source names none of the clock, environment,
/// file, network and process modules of the standard library.
#[test]
fn the_module_names_no_system_api() {
    let source = include_str!("emulated_host.rs");
    let named: Vec<String> = ["time", "env", "fs", "net", "process"]
        .iter()
        .map(|module| format!("std::{module}"))
        .filter(|path| source.contains(path.as_str()))
        .collect();
    assert!(named.is_empty(), "{named:?}");
}
