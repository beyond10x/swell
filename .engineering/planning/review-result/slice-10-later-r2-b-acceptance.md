---
format: aep.planning-md/3
id: review-result:slice-10-later-r2-b-acceptance
kind: review-result
status: active
title: 'Slice 10, later stories, set B, second reading, the acceptance critic: the five stories of orders 7 and 8 as revised'
relations:
- reviews: story:yield-star-in-async-generators
- reviews: story:async-generator-methods-in-object-literals
- reviews: story:async-generator-function-constructor
- reviews: story:array-from-async
- reviews: story:slice-10-conformance-close
revision: 1
---
approve

All 11 first findings are applied. No finding is open and I found no new one.

Applied, 11 of 11. I recomputed the figures from `.out/decompose-slice-10/base-copy.tsv`, not from the stories.
- **`story:yield-star-in-async-generators`, check (b):** the story now says it prints `0` on the property-escapes base too, and has the unit report the count over the join's base run.
- **`story:yield-star-in-async-generators`, part:** `part-yield-star-sync.cases.txt` (8 cases) and `test/staging/sm/generators` are added. The part is 995 cases and 1,936 runs, with `B` 246, `F` 595 and 1,190 failing lines, which matches my recount.
- **`story:yield-star-in-async-generators`, `reentry_depth`:** `yield_star_over_a_script_iterator_at_reentry_depth_throws_a_range_error_at_the_yield_star` and the chain test now cover it. The harness it names (`stack_agent`, `reenter`) exists in `crates/swell/tests/support/stack.rs`.
- **`story:async-generator-methods-in-object-literals`, check (b):** it has the same fix as the `yield*` story. The part still counts 736 cases, 1,365 runs and `B` 288.
- **`story:async-generator-methods-in-object-literals`, Bounds:** a Bounds line and `an_async_generator_method_ends_at_each_budget_and_at_an_interrupt` are added.
- **`story:async-generator-function-constructor`, tests:** the charge-and-interrupt test is added, with four more tests of the same form as the `Function` tests. Each `Function` test named in the story exists in `crates/swell/tests/dynamic_function.rs` (lines 148, 174, 221, 245).
- **`story:async-generator-function-constructor`, part:** `test/built-ins/Function` is in the part (815 cases, 1,457 runs, `B` 649 on my recount). The two cases moved onto its list (`built-ins/Function/prototype/toString/AsyncGenerator.js` and `built-ins/Object/seal/seal-asyncgeneratorfunction.js`) need no `yield*`.
- **`story:array-from-async`, edge:** the edge to `story:async-methods-in-object-literals` is added.
- **`story:array-from-async`, waiting half:** `a_termination_while_from_async_waits_keeps_it_and_a_later_checkpoint_continues_it` covers it.
- **`story:slice-10-conformance-close`, base numbers:** the base side of "same two numbers" now has an awk command over `.out/test262-base/runs.tsv`.
- **`story:slice-10-conformance-close`, paragraph check:** the paragraph is checked by an awk over the lines of `moves.tsv` whose `from` is 10. The fields are right and it prints `all 0` before the change, since `moves.tsv` holds none.

Answered, 0. Open, 0.

New checks on the revision:
- The `HOST-207` and `HOST-208` greps match `conformance/host/requirements.txt`: both are `pending slice 10` on `3648bf6`, and the slice 10 total there is 31.
- The check that `Array.fromAsync` adds no change to `limits.rs` or `surface.txt` holds as written.
- The 36 cases moved to the `yield*` list all match my scan for `yield*`.

**What you read:** 5 stories (`story:yield-star-in-async-generators`, `story:async-generator-methods-in-object-literals`, `story:async-generator-function-constructor`, `story:array-from-async`, `story:slice-10-conformance-close`), whole, with `aep plan artifact show`. I also read the revision's diff stat, `part.sh`, `check.sh`, the list files, `requirements.txt` and the harness and test names the stories cite.

**What I could not establish:**
- What check (b) of the `yield*` and methods stories prints on the join's base run, since that run does not exist yet.
- The exact off-by-one of the chain test at `frame_depth + 1`. The design text supports it, and the unit measures it.

```findings
[]
```
