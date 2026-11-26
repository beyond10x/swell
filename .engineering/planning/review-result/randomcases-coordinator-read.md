---
format: aep.planning-md/3
id: review-result:randomcases-coordinator-read
kind: review-result
status: active
title: 'Coordinator''s read of the unit randomcases: four cases of the second adversary pass on Math.random, tests only, no defect'
relations:
- reviews: story:math-random
revision: 1
---
Coordinator's read of the unit randomcases (impl/randomcases, b9bd2838 to 497fa1ff),
2026-11-26. Verdict: no defect; tests only.

The unit takes the four cases of review-result:adversary-random-pass-2 into the tests. The
pass was the last of its unit in wave big-1; its cases passed and stayed uncommitted in the
unit's old tree, where the coordinator found them on 2026-11-26 and saved them as a patch.

Commit 497fa1ff (2 files, 271 added lines, no line under `crates/*/src`):

- `crates/num/tests/random.rs` (+78):
  `random::the_derived_states_agree_with_the_standard_librarys_siphash_2_4` holds `from_seed`
  and the committed `DERIVED` words to the standard library's SipHash-2-4, after that oracle
  reproduces two published values.
- `crates/swell/tests/math_random.rs` (+193):
  `math_random::the_first_16_values_of_realms_0_and_1_equal_the_designs_text`,
  `math_random::a_loop_of_calls_costs_the_steps_and_bytes_of_a_loop_of_math_max` and
  `math_random::the_stream_resumes_after_an_interrupted_entry`.
- The names carry no `adversary2_` prefix; each file has one copy of the helper
  `std_siphash24` with `#[expect(deprecated, reason = ...)]`. The patch as saved failed
  clippy in `crates/num` (`unwrap_used` on four lines of the helpers); the unit rewrote the
  two helpers without `unwrap`.

What each test adds, as the unit reasoned it from the pass's report (no change was planted):
the first two catch nothing a committed test does not catch and add an oracle that uses no
code of `swell_num::siphash`; the loop test catches a `Math.random` that allocates per call
(the committed test reads steps and hooks only); the interrupt test catches a generator state
that is rolled back or written back only when an entry ends normally (no committed test ran
`Math.random` across a termination).

The unit's runs at 497fa1ff: swell-num `153 passed, 0 skipped`; swell `math_random::` 15
passed in the default mode and with `collect-every-safe-point`; clippy exit 0; fmt exit 0;
`test-main.sh check` 9 crates. The slowest of the four tests takes 0.502 s
(`a_loop_of_calls...`, collect mode).

Not checked: no change was planted for any of the four tests. The loop test compares readings
of `heap_bytes()` of two scripts; `task test-realm-sizes` on the wave tip at the close is the
check that it holds at each realm size.

The unit reported two lines of `docs/plan/unit-invariants.md` that contradicted the wave's
rules (a unit does not commit; a copy of the tree is never built). The page is corrected in
the same commit as this record.
