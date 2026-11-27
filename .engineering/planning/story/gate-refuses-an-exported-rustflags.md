---
format: aep.planning-md/3
id: story:gate-refuses-an-exported-rustflags
kind: story
status: draft
title: The gate refuses an exported RUSTFLAGS
summary: An exported RUSTFLAGS wins over the env of a gate step (Task 3.54.0), so a local gate can build with other flags than the forge, unseen. Backlog, from pass 1 of the adversary on unit realmwalk of wave big-2.
revision: 1
---
## Context

Found by pass 1 of the adversary on the unit `realmwalk` of wave big-2
(`review-result:adversary-realmwalk-pass-1`, finding 13; read in
`review-result:realmwalk-correction-1-coordinator-read`). It is older than the unit.

The gate's steps set `RUSTFLAGS` in the `env:` of their task (`-D warnings`; since the unit,
`TEST_RUSTFLAGS` of `taskfiles/realm-places.yml` for `build`, `build-tests`, `test`,
`test-doc` and the writer of the realm places). Task 3.54.0 gives a step the exported value
when the shell that starts `task` already exports `RUSTFLAGS`: the adversary read
`RUSTFLAGS=outer` in a step whose `env:` names a literal. So a gate started from a shell
with an exported `RUSTFLAGS` builds with other flags than the forge, and no line of the gate
says so: `test-vars-check` compares the template value, not the environment, and `gate-vars`
refuses only a value that names `test_modules`.

The unit left literal copies of the flags in other steps (`slow-tests`, `build-realm-sizes`,
`taskfiles/collect-mode.yml`, `taskfiles/test-binaries.yml`, and the steps with a manifest or
a target of their own); `AGENTS.md` names them ("Commands and the gate", the bullet on the
walk of the realm's functions).

The finding is older than the wave and is filed here, in the backlog. No wave takes this
story before the planned waves are delivered.

## Change

To be written when the story is planned. The direction: the gate's first step (`gate-vars`)
fails with one line when `RUSTFLAGS`, `CARGO_ENCODED_RUSTFLAGS` or `CARGO_BUILD_RUSTFLAGS` is
exported with a value other than the one the step would set, or each step sets the value in
its command (`RUSTFLAGS=... cargo ...`) so that the exported one cannot win; and the literal
copies read one definition where their step shares a build with the gate's test build.

## Acceptance

To be written when the story is planned: `RUSTFLAGS=--cfg=outer task check` fails at its
first step with a line that names the variable, or every step is shown to compile with the
gate's flags under it; a selftest under `tests/gate/` holds it.
