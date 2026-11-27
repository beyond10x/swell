---
format: aep.planning-md/3
id: story:slice-08-conformance-close
kind: story
status: implemented
title: 'Every run of slice 8 passes: 9 cases move to later slices (5 to slice 13, 4 to slice 18), the slice-9 and passing cases are checked, passing cases of later slices come in'
summary: 9 cases (18 runs) moved, 5 to slice 13 and 4 to slice 18, with evidence in moves.tsv; 91 cases of slice 9's parts and 5 passing cases checked; section 12 and expected.txt.
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
- depends_on: story:private-fields-and-private-in
- depends_on: story:eval-in-class-field-initializers
- depends_on: story:private-names-in-eval-code
- depends_on: story:host-ensure-can-add-private-element
- depends_on: story:slice-04-conformance-close
- depends_on: story:slice-09-conformance-close
- depends_on: story:gate-test-steps-hold-the-budget-on-the-slower-runner
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
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-11-26T08:23:49Z", actor: "human:operator", revision: 7, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-26T08:23:49Z", actor: "human:operator", revision: 8, executor: "agent:coordinator"}
- {from: "active", to: "implemented", at: "2026-11-27T21:14:36Z", actor: "human:operator", revision: 9, decided_on: {"recorded":{"test_result":1,"review_outcome":2,"verification":1}}, executor: "agent:coordinator"}
---
## Context

The epic asks every run of slice 8 to pass, and a case that needs a later slice is moved by a rule line of the `swell-slices` block with its reason, in the change that moves both slices' counts, each move with its evidence in `conformance/test262/moves.tsv` (`AGENTS.md`, "A case that cannot pass in its slice"; `docs/design/engine.md`, section 12), as `story:slice-10-conformance-close`, `story:slice-04-conformance-close` and `story:slice-09-conformance-close` do for their slices. This story makes that move for slice 8 once every other story of the slice has landed, owns the block, its paragraph and its table for these lines, and writes no engine file.

Measured by the decomposition on `62c7cf59` with the full run of 2026-11-26 on `149e12f1` (its slice line `test262-slice 08 9302 18455 0 0 9302 5 9297 0` equals `conformance/test262/expected.txt` on `62c7cf59`; fields 3 and 4 of its slice lines sum to `53801 103360`). The 9,302 cases of slice 8 and the 37 that slice 9's close is to move into it are partitioned by the lists of `docs/plan/slice-08/` (`docs/plan/slice-08/scan.awk` gives the assignment, `scan.tsv`; `sh docs/plan/slice-08/lists.sh <runs>` prints one line per list and then `slice 9302 lists 9339 incoming 37`, exit 0, over that run): 9,234 on the 12 story lists and 105 on the three close lists (68 of slice 8, 37 incoming), and `incoming.tsv` beside them:

- `close-later.tsv`: 9 cases (18 runs), three fields (path, the later slice, the parts): 5 that need slice 13 (binary data: `language/statements/class/subclass/builtin-objects/{ArrayBuffer,DataView}/{regular-subclassing,super-must-be-called}.js` and `language/statements/class/subclass/builtins.js`; tokens `ArrayBuffer` and `DataView`, `conformance/test262/later-parts.tsv`) and 4 that need slice 18 (annex B: `language/{statements,expressions}/class/elements/private-{getter,setter}-is-not-a-own-property.js`, which call `__lookupGetter__` or `__lookupSetter__`; tokens of the same names, `later-parts.tsv`), the 4 taken off the list of `story:private-names-and-private-methods` by the coordinator's decision 1 of revision 1. The coordinator adds to it, in the same commit, each case it takes off a story list during the wave (path, the slice or the story, `story:<name>`, that takes it, one reason). This story settles each line, Acceptance (7).
- `close-slice-9.cases.txt`: 91 cases (182 runs) that need a part of slice 9: 54 of slice 8 (`Proxy`, `Reflect`, `Map`, `Set`, `WeakMap`, `WeakSet`; among them the 16 that subclass `Map`, `Set`, `WeakMap` or `WeakSet`, taken off the list of `story:derived-classes-and-super-call` by the coordinator's decision 2) and the 37 of `docs/plan/slice-08/incoming.tsv` (36 of `built-ins/Set/prototype/<set method>` and 1 of `language/expressions/optional-chaining`, which `story:slice-09-conformance-close` moves from slice 9 to slice 8: `class` or `super`, or a refusal of slice 8 in their first failure; 74 runs, each failed on the plan's run). Slice 9 is delivered before slice 8, so these stay in slice 8 and must pass: a case that fails on a part of slice 8 goes back to the story whose part it needs (by `scan.tsv`'s parts) and this story waits; one that fails on a part of slice 9 is reported to the coordinator as a defect of slice 9, and this story waits.
- `close-pass.cases.txt`: 5 cases (10 runs) that pass on the base (4 `*-init-fn-name-fn` cases of `dstr` and `language/expressions/new.target/escaped-new.js`): this story checks they still pass.
- `incoming.tsv` is no list of its own: its cases are on `close-slice-9.cases.txt`, and `lists.sh` counts a line of it the run does not count in slice 8 yet as `incoming`.

Cases of slices delivered after 8 (the order `1,2,3,6,5,7,10,4,9,8,...`): section 12, "A case that comes into a slice from one delivered later", decides which of them change slice: a case that failed in the start runs and passes in every run on the code of slice 8 and whose `feature` and `flag` lines name no slice delivered after 8 gets a rule line `dir <file> 8` (or `dir <directory>/ 8` when every case of the directory is one) and no line of `moves.tsv`; a case counted in a slice delivered before 8 gets no line. Which cases those are is read from the full run after the last wave, not estimated here. No refusal of `swell-syntax` or `swell-compiler` names slice 8 once the slice is built: `grep -rnE 'refuse[_a-z]*\("[^"]*", 8' crates/syntax/src crates/compiler/src | wc -l` prints `5` on `61f2a8e2` and `0` after the stories (Acceptance).

`conformance/host/requirements.txt`: `grep -c 'pending slice 8$' conformance/host/requirements.txt` prints `1` on `61f2a8e2` (HOST-119), and `0` once `story:host-ensure-can-add-private-element` has closed HOST-119 and the HOST-223 it adds; this story reads `0` and changes no line of the file.

Design it builds on: section 12 of `docs/design/engine.md` and its block as the closes of slices 4 and 9 leave them ("the longest matching prefix wins"; "a case is counted in the slice delivered last among the slices of its lines"; "A case that comes into a slice from one delivered later", its rules 1 to 4); the table of slices and the block after "Its output on this document:". The epic's counts are the coordinator's (it rewrites them: 9,302 cases and 18,455 runs).

## What later slices reuse

Nothing of the engine. The counts of slices 13 and 18 in the table grow by the cases this close gives them (on the plan's numbers 5 cases and 10 runs to 13, 4 cases and 8 runs to 18).

## Domain relations

- None new.

## Change

Failing check first: the gate line `test262-slice 08` with failed runs is the failing state; the story ends when the line reads every run passed and the moves check passes.

- Measure with three sets of run lines: `.out/test262-start/runs.tsv` of the commit the first story of slice 8 was merged onto and `.out/test262-base/runs.tsv` of the wave branch after the last other story of the slice, both made by the coordinator and copied into the worktree; and `.out/test262/runs.tsv` after this change, made by the unit in parts, each part one pool job (`slot --timeout 30m ...`), with its count lines in `.out/test262/lines.txt` and the base's in `.out/test262-base/lines.txt`. Each failing case of slice 8 in the base runs is read with its first failure: on a story list, it goes back to that story and this story waits; on `close-later.tsv`, it moves (or, when field 2 names a story of slice 8, it goes back to that story); on `close-slice-9.cases.txt`, as Context says. From the runs of slices delivered after 8, each case that failed in the start runs and passes in every run is sorted by the rule of section 12 named in Context into `docs/plan/slice-08/close-into-08.tsv` (path, the slice it leaves, the line that takes it) or no line.
- `conformance/test262/moves.tsv`: one line per case whose slice the change moves to a later slice (on the plan's numbers 9 cases, 18 runs: 5 to 13 and 4 to 18; kind `part`, the token its evidence), five fields, sorted by path.
- `conformance/test262/later-parts.tsv`: a token the moves need and the file lacks (none expected: `ArrayBuffer`, `DataView`, `__lookupGetter__` and `__lookupSetter__` are there on `61f2a8e2`).
- `docs/design/engine.md`, section 12: the rule lines of the block (one per directory when every case of slice 8 in it moves to one slice, one per file otherwise), and the lines of `close-into-08.tsv`; a paragraph "Slice 8's cases that need a later slice" with each group by target slice, its count and its reason, the counts summing to the lines this story adds to `moves.tsv`, and the lines that take cases into slice 8 with their count by directory and the slice each case leaves, and the sentence that these have no line of `moves.tsv`; the rows of slice 8, of slices 13 and 18 and of every slice that loses a case to 8 in the table and in the block after "Its output on this document", the lines summing to the totals they had before the change; the sentence on slice 8 under the table with the command that measured it.
- `conformance/test262/expected.txt` and `conformance/test262/expected.digest`: the lines of the run, in the same change, and the digest of its sources beside them, both written by `task test262-expected`, which `test262-check` compares.
- For the coordinator, in the change that closes the slice (this story does not write these files): `website/data/status.json`, written again by `task site-data` (it reads `moves.tsv`, section 12 and `expected.txt`); `8` added to `conformance/slices-delivered.txt`; the epic's counts; `AGENTS.md`, `README.md` and `CHANGELOG.md` with the gate's numbers.
- Bounds, as `AGENTS.md` and the `AGENTS.md` of the folder that holds the repository ("Tests stay fast", "A job in the pool ends within 10 minutes", "Scripts that do not build stay off slot", "No Python for permanent tools") state them: this story adds no test; a single test is terminated at 60 s and the gate's test steps end within 120 s. The three full runs are the slow lane, made in parts, each part started once, in the background, with `slot --timeout 30m`; a pool job ends after 10 minutes unless it names its time (30 minutes at most), and a script that measures, counts or waits (`lists.sh`, `check.sh`, the `awk` checks below, the sort into `close-into-08.tsv`) runs without the `slot` prefix. The unit never runs `slot task check`: the whole gate runs once on the wave branch. No Python in any command; intermediate files in the scratch directory or `.out/`.

## Requirements

- No host requirement of slice 8 is pending when this story merges: `grep -c 'pending slice 8$' conformance/host/requirements.txt` prints `0` (after `story:host-ensure-can-add-private-element`; `1` on `61f2a8e2`), `git diff --quiet "$BASE" -- conformance/host/requirements.txt` exits 0, and `host-requirements-check` with `8` delivered exits 0 (Acceptance).
- No unsafe code and no third-party crate: `git diff --stat "$BASE" -- Cargo.lock "*/Cargo.toml"` prints nothing. No source file of a crate changes: `git diff --stat "$BASE" -- crates` prints nothing.

Writes: `docs/design/engine.md` (section 12 as listed), `conformance/test262/moves.tsv`, `conformance/test262/later-parts.tsv` (only if a token is missing), `conformance/test262/expected.txt`, `conformance/test262/expected.digest`, `docs/plan/slice-08/close-into-08.tsv` (new). Order 7, the only story of its order. It depends on every other story of the slice (their lists are what it checks), on `story:slice-04-conformance-close` and on `story:slice-09-conformance-close` (slice 9 is delivered after 4 and before 8; both write the same five files: section 12's rule lines, table rows and block, `moves.tsv`, `later-parts.tsv`, `expected.txt`, `expected.digest`; slice 9's close moves the 37 incoming cases into slice 8, and run after this one it would take passing cases of slice 8 back). This story builds on what those leave. Not written: `conformance/test262/sections.txt` (no line expected to change; if the run shows one, the unit reports it before it writes), `website/data/status.json` (the coordinator's), every source file, `docs/plan/slice-08/` but `close-into-08.tsv`.

## Acceptance

`slot --timeout 30m task test262` (or its parts, joined as the coordinator joins them) exits 0 and prints `test262: <n> slice lines equal the design` with no other complaint and one line `test262-slice 08 <c> <r> 0 0 <c> <c> 0 0` (every case of slice 8 run and passed; on the plan's numbers 9,330 cases and 18,511 runs before the cases that come in from later slices: 9,302 + 37 − 9 and 18,455 + 74 − 18; the exact `<c>` and `<r>` those the run measures and the design table states). The sum check: `awk '$1 == "test262-slice" { c += $3; r += $4 } END { print c, r }' .out/test262/lines.txt` prints the same two numbers as `awk '$1 == "test262-slice" { c += $3; r += $4 } END { print c, r }' .out/test262-base/lines.txt` (`53801 103360` on the plan's run, while the pin of test262 is unchanged: no case enters or leaves the run). With the three sets of run lines (S start, B base, N after):

(1) `awk -F'\t' '$2 == "08" && $5 != "Passed"' .out/test262/runs.tsv | wc -l` prints `0` (over the plan's run it printed 18,445; over `.out/test262-base/runs.tsv` it prints the runs of the cases that move, which the report quotes).
(2) Every case whose slice moves to a later one has a line of `moves.tsv`: `awk -F'\t' 'FILENAME == ARGV[1] { b[$3] = $2; next } FILENAME == ARGV[2] { m[$1] = 1; next } ($3 in b) && b[$3] == "08" && $2 != "08" && !($3 in m) { print $3; bad = 1 } END { exit bad }' .out/test262-base/runs.tsv conformance/test262/moves.tsv .out/test262/runs.tsv` exits 0.
(3) No run that passed at the start of the slice fails now: `awk -F'\t' 'NR == FNR { if ($5 == "Passed") p[$3 "\t" $4] = 1; next } (($3 "\t" $4) in p) && $5 != "Passed" { print $3, $4, $6 }' .out/test262-start/runs.tsv .out/test262/runs.tsv` prints nothing.
(4) Every case that comes into slice 8 by a line of the block is counted there and passes in every run: `awk -F'\t' 'NR == FNR { w[$1] = 1; next } ($3 in w) && ($2 != "08" || $5 != "Passed") { print $3, $2, $5; bad = 1 } END { exit bad }' docs/plan/slice-08/close-into-08.tsv .out/test262/runs.tsv` exits 0.
(5) Every case moved out of slice 8 has a failed run on the base and none of its runs fails on a refusal of slice 8: `awk -F'\t' 'NR == FNR { if ($2 == "8") mv[$1] = 1; next } ($3 in mv) { if ($5 != "Passed") failed[$3] = 1; if ($6 ~ /is not supported before slice 8/) bad[$3] = 1 } END { for (k in mv) if (!(k in failed) || (k in bad)) print k }' conformance/test262/moves.tsv .out/test262-base/runs.tsv | wc -l` prints `0` (field 2 of `moves.tsv` is the slice a case leaves, without a leading zero).
(6) The incoming cases are counted in slice 8 and pass: `awk -F'\t' 'NR == FNR { w[$1] = 1; next } ($3 in w) && ($2 != "08" || $5 != "Passed") { print $3, $2, $5; bad = 1 } END { exit bad }' docs/plan/slice-08/incoming.tsv .out/test262/runs.tsv` exits 0.
(7) The lists: `sh docs/plan/slice-08/lists.sh .out/test262-base/runs.tsv` exits 0 and its last line is `slice 9339 lists 9339 incoming 0` on the plan's numbers (slice 9's close has moved the 37; a case another close took out of slice 8 or into it before this story, each named in the report with the commit of the coordinator that changed the list, changes both numbers alike); and `close-later.tsv` is settled line by line, the decomposition's 9 and those the coordinator added: each case passes in every run after this change, or has a line of `moves.tsv` to the slice of field 2 (13 or 18 for the 9; the slice of the story it names when that story is of another slice): `awk -F'\t' 'FILENAME == ARGV[1] { c[$1] = $2; next } FILENAME == ARGV[2] { m[$1] = 1; next } ($3 in c) && $5 != "Passed" && !($3 in m) { print $3, c[$3]; bad = 1 } END { exit bad }' docs/plan/slice-08/close-later.tsv conformance/test262/moves.tsv .out/test262/runs.tsv` exits 0; a case whose field 2 names a story of slice 8 has no line of `moves.tsv`. The paragraph "Slice 8's cases that need a later slice" matches `moves.tsv`: `awk -F'\t' '$2 == "8" { n[$3]++; t++ } END { for (s in n) print s, n[s]; print "all", t + 0 }' conformance/test262/moves.tsv | LC_ALL=C sort` prints `13 5`, `18 4` and `all 9` on the plan's numbers (a case of the 9 that passes stays and lowers its group; one the coordinator added raises its group; the report names each).

Then: `sh docs/plan/slice-08/check.sh` over each of the 12 story lists, `close-slice-9.cases.txt` (91 cases) and `close-pass.cases.txt` (5 cases), with `.out/test262-base/runs.tsv` and `.out/test262/runs.tsv`, exits 0 each. `slot task -d . -t taskfiles/test262.yml test262-moves-check` exits 0 and its count of moved cases equals the lines of `conformance/test262/moves.tsv`; `slot task test262-check` exits 0; after `{ cat conformance/slices-delivered.txt; echo 8; } > .out/slices-delivered-8.txt`, `slot task -d . -t taskfiles/host-requirements.yml host-requirements-check HOST_REQ_DELIVERED=.out/slices-delivered-8.txt` exits 0; `grep -rnE 'refuse[_a-z]*\("[^"]*", 8' crates/syntax/src crates/compiler/src | wc -l` prints `0` (`5` on `61f2a8e2`); `git diff --stat "$BASE" -- crates Cargo.lock "*/Cargo.toml"` prints nothing. The unit's report quotes the output of each check, the counts of each list before and after, the cases that came in by directory, and any case sent back to a story with its first failure.

## Out of scope

Any engine, compiler, parser or runner source file (a failing case of a part of slice 8 goes back to its owner); binary data (slice 13), annex B's `__lookupGetter__` and `__lookupSetter__` (slice 18), `Proxy`, `Reflect` and the collections (slice 9, delivered before), modules (slice 11), BigInt (slice 12); the epic, `AGENTS.md`, `README.md`, `CHANGELOG.md`, `website/data/status.json` and `conformance/slices-delivered.txt` (the coordinator); `taskfiles/test262.yml` (the moves check as it stands); the design text outside section 12.
