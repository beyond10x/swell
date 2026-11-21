---
format: aep.planning-md/3
id: epic:foundation
kind: epic
status: implemented
title: 'Foundation: workspace, gate, the test262 pin and the counting runner'
summary: A gate, test262 fetched by commit and checked, a runner that prints 53,801 cases per directory and per slice against an engine that runs nothing, the feature, section and expected files, the Unicode data pin.
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T03:15:03Z", actor: "human:operator", revision: 3, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-21T03:15:03Z", actor: "human:operator", revision: 4, executor: "agent:coordinator"}
- {from: "active", to: "implemented", at: "2026-11-21T05:02:02Z", actor: "human:operator", revision: 5, decided_on: {"recorded":{"test_result":1}}, executor: "agent:coordinator"}
---
## Context

swell starts from a design and no code (`docs/design/engine.md`, accepted on 2026-11-21). Every
delivery slice of the design ends with "the gate green and the expected file raised" (section
12), so before the first slice there has to be a gate, a pinned test262, and a runner that
counts: how many cases exist, are excluded, run and pass, per directory and per slice. This
epic builds that, against an engine that runs nothing, so that slice 1 starts from a count of
zero passes and raises it.

The design decisions it carries out: 1 (the pins and how an exclusion is recorded), 2 and 14
(the workspace, the crates, the names), 8 (the Unicode data pin), 10 (the host interface
specification is held by the gate), 11 (fetching test262 by commit, the runner's output, the
expected file, the sections file).

## Outcome

- A Cargo workspace at the root with the crates of decision 2 that this epic needs
  (`crates/swell` as an engine that runs nothing, `crates/test262`), every crate with
  `#![forbid(unsafe_code)]` and the licence declared, and a committed root `Cargo.lock` that
  lists no package from outside the repository.
- `Taskfile.yml` with `task check` as the one gate. It fails on: a compiler or clippy warning
  under the denials of decision 13, a failing test, a package from outside the repository in
  the root `Cargo.lock`, a crate without `#![forbid(unsafe_code)]`, a Python file in the tree, a
  specification that does not validate or whose conformance run differs, and a conformance
  count that differs from the expected file.
- `task test262-fetch`: test262 at the pinned commit
  `2a1ade303f72d3de80565c921d685222405c32ce` in `.cache/test262/<commit>/`, checked against
  the tree hash `70ab70455f270f26e92d097d863a0b08ad020b94` and the SHA-256 of
  `git ls-tree -r <commit>`, `1756c106c60a6360007cc35c6e4f2d18b23e8045551e27c525612782717cf17f`
  (decision 11). A checkout that is already there and passes the check is not fetched again; a
  checkout that fails it fails the task.
- `swell-test262`, std only: reads the front matter of every case (flags, features, negative,
  includes, both list forms), assigns each case its slice by the rules of the `swell-slices`
  block read from `docs/design/engine.md`, and prints one line per directory in the format of
  decision 11, one line per slice, and a total. Against the engine that runs nothing every
  case is counted as existing, as excluded or not applicable where the rules say so, and none
  as passed.
- `conformance/test262/features.txt` (every feature name test262 declares, `run` or `exclude`
  with reason and decision; a feature the file does not list fails the gate),
  `conformance/test262/sections.txt` (sections of ECMA-262, their test262 paths, their
  omissions; the format and a first set of sections), `conformance/test262/expected.txt` (the
  runner's lines, compared by the gate).
- The Unicode data pin of decision 8: `data/unicode/17.0.0/` with the files the decision lists,
  a `SOURCE` file of addresses and SHA-256 sums, and the data's licence beside it. The
  generator and the tables belong to slice 5.
- The host interface specification under `spec/` is part of the gate: validated with warnings
  refused, its suite synthesized with a ceiling on refusals, and run against the interpreter
  built into `ess`.

- The host interface requirements are part of the gate (`AGENTS.md`, "Rules"; added on
  2026-11-21 after the first ESS model stated about half of the interface):
  `conformance/host/requirements.txt` lists every `HOST-<n>` of
  `docs/design/host-interface.md` with the test that checks it or `omitted <reason>`. The gate
  fails on a requirement with neither, on a listed identifier the document does not have, and
  on an `UNMAPPED:` marker under `spec/` that names neither an existing `HOST-<n>` nor
  `ESS limit`. Before any code, every requirement is `omitted` with the slice that will meet it.

### Open, to be settled by this epic's stories

- Where the 17.0.0 emoji sequence files are published, and the text of the Unicode data
  licence (design, "Not checked").
- The line format per slice: decision 11 gives the directory line and the total; the slice
  line is this epic's to define, and the design is updated with it.
- Which sections `sections.txt` lists first. A section is `complete` only when every case of
  its paths passes and its omission list is empty, so every section starts `incomplete`.

## Counts

Measured on the pinned checkout by the commands of the design's "Inventory" and "How the
counts were made": 53,801 cases, 103,360 runs; by top directory `built-ins` 23,885, `language`
23,832, `intl402` 3,394, `staging` 1,487, `annexB` 1,086, `harness` 117; by slice the 22 lines
at the end of the design (slices 1 to 19: 44,931 cases, 85,794 runs; slice 20: 4,617; slice 21:
1,662; stage 3: 2,591). 199 feature names are declared, 198 used.

A count is measured again on the checkout before a story's acceptance uses it (`AGENTS.md`,
"How the work is run").

## Out of scope

- Lexing, parsing, compiling or running any script: slice 1.
- The Unicode table generator and the generated tables: slice 5.
- The conformance runner over the hand-written host interface (decision 10's adapter and its
  separate workspace): the first slice that has a host interface to adapt, slice 1.
- Publishing, a remote, continuous integration on a forge: the operator's word (`AGENTS.md`,
  "Not yet decided").

## Constraints

- Nothing third-party in the root workspace; `#![forbid(unsafe_code)]` in every crate; no
  panics on any input in the runner's readers either (a malformed front matter is a counted
  error, not a crash).
- No Python anywhere; nothing under `/tmp`; scratch output in `.out/`, fetched data in
  `.cache/`.
- test262 is never vendored, and nothing of it is committed.
- The runner is tooling: it may use threads and the file system. The engine crate may not
  (`AGENTS.md`, "No ambient authority").
- Nothing personal in the tree.

## Acceptance

`slot task check` exits 0 on `main`, and in that run:

1. The runner prints a total line with 53,801 cases, and the counts of excluded, not
   applicable, run and passed cases; the per-directory lines sum to the total.
2. The runner prints one line per slice whose case and run counts equal the 22 lines of the
   design's "Its output on this document" (`01 82 164` to `x-stage3 2591 5008`).
3. The printed lines equal `conformance/test262/expected.txt`, and a changed count in either
   fails the gate, shown by a test that plants one.
4. A case naming a feature that `features.txt` does not list fails the gate, shown by a test.
5. `task test262-fetch` on a checkout whose tree hash differs fails, shown by a test.
6. The root `Cargo.lock` lists no package from outside the repository, and a planted outside
   package fails the gate, shown by a test.
7. The specification under `spec/` validates, and its suite passes on the interpreter with
   the refusal count at or below the committed ceiling.
8. Every file under `data/unicode/17.0.0/` has the SHA-256 its `SOURCE` line gives, checked by
   the gate.
9. Every `HOST-<n>` of `docs/design/host-interface.md` has a line in
   `conformance/host/requirements.txt`, and a requirement without a line, a line without a
   requirement, and an `UNMAPPED:` marker that names neither each fail the gate, shown by a
   test per case.
