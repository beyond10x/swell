---
format: aep.planning-md/3
id: review-result:laneseven-coordinator-read
kind: review-result
status: active
title: 'Coordinator''s read of the unit laneseven after two adversary passes: ready to merge, one note to the backlog'
relations:
- reviews: story:gate-test-steps-hold-the-budget-on-the-slower-runner
revision: 1
---
Coordinator's read of the unit `laneseven` (branch `impl/laneseven`, `0c4df9b7`, `c47fc9e2`
and `971ac76a` on `9a8ad008`), 2026-11-28, after two adversary passes: the budget of two
passes is used. Verdict: the unit does what item 5 of the story asks; one note of the second
pass goes to the backlog; one sentence of the story was wrong and is corrected with this
read. Nothing is left to correct before the merge.

What the unit is. The gate's step `test` runs more of its tests at opt-level 1. Besides the
tests of `swell::main` that the stack rule prints "other", the run in the profile `collect`
now holds every test of swell's library (the binary `swell`), of `swell-num` (`swell-num`,
`swell-num::main`) and of `swell-test262` (`swell-test262`, `swell-test262::main`) but the
two pairs of the new variable `TEST_OPT0_TESTS`, and the 17 tests the macro
`round_trip_planes!` writes, which now stand on a `fn` line each so that the rule prints
them "other". The filter of each run is one expression written twice by one function, the
second as `not (...)` of the first. The summed line reads its count of binaries and of
tests from the list of the workspace without a filter, taken as JSON.

## What was read

- The diff `9a8ad008..971ac76a`: `Taskfile.yml` (255 lines changed: the variables at the
  top, `GATE_VARIABLES`, the tasks `test`, `test-split`, `test-sum`, `build-tests`,
  `test-opt1-vars-check` and the summary of `gate-vars`), `crates/swell/tests/builtins_uri.rs`
  (the macro's calls) and six scripts under `tests/gate/` (two new). No file item 5 lists
  under "Not changed" is in it: no file under `crates/*/src`, no `Cargo.lock`, no
  `Cargo.toml`, no file under `taskfiles/`, `.config/` or `.cargo/`.
- The unit's report, its correction report and their logs; both adversary reports and the
  outputs of their mutant scripts.
- The numbers of the unit's three whole gates on the build node (32 cores), each exit 0:
  - base `9a8ad008`: `test: swell::main 1647 at opt-level 1, 123 at opt-level 0`, `19 test
    binaries: 2532 passed, 0 failed, 1 ignored`, `gate-time test-steps 38.4 of 120, cpu
    320.4, load 9.16 on 32 cores`, the step `test` 146.8 CPU seconds;
  - head `c47fc9e2`: `test: swell::main 1664 at opt-level 1, 106 at opt-level 0`, `test:
    2187 tests at opt-level 1, 345 at opt-level 0`, the same summed line, `gate-time
    test-steps 39.2 of 120, cpu 228.5, load 9.76 on 32 cores`, the step `test` 76.6;
  - head `971ac76a`: the same three lines, `gate-time test-steps 49.3 of 120, cpu 251.4,
    load 16.20 on 32 cores`, the step `test` 107.4, `gate-selftest: 114 passed, 0 failed`.
  The two heads run the same 2,187 and 345 tests; the correction changes no list. The node
  gives two levels of CPU seconds for the same work (the sixth unit's read says so), so the
  whole gates do not carry the cut.
- The reading that carries the cut: one pool job ran the 540 moved tests once at each
  opt-level, each test through a runner that reads its process's CPU time. 37.16 CPU
  seconds at opt-level 0, 6.01 at opt-level 1, ratio 0.162 against the story's bound of
  0.40. By binary: `swell` 291 tests 16.69 to 2.33; `swell-test262::main` 62 tests 10.51 to
  1.64; `swell-num::main` 153 tests 5.02 to 1.27; the 17 tests of the macro 4.49 to 0.61;
  `swell-num` 8 tests 0.42 to 0.14; `swell-test262` 9 tests 0.02 to 0.02.
- The digest of the test262 sources does not move: no file it covers is in the diff.

## The story's sentence that was wrong

The fifth body said that n plus m of the new line `test: <n> tests at opt-level 1, <m> at
opt-level 0` is p plus f plus i of the summed line (2533). The lists of the two runs hold
the tests that run; the one ignored test is in neither. So n plus m is p plus f (2532), and
p plus f plus i (2533) is the count of tests of the list without a filter. The unit and the
first pass both found it. The step is right; the sentence of the acceptance is corrected in
the same commit as this read, and it says that it was corrected.

## The two passes

- Pass 1 (`review-result:adversary-laneseven-pass-1`, on `c47fc9e2`): red, five findings.
  The equation above; a mutant that drops `& not binary_id(=swell::main)` from the filter
  and passes every planted script, with every stack test of `swell::main` at opt-level 1
  and still in one run only; a pair of `TEST_OPT0_TESTS` that names a test outside the
  moved binaries was accepted and changed nothing; the summary of `build-tests` did not
  name the new variable; the report gave no seconds of the scripts. Correction 1
  (`971ac76a`) took the pass's case into `tests/gate/` with the equation corrected,
  refuses such a pair and a pair named twice before any list, requires the term of
  `swell::main` and its exclusion once each in the written line, and gives the seconds
  (0.85 to 1.51 each, run alone). Outcome: fixed.
- Pass 2 (`review-result:adversary-laneseven-pass-2`, on `971ac76a`): green, one note.
  Thirteen mutants over the six scripts and three over the real lists. The dropped
  exclusion is now refused by all six scripts; packages are matched exactly; a pair of the
  library is accepted and takes effect; a caller's value is refused before the pairs are
  read; the added lines pass under dash. Outcome: no-op, with the note below.

## The note that stays

The check of the written filter line counts spellings: `package(=`, `binary_id(swell::main)
& (` and `) & not binary_id(=swell::main)`. A second package part written with the glob
matcher, `| package(swell)`, added to the function that writes the filter, keeps all three
counts, passes the six planted scripts and passes the split with the real lists: `test:
swell::main 1770 at opt-level 1, 0 at opt-level 0`, the 73 tests the rule prints "stack"
in the run at opt-level 1 and each test still in one run. The guard catches it when the
tests run: those tests panic there with the guard's line, so the gate is red, after the
split and not before it. It needs an edit of the function, and no value a caller or a
file of the tree gives reaches it.

Not corrected in this unit. The pass named a fix (count every `package(` and every
`binary_id(` whatever the matcher), which is again a count of spellings. The check that
does not depend on the spelling compares the lists: no test of `swell::main` that the rule
prints "stack" or does not print is in the list of the run at opt-level 1. That is a
change of `test-split` with planted cases of its own; it is filed as a backlog draft
(`story:test-split-holds-the-stack-tests-of-swell-main-by-its-lists`).

## Limits that stay

- The awk that reads the JSON list depends on the indentation of `--message-format
  json-pretty`; an empty or changed list fails the step with a line (sub-cases of
  `test-lists-and-summed-line-count-the-same-tests.sh`).
- `TEST_OPT1_ARGS` is read for `-p <name>` and `--package <name>`; the forms `--package=x`
  and `-px` are not read, and two planted scripts go red when the value uses one.
- A new test of a moved binary lands in the run at opt-level 1 by the package term. One
  that needs unoptimized frames is named in `TEST_OPT0_TESTS` with its reason in the
  summary of the task `test`.
- The planted scripts use a stub of cargo that copies the membership and the JSON; it is
  not nextest. The second pass ran the split with the real lists for that reason.

## What went wrong in the unit

For about 40 s two whole gates of the unit's base ran at once on the build node, against
the rule of one pool job at a time. The unit cancelled both and ran the base gate again;
the numbers above are of that run.

## Verification before the merge

On `971ac76a` on the build node: `slot --timeout 30m task check-runner` exit 0 with the
lines above; `slot task gate-selftest` printed `gate-selftest: 114 passed, 0 failed` in the
unit's run and in the second pass's; `slot task kill-check` printed `kill-check: no kill
with two dashes after its signal operand in 152 file(s)`. The lane's `slow-tests` on base
and head: the collect mode's part at opt-level 0 went from `391 passed` to `374 passed`,
its part at opt-level 1 from `1083 passed` to `1100 passed`. The build before the clock
with a target directory of its own: 824 s on the base, 814 s on the head; `build-collect`
counts 10 test executables instead of 3.

What the story is for is read on the forge, on the pull request that carries this unit:
the test steps of the first attempt of `check` against 29.5 yardsticks and the budget of
120 s.
