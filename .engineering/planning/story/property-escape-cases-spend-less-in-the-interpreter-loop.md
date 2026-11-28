---
format: aep.planning-md/3
id: story:property-escape-cases-spend-less-in-the-interpreter-loop
kind: story
status: implemented
title: The property-escape cases spend less in the interpreter loop
summary: Three quarters of a property-escape case of test262 is the interpreter running the harness loop buildString; those cases are 73 % of the test262 run's test time. The loop's CPU seconds drop to 90 % or less (aim 80 %), with every step count, heap reading and stack bound unchanged. Lane unit 5 of wave big-2.
revision: 10
transitions:
- {from: "draft", to: "proposed", at: "2026-11-27T08:41:58Z", actor: "human:operator", revision: 6, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-27T08:41:59Z", actor: "human:operator", revision: 7, executor: "agent:coordinator"}
- {from: "active", to: "implemented", at: "2026-11-27T22:05:32Z", actor: "human:operator", revision: 10, decided_on: {"recorded":{"test_result":1,"review_outcome":2,"verification":1}}, executor: "agent:coordinator"}
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
Three quarters of a case's time is the loop, not the match. The release profile strips the
debug information, so `Agent::run` holds what the compiler inlined into it.

That unit reported the property-escape cases as 854 of the 1,171 seconds of test time of the
test262 run on the build node (73 %), summed from the wall time the runner measures per run,
and estimated that each 10 % the interpreter gains on that loop takes about 20 s off the part
`slow-test262` of the lane on the forge.

The lane `slow` was measured on the forge on 2026-11-27 with that unit merged (run
45963552396): `slow-time lane 882.6 of 900, longest part slow-test262 504.8 of 600`. The
margin to the bound of `story:realm-size-lane-within-its-share` is 17.4 s, and the wave's
remaining units add tests and cases to the lane. The operator took this story into wave
big-2 on that figure (`approval-record:wave-big-2-lane-unit-5`); before, it was a backlog
draft.

## Change

The interpreter spends fewer CPU seconds per iteration of a loop of this kind. Nothing a
script or a host can observe changes.

1. A measurement source, kept in the tree: `tests/perf/build-string-loop.js`. It copies the
   body of `buildString` over ranges of a fixed size with no regexp match, is sized so that
   one run takes at least 5 CPU seconds on the base, needs no harness file and ends with
   `print(result.length)`.
2. A script `taskfiles/perf-loop.sh` (POSIX sh and awk) and a task `perf-loop` in a new file
   `taskfiles/perf.yml` whose one command is that script (the coordinator adds the include
   line of `Taskfile.yml`; a variable of the task has the prefix `PERF_` and a line in
   `GATE_VARIABLES`). The script builds nothing. It runs the release `swell` that
   `PERF_BIN` names (default `target/release/swell`) with no limit flag, three times,
   alternating with the binary `PERF_BASE_BIN` names when it is set (the release `swell`
   built at the commit that adds the script, kept under `.out/perf/`). It reads CPU
   seconds as user plus system of the child, as `taskfiles/gate-time.yml` reads them, and
   prints `perf-loop: run <n> <head|base> <seconds> cpu s, pressure <avg10 of
   /proc/pressure/cpu>` per run and one line `perf-loop: head <median> base <median> ratio
   <r>`. It exits non-zero when a run's exit status is not 0 or its output is not the
   committed line. It asserts nothing about time: no committed test and no script of the
   gate reads a clock or a CPU time against a bound. The script is a measurement: it is
   in no gate and no lane, it is started by its path without the pool's prefix, both
   binaries run on one machine, and it reads and writes nothing under `bench/`
   (`story:bench-image-and-first-measurements` owns that tree).
3. A profile of that source (`perf record` on the release CLI), read
   before any change; the unit's report names the functions over 2 % and lists the
   instructions of the loop body from `swell_compiler::disasm::disassemble` (a scratch
   test, not committed).
4. Cuts where the profile points, each kept only when the task shows it. The candidates:
   - `replace(0, v)` is `push_value(v)` with no truncate (the commonest `replace` has a
     count of 0: `GetLocal`, the constants, `Dup`). For a count of one or more nothing is
     pushed past the capacity, so no charge arises, and the value may be written in place.
     `truncate_stack` calls `credit` only when the removed entries held bytes; a removed
     `Enumerator` or `Iterator` entry still leaves `frame.charged` and is credited.
   - `Frame::instructions` (`frame.rs`) and its caller `execute_next` (`dispatch.rs`) may be
     rewritten inside their bodies.
   - The operands of an instruction are read in one borrow of the frame (a function that
     answers two or three entries, with the same fault on a missing entry or an entry that
     is no value) where `peek_value` is called two or three times today; `get_local`,
     `get_local_checked` and `set_local_checked` read the frame once.
   - `to_numeric` answers a `Number` unchanged, without the calls of `to_primitive` and
     `to_number`.
   - The indexed put on an array with an integer index: the body of `put_index` and
     `array_set_index`. The step per prototype hop stays one `charge_steps(1)` call per
     hop (each call reads the interrupt), and the `keep` and `release` of `put_index` stay,
     because they hold the roots for a setter.
5. What stays as it is:
   - no `unsafe`, no outside crate, `#![forbid(unsafe_code)]` in every engine crate;
   - the layout of `Frame` and of `Entry`: no field is added, removed or changed, since
     `frame_bytes()` and `entry_bytes()` are charged and every heap reading would move;
   - the signatures of `replace`, `push_value`, `pop_value`, `peek_value`, `top_frame`,
     `push_entry`, `drop_entries` and `Frame::instructions`: the wave's open units call them
     (`interp/class.rs`, `interp/private.rs`, `dynamic_code.rs`). A new helper is added
     beside them and the old ones may call it. The arm line `I::PutIndex =>
     self.put_index(strict)` of `execute` stays, since an open unit adds arms after it;
   - the steps a script spends, the points where the step budget and the interrupt are
     read, and the bytes a script is charged;
   - the results of test262: `conformance/test262/expected.txt` is not changed by the unit
     (the coordinator regenerates its digest at the merge, since the digest covers the
     sources of the runner's crates);
   - the native stack a re-entry takes: `slot --timeout 30m task test-stack-profiles` exits
     0 on the unit's head merged with the wave branch (`AGENTS.md`, "A change under
     `crates/swell/src` runs the stack lane on its merged head"), with no bound of a stack
     test raised. The bound is measured in the profile `stack-debug`, which inlines nothing,
     and one route had 60,768 bytes too few on 2026-11-27. A path that grows something is a
     function of its own marked `#[inline(never)]`, as the comment of the loop in
     `dispatch.rs` requires; `replace`, `push_entry`, `top_frame` and `put_index` gain no
     `#[inline(always)]`, and no fast path is written into the bodies of `execute`,
     `put_index` or `call_instruction`.

## Acceptance

- `PERF_BIN=<the head binary> PERF_BASE_BIN=<the base binary> sh taskfiles/perf-loop.sh`
  prints `perf-loop: head
  <m> base <m> ratio <r>` with `r` at most 0.90, and exits 0. The six run lines and the
  ratio line are in the unit's report. The unit's aim is 0.80; the report says which of the
  two it reached.
- The property-escape part of the test262 run: one part of `task test262-part` that holds
  at least 30 files under `built-ins/RegExp/property-escapes/generated/` (the report names
  it), with `TEST262_WORKERS=1`, on the base and on the head, three runs each, alternating.
  The sum of the nanoseconds field of the part file over the runs of those files (the awk
  line is in the report): the median of the head is at most 93 % of the median of the base.
  These are wall nanoseconds of one worker; the report gives `/proc/pressure/cpu` before
  each run.
- The run lines of that part (path, mode, outcome, cause) and their steps are equal on the
  base and the head (`cmp` of the part files with the nanoseconds field cut), and `git diff
  --quiet <base> -- conformance/test262/expected.txt` exits 0.
- No assertion of a step count, a heap reading or a stack bound is changed: `git diff -U0
  <base> -- 'crates/*/tests' | grep -E '^-[^-]'` prints only lines of a generated
  `tests/main.rs`, and for each file of `crates/*/src` with a `#[cfg(test)]` line no hunk of
  `git diff -U0 <base> -- <file>` starts at or after that line except to add a test. The
  report lists the new tests.
- For each cut that is kept, the tests of this list that touch its code: an operand stack
  that grows across the changed push (the test asserts a literal that is `heap_bytes()`
  measured on the base; the report names it), a `replace` of 0, 1 and several entries with
  an `Enumerator` or `Iterator` entry among those removed, a read and a write of a local in
  its temporal dead zone, a numeric operator on a Number, on an object with `valueOf`
  and on a Symbol (the TypeError), an indexed put past the array's length, on a
  frozen array, with a setter on the prototype chain and with a Proxy in the chain, and one
  that ends at the step budget inside the prototype walk of an indexed put at each of the
  last three steps. Each test alone is under 1.5 CPU s in the default build and 2 s in a
  collect build, measured as `AGENTS.md` says (`--exact`, `--test-threads 1`, `times`).
- `slot --timeout 30m task test-stack-profiles` exits 0 on the unit's head merged with the
  wave branch.
- The gate (`slot --timeout 30m task check-runner`) exits 0 with `gate-time test-steps <s>
  of 120` (the line of `test262-check` on the digest is the coordinator's at the merge and
  is named as known red when it is the only one), and `slot --timeout 30m task slow` on the
  head exits 0; its `slow-time` lines are in the report.
- Stop rule: with a ratio above 0.90 after the cuts of item 4, the unit stops: `git diff
  --stat <the commit that adds the task>..HEAD -- crates` is empty, only the measurement
  source and the task are committed, and the report says `stopped, ratio <r>` with the
  profile. With a ratio at most 0.90 the cuts are kept and the lines above apply.

## Not in this story

- A change of the instruction set, of the compiler's output or of the layout of `Frame` or
  `Entry`; anything that moves a step count or a heap reading.
- The regexp matcher (12 % of a case) and the string concatenation of `buildString`.
- A new column of CPU time in the part files of the test262 runner.
- The forge's run of the lane: the coordinator starts it on the wave branch after the merge
  and reads `slow-time lane` there.

## As delivered

The unit `lanefive` of wave big-2 delivered the story on `impl/lanefive` (one correction after
the adversary's pass), merged into `wave/big-2`. Where the delivery differs from the text
above, and what was measured:

- Three cuts were taken, each its own commit with its six run lines in the body:
  - A (`a900baf`): `replace(0)` is a push, `replace(n)` for n of 1 or more writes in place,
    and `Pop` and the conditional jumps drop the top entry. Ratio 0.809 against the binary
    before it.
  - B (`997628f`): `next_instruction` reads the instruction without a call of
    `Frame::instructions`. Ratio 0.879.
  - C (`167536e`): the operands of an instruction are read in one borrow, a checked local is
    read once, `held_bytes` is inlined. Ratio 0.954.
- A fourth candidate (`to_numeric` returns a Number unchanged) read 1.034 and 1.012 in two
  runs and was reverted. A fifth (the indexed put) was not tried: the aim was reached.
- The measurement `tests/perf/build-string-loop.js` runs 80 rounds over a range of 132,097
  code points, not the whole range of Unicode: eight rounds over the whole range end at the
  step budget, because the growth of the result String charges a step per code unit.
- `sh taskfiles/perf-loop.sh` on the unit's head against the base `3e39e71`: head 3.76, base
  5.19 CPU seconds (median of three runs each, alternating), ratio 0.724; the bound is 0.90.
  It was read on a workstation of 20 cores with a pressure of 0.00 in every run, not on a
  build node. The count of user instructions of one run fell from 67.30 to 51.82 thousand
  million.
- The test262 part: part 1/14 with one worker holds 34 property-escape files (68 runs); head
  and base ran three times each on a build node, alternating. The medians of the summed time
  of those cases: head 45.78 s, base 53.35 s, ratio 0.858; the bound is 0.93. The six part
  files are equal in path, mode, outcome, cause and steps.
- No step count, heap reading or stack bound of the tree's tests moved. No instruction was
  added and no public item changed. The engine's diff ends before the first test module of
  each file it touches.
- Tests: four in `crates/swell/tests/interpreter_loop.rs` and two library tests in
  `crates/swell/src/interp/loop_tests.rs` with the cuts, each red with a defect planted in
  its cut; two more in `interpreter_loop.rs` with the correction, for a write in the temporal
  dead zone, which no test of the tree held (the adversary's pass found it with a mutant).
  The slowest takes 0.234 CPU seconds in the collect build.
- The adversary ran 411 scripts on the base and on the head: the same completion, text,
  steps, heap bytes and peak, and the same outcome at every step budget and at every memory
  budget 8 bytes apart. A growth of the operand stack that the memory budget refuses leaves
  the reserved capacity uncharged until the frame unwinds, on the base as on the head:
  `story:operand-stack-charges-its-growth-before-it-reserves` (a backlog draft).
- `taskfiles/perf-loop.sh` builds nothing, is in no gate and no lane and asserts nothing
  about time. It is started by its path after the two release binaries are built.
- What the story is for, the lane `slow` on the forge, is shown only by a forge run of the
  merged tree; the wave's second pull request has it.
