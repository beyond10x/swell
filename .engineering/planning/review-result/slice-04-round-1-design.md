---
format: aep.planning-md/3
id: review-result:slice-04-round-1-design
kind: review-result
status: active
title: 'Slice 4, round 1, the design critic: the three stories of the first wave'
relations:
- reviews: story:math-constants-and-exact-functions
- reviews: story:swell-num-calendar-and-local-time
- reviews: story:date-object-constructor-now-utc-and-getters
revision: 1
---
needs-revision
story:date-object-constructor-now-utc-and-getters — its consumer test `consumer_references_of_surf_resolve_as_the_standard_says` calls the seven `Math` functions that only `story:math-constants-and-exact-functions` adds, and its acceptance runs `builtins_math::`, but its only `depends_on` edge runs to the calendar story, so a unit on a branch without the Math story cannot pass its own test; add `depends_on story:math-constants-and-exact-functions` (the Math story's line 44 hands this test to the Date story) — .engineering/planning/story/date-object-constructor-now-utc-and-getters.md:68 and :82, .engineering/planning/story/math-constants-and-exact-functions.md:44, `aep plan artifact graph`
story:date-object-constructor-now-utc-and-getters — two separable things sit in one story: the clock part (the `Date` function, `OrdinaryData::Date`, `Date.now`, `new Date()`, HOST-125 and HOST-126, which are the consumer's only `Date` need) needs no calendar function, while the zone part (getters, `Date.UTC`, multi-argument constructor, `getTimezoneOffset`, HOST-128) needs the calendar story; the single edge gates the consumer's first gap behind the calendar story and 1,350 lines; cut it into a clock story with no edge to the calendar story (its `new Date(value)` form needs only `time_clip`, which can be listed as the one reuse), and a zone story depending on both the calendar story and the clock story — .engineering/planning/story/date-object-constructor-now-utc-and-getters.md:63 and :84 (the sentence that rules out the cut names only the string forms), `aep plan artifact graph`
story:date-object-constructor-now-utc-and-getters — the constructor and `Date.UTC` apply `MakeFullYear` (0 to 99 meaning 1900 to 1999), but `crates/num/src/date.rs` has no such function and the calendar story's list of missing functions does not name it, so the arithmetic has no owner and is left to be written inline in the engine crate untested; add `make_full_year` to the calendar story (with cases for NaN, the infinities, -0.5, 0, 99, 100), or say in this story that it is written in `builtins/date.rs` and tested there — .engineering/planning/story/date-object-constructor-now-utc-and-getters.md:63, .engineering/planning/story/swell-num-calendar-and-local-time.md:47
story:date-object-constructor-now-utc-and-getters — the story fixes the hook call counts and charges (the identifier once per conversion, one step per hook call, at most 59 calls for `UTC ( t )`) as if they were settled, but they are design precondition 4 that a design author is still writing; it does not name that precondition, so a unit cannot tell the numbers may move; name it, and say the numbers come from the design sentence once it exists — .engineering/planning/story/date-object-constructor-now-utc-and-getters.md:66, `docs/design/host-interface.md` (the clock and time zones has no sentence on call counts)
story:swell-num-calendar-and-local-time — it names precondition 1 (`MakeDay`) and precondition 3 (offset clamp) as design sentences for a design author, but the 57-probe bound of the `tBefore` search and its NaN answer are written as decided behaviour without naming the precondition (design change 2 of the decomposition report); name it beside the other two — .engineering/planning/story/swell-num-calendar-and-local-time.md:48 and :59
story:swell-num-calendar-and-local-time — `utc_from_local` says what a `t` refused by `date_fields` does at the entry (NaN, no call), but not what a probe of the gallop or bisection does when `t - 2^k` falls below `-LOCAL_TIME_LIMIT` and `date_fields` refuses it; the unit has to choose between treating it as empty (the search ends in NaN) and an error, so state the rule and add a test at the lower edge — .engineering/planning/story/swell-num-calendar-and-local-time.md:47 and :48

What I read: 3 stories whole and the epic list; `aep plan artifact relations`, `graph` and `validate` (valid; 3 unrelated active stories without scope). I walked the 5 `decomposes` and `depends_on` edges of the set plus the epic's own edge to slice 3, with no cycle and no chain. I traced 4 acceptance reads to a producer in the set and 3 of them are recorded by an edge: the Date story reads the calendar story's functions (edge recorded) and the Math story's functions through its consumer test (edge missing). I also read the decomposition report, the files named in `Writes:`, `crates/num/src/date.rs`, `crates/swell/src/host/hooks.rs`, `crates/swell/tests/host_objects.rs`, the global-name lists in `limits_scripts.rs` and `fuzz.rs`, the three case lists and part of the pinned test262 text.

What I could not establish:
- **Counts and slice-10 hunks:** I did not read `docs/plan/waves.md`, `engine.md` or the slice-7 and slice-10 review results beyond what the stories cite, and I did not check the 133, 364 and 287 counts.
- **Slice-10 conflicts:** the Date story's slice-10 file conflicts (`builtins/mod.rs`, `intrinsics.rs`, `global_object.rs`, `builtin_function.rs`) are single lines added to an enum, table or list, so the join can take both and no edge is needed. This is parallel-safety's lane.
- **Out of my lane, not counted:** the Date story names HOST-187 but not HOST-186, although it adds a kind (`OrdinaryData::Date`); both wave stories may write `builtins_symbol_values.rs` conditionally while the Date story says the stories share no file; `typeof Date(...)` cases such as `S15.9.2.1_A1.js` and `typeof/string.js` are on the list and read the `date_to_string` seam, but they pass legitimately because only the type is checked.
- **Writes lines:** I checked the `OrdinaryData` and `Intrinsic` matches and the global-name tables and found nothing missed.

```findings
- file: .engineering/planning/story/date-object-constructor-now-utc-and-getters.md
  line: 68
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "its consumer test calls the seven Math functions that only story:math-constants-and-exact-functions adds and its acceptance runs builtins_math::, but its only depends_on edge runs to the calendar story, so a unit without the Math story cannot pass its own test; add depends_on story:math-constants-and-exact-functions"
- file: .engineering/planning/story/date-object-constructor-now-utc-and-getters.md
  line: 63
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the clock part (Date function, OrdinaryData::Date, Date.now, new Date(), HOST-125 and HOST-126, the consumer's only Date need) needs no calendar function while the zone part (getters, Date.UTC, multi-argument constructor, HOST-128) does, so one edge gates the consumer's first gap behind the calendar story and 1,350 lines; cut it into a clock story with no edge to the calendar story and a zone story depending on the calendar story and the clock story"
- file: .engineering/planning/story/date-object-constructor-now-utc-and-getters.md
  line: 63
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the constructor and Date.UTC apply MakeFullYear, which crates/num/src/date.rs does not have and the calendar story's list of missing functions does not name, so it has no owner; add make_full_year to the calendar story with its cases or say this story writes and tests it in builtins/date.rs"
- file: .engineering/planning/story/date-object-constructor-now-utc-and-getters.md
  line: 66
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "it fixes the hook call counts and charges (identifier once per conversion, one step per hook call, at most 59 calls for UTC) as settled but they are design precondition 4, still being written, and the story does not name it; name the precondition and say the numbers come from the design sentence once it exists"
- file: .engineering/planning/story/swell-num-calendar-and-local-time.md
  line: 48
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "it names the MakeDay and offset-clamp design sentences as preconditions but writes the 57-probe bound of the tBefore search and its NaN answer as decided behaviour without naming that precondition; name it beside the other two"
- file: .engineering/planning/story/swell-num-calendar-and-local-time.md
  line: 47
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "utc_from_local states what a t refused by date_fields does at the entry but not what a gallop or bisection probe does when t - 2^k falls below -LOCAL_TIME_LIMIT and date_fields refuses it; state the rule (an empty probe, ending in NaN) and add a test at the lower edge"
```
