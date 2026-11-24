---
format: aep.planning-md/3
id: story:gate-test-steps-fit-four-cores
kind: story
status: implemented
title: The gate's test steps fit the forge's four cores; the local gate bounds their CPU seconds where it has an entry and refuses a build inside the clock everywhere
summary: 'The forge''s runner of four cores took 127.9 s for the test steps of release 0.8.0 where the local gate took 94.3 s; the collect mode runs at opt-level 1 in a profile of its own, and the release runner of test262 is built before the clock instead of inside gate-selftest. check-runner bounds the CPU seconds of the test steps on a machine whose core count has an entry in a table of CPU seconds per kind of machine (16 cores: 245, provisional) and prints them unbounded on any other, and on every machine refuses a run that writes into target/*/deps or conformance/ess/target/*/deps inside the clock; the forge judges its own time.'
relations:
- decomposes: epic:foundation
- depends_on: story:gate-test-steps-cost-half
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T23:54:01Z", actor: "human:operator", revision: 2, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-23T23:54:01Z", actor: "human:operator", revision: 3, executor: "agent:coordinator"}
- {from: "active", to: "implemented", at: "2026-11-24T08:04:07Z", actor: "human:operator", revision: 9, decided_on: {"recorded":{"test_result":2}}, executor: "agent:coordinator"}
---

## Context

A defect under the organization's rules "Tests stay fast" (the test steps of a gate end within
120 s; a step over the budget is fixed before the next feature; a longer timeout, a shard or
waiting longer is not a fix) and "The local gate equals the forge" (when the forge goes red after
a green local gate, the fix includes the change that makes the local gate catch the same
failure). Release 0.8.0 waits for it.

On `d9d1b95` (`main`, the code of release 0.8.0) the local gate was green and the forge's `check`
red on its time line alone:

- forge, 4 cores: `gate-time test-steps 127.9 of 120, cpu 448.5, load 3.73 on 4 cores`; test
  31.3, test-doc 3.2, test-collect 57.9, test-faults 0.2, conform-engine 1.4, test262-check 0.1,
  test262-moves-check 1.5, gate-selftest 32.3;
- `slot --timeout 30m task check-runner` on the same code (`a350de3`), load 26 to 34, `nproc` 16
  inside the pool job (the machine has 10 cores, 20 hardware threads): `gate-time test-steps 94.3
  of 120, cpu 312.1, load 33.84 on 16 cores`; test 28.5, test-doc 3.7, test-collect 43.6,
  test-faults 0.2, conform-engine 1.6, test262-check 0.1, test262-moves-check 7.2, gate-selftest
  9.4.

### The runner

Its wall time follows its CPU seconds: 0.285 s of wall per CPU second on the red run, 0.290 to
0.306 on the ten other runs of `check` on `main` with a time line (`forge-runs.txt` of the
coordinator), so the fix is fewer CPU seconds in the test steps, not more parallelism. The
runner comes in two speeds. Pairs of runs of the same tests, steps unchanged between them:

| tests (test / collect) | fast run | slow run | slow / fast |
|---|---|---|---|
| 977 / 796 | `635dd5b` 73.0 s, cpu 241.3 | `4e71892` 117.1 s, cpu 393.9 | 1.60 |
| 858 / 684 | `c08452a` 76.3 s, cpu 249.5 | `3cec594` 99.0 s, cpu 327.9 | 1.30 |

The yardstick of a run's speed is the wall time of `conform-engine` plus `test262-moves-check`,
fixed work that grew little over these runs: 1.7 s on the fast runs (`e0bb1ec`, `635dd5b`,
`1f21c0a`), 2.1 to 2.2 on the middle ones, 2.8 to 2.9 on the slowest (`a6f22f6`, `4e71892`,
`3cec594`, `d9d1b95`). The red run has the slowest figure (1.4 + 1.5 = 2.9). Release 0.7.0
(`e0bb1ec`, 69.0 s, cpu 234.0) ran on a fast one; on the slowest it would have taken about
69.0 × 1.52 = 105 s (1.52 = 3.2 / 2.1, the ratio of `test-doc` between them). From 0.7.0 to
`d9d1b95`, at the slowest speed: test 24.8 to 31.3 (1,104 to 1,215 tests), test-collect 42.1 to
57.9 (864 to 934 tests), gate-selftest 32.0 to 32.3; the rest of the 58.9 s is the runner's
speed. Every bound and margin below is stated against the slowest runner seen.

### Where the CPU seconds go

Measured on `d9d1b95` here, each step alone in check-runner's environment after the build
phase, with `gate-time-start` before it and `gate-time-sum` after it in one task process, so
that the gate's own CPU accounting reads it; one pool job:

| step | CPU s | wall s | load before → after |
|---|---|---|---|
| test | 108.3 | 77.4 | 57.9 → 58.2 |
| test-doc | 6.5 | 14.2 | 58.2 → 55.1 |
| test-collect | 210.3 | 128.9 | 55.1 → 55.5 |
| test-faults | 0.2 | 3.3 | 55.5 → 53.7 |
| conform-engine | 2.2 | 10.0 | 53.7 → 51.2 |
| test262-check | 0.1 | 0.7 | 51.2 → 51.2 |
| test262-moves-check | 3.5 | 38.3 | 51.2 → 56.9 |

Each test alone (its test binary run directly, `--exact <name> --test-threads 1`), load 74 to
91: the package `swell` costs 82.9 CPU seconds in the default mode and 213.3 with a collection
at every safe point (759 integration tests 195.8, 176 library tests 17.5). The step
`test-collect` is 56 % of the CPU seconds of the test steps (205.7 of 367.3 and 211.3 of 374.5
in pairs 3 and 2 below), 63 to 64 % of those without `gate-selftest` (205.7 of 327.1, 211.3 of
333.0, and 210.3 of 331.1 in the table above), spread over hundreds of tests
(`story:gate-test-steps-cost-half`).

The planted scripts. Six of them (`test262-refuses-*`, four,
`test262-paths-agree-when-exported-and-passed`, `slow-refuses-a-changed-expected-line`) count a
small checkout through the task `test262`, which runs `cargo run --locked --release -q -p
swell-test262`. No step before the clock builds that release runner, so the first of them
builds it inside `gate-selftest` whenever a source of the runner changed. Here it is hidden: the
compile cache answers the build (its CPU seconds are the cache server's, not the gate's), and a
target directory where test262 ran before holds it already. On the forge's runner the compile
cache does not answer, and what its target directory lacks is built inside the clock. Its log
of the red run does not show that build (the steps print no cargo line); its build phase shows
a target directory that is not empty (`build` finished in 0.01 s, `build-collect` in 10.87 s,
`build-conform-engine` in 61 s), so the release runner there is rebuilt for what changed, the
`swell` crate in every wave. Shown here on a tree that had no fresh release build: the first
`gate-selftest` took 107 s of wall time, the files of `target/release/deps` were written inside
it, the second on the same tree took 75 s. With the runner's `main.rs` touched (pairs 2 and 3
below) `d9d1b95`'s form wrote 2 files into `target/release/deps` inside the clock.

The local gate did not catch the failure for three reasons: its wall time ran on 16 cores of a
shared machine, where the runner has 4; its CPU seconds left out the release build (compile
cache, warm target); and the gate printed the CPU seconds without a bound.

What this change makes the local gate show, and no more: on a machine whose core count has an
entry it bounds the CPU seconds of the test steps; on every machine it refuses a build inside the
clock. It does not count the test steps as the forge's runner does. Where check-runner runs on a
core count without an entry (the pool's 64-core node), the CPU seconds are printed, not bounded:
a green there says the test steps' wall time stayed within 120 s on that machine and nothing was
built inside the clock, and nothing of the forge's time, which the forge judges; a return of the
collect mode to opt-level 0 stays green there, and the forge or a machine with an entry catches
it.

## Change

1. **The collect mode runs at opt-level 1 in the gate.** A cargo profile `collect` (`inherits =
   "test"`, `opt-level = 1`, nothing else) in `.cargo/collect-profile.toml`, given with
   `--config .cargo/collect-profile.toml`. One owner holds that flag: the variable
   `COLLECT_CONFIG` of `taskfiles/collect-mode.yml`, used by the three commands. The name
   `collect` is written where a path or `--profile` needs it: `build-collect` and
   `test-collect` with `--cargo-profile collect`, the collect half of `test-doc` with
   `--profile collect`, `build-collect`'s `<dir>/collect/deps`. The file
   is not `.cargo/config.toml` and not the root manifest, both sources of the digest of
   `conformance/test262/expected.txt`, so no test262 run follows from it. Overflow checks and
   debug assertions stay on (inherited from `test`). `build-collect` checks the file: its
   non-blank lines are exactly the three, or it fails naming the line. It also runs
   `taskfiles/no-debug-sections.sh` on the test executables of `target/collect/deps`, which
   `test-binaries-check` (test profile) does not read. What stays at opt-level 0: the step
   `test` (every test, the stack tests among them, in the default mode), and the lane `slow`'s
   collect half, which stays in the test profile and adds every module of swell's tests that
   calls `stack_with_room` (27 modules call it; the lane ran 13 of them already, as modules
   that read `SWELL_FULL_COUNTS`, and now runs the other 14 too: `cargo nextest list`
   with its filter gives 506 tests before and 732 after, 33 of them named for the promised
   stack or `min_stack_bytes`; the 226 added cost about 54 CPU seconds, each test measured
   alone at load 74 to 91): the stack tests of the collect mode keep their unoptimized frames
   there. `test-collect` took 205.7 to 211.3 CPU seconds at opt-level 0 and 29.1 to 30.9 at
   opt-level 1 (loads 55 to 116), with the same 934 passed and 1 ignored.
2. **The release runner of test262 is built before the clock.** A task `build-test262-runner`
   (`cargo build --locked --release -p swell-test262`, without `RUSTFLAGS`, as the task
   `test262` runs it, so that its `cargo run` finds the build fresh) is the last step of
   `build-tests`. `taskfiles/test262.yml` is not changed.
3. **No build inside the clock.** `gate-time-start` writes `.out/gate-time/start` once;
   `gate-time-sum` fails when a file under `<dir>/<profile>/deps` has a modification time after
   it, for two directories only: `<dir>` is `CARGO_TARGET_DIR` where it is set, `target`
   otherwise, and `conformance/ess/target`. It names the count, the first file and the task
   `build-tests` (`gate-time: the test steps wrote <k> file(s) into <dir>/*/deps or
   conformance/ess/target/*/deps inside the clock, the first <path>; ...`, `<k>` counting the
   files of both directories), prints `gate-time: no file of <n> under <dir>/*/deps changed
   while the test steps ran` otherwise (`<n>` counts the files of both directories too, though
   the line names only `<dir>`), and fails on a tree without `<dir>/*/deps`.
   A compiler that writes while the clock runs is seen whatever the
   compile cache answers. The check cannot tell whose compiler wrote. The pool keeps one target
   directory per tree path, on its build node as on the fitted machine, in cargo's layout
   `<dir>/<profile>/deps`; cargo's lock serializes two builds into it, not one job's build and
   another job's tests. So a second job of the same tree that builds while the clock runs makes
   the check red, on the build node as here, and the gate runs one job per tree at a time. The
   planted scripts of `gate-selftest` that build a small crate of their own build it under
   `.out/gate-tests/<name>/`, outside these two directories: their builds are part of what that
   step measures, on the forge as here. A red line of this check is fixed by building before
   the clock, never by narrowing the directories.
4. **The CPU seconds of each step and one bound per kind of machine.** `gate-time` notes the CPU
   seconds of each step (the task process's waited-for children since the last mark, to the
   hundredth). The bound is a table, `GATE_CPU_BOUNDS`, pairs `<cores>:<CPU seconds>`, keyed by
   the core count that the wall line prints (`gate-time test-steps ... on <n> cores`, from
   `nproc`). `gate-time-sum` looks its own core count up:
   - an entry: `gate-cpu test-steps <cpu> of <bound> on <n> cores: <step> <cpu>, ...`, and over
     the bound `gate-cpu: the test steps took <cpu> CPU seconds, over the bound of <bound> for
     <n> cores; the costliest: <step> <cpu>, <step> <cpu>, <step> <cpu>`; one bound, red over
     it, no band;
   - no entry: `gate-cpu test-steps <cpu>, no bound for <n> cores: <step> <cpu>, ...`; the CPU
     seconds decide nothing, and the exit status is the wall budget's and the `deps` check's;
   - no table (`check` alone): `gate-cpu test-steps <cpu>: <step> <cpu>, ...`.
   An entry that is not `<cores>:<CPU seconds>` fails (`gate-cpu: GATE_CPU_BOUNDS holds <entry>,
   not <cores>:<CPU seconds>`), and so do two entries for one core count (`gate-cpu:
   GATE_CPU_BOUNDS holds <n> cores twice`). A run whose CPU seconds cannot be read fails on a
   core count with an entry (`...; the bound of <bound> for <n> cores needs them`) and passes
   without one, printing `gate-cpu test-steps: not read (...)`.
   `GATE_TIME_CORES` plants the core count; it exists for the planted scripts only. The CPU
   seconds are a lower bound of the work: a compile the cache wrapper runs is counted in its
   server, which is why change 3 checks the target directories. The line `gate-time yardstick
   <s> s: conform-engine <a> and test262-moves-check <b>` is printed and decides nothing; on the
   forge it tells the runner's speed (figures under "The runner").
5. **check-runner gives the table**: `GATE_CPU_BOUNDS` defaults to `CPU_BOUNDS_BY_CORES` of
   `taskfiles/forge.yml`, `16:245` (no entry for 64 cores: see "The entries"), and is on
   `check`'s command line, shown by
   `gate-vars`. The forge's 4 cores have no entry, and `check` alone, as the forge runs it, has
   no table: there the wall time decides, and the line prints the runner's own CPU seconds per
   step.
6. **Planted scripts**, red before their check.
   `gate-time-refuses-test-steps-over-the-cpu-bound-or-building` plants the core count 7, which
   no machine of the table has, so that it reads the same on any machine; a step `first`, and a
   step `burnt` that spends about a tenth of a CPU second in awk when `BURN=1` and writes
   `target/release/deps/planted-inside-the-clock` when `WRITE=1`. Five runs:
   `GATE_CPU_BOUNDS="16:280"`, no entry for 7 cores, passes with the no-bound line and the
   `deps` line green; `"16:280 7:100"` passes with `of 100 on 7 cores`; burnt and written under
   `"7:0.05 16:280"`, refused twice, the burnt step named first, the file and `build-tests`
   named, the wall line still printed; `"16:280 7:fast"`, refused naming the entry;
   `"16:280 16:300"`, refused with `gate-cpu: GATE_CPU_BOUNDS holds 16 cores twice`.
   `gate-time-refuses-test-steps-over-the-budget` (two task processes) holds the line of a run
   without a table, and refuses an entry for the planted core count whose CPU seconds cannot be
   read. Both unset `CARGO_TARGET_DIR`, so that the target directory they plant is the one
   read. `test-collect-refuses-a-crate-without-the-feature` copies the profile file into its
   planted workspace, expects cargo's Finished line of the profile `collect` and the
   debug-section line, and plants a fourth key in the profile: refused before the build. The
   clock tasks move unchanged to `taskfiles/gate-time.yml`, included by the root
   `Taskfile.yml`: a task of an included file runs in the directory of the Taskfile that
   includes it, so a planted Taskfile that included the root one would run the clock on the
   gate's own `.out/gate-time` (it did in an earlier form of this tree, and cost that run its CPU
   line). `gate-selftest` has 66 scripts, one more than `d9d1b95`.

### Why CPU seconds per kind of machine, and not a ratio

The same work counts other CPU seconds on another machine: the test steps of `d9d1b95` took
312.1 to 374.5 CPU seconds on the fitted machine and 254.0 on the pool's build node; the
change's 186.9 to 195.0 and 116.1. Two references were tried to divide that out, and neither
held still:

- the CPU seconds of the steps `conform-engine` + `test262-moves-check` of the same run: on the
  build node they read 2.6 and 4.1 for the same code within five minutes (`conform-engine` 1.3
  and 2.7: 12 tests in 3 binaries and their process starts), and the ratio of `d9d1b95`'s steps
  read 96.6 there against 64.6 to 66.8 on the fitted machine;
- a pinned piece of the test steps' own work, swell's `builtins_uri::` tests on one thread, five
  runs in a row in one job on the build node: once 4.93, 5.36, 4.56, 6.65, 4.64 CPU seconds
  (largest / smallest 1.46); twice in a row 10.55, 13.14, 12.77, 10.14, 8.29 (1.59). CPU seconds
  equalled wall seconds in every run, so the spread is the speed of the node's core from one
  minute to the next.

A ratio to work measured on one core cannot be steadier than that core. The sum of the test
steps' CPU seconds is spread over many processes and cores, and on one kind of machine it held:
the change 186.9 to 195.0 over five runs at loads 66 to 128 on the fitted machine. So the bound
is CPU seconds, one entry per kind of machine, each fitted on that kind of machine.

The key is the core count the gate prints, `nproc`'s (or `GATE_TIME_CORES` in the planted
scripts): a stand-in for "the machine the pair was measured on", nothing more. What it cannot
tell apart:
- two machines with the same core count and a different speed: the entry is wrong for one of
  them until a pair is recorded on it;
- a change of the pool's core allotment (a job that gets 32 cores instead of 64) has no entry,
  and its CPU seconds decide nothing until a pair is recorded for it.
An entry is added or changed only together with a pair measured on that kind of machine (the
code before the change over it, the change under it, each a whole `check-runner` job), recorded
in the same commit. On the forge the wall time decides: its 4 cores have no entry, because they
come in two speeds (241.3 against 393.9 CPU seconds for nearly the same tests, table under "The
runner"), and one entry for 4 cores would be wrong for one of them.

### The entries

Every run with the same test steps (times are when the job started), the base form (`d9d1b95`'s
steps under the new clock, see
acceptance 2) and the change, each a whole `check-runner` job or the same steps in one job:

| kind of machine | form | run | load at the end | CPU s |
|---|---|---|---|---|
| the fitted machine, 16 cores | base | check-runner on `a350de3` | 34 | 312.1 |
| the fitted machine, 16 cores | base | pair 3 | 74 | 367.3 |
| the fitted machine, 16 cores | base | pair 2 | 77 | 374.5 |
| the fitted machine, 16 cores | change | steps one by one, one job | 128 | 186.9 |
| the fitted machine, 16 cores | change | pair 1 | 69 | 192.0 |
| the fitted machine, 16 cores | change | check-runner | 90 | 192.4 |
| the fitted machine, 16 cores | change | pair 2 | 99 | 192.8 |
| the fitted machine, 16 cores | change | pair 3 | 66 | 195.0 |
| the build node, 64 cores | base | check-runner 04:33 | 28.4 | 254.0 |
| the build node, 64 cores | base | check-runner 05:01 | 39.5 | 301.7 |
| the build node, 64 cores | change | check-runner 04:38 | 5.7 | 116.1 |
| the build node, 64 cores | change | check-runner 05:02 | 61.6 | 200.6 |
| the build node, 64 cores | change | check-runner 05:03 | 51.8 | 176.9 |

The rule for an entry: the geometric mean of the highest change and the lowest base, rounded to
5, provided the highest change × 1.25 is at most the entry and the entry × 1.25 is at most the
lowest base.

- 16 cores: 245. √(195.0 × 312.1) = 246.7, rounded to 5: 245. The checks: the highest change
  195.0 × 1.25 = 243.8 ≤ 245; 245 × 1.25 = 306.3 ≤ the lowest base 312.1. The lowest base is
  27 % over it (312.1 / 245 = 1.274), the highest change 26 % under it (245 / 195.0 = 1.256).
  The entry is provisional: it is derived from runs of earlier forms of this tree on the fitted
  machine (the base 312.1 from a check-runner of `a350de3`, the others from pairs 2 and 3 and
  the change's runs, with the same test steps and the same CPU sum as this form); no pair under
  this form exists on that machine yet, and the first two check-runner jobs on a 16-core machine
  after the merge record it. An estimate, not a measurement: the code that read 312.1 CPU
  seconds here took 448.5 CPU seconds in 127.9 s of wall on the slowest runner seen, so 245 here
  would come to about 245 × 448.5 / 312.1 = 352 CPU seconds there, 100 to 108 s of wall at the
  runner's 0.285 to 0.306 s of wall per CPU second ("The runner"). This is an upper estimate:
  the runner's 448.5 likely held the release runner's build inside `gate-selftest`, which the
  312.1 here did not (see "Where the CPU seconds go").
- 64 cores: no entry. The rule fails: the highest change, 200.6, × 1.25 = 250.8, is above
  the lowest base, 254.0, / 1.25 = 203.2 (√(200.6 × 254.0) = 225.7 would leave 12 % and 13 %).
  On the build node the CPU seconds of the same code move with the node's load: the change read
  116.1 at load 5.7, 176.9 at 51.8 and 200.6 at 61.6 on 64 cores (other jobs share the node's
  cores; a core whose sibling thread is busy counts more CPU seconds for the same work). The
  fitted machine ran at loads 66 to 128 on 16 cores in every change run, oversubscribed each
  time, which may be why its figures held within 4.3 % (195.0 / 186.9);
  below load 26 it was not measured.
  Without an entry the build node prints `no bound for 64 cores`, and its exit status is the
  wall budget's and the `deps` check's.

Before and after, per step, in one pool job (pair 3; `d9d1b95`'s form at load 91 → 74, the
change at 74 → 66), CPU s / wall s:

| step | `d9d1b95`'s form | the change |
|---|---|---|
| test | 109.0 / 94.4 | 111.5 / 111.5 |
| test-doc | 6.7 / 22.1 | 6.7 / 34.1 |
| test-collect | 205.7 / 86.6 | 30.1 / 32.4 |
| test-faults | 0.2 / 6.5 | 0.2 / 3.7 |
| conform-engine | 2.1 / 12.7 | 2.1 / 8.4 |
| test262-check | 0.1 / 3.6 | 0.1 / 1.9 |
| test262-moves-check | 3.4 / 23.9 | 3.5 / 18.3 |
| gate-selftest | 40.2 / 45.6 | 40.8 / 84.3 |
| sum | 367.3 / 295.4 | 195.0 / 294.6 |

The wall times here are the shared machine's at its load, not the runner's. `build-collect`
in the profile `collect`: 8 min 0 s the first time (load about 100, no compile cache entry for
the new profile), 0.09 to 0.15 s after; in the test profile it took 53.7 s the first time (load
58) and 0.07 to 1.97 s warm. The forge's build phase pays the first build of the profile once
per cold target directory.

When the bound changes: each forge run of `check` prints its wall line, its `gate-cpu` line
without a bound and the yardstick.
- A forge sum over 120 s is a budget defect, fixed before the next feature. If that code was
  green under an entry of the table, that entry is too loose: it is set again with a new pair
  on its kind of machine, in one change that records the forge run and the pair.
- A forge sum between 100 and 120 s passes: the budget is 120 s; 100 s is the aim of this
  story, not a check.
- A yardstick over the slowest seen (2.9 s) is a runner slower than any seen: the wall budget
  still decides, and the figure under "The runner" moves to it.
A forge run whose `gate-selftest` holds a release build shows it as the `deps` line of
change 3.

The lane `slow` does not run `gate-time-sum`: its steps (the full test262 run, the counts at
full size, the stack tests at opt-level 0) are outside the gate and its budget of 120 s, and
their own time is the 10-minute limit of a pool job.

Not covered: the lane's filter keys on the string `stack_with_room`; a future module that sizes
its own stack with `stack_size(`, `min_stack_bytes` or `on_bounded_stack` and does not read
`SWELL_FULL_COUNTS` is not run in the lane at opt-level 0. The forge's CPU seconds of the
change are known only from its first run after the merge. The build node's 64 cores get no
entry: the rule cannot be met there (the change 116.1 to 200.6 CPU seconds, the base 254.0 and
301.7, the change's CPU seconds rising with the node's load), so on such a machine only the
wall budget and the `deps` check decide. Where check-runner runs on a core count without an
entry (the pool's 64-core node), the CPU seconds are printed, not bounded: a green there says
the test steps' wall time stayed within 120 s on that machine and nothing was built inside the
clock, and nothing of the forge's time, which the forge judges; a return of the collect mode to
opt-level 0 stays green there, and the forge or a machine with an entry catches it. No check
holds the value of an entry, its pair or the rule of 1.25: a commit can change
`CPU_BOUNDS_BY_CORES` without a pair and the gate stays green. No planted case holds the path of
the table from `taskfiles/forge.yml` through `check`'s command line to `gate-time-sum`: the
planted scripts give `GATE_CPU_BOUNDS` to the clock tasks directly.

Not shown: an entry for the build node's 64 cores (the rule fails there, table above); that
the entry of 16 cores holds on the fitted machine below load 26, or with the change's steps at
a load where they count fewer CPU seconds than 186.9; a check-runner job of this form on the
16-core machine (the pool sent every job of this form to the build node); a pair for
the entry of 16 cores made as acceptance 7 names it (its figures come from runs of earlier
forms of this tree, with the same test steps and the same CPU sum as this form; the base figure
312.1 from a check-runner of `a350de3` before this story's clock mechanics); why the build
node's CPU seconds rise with its load (inferred, not measured: the sibling threads of its
cores).

Two things move, each allowed for a reason of its own:
- The release build of the test262 runner moves from inside `gate-selftest` to `build-tests`,
  before the clock. A build is setup, not a test step, and the `deps` check of change 3 makes
  any build inside the clock red, so the move cannot hide one.
- The collect mode's stack modules (those that call `stack_with_room`) run at opt-level 1 in
  the gate and at opt-level 0 in the lane `slow`. The gate's proof for them changes frame size:
  the gate shows that the promised stack holds with the frames of opt-level 1, not of 0; the
  lane `slow` shows it with the unoptimized frames, and the step `test` still does in the
  default mode at opt-level 0.
The 120 s budget, the list of timed steps and the number of jobs are unchanged. No test,
assertion, planted defect or count is removed; no timeout, part count or engine source changes.

Writes: Taskfile.yml, taskfiles/gate-time.yml (new), taskfiles/collect-mode.yml,
taskfiles/forge.yml, .cargo/collect-profile.toml (new),
tests/gate/gate-time-refuses-test-steps-over-the-budget.sh,
tests/gate/gate-time-refuses-test-steps-over-the-cpu-bound-or-building.sh (new),
tests/gate/test-collect-refuses-a-crate-without-the-feature.sh

## Acceptance

1. The failing tests first:
   `sh tests/gate/gate-time-refuses-test-steps-over-the-cpu-bound-or-building.sh` and
   `sh tests/gate/gate-time-refuses-test-steps-over-the-budget.sh`. On `d9d1b95` both print
   `<name>: FAIL no taskfiles/gate-time.yml`. On the tree of the ratio form they print
   `gate-time-refuses-test-steps-over-the-cpu-bound-or-building: FAIL no line "gate-cpu
   test-steps <cpu>, no bound for 7 cores: first <cpu>, burnt <cpu>" (...)` and
   `gate-time-refuses-test-steps-over-the-budget: FAIL gate-time-sum without a bound printed no
   line "gate-cpu test-steps <cpu>: paused <cpu>, ..." (...)`. On this form without the refusal
   of two entries for one core count, the first prints
   `gate-time-refuses-test-steps-over-the-cpu-bound-or-building: FAIL the timed steps exited 0
   with two entries for 16 cores in GATE_CPU_BOUNDS (...)`. On the change both print `: ok`.
2. The base form is the change's tree with `d9d1b95`'s `taskfiles/collect-mode.yml` and
   `tests/gate/test-collect-refuses-a-crate-without-the-feature.sh`, `test-doc`'s collect line
   without the collect profile, and `build-tests` without `- task: build-test262-runner`: the
   steps of `d9d1b95` under the new clock, after `touch crates/test262/src/main.rs` (content
   unchanged, as any wave that changes a source of the runner). `slot --timeout 30m task
   check-runner` on it prints `gate-cpu: the test steps took <c> CPU seconds, over the bound of
   245 for 16 cores; ...` on the fitted machine (figures: 312.1, 367.3, 374.5) and
   `gate-cpu test-steps <c>, no bound for 64 cores: ...` on the build node (254.0, 301.7), and
   `gate-time: the test steps wrote <k> file(s) into <dir>/*/deps or
   conformance/ess/target/*/deps inside the clock, the first <dir>/release/deps/...` where the
   release runner was rebuilt (2 files on the fitted machine, 23 and 2 on the build node).
3. `slot --timeout 30m task check-runner` on the change prints `build-test262-runner: Finished
   ...`, `build-collect: <n> test executables in <dir>/collect/deps, none with a .debug_
   section`, `gate-time test-steps <s> of 120, cpu <c>, load <l> on <n> cores`, `gate-cpu
   test-steps <c> of 245 on 16 cores: ...` on 16 cores or `gate-cpu test-steps <c>, no bound for
   <n> cores: ...` on any other count, and `gate-time: no file of <k> under <dir>/*/deps changed
   while the test steps ran`, `<dir>` being `CARGO_TARGET_DIR` or `target` as the code prints
   it; the counts are those of `d9d1b95` (1215, 13 and 13, 934, 1, 12) and 66 planted scripts
   (65 + 1). It exits 0, or 201 with `gate-time test-steps` as its only red line and a load
   above its core count ("A red time line under load goes to the forge"); a red `gate-cpu` line
   or `deps` line holds the commit. Measured on the build node (started 05:03) on the tree
   before the entry moved from 280 to 245, which changes nothing on a core count without an
   entry (that job ran with `GATE_CPU_BOUNDS=16:280`, its log line 3): exit 0, `gate-time
   test-steps 31.6 of 120, cpu 176.9, load 51.75 on 64 cores`, `gate-cpu test-steps 176.9, no
   bound for 64 cores: ...`, `gate-time: no file of 331 under target/*/deps changed while the
   test steps ran`, the counts above. Where check-runner runs on a core count without an entry
   (the pool's 64-core node), the CPU seconds are printed, not bounded: a green there says the
   test steps' wall time stayed within 120 s on that machine and nothing was built inside the
   clock, and nothing of the forge's time, which the forge judges; a return of the collect mode
   to opt-level 0 stays green there, and the forge or a machine with an entry catches it.
4. `gate-selftest` grows by at most 1 CPU second: the two clock scripts of the change together
   take at most 1.0 CPU second more than `d9d1b95`'s one, three runs each with plain `sh` on one
   machine, off the pool, from the root of the tree, each script measured as
   `sh -c "sh '<script>' > /dev/null 2>&1; echo \$?; times"` (the CPU seconds are the user and
   system time of the children, the third line of `times`), `d9d1b95`'s script read from a copy
   of that commit's `tests/gate/`. Measured on the fitted machine at load 13 to 14: `d9d1b95`'s
   1.10, 1.46, 1.15; the change's 0.32 + 0.97, 0.36 + 1.13, 0.28 + 0.89 (+0.19, +0.03, +0.02).
5. `sh taskfiles/test-main.sh check`, `cargo clippy --workspace --all-targets --locked -- -D
   warnings` and `cargo fmt --all --check` exit 0 (no source of a crate changes).
6. After the merge, the forge's `check` on 4 cores prints `build-collect: Finished \`collect\`
   profile ...` and `build-collect: <n> test executables in target/collect/deps, none with a
   .debug_ section` in its build phase, `gate-time: no file of <n> under target/*/deps changed
   while the test steps ran`, and `gate-cpu test-steps <c>: ...` without a bound.
   - The forge's sum of the test steps is at most 120 s (aim: 100 s).
   - The lane `slow` passes, with its collect half in the test profile and the stack modules
     included.
7. An entry of `CPU_BOUNDS_BY_CORES` is added or changed only in a commit that records, for its
   kind of machine, the base form of acceptance 2 over it and the change under it, one whole
   `slot --timeout 30m task check-runner` job each, and meets the rule of "The entries", except
   `16:245`, whose pair under this tree is recorded by the first two check-runner jobs on a
   16-core machine after the merge.
   - 16 cores, the fitted machine: 245, provisional, from runs of earlier forms of this tree
     (base 312.1 to 374.5, change 186.9 to 195.0).
   - 64 cores, the build node: no entry; the five figures fail the rule.
