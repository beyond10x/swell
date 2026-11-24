---
format: aep.planning-md/3
id: review-result:slice-04-round-1-parallel-safety
kind: review-result
status: active
title: 'Slice 4, round 1, the parallel-safety critic: the three stories of the first wave'
relations:
- reviews: story:math-constants-and-exact-functions
- reviews: story:swell-num-calendar-and-local-time
- reviews: story:date-object-constructor-now-utc-and-getters
revision: 1
---
needs-revision

story:date-object-constructor-now-utc-and-getters — blocker: its consumer test `consumer_references_of_surf_resolve_as_the_standard_says` and the `builtins_math::` filter of its acceptance read the `Math` functions that `story:math-constants-and-exact-functions` creates, yet its only `depends_on` edge runs to the calendar story, so `aep plan artifact waves` puts both in one wave (reader beside producer, cited). The fix is to add a `depends_on` edge from the date story to `story:math-constants-and-exact-functions`, or to merge the two stories. On a unit branch without the math story, `Math.floor`, `min`, `abs` and `ceil` do not exist, so the test cannot pass and no stand-in can be written without writing `math.rs`. — .engineering/planning/story/date-object-constructor-now-utc-and-getters.md:68 and :82; .engineering/planning/story/math-constants-and-exact-functions.md:44 ("the last story of the wave")

story:date-object-constructor-now-utc-and-getters — warning: both this story and the math story list `crates/swell/tests/builtins_symbol_values.rs` in `Writes:` ("only if the sweep needs an entry"), and neither names the other (cited, both bodies). This story also states "Files shared with the other stories of the wave: none". The remedy is either to say the conditional shared file in both bodies or, since the sweep accepts any answer or TypeError and `expected` lists only the eight known exceptions, to drop it from both `Writes:` lines. An ordering edge is not needed if the first remedy is taken. — .engineering/planning/story/date-object-constructor-now-utc-and-getters.md:78 and :69; .engineering/planning/story/math-constants-and-exact-functions.md:55

story:date-object-constructor-now-utc-and-getters — warning: the slice-10 sharing paragraph names the open stories for `builtins/mod.rs`, `intrinsics.rs`, `global_object.rs` and `builtin_function.rs`. It names none for `conformance/host/requirements.txt` (cited by `promise-statics-finally-and-aggregate-error`, `for-await-and-async-from-sync-iterator`, `async-generators-and-their-request-queue`, `array-from-async`), `website/data/status.json` (the same four) or `crates/swell/tests/main.rs` (those four and `async-generator-methods-in-object-literals`). It also gives no hunk for the two likely textual conflicts. These are `mod date;` between `boolean` and `error` in the `mod` list of `builtins/mod.rs`, next to the `async_*` inserts after line 41, and `("Date", Intrinsic::Date)` appended at the end of the global name list in `global_object.rs`, where `promise-statics-finally-and-aggregate-error` also appends. No edge is needed, because the join can take both. The body should say so and name the stories and hunks. The requirement lines 125, 126 and 128 are far from the lines slice 10 moves. — .engineering/planning/story/date-object-constructor-now-utc-and-getters.md:78; crates/swell/src/builtins/mod.rs:41-43; crates/swell/src/realm/global_object.rs:66-80 (cited)

What I read: the three stories, the epic graph and the scopes of the nine open slice-10 stories (`aep plan artifact show`, `aep plan artifact graph`). I also read `docs/plan/slice-04/part.sh`, `check.sh` and the case lists, and the code the `Writes:` lines name. That code is `OrdinaryData` and its matches, the `BuiltinId`, `Intrinsic` and global-name sites, `support/host.rs`, `crates/num` and its tests, the symbol sweep and the HOST-187 test.

Surfaces: 3 stories placed by cited paths, 0 inferred, 0 unplaced. I traced 4 acceptance reads to a producer in the set (the calendar functions, the `Math` functions, `builtins_math::` and the sweep file), and `depends_on` records 1 of them, the calendar edge.

Pairs:
- Math and calendar share no file.
- Calendar and date are ordered by an edge.
- Math and date share no file except the conditional sweep file above.

I checked and found no collision:
- No other `match` over `OrdinaryData` lacks a wildcard (json at `json/mod.rs:418` has one; the class tag at `object/mod.rs:560` is the only arm to add).
- `TestHost` is built in one place only (`support/host.rs`).
- The `Host` trait and `surface.txt` are not touched.
- No test pins the absence of `Date` or of `Math.floor`.
- The test262 list for the date story has no case that mentions `Math.`, and the list for the math story has no case that mentions `Date`.
- The two lists share no case.

Not established, or outside my lane:
- Design lane: the calendar story says "at most 58 steps" for one `UTC ( t )`, while the date story charges the time zone identifier call as well ("at most 59", each charged). The two contracts differ by one step.
- Design lane: the date story states the hook-call counts and the charge per call as settled. It does not name the pending design change on hook-call counts as its precondition.
- Not checked: whether the 12,500-call `Math.max` test at `builtin_function.rs:746` exists as the math story says (acceptance lane).
- Not checked: whether the three-story cut is right (design lane).

```findings
- file: .engineering/planning/story/date-object-constructor-now-utc-and-getters.md
  line: 68
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the consumer test and the builtins_math:: filter of the acceptance read the Math functions that story:math-constants-and-exact-functions creates (its body line 44 calls the date story 'the last story of the wave'), but the only depends_on edge runs to the calendar story, so one wave holds both; add depends_on to the math story or merge the two"
- file: .engineering/planning/story/date-object-constructor-now-utc-and-getters.md
  line: 78
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "this story and story:math-constants-and-exact-functions both list crates/swell/tests/builtins_symbol_values.rs in Writes (only if the sweep needs an entry) and neither names the other; this body says files shared with the other stories of the wave: none; name the shared conditional file in both, or drop it from both since the sweep needs no entry for these functions"
- file: .engineering/planning/story/date-object-constructor-now-utc-and-getters.md
  line: 78
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the slice-10 sharing paragraph names no open story for conformance/host/requirements.txt, website/data/status.json and crates/swell/tests/main.rs and gives no hunk for the likely textual conflicts (mod date; between boolean and error in builtins/mod.rs next to the async_* inserts; the Date entry appended to the global name list in global_object.rs where promise-statics-finally-and-aggregate-error also appends); say that the join takes both and name the stories and hunks"
```
