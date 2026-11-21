---
format: aep.planning-md/3
id: epic:slice-01-harness
kind: epic
status: active
title: 'Slice 1, harness: the first engine and the test262 runner'
summary: 'First engine that runs a script, with the test262 run path and the host limits: 31 cases and 62 runs of test/harness (82 by directory; 51 need later slices), and the 68 host requirements marked for slice 1.'
relations:
- depends_on: epic:foundation
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T04:49:58Z", actor: "human:operator", revision: 6, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-21T04:49:58Z", actor: "human:operator", revision: 7, executor: "agent:coordinator"}
---
## Context

Slice 1 of 19, `harness` (`docs/design/engine.md`, section 12): "nothing can be counted before
slice 1". It needs no earlier slice, only the foundation epic: workspace, gate, test262 pin, the
runner that prints counts against an engine that runs nothing, the host interface specification
(`AGENTS.md`, "How the work is run"). It builds the first engine that runs a script: lexer, parser
and compile step for the ES5 grammar, the interpreter, the heap, the builtins the harness files
call, `$262`, `print` and the runner. Governing decisions (`docs/design/engine.md`, decision N): 2
crates, 3 execution model and budgets, 4 values and heap, 5 strings, 9 host interface, 10 ESS
model (starting with this slice), 11 the runner, 13 security.

## Outcome

- `swell-syntax` and `swell-compiler`: lexer, parser and compile step for the ES5 grammar, with a
  nesting counter in the parser (decision 3), with the strict mode grammar and runtime (the runner
  makes a strict run of every case, so no case passes without it). `eval`, `with`, `arguments`, labels and
  automatic semicolon insertion are slice 2's.
- `swell`: a stack-based bytecode interpreter with every ECMAScript frame on the engine's own frame
  stack (frame: instruction index, operand stack, local slots, environment, handler table), the
  step budget charged one unit per instruction and checked at every backward jump, call, return and
  re-entry, and the depth bounds of decision 3, each ending in its outcome.
- Values and the heap (decision 4): the `Value` enum, arenas with `(index, generation)` handles typed
  by kind, plain property maps, the memory budget through one accounting function. Strings in
  Latin-1 or 16-bit form, property keys interned (decision 5).
- The builtins the harness files call (section 12, item 1). `assert.js`, `sta.js` and
  `propertyHelper.js` call `Function.prototype.call.bind`, `Object.getPrototypeOf`,
  `Object.getOwnPropertyDescriptor`, `Object.getOwnPropertyNames`, `Object.defineProperty`,
  `Object.prototype.hasOwnProperty`, `Object.prototype.propertyIsEnumerable`,
  `Object.prototype.toString`, `Array.prototype.join`, `.push`, `.map`, `Array.isArray`,
  `Math.max` and `JSON.stringify`, and use `for-in`, `switch`, `try`, `instanceof` and `typeof`.
- `$262` and `print`, as the harness needs them (decision 11).
- `swell-test262` runs cases (decision 11). `epic:foundation` delivers the counting, the
  per-directory, per-slice and total lines, the comparison with
  `conformance/test262/expected.txt` and `slot task test262`; in its runner one function marks
  every run case as failed. This slice replaces that function: a fresh agent and realm per run,
  `assert.js` and `sta.js` before every case but `raw` ones, the `includes:` files in order (no
  case of this slice is `async`: `doneprintHandle.js` and the async run path are slice 10's),
  strict and non-strict runs, `negative` handling, the
  budgets (first values 2^30 steps and 512 MiB, printed as `budget` when reached), the twenty
  most expensive cases by steps, workers through a flag.
- The host interface, written by hand against its two specifications, which exist before this
  slice starts: the ESS model under `spec/` and the numbered requirements of
  `docs/design/host-interface.md`. This slice meets every requirement marked `[slice 1]`, and
  each story that implements part of the interface names the `HOST-<n>` it closes and moves
  their lines in `conformance/host/requirements.txt` from `omitted` to `test`.
- A conformance target in a workspace of its own (decision 10) that runs the ESS model's
  scenarios against the hand-written interface, as far as this slice's entry points reach.

### Open, to be settled by this epic's first story

- Which parts of the host interface this slice delivers is settled by the `[slice 1]` marks in
  `docs/design/host-interface.md`; a story that finds a mark wrong says so and the document is
  changed first.
- How much of `Object`, `Array`, `Function` and `JSON` the harness files need now. Section 12 gives
  those builtins to slice 3 and lists the calls above for slice 1.
- Which members of `$262` exist in this slice. The design lists `createRealm`, `detachArrayBuffer`,
  `evalScript`, `gc`, `global`, `IsHTMLDDA`, `AbstractModuleSource` and `agent`; several need later
  slices (13, 16, 18, and open item B for `agent`).
- Whether the collector is part of this slice. Section 12 says "the heap" only; the test mode that
  collects at every safe point (decisions 4 and 13) needs it.
- Which parts of `swell-num`, `swell-unicode` and `swell-regexp` this slice takes early. The rule
  is the design's "A part before its slice" (section 12): `swell-num` gets the keyed hash, decimal
  literal parsing and `Number::toString` for radix 10 now; the `Zs` table for white space; identifiers outside
  ASCII and regular expression literals are recorded omissions with a defined refusal until
  slice 5. The decomposition measured what the harness files and the cases need beyond
  that.
- The size of `Value` (expected 16 bytes) and the Rust stack one re-entry needs are not measured
  (design, "Not checked").

## Counts

- Slice 1 (`harness`): 31 cases, 62 runs. The design's table gives 82 cases and 164 runs: that is the
  share of `test/harness/` by directory. Measured by the decomposition on the pinned checkout
  (2026-11-21), 51 of the 82 need features of later slices (23 the `arguments` object and the global
  `Function` through `propertyHelper.js`, 16 ES2015 syntax, 6 `Map`, 2 `Date` and `Math.pow`, 2
  `ArrayBuffer`, 1 a BigInt literal, 1 automatic semicolon insertion). They move by rule lines in the
  `swell-slices` block with their reasons (`story:test262-run-path`); the 22 count lines still sum to
  53,801. Decided the same day by the session that decides for the operator.
- Directories the design names for it (paths under `test/`, cases): `harness` 82. Of the 117 files
  in `test/harness/`, 35 fall after slice 1: they test `asyncHelpers.js`, `deepEqual.js` on maps
  and sets, and others that need later slices.
- Lines of the `swell-slices` block that assign cases to this slice:
  - `dir test/harness/ 1`
  - no `feature` line and no `flag` line.

Source: `docs/design/engine.md`, section 12 and the output under 'How the counts were made'; a count is measured again on the pinned test262 checkout before a story's acceptance uses it (`AGENTS.md`, 'How the work is run').

## Out of scope

- `eval`, `with`, `arguments`, labels, automatic semicolon insertion: slice 2. The 51 cases of
  `test/harness/` that need a later slice: counted there.
- The builtins beyond what the harness calls (`Object`, `Function`, `Array`, `String`, `Boolean`,
  `Error`, `JSON` in full): slice 3.
- `Number`, `Math`, `Date` and the `swell-num` functions: slice 4. Regular expressions and
  `swell-unicode`: slice 5.
- `let`, `const`, destructuring, iterators, `Symbol`: slice 6. Generators: 7. Classes: 8. Promises
  and jobs: 10. Modules: 11.
- The 35 files of `test/harness/` that need later slices: counted in those slices.
- `Temporal` (slice 20, decision 1), ECMA-402 (slice 21, open item A), stage 3 features (the
  `exclude` lines), concurrent agents (open item B).

## Constraints

- Nothing third-party in anything that ships; `#![forbid(unsafe_code)]` in every crate; the root
  `Cargo.lock` lists no package from outside the repository (`AGENTS.md`, "Rules").
- No panics: no `unwrap`, `expect` or unchecked indexing in engine crates; no ambient authority,
  so no file, network, clock, randomness or environment access and no `RandomState` in the engine
  (`AGENTS.md`, "Rules"; decision 13).
- Every loop the engine runs for a script is charged to the step budget, every allocation to the
  memory budget, every recursion is bounded; each limit ends in an outcome the host reads, and a
  new limit gets a test that reaches it (`AGENTS.md`, "Rules").
- Spec first: the host interface is stated as an ESS model under `spec/` before its code
  (`AGENTS.md`, "Rules"; decision 10).
- A builtin's module and comments cite its clause; no Python anywhere (`AGENTS.md`, "Rules").

## Acceptance

`slot task check` exits 0 on `main`, and in that run:

1. The conformance runner's line for slice 1 reads 31 cases and 62 runs that exist, and every run of the slice passes; no case of the slice is counted as failed or as ended by a budget.
2. No pass count of an earlier slice or of any directory line in `conformance/test262/expected.txt` is lower than before the slice.
3. `conformance/test262/expected.txt` is committed with the lines of that run, and `AGENTS.md` and `README.md` carry the gate's numbers.
4. A case of this slice that is found to need a later slice is moved by a rule line in the `swell-slices` block of the design with its reason, in the same change that moves the counts of both slices; none is skipped silently.
5. The script `while(1){}` ends with `Terminated(StepBudget)`, with no panic, abort or hang.
6. A `try { while(1){} } finally { ... }` ends with `Terminated(StepBudget)` and the `finally` block does not run (decision 3: no `catch` or `finally` block runs).
7. A function that calls itself without end, with the host's frame depth limit set, throws a `RangeError` in the script; a script that catches it and recurses again ends with `Terminated(StepBudget)`.
8. A source text of 100,000 nested brackets, with the host's parser nesting limit set, is a parse failure through the host; no Rust stack overflow.
9. An allocation that takes the agent over its memory budget ends the run with `Terminated(MemoryBudget)`.
10. An interrupt handle set from another thread while `while(1){}` runs ends the run with `Terminated(Interrupted)`.
11. A handle whose generation does not match its slot (reached by a unit test on the arena) ends the run with `Terminated(EngineFault)`, and every later call on that agent answers `EngineFault`.
12. A case that exceeds the runner's budgets is a failed case printed as `budget`, and the runner prints the twenty most expensive cases by steps (decision 11).
13. The same script run twice prints the same bytes (decision 13, non-determinism).
14. A test holds the size of `Value` (decision 4; expected 16 bytes, not yet measured).
