---
format: aep.planning-md/3
id: story:module-syntax-and-early-errors
kind: story
status: draft
title: 'The parser''s Module goal: import and export declarations of every form, attributes and module early errors, with namespace imports and top-level await held'
summary: swell_syntax::parse_module, the Module goal (strict, no HTML-like comments), every import and export form with attributes and string names, the five lists of entries, module early errors; import and export in a Script become SyntaxErrors; holds namespace import and top-level await. Order 1; 2 cases.
relations:
- decomposes: epic:slice-11-modules
scope:
- confidence: cited
  path: crates/syntax/src/ast.rs
- confidence: cited
  path: crates/syntax/src/early.rs
- confidence: cited
  path: crates/syntax/src/error.rs
- confidence: cited
  path: crates/syntax/src/lexer.rs
- confidence: cited
  path: crates/syntax/src/lib.rs
- confidence: cited
  path: crates/syntax/src/parser/mod.rs
- confidence: inferred
  path: crates/syntax/src/parser/module.rs
- confidence: cited
  path: crates/syntax/src/parser/statement.rs
- confidence: cited
  path: crates/syntax/tests/main.rs
- confidence: inferred
  path: crates/syntax/tests/modules.rs
- confidence: inferred
  path: crates/syntax/tests/sources/early-errors/export-declaration.js
- confidence: inferred
  path: crates/syntax/tests/sources/early-errors/import-declaration.js
- confidence: cited
  path: crates/syntax/tests/sources/refused/export-declaration.js
- confidence: cited
  path: crates/syntax/tests/sources/refused/import-declaration.js
revision: 3
---
## Context

Slice 11 brings modules (`epic:slice-11-modules`; `docs/design/engine.md`, section 12, item 11). This story is the first of order 1: the parser's Module goal. On the base (`f962c324`) `swell-syntax` has one goal, Script, and refuses every module construct: `crates/syntax/src/parser/statement.rs` line 238 (`export declaration`, slice 11) and lines 239 to 248 (the arm `Keyword::Import`: `import.meta`, `dynamic import`, `import declaration`); the lexer refuses `<!--` and `-->` as an `HTML-like comment` of slice 18 (`crates/syntax/src/lexer.rs` line 318), which in the Module goal are no comment at all (ECMA-262 `#sec-html-like-comments`: the extension holds for Script only). `HOLDS` (`crates/syntax/src/parser/mod.rs` line 67) is empty and `Parser::hold` was deleted with the last hold of slice 8; the field `held` of the parse (`parser/mod.rs` line 147) and the test of `tests/sources/held/` (`crates/syntax/tests/sources.rs` line 263) remain.

The count. Of the 1,281 cases (1,833 runs) of slice 11 on the base run (the last full run on `f962c324`, `test262-slice 11 1281 1833 0 0 1281 23 1258 0`), 2 are this story's (`docs/plan/slice-11/story-module-syntax-and-early-errors.cases.txt`): `test/language/global-code/export.js` and `import.js`, Script-goal cases whose `negative` is a parse `SyntaxError` and which fail on the base on the refusal (`export declaration`, `import declaration`); they pass once a declaration in a Script is the grammar's `SyntaxError`. Every other case of the slice that this story's grammar decides (180 module cases with a parse `SyntaxError` among them) is a module run, which the runner makes from `story:module-records-loading-and-module-runs` on: the scan (`docs/plan/slice-11/scan.awk`) puts each case in the first story, in the order of the edges, whose closure holds every part it needs, and a module run needs that story. This story is shown by the tests of `swell-syntax`.

Design it follows: ECMA-262 16.2 (`#sec-modules`: Module, ModuleItem, ImportDeclaration with `with` attributes, ExportDeclaration, ModuleExportName with string names, the static semantics ImportEntries, ExportEntries, ModuleRequests, the early errors of `#sec-module-semantics-static-semantics-early-errors` and `#sec-imports-static-semantics-early-errors`, `#sec-exports-static-semantics-early-errors`); `docs/design/host-interface.md` section 9 (`parse_module`, line 928 on the base); `engine.md` section 11, "Refusals", and section 12, "A part before its slice".

## What later stories reuse

The entry `swell_syntax::parse_module` and its tree: the module items, and per module the lists ModuleRequests (specifier and attributes in source order, each specifier once with its attributes), ImportEntries, LocalExportEntries, IndirectExportEntries and StarExportEntries, computed once at parse time. `story:module-code-compiles` compiles that tree; `story:module-records-loading-and-module-runs` reads ModuleRequests; `story:module-linking-and-evaluation` reads the entries; `story:module-namespace-objects` and `story:top-level-await` lift the two holds this story adds.

## Domain relations

- A module (the parsed tree) → its module requests: one to many, ordered by first appearance in the source, a specifier with one set of attributes once — inferable (inferred from ECMA-262 `#sec-static-semantics-modulerequests`; no ess/1 document declares it: `spec/domains/host.yaml` holds the attributes of a load (`swell.host.ModuleLoad`, field `attributes`) and no tree, and ESS does not restate ECMA-262, `AGENTS.md`, "Rules").
- A module → its import and export entries: one to many each, owned by the module's tree — inferable (inferred from ECMA-262 `#sec-importentry-record` and `#sec-exportentry-record`; no ess/1 document declares them, for the same reason).

## Change

Failing tests first: the tests and sources below are written before the code and fail on the base (no `parse_module`; the declarations refused).

- `crates/syntax/src/lib.rs`: `pub fn parse_module(source: &[u16], options: ParseOptions) -> Result<Module, Vec<SyntaxError>>` and `parse_module_with` (after `parse_script_with`, line 168, in their form), and the tree type `Module` beside `Script` (line 109): its items and the five lists of entries. The Module goal is strict from its start, has no `return` at its top, and lexes `<!--` and `-->` as punctuators (`crates/syntax/src/lexer.rs` line 318: the refusal holds in the Script goal only; the lexer gets the goal as a flag of its state).
- `crates/syntax/src/ast.rs`: the module items (`Stmt` line 587 gets no variant; a `ModuleItem` enum of its own, `Import(ImportDeclaration)`, `Export(ExportDeclaration)`, `Stmt(StmtId)`), the declarations and `ModuleExportName` (an identifier or a string literal, the string checked for well-formed UTF-16, `#sec-static-semantics-isstringwellformedunicode`). `crates/syntax/src/parser/statement.rs`: a new file `parser/module.rs` holds the grammar of ImportDeclaration (default, namespace, named, side-effect, with `with { key: "value", ... }` attributes) and of ExportDeclaration (`export var|let|const|function|async function|class|generator`, `export default` of a HoistableDeclaration, ClassDeclaration or AssignmentExpression, `export { a as b }`, `export { a } from`, `export * from`, `export * as ns from`); in a Script, the arm at line 238 becomes the grammar's `SyntaxError` for `export`, and the `import` declaration branch at lines 245 to 247 (`_ =>`) the same, by the messages of `crates/syntax/src/error.rs`, not a refusal.
- Early errors (`crates/syntax/src/early.rs` or the walk the parser already runs after a Script): duplicate exported names; an exported local name that is not declared; duplicate lexically declared names and var/lexical clashes over the module's top level, with import bindings counted as lexical; an `import` attribute key given twice (`#sec-imports-static-semantics-early-errors`); `await` as an identifier anywhere in the module; `export default` twice; a ModuleExportName string that is not well-formed; `new.target`, `super` and `arguments` rules of the top level as a Script has them; and the label and `break`/`continue` rules over module items.
- Two holds (`engine.md` section 12, "A part before its slice"; the mechanism slice 8 used): `Parser::hold(construct, slice)` comes back in `parser/mod.rs` beside `refuse` (line 513), keeping the first held construct in `held` and going on; `HOLDS` gets the lines `("namespace import", 11)` and `("top-level await", 11)`. Held: `import * as ns` and `export * as ns from` (lifted by `story:module-namespace-objects`), and an `await` at the top level of a module outside a function (lifted by `story:top-level-await`; the grammar `[+Await]` of ModuleItemList and its early errors are this story's, so a held source still reports its first grammar or early error). A source with no error and a held construct answers the one refusal `<construct> is not supported before slice 11`, counted by `Agent::refusals` once `Agent::parse_module` exists.
- Sources: `crates/syntax/tests/sources/refused/export-declaration.js` and `import-declaration.js` move to `crates/syntax/tests/sources/early-errors/` with the header `SyntaxError "<message>"` of the grammar (Script goal). Module sources are tested by a new module of the test binary, `crates/syntax/tests/modules.rs`, with the sources inline: one test per form of ImportDeclaration and ExportDeclaration (the entries it yields, compared with the lists the draft gives for the forms of `#sec-importentry-record` Table 61 and `#sec-exportentry-record` Table 63), one per early error above, `html_like_comment_is_no_comment_in_a_module` (`a <!-- b` parses as `a < !(--b)`), `a_module_is_strict` (`with`, octal literals, `eval = 1` each a `SyntaxError`), `a_namespace_import_is_held_after_its_early_errors` and `top_level_await_is_held_after_its_early_errors` (the refusal when no error, the grammar's error when one), and `export_and_import_in_a_script_are_syntax_errors`. `crates/syntax/tests/main.rs` generated again.
- Nesting: `parse_module` keeps the parser's nesting bound and the stack bound of `ParseOptions::max_nesting` (lib.rs line 75 to 87): a test of `tests/modules.rs` parses an `export default` of 2,000 nested parentheses at `max_nesting` 1,000 and gets the one `SyntaxError` naming `parser_nesting` (the shape of HOST-138, whose engine test is `story:module-records-loading-and-module-runs`'s).
- Rules (the `AGENTS.md` of the folder that holds the repository, and `AGENTS.md`, "Rules" and "Commands and the gate"): no `unsafe` (`#![forbid(unsafe_code)]`), no third-party crate, no panic, no `unwrap`, `expect` or unchecked indexing in library code; every loop of the change (a walk of a module graph, of a list of entries, of a namespace's names) is charged to the step budget and every allocation to the memory budget, and a new limit gets a test that reaches it; a builtin's module and comments cite its clause. Time ("Tests stay fast"): a single test is terminated at 60 s and counts as failed, and the gate fails when the wall time of its test steps passes 120 s; the unit times each test before it adds it and names in its report every test over 5 s with its time; one integration-test binary per crate (a new test file is a module of the crate's `tests/main.rs`, written again by `sh taskfiles/test-main.sh write`); an expensive product several tests read (a parsed corpus, a fixture tree, a built part of the test262 checkout) is made once, as a committed file or one shared fixture, never per test and never in a `static`; no test makes a network request or a model call. Builds and tests go through the pool: `cargo` is the shim of the pool, anything else that compiles or tests is prefixed `slot`; a pool job ends after 10 minutes unless it names its time with `slot --timeout` or `SLOT_TIMEOUT`, at most 30 minutes; a script that only measures, probes or waits starts without the `slot` prefix. No Python in anything committed (tools, tasks, gates, tests, helpers): Rust or POSIX sh. The unit runs the tests of its own change, never the whole gate and never a whole-workspace test run; intermediate files go in the scratch directory; no `cd ... &&` with a write or a delete.
- Size: about 1,300 lines (the grammar 500, the entries and early errors 350, the tests 450): one session.

The only story of order 1. `story:import-call-and-import-meta-syntax` (order 2) builds on it: it uses `Parser::hold` and the Module goal of this story (`import.meta` is valid in the Module goal only) and writes the `Some(Punct::Dot)` and `Some(Punct::LParen)` branches of the arm `Keyword::Import` (lines 244 and 245), which this story leaves refusing; this story writes that arm's `_ =>` branch (the declaration, line 246) and the `Keyword::Export` arm (line 238).

## Requirements

None of `conformance/host/requirements.txt` moves (HOST-138 is shown through `Agent::parse_module`, `story:module-records-loading-and-module-runs`).

Writes: `crates/syntax/src/{lib,ast,lexer,early,error}.rs`, `crates/syntax/src/parser/{mod,statement}.rs`, `crates/syntax/src/parser/module.rs` (new), `crates/syntax/tests/modules.rs` (new), `crates/syntax/tests/main.rs` (generated), `crates/syntax/tests/sources/refused/{export,import}-declaration.js` moved to `crates/syntax/tests/sources/early-errors/`. Checked against the code on `f962c324`: `swell-compiler` matches `Stmt` and `Expr` without a wildcard; this story adds no variant to either, so no compiler file moves. Not written: `crates/compiler/` (`story:module-code-compiles`), `crates/swell/` and `crates/test262/` (`story:module-records-loading-and-module-runs`), `conformance/`, `docs/design/`.

Order 1, alone; it depends on no open story.

## Acceptance

`cargo nextest run --locked -p swell-syntax --test main` exits 0 (the new module `modules::`, `sources::` with the two moved sources, and every test that existed); `cargo nextest run --locked -p swell-syntax --lib` exits 0; `cargo nextest run --locked -p swell-compiler -p swell --no-run` exits 0 (the crates that read the tree still build). `grep -rnE 'refuse[_a-z]*\("[^"]*", 11' crates/syntax/src | wc -l` prints `4` (`6` on the base: the refusals of `export declaration` and `import declaration` leave; the four of `import.meta` and `dynamic import` are `story:import-call-and-import-meta-syntax`'s); `grep -c '", 11)' crates/syntax/src/parser/mod.rs` prints at least `2` (the two lines of `HOLDS`). `slot task clippy` exits 0.

test262. The full run is not the unit's. In one pool job on the build node: `slot --timeout 30m --fetch .out/test262-result sh docs/plan/slice-11/run-part.sh docs/plan/slice-11/story-module-syntax-and-early-errors.cases.txt docs/plan/slice-11/guard-syntax.cases.txt`; then on the workstation, with the base run of the join the unit starts from copied to `.out/test262-base/runs.tsv`, `sh docs/plan/slice-11/check.sh docs/plan/slice-11/story-module-syntax-and-early-errors.cases.txt .out/test262-base/runs.tsv .out/test262-result/runs.tsv` exits 0 and prints `passed 987 bound 987 B 985 F 2 R 0` (over the base run of `f962c324` the part holds 987 cases: the 2 of the list, which fail there, and the 985 of `guard-syntax.cases.txt`, every one passing there: reserved words, comments, identifiers, keywords, white space, line terminators, and `await`, call, `new`, labelled and expression statements and assignment target types; the guard shows the Script goal unchanged).

## Out of scope

`import()` and `import.meta` (`story:import-call-and-import-meta-syntax`); compiling a module (`story:module-code-compiles`); module records, `Agent::parse_module` and the runner's module runs (`story:module-records-loading-and-module-runs`); lifting the holds (`story:module-namespace-objects`, `story:top-level-await`); HTML-like comments in a Script (slice 18).
