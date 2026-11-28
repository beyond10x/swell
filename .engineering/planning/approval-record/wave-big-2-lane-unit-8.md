---
format: aep.planning-md/3
id: approval-record:wave-big-2-lane-unit-8
kind: approval-record
status: draft
title: Wave big-2 takes the story of the lane slow on the slower runner as lane unit 8
relations:
- decides: story:slow-lane-holds-its-bound-on-the-slower-runner
revision: 1
---
The forge ran the lane `slow` on the third pull request of wave big-2 at `f962c324` on
2026-11-28 (run 46091909506, attempt 1) and failed it: `slow-time lane 1123.4 of 900,
longest part slow-test262 604.9 of 600`. Its runner line reads `slow-runner: 4 cores, AMD
EPYC 7763 64-Core Processor, 16366632 kB, sh is dash`. The job `check` of the same head
was green on its first attempt (run 46091909500, test steps 84.8 s of 120).

The pull request changes no task of the lane: it moves tests of the gate's step `test` to
opt-level 1 (the seventh unit of the lane of work on the gate's time). The lane ran 673.0 s
on the head of the second pull request (`5ab9f8e5`, run 46069633020) on an AMD EPYC 9V45,
and 1,142.0 s on the first pull request's code (`d16fe575`, run 45967542195) on the runner
model of this failure. `story:slow-lane-holds-its-bound-on-the-slower-runner` was filed in
the backlog from the first pull request for this gap.

The coordinator put the question to the operator with two options: merge the pull request
when a second attempt of the lane's run is green, and take the backlog story into the wave
as lane unit 8, built before any feature unit of slices 11 and 12; or hold the pull request
until that unit is built. The operator's decision of 2026-11-28, 02:39 CET: the first
option.

`story:slow-lane-holds-its-bound-on-the-slower-runner` is taken into wave big-2 as lane
unit 8 with this record. Wave big-2 has 29 stories with it. The planning of slices 11 and
12 goes on beside the unit; none of their stories starts before it is built.
