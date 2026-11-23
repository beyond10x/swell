---
format: aep.planning-md/3
id: review-result:slice-07-round-3-acceptance
kind: review-result
status: active
title: 'Slice 7, third reading, the acceptance critic: the three stories that had a blocker'
relations:
- reviews: story:generator-function-constructor
- reviews: story:yield-star-delegation
- reviews: story:slice-07-conformance-close
revision: 1
---
needs-revision

Round-2 findings on these three stories (all six closed):
- closed — story:generator-function-constructor, the bound of check (b): it now reads B1 + F1 − R1 and B + F − R, with F and F1 counted from the base run, and Context names the 2 cases that already pass — .engineering/planning/story/generator-function-constructor.md:68, :34
- closed — story:generator-function-constructor, full count: the acceptance now runs `swell-syntax` with `SWELL_FULL_COUNTS=1` and bounds each part at 2.5 CPU seconds at the full count — .engineering/planning/story/generator-function-constructor.md:68
- closed — story:yield-star-delegation, the bound of check (c): it now reads B + F − R and B1 + F1 − R1 (about 46 and 40 when the first story passes the 3 cases) — .engineering/planning/story/yield-star-delegation.md:90
- closed — story:yield-star-delegation, base of check (b): about 92 over `.out/test262-base/runs.tsv`, and `0` on `4971b9b` with the reason given — .engineering/planning/story/yield-star-delegation.md:90
- closed — story:slice-07-conformance-close, the second half of check (4): it now has a command, which prints each passing case of a later slice that failed at the start of slice 7 and has no line, and the printed cases must match the report's list — .engineering/planning/story/slice-07-conformance-close.md:63
- closed — story:slice-07-conformance-close, base of check (1): at least 80 over the base run, 2,825 on `4971b9b` — .engineering/planning/story/slice-07-conformance-close.md:63

The changed check (4) still shows what it must. It covers every case of a later slice that slice 7 made pass: each one that has no line in `close-into-7.tsv` is printed and has to be in the report's list. The opening `FILENAME == ARGV[1]` test also works when `close-into-7.tsv` is empty. Leaving out the cases that already passed at the start matches the close of slice 5. The problem is that only the acceptance has this filter, not the Change:

story:slice-07-conformance-close — Change still has the unit sort "each case that passes in every run" of a later slice into `close-into-7.tsv` or no line, with no filter on the start run, while check (4) says the about 1,288 cases that already passed at the start "are not this close's". So a unit that follows the Change writes rule lines for those cases and moves the counts of slices 4, 9, 18, 19 and others, and the acceptance passes either way. Add to the Change sentence "that failed in the start runs (`.out/test262-start/runs.tsv`) and" before "passes in every run", and the same limit to the rule sentence in Context (line 33) — .engineering/planning/story/slice-07-conformance-close.md:51

Read: 3 of 3 stories, the Acceptance, Context and Change of each (`aep plan artifact show`, the store files at `82790ef`). I made no scratch files.

Could not establish: section 12, rule 1, does not itself say that only cases failing at the start come into a slice; the design text reads as covering every passing case. Whether the design needs that sentence belongs to the design critic.

```findings
- file: .engineering/planning/story/slice-07-conformance-close.md
  line: 51
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "Change still has the unit sort each case of a later slice that passes in every run into close-into-7.tsv or no line, with no filter on the start run, while check (4) says the about 1,288 cases that already passed at the start are not this close's; a unit that follows the Change writes rule lines for those cases and moves the counts of slices 4, 9, 18, 19 and others, and the acceptance passes either way; add the limit 'that failed in the start runs' to the Change sentence and to the rule sentence in Context (line 33)"
```
