---
format: aep.planning-md/3
id: story:math-random-hook-count-reads-a-hook-that-runs
kind: story
status: draft
title: The hook count of Math.random() is read from a counter that counts
summary: math_random::a_call_makes_no_hook_call_and_costs_one_step asserts a count of 0 and passes when the host's counting hooks are never reached. The test first reads a count above 0 from the same counter. Backlog, from pass 1 of the adversary on the unit laneeight of wave big-2.
revision: 1
---
## Context

`crates/swell/tests/math_random.rs`, test
`math_random::a_call_makes_no_hook_call_and_costs_one_step`, asserts that a call of
`Math.random()` makes no hook call: `assert_eq!(random_hooks, 0, ...)`. The count is read
from the test's counting host. The assertion checks absence only: when the host's counting
hooks are never reached at all (a host whose hooks are not wired to the counter), the count
is 0 and the test passes.

Found by the first adversary pass on the lane's eighth unit of wave big-2
(`story:slow-lane-holds-its-bound-on-the-slower-runner`, step 1): with the dispatch of the
scripted host changed so that it never calls the counting hooks, the test stays green. The
same assertion is on the unit's base; the unit did not bring it in.

## Change

The test proves that its counter counts before it reads a zero from it: in the same test,
one script that does make a hook call (a promise job, as the other tests of the module
enqueue one) reads a count above 0 on the same agent, and the reading of `Math.random()`
follows it as a difference of two counts.

## Acceptance

- With the counting hooks of the test's host replaced by hooks that do not count (a plant
  in the worktree, put back after), `a_call_makes_no_hook_call_and_costs_one_step` fails.
- On the unchanged tree the test passes, and its name is unchanged.
- No file under `crates/*/src/` changes.

## Out of scope

Other assertions of absence in the tests; a survey of them is not part of this story.

## Scope

`crates/swell/tests/math_random.rs`, the one test.
