---
format: aep.planning-md/3
id: review-result:slice-11-round-1-acceptance
kind: review-result
status: active
title: 'Slice 11, round 1, the acceptance critic: needs-revision, 12 findings'
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
Plan critic, acceptance, round 1 on the thirteen draft stories of
`epic:slice-11-modules` (`plan/slice-11` at `c3021d72`), 2026-11-28.
Verdict: needs-revision, 12 findings; revision 1 answers them. The critic's answer, as returned:

needs-revision

1. `story:slice-11-conformance-close` (`.engineering/planning/story/slice-11-conformance-close.md:70`, check (6)) — blocking. The check can never exit 0 once one case moves out or comes in.
   - Check (6) runs `sh docs/plan/slice-11/lists.sh .out/test262/runs.tsv`, the run after the close, and expects `slice <c> lists <m> incoming 0` with "the cases that come in … on no list".
   - `lists.sh` counts every slice-11 case of the run in `n`. A come-in case on no list prints `missing` and exits 1. A moved case on `close-later.tsv` is no longer counted in slice 11, so it prints `outside` and exits 1. The story's own Change says the close adds `close-later.tsv` lines and `close-into-11.tsv` lines.
   - The slice 9 close ran `lists.sh` over `.out/test262-base/runs.tsv` (the join before the close) and expected `incoming 0`.
   - Settle it: run (6) over `.out/test262-base/runs.tsv` and state `slice 1281 lists 1281 incoming 0`. Or state the exact `missing` and `outside` lines allowed and drop "exits 0".

2. `story:module-namespace-objects` (`module-namespace-objects.md:75`) and `story:dynamic-import` (`dynamic-import.md:115`) — blocking. The printed line they state is not what `check.sh` prints on the base they tell the unit to use.
   - Both say to use "the base run of the join the unit starts from", yet quote numbers measured on `f962c324`.
   - Each part pulls in cases of earlier stories. I rebuilt both parts with `part.sh` into scratch (`part-module-namespace-objects.files`, `part-dynamic-import.files`):
     - The namespace part holds 9 cases of the linking list (`instn-iee-*`) and 1 of the dynamic-import list (`verify-dfs.js`).
     - The dynamic-import part holds 9 linking cases and 2 namespace cases (`instn-star-*`).
   - After order 4 and order 5 those extras pass, so on the join the unit's base gives:

     | Story | Stated | Actual on the join |
     |---|---|---|
     | namespace | `bound 834 B 764` | `B 773 bound 843` |
     | dynamic-import | `bound 782 B 298` | `B 309 bound 793` |

   - The unit cannot match the stated line, and "P at least 834" and "P at least 782" are weaker than what `check.sh` enforces.
   - Settle it: state the join figures, or say these two lines are read over the `f962c324` base. In the stories of orders 3 and 4 the extras still fail at their join, so their lines are right.

3. `story:module-records-loading-and-module-runs` (`:105`) — blocking. `cargo nextest run --locked -p swell-test262 --test main` exits 0 on the base, so it passes with the new runner tests unwritten.
   - Change names `a_module_case_with_a_parse_negative_passes_on_its_parse` and `a_fixture_is_loaded_and_is_no_case`, and 3 fixture cases.
   - Settle it: name the 2 new tests of `run::`, the same way the swell tests are counted, plus a count of the fixture cases.

4. `story:module-linking-and-evaluation` (`:83`) — blocking. Same defect as 3. The two runner tests are `a_module_graph_links_and_runs` and `a_resolution_negative_passes_on_the_link_error`, plus 3 fixture cases.

5. `story:json-modules` (`:77`) — blocking. Same defect as 3. The runner test is `a_json_module_loads_its_value`, plus 2 fixture cases and 2 fixture files.

6. `story:dynamic-import` (`:113`) — blocking. Same defect as 3. The runner test is `a_script_imports_a_module`, plus 1 fixture case.

7. `story:module-syntax-and-early-errors` (`:83`) — blocking. The acceptance never counts the new tests.
   - `modules::` of `swell-syntax` exits 0 without being written. Change gives "one test per form … one per early error" and no number.
   - The same holds for the two sources moved to `early-errors/` (same line, "`sources::` with the two moved sources").
   - `grep -c '", 11)' … prints at least 2` can be exactly 2. Story 2 states exactly 4.
   - Settle it: name the number of tests in `modules::`, name the 2 moved sources, and make the grep `2`. This is the slice 8 and 9 finding "the filter exits 0 without the new module".

8. `story:import-call-and-import-meta-syntax` (`:90`) — blocking. Same defect as 7. It names 6 tests of `import_call::`, 5 new sources and 2 moved sources, and the acceptance counts none of them. `sources::` and `import_call::` pass without them.

9. `story:module-records-loading-and-module-runs` (`:105`) — note. "the 13 tests of `modules::` and `misuse::host_153_...`" reads as 13 or 14. Change lists 13 in `modules::` plus `host_153`. Write "13 in `modules::` and 1 in `misuse::`".

10. `story:module-records-loading-and-module-runs` (Change, entry-points bullet) — note. `parse_module` of a held source is "a refusal counted by `Agent::refusals`" (`engine.md` section 11), but none of the 13 tests or the acceptance reads that count. Add one test, or drop the claim.

11. `story:import-meta` (Change, compiler bullet; acceptance `:85`) — note. The new `crates/compiler/tests/sources/module-import-meta.{js,disasm}` has no `ls | wc -l`. `story:dynamic-import` has the same with 2 sources. `story:module-code-compiles` does state its count. The 6 cases and `import_meta::` still show the transition.

12. `story:import-call-and-import-meta-syntax` (`:54`) — note. The Context breakdown "116 generated … 19 others, 12 noStrict, 1 onlyStrict" does not reproduce. The list has 143 cases in `dynamic-import/syntax/invalid/`, and 126 files carry the generated header (124 in `invalid/`, 114 of them without `noStrict`). The total 148 and every acceptance number are right. Fix the sentence or drop it.

**Checked and found right**
- Every `passed … bound … B … F …` line of orders 1 to 4 and 5 to 6 except finding 2, recomputed from `runs.tsv` with the real `part.sh` output. The part sizes match: 987, 1137, 1190, 423, 844, 538, 6, 87, 495.
- Lists and cause counts, requirement counts and test counts.
  - `lists.sh` prints `slice 1281 lists 1281 incoming 0` and exits 0; the list sums are 1,279 + 2 cases and 1,830 + 3 runs.
  - Cause counts for dynamic-import (847+45+2) and for the other lists; the close figures 1789, 44, 23 and `check (5)`.
  - The 32 pending lines equal 13+11+1+2+1+3+1; HOST-224 is free; the greps print 6, 0 and 2 on the base.
  - Named test counts: modules 13 in records, 13 in linking, 9 in namespace, 7 in tla, 5 in import-meta, 5 in json.
  - ESS greps; all 13 stories carry the time bounds, no `unsafe`, no Python.
- Not run: nothing was built or tested, and `check.sh` was run only on empty and missing inputs, which exit 1 as its header says.

**Open questions**
- Q4: A. One step per module visited gives tests that reach the step budget: `a_graph_of_many_modules_ends_in_step_budget`, `a_long_chain_of_modules_ends_link_in_step_budget`, `gather_available_ancestors_of_many_parents_ends_in_step_budget`. B leaves nothing to observe.
- Q6: A. The records acceptance tests (`host_59`) and the loader fixtures already assume the load finishes inside the hook.
- Q7: B. No story or acceptance carries a command-line loader.
- Q8: A. The acceptance names `grep -c '^HOST-224 test '` and `host-requirements-check`, which needs the design line first.
- Q9: A. Four acceptances cite "the paragraph Modules"; under B they cite nothing.
- Q10: A, no acceptance view.
- Q1, Q2, Q3, Q5: no view (design).

Out of my lane:
- `story:import-meta` needs HOST-224 in `host-interface.md` before it starts. `host-requirements-check` reads the design's lines.
- The records and linking stories need the "Modules" paragraph of `engine.md` first.
- Both dependencies are not recorded as relations.
- Slice 12 and slice 11 both append to `bytecode.rs`, `dispatch.rs` and `collect.rs`.
- The records and linking stories leave `conformance/host/surface.txt` and `CHANGELOG.md` to the coordinator, while `host-surface-check` needs each new public name named.

```findings
- file: .engineering/planning/story/slice-11-conformance-close.md
  line: 70
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "check (6) runs lists.sh over .out/test262/runs.tsv (the run after the close) and expects exit 0 with the cases that come in on no list, but lists.sh prints missing for a come-in case and outside for a moved case and exits 1; run it over .out/test262-base/runs.tsv as the slice 9 close did, or state the allowed lines"
- file: .engineering/planning/story/module-namespace-objects.md
  line: 75
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the stated line 'bound 834 B 764' is the f962c324 reading, but the unit reads the base of the join, where the 9 linking cases and verify-dfs in the part change the line to 'B 773 bound 843'; state the join figures"
- file: .engineering/planning/story/dynamic-import.md
  line: 115
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the stated line 'bound 782 B 298' is the f962c324 reading, but on the join the 11 extra cases (9 linking, 2 namespace) pass, so check.sh prints 'B 309 bound 793'; state the join figures"
- file: .engineering/planning/story/module-records-loading-and-module-runs.md
  line: 105
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "cargo nextest run -p swell-test262 --test main exits 0 on the base and names no count, so it passes without the 2 new runner tests and 3 fixture cases of Change; name the count"
- file: .engineering/planning/story/module-linking-and-evaluation.md
  line: 83
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "swell-test262 --test main exits 0 without a_module_graph_links_and_runs and a_resolution_negative_passes_on_the_link_error and the 3 fixture cases; name the count"
- file: .engineering/planning/story/json-modules.md
  line: 77
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "swell-test262 --test main exits 0 without a_json_module_loads_its_value and the 2 fixture cases; name the count"
- file: .engineering/planning/story/dynamic-import.md
  line: 113
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "swell-test262 --test main exits 0 without a_script_imports_a_module and its fixture case; name the count"
- file: .engineering/planning/story/module-syntax-and-early-errors.md
  line: 83
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the swell-syntax --test main run names no count of the new modules:: tests or of the 2 moved sources, so it passes with none written, and the HOLDS grep says 'at least 2' where 2 is exact"
- file: .engineering/planning/story/import-call-and-import-meta-syntax.md
  line: 90
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the swell-syntax --test main run names no count of the 6 import_call:: tests, the 5 new sources and the 2 moved sources, so it passes with none written"
- file: .engineering/planning/story/module-records-loading-and-module-runs.md
  line: 105
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "'the 13 tests of modules:: and misuse::host_153_...' reads as 13 or 14; Change lists 13 in modules:: plus host_153, so write 13 and 1"
- file: .engineering/planning/story/module-records-loading-and-module-runs.md
  line: 95
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Change says parse_module answers a held construct as a refusal counted by Agent::refusals but no test and no acceptance reads that count"
- file: .engineering/planning/story/import-meta.md
  line: 85
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the new compiler source module-import-meta.js and its .disasm have no ls count, so swell-compiler --test main exits 0 without them (dynamic-import has the same for its 2 sources)"
- file: .engineering/planning/story/import-call-and-import-meta-syntax.md
  line: 54
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the Context breakdown '116 generated, 19 others, 12 noStrict, 1 onlyStrict' does not reproduce: invalid/ holds 143 cases of the list and 126 files carry the generated header; the total 148 is right"
```
