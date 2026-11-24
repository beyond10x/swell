---
format: aep.planning-md/3
id: review-result:slice-04-round-3-design
kind: review-result
status: active
title: 'Slice 4, round 3, the design critic: the 17 stories'
relations:
- reviews: story:math-constants-and-exact-functions
- reviews: story:swell-num-calendar-and-local-time
- reviews: story:date-object-clock-and-time-value
- reviews: story:date-object-constructor-now-utc-and-getters
- reviews: story:number-format
- reviews: story:swell-num-math-double-double
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
revision: 1
---
1. **approve** (3 warnings and 8 notes in the findings block hold none of the 17 stories).

2. Per story:
- `math-constants-and-exact-functions`: approve
- `swell-num-calendar-and-local-time`: approve
- `date-object-clock-and-time-value`: approve
- `date-object-constructor-now-utc-and-getters`: approve
- `number-format`: approve
- `swell-num-math-double-double`: approve
- `swell-num-date-text`: approve
- `math-sum-precise`: approve
- `number-functions`: approve
- `swell-num-math-exp-and-log`: approve
- `swell-num-math-trigonometry`: approve
- `date-setters`: approve
- `math-functions`: approve
- `date-text-formats`: approve
- `exponentiation-operator`: approve
- `math-random`: approve
- `slice-04-conformance-close`: approve

3. Round-2 blockers in the design lane:
- A1, `number-format` runs `libm-check` with no edge: answered. `number-format` line 65 drops it and keeps its own grep.
- A3, `date_clock.rs` missing from the zone story's Writes: answered. Zone Writes (line 65) lists it with the two rewritten tests; `date-setters` (lines 43, 53) and `date-text-formats` (line 57) do the same.
- B1, `math-sum-precise` runs `libm-check` with no edge: answered. `math-sum-precise` line 60 drops it and keeps its own grep.
- The warnings of round 2 are answered in these places:
  - The clock story has no edge to the math story.
  - The `tBefore` list is held and credited by the caller's closure alone (calendar line 44, zone line 50).
  - The identifier is handed over as 20 units (zone line 50).
  - `round_ties_even` is stated as stricter than the list (`math-constants` line 56).
  - The pending counts and the `math-data-check` counts are relative.
  - `swell-num-date-text` no longer names `story:swell-num-math`.
  - `math-functions` has no edge to `number-functions`.
  - Pow is named `exponentiate`, and the Math.random message is 25 bytes.
- Not answered: the double-double story still holds the two gates. The coordinator decided to keep them (decision 1), so I report it as settled.

4. New findings. Each of the four start gates is a `grep -cF` on a bold marker. Each prints 0 on `swell-wave-s10-1` today and can print 1 once the sentence is written. The `math-random` gate prints 0 on its first two items today, and its third item already prints 2.
- `math-random`, Requirements and prerequisite (lines 69-77): the split adds `HOST-214` to the design. `host-requirements-check` fails on a requirement with no line, and no story writes `HOST-214 pending slice 9` into `requirements.txt`. The acceptance needs that check to exit 0. Name the owner: the design author or this story's Writes.
- `swell-num-math-exp-and-log` line 40 and `swell-num-math-trigonometry` line 40: the quoted "sentence" is an empty `> `. Say "the sentence of `swell-num-math-double-double`", where it stands in full.
- `date-text-formats` lines 56 and 58: it says "12 new functions" and "count plus 12". The list is 10: `toString`, `toDateString`, `toTimeString`, `toUTCString`, `toISOString`, `toJSON`, three `toLocale*String` and `[Symbol.toPrimitive]`.
- `date-object-clock-and-time-value` lines 6 and 7: the title and summary still promise "the nine references" and a consumer test of nine. The body builds two `Date.now` reads, and the nine are in the close.
- `math-constants-and-exact-functions` line 46 names the two `Date` reads as `new Date()` and `Date.now()`. The clock story (line 47) says both are `Date.now()`. Make them agree.
- `math-functions` line 7: the summary says 21 functions and 191 cases. The body says 22 and 189.
- `number-functions` line 70 ends "the header sentence is the one hunk both change: )", and the sentence itself is missing. Line 35, "No other story waits for it", is false, since `exponentiation-operator` has an edge to it.
- `swell-num-math-double-double` line 67: "n one more than on the unit's base" cannot be read, because the task does not exist on the base. This story creates it, so state `1 file(s)`.
- `date-setters` line 45: the grep `Set.*date_not_built` depends on a naming of table rows that the clock story does not fix. Name the line shape in the clock story, or anchor the grep on the quoted setter names.
- `math-sum-precise` line 63: "wave 2" is stale; the waves put it in wave 5. The same sentence says `math-functions` "comes first" and also that this story adds `SumPrecise` before `math-functions`' variants. State one order.
- Trade-off, `exponentiation-operator` (edges to `math-constants-and-exact-functions`, `number-functions` and `math-functions`): the hard edge is `swell-num-math-exp-and-log`. The other three exist for 12, 1 and 3 cases. Moving those cases to the close's joint list, as decision 3 did for `math-functions`, would let `**` start after `exp-and-log`. The same holds for the `number-functions` edges to the math and clock stories (4 and 2 cases) and the `date-text-formats` edge to math (1 case). Keep them or move the cases; no change is required.
- `math-constants-and-exact-functions` line 21 and `swell-num-calendar-and-local-time` line 21 say "in the first wave". The waves put them in 2 and 3. Both are buildable at once, with no design prerequisite.

Out of lane, not decisive: the shared `crates/num` files and the `Taskfile.yml` hunks are named in both stories. The waves print only collisions of that kind, and each has a named hunk.

5. Edges and reads:
- **Graph:** the whole store graph, 1,303 lines, including 42 `depends_on` edges from the 17 stories and the 8 out of `slice-10-conformance-close`, so I did walk outside the set. I found no cycle and no edge from outside back into slice 4.
- **Epic edges:** 17 `decomposes` edges, and `spec-scenarios-for-the-numbers-and-dates-requirements` appears only as a source of outgoing edges and is not named by any of the 17.
- **Acceptance reads:** about 42 reads of state a sibling produces, all recorded by an edge; the only unrecorded one is the Taskfile hunk shared with `gate-cpu-table-held-by-a-check`, which is named in both stories.
- **Validate:** `aep plan artifact validate` prints `valid`, with 3 unrelated stories that have no scope.

```findings
- file: .engineering/planning/story/math-random.md
  line: 69
  category: design
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the HOST-147 split adds HOST-214 to the design, host-requirements-check fails on a requirement with no line, and no story writes 'HOST-214 pending slice 9' into requirements.txt; name its owner."
- file: .engineering/planning/story/swell-num-math-exp-and-log.md
  line: 40
  category: design
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the quoted design sentence is an empty block quote; point to the sentence of swell-num-math-double-double or repeat it."
- file: .engineering/planning/story/swell-num-math-trigonometry.md
  line: 40
  category: design
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the quoted design sentence is an empty block quote; point to the sentence of swell-num-math-double-double or repeat it."
- file: .engineering/planning/story/date-text-formats.md
  line: 58
  category: design
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "it counts 12 new functions and a sweep count of plus 12, but the list is 10 (nine string methods and Symbol.toPrimitive)."
- file: .engineering/planning/story/date-object-clock-and-time-value.md
  line: 6
  category: design
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "title and summary still promise the nine references and the consumer test of nine, which the body moved to the close; the story holds the two Date reads."
- file: .engineering/planning/story/math-constants-and-exact-functions.md
  line: 46
  category: design
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "it names the consumer's two Date reads as new Date() and Date.now() while the clock story names two Date.now() reads; make them agree."
- file: .engineering/planning/story/math-functions.md
  line: 7
  category: design
  severity: note
  verdict: NEEDS-CHANGE
  origin: pre-existing
  message: "the summary says 21 functions and 191 cases while the body says 22 and 189."
- file: .engineering/planning/story/number-functions.md
  line: 70
  category: design
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the sentence naming the header hunk shared with number-format is missing (text ends 'both change: )'), and line 35 says no other story waits for it though exponentiation-operator depends on it."
- file: .engineering/planning/story/swell-num-math-double-double.md
  line: 80
  category: design
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "'n one more than on the unit's base' cannot be read because the task does not exist on the base; state 1 file(s)."
- file: .engineering/planning/story/date-setters.md
  line: 45
  category: design
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the grep 'Set.*date_not_built' depends on a row naming that no earlier story fixes; name the line shape or anchor on the quoted setter names."
- file: .engineering/planning/story/math-sum-precise.md
  line: 63
  category: design
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "'wave 2' is stale (wave 5), and the ordering sentence with math-functions says both 'first' and 'before' for SumPrecise; state one order."
- file: .engineering/planning/story/exponentiation-operator.md
  line: 10
  category: design
  severity: note
  verdict: CONFIRMED
  origin: undecided
  message: "three of its four edges rest on 12, 1 and 3 case reads; as with decision 3 for math-functions, the cases could go to the close's joint list so the operator starts after exp-and-log; a trade-off, not a defect."
```
