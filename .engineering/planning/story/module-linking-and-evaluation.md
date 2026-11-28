---
format: aep.planning-md/3
id: story:module-linking-and-evaluation
kind: story
status: draft
title: link and evaluate_module of a graph without top-level await; the runner judges resolution and runtime negatives of module runs
summary: InnerModuleLinking, ResolveExport, GetExportedNames, InitializeEnvironment and synchronous evaluation on explicit work stacks; import slots read through to their target; the runner links, evaluates and judges; HOST-38, 40, 42-44, 74, 75, 97, 98, 150, 157. Order 4; 122 cases.
relations:
- decomposes: epic:slice-11-modules
- depends_on: story:module-records-loading-and-module-runs
scope:
- confidence: cited
  path: conformance/host/requirements.txt
- confidence: cited
  path: crates/swell/src/compile_fail.rs
- confidence: cited
  path: crates/swell/src/host/agent.rs
- confidence: cited
  path: crates/swell/src/host/mod.rs
- confidence: inferred
  path: crates/swell/src/interp/dispatch.rs
- confidence: inferred
  path: crates/swell/src/module.rs
- confidence: cited
  path: crates/swell/tests/faults.rs
- confidence: cited
  path: crates/swell/tests/fuzz.rs
- confidence: cited
  path: crates/swell/tests/main.rs
- confidence: cited
  path: crates/swell/tests/misuse.rs
- confidence: inferred
  path: crates/swell/tests/modules.rs
- confidence: cited
  path: crates/test262/src/run.rs
- confidence: inferred
  path: crates/test262/tests/fixtures/modules
- confidence: cited
  path: crates/test262/tests/run.rs
- confidence: cited
  path: website/data/status.json
revision: 3
---
## Context

Slice 11 brings modules (`epic:slice-11-modules`; `docs/design/engine.md`, section 12, item 11). This story links and evaluates a module graph whose modules have no top-level `await`: `Agent::link` and `Agent::evaluate_module` of `docs/design/host-interface.md` section 9, and the runner's module run to its end. It builds on the records, loading and the runner's loader of `story:module-records-loading-and-module-runs`, after which a module case that is not a parse negative ends failed with the cause `a module run is not linked before story:module-linking-and-evaluation`.

The count. 122 cases (122 runs, every one a module run) are this story's (`docs/plan/slice-11/story-module-linking-and-evaluation.cases.txt`): 89 that link and run, 29 whose `negative` is a `resolution` `SyntaxError` (an import of a name the target does not export, an ambiguous star export, a cycle of indirect exports: `#sec-resolveexport`, `#sec-source-text-module-record-initialize-environment` step 7), and 4 with a `runtime` negative (`Test262Error` 2, `TypeError` 1, `ReferenceError` 1: an import binding read before its initialization). Every one fails on the base on "a module run is not made before slice 11". The cases with a namespace import, top-level `await`, `import.meta`, a JSON module or `import()` are the lists of orders 5 and 6.

Design it follows: `host-interface.md` section 9 (`link`, `evaluate_module`), decision 10 (a termination of linking leaves the graph `Unlinked`, HOST-75; of evaluation leaves each evaluating module at `Evaluating` with `module_terminated`, HOST-74), decision 13 (`module_status`), decision 1 (`HookCx` has no call that evaluates a module, HOST-150); ECMA-262 16.2.1.6 (`#sec-moduledeclarationlinking`, `#sec-InnerModuleLinking`, `#sec-moduleevaluation`, `#sec-innermoduleevaluation`, `#sec-resolveexport`, `#sec-getexportednames`, `#sec-source-text-module-record-initialize-environment`, `#sec-source-text-module-record-execute-module`); `engine.md` decision 11, "`negative`" (`resolution`: "module loading or linking must fail with it"), and the paragraph "Modules" the design author adds before order 3 (the explicit work stacks and the charge of each walk; Context of `story:module-records-loading-and-module-runs`).

## What later stories reuse

Link and evaluation over the record's fields; ResolveExport and GetExportedNames, which `story:module-namespace-objects` calls; InnerModuleEvaluation with the fields of the async part declared and left at their sync values ([[HasTLA]] false, [[AsyncEvaluationOrder]] unset), which `story:top-level-await` completes; the runner's judgement of `resolution` and `runtime` for a module run, which every later story's cases use.

## Domain relations

- Module → its environment: one to one, owned, made by InitializeEnvironment at `link` and kept for the module's life — inferable (inferred from ECMA-262 `#sec-source-text-module-record-initialize-environment` and `#sec-cyclic-module-records`, field [[Environment]]; no ess/1 document declares it: `swell.host.Module` in `spec/domains/host.yaml` holds no environment).
- Module → [[CycleRoot]]: many to one, a module of the same strongly connected component, set at evaluation — inferable (inferred from `#sec-innermoduleevaluation` step 16; no ess/1 document declares it).
- Module status: the lifecycle `New` → `Unlinked` → `Linked` → `Evaluated` of `swell.host.Module` (`spec/domains/host.yaml`, transitions `load`, `link`, `evaluate`); the states `linking` and `evaluating` are `UNMAPPED` there (line 237: they hold only inside one call), and the engine's `ModuleStatus` (`crates/swell/src/host/outcome.rs` line 84) has them.

## Change

Failing tests first: the tests below and the runner's fixture cases are written before the code and fail on the base.

- `crates/swell/src/module.rs`: `link` (InnerModuleLinking over an explicit work stack, Tarjan's indices [[DFSIndex]] and [[DFSAncestorIndex]], one step per module visited, charged before; on a throw every module of the stack goes back to `Unlinked`), ResolveExport (its resolveSet and the star-export search, `ambiguous` and `null` as the draft has them, one step per (module, name) pair visited), GetExportedNames (its exportStarSet, one step per module visited), InitializeEnvironment (the module environment of the layout `story:module-code-compiles` gives, each import slot set to the binding ResolveExport answered, a `SyntaxError` for an import that resolves to `null` or `ambiguous`, the functions instantiated, the lexical bindings uninitialized; the environment charged to the memory budget), `evaluate_module` (InnerModuleEvaluation over an explicit work stack; ExecuteModule runs the body as an entry of its own frame, `this` undefined, the realm the module's; [[EvaluationError]] kept and answered again to every later evaluation; the promise of the top-level capability made with `Cx::new_promise_capability` and settled before the call returns). `crates/swell/src/interp/`: `GetNamed` and the instructions that write a named slot read an import slot through to its target, and an uninitialized target throws a `ReferenceError` (`#sec-module-environment-records-getbindingvalue-n-s`).
- Entry points (`crates/swell/src/host/agent.rs`, after `module_status`; the `Cx` forms in `crates/swell/src/host/mod.rs`): `link(&mut self, module) -> Result<Outcome<()>, HostError>` (`Err(ModuleStatus { found })` for `New`, HOST-42) and `evaluate_module(&mut self, module) -> Result<Outcome<Root>, HostError>` (`Err(ModuleStatus { found })` for `New` and `Unlinked`, HOST-43; `Err(EvaluationInProgress)` from a host function called by module code during an evaluation, HOST-44; `Err(ModuleTerminated)` for a graph a termination left, HOST-74). `HookCx` gets neither (HOST-150, a `compile_fail` test). `FaultPoint` (`agent.rs` line 252) gets `ParseModule` and `Link`, and the injected fault ends each with `Terminated(EngineFault)` (HOST-157).
- `swell-test262`: `crates/test262/src/run.rs`, the module run after loading (the line `a module run is not linked before story:module-linking-and-evaluation` of `story:module-records-loading-and-module-runs` goes): a rejected load or a `Throw` of `link` is judged against a `negative` of phase `resolution` (`Negative::Resolution`, line 308 on the base, by `parse_kind` of the thrown value, line 421), and fails a run without one; then `evaluate_module`, a microtask checkpoint, and the promise: `Fulfilled` passes a case without a `negative`, `Rejected` is judged as a `runtime` negative is for a script (`describe`, line 657) and fails one without; a module case with the flag `async` is judged by what `print` received after the checkpoint (`judge_async`, line 607). Fixture cases added to `crates/test262/tests/fixtures/modules/test/language/module-code/`: `links.js` (imports `x` from `./links_FIXTURE.js`, which exports it; passes), `unresolvable.js` (`negative` `resolution` `SyntaxError`, imports a name the fixture does not export; passes), `throws.js` (`negative` `runtime` `Test262Error`, body `throw new Test262Error()`; passes); `loads.js` of the records story now passes. Tests in `crates/test262/tests/run.rs`: `a_module_graph_links_and_runs`, `a_resolution_negative_passes_on_the_link_error`.
- Tests, each in both collection modes, in `crates/swell/tests/modules.rs`: `host_38_a_throwing_body_rejects_the_promise`, `host_40_an_import_of_a_missing_export_throws_a_syntax_error_and_stays_unlinked`, `host_42_link_of_a_new_module_answers_module_status`, `host_43_evaluate_of_a_new_or_unlinked_module_answers_module_status`, `host_44_evaluate_inside_an_evaluation_answers_evaluation_in_progress`, `host_74_a_terminated_evaluation_leaves_evaluating_and_module_terminated`, `host_75_a_terminated_link_leaves_unlinked_and_a_larger_budget_links`, `host_97_status_through_parse_load_link_and_evaluate`, `host_98_a_host_function_in_the_body_reads_evaluating`, `a_cycle_of_three_modules_links_and_evaluates_once`, `an_import_read_before_its_initialization_throws_a_reference_error`, `a_long_chain_of_modules_ends_link_in_step_budget` (a chain of 500 modules: the walk does not grow the Rust stack, and a small `steps_per_entry` ends it) and `a_star_export_cycle_resolves_to_null`. `crates/swell/tests/faults.rs`: `host_157_injected_fault_in_parse_module_or_link_faults_the_agent` (with the feature `fault-injection`). `crates/swell/src/compile_fail.rs`: `host_150_hook_cx_cannot_evaluate_a_module` (`compile_fail,E0599`). `crates/swell/tests/misuse.rs` (`host_48`, `agent_entries_refuse_foreign`) and `crates/swell/tests/fuzz.rs` (`host_3` and its parts): `link` and `evaluate_module` among their calls. `crates/swell/tests/main.rs` generated again.
- Rules (the `AGENTS.md` of the folder that holds the repository, and `AGENTS.md`, "Rules" and "Commands and the gate"): no `unsafe` (`#![forbid(unsafe_code)]`), no third-party crate, no panic, no `unwrap`, `expect` or unchecked indexing in library code; every loop of the change (a walk of a module graph, of a list of entries, of a namespace's names) is charged to the step budget and every allocation to the memory budget, and a new limit gets a test that reaches it; a builtin's module and comments cite its clause. Time ("Tests stay fast"): a single test is terminated at 60 s and counts as failed, and the gate fails when the wall time of its test steps passes 120 s; the unit times each test before it adds it and names in its report every test over 5 s with its time; one integration-test binary per crate (a new test file is a module of the crate's `tests/main.rs`, written again by `sh taskfiles/test-main.sh write`); an expensive product several tests read (a parsed corpus, a fixture tree, a built part of the test262 checkout) is made once, as a committed file or one shared fixture, never per test and never in a `static`; no test makes a network request or a model call. Builds and tests go through the pool: `cargo` is the shim of the pool, anything else that compiles or tests is prefixed `slot`; a pool job ends after 10 minutes unless it names its time with `slot --timeout` or `SLOT_TIMEOUT`, at most 30 minutes; a script that only measures, probes or waits starts without the `slot` prefix. No Python in anything committed (tools, tasks, gates, tests, helpers): Rust or POSIX sh. The unit runs the tests of its own change, never the whole gate and never a whole-workspace test run; intermediate files go in the scratch directory; no `cd ... &&` with a write or a delete.
- Size: about 1,700 lines (link, ResolveExport, GetExportedNames and InitializeEnvironment 600, evaluation 300, the interpreter 100, the runner 150, the tests and fixtures 550): one session.

## Requirements

HOST-38, HOST-40 (`host-interface.md` lines 648, 652), HOST-42 to HOST-44 (lines 709 to 713), HOST-74, HOST-75 (lines 1083, 1085), HOST-97, HOST-98 (lines 1551, 1553), HOST-150 (line 122) and HOST-157 (line 1077): their lines of `conformance/host/requirements.txt` move from `pending slice 11` to `HOST-<n> test swell::modules::host_<n>_...` (HOST-150 `swell::doc::compile_fail::...`, HOST-157 `swell::faults::...`). `website/data/status.json` written again. For the coordinator (this story does not write `CHANGELOG.md`): under `### Host interface` of `## [Unreleased]`, `Agent::link`, `Agent::evaluate_module`, `Cx::link`, `Cx::evaluate_module`, `FaultPoint::ParseModule`, `FaultPoint::Link` and the 11 `HOST-<n>` lines; the unit runs `slot task host-surface-check` and reports the lines it prints.

Writes: `crates/swell/src/module.rs`, `crates/swell/src/host/{agent,mod}.rs`, `crates/swell/src/interp/` (the named-slot instructions), `crates/swell/src/compile_fail.rs`; `crates/swell/tests/{modules,faults,misuse,fuzz}.rs`, `crates/swell/tests/main.rs` (generated); `crates/test262/src/run.rs`, `crates/test262/tests/run.rs`, `crates/test262/tests/fixtures/modules/` (three cases and one fixture); `conformance/host/requirements.txt` (11 lines); `website/data/status.json`. A unit that meets a file outside this line that its change cannot compile or pass without makes the smallest change there and reports it. Not written: `crates/syntax/`, `crates/compiler/` (unless the layout of `story:module-code-compiles` lacks what InitializeEnvironment needs, which the unit reports), `crates/swell/src/heap/` (the record and its trace are the records story's), `spec/`, `conformance/ess/`, `docs/design/`, `CHANGELOG.md`.

The only story of order 4. It depends on `story:module-records-loading-and-module-runs`.

## Acceptance

Run by the binaries of this change, each in both collection modes: `cargo nextest run --locked -p swell --test main -E 'test(/^modules::/) | test(/^misuse::/) | test(/^fuzz::host_3_/) | test(/^termination::/) | test(/^budgets::/)'` exits 0 with the 13 new tests of `modules::` (none of which exists on the base); `cargo nextest run --locked -p swell --features fault-injection --test main -E 'test(/^faults::/)'` exits 0 with `host_157_...`; `cargo nextest run --locked -p swell --lib` and `cargo test --locked -p swell --doc host_` exit 0; `cargo nextest run --locked -p swell-test262 --test main` exits 0. `slot task -d . -t taskfiles/host-requirements.yml host-requirements-check` exits 0 and `grep -c 'pending slice 11$' conformance/host/requirements.txt` prints 11 fewer than on the join the unit starts from (`19` after the records story, so `8`). `git grep -n 'is not linked before story:module-linking-and-evaluation' -- crates` prints nothing. `slot task clippy` exits 0; `slot --local task site-data-check` exits 0.

test262. The full run is not the unit's. In one pool job on the build node: `slot --timeout 30m --fetch .out/test262-result sh docs/plan/slice-11/run-part.sh docs/plan/slice-11/story-module-linking-and-evaluation.cases.txt docs/plan/slice-11/guard-async.cases.txt`; then on the workstation, with the base run of the join the unit starts from copied to `.out/test262-base/runs.tsv`, `sh docs/plan/slice-11/check.sh docs/plan/slice-11/story-module-linking-and-evaluation.cases.txt .out/test262-base/runs.tsv .out/test262-result/runs.tsv` exits 0 and prints `passed <P> bound 420 B 298 F 122 R 0` with `<P>` at least 420 (over the base run of `f962c324` the part holds 423 cases: the 122 of the list, all failing there; the 298 of `guard-async.cases.txt` (`Promise.all`, `resolve`, `then`, `await`, `async function`), all passing, which the module run's checkpoint must not change; and 3 cases of other lists that a file of the list names by `./`).

## Out of scope

Namespace objects and `import * as` (`story:module-namespace-objects`); top-level `await`, [[HasTLA]], the async parts of evaluation and HOST-99 (`story:top-level-await`); `import.meta` (`story:import-meta`); JSON modules (`story:json-modules`); `import()` (`story:dynamic-import`); the ESS target (`story:ess-target-module-commands`); determinism over modules, HOST-159 (`story:module-determinism-faults-and-emulated-loading`).
