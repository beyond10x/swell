---
format: aep.planning-md/3
id: story:bench-image-and-first-measurements
kind: story
status: active
title: Pinned engines in one image, and the first K2 to K5 measurements
summary: bench-v8 v7, are-we-fast-yet, start, peak RSS and size for the default engines, through the pool, in swell-bench/1
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-25T12:46:20Z", actor: "human:operator", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-25T12:46:35Z", actor: "human:operator", revision: 4}
---
The owner, 2026-11-25: evaluate the KPIs for other ECMAScript engines and for swell; benchmark
runs go through the shared build pool ("Benchmark runs can just be run through slot"). Design:
`docs/design/native-code.md` § 5 and Q7, Q12; `docs/design/quality-and-comparison.md` §§ 1, 4,
5 (K2 to K5). Stories 3, 4, 5 of `native-code.md` § 7 and story 12 of
`quality-and-comparison.md` § 9 in one unit. Base: `plan/native-code` at `541d32e`.

## Context

- The build node is x86_64, 32 cores, with docker and podman and no `/dev/kvm`; a pool job runs
  there with `slot --timeout 30m <cmd>` and is killed at 30 minutes.
- jsvu installs V8, SpiderMonkey, JavaScriptCore, QuickJS-ng, Hermes, XS and GraalJS without
  root; esvu adds QuickJS, Boa and engine262. Interpreter-only flags: V8 `--jitless`;
  SpiderMonkey `--no-ion --no-baseline --no-blinterp` (unverified); JavaScriptCore
  `JSC_useJIT=false` (unverified).
- The result schema `swell-bench/1` is agreed with codespec (`native-code.md` Q12): named and
  versioned; a refused script is a row with its reason.

## Change

Failing checks first (Acceptance). Then:

- `bench/image/`: a container definition that installs the eight default engines by pinned
  version (sha256 recorded) and the suites bench-v8 v7 and are-we-fast-yet by pinned commit;
  the run uses `--network none`, a pinned cpuset and a memory limit.
- `bench/run.sh` (POSIX sh) or a Rust tool in a tooling workspace under `bench/` (no Python):
  one process per measurement; for each engine and mode (interpreter-only, JIT reference):
  K2 per benchmark of each suite (median of N runs, N in the record) and the suite's geometric
  mean as its own output; K3 empty script, process start to exit, in ms; K4 peak RSS of a pinned
  workload in KiB; K5 size of the engine binary in bytes.
- `bench/schema/swell-bench-1.json` and a schema test over every committed result file.
- `bench/results/<date>-<commit>.json`: the first run, committed. swell's rows read "not
  measured: no binary yet" until `story:cli-runs-a-script-file` lands; then they are measured
  the same way.
- `task bench-run` (manual) runs it through the pool, one job per (engine, suite), each named
  with `--timeout`; a job that does not fit is split.

## Acceptance

- The schema test refuses a planted result file with a missing `version` or `runs` and passes on
  the committed file.
- `bench/results/` holds one run with K2, K3, K4 and K5 rows for the eight default engines in
  interpreter-only mode, and JIT reference rows for V8, SpiderMonkey and JavaScriptCore; each row
  names version, flags, runs, host and date.
- `slot --timeout 30m task check-runner` exits 0.
