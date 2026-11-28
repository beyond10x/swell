---
format: aep.planning-md/3
id: story:json-modules
kind: story
status: draft
title: 'JSON modules: parse_json_module, the synthetic record, the NotJsonModule check and the runner''s .json loads'
summary: parse_json_module and the synthetic kind with one default export; finish answers NotJsonModule for a source-text module on a type json request; the runner supports the attribute type and loads .json files; HOST-64. Order 5; 10 cases.
relations:
- decomposes: epic:slice-11-modules
- depends_on: story:module-linking-and-evaluation
scope:
- confidence: cited
  path: conformance/host/requirements.txt
- confidence: cited
  path: crates/swell/src/host/agent.rs
- confidence: cited
  path: crates/swell/src/host/mod.rs
- confidence: inferred
  path: crates/swell/src/module/json.rs
- confidence: inferred
  path: crates/swell/src/module/loading.rs
- confidence: cited
  path: crates/swell/tests/fuzz.rs
- confidence: inferred
  path: crates/swell/tests/json_modules.rs
- confidence: cited
  path: crates/swell/tests/misuse.rs
- confidence: cited
  path: crates/test262/src/host.rs
- confidence: inferred
  path: crates/test262/tests/fixtures/modules-json
- confidence: cited
  path: crates/test262/tests/run.rs
revision: 7
---
## Context

Slice 11 brings modules (`epic:slice-11-modules`; `docs/design/engine.md`, section 12, item 11; the `feature` lines `json-modules` and `import-attributes`). This story brings JSON modules: `Agent::parse_json_module` of `docs/design/host-interface.md` section 9, the synthetic module record with one default export, the check `NotJsonModule` of `finish_loading_imported_module`, and the runner's loading of a `.json` file. After `story:module-records-loading-and-module-runs` the runner finishes a `.json` load with a thrown `SyntaxError` and the cause `a JSON module is not loaded before story:json-modules`, and the runner's host keeps the default `SUPPORTED_IMPORT_ATTRIBUTES` (`&[]`, `crates/swell/src/host/hooks.rs` line 176), so an import `with { type: "json" }` rejects its graph (HOST-58).

The count. 10 cases (10 runs, all module runs) are this story's (`docs/plan/slice-11/story-json-modules.cases.txt`): all under `language/import/import-attributes/`: 8 that import a JSON file and read its value (an array, a boolean, `null`, a number, an object, a string; the extensibility of an array and of an object) and 2 with a `resolution` `SyntaxError` (`json-invalid.js`, an invalid JSON file; `json-named-bindings.js`, a named import of a JSON module, which has only `default`). Every one fails on the base on "a module run is not made before slice 11". The case `language/import/import-attributes/json-via-namespace.js` needs this story and `story:module-namespace-objects` both, and is on the list of `story:dynamic-import`, the first story whose dependencies hold both.

Design it follows: ECMA-262 16.2.1.8 and 16.2.1.9 (`#sec-synthetic-module-records`, `#sec-create-default-export-synthetic-module`, `#sec-parse-json-module`; a Synthetic Module Record's GetExportedNames answers its export names and its ResolveExport a binding of its environment for each, `null` for any other name), 16.2.1.10 (`#sec-HostLoadImportedModule`: a request with `type` `"json"` is finished with the result of ParseJSONModule), `#sec-hostgetsupportedimportattributes`; `host-interface.md` section 9 (`parse_json_module(&mut self, realm, source) -> Result<Outcome<Module>, HostError>`; `FinishErrorKind::NotJsonModule`) and HOST-64 (line 967); `engine.md` decision 11, "Modules" (`.json` is a JSON module), and the paragraph `**Module records.**` (the state of each part in a file of its own).

## What later stories reuse

`parse_json_module` and the synthetic kind of a module record, which `story:dynamic-import` loads for `import("./x.json", { with: { type: "json" } })`; the runner's `SUPPORTED_IMPORT_ATTRIBUTES` `["type"]`.

## Domain relations

- A JSON module → its default export value: one to one, the value of JSON.parse of its source, made at `parse_json_module` and kept by the record — inferable (inferred from ECMA-262 `#sec-create-default-export-synthetic-module`; no ess/1 document declares it: `spec/domains/host.yaml` declares `swell.host.Module` with no kind, and ESS does not restate ECMA-262).
- A load request with the attribute `type: "json"` → a JSON module only: the host may finish it with no other kind — inferable (inferred from `docs/design/host-interface.md` line 967, HOST-64).

## Change

Failing tests first: the tests below are written before the code and fail on the base.

- `swell`, `crates/swell/src/module/json.rs` (the file the records story created with `SyntheticState` empty, to which the linking story added the four methods a synthetic record answers through): `SyntheticState` holds the default export value, with its trace; GetExportedNames of the synthetic kind (`exported_names`) answers `["default"]`, ResolveExport (`resolve_export`) the binding `default` of its environment and `null` for any other name (so `json-named-bindings.js` fails to link with a `SyntaxError`), its link (`link`) makes the environment with the one binding `default`, initialized at link, and its evaluation (`evaluate`) runs nothing, the promise fulfilled; `parse_json_module` (the source parsed by the JSON parser of `JSON.parse`, `crates/swell/src/builtins/json/`, with no reviver; its `SyntaxError` answered as a `Throw` outcome; every value charged to the memory budget and every step of the parse to the step budget, as `JSON.parse` charges them). `crates/swell/src/module/loading.rs` (the records story's), one hunk in `finish_loading_imported_module`: a request whose attributes hold `type` `"json"` finished with a module of the source-text kind answers `FinishError { error: NotJsonModule, request }` and the token handed back (HOST-64). Entry points `Agent::parse_json_module`, `Cx::parse_json_module` and `HookCx::parse_json_module` (`crates/swell/src/host/agent.rs` after `evaluate_module`; `crates/swell/src/host/mod.rs` after `Cx::evaluate_module` and in `HookCx`).
- `swell-test262`, `crates/test262/src/host.rs`: `RunnerHost` sets `SUPPORTED_IMPORT_ATTRIBUTES` to `&["type"]`; the loader finishes a request with `type` `"json"` by `parse_json_module` of the file, a request whose `type` is another value with a thrown `TypeError`, and a `.json` file requested without `type` `"json"` as source text (whose parse fails as any invalid module's); the line `a JSON module is not loaded before story:json-modules` goes. A fixture tree of this story's own, `crates/test262/tests/fixtures/modules-json/` (new; `harness/assert.js`, `harness/sta.js`), with 2 cases under `test/language/import/`: `json.js` (imports `./json_FIXTURE.json` with `type: "json"` and compares a value; passes) and `json-invalid.js` (`negative` `resolution` `SyntaxError`, importing the invalid `json_invalid_FIXTURE.json`; passes), and the 2 fixture files; 1 new test in `crates/test262/tests/run.rs`: `a_json_module_loads_its_value`.
- Tests in a new module `crates/swell/tests/json_modules.rs`, 5, each in both collection modes: `host_64_a_json_request_takes_only_a_json_module`, `a_json_module_exports_its_value_as_default`, `two_imports_of_one_json_module_share_one_value`, `an_invalid_json_module_answers_a_syntax_error`, `a_large_json_module_ends_in_memory_budget`. `crates/swell/tests/misuse.rs` (`host_48`, the helper `agent_entries_refuse_foreign`) and `crates/swell/tests/fuzz.rs` (`host_3` and its parts): `parse_json_module` among their calls.
- Rules (the `AGENTS.md` of the folder that holds the repository, and `AGENTS.md`, "Rules" and "Commands and the gate"): no `unsafe` (`#![forbid(unsafe_code)]`), no third-party crate, no panic, no `unwrap`, `expect` or unchecked indexing in library code; every loop of the change (a walk of a module graph, of a list of entries, of a namespace's names) is charged to the step budget and every allocation to the memory budget, and a new limit gets a test that reaches it; a builtin's module and comments cite its clause. Time ("Tests stay fast"): a single test is terminated at 60 s and counts as failed, and the gate fails when the wall time of its test steps passes 120 s; the unit times each test before it adds it and names in its report every test over 5 s with its time; one integration-test binary per crate (a new test file is a module of the crate's `tests/main.rs`, written again by `sh taskfiles/test-main.sh write`); an expensive product several tests read (a parsed corpus, a fixture tree, a built part of the test262 checkout) is made once, as a committed file or one shared fixture, never per test and never in a `static`; no test makes a network request or a model call. Builds and tests go through the pool: `cargo` is the shim of the pool, anything else that compiles or tests is prefixed `slot`; a pool job ends after 10 minutes unless it names its time with `slot --timeout` or `SLOT_TIMEOUT`, at most 30 minutes; a script that only measures, probes or waits starts without the `slot` prefix. No Python in anything committed (tools, tasks, gates, tests, helpers): Rust or POSIX sh. The unit runs the tests of its own change, never the whole gate and never a whole-workspace test run; intermediate files go in the scratch directory; no `cd ... &&` with a write or a delete.
- Report: the sum of the CPU seconds of the story's new tests in the default mode and in the mode `collect-every-safe-point` (each test run alone, user plus system), and the wall seconds of its part run when it has one; no story bounds the CPU seconds of a whole gate.
- Generated files: the generated `tests/main.rs` of every crate and `website/data/status.json` are in no story's scope. A unit that adds a test module writes `tests/main.rs` again with `sh taskfiles/test-main.sh write`; a unit that meets a conflict in a generated file takes either side and writes it again; the coordinator writes both again on the join (`sh taskfiles/test-main.sh write`, `slot --local task site-data`).
- Line numbers are those of `f962c324`; anchors are found by name.
- Size: about 600 lines (the record and the entry 200, the runner 80, the tests and fixtures 320): one session.

Shared with the other stories of order 5 (`story:module-namespace-objects`, `story:top-level-await`, `story:import-meta`): `conformance/host/requirements.txt` (this story: line 64; the others lines 68, 99, 158 and the line of HOST-224, none adjacent to it). Nothing else: each story of order 5 writes its own file of `crates/swell/src/module/`, and `module/loading.rs`, `crates/swell/src/host/{agent,mod}.rs`, `crates/swell/tests/{misuse,fuzz}.rs` and `crates/test262/` are this story's alone in order 5.

## Requirements

HOST-64 (`host-interface.md` line 967): line 64 of `conformance/host/requirements.txt` moves from `pending slice 11` to `HOST-64 test swell::json_modules::host_64_a_json_request_takes_only_a_json_module`. For the coordinator (this story does not write `CHANGELOG.md` or `conformance/host/surface.txt`): under `### Host interface` of `## [Unreleased]`, `Agent::parse_json_module`, `Cx::parse_json_module`, `HookCx::parse_json_module` and the line HOST-64; the unit runs `slot task host-surface-check` and reports the lines it prints. The coordinator writes `website/data/status.json` again on the join.

Writes: `crates/swell/src/module/{json,loading}.rs` (the hunks above), `crates/swell/src/host/{agent,mod}.rs`, `crates/swell/tests/json_modules.rs` (new), `crates/swell/tests/{misuse,fuzz}.rs`, `crates/test262/src/host.rs`, `crates/test262/tests/run.rs`, `crates/test262/tests/fixtures/modules-json/` (new: two cases, two fixtures, the harness), `conformance/host/requirements.txt` (line 64). Not written: `crates/swell/src/builtins/json/` (read, not changed; if its parser has no entry a module can call, the unit adds the smallest one and reports it), `crates/swell/src/module/{link,evaluate}.rs`, `crates/syntax/`, `crates/compiler/`, `spec/`, `docs/design/`.

Order 5, beside `story:module-namespace-objects`, `story:top-level-await` and `story:import-meta`; it depends on `story:module-linking-and-evaluation`.

## Acceptance

`cargo nextest run --locked -p swell --test main -E 'test(/^json_modules::/)'` runs 5 tests (none on the base: the module does not exist) and exits 0 in both collection modes; `cargo nextest run --locked -p swell --test main -E 'test(/^modules::/) | test(/^builtins_json_parse::/) | test(/^misuse::/) | test(/^fuzz::host_3_/)'` exits 0 in both collection modes; `cargo nextest run --locked -p swell --lib` exits 0; `cargo nextest run --locked -p swell-test262 --test main` exits 0 and `-E 'test(/^run::/)'` runs 33 tests (32 on the join the unit starts from, 28 on the base); `find crates/test262/tests/fixtures/modules-json/test -name '*.js' ! -name '*_FIXTURE*' | wc -l` prints `2` and `find crates/test262/tests/fixtures/modules-json/test -name '*_FIXTURE.json' | wc -l` prints `2` (each `0` before). `grep -c '^HOST-64 pending slice 11$' conformance/host/requirements.txt` prints `0` (`1` on that join) and `grep -c '^HOST-64 test swell::json_modules::' conformance/host/requirements.txt` prints `1`; `slot task -d . -t taskfiles/host-requirements.yml host-requirements-check` exits 0; `git grep -n 'is not loaded before story:json-modules' -- crates` prints nothing. `slot task clippy` exits 0.

test262. The full run is not the unit's. In one pool job on the build node: `slot --timeout 30m --fetch .out/test262-result sh docs/plan/slice-11/run-part.sh docs/plan/slice-11/story-json-modules.cases.txt docs/plan/slice-11/guard-json.cases.txt`; then on the workstation, with the base run of the join the unit starts from copied to `.out/test262-base/runs.tsv`, `sh docs/plan/slice-11/check.sh docs/plan/slice-11/story-json-modules.cases.txt .out/test262-base/runs.tsv .out/test262-result/runs.tsv` exits 0 and prints `passed 87 bound 87 B 77 F 10 R 0` (`B` counts every case of the part that passes on the base run read, and the bound is `B + F - R`; the part holds 87 cases and pulls in none by `./`: the 10 of the list, all failing, and the 77 of `guard-json.cases.txt`, `built-ins/JSON/parse/`, all passing; the line is the same on `f962c324` and on the join).

## Out of scope

A JSON module imported as a namespace and `import()` with `type: "json"` (`json-via-namespace.js` and the rest: `story:dynamic-import`); the command-line host's JSON modules (`story:cli-module-loader` names where they go); `import text` and `import bytes` (stage 3, excluded); CSS modules (HTML's).
