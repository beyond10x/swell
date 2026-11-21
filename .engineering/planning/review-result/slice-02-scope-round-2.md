---
format: aep.planning-md/3
id: review-result:slice-02-scope-round-2
kind: review-result
status: active
title: scope critic, slice 2 stories, round 2
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
epic:slice-02-language-core — The epic promises "each such run is listed in the wave record that removes it", but no story claims that list. story:eval-and-function-constructor is the natural owner: its acceptance lets a run that passed on the base fail with `counted refusal:` and neither prints those runs nor hands them over. Its acceptance should add an `awk` over `.out/test262-base/runs.tsv` and `.out/test262/runs.tsv` that prints them, and its body should say the unit gives that output to the coordinator for the wave 3 record — .engineering/planning/epic/slice-02-language-core.md:100, .engineering/planning/story/eval-and-function-constructor.md:215

All ten findings of round 1 are met. 34 promises of the epic, 33 traced. Note outside the lane: `test262-moves-check` takes its base from the design on `main` and reads `runs.tsv` evidence that stops holding once a later slice passes a moved case, so as written it can run once and cannot stay in `check`.

```findings
- file: .engineering/planning/epic/slice-02-language-core.md
  line: 100
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "no story claims the list of runs that lose a wrong-reason pass; the eval story prints it and hands it to the coordinator"
```
