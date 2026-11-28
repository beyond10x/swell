---
format: aep.planning-md/3
id: story:module-syntax-and-early-errors
kind: story
status: active
title: 'The parser''s Module goal: import and export declarations of every form, attributes and module early errors, with namespace imports and top-level await held'
summary: swell_syntax::parse_module, the Module goal (strict, no HTML-like comments), every import and export form with attributes and string names, the five lists of entries, module early errors; import and export in a Script become SyntaxErrors; holds namespace import and top-level await with held sources of the Module goal. Order 1; 3 cases.
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
  path: crates/syntax/src/parser/expression.rs
- confidence: cited
  path: crates/syntax/src/parser/mod.rs
- confidence: inferred
  path: crates/syntax/src/parser/module.rs
- confidence: cited
  path: crates/syntax/src/parser/statement.rs
- confidence: inferred
  path: crates/syntax/tests/modules.rs
- confidence: cited
  path: crates/syntax/tests/sources.rs
- confidence: inferred
  path: crates/syntax/tests/sources/early-errors/export-declaration.js
- confidence: inferred
  path: crates/syntax/tests/sources/early-errors/import-declaration.js
- confidence: inferred
  path: crates/syntax/tests/sources/held/module-export-star-as.js
- confidence: inferred
  path: crates/syntax/tests/sources/held/module-namespace-import.js
- confidence: inferred
  path: crates/syntax/tests/sources/held/module-top-level-await.js
- confidence: cited
  path: crates/syntax/tests/sources/refused/export-declaration.js
- confidence: cited
  path: crates/syntax/tests/sources/refused/import-declaration.js
revision: 11
transitions:
- {from: "draft", to: "proposed", at: "2026-11-28T03:55:09Z", actor: "human:operator", revision: 10, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-28T03:55:10Z", actor: "human:operator", revision: 11, executor: "agent:coordinator"}
---
## Context

Slice 11 brings modules (`epic:slice-11-modules`; `docs/design/engine.md`, section 12, item 11). This story is the first of order 1: the parser's Module goal. On the base (`f962c324`) `swell-syntax` has one goal, Script, and refuses every module construct: `crates/syntax/src/parser/statement.rs` line 238 (`export declaration`, slice 11) and the arm `Keyword::Import` (lines 239 to 249: `import.meta` at line 245, `dynamic import` at 246, `import declaration` at 247); the lexer refuses `<!--` and `-->` as an `HTML-like comment` of slice 18 (`crates/syntax/src/lexer.rs` line 318), which in the Module goal are no comment at all (ECMA-262 `#sec-html-like-comments`: the extension holds for Script only). `HOLDS` (`crates/syntax/src/parser/mod.rs` line 67) is empty and `Parser::hold` was deleted with the last hold of slice 8; the field `held` of the parse (`parser/mod.rs` line 147) and the test of `tests/sources/held/` (`crates/syntax/tests/sources.rs`, `held_sources_answer_their_first_construct_still_held`, lines 263 to 320) remain. That test parses every held source in the Script goal (line 301) and fails when a line of `HOLDS` is named by no held source (lines 309 to 315).

The count. Of the 1,281 cases (1,833 runs) of slice 11 on the base run (the last full run on `f962c324`, `test262-slice 11 1281 1833 0 0 1281 23 1258 0`), 3 are this story's (`docs/plan/slice-11/story-module-syntax-and-early-errors.cases.txt`, 6 runs, 2 of them passing on the base): `test/language/global-code/export.js` and `import.js`, Script-goal cases whose `negative` is a parse `SyntaxError` and which fail on the base on the refusal (`export declaration`, `import declaration`), and pass once a declaration in a Script is the grammar's `SyntaxError`; and `test/language/module-code/top-level-await/new-await-script-code.js`, a Script that uses `await` as an identifier, which passes on the base and is on this list because it needs no part of the slice (the scan's rule for a case with no part, `docs/plan/slice-11/scan.awk`): it guards the Script goal against this story's `await` rules. Every other case of the slice that this story's grammar decides (180 module cases with a parse `SyntaxError` among them) is a module run, which the runner makes from `story:module-records-loading-and-module-runs` on (the scan puts each case in the first story, in the order of the edges, whose closure holds every part it needs). This story is shown by the tests of `swell-syntax`.

Design it follows: ECMA-262 16.2 (`#sec-modules`: Module, ModuleItem, ImportDeclaration with `with` attributes, ExportDeclaration, ModuleExportName with string names, the static semantics ImportEntries, ExportEntries, ModuleRequests, the early errors of `#sec-module-semantics-static-semantics-early-errors`, `#sec-imports-static-semantics-early-errors` and `#sec-exports-static-semantics-early-errors`); `docs/design/host-interface.md` section 9 (`parse_module`, line 928); `engine.md` section 11, "Refusals", and section 12, "A part before its slice".

## What later stories reuse

The entry `swell_syntax::parse_module` and its tree: the module items, and per module the lists ModuleRequests (specifier and attributes in source order, each specifier once with its attributes), ImportEntries, LocalExportEntries, IndirectExportEntries and StarExportEntries, computed once at parse time. `story:module-code-compiles` compiles that tree and carries ModuleRequests into the code; `story:module-records-loading-and-module-runs` reads them; `story:module-linking-and-evaluation` reads the entries; `story:import-call-and-import-meta-syntax` uses `Parser::hold`, the Module goal and the held sources of the Module goal; `story:module-namespace-objects` and `story:top-level-await` lift the two holds this story adds, each by deleting its `hold` call and its line of `HOLDS` only.

## Domain relations

- A module (the parsed tree) → its module requests: one to many, ordered by first appearance in the source, a specifier with one set of attributes once — inferable (inferred from ECMA-262 `#sec-static-semantics-modulerequests`; no ess/1 document declares it: `spec/domains/host.yaml` holds the attributes of a load (`swell.host.ModuleLoad`, field `attributes`) and no tree, and ESS does not restate ECMA-262, `AGENTS.md`, "Rules").
- A module → its import and export entries: one to many each, owned by the module's tree — inferable (inferred from ECMA-262 `#sec-importentry-record` and `#sec-exportentry-record`; no ess/1 document declares them, for the same reason).

## Change

Failing tests first: the tests and sources below are written before the code and fail on the base (no `parse_module`; the declarations refused).

- `crates/syntax/src/lib.rs`: `pub fn parse_module(source: &[u16], options: ParseOptions) -> Result<Module, Vec<SyntaxError>>` and `parse_module_with` (after `parse_script_with`, line 168, in their form), and the tree type `Module` beside `Script` (line 109): its items, the five lists of entries, and `top_level_await: bool` ([[HasTLA]] of ParseModule, `#sec-parsemodule`), set by the parser where it meets an `await` or a `for await` at the module's top level (the places of the hold below), which `story:top-level-await` reads through the compiler once it lifts the hold. The Module goal is strict from its start, has no `return` at its top, and lexes `<!--` and `-->` as punctuators (`crates/syntax/src/lexer.rs` line 318: the refusal holds in the Script goal only; the lexer gets the goal as a flag of its state).
- `crates/syntax/src/ast.rs`: the module items (`Stmt` line 587 gets no variant; a `ModuleItem` enum of its own, `Import(ImportDeclaration)`, `Export(ExportDeclaration)`, `Stmt(StmtId)`), after `ClassElement` (line 738), the declarations and `ModuleExportName` (an identifier or a string literal, the string checked for well-formed UTF-16, `#sec-static-semantics-isstringwellformedunicode`). A new file `crates/syntax/src/parser/module.rs` holds the grammar of ImportDeclaration (default, namespace, named, side-effect, with `with { key: "value", ... }` attributes) and of ExportDeclaration (`export var|let|const|function|async function|class|generator`, `export default` of a HoistableDeclaration, ClassDeclaration or AssignmentExpression, `export { a as b }`, `export { a } from`, `export * from`, `export * as ns from`). In `crates/syntax/src/parser/statement.rs`, in a Script, the arm `Keyword::Export` (line 238) becomes the grammar's `SyntaxError` and the declaration branch `_ =>` of the arm `Keyword::Import` (line 247) the same, by the messages of `crates/syntax/src/error.rs`, not a refusal; the branches `Some(Punct::Dot)` and `Some(Punct::LParen)` (lines 245 and 246) are left refusing, for `story:import-call-and-import-meta-syntax`.
- Early errors (`crates/syntax/src/early.rs`): duplicate exported names; an exported local name that is not declared; duplicate lexically declared names and var/lexical clashes over the module's top level, with import bindings counted as lexical; an `import` attribute key given twice (`#sec-imports-static-semantics-early-errors`); `await` as an identifier anywhere in the module; `export default` twice; a ModuleExportName string that is not well-formed; and the label and `break`/`continue` rules over module items.
- Two holds (`engine.md` section 12, "A part before its slice"; the mechanism slice 8 used): `Parser::hold(construct, slice)` comes back in `parser/mod.rs` after `refuse_at` (line 519), keeping the first held construct in `held` and going on; `HOLDS` gets the lines `("namespace import", 11)` and `("top-level await", 11)`, in that order, and `#[rustfmt::skip]` above the array with its line of documentation, as slice 8 had them (`rustfmt` would join a short list on one line; on the base the array is `&[]` and carries no attribute, `crates/syntax/src/parser/mod.rs` line 67); the stories that lift a hold delete its line and leave the attribute. Where the two hold calls stand: `namespace import` in `parser/module.rs`, at `import * as` and at `export * as ns from`; `top-level await` in `crates/syntax/src/parser/expression.rs`, in `prefix_operators` (line 771) at the `Prefix::Await` of `prefix` (line 742) when the parser's `awaits` flag was set by the Module goal's `[+Await]` and no function encloses the expression, and in `parser/statement.rs` at a `for await` of the module's top level. The grammar `[+Await]` of ModuleItemList and its early errors are this story's, so a held source still reports its first grammar or early error. A source with no error and a held construct answers the one refusal `<construct> is not supported before slice 11`. The flag `top_level_await` is set at the hold's places while the hold stands, and a unit test in `crates/syntax/src/parser/module.rs` reads it from the parser's tree before the held check: `the_top_level_await_flag_is_set_at_await_and_for_await` (`await 0;` and `for await (x of y);` set it, `async function f() { await 0; }` does not).
- Held sources of the Module goal: `crates/syntax/tests/sources.rs`, `held_sources_answer_their_first_construct_still_held` (lines 263 to 320): a held source whose file name starts with `module-` is parsed by `parse_module`, as written and with a `"use strict";` prologue, and the rest of the test is unchanged (a source none of whose constructs is held any more parses, through `parse_module`, as a valid module does); the test's documentation says so. This story owns that change; `story:import-call-and-import-meta-syntax` adds a held source of each kind, and the stories that lift a hold write neither `sources.rs` nor `held/`. New held sources: `crates/syntax/tests/sources/held/module-namespace-import.js` (`import * as ns from "a";`, header `held "namespace import"`), `module-export-star-as.js` (`export * as ns from "a";`, the same header) and `module-top-level-await.js` (`await 0;`, header `held "top-level await"`).
- Moved sources: `crates/syntax/tests/sources/refused/export-declaration.js` and `import-declaration.js` move to `crates/syntax/tests/sources/early-errors/` with the header `SyntaxError "<message>"` of the grammar (Script goal).
- Tests in a new module `crates/syntax/tests/modules.rs`, 16, with the sources inline: `import_default_named_and_side_effect_forms_yield_their_entries`, `import_with_attributes_yields_the_request_with_its_attributes` (compared with the lists the draft gives, `#sec-importentry-record` Table 61), `duplicate_attribute_key_is_a_syntax_error`, `export_declarations_yield_local_entries`, `export_default_forms_bind_default`, `export_named_and_from_forms_yield_indirect_entries`, `export_star_from_yields_a_star_entry` (`#sec-exportentry-record` Table 63), `string_export_names_must_be_well_formed`, `duplicate_export_names_are_a_syntax_error`, `an_exported_name_that_is_not_declared_is_a_syntax_error`, `import_bindings_clash_with_lexical_and_var_declarations`, `await_is_no_identifier_in_a_module`, `html_like_comment_is_no_comment_in_a_module` (`a <!-- b` parses as `a < !(--b)`), `a_module_is_strict` (`with`, an octal literal and `eval = 1` each a `SyntaxError`), `export_and_import_in_a_script_are_syntax_errors`, and `parse_module_reaches_parser_nesting` (an `export default` of 2,000 nested parentheses at `max_nesting` 1,000 answers the one `SyntaxError` naming `parser_nesting`, the shape of HOST-138, whose engine test is the records story's). The holds are shown by the held sources, not by tests of this module, so that lifting a hold changes no test here.
- Rules (the `AGENTS.md` of the folder that holds the repository, and `AGENTS.md`, "Rules" and "Commands and the gate"): no `unsafe` (`#![forbid(unsafe_code)]`), no third-party crate, no panic, no `unwrap`, `expect` or unchecked indexing in library code; every loop of the change (a walk of a module graph, of a list of entries, of a namespace's names) is charged to the step budget and every allocation to the memory budget, and a new limit gets a test that reaches it; a builtin's module and comments cite its clause. Time ("Tests stay fast"): a single test is terminated at 60 s and counts as failed, and the gate fails when the wall time of its test steps passes 120 s; the unit times each test before it adds it and names in its report every test over 5 s with its time; one integration-test binary per crate (a new test file is a module of the crate's `tests/main.rs`, written again by `sh taskfiles/test-main.sh write`); an expensive product several tests read (a parsed corpus, a fixture tree, a built part of the test262 checkout) is made once, as a committed file or one shared fixture, never per test and never in a `static`; no test makes a network request or a model call. Builds and tests go through the pool: `cargo` is the shim of the pool, anything else that compiles or tests is prefixed `slot`; a pool job ends after 10 minutes unless it names its time with `slot --timeout` or `SLOT_TIMEOUT`, at most 30 minutes; a script that only measures, probes or waits starts without the `slot` prefix. No Python in anything committed (tools, tasks, gates, tests, helpers): Rust or POSIX sh. The unit runs the tests of its own change, never the whole gate and never a whole-workspace test run; intermediate files go in the scratch directory; no `cd ... &&` with a write or a delete.
- Report: the sum of the CPU seconds of the story's new tests in the default mode and in the mode `collect-every-safe-point` (each test run alone, user plus system), and the wall seconds of its part run when it has one; no story bounds the CPU seconds of a whole gate.
- Generated files: the generated `tests/main.rs` of every crate and `website/data/status.json` are in no story's scope. A unit that adds a test module writes `tests/main.rs` again with `sh taskfiles/test-main.sh write`; a unit that meets a conflict in a generated file takes either side and writes it again; the coordinator writes both again on the join (`sh taskfiles/test-main.sh write`, `slot --local task site-data`).
- Line numbers are those of `f962c324`; anchors are found by name.
- Size: about 1,300 lines (the grammar 500, the entries and early errors 350, the tests and sources 450): one unit.

The only story of order 1 that writes code; `story:modules-design-text` (order 1, the design author's) writes only `docs/design/` and one line of `conformance/host/requirements.txt`. `story:import-call-and-import-meta-syntax` (order 2) builds on this story: it writes the branches at lines 245 and 246, which this story leaves refusing.

## Requirements

None of `conformance/host/requirements.txt` moves (HOST-138 is shown through `Agent::parse_module`, `story:module-records-loading-and-module-runs`).

Writes: `crates/syntax/src/{lib,ast,lexer,early,error}.rs`, `crates/syntax/src/parser/{mod,statement,expression}.rs`, `crates/syntax/src/parser/module.rs` (new), `crates/syntax/tests/sources.rs` (the Module goal of a held source), `crates/syntax/tests/modules.rs` (new), `crates/syntax/tests/sources/held/module-{namespace-import,export-star-as,top-level-await}.js` (new), `crates/syntax/tests/sources/refused/{export,import}-declaration.js` moved to `crates/syntax/tests/sources/early-errors/`. Checked against the code on `f962c324`: `swell-compiler` matches `Stmt` and `Expr` without a wildcard; this story adds no variant to either, so no compiler file moves. Not written: `crates/compiler/` (`story:module-code-compiles`), `crates/swell/` and `crates/test262/` (`story:module-records-loading-and-module-runs`), `conformance/`, `docs/design/`.

Order 1, beside `story:modules-design-text`; it depends on no story.

## Acceptance

`cargo nextest run --locked -p swell-syntax --test main` exits 0 with 16 tests in `modules::` (none on the base: `cargo nextest run --locked -p swell-syntax --test main -E 'test(/^modules::/)'` runs 16) and every test that existed; `ls crates/syntax/tests/sources/held/module-*.js | wc -l` prints `3` (`0` on the base), `ls crates/syntax/tests/sources/refused | wc -l` prints `7` (`9` on the base) and `ls crates/syntax/tests/sources/early-errors/export-declaration.js crates/syntax/tests/sources/early-errors/import-declaration.js` lists both; `cargo nextest run --locked -p swell-syntax --lib` exits 0 and `-E 'test(the_top_level_await_flag_is_set_at_await_and_for_await)'` runs 1 test; `grep -B1 'pub const HOLDS' crates/syntax/src/parser/mod.rs | grep -c 'rustfmt::skip'` prints `1` (`0` on the base); `cargo nextest run --locked -p swell-compiler -p swell --no-run` exits 0 (the crates that read the tree still build). `grep -rnE 'refuse[_a-z]*\("[^"]*", 11' crates/syntax/src | wc -l` prints `4` (`6` on the base: the refusals of `export declaration` and `import declaration` leave; the four of `import.meta` and `dynamic import` are `story:import-call-and-import-meta-syntax`'s); `grep -c '", 11)' crates/syntax/src/parser/mod.rs` prints `2` (the two lines of `HOLDS`; `0` on the base). `slot task clippy` exits 0.

test262. The full run is not the unit's. In one pool job on the build node: `slot --timeout 30m --fetch .out/test262-result sh docs/plan/slice-11/run-part.sh docs/plan/slice-11/story-module-syntax-and-early-errors.cases.txt docs/plan/slice-11/guard-syntax.cases.txt`; then on the workstation, with the base run of the join the unit starts from copied to `.out/test262-base/runs.tsv`, `sh docs/plan/slice-11/check.sh docs/plan/slice-11/story-module-syntax-and-early-errors.cases.txt .out/test262-base/runs.tsv .out/test262-result/runs.tsv` exits 0 and prints `passed 988 bound 988 B 986 F 2 R 0` (the part holds 988 cases and pulls in none by `./`: the 3 of the list, of which `new-await-script-code.js` passes on the base and the 2 others fail, and the 985 of `guard-syntax.cases.txt`, every one passing there: reserved words, comments, identifiers, keywords, white space, line terminators, and `await`, call, `new`, labelled and expression statements and assignment target types; the guard shows the Script goal unchanged; `B` is the same on `f962c324` and on the join, since no story lands before this one).

## Out of scope

`import()` and `import.meta` (`story:import-call-and-import-meta-syntax`); compiling a module (`story:module-code-compiles`); module records, `Agent::parse_module` and the runner's module runs (`story:module-records-loading-and-module-runs`); lifting the holds (`story:module-namespace-objects`, `story:top-level-await`); HTML-like comments in a Script (slice 18).
