---
format: aep.planning-md/3
id: review-result:slice-01-scope-round-2
kind: review-result
status: active
title: scope critic, slice 1 stories, round 2
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
approve

Both round-1 findings are fixed:
- **`HOST-10` and `HOST-16` named twice:** the sentence pointing them to `story:host-interface-hardening` now sits in Context, not in `## Requirements` (`story/interpreter-and-script-entry-points.md:45-47`).
- **Acceptance item 13 had two owners:** `story:interpreter-and-script-entry-points` now holds it (lines 129-130). `story:host-interface-hardening` only cites it (lines 39-42), and also says item 9 belongs to `story:heap-values-and-host-surface`.

**What I read:** 11 artifacts: `epic:slice-01-harness` revision 3, read first and turned into a fresh list of promises, then all ten stories. Commands: `aep plan artifact show` on each, plus reading the store files with line numbers; `aep plan artifact validate` (`valid`).

**The requirements check, by program:** `docs/design/host-interface.md` has 159 requirements, 68 marked `[slice 1]`. I compared those with every `HOST-<n>` inside the ten `## Requirements` sections:
- No duplicate, none missing, none extra, and no starred line.
- The 68 split as follows:

| Story | Requirements |
|---|---|
| `story:heap-values-and-host-surface` | 17 |
| `story:object-model-and-realms` | 9 |
| `story:interpreter-and-script-entry-points` | 28 |
| `story:host-interface-hardening` | 14 |

- The split-off later halves, `HOST-152` to `HOST-159`, are all marked for slices 2 to 14.
- The scratch directory `.out/critic-scope/` is removed.

**The epic's promises:** 33 extracted, 33 traced, each to exactly one story:
- 8 outcome items.
- 6 open items, all settled:
  - which requirements are slice 1's: the split document;
  - the builtins: `story:harness-builtins`;
  - the `$262` members: `story:test262-run-path`;
  - the collector: `story:heap-values-and-host-surface`;
  - the early `swell-num` and `swell-unicode` parts: `story:num-and-unicode-early-parts` and `story:syntax-es5-and-strict`;
  - the size of `Value` and the Rust stack a re-entry needs: `story:heap-values-and-host-surface` and `story:interpreter-and-script-entry-points` (re-measured through the builtins by `story:host-interface-hardening`).
- 5 constraints.
- 14 acceptance items, each held by one story:

| Story | Acceptance items |
|---|---|
| `story:interpreter-and-script-entry-points` | 5, 6, 7, 8, 10, 13 |
| `story:heap-values-and-host-surface` | 9, 11, 14 |
| `story:test262-run-path` | 1, 3, 4, 12 |

Item 2 is met by the baseline: before slice 1 every pass count is 0. Nothing drafted lands in the epic's exclusions.

**What I could not establish:**
- **One promise is deferred, by name.** The epic promises `doneprintHandle.js` for `async` cases (`epic/slice-01-harness.md:47-48`). `story:test262-run-path` defers it to slice 10, with its reason, in Context (lines 26-30) and Out of scope (line 176). None of the 31 cases is `async`, and the epic puts promises and jobs in slice 10. Because the drafter names it, it is not a gap. The epic text still promises it, though.
- **The epic contradicts the stories on `Zs` white space.** The epic still says white space outside ASCII is a refused omission until slice 5 (`epic/slice-01-harness.md:73-75`). `story:syntax-es5-and-strict` accepts `Zs` white space using the slice 1 `Zs` table (lines 44-50). This settles the epic's open item on early parts and adds no work the epic excludes. The epic text needs the same rewrite its summary line needs, which still says 82 cases and 164 runs (`epic/slice-01-harness.md:7`).
- **Out of my lane (design):** `story:object-model-and-realms:42` says `HOST-153` and `HOST-154` give their answers "in those slices" (10 and 11). The document marks `HOST-154` `[slice 14]`.
- **Not re-checked:** the moved-case counts, the 13 passed / 48 unsupported ESS scenarios, and the `expected.txt` line counts (280, 283), which rest on the foundation's wave 3. Those belong to the acceptance critic.

```findings
[]
```
