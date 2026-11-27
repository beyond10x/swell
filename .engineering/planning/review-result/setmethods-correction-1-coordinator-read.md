---
format: aep.planning-md/3
id: review-result:setmethods-correction-1-coordinator-read
kind: review-result
status: active
title: 'Coordinator''s read of corrections 1 and 2 of the unit setmethods: no defect'
relations:
- reviews: story:set-methods
revision: 1
---
Coordinator's read of corrections 1 and 2 of the unit setmethods (impl/setmethods, d5791fc9 to
18c744e1), 2026-11-26. Verdict: no defect.

Correction 1 answers review-result:adversary-setmethods-pass-1 (two red cases of 8, ten
findings). It is three commits; two files change, `crates/swell/src/builtins/set/methods.rs`
(459 lines) and `crates/swell/tests/builtins_set_methods.rs` (673 lines).

Commit 8022f1c, the stack of a re-entry. The coordinator decided that the promise stays
(`STACK_BYTES_PER_REENTRY`, 14 KiB) and the frames get smaller. The steps and their order
are the clauses' as before. Read:

- `run_set_methods_builtin` does steps 1 and 2 (RequireInternalSlot of the this value), keeps
  the this value, runs GetSetRecord (step 3, shared by the seven) and only then calls the
  method's own function (`union_steps` and six beside it, each `#[inline(never)]`), so no
  frame that holds the locals of all seven methods is live across a call of the set-like's
  code. The mark is released on every way out, also when GetSetRecord throws.
- The work with one key (`take_key`) and with one entry (`take_entry`) runs no code and
  passes no safe point, and returns before the next call of `next` or `has`; the key needs
  no root there. `take_key` was read against the clauses of the six readers: union appends;
  intersection appends when the receiver has the key; difference deletes; symmetricDifference
  reads alreadyInResult first, then deletes when the receiver has the key and it is in the
  result, and appends when the receiver has it not and the result has it not; isSupersetOf
  and isDisjointFrom answer whether the iterator is closed.
- The messages of GetSetRecord are made in `set_record_error`.
- Bytes per re-entry in the dev profile, by bisection of the thread stack at depths 200 and
  1,000 over 32 routes, before and after, the largest of the seven on each way: the getter of
  `size` 13,503 and 10,304, `has` 12,463 and 9,824, `keys` 11,792 and 9,503, `next` 13,568
  and 10,287, `return` 13,375 and 9,999, the `valueOf` of the size 14,336 and 11,135. Every
  route is under the 11,500 the correction was given. In `stack-debug` the largest is 12,192
  (12,385,880 bytes at depth 1,000 of the 12,838,560 the tests run on), in the release
  profile 4,015.
- One stack test per way, each for the method with the largest frames there: the adversary's
  two red cases (union through `next` and through the getter of `size`), now green, and four
  more (intersection through `has`, `keys` and `next`, isDisjointFrom through `return`). In
  the default mode a test fails above about 12,690 bytes per re-entry. In the collect mode
  the depth is 50 and a test fails only above about 15,390 bytes, more than the promise:
  that mode holds the route, not the bound, and the helper's comment says so.

Commit d86c630, the cases and the times:

- The adversary's 6 green cases stay; in a copy, each of the three planted changes of
  findings 7 to 9 turns its case red (a cursor left open on one way out; one read out of the
  clause's order; a walk that does not see a mutation of the receiver).
- Four tests ran over the time bound in the lane's collect build (2.2 to 5.9 CPU s). Each
  keeps the story's count in the default mode and takes a smaller one with
  collect-every-safe-point, and each smaller count was shown red in that mode with its
  defect planted: `a_throwing_callback_leaves_no_cursor` at 10 rounds (a rise of 1,024 bytes
  against a bound of 32), `the_result_keeps_its_keys_across_a_collection` at 2 keys
  (`Terminated(EngineFault)`), `get_set_record_rejects_...` with two of the seven methods (a
  NaN size accepted), the memory-budget test at a room of 32 KiB (the growth of the table not
  charged ends at the step budget, which the test now sets to half the room in bytes so the
  planted run ends at once and not at the 60 s kill).
- After the change, each of the 25 tests alone: at most 0.147 CPU s in the default build,
  1.633 in the lane's collect build at opt-level 0, 0.114 in the gate's collect profile.

Commit d15e536: the merge of `wave/big-2` at 0a468d7. No conflict.

On the merged head the implementor ran the acceptance filter (122 passed in both modes), the
library tests in both modes, the story's 108 cases of test262 with the guard (`passed 285
bound 285 B 177 F 108 R 0`), `task test-realm-sizes` (0 runs red), clippy, fmt and the
test-main check.

Left for the coordinator, from the correction's notes: the story's "As delivered" (25 tests,
the counts of the collect mode); the seven stack tests and the `stack-debug` figures of 12
routes for story:stack-tests-hold-in-the-stack-debug-profile (`JSON.stringify` through
`toJSON` needs 12,833,224 of 12,838,560 bytes there, a route outside this unit).

Correction 2, commit 18c744e1 (one file, `crates/swell/tests/promise_statics.rs`, 60 lines
added, 24 removed). The coordinator's verification of d15e5361 found
`promise_statics::the_values_of_a_pending_promise_all_are_charged_until_it_settles` red with
collect-every-safe-point at the realm's real size (`after` 487,869 against `before` 389,293
with an allowance of 640), green in the default mode and at every padded size; correction
1's own check had run the acceptance filter and not the whole binary in that mode.

- The cause, measured per arena in a copy: the arena of objects grew from 512 to 1,024 slots
  between the reading before and the pending one (live slots 427, 509, 407; its room rose by
  108,288 bytes while the live bytes fell by 10,512). The first round's combined promise and
  its Array lived on into the measured round, so only that round's peak passed 512 slots,
  and only at the size the realm has with the seven set methods and no padding. The defect
  of story tests-hold-at-any-size-of-the-realm in one more test; the unit moved the realm
  onto it.
- The test now warms up with `Promise.all` over twice as many promises, and each round's
  first statement assigns `ps`, `resolvers`, `combined` and `values` again, so no arena
  grows between the readings of a round.
- A second defect of the test showed when the correction planted its defect: the reading
  after was compared with the reading before alone, and the settle frees more than the
  element boxes, so boxes that stayed charged passed, in the old form too. The test has a
  control round now, `Promise.race` over as many promises with an Array of as many values
  pushed after its settle, and both bounds compare with it: `pending - before` at least the
  race's rise plus the boxes, `after - before` at most the race's plus the room of the
  Array. Read: both are written as sums on each side, so no reading is subtracted from a
  smaller one.
- Planted "charged twice, credited once" (`builtin_function.rs:610`, in a copy): red in both
  modes at the real size and at 4 and 128, the reading after above the race's by the boxes'
  total (2,880 bytes in the collect mode, 144,000 in the default one).
- Not held, and not held before: boxes that are not charged at all pass the lower bound,
  because each element function also owns a property table of 224 bytes that race does not
  make (a margin of 5,216 bytes over the 2,880 of the boxes in the collect mode). No library
  test asserts the charge either (`builtin_function.rs:1178` asserts the size alone). The
  coordinator files this as a backlog story at the wave's close: a library test that calls
  `create_promise_closure` and asserts that the account rises by `slots.bytes()`.
- Green at the real size and at 1, 2, 3, 4, 7, 11, 17, 38, 64, 128 and 200 padded functions
  in both modes; the reading after of `Promise.all` equals the race round's at each. The
  test takes 1,969 ms alone in the collect mode at the real size (three rounds where there
  were two). The other tests of the file do not have the form.
- On 18c744e1 the implementor ran the whole binary in both modes (1336 passed each), the
  library tests in both modes, `task test-realm-sizes` (0 runs red), clippy and fmt.
