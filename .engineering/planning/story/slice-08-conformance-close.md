---
format: aep.planning-md/3
id: story:slice-08-conformance-close
kind: story
status: draft
title: 'Every run of slice 8 passes: 5 cases move to slice 13, the slice-9 and passing cases are checked, passing cases of later slices come in'
summary: 5 cases (10 runs) to slice 13 with evidence in moves.tsv; 38 slice-9 and 5 passing cases checked; section 12 and expected.txt.
relations:
- decomposes: epic:slice-08-classes
- depends_on: story:new-target-meta-property
- depends_on: story:super-property-and-super-syntax
- depends_on: story:class-syntax-and-early-errors
- depends_on: story:class-definition-evaluation
- depends_on: story:class-element-syntax-and-early-errors
- depends_on: story:derived-classes-and-super-call
- depends_on: story:public-class-fields
- depends_on: story:class-static-blocks
- depends_on: story:private-names-and-private-methods
- depends_on: story:instance-elements-initialization-order
- depends_on: story:private-fields-and-private-in
- depends_on: story:eval-in-class-field-initializers
- depends_on: story:private-names-in-eval-code
- depends_on: story:host-ensure-can-add-private-element
- depends_on: story:slice-04-conformance-close
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
  path: docs/design/engine.md
- confidence: inferred
  path: docs/plan/slice-08/close-into-08.tsv
revision: 3
---
## Context

The epic asks every run of slice 8 to pass, and a case that needs a later slice is moved by a rule line of the `swell-slices` block with its reason, in the change that moves both slices' counts, each move with its evidence in `conformance/test262/moves.tsv` (`AGENTS.md`, "A case that cannot pass in its slice"; `docs/design/engine.md`, section 12), as `story:slice-10-conformance-close` and `story:slice-04-conformance-close` do for their slices. This story makes that move for slice 8 once every other story of the slice has landed, owns the block, its paragraph and its table for these lines, and writes no engine file.

Measured by the decomposition on `62c7cf59` with the full run of 2026-11-26 on `149e12f1` (its total line `test262 total 53801 8982 7 44812 25943 18867 2` and its slice line `test262-slice 08 9302 18455 0 0 9302 5 9297 0` equal `conformance/test262/expected.txt` on `62c7cf59`). The 9,302 cases of slice 8 are partitioned by the lists of `docs/plan/slice-08/` (`sh docs/plan/slice-08/lists.sh <runs.tsv>` prints `lists: 9302 cases of slice 8, 9302 lines in 16 lists` and exits 0 over that run; `docs/plan/slice-08/scan.awk` gives the assignment, `scan.tsv`): 9,254 on the 13 story lists, and three close lists:

- `close-later.tsv`: 5 cases (10 runs) that need slice 13 (binary data): `language/statements/class/subclass/builtin-objects/{ArrayBuffer,DataView}/{regular-subclassing,super-must-be-called}.js` and `language/statements/class/subclass/builtins.js`. They move to slice 13, the evidence its token (`ArrayBuffer`, `DataView`, a typed array; all in `conformance/test262/later-parts.tsv`).
- `close-slice-9.cases.txt`: 38 cases (76 runs) that need a part of slice 9 (`Proxy`, `Reflect`, `Map`, `Set`, `WeakMap`, `WeakSet`): 20 of `class/elements`, 5 of `class/subclass`, 3 of `Function/prototype/bind`, 3 of `Proxy/construct` and others. Slice 9 is delivered before slice 8 (`order delivery 1,2,3,6,5,7,10,4,9,8,...`), so these stay in slice 8 and must pass: this story checks each; one that fails on a part of slice 8 goes back to the story whose part it needs (by `scan.tsv`'s parts) and this story waits; one that fails on a part of slice 9 is reported to the coordinator as a defect of slice 9, and this story waits.
- `close-pass.cases.txt`: 5 cases (10 runs) that pass on the base (4 `*-init-fn-name-fn` cases of `dstr` and `language/expressions/new.target/escaped-new.js`): this story checks they still pass.

Cases of slices delivered after 8 (the order above: 11, 12, 13 and the rest): section 12, "A case that comes into a slice from one delivered later", decides which of them change slice, as for the closes before: a case that failed in the start runs and passes in every run on the code of slice 8 and whose `feature` and `flag` lines name no slice delivered after 8 gets a rule line `dir <file> 8` (or `dir <directory>/ 8` when every case of the directory is one) and no line of `moves.tsv`; a case counted in a slice delivered before 8 gets no line. Which cases those are is read from the full run after the last wave, not estimated here.

`conformance/host/requirements.txt` has no line pending for slice 8 once `story:host-ensure-can-add-private-element` has closed HOST-119, the only one (`grep -c "pending slice 8$"` prints `1` on the base), which `host-requirements-check` with slice 8 delivered verifies.

Design it builds on: section 12 of `docs/design/engine.md` and its block as the closes of slices 4 and 9 leave them ("the longest matching prefix wins"; "a case is counted in the slice delivered last among the slices of its lines"; "A case that comes into a slice from one delivered later", its rules); the table of slices and the block after "Its output on this document:". The epic `epic:slice-08-classes` states 6,188 cases and says the async members of classes are counted in slice 10; the table states 9,302 cases and 18,455 runs, since the close of slice 10 moved 2,743 cases to slice 8 (the coordinator rewrites the epic's numbers, below).

## What later slices reuse

Nothing of the engine. The count of slice 13 in the table grows by the 5 cases this close gives it.

## Domain relations

- None new.

## Change

Failing check first: the gate line `test262-slice 08` with failed runs is the failing state; the story ends when the line reads every run passed and the moves check passes.

- Measure with three sets of run lines: `.out/test262-start/runs.tsv` of the commit the first story of slice 8 was merged onto and `.out/test262-base/runs.tsv` of the wave before this change, both made by the coordinator and copied into the worktree; and `.out/test262/runs.tsv` after it, made by the unit in parts (`slot --timeout 30m ...`, each part one pool job). `sh docs/plan/slice-08/lists.sh .out/test262-start/runs.tsv` exits 0 (no case entered or left slice 8 since the plan; if one did, the unit adds it to the list of the story whose parts it needs or to a close list and reports it). Each failing case of slice 8 in the base runs is read with its first failure: on a story list, it goes back to that story; on `close-later.tsv`, it moves; on `close-slice-9.cases.txt`, it is handled as Context says. From the runs of slices delivered after 8, each case that failed in the start runs and passes in every run is sorted by the rule of section 12 into `docs/plan/slice-08/close-into-08.tsv` (path, the slice it leaves, the line that takes it) or no line.
- `conformance/test262/moves.tsv`: one line per case whose slice the change moves to a later slice (5, 10 runs, all to 13; kind `part`, the token its evidence), five fields, sorted by path.
- `conformance/test262/later-parts.tsv`: a token the moves need and the file lacks (none expected: `ArrayBuffer`, `DataView` and the typed arrays are there).
- `docs/design/engine.md`, section 12: the rule lines of the block (one per file for the 5 moves, or `dir test/language/statements/class/subclass/builtin-objects/ArrayBuffer/ 13` and the same for `DataView` when every case of slice 8 in the directory moves, and one line for `subclass/builtins.js`), and the lines of `close-into-08.tsv`; a paragraph "Slice 8's cases that need a later slice" with each group by target slice, its count and its reason, the counts summing to the lines this story adds to `moves.tsv`, and the lines that take cases into slice 8 with their count by directory and the slice each case leaves, and the sentence that these have no line of `moves.tsv`; the rows of slice 8, of slice 13 and of every slice that loses a case to 8 in the table and in the block after "Its output on this document", the lines summing to the totals they had before the change; the sentence on slice 8 under the table with the command that measured it.
- `conformance/test262/expected.txt` and `conformance/test262/expected.digest`: the lines of the run, in the same change, and the digest of its sources beside them, both written by `task test262-expected`, which `test262-check` compares.
- For the coordinator, in the change that closes the slice (this story does not write these files): `website/data/status.json`, written again by `task site-data` (it reads `moves.tsv`, section 12 and `expected.txt`); `8` added to `conformance/slices-delivered.txt`; the epic's counts (9,302 cases and 18,455 runs; the async members of classes in slice 8, not 10); `AGENTS.md`, `README.md` and `CHANGELOG.md` with the gate's numbers; the edge `depends_on` from this story to slice 9's close story when it is drafted.
- Time ("Tests stay fast", the `AGENTS.md` of the folder that holds the repository, and `AGENTS.md`, "Commands and the gate"): this story adds no test. The three full runs are the slow lane, made in parts, each part started once, in the background, with `slot --timeout 30m`; a pool job ends after 10 minutes unless it names its time (30 minutes at most), and a script that measures, counts or waits (`lists.sh`, `check.sh`, the sort into `close-into-08.tsv`) runs without the `slot` prefix. The unit never runs `slot task check`: the whole gate runs once on the wave branch. No Python in any command; intermediate files in the scratch directory or `.out/`.

## Requirements

- No host requirement of slice 8 is pending when this story merges: `grep -c "pending slice 8$" conformance/host/requirements.txt` prints `0` (after `story:host-ensure-can-add-private-element`), and `slot task host-requirements-check` exits 0 with `8` in `conformance/slices-delivered.txt` (the coordinator's line, checked on the join).
- Rules every story carries. No unsafe code and no third-party crate: `git diff --stat "$BASE" -- Cargo.lock "*/Cargo.toml"` prints nothing. No source file of a crate changes: `git diff --stat "$BASE" -- crates` prints nothing.

Writes: `docs/design/engine.md` (section 12 as listed), `conformance/test262/moves.tsv`, `conformance/test262/later-parts.tsv` (only if a token is missing), `conformance/test262/expected.txt`, `conformance/test262/expected.digest`, `docs/plan/slice-08/close-into-08.tsv` (new). The only story of its order. It depends on every other story of the slice (their lists are what it checks) and on `story:slice-04-conformance-close`, the open close of the slice delivered before 9 and 8, which writes the same five files (section 12's rule lines, table rows and block, `moves.tsv`, `later-parts.tsv`, `expected.txt`, `expected.digest`); slice 9's close, which is delivered between them and writes the same files, gets its edge from the coordinator when it is drafted (question Q6 of the decomposition's report). This story builds on what those leave: section 12 and its block as they leave them, slices 4 and 9 delivered, and the cases those closes take into their slices. Not written: `conformance/test262/sections.txt` (no line expected to change; if the run shows one, the unit reports it before it writes), `website/data/status.json` (the coordinator's), every source file, `docs/plan/slice-08/` but `close-into-08.tsv`.

## Acceptance

`slot --timeout 30m task test262` (or its parts, joined as the coordinator joins them) exits 0 and prints `test262: <n> slice lines equal the design` with no other complaint and one line `test262-slice 08 <c> <r> 0 0 <c> <c> 0 0` (every case of slice 8 run and passed; 9,297 cases and 18,445 runs before the cases that come in, 9,302 − 5 and 18,455 − 10, the exact `<c>` and `<r>` those the run measures and the design table states). Fields 3 and 4 of the slice lines sum, over the table, to the totals they had before this change: `awk '$1 == "test262-slice" { c += $3; r += $4 } END { print c, r }' .out/test262/lines.txt` prints the same two numbers as over the lines of `.out/test262-base/runs.tsv`'s count (no case enters or leaves the run). `sh docs/plan/slice-08/check.sh` over each story list with `.out/test262-base/runs.tsv` and `.out/test262/runs.tsv` exits 0 (13 lists), and over `docs/plan/slice-08/close-slice-9.cases.txt` and `close-pass.cases.txt` the same (38 and 5 cases, every run passed). The paragraph "Slice 8's cases that need a later slice" matches `moves.tsv`: `awk -F'\t' '$2 == "8" { n[$3]++; t++ } END { for (s in n) print s, n[s]; print "all", t + 0 }' conformance/test262/moves.tsv | LC_ALL=C sort` prints the groups the paragraph states (`13 5`, `all 5` with no other move found). `slot task test262-moves-check` and `slot task test262-check` exit 0; `git diff --stat "$BASE" -- crates Cargo.lock "*/Cargo.toml"` prints nothing. The unit's report states the counts of each list before and after, the cases that came in by directory, and any case sent back to a story with its first failure.

## Out of scope

Any engine, compiler, parser or runner source file (a failing case of a part of slice 8 goes back to its owner); binary data (slice 13), `Proxy`, `Reflect` and the collections (slice 9, delivered before), modules (slice 11), BigInt (slice 12); the epic, `AGENTS.md`, `README.md`, `CHANGELOG.md`, `website/data/status.json` and `conformance/slices-delivered.txt` (the coordinator); `taskfiles/test262.yml` (the moves check as it stands); the design text outside section 12.
