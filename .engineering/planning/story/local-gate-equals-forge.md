---
format: aep.planning-md/3
id: story:local-gate-equals-forge
kind: story
status: active
title: One pinned compiler, the runner's differences as a task, planning changes skip the tests
relations:
- depends_on: story:gate-test-steps-within-budget
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T21:45:40Z", actor: "human:operator", revision: 2, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-21T21:45:40Z", actor: "human:operator", revision: 3, executor: "agent:coordinator"}
---
## Context

Two rules for every repository of the organization, set on 2026-11-21 (the `AGENTS.md` of
the folder that holds this repository: "The local gate equals the forge" and "Planning-only
pull requests skip the tests"). Each is built as a story ahead of the next feature.

What swell has today:

- The forge's workflow names the compiler itself (`.github/workflows/check.yml`:
  `toolchain: "1.99.1"`); there is no `rust-toolchain.toml`, so the local gate uses whatever
  compiler the machine has. On 2026-11-21 both were 1.99.1.
- Twice on 2026-11-21 a commit passed the local gate and failed on the forge with a stack
  overflow in a stack test (61 and 59 minutes of blocked merges): the forge built with full
  debug information and the frame of the run loop was 19,216 bytes there against 18,432
  locally. Since then `.cargo/config.toml` sets the dev profile as the forge builds it, the
  stack tests run on seven eighths of `Limits::min_stack_bytes()`, and the frame of the run
  loop is 336 bytes (`docs/plan/slice-02-run-loop-frame.md`). What the runner sets beyond that
  (`CI`, `GITHUB_ACTIONS`, `CARGO_TERM_COLOR`, `CARGO_INCREMENTAL`, no `RUSTC_WRAPPER`) is
  known to the coordinator as a command line ("runner form") and written in `AGENTS.md`, not
  in a task.
- Every push to `main` and every pull request runs the whole gate, also when the change holds
  only planning files or documents. Of the commits on `main` after release 0.2.0, three
  changed documents only.

## Change

1. **One pinned compiler.** `rust-toolchain.toml` at the root pins the channel the workflow
   names today, with the `clippy` component; the workflow no longer names a version and
   installs what the file says (keep the action pinned by its hash; if the pinned action
   cannot read the file, the workflow reads the channel from the file in a step of its own
   and passes it on). `conformance/ess`, a workspace of its own, builds with the same file
   (say how: a file there, or the root file found from it). A fast check of the gate fails
   when the workflow names a compiler version or when `rustc --version` in the gate is not
   the pinned one, with both versions in its line.
2. **The runner's differences are a task.** `task check-runner` runs `check` with the
   variables the forge's runner sets, read from one place that the workflow also uses or that
   a check compares with the workflow (`CI`, `GITHUB_ACTIONS`, `CARGO_TERM_COLOR`,
   `CARGO_INCREMENTAL`, an empty `RUSTC_WRAPPER`). The task's summary lists each difference
   between the runner and a developer's shell that the repository knows, and where it is
   written: the dev profile (`.cargo/config.toml`), the environment (this task), the thread
   stack of the stack tests (`crates/swell/tests/support/host.rs`), the files that exist only
   in a local tree (`.cache/`, `.out/`, `target/`; a test that reads the tree reads tracked
   files only: search the tests for one that does not, and report it).
   **The debug information of the dev profile, measured.** `.cargo/config.toml` sets
   `debug = true` so that the local build has the frames of the forge's. A cargo config file
   wins over a manifest, and a developer's own cargo config may set another level, which is
   why the setting is in the repository's config file and stays there. Measure, on one test
   binary, with `readelf -S`: its size and the size of its `.debug_*` sections under `true`,
   `"line-tables-only"` and `false`, and the wall time of a clean build of the test binaries
   under each (one `cargo clean` between them, each build in the background). If a lower
   level saves more than a third of the size, report it with the three rows and the bytes
   per re-entry the stack tests would then see (the measuring harness of
   `docs/plan/slice-02-run-loop-frame.md`); do not change the level: the constant
   `STACK_BYTES_PER_REENTRY` was measured under `true`, and the coordinator decides with the
   numbers.
3. **Planning and prose changes skip build and test on the forge.** The required job of
   `check.yml` still runs on every pull request and every push to `main`. Its first step
   lists the changed files (for a pull request against its base; for a push against the
   commit before it; a first push or a history it cannot read counts as code). When every
   changed file is a planning or prose file, the job prints one line that says so with the
   count, runs the checks that read such files (`python-check`, the plan store's
   `aep plan artifact validate` if the gate has it, nothing that compiles) and exits 0;
   otherwise it runs `task check` as today. The list of such paths is one file in the
   repository, read by a script under `tests/gate/` or `taskfiles/` that the workflow calls,
   so that the same script runs locally: `.engineering/**` and `*.md`, without the files a
   gate step reads as data. Find those by reading every task: `docs/design/engine.md` (the
   block `swell-slices` and the count table are read by the runner),
   `docs/design/host-interface.md` (the requirements), `AGENTS.md` and `README.md` if a check
   reads their numbers, and any other. Each of them counts as code; name each in the report
   with the task that reads it. `website/` and other built sites are not prose for this
   purpose.
4. **Planted defects**, each a script under `tests/gate/` that runs in under 5 s, written
   and seen red before its check: a workflow that names a compiler version is refused; a
   change of one `.engineering/` file and one `docs/plan/*.md` file is classified as
   planning-only; the same change plus one `crates/` file is classified as code; a change of
   `docs/design/engine.md` alone is classified as code; a variable of the runner form that
   the workflow sets and the task does not is refused.

## Constraints

- No ruleset, branch protection or required-check setting is read or changed: they are the
  operator's. The job that is required today keeps its name (`swell / task check`).
- `.github/workflows/shared-gates.yml` is not touched. Every action stays pinned by its full
  commit hash.
- No network request from the unit. Nothing third-party in what ships; no engine code
  changes.
- The gate's test steps stay within the budget of `story:gate-test-steps-within-budget`
  (120 s, a test killed at 60 s); every new script is timed, and one over 5 s is named.
- `AGENTS.md`, `README.md`, `docs/releasing.md` and `docs/plan/` are the coordinator's: list
  the lines to change.

Writes: `rust-toolchain.toml` (new), `.github/workflows/check.yml`, `.github/workflows/slow.yml`
(the compiler pin only), the root `Taskfile.yml`, `taskfiles/` (new files and the gate
variables), `tests/gate/` (new scripts), one new file for the path list (you name it),
`conformance/ess/rust-toolchain.toml` if point 1 needs it.

## Acceptance

On the unit's branch:

1. `rustup show active-toolchain` in the root and in `conformance/ess` prints the pinned
   channel, and `grep -c 'toolchain: "' .github/workflows/*.yml` prints `0` for each file.
2. `slot task check-runner` exits 0 and prints the same counted lines as `slot task check`.
3. The classifier, run on the four changes of point 4, prints `planning-only`, `code`, `code`
   and, for the compiler pin, the refusal; the report quotes each line.
4. `gate-selftest` passes with the scripts of today plus the five of point 4, and its
   `gate-time` line stays within the budget.
5. The five planted scripts each ran red before their check existed; the report quotes the
   red line of each.
