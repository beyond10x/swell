---
format: aep.planning-md/3
id: epic:slice-19-staging
kind: epic
status: draft
title: 'Slice 19, staging: what test/staging asks beyond the above'
summary: 'Whatever test/staging asks beyond slices 1 to 18: 1,474 cases and 2,766 runs.'
relations:
- depends_on: epic:slice-18-annex-b
revision: 1
---
## Context

Slice 19 of 19 (`docs/design/engine.md`, section 12): `staging`, "what
`test/staging/` asks beyond the above". It needs slices 1 to 18: it is the last slice in the design's
order of dependencies, and its content is what the earlier slices did not cover. Slices 1 to 19
total 44,931 cases and 85,794 runs; slice 20 (`Temporal`) comes after it with a design of its own
(decision 1), and ECMA-402 is slice 21 (open item A). Governing decisions
(`docs/design/engine.md`, decision N): 1 (scope), 11 (the runner, budgets), 4 (memory budget), 13.

## Outcome

- Whatever `test/staging/` asks beyond slices 1 to 18, counted by the one `dir` line below:
  `staging/sm` is the directory the design names (1,404 cases).
- Every case the one `dir` line counts here runs and passes; the cases of `test/staging/` that
  other lines move to slice 20, 21 or `x` are not this slice's.
- The design gives no list of builtins, syntax or host features for this slice.

### Open, to be settled by this epic's first story

- What this slice builds. Section 12 says "what `test/staging/` asks beyond the above" and names
  `staging/sm` only; the first story lists the cases that fail after slice 18 and groups them.
- Why the cases of this slice are 1,474 where `test/staging/` holds 1,487 and `staging/sm` is 1,406
  in the inventory and 1,404 in section 12 (the design gives no breakdown).
- Whether the first budget values (2^30 steps and 512 MiB, decision 11) suffice for the largest
  test file, `test/staging/sm/String/string-upper-lower-mapping.js` (3,234,014 bytes; the next are
  299,825 and 130,705 bytes), given that the parser's tree is charged to the memory budget while
  it exists (decision 4).
- Which of the 39 features with no `swell-slices` line fall inside this slice.

## Counts

- Slice 19 (`staging`): 1,474 cases, 2,766 runs (section 12 table).
- Directories the design names for it (paths under `test/`, cases): `staging/sm` 1,404. Inventory:
  `test/staging/` 1,487 cases, `staging/sm` 1,406.
- Lines of the `swell-slices` block that assign cases to this slice:
  - `dir test/staging/ 19`
  - no `feature` line and no `flag` line.
- The longer line `dir test/staging/Temporal/ 20` moves the cases under that prefix to slice 20.

Source: `docs/design/engine.md`, section 12 and the output under 'How the counts were made'; a count is measured again on the pinned test262 checkout before a story's acceptance uses it (`AGENTS.md`, 'How the work is run').

## Out of scope

- `test/staging/Temporal/` and every case with the feature `Temporal` or `canonical-tz`: slice 20,
  excluded at first (decision 1).
- Staging cases that carry an `Intl` feature: slice 21, ECMA-402 (open item A).
- Staging cases that carry a stage 3 feature (the `exclude` lines): counted in `x`.
- Concurrent agents (open item B).

## Constraints

- Nothing third-party in what ships; `#![forbid(unsafe_code)]`; no panics, no `unwrap`, `expect` or
  unchecked indexing (`AGENTS.md`, "Rules").
- Every loop charged to the step budget, every allocation to the memory budget, the parser's tree
  included while it exists (decision 4); a new limit gets a test that reaches it (`AGENTS.md`,
  "Rules").
- A case moved to a later slice or excluded is recorded where the gate reads it, never left
  implicit (`AGENTS.md`, "Rules").
- A builtin's module and comments cite its clause (`AGENTS.md`, "Rules").

## Acceptance

`slot task check` exits 0 on `main`, and in that run:

1. The conformance runner's line for slice 19 reads 1,474 cases and 2,766 runs that exist, and every run of the slice passes; no case of the slice is counted as failed or as ended by a budget.
2. No pass count of an earlier slice or of any directory line in `conformance/test262/expected.txt` is lower than before the slice.
3. `conformance/test262/expected.txt` is committed with the lines of that run, and `AGENTS.md` and `README.md` carry the gate's numbers.
4. A case of this slice that is found to need a later slice is moved by a rule line in the `swell-slices` block of the design with its reason, in the same change that moves the counts of both slices; none is skipped silently.
5. The case `test/staging/sm/String/string-upper-lower-mapping.js` (3,234,014 bytes) passes within the runner's budgets; where it does not, the budget values are set again from the runner's printed list of the twenty most expensive cases and the change is recorded (decision 11: "chosen, not derived"), and the case is never ended by a panic, an abort or a hang (decision 13).
