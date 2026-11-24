---
format: aep.planning-md/3
id: review-result:slice-10-later-r2-a-scope
kind: review-result
status: active
title: 'Slice 10, later stories, set A, second reading, the scope critic: the eight stories of orders 4 to 6 as revised'
relations:
- reviews: story:promise-statics-finally-and-aggregate-error
- reviews: story:promise-rejection-tracker-and-promise-reads
- reviews: story:async-methods-in-object-literals
- reviews: story:async-function-constructor
- reviews: story:for-await-and-async-from-sync-iterator
- reviews: story:async-generator-syntax-and-early-errors
- reviews: story:async-generators-and-their-request-queue
- reviews: story:ess-target-checkpoint-and-promise-hooks
revision: 1
---
needs-revision

- epic:slice-10-promises-and-async — warning — the epic was not revised with the stories and now contradicts them in two places:
  - Its Outcome still says "The 23 requirements of the host interface pending for slice 10 each get a test" (line 58), but `conformance/host/requirements.txt` now holds 31 `pending slice 10` lines (`grep -c 'pending slice 10$'` prints 31). `HOST-201` to `HOST-208` are owned by the statics, `for await`, request-queue and `array-from-async` stories, so those four claim eight requirements the parent does not promise.
  - Its "Stories" section (lines 152 to 168) still lists seven orders with the statics story at order 4 beside the tracker. `docs/plan/waves.md` has eight orders: statics at 5, the request-queue story alone at 6, the rest of the later stories at 7, the close at 8.
  - What would settle it: reword the epic to "31 requirements (`HOST-46` … `HOST-208`)" and renumber the order list to match `waves.md`.

First findings: 9 applied, 0 answered, 0 open. I checked each against the text, the case lists and the code.
- The two cases that call `.constructor` are on `story-ag-constructor.cases.txt` (lines 21 and 22), the list has 23, and the request-queue list has 1,124. All lists are disjoint, with no duplicates in `cat story-*.cases.txt | sort | uniq -d`.
- The request-queue story now owns `constructor` of `%AsyncGeneratorPrototype%`.
- It has the edge to the statics story, which covers its eight `Promise.all` cases.
- The `Writes:` lines now hold `errors.rs` and `fuzz.rs` (statics) and `fuzz.rs` plus the `reads` helper (tracker).
- Both instruction-adding stories carry a Run loop sentence.
- The `reentry_depth` and endless-queue tests exist: `an_async_generator_resuming_an_async_generator_reaches_reentry_depth`, `a_script_that_queues_requests_without_end_ends_at_a_budget`, `promise_all_at_reentry_depth_rejects_with_a_range_error`, `promise_all_over_an_iterator_without_end_ends_at_a_budget`.

New defects from the revision: none in the stories. The new edges (the statics story on the tracker, the request-queue story on the statics, `yield*` of slice 7 and the AsyncFunction constructor, the tracker on async functions) are in the frontmatter and match `waves.md`. I re-read the epic's coverage: all 19 promises are still claimed once.

Not established: the other three critics' lanes (not read), and set B's moved 36 `yield*` cases, which are not my set.

```findings
- file: .engineering/planning/epic/slice-10-promises-and-async.md
  line: 58
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the epic still promises 23 pending host requirements and lists seven orders (lines 58 and 152-168), but conformance/host/requirements.txt holds 31 pending for slice 10 (HOST-201 to HOST-208 added and owned by four stories) and docs/plan/waves.md has eight orders; reword the outcome and renumber the order list"
```
