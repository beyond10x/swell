---
format: aep.planning-md/3
id: review-result:lanetests-correction-1-coordinator-read
kind: review-result
status: active
title: 'Coordinator''s read of correction 1 of the unit lanetests: no defect'
relations:
- reviews: story:realm-size-lane-within-its-share
revision: 1
---
Coordinator's read of correction 1 of the unit lanetests (impl/lanetests, d693bdc to
4f4d1a2), 2026-11-27. Verdict: no defect.

The correction answers review-result:adversary-lanetests-pass-1 (eight findings, one of them
introduced by the unit with the weight of a warning). It is five commits in test files, no
file under `crates/*/src`, and the merge of `wave/big-2` at b9d3c92 (b04fa6b, no conflict).
Read: the five commits in full, 186 lines added and 56 removed in eight files.

- Finding 1 (the earlier collect counts ran in no lane), as the coordinator decided: a
  changed test whose earlier count took at most 2.5 CPU s alone in the lane's collect build
  runs that count again with `SWELL_FULL_COUNTS=1`; the others keep the smaller count in every
  collect run. Commit 31092e3:
  `async_generators::settling_a_request_credits_nothing_until_the_generator_is_freed` (100
  requests in the lane, 40 in the gate; `settling_requests`), the memory round of
  `a_script_that_queues_requests_without_end_ends_at_a_budget` (64 KiB and 16 KiB;
  `loop_room`), `property_index::the_same_seeds_give_the_same_steps_and_heap_bytes` (50 and
  20, with the expected answer of each count) and
  `new_target::arrow_boxes_are_charged_and_credited` (40 and 20). Read: each reads the
  variable beside the feature, the default mode is untouched, and the assertions use the
  count the script ran. `tagged_templates::many_sites_end_at_the_memory_budget` keeps 40 (100
  took 2.65 to 3.36 CPU s) and HOST-206 keeps 40 (100 took 2.53 s); the doc comments say so.
  Commit 4f4d1a2: the fill of
  `termination::an_agent_that_ended_in_memory_budget_runs_again_after_a_collection` has 4 KiB
  of room in every collect run (64 KiB took 4.4 s in the lane).
- Finding 8, commit 7f3271c: the stack test through `super` is four tests, one per form
  (getter, setter, the key of `super[k]`, the key of `super[k] += 1`), each on its own bounded
  stack at the depth the one test had (`BUILTINS_REENTRY`). Read: the four sources are the
  four of the old body, text for text; the first test keeps the old name.
- Finding 2, commit 8c4f361: the random operations of Map and Set are two tests over two
  seeds, 1,000 operations each in the default mode (the story's 2,000 from one seed before)
  and 6 each in the collect mode (12 before). The script counts the special keys it set and
  each test asserts the count: 8 in the default mode, 0 in the collect mode, where the 12
  operations set none either. The second half of the first seed's 2,000 operations is no
  longer run; the second seed takes its place. The implementor's two plants: a size never
  counted is red in all four runs; a `delete` that answers false is red in the default mode
  and green in the collect mode at 6 operations, as it was at 12.
- Finding 6, commit 710ec8b:
  `language::the_language_directory_holds_only_scripts_the_parts_run` fails naming any entry
  of `tests/language/` that is not a file `*.js`. A planted `.mjs` file is red in both modes.
- Finding 7: the split of the parse cases of `new.target` is 29 and 25; the figure is
  corrected in the unit's notes. Findings 3, 4 and 5: no change, as decided; findings 4 and 5
  are backlog drafts.

Checks of the implementor on the build node: the whole `swell::main` in both modes on
4f4d1a2, `1389 tests run: 1389 passed, 1 skipped` each; the changed files with
`SWELL_FULL_COUNTS=1` on one thread, 229 passed in each build; `task slow-tests` on 4f4d1a2,
exit 0; `task test-realm-sizes` on b04fa6b, 2 runs red, both the known
`proxy::revoke_is_idempotent_and_drops_target_and_handler` at k = 200, which the unit
proxytraps corrects; `task test-stack-profiles` on b04fa6b red in the profile `stack-debug`
in the known `limits_scripts::min_stack_bytes_is_enough_through_builtins` only, which the
unit stackjoin corrects; clippy, the format check and `taskfiles/test-main.sh check`.

The collect runs of `test-realm-sizes` on the merged head sum to 1,625.6 test-seconds (JUnit;
2,015.2 at the unit's first delivery, 2,588.5 at its base), measured at different loads.

Left open, outside the unit's changes:

- Six tests the unit did not change take over 2.5 CPU s alone with `SWELL_FULL_COUNTS=1` in
  the lane's collect build (2.96 to 9.73 s). A backlog draft takes them.
- `promise_statics::the_values_of_a_pending_promise_all_are_charged_until_it_settles` takes
  2.19 s alone in the lane's collect build since the unit setmethods gave it a larger first
  round, over the bound of 2 s. It is corrected in this wave after this merge.
