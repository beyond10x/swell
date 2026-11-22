---
format: aep.planning-md/3
id: story:a-terminated-array-length-write-leaves-the-array-as-it-was
kind: story
status: implemented
title: A termination that ends ArraySetLength before its first deletion leaves the Array's old length
summary: ArraySetLength writes the new length before its deletions can be terminated; on such a termination it puts the old length back, so that an Array's length is never below an index key it still has. The design states this one exception to "a termination leaves what was changed". No step total changes.
relations:
- decomposes: epic:slice-06-bindings-and-iteration
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T13:46:15Z", actor: "human:operator", revision: 2, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-22T13:46:16Z", actor: "human:operator", revision: 3, executor: "agent:coordinator"}
- {from: "active", to: "implemented", at: "2026-11-22T17:07:49Z", actor: "human:operator", revision: 4, decided_on: {"recorded":{"test_result":1}}, executor: "agent:coordinator"}
---
Waits for: nothing. No requirement of `host-interface.md`. Consumer: none named. The commit
`a34ea7c` named this gap in its last paragraph. An Array exotic object keeps every array index
below its `length` (`#sec-array-exotic-objects`); a termination is not observable by the script
it ends, but the host may run a later script on the same agent, which then reads the Array.

## Context

Line numbers are of `wave/budget-1` at `a6815ca`; they are read again on the unit's base.

- `array_set_length` (`crates/swell/src/object/array.rs:427-476`) is ArraySetLength
  (`#sec-arraysetlength`). Step 15 writes the new length (`:454`) with `[[Writable]]` true
  (`:450-453`; the old one was true, else `:447-449` answered false). Step 17 runs in
  `truncate_elements` (`:459`, defined at `:493-510`) and `truncate_listed` (`:515-555`).
- `truncate_elements` can end the run at three points: the step charge of the list's length
  (`:495-497`), the charge of the list's bytes (`:500`, and the failed reservation at
  `:502-505`), and, in `truncate_listed`, the step charge after the search (`:545`). Each step
  charge also reads the interrupt (`Agent::charge_steps`, `crates/swell/src/host/agent.rs:799-810`).
  All of them come before the first change of the Array: the dense truncation (`:546-549`) and
  the removals (`:550-553`).
- A termination at any of them returns through the `?` at `:459` and leaves the length lowered
  with every index key at or above it in place. The test
  `the_list_of_deleted_keys_is_charged_before_it_is_made` (`array.rs:1089-1164`) says so in its
  comment (`:1097-1098`) and asserts nothing about it.
- `docs/design/engine.md:623` says "A termination closes nothing: no code runs after it,
  `return` included." The design states no rule on what a terminated builtin leaves; builtins
  leave what they had changed.

## Change

Failing tests first (Acceptance). Then:

1. **`array_set_length`.** When `truncate_elements` ends in `Abrupt::Terminated`, it writes the
   old length back through the same `ordinary_define_own_property` as step 15 and returns the
   termination. The write changes the value of an existing property: `ordinary.rs:202-211` is
   the only place that path adds a property, and it is not reached; the old `[[Writable]]` was
   true, so the value is all there is to put back. No step and no byte is charged by it. The
   Array is then as it was before the call: the old length and every element.
2. **`truncate_elements`.** Its doc (`:478-492`) states that every charge comes before the first
   change of the Array, which the restore relies on; the code keeps that order.
3. **Not taken:** charging the search before step 15. A step 15 that answers false (a descriptor
   ValidateAndApplyPropertyDescriptor refuses, `:454-456`) would then pay the search's steps, and
   a step total would change.
4. **The design.** After the sentence of `docs/design/engine.md:623`, one sentence: "A
   termination leaves what a builtin had changed until then, with one exception: ArraySetLength
   puts the old length back when a termination ends its deletions, so that an Array's `length`
   is never below an index key it still has (`object/array.rs`)."
5. **Stays as it is.** Every result and its order; every step total; determinism.

No requirement of `host-interface.md` changes.

Writes: `crates/swell/src/object/array.rs` (`array_set_length`, lines 427-476; the doc of
`truncate_elements`, lines 478-492; a new test in its `#[cfg(test)]` module, line 565; the test
at lines 1089-1164) and `docs/design/engine.md` (one sentence after line 623). Test-only hooks:
none new; `TRUNCATION_VISITS` (`array.rs:558-563`, private) is read as it is.

## Acceptance

Unit tests in the lib test binary of `swell`:

- `crates/swell/src/object/array.rs`: `a_terminated_truncation_leaves_the_array_as_it_was`. Four
  Arrays of `listed_array(N = 10,000)` (`:887-914`: dense elements 0 to 2, N index keys from
  2,000 up, a list of N + 1 entries with `"length"`), made global as `a`, `b`, `c` and `d`, with
  the roomy agent of `roomy()` (`:863-871`). The step budgets are a measured reading plus a
  stated room: `S`, the steps of the entry `d.length = 0` measured with `entry_steps`
  (`:986-997`); `L`, the list's length, N + 1, read from the record; `R`, the steps charged
  after the search, 3 + N (the dense positions passed and the keys removed, no stop; the doc of
  `truncate_elements`). `a.length = 0` with `S - R - L / 2` steps ends in
  `Terminated(StepBudget)` at the charge of the list's length; `b.length = 0` with `S - R / 2`
  steps ends in `Terminated(StepBudget)` at the charge after the search. Then, with a roomy
  budget, a script for each of `a` and `b` reads its length, 2,000 + N (today 0); the number of
  its own property names, N + 4 (`Object.getOwnPropertyNames`); and whether every name is
  `"length"` or an index below the length, answered as 1 or 0: 1 (today 0). The script
  `c.length = 0` answers 0, and `Object.getOwnPropertyNames(c).length` answers 1.
- `the_list_of_deleted_keys_is_charged_before_it_is_made` (`:1101-1164`): the comment at
  `:1097-1098` goes, and before the script `c.length` a script `a.length` answers 2,000 + N
  after the memory-budget termination (today 0).
- Guards: `step_totals_are_unchanged` (`:1010-1059`), `truncating_a_small_list_charges_as_today`
  (`:947-982`), `setting_a_length_charges_before_the_walk` (`:921-940`) and
  `array_length_truncates_and_refuses_non_writable` (`:792`) pass as they are.
- The design sentence: `git grep -n 'ArraySetLength puts the old length back' docs/design/engine.md`
  prints one line.

Each test runs under 5 s in both collection modes.

Commands. Each round: `cargo nextest run --locked -p swell --profile default --lib`, and
`cargo nextest run --locked -p swell --profile default --test <b>` for each `<b>` of
`builtins_array_mutation` and `builtins_array`, the same with `--features
collect-every-safe-point`. Once at the end, the whole package in both collection modes:
`cargo nextest run --locked -p swell --profile default --no-fail-fast` and the same with
`--features collect-every-safe-point` (655 tests per mode on `a6815ca`, a count this story did
not make; each mode under 60 s of run time), which holds `termination`, `determinism` and the
stack tests. Then `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo fmt
--all --check`, `cargo nextest run --locked -p swell-test262`, and once
`slot --timeout 30m task test262 TEST262_OUT=.out/test262-unit`, whose `runs.tsv` is compared
whole with that of `slot --timeout 30m task test262 TEST262_OUT=.out/test262-base` run on the
base: `cmp .out/test262-base/runs.tsv .out/test262-unit/runs.tsv` exits 0 (all six fields,
`crates/test262/src/count.rs:309-315`). Never `cargo test -p swell` whole, `slot task
test-collect`, `slot task test` or `slot task check`.

Rules this unit keeps. The failing test comes first: each test of the acceptance is written and
seen red before the change, for the reason stated beside it, except those named as guards. A
test is killed at 60 s and the gate's test steps fail over 120 s in sum; each test runs in a
process of its own under cargo-nextest, so nothing is shared through a `static` between tests.
Each new test is timed, and one over 5 s is named in the report with its time. A job of the
build pool ends after 10 minutes unless it names its time, 30 minutes at most (`slot --timeout
30m`, or `SLOT_TIMEOUT=30m` for `cargo`); a `timeout` wrapped around a command extends nothing.
Determinism: the same script under the same budgets ends the same way at the same count on
every run. No third-party crate, no unsafe code, no Python.

## Cost

No step and no byte more: the write-back runs only after a termination, and nothing else
changes on the path of a length write.

## Report

The new test with its time in both modes, the whole-package runs, the test262 comparison, and
this line for the changelog's "Changed" heading (no name of the host interface changes): "A
termination that ends the deletions of an Array length write leaves the Array's old length, so
its length is never below an index it still has."
