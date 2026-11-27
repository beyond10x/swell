---
format: aep.planning-md/3
id: review-result:upsert-correction-1-coordinator-read
kind: review-result
status: active
title: Coordinator's read of correction 1 of the unit upsert
relations:
- reviews: story:map-upsert-and-group-by
revision: 1
---
Coordinator's read of correction 1 of the unit upsert (impl/upsert, 5458d2e to 5233731),
2026-11-27. Verdict: no defect.

The adversary's pass 1 was green on the code (`review-result:adversary-upsert-pass-1`: no
failing case; two warnings and five notes, each a gap in the unit's tests or texts; the
engine matched the clauses on every case of its probe of 62). Read: the diff of `5233731`
(one file, `crates/swell/tests/builtins_map_upsert.rs`, +143 -12), the unit's report and its
notes for the coordinator.

Correction 1 (`7ae0488`, the merge of `wave/big-2` at `53d425f`, no conflict; `5233731`):

- F1 (warning: no test holds the key of `getOrInsertComputed` when only the arguments of the
  running builtin hold it): `get_or_insert_computed_keeps_a_key_only_its_arguments_hold`,
  called through `src.forEach(Map.prototype.getOrInsertComputed, m)` with a callback that
  clears the source Map, drops its parameter and makes objects; expected `"7:8"`. With the
  activation planted to keep no copy of its arguments (`object/builtin_function.rs`) it is
  red in a collect build (`Terminated(EngineFault)`) and green in the default build, where
  no collection falls into the callback; the file was put back.
- F2 (warning: the memory test never grows a group):
  `one_growing_group_ends_at_the_memory_budget`, default build only (a collect build takes a
  collection per element): every element into one group, the run ends in
  `Terminated(MemoryBudget)` after 150,933 steps and 4,438 elements. With the charge of an
  Array's dense storage planted to 0 (`object/array.rs`) it is red: `Terminated(StepBudget)
  after 4000000 steps, 117646 elements`; the file was put back.
- F4 (note: the memory test accepted a run that stopped after 16 elements): the test holds
  again how much a grouped element may cost, at most 768 bytes charged per element in both
  builds (measured 624 in the default build at each realm size, 451 to 545 in a collect
  build), and in the default build at least room / 768 = 342 elements (419 measured). In a
  collect build no bound from the room holds: the run ends at the first growth of an arena
  the room cannot take, 29 to 134 elements by the realm's size; the floor of 16 stays. That
  the new bound refuses an early stop is reasoned from the measured figures, not shown by a
  plant.
- F5 (note): `a_present_key_whose_value_is_undefined_is_found`, both methods.
- F6 (note): `the_shared_steps_name_the_method_in_their_messages` pins five messages; that
  RequireObjectCoercible comes before IsCallable shows in `Map.groupBy(undefined, 1)`. The
  message of an index above 2^53 - 1 needs that many elements and is pinned by no test.
- F3 (note, older than the unit: no reading can tell whether the keyed table is charged
  before or after it grows): no test. The unit's texts now say that this order is held by
  reading `object/keyed_entries.rs`.
- F7 (note: the story names the key coercion `zero`, the pinned draft and the code
  `collection`): "As delivered" of the story says so.

The realm figures on the merged head are the unit's of before the merge (394,751 and
282,792 bytes; 347 places in parts of 183 + 164): the units merged since change none.

The coordinator's runs on `5233731`, build node, before the merge: `swell` main 1692 passed,
1 skipped (default build) and 1664 passed, 1 skipped (collect build); `swell` lib 283 passed;
`task test-stack-profiles` 7 passed in each of the two profiles; `task clippy` exit 0;
`cargo fmt --all -- --check` exit 0.

Not covered: no plant shows the bound of F4 refusing an early stop; the adversary did not
attack `Map.groupBy` called from another realm, the stack-profile lane or the library tests
(the coordinator's runs above include the last two); four of the adversary's first plants
ran in the workstation's pool, from a clone outside the managed worktrees, before the brief
was changed.
