---
format: aep.planning-md/3
id: story:property-escape-cases-spend-less-in-the-interpreter-loop
kind: story
status: draft
title: The property-escape cases spend less in the interpreter loop
summary: Three quarters of a property-escape case of test262 is the interpreter running the harness loop buildString (about 1.1 million iterations per case); those cases are 73 % of the test262 run's CPU time. Backlog, from the unit lanefour of wave big-2.
revision: 1
---
## Context

Found by the unit `lanefour` of wave big-2 (its finding F1), which bounds the lane `slow`. It
is older than the unit and outside its scope: the unit changes how the lane builds and runs,
not the interpreter.

The cases of test262 under `built-ins/RegExp/property-escapes/generated/` build their
subject strings with the harness's `buildString`, a loop of about 1.1 million iterations per
case that runs in the interpreter. A profile on the build node (`perf`, one worker, the three
slowest cases of slice 05, after the unit's memo of the regexp classes) reads: `Agent::run`
21.4 %, `push_entry` 8.9 %, `Vec<Entry>::truncate` 6.4 %, `replace` 6.4 %,
`Frame::instructions` 3.0 %, and 2 to 3 % each for `put_index`, `internal_set`, `get_local`,
`get_local_checked`, `convert_to_numeric`, `set_local_checked` and `strict_equality`; the
regexp matcher (`attempt`, `push`, `read`) takes 12 % together. Three quarters of a case's
time is the loop, not the match.

The property-escape cases are 854 of the 1,171 CPU seconds of the test262 run on the build
node (73 %). The unit estimates that each 10 % the interpreter gains on that loop takes about
20 s off the part `slow-test262` of the lane on the forge.

The finding is older than the wave and is filed here, in the backlog. No wave takes this
story before the planned waves are delivered.

## Change

To be written when the story is planned. The direction: measure the loop of `buildString`
alone (a source with the harness function and a count that the time bounds allow), name the
instructions and the helper calls it spends its time in, and cut the cost per iteration where
the profile points: the entry stack's push and truncate per call, the checked reads and
writes of locals, the indexed put. No `unsafe`, no dependency, and the step budget, the
memory budget and the interrupt are charged and read as before.

## Acceptance

To be written when the story is planned: the CPU seconds of that source before and after
on the build node, three runs each; the CPU seconds of the property-escape part of the
test262 run before and after; every test of the gate and the lane `slow` green; the steps a
script spends unchanged in every test that asserts a step count.
