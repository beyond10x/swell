---
format: aep.planning-md/3
id: story:slice-06-conformance-close
kind: story
status: implemented
title: 'Every run of slice 6 passes: the cases that need a later slice move by rule lines with their evidence'
summary: About 657 failing cases move to slices 4 to 18 by the estimate, each with its evidence in moves.tsv and none failing on a part of slice 6; the design counts, HOST-156's line, the omissions of slice 6 and expected.txt in the same change.
relations:
- decomposes: epic:slice-06-bindings-and-iteration
- depends_on: story:tagged-templates
- depends_on: story:array-from-and-object-from-entries
- depends_on: story:symbol-values-in-core-builtins
- depends_on: story:slice-03-conformance-close
scope:
- confidence: cited
  path: conformance/host/requirements.txt
- confidence: cited
  path: conformance/test262/expected.txt
- confidence: cited
  path: conformance/test262/later-parts.tsv
- confidence: cited
  path: conformance/test262/moves.tsv
- confidence: cited
  path: conformance/test262/sections.txt
- confidence: cited
  path: docs/design/engine.md
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T00:38:43Z", actor: "human:operator", revision: 5, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-22T00:38:43Z", actor: "human:operator", revision: 6, executor: "agent:coordinator"}
- {from: "active", to: "implemented", at: "2026-11-23T02:42:39Z", actor: "human:operator", revision: 7, decided_on: {"recorded":{"test_result":1}}, executor: "agent:coordinator"}
---
## Context

The epic asks every run of slice 6 to pass, and a case that needs a later slice is moved by a rule line of the `swell-slices` block with its reason, in the change that moves both slices' counts, each move with its evidence in `conformance/test262/moves.tsv` (`AGENTS.md`, "A case that cannot pass in its slice"; `docs/design/engine.md`, section 12), as `story:slice-03-conformance-close` does for slice 3. This story makes that move for slice 6 once every story of the slice has landed, and it owns the block, its paragraph and its table for these lines. It also writes the lines of the lists that the stories of the slice would otherwise each edit: `HOST-156` of `conformance/host/requirements.txt` and the omission of slice 6 in `conformance/test262/sections.txt`. It writes no engine file.

Measured by the decomposition on `main` (`6a43e18`; the line `test262-slice 06 4204 7952 0 0 4204 94 4110 0` of `.out/test262/lines.txt`: the epic's 3,701 cases and 7,130 runs predate the moves of slice 2, which added 487), as an estimate before the code. Of the 4,110 failing cases of slice 6 and the 61 failing cases of the six directories `story:slice-03-conformance-close` moves here, the scan of `docs/plan/slice-06/lists-b.awk` (its head gives the command) puts 3,483 on the lists of the stories of the slice (16 of them placed by hand, `docs/plan/slice-06/hand-placed.tsv`, and 21 added to the lists of the first five stories, `docs/plan/slice-06/part-a-additions.tsv`), 2 in `docs/plan/slice-06/cross-story.tsv`, and 657 (1,221 failing runs) on none because their text or harness files use a part of a later slice: `docs/plan/slice-06/move-estimate.tsv` (path, estimated slice, evidence; `awk -F'\t' '{ c[$2]++ } END { for (k in c) print k, c[k] }'` over it): slice 4: 69, 5: 274 (regular expressions and the String methods of slice 5), 7: 92 (generators), 8: 73 (classes), 9: 99 (`Reflect`, `Proxy`, `Map`, `Set`), 10: 11, 11: 2, 12: 20, 13: 12, 18: 5; the evidence of a line is the first text that matched, a guess the measurement replaces. The rest, 29 cases, are on `docs/plan/slice-06/close-remainder.cases.txt`: they need a part the scan does not see, and this story reads each.

`story:slice-03-conformance-close` adds about 224 cases of slice 3 to slice 6 by its estimate (68 of them in the six directories); those outside the six directories are measured here for the first time, from the run lines.

Lists that wait for the slice: `conformance/host/requirements.txt` line 156 reads `HOST-156 pending slice 6`; `story:let-const-and-block-scopes` delivers its test `swell::lexical_declarations::host_156_evaluating_a_let_script_twice_throws_a_syntax_error`. `conformance/test262/sections.txt` line 93 reads `omit sec-arguments-exotic-objects symbol-iterator-property section-12-slice-6`; `story:iterator-protocol-and-array-string-iterators` delivers `@@iterator` of the arguments object. The file has no section of template literals (`grep -c sec-template-literals conformance/test262/sections.txt` prints `0`), so the `RangeError` at `string_length` of a template is not listed as an omission here. `docs/design/engine.md` gives the block functions an omission of Annex B.3.2, written here with the other lines of the file (Change).

Design it builds on: section 12 of `docs/design/engine.md` and its block as `story:slice-03-conformance-close` leaves them; no new design.

## Domain relations

- None new.

## Change

Failing check first: the gate line `test262-slice 06` with failed runs is the failing state; the story ends when the line reads every run passed and the moves check passes.

- Measure with three sets of run lines, each made by the full run (the slow lane, in the background through the pool, about 90 s each with its build): `.out/test262-start/runs.tsv` of the commit slice 6 started from (the base of its first wave, made as `story:slice-03-conformance-close` makes its start runs, from `git archive` of that commit), `.out/test262-base/runs.tsv` of the wave before this change (`slot task test262 TEST262_OUT=.out/test262-base`) and `.out/test262/runs.tsv` after it. From the failed runs of slice 6 in the base runs, each failing case is read with its first failure and its text: a case that needs a later slice moves to that slice with a token of that slice found in the case or a harness file it includes; a case that fails on a part of slice 6 goes back to the story that owns that part, and this story waits for its fix.
- `conformance/test262/later-parts.tsv`: the tokens the moves need and the file lacks, each with its slice and item.
- `conformance/test262/moves.tsv`: one line per case whose slice the change moves, five fields; a case moved earlier keeps its line with its `from` and gets the new `to` and evidence.
- `docs/design/engine.md`, section 12: the rule lines of the block (one per directory when every case of slice 6 in it moves to one slice, one per file otherwise), a paragraph "Slice 6's cases that need a later slice" with each group by target slice, its count and its reason, the counts summing to the lines this story adds or changes in `moves.tsv`; item 6 of "What each slice builds" and the slice's rows of the table and the block after "Its output on this document" with the new counts of every slice the lines touch, the 22 lines summing to the totals they had before the change; the sentence on slice 6 under the table with the command that measured it.
- `conformance/host/requirements.txt` line 156: `HOST-156 test swell::lexical_declarations::host_156_evaluating_a_let_script_twice_throws_a_syntax_error`.
- `conformance/test262/sections.txt`: line 93 (`omit sec-arguments-exotic-objects symbol-iterator-property section-12-slice-6`) removed; and the omission of B.3.2 that `docs/design/engine.md`, "Function declarations in a block" (lines 545 to 609), records for slice 18: the section `sec-block-level-function-declarations-web-legacy-compatibility-semantics` with its three `path` lines (`test/annexB/language/function-code/`, `test/annexB/language/global-code/`, `test/annexB/language/eval-code/`) and the line `omit sec-block-level-function-declarations-web-legacy-compatibility-semantics var-binding-of-block-functions section-12-slice-18`, in the form of the file. This story is the one writer of `sections.txt` in slice 6 (`story:block-function-declarations` hands its omission here, since `story:slice-03-conformance-close` writes the same file and may run beside it).
- `conformance/test262/expected.txt`: the lines of the run, in the same change, with the digest of its sources as the gate asks once `story:gate-test-steps-within-budget` has landed.
- For the coordinator, in the change that closes the slice (this story does not write these files): `6` added to `conformance/slices-delivered.txt` (then `host-requirements-check` fails while a requirement is pending for slice 6), and the epic's counts.
- Time ("Tests stay fast", the `AGENTS.md` of the folder that holds the repository, and the `AGENTS.md` of the repository once `story:gate-test-steps-within-budget` has landed): this story adds no test. The three full runs are the slow lane, each started once, in the background. The unit never runs `slot task check`: the whole gate runs once on the wave branch.

Writes: `docs/design/engine.md` (section 12 as listed), `conformance/test262/moves.tsv`, `conformance/test262/later-parts.tsv`, `conformance/host/requirements.txt` (line 156), `conformance/test262/sections.txt` (line 93 and the section of B.3.2), `conformance/test262/expected.txt`. The only story of its wave.

## Acceptance

`slot task test262` exits 0 and prints `test262: 22 slice lines equal the design` and one line `test262-slice 06 <c> <r> 0 0 <c> <c> 0 0` (every case of slice 6 run and passed; about 3,771 cases by the estimates above, 4,204 + 224 - 657, the exact `<c>` and `<r>` those the run measures and the design table states); `awk '$1 == "test262-slice" { c += $3; r += $4 } END { print c, r }' .out/test262/lines.txt` prints the same two numbers as on the base runs (the slice counts sum as before the move); `slot task -d . -t taskfiles/test262.yml test262-moves-check` exits 0 and its count of moved cases equals the lines of `conformance/test262/moves.tsv`; after `{ cat conformance/slices-delivered.txt; echo 6; } > .out/slices-delivered-6.txt`, `slot task -d . -t taskfiles/host-requirements.yml host-requirements-check HOST_REQ_DELIVERED=.out/slices-delivered-6.txt` exits 0 (no requirement pending for slice 6); `grep -c 'section-12-slice-6' conformance/test262/sections.txt` prints `0` (on `6a43e18` `1`); `grep -c 'pending slice 6$' conformance/host/requirements.txt` prints `0` (on `6a43e18` `1`). With the three sets of run lines (S start, B base, N after): (1) `awk -F'\t' '$2 == "06" && $5 != "Passed"' .out/test262/runs.tsv | wc -l` prints `0` (on `6a43e18` `7766`, the failed runs of slice 6); (2) every case whose slice moves has a line of `moves.tsv`: `awk -F'\t' 'FILENAME == ARGV[1] { b[$3] = $2; next } FILENAME == ARGV[2] { m[$1] = 1; next } ($3 in b) && b[$3] != $2 && !($3 in m) { print $3; bad = 1 } END { exit bad }' .out/test262-base/runs.tsv conformance/test262/moves.tsv .out/test262/runs.tsv` exits 0; (3) no run that passed at the start of the slice fails now: `awk -F'\t' 'NR == FNR { if ($5 == "Passed") p[$3 "\t" $4] = 1; next } (($3 "\t" $4) in p) && $5 != "Passed" { print $3, $4, $6 }' .out/test262-start/runs.tsv .out/test262/runs.tsv` prints nothing; (4) every case of `docs/plan/slice-06/close-remainder.cases.txt` passes or has a line of `moves.tsv`: `awk -F'\t' 'FILENAME == ARGV[1] { w[$1] = 1; next } FILENAME == ARGV[2] { m[$1] = 1; next } ($3 in w) && !($3 in m) && $5 != "Passed" { print $3, $4, $6; bad = 1 } END { exit bad }' docs/plan/slice-06/close-remainder.cases.txt conformance/test262/moves.tsv .out/test262/runs.tsv` exits 0 (on `6a43e18` it exits 1); (5) every case moved out of slice 6 has a failed run on the base and none of its runs fails on a part of slice 6: `awk -F'\t' 'NR == FNR { if ($2 == "06" || $2 == "6") mv[$1] = 1; next } ($3 in mv) { if ($5 != "Passed") failed[$3] = 1; if ($6 ~ /is not supported before slice 6 |Symbol is not defined/) bad[$3] = 1 } END { for (k in mv) if (!(k in failed) || (k in bad)) print k }' conformance/test262/moves.tsv .out/test262-base/runs.tsv | wc -l` prints `0` (the rule "A case that fails on a part of its own slice is never moved"; over the runs of `6a43e18`, with the estimate as the moves, `awk -F'\t' 'BEGIN { OFS = "\t" } { print $1, "6", $2 }' docs/plan/slice-06/move-estimate.tsv` in place of `moves.tsv`, it prints `306`, the cases whose runs still end on a part of slice 6 there); (6) `grep -c '^omit sec-block-level-function-declarations-web-legacy-compatibility-semantics var-binding-of-block-functions section-12-slice-18$' conformance/test262/sections.txt` prints `1` (on `6a43e18` `0`). The report quotes the output of each check.

## Out of scope

Any engine, compiler, parser or runner source file (a failing case of a part of slice 6 goes back to its owner); the epic, `AGENTS.md`, `README.md`, `CHANGELOG.md` and `conformance/slices-delivered.txt` (the coordinator); `taskfiles/test262.yml` (`story:slice-03-conformance-close` changes the moves check; this story uses it as it stands); the design text outside section 12; every line of `conformance/host/requirements.txt` and `conformance/test262/sections.txt` but those named in Change.
