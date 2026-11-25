---
format: aep.planning-md/3
id: story:swell-in-the-benchmarks
kind: story
status: draft
title: swell measured beside the other engines
summary: The released Linux binary in the bench image; swell's K2 to K5 rows measured or refused with a reason
revision: 2
---
Wave B of the binaries plan, 2026-11-25. Needs the build node (benchmarks do not run on the
workstation). Depends on `story:releases-ship-the-shell-for-three-systems` (the first release
with assets) and `story:bench-image-and-first-measurements` (the image and the runner).

## Context

- The compare page reads K2 to K5 from `bench/results/*.json`; swell's rows read "not measured: no
  binary yet" (`taskfiles/site-engines.jq`).
- `bench/image/engines.tsv` pins each engine by version, archive URL and sha256;
  `bench/image/configs.tsv` lists the configurations.

## Change

- `bench/image/engines.tsv`: a `swell` line for the release archive
  `swell-<version>-x86_64-unknown-linux-gnu.tar.gz` of the newest release, URL and sha256 from its
  `SHA256SUMS`; `configs.tsv`: `swell` (interpreter), flags `--steps max` and the memory limit of
  the run, recorded.
- `bench/run.sh all` measures swell like every other configuration; the result file has swell
  rows with status `measured`, or `refused` with the engine's message for a benchmark swell
  cannot run yet (for example a construct of a later slice).
- K5 for swell is the size of the released binary (stripped by the release build), with target
  triple, profile and toolchain in the row.

## Acceptance

- One committed result file with K2, K3, K4 and K5 rows for swell beside the eight default
  engines; every swell row is `measured` or `refused` with a reason, none `not-measured`.
- `/compare` shows swell's K2 to K5 cells; `slot --timeout 30m task check-runner` exits 0.
