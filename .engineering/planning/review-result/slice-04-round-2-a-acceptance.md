---
format: aep.planning-md/3
id: review-result:slice-04-round-2-a-acceptance
kind: review-result
status: active
title: 'Slice 4, round 2, the acceptance critic: the six stories that start first'
relations:
- reviews: story:math-constants-and-exact-functions
- reviews: story:swell-num-calendar-and-local-time
- reviews: story:date-object-clock-and-time-value
- reviews: story:date-object-constructor-now-utc-and-getters
- reviews: story:number-format
- reviews: story:swell-num-math-double-double
revision: 1
---
needs-revision — panel A

- `story:math-constants-and-exact-functions` — needs-revision
- `story:swell-num-calendar-and-local-time` — needs-revision
- `story:date-object-clock-and-time-value` — needs-revision
- `story:date-object-constructor-now-utc-and-getters` — needs-revision
- `story:number-format` — needs-revision
- `story:swell-num-math-double-double` — needs-revision

I read all six whole and checked their counts and commands against the wave worktree and the base run. I recomputed the counts from the base run with awk. Line numbers are the story file's own; `cat -n` was run per file. The wave worktree was not rebuilt or run.

Findings, per story:

**`story:number-format`**
- Acceptance (line 66): `slot task libm-check` must exit 0. That task is created by `story:swell-num-math-double-double`, and number-format has no edge to it. It does not exist in the wave worktree's `Taskfile.yml` or `taskfiles/`. Both stories are in panel A and can run together, so the check cannot pass. Replace it with a grep, or add the edge.
- Change line 52: the table says `(1000000000000000128).toFixed(0)` is `1000000000000000100`. The pinned spec note (`ecma262.txt:32726`) says `toFixed(0)` returns `"1000000000000000128"`; `"…100"` is the `toString` text. A correct `toFixed` fails this row. test262 `toFixed/exactness.js` pins the same.
- Lines 52, 66, 70: the counts do not reproduce. The part with the three directories copied whole is 671 cases and 1,342 runs, not 304 and 608. On the base `check.sh` prints `passed 499 bound 499 B 499 F 48 R 48`, not `passed 250 … B 250`. The 96 `failing` lines are right. The same method reproduces the math and both date parts.
- Lines 35, 51: `toLocaleString` has no test of its text and no grep for the header sentence it promises. Add a test that `x.toLocaleString()` equals `x.toString()` over a table, and a grep for the header sentence. The recommended text (open point 2) is checkable once those exist.
- Line 52: "`memory_budget` just below the 2,048 bytes" cannot be an absolute limit, because a realm needs more heap than that. State how the room of 2,047 bytes is set, as the HOST-188 test does.
- Line 66: the filter repeats `host_186`, `host_187`, `limits_scripts`, `budgets`, `eval_stack`, `termination`, `interrupt` and `tracing`. Remove the duplicates.

**`story:date-object-clock-and-time-value`**
- Line 74: the consumer test expects `5`. With `clock_step_ns` 1,000,000 added after each answer, the loop runs 4 times (reads T+1 to T+4 are above zero, T+5 gives 0). A correct implementation prints 4. Fix the number, or `end = start + 6`.
- Lines 75, 91: "the base count plus 22". I count 21 functions: `Date`, `now`, `parse`, `UTC`, `getTime`, `valueOf`, `setTime` and 14 setters. The sweep dedupes by object, so `Date.prototype.constructor` adds none. Correct the count, or name the 22nd.
- Line 68: the `time_clip` list says `8.64e15 + 2` is "the next Number above". The next Number is `8.64e15 + 1` (spacing 1 below 2^53). Add `8.64e15 + 1`.
- Line 48: "17 cases `S15.9.5_A*`" should be 18. The case file holds 18, and 41 = 23 + 18.

**`story:date-object-constructor-now-utc-and-getters`**
- Change (story line 53): the test "a time value near the edge whose local time passes `LOCAL_TIME_LIMIT` gives NaN" cannot be built. A valid time value is at most 8.64e15 and the clamped offset is under 86,400,000 ms, so the local time stays inside the limit. The design says so (`engine.md` ~2292, "`LocalTime` of a time value stays inside the same limit"). Remove the test, or test `date_fields(None)` only in `swell-num`.
- Stale text (line 61) and Acceptance (line 67): `grep -n "UTC.*date_not_built"` is meant to print nothing after this story. The 7 interim `setUTC*` setters stay until `date-setters`, and their lines match, so it prints 7 lines. It also does not print "one line" on the clock result; it prints 8. Anchor it on the `Date.UTC` entry (`"UTC".*date_not_built`).

**`story:math-constants-and-exact-functions`**
- Change line 48 and Acceptance line 62: `grep -n "No script runs" crates/swell/tests/builtins_math.rs` prints nothing on the base, because "No script" ends line 2 and "runs." starts line 3. The check has no transition. Use `grep -n "No script$"`.
- Line 46 and Acceptance line 62: the `Math.min` twin library test has no name. Name it.
- Line 47: `clz32` of `2**32`. `**` is refused on the base (`parser/expression.rs:486`). Write 4294967296, passed through `cx.call`.
- Line 46: `builtin_function.rs:746` is of the older base; the test is at line 942 on `wave/s10-1`. "The lines after the existing test" is the safe wording.

**`story:swell-num-calendar-and-local-time`**
- Line 46: "every day from -1000-01-01 to 3000-12-31 (1,461,000 days)". The count is 4,001 × 365 + 970 = 1,461,335. Fix the number.
- Line 41: the offset-string grammar (`ecma262.txt:33798` ff.) takes `.` or `,` as the fraction separator and only ASCII `+`/`-` as sign. No `,` case is listed. The expected result for U+2212 is not stated; it should be `None`. Add both.

**`story:swell-num-math-double-double`**
- Lines 58, 70: the tests assert "the bound the unit measured", so any measure passes, and nothing fails the acceptance at the design's 2^-100. Name the ceiling at which the story is returned, or say that a worse measure is accepted once reported.
- Line 58: "`two_sum` and `two_product` are exact … and the boundaries". That cannot hold for a subnormal or underflowing product, or for an overflowing sum. State the range of exactness and what the boundary cases assert.
- Lines 57, 70: `math-data-check` names no fixture paths and no passing fixture with one file. "`check` lists both tasks" is read through `task --list-all`, which does not show dependencies. Name the fixtures and the commands with their expected lines, and use `grep -n "task: libm-check" Taskfile.yml` for the dependency.
- The story writes tasks and shell scripts but carries no "No Python" line (`AGENTS.md`).

**The open points in panel A**
- **Point 2:** the recommendation is acceptable if the `toLocaleString` test is added.
- **Point 4:** the split of the reference data file is not stated in this story, since it commits none. See the `math-data-check` finding.
- **Point 5:** not established. `scan.tsv` holds 165 + 197 `date-clock` and `date-zone` rows and no `other` row, so the two cases must sit under `date-setters`, `date-text` or a later part.

Out of lane, not in the verdict:
- `crates/num/src/lib.rs` and `crates/num/tests/main.rs` are written by number-format, double-double, the clock story and the calendar story. Calendar says it shares only `main.rs` with the clock story.
- The front matter of number-format and the clock story omits `crates/num/src/lib.rs`, which they write.

What I could not establish:
- Whether `1.414213562373095e308` is the shortest text of `hypot(1e308, 1e308)`.
- The 96 and 394 `failing`-line counts for the Date and number-format parts. The clock and zone `Date is not defined` counts (1,356 and 1,184) and the guard counts did reproduce.
- `docs/design/engine.md` decision 17 was not read whole.

Files: the six story files under `.engineering/planning/story/` of `swell-plan-slice-04`.

```findings
- file: .engineering/planning/story/number-format.md
  line: 66
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the acceptance requires `slot task libm-check` to exit 0, but that task is created by story:swell-num-math-double-double, with which number-format has no edge, and it does not exist in the wave worktree"
- file: .engineering/planning/story/number-format.md
  line: 52
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the table says (1000000000000000128).toFixed(0) is 1000000000000000100, but ECMA-262 21.1.3.3 and test262 exactness.js give 1000000000000000128, so a correct toFixed fails the row"
- file: .engineering/planning/story/number-format.md
  line: 70
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the stated part size (304 cases, 608 runs) and the check.sh line (passed 250 bound 250 B 250) do not reproduce; the part with the three directories whole is 671 cases, 1342 runs and prints passed 499 bound 499 B 499 F 48 R 48"
- file: .engineering/planning/story/number-format.md
  line: 51
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "toLocaleString has no test of its text and no grep for the header sentence that fixes it, so the open choice (text of toString) is not checkable"
- file: .engineering/planning/story/number-format.md
  line: 52
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "a memory_budget just below 2,048 bytes cannot be an absolute limit, since the realm needs more heap; the story must state how the room of 2,047 bytes is set"
- file: .engineering/planning/story/number-format.md
  line: 66
  category: acceptance
  severity: note
  verdict: needs-revision
  origin: introduced
  message: "the nextest filter repeats host_186, host_187, limits_scripts, budgets, eval_stack, termination, interrupt and tracing"
- file: .engineering/planning/story/date-object-clock-and-time-value.md
  line: 74
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the consumer test expects 5 but with clock_step_ns 1,000,000 added after each answer the busy-wait loop runs 4 times, so a correct implementation fails it"
- file: .engineering/planning/story/date-object-clock-and-time-value.md
  line: 75
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the stated sweep count of the base plus 22 does not match the 21 functions the story adds (Date, 3 statics, 3 prototype methods, 14 setters)"
- file: .engineering/planning/story/date-object-clock-and-time-value.md
  line: 68
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the time_clip cases call 8.64e15 + 2 the next Number above, but the next is 8.64e15 + 1, which is the first clipped value and is not in the list"
- file: .engineering/planning/story/date-object-clock-and-time-value.md
  line: 48
  category: acceptance
  severity: note
  verdict: needs-revision
  origin: introduced
  message: "the text says 17 cases S15.9.5_A* but the case list holds 18 (4 named properties and 14 setters), which is what makes the 41 of built-ins/Date/prototype"
- file: .engineering/planning/story/date-object-constructor-now-utc-and-getters.md
  line: 53
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the test of a time value whose local time passes LOCAL_TIME_LIMIT cannot be built, since a valid time value plus a clamped offset below one day stays inside the limit (engine.md says LocalTime stays inside it)"
- file: .engineering/planning/story/date-object-constructor-now-utc-and-getters.md
  line: 61
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "grep -n \"UTC.*date_not_built\" is to print nothing after this story but also matches the 7 interim setUTC* setters that stay until date-setters, and prints 8 lines rather than one on the clock result"
- file: .engineering/planning/story/math-constants-and-exact-functions.md
  line: 48
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "grep -n \"No script runs\" crates/swell/tests/builtins_math.rs prints nothing on the base because the phrase wraps across lines 2 and 3, so the check has no before state"
- file: .engineering/planning/story/math-constants-and-exact-functions.md
  line: 46
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the Math.min twin of a_loop_of_builtin_calls_collects_within_its_bound has no test name, so the acceptance cannot select it"
- file: .engineering/planning/story/math-constants-and-exact-functions.md
  line: 47
  category: acceptance
  severity: note
  verdict: needs-revision
  origin: introduced
  message: "the clz32 input 2**32 is refused by the parser on the base, so the table must hold 4294967296 passed through cx.call"
- file: .engineering/planning/story/math-constants-and-exact-functions.md
  line: 46
  category: acceptance
  severity: note
  verdict: needs-revision
  origin: introduced
  message: "builtin_function.rs:746 is of the older base (the test is at line 942 on wave/s10-1), so the hunk should be named by the existing test and not the number"
- file: .engineering/planning/story/swell-num-calendar-and-local-time.md
  line: 46
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the full walk from -1000-01-01 to 3000-12-31 is 1,461,335 days, not the stated 1,461,000"
- file: .engineering/planning/story/swell-num-calendar-and-local-time.md
  line: 41
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the offset-string cases omit the comma fraction separator the grammar allows and state no expected result for the U+2212 minus sign, which the grammar rejects"
- file: .engineering/planning/story/swell-num-math-double-double.md
  line: 70
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the tests assert the bound the unit measured, so any measured error passes; the acceptance names no ceiling at which it fails"
- file: .engineering/planning/story/swell-num-math-double-double.md
  line: 58
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "exactness of two_sum and two_product at the boundaries cannot hold for a subnormal or underflowing product or an overflowing sum, and the story does not state the range or what those cases assert"
- file: .engineering/planning/story/swell-num-math-double-double.md
  line: 70
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "math-data-check names no fixture paths, no passing fixture with a file and no expected lines, and `task --list-all` does not show that check lists both tasks"
- file: .engineering/planning/story/swell-num-math-double-double.md
  line: 58
  category: acceptance
  severity: note
  verdict: needs-revision
  origin: introduced
  message: "the story writes tasks and shell scripts but carries no statement of the no-Python rule"
```
