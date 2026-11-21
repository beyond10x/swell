---
format: aep.planning-md/3
id: story:host-interface-hardening
kind: story
status: active
title: Fuzzing, misuse, fault injection and determinism across the slice 1 host interface
summary: A seeded fuzz test of every entry point, stale, foreign and faulted answers everywhere, fault injection, determinism in one and two processes, the limit tests that need builtins and scripts; 13 HOST lines, four after their split.
relations:
- decomposes: epic:slice-01-harness
- depends_on: story:interpreter-and-script-entry-points
- depends_on: story:harness-builtins
scope:
- confidence: cited
  path: conformance/host/requirements.txt
- confidence: cited
  path: crates/swell/Cargo.toml
- confidence: cited
  path: crates/swell/src
- confidence: cited
  path: crates/swell/src/host/agent.rs
- confidence: cited
  path: crates/swell/src/host/limits.rs
- confidence: cited
  path: crates/swell/tests
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T04:49:59Z", actor: "human:operator", revision: 4, decided_on: {"recorded":{"review_outcome":2}}, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-21T04:49:59Z", actor: "human:operator", revision: 5, decided_on: {"recorded":{"review_outcome":2}}, executor: "agent:coordinator"}
---
## Context

The requirements of slice 1 that hold across the whole interface rather than one part of it:
no panic for any argument (`HOST-3`, decision 13, "Fuzzing is tooling"), defined answers to
misuse at every entry point and read (`docs/design/host-interface.md`, section 6), fault
injection at the calls that can find a broken invariant (section 10), determinism (section 19),
and the requirements whose test needs the builtins of `story:harness-builtins` and the scripts of
`story:interpreter-and-script-entry-points` together: `HOST-10` (`push`), `HOST-16` (`map`),
`HOST-17` (re-entry at full depth), `HOST-49` (`this.Array === Array`), `HOST-53`
(`Object.prototype.hasOwnProperty.call(this, "x")`). The requirements were split in the
document so that each of these is met whole in slice 1 (their later-slice parts are
`HOST-152` to `HOST-159`).

Two things meet here for the first time, because this story depends on both wave 4 stories:

- Builtins with ECMAScript functions as callbacks. `story:harness-builtins` tests `map`, `bind`,
  `call`, `toJSON` and the replacer with host functions as callbacks; this story tests them with
  script functions.
- The Rust stack a re-entry needs. `story:interpreter-and-script-entry-points` measured
  `Limits::min_stack_bytes` on re-entry through host functions; the builtins add re-entry through
  `Function.prototype.call`, the `map` callback, `toJSON` and the replacer, whose Rust frames
  differ. This story runs the measurement again through each of them and corrects the constants
  in `host/limits.rs` when it fails. `story:test262-run-path` runs its workers on exactly
  `min_stack_bytes()`, so it depends on this story.

Epic acceptance item 13 (the same script run twice prints the same bytes) is held by
`story:interpreter-and-script-entry-points` (`same_script_twice_gives_the_same_steps_and_value`);
`HOST-133` here extends it to hook calls and to two processes and cites it. Item 9 (the memory
budget) is held by `story:heap-values-and-host-surface`; `HOST-10` reaches it from a script.

A fault one of these tests finds is a finding that goes back to the story that owns the failing
part, which the coordinator resumes; this story writes only the files on its `Writes:` line.

## Requirements

Lines moved from `pending slice 1` to `test`:

- `HOST-3` `swell::fuzz::host_3_no_entry_point_read_or_hook_panics`
- `HOST-10` `swell::limits_scripts::host_10_endless_allocation_ends_in_memory_budget`
- `HOST-16` `swell::limits_scripts::host_16_zero_reentry_depth_refuses_the_map_callback`
- `HOST-17` `swell::limits_scripts::host_17_deepest_reentry_parses_at_full_nesting`
- `HOST-30` `swell::misuse::host_30_stale_handles_and_released_roots`
- `HOST-31` `swell::misuse::host_31_handles_of_another_agent`
- `HOST-45` `swell::misuse::host_45_realm_root_and_script_of_another_agent`
- `HOST-48` `swell::misuse::host_48_a_faulted_agent_answers_faulted_everywhere`
- `HOST-49` `swell::limits_scripts::host_49_setup_closure_and_clause_19_globals`
- `HOST-53` `swell::limits_scripts::host_53_parse_defines_no_global_binding`
- `HOST-71` `swell::faults::host_71_injected_fault_at_each_call_faults_the_agent`
- `HOST-102` `swell::misuse::host_102_reads_change_nothing_and_call_no_hook`
- `HOST-129` `swell::misuse::host_129_collect_runs_no_code_and_calls_no_hook`
- `HOST-133` `swell::determinism::host_133_same_calls_same_hooks_same_outcomes`
- `HOST-160` `swell::misuse::host_160_a_handle_returned_by_the_host_faults_the_agent`

## Additions after wave 2 of slice 1 (2026-11-21)

The design changed after this story was accepted (`docs/design/host-interface.md`, decision 5,
"A method that takes a handle returns `Result<_, HostError>`", and decision 6, "A handle the
host returns", with the new requirement `HOST-160`; found by the unit of
`story:heap-values-and-host-surface`, see `docs/plan/slice-01-wave-2.md`).

- The corrected signatures are written by `story:object-model-and-realms` and
  `story:interpreter-and-script-entry-points`. This story holds them:
  `host_30_stale_handles_and_released_roots` and `host_31_handles_of_another_agent` in
  `misuse.rs` cover `new_host_object`, `new_error`, `new_host_function`, `cx.call`,
  `cx.construct`, `cx.get`, `cx.set`, `cx.to_number`, `cx.evaluate_script` and
  `cx.parse_script`, each with a stale handle, a released root and a handle of another agent
  (`Err(StaleHandle)` or `Err(ForeignHandle)`, nothing changed, the agent `Alive`), and once
  inside a run already terminated (`Err(StaleHandle)`, the run stays terminated).
- This story closes one more requirement, the fifteenth:
  `HOST-160` `swell::misuse::host_160_a_handle_returned_by_the_host_faults_the_agent`: a host
  function whose `Ok` value holds a stale handle, one whose `Abrupt::Throw` value holds a
  handle of another agent, and a `GlobalSetup` closure whose `GlobalObjects` hold a stale one;
  each ends in `Terminated(EngineFault)`, `state()` reads `Faulted`, another entry point
  answers `Err(HostError::Faulted)`, nothing panics. Its line moves from `pending slice 1` to
  `test` with the fourteen of Requirements. If the check is missing in the code of
  `story:object-model-and-realms`, that is a finding against that story, reported, not fixed
  here.
- Acceptance reads, with the fifteen lines: `host-requirements: 161 requirement(s), 161
  line(s): 70 test, 91 pending, 0 omitted`, and no entry for slice 1 in the second line.

- The current realm (2026-11-21, `docs/design/host-interface.md` decision 7, `HOST-161`, closed by
  `story:object-model-and-realms`): the fuzz test and `misuse.rs` cover `with_cx`, `Agent::call`
  and `Agent::construct` with a stale, a released and a foreign `Realm` (`Err(StaleHandle)` or
  `Err(ForeignHandle)`, the agent `Alive`).

- `Writes:` is extended for this wave (2026-11-21): any file under `crates/swell/src/` for a
  defect of the engine that a test of this story exposes. Each fix is the smallest change that
  makes the engine do what the requirement or the clause says, has a test that failed before
  it, and is listed in the unit's report (file, function, what was wrong, the `HOST-<n>` or
  clause). A defect that needs a design decision is reported with the test output and left
  failing. The sentence above that a missing check is "reported, not fixed here" is withdrawn:
  the earlier stories are closed and this is the last story that writes the engine in slice
  1.

## Change

Failing tests first: the tests of Requirements and the tests named below are written first;
each fails if an entry point panics, answers another error, calls a hook it must not, or runs
out of Rust stack.

- `crates/swell/tests/fuzz.rs`: a generator written with `std` only, from a seed, of sequences of
  interface calls (every `Agent` entry point and read of slice 1, `with_cx` with every `Cx`
  operation, host functions that call back) with arbitrary arguments, stale, released, foreign
  and wrong-kind handles among them, and of small scripts of the slice 1 grammar, under small
  budgets; 2,000 sequences from fixed seeds per run, and the seed of a failing sequence printed.
  A panic, an abort, a run past its budget, or a difference between two runs of one seed fails.
- `crates/swell/tests/misuse.rs`: for each entry point and read, the stale, foreign and faulted
  answers, checked with a host that records every hook call.
- `crates/swell/tests/faults.rs` and the feature `fault-injection` of `crates/swell/Cargo.toml`
  (off by default): a test-only switch in `host/agent.rs` that makes `parse_script`,
  `create_realm` or `collect` find a broken invariant. `story:ess-conformance-target` uses the
  same switch for the `engine-fault` outcome of `Collect`.
- `crates/swell/tests/determinism.rs`: a fixed corpus of slice 1 scripts run by two agents with
  equal limits and seeds in one process, and by two child processes (the test binary started
  again with an environment variable selecting the corpus run), comparing hook calls, outcomes
  and `last_entry_steps()` byte for byte.
- `crates/swell/tests/limits_scripts.rs`: the five tests of Requirements in it, and
  `min_stack_bytes_is_enough_through_builtins`: on a thread with exactly `min_stack_bytes()` for
  `frame_depth` 10,000, `reentry_depth` 1,000 and `parser_nesting` 1,000, a script reaches
  `reentry_depth` re-entries through each of `Function.prototype.call`, a `map` callback,
  `toJSON` and a replacer of `JSON.stringify` (four runs) and parses 1,000 nested brackets at the
  deepest, without a stack overflow. If it fails, the per-level constants of `host/limits.rs`
  are raised to what it measures, with this test named beside them.
- `crates/swell/tests/builtins_scripts.rs`, each citing its clause:
  `map_with_a_script_callback`, `call_and_bind_of_a_script_function`,
  `bound_script_function_constructs_and_has_instance`,
  `to_json_and_replacer_as_script_functions`.

A feature changes no package, so `Cargo.lock` does not change.

Writes: `crates/swell/Cargo.toml` (the feature), `crates/swell/src/host/agent.rs` (the injection
points), `crates/swell/src/host/limits.rs` (the constants, only when
`min_stack_bytes_is_enough_through_builtins` fails),
`crates/swell/tests/{fuzz,misuse,faults,determinism,limits_scripts,builtins_scripts}.rs`,
`conformance/host/requirements.txt` (the fifteen lines).

## Acceptance

`cargo test --locked -p swell` and `cargo test --locked -p swell --features fault-injection
--test faults` exit 0 and run the fifteen tests of Requirements (`HOST-160` among them),
`min_stack_bytes_is_enough_through_builtins` and the four tests of `builtins_scripts.rs`, and
`slot task -d . -t taskfiles/host-requirements.yml host-requirements-check` exits 0 and prints
`host-requirements: 161 requirement(s), 161 line(s): 70 test, 91 pending, 0 omitted; 50
UNMAPPED marker(s), each naming a HOST-<n> or ESS limit` and `host-requirements: pending by
slice: 2:5 3:5 4:9 5:2 6:1 7:1 8:1 9:5 10:14 11:32 12:2 13:2 14:6 16:5 18:1`, with no entry for
slice 1.

## Out of scope

Coverage-guided fuzzing with third-party tools (decision 13: a separate workspace, later);
`HOST-152` to `HOST-159` (their slices); determinism across release platforms (the release
process); fixing a fault in a file of another story (it goes back to that story).

## Domain relations

- Agent to Realm, one-to-many, owns: inferable (`spec/domains/host.yaml`, entity
  `swell.host.Agent`, relation `realms`); a realm handle of another agent is `ForeignHandle`.
- Script to Realm, many-to-one, references: inferable (`spec/domains/host.yaml`, the Script
  entity, relation `realm`); a script of another agent is `ForeignHandle`.
