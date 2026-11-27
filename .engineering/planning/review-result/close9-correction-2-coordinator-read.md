---
format: aep.planning-md/3
id: review-result:close9-correction-2-coordinator-read
kind: review-result
status: active
title: 'Coordinator''s read of corrections 1 and 2 of the unit close9: no defect, check (5) reads the start run for the moves to slice 8'
relations:
- reviews: story:slice-09-conformance-close
revision: 1
---
Coordinator's read of corrections 1 and 2 of the unit close9 (impl/close9, 66fe379 to
6cc4ca7), 2026-11-27, in place of a second adversary pass. Verdict: no defect; one check of
the story's acceptance is read differently for the cases that move to slice 8, and the story
says so now.

The adversary's pass 1 was red (`review-result:adversary-close9-pass-1`): F1, 32 cases of
slice 19 meet the story's rule for coming into slice 9 and stayed out; F2, the 37 cases of
`close-later.tsv` that need slice 8 stayed counted in slice 9; F3, a figure of the design
paragraph ("3,740 with a token of slice 8 only") held 108 cases with a later token too; F4,
older than the unit: the moves check does not see a block line without its line of the list
(the draft `story:moves-check-sees-a-block-move-without-its-line`).

Correction 1 (`37aaed6`, after the merge `acccedd` of `wave/big-2` at `b9adbe3`):

- F1: 31 of the 32 cases come in (130 in all, `docs/plan/slice-09/close-into-09.tsv`). The
  32nd, `test/staging/sm/Array/unscopables.js`, stays in slice 19: `harness/propertyHelper.js`
  uses a part of a later slice there (lines 42 and 236), which is no guard. The unit's own
  scan decided each case; the adversary's list was the start, not the proof.
- F3: the sentence states 107 and says what "only" means.
- The merge conflict of the expected files was taken from the wave's side and the full run
  made again.

Correction 2 (`6cc4ca7`), on the coordinator's order after the plan of slice 8 was read
(`docs/plan/waves.md`, the row of order 5 of slice 9, and `docs/plan/slice-08/incoming.tsv`
plan the 37 as cases the close of slice 9 moves into slice 8):

- F2: the 37 move to slice 8, each with a line of `conformance/test262/moves.tsv`: 36 new
  lines `9 8 part class`, and the line of
  `language/expressions/optional-chaining/new-target-optional-call`, which keeps its `from`
  2 and reads `8 part new.target`. The block gains 36 lines `dir <file> 8` and changes one.
  Each of the 37 failed in every run at the start of the slice on a refusal of slice 8 (74
  runs) and holds the token in its own file.
- The table of section 12: slice 8 9,339 cases and 18,529 runs, slice 9 1,858 and 3,678,
  slice 12 381 and 760, slice 19 1,233 and 2,299; the sums do not move.

Check (5) of the story's acceptance ("every case moved out of slice 9 has a failed run on
the base") prints 36 over the base run and 0 over the start run. The coordinator ran it: the
36 are the 36 lines `9 8` of the list and no other case (`cmp` of the two sorted lists). They
pass on the base because the base holds the code of slice 8 merged so far; the check was
written when the 37 were not planned to pass before the close. The story's check (5) now
reads the start run for the cases that move to slice 8 and the base run for the others. This
is the coordinator's decision; no option of it changes a file of the unit.

The coordinator's own runs on `6cc4ca7`, on the unit's run files: check (5) as above;
`sh docs/plan/slice-08/lists.sh .out/test262/runs.tsv` prints `slice 9339 lists 9339 incoming
0`; `sh docs/plan/slice-09/lists.sh .out/test262/runs.tsv` prints `slice 1858 lists 1787
incoming 0`; no run of slice 9 in the head's run lines is anything but `Passed`; the lines
`test262 total 53801 8982 7 44812 36027 8783 2` and `test262-slice 09 1858 3678 0 0 1858 1858
0 0`. The unit's pool job on that head: `test262-check` exit 0 (22 slice lines equal the
design, 287 lines, the sources digest equal), `test262-moves: 2022 case(s) moved`,
`host-requirements-check` with `9` delivered exit 0. The whole gate runs on the merged wave
branch with the coordinator's files of the close.

Read: the diff `b9adbe35..6cc4ca7` (5 files: the two expected files, `moves.tsv` +60,
`docs/design/engine.md`, `close-into-09.tsv` +130), the paragraph "Slice 9's cases that
need a later slice", the unit's three reports and its notes for the coordinator.

Not covered: no second adversary pass read the 31 cases that came in with correction 1 or
the 37 moves of correction 2; both are held by the moves check (evidence token per line),
by the equality of the 22 slice lines with the design and by the head's run, in which every
run of slice 9 passes. The directory counts of section 12, item 9 (`Set` 381, `Proxy` 308,
`Map` 202) were not brought up to date by the close.
