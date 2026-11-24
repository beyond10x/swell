---
format: aep.planning-md/3
id: review-result:slice-04-round-1-scope
kind: review-result
status: active
title: 'Slice 4, round 1, the scope critic: the three stories of the first wave'
relations:
- reviews: story:math-constants-and-exact-functions
- reviews: story:swell-num-calendar-and-local-time
- reviews: story:date-object-constructor-now-utc-and-getters
revision: 1
---
needs-revision

- story:date-object-constructor-now-utc-and-getters — the story has no `depends_on` edge to `story:math-constants-and-exact-functions`, yet its consumer test runs the seven `Math` calls and its acceptance runs `builtins_math::`; it lands in wave 2 only because of the calendar edge, so add the edge or move the Math half of the nine-reference test — `.engineering/planning/story/date-object-constructor-now-utc-and-getters.md:10` (relations: only `depends_on story:swell-num-calendar-and-local-time`; `aep plan artifact graph` shows no other edge)
- story:date-object-constructor-now-utc-and-getters — the intrinsics hunk lists the enum, the fields, the accessor and the creation, but not `Intrinsics::trace`, which marks each intrinsic object; a Date or `Date.prototype` left out of it is freed by a collection — `crates/swell/src/realm/intrinsics.rs:289-297` (the list holding `self.number_prototype`), story line 64
- story:date-object-constructor-now-utc-and-getters — the story names no design precondition for the hook cost (the identifier once per conversion, one step per hook call, 59 calls at most) or for the offset-out-of-range requirement. The new requirement line has no test, and no story moves it, although the story carries a test of the clamp — `.engineering/planning/story/date-object-constructor-now-utc-and-getters.md:66` (and `:74`, which lists only HOST-125, 126, 128 and 127), brief item 1
- story:swell-num-calendar-and-local-time — the 57-probe bound of `UTC ( t )` and the NaN answer when no `tBefore` is found are design preconditions, but the story names only two design sentences (`MakeDay` and the offset hook) — `.engineering/planning/story/swell-num-calendar-and-local-time.md:48` (the two named at `:27` and `:49`)
- story:date-object-constructor-now-utc-and-getters — the story does not say what a Date object is charged, and the OrdinaryData variant it adds has no test that the object record stays 200 bytes. `object::generator::tests::a_generator_keeps_the_object_record_at_its_size` (a `--lib` test) is not named, and `host_186_a_host_function_charges_every_byte` is not run beside `host_187` — `.engineering/planning/story/date-object-constructor-now-utc-and-getters.md:69` and `:76`; `crates/swell/src/object/generator.rs:60-69`; `crates/swell/tests/host_objects.rs:970`
- story:math-constants-and-exact-functions — the story adds 11 functions to every realm and runs only `host_187`; `host_186_a_host_function_charges_every_byte` is neither run nor named, as `story:generator-functions-and-suspended-frames` does — `.engineering/planning/story/math-constants-and-exact-functions.md:45` and acceptance `host_objects::host_187`
- story:math-constants-and-exact-functions — the run-loop rule is missing: nothing says that `Agent::execute`'s frame, `STACK_BYTES_PER_REENTRY` and the stack tests (`limits_scripts`, `budgets`, `eval_stack`) are unchanged or what the measured change is. The story adds 11 arms to `run_math_builtin`, which `builtin_function.rs:586` dispatches — `.engineering/planning/story/math-constants-and-exact-functions.md:43` (Charges), `crates/swell/src/host/limits.rs:46`
- story:date-object-constructor-now-utc-and-getters — the same run-loop rule is missing, and here the story adds `BuiltinId::Date` and its dispatch arm; its acceptance filter does not name `limits_scripts`, `budgets` or `eval_stack` — `.engineering/planning/story/date-object-constructor-now-utc-and-getters.md:76` and the acceptance filter, `crates/swell/src/object/builtin_function.rs:586`
- story:swell-num-calendar-and-local-time — no `f64` call is named with a statement that IEEE 754 fixes its result: `floor(t / msPerDay)`, `trunc`, the division by `msPerDay` and the `f64` to `i64` casts are used, and the story only says "no std transcendental function" — `.engineering/planning/story/swell-num-calendar-and-local-time.md:57`
- story:date-object-constructor-now-utc-and-getters — the same gap: the `i128` to `f64` read of the clock, the division in `getTimezoneOffset` and `time_clip` are not named with an IEEE statement; the Requirements text only bans std transcendentals — `.engineering/planning/story/date-object-constructor-now-utc-and-getters.md:76`

**Cut.** The three stories hold: each outcome of the wave is claimed once, and the nine consumer references are all covered (seven `Math` calls in the Math story and `Date.now` twice in the Date story, with the combined test in the Date story). `setTime` is in the Date story only, since `scan.awk` excludes it from `date-setters`. Date being one 1,350-line session is a size question for the other critics. The calendar story shows nothing in the engine alone (0 cases), but `crates/num/tests/calendar.rs` shows it without an engine.

**Seams.** The only cases on the Date list that reach a seam are `S15.9.2.1_A1` and `language/expressions/typeof/string`, and both assert `typeof Date() === "string"`. The seam answer "Invalid Date" passes them correctly, and no consumer script reads a seam.

**Writes line.** Checked against the code: no other `match` on `OrdinaryData` lacks a wildcard (`ordinary.rs:90`, `object/mod.rs:560`, `json/mod.rs:418`); the global-names test reads only the tail names (`builtins_global_this.rs:20`), so adding `Date` to the table breaks nothing; `crates/test262/tests` records no Math or Date line; and no hook default or public item changes, so `surface.txt` stays untouched. The one site the line misses is `Intrinsics::trace`, in the second finding.

Part 3, what I read: 3 stories, the epic, the decomposition report and the graph (`aep plan artifact show` on the three stories and the epic, `aep plan artifact graph`, `aep plan artifact list`). I also read `scan.awk`, the three case lists, the runner host and the other slice-10 stories' scopes. I extracted 17 promises from the epic (8 outcome bullets and 9 acceptance items); the wave claims 4 of them: Math (in part), Date arithmetic, clock and zone, and acceptance 8. The other 13 belong to the later waves the brief names.

Part 4, what I could not establish:
- I have no verdict on the shared-file hunks of the slice-10 stories (out of lane, parallel-safety). The Date story gives its own hunks. It does not state for `builtin_function.rs` whether `async-generator-function-constructor` is among the five open writers, and it does not say that the join can take both.
- The acceptance runs `slot task check` where the brief wants only the story's own tests and `cargo clippy` (out of lane, acceptance).
- Whether the runner clock fixed at 0 (`crates/test262/src/host.rs:106`) and the default zone UTC satisfy the epic's sentence "the command-line host and the test262 runner answer 'UTC' unless told otherwise" is not claimed by any of the three stories. It looks met by defaults, so I raised no finding.

```findings
- file: .engineering/planning/story/date-object-constructor-now-utc-and-getters.md
  line: 10
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the story has no depends_on edge to story:math-constants-and-exact-functions, yet its consumer test runs the seven Math calls and its acceptance runs builtins_math::; it lands in wave 2 only because of the calendar edge, so add the edge or move the Math half of the nine-reference test"
- file: crates/swell/src/realm/intrinsics.rs
  line: 289
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the story's intrinsics hunk (enum, fields, accessor, creation) omits Intrinsics::trace, which marks every intrinsic object; a Date or Date.prototype left out of it is freed by a collection"
- file: .engineering/planning/story/date-object-constructor-now-utc-and-getters.md
  line: 66
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the story names no design precondition for the hook cost (identifier once per conversion, one step per hook call, 59 calls at most) or for the offset-out-of-range requirement; the new requirement line has no test and no story moves it, although the story carries a test of the clamp"
- file: .engineering/planning/story/swell-num-calendar-and-local-time.md
  line: 48
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the 57-probe bound of UTC ( t ) and the NaN answer when no tBefore is found are design preconditions, but the story names only two design sentences (MakeDay and the offset hook)"
- file: .engineering/planning/story/date-object-constructor-now-utc-and-getters.md
  line: 69
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the story does not say what a Date object is charged, and neither the object-record-stays-200-bytes test (object::generator::tests::a_generator_keeps_the_object_record_at_its_size) nor host_186_a_host_function_charges_every_byte is named beside host_187"
- file: .engineering/planning/story/math-constants-and-exact-functions.md
  line: 45
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the story adds 11 functions to every realm and runs only host_187; host_186_a_host_function_charges_every_byte is neither run nor named"
- file: .engineering/planning/story/math-constants-and-exact-functions.md
  line: 43
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the run-loop rule is missing: nothing says that the frame of Agent::execute, STACK_BYTES_PER_REENTRY and the stack tests (limits_scripts, budgets, eval_stack) are unchanged or what the measured change is, though the story adds 11 arms to run_math_builtin"
- file: .engineering/planning/story/date-object-constructor-now-utc-and-getters.md
  line: 76
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the run-loop rule is missing: the story adds BuiltinId::Date and its dispatch arm and says nothing of the frame of Agent::execute, STACK_BYTES_PER_REENTRY or the stack tests, and its acceptance filter does not name limits_scripts, budgets or eval_stack"
- file: .engineering/planning/story/swell-num-calendar-and-local-time.md
  line: 57
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "no f64 call is named with a statement that IEEE 754 fixes its result: floor(t / msPerDay), trunc, the division by msPerDay and the f64 to i64 casts are used, and the story only says no std transcendental function"
- file: .engineering/planning/story/date-object-constructor-now-utc-and-getters.md
  line: 76
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "no f64 operation is named with an IEEE statement: the i128 to f64 read of the clock, the division in getTimezoneOffset and time_clip; the Requirements text only bans std transcendentals"
```
