---
format: aep.planning-md/3
id: review-result:slice-10-later-r2-a-parallel-safety
kind: review-result
status: active
title: 'Slice 10, later stories, set A, second reading, the parallel-safety critic: the eight stories of orders 4 to 6 as revised'
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

No finding is open or new.

Of my first 11 findings, 10 are applied, 1 is answered, and 0 are open.

- **Applied (10):** the checks below were made against the code and the case lists, not from the stories' claims.
  - **Tracker edge:** the tracker story now depends on `story:async-functions-and-await`. Its counts hold: 31 `pending slice 10` lines today, 14 after the promise story, 12 after async functions.
  - **`builtins_symbol.rs` and `builtins_object_enumeration.rs`:** both are now in the Writes line, scope and acceptance of the `for await` story.
  - **`iterator_complete`:** the `for await` story says it makes the same word `pub(crate)` on the same line as `story:yield-star-delegation`. That is an identical edit, so it merges cleanly.
  - **Sharing and ordering:** the same-hunk sharing of `lib.rs` and `parser/mod.rs` is named in both stories, as is the sharing of `instantiate_function` and `compiler/src/expression.rs`.
  - **Symbol sweep roots:** `story:async-function-constructor` owns the list of roots in `builtins_symbol_values.rs`. The request-queue story appends a line and now depends on that story.
  - **Request-queue edges:** it now has edges to `story:promise-statics-finally-and-aggregate-error`, `story:yield-star-delegation` and `story:async-function-constructor`. That orders the shared `builtin_function.rs`, `intrinsics.rs` and `builtins/mod.rs`, and the `Promise.all` cases.
  - **Case lists:** the 36 `yield*` cases are in `story-ag-yield-star.cases.txt`. `story-ag-functions.cases.txt` has 1,124 cases and none of them contains `yield*`. The lists are pairwise disjoint.
  - **`waves.md`:** it now shows eight orders, and `aep plan artifact graph` shows the new edges.
- **Answered (1):** the `errors.rs` finding. The statics story no longer grows `ErrorKind::ALL`: it keeps 7 entries, makes `AggregateError` by a step of its own, and adds `#[non_exhaustive]`. So `ERRORS` and `every_engine_error_records_its_kind` stay valid. No test or crate outside `swell` matches over `ErrorKind`.

**New defects from the revision:** none. The new edges leave no unnamed pair. The only remaining pairs without a path of edges are named in both bodies, in hunks apart:
- `story:async-methods-in-object-literals` and the request-queue story;
- `story:promise-statics-finally-and-aggregate-error` and the order-4 stories;
- the open stories of slice 7.

**What I read:** `aep plan artifact graph`, `git show --stat 4f609e0`, the committed text of my eight stories with the Writes and Shared paragraphs and acceptance lines (the diffs of the `for await`, ess, async-methods and async-generator-syntax stories), `waves.md`, `later.tsv`, and the case lists under `docs/plan/slice-10/`. In the code I checked `ErrorKind`, `WellKnownSymbol` and `functions_of_the_realm`.

**Not established:** surfaces 8 cited, 0 inferred, 0 unplaced. Not in my lane: the story texts for the run-loop rule and `reentry_depth`, which I flagged last time and which no longer bears on this verdict.

```findings
[]
```
