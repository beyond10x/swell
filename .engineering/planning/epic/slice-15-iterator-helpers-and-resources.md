---
format: aep.planning-md/3
id: epic:slice-15-iterator-helpers-and-resources
kind: epic
status: draft
title: 'Slice 15, iterator-helpers-and-resources: Iterator, using'
summary: 'Iterator helpers, using, DisposableStack and AsyncDisposableStack: 1,094 cases and 2,170 runs.'
relations:
- depends_on: epic:slice-14-shared-memory
revision: 1
---
## Context

Slice 15 of 19, `iterator-helpers-and-resources` (`docs/design/engine.md`, section 12). It follows
slices 1 to 14 in the design's order of dependencies; the design states no reason for the position.
It builds the `Iterator` helpers and explicit resource management: `using`, `await using`,
`DisposableStack`, `AsyncDisposableStack`, `SuppressedError`. The async parts sit after slice 10
in that order. Governing decisions (`docs/design/engine.md`, decision N): 1 (scope, stage 3
exclusions), 2 (one module per clause), 3 (step budget), 13.

## Outcome

- `Iterator` and its helpers, by the `feature` lines below: `iterator-helpers`,
  `iterator-sequencing`, `iterator-chunking`, `iterator-includes`, `joint-iteration`.
- Explicit resource management (`explicit-resource-management`): the declarations `using` and
  `await using`, `DisposableStack`, `AsyncDisposableStack`, `SuppressedError` (the `dir` lines
  below).
- Helper and disposal loops charged to the step budget (decision 3).

### Open, to be settled by this epic's first story

- Which of the 39 features with no `swell-slices` line fall inside this slice.
- When `Iterator.prototype.join` moves from `exclude` to `run`: "it moves to `run` when the pinned
  draft has it" (decision 1); `finished-proposals.md` lists "Iterator Join" as finished while
  test262's `features.txt` still lists it under "Proposed language features".

## Counts

- Slice 15 (`iterator-helpers-and-resources`): 1,094 cases, 2,170 runs (section 12 table).
- Directories the design names for it (paths under `test/`, cases): `Iterator` 666,
  `AsyncDisposableStack` 104.
- Lines of the `swell-slices` block that assign cases to this slice:
  - `dir test/built-ins/Iterator/ 15`
  - `dir test/built-ins/DisposableStack/ 15`
  - `dir test/built-ins/AsyncDisposableStack/ 15`
  - `dir test/built-ins/SuppressedError/ 15`
  - `dir test/language/statements/using/ 15`
  - `dir test/language/statements/await-using/ 15`
  - `feature iterator-helpers 15`
  - `feature iterator-sequencing 15`
  - `feature iterator-chunking 15`
  - `feature iterator-includes 15`
  - `feature joint-iteration 15`
  - `feature explicit-resource-management 15`
  - no `flag` line.

Source: `docs/design/engine.md`, section 12 and the output under 'How the counts were made'; a count is measured again on the pinned test262 checkout before a story's acceptance uses it (`AGENTS.md`, 'How the work is run').

## Out of scope

- `Iterator.prototype.join`: excluded as stage 3 until the pinned draft has it (the `exclude` line).
- `Array.fromAsync` and the async iteration builtins: slice 10.
- `Temporal` (slice 20), ECMA-402 (slice 21), concurrent agents.

## Constraints

- Nothing third-party in what ships; `#![forbid(unsafe_code)]`; no panics, no `unwrap`, `expect` or
  unchecked indexing (`AGENTS.md`, "Rules").
- Every helper and disposal loop is charged to the step budget and every allocation to the memory
  budget; a new limit gets a test that reaches it (`AGENTS.md`, "Rules").
- A builtin's module and comments cite its clause (`AGENTS.md`, "Rules").
- A deliberate omission is recorded where the gate reads it (`features.txt`; `AGENTS.md`, "Rules").

## Acceptance

`slot task check` exits 0 on `main`, and in that run:

1. The conformance runner's line for slice 15 reads 1,094 cases and 2,170 runs that exist, and every run of the slice passes; no case of the slice is counted as failed or as ended by a budget.
2. No pass count of an earlier slice or of any directory line in `conformance/test262/expected.txt` is lower than before the slice.
3. `conformance/test262/expected.txt` is committed with the lines of that run, and `AGENTS.md` and `README.md` carry the gate's numbers.
4. A case of this slice that is found to need a later slice is moved by a rule line in the `swell-slices` block of the design with its reason, in the same change that moves the counts of both slices; none is skipped silently.
