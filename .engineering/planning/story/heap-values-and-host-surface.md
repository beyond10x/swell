---
format: aep.planning-md/3
id: story:heap-values-and-host-surface
kind: story
status: implemented
title: The heap, values and the host surface that needs no realm, with the check that a test line names a test
summary: Arenas, accounting, collector, roots, strings and atoms, the keyed map; Agent::new, Limits, Seeds, Cx, host objects, InterruptHandle; the swell crate layout; 17 HOST lines to test; the gate checks test names.
relations:
- decomposes: epic:slice-01-harness
- depends_on: story:num-and-unicode-early-parts
- depends_on: story:host-requirements-pending
scope:
- confidence: cited
  path: conformance/host/requirements.txt
- confidence: cited
  path: crates/swell/Cargo.toml
- confidence: cited
  path: crates/swell/src
- confidence: cited
  path: crates/swell/tests/budgets.rs
- confidence: cited
  path: crates/swell/tests/collect.rs
- confidence: cited
  path: crates/swell/tests/host_objects.rs
- confidence: cited
  path: crates/swell/tests/interrupt.rs
- confidence: cited
  path: crates/swell/tests/source_scan.rs
- confidence: cited
  path: crates/swell/tests/support/host.rs
- confidence: cited
  path: crates/swell/tests/support/mod.rs
- confidence: cited
  path: crates/swell/tests/support/script.rs
- confidence: cited
  path: crates/swell/tests/values.rs
- confidence: cited
  path: taskfiles/host-requirements.yml
- confidence: cited
  path: tests/gate/host-requirements-refuses-a-test-line-naming-a-missing-test.sh
- confidence: cited
  path: tests/gate/host-requirements-refuses-a-test-line-while-tests-are-unchecked.sh
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T04:49:58Z", actor: "human:operator", revision: 5, decided_on: {"recorded":{"review_outcome":1}}, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-21T04:49:58Z", actor: "human:operator", revision: 6, decided_on: {"recorded":{"review_outcome":1}}, executor: "agent:coordinator"}
- {from: "active", to: "implemented", at: "2026-11-21T06:13:57Z", actor: "human:operator", revision: 8, decided_on: {"recorded":{"test_result":1,"review_outcome":1,"verification":1}}, executor: "agent:coordinator"}
---
## Context

The engine needs its values, its heap and the part of the host interface that needs no realm and
no code before anything can run (`docs/design/engine.md`, decisions 4, 5, 9 and 13;
`docs/design/host-interface.md`, sections 1 to 4, 12, 13, 15 and 18). This story builds them and
states the layout of the `swell` crate once, so that the later stories of the slice land in
files of their own.

What the heap is: arenas of slots per kind with `(index, generation)` handles that carry the
agent key (decision 4, interface section 4); the memory budget charged through one accounting
function, with `Vec::try_reserve` for growth (decision 4, "Memory budget"); a precise mark and
sweep collector run only at safe points or when the host asks, triggered by bytes, never by time
(decision 4, "Collector"; the epic leaves open whether the collector is in slice 1: it is, since
`collect` and the `HOST-28` to `HOST-30`, `HOST-109` to `HOST-111` and `HOST-132` requirements
of slice 1 need it); the root table with its release queue; strings in Latin-1 or UTF-16 form,
property keys interned as atoms (decision 5); hash maps keyed through the SipHash of
`swell-num` with `Seeds::hash_key` (`HOST-145`).

This story is the first to move a requirement line from `pending` to `test`, so it also delivers
what `story:host-requirements-pending` leaves to it: the check that the test a `test` line names
exists.

Epic acceptance items held here: 9 (an allocation over the memory budget ends the run with
`Terminated(MemoryBudget)`: `memory_budget_ends_with_cx_in_memory_budget`; `HOST-10` of
`story:host-interface-hardening` reaches it from a script), 11 (a handle whose generation does
not match its slot ends the run with `Terminated(EngineFault)`, and every later call answers
`Err(HostError::Faulted)`) and 14 (the size of `Value`).

## Requirements

Each line of `conformance/host/requirements.txt` below moves from `HOST-<n> pending slice 1` to
`HOST-<n> test <name>`, with the test name given here:

- `HOST-1` `swell::source_scan::host_1_forbid_unsafe_no_outside_package_no_thread_spawn`
- `HOST-2` `swell::source_scan::host_2_no_ambient_authority_in_engine_crates`
- `HOST-146` `swell::source_scan::host_146_only_agent_new_takes_seeds`
- `HOST-136` `swell::doc::compile_fail::host_136_agent_is_not_send` (with the static assertion
  that `InterruptHandle` is `Send` in the same file)
- `HOST-94` `swell::interrupt::host_94_interrupt_handle_moves_into_a_thread`
- `HOST-95` `swell::interrupt::host_95_interrupt_on_a_faulted_agent_returns`
- `HOST-28` `swell::collect::host_28_an_object_held_only_by_a_root_survives_collect`
- `HOST-29` `swell::collect::host_29_dropping_the_last_root_lowers_heap_bytes`
- `HOST-132` `swell::collect::host_132_report_matches_heap_bytes`
- `HOST-32` `swell::values::host_32_to_utf16_keeps_lone_surrogates`
- `HOST-33` `swell::values::host_33_lossy_string_replaces_lone_surrogates`
- `HOST-34` `swell::values::host_34_new_string_is_utf16_of_the_str`
- `HOST-108` `swell::host_objects::host_108_host_data_checks_brand_and_type`
- `HOST-109` `swell::host_objects::host_109_trace_keeps_listed_values`
- `HOST-110` `swell::host_objects::host_110_trace_of_a_stale_handle_faults_collect`
- `HOST-111` `swell::host_objects::host_111_host_data_is_dropped_with_its_object`
- `HOST-145` `swell::lib::heap::map::tests::host_145_hash_is_keyed_by_hash_key_alone`

## Change

Failing tests first: the seventeen tests above, `value_is_16_bytes`, `mark_phase_calls_every_site_of_the_layout`,
`stale_generation_faults_the_agent`, `memory_budget_ends_with_cx_in_memory_budget` and
`tests/gate/host-requirements-refuses-a-test-line-naming-a-missing-test.sh` are written before
the code; the Rust ones fail to build, the script fails because the task accepts any `test` line.

Layout of `crates/swell/src/`, stated once here. This story creates every file, the ones it does
not fill as a module with its doc comment naming the clause or decision and the story that fills
it, so no later story edits `lib.rs` or a `mod.rs` to add a module:

- `lib.rs` (the public interface: `pub use` of `value::*` and `host::*` only, by glob, so a
  type becomes public by being `pub` in `value.rs` or in a file `host/mod.rs` re-exports by
  glob, and no later story edits `lib.rs`; every engine-internal item is `pub(crate)`, which
  `HOST-146` scans), `compile_fail.rs` (`compile_fail` doc tests),
  `value.rs` (`Value`, `PropertyKey`, the typed refs, and the root wrappers `Realm`, `Script`,
  `Module` with `RealmRef`, `ScriptRef`, `ModuleRef` of section 4; 6.1).
- `heap/`: `mod.rs`, `arena.rs`, `accounting.rs`, `collect.rs`, `roots.rs`, `string.rs` (strings
  and atoms), `map.rs` (the keyed map). This story.
- `host/`: `mod.rs` (`Cx`, `HookCx`, `with_cx`, and `pub use` by glob of every other file of
  `host/`, so `Job`, `LoadRequest` and the hook types `story:object-model-and-realms` adds to
  `hooks.rs` are public without an edit of `mod.rs` or `lib.rs`), `agent.rs` (`Agent`,
  `AgentState`, the reads), `limits.rs` (`Limits`, `Seeds`, `CreateError`), `outcome.rs`
  (`Outcome`, `Parsed`, `Abrupt`, `Termination`, `HostError`, `CollectReport`), `interrupt.rs`,
  `host_object.rs` (`Brand`, `HostData`, `Tracer`, `HostObjectSpec`), `hooks.rs` (the `Host`
  trait). This story; `hooks.rs` holds the trait with no method until
  `story:object-model-and-realms`.
- `object/`: `mod.rs` (the internal-method dispatch), `property.rs`, `ordinary.rs`, `array.rs`,
  `string.rs`, `builtin_function.rs` (10.1 to 10.4 but bound and ECMAScript functions):
  `story:object-model-and-realms`, which also writes the dispatch arms of the two function kinds
  that follow. `bound_function.rs` (10.4.1): filled by `story:harness-builtins`.
  `ecmascript_function.rs` (10.2): filled by `story:interpreter-and-script-entry-points`.
- `ops/`: `mod.rs`, `conversion.rs` (7.1), `testing.rs` (7.2), `objects.rs` (7.3),
  `operators.rs` (the runtime semantics of the operators of 13): `story:object-model-and-realms`.
- `realm/`: `mod.rs` (9.3), `intrinsics.rs`, `global_object.rs`: `story:object-model-and-realms`.
- `env/`: `mod.rs`, `declarative.rs`, `object.rs`, `global.rs`, `function.rs` (9.1);
  `interp/`: `mod.rs`, `frame.rs`, `dispatch.rs`, `budget.rs`; `script.rs` (16.1):
  `story:interpreter-and-script-entry-points`.
- `builtins/`: `mod.rs`, `global.rs` (19), `error.rs` (20.5), `boolean.rs` (20.3), `number.rs`
  (21.1): `story:object-model-and-realms`; `object.rs` (20.1), `function.rs` (20.2), `array.rs`
  (23.1), `string.rs` (22.1), `math.rs` (21.3), `json.rs` (25.5): `story:harness-builtins`. One
  module per clause, as decision 2 says; later slices add files here.

What this story fills:

- `Agent::new`, `Limits` (all seven fields; `min_stack_bytes` is
  `story:interpreter-and-script-entry-points`), `Seeds`, `state()`, `heap_bytes()`, `collect()`,
  `interrupt_handle()`, `with_cx`, `host()`, `host_mut()`; `Cx` and `HookCx` with `root`, `get`,
  `duplicate`, `new_string`, `new_string_utf16`, `to_utf16`, `to_rust_string_lossy`,
  `new_host_object`, `brand`, `host_data`, `host_data_mut` (interface sections 4 and 15).
  `HostObjectSpec` has `prototype`, `brand` and `data`; `overrides` and `is_html_dda` are
  added by the slices of their requirements (9 and 18).
- The agent key, the generation check on every handle (a mismatch inside an entry ends it in
  `Terminated(EngineFault)` and faults the agent; a stale or foreign handle given by the host is
  `Err(HostError::StaleHandle)` or `Err(HostError::ForeignHandle)`).
- The accounting function: every allocation and growth charged, a sweep credits; over
  `memory_bytes` the allocation ends the run in `Terminated(MemoryBudget)`.
- The collector: precise mark and sweep. The mark phase is a loop over an explicit worklist (no
  Rust recursion), fed by two kinds of sites, both defined here once and called from
  `heap/collect.rs` in a fixed order:
  - a trace function per heap kind, in the kind own file, with one signature
    (`fn trace(&self, mark: &mut Mark)`; `Mark` reports a value, an object or another handle and
    is the `Tracer` that `HostData::trace` receives);
  - a root call site per module that owns roots (`fn trace_roots(agent, mark: &mut Mark)`).
  A handle a site reports that is stale ends the collection in `Terminated(EngineFault)` and
  faults the agent. This story creates every site of the layout; where its owner comes later the
  site reports nothing, which is right because no such record or object can exist before its
  story lands. The sites, with the story that fills each:

  | site | file | filled by |
  |---|---|---|
  | root: the root table | `heap/roots.rs` | this story |
  | root: handles of the running `with_cx`, host function and hook scopes | `host/agent.rs` | this story |
  | root: agent-wide values (the well-known symbols) | `realm/mod.rs` | `story:object-model-and-realms` |
  | root: slots of running builtins (values a builtin keeps across a call into code) | `object/builtin_function.rs` | `story:object-model-and-realms` |
  | root: the frame stack (operand stacks, locals, environments, function and realm of each frame) | `interp/mod.rs` | `story:interpreter-and-script-entry-points` |
  | kind: strings and atoms (no handle) | `heap/string.rs` | this story |
  | kind: objects, the common record (prototype; properties once they exist) and the per-kind marking dispatch | `object/mod.rs` | this story (prototype, the host object arm), then `story:object-model-and-realms` (properties, one marking arm per object kind beside the call arms) |
  | object kind: host object (`HostData::trace`) | `host/host_object.rs` | this story |
  | object kinds: ordinary (with the Boolean and Number wrappers), Array, String (`[[StringData]]`), built-in function (`[[Realm]]`, `[[InitialName]]`) | `object/ordinary.rs`, `object/array.rs`, `object/string.rs`, `object/builtin_function.rs` | `story:object-model-and-realms` |
  | object kind: bound function (target, `this`, arguments) | `object/bound_function.rs` | created by `story:object-model-and-realms` with its call arms, filled by `story:harness-builtins` |
  | object kind: ECMAScript function (environment, realm, script, home object) | `object/ecmascript_function.rs` | created by `story:object-model-and-realms`, filled by `story:interpreter-and-script-entry-points` |
  | kind: realm record (intrinsics, global object, global `this`, global environment) | `realm/mod.rs` | `story:object-model-and-realms` |
  | the global environment a realm holds (`RealmGlobalEnv`, its constructor and its trace) | `env/global.rs` | `story:interpreter-and-script-entry-points` (the realm record of `story:object-model-and-realms` holds the field and calls both) |
  | kind: environment records | `env/mod.rs` | `story:interpreter-and-script-entry-points` |
  | kind: script records (realm, code constants) | `script.rs` | `story:interpreter-and-script-entry-points` |

  Later slices add their sites the same way (the job queue in slice 10, the module map in 11, the
  kept objects of 9.10 in 16). Host data is dropped by the sweep that frees its object.
  The mechanism is tested here with a kind and a root of its own: `HOST-28` (a handle the root
  table reports survives), `HOST-109` (a handle a trace function reports survives, one it does not
  report is freed), `HOST-110` (a stale handle from a trace function is `Terminated(EngineFault)`)
  and the unit test `heap::collect::tests::mark_phase_calls_every_site_of_the_layout` (a counting
  build of the mark phase calls each root site and each kind trace function of the table once per
  collection).
- `InterruptHandle` (`interrupt`, `clear`, `is_set`, `Send` and `Sync`); the checks that read it
  are `story:object-model-and-realms` and `story:interpreter-and-script-entry-points`.
- Unit tests in the lib target: `heap::arena::tests::stale_generation_faults_the_agent`,
  `value::tests::value_is_16_bytes` (the epic, item 14: an assertion on
  `std::mem::size_of::<Value>()`; if it is not 16, the test states the measured size and the design
  line "expected 16 bytes" is corrected in the same change), `heap::map::tests::host_145_...`.
  Integration test files `crates/swell/tests/{source_scan,interrupt,collect,values,host_objects,
  budgets}.rs`; `budgets.rs` holds `memory_budget_ends_with_cx_in_memory_budget` (a `with_cx`
  whose `new_string` of 2,000 code units passes a `memory_bytes` of 1,024).
- `crates/swell/tests/support/mod.rs`, declaring `pub mod host;` (a test `Host` that records every
  hook call, and the `Limits` and `Seeds` the tests share) and `pub mod script;` (empty here,
  filled by `story:interpreter-and-script-entry-points`); `tests/support/host.rs`,
  `tests/support/script.rs`.
- `crates/swell/Cargo.toml`: dependency on `swell-num` (path); the feature
  `collect-every-safe-point` (off by default), which makes every safe point collect (decisions 4
  and 13). Safe points here are the entry and re-entry boundaries (`with_cx`, a host function, a
  hook, `Cx::call` once `story:object-model-and-realms` adds it); the interpreter adds instruction
  boundaries.

Test names in `requirements.txt`: a `test` line reads `HOST-<n> test <package>::<target>::<test
path>`, where `<target>` is `lib`, the stem of a file under `tests/`, or `doc`, and `<test path>`
is the path `cargo test -- --list` prints for that target (for a doc test, the item path without
the file and line). `taskfiles/host-requirements.yml` gains the task
`host-requirements-test-list`, which writes every test of the workspace in that form to
`.out/host-requirements/tests.txt` (one `cargo test --locked --workspace --no-run` and one
`-- --list` per target), and `host-requirements-check` gains the variable `HOST_REQ_TEST_LIST`
(default that file; the task depends on `host-requirements-test-list`, a task of the same file,
when the variable is not given), loses the fault `a test line, and test lines are not checked
yet`, and gains the fault `<id>: test <name> is not a test of the workspace`. A name ending in
`*` matches every test with that prefix and needs at least one (`HOST-124` has one test per
hook; `story:object-model-and-realms` states their number). The summary of the task says so. The
script `tests/gate/host-requirements-refuses-a-test-line-while-tests-are-unchecked.sh` is
replaced by `tests/gate/host-requirements-refuses-a-test-line-naming-a-missing-test.sh` (a valid
set whose test list lacks the named test: exit 1 and that line). The coordinator adds
`HOST_REQ_TEST_LIST` to `GATE_VARIABLES` in the root `Taskfile.yml`.

`Cargo.lock`: the unit regenerates it in its own tree (the new path dependency), because its
build needs it, and runs its acceptance with `--locked` against that lock; the coordinator does
not take the unit lock into the wave branch but regenerates it there after the merges of the
wave.

Writes: `crates/swell/Cargo.toml`, `crates/swell/src/` (every file of the layout above),
`crates/swell/tests/source_scan.rs`, `crates/swell/tests/interrupt.rs`,
`crates/swell/tests/collect.rs`, `crates/swell/tests/values.rs`,
`crates/swell/tests/host_objects.rs`, `crates/swell/tests/budgets.rs`,
`crates/swell/tests/support/mod.rs`, `crates/swell/tests/support/host.rs`,
`crates/swell/tests/support/script.rs`, `conformance/host/requirements.txt` (the seventeen lines),
`taskfiles/host-requirements.yml`, `tests/gate/host-requirements-refuses-a-test-line-naming-a-missing-test.sh`
(new), `tests/gate/host-requirements-refuses-a-test-line-while-tests-are-unchecked.sh` (removed),
`Cargo.lock` (regenerated in the unit tree; the wave branch lock is regenerated by the
coordinator).

## Acceptance

`cargo test --locked -p swell` and `cargo test --locked -p swell --features
collect-every-safe-point --test collect --test host_objects` exit 0 and run the seventeen tests
of Requirements, `value_is_16_bytes`, `stale_generation_faults_the_agent`,
`mark_phase_calls_every_site_of_the_layout` and
`memory_budget_ends_with_cx_in_memory_budget`; `slot task -d . -t taskfiles/host-requirements.yml
host-requirements-check` exits 0 and prints `host-requirements: 159 requirement(s), 159 line(s):
17 test, 142 pending, 0 omitted; 50 UNMAPPED marker(s), each naming a HOST-<n> or ESS limit` and
`host-requirements: pending by slice: 1:51 2:5 3:5 4:9 5:2 6:1 7:1 8:1 9:5 10:14 11:32 12:2 13:2
14:6 16:5 18:1`; and `sh tests/gate/host-requirements-refuses-a-test-line-naming-a-missing-test.sh`
exits 0 and prints `host-requirements-refuses-a-test-line-naming-a-missing-test: ok`.

## Out of scope

Objects with internal methods, realms, the hooks and their defaults
(`story:object-model-and-realms`); code, frames, the step budget, the depth bounds and the
interrupt checks (`story:interpreter-and-script-entry-points`); `Overrides` and their invariant
checks (slice 9), `is_html_dda` (slice 18), weak references (slice 16). The root `Cargo.toml`,
`Taskfile.yml`, `AGENTS.md`, and the `Cargo.lock` of the wave branch (the coordinator).

## Domain relations

- Agent to Realm, one-to-many, the agent owns its realms: inferable (`spec/domains/host.yaml`,
  entity `swell.host.Agent`, relation `realms`, `owns`, `many`, via `agent_id`). Here: every
  handle carries the agent key, and a handle of another agent is `ForeignHandle`.
- Root to value, one root holds one value until released: inferable
  (`docs/design/host-interface.md`, section 4, the `Root` paragraph; decision 4 of `engine.md`).
- Host object to host data, at most one `HostData` per object, owned by the object and dropped
  with it: inferable (`docs/design/host-interface.md`, section 15, `HostObjectSpec::data`).
- Agent to InterruptHandle, one agent, any number of clones of its handle: inferable
  (`docs/design/host-interface.md`, section 12).
