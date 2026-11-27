---
format: aep.planning-md/3
id: story:slow-lane-holds-its-bound-on-the-slower-runner
kind: story
status: draft
title: The lane slow holds its bound on the forge's slower runner
summary: 'The forge ran the lane slow on the same code in 882.6 s, 1,142.0 s and 876.4 s of its 900: one runner is 1.294 times slower than the other. The lane ends within 900 s and each part within 600 s on the slower runner (about 695 s on the faster), with the bounds unchanged. Backlog, from the first pull request of wave big-2.'
revision: 1
---
## Context

Found at the first pull request of wave big-2. The lane `slow` is bounded on the forge at
900 s, with 600 s for each part (`story:realm-size-lane-within-its-share`). The forge ran the
lane three times on 2026-11-27 on code that differs in no file the lane builds or runs
(`b9adbe35`, `d16fe575`, and `8d6cfebc`, which changes the lane's shell only):

| run | head | lane | slow-test262 | slow-tests | test-realm-sizes | stack-profiles |
|---|---|---|---|---|---|---|
| 45963552396 | `b9adbe35` | 882.6 | 504.8 | 170.4 | 185.2 | 22.2 |
| 45967542195 | `d16fe575` | 1,142.0 | 634.0 | 223.0 | 259.6 | 25.4 |
| 45972624800 | `8d6cfebc` | 876.4 | 504.4 | 167.6 | 183.3 | 21.1 |

The second run is over both bounds (the lane, and the part `slow-test262` at 634.0 of 600).
It ran on the same runner image as the first. Its builds took 1.11 to 1.14 times the first
run's (the release build of the stack tests 13.77 s against 15.75 s, the build of the profile
collect 382.2 s against 425.7 s) and its single tests 1.4 to 1.56 times. The logs of the
first two runs do not name the runner's CPU. From `8d6cfebc` on the lane prints it; the third
run reads `slow-runner: 4 cores, AMD EPYC 9V74 80-Core Processor, 16366632 kB, sh is dash`.

The lane as a whole took 1.294 times as long in the second run as in the first. A lane
that reads 900 s on the slower runner reads about 695 s on the faster one; today it reads
876 to 883 s there. The margin the bound has on the faster runner (17 to 24 s) is smaller
than the difference between two runners of the forge (about 260 s).

`story:property-escape-cases-spend-less-in-the-interpreter-loop` takes about 20 s off
`slow-test262` on the forge for each 10 % the interpreter gains on one loop. It does not
close this gap alone.

## Change

The lane `slow` ends within its bounds on the forge's slower runner, with the bounds as they
are (900 s and 600 s). A bound is not raised, a part is not dropped from the lane, and no
test leaves the lane without a place that still runs it.

The unit first names the slower runner: it reads the `slow-runner` line of the forge's runs
of the lane since `8d6cfebc` and reports each CPU model with the lane's seconds on it. With
one model only in those runs, it says so and works from the ratio 1.294.

Then it cuts where the seconds are, measured before and after on the forge. Where they are
in run 45972624800, as a start for the unit's own reading:

- `slow-test262`, 504.4 s: the test262 run with 4 workers beside the build of the profile
  collect (375.1 s of build in the background).
- `test-realm-sizes`, 183.3 s: the tests that read the realm's room, in the default mode and
  the collect mode at every size of `REALM_SIZES`.
- `slow-tests`, 167.6 s: the full counts in three builds; two library tests of the collect
  build take 21.4 s and 17.1 s (`holders_give_the_kept_stack_back_its_room_across_a_call`,
  `the_step_totals_of_the_sites_held_across_a_call_are_unchanged`).

## Acceptance

- A run of the workflow `slow` on the forge on the unit's head prints `slow-time lane <s> of
  900` with s at or under 695 on a runner whose `slow-runner` line names the CPU of run
  45972624800, and no part over 463 (600 divided by 1.294); or, on a runner the unit showed
  to be the slower one, s at or under 900 and no part over 600. The pull request names the
  run and its `slow-runner` line.
- The bounds in `taskfiles/gate-time.yml` are unchanged, and
  `tests/gate/slow-time-refuses-a-part-over-its-bound.sh` and
  `tests/gate/slow-time-refuses-the-lane-over-its-bound.sh` pass unchanged.
- Every test the lane ran before the unit runs after it, in the lane or in the gate: the
  unit's report compares the lists of test names of both, per build.
- The gate's test steps stay within 120 s and no test runs over 60 s.

## Out of scope

- The forge's choice of runner, and a larger or a paid runner: the operator's.
- Splitting the lane into several forge jobs (one per part): it changes what the bound of
  900 s measures and is the operator's decision.
- The interpreter loop: `story:property-escape-cases-spend-less-in-the-interpreter-loop`.
