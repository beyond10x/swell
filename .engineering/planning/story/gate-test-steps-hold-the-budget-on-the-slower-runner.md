---
format: aep.planning-md/3
id: story:gate-test-steps-hold-the-budget-on-the-slower-runner
kind: story
status: active
title: The gate's test steps hold 120 s on the forge's slower runner
summary: 'The forge ran the gate''s test steps on the same code in 77.2 s and in 147.6 s of their 120: one runner took 1.94 times the CPU seconds. The steps end within 120 s and within 29.5 yardsticks on that runner with the budget unchanged, the log of check names the runner''s CPU. Lane unit 6 of wave big-2 took the tests of swell::main to opt-level 1 (117.6 s, 30.2 yardsticks on the forge); lane unit 7 takes more of the step test there.'
relations:
- informed_by: story:realm-size-lane-within-its-share
- informed_by: story:slow-lane-holds-its-bound-on-the-slower-runner
revision: 10
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
lane `slow` and no task of the lane changes (item 5 says what the lane then runs): it has
no room (`story:slow-lane-holds-its-bound-on-the-slower-runner`).

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

5. The seventh unit of the lane. The forge read the sixth unit's head at 117.6 s of 120 and
   30.2 yardsticks ("As delivered"); the bound of the acceptance is 29.5. More of the step
   `test` runs at opt-level 1, inside option A of
   `approval-record:wave-big-2-lane-unit-6-cut` (the step `test` at opt-level 1, the stack
   tests at opt-level 0 behind the guard, debug assertions and overflow checks on). The
   lists of item 2 give what the step still runs at opt-level 0, in CPU seconds on the
   build node: the library tests of swell 24.1, the tests of the test262 runner 19.6, of
   the parser 17.2, the tests of `swell::main` outside the run at opt-level 1 24.6 (of
   them the 17 tests of `round_trip_planes!` 8.0), of swell-num 8.9, of the other crates
   11.8.
   - What moves into the run at opt-level 1:
     - every test of the library of swell (the binary `swell`). No test under
       `crates/swell/src` sets a thread's stack size, and the step `test-collect` runs
       them at opt-level 1 already;
     - every test of the package `swell-num` (the binaries `swell-num::main` and
       `swell-num`): the crate has no dependency, and no test of it reads a stack, a clock
       it judges by, or a child process;
     - every test of the package `swell-test262` (the binaries `swell-test262::main` and
       `swell-test262`; its executable holds no test) but for the two tests of the list
       below;
     - the 17 tests that the macro `round_trip_planes!` of
       `crates/swell/tests/builtins_uri.rs` writes. They call no stack helper. The unit
       writes the macro's calls in the form `language_parts!` of
       `crates/swell/tests/language.rs` has (a `#[test]` line and a `fn <name>` line for
       each test), so the rule prints them "other"; the tests' names and bodies do not
       change, and `taskfiles/realm-room-tests.awk` does not change.
   - What stays at opt-level 0, by name: a new variable of `Taskfile.yml`,
     `TEST_OPT0_TESTS`, one plain line of pairs `<binary> <test>`, names tests of a moved
     binary that stay in the run at opt-level 0. `test-opt1-vars-check` holds its value
     as it holds the other three, and `GATE_VARIABLES` gets it on a line of its own. The
     reason of each pair stands in the summary of the task `test`, not in a comment. It
     starts with two tests of `swell-test262::main`:
     `run::eval_script_cases_declare_throw_answer_and_fail_on_a_refusal_and_the_nesting_limit`
     and `run::runs_file_names_the_refusal_and_the_thrown_error`. They are a sample, one
     for each fixture tree that holds a case at the parser's nesting limit (`eval-script`
     and `refused`): with them the gate still runs the runner's worker thread at that
     limit with unoptimized frames. The other tests that read those trees move; what they
     hold is what the runner prints. The bound of the stack itself is held at opt-level 0
     by the parser's tests and by the stack tests of `swell::main`, which do not move. A
     moved test that fails at opt-level 1 goes on the list with its output in the report;
     the unit changes no test and no engine code to make one pass. A pair that names a
     test the list without a filter does not hold fails the step, naming the pair.
   - What stays at opt-level 0 as it is: the tests of `swell::main` the rule prints "stack"
     or does not print (the 33 tests of the macros `reaches_the_reentry_limit!` and
     `chain_test!` call `stack_with_room`), and every test of the packages of the parser,
     the compiler, the regexp and unicode crates and the command line.
   - The run at opt-level 1 stays one `cargo nextest run` in the profile `collect` without
     the feature, built by one `cargo nextest run --no-run` in `build-tests` before
     `build-collect`; `TEST_OPT1_ARGS` names the packages, and what holds its value
     (`test-opt1-vars-check`, the summaries, the planted scripts) follows the new value.
     The guard `SWELL_NO_STACK_TESTS=1` is set for the whole run, as now. The executable
     `swell-test262` that the runner's tests start is the one cargo builds for the run
     they are in (`CARGO_BIN_EXE_swell-test262`).
   - The two filters. `test-split` writes the `default-filter` of each run from the rule's
     list, the moved binaries and `TEST_OPT0_TESTS`, and its check of the written line
     (today: as many `test(=` as the rule prints "other") follows the new form. The filter
     of the run at opt-level 0 is the complement of the other, as now.
   - Every test runs once: the check of `test-split` over the three lists holds for every
     binary. The step prints the line `test: swell::main <a> at opt-level 1, <b> at
     opt-level 0` as now, and after it one new line `test: <n> tests at opt-level 1, <m>
     at opt-level 0` over the workspace: n is the count of `.out/test-list-opt1.txt`, m
     that of `.out/test-list-opt0.txt`, and n plus m is the count of the list without a
     filter. `TEST_OPT1_PROFILE_TEST` stays in the run at opt-level 1.
   - The summed line keeps its form and its meaning: p and f are sums over both runs; i is
     the count of the list without a filter less p and f; b is the count of test binaries
     of the workspace, each counted once, 19 on the base. Today `test-sum` reads b and i
     from the run at opt-level 0, whose filter then leaves whole binaries out; the unit
     says in its report where it reads them from. The ten `gate-slowest test` lines are
     taken over both runs, as now.
   - The lane `slow` reads the same rule, so two of its parts run the 17 tests in another
     place, with no task of the lane changed. In `slow-tests` the collect mode runs them
     in its part at opt-level 1, where it ran them at opt-level 0 (the module is one of
     the lane's: it holds a stack test). `test-realm-sizes` runs what the room rule does
     not print "other", and ran the 17 only because it could not read them; it no longer
     runs them at each size of the realm. They read no heap and set no budget, which is
     that rule's reason for leaving a test out.
   - Not changed: `taskfiles/realm-room-tests.awk`, `taskfiles/gate-time.yml`,
     `taskfiles/collect-mode.yml`, `taskfiles/stack-profiles.yml`, `taskfiles/test262.yml`,
     `taskfiles/nextest.yml`, `taskfiles/forge.yml`, `.cargo/`, `.config/`, `Cargo.toml`,
     every `crates/*/Cargo.toml`, every file under `crates/*/src`, `.github/workflows/`,
     `conformance/`, and in `Taskfile.yml` the tasks of the lane `slow`. The unit changes
     `crates/swell/tests/builtins_uri.rs`, in `Taskfile.yml` the tasks `test`,
     `test-split`, `test-sum`, `build-tests`, `test-opt1-vars-check`, the variables of the
     run at opt-level 1 and `GATE_VARIABLES`, and under `tests/gate/` the scripts that
     hold the old forms (`test-runs-each-test-once-at-one-opt-level.sh`,
     `test-keeps-the-profile-test-in-its-run-at-opt-level-1.sh`,
     `test-refuses-a-caller-value-of-its-run-at-opt-level-1.sh` and any other that reads
     `--test main` or the filter line) and new ones. `AGENTS.md`, `CHANGELOG.md` and
     `README.md` are the coordinator's: the unit writes their text into its notes.

## Acceptance

Checked by the coordinator on the pull request that carries a unit of this story, not a
condition of a unit's report. Read on the wave's second pull request for the sixth unit:
the first line did not hold ("As delivered"). Read again on the pull request that carries
the seventh unit:

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

Checked on the report of the seventh unit (item 5):

- Base and head: the base is `9a8ad008`, the commit of `wave/big-2` the unit's branch
  starts at; the head is the unit's last commit. A run on the base is a run on a detached
  checkout of it in the unit's worktree, with a clean tree. The digest line is set for the
  gate runs and put back as written above for the sixth unit; base and head have the same
  line (the digest does not cover `crates/*/tests/`, `Taskfile.yml` or `tests/gate/`).
- The moved tests: the lines of `.out/test-list-opt1.txt` on the head that
  `.out/test-list-opt1.txt` on the base does not hold. The report gives their count for
  each binary (`swell`, `swell-num`, `swell-num::main`, `swell-test262`,
  `swell-test262::main`, and 17 of `swell::main`).
- The cut, read in one pool job on the head (`slot --timeout 30m --fetch <a directory
  under .out/> sh <the script>`; the script is not committed). The job builds both
  profiles first. Then it runs the moved tests twice with the repository's
  `.config/nextest.toml` (not the step's two files) and one `-E` that names them: once
  with `TEST_NEXTEST_ARGS`, once with `TEST_OPT1_ARGS` and the environment of the run at
  opt-level 1. Each test is started through a runner script
  (`CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER`) that writes the CPU seconds, user plus
  system, of the test's own process and its children. Neither run compiles a crate, and
  each runs as many tests as there are moved tests. The sum over all moved tests at
  opt-level 1 is at most 0.40 of the sum at opt-level 0; a sum over 0.40 stops the unit
  with the two lists. The report gives both sums, the sums for each binary (not bounded)
  and the load at the start and the end of the job.
- Nothing left the gate: the line `<b> test binaries: <p> passed, <f> failed, <i>
  ignored` of `test` is the same on base and head (19, 2532, 0, 1), and so are those of
  `test-collect` and `test-doc`. The line `test: swell::main <a> at opt-level 1, <b> at
  opt-level 0` reads a 17 higher and b 17 lower than on the base (1647 and 123). In the
  new line `test: <n> tests at opt-level 1, <m> at opt-level 0`, n plus m is p plus f plus
  i of the summed line (2533), and n is a plus the counts of the other moved binaries
  given above.
- The list holds, for each test t of `TEST_OPT0_TESTS`: `cargo nextest list
  --message-format oneline` with the arguments, the configuration file and the
  environment of the run at opt-level 1 and `-E 'test(=<t>)'` prints no line for it, and
  with those of the run at opt-level 0 it prints one. The report quotes the lines, and
  for a test it added to the two of item 5 the test's output at opt-level 1.
- The split holds: `slot task gate-selftest` runs planted scripts that show the step red,
  with the test named, when a test of a moved binary other than `swell::main` is in
  neither run and when one is in both; red, with the pair named, when `TEST_OPT0_TESTS`
  names a test the list without a filter does not hold; and red before any test runs when
  a caller gives `TEST_OPT0_TESTS` or `TEST_OPT1_ARGS` a value. They plant the filter, the
  list or the variable; nothing is built. Every script under `tests/gate/` that passed on
  the base passes on the head; the report lists the scripts it changed and the new ones,
  each with its seconds run alone.
- The diff: `git diff --name-only <base>..<head>` lists `Taskfile.yml`,
  `crates/swell/tests/builtins_uri.rs` and files under `tests/gate/`, and nothing else.
  The lines of `cargo nextest list --message-format oneline` that hold
  `every_code_point_of_plane_` are 17 and the same on base and head (`diff` prints
  nothing).
- The lane, without a run: `awk -f taskfiles/realm-room-tests.awk` over the files
  `test-realm-sizes` hands it prints the same lines on base and head but for 17 more
  "other" lines, the 17 tests; with `-v rule=stack` over the files `slow-tests` hands it,
  the same. The report quotes the two counts of each.
- The lane, one run: `slot --timeout 30m task slow-tests` on base and on head. Its lines
  differ only in the two lines of the collect mode: 17 tests less in the part at
  opt-level 0 and 17 more in the part at opt-level 1, with the sum of passed over the two
  the same and no test failed. The report quotes the lines of both runs.
- The build before the clock: `CARGO_TARGET_DIR=target/<name> slot --timeout 30m task
  build-tests` once on base and once on head, each with a name of its own whose directory
  does not exist before the job. The report gives the seconds the pool prints for each
  job, the line `build-tests opt-level 1: Finished ... in <t>` of each, the load, the
  line `build-collect: <n> test executables in <target>/collect/deps, none with a .debug_
  section` of each, and the crates that the build of the run at opt-level 1 compiles on
  the head and not on the base (the `Compiling` lines of `.out/build-tests-opt1.log`).
- `slot --timeout 30m task check-runner` exits 0 on the head with the digest line set as
  above. The report quotes its lines `gate-runner`, both `test:` lines, the summed line,
  `gate-time test`, `gate-time test-steps`, `gate-cpu test-steps` and `build-tests
  opt-level 1`, and the same lines of one such run on the base taken in the job before or
  after it. No bound is set on these two runs: the node gives two levels of CPU seconds
  for one code ("As delivered").

## Out of scope

- The forge's choice of runner, a larger or a paid runner, more jobs for `check`: the
  operator's.
- The tests of the parser, the compiler, the regexp and unicode crates and the command
  line at opt-level 1: their tests of a stack bound (`crates/syntax/tests/sources.rs`,
  `crates/compiler/tests/compile.rs`) read unoptimized frames and have no rule and no
  guard. The 17 parts of `every_nesting_kind_fits_the_stack_bound` are most of the
  parser's seconds.
- The 33 tests of `swell::main` that the macros `reaches_the_reentry_limit!` and
  `chain_test!` write: they call `stack_with_room` and stay at opt-level 0, not printed
  by the rule.
- The step `test-collect` and the collect step's modules: unchanged.
- The cuts of the second body (merged cases, shared realms, sweeps by a stride, scripts
  that read one product): not taken; their limit is in the context above.
- A shorter build of the binary at opt-level 1: the unit reports its seconds and does not
  cut them.
- The lane `slow`: `story:slow-lane-holds-its-bound-on-the-slower-runner`.
- The interpreter's speed: `story:property-escape-cases-spend-less-in-the-interpreter-loop`.

## As delivered

The unit `lanesix` of wave big-2 delivered the story on `impl/lanesix` (two corrections after
two adversary passes), merged into `wave/big-2`. Where the delivery differs from the text
above, and what was measured:

- Item 1 (`92f3d55`): the line `gate-runner: ...` in the log of `check`, as written.
- Item 3 (`4d04f9dd`, `1bdddc9a`, `1910825c`): the step `test` in two runs. On the head
  `test: swell::main 1647 at opt-level 1, 123 at opt-level 0`; `19 test binaries: 2532
  passed, 0 failed, 1 ignored` (2531 on the base: the new test). The new test is
  `realm_padding::the_profile_keeps_debug_assertions_and_overflow_checks_on`; with
  `--config 'profile.collect.debug-assertions=false'` and with `--config
  'profile.collect.overflow-checks=false'` it fails with its own text for each.
- Differences from the story's text:
  - The count of test binaries in the summed line is read from the run at opt-level 0,
    whose line counts every binary of the workspace (19), not from the lists, which name
    only the 14 binaries that hold tests.
  - The step takes `cargo nextest list` three times inside the clock.
  - The task `test-split` sets `RUSTFLAGS` itself (a task's `env` does not reach the tasks
    it calls), and a list that compiles fails the step.
  - The lane's `slow-tests` prints the same binaries, passed and failed on base and head;
    each of its `ignored` counts reads one more on the head, because the new test is in
    `swell::main` and outside the lane's filter.
  - The profile's two checks with the `--config` keys were run before the commit of the
    test and again by the adversary's first pass on `808f926b`.
- The CPU seconds of the test steps, whole gates on a build node of 32 cores, base
  `f93ab563` against head `1910825c`:
  - pair 1: 441.5 (load 30.8) against 232.4 (load 10.1): the loads differ by more than 5,
    the pair does not count;
  - pair 1 taken again: 416.9 (load 36.7) against 315.9 (load 38.5), ratio 0.758; the
    head's gate exited non-zero on wall time alone (the test steps 136.7 s of 120 at a load
    above the core count, the untouched step `test-collect` 52.1 s);
  - pair 2: 314.1 (load 16.3) against 245.6 (load 19.5), ratio 0.782.
  Two pairs count and are at or under 0.80. The step `test` alone went from 231.9 to 141.7
  and from 191.6 to 118.7.
- After the corrections the coordinator took one more pair on `362b69b1`, each side two
  gates in a row: the second gates (nothing built) read 476.5 against 348.7 (loads 15.4
  and 14.1), ratio 0.732; the first gates read 294.0 against 338.4 (loads 13.5 and 10.0),
  ratio 1.151.
- That last pair is not a reading of the unit: in it the step `test-collect`, which the
  unit does not change, read 64.8 CPU seconds on the base and 121.1 on the head. Across
  the gates of this unit that step read 64.8 to 127.9 on one code and one node: the node
  gives two levels of CPU seconds for the same work, and the load does not say which.
  Read against `test-collect` of the same gate, the step `test` costs 2.2 to 2.9 times
  that step on the base (four gates) and 1.25 to 1.74 times on the head (seven gates).
  At one level the pairs read 0.73 to 0.78; a base at the lower level against the one
  head read there (294.0 against 245.6) reads 0.84. The cut is about a quarter of the
  test steps; the node cannot show a margin under the bound of 0.80.
- Item 4: no entry for 32 cores in `CPU_BOUNDS_BY_CORES`. The highest head is 315.9 and the
  lowest base 314.1; an entry needs the head at 0.64 of the base. The local gate still
  cannot see the forge's wall time.
- The build before the clock: `build-tests` with a target directory of its own took 473.0 s
  on the base and 883.9 s on the head (loads 8 to 15 and 20 to 32; 1,078,024 and 1,134,912
  kB); the binary at opt-level 1 alone 5 min 34 s to 8 min 44 s after a change of swell's
  tests. `build-collect` counts one test executable more.
- The adversary's first pass (`review-result:adversary-lanesix-pass-1`) found that a
  caller's value of `TEST_OPT1_ARGS` or `NO_STACK_TESTS` was accepted, that the summed line
  failed when `task` was called by a relative path, and that nothing kept the profile's
  test in the run at opt-level 1. Correction 1 (`69868891`, `4ad70476`, `c63040c1`) added
  the internal tasks `test-opt1-vars-check` and `test-opt1-profile-check`, the variable
  `TEST_OPT1_PROFILE_TEST` and four planted scripts.
- The second pass (`review-result:adversary-lanesix-pass-2`) found that the profile check
  did not read an included cargo configuration and that `test-split` failed without a line
  of its own with no support file. Correction 2 (`ef47efb7`, `362b69b1`) refuses the
  include, narrows the check to the profiles `dev`, `test` and `collect`, gives the step
  its line, and adds a sixth planted script. `gate-selftest` reads 112 passed.
- Limits that stay, stated in the tasks' summaries: the profile check does not read a cargo
  configuration file outside the tree, a key written with an escape or an inline table
  over several lines; the summed line of a run that a signal ends undercounts while the
  step is red; the tasks described as internal can be called by name.
- Not the unit's and left as they were: the loop form that ends a command silently in two
  tasks of the lane `slow`, and the path of the executable in `check-runner`.
- The acceptance on the forge (the test steps of the first attempt of `check` against its
  yardstick and the budget, the duration of the job, the lane's `slow-time slow-tests`) is
  read on the wave's second pull request.

- Read on the wave's second pull request, on its head `5ab9f8e5` (2026-11-27). The first
  attempt of `check` on the forge (run 46069632845) is green and prints `gate-runner: 4
  cores, AMD EPYC 7763 64-Core Processor, 16366632 kB, sh is dash`, `gate-time test-steps
  117.6 of 120, cpu 407.6, load 4.65 on 4 cores` and `gate-time yardstick 3.9 s:
  conform-engine 1.9 and test262-moves-check 2.0`. The budget of 120 s holds with 2.4 s. The
  bound of 29.5 yardsticks does not: it is 115.1 s on that runner, and the run read 117.6 s
  (30.2 yardsticks). So the first line of the acceptance is not met, and the story stays
  active. A runner of the same yardstick (3.8 s) read 147.6 s before the unit (`fd71252b`,
  run 45979562872: `test` 80.6 s, `test-collect` 33.1 s, `gate-selftest` 24.6 s); with the
  unit `test` reads 46.6 s with 87 more tests, `test-collect` 33.8 s and `gate-selftest`
  28.0 s. The job `check` took 16 min 29 s of its 30 minutes (6 min 49 s in run
  45972624878). The guard holds: the lane's `slow-time slow-tests` read 145.3 s (run
  46069633020). What is left of the story is a further cut, built as a seventh unit of the
  lane before any further feature unit.
