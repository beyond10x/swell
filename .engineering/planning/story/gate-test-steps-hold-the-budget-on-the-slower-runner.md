---
format: aep.planning-md/3
id: story:gate-test-steps-hold-the-budget-on-the-slower-runner
kind: story
status: active
title: The gate's test steps hold 120 s on the forge's slower runner
summary: 'The forge ran the gate''s test steps on the same code in 77.2 s and in 147.6 s of their 120: one runner took 1.94 times the CPU seconds. The steps end within 120 s on that runner (about 62 s on the faster one) with the budget unchanged, the log of check names the runner''s CPU, and the local gate bounds the CPU seconds on the build node. Lane unit 6 of wave big-2.'
relations:
- informed_by: story:realm-size-lane-within-its-share
- informed_by: story:slow-lane-holds-its-bound-on-the-slower-runner
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-27T10:38:23Z", actor: "human:operator", revision: 3, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-27T10:38:24Z", actor: "human:operator", revision: 4, executor: "agent:coordinator"}
---
## Context

Found at the first pull request of wave big-2. The gate's test steps are bounded at 120 s of
wall time (`GATE_TIME_BUDGET`, task `gate-time-sum` of `taskfiles/gate-time.yml`); on the
forge, where `check` runs without a table of CPU bounds, the wall time on 4 cores is the
measure. The forge ran `check` twice on 2026-11-27 on code that differs in files under
`.engineering/` only (`8d6cfebc`, then `fd71252b`):

| run | head | test steps | CPU seconds | yardstick | load at the end |
|---|---|---|---|---|---|
| 45972624878 | `8d6cfebc` | 77.2 of 120 | 274.3 | 2.1 s | 4.01 on 4 cores |
| 45979562872, attempt 1 | `fd71252b` | 147.6 of 120 | 531.6 | 3.8 s | 4.38 on 4 cores |

The second run failed with `gate-time: the test steps ran 147.6 s, over the budget of 120 s;
the slowest: test 80.6, test-collect 33.1, gate-selftest 24.6`. Every test passed in both.
The same work took 1.94 times the CPU seconds and 1.91 times the wall time; the yardstick
line (`gate-time yardstick`, the wall time of two steps of fixed work) read 1.81 times as
much. The log of `check` does not name the runner's CPU, so the runs do not say whether the
second runner is another model or a loaded one. The lane `slow` saw a runner 1.294 times
slower on the same day (`story:slow-lane-holds-its-bound-on-the-slower-runner`).

A run that reads 120 s on the second runner reads about 62 s on the first (120 / 1.94); the
steps read 77.2 s there. The cut the budget needs is to 80 % (62 / 77.2). Where the seconds
are in run 45972624878, wall and CPU per step:

| step | wall | CPU | what runs |
|---|---|---|---|
| `test` | 39.3 | 153.1 | 2,445 tests in 19 binaries |
| `test-collect` | 17.1 | 67.2 | 1,944 tests in 2 binaries, a collection at every safe point |
| `gate-selftest` | 15.1 | 47.5 | 105 planted scripts |
| `test-doc` | 3.3 | 3.7 | 19 doc tests, twice |
| the four other steps | 2.4 | 2.7 | `test-faults`, `conform-engine`, the two test262 checks |

No single test is long: the slowest of `test` reads 1.44 s, and its ten slowest sum to 10.7
s; the two slowest of `test-collect` read 1.55 s and 1.28 s. The seconds are in the count.

The local gate did not see it: on the build node the same steps read 39.4 s and 49.4 s of
120 (CPU 399.5 and 462.2, at loads 10 and 25), and `gate-cpu` printed `no bound for 32
cores`. `AGENTS.md` ("check-runner passes GATE_CPU_BOUNDS") calls that node one of 64 cores.

The unit's first part (2026-11-27, branch `impl/lanesix`, `92f3d55`) named the runner and
measured the steps on the build node, each test and each script by the CPU seconds of its
own process, at a load of 28 to 38 on 32 cores:

| step | CPU seconds | what the list shows |
|---|---|---|
| `test` | 300.1 | 282.9 in 2,522 test processes: `swell::main` 201.3, the library tests of swell 24.1, the other crates 57.4 |
| `test-collect` | 124.9 | 111.3 in 1,987 test processes |
| `gate-selftest` | 67.2 | 106 scripts; 40.5 at a load of 14 |

A cut to 80 % takes about 100 CPU seconds out. The median test costs 0.026 s and the 40
costliest sum to about 70 s. The cuts the story's second body allowed (cases merged into one
test function, a realm shared by merged cases, sweeps by a stride, planted scripts that read
one product) sum to about 93 s with every one of them at its limit, and to well under half
of that inside the story's other rules. The unit stopped there, as its brief said, and made
no cut.

The coordinator put three options to the operator: the step `test` at opt-level 1 with the
stack tests kept at opt-level 0 behind a guard; the step `test-collect` out of the gate; no
cut. The operator's decision of 2026-11-27, 12:19 CET
(`approval-record:wave-big-2-lane-unit-6-cut`): the first, on the condition that debug
assertions and overflow checks stay on in that profile. The gate's `test-collect` runs at
opt-level 1 since `story:gate-test-steps-fit-four-cores` (168.5 to 33.3 CPU seconds), and
the lane `slow` splits its collect mode the same way since
`story:realm-size-lane-within-its-share`, with the rule and the guard this story reads.

A probe on the unit's base (`f93ab563`, the merge of `wave/big-2` at `3d154437`; one pool
job at a load of 3 to 8 on 32 cores; nothing committed) ran every test of `swell::main` at
both opt-levels, the second in the profile `collect` without the collect feature:

| tests of `swell::main` | test processes | CPU s at opt-level 0 | at opt-level 1 |
|---|---|---|---|
| all (1,769 tests passed in each run) | 1,771 | 119.1 | 20.4 |
| printed "other" by the rule `stack` | 1,646 | 94.5 | 16.7 |
| printed "stack" | 73 | 15.7 | 2.4 |
| not printed (a macro writes them) | 52 | 8.9 | 1.3 |

The build of that binary at opt-level 1 took 305.7 s of wall time there and compiled one
crate, swell with its test binary; the other crates of the profile were built for the
collect step. The forge's job `check` took 6 min 49 s in run 45972624878.

## Change

The gate's test steps end within 120 s on a forge runner that takes 1.94 times the CPU
seconds of run 45972624878 for the same work, with the budget as it is. `GATE_TIME_BUDGET`
is not raised, no step leaves the clock, and no test leaves the gate. No test moves into the
lane `slow` and the lane does not change: it has no room
(`story:slow-lane-holds-its-bound-on-the-slower-runner`).

1. Built (`92f3d55`). The runner is named: `check` prints, after `gate-time-start` and
   before the first test step, one line `gate-runner: <n> cores, <model name>, <MemTotal> kB,
   sh is <sh>`, from the task `slow-runner` with the label `gate-runner`; the `echo` exists
   once; `tests/gate/check-prints-the-runner-line-before-its-first-test-step.sh` holds it.

2. Done. The lists of the step's tests and scripts by CPU seconds, on the first base.

3. The step `test` runs the tests of swell's integration-test binary (`swell::main`) that
   the lane's rule prints "other" at opt-level 1, and every other test as now.
   - The rule: `taskfiles/realm-room-tests.awk -v rule=stack`, handed
     `crates/swell/tests/support/*.rs` and every `crates/swell/tests/*.rs` but `main.rs`,
     as the task `slow-tests` hands it. It prints one line for each `#[test]` function
     whose `fn` line names it, "stack" or "other" and the test as `<module>::<test>`; a
     line may end in a third field, which the step does not read. A test it does not print
     (a macro writes it) and a name it prints that the binary does not hold (a test behind
     a `cfg`) are not in the run at opt-level 1.
   - The run at opt-level 1: the tests of `swell::main` that the rule prints "other" and
     that `cargo nextest list` names for the binary. It is the default mode, without the
     feature `collect-every-safe-point`, in the profile `collect` of
     `.cargo/collect-profile.toml`, which is not changed and gets no second profile
     (`build-collect` refuses any other line in that file): `-p swell --test main --config
     .cargo/collect-profile.toml --cargo-profile collect`, in a variable of its own.
     `COLLECT_MAIN` carries the feature and is not used for it.
   - The run at opt-level 0: every other test of the workspace, with `TEST_NEXTEST_ARGS`
     unchanged (`test-vars-check` holds its value) and the filter appended.
   - The guard is the lane's: the run at opt-level 1 sets `SWELL_NO_STACK_TESTS=1`
     (`NO_STACK_TESTS`), so `stack_with_room` and `stack_thread` of
     `crates/swell/tests/support/host.rs` panic there, and a stack test in a form the rule
     does not read fails the gate. The run at opt-level 0 does not set it. The line they
     panic with (`NO_STACK_TESTS_LINE`) names the lane's task; the unit rewords it to name
     both runs, with the test that holds the line.
   - Debug assertions and overflow checks are on in the profile `collect`: it inherits
     `test`, and `Cargo.toml` sets `overflow-checks = true` there. One new test holds both.
     It fails when `debug_assertions` is off, and it fails when an `i32` addition that
     overflows at run time (its operands through `std::hint::black_box`) does not panic. It
     stands in an existing module of `crates/swell/tests/` whose file names none of
     `SWELL_FULL_COUNTS`, `stack_with_room`, `stack_thread` and `stack_size(`, so the lane's
     counts do not move and the generated `crates/swell/tests/main.rs` does not change; the
     rule prints it "other", and its name says what it holds.
   - Every test runs once. From `cargo nextest list` the step takes the tests of each run
     and the tests of the workspace without a filter, and fails, naming the test, when one
     is in not exactly one run. It prints `test: swell::main <a> at opt-level 1, <b> at
     opt-level 0`.
   - The step prints one line `<b> test binaries: <p> passed, <f> failed, <i> ignored` for
     both runs, as now: b counts `swell::main` once; p and f are sums; i is the count of
     tests neither run ran, which is 1 on the base (the test marked `#[ignore]`), not the
     sum of what each run's filter skipped. The ten `gate-slowest test` lines are taken
     over both runs. How it is made is the unit's (the JUnit file of the first run is
     copied before the second writes it); the tasks `nextest-report` and `compile-check`
     of `taskfiles/nextest.yml` do not change, and `compile-check` reads both logs.
   - The binary at opt-level 1 is built before the clock, in `build-tests`, before its step
     `build-collect`, whose check of `<target>/collect/deps` for `.debug_` sections then
     covers it.
   - Not changed: `taskfiles/realm-room-tests.awk`, `taskfiles/gate-time.yml` (its `deps`
     check already reads every `target/*/deps`), `taskfiles/collect-mode.yml`,
     `taskfiles/stack-profiles.yml`, `taskfiles/test262.yml`, `.cargo/`, `Cargo.toml`,
     `.github/workflows/`, `conformance/`, and in `Taskfile.yml` the tasks `slow-tests`,
     `slow-collect-start`, `build-realm-sizes`, `test-realm-sizes` and the four variables at
     its top. In `Taskfile.yml` the unit changes the tasks `test` and `build-tests`, adds
     internal tasks and variables, and adds each new variable to `GATE_VARIABLES` on a line
     of its own. It may correct the sentence of `taskfiles/realm-places.yml` that says its
     binary shares one build with the step `test`. `AGENTS.md`, `CHANGELOG.md` and
     `README.md` are the coordinator's: the unit writes their text into its notes.

4. The local gate and the CPU seconds. `check-runner` gives `check` the table
   `GATE_CPU_BOUNDS` (`CPU_BOUNDS_BY_CORES`, `taskfiles/forge.yml`), which has no entry for
   the build node. The rule of an entry is `AGENTS.md`'s: the geometric mean of the highest
   reading of the change and the lowest of the base, rounded to 5, the highest change × 1.25
   at most the entry and the entry × 1.25 at most the lowest base; both hold only when the
   highest head is at most 0.64 of the lowest base. The unit takes the readings of the
   acceptance below and adds the entry for 32 cores to `taskfiles/forge.yml` when both
   margins hold, with the readings recorded in the same commit. When a margin fails it adds
   no entry, and the report gives the readings and the loads. The local gate then still
   cannot see the forge's wall time; the report says so in one line, and what is left is
   the coordinator's to file.

## Acceptance

Checked by the coordinator on the wave's second pull request, not a condition of the
unit's report:

- The log of the first attempt of `check` on the forge holds one line `gate-runner: <n>
  cores, <model name>, <kB> kB, sh is <sh>` before the first `gate-time <step>` line, and
  its `gate-time test-steps <s> of 120` reads s at or under 29.5 times the seconds of its
  `gate-time yardstick` line (62 / 2.1), and at or under 120. The pull request names the
  run with the three lines.
- The job `check` on the forge ends inside its limit of 30 minutes
  (`.github/workflows/check.yml`), which is the bound of the build before the clock; the
  pull request names the job's duration beside the 6 min 49 s of run 45972624878.
- A guard, true before the unit: the lane's `slow-time slow-tests` on the forge reads no
  more than 175 s (168.8 and 167.6 in the runs on `fd71252b` and `8d6cfebc`).

Checked on the unit's report:

- Base and head: the base is `f93ab563`, the merge of `wave/big-2` at `3d154437` into the
  unit's branch; the head is the unit's last commit. A run on the base is a run on a
  detached checkout of it in the unit's worktree, with a clean tree.
- The digest line: for the gate runs below, and for no commit, the line `test262-sources`
  of `conformance/test262/expected.digest` is set in the tree to the line `task
  test262-digest` prints, read once before the first run
  (base and head have the same: no file of the digest changes), and the file is put back
  after the last (`git status --short` is empty before the next commit).
- The CPU seconds: `slot --timeout 30m task check-runner` runs base, head, base, head, in
  that order, one job at a time. The reading is the first number of the line `gate-cpu
  test-steps <c>`; the load is that of the line `gate-time test-steps`. A pair (the k-th
  base and the k-th head) counts when its two loads differ by at most 5. Two pairs that
  count have head / base at most 0.80; a pair that does not count is taken again, at most
  two more times. Without two such pairs the cut is not accepted: the unit stops and the
  report gives every reading. The report quotes the `gate-cpu test-steps` and `gate-time
  test-steps` lines of every run and the `gate-time` line of the step `test`.
- The profile's two checks: `cargo nextest run` with the arguments of the run at
  opt-level 1, `-E 'test(=<the new test>)'` and `--config
  'profile.collect.debug-assertions=false'` exits non-zero with the test's own text for
  debug assertions; the same with `--config 'profile.collect.overflow-checks=false'` exits
  non-zero with its text for the overflow; without either flag it passes. The report
  quotes the three results, and the line of `cargo nextest list` that shows the test in the
  run at opt-level 1. No file is changed for these.
- Nothing left the gate: the lines `<b> test binaries: <p> passed, <f> failed, <i>
  ignored` of `test` on base and head differ in p by 1 (the new test) and in nothing else;
  i is 1 in both. Those of `test-collect` differ in p by 1 and of `test-doc` in nothing.
  The line `test: swell::main <a> at opt-level 1, <b> at opt-level 0` is quoted; a plus b
  is the count `cargo nextest list` gives for the binary.
- The split holds: a new planted script under `tests/gate/` shows the step red, each with
  the test named, when a test of `swell::main` is in neither run and when one is in both
  (it plants the filter or the list; nothing is built). A stack test put into the run at
  opt-level 1 (`cargo nextest run` with that run's arguments and environment, `-E
  'test(=<a test the rule prints "stack">)'`) panics with the guard's line; the report
  quotes it.
- The diff: `git diff <base>..<head> -- crates/` changes the file of the new test,
  `crates/swell/tests/support/host.rs` and the file of the test that holds the guard's
  line, and no file under `crates/*/src`. A guard, true before the unit: `slot --timeout
  30m task test-stack-profiles` passes on the head, 7 in each profile.
- The build before the clock: `CARGO_TARGET_DIR=target/<a new name> slot --timeout 30m
  task build-tests` once on base and once on head, with the seconds the pool prints for
  the job and `du -sk` of the directory in the same job. The host's compile cache answers
  both; the report says so. The line `build-collect: <n> test executables in
  <target>/collect/deps, none with a .debug_ section` reads one more on the head.
- The bounds: `git diff <base>..<head>` changes no line of `taskfiles/gate-time.yml`, and
  `tests/gate/gate-time-refuses-test-steps-over-the-budget.sh`,
  `tests/gate/gate-time-refuses-test-steps-over-the-cpu-bound-or-building.sh`,
  `tests/gate/nextest-report-refuses-a-step-that-compiles.sh` and
  `tests/gate/nextest-report-reads-a-coloured-log.sh` are byte-identical.
- Each new script with its seconds run alone; the new test with its `gate-slowest` line,
  or the report says it is not among the ten.
- Item 4: `CPU_BOUNDS_BY_CORES` holds an entry for 32 cores with its readings in the
  commit, or the report says which margin failed, with the numbers.
- The lane: `slot --timeout 30m task slow-tests` once on base and once on head prints the
  same counts (`slow-tests: <b> test binaries: <p> passed, ...`).
- `slot --timeout 30m task check-runner` exits 0 on the head with the digest line set as
  above, and `task gate-selftest` passes with the new scripts.

## Out of scope

- The forge's choice of runner, a larger or a paid runner, more jobs for `check`: the
  operator's.
- The tests of the other crates and the library tests of swell at opt-level 1: their tests
  of a stack bound (`crates/syntax/tests/sources.rs`, `crates/compiler/tests/compile.rs`,
  the regexp and unicode crates, the test262 runner, the command line) read unoptimized
  frames and have no rule and no guard.
- The tests a macro writes in `swell::main` (52, 8.9 CPU seconds at opt-level 0): the rule
  does not print them; they stay at opt-level 0.
- The step `test-collect` and the collect step's modules: unchanged.
- The cuts of the second body (merged cases, shared realms, sweeps by a stride, scripts
  that read one product): not taken; their limit is in the context above.
- A shorter build of the binary at opt-level 1: the unit reports its seconds and does not
  cut them.
- The lane `slow`: `story:slow-lane-holds-its-bound-on-the-slower-runner`.
- The interpreter's speed: `story:property-escape-cases-spend-less-in-the-interpreter-loop`.
