---
format: aep.planning-md/3
id: review-result:slice-02-acceptance-round-2
kind: review-result
status: active
title: acceptance critic, slice 2 stories, round 2
relations:
- reviews: story:test262-run-causes
- reviews: story:syntax-language-core
- reviews: story:number-boolean-and-global-functions-early
- reviews: story:with-arguments-and-dynamic-names
- reviews: story:eval-and-function-constructor
- reviews: story:slice-02-conformance-close
revision: 1
---
needs-revision
story:eval-and-function-constructor — the acceptance expects the host-requirements line to end `50 UNMAPPED marker(s)`, but on the base this story builds on the task counts 51 markers (merge `ae07c5c` replaced one marker with two). The story writes nothing under `spec/`, so the line should read the number the same task prints on the base of the wave instead of the fixed 50 — .engineering/planning/story/eval-and-function-constructor.md:215

All nine findings of round 1 are fixed. Checked and found correct: requirements 161 to 164 with 80 test and 84 pending, the slice lines `02 5267 9217` and `03 7581 14994`, the sums 53,801 / 103,360 and 44,931 / 85,794, the line counts of `expected.txt`.

```findings
- file: .engineering/planning/story/eval-and-function-constructor.md
  line: 215
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the acceptance fixes 50 UNMAPPED markers where the base counts 51; it should read the count of the wave base"
```
