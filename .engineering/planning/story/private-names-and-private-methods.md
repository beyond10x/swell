---
format: aep.planning-md/3
id: story:private-names-and-private-methods
kind: story
status: draft
title: Private Names, [[PrivateElements]], and private methods and accessors
summary: 1,981 cases (3,950 runs); lifts private method; one add function for every private element.
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
- confidence: cited
  path: crates/compiler/src/expression.rs
- confidence: cited
  path: crates/compiler/src/resolve.rs
- confidence: inferred
  path: crates/compiler/tests/sources/class-private-methods.disasm
- confidence: inferred
  path: crates/compiler/tests/sources/class-private-methods.js
- confidence: cited
  path: crates/swell/src/builtins/function.rs
- confidence: cited
  path: crates/swell/src/heap/arena.rs
- confidence: cited
  path: crates/swell/src/heap/collect.rs
- confidence: inferred
  path: crates/swell/src/interp/class.rs
- confidence: cited
  path: crates/swell/src/interp/dispatch.rs
- confidence: cited
  path: crates/swell/src/object/ecmascript_function.rs
- confidence: cited
  path: crates/swell/src/object/generator.rs
- confidence: cited
  path: crates/swell/src/object/mod.rs
- confidence: cited
  path: crates/swell/src/object/ordinary.rs
- confidence: inferred
  path: crates/swell/src/object/private_names.rs
- confidence: cited
  path: crates/swell/tests/main.rs
- confidence: inferred
  path: crates/swell/tests/private_methods.rs
- confidence: inferred
  path: crates/syntax/src/parser/class.rs
- confidence: cited
  path: crates/syntax/src/parser/mod.rs
- confidence: inferred
  path: crates/syntax/tests/sources/refused/private-method.js
- confidence: inferred
  path: crates/syntax/tests/sources/valid/private-method.js
revision: 3
---
## Context

Slice 8 of 19, `classes` (`epic:slice-08-classes`; `docs/design/engine.md`, section 12, item 8). This story brings private names and runs private methods and accessors: the Private Name value (`#sec-private-names`, one new value per private name per evaluation of a class), the class's PrivateEnvironment (`#sec-privateenvironment-records`, resolution of `#x` to the Private Name of the nearest enclosing class that declares it, `#sec-resolve-private-identifier`), [[PrivateElements]] of every object (`#sec-object-internal-methods-and-internal-slots`), PrivateMethodOrAccessorAdd (`#sec-privatemethodoraccessoradd`: a TypeError when the object has the element), PrivateElementFind, PrivateGet and PrivateSet (`#sec-privateget`, `#sec-privateset`: a TypeError when the element is missing, when a method is set, when an accessor lacks the half asked for), the private methods of an instance added by InitializeInstanceElements before any field (the methods loop of the seam), static private methods and accessors added to the constructor at the class's definition, `obj.#m()` with `obj` as `this`, private methods as one function object per class evaluation with [[HomeObject]] the prototype or the constructor, and `Function.prototype.toString` of a private method. It lifts the hold `private method is not supported before slice 8` of `story:class-element-syntax-and-early-errors`. Order 3, after `story:class-definition-evaluation` and `story:class-element-syntax-and-early-errors`, built beside `story:derived-classes-and-super-call`, `story:public-class-fields` and `story:class-static-blocks`.

Every add of a private element goes through one function of the object model, so that `story:private-fields-and-private-in` adds fields through it and `story:host-ensure-can-add-private-element` gives it the call of HostEnsureCanAddPrivateElement (`#sec-hostensurecanaddprivateelement`, HOST-119). In this story the hook is not called: HOST-119 says the engine calls it only for host objects with an override table, and the override tables are slice 9's (HOST-114).

Private names are invisible to scripts: no function of the language or of the host interface lists, copies or reads them (`Object.getOwnPropertyNames`, `Object.assign`, `JSON.stringify`, `Object.freeze` do not touch them), and a Private Name reachable from outside its class is a logic error of the object model (section 13 of `docs/design/engine.md`).

The list is `docs/plan/slice-08/story-private-methods.cases.txt`: the 1,981 cases of the key `private-methods` of `docs/plan/slice-08/scan.tsv` (a class with private methods or accessors and no heritage, SuperCall, field or static block; no negative case). By directory: `language/{statements,expressions}/class/dstr` 1,464 (patterns in the parameters of private methods of every kind), `language/{statements,expressions}/class/elements` 426 (the `private-method*`, `private-getter*`, `private-setter*`, `private-static-*` and `*-rs-private-*` cases), `built-ins/Function/prototype` 4 (`toString`), `language/expressions/class/private-*-multiple-evaluations-of-class-*` and others. Their 3,950 runs fail on the plan's base run: 3,920 on `class is not supported before slice 8` and 30 as counted refusals. The base run of this plan is the full run of 2026-11-26 on `149e12f1` (slice 8: `test262-slice 08 9302 18455 0 0 9302 5 9297 0`).

Design it builds on: section 13 of `docs/design/engine.md` (logic errors of the object model), decision 13 (no panic) and the collector's kind sites. Design change 2 of this decomposition, written by a design author into a new section "Private names" before the units start: the Private Name value (question Q1/Q2 of the decomposition's report: its own arena of the heap and a slot kind no script name reaches, the default, against a Symbol marked private), its resolution in the compiler as hidden bindings of the class scope that hold the class evaluation's Private Names, [[PrivateElements]] as a box per object record that grows `ObjectRecord` from 200 to 208 bytes (the default of Q1; against the property map with a private key kind or a side table), the brand check, the charges to the memory budget, the collector (a Private Name and an element are traced from the object and from the class scope), and the invisibility. The unit reads the text that landed and follows it where it differs from this story; if the decision keeps the record at 200 bytes, the size test stays.

## What later stories reuse

The Private Name value and its arena; the class scope's hidden bindings; [[PrivateElements]] and the add, find, get and set functions; the methods loop of the seam; the private member instructions (`story:private-fields-and-private-in` adds the field kind and `#x in`; `story:private-names-in-eval-code` resolves eval code's private names to the same values; `story:host-ensure-can-add-private-element` gives the add its hook).

## Domain relations

- A Private Name → the class evaluation that made it (one; two evaluations of a class make two Private Names); an object → its private elements (each keyed by a Private Name, at most one per name) — inferable (inferred from ECMA-262 `#sec-private-names` and `#sec-privateelement-specification-type`; no ess/1 document declares it: `spec/domains/host.yaml` holds no engine value, and ESS does not restate ECMA-262, `AGENTS.md`, "Rules").

## Change

Failing tests first: the tests below are written before the code and fail on the base.

- `crates/syntax/src/parser/class.rs`: the hold `private method` is lifted (its line); the documentation of `held` in `parser/mod.rs` drops its sentence; `crates/syntax/tests/sources/refused/private-method.js` moves to `valid/`.
- `swell-compiler`: `crates/compiler/src/class.rs`, the arm of a private method, getter and setter (static or not; one function per class evaluation); `resolve.rs`, the hidden bindings of the class scope, one per private name, made at the class's definition and read by `#x` in every function of the class body (a closure captures them as it captures other bindings); the instructions of a private get, set, call and the add of a method in `bytecode.rs` and `disasm.rs`; `expression.rs`, the arms of `Expr::PrivateMember` and of the private link of an optional chain (`a?.#m()`). A compiler test `crates/compiler/tests/sources/class-private-methods.{js,disasm}` (new).
- `crates/swell`: the Private Name value (its arena in the heap, its slot kind, traced by the collector's kind sites; its description, the `#x` text, kept for `toString` of nothing and for error messages only), [[PrivateElements]] on the object record (design change 2; traced, counted in the object's bytes, charged to the memory budget), the add, find, get and set functions in a new file `crates/swell/src/object/private_names.rs` (one add function for every kind), the methods loop of the seam in `interp/class.rs` (before the fields loop of `story:public-class-fields`), the static private methods added to the constructor at the class's definition, the box's [[PrivateMethods]] (`object/ecmascript_function.rs`), each new instruction's work in a function of its own called from its arm in `interp/dispatch.rs`; `builtins/function.rs`, `function_to_string` of a private method (its source text from its first token). The record size test (`object::generator::tests::a_generator_keeps_the_object_record_at_its_size`) moves to the measured size with the measurement named in the report.
- Tests, in `crates/swell/tests/private_methods.rs` (new; `tests/main.rs` generated by `sh taskfiles/test-main.sh write`), each in both collection modes: a private method, getter, setter and accessor pair, static ones, `this.#m()`, `o.#m` on an object without the brand (TypeError), on a primitive (TypeError), set of a method (TypeError), get of a setter-only accessor (TypeError), the same function object for two instances, two evaluations of one class making two brands, a private method of an inner class shadowing an outer one; invisibility: `Object.getOwnPropertyNames`, `Object.getOwnPropertySymbols`, `JSON.stringify`, `Object.assign`, `for in` and `Object.freeze` of an instance see no private element, and the host interface's property listing of the object neither; the collector frees a class evaluation's Private Names and elements when nothing holds them (`heap_bytes()` back to its value after 10,000 class evaluations and a collection) and keeps them while an instance lives (a collection at every safe point between the instance's creation and `o.#m()`); a class with 1,000 private methods; 10,000 instances within a `MemoryBudget` (`Termination::MemoryBudget` when too small); a recursion through `this.#m()` stopping at `frame_depth` with a RangeError.
- Time ("Tests stay fast", the `AGENTS.md` of the folder that holds the repository, and `AGENTS.md`, "Commands and the gate"): a test is terminated at 60 s and counts as failed, and the gate fails when the wall time of its test steps passes 120 s. A test stays under 1.5 CPU seconds alone in the default mode, under 2 with `collect-every-safe-point` and under 2.5 at `SWELL_FULL_COUNTS=1`, and a language script under 0.5 CPU seconds per run, measured as `AGENTS.md` says (one job of the pool, the test binary run with `--exact <name> --test-threads 1`, user plus system). A product several tests share is built once inside one test function or is a committed file, never per test; no test makes a network request or a model call; the unit runs the test binaries of its own change, never the whole gate (`slot task check`) and never a whole-workspace test run. Builds and tests go through the pool: `cargo` is the shim of the pool, anything else that compiles or tests is prefixed `slot`; a pool job ends after 10 minutes unless it names its time (`slot --timeout 30m` or `SLOT_TIMEOUT=30m`, 30 minutes at most); a script that measures or waits runs without the prefix. No Python in any tool, test or command (a scratch helper that is never committed excepted); intermediate files in the scratch directory or `.out/`.
- Size: about 1,000 lines (compiler 250, object model and heap 350, engine arms 100, tests 300): one session.

## Requirements

- None of `conformance/host/requirements.txt` moves: `grep -c "pending slice 8$" conformance/host/requirements.txt` prints `1` before and after (HOST-119 stays pending: `story:host-ensure-can-add-private-element`). The heap's new kind is a kind of the collector: `host_objects::host_186_a_host_function_charges_every_byte` and `host_objects::host_187` pass in both modes, with their readings before and after named in the report (equal, or the change explained by the record's 8 bytes); no realm item and no intrinsic is added (if one is, `Intrinsics::trace` marks it).
- Rules every story carries. No unsafe code and no third-party crate: `git diff --stat "$BASE" -- Cargo.lock "*/Cargo.toml"` prints nothing and `slot task forbid-unsafe-check` exits 0. Decision 13: no panic on any input (no `unwrap`, no `expect`, no unchecked indexing; arithmetic that can exceed its type checked or explicitly wrapping); a Private Name reached where the object model does not expect one is a logic error of section 13 (`Terminated(EngineFault)`), never a panic. The run loop: nothing is added to the frame of `Agent::execute`; each new instruction does its work in a function of its own; `STACK_BYTES_PER_REENTRY` (14 KiB) is unchanged, and if a stack test shows a change it is measured as `docs/plan/slice-02-run-loop-frame.md` does, never raised unmeasured.

Writes: `crates/syntax/src/parser/{class,mod}.rs` (the hold line, the documentation of `held`), `crates/syntax/tests/sources/{refused,valid}/private-method.js` (moved); `crates/compiler/src/{class,resolve,expression,bytecode,disasm}.rs`, `crates/compiler/tests/sources/class-private-methods.{js,disasm}` (new); `crates/swell/src/object/{private_names,mod,ordinary,ecmascript_function,generator}.rs` (`private_names.rs` new; the record's field in its definition; the size test), `crates/swell/src/heap/` (the arena of Private Names and its kind site), `crates/swell/src/interp/{class,dispatch}.rs`, `crates/swell/src/builtins/function.rs` (`function_to_string`), `crates/swell/tests/{private_methods,main}.rs`. A unit that meets a file outside this line that its change cannot compile or pass without makes the smallest change there and reports it. Not written: `crates/syntax/src/{lexer,early}.rs`, `crates/swell/src/host/hooks.rs`, `crates/swell/src/dynamic_code.rs`, `crates/test262`, `conformance/`, `website/`, `docs/`.

Shared with the stories of order 3, built at the same time, each in hunks of its own: `crates/syntax/src/parser/class.rs` and the documentation of `held` (one hold line and one sentence each); `crates/compiler/src/class.rs` (each its own element arm); `crates/compiler/src/resolve.rs` (`story:derived-classes-and-super-call`, the `this` of a derived constructor; `story:class-static-blocks`, the block's scope; this story, the hidden bindings of private names); `crates/compiler/src/expression.rs` (`story:derived-classes-and-super-call`, `Expr::This` in a derived constructor; this story, `Expr::PrivateMember`); `crates/swell/src/interp/class.rs` (this story the methods loop of the seam, placed before the fields loop of `story:public-class-fields`: the second to merge keeps both loops, methods first); the box in `object/ecmascript_function.rs` (one field each); `crates/compiler/src/{bytecode,disasm}.rs` and `interp/dispatch.rs` (each its own instructions and arms); `crates/swell/tests/main.rs` (generated).

## Acceptance

The tests named in Change pass, run by the binaries of this change and not as a whole step: `cargo nextest run --locked -p swell-syntax --test main`; `cargo nextest run --locked -p swell-compiler --test main`; `cargo nextest run --locked -p swell --test main -E 'test(/^private_methods::/) | test(/^classes::/) | test(/^builtins_object::/) | test(/^builtins_object_enumeration::/) | test(/^builtins_object_integrity::/) | test(/^builtins_json::/) | test(/^builtins_function_source::/) | test(/^host_objects::/) | test(/^language::/) | test(/^limits_scripts::/) | test(/^budgets::/) | test(/^eval_stack::/) | test(/^collect::/) | test(/^termination::/)'` and the same with `--features collect-every-safe-point`; `cargo nextest run --locked -p swell --lib` and the same with `--features collect-every-safe-point` (the record size test, `heap::collect`); `cargo nextest run --locked -p swell-test262 --test main`; each exits 0. `slot task clippy` exits 0; `sh taskfiles/test-main.sh check` exits 0; `git diff --stat "$BASE" -- Cargo.lock "*/Cargo.toml"` prints nothing; `slot task forbid-unsafe-check` exits 0; `grep -c "pending slice 8$" conformance/host/requirements.txt` prints `1`. The text: `grep -rn 'private method is not supported\|"private method", 8' crates` prints nothing (on the unit's base it prints the hold line and the fixture's header). Time: every test this story adds stays under its bound as the unit measured it; the report lists each time and the sum, and a time over its bound fails this acceptance.

test262. With the list as committed on the wave branch when the wave closes, `git show "$WAVE:docs/plan/slice-08/story-private-methods.cases.txt" > .out/story-private-methods.cases.txt` (`WAVE` the wave branch the unit merges into) and the guard `docs/plan/slice-08/guard-objects.cases.txt` (the 523 passing cases of the integrity, key and entry functions of `Object` and `JSON.stringify`, which must not see a private element), `cat .out/story-private-methods.cases.txt docs/plan/slice-08/guard-objects.cases.txt | LC_ALL=C sort -u > .out/part.cases.txt` and `sh docs/plan/slice-08/part.sh "$CHECKOUT" .out/test262-part .out/part.cases.txt` (`CHECKOUT` `.cache/test262/<commit of conformance/test262/PIN>`; 2,504 cases, 4,990 runs over the plan's base run), then, in the background, `SLOT_TIMEOUT=30m cargo run --locked --release -q -p swell-test262 -- count --test262 .out/test262-part --design docs/design/engine.md --features conformance/test262/features.txt --runs .out/test262-part/runs.tsv > .out/test262-part/lines.txt`, and the base run the coordinator made on the join the unit starts from copied to `.out/test262-base/runs.tsv`: (a) `sh docs/plan/slice-08/check.sh .out/story-private-methods.cases.txt .out/test262-base/runs.tsv .out/test262-part/runs.tsv` exits 0 (over the plan's base run, restricted to the part, it exits 1 and prints 3,950 lines `failing` and `passed 523 bound 523 B 523 F 1981 R 1981`); (b) `awk -F'\t' '$6 ~ /(class|private method) is not supported/' .out/test262-part/runs.tsv | wc -l` prints `0` (3,920 over the plan's base run). On the join, the coordinator's full run, made in parts, passes (a) over the whole run.

## Out of scope

Private fields and `#x in` (`story:private-fields-and-private-in`); private methods with fields or in derived classes (`story:instance-elements-initialization-order`); private names in eval code (`story:private-names-in-eval-code`); the call of HostEnsureCanAddPrivateElement (`story:host-ensure-can-add-private-element`); the counts of `conformance/test262/expected.txt` (`story:slice-08-conformance-close`).
