---
format: aep.planning-md/3
id: story:the-host-charges-its-own-bytes-and-steps
kind: story
status: active
title: The host charges bytes it holds for the agent to the memory budget, resizes that charge in place, gives it back by dropping it, and charges steps for its own work
summary: Cx and HookCx gain charge_host_bytes, which answers a HostBytes the host keeps with what it charged for, and resize_host_bytes, which changes that charge in place; dropping a HostBytes queues the credit the agent applies after the sweep or at the next charge, as the engine's own Grown does, and Agent::credit_host_bytes credits one at once. Cx gains charge_steps over Agent::charge_steps. A host that charges without end ends the run in Terminated(MemoryBudget) or Terminated(StepBudget).
relations:
- depends_on: story:arena-capacity-and-collector-workspace-on-the-ledger
- depends_on: story:values-read-from-a-root-live-for-the-scope
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T06:48:25Z", actor: "human:operator", revision: 2, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-22T06:48:25Z", actor: "human:operator", revision: 3, executor: "agent:coordinator"}
---
Waits for: the budget batch's `story:arena-capacity-and-collector-workspace-on-the-ledger` (it
adds `live_bytes()` beside `heap_bytes()`, `crates/swell/src/host/agent.rs:210-214`, and moves
`Heap::collected` into `collect_now`, `:590-596`, where this story's credit goes) and
`story:values-read-from-a-root-live-for-the-scope` (both write the `scoped_methods!` macro of
`crates/swell/src/host/mod.rs`, other lines, and `crates/swell/tests/support/host.rs`, other
fields). Stories that wait for this one: `story:an-error-object-is-read-without-running-code`
(both write the faulted-agent helper of `crates/swell/tests/faults.rs`) and
`story:host-functions-and-host-objects-are-charged-within-stated-ceilings` (its design sentence
names `cx.charge_steps`). Batch H2. Requirements HOST-175, HOST-176, HOST-177. Consumer: the
browser, requirement 3 of its list, and the part of requirement 2 that is the host's own work.
Decision D3 (taken: option A, with its conditions, Change 1 to 4, and the addition of 07:25 CET,
Change 3).

## Context

Line numbers are of `wave/budget-1` at `ed56b05`.

- The memory budget has one accounting: `Accounting::charge`, `credit`, `check`
  (`crates/swell/src/heap/accounting.rs:30-59`), reached as `self.heap.accounting`.
  `Accounting::credit` saturates at 0 (`:51-53`): a second credit of the same bytes would take
  another charge's room silently, so no credit of this story relies on it.
- What the host can charge today: the size of its host data's `Box` when it makes a host object
  (`size_of_val`, `crates/swell/src/host/mod.rs:238`). Memory the data owns behind that box (a
  `Vec`, a tree node, text) and the host's DOM tree beside the agent are charged nowhere.
- Where a host learns that an object died: the `Drop` of its host data, which the sweep runs
  (HOST-111, `docs/design/host-interface.md:1275`), with no agent reachable and no host code
  called by the engine (HOST-129, `:1431`). The engine's own bytes that die in a sweep take the
  same road: a record keeps what its property map was charged in `Grown`, whose `Drop` adds the
  bytes to the agent's queue, an `Rc<Cell<u64>>` (`crates/swell/src/object/mod.rs:45-70`,
  `Agent::credit`, `crates/swell/src/host/agent.rs:93`), and `collect_now` credits the queue
  after the sweep (`agent.rs:590-596`, the credit at `:592-593`).
- Steps: `Agent::charge_steps` is the one function that charges builtins' steps; it reads the
  interrupt flag first, and a charge past `steps_per_entry` ends the run in
  `Terminated(StepBudget)` and spends nothing (`agent.rs:559-571`). It is `pub(crate)`. A host
  function costs one step at its call (`enter_function`,
  `crates/swell/src/object/builtin_function.rs:247-255`) and nothing for its own work, so a host
  function that walks a tree of n nodes (`querySelectorAll`) is one step whatever n is.
- A termination inside a host function is sticky (decision 10): every later `Cx` call answers it
  (`Agent::running`, `agent.rs:531-537`). A `MemoryBudget` termination makes the next safe point
  collect (`agent.rs:573-583`). A faulted agent answers every entry point and read with
  `Err(HostError::Faulted)` and runs no closure of the host (HOST-48), which the helper
  `answers_faulted_everywhere` asserts (`crates/swell/tests/faults.rs:30-67`, called once, by
  HOST-71's test at `:115`).
- `TestHost` (`crates/swell/tests/support/host.rs:18-26`) has `calls`, `jobs`, `loads`, `now`
  and `count`; `host_mut` is a scoped method, so the setup closure's `HookCx` reaches it
  (`host/mod.rs:423`).

## Change

Failing tests first (Acceptance). Then:

1. **Bytes, charged** (decision D3, option A; its first condition). In `scoped_methods!`, so on
   `Cx` and `HookCx`:

   ```rust
   /// Charge `bytes` the host holds for the agent to its memory budget; the answer holds the
   /// charge, which goes back to the budget when the answer is dropped.
   pub fn charge_host_bytes(&mut self, bytes: u64) -> Completion<HostBytes>;
   ```

   It checks the sticky termination, credits the queue of point 2, then charges through
   `Accounting::charge`; over the budget nothing is charged and the run ends in
   `Terminated(MemoryBudget)` through `Agent::terminate`, as the engine's own charge does. It is
   not a safe point: no collection runs in it. Inside the `Cx` call in which the agent faulted it
   answers `Err(Abrupt::Terminated(EngineFault))`; no later call reaches it (`with_cx` and
   `create_realm` of a faulted agent run no closure).
2. **Bytes, given back** (the credit route that needs no agent, in the pattern of `Grown`).
   `HostBytes`, a new public type in `crates/swell/src/host/host_bytes.rs` (re-exported by the
   glob of `host/mod.rs`), holds the bytes, the agent's key and a clone of a second queue of the
   agent, `Agent::host_credit: Rc<Cell<u64>>`. It has no public method, is not `Clone`, and its
   `Drop` adds its bytes to that queue: it touches its own cell only, with a saturating add, so
   it cannot panic, also when its agent was dropped (the cell outlives it and nobody reads it) or
   faulted. The host keeps it where it keeps what it charged for: in the host data of the host
   object that owns the node, so the sweep that frees the object drops it, or in its own state.
   `collect_now` credits the queue after the sweep beside `Agent::credit`, before the report reads
   `heap_bytes_after` and before `Heap::collected` sets the threshold; and `charge_host_bytes` and
   a growing `resize_host_bytes` credit it before they charge, so a host that dropped charges
   outside a collection, or inside a host function, gets their room back at its next charge.
   `Agent::credit_host_bytes(&mut self, charge: HostBytes) -> Result<u64, HostError>` takes the
   handle by value, credits its bytes at once and sets them to 0 before the handle is dropped,
   so its `Drop` adds nothing: one credit per charge, with no reliance on the saturation of
   `Accounting::credit`. It needs no realm and no scope (a dropped document) and answers the
   bytes. A leaked `HostBytes` (`mem::forget`, or kept for good) stays charged for the agent's
   life.
3. **A charge resized in place** (the addition to D3 of 07:25 CET). In `scoped_methods!`:

   ```rust
   /// Change the bytes `charge` holds to `bytes`: growing charges the difference first,
   /// shrinking gives it back at once.
   pub fn resize_host_bytes(&mut self, charge: &mut HostBytes, bytes: u64)
       -> Result<Completion<()>, HostError>;
   ```

   Growing credits the queue, then charges the difference before the host grows its storage;
   over the budget nothing is charged, the run ends in `Terminated(MemoryBudget)` as any charge
   does, and the handle keeps its old bytes. Shrinking credits the difference through
   `Accounting::credit` and the handle holds the new bytes; an equal size does nothing. A handle
   of another agent answers `Err(HostError::ForeignHandle)` and changes nothing. The method is on
   the scope, not on the handle, because growing needs the agent's ledger, which the handle does
   not hold. Replacing a handle by a new one stays possible and charges old plus new until the old
   one is dropped.
4. **What a credit cannot do** (D3's second condition). A credit is always the bytes of one
   charge, as resized, given back once, to the agent that charged them (the `HostBytes` holds that
   agent's queue): no credit can pass what the host charged, and nothing wraps. The one misuse,
   `Agent::credit_host_bytes` of a `HostBytes` of another agent, answers
   `Err(HostError::ForeignHandle)` and credits nothing to this agent; the handle, dropped there,
   gives its bytes back to its own agent's queue. A faulted agent answers
   `Err(HostError::Faulted)`; the handle, dropped there, adds to its own cell.
5. **Steps.** On `Cx` only:

   ```rust
   /// Charge `steps` of the host's own work to the running outermost entry.
   pub fn charge_steps(&mut self, steps: u64) -> Completion<()>;
   ```

   It is `Agent::charge_steps` after the sticky termination: it reads the interrupt flag, and
   steps past the budget are not spent and end the run in `Terminated(StepBudget)`. Inside
   `with_cx`, the running entry is that of the last `Cx` call that began one (decision 2);
   inside a host function, the outermost entry's, as every nested entry spends.
6. **Readings.** `heap_bytes()` and `live_bytes()` both count the host's bytes while their
   `HostBytes` lives, and both fall by them once it is credited; no read of the host's total
   is added (the host knows what it holds).
7. **Stays as it is.** The one `Accounting` and the one `charge_steps`: no second budget and no
   second counter of steps. The charge of a host object's box at creation. No host code runs in
   a collection (the queue is a `Cell`). Determinism: a charge, a resize, a drop and a credit are
   inputs the host gives, like its other calls.

Design preconditions (the design author writes them before the unit starts):
`docs/design/host-interface.md` section 2, the `memory_bytes` row (`:154`), "what it counts":
"the agent's heap over its life, and the bytes the host charged with `charge_host_bytes` while
it holds their `HostBytes`"; section 2, "Two readings of the heap" (`:169-181`), the amendment:
"Both readings count the bytes the host charged with `charge_host_bytes` while it holds their
`HostBytes`; a dropped `HostBytes` is credited after the next sweep or at the host's next
charge, or at once by `Agent::credit_host_bytes`, which empties it."; section 14 gains the
methods and `HostBytes` with the sentences "A host function charges the steps of its own work
with `cx.charge_steps`, as a builtin does, before the work; and the bytes it holds for the agent
outside the engine's heap with `charge_host_bytes` before it allocates them, keeping the answer
with them: dropping it gives the bytes back, in a sweep too, with no agent at hand; a
`HostBytes` kept for good keeps its bytes charged for the agent's life." and "For bytes whose
size changes while their holder lives (a text node's data, an attribute list),
`resize_host_bytes` changes the charge in place: growing charges the difference before the host
grows its storage and, past the budget, ends the entry as any charge does, the handle keeping its
old charge; shrinking gives the difference back at once. Replacing the handle by a new one
charges both until the old one is dropped." And the lines of HOST-175 to HOST-177 below, with
their pending lines in `conformance/host/requirements.txt`.

New requirements:

- **HOST-175** (swell:D2) A host function that charges 64 bytes with `cx.charge_host_bytes`
  and keeps the answer with a 64-byte buffer in the host on each call, run by the script
  `while (true) make();` with `memory_bytes` M and a step budget large enough, ends in
  `Terminated(MemoryBudget)`, and `heap_bytes()` then reads at most M; after the host drops
  the buffers with their `HostBytes`, the script `1` answers `Normal(1)`. [slice 6]
- **HOST-176** (swell:D2) A `HostBytes` gives back exactly what it charged, once. With L0 the
  `live_bytes()` before a charge of 100, a host object made after the charge with that
  `HostBytes` in its host data and no root, and L1 and H1 the two readings once the object
  exists: after `collect()` frees the object, `live_bytes()` reads exactly L0 and `heap_bytes()`
  reads exactly H1 − 100 − `size_of_val` of the object's host data, and a second `collect()`
  changes neither. `Agent::credit_host_bytes` of a second charge of 100 answers `Ok(100)`, both
  readings fall by exactly 100 at the call, and a later `collect()` changes neither.
  `Agent::credit_host_bytes` of a `HostBytes` of another agent answers
  `Err(HostError::ForeignHandle)` with this agent's readings unchanged, and the other agent's
  `live_bytes()` falls by its bytes at its next `collect()`. [slice 6]
- **HOST-177** (swell:D2) A host function that calls `cx.charge_steps(1_000)` on each call, run
  by the script `while (true) walk();` with `steps_per_entry` N, ends in
  `Terminated(StepBudget)` with `last_entry_steps()` at most N; one that calls
  `cx.charge_steps(N + 1)` ends the entry at its first call with `last_entry_steps()` below N;
  an interrupt set before the charge ends it in `Terminated(Interrupted)`. [slice 6]

Writes: `crates/swell/src/host/host_bytes.rs` (new: `HostBytes` and its `Drop`),
`crates/swell/src/host/agent.rs` (the field `host_credit`, its credit in `collect_now`,
`Agent::credit_host_bytes`), `crates/swell/src/host/mod.rs` (the `mod` and `pub use` lines,
`charge_host_bytes` and `resize_host_bytes` in `scoped_methods!`, `Cx::charge_steps`),
`crates/swell/tests/budgets.rs`, `crates/swell/tests/faults.rs` (the helper
`answers_faulted_everywhere`, lines 30-67: one parameter, a `HostBytes`, and its assertion;
HOST-71's test at `:73-124`: the charge before the fault, its argument),
`crates/swell/tests/support/host.rs` (one field of `TestHost`, `charges: Vec<HostBytes>`, the
charges a test's host functions keep; no other line), and in
`conformance/host/requirements.txt` the lines of HOST-175 to HOST-177 only. Not written:
`crates/swell/src/heap/accounting.rs`, the collector, `crates/swell/src/object/mod.rs`.

## Acceptance

In `crates/swell/tests/budgets.rs`, each in both collection modes:

- `host_175_host_bytes_without_end_end_in_memory_budget`: as HOST-175, M = `heap_bytes()` after
  the realm + 64 KiB. Today it does not compile; with the method stubbed to charge nothing the
  script runs to the step budget.
- `host_176_a_host_charge_gives_back_what_it_charged`: as HOST-176; the charge is made in
  `with_cx`, the host object made in the same `with_cx` with the `HostBytes` in its host data
  and no root; then the host drops nothing itself, and `collect()` frees the object. The foreign
  handle comes from a second agent with its own realm. Today it does not compile; a unit that
  credits twice fails the readings after the second `collect()`.
- `a_dropped_charge_gives_its_room_to_the_next_charge`: in one `with_cx`, with `memory_bytes`
  room R left, a charge of R answers a `HostBytes`, which the closure drops; a second charge of R
  in the same closure, with no other call between and so no collection, answers `Ok`. A unit
  that does not credit the queue before it charges ends the second in
  `Terminated(MemoryBudget)`. Today it does not compile.
- `a_charge_dropped_inside_a_host_function_gives_its_room_back`: with the script `f()` parsed,
  R the room left (`memory_bytes` set so that R is 1 MiB) and a `HostBytes` of R − 4,096 bytes in
  `TestHost::charges` (4,096 left for the call's own bytes), the host function `f` takes it out
  through `cx.host_mut()`, drops it, and charges R − 4,096 again: the script answers `Normal`. A
  unit that does not credit the queue before it charges ends it in `Terminated(MemoryBudget)`.
  Today it does not compile.
- `a_charge_dropped_after_its_agent_is_harmless`: a `HostBytes` taken out of the host, its agent
  then dropped, and the handle dropped after it: no panic. Today it does not compile.
- `a_resized_charge_fits_where_a_replacement_does_not` (decision D3, 07:25 CET): with room R
  left, a charge of R/2 + 1; `resize_host_bytes` to R/2 and then back to R/2 + 1 answers
  `Ok(Ok(()))` each, `live_bytes()` following the size, and the entry goes on (the script `1`
  answers `Normal(1)` after it); a resize to R + 1 ends in `Terminated(MemoryBudget)` with the
  handle's charge, and `live_bytes()`, unchanged; in a fresh agent, a second charge of R/2 + 1
  while the first is held, the replacement, ends in `Terminated(MemoryBudget)`; a resize with a
  handle of another agent answers `Err(HostError::ForeignHandle)`. Today it does not compile.
- `host_177_host_steps_end_in_step_budget`: as HOST-177. Today it does not compile.
- `host_charges_from_a_hook_count_as_from_cx`: two fresh agents with the same limits each make
  one realm with the same setup closure, which in the first agent also charges 1,000 bytes with
  the `HookCx` and keeps the `HostBytes` in `TestHost::charges` through `cx.host_mut()`, outside
  the heap, so no box differs: `heap_bytes()` and `live_bytes()` of the first agent are exactly
  1,000 above the second's; a `HookCx` charge past `memory_bytes` ends `create_realm` in
  `Terminated(MemoryBudget)`. Today it does not compile.
- `host_charges_inside_with_cx_are_sticky`: in `with_cx` with `memory_bytes` left 10, a charge
  of 11 answers `Err(Abrupt::Terminated(MemoryBudget))`, and a following `cx.new_string("a")`
  answers the same; the next `with_cx` runs afresh. Today it does not compile.

In `crates/swell/tests/faults.rs`, with the feature `fault-injection` as today: HOST-71's test
charges 8 bytes in a `with_cx` before it injects the fault and passes the `HostBytes` to
`answers_faulted_everywhere`, which asserts that `agent.credit_host_bytes(charge)` answers
`Err(HostError::Faulted)` and that the handle, dropped in the call, does not panic. Today it
does not compile.

A leaked `HostBytes` has no test: it stays charged for the agent's life (Change 2).

Commands, each one job of the build pool (one slot): `slot cargo nextest run --locked -p swell
--profile default --test <b>` for each `<b>` of `budgets`, `host_objects` and `collect`, the same
with `--features collect-every-safe-point`, `slot cargo nextest run --locked -p swell --profile
default --features fault-injection --test faults`, `slot cargo nextest run --locked -p swell
--profile default --lib` in both modes, `slot cargo clippy --workspace --all-targets --locked --
-D warnings`, and last, once, in the background, `slot timeout 1800 task
host-requirements-check` (it builds every test target of the workspace). No test262 run: no
builtin and no instruction changes, and no case can call the new methods. Never `cargo test -p
swell` whole, `slot task test-collect`, `slot task test` or `slot task check`.

Rules this unit keeps. The failing test comes first: each test of the acceptance is written and
seen red before the change. A test is killed at 60 s and the gate's test steps fail over 120 s;
each test runs in a process of its own under cargo-nextest, so no fixture and no counter is
shared through a `static` between tests (a host's charges live in its `TestHost` value). Each
new test is timed, and one over 5 s is named in the report with its time. A job of the build
pool asks for one slot; one over 10 minutes is killed, so `host-requirements-check` on a cold
tree and the full test262 run are named `slot timeout 1800 <command>`. Determinism: the same
script under the same budgets and the same host charges ends the same way at the same count on
every run. Nothing is added to the frame of `Agent::execute`; `STACK_BYTES_PER_REENTRY` and the
stack tests are unchanged. No third-party crate, no unsafe code.

## Report

The new tests with their times, and this line for the changelog's "Host interface" heading:
"`charge_host_bytes` (new, on `Cx` and `HookCx`) charges bytes the host holds for the agent to
`memory_bytes` and answers a `HostBytes` (new), whose drop gives them back, in a collection or at
the next charge; `resize_host_bytes` (new, on `Cx` and `HookCx`) changes that charge in place;
`credit_host_bytes` (new, on `Agent`) gives one back at once; `charge_steps` (new, on `Cx`)
charges the steps of a host function's own work (`HOST-175`, `HOST-176`, `HOST-177`)."
