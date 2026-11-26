---
format: aep.planning-md/3
id: story:public-class-fields
kind: story
status: draft
title: Public instance and static fields, their initializers run as frames of the run loop
summary: 515 cases (1,029 runs); lifts class field.
relations:
- decomposes: epic:slice-08-classes
- depends_on: story:class-definition-evaluation
- depends_on: story:class-element-syntax-and-early-errors
scope:
- confidence: cited
  path: crates/compiler/src/bytecode.rs
- confidence: inferred
  path: crates/compiler/src/class.rs
- confidence: cited
  path: crates/compiler/src/disasm.rs
- confidence: inferred
  path: crates/compiler/tests/sources/class-fields.disasm
- confidence: inferred
  path: crates/compiler/tests/sources/class-fields.js
- confidence: inferred
  path: crates/swell/src/interp/class.rs
- confidence: cited
  path: crates/swell/src/interp/dispatch.rs
- confidence: cited
  path: crates/swell/src/object/ecmascript_function.rs
- confidence: inferred
  path: crates/swell/tests/class_fields.rs
- confidence: cited
  path: crates/swell/tests/main.rs
- confidence: inferred
  path: crates/syntax/src/parser/class.rs
- confidence: cited
  path: crates/syntax/src/parser/mod.rs
- confidence: inferred
  path: crates/syntax/tests/sources/refused/class-field.js
- confidence: inferred
  path: crates/syntax/tests/sources/valid/class-field.js
revision: 3
---
## Context

Slice 8 of 19, `classes` (`epic:slice-08-classes`; `docs/design/engine.md`, section 12, item 8). This story runs public fields, instance and static: ClassFieldDefinitionEvaluation (`#sec-runtime-semantics-classfielddefinitionevaluation`: a computed key is evaluated once, at the class's definition, in order with the other elements, and converted with ToPropertyKey), the initializer (a function with [[HomeObject]] the prototype, or the constructor for a static field, run with the instance or the constructor as `this`; `new.target` undefined; SuperProperty through the home object), DefineField (`#sec-definefield`: CreateDataPropertyOrThrow, so a non-extensible receiver or a non-configurable property throws a TypeError), the anonymous function name of an initializer (NamedEvaluation with the field's key), the instance fields run by InitializeInstanceElements at a base class's [[Construct]] in their order, and the static fields defined on the constructor after every method, in the order of the static elements. It lifts the hold `class field is not supported before slice 8` of `story:class-element-syntax-and-early-errors`. Order 3, after `story:class-definition-evaluation` (the class's form and the seam) and `story:class-element-syntax-and-early-errors` (the field's tree and context), built beside `story:derived-classes-and-super-call`, `story:class-static-blocks` and `story:private-names-and-private-methods`.

A field initializer runs as a frame of the run loop, not as a reentry: the seam `initialize_instance_elements` pushes the initializer's frame before the constructor body's (design change 1), so that the step budget, the frame depth and an interrupt reach the code of an initializer as they reach a call.

The list is `docs/plan/slice-08/story-public-fields.cases.txt`: the 515 cases of the key `public-fields` of `docs/plan/slice-08/scan.tsv` (a class with public fields and no heritage, SuperCall, private name or static block; no negative case). By directory: `language/{statements,expressions}/class/elements` 393 (the generated `*-field-*`, `*-computed-names*`, `*-literal-names*`, `fields-*` and `static-field-*` cases), `language/statements/class/cpn-class-decl-fields-*` and `language/expressions/class/cpn-class-expr-fields-*` (computed keys of fields), and others. Their 1,029 runs fail on the plan's base run, all on `class is not supported before slice 8`. The base run of this plan is the full run of 2026-11-26 on `149e12f1` (slice 8: `test262-slice 08 9302 18455 0 0 9302 5 9297 0`).

Design it builds on: design change 1 of this decomposition (the section "Classes", written by a design author before the units start): the class constructor box's list [[Fields]] (key, or the index of a computed key evaluated at definition, and placement), the field initializers of a class as one synthetic method per placement (instance and static), whose body defines each field in turn with its key, and its frames in the run loop. The unit reads the text that landed and follows it where it differs from this story; question Q3 of the decomposition's report (one synthetic method per class, the default, against one function per field) is decided there.

## What later stories reuse

[[Fields]] and the fields loop of the seam (`story:instance-elements-initialization-order` orders it after the private methods and runs it after `super()`; `story:private-fields-and-private-in` adds private fields to the same list); the synthetic initializer method and its frame (`story:eval-in-class-field-initializers` gives it [[ClassFieldInitializerName]]; `story:class-static-blocks` runs its blocks in the same list of static elements).

## Domain relations

- A class constructor → its fields (an ordered list, each with one key fixed at the class's definition); a field initializer → the class's home object of its placement — inferable (inferred from ECMA-262 `#sec-classfielddefinition-record-specification-type` and `#sec-definefield`; no ess/1 document declares it: `spec/domains/host.yaml` holds no engine value, and ESS does not restate ECMA-262, `AGENTS.md`, "Rules").

## Change

Failing tests first: the tests below are written before the code and fail on the base.

- `crates/syntax/src/parser/class.rs`: the hold `class field` is lifted (its line); the documentation of `held` in `parser/mod.rs` drops its sentence; `crates/syntax/tests/sources/refused/class-field.js` moves to `valid/`.
- `swell-compiler`, `crates/compiler/src/class.rs`: the arm of a public field (the computed key evaluated in order and kept in a slot of the class scope; the synthetic initializer method of each placement, its body one definition per field); the instruction that defines a field (CreateDataPropertyOrThrow on `this`) in `bytecode.rs` and `disasm.rs`; the name of an anonymous function or class initializer. A compiler test `crates/compiler/tests/sources/class-fields.{js,disasm}` (new).
- `crates/swell`: the box's [[Fields]] (`object/ecmascript_function.rs`, traced and counted); the fields loop of the seam in `interp/class.rs` (the initializer's frame pushed before the constructor body, its result discarded, an abrupt completion ending the construct); the static fields run on the constructor at the end of the class's definition, in the list of static elements; the define instruction's work in a function of its own, called from its arm in `interp/dispatch.rs`.
- Tests, in `crates/swell/tests/class_fields.rs` (new; `tests/main.rs` generated by `sh taskfiles/test-main.sh write`), each in both collection modes: fields with and without initializers, their order, computed keys evaluated once with their side effects at definition, `this` and `new.target` in an initializer, `super.x` in an initializer, the name of `x = function () {}` and `x = class {}`, a field over a getter of the prototype (a data property on the instance), a static field defined after an earlier static initializer froze the constructor (`static a = Object.freeze(this); static b = 1;`: TypeError from DefineField), static fields and their `this`; an initializer that loops stopped by the step budget (`Termination::StepBudget`) and by an interrupt (`Termination::Interrupted`); 10,000 constructions of a class with 100 fields within a `MemoryBudget` (`Termination::MemoryBudget` when it is too small; `heap_bytes()` back to its value after a collection); a recursion of `new C` in an initializer stopping at `frame_depth` with a RangeError; the record size test unchanged.
- Time ("Tests stay fast", the `AGENTS.md` of the folder that holds the repository, and `AGENTS.md`, "Commands and the gate"): a test is terminated at 60 s and counts as failed, and the gate fails when the wall time of its test steps passes 120 s. A test stays under 1.5 CPU seconds alone in the default mode, under 2 with `collect-every-safe-point` and under 2.5 at `SWELL_FULL_COUNTS=1`, and a language script under 0.5 CPU seconds per run, measured as `AGENTS.md` says (one job of the pool, the test binary run with `--exact <name> --test-threads 1`, user plus system). A product several tests share is built once inside one test function or is a committed file, never per test; no test makes a network request or a model call; the unit runs the test binaries of its own change, never the whole gate (`slot task check`) and never a whole-workspace test run. Builds and tests go through the pool: `cargo` is the shim of the pool, anything else that compiles or tests is prefixed `slot`; a pool job ends after 10 minutes unless it names its time (`slot --timeout 30m` or `SLOT_TIMEOUT=30m`, 30 minutes at most); a script that measures or waits runs without the prefix. No Python in any tool, test or command (a scratch helper that is never committed excepted); intermediate files in the scratch directory or `.out/`.
- Size: about 650 lines (compiler 180, engine 200, tests 270): one session.

## Requirements

- None of `conformance/host/requirements.txt` moves: `grep -c "pending slice 8$" conformance/host/requirements.txt` prints `1` before and after.
- Rules every story carries. No unsafe code and no third-party crate: `git diff --stat "$BASE" -- Cargo.lock "*/Cargo.toml"` prints nothing and `slot task forbid-unsafe-check` exits 0. Decision 13: no panic on any input (no `unwrap`, no `expect`, no unchecked indexing; arithmetic that can exceed its type checked or explicitly wrapping). The run loop: nothing is added to the frame of `Agent::execute`; the define instruction does its work in a function of its own; the initializer runs as a frame of the run loop, never through a reentry; `STACK_BYTES_PER_REENTRY` (14 KiB) is unchanged, and if a stack test shows a change it is measured as `docs/plan/slice-02-run-loop-frame.md` does, never raised unmeasured.

Writes: `crates/syntax/src/parser/{class,mod}.rs` (the hold line, the documentation of `held`), `crates/syntax/tests/sources/{refused,valid}/class-field.js` (moved); `crates/compiler/src/{class,bytecode,disasm}.rs`, `crates/compiler/tests/sources/class-fields.{js,disasm}` (new); `crates/swell/src/interp/{class,dispatch}.rs`, `crates/swell/src/object/ecmascript_function.rs` (the box's [[Fields]]), `crates/swell/tests/{class_fields,main}.rs`. A unit that meets a file outside this line that its change cannot compile or pass without makes the smallest change there and reports it. Not written: `crates/syntax/src/{lexer,early}.rs`, `crates/swell/src/dynamic_code.rs`, `crates/test262`, `conformance/`, `website/`, `docs/`.

Shared with the stories of order 3, built at the same time, each in hunks of its own: `crates/syntax/src/parser/class.rs` and the documentation of `held` (one hold line and one sentence each: `story:derived-classes-and-super-call`, `story:class-static-blocks`, `story:private-names-and-private-methods`); `crates/compiler/src/class.rs` (each its own element arm); `crates/swell/src/interp/class.rs` (this story the fields loop of the seam and the static fields in the list of static elements; `story:private-names-and-private-methods` the methods loop of the seam, which it puts before this loop; `story:class-static-blocks` the static blocks in the same list of static elements, one arm of its walk; `story:derived-classes-and-super-call` the SuperCall); the box in `object/ecmascript_function.rs` (one field each); `crates/compiler/src/{bytecode,disasm}.rs` and `interp/dispatch.rs` (each its own instructions and arms); `crates/swell/tests/main.rs` (generated).

## Acceptance

The tests named in Change pass, run by the binaries of this change and not as a whole step: `cargo nextest run --locked -p swell-syntax --test main`; `cargo nextest run --locked -p swell-compiler --test main`; `cargo nextest run --locked -p swell --test main -E 'test(/^class_fields::/) | test(/^classes::/) | test(/^builtins_object_integrity::/) | test(/^language::/) | test(/^limits_scripts::/) | test(/^budgets::/) | test(/^eval_stack::/) | test(/^collect::/) | test(/^termination::/) | test(/^interrupt::/)'` and the same with `--features collect-every-safe-point`; `cargo nextest run --locked -p swell --lib` and the same with `--features collect-every-safe-point`; `cargo nextest run --locked -p swell-test262 --test main`; each exits 0. `slot task clippy` exits 0; `sh taskfiles/test-main.sh check` exits 0; `git diff --stat "$BASE" -- Cargo.lock "*/Cargo.toml"` prints nothing; `slot task forbid-unsafe-check` exits 0; `grep -c "pending slice 8$" conformance/host/requirements.txt` prints `1`. The text: `grep -rn 'class field is not supported\|"class field", 8' crates` prints nothing (on the unit's base it prints the hold line and the fixture's header). Time: every test this story adds stays under its bound as the unit measured it; the report lists each time and the sum, and a time over its bound fails this acceptance.

test262. With the list as committed on the wave branch when the wave closes, `git show "$WAVE:docs/plan/slice-08/story-public-fields.cases.txt" > .out/story-public-fields.cases.txt` (`WAVE` the wave branch the unit merges into) and the guard `docs/plan/slice-08/guard-objects.cases.txt` (the 523 passing cases of `Object.freeze`, `seal`, `preventExtensions`, the `is*` tests, the key and entry functions, `assign`, `getOwnPropertyDescriptors` and `JSON.stringify`, which read own properties as fields make them), `cat .out/story-public-fields.cases.txt docs/plan/slice-08/guard-objects.cases.txt | LC_ALL=C sort -u > .out/part.cases.txt` and `sh docs/plan/slice-08/part.sh "$CHECKOUT" .out/test262-part .out/part.cases.txt` (`CHECKOUT` `.cache/test262/<commit of conformance/test262/PIN>`; 1,038 cases, 2,069 runs over the plan's base run), then, in the background, `SLOT_TIMEOUT=30m cargo run --locked --release -q -p swell-test262 -- count --test262 .out/test262-part --design docs/design/engine.md --features conformance/test262/features.txt --runs .out/test262-part/runs.tsv > .out/test262-part/lines.txt`, and the base run the coordinator made on the join the unit starts from copied to `.out/test262-base/runs.tsv`: (a) `sh docs/plan/slice-08/check.sh .out/story-public-fields.cases.txt .out/test262-base/runs.tsv .out/test262-part/runs.tsv` exits 0 (over the plan's base run, restricted to the part, it exits 1 and prints 1,029 lines `failing` and `passed 523 bound 523 B 523 F 515 R 515`); (b) `awk -F'\t' '$6 ~ /class (field )?is not supported/' .out/test262-part/runs.tsv | wc -l` prints `0` (1,029 over the plan's base run). On the join, the coordinator's full run, made in parts, passes (a) over the whole run.

## Out of scope

Private fields (`story:private-fields-and-private-in`); fields of a derived class and their order with private methods (`story:instance-elements-initialization-order`); eval in an initializer (`story:eval-in-class-field-initializers`); static blocks (`story:class-static-blocks`); the counts of `conformance/test262/expected.txt` (`story:slice-08-conformance-close`).
