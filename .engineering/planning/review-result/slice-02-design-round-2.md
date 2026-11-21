---
format: aep.planning-md/3
id: review-result:slice-02-design-round-2
kind: review-result
status: active
title: design critic, slice 2 stories, round 2
relations:
- reviews: story:test262-run-causes
- reviews: story:syntax-language-core
- reviews: story:number-boolean-and-global-functions-early
- reviews: story:with-arguments-and-dynamic-names
- reviews: story:eval-and-function-constructor
- reviews: story:slice-02-conformance-close
revision: 1
---
approve

All findings of round 1 are met: the parse entries and the direct-`eval` mark moved to story:eval-and-function-constructor with the syntax files on its `Writes:`; the section 3 correction and the omission rule are named as design changes the stories wait for. 13 edges walked inside the set, no cycle, waves 3, 1, 1, 1; 8 acceptance reads traced to a producer, each covered by an edge.

Notes outside the lane: the wave 1 test `a_refusal_inside_eval_script_fails_the_run` asserts a run line that wave 3 changes and is not listed among the tests whose expectation changes; story:test262-run-causes does not say whether a run `evalScript` marks counts in `test262-refused`; 18 cases of slice 18 call `$262.evalScript` and were not checked for a wrong-reason pass on `main`; the "Out of scope" of story:test262-run-causes still says the runner cannot see a refusal inside `eval`.

```findings
[]
```
