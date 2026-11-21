---
format: aep.planning-md/3
id: review-result:foundation-scope-round-2
kind: review-result
status: active
title: Scope critic, foundation stories, round 2
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

**What I read:** 8 artifacts, all from scratch: `epic:foundation` (revision 2, read first and turned into a fresh list of promises), then all seven stories. Commands: `aep plan artifact show` on each, plus reading the store files under `.engineering/planning/` with line numbers; `aep plan artifact graph`; `aep plan artifact validate` (`valid`, 31 artifacts). I also read `AGENTS.md` "Rules" lines 44-73 and checked that `spec/` is in the tree, where `grep -rc 'UNMAPPED:' spec` finds 49 markers in `domains/host.yaml` and 1 in `system.yaml`. The epic makes 37 promises and all 37 trace to exactly one story:
- 20 outcome promises, including the new host-requirements bullet split into 5: the list, its three gate failures, and every line `omitted` with its slice.
- 3 open items.
- 5 constraints.
- 9 acceptance items.

`story:host-requirements-gate` covers the new promise: its 8 planted scripts include the three cases acceptance item 9 asks for (`epic/foundation.md:126-129`). No story claims something the epic did not ask for, no two stories claim one outcome, and only this set decomposes the epic.

**What I could not establish:**
- **Some requirements may not get a slice.** The epic says every requirement starts `omitted` "with the slice that will meet it" (`epic/foundation.md:65`). Where the document gives no slice, `story:host-requirements-gate` writes `slice unassigned` and lists it in the report for the coordinator (`story/host-requirements-gate.md:41-43`). That is named, so it is not a silent narrowing.
- **`test` lines are refused for now.** The same story refuses every `test` line until a later story checks that the named test exists (lines 44-50). That is stricter than the epic and also named.
- **The runner no longer calls the engine.** The epic says cases are counted "against the engine that runs nothing" (`epic/foundation.md:44-46`). Now `crates/swell` has no item and the runner has no dependency; a stub inside the runner marks every run failed (`story/test262-counting-runner.md:20-25`, `story/workspace-and-gate.md:25-29`). The promised outcome, every case counted and none passed, is still covered. Whether this departs from decision 2's "depends on `swell`" is for the design critic, not me.
- **Not checkable yet:** `docs/design/host-interface.md` is not in this tree, so the requirement count cannot be checked here. The story says it measures the count on the wave's base.
- **The ECMA-262 pin is still out on purpose.** `story/test262-fetch.md:106-107` names it as out of scope, so it is an honest omission, not a gap.

```findings
[]
```
