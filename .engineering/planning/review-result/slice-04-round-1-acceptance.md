---
format: aep.planning-md/3
id: review-result:slice-04-round-1-acceptance
kind: review-result
status: active
title: 'Slice 4, round 1, the acceptance critic: the three stories of the first wave'
relations:
- reviews: story:math-constants-and-exact-functions
- reviews: story:swell-num-calendar-and-local-time
- reviews: story:date-object-constructor-now-utc-and-getters
revision: 1
---
needs-revision
story:math-constants-and-exact-functions — the acceptance names `slot task check`, the whole gate, for "no new warning", where the story's own tests plus `slot task clippy` are the check — .engineering/planning/story/math-constants-and-exact-functions.md:59
story:swell-num-calendar-and-local-time — the acceptance names `slot task check`, the whole gate, for "no new warning", where its own tests plus `slot task clippy` are the check — .engineering/planning/story/swell-num-calendar-and-local-time.md:63
story:date-object-constructor-now-utc-and-getters — the acceptance names `slot task check`, the whole gate, for "no new warning", where its own tests plus `slot task clippy` are the check — .engineering/planning/story/date-object-constructor-now-utc-and-getters.md:82
story:math-constants-and-exact-functions — the acceptance has no time clause: no measured time per test, no sum, and nothing that fails it on a test over 1.5 CPU seconds (2 with `collect-every-safe-point`), though Change states the bounds; the accepted stories end with "a time over its bound fails this acceptance" — .engineering/planning/story/math-constants-and-exact-functions.md:59
story:date-object-constructor-now-utc-and-getters — the acceptance has no time clause either, so the 1.5 and 2 CPU second bounds of Change are not checked at close — .engineering/planning/story/date-object-constructor-now-utc-and-getters.md:82
story:swell-num-calendar-and-local-time — the acceptance says "no test runs over 5 s" while Change says each test stays under 1.5 CPU seconds, and the 1,461,000-day walk with a month-by-month oracle is "expected under 0.5 s" at opt-level 0 with no measurement, so the check is looser than the rule and the walk can break it — .engineering/planning/story/swell-num-calendar-and-local-time.md:63
story:math-constants-and-exact-functions — the acceptance runs only `--test main`, so the lib tests are not run: `a_loop_of_builtin_calls_collects_within_its_bound` (the 12,500-call `Math.max` test that the `coerce_all` refactor changes), `a_generator_keeps_the_object_record_at_its_size` (the 200-byte record) and `heap::collect`; `host_186_a_host_function_charges_every_byte` is not named either — .engineering/planning/story/math-constants-and-exact-functions.md:59
story:date-object-constructor-now-utc-and-getters — the acceptance omits `--lib` in both modes (the 200-byte `ObjectRecord` test, `heap::collect`), though `OrdinaryData::Date(f64)` is a new kind, and it omits `host_186_a_host_function_charges_every_byte`; only `host_187` is named — .engineering/planning/story/date-object-constructor-now-utc-and-getters.md:82
story:math-constants-and-exact-functions — the Change puts the `Math.min` twin of the 12,500-call test in `tests/builtins_math.rs` "so that `builtin_function.rs` is not written", but that test reads `crate::heap::COLLECTIONS`, `MIN_TRIGGER_BYTES` and `roomy()`, which are `pub(crate)` or `#[cfg(test)]` in the library; the twin cannot be built there, and the Writes line misses `crates/swell/src/object/builtin_function.rs`, which five open slice-10 stories write — .engineering/planning/story/math-constants-and-exact-functions.md:43
story:math-constants-and-exact-functions — `min` adds a loop and `max` is refactored, but the tests name no case that ends a call at the step budget or at an interrupt, for example `Math.min` over many arguments with `steps_per_entry` small — .engineering/planning/story/math-constants-and-exact-functions.md:45
story:math-constants-and-exact-functions — two checks are stated outside the acceptance and so are not enforced: the libm grep ("prints nothing", under Requirements) and the stale-text grep ("prints nothing after", under Change); the stale-text grep covers one phrase, and misses `math.rs:22` ("with `max` and @@toStringTag") and the header of `tests/builtins_math.rs` — .engineering/planning/story/math-constants-and-exact-functions.md:46
story:swell-num-calendar-and-local-time — the checks `git diff --stat crates/num/Cargo.toml` prints nothing and the `.sin()`/`.exp()` grep (under Requirements) are not in the acceptance, so nothing enforces them at close — .engineering/planning/story/swell-num-calendar-and-local-time.md:57
story:swell-num-calendar-and-local-time — the search names no behaviour for a gallop probe that falls past `LOCAL_TIME_LIMIT`, where `date_fields` answers `None`, near ±8.64e15 with an empty `possible`, and no test covers it, so a unit picks the answer — .engineering/planning/story/swell-num-calendar-and-local-time.md:48
story:date-object-constructor-now-utc-and-getters — the story moves lines 125, 126 and 128 of `conformance/host/requirements.txt` and regenerates `website/data/status.json`, but the acceptance gives no before and after (`grep -c 'pending slice 4$'` 9 to 6) and does not run `host-requirements-check` or `site-data-check`, as the accepted slice 10 and 7 stories do — .engineering/planning/story/date-object-constructor-now-utc-and-getters.md:82
story:date-object-constructor-now-utc-and-getters — the consumer test calls the seven `Math` functions and the acceptance runs `builtins_math::`, yet the relations hold only `depends_on story:swell-num-calendar-and-local-time` and the Writes line says "this story reads nothing of" the Math story; the acceptance cannot pass without `story:math-constants-and-exact-functions` merged, so an edge is needed — .engineering/planning/story/date-object-constructor-now-utc-and-getters.md:10
story:date-object-constructor-now-utc-and-getters — the Change lists the sites of `realm/intrinsics.rs` (enum, fields, accessor, creation) but not `Intrinsics::trace` (the root list at about line 292) or the `Intrinsics { .. }` literal at about line 519; a `%Date.prototype%` missing from the trace is collected after `delete globalThis.Date`, and no test in the story does that under `collect-every-safe-point` — .engineering/planning/story/date-object-constructor-now-utc-and-getters.md:64
story:date-object-constructor-now-utc-and-getters — the story removes "Date is not defined" but gives no search command for the sentences and fixtures that state it; `grep -rnI "Date is not defined"` and `Date.*slice 4` find `conformance/test262/later-parts.tsv:71` and the `Date` and `Math` rows of `website/docs/limits.md:30`, which are not named as left to the close or the release — .engineering/planning/story/date-object-constructor-now-utc-and-getters.md:62
story:math-constants-and-exact-functions — Context and Requirements carry no statement of "no unsafe code and no third-party crate" or of the run loop (`Agent::execute` frame and `STACK_BYTES_PER_REENTRY` unchanged, where `ToNumber` re-enters user code from `min` and `max`) — .engineering/planning/story/math-constants-and-exact-functions.md:53
story:date-object-constructor-now-utc-and-getters — Requirements carry no statement of "no unsafe code and no third-party crate" or of the run loop (`Agent::execute` frame and `STACK_BYTES_PER_REENTRY` unchanged, where the constructor's `ToPrimitive` and `ToNumber` re-enter user code) — .engineering/planning/story/date-object-constructor-now-utc-and-getters.md:76

What I read: 3 ids (`story:math-constants-and-exact-functions`, `story:swell-num-calendar-and-local-time`, `story:date-object-constructor-now-utc-and-getters`), all in full through `aep plan artifact show`, plus `aep plan artifact kinds` and `aep plan artifact lifecycle story`. In the code and the tree I read `check.sh`, `part.sh`, `scan.awk`, `math.rs`, `ordinary.rs`, `intrinsics.rs`, `hooks.rs`, `support/host.rs`, the HOST-187 test, `crates/num/src/date.rs` and the tests of `crates/num`, `requirements.txt`, the acceptance of three accepted stories and the base run. I did not read the other slice-10 stories.

Counts reproduced on the base run (read-only awk and `check.sh`, no build; scratch removed):
- Math: part 630 cases and 1,253 runs; `check.sh` prints 259 `failing` lines and `passed 287 bound 287 B 287 F 133 R 133`; the guard has 287 cases, all passing on the base.
- Date: part 917 cases and 1,832 runs; 728 `failing` lines and `passed 186 bound 186 B 186 F 364 R 364`; the "Date is not defined" count is 1,394; the guard has 186 cases, all passing on the base.
- Both story lists have every case failing on the base.

Seams: two list cases read `date_to_string`, `S15.9.2.1_A1` and `language/expressions/typeof/string`, which assert only `typeof Date(...) === "string"`. They pass correctly, and no list case passes a string through a variable, so I filed no finding on the seams.

What I could not establish:
- Out of my lane (no effect on my verdict): the cut of the Date story (1,350 lines; the clock and getters could be shown in two parts), the `IsoDateTime` i64 to i32 conversion in the closures, and the `host_187` reading, which measures only the rise after `create_realm` and so will probably not move.
- Whether the walk test stays under 1.5 CPU seconds; I did not run it.
- The slice-10 file overlaps beyond the three files I found (`builtin_function.rs` for the Math twin and the Date arm, `intrinsics.rs` for Date), and the other stories' scopes.

```findings
- file: .engineering/planning/story/math-constants-and-exact-functions.md
  line: 59
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance names `slot task check`, the whole gate, for \"no new warning\", where the story's own tests plus `slot task clippy` are the check"
- file: .engineering/planning/story/swell-num-calendar-and-local-time.md
  line: 63
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance names `slot task check`, the whole gate, for \"no new warning\", where its own tests plus `slot task clippy` are the check"
- file: .engineering/planning/story/date-object-constructor-now-utc-and-getters.md
  line: 82
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance names `slot task check`, the whole gate, for \"no new warning\", where its own tests plus `slot task clippy` are the check"
- file: .engineering/planning/story/math-constants-and-exact-functions.md
  line: 59
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance has no time clause: no measured time per test, no sum, and nothing that fails it on a test over 1.5 CPU seconds (2 with collect-every-safe-point), though Change states the bounds; the accepted stories end with a time over its bound fails this acceptance"
- file: .engineering/planning/story/date-object-constructor-now-utc-and-getters.md
  line: 82
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance has no time clause either, so the 1.5 and 2 CPU second bounds of Change are not checked at close"
- file: .engineering/planning/story/swell-num-calendar-and-local-time.md
  line: 63
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance says \"no test runs over 5 s\" while Change says each test stays under 1.5 CPU seconds, and the 1,461,000-day walk with a month-by-month oracle is \"expected under 0.5 s\" at opt-level 0 with no measurement, so the check is looser than the rule and the walk can break it"
- file: .engineering/planning/story/math-constants-and-exact-functions.md
  line: 59
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance runs only `--test main`, so the lib tests are not run: a_loop_of_builtin_calls_collects_within_its_bound (the 12,500-call Math.max test that the coerce_all refactor changes), a_generator_keeps_the_object_record_at_its_size (the 200-byte record) and heap::collect; host_186_a_host_function_charges_every_byte is not named either"
- file: .engineering/planning/story/date-object-constructor-now-utc-and-getters.md
  line: 82
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance omits `--lib` in both modes (the 200-byte ObjectRecord test, heap::collect), though OrdinaryData::Date(f64) is a new kind, and it omits host_186_a_host_function_charges_every_byte; only host_187 is named"
- file: .engineering/planning/story/math-constants-and-exact-functions.md
  line: 43
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the Change puts the Math.min twin of the 12,500-call test in tests/builtins_math.rs so that builtin_function.rs is not written, but that test reads crate::heap::COLLECTIONS, MIN_TRIGGER_BYTES and roomy(), which are pub(crate) or cfg(test) in the library; the twin cannot be built there, and the Writes line misses crates/swell/src/object/builtin_function.rs, which five open slice-10 stories write"
- file: .engineering/planning/story/math-constants-and-exact-functions.md
  line: 45
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "min adds a loop and max is refactored, but the tests name no case that ends a call at the step budget or at an interrupt, for example Math.min over many arguments with steps_per_entry small"
- file: .engineering/planning/story/math-constants-and-exact-functions.md
  line: 46
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "two checks are stated outside the acceptance and so are not enforced: the libm grep (Requirements) and the stale-text grep (Change); the stale-text grep covers one phrase and misses math.rs:22 (with `max` and @@toStringTag) and the header of tests/builtins_math.rs"
- file: .engineering/planning/story/swell-num-calendar-and-local-time.md
  line: 57
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the checks `git diff --stat crates/num/Cargo.toml` prints nothing and the .sin()/.exp() grep (Requirements) are not in the acceptance, so nothing enforces them at close"
- file: .engineering/planning/story/swell-num-calendar-and-local-time.md
  line: 48
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the search names no behaviour for a gallop probe that falls past LOCAL_TIME_LIMIT, where date_fields answers None, near +-8.64e15 with an empty possible, and no test covers it, so a unit picks the answer"
- file: .engineering/planning/story/date-object-constructor-now-utc-and-getters.md
  line: 82
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the story moves lines 125, 126 and 128 of conformance/host/requirements.txt and regenerates website/data/status.json, but the acceptance gives no before and after (grep -c 'pending slice 4$' 9 to 6) and does not run host-requirements-check or site-data-check, as the accepted slice 10 and 7 stories do"
- file: .engineering/planning/story/date-object-constructor-now-utc-and-getters.md
  line: 10
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the consumer test calls the seven Math functions and the acceptance runs builtins_math::, yet the relations hold only depends_on story:swell-num-calendar-and-local-time and the Writes line says this story reads nothing of the Math story; the acceptance cannot pass without story:math-constants-and-exact-functions merged, so an edge is needed"
- file: .engineering/planning/story/date-object-constructor-now-utc-and-getters.md
  line: 64
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the Change lists the sites of realm/intrinsics.rs (enum, fields, accessor, creation) but not Intrinsics::trace (the root list at about line 292) or the Intrinsics literal at about line 519; a %Date.prototype% missing from the trace is collected after delete globalThis.Date, and no test in the story does that under collect-every-safe-point"
- file: .engineering/planning/story/date-object-constructor-now-utc-and-getters.md
  line: 62
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the story removes \"Date is not defined\" but gives no search command for the sentences and fixtures that state it; grep -rnI \"Date is not defined\" and \"Date.*slice 4\" find conformance/test262/later-parts.tsv:71 and the Date and Math rows of website/docs/limits.md:30, which are not named as left to the close or the release"
- file: .engineering/planning/story/math-constants-and-exact-functions.md
  line: 53
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Context and Requirements carry no statement of no unsafe code and no third-party crate or of the run loop (Agent::execute frame and STACK_BYTES_PER_REENTRY unchanged, where ToNumber re-enters user code from min and max)"
- file: .engineering/planning/story/date-object-constructor-now-utc-and-getters.md
  line: 76
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Requirements carry no statement of no unsafe code and no third-party crate or of the run loop (Agent::execute frame and STACK_BYTES_PER_REENTRY unchanged, where the constructor's ToPrimitive and ToNumber re-enter user code)"
```
