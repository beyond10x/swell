---
format: aep.planning-md/3
id: story:dynamic-import
kind: story
status: draft
title: 'import(): a load from a script, a module or a realm, through linking and evaluation to the namespace'
summary: Instruction::ImportCall, EvaluateImportCall and ContinueDynamicImport; the active script or module of frames and jobs; LoadedModules of scripts and realms; the runner resolves script referrers; lifts the last hold; HOST-55, 65, 66. Order 6; 484 cases.
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
  path: crates/swell/src/host/jobs.rs
- confidence: cited
  path: crates/swell/src/interp/dispatch.rs
- confidence: cited
  path: crates/swell/src/interp/frame.rs
- confidence: inferred
  path: crates/swell/src/module.rs
- confidence: cited
  path: crates/swell/src/realm/mod.rs
- confidence: cited
  path: crates/swell/src/script.rs
- confidence: inferred
  path: crates/swell/tests/dynamic_import.rs
- confidence: cited
  path: crates/swell/tests/main.rs
- confidence: cited
  path: crates/syntax/src/parser/expression.rs
- confidence: cited
  path: crates/syntax/src/parser/mod.rs
- confidence: cited
  path: crates/syntax/src/parser/statement.rs
- confidence: inferred
  path: crates/syntax/tests/import_call.rs
- confidence: inferred
  path: crates/syntax/tests/sources/held/dynamic-import.js
- confidence: inferred
  path: crates/syntax/tests/sources/valid/dynamic-import.js
- confidence: cited
  path: crates/test262/src/host.rs
- confidence: inferred
  path: crates/test262/tests/fixtures/modules/test/language/expressions/dynamic-import
- confidence: cited
  path: crates/test262/tests/run.rs
- confidence: cited
  path: website/data/status.json
revision: 3
---
## Context

Slice 11 brings modules (`epic:slice-11-modules`; `docs/design/engine.md`, section 12, item 11; the `feature` lines `dynamic-import` and `import-attributes`, and the directory line `dir test/language/expressions/dynamic-import/ 11`). This story evaluates `import()`: a load requested from a script, a module or a realm, with the promise of the call as its payload, through loading, linking and evaluation to the module's namespace; and it lifts the hold `dynamic import` that `story:import-call-and-import-meta-syntax` adds. On the base (`f962c324`) a script records no loaded modules ([[LoadedModules]] "comes with modules (slice 11)", `crates/swell/src/script.rs` line 101), a frame knows its script only (`crates/swell/src/interp/frame.rs` line 175, `script: Handle`), and `LoadRequest` has a referrer `Referrer::Script` or `Referrer::Realm` that nothing makes (`crates/swell/src/host/hooks.rs` lines 53 to 60).

The count. 484 cases (894 runs) are this story's (`docs/plan/slice-11/story-dynamic-import.cases.txt`): 252 generated `async` cases and 95 generated others of `expressions/dynamic-import/` (the call in every syntactic position, its specifier and options evaluated in order, the namespace it fulfils with, a rejection for each error of loading, linking and evaluation, the `with` options of `import-attributes`), 52 other `async` cases, 32 module cases with the flag `async` (an `import()` inside a module, of a module with top-level `await`, of a JSON module), and the rest. On the base run they fail on the refusal `dynamic import is not supported before slice 11` (847 runs), on "a module run is not made before slice 11" (45) and on a counted refusal caught by `eval` (2); after order 2 they fail on the held refusal. The story closes the slice's last part: its scan key holds every other part (`docs/plan/slice-11/scan.awk`, key `dimp`), so it depends on each story of order 5.

Design it follows: ECMA-262 13.3.10.1 (`#sec-import-call-runtime-semantics-evaluation`, EvaluateImportCall: the specifier and the options evaluated, the `with` object read with its own errors (each a rejected promise, never a throw), the attributes sorted and checked against HostGetSupportedImportAttributes), `#sec-ContinueDynamicImport`, `#sec-HostLoadImportedModule` (Note 1: a realm referrer when there is no active script or module), `#sec-getactivescriptormodule`, `#sec-FinishLoadingImportedModule` for a script referrer; `host-interface.md` section 9 (`Referrer`, `LoadRequest::is_dynamic`, `host_defined()` `None` for `import()`), HOST-55 (line 883), HOST-65 and HOST-66 (lines 969 and 971); `engine.md` decision 3 (line 2294 on the base: a job keeps the script or module active when it was queued), decision 11, "Modules".

## What later stories reuse

Nothing in this slice but the close. Slice 12 (`epic:slice-12-bigint`, which depends on this epic) has 4 cases that need `import()` with an options argument (`expressions/dynamic-import/import-attributes/2nd-param-non-object.js`, `2nd-param-with-non-object.js`, `2nd-param-with-undefined.js`, `2nd-param-with-value-non-string.js`, counted in slice 12 for their BigInt values), which pass once this story and BigInt have landed.

## Domain relations

- A module load requested by `import()` → its referrer: many to one, the active script, the active module, or the current realm when neither is active — inferable (inferred from `crates/swell/src/host/hooks.rs` line 53, `pub enum Referrer { Script(ScriptRef), Module(ModuleRef), Realm(RealmRef) }`, and ECMA-262 `#sec-HostLoadImportedModule` Note 1; the model marks it `UNMAPPED`: `spec/domains/host.yaml` line 296, entity `swell.host.ModuleLoad`, which states only a module referrer, carried by HOST-65 and HOST-66).
- A script → the modules loaded for its requests ([[LoadedModules]]): one to many, set once per (specifier, attributes) — inferable (inferred from ECMA-262 `#sec-script-records`, Table 40, and `#sec-FinishLoadingImportedModule`; no ess/1 document declares it: `swell.host.Script` in `spec/domains/host.yaml` has no relation to a module).
- A realm → the modules loaded for an `import()` with no active script or module ([[LoadedModules]] of the Realm Record): one to many — inferable (inferred from ECMA-262 `#sec-code-realms`, Table 24; no ess/1 document declares it).

## Change

Failing tests first: the tests below are written before the code and fail on the base.

- `swell-syntax`: the hold `dynamic import` goes (its `hold` calls in `crates/syntax/src/parser/expression.rs` and `statement.rs` and its line of `HOLDS`, which is then empty again; `HOLDS` and `Parser::hold` stay for the holds of later slices, as slice 8 left them); `crates/syntax/tests/sources/held/dynamic-import.js` moves to `crates/syntax/tests/sources/valid/dynamic-import.js` with the header of a valid source; `crates/syntax/tests/import_call.rs`: `import_call_with_options_and_trailing_comma_is_held` becomes `..._parses`.
- `swell-compiler`: `Instruction::ImportCall { options: bool }` after `Instruction::ImportMeta` of `story:import-meta` (`crates/compiler/src/bytecode.rs`), its line in `disasm.rs`, and the arms of `Expr::ImportCall` in `crates/compiler/src/expression.rs` and `resolve.rs` (left answering `CompileError::Tree` by order 2) compile the specifier, then the options, then the instruction; sources `crates/compiler/tests/sources/import-call.js` and `import-call-with-options.js` with their `.disasm`.
- `swell`: the arm of `Instruction::ImportCall` in `crates/swell/src/interp/dispatch.rs` (after that of `ImportMeta`): EvaluateImportCall (a promise capability first; the specifier's `ToString` and every error of reading the options reject it; the attribute keys checked against `SUPPORTED_IMPORT_ATTRIBUTES`, an unsupported key a rejection with a `SyntaxError` and no hook call), then `host_load_imported_module` with a `LoadRequest` whose referrer is the active script or module, or the current realm when neither is active (GetActiveScriptOrModule over the frames; a frame of a module's body knows its module: the frame's `script: Handle` becomes a script-or-module handle, `frame.rs` line 175, and a promise job keeps the one active when it was queued, `engine.md` line 2294), whose payload is the capability, `is_dynamic()` true and `host_defined()` `None`. `crates/swell/src/module.rs`: ContinueDynamicImport (after the functions of JSON modules): a finish with a thrown value rejects; with a module, `load_requested_modules` of it, then `link`, then `evaluate_module`, and the promise fulfilled with GetModuleNamespace of `story:module-namespace-objects`, each failure a rejection; a script or realm referrer keeps its loaded modules (`ScriptRecord` gets [[LoadedModules]], `script.rs` line 101; the realm record its own), traced, and a second `import()` of one (specifier, attributes) from one referrer reuses its module.
- `swell-test262`, `crates/test262/src/host.rs`: the loader resolves the specifier of a load with a script referrer against the directory of the case (a harness script's `import()` against the case's directory as well) and with a realm referrer the same; `crates/test262/src/run.rs`: none (the checkpoint and the async judgement exist). A fixture case `crates/test262/tests/fixtures/modules/test/language/expressions/dynamic-import/imports.js` (flag `async`, `import("./imports_FIXTURE.js").then(ns => ...).then($DONE, $DONE)`; passes) and a test `a_script_imports_a_module` in `crates/test262/tests/run.rs`; the tree's `harness/` gets `doneprintHandle.js` as `fixtures/async/` has it.
- Tests in `crates/swell/tests/dynamic_import.rs` (new), each in both collection modes: `host_55_host_defined_is_the_same_rc_and_the_referrer_is_that_script_or_module`, `host_65_import_in_a_script_has_a_script_referrer_and_no_host_defined`, `host_66_import_from_a_host_function_without_a_script_has_a_realm_referrer`, `an_options_object_that_throws_rejects_and_calls_no_hook`, `an_unsupported_attribute_rejects_with_a_syntax_error`, `import_of_a_module_that_throws_rejects_with_its_error`, `two_imports_of_one_specifier_share_one_module`, `import_inside_a_module_has_the_module_as_referrer`, `import_of_a_module_with_top_level_await_settles_after_it`, `a_dropped_request_of_import_leaves_its_promise_pending`. `crates/swell/tests/main.rs` generated again.
- Rules (the `AGENTS.md` of the folder that holds the repository, and `AGENTS.md`, "Rules" and "Commands and the gate"): no `unsafe` (`#![forbid(unsafe_code)]`), no third-party crate, no panic, no `unwrap`, `expect` or unchecked indexing in library code; every loop of the change (a walk of a module graph, of a list of entries, of a namespace's names) is charged to the step budget and every allocation to the memory budget, and a new limit gets a test that reaches it; a builtin's module and comments cite its clause. Time ("Tests stay fast"): a single test is terminated at 60 s and counts as failed, and the gate fails when the wall time of its test steps passes 120 s; the unit times each test before it adds it and names in its report every test over 5 s with its time; one integration-test binary per crate (a new test file is a module of the crate's `tests/main.rs`, written again by `sh taskfiles/test-main.sh write`); an expensive product several tests read (a parsed corpus, a fixture tree, a built part of the test262 checkout) is made once, as a committed file or one shared fixture, never per test and never in a `static`; no test makes a network request or a model call. Builds and tests go through the pool: `cargo` is the shim of the pool, anything else that compiles or tests is prefixed `slot`; a pool job ends after 10 minutes unless it names its time with `slot --timeout` or `SLOT_TIMEOUT`, at most 30 minutes; a script that only measures, probes or waits starts without the `slot` prefix. No Python in anything committed (tools, tasks, gates, tests, helpers): Rust or POSIX sh. The unit runs the tests of its own change, never the whole gate and never a whole-workspace test run; intermediate files go in the scratch directory; no `cd ... &&` with a write or a delete.
- Size: about 1,100 lines (the instruction, EvaluateImportCall and ContinueDynamicImport 400, the active script or module 100, the runner 50, the tests and fixtures 550): one session.

Shared with the other stories of order 6 (`story:ess-target-module-commands`, `story:module-determinism-faults-and-emulated-loading`): `conformance/host/requirements.txt` (this story: lines 55, 65 and 66; the determinism story line 159) and `crates/swell/tests/main.rs` (generated); no other file.

## Requirements

HOST-55, HOST-65 and HOST-66: lines 55, 65 and 66 of `conformance/host/requirements.txt` move from `pending slice 11` to `HOST-<n> test swell::dynamic_import::host_<n>_...`. `website/data/status.json` written again. For the coordinator: the three lines under `### Host interface` of `## [Unreleased]`; no new public name (`Referrer`, `LoadRequest::is_dynamic` and `host_defined` exist).

Writes: `crates/syntax/src/parser/{mod,expression,statement}.rs` (the hold), `crates/syntax/tests/import_call.rs`, `crates/syntax/tests/sources/held/dynamic-import.js` (moved to `valid/`), `crates/compiler/src/{bytecode,disasm,expression,resolve}.rs`, `crates/compiler/tests/sources/import-call{,-with-options}.{js,disasm}` (new), `crates/swell/src/interp/{dispatch,frame}.rs` and the files that make a frame, `crates/swell/src/{module,script}.rs`, `crates/swell/src/realm/mod.rs` ([[LoadedModules]] of a realm), `crates/swell/src/host/jobs.rs` (the active script or module of a job), `crates/swell/tests/dynamic_import.rs` (new), `crates/swell/tests/main.rs` (generated), `crates/test262/src/host.rs`, `crates/test262/tests/run.rs`, `crates/test262/tests/fixtures/modules/`, `conformance/host/requirements.txt` (3 lines), `website/data/status.json`. A unit that meets a file outside this line that its change cannot compile or pass without makes the smallest change there and reports it. Not written: `spec/`, `conformance/ess/`, `docs/design/`, `CHANGELOG.md`.

Order 6, beside `story:ess-target-module-commands` and `story:module-determinism-faults-and-emulated-loading`; it depends on `story:module-namespace-objects`, `story:top-level-await`, `story:import-meta` and `story:json-modules` (its cases use each part; its instruction comes after that of `import.meta`).

## Acceptance

`cargo nextest run --locked -p swell --test main -E 'test(/^dynamic_import::/) | test(/^modules::/) | test(/^top_level_await::/) | test(/^json_modules::/) | test(/^jobs::/) | test(/^promises::/) | test(/^eval::/)'` exits 0 in both collection modes, with the 10 tests of `dynamic_import::` (none on the base); `cargo nextest run --locked -p swell-syntax --test main`, `cargo nextest run --locked -p swell-compiler --test main`, `cargo nextest run --locked -p swell --lib` and `cargo nextest run --locked -p swell-test262 --test main` exit 0. `grep -c '", 11)' crates/syntax/src/parser/mod.rs` prints `0` (`HOLDS` is empty; `4` after order 2); `grep -c 'pending slice 11$' conformance/host/requirements.txt` prints 3 fewer than on the join the unit starts from; `slot task -d . -t taskfiles/host-requirements.yml host-requirements-check` exits 0. `slot task clippy` exits 0.

test262. The full run is not the unit's. In one pool job on the build node: `slot --timeout 30m --fetch .out/test262-result sh docs/plan/slice-11/run-part.sh docs/plan/slice-11/story-dynamic-import.cases.txt docs/plan/slice-11/guard-async.cases.txt`; then on the workstation, with the base run of the join the unit starts from copied to `.out/test262-base/runs.tsv`, `sh docs/plan/slice-11/check.sh docs/plan/slice-11/story-dynamic-import.cases.txt .out/test262-base/runs.tsv .out/test262-result/runs.tsv` exits 0 and prints `passed <P> bound 782 B 298 F 484 R 0` with `<P>` at least 782 (over the base run of `f962c324` the part holds 793 cases: the 484 of the list, all failing there; the 298 of `guard-async.cases.txt`, all passing; and 11 cases of other lists that a file of the list names by `./`).

## Out of scope

`import.source` and `import.defer` (stage 3, excluded); the 4 slice-12 cases above (slice 12's; they need BigInt); the ESS target's `HostLoadImportedModule` with a script referrer, which the model does not state (`spec/domains/host.yaml` line 296; `story:ess-target-module-commands`); a module loader of the command-line host.
