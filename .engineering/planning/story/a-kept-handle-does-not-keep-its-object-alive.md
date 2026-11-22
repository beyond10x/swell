---
format: aep.planning-md/3
id: story:a-kept-handle-does-not-keep-its-object-alive
kind: story
status: active
title: 'A handle the host keeps without a root is a weak reference: it reads its object while the object lives and answers StaleHandle once a collection freed it'
summary: The generation check of the arenas already makes a kept ObjectRef weak; the design states it as a guarantee of decision 4, a requirement holds it with a test, and a committed arena test shows the check catches a reused generation. No new type.
relations:
- depends_on: story:arena-capacity-and-collector-workspace-on-the-ledger
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T06:48:24Z", actor: "human:operator", revision: 2, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-22T06:48:24Z", actor: "human:operator", revision: 3, executor: "agent:coordinator"}
---
Waits for: the budget batch's `story:arena-capacity-and-collector-workspace-on-the-ledger`
(it rewrites `Arena::allocate` and `free_index` in `crates/swell/src/heap/arena.rs`, whose
module comment and tests this story writes). Batch H2. Requirement HOST-178. Consumer: the
browser, requirement 4 of its list. Decision D4 (taken: option A, with its condition: the
browser withdrew its need and keeps wrappers strong per document; this story writes the promise
down and tests it).

## Context

Line numbers are of `wave/budget-1` at `ed56b05`.

- A handle is the agent's key, a slot index and the slot's generation. Freeing a record moves its
  slot's generation, and a slot whose generation reaches `u32::MAX` is retired, so a generation
  is never used twice (`crates/swell/src/heap/arena.rs:1-9`, the move at `:136-138`); every
  access compares key and generation (`get`, `:108`). The collector does not move a record.
- A handle the host passes in is checked before anything else, and a freed one answers
  `Err(HostError::StaleHandle)`, changes nothing and leaves the agent `Alive` (HOST-30,
  `docs/design/host-interface.md:421`, held by `crates/swell/tests/misuse.rs:657`).
- So a host that keeps an `ObjectRef` past its scope, with no `Root` and listed by no `trace`,
  holds a reference that does not keep the object alive and reads as gone once it is freed.
  The design does not promise it: decision 4 says a handed-out value "is valid for the scope it
  was handed in" and that a handle kept past it "is found by the generation check, never by
  reading the wrong object" (`docs/design/host-interface.md:343-352`); nothing says the kept
  handle keeps working while the object lives, and no test holds it.
- Two places where a stale handle from the host faults the agent instead: a handle a host
  function returns (HOST-160, `:559`) and one a `trace` lists (HOST-110, `:1273`). A host that
  keeps weak handles checks one (`cx.brand(object)`, `crates/swell/src/host/mod.rs:472`) before
  it returns it.
- The arena's unit tests: `stale_generation_faults_the_agent` (`arena.rs:201`).

## Change

The behaviour exists, so the requirement's tests are guards; the committed red case is an
arena unit test (point 3). Then:

1. **The guarantee, in the design** (decision 4). "A handle the host keeps past its scope, with
   no `Root` and listed by no `trace`, does not keep its record alive. While the record lives,
   every call with the handle answers as it would inside the scope; once a collection freed it,
   every call answers `Err(StaleHandle)`, never another record, because a generation is never
   used twice. A host that keeps such a handle checks it before it returns it to the engine
   (HOST-160) and never lists it in a `trace` (HOST-110)."
2. **No new type.** No `WeakRoot`: the kept handle is the weak reference (decision D4). The rule
   that an arena does not move a record and never reuses a generation is stated in the module
   comment of `crates/swell/src/heap/arena.rs` with the requirement's number, HOST-178.
3. **A committed red case.** `arena.rs` gains, under `#[cfg(test)]`, a switch of the arena that
   makes `free_index` leave the generation as it was (the defect a kept handle must never meet),
   and a helper the tests share, `a_kept_handle_reads_stale(arena, handle) -> bool`. Two unit
   tests: `a_kept_handle_reads_stale_after_its_slot_is_reused` (the switch off: allocate, keep
   the handle, free, allocate 1,000 records into the freed slots; the helper answers true, a
   guard) and `the_kept_handle_check_catches_a_reused_generation` (the switch on, the same steps:
   the helper answers false, so the check of the guard and of HOST-178 is shown to fail on the
   defect, from the tree, on every run).

Design preconditions (the design author writes them before the unit starts): the sentence of
point 1 in `docs/design/host-interface.md` decision 4, after "never by reading the wrong
object." (`:352`); the line of HOST-178 below, with its pending line in
`conformance/host/requirements.txt`.

New requirement:

- **HOST-178** (swell:D4) A host object made in one `with_cx` and kept by the host as an
  `ObjectRef` with no root, stored by a script in a global, answers its brand in a later
  `with_cx` and after `collect`; once the script deleted the global and `collect` ran, `brand` of
  the kept handle answers `Err(HostError::StaleHandle)`, also after 1,000 new host objects were
  made in the freed slots, and `state()` reads `Alive`. [slice 6]

Writes: `crates/swell/tests/host_objects.rs`, `crates/swell/tests/support/host.rs` (one field of
`TestHost`, `kept: Vec<Option<ObjectRef>>`, the host's map from a node number to its kept
handle; no other line), `crates/swell/src/heap/arena.rs` (the module comment, the
`#[cfg(test)]` switch and the two unit tests; no line of the code the build ships), and in
`conformance/host/requirements.txt` the line of HOST-178 only. Not written: any other code of the
engine.

## Acceptance

In `crates/swell/tests/host_objects.rs`, each in both collection modes:

- `host_178_a_kept_handle_is_weak`: as HOST-178, the kept handles in `TestHost::kept`; in the
  collect mode the object is freed at the first safe point after the global is deleted. A guard
  (passes on the base).
- `a_kept_handle_of_a_live_object_is_the_same_object` (a guard): the script reads the object back
  from its global after the host's `with_cx` returned it through a host function (`o === g`)
  answers `true`; the host function checks the kept handle with `brand` first and makes a new
  object when it answers `Err(StaleHandle)`.

In the lib test binary, `crates/swell/src/heap/arena.rs`:

- `the_kept_handle_check_catches_a_reused_generation`: as point 3. Today it does not compile (no
  switch, no helper); after, it passes and shows the detector red on the defect.
- `a_kept_handle_reads_stale_after_its_slot_is_reused`: as point 3, a guard.

Commands, each one job of the build pool (one slot): `slot cargo nextest run --locked -p swell
--profile default --test host_objects`, the same with `--features collect-every-safe-point`,
`slot cargo nextest run --locked -p swell --profile default --lib` in both modes, `slot cargo
clippy --workspace --all-targets --locked -- -D warnings`, and last, once, in the background,
`slot timeout 1800 task host-requirements-check` (it builds every test target of the workspace). No test262
run: no code the build ships changes. Never `cargo test -p swell` whole, `slot task
test-collect`, `slot task test` or `slot task check`.

Rules this unit keeps. The failing test comes first: `the_kept_handle_check_catches_a_reused_generation`
is written and seen red (it does not compile) before the switch exists; the other three are
guards, named so. A test is killed at 60 s and the gate's test steps fail over 120 s; each test
runs in a process of its own under cargo-nextest, so nothing is shared through a `static`
between tests (the switch is a field of the arena under test). Each new test is timed, and one
over 5 s is named in the report with its time. A job of the build pool asks for one slot; one over 10 minutes is killed, so `host-requirements-check` on a cold tree and the full test262 run are named `slot timeout 1800 <command>`.
Determinism: the same calls free the same objects at the same collection. Nothing is added to
the frame of `Agent::execute`; `STACK_BYTES_PER_REENTRY` and the stack tests are unchanged. No
third-party crate, no unsafe code.

## Report

The tests with their times, and this line for the changelog's "Host interface" heading: "A
handle the host keeps without a root is now a stated weak reference (`HOST-178`): it reads its
object while the object lives and answers `StaleHandle` after a collection freed it."
