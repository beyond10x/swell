---
format: aep.planning-md/3
id: epic:slice-02-language-core
kind: epic
status: draft
title: 'Slice 2, language-core: the ES5-level language'
summary: 'Strict mode, eval, with, arguments, labels and semicolon insertion: 5,244 cases and 9,171 runs.'
relations:
- depends_on: epic:slice-01-harness
revision: 1
---
## Context

Slice 2 of 19, `language-core` (`docs/design/engine.md`, section 12). It needs slice 1: the lexer,
parser, compile step, interpreter, heap and runner it extends. Section 12 puts slices 2 to 5 as the
language and library of ES5, in the order of dependencies. It completes the ES5-level language:
strict mode, `eval`, `with`, `arguments`, labels and automatic semicolon insertion. Governing
decisions (`docs/design/engine.md`, decision N): 2 (`swell-syntax` with early errors, strict mode,
scope analysis), 3 (parser nesting bound, `eval` as a re-entry), 9 (hooks), 11 (the runner), 13.

## Outcome

- `swell-syntax`: strict mode, the early errors of the ES5-level grammar, automatic semicolon
  insertion, labels, scope analysis (decision 2).
- `swell-compiler` and the interpreter: `with`, `arguments`, `eval` (decision 3: a parse that
  exceeds the nesting bound through `eval` is a `RangeError`).
- The host hooks that `eval` reaches (decision 9): `HostEnsureCanCompileStrings` has the seven
  parameters of the pinned draft; `HostGetCodeForEval` is new in it (design, "What the brief
  assumed").
- The cases of `test/language/` counted here by the rule `dir test/language/ 2`: 5,244 cases, of
  which the design names `expressions/compound-assignment`, `expressions/assignmenttargettype`,
  `expressions/object` and `eval-code` as the directories it raises most.
- Parse negatives (`negative: phase: parse`, type `SyntaxError`; 4,680 cases in all slices) fail
  before anything runs, as the runner requires (decision 11).

### Open, to be settled by this epic's first story

- Which slice wires `HostEnsureCanCompileStrings` and `HostGetCodeForEval`. `eval` is here and
  `Function` is slice 3; the design assigns no hook to a slice.
- The same open items as slice 1 where they bear on this slice: where `swell-unicode` (identifiers,
  white space) and `swell-num` (numeric literals) first exist.

## Counts

- Slice 2 (`language-core`): 5,244 cases, 9,171 runs (section 12 table).
- Directories the design names for it, as it writes them (cases): `expressions/compound-assignment`
  407, `expressions/assignmenttargettype` 318, `expressions/object` 282, `eval-code` 253.
  Inventory, in all slices: `language/expressions` 11,156 cases, `language/statements` 9,399.
- Lines of the `swell-slices` block that assign cases to this slice:
  - `dir test/language/ 2`
  - no `feature` line and no `flag` line.

Source: `docs/design/engine.md`, section 12 and the output under 'How the counts were made'; a count is measured again on the pinned test262 checkout before a story's acceptance uses it (`AGENTS.md`, 'How the work is run').

## Out of scope

- Everything under `test/language/` that a longer `dir` line, a `feature` line or a `flag` line
  moves to a later slice: regular expression literals (slice 5), `let`, `const`, destructuring,
  arrows, templates, `for-of` (6), generators (7), classes, `super`, `new.target` (8), async
  functions and `for await` (10), modules, `import`, `export`, dynamic `import` (11), tail calls
  (17), `async` and `module` flagged cases (10 and 11).
- The `Object`, `Function`, `Array` and other builtins: slice 3. `Number`, `Math`, `Date`: slice 4.
- Annex B syntax and semantics (`annexB/`): slice 18.
- `Temporal` (slice 20), ECMA-402 (slice 21), stage 3 features (the `exclude` lines).

## Constraints

- Nothing third-party in what ships; `#![forbid(unsafe_code)]` in every crate; no panics, no
  `unwrap`, `expect` or unchecked indexing in engine crates (`AGENTS.md`, "Rules").
- Parser, scope analysis and compile step walk hostile source: the parser's nesting bound holds for
  all three, and every recursion is bounded (`AGENTS.md`, "Rules"; decision 3).
- Every loop charged to the step budget, every allocation to the memory budget; a new limit gets a
  test that reaches it (`AGENTS.md`, "Rules").
- A deliberate omission is recorded where the gate reads it, never left implicit
  (`AGENTS.md`, "Rules").

## Acceptance

`slot task check` exits 0 on `main`, and in that run:

1. The conformance runner's line for slice 2 reads 5,244 cases and 9,171 runs that exist, and every run of the slice passes; no case of the slice is counted as failed or as ended by a budget.
2. No pass count of an earlier slice or of any directory line in `conformance/test262/expected.txt` is lower than before the slice.
3. `conformance/test262/expected.txt` is committed with the lines of that run, and `AGENTS.md` and `README.md` carry the gate's numbers.
4. A case of this slice that is found to need a later slice is moved by a rule line in the `swell-slices` block of the design with its reason, in the same change that moves the counts of both slices; none is skipped silently.
5. `eval` of a source text nested deeper than the host's parser nesting limit (for example 100,000 nested brackets) throws a `RangeError` that the script can catch (decision 3); no Rust stack overflow.
