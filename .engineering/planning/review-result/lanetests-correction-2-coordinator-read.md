---
format: aep.planning-md/3
id: review-result:lanetests-correction-2-coordinator-read
kind: review-result
status: active
title: 'Coordinator''s read of correction 2 of the unit lanetests: no defect'
relations:
- reviews: story:realm-size-lane-within-its-share
revision: 1
---
Coordinator's read of correction 2 of the unit lanetests (impl/lanetests, 4e788b3 to
d89192e), 2026-11-27. Verdict: no defect. No further adversary pass ran: the correction
changes one test file, and the read below is of its whole diff.

Why the correction: after the first merge of the unit (7761c2a),
`promise_statics::the_values_of_a_pending_promise_all_are_charged_until_it_settles` took
2.19 s alone in the collect build of the lane, over the bound of 2 s of a test in a collect
build. The test came with the unit setmethods (18c744e) and its count was not among the
tests the unit lanetests had changed.

The change (`crates/swell/tests/promise_statics.rs`, 41 lines added, 21 removed, two
commits, 79cd3f3 and d89192e, after the merge 4e788b3 of `wave/big-2` at 7761c2a):

- `PENDING` is 12 with the feature `collect-every-safe-point` (20 before) and 1,000 without
  it, as before.
- `pending_promises()` returns 20 with the feature and `SWELL_FULL_COUNTS=1`, and `PENDING`
  otherwise; the form is that of `settling_requests` in `async_generators.rs`. The test
  reads its count once and uses it in the three rounds, the pushed Array, the two
  comparisons and the printed line; no use of `PENDING` is left in the test's body.
- The comparisons are unchanged but for the count: the lower bound (the boxes of the element
  functions, 144 bytes per promise) and the upper bound (the room of the Array of values,
  two Values per promise).

Times, measured by the implementor on the build node, the test alone, CPU seconds, three
runs each: collect build 1.299 to 1.306 (bound 2 s); collect build with
`SWELL_FULL_COUNTS=1`, count 20, 2.189 to 2.214 at a load of 11 to 12 (bound 2.5 s); default
build 0.720 to 0.724 (bound 1.5 s). The test runs four times its count of promises at about
0.11 s each in the collect build.

What the smaller count still shows, by the implementor's plants in a copy at count 12:

- the boxes charged twice and credited once: red at 12 sizes of the realm (the real size and
  `SWELL_TEST_REALM_FUNCTIONS` 1, 2, 3, 4, 7, 11, 17, 38, 64, 128, 200) in both modes; in
  the collect build the rise is 1,728 bytes above that of race (12 times 144) against a room
  of 384;
- the boxes not charged: green at every size in both modes, as before the change and as the
  commit 18c744e says: the lower bound does not show this defect at any count. Nothing is
  claimed for it.

Green without a plant: the 12 sizes in both modes; in `task test-realm-sizes` the test at
every size in both modes.

Checks of the implementor on d89192e: the 20 tests of `promise_statics` in both modes; the
test alone with `SWELL_FULL_COUNTS=1` in both builds (the default build prints "Promise.all
of 1000"); clippy; the format check.

Not measured: the time of the test with `SWELL_FULL_COUNTS=1` on a runner of the forge.
