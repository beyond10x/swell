---
format: aep.planning-md/3
id: story:library-tests-fit-the-time-bound-in-the-collect-build
kind: story
status: draft
title: The library tests of swell take under 2 s each in the collect build
summary: One library test takes 13 to 25 s with a collection at every safe point at opt-level 0. Backlog, measured in wave big-2.
revision: 1
---
## Context

Measured in wave big-2 by three units (`super`, `weak` and the adversary's pass on `weak`):
the library test
`object::builtin_function::tests::holders_give_the_kept_stack_back_its_room_across_a_call`
takes 0.15 s in the default build and 13.2 to 24.5 s in the library's test build with
`collect-every-safe-point` at opt-level 0 (15.5 s in the runs of `weak`). It runs `flat` of an
Array nested 300 deep, a dynamic function with 300 parameters and `JSON.stringify` with a list
of 300 property names, each twice. The wave's rule for a test is under 2 CPU s alone in a
collect build; the unit `lanetests` of the wave applies it to the integration tests
(`crates/swell/tests/`) and changes no line of `crates/*/src`, where this test lives. The test
is older than the wave and is filed here, in the backlog. No wave takes it before the planned
waves are delivered.

## Change

Every library test of `swell` takes under 2 CPU s alone in the collect build. A test over it
takes a smaller count in that mode only (`cfg!(feature = "collect-every-safe-point")` beside
the count), the default mode's count unchanged, and the smaller count is shown red in that
mode with the test's defect planted.

## Acceptance

- A table of every library test of `swell` alone on one thread in the collect build at
  opt-level 0, before and after; none over 2 s after.
- For each changed test: the planted defect and its red line at the smaller count.
- `cargo nextest run --locked -p swell --lib` passes in both modes.
