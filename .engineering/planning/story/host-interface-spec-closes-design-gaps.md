---
format: aep.planning-md/3
id: story:host-interface-spec-closes-design-gaps
kind: story
status: draft
title: The host interface specification declares what the design states
relations:
- depends_on: story:spec-gate
revision: 1
---
## Context

Pass 1 of the hardening of the host interface specification (`review-result:harden-host-interface-pass-1`, skill `ess:hardening`, 2026-11-22) reviewed `docs/design/host-interface.md` against `spec/`. Of the 165 requirements, 109 are named by an `UNMAPPED:` marker and 12 by a declaration; 26 of the rest are stated by the design and neither declared nor marked. Several markers give a reason that no longer holds on the pinned ESS release (0.96.0): the unit validated edited copies to show it.

## Change

An ESS author changes `spec/domains/host.yaml` and `spec/system.yaml` for findings 1 to 8 and 10 to 13 of that result, each as the result states it: the two refusals of CreateRealm (`HOST-12`, `HOST-41`); LinkModule on a New module (`HOST-42`); the terminations of LoadRequestedModules, FinishLoadingImportedModule and LinkModule (`HOST-75`, `HOST-76`); `ForeignHandle` for a module or load of another agent (`HOST-31`, `HOST-153`); `queue_microtask` and the two job hooks (`HOST-78`, `HOST-82`); `InterruptHandle::clear` as a command (`HOST-93`); `CreateError::AgentKeysExhausted`; a parse refused for a later slice (`HOST-164`); and the markers whose reason is stale or wrong (lines 40, 123, 147, 186, 222, 929, 967, 1034, 1229 of `host.yaml`; the carried-by lists of `system.yaml`). A rule the pinned release cannot express keeps a marker with the reason that holds now. Finding 9 (which entry points read the interrupt flag: decision 12 against decision 5) is a design change a design author makes first.

Every scenario file this adds or edits under `spec/scenarios/` needs the policy admission of the deciding session before its commit. The seed of the synthesis is quoted nowhere.

Writes: `spec/domains/host.yaml`, `spec/system.yaml`, `spec/scenarios/` (new files), `conformance/ess/src/` where a new command needs the target to answer `unsupported` with its reason, `docs/design/host-interface.md` (the marker counts it quotes).

## Acceptance

`slot task validate`, `slot task conform` and `slot task conform-engine` exit 0; the interpreter passes every scenario; the engine line counts no failure and no error, and its passed count is at least 14; the count of `UNMAPPED:` markers the gate prints is stated before and after with the reason for each one removed; the review script of the hardening pass (`review.sh` in its scratch directory, copied into the unit's own) reports nothing missing for the findings above.
