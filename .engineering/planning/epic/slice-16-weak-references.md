---
format: aep.planning-md/3
id: epic:slice-16-weak-references
kind: epic
status: draft
title: 'Slice 16, weak-references: WeakRef and FinalizationRegistry'
summary: 'WeakRef, FinalizationRegistry and $262.gc on a deterministic collector: 82 cases and 164 runs.'
relations:
- depends_on: epic:slice-15-iterator-helpers-and-resources
revision: 1
---
## Context

Slice 16 of 19, `weak-references` (`docs/design/engine.md`, section 12). It follows slices 1 to 15 in
the design's order of dependencies; the design states no reason for the position. It needs the
collector of decision 4 and the job queue of slice 10 for cleanup jobs. It builds `WeakRef`,
`FinalizationRegistry` and `$262.gc`. In scope because "the collector is precise and deterministic
(decision 4), and the runner provides `$262.gc`" (decision 1). Governing decisions
(`docs/design/engine.md`, decision N): 4 (collector, weak references, determinism), 9 (jobs and
hooks), 11 (the runner), 13.

## Outcome

- `WeakRef` and `FinalizationRegistry`.
- Weak references cleared after marking (9.9); finalization callbacks become cleanup jobs through
  `HostEnqueueFinalizationRegistryCleanupJob` (decision 4); the kept-objects list of 9.10 as a root.
- `$262.gc` in the runner, and the cases flagged `host-gc-required` (the `feature` line below).
- Determinism: collection triggers count bytes, never time, so for the same inputs and budgets the
  same objects are collected at the same points, and `WeakRef` and `FinalizationRegistry` results
  are deterministic (decision 4).
- `collect` as a host entry point (decision 9).

### Open, to be settled by this epic's first story

- Whether the kept-objects list and the clearing of weak references after marking already exist
  from slice 9 (`WeakMap`, `WeakSet`) or from the collector of slice 1; section 12 gives them to
  this slice.
- The scheduling of cleanup jobs relative to the promise job queue: decision 9 says generic and
  timeout jobs go to the host; it does not say the same for finalization callbacks beyond the hook.

## Counts

- Slice 16 (`weak-references`): 82 cases, 164 runs (section 12 table).
- Directories the design names for it (paths under `test/`, cases): `FinalizationRegistry` 47,
  `WeakRef` 29. Inventory, in all slices: `WeakRef` 37 cases, `FinalizationRegistry` 49,
  `host-gc-required` 15.
- Lines of the `swell-slices` block that assign cases to this slice:
  - `dir test/built-ins/WeakRef/ 16`
  - `dir test/built-ins/FinalizationRegistry/ 16`
  - `feature WeakRef 16`
  - `feature FinalizationRegistry 16`
  - `feature host-gc-required 16`
  - no `flag` line.

Source: `docs/design/engine.md`, section 12 and the output under 'How the counts were made'; a count is measured again on the pinned test262 checkout before a story's acceptance uses it (`AGENTS.md`, 'How the work is run').

## Out of scope

- `WeakMap` and `WeakSet`: slice 9.
- A collector other than the precise, stop-the-world mark and sweep, or one triggered by time:
  not in the design; triggers count bytes, never time (decision 4).
- The host's own weak references to DOM objects: host data is traced, not rooted (decision 9).
- `Temporal` (slice 20), ECMA-402 (slice 21), stage 3 features, concurrent agents.

## Constraints

- Nothing third-party in what ships; `#![forbid(unsafe_code)]`: weak references are handles checked
  by generation, not pointers (`AGENTS.md`, "Rules"; decision 4).
- No ambient authority: no clock or randomness in collection triggers (`AGENTS.md`, "Rules";
  decision 4).
- No panics, no `unwrap`, `expect` or unchecked indexing (`AGENTS.md`, "Rules").
- Every loop (marking, sweeping, cleanup) is charged to the step budget and every allocation to
  the memory budget; a new limit gets a test that reaches it (`AGENTS.md`, "Rules").
- A builtin's module and comments cite its clause (`AGENTS.md`, "Rules").

## Acceptance

`slot task check` exits 0 on `main`, and in that run:

1. The conformance runner's line for slice 16 reads 82 cases and 164 runs that exist, and every run of the slice passes; no case of the slice is counted as failed or as ended by a budget.
2. No pass count of an earlier slice or of any directory line in `conformance/test262/expected.txt` is lower than before the slice.
3. `conformance/test262/expected.txt` is committed with the lines of that run, and `AGENTS.md` and `README.md` carry the gate's numbers.
4. A case of this slice that is found to need a later slice is moved by a rule line in the `swell-slices` block of the design with its reason, in the same change that moves the counts of both slices; none is skipped silently.
5. A script that uses `WeakRef`, `FinalizationRegistry` and `$262.gc`, run twice with the same inputs and budgets, prints the same bytes: the same objects are collected at the same points (decision 4).
6. A stale handle to a collected object ends the run with `Terminated(EngineFault)`, not with another object and not with a panic (decisions 4 and 13).
