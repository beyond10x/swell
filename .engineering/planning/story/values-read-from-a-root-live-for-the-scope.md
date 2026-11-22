---
format: aep.planning-md/3
id: story:values-read-from-a-root-live-for-the-scope
kind: story
status: active
title: A value read from a root with Cx::get is handed out to the running scope, as a new String is
summary: Cx::get and HookCx::get take &mut self and record the value in the running scope, so it stays valid until the scope ends when the host drops the root and re-enters the engine.
relations:
- depends_on: story:interrupt-read-at-every-parse-entry
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T05:10:14Z", actor: "human:operator", revision: 2, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-22T05:10:14Z", actor: "human:operator", revision: 3, executor: "agent:coordinator"}
---
Waits for: `story:interrupt-read-at-every-parse-entry` (both write
`crates/swell/src/host/mod.rs`). Base: `wave/s3-1` with the gate unit
(`gate-test-steps-within-budget`) and the design text merged, and that story merged.
Precondition: the design text of decision D6 (taken: option A), which the base holds:
`host-interface.md` section 4 ("A value read from a root with `get` is handed out to the running
scope like any other."; `pub fn get(&mut self, root: &Root) -> Result<Value, HostError>;`, and
the paragraph after the block: handing a value out charges nothing).

## Context

Line numbers are of `wave/s3-1` at `c108c2d`.

- `Cx::get` and `HookCx::get` (`crates/swell/src/host/mod.rs:432-435`, one macro for both) call
  `scoped_get` (`:176-179`), which reads the value out of the root table
  (`crates/swell/src/heap/roots.rs:121-126`) and records nothing. `new_string` records what it
  makes in the running scope (`host/mod.rs:181-186`, `Agent::hand_out`,
  `crates/swell/src/host/agent.rs:615-617`), whose values the collector reports
  (`agent.rs:633-641`) until the scope ends (`host/mod.rs:163-165`, and the host function scope
  of `crates/swell/src/object/builtin_function.rs:350-360`).
- A dropped `Root` queues its release (`crates/swell/src/value.rs:74-80`); every safe point
  drains the queue and may collect (`agent.rs:602-612`), and a re-entry runs safe points: one per
  instruction of the run loop (`crates/swell/src/interp/dispatch.rs:43`) and one at each builtin
  or host function call (`builtin_function.rs:284`).
- So a host that reads a value with `cx.get`, drops the only root of it and calls back into the
  engine (`cx.call`) inside the same scope can have the value swept before the scope ends. A
  host function that then returns it fails the check of its answer
  (`builtin_function.rs:363-366`) and the agent is faulted. `docs/design/host-interface.md:327-329`
  says: "A value handed to host code is valid for the scope it was handed in".
- `Cx::realm_ref` (`host/mod.rs:495-498`, through `scoped_realm_ref`, `:296-301`) records
  nothing either, and no root site reports a realm a `RealmRef` names
  (`crates/swell/src/realm/mod.rs:149-151` reports nothing). A realm is held through a `Realm`
  root (section 4, `host-interface.md:329`): once the host drops it and a collection frees the
  realm, the `RealmRef` answers `Err(HostError::StaleHandle)` from every method that takes it
  (`host/mod.rs:310`, `:764`), the generation check keeps it from reaching another record, and
  the agent stays `Alive`. This story does not change that; its test holds it.

## Change

Failing test first (Acceptance). Then:

- `Cx::get` and `HookCx::get` take `&mut self` (decision D6) and record a value that names a
  record (a String, a Symbol, an object) in the running scope through `Agent::hand_out`, as
  `scoped_new_string` does; a primitive is not recorded, since the collector reports none
  (`crates/swell/src/heap/collect.rs:69-77`). No second list, no new root.
- The callers that hold a shared `Cx` and read through it change to `&mut`:
  `crates/swell/tests/support/host.rs:126` (`object`), `crates/swell/tests/misuse.rs:268`
  (`good_values`), `crates/test262/src/host.rs:310` (`realm_of`, which also reads `cx.host()` in
  the same loop and is restructured to take the root it needs first). Every other caller in the
  workspace already holds a `&mut Cx` or `&mut HookCx` (`conformance/ess/src/target.rs:611`
  among them). The gate unit does not change `crates/swell/tests/support/host.rs`.
- Stays as it is: the root table and its release queue; HOST-30 (a released root answers
  `StaleHandle`); HOST-102 (a read changes no reading of the heap: the scope is not charged,
  `engine.md`, "Memory budget"); the generation check; the step counter.
- The scope grows by one entry per `get` of a record while it lasts, as it does per
  `new_string`; it is the host's own call and ends with the scope.

Requirement (section 4, `[slice 6]`): HOST-170, as the design text words it. Its line in
`conformance/host/requirements.txt` is the one the design text added, `HOST-170 pending slice
6`; this story changes only that line (Acceptance).

Writes: `crates/swell/src/host/mod.rs` (`scoped_get` and the `get` of `scoped_methods!`,
nothing else), `crates/swell/src/compile_fail.rs` (one doc test and its twin),
`crates/swell/tests/support/host.rs` (`object`), `crates/swell/tests/misuse.rs`
(`good_values`), `crates/test262/src/host.rs` (`realm_of`), `crates/swell/tests/collect.rs`,
`conformance/host/requirements.txt` (the line of HOST-170). Running units: no file of
`let-const-and-block-scopes` or `template-literals`. Shared: `tests/collect.rs` and
`tests/misuse.rs` with `story:arena-capacity-and-collector-workspace-on-the-ledger` (its tests
of HOST-29, HOST-171 and HOST-102, other functions), and the lines of `requirements.txt` next to
those of stories 1, 3 and 4; the unit that lands second gets the wave branch merged into its
tree and resolves the overlap (`AGENTS.md`, "How the work is run").

## Acceptance

In `crates/swell/tests/collect.rs`:

- `host_170_a_value_read_from_a_dropped_root_lives_until_its_scope_ends`: `with_cx` makes a
  host object nothing else references and roots it; the host keeps the root in its own value. A host
  function `take` reads the object with `cx.get`, drops the root, calls the script function
  `function churn() { return "x".repeat(n).length; }` through `cx.call`, and returns the object;
  `n` is at least twice `heap_bytes()` after the realm was made (a collection starts when the
  bytes since the last one pass the threshold, `crates/swell/src/heap/mod.rs:106-116`), measured
  by the unit. The host evaluates the script `take()`. Today it answers
  `Terminated(EngineFault)` and `state()` reads `Faulted`, in both collection modes. After it
  answers `Normal` with a root that holds that same object (the `ObjectRef` `take` read, read
  back through `cx.get`), `state()` reads `Alive`, and once that root is dropped a following
  `collect()` frees the object (`cx.brand` of it then answers `Err(HostError::StaleHandle)`, as
  the test of HOST-28 reads a freed object).
- `realm_ref_after_the_realm_root_is_dropped` (a guard, green before and after): inside one
  `with_cx` of a first realm, `cx.realm_ref` of a second realm whose only root the host then
  drops, `cx.call` of the `churn` above, then
  `cx.new_error(that realm_ref, RangeError, "m")`: it answers `Err(HostError::StaleHandle)` and
  `state()` reads `Alive`.
- `crates/swell/src/compile_fail.rs`, `hook_cx_get_takes_mut_self`: a `compile_fail` doc test in
  which a function given a `&HookCx<'_, H>` calls `get` on it, with its twin that differs in one
  token (`&mut HookCx`) and compiles, the form of `host_136_agent_is_not_send` there. Today the
  first compiles: red.
- `conformance/host/requirements.txt`: the line `HOST-170 pending slice 6` becomes `HOST-170 test swell::collect::host_170_a_value_read_from_a_dropped_root_lives_until_its_scope_ends`;
  `slot task host-requirements-check` then prints one pending line fewer than on the base.

Commands, each one job of the build pool: `slot cargo nextest run --locked -p swell --profile
default --lib`, `slot cargo nextest run --locked -p swell --profile default --test <b>` for each
`<b>` of `collect`, `misuse` and `calls`, the same with `--features collect-every-safe-point`,
`slot cargo test --locked -p swell --doc` (the doc tests; nextest runs none),
`slot cargo nextest run --locked -p swell-test262 --profile default --lib`, `slot cargo clippy
--workspace --all-targets --locked -- -D warnings`, `slot task conform` and `slot task
conform-engine` (the counts of the base), and at the end, once, `slot task test262
TEST262_OUT=.out/test262-unit`, whose `runs.tsv` is compared with that of `slot task test262
TEST262_OUT=.out/test262-base` run on the base: `awk -F'\t' '{ print $3, $4, $5 }'` over each,
and `cmp` of the two outputs exits 0 (no pass lost in the directories named here). Never `cargo
test -p swell` whole, `slot task test-collect`, `slot task test` or `slot task check`.

Directories: all; the runner's own host (`crates/test262/src/host.rs`) is the one caller in the
run that changes.

Rules this unit keeps. The failing test comes first: each test of the acceptance is written and
seen red before the change, except one named as a guard. A test is killed at 60 s and the
gate's test steps fail over 120 s; each test runs in a process of its own under cargo-nextest,
so no fixture and no counter is shared through a `static` between tests (a counting build
counts within one test's process). Each new test is timed, and one over 5 s is named in the
report with its time. A job of the build pool asks for one slot. Determinism: the same script
under the same budgets ends the same way at the same count on every run. Nothing is added to
the frame of `Agent::execute`; `STACK_BYTES_PER_REENTRY` and the stack tests are unchanged. No
third-party crate, no unsafe code.

## Cost

Bound: the CPU seconds (user + system) of `slot cargo nextest run --locked -p swell --profile
default --test calls` (host functions and `Cx` calls), built first with `--no-run` and measured
with `/usr/bin/time -f '%U %S'` inside the pool job, three runs on the base and three on the
unit, the median of each, rise by at most 2 %. Why 2 %: one push on the scope per `get` of a
record, against the root-table lookup it follows; 2 % is the noise of three runs. Past the bound
the unit reports the three numbers of each side and stops before merging.

## Report

The unit's report gives the new tests with their times, `n` with the command that measured it,
the Cost numbers, and this sentence for the changelog: "Breaking change of the host interface:
`Cx::get` and `HookCx::get` take `&mut self`. The value read is handed out to the running scope
and stays valid until the scope ends, also when the host drops the root and calls back into the
engine; a caller that read through a shared `&Cx` or `&HookCx` takes `&mut` instead."
