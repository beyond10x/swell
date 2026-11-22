---
format: aep.planning-md/3
id: story:for-of-statement
kind: story
status: draft
title: 'for-of statements: head and body evaluation, per-iteration bindings, IteratorClose on every abrupt exit'
summary: Lifts 10 of the 12 for-of statements of surf's harness (two have an array pattern in the head); 250 failing cases of slice 6 on its list.
relations:
- decomposes: epic:slice-06-bindings-and-iteration
- depends_on: story:iterator-protocol-and-array-string-iterators
- depends_on: story:block-function-declarations
scope:
- confidence: cited
  path: crates/compiler/src/bytecode.rs
- confidence: cited
  path: crates/compiler/src/disasm.rs
- confidence: cited
  path: crates/compiler/src/resolve.rs
- confidence: cited
  path: crates/compiler/src/statement.rs
- confidence: cited
  path: crates/swell/src/interp/dispatch.rs
- confidence: cited
  path: crates/swell/src/interp/for_of.rs
- confidence: cited
  path: crates/swell/src/interp/mod.rs
- confidence: cited
  path: crates/swell/src/ops/iteration.rs
- confidence: cited
  path: crates/swell/tests/for_of.rs
- confidence: cited
  path: crates/swell/tests/language/for-of-array.js
- confidence: cited
  path: crates/swell/tests/language/for-of-close.js
- confidence: cited
  path: crates/swell/tests/language/for-of-let-per-iteration.js
- confidence: cited
  path: crates/swell/tests/language/for-of-string.js
- confidence: cited
  path: crates/syntax/src/ast.rs
- confidence: cited
  path: crates/syntax/src/early.rs
- confidence: cited
  path: crates/syntax/src/parser/statement.rs
- confidence: cited
  path: crates/syntax/src/scope.rs
- confidence: cited
  path: crates/syntax/tests/sources.rs
- confidence: cited
  path: crates/syntax/tests/sources/refused/for-of-var.js
- confidence: cited
  path: crates/syntax/tests/sources/refused/for-of.js
revision: 2
---
## Context

Slice 6 is delivered before slices 4 and 5 because it closes most of the first gap of the consumer, the testharness.js tests of the HTML parser of surf (`docs/plan/waves.md`, "The order after slice 3"): 191 refused constructs of slice 6 in `resources/testharness.js` and the scripts of `html/syntax/parsing/html5lib_url.html`, measured with the parser of `main` (`6a43e18`). Of the 191, 12 are `for`-`of` statements: 4 in `testharness.js` (lines 2550, 3540, 4553, 4582) and 8 in `html/syntax/parsing/resources/test.js` (lines 23, 75, 80, 85, 122, 171, 259, 334). This story lifts the 12; two of them have an array pattern in their head (`testharness.js` line 2550, `test.js` line 75) and parse once `story:array-destructuring-and-spread` has landed, which is the story after which both scripts parse with no refusal of slice 6 (the command is in that story). The first failing run of 210 cases of slice 6 ends on `for-of statement is not supported` (`awk -F'\t' '$2 == "06" && $5 != "Passed" && !($3 in s) { s[$3] = 1; if ($6 ~ /for-of statement is not supported/) c++ } END { print c }' .out/test262/runs.tsv` prints `210`; 602 runs over all slices end on it).

The base refuses the statement after its head is read, when `of` follows the first part (`crates/syntax/src/parser/statement.rs`, line 582, in the statement that `for_head` at line 565 begins); `story:let-const-and-block-scopes` keeps the refusal for `for (let x of ...)` and `for (const x of ...)`, and `story:object-destructuring` for an object pattern in the head. Fixtures that rely on it: `crates/syntax/tests/sources/refused/for-of.js` and `for-of-var.js`, removed.

The list of this story is `docs/plan/slice-06/story-for-of.cases.txt`: 250 failing cases (479 runs) of slice 6 and of the six directories `story:slice-03-conformance-close` moves to slice 6 (`test/built-ins/Array/from/`, `Array/prototype/keys/`, `values/`, `entries/`, `Object/fromEntries/`, `Object/groupBy/`), on no list of the first five stories of the slice, whose text and harness files use a `for`-`of` statement, no other part of slice 6 than those of the stories this one depends on, and no part of a later slice. Found by the token scan `docs/plan/slice-06/scan.awk` (run as `story:let-const-and-block-scopes` gives it, but with a copy of `meta.tsv` whose includes leave out `propertyHelper.js`, whose three later tokens are a branch behind `typeof`, a case of a bigint value and the name `checkSet`; the comment at the head of `lists-b.awk` gives the command; over the first failing run of each failing case of slice 6 and of those six directories on `main`, `6a43e18`) and the assignment `docs/plan/slice-06/lists-b.awk` (`gawk -v root=<checkout> -v meta=.cache/inventory/meta.tsv -f docs/plan/slice-06/lists-b.awk <scan output>`, the cases of the five lists of the first part taken out), which puts a case on the list of the first story of the slice that holds every part the case needs and whose own construct the case uses. By directory: `language/statements/for-of` 245, `language/statements/for-in`, `language/statements/const`, `language/expressions/optional-chaining`, `language/expressions/assignmenttargettype` and `built-ins/Array` 1 each. A case of the list that ends on a part outside this story is reported with that part, and the coordinator takes it off the list by a commit of the list file on the wave branch whose body names that part; the acceptance reads the list as committed there when the wave closes.

Design it builds on, a precondition (named by the decomposition for a design author, before this story starts): `docs/design/engine.md`, decision 3, the loop over an iterator in the frame: where the iterator record of a running `for`-`of` lives (on the operand stack, below the values of the body, traced as stack values), how IteratorClose (`#sec-iteratorclose`) is reached on every exit that is not the end of the iteration (a `break`, a `continue` or `break` to an outer label, a `return`, a throw from the body, not a throw from `next` or from reading `done` or `value`, `#sec-runtime-semantics-forin-div-ofbodyevaluation-lhs-stmt-iterator-lhskind-labelset`), as a handler of the handler table or code the compiler emits on each exit path, and the charge of a step per iteration. The per-iteration bindings of `let` and `const` in the head follow item 3 of "Lexical bindings" (written for slice 6).

## Domain relations

- None new (the iterator record is held by the frame for the life of the loop; no record of the engine gains a relation).

## Change

Failing tests first: the tests and fixtures below are written before the code and fail on the base (the parser refuses the statement).

- `swell-syntax`. `parser/statement.rs`: `for (LeftHandSideExpression of AssignmentExpression)`, `for (var ForBinding of ...)` and `for (ForDeclaration of ...)` with `let` and `const` (`#sec-for-in-and-for-of-statements`), the head expression an AssignmentExpression with `in` allowed; an object pattern in the head (`story:object-destructuring`); an array pattern keeps the refusal `destructuring pattern`; `for await` keeps the refusal of slice 10. `ast.rs`: the statement (a variant of `Stmt` or a kind of the `for`-`in` statement), with its arms in `Stmt::children` (line 590, no wildcard) and, if it reuses `ForInTarget`, in the matches over it. `scope.rs`: the names of the head. `early.rs`: `#sec-for-in-and-for-of-statements-static-semantics-early-errors` (a head `let` as an identifier in `for (let of ...)`, `async of`, a target that is not a valid simple assignment target, a name bound twice, a body that declares a head name lexically). `crates/syntax/tests/sources.rs`: the kind `for_of` (`for (x of a) for (x of a) ... ;`) added to `NESTING_KINDS` of `every_nesting_kind_fits_the_stack_bound` in the form the file has on the base of the unit (on `wave/s3-1` an arm of `nested`, line 417, and an entry of the array at line 462); the unit times `cargo nextest run --locked -p swell-syntax --test sources` before and after its change (29.6 s on `wave/s3-1`; `story:gate-test-steps-within-budget` changes how the binary runs) and reports both times. Fixtures `crates/syntax/tests/sources/valid/for-of-*.js` and `early-errors/for-of-*.js` (new); the two refused fixtures removed.
- `swell-compiler`. `statement.rs`: ForIn/OfHeadEvaluation with the scope of the head names (`#sec-runtime-semantics-forinofheadevaluation`), ForIn/OfBodyEvaluation (`#sec-runtime-semantics-forinofloopevaluation`) with the iterator record of the design, the per-iteration record of item 3 of "Lexical bindings", the completion value, and IteratorClose on every exit the design names; `resolve.rs`: the scopes of the head and the body, and the arms of the statement in the matches over `Stmt` (line 351) and `ForInTarget` (line 422), no wildcard; compiler `statement.rs`: the arms in the matches at lines 52 and 201 (no wildcard); `bytecode.rs`: the new instructions (GetIterator of a value, the step that jumps when done, IteratorClose), each documented with its stack effect and its arm in `Instruction::target` (line 339, no wildcard), and a handler kind if the design makes IteratorClose a handler; `disasm.rs`: their text and, for a new handler kind, its arm in the `HandlerKind` match (line 142, no wildcard). Fixtures `crates/compiler/tests/sources/for-of-*.js` and `.disasm` (new: over an expression, with `var`, `let` and `const`, `break`, `continue` to an outer label, `return` inside, a closure per iteration).
- `swell`. `interp/for_of.rs` (new): the work of each new instruction in a function of its own, through the operations of `crates/swell/src/ops/iteration.rs`, called from one arm each of `interp/dispatch.rs` (line 237, no wildcard; and the entry of a handler of a new kind); nothing added to the frame of `Agent::execute`; `interp/mod.rs`: the `mod` line; `ops/iteration.rs`: the marks of the operations this story is the first to call are removed (mark lines only). A step is charged per iteration (the backward jump of the loop is a `LoopJump`, as for the other loops) and per `next`, `done` and `value` read as the operations charge.
- Tests, each in both collection modes (`test-collect`): `crates/swell/tests/for_of.rs` (new): `for_of_iterates_an_array_a_string_and_arguments`, `for_of_calls_return_once_on_break_continue_outer_return_and_a_throw_from_the_body`, `for_of_does_not_call_return_when_next_throws_or_the_iterator_is_done`, `a_throw_of_return_does_not_hide_the_throw_from_the_body`, `let_in_the_head_gives_each_iteration_its_binding` (closures of each iteration, with `var` as the control that shares one binding), `for_of_charges_each_iteration` (`last_entry_steps()` after a loop over 1,000 elements exceeds that after a loop over 10, both arrays made by an earlier script, by at least 990), `a_user_iterator_re_enters_on_the_promised_stack` (`reach_through` of `tests/support/stack.rs`, on seven eighths of `min_stack_bytes()`). `crates/swell/tests/language/` (new files): `for-of-array.js`, `for-of-string.js`, `for-of-close.js`, `for-of-let-per-iteration.js`. No test uses an array pattern or a spread (`story:array-destructuring-and-spread`).
- Time of the tests ("Tests stay fast", the `AGENTS.md` of the folder that holds the repository): the unit times each test before it adds it, names in its report each one over 5 s with its time and the reason, and adds none over 60 s; a product several tests share is built once inside one test function or is a committed file, never in a `static` (the test steps run a process per test); no test makes a network request; the unit runs the test binaries of its change and never the whole gate. None is expected over 1 s.
- Run loop: each new instruction does its work in a function of its own in `interp/for_of.rs`, called from one arm of `interp/dispatch.rs`; nothing is added to the frame of `Agent::execute`; `STACK_BYTES_PER_REENTRY` and the stack tests (`limits_scripts.rs`, `budgets.rs`, `eval_stack.rs`) do not change and pass on seven eighths of `min_stack_bytes()`; a path that does not fit is taken apart and reported with its bytes per re-entry before and after.

Writes: `crates/syntax/src/{ast,scope,early}.rs`, `crates/syntax/src/parser/statement.rs`, `crates/syntax/tests/sources.rs` (one nesting kind), `crates/syntax/tests/sources/valid/for-of-*.js` and `early-errors/for-of-*.js` (new), `crates/syntax/tests/sources/refused/{for-of,for-of-var}.js` (removed), `crates/compiler/src/{statement,resolve,bytecode,disasm}.rs`, `crates/compiler/tests/sources/for-of-*.js` and `.disasm` (new), `crates/swell/src/interp/for_of.rs` (new), `crates/swell/src/interp/{mod,dispatch}.rs`, `crates/swell/src/ops/iteration.rs` (mark lines only), `crates/swell/tests/for_of.rs` (new), `crates/swell/tests/language/{for-of-array,for-of-string,for-of-close,for-of-let-per-iteration}.js` (new). Not written: `conformance/test262/expected.txt` (the coordinator regenerates it on the wave branch after the merges), the list files under `docs/plan/slice-06/` (the coordinator), any file under `crates/swell/src/builtins/`. The matches with no wildcard this story adds arms to are all in files above: `Stmt` (`ast.rs` line 590, `resolve.rs` line 351, compiler `statement.rs` line 52), `ForInTarget` (`early.rs` line 316, `resolve.rs` line 422, compiler `statement.rs` line 201, `parser/statement.rs`) if the statement reuses it, `Instruction` (`bytecode.rs` line 339, `dispatch.rs` line 237), `HandlerKind` (`disasm.rs` line 142) for a new kind. Shared: none with a story that may run beside it (`story:symbol-values-in-core-builtins` writes builtins and tests only); every file of `crates/syntax` and `crates/compiler` it writes is also written by `story:block-function-declarations`, which lands before it.

## Acceptance

The tests and fixtures named in Change pass in both collection modes, run by the binaries of this change and not as a whole step: `cargo nextest run --locked -p swell --test for_of --test language --test limits_scripts --test budgets --test eval_stack` and the same with `--features collect-every-safe-point`, `cargo nextest run --locked -p swell --lib` (the unit tests of `ops/iteration.rs`), `cargo nextest run --locked -p swell-syntax --test sources` and `cargo nextest run --locked -p swell-compiler --test compile` exit 0, and `slot task clippy` exits 0; `slot task test262` is run once after the change (the slow lane, in the background through the pool, never a test step) and ends with `test262: <n> line(s) differ from conformance/test262/expected.txt` and no other complaint, and on its run lines, with the list as committed on the wave branch when the wave closes, copied by `git show "$WAVE:docs/plan/slice-06/story-for-of.cases.txt" > .out/story-for-of.cases.txt` (`WAVE` the wave branch the unit merges into; the coordinator takes a case off the list by a commit there whose body names the part it ends on), (a) `awk -F'\t' 'NR == FNR { want[$1] = 1; next } ($3 in want) && $5 != "Passed" { print $3, $4, $6; bad = 1 } END { exit bad }' .out/story-for-of.cases.txt .out/test262/runs.tsv` exits 0 (every case of that copy passes in every mode; on `6a43e18`, with the list as committed today, it exits 1 and prints 479 runs), (b) `awk -F'\t' '$3 ~ /^test\/language\/statements\/for-of\// && $6 ~ /for-of statement is not supported/' .out/test262/runs.tsv | wc -l` prints `0` (on `6a43e18` `445`), (c) the line `test262 language/statements/for-of` of `.out/test262/lines.txt` reads at least B + L - R passed in the seventh field, where B is the passes of that line on the base of the unit (3 on `6a43e18`), L the cases of the copy under that directory (245 on the list of today) and R the cases under it among the runs (a) prints; it is a bound and not the count because a case off the list may pass as well, and (d) with the run of the base commit of the unit made before the change (`slot task test262 TEST262_OUT=.out/test262-base`), `awk -F'\t' 'NR == FNR { if ($5 == "Passed") p[$3 "\t" $4] = 1; next } (($3 "\t" $4) in p) && $5 != "Passed" { print $3, $4, $6 }' .out/test262-base/runs.tsv .out/test262/runs.tsv` prints nothing (no run that passed on the base fails).

## Out of scope

Array patterns in the head (`story:array-destructuring-and-spread`); `for await` (slice 10); generators as iterables (slice 7); the Annex B initializer of a `for`-`in` head (slice 18); the moves of cases and the slice lines of the design (`story:slice-06-conformance-close`).
