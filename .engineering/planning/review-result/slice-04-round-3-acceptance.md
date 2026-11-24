---
format: aep.planning-md/3
id: review-result:slice-04-round-3-acceptance
kind: review-result
status: active
title: 'Slice 4, round 3, the acceptance critic: the 17 stories'
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
needs-revision

**Per story**
- math-constants-and-exact-functions: approve
- swell-num-calendar-and-local-time: approve
- date-object-clock-and-time-value: approve
- date-object-constructor-now-utc-and-getters: approve
- number-format: approve
- swell-num-math-double-double: needs-revision
- swell-num-date-text: approve
- math-sum-precise: approve
- number-functions: approve
- swell-num-math-exp-and-log: approve
- swell-num-math-trigonometry: approve
- date-setters: approve
- math-functions: approve
- date-text-formats: approve
- exponentiation-operator: needs-revision
- math-random: needs-revision
- slice-04-conformance-close: approve

**Round-2 blockers in my lane (7)**
- number-format, `libm-check`: answered. `number-format.md` line 65 says the story does not run it, and the acceptance (line 72) no longer lists it.
- number-format, `toFixed(0)` row: answered. Line 58 now gives `1000000000000000128`.
- clock story, consumer test count: answered. Line 77 works the loop through and prints `4`. The close (line 83) says `n` is 4.
- zone story, local time past `LOCAL_TIME_LIMIT`: answered. The test is gone (line 52), and the edge is tested through `new Date(8.64e15)` with `i128::MAX` (line 55).
- zone story, `UTC.*date_not_built`: answered. Line 63 anchors on `"UTC".*date_not_built`. See warning 5 below.
- math-sum-precise, `libm-check`: answered. Line 60 says the story calls no function of `swell_num::math`, and the acceptance drops the task.
- date-text-formats, pending count: answered. Lines 63 and 71 say "lower by 1 than on the unit's base".

**New findings**
1. `swell-num-math-double-double`, Change, line 68, blocker. The test says `hypot(1e308, 1e308)` is `1.414213562373095e308`. I computed it with `bc`. The Number 1e308 is 1.00000000000000001098e308, so the correctly rounded result is 1.41421356237309513e308 and prints as `1.4142135623730951e308`. The literal in the story parses to the Number one below. A correct `hypot` fails this test. Write the expected value as the result's bits or as 1.4142135623730951e308, or compare within 1 ulp.
2. `exponentiation-operator`, Change line 66 and Acceptance line 78, blocker. `grep -rn "exponentiation operator" crates` is meant to print nothing after the story. On `swell-wave-s10-1` it prints five lines. Two are the parser refusals and two are the fixtures this story deletes. The fifth is a comment in `crates/swell/tests/builtins_array_from_async.rs` (line 14), a file the story does not write. Restrict the grep to `crates/syntax/src` and `crates/syntax/tests`, or add that file with its hunk. The acceptance also says the first grep prints the two refusal lines on the base, which is wrong (five).
3. `math-random`, Requirements line 67 and Acceptance line 85, blocker. The story says the `HOST-214` line "stays pending slice 9". No such line exists. `host-requirements.yml` fails on "a requirement with no line". Once the design author writes `HOST-214`, `host-requirements-check` is red on the unit's base. The "exits 0 before and after" claim is then false, and the close's check with slice 4 delivered fails too. Make this story's `requirements.txt` hunk add `HOST-214 pending slice 9`. Alternatively the design author's sentence must come with that line.
4. `swell-num-math-exp-and-log` and `swell-num-math-trigonometry`, line 40 of each, warning. The prerequisite says "The sentence:" and the block quote is empty. The gate (`**Reference data of Math**`) can pass on the sentence in `swell-num-math-double-double`. These two stories should quote it or point to its place.
5. `date-object-clock-and-time-value`, line 53, warning. The clock story does not fix the layout of the interim bodies. The zone story's `"UTC".*date_not_built` and the setters story's `Set.*date_not_built` (14 lines) each assume a layout, and the two assume different ones. Nothing guarantees one line before and none after. State one line per interim, with the name literal on it.
6. `date-text-formats`, lines 56 and 58, warning. The story says 12 new functions and "the count before plus 12". The list holds 10: `toString`, `toDateString`, `toTimeString`, `toUTCString`, `toISOString`, `toJSON`, the three `toLocale*`, and `[Symbol.toPrimitive]`. Correct both places.
7. `math-functions`, line 7, warning. The summary still says "21 Math functions" and "191 cases". The body says 22 and 189.

**Start gates.** Each gate is a `grep -c` that passes only once its sentence is written.
- Run on `swell-wave-s10-1`, the three `engine.md` gates (toLocale, Reference data, parseInt) print 0 now.
- The three `math-random` gates print 0, 0 and 2 now. They print 1, 1 and 2 after the split.
- None of the gates holds `math-constants-and-exact-functions` or `date-object-clock-and-time-value`, which have none.

I also checked the case counts and part sizes against the pinned test262 checkout. They reproduce; the Date parts differ by 8 files, which are the Temporal tests the runner excludes. The scan totals, the stale-text greps of the Math, calendar and number stories, and the absence of `spec-scenarios-for-the-numbers-and-dates-requirements` and of person, path or session names in the 17 files all hold.

Out of my lane, not in the verdict: the `crates/num/src/lib.rs` header names `radix.rs`, `math.rs` and `random.rs` as slice 4's, while `number-functions` writes `parse.rs`.

What I read: all 17 story files whole, `review-result:slice-04-round-2-a-acceptance` and `-b-acceptance`, and the round-2 revision brief. I ran `aep plan artifact validate`, which printed `valid`, and `waves`, whose output was too long for me to read whole. I did not build or run any test.

What I could not establish: that the clock, zone and setters story layouts match their greps (nothing is written yet), and that the Math accuracy claims hold (they are the units' to measure).

```findings
- file: .engineering/planning/story/swell-num-math-double-double.md
  line: 68
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the test expects hypot(1e308, 1e308) to be 1.414213562373095e308, but the correctly rounded result is 1.41421356237309513e308 (prints 1.4142135623730951e308) and the literal parses to the Number one below, so a correct hypot fails it"
- file: .engineering/planning/story/exponentiation-operator.md
  line: 66
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "grep -rn \"exponentiation operator\" crates cannot print nothing after the story because crates/swell/tests/builtins_array_from_async.rs line 14 also matches and is not in Writes, and the stated base output of two lines is five"
- file: .engineering/planning/story/math-random.md
  line: 67
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the story says the HOST-214 line stays pending slice 9 but no such line exists, so host-requirements-check is red on the unit's base once the design is split and the close's check cannot pass; this story's requirements.txt hunk must add HOST-214 pending slice 9"
- file: .engineering/planning/story/swell-num-math-exp-and-log.md
  line: 40
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the prerequisite sentence the unit follows as written is an empty block quote; quote it or name where it stands in swell-num-math-double-double"
- file: .engineering/planning/story/swell-num-math-trigonometry.md
  line: 40
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the prerequisite sentence the unit follows as written is an empty block quote; quote it or name where it stands in swell-num-math-double-double"
- file: .engineering/planning/story/date-object-clock-and-time-value.md
  line: 53
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the layout of the interim bodies is not fixed, so the zone story's grep of the \"UTC\" entry and the setters story's grep of the Set arms (14 lines) have no guaranteed before state; state one line per interim with the name literal on it"
- file: .engineering/planning/story/date-text-formats.md
  line: 58
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the story says 12 new functions and a sweep count of the base plus 12 but lists 10 (toString, toDateString, toTimeString, toUTCString, toISOString, toJSON, three toLocale*, Symbol.toPrimitive)"
- file: .engineering/planning/story/math-functions.md
  line: 7
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the summary still says 21 Math functions and 191 cases while the body states 22 functions and 189 cases"
```
