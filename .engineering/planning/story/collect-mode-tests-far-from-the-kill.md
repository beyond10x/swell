---
format: aep.planning-md/3
id: story:collect-mode-tests-far-from-the-kill
kind: story
status: active
title: The tests of swell with a collection at every safe point stay far from the 60 s kill
summary: At a load of 112 the slowest test of the collect-mode run took 58.5 s of the 60 s a test gets; five heavy tests get gate counts with their full counts in the lane slow, and a rule states what a new script and a new test may cost there.
relations:
- decomposes: epic:slice-06-bindings-and-iteration
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T16:57:50Z", actor: "human:operator", revision: 2, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-22T16:57:50Z", actor: "human:operator", revision: 3, executor: "agent:coordinator"}
---
Waits for: nothing. A defect under the organization's rule "Tests stay fast": a test 1.5 s under
the kill is fixed before the next feature, and raising a timeout, adding shards or waiting
longer is not a fix.

## Context

Measured on `wave/budget-1` at `db0978a`, at a load of 112 on 20 cores: the package swell with
`--features collect-every-safe-point` took 274 s, and its slowest tests were parts of the
`language` test (part_12 58.5 s, part_13 48.7 s, part_11 34.4 s). Two merges earlier, at a
lower load, the slowest part took 12.0 s; at a load of 85 the run took 144.8 s with the slowest
test at 20.3 s. Measured per test and per script by the unit, alone, at a load of 41 to 59:
no language script takes over 0.5 CPU seconds per mode, part_12 takes 2.35 CPU seconds and ran
18.5 s of wall time alone at a load of 58. The cost is the machine's load on tests that are a
few CPU seconds each; the heaviest were `determinism` host_133 (5.25 CPU s), the flat-source
tests of `src/script.rs` (3.09, 2.17, 1.30), the 21 re-entry tests (15.8 together), the
stringify nesting test (2.10) and the sort charge test (2.09).

Every instruction is a safe point that collects in that mode: in the test build without
optimization an empty `for` iteration takes 2.2 ms there against 0.02 ms in the default mode.

## Change

1. Gate counts, each with its full count under `SWELL_FULL_COUNTS=1` (the lane `slow`), each
   test still showing what it showed:
   - `crates/swell/tests/support/stack.rs`: `BUILTINS_REENTRY` with a collection at every safe
     point is 50, not 100; the tests still re-enter through every builtin at every level.
   - `crates/swell/src/script.rs`: the flat source has 5,000 lines in the gate and 20,000 at
     the full count; the assertions compare readings of the same source, and the parse still
     polls 8 times.
   - `crates/swell/tests/determinism.rs`, host_133: one child process in the collect gate, two
     otherwise; it still compares across processes.
   - `crates/swell/tests/builtins_json.rs`: the stringify nesting is 1,000 in the collect
     gate, 2,000 at the full count.
   - `crates/swell/tests/builtins_array_mutation.rs`: the sort charge test sorts 129 elements
     in the collect gate (2^7 + 1, as 513 is 2^9 + 1), 513 at the full count.
2. `crates/swell/tests/language.rs` prints one line per run of a script,
   `script-time <wall> <cpu> <script> <mode>`, when `SWELL_SCRIPT_TIMES=1` is set. No script
   changes and the number of parts stays 13: more parts move the largest part from about 2.3
   to 1.7 CPU seconds.
3. `AGENTS.md`, "Commands and the gate": a new language script stays under 0.5 CPU seconds per
   run and a new test under 2 s alone with a collection at every safe point, with the command
   that measures it.

No test is deleted, ignored or moved out of the gate; no timeout changes.

## Acceptance

- With a collection at every safe point no test takes over 10 s alone, and none over 20 s in a
  whole-package run, at the load of the run, which is stated.
- The count of tests equals the base's in both modes (677 and 675), all passing; the touched
  tests pass with `SWELL_FULL_COUNTS=1` in both modes.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` and `cargo fmt --all
  --check` exit 0.

## Not in this story

Building the tests of that mode with `opt-level = 1`: measured by the unit, the collect run
then takes 20.1 CPU seconds where it takes 111.4, and the build takes 4 min 51 s at a load of
75 where it takes 1 min 6 s. A story of its own decides it with a profile for that mode alone,
so that the stack bounds keep their frames and the default mode its build time.
