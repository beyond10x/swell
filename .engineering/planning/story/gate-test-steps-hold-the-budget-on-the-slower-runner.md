---
format: aep.planning-md/3
id: story:gate-test-steps-hold-the-budget-on-the-slower-runner
kind: story
status: active
title: The gate's test steps hold 120 s on the forge's slower runner
summary: 'The forge ran the gate''s test steps on the same code in 77.2 s and in 147.6 s of their 120: one runner took 1.94 times the CPU seconds. The steps end within 120 s on that runner (about 62 s on the faster one) with the budget unchanged, the log of check names the runner''s CPU, and the local gate bounds the CPU seconds on the build node. Lane unit 6 of wave big-2.'
revision: 4
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

## Change

The gate's test steps end within 120 s on a forge runner that takes 1.94 times the CPU
seconds of run 45972624878 for the same work, with the budget as it is. `GATE_TIME_BUDGET`
is not raised, no step leaves the clock, and no test leaves the gate without a place that
still runs it. No test moves into the lane `slow` and no test of the lane changes: that
lane has no room (`story:slow-lane-holds-its-bound-on-the-slower-runner`).

Order: item 1 is the unit's first commit. The lists of item 2 are taken on the unit's base
with that commit. Item 4 is the last and may add nothing.

1. The runner is named. `check` prints, after `gate-time-start` and before the first test
   step, one line `gate-runner: <n> cores, <model name>, <MemTotal> kB, sh is <sh>`. It
   comes from the task `slow-runner` of `Taskfile.yml`, which gets a label (default
   `slow-runner`, so the lane's line and
   `tests/gate/slow-collect-builds-beside-the-lane-and-ends-with-it.sh` stay as they are);
   the `echo` that prints the line exists once. Its `desc` and summary say that `check`
   calls it too. A new planted script under `tests/gate/` holds the line of `check`; its
   name says so. In `Taskfile.yml` the unit changes the tasks `check` and `slow-runner` and
   nothing else: an include line for `taskfiles/perf.yml` is the coordinator's, at the join
   with the interpreter-loop unit.

2. The unit measures before it cuts. One pool job on the build node lists every test of the
   steps `test` and `test-collect` with its CPU seconds (user plus system of the test's own
   process: the runner starts one process per test; the report names the command that reads
   them, and wall time is not used), and every script of `gate-selftest` with its CPU
   seconds. The lists are summed per test module and per script and kept in the unit's work
   directory.

3. The cuts go where the lists put the seconds. What a cut may be:
   - a product several tests build each for itself (a realm with the same setup, a compiled
     source, a table) is made once and shared, in a form that holds under a runner that
     starts one process per test (the organization's rule that an expensive product is made
     once, never per test): a file made by a setup step, or one test function that runs the
     cases in one process and names each case in its failure text;
   - a sweep (a budget moved one step or 8 bytes at a time across a script) covers the same
     range in fewer runs, by a stride or by stepping over budgets that stop at the same
     charge, and reads no `SWELL_FULL_COUNTS`: each sweep the unit changes names its planted
     defect and shows it red before and after, at the budget where it fails. A sweep that
     would give up a part of its range is not changed;
   - a script of `gate-selftest` that runs `cargo` or repeats a build reads a product made
     once before the clock.
   What a cut may not be: a test deleted or marked ignored, a test or module taken out of
   the collect step, an assertion weakened, a plant that no test holds any more, a step
   moved out of the clock, a count that reads `SWELL_FULL_COUNTS` (it would run full in the
   lane), more shards or jobs on the forge. `.github/workflows/check.yml` does not change.

   Files the cuts leave alone:
   - a cut stays inside a test module; no module is merged, renamed or removed, so the
     generated `crates/swell/tests/main.rs` does not change;
   - a test named in `conformance/host/requirements.txt` and a test of `STACK_TESTS`
     (`taskfiles/stack-profiles.yml`) keeps its path and its name;
   - the files of the units that run beside this one: `crates/swell/tests/eval.rs`,
     `eval_private_names.rs`, `dynamic_function.rs`, `budgets.rs`, `private_methods.rs`,
     `private_fields.rs`, `private_element_hook.rs`, `interpreter_loop.rs`,
     `crates/swell/src/dynamic_code.rs`, `crates/swell/src/interp/`, and the compiler and
     syntax crates;
   - a cut in a test module under `crates/*/src` changes the digest of the test262 sources:
     `task test262-check` is then red on the unit's branch, the coordinator writes the
     digest at the wave's close, and the report names each such file.

4. The local gate and the CPU seconds. `check-runner` gives `check` the table
   `GATE_CPU_BOUNDS` (`CPU_BOUNDS_BY_CORES`, `taskfiles/forge.yml`), which has no entry for
   the build node. The rule of an entry is `AGENTS.md`'s: the geometric mean of the highest
   reading of the change and the lowest of the base, rounded to 5, the highest change ×
   1.25 at most the entry and the entry × 1.25 at most the lowest base. The unit takes the
   readings of the acceptance below and adds the entry for the core count that `nproc`
   prints in a pool job when both margins hold, with the readings recorded in the same
   commit. A cut to 80 % does not reach the margins (they need 64 %), and the same code
   read 399.5 and 462.2 CPU seconds there at two loads: when a margin fails the unit adds
   no entry, and the report gives the readings, the loads and the core count. The local
   gate then still cannot see the forge's wall time; the report says so in one line, and
   what is left is the coordinator's to file.

## Acceptance

Checked by the coordinator on the wave's second pull request, not a condition of the
unit's report:

- The log of the first attempt of `check` on the forge holds one line `gate-runner: <n>
  cores, <model name>, <kB> kB, sh is <sh>` before the first `gate-time <step>` line, and
  its `gate-time test-steps <s> of 120` reads s at or under 29.5 times the seconds of its
  `gate-time yardstick` line (62 / 2.1), and at or under 120. The pull request names the
  run with the three lines.
- The lane's `slow-time slow-tests` on the forge reads no more than 175 s (168.8 and 167.6
  in the runs on `fd71252b` and `8d6cfebc`).

Checked on the unit's report:

- Base and head: the base is the commit of `wave/big-2` the unit's branch is cut from, with
  item 1 on it; the head is the unit's last commit before it merges a later state of the
  wave. Neither holds the interpreter-loop unit or the unit of the private names in eval
  code; the report names both commits.
- The CPU seconds: `slot --timeout 30m task check-runner` runs base, head, base, head, in
  that order, one job at a time. The reading is the first number of the line `gate-cpu
  test-steps <c>`; the load is that of the line `gate-time test-steps`. A pair (the k-th
  base and the k-th head) counts when its two loads differ by at most 5. Two pairs that
  count have head / base at most 0.80; a pair that does not count is taken again, at most
  two more times, and then the report says what was read. The report quotes the `gate-cpu
  test-steps` and `gate-time test-steps` lines of every run.
- The lists of item 2: the 40 costliest tests and the 15 costliest scripts with their CPU
  seconds and their share of the step, on base and head, and the command that made them.
- The runner line: `task gate-selftest` passes with the new script; `git grep -n 'cores,
  \$cpu, \$mem'` has one hit; the line of the lane `slow` and its script are unchanged.
- Nothing left the gate: the `passed` and `ignored` counts of `test`, `test-collect` and
  `test-doc` on base and head (`<b> test binaries: <p> passed, <f> failed, <i> ignored`);
  every difference is named test by test, with the test that runs its cases now and the
  names of those cases. `ignored` stays 1 in `test` and 1 in `test-collect`.
  `Taskfile.yml`'s `check` lists the same steps between `gate-time-start` and
  `gate-time-sum`, plus the runner line.
- The bounds: `git diff <base>..<head>` changes no line of `taskfiles/gate-time.yml` that
  holds `default "120"`, `default "900"` or `default "600"`, and
  `tests/gate/gate-time-refuses-test-steps-over-the-budget.sh` and
  `tests/gate/gate-time-refuses-test-steps-over-the-cpu-bound-or-building.sh` are
  byte-identical.
- Each changed sweep: its plant red before and after, with the budget.
- Each product made once: the line that makes it, the tests or scripts that read it, and
  one plant that shows a reader fails when the product is wrong.
- Each new or changed test and script with its seconds from the `gate-slowest` lines; one
  over 5 s is named. No test over 60 s.
- Item 4: `CPU_BOUNDS_BY_CORES` holds an entry for the node's core count with its readings
  in the commit, or the report says which margin failed, with the numbers. The report gives
  the `nproc` of a pool job; the coordinator corrects the core count in `AGENTS.md`.
- The lane: `slot --timeout 30m task slow-tests` once on base and once on head prints the
  same counts (`slow-tests: <b> test binaries: <p> passed, ...`), but for the differences
  named above.
- `slot --timeout 30m task check-runner` exits 0 on the head and on the head merged with
  the wave (but for `test262-check` where a file under `crates/*/src` changed).

## Out of scope

- The forge's choice of runner, a larger or a paid runner, more jobs for `check`: the
  operator's.
- A test or module that runs in one of the two steps `test` and `test-collect` only: it
  narrows what the collect step covers and is the operator's decision. No module leaves
  the collect step in this unit.
- The lane `slow`: `story:slow-lane-holds-its-bound-on-the-slower-runner`.
- The interpreter's speed: `story:property-escape-cases-spend-less-in-the-interpreter-loop`.
  Its gain and the tests the open units add are not counted in the 80 %.
- The build before the clock (`build-tests`): it is not in the 120 s.
