---
format: aep.planning-md/3
id: story:dynamic-import
kind: story
status: draft
title: 'import(): a load from a script, a module or a realm, through linking and evaluation to the namespace'
summary: Instruction::ImportCall, EvaluateImportCall and ContinueDynamicImport; the referrer of a load from the active script or module or the realm; LoadedModules of scripts and realms; the runner resolves script referrers; lifts the last hold; HOST-55, 65, 66. Order 6; 485 cases.
relations:
- decomposes: epic:slice-11-modules
- depends_on: story:module-namespace-objects
- depends_on: story:top-level-await
- depends_on: story:import-meta
- depends_on: story:json-modules
scope:
- confidence: cited
  path: conformance/host/requirements.txt
- confidence: cited
  path: crates/compiler/src/bytecode.rs
- confidence: cited
  path: crates/compiler/src/disasm.rs
- confidence: cited
  path: crates/compiler/src/expression.rs
- confidence: cited
  path: crates/compiler/src/resolve.rs
- confidence: inferred
  path: crates/compiler/tests/sources/import-call-with-options.disasm
- confidence: inferred
  path: crates/compiler/tests/sources/import-call-with-options.js
- confidence: inferred
  path: crates/compiler/tests/sources/import-call.disasm
- confidence: inferred
  path: crates/compiler/tests/sources/import-call.js
- confidence: cited
  path: crates/swell/src/builtins/async_generator.rs
- confidence: cited
  path: crates/swell/src/builtins/promise.rs
- confidence: cited
  path: crates/swell/src/builtins/proxy.rs
- confidence: cited
  path: crates/swell/src/interp/async_function.rs
- confidence: cited
  path: crates/swell/src/interp/dispatch.rs
- confidence: inferred
  path: crates/swell/src/module/dynamic.rs
- confidence: inferred
  path: crates/swell/src/module/loading.rs
- confidence: cited
  path: crates/swell/src/object/async_builtin.rs
- confidence: cited
  path: crates/swell/src/object/builtin_function.rs
- confidence: cited
  path: crates/swell/src/ops/promise.rs
- confidence: cited
  path: crates/swell/src/realm/mod.rs
- confidence: cited
  path: crates/swell/src/script.rs
- confidence: inferred
  path: crates/swell/tests/dynamic_import.rs
- confidence: cited
  path: crates/syntax/src/parser/expression.rs
- confidence: cited
  path: crates/syntax/src/parser/mod.rs
- confidence: cited
  path: crates/test262/src/host.rs
- confidence: inferred
  path: crates/test262/tests/fixtures/modules-dynamic
- confidence: cited
  path: crates/test262/tests/run.rs
revision: 10
---
## Context

Slice 11 brings modules (`epic:slice-11-modules`; `docs/design/engine.md`, section 12, item 11; the `feature` lines `dynamic-import` and `import-attributes`, and the directory line `dir test/language/expressions/dynamic-import/ 11`). This story evaluates `import()`: a load requested from a script, a module or a realm, with the promise of the call as its payload, through loading, linking and evaluation to the module's namespace; and it lifts the hold `dynamic import` that `story:import-call-and-import-meta-syntax` adds. On the base (`f962c324`) a script records no loaded modules ([[LoadedModules]] "comes with modules (slice 11)", `crates/swell/src/script.rs` line 101), and `LoadRequest` has a referrer `Referrer::Script` or `Referrer::Realm` that nothing makes (`crates/swell/src/host/hooks.rs` lines 53 to 60). The ScriptOrModule component of frames, functions, environments and jobs as a handle of two kinds, and the payload of a `LoadRequest` as a sum with the variant `Dynamic`, are `story:module-records-loading-and-module-runs`'s; this story reads them and keeps the referrer logic only.

The count. 485 cases (895 runs) are this story's (`docs/plan/slice-11/story-dynamic-import.cases.txt`): 252 generated `async` cases and 95 generated others of `expressions/dynamic-import/` (the call in every syntactic position, its specifier and options evaluated in order, the namespace it fulfils with, a rejection for each error of loading, linking and evaluation, the `with` options of `import-attributes`), 52 other `async` cases, 32 module cases with the flag `async` (an `import()` inside a module, of a module with top-level `await`, of a JSON module), `language/import/import-attributes/json-via-namespace.js` (a static namespace import of a JSON module: it needs the namespace and the JSON parts both, and this story is the first whose dependencies hold both), and the rest. On the base run they fail on the refusal `dynamic import is not supported before slice 11` (847 runs), on "a module run is not made before slice 11" (46) and on a counted refusal caught by `eval` (2); after order 2 they fail on the held refusal. The story closes the slice's last engine part: its scan key holds every other part (`docs/plan/slice-11/scan.awk`, key `dimp`), so it depends on each story of order 5.

Design it follows: ECMA-262 13.3.10.1 (`#sec-import-call-runtime-semantics-evaluation`, EvaluateImportCall: the specifier and the options evaluated, the `with` object read with its own errors (each a rejected promise, never a throw), the attributes sorted and checked against HostGetSupportedImportAttributes), `#sec-ContinueDynamicImport`, `#sec-HostLoadImportedModule` (Note 1: a realm referrer when there is no active script or module), `#sec-getactivescriptormodule`, `#sec-FinishLoadingImportedModule` for a script referrer; `host-interface.md` section 9 (`Referrer`, `LoadRequest::is_dynamic`, `host_defined()` `None` for `import()`), HOST-55 (line 883), HOST-65 and HOST-66 (lines 969 and 971); `engine.md` decision 3 (line 2294 on the base: a job keeps the script or module active when it was queued), decision 11, "Modules", and the paragraph `**Modules.**` (the `LoadRequest` payload and the finish's choice).

## What later stories reuse

Nothing in this slice but the close. Slice 12 (`epic:slice-12-bigint`, which depends on this epic) has 4 cases that need `import()` with an options argument (`expressions/dynamic-import/import-attributes/2nd-param-non-object.js`, `2nd-param-with-non-object.js`, `2nd-param-with-undefined.js`, `2nd-param-with-value-non-string.js`, counted in slice 12 for their BigInt values), which pass once this story and BigInt have landed.

## Domain relations

- A module load requested by `import()` → its referrer: many to one, the active script, the active module, or the current realm when neither is active — inferable (inferred from `crates/swell/src/host/hooks.rs` line 53, `pub enum Referrer { Script(ScriptRef), Module(ModuleRef), Realm(RealmRef) }`, and ECMA-262 `#sec-HostLoadImportedModule` Note 1; the model marks it `UNMAPPED`: `spec/domains/host.yaml` line 296, entity `swell.host.ModuleLoad`, which states only a module referrer, carried by HOST-65 and HOST-66).
- A script → the modules loaded for its requests ([[LoadedModules]]): one to many, set once per (specifier, attributes) — inferable (inferred from ECMA-262 `#sec-script-records`, Table 40, and `#sec-FinishLoadingImportedModule`; no ess/1 document declares it: `swell.host.Script` in `spec/domains/host.yaml` has no relation to a module).
- A realm → the modules loaded for an `import()` with no active script or module ([[LoadedModules]] of the Realm Record): one to many — inferable (inferred from ECMA-262 `#sec-code-realms`, Table 24; no ess/1 document declares it).

## Change

Failing tests first: the tests below are written before the code and fail on the base.

- `swell-syntax`: the hold `dynamic import` goes: its `hold` call in `crates/syntax/src/parser/expression.rs` (the hunk of lines 1222 to 1230 that order 2 wrote) and its line of `HOLDS` (`crates/syntax/src/parser/mod.rs`), which is then empty again (the attribute `#[rustfmt::skip]` above the array and its line of documentation stay, for the holds of later slices); `HOLDS` and `Parser::hold` stay for the holds of later slices, as slice 8 left them. The held sources `dynamic-import.js` and `dynamic-import-options.js` of order 2 then parse as valid sources do; this story writes no file of `crates/syntax/tests/`.
- `swell-compiler`: `Instruction::ImportCall { options: bool }` after `Instruction::ImportMeta` of `story:import-meta` (`crates/compiler/src/bytecode.rs`), its line in `disasm.rs`, and the arms of `Expr::ImportCall` in `crates/compiler/src/expression.rs` (beside line 342) and `resolve.rs` (beside line 1746), left answering `CompileError::Tree` by order 2, compile the specifier, then the options, then the instruction; 2 new sources `crates/compiler/tests/sources/import-call.js` and `import-call-with-options.js` with their `.disasm`.
- `swell`: the arm of `Instruction::ImportCall` in `crates/swell/src/interp/dispatch.rs` (after that of `ImportMeta`): EvaluateImportCall (a promise capability first; the specifier's `ToString` and every error of reading the options reject it; the attribute keys checked against `SUPPORTED_IMPORT_ATTRIBUTES`, an unsupported key a rejection with a `SyntaxError` and no hook call), then `host_load_imported_module` with a `LoadRequest` whose referrer is the active script or module (GetActiveScriptOrModule: the ScriptOrModule component of the running frame, or of the job when no frame runs, `engine.md` line 2294), or the current realm when neither is active, whose payload is `LoadPayload::Dynamic` with the capability, `is_dynamic()` true and `host_defined()` `None`. `crates/swell/src/module/loading.rs`, three hunks: the `#[expect(dead_code, reason = "story:dynamic-import")]` of the variant `Dynamic` goes, and the two arms `Referrer::Script` and `Referrer::Realm` of the dispatch on the referrer's kind that the records story left answering `Terminated(EngineFault)` (FinishLoadingImportedModule step 1) keep the module in the script's or the realm's [[LoadedModules]] below, with the check of HOST-63 that the module arm has. `crates/swell/src/module/dynamic.rs` (the records story's, whose `continue_dynamic_import` answered an engine fault): ContinueDynamicImport: a finish with a thrown value rejects; with a module, `load_requested_modules` of it, then `link`, then `evaluate_module`, and the promise fulfilled with GetModuleNamespace of `story:module-namespace-objects`, each failure a rejection. A script or realm referrer keeps its loaded modules: `ScriptRecord` (`crates/swell/src/script.rs` line 103) gets [[LoadedModules]], traced in its `trace` (line 114), with the field in each of its three literals (lines 146, 509 and 576); `RealmRecord` (`crates/swell/src/realm/mod.rs` line 119) the same, in its trace and its three literals (lines 148, 192 and 344); a second `import()` of one (specifier, attributes) from one referrer reuses its module. The closures of ContinueDynamicImport (`#sec-ContinueDynamicImport`, steps 4 to 8: rejectedClosure, linkAndEvaluateClosure and the fulfilledClosure it makes), built-in functions that hold the module and the promise capability, in `crates/swell/src/object/builtin_function.rs`, after those of `story:top-level-await`: a variant `BuiltinId::DynamicImport(DynamicImportStep)` after `AsyncModule` (`DynamicImportStep` in `module/dynamic.rs`: `LinkAndEvaluate`, `Fulfilled`, `Rejected`); a variant `Slots::DynamicImport { module: Handle, promise: ObjectRef, resolve: Value, reject: Value }` after `AsyncModule`, its arms in `Slots::trace` and `Slots::bytes`; since `run_builtin_part_d` holds nine groups once `story:top-level-await` added its own, the group goes into a new part `run_builtin_part_e` with an arm of its own in the router `run_builtin`, by the rule of its documentation (lines 796 to 810 on the base), whose arm of the engine fault lists every other id; and the id joins the arms of the engine fault of parts a to d. The eight matches of `Slots` without a wildcard outside that file (`grep -rn 'Slots::AsyncFromSync(_)' crates/swell/src`: `object/async_builtin.rs`, `interp/async_function.rs`, `ops/promise.rs`, `builtins/promise.rs` three, `builtins/async_generator.rs`, `builtins/proxy.rs`) each get the variant in their arm of the engine fault, beside the `AsyncModule` that `story:top-level-await` put there.
- `swell-test262`, `crates/test262/src/host.rs`: the loader resolves the specifier of a load with a script referrer against the directory of the case (a harness script's `import()` against the case's directory as well) and with a realm referrer the same; `crates/test262/src/run.rs`: none (the checkpoint and the async judgement exist). A fixture tree of this story's own, `crates/test262/tests/fixtures/modules-dynamic/` (new; `harness/assert.js`, `harness/sta.js` and `harness/doneprintHandle.js` as `fixtures/async/` has them), with 1 case `test/language/expressions/dynamic-import/imports.js` (flag `async`, `import("./imports_FIXTURE.js").then(ns => ...).then($DONE, $DONE)`; passes) and its fixture; 1 new test in `crates/test262/tests/run.rs`: `a_script_imports_a_module`.
- Tests in a new module `crates/swell/tests/dynamic_import.rs`, 10, each in both collection modes: `host_55_host_defined_is_the_same_rc_and_the_referrer_is_that_script_or_module`, `host_65_import_in_a_script_has_a_script_referrer_and_no_host_defined`, `host_66_import_from_a_host_function_without_a_script_has_a_realm_referrer`, `an_options_object_that_throws_rejects_and_calls_no_hook`, `an_unsupported_attribute_rejects_with_a_syntax_error`, `import_of_a_module_that_throws_rejects_with_its_error`, `two_imports_of_one_specifier_share_one_module`, `import_inside_a_module_has_the_module_as_referrer`, `import_of_a_module_with_top_level_await_settles_after_it`, `a_dropped_request_of_import_leaves_its_promise_pending`.
- Rules (the `AGENTS.md` of the folder that holds the repository, and `AGENTS.md`, "Rules" and "Commands and the gate"): no `unsafe` (`#![forbid(unsafe_code)]`), no third-party crate, no panic, no `unwrap`, `expect` or unchecked indexing in library code; every loop of the change (a walk of a module graph, of a list of entries, of a namespace's names) is charged to the step budget and every allocation to the memory budget, and a new limit gets a test that reaches it; a builtin's module and comments cite its clause. Time ("Tests stay fast"): a single test is terminated at 60 s and counts as failed, and the gate fails when the wall time of its test steps passes 120 s; the unit times each test before it adds it and names in its report every test over 5 s with its time; one integration-test binary per crate (a new test file is a module of the crate's `tests/main.rs`, written again by `sh taskfiles/test-main.sh write`); an expensive product several tests read (a parsed corpus, a fixture tree, a built part of the test262 checkout) is made once, as a committed file or one shared fixture, never per test and never in a `static`; no test makes a network request or a model call. Builds and tests go through the pool: `cargo` is the shim of the pool, anything else that compiles or tests is prefixed `slot`; a pool job ends after 10 minutes unless it names its time with `slot --timeout` or `SLOT_TIMEOUT`, at most 30 minutes; a script that only measures, probes or waits starts without the `slot` prefix. No Python in anything committed (tools, tasks, gates, tests, helpers): Rust or POSIX sh. The unit runs the tests of its own change, never the whole gate and never a whole-workspace test run; intermediate files go in the scratch directory; no `cd ... &&` with a write or a delete.
- Report: the sum of the CPU seconds of the story's new tests in the default mode and in the mode `collect-every-safe-point` (each test run alone, user plus system), and the wall seconds of its part run when it has one; no story bounds the CPU seconds of a whole gate.
- Generated files: the generated `tests/main.rs` of every crate and `website/data/status.json` are in no story's scope. A unit that adds a test module writes `tests/main.rs` again with `sh taskfiles/test-main.sh write`; a unit that meets a conflict in a generated file takes either side and writes it again; the coordinator writes both again on the join (`sh taskfiles/test-main.sh write`, `slot --local task site-data`).
- Line numbers are those of `f962c324`; anchors are found by name.
- Size: about 1,100 lines (the instruction, EvaluateImportCall and ContinueDynamicImport 400, its closures and their arms 100, the referrer and the loaded modules of a script or realm 100, the runner 50, the tests and fixtures 450): one unit.

Shared with the other stories of order 6 (`story:ess-target-module-commands`, `story:module-determinism-and-emulated-loading`): `conformance/host/requirements.txt` (this story: lines 55, 65 and 66; `story:module-determinism-and-emulated-loading` line 159); no other file. `object/builtin_function.rs` and the eight matches of `Slots` were written by `story:top-level-await` in order 5, which this story follows. Shared with stories outside the slice, whose edges the coordinator sets: `object/builtin_function.rs` with `bigint-values-and-literals` and `bigint-constructor-and-prototype` of slice 12 and with the backlog draft `promise-closure-boxes-are-charged-in-a-library-test`.

## Requirements

HOST-55, HOST-65 and HOST-66: their 3 lines of `conformance/host/requirements.txt` move from `pending slice 11` to `HOST-<n> test swell::dynamic_import::host_<n>_...`. For the coordinator: the three lines under `### Host interface` of `## [Unreleased]`; no new public name (`Referrer`, `LoadRequest::is_dynamic` and `host_defined` exist). The coordinator writes `website/data/status.json` again on the join.

Writes: `crates/syntax/src/parser/{mod,expression}.rs` (the hold), `crates/compiler/src/{bytecode,disasm,expression,resolve}.rs`, `crates/compiler/tests/sources/import-call{,-with-options}.{js,disasm}` (new), `crates/swell/src/interp/dispatch.rs` (one arm), `crates/swell/src/module/{dynamic,loading}.rs` (the hunks above), `crates/swell/src/script.rs`, `crates/swell/src/realm/mod.rs` ([[LoadedModules]] of a script and of a realm, the traces and the six literals), `crates/swell/src/object/{builtin_function,async_builtin}.rs`, `crates/swell/src/interp/async_function.rs`, `crates/swell/src/ops/promise.rs`, `crates/swell/src/builtins/{promise,async_generator,proxy}.rs` (the closures and their arms), `crates/swell/tests/dynamic_import.rs` (new), `crates/test262/src/host.rs`, `crates/test262/tests/run.rs`, `crates/test262/tests/fixtures/modules-dynamic/` (new), `conformance/host/requirements.txt` (3 lines). Not written: `crates/syntax/tests/`, `crates/swell/src/interp/frame.rs`, `crates/swell/src/host/jobs.rs` (the records story's), `spec/`, `conformance/ess/`, `docs/design/`, `CHANGELOG.md`.

Order 6, beside `story:ess-target-module-commands` and `story:module-determinism-and-emulated-loading`; `story:cli-module-loader` (order 7) depends on it; it depends on `story:module-namespace-objects`, `story:top-level-await`, `story:import-meta` and `story:json-modules` (its cases use each part; its instruction comes after that of `import.meta`).

## Acceptance

`cargo nextest run --locked -p swell --test main -E 'test(/^dynamic_import::/)'` runs 10 tests (none on the base: the module does not exist) and exits 0 in both collection modes; `cargo nextest run --locked -p swell --test main -E 'test(/^modules::/) | test(/^top_level_await::/) | test(/^json_modules::/) | test(/^jobs::/) | test(/^promises::/) | test(/^eval::/)'` exits 0 in both collection modes; `cargo nextest run --locked -p swell-syntax --test main`, `cargo nextest run --locked -p swell-compiler --test main` and `cargo nextest run --locked -p swell --lib` exit 0; `cargo nextest run --locked -p swell-test262 --test main` exits 0 and `-E 'test(/^run::/)'` runs 34 tests (33 on the join the unit starts from, 28 on the base); `find crates/test262/tests/fixtures/modules-dynamic/test -name '*.js' ! -name '*_FIXTURE*' | wc -l` prints `1` (`0` before); `ls crates/compiler/tests/sources/import-call.js crates/compiler/tests/sources/import-call.disasm crates/compiler/tests/sources/import-call-with-options.js crates/compiler/tests/sources/import-call-with-options.disasm | wc -l` prints `4` and `ls crates/compiler/tests/sources | wc -l` prints `292` (`288` on that join, `268` on the base). `grep -c '", 11)' crates/syntax/src/parser/mod.rs` prints `0` (`HOLDS` is empty; `1` on that join); `grep -cE '^HOST-(55|65|66) pending slice 11$' conformance/host/requirements.txt` prints `0` (`3` on that join) and `grep -cE '^HOST-(55|65|66) test swell::dynamic_import::' conformance/host/requirements.txt` prints `3`; `slot --timeout 30m task -d . -t taskfiles/host-requirements.yml host-requirements-check` exits 0. `slot task clippy` exits 0.

test262. The full run is not the unit's. In one pool job on the build node: `slot --timeout 30m --fetch .out/test262-result sh docs/plan/slice-11/run-part.sh docs/plan/slice-11/story-dynamic-import.cases.txt docs/plan/slice-11/guard-async.cases.txt`; then on the workstation, with the base run of the join the unit starts from copied to `.out/test262-base/runs.tsv`, `sh docs/plan/slice-11/check.sh docs/plan/slice-11/story-dynamic-import.cases.txt .out/test262-base/runs.tsv .out/test262-result/runs.tsv` exits 0 and prints `passed <P> bound 794 B 309 F 485 R 0` with `<P>` at least 794. The formula: `B` counts every case of the part that passes on the base run read, and the bound is `B + F - R`; the part holds 794 cases: the 485 of the list, all failing; the 298 of `guard-async.cases.txt`, all passing; and 11 cases of other lists that a file of the list names by `./` (9 of `story:module-linking-and-evaluation`, 2 of `story:module-namespace-objects`, `instn-star-equality.js` and `instn-star-iee-cycle.js`), whose owners have landed on this story's join, so they count in `B` (298 + 11 = 309). Over the base run of `f962c324` the same part reads `bound 783 B 298`.

## Out of scope

`import.source` and `import.defer` (stage 3, excluded); the 4 slice-12 cases above (slice 12's; they need BigInt); the ESS target's `HostLoadImportedModule` with a script referrer, which the model does not state (`spec/domains/host.yaml` line 296; `story:ess-target-module-commands`); `import()` in the command-line host (`story:cli-module-loader` names the entry of `docs/plan/waves.md` that takes it).
