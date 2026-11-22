---
format: aep.planning-md/3
id: story:lane-tests-under-2-5-cpu-seconds
kind: story
status: active
title: The tests of the lane slow stay under 2.5 CPU seconds at their full counts
summary: The lane's bound for a test alone at its full count goes from 5 to 2.5 CPU seconds in either collection mode; every test over it gets a smaller full count, the gate's count in the collect mode where the size is not what that mode shows, and the stack checks keep a scaled stack with its derivation.
relations:
- decomposes: epic:slice-06-bindings-and-iteration
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T23:03:03Z", actor: "human:operator", revision: 2, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-22T23:03:04Z", actor: "human:operator", revision: 3, executor: "agent:coordinator"}
---

# The tests of the lane slow stay under 2.5 CPU seconds at their full counts

## Context

`story:heavy-tests-far-from-the-kill` set the lane's bound at 5 CPU seconds alone at the full
count. On the wave branch at `a6f22f6`, `task slow`, step `slow-tests`, collect half, at a load
that rose from 37 to 165 on 18 cores: `430 passed, 1 failed, 336 ignored`;
`builtins_iterators::each_next_is_charged` was killed at 60 s. It takes 4.1 to 4.2 CPU seconds
alone at its full count in collect mode, under the 5 s bound, and alone at a load of 125 to 155
it passed in 30.6 s. So the bound becomes 2.5 CPU seconds alone at the full count, in either
mode.

Measured on `a6f22f6`, every test of the two lane runs alone (the test binary run directly
with `--exact`, user + system), at loads of 32 to 176:

- collect mode, over 2.5: `termination::same_script_twice_gives_the_same_steps_and_value` 4.35,
  `determinism::host_133_same_calls_same_hooks_same_outcomes` 4.17,
  `builtins_iterators::each_next_is_charged` 4.13,
  `heap::collect::tests::the_collection_threshold_follows_the_live_reading` 3.77, the budget
  test of `computed_names_and_object_spread` 3.75,
  `dynamic_function::dynamic_function_recursion_ends_in_a_range_error` 3.62,
  `builtins_iterators::an_array_iterator_over_a_growing_array_ends_at_the_step_budget` 3.03,
  `script::tests::host_169_…` 2.91, `sort_with_a_comparator_that_throws_or_lies_ends` 2.83,
  fuzz part 4 2.64, `ops::objects` `step_budget_inside_enumerate_object_properties` 2.55; at it:
  `for_in_enumerators_held_by_a_recursion_end_in_memory_budget` 2.48,
  `sort_charges_a_step_per_comparison` 2.36 to 2.57;
- default mode, over 2.5: `builtins_json::stringify_of_100000_nested_arrays_needs_no_rust_stack`
  4.20 to 4.42, `property_index::the_same_seeds_give_the_same_steps_and_heap_bytes` 4.23 to
  4.33, the eight fuzz parts 3.80 to 4.34, the reviver result test 3.33 to 3.42,
  `script::tests::host_169_…` 3.11 to 3.12, `ops::objects` `json_parse_with_a_reviver` 2.90 to
  2.93, `normal_end` 2.80 to 2.93, `step_budget_in_the_body` 2.57 to 2.70,
  `get_own_property_descriptors` 2.49 to 2.59, `tracing::operand_stack_and_local_values_survive_collect`
  2.80 to 2.85, `builtins_function::bound_chain_of_100000_links_needs_no_rust_stack` 2.71 to
  2.81, `heap::collect` threshold test 2.56 to 2.61, `swell-syntax` `sources` part 8 2.50 to 2.53.

The lane's whole runs on `a6f22f6`: default `518 passed, 342 skipped` in 335.6 s (load 138 to
172); collect `431 passed, 336 skipped` in 84.4 s (load 173 to 86).

## Change

Where a collect-mode test's property does not depend on the size (a value held without a root
shows at any count), the lane takes the gate's count in that mode; the default mode keeps a full
count above the gate's, chosen to stay under 2 CPU seconds alone. Each count is named in the
test's documentation with the measurement.

Collect mode, the lane takes the gate's count:
- `builtins_iterators.rs`: 100 calls of `next` (was 1,000) and 4,000 steps for the growing
  array (was 20,000). `each_next_is_charged` still fails on a `next` that charges no step; the
  growing array still ends at the step budget with an element per hundred steps.
- `termination.rs`: `fib(7)` (was `fib(12)`), 41 calls a run, steps compared run for run.
- `dynamic_function.rs` and `eval.rs`: `frame_depth` 100 (was 500); the `RangeError` and the
  count of calls are asserted as before. The helper `in_the_collect_gate` of each file is
  removed (no other user). The eval test was under the bound (2.1 to 2.4 CPU seconds, rising
  with the load) and is the same test through `eval`.
- `computed_names_and_object_spread.rs`: 1,000 properties (was 10,000); both budgets are
  readings plus a room that scales with the count.
- `builtins_array_mutation.rs`: the lying comparators sort 12 (was 20), four passes; the sort
  charge test sorts 129 (was 513), 2^7 + 1, a lone last run through every pass.
- `fuzz.rs`: 15 sequences (was 50); the sequences from 15 on are the costly ones in that mode.
- `src/heap/collect.rs`: the threshold test stores 65,536 records and runs the loop twice (was
  262,144 and 4); in that mode it passes whichever reading the threshold takes.
- `src/ops/objects.rs`: the prototype-chain test takes a tenth of keys, chain and steps (was the
  full counts); the for-in recursion nests 25,000 (was 100,000).
- `src/script.rs`: the flat source has 5,000 lines (was 20,000).

Default mode, a smaller full count:
- `fuzz.rs`: 400 sequences (was 1,000), 50 a part, each kind about 280 times.
- `property_index.rs`: 5,000 properties (was 10,000), expected `3333,p1,p4999,4999`.
- `builtins_json.rs` (stringify) 40,000 levels and `builtins_function.rs` (bound chain) 50,000
  links (were 100,000), on the same 256 KiB stack: a Rust call of at least 16 bytes a level
  needs 640,000 and 800,000 bytes, 2.4 and 3 times the stack; the gate's 25,000 still needs
  400,000.
- `builtins_json_parse.rs`: the reviver nests 60,000 (was 100,000), on `nesting_stack` of it,
  the stack scaled by 60,000 / 100,000, so it overflows whenever 100,000 does.
- `tracing.rs`: 15,000 garbage arrays a loop (was 25,000), 6,960,000 bytes, six times the
  threshold; `garbage_passes_the_threshold_and_the_budget` checks it.
- `src/script.rs`: 10,000 lines (was 20,000); each test compares readings of one source.
- `src/heap/collect.rs`: the story's 262,144 records, and as many Strings as fill the room and
  two more, as the gate (was 128 Strings, 64 MiB).
- `src/ops/objects.rs`: 5,000 keys with the story's 10 calls (was 10,000), and 200,000 steps
  for the exits of a `for`-`in` (was 1,000,000); at 1,000 keys the list's charge already fails
  an uncharged list.
- `crates/syntax/tests/sources.rs`: the deep limit 7,500 (was 10,000), on `deep_stack` of it,
  three quarters of the stack of 10,000: f + 7,500 p ≥ 3 (f + 10,000 p) / 4 > 3 S / 4, so it
  overflows whenever 10,000 does.

No test is deleted, ignored or moved out of a lane: `builtins_iterators.rs`,
`computed_names_and_object_spread.rs`, `dynamic_function.rs` and `eval.rs` no longer read
`SWELL_FULL_COUNTS` in code and stay in the lane's filter through the name in their
documentation. No timeout, part count, task file or engine source outside `#[cfg(test)]`
changed.

Writes: crates/swell/src/heap/collect.rs (test module), crates/swell/src/ops/objects.rs (test
module), crates/swell/src/script.rs (test module), crates/swell/tests/builtins_array_mutation.rs,
crates/swell/tests/builtins_function.rs, crates/swell/tests/builtins_iterators.rs,
crates/swell/tests/builtins_json.rs, crates/swell/tests/builtins_json_parse.rs,
crates/swell/tests/computed_names_and_object_spread.rs, crates/swell/tests/dynamic_function.rs,
crates/swell/tests/eval.rs, crates/swell/tests/fuzz.rs, crates/swell/tests/property_index.rs,
crates/swell/tests/termination.rs, crates/swell/tests/tracing.rs, crates/syntax/tests/sources.rs

## Acceptance

- `cargo clippy --workspace --all-targets --locked -- -D warnings`: exit 0.
- `cargo fmt --all --check`: exit 0.
- `sh taskfiles/test-main.sh check`: exit 0.
- `cargo nextest run --locked -p swell --profile default` and the same with
  `--features collect-every-safe-point`: all pass.
- The lane's step `slow-tests` (`SWELL_FULL_COUNTS=1`, both modes): all pass, none timed out.
- Every test of the two lane runs, its binary run directly with `--exact <name>
  --test-threads 1` under bash `time` (`TIMEFORMAT='%U %S'`) with `SWELL_FULL_COUNTS=1`: user
  + system at most 2.5 s; except `determinism::host_133_same_calls_same_hooks_same_outcomes`
  in collect mode (4.2 to 4.3 s: three runs of the corpus at about 1.4 s, the fewest its
  comparisons in one process and across two need).

## Out of scope

- The engine's cost in the collect mode (every safe point marks the whole heap).
- Tests at 2.0 to 2.4 CPU seconds in the lane, under the bound: `swell-num` `to_string` round
  trips, `builtins_array::to_string_holds_its_object_across_the_call`, the stringify and
  reviver tests in collect mode. A test's CPU seconds alone rise with the machine's load
  (`sort_charges_a_step_per_comparison` 2.36 at a load of 40, 2.57 at 131).
- `AGENTS.md`, the task files and `.config/nextest.toml`.
