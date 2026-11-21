---
format: aep.planning-md/3
id: story:template-literals
kind: story
status: draft
title: 'Template literals without a tag: the four template tokens and ToString of each substitution'
summary: Lifts the 22 template literals of surf's harness; 43 failing cases of slice 6 on its list; a tagged template is refused by its own name until a later story.
relations:
- decomposes: epic:slice-06-bindings-and-iteration
- depends_on: story:function-apply-and-source-text
scope:
- confidence: cited
  path: crates/compiler/src/bytecode.rs
- confidence: cited
  path: crates/compiler/src/disasm.rs
- confidence: cited
  path: crates/compiler/src/expression.rs
- confidence: cited
  path: crates/compiler/tests/sources/template-literal.disasm
- confidence: cited
  path: crates/compiler/tests/sources/template-literal.js
- confidence: cited
  path: crates/swell/src/interp/dispatch.rs
- confidence: cited
  path: crates/swell/src/interp/mod.rs
- confidence: cited
  path: crates/swell/src/interp/template.rs
- confidence: cited
  path: crates/swell/tests/language/template-literal-escapes-and-line-ends.js
- confidence: cited
  path: crates/swell/tests/language/template-literal-nested.js
- confidence: cited
  path: crates/swell/tests/language/template-literal-no-substitution.js
- confidence: cited
  path: crates/swell/tests/language/template-literal-substitutions.js
- confidence: cited
  path: crates/swell/tests/template_literals.rs
- confidence: cited
  path: crates/syntax/src/ast.rs
- confidence: cited
  path: crates/syntax/src/lexer.rs
- confidence: cited
  path: crates/syntax/src/parser/expression.rs
- confidence: cited
  path: crates/syntax/src/parser/mod.rs
- confidence: cited
  path: crates/syntax/src/token.rs
- confidence: cited
  path: crates/syntax/tests/sources/refused/tagged-template.js
- confidence: cited
  path: crates/syntax/tests/sources/refused/template-literal.js
revision: 3
---
## Context

Slice 6 is delivered before slices 4 and 5 because it closes most of the first gap of the consumer, the testharness.js tests of the HTML parser of surf (`docs/plan/waves.md`, "The order after slice 3"): 191 refused constructs of slice 6 in `resources/testharness.js` and the scripts of `html/syntax/parsing/html5lib_url.html`, measured with the parser of `main` (`6a43e18`), of which 22 template literals (1 in `testharness.js`, 21 in `html/syntax/parsing/resources/test.js`). This story lifts all 22: none is tagged (the one site the rewrite log of the measurement labels a tagged template, `test.js` line 256, is `` return `\\x${...}` ``, an untagged template after `return`).

The base refuses the backtick in the lexer (`crates/syntax/src/lexer.rs`, line 259). Measured on `main` (`6a43e18`) from the run lines of `slot task test262` (`.out/test262/runs.tsv`): the first failing run of 90 cases of slice 6 ends on that refusal (`awk -F'\t' '$2 == "06" && $5 != "Passed" && !($3 in s) { s[$3] = 1; if ($6 ~ /template literal is not supported/) t++ } END { print t }' .out/test262/runs.tsv` prints `90`), 441 runs over all slices; `test262 language/expressions/template-literal 57 0 0 57 0 57 0` and `test262 language/expressions/tagged-template 27 0 0 27 0 27 0` pass nothing.

The list of this story is `docs/plan/slice-06/story-template.cases.txt`: 43 failing cases of slice 6 (86 runs) whose text and harness files use no construct of slice 6 but an untagged template, no part of a later slice and no builtin of slice 3 missing on the base (`docs/plan/slice-06/scan.awk` and `lists.awk`, as `story:let-const-and-block-scopes` gives the command): `language/expressions/template-literal` 39, `optional-chaining` 2, `assignmenttargettype` 2. A case of the list that ends on a part outside this story is reported with that part, and the coordinator takes it off the list in the wave record; two cases that need this story and `story:let-const-and-block-scopes` are on `docs/plan/slice-06/cross-story.tsv`.

What a tagged template needs, for the later story of the slice that brings it (not here): GetTemplateObject (`#sec-gettemplateobject`): for each site and realm one array of the cooked strings and one of the raw strings, both frozen, kept in the [[TemplateMap]] of the realm and traced by the collector; SetIntegrityLevel (`crates/swell/src/ops/objects.rs`, marked dead until the stories of slice 3 that use it remove the mark, so a story that calls it waits for them); a cooked string `undefined` for a NotEscapeSequence; the call of the tag with the template object and the substitutions, as a member call where the tag is a member; and `String.raw` (slice 3, `story:string-access-and-search-methods`) for most of its cases.

Tests of earlier waves that rely on the refusal: `crates/syntax/tests/sources/refused/template-literal.js` (`` var s = `a`; ``). It becomes the refusal of a tagged template.

## Domain relations

- None. A template literal is an expression of the language; it adds no entity and no relation between the records of the engine (the strings it makes are values, charged as any string).

## Change

Failing tests first: the tests and fixtures below are written before the code and fail on the base (the lexer refuses the backtick).

- `swell-syntax`. `lexer.rs` and `token.rs`: the tokens NoSubstitutionTemplate, TemplateHead, TemplateMiddle and TemplateTail (`#sec-template-literal-lexical-components`) with their TV and TRV (`#sec-static-semantics-tv`, `#sec-static-semantics-trv`: a CR and a CR LF are a LF in both), a NotEscapeSequence and a legacy octal escape an early error of an untagged template (`#sec-static-semantics-template-early-errors`); the part after the `}` that closes a substitution is lexed again at the request of the parser, as `rescan_regexp` (`lexer.rs` line 596, `parser/mod.rs` line 307) does for a `/`, also when that `}` was the token the parser had peeked at. `parser/mod.rs`: that request. `parser/expression.rs`: the TemplateLiteral of PrimaryExpression (`#sec-template-literals`), each substitution an Expression with the `in` operator allowed and counted against `parser_nesting` as a parenthesized expression is; a template after a MemberExpression, a CallExpression or `new` (a tagged template) is refused as `tagged template is not supported before slice 6`; an OptionalChain followed by a template is the `SyntaxError` of `#sec-left-hand-side-expressions-static-semantics-early-errors`. `ast.rs`: `Expr::Template` with the cooked strings and the substitutions. Fixtures: `crates/syntax/tests/sources/valid/template-*.js` and `early-errors/template-*.js` (new files); `refused/template-literal.js` is renamed `refused/tagged-template.js` and refuses `` tag`a` ``.
- `swell-compiler`. `expression.rs`: a template evaluates its substitutions left to right, each through ToString (`#sec-template-literals-runtime-semantics-evaluation`: ToString, so an object gives its `toString` first, where `+` calls ToPrimitive with no hint and so `valueOf` first), and concatenates the strings in order; `bytecode.rs`: one instruction, ToString of the top of the stack, documented with its stack effect; `disasm.rs`: its text. Fixture `crates/compiler/tests/sources/template-literal.js` and `.disasm` (new).
- `swell`. `interp/template.rs` (new): the work of that instruction in a function of its own (ToString of `crates/swell/src/ops/conversion.rs`, which is read and not written), called from one arm of `interp/dispatch.rs`; nothing added to the frame of `Agent::execute`; `interp/mod.rs`: the `mod` line. The concatenation charges per code unit and is bounded by `string_length` as `+` is.
- Tests, each in both collection modes (`test-collect`): `crates/swell/tests/template_literals.rs` (new): `a_substitution_calls_to_string_before_value_of`, `substitutions_are_evaluated_left_to_right_once`, `a_template_longer_than_string_length_ends_as_concatenation_does` (a substitution that doubles a string past a `string_length` of 1,024 code units), `substitutions_count_against_parser_nesting` (`parse_script` of 40 nested templates under a `parser_nesting` of 32 answers the `SyntaxError` that names `parser_nesting`). `crates/swell/tests/language/` (new files): `template-literal-no-substitution.js`, `template-literal-substitutions.js`, `template-literal-escapes-and-line-ends.js`, `template-literal-nested.js`. No test of this story uses `let`, `const`, an arrow function, a default or rest parameter or a destructuring pattern; a result is compared by `===` with a string literal.
- Time of the tests ("Tests stay fast", the `AGENTS.md` of the folder that holds the repository): the unit times each test before it adds it, names in its report each one over 5 s with its time and the reason, and adds none over 60 s; a product several tests share is built once inside one test function or is a committed file, never in a `static`; no test makes a network request; the unit runs the test binaries of its change and never the whole gate. None is expected over 1 s.
- Stack: `STACK_BYTES_PER_REENTRY` and the stack tests do not change; the ToString of a substitution re-enters the interpreter as `+` does, on the path the stack tests already measure.

Writes: `crates/syntax/src/{lexer,token,ast}.rs`, `crates/syntax/src/parser/{mod,expression}.rs`, `crates/syntax/tests/sources/valid/template-*.js` and `early-errors/template-*.js` (new), `crates/syntax/tests/sources/refused/template-literal.js` (renamed `refused/tagged-template.js`), `crates/compiler/src/{expression,bytecode,disasm}.rs`, `crates/compiler/tests/sources/template-literal.js` and `template-literal.disasm` (new), `crates/swell/src/interp/template.rs` (new), `crates/swell/src/interp/{mod,dispatch}.rs`, `crates/swell/tests/template_literals.rs` (new), `crates/swell/tests/language/{template-literal-no-substitution,template-literal-substitutions,template-literal-escapes-and-line-ends,template-literal-nested}.js` (new). Not written: `conformance/test262/expected.txt` (the coordinator regenerates it on the wave branch after the merges; the run of this unit prints the lines that differ and the unit commits none), any file under `crates/swell/src/builtins/`, `crates/swell/src/ops/`. Shared with the story beside it, `story:let-const-and-block-scopes`: `crates/syntax/src/ast.rs` (this story `Expr`, that one the statement and scope types), `crates/compiler/src/{expression,bytecode,disasm}.rs` and `crates/swell/src/interp/{mod,dispatch}.rs` (one variant, one arm, one text and one `mod` line here); each story writes its own hunks, and the unit that lands second merges the wave branch into its tree and resolves the overlap (`AGENTS.md`, "How the work is run"). No file of a story of slice 3 that runs beside it: `crates/syntax/src/{ast.rs,parser/expression.rs}` and `crates/compiler/src/bytecode.rs` are written by `story:function-apply-and-source-text` and are written here after it lands, by the edge below.

## Acceptance

The tests and fixtures named in Change pass in both collection modes, run by the binaries of this change and not as a whole step: `cargo nextest run --locked -p swell --test template_literals --test language --test limits_scripts` and the same with `--features collect-every-safe-point`, `cargo nextest run --locked -p swell-syntax --test sources` and `cargo nextest run --locked -p swell-compiler --test compile` exit 0, and `slot task clippy` exits 0; `slot task test262` is run once after the change (the slow lane, in the background through the pool, never a test step) and ends with `test262: <n> line(s) differ from conformance/test262/expected.txt` and no other complaint, and on its run lines (a) `awk -F'\t' 'NR == FNR { want[$1] = 1; next } ($3 in want) && $5 != "Passed" { print $3, $4, $6; bad = 1 } END { exit bad }' docs/plan/slice-06/story-template.cases.txt .out/test262/runs.tsv` exits 0 (every case of the list, less the ones the wave record takes off it, passes in every mode; on the base it exits 1 and prints 86 runs), (b) `awk -F'\t' '$3 ~ /^test\/language\/expressions\/template-literal\// && $6 ~ /template literal is not supported/' .out/test262/runs.tsv | wc -l` prints `0` (on the base `112`), (c) the line `test262 language/expressions/template-literal` of `.out/test262/lines.txt` reads at least B + L - R passed in its seventh field, where B is its passes on the base of the unit (0 on `6a43e18`), L the cases of the list under that directory as the list is committed when the wave closes (`grep -c '^test/language/expressions/template-literal/' docs/plan/slice-06/story-template.cases.txt` prints 39 today) and R the cases under it among the runs (a) prints; it is a bound because a case off the list may pass as well, and (d) with the run of the base commit of the unit made before the change (`slot task test262 TEST262_OUT=.out/test262-base`), `awk -F'\t' 'NR == FNR { if ($5 == "Passed") p[$3 "\t" $4] = 1; next } (($3 "\t" $4) in p) && $5 != "Passed" { print $3, $4, $6 }' .out/test262-base/runs.tsv .out/test262/runs.tsv` prints nothing.

## Out of scope

Tagged templates, the template objects and their registry, `String.raw` with a template (a later story of the slice, after the stories of slice 3 that use SetIntegrityLevel); the cooked value `undefined` of a NotEscapeSequence, which only a tagged template has; the moves of cases and the slice lines of the design (the close of the slice).
