---
format: aep.planning-md/3
id: story:syntax-language-core
kind: story
status: implemented
title: swell-syntax parses the grammar of slice 2 with plain methods and shorthand properties; labels, ??, ?. and logical assignment compile
summary: Semicolon insertion, labels, ??, ?., logical assignment, optional catch, trailing commas, separators, hashbang, ASCII identifier escapes, plain methods and shorthand properties; ** refused naming slice 4; with and arguments stay refused.
relations:
- decomposes: epic:slice-02-language-core
scope:
- confidence: cited
  path: crates/compiler/src/bytecode.rs
- confidence: cited
  path: crates/compiler/src/disasm.rs
- confidence: cited
  path: crates/compiler/src/expression.rs
- confidence: cited
  path: crates/compiler/src/lib.rs
- confidence: cited
  path: crates/compiler/src/resolve.rs
- confidence: cited
  path: crates/compiler/src/statement.rs
- confidence: cited
  path: crates/compiler/tests/compile.rs
- confidence: cited
  path: crates/compiler/tests/sources/coalesce.disasm
- confidence: cited
  path: crates/compiler/tests/sources/coalesce.js
- confidence: cited
  path: crates/compiler/tests/sources/labelled-break.disasm
- confidence: cited
  path: crates/compiler/tests/sources/labelled-break.js
- confidence: cited
  path: crates/compiler/tests/sources/labelled-continue.disasm
- confidence: cited
  path: crates/compiler/tests/sources/labelled-continue.js
- confidence: cited
  path: crates/compiler/tests/sources/logical-assignment.disasm
- confidence: cited
  path: crates/compiler/tests/sources/logical-assignment.js
- confidence: cited
  path: crates/compiler/tests/sources/method-definition.disasm
- confidence: cited
  path: crates/compiler/tests/sources/method-definition.js
- confidence: cited
  path: crates/compiler/tests/sources/optional-catch.disasm
- confidence: cited
  path: crates/compiler/tests/sources/optional-catch.js
- confidence: cited
  path: crates/compiler/tests/sources/optional-chain.disasm
- confidence: cited
  path: crates/compiler/tests/sources/optional-chain.js
- confidence: cited
  path: crates/compiler/tests/sources/regexp.disasm
- confidence: cited
  path: crates/compiler/tests/sources/shorthand-property.disasm
- confidence: cited
  path: crates/compiler/tests/sources/shorthand-property.js
- confidence: cited
  path: crates/swell/src/interp/dispatch.rs
- confidence: cited
  path: crates/swell/tests/language/asi-postfix-line-terminator.js
- confidence: cited
  path: crates/swell/tests/language/asi-return-line-terminator.js
- confidence: cited
  path: crates/swell/tests/language/coalesce-short-circuit.js
- confidence: cited
  path: crates/swell/tests/language/hashbang-first-line.js
- confidence: cited
  path: crates/swell/tests/language/identifier-escape-ascii.js
- confidence: cited
  path: crates/swell/tests/language/label-break-block.js
- confidence: cited
  path: crates/swell/tests/language/label-break-through-finally.js
- confidence: cited
  path: crates/swell/tests/language/label-continue-outer-loop.js
- confidence: cited
  path: crates/swell/tests/language/logical-assignment-function-name.js
- confidence: cited
  path: crates/swell/tests/language/logical-assignment-short-circuit.js
- confidence: cited
  path: crates/swell/tests/language/numeric-separator-value.js
- confidence: cited
  path: crates/swell/tests/language/object-method-is-not-a-constructor.js
- confidence: cited
  path: crates/swell/tests/language/object-method-name-length-and-enumerable.js
- confidence: cited
  path: crates/swell/tests/language/object-method-this.js
- confidence: cited
  path: crates/swell/tests/language/object-shorthand-property.js
- confidence: cited
  path: crates/swell/tests/language/optional-call-this.js
- confidence: cited
  path: crates/swell/tests/language/optional-catch-binding.js
- confidence: cited
  path: crates/swell/tests/language/optional-chain-short-circuit.js
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
  path: crates/syntax/src/parser/function.rs
- confidence: cited
  path: crates/syntax/src/parser/mod.rs
- confidence: cited
  path: crates/syntax/src/parser/statement.rs
- confidence: cited
  path: crates/syntax/src/scope.rs
- confidence: cited
  path: crates/syntax/src/token.rs
- confidence: cited
  path: crates/syntax/tests/sources
- confidence: cited
  path: crates/syntax/tests/sources.rs
revision: 11
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T13:41:53Z", actor: "human:operator", revision: 8, decided_on: {"recorded":{"review_outcome":2}}, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-21T13:41:53Z", actor: "human:operator", revision: 9, decided_on: {"recorded":{"review_outcome":2}}, executor: "agent:coordinator"}
- {from: "active", to: "implemented", at: "2026-11-21T14:46:07Z", actor: "human:operator", revision: 11, decided_on: {"recorded":{"test_result":1,"review_outcome":2,"verification":1}}, executor: "agent:coordinator"}
---
## Context

`swell-syntax` refuses, naming slice 2, every construct of `test/language/` that the ES5 grammar of slice 1 did not take (`refuse("<construct>", 2)` in `crates/syntax/src/parser/{mod,statement,expression,function}.rs` and `crates/syntax/src/lexer.rs`). This story parses all of them except two (`with` and `arguments`), takes plain method definitions and shorthand properties of object literals (refused naming slice 6 today), and compiles all it parses, so that slice 2 then fails only on its run-time parts (`with` and `arguments`: `story:with-arguments-and-dynamic-names`; `eval` and `Function`: `story:eval-and-function-constructor`, which also owns the parse entries of `eval` and `Function` and the mark of a direct `eval` call, so that one story owns the whole dynamic-code entry). It owns the parser files in wave 1; the later stories of the slice edit them in later waves (named in their `Writes:`).

Measured on the base (`6319af2`) with the per-run causes of a throwaway runner (`story:test262-run-causes` makes the runner print them), cases of slice 2 whose first failed run is ended by the refusal of a construct this story takes: automatic semicolon insertion 411 (and 1 more through `harness/asyncHelpers.js`), labelled statements 84 and labelled `break`/`continue` 2, logical assignment 77, numeric separators 37, `??` 28, hashbang comments 7, `?.` 6, trailing commas in arguments and parameters 4, optional catch binding 3, identifier with a `\u` escape 352 (300 of them have only escapes that decode to ASCII: `grep -iE` over each case for an escape of `\u0080` or above, see the decomposition report). Across all slices 13,021 runs are refused naming slice 2; 9,929 of them for `arguments` (mostly `harness/propertyHelper.js`), which this story does not take.

Slice numbers in refusals, checked against `docs/design/engine.md` section 12 (the wave plan asks each slice to check them): every construct refused naming slice 2 has its test262 cases under `test/language/`, which `dir test/language/ 2` gives to slice 2, and no feature line of the block moves them, except exponentiation: the operator `**` needs the deterministic `pow` of `swell-num`, which slice 4 brings (`docs/plan/waves.md`, "What later slices take over"), so `**` and `**=` are refused naming slice 4 from here on, and `story:slice-02-conformance-close` adds `feature exponentiation 4` to the block. An identifier escape that decodes to an ASCII identifier character needs no Unicode table: only an escape to a code point outside ASCII stays refused naming slice 5 (the recorded omission `identifiers-outside-ascii` of `sec-names-and-keywords` stays as it is). A function declaration in a block stays refused naming slice 6 with the lexical declarations of slice 6, and computed property names stay slice 6.

Method definitions and shorthand properties: a unit of slice 1 refused them naming slice 6 (`crates/syntax/src/parser/expression.rs:914` and `:919`, cited), and no line of the design gives them a slice: `grep -n -i "method definition\|shorthand" docs/design/engine.md` finds only lines 781 and 782, which describe two cases of `test/harness`, and item 6 of "What each slice builds" does not name them. They are ES2015 object-literal syntax with no run-time part of a later slice, so slice 2 takes them (decided by the coordinator, revision round 1): a plain method (`m() {}`, also named `get` or `set`) and a shorthand property (`{ a }`). Generator and async methods, computed names, `super` and a CoverInitializedName (`{ a = 1 }`, valid only as a destructuring target) stay refused naming slices 7, 10, 6, 8 and 6. Measured on the base with the per-run causes (`awk` over the run lines of the throwaway runner, the first refusal of each case): in slice 2, the first failed run of 45 cases ends in the refusal `method definition` and of 19 in `shorthand property`; the static scan of those 64 finds 45 that need no part of a later slice besides, and 19 that still need slice 3 (1), 6 (8), 7 (5), 8 (4) or 10 (1). In the later slices the two refusals end the first failed run of 3: 30, 4: 30, 5: 1, 6: 327, 7: 9, 8: 28, 9: 10, 10: 99, 11: 1, 12: 2, 13: 59, 14: 35, 15: 38, 18: 13 and 19: 58 cases.

Two refusals stay until their run-time exists, so that no tree reaches the compiler that it cannot compile: `with statement in non-strict code` and `reference to arguments inside a function` (`story:with-arguments-and-dynamic-names` lifts both).

The refusal of a regular expression literal is not read as one. The compiler emits `ThrowSyntaxError` with `REGEXP_REFUSAL` (`crates/compiler/src/expression.rs:24`, cited): "regular expression literals are not supported before slice 5", while `swell_syntax::message_kind` reads a refusal only in a message `<construct> is not supported before slice <n>` (`crates/syntax/src/error.rs:20`, `REFUSED` = `" is not supported before slice "`, cited), the form the design gives (`docs/design/engine.md`, decision 11 "Refusals", and `host-interface.md`, "A count of refusals"). So the message becomes "regular expression literal is not supported before slice 5", which `message_kind` reads as a refusal of slice 5. Measured on `main` (`6319af2`, the throwaway runner's run lines: `awk -F'\t' '$1 == "test262-run" && $6 ~ /regular expression literals are not supported before slice 5/' count.log`): 1,484 runs of 744 cases end with that message, all counted failed, none passed and none in `test262-refused` (the runner reads the kind of a parse error only, and this error is thrown when the literal is evaluated): 1,480 as `threw SyntaxError: ...` and 4 as `threw Test262Error` whose message quotes it; by slice 02: 58, 03: 14, 05: 1,006, 06: 252, 09: 6, 18: 60, 19: 88 runs. After this story they stay failed and outside `test262-refused`; from wave 3 the agent counts them (`Agent::refusals`) and the runner counts them refused (`story:eval-and-function-constructor`).

Design it builds on, on the branch since merge `63dff4d` (a precondition that holds before wave 1): section 12, "A part before its slice", takes plain method definitions and shorthand properties into slice 2, and decision 11, "Refusals", gives the form of a refusal message. The refusal of `**` naming slice 4 follows the wave plan; the feature line is `story:slice-02-conformance-close`.

## Additions in wave 1 of slice 2 (2026-11-21)

Binding; where this section and the text below differ, this section governs.

- **A test of the conformance target relied on a refusal this story lifts.**
  `conformance/ess/src/target.rs`, the test
  `a_construct_the_parser_refuses_until_a_later_slice_is_unsupported_not_parse_failed`, parses
  `1 + 1` and expects a refusal for semicolon insertion. Its source becomes a construct of a
  late slice that stays refused as long as possible (not `with`, which wave 2 of this slice
  delivers): a construct of slice 11 or later, as the parser refuses it today. Only that
  test's source and what it asserts about the message change; the target does not.
- **A script that starts with a hashbang cannot carry its expectation on line 1.**
  `crates/swell/tests/language.rs` reads the expectation from the first line and puts the
  strict prologue before it. It skips a leading `#!` line when it reads the expectation and
  inserts the prologue after that line, so that `hashbang-first-line.js` runs in both modes.
- **The doc comment of `crates/swell/src/object/ecmascript_function.rs`** that says "accessor"
  where methods now apply is corrected (the comment only).

Writes, added: `conformance/ess/src/target.rs` (the one test), `crates/swell/tests/language.rs`,
`crates/swell/src/object/ecmascript_function.rs` (the doc comment).

## Change

Failing tests first: the source files and tests below are written before the code; the moved sources fail as refused, the new ones fail to parse or to compile.

`swell-syntax`:

- `src/lexer.rs`, `src/token.rs`: a `HashbangComment` at the very start of a source (`sec-hashbang`), nowhere else; `NumericLiteralSeparator` in every numeric literal form that allows it, with its early errors (`sec-numeric-literals-early-errors`: not after a leading `0`, not doubled, not trailing, not next to `.`, `e` or the prefix; `swell_num::literal::numeric_literal` already takes `_`, so `crates/num` is not edited); a `\u` escape in an `IdentifierName` (`sec-names-and-keywords`, `sec-identifier-names-static-semantics-early-errors`) whose code point is an ASCII `IdentifierStartChar` or `IdentifierPartChar`; an escape to a code point outside ASCII stays refused naming slice 5. An escaped `IdentifierName` is never a keyword token: as an `Identifier` whose StringValue is a reserved word it is the early error of `sec-identifier-names-static-semantics-early-errors`, as a property name it is allowed.
- `src/parser/mod.rs`, `src/parser/statement.rs`, `src/parser/expression.rs`, `src/parser/function.rs`: automatic semicolon insertion (`sec-automatic-semicolon-insertion`, all three rules and the restricted productions: postfix `++`/`--`, `continue`, `break`, `return`, `throw` with no LineTerminator, never inside a `for` head, never to make an empty statement); labelled statements and labelled `break`/`continue` (`sec-labelled-statements`); `??` (`sec-binary-logical-operators`, a `CoalesceExpression` mixed with `&&` or `||` without parentheses is a SyntaxError); optional chains (`sec-optional-chains`, with calls, `delete` and `typeof` of a chain; a chain as an assignment target is an early error; a tagged template in a chain is refused naming slice 6 as templates are); `&&=`, `||=`, `??=` (`sec-assignment-operators`, simple targets only); optional catch binding (`sec-try-statement`); a trailing comma in arguments and in formal parameters; plain method definitions in object literals (`sec-method-definitions`, `MethodDefinition : ClassElementName ( UniqueFormalParameters ) { FunctionBody }` with an identifier, string or numeric name) and shorthand properties (`sec-object-initializer`, `PropertyDefinition : IdentifierReference`); `**` and `**=` refused naming slice 4.
- `src/ast.rs`: the nodes these need (a labelled statement, labelled `break`/`continue`, a catch without parameter, an optional chain, the operators `??`, `&&=`, `||=`, `??=`, a method and a shorthand property, `FunctionKind::Method`). Every node records its position.
- `src/early.rs`: `ContainsDuplicateLabels`, `ContainsUndefinedBreakTarget`, `ContainsUndefinedContinueTarget` (`sec-labelled-statements-static-semantics-early-errors` and the early errors of `break` and `continue`), a `continue` whose label names no iteration statement, the escaped-keyword errors above, the early errors of a method (`sec-method-definitions-static-semantics-early-errors`: UniqueFormalParameters, so duplicate parameters are an error in every method; a `"use strict"` body makes the method strict, with the checks on its parameters) and of a shorthand property (its IdentifierReference: a reserved word is an error, `eval` and `arguments` as for any reference).
- `src/scope.rs`: a method is a function scope like a function expression without a name of its own; a catch clause without a parameter binds nothing; labels do not change scopes.
- `src/error.rs`, `src/lib.rs`: `REGEXP_REFUSAL`, the message a regular expression literal throws before slice 5, moves here beside `REFUSED`, as "regular expression literal is not supported before slice 5", exported for the compiler; `parse_script` and `ParseOptions` keep their form, so no caller in `crates/swell` or `crates/test262` changes; the module comment lists the refusals that change.
- `ParseOptions::stack_bytes()` stays the bound of the Rust stack a parse at `max_nesting` needs; `every_nesting_kind_fits_the_stack_bound` gains the new nesting kinds (a labelled statement in a labelled statement, a chain of `?.`, `??` inside `??`, a method in a method of a nested object literal), and the constants change only if that test shows a need, measured as it was in slice 1, wave 3 (no incremental compilation, no compiler cache).

`swell-compiler`:

- `src/statement.rs`, `src/lib.rs`: LabelledEvaluation (`sec-labelled-statements`): `break <label>` out of any labelled statement, `continue <label>` to the labelled iteration statement, through `finally` blocks with the completion tags that exist; the completion value of a labelled statement in a script (UpdateEmpty). A catch clause without a parameter makes no environment.
- `src/expression.rs`: `??` and the logical assignments with their short circuit (no PutValue when the assignment does not happen; NamedEvaluation of an anonymous function on an identifier target); optional chains with short circuit to `undefined`, a call in a chain keeping its `this`, `delete` of a short-circuited chain answering `true`.
- `src/expression.rs`, `src/bytecode.rs`, `src/disasm.rs`: a method is a closure of kind `Method` defined on the object by a new instruction `DefineMethod(k)` (`sec-runtime-semantics-methoddefinitionevaluation`, DefineMethodProperty: `{ [[Value]]: f, [[Writable]]: true, [[Enumerable]]: true, [[Configurable]]: true }`, and MakeMethod with the object as [[HomeObject]], as `DefineGetter` does today), documented in the contract as the others; `disasm.rs` prints the kind `method`. A shorthand property is `DefineField` of the value of its IdentifierReference. `src/expression.rs` emits `ThrowSyntaxError` with `swell_syntax::REGEXP_REFUSAL` and drops its own constant.
- `src/resolve.rs`: the walk visits the new nodes; the IdentifierReference of a shorthand property resolves as any reference (no binding changes).

`swell`: `src/interp/dispatch.rs` runs `DefineMethod` (MakeMethod through `make_method`, then the data property). A function of kind `Method` gets no [[Construct]] and no `prototype` property without a change to `src/object/ecmascript_function.rs`: `constructor` is true only for `FunctionKind::Declaration` and `FunctionKind::Expression` there (line 351, inferred from the code), which the tests below check.

Tests, each source file starting with the comment that names its clause anchor and expected result, as the files of slice 1 do:

- Tests of an earlier wave whose expectation changes: under `crates/syntax/tests/sources/refused/` the files `asi-closing-brace.js`, `asi-do-while.js`, `asi-end-of-input.js`, `asi-line-terminator.js`, `asi-restricted-break.js`, `asi-restricted-postfix.js`, `asi-restricted-return.js`, `asi-variable-statement.js`, `labelled-statement.js`, `break-with-label.js`, `continue-with-label.js`, `logical-assignment.js`, `logical-assignment-and.js`, `logical-assignment-nullish.js`, `nullish-coalescing.js`, `optional-chaining.js`, `optional-catch-binding.js`, `trailing-comma-in-arguments.js`, `trailing-comma-in-parameters.js`, `numeric-separator.js`, `method-definition.js`, `shorthand-property.js` and `identifier-escape.js` move to `valid/` (the last one with an escape that decodes to ASCII; a new `refused/identifier-escape-outside-ascii.js` keeps slice 5); `exponentiation.js` and `exponentiation-assignment.js` stay in `refused/` and name slice 4; `generator-method.js`, `async-method.js`, `computed-property-name.js`, `with-non-strict.js`, `arguments-in-function.js` and `arguments-in-nested-function.js` stay as they are.
- New `valid/` files: `hashbang.js`, `labelled-continue-nested-loops.js`, `optional-chain-call.js`, `coalesce-parenthesized-with-or.js`, `identifier-escape-in-property-name.js`, `method-named-get-and-set.js`. New `early-errors/` files: `undefined-break-label.js`, `undefined-continue-label.js`, `continue-to-non-iteration-label.js`, `duplicate-label.js`, `coalesce-mixed-with-or.js`, `optional-chain-assignment-target.js`, `logical-assignment-invalid-target.js`, `escaped-keyword-as-identifier.js`, `escaped-keyword-as-keyword.js`, `numeric-separator-trailing.js`, `numeric-separator-doubled.js`, `numeric-separator-after-leading-zero.js`, `hashbang-not-at-start.js`, `asi-throw-line-terminator.js`, `method-duplicate-parameters.js`, `shorthand-reserved-word.js`. New `refused/` file: `cover-initialized-name.js` (slice 6).
- `crates/syntax/tests/sources.rs`: `every_nesting_kind_fits_the_stack_bound` extended as above, and `message_kind_reads_the_regular_expression_literal_refusal` beside `message_kind_tells_refusals_from_early_errors` (`message_kind(REGEXP_REFUSAL)` is `MessageKind::Refusal { slice: 5 }`).
- Tests of an earlier wave that quote the old text of the regular expression refusal, whose expectation changes: `crates/compiler/tests/compile.rs` (line 454) and `crates/compiler/tests/sources/regexp.disasm` (line 2). No test of `crates/swell` or `crates/test262` quotes it (`grep -rn "regular expression literals are not supported"` over `crates/`).
- `crates/compiler/tests/sources/`: `labelled-break.js`, `labelled-continue.js`, `coalesce.js`, `optional-chain.js`, `logical-assignment.js`, `optional-catch.js`, `method-definition.js`, `shorthand-property.js`, each with its `.disasm` written before the code, run by the tests of `crates/compiler/tests/compile.rs`.
- `crates/swell/tests/language/` (new files only, run in both modes by `language_scripts_answer_their_expected_outcome`): `label-break-block.js`, `label-continue-outer-loop.js`, `label-break-through-finally.js`, `asi-return-line-terminator.js`, `asi-postfix-line-terminator.js`, `coalesce-short-circuit.js`, `optional-chain-short-circuit.js`, `optional-call-this.js`, `logical-assignment-short-circuit.js`, `logical-assignment-function-name.js`, `optional-catch-binding.js`, `numeric-separator-value.js`, `identifier-escape-ascii.js`, `hashbang-first-line.js`, `object-method-is-not-a-constructor.js` (`new o.m()` throws a `TypeError`, `o.m` has no `prototype`), `object-method-name-length-and-enumerable.js`, `object-method-this.js`, `object-shorthand-property.js`.

`conformance/test262/expected.txt`: the unit writes the lines of its run in its own tree; with three stories of the wave changing passes, the coordinator writes the file on the wave branch from the gate run after the merges, as it does `Cargo.lock`.

Writes: `crates/syntax/src/lib.rs`, `crates/syntax/src/lexer.rs`, `crates/syntax/src/token.rs`, `crates/syntax/src/ast.rs`, `crates/syntax/src/early.rs`, `crates/syntax/src/scope.rs`, `crates/syntax/src/error.rs`, `crates/syntax/src/parser/{mod,statement,expression,function}.rs`, `crates/syntax/tests/sources.rs`, `crates/syntax/tests/sources/` (`valid/`, `early-errors/`, `refused/` as listed), `crates/compiler/src/{lib,statement,expression,resolve,bytecode,disasm}.rs`, `crates/compiler/tests/compile.rs`, `crates/compiler/tests/sources/` (the sixteen new files and `regexp.disasm`), `crates/swell/src/interp/dispatch.rs`, `crates/swell/tests/language/` (the eighteen new files listed, by name), `conformance/test262/expected.txt` (the unit tree; the wave branch file is the coordinator).

## Acceptance

`cargo test --locked -p swell-syntax -p swell-compiler` and `cargo test --locked -p swell --test language` exit 0 and run the tests and sources named in Change, and `slot task test262` exits 0 with the unit lines in `conformance/test262/expected.txt`, prints `test262-slice 02 5245 9173 0 0 5245 <p> <f> 0` with `<p>` at least 2,125 (a bound, not a target: the 1,075 cases whose first failure is the refusal of a construct this story takes (352 of them an identifier escape, of which 300 decode to ASCII only; 64 a method or a shorthand property), measured by the decomposition, mostly fail next on `eval`, `with`, `arguments` or a later slice, which no measurement before the code can separate), and after `git show main:conformance/test262/expected.txt > .out/expected-base.txt` the comparison `awk 'NR == FNR { if ($1 == "test262") p[$1 " " $2] = $7; if ($1 == "test262-slice") p[$1 " " $2] = $8; next } { k = $1 " " $2; v = ($1 == "test262-slice") ? $8 : $7 } ($1 == "test262" || $1 == "test262-slice") && (k in p) && v < p[k] { print; bad = 1 } END { exit bad }' .out/expected-base.txt conformance/test262/expected.txt` exits 0 (no directory line, not the total and no `test262-slice` line passes fewer cases than on `main`, the base of the wave).

## Out of scope

`with` and `arguments` (refused until `story:with-arguments-and-dynamic-names`); the parse entries of `eval` and `Function`, the mark of a direct `eval` call, eval code and the `Function` constructor (`story:eval-and-function-constructor`); `**` (slice 4); generator and async methods, computed property names, `super`, CoverInitializedName, a function declaration in a block, `let`, `const` and every other construct of slice 6 or later; identifiers outside ASCII (slice 5); the block of section 12 (`story:slice-02-conformance-close`); `AGENTS.md`, `README.md`, the root files (the coordinator).
