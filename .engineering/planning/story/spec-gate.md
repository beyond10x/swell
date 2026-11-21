---
format: aep.planning-md/3
id: story:spec-gate
kind: story
status: implemented
title: 'Hold the host interface specification in the gate: validate, suite under a refusal ceiling, conform'
summary: taskfiles/spec.yml with validate (warnings refused), suite (refusals at or below REFUSALS_MAX, 33 on 609b568) and conform on the ess interpreter (64 scenarios passed on 609b568); counts set on the base of the wave.
relations:
- decomposes: epic:foundation
scope:
- confidence: cited
  path: taskfiles/spec.yml
- confidence: cited
  path: tests/gate/suite-refuses-a-refusal-count-above-the-ceiling.sh
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T03:15:04Z", actor: "human:operator", revision: 5, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-21T03:15:04Z", actor: "human:operator", revision: 6, executor: "agent:coordinator"}
- {from: "active", to: "implemented", at: "2026-11-21T04:04:14Z", actor: "human:operator", revision: 9, decided_on: {"recorded":{"test_result":1,"verification":1}}, executor: "agent:coordinator"}
---
## Context

Design decision 10: an ESS model under `spec/` states the host interface (about half of it,
`AGENTS.md` "Rules"; the rest is `docs/design/host-interface.md`, held by
`story:host-requirements-gate`). The epic puts the model into the gate: validated with warnings
refused, its suite synthesized with a ceiling on refusals, and run against the interpreter built
into `ess`. `spec/` is merged (`system.yaml`, `ess-inputs.yaml` with `requires: ess 0.96.0`,
`domains/host.yaml`, three authored scenarios under `scenarios/`).

Measured on 2026-11-21 in the store worktree at commit `3b79f83` with `ess 0.96.0`
(`ess --version`), logs under `.out/spec-measure/`:

- `ess --strict-requires specify validate --path spec`: exit 0, last line
  `swell v1 — 2 file(s), 3 scenario(s), valid`; `grep -c '^warning'` on the log prints 0.
- `ess verify conform synthesize --path spec --scenarios spec --out .out/spec-measure/suite.json`:
  exit 0, last line `64 scenario(s) (3 authored), 33 refusal(s), written to
  .out/spec-measure/suite.json`; `grep -c '^refused:'` prints 33, `grep -c '^note:'` 18. By code
  (`grep -o 'ESS-[A-Z]*-[0-9]*' | sort | uniq -c`): `ESS-SYNTH-003` 14, `ESS-SYNTH-023` 9,
  `ESS-SYNTH-001` 6, `ESS-SYNTH-012` 4.
- `ess verify conform run --path spec --suite .out/spec-measure/suite.json --target interpreted
  --report-format 2 --report-out .out/spec-measure/report.json`: exit 0, last line
  `  64 scenarios: 64 passed, 0 failed, 0 error, 0 unsupported` (two leading spaces).

The coordinator measured the same three lines. The model changes once more before wave 1 (a
second author round), so the ceiling and the scenario count in the task are what these commands
print on the base of the wave, quoted in the report of the unit; 33 and 64 are the values of today.

These tasks follow surf (`$HOME/beyond10x/surf/Taskfile.yml`, tasks `validate`, `suite`,
`conform`). The conformance runner over the hand-written host interface (decision 10, the adapter
in a separate workspace) is not this story: there is no host interface to adapt until slice 1.

The gate is put together as `story:workspace-and-gate` describes: this story writes its own
include file `taskfiles/spec.yml`, which the root `Taskfile.yml` includes as optional, and its
planted-defect test as a script under `tests/gate/` in the form that story gives (it prints
`<test name>: ok` when the defect is refused). Its tasks run alone with
`task -d . -t taskfiles/spec.yml <task>`.

## Change

Failing test first: `tests/gate/suite-refuses-a-refusal-count-above-the-ceiling.sh` is written
before the tasks; it prints `FAIL` (no task) before the change and `ok` after.

- `taskfiles/spec.yml`, variable `SPEC_REFUSALS_MAX`, starting at 33 and set to the count of
  `refused:` lines that `task suite` prints on the base of the wave, and three tasks:
  - `validate`: `ess --strict-requires specify validate --path spec`, log `.out/validate.log`;
    fails on a non-zero exit or on any line starting with `warning`; prints the last line.
  - `suite`: `ess verify conform synthesize --path spec --scenarios spec --out .out/suite.json`,
    log `.out/synthesize.log`; fails on a non-zero exit or when the count of lines starting
    `refused:` is above `SPEC_REFUSALS_MAX`, printing both numbers; prints the last line.
  - `conform` (depends on `suite`): `ess verify conform run --path spec --suite .out/suite.json
    --target interpreted --report-format 2 --report-out .out/report.json`, log
    `.out/conform.log`; fails on a non-zero exit, printing the lines of failed and errored
    scenarios and the last line; prints the last line.
- `tests/gate/suite-refuses-a-refusal-count-above-the-ceiling.sh`: runs
  `task -d . -t taskfiles/spec.yml suite`, counts the `refused:` lines `<r>` of the log, runs it
  again with `SPEC_REFUSALS_MAX=<r - 1>`, and requires a non-zero exit.
- The report lists each refusal (`grep '^refused:' .out/synthesize.log`) by code, so that the
  coordinator records them beside the ceiling in `AGENTS.md`, as surf does.

Gate tasks (the coordinator adds them to `check` in `Taskfile.yml`, before `build`):
`- task: validate` and `- task: conform`.

- Added after the unit's first report (2026-11-21): the task variables are `SPEC_REFUSALS_MAX` and
  `SPEC_OUT` (default `.out`), so that an exported `OUT` or `REFUSALS_MAX` cannot redirect the task
  or raise the ceiling; and `conform` fails unless the last line of the run reads `<n> scenarios:
  <n> passed, 0 failed, 0 error, 0 unsupported` with the same `<n>` twice, whatever `ess` exits
  with.

Writes: `taskfiles/spec.yml`, `tests/gate/suite-refuses-a-refusal-count-above-the-ceiling.sh`.

## Acceptance

On the base of the wave, `slot task -d . -t taskfiles/spec.yml validate` and
`slot task -d . -t taskfiles/spec.yml conform` each exit 0 and print, with the counts of that
base quoted in the report (on `3b79f83`: `swell v1 — 2 file(s), 3 scenario(s), valid`,
`64 scenario(s) (3 authored), 33 refusal(s), written to .out/suite.json`,
`  64 scenarios: 64 passed, 0 failed, 0 error, 0 unsupported`), the `valid` line with no line of
the log starting `warning`, the synthesize line whose refusal count equals `SPEC_REFUSALS_MAX`, and
the interpreter line with every scenario passed and 0 failed, 0 error, 0 unsupported; and
`sh tests/gate/suite-refuses-a-refusal-count-above-the-ceiling.sh`, run in the worktree of this
story, exits 0 and prints `suite-refuses-a-refusal-count-above-the-ceiling: ok`.

## Out of scope

Writing or changing anything under `spec/` (the specification author). The requirements of
`docs/design/host-interface.md` (`story:host-requirements-gate`). The conformance runner over the
hand-written host interface and its separate Cargo workspace: slice 1. Generating code from ESS
(decision 10). `AGENTS.md` (the coordinator records the counts).
