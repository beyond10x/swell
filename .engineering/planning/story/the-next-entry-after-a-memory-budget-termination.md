---
format: aep.planning-md/3
id: story:the-next-entry-after-a-memory-budget-termination
kind: story
status: implemented
title: 'The next entry after a run ended in the memory budget: the compile step charges what it holds, and HOST-70 says what the host can rely on'
summary: The compile step charges one node's expansion ten times what it holds, and HOST-70 promises that the script 1 runs after Terminated(MemoryBudget) whatever room is left. The step charges an expansion as it is made, and HOST-70 states the four things a host relies on after that termination.
relations:
- decomposes: epic:slice-06-bindings-and-iteration
- depends_on: story:frontend-scratch-on-the-memory-ledger
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T11:42:55Z", actor: "human:operator", revision: 2, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-22T11:42:55Z", actor: "human:operator", revision: 3, executor: "agent:coordinator"}
- {from: "active", to: "implemented", at: "2026-11-22T17:07:53Z", actor: "human:operator", revision: 4, decided_on: {"recorded":{"test_result":1}}, executor: "agent:coordinator"}
---
Waits for: nothing; everything is in the base, the head of `wave/budget-1` (the frontend's
charges, `story:frontend-scratch-on-the-memory-ledger`, HOST-168 and HOST-169; the host's bytes,
HOST-175 and HOST-176; the charged reading, HOST-171). Consumer: the browser; its embedding runs
one agent per document, calls `collect()` once after `Terminated(MemoryBudget)` and tries the
next entry.

## Context

Numbers are measured on `wave/budget-1` at `7a7bf0a`; measure them again on your base.

- The compile step charges the expansion of a node before it is made, as a bound from the
  number of the node's children (`expansion_bytes`, `crates/compiler/src/lib.rs`, and its use in
  `run`): `(8 × children + 32) × 56` bytes, 1,792 for a leaf, 2,240 for one child, 2,688 for
  two, credited once the node's tasks are on the stack. Each of these expansions takes 224
  bytes. The peak of the parse and compile of the script `1` is 2,411 bytes above its tree
  (881) and its code (144); `1` needs 3,438 bytes of room above `heap_bytes()` to answer, and
  `1 + 1` needs 3,895.
- `crates/swell/tests/frontend_ledger.rs`,
  `the_compile_step_of_a_short_script_charges_near_what_it_holds`, is ignored and red: "peak
  2411 above the tree 881 and the code 144".
- When a script that keeps its objects reachable ends in `Terminated(MemoryBudget)`, the room
  `memory_bytes - heap_bytes()` is anywhere below the growth that was refused; nothing bounds
  it from below. `crates/swell/tests/termination.rs`,
  `host_70_one_runs_after_a_loop_that_keeps_its_objects_ended_in_memory_budget`, is ignored
  and red: with 1,000 bytes of room, `1` answers `Terminated(MemoryBudget)`. This held before
  the frontend was charged too (the script record of `1` is 546 bytes).
- `crates/swell/tests/property_index.rs`, `memory_budget_of_8_mib_ends_a_push_loop`, is red
  with a collection at every safe point: after the loop ended in the memory budget, `1 + 1`
  has 3,549 bytes and needs 3,895. In the default mode it passes on the room the last refused
  growth happened to leave.
- HOST-70 (`docs/design/host-interface.md`): "After `Terminated(StepBudget)`,
  `Terminated(MemoryBudget)` or `Terminated(Interrupted)`, `state()` reads `Alive` and a
  following script `1` answers `Normal(1)`."

## Change

Failing tests first (Acceptance). Then:

1. **The compile step charges an expansion as it is made.** The lists an expansion builds (its
   tasks and the small lists beside them) are charged through the ledger before each growth,
   by what the growth takes, as `run` charges the task stack (`grow_by`), and credited when
   they drop or once their content is on the stack. `expansion_bytes` goes, or stays only as a
   bound that is at most twice what the expansion holds plus 64 bytes. No allocation whose
   size follows the source is made uncharged, and the rule of the design holds: storage is
   charged before it is allocated. The bytes of the finished code and what passes to the
   script record are unchanged.
2. **HOST-70 is stated per termination.** The text keeps its sentence for
   `Terminated(StepBudget)` and `Terminated(Interrupted)`. For `Terminated(MemoryBudget)` it
   states, as one requirement or as HOST-70 and requirements appended after HOST-187 (your
   choice; each sentence a test can show is a requirement with a test):
   a. `state()` reads `Alive`, and every call that takes no memory still answers: reading the
      outcome and values the host holds, dropping roots, `collect()`, `credit_host_bytes`,
      dropping the agent. `collect()` answers `Normal` with a room of 0.
   b. An entry whose parse and script record the room does not cover ends in
      `Terminated(MemoryBudget)` (`Parsed::Terminated(MemoryBudget)` for `parse_script`), the
      same outcome on every run from the same state; `state()` stays `Alive`, and no other
      outcome, no fault and no panic.
   c. The host reads the state without a trial entry: `heap_bytes()` is what is charged
      (HOST-171) and `memory_bytes` is the limit it gave.
   d. After the host made room (roots dropped and `collect()`, or `credit_host_bytes`), an
      entry that the room covers runs as in an agent that never ended in the memory budget:
      the earlier termination leaves no state behind. The script `1` answers `Normal(1)` when
      the room is at least what the text states for it, a number measured after change 1 and
      stated with its test.
   Decision D10 of the same document and its paragraph on the memory budget ("the agent stays
   usable") say the same in prose. `docs/design/engine.md`, "Memory budget", gets one
   sentence: the room after a memory-budget termination has no lower bound, and the host makes
   room before the next entry.
3. **The tests say it.** The ignored test of `frontend_ledger.rs` runs (no `#[ignore]`). The
   ignored test of `termination.rs` is rewritten to the new text: with 1,000 bytes of room
   (or the largest room below what `1` needs after change 1), `1` ends in
   `Terminated(MemoryBudget)` twice in a row with the same `heap_bytes()` after each,
   `collect()` answers `Normal`, and after the host gave back bytes it had charged before the
   loop, `1` answers `Normal(1)`. `memory_budget_of_8_mib_ends_a_push_loop` states its room
   from below in both modes, as
   `an_agent_that_ended_in_memory_budget_runs_again_after_a_collection` does since `3a2dd02`
   (bytes the host holds charged during the loop and gives back after it), and keeps what it
   shows: the loop ends by memory within its time, and the agent then runs `1 + 1`.
   `host_70_agent_stays_alive_after_each_budget` keeps passing; if its memory part passes
   only on the room its sizes leave, it gets the same floor.
4. **Stays as it is.** Every step total; the outcome of every test262 case; determinism; the
   parser; `STACK_BYTES_PER_REENTRY`; the scenarios under `spec/scenarios/` and the model
   under `spec/` (the scenario `memory-budget-ends-one-entry` forces its answers and stays
   true; you edit nothing under `spec/`).

Writes: `crates/compiler/src/` (`lib.rs`, and `statement.rs`, `expression.rs`, `function.rs`
where an expansion builds a list; tests in their `#[cfg(test)]` modules or
`crates/compiler/tests/compile.rs`), `crates/swell/tests/frontend_ledger.rs`,
`crates/swell/tests/termination.rs`, `crates/swell/tests/property_index.rs`,
`docs/design/host-interface.md` (HOST-70, appended requirements, decision D10's paragraph, the
traceability lines that list HOST-70), `docs/design/engine.md` (one sentence),
`conformance/host/requirements.txt` (the line of HOST-70 if its test name changes, and one line
per appended requirement, `test <path>`). No file of `crates/swell/src/` unless a sentence of
2a or 2b does not hold in the engine; then report what does not hold before you change engine
source there, with the test that shows it.

## Acceptance

- `swell::frontend_ledger the_compile_step_of_a_short_script_charges_near_what_it_holds`: not
  ignored, green in both collection modes (red on the base with the message above).
- A new test of the compile step for a wide node (an Array literal of 10,000 elements, or a
  call with as many arguments as the parser admits): the ledger's high-water mark during the
  step is at least what the step's lists hold at that point and at most twice that plus 1 KiB
  (the capacities read in the test), and a ledger that refuses the n-th charge, for every n up
  to the number of charges of a small script, ends the step with every byte credited. Red on
  the base for the upper bound.
- The rewritten test of 2a, 2b and 2d in `termination.rs`, not ignored, green in both modes;
  red on the base in the form that asserts the old text.
- `swell::property_index memory_budget_of_8_mib_ends_a_push_loop`: green in both modes; its
  time is stated (it took 0.5 s with a collection at every safe point on the base).
- The package swell whole in both modes: no red test. `swell-compiler` and `swell-syntax`:
  no red test.
- `slot --local task host-requirements-check` and `slot --local task host-surface-check`: the
  first exits 0; the second names the `HOST-<n>` lines you changed or appended, which is
  expected: give the coordinator the names and one sentence per name for the changelog's
  "Host interface" heading.
- test262: one full run on the base and one on the unit (`slot --timeout 30m task test262
  TEST262_OUT=.out/test262-base` and `-unit`), `awk -F'\t' '{ print $3, $4, $5 }'` over each
  `runs.tsv`, and `cmp` of the two outputs exits 0. A case that changes its outcome is named
  with both outcomes, not hidden.

Each new or changed test runs under 5 s in both modes; one that does not is named with its
time. A test is killed at 60 s, and the gate's test steps fail over 120 s in sum.

## Cost

CPU of the compile step: the test binary `language` of swell, user plus system time, seven
runs alternating with the base, the median at most 1.03 times the base's. The script that
measures starts without the `slot` prefix.

## Report

The red and the green run of each test, the room `1` and `1 + 1` need after the change, the
text of HOST-70 and of each appended requirement as written, the names `host-surface-check`
asks for with one sentence each, the test262 comparison, the CPU numbers, and this line for the
changelog's "Changed" heading, corrected to what you built: "The compile step charges what a
node's expansion holds as it is made (a short script needed 2.4 KB more room at its peak
before). After `Terminated(MemoryBudget)` the agent stays `Alive`; the next entry runs when the
free room covers it and ends in `Terminated(MemoryBudget)` again when it does not, so a host
makes room first."
