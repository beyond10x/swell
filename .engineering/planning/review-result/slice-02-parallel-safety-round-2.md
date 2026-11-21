---
format: aep.planning-md/3
id: review-result:slice-02-parallel-safety-round-2
kind: review-result
status: active
title: parallel-safety critic, slice 2 stories, round 2
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
story:eval-and-function-constructor — its `Writes:` line and recorded scope leave out `crates/syntax/src/ast.rs`, but the body puts the direct-`eval` mark per function and for the script, exported on the tree for the compiler, and the place where scope analysis stores per-function and script results is `ast::Scope` (`crates/syntax/src/ast.rs:334`), so the file must be added to `Writes:` and to the scope (surface inferred from the code) — .engineering/planning/story/eval-and-function-constructor.md:174, .engineering/planning/story/eval-and-function-constructor.md:203, crates/syntax/src/ast.rs:334

All four findings of round 1 are met. Wave 1 shares no file once `expected.txt` is set aside, checked against the code. Notes outside the lane: `Writes:` of story:test262-run-causes says seven fixture files where its Change lists eight; `test262-moves-check` reads `.cache/inventory/meta.tsv` and `slices.awk`, which no task produces.

```findings
- file: .engineering/planning/story/eval-and-function-constructor.md
  line: 203
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "Writes and scope leave out crates/syntax/src/ast.rs, where ast::Scope holds what the direct-eval mark is exported on"
```
