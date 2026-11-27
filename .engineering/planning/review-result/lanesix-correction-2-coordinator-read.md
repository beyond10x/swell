---
format: aep.planning-md/3
id: review-result:lanesix-correction-2-coordinator-read
kind: review-result
status: active
title: 'Coordinator''s read of correction 2 of the unit lanesix: ready to merge'
relations:
- reviews: story:gate-test-steps-hold-the-budget-on-the-slower-runner
revision: 1
---
Coordinator's read of correction 2 of the unit `lanesix` (branch `impl/lanesix`, `c63040c1` to
`362b69b1`), 2026-11-27, in place of a third adversary pass: the budget of two passes is
used. Verdict: the correction does what the second pass asked for where the coordinator
took the finding; nothing is left to correct before the merge.

What the unit is. The gate's step `test` runs in two runs: the 1,647 tests of
`swell::main` that the stack rule prints "other" at opt-level 1 in the profile `collect`
without the collect feature, with the lane's guard set; the other 123 tests of that binary
and every other test of the workspace at opt-level 0. The step compares the three lists of
`cargo nextest list`, fails when a test is in not exactly one run, and prints one summed
line. `build-tests` builds the binary at opt-level 1 before the clock.

The two passes.

- Pass 1 (`review-result:adversary-lanesix-pass-1`, on `808f926b`): red. Three defects
  with a planted script each (a caller's value of `TEST_OPT1_ARGS` or `NO_STACK_TESTS` was
  accepted; `test-sum` failed when `task` was called by a relative path; nothing kept the
  profile's test in the run at opt-level 1), a note that the profile's test reads its own
  crate only, three mutants the unit's script let pass, and a note on the summed line of a
  run a signal ends. Correction 1 (`69868891`, `4ad70476`, `c63040c1`) took all but the
  last, which stays: the step is red there and names the missing Summary line. Outcome:
  fixed.
- Pass 2 (`review-result:adversary-lanesix-pass-2`, on `c63040c1`): red. The checks of
  correction 1 held every input tried but two: the check of the profile tables did not
  read a file the cargo configuration includes, nor a key written with an escape or an
  inline table over several lines; and `test-split` ended without a line of its own when
  the stack rule had no support file. Three more mutants were caught by cases the pass
  added. Two notes of judgement: the check refused profiles the run never uses, and the
  tasks described as internal can be called by name. Outcome: fixed, with the limits
  below.

What correction 2 changed (`ef47efb7`, `362b69b1`), read in the diff of `Taskfile.yml`
(56 lines in the tasks `test-opt1-profile-check`, `test-split` and the summary of
`build-tests`) and of the scripts:

- `test-opt1-profile-check` refuses a key `include` at the top of `.cargo/config.toml` or
  `.cargo/config`, in both of its forms, with a line that says why: the check does not
  read a file the configuration includes. Read: the test is on the path with no table
  open and not on `Cargo.toml`; a key `include` inside another table is not matched.
- The same check refuses a profile of one package or of build scripts only in the profiles
  `dev` and `test`, which the profile `collect` inherits, and any definition of `collect`;
  `release` and `bench` pass, and the environment's variables follow the same names. A
  form the pass had not named was closed with it: a one-line inline table `profile = {
  test.package... }`.
- `test-split` gathers the support files with `if ... fi` and fails with a line that names
  the step and the missing files. The other loops the unit added do not end in a false
  test.
- The cases: the include in both forms, the two profiles that pass, the inline form, the
  missing support files, and the three cases of the pass for its mutants. The correction
  planted nine mutants in the worktree (the pass's three and six that undo one change
  each); each is red in one of the six scripts.
- No file under `crates/`, no file or task of the story's list "Not changed", no value of
  an existing variable. `gate-selftest` reads 112 passed, 0 failed.

Decided by the coordinator, and where each stands:

- Not read by the check, stated in the task's summary: a cargo configuration file outside
  the tree; a key written with an escape; an inline table over several lines. The check
  guards a change made by mistake in a file a reviewer reads, not a hidden one.
- Known over-refusal: one top-level line `profile = {...}` that names a package profile
  of `release` or `bench` and also `dev` or `test`; profile text inside a multi-line
  string. Neither form is in the tree.
- The tasks described as internal stay callable by name: the file's convention, and the
  planted scripts call some of them. `test` runs `test-split` itself, so no result of the
  gate carries a value given to the inner task.
- Not the unit's, and left as they were: the same loop form in two tasks of the lane
  `slow`, and the path of the executable in the task `check-runner`
  (`taskfiles/forge.yml`). Both go to the backlog as drafts.

The measurement the operator's decision asked for. Whole gates on the build node (32 cores),
the CPU seconds of the test steps, base `f93ab563` against the head of the time:

- 416.9 against 315.9 (loads 36.7 and 38.5), ratio 0.758, head `1910825c`;
- 314.1 against 245.6 (loads 16.3 and 19.5), ratio 0.782, head `1910825c`;
- 476.5 against 348.7 (loads 15.4 and 14.1), ratio 0.732, head `362b69b1`, each the second
  of two gates in a row, so that neither built anything;
- 294.0 against 338.4 (loads 13.5 and 10.0), ratio 1.151, head `362b69b1`, each the first
  of those two gates.

The first pass recomputed the first two from the logs; the coordinator took the last two
after two gates on the corrected head had read 341.5 and 364.8 with no base beside them.
The bound is 0.80. Three pairs are under it and one is far over it, and that one is not a
reading of the unit: in it the step `test-collect`, which the unit does not touch, read
64.8 CPU seconds on the base and 121.1 on the head. Across all gates of this unit that
step read 64.8 to 127.9 on one code and one node, at loads that do not explain it; the
node gives two levels of CPU seconds for the same work, and the load average does not say
which. Read against `test-collect` of the same gate, the step `test` costs 2.2 to 2.9
times that step on the base (four gates) and 1.25 to 1.74 times on the head (seven
gates). At one level the pairs read 0.73 to 0.78; a base at the lower level against the
one head read there (294.0 against 245.6) reads 0.84. So the cut is real and about a
quarter of the test steps; the node cannot show whether it is under 0.80 by a margin.
Item 4 adds no entry to the CPU table: an entry needs a head at 0.64 of the base. What
the story is for is read on the forge, on the wave's second pull request: the test steps
of the first attempt of `check` against its yardstick and the budget of 120 s, and the
duration of the job with the build of the binary at opt-level 1 before the clock (5 min
14 s to 8 min 44 s on the build node).

Verification before the merge: three whole gates on `362b69b1` on the build node with the
digest line of the test262 sources set in the tree and put back after, each exit 0: `19
test binaries: 2532 passed, 0 failed, 1 ignored`, `gate-selftest: 112 passed, 0 failed`,
the test steps 38.3 to 45.5 s of 120.
