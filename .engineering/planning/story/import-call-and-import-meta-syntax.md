---
format: aep.planning-md/3
id: story:import-call-and-import-meta-syntax
kind: story
status: draft
title: import() and import.meta parsed whole with their early errors, then held until the stories that run them
summary: ImportCall with options and import.meta (Module goal only) in the tree with their early errors and assignment-target rules; held dynamic import and import.meta; compiler arms answer CompileError::Tree. Order 2; 152 cases (130 failing on the base).
relations:
- decomposes: epic:slice-11-modules
- depends_on: story:module-syntax-and-early-errors
scope:
- confidence: cited
  path: crates/compiler/src/expression.rs
- confidence: cited
  path: crates/compiler/src/resolve.rs
- confidence: cited
  path: crates/syntax/src/ast.rs
- confidence: cited
  path: crates/syntax/src/early.rs
- confidence: cited
  path: crates/syntax/src/parser/expression.rs
- confidence: cited
  path: crates/syntax/src/parser/mod.rs
- confidence: cited
  path: crates/syntax/src/parser/statement.rs
- confidence: inferred
  path: crates/syntax/tests/import_call.rs
- confidence: inferred
  path: crates/syntax/tests/sources/early-errors/import-call-as-assignment-target.js
- confidence: inferred
  path: crates/syntax/tests/sources/early-errors/import-call-with-spread.js
- confidence: inferred
  path: crates/syntax/tests/sources/early-errors/import-call-with-three-arguments.js
- confidence: inferred
  path: crates/syntax/tests/sources/early-errors/import-call-without-argument.js
- confidence: inferred
  path: crates/syntax/tests/sources/early-errors/import-meta-in-script.js
- confidence: inferred
  path: crates/syntax/tests/sources/early-errors/new-import-call.js
- confidence: inferred
  path: crates/syntax/tests/sources/held/dynamic-import-options.js
- confidence: inferred
  path: crates/syntax/tests/sources/held/dynamic-import.js
- confidence: inferred
  path: crates/syntax/tests/sources/held/module-import-meta.js
- confidence: cited
  path: crates/syntax/tests/sources/refused/dynamic-import.js
- confidence: cited
  path: crates/syntax/tests/sources/refused/import-meta.js
revision: 8
---
## Context

Slice 11 brings modules (`epic:slice-11-modules`; `docs/design/engine.md`, section 12, item 11). This story parses `import(...)` and `import.meta` whole, with their early errors, and holds both until the stories that run them. On the base (`f962c324`) the parser refuses both at the first token after `import`: `crates/syntax/src/parser/expression.rs` lines 1227 and 1228 (a primary expression) and `crates/syntax/src/parser/statement.rs` lines 245 (`Some(Punct::Dot)`) and 246 (`Some(Punct::LParen)`) of the arm `Keyword::Import` (an expression statement that starts with `import`; line 247, the declaration branch, is `story:module-syntax-and-early-errors`'s). A refusal ends the parse, so a source whose `import(...)` is itself an early error (no argument, a spread, three arguments, `new import(x)`, an escaped `import`, `import(x) = 1`) fails as a refusal and not as the `SyntaxError` the case asks for; and `import.meta` in a Script, which is the grammar's `SyntaxError` (`#sec-module-semantics-static-semantics-early-errors`: ImportMeta outside the Module goal), is refused too.

The count. 152 cases (291 runs) of slice 11 are this story's (`docs/plan/slice-11/story-import-call-and-import-meta-syntax.cases.txt`): 148 with a parse `SyntaxError` as their `negative` (143 of them under `test/language/expressions/dynamic-import/syntax/invalid/`) and 4 that run, all in the Script goal. On the base run (the last full run on `f962c324`) 22 of them pass (42 runs: the grammar's error comes before the refusal) and 130 fail: 235 runs on the refusal `dynamic import`, 6 on `import.meta`, 8 on a counted refusal caught by `eval`. Every one passes once the grammar and the early errors are this story's and the two constructs are held (`engine.md` section 12, "A part before its slice": a held construct is refused only when the source has no error). The 485 cases that evaluate `import()` are `story:dynamic-import`'s and stay failing, on the held refusal.

Design it follows: ECMA-262 13.3.10 (`#sec-import-calls`: ImportCall `import ( AssignmentExpression ,opt )` and `import ( AssignmentExpression , AssignmentExpression ,opt )`), 13.3.12 (`#sec-meta-properties`, ImportMeta), the early errors of `#sec-module-semantics-static-semantics-early-errors` and `#sec-static-semantics-assignmenttargettype` (neither is a valid assignment target); `engine.md` section 11, "Refusals", and section 12.

## What later stories reuse

The two nodes of the tree and their early errors; the two holds, which `story:import-meta` and `story:dynamic-import` lift, each by deleting its `hold` call and its line of `HOLDS` only (the held sources of this story then parse as valid sources do, by the test `story:module-syntax-and-early-errors` made goal-aware, so the lifting stories write no source and no test of `crates/syntax/tests/`).

## Domain relations

- None new: an ImportCall is an expression of the tree; the relation of a load it requests to its referrer is `story:dynamic-import`'s.

## Change

Failing tests first: the tests and sources below are written before the code and fail on the base.

- `crates/syntax/src/ast.rs`: two variants at the end of `Expr` (`NewTarget`, line 318, is its last on the base): `ImportCall { specifier: ExprId, options: Option<ExprId> }` and `ImportMeta`; the walks of the tree that list every variant (`ast.rs` lines 1204 and 1411, beside `Expr::NewTarget`) get them.
- `crates/syntax/src/parser/expression.rs`, the hunk of lines 1222 to 1230 only, and `crates/syntax/src/parser/statement.rs`, the branches at lines 245 and 246 only: the grammar of ImportCall (one or two AssignmentExpressions, a trailing comma, `[+In]` inside the parentheses, no spread; `new import(...)` and an `import` without `(` or `.` a `SyntaxError`; `import` written with an escape a `SyntaxError`, `#sec-keywords-and-reserved-words`) and of `import.meta` (`meta` written with an escape a `SyntaxError`; in the Script goal and in eval code of a Script, which `parse_eval` answers, a `SyntaxError`; in the Module goal of `story:module-syntax-and-early-errors` a node); neither is a valid assignment target, a valid target of `++`/`--`, of a `for-in`/`for-of` head or of a destructuring pattern (AssignmentTargetType invalid; `crates/syntax/src/early.rs`). The two branches of `statement.rs` parse an expression statement, so that the nodes are made in `parser/expression.rs` only, and there, in the hunk of lines 1222 to 1230, stand `self.hold("dynamic import", 11)` and `self.hold("import.meta", 11)` (the `Parser::hold` of `story:module-syntax-and-early-errors`), the one place each lifting story deletes; `HOLDS` (`crates/syntax/src/parser/mod.rs` line 67) gets the lines `("dynamic import", 11)` and `("import.meta", 11)` after the two lines that story adds, so it reads, in order, `namespace import`, `top-level await`, `dynamic import`, `import.meta`.
- `swell-compiler`: the arms of the two variants in the matches over `Expr`, beside `Expr::NewTarget` (`crates/compiler/src/expression.rs` line 342 and `crates/compiler/src/resolve.rs` line 1746), each answering `CompileError::Tree` (`crates/compiler/src/lib.rs` line 678): a held source never reaches the compiler, so the arm is the place `story:import-meta` and `story:dynamic-import` fill.
- Sources (the held sources are read by `held_sources_answer_their_first_construct_still_held`, `crates/syntax/tests/sources.rs`, as `story:module-syntax-and-early-errors` leaves it: a file name that starts with `module-` is parsed in the Module goal; this story does not write `sources.rs`): `crates/syntax/tests/sources/refused/dynamic-import.js` moves to `crates/syntax/tests/sources/held/dynamic-import.js` with the header `held "dynamic import"` (a Script); new `held/dynamic-import-options.js` (`import("a", { with: { type: "json" } },);`, the same header) and `held/module-import-meta.js` (`export default import.meta;`, header `held "import.meta"`); `refused/import-meta.js` moves to `crates/syntax/tests/sources/early-errors/import-meta-in-script.js` with the header `SyntaxError "<message>"` (a Script); new sources under `early-errors/`: `import-call-without-argument.js`, `import-call-with-spread.js`, `import-call-with-three-arguments.js`, `new-import-call.js` and `import-call-as-assignment-target.js`.
- Tests in a new module `crates/syntax/tests/import_call.rs`, 6, each on an error that stays an error once the holds are lifted (the holds are shown by the held sources only, so lifting a hold changes no test here): `an_early_error_beside_an_import_call_is_reported`, `import_call_is_no_assignment_or_update_target` (`import(x)++`, `for (import(x) of y)`, `[import(x)] = y`), `new_import_and_bare_import_are_syntax_errors`, `import_meta_is_no_assignment_target_in_a_module` (through `parse_module`), `escaped_import_and_escaped_meta_are_syntax_errors`, `import_meta_in_eval_code_of_a_script_is_a_syntax_error` (through `parse_eval`).
- `swell`: `crates/swell/tests/` has no test that reads the two refusal texts on the base (`git grep -n 'slice 11' -- crates/swell/tests` prints only the comments of `emulated_host.rs` line 9 and `hooks.rs` line 4); a refusal caught by `eval` is still counted, held as it is now, by `Agent::refusals` (`engine.md` section 11, "Refusals").
- Rules (the `AGENTS.md` of the folder that holds the repository, and `AGENTS.md`, "Rules" and "Commands and the gate"): no `unsafe` (`#![forbid(unsafe_code)]`), no third-party crate, no panic, no `unwrap`, `expect` or unchecked indexing in library code; every loop of the change (a walk of a module graph, of a list of entries, of a namespace's names) is charged to the step budget and every allocation to the memory budget, and a new limit gets a test that reaches it; a builtin's module and comments cite its clause. Time ("Tests stay fast"): a single test is terminated at 60 s and counts as failed, and the gate fails when the wall time of its test steps passes 120 s; the unit times each test before it adds it and names in its report every test over 5 s with its time; one integration-test binary per crate (a new test file is a module of the crate's `tests/main.rs`, written again by `sh taskfiles/test-main.sh write`); an expensive product several tests read (a parsed corpus, a fixture tree, a built part of the test262 checkout) is made once, as a committed file or one shared fixture, never per test and never in a `static`; no test makes a network request or a model call. Builds and tests go through the pool: `cargo` is the shim of the pool, anything else that compiles or tests is prefixed `slot`; a pool job ends after 10 minutes unless it names its time with `slot --timeout` or `SLOT_TIMEOUT`, at most 30 minutes; a script that only measures, probes or waits starts without the `slot` prefix. No Python in anything committed (tools, tasks, gates, tests, helpers): Rust or POSIX sh. The unit runs the tests of its own change, never the whole gate and never a whole-workspace test run; intermediate files go in the scratch directory; no `cd ... &&` with a write or a delete.
- Report: the sum of the CPU seconds of the story's new tests in the default mode and in the mode `collect-every-safe-point` (each test run alone, user plus system), and the wall seconds of its part run when it has one; no story bounds the CPU seconds of a whole gate.
- Generated files: the generated `tests/main.rs` of every crate and `website/data/status.json` are in no story's scope. A unit that adds a test module writes `tests/main.rs` again with `sh taskfiles/test-main.sh write`; a unit that meets a conflict in a generated file takes either side and writes it again; the coordinator writes both again on the join (`sh taskfiles/test-main.sh write`, `slot --local task site-data`).
- Line numbers are those of `f962c324`; anchors are found by name.
- Size: about 600 lines (the grammar 150, the early errors 100, the tests and sources 350): one unit.

Shared with `story:module-code-compiles`, the other story of order 2, each in its own hunks: `crates/compiler/src/expression.rs` (this story: two arms beside `Expr::NewTarget`, line 342; that story: none in that match) and `crates/compiler/src/resolve.rs` (this story: two arms beside `Expr::NewTarget`, line 1746; that story: the module scope, new functions after the script's scope, and no arm of that match). That story writes no file of `crates/syntax/`. Later in the slice: `story:import-meta` and `story:dynamic-import` each replace one of this story's compiler arms and delete one hold call and one line of `HOLDS` (lines 4 and 3, in orders 5 and 6).

## Requirements

None of `conformance/host/requirements.txt` moves.

Writes: `crates/syntax/src/{ast,early}.rs`, `crates/syntax/src/parser/{mod,expression,statement}.rs` (the hunks named above), `crates/compiler/src/{expression,resolve}.rs` (the two arms each), `crates/syntax/tests/import_call.rs` (new), `crates/syntax/tests/sources/refused/{dynamic-import,import-meta}.js` moved (to `held/dynamic-import.js` and `early-errors/import-meta-in-script.js`), `crates/syntax/tests/sources/held/{dynamic-import-options,module-import-meta}.js` (new), `crates/syntax/tests/sources/early-errors/` (the five new sources). Not written: `crates/syntax/src/lib.rs`, `parser/module.rs`, the lexer and `crates/syntax/tests/sources.rs` (`story:module-syntax-and-early-errors`), `crates/swell/`, `crates/test262/`, `conformance/`, `docs/design/`.

Order 2, beside `story:module-code-compiles`; it depends on `story:module-syntax-and-early-errors` (the Module goal, `Parser::hold`, the held sources of the Module goal).

## Acceptance

`cargo nextest run --locked -p swell-syntax --test main` exits 0, and `-E 'test(/^import_call::/)'` runs 6 tests (none on the join it starts from); `ls crates/syntax/tests/sources/held | wc -l` prints `31` (`28` on that join, `25` on the base), `ls crates/syntax/tests/sources/early-errors | wc -l` prints `404` (`398` on that join, `396` on the base) and `ls crates/syntax/tests/sources/refused | wc -l` prints `5` (`7` on that join, `9` on the base); `cargo nextest run --locked -p swell-compiler --test main` and `cargo nextest run --locked -p swell --test main -E 'test(/^eval::/) | test(/^dynamic_function::/) | test(/^scripts::/)'` exit 0. `grep -rnE 'refuse[_a-z]*\("[^"]*", 11' crates/syntax/src | wc -l` prints `0` (`4` on that join, `6` on the base); `grep -c '", 11)' crates/syntax/src/parser/mod.rs` prints `4` (the lines of `HOLDS`; `2` on that join). `slot task clippy` exits 0.

test262. The full run is not the unit's. In one pool job on the build node: `slot --timeout 30m --fetch .out/test262-result sh docs/plan/slice-11/run-part.sh docs/plan/slice-11/story-import-call-and-import-meta-syntax.cases.txt docs/plan/slice-11/guard-syntax.cases.txt`; then on the workstation, with the base run of the join the unit starts from copied to `.out/test262-base/runs.tsv`, `sh docs/plan/slice-11/check.sh docs/plan/slice-11/story-import-call-and-import-meta-syntax.cases.txt .out/test262-base/runs.tsv .out/test262-result/runs.tsv` exits 0 and prints `passed 1137 bound 1137 B 1007 F 130 R 0` (the part holds 1,137 cases and pulls in none by `./`: the 152 of the list, 22 passing and 130 failing on the base, and the 985 of `guard-syntax.cases.txt`, every one passing there; `check.sh` counts in `B` every case of the part that passes on the base run it reads, and the part holds no case of another story's list, so the line is the same on `f962c324` and on the join).

## Out of scope

Evaluating `import()` and its `LoadRequest` (`story:dynamic-import`); evaluating `import.meta` and its hooks (`story:import-meta`); the Module goal, import and export declarations (`story:module-syntax-and-early-errors`); `import.source`, `import.defer` and the other stage 3 phases (excluded by the design, the epic's "Out of scope").
