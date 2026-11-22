---
format: aep.planning-md/3
id: story:every-host-interface-change-is-named-in-the-changelog
kind: story
status: active
title: The gate fails when a public item of the host interface or a HOST requirement changed since the last release and the changelog's unreleased section does not name it in the stated form
summary: A committed snapshot of the interface at the last release (every public item of crates/swell/src/{host,value} and every HOST line), compared with a scan of the tree by a fast check; each changed item must be named, in one stated name form, under "### Host interface" of [Unreleased]. The release procedure renews the snapshot. Checkable from the tree alone.
relations:
- depends_on: story:gate-test-steps-within-budget
- depends_on: story:local-gate-equals-forge
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T06:48:29Z", actor: "human:operator", revision: 2, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-22T06:48:29Z", actor: "human:operator", revision: 3, executor: "agent:coordinator"}
---
Waits for: `story:gate-test-steps-within-budget` (the gate's step list and time lines) and
`story:local-gate-equals-forge` (its file of planning-only paths, to which this story adds a
line, and its classifier script, which this story's planted script calls). Batch H1. No new
requirement. Coordinate with `story:spec-diff-in-the-gate` (draft), which compares `spec/` with the
last tag: this story covers the Rust interface and the `HOST-<n>` lines, that one the ESS model,
and neither covers the other's files. Consumer: the browser, requirement 7 of its list. Decision
D7 (taken: option A, a committed snapshot of the host interface, checked from the tree).

## Context

Line numbers are of `wave/budget-1` at `ed56b05`.

- The public interface is the glob re-export of `host` and `value` and nothing else
  (`crates/swell/src/lib.rs:6-8`, `:40-41`); every file of `crates/swell/src/host/` is public
  through the globs of `host/mod.rs` (`:14-19`, `:21-26`). Public items there: `pub fn`, `pub
  struct` with `pub` fields, `pub enum` with its variants, `pub type`, `pub const`, the `Host`
  trait's methods (no `pub` keyword; `crates/swell/src/host/hooks.rs`), and the methods of the
  `scoped_methods!` macro (`host/mod.rs:414-512`), which `Cx` and `HookCx` both get (`:514-515`).
- Two types can have a method of one name: `Agent::host` and `Agent::host_mut`
  (`crates/swell/src/host/agent.rs:196`, `:201`) beside the scoped `host` and `host_mut`
  (`host/mod.rs:418`, `:423`). `get` (`:433`) is a scoped method, so `Cx::get` is named `get`.
- The requirements are the lines of `docs/design/host-interface.md` that start with `**HOST-`;
  `conformance/host/requirements.txt` gives each a state.
- `docs/releasing.md:35`: before 1.0 "the host interface may change in any release, and the
  changelog says where"; nothing checks it. `## [Unreleased]` is empty (`CHANGELOG.md:11`); 0.2.0's
  section has "Added", "Changed" and "Limits of this release" (`:21`, `:57`, `:74`), no heading
  for the host interface. The tag `0.2.0` is the commit `29f674c`, in the local repository.
- `story:local-gate-equals-forge` lets a change made only of `.engineering/**` and `*.md` files
  skip the build and test jobs, without the files a gate step reads as data, which count as code;
  it names `docs/design/host-interface.md` among them (read by `host-requirements-check`).
  `CHANGELOG.md` becomes such a file with this story.
- The check reads the tree alone: it needs no tag and no history in the forge's checkout.

## Change

Failing tests first: the planted scripts of point 5 are written and seen red before the check.
Then (decision D7, option A):

1. **The name form**, stated once, in `docs/releasing.md` and in the check's message: a function
   or method by its bare name in backticks (`charge_steps`), with its type before it where two
   types of the interface have a method of that name (`Agent::host`); a method of
   `scoped_methods!`, which both `Cx` and `HookCx` have, once, by its bare name (`host`, `get`); a
   variant or a field by `Type::name` (`HostError::StaleHandle`, `ErrorSite::line`); a type,
   trait, constant or type alias by its name (`HostBytes`); a requirement by `HOST-<n>`. The check
   accepts exactly that form: a name is found when the token, in backticks, stands under the
   heading; another form of the same item is not found.
2. **A scan.** `taskfiles/host-surface.yml`, task `host-surface-scan`, writes the interface of
   the tree as sorted lines with `awk`, `sort` and `sha256sum` only, each `<name> <rest>`: one per
   public item of `crates/swell/src/value.rs` and `crates/swell/src/host/*.rs` (its signature
   joined to one line up to `{` or `;`, spaces collapsed, doc comments and bodies left out), one
   per variant and per `pub` field, one per method of the `Host` trait and one per method of
   `scoped_methods!`; the name in the form of point 1 (the scan tracks the `impl` block or the
   macro a method is in, and prefixes the type of a method whose name two types have); and one
   per `**HOST-<n>**` line of `docs/design/host-interface.md`, `HOST-<n> <sha256 of the line>`.
3. **A snapshot.** `conformance/host/surface.txt` is the scan at the last release, with a first
   line `# swell <version>`. The unit writes it for 0.2.0 from the objects of the tag (`git show
   0.2.0:<path>` into `.out/host-surface-0.2.0/`, the scan run there), no network.
4. **The check.** `host-surface-check` runs the scan on the tree and compares it with the
   snapshot (`comm -3`); for each line added or removed it takes the name, and fails, one line per
   name, when the name does not stand in backticks under a `### Host interface` heading of `##
   [Unreleased]` in the file `HOST_SURFACE_CHANGELOG` names (default `CHANGELOG.md`):
   `host-surface: <name> changed since <version> and [Unreleased] does not name it (name form:
   docs/releasing.md)`. It prints `host-surface: <n> change(s) since <version>, each named` when
   it passes. A rename shows as a removal and an addition, both to be named. A doc comment, a
   private item and a body change no line, so they never fail it.
5. **Planted defects**, each a script under `tests/gate/`, under 5 s, working on a copy of the
   files under `.out/gate-tests/`, each with a refused leg and a passing leg:
   `host-surface-check-refuses-a-method-not-named-bare.sh` (a changed `pub fn` that one type has:
   named `Agent::<name>` refused, bare passes); `host-surface-check-refuses-a-shared-method-named-bare.sh`
   (a changed `Agent::host`: `host` refused, `Agent::host` passes);
   `host-surface-check-refuses-a-scoped-method-named-by-its-type.sh` (a changed scoped `get`:
   `Cx::get` refused, `get` passes); `host-surface-check-refuses-a-variant-named-bare.sh` (a
   removed variant of `HostError`: bare refused, `HostError::<name>` passes);
   `host-surface-check-refuses-a-field-named-bare.sh` (a `pub` field added to `Limits`: bare
   refused, `Limits::<name>` passes); `host-surface-check-refuses-a-requirement-not-named.sh` (a
   new `**HOST-<n>**` line: unnamed refused, `HOST-<n>` passes);
   `host-surface-check-refuses-a-name-under-another-heading.sh` (the right name under `###
   Changed` refused, under `### Host interface` passes); and
   `host-surface-check-passes-a-changed-doc-comment.sh` (nothing named, exit 0).
6. **The classifier.** The line `CHANGELOG.md` is added to the code side of
   `story:local-gate-equals-forge`'s path file (the files a gate step reads as data), with the
   task that reads it, `host-surface-check`; `docs/design/host-interface.md` is there already.
   `conformance/host/surface.txt` is code already (not `*.md`, not under `.engineering/`). A
   planted script `tests/gate/planning-only-classifier-counts-the-changelog-as-code.sh` runs that
   story's classifier on a change of `CHANGELOG.md` alone and passes when it answers `code`.
7. **The release procedure** (the coordinator's `docs/releasing.md`, step 2, the unit gives the
   text): the version's changelog section keeps its `### Host interface` heading (with `- None.`
   when nothing changed); `task host-surface-snapshot` writes the scan of the release commit into
   `conformance/host/surface.txt` in the same commit; the notes' `**Breaking**` part lists every
   removed or changed name of that heading. The name form of point 1 is stated there.
8. **The unreleased section now.** The unit lists, for the coordinator, every name the check
   reports on the base against 0.2.0, so that the coordinator writes the heading's lines in the
   change that adds the step to `check`.

Design: none changes in `docs/design/`. `docs/releasing.md` and `CHANGELOG.md` are the
coordinator's; the unit gives their lines in its report.

Writes: `taskfiles/host-surface.yml` (new), `conformance/host/surface.txt` (new),
`tests/gate/host-surface-check-*.sh` (new, eight), `tests/gate/planning-only-classifier-counts-the-changelog-as-code.sh`
(new), and in `story:local-gate-equals-forge`'s path file the one line of point 6. Not written:
any file under `crates/` or `spec/`, the root `Taskfile.yml`, `CHANGELOG.md`, `docs/releasing.md`
(the coordinator's, from the report's lines).

## Acceptance

On the unit's branch:

1. `slot task --taskfile taskfiles/host-surface.yml host-surface-scan` prints the scan of the
   tree (its tasks set `dir: '{{.TASKFILE_DIR}}/..'`, so they run the same included or alone); its
   count of `HOST-` lines equals `grep -c '^\*\*HOST-' docs/design/host-interface.md`, and both
   counts are in the report.
2. `host-surface-check` exits 1 on the tree as it is (`[Unreleased]` empty) and prints one line
   per name, which the report quotes; with `HOST_SURFACE_CHANGELOG=.out/CHANGELOG.md`, a copy of
   `CHANGELOG.md` with the heading and the names of point 8 under `## [Unreleased]`, it exits 0
   and prints the count. The tree's `CHANGELOG.md` is not written.
3. `slot task gate-selftest` passes with the nine new scripts, each seen red before its check
   existed (the report quotes each red line).
4. `host-surface-check` runs in under 2 s; nothing in it compiles.
5. `task --taskfile .out/Taskfile.yml --list`, on a copy of the root `Taskfile.yml` with the
   report's lines and its include paths prefixed `../`, names `host-surface-check`.

Commands, each one job of the build pool (one slot): the tasks above and `slot task
gate-selftest`. No cargo command, no test262 run, no `host-requirements-check`: nothing under
`crates/` and no requirement line changes. Never the whole gate (`slot task check`).

Rules this unit keeps. A test is killed at 60 s and the gate's test steps fail over 120 s: the
check and the scripts read files only and run under 5 s each, and the check is a step before
`build`, outside the test steps. A job of the build pool asks for one slot; one over 10 minutes is killed, so `host-requirements-check` on a cold tree and the full test262 run are named `slot timeout 1800 <command>`. No Python; `awk`,
`sort`, `comm`, `sha256sum` and `git show` for the one-time snapshot. No network.

## Report

The scan's counts, the names reported against 0.2.0 for the coordinator's heading, the commit the
snapshot was taken from, the red lines of the planted scripts, and the coordinator's lines:

- `Taskfile.yml` `includes:`: `host-surface: {taskfile: taskfiles/host-surface.yml, flatten:
  true, optional: true}`, written in the block form of the others.
- In `check`: `- task: host-surface-check` after `- task: host-requirements-check`, before `-
  task: build`.
- `docs/releasing.md`, step 2: the text of point 7.
- `CHANGELOG.md`: under `## [Unreleased]`, `### Host interface` with the names of point 8; and,
  under "Changed": "The gate now fails when a public item of the host interface or a `HOST-<n>`
  requirement changed since the last release and the changelog does not name it under Host
  interface, in the name form of `docs/releasing.md`."
