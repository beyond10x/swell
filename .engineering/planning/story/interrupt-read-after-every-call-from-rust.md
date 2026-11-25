---
format: aep.planning-md/3
id: story:interrupt-read-after-every-call-from-rust
kind: story
status: draft
title: The interrupt flag is read after every call from Rust into a builtin or host function, not only after Call and New
summary: Getters, setters, conversions, iterator next, comparators, replacers and revivers that throw with the flag set; measured first, then fixed where a catch can store before the next read.
revision: 1
---
Found by the unit of `story:interrupt-read-after-a-host-function-throws` (2026-11-25); not
introduced by it. Draft: not decomposed, not scoped, not ordered.

## Context

- That story reads the interrupt flag after `Call(n)`, `New(n)` and their spread forms return
  or throw (`Agent::check_interrupt_after`, `crates/swell/src/interp/budget.rs`).
- A builtin or host function reached another way is not followed by a read of the flag,
  neither after a return nor after a throw: a getter or setter called through `ordinary_get`
  and `call_value`, the `valueOf` and `toString` of a conversion, an iterator's `next`, a
  comparator, a replacer, a reviver. The flag is read at the next `LoopJump`, call, return or
  step charge (`Agent::charge_steps`).
- So a host getter that sets the flag and throws hands its value to a script `catch` when the
  `catch` block charges no step before it stores.

## Change (to be decided at decomposition)

- Measure first: which of these paths can store a script-visible value before the next read
  of the flag; a path where `charge_steps` runs first is not a defect.
- For each path that can, read the flag after the call as `check_interrupt_after` does, and a
  requirement beside HOST-216 that names the path and its script.
