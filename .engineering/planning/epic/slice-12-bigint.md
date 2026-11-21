---
format: aep.planning-md/3
id: epic:slice-12-bigint
kind: epic
status: draft
title: 'Slice 12, bigint: BigInt'
summary: 'BigInt on swell-num''s big integers, with a size limit that ends in RangeError: 316 cases and 632 runs.'
relations:
- depends_on: epic:slice-11-modules
revision: 1
---
## Context

Slice 12 of 19, `bigint` (`docs/design/engine.md`, section 12). It needs slice 4 for `swell-num` and
the earlier slices for the engine; it comes before slice 13 because "13 needs 12 for `BigInt64Array`".
It builds `BigInt`: the builtin, the literals and the arithmetic on `swell-num`'s big integers.
The design states no reason for 12 to follow 11 beyond the order of dependencies. Governing
decisions (`docs/design/engine.md`, decision N): 6 (the `BigInt` algorithms and the size limit),
4 (`BigInt` digits in their own arena, charged to the memory budget), 3 (a step unit per limb
operation), 13 (`2n**(2n**40n)`).

## Outcome

- `BigInt` written in `swell-num` (decision 6): 32-bit limbs with 64-bit intermediates, schoolbook
  multiplication with Karatsuba above a size, Knuth's algorithm D for division, radix conversion by
  divide and conquer.
- The `BigInt` value type and its arena in the heap (decision 4); limbs charged to the memory budget
  (decision 4).
- Per limb operation, a step unit is charged (decision 3).
- An operation whose result size is known in advance to exceed the host's `BigInt` limit throws a
  `RangeError` before computing (decision 6; the limit is one of those the host sets at agent
  creation, decision 9).
- The `BigInt` builtin and `BigInt` literals. The `feature BigInt 12` line counts the cases that
  carry the feature in this slice or in a later one, by the highest-slice rule (section 12).

### Open, to be settled by this epic's first story

- Whether ECMA-262 words an allowance for a `RangeError` when a `BigInt` result is too large: "not
  checked" (decision 6, "Not checked").
- The size above which Karatsuba is used: decision 6 says "above a size" and gives none.
- Whether the `BigInt` part of `swell-num` (decision 2) is written in this slice or already in slice 4.

## Counts

- Slice 12 (`bigint`): 316 cases, 632 runs (section 12 table).
- Directories the design names for it (paths under `test/`, cases): `BigInt` 77, `literals` 59.
  Inventory, in all slices: the feature `BigInt` is carried by 1,506 cases.
- Lines of the `swell-slices` block that assign cases to this slice:
  - `dir test/built-ins/BigInt/ 12`
  - `feature BigInt 12`
  - no `flag` line.

Source: `docs/design/engine.md`, section 12 and the output under 'How the counts were made'; a count is measured again on the pinned test262 checkout before a story's acceptance uses it (`AGENTS.md`, 'How the work is run').

## Out of scope

- `BigInt64Array` and the other typed arrays: slice 13 (it needs this slice).
- `Number` and `Math`: slice 4.
- ECMA-402 (`Intl`): slice 21, open item A.
- `Temporal` (slice 20), other stage 3 features, concurrent agents.

## Constraints

- Nothing third-party in what ships, so no big-integer crate; `#![forbid(unsafe_code)]`; no panics,
  with overflow checks on in `swell-num` (`AGENTS.md`, "Rules"; decision 13).
- Every limb loop is charged to the step budget and every limb allocation to the memory budget; the
  `BigInt` limit is a new limit and gets a test that reaches it (`AGENTS.md`, "Rules").
- A builtin's module and comments cite its clause (`AGENTS.md`, "Rules").
- A deliberate omission is recorded where the gate reads it (`AGENTS.md`, "Rules").

## Acceptance

`slot task check` exits 0 on `main`, and in that run:

1. The conformance runner's line for slice 12 reads 316 cases and 632 runs that exist, and every run of the slice passes; no case of the slice is counted as failed or as ended by a budget.
2. No pass count of an earlier slice or of any directory line in `conformance/test262/expected.txt` is lower than before the slice.
3. `conformance/test262/expected.txt` is committed with the lines of that run, and `AGENTS.md` and `README.md` carry the gate's numbers.
4. A case of this slice that is found to need a later slice is moved by a rule line in the `swell-slices` block of the design with its reason, in the same change that moves the counts of both slices; none is skipped silently.
5. `2n**(2n**40n)` throws a `RangeError` before any limb is computed, under the host's `BigInt` limit; no panic, abort or hang (decisions 6 and 13).
6. A `BigInt` operation whose result fits the `BigInt` limit but not the memory budget ends the run with `Terminated(MemoryBudget)` (decisions 4 and 13).
