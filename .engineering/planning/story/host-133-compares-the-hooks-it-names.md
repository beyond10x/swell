---
format: aep.planning-md/3
id: story:host-133-compares-the-hooks-it-names
kind: story
status: draft
title: The determinism test of HOST-133 compares the hooks its name promises
summary: The transcripts the test compares hold no line of a hook. Backlog, from adversary pass 1 on unit lanetests of wave big-2.
revision: 1
---
## Context

Found by the adversary's first pass on the unit `lanetests` of wave big-2
(`review-result:adversary-lanetests-pass-1`, finding 5), in a test older than the wave.
`determinism::host_133_same_calls_same_hooks_same_outcomes` runs one list of calls in two
agents and compares their transcripts. The transcripts of a run hold the outcomes of the
calls (57 or 58 lines: normal ends, three kinds of throw, the step budget, the memory budget,
print, calls back into the host) and no line of a hook: 0 hook lines in the default mode and
in the collect mode, at the counts of the base of the wave and at the smaller counts. So the
part "same hooks" of the test's name is held by nothing. The finding is older than the wave
and is filed here, in the backlog. No wave takes it before the planned waves are delivered.

## Change

The list of calls of the test makes each hook of the host interface that a script can
reach fire at least once (the hooks HOST-133 names; the design's list decides), each hook
writes its line into the transcript with its arguments, and the test fails when a kind of
hook the list is meant to reach has no line.

## Acceptance

- The transcript of a run holds at least one line of each hook kind the test names, and the
  test asserts the set of kinds.
- A hook made to differ between the two agents in a copy (an argument that depends on an
  address) turns the test red in both modes.
- The test takes under 1.5 CPU s alone in the default build and under 2 s in a collect build.
