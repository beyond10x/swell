---
format: aep.planning-md/3
id: review-result:laneeight-step-2-coordinator-read
kind: review-result
status: active
title: 'Coordinator''s read of step 2 of the unit laneeight after two adversary passes: ready to merge, the forge''s lane run is open'
relations:
- reviews: story:slow-lane-holds-its-bound-on-the-slower-runner
revision: 1
---
Coordinator's read of step 2 of `story:slow-lane-holds-its-bound-on-the-slower-runner`
(the lane's eighth unit of wave big-2) after two adversary passes and two corrections,
2026-11-28. Verdict: ready to merge into the wave branch with step 1; the story stays
active until the forge's run of the lane on the slower runner.

What step 2 is. The lane's runs that can move run at opt-level 1: the default mode of
`slow-tests`, the library part of its collect mode, and the default mode of
`test-realm-sizes` at each size. The stack tests and the tests of `swell-syntax` stay at
opt-level 0. `test-split` splits a run of the lane as it splits the gate's, the lane makes
three builds at opt-level 1 in the background with the full counts, and each moved run
prints its counts and fails on a cargo `Compiling` line.

The readings, on the build node (32 cores; the unit's two runs of the lane, one before on
step 1's head and one after):

- `slow-time lane` 383.8 s and 312.5 s of 900; `slow-tests` 63.4 and 20.8;
  `test-realm-sizes` 49.1 and 32.1; `slow-test262` 245.8 and 258.9 of 600.
- `slow-build opt1 71.1 s`, `lib 34.0 s`, `collect 1.0 s`, each exit 0, waited 0 s; before:
  `slow-build collect 57.1 s`.
- `slow-tests: 977 tests at opt-level 1, 116 at opt-level 0`; the library part 287 and 0;
  `realm-sizes: 314 tests at opt-level 1, 50 at opt-level 0` at each size.
- The set of test names of each run, before against after: equal, but for the profile
  test, which the two default runs gain.
- The unit's gate on its first head: exit 0, `gate-time test-steps 43.8 of 120`.

Pass 1 (on `fddb4a52`): red, 7 findings, 2 failing cases. What stood: the two parts of
each split are complements by construction, the guard is set in every moved run, each
build and the runs that read it agree in variable, flags and profile, the gate's own call
of `test-split` prints the same bytes as on the base, only the nine tasks changed, and
the bounds 900 and 600 are unchanged. The counts that differ from the probe are explained:
45 stack tests are the probe's 44 stopped by the guard and one that passes under it; 313
moved realm tests against 305 are 8 tests added after the probe's base. What was found
and what became of it:

- A caller's `SPLIT_*` value, on the command line or in the environment, made the gate's
  step `test` and the lane's `slow-tests` leave a test out of both parts and pass. Taken:
  the gate's call passes each variable empty, the lane's calls pass each one,
  `test-opt1-vars-check` refuses a caller's value for the gate's steps, the lane and a
  part run alone, and `GATE_VARIABLES` lists the seven. The tasks `gate-vars` and `test`
  joined the unit's scope for it, by the coordinator's ruling.
- A count file of an earlier run was printed for a part that did not run. Taken.
- An exclusion list with no line wrote a filter nextest cannot read. Taken: no term.
- `slow-tests` waited up to 400 s for each of three builds inside a part bounded at 600 s.
  Taken: 400 s for the three in sum.
- Three mutants of the unit's lines passed all seven lane scripts. Taken: three
  assertions, each failing under its mutant.
- A note on the task runner's shell: a `continue` inside an `if`, a `case` clause or a
  brace group does not skip the rest of that block. The unit listed every shell
  `continue` of the task files: each is at its loop's level or the last command of its
  block; the one that was not is gone.

Pass 2 (on `2e278984`, the first correction): green, one note. Each of the four fixes
fails its own script when taken out alone; the three new assertions fail under their
mutants applied anew; real nextest reads the filter `all()`; the step `test` prints the
same bytes on the base and the head with no caller value; a caller's `SPLIT_RUN` is
refused by `build-tests`, `test`, `slow-tests` and `test-realm-sizes` before anything is
built, listed or run. The note: with the seven refusal lines deleted, all 119 scripts of
the gate's self-test pass, because the two other layers keep a caller's value out.

The second correction (`c6958520`, one file under `tests/gate/`, 43 lines): three cases in
`slow-refuses-a-caller-value-of-its-runs-at-opt-level-1.sh`. `task test` with two `SPLIT_*`
values on the command line, and `task slow-tests` and `task test-realm-sizes` with values
in the environment, each exit non-zero, print `<step>: SPLIT_RUN=x is given by the caller`
and leave the stub's log empty. With the seven refusal lines removed the script fails at
its first new case; the two further cases read the same lines and are not run under the
mutant, since the script stops at its first failure. `gate-selftest: 119 passed, 0 failed`.

Checked by the coordinator: the head of the unit's branch, its diff against step 1's head
(`Taskfile.yml` and files under `tests/gate/` only), and the whole gate and the whole
lane on the wave branch after the merge; their lines are the story's verification.

What the story is for is read on the forge, on the pull request that carries the unit:
`slow-time clock` at or under 900 and the longest part at or under 600 on a runner whose
line names the AMD EPYC 7763. The seconds of test262 itself on that runner are read there
for the first time. Inferred, not measured: the part `slow-test262` is the one nearest to
its bound.

```findings
[]
```
