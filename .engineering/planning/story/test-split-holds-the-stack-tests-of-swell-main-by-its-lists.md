---
format: aep.planning-md/3
id: story:test-split-holds-the-stack-tests-of-swell-main-by-its-lists
kind: story
status: draft
title: test-split holds the stack tests of swell::main by its lists
summary: The step test checks the written filter of its run at opt-level 1 by counting spellings; a second package part in another spelling passes them and the split, and only the guard stops it when the tests run. Backlog, from pass 2 of the adversary on unit laneseven of wave big-2.
revision: 1
---
## Context

Found by pass 2 of the adversary on the unit `laneseven` of wave big-2
(`review-result:adversary-laneseven-pass-2`, its one note; read in
`review-result:laneseven-coordinator-read`). Introduced by that unit.

The gate's step `test` runs in two runs. The internal task `test-split` writes the filter
of the run at opt-level 1 as one line, `(binary_id(swell::main) & (test(=<name>) | ...)) |
((package(=<p>) | ...) & not binary_id(=swell::main) & not (binary_id(=<binary>) &
test(=<test>)) ...)`, and the filter of the run at opt-level 0 as `not (...)` of it. The
exclusion `& not binary_id(=swell::main)` keeps the package term from taking every test of
`swell::main`, the stack tests among them, into the run at opt-level 1.

`test-split` holds that exclusion by counting spellings in the written line: `package(=`
as often as `TEST_OPT1_ARGS` names packages, `binary_id(swell::main) & (` once and `) & not
binary_id(=swell::main)` once. A second package part in another spelling, `|
package(swell)` with the glob matcher, added to the function that writes the line, keeps
all three counts. With it the six planted scripts pass and the split passes with the real
lists: `test: swell::main 1770 at opt-level 1, 0 at opt-level 0`, each test in one run,
the 73 tests the stack rule prints "stack" at opt-level 1. Only the guard
(`SWELL_NO_STACK_TESTS=1`) stops it, when the tests run: the gate is red after the split,
with a panic for each such test, and no line of the split says why.

It needs an edit of the function. No value of a caller and no file of the tree reaches it.

Filed here, in the backlog. No wave takes this story before the planned waves are
delivered.

## Change

To be written when the story is planned. The direction: `test-split` compares lists and
not spellings. From `.out/test-stack.txt` and `.out/test-list-opt1.txt` it fails, naming
the test and before any test runs, when the list of the run at opt-level 1 holds a test of
`swell::main` that the stack rule does not print "other" (one it prints "stack", one a
macro writes without a `fn` line). The counts of spellings stay or go with that check. A
planted script under `tests/gate/` shows the second package part red, in both spellings,
without a build.

## Acceptance

To be written when the story is planned: the mutant of the pass (a second package part
with the glob matcher) and the same part with `=` are refused by a planted script and by
the split with the real lists, with a line that names a stack test; the unchanged step
prints the same two `test:` lines and the same summed line; `gate-selftest` passes; the
seconds of the new script run alone are in the report.
