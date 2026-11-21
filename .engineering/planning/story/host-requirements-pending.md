---
format: aep.planning-md/3
id: story:host-requirements-pending
kind: story
status: implemented
title: 'Split the requirement states: pending names its slice, omitted is a decision that stays'
summary: requirements.txt lines become test, pending slice <s> or omitted <reason>; the gate prints three counts and fails on a pending requirement of a delivered slice; 148 pending, 68 of them for slice 1.
relations:
- decomposes: epic:foundation
- depends_on: story:host-requirements-gate
scope:
- confidence: cited
  path: conformance/host/requirements.txt
- confidence: cited
  path: conformance/slices-delivered.txt
- confidence: cited
  path: docs/design/host-interface.md
- confidence: cited
  path: taskfiles/host-requirements.yml
- confidence: cited
  path: tests/gate
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T04:05:41Z", actor: "human:operator", revision: 3, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-21T04:05:41Z", actor: "human:operator", revision: 4, executor: "agent:coordinator"}
- {from: "active", to: "implemented", at: "2026-11-21T04:37:19Z", actor: "human:operator", revision: 5, decided_on: {"recorded":{"test_result":1,"verification":1}}, executor: "agent:coordinator"}
---
## Context

`story:host-requirements-gate` (wave 1) put the 148 requirements of
`docs/design/host-interface.md` into the gate, each as `HOST-<n> omitted slice <s> not
delivered`, and the gate prints `0 test, 148 omitted`. That makes a requirement nobody has
implemented yet look the same as a deliberate omission that stays. The rule of the project is
that complete means complete as to the specifications (`AGENTS.md`, "Goals"), so the two states
are split (decided on 2026-11-21 by the session that decides for the operator):

- `HOST-<n> pending slice <s>`: not implemented yet; slice `<s>` will close it. `<s>` is the
  number in the requirement's `[slice <s>]` mark.
- `HOST-<n> omitted <reason>`: a deliberate omission that stays, with its reason. None exists
  today.
- `HOST-<n> test <crate>::<test name>`: held by a test (still refused until the first story
  that writes one also checks that the test exists).

A slice is complete only when no requirement is pending for it. The gate has to know which
slices are delivered: `conformance/slices-delivered.txt` holds one slice number per line (empty
today; a comment line starts with `#`), and the coordinator adds a slice's number in the change
that closes the slice.

Measured on `main` at `a64b991`: `grep -c '^\*\*HOST-' docs/design/host-interface.md` prints
148; `grep -c ' omitted slice ' conformance/host/requirements.txt` prints 148; by slice mark
68 are for slice 1 (`grep -c '\[slice 1\]$'` over the requirement lines).

## Change

Failing test first: `tests/gate/host-requirements-refuses-a-pending-line-of-a-delivered-slice.sh`
is written before the task changes; it fails because the task knows no `pending` line.

- `conformance/host/requirements.txt`: every line becomes `HOST-<n> pending slice <s>`, `<s>`
  from the requirement's mark. 148 lines, none `omitted`, none `test`.
- `conformance/slices-delivered.txt`: new, with a first line that is a comment saying what the
  file is, and no slice.
- `taskfiles/host-requirements.yml`, task `host-requirements-check`: a line is `test`,
  `pending slice <s>` or `omitted <reason>` (a reason is at least one word, and a reason that
  starts with `slice` and ends with `not delivered` is refused as the old form of pending).
  New faults, one line each:
  - `<id>: pending slice <s>, and host-interface.md marks it [slice <t>]` (replaces the same
    check on `omitted` lines; an `omitted` line names no slice and is not compared);
  - `<id>: pending, and slice <s> is delivered` when `<s>` is in the delivered file;
  - `slices-delivered.txt:<line>: not a slice number` for a line that is neither a comment,
    blank, nor a number.
  A new variable `HOST_REQ_DELIVERED` (default `conformance/slices-delivered.txt`); a missing
  file fails as the other missing files do. The success line becomes `host-requirements: <r>
  requirement(s), <r> line(s): <t> test, <p> pending, <o> omitted; <m> UNMAPPED marker(s), each
  naming a HOST-<n> or ESS limit`, followed by one line `host-requirements: pending by slice:
  1:<n> 2:<n> ...` listing every slice that has a pending requirement.
- The nine existing scripts follow the new line forms. The script
  `host-requirements-refuses-an-omitted-line-naming-another-slice.sh` is renamed
  `host-requirements-refuses-a-pending-line-naming-another-slice.sh`. Two new scripts:
  `host-requirements-refuses-a-pending-line-of-a-delivered-slice.sh` and
  `host-requirements-refuses-the-old-omitted-form.sh`.
- `docs/design/host-interface.md`, section "How the gate holds this document": the three line
  forms, the delivered file, and that a slice is complete only with no pending requirement.
  No requirement line changes.

Gate task: unchanged (`- task: host-requirements-check` is already in `check`). The coordinator
adds `HOST_REQ_DELIVERED` to `GATE_VARIABLES` in the root `Taskfile.yml`.

Writes: `conformance/host/requirements.txt`, `conformance/slices-delivered.txt`,
`taskfiles/host-requirements.yml`, `docs/design/host-interface.md` (the one section),
`tests/gate/host-requirements-*.sh` (the nine, one of them renamed, and the two new ones).

## Acceptance

`slot task -d . -t taskfiles/host-requirements.yml host-requirements-check` exits 0 and prints
`host-requirements: 148 requirement(s), 148 line(s): 0 test, 148 pending, 0 omitted; 50 UNMAPPED
marker(s), each naming a HOST-<n> or ESS limit` and a line `host-requirements: pending by
slice:` whose entry for slice 1 is `1:68`; the same task with `HOST_REQ_DELIVERED` pointing at
a file holding the line `1` exits non-zero and prints 68 lines of the form `HOST-<n>: pending,
and slice 1 is delivered`; and the eleven scripts, run as `sh tests/gate/<name>.sh` in the
worktree of this story, each exit 0 and print `<name>: ok`.

## Out of scope

Checking that the test of a `test` line exists (the first slice 1 story that closes a
requirement). Marking a story, as opposed to a slice, as done: stories close requirements by
moving their lines to `test`, and the slice is what the delivered file names. Any change to a
requirement line or to `spec/`. `AGENTS.md` and the root `Taskfile.yml` (the coordinator).
