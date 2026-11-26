---
format: aep.planning-md/3
id: story:stack-bound-parts-fit-their-cpu-bounds
kind: story
status: draft
title: Each part of the stack-bound test fits its CPU bound, at full counts too
revision: 1
---
## Context

Found by the unit `classsyntax` of wave big-2 while it timed its own tests
(`docs/plan/wave-big-2.md`). The syntax crate's test
`sources::every_nesting_kind_fits_the_stack_bound` is split into 17 parts, each bound to 1.5
CPU seconds in the gate's count and to 2.5 with `SWELL_FULL_COUNTS=1`
(`crates/syntax/tests/sources.rs`, the doc comment above the parts). Measured on the build
node on 2026-11-26, each part alone, user plus system seconds:

- part 8 with `SWELL_FULL_COUNTS=1`: 2.69 on the wave's base (`896bcbe9`) and 2.66 on the
  unit's head, over the bound of 2.5;
- part 16 with `SWELL_FULL_COUNTS=1`: 2.42 on the base and 2.48 on the unit's head, 2.56 in
  an earlier measurement of the same tree.

The unit's change moves neither part (its two new nesting kinds land in the first part and
in parts 2, 3, 4, 5 and 17, the slowest of them 1.89). The finding is older than the wave, so
it is filed here, in the backlog. No wave takes it before the planned waves are delivered.
In the gate's count the slowest part takes 1.11 (part 16), under its bound of 1.5.

## Change

Not designed. The doc comment of the test says how the last such case was handled: the full
count went from 10,000 to its present value when the eighth part took 2.5 CPU seconds alone.
The choices a story weighs: a further split of parts 8 and 16, a lower full count, or a
cheaper source per nesting kind in those parts. Nothing checks the two bounds today; they
are stated in a comment and measured by hand.

## Acceptance

Each of the 17 parts, run alone on the build node, takes at most 1.5 CPU seconds in the
gate's count and at most 2.5 with `SWELL_FULL_COUNTS=1`, the report gives the 34 measured
times, and `task check-runner` passes.
