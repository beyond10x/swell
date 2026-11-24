---
format: aep.planning-md/3
id: review-result:slice-04-round-2-b-design
kind: review-result
status: active
title: 'Slice 4, round 2, the design critic: the twelve later stories'
relations:
- reviews: story:swell-num-date-text
- reviews: story:math-sum-precise
- reviews: story:number-functions
- reviews: story:swell-num-math-exp-and-log
- reviews: story:swell-num-math-trigonometry
- reviews: story:date-setters
- reviews: story:math-functions
- reviews: story:date-text-formats
- reviews: story:exponentiation-operator
- reviews: story:math-random
- reviews: story:slice-04-conformance-close
- reviews: story:spec-scenarios-for-the-numbers-and-dates-requirements
revision: 1
---
needs-revision — panel B

- `swell-num-date-text`: approve
- `math-sum-precise`: needs-revision
- `number-functions`: approve
- `swell-num-math-exp-and-log`: approve
- `swell-num-math-trigonometry`: approve
- `date-setters`: approve
- `math-functions`: approve
- `date-text-formats`: approve
- `exponentiation-operator`: approve
- `math-random`: approve
- `slice-04-conformance-close`: approve
- `spec-scenarios-for-the-numbers-and-dates-requirements`: approve

**Findings**

1. `math-sum-precise.md:59` (blocker). The acceptance runs `slot task libm-check`, which `swell-num-math-double-double.md:56` creates, and no `depends_on` edge runs to it. `waves` puts both stories in wave 2, so the check fails with "Task does not exist" until the double-double story merges. Add `depends_on: story:swell-num-math-double-double`, or drop `libm-check` from this acceptance and keep its own grep for the banned methods. `math-random` and `exponentiation-operator` also run `libm-check`, but both reach the double-double story through `math-functions`.

2. `math-sum-precise.md:47` and `:55` (warning). The stale-text grep and the `builtins_math::` run read `crates/swell/tests/builtins_math.rs`, but Writes does not list it. `math-constants-and-exact-functions.md:47` and `math-functions.md:54` both assert that `Math` has exactly the own keys the table gives. Say in all three that this test reads the table and not a literal list, because `sumPrecise` (wave 2) and `random` (wave 6) add entries with no edge to the stories that own the test.

3. `math-functions.md:12`, `:32`, `:36` (warning). The edge to `number-functions` rests on "2 cases of isNaN or isFinite". The scan (`docs/plan/slice-04/scan.tsv`) names `harness/nans.js` and `language/expressions/typeof/built-in-functions.js`. The first reads `Number.NaN`, which exists on the base. The second only reads `typeof parseInt` beside `Math.exp`. The `built-ins/isNaN` and `built-ins/isFinite` cases read the global functions that already exist. That one edge puts `math-functions`, `math-random`, `exponentiation-operator` and `spec-scenarios` behind `number-functions`, and so behind the clock story. Name the real cases. Then either keep the edge or move those two cases to the close and drop it.

4. `math-functions.md:7`, `:32`, `:53` (warning). Title, summary and two sentences say 21 functions, but the list and `:51` give 22. Make them agree.

5. `math-functions.md:51` (warning). The `Pow` arm "calls `swell_num::math::<name>`", but no `pow` exists. `swell-num-math-exp-and-log.md:38` exports `exponentiate`, and decision 6 ("`**` and `Math.pow`", `engine.md:2216`) says one function. Name `exponentiate` for `Math.pow` in the story.

6. `math-random.md:52` (warning). "Builds the 41-byte message" is wrong. Decision 6 (`engine.md:2119-2120`) gives `random_seed[16..32]` (16 bytes), the index (8) and the byte i (1), which is 25 bytes.

7. `swell-num-math-exp-and-log.md:62` against `swell-num-math-trigonometry.md:62` (warning). The first expects `math-data-check: 1 file(s)` and the second "one more than before". The two stories have no edge, so if trigonometry merges first the first check prints 2. Make the first relative.

8. `swell-num-math-exp-and-log.md:32` and `swell-num-math-trigonometry.md:32` (warning). One data file per story differs from decision 6 (`engine.md:1989-1992`: one `reference.txt`, one test `math.rs`). Splitting by owner is a sound answer to open point 4. But, unlike `number-functions.md:64`, neither story's Writes names the design edit or who makes it. Name it and its owner in both.

9. `date-text-formats.md:58` and `:66` (warning). The acceptance expects `grep -c "pending slice 4$"` to print 5 (6 before) and `4:5` after `pending by slice:`. Wave order puts `math-functions` (HOST-135) first, so the count is 5 before and 4 after. The story has no edge to the stories that move other lines. Make the check relative ("one less than before"), as `math-functions`, `math-random` and `number-functions` do.

10. `swell-num-date-text.md:48` (warning). `story:swell-num-math` no longer exists after the three-way split. Replace it with the `swell-num-math-*` stories that add a `pub mod` line.

**Open points (panel B)**
- Point 1: `math-random` handles both branches. HOST-147 names a `Map` (`host-interface.md:366`).
- Point 2: `date-text-formats` agrees with decision 6 (`engine.md:2348-2350`: NaN "unless a later decision"). The sentence must be written first.
- Point 3: the `parseInt` method is a precondition, and `number-functions` lists the design edit in its Writes.
- Point 4: see finding 8.
- Point 6: the order matches `1,2,3,6,5,7,10,4`. The eight edges out of `slice-10-conformance-close` do not return to slice 4, so there is no cycle.
- Date stories: they match decision 17 (61 hook calls at most; `toString` 2, `Date()` 3, `parse` 59).
- Math stories: they match decision 6 (groups and ulp bounds; `Math.random` words and index; `sumPrecise` 544 bytes and charges). `Nat` stays in `number-format`, and `Math.round` stays in panel A.

**Not in my lane:** the shared `crates/swell/src/builtins/math.rs` between `math-sum-precise`, `math-functions` and `math-random`. `math-sum-precise` has no edge to the other two, and the join "takes all three" (parallel-safety).

**What I read:** 12 panel B stories whole, plus the design text (`engine.md` section 6, `host-interface.md` decision 17), the brief, and parts of panel A (`swell-num-math-double-double`, `date-object-constructor-now-utc-and-getters`, `date-object-clock-and-time-value`). I also checked the test262 sources of the two cases in finding 3.

**Edges and reads traced:** I walked 44 `depends_on` edges between stories plus the 8 out of `slice-10-conformance-close`, so I did walk outside the set. I found no cycle and no serialising chain. The longest chain is six deep: `math-constants-and-exact-functions`, `date-object-clock-and-time-value`, `number-functions`, `math-functions`, `math-random`, close. I traced about 18 acceptance reads to a producer in the set, and 15 have an edge. The three without are findings 1, 2 and 9.

**Could not establish:** `aep plan artifact validate` printed `valid` with 3 warnings (active stories without a scope), none on these stories. I did not check the case counts against a run of the scan.

```findings
- file: .engineering/planning/story/math-sum-precise.md
  line: 59
  category: design
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the acceptance runs slot task libm-check, which story:swell-num-math-double-double creates, and no depends_on edge runs to it, so in wave 2 the check cannot pass until that story lands"
- file: .engineering/planning/story/math-sum-precise.md
  line: 55
  category: design
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "builtins_math.rs is read by the stale-text grep and the builtins_math:: run but is not in Writes, and the exact-own-keys tests of math-constants-and-exact-functions and math-functions can break on sumPrecise or random landing first; say the test reads the table"
- file: .engineering/planning/story/math-functions.md
  line: 36
  category: design
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the edge to number-functions is described as 2 isNaN or isFinite cases but the scan names harness/nans.js (Number.NaN, on the base) and typeof/built-in-functions.js (typeof parseInt), so name the real cases and keep the edge or move those two cases to the close"
- file: .engineering/planning/story/math-functions.md
  line: 7
  category: design
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "title, summary and context say 21 functions while the list and the Change section give 22; make them agree"
- file: .engineering/planning/story/math-functions.md
  line: 51
  category: design
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the Pow arm calls swell_num::math::<name>, which is no function; name swell_num::math::exponentiate as exp-and-log exports it and decision 6 says"
- file: .engineering/planning/story/math-random.md
  line: 52
  category: design
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the SipHash message is 25 bytes by decision 6 (16 + 8 + 1), not the 41 bytes the story says"
- file: .engineering/planning/story/swell-num-math-exp-and-log.md
  line: 62
  category: design
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the acceptance expects math-data-check to print 1 file(s), which is wrong if the trigonometry story, with no edge between them, merges first; say one more than before"
- file: .engineering/planning/story/swell-num-math-exp-and-log.md
  line: 32
  category: design
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the one-file-per-story reference data departs from decision 6 (one reference.txt, tests/math.rs) and no Writes entry of this story or of the trigonometry story names the design edit or who makes it, unlike number-functions"
- file: .engineering/planning/story/date-text-formats.md
  line: 66
  category: design
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the absolute pending counts (5, 6 before, 4:5) ignore HOST-135 moved by math-functions in an earlier wave and no edge orders it; state the check as one less than before"
- file: .engineering/planning/story/swell-num-date-text.md
  line: 48
  category: design
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "story:swell-num-math is named as a sharer of lib.rs but no such story exists after the three-way split; name the swell-num-math-* stories"
```
