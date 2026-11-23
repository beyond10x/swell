---
format: aep.planning-md/3
id: review-result:slice-10-later-b-scope
kind: review-result
status: active
title: 'Slice 10, later stories, set B, the scope critic: the five stories of orders 6 and 7'
relations:
- reviews: story:yield-star-in-async-generators
- reviews: story:async-generator-methods-in-object-literals
- reviews: story:async-generator-function-constructor
- reviews: story:array-from-async
- reviews: story:slice-10-conformance-close
revision: 1
---
approve

Set B claims every promise of the epic that falls to it exactly once, and reaches nothing beyond the epic.

**What I read:** 1 epic, the 5 stories of set B and the front matter of the other 12 stories. I ran `aep plan artifact show` on each, plus `aep plan artifact graph`. I read `docs/plan/slice-10/` (the case lists, `close-later.tsv`, `close-cross.cases.txt`, `scan.tsv`, `later.tsv`) and the pinned test262 files they name. From the epic I extracted 22 promises (7 outcome bullets that split into 12, the runner's async handling, the generic/timeout jobs, the 23 requirements, and the 7 acceptance lines). I traced all 22 to a story. The ones that fall to set B:
- `Array.fromAsync` goes to `story:array-from-async`.
- `AsyncGeneratorFunction` goes to `story:async-generator-function-constructor`.
- Async generator methods go to `story:async-generator-methods-in-object-literals`.
- The async kind of `yield*` goes to `story:yield-star-in-async-generators`.
- Acceptances 1, 2, 4 and the counts go to `story:slice-10-conformance-close`.

**Coverage checks I ran:**
- The 13 case lists are disjoint: 3,668 distinct cases, no case on two lists.
- 3,668 plus `close-later.tsv` (138) plus `close-cross.cases.txt` (60) equals the 3,866 failing cases of `scan.tsv`. None of the three overlap.
- The `Array/fromAsync` directory is 93 cases: 73 on the story, 2 in the cross list and 18 in `close-later.tsv`. All 93 are accounted for.
- The 23 pending requirements are each placed in exactly one story: 8 in `job-queue-and-microtask-checkpoint`, 9 in `promise-constructor-then-and-jobs`, 2 in `async-functions-and-await`, 4 in `promise-rejection-tracker-and-promise-reads`.
- Every cases-list narrowing in set B names where the dropped cases go: the 58 `ag-cross` and 2 `fromAsync` cases go to the close's check 6, and the 18 later-slice cases go to `close-later.tsv`.
- The close story's edges reach every other story of the slice, directly or through another story.
- The close's rule for taking later-slice cases into slice 10 follows the precedent in `story:slice-07-conformance-close` and section 12 of `docs/design/engine.md`.
- The cases on the set B lists use no `Promise` static, `finally`, `AggregateError`, `class`, `Proxy` or `Reflect`, so no edge to the statics story is missing.

**What I could not establish:**
- Out of my lane, so not set against the verdict: `yield-star-in-async-generators` says "33 cases of `return` and `throw`", but the 33 `AsyncFromSyncIteratorPrototype` cases are 10 `next`, 8 `return` and 15 `throw`. All 33 do use `yield*`. This is wording only.
- I did not check the instruction-level `Writes:` lines of the set B stories beyond the `ObjectKind` and `KIND_SITES` sites of `array-from-async`. Those match the code (16 sites today, 20 and 21 after the stories). The rest is for the other critics' lanes.
- `AGENTS.md` and `README.md` (epic acceptance 3) belong to the coordinator, and the close story names them as such, so I count them as claimed.

```findings
[]
```
