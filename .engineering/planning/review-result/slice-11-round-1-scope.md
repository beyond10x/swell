---
format: aep.planning-md/3
id: review-result:slice-11-round-1-scope
kind: review-result
status: active
title: 'Slice 11, round 1, the scope critic: needs-revision, 6 findings'
relations:
- reviews: story:dynamic-import
- reviews: story:ess-target-module-commands
- reviews: story:import-call-and-import-meta-syntax
- reviews: story:import-meta
- reviews: story:json-modules
- reviews: story:module-code-compiles
- reviews: story:module-determinism-and-emulated-loading
- reviews: story:module-linking-and-evaluation
- reviews: story:module-namespace-objects
- reviews: story:module-records-loading-and-module-runs
- reviews: story:module-syntax-and-early-errors
- reviews: story:slice-11-conformance-close
- reviews: story:top-level-await
revision: 1
---
Plan critic, scope, round 1 on the thirteen draft stories of
`epic:slice-11-modules` (`plan/slice-11` at `c3021d72`), 2026-11-28.
Verdict: needs-revision, 6 findings; revision 1 answers them, and adds two stories. The critic's answer, as returned:

needs-revision

1. `story:module-namespace-objects` and `story:dynamic-import` (blocking). Both acceptances print `B` and `bound` values that the unit cannot see.
   - Places: `.engineering/planning/story/module-namespace-objects.md:75`, `.engineering/planning/story/dynamic-import.md:115`.
   - What is wrong: the lines promise `passed <P> bound 834 B 764 F 70 R 0` and `passed <P> bound 782 B 298 F 484 R 0`. The base run the unit uses is the join's, not `f962c324`.
   - `part.sh` pulls in extra cases by `./name`. At the join these extras already pass, because their owning story has landed.
   - `check.sh` counts them in `B`. Its `bound` is `B + F - R`.
   - Evidence: I ran `part.sh` into scratch for every part. The part sizes match the stories (987, 1137, 1190, 423, 844, 538, 6, 87, 793). The extras and their owners:
     - **Namespace part:** 10 extras. 9 are `instn-iee-*` cases owned by the linking list, which pass after order 4. `verify-dfs.js` belongs to the dynamic-import list and still fails.
     - **Dynamic part:** 11 extras. 9 are the same linking cases, and `instn-star-equality.js` and `instn-star-iee-cycle.js` are on the namespace list. All 11 pass at the order-6 join.
     - **Printed values at the join:** namespace prints `B 773 … bound 843`. Dynamic-import prints `B 309 … bound 793`.
     - **Parts without the problem:** records and linking have extras whose owners come later, so they still fail and `B` is exact. Syntax, import-call, top-level-await, import-meta and json-modules have no extras.
   - What settles it: write the printed line for the join (`bound 843 B 773`, `P` at least 843; `bound 793 B 309`, `P` at least 793). Alternatively, state that `B` counts the extras whose owner has landed and give the formula.

2. `epic:slice-11-modules` and `story:slice-11-conformance-close` (blocking). The epic's out-of-scope sentence about the command-line host's loader is claimed by no story, and the decision lives only in the scratch report (Q7).
   - Quoted promise: "The command-line host and the runner provide their own loaders." (`.engineering/planning/epic/slice-11-modules.md:80`).
   - The runner's loader is covered by `story:module-records-loading-and-module-runs`.
   - Three stories point to "the question in the decomposition's report", which is not in the repository. The records story's Out-of-scope is at `.engineering/planning/story/module-records-loading-and-module-runs.md:111`.
   - `crates/cli/src/shell.rs:38-39` says "the shell has no event loop and no module loader". After slice 11 `import()` from the command-line host leaves its promise pending.
   - Precedent: slice 9 round 1's only finding was an epic out-of-scope line that contradicted its stories (`.engineering/planning/review-result/slice-09-round-1-scope.md`).
   - What settles it: take Q7 option A. Otherwise add to the close story's coordinator list (`.engineering/planning/story/slice-11-conformance-close.md:61`) the rewrite of that epic sentence, for example "a command-line host module loader: not in this slice". Also give the three "not in this slice" mentions a target other than the report.

3. `story:module-linking-and-evaluation`, `story:dynamic-import`, `story:ess-target-module-commands` (blocking, cheap to fix). They name an artifact that does not exist, `story:module-determinism-faults-and-emulated-loading`. The real id is `story:module-determinism-and-emulated-loading`.
   - Evidence: `grep -rn determinism-faults .engineering docs` finds five hits. Linking has one. `dynamic-import.md` has two (lines 101 and 109). `ess-target-module-commands.md` has two (lines 47 and 55). The graph lists the real id only.
   - What settles it: replace the id in those five places.

4. `story:slice-11-conformance-close` (note). The epic's open question 3 is not answered in any story body.
   - Quoted: "Which of the 39 features with no `swell-slices` line fall inside this slice." (`.engineering/planning/epic/slice-11-modules.md:48`), "to be settled by this epic's first story".
   - The report says "no effect on this slice". No story carries the answer.
   - Slice 9's close story does, at `.engineering/planning/story/slice-09-conformance-close.md:49`.
   - From the cases: the slice's cases use `globalThis` (11 cases) and `hashbang` (1 case), which have no `feature` line in `engine.md`. Every other feature of the slice's cases has one.
   - What settles it: one sentence in the close story's Context, in the form of slice 9.

5. `story:module-syntax-and-early-errors` and `story:import-call-and-import-meta-syntax` (note). The line numbers for the `import` arm of `statement.rs` are off by one.
   - On the base, the refusals are at 245 (`import.meta`), 246 (`dynamic import`) and 247 (`import declaration`). The `export` refusal is at 238, which is correct.
   - The syntax story's Change bullet says 245 to 247, but its closing paragraph (line 71) says 244, 245 and 246. The import-call story (line 71) says "lines 244 and 245".
   - The `grep … | wc -l` counts in both acceptances are right (6, then 4, then 0).
   - What settles it: use 245, 246 and 247 throughout.

6. `story:ess-target-module-commands` (note, within scope only). The epic says the suite must hold a passed scenario "for the module commands and events it states" (`.engineering/planning/epic/slice-11-modules.md`, acceptance item 5).
   - The acceptance names the seven commands, one passed scenario each.
   - It names none of the four events (`ModuleParsed`, `ModuleLoadRequested`, `ModuleLoadEnded`, `ModuleStatusChanged`).
   - What settles it: add to the report, per event, the passed scenario that emits it.

**What I checked and found right.**
- **Coverage:** I extracted 58 promises: 16 epic outcome items, 3 open questions, 5 acceptance items, the 32 `pending slice 11` requirements and 2 waves entries. 55 trace to a story body. 3 rest on the report or the coordinator only: open question 3, acceptance 3's AGENTS.md and README.md numbers, and the headline `slot task check`. Nothing in the set reaches beyond the epic.
- **Case and requirement counts:**
  - `lists.sh` prints `slice 1281 lists 1281 incoming 0` and exits 0.
  - Cases sum to 1,279 plus 2, and runs to 1,830 plus 3, which equals 1,833. The base outcomes confirm the per-list case and run counts.
  - Failure causes per list match the stories (847, 45 and 2 runs for dynamic-import).
  - The 44 passing runs and 1,789 failing runs are right.
  - The bounds 987, 1137, 985, 1178, 420, 538, 6 and 87 are right.
  - The 32 requirements split 13, 11, 1, 2, 1, 3 and 1, and HOST-224 is the next free number.
- **Counts in the stories:** the test counts (13, 13, 9, 7, 5, 5, 10, 3 and 9) match the lists. The 4 slice-12 `2nd-param-*` cases are slice 12 in `runs.tsv`. The 2 stale "slice 11" comments in Map and Set and the later-slice token scan came out clean. The HOLDS and refusal greps are consistent.

**What I could not establish.**
- Whether the 22 passing import-call cases and the 130 failing ones stay the same after order 1, since I ran no build.
- Whether the ESS acceptance's `more passed, fewer unsupported` is reachable without editing a scenario.
- Out of my lane: the "files that make a frame" scope entry in `story:dynamic-import` (parallel-safety), and the order-5 shared hunks of `module.rs` (parallel-safety).

**Open questions.**
- Q1 A, Q2 A, Q3 A, Q4 A, Q5 A.
- Q6 A.
- Q7 B, but only with finding 2 settled (the epic sentence rewritten and the decision held in the store).
- Q8 A.
- Q9 A. A design precondition is the convention in this store (for example `story:math-random`).
- Q10 A.

```findings
- file: .engineering/planning/story/module-namespace-objects.md
  line: 75
  category: scope
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the acceptance prints `bound 834 B 764` but over the join's base run the nine linking-list cases the part pulls in by ./ already pass, so the unit reads `B 773 bound 843`; state the join's numbers or the formula"
- file: .engineering/planning/story/dynamic-import.md
  line: 115
  category: scope
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the acceptance prints `bound 782 B 298` but at the order-6 join the 11 extra cases of the part (nine of the linking list, two of the namespace list) pass, so the unit reads `B 309 bound 793`; state the join's numbers or the formula"
- file: .engineering/planning/epic/slice-11-modules.md
  line: 80
  category: scope
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the epic's sentence 'The command-line host and the runner provide their own loaders.' has no story, the decision (Q7) lives only in the scratch report, and the close story's coordinator list does not carry the rewrite of that sentence"
- file: .engineering/planning/story/dynamic-import.md
  line: 101
  category: scope
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the body names story:module-determinism-faults-and-emulated-loading (here and at line 109), which does not exist; the story is story:module-determinism-and-emulated-loading, and the same wrong id stands in ess-target-module-commands.md lines 47 and 55 and module-linking-and-evaluation.md line 89"
- file: .engineering/planning/story/slice-11-conformance-close.md
  line: 44
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the epic's open question 'Which of the 39 features with no swell-slices line fall inside this slice' (epic line 48) is answered by no story body; globalThis (11 cases) and hashbang (1) have no feature line, and slice 9's close carries its answer"
- file: .engineering/planning/story/import-call-and-import-meta-syntax.md
  line: 71
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the body cites statement.rs lines 244 and 245 for the import arm's Dot and LParen branches; on f962c324 they are 245 and 246 and the declaration branch is 247, and module-syntax-and-early-errors.md line 71 repeats 244, 245 and 246 against its own 245 to 247 in the Change bullet"
- file: .engineering/planning/story/ess-target-module-commands.md
  line: 116
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "epic acceptance 5 asks a passed scenario for the module commands and events, and the acceptance names the seven commands but none of the four events (ModuleParsed, ModuleLoadRequested, ModuleLoadEnded, ModuleStatusChanged)"
```
