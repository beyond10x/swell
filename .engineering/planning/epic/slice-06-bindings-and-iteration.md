---
format: aep.planning-md/3
id: epic:slice-06-bindings-and-iteration
kind: epic
status: active
title: 'Slice 6, bindings-and-iteration: let, destructuring, iterators'
summary: 'let, const, destructuring, spread, arrows, templates, Symbol, iterators and for-of: 3,701 cases and 7,130 runs.'
relations:
- depends_on: epic:slice-03-builtins-core
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T00:38:43Z", actor: "human:operator", revision: 3, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-22T00:38:43Z", actor: "human:operator", revision: 4, executor: "agent:coordinator"}
---
## Context

Slice 6 of 19, `bindings-and-iteration` (`docs/design/engine.md`, section 12): `let`, `const`,
destructuring, spread, arrows, templates, `Symbol`, iterators and `for`-`of`. It is delivered
after slice 3 and before slices 4 and 5: the order of delivery follows the consumer's first
gap (`docs/plan/waves.md`, "The order after slice 3"). The consumer's first script test, the
testharness.js tests of surf's HTML parser, needs 191 constructs of this slice (`const` 75,
arrow functions 41, `let` 23, template literals 22, `for`-`of` 12, default parameters 9,
destructuring 6, rest parameters 2, spread 1), 12 regular expression literals of slice 5 and
3 constructs of slice 10. The slice needs slices 1 to 3; a case of it that needs slice 4 or 5
moves there at its close. Governing decisions (`docs/design/engine.md`, decision N): 2
(parser, compiler), 3 (frames, the step budget, lexical bindings, arrows, parameter lists,
block functions, iterator records, IteratorClose), 4 (values, symbols and the registry,
template objects), 5 (property keys), 13.

Rewritten on 2026-11-22, after the slice was decomposed into fifteen stories in two parts
(`docs/plan/slice-06/`): the counts are those of `main` (`6a43e18`), and the design text the
stories build on is in the design.

## Outcome

- Language: `let`, `const`, destructuring (binding and assignment), default parameters, rest
  parameters, object spread and object rest, arrow functions, template literals and tagged
  templates, computed property names, `for`-`of`, function declarations in blocks and in
  `switch` (section 12, item 6; the `feature` lines below).
- Builtins: `Symbol` with `Symbol.for`, `Symbol.keyFor`, `Symbol.prototype.description` and
  the well-known symbols the `feature` lines name (`Symbol.iterator`, `Symbol.species`,
  `Symbol.toPrimitive`, `Symbol.toStringTag`, `Symbol.unscopables`, `Symbol.hasInstance`,
  `Symbol.isConcatSpreadable`, `Symbol.match`, `Symbol.matchAll`, `Symbol.replace`,
  `Symbol.search`, `Symbol.split`); the iterator prototypes of `Array` and `String`
  (`ArrayIteratorPrototype`, `StringIteratorPrototype`) with
  `Array.prototype.values`, `keys` and `entries` and `@@iterator` of the `arguments` object;
  `Array.from`, `Object.fromEntries` and `Object.groupBy`, which slice 3's close moves here;
  and what a Symbol value asks of the builtins of slice 3 (a `TypeError` where a Symbol would
  be converted, the hooks `@@toPrimitive`, `@@species`, `@@toStringTag`).
- The `Symbol` handle of the `Value` enum and symbol property keys in the heap (decision 4).
- Loops over iterators and spread charged to the step budget like every builtin loop
  (decision 3).
- The host requirement tagged `[slice 6]` has its test: `HOST-156`.
- After the story of array patterns and spread, the two scripts of the consumer's first test
  (`resources/testharness.js` and `html/syntax/parsing/test.js` of web-platform-tests at the
  commit surf pins) parse with no refusal of slice 6.

### Settled since the epic was drafted

- Which features with no `swell-slices` line fall inside this slice: the design answers it
  (section 12: they are small additions inside the directory's own slice).
- The Annex B semantics of a function in a block (B.3.2, B.3.3) are slice 18's; until then a
  block function is a lexical binding in non-strict code too, a recorded omission.

## Counts

- Slice 6 on `main` (`6a43e18`): 4,204 cases and 7,952 runs; 94 cases pass, 4,110 fail. (The
  design's table said 3,701 cases before slices 1 and 2 moved 487 cases here.)
- Slice 3's close adds about 224 cases, 68 of them in the six directories of builtins that
  need iterators.
- The close of this slice moves about 657 cases on (1,221 failing runs,
  `docs/plan/slice-06/move-estimate.tsv`), by target slice: 4: 69, 5: 274, 7: 92, 8: 73,
  9: 99, 10: 11, 11: 2, 12: 20, 13: 12, 18: 5. About 3,770 cases stay; the close measures
  the counts its acceptance uses.
- Directories the design names for it (paths under `test/`, cases): `statements/for-of` 584,
  `expressions/assignment` 344, `expressions/arrow-function` 310.
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
  - the `dir` lines slice 3's close adds for the six directories it moves here.

Source: `conformance/test262/expected.txt`, `.out/test262/runs.tsv` of `main` and
`docs/design/engine.md`, section 12; a count is measured again on the pinned test262 checkout
before a story's acceptance uses it (`AGENTS.md`, "How the work is run").

## Out of scope

- Generators and `yield`: slice 7 (`feature generators 7`).
- Classes, `super`, `new.target`, private names: slice 8.
- `Map`, `Set`, `WeakMap`, `WeakSet`, `Proxy`, `Reflect`, and their iterator prototypes: slice 9.
- `Symbol.asyncIterator`, `for await`, async functions: slice 10.
- `Iterator` helpers, `using`: slice 15.
- Annex B forms of these constructs, the `var` binding of a function in a block included:
  slice 18.
- `Temporal` (slice 20), ECMA-402 (slice 21), stage 3 features, concurrent agents.
- The objects of the DOM and of the host that the consumer's scripts call: surf's side.

## Constraints

- Nothing third-party in what ships; `#![forbid(unsafe_code)]`; no panics, no `unwrap`, `expect` or
  unchecked indexing (`AGENTS.md`, "Rules").
- Every loop, including `for`-`of` over an iterator and spread, is charged to the step budget, and
  every allocation to the memory budget; a new limit gets a test that reaches it
  (`AGENTS.md`, "Rules").
- A builtin's module and comments cite its clause (`AGENTS.md`, "Rules").
- A deliberate omission is recorded where the gate reads it (`AGENTS.md`, "Rules").
- A new instruction does its work in a function of its own; nothing is added to the frame of
  the run loop (`AGENTS.md`, "Commands and the gate").
- The gate's test steps end within 120 s and a test within 60 s; a unit times each test it
  adds (the organization's rule "Tests stay fast").

## Acceptance

`slot task check` exits 0 on `main`, `slot task slow` exits 0, and in those runs:

1. The conformance runner's line for slice 6 reads `test262-slice 06 <c> <r> 0 0 <c> <c> 0 0` with the cases and runs the design's table states for the slice after its close: every run of the slice passes, and no case of it is counted as failed or as ended by a budget.
2. No pass count of an earlier slice or of any directory line in `conformance/test262/expected.txt` is lower than before the slice, except a case that passed only on the absence of a part this slice delivers and fails on a part of a later slice; each such case is named in the wave record that lowered the line.
3. `conformance/test262/expected.txt` is committed with the lines of that run, and `AGENTS.md` and `README.md` carry the gate's numbers.
4. A case of this slice that is found to need a later slice is moved by a rule line in the `swell-slices` block of the design with its evidence in `conformance/test262/moves.tsv`, in the same change that moves the counts of both slices, and `test262-moves-check` passes; none is skipped silently, and a case that fails on a part of this slice is fixed, not moved.
5. `host-requirements-check` counts no requirement pending for slice 6 (`HOST-156` names its test), and `conformance/slices-delivered.txt` holds `6`.
6. The omission of the `@@iterator` property of the `arguments` object (`conformance/test262/sections.txt`) is gone, and the omission of Annex B.3.2 is recorded there with slice 18.
7. The two scripts of the consumer's first test parse with no refusal of slice 6, by the check of `story:array-destructuring-and-spread`.
