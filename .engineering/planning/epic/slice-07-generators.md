---
format: aep.planning-md/3
id: epic:slice-07-generators
kind: epic
status: active
title: 'Slice 7, generators: frames saved in generator objects'
summary: 'Generators, with frames saved in objects and resumed: 1,491 cases and 2,841 runs.'
relations:
- depends_on: epic:slice-06-bindings-and-iteration
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T11:15:31Z", actor: "human:operator", revision: 4, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-23T11:15:31Z", actor: "human:operator", revision: 5, executor: "agent:coordinator"}
---
## Context

Slice 7 of 19, `generators` (`docs/design/engine.md`, section 12): "frames saved in objects". It is
delivered after slices 1, 2, 3, 6 and 5 and before slice 10 (section 12, "The order of delivery":
1, 2, 3, 6, 5, 7, 10, 4, 9, 8), and it needs the iterator protocol and `for`-`of` of slice 6.
Slice 10 (`async` functions, the next gap of the first consumer's test harness) "needs 7's
suspended frames". It is the first use of the decision that every ECMAScript frame is data on the
engine's own frame stack: `yield` moves the frame into the generator object and a resumption moves
it back. Governing decisions (`docs/design/engine.md`, decision N): 3 (the execution model, with
the paragraphs "Suspended frames"), 4 (the collector's roots and the accounting), 2, 13.

## Outcome

- Generator functions and generator objects: `yield` and `yield*` move the whole frame value
  (instruction index, operand stack, local slots, the environments it pushed) into the object that
  suspended it, and a resumption moves it back; the handler table stays with the function's code
  and stays valid through the saved index ("Suspended frames", item 1). No host thread, no Rust
  stack kept alive, no unsafe stack switching.
- `%GeneratorFunction%`, `%GeneratorFunction.prototype%` and `%GeneratorPrototype%` with `next`,
  `return` and `throw`; generator declarations, expressions and methods of object literals; the
  `GeneratorFunction` constructor through the dynamic-function path and the host's hook (section
  12, item 7; the `dir` lines below).
- A suspended frame is held in a box by the object that suspended it and traced through that
  object's kind site, not as a root; its charge moves with it and is credited once, when the
  generator completes or the object is swept; it does not count against `frame_depth` while it is
  suspended ("Suspended frames", items 2 to 4).
- The steps a generator runs go to the entry that resumes it; a resumption from a builtin is one
  re-entry of the call path with `STACK_BYTES_PER_REENTRY` unchanged; a chain of `yield*` of n
  levels holds n frames and takes no Rust frame per level ("Suspended frames", items 5 and 6).
- `HOST-73`, the one requirement of the host interface pending for slice 7, is met.

### What the first reading of the epic left open, and where it is settled

- How a suspended frame is accounted to the memory budget and how a generator that is never
  resumed is released: decision 3, "Suspended frames", items 2 and 3, written for this slice.
- Which features with no `swell-slices` line fall inside this slice: none. 38 features have no
  line; among the cases that use generators only `globalThis` (4 cases), `cross-realm` (3) and
  `Math.sumPrecise` (1) appear, and none of the three belongs to slice 7.

## Counts

- Slice 7 (`generators`): 1,491 cases and 2,841 runs (section 12 table;
  `conformance/test262/expected.txt`, `test262-slice 07 1491 2841 0 0 1491 6 1485 0` on the code
  of release `0.7.0` with `matchAll`: 6 cases pass, 1,485 fail). The count is by the order of
  delivery: a case is counted in the slice delivered last among the slices it needs.
- The 1,485 failing cases by the story that makes them pass (`docs/plan/slice-07/`, with the scan
  and its command in `scan.awk`): 114 negative parse cases, 978 cases of generators that run, 49 of
  `yield*`, 281 of generator methods (one of them a negative parse case that needs the grammar of methods), 21 of the constructor, 2 that need both `yield*` and methods,
  and 40 that need `class` or `super` and move to slice 8 at the close.
- Directories the design names for it (paths under `test/`, cases): `expressions/object` 303,
  `expressions/generators` 288, `statements/generators` 266.
- Lines of the `swell-slices` block that assign cases to this slice:
  - `dir test/built-ins/GeneratorPrototype/ 7`
  - `dir test/built-ins/GeneratorFunction/ 7`
  - `dir test/language/expressions/generators/ 7`
  - `dir test/language/statements/generators/ 7`
  - `dir test/language/expressions/yield/ 7`
  - `feature generators 7`
  - no `flag` line.

A count is measured again on the pinned test262 checkout before a story's acceptance uses it
(`AGENTS.md`, "How the work is run").

## Out of scope

- Async functions, async generators, `await`, `for await`, `AsyncGeneratorPrototype` and its request
  queue (27.6.3): slice 10.
- Classes whose methods are generators: counted in slice 8, which is delivered after slice 7.
- `Iterator` helpers: slice 15.
- A register machine, or any change to the stack machine: rejected for now (decision 3).
- `Temporal` (slice 20), ECMA-402 (slice 21), stage 3 features, concurrent agents.

## Constraints

- Nothing third-party in what ships; `#![forbid(unsafe_code)]`: a frame is data, no unsafe stack
  switching (`AGENTS.md`, "Rules"; decision 3).
- No panics, no `unwrap`, `expect` or unchecked indexing (`AGENTS.md`, "Rules").
- Every loop charged to the step budget and every allocation to the memory budget, a saved frame
  included; a new limit gets a test that reaches it (`AGENTS.md`, "Rules").
- A step stands for a bounded amount of work on every path a generator adds ("Suspended frames",
  item 6): a resumption, a `yield`, a level of `yield*`.
- Nothing is added to the frame of the run loop: a new instruction does its work in a function of
  its own (`AGENTS.md`, "Commands and the gate").
- A builtin's module and comments cite its clause (`AGENTS.md`, "Rules").

## Acceptance

`slot task check` exits 0 on `main`, and in that run:

1. The conformance runner's line for slice 7 reads `test262-slice 07 <c> <r> 0 0 <c> <c> 0 0` with the cases and runs the design's table states for the slice after its close, and every run of the slice passes; no case of the slice is counted as failed or as ended by a budget.
2. No pass count of an earlier slice or of any directory line in `conformance/test262/expected.txt` is lower than before the slice.
3. `conformance/test262/expected.txt` is committed with the lines of that run, and `AGENTS.md` and `README.md` carry the gate's numbers.
4. A case of this slice that is found to need a later slice is moved by a rule line in the `swell-slices` block of the design with its reason, in the same change that moves the counts of both slices; none is skipped silently.
5. The engine's own tests of generators pass in the test mode that collects at every safe point (decisions 4 and 13), and one of them shows a frame saved in a generator object traced through that object and credited once.
6. A generator that is running and one that is suspended are each ended by `Terminated(StepBudget)`, `Terminated(MemoryBudget)` and `Terminated(Interrupted)` with no `finally` run and nothing left charged, and `reentry_depth` and `frame_depth` are each reached through generators by a test.

## Stories

Decomposed on 2026-11-23 into six stories, read by the four plan critics in two rounds
(`review-result:slice-07-round-1-*`: 15 findings with 2 blockers on the first draft, which was then
cut in two; `review-result:slice-07-round-2-*`: 17 findings with 3 blockers on the six, 16 applied)
and by two of them a third time for the three stories that had a blocker
(`review-result:slice-07-round-3-*`: 2 findings, both applied). The design text "Suspended
frames" was written before the first story that needs it. In the order of their dependencies:

1. `generator-syntax-and-early-errors`
2. `generator-functions-and-suspended-frames` (after it, slice 10 can be decomposed)
3. `generator-function-constructor`, `yield-star-delegation`
4. `generator-methods-in-object-literals` (after `yield-star-delegation`: both write
   `crates/syntax/src/parser/expression.rs` and `crates/syntax/tests/sources.rs`)
5. `slice-07-conformance-close` (after `slice-05-conformance-close`)
