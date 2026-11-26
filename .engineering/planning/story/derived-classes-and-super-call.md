---
format: aep.planning-md/3
id: story:derived-classes-and-super-call
kind: story
status: draft
title: 'Derived classes: heritage, super(), the this TDZ, super in class methods, arrows and eval code'
summary: 356 cases (712 runs); lifts class heritage, super in an arrow function and super in eval code.
relations:
- decomposes: epic:slice-08-classes
- depends_on: story:class-definition-evaluation
- depends_on: story:super-property-and-super-syntax
scope:
- confidence: cited
  path: crates/compiler/src/bytecode.rs
- confidence: inferred
  path: crates/compiler/src/class.rs
- confidence: cited
  path: crates/compiler/src/disasm.rs
- confidence: cited
  path: crates/compiler/src/expression.rs
- confidence: cited
  path: crates/compiler/src/resolve.rs
- confidence: inferred
  path: crates/compiler/tests/sources/class-derived.disasm
- confidence: inferred
  path: crates/compiler/tests/sources/class-derived.js
- confidence: cited
  path: crates/swell/src/dynamic_code.rs
- confidence: cited
  path: crates/swell/src/interp/arrow.rs
- confidence: inferred
  path: crates/swell/src/interp/class.rs
- confidence: cited
  path: crates/swell/src/interp/dispatch.rs
- confidence: inferred
  path: crates/swell/src/interp/super_ref.rs
- confidence: cited
  path: crates/swell/src/object/ecmascript_function.rs
- confidence: inferred
  path: crates/swell/tests/derived_classes.rs
- confidence: inferred
  path: crates/swell/tests/language/derived-classes.js
- confidence: cited
  path: crates/swell/tests/main.rs
- confidence: cited
  path: crates/syntax/src/early.rs
- confidence: cited
  path: crates/syntax/src/lib.rs
- confidence: inferred
  path: crates/syntax/src/parser/class.rs
- confidence: cited
  path: crates/syntax/src/parser/expression.rs
- confidence: cited
  path: crates/syntax/src/parser/function.rs
- confidence: cited
  path: crates/syntax/src/parser/mod.rs
- confidence: inferred
  path: crates/syntax/tests/sources/refused/class-heritage.js
- confidence: inferred
  path: crates/syntax/tests/sources/refused/super-in-arrow.js
- confidence: inferred
  path: crates/syntax/tests/sources/refused/super-in-eval.js
revision: 3
---
## Context

Slice 8 of 19, `classes` (`epic:slice-08-classes`; `docs/design/engine.md`, section 12, item 8). This story runs a derived class and every `super` in a class: the heritage (`#sec-runtime-semantics-classdefinitionevaluation`, the steps of ClassHeritage: `null` gives a prototype without parent and `%Function.prototype%` as the constructor's parent; a value that is not a constructor, or whose `prototype` is neither an object nor `null`, throws a TypeError), [[ConstructorKind]] derived, the default derived constructor (`#sec-default-constructor-functions`: it passes its arguments to the parent without reading `%Array.prototype%[@@iterator]`), SuperCall (`#sec-super-keyword-runtime-semantics-evaluation`: GetSuperConstructor, the IsConstructor check, Construct with the running NewTarget, BindThisValue, a ReferenceError when `this` is bound twice, then InitializeInstanceElements on the result), the `this` of a derived constructor in its TDZ until `super()` returns (`#sec-function-environment-records`, GetThisBinding throws a ReferenceError), the return of a derived constructor (an object, or `undefined` with `this` bound, else a TypeError; `#sec-ecmascript-function-objects-construct-argumentslist`), SuperProperty in class methods, accessors, static methods and constructors (the static ones' [[HomeObject]] is the constructor), `super` in an arrow function at run time (through the function its `this` comes from), and `super` in direct eval code (PerformEval's `inMethod` and `inDerivedConstructor`: `#sec-performeval`, "If inMethod is false, and body Contains SuperProperty, throw a SyntaxError exception", and the same for SuperCall with `inDerivedConstructor`). Subclassing a builtin (`class P extends Promise`, `extends Array`, `extends RegExp`, `extends Error`) needs nothing more than this: the builtin constructors already make their object from NewTarget. Order 3, after `story:class-definition-evaluation` (the class's form, the seam) and `story:super-property-and-super-syntax` (the super grammar and its instructions, its two holds), built beside `story:public-class-fields`, `story:class-static-blocks` and `story:private-names-and-private-methods`.

It lifts the holds `class heritage is not supported before slice 8` (`story:class-syntax-and-early-errors`), `super in an arrow function is not supported before slice 8` and `super in eval code is not supported before slice 8` (`story:super-property-and-super-syntax`), and admits a SuperCall in the constructor of a class with heritage (the early error stays everywhere else: a base class constructor, a method, a field initializer, a static block, a function). It reads the eval context of `story:new-target-meta-property` (its flags `inMethod` and `inDerivedConstructor`, true now for the functions this story makes) and its arrow box. The seam `initialize_instance_elements` is called after `super()` with the constructor the SuperCall belongs to (for a SuperCall in an arrow, the function the arrow's `this` comes from); its lists are empty until the element stories fill them.

Design it builds on: decision 3 of `docs/design/engine.md`, "`this` and `arguments` in an arrow function", item 1: slice 8 changes the arrows of a derived constructor only, so that `this` becomes a slot of the constructor's function environment, read by an arrow made before `super()` and bound by a `super()` in that arrow. Design changes 1 and 3 of this decomposition (the section "Classes" and the arrow's box, written by a design author before the units start) say where that slot lives (the frame of the derived constructor, reached by the arrow through its box) and how the TDZ of `this` is read; design change 4 says how the eval context's flags are read. The unit reads the text that landed and follows it where it differs.

The list is `docs/plan/slice-08/story-derived.cases.txt`: the 356 cases of the key `derived` of `docs/plan/slice-08/scan.tsv` (a class with heritage or a SuperCall and no field, static block or private name; a SuperProperty in a class; `super` in eval code; `super` in an arrow; 29 negative). By directory: `language/statements/class/subclass` 96, `language/expressions/super` 74, `language/{statements,expressions}/class/subclass-builtins` 42, `language/statements/class/{definition,super,syntax,name-binding,arguments}` 32, `language/{statements,expressions}/class/elements` 26, class files 13, `language/eval-code` 12, `built-ins/Promise` 26 (a Promise subclass as the receiver of a static method), `built-ins/Function` 9 (`[[Construct]]` of a derived class, `toString` with heritage), `built-ins/{RegExp,Error,String,AsyncGeneratorPrototype}` 5, `language/expressions/{optional-chaining,delete,arrow-function,assignment}` 15, and others. Their 712 runs fail on the plan's base run: 676 on `class is not supported before slice 8`, 4 on `super is not supported before slice 8`, 32 as counted refusals. The base run of this plan is the full run of 2026-11-26 on `149e12f1` (slice 8: `test262-slice 08 9302 18455 0 0 9302 5 9297 0`). The subclass cases of `ArrayBuffer`, `DataView` and the typed arrays need slice 13 and are on `close-later.tsv`; those through `Reflect.construct` or a `Proxy` need slice 9 and are on `close-slice-9.cases.txt`.

## What later stories reuse

The derived [[Construct]] and the `this` slot of a derived constructor (`story:instance-elements-initialization-order` reads it for field initializers of a derived class); the call of the seam after `super()`; the rules of eval code for `super` (`story:eval-in-class-field-initializers` adds the field-initializer rule beside them).

## Domain relations

- A derived class constructor → its parent constructor (its [[Prototype]], read at each SuperCall) and its prototype → the parent's `prototype` (read once at ClassDefinitionEvaluation); a derived constructor's frame → at most one bound `this` — inferable (inferred from ECMA-262 `#sec-getsuperconstructor`, `#sec-bindthisvalue` and `#sec-runtime-semantics-classdefinitionevaluation`; no ess/1 document declares it: `spec/domains/host.yaml` holds no engine value, and ESS does not restate ECMA-262, `AGENTS.md`, "Rules").

## Change

Failing tests first: the tests below are written before the code and fail on the base.

- `crates/syntax`: the three holds of Context are lifted (`parser/class.rs`, `parser/expression.rs` and the eval path, one line each, and the documentation of `held` in `parser/mod.rs`); the class constructor of a class with heritage sets the flag of SuperCall (`parser/class.rs`), an arrow takes it from the code around it (`parser/function.rs`, where `story:new-target-meta-property` and `story:super-property-and-super-syntax` keep their flags); `crates/syntax/src/lib.rs`, `parse_eval_with`: SuperProperty in eval code is an early error unless `inMethod`, SuperCall unless `inDerivedConstructor` (`early.rs`); the documentation of `parse_eval` names these two rules. `crates/syntax/tests/sources/refused/{class-heritage,super-in-arrow,super-in-eval}.js` move to `valid/`, and `early-errors/super-call-*.js` (new: in a base constructor, a method, a static method, an arrow in a base constructor).
- `swell-compiler`: heritage and the derived constructor in `crates/compiler/src/class.rs`; SuperCall (an instruction with its arguments, spread included); the default derived constructor; the `this` of a derived constructor and of the arrows inside it read through the TDZ check (an instruction of its own; `Expr::This` in `expression.rs` near line 258 compiles to it in a derived constructor and its arrows only, the other functions keep theirs); `super` in an arrow compiles to the super instructions of `story:super-property-and-super-syntax`, with the home object read through the arrow's box. Compiler tests `crates/compiler/tests/sources/class-derived-*.{js,disasm}` (new).
- `crates/swell`: `interp/class.rs`, the heritage checks and the derived constructor's prototype chain; the SuperCall's work in a function of its own (GetSuperConstructor, IsConstructor, `construct` with the frame's NewTarget, BindThisValue on the constructor's frame, the seam), called from its arm in `interp/dispatch.rs`; `object/ecmascript_function.rs`, the derived [[Construct]] (no `this` made, the return rule) and the box's kind; `interp/super_ref.rs`, the home object read through an arrow's box; `dynamic_code.rs`, the two flags computed for a class method and a derived constructor (the context's computation of `story:new-target-meta-property` reads the class box). The `this` slot of decision 3 for the arrows of a derived constructor: an arrow made before `super()` reads `this` from the constructor's frame through its box, so that `super()` in the arrow or in the constructor binds it once for both.
- Tests, in `crates/swell/tests/derived_classes.rs` (new; `tests/main.rs` generated by `sh taskfiles/test-main.sh write`) and the language script `crates/swell/tests/language/derived-classes.js` (new), each in both collection modes: `extends` of a class, of a function, of `null` (constructing throws), of a non-constructor and of a bad `prototype` (TypeError); `super()` twice (ReferenceError after the parent ran), `this` before `super()` (ReferenceError), `super()` in an arrow before and after the arrow is called, after a collection at every safe point between the arrow's creation and its call; a derived constructor returning an object, `undefined`, a primitive (TypeError); `super.m()` in methods, static methods (home object the constructor) and accessors; `super` in eval in a method and in a derived constructor, and its SyntaxError elsewhere; `class extends Promise`, `extends Array` (its `length`), `extends Error`, `extends RegExp`; a chain of 1,000 derived classes constructed within the step budget with `steps_per_entry` small (`Termination::StepBudget`) and its depth in nested `super()` stopping at `frame_depth` with a RangeError; the reentry of a `super()` into a host constructor bounded by `reentry_depth` (a RangeError, not a panic); the record size test stays at 200 bytes.
- Time ("Tests stay fast", the `AGENTS.md` of the folder that holds the repository, and `AGENTS.md`, "Commands and the gate"): a test is terminated at 60 s and counts as failed, and the gate fails when the wall time of its test steps passes 120 s. A test stays under 1.5 CPU seconds alone in the default mode, under 2 with `collect-every-safe-point` and under 2.5 at `SWELL_FULL_COUNTS=1`, and a language script under 0.5 CPU seconds per run, measured as `AGENTS.md` says (one job of the pool, the test binary run with `--exact <name> --test-threads 1`, user plus system). A product several tests share is built once inside one test function or is a committed file, never per test; no test makes a network request or a model call; the unit runs the test binaries of its own change, never the whole gate (`slot task check`) and never a whole-workspace test run. Builds and tests go through the pool: `cargo` is the shim of the pool, anything else that compiles or tests is prefixed `slot`; a pool job ends after 10 minutes unless it names its time (`slot --timeout 30m` or `SLOT_TIMEOUT=30m`, 30 minutes at most); a script that measures or waits runs without the prefix. No Python in any tool, test or command (a scratch helper that is never committed excepted); intermediate files in the scratch directory or `.out/`.
- Size: about 900 lines (syntax 80, compiler 200, engine 300, tests 320): one session.

## Requirements

- None of `conformance/host/requirements.txt` moves: `grep -c "pending slice 8$" conformance/host/requirements.txt` prints `1` before and after.
- Rules every story carries. No unsafe code and no third-party crate: `git diff --stat "$BASE" -- Cargo.lock "*/Cargo.toml"` prints nothing and `slot task forbid-unsafe-check` exits 0. Decision 13: no panic on any input (no `unwrap`, no `expect`, no unchecked indexing; arithmetic that can exceed its type checked or explicitly wrapping). The run loop: nothing is added to the frame of `Agent::execute`; each new instruction does its work in a function of its own; a SuperCall into a host constructor goes through the reentry of a host call as `new` does (`reentry_depth`), into an ECMAScript constructor as a frame of the run loop (`frame_depth`); `STACK_BYTES_PER_REENTRY` (14 KiB) is unchanged, and if a stack test shows a change it is measured as `docs/plan/slice-02-run-loop-frame.md` does, never raised unmeasured.

Writes: `crates/syntax/src/{early,lib}.rs`, `crates/syntax/src/parser/{class,expression,function,mod}.rs` (the holds' lines, the SuperCall flag, the documentation of `held`), `crates/syntax/tests/sources/{refused,valid,early-errors}/*` as listed; `crates/compiler/src/{class,bytecode,disasm,expression,resolve}.rs`, `crates/compiler/tests/sources/class-derived-*` (new); `crates/swell/src/interp/{class,dispatch,super_ref,arrow}.rs`, `crates/swell/src/object/ecmascript_function.rs`, `crates/swell/src/dynamic_code.rs`, `crates/swell/tests/{derived_classes,main}.rs`, `crates/swell/tests/language/derived-classes.js` (new). A unit that meets a file outside this line that its change cannot compile or pass without makes the smallest change there and reports it. Not written: `crates/syntax/src/lexer.rs`, `crates/swell/src/builtins/` (subclassing needs no builtin change; a case that shows one is reported, not fixed here), `crates/test262`, `conformance/`, `website/`, `docs/`.

Shared with the stories of order 3, built at the same time, each in hunks of its own: `crates/syntax/src/parser/class.rs` (each story its own hold line: this story `class heritage`, `story:public-class-fields` `class field`, `story:class-static-blocks` `class static block`, `story:private-names-and-private-methods` `private method`; the second to merge keeps the other lines' removal) and the documentation of `held` (each removes its own sentence); `crates/compiler/src/class.rs` (each its own element arm: this story the heritage and the constructor kind; the others their element kinds); `crates/swell/src/interp/class.rs` (this story the SuperCall and the heritage functions; `story:public-class-fields` the fields loop of the seam; `story:private-names-and-private-methods` the methods loop; `story:class-static-blocks` the static elements' run); the class constructor box in `object/ecmascript_function.rs` (this story its kind; `story:public-class-fields` [[Fields]]; `story:private-names-and-private-methods` [[PrivateMethods]]: one field each); `crates/compiler/src/{bytecode,disasm}.rs` and `interp/dispatch.rs` (each its own instructions and arms); `crates/swell/tests/main.rs` (generated: `sh taskfiles/test-main.sh write` after the merge). `crates/swell/src/dynamic_code.rs` and `crates/syntax/src/lib.rs` (the eval rules) are edited later by `story:eval-in-class-field-initializers` and `story:private-names-in-eval-code`, each beside this story's rules.

## Acceptance

The tests named in Change pass, run by the binaries of this change and not as a whole step: `cargo nextest run --locked -p swell-syntax --test main` and the same with `SWELL_FULL_COUNTS=1`; `cargo nextest run --locked -p swell-compiler --test main`; `cargo nextest run --locked -p swell --test main -E 'test(/^derived_classes::/) | test(/^classes::/) | test(/^super_property::/) | test(/^new_target::/) | test(/^arrow_functions::/) | test(/^eval::/) | test(/^promises::/) | test(/^realms::/) | test(/^language::/) | test(/^limits_scripts::/) | test(/^budgets::/) | test(/^eval_stack::/) | test(/^collect::/) | test(/^termination::/)'` and the same with `--features collect-every-safe-point`; `cargo nextest run --locked -p swell --lib` and the same with `--features collect-every-safe-point`; `cargo nextest run --locked -p swell-test262 --test main`; each exits 0. `slot task clippy` exits 0; `sh taskfiles/test-main.sh check` exits 0; `git diff --stat "$BASE" -- Cargo.lock "*/Cargo.toml"` prints nothing; `slot task forbid-unsafe-check` exits 0; `grep -c "pending slice 8$" conformance/host/requirements.txt` prints `1`. The text: `grep -rn 'class heritage is not supported\|super in an arrow function is not supported\|super in eval code is not supported\|"class heritage", 8' crates` prints nothing (on the unit's base it prints the three hold lines and their fixtures). Time: every test this story adds stays under its bound as the unit measured it; the report lists each time and the sum, and a time over its bound fails this acceptance.

test262. With the list as committed on the wave branch when the wave closes, `git show "$WAVE:docs/plan/slice-08/story-derived.cases.txt" > .out/story-derived.cases.txt` (`WAVE` the wave branch the unit merges into) and the guard `docs/plan/slice-08/guard-functions.cases.txt` (the 2,546 passing cases of functions, arrows, methods, calls, `new`, eval code and `Function`), `cat .out/story-derived.cases.txt docs/plan/slice-08/guard-functions.cases.txt | LC_ALL=C sort -u > .out/part.cases.txt` and `sh docs/plan/slice-08/part.sh "$CHECKOUT" .out/test262-part .out/part.cases.txt` (`CHECKOUT` `.cache/test262/<commit of conformance/test262/PIN>`; 2,902 cases, 4,984 runs over the plan's base run), then, in the background, `SLOT_TIMEOUT=30m cargo run --locked --release -q -p swell-test262 -- count --test262 .out/test262-part --design docs/design/engine.md --features conformance/test262/features.txt --runs .out/test262-part/runs.tsv > .out/test262-part/lines.txt`, and the base run the coordinator made on the join the unit starts from copied to `.out/test262-base/runs.tsv`: (a) `sh docs/plan/slice-08/check.sh .out/story-derived.cases.txt .out/test262-base/runs.tsv .out/test262-part/runs.tsv` exits 0 (over the plan's base run, restricted to the part, it exits 1 and prints 712 lines `failing` and `passed 2546 bound 2546 B 2546 F 356 R 356`); (b) `awk -F'\t' '$6 ~ /(class|class heritage|super|super in an arrow function|super in eval code) is not supported/' .out/test262-part/runs.tsv | wc -l` prints `0` (680 over the plan's base run). On the join, the coordinator's full run, made in parts, passes (a) over the whole run.

## Out of scope

Fields, private names and static blocks in derived classes (`story:instance-elements-initialization-order` and the element stories); `super` in field initializers and eval in them (`story:public-class-fields`, `story:eval-in-class-field-initializers`); subclasses of binary data (slice 13, `close-later.tsv`); `Reflect.construct` and `Proxy` (slice 9, `close-slice-9.cases.txt`); the counts of `conformance/test262/expected.txt` (`story:slice-08-conformance-close`).
