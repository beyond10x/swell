---
format: aep.planning-md/3
id: epic:slice-07-generators
kind: epic
status: draft
title: 'Slice 7, generators: frames saved in generator objects'
summary: 'Generators, with frames saved in objects and resumed: 1,432 cases and 2,771 runs.'
relations:
- depends_on: epic:slice-06-bindings-and-iteration
revision: 1
---
## Context

Slice 7 of 19, `generators` (`docs/design/engine.md`, section 12): "frames saved in objects". It needs
slices 1 to 6 in the design's order of dependencies (iterators and `for-of` are slice 6's); the
design states no further reason, and says slice 10 "needs 7's suspended frames". It is the first use
of the decision that every ECMAScript frame is data on the engine's own frame stack: `yield` moves
the frame into the generator object and resume moves it back. Governing decisions
(`docs/design/engine.md`, decision N): 3 (execution model), 4 (collector roots), 2, 13.

## Outcome

- Generator functions and generator objects: `yield` and `yield*` save the frame (instruction index,
  operand stack, local slots, environment, handler table) in the generator object and resume moves
  it back (decision 3). No host thread, no Rust stack kept alive, no unsafe stack switching.
- `GeneratorFunction` and `GeneratorPrototype` builtins; generator declarations, expressions and
  methods (section 12, item 7; the `dir` lines below).
- Frames saved in generator objects are traced by the collector through the object that holds them
  (decision 4: roots are the frame stack, and what is reachable from objects is traced).
- A generator run charges the step budget and the memory budget like any other frame (decision 3,
  decision 4).

### Open, to be settled by this epic's first story

- How a suspended frame is accounted to the memory budget, and how a generator that is never
  resumed is released: decision 4 names the roots and the accounting function but not suspended
  frames.
- Which of the 39 features with no `swell-slices` line fall inside this slice.

## Counts

- Slice 7 (`generators`): 1,432 cases, 2,771 runs (section 12 table).
- Directories the design names for it (paths under `test/`, cases): `expressions/object` 303,
  `expressions/generators` 288, `statements/generators` 266. Inventory: the feature `generators`
  is carried by 4,129 cases in all slices.
- Lines of the `swell-slices` block that assign cases to this slice:
  - `dir test/built-ins/GeneratorPrototype/ 7`
  - `dir test/built-ins/GeneratorFunction/ 7`
  - `dir test/language/expressions/generators/ 7`
  - `dir test/language/statements/generators/ 7`
  - `dir test/language/expressions/yield/ 7`
  - `feature generators 7`
  - no `flag` line.

Source: `docs/design/engine.md`, section 12 and the output under 'How the counts were made'; a count is measured again on the pinned test262 checkout before a story's acceptance uses it (`AGENTS.md`, 'How the work is run').

## Out of scope

- Async functions, async generators, `await`, `for await`, `AsyncGeneratorPrototype` and its request
  queue (27.6.3): slice 10.
- Classes whose methods are generators: counted in slice 8 where the `class` feature raises them
  (the rule counts a case at the highest slice).
- `Iterator` helpers: slice 15.
- A register machine, or any change to the stack machine: rejected for now (decision 3).
- `Temporal` (slice 20), ECMA-402 (slice 21), stage 3 features, concurrent agents.

## Constraints

- Nothing third-party in what ships; `#![forbid(unsafe_code)]`: a frame is data, no unsafe stack
  switching (`AGENTS.md`, "Rules"; decision 3).
- No panics, no `unwrap`, `expect` or unchecked indexing (`AGENTS.md`, "Rules").
- Every loop charged to the step budget and every allocation to the memory budget, a saved frame
  included; a new limit gets a test that reaches it (`AGENTS.md`, "Rules").
- A builtin's module and comments cite its clause (`AGENTS.md`, "Rules").

## Acceptance

`slot task check` exits 0 on `main`, and in that run:

1. The conformance runner's line for slice 7 reads 1,432 cases and 2,771 runs that exist, and every run of the slice passes; no case of the slice is counted as failed or as ended by a budget.
2. No pass count of an earlier slice or of any directory line in `conformance/test262/expected.txt` is lower than before the slice.
3. `conformance/test262/expected.txt` is committed with the lines of that run, and `AGENTS.md` and `README.md` carry the gate's numbers.
4. A case of this slice that is found to need a later slice is moved by a rule line in the `swell-slices` block of the design with its reason, in the same change that moves the counts of both slices; none is skipped silently.
5. The slice's cases also pass in the test mode that collects at every safe point (decisions 4 and 13), so that a frame saved in a generator object is shown to be traced.
