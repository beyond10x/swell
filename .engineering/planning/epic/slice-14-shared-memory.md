---
format: aep.planning-md/3
id: epic:slice-14-shared-memory
kind: epic
status: draft
title: 'Slice 14, shared-memory: SharedArrayBuffer and Atomics'
summary: 'SharedArrayBuffer and Atomics for one agent that cannot block: 598 cases and 1,192 runs.'
relations:
- depends_on: epic:slice-13-binary-data
revision: 1
---
## Context

Slice 14 of 19, `shared-memory` (`docs/design/engine.md`, section 12): `SharedArrayBuffer` and `Atomics`
for one agent. It follows slices 1 to 13 in the design's order of dependencies (`SharedArrayBuffer`
and `Atomics` sit beside `ArrayBuffer` and the typed arrays of slice 13); the design gives no
other reason.
The agent is single and its [[CanBlock]] is false, as a window's agent is (decision 1). Concurrent
agents are out until surf needs workers (open item B). Governing decisions
(`docs/design/engine.md`, decision N): 1 (scope), 9 (hooks, jobs), 11 (the runner), 13.

## Outcome

- `SharedArrayBuffer` and `Atomics` for one agent whose [[CanBlock]] is false, including
  `Atomics.waitAsync` and `Atomics.pause` (the `feature` lines below).
- The host hook `HostGrowSharedArrayBuffer` (decision 9).
- `Atomics.waitAsync` timeouts as timeout jobs handed to the host (`HostEnqueueTimeoutJob`,
  decision 9); the runner's host runs the job when the queue is otherwise empty (decision 11).
- Cases whose flags do not apply to the host configuration count as `not-applicable`, visible per
  directory: `CanBlockIsTrue` while [[CanBlock]] is false (decision 1).
- `$262.agent` is not provided by this slice (open item B).

### Open, to be settled by this epic's first story

- How the 112 `$262.agent` cases (all under `built-ins/Atomics`) are counted. Decision 1 says
  concurrent agents are out and open item B recommends "not before surf plans workers", but the
  `swell-slices` block has no `exclude`, `feature` or `flag` line that sets them apart, so they
  are among the 598 cases of this slice and would fail the acceptance below.
- Why this slice has 598 cases where the union of the `SharedArrayBuffer`, `Atomics`,
  `Atomics.waitAsync` and `Atomics.pause` features is 615 cases (design, "Inventory").

## Counts

- Slice 14 (`shared-memory`): 598 cases, 1,192 runs (section 12 table).
- Directories the design names for it (paths under `test/`, cases): `Atomics` 380,
  `SharedArrayBuffer` 104. Inventory, in all slices: `SharedArrayBuffer` 467 cases, `Atomics` 386,
  `Atomics.waitAsync` 101; the union of the four features is 615; `CanBlockIsTrue` 7 and
  `CanBlockIsFalse` 2 cases.
- Lines of the `swell-slices` block that assign cases to this slice:
  - `dir test/built-ins/SharedArrayBuffer/ 14`
  - `dir test/built-ins/Atomics/ 14`
  - `feature SharedArrayBuffer 14`
  - `feature Atomics 14`
  - `feature Atomics.waitAsync 14`
  - `feature Atomics.pause 14`
  - `flag CanBlockIsTrue 14`
  - `flag CanBlockIsFalse 14`

Source: `docs/design/engine.md`, section 12 and the output under 'How the counts were made'; a count is measured again on the pinned test262 checkout before a story's acceptance uses it (`AGENTS.md`, 'How the work is run').

## Out of scope

- Concurrent agents, `$262.agent` (112 cases) and a blocking `Atomics.wait`: out until surf needs
  workers (open item B; the recommended option (b) is each agent as its own engine on a host
  thread, "not built or measured").
- `ArrayBuffer`, typed arrays and `DataView`: slice 13.
- Web Workers and the HTML Standard's agent clusters: surf's.
- `Temporal` (slice 20), ECMA-402 (slice 21), stage 3 features.

## Constraints

- Nothing third-party in what ships; `#![forbid(unsafe_code)]`: shared memory in this slice is one
  agent's safe storage, no raw shared memory (`AGENTS.md`, "Rules"; decision 13).
- No ambient authority: no clock and no thread in the engine; timeouts are jobs the host runs
  (`AGENTS.md`, "Rules"; decision 9).
- No panics, no `unwrap`, `expect` or unchecked indexing (`AGENTS.md`, "Rules").
- Every loop charged to the step budget, every allocation to the memory budget; a new limit gets a
  test that reaches it (`AGENTS.md`, "Rules").
- A deliberate omission is recorded where the gate reads it, so the cases that need concurrent
  agents are visible (`AGENTS.md`, "Rules").

## Acceptance

`slot task check` exits 0 on `main`, and in that run:

1. The conformance runner's line for slice 14 reads 598 cases and 1,192 runs that exist, and every run of the slice passes; no case of the slice is counted as failed or as ended by a budget.
2. No pass count of an earlier slice or of any directory line in `conformance/test262/expected.txt` is lower than before the slice.
3. `conformance/test262/expected.txt` is committed with the lines of that run, and `AGENTS.md` and `README.md` carry the gate's numbers.
4. A case of this slice that is found to need a later slice is moved by a rule line in the `swell-slices` block of the design with its reason, in the same change that moves the counts of both slices; none is skipped silently.
5. A case with the flag `CanBlockIsTrue`, run while [[CanBlock]] is false, is printed as `not-applicable` in its directory line (decision 1).
6. An `Atomics.waitAsync` case with a timeout passes with no timer in the engine: the runner's host answers `HostEnqueueTimeoutJob` by running the job when the queue is otherwise empty (decision 11).
