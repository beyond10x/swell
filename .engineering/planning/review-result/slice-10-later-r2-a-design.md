---
format: aep.planning-md/3
id: review-result:slice-10-later-r2-a-design
kind: review-result
status: active
title: 'Slice 10, later stories, set A, second reading, the design critic: the eight stories of orders 4 to 6 as revised'
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

All 13 findings from my first reading are applied. I found nothing new in my role. I read the revision in `4f609e0` and checked each fix against the committed text, the code and the case lists.

13 of 13 first findings applied, 0 answered, 0 open.

How I checked each:
- **Statics, `ErrorKind`:** `ALL` keeps its 7 entries and `ErrorKind` is `#[non_exhaustive]`. So `errors.rs` and `fuzz.rs` stand unchanged, and the story says why. The acceptance now expects `host-surface-check` to name `ErrorKind::AggregateError` and `ErrorKind`, and not `ALL`. No test file outside swell's own `errors.rs` and `fuzz.rs` matches over `ErrorKind`.
- **Statics, depth tests:** three tests were added: `promise_all_at_reentry_depth_rejects_with_a_range_error`, `promise_all_at_frame_depth_rejects_with_a_range_error` and `promise_all_over_an_iterator_without_end_ends_at_a_budget`.
- **`for await` story:**
  - The two symbol tests are on the Writes line.
  - The run-loop paragraph is present.
  - The tests `an_async_from_sync_iterator_is_charged_until_it_is_freed`, the `reentry_depth` and `frame_depth` tests, and an endless-iterator test were added.
- **Request-queue story:**
  - The run-loop paragraph is present.
  - The `reentry_depth` test and the endless `next` loop test were added.
  - `interp/async_function.rs` and `object/async_function.rs` are on the Writes line.
  - Edges to the statics, `yield-star-delegation` and `async-function-constructor` exist.
  - The list is 1,124. Only 3 case files in it still contain `yield*`, and all three are only in the description comment.
- **Tracker story:** it depends on `async-functions-and-await`, and its acceptance is now stated against the count the unit reads on its base. The first-reading sentence about 4 versus 6 pending requirements is gone. The pending counts are consistent: 12 after the first wave, then 8 after the tracker.
- **ESS story:** the quoted sentence is in `job-queue-and-microtask-checkpoint.md` at line 76.
- **Graph:** `aep plan artifact validate` is valid on 224 artifacts. The new edges form no cycle, and orders 4 to 6 in `waves.md` are consistent with them.

```findings
[]
```

I had not yet written the reply to the earlier task, so this answers both. The first-reading reply stays as I gave it above.
