---
format: aep.planning-md/3
id: review-result:slice-04-round-2-a-design
kind: review-result
status: active
title: 'Slice 4, round 2, the design critic: the six stories that start first'
relations:
- reviews: story:math-constants-and-exact-functions
- reviews: story:swell-num-calendar-and-local-time
- reviews: story:date-object-clock-and-time-value
- reviews: story:date-object-constructor-now-utc-and-getters
- reviews: story:number-format
- reviews: story:swell-num-math-double-double
revision: 1
---
needs-revision — panel A (checked against the six stories, the store graph, decision 6 and decision 17)

Per story:
- `story:math-constants-and-exact-functions` — needs-revision
- `story:swell-num-calendar-and-local-time` — needs-revision
- `story:date-object-clock-and-time-value` — needs-revision
- `story:date-object-constructor-now-utc-and-getters` — needs-revision
- `story:number-format` — needs-revision
- `story:swell-num-math-double-double` — needs-revision

Findings:
1. `number-format` — blocker. Its acceptance (line 66) runs `slot task libm-check`, a task that only `swell-num-math-double-double` creates (its lines 56–57). The graph has no `depends_on` from `number-format` to it. The waves put `number-format` in wave 1 and `swell-num-math-double-double` in wave 2, because the double-double story shares `Taskfile.yml` with `gate-cpu-table-held-by-a-check`. On `number-format`'s base the task does not exist.
   - Option A: add the edge, which moves `number-format` to wave 3.
   - Option B: drop that acceptance item. Its own grep and `forbid-unsafe-check` already cover the rule.
   - Finding 2 is a third option.
2. `swell-num-math-double-double` — warning. This is two things in one story: the double-double arithmetic with `hypot` (`dd.rs`, `math.rs`), and two repository gates (`libm.yml`, `math-data.yml`, `Taskfile.yml`). The gates are tested only by fixtures and use nothing of `dd.rs`. The seam is the cause of finding 1 and of the wave 2 placement. Closing it: move `libm-check` and `math-data-check` into their own story, which `number-format` and the `math-*` stories can depend on, and which takes the `Taskfile.yml` hunk. The alternative is finding 1, option A.
3. `date-object-constructor-now-utc-and-getters` — blocker. Its acceptance (line 66–67) runs `test(/^date_clock::/)` and expects exit 0. Its change replaces the seam `date_from_components` and the interim `Date.UTC`. The clock story's `date_clock.rs` asserts those two seams (`new Date(1, 2)` gives NaN; each of the 16 interim bodies throws, which includes `Date.UTC`; clock line 74). After this story those tests are false. `date_clock.rs` is not in this story's Writes (line 63) or scope, so no hunk is named. The story must list `date_clock.rs` with the two tests it rewrites. The same shape holds for `date-setters` and `date-text-formats` (panel B): the `Date()` test expecting "Invalid Date" and the interim setter test.
4. `date-object-constructor-now-utc-and-getters` — warning. Line 48 reads "first 19 code units only" of the identifier. Decision 17 item 2 says a longer identifier is a named zone by its length. A 20-unit identifier whose first 19 units form an offset string would be read as an offset string. That contradicts the story's own 20-unit test and the calendar story's rule (line 41). State that the caller hands over 20 units, or the whole string, so that the length is visible.
5. `swell-num-calendar-and-local-time` — warning. The closure returns `Result<Vec<i128>, E>` (line 42), and the search keeps the `tBefore` list inside `swell-num` until the conversion ends. Line 44 leaves the charge to the caller. The zone story (line 48) releases its charge "when the probe ends". Decision 17 item 7 credits the bytes only when the list is dropped, which for the `tBefore` list is the end of the conversion. As drawn, the engine un-charges memory that is still held. Either change the closure to return only the first and last element, so that no `Vec` outlives it, or state who holds the list and when it is credited.
6. `date-object-clock-and-time-value` — warning (trade-off). The only reason for its `depends_on` to `math-constants-and-exact-functions` is that its consumer test repeats the seven `Math` calls (lines 42 and 74). The math story already holds that test (its line 46). The edge puts `Date.now()`, the consumer's other first gap, in wave 2, and makes a chain math, then clock, then zone. The `builtin_function.rs` hunks differ, so the edge is not needed for the shared file. Option A: keep the edge, since the nine-reference test is one test. Option B: keep only the two `Date.now` reads in the clock story and let the math story or the conformance close hold the combined test. Option B makes the clock story buildable in wave 1.
7. `math-constants-and-exact-functions` — warning. Line 56 says decision 6, "Which `f64` methods an engine crate may call", does not allow `round_ties_even`. That list allows it, and also `round`, `signum` and `fract`. Only the `Math.round` bullet rules it out. Cite the bullet as the source and say the story is stricter than the list (the design author may want to strike it from the list). The grep ban stays.

Open points in panel A:
- Point 2, `toLocaleString` (`number-format`): the recommended text of `toString()` is fine. The design does not state it yet, so the sentence should land in decision 6 before the story starts. I found no shape defect.
- Point 4, data file split: the double-double story only creates `SOURCE` and the checking task, and works for any number of files. The file split sits in the exp-and-log and trigonometry stories.
- Point 5, the two cases in neither list: that belongs to the scope critic.
- Points 1, 3 and 6 are panel B.

Out of my lane (they do not set my verdict):
- Acceptance critic, `number-format` line 66: the nextest `-E` filter repeats six terms.
- Acceptance critic, "Date and 22 functions": the clock story lists 20 functions (`now`, `parse`, `UTC`, `getTime`, `valueOf`, `setTime` and 14 setters), 21 with `Date` itself.
- Parallel-safety critic: the `math-constants` Change says `builtin_function.rs` "is not written", then adds a test there. The literal `12 pending` counts in the calendar and math stories depend on merge order.
- Parallel-safety critic: the module header of `number.rs` is split by line between `number-format` and `number-functions`.
- Stale line numbers: they are of the older base. On `swell-wave-s10-1`:

  | Item | Story says | On `swell-wave-s10-1` |
  |---|---|---|
  | `math.rs` | 87 lines | 86 lines |
  | Test `a_loop_of_builtin_calls_collects_within_its_bound` | line 746 | line 942 |
  | Dispatch | line 586 | line 760 |
  | `BuiltinId::Math` | line 62 | line 71 |
  | Pending lines in `requirements.txt` | 12 (14 on the wave) | 14 |

  The stories already say the pending count is 14 on the wave.
- The `IsoDateTime` narrowing from `i64` (`swell-num`) to `i32` (the hook record) has no owner in either the calendar or the zone story. It is harmless for years up to 275,760 but should be one sentence.

What I read: 6 stories whole, 12 more by edge only, and decision 6 and decision 17 (the sections by line). I ran `relations`, `graph`, `validate` and `waves --kind story --status draft`. `validate` reports only three unrelated stories with no scope. The graph has 959 edges and 258 `depends_on` lines. I read every edge touching the slice 4 stories, plus `slice-10-conformance-close` and `gate-cpu-table-held-by-a-check`. I found no cycle and no edge that returns into slice 4 from outside. Six reads of sibling-produced state were traced (clock reads Math, zone reads calendar, zone reads clock three ways, `number-format` reads `libm-check`). Five are recorded by an edge; one, finding 1, is not.

Not established:
- I did not check the case counts (165, 197, 133, 48) against `scan.tsv`. I only saw that the four list files have those line counts.
- I could not check the claim that the search finds no forbidden call today.
- The brief said no writes. I redirected the graph and two excerpts of the design text into the session scratch directory (`graph-A.txt`, `d6.txt`, `d17.txt`). Nothing was written in a repository.

```findings
- file: .engineering/planning/story/number-format.md
  line: 66
  category: design
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "its acceptance runs `slot task libm-check`, which only story:swell-num-math-double-double creates (and that story lands a wave later), and no depends_on edge runs from number-format to it; add the edge, move the gates to their own earlier story, or drop the item"
- file: .engineering/planning/story/swell-num-math-double-double.md
  line: 56
  category: design
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the story holds two things, the double-double arithmetic with hypot and the two repository gates (libm-check, math-data-check) that use none of it; split the gates and their Taskfile.yml hunk into a story of their own so dependents can order on it"
- file: .engineering/planning/story/date-object-constructor-now-utc-and-getters.md
  line: 63
  category: design
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "its acceptance runs date_clock:: and expects exit 0, but the change replaces the seams (date_from_components, interim Date.UTC) that date_clock.rs asserts (date-object-clock-and-time-value.md:74), and Writes does not list date_clock.rs or the tests it rewrites"
- file: .engineering/planning/story/date-object-constructor-now-utc-and-getters.md
  line: 48
  category: design
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "reading the first 19 code units only makes a 20-unit identifier whose first 19 units are an offset string look like an offset string, against decision 17 item 2 and the story's own 20-unit test; the identifier length must stay visible to parse_time_zone_offset_string"
- file: .engineering/planning/story/swell-num-calendar-and-local-time.md
  line: 42
  category: design
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the possible-instants closure returns a Vec that swell-num keeps for tBefore until the conversion ends while the caller releases its charge when the probe ends (zone story line 48), against decision 17 item 7; return only the first and last element or state who holds and credits the list"
- file: .engineering/planning/story/date-object-clock-and-time-value.md
  line: 42
  category: design
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the depends_on edge to math-constants-and-exact-functions exists only because its consumer test repeats the seven Math calls the math story already tests, which delays Date.now() to wave 2 and builds a math-clock-zone chain; keep it as a stated trade-off or split the combined test out"
- file: .engineering/planning/story/math-constants-and-exact-functions.md
  line: 56
  category: design
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "it says decision 6 'Which f64 methods an engine crate may call' does not allow round_ties_even, but that list allows it and only the Math.round bullet excludes it; cite the bullet and state that the story is stricter than the list"
```
