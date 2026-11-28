---
format: aep.planning-md/3
id: story:slice-11-conformance-close
kind: story
status: draft
title: 'Every run of slice 11 passes: the cases that need a later slice move by rule lines, and passing cases of later slices come in'
summary: 'The close of slice 11: the full run, the moves with evidence in moves.tsv, the 2 cases of close-joint.cases.txt, cases of later slices that pass come in by dir lines; section 12 and expected.txt; the two stale slice-11 comments of Map and Set. Order 7.'
relations:
- decomposes: epic:slice-11-modules
- depends_on: story:dynamic-import
- depends_on: story:ess-target-module-commands
- depends_on: story:module-determinism-and-emulated-loading
scope:
- confidence: cited
  path: conformance/test262/expected.digest
- confidence: cited
  path: conformance/test262/expected.txt
- confidence: cited
  path: conformance/test262/later-parts.tsv
- confidence: cited
  path: conformance/test262/moves.tsv
- confidence: cited
  path: crates/swell/src/builtins/map/mod.rs
- confidence: cited
  path: crates/swell/src/builtins/set/mod.rs
- confidence: cited
  path: docs/design/engine.md
- confidence: inferred
  path: docs/plan/slice-11/close-into-11.tsv
- confidence: cited
  path: docs/plan/slice-11/close-later.tsv
revision: 3
---
## Context

The epic asks every run of slice 11 to pass, and a case that needs a later slice is moved by a rule line of the `swell-slices` block with its reason, in the change that moves both slices' counts, each move with its evidence in `conformance/test262/moves.tsv` (`AGENTS.md`, "A case that cannot pass in its slice"; `docs/design/engine.md`, section 12), as `story:slice-10-conformance-close`, `story:slice-09-conformance-close` and `story:slice-08-conformance-close` do. This story makes that move for slice 11 once every other story of the slice has landed, owns the block, its paragraph and its table for these lines, and writes no engine file.

Measured by the decomposition on the base run (the last full run on `f962c324`; `test262-slice 11 1281 1833 0 0 1281 23 1258 0` in `conformance/test262/expected.txt`): the 1,281 cases of slice 11 are on the lists of `docs/plan/slice-11/` (`sh docs/plan/slice-11/lists.sh .out/test262-base/runs.tsv` prints `slice 1281 lists 1281 incoming 0` and exits 0): 1,279 on the lists of nine stories (2 + 152 + 193 + 122 + 70 + 240 + 6 + 10 + 484; `story:module-code-compiles`, `story:ess-target-module-commands` and `story:module-determinism-and-emulated-loading` have none) and 2 on `close-joint.cases.txt`, which this story checks: `test/language/import/import-attributes/json-via-namespace.js` (a JSON module imported as a namespace: it needs `story:json-modules` and `story:module-namespace-objects`, which no one story's closure holds) and `test/language/module-code/top-level-await/new-await-script-code.js` (no part of the slice: a Script that uses `await` as an identifier; it passes on the base). `close-later.tsv` is empty: the scan (`docs/plan/slice-11/scan.awk`) finds no case of slice 11 with a token or a refusal of a slice delivered after 11 (BigInt, binary data, `Atomics`, iterator helpers, `WeakRef`, the Annex B accessors). The 4 cases moved into slice 11 by earlier closes (`moves.tsv`: `assignmenttargettype/direct-import.meta.js`, `parenthesized-import.meta.js`, `global-code/export.js` and `import.js`, from slice 2) are on the lists of order 1 and 2.

Two entries of `docs/plan/waves.md`, "What later slices take over", reach this slice. "Slices 2, 9 and 11: a requirement for a stale or foreign handle returned by the host at the return sites of that slice" is `story:import-meta`'s (HOST-224). "Each slice: the slice number in a refusal message of `swell-syntax` is the reading of the unit that wrote it; check it against the design when the slice is decomposed": checked on `f962c324`: the six refusals that name slice 11 (`crates/syntax/src/parser/expression.rs` lines 1227 and 1228, `statement.rs` lines 238, 244, 245 and 246) are constructs of slice 11 by the block's lines (`dir test/language/import/ 11`, `export/`, `feature dynamic-import 11`, `feature import.meta 11`); no refusal of another construct names 11; two module comments name slice 11 for a part of slice 9 (`crates/swell/src/builtins/map/mod.rs` line 1 and `crates/swell/src/builtins/set/mod.rs` line 1: "The Map objects of slice 11", "The Set objects of slice 11"; `Map` and `Set` are slice 9, `engine.md` section 12, item 9), which this story corrects.

Cases of slices delivered after 11 (the order `1,2,3,6,5,7,10,4,9,8,11,12,...`): section 12, "A case that comes into a slice from one delivered later", decides which of them change slice: a case that failed in the start runs and passes in every run on the code of slice 11 and whose `feature` and `flag` lines name no slice delivered after 11 gets a rule line `dir <file> 11` (or `dir <directory>/ 11`), longer than the line that gave it its slice, and no line of `moves.tsv`. Which cases those are is read from the full run after the last wave, not estimated here; cases of slice 19 (`test/staging/`) with `import` or `export` are the likeliest.

The epic's counts (1,277 cases and 1,825 runs, its summary, "Counts" and acceptance 1) are older than the design table (row 11, line 3675 on the base: 1,281 and 1,833) and the gate line; the coordinator rewrites them (Change, "For the coordinator").

## What later slices reuse

Nothing of the engine. The counts of the slices that lose a case to slice 11 or gain one from it move in the table.

## Domain relations

- None new.

## Change

Failing check first: the gate line `test262-slice 11` with failed runs is the failing state; the story ends when the line reads every run passed and the moves check passes.

- Measure with three sets of run lines: `.out/test262-start/runs.tsv` of the commit the first story of slice 11 was merged onto and `.out/test262-base/runs.tsv` of the join after the last wave, both made by the coordinator and copied into the worktree; and `.out/test262/runs.tsv` after this change, made by the unit (`slot --timeout 30m task test262-expected`, or its parts, each part one pool job). Each failing case of slice 11 in the base runs is read with its first failure: a case that needs a later slice moves to it with a token of that slice found in the case or a file it includes or loads; a case that fails on a part of slice 11 goes back to the story whose list holds it, and this story waits for its fix (the coordinator takes it off that list into `close-later.tsv` only when it moves). From the runs of slices delivered after 11, each case that failed at the start and passes in every run is sorted by the rule named in Context into `docs/plan/slice-11/close-into-11.tsv` (path, the slice it leaves, the line that takes it) or no line.
- `conformance/test262/moves.tsv`, `conformance/test262/later-parts.tsv` (a token the moves need and the file lacks; the three lines of slice 11, lines 324 to 326, stay: they name the evidence of the 4 cases moved into 11), `docs/design/engine.md` section 12 (the rule lines, a paragraph "Slice 11's cases that need a later slice" with each group, its count and its reason, or the sentence that there is none; the lines of `close-into-11.tsv` with their counts; the rows of slice 11 and of every slice that loses or gains a case in the table and in the block after "Its output on this document"), `conformance/test262/expected.txt` and `conformance/test262/expected.digest` (written by `task test262-expected`, which `test262-check` compares).
- `crates/swell/src/builtins/map/mod.rs` and `crates/swell/src/builtins/set/mod.rs`, line 1 each: "slice 11" becomes "slice 9" (a comment; no code).
- For the coordinator, in the change that closes the slice (this story does not write these files): `website/data/status.json` (`task site-data`), `11` added to `conformance/slices-delivered.txt`, the epic's counts (1,281 cases and 1,833 runs on `f962c324`, and the numbers this close leaves), `AGENTS.md`, `README.md` and `CHANGELOG.md` with the gate's numbers.
- Time ("Tests stay fast", the `AGENTS.md` of the folder that holds the repository, and `AGENTS.md`, "Commands and the gate"): this story adds no test; a single test is still terminated at 60 s and the gate's test steps still end within 120 s. The full runs are the slow lane, made in parts, each part started once, in the background, with `slot --timeout 30m` (a pool job ends after 10 minutes unless it names its time, at most 30 minutes), and a script that measures or waits runs without the `slot` prefix. The unit never runs `slot task check`: the whole gate runs once on the wave branch. No Python in any command or committed file; no `cd ... &&` with a write or a delete; intermediate files in the scratch directory. No `unsafe`, no third-party crate and no panic are touched by a comment change.

Writes: `docs/design/engine.md` (section 12 as listed), `conformance/test262/{moves,later-parts}.tsv`, `conformance/test262/expected.{txt,digest}`, `docs/plan/slice-11/close-into-11.tsv` (new), `docs/plan/slice-11/close-later.tsv` (a line per case this story moves, with the story list it leaves, which the coordinator edits in the same commit), `crates/swell/src/builtins/{map,set}/mod.rs` (line 1 each). The only story of order 7. Not written: every other source file, `conformance/test262/sections.txt` (no line expected to change; if the run shows one, the unit reports it before it writes), `website/data/status.json`, `conformance/slices-delivered.txt`, the epic, `AGENTS.md`, `README.md`, `CHANGELOG.md` (the coordinator).

It depends on every other story of the slice (their lists are what it checks); directly on `story:dynamic-import`, `story:ess-target-module-commands` and `story:module-determinism-and-emulated-loading`, which depend on the rest. No open story of an earlier slice writes section 12 (slices 1 to 10 but 11 are delivered: `conformance/slices-delivered.txt` on the base).

## Acceptance

`slot --timeout 30m task test262` (or its parts, joined as the coordinator joins them) exits 0 and prints `test262: <n> slice lines equal the design` with no other complaint and one line `test262-slice 11 <c> <r> 0 0 <c> <c> 0 0` (every case of slice 11 run and passed; 1,281 cases and 1,833 runs before any case moves or comes in, the exact `<c>` and `<r>` those the run measures and the design table states). With the three sets of run lines (start, base, after): (1) `awk -F'\t' '$2 == "11" && $5 != "Passed"' .out/test262/runs.tsv | wc -l` prints `0` (`1789` over the base run of `f962c324`: 1,833 runs less the 44 runs of the 23 cases that pass); (2) every case whose slice moves to a later one has a line of `moves.tsv`: `awk -F'\t' 'FILENAME == ARGV[1] { b[$3] = $2; next } FILENAME == ARGV[2] { m[$1] = 1; next } ($3 in b) && b[$3] == "11" && $2 != "11" && !($3 in m) { print $3; bad = 1 } END { exit bad }' .out/test262-base/runs.tsv conformance/test262/moves.tsv .out/test262/runs.tsv` exits 0; (3) no run that passed at the start of the slice fails now: `awk -F'\t' 'NR == FNR { if ($5 == "Passed") p[$3 "\t" $4] = 1; next } (($3 "\t" $4) in p) && $5 != "Passed" { print $3, $4, $6 }' .out/test262-start/runs.tsv .out/test262/runs.tsv` prints nothing; (4) every case that comes into slice 11 by a line of the block is counted there and passes in every run: `awk -F'\t' 'NR == FNR { w[$1] = 1; next } ($3 in w) && ($2 != "11" || $5 != "Passed") { print $3, $2, $5; bad = 1 } END { exit bad }' docs/plan/slice-11/close-into-11.tsv .out/test262/runs.tsv` exits 0; (5) the two cases of the close pass: the same command over `docs/plan/slice-11/close-joint.cases.txt` exits 0 (over the base run it exits 1 and prints the one run of `json-via-namespace.js`); (6) the lists partition the slice: `sh docs/plan/slice-11/lists.sh .out/test262/runs.tsv` exits 0 and its last line reads `slice <c> lists <m> incoming 0` with `<m>` the cases of the lists, `<c>` less the lines of `close-into-11.tsv` (the cases that come in are on no list). `slot task -d . -t taskfiles/test262.yml test262-moves-check` exits 0. After `{ cat conformance/slices-delivered.txt; echo 11; } > .out/slices-delivered-11.txt`, `slot task -d . -t taskfiles/host-requirements.yml host-requirements-check HOST_REQ_DELIVERED=.out/slices-delivered-11.txt` exits 0 (`grep -c 'pending slice 11$' conformance/host/requirements.txt` prints `0`; `32` on the base). `grep -rnE 'refuse[_a-z]*\("[^"]*", 11' crates/syntax/src crates/compiler/src | wc -l` prints `0` (`6` on the base) and `git grep -n 'slice 11' -- crates/swell/src/builtins` prints nothing (`2` lines on the base). The report quotes the output of each check.

## Out of scope

Any engine, compiler, parser or runner source file but the two comments (a failing case of a part of slice 11 goes back to its owner); BigInt (slice 12) and the 4 slice-12 cases that use `import()`; the epic, `AGENTS.md`, `README.md`, `CHANGELOG.md`, `website/data/status.json` and `conformance/slices-delivered.txt` (the coordinator); `taskfiles/test262.yml`; the design text outside section 12.
