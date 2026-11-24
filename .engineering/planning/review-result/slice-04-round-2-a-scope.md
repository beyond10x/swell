---
format: aep.planning-md/3
id: review-result:slice-04-round-2-a-scope
kind: review-result
status: active
title: 'Slice 4, round 2, the scope critic: the six stories that start first'
relations:
- reviews: story:math-constants-and-exact-functions
- reviews: story:swell-num-calendar-and-local-time
- reviews: story:date-object-clock-and-time-value
- reviews: story:date-object-constructor-now-utc-and-getters
- reviews: story:number-format
- reviews: story:swell-num-math-double-double
revision: 1
---
approve — panel A (scope lane). No blocker in the lane. Two warnings and one note, and three out-of-lane defects that I think are blockers (below).

**Per story**
- `story:math-constants-and-exact-functions` — approve
- `story:swell-num-calendar-and-local-time` — approve
- `story:date-object-clock-and-time-value` — approve (one warning)
- `story:date-object-constructor-now-utc-and-getters` — approve
- `story:number-format` — approve
- `story:swell-num-math-double-double` — approve (one warning)

I applied the brief's rule that a warning does not stop approval. Under the rubric's stricter wording the panel would be `needs-revision`.

**Coverage**
- I extracted 14 promises from the epic that fall to panel A (or are named in its text) and traced all 14 to a story or to the base: calendar, clock, time zone, `toFixed`/`toExponential`/`toPrecision`, exact `Math`, `hypot`, and the fixed-clock acceptance (epic line 103).
- The Date key lists (clock 165, zone 197, setters 178, text 165) equal their `scan.tsv` keys exactly. The 11 keys together hold 1,351 rows, which is the 1,612 cases of the base run less its 261 passes.
- The lists are disjoint inside panel A. The three cases that are on no list belong to panel B (see "What I could not establish").

**Open points that are in panel A**
- **Point 5 (two of the 364 cases):** no gap. Every failing case of the base run is on exactly one list, so the two cases are not failing.
- **Point 2 (`Number.prototype.toLocaleString`):** inside the epic. Design line 180 allows "ECMA-262's locale-independent definitions of `toLocaleString`" meanwhile, so answering the text of `toString()` stays within the exclusion.
- **Point 4 (split of the reference data):** finding 1 below.

**Findings**
1. `story:swell-num-math-double-double`, "Out of scope" (line 77) and the test line 58 — warning. The design's `data/math/reference.txt` holds `hypot` lines. The story commits no data. The other two stories list no `hypot`, and line 77 says "the other two commit theirs". The story checks `hypot` against an oracle computed at test time, which design line 1993 rejects as "a second implementation … at test time". Name an owner for the `hypot` reference lines. This story is the natural one, with a `data/math/SOURCE` line. Epic acceptance 5 itself is met by `math-functions`' per-function bit tables.
2. `story:date-object-clock-and-time-value`, "Out of scope" (line 99) — warning. It says "the 17 getters but `getTime` and `getTimezoneOffset`". The zone story counts `getTimezoneOffset` among its 17 and holds HOST-127's first half. The clock story does not build it. Reword so `getTimezoneOffset` is plainly the zone story's.
3. Line numbers — note. The stories cite `object/builtin_function.rs` lines 62, 586 and 746. On `wave/s10-1` (`2b5b095`) they are 71, 760 and 942. The 87 lines of `math.rs` are 86 there. Other counts match: `number.rs` 241, `to_string.rs` 493, `date.rs` 173, 14 pending lines, and the stale-text greps. The hunks "after line 746" need re-reading on the wave.

Epic sentences that are stale and relevant:
- The epic's `round_ties_even` and `mul_add` "from `std`" (lines 32-33): the math story rightly follows design line 1878 and forbids both.
- `AvailableNamedTimeZoneIdentifiers`: the zone story drops it by name with a reason, and I did not count that as a gap.
- "Command-line host answers UTC": `swell-cli` does not exist yet. The runner host already answers epoch 0 and the trait default zone "UTC".

**Out of my lane, did not set the verdict (acceptance and parallel-safety)**
- The zone, setters and text stories all run `date_clock::` in their acceptance. The clock story's tests assert the interim state: 16 interim bodies throw, `new Date(1, 2)` is NaN, `new Date("…")` is NaN, and a call without `new` gives "Invalid Date". Those tests fail once the later stories replace the seams, and none of those stories writes `crates/swell/tests/date_clock.rs`. I read this as a blocker for the zone story. Name that file and the hunk in each of the three.
- `story:number-format` acceptance (line 66) runs `slot task libm-check`. That task is created by the double-double story. `number-format` has no edge to it, so the task does not exist when `number-format` is built. I read this as a blocker.
- `story:swell-num-math-double-double` writes `Taskfile.yml` and names only `slow-lane-starts-by-hand`. The graph also shows a collision with `story:gate-cpu-table-held-by-a-check`.
- The clock story depends on the math story only for its nine-reference test. That puts `Date.now()` in wave 2 although the consumer's first gap is wave 1 work.

**What I read**
- 6 panel A stories whole and the epic whole.
- The partition of the case lists against `scan.tsv`.
- The design text of decisions 6 and 17 and the hooks, the `conformance/host/requirements.txt` lines, and the claimed line numbers and greps, on `wave/s10-1`.

**What I could not establish**
- I read panel B only far enough to place edges.
- `story-exponentiation.cases.txt` (76 against 78 in `scan.tsv`) and the sumPrecise case `throws-on-non-number` (moved to slice 12 in its story) belong to panel B. They are not this panel's.
- I ran no tests and measured no times.

```findings
- file: .engineering/planning/story/swell-num-math-double-double.md
  line: 77
  category: scope
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the design's data/math/reference.txt holds hypot lines but this story commits no data and says the other two stories commit theirs, and neither lists hypot, so those reference lines have no owner and hypot is checked only by an in-test exact oracle"
- file: .engineering/planning/story/date-object-clock-and-time-value.md
  line: 99
  category: scope
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Out of scope says 'the 17 getters but getTime and getTimezoneOffset' although the zone story counts getTimezoneOffset among its 17 and holds HOST-127's first half, so ownership of getTimezoneOffset reads as unclear"
- file: .engineering/planning/story/math-constants-and-exact-functions.md
  line: 43
  category: scope
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the quoted lines of object/builtin_function.rs (62, 586, 746) and the 87-line math.rs are of the older base; on wave/s10-1 they are 71, 760, 942 and 86, so the unit must re-read the hunks there"
```
