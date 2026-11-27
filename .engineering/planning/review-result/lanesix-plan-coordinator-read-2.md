---
format: aep.planning-md/3
id: review-result:lanesix-plan-coordinator-read-2
kind: review-result
status: active
title: 'Coordinator''s read of the story of the gate''s test steps after the critics'' second read: ready with its fourth body'
relations:
- reviews: story:gate-test-steps-hold-the-budget-on-the-slower-runner
revision: 1
---
Coordinator's read of `story:gate-test-steps-hold-the-budget-on-the-slower-runner` after a
second read by two plan critics (acceptance, design), 2026-11-27. Verdict: ready for its
unit with its fourth body.

The unit stopped after its first part: no cut the second body allowed reaches 80 % of the
test steps' CPU seconds. The operator decided the cut
(`approval-record:wave-big-2-lane-unit-6-cut`): the tests of `swell::main` at opt-level 1,
the stack tests at opt-level 0 behind the lane's guard, debug assertions and overflow checks
on. The coordinator wrote the third body; both critics read it on `wave/big-2` at
`4d5bacb2` and asked for a revision. A probe on the unit's base ran beside them (119.1 CPU
seconds at opt-level 0 and 20.4 at opt-level 1 for the binary; 305.7 s for its build). What
each finding became in the fourth body:

The acceptance critic, nine findings:

- "The rule prints each test" is false of `taskfiles/realm-room-tests.awk`: it prints
  `#[test]` functions with a name on the `fn` line, may add a third field, and prints a
  test behind a `cfg` under a name the binary does not hold. Taken: the body says what the
  rule prints, that the step reads two fields, and that a test not printed or not in the
  binary stays at opt-level 0.
- "In neither or in both" had no source of names. Taken in the critic's form: the step
  compares `cargo nextest list` of each run with the list without a filter.
- "`ignored` stays 1" cannot be the sum of two filtered runs. Taken: the line's `ignored`
  is the count of tests neither run ran, and the binary is counted once.
- The plants of the profile had no command, and `build-collect` refuses another line in
  the profile's file. Taken: the two plants are `--config` keys on the command of the run
  at opt-level 1, no file changes, the overflow goes through `black_box`, and the report
  shows the test in that run's list.
- The digest line: the check prints the digest only after the test steps, and no checkout
  was named. Taken: `task test262-digest` before the first run, base as a detached
  checkout; and without two counting pairs the cut is not accepted.
- "One test file" and the guard's line. Decided against the critic's fix (leave the line):
  a stack test caught in the gate would name the lane's task. The unit rewords the line to
  name both runs, with the test that holds it; the new test goes into an existing module,
  so the generated `main.rs` does not change.
- `test-binaries-check` reads the test profile only; the cold build had no command. Taken:
  `CARGO_TARGET_DIR` under `target/` in a pool job that names its time, the pool's seconds,
  `du -sk`, and the count line of `build-collect`'s check of `collect/deps`.
- Statements true before the unit (the lane's 175 s, the stack profiles) are labelled
  guards; "no test over 60 s" is cut (the runner kills at 60 s); scripts are timed alone.
- No contradiction with `AGENTS.md`; noted that an entry of the CPU table needs 0.64. Taken
  into item 4 as a sentence.

The design critic, ten findings:

- The profile choice reached into `build-collect` and a planted script that copies the
  profile's file. Taken: the profile `collect`, its file unchanged, no second profile; the
  run is the default mode without the feature, in a variable of its own.
- The route of the profile's plants: as above.
- `nextest-report` reads one run. Taken: it and `compile-check` do not change; the step
  makes the summed line and the ten slowest over both runs, and two planted scripts of
  `nextest-report` join the byte-identical list.
- The definition of `ignored`: as above.
- The new test's module must name none of `SWELL_FULL_COUNTS`, `stack_with_room`,
  `stack_thread`, `stack_size(`, or the lane's counts move. Taken.
- The list of what the unit may change in `Taskfile.yml` was wider than item 3 needs.
  Taken: `test` and `build-tests`, new internal tasks and variables; the lane's tasks, its
  four variables, the rule's file and the task files named in the body do not change.
  The edges to the two lane stories are recorded as `informed_by`.
- The `deps` check already reads every `target/*/deps`. Taken: `taskfiles/gate-time.yml`
  is not edited.
- Where the new binary is checked for debug sections. Taken: it is built before
  `build-collect`, whose check covers the directory.
- `taskfiles/test262.yml` is a source of the digest and was not named. Taken.
- The build's cost was recorded and not bounded. Decided: its bound is the forge's limit
  of 30 minutes for the job, read by the coordinator on the pull request beside the 6 min
  49 s of the run before; the unit reports the seconds and does not cut them. The operator
  has the probe's 305.7 s.

Not taken into the unit: the tests a macro writes (52, 8.9 CPU seconds) and the other
crates stay at opt-level 0. With the probe's numbers the expected ratio is about 0.73; the
bound stays 0.80.
