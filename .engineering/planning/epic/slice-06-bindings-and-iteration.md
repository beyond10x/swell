---
format: aep.planning-md/3
id: epic:slice-06-bindings-and-iteration
kind: epic
status: draft
title: 'Slice 6, bindings-and-iteration: let, destructuring, iterators'
summary: 'let, const, destructuring, spread, arrows, templates, Symbol, iterators and for-of: 3,701 cases and 7,130 runs.'
relations:
- depends_on: epic:slice-03-builtins-core
revision: 1
---
## Context

Slice 6 of 19, `bindings-and-iteration` (`docs/design/engine.md`, section 12). It needs slices 1 to 5:
section 12 says slices 2 to 5 are the language and library of ES5 and slices 6 to 8 "the syntax
most later tests are written in" (4,916 cases carry `class`). It adds `let`, `const`, destructuring,
spread, arrows, templates, `Symbol`, iterators and `for-of`. Slices 7 and 8 follow it in the
design's order. Governing decisions (`docs/design/engine.md`, decision N): 2 (parser, compiler),
3 (frames, step budget), 4 (values: `Symbol` handle), 5 (property keys), 13.

## Outcome

- Language: `let`, `const`, destructuring (binding and assignment), default parameters, rest
  parameters, object spread and object rest, arrow functions, template literals and tagged
  templates, computed property names, `for-of` (section 12, item 6; the `feature` lines below).
- Builtins: `Symbol` with the well-known symbols the `feature` lines name (`Symbol.iterator`,
  `Symbol.species`, `Symbol.toPrimitive`, `Symbol.toStringTag`, `Symbol.unscopables`,
  `Symbol.hasInstance`, `Symbol.isConcatSpreadable`, `Symbol.match`, `Symbol.matchAll`,
  `Symbol.replace`, `Symbol.search`, `Symbol.split`, `Symbol.prototype.description`), the iterator
  prototypes of `Array` and `String` (`ArrayIteratorPrototype`, `StringIteratorPrototype`).
- The `Symbol` handle of the `Value` enum and symbol property keys in the heap (decision 4).
- Loops over iterators and spread charged to the step budget like every builtin loop (decision 3).

### Open, to be settled by this epic's first story

- Which of the 39 features with no `swell-slices` line fall inside this slice (the design says
  they "are small additions inside the directory's own slice" and names `exponentiation`,
  `optional-chaining` and `Object.hasOwn` only as examples).

## Counts

- Slice 6 (`bindings-and-iteration`): 3,701 cases, 7,130 runs (section 12 table).
- Directories the design names for it (paths under `test/`, cases): `statements/for-of` 584,
  `expressions/assignment` 335, `expressions/arrow-function` 310.
- Lines of the `swell-slices` block that assign cases to this slice:
  - `dir test/built-ins/Symbol/ 6`
  - `dir test/built-ins/ArrayIteratorPrototype/ 6`
  - `dir test/built-ins/StringIteratorPrototype/ 6`
  - `dir test/language/statements/for-of/ 6`
  - `dir test/language/statements/let/ 6`
  - `dir test/language/statements/const/ 6`
  - `dir test/language/destructuring/ 6`
  - `dir test/language/rest-parameters/ 6`
  - `dir test/language/computed-property-names/ 6`
  - `dir test/language/expressions/arrow-function/ 6`
  - `dir test/language/expressions/template-literal/ 6`
  - `dir test/language/expressions/tagged-template/ 6`
  - `feature arrow-function 6`
  - `feature default-parameters 6`
  - `feature rest-parameters 6`
  - `feature destructuring-binding 6`
  - `feature destructuring-assignment 6`
  - `feature computed-property-names 6`
  - `feature template 6`
  - `feature let 6`
  - `feature const 6`
  - `feature for-of 6`
  - `feature object-spread 6`
  - `feature object-rest 6`
  - `feature Symbol 6`
  - `feature Symbol.iterator 6`
  - `feature Symbol.species 6`
  - `feature Symbol.toPrimitive 6`
  - `feature Symbol.toStringTag 6`
  - `feature Symbol.unscopables 6`
  - `feature Symbol.hasInstance 6`
  - `feature Symbol.isConcatSpreadable 6`
  - `feature Symbol.match 6`
  - `feature Symbol.matchAll 6`
  - `feature Symbol.replace 6`
  - `feature Symbol.search 6`
  - `feature Symbol.split 6`
  - `feature Symbol.prototype.description 6`
  - no `flag` line.

Source: `docs/design/engine.md`, section 12 and the output under 'How the counts were made'; a count is measured again on the pinned test262 checkout before a story's acceptance uses it (`AGENTS.md`, 'How the work is run').

## Out of scope

- Generators and `yield`: slice 7 (`feature generators 7`).
- Classes, `super`, `new.target`, private names: slice 8.
- `Map`, `Set`, `WeakMap`, `WeakSet`, `Proxy`, `Reflect`, and their iterator prototypes: slice 9.
- `Symbol.asyncIterator`, `for await`, async functions: slice 10.
- `Iterator` helpers, `using`: slice 15.
- Annex B forms of these constructs: slice 18.
- `Temporal` (slice 20), ECMA-402 (slice 21), stage 3 features, concurrent agents.

## Constraints

- Nothing third-party in what ships; `#![forbid(unsafe_code)]`; no panics, no `unwrap`, `expect` or
  unchecked indexing (`AGENTS.md`, "Rules").
- Every loop, including `for-of` over an iterator and spread, is charged to the step budget, and
  every allocation to the memory budget; a new limit gets a test that reaches it
  (`AGENTS.md`, "Rules").
- A builtin's module and comments cite its clause (`AGENTS.md`, "Rules").
- A deliberate omission is recorded where the gate reads it (`AGENTS.md`, "Rules").

## Acceptance

`slot task check` exits 0 on `main`, and in that run:

1. The conformance runner's line for slice 6 reads 3,701 cases and 7,130 runs that exist, and every run of the slice passes; no case of the slice is counted as failed or as ended by a budget.
2. No pass count of an earlier slice or of any directory line in `conformance/test262/expected.txt` is lower than before the slice.
3. `conformance/test262/expected.txt` is committed with the lines of that run, and `AGENTS.md` and `README.md` carry the gate's numbers.
4. A case of this slice that is found to need a later slice is moved by a rule line in the `swell-slices` block of the design with its reason, in the same change that moves the counts of both slices; none is skipped silently.
