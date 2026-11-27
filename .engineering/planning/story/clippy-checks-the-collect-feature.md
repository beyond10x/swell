---
format: aep.planning-md/3
id: story:clippy-checks-the-collect-feature
kind: story
status: draft
title: clippy checks the code that exists only with collect-every-safe-point
summary: task clippy checks the default features only. Backlog, named by the correction of unit proxytraps of wave big-2.
revision: 1
---
## Context

Named by the correction of the unit `proxytraps` of wave big-2: `task clippy` runs `cargo
clippy --workspace --all-targets --locked -- -D warnings` (`Taskfile.yml`, task `clippy`),
which checks the default features. Code that exists only with the feature
`collect-every-safe-point` (in that unit a combined chain test and its constants; in other
test files the counts and helpers behind `#[cfg(feature = "collect-every-safe-point")]`) is
compiled by the collect runs of the gate and the lane but is never checked by clippy. The gap
is older than the wave: the feature and the task are of earlier slices. It is filed here, in
the backlog. No wave takes it before the planned waves are delivered.

## Change

The gate checks the collect-only code with clippy as well, without a second full compile of
the workspace in the gate's test budget: either one more clippy call for the crates that have
the feature, in the gate if its time allows and in the lane `slow` if not, or a stated reason
in the task's summary why the code is not checked.

## Acceptance

- A warning planted in a copy inside a `#[cfg(feature = "collect-every-safe-point")]` item of
  a test file turns the chosen step red.
- The step's seconds, cold and warm, are in the change's report; the gate's test steps stay
  within 120 s and the lane `slow` within its bound.
