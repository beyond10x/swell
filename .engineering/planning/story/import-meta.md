---
format: aep.planning-md/3
id: story:import-meta
kind: story
status: draft
title: import.meta with its two hooks; a stale or foreign handle in the hook's answer faults the agent
summary: Instruction::ImportMeta, the object made once per module through host_get_import_meta_properties and host_finalize_import_meta, each returned handle checked; lifts the hold import.meta; HOST-68, 158 and HOST-224. Order 5; 6 cases.
relations:
- decomposes: epic:slice-11-modules
- depends_on: story:module-linking-and-evaluation
- depends_on: story:modules-design-text
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
  path: crates/compiler/tests/sources/module-import-meta.disasm
- confidence: inferred
  path: crates/compiler/tests/sources/module-import-meta.js
- confidence: cited
  path: crates/swell/src/interp/dispatch.rs
- confidence: inferred
  path: crates/swell/src/module/import_meta.rs
- confidence: cited
  path: crates/swell/tests/hooks.rs
- confidence: inferred
  path: crates/swell/tests/import_meta.rs
- confidence: cited
  path: crates/syntax/src/parser/expression.rs
- confidence: cited
  path: crates/syntax/src/parser/mod.rs
revision: 9
---
## Context

Slice 11 brings modules (`epic:slice-11-modules`; `docs/design/engine.md`, section 12, item 11; the `feature` line `import.meta`). This story evaluates `import.meta` with the two hooks of `docs/design/host-interface.md` section 9, `host_get_import_meta_properties` and `host_finalize_import_meta` (`crates/swell/src/host/hooks.rs` lines 290 to 305 on the base, with their defaults), and lifts the hold `import.meta` that `story:import-call-and-import-meta-syntax` adds; the node `Expr::ImportMeta` is compiled to `CompileError::Tree` until this story.

It also takes the entry "Slices 2, 9 and 11" of `docs/plan/waves.md`, "What later slices take over": "a requirement for a stale or foreign handle returned by the host at the return sites of that slice (`host_get_code_for_eval`; an override's result; `host_get_import_meta_properties`; the other hooks whose answer holds a handle). The rule is decision 6 of `docs/design/host-interface.md`". Of the hooks of slice 11, `host_get_import_meta_properties` is the only one whose answer holds a handle (`host_load_imported_module` and `host_finalize_import_meta` answer nothing; a `Module` or a thrown value given to `finish_loading_imported_module` is a call of the host's, which HOST-153 and `FinishError` cover). The requirement is HOST-224 (decided), which `story:modules-design-text` writes into `host-interface.md` section 9 after HOST-68 (line 975 on the base) with the line `HOST-224 pending slice 11` of `conformance/host/requirements.txt`; this story moves that line to its test.

The count. 6 cases (6 runs, all module runs) are this story's (`docs/plan/slice-11/story-import-meta.cases.txt`: `language/expressions/import.meta/` that evaluate it: an ordinary object, the same object at each evaluation in one module and a different one in another, a `SyntaxError` from direct `eval` in a module, and the two `syntax/goal-module` cases, which run), each failing on the base on "a module run is not made before slice 11". The early errors of `import.meta` are `story:import-call-and-import-meta-syntax`'s.

Design it follows: ECMA-262 13.3.12.1 (`#sec-meta-properties-runtime-semantics-evaluation`: GetActiveScriptOrModule, [[ImportMeta]] empty, OrdinaryObjectCreate(null), HostGetImportMetaProperties, CreateDataPropertyOrThrow for each, HostFinalizeImportMeta), `#sec-hostgetimportmetaproperties`, `#sec-hostfinalizeimportmeta`; `host-interface.md` section 9, decision 6 (misuse ends in a defined answer, a handle the host returns is checked), decision 16 (the defaults, HOST-158, line 2020); HOST-68 (line 975); HOST-211 (line 2153: every hook call is charged one step before it is made); HOST-224; the paragraph `**Module records.**` of `engine.md` (the state of each part in a file of its own).

## What later stories reuse

Nothing of the engine; the instruction is used only by `import.meta`. The check of a returned handle is the form a later hook with a handle in its answer follows.

## Domain relations

- Module → its `import.meta` object: one to zero or one, made at the first evaluation in that module and kept in the record's `ImportMetaState` for the module's life — inferable (inferred from ECMA-262 `#sec-meta-properties-runtime-semantics-evaluation` step 3 and `#sec-cyclic-module-records`, field [[ImportMeta]] of Table 49; no ess/1 document declares it: `spec/domains/host.yaml` holds no engine value).

## Change

Failing tests first: the tests below are written before the code and fail on the base.

- `swell-syntax`: the hold `import.meta` goes: its `hold` call in `crates/syntax/src/parser/expression.rs` (the hunk of lines 1222 to 1230 that `story:import-call-and-import-meta-syntax` wrote) and its line of `HOLDS` (`crates/syntax/src/parser/mod.rs`, the fourth line; the attribute `#[rustfmt::skip]` above the array stays). The held source `module-import-meta.js` of order 2 then parses as a valid module does; this story writes no file of `crates/syntax/tests/`.
- `swell-compiler`: `Instruction::ImportMeta` after `AsyncDelegateYield` (`crates/compiler/src/bytecode.rs` line 978 on the base), its line in `crates/compiler/src/disasm.rs` (line 396), and the arms of `Expr::ImportMeta` in `crates/compiler/src/expression.rs` (beside line 342) and `crates/compiler/src/resolve.rs` (beside line 1746), the two that `story:import-call-and-import-meta-syntax` left answering `CompileError::Tree`, compile to it; a new source `crates/compiler/tests/sources/module-import-meta.js` with its `.disasm`.
- `swell`: the arm of `Instruction::ImportMeta` in the dispatch (`crates/swell/src/interp/dispatch.rs`, after the last arm, line 653 on the base), calling the function of `crates/swell/src/module/import_meta.rs` (the file the records story created, with `ImportMetaState` empty): `ImportMetaState` holds the object once made, with its trace; the function reads the running frame's ScriptOrModule component (the records story's; a module, since the grammar admits `import.meta` in module code only) and answers the module's `import.meta` object, made at the first evaluation: one step charged, then `host_get_import_meta_properties` through `HookCx`; each key and value of its `Ok` vector checked as every returned handle is (decision 6: a stale or foreign one ends the entry with `Terminated(EngineFault)`, HOST-224); a `Throw` propagated as the draft's `?`; the properties defined, charged to the memory budget; one step charged and `host_finalize_import_meta`; the object kept in `ImportMetaState`.
- Tests in a new module `crates/swell/tests/import_meta.rs`, 5, each in both collection modes: `host_68_the_first_import_meta_calls_both_hooks_once` (a host that counts its calls and answers one property: the object has it, a second evaluation calls neither hook), `host_224_a_stale_or_foreign_handle_from_import_meta_properties_faults_the_agent` (a key with a released `StringRef`, then a value with an `ObjectRef` of another agent: `Terminated(EngineFault)`, `state()` reads `Faulted`, nothing panics), `a_throw_from_import_meta_properties_is_thrown_by_import_meta`, `each_module_has_its_own_import_meta`, `import_meta_charges_a_step_for_each_hook_call`. `crates/swell/tests/hooks.rs`, 1 new: `host_158_import_meta_hooks_left_as_provided_give_the_defaults` (its module documentation, line 4, changes with it).
- Rules (the `AGENTS.md` of the folder that holds the repository, and `AGENTS.md`, "Rules" and "Commands and the gate"): no `unsafe` (`#![forbid(unsafe_code)]`), no third-party crate, no panic, no `unwrap`, `expect` or unchecked indexing in library code; every loop of the change (a walk of a module graph, of a list of entries, of a namespace's names) is charged to the step budget and every allocation to the memory budget, and a new limit gets a test that reaches it; a builtin's module and comments cite its clause. Time ("Tests stay fast"): a single test is terminated at 60 s and counts as failed, and the gate fails when the wall time of its test steps passes 120 s; the unit times each test before it adds it and names in its report every test over 5 s with its time; one integration-test binary per crate (a new test file is a module of the crate's `tests/main.rs`, written again by `sh taskfiles/test-main.sh write`); an expensive product several tests read (a parsed corpus, a fixture tree, a built part of the test262 checkout) is made once, as a committed file or one shared fixture, never per test and never in a `static`; no test makes a network request or a model call. Builds and tests go through the pool: `cargo` is the shim of the pool, anything else that compiles or tests is prefixed `slot`; a pool job ends after 10 minutes unless it names its time with `slot --timeout` or `SLOT_TIMEOUT`, at most 30 minutes; a script that only measures, probes or waits starts without the `slot` prefix. No Python in anything committed (tools, tasks, gates, tests, helpers): Rust or POSIX sh. The unit runs the tests of its own change, never the whole gate and never a whole-workspace test run; intermediate files go in the scratch directory; no `cd ... &&` with a write or a delete.
- Report: the sum of the CPU seconds of the story's new tests in the default mode and in the mode `collect-every-safe-point` (each test run alone, user plus system), and the wall seconds of its part run when it has one; no story bounds the CPU seconds of a whole gate.
- Generated files: the generated `tests/main.rs` of every crate and `website/data/status.json` are in no story's scope. A unit that adds a test module writes `tests/main.rs` again with `sh taskfiles/test-main.sh write`; a unit that meets a conflict in a generated file takes either side and writes it again; the coordinator writes both again on the join (`sh taskfiles/test-main.sh write`, `slot --local task site-data`).
- Line numbers are those of `f962c324`; anchors are found by name.
- Size: about 450 lines (the instruction and the evaluation 150, the tests 300): one unit.

Shared with the other stories of order 5 (`story:module-namespace-objects`, `story:top-level-await`, `story:json-modules`), each in its own hunks: `crates/syntax/src/parser/mod.rs`, `HOLDS` (this story deletes the fourth line; `story:module-namespace-objects` and `story:top-level-await` the first and second, not adjacent to it); `crates/syntax/src/parser/expression.rs` (this story: the hunk of lines 1222 to 1230; `story:top-level-await`: the hold in `prefix_operators`, near line 771); `crates/compiler/src/expression.rs` (this story: the arm `Expr::ImportMeta` near line 342; `story:top-level-await`: the `await` check near line 1233); `crates/compiler/src/resolve.rs` (this story: the arm `Expr::ImportMeta` beside line 1746, in `Resolver::expr`; `story:top-level-await`: the activation slot in `resolve_module_with`, which `story:module-code-compiles` placed after `resolve_with` (line 699 of the base, before `impl Resolver` at line 701): more than a thousand unchanged lines lie between the two hunks, so no edge is needed); `crates/swell/src/interp/` (this story: one arm after the last of `dispatch.rs`; `story:top-level-await`: `async_function.rs`); `conformance/host/requirements.txt` (this story: lines 68 and 158 and the line of HOST-224; `story:top-level-await` line 99, `story:json-modules` line 64). This story only in order 5: `crates/compiler/src/{bytecode,disasm}.rs`, `crates/swell/src/module/import_meta.rs`, `crates/swell/tests/hooks.rs`. `story:dynamic-import` (order 6) appends its instruction after this one and replaces the other arm of `Expr` left by order 2.

## Requirements

HOST-68 (`host-interface.md` line 975), HOST-158 (line 2020) and HOST-224 (the line `story:modules-design-text` adds): their 3 lines of `conformance/host/requirements.txt` move from `pending slice 11` to `HOST-68 test swell::import_meta::host_68_the_first_import_meta_calls_both_hooks_once`, `HOST-158 test swell::hooks::host_158_import_meta_hooks_left_as_provided_give_the_defaults` and `HOST-224 test swell::import_meta::host_224_a_stale_or_foreign_handle_from_import_meta_properties_faults_the_agent`. For the coordinator: the lines HOST-68, HOST-158 and HOST-224 under `### Host interface` of `## [Unreleased]`; no new public name. The coordinator writes `website/data/status.json` again on the join.

Writes: `crates/syntax/src/parser/{mod,expression}.rs` (the hold), `crates/compiler/src/{bytecode,disasm,expression,resolve}.rs`, `crates/compiler/tests/sources/module-import-meta.{js,disasm}` (new), `crates/swell/src/interp/dispatch.rs` (one arm), `crates/swell/src/module/import_meta.rs`, `crates/swell/tests/import_meta.rs` (new), `crates/swell/tests/hooks.rs`, `conformance/host/requirements.txt` (3 lines). Checked against the code on `f962c324`: `Instruction` is matched without a wildcard in `disasm.rs` and `dispatch.rs` only, both written here. Not written: `crates/syntax/tests/`, `crates/swell/src/host/hooks.rs` (the hooks and their defaults exist), `crates/test262/` (the runner's host keeps the defaults), `docs/design/` (`story:modules-design-text`), `spec/`.

Order 5, beside `story:module-namespace-objects`, `story:top-level-await` and `story:json-modules`; it depends on `story:module-linking-and-evaluation` (and through it on `story:import-call-and-import-meta-syntax`, whose node and hold it completes) and `story:modules-design-text` (HOST-224).

## Acceptance

`cargo nextest run --locked -p swell --test main -E 'test(/^import_meta::/)'` runs 5 tests (none on the base: the module does not exist) and `-E 'test(/^hooks::/)'` runs 18 (17 on the base), each exiting 0 in both collection modes; `cargo nextest run --locked -p swell --test main -E 'test(/^modules::/) | test(/^misuse::/)'` exits 0 in both collection modes; `cargo nextest run --locked -p swell-syntax --test main`, `cargo nextest run --locked -p swell-compiler --test main` and `cargo nextest run --locked -p swell --lib` exit 0. `ls crates/compiler/tests/sources/module-import-meta.js crates/compiler/tests/sources/module-import-meta.disasm | wc -l` prints `2` and `ls crates/compiler/tests/sources | wc -l` prints `288` (`286` on the join the unit starts from, `268` on the base). `grep -c '"import.meta"' crates/syntax/src/parser/mod.rs` prints `0` (`1` on that join); `grep -cE '^HOST-(68|158|224) pending slice 11$' conformance/host/requirements.txt` prints `0` (`3` on that join) and `grep -cE '^HOST-(68|158|224) test ' conformance/host/requirements.txt` prints `3`; `slot --timeout 30m task -d . -t taskfiles/host-requirements.yml host-requirements-check` exits 0. `slot task clippy` exits 0.

test262. The full run is not the unit's. In one pool job on the build node: `slot --timeout 30m --fetch .out/test262-result sh docs/plan/slice-11/run-part.sh docs/plan/slice-11/story-import-meta.cases.txt`; then on the workstation, with the base run of the join the unit starts from copied to `.out/test262-base/runs.tsv`, `sh docs/plan/slice-11/check.sh docs/plan/slice-11/story-import-meta.cases.txt .out/test262-base/runs.tsv .out/test262-result/runs.tsv` exits 0 and prints `passed 6 bound 6 B 0 F 6 R 0` (the part is the 6 cases of the list, all failing on the base, and pulls in none by `./`; the line is the same on `f962c324` and on the join).

## Out of scope

The early errors of `import.meta` (`story:import-call-and-import-meta-syntax`); `import.meta` properties a real host gives (`url`, `resolve`: the host's, HTML's); the command-line host's hooks (they keep the defaults; `story:cli-module-loader`).
