---
format: aep.planning-md/3
id: story:module-determinism-and-emulated-loading
kind: story
status: draft
title: Determinism over a corpus of modules, and the emulated host loads modules from memory
summary: A fixed corpus of modules run by two agents in one process and by two child processes with byte-equal transcripts (HOST-159); EmulatedHost answers the load hook from an in-memory map. Order 6; no case.
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
  path: crates/swell/tests/determinism.rs
- confidence: cited
  path: crates/swell/tests/emulated_host.rs
revision: 5
---
## Context

Slice 11 brings modules (`epic:slice-11-modules`; `docs/design/engine.md`, section 12, item 11). This story holds the host interface's determinism for modules (HOST-159, `docs/design/host-interface.md` decision 19, line 2198 on the base) and gives the emulated host its third part, loading modules from memory. On the base (`f962c324`) `crates/swell/tests/determinism.rs` runs a fixed corpus of scripts (HOST-133) by two agents in one process, their calls interleaved, and by two child processes (the test binary started again with `SWELL_DETERMINISM_CHILD` set), and compares the transcripts byte for byte; its host answers `host_load_imported_module` with the default. `crates/swell/tests/emulated_host.rs` (lines 9 and 10: "Module loading from memory, the third part of such a host, is slice 11 and not delivered: the load hook keeps nothing", and line 49) is the emulated adapter `engine.md` names at line 282 ("modules from memory once slice 11 delivers loading").

It has no case of test262; its acceptance is the tests of the host interface.

Design it follows: `host-interface.md` decision 19 (HOST-133, HOST-159: "the same holds for a fixed corpus of modules: two agents created with the same limits and seeds, given the same calls with the same arguments by hosts whose hooks answer the same values, make the same sequence of hook calls with the same arguments and return the same outcomes, in one process and across two processes"); decision 1, "Adapters"; `engine.md` section 2, "The engine core has no `std`" (line 276).

## What later stories reuse

The corpus of modules and the emulated host's loader; `story:slice-11-conformance-close` reads nothing of them.

## Domain relations

- None new: the host's map from a specifier to a source is the test host's own, not the engine's.

## Change

Failing tests first: the tests below are written before the code and fail on the base.

- `crates/swell/tests/determinism.rs`: a corpus of modules, a constant of the file beside the scripts' corpus (no file read at run time): a graph of five modules with a cycle, named and namespace imports, `export * as`, a module with top-level `await` on a promise the graph resolves, `import.meta` read twice, a JSON module, and a module whose link fails; `Transcript` (line 27) answers `host_load_imported_module` from that corpus, finishing each load inside the hook, and records the call with its specifier, attributes and referrer; `host_get_import_meta_properties` answers one fixed property. The test `host_159_a_corpus_of_modules_makes_the_same_calls_in_one_process_and_across_two` runs the corpus as the scripts' test does (`parse_module`, `load_requested_modules`, `link`, `evaluate_module`, a checkpoint, for each root module), by two agents interleaved and by two child processes through `spawn_child` (line 744), and compares the transcripts, `last_entry_steps()`, `heap_bytes()` and collection reports byte for byte. Its child mode reads the same variable as the scripts' test, with the test path of its own.
- `crates/swell/tests/emulated_host.rs`: `EmulatedHost` keeps a map from specifier to source in memory and answers `host_load_imported_module` by `cx.parse_module` and `cx.finish_loading_imported_module` (a missing specifier finished with a thrown `TypeError` the host makes); its module documentation (lines 9 and 10) changes with it; tests `the_emulated_host_loads_a_graph_from_memory` (two modules, the value of an import read through a host function) and `a_missing_module_of_the_emulated_host_rejects_the_graph`. `the_module_names_no_system_api` still passes over the changed file (it reads the file with the scanners of HOST-2).
- Rules (the `AGENTS.md` of the folder that holds the repository, and `AGENTS.md`, "Rules" and "Commands and the gate"): no `unsafe` (`#![forbid(unsafe_code)]`), no third-party crate, no panic, no `unwrap`, `expect` or unchecked indexing in library code; every loop of the change (a walk of a module graph, of a list of entries, of a namespace's names) is charged to the step budget and every allocation to the memory budget, and a new limit gets a test that reaches it; a builtin's module and comments cite its clause. Time ("Tests stay fast"): a single test is terminated at 60 s and counts as failed, and the gate fails when the wall time of its test steps passes 120 s; the unit times each test before it adds it and names in its report every test over 5 s with its time; one integration-test binary per crate (a new test file is a module of the crate's `tests/main.rs`, written again by `sh taskfiles/test-main.sh write`); an expensive product several tests read (a parsed corpus, a fixture tree, a built part of the test262 checkout) is made once, as a committed file or one shared fixture, never per test and never in a `static`; no test makes a network request or a model call. Builds and tests go through the pool: `cargo` is the shim of the pool, anything else that compiles or tests is prefixed `slot`; a pool job ends after 10 minutes unless it names its time with `slot --timeout` or `SLOT_TIMEOUT`, at most 30 minutes; a script that only measures, probes or waits starts without the `slot` prefix. No Python in anything committed (tools, tasks, gates, tests, helpers): Rust or POSIX sh. The unit runs the tests of its own change, never the whole gate and never a whole-workspace test run; intermediate files go in the scratch directory; no `cd ... &&` with a write or a delete.
- Report: the sum of the CPU seconds of the story's new tests in the default mode and in the mode `collect-every-safe-point` (each test run alone, user plus system), and the wall seconds of its part run when it has one; no story bounds the CPU seconds of a whole gate.
- Generated files: the generated `tests/main.rs` of every crate and `website/data/status.json` are in no story's scope. A unit that adds a test module writes `tests/main.rs` again with `sh taskfiles/test-main.sh write`; a unit that meets a conflict in a generated file takes either side and writes it again; the coordinator writes both again on the join (`sh taskfiles/test-main.sh write`, `slot --local task site-data`).
- Line numbers are those of `f962c324`; anchors are found by name.
- The test of HOST-159 starts two child processes, as the scripts' test does: the unit measures its time alone (wall and CPU) before it is added, and it stays under 5 s of wall time; the corpus is a constant, parsed by each agent as the test's subject, not a shared product.
- Size: about 450 lines (the corpus 150, the hosts 100, the tests 200): one session.

Shared with the other stories of order 6 (`story:dynamic-import`, `story:ess-target-module-commands`, `story:cli-module-loader`): `conformance/host/requirements.txt` (this story: line 159; `story:dynamic-import`: lines 55, 65, 66, none adjacent to it); no other file (this story adds no test module, both test files exist).

## Requirements

HOST-159 (`host-interface.md` line 2198): line 159 of `conformance/host/requirements.txt` moves from `pending slice 11` to `HOST-159 test swell::determinism::host_159_a_corpus_of_modules_makes_the_same_calls_in_one_process_and_across_two`. For the coordinator: the line HOST-159 under `### Host interface` of `## [Unreleased]`. The coordinator writes `website/data/status.json` again on the join.

Writes: `crates/swell/tests/determinism.rs`, `crates/swell/tests/emulated_host.rs`, `conformance/host/requirements.txt` (line 159). Not written: every file of `crates/swell/src/` (a difference between the two transcripts is a defect of the engine: the unit reports it with the first line that differs, and the coordinator gives it back to the story that owns the part), `crates/test262/`, `spec/`, `docs/design/`.

Order 6, beside `story:dynamic-import`, `story:ess-target-module-commands` and `story:cli-module-loader`; it depends on `story:module-namespace-objects`, `story:top-level-await`, `story:import-meta` and `story:json-modules` (the corpus uses each part).

## Acceptance

`cargo nextest run --locked -p swell --test main -E 'test(/^determinism::/)'` runs 4 tests (3 on the base) and `-E 'test(/^emulated_host::/)'` runs 7 (5 on the base), each exiting 0 in both collection modes: 1 new test in `determinism::` and 2 in `emulated_host::`. `grep -c '^HOST-159 pending slice 11$' conformance/host/requirements.txt` prints `0` (`1` on the join the unit starts from) and `grep -c '^HOST-159 test swell::determinism::' conformance/host/requirements.txt` prints `1`; `slot task -d . -t taskfiles/host-requirements.yml host-requirements-check` exits 0. `git log --no-merges --first-parent --format= --name-only "$BASE"..HEAD -- crates/swell/src` prints nothing (`BASE` the join the unit starts from: the unit's own commits write no engine file, whatever a merge of the wave branch brings). `slot task clippy` exits 0. Time: the report names the wall and CPU time of `host_159_...` alone.

test262: no case moves; the story writes no file the runner reads.

## Out of scope

`import()` in the corpus (`story:dynamic-import` holds its own tests; a module corpus with `import()` is a later extension of this test, not this story's); the command-line host's loader (`story:cli-module-loader`); determinism of the clock (slice 4) and of weak references (slice 16).
