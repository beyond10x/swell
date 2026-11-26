---
format: aep.planning-md/3
id: story:cli-runs-a-script-file
kind: story
status: implemented
title: A swell shell binary runs a script file
summary: crates/cli runs a file with print, a host clock and limit flags, and exits with a status per outcome
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-25T12:46:20Z", actor: "human:operator", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-25T12:46:20Z", actor: "human:operator", revision: 5}
- {from: "active", to: "implemented", at: "2026-11-26T05:55:44Z", actor: "human:operator", revision: 6, decided_on: {"recorded":{"test_result":1}}}
---
Proposed by `docs/design/native-code.md` § 7 (story 2) and `docs/design/quality-and-comparison.md`
§ 7 and § 9 (story 7). swell is a library today and has no binary, so no engine comparison can
run it from a command line, KPI K5 (binary size) has nothing to measure, and codespec's
`native-speed-comparison` and its oracle run (beyond10x/codespec#68) show swell as "not
measured". Base: `plan/native-code` at `541d32e`.

## Context

- `quality-and-comparison.md` § 7 and Q1: the shell lives in `crates/cli` in the workspace; the
  engine crates keep `#![forbid(unsafe_code)]` and no third-party crate; so does the shell.
- The test262 digest (`task test262-digest`) covers the sources the test262 run depends on. A
  new crate under `crates/` must not force a full test262 run when only the shell changes.

## Change

Failing tests first (Acceptance). Then:

- `crates/cli` builds a binary `swell`: `swell [flags] <file.js>` runs the file as a script;
  `print(...)` writes its arguments as strings joined by a space and a newline to stdout; a host
  clock answers `Date.now()`; flags `--steps <n>`, `--memory <bytes>`, `--depth <n>` set the
  limits (defaults: the engine's defaults); `-e <source>` runs a source string.
- Exit status per outcome: 0 normal; 1 uncaught throw (message on stderr); 2 syntax error;
  3 step budget; 4 memory budget; 5 depth limit; 6 interrupted; 64 usage error.
- The test262 digest is narrowed so it does not cover `crates/cli`, and a test proves a change
  in `crates/cli` keeps the digest.
- `website/docs/embedding.md` or a new page names the binary and its flags; `CHANGELOG.md`
  `## [Unreleased]` `### Added`.

## Acceptance

- Tests under `crates/cli/tests/` run the built binary on fixtures: one per exit status, `print`
  of a number, a string and an object, `-e`, a step budget that ends a loop with status 3.
- `slot cargo build --release -p swell-cli` gives one binary; its size, stripped, is reported.
- `slot --timeout 30m task check-runner` exits 0 with the digest unchanged.
