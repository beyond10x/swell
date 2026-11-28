---
format: aep.planning-md/3
id: review-result:slice-11-round-2-acceptance
kind: review-result
status: active
title: 'Slice 11, round 2, the acceptance critic: needs-revision, 3 notes'
relations:
- reviews: story:cli-module-loader
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
- reviews: story:modules-design-text
- reviews: story:module-syntax-and-early-errors
- reviews: story:slice-11-conformance-close
- reviews: story:top-level-await
revision: 1
---
Plan critic, acceptance, round 2 on the fifteen draft stories of
`epic:slice-11-modules` (`plan/slice-11` at `e1eaf39a`), 2026-11-28.
Verdict: needs-revision, 3 notes, the 12 findings of round 1 settled; revision 2 answers them. The critic's answer, as returned:

needs-revision

All 12 findings of round 1 are settled. The three new findings are notes: none blocks, but each would change the text, so the verdict is `needs-revision`.

**Round-1 findings**
1. close check (6): settled, `slice-11-conformance-close.md:73` (now check (5)). It runs `lists.sh .out/test262-base/runs.tsv` and states that the moves and come-ins are read by (2) and (4). I ran `lists.sh` on the base run: `slice 1281 lists 1281 incoming 0`, exit 0.
2. namespace and dynamic-import bounds: settled, `module-namespace-objects.md:76` and `dynamic-import.md:104`. I rebuilt each part with `part.sh` into scratch and counted by owner list.
   - Namespace: 844 files = 70 + 764 + 9 linking + 1 dynamic. Join `bound 843 B 773`; on `f962c324`, `834 / 764`.
   - Dynamic-import: 794 files = 485 + 298 + 9 linking + 2 namespace. Join `bound 794 B 309`; on `f962c324`, `783 / 298`.
3. records runner tests: settled, `module-records-loading-and-module-runs.md:145`. `run::` runs 30 (`#[test]` count in `crates/test262/tests/run.rs` is 28 on the base). The 3 fixture cases are counted by a `find` that prints `3`.
4. linking runner tests: settled, `module-linking-and-evaluation.md:99`. `run::` runs 32 and the `find` prints `3`.
5. json runner test: settled, `json-modules.md:78`. `run::` runs 33; two `find` counts print `2` each.
6. dynamic-import runner test: settled, `dynamic-import.md:102`. `run::` runs 34 and the `find` prints `1`.
7. module-syntax counts: settled, `module-syntax-and-early-errors.md:95`.
   - The 16 tests named in Change count 16; `modules::` runs 16.
   - `held/module-*` prints 3, `refused/` goes 9 to 7, and the two moved sources are named.
   - The `HOLDS` grep prints exactly `2`.
   - The 6 grep on the base and the `refuse…, 11` count of 4 reproduce.
8. import-call counts: settled, `import-call-and-import-meta-syntax.md:96`. The `import_call::` count is 6, matching the 6 tests named. `held/` goes 25→28→31, `early-errors/` 396→398→404 and `refused/` 9→7→5; all reproduce on the tree.
9. records "13 and 1": settled, `module-records-loading-and-module-runs.md:145`. `misuse::` goes 8→9, and the base grep prints 8.
10. `Agent::refusals` claim in records: settled, Change entry-points bullet (`:125`). The claim is dropped, with the reason in the body.
11. `ls` counts for compiler sources: settled.
    - `import-meta.md:85`: sources 286→288, and the base count is 268.
    - `dynamic-import.md:102`: 4 files, 288→292.
    - `module-code-compiles.md:106`: 9 pairs, 268→286.
12. import-call Context breakdown: settled, `import-call-and-import-meta-syntax.md:56`. "143 of them under …/syntax/invalid/" reproduces: 143 in the list.

**New findings**

1. `story:cli-module-loader` (`cli-module-loader.md:49` Change, shell bullet; `:71` Acceptance) — note. Some behaviors the Change names are read by none of the 6 new tests, so the count of 22 passes without them. This is the shape of round-1 finding 10.
   - Evidence: the bullet gives a `TypeError` for a bare specifier and for a file that is not UTF-8, and the message `the module's evaluation did not settle` for a pending evaluation. The tests cover a missing file, a missing export, a syntax error, an import cycle with a nested import, `await` and `-e`. A request with an attribute "keeps the request as today", so its load promise stays pending, and the bullet states no exit status for that case.
   - Settle: add a test each for the bare specifier, the pending evaluation and the pending load (name the exit status and update the 22 and the 6), or drop those claims.
2. `story:module-code-compiles` (`module-code-compiles.md:84` Change; `:106` Acceptance) — note. The slot kind "import of a namespace" and the `export * as ns from` form cannot be shown by any source of this story. Both constructs are held until `story:module-namespace-objects` (order 5), and a held construct makes `parse_module` answer the refusal.
   - Evidence: `module-syntax-and-early-errors.md:73` (`namespace import` held at `import * as` and at `export * as ns from`) and `crates/syntax/src/parser/mod.rs` doc of `HOLDS`. The 9 sources and 3 tests name no namespace form.
   - Settle: say in the body that the namespace slot and `export * as` compile paths are written here and first shown by the namespace story's cases, so a unit does not write a source that parse refuses.
3. `story:modules-design-text` (`modules-design-text.md:58`) — note. The four greps count only the headings `**Modules.**` and `**Module records.**`, so a heading over one line passes. Nothing observable separates it from a paragraph that states the decisions Q1 to Q6, which are the story's purpose.
   - Settle: add greps for load-bearing names the paragraphs must state, such as `Item::Module`, `KIND_SITES`, `LoadPayload` and the import slot kind, each printing at least 1 inside `engine.md`.

**Checked and found right**
- I rebuilt all 9 parts and recomputed their B, F and bound lines: syntax `988/986/2`, import-call `1137/1007/130`, records `1190/985/193 → 1178`, linking `423/298/122 → 420`, namespace, tla `538/298/240`, import-meta `6/0/6`, json `87/77/10`, dynamic.
- `lists.sh` gives `1281 / 1281 / 0`. The run sums are 1,833 and the case sums 1,281. The 33 pending lines equal 13+11+1+1+3+3+1. The 1789 and 23-case/44-run figures of the close reproduce.
- The new-story counts reproduce on the tree: `swell-cli` 16 tests and 7 fixtures, `determinism::` 3→4, `emulated_host::` 5→7, planted-defect `#[test]` 2→3, and the ESS floor and maximum greps match line 86 and line 5 of `taskfiles/conform-engine.yml`.
- HOST-224 is free, and the anchors of `modules-design-text` (lines 1999, 2419, 3561, 975, 223) are right.
- All stories carry the time bounds, no `unsafe` and no Python. `aep plan artifact validate` prints `valid`.

**Could not establish**
- Whether `slot task suite` and `slot task conform-engine` fit the 10-minute pool bound. `ess-target-module-commands.md` names no `slot --timeout` for them, and I ran no build.
- Whether `.out/conform-engine.json` lists commands and events per scenario, which the ESS acceptance relies on.

Out of my lane:
- `module-syntax-and-early-errors.md:73` still says the held refusal is "counted by `Agent::refusals`", though no test reads it.
- 15 or more stories carry "Size: about N lines … one session", an effort-size statement that the owner's rule on time estimates forbids.

```findings
- file: .engineering/planning/story/cli-module-loader.md
  line: 49
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the Change names a TypeError for a bare specifier and a non-UTF-8 file, the message 'the module's evaluation did not settle', and a pending load promise for a request with an attribute, but none of the 6 new tests reads them and no exit status is given for the pending load, so the count of 22 passes without them; add a test each or drop the claims"
- file: .engineering/planning/story/module-code-compiles.md
  line: 84
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the slot kind 'import of a namespace' and the export * as ns from form are named in Change but both constructs are held until story:module-namespace-objects and parse_module answers the refusal, so no source or test of this story can show them; say that they are first shown by the namespace story's cases"
- file: .engineering/planning/story/modules-design-text.md
  line: 58
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the greps count only the headings '**Modules.**' and '**Module records.**', so a heading over one line passes and nothing observable shows that the paragraphs state the decisions Q1 to Q6; add greps for the names they must carry (Item::Module, KIND_SITES, LoadPayload, the import slot kind)"
```
