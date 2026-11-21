---
format: aep.planning-md/3
id: review-result:slice-02-acceptance-round-1
kind: review-result
status: active
title: acceptance critic, slice 2 stories, round 1
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
story:slice-02-conformance-close — the acceptance is met by any rule lines that move every failing case out of slice 2, whatever the cause: a case that fails on a slice 2 part, a move to an excluded slice, even `test262-slice 02 0 0 0 0 0 0 0 0`; so it does not show the outcome the story names (only cases that need a later slice move, each with its reason). It should name a per-case check, such as the scan output naming a later-slice construct, builtin or refusal for every moved case, with the group counts in the paragraph summing to the moved total, and require that slices 1 to 19 still sum to 44,931 cases and 85,794 runs — .engineering/planning/story/slice-02-conformance-close.md:33

Read: all 6 stories and the epic. Checked and found correct: every count and command in the six acceptances (requirements 162 / 78 / 84; slice lines `02 5266 9215`, `03 7580 14992`, `06 3719 7166`; the bounds; `expected.txt` at 284 and then 286 lines; the run-line fields). Not established: the pass-floor comparison takes its base from `git show HEAD:conformance/test262/expected.txt`, which compares the file with itself if run after a commit. Outside the lane: the two design-change dependencies the bodies do not name; the acceptance of `story:eval-and-function-constructor` does not check that item 3 of "What each slice builds" no longer gives `Function` to slice 3, and has no test that reaches `reentry_depth` or the minimum stack through `Function(...)`.

```findings
- file: .engineering/planning/story/slice-02-conformance-close.md
  line: 33
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the acceptance is met by rule lines that move every failing case whatever the cause; it needs a per-case check that each moved case names a later-slice construct, builtin or refusal, and the sum over slices 1 to 19"
```
