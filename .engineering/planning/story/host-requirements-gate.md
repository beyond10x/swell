---
format: aep.planning-md/3
id: story:host-requirements-gate
kind: story
status: implemented
title: 'Hold the host interface requirements in the gate: every HOST-<n> listed, every UNMAPPED marker named'
summary: conformance/host/requirements.txt with one omitted line per HOST-<n> of docs/design/host-interface.md, and a gate task that refuses a missing, unknown, duplicate or malformed line, an unchecked test line and an UNMAPPED marker naming neither a HOST-<n> nor ESS limit.
relations:
- decomposes: epic:foundation
scope:
- confidence: cited
  path: conformance/host/requirements.txt
- confidence: cited
  path: taskfiles/host-requirements.yml
- confidence: cited
  path: tests/gate/host-requirements-refuses-a-duplicate-line.sh
- confidence: cited
  path: tests/gate/host-requirements-refuses-a-line-without-a-requirement.sh
- confidence: cited
  path: tests/gate/host-requirements-refuses-a-malformed-line.sh
- confidence: cited
  path: tests/gate/host-requirements-refuses-a-requirement-stated-twice.sh
- confidence: cited
  path: tests/gate/host-requirements-refuses-a-requirement-without-a-line.sh
- confidence: cited
  path: tests/gate/host-requirements-refuses-a-test-line-while-tests-are-unchecked.sh
- confidence: cited
  path: tests/gate/host-requirements-refuses-an-omitted-line-naming-another-slice.sh
- confidence: cited
  path: tests/gate/host-requirements-refuses-an-unmapped-marker-naming-an-unknown-requirement.sh
- confidence: cited
  path: tests/gate/host-requirements-refuses-an-unmapped-marker-naming-neither.sh
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T03:15:04Z", actor: "human:operator", revision: 4, decided_on: {"recorded":{"review_outcome":1}}, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-21T03:15:04Z", actor: "human:operator", revision: 5, decided_on: {"recorded":{"review_outcome":1}}, executor: "agent:coordinator"}
- {from: "active", to: "implemented", at: "2026-11-21T04:04:15Z", actor: "human:operator", revision: 7, decided_on: {"recorded":{"test_result":1,"review_outcome":1,"verification":1}}, executor: "agent:coordinator"}
---
## Context

The host interface has two specifications (`AGENTS.md`, "Rules"): the ESS model under `spec/`,
which states about half of it, and `docs/design/host-interface.md`, which states the rest as
numbered requirements, one per sentence an implementation can break, each `HOST-<n>` with its
clause and its observable outcome. Each `UNMAPPED:` marker in `spec/` names the `HOST-<n>` that
carries it. `conformance/host/requirements.txt` lists every `HOST-<n>` with the test that checks
it or `omitted <reason>`. The epic (Outcome, last bullet; acceptance item 9) puts that list into
the gate. Before any code, every requirement is `omitted` with the slice that will meet it.

`docs/design/host-interface.md` is being written in another worktree and lands on the base of
the wave before this story starts, as `spec/` did; the markers of `spec/` are revised in the same
round to name a `HOST-<n>`. No count is written here in advance: the requirement count is what
`grep -c '^\*\*HOST-' docs/design/host-interface.md` prints on the base of the wave, and the
marker count what `grep -r 'UNMAPPED:' spec | wc -l` prints there. Measured on `609b568` (before
the second round): `grep -r 'UNMAPPED:' spec | wc -l` prints 50 (49 in `spec/domains/host.yaml`,
1 in `spec/system.yaml`), and `grep -rc 'HOST-\|ESS limit' spec` prints 0 for every file, so on
that tree the check would refuse all 50; the document does not exist there yet.

Settled here:

- **A requirement** is a line of the document that starts with `**HOST-<n>**`; its identifier is
  `HOST-<n>`. **A marker** is a line under `spec/` holding `UNMAPPED:` together with the comment
  lines that directly follow it (lines whose first non-blank character is `#`), up to the next
  line that is not a comment or holds `UNMAPPED:` again; it names a requirement when its text
  holds `HOST-<n>`, and it names the limit of ESS when its text holds `ESS limit`. **The slice of a requirement** is the number in the `[slice <t>]` mark at the end of its line.
- **The list format:** one line per requirement, `HOST-<n> test <crate>::<test name>` or
  `HOST-<n> omitted <reason>`, sorted by the number, no comments. Today every line is
  `HOST-<n> omitted slice <s> not delivered`, `<s>` the slice the document gives for the
  requirement. A requirement the document gives no slice for is written
  `HOST-<n> omitted slice unassigned not delivered` and listed in the report, so that the
  coordinator settles it; no slice is guessed.
- **Whether the test of a `test` line exists** is left to the first story that writes a `test`
  line, and until then the check refuses every `test` line. Why: no test of the host interface
  exists before slice 1, so a check of test names written now could only be shown on a fixture
  crate invented here; how a test is named across unit and integration tests, and whether the
  check lists tests with `cargo test -p <crate> -- --list` (which compiles) or reads source, is
  decided best by the story that has the first real test. Refusing `test` lines meanwhile
  keeps an unchecked test claim out of the gate.

The gate is put together as `story:workspace-and-gate` describes: this story writes its own
include file `taskfiles/host-requirements.yml`, which the root `Taskfile.yml` includes as
optional, and its planted-defect tests as scripts under `tests/gate/` in the form that story gives
(each prints `<test name>: ok` when the defect is refused). Its task runs alone with
`task -d . -t taskfiles/host-requirements.yml host-requirements-check`. Shell and `awk` only.

## Change

Failing test first: `tests/gate/host-requirements-refuses-a-requirement-without-a-line.sh` is
written before the task; it prints `FAIL` (no task) before the change and `ok` after.

- `conformance/host/requirements.txt`: one `omitted` line per requirement of the document on the
  base of the wave, as settled above.
- `taskfiles/host-requirements.yml`, task `host-requirements-check`, variables `HOST_REQ_DOC` (default
  `docs/design/host-interface.md`), `HOST_REQ_LIST` (default `conformance/host/requirements.txt`) and
  `HOST_REQ_SPEC` (default `spec`). It prints every fault as one line and fails when there is one:
  - `<id>: in host-interface.md, no line in requirements.txt`;
  - `<id>: in requirements.txt, not in host-interface.md`;
  - `<id>: listed <k> times in requirements.txt` and `<id>: stated <k> times in host-interface.md`;
  - `requirements.txt:<line>: neither HOST-<n> test <crate>::<test> nor HOST-<n> omitted <reason>`;
  - `<id>: a test line, and test lines are not checked yet`;
  - `<id>: omitted slice <s>, and host-interface.md marks it [slice <t>]`: an `omitted` line of the
    form `omitted slice <s> not delivered` whose `<s>` is not the slice in the `[slice <t>]` mark
    at the end of the requirement's line (`unassigned` where the line has no mark);
  - `<file>:<line>: UNMAPPED: names neither a HOST-<n> of host-interface.md nor ESS limit`;
  - `<file>:<line>: UNMAPPED: names <id>, which host-interface.md does not have`.
  A missing `HOST_REQ_DOC` or `HOST_REQ_LIST` fails too. Success: `host-requirements: <r> requirement(s), <r>
  line(s): <t> test, <o> omitted; <m> UNMAPPED marker(s), each naming a HOST-<n> or ESS limit`.
- One script per fault under `tests/gate/`, each building a small document, list and `spec/` in
  its sandbox, running the task on the valid set first (it must pass), then on the set with one
  defect planted (it must fail and print that fault line, and no other):
  `host-requirements-refuses-a-requirement-without-a-line.sh`,
  `host-requirements-refuses-a-line-without-a-requirement.sh`,
  `host-requirements-refuses-a-duplicate-line.sh`,
  `host-requirements-refuses-a-requirement-stated-twice.sh`,
  `host-requirements-refuses-a-malformed-line.sh`,
  `host-requirements-refuses-a-test-line-while-tests-are-unchecked.sh`,
  `host-requirements-refuses-an-omitted-line-naming-another-slice.sh`,
  `host-requirements-refuses-an-unmapped-marker-naming-neither.sh`,
  `host-requirements-refuses-an-unmapped-marker-naming-an-unknown-requirement.sh`.
  The valid set holds one marker naming a requirement on its continuation line and one naming
  `ESS limit`, so both forms are shown accepted.

Gate task (the coordinator adds it to `check` in `Taskfile.yml`, before `build`):
`- task: host-requirements-check`.

Writes: `conformance/host/requirements.txt`, `taskfiles/host-requirements.yml`,
`tests/gate/host-requirements-refuses-a-requirement-without-a-line.sh`,
`tests/gate/host-requirements-refuses-a-line-without-a-requirement.sh`,
`tests/gate/host-requirements-refuses-a-duplicate-line.sh`,
`tests/gate/host-requirements-refuses-a-requirement-stated-twice.sh`,
`tests/gate/host-requirements-refuses-a-malformed-line.sh`,
`tests/gate/host-requirements-refuses-a-test-line-while-tests-are-unchecked.sh`,
`tests/gate/host-requirements-refuses-an-omitted-line-naming-another-slice.sh`,
`tests/gate/host-requirements-refuses-an-unmapped-marker-naming-neither.sh`,
`tests/gate/host-requirements-refuses-an-unmapped-marker-naming-an-unknown-requirement.sh`.

## Acceptance

On the base of the wave, `slot task -d . -t taskfiles/host-requirements.yml
host-requirements-check` exits 0 and prints `host-requirements: <r> requirement(s), <r> line(s):
0 test, <r> omitted; <m> UNMAPPED marker(s), each naming a HOST-<n> or ESS limit`, with `<r>` what
`grep -c '^\*\*HOST-' docs/design/host-interface.md` prints there and `<m>` what
`grep -r 'UNMAPPED:' spec | wc -l` prints there, both quoted in the report; and the nine scripts,
run as `sh tests/gate/<name>.sh` in the worktree of this story, each exit 0 and print
`host-requirements-refuses-a-requirement-without-a-line: ok`,
`host-requirements-refuses-a-line-without-a-requirement: ok`,
`host-requirements-refuses-a-duplicate-line: ok`,
`host-requirements-refuses-a-requirement-stated-twice: ok`,
`host-requirements-refuses-a-malformed-line: ok`,
`host-requirements-refuses-a-test-line-while-tests-are-unchecked: ok`,
`host-requirements-refuses-an-omitted-line-naming-another-slice: ok`,
`host-requirements-refuses-an-unmapped-marker-naming-neither: ok`,
`host-requirements-refuses-an-unmapped-marker-naming-an-unknown-requirement: ok`.

## Out of scope

Writing `docs/design/host-interface.md` or anything under `spec/`, the markers included (their
authors). Checking that the test of a `test` line exists: the first story that writes one. Any
test of the host interface: slice 1 and later. `AGENTS.md` (the coordinator records the counts).
