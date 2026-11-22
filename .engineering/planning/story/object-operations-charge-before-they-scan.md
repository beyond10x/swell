---
format: aep.planning-md/3
id: story:object-operations-charge-before-they-scan
kind: story
status: implemented
title: Setting an array's length, compacting a property or binding list and calling a bound function charge their steps before the scan or the copy
summary: Three operations charge in proportion to their work but after it; each charges first the count it can read before the work, from lengths it holds, and the rest after. No step total changes.
relations:
- depends_on: story:own-keys-charged-before-the-list-exists
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T10:11:57Z", actor: "human:operator", revision: 2, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-22T10:11:58Z", actor: "human:operator", revision: 3, executor: "agent:coordinator"}
- {from: "active", to: "implemented", at: "2026-11-22T17:07:51Z", actor: "human:operator", revision: 4, decided_on: {"recorded":{"test_result":1}}, executor: "agent:coordinator"}
---
Waits for: the budget batch's `story:own-keys-charged-before-the-list-exists`, which writes
`crates/swell/src/object/property.rs` (`keys`, `sort_indices`, lines 479-523) and
`crates/swell/src/object/mod.rs` (other lines); and the merge of the slice-3 unit
`let-const-and-block-scopes`, which modifies `crates/swell/src/env/mod.rs` (this story writes
lines 468-478 of it; after that merge the line numbers are re-read). Batch H2. No requirement of
`host-interface.md`. Consumer: the browser, requirement 2 of its list. Not a gap of proportion
(each is charged within a constant factor of its work); a gap of the rule of decision D2 that work
is charged before it is done.

## Context

Line numbers are of `wave/budget-1` at `ed56b05`.

- ArraySetLength's `truncate_elements` (`crates/swell/src/object/array.rs:444-481`) walks the
  whole property list and collects the index keys at or above the new length (`:452-461`), then
  charges the dense elements passed, the listed keys removed, one for the stop and, for a list of
  more than `SMALL` entries, the list's length (`:466-472`). The list's length
  (`record.properties.len()`, `:449`) is known before the walk; the dense part passed and the keys
  removed depend on the stop the walk finds.
- A property list or a binding list compacts when it is full and half of it is removed
  (`KeyedList::make_room`, `crates/swell/src/object/property.rs:313-330`, the condition at
  `:324-326`, the count at `:328`); the entries visited are charged by the caller after
  `make_room` returned: `add_property` (`crates/swell/src/object/mod.rs:214-250`, the charge at
  `:239-241`) and `add_binding` (`crates/swell/src/env/mod.rs:451-485`, the charge at
  `:474-476`).
- A bound function's chain walk (`walk`, `crates/swell/src/object/bound_function.rs:137`) clones
  each link's bound arguments (`link`, `:108-121`, the clone at `:118`) before it charges a step
  per argument and per link (`:146`).

## Change

Failing tests first (Acceptance). Then:

1. **`truncate_elements`, the exact rule.** For a list of more than `SMALL` entries it charges
   the list's length before the walk, and after the walk the dense elements passed, the listed
   keys removed and one for a stop, as today. For a list of at most `SMALL` entries it charges as
   today, all after the walk, which visits at most `SMALL` entries. The total is today's in both
   cases.
2. **Compaction.** `KeyedList` gains `compaction_visits(&self) -> u64`: the entries `make_room`
   would visit if it compacted now (the length when the list is full and the condition of
   `:324-326` holds, else 0), read from the length, the capacity and `index.removed` without a
   walk. `add_property` and `add_binding` charge it before they call `make_room` and no longer
   charge `room.visited` after; `Room::visited` is removed if nothing else reads it. Totals
   unchanged.
3. **Bound functions.** `walk` reads the number of bound arguments of a link from its record,
   charges it with the link's step, then clones. Totals unchanged.
4. **Stays as it is.** Every result and its order; `SMALL`; the memory charges of `make_room`;
   every step total; determinism.

Design: none changes. No requirement of `host-interface.md` changes.

Writes: `crates/swell/src/object/array.rs` (`truncate_elements`, lines 444-481, a test-only count
of the entries its walk visits under `#[cfg(test)]`, and tests in its `#[cfg(test)]` module,
line 485), `crates/swell/src/object/property.rs` (`compaction_visits` beside `make_room`, lines
313-330, and tests in its `#[cfg(test)]` module, line 534), `crates/swell/src/object/mod.rs`
(`add_property`, lines 234-243, nothing else), `crates/swell/src/env/mod.rs` (`add_binding`,
lines 468-478, and tests in its `#[cfg(test)]` module, line 640),
`crates/swell/src/object/bound_function.rs` (`link` and `walk`, lines 108-150, a test-only count
of the arguments cloned, and a new `#[cfg(test)]` module). Each test-only count is a thread-local
under `#[cfg(test)]`, reset by the test that reads it.

## Acceptance

Unit tests in the lib test binary of `swell`:

- `crates/swell/src/object/array.rs`: `setting_a_length_charges_before_the_walk`: an array with
  100,000 listed index keys above its dense part and 10 steps left; `a.length = 0` ends in
  `Terminated(StepBudget)` and the walk's count reads 0 (today 100,000).
  `truncating_a_small_list_charges_as_today` (a guard): lists of 0, 1, 8 entries and 9 entries
  (`SMALL` is 8), with and without a non-configurable index: the steps of `a.length = n` equal the
  counts the test records from the base and states as constants.
- `crates/swell/src/object/property.rs`: `a_compaction_is_counted_before_it_runs`: a list whose
  length equals its capacity, 65,536, with 32,768 entries removed: `compaction_visits` answers
  65,536, and 0 for the same list with one entry fewer removed than half. Today it does not
  compile. `an_added_property_charges_its_compaction_before_it_runs`: an object whose property
  list is in that state and one step left: `add_property` of a new key ends in
  `Terminated(StepBudget)` and the list's length and `index.removed` are as before (today the
  compaction runs and the charge comes after it).
- `crates/swell/src/env/mod.rs`: `an_added_binding_charges_its_compaction_before_it_runs`: a
  declarative environment whose binding list is in that state, made at the engine level in the
  test, and one step left: `add_binding` ends in `Terminated(StepBudget)` with the list as
  before (today it compacts first).
- `crates/swell/src/object/bound_function.rs`: `bound_arguments_are_charged_before_they_are_copied`:
  a function bound with 100,000 arguments through `Function.prototype.bind.apply`, 10 steps left
  at its call: `Terminated(StepBudget)` and the clone count reads 0 (today 100,000).
- `crates/swell/src/object/array.rs`: `step_totals_are_unchanged` (a guard): `last_entry_steps()`
  of a script that sets lengths over lists of 0, 8, 9 and 1,000 entries, adds properties until a
  list compacts, and calls bound functions of 0, 1 and 1,000 bound arguments through chains of 1
  and 3 links: the constants the test records from the base.

Each test runs under 5 s in both collection modes.

Commands, each one job of the build pool (one slot): `slot cargo nextest run --locked -p swell
--profile default --lib`, `slot cargo nextest run --locked -p swell --profile default --test
<b>` for each `<b>` of `builtins_array_mutation`, `property_index` and `builtins_function`, the
same with `--features collect-every-safe-point`, `slot cargo clippy --workspace --all-targets
--locked -- -D warnings`; at the end, once, `slot timeout 1800 task test262 TEST262_OUT=.out/test262-unit`,
whose `runs.tsv` is compared with that of `slot timeout 1800 task test262 TEST262_OUT=.out/test262-base` run on
the base: `awk -F'\t' '{ print $3, $4, $5 }'` over each, and `cmp` of the two outputs exits 0;
and last, once, in the background, `slot timeout 1800 task host-requirements-check` (it builds every test
target of the workspace). Never `cargo test -p swell` whole, `slot task test-collect`, `slot task
test` or `slot task check`.

Rules this unit keeps. The failing test comes first: each test of the acceptance is written and
seen red before the change, except those named as guards. A test is killed at 60 s and the
gate's test steps fail over 120 s; each test runs in a process of its own under cargo-nextest, so
nothing is shared through a `static` between tests (each count is thread-local and reset by its
test). Each new test is timed, and one over 5 s is named in the report with its time. A job of
the build pool asks for one slot; one over 10 minutes is killed, so `host-requirements-check` on a cold tree and the full test262 run are named `slot timeout 1800 <command>`. Determinism: the same script under the same budgets ends the
same way at the same count on every run. Nothing is added to the frame of `Agent::execute`;
`STACK_BYTES_PER_REENTRY` and the stack tests are unchanged. No third-party crate, no unsafe
code.

## Cost

No step more. CPU, in one pool slot, the smallest of three runs: the property-index probes
`index-write-read` and `named-write-read` of `crates/swell/tests/probes/property-index/` at most
1.05 times the base.

## Report

The new tests with their times, the probe numbers, the test262 comparison, and this line for the
changelog's "Changed" heading (no name of the host interface changes): "Setting an array's
length, compacting a property or binding list, and calling a bound function charge their steps
before the work; the totals are as before."
