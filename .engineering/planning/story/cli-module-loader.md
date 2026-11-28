---
format: aep.planning-md/3
id: story:cli-module-loader
kind: story
status: draft
title: The command-line host runs a module file and its static imports from disk
summary: 'swell --module <file>: the shell''s load hook reads ./ and ../ specifiers beside the referring file, one module per path, every load finished inside the hook; top-level await through the checkpoint the shell performs; import() and JSON modules in the shell named for a later slice. Order 6; no case.'
relations:
- decomposes: epic:slice-11-modules
- depends_on: story:module-namespace-objects
- depends_on: story:top-level-await
- depends_on: story:import-meta
scope:
- confidence: cited
  path: crates/cli/src/args.rs
- confidence: cited
  path: crates/cli/src/main.rs
- confidence: cited
  path: crates/cli/src/shell.rs
- confidence: inferred
  path: crates/cli/tests/fixtures/modules
- confidence: cited
  path: crates/cli/tests/shell.rs
revision: 3
---
## Context

`docs/design/engine.md`, section 9, lists among what swell proves without surf "scripts and modules from files through the command-line host" (line 3495), and the epic says "The command-line host and the runner provide their own loaders" (its "Out of scope", the bullet on a real module resolver). The runner's loader is `story:module-records-loading-and-module-runs`'s; this story is the command-line host's. On the base (`f962c324`) the shell (`crates/cli/src/shell.rs`) runs one script from a file or from `-e` (`crates/cli/src/main.rs`, `args.rs`), then performs the microtask checkpoint once (`evaluate`, line 128: "Runs `text` as a script with `limits`, then the promise jobs it queued"); its host keeps "the generic, timeout and cleanup jobs and the module loads the engine hands over ... and never performed: the shell has no event loop and no module loader" (`ShellHost`, lines 38 to 44, its field `loads: Vec<LoadRequest>`).

What of a module graph the shell can then run. A graph without top-level `await` needs no job: loading (every load finished inside the hook), `link` and `evaluate_module` settle the promise of the evaluation before the call returns. A graph with top-level `await` needs promise jobs, which the shell already performs: the checkpoint after the evaluation runs them (slice 10), and nothing else is needed. The smaller scope, taken here: `swell --module <file>` runs a module file and its static imports from disk, with top-level `await` through the checkpoint the shell performs today. Out of scope and where it goes: `import()` from a script or a module of the shell and JSON modules in the shell (they need the shell to answer a load whose referrer is a script, and the attribute `type`; the coordinator files them as an entry of `docs/plan/waves.md`, "What later slices take over", at the close of slice 11, named in `story:slice-11-conformance-close`); generic and timeout jobs (slice 14), which the shell keeps and never performs.

It has no case of test262; its acceptance is the tests of the command-line host on files of the repository.

Line numbers in this story are those of `f962c324`; anchors are found by name.

## What later stories reuse

Nothing in this slice; the entry named above extends the loader to `import()` and JSON modules.

## Domain relations

- A module the shell loads → its file: one to one, by the path resolved against the directory of the referring module's file — inferable (inferred from `docs/design/engine.md` line 3495 and the runner's rule of decision 11, "`./name` is the file beside the case", which this host follows for files; no ess/1 document declares a host's loader: `spec/domains/host.yaml` states the hand-over, `swell.host.ModuleLoad`, not how a host finds a source).

## Change

Failing tests first: the tests and fixtures below are written before the code and fail on the base (no `--module`).

- `crates/cli/src/args.rs`: the flag `--module` (a run of `<file>` as a module; refused with `-e`, a usage error 64), its line in `USAGE` (lines 25 to 60), and `Script::ModuleFile(PathBuf)` beside `Script::File`. `crates/cli/src/main.rs`: a module run reads no file itself; it hands the path to the shell, which reads through its host.
- `crates/cli/src/shell.rs`: `ShellHost` gets the directory of the root module and a map from a canonical path to the `Module` already loaded in this run (one module per file, so a cycle loads each file once); `host_load_imported_module` (line 59) resolves a specifier that starts with `./` or `../` against the directory of the referring module's file, reads the file, parses it with `cx.parse_module` and finishes the load inside the hook with `cx.finish_loading_imported_module`; a bare specifier, an unreadable file or a file that is not UTF-8 finishes the load with a thrown `TypeError` naming the specifier; a request whose referrer is a script or a realm, or with an attribute, keeps the request as today (out of scope above; its promise stays pending, and the shell says nothing of it). `run` of a module: `parse_module` (its errors exit 2, as a script's), `load_requested_modules`, the checkpoint, the promise's state (rejected: exit 1 with the thrown value's message, as an uncaught throw), `link` (a throw: exit 1), `evaluate_module`, the checkpoint, the promise's state (rejected: exit 1; pending after the checkpoint, a module whose `await` waits for nothing the shell performs: exit 1 with the message `the module's evaluation did not settle`); the exit statuses of `USAGE` are unchanged. The module documentation of `shell.rs` (lines 38 and 39) changes with it.
- Fixtures `crates/cli/tests/fixtures/modules/` (new): `main.mjs` (imports `./lib/value.mjs` and prints what it imports), `lib/value.mjs` (imports `../cycle.mjs`), `cycle.mjs` (imports `./lib/value.mjs`), `tla.mjs` (`await Promise.resolve(); print("after await");`), `missing.mjs` (imports `./absent.mjs`), `unresolved.mjs` (imports a name `./lib/value.mjs` does not export), `syntax.mjs` (`export default 1; export default 2;`): 7 files.
- Tests in `crates/cli/tests/shell.rs`, 6 new: `a_module_file_runs_with_its_imports_and_a_cycle` (exit 0, the printed line), `top_level_await_in_a_module_runs_through_the_checkpoint` (exit 0, `after await`), `a_missing_import_exits_1_and_names_the_specifier`, `an_import_of_a_missing_export_exits_1_with_a_syntax_error`, `a_module_syntax_error_exits_2`, `module_with_dash_e_is_a_usage_error`; and `help_exits_0_and_names_every_flag` (line 229) gets `--module`.
- Rules (the `AGENTS.md` of the folder that holds the repository, and `AGENTS.md`, "Rules" and "Commands and the gate"): no `unsafe` (`#![forbid(unsafe_code)]`), no third-party crate, no panic, no `unwrap`, `expect` or unchecked indexing in library code; every loop of the change (a walk of a module graph, of a list of entries, of a namespace's names) is charged to the step budget and every allocation to the memory budget, and a new limit gets a test that reaches it; a builtin's module and comments cite its clause. Time ("Tests stay fast"): a single test is terminated at 60 s and counts as failed, and the gate fails when the wall time of its test steps passes 120 s; the unit times each test before it adds it and names in its report every test over 5 s with its time; one integration-test binary per crate (a new test file is a module of the crate's `tests/main.rs`, written again by `sh taskfiles/test-main.sh write`); an expensive product several tests read (a parsed corpus, a fixture tree, a built part of the test262 checkout) is made once, as a committed file or one shared fixture, never per test and never in a `static`; no test makes a network request or a model call. Builds and tests go through the pool: `cargo` is the shim of the pool, anything else that compiles or tests is prefixed `slot`; a pool job ends after 10 minutes unless it names its time with `slot --timeout` or `SLOT_TIMEOUT`, at most 30 minutes; a script that only measures, probes or waits starts without the `slot` prefix. No Python in anything committed (tools, tasks, gates, tests, helpers): Rust or POSIX sh. The unit runs the tests of its own change, never the whole gate and never a whole-workspace test run; intermediate files go in the scratch directory; no `cd ... &&` with a write or a delete.
- Report: the sum of the CPU seconds of the story's new tests in the default mode and in the mode `collect-every-safe-point` (each test run alone, user plus system), and the wall seconds of its part run when it has one; no story bounds the CPU seconds of a whole gate.
- Generated files: the generated `tests/main.rs` of every crate and `website/data/status.json` are in no story's scope. A unit that adds a test module writes `tests/main.rs` again with `sh taskfiles/test-main.sh write`; a unit that meets a conflict in a generated file takes either side and writes it again; the coordinator writes both again on the join (`sh taskfiles/test-main.sh write`, `slot --local task site-data`).
- Line numbers are those of `f962c324`; anchors are found by name.
- The shell is a host with system access (decision 1, "Adapters"): it reads files; the engine reaches none. The tests start the binary (`CARGO_BIN_EXE_swell`) as the existing tests of `tests/shell.rs` do; each runs one small file.
- Size: about 400 lines (the loader and the run 200, the arguments 30, the tests and fixtures 170): one session.

Shared with the other stories of order 6 (`story:dynamic-import`, `story:ess-target-module-commands`, `story:module-determinism-and-emulated-loading`): no file; `crates/cli/` is this story's alone in the slice.

## Requirements

None of `conformance/host/requirements.txt` moves.

Writes: `crates/cli/src/{args,main,shell}.rs`, `crates/cli/tests/shell.rs`, `crates/cli/tests/fixtures/modules/` (new, 7 files). Not written: `crates/swell/` (the entry points exist), `crates/test262/`, `docs/design/`, `spec/`.

Order 6, beside `story:dynamic-import`, `story:ess-target-module-commands` and `story:module-determinism-and-emulated-loading`; it depends on `story:module-namespace-objects`, `story:top-level-await` and `story:import-meta` (a module file of a user may use each; `import()` and JSON modules are out of scope).

## Acceptance

`cargo nextest run --locked -p swell-cli --test main` exits 0 and runs 22 tests (16 on `f962c324`, `grep -c '#\[test\]' crates/cli/tests/shell.rs` prints `16` there and `22` after); `find crates/cli/tests/fixtures/modules -type f | wc -l` prints `7`, and `0` on the base, where the directory does not exist), and `swell --help` names `--module` (the test above). `slot task clippy` exits 0.

test262: no case moves; the story writes no file the runner reads.

## Out of scope

`import()` and JSON modules in the shell (the entry of `waves.md` named in Context, written at the close); an event loop, generic and timeout jobs (slice 14); a resolver of URLs or packages (the host's own choice, not this shell's).
