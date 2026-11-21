---
format: aep.planning-md/3
id: epic:slice-17-tail-calls
kind: epic
status: draft
title: 'Slice 17, tail-calls: PrepareForTailCall'
summary: 'Tail calls in strict code as a frame replacement: 35 cases and 35 runs.'
relations:
- depends_on: epic:slice-16-weak-references
revision: 1
---
## Context

Slice 17 of 19, `tail-calls` (`docs/design/engine.md`, section 12): `PrepareForTailCall` (15.10). It
follows slices 1 to 16 in the design's order of dependencies; the design states no reason for the
position. Its cases are "spread over `language/`": it has no `dir` line, only the `feature` line
`tail-call-optimization 17`. In scope because "the draft requires them in strict code, and with
frames on the engine's own stack a tail call is a frame replacement" (decision 1). Governing
decisions (`docs/design/engine.md`, decision N): 1 (scope), 3 (frames on the engine's own stack,
the frame depth bound), 13.

## Outcome

- Tail calls in strict code (15.10, `PrepareForTailCall`): a tail call replaces the calling frame
  on the engine's frame stack (decision 1, decision 3).
- No other construct: the slice is one feature, `tail-call-optimization`.

### Open, to be settled by this epic's first story

- Why the slice has 35 runs for 35 cases: the design gives the counts and not the flags that make
  each case run once.
- Where in the compile step a call is marked as a tail call (the design gives no detail).

## Counts

- Slice 17 (`tail-calls`): 35 cases, 35 runs (section 12 table).
- Directories the design names for it: none; "Spread over `language/`". Inventory, in all slices:
  `tail-call-optimization` is carried by 35 cases.
- Lines of the `swell-slices` block that assign cases to this slice:
  - `feature tail-call-optimization 17`
  - no `dir` line and no `flag` line.

Source: `docs/design/engine.md`, section 12 and the output under 'How the counts were made'; a count is measured again on the pinned test262 checkout before a story's acceptance uses it (`AGENTS.md`, 'How the work is run').

## Out of scope

- Tail calls in sloppy code: the draft requires them in strict code only (decision 1).
- Any other optimisation of the interpreter (shapes, ropes, NaN-boxing, a register machine): not
  before a measurement asks for it (design, "Not in scope").
- `Temporal` (slice 20), ECMA-402 (slice 21), stage 3 features, concurrent agents.

## Constraints

- Nothing third-party in what ships; `#![forbid(unsafe_code)]`; no panics, no `unwrap`, `expect` or
  unchecked indexing (`AGENTS.md`, "Rules").
- A frame replacement must keep every limit: each call still charges the step budget, and the memory
  budget still counts what the replaced frame held; a new limit gets a test that reaches it
  (`AGENTS.md`, "Rules").
- Every recursion stays bounded: a tail call must not make an unbounded loop that escapes the step
  budget (`AGENTS.md`, "Rules").
- A deliberate omission is recorded where the gate reads it (`AGENTS.md`, "Rules").

## Acceptance

`slot task check` exits 0 on `main`, and in that run:

1. The conformance runner's line for slice 17 reads 35 cases and 35 runs that exist, and every run of the slice passes; no case of the slice is counted as failed or as ended by a budget.
2. No pass count of an earlier slice or of any directory line in `conformance/test262/expected.txt` is lower than before the slice.
3. `conformance/test262/expected.txt` is committed with the lines of that run, and `AGENTS.md` and `README.md` carry the gate's numbers.
4. A case of this slice that is found to need a later slice is moved by a rule line in the `swell-slices` block of the design with its reason, in the same change that moves the counts of both slices; none is skipped silently.
5. A strict-code function that calls itself in tail position more times than the host's frame depth limit ends normally, without a `RangeError` (a tail call is a frame replacement); the same recursion not in tail position throws a `RangeError` (decisions 1 and 3).
6. A tail-recursive function that never ends ends with `Terminated(StepBudget)`, not in a hang (decision 3).
