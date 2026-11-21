---
format: aep.planning-md/3
id: review-result:foundation-design-round-2
kind: review-result
status: active
title: Design critic, foundation stories, round 2
relations:
- reviews: story:workspace-and-gate
- reviews: story:test262-fetch
- reviews: story:unicode-data-pin
- reviews: story:spec-gate
- reviews: story:test262-counting-runner
- reviews: story:test262-expected-and-sections
- reviews: story:host-requirements-gate
revision: 1
---
approve

Both round-1 findings are fixed, and I found nothing new that would change the drafted set.
- **The `swell` crate:** it now has no public item, and nothing in it stands in for the host interface (`.engineering/planning/story/workspace-and-gate.md:25`).
- **The runner:** it has no dependency and calls no engine. One function in its `count` module marks every run as failed, and slice 1 changes only that function (`.engineering/planning/story/test262-counting-runner.md:25`, `:166`).

The set holds together without the placeholder.

**What I read:**
- All 7 stories and `epic:foundation` in full, with `aep plan artifact show`.
- `aep plan artifact graph`, `validate` (31 artifacts, valid) and `waves`.
- The edge meanings, `AGENTS.md`, the unit invariants and the design, carried over from round 1.

**What I checked:**
- **Edges:** I walked 53, inside and outside the set: 10 within the set (7 `decomposes`, 3 `depends_on`), 24 `reviews` edges from the round-1 review results, and 19 `depends_on` edges between the slice epics. There is no cycle.
- **Order:** only workspace-and-gate → counting runner → expected-and-sections is ordered, with fetch also feeding the runner. Unicode data, spec gate and host requirements stand alone, so the set is not one long queue.
- **Acceptance reads:** 8 lines read something another story in the set produces, and an edge records all 8.
  - The runner reads 5: the root `Cargo.toml`, the root `Taskfile.yml` with its includes, `lock-check`, `conformance/test262/PIN` and the `test262-fetch` task.
  - Expected-and-sections reads 3: `crates/test262`, `taskfiles/test262.yml` and the runner's test fixtures.
- **Shared files:** the runner and expected-and-sections both write `crates/test262/src/main.rs`, `src/lib.rs` and `taskfiles/test262.yml`. Each story still works on its own, and an ordering edge covers the overlap, so this is not a split abstraction.
- **Gate wiring:** the root `Taskfile.yml` names five optional include files, and each one has a story in the set that writes it.

**What I could not establish:**
- spec-gate and host-requirements-gate depend on `spec/` and on `docs/design/host-interface.md`, which come from outside the set. The design document is not in this tree yet, so no edge can record that dependency. That is not a design defect.
- Not my lane (parallel safety): the runner's claim that it "is alone in its wave", which lets it rewrite `Cargo.lock`. No story records a scope, so `waves` reports all 7 as unassessed.
- Not my lane (acceptance): the epic's acceptance relies on the coordinator adding each story's `- task:` line to `check`. No story's acceptance checks that the lines were added.
- Housekeeping: I wrote one scratch file of `aep plan artifact show` output under `.out/critic-design-r2/` in the store worktree (git ignores `.out/`). I removed it in the same session, so the tree is as I found it.

```findings
[]
```
