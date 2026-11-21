---
format: aep.planning-md/3
id: epic:slice-02-language-core
kind: epic
status: implemented
title: 'Slice 2, language-core: the ES5-level language'
summary: 'The rest of the ES5-level language: eval, with, arguments, labels, semicolon insertion and the global Function; 5,245 cases on main after slice 1, about 4,270 after the moves.'
relations:
- depends_on: epic:slice-01-harness
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T13:41:53Z", actor: "human:operator", revision: 4, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-21T13:41:54Z", actor: "human:operator", revision: 5, executor: "agent:coordinator"}
- {from: "active", to: "implemented", at: "2026-11-21T19:28:42Z", actor: "human:operator", revision: 6, decided_on: {"recorded":{"test_result":1}}, executor: "agent:coordinator"}
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
- `swell-compiler` and the interpreter: `with`, `arguments`, `eval`, and the global `Function`
  with its constructor (`AGENTS.md`, "Slice 2 delivers the global `Function`"). A parse that
  exceeds the nesting bound through `eval` or `Function` is a `SyntaxError` whose message
  names `parser_nesting` (`HOST-139`, `HOST-140`, `HOST-141`).
- The host hooks that `eval` reaches (decision 9): `HostEnsureCanCompileStrings` has the seven
  parameters of the pinned draft; `HostGetCodeForEval` is new in it (design, "What the brief
  assumed").
- The cases of `test/language/` counted here by the rule `dir test/language/ 2`: 5,245 cases on
  `main` after slice 1 (`test262-slice 02 5245 9173 0 0 5245 2125 3120 0`), of
  which the design names `expressions/compound-assignment`, `expressions/assignmenttargettype`,
  `expressions/object` and `eval-code` as the directories it raises most.
- Parse negatives (`negative: phase: parse`, type `SyntaxError`; 4,680 cases in all slices) fail
  before anything runs, as the runner requires (decision 11).

### Settled at the decomposition (2026-11-21)

- Slice 2 wires `HostEnsureCanCompileStrings` and `HostGetCodeForEval`: `eval` and `Function`
  are both here (`HOST-116`, `HOST-117`, `HOST-118`).
- `swell-unicode` and `swell-num` exist since slice 1.
- Parts taken before their slice, each complete for what it covers (design, section 12):
  plain method definitions and shorthand properties; `Number` with `toString` in every radix;
  `Boolean`; `isNaN` and `isFinite`; `String.prototype.toString` and `valueOf`. 365 cases of
  the slice stop first on `Number` or `Boolean`, 45 on a method or shorthand.
- About 1,000 cases the directory rule counts here need syntax or builtins of a later slice
  (`let`, `const`, classes, arrows, regular expressions and others). They move by rule lines,
  each with its evidence in a committed list that a gate task checks
  (`story:slice-02-conformance-close`); about 4,270 cases stay.
- The engine counts the refusals it raises, and the runner fails every run in which the count
  moved, so that a refusal a script catches through `eval` or `Function` is never a pass
  (`HOST-164`).
- `$262.evalScript` is delivered by the runner in this slice.

## Counts

- Slice 2 (`language-core`): 5,245 cases, 9,173 runs on `main` after slice 1; the design's table
  said 5,244 and 9,171 before wave 5 of slice 1. The stories move the number: up by the 22
  harness cases that `Function`, `arguments` and methods unblock, down by the cases that need a
  later slice.
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
- The `Object`, `Array` and other builtins, and what remains of `Function`: slice 3. `Math`,
  `Date` and the rest of `Number`: slice 4.
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

1. The conformance runner's line for slice 2 reads the cases and runs the `swell-slices` block gives the slice after the moves of `story:slice-02-conformance-close` (measured there; about 4,270 cases), and every run of the slice passes; no case of the slice is counted as failed or as ended by a budget.
2. No pass count of an earlier slice, of any slice line or of any directory line in `conformance/test262/expected.txt` is lower than before the slice, except by runs that passed only because a refusal went unseen (a script that caught it, or `eval` being undefined); each such run is listed in the wave record that removes it.
3. `conformance/test262/expected.txt` is committed with the lines of that run, and `AGENTS.md` and `README.md` carry the gate's numbers.
4. A case of this slice that is found to need a later slice is moved by a rule line in the `swell-slices` block of the design with its reason, in the same change that moves the counts of both slices; none is skipped silently.
5. `eval` and `Function` of a source text nested deeper than the host's parser nesting limit throw a `SyntaxError` whose message names `parser_nesting`, which the script can catch (`HOST-139`, `HOST-140`, `HOST-141`); no Rust stack overflow (`HOST-152`, `HOST-163`).
