---
format: aep.planning-md/3
id: story:private-names-in-eval-code
kind: story
status: draft
title: 'Private names in direct eval code: the private environment of the caller'
summary: 107 cases (203 runs).
relations:
- decomposes: epic:slice-08-classes
- depends_on: story:private-fields-and-private-in
- depends_on: story:eval-in-class-field-initializers
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: crates/compiler/src/bytecode.rs
- confidence: cited
  path: crates/compiler/src/disasm.rs
- confidence: cited
  path: crates/compiler/src/expression.rs
- confidence: cited
  path: crates/compiler/src/resolve.rs
- confidence: inferred
  path: crates/compiler/tests/sources/eval-private-name.disasm
- confidence: inferred
  path: crates/compiler/tests/sources/eval-private-name.js
- confidence: cited
  path: crates/swell/src/dynamic_code.rs
- confidence: inferred
  path: crates/swell/src/interp/class.rs
- confidence: cited
  path: crates/swell/src/interp/dispatch.rs
- confidence: inferred
  path: crates/swell/src/object/private_names.rs
- confidence: inferred
  path: crates/swell/tests/eval_private_names.rs
- confidence: cited
  path: crates/swell/tests/main.rs
- confidence: cited
  path: crates/syntax/src/early.rs
- confidence: cited
  path: crates/syntax/src/lib.rs
revision: 3
---
## Context

Slice 8 of 19, `classes` (`epic:slice-08-classes`; `docs/design/engine.md`, section 12, item 8). This story is private names in direct eval code: PerformEval's private environment (`#sec-performeval`: "Let privateIdentifiers be a new empty List. Let pointer be privateEnv. Repeat, while pointer is not null ... If AllPrivateIdentifiersValid of body with argument privateIdentifiers is false, throw a SyntaxError exception"), and the evaluation of `#x`, `o.#x`, `#x in o` in eval code against the Private Names of the classes around the eval call (EvalDeclarationInstantiation and the eval's running execution context take the caller's PrivateEnvironment, `#sec-eval-declaration-instantiation`), in methods, constructors, field initializers, static blocks and arrows of a class, each evaluation of the class seeing its own names. Indirect eval and `Function` code have no private environment (a private name in them is the SyntaxError). Order 6, after `story:private-fields-and-private-in` (every private element kind and `#x in`) and `story:eval-in-class-field-initializers` (eval in an initializer), built beside `story:host-ensure-can-add-private-element`.

The list is `docs/plan/slice-08/story-eval-private.cases.txt`: the 107 cases of the key `eval-private` of `docs/plan/slice-08/scan.tsv` (a class with a private name and a call of `eval`; no negative case). By directory: `language/statements/class/elements` 59, `language/expressions/class/elements` 41 (`privatename-*-eval-*`, `private-*-direct-eval*`, `*-indirect-eval-*`, `private-field-as-*-eval*`), `language/expressions/class/private-*-multiple-evaluations-of-class-{direct-eval,eval}.js` 7. Their 203 runs fail on the plan's base run: 196 on `class is not supported before slice 8` and 7 as counted refusals. The base run of this plan is the full run of 2026-11-26 on `149e12f1` (slice 8: `test262-slice 08 9302 18455 0 0 9302 5 9297 0`).

Design it builds on: design change 4 of this decomposition (eval in class bodies: the private environment of eval code; eval code is compiled after its class, so its private names resolve against the running frame's chain of class scopes at the time of the call, not against the compiler's scope of the class), written by a design author before the units start; design change 2 (the Private Name value). The unit reads the text that landed and follows it where it differs.

## What later stories reuse

Nothing of slice 8. The private environment of eval code is read by any later slice that adds a kind of code with a class context (none is planned).

## Domain relations

- Direct eval code → the private environment of its caller's running execution context (the chain of class scopes, one) — inferable (inferred from ECMA-262 `#sec-performeval` and `#sec-privateenvironment-records`; no ess/1 document declares it: `spec/domains/host.yaml` holds no engine value, and ESS does not restate ECMA-262, `AGENTS.md`, "Rules").

## Change

Failing tests first: the tests below are written before the code and fail on the base.

- `crates/syntax/src/lib.rs`, `parse_eval_with`: the eval context carries the private names declared by the enclosing classes (their text, outermost last), and AllPrivateIdentifiersValid of eval code reads them (`early.rs`); the documentation of `parse_eval` names it.
- `swell-compiler`: eval code's private names compile to a lookup by text in the running frame's private environment (an instruction of its own, `bytecode.rs` and `disasm.rs`), since the class's hidden bindings of `story:private-names-and-private-methods` are not in the eval's compile-time scope; a compiler test `crates/compiler/tests/sources/eval-private-name.{js,disasm}` (new).
- `crates/swell/src/dynamic_code.rs`: `eval_code_of` and `parse_eval_code` collect the private names of the classes around the call from the running frame (or from the function an arrow's `this` comes from) and pass them in the eval context; the eval's frame gets the caller's private environment; the lookup instruction's work in a function of its own (`interp/class.rs` or `object/private_names.rs`), called from its arm in `interp/dispatch.rs`. An indirect eval passes none.
- Tests, in `crates/swell/tests/eval_private_names.rs` (new; `tests/main.rs` generated by `sh taskfiles/test-main.sh write`), each in both collection modes: `eval("this.#x")` in a method, a constructor, a field initializer, a static block and an arrow inside each; `eval("#x in o")`; writes and calls through eval; a name of an outer class from an inner class's eval; two evaluations of one class each reading its own brand through eval; indirect eval and `Function("return this.#x")` (SyntaxError); an eval that names an undeclared private name (SyntaxError); a collection at every safe point between the eval's parse and its run; nested eval bounded by `eval_stack` and `frame_depth` (RangeError, not a panic); a loop of 10,000 evals reading a private field within the step budget with `steps_per_entry` small (`Termination::StepBudget`) and leaking nothing (`heap_bytes()` back to its value after a collection).
- Time ("Tests stay fast", the `AGENTS.md` of the folder that holds the repository, and `AGENTS.md`, "Commands and the gate"): a test is terminated at 60 s and counts as failed, and the gate fails when the wall time of its test steps passes 120 s. A test stays under 1.5 CPU seconds alone in the default mode, under 2 with `collect-every-safe-point` and under 2.5 at `SWELL_FULL_COUNTS=1`, measured as `AGENTS.md` says (one job of the pool, the test binary run with `--exact <name> --test-threads 1`, user plus system). A product several tests share is built once inside one test function or is a committed file, never per test; no test makes a network request or a model call; the unit runs the test binaries of its own change, never the whole gate (`slot task check`) and never a whole-workspace test run. Builds and tests go through the pool: `cargo` is the shim of the pool, anything else that compiles or tests is prefixed `slot`; a pool job ends after 10 minutes unless it names its time (`slot --timeout 30m` or `SLOT_TIMEOUT=30m`, 30 minutes at most); a script that measures or waits runs without the prefix. No Python in any tool, test or command (a scratch helper that is never committed excepted); intermediate files in the scratch directory or `.out/`.
- Size: about 500 lines (syntax 60, compiler 80, engine 120, tests 240): one session.

## Requirements

- None of `conformance/host/requirements.txt` moves: `grep -c "pending slice 8$" conformance/host/requirements.txt` prints `1` before and after. `swell_syntax::parse_eval_with`'s eval context gains a field: a change of a public item of `swell-syntax`, named in one line under `## [Unreleased]` in `CHANGELOG.md` (the draft `story:syntax-surface-is-promised` would make that a gate check; if its `syntax-surface-check` is in the gate on the unit's base, it exits 0; `story:new-target-meta-property`, which introduces the context, names it the same way).
- Rules every story carries. No unsafe code and no third-party crate: `git diff --stat "$BASE" -- Cargo.lock "*/Cargo.toml"` prints nothing and `slot task forbid-unsafe-check` exits 0. Decision 13: no panic on any input (no `unwrap`, no `expect`, no unchecked indexing; arithmetic that can exceed its type checked or explicitly wrapping); the private names passed to the eval context are charged to the ledger and bounded by the sources of the classes. The run loop: nothing is added to the frame of `Agent::execute`; the lookup instruction does its work in a function of its own; `STACK_BYTES_PER_REENTRY` (14 KiB) is unchanged, and if a stack test shows a change it is measured as `docs/plan/slice-02-run-loop-frame.md` does, never raised unmeasured.

Writes: `crates/syntax/src/{lib,early}.rs`, `crates/compiler/src/{bytecode,disasm,expression,resolve}.rs`, `crates/compiler/tests/sources/eval-private-name.{js,disasm}` (new), `crates/swell/src/dynamic_code.rs`, `crates/swell/src/interp/{class,dispatch}.rs` or `crates/swell/src/object/private_names.rs` (the lookup), `crates/swell/tests/{eval_private_names,main}.rs`, `CHANGELOG.md` (one line under `## [Unreleased]`). A unit that meets a file outside this line that its change cannot compile or pass without makes the smallest change there and reports it. Not written: `crates/syntax/src/parser/`, `crates/swell/src/host/`, `crates/test262`, `conformance/`, `website/`, `docs/`.

Shared with `story:host-ensure-can-add-private-element`, built at the same time: `crates/swell/src/object/private_names.rs` (that story, the hook's call in the add function; this story, if it puts the lookup there, a function of its own beside it) , `CHANGELOG.md` (each one line of its own under `## [Unreleased]`: the second to merge keeps both) and `crates/swell/tests/main.rs` (generated). `crates/syntax/src/lib.rs` and `crates/swell/src/dynamic_code.rs` were last edited by `story:eval-in-class-field-initializers` in order 5; this story adds its field beside that flag.

## Acceptance

The tests named in Change pass, run by the binaries of this change and not as a whole step: `cargo nextest run --locked -p swell-syntax --test main`; `cargo nextest run --locked -p swell-compiler --test main`; `cargo nextest run --locked -p swell --test main -E 'test(/^eval_private_names::/) | test(/^private_fields::/) | test(/^private_methods::/) | test(/^class_field_eval::/) | test(/^eval::/) | test(/^dynamic_function::/) | test(/^language::/) | test(/^limits_scripts::/) | test(/^budgets::/) | test(/^eval_stack::/) | test(/^collect::/) | test(/^termination::/)'` and the same with `--features collect-every-safe-point`; `cargo nextest run --locked -p swell --lib` and the same with `--features collect-every-safe-point`; `cargo nextest run --locked -p swell-test262 --test main`; each exits 0. `slot task clippy` exits 0; `sh taskfiles/test-main.sh check` exits 0; `git diff --stat "$BASE" -- Cargo.lock "*/Cargo.toml"` prints nothing; `slot task forbid-unsafe-check` exits 0; `grep -c "pending slice 8$" conformance/host/requirements.txt` prints `1`. Time: every test this story adds stays under its bound as the unit measured it; the report lists each time and the sum, and a time over its bound fails this acceptance.

test262. With the list as committed on the wave branch when the wave closes, `git show "$WAVE:docs/plan/slice-08/story-eval-private.cases.txt" > .out/story-eval-private.cases.txt` (`WAVE` the wave branch the unit merges into) and the guard `docs/plan/slice-08/guard-functions.cases.txt` (the 2,546 passing cases of functions, arrows, methods, calls, `new`, eval code and `Function`), `cat .out/story-eval-private.cases.txt docs/plan/slice-08/guard-functions.cases.txt | LC_ALL=C sort -u > .out/part.cases.txt` and `sh docs/plan/slice-08/part.sh "$CHECKOUT" .out/test262-part .out/part.cases.txt` (`CHECKOUT` `.cache/test262/<commit of conformance/test262/PIN>`; 2,653 cases, 4,475 runs over the plan's base run), then, in the background, `SLOT_TIMEOUT=30m cargo run --locked --release -q -p swell-test262 -- count --test262 .out/test262-part --design docs/design/engine.md --features conformance/test262/features.txt --runs .out/test262-part/runs.tsv > .out/test262-part/lines.txt`, and the base run the coordinator made on the join the unit starts from copied to `.out/test262-base/runs.tsv`: (a) `sh docs/plan/slice-08/check.sh .out/story-eval-private.cases.txt .out/test262-base/runs.tsv .out/test262-part/runs.tsv` exits 0 (over the plan's base run, restricted to the part, it exits 1 and prints 203 lines `failing` and `passed 2546 bound 2546 B 2546 F 107 R 107`); (b) `awk -F'\t' '$6 ~ /is not supported before slice 8/' .out/test262-part/runs.tsv | wc -l` prints `0` (196 over the plan's base run). On the join, the coordinator's full run, made in parts, passes (a) over the whole run.

## Out of scope

The hook of HOST-119 (`story:host-ensure-can-add-private-element`); private names in `Function` code (none, per the clause); the counts of `conformance/test262/expected.txt` (`story:slice-08-conformance-close`).
