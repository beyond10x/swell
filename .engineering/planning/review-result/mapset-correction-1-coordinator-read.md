---
format: aep.planning-md/3
id: review-result:mapset-correction-1-coordinator-read
kind: review-result
status: active
title: 'Coordinator''s read of correction 1 of the unit mapset: tests only and the merge of the wave branch, no defect'
relations:
- reviews: story:map-and-set
revision: 1
---
Coordinator's read of correction 1 of the unit mapset (impl/mapset, 9cae087a to a2898842),
2026-11-26. Verdict: no defect; the correction changes tests only and merges the wave branch
without a conflict.

The correction answers review-result:adversary-mapset-pass-1, whose two findings are a count
of two tests below the count their requirements state (F1) and step charges that no test of
the unit held (F2). It is two commits.

Commit 5d17ff05 (2 files, 710 added and 40 removed lines, both under `crates/swell/tests/`):

- `builtins_map_and_set.rs`: the adversary's 12 cases under the unit's names; the module runs
  29 tests. F2 is answered by `each_entry_of_next_for_each_and_the_constructor_is_a_step`:
  with the four step charges removed in a copy of the tree (`builtins/map/mod.rs:372`, `:561`,
  `:656`, `builtins/set/mod.rs:194`) it is red (`left: [0, 1000, 3000] right: [1000, 2000,
  4000]`), and green without.
- `determinism.rs`, `ordered_keys` (read line by line): HOST-25 and HOST-214 keep the 10,000
  keys their requirements state. The lane `slow` (`SWELL_FULL_COUNTS=1`) runs 10,000 keys in
  the default mode and 100 with `collect-every-safe-point`; the gate runs 1,000 and 12. The
  comment says what each count catches: an iteration planted to follow the index map made
  HOST-25 red at 1,000 and at 12 keys; HOST-214 compares two processes with the same `Seeds`.
  `task slow-tests` names both modules in both runs; its lines: default `815 passed, 0
  failed`, HOST-25 7.3 s and HOST-214 4.0 s; collect `998 passed, 0 failed`, HOST-25 16.2 s
  and HOST-214 11.4 s.
- Four more counts of the unit's tests that the story's text states with a number got the same
  two counts (gate / lane): abandoned iterators 2,000 and 30 / 10,000 and 200; the throw in
  `forEach` in the collect mode 40 / 100; finished loops in the collect mode 10 / 100; the Set
  of 10,000 keys in the collect mode 100 / 700 (10,000 keys ran into the 60 s bound of a test
  there, 2,000 took 33.6 s). One stays below the story's number in both: the table of the
  cursor tests in the collect mode has 32 keys where the story says 1,000 (27.6 s and 28.3 s
  at 1,000). The story's "As delivered" section takes these at the wave's close.

Commit a2898842: the merge of `wave/big-2` at 66882ac0 (the units classsyntax and super). No
conflict; `tests/main.rs` merged on its own; no count a test holds moved.

On the merged head the implementor ran: swell main `1236 passed, 1 skipped` in both modes, the
library's tests 250 and 246, the story's part of test262 `passed 532 bound 532 B 156 F 376 R
0`, `task test-realm-sizes` green at all seven sizes in both modes (`realm-sizes: 0 run(s)
red`), clippy and fmt clean.

Open after the read, for the close: after the Set's lane count went from 1,000 to 700 the
whole lane was not run again (the wave's close runs `task slow` whole). Two tests of the
module took 5.7 s and 6.5 s in the whole collect run under load and under 5 s alone.

Not read: the 12 cases line by line; the read took their names, the renamed helpers and the
step test's assertion.
