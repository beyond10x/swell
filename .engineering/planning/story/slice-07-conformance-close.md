---
format: aep.planning-md/3
id: story:slice-07-conformance-close
kind: story
status: draft
title: 'Every run of slice 7 passes: the cases that need class or super move to slice 8, and passing cases of later slices come in by rule lines'
summary: 40 cases (80 runs) move to slice 8 with evidence in moves.tsv, the 2 cases that need yield* in a generator method are checked, cases of slices delivered after 7 that pass come in by dir lines or keep their slice; section 12 and expected.txt.
relations:
- decomposes: epic:slice-07-generators
- depends_on: story:yield-star-delegation
- depends_on: story:generator-methods-in-object-literals
- depends_on: story:generator-function-constructor
scope:
- confidence: cited
  path: conformance/test262/expected.txt
- confidence: cited
  path: conformance/test262/later-parts.tsv
- confidence: cited
  path: conformance/test262/moves.tsv
- confidence: cited
  path: docs/design/engine.md
- confidence: cited
  path: docs/plan/slice-07/close-into-7.tsv
revision: 3
---
## Context

The epic asks every run of slice 7 to pass, and a case that needs a later slice is moved by a rule line of the `swell-slices` block with its reason, in the change that moves both slices' counts, each move with its evidence in `conformance/test262/moves.tsv` (`AGENTS.md`, "A case that cannot pass in its slice"; `docs/design/engine.md`, section 12), as `story:slice-05-conformance-close` does for slice 5. This story makes that move for slice 7 once every other story of the slice has landed, owns the block, its paragraph and its table for these lines, and writes no engine file.

Measured by the decomposition on `4971b9b` (the line `test262-slice 07 1491 2841 0 0 1491 6 1485 0`), as an estimate before the code. Of the 1,485 failing cases of slice 7, the scan of `docs/plan/slice-07/scan.awk` (its head gives the commands; its assignment is `docs/plan/slice-07/scan.tsv`) puts 1,443 on the lists of the four other stories (115 + 978 + 49 + 280 + 21) and 42 on none: `docs/plan/slice-07/close-later-8.cases.txt`, 40 cases (80 runs) whose text uses `class` or `super`, which slice 8 brings (evidence in field 3 of `scan.tsv`: the refusal `class is not supported before slice 8` as the first failure 22, `class {` 12, `class f {` 2, `super` 4; by directory `language/arguments-object` 20, the `cls-*-gen-meth-*` cases, `language/expressions/object` 8, `language/statements/generators` 4, `language/expressions/generators` 4, `language/statements/switch` 2, `language/block-scope/syntax` 2); and `docs/plan/slice-07/close-cross.cases.txt`, 2 cases (4 runs, `language/expressions/object/method-definition/yield-star-{after,before}-newline.js`) that need both `yield*` and a generator method, a part of slice 7 that only the join of `story:yield-star-delegation` and `story:generator-methods-in-object-literals` gives, so this story shows they pass (check (6)). The evidence of a line is the first text that matched, a guess the measurement replaces.

Cases of slices delivered after 7 (the order `1,2,3,6,5,7,10,4,9,8,...`): on `4971b9b`, 1,054 cases counted in such slices end every failing run on `generator function is not supported before slice 7` (slice 8: 533, 10: 242, 15: 175, 19: 55, 9: 33, 11: 6, 18: 5, 13: 4, 4: 1). Section 12, "A case that comes into a slice from one delivered later" (added for slice 5), decides which of them change slice: a case that passes in every run on the code of slice 7 and whose `feature` and `flag` lines name no slice delivered after 7 gets a rule line `dir <file> 7`, or `dir <directory>/ 7` when every case of the directory is one, longer than the line that gave it its slice, and no line of `moves.tsv`; a case that passes but whose `feature` or `flag` line names a later slice cannot be taken by a `dir` line (rule 1 of that paragraph) and gets no line, staying counted, passed, in its slice. A case counted in a slice delivered before 7 (1, 2, 3, 6, 5) gets no line: a delivered slice takes no case this way and keeps its counts. Which cases those are is read from the full run after the wave, not estimated here.

Lists that wait for the close: `conformance/test262/sections.txt` maps `sec-generatorfunction-objects` to `test/built-ins/GeneratorFunction/` (lines 64 and 65) with no omission, and no omission names slice 7, so no line of it is expected to change; a case of that section that fails at the close goes back to `story:generator-function-constructor`. `conformance/host/requirements.txt` has no line pending for slice 7 after `story:generator-functions-and-suspended-frames`.

Design it builds on: section 12 of `docs/design/engine.md` and its block as the close of slice 5 leaves them ("the longest matching prefix wins"; "a case is counted in the slice delivered last among the slices of its lines"; "A case that comes into a slice from one delivered later", its rules 1 to 4); the table of slices (row 7 at line 1877 on `c0284ed`: slice 7, generators, 1,491 cases, 2,841 runs) and the block after "Its output on this document:".

## What slice 10 reuses

Nothing of the engine. The counts of slice 10 in the table move by the cases this close takes from it.

## Domain relations

- None new.

## Change

Failing check first: the gate line `test262-slice 07` with failed runs is the failing state; the story ends when the line reads every run passed and the moves check passes.

- Measure with three sets of run lines, each made by the full run (the slow lane, in the background through the pool, `slot --timeout 30m`): `.out/test262-start/runs.tsv` of the commit slice 7 started from (`4971b9b`, made from `git archive` of that commit as the close of slice 6 made its start runs), `.out/test262-base/runs.tsv` of the wave before this change (`slot --timeout 30m task test262 TEST262_OUT=.out/test262-base`) and `.out/test262/runs.tsv` after it. From the failed runs of slice 7 in the base runs, each failing case is read with its first failure and its text: a case that needs a later slice moves to that slice with a token of that slice found in the case or a harness file it includes; a case that fails on a part of slice 7 goes back to the story that owns that part (the list it is on), and this story waits for its fix. From the runs of slices delivered after 7, each case that passes in every run is sorted by the rule of section 12 named in Context into `docs/plan/slice-07/close-into-7.tsv` (path, the slice it leaves, the line that takes it) or no line.
- `conformance/test262/later-parts.tsv`: the tokens the moves need and the file lacks (`class` and `super` are there on `c0284ed`, lines 310 and 311), each with its slice and item.
- `conformance/test262/moves.tsv`: one line per case whose slice the change moves to a later slice (about 40, from 7 to 8, kind `part`, the token its evidence), five fields, sorted by path.
- `docs/design/engine.md`, section 12: the rule lines of the block (one per directory when every case of slice 7 in it moves to one slice, one per file otherwise; the 20 `cls-*-gen-meth-*` files of `test/language/arguments-object/` each by its file, as the directory holds cases of other slices), and the lines of `close-into-7.tsv`; a paragraph "Slice 7's cases that need a later slice" with each group by target slice, its count and its reason, the counts summing to the lines this story adds to `moves.tsv`, and, by the rule named in Context, the lines that take cases into slice 7 with their count by directory and the slice each case leaves, and the sentence that these have no line of `moves.tsv`; the rows of slice 7, of slice 8 and of every slice that loses a case to 7 in the table and in the block after "Its output on this document", the lines summing to the totals they had before the change; the sentence on slice 7 under the table with the command that measured it.
- `conformance/test262/expected.txt`: the lines of the run, in the same change, with the digest of its sources as the gate asks.
- For the coordinator, in the change that closes the slice (this story does not write these files): `website/data/status.json`, written again by `task site-data` (it reads `moves.tsv`, section 12 and `expected.txt`, which this story changes); `7` added to `conformance/slices-delivered.txt`; the epic's counts; `AGENTS.md`, `README.md` and `CHANGELOG.md` with the gate's numbers.
- Time ("Tests stay fast", the `AGENTS.md` of the folder that holds the repository, and `AGENTS.md`, "Commands and the gate"): this story adds no test. The three full runs are the slow lane, each started once, in the background, with `slot --timeout 30m`; a pool job ends after 10 minutes unless it names its time, and a script that measures or waits runs without the `slot` prefix. The unit never runs `slot task check`: the whole gate runs once on the wave branch. No Python in any command; no `cd ... &&` with a write or a delete; intermediate files in the scratch directory.

Writes: `docs/design/engine.md` (section 12 as listed), `conformance/test262/moves.tsv`, `conformance/test262/later-parts.tsv`, `conformance/test262/expected.txt`, `docs/plan/slice-07/close-into-7.tsv` (new). The only story of its wave. Not written: `conformance/test262/sections.txt` (no line expected to change; if the run shows one, the unit reports it before it writes), `website/data/status.json` (the coordinator's file: `task site-data` reads `moves.tsv`, section 12 of `engine.md` and `expected.txt`, `taskfiles/site.yml` lines 24 to 35, which this story changes, so the coordinator runs `task site-data` and commits `status.json` in the change that merges this story), every source file.

## Acceptance

`slot --timeout 30m task test262` exits 0 and prints `test262: <n> slice lines equal the design` with no other complaint and one line `test262-slice 07 <c> <r> 0 0 <c> <c> 0 0` (every case of slice 7 run and passed; about 1,451 cases and 2,761 runs before the cases that come in, 1,491 − 40 and 2,841 − 80, the exact `<c>` and `<r>` those the run measures and the design table states); `awk '$1 == "test262-slice" { c += $3; r += $4 } END { print c, r }' .out/test262/lines.txt` prints the same two numbers as on the base runs; `slot task -d . -t taskfiles/test262.yml test262-moves-check` exits 0 and its count of moved cases equals the lines of `conformance/test262/moves.tsv`; after `{ cat conformance/slices-delivered.txt; echo 7; } > .out/slices-delivered-7.txt`, `slot task -d . -t taskfiles/host-requirements.yml host-requirements-check HOST_REQ_DELIVERED=.out/slices-delivered-7.txt` exits 0. With the three sets of run lines (S start, B base, N after): (1) `awk -F'\t' '$2 == "07" && $5 != "Passed"' .out/test262/runs.tsv | wc -l` prints `0`; (2) every case whose slice moves to a later one has a line of `moves.tsv`: `awk -F'\t' 'FILENAME == ARGV[1] { b[$3] = $2; next } FILENAME == ARGV[2] { m[$1] = 1; next } ($3 in b) && b[$3] == "07" && $2 != "07" && !($3 in m) { print $3; bad = 1 } END { exit bad }' .out/test262-base/runs.tsv conformance/test262/moves.tsv .out/test262/runs.tsv` exits 0; (3) no run that passed at the start of the slice fails now: `awk -F'\t' 'NR == FNR { if ($5 == "Passed") p[$3 "\t" $4] = 1; next } (($3 "\t" $4) in p) && $5 != "Passed" { print $3, $4, $6 }' .out/test262-start/runs.tsv .out/test262/runs.tsv` prints nothing; (4) every case that comes into slice 7 by a line of the block is counted there and passes in every run: `awk -F'\t' 'NR == FNR { w[$1] = 1; next } ($3 in w) && ($2 != "07" || $5 != "Passed") { print $3, $2, $5; bad = 1 } END { exit bad }' docs/plan/slice-07/close-into-7.tsv .out/test262/runs.tsv` exits 0, and no case of a slice delivered after 7 that passes in every run lacks a line unless a `feature` or `flag` line of a later slice holds it (the report lists those, with the line that holds each); (5) every case moved out of slice 7 has a failed run on the base and none of its runs fails on a part of slice 7: `awk -F'\t' 'NR == FNR { if ($2 == "07" || $2 == "7") mv[$1] = 1; next } ($3 in mv) { if ($5 != "Passed") failed[$3] = 1; if ($6 ~ /is not supported before slice 7 /) bad[$3] = 1 } END { for (k in mv) if (!(k in failed) || (k in bad)) print k }' conformance/test262/moves.tsv .out/test262-base/runs.tsv | wc -l` prints `0`; (6) the two cases that need `yield*` in a generator method pass: `awk -F'\t' 'NR == FNR { w[$1] = 1; next } ($3 in w) && ($2 != "07" || $5 != "Passed") { print $3, $2, $5; bad = 1 } END { exit bad }' docs/plan/slice-07/close-cross.cases.txt .out/test262/runs.tsv` exits 0 (on `4971b9b` it exits 1 and prints 4 runs). The report quotes the output of each check.

## Out of scope

Any engine, compiler, parser or runner source file (a failing case of a part of slice 7 goes back to its owner); `class`, `super` and generator methods of classes (slice 8); async functions and generators (slice 10); the epic, `AGENTS.md`, `README.md`, `CHANGELOG.md` and `conformance/slices-delivered.txt` (the coordinator); `taskfiles/test262.yml` (the moves check as it stands); the design text outside section 12.
