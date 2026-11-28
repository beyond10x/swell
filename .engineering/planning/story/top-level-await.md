---
format: aep.planning-md/3
id: story:top-level-await
kind: story
status: draft
title: 'Top-level await: the asynchronous evaluation of a module graph'
summary: await at a module's top level compiled as an asynchronous body (the activation slot in resolve.rs, async_slot set in lib.rs); AsyncEvaluationOrder, pending dependencies, async parents, ExecuteAsyncModule with its two built-in closures, GatherAvailableAncestors and the settling jobs; lifts the hold top-level await; HOST-99. Order 5; 240 cases.
relations:
- decomposes: epic:slice-11-modules
- depends_on: story:module-linking-and-evaluation
- depends_on: story:modules-design-text
scope:
- confidence: cited
  path: conformance/host/requirements.txt
- confidence: cited
  path: crates/compiler/src/expression.rs
- confidence: cited
  path: crates/compiler/src/lib.rs
- confidence: cited
  path: crates/compiler/src/resolve.rs
- confidence: cited
  path: crates/compiler/src/statement.rs
- confidence: cited
  path: crates/swell/src/builtins/async_generator.rs
- confidence: cited
  path: crates/swell/src/builtins/promise.rs
- confidence: cited
  path: crates/swell/src/builtins/proxy.rs
- confidence: cited
  path: crates/swell/src/interp/async_function.rs
- confidence: inferred
  path: crates/swell/src/module/async_eval.rs
- confidence: inferred
  path: crates/swell/src/module/evaluate.rs
- confidence: cited
  path: crates/swell/src/object/async_builtin.rs
- confidence: cited
  path: crates/swell/src/object/builtin_function.rs
- confidence: cited
  path: crates/swell/src/ops/promise.rs
- confidence: inferred
  path: crates/swell/tests/top_level_await.rs
- confidence: cited
  path: crates/syntax/src/parser/expression.rs
- confidence: cited
  path: crates/syntax/src/parser/mod.rs
- confidence: cited
  path: crates/syntax/src/parser/statement.rs
revision: 10
---
## Context

Slice 11 brings modules (`epic:slice-11-modules`; `docs/design/engine.md`, section 12, item 11; the `feature` line `top-level-await` of the `swell-slices` block). This story brings `await` at the top level of a module and the asynchronous part of module evaluation, and lifts the hold `top-level await` that `story:module-syntax-and-early-errors` adds. After `story:module-linking-and-evaluation` a module graph without top-level `await` evaluates, every module synchronously, and a module with one is answered by the held refusal.

The count. 240 cases (240 runs, all module runs) are this story's (`docs/plan/slice-11/story-top-level-await.cases.txt`): 204 generated cases of `language/module-code/top-level-await/syntax/` (an `await` in each expression and statement position of a module's top level, which runs), 32 with the flag `async` (the order of evaluation across a graph with asynchronous modules, judged on `$DONE`), 3 with a `runtime` negative (`TypeError` 2, `RangeError` 1) and 1 other. Every one fails on the base on "a module run is not made before slice 11".

Design it follows: ECMA-262 16.2.1.6 (`#sec-innermoduleevaluation` with [[HasTLA]], [[AsyncEvaluationOrder]], [[PendingAsyncDependencies]] and [[AsyncParentModules]]; `#sec-execute-async-module`, `#sec-gather-available-ancestors`, `#sec-async-module-execution-fulfilled`, `#sec-async-module-execution-rejected`; `#sec-source-text-module-record-execute-module` with a capability, AsyncBlockStart), `#sec-await`; `host-interface.md` section 9 and HOST-99 (line 1555); `spec/domains/host.yaml`, the transitions `evaluate_async` and `settle` of `swell.host.Module` and the command `AsyncModuleExecutionSettled`; `engine.md` decision 3 (an activation suspended at `await` and resumed by a promise job, as slice 10 runs an `async` function) and the paragraph `**Modules.**` of `story:modules-design-text`, decided: a module body runs in a frame of its own whose ScriptOrModule component is the module, suspended at `await` and resumed as an `async` function's activation is.

## What later stories reuse

The asynchronous evaluation of a graph, which `story:dynamic-import` reaches through `evaluate_module`'s promise (an `import()` of a module with top-level `await` settles after it), and the command `AsyncModuleExecutionSettled` of the model, which `story:ess-target-module-commands` maps.

## Domain relations

- Module → its asynchronous parent modules ([[AsyncParentModules]]): many to many, each a module that waits for this one; module → its pending asynchronous dependencies (a count) — inferable (inferred from ECMA-262 `#sec-cyclic-module-records`, Table 46; no ess/1 document declares the relation: `swell.host.Module` in `spec/domains/host.yaml` holds the states `EvaluatingAsync` and `Evaluated` and the transition `settle`, no relation between modules).

## Change

Failing tests first: the tests below are written before the code and fail on the base.

- `swell-syntax`: the hold `top-level await` goes: its `hold` calls where `story:module-syntax-and-early-errors` placed them, `crates/syntax/src/parser/expression.rs` in `prefix_operators` (line 771 on the base) at the `Prefix::Await` of `prefix` (line 742), and `crates/syntax/src/parser/statement.rs` at a top-level `for await`, and its line of `HOLDS` (`crates/syntax/src/parser/mod.rs`, the second line; the attribute `#[rustfmt::skip]` above the array stays). The held source `module-top-level-await.js` of order 1 then parses as a valid module does; this story writes no file of `crates/syntax/tests/`.
- `swell-compiler`: a module whose tree has `top_level_await` (the flag `story:module-syntax-and-early-errors` sets) is compiled as an asynchronous body, with the instructions an `async` function's body uses, and its code says so: this story sets `ModuleCode::async_slot`, [[HasTLA]], which `story:module-code-compiles` declared and its disassembly already prints (`module locals <n> strict tla <slot>`), so this story writes neither `bytecode.rs` nor `disasm.rs`. The hunks: `crates/compiler/src/resolve.rs`, in `resolve_module_with` of `story:module-code-compiles` (after `resolve_with`, which ends at line 699 on the base): when the tree has `top_level_await`, the module's top level reserves the local slot of its activation, the last of its locals, as lines 667 to 673 do for an `async` function; `crates/compiler/src/lib.rs`, in `compile_module_with` of `story:module-code-compiles`: `async_slot` set, and the body emitted as `async_start` and `async_body` of `crates/compiler/src/function.rs` (lines 233 to 262) emit it, `Compiler::asynchronous` set to the slot, `AsyncStart(slot)` first, the items, the return through `exit_return`, and the region of the body a `HandlerKind::Catch` whose block is `AsyncReject(slot)`. The two helpers of `function.rs` are not called and stay private and unchanged: each takes a `&Function` and runs `f.body`, and a module's body is its items, not a function's statements; `function.rs` is not written. The check that admits `await` only in an `async` body (`crates/compiler/src/expression.rs` line 1233, `malformed("an await outside an async function")`) and the one of `for await` (`crates/compiler/src/statement.rs` line 866) admit the top level of a module through `Compiler::asynchronous` set above; their hunks are the two conditions. No new instruction.
- `swell`, `crates/swell/src/module/async_eval.rs` (the file the records story created, with `AsyncState` empty): `AsyncState` gets [[AsyncEvaluationOrder]] (from a counter of the agent), [[PendingAsyncDependencies]] and [[AsyncParentModules]] with their trace ([[HasTLA]] is read from the code's `async_slot`; [[TopLevelCapability]] is the linking story's, in `LinkState`, which this story reads and does not move), and the functions ExecuteAsyncModule (the body run with a capability, suspended at `await`, resumed by a promise job of the queue of slice 10; its onFulfilled and onRejected are the two built-in closures below), AsyncModuleExecutionFulfilled and Rejected, GatherAvailableAncestors (its list sorted by [[AsyncEvaluationOrder]], one step per module visited and per comparison, charged before), the top-level capability settled from the job that settles the cycle root. `crates/swell/src/module/evaluate.rs` (the linking story's): the async branches of InnerModuleEvaluation, which call into `async_eval.rs`; no other story of order 5 writes `evaluate.rs`. `crates/swell/src/interp/async_function.rs` (`await_instruction`, line 116, and `await_function`, line 223): a module body's frame suspended at `await` and resumed by the promise job, as an `async` function's activation is; the frame's realm and its ScriptOrModule component (the records story's) are the module's. The two closures of ExecuteAsyncModule (`#sec-execute-async-module`, steps 3 to 6), built-in functions that hold the module, in `crates/swell/src/object/builtin_function.rs`: a variant `BuiltinId::AsyncModule(ReactionType)` after `WeakSet` (line 139 on the base), as `AsyncBuiltinAwait(ReactionType)` is; a variant `Slots::AsyncModule { module: Handle }` after `Revocable` (line 281), its arm in `Slots::trace` (line 287; it marks the module record) and in `Slots::bytes` (line 307, with the variants that share nothing); the arm in the router `run_builtin` (line 819) to `run_builtin_part_d`, the part that holds eight groups on the base (its documentation: a new group goes into the last part while it holds fewer than nine), the steps in `run_builtin_part_d` (line 1041), and the id in the arm of the other parts' engine fault (`run_builtin_part_a`, `_b` and `_c`, lines 870, 927 and 979). The matches of `Slots` without a wildcard outside that file, eight, each get the variant in their arm of the engine fault (found by `grep -rn 'Slots::AsyncFromSync(_)' crates/swell/src`, the arm every such match has; the matches of `builtins/async_iteration.rs` end in a wildcard): `object/async_builtin.rs` line 348, `interp/async_function.rs` line 236 (in `await_function`, the function this story already changes), `ops/promise.rs` line 266, `builtins/promise.rs` lines 478, 582 and 607, `builtins/async_generator.rs` line 711 and `builtins/proxy.rs` line 136.
- Tests in a new module `crates/swell/tests/top_level_await.rs`, 7, each in both collection modes: `host_99_a_module_with_top_level_await_reads_evaluating_async_then_evaluated`, `a_graph_waits_for_its_async_dependency_before_its_body` (three modules, the order of their bodies printed through a host function and compared), `a_rejected_await_rejects_the_graph_and_every_ancestor`, `two_async_siblings_run_in_async_evaluation_order`, `an_evaluation_after_a_rejection_answers_the_same_error`, `a_termination_inside_an_awaited_body_leaves_the_module_evaluating_async` (and a later checkpoint changes nothing of it; the outcome decision 10 gives a termination of a job), and `gather_available_ancestors_of_many_parents_ends_in_step_budget`.
- Rules (the `AGENTS.md` of the folder that holds the repository, and `AGENTS.md`, "Rules" and "Commands and the gate"): no `unsafe` (`#![forbid(unsafe_code)]`), no third-party crate, no panic, no `unwrap`, `expect` or unchecked indexing in library code; every loop of the change (a walk of a module graph, of a list of entries, of a namespace's names) is charged to the step budget and every allocation to the memory budget, and a new limit gets a test that reaches it; a builtin's module and comments cite its clause. Time ("Tests stay fast"): a single test is terminated at 60 s and counts as failed, and the gate fails when the wall time of its test steps passes 120 s; the unit times each test before it adds it and names in its report every test over 5 s with its time; one integration-test binary per crate (a new test file is a module of the crate's `tests/main.rs`, written again by `sh taskfiles/test-main.sh write`); an expensive product several tests read (a parsed corpus, a fixture tree, a built part of the test262 checkout) is made once, as a committed file or one shared fixture, never per test and never in a `static`; no test makes a network request or a model call. Builds and tests go through the pool: `cargo` is the shim of the pool, anything else that compiles or tests is prefixed `slot`; a pool job ends after 10 minutes unless it names its time with `slot --timeout` or `SLOT_TIMEOUT`, at most 30 minutes; a script that only measures, probes or waits starts without the `slot` prefix. No Python in anything committed (tools, tasks, gates, tests, helpers): Rust or POSIX sh. The unit runs the tests of its own change, never the whole gate and never a whole-workspace test run; intermediate files go in the scratch directory; no `cd ... &&` with a write or a delete.
- Report: the sum of the CPU seconds of the story's new tests in the default mode and in the mode `collect-every-safe-point` (each test run alone, user plus system), and the wall seconds of its part run when it has one; no story bounds the CPU seconds of a whole gate.
- Generated files: the generated `tests/main.rs` of every crate and `website/data/status.json` are in no story's scope. A unit that adds a test module writes `tests/main.rs` again with `sh taskfiles/test-main.sh write`; a unit that meets a conflict in a generated file takes either side and writes it again; the coordinator writes both again on the join (`sh taskfiles/test-main.sh write`, `slot --local task site-data`).
- Line numbers are those of `f962c324`; anchors are found by name.
- Size: about 1,100 lines (the async evaluation 400, the compiler, the interpreter and the two closures with their arms 250, the tests 450): one unit.

Shared with the other stories of order 5 (`story:module-namespace-objects`, `story:import-meta`, `story:json-modules`), each in its own hunks: `crates/syntax/src/parser/mod.rs`, `HOLDS` (this story deletes the second line, `story:module-namespace-objects` the first: adjacent lines, so the branch merged second takes both deletions; `story:import-meta` deletes the fourth); `crates/syntax/src/parser/expression.rs` (this story: the hold in `prefix_operators`, near line 771; `story:import-meta`: its hold in the hunk of lines 1222 to 1230); `crates/compiler/src/expression.rs` (this story: the `await` check near line 1233; `story:import-meta`: the arm `Expr::ImportMeta` near line 342); `crates/compiler/src/lib.rs` (this story: `compile_module_with`; `story:import-meta`: none of `lib.rs`); `crates/compiler/src/resolve.rs` (this story: the slot in `resolve_module_with`, placed after line 699 of the base; `story:import-meta`: the arm `Expr::ImportMeta` beside line 1746, more than a thousand unchanged lines away, so no edge is needed); `crates/swell/src/interp/` (this story: `async_function.rs` only; `story:import-meta`: one arm in `dispatch.rs` and its function); `crates/swell/src/object/` (this story: `builtin_function.rs` and `async_builtin.rs`; `story:module-namespace-objects`: `mod.rs`, `module_namespace.rs`, `array.rs`, `ordinary.rs`; no other story of order 5 writes `builtin_function.rs`); `crates/swell/src/ops/promise.rs` and `crates/swell/src/builtins/{promise,async_generator,proxy}.rs`, this story's alone in order 5; `conformance/host/requirements.txt` (this story: line 99; `story:import-meta`: lines 68, 158 and the line of HOST-224; `story:json-modules`: line 64; no two stories of the order write adjacent lines). This story only in order 5: `crates/syntax/src/parser/statement.rs`, `crates/compiler/src/statement.rs`, `crates/swell/src/module/{async_eval,evaluate}.rs`. In order 6 `story:dynamic-import` writes `object/builtin_function.rs` and the eight matches of `Slots` again, for its own two closures, after this story (it depends on it). Shared with stories outside the slice, whose edges the coordinator sets: `object/builtin_function.rs` with `bigint-values-and-literals` and `bigint-constructor-and-prototype` of slice 12 and with the backlog draft `promise-closure-boxes-are-charged-in-a-library-test`.

## Requirements

HOST-99 (`host-interface.md` line 1555): line 99 of `conformance/host/requirements.txt` moves from `pending slice 11` to `HOST-99 test swell::top_level_await::host_99_a_module_with_top_level_await_reads_evaluating_async_then_evaluated`. For the coordinator: the line `HOST-99` under `### Host interface` of `## [Unreleased]`; no new public name. The coordinator writes `website/data/status.json` again on the join.

Writes: `crates/syntax/src/parser/{mod,expression,statement}.rs` (the hold), `crates/compiler/src/{lib,resolve,expression,statement}.rs` (the hunks above), `crates/swell/src/module/{async_eval,evaluate}.rs`, `crates/swell/src/interp/async_function.rs`, `crates/swell/src/object/{builtin_function,async_builtin}.rs`, `crates/swell/src/ops/promise.rs`, `crates/swell/src/builtins/{promise,async_generator,proxy}.rs` (the arms above), `crates/swell/tests/top_level_await.rs` (new), `conformance/host/requirements.txt` (line 99). Not written: `crates/syntax/tests/`, `crates/syntax/src/ast.rs` (the flag is order 1's), `crates/compiler/src/{bytecode,disasm,function}.rs` (the field and its line are order 2's; the helpers are not called), `crates/test262/` (the module run of the linking story judges `async` cases already), the other files of `crates/swell/src/object/`, the other files of `crates/swell/src/module/`, `spec/`, `docs/design/`.

Order 5, beside `story:module-namespace-objects`, `story:import-meta` and `story:json-modules`; it depends on `story:module-linking-and-evaluation` and `story:modules-design-text`.

## Acceptance

`cargo nextest run --locked -p swell --test main -E 'test(/^top_level_await::/)'` runs 7 tests (none on the base: the module does not exist) and exits 0 in both collection modes; `cargo nextest run --locked -p swell --test main -E 'test(/^modules::/) | test(/^async_functions::/) | test(/^promises::/) | test(/^jobs::/)'` exits 0 in both collection modes; `cargo nextest run --locked -p swell-syntax --test main`, `cargo nextest run --locked -p swell-compiler --test main` and `cargo nextest run --locked -p swell --lib` exit 0. `grep -c '"top-level await"' crates/syntax/src/parser/mod.rs` prints `0` (`1` on the join the unit starts from); `grep -c '^HOST-99 pending slice 11$' conformance/host/requirements.txt` prints `0` (`1` on that join) and `grep -c '^HOST-99 test swell::top_level_await::' conformance/host/requirements.txt` prints `1`; `slot --timeout 30m task -d . -t taskfiles/host-requirements.yml host-requirements-check` exits 0. `slot task clippy` exits 0.

test262. The full run is not the unit's. In one pool job on the build node: `slot --timeout 30m --fetch .out/test262-result sh docs/plan/slice-11/run-part.sh docs/plan/slice-11/story-top-level-await.cases.txt docs/plan/slice-11/guard-async.cases.txt`; then on the workstation, with the base run of the join the unit starts from copied to `.out/test262-base/runs.tsv`, `sh docs/plan/slice-11/check.sh docs/plan/slice-11/story-top-level-await.cases.txt .out/test262-base/runs.tsv .out/test262-result/runs.tsv` exits 0 and prints `passed 538 bound 538 B 298 F 240 R 0` (`B` counts every case of the part that passes on the base run read, and the bound is `B + F - R`; the part holds 538 cases and pulls in none by `./`: the 240 of the list, all failing, and the 298 of `guard-async.cases.txt`, all passing; the line is the same on `f962c324` and on the join).

## Out of scope

`import()` of a module with top-level `await` (`story:dynamic-import`); the ESS target's `AsyncModuleExecutionSettled` (`story:ess-target-module-commands`); `Atomics.waitAsync` and other jobs (slice 14).
