# Running the benchmarks

The commands below run the comparison of `docs/design/quality-and-comparison.md` (K2 to K5) on the
build node. Every measuring step is a pool job on the node (`slot`, `--timeout` at most 30
minutes); none runs on the workstation, and each job refuses to measure when it finds itself on
the machine that started it (the origin digest, the first 16 hex digits of the sha256 of the
starting machine's `/etc/machine-id`).

Run from the repository root of a managed worktree, in this order:

1. Smoke: image, tool, one hello script per configuration, the flag variants.

   ```sh
   slot --timeout 20m sh bench/smoke.sh "$(sha256sum /etc/machine-id | cut -c1-16)"
   ```

   Read `.out/bench-probe/` for each engine's "hello 2" line and which of `class`, destructuring,
   `performance.now`, `preciseTime` it has; set flags in `bench/image/configs.tsv` from it.

2. Pilot: one job of one configuration and one suite with one process per benchmark (RUNS 1), to
   read `iteration_ms` of each benchmark and set `bench/suites/iterations.tsv`. A pilot starts
   only when the build node lists no job of any repository, one at a time, never two:

   ```sh
   o=$(sha256sum /etc/machine-id | cut -c1-16)
   slot --timeout 30m --tokens 2 sh bench/run.sh job boa are-we-fast-yet 1 "$o" 1
   ```

   Run it for the slowest configuration of each suite (boa on both, see "The counts" below).
   The job's rows are between `== rows begin` and `== rows end` of its output.

   Every job and `prepare` check the file before they build or measure anything: each inner
   count of are-we-fast-yet must be one that the benchmark's own `verifyResult` in the image
   accepts, and each count must be a positive whole number. Otherwise the step stops with
   "holds counts the suite does not verify; nothing measured" and one line per benchmark:
   its count and the counts its module verifies. A benchmark whose `verifyResult(result)`
   checks each call, or that checks itself (DeltaBlue), takes any count. The selftest
   `tests/gate/bench-job-refuses-an-inner-count-the-suite-does-not-verify.sh` holds this.

   A benchmark that fails is `not-measured`. Its reason is the engine's own message: the first
   line of stderr that is not a stack line, or else of stdout, cut to 200 characters (selftest
   `tests/gate/bench-job-names-the-engine-error-of-a-failed-benchmark.sh`).

3. The run: one pool job per (configuration, suite), one at a time. The orchestrator itself only
   waits, so it starts without the `slot` prefix; each job it starts carries it. This host admits
   one benchmark job at a time, and a run starts only when `slot status` shows no other job of
   this repository on the node.

   ```sh
   sh bench/run.sh all 1
   ```

   It writes `bench/results/<date>-<commit>.json` and stops at the first job the pool refuses.

4. The gate, then the site data:

   ```sh
   slot --timeout 30m task check-runner
   task site-data
   ```

## The counts

`bench/suites/iterations.tsv` names, per benchmark, the iterations of one process (`iterations`)
and, for are-we-fast-yet, the argument of `innerBenchmarkLoop` (`inner`). The suite is pinned at
commit `f35aed0a61171594804a44efe9d75f1f35360700`; its `verifyResult` knows these inner counts:

| Benchmark | Inner counts the suite verifies | Inner | Iterations |
|---|---|---|---|
| Bounce, List, Permute, Queens, Richards, Sieve, Storage, Towers, Json | any (each call is checked) | 15, 15, 10, 10, 1, 30, 10, 6, 1 | 5 |
| DeltaBlue | any (the benchmark throws on a wrong result) | 120 | 5 |
| CD | 1000, 500, 250, 200, 100, 10, 2 | 10 | 5 |
| Havlak | 15000, 1500, 150, 15, 1 | 15 | 2 |
| Mandelbrot | 500, 750, 1 | 500 | 2 |
| NBody | 250000, 1 | 250000 | 2 |

By their code, Mandelbrot at 1 computes one pixel and NBody at 1 one step, far below the
millisecond of `Date.now`, the only clock of boa, xs and hermes; so they take 500 and 250000.
Havlak's 51 loop searches of its graph do not depend on the count, so 1 would not be cheaper
than 15. The iterations of Havlak, Mandelbrot and NBody are 2 so that the slowest job ends
within 20 minutes. bench-v8 runs 20 iterations of each benchmark.

A full run starts each (configuration, suite) as one job with five runs, killed after 30
minutes. The pilot jobs of 2026-11-27 (one run each, these counts, `iteration_ms` in their rows)
measured, in seconds per run (`job` is the node's own record of the job, without the sync):

| Configuration | Suite | Measured rows | Slowest benchmark, ms per iteration | Job, s | Five runs, min |
|---|---|---|---|---|---|
| boa | are-we-fast-yet | 13 of 14 | Havlak 45123 | 122 | 10.2 |
| xs | are-we-fast-yet | 14 of 14 | Havlak 48099 | 115 | 9.6 |
| spidermonkey-interp | are-we-fast-yet | 14 of 14 | Havlak 5880 | 23 | 1.9 |
| quickjs | are-we-fast-yet | 14 of 14 | Havlak 3568 | 22 | 1.8 |
| hermes | are-we-fast-yet | 13 of 14 | Havlak 2967 | 9 | 0.8 |
| jsc-nojit | are-we-fast-yet | 14 of 14 | Havlak 1784 | 7 | 0.6 |
| boa | bench-v8 | 8 of 8 | EarleyBoyer 1801 | 132 | 11.0 |
| xs | bench-v8 | 8 of 8 | EarleyBoyer 1347 | 65 | 5.4 |
| hermes | bench-v8 | 8 of 8 | RegExp 144 | 9 | 0.8 |
| v8-jitless | bench-v8 | 8 of 8 | Crypto 89 | 22 | 1.8 |

The quickjs job ran Havlak at 5 iterations. The boa and v8-jitless bench-v8 jobs ran on
2026-11-27 before this table's other jobs; they are timed by their logs on the workstation, with
the sync. The five-run column is five times the job. Other pool jobs started on the node beside
most of these jobs (the pilot checks the node only at its start), so the numbers set the counts
and are not results.

The slowest configuration is boa on both suites: 11.0 minutes for bench-v8 and 10.2 for
are-we-fast-yet. The pairs without a pilot job are taken to fit because each would have to be
nearly twice as slow as boa: spidermonkey-interp and jsc-nojit on bench-v8 and
v8-jitless on are-we-fast-yet are interpreters whose pilot on the other suite took at most a
fifth of boa's job; quickjs on bench-v8 and quickjs-ng on both suites are QuickJS, whose
are-we-fast-yet job took 22 s against boa's 122; v8, spidermonkey and jsc compile to machine
code and are not slower than their own interpreter configurations.

DeltaBlue does not run on boa 0.20 ("Uncaught ReferenceError: access to uninitialized binding")
and hermes 0.13 ("Uncaught TypeError: Cannot read property 'at' of undefined"): both stop in the
static class fields of its `Strength`. Those rows and the suite's geomean stay `not-measured`.
The `start` suite runs the empty script and bench-v8 Splay ten times each; boa's Splay process
of 20 iterations measured 0.74 s in its bench-v8 pilot.
