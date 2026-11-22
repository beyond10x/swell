---
format: aep.planning-md/3
id: story:default-and-rest-parameters
kind: story
status: implemented
title: 'Default and rest parameters: FunctionDeclarationInstantiation of a parameter list that is not simple'
summary: Lifts the 9 default and 2 rest parameters of surf's harness, for functions, methods, accessors and arrows; 114 failing cases of slice 6 on its list.
relations:
- decomposes: epic:slice-06-bindings-and-iteration
- depends_on: story:arrow-functions
scope:
- confidence: cited
  path: crates/compiler/src/bytecode.rs
- confidence: cited
  path: crates/compiler/src/disasm.rs
- confidence: cited
  path: crates/compiler/src/function.rs
- confidence: cited
  path: crates/compiler/src/resolve.rs
- confidence: cited
  path: crates/swell/src/dynamic_code.rs
- confidence: cited
  path: crates/swell/src/interp/dispatch.rs
- confidence: cited
  path: crates/swell/src/interp/frame.rs
- confidence: cited
  path: crates/swell/src/interp/mod.rs
- confidence: cited
  path: crates/swell/src/interp/parameters.rs
- confidence: cited
  path: crates/swell/src/object/ecmascript_function.rs
- confidence: cited
  path: crates/swell/tests/language/parameters-arrow-default-and-rest.js
- confidence: cited
  path: crates/swell/tests/language/parameters-default-temporal-dead-zone.js
- confidence: cited
  path: crates/swell/tests/language/parameters-default.js
- confidence: cited
  path: crates/swell/tests/language/parameters-length.js
- confidence: cited
  path: crates/swell/tests/language/parameters-rest.js
- confidence: cited
  path: crates/swell/tests/parameters.rs
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
  path: crates/syntax/tests/sources/refused/default-parameter.js
- confidence: cited
  path: crates/syntax/tests/sources/refused/rest-parameter.js
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T23:19:10Z", actor: "human:operator", revision: 5, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-21T23:19:10Z", actor: "human:operator", revision: 6, executor: "agent:coordinator"}
- {from: "active", to: "implemented", at: "2026-11-22T17:07:50Z", actor: "human:operator", revision: 7, decided_on: {"recorded":{"test_result":1}}, executor: "agent:coordinator"}
---
## Context

Slice 6 is delivered before slices 4 and 5 because it closes most of the first gap of the consumer, the testharness.js tests of the HTML parser of surf (`docs/plan/waves.md`, "The order after slice 3"): 191 refused constructs of slice 6 in `resources/testharness.js` and the scripts of `html/syntax/parsing/html5lib_url.html`, measured with the parser of `main` (`6a43e18`), of which 9 default parameters and 2 rest parameters, all 11 in `testharness.js` and all in function declarations and expressions (for example `function shorten_array(arr, offset = 0)`, line 1699, `function assert_wrapper(...args)`, line 1504, and `function(cond, description, timeout=3000, interval=100)`, line 3113). This story lifts the 11, for functions, methods, accessors and arrows.

The base refuses both in `parameter` of `crates/syntax/src/parser/function.rs` (line 207: `rest parameter` at line 209, `default parameter` at line 213); a call binds each argument to its parameter slot before the code of the function runs (`crates/compiler/src/bytecode.rs`, `FunctionTemplate`, line 517: "A call binds `this`, ... its `params`"), and the arguments object is mapped for every non-strict function, "whose parameter list is simple in this slice" (`ArgumentsObject`, line 496). Measured on `main` (`6a43e18`) from the run lines of `slot task test262`: the first failing run of 80 cases of slice 6 ends on the refusal of a default parameter and of 22 on that of a rest parameter (`awk -F'\t' '$2 == "06" && $5 != "Passed" && !($3 in s) { s[$3] = 1; if ($6 ~ /default parameter is not supported/) d++; if ($6 ~ /rest parameter is not supported/) r++ } END { print d, r }' .out/test262/runs.tsv` prints `80 22`); `test262 language/rest-parameters 11 0 0 11 0 11 0`.

The list of this story is `docs/plan/slice-06/story-parameters.cases.txt`: 114 failing cases of slice 6 (176 runs) whose text and harness files use no construct of slice 6 but default or rest parameters, arrow functions and `let` or `const`, no part of a later slice and no builtin of slice 3 missing on the base (`docs/plan/slice-06/scan.awk` and `lists.awk`, as `story:let-const-and-block-scopes` gives the command; a rest parameter is told from a spread element by the `)` and the `{` or `=>` after its name): `language/eval-code` 36, `language/expressions/object` 21, `language/expressions/function` 20, `language/statements/function` 18, `language/expressions/arrow-function` 12, `language/rest-parameters` 5, the rest 2. A case of the list that ends on a part outside this story is reported with that part, and the coordinator takes it off the list by a commit of the list file on the wave branch whose body names that part; the acceptance reads the list as committed there when the wave closes.

Tests and fixtures of earlier waves that rely on the refusal: `crates/syntax/tests/sources/refused/default-parameter.js` (`function f(a = 1) {}`) and `rest-parameter.js` (`function f(...a) {}`), removed; `refused/destructuring-parameter.js` (`function f([a]) {}`) stays. `crates/syntax/tests/sources.rs`: the parameter text `a = 1` that the CreateDynamicFunction test expects to fail (line 716) and the error text it expects of it, `default parameter is not supported before slice 6` (lines 722 to 726); they change to a parameter text the lexer refuses until slice 18, `a <!-- x`, and its text `HTML-like comment is not supported before slice 18`, the probe `story:let-const-and-block-scopes` gives the other refusals, and `("a = 1", "return a")` joins the texts that parse.

Design it builds on: `docs/design/engine.md`, decision 3, "A parameter list that is not simple" (lines 497 to 543, written for this slice; FunctionDeclarationInstantiation, `#sec-functiondeclarationinstantiation`, steps 19 to 30). What this story implements, each point the design's: the function's code binds the parameters, not the call: a call of such a function binds `this`, makes the function environment, keeps its argument list in the frame (charged to the memory budget with the frame and traced with its locals), stores the arguments object when the template asks for one and starts the code, which initializes each parameter in order from the kept list, a parameter uninitialized until its turn (the state of `story:let-const-and-block-scopes`); a simple list keeps the call's binding of today (item 1); a scope for the parameters and `arguments` and, when the list has an expression, a second one for the body's `var` names and function declarations (step 30), a `var` of a parameter's name starting with its value; a record pushed inside the function environment for the parameters when a direct `eval` stands in a parameter expression of non-strict code (step 20); a direct `eval` in a parameter expression adds its `var` bindings to the function scope, one in the body to the body's `var` scope (item 2); the arguments object of such a list is unmapped, also in non-strict code (step 22), and none is made when a parameter is named `arguments` (step 17) or, in a list without an expression, when the body declares a function or a lexical binding of that name (step 18); `length` is ExpectedArgumentCount; binding the parameters is the code's instructions, one step each, and the rest Array a step per element and its allocation (item 3). Step numbers in this story are those of the pinned draft (`ecma262.html`), each `Else` counted as a step of its own.

## Domain relations

- None new. A parameter is a binding of the function environment the call makes; the records and their outer record are those `story:let-const-and-block-scopes` names (inferred from `crates/swell/src/env/mod.rs:144`, the field `outer: Option<Handle>`; no ess/1 document declares this relation).

## Change

Failing tests first: the tests and fixtures below are written before the code and fail on the base (the parser refuses both parameters).

- `swell-syntax`. `parser/function.rs`: FormalParameter with an Initializer and a FunctionRestParameter as the last of FormalParameters with no trailing comma after it (`#sec-function-definitions`), in declarations, expressions, methods and accessors (a setter has one parameter, which may have a default and may not be a rest parameter, `#sec-method-definitions-static-semantics-early-errors`); a pattern as a parameter or after `...` keeps the refusal `destructuring pattern`. `parser/expression.rs`: the same in the ArrowFormalParameters read again from the cover of `story:arrow-functions` (`(a = 1, ...r) =>`). `ast.rs`: a parameter with its initializer, the rest parameter; the field `params` of `Function` keeps its type and holds every parameter's name in order, with the initializers and the rest mark beside it, so that its readers (`scope.rs` line 149, `early.rs` line 460, compiler `resolve.rs` lines 619, 730 and 864, `sources.rs` line 683) read it as today and only those this story changes are written. `early.rs`: a name bound twice in a list that is not simple, also in non-strict code; `"use strict"` in the body of a function whose list is not simple (`#sec-function-definitions-static-semantics-early-errors`) and in the body of an arrow whose list is not simple (`#sec-arrow-function-definitions-static-semantics-early-errors`, the rule `story:arrow-functions` leaves to this story, since no list of an arrow that is not simple parses before it); `scope.rs`: the references of the parameter expressions and their direct `eval`. `crates/syntax/tests/sources.rs`: the change of lines 716 and 722 to 726 named in Context, and the kind `default_parameter`, a parameter initializer nested in a parameter initializer (`x = function (a = function (a = ... 0) {}) {};`), added to `NESTING_KINDS` of `every_nesting_kind_fits_the_stack_bound` in the form the file has on the base of the unit (on `wave/s3-1` an arm of `nested`, line 417, and an entry of the array at line 462), so that the test shows the frames of that recursion fit the parser's stack bound; the unit times `cargo nextest run --locked -p swell-syntax --test sources` before and after its change (29.6 s today; `story:gate-test-steps-within-budget` changes how the binary runs) and reports both times. Fixtures `crates/syntax/tests/sources/valid/parameters-*.js` and `early-errors/parameters-*.js` (new); the two refused fixtures removed.
- `swell-compiler`. `function.rs`: `length` is ExpectedArgumentCount (`#sec-static-semantics-expectedargumentcount`, the parameters before the first default or rest), the binding of the parameters as the design says, each default evaluated only for `undefined` with NamedEvaluation for an anonymous function (`#sec-runtime-semantics-iteratorbindinginitialization`), the rest parameter an Array of the remaining arguments; `resolve.rs`: the scopes of item 2 (the parameters' scope, the record of step 20, the body's `var` scope of step 30) and the unmapped arguments object of step 22 with the cases of steps 17 and 18 that make none; `bytecode.rs`: the new instructions and the fields of `FunctionTemplate` and `ArgumentsObject` they need, documented; `disasm.rs`: their text. Fixtures `crates/compiler/tests/sources/parameters-*.js` and `.disasm` (new).
- `swell`. `interp/parameters.rs` (new): the work of each new instruction in a function of its own (the rest Array through `create_array_from_list` of `crates/swell/src/builtins/object/mod.rs`, read and not written: a step per element and the Array charged to the memory budget), called from one arm each of `interp/dispatch.rs`; nothing added to the frame of `Agent::execute`; `interp/mod.rs`: the `mod` line; `object/ecmascript_function.rs` and `interp/frame.rs`: the call of a function whose list is not simple keeps its argument list in the frame for that code, charged with the frame and traced with its locals (item 1); `dynamic_code.rs`: a direct `eval` in a parameter expression of non-strict code declares its `var` names in the record of step 20, and the scope a direct `eval` adds `var` bindings to is the function scope for one in a parameter expression and the body's `var` scope for one in the body (item 2).
- Tests, each in both collection modes (`test-collect`): `crates/swell/tests/parameters.rs` (new): `a_default_is_evaluated_only_for_undefined_and_left_to_right`, `a_default_reads_an_earlier_parameter_and_a_later_one_throws_reference_error`, `length_counts_the_parameters_before_the_first_default_or_rest`, `rest_collects_the_remaining_arguments_into_a_new_array`, `rest_charges_a_step_per_element` (no `apply`, which charges a step per element itself: `function f(...r) {}` and `function g() {}` are each called by a script of its own with the same 1,000 arguments written out, and `last_entry_steps()` after the script that calls `f` exceeds that after the script that calls `g` by at least 1,000, the two calls differing only in the rest Array), `the_arguments_object_of_a_list_that_is_not_simple_is_unmapped`, `a_closure_in_a_default_does_not_see_the_var_bindings_of_the_body`, `a_direct_eval_in_a_default_declares_outside_the_parameters`. `crates/swell/tests/language/` (new files): `parameters-default.js`, `parameters-default-temporal-dead-zone.js`, `parameters-rest.js`, `parameters-arrow-default-and-rest.js`, `parameters-length.js`. No test of this story uses a template literal or a destructuring pattern.
- Time of the tests ("Tests stay fast", the `AGENTS.md` of the folder that holds the repository): the unit times each test before it adds it, names in its report each one over 5 s with its time and the reason, and adds none over 60 s; a product several tests share is built once inside one test function or is a committed file, never in a `static`; no test makes a network request; the unit runs the test binaries of its change and never the whole gate. Large by design, none expected over 5 s: `rest_charges_a_step_per_element` (two calls of 1,000 arguments each).
- Stack: `STACK_BYTES_PER_REENTRY` and the stack tests do not change and pass on seven eighths of `min_stack_bytes()`; a call path that grows past them is taken apart and reported with its bytes per re-entry before and after.

Writes: `crates/syntax/src/{ast,scope,early}.rs`, `crates/syntax/src/parser/{function,expression}.rs`, `crates/syntax/tests/sources.rs` (lines 716 and 722 to 726, and the nesting kind), `crates/syntax/tests/sources/valid/parameters-*.js` and `early-errors/parameters-*.js` (new), `crates/syntax/tests/sources/refused/{default-parameter,rest-parameter}.js` (removed), `crates/compiler/src/{function,resolve,bytecode,disasm}.rs`, `crates/compiler/tests/sources/parameters-*.js` and `parameters-*.disasm` (new), `crates/swell/src/interp/parameters.rs` (new), `crates/swell/src/interp/{mod,dispatch,frame}.rs`, `crates/swell/src/object/ecmascript_function.rs`, `crates/swell/src/dynamic_code.rs`, `crates/swell/tests/parameters.rs` (new), `crates/swell/tests/language/{parameters-default,parameters-default-temporal-dead-zone,parameters-rest,parameters-arrow-default-and-rest,parameters-length}.js` (new). Not written: `conformance/test262/expected.txt` (the coordinator regenerates it on the wave branch after the merges; the run of this unit prints the lines that differ and the unit commits none), any file under `crates/swell/src/builtins/` (`create_array_from_list` is read there, line 393), the list files under `docs/plan/slice-06/` (the coordinator's). The matches with no wildcard this story adds arms to are over `Instruction` (`bytecode.rs` line 339, `dispatch.rs` line 237), both above; a parameter adds no variant to `Expr` or `Stmt`. Shared: no file with a story at its depth (it is the only one); every file of `crates/syntax` and `crates/compiler` it writes is also written by a story it depends on, which has landed before it starts.

## Acceptance

The tests and fixtures named in Change pass in both collection modes, run by the binaries of this change and not as a whole step: `cargo nextest run --locked -p swell --test parameters --test language --test eval --test dynamic_function --test limits_scripts --test budgets --test eval_stack` (`dynamic_function` is the existing binary of CreateDynamicFunction, whose parameter text now takes defaults and rest) and the same with `--features collect-every-safe-point`, `cargo nextest run --locked -p swell-syntax --test sources` and `cargo nextest run --locked -p swell-compiler --test compile` exit 0, and `slot task clippy` exits 0; `slot task test262` is run once after the change (the slow lane, in the background through the pool, never a test step) and ends with `test262: <n> line(s) differ from conformance/test262/expected.txt` and no other complaint, and on its run lines, with the list as committed on the wave branch when the wave closes, copied by `git show "$WAVE:docs/plan/slice-06/story-parameters.cases.txt" > .out/story-parameters.cases.txt` (`WAVE` the wave branch the unit merges into), (a) `awk -F'\t' 'NR == FNR { want[$1] = 1; next } ($3 in want) && $5 != "Passed" { print $3, $4, $6; bad = 1 } END { exit bad }' .out/story-parameters.cases.txt .out/test262/runs.tsv` exits 0 (every case of that copy passes in every mode; on `6a43e18`, with the list as committed today, it exits 1 and prints 176 runs), (b) `awk -F'\t' '$3 ~ /^test\/language\/(rest-parameters|expressions\/function|statements\/function)\// && $6 ~ /(default|rest) parameter is not supported/' .out/test262/runs.tsv | wc -l` prints `0` (on `6a43e18` `110`), (c) the lines `test262 language/rest-parameters`, `test262 language/expressions/function` and `test262 language/statements/function` of `.out/test262/lines.txt` each read at least B + L - R passed in their seventh field, where B is the passes of that line on the base of the unit (0, 25 and 212 on `6a43e18`), L the cases of that copy under that directory (`grep -c '^test/language/rest-parameters/' .out/story-parameters.cases.txt`; 5 on the list of today, `expressions/function/` 20, `statements/function/` 18) and R the cases under it among the runs (a) prints; it is a bound because a case off the list may pass as well, and (d) with the run of the base commit of the unit made before the change (`slot task test262 TEST262_OUT=.out/test262-base`), `awk -F'\t' 'NR == FNR { if ($5 == "Passed") p[$3 "\t" $4] = 1; next } (($3 "\t" $4) in p) && $5 != "Passed" { print $3, $4, $6 }' .out/test262-base/runs.tsv .out/test262/runs.tsv` prints nothing.

## Out of scope

Patterns as parameters (`story:object-destructuring` for objects, a later story for arrays); spread arguments in a call (a later story, with iterators); generator, async and class methods (slices 7, 10 and 8); the moves of cases and the slice lines of the design (the close of the slice).
