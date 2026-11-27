---
format: aep.planning-md/3
id: approval-record:wave-big-2-lane-unit-6
kind: approval-record
status: draft
title: Wave big-2 takes the story of the gate's test steps on the slower runner as lane unit 6
relations:
- decides: story:gate-test-steps-hold-the-budget-on-the-slower-runner
revision: 2
---
The forge ran `check` on the first pull request of wave big-2 at `fd71252b` on 2026-11-27 (run
45979562872, attempt 1) and failed it with `gate-time: the test steps ran 147.6 s, over the
budget of 120 s`. The same steps read 77.2 s on `8d6cfebc` (run 45972624878), which differs
from `fd71252b` in files under `.engineering/` only: the second runner took 1.94 times the
CPU seconds for the same work.

The coordinator put the question to the operator with two options: merge the pull request
when a second attempt of the run is green, and build a unit that makes the test steps hold
120 s on the slower runner before the wave's second pull request; or hold the pull request
until that unit is built. The operator's decision of 2026-11-27, 11:31 CET: the first
option, and the unit, with a line in the log of `check` that names the runner's CPU, is built
before any further feature unit.

`story:gate-test-steps-hold-the-budget-on-the-slower-runner` is filed with this record and
taken into wave big-2 as lane unit 6. Wave big-2 has 28 stories with it. The feature unit
that has not started (`story:slice-08-conformance-close`) starts after this unit is built;
the units that run (`story:private-names-in-eval-code`,
`story:property-escape-cases-spend-less-in-the-interpreter-loop`) go on.
