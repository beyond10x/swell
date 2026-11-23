---
format: aep.planning-md/3
id: review-result:slice-10-later-r2-a-acceptance
kind: review-result
status: active
title: 'Slice 10, later stories, set A, second reading, the acceptance critic: the eight stories of orders 4 to 6 as revised'
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
approve

All 15 of my first-reading findings hold as applied; I found nothing new in the changes the revision made across stories.

- **First findings:** 15 applied, 0 answered, 0 open.
- **Statics, `ErrorKind`:** `story:promise-statics-finally-and-aggregate-error` keeps `ALL` at seven entries and `[ErrorIntrinsics; 7]`. It adds `name`, `Intrinsics::error`, `global_object` and the requirement lines to the Writes line, and says what `new_error` makes (HOST-201 and HOST-202). The Writes line does not list `errors.rs` or `fuzz.rs`, because both read `ALL` only and stay unchanged.
- **ESS acceptance:** The four declarations each have a search that prints 0 on the base; I confirmed that on the tree. The planted-defect line is added to the task, and the floor and maximum are checked by `grep -c`. On the base, `passed < 14)` and `default 44}}` each match once.
- **Request queue:** `story:async-generators-and-their-request-queue` now has `next` tests at `frame_depth` and at `reentry_depth`. It depends on the statics story, which covers the 8 `Promise.all` cases. It appends a sweep root and states the unit reads check (b) on the base run. Its `admitted` handling matches the syntax story.
- **Syntax story:** `story:async-generator-syntax-and-early-errors` now gives the `admitted` chain the same way as the request-queue story. Check (b) states its base count (297), and the unit reads it on the base run.
- **`for await`:** `story:for-await-and-async-from-sync-iterator` drops `docs/design` from its search. The search now prints 4 lines on the tree here; `engine.md` is out of it, and the rest lie on the first wave's refusal. The story states the sweep adds no root and says why, adds `host_object.rs` for `WellKnownSymbol` to its Writes line, and states the base count for check (b).
- **`AsyncFunction` constructor:** `story:async-function-constructor` adds the nesting, `reentry_depth`, step-budget and memory-budget tests. It adds the sweep root list and has an awk search that prints 1 on the base.

Not re-run: the case counts in the lists (1,124 and 23) and the `passed N bound N` figures. There is no `runs.tsv` in the tree, so I only checked that the list lengths agree with the stories.

```findings
[]
```
