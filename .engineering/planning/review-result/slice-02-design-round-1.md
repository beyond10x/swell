---
format: aep.planning-md/3
id: review-result:slice-02-design-round-1
kind: review-result
status: active
title: design critic, slice 2 stories, round 1
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
story:syntax-language-core — `parse_eval`, `parse_dynamic_function` and the direct-`eval` mark of `scope.rs` are half of the dynamic-code entry whose only caller is story:eval-and-function-constructor two waves later, whose `Writes:` names no `crates/syntax` file and whose `compile_eval` restates the eval-code declaration list that `parse_eval` returns, so the two entries, the mark and their tests move into story:eval-and-function-constructor with `crates/syntax/src/{lib,scope}.rs` and `crates/syntax/tests/sources.rs` on its `Writes:` — .engineering/planning/story/syntax-language-core.md:22, .engineering/planning/story/eval-and-function-constructor.md:37, AGENTS.md:285
story:eval-and-function-constructor — the body says it waits for the design author's HOST-117/141 retag and HOST-162 but does not say it also waits for the correction of section 3, which still says a parse past the nesting bound "through `eval` or `Function`" is a `RangeError` where this story builds a `SyntaxError` — .engineering/planning/story/eval-and-function-constructor.md:26, docs/design/engine.md:295
story:with-arguments-and-dynamic-names — the body records the `@@iterator` omission "as the design says" and names only the name-resolution rule as a design change it waits for, but section 12's omission rule covers only parts taken before their slice, so the body does not state that it waits for the design change that allows this omission — .engineering/planning/story/with-arguments-and-dynamic-names.md:45, docs/design/engine.md:816

Read: the epic and all 6 stories, the relations and the graph (no cycle; waves 3, 1, 1, 1; six acceptance reads traced to a producer in the set, all covered by an edge).

```findings
- file: .engineering/planning/story/syntax-language-core.md
  line: 22
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the parse entries of eval and Function and the direct-eval mark belong to story:eval-and-function-constructor, their only caller"
- file: .engineering/planning/story/eval-and-function-constructor.md
  line: 26
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the body does not say it waits for the correction of section 3 (RangeError to SyntaxError)"
- file: .engineering/planning/story/with-arguments-and-dynamic-names.md
  line: 45
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the body does not state that it waits for the design change that allows the @@iterator omission"
```
