---
format: aep.planning-md/3
id: story:operand-stack-charges-its-growth-before-it-reserves
kind: story
status: draft
title: The operand stack charges its growth before it reserves it
summary: Agent::push_entry reserves the doubled operand stack before it charges it, so a growth the memory budget refuses leaves the reserved capacity uncharged (about 1 MiB at 65,536 entries) until the frame unwinds. The charge comes first and a refusal leaves the capacity as it was. Backlog, from the adversary's pass on the unit lanefive of wave big-2.
revision: 1
---
## Context

Found by the adversary's pass on the unit `lanefive` of wave big-2
(`review-result:adversary-lanefive-pass-1`, finding 2); older than that unit, the same on its
base. `Agent::push_entry` (`crates/swell/src/interp/mod.rs`) grows the running frame's operand
stack to twice its capacity: it calls `try_reserve_exact` first and charges the added entries
to the memory budget second. When the budget refuses the charge, the run ends with
`Terminated(MemoryBudget)`, and the vector keeps the capacity it reserved, uncharged, until
the frame unwinds. The adversary's host function logged it at each power of two from 8 to
65,536 entries, for example `short cap 8 growth 128: Terminated(MemoryBudget) len+0 cap 16
used+0`: the capacity is 16 and nothing is charged. At 65,536 entries the reserve that is
not charged is about 1 MiB.

The engine's rule is that a buffer's growth is charged before it grows (`AGENTS.md`, the
lint of `growth`; the `#[expect]` on this call says "charged by the next statement"). The
window here is short, because the refusal ends the run and the frame's unwinding frees the
vector. It is still room the host's budget did not allow.

## Change

The operand stack charges a growth before it reserves it: the charge first; on a refusal the
run ends and the vector's capacity is what it was; on a failed reservation after a charge
the charge is given back and the run ends. The reason text of the `#[expect]` states the new
order. The unit that grows the stack by another path
(`story:property-escape-cases-spend-less-in-the-interpreter-loop`, `push_operand_grown`)
gets the same order.

## Acceptance

- A test holds it at each power of two from 8 to 65,536 entries: with a memory budget one
  byte short of the growth the run ends in `Terminated(MemoryBudget)` and the capacity of
  the frame's operand stack is unchanged, read where the adversary's host function read it;
  with the exact budget the push succeeds and the bytes charged equal the entries added
  times the size of an entry.
- The test is red on the code before the change.
- No step count, heap reading or stack bound of the tree's tests moves; the frame of the run
  loop is not larger (`slot --timeout 30m task test-stack-profiles`).
- The gate's test steps stay within 120 s and no test runs over 60 s.

## Out of scope

- Other buffers that reserve before they charge: the lint of `growth` lists the points of
  reservation; a read of that list is its own story.
