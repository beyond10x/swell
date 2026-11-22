---
format: aep.planning-md/3
id: story:heavy-tests-far-from-the-kill
kind: story
status: implemented
title: The heaviest tests of the gate and of the lane slow stay far from the 60 s kill
summary: Measured per test in CPU seconds, alone, in the four runs; the tests over 1.5 s (gate, default mode), 2 s (gate, collect mode) and 5 s (lane slow at full counts) get smaller counts where the size is not the property, and a gate count where it is; one gate stack bound is scaled the way sources.rs scales it.
relations:
- decomposes: epic:slice-06-bindings-and-iteration
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T20:12:58Z", actor: "human:operator", revision: 2, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-22T20:12:59Z", actor: "human:operator", revision: 3, executor: "agent:coordinator"}
- {from: "active", to: "implemented", at: "2026-11-22T20:27:24Z", actor: "human:operator", revision: 4, decided_on: {"recorded":{"test_result":1}}, executor: "agent:coordinator"}
---

# The heaviest tests of the gate and of the lane slow stay far from the 60 s kill

## Context

The organization's rule "Tests stay fast": a test near the 60 s kill of cargo-nextest is a
defect, and raising a timeout, adding shards or waiting longer is not a fix.

On 2026-11-22 the machine ran at a load of 110 to 166 on 18 cores, and tests that pass on a quiet
machine were killed at 60 s:

- gate, step `test`: `frontend_ledger::eval_and_function_of_a_flat_source_end_in_memory_budget`
  killed at 60.2 s once (23.1 s in another run, 16.1 s alone at that load);
- lane `slow`, default half: 11 killed, none a failed assertion: the eight parts of
  `fuzz::host_3_no_entry_point_read_or_hook_panics` (46.6 to 54.9 s each at a load of about
  70), `property_index::the_same_seeds_give_the_same_steps_and_heap_bytes`,
  `tracing::operand_stack_and_local_values_survive_collect` and the library test
  `ops::objects::tests::every_exit_of_a_for_in_credits_its_key_list::normal_end`;
- lane `slow`, collect half, at a load of about 100: the two `dynamic_function::step_budget_*`
  tests 50.5 and 48.4 s, `builtins_json_parse::reviver_on_100000_nested_arrays_ends_in_a_result_or_a_budget`
  36.5 s, `budgets::catching_the_depth_error_and_recursing_again_ends_in_step_budget` 36.0 s,
  the three `eval::step_budget_ends_an_endless_*` 29.7 to 33.7 s.

Wall time at that load is ten to twenty times the CPU time; what a test costs is its CPU time.
Measured per test, alone (the test binary run directly with `--exact`, user + system seconds),
on the base `ea0d899`:

- gate, default mode, over 1.5 s: `sort_is_stable_for_11_513_and_2048_elements` 1.60,
  `parse_of_100000_nested_brackets_ends_in_a_result_or_the_memory_budget` 1.60,
  `the_properties_of_a_large_pattern_are_charged_as_they_are_read` 1.57;
- gate, collect mode, over 2 s: `host_133_same_calls_same_hooks_same_outcomes` 4.34, the
  `language` parts 3, 9 and 8 at 2.52, 2.16 and 1.91 to 2.20;
- lane, default mode, over 5 s: the eight fuzz parts 7.7 to 8.5, `operand_stack_and_local_values_survive_collect`
  11.3 to 11.5;
- lane, collect mode, over 5 s: the five `step_budget_*` tests of `eval` and
  `dynamic_function` 17.5 to 18.1, `catching_the_depth_error_and_recursing_again_ends_in_step_budget`
  18.4, fuzz parts 3 and 4 14.7 and 16.8 (seeds 55 and 58 alone 13 and 16),
  `step_budget_ends_a_loop_of_lookups_by_name` 12.9 to 13.4, `min_stack_bytes_is_enough` 12.2
  to 12.9, `host_17_deepest_reentry_parses_at_full_nesting` 9.0 to 9.5, the reviver result
  test 7.3 to 7.7, `sort_with_a_comparator_that_throws_or_lies_ends` 5.9 to 6.0,
  `host_133_same_calls_same_hooks_same_outcomes` 5.7.

The whole lane collect run on the base: 395 tests run, 391 passed, 4 timed out (two `eval`
step tests, fuzz parts 3 and 4), at a load of 68 to 63.

## Change

Each test keeps what it shows; the counts are named in the test's documentation with the
measurement.

- `fuzz.rs`: the full count is 1,000 sequences in the default mode (was 2,000) and 50 in the
  collect mode (was 150); the gate's 160 and 15 stay. 2,000 and 150 came from the story
  `host-interface-hardening` and the time of one pool job, not from a defect found. 1,000
  sequences, 125 a part, draw each kind of the generator about 700 times. In the collect mode
  the seeds 55 and 58 cost 13 and 16 CPU s each; the first 50 take 4.2 together. A sequence
  still fails on a panic, an abort, a run past its budget or a transcript that differs.
- `tracing.rs`: the lane's garbage loop is 25,000 iterations (was 100,000): 11,600,000 bytes a
  loop, eleven times the collection threshold; `garbage_passes_the_threshold_and_the_budget`
  still checks that the count passes the threshold and the budget.
- `eval.rs`, `dynamic_function.rs`, `budgets.rs`, `names.rs`: with the collector at every safe
  point the step budget is 2,000 in the lane too (was 100,000 there). The scripts loop or
  recurse without end, so every budget ends them in `Terminated(StepBudget)`, which the tests
  assert; 2,000 still reach the `RangeError` at `frame_depth` 20 (budgets) and run past the
  name loop's 1,000 (names).
- `budgets.rs` (`min_stack_bytes_is_enough`), `limits_scripts.rs` (HOST-17): the re-entry depth
  is 100 in the lane's collect mode too (was 1,000). The stack bound is a property of the
  default mode's frames, which keeps 1,000; in the collect mode every safe point of the deepest
  levels still collects while 100 re-entries are live.
- `builtins_array_mutation.rs`: the stable sort sorts 11, 513 and 128 elements in the default
  mode's gate, 2,048 in the lane; 128 takes the merge sort through 7 passes, an odd number, so
  the result is copied back from the buffer as after the 11 passes of 2,048, and 513 merges a
  run of one. The lying comparators sort 20 elements in the lane's collect mode (was 40), five
  passes with every lie answered in each.
- `builtins_json_parse.rs`: the parse that must end in a result nests 40,000 in the default
  mode's gate (100,000 in the lane and in the collect mode), and the parse under the memory
  budget keeps 100,000 everywhere (it ends at level 877). The reviver nests 1,000 in the lane's
  collect mode (was 2,000; its cost grows with the square of the nesting there). The parse and
  the reviver runs now run on `nesting_stack(depth)`, the stack of the full nesting scaled by
  depth / 100,000 in the default mode, as `deep_stack` of `crates/syntax/tests/sources.rs`
  scales it: a walk that overflows at 100,000 overflows at the gate's 40,000. Before, the gate's
  reviver walk of 40,000 ran on the whole stack of 100,000, where only a frame of 88 bytes or
  more a level would have overflowed.
- `determinism.rs`: HOST-133 runs one child process in the lane's collect mode too (was two):
  this process against the child is still two processes against each other.
- `frontend_ledger.rs`: the flat source has 5,000 lines (was 20,000), as the gate's flat source
  of `src/script.rs`. R, what a successful call keeps, grows with the lines while the room stays
  4 KiB, so both calls still end in `Terminated(MemoryBudget)` and a tree that was not charged
  would let them end normally.
- `object_patterns.rs`: the large pattern has 1,000 properties in the default mode's gate too
  (10,000 in the lane); 1,000 still grow every list of the parse and the compile step past its
  first capacity many times over.
- `language/function-recursion.js`: `fib(7)` (41 calls, both branches recurse) rather than
  `fib(10)`; with a `try-completion-*.js` script its part took 2.1 CPU s in the collect gate.

No test is deleted, ignored or moved out of a lane: `names.rs` and `determinism.rs` no longer
read `SWELL_FULL_COUNTS` and stay in the lane's filter through the name in their documentation.
No timeout, no part count and no engine source changed.

Writes: crates/swell/tests/budgets.rs, crates/swell/tests/builtins_array_mutation.rs,
crates/swell/tests/builtins_json_parse.rs, crates/swell/tests/determinism.rs,
crates/swell/tests/dynamic_function.rs, crates/swell/tests/eval.rs,
crates/swell/tests/frontend_ledger.rs, crates/swell/tests/fuzz.rs,
crates/swell/tests/language.rs, crates/swell/tests/language/function-recursion.js,
crates/swell/tests/limits_scripts.rs, crates/swell/tests/names.rs,
crates/swell/tests/object_patterns.rs, crates/swell/tests/tracing.rs

## Acceptance

- `cargo clippy --workspace --all-targets --locked -- -D warnings`: exit 0.
- `cargo fmt --all --check`: exit 0.
- `sh taskfiles/test-main.sh check`: exit 0.
- `cargo nextest run --workspace --locked --profile default`: all pass.
- `cargo nextest run --locked -p swell --features collect-every-safe-point --profile default`:
  all pass.
- The lane's step `slow-tests` (`SWELL_FULL_COUNTS=1`, both modes): all pass, none timed out.
- Each changed test, its binary run directly with `--exact <name> --test-threads 1` under bash
  `time` (`TIMEFORMAT='%U %S'`): user + system at most 1.5 s in the default mode's gate, 2 s in
  the collect mode's gate, 5 s with `SWELL_FULL_COUNTS=1` in either mode; except
  `determinism::host_133_same_calls_same_hooks_same_outcomes` in the collect gate (4.3 s, three
  runs of the corpus at 1.4 s each that the requirement's comparisons need).

## Out of scope

- The engine's cost in the collect mode: every safe point marks the whole heap, so a test's
  cost grows with steps times heap; that is the mode's design, not a defect.
- The `language` parts 3 and 9 of the collect gate (2.2 to 2.5 and 1.8 to 2.2 CPU s): each holds two scripts
  of about 0.5 s a run in both modes; the 13-part partition is the design of
  `story:collect-mode-tests-far-from-the-kill`, and more parts only spread the same work.
- The tests under their target that were near it at a high load
  (`property_index::memory_budget_of_8_mib_ends_a_push_loop` 1.13 s,
  `property_index::the_same_seeds_give_the_same_steps_and_heap_bytes` 4.1 s in the lane,
  `ops::objects::tests::every_exit_of_a_for_in_credits_its_key_list::normal_end` 2.9 s in the
  lane, the `swell-test262` run tests 0.9 s).
- `AGENTS.md`, the task files and `.config/nextest.toml`.
