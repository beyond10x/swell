---
format: aep.planning-md/3
id: story:realm-size-lane-within-its-share
kind: story
status: draft
title: The realm-size step of the lane slow costs 610 s on the forge; it is brought back near the lane's earlier time
summary: task slow took 26 min 25 s on pull request 6 against 10 min 57 s before; test-realm-sizes 610.2 s plus a 3 min 47 s collect build.
revision: 1
---
Measured on the forge, 2026-11-25, pull request 6 (the slice 4 batch with
`story:tests-hold-at-any-size-of-the-realm`). Draft: not scoped, not ordered.

## Context

- The step `task slow` of `.github/workflows/slow.yml` took 26 min 25 s on pull request 6
  (job 139103582979, 09:35:07Z to 10:01:32Z). On pull request 5, the release 0.10.0 without the
  batch, it took 10 min 57 s (job 139052731087).
- Its log: `slow-time test-realm-sizes 610.2`; before it `build-realm-sizes` built the `collect`
  profile in 3 min 47 s. `slow-tests` and `slow-tests collect-every-safe-point` ended at 09:46:58Z,
  about 12 min after the start, as before.
- So `test-realm-sizes` and its build cost about 14 of the 26 minutes.

## Change (to be decided at decomposition)

- Measure where the 610 s go: per size of `REALM_SIZES`, per mode, per test.
- Bring the step back near the lane's earlier time without dropping a size that found a defect:
  fewer sizes, one mode for the sizes between the ends, a test list limited to the tests that
  read the realm's room, or a build shared with `slow-tests`.
- Acceptance: `task slow` on the forge within 15 min on a pull request that changes the engine,
  with `slow-time test-realm-sizes` printed.
