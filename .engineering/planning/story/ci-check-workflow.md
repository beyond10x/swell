---
format: aep.planning-md/3
id: story:ci-check-workflow
kind: story
status: implemented
title: The gate and the shared source gates as GitHub workflows, before the first push
summary: check.yml runs task check on push and pull request; shared-gates.yml as surf has it; one cold task check measured
relations:
- decomposes: epic:foundation
scope:
- confidence: cited
  path: .github/workflows/check.yml
- confidence: cited
  path: .github/workflows/shared-gates.yml
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T05:36:55Z", actor: "human:operator", revision: 3, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-21T05:36:55Z", actor: "human:operator", revision: 4, executor: "agent:coordinator"}
- {from: "active", to: "implemented", at: "2026-11-21T06:13:57Z", actor: "human:operator", revision: 5, decided_on: {"recorded":{"test_result":1,"verification":1}}, executor: "agent:coordinator"}
---
## Context

The repository is published as the public repository `beyond10x/swell` at the close of wave 2
of slice 1 (the operator's answer "A" on 2026-11-21, relayed by the session that decides for
him; that session does the forge side). Before the first push the tree needs the two workflows
every published repository of the organization carries, as `../surf` has them
(`../surf/.github/workflows/check.yml`, `shared-gates.yml`): one that runs the gate, one that
calls the organization's shared source gates.

Added on 2026-11-21 as one story, after the foundation epic was delivered; it went through no
critic panel (two workflow files, no engine code).

Not wanted (the deciding session, same message): `b10x.docs.yaml`, a documentation site
workflow, a release workflow.

## Change

- `.github/workflows/check.yml`: on `pull_request` and on `push` to `main`, one job on
  `ubuntu-latest` that checks the repository out, installs the Rust toolchain the workspace is
  built with (the version `rustc --version` prints here, with the components the gate's tasks
  call: at least `clippy`), installs go-task at the version the task files were measured on
  (`task --version` here: 3.54.0) and `ess` 0.96.0, each by a pinned release asset with its
  SHA-256 checked, and runs `task check`. Shape, pins of the actions (full commit ids),
  `permissions: contents: read`, `persist-credentials: false`, the concurrency group and the
  timeout as in surf's file; nothing of surf's that swell's gate does not need
  (`HTML_BASE`, `XDG_RUNTIME_DIR`, `fetch-depth: 0` unless a task of the gate reads history).
- `.github/workflows/shared-gates.yml`: surf's file, unchanged (the same pinned
  `beyond10x/gates/.github/workflows/common.yml`, the same two secrets).
- The gate must run on a fresh checkout with no `.cache/` and no `target/`: `task check`
  fetches test262 by the pin itself (`taskfiles/test262-fetch.yml`). Measure one cold
  `task check` in a scratch clone of this worktree's commit (empty test262 cache, empty
  `target/`, run as `slot task check`), report its wall time and its last lines, and report
  each thing the gate reads that a fresh clone does not have (a tool on `PATH`, a file under
  `.cache/`, an environment variable). A task that needs such a thing is a finding, not
  something to work around in the workflow.
- No Python, no third-party action beyond the ones surf's two files use, no secret beyond the
  two of `shared-gates.yml`, no comment in the workflow files.

The SHA-256 of the go-task 3.54.0 release asset is not known in this repository and no
download from a forge is made by this unit: write the version, and leave the hash as the one
value the report names as open, unless the brief gives it.

Gate task: none new.

Writes: `.github/workflows/check.yml`, `.github/workflows/shared-gates.yml`.

## Acceptance

Both files exist; `shared-gates.yml` is byte-equal to surf's; every `uses:` in both names a
full commit id; both declare `permissions`; `check.yml` names no path under a home directory
and no person; and the cold `slot task check` of the scratch clone exits 0 with the total line
`test262 total 53801 8982 7 44812 0 44812 0`, its wall time stated in the report.

## Out of scope

Any call to a forge, a remote, a push; the root `Taskfile.yml`, `AGENTS.md`, `README.md`
(the coordinator); a documentation or release workflow.
