---
format: aep.planning-md/3
id: review-result:laneseven-plan-coordinator-read
kind: review-result
status: active
title: 'Coordinator''s read of the story of the gate''s test steps after the critics'' read of its fifth body: ready for the lane''s seventh unit'
relations:
- reviews: story:gate-test-steps-hold-the-budget-on-the-slower-runner
revision: 1
---
Coordinator's read of `story:gate-test-steps-hold-the-budget-on-the-slower-runner` after
two plan critics (acceptance, design) read the draft of its fifth body, 2026-11-27.
Verdict: ready for the lane's seventh unit with its fifth body.

The forge read the sixth unit's head at 117.6 s of 120 for the test steps and 30.2
yardsticks against the story's bound of 29.5 (run 46069632845), so the story stays active.
The fifth body adds item 5: more of the step `test` at opt-level 1, inside option A of
`approval-record:wave-big-2-lane-unit-6-cut`. The lists of the sixth unit give 60.6 CPU
seconds at opt-level 0 on the build node for what moves: the library tests of swell 24.1,
the package of the test262 runner 19.6, swell-num 8.9, the 17 tests of the macro
`round_trip_planes!` 8.0. Both critics read the draft and asked for a revision. What each
finding became:

The acceptance critic, ten findings:

- The reading of the cut named no instrument, no filter and no job. Taken: one pool job,
  both builds first, two runs of the moved tests with the repository's nextest file and
  one `-E`, each test through a runner script that writes the CPU seconds of its process.
- The bound of 0.40 could be read for the sum or for each binary. Taken: the sum over all
  moved tests; the sums of each binary are reported and not bounded.
- A run that selects no test prints an error, not a count line, and "both tests" does not
  fit a list that may grow. Taken: `cargo nextest list` prints no line in one run and one
  in the other, for each test of the list.
- The new line `test: <n> tests at opt-level 1, <m> at opt-level 0` had no relation.
  Taken: n plus m is p plus f plus i of the summed line, and n is the count of
  `swell::main` plus the counts of the other moved binaries. The runner's executable holds
  no test and is not named as a moved binary.
- The digest line was said to differ between base and head. Taken: it is the same; the
  digest does not cover the tests of the crates.
- The 17 tests leave the lane's part `test-realm-sizes`, and the body said the lane does
  not change. Taken: item 5 says what the two parts of the lane then run and why, the head
  of the section says that no task of the lane changes, and the acceptance reads the two
  rules' output on base and head.
- The diff line covered four of the files said not to change. Taken: `git diff
  --name-only` lists `Taskfile.yml`, the one test file and files under `tests/gate/`.
- A count and one name do not show the 17 names unchanged. Taken: the 17 lines of the
  list are compared by `diff`.
- One target directory for two builds. Taken: one for each, absent before its job.
- `task gate-selftest` without the pool. Taken: `slot task gate-selftest`.

The design critic, five findings:

- The summed line reads its count of binaries and of ignored tests from the run at
  opt-level 0, whose filter then leaves whole binaries out. Taken: the body says what b
  and i mean (each binary once; the list without a filter less passed and failed), the
  acceptance keeps 19, 2532, 0, 1, and the unit reports where it reads them from.
- The lane changes: `slow-tests` runs the 17 tests in its part at opt-level 1, and
  `test-realm-sizes` no longer runs them. Taken, as above; `slow-tests` runs once on base
  and head, and its two collect lines move 17 between them.
- The list of tests that stay at opt-level 0 as lines with a comment conflicts with the
  form the holders read and with the rule of no comments in such files. Taken: one plain
  line of pairs, held by `test-opt1-vars-check`, the reasons in the summary of `test`.
- Two tests are on the list, and more read the fixture trees with a case at the nesting
  limit. Taken: the two are a sample, one for each tree; the others move, and the body
  says which tests hold the stack's bound at opt-level 0.
- The planted scripts and the check of the filter line depend on `--test main` and on the
  count of `test(=`. Taken: the body names the three scripts that change with the forms
  and says that the check of the written line follows the new form.

Not taken: nothing. Not read again by a critic: the revision takes each finding in the
critic's own smallest change, and the unit's adversary reads the result.
