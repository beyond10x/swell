---
format: aep.planning-md/3
id: story:arrow-functions
kind: story
status: active
title: 'Arrow functions: the this and arguments of the code around them, no construct, their source text'
summary: Lifts the 41 arrow functions of surf's harness (two take object patterns and parse after object-destructuring); 121 failing cases of slice 6 on its list.
relations:
- decomposes: epic:slice-06-bindings-and-iteration
- depends_on: story:let-const-and-block-scopes
- depends_on: story:template-literals
scope:
- confidence: cited
  path: crates/compiler/src/bytecode.rs
- confidence: cited
  path: crates/compiler/src/disasm.rs
- confidence: cited
  path: crates/compiler/src/expression.rs
- confidence: cited
  path: crates/compiler/src/function.rs
- confidence: cited
  path: crates/compiler/src/resolve.rs
- confidence: cited
  path: crates/swell/src/interp/arrow.rs
- confidence: cited
  path: crates/swell/src/interp/dispatch.rs
- confidence: cited
  path: crates/swell/src/interp/mod.rs
- confidence: cited
  path: crates/swell/src/object/ecmascript_function.rs
- confidence: cited
  path: crates/swell/tests/arrow_functions.rs
- confidence: cited
  path: crates/swell/tests/language/arrow-function-block-body.js
- confidence: cited
  path: crates/swell/tests/language/arrow-function-expression-body.js
- confidence: cited
  path: crates/swell/tests/language/arrow-function-lexical-arguments.js
- confidence: cited
  path: crates/swell/tests/language/arrow-function-lexical-this.js
- confidence: cited
  path: crates/swell/tests/language/arrow-function-name.js
- confidence: cited
  path: crates/syntax/src/ast.rs
- confidence: cited
  path: crates/syntax/src/early.rs
- confidence: cited
  path: crates/syntax/src/parser/expression.rs
- confidence: cited
  path: crates/syntax/src/parser/function.rs
- confidence: cited
  path: crates/syntax/src/scope.rs
- confidence: cited
  path: crates/syntax/tests/sources.rs
- confidence: cited
  path: crates/syntax/tests/sources/refused/arrow-function-empty.js
- confidence: cited
  path: crates/syntax/tests/sources/refused/arrow-function-parenthesized.js
- confidence: cited
  path: crates/syntax/tests/sources/refused/arrow-function.js
- confidence: cited
  path: crates/test262/tests/fixtures/refused/harness/refused.js
- confidence: cited
  path: crates/test262/tests/fixtures/refused/test/language/refused/negative.js
- confidence: cited
  path: crates/test262/tests/fixtures/refused/test/language/refused/positive.js
- confidence: cited
  path: crates/test262/tests/run.rs
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T23:19:10Z", actor: "human:operator", revision: 5, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-21T23:19:10Z", actor: "human:operator", revision: 6, executor: "agent:coordinator"}
---
## Context

Slice 6 is delivered before slices 4 and 5 because it closes most of the first gap of the consumer, the testharness.js tests of the HTML parser of surf (`docs/plan/waves.md`, "The order after slice 3"): 191 refused constructs of slice 6 in `resources/testharness.js` and the scripts of `html/syntax/parsing/html5lib_url.html`, measured with the parser of `main` (`6a43e18`), of which 41 arrow functions (11 in `testharness.js`, 30 in `html/syntax/parsing/resources/test.js`, whose whole code is the body of one arrow, line 10). This story lifts the 41; two of them take an object pattern as their parameter (`test.js` lines 95 and 125) and parse once `story:object-destructuring` has landed. By the count of the measurement arrows come second after `let` and `const` (98); this story runs after `story:let-const-and-block-scopes` and not beside it because both rewrite the scope analysis and the resolution of names (`crates/syntax/src/scope.rs`, `crates/compiler/src/resolve.rs`), and after `story:template-literals` so that no file of this story is shared with a story at its depth.

The base refuses `=>` in `crates/syntax/src/parser/expression.rs`: `assign_tail` (line 157, after an operand) and `paren_open` (lines 803 and 808, `()` and `(...` before `=>`). Measured on `main` (`6a43e18`) from the run lines of `slot task test262`: the first failing run of 218 cases of slice 6 ends on that refusal (`awk -F'\t' '$2 == "06" && $5 != "Passed" && !($3 in s) { s[$3] = 1; if ($6 ~ /arrow function is not supported/) t++ } END { print t }' .out/test262/runs.tsv` prints `218`), and 2,297 runs over all slices end on it; `test262 language/expressions/arrow-function 343 0 0 343 45 298 0`.

The list of this story is `docs/plan/slice-06/story-arrow.cases.txt`: 121 failing cases of slice 6 (185 runs) whose text and harness files use no construct of slice 6 but arrow functions and `let` or `const`, no part of a later slice and no builtin of slice 3 missing on the base (`docs/plan/slice-06/scan.awk` and `lists.awk`, as `story:let-const-and-block-scopes` gives the command): `language/expressions/arrow-function` 48, `compound-assignment` 22, `assignmenttargettype` 13, `language/statementList` 9, `test/harness` 7, the rest 22. A case of the list that ends on a part outside this story is reported with that part, and the coordinator takes it off the list by a commit of the list file on the wave branch whose body names that part; the acceptance reads the list as committed there when the wave closes.

Tests and fixtures of earlier waves that use an arrow function as the construct a parser refuses: `crates/syntax/tests/sources/refused/arrow-function.js`, `arrow-function-empty.js` and `arrow-function-parenthesized.js`; `crates/test262/tests/fixtures/refused/harness/refused.js` (`var identity = x => x;`), `crates/test262/tests/fixtures/refused/test/language/refused/negative.js` and `positive.js` (`var f = a => a;`), and the comment of `a_refused_negative_parse_case_is_counted_failed` in `crates/test262/tests/run.rs` (lines 332 and 333) that names the arrow. The three fixtures of the runner change to the HTML-like comment of slice 18 (`<!-- x`) that `story:let-const-and-block-scopes` gives the other refusal probes, the comment names it, and the counts the tests expect stay.

Design it builds on: `docs/design/engine.md`, decision 3, "`this` and `arguments` in an arrow function" (lines 450 to 495, written for this slice), and rule 4 of "Names" (line 323). What this story implements, each point the design's: `this` is taken when the arrow is made: the closure instruction of an arrow stores the `this` value of the frame that makes it in the function object, which the collector traces, and a call of the arrow puts that value in its frame's `this` and runs no OrdinaryCallBindThis (`#sec-ordinarycallbindthis`, step 2); the `This` instruction and the `this` a direct `eval` takes from its caller's frame (`dynamic_code.rs`) stay as they are, so an arrow in an arrow and eval code in an arrow get it too (item 1); `arguments` resolves outward like any name: the scope analysis counts an arrow's references to `arguments` and its direct `eval` for the nearest enclosing function that is not an arrow, which then makes its arguments object, and an arrow with a parameter or a declaration named `arguments` binds that name itself (item 2, rule 4 of "Names"); a direct `eval` in an arrow puts its `var` names in the arrow's own VariableEnvironment; an arrow has no [[Construct]] and no `prototype`. `new.target` and `super` in an arrow stay refused with slice 8, whose derived constructors change item 1 for their arrows (item 3).

## Domain relations

- Arrow function object → the `this` value of the code that made it, many-to-one, the function keeps the value alive (traced by the collector) — inferable only once the design states it (Design above); today a function object references its [[Environment]] and its compiled code only (inferred from `crates/swell/src/object/ecmascript_function.rs:50`, the struct `EcmascriptFunction`; no ess/1 document declares this relation).

## Change

Failing tests first: the tests and fixtures below are written before the code and fail on the base (the parser refuses `=>`).

- `swell-syntax`. `parser/expression.rs`: ArrowFunction (`#sec-arrow-function-definitions`): `x => ...` and the CoverParenthesizedExpressionAndArrowParameterList read again as ArrowFormalParameters (`()`, `(a, b)`, a trailing comma), with no line terminator before `=>`; a ConciseBody (an AssignmentExpression, with `in` as the context allows) or a FunctionBody; a parameter with a default, a rest parameter or a pattern keeps the refusal it has (`default parameter`, `rest parameter`, `destructuring pattern`), and `async` arrows keep the refusal of slice 10. `parser/function.rs`: the body of an arrow and its span, from the first token of its parameters to the end of its body, as `story:function-apply-and-source-text` gives the spans of the other functions (`#sec-runtime-semantics-instantiatearrowfunctionexpression`: "the source text matched by ArrowFunction"). `ast.rs`: `FunctionKind::Arrow` and the concise body. `early.rs`: the early errors of `#sec-arrow-function-definitions-static-semantics-early-errors` (a name bound twice in the parameters, also in non-strict code; a parameter that the body declares lexically; `eval` and `arguments` as parameters in strict code); its rule for `"use strict"` in the body of an arrow whose parameter list is not simple belongs to `story:default-and-rest-parameters`, the first story in which such a list parses. `scope.rs`: an arrow is a function scope with no `this` and no `arguments` of its own: its references to `arguments` and its direct `eval` count for the nearest enclosing non-arrow function (`Scope::arguments_object`, rule 4 of "Names"). `FunctionKind::Arrow` gets its arms in the matches with no wildcard over `FunctionKind`: `parser/function.rs` line 126 and compiler `disasm.rs` line 62. `crates/syntax/tests/sources.rs`: the kinds `arrow` (a concise body nested in a concise body, `x = () => () => ... 0;`) and `arrow_block` (`x = () => { return () => { return ... 0; }; };`), added to `NESTING_KINDS` of `every_nesting_kind_fits_the_stack_bound` in the form the file has on the base of the unit (on `wave/s3-1` an arm of `nested`, line 417, and an entry of the array at line 462), so that the test shows the frames of that recursion fit the parser's stack bound; the unit times `cargo nextest run --locked -p swell-syntax --test sources` before and after its change (29.6 s today; `story:gate-test-steps-within-budget` changes how the binary runs) and reports both times. Fixtures `crates/syntax/tests/sources/valid/arrow-*.js` and `early-errors/arrow-*.js` (new); the three refused arrow fixtures are removed.
- `swell-compiler`. `resolve.rs`: the references of an arrow to `arguments` resolve outward; `function.rs`: the template of an arrow (no [[Construct]], no `prototype`, NamedEvaluation as for an anonymous function expression, `#sec-runtime-semantics-namedevaluation`); `expression.rs`: `this` in an arrow, and the closure of an arrow taking the `this` of its frame as the design says; `bytecode.rs`: what the template and the instruction carry, documented; `disasm.rs`: their text. Fixtures `crates/compiler/tests/sources/arrow-*.js` and `.disasm` (new: an expression body, a block body, `this` and `arguments` of the enclosing function, an arrow in an arrow).
- `swell`. `object/ecmascript_function.rs`: [[ThisMode]] lexical, the kept `this` traced by the collector, OrdinaryCallBindThis that binds nothing for an arrow, no [[Construct]] (so `new` throws the `TypeError` of a value that is not a constructor) and no `prototype` property; `interp/dispatch.rs`: the arm of the closure of an arrow calls a function of its own in `interp/arrow.rs` (new); nothing added to the frame of `Agent::execute`; `interp/mod.rs`: the `mod` line. `dynamic_code.rs` is not written: the `this` a direct `eval` takes from its caller's frame stays as it is (item 1).
- The refusal probes of the runner named in Context: `crates/test262/tests/fixtures/refused/harness/refused.js`, `crates/test262/tests/fixtures/refused/test/language/refused/{negative,positive}.js` and the comment at lines 332 and 333 of `crates/test262/tests/run.rs`.
- Tests, each in both collection modes (`test-collect`): `crates/swell/tests/arrow_functions.rs` (new): `an_arrow_takes_this_from_the_code_that_made_it` (a method, a function called with `call`, the script), `an_arrow_reads_and_writes_the_arguments_of_the_enclosing_function`, `an_arrow_is_not_a_constructor_and_has_no_prototype`, `the_source_text_of_an_arrow_is_its_text` (`Function.prototype.toString`, delivered by `story:function-apply-and-source-text`), `an_arrow_kept_after_its_function_returned_still_sees_its_this` (with a `collect()` between), `calls_of_arrows_reach_the_reentry_limit_on_the_promised_stack` (through `reach_through` of `tests/support/stack.rs`, on seven eighths of `min_stack_bytes()`). `crates/swell/tests/language/` (new files): `arrow-function-expression-body.js`, `arrow-function-block-body.js`, `arrow-function-lexical-this.js`, `arrow-function-lexical-arguments.js`, `arrow-function-name.js`. No test of this story uses a template literal, a default or rest parameter or a destructuring pattern.
- Time of the tests ("Tests stay fast", the `AGENTS.md` of the folder that holds the repository): the unit times each test before it adds it, names in its report each one over 5 s with its time and the reason, and adds none over 60 s; a product several tests share is built once inside one test function or is a committed file, never in a `static`; no test makes a network request; the unit runs the test binaries of its change and never the whole gate. Large by design, none expected over 5 s: the re-entry test (one run; 0.04 s and 0.45 s with `collect-every-safe-point` for such a run on the base, measured by `story:function-apply-and-source-text`).
- Stack: `STACK_BYTES_PER_REENTRY` and the stack tests do not change and pass on seven eighths of `min_stack_bytes()`; a call path that grows past them is taken apart and reported with its bytes per re-entry before and after.

Writes: `crates/syntax/src/{ast,scope,early}.rs`, `crates/syntax/src/parser/{expression,function}.rs`, `crates/syntax/tests/sources.rs` (the two nesting kinds), `crates/syntax/tests/sources/valid/arrow-*.js` and `early-errors/arrow-*.js` (new), `crates/syntax/tests/sources/refused/{arrow-function,arrow-function-empty,arrow-function-parenthesized}.js` (removed), `crates/compiler/src/{resolve,function,expression,bytecode,disasm}.rs`, `crates/compiler/tests/sources/arrow-*.js` and `arrow-*.disasm` (new), `crates/swell/src/object/ecmascript_function.rs`, `crates/swell/src/interp/arrow.rs` (new), `crates/swell/src/interp/{mod,dispatch}.rs`, `crates/swell/tests/arrow_functions.rs` (new), `crates/swell/tests/language/{arrow-function-expression-body,arrow-function-block-body,arrow-function-lexical-this,arrow-function-lexical-arguments,arrow-function-name}.js` (new), `crates/test262/tests/run.rs` (lines 332 and 333), `crates/test262/tests/fixtures/refused/harness/refused.js`, `crates/test262/tests/fixtures/refused/test/language/refused/{negative,positive}.js`. Not written: `conformance/test262/expected.txt` (the coordinator regenerates it on the wave branch after the merges; the run of this unit prints the lines that differ and the unit commits none), any file under `crates/swell/src/builtins/`, `crates/swell/src/dynamic_code.rs`, the list files under `docs/plan/slice-06/` (the coordinator's). Shared: no file with a story at its depth (it is the only one); every file of `crates/syntax` and `crates/compiler` it writes is also written by a story it depends on, which has landed before it starts.

## Acceptance

The tests and fixtures named in Change pass in both collection modes, run by the binaries of this change and not as a whole step: `cargo nextest run --locked -p swell --test arrow_functions --test language --test eval --test limits_scripts --test budgets --test eval_stack` and the same with `--features collect-every-safe-point`, `cargo nextest run --locked -p swell-syntax --test sources`, `cargo nextest run --locked -p swell-compiler --test compile` and `cargo nextest run --locked -p swell-test262 --test run` exit 0, and `slot task clippy` exits 0; `slot task test262` is run once after the change (the slow lane, in the background through the pool, never a test step) and ends with `test262: <n> line(s) differ from conformance/test262/expected.txt` and no other complaint, and on its run lines, with the list as committed on the wave branch when the wave closes, copied by `git show "$WAVE:docs/plan/slice-06/story-arrow.cases.txt" > .out/story-arrow.cases.txt` (`WAVE` the wave branch the unit merges into), (a) `awk -F'\t' 'NR == FNR { want[$1] = 1; next } ($3 in want) && $5 != "Passed" { print $3, $4, $6; bad = 1 } END { exit bad }' .out/story-arrow.cases.txt .out/test262/runs.tsv` exits 0 (every case of that copy passes in every mode; on `6a43e18`, with the list as committed today, it exits 1 and prints 185 runs), (b) `awk -F'\t' '$3 ~ /^test\/language\/expressions\/arrow-function\// && $6 ~ /arrow function is not supported/' .out/test262/runs.tsv | wc -l` prints `0` (on `6a43e18` `224`), (c) the line `test262 language/expressions/arrow-function` of `.out/test262/lines.txt` reads at least B + L - R passed in its seventh field, where B is its passes on the base of the unit (45 on `6a43e18`), L the cases of that copy under that directory (`grep -c '^test/language/expressions/arrow-function/' .out/story-arrow.cases.txt`; 48 on the list of today) and R the cases under it among the runs (a) prints; it is a bound because a case off the list may pass as well, and (d) with the run of the base commit of the unit made before the change (`slot task test262 TEST262_OUT=.out/test262-base`), `awk -F'\t' 'NR == FNR { if ($5 == "Passed") p[$3 "\t" $4] = 1; next } (($3 "\t" $4) in p) && $5 != "Passed" { print $3, $4, $6 }' .out/test262-base/runs.tsv .out/test262/runs.tsv` prints nothing.

## Out of scope

Default and rest parameters of arrows (`story:default-and-rest-parameters`), patterns as their parameters (`story:object-destructuring`, a later story for arrays); `async` arrows (slice 10); `new.target` and `super` in an arrow (slice 8); the moves of cases and the slice lines of the design (the close of the slice).
