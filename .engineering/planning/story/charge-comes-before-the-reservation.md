---
format: aep.planning-md/3
id: story:charge-comes-before-the-reservation
kind: story
status: draft
title: A buffer's charge comes before its reservation at three sites
summary: interp/mod.rs:258, env/mod.rs:364 and heap/string.rs:242 reserve before they charge; on a refused charge the grown buffer stays uncharged. Backlog, from adversary pass 1 on unit growth of wave big-1.
revision: 2
---
## Context

Found by the adversary's first pass on the unit `growth` of wave big-1
(`review-result:adversary-growth-pass-1`, findings 4 to 6), on `impl/buffer-growth` at
`3a16be13`; the same code is on the base of that unit, so the finding is older than the wave
and is filed here, in the backlog. No wave takes it before the planned waves are delivered.

`story:buffer-growth-outside-the-charged-list-is-listed` gave every call that reserves room in
a `Vec`, `VecDeque` or `String` of `crates/swell` an `expect` whose reason names the charge of
the buffer. At three sites the reason is true and says that the charge comes after the
reservation:

- `crates/swell/src/interp/mod.rs:258` to `:262`: the operand stack is grown with
  `try_reserve_exact`, and the charge is the next statement.
- `crates/swell/src/env/mod.rs:364`: the slots of an environment are reserved after
  `accounting.check` (`:358`); the charge is made by `store_environment` (`:368`).
- `crates/swell/src/heap/string.rs:242` (`room`): `new_string_from_str` and
  `new_string_from_units` reserve after a `check` (`:257`, `:276`); the charge is made by
  `new_string_record` (`:269`, `:288`). The caller `new_atom_key` charges first (`:374`).

No script runs between the reservation and the charge at any of the three. What is open: when
the charge is refused, or when the reservation succeeds and a step between the two fails, the
grown buffer stays allocated without a charge until its owner is dropped (the operand stack:
until the run ends). `docs/design/engine.md`, "Memory budget", states the rule the engine
follows elsewhere: a buffer is charged to `memory_bytes` before it is made, or not made.

## Change

At each of the three sites the charge is made before the reservation, and credited again when
the reservation fails, as `ChargedVec` does; or the site is shown to release the buffer on the
refusal path, and its reason says so. The unit reads each path from the reservation to the
charge and names, per site, what can fail between them.

## Acceptance

For each of the three sites, a test with `memory_bytes` set so that the charge of the growth
is refused shows the run ends with the engine's out-of-memory outcome and the agent's
accounted bytes after the refusal equal the bytes before the growth was tried (the buffer was
not grown, or was released); the test fails on the tree before the change at each site where
the buffer stays grown. A site where the test cannot be made to fail before the change is
reported with the reason (the reservation cannot succeed when the charge is refused) and its
`expect` reason is reworded to say that. The `expect` reasons at the three sites name the
charge as made before the reservation. `task clippy` exits 0. No test this story adds runs
longer than 5 s; the organization's bounds hold (one test at most 60 s, the gate's test steps
at most 120 s).

## Out of scope

The stack of open values of `JSON.parse` (`builtins/json/parse.rs`), which the correction of
the unit `growth` handles; growth without a reservation (`push`, `extend`, `collect`); other
crates.
