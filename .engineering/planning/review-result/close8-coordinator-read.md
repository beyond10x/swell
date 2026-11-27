---
format: aep.planning-md/3
id: review-result:close8-coordinator-read
kind: review-result
status: active
title: 'Coordinator''s read of the unit close8 after pass 1: no defect, one sentence fixed with the delivery'
relations:
- reviews: story:slice-08-conformance-close
revision: 1
---
The coordinator's read of the unit `close8` (`impl/close8`, one commit `d61ed17b` on
`2b074619`) after pass 1 of the adversary (`review-result:adversary-close8-pass-1`, green,
six notes). No second pass: the first found no defect, and each note is decided here.

## What was read

- The unit's report and its commit: 5 files, `conformance/test262/moves.tsv` (+9),
  `docs/design/engine.md` (134 block lines, the paragraph, the rows of slices 8, 13, 18 and
  19), `docs/plan/slice-08/close-into-08.tsv` (148 lines), `conformance/test262/expected.txt`
  and `expected.digest`. No file under `crates/`, no `Cargo.lock`, no `Cargo.toml`.
- The story's checks, run again by the coordinator over the three sets of run lines (awk
  only): each `lines.txt` sums to `53801 103360`; (1) prints `0` (`18` on the base); (2)
  exits 0; (3) prints nothing; (4) exits 0; (5) prints `0`; (6) exits 0; (7) `slice 9339
  lists 9339 incoming 0` over the base, the line check of `close-later.tsv` exits 0, the
  groups read `13 5`, `18 4`, `all 9`. Between the base and the head no run changes its
  outcome; the changes of slice are 10 runs from 8 to 13, 8 from 8 to 18 and 292 from 19 to
  8. `lines.txt` of the head's run equals the committed `expected.txt`.
- The slice line of the head: `test262-slice 08 9478 18803 0 0 9478 9478 0 0`; the whole
  run `test262 total 53801 8982 7 44812 37097 7713 2`, as on the base.

## The readings the unit made

- `language/statements/class/subclass/builtins.js` moves to slice 13 on the token
  `Uint8Array`; the story named `ArrayBuffer` and `DataView` for the five. The adversary
  read the case: it never names `ArrayBuffer`. The token stands.
- `harness/propertyHelper.js` is read as a file that names later slices only behind a
  guard: its `Uint8Array.prototype` follows `typeof Uint8Array !== "undefined" &&`, and its
  `BigInt(0)` stands in the `case "bigint"` of a `switch (typeof ...)`. The adversary read
  the same lines and found the same 148 of 155 candidates with a scan of its own. The
  reading is wider than the paragraph of slice 9's close, which kept
  `staging/sm/Array/unscopables` in slice 19; that case passed at the start of slice 8, so
  it is no case of this close, and the delivered count of slice 9 does not move. The
  paragraph of slice 8 says so. The reading stands.
- `staging/sm/Array/species` failed at the start on `Proxy`, not on a part of slice 8; the
  code it evaluates builds `class ... extends Array`. It comes in, and the paragraph names
  it. The reading stands.

## The six notes

1. `sh docs/plan/slice-08/lists.sh` over the head's run exits 1 (`slice 9478 lists 9339
   incoming 0`): the 148 cases that came in are on no list and the 9 that moved are still
   on `close-later.tsv`. No change: the lists are the plan's partition of the slice as
   planned, check (7) reads the base run and holds, and the lists have no reader after the
   close. The wave page says that the lists describe the slice before its close.
2. The paragraph said every case of `test/staging/sm/PrivateName/` comes in; one of its 17
   files, `modify-non-extensible.js`, carries the excluded feature
   `nonextensible-applies-to-private`, is not run and stays counted under the stage-3
   exclusions. Fixed by the coordinator in the commit that delivers the slice: the sentence
   names the 16 cases that run and the one that is excluded. No count and no line of the
   block moves, so the digest of the expected file stays.
3. A line taken out of `moves.tsv` with its block line kept passes the moves check. Older
   than the unit; `story:moves-check-sees-a-block-move-without-its-line` (backlog) holds
   it. The unit compared its seven block lines with the list itself: 9 of 9 cases.
4. A wrong field 2 (the slice a case leaves) of a line of `moves.tsv` passes every check;
   only the groups line shows it. Older than the unit; added to the same backlog story.
5. Text that names a refusal "before slice 8" is left in tests: a branch of
   `crates/syntax/tests/sources.rs` that no shape reaches, and four comments. No source
   file of a crate holds such a refusal (the story's grep prints `0`). Older than the unit,
   which writes no file of a crate; filed as a backlog draft.
6. `staging/sm/fields/await-identifier-script.js` (slice 19) expects `x = await;` in a
   class field of an async function to read a variable; the grammar's `Initializer[+In,
   ?Yield, ?Await]` makes it an `await` without an operand, and the parser's comment says
   so. No change; the case stays in slice 19.

## Outcome

No defect of the unit. Pass 1: one sentence fixed (note 2). The read: no change of the
unit's commit. The gate runs on the wave branch after the merge with `8` in the list of
delivered slices.
