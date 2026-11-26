---
format: aep.planning-md/3
id: story:realm-size-lane-within-its-share
kind: story
status: active
title: The lane slow ends within 15 minutes on the forge and each of its parts within 10
summary: 'task slow took 22 min 35 s on pull request 19; two profile builds are 433 s of it and test-realm-sizes 524 s. Bound: 900 s for the lane, 600 s per part.'
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-26T20:04:08Z", actor: "human:operator", revision: 5, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-26T20:04:08Z", actor: "human:operator", revision: 6, executor: "agent:coordinator"}
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

The operator, 2026-11-26 (D53, option A): the lane `slow` gets 15 minutes on the forge and 10
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
