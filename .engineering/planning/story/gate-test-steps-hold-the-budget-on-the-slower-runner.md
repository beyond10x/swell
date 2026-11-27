---
format: aep.planning-md/3
id: story:gate-test-steps-hold-the-budget-on-the-slower-runner
kind: story
status: active
title: The gate's test steps hold 120 s on the forge's slower runner
summary: 'The forge ran the gate''s test steps on the same code in 77.2 s and in 147.6 s of their 120: one runner took 1.94 times the CPU seconds. The steps end within 120 s on that runner (about 62 s on the faster one) with the budget unchanged, the log of check names the runner''s CPU, and the local gate bounds the CPU seconds on the build node. Lane unit 6 of wave big-2.'
revision: 5
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
`story:realm-size-lane-within-its-share`, with the rule and the guard this story takes over.

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

2. Done. The lists of the step's tests and scripts by CPU seconds, on the base.

3. The step `test` runs the tests of swell's integration-test binary (`swell::main`) at
   opt-level 1, except those that run on a bounded stack.
   - The split is the lane's: `taskfiles/realm-room-tests.awk -v rule=stack` over every
     module of `crates/swell/tests/` prints each test "stack" or "other". The tests printed
     "other" run at opt-level 1. Every other test of `swell::main` (the stack tests and the
     tests a macro writes, which the rule does not see), the library tests of swell and the
     tests of every other crate of the workspace run at opt-level 0, as now.
   - The guard is the lane's: the run at opt-level 1 sets `SWELL_NO_STACK_TESTS=1`
     (`NO_STACK_TESTS`), so `stack_with_room` and `stack_thread` of
     `crates/swell/tests/support/host.rs` panic there. A stack test in a form the rule does
     not read fails the gate instead of passing on frames of another size. The runs at
     opt-level 0 do not set it.
   - The profile at opt-level 1 inherits `test` and changes the opt-level only. Debug
     assertions and overflow checks are on in it. It stands in a file under `.cargo/` that
     the command names with `--config`, as `.cargo/collect-profile.toml` does, not in
     `Cargo.toml` or `.cargo/config.toml` (sources of the digest of the test262 counts). The
     unit may use the profile `collect` or add a second profile; the report says which and
     why.
   - One test of `swell::main` that the rule prints "other" holds the two checks: it fails
     when `debug_assertions` is off, and it fails when an `i32` addition that overflows at
     run time does not panic. It runs in the run at opt-level 1 (and so holds the profile
     of the gate) and names what it holds.
   - Every test runs once. The step counts the tests of `swell::main` in both runs and
     fails when a test that nextest lists for the binary is in neither or in both, naming
     it. The line `<b> test binaries: <p> passed, <f> failed, <i> ignored` sums the runs,
     and the ten `gate-slowest test` lines are taken over all of them.
   - The binary at opt-level 1 is built before the clock by `build-tests`. The step fails
     when its log has a cargo Compiling line (`compile-check`), and the `deps` check of
     `gate-time-sum` covers the profile's directory.
   - `task test262-check`, `conformance/test262/expected.digest` and the lane `slow` do not
     change. `.github/workflows/check.yml` does not change.
   - In `Taskfile.yml` the unit changes the tasks `test`, `build-tests`, `check` and
     `slow-runner`, the variables at its top and the list `GATE_VARIABLES` (a variable it
     adds stands there), and nothing else; under `taskfiles/` what the step needs.
     `AGENTS.md`, `CHANGELOG.md` and `README.md` are the coordinator's: the unit writes their
     text into its notes.

4. The local gate and the CPU seconds. `check-runner` gives `check` the table
   `GATE_CPU_BOUNDS` (`CPU_BOUNDS_BY_CORES`, `taskfiles/forge.yml`), which has no entry for
   the build node. The rule of an entry is `AGENTS.md`'s: the geometric mean of the highest
   reading of the change and the lowest of the base, rounded to 5, the highest change × 1.25
   at most the entry and the entry × 1.25 at most the lowest base. The unit takes the
   readings of the acceptance below and adds the entry for 32 cores when both margins hold,
   with the readings recorded in the same commit. When a margin fails it adds no entry, and
   the report gives the readings and the loads. The local gate then still cannot see the
   forge's wall time; the report says so in one line, and what is left is the
   coordinator's to file.

## Acceptance

Checked by the coordinator on the wave's second pull request, not a condition of the
unit's report:

- The log of the first attempt of `check` on the forge holds one line `gate-runner: <n>
  cores, <model name>, <kB> kB, sh is <sh>` before the first `gate-time <step>` line, and
  its `gate-time test-steps <s> of 120` reads s at or under 29.5 times the seconds of its
  `gate-time yardstick` line (62 / 2.1), and at or under 120. The pull request names the
  run with the three lines.
- The job `check` on the forge ends inside its limit of 30 minutes
  (`.github/workflows/check.yml`); the pull request names its duration beside that of run
  45972624878.
- The lane's `slow-time slow-tests` on the forge reads no more than 175 s (168.8 and 167.6
  in the runs on `fd71252b` and `8d6cfebc`).

Checked on the unit's report:

- Base and head: the base is the merge of `wave/big-2` at `3d154437` into the unit's
  branch (the wave with the interpreter-loop unit and the unit of the private names in
  eval code, and item 1); the head is the unit's last commit. The report names both.
- The CPU seconds: `slot --timeout 30m task check-runner` runs base, head, base, head, in
  that order, one job at a time. For these runs, and for no commit, the line
  `test262-sources` of `conformance/test262/expected.digest` may be set in the tree to the
  digest the check prints, and is put back after each (`git status --short` is empty before
  the next commit). The reading is the first number of the line `gate-cpu test-steps <c>`;
  the load is that of the line `gate-time test-steps`. A pair (the k-th base and the k-th
  head) counts when its two loads differ by at most 5. Two pairs that count have head /
  base at most 0.80; a pair that does not count is taken again, at most two more times,
  and then the report says what was read. The report quotes the `gate-cpu test-steps` and
  `gate-time test-steps` lines of every run, and the `gate-time` line of the step `test`.
- The profile: the report quotes the profile's lines. With `debug-assertions = false`
  planted in it the test of the two checks is red, and with `overflow-checks = false`
  planted it is red; each red line is quoted, and the file is put back.
- Nothing left the gate: the `passed` and `ignored` counts of `test`, `test-collect` and
  `test-doc` on base and head; `test` differs by the one new test and by nothing else.
  `ignored` stays 1 in `test` and 1 in `test-collect`. The report gives the count of tests
  of `swell::main` at each opt-level; their sum is the count nextest lists for the binary.
- The split holds: a new planted script under `tests/gate/` shows the step red, each with
  the test named, when a test of `swell::main` is in neither run and when one is in both.
  A stack test moved into the run at opt-level 1 (the rule's line changed in a copy)
  panics with the guard's text; the report quotes it.
- The stack tests: `slot --timeout 30m task test-stack-profiles` passes on the head, 7 in
  each profile, and no stack bound of a test changes (`git diff <base>..<head> --
  crates/` changes one test file, for the new test, and no file under `crates/*/src`).
- The build before the clock: the wall seconds of `build-tests` on base and head, from one
  cold job each (a new target directory), and the size of `target/` after each. Test
  binaries stay one per crate and profile (`task test-binaries-check` passes).
- The bounds: `git diff <base>..<head>` changes no line of `taskfiles/gate-time.yml` that
  holds `default "120"`, `default "900"` or `default "600"`, and
  `tests/gate/gate-time-refuses-test-steps-over-the-budget.sh` and
  `tests/gate/gate-time-refuses-test-steps-over-the-cpu-bound-or-building.sh` are
  byte-identical.
- Each new or changed test and script with its seconds from the `gate-slowest` lines; one
  over 5 s is named. No test over 60 s.
- Item 4: `CPU_BOUNDS_BY_CORES` holds an entry for 32 cores with its readings in the
  commit, or the report says which margin failed, with the numbers.
- The lane: `slot --timeout 30m task slow-tests` once on base and once on head prints the
  same counts (`slow-tests: <b> test binaries: <p> passed, ...`).
- `slot --timeout 30m task check-runner` exits 0 on the head with the digest line set as
  above, and `task gate-selftest` passes with the new scripts.

## Out of scope

- The forge's choice of runner, a larger or a paid runner, more jobs for `check`: the
  operator's.
- The tests of the other crates and the library tests of swell at opt-level 1: the syntax
  crate's tests of the stack bound read unoptimized frames and have no rule and no guard.
- The step `test-collect` and the collect step's modules: unchanged.
- The cuts of the second body (merged cases, shared realms, sweeps by a stride, scripts
  that read one product): not taken; their limit is in the context above.
- The lane `slow`: `story:slow-lane-holds-its-bound-on-the-slower-runner`.
- The interpreter's speed: `story:property-escape-cases-spend-less-in-the-interpreter-loop`.
