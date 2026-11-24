---
format: aep.planning-md/3
id: review-result:slice-04-round-2-a-parallel-safety
kind: review-result
status: active
title: 'Slice 4, round 2, the parallel-safety critic: the six stories that start first'
relations:
- reviews: story:math-constants-and-exact-functions
- reviews: story:swell-num-calendar-and-local-time
- reviews: story:date-object-clock-and-time-value
- reviews: story:date-object-constructor-now-utc-and-getters
- reviews: story:number-format
- reviews: story:swell-num-math-double-double
revision: 1
---
needs-revision (panel A)

- `story:math-constants-and-exact-functions`: approve
- `story:swell-num-calendar-and-local-time`: approve
- `story:date-object-clock-and-time-value`: approve
- `story:date-object-constructor-now-utc-and-getters`: approve
- `story:number-format`: needs-revision
- `story:swell-num-math-double-double`: approve

**Findings**

1. `story:number-format`, blocker, a reader beside its producer (cited).
   - Its Acceptance (line 66) requires `slot task libm-check` to exit 0.
   - That task is created by `story:swell-num-math-double-double`, in `taskfiles/libm.yml` (line 56, "Change"). It does not exist on `wave/s10-1`: `grep libm Taskfile.yml taskfiles/*.yml` finds nothing there.
   - `number-format` has only a `decomposes` relation, so a unit on its own base cannot meet the acceptance, or would have to write a stand-in task.
   - Fix: add a `depends_on` edge from `number-format` to `swell-num-math-double-double`, or drop `libm-check` from `number-format`'s acceptance. `number-format` calls no libm function: its f64 operations are `to_bits`, `is_sign_negative` and `abs`.

2. `story:number-format`, warning, an unnamed shared hunk (cited).
   - Line 62 says `crates/swell/src/builtins/number.rs` is shared with `story:number-functions` as "other variants, other arms, other lines of the table".
   - Both stories also rewrite the module header: `number-format` rewrites its "The rest of `Number` is slice 4" sentence (Change); `number-functions` rewrites header lines 15 and 16 (its line 56). The two hunks are adjacent and neither story has an edge to the other.
   - The header hunk is not named in `number-format`.
   - Fix: add an edge from `number-functions` to `number-format` that records the header as the shared hunk, or say which lines of the header each story takes.

3. `story:swell-num-math-double-double`, warning, an unnamed collision (cited).
   - Its `Writes` (line 66) names only the slice-10 story `slow-lane-starts-by-hand` for `Taskfile.yml`.
   - `story:gate-cpu-table-held-by-a-check` (epic foundation, draft) also has `Taskfile.yml` in its scope. The waves output lists the collision (`story:gate-cpu-table-held-by-a-check story:swell-num-math-double-double Taskfile.yml`).
   - Its hunk is the collect-profile name (its line 37), while `swell-num-math-double-double` changes `includes:` and the `check` list.
   - Fix: name that story and the hunk of each in `swell-num-math-double-double`, or record an ordering edge with the file as its reason.

4. `story:swell-num-calendar-and-local-time`, warning, an incomplete statement of shared files (cited).
   - Line 54 says the only shared file is `crates/num/tests/main.rs`, shared with the clock story. Three other stories of the panel also add a module to `crates/num/src/lib.rs` and a `mod` line to the generated `crates/num/tests/main.rs`: `number-format`, `swell-num-math-double-double` and the clock story (`time_value`).
   - `lib.rs` is not written by `calendar`, so the claim holds there.
   - Fix: say that `tests/main.rs` is also shared with `number-format` and `swell-num-math-double-double`.
   - The same gap is in `date-object-clock-and-time-value`, line 87: it names `lib.rs` as "one `pub mod` line" with no partner, and `tests/main.rs` with calendar only.
   - The conflict is one generated line plus an alphabetic insert, so it is cheap at merge. It is a record gap, not a blocker.

5. `story:date-object-clock-and-time-value` and `story:math-constants-and-exact-functions`, warning, a pair that does not name each other (cited, ordered by an edge).
   - Both write `crates/swell/src/object/builtin_function.rs`. The clock story takes the `BuiltinId::Date` variant and its arm; `math-constants` adds a test after `a_loop_of_builtin_calls_collects_within_its_bound`.
   - The clock story depends on `math-constants`, so they never run at once.
   - `math-constants` line 58 lists only slice-10 stories as sharers of this file. The clock story, line 87, lists only the zone, setters and text stories.
   - Add the pair, and `story:math-functions` for `math-constants`, to the sharing sentences.
   - The line numbers in both stories are of the older base: on `wave/s10-1` the test is at line 942, the `Math` variant at 71 and the dispatch at 760. `math.rs` is 86 lines there, not 87. The hunks are anchored by name, so this is not a defect.

**Observations that do not change a verdict**

- Round-1 findings are answered. The clock story now depends on `math-constants`. `builtins_symbol_values.rs` is dropped from every `Writes` line. The slice-10 files are named per story, with the hunk's kind.
- No story of panel A sits beside an open slice-10 story outside the five files you listed. `regexp-match-all-and-escape` is still `active` in the store but is merged on `wave/s10-1`, so `ordinary.rs` is free.
- Outside panel A:
  - `story:math-sum-precise` and `story:math-functions` both write `crates/swell/src/builtins/math.rs` with no edge between them. `math-constants` names both.
  - `number-functions`, which depends on the clock story, also writes `intrinsics.rs` and `global_object.rs`, so those two files are ordered.
- Open points 2 and 4: `number-format` writes the `toLocaleString` choice into the `number.rs` header and names it for the design author. `data/math/SOURCE` is named by `swell-num-math-double-double` as a one-line-each shared file. I found no parallel-safety defect in either.
- Open points 1, 3, 5 and 6 are not in panel A or not in my lane.

What I read: the 6 panel-A stories whole, the scope and edge lines of the 12 panel-B stories, `aep plan artifact waves --kind story --status draft`, round-1 parallel-safety, and the non-implemented stories whose scopes meet panel A's paths. I checked these in `swell-wave-s10-1`: `Taskfile.yml`, `taskfiles/`, `crates/num`, `requirements.txt` (14 pending), `builtin_function.rs`, `math.rs`, `number.rs`, `engine.md` decision 6.

Surfaces: 6 stories placed by a cited path, 0 placed only by inference (several scope entries are marked inferred in the store, all in files the story creates), 0 unplaced. I traced 4 acceptance reads to a producer in the set and `depends_on` records 3: clock→math-constants, zone→calendar, zone→clock. The one missing is `number-format`→`swell-num-math-double-double`, finding 1.

Not established: whether `gate-cpu-table-held-by-a-check` will be built beside slice 4. Outside my lane: the absolute `pending slice 4$` counts (12, 10, 6) in several acceptances hold only in the order the edges give. I did not check them.

```findings
- file: .engineering/planning/story/number-format.md
  line: 66
  category: parallel-safety
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the acceptance requires `slot task libm-check` to exit 0, a task that story:swell-num-math-double-double creates (it does not exist on wave/s10-1) and no depends_on edge orders the two, so add the edge or drop the check from this acceptance (cited)."
- file: .engineering/planning/story/number-format.md
  line: 62
  category: parallel-safety
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the sharing sentence for crates/swell/src/builtins/number.rs with story:number-functions omits the module header, which both stories rewrite at adjacent lines with no edge between them, so name the header hunk of each or add an ordering edge that records it (cited)."
- file: .engineering/planning/story/swell-num-math-double-double.md
  line: 66
  category: parallel-safety
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Writes names only slow-lane-starts-by-hand for Taskfile.yml but story:gate-cpu-table-held-by-a-check also has Taskfile.yml in its scope (the waves output lists the collision), so name it with each story's hunk or record an ordering edge (cited)."
- file: .engineering/planning/story/swell-num-calendar-and-local-time.md
  line: 54
  category: parallel-safety
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "it says the only shared file is crates/num/tests/main.rs with the clock story, but number-format and swell-num-math-double-double also add a mod line to that generated file, so name them (cited, generated file)."
- file: .engineering/planning/story/date-object-clock-and-time-value.md
  line: 87
  category: parallel-safety
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "crates/num/src/lib.rs and crates/num/tests/main.rs are shared with number-format and swell-num-math-double-double as well as calendar, and builtin_function.rs with math-constants (ordered by the edge), but the sharing sentence names none of these (cited)."
- file: .engineering/planning/story/math-constants-and-exact-functions.md
  line: 58
  category: parallel-safety
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the builtin_function.rs sharing sentence lists only slice-10 stories and omits date-object-clock-and-time-value and math-functions, which write that file after this one by edge, and its line numbers (746, 586) are of the older base (942 and 760 on wave/s10-1) (cited)."
```
