---
format: aep.planning-md/3
id: story:a-limit-of-an-integer-width-has-a-test-or-a-stated-reason
kind: story
status: draft
title: A limit of the width of an integer type has a test that reaches it, or a stated reason
summary: No u32 or u64 limit of the engine has a test that reaches it (arena max_slots, generation retirement, agent keys, realm counter, the entry table's positions and cursor places). Backlog, from the adversary's passes on unit keyed of wave big-2.
revision: 2
---
## Context

Found by the adversary's two passes on the unit `keyed` of wave big-2
(`review-result:adversary-keyed-pass-1`, finding 4, and
`review-result:adversary-keyed-pass-2`, finding 7). `AGENTS.md`, "Rules", says: "A new limit
gets a test that reaches it." The unit added two limits of the width of a `u32` (the position
of an entry and the place of a cursor in `crates/swell/src/object/keyed_entries.rs`) and
could not write a test that reaches either: 2^32 entries cannot be made in a test, and the
engine has no form by which a test lowers such a limit. The adversary then looked for a limit
of this kind with a test anywhere in the engine and found none. The sites it names:

- `max_slots` of `crates/swell/src/heap/arena.rs` (line 64): the count of slots of an arena.
- The retirement of a slot whose generation is at its last value (`heap/arena.rs` lines 374
  to 384, `heap/roots.rs` line 196).
- The keys of an agent (`crates/swell/src/host/agent.rs` lines 69 to 78).
- The realm counter (`host/agent.rs` line 440), a `u64`.
- The two limits of the entry table named above.

The finding is older than the wave for every site but the last, so it is filed here, in the
backlog, once for the engine. No wave takes it before the planned waves are delivered.

## Change

Not designed. Two questions a story that takes this answers first:

1. Which form lets a test reach a limit of the width of an integer type: a limit the test
   sets low (a field read where the constant is read today, with the type's bound as the
   default), or a counter the test sets near its end (the entry table's generation counter
   has such a test, since the test module writes the field). The form must add no branch to
   a hot path that the benchmarks then measure.
2. Whether the rule of `AGENTS.md` keeps its wording or names the limits a test cannot reach
   and what stands in for the test (the arithmetic, written at the site).

## Acceptance

Each site listed in Context has a test that reaches its limit and reads the outcome the host
sees, or a sentence at the site and in `AGENTS.md` that states why none can, and
`task check-runner` passes.
