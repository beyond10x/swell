---
format: aep.planning-md/3
id: story:gate-test-steps-hold-the-budget-on-the-slower-runner
kind: story
status: draft
title: The gate's test steps hold 120 s on the forge's slower runner
summary: 'The forge ran the gate''s test steps on the same code in 77.2 s and in 147.6 s of their 120: one runner took 1.94 times the CPU seconds. The steps end within 120 s on that runner (about 62 s on the faster one) with the budget unchanged, the log of check names the runner''s CPU, and the local gate bounds the CPU seconds on the build node. Lane unit 6 of wave big-2.'
revision: 1
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
The same work took 1.94 times the CPU seconds and 1.91 times the wall time. The log of
`check` does not name the runner's CPU, so the runs do not say whether the second runner is
another model or a loaded one. The lane `slow` saw a runner 1.294 times slower on the same
day (`story:slow-lane-holds-its-bound-on-the-slower-runner`).

A run that reads 120 s on the second runner reads about 62 s on the first; the steps read
77.2 s there. Where they are in run 45972624878, wall seconds and CPU seconds per step:

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
120 on 32 cores (CPU 399.5 and 462.2, at loads 10 and 25), and `gate-cpu` printed `no bound
for 32 cores`.

## Change

The gate's test steps end within 120 s on a forge runner that takes 1.94 times the CPU
seconds of run 45972624878 for the same work, with the budget as it is. `GATE_TIME_BUDGET`
is not raised, no step leaves the clock, and no test leaves the gate without a place that
still runs it. No test moves into the lane `slow`: that lane has no room
(`story:slow-lane-holds-its-bound-on-the-slower-runner`).

1. The runner is named. `check` prints one line with the runner's cores, CPU model, memory
   and `sh`, as the lane `slow` does (`slow-runner: ...`, the task `slow-runner`), from
   the same code and not a copy of it. The line is printed on every machine, before the
   first test step, and a planted script of `gate-selftest` holds it.

2. The unit measures before it cuts. One pool job on the build node lists every test of the
   steps `test` and `test-collect` with its CPU seconds (user plus system of the test's
   process, from the runner's report or `/proc`, not wall time), and every script of
   `gate-selftest` with its CPU seconds; the lists are sorted and summed per test module and
   per script. The report gives the 40 costliest tests and the 15 costliest scripts with
   their share of the step.

3. The cuts go where the lists put the seconds. What a cut may be:
   - a product several tests build each for itself (a realm with the same setup, a compiled
     source, a table) is built once and shared, in a form that holds under a runner that
     starts one process per test (`AGENTS.md`, "Tests stay fast"): a file made by a setup
     step, or one test function that runs the cases in one process;
   - a sweep (a budget moved one step or 8 bytes at a time across a script) covers the same
     range with fewer runs where its plants stay red: each sweep the unit thins names its
     planted defect and shows it red before and after, at the budget where it fails;
   - a test that runs in both the default and the collect step and reads nothing a
     collection changes runs in one of them; the unit names the rule by which it decides
     and applies it to a module as a whole, not test by test;
   - a script of `gate-selftest` that runs `cargo` or repeats a build reads a product made
     once before the clock.
   What a cut may not be: a test deleted or marked ignored, an assertion weakened, a plant
   that no test holds any more, a step moved out of the clock, more shards or jobs on the
   forge.

4. The local gate catches the same failure. `check-runner` gives `check` the table
   `GATE_CPU_BOUNDS` (`CPU_BOUNDS_BY_CORES`, `taskfiles/forge.yml`); it has no entry for the
   build node's 32 cores. The unit adds the entry with its pair of runs, the code before the
   cuts over it and the code after them under it, both recorded in the same commit, as the
   table's rule in `taskfiles/forge.yml` asks. The CPU seconds on that node move with its
   load (399.5 at load 10, 462.2 at load 25 for the same code): the unit takes three runs of
   each side, names the load of each, and sets the entry between the highest reading after
   and the lowest reading before. Where the two ranges overlap it adds no entry and reports
   the readings.

## Acceptance

- `check` on the forge on a head with the unit prints the runner line, and its
  `gate-time test-steps <s> of 120` reads s at or under 62 on a runner whose yardstick line
  reads at most 2.3 s, or s at or under 120 on a runner whose yardstick reads more. The
  pull request names the run with both lines. The coordinator starts the run; the unit
  cannot.
- On the build node, `gate-cpu test-steps` of `slot --timeout 30m task check-runner` on the
  unit's head reads at most 80 % of its reading on the unit's base at a load within 5 of the
  base run's, in two of three runs; the report gives the six lines.
- `GATE_TIME_BUDGET`, the bounds of the lane `slow` and the scripts
  `tests/gate/gate-time-refuses-test-steps-over-the-budget.sh` and
  `tests/gate/gate-time-refuses-test-steps-over-the-cpu-bound-or-building.sh` are unchanged
  but for a line the new entry needs.
- The list of test names of each step before and after: a name that is gone is named in the
  report with the test that holds its assertions now. The count of assertions that ran is
  not smaller for a merged test: the report shows the cases of the old tests in the new one.
- Each thinned sweep: its plant red before and after, with the budget.
- No test over 60 s; the unit's own tests each name their time.
- `task check-runner` exit 0 on the merged head; the lane `slow` is not slower: the steps
  the unit touched that the lane also runs (`slow-tests`) read no more than before in one
  run of `slot --timeout 30m task slow-tests` on base and head.

## Out of scope

- The forge's choice of runner, a larger or a paid runner, more jobs for `check`: the
  operator's.
- The lane `slow`: `story:slow-lane-holds-its-bound-on-the-slower-runner`.
- The interpreter's speed: `story:property-escape-cases-spend-less-in-the-interpreter-loop`
  is built beside this unit; its gain is not counted here, and the unit's base and head
  runs are both taken without it or both with it.
- The build before the clock (`build-tests`): it is not in the 120 s.
