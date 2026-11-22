---
format: aep.planning-md/3
id: story:spec-diff-in-the-gate
kind: story
status: draft
title: The gate classifies a change of the host interface specification
relations:
- depends_on: story:local-gate-equals-forge
revision: 1
---
## Context

Pass 1 of the hardening of the host interface specification (`review-result:harden-host-interface-pass-1`, technique 7) showed that the pinned ESS release classifies a change of `spec/`: a removed enum variant and an added required input each exit 4 with a named line under `ess verify diff --fail-on breaking-or-unknown`; `spec/` against release 0.2.0 exits 0; against 0.1.0 it exits 4 on two changes classified `unknown`. Nothing in the gate holds this today, so a narrowing of the host interface could be released as if it were additive. surf will depend on swell by a release tag.

## Change

A task `spec-diff` in the gate: it writes `spec/` as it is at the last release tag into `.out/` (from the repository's own objects, no network) and runs `ess verify diff --from <that> --to spec --fail-on breaking-or-unknown` with the pinned release. A change that is breaking or unknown fails the gate unless a committed acknowledgement file outside `spec/` (`ess-diff-acknowledgements/1`) names it. A planted-defect script under `tests/gate/` shows the refusal on a copy (a removed variant) and runs under 5 s. The forge's checkout must hold the tags: say what the workflow needs and list it for the coordinator. A known limit goes into the task's summary: a prose edit of an external cause is classified `unknown` (finding 14), so a wording fix needs an acknowledgement.

Writes: `taskfiles/spec.yml` or a new task file, the root `Taskfile.yml` (the gate's step and its variables), `tests/gate/` (one script), the acknowledgement file (you name it), `.github/workflows/check.yml` (the fetch of tags only), `docs/releasing.md` is the coordinator's.

## Acceptance

`slot task spec-diff` exits 0 on the tree and prints the tag it compared with; the planted script passes in `gate-selftest`; the step runs under 2 s and is counted in the gate's time line.
