---
format: aep.planning-md/3
id: review-result:realmwalk-plan-coordinator-read
kind: review-result
status: active
title: 'Coordinator''s read of the realm-walk story after the plan critics'' second read: ready for its unit'
relations:
- reviews: story:realm-walk-in-parts-under-the-cpu-bound
revision: 1
---
Coordinator's read of the plan of `story:realm-walk-in-parts-under-the-cpu-bound` after the
second read of its three plan critics, 2026-11-27. Verdict: the story is ready for its unit.

The story was written again after its first unit stopped at the story's own stop rule. Three
plan critics (acceptance, design, parallel safety) read revision 5 and each asked for a
revision; they read revision 8 and each asked for one more. The budget of two reads is spent;
the coordinator decided each remaining finding, had the text changed, and read the whole
change of the body against revision 8 (241 changed lines).

What the second read found and what the text says now:

- Acceptance (1 blocker, 5 notes; all 12 findings of its first read closed). Blocker: the
  writer removed no output of an earlier run and did not fail on a failed nextest run, so the
  plant "the write taken out of the test" and the second writer run could pass on a stale file.
  Now: the writer removes `.out/realm-places/walk.txt` before the run and exits 1 with its own
  line when nextest fails. Notes, each written in: the include carries `flatten: true` and
  `optional: true`; the plant at 3 parts sets `WALK_PARTS` to 3 for any `n` of the base; the
  doc-comment check defines its hash and requires a line in each of the two comments; a plant
  for the empty-variable exit; the order of the acceptance items that make and read
  `first.txt`; two letters for two meanings of one placeholder; the output of `od` as it
  prints; the least number of places as a constant the message is built from; the hash over
  the line without its line end.
- Design (2 blockers, 7 findings and notes; all 11 findings of its first read closed).
  Blocker 1: the list of the uses of the feature `collect-every-safe-point` under
  `crates/swell/src` lived only in this story, which closes. Now: the eight lines are in the
  sentence the story writes for `AGENTS.md`; a unit whose grep differs reports the line, and
  the coordinator updates the list at the merge. Blocker 2: a top-level `vars:` block of
  `Taskfile.yml`, which a unit of the lane `slow` may add too, merges into a duplicate key.
  Now: no such block; the two variables are defined in `taskfiles/realm-places.yml`, as
  `taskfiles/collect-mode.yml` defines its own, and the steps `test` and `build-tests` read
  them after the flatten and fail with one line when either is empty. With `build-tests`
  reading them, the claim that the writer and the gate share one build holds. Also written
  in: a unit over the rule adds parts until the rule holds and measures every part again; a
  conflict in `walk_parts!` resolves to the larger number; a hash gives no balance, the
  slowest part sets the number of parts, and the cover test bounds a part at twice the
  ceiling of the mean (on `158888d` the largest part is at most 1.17 times it; at 2 parts the
  bound is the whole file and cannot fail); the fallback's sources name the walk's own code;
  the `cfg` of the collect build covers every `use` only the walk needs; the coordinator runs
  the awk of the acceptance again on a merged tree.
- Parallel safety (1 blocker, 3 notes; its first findings closed, two of them in the store's
  front matter). Blocker: the story did not state that it changes the number of tests of
  `swell::main`, which a unit of the lane may hold as a count. Now: with `n` parts the binary
  has `n` + 1 tests more in the default build and `n` more in the collect build, and at a join
  with a unit that writes `Taskfile.yml` or `taskfiles/` the coordinator reads those counts
  again. The notes are covered by the form of the variables above.

Decided by the coordinator against a finding:

- The hash keeps the form the probe measured (FNV-1a over the line, the 64-bit value modulo
  the number of parts). The design critic asked to decide on the high bits; that form would
  need a new probe and changes every recorded reading, and the measured parts are within 1.17
  times the mean at every number of parts from 2 to 8.
- One sentence of the revision said the slowest part is not monotone in the number of parts
  and gave two readings that fall (0.916 at 2 parts, 0.656 at 3). The coordinator replaced it:
  the slowest part need not fall with each added part, because a hash gives no balance.

The form of the committed file (no digest; one test in the default build makes the list again
at every gate run) was put to the operator with three options; no answer came within the 15
minutes of the folder's rule "A default proceeds", and the default was taken. The wave page
records it.

Not settled by reading, left to the unit's first run: whether go-task passes an empty
variable through `env:` so that the check in `build-tests` and `test` sees it; the cost of the
equality test as one test (about 0.07 CPU s, added up from parts of the probe). The skew
assertion and the writer's exit on a failed nextest run have no planted defect. The step
`slow-tests` keeps its own copy of the nextest arguments, because it is the lane's.
