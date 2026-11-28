---
format: aep.planning-md/3
id: review-result:slice-11-round-2-parallel-safety
kind: review-result
status: active
title: 'Slice 11, round 2, the parallel-safety critic: needs-revision, 9 findings (3 blockers)'
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
Plan critic, parallel-safety, round 2 on the fifteen draft stories of
`epic:slice-11-modules` (`plan/slice-11` at `e1eaf39a`), 2026-11-28.
Verdict: needs-revision, 9 findings, 3 of them blockers (matches without a wildcard outside the Writes of three stories); revision 2 answers them. The review ends with the files each story shares with a story of slice 12. The critic's answer, as returned:

needs-revision

**Round 1 (parallel-safety critic)**
1. settled: `module-records-loading-and-module-runs.md` line 124 and `module-linking-and-evaluation.md` line 79 split the ScriptOrModule handle and the import slot kind between records and linking.
2. open: `top-level-await.md` lines 58 and 73 name their parser and compiler hunks, but the compiler part leaves two files out. See N3.
3. settled: `module-syntax-and-early-errors.md` makes the held-sources test goal-aware, and the lifting stories write no file of `crates/syntax/tests/`.
4. settled: `module-namespace-objects.md` line 62 and `top-level-await.md` line 67 state the HOLDS merge rule. Both merge orders give the same text (see the Point 1 table).
5. settled: `crates/swell/src/module/` is a directory and `module/mod.rs` has one writer per `mod` line.
6. settled: no story's scope or Writes names `website/data/status.json` or a generated `tests/main.rs`.
7. settled: the story ids are right in all 5 places.
8. settled: the close checks its lists with `lists.sh` and expects `slice 1281 lists 1281 incoming 0`.
9. settled: `story:modules-design-text` exists, with `depends_on` from compiles, records, linking, top-level-await and import-meta.

**Point 1: shared files of orders 5 and 6**

| File | Hunks | Result |
|---|---|---|
| `parser/mod.rs` HOLDS | namespace deletes line 1, top-level-await line 2, import-meta line 4, dynamic-import line 3 | Lines 1 and 2 are adjacent. Both stories state "the branch merged second takes both deletions", and the result is the same text in either order. Line 4 is two lines from line 2. Line 3 is dynamic-import's alone, and it runs after import-meta. |
| `parser/expression.rs` | top-level-await in `prefix_operators` near line 771, import-meta at lines 1222–1230, dynamic-import's `Keyword::Import` hunk | Apart. |
| `compiler/expression.rs` | top-level-await at the `await` check near line 1233, import-meta and dynamic-import at the arm beside line 342 | Apart. The import-meta and dynamic-import hunks sit in the same area but are serial by `depends_on`. |
| `requirements.txt` | json-modules line 64, dynamic-import lines 65 and 66, import-meta lines 68, 158 and HOST-224, top-level-await line 99, determinism line 159 | Apart within order 5. Line 64 against lines 65/66 is adjacent, but dynamic-import `depends_on` json-modules, so they merge in sequence. Apart within order 6. |

No two units write the same lines without a stated rule.

**New findings**

1. `story:module-namespace-objects`, Change and Writes. blocking.
   - What is wrong: the story adds `ObjectKind::ModuleNamespace`, and its Writes say `ObjectKind` and `KindTag` are matched without a wildcard "in `object/mod.rs` and `ops/objects.rs`". A third exhaustive match exists.
   - Evidence: `crates/swell/src/object/array.rs` lines 326–355 match `&record.kind` with no wildcard, so the engine does not compile after the variant is added. That file is in no scope. The test `mark_phase_calls_every_site_of_the_layout` in `heap/collect.rs` builds one object of every kind (the `kinds` array from line 545). It needs a `for_test` constructor and an entry, beyond the "one line of `KIND_SITES`" the story states.
   - To settle: put `object/array.rs` in Writes with its arm, widen the `collect.rs` hunk to the test's `kinds` array, and add the constructor to `module_namespace.rs`.

2. `story:module-linking-and-evaluation`, import slot kind (Change line 79, Writes line 93). blocking.
   - What is wrong: the new `SlotValue` variant and `SlotKind::Import` break exhaustive matches outside the Writes line. The "makes the smallest change there and reports it" clause is open-ended. The coordinator cannot turn it into edges.
   - Evidence, `SlotValue` matches with no wildcard:
     - `env/global.rs` lines 256–262
     - `interp/private.rs` lines 76–77
     - `interp/class.rs` lines 516–519, 542–545 and 564–566
     - `interp/parameters.rs` line 32
   - Evidence, `SlotKind` matches with no wildcard: `dynamic_code.rs` lines 853–863.
   - Ownership: `class.rs`, `parameters.rs` and `dynamic_code.rs` are in the records story's Writes, so they need the linking story to touch them after order 3. `env/global.rs` and `interp/private.rs` are in no scope.
   - To settle: list these files and the arms in linking's Writes, or state the arms in records' ScriptOrModule hunks. Delete the catch-all sentence.

3. `story:top-level-await`, compiler hunk (line 58) and Writes (line 73). blocking.
   - What is wrong: the story says a module body is "compiled as an asynchronous body, with the instructions an `async` function's body uses; the module's `Code` says so ([[HasTLA]])". Three needed places are outside its Writes.
   - Evidence:
     - `async_slot` is reserved by `compiler/resolve.rs` lines 341 and 665–669, only for functions. The module's `locals` come from `resolution.script_locals` (`lib.rs` line 310). The module scope is in `module-code-compiles`.
     - `ModuleCode` in `module-code-compiles.md` line 83 has no HasTLA field. Adding one means `bytecode.rs`, and `disasm.rs` prints it.
     - `c.asynchronous = Some(slot)` and the `AsyncStart` emission are in `function.rs` line 236, which the story does not mention.
   - To settle: add `bytecode.rs`, `disasm.rs`, `resolve.rs` and the function that starts the module body (in `lib.rs` or `function.rs`) to Writes, with their hunks. Or move the HasTLA field and the async slot into `module-code-compiles` (order 2). The `bytecode.rs` hunk then needs a stated place beside the `ImportMeta` and `ImportCall` variants.

4. `story:json-modules`, Writes line 72. warning.
   - What is wrong: the Writes say "Not written: `crates/swell/src/builtins/json/`" and also "if its parser has no entry a module can call, the unit adds the smallest one". The entry is missing, so the exception is certain.
   - Evidence: `builtins/json/parse.rs` line 198 is `pub(super) fn json_parse(&mut self, args: &[Value])`, visible only inside `builtins/json/` and taking arguments held by a builtin activation. The slice-12 story `bigint-json-number-and-joined-cases` writes `builtins/json/mod.rs` and `parse.rs`.
   - To settle: name `builtins/json/parse.rs` (and `mod.rs` if needed) in Writes with the entry's signature. The coordinator sets an edge to the slice-12 story.

5. `story:cli-module-loader`, scope and Writes. warning.
   - What is wrong: the story adds the flag `--module`, and `website/docs/embedding.md` (lines 241–250) tables every flag, `-e` and `-h` included. It is not in scope.
   - Evidence: `website/docs/embedding.md` lines 226–250. The embedding.md also states the usage line `swell [flags] script.js`.
   - To settle: add `website/docs/embedding.md` (the flag row and the usage line) to scope as a cited path.

6. `story:module-namespace-objects`, the prototype-chain walk. warning.
   - What is wrong: a namespace can be the prototype of another object, and an ordinary walk then reaches its own [[Get]], [[Set]] and [[HasProperty]]. The change dispatches the eleven methods but not this walk.
   - Evidence: `object/ordinary.rs` lines 331–337 `walk_hands_over_to` hands over only for `KindTag::Proxy` and `KindTag::Host` and ends in `_ => Ok(false)`. A namespace in the chain would be walked as ordinary, with no compile error to catch it. `ordinary.rs` is not in scope and the story has no test of it.
   - To settle: add `object/ordinary.rs` (the arm for the new tag) to Writes, with a test of `Object.create(ns)`. Or state in the story that the walk treats a namespace as ordinary and why that is correct.

7. `story:module-code-compiles`, Writes line 100. warning.
   - What is wrong: the story says "a new field breaks no reader" and writes `bytecode.rs`, `disasm.rs` and `resolve.rs`. Nothing names the `FunctionTemplate` and `Code` consumers in `swell` for a module-body function. The records and linking stories cover them only through the catch-all clause.
   - Evidence: `Code` is built only by the compiler (grep result in the story). The `module: Option<ModuleCode>` field has no reader outside `module/`.
   - To settle: a sentence naming the readers (`module/mod.rs` of records, `link.rs` of linking). No file moves.

8. `story:ess-target-module-commands`. note.
   - It shares `target.rs`, `planted_defect.rs` and `taskfiles/conform-engine.yml` with the draft `story:spec-scenarios-for-the-numbers-and-dates-requirements`, outside the slice. The coordinator needs an edge there, as report-r1 already says.

9. `story:module-linking-and-evaluation` and `story:module-records-loading-and-module-runs`. warning.
   - What is wrong: the linking story changes `module/json.rs` (the four `SyntheticState` methods) and one expectation in `tests/modules.rs`, files the records story created. Records' Writes do not say that the hunks are apart.
   - Evidence: the Writes lines of both stories. They are serial (`depends_on`), so no conflict arises.
   - To settle: one sentence in the records story, "linking adds the four methods after the empty `SyntheticState`".

**Checked**
- Instruction variants (`ImportMeta`, `ImportCall`) land in the three exhaustive places (`bytecode.rs` `target()`, `disasm.rs`, `dispatch.rs`), and both stories write all three. The hold test `held_sources_answer_their_first_construct_still_held` stays with order 1.
- The new `ObjectKind` tag is covered in `object/mod.rs` and `ops/objects.rs`; a wildcard arm at `ops/objects.rs` line 605 and `host_overrides.rs` line 325 covers the rest.

**Counts**
- Stories read: 15 (`slice-11-conformance-close` was only grepped).
- Scope paths: cited 130, inferred 77, unplaced 0. The `waves` tool compares paths only, so it still splits orders 5 and 6.
- Acceptance reads traced: the `HOLDS` grep (`2`), the `lists.sh` check, the four `run::` counts, `refused/` 9→7→5, and the `git log ... -- crates spec` and `-- crates/swell/src` empty checks of ess and determinism. All trace to a file and a line.

**Not established**
- Whether the merge rule for HOLDS lines 1 and 2 works in the tooling. I read the stated rule and checked that either order gives the same text. I did not run a merge.
- Whether `module-with-top-level-await` passes the ESS gate unedited (the story says it is unverified).
- Whether `private.rs` and `global.rs` need only arms or also tests.

```findings
[
{"file":"crates/swell/src/object/array.rs","line":331,"category":"parallel-safety","severity":"blocker","verdict":"needs-revision","origin":"introduced","message":"Exhaustive match on ObjectKind with no wildcard (lines 326-355) breaks when ObjectKind::ModuleNamespace is added; the file is in no Writes. The collect.rs marking test also needs a kinds entry and a for_test constructor, beyond one KIND_SITES line."},
{"file":".engineering/planning/story/module-linking-and-evaluation.md","line":93,"category":"parallel-safety","severity":"blocker","verdict":"needs-revision","origin":"introduced","message":"New SlotValue variant and SlotKind::Import break exhaustive matches in env/global.rs 256-262, interp/private.rs 76-77, interp/class.rs 516-566, interp/parameters.rs 32 and dynamic_code.rs 853-863; global.rs and private.rs are in no scope, the others are records' files; the catch-all 'smallest change' clause names no file."},
{"file":".engineering/planning/story/top-level-await.md","line":58,"category":"parallel-safety","severity":"blocker","verdict":"needs-revision","origin":"introduced","message":"The module body's async slot (resolve.rs 341, 665-669; function.rs 236), the HasTLA field of ModuleCode (bytecode.rs, disasm.rs) and the module's AsyncStart emission are outside Writes; ModuleCode in module-code-compiles has no HasTLA field."},
{"file":"crates/swell/src/builtins/json/parse.rs","line":198,"category":"parallel-safety","severity":"warning","verdict":"needs-revision","origin":"introduced","message":"json_parse is pub(super) and takes builtin args; the story says builtins/json/ is not written and also that the unit adds an entry. File should be named; slice-12 bigint-json-number-and-joined-cases writes parse.rs and mod.rs."},
{"file":"website/docs/embedding.md","line":241,"category":"parallel-safety","severity":"warning","verdict":"needs-revision","origin":"introduced","message":"The flag table lists every flag of the shell; the --module flag adds a row and the usage line changes; the file is not in scope."},
{"file":"crates/swell/src/object/ordinary.rs","line":331,"category":"parallel-safety","severity":"warning","verdict":"needs-revision","origin":"introduced","message":"walk_hands_over_to ends in a wildcard, so a namespace in a prototype chain is walked as ordinary with no compile error; the file is not in Writes and no test covers it. Inferred surface."},
{"file":".engineering/planning/story/module-code-compiles.md","line":100,"category":"parallel-safety","severity":"warning","verdict":"needs-revision","origin":"introduced","message":"Readers of the new Code.module field in swell are not named (inferred: module/mod.rs and link.rs); no file moves."},
{"file":".engineering/planning/story/module-linking-and-evaluation.md","line":93,"category":"parallel-safety","severity":"warning","verdict":"needs-revision","origin":"introduced","message":"Linking edits module/json.rs and one expectation in tests/modules.rs, files records creates; records' Writes do not say the hunks are apart. Serial by depends_on."}
]
```

**Point 3: files each slice-11 story shares with a slice-12 story**

The slice-12 story is on `plan/slice-12` at `0a1577f2`. `V` is `bigint-values-and-literals`, `A` is `bigint-arithmetic-and-bitwise-operators`, `J` is `bigint-json-number-and-joined-cases`, `C` is `slice-12-conformance-close`, `N` is `swell-num-bigint-core`.

| Slice-11 story | Shared files | Slice-12 story |
|---|---|---|
| `modules-design-text` | `docs/design/engine.md` | C, N |
| | `conformance/host/requirements.txt` | V (line 219), A (lines 23, 24) |
| `module-syntax-and-early-errors` | `syntax/src/ast.rs`, `lexer.rs`, `parser/mod.rs` | V |
| `import-call-and-import-meta-syntax` | `syntax/src/ast.rs`, `parser/mod.rs`, `compiler/src/expression.rs`, `resolve.rs` | V |
| `module-code-compiles` | `compiler/src/bytecode.rs`, `disasm.rs`, `lib.rs`, `resolve.rs` | V |
| `module-records-loading-and-module-runs` | `heap/collect.rs`, `heap/mod.rs`, `host/mod.rs`, `value.rs` | V |
| | `interp/dispatch.rs`, `requirements.txt` | V, A |
| `module-linking-and-evaluation` | `host/mod.rs` | V |
| | `interp/dispatch.rs`, `requirements.txt` | V, A |
| `module-namespace-objects` | `heap/collect.rs`, `syntax/src/parser/mod.rs` | V |
| `top-level-await` | `syntax/src/parser/mod.rs`, `compiler/src/expression.rs`, `lib.rs` | V |
| | `requirements.txt` | V, A |
| | after N3 is fixed: `bytecode.rs`, `disasm.rs`, `resolve.rs` | V |
| `import-meta` | `syntax/src/parser/mod.rs`, `compiler/src/{bytecode,disasm,expression,resolve}.rs` | V |
| | `interp/dispatch.rs`, `requirements.txt` | V, A |
| `json-modules` | `host/mod.rs` | V |
| | `requirements.txt` | V, A |
| | after N4 is fixed: `builtins/json/parse.rs` | J |
| `dynamic-import` | `syntax/src/parser/mod.rs`, `compiler/src/{bytecode,disasm,expression,resolve}.rs` | V |
| | `interp/dispatch.rs`, `requirements.txt` | V, A |
| `module-determinism-and-emulated-loading` | `conformance/host/requirements.txt` only | V, A |
| `slice-11-conformance-close` | `conformance/test262/{expected.txt,expected.digest,later-parts.tsv,moves.tsv}` | C |
| | `docs/design/engine.md` | C, N |
| `ess-target-module-commands` | none | |
| `cli-module-loader` | none | |

- `V` shares 14 files with slice 11. The decomposer counted 16. The difference is directory entries under `crates/syntax/tests/sources/early-errors/`: V adds files of other names there, so I list no overlap.
- Edges for the coordinator: V and A come after every slice-11 story in the table, each in its own hunks. J comes after `json-modules`. C and N come after `modules-design-text` and `slice-11-conformance-close`.
- `requirements.txt` hunks are apart in every pair: slice 11 moves existing `pending slice 11` lines, V and A move lines 219, 23 and 24, and HOST-224 is appended at the end.

Review files read, all under `<trees>/swell-plan-slice-11/.engineering/planning/story/`: the 15 story files of slice 11, with `import-meta.md` and `top-level-await.md` read before the last summary. My scratch tables are in `<scratch>/wave3/plan-11/`.
