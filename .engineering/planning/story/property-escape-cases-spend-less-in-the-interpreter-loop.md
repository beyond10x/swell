---
format: aep.planning-md/3
id: story:property-escape-cases-spend-less-in-the-interpreter-loop
kind: story
status: draft
title: The property-escape cases spend less in the interpreter loop
summary: Three quarters of a property-escape case of test262 is the interpreter running the harness loop buildString; those cases are 73 % of the test262 run's CPU time. The loop's CPU seconds drop to 80 % or less, with every step count, heap reading and stack bound unchanged. Lane unit 5 of wave big-2.
revision: 3
---
## Context

Found by the unit `lanefour` of wave big-2 (its finding F1), which bounds the lane `slow`. It
is older than that unit and outside its scope: that unit changes how the lane builds and
runs, not the interpreter.

The cases of test262 under `built-ins/RegExp/property-escapes/generated/` build their
subject strings with the harness's `buildString` (`harness/regExpUtils.js`), a loop of about
1.1 million iterations per case that runs in the interpreter: per iteration one indexed put
(`codePoints[length++] = codePoint`), one strict comparison, one `<=`, one `++`, and each
10,000 iterations one `String.fromCodePoint.apply` and one `+=` of strings. A profile on the
build node (`perf`, one worker, the three slowest cases of slice 05, after that unit's memo of
the regexp classes) reads: `Agent::run` 21.4 %, `push_entry` 8.9 %, `Vec<Entry>::truncate`
6.4 %, `replace` 6.4 %, `Frame::instructions` 3.0 %, and 2 to 3 % each for `put_index`,
`internal_set`, `get_local`, `get_local_checked`, `convert_to_numeric`, `set_local_checked`
and `strict_equality`; the regexp matcher (`attempt`, `push`, `read`) takes 12 % together.
Three quarters of a case's time is the loop, not the match.

The property-escape cases are 854 of the 1,171 CPU seconds of the test262 run on the build
node (73 %). That unit estimates that each 10 % the interpreter gains on that loop takes
about 20 s off the part `slow-test262` of the lane on the forge.

The lane `slow` was measured on the forge on 2026-11-27 with that unit merged (run
45963552396): `slow-time lane 882.6 of 900, longest part slow-test262 504.8 of 600`. The
margin to the bound of `story:realm-size-lane-within-its-share` is 17.4 s, and the wave's
remaining units add tests and cases to the lane. The operator took this story into wave
big-2 on that figure (`approval-record:wave-big-2-lane-unit-5`); before, it was a backlog
draft.

## Change

The interpreter spends fewer CPU seconds per iteration of a loop of this kind. Nothing a
script or a host can observe changes.

1. A measurement source, kept in the tree: `tests/perf/build-string-loop.js`, the body of
   `buildString` over one range of a fixed size with no regexp match, and a task `perf-loop`
   that runs it on the release build of the CLI three times and prints the CPU seconds of
   each run (`perf-loop: run <n> <seconds> cpu s`) and the median. The task is a
   measurement: it is in no gate and no lane, and it is not run through the pool.
2. A profile of that source on the build node (`perf record` on the release CLI), read
   before any change; the unit's report names the functions over 2 % and the instructions
   the loop executes per iteration.
3. Cuts where the profile points, each kept only when the task shows it. The candidates the
   first profile names:
   - the operand stack: `push_entry` reads the running frame twice and checks the capacity
     per push; `replace` drops entries and pushes (a truncate and a push) where one write in
     place serves when the count is one or more;
   - `Frame::instructions` resolves the template's instruction slice per instruction;
   - the checked reads and writes of locals (`get_local_checked`, `set_local_checked`) and
     `get_local` reach the frame through `top_frame` once per step of their own;
   - the indexed put on an array with an integer index (`put_index`, `internal_set`).
4. What stays as it is:
   - no `unsafe`, no outside crate, `#![forbid(unsafe_code)]` in every engine crate;
   - the steps a script spends: every test that asserts a step count is unchanged and green,
     and `expected.txt` is byte for byte the file of the base (the run's results and its
     lines);
   - the memory a script is charged: the operand stack's growth is charged as before, and
     every test that reads `heap_bytes()` or `live_bytes()` is unchanged and green;
   - the interrupt and the step budget are read where they are read now;
   - the native stack a re-entry takes: the stack tests (`taskfiles/realm-room-tests.awk`
     with `rule=stack`) and the task `stack-profiles` stay green with no bound raised. The
     loop of `dispatch.rs` keeps each instruction's locals in a function of its own for this
     reason; a cut that moves locals into the loop's frame is measured against these tests
     first.

## Acceptance

- `task perf-loop` on the build node, the base and the head, three runs each: the median of
  the head is at most 80 % of the median of the base. Both sets of three lines are in the
  unit's report, with the load of the node at each.
- The property-escape part of the test262 run on the build node (the cases under
  `built-ins/RegExp/property-escapes/generated/`, the run's CPU seconds summed from its run
  lines), the base and the head: the head is at most 87 % of the base.
- `conformance/test262/expected.txt` and its digest are unchanged by the unit.
- No test that asserts a step count, a heap reading or a stack bound is edited by the unit:
  `git diff --stat` of the unit over `crates/*/tests` shows new tests only, and the unit's
  report lists them.
- A test per cut that holds what the cut could break: an operand stack that grows across the
  changed push (the charge equals the bytes before the change, read through
  `heap_bytes()`), a `replace` of 0, 1 and several entries, a read and a write of a local in
  its temporal dead zone, an indexed put past the array's length, on a frozen array, with a
  setter on the prototype chain and with a Proxy in the chain. Each test alone is under
  1.5 CPU s in the default build and 2 s in a collect build.
- The gate (`task check-runner`) exits 0 with its test steps within 120 s, and the lane
  `slow` on the build node is green with each part at or under its time on the base.
- Stop rule: when the cuts of item 3 together leave the median of `task perf-loop` above
  90 % of the base, the unit stops, commits the measurement source and the task alone, and
  reports the profile; nothing of the interpreter is merged on a gain under 10 %.

## Not in this story

- A change of the instruction set, of the compiler's output or of the frame's layout that
  moves a step count or a heap reading.
- The regexp matcher (12 % of a case) and the string concatenation of `buildString`.
- The forge's run of the lane: the coordinator starts it on the wave branch after the merge
  and reads `slow-time lane` there.
