---
format: aep.planning-md/3
id: story:super-property-and-super-syntax
kind: story
status: draft
title: 'super outside classes: SuperProperty and SuperCall grammar, their early errors, super.x in object-literal methods'
summary: 66 cases (128 runs); super in an arrow and in eval code are held for the derived story.
relations:
- decomposes: epic:slice-08-classes
scope:
- confidence: cited
  path: crates/compiler/src/bytecode.rs
- confidence: cited
  path: crates/compiler/src/disasm.rs
- confidence: cited
  path: crates/compiler/src/expression.rs
- confidence: cited
  path: crates/compiler/src/resolve.rs
- confidence: inferred
  path: crates/compiler/tests/sources/super-property.disasm
- confidence: inferred
  path: crates/compiler/tests/sources/super-property.js
- confidence: cited
  path: crates/swell/src/interp/dispatch.rs
- confidence: cited
  path: crates/swell/src/interp/mod.rs
- confidence: inferred
  path: crates/swell/src/interp/super_ref.rs
- confidence: cited
  path: crates/swell/tests/main.rs
- confidence: inferred
  path: crates/swell/tests/super_property.rs
- confidence: cited
  path: crates/syntax/src/ast.rs
- confidence: cited
  path: crates/syntax/src/early.rs
- confidence: cited
  path: crates/syntax/src/parser/expression.rs
- confidence: cited
  path: crates/syntax/src/parser/function.rs
- confidence: cited
  path: crates/syntax/src/parser/mod.rs
- confidence: inferred
  path: crates/syntax/tests/sources/early-errors/super-property-in-function.js
- confidence: inferred
  path: crates/syntax/tests/sources/refused/super-in-arrow.js
- confidence: inferred
  path: crates/syntax/tests/sources/refused/super-in-eval.js
- confidence: cited
  path: crates/syntax/tests/sources/refused/super.js
- confidence: inferred
  path: crates/syntax/tests/sources/valid/super-property.js
revision: 3
---
## Context

Slice 8 of 19, `classes` (`epic:slice-08-classes`; `docs/design/engine.md`, section 12, item 8). This story is the `super` keyword outside classes: the grammar of SuperProperty (`super.name`, `super[expression]`) and SuperCall (`super(arguments)`, `#sec-super-keyword`), the early errors that forbid them in functions, generators, async functions, object-literal methods (SuperCall) and global code (`#sec-function-definitions-static-semantics-early-errors`, `#sec-method-definitions-static-semantics-early-errors` HasDirectSuper, `#sec-scripts-static-semantics-early-errors`), and the evaluation of a SuperProperty in an object-literal method, getter, setter, generator method and async method (`#sec-super-keyword-runtime-semantics-evaluation`, MakeSuperPropertyReference, GetSuperBase `#sec-getsuperbase`, the reference's [[ThisValue]]; `delete super[...]` throws a ReferenceError, `#sec-delete-operator-runtime-semantics-evaluation`). It needs no other story of slice 8: order 1, built beside `story:new-target-meta-property` and `story:class-syntax-and-early-errors`.

`super` inside an arrow function at run time (the arrow reaches [[HomeObject]] through the function its `this` comes from, a box `story:new-target-meta-property` brings) and `super` in direct eval code (PerformEval's `inMethod` and `inDerivedConstructor`, also that story's) are `story:derived-classes-and-super-call`'s: this story holds them with the refusals `super in an arrow function is not supported before slice 8` and `super in eval code is not supported before slice 8` (held to the end of the parse, `Parser::held`), so that no answer is wrong in between. Their four cases (`expressions/super/prop-{dot,expr}-{obj,cls}-val-from-arrow.js`) and the cases of `super` in eval are on that story's list. The early errors of `super` in an arrow at the top (`global-code/super-prop-arrow.js`, `async-arrow-function/early-errors-arrow-*-contains-super-*`) are this story's: an arrow takes the answer of the code around it.

On the base (`62c7cf59`; the unit reads every place named here again on its own base): the parser refuses `super` at once (`crates/syntax/src/parser/expression.rs`, `primary_kind`, the arm `Keyword::Super` near line 1024); `crates/syntax/src/early.rs` line 1241 says in a comment that `super` is refused; an object-literal method already has its [[HomeObject]] (`EcmascriptFunction::home_object`, `crates/swell/src/object/ecmascript_function.rs` line 62, set by `make_method`, line 626). The refusal `super is not supported before slice 8` matches section 12, item 8 (the entry "Each slice" of `docs/plan/waves.md`, "What later slices take over", checked); the fixture `crates/syntax/tests/sources/refused/super.js` (`function f() { return super.x; }`) holds it and becomes an early-error fixture.

The list is `docs/plan/slice-08/story-super.cases.txt`: the 66 cases of the key `super` of `docs/plan/slice-08/scan.tsv` (a SuperProperty or SuperCall outside a class and outside eval; 34 of them negative parse cases). By directory: `language/expressions/super` 18, `language/expressions/object` 16 (12 of them `method-definition`), the early errors of functions, async functions, async generators and async arrows 24, `language/global-code` 4, `language/computed-property-names/object` 3, `language/expressions/delete` 1. Their 128 runs fail on the plan's base run, all on `super is not supported before slice 8`. The base run of this plan is the full run of 2026-11-26 on `149e12f1`, whose total and slice lines equal `conformance/test262/expected.txt` on `62c7cf59` (slice 8: `test262-slice 08 9302 18455 0 0 9302 5 9297 0`).

Design it builds on: section 5 of `docs/design/engine.md` (references and the run loop). Design change 1 of this decomposition (a new section "Classes", written by a design author before the units of slice 8 start) names a SuperProperty's evaluation as instructions that read [[HomeObject]] from the running function; the unit reads the text that landed and follows it where it differs from this story.

## What later stories reuse

`Expr::SuperMember` and `Expr::SuperCall` and their parse; the parser's flag that admits a SuperProperty (`story:derived-classes-and-super-call` sets it in every class method, accessor and constructor, `story:class-element-syntax-and-early-errors` in field initializers and static blocks, and adds the flag of SuperCall for derived constructors); the instructions of a super reference (the class stories compile `super.x` in class methods with them; the receiver is the frame's `this`); the helper `Parser::hold` (below), which `story:class-syntax-and-early-errors` adds too, with the same signature.

## Domain relations

- A SuperProperty → the [[HomeObject]] of the method whose code holds it (one), and its base → that object's [[Prototype]] when the reference is made — inferable (inferred from ECMA-262 `#sec-getsuperbase` and `#sec-makesuperpropertyreference`; no ess/1 document declares it: `spec/domains/host.yaml` holds no engine value, and ESS does not restate ECMA-262, `AGENTS.md`, "Rules").

## Change

Failing tests first: the tests and fixtures below are written before the code and fail on the base.

- `crates/syntax`: `super` followed by `.` or `[` is `Expr::SuperMember` (a name or a computed key; `crates/syntax/src/ast.rs`, a new variant), followed by `(` is `Expr::SuperCall` (arguments as a call's; a new variant), any other `super` the grammar error `unexpected token super`. A SuperProperty stands where the nearest function that is not an arrow is a method of an object literal (its parameters and its body), where an arrow function takes the answer of the code around it: a flag of `Parser` beside `yields` and `awaits`, saved and put back at each function boundary (`crates/syntax/src/parser/function.rs`, `save_context`, `open_context`, `function_close`; the object-literal method head of `parser/expression.rs`, `method_head`, sets it). Anywhere else a SuperProperty is an early error, and a SuperCall is an early error everywhere this story parses (only a derived class's constructor admits it, `story:derived-classes-and-super-call`). Each early error with the message of the rule (`crates/syntax/src/early.rs`; the comment of line 1241 rewritten). `super` inside an arrow function where the flag admits it, and `super` in eval code, hold the refusals named in Context: the helper `fn hold(&mut self, construct: &str, slice: u32, span: Span)` beside `refuse_at` in `parser/mod.rs` keeps the first held refusal (`Parser::held`) and lets the parse go on; the documentation of `held` (`parser/mod.rs` lines 93 to 98, "No construct holds one") names the two holds and the story that lifts them. `super` is not an assignment target as a whole and `super.x` is (`++super.x`, `super[k] += 1`).
- `swell-compiler` (`crates/compiler/src/{bytecode,disasm,expression,resolve}.rs`): a SuperProperty compiles to instructions of their own: the super base (the [[Prototype]] of the running function's [[HomeObject]], read before the key is converted, `prop-expr-getsuperbase-before-topropertykey-*`), a get with the frame's `this` as receiver, a set with it (a failed set throws in strict code, `prop-*-obj-ref-strict.js`), a call with it as `this`, the forms of compound and logical assignment, `++` and `--`, and `delete super[...]` (evaluates the key expression, converts nothing, throws a ReferenceError). Each instruction with its documentation in `Instruction`, its line in `disasm.rs` and a compiler test (`crates/compiler/tests/sources/super-property.js` and `.disasm`, new). `Expr::SuperCall` gets the arm of every exhaustive match and compiles to nothing reachable: the parser never gives one to the compiler before `story:derived-classes-and-super-call`, and the arm answers `CompileError::Tree` (the one variant of this story allowed to, because no source reaches it: a test states that every SuperCall a source can hold is an early error).
- `crates/swell`: the arms of the new instructions in `interp/dispatch.rs` call functions of a new file `interp/super_ref.rs` (the run loop holds no instruction's locals); the super base is null or undefined → TypeError on access (`prop-*-obj-null-proto.js`); a getter or setter reached through `super` gets `this`; `__proto__` is read through [[GetPrototypeOf]], not as a property (`prop-poisoned-underscore-proto.js`).
- Tests, in `crates/swell/tests/super_property.rs` (new; `tests/main.rs` is generated by `sh taskfiles/test-main.sh write`), each in both collection modes: `super.x`, `super[k]`, `super.m()` (its `this`), `super.x = v` (the receiver gets the property, the prototype not), in a method, a getter, a setter, a generator method resumed after a collection, an async method across an await; the base read once per evaluation after `Object.setPrototypeOf` on the home object; a null prototype; `delete super[k]` (ReferenceError, the key's `toString` not called); a method copied to another object keeps its home object; a loop of 10,000 `super.m()` calls within the step budget with `steps_per_entry` small (`Termination::StepBudget`) and a deep recursion through `super.m()` stopping at `frame_depth` with a RangeError; the record size test stays at its value. `crates/syntax/tests/sources/`: `refused/super.js` moves to `early-errors/super-property-in-function.js` with its message; new `early-errors/super-*.js` (in a function, a generator, an async function, parameters, global code, an arrow at the top, `super()` in an object method), `valid/super-property.js`, `refused/super-in-arrow.js` and `refused/super-in-eval.js` (with the slice-8 message each). `NESTING_KINDS` unchanged.
- Time ("Tests stay fast", the `AGENTS.md` of the folder that holds the repository, and `AGENTS.md`, "Commands and the gate"): a test is terminated at 60 s and counts as failed, and the gate fails when the wall time of its test steps passes 120 s. A test stays under 1.5 CPU seconds alone in the default mode, under 2 with `collect-every-safe-point` and under 2.5 at `SWELL_FULL_COUNTS=1`, and a language script under 0.5 CPU seconds per run, measured as `AGENTS.md` says (one job of the pool, the test binary run with `--exact <name> --test-threads 1`, user plus system). A product several tests share is built once inside one test function or is a committed file, never per test; no test makes a network request or a model call; the unit runs the test binaries of its own change, never the whole gate (`slot task check`) and never a whole-workspace test run. Builds and tests go through the pool: `cargo` is the shim of the pool, anything else that compiles or tests is prefixed `slot`; a pool job ends after 10 minutes unless it names its time (`slot --timeout 30m` or `SLOT_TIMEOUT=30m`, 30 minutes at most); a script that measures or waits runs without the prefix. No Python in any tool, test or command (a scratch helper that is never committed excepted); intermediate files in the scratch directory or `.out/`.
- Size: about 650 lines (parser and early errors 160, compiler 140, engine 120, tests and fixtures 230): one session.

## Requirements

- None of `conformance/host/requirements.txt` moves: `grep -c "pending slice 8$" conformance/host/requirements.txt` prints `1` before and after.
- Rules every story carries. No unsafe code and no third-party crate: `git diff --stat "$BASE" -- Cargo.lock "*/Cargo.toml"` prints nothing and `slot task forbid-unsafe-check` exits 0. Decision 13: no panic on any input (no `unwrap`, no `expect`, no unchecked indexing; arithmetic that can exceed its type checked or explicitly wrapping). The run loop: nothing is added to the frame of `Agent::execute`; each new instruction does its work in a function of its own; `STACK_BYTES_PER_REENTRY` (`crates/swell/src/host/limits.rs`, 14 KiB) is unchanged, and if a stack test shows a change it is measured as `docs/plan/slice-02-run-loop-frame.md` does, never raised unmeasured.

Writes: `crates/syntax/src/{ast,early}.rs`, `crates/syntax/src/parser/{mod,expression,function}.rs` (the flag and `hold`; the arm `Keyword::Super` of `primary_kind` and the super suffixes; `method_head`; `save_context`, `open_context`, `function_close`), `crates/syntax/tests/sources/{early-errors,valid,refused}/super*.js`; `crates/compiler/src/{bytecode,disasm,expression,resolve}.rs`, `crates/compiler/tests/sources/super-property.{js,disasm}` (new), `crates/compiler/tests/main.rs` (generated, if the compiler's tests are listed there); `crates/swell/src/interp/{dispatch,super_ref}.rs` (`super_ref.rs` new; `interp/mod.rs` its `mod` line), `crates/swell/tests/super_property.rs` (new), `crates/swell/tests/main.rs` (generated). A unit that meets a file outside this line that its change cannot compile or pass without makes the smallest change there and reports it. Not written: `crates/syntax/src/{lexer,scope,lib}.rs`, `crates/syntax/src/parser/{statement,class}.rs`, `crates/swell/src/object/`, `crates/swell/src/dynamic_code.rs`, `crates/test262`, `conformance/`, `website/`, `docs/`.

Shared with the stories of order 1, built at the same time, each in hunks of its own: `crates/syntax/src/parser/expression.rs` (`story:new-target-meta-property`, `new_rest`; `story:class-syntax-and-early-errors`, the arm `Keyword::Class` of `primary_kind`; this story, the arm `Keyword::Super` and `method_head`); `crates/syntax/src/parser/function.rs` and `Parser` in `parser/mod.rs` (`story:new-target-meta-property` adds its flag beside this one in the same three functions: the second to merge keeps both lines); `Parser::hold` (`story:class-syntax-and-early-errors` adds the same helper with the same signature: the second to merge keeps one) and the documentation of `held` (both name their holds: the second keeps both sentences); `crates/syntax/src/ast.rs` and every exhaustive match over `Expr` (each story its own variants and arms); `crates/syntax/src/early.rs` (each its own rules); `crates/compiler/src/{bytecode,disasm,expression,resolve}.rs` and `crates/swell/src/interp/dispatch.rs` (`story:new-target-meta-property`, its instruction; each its own variants and arms); `crates/swell/tests/main.rs` and `crates/compiler/tests/main.rs` (generated: `sh taskfiles/test-main.sh write` after the merge).

## Acceptance

The tests named in Change pass, run by the binaries of this change and not as a whole step: `cargo nextest run --locked -p swell-syntax --test main` and the same with `SWELL_FULL_COUNTS=1`; `cargo nextest run --locked -p swell-compiler --test main`; `cargo nextest run --locked -p swell --test main -E 'test(/^super_property::/) | test(/^objects::/) | test(/^generators::/) | test(/^async_functions::/) | test(/^language::/) | test(/^limits_scripts::/) | test(/^budgets::/) | test(/^eval_stack::/) | test(/^collect::/) | test(/^termination::/)'` and the same with `--features collect-every-safe-point` (a module named here that does not exist on the unit's base is left out and named in the report); `cargo nextest run --locked -p swell --lib` and the same with `--features collect-every-safe-point`; `cargo nextest run --locked -p swell-test262 --test main`; each exits 0. `slot task clippy` exits 0; `sh taskfiles/test-main.sh check` exits 0; `git diff --stat "$BASE" -- Cargo.lock "*/Cargo.toml"` prints nothing; `slot task forbid-unsafe-check` exits 0; `grep -c "pending slice 8$" conformance/host/requirements.txt` prints `1`. The text: `grep -rn '"super", 8\|super is not supported' crates conformance docs/design website/docs` prints nothing (on the base it prints the arm of `primary_kind` and the fixture's header). Time: every test this story adds stays under 1.5 CPU seconds alone in the default mode and under 2 with `collect-every-safe-point`, as the unit measured them; the report lists each measured time and the sum, and a time over its bound fails this acceptance.

test262. With the list as committed on the wave branch when the wave closes, `git show "$WAVE:docs/plan/slice-08/story-super.cases.txt" > .out/story-super.cases.txt` (`WAVE` the wave branch the unit merges into), and the guards `docs/plan/slice-08/guard-syntax.cases.txt` (2,469 passing negative parse cases of `test/language/`) and `docs/plan/slice-08/guard-functions.cases.txt` (2,546 passing cases of functions, arrows, methods, calls, `new`, eval code and `Function`), `cat .out/story-super.cases.txt docs/plan/slice-08/guard-syntax.cases.txt docs/plan/slice-08/guard-functions.cases.txt | LC_ALL=C sort -u > .out/part.cases.txt` and `sh docs/plan/slice-08/part.sh "$CHECKOUT" .out/test262-part .out/part.cases.txt` (`CHECKOUT` `.cache/test262/<commit of conformance/test262/PIN>`; 4,822 cases, 8,493 runs over the plan's base run), then, in the background, `SLOT_TIMEOUT=30m cargo run --locked --release -q -p swell-test262 -- count --test262 .out/test262-part --design docs/design/engine.md --features conformance/test262/features.txt --runs .out/test262-part/runs.tsv > .out/test262-part/lines.txt`, and the base run the coordinator made on the join the unit starts from copied to `.out/test262-base/runs.tsv`: (a) `sh docs/plan/slice-08/check.sh .out/story-super.cases.txt .out/test262-base/runs.tsv .out/test262-part/runs.tsv` exits 0 (over the plan's base run, restricted to the part, it exits 1 and prints 128 lines `failing` and `passed 4756 bound 4756 B 4756 F 66 R 66`); (b) `awk -F'\t' '$6 ~ /super is not supported/' .out/test262-part/runs.tsv | wc -l` prints `0` (128 over the plan's base run). On the join, the coordinator's full run, made in parts, passes (a) over the whole run.

## Out of scope

`super` in classes, in arrow functions at run time and in eval code, and SuperCall (`story:derived-classes-and-super-call`); `super` in field initializers and static blocks (`story:class-element-syntax-and-early-errors` for the grammar, the class stories for the run); `Reflect` and `Proxy` (slice 9); the counts of `conformance/test262/expected.txt` (`story:slice-08-conformance-close`).
