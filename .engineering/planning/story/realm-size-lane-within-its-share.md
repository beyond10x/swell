---
format: aep.planning-md/3
id: story:realm-size-lane-within-its-share
kind: story
status: implemented
title: The lane slow ends within 15 minutes on the forge and each of its parts within 10
summary: 'task slow took 22 min 35 s on pull request 19; two profile builds are 433 s of it and test-realm-sizes 524 s. Bound: 900 s for the lane, 600 s per part.'
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-11-26T20:04:08Z", actor: "human:operator", revision: 5, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-26T20:04:08Z", actor: "human:operator", revision: 6, executor: "agent:coordinator"}
- {from: "active", to: "implemented", at: "2026-11-27T08:38:03Z", actor: "human:operator", revision: 9, decided_on: {"recorded":{"test_result":1,"review_outcome":10,"verification":5}}, executor: "agent:coordinator"}
---
A defect of the lane `slow`, not a feature. Ordered ahead of the next feature units of wave
big-2 on the operator's order of 2026-11-26; no feature unit of the wave merges before it.

## Context

- The step `task slow` of `.github/workflows/slow.yml` took 26 min 25 s on pull request 6
  (job 139103582979, 2026-11-25, 09:35:07Z to 10:01:32Z). On pull request 5, the release 0.10.0
  without the batch of slice 4, it took 10 min 57 s (job 139052731087).
- Its log then: `slow-time test-realm-sizes 610.2`; before it `build-realm-sizes` built the
  `collect` profile in 3 min 47 s.
- On pull request 19 (run 45884135882, head `d5e2aac9`, 2026-11-26) the step took 22 min 35 s
  (19:37:24Z to 19:59:59Z). Its lines:
  - `slow-time slow-test262 409.7` (the full test262 run, 85,556 runs);
  - `slow-time slow-tests 174.5`;
  - `slow-time test-realm-sizes 524.3`, of which `build-realm-sizes` built the `collect`
    profile in 3 min 33 s; 14 runs, 7 sizes in 2 modes, each of 1,343 and 1,339 tests;
  - `slow-time stack-profiles 246.4`, of which `build-stack-profiles` built the `release`
    profile in 3 min 40 s and `stack-debug` in 24.47 s.
- The two profile builds are 433 s of the 1,355 s.
- The gate's test steps are a different step and within their rule: `gate-time test-steps 39.7
  of 120` on the same head.
- The lane found a defect on 2026-11-26 that the gate did not: two tests red at realm size 200
  (pull request 19, fixed by `5078361a`). The size that found it stays.

## Decision

The operator, 2026-11-26 (option A): the lane `slow` gets 15 minutes on the forge and 10
minutes for each of its parts; the two profile builds are shared; the lane keeps running on
every pull request that changes the engine.

## Change

1. Measure first, on the build node, on the unit's base: per part of the lane the seconds of
   its builds and of its runs; for `test-realm-sizes` per size and mode; which tests of swell
   read the realm's room (the tests a padded realm can move). The numbers go into the report.
2. Share the builds: no part of the lane compiles a profile that another part or the gate has
   already compiled for the same sources, where one profile can serve both. Candidates, to be
   decided by the measurement: the `collect` profile of `build-realm-sizes` and the collect
   build of `slow-tests`; the `release` profile of `build-stack-profiles` and the release
   build of the test262 runner. A profile a test's subject depends on (the unoptimized frames
   of the stack tests, `stack-debug`) keeps its build.
3. Bring `test-realm-sizes` down without dropping a size that found a defect and without
   dropping the mode that found it: a test list limited to the tests that read the realm's
   room, or one mode for the sizes between the ends. Each cut names what it no longer runs
   and why no defect of the kind the step exists for hides there.
4. The lane states its bound and holds it: `task slow` fails when the lane ran over 900 s, and
   when one part ran over 600 s, with the line that names the part and its seconds. The
   bounds are read from one place in `Taskfile.yml` or the lane's task file.
5. No raised timeout, no shard, no part moved off pull requests.

## Acceptance

- `task slow` on the forge, on a pull request that changes the engine, ends within 15 minutes;
  each `slow-time <part> <seconds>` line is under 600; the step's own start and end times are
  quoted from the run.
- On the build node, each part run as its own pool job ends within 10 minutes without
  `slot --timeout`.
- A gate selftest under `tests/gate/` goes red when a part over its bound is not refused, and
  one when the lane over its bound is not refused (a planted time, no real run).
- With `5078361a` reverted in a copy of the tree, `task test-realm-sizes` is red at size 200
  and names the two tests (`misuse` and `termination`), as it was on pull request 19 before
  the fix.
- `task check-runner` exits 0 with `gate-time test-steps` under 120.
- The report gives the lane's times before and after, per part, from runs on the build node.

## Writes

`Taskfile.yml`, the lane's task files under `taskfiles/`, `.cargo/` profile files where a
profile is shared, `.github/workflows/slow.yml` only where a step's command changes,
`tests/gate/` (the two selftests). The coordinator's: `AGENTS.md` (the lane's bound),
`CHANGELOG.md`.

## As delivered

Four units of wave big-2 delivered the story: `slowlane` (the parts are timed and bounded),
`lanebuilds`, `lanetests` and `lanefour`; `docs/plan/wave-big-2.md` has the row of each. The
first three brought the lane on the forge to 1,137.3 s of 900 (run 45925590365 on
`158888d`: parts 419.3, 268.4, 431.2 and 18.4). The fourth:

Unit lanefour measured the lane part by part on the build node, with builds and runs
separate. It took two of the four candidates and one change of how the lane runs.

- **(a) Taken.** The lane's collect mode runs at opt-level 1, apart from the tests that run
  on a bounded stack (71 on the unit's head, 73 after its merge), the tests a macro writes,
  and the library's unit tests (381 at opt-level 0, 1,034 at opt-level 1 on the unit's head).
  - The cost is one cold build of the profile `collect`: 3 min 33 s on the forge.
  - The seven collect runs of `test-realm-sizes` went from 312 s on the forge to an
    estimated 32 s.
  - The collect runs of `slow-tests` went from 128.4 s to 39.4 s, on 4 test threads on the
    node.
  - A guard holds the split: a stack test in the run at opt-level 1 fails there. Its first
    run found 14 stack tests a macro writes that ran in both runs; they now run at
    opt-level 0 only.
- **The build beside the lane's first part.** The lane starts the build of the profile
  `collect` right after `gate-time-start` and `slow-tests` waits for it. On 4 CPUs of the
  node the build took 250.7 s beside `slow-test262`, which took 56.6 s longer than alone
  (347.8 against 291.2); `slow-tests` waited 0 s. No cargo lock wait was logged.
- **(b) Taken.** A character class keeps its last answer for the range around it. The
  property-escape cases run 6 % faster with the same steps, results and expected file.
- **(c) Dropped.** A nextest start over the list costs 0.085 s and a process start 1.5 ms.
  The 16 s of a default-mode run is test work.
- **(d) Not taken.** It needs a claim that no defect hides at the cut sizes. The record
  contradicts that claim.

Measured on the forge on the tree merged with wave big-2 (`wave/big-2` at `b9adbe35`, run
45963552396, started by hand, 2026-11-27, 09:09 to 09:24 CET, green): `slow-time lane 882.6
of 900, longest part slow-test262 504.8 of 600`, the parts 504.8, 170.4, 185.2 and 22.2, the
build of the profile `collect` 382.2 s beside the first part with `slow-tests` waiting 0 s.
The story's bound holds with 17.4 s of margin, from 1,137.3 s. The unit's estimate for that
tree was about 848 s and its own aim 800 s; neither is met. Measured on the build node: the
same head as one job 454.4 s on 32 cores (parts 272.8, 76.9, 103.8 and 0.9); cold on the
unit's head 444.2 s on 32 cores after its merge of the wave (424.1 s before it) and 619.8 s
on 4 CPUs before it.

On that margin `story:property-escape-cases-spend-less-in-the-interpreter-loop` entered the
wave as a fifth unit of the lane (`approval-record:wave-big-2-lane-unit-5`); it is a story of
its own and not a part of this one.

The reviews of the fourth unit: `review-result:adversary-lanefour-pass-1`,
`review-result:adversary-lanefour-pass-2` and the coordinator's read of its second correction
(`review-result:lanefour-correction-2-coordinator-read`).
