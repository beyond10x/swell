---
format: aep.planning-md/3
id: review-result:slice-04-round-3-parallel-safety
kind: review-result
status: active
title: 'Slice 4, round 3, the parallel-safety critic: the 17 stories'
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
1. needs-revision

2. Per story
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
- `math-random`: needs-revision
- `slice-04-conformance-close`: approve

3. Round-2 blockers in my lane
- `number-format` and `libm-check`: answered. The acceptance no longer runs it. "Rules every story carry" says the story calls no `swell_num::math` function and keeps its own `grep`. `math-sum-precise`, `math-random` and the close do the same.
- `date-text-formats` and the absolute pending count: answered. Requirements and Acceptance now say "lower by 1 than on the unit's base" and name `math-functions` and `math-random` as the other movers.
- Round-2 warnings: I checked these and they are answered.
  - `swell-num-math-double-double` names `story:gate-cpu-table-held-by-a-check` and the hunk of each for `Taskfile.yml`.
  - The sharing sentences for `crates/num/src/lib.rs`, `crates/num/tests/main.rs` and `builtin_function.rs` name their partners.
  - The scopes of `exp-and-log` and `trigonometry` list `crates/num/src/math.rs` and `data/math/SOURCE`.
  - The zone story, `date-setters` and `date-text-formats` list `date_clock.rs` with the test each rewrites. The sequence (the zone story leaves 15 interim bodies, `date-setters` leaves 1, `date-text-formats` deletes the test) is consistent.
- No story of the 17 names `spec-scenarios-for-the-numbers-and-dates-requirements`, and no edge of the 17 points at it. The close depends on the other 16 and on the slice 10 close.

Surfaces: 17 cited, 0 inferred, 0 unplaced. I traced 41 acceptance reads to a producer in the set, and 41 are recorded by a `depends_on` edge. The `waves` tool's collision lines (121) are all pairs that a body names, or generated files (`tests/main.rs`, `status.json`). All four start gates are `grep -c` commands that print 0 now and 1 once the sentence is written, except the `HOST-147` split, which blocks (finding 1).

4. New findings
- `math-random`, lines 67 to 77, blocker. The split of `HOST-147` adds `HOST-214` to `host-interface.md`.
  - `host-requirements-check` fails on "a requirement with no line" until `conformance/host/requirements.txt` holds `HOST-214 pending slice 9`.
  - No story or design prerequisite writes that line. The story's Writes lists only `HOST-26`, `27`, `144`, `147`, `212` and `213`, and the start gate checks only the design.
  - Once the design author lands the split, "host-requirements-check exits 0 before" fails on the base of every story that runs it, the first-wave clock and `math-constants` included.
  - Fix: make the line part of the design author's change, with a gate (`grep -c '^HOST-214 pending slice 9$' conformance/host/requirements.txt` prints 1). Or put it in `math-random`'s Writes and keep the split out of the base until that story runs. This is a missing owner, not a choice of order.
- `number-functions`, Writes, warning. It reads "the header sentence is the one hunk both change: )". The sentence is missing. The Change section never gives the words this story strikes, and `number-format` names them. Paste the same sentence as `number-format`.
- `swell-num-math-exp-and-log` and `swell-num-math-trigonometry`, Context line 40 in each, warning. The prerequisite quote after "The sentence:" is empty. Both say the unit follows it "as written". Only `swell-num-math-double-double` holds the text. Paste it, or point to that story by name. `math-data-check` and the gate depend on the file names in it.
- `date-object-clock-and-time-value`, line 90, note. `intrinsics.rs` and `global_object.rs` are also written by `number-functions` (edge from `number-functions` to this story, so the order is fixed). The sharing sentence says `global_object.rs` is no longer shared.
- `swell-num-date-text`, line 50, and the other stories that add a module to `swell-num`, note. The header comment of `crates/num/src/lib.rs` is one two-line sentence, "Slice 4 adds `radix.rs` ... `math.rs` and `random.rs`". Eight stories edit that one hunk, not "one line at the end of the list". It names `radix.rs`, which no story creates (`number-functions` creates `parse.rs`). Name the sentence once as the shared hunk.
- `math-functions`, line 63, note. The scope lacks `website/data/status.json`, though its Writes names it. `date-text-formats` and the zone story name `math-functions` and `math-random` for `status.json`, but not `math-sum-precise`. The file is generated, so the join is cheap.
- `math-constants-and-exact-functions` line 45, `math-sum-precise` line 63 and `math-functions` line 53, note. They give three different positions for `sumPrecise` and `random` in the `Math` table. The own-keys test reads the table, so nothing breaks. Say one order.
- Scheduling, note. `waves` puts the clock story in wave 1 and `math-constants-and-exact-functions` in wave 2, only because both name `builtin_function.rs`. Both bodies state different hunks and no edge. Neither has a start gate. The operator can run them together.

```findings
- file: .engineering/planning/story/math-random.md
  line: 67
  category: parallel-safety
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "The HOST-147 split adds HOST-214 to the design but nothing writes the line `HOST-214 pending slice 9` in conformance/host/requirements.txt, so host-requirements-check fails on the base of every story that runs it once the split lands; name an owner and a start gate for that line (cited)."
- file: .engineering/planning/story/number-functions.md
  line: 70
  category: parallel-safety
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "The sentence naming the number.rs header hunk shared with number-format ends at 'both change: )' with its content missing, so the words this story strikes are stated only in the other story; paste the same sentence in both (cited)."
- file: .engineering/planning/story/swell-num-math-exp-and-log.md
  line: 40
  category: parallel-safety
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "The prerequisite quote for the reference data of Math is an empty blockquote although the unit is told to follow it as written; paste the sentence or point to swell-num-math-double-double by name (cited)."
- file: .engineering/planning/story/swell-num-math-trigonometry.md
  line: 40
  category: parallel-safety
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "The prerequisite quote for the reference data of Math is an empty blockquote although the unit is told to follow it as written; paste the sentence or point to swell-num-math-double-double by name (cited)."
- file: .engineering/planning/story/date-object-clock-and-time-value.md
  line: 90
  category: parallel-safety
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "intrinsics.rs and global_object.rs are also written by number-functions (ordered by its edge to this story) but the sharing sentence says global_object.rs is no longer shared (cited)."
- file: .engineering/planning/story/swell-num-date-text.md
  line: 50
  category: parallel-safety
  severity: note
  verdict: NEEDS-CHANGE
  origin: pre-existing
  message: "The lib.rs header comment is one two-line sentence that eight stories edit, described here as one line each at the end of a list, and it names radix.rs which no story creates; name it once as the shared hunk (cited)."
- file: .engineering/planning/story/math-functions.md
  line: 63
  category: parallel-safety
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Writes names website/data/status.json (generated) but the scope lacks it, and date-text-formats and the zone story do not name math-sum-precise for that file (cited)."
- file: .engineering/planning/story/math-sum-precise.md
  line: 63
  category: parallel-safety
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Three stories state three different table positions for sumPrecise and random in Math (last two, after max, last); the own-keys test reads the table so nothing breaks, but say one order (cited)."
```
