---
title: KPIs
description: What makes an ECMAScript engine good for swell's purpose, KPI by KPI, how each is measured and where each engine's value comes from.
---

# KPIs

These are the measures of what makes swell good, K1 to K9, and the facts shown beside them. Each is a **gate** (judged on swell; it can fail a merge or a release), **compared** (shown beside other engines on the [compare page](/compare); it judges nothing), or both. The definitions are those of the design, [`docs/design/quality-and-comparison.md`](https://github.com/beyond10x/swell/blob/main/docs/design/quality-and-comparison.md), section 1.

Common terms:

- **Exact**: one value per run, no tolerance. **Sampled**: several runs, aggregated; the number of runs, the runner and the load bound are part of the definition.
- Every sampled value is measured on the pool's build node, never on a workstation: a job that finds itself on the machine that started the run refuses to measure ([`bench/run.sh`](https://github.com/beyond10x/swell/blob/main/bench/run.sh)). There is no container. The engines and the suite files are downloaded once into a directory on the build node, each checked against its pinned sha256 ([`bench/image/build.sh`](https://github.com/beyond10x/swell/blob/main/bench/image/build.sh)), and every measured process runs without network (`unshare -rn`), on two named CPUs (`taskset -c`) and under a 4 GiB data limit (`prlimit --data`) (`bench/run.sh`). The record of a run holds the node's 1-minute load average at the start and end of each job, CPU model, core count, kernel and `image_digest` (the sha256 of that directory's list of files and their sha256), and no host name.
- A figure is marked `loaded` when one of these held for its job, and `loaded_reason` names which: `neighbour` (another pool job ran on the node during the job), `load` (the node's 1-minute load per CPU at the job's start or end was over one) or `unwatched` (the run has no record of the node for the job). A figure made of others, such as a geomean, is loaded when any of them is. The compare page shows a loaded figure with the mark "loaded" and its reason beside it ([`bench/RUNBOOK.md`](https://github.com/beyond10x/swell/blob/main/bench/RUNBOOK.md), step 3; `taskfiles/site-engines.jq`).
- One unit per measurement: nothing is converted.

Where the values come from:

- **swell's own files**: the gate's committed files (`conformance/test262/expected.txt`, `Cargo.lock`, the crate roots, the host requirements). These values are as of the commit the site is built from.
- **Our runs**: the result files under [`bench/results/`](https://github.com/beyond10x/swell/tree/main/bench/results) (format `swell-bench/1`), written by `bench/run.sh` on the build node as `bench/RUNBOOK.md` describes; `task bench-schema-check` holds each to the schema. Each figure links to the file it comes from, and a figure measured while another job ran on the build node carries the mark `loaded`. A benchmark an engine could not run reads "not measured" with the engine's own error message.
- **Published**: [javascript-zoo-data](https://github.com/ivankra/javascript-zoo-data) (MIT), imported by `task bench-import-zoo` into `bench/published/`, each file with its data commit, sha256 and fetch date.
- **Documented**: the engine catalog, `bench/engines.jsonl`: one line per engine, a source URL per field, "unknown" where no source says.

The page `website/data/engines.json` is written from these by `task site-data`, and `task site-data-check` fails when it differs from its regeneration; the site types no number.

## K1

**test262 conformance**

- **Definition**: test262 cases passed at one pinned triple: the test262 revision, the harness and the exclusion list. In a comparison nothing is excluded: an unsupported feature counts as failed.
- **Unit**: cases. **Direction**: higher is better. **Kind**: exact.
- **Gate** (swell only): the passing case set must not lose a case. Today the gate holds the counts per directory (`test262-check`). **Compared**: the pass count, in total and per top directory of test262.
- **How measured**: a case passes when every mode that applies to it passes (as written and in strict mode).
- **Sources**: swell's value is its own runner's count at its pin, labelled "swell's runner" with the test262 commit. Two readings of it are shown: passed over every case at the pin (excluded and not applicable cases counted as failed), and passed over the cases the runner runs. Other engines' values are javascript-zoo-data's counts at test262 `4a1e962`, labelled "zoo runner", one outcome per test file. The two are at different revisions and runners: the page shows them side by side and ranks only within one source. A black-box runner of our own, run on every engine and on the swell shell at swell's pin, replaces both readings once it exists.

## K2

**Interpreter throughput**

- **Definition**: time per iteration of each benchmark of two suites, bench-v8 version 7 and [are-we-fast-yet](https://github.com/smarr/are-we-fast-yet); never a mean across both suites. Per suite, the geometric mean of the benchmark medians is a separate output.
- **Unit**: ms per iteration. **Direction**: lower is better. **Kind**: sampled, 5 runs, each a separate process, every iteration kept, timed inside the process with the shell's clock.
- **Compared** only; no release judges it yet.
- **Sources**: our runs. Interpreter configurations are compared with each other (V8 `--jitless`, SpiderMonkey and JavaScriptCore with their JITs off, QuickJS, Hermes, XS, Boa); JIT configurations are a separate band labelled "JIT reference", hidden by default. A benchmark an engine cannot run is a row with its message.

## K3

**Start**

- **Definition**: two measurements. Empty script: process start to exit of the shell running an empty file. New realm: time per realm over many calls of `$262.createRealm()` in one process.
- **Unit**: µs. **Direction**: lower is better. **Kind**: sampled, 10 runs, median with min and max.
- **Compared** only. A shell without `$262.createRealm` reads "not measured" with that reason.
- **Sources**: our runs. The harness measures the empty script today; the new realm is not measured yet (`bench/run.sh`, suite `start`).

## K4

**Memory**

- **Definition**: peak resident set size of the engine's process on a pinned workload: `ru_maxrss` of `getrusage(RUSAGE_CHILDREN)`, read by the harness's helper ([`bench/tool`](https://github.com/beyond10x/swell/tree/main/bench/tool)) after it has reaped that one process: the empty script, and bench-v8 Splay at the iterations `bench/suites/iterations.tsv` names. For swell also the bytes of an empty realm, from its own allocator count (the charge to `memory_bytes`).
- **Unit**: bytes. **Direction**: lower is better. **Kind**: peak RSS sampled (median of 10 runs); the empty realm exact.
- **Compared**: peak RSS. **Gate** (swell only, can gate): bytes of an empty realm.
- **Sources**: our runs.

## K5

**Binary size**

- **Definition**: bytes of the shell binary, for target `x86_64-unknown-linux-gnu`, profile `release`, stripped with `strip --strip-all`, built with the toolchain of `rust-toolchain.toml`. A peer's binary names the same four, or is labelled "as distributed" with its stripped state; a shell that loads shared libraries from its own directory counts them and says so.
- **Unit**: bytes. **Direction**: lower is better. **Kind**: exact.
- **Gate** (swell): does not rise against the accepted value. **Compared**: yes.
- **Sources**: our runs. swell has no shell binary yet.

## K6

**Resource control for untrusted scripts**

- **Definition**: what a host can bound when it runs a script it does not trust: a memory cap, a step or instruction budget, an interrupt or terminate call, a stack or recursion limit.
- **Gate** (swell, exact): no step beyond the step budget, no byte beyond the memory budget, the agent runs the next script after each of its limits, and the interrupt's latency in steps. **Compared**: a capability matrix per engine.
- **Unit**: none; each cell is the documented capability in words. **Direction**: none.
- **Sources**: swell's cells cite the line of its source that defines each limit. Other engines' cells are the engine catalog's fields, each with its source URL (an API header, the documentation). A measured overshoot for other engines needs an embedding harness per engine and is not part of the page.

## K7

**Safety**

- **Definition**: crate roots without `#![forbid(unsafe_code)]`, and packages of `Cargo.lock` from outside the repository; a fuzz run over a fixed corpus and seed, once a fuzzer exists.
- **Unit**: crate roots; packages. **Direction**: more crate roots with the line is better; fewer outside packages is better. **Kind**: exact.
- **Gate** (swell): `forbid-unsafe-check` and `lock-check`. **Compared** as context only: whether an engine's implementation language is memory-safe, from the catalog. Vulnerability counts are not a row: they follow how widely an engine is used and fuzzed as much as its quality.
- **Sources**: swell's counts are the scan `task site-data` makes of the crate roots and `Cargo.lock`; the memory-safety cell of other engines is the catalog's.

## K8

**Determinism**

- **Definition**: the same script with the same host answers gives the same output bytes and the same step counts.
- **Gate** (swell): a test replays a corpus twice and compares (host requirement `HOST-133`). **Compared**: whether an engine offers a deterministic step count, from the catalog.
- **Unit**: none. **Direction**: none.
- **Sources**: swell's cell cites the requirement line and its test. The catalog has no determinism field yet, so other engines read "unknown".

## K9

**Development**

- **Definition**: the gate's test steps end within their wall-time budget, and no single test runs longer than its bound (`GATE_TIME_BUDGET`, `.config/nextest.toml`), with queue wait apart.
- **Gate** only; it is not compared and has no row on the compare page.
- **Sources**: the gate's own `gate-time` lines.

## Facts

Not KPIs: per engine, with a source per cell, the implementation language, the licence, the execution model, whether an interpreter-only mode exists, the latest release, and the releases in the last 12 months. For swell they come from `Cargo.toml` and the design; for other engines from the catalog.

Not shown, on purpose: raw JIT peak throughput against an interpreter, SunSpider, Kraken and Octane (retired by their authors), vulnerability counts, and GC pauses (no data across engines).
