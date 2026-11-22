---
format: aep.planning-md/3
id: story:block-function-declarations
kind: story
status: implemented
title: 'Function declarations in blocks and switch: BlockDeclarationInstantiation and the block''s function bindings'
summary: 49 failing cases of slice 6 on its list; BlockDeclarationInstantiation as decision 3 gives it; needs no builtin of slice 3 and starts before slice 3's close.
relations:
- decomposes: epic:slice-06-bindings-and-iteration
- depends_on: story:computed-names-and-object-spread
scope:
- confidence: cited
  path: crates/compiler/src/bytecode.rs
- confidence: cited
  path: crates/compiler/src/disasm.rs
- confidence: cited
  path: crates/compiler/src/lib.rs
- confidence: cited
  path: crates/compiler/src/resolve.rs
- confidence: cited
  path: crates/compiler/src/statement.rs
- confidence: cited
  path: crates/swell/src/interp/dispatch.rs
- confidence: cited
  path: crates/swell/src/interp/lexical.rs
- confidence: cited
  path: crates/swell/tests/block_functions.rs
- confidence: cited
  path: crates/swell/tests/language/block-function-hoisting.js
- confidence: cited
  path: crates/swell/tests/language/block-function-scope.js
- confidence: cited
  path: crates/swell/tests/language/block-function-switch.js
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
  path: crates/syntax/tests/sources/refused/function-declaration-in-block.js
- confidence: cited
  path: crates/syntax/tests/sources/refused/function-declaration-in-switch.js
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T00:38:40Z", actor: "human:operator", revision: 5, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-22T00:38:40Z", actor: "human:operator", revision: 6, executor: "agent:coordinator"}
- {from: "active", to: "implemented", at: "2026-11-22T22:03:56Z", actor: "human:operator", revision: 7, decided_on: {"recorded":{"test_result":1}}, executor: "agent:coordinator"}
---
## Context

Slice 6 is delivered before slices 4 and 5 because it closes most of the first gap of the consumer, the testharness.js tests of the HTML parser of surf (`docs/plan/waves.md`, "The order after slice 3"): 191 refused constructs of slice 6 in `resources/testharness.js` and the scripts of `html/syntax/parsing/html5lib_url.html`, measured with the parser of `main` (`6a43e18`). This story lifts none of the 191 (both scripts declare their functions at the top of a function body). It follows `story:computed-names-and-object-spread` because it needs no builtin of slice 3 and writes the same files of the parser and the compiler: the first failing run of 37 cases of slice 6 ends on `function declaration inside a block is not supported` (`awk -F'\t' '$2 == "06" && $5 != "Passed" && !($3 in s) { s[$3] = 1; if ($6 ~ /function declaration inside a block is not supported/) c++ } END { print c }' .out/test262/runs.tsv` prints `37`; 254 runs over all slices end on it), and 12 more cases of `language/eval-code` end on a counted refusal of the same construct in their eval code.

The base refuses a FunctionDeclaration where `Place::Block` holds (`crates/syntax/src/parser/statement.rs`, line 155): in a Block and in the clauses of a `switch` (line 780). A function declaration in the clause of an `if` in non-strict code (B.3.3, line 157) and the web compatibility semantics of B.3.2 (a `var` binding beside the block one, duplicate declarations in a block) are Annex B, slice 18 (the out of scope of the epic), and stay as they are. Fixtures that rely on the refusal: `crates/syntax/tests/sources/refused/function-declaration-in-block.js` and `function-declaration-in-switch.js`, removed; `function-declaration-in-if-clause.js` stays.

The list of this story is `docs/plan/slice-06/story-block-functions.cases.txt`: 49 failing cases (81 runs) of slice 6 and of the six directories `story:slice-03-conformance-close` moves to slice 6 (`test/built-ins/Array/from/`, `Array/prototype/keys/`, `values/`, `entries/`, `Object/fromEntries/`, `Object/groupBy/`), on no list of the first five stories of the slice, whose text and harness files use a function declaration in a block or a `switch` clause, no other part of slice 6 than those of the stories this one depends on, and no part of a later slice. Found by the token scan `docs/plan/slice-06/scan.awk` (run as `story:let-const-and-block-scopes` gives it, but with a copy of `meta.tsv` whose includes leave out `propertyHelper.js`, whose three later tokens are a branch behind `typeof`, a case of a bigint value and the name `checkSet`; the comment at the head of `lists-b.awk` gives the command; over the first failing run of each failing case of slice 6 and of those six directories on `main`, `6a43e18`) and the assignment `docs/plan/slice-06/lists-b.awk` (`gawk -v root=<checkout> -v meta=.cache/inventory/meta.tsv -f docs/plan/slice-06/lists-b.awk <scan output>`, the cases of the five lists of the first part taken out), which puts a case on the list of the first story of the slice that holds every part the case needs and whose own construct the case uses. By directory: `language/eval-code` 12, `language/block-scope` 12, `language/statements/break` 7, `language/statements/switch` 5, `language/statements/continue`, `language/global-code` and `language/function-code` 3 each, the rest 4. A case of the list that ends on a part outside this story is reported with that part, and the coordinator takes it off the list by a commit of the list file on the wave branch whose body names that part; the acceptance reads the list as committed there when the wave closes.

Design it builds on: `docs/design/engine.md`, decision 3, "Function declarations in a block" (lines 545 to 609, merged on `wave/s3-1`): a function declared in a Block or a CaseBlock is a lexical binding of that scope under item 1 of "Lexical bindings" (line 340), a local slot or a slot of the scope's record, made at the entry of the scope with the closure instruction in source order and initialized before its first statement (in a `switch` after the discriminant), so each entry makes a new object (item 1); no access to it is checked (item 2); in eval code a function of a Block belongs to that Block's scope, never to the record of PerformEval, and a top-level function of eval code stays a `var` declaration (item 3); slice 18 adds the `var` binding of B.3.2 and the copy at the position of the declaration, which emits nothing before then (item 4); each object is the closure instruction's allocation and step, a record charged by item 6 of "Lexical bindings" (item 5). The omission of B.3.2 is recorded in `conformance/test262/sections.txt` as the design gives it, by `story:slice-06-conformance-close`, the one writer of that file in slice 6 (`story:slice-03-conformance-close` writes it as well and may run beside this story).

## Domain relations

- None new. The function object a block makes references its [[Environment]], the record of the block when there is one (inferred from `crates/swell/src/object/ecmascript_function.rs`, `EcmascriptFunction`; no ess/1 document declares this relation).

## Change

Failing tests first: the tests and fixtures below are written before the code and fail on the base (the parser refuses the declaration).

- `swell-syntax`. `parser/statement.rs`: a FunctionDeclaration as a StatementListItem of a Block and of a `case` or `default` clause (`#sec-block`, `#sec-switch-statement`), in strict and non-strict code; the `if` clause keeps its refusal. `scope.rs`: the function declarations of a block or a `case` block are LexicallyScopedDeclarations of that scope and no VarScopedDeclarations of the function or the script (`#sec-block-static-semantics-lexicallyscopeddeclarations`, `#sec-block-static-semantics-varscopeddeclarations`). `early.rs`: the rules of `#sec-block-static-semantics-early-errors` and `#sec-switch-statement-static-semantics-early-errors` with function names among the LexicallyDeclaredNames (a function and a `let`, `const` or second function of one name in one block, and a name that is also var-declared in the block), in both modes, B.3.2.4 waiting for slice 18. `ast.rs`: the lexical functions of a block in `Scope` if the scope analysis keeps them there. `crates/syntax/tests/sources.rs`: the kind `block_function` (`{ function f() { { function f() { ... } } } }`) added to `NESTING_KINDS` of `every_nesting_kind_fits_the_stack_bound` in the form the file has on the base of the unit (on `wave/s3-1` an arm of `nested`, line 417, and an entry of the array at line 462); the unit times `cargo nextest run --locked -p swell-syntax --test sources` before and after its change (29.6 s on `wave/s3-1`; `story:gate-test-steps-within-budget` changes how the binary runs) and reports both times. Fixtures `crates/syntax/tests/sources/valid/block-function-*.js` and `early-errors/block-function-*.js` (new); the two refused fixtures named in Context removed.
- `swell-compiler`. `resolve.rs`: the function binding of a block and of a `case` block in its scope, placed as the design gives; `statement.rs`: BlockDeclarationInstantiation at the entry of the scope, each function object made and its binding initialized before the first statement, in source order of the declarations; `lib.rs`: the eval code of a block with a function declaration compiles as any other block. No new instruction is expected (the closure instruction and the initialization of `story:let-const-and-block-scopes` do the work); one that is needed gets its arm in `Instruction::target` (`bytecode.rs` line 339), and in `Instruction::with_target` (line 444, whose last arm is `other => other`) if it jumps, with a fixture whose `.disasm` shows the patched target, and `interp/dispatch.rs` line 237, and a function of its own in `crates/swell/src/interp/lexical.rs`, and the unit names it in its report. Fixtures `crates/compiler/tests/sources/block-function-*.js` and `.disasm` (new: a function in a block, in a loop body, captured by a closure, in a `case` clause).
- Tests, each in both collection modes (`test-collect`): `crates/swell/tests/block_functions.rs` (new): `a_block_function_is_callable_before_its_declaration_in_the_block`, `a_block_function_is_not_visible_after_the_block` (a `ReferenceError` in strict and in non-strict code; the non-strict half changes its answer in slice 18, when B.3.2 gives the function a `var` binding as well, item 4 of the design), `each_entry_of_a_block_makes_a_new_function_object` (two iterations of a loop body give two objects that are not `===`), `a_case_clause_function_is_shared_by_every_clause`, `a_block_function_in_eval_code_belongs_to_its_block` (a function of a Block of direct and indirect eval code is not visible after that Block, in the eval code or in the code that called it, while a top-level function of the eval code is a `var` of it, item 3). `crates/swell/tests/language/` (new files): `block-function-hoisting.js`, `block-function-scope.js`, `block-function-switch.js`. No test uses a part of a later story of the slice.
- Time of the tests ("Tests stay fast", the `AGENTS.md` of the folder that holds the repository): the unit times each test before it adds it, names in its report each one over 5 s with its time and the reason, and adds none over 60 s; a product several tests share is built once inside one test function or is a committed file, never in a `static` (the test steps run a process per test); no test makes a network request; the unit runs the test binaries of its change and never the whole gate. None is expected over 1 s.
- Run loop: no instruction is expected; one that is needed does its work in a function of its own in `interp/lexical.rs`, called from one arm of `interp/dispatch.rs`; nothing is added to the frame of `Agent::execute`; `STACK_BYTES_PER_REENTRY` and the stack tests (`limits_scripts.rs`, `budgets.rs`, `eval_stack.rs`) do not change and pass on seven eighths of `min_stack_bytes()`.

Writes: `crates/syntax/src/{ast,scope,early}.rs`, `crates/syntax/src/parser/statement.rs`, `crates/syntax/tests/sources.rs` (one nesting kind), `crates/syntax/tests/sources/valid/block-function-*.js` and `early-errors/block-function-*.js` (new), `crates/syntax/tests/sources/refused/{function-declaration-in-block,function-declaration-in-switch}.js` (removed), `crates/compiler/src/{resolve,statement,lib}.rs`, `crates/compiler/tests/sources/block-function-*.js` and `.disasm` (new), `crates/swell/tests/block_functions.rs` (new), `crates/swell/tests/language/{block-function-hoisting,block-function-scope,block-function-switch}.js` (new); only if an instruction is needed, `crates/compiler/src/{bytecode,disasm}.rs`, `crates/swell/src/interp/{lexical,dispatch}.rs`. Not written: `conformance/test262/expected.txt` (the coordinator regenerates it on the wave branch after the merges), the list files under `docs/plan/slice-06/` (the coordinator), `crates/swell/src/{script,dynamic_code}.rs` (GlobalDeclarationInstantiation and EvalDeclarationInstantiation read the declarations of the top level only), any file under `crates/swell/src/builtins/`. The matches with no wildcard: no variant is added to `Stmt` (the declaration is `Stmt::FunctionDeclaration`, `ast.rs` line 345); the walks over `Stmt` (`ast.rs` line 590, `resolve.rs` line 351, compiler `statement.rs` line 52) change in their arm for `Stmt::FunctionDeclaration` and `Stmt::Block`. Shared: it depends on `story:computed-names-and-object-spread` for `ast.rs`, `early.rs`, `parser/statement.rs`, `resolve.rs` and compiler `statement.rs`, which both write. `story:function-apply-and-source-text` (slice 3) writes `crates/compiler/src/lib.rs`, `crates/syntax/src/ast.rs` and `crates/syntax/tests/sources.rs` (and `bytecode.rs` of this story's optional instruction); this story comes after it by the chain `story:computed-names-and-object-spread`, `story:object-destructuring`, `story:default-and-rest-parameters`, `story:arrow-functions`, `story:let-const-and-block-scopes` and `story:template-literals`, which both depend on it, and that order is the reason none of them meets it. `conformance/test262/sections.txt` is not written (`story:slice-06-conformance-close`).

## Acceptance

The tests and fixtures named in Change pass in both collection modes, run by the binaries of this change and not as a whole step: `cargo nextest run --locked -p swell --test block_functions --test language --test eval --test limits_scripts` and the same with `--features collect-every-safe-point`, `cargo nextest run --locked -p swell-syntax --test sources` and `cargo nextest run --locked -p swell-compiler --test compile` exit 0, and `slot task clippy` exits 0; `slot task test262` is run once after the change (the slow lane, in the background through the pool, never a test step) and ends with `test262: <n> line(s) differ from conformance/test262/expected.txt` and no other complaint, and on its run lines, with the list as committed on the wave branch when the wave closes, copied by `git show "$WAVE:docs/plan/slice-06/story-block-functions.cases.txt" > .out/story-block-functions.cases.txt` (`WAVE` the wave branch the unit merges into; the coordinator takes a case off the list by a commit there whose body names the part it ends on), (a) `awk -F'\t' 'NR == FNR { want[$1] = 1; next } ($3 in want) && $5 != "Passed" { print $3, $4, $6; bad = 1 } END { exit bad }' .out/story-block-functions.cases.txt .out/test262/runs.tsv` exits 0 (every case of that copy passes in every mode; on `6a43e18`, with the list as committed today, it exits 1 and prints 81 runs), (b) `awk -F'\t' '$3 ~ /^test\/language\/(block-scope|statements\/switch)\// && $6 ~ /function declaration inside a block is not supported/' .out/test262/runs.tsv | wc -l` prints `0` (on `6a43e18` `94`), (c) the lines `test262 language/block-scope` and `test262 language/statements/switch` of `.out/test262/lines.txt` each read at least B + L - R passed in the seventh field, where B is the passes of that line on the base of the unit (16 and 35 on `6a43e18`), L the cases of the copy under that directory that have a failed run in `.out/test262-base/runs.tsv` (all of them fail on `6a43e18`: 12 and 5 on the list of today) and R the cases under it among the runs (a) prints; it is a bound and not the count because a case off the list may pass as well, and (d) with the run of the base commit of the unit made before the change (`slot task test262 TEST262_OUT=.out/test262-base`), `awk -F'\t' 'NR == FNR { if ($5 == "Passed") p[$3 "\t" $4] = 1; next } (($3 "\t" $4) in p) && $5 != "Passed" { print $3, $4, $6 }' .out/test262-base/runs.tsv .out/test262/runs.tsv` prints nothing (no run that passed on the base fails).

## Out of scope

The Annex B semantics of block functions (B.3.2 and B.3.3, slice 18); generator and async declarations in blocks (slices 7 and 10); class declarations (slice 8); the moves of cases and the slice lines of the design (`story:slice-06-conformance-close`).
