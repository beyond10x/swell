---
format: aep.planning-md/3
id: review-result:slice-02-scope-round-1
kind: review-result
status: active
title: scope critic, slice 2 stories, round 1
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
epic:slice-02-language-core — The epic promises that every run of slice 2 passes, but no story claims `$262.evalScript`. Slice 1 put it off to slice 2 (`.engineering/planning/story/test262-run-path.md:90`, `crates/test262/src/host.rs:4`), and 13 cases under `test/language/global-code/` call it; `script-decl-func-dups.js` needs nothing from a later slice, so `story:slice-02-conformance-close` cannot move it and has no story to send it back to. `story:test262-run-causes`, which already writes `crates/test262`, is the natural owner. — .engineering/planning/epic/slice-02-language-core.md:81
story:syntax-language-core — The story keeps method definitions and shorthand properties refused as slice 6, but no line of `docs/design/engine.md` section 12 puts them in slice 6. About 45 cases that `dir test/language/ 2` counts in slice 2 (34 of them under `expressions/object`) would move on a slice number a slice-1 unit chose, which `docs/plan/waves.md:98` says must be checked against the design. The story should either take them or cite the design line that puts them in slice 6. — .engineering/planning/story/syntax-language-core.md:18

Read: the epic and all 6 stories; 23 promises of the epic (10 outcome items, 2 open items, 6 constraints, 5 acceptance items), all mapped to a story or to slice 1 except the `$262.evalScript` cases. Not established, outside the lane: the regression check compares directory lines and the total, not the `test262-slice` lines; `story:number-boolean-and-global-functions-early` leaves out `Number.prototype.toString`, so a Number object converted with the string hint falls back to `Object.prototype.toString`.

```findings
- file: .engineering/planning/epic/slice-02-language-core.md
  line: 81
  category: scope
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "no story claims $262.evalScript, which 13 cases under test/language/global-code/ call"
- file: .engineering/planning/story/syntax-language-core.md
  line: 18
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "method definitions and shorthand properties stay refused as slice 6 with no design line that puts them there; about 45 cases of slice 2 would move"
```
