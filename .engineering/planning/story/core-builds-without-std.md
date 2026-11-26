---
format: aep.planning-md/3
id: story:core-builds-without-std
kind: story
status: draft
title: The engine core builds without std; the host injects every adapter
summary: 'The six engine crates are no_std with alloc: allocator, clock, randomness and modules come from the host, which may have system access or emulate it'
revision: 2
---
Decided by the owner on 2026-11-26, 02:51 CET: "I think it should be no STD for the core, and
then we'll just inject the adapters from the outside, and they can have, of course, system
access, but they don't have to. Yeah, it could also just be emulated." A change of architecture;
the design documents change with it.

## Context

- The six engine crates (`crates/swell`, `compiler`, `syntax`, `num`, `regexp`, `unicode`) use no
  file, environment, clock, thread, network or process API of `std` (counted on main `911a83d`:
  0 lines; `HOST-2` holds the same by a source scan). They do import from `std`: 243 lines in 82
  files.
- What comes from the system is already the host's: the clock
  (`host_system_utc_epoch_nanoseconds`), the time zones, the timer queue
  (`host_enqueue_timeout_job`), the two seeds of `Agent::new`, and module loading (the hook of
  slice 11). The shell binary `crates/cli` is one such host.
- What the core takes from `std` beyond `core` and `alloc`, on main `911a83d`:
  - the global allocator, through `alloc`'s collections;
  - float functions (`sin`, `pow` and the like): 34 call sites, 15 in `crates/swell/src` and 19
    in `crates/num/src`;
  - `thread_local!` in three places: `crates/swell/src/heap/mod.rs`,
    `crates/swell/src/heap/roots.rs`, `crates/num/src/nat.rs`;
  - `std::collections::HashMap` in `crates/swell/src/heap/map.rs`;
  - `std::error::Error` in `compiler` and `syntax` (in `core` under the same name).
- The released binaries of 0.12.0 load the system C library (`libc.so.6` with glibc 2.34 or
  newer on Linux, `VCRUNTIME140.dll` on Windows).

## Change

- Each of the six engine crates is `#![no_std]` with `extern crate alloc`. The crates name no
  allocator: the program that links them provides it (the shell: the system's; an embedder: its
  own, a fixed block included).
- Float functions the engine needs are computed in Rust inside the repository (`swell-num`), with
  no call into a C math library. Their results are held equal to today's on a fixed corpus and
  by the test262 counts; a function whose result differs in the last bit is named in the
  changelog with the cases.
- The three `thread_local!` states move into values the agent owns or receives.
- The hash map of `heap/map.rs` no longer comes from `std`; its SipHash keyed by `hash_key`
  stays (`HOST-145`).
- An adapter is whatever implements the host trait: with system access (the shell), without, or
  emulated (a fixed clock, a seeded generator, modules from memory). The design documents say
  so, and one test host in the repository is of the emulated kind and uses no `std` API of the
  system either.
- `HOST-2` widens: the source scan refuses any `std::` path and any missing `#![no_std]` in the
  six crates.
- The gate builds the six crates for a target that has no `std` (a bare `-none` target), so a
  use of `std` through a dependency fails the build. The local gate and the forge install that
  target the same way (`rust-toolchain.toml`).
- `crates/cli`, `crates/test262` and `crates/unicode-gen` stay `std` programs.

## Order

1. `unicode`, `syntax`, `compiler`, `regexp`: imports only (`std::` to `core::`/`alloc::`).
2. `num`: float functions in Rust and the `thread_local!` of `nat.rs`.
3. `swell`: the map, the two `thread_local!`, the remaining imports.
4. The scan, the bare-target build in the gate, the design documents.

## Acceptance

- `cargo build` of the six crates for the bare target succeeds in the gate; the scan of `HOST-2`
  finds no `std::` path in them.
- `test262 total` prints the same seven numbers as before the change.
- The gate's test steps stay within 120 s; the bare-target build is a build step, outside that
  clock.
- An example host built without `std` runs a script and reads its result (a test on the
  workstation's own target, with a fixed block as its allocator).

## Not in this story

- A musl or static build of the shell binary; a WebAssembly build of the core. Both get easier
  after it and are stories of their own.
