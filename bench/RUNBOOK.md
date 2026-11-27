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
   one benchmark job at a time.

   ```sh
   BENCH_NODE_STATE='<command that prints the JSON of `slot state` on the build node>' \
     sh bench/run.sh all 1
   ```

   `BENCH_NODE_STATE` is a shell command the operator fills in: it prints the node's job list,
   the JSON of `slot state` run on the build node (`.now_ms`, `.active[]`, `.done[]`, each job
   with `.id`, `.started_ms`, `.ended_ms`, `.repo`, `.argv`). Without it, `all` refuses to start:
   a run whose rows cannot say whether they were measured alone is not started.
   `BENCH_ALLOW_UNWATCHED=1` runs it anyway, and then every row is `loaded: true` with
   `loaded_reason` `unwatched`.

   Before each job, `all` reads the node's job list every 10 s until the node runs no pool job
   of any repository, at most 600 s (`BENCH_IDLE_POLL_S`, `BENCH_IDLE_WAIT_S`); then it starts
   the job either way, after "the node is not idle after 600 s; starting job <config> <suite>
   beside it". After each job, it reads the list again and writes
   `logs/<config>.<suite>.neighbours.json` beside the job's log: the node's record of the job
   (id, start, end) and every other pool job whose time overlaps it (id, repository, start,
   end, the first 80 characters of its command with `/home/<name>/` cut to `~/`).

   It writes `bench/results/<date>-<commit>.json` and stops at the first job the pool refuses.

   Every row says whether it was measured alone. `loaded: true` means one of these held for its
   job, and `loaded_reason` names which (a list when more than one holds):

   - `neighbour`: another pool job ran on the node during the job;
   - `load`: the node's 1-minute load per CPU at the job's start or end was over
     `limits.loaded_bound_per_cpu`;
   - `unwatched`: the run had no record of the node for the job (no `BENCH_NODE_STATE`, or the
     node's list had no record of it).

   `loaded: false` means none held, and the row has no `loaded_reason`. The result's `loaded`
   names this rule in one line and counts the jobs, the jobs with a neighbour and the rows
   loaded. `task bench-schema-check` refuses a row with `loaded: false` and a `loaded_reason`,
   and a row with `loaded: true` without one. The selftests `tests/gate/bench-all-*.sh` hold the
   wait, the refusal and the marks.

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
