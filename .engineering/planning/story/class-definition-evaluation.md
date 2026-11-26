---
format: aep.planning-md/3
id: story:class-definition-evaluation
kind: story
status: draft
title: 'Base classes run: ClassDefinitionEvaluation, class constructors, the class binding, toString and the instance-elements seam'
summary: 3,288 cases (6,494 runs); lifts the hold class; takes the class part of the global declarative record.
relations:
- decomposes: epic:slice-08-classes
- depends_on: story:class-syntax-and-early-errors
- depends_on: story:new-target-meta-property
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
  path: crates/compiler/src/lib.rs
- confidence: cited
  path: crates/compiler/src/resolve.rs
- confidence: cited
  path: crates/compiler/src/statement.rs
- confidence: inferred
  path: crates/compiler/tests/sources/class-base.disasm
- confidence: inferred
  path: crates/compiler/tests/sources/class-base.js
- confidence: cited
  path: crates/swell/src/builtins/function.rs
- confidence: cited
  path: crates/swell/src/dynamic_code.rs
- confidence: cited
  path: crates/swell/src/env/global.rs
- confidence: inferred
  path: crates/swell/src/interp/class.rs
- confidence: cited
  path: crates/swell/src/interp/dispatch.rs
- confidence: cited
  path: crates/swell/src/interp/mod.rs
- confidence: cited
  path: crates/swell/src/object/ecmascript_function.rs
- confidence: cited
  path: crates/swell/src/script.rs
- confidence: inferred
  path: crates/swell/tests/classes.rs
- confidence: inferred
  path: crates/swell/tests/language/classes.js
- confidence: cited
  path: crates/swell/tests/main.rs
- confidence: cited
  path: crates/syntax/src/lib.rs
- confidence: inferred
  path: crates/syntax/src/parser/class.rs
- confidence: cited
  path: crates/syntax/src/parser/mod.rs
- confidence: cited
  path: crates/syntax/tests/sources/refused/class-declaration.js
- confidence: cited
  path: crates/syntax/tests/sources/refused/class-expression.js
- confidence: inferred
  path: crates/syntax/tests/sources/valid/class-declaration.js
- confidence: inferred
  path: crates/syntax/tests/sources/valid/class-expression.js
revision: 3
---
## Context

Slice 8 of 19, `classes` (`epic:slice-08-classes`; `docs/design/engine.md`, section 12, item 8). This story runs a base class: ClassDefinitionEvaluation (`#sec-runtime-semantics-classdefinitionevaluation`) for a class without heritage, fields, private names or static blocks: the class scope and its inner name binding (in its TDZ while the computed keys are evaluated), the prototype object, the constructor (the `constructor` method or the default `constructor() {}`, `#sec-default-constructor-functions`), every method, getter, setter, generator, async and async generator method, static or not, with literal or computed keys (DefineMethodProperty, non-enumerable; a computed static key `prototype` throws a TypeError), [[HomeObject]] of each method, `prototype` and `constructor` with their attributes, the name of an anonymous class (NamedEvaluation); the class binding in its block, at the top of a script in the declarative record (GlobalDeclarationInstantiation `#sec-globaldeclarationinstantiation`) and in eval code; a class constructor's [[Call]] throws a TypeError, its [[Construct]] for a base class makes `this` from NewTarget (OrdinaryCreateFromConstructor) and runs InitializeInstanceElements (`#sec-initializeinstanceelements`) before the body; `Function.prototype.toString` of a class and of its methods gives their source text (`#sec-function.prototype.tostring`). It lifts the hold `class` of `story:class-syntax-and-early-errors` and reads the frame's NewTarget of `story:new-target-meta-property` (both edges). Order 2, built beside `story:class-element-syntax-and-early-errors`.

It creates the seam the element stories fill: `initialize_instance_elements` (a function of a new file `crates/swell/src/interp/class.rs`), which reads the constructor's lists of instance elements (private methods, then fields) and does each; in this story both lists are empty, the call is made where the clause makes it, and a test states the call. `story:public-class-fields` fills the fields, `story:private-names-and-private-methods` the private methods, `story:derived-classes-and-super-call` calls it after `super()`.

On the base (`62c7cf59`; the unit reads every place named here again on its own base): `crates/swell/src/env/global.rs` line 10 says "`class` comes with slice 8" (the entry "Slice 6: declarative part of the global env" of `docs/plan/waves.md`, "What later slices take over", which this story takes: a class declaration at the top of a script binds in the declarative record, `script.rs`, `global_declaration_instantiation`, line 635); `EcmascriptFunction` (`crates/swell/src/object/ecmascript_function.rs`) has `home_object`, `constructor`, `strict` and no class slot; `ordinary_create_from_constructor` and `get_prototype_from_constructor` (`object/ordinary.rs` lines 469 and 480) exist; `function_to_string` (`builtins/function.rs` line 276) slices the source of a function; `ObjectRecord` is 200 bytes (`object::generator::tests::a_generator_keeps_the_object_record_at_its_size`). The builtin constructors (Boolean, Number, Error, Date, Object, Array, String, RegExp, Promise, Function) honour NewTarget already.

The list is `docs/plan/slice-08/story-classes.cases.txt`: the 3,288 cases of the key `classes` of `docs/plan/slice-08/scan.tsv` (a class that needs no other part of slice 8 than this story and `new.target`; no negative case). By directory: `language/{statements,expressions}/class/dstr` 2,088 (patterns in the parameters of class methods of every kind, which run through the binding code as it is), class files 271, `async-gen-method(-static)` 316, `async-method(-static)` 80, `gen-method(-static)` 68, `method(-static)` 56, `accessor-name-{inst,static}` 84, `definition` 36, `language/computed-property-names/class` 29, `language/statements/for-await-of` 30, `built-ins/Function/prototype` 20 (`toString`), and others. Their 6,494 runs fail on the plan's base run: 6,463 on `class is not supported before slice 8`, 3 on `evalScript: refused: class is not supported before slice 8`, 28 as counted refusals. The base run of this plan is the full run of 2026-11-26 on `149e12f1`, whose total and slice lines equal `conformance/test262/expected.txt` on `62c7cf59` (slice 8: `test262-slice 08 9302 18455 0 0 9302 5 9297 0`). This is the largest list of the slice; 2,088 of its cases are generated variants of one template each (the `dstr` directories), and the work is the class's evaluation, not the patterns.

Design it builds on: section 5 of `docs/design/engine.md` (compiled code, the run loop), decision 13 (no panic) and the collector's kind sites. Design change 1 of this decomposition, written by a design author into a new section "Classes" before the units of slice 8 start: ClassDefinitionEvaluation as compiled code (instructions that make the prototype and the constructor and define each element, run by the run loop, no reentry); the class scope as a declarative scope of the compiler; the slots of a class constructor ([[ConstructorKind]], [[Fields]], [[PrivateMethods]], [[ClassFieldInitializerName]]) in one box that `EcmascriptFunction` holds as an `Option`, so that `ObjectRecord` keeps its 200 bytes; default constructors as compiled functions of the class; the two lists of instance elements and the seam. The unit reads the text that landed and follows it where it differs from this story.

## What later stories reuse

The class's compiled form (`crates/compiler/src/class.rs`, new) and its instructions; the class constructor box; the seam `initialize_instance_elements` and its two lists; the base [[Construct]] of a class constructor; the class binding at the top of a script and in eval code; `toString` of a class. Every later class story compiles its elements into this form.

## Domain relations

- A class constructor → its prototype object (one, by `prototype`, and back by `constructor`), a method → its [[HomeObject]] (the prototype, or the constructor for a static method), a class constructor → its lists of instance elements — inferable (inferred from ECMA-262 `#sec-runtime-semantics-classdefinitionevaluation` and `#sec-ecmascript-function-objects`; no ess/1 document declares it: `spec/domains/host.yaml` holds no engine value, and ESS does not restate ECMA-262, `AGENTS.md`, "Rules").

## Change

Failing tests first: the tests below are written before the code and fail on the base (a class is refused).

- `crates/syntax`: the hold `class` is lifted (`parser/class.rs`, its line); the documentation of `held` (`parser/mod.rs`) and of `parse_script` (`lib.rs`) drop it; `refused/class-declaration.js` and `refused/class-expression.js` move to `crates/syntax/tests/sources/valid/`.
- `swell-compiler`: `crates/compiler/src/class.rs` (new; its `mod` line): a class compiles to instructions of their own (make the prototype from `%Object.prototype%`, make the constructor from its template with its home object, define a method, getter or setter non-enumerable on the prototype or the constructor, set the name of an anonymous class) and to the class scope (`resolve.rs`: the inner binding, in its TDZ until the class is made); the default constructor is a function template of the class; a class constructor's template says it is one and is strict. `statement.rs` and `expression.rs`: the arms of `Stmt::Class` and `Expr::Class` (the `CompileError::Tree` arms of `story:class-syntax-and-early-errors` go). Each instruction has its documentation in `Instruction`, its line in `disasm.rs` and a compiler test (`crates/compiler/tests/sources/class-*.js` and `.disasm`, new).
- `crates/swell`: `crates/swell/src/interp/class.rs` (new; `interp/mod.rs` its `mod` line): the work of each new instruction in a function of its own, called from its arm in `interp/dispatch.rs` (the run loop holds no instruction's locals), and `initialize_instance_elements`. `object/ecmascript_function.rs`: the class constructor box (design change 1), traced in `EcmascriptFunction::trace` and counted in the object's bytes; [[Call]] of a class constructor throws a TypeError; its [[Construct]] (`construct_function`, `make_frame`) makes `this` with `ordinary_create_from_constructor(new_target, Object.prototype)` and calls the seam before the body. `script.rs`, `global_declaration_instantiation`: a class declaration binds in the declarative record (uninitialized until the declaration runs), with the redeclaration errors of `#sec-globaldeclarationinstantiation`; `env/global.rs` line 10 rewritten to say so. `dynamic_code.rs`: a class in eval code binds in the eval's lexical environment, as `let` does. `builtins/function.rs`, `function_to_string`: the source text of a class from `class` to its `}`, and of a method from its first token (`static` and `async` included, per `#sec-function.prototype.tostring`).
- Tests, in `crates/swell/tests/classes.rs` (new; `tests/main.rs` is generated by `sh taskfiles/test-main.sh write`) and the language script `crates/swell/tests/language/classes.js` (new), each in both collection modes: a class declaration and expression, each method kind, static methods, accessors, computed keys in order with their side effects, the inner binding immutable and in its TDZ during computed keys, `static ["prototype"]` (TypeError), calling a class (TypeError), `new C` (an object whose prototype is `C.prototype`, also when NewTarget differs through a bound function), the seam called once per construct with empty lists, the class binding in a block, at the top of a script (declarative, not a property of the global object, its TDZ across two scripts), in eval code, `toString`; a class made in a loop of 10,000 iterations within a small `MemoryBudget` (`Termination::MemoryBudget` when it is too small; with enough, `heap_bytes()` after a collection returns to its value: the box and the prototypes leak nothing); a class with 1,000 methods; a deep recursion through a method stopping at `frame_depth` with a RangeError; the record size test stays at 200 bytes.
- Time ("Tests stay fast", the `AGENTS.md` of the folder that holds the repository, and `AGENTS.md`, "Commands and the gate"): a test is terminated at 60 s and counts as failed, and the gate fails when the wall time of its test steps passes 120 s. A test stays under 1.5 CPU seconds alone in the default mode, under 2 with `collect-every-safe-point` and under 2.5 at `SWELL_FULL_COUNTS=1`, and a language script under 0.5 CPU seconds per run, measured as `AGENTS.md` says (one job of the pool, the test binary run with `--exact <name> --test-threads 1`, user plus system). A product several tests share is built once inside one test function or is a committed file, never per test; no test makes a network request or a model call; the unit runs the test binaries of its own change, never the whole gate (`slot task check`) and never a whole-workspace test run. Builds and tests go through the pool: `cargo` is the shim of the pool, anything else that compiles or tests is prefixed `slot`; a pool job ends after 10 minutes unless it names its time (`slot --timeout 30m` or `SLOT_TIMEOUT=30m`, 30 minutes at most); a script that measures or waits runs without the prefix. No Python in any tool, test or command (a scratch helper that is never committed excepted); intermediate files in the scratch directory or `.out/`.
- Size: about 1,100 lines (compiler 350, engine 350, global and eval bindings 80, `toString` 40, tests 280): one session at the upper end; the split, if the critics ask for one, is `toString` and the global binding into a story of their own (about 250 lines, 40 cases).

## Requirements

- None of `conformance/host/requirements.txt` moves: `grep -c "pending slice 8$" conformance/host/requirements.txt` prints `1` before and after. No realm item and no intrinsic is added (the class's prototype objects are made per class): if the unit adds one, `Intrinsics::trace` marks it, `host_objects::host_186_a_host_function_charges_every_byte` and `host_objects::host_187` pass in both modes with their readings before and after named in the report.
- Rules every story carries. No unsafe code and no third-party crate: `git diff --stat "$BASE" -- Cargo.lock "*/Cargo.toml"` prints nothing and `slot task forbid-unsafe-check` exits 0. Decision 13: no panic on any input (no `unwrap`, no `expect`, no unchecked indexing; arithmetic that can exceed its type checked or explicitly wrapping). The run loop: nothing is added to the frame of `Agent::execute`; each new instruction does its work in a function of its own; `STACK_BYTES_PER_REENTRY` (14 KiB) is unchanged, and if a stack test shows a change it is measured as `docs/plan/slice-02-run-loop-frame.md` does, never raised unmeasured. Every allocation of a class (prototype, constructor, box, methods) is charged to the memory budget as other objects are.

Writes: `crates/syntax/src/parser/{class,mod}.rs` (the hold line; the documentation of `held`), `crates/syntax/src/lib.rs` (the documentation of `parse_script`), `crates/syntax/tests/sources/{refused,valid}/class-{declaration,expression}.js` (moved); `crates/compiler/src/{class,bytecode,disasm,statement,expression,resolve,lib}.rs` (`class.rs` new), `crates/compiler/tests/sources/class-*.{js,disasm}` (new); `crates/swell/src/interp/{class,dispatch,mod}.rs` (`class.rs` new), `crates/swell/src/object/ecmascript_function.rs` (the box, its trace, [[Call]], `construct_function`, `make_frame`), `crates/swell/src/{script,dynamic_code}.rs`, `crates/swell/src/env/global.rs` (line 10), `crates/swell/src/builtins/function.rs` (`function_to_string`), `crates/swell/tests/{classes,main}.rs` (`classes.rs` new, `main.rs` generated), `crates/swell/tests/language/classes.js` (new). A unit that meets a file outside this line that its change cannot compile or pass without makes the smallest change there and reports it. Not written: `crates/syntax/src/{lexer,early,scope}.rs`, `crates/test262`, `conformance/`, `website/`, `docs/`.

Shared with `story:class-element-syntax-and-early-errors`, built at the same time, each in hunks of its own: `crates/syntax/src/parser/class.rs` (this story, the hold line `class`; that story, the element parse and its holds) and the documentation of `held` in `parser/mod.rs` (each its own sentence); `crates/syntax/src/ast.rs` (that story adds element kinds to the `Class` record; this story's compiler arms match the kinds of its base and answer `CompileError::Tree` for a kind it does not know, which that story's holds keep unreachable: the second to merge adds the arm of the new kinds). Files later stories edit after this one: `crates/compiler/src/class.rs` and `crates/swell/src/interp/class.rs` (`story:derived-classes-and-super-call`, `story:public-class-fields`, `story:class-static-blocks`, `story:private-names-and-private-methods`, `story:private-fields-and-private-in`, each its own element arm and its own loop of the seam); the class constructor box (`story:derived-classes-and-super-call` its kind, `story:public-class-fields` [[Fields]], `story:private-names-and-private-methods` [[PrivateMethods]], `story:eval-in-class-field-initializers` [[ClassFieldInitializerName]]).

## Acceptance

The tests named in Change pass, run by the binaries of this change and not as a whole step: `cargo nextest run --locked -p swell-syntax --test main` and the same with `SWELL_FULL_COUNTS=1`; `cargo nextest run --locked -p swell-compiler --test main`; `cargo nextest run --locked -p swell --test main -E 'test(/^classes::/) | test(/^new_target::/) | test(/^scripts::/) | test(/^eval::/) | test(/^builtins_function_source::/) | test(/^lexical_declarations::/) | test(/^generator_methods::/) | test(/^async_methods::/) | test(/^async_generator_methods::/) | test(/^language::/) | test(/^limits_scripts::/) | test(/^budgets::/) | test(/^eval_stack::/) | test(/^collect::/) | test(/^termination::/)'` and the same with `--features collect-every-safe-point`; `cargo nextest run --locked -p swell --lib` and the same with `--features collect-every-safe-point` (the record size test, `heap::collect`); `cargo nextest run --locked -p swell-test262 --test main`; each exits 0. `slot task clippy` exits 0; `sh taskfiles/test-main.sh check` exits 0; `git diff --stat "$BASE" -- Cargo.lock "*/Cargo.toml"` prints nothing; `slot task forbid-unsafe-check` exits 0; `grep -c "pending slice 8$" conformance/host/requirements.txt` prints `1`. The text: `grep -rn 'class. comes with slice 8\|"class", 8' crates` prints nothing (on the base it prints `crates/swell/src/env/global.rs:10` and `parser/statement.rs:222`, the latter gone with `story:class-syntax-and-early-errors`), and `ls crates/syntax/tests/sources/refused/class-declaration.js` fails. Time: every test this story adds stays under its bound as the unit measured it; the report lists each time and the sum, and a time over its bound fails this acceptance.

test262. With the list as committed on the wave branch when the wave closes, `git show "$WAVE:docs/plan/slice-08/story-classes.cases.txt" > .out/story-classes.cases.txt` (`WAVE` the wave branch the unit merges into) and the guard `docs/plan/slice-08/guard-functions.cases.txt` (the 2,546 passing cases of functions, arrows, methods, calls, `new`, eval code and `Function`), `cat .out/story-classes.cases.txt docs/plan/slice-08/guard-functions.cases.txt | LC_ALL=C sort -u > .out/part.cases.txt` and `sh docs/plan/slice-08/part.sh "$CHECKOUT" .out/test262-part .out/part.cases.txt` (`CHECKOUT` `.cache/test262/<commit of conformance/test262/PIN>`; 5,834 cases, 10,766 runs over the plan's base run), then, in the background, `SLOT_TIMEOUT=30m cargo run --locked --release -q -p swell-test262 -- count --test262 .out/test262-part --design docs/design/engine.md --features conformance/test262/features.txt --runs .out/test262-part/runs.tsv > .out/test262-part/lines.txt`, and the base run the coordinator made on the join the unit starts from copied to `.out/test262-base/runs.tsv`: (a) `sh docs/plan/slice-08/check.sh .out/story-classes.cases.txt .out/test262-base/runs.tsv .out/test262-part/runs.tsv` exits 0 (over the plan's base run, restricted to the part, it exits 1 and prints 6,494 lines `failing` and `passed 2546 bound 2546 B 2546 F 3288 R 3288`); (b) `awk -F'\t' '$6 ~ /class is not supported/' .out/test262-part/runs.tsv | wc -l` prints `0` (6,466 over the plan's base run). On the join, the coordinator's full run, made in parts, passes (a) over the whole run.

## Out of scope

`extends`, `super` in a class, derived constructors (`story:derived-classes-and-super-call`); fields, static blocks, private names (their stories); subclassing builtins (`story:derived-classes-and-super-call`); `Reflect.construct` (slice 9); the counts of `conformance/test262/expected.txt` (`story:slice-08-conformance-close`).
