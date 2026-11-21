---
format: aep.planning-md/3
id: review-result:slice-01-scope-round-1
kind: review-result
status: active
title: scope critic, slice 1 stories, round 1
relations:
- reviews: story:num-and-unicode-early-parts
- reviews: story:syntax-es5-and-strict
- reviews: story:heap-values-and-host-surface
- reviews: story:bytecode-compiler
- reviews: story:object-model-and-realms
- reviews: story:interpreter-and-script-entry-points
- reviews: story:harness-builtins
- reviews: story:host-interface-hardening
- reviews: story:test262-run-path
- reviews: story:ess-conformance-target
revision: 1
---
needs-revision

story:interpreter-and-script-entry-points — its `## Requirements` section names `HOST-10` and `HOST-16`, which `story:host-interface-hardening` claims in its own `## Requirements`, so a program check finds each of them named by two stories; the pointer sentence belongs in Context — .engineering/planning/story/interpreter-and-script-entry-points.md:76
story:host-interface-hardening — it claims epic acceptance item 13 ("The same script run twice prints the same bytes"), which `story:interpreter-and-script-entry-points` already claims with `same_script_twice_gives_the_same_steps_and_value`, so the item has two owners; one body should hold it and the other should only cite it — .engineering/planning/story/host-interface-hardening.md:79

**What I read:** 11 artifacts: `epic:slice-01-harness`, read first and turned into a list of promises, then the ten stories. Commands: `aep plan artifact show` on each, plus reading the store files with line numbers; `aep plan artifact graph`; `aep plan artifact validate` (`valid`). I also read decision 3 and "A part before its slice" in `docs/design/engine.md` (lines 760-783).

**The requirements check, by program:** I took the `[slice 1]` lines of `docs/design/host-interface.md` (68 of its 148 requirements) and compared them with the `HOST-<n>` named inside each story's `## Requirements` section:
- None of the 68 is missing, and no story names a requirement outside them.
- `HOST-10` and `HOST-16` are named twice: the first finding.
- The other 66 are named once:

| Story | Requirements |
|---|---|
| `story:heap-values-and-host-surface` | 17 |
| `story:object-model-and-realms` | 9 |
| `story:interpreter-and-script-entry-points` | 28, plus `HOST-54` until it is split |
| `story:host-interface-hardening` | 13 |

The scratch directory `.out/critic-scope/` is removed.

**The epic's promises:** 33 extracted, 33 traced:
- 8 outcome items.
- 6 open items, all settled:
  - `[slice 1]` marks found wrong in part: reported by the interpreter and hardening stories.
  - The builtins: `story:harness-builtins`.
  - The `$262` members: `story:test262-run-path`.
  - The collector, in this slice: `story:heap-values-and-host-surface`.
  - The early `swell-num` and `swell-unicode` parts: `story:num-and-unicode-early-parts` and `story:syntax-es5-and-strict`.
  - The size of `Value` and the Rust stack a re-entry needs: the heap and interpreter stories.
- 5 constraints.
- 14 acceptance items. Only item 13 has two owners (second finding). Item 1 traces at 31 cases, which you have already queued.

Nothing drafted lands in the epic's exclusions. The parts taken early are each traced to a source:
- the `Zs` table, its generator and the `unicode-tables-check` gate task: the epic's open item on early parts and the design's rule "A crate is created in the first slice that needs any part of it";
- `GetUTCEpochNanoseconds`: `HOST-124`;
- the three limit omissions in `sections.txt`: `docs/design/host-interface.md:166`;
- the check that a `test` line names a real test: left to this slice by the foundation's `story:host-requirements-gate`.

**What I could not establish:**
- **The 51 moved cases are not re-checked.** I did not re-measure the move list or the new slice counts (`01 31 62` and the others) against the checkout. That belongs to the acceptance critic.
- **Item 9 has no story that names it.** Epic acceptance item 9 (memory budget) is covered in substance twice: `memory_budget_ends_with_cx_in_memory_budget` in `story:heap-values-and-host-surface` and `HOST-10` in `story:host-interface-hardening`. Neither story says it holds item 9, so this is not a gap.
- **Out of my lane (design):**
  - `story:syntax-es5-and-strict` refuses a regular expression literal "where the lexer reads one" (line 87). The design says the literal "is lexed to its end and evaluating it throws a `SyntaxError`" (`engine.md:778`).
  - `story:syntax-es5-and-strict` still refuses `Zs` white space in source, while `story:num-and-unicode-early-parts` brings the `Zs` table into slice 1 for `StringToNumber`.
- **Out of my lane (acceptance):** `doneprintHandle.js` for `async` cases is named only in the Context of `story:test262-run-path` (line 19); no test in its Change exercises it.

```findings
- file: .engineering/planning/story/interpreter-and-script-entry-points.md
  line: 76
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its `## Requirements` section names `HOST-10` and `HOST-16`, which `story:host-interface-hardening` claims in its own `## Requirements`, so a program check finds each of them named by two stories; the pointer sentence belongs in Context"
- file: .engineering/planning/story/host-interface-hardening.md
  line: 79
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "it claims epic acceptance item 13 (\"The same script run twice prints the same bytes\"), which `story:interpreter-and-script-entry-points` already claims with `same_script_twice_gives_the_same_steps_and_value`, so the item has two owners; one body should hold it and the other should only cite it"
```
