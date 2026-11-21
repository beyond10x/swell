---
format: aep.planning-md/3
id: story:workspace-and-gate
kind: story
status: implemented
title: A Cargo workspace with an engine that runs nothing, and task check as the one gate
summary: Root Cargo.toml, Cargo.lock, clippy.toml and Taskfile.yml with five optional includes; crates/swell with no public item; check refuses a compiler warning, a denied clippy lint, an outside package, a crate without forbid(unsafe_code) and a Python file, each shown by a planted script.
relations:
- decomposes: epic:foundation
scope:
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: Cargo.toml
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: clippy.toml
- confidence: cited
  path: crates/swell
- confidence: cited
  path: tests/gate/build-refuses-a-compiler-warning.sh
- confidence: cited
  path: tests/gate/clippy-refuses-an-unwrap-in-an-engine-crate.sh
- confidence: cited
  path: tests/gate/forbid-unsafe-check-refuses-a-crate-without-it.sh
- confidence: cited
  path: tests/gate/lock-check-refuses-an-outside-package.sh
- confidence: cited
  path: tests/gate/python-check-refuses-a-python-file.sh
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T03:15:03Z", actor: "human:operator", revision: 5, decided_on: {"recorded":{"review_outcome":3}}, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-21T03:15:04Z", actor: "human:operator", revision: 6, decided_on: {"recorded":{"review_outcome":3}}, executor: "agent:coordinator"}
- {from: "active", to: "implemented", at: "2026-11-21T04:04:13Z", actor: "human:operator", revision: 8, decided_on: {"recorded":{"test_result":1,"review_outcome":3,"verification":1}}, executor: "agent:coordinator"}
---
## Context

swell has a design and no code (`AGENTS.md`, "Status"). This story makes the root Cargo workspace,
the engine crate as an engine that runs nothing, and `task check` as the one gate, so that every
later story of `epic:foundation` adds one gate task and nothing else at the root.

What it carries out: decisions 2 and 14 of `docs/design/engine.md` (one Cargo workspace,
`crates/<name without the swell- prefix>/`, `crates/swell/` for the engine, Apache-2.0 declared by
every crate, no package from outside the repository in the root `Cargo.lock`, checked as surf
checks it), decision 13 (warnings denied; clippy `unwrap_used`, `expect_used`, `panic` and
`indexing_slicing` denied in the engine crates; `overflow-checks = true` in every profile), and the
`AGENTS.md` rules `#![forbid(unsafe_code)]` in every crate and no Python file in the tree.

The engine that runs nothing is `crates/swell` with no public item: a library holding only
`#![forbid(unsafe_code)]`. The host interface is stated in `spec/` and
`docs/design/host-interface.md` before its code (decision 10, `AGENTS.md` "Rules"), and slice 1
writes its first items; nothing here stands in for them. The conformance runner does not call the
engine in this epic (`story:test262-counting-runner`).

How the gate is put together across this epic (the design chosen for the root files): this story
is the only one that writes the root `Taskfile.yml`, `Cargo.toml`, `Cargo.lock` and `clippy.toml`;
its brief assigns them, since no other unit of its wave touches them. The root `Taskfile.yml`
includes five files that later stories create, each with `flatten: true` and `optional: true`, so
the root file works before they exist: `taskfiles/test262-fetch.yml`, `taskfiles/test262.yml`,
`taskfiles/unicode-data.yml`, `taskfiles/spec.yml`, `taskfiles/host-requirements.yml`. Each later
story writes only its own include file and names the `- task:` line the coordinator adds to
`check` when it merges the story (task 3.54 fails a task that names a task no file defines;
measured with a scratch Taskfile, `slot task all` printed
`task: Failed to run task "all": task: Task "nothere" does not exist`). An include file runs alone
from the root with `task -d . -t taskfiles/<name>.yml <task>`, and its tasks then run in the root
directory (measured the same way: `pwd` printed the root).

Planted-defect tests of the gate are shell scripts, one per test, under `tests/gate/`:
`tests/gate/<test name>.sh`, run from the repository root with `sh`, working only under
`.out/gate-tests/<test name>/` (removed and made again at the start), printing
`<test name>: ok` and exiting 0 when the gate refuses the planted defect as it should, printing
`<test name>: FAIL <why>` and exiting 1 otherwise. The task `gate-selftest` runs every one of
them. Later stories add scripts there and touch nothing else. Each gate task takes the path it
checks as a variable with the real path as its default, so a script points it at a planted copy
instead of copying the Taskfile. A script that compiles calls `cargo`, which is the build pool
shim: it queues in the pool when run from a shell and runs directly inside a pool job such as
`slot task check` (the shim, `$HOME/.local/share/slot/shims/cargo`: "any command inside a slot
job (SLOT_JOB set)" runs directly), so the scripts never wait on the pool they run in.

clippy 1.99.1 accepts the test allowances below: a scratch crate with the four lints denied in
`[lints.clippy]` and `allow-unwrap-in-tests`, `allow-expect-in-tests`, `allow-panic-in-tests`,
`allow-indexing-slicing-in-tests` in `clippy.toml` passed `cargo clippy --all-targets -- -D warnings`
with an `unwrap()` and an index in a `#[cfg(test)]` module.

## Change

Failing tests first: the five scripts under `tests/gate/` are written before the tasks and the
crate. Before the change each prints `FAIL` (there is no task, no workspace to copy); after it
each prints `ok`.

- `Cargo.toml`: a virtual workspace, `members = ["crates/*"]`, `resolver = "3"`;
  `[workspace.package]` with `edition = "2024"`, `license = "Apache-2.0"`, `version = "0.0.0"`;
  `overflow-checks = true` under `[profile.dev]`, `[profile.release]`, `[profile.test]` and
  `[profile.bench]`; no `[workspace.dependencies]`, no `[patch]`.
- `clippy.toml`: `allow-unwrap-in-tests`, `allow-expect-in-tests`, `allow-panic-in-tests` and
  `allow-indexing-slicing-in-tests`, each `true`.
- `crates/swell/Cargo.toml` (package `swell`, no dependency, `license.workspace = true`,
  `edition.workspace = true`, `version.workspace = true`; `[lints.clippy]` `unwrap_used`,
  `expect_used`, `panic`, `indexing_slicing` each `"deny"`) and `crates/swell/src/lib.rs`, which
  holds `#![forbid(unsafe_code)]`, a crate documentation comment, and no item.
- `Cargo.lock`: written by `cargo generate-lockfile --offline`; one package, `swell`.
- `Taskfile.yml`: `version: '3'`, `set: [pipefail]`, the five optional includes above, and these
  tasks, each writing its log under `.out/` and printing the line given:
  - `python-check` (variable `GATE_ROOT`, default `.`): lists the files of `GATE_ROOT` with
    `git -C "$GATE_ROOT" ls-files --cached --others --exclude-standard`, so ignored paths (`.cache/`,
    `.out/`, `target/`) are not read (the test262 checkout tracks 52 `.py` files:
    `git -C .cache/test262/<commit> ls-files | grep -c '\.py$'`); a file whose name ends in `.py`,
    `.pyw` or `.pyi`, or whose first line starts with `#!` and contains `python`, is printed as
    `<path>: a Python file` and fails the task. Success:
    `python-check: no Python file among <n> file(s)`.
  - `forbid-unsafe-check` (variable `GATE_CRATES`, default `crates`): every crate root under
    `GATE_CRATES/*/` (`src/lib.rs`, `src/main.rs`, `src/bin/*.rs`) holds the line
    `#![forbid(unsafe_code)]`; one without it is printed as
    `<path>: no #![forbid(unsafe_code)]` and fails the task. Success:
    `forbid-unsafe-check: <n> crate root(s), each with #![forbid(unsafe_code)]`.
  - `lock-check` (variable `GATE_LOCK`, default `Cargo.lock`): as the `lock-check` task of surf (its
    root `Taskfile.yml`): a package with a `source = ` line is printed as
    `Cargo.lock: <name> <version> from <source>` and fails the task; a missing file fails.
    Success: `Cargo.lock: <n> package(s), none from outside this repository`.
  - `build`: `cargo build --workspace --all-targets --locked` with `RUSTFLAGS=-D warnings`.
  - `clippy`: `cargo clippy --workspace --all-targets --locked -- -D warnings`.
  - `test`: `cargo test --workspace --locked` with `RUSTFLAGS=-D warnings`, printing
    `<n> test suites: <p> passed, <f> failed, <i> ignored` summed from the `test result:` lines.
  - `gate-selftest`: runs `sh` on every `tests/gate/*.sh` in byte order, prints the line of each
    script, then `gate-selftest: <p> passed, <f> failed`, and fails when one failed.
  - `check`, in this order: `python-check`, `forbid-unsafe-check`, `lock-check`, `build`,
    `clippy`, `test`, `gate-selftest`.
- `tests/gate/lock-check-refuses-an-outside-package.sh`: a copy of `Cargo.lock` with one package
  that has `source = "registry+https://github.com/rust-lang/crates.io-index"`;
  `task -d . lock-check GATE_LOCK=<copy>` must exit non-zero and print `Cargo.lock: <that name>
  <version> from registry+https://github.com/rust-lang/crates.io-index`.
- `tests/gate/forbid-unsafe-check-refuses-a-crate-without-it.sh`: a planted `crates/` with two
  crates, one root without the line; the task with `GATE_CRATES=<planted>` must exit non-zero and
  name that file only.
- `tests/gate/python-check-refuses-a-python-file.sh`: a planted directory made a git repository
  with `git init -q`, holding `tool.py` and a file `run` whose first line is
  `#!/usr/bin/env python3`; the task with `GATE_ROOT=<planted>` must exit non-zero and name both.
- `tests/gate/clippy-refuses-an-unwrap-in-an-engine-crate.sh`: copies the root `Cargo.toml`,
  `Cargo.lock`, `clippy.toml` and `crates/swell/Cargo.toml` into its sandbox, writes
  `crates/swell/src/lib.rs` with `#![forbid(unsafe_code)]` and one public function that calls
  `unwrap()` outside any test, and runs the command of the `clippy` task there
  (`cargo clippy --workspace --all-targets --locked -- -D warnings`, in the sandbox, so its
  `target/` is the sandbox one); it must exit non-zero and its output hold the line clippy 1.99.1 prints for a lint denied by the manifest, ``requested on the command line with `-D clippy::unwrap-used` `` (clippy writes the lint with a hyphen there; measured by the unit on 2026-11-21).
  With the lints of `crates/swell/Cargo.toml` removed the same crate passes, which the script
  also runs, so the refusal comes from the lint configuration and not from something else.
- `tests/gate/build-refuses-a-compiler-warning.sh`: the same copy with a `lib.rs` holding a
  function with an unused variable, and the command of the `build` task
  (`RUSTFLAGS="-D warnings" cargo build --workspace --all-targets --locked`); it must exit
  non-zero and its output name `unused variable`.

Writes: `Cargo.toml`, `Cargo.lock`, `clippy.toml`, `Taskfile.yml`, `crates/swell/Cargo.toml`,
`crates/swell/src/lib.rs`, `tests/gate/lock-check-refuses-an-outside-package.sh`,
`tests/gate/forbid-unsafe-check-refuses-a-crate-without-it.sh`,
`tests/gate/python-check-refuses-a-python-file.sh`,
`tests/gate/clippy-refuses-an-unwrap-in-an-engine-crate.sh`,
`tests/gate/build-refuses-a-compiler-warning.sh`.

## Acceptance

In this story, in its own worktree, where none of the five optional include files exists,
`slot task check` exits 0 and prints `python-check: no Python file among <n> file(s)`,
`forbid-unsafe-check: 1 crate root(s), each with #![forbid(unsafe_code)]`,
`Cargo.lock: 1 package(s), none from outside this repository`, the `test` line with `0 failed`,
and the five lines `build-refuses-a-compiler-warning: ok`,
`clippy-refuses-an-unwrap-in-an-engine-crate: ok`, `forbid-unsafe-check-refuses-a-crate-without-it: ok`,
`lock-check-refuses-an-outside-package: ok`, `python-check-refuses-a-python-file: ok`. The run of
`slot task check` on the wave branch, with every script of the wave, belongs to the coordinator.

## Out of scope

Every other crate of decision 2 (each comes with the slice that needs it; `crates/test262` with
`story:test262-counting-runner`). Any public item of `swell`, any lexing, parsing or running of a
script, and any part of the host interface: slice 1, after `spec/` and
`docs/design/host-interface.md`. The tasks of the other foundation stories (test262, Unicode
data, specification, host requirements). Continuous integration, a remote, publishing.
`AGENTS.md` and `README.md` (the coordinator updates them when the gate first runs).
