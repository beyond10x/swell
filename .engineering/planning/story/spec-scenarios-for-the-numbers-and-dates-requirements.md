---
format: aep.planning-md/3
id: story:spec-scenarios-for-the-numbers-and-dates-requirements
kind: story
status: draft
title: Scenarios for the clock, time zone, random and platform requirements of slice 4, or the limit that stops each
summary: The specification author states HOST-26, 27, 125 to 128, 135, 144, 147 and 209 to 213 as ESS scenarios where ESS can, and names the limit in the model where it cannot.
relations:
- depends_on: story:date-object-clock-and-time-value
- depends_on: story:date-object-constructor-now-utc-and-getters
- depends_on: story:date-text-formats
- depends_on: story:math-functions
- depends_on: story:math-random
scope:
- confidence: inferred
  path: conformance/ess/src/target.rs
- confidence: inferred
  path: conformance/ess/tests
- confidence: cited
  path: docs/design/host-interface-model.md
- confidence: cited
  path: spec/domains/host.yaml
- confidence: inferred
  path: spec/scenarios
- confidence: inferred
  path: taskfiles/conform-engine.yml
revision: 3
---
## Context

The host interface specification under `spec/` (the ESS model: `spec/domains/host.yaml`, `spec/system.yaml`, `spec/scenarios/*.yaml`) states what a host sees of the engine; the document `docs/design/host-interface.md` states the same as `HOST-<n>` requirements, each held by a test of the engine or by a marker. Slice 4 adds or finishes 14 requirements that no scenario states: HOST-26, HOST-27, HOST-144, HOST-147, HOST-212 and HOST-213 (`Math.random`, from the seed), HOST-125 and HOST-126 (the clock), HOST-127 (the text of a date), HOST-128 and HOST-209 to HOST-211 (the time zone hooks) and HOST-135 (the same bytes on every platform). On the base `spec/scenarios/` has 8 files (`step-budget-ends-one-run.yaml` and the others: budgets, interrupt, parse errors, a faulted agent, top-level `await`), none of them for these. `spec/domains/host.yaml` (the comment above `swell.host.Agent`, about lines 129 to 146) says that the clock, the time zone operations and the random bytes "are not stated at all", and carries `HOST-4, HOST-26, HOST-27, HOST-83, HOST-84, HOST-116 to HOST-128`; `docs/design/host-interface-model.md` line 96 (row 4 of its table) carries the same list. The design branch added HOST-209 to HOST-211 to `host-interface.md`'s own copy of that list; the model file and the row do not have them.

This story is the specification author's. It does not change the engine. It states, as scenarios the executable-specification target runs against the engine, each of the 14 requirements that ESS can express, and for each that it cannot it writes the reason into the marker of `host.yaml` that already carries the requirement, so that a requirement is never carried by nothing. It depends on the stories that make the behaviour true (the target runs the scenarios against the engine): `story:date-object-clock-and-time-value` (HOST-125, 126), `story:date-object-constructor-now-utc-and-getters` (HOST-128, 209 to 211), `story:date-text-formats` (HOST-127), `story:math-functions` (HOST-135) and `story:math-random` (the rest).

**Precondition: what ESS can state.** ESS has commands a caller sends, events the system publishes and system-issued commands; the comment in `host.yaml` says that a hook that returns an answer to the engine "is not a call back with an answer the system waits for", so the clock and the zone hooks (HOST-125 to 128, 209 to 211) probably cannot be scenarios at all, and a requirement about two agents (HOST-26, 144, 213) needs a scenario that compares the completion values of two scripts. The unit starts with a probe, in the scratch directory, that writes the smallest scenario of each kind (a script whose completion value is a Number, run in two agents with one `random_seed` and the second compared with the first through a `capture` and a later `input`, or an `outcome` field that can hold a captured value; one hook answer forced by `external:`) and runs it through `slot task conform-engine`; its result, the line of the ESS documentation or of `ess` that allows or refuses each, is the first thing the report quotes. Decision recommended for a form ESS cannot state: no scenario, the marker keeps the requirement and names the limit (`ESS limit` is the form `host-requirements-check` accepts; the unit reads the existing markers and copies their form), and the requirement keeps its test in the engine (the stories above own those tests).

Design it follows, by name: `docs/design/host-interface.md` HOST-26, 27, 125 to 128, 135, 144, 147 and 209 to 213, decision 17 (the time hooks), decision 3 (`random_seed`); `docs/design/host-interface-model.md` (the gaps and where each is carried); the existing scenarios for the form (`step-budget-ends-one-run.yaml`: `type: ess-scenario/6`, `arrange`, `timeline`, `outcome`, `capture`).

## What later stories reuse

The scenarios are part of the gate (`task spec`, `task conform`, `task conform-engine`). A later slice that adds a hook states it in the model here the same way.

## Domain relations

- Requirement HOST-n -> scenario or marker: each requirement of this list has one or the other, never neither - stated (`host-requirements-check`: an UNMAPPED marker names a `HOST-<n>` of the document).
## Change

Failing checks first: `slot task spec` and `slot task conform` pass on the base; the failing state is the scenario written and not yet true, or the marker list that lacks HOST-209 to HOST-213 (a reading, by `grep -c "HOST-209" spec/domains/host.yaml docs/design/host-interface-model.md`, which prints 0 and 0 on the base).

- The probe (above), reported first. If ESS can state the equality of two completion values across two agents, the unit writes: `spec/scenarios/math-random-follows-the-seed.yaml` (two agents with one `random_seed`, the first realm of each, a script that returns the first value of `Math.random()` in each: equal; a third agent with another seed: different) for HOST-26 and HOST-144; `spec/scenarios/math-random-is-per-realm.yaml` (two realms of one agent: different; the second realm of two agents with one seed: equal, with the calls in the other order) for HOST-27 and HOST-213; `spec/scenarios/math-random-is-a-number-in-range.yaml` if a script can return the booleans of HOST-212 (a loop of 1,000 values, the range, the sign and the 53 bits, returned as `true`) for HOST-212. HOST-147 (two processes) and HOST-135 (every release platform) are about runs outside one ESS timeline: they get markers, not scenarios.
- The clock and the zone: if the probe shows that a forced hook answer is expressible (`external:`), `spec/scenarios/date-reads-the-host-clock.yaml` (the hook answers one instant; `Date.now()` returns its millisecond; a second call after a second answer returns that one) for HOST-125 and HOST-126, and `spec/scenarios/date-reads-the-host-time-zone.yaml` (the identifier `+01:00`: `new Date(0).getHours()` is 1 and the offset hook is not called; an identifier that is a name: the offset hook answers 3,600 seconds and `getHours` is 1; an offset out of range is clamped: `getHours()` 23 and `getMilliseconds()` 999 for the largest answer, HOST-209) for HOST-128, 209 and 211; HOST-210 (an empty answer list gives NaN after 31 calls) and HOST-127 (`Date.parse` of its own text) as the probe allows. Otherwise: no scenario and the markers below.
- `spec/domains/host.yaml`: the comment above `swell.host.Agent` lists HOST-209 to HOST-213 beside `HOST-116 to HOST-128` ("Carried by HOST-4, HOST-26, HOST-27, HOST-83, HOST-84, HOST-116 to HOST-128, HOST-135, HOST-144, HOST-147, HOST-209 to HOST-213" in the form the file uses), and each sentence that said the clock, the zone operations or the random bytes are not stated at all says which of them a scenario now states and which stays unstated and why (an ESS limit, named). `grep -n "not stated at all" spec/domains/host.yaml` prints one line before and either nothing or a rewritten sentence after.
- `docs/design/host-interface-model.md` line 96 (row 4 of the table of gaps): `HOST-116 to HOST-128` becomes the list above in the row's form; the lines of the same row that say what is carried by decisions 1, 3, 16 and 17 are read and kept. `docs/design/host-interface.md` has its copy of the `host.yaml:129` list since the design branch; the unit checks that the two lists are equal (`grep -n "HOST-209" docs/design/host-interface.md docs/design/host-interface-model.md spec/domains/host.yaml` prints one line in each of the three).
- The target. If a scenario needs a step that `conformance/ess/src/target.rs` answers `unsupported` today (a script that returns a Number, a second agent with another `random_seed`), the unit adds that step to the target (the smallest change; `conformance/ess/tests/` gets a test), and raises the floor of passed scenarios in `taskfiles/conform-engine.yml` (`fewer than 14 passed` in its description and the check, and `ESS_UNSUPPORTED_MAX`, default 44, lowered by the steps it now supports, each number measured and stated: the unit reports the line `<n> scenarios: <p> passed, <f> failed, <e> error, <u> unsupported` before and after).
- For each new scenario the unit plants the defect it is to catch (for the seed scenarios: `Math.random` that ignores the realm's index in a copy; for the zone scenarios: the offset clamp removed in a copy), shows the red line, restores, shows green, as `story:host-interface-scenarios-with-engine-evidence` does. Every scenario file needs the policy admission of the deciding session before its commit; the unit lists the files and asks the coordinator, and commits none.
- Time: this story adds no engine test and no build of the engine beyond the ESS target; `slot task spec`, `slot task conform` and `slot task conform-engine` each run in the pool (a pool job ends after 10 minutes; the unit measures each and states the times; a command over 10 minutes names `slot --timeout 30m`). The probe scripts are not committed. No Python in any command.

Writes: `spec/scenarios/` (new files, up to 5 as above), `spec/domains/host.yaml` (the comment above `swell.host.Agent` and the markers it carries), `docs/design/host-interface-model.md` (line 96), `conformance/ess/src/target.rs` and `conformance/ess/tests/` (only where a step is `unsupported`), `taskfiles/conform-engine.yml` (the floor, if it rises). Files shared with other stories: `spec/domains/host.yaml` with `story:spec-diff-in-the-gate` (draft: it may add a diff classification, not these lines; the join takes both), `taskfiles/conform-engine.yml` with the same story if it touches the floor. No engine crate is written. Size: about 350 lines of YAML and prose, one session; the probe may end it earlier.

## Requirements

None moves: the requirement lines are the engine stories'. `slot task -d . -t taskfiles/host-requirements.yml host-requirements-check` exits 0 before and after (every UNMAPPED marker names a `HOST-<n>` of the document, now including 209 to 213).

## Acceptance

`slot task spec`, `slot task conform` and `slot task conform-engine` exit 0 after `slot task suite`; the engine passes at least 14 plus the number of new scenarios that the probe allows (the report states the number, and the floor in `taskfiles/conform-engine.yml` is that number); with the defect planted in a copy for each new scenario, the engine suite fails by name (the report quotes each red line); `grep -n "HOST-209" docs/design/host-interface.md docs/design/host-interface-model.md spec/domains/host.yaml` prints one line in each of the three files; `slot task -d . -t taskfiles/host-requirements.yml host-requirements-check` exits 0 with the same counts of test and pending lines as before; the probe's result for each of the 14 requirements (scenario, or the limit that stops it) is a table in the report.

## Out of scope

The engine, the stories that make the behaviour true, and the requirement lines of `conformance/host/requirements.txt`; a hook of the clock or the zone as an ESS construct, if the probe shows that ESS has none (that is a request to the ESS maintainers, named in the report, not a change here); the epic and `CHANGELOG.md` (the coordinator).
