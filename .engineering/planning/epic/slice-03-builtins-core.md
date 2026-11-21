---
format: aep.planning-md/3
id: epic:slice-03-builtins-core
kind: epic
status: draft
title: 'Slice 3, builtins-core: Object, Function, Array, String, JSON'
summary: 'Object, Function, Array, String, Boolean, Error, JSON and the global functions: 7,580 cases and 14,992 runs.'
relations:
- depends_on: epic:slice-02-language-core
revision: 1
---
## Context

Slice 3 of 19, `builtins-core` (`docs/design/engine.md`, section 12). It needs slices 1 and 2: the
interpreter and heap its builtins read and write, and the ES5-level language the builtins are
written against and tested in. Section 12 puts slices 2 to 5 as the language and library of ES5.
It builds the core library: `Object`, `Function`, `Array`, `String`, `Boolean`, `Error`, `JSON` and
the global functions. Governing decisions (`docs/design/engine.md`, decision N): 2 (one module per
clause in `swell`), 3 (re-entry, `JSON.parse` stack), 4 (memory budget), 5 (strings), 8, 13.

## Outcome

- `Object`, `Function`, `Array`, `String`, `Boolean`, `Error`, `JSON` and the global functions,
  each builtin in a module that cites its ECMA-262 clause (decision 2; clauses 19 to 28).
- Builtins that call back into script (an accessor, `Array.prototype.map`'s callback, `toString`,
  `valueOf`, `JSON.stringify`'s `toJSON`) enter the interpreter again from Rust, and every
  re-entry counts toward the frame depth bound and the re-entry limit (decision 3).
- Step units charged by builtins in proportion to their work: per element for array loops, per code
  unit for string operations, per node for `JSON.parse` (decision 3).
- `JSON.parse` is a loop with an explicit stack bounded by the memory budget (decision 3).
- Strings: the length limit the host sets throws a `RangeError` when exceeded (decision 5).
- `toLocaleString` and `localeCompare` as ECMA-262's locale-independent definitions have them
  (decision 1, per area, ECMA-402).
- `Function` through `HostEnsureCanCompileStrings` (decision 9) and with the parser nesting bound
  of decision 3, as a `RangeError`.
- The interface documents the minimum Rust stack the host must give the agent, and a test runs at
  the re-entry limit on a thread of that size (decision 9).

### Open, to be settled by this epic's first story

- Which global functions are this slice's. `parseInt` and `parseFloat` are slice 4's by the
  `dir` lines; the design does not list the others.
- How `String.prototype.toUpperCase`, `toLowerCase` and `normalize` get their tables.
  Decision 8 puts them in `swell-unicode`, which section 12 builds in slice 5.
- How `ToString` of a Number works before slice 4 (`swell-num`'s `Number::toString`); `Array`
  and `String` conversions need it.
- The Rust stack one re-entry needs: "The size is not measured yet" (decision 9).
- Whether the "implementation limits" wording of ECMA-262 allows the `RangeError` for string
  length: the design checks this only for `BigInt` and pattern nesting.

## Counts

- Slice 3 (`builtins-core`): 7,580 cases, 14,992 runs (section 12 table).
- Directories the design names for it (paths under `test/`, cases): `Object` 3,160, `Array` 2,665,
  `String` 780, `Function` 417. Inventory, in all slices: `built-ins/Object` 3,411, `built-ins/Array`
  3,083, `built-ins/String` 1,223.
- Lines of the `swell-slices` block that assign cases to this slice:
  - `dir test/built-ins/ 3`
  - no `feature` line and no `flag` line.

Source: `docs/design/engine.md`, section 12 and the output under 'How the counts were made'; a count is measured again on the pinned test262 checkout before a story's acceptance uses it (`AGENTS.md`, 'How the work is run').

## Out of scope

- `Number`, `Math`, `Date`, `parseInt`, `parseFloat`: slice 4 (the `dir` lines under
  `test/built-ins/`).
- `RegExp` and `String.prototype` `match`, `matchAll`, `replace`, `replaceAll`, `search`, `split`:
  slice 5.
- `Symbol` and the iterator prototypes: slice 6. `Map`, `Set`, `WeakMap`, `WeakSet`, `Proxy`,
  `Reflect`: 9. `Promise` and the async builtins: 10. `BigInt`: 12. `ArrayBuffer`, typed arrays,
  `DataView`: 13. `Atomics`, `SharedArrayBuffer`: 14. `Iterator`: 15. `WeakRef`: 16.
- `Error.prototype.stack` (the `error-stack-accessor` line, stage 3, excluded).
- `Temporal` (slice 20), `Intl` (slice 21, open item A), other stage 3 features, concurrent agents.

## Constraints

- Nothing third-party in what ships; `#![forbid(unsafe_code)]`; no panics, no `unwrap`, `expect` or
  unchecked indexing in engine crates (`AGENTS.md`, "Rules").
- A builtin's module and comments cite its clause (`AGENTS.md`, "Rules"; decision 2).
- Every builtin loop is charged to the step budget and every allocation to the memory budget; a new
  limit gets a test that reaches it (`AGENTS.md`, "Rules").
- No ambient authority: no `RandomState`, no clock, no environment in the builtins
  (`AGENTS.md`, "Rules").

## Acceptance

`slot task check` exits 0 on `main`, and in that run:

1. The conformance runner's line for slice 3 reads 7,580 cases and 14,992 runs that exist, and every run of the slice passes; no case of the slice is counted as failed or as ended by a budget.
2. No pass count of an earlier slice or of any directory line in `conformance/test262/expected.txt` is lower than before the slice.
3. `conformance/test262/expected.txt` is committed with the lines of that run, and `AGENTS.md` and `README.md` carry the gate's numbers.
4. A case of this slice that is found to need a later slice is moved by a rule line in the `swell-slices` block of the design with its reason, in the same change that moves the counts of both slices; none is skipped silently.
5. `"x".repeat(2**30)` ends in a `RangeError` (the host's string length limit) or in `Terminated(MemoryBudget)`, never in a panic, an abort or a hang (decisions 5, 4 and 13).
6. `JSON.parse` of a text of 100,000 nested brackets ends in a result or in `Terminated(MemoryBudget)`, never in a Rust stack overflow (decision 3).
7. A script that recurses through a builtin that calls back (`Array.prototype.map`'s callback, `toString`, `valueOf`, `JSON.stringify`'s `toJSON`) deeper than the re-entry limit throws a `RangeError`; the test runs on a thread of the documented minimum stack size (decisions 3 and 9).
8. `Function` called with a body nested deeper than the host's parser nesting limit throws a `RangeError` (decision 3).
