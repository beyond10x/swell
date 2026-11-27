---
format: aep.planning-md/3
id: story:gate-time-sum-counts-every-step
kind: story
status: draft
title: The gate's time sum holds every test step, also one that writes no time line
summary: gate-time-sum adds only the lines its steps wrote; a step without a line is in no sum. Backlog, from adversary pass 1 on unit slowlane of wave big-2.
revision: 1
---
## Context

Found by the adversary's first pass on the unit `slowlane` of wave big-2
(`review-result:adversary-slowlane-pass-1`, finding 2) and left open by its correction. The
finding was that `slow-time-sum` added up only the time lines the lane's own steps wrote, so a
part of the lane that wrote no line was in no sum; the correction made `slow-time-sum` read
the lane's clock (`.out/gate-time/start`) as well. `gate-time-sum`, the last step of `check`
(`taskfiles/gate-time.yml`), has the same form: it adds the lines of `.out/gate-time/steps`
and compares the sum with `GATE_TIME_BUDGET` (120 s). A test step placed in `check` with no
`gate-time` call after it is in no sum. No such step exists at the merge of the unit
(`wave/big-2` at `11d92ae`), so the gap is older than the wave and is filed here, in the
backlog. No wave takes it before the planned waves are delivered.

## Change

`gate-time-sum` holds the whole span of the test steps: from the mark the step after clippy
sets to the moment `gate-time-sum` runs, read from the clock, beside the sum of the lines. A
difference between the two above a stated allowance fails the step and names the seconds no
line covers.

## Acceptance

- A selftest under `tests/gate/` runs a copy of the gate's time tasks with a step of 5 s that
  writes no time line, and `gate-time-sum` fails there with a line that names the uncovered
  seconds; without that step it passes.
- `task check` passes on the unchanged tree, within its budget.
