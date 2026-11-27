---
format: aep.planning-md/3
id: approval-record:wave-big-2-lane-unit-6-cut
kind: approval-record
status: draft
title: Lane unit 6 of wave big-2 runs the gate's tests of swell::main at opt-level 1
relations:
- decides: story:gate-test-steps-hold-the-budget-on-the-slower-runner
revision: 1
---
Lane unit 6 of wave big-2 (`story:gate-test-steps-hold-the-budget-on-the-slower-runner`)
stopped on 2026-11-27 after its first part. It had named the runner in the log of `check`
(`impl/lanesix`, `92f3d55`) and measured the gate's test steps on the build node: `test`
300.1, `test-collect` 124.9 and `gate-selftest` 67.2 CPU seconds at a load of 28 to 38 on 32
cores. A cut to 80 % of those seconds takes about 100 out. The cuts the story allowed (cases
merged into one test function, a realm shared by merged cases, sweeps by a stride, planted
scripts that read one product) sum to about 93 with every one at its limit. The unit made
none, as its brief said.

The coordinator put the question to the operator with three options:

- A: the gate's step `test` builds and runs at opt-level 1, and the stack tests stay at
  opt-level 0 behind a guard, as the gate's collect step and the lane's collect mode do.
  Estimated from the collect mode (168.5 to 33.3 CPU seconds), not measured: 300 to between
  60 and 100. Cost: a longer build before the clock, and the engine's tests other than the
  stack tests no longer run unoptimized in the gate.
- B: the step `test-collect` leaves the gate (25 % of the seconds). Cost: the lane `slow`
  runs the collect mode only for the modules with full counts and has 33 s of room, so the
  collect tests of the other modules would run nowhere.
- C: no cut; a slow runner of the forge needs a second attempt of `check`.

The operator's decision of 2026-11-27, 12:19 CET: option A, on the condition that debug
assertions and overflow checks stay on in that profile. The unit measures base and head; if
the test steps stay over 80 %, the question comes back to the operator.

The story gets its third body with this record: the split and the guard of the lane for the
tests of `swell::main`, the profile's two checks held by a test, every test in one of the
two runs, and the measurement as before. Two plan critics read it before the unit goes on.
