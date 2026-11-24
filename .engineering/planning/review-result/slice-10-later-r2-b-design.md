---
format: aep.planning-md/3
id: review-result:slice-10-later-r2-b-design
kind: review-result
status: active
title: 'Slice 10, later stories, set B, second reading, the design critic: the five stories of orders 7 and 8 as revised'
relations:
- reviews: story:yield-star-in-async-generators
- reviews: story:async-generator-methods-in-object-literals
- reviews: story:async-generator-function-constructor
- reviews: story:array-from-async
- reviews: story:slice-10-conformance-close
revision: 1
---
needs-revision

story:yield-star-in-async-generators — warning: the chain tests state frame boundaries that the code does not give.
- `a_chain_..._longer_than_frame_depth_rejects_the_outermost_request_with_a_range_error` says a chain of `frame_depth` + 1 levels is rejected and "with one level fewer it is fulfilled".
- `frame_depth` counts "a script's own frame included", and a push is refused when `depth >= frame_depth`. With the default 100 and the script's frame, level 100 already fails to push. A chain of exactly `frame_depth` levels is therefore rejected, and the largest fulfilled chain is 99.
- `a_next_through_101_levels_spends_400_to_2000_steps_more_than_through_one` names 101 levels but does not say that the test raises `frame_depth`. The default is 100 (`crates/swell/src/host/hooks.rs:394`), and 101 levels would be rejected there.
- The story should state the boundary as the script frame plus n levels against `frame_depth`, and name the limits the 101-level test runs under.
- Citations: `crates/swell/src/interp/budget.rs:14` and `crates/swell/src/interp/budget.rs:44`; `.engineering/planning/story/yield-star-in-async-generators.md:75`.

First findings: 5 applied, 0 answered, 0 open.
- **Blocker:** applied. The story now cites `engine.md` item 6 of "Async generators on suspended frames" (line 1352). Its two chain tests replace the old one: one at 50 levels, between `reentry_depth` and `frame_depth`, and one at `frame_depth` + 1 where the outermost request is rejected with a `RangeError`. It adds a `reentry_depth` test for a script iterator at the `yield*`.
- **Design text against the standard:** I read `engine.md` item 6 against the draft's YieldExpression `yield*` steps in `.cache/ecma262/.../ecma262.txt`. The inner `Call` of `next`, the `Await` of its result, `AsyncGeneratorYield` with no second `await`, and the throw and return branches all match. So does the claim that an inner async generator's `next` resumes it at once through AsyncGeneratorResume.
- **`with_target`:** applied (story line 69, and Writes).
- **Run-loop rule:** applied (story line 73).
- **`array-from-async` depth test:** applied. `from_async_at_reentry_depth_rejects_with_a_range_error` and `from_async_at_frame_depth_rejects_w…` are added, and the story states the rejected-promise outcome.
- **`async-generator-function-constructor` search:** applied. I ran its awk over `crates/syntax/src/lib.rs` and it prints `1` on today's code, so the wrapped sentence is found.

What the revision changed:
- **Edges:**
  - `aep plan artifact validate` says valid, and I found no cycle.
  - The new edges are statics → tracker, request-queue → statics, `yield-star-delegation` and the `AsyncFunction` constructor, tracker → async functions, and `array-from-async` → async methods.
  - `docs/plan/waves.md` has eight orders that match them.
- **List moves:** 36 cases moved into the `yield*` list. All 36 are negative parse cases. 2 moved into the constructor list.
- **Design text for built-in async functions:** I read "A built-in async function" and "The Async-from-Sync Iterator". They agree with `array-from-async`'s kind site and with the `KIND_SITES` counts of 19, 20 and 21.

```findings
- file: .engineering/planning/story/yield-star-in-async-generators.md
  line: 75
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the chain tests state boundaries the code does not give: frame_depth counts the script's own frame and refuses a push at depth >= frame_depth, so with the default 100 a chain of frame_depth levels is already rejected and the largest fulfilled chain is 99, not frame_depth, and the 101-level step test does not say it raises frame_depth above the default 100; the story states the boundary as the script frame plus n levels and names the limits of the 101-level test"
```
