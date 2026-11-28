---
format: aep.planning-md/3
id: story:ess-target-module-commands
kind: story
status: draft
title: The ESS target runs the seven module commands of the model over the Rust interface
summary: Arms of the conformance target for ParseModule, LoadRequestedModules, FinishLoadingImportedModule, LinkModule, EvaluateModule and the awaited HostLoadImportedModule and AsyncModuleExecutionSettled; a planted-defect test; the floor and maximum of conform-engine set to the measured counts. Order 6; no case.
relations:
- decomposes: epic:slice-11-modules
- depends_on: story:top-level-await
- depends_on: story:module-namespace-objects
- depends_on: story:import-meta
- depends_on: story:json-modules
scope:
- confidence: cited
  path: conformance/ess/src/target.rs
- confidence: cited
  path: conformance/ess/tests/planted_defect.rs
- confidence: cited
  path: taskfiles/conform-engine.yml
revision: 6
---
## Context

The epic's acceptance 5: "The conformance suite of the ESS model holds a passed scenario for the module commands and events it states, run over the thin adapter to the Rust interface (decision 10)". The model already states them (`spec/domains/host.yaml`: the entities `swell.host.Module`, line 242, and `swell.host.ModuleLoad`, line 302; the commands `ParseModule` (line 712), `LoadRequestedModules`, `FinishLoadingImportedModule`, `LinkModule`, `EvaluateModule`, the hook `HostLoadImportedModule` and `AsyncModuleExecutionSettled`; the events `ModuleParsed`, `ModuleLoadRequested`, `ModuleLoadEnded`, `ModuleStatusChanged`), so the epic's first open question ("Whether the ESS model of the foundation already holds the module commands and events, or this epic adds them") is answered by the model: it holds them, and this story adds none. The second ("How ESS states the asynchronous loading as a library interface") is answered by the model's form, which this story runs: the hook `HostLoadImportedModule` publishes `ModuleLoadRequested`, the host ends the load with the command `FinishLoadingImportedModule` (`engine.md` decision 10, line 3509 on the base). On the base (`f962c324`) the target answers `unsupported` for these seven commands (`conformance/ess/src/target.rs`, `later_slice`, lines 381 to 392: "slice 11 (modules)"; its module documentation, line 36), and its host answers the load hook with nothing (line 188).

The count. The suite synthesized on the base (`ESS_TOOLCHAIN=0.97.4 ess verify conform synthesize`, the toolchain the spec pins) has 60 scenarios; 34 of them use a module command (`ParseModule` in 138 steps, `LoadRequestedModules` 149, `LinkModule` 98, `EvaluateModule` 91, counted with `grep -o` over the suite), and the gate's line on the base is `60 scenarios: 18 passed, 0 failed, 0 error, 42 unsupported` (`taskfiles/conform-engine.yml`: the floor `passed < 18`, line 86; `ESS_UNSUPPORTED_MAX` default 42, line 5). The authored scenario `spec/scenarios/module-with-top-level-await.yaml` is among them.

Design it follows: `engine.md` decision 10 (the ESS model of modules; the thin adapter); `host-interface.md` section 9; the story form of `story:ess-target-checkpoint-and-promise-hooks` (slice 10: the arms of `invoke`, the forced answers of `force`, the awaited hooks recorded in the scenario's host data, a planted-defect test, the floor and the maximum of the task).

## What later stories reuse

Nothing of the engine. The floor and the maximum of `taskfiles/conform-engine.yml`, which a later slice raises.

## Domain relations

- None new: the relations are those of `spec/domains/host.yaml` (`swell.host.Module`, relation `realm`; `swell.host.ModuleLoad`, relation `referrer_module`), which the target maps and does not change.

## Change

Failing tests first: the planted-defect test below is written before the code and fails on the base (the target answers `unsupported` for every module command).

- `conformance/ess/src/target.rs`: an arm of `invoke` (beside `"swell.host.EvaluateScript"`, line 456 on the base) for each of `ParseModule` (`Agent::parse_module`; `parsed`, `parse-failed`, `no-realm`, ...), `LoadRequestedModules` (`load_requested_modules`), `FinishLoadingImportedModule` (`finish_loading_imported_module` with the `LoadRequest` the scenario holds for its `load_id`, a module or a thrown value), `LinkModule` (`link`) and `EvaluateModule` (`evaluate_module`), each answering the outcomes of the model for the interface's answers (`Err(ModuleStatus { found })` → the model's `not-linked` and its kin, `Terminated(StepBudget)` → `step-budget`, `Err(Faulted)` → `faulted`), and their forced answers in `force`; `ConformanceHost` (line 126) overrides `host_load_imported_module` (line 188) to keep the `LoadRequest` in the scenario's host data under a new `load_id` and record the call; `await_outcome` (line 1102) answers the awaited `HostLoadImportedModule` from that record (`ModuleLoadRequested`) and `AsyncModuleExecutionSettled` from `module_status` after a checkpoint (`settled` when it reads `Evaluated`). A module of another agent or a request the scenario does not hold answers `no-module` and its kin without a call, as `no-script` is answered. `later_slice` keeps only `HostEnqueueGenericJob` and `HostEnqueueTimeoutJob` (slice 14); the module documentation (line 36) changes with it.
- `conformance/ess/tests/planted_defect.rs`: `a_target_whose_link_does_nothing_fails_a_scenario`, in the form of `a_target_whose_checkpoint_runs_no_job_fails_a_scenario` (line 108): a target whose `LinkModule` answers `linked` without calling `link` fails a scenario that then evaluates the module. The crate keeps its test binaries as they are.
- `taskfiles/conform-engine.yml`: the floor of passed scenarios (`passed < 18`, line 86, its message, `desc` and `summary`) set to the passed count the unit measures, `ESS_UNSUPPORTED_MAX` (`default 42`, line 5) to the unsupported count it measures, and the grep of the planted-defect step (lines 62 to 71) extended to the new test.
- No scenario is added or edited (each would need a policy admission, which the coordinator asks for); a module scenario that stays `unsupported` is listed in the report with the reason the target gives.
- Rules (the `AGENTS.md` of the folder that holds the repository, and `AGENTS.md`, "Rules" and "Commands and the gate"): no `unsafe` (`#![forbid(unsafe_code)]`), no third-party crate, no panic, no `unwrap`, `expect` or unchecked indexing in library code; every loop of the change (a walk of a module graph, of a list of entries, of a namespace's names) is charged to the step budget and every allocation to the memory budget, and a new limit gets a test that reaches it; a builtin's module and comments cite its clause. Time ("Tests stay fast"): a single test is terminated at 60 s and counts as failed, and the gate fails when the wall time of its test steps passes 120 s; the unit times each test before it adds it and names in its report every test over 5 s with its time; one integration-test binary per crate (a new test file is a module of the crate's `tests/main.rs`, written again by `sh taskfiles/test-main.sh write`); an expensive product several tests read (a parsed corpus, a fixture tree, a built part of the test262 checkout) is made once, as a committed file or one shared fixture, never per test and never in a `static`; no test makes a network request or a model call. Builds and tests go through the pool: `cargo` is the shim of the pool, anything else that compiles or tests is prefixed `slot`; a pool job ends after 10 minutes unless it names its time with `slot --timeout` or `SLOT_TIMEOUT`, at most 30 minutes; a script that only measures, probes or waits starts without the `slot` prefix. No Python in anything committed (tools, tasks, gates, tests, helpers): Rust or POSIX sh. The unit runs the tests of its own change, never the whole gate and never a whole-workspace test run; intermediate files go in the scratch directory; no `cd ... &&` with a write or a delete.
- Report: the sum of the CPU seconds of the story's new tests in the default mode and in the mode `collect-every-safe-point` (each test run alone, user plus system), and the wall seconds of its part run when it has one; no story bounds the CPU seconds of a whole gate.
- Generated files: the generated `tests/main.rs` of every crate and `website/data/status.json` are in no story's scope. A unit that adds a test module writes `tests/main.rs` again with `sh taskfiles/test-main.sh write`; a unit that meets a conflict in a generated file takes either side and writes it again; the coordinator writes both again on the join (`sh taskfiles/test-main.sh write`, `slot --local task site-data`).
- Line numbers are those of `f962c324`; anchors are found by name.
- Size: about 600 lines (the target 450, the test 60, the task 20): one unit.

Shared with the other stories of order 6 (`story:dynamic-import`, `story:module-determinism-and-emulated-loading`): no file.

## Requirements

None of `conformance/host/requirements.txt` moves.

Writes: `conformance/ess/src/target.rs`, `conformance/ess/tests/planted_defect.rs` (one test), `taskfiles/conform-engine.yml` (the floor, the maximum, the grep). Not written: `spec/` (the model holds the commands; a declaration found wrong is reported, not changed), `crates/` (every engine file), `conformance/host/`, `docs/design/`.

Order 6, beside `story:dynamic-import` and `story:module-determinism-and-emulated-loading`; it depends on `story:top-level-await` (the last part `AsyncModuleExecutionSettled` needs), `story:module-namespace-objects`, `story:import-meta` and `story:json-modules` (a scenario's module source may hold a namespace import, `import.meta` or a JSON request; with every engine part of the modules but `import()` landed, the floor and the maximum the unit measures do not move again by a later story of the slice, and `import()` from a script is not stated by the model), and through them on the records and linking stories.

## Acceptance

`slot --timeout 30m task suite` exits 0, then `slot --timeout 30m task conform-engine` exits 0 and prints `conform-engine: planted defect refused: test a_target_whose_link_does_nothing_fails_a_scenario ... ok`; its summary line has more passed and fewer unsupported scenarios than on the base (`60 scenarios: 18 passed, 0 failed, 0 error, 42 unsupported`), and the report quotes both lines. The floor and the maximum equal the measured counts: with `last=$(tail -n 1 .out/conform-engine.log)`, `grep -c "passed < $(printf '%s\n' "$last" | awk '{ print $3 }'))" taskfiles/conform-engine.yml` prints `1` and `grep -c "default $(printf '%s\n' "$last" | awk '{ print $9 }')}}" taskfiles/conform-engine.yml` prints `1`. Each of the seven commands is executed by at least one passed scenario, and each of the four events of the model (`ModuleParsed`, `ModuleLoadRequested`, `ModuleLoadEnded`, `ModuleStatusChanged`) is emitted by at least one passed scenario: the report names, per command and per event, one passed scenario from `.out/conform-engine.json`; `module-with-top-level-await` is among the passed. `grep -c '#\[test\]' conformance/ess/tests/planted_defect.rs` prints `3` (`2` on the base). `grep -rn 'slice 11 (modules)' conformance spec taskfiles` prints nothing (line 390 of `target.rs` on the base). `git log --no-merges --first-parent --format= --name-only "$BASE"..HEAD -- crates spec` prints nothing (`BASE` the join the unit starts from: the unit's own commits write no engine file and no specification, whatever a merge of the wave branch brings). `slot task clippy` exits 0. Time: the new test stays under 1.5 CPU seconds alone, and the step `conform-engine` is timed before and after (both times reported; a rise over 5 s is fixed in this change).

test262: no case moves; the story writes no file the runner reads.

## Out of scope

A load requested by `import()` with a script or realm referrer, which the model does not state (`spec/domains/host.yaml` line 296, `UNMAPPED`, HOST-65 and HOST-66: the tests of `story:dynamic-import` carry them); `HostEnqueueGenericJob` and `HostEnqueueTimeoutJob` (slice 14); any engine file.
