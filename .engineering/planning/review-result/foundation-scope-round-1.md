---
format: aep.planning-md/3
id: review-result:foundation-scope-round-1
kind: review-result
status: active
title: Scope critic, foundation stories, round 1
relations:
- reviews: story:workspace-and-gate
- reviews: story:test262-fetch
- reviews: story:unicode-data-pin
- reviews: story:spec-gate
- reviews: story:test262-counting-runner
- reviews: story:test262-expected-and-sections
revision: 1
---
approve

**What I read:** 7 artifacts: `epic:foundation`, read first and turned into a list of promises, then the six stories. Commands: `aep plan artifact show` on each, `aep plan artifact graph`, `aep plan artifact relations`, `aep plan artifact validate` (`valid`, 26 artifacts). I also read `AGENTS.md`, `docs/plan/unit-invariants.md` and the design's decisions 1, 2, 8, 10, 11, 13 and 14 and section 12, plus `.gitignore`. The epic makes 31 promises and all 31 trace to exactly one story:
- 15 outcome promises: the workspace, the 7 conditions `check` fails on, fetch, runner, `features.txt`, `sections.txt`, `expected.txt`, the Unicode pin, `spec/` in the gate.
- 3 open items.
- 5 constraints.
- 8 acceptance items.

No story claims something the epic did not ask for. No two stories claim the same outcome. Nothing else in the graph decomposes the epic.

**What I could not establish:**
- **Promises handed to the coordinator, not a story.** Three things the epic promises are passed to the coordinator, and each story says so by name, so none is a silent gap:
  - adding each story's `- task:` line to `check`;
  - writing the slice line into decision 11 (`epic/foundation.md:63-64`);
  - moving the Unicode items out of "Not checked".
- **No story runs the whole gate.** No story's acceptance runs `slot task check` with every task in it. The epic's acceptance (`epic/foundation.md:102`) is only met when the wave runs the full gate, as `docs/plan/unit-invariants.md:47-48` intends.
- **The ECMA-262 pin is left out on purpose.** The epic says it carries out decision 1, "the pins" (`epic/foundation.md:19`). Only the test262 pin is written to a file (`story/test262-fetch.md:52-56`). That story names the ECMA-262 pin as out of scope (`story/test262-fetch.md:104-105`), so this is an honest omission, not a gap. Whether decision 1's "both pins move together" should also be held by a file is for the operator.
- **Out of my lane (acceptance or design):** `story:test262-expected-and-sections` says "clauses 20 to 28" in its summary (line 7) but "clauses 19 to 28" in its rule (line 25).
- **Not re-measured.** I did not check the stories' measured numbers (33 sections, 278 lines, 221 directory lines) against the checkout. That belongs to the acceptance critic.

```findings
[]
```
