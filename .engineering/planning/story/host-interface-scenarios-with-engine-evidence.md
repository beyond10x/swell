---
format: aep.planning-md/3
id: story:host-interface-scenarios-with-engine-evidence
kind: story
status: draft
title: Scenarios that give the engine evidence for the faulted agent and the budgets
relations:
- depends_on: story:ess-conformance-target
revision: 1
---
## Context

Pass 1 of the hardening of the host interface specification (`review-result:harden-host-interface-pass-1`) planted two defects in the engine. The first (`begin()` no longer clears the interrupt flag) failed one scenario of the engine suite. The second (`alive()` answers `Ok` for a faulted agent) changed no verdict: 14 passed, 0 failed. Three rules have no evidence on the engine because their only scenario is `unsupported` there for a step that has nothing to do with the rule: the faulted agent (its scenario faults through EvaluateScript), the step budget of EvaluateScript (its last step is a microtask checkpoint of slice 10) and its memory budget (it first forces ParseScript into the memory budget, which the target cannot reach). Five of the 14 engine passes are NotHeld answers the adapter gives without calling the engine.

## Change

Authored scenarios that the engine target runs whole, for findings 16, 19, 20, 21 and 23 of that result: a faulted agent refuses every call, faulted by a path the target supports (Collect with the fault switch, or what the unit finds); the step budget ends one run of EvaluateScript with no later step of another slice; the memory budget ends one entry of EvaluateScript without ParseScript's budget; `faulted` before `foreign-realm` in ParseScript (`HOST-48`); ParseModule on a faulted agent and EvaluateScript without a script on the interpreter. And the three relations the pass names as runnable today on the engine, as paired scenarios: a failed parse changes nothing; a run ended by a budget or an interrupt leaves later answers as they would be without it (`HOST-70`); a Collect between two calls changes no later answer.

For each new scenario the unit plants the defect it is to catch (the second defect above for the first one), shows the red line, restores, shows green. Every scenario file needs the policy admission of the deciding session before its commit.

Writes: `spec/scenarios/` (new files), `conformance/ess/src/target.rs` and `conformance/ess/tests/` where the target must support a step it answers `unsupported` today, `taskfiles/conform-engine.yml` (the floor of passed scenarios).

## Acceptance

`slot task conform` and `slot task conform-engine` exit 0; the engine passes at least 14 plus the number of new scenarios; with `alive()` answering `Ok` for a faulted agent planted in a copy, the engine suite fails by name; the report quotes each red line.
