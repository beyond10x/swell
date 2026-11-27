---
format: aep.planning-md/3
id: review-result:lanefour-correction-2-coordinator-read
kind: review-result
status: active
title: Coordinator's read of correction 2 of the unit lanefour
relations:
- reviews: story:realm-size-lane-within-its-share
revision: 1
---
Coordinator's read of correction 2 of the unit lanefour (impl/lanefour, b13c3eb to 826214f),
2026-11-27. Verdict: no defect. The lane's bound on the forge is not shown: no forge run
exists for this head.

The unit's two adversary passes are spent (`review-result:adversary-lanefour-pass-1`,
`review-result:adversary-lanefour-pass-2`, the second red without a blocker), so the
correction after the second pass is read here. Read: the diff of `9a8a24c` (the guard in
`crates/swell/tests/support/host.rs`, the six sites that took a thread with
`Builder::stack_size` and now call `stack_thread`, `Taskfile.yml`,
`taskfiles/realm-room-tests.awk`, the selftests under `tests/gate/`), the merge `34dad48` of
`wave/big-2` at `ed8a017` (no conflict), the diff of `826214f`, and the unit's three reports.

Correction 2:

- The stack rule misses forms (pass 2, two warnings: a struct field or a comment taken for a
  binding; twelve forms of reaching a stack helper printed "other"). Two answers. The rule
  reads more forms (comments are stripped before a binding is looked for; a turbofish of any
  depth, a function in an array or as a field's value, `extern` and `pub(in ..)` functions,
  a path through `crate::support::`), and its head lists the four it does not read (a helper
  imported by `use`, under another name, a `const` or `static` function pointer, a pointer
  made by a cast). And a guard holds the split at run time whatever the rule reads: the run
  at opt-level 1 sets `SWELL_NO_STACK_TESTS=1`, and `stack_with_room` and `stack_thread`, the
  two ways of the tests to a bounded stack, panic when it is set, naming the way out.
  `limits_scripts::a_stack_test_fails_by_itself_where_no_stack_tests_is_set` holds the guard
  with a child process for each way, with and without the variable; it is red without the
  guard's calls.
- The guard's first run found a gap the rule cannot see: 14 stack tests written by macros
  (11 in `builtins_array_iteration`, 3 in `proxy`) ran at opt-level 1. The run at opt-level 1
  now takes only the tests the rule prints "other"; every other test of the same modules
  runs at opt-level 0. Read for soundness: a test the rule does not see (a macro's) is in
  neither list, so it falls to the opt-level 0 run and no test runs in neither.
- The selftest of the background build stayed green on three plants (pass 2, a warning): it
  now fails on an earlier lane's status left in `.out/slow-collect`, on a wait without its
  bound and on a build without its flags; each plant's red line is in the unit's report.
- One definition of the flags (pass 2, a note): `COLLECT_RUSTFLAGS` at the top of
  `Taskfile.yml`, read by the build and by the runs on it; the third plant above holds it.
- The count line counts what runs (pass 2, a note): a test compiled out of the collect
  build is printed "not-in-collect" and counted apart.
- `slow-collect-stop` ends only a group whose command line names this tree's
  `.out/slow-collect` (pass 2, a note), and `slow-collect-start` names `setsid` when it is
  missing (a note); both held by cases of the selftest.
- `class_elements_scale_linearly` bounds a ratio of two wall times (pass 2, a warning,
  older than the unit): not the unit's; a backlog draft exists.
- `826214f`: `GATE_VARIABLES` names `COLLECT_MAIN`, `COLLECT_RUSTFLAGS`,
  `SLOW_COLLECT_WAIT` and `NO_STACK_TESTS`, so the first line of the gate and of the lane
  shows them. No check refuses a value of them given on the command line; the line shows it.
  Its message states the count the message of `9a8a24c` has wrong (68 stack tests, not 67).

On the merged head the rule prints 71 stack tests and 300 room tests of 1,609; the three
stack tests the wave added (`host_overrides`) are printed "stack", and the wave adds no
direct `stack_size` site. `slot --timeout 30m task slow-tests` there: exit 0, 381 tests at
opt-level 0 and 1,034 at opt-level 1 in the collect mode, the guard never fired.

The lane's time. One cold run of `task slow` on the build node, 32 cores, at `826214f` with
the expected file written again: `slow-time lane 444.2 of 900` (parts 297.4, 83.3, 38.5,
25.0; the build of the profile `collect` beside the first part 245.2 s, waited 0 s). The
unit's estimate for the forge is about 848 s of 900 (about 806 s before the merge of the
wave), each part scaled from the forge's last measured run (1,137.3 s on `158888d`). It is
an estimate; the unit's own aim of 800 s is not met by it. The pull request of the wave
waits for a measured run of the forge at or under 900 s.

The coordinator's runs on `826214f`, build node, before the merge: `swell-syntax` main 80
passed (default counts and full counts); `swell-compiler` main 17 passed; `swell` main 1658
passed, 1 skipped (default build) and 1631 passed, 1 skipped (collect build); `swell` lib 283
passed; `task test-stack-profiles` 7 passed in each of the two profiles; `task clippy` exit
0; `cargo fmt --all -- --check` exit 0. The wave's head moved after the unit's merge of it
(the units `staticblocks` and `upsert`): the stack rule and `slow-tests` run again on the
merged branch after this merge.

Not covered: no forge run; the rule on an awk other than gawk; a TERM sent to the task
runner in the middle of a run; the stop's check reads `/proc` and works on Linux only; a
compile inside the compile cache's server was not caught in flight when a build was ended.
The unit's whole-gate runs and cold lanes ran in fresh clones under `.out/` of its worktree,
each one pool job on a build node of 32 cores, with the expected file, its digest and the
site's status file written again in the clone. On the unit's branch `gate-selftest` reads
102 passed, 2 failed: both are `site-data-check` on the wave's `website/data/status.json`,
which is written again at the wave's close, like the expected file of test262.
