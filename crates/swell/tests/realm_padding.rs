//! The instrument that runs every test at several sizes of the realm
//! (`story:tests-hold-at-any-size-of-the-realm`): `support::host::pad_realm` pads a realm by k
//! host functions, reached from the realm's global object under a Symbol only, and
//! `support::host::create_realm` pads every realm of a test by the k of
//! `SWELL_TEST_REALM_FUNCTIONS`, which the task `test-realm-sizes` sets. Run also with the feature
//! `collect-every-safe-point`. And the instrument's other half, the profile the tests run in:
//! debug assertions and overflow checks stay on at every optimization level the gate builds.
#![cfg(test)]

use crate::support;

use support::host::{agent, pad_realm};
use support::script::{Outcome, eval_text};
use swell::{HOST_FUNCTION_LIVE_BYTES, Outcome as HostOutcome, PROPERTY_LIVE_BYTES};

/// The sizes the test pads by: none, and three of the sizes the task `test-realm-sizes` runs.
const SIZES: [u64; 4] = [0, 17, 38, 200];

/// What one agent read at one k: the names of the global object, `live_bytes()` and
/// `heap_bytes()` after the padded realm and a collection.
struct Reading {
    names: Outcome,
    live: u64,
    heap: u64,
}

/// A realm padded by `k` functions, passed as the argument and not read from the variable, in an
/// agent of its own: its `live_bytes()` and `heap_bytes()` after a collection and the String
/// keys of its global object. Once the realm and every root of it are dropped, two collections
/// bring `live_bytes()` back to its reading before `create_realm`: the padding dies with the
/// realm.
fn padded(k: u64) -> Reading {
    let mut agent = agent().unwrap();
    assert!(matches!(agent.collect(), Ok(HostOutcome::Normal(_))));
    let before = agent.live_bytes().unwrap();
    // The bare realm, padded by `k` (an exception of `support::host::create_realm`).
    let HostOutcome::Normal(realm) = agent.create_realm(None, None).unwrap() else {
        panic!("create_realm did not answer Normal");
    };
    pad_realm(&mut agent, &realm, usize::try_from(k).unwrap());
    assert!(matches!(agent.collect(), Ok(HostOutcome::Normal(_))));
    let live = agent.live_bytes().unwrap();
    let heap = agent.heap_bytes().unwrap();
    let names = eval_text(
        &mut agent,
        &realm,
        "Object.getOwnPropertyNames(globalThis).join();",
    );
    drop(realm);
    assert!(matches!(agent.collect(), Ok(HostOutcome::Normal(_))));
    assert!(matches!(agent.collect(), Ok(HostOutcome::Normal(_))));
    assert_eq!(
        agent.live_bytes().unwrap(),
        before,
        "k = {k}: live_bytes after the realm was dropped and two collections"
    );
    Reading { names, live, heap }
}

/// `pad_realm` at k = 0, 17, 38 and 200, each in an agent of its own: the String keys of the
/// global object (`Object.getOwnPropertyNames(globalThis)`) are the same list at each k; after
/// `collect()`, `live_bytes()` rises from its reading at k = 0 by more than 0 and by at most
/// k × (`HOST_FUNCTION_LIVE_BYTES` + `PROPERTY_LIVE_BYTES`), the ceilings of a host function and
/// of a property's definition; and once the realm is dropped two collections bring
/// `live_bytes()` back to its reading before `create_realm`. The test prints `heap_bytes()`
/// after the realm and the bytes per function of each reading at each k.
#[test]
fn a_padded_realm_holds_k_functions_more() {
    let readings: Vec<(u64, Reading)> = SIZES.iter().map(|&k| (k, padded(k))).collect();
    let (_, bare) = &readings[0];
    assert!(
        matches!(&bare.names, Outcome::Normal(names) if names.contains("globalThis")),
        "{:?}",
        bare.names
    );
    for (k, reading) in &readings {
        let rise = reading.live.saturating_sub(bare.live);
        let heap_rise = reading.heap.saturating_sub(bare.heap);
        let (per_live, per_heap) = if *k == 0 {
            (0, 0)
        } else {
            (rise / k, heap_rise / k)
        };
        println!(
            "k = {k}: heap_bytes {} after the realm, live_bytes {}; over k = 0: live +{rise} \
             ({per_live} per function), heap +{heap_rise} ({per_heap} per function)",
            reading.heap, reading.live
        );
        assert_eq!(
            reading.names, bare.names,
            "k = {k}: the String keys of the global"
        );
        if *k > 0 {
            assert!(
                reading.live > bare.live,
                "k = {k}: live_bytes {} not above {}",
                reading.live,
                bare.live
            );
            assert!(
                rise <= k * (HOST_FUNCTION_LIVE_BYTES + PROPERTY_LIVE_BYTES),
                "k = {k}: live_bytes rose by {rise}, more than {k} x ({HOST_FUNCTION_LIVE_BYTES} \
                 + {PROPERTY_LIVE_BYTES})"
            );
        }
    }
}

/// The gate's step `test` runs most tests of this binary at opt-level 1, in the profile
/// `collect` of `.cargo/collect-profile.toml`, which inherits `test`; `Cargo.toml` sets
/// `overflow-checks = true` there. A test proves at opt-level 1 what it proves at opt-level 0
/// only while debug assertions and overflow checks stay on: this test fails when
/// `debug_assertions` is off, and when an `i32` addition that overflows at run time (its
/// operands through `black_box`, so the compiler cannot fold it) does not panic.
#[test]
fn the_profile_keeps_debug_assertions_and_overflow_checks_on() {
    if !cfg!(debug_assertions) {
        panic!("debug assertions are off in the profile this test binary was built in");
    }
    let sum =
        std::panic::catch_unwind(|| std::hint::black_box(i32::MAX) + std::hint::black_box(1_i32));
    assert!(
        sum.is_err(),
        "an i32 addition that overflows did not panic: overflow checks are off in the profile \
         this test binary was built in (it gave {sum:?})"
    );
}
