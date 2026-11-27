---
format: aep.planning-md/3
id: review-result:lanefive-correction-1-coordinator-read
kind: review-result
status: active
title: 'Coordinator''s read of correction 1 of the unit lanefive: two tests, no engine code'
relations:
- reviews: story:property-escape-cases-spend-less-in-the-interpreter-loop
revision: 1
---
Coordinator's read of correction 1 of the unit lanefive (impl/lanefive, 2675d93 to
dd3c5d41), 2026-11-27, in place of a second adversary pass: the correction changes one test
file and no line of the engine. Verdict: no defect.

The adversary's pass 1 on `2675d93` was green (`review-result:adversary-lanefive-pass-1`)
with three notes. Read: the diff of `17593fe1`, the merge `dd3c5d41` of `wave/big-2` at
`41622d9f`, the adversary's candidate diff and the unit's report of the correction.

- Note 1, a checked write that stores before it throws passes every test of the tree:
  `17593fe1` adds two tests to `crates/swell/tests/interpreter_loop.rs` (30 lines, the only
  file of the commit). Their lines equal the first two cases of the adversary's candidate
  diff, compared line by line. Each runs in sloppy and in strict mode. With the store planted
  before the check in `crates/swell/src/interp/lexical.rs`, the first case is red: the answer
  is `"ReferenceError,5,number,true"` in both modes, where it must be four ReferenceErrors and
  `true`. The second case passes on that plant; it holds the binding named in the message of
  a read, a write and a compound assignment among three bindings. The planted line is out
  again: the commit holds the test file only.
- The adversary's third candidate case (a `for`-`in` and a `for`-`of` loop give back what
  their enumerator and Iterator Record held) is not taken. It holds no note of the pass: the
  pass found no change that it would catch on this unit, an enumerator and an Iterator Record
  leave the operand stack through `Pop`, whose credit the unit's library test of `replace`
  and the 411 scripts of the pass cover, and it takes 1.46 s in the collect build against
  0.07 s for each of the two taken.
- Note 2, a growth of the operand stack that the memory budget refuses leaves the reserved
  capacity uncharged until the frame unwinds: the base has the same, and the unit's
  `push_operand_grown` reaches it only through `push_entry`. It is filed as a backlog draft
  (`story:operand-stack-charges-its-growth-before-it-reserves`); no wave takes it before the
  operator accepts it.
- Note 3, the frames of three helpers grew by 16 to 32 bytes and the run loop's did not: no
  stack bound of the tree moved, and `task test-stack-profiles` passes on the merged head in
  both profiles.
- The merge of `wave/big-2` at `41622d9f` had no conflict. `git log 933a3de..wave/big-2 --
  crates/swell/src/interp/` is empty: the wave changed no file that the unit's cuts changed.
  The 45 tests of the eval unit, of the dynamic code's library module and of this unit pass
  together in both builds.
- The CPU seconds of the two tests alone, three runs each on a build node: 0.012 to 0.014 in
  the default build, 0.056 to 0.070 in the collect build.
- One run of the collect build's main tests on the unit's head took 369 s of wall time on a
  build node whose pool reported CPU pressure of 28 %; no single test took over 5.5 s, and
  the run before the merge took 29.8 s. The coordinator's run of the same command on the
  same head took 187.9 s, all 1,741 passed. The node's load explains both: in the same
  verification the tests of the syntax crate with full counts, which the unit does not
  touch, took 10.0 s against 2.2 s in the verification of the unit before it, and the main
  tests of the default build 48.0 s against 8.3 s. The gate's own reading of its test steps
  on the merged tree is taken at the wave's close.

The unit ran `perf record` and the first timing runs of its cuts on a workstation of 20
cores, not on a build node; the brief did not forbid it and later briefs do. The ratio the
story bounds (`sh taskfiles/perf-loop.sh`, 0.724) was read there with a pressure of 0.00 in
every run; the test262 part (0.858) was read on a build node.
