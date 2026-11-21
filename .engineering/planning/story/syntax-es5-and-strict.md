---
format: aep.planning-md/3
id: story:syntax-es5-and-strict
kind: story
status: implemented
title: swell-syntax parses the ES5 grammar with strict mode, refuses later syntax by slice, bounds nesting
summary: Lexer, parser, early errors and scope analysis for the slice 1 grammar with strict mode; later constructs refused naming their slice; three refusals recorded as omissions in sections.txt.
relations:
- decomposes: epic:slice-01-harness
- depends_on: story:test262-expected-and-sections
- depends_on: story:num-and-unicode-early-parts
scope:
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: Cargo.toml
- confidence: cited
  path: conformance/test262/expected.txt
- confidence: cited
  path: conformance/test262/sections.txt
- confidence: cited
  path: crates/syntax
- confidence: cited
  path: tests/sources
- confidence: cited
  path: tests/sources.rs
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T04:49:58Z", actor: "human:operator", revision: 4, decided_on: {"recorded":{"review_outcome":1}}, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-21T04:49:58Z", actor: "human:operator", revision: 5, decided_on: {"recorded":{"review_outcome":1}}, executor: "agent:coordinator"}
- {from: "active", to: "implemented", at: "2026-11-21T06:13:57Z", actor: "human:operator", revision: 6, decided_on: {"recorded":{"test_result":1,"review_outcome":1,"verification":1}}, executor: "agent:coordinator"}
---
## Context

The first engine needs a lexer, a parser and the early errors for the grammar the 31 cases of
slice 1 and their harness files use (`assert.js`, `sta.js`, `compareArray.js`,
`byteConversionValues.js`, `dateConstants.js`, `decimalToHexString.js`, `proxyTrapsHelper.js`,
`tcoHelper.js`; the 51 other cases move to later slices, `story:test262-run-path`). That grammar
is the ES5 one: `var` and function declarations, every ES5 statement but `with` and labelled
statements, and every ES5 expression form (object literals with getters and setters, array
literals with holes, function expressions, `new`, `delete`, `typeof`, `instanceof`, `in`).

Strict mode comes into slice 1 (agreed on 2026-11-21 by the session that decides for the
operator): the runner runs every case of slice 1 twice, once with `"use strict";` prepended
(decision 11; the 82 cases of the directory have no `flags`, `features` or `negative`: over
their lines of `swell-test262 meta`, `cut -f<n> | grep -c .` prints 0 for each of fields 2 to
5), so without strict mode no run of the slice can pass. What strict mode adds to the grammar is
the directive prologue and the strict early errors; this story takes both. The runtime half
(`this` not coerced, a `ReferenceError` on assignment to an unresolvable name, a `TypeError`
when PutValue or `delete` fails) is `story:interpreter-and-script-entry-points`.

Every construct of a later slice is refused with a defined SyntaxError, never parsed into a
stand-in (`docs/design/engine.md`, section 12, "A part before its slice"). Two gaps fall inside
sections whose test262 cases are counted in slice 2, so they are recorded as omissions in
`conformance/test262/sections.txt`, as that rule says:

- an identifier character outside ASCII is a SyntaxError (the identifier tables are slice 5);
- a regular expression literal is lexed to its end and evaluating it throws a SyntaxError, as
  the design states it: this story lexes and parses the literal into its own node;
  `story:bytecode-compiler` compiles the node to a throw of that SyntaxError and
  `story:interpreter-and-script-entry-points` runs it.

`Zs` white space is accepted, not refused: the `Zs` table arrives in slice 1 with
`story:num-and-unicode-early-parts` (it is needed there for StringToNumber), so the lexer reads
every white space code point of `sec-white-space` now, and this story depends on that one. The
edge adds no wave: this story then runs in wave 2 beside `story:heap-values-and-host-surface`,
`story:bytecode-compiler` in wave 3 beside `story:object-model-and-realms`, and the interpreter
stays in wave 4. The sentence of "A part before its slice" that names `Zs` white space as a
SyntaxError before slice 5 is corrected in `story:test262-run-path`, which edits the design.

The parser carries the nesting counter of decision 3: passing `parser_nesting` is a SyntaxError
whose message names `parser_nesting` (`docs/design/host-interface.md`, `HOST-137`, closed by
`story:interpreter-and-script-entry-points`, which owns `parse_script`).

## Change

Failing tests first: `crates/syntax/tests/sources.rs` and the source files below are written
before the code and fail (the crate does not exist).

Crate layout, stated once here: `crates/syntax/` (package `swell-syntax`, dependency
`swell-unicode` by path, `#![forbid(unsafe_code)]`, the four clippy denials of
`crates/swell/Cargo.toml`):

- `src/lib.rs`: `pub fn parse_script(source: &[u16], options: ParseOptions) -> Result<Script,
  Vec<SyntaxError>>` with `ParseOptions { max_nesting: u32 }`. Input is UTF-16 code units; a
  UTF-8 source is converted by the caller.
- `src/error.rs`: `SyntaxError { message, line, column }`. A refusal reads
  `<construct> is not supported before slice <n>`; the nesting error reads
  `nesting deeper than parser_nesting (<limit>)`.
- `src/token.rs`, `src/lexer.rs` (12, `sec-ecmascript-language-lexical-grammar`): white space
  (`Zs` through `swell_unicode::is_space_separator`), line terminators, comments, ASCII
  identifiers and reserved words, punctuators, numeric literals (decimal, `0x`, legacy octal;
  the literal text is kept, its value is computed by `swell-compiler` with `swell-num`), string
  literals with every escape (a lone surrogate from `\uD800` or from a UTF-16 source is kept as a
  code unit), regular expression literals lexed to their end
  (`sec-literals-regular-expression-literals`: body and flags, with the goal symbol chosen by
  the parser).
- `src/ast.rs`: the tree, a `RegExpLiteral` node among the expressions. Every node records its
  source position.
- `src/parser/mod.rs`, `src/parser/statement.rs` (14), `src/parser/expression.rs` (13),
  `src/parser/function.rs` (15.2): recursive descent; every recursive entry goes through one
  depth counter; at `max_nesting` 0 any non-empty source is refused.
- `src/early.rs` (`sec-scripts-static-semantics-early-errors` and the early errors of each
  production in 13 to 15 for the slice 1 grammar), strict ones included: `delete` of an
  identifier, `eval` or `arguments` as a binding or assignment target, duplicate parameters,
  legacy octal literals and escapes, `\8` and `\9`, the strict reserved words, `with`.
- `src/scope.rs`: the directive prologue (`sec-directive-prologues-and-the-use-strict-directive`,
  the exact code units, no escapes), the strictness of each function, VarDeclaredNames and the
  function declarations to hoist per function and per script
  (`sec-scripts-static-semantics-vardeclarednames`), which identifiers each function references.
- Every walk over the tree (early errors, scope analysis, dropping the tree) is bounded by the same
  depth, so its Rust stack use is a function of `max_nesting`.

Refused with `<construct> is not supported before slice <n>`: labelled statements, `with` in
non-strict code, a reference to `arguments` inside a function, every place where
`sec-automatic-semicolon-insertion` would insert a semicolon (including the restricted
productions) (slice 2); a function declaration inside a block, `let`, `const`, arrow functions,
template literals, spread, destructuring, computed and shorthand properties and methods,
`for-of` (slice 6); generators (7); classes, `super`, `new.target` (8); `async`, `await` (10);
`import`, `export`, `import.meta` (11); BigInt literals (12); `**`, `??`, `?.`, logical
assignment, numeric separators, optional catch binding (the slice of their feature rule in the
design, 2 when none); an identifier with a code point outside ASCII, a `\u` escape included (5,
a recorded omission).

Tests, each source file starting with a comment that names its clause anchor and expected result:

- `crates/syntax/tests/sources/valid/*.js`: one file per statement and expression form of the
  grammar above, `Zs` white space and a regular expression literal among them, each parsed both
  as written and with a `"use strict";` prologue.
- `crates/syntax/tests/sources/early-errors/*.js`: one file per early error above (strict ones
  start with the prologue), each with the expected message.
- `crates/syntax/tests/sources/refused/*.js`: one file per refused construct, with its slice.
- `crates/syntax/tests/sources.rs`: `valid_sources_parse_in_both_modes`,
  `early_error_sources_fail_with_their_message`, `refused_sources_name_their_slice` (each of the
  three fails when its directory holds no `.js` file: `AGENTS.md`, "A check that finds nothing
  to read fails"), `nesting_of_100000_brackets_is_one_error` (limit 1,000, on a spawned thread
  with a 256 KiB stack), `nesting_zero_refuses_any_non_empty_source`,
  `nesting_error_names_parser_nesting`, `lone_surrogate_in_a_string_literal_is_kept`,
  `use_strict_with_an_escape_is_not_a_directive`, `function_bodies_inherit_and_set_strictness`,
  `var_declared_names_and_hoisted_functions`, `every_zs_code_point_is_white_space` (the 17),
  `regexp_literal_is_lexed_to_its_end` (a `/` inside a class and an escaped `/` do not end it).

Omissions: `conformance/test262/sections.txt` gains, after the 33 sections, in clause order,
`section sec-names-and-keywords`, `path sec-names-and-keywords test/language/identifiers/`,
`omit sec-names-and-keywords identifiers-outside-ascii section-12-part-before-its-slice`;
`section sec-literals-regular-expression-literals`,
`path sec-literals-regular-expression-literals test/language/literals/regexp/`,
`omit sec-literals-regular-expression-literals regexp-literals section-12-part-before-its-slice`.
`conformance/test262/expected.txt` gains the two lines `test262-section sec-names-and-keywords
incomplete 268 0 1` and `test262-section sec-literals-regular-expression-literals incomplete
240 0 1` (case counts from `grep -c "^<path>" .cache/inventory/files.txt`; neither path holds a
fixture file); 280 lines. The engine still runs nothing here, so no other line changes.

`Cargo.lock`: the unit regenerates it in its own tree (one package added), because its build
needs it, and runs its acceptance with `--locked` against that lock; the coordinator does not
take the unit lock into the wave branch but regenerates it there after the merges of the wave.
Gate task: none new.

Writes: `crates/syntax/` (new: `Cargo.toml`, `src/lib.rs`, `src/error.rs`, `src/token.rs`,
`src/lexer.rs`, `src/ast.rs`, `src/parser/mod.rs`, `src/parser/statement.rs`,
`src/parser/expression.rs`, `src/parser/function.rs`, `src/early.rs`, `src/scope.rs`,
`tests/sources.rs`, `tests/sources/`), `conformance/test262/sections.txt`,
`conformance/test262/expected.txt`, `Cargo.lock` (regenerated in the unit tree; the wave
branch lock is the coordinator).

## Acceptance

`cargo test --locked -p swell-syntax` exits 0 and runs the twelve tests named in Change over every
source file under `crates/syntax/tests/sources/`, and `slot task test262` exits 0 and prints
`test262: 280 lines equal conformance/test262/expected.txt`.

## Out of scope

The value of numeric literals and the bytecode, the throw of a regular expression literal
(`story:bytecode-compiler`, `story:interpreter-and-script-entry-points`); `parse_script` of the
host interface, its `Parsed::Errors` objects and `HOST-18`, `HOST-19`, `HOST-36`, `HOST-137`
(`story:interpreter-and-script-entry-points`); the runtime half of strict mode; every refused
construct (its slice); `swell-num` as a dependency of this crate; the design text
(`story:test262-run-path`). The root `Cargo.toml`, `Taskfile.yml` (the coordinator).
