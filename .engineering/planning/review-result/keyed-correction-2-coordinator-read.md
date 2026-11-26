---
format: aep.planning-md/3
id: review-result:keyed-correction-2-coordinator-read
kind: review-result
status: active
title: 'Coordinator''s read of correction 2 of the unit keyed: no defect, pass 2 closed'
relations:
- reviews: story:keyed-entry-table
revision: 1
---
Coordinator's read of correction 2 of the unit keyed of wave big-2
(`impl/keyed`, `61a171bf..763d7bf6`, one file, `crates/swell/src/object/keyed_entries.rs`,
306 lines added, 5 removed), 2026-11-26. The adversary's budget of two passes is used; this
read stands in for a third pass.

## What was read

Every changed line outside `mod tests` (4 hunks: the module documentation, the doc comment
of `CursorId`, the doc comment and the body of `open_cursor`), and of the four tests the
whole of `the_generation_counter_ends_the_run_at_its_last_value` and the head of
`an_id_stays_stale_through_reuse_compaction_clear_and_a_remade_list` and of
`a_compaction_at_every_cursor_position_and_every_budget_matches_the_model`. The four tests
are the adversary's own cases of pass 2 under permanent names; the adversary ran them green
on `61a171bf`.

## The seven findings of pass 2

1. The end of the generation counter has its test: the test module sets `next_generation` to
   `u64::MAX - 1`, the open after it takes the last generation, and the next open, by a push
   and through the free list, each on an agent of its own, ends the run with the bytes of the
   heap, the count of open cursors, the list, the free list and the counter as they were. A
   `wrapping_add` fails there.
2. The kind of termination: `open_cursor` answers `Err(agent.fault())`, which is
   `Terminated(EngineFault)` and marks the agent faulted, the form `Agent::create_realm` uses
   at the end of its counter and the form the table's BigInt key already used. The unit put
   the old line back once and showed the test red (`left: Err(Terminated(MemoryBudget))`).
   The step of the refused open is charged before the fault; the test reads bytes, not steps,
   and a faulted agent runs nothing more, so the step is of no account.
3. The free-list path that does not advance the counter: shown red by the unit at
   `assert_ne!(t, u)` with the mutant planted, and taken back.
4. `clear` counting live entries only: shown red by the unit (`left: 1`, `right: 16`) with
   the mutant planted, and taken back.
5. The wording of `story:keyed-entry-table` (Bounds) and of one test text of
   `story:map-and-set`: changed by the coordinator in the store (a step per place of the
   cursor list). The numbers of the tests those texts name do not change.
6. An id of one table used on another: no caller can do it (`story:map-and-set`: an iterator
   holds its collection and its id). The doc comment of `CursorId` now says an id is good for
   the table that made it only.
7. The two u32 limits have no test: filed once for the engine,
   `story:a-limit-of-an-integer-width-has-a-test-or-a-stated-reason`, a backlog draft, with
   the four older sites the adversary named.

## What the read found

- No defect in the lines read.
- No `#[allow`, no `#[ignore` and no name that starts with `adversary_` in the file.
- The unit ran the story's filter in both modes and clippy before its `rustfmt` pass over
  the file, and only the module's tests in the default mode after it. The coordinator runs
  the story's filter in both modes, the gate's clippy step and the format check again on
  `763d7bf6` before the merge and records the result as verification.

## Verdict

The correction closes findings 1 to 4 and 6 of pass 2; 5 and 7 are closed in the store.

```findings
[]
```
