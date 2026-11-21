---
format: aep.planning-md/3
id: story:ess-conformance-target
kind: story
status: implemented
title: The ESS conformance target runs the model scenarios against the slice 1 interface
summary: conformance/ess, its own workspace and lock with ess-conformance at tag 0.96.0, six commands mapped, the rest unsupported under a ceiling, a planted-defect test, task conform-engine.
relations:
- decomposes: epic:slice-01-harness
- depends_on: story:interpreter-and-script-entry-points
- depends_on: story:host-interface-hardening
scope:
- confidence: cited
  path: conformance/ess/Cargo.lock
- confidence: cited
  path: conformance/ess/Cargo.toml
- confidence: cited
  path: conformance/ess/src
- confidence: cited
  path: conformance/ess/tests/planted_defect.rs
- confidence: cited
  path: taskfiles/conform-engine.yml
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T04:50:00Z", actor: "human:operator", revision: 4, decided_on: {"recorded":{"review_outcome":1}}, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-21T04:50:00Z", actor: "human:operator", revision: 5, decided_on: {"recorded":{"review_outcome":1}}, executor: "agent:coordinator"}
- {from: "active", to: "implemented", at: "2026-11-21T13:39:23Z", actor: "human:operator", revision: 7, decided_on: {"recorded":{"test_result":1,"review_outcome":1,"verification":1}}, executor: "agent:coordinator"}
---
## Context

Decision 10 of `docs/design/engine.md` and the epic: a conformance target in a workspace of its own
runs the scenarios of the ESS model under `spec/` against the hand-written host interface, as far
as this slice entry points reach. `story:spec-gate` already synthesizes the suite
(`taskfiles/spec.yml`, task `suite`, into `.out/suite.json`) and runs it against the interpreter
built into ess (task `conform`).

The runner is the `ess-conformance` crate, taken as surf takes it
(`$HOME/beyond10x/surf/conformance/Cargo.toml`): cargo git dependencies on
`https://github.com/beyond10x/ess` at `tag = "0.96.0"` (`ess-conformance`, `ess-primitives`,
`ess-domain`), in a crate with its own `[workspace]` and its own `Cargo.lock`, which nothing
that ships depends on and the root workspace does not reach (allowed by the ruling of 2026-11-21
of the session that decides for the operator; no blocker). The surf lock pins ess at commit
`9e91e8b5754b2aacbc2c385421b27ee1e9939999`.

What the slice 1 interface reaches, measured on this base. `slot task -d . -t taskfiles/spec.yml
suite` prints `61 scenario(s) (3 authored), 32 refusal(s)`. Of the 18 commands of
`spec/domains/host.yaml`, six have a slice 1 entry point: `CreateAgent` (`Agent::new`),
`CreateRealm`, `ParseScript`, `EvaluateScript`, `Interrupt`, `Collect`. 19 scenarios execute only
those six:

    jq -r '.scenarios | to_entries[] | select([.value.steps[] | select(.step == "execute_command")
      | .command] | all(. as $c | ["swell.host.CreateAgent", "swell.host.CreateRealm",
      "swell.host.ParseScript", "swell.host.EvaluateScript", "swell.host.Interrupt",
      "swell.host.Collect"] | index($c))) | .key' .out/suite.json | wc -l

prints 19. Of these, by their `configure_external_outcome` and `await_outcome` steps:

- 10 force nothing and await nothing: `CreateAgent/outcome/created`, `CreateRealm/outcome/created`,
  `CreateRealm/outcome/no-agent`, `ParseScript/outcome/parsed`, `ParseScript/outcome/no-agent`,
  `ParseScript/outcome/no-realm`, `EvaluateScript/outcome/completed`,
  `EvaluateScript/outcome/no-agent`, `Interrupt/outcome/no-agent`, `Collect/outcome/no-agent`.
  Their only script is `1 + 1`, so no builtin of `story:harness-builtins` is reached and no edge
  to it is needed.
- 3 force an outcome the target produces through the interface: `EvaluateScript/outcome/interrupted`
  (the target calls `interrupt()` on the handle before the call, `HOST-92`), and
  `Collect/outcome/engine-fault` and `Agent/transition/fault/by/Collect/engine-fault` (the
  `fault-injection` feature of `story:host-interface-hardening` makes `collect` find a broken
  invariant, `HOST-71`; hence the edge to that story).
- 5 force an outcome that the scenario input cannot give: `ParseScript` `parse-failed` and
  `memory-budget`, `EvaluateScript` `step-budget`, `memory-budget` and `threw`, each on the
  source `1 + 1` with the limits of its `CreateAgent` (1,000,000 steps, 64 MiB). The target
  answers `unsupported` for them rather than evaluating another source than the one the
  scenario gives.
- 1 awaits a hook the engine cannot issue before slice 10:
  `HostPromiseRejectionTracker/outcome/tracked`; `unsupported`.

So the slice 1 run is 61 scenarios: 13 passed, 0 failed, 0 error, 48 unsupported (the 42 that
execute another command, the 5 and the 1 above). The ceiling on unsupported is that number, 48,
and later slices lower it.

## Additions in wave 6 of slice 1 (2026-11-21)

Binding; where this section and the text below differ, this section governs.

- **The numbers are 14 passed and 44 unsupported of 58**, not 13 and 48 of 61. The unit's
  first run gave 9 of 61: the model's synthesized scenarios parsed on one agent with a realm
  created on another, and the model's example source had no semicolon. The model was
  corrected in the same wave (`spec/`: a Realm carries its agent, `ParseScript` refuses a
  realm of another agent with `foreign-realm`, authored scenarios on the realm's own agent),
  and the numbers were measured on it. The task `conform-engine` fails on fewer than 14
  passed and on more than `ESS_UNSUPPORTED_MAX` (44) unsupported.
- **The planted defect is a target whose `Interrupt` leaves the flag unset.** It must fail the
  authored scenario `interrupt-ends-the-next-run`, and the correct target must fail none.
- **The target depends on `swell-syntax`** beside `swell`: it reads `message_kind` to report a
  refusal of the parser for a later slice as unsupported, never as `parse-failed`.
- **Gate variables**: `ESS_SUITE` and `ESS_UNSUPPORTED_MAX`, both in `GATE_VARIABLES`.

## Change

Failing tests first: `conformance/ess/tests/planted_defect.rs` is written before the target and
fails to build.

- `conformance/ess/Cargo.toml`: package `swell-conformance`, `publish = false`,
  `license = "Apache-2.0"`, `[workspace]` with `resolver = "3"`; dependencies `swell` (path
  `../../crates/swell`, feature `fault-injection`) and the three git dependencies above at
  `tag = "0.96.0"`. `conformance/ess/Cargo.lock`: generated by cargo, committed; it is the one
  place third-party packages are listed, as in surf, and the gate lock check reads the root
  `Cargo.lock` only.
- `conformance/ess/src/lib.rs`, `src/target.rs` (one function per command: the model input to
  the interface call, the interface answer to the model outcome and events; model identifiers
  mapped to `Root`, `Realm` and `Script` values kept per scenario; the model seeds given to
  `Seeds`, a shorter seed padded with zero bytes to the length `Seeds` has, since the
  synthesized scenarios give one-byte seeds; the forced outcomes as Context says),
  `src/values.rs` (model values to Rust values), `src/main.rs` (`swell-conformance <suite>
  <report>`: runs every scenario on a fresh agent per scenario, writes the
  `ess-conformance-report/2`, prints the runner summary line). Every file starts with
  `#![forbid(unsafe_code)]`.
- `conformance/ess/tests/planted_defect.rs`:
  `a_target_whose_interrupt_does_nothing_fails_a_scenario` (the suite over a target whose
  `Interrupt` and forced `interrupted` leave the flag unset reports at least one failed
  scenario).
- `taskfiles/conform-engine.yml`, task `conform-engine`, no `deps` (a `deps` entry naming `suite`
  of `taskfiles/spec.yml` fails when the file is run alone: measured with a probe task file,
  `task -d . -t <file> probe` printed `task: Task "suite" does not exist` and exited 200), and
  variables `ESS_SUITE` (default `.out/suite.json`; missing, the task fails with
  `conform-engine: <file> missing; run task suite first`) and `ESS_UNSUPPORTED_MAX` (default
  48): runs the crate tests with `RUSTFLAGS=-D warnings` (`cargo test --locked --manifest-path
  conformance/ess/Cargo.toml`), checks that the planted-defect test ran and passed, runs the
  suite into `.out/conform-engine.json`, and fails when a scenario failed or errored, when fewer
  than 13 passed, or when more than `ESS_UNSUPPORTED_MAX` were unsupported, naming both numbers;
  it prints the runner summary line.

The coordinator includes `taskfiles/conform-engine.yml` in the root `Taskfile.yml`, adds
`- task: conform-engine` to `check` after `conform` (which runs `suite` first), and adds
`ESS_UNSUPPORTED_MAX` to `GATE_VARIABLES`. The root `Cargo.lock` does not change.

Writes: `conformance/ess/Cargo.toml`, `conformance/ess/Cargo.lock`,
`conformance/ess/src/{lib,target,values,main}.rs`, `conformance/ess/tests/planted_defect.rs`,
`taskfiles/conform-engine.yml`.

## Acceptance

`slot task -d . -t taskfiles/spec.yml suite` and then `slot task -d . -t
taskfiles/conform-engine.yml conform-engine` exit 0; the second prints the runner summary line
`61 scenarios: 13 passed, 0 failed, 0 error, 48 unsupported` after the test
`a_target_whose_interrupt_does_nothing_fails_a_scenario` ran and passed.

## Out of scope

The twelve commands without a slice 1 entry point (slices 10 and 11); the five forced outcomes
the scenario input cannot give; changing the ESS model or its scenarios; coverage-guided
fuzzing. The root `Taskfile.yml`, `Cargo.toml` and `Cargo.lock` (the coordinator).

## Domain relations

- Agent to Realm, one-to-many, owns, via `agent_id`: inferable (`spec/domains/host.yaml`, entity
  `swell.host.Agent`, relation `realms`). The target keeps the realms of a scenario per agent.
- Script to Realm, many-to-one, references, via `realm_id`: inferable (`spec/domains/host.yaml`,
  the Script entity, relation `realm`). `ParseScript` takes the realm the model names.
