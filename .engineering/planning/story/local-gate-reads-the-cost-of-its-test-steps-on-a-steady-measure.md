---
format: aep.planning-md/3
id: story:local-gate-reads-the-cost-of-its-test-steps-on-a-steady-measure
kind: story
status: draft
title: The local gate reads the cost of its test steps on a steady measure
summary: The CPU seconds of the gate's test steps differ by half between two gates of one code on the build node, no entry of the CPU table can be set there, and a green local gate says nothing of the forge's 120 s. Backlog, from the measurement of unit lanesix of wave big-2.
revision: 1
---
## Context

Found while the unit `lanesix` of wave big-2 was measured
(`story:gate-test-steps-hold-the-budget-on-the-slower-runner`, its section "As delivered";
`review-result:lanesix-correction-2-coordinator-read`).

The local gate bounds the wall time of its test steps (120 s) and, on a machine with an
entry in `CPU_BOUNDS_BY_CORES` (`taskfiles/forge.yml`), their CPU seconds. The build node
the gate runs on has 32 cores and no entry; the forge's runner has 4 cores and no entry
either. On the node the test steps take 36 to 72 s of wall time, so a change that would
take the forge over its 120 s passes the local gate: on 2026-11-27 the forge failed `check`
at 147.6 s on a code the node had passed.

The unit tried to bound the cost by the CPU seconds of whole gates, base against head, and
found the measure unsteady on that node. The step `test-collect`, which the unit does not
change, read 64.8 to 127.9 CPU seconds across gates of one code at loads of 10 to 38, at
two levels that the load average does not tell apart; the test steps of one base read
294.0 and 476.5 in two gates run one after the other. Four pairs of base and head read
0.758, 0.782, 0.732 and 1.151. An entry of the CPU table needs a head at 0.64 of the base
(the rule of `AGENTS.md`), so no entry could be set, and with readings that differ by half
on one code an entry would fail gates that changed nothing.

So after the unit the local gate still cannot see the forge's time, and a pair of CPU
readings on the node cannot show a change of less than about a third.

Filed here, in the backlog. No wave takes this story before the planned waves are
delivered.

## Change

To be written when the story is planned. Directions to weigh, each measured before it is
chosen:

- a measure that does not move with the node's other work: the instructions the test
  processes retire (`perf stat` over the step, or per test through the runner wrapper the
  unit used for its lists), with a bound per step recorded beside the code;
- the gate's test steps run in a CPU set of 4 cores on the node, so that their wall time
  is read against the same budget as the forge's;
- a control in every reading: a fixed piece of work run beside the steps (the gate has a
  `gate-time yardstick` line of two small steps), with the bound stated in multiples of
  it, as the forge's acceptance of the unit already is (29.5 yardsticks).

## Acceptance

To be written when the story is planned: two gates on one code read the chosen measure
within a stated share of each other on the build node at a load under 16 and at a load
over 32; a planted slower test step (twice the work in the step `test`) fails the local
gate on the node; the forge's `check` on the same plant fails its budget or is shown to
hold it with the reading.
