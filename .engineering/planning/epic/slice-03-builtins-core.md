---
format: aep.planning-md/3
id: epic:slice-03-builtins-core
kind: epic
status: active
title: 'Slice 3, builtins-core: Object, Function, Array, String, JSON'
summary: 'Object, Function, Array, String, Boolean, Error, JSON and the global functions: 7,580 cases and 14,992 runs.'
relations:
- depends_on: epic:slice-02-language-core
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T21:30:47Z", actor: "human:operator", revision: 5, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-21T21:30:47Z", actor: "human:operator", revision: 6, executor: "agent:coordinator"}
---
## Context

Slice 3 of 19, `builtins-core` (`docs/design/engine.md`, section 12). It needs slices 1 and 2: the
interpreter and heap its builtins read and write, and the ES5-level language the builtins are
written against and tested in. Section 12 puts slices 2 to 5 as the language and library of ES5.
It builds the core library: `Object`, `Function`, `Array`, `String`, `Error`, `JSON` and the
global functions that are not numbers. Governing decisions (`docs/design/engine.md`, decision N):
2 (one module per clause in `swell`), 3 (re-entry, `JSON.parse` stack), 4 (memory budget, the
object model), 5 (strings), 8, 13.

Rewritten on 2026-11-21, after slices 1 and 2 were delivered and the slice was decomposed
(`docs/plan/waves.md`, "Slice 3"): the counts are those of `main` after the close of slice 2,
and what slices 1 and 2 took of this slice early is named as delivered.

## Outcome

- `Object`, `Function`, `Array`, `String`, `Error`, `JSON`, `globalThis` and the URI functions,
  each builtin in a module that cites its ECMA-262 clause (decision 2; clauses 19 to 28). Already
  delivered and not this slice's: `Boolean`, `Number`, `isNaN`, `isFinite`,
  `String.prototype.toString` and `valueOf`, `Array.prototype.toString`, the `Function`
  constructor with its hook and its nesting bound (section 12, "A part before its slice").
- Builtins that call back into script (an accessor, a callback of an `Array.prototype` method,
  `toString`, `valueOf`, a reviver of `JSON.parse`) enter the interpreter again from Rust, and
  every re-entry counts toward the frame depth bound and the re-entry limit (decision 3). Each
  such builtin has a test at the re-entry limit on the stack `Limits::min_stack_bytes()`
  promises; the promise is not raised without a measurement
  (`docs/plan/slice-02-run-loop-frame.md`).
- Step units charged by builtins in proportion to their work: per element for array loops, per code
  unit for string operations, per node for `JSON.parse` (decision 3).
- `JSON.parse` is a loop with an explicit stack bounded by the memory budget (decision 3).
- Strings: the length limit the host sets throws a `RangeError` when exceeded (decision 5).
- `toLocaleString` and `localeCompare` as ECMA-262's locale-independent definitions have them
  (decision 1, per area, ECMA-402).
- The rest of `Function`: `Function.prototype.apply` and `toString`, the source text of a
  function kept with it and charged to the memory budget (`HOST-120`).
- The object model of the design, decision 4: a property is found through an index map above a
  small size and an array's elements are stored densely, so that the time of a lookup does not
  grow with the size of the object; the bindings a non-strict direct `eval` adds are found the
  same way or their scan is charged (`docs/plan/waves.md`, "What later slices take over":
  slice 3 at the latest).
- The four host requirements tagged `[slice 3]` have their tests: `HOST-21`, `HOST-120`,
  `HOST-155`, `HOST-165` (the counted refusal of a String method given a code unit outside
  ASCII before slice 5).
- Every case of the slice that needs a later slice is moved there with its evidence in
  `conformance/test262/moves.tsv`, checked by `test262-moves-check`, as slice 2 did
  (`docs/plan/slice-02-wave-4.md`). The task file of the runner reads its paths the same way
  whether a variable is exported or passed on the command line (`docs/plan/waves.md`, the
  entry on `taskfiles/test262.yml`).

### Settled since the epic was drafted

- Which global functions are this slice's: the URI functions and `globalThis`; `parseInt` and
  `parseFloat` are slice 4's (section 12, "A part before its slice").
- The tables for case mapping and `normalize`: slice 5 (the same section). What a String
  method does with a code unit outside ASCII before slice 5 is a design change made before the
  wave that delivers those methods.
- `ToString` of a Number: `swell-num` prints every radix since wave 1 of slice 2.
- The Rust stack one re-entry needs: measured, 11,100 bytes on the worst path against 14 KiB
  promised (`docs/plan/slice-02-run-loop-frame.md`).
- The `RangeError` for string length: a stated deviation (`docs/design/host-interface.md`,
  section 2), with its `omit` lines written by the close of the slice, which also moves the
  requirement lines; the stories that deliver the behaviour check their line on a copy.

## Counts

- Slice 3 (`builtins-core`) on `main` after the close of slice 2:
  `test262-slice 03 7635 15074 0 0 7635 2762 4873 0`: 7,635 cases, 15,074 runs, 2,762 cases
  pass. The slice's close moves the cases that need a later slice, and the design's table then
  states the counts the acceptance uses.
- Lines of the `swell-slices` block that assign cases to this slice: `dir test/built-ins/ 3`
  and the lines of the moves of slices 1 and 2 with target 3.

Source: `conformance/test262/expected.txt` and `docs/design/engine.md`, section 12; a count is
measured again on the pinned test262 checkout before a story's acceptance uses it (`AGENTS.md`,
"How the work is run").

## Out of scope

- `Number` beyond what slice 2 took, `Math`, `Date`, `parseInt`, `parseFloat`: slice 4.
- `RegExp` and `String.prototype` `match`, `matchAll`, `replace`, `replaceAll`, `search`, `split`,
  and case mapping and `normalize` outside ASCII: slice 5.
- `Symbol`, the iterator prototypes and the builtins that need an iterator (`Array.from`, the
  Array iterator methods, `Object.fromEntries`, `Object.groupBy`): slice 6. `Map`, `Set`,
  `WeakMap`, `WeakSet`, `Proxy`, `Reflect`: 9. `Promise` and the async builtins: 10. `BigInt`: 12.
  `ArrayBuffer`, typed arrays, `DataView`: 13. `Atomics`, `SharedArrayBuffer`: 14. `Iterator`: 15.
  `WeakRef`: 16.
- `Error.prototype.stack` (the `error-stack-accessor` line, stage 3, excluded).
- `Temporal` (slice 20), `Intl` (slice 21, open item A), other stage 3 features, concurrent agents.
- A way for a host function to create a realm, and whether a host function reads its data back:
  both wait for a decision on the host interface (`docs/plan/waves.md`).

## Constraints

- Nothing third-party in what ships; `#![forbid(unsafe_code)]`; no panics, no `unwrap`, `expect` or
  unchecked indexing in engine crates (`AGENTS.md`, "Rules").
- A builtin's module and comments cite its clause (`AGENTS.md`, "Rules"; decision 2).
- Every builtin loop is charged to the step budget and every allocation to the memory budget; a new
  limit gets a test that reaches it (`AGENTS.md`, "Rules").
- No ambient authority: no `RandomState`, no clock, no environment in the builtins
  (`AGENTS.md`, "Rules").
- A builtin's work is a function of its own; nothing is added to the frame of the run loop
  (`AGENTS.md`, "Commands and the gate").

## Acceptance

`slot task check` exits 0 on `main`, and in that run:

1. The conformance runner's line for slice 3 reads the cases and runs the design's table states for the slice after its close, and every run of the slice passes; no case of the slice is counted as failed or as ended by a budget.
2. No pass count of an earlier slice or of any directory line in `conformance/test262/expected.txt` is lower than before the slice.
3. `conformance/test262/expected.txt` is committed with the lines of that run, and `AGENTS.md` and `README.md` carry the gate's numbers.
4. A case of this slice that is found to need a later slice is moved by a rule line in the `swell-slices` block of the design with its evidence in `conformance/test262/moves.tsv`, in the same change that moves the counts of both slices, and `test262-moves-check` passes; none is skipped silently, and a case that fails on a part of this slice is fixed, not moved.
5. `"x".repeat(2**30)` ends in a `RangeError` (the host's string length limit) or in `Terminated(MemoryBudget)`, never in a panic, an abort or a hang (decisions 5, 4 and 13).
6. `JSON.parse` of a text of 100,000 nested brackets ends in a result or in `Terminated(MemoryBudget)`, never in a Rust stack overflow (decision 3).
7. A script that recurses through a builtin that calls back (a callback of an `Array.prototype` method, `toString`, `valueOf`, a reviver of `JSON.parse`) deeper than the re-entry limit throws a `RangeError`; the test runs on a thread of the documented minimum stack size (decisions 3 and 9).
8. The time of a script that appends n elements to an array, or adds n properties to an object, grows in proportion to n and not to its square, by the bound the story of the object model states and a test holds (decision 4).
9. `host-requirements-check` counts no requirement pending for slice 3, and `conformance/slices-delivered.txt` holds `3`.
