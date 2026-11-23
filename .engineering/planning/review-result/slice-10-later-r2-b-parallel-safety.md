---
format: aep.planning-md/3
id: review-result:slice-10-later-r2-b-parallel-safety
kind: review-result
status: active
title: 'Slice 10, later stories, set B, second reading, the parallel-safety critic: the five stories of orders 7 and 8 as revised'
relations:
- reviews: story:yield-star-in-async-generators
- reviews: story:async-generator-methods-in-object-literals
- reviews: story:async-generator-function-constructor
- reviews: story:array-from-async
- reviews: story:slice-10-conformance-close
revision: 1
---
needs-revision

yield-star-in-async-generators — warning — Its Writes line says `story:async-function-constructor` is one of the "earlier stories of the slice that no path of edges orders against this one". That is no longer true. This story depends on `story:async-generators-and-their-request-queue`, and that story now depends on `story:async-function-constructor`, so a path exists. Move `story:async-function-constructor` to the "ordered by a path of edges" group, as `async-generator-function-constructor` and `array-from-async` do for the statics story. The hunks stay apart. `story:async-methods-in-object-literals` stays in the unordered group, because no path joins it to this story. — .engineering/planning/story/yield-star-in-async-generators.md:65

First findings: 5 applied, 0 answered, 0 open (one new warning above).

I checked each one against the revised stories and the tree at `4f609e0`.
- `array-from-async` list (`async next()` case): applied. `depends_on story:async-methods-in-object-literals` is now in its relations, and the case is still on `docs/plan/slice-10/story-from-async.cases.txt`.
- `array-from-async` sharing `builtin_function.rs` and `builtins_symbol_values.rs` with the statics story and `async-function-constructor`: applied. Its Writes line names both stories, the hunks and the order through `async-generators-and-their-request-queue`.
- `async-generator-function-constructor` sharing `builtin_function.rs` and `intrinsics.rs` with the statics story: applied. It names the statics story and `async-function-constructor`, with the hunks. The path through `async-generators-and-their-request-queue` is real: that story now depends on the statics story and on `async-function-constructor`.
- `yield-star-in-async-generators` sharing files with `async-methods-in-object-literals` and `async-function-constructor`: applied. The Writes line now names both stories and the hunks (the new warning is about one sentence of this text).
- `slice-10-conformance-close` missing `conformance/test262/expected.digest`: applied. It is in the scope and the Writes line, and in the file slice 7's close shares.

New checks from the revision, all clear:
- **`requirements.txt` and `status.json`:**
  - The owners write different lines of `conformance/host/requirements.txt` (and `website/data/status.json`, which is generated). Lines 201 to 208 exist as `pending slice 10`.
  - The statics story owns lines 201 and 202, `for-await` line 203, the request-queue story lines 204 to 206, and `array-from-async` lines 207 and 208.
  - Edges order all of them. The one pair with no edge between them, the statics story and `for-await`, names the sharing in the statics Writes line.
- **`object/async_builtin.rs`:** `array-from-async` is the only story that creates it.
- **No dependency cycle:** the new edges (statics to tracker, request-queue to statics, `yield-star-delegation` and `async-function-constructor`, tracker to async functions, `array-from-async` to async methods) form none.
- **`yield-star-in-async-generators`, `async-generator-methods-in-object-literals`, `async-generator-function-constructor`, `array-from-async`, `slice-10-conformance-close`:** `aep plan artifact waves` prints no collision between these and a story of slice 10. Its collisions for these stories are with the implemented slice 6 story `array-destructuring-and-spread`, which predate this set.
- **Not checked:** the content of the new list `docs/plan/slice-10/part-yield-star-sync.cases.txt`. That belongs to the acceptance critic.

```findings
- file: .engineering/planning/story/yield-star-in-async-generators.md
  line: 65
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the Writes line lists story:async-function-constructor among earlier stories that no path of edges orders against this one, but this story depends on story:async-generators-and-their-request-queue, which now depends on story:async-function-constructor; move it to the ordered group (hunks stay apart)"
```
