---
format: aep.planning-md/3
id: story:interrupt-read-after-every-call-from-rust
kind: story
status: implemented
title: The interrupt flag is read after every call from Rust into a builtin or host function, not only after Call and New
summary: Getters, setters, conversions, iterator next, comparators, replacers and revivers that throw with the flag set; measured first, then fixed where a catch can store before the next read.
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: conformance/host/requirements.txt
- confidence: cited
  path: conformance/test262/expected.digest
- confidence: cited
  path: crates/swell/src/interp/budget.rs
- confidence: cited
  path: crates/swell/src/object/builtin_function.rs
- confidence: cited
  path: crates/swell/tests/interrupt.rs
- confidence: cited
  path: docs/design/host-interface.md
- confidence: cited
  path: website/data/status.json
revision: 13
transitions:
- {from: "draft", to: "proposed", at: "2026-11-25T11:18:47Z", actor: "human:operator", revision: 11}
- {from: "proposed", to: "active", at: "2026-11-25T11:18:47Z", actor: "human:operator", revision: 12}
- {from: "active", to: "implemented", at: "2026-11-25T12:14:16Z", actor: "human:operator", revision: 13, decided_on: {"recorded":{"test_result":1}}}
---
Found by the unit of `story:interrupt-read-after-a-host-function-throws` (2026-11-25) and
confirmed by the first consumer (surf) at `0.10.0` with a test: a host getter that sets the
interrupt and throws, as the last operation of a script, leaves `evaluate_script` answering
`Throw` with the flag still set; the host's next entry takes the flag. Base: `main` at
`3785379` (release `0.11.0`). Ruling of the coordinator, 2026-11-25: HOST-216 holds for every
path that runs a host function, not only `Call` and `New`.

## Context

Line numbers are of `main` at `3785379`.

- `Agent::check_interrupt_after` (`crates/swell/src/interp/budget.rs`) reads the flag after a
  host function's return or throw at four sites: `Call(n)`, `New(n)` and their spread forms
  (`interp/dispatch.rs`, `interp/spread.rs`).
- A host function reached another way is not followed by that read: a getter or setter through
  `ordinary_get` / `ordinary_set` (`object/ordinary.rs:343`) and `call_value`
  (`ops/objects.rs:148`), `valueOf` and `toString` of a conversion, an iterator's `next`, a
  comparator, a replacer, a reviver. The flag is read only at the next `LoopJump`, call, return
  or step charge, and not at all when the script ends first.
- Every host function, whoever calls it, runs through `Agent::run_host_function`
  (`object/builtin_function.rs:830-876`), which checks the host's answer after `running()`.

## Change

Failing tests first (Acceptance). Then:

- In `run_host_function`, after `running()` and the check of the answer, the flag is read as
  `check_interrupt_after` reads it: after an `Ok` and an `Err(Abrupt::Throw(_))`, a set flag
  ends the run in `Terminated(Interrupted)` and the value is dropped; an
  `Err(Abrupt::Terminated(_))` keeps its reason. The four sites of
  `story:interrupt-read-after-a-host-function-throws` may then drop their own read, or keep it;
  the unit says which and why.
- `docs/design/host-interface.md`, after HOST-217, one requirement `[slice 1]`:
  **HOST-221** (swell:D12) A host function that calls `interrupt()` on its agent's interrupt
  handle and then answers `Err(Abrupt::Throw(v))` or `Ok(v)`, reached as a getter
  (`o.p` with `p` a host accessor), as a setter (`o.p = 1`), as `valueOf` in `+o`, and as the
  `next` of an iterator in `for (x of it) {}`, makes `evaluate_script` answer
  `Terminated(Interrupted)` with `is_set()` false after, both when the read is followed by
  `try`/`catch` code that stores a global and when it is the script's last operation.
- `conformance/host/requirements.txt`: `HOST-221 test swell::interrupt::<name>`;
  `CHANGELOG.md` `## [Unreleased]`: `### Fixed` (the interrupt ends the run after a host
  function reached by a property read or write, a conversion or an iterator) and
  `### Host interface` naming `HOST-221`; `task site-data`.
- Stays: HOST-90, 91, 92, 198, 216, 217; a run without an interrupt is unchanged.

## Acceptance

- On the base the HOST-221 test fails for the getter as the last operation (surf's case:
  `Throw` and the flag still set) and for the getter followed by a `catch` that stores; it
  passes after the change in both collection modes.
- `slot cargo nextest run --locked -p swell --test main -E 'test(/^interrupt::/) |
  test(/^termination::/) | test(/^host_objects::/)'` and the same with
  `--features collect-every-safe-point` exit 0; `slot cargo nextest run --locked -p
  swell-test262 --test main` exits 0.
- The full test262 run (8 parts) leaves `expected.txt` unchanged and writes the digest.
- `slot --timeout 30m task check-runner` exits 0.
