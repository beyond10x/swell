---
format: aep.planning-md/3
id: story:class-elements-scale-test-reads-no-wall-time
kind: story
status: draft
title: The scale test of class elements reads no wall time
summary: sources::class_elements_scale_linearly bounds a ratio of two wall times and failed once under load in the lane slow (9.4 ms to 267 ms). Backlog, from pass 2 of the adversary on unit lanefour of wave big-2.
revision: 1
---
## Context

Found by the unit `lanefour` of wave big-2 and by pass 2 of the adversary on it
(`review-result:adversary-lanefour-pass-2`, finding F5). It is older than the unit: the test
came with the commit `ae0397e`, which the unit's base holds.

`sources::class_elements_scale_linearly` (`crates/syntax/tests/sources.rs`, the test at line
3009, its time assertion near line 3015) parses five shapes of a class at two sizes and holds,
beside the bytes charged and the peak, that four times the elements "take at most ten times
the time": it compares two wall times, with 50 ms added. In a cold run of the lane `slow` on
the build node at 4 CPUs under a load of 16 the test failed once, for the shape "one private
name used n times" (9.4 ms at the small size, 267 ms at the large one), and passed on the next
run. It runs in the gate's step `test` and in the lane's `slow-tests`.

A test that bounds a wall time fails under load without a defect, and a lane that fails once
in some runs is run again until it is green, which hides the next real failure. The bytes
charged and the peak, which the same test holds, do not depend on the load.

The finding is older than the wave and is filed here, in the backlog. No wave takes this
story before the planned waves are delivered.

## Change

To be written when the story is planned. The direction: the test holds the growth with a
count that does not depend on the load (the steps or tokens the parser and the scope analysis
count, or the CPU time of the test's own thread, read as the other tests of the repository
read it), and keeps the bytes and the peak as they are. A quadratic walk planted in one shape
is red; the test stays under the time bounds of `AGENTS.md` ("Commands and the gate").

## Acceptance

To be written when the story is planned: the test passes 20 runs in a row on the build node
beside a job that loads every core, and is red with a quadratic loop planted in the shape
"one private name used n times".
