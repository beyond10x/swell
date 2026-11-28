---
format: aep.planning-md/3
id: review-result:slice-11-round-1-parallel-safety
kind: review-result
status: active
title: 'Slice 11, round 1, the parallel-safety critic: needs-revision, 9 findings'
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
Plan critic, parallel-safety, round 1 on the thirteen draft stories of
`epic:slice-11-modules` (`plan/slice-11` at `c3021d72`), 2026-11-28.
Verdict: needs-revision, 9 findings; revision 1 answers them. The critic's answer, as returned:

needs-revision

Findings 1 to 3 are blocking and findings 4 to 8 are notes.

1. **blocking.** Set (`story:module-linking-and-evaluation`, `story:top-level-await`, `story:import-meta`, `story:dynamic-import`). This is a reader beside a producer, with the producer in the later story. The change that lets a frame run module code is scoped only in `story:dynamic-import` (order 6, cited).
   - **Where:** the stories that need it sit earlier. `module-linking-and-evaluation.md:66` says ExecuteModule "runs the body as an entry of its own frame". Its Writes line (`:77`) gives `interp/` only as "the named-slot instructions". `import-meta.md:70` reads "the active module's `import.meta` object". `top-level-await.md:56` reads "the frame's realm and its active module the module's". `dynamic-import.md:95` says "a frame of a module's body knows its module: the frame's `script: Handle` becomes a script-or-module handle". Its Writes line (`:107`) adds "and the files that make a frame" and `host/jobs.rs`.
   - **Evidence in the code:**
     - `crates/swell/src/interp/frame.rs:175` holds `script: Handle`, documented as "The ScriptOrModule component", and `:284` traces it as `Item::Script(self.script)`.
     - It is read at `arrow.rs:31`, `parameters.rs:98`, `class.rs:149` and `:311`, `lexical.rs:95`, `templates.rs:29`, `dispatch.rs:1439` and `:1596`, and `host/jobs.rs:247`.
     - The compiled code of any function made inside module code is found through that handle, so the ScriptOrModule component is already needed at order 4.
   - **Why the edges do not settle it:** an edge from `import-meta` or `top-level-await` to `dynamic-import` would close a cycle, because `dynamic-import` depends on both.
   - **What would settle it:** the ScriptOrModule component, its readers and the job's active module move into `module-linking-and-evaluation`, or into `module-records-loading-and-module-runs`. Their Writes lines and scopes name the files, and `dynamic-import` keeps only the referrer logic.

2. **blocking.** `story:top-level-await`, `:55`, `:61` and `:67`. Its surfaces are incomplete, and the files it leaves out collide with `story:import-meta` (inferred, except where noted).
   - **Await check (cited in the code):** its text "the check that admits `await` only in an `async` body" has no file. The check is `crates/compiler/src/expression.rs:1233` (`malformed("an await outside an async function")`) and `crates/compiler/src/statement.rs:866`. `import-meta.md:81` also writes `compiler/src/expression.rs`, and neither Shared paragraph names it (`:61` and `import-meta.md:75` name only `lib.rs`).
   - **Where the hold lives:** `story:top-level-await` deletes the hold "in `parser/module.rs` or wherever order 1 placed it". The `await` prefix is parsed in `crates/syntax/src/parser/expression.rs:742` and `:771`, which is where the hold call must stand. `import-meta` writes the same file. The order-1 story's Writes line (`module-syntax-and-early-errors.md:77`) has no `parser/expression.rs` either.
   - **What would settle it:** order 1 names where the hold call is. `top-level-await` lists the files and hunks (await check in `expression.rs` near 1233 and `statement.rs` near 866; the hold in its real file). `import-meta` names its own hunks in `expression.rs` and `parser/expression.rs`.

3. **blocking.** `story:module-syntax-and-early-errors`, `:43`, `:77` and `:83`, and `story:import-call-and-import-meta-syntax`, `:84`. The acceptance cannot pass, and a surface is missing from the scopes.
   - **What fails:** `crates/syntax/tests/sources.rs:309-315` fails when a line of `HOLDS` is named by no header under `tests/sources/held/` ("the hold … of HOLDS has no held source"). `sources.rs:301-303` parses each held source with the Script-goal `parse`.
   - **Why:** order 1 adds `("namespace import", 11)` and `("top-level await", 11)`. Order 2 adds `("import.meta", 11)`. All three constructs are module-only. Both stories test them inline in `tests/modules.rs` or `tests/import_call.rs` and add no held source.
   - **Result:** `cargo nextest run -p swell-syntax --test main` (acceptance, `:83`) fails on `held_sources_answer_their_first_construct_still_held`. The file `crates/syntax/tests/sources.rs` is in no scope.
   - **Collision:** the stories that lift a hold (`module-namespace-objects`, `top-level-await`, `import-meta`) would each edit that test or `held/`, with no named hunk.
   - **What would settle it:** order 1 owns the change to `sources.rs` (a held source with a goal header, or a module-aware path). Orders 1, 2 and 5 name `sources.rs` and `held/` in their Writes lines and Shared paragraphs.

4. **note.** `story:module-namespace-objects`, `:61` and `:67`. It shares `crates/syntax/src/parser/module.rs` and `crates/syntax/tests/modules.rs` with `story:top-level-await` (`top-level-await.md:54` and `:67`). The Shared paragraphs of both name only `parser/mod.rs` `HOLDS`.
   - The hunks differ: the `hold` calls of two constructs, and two renamed tests.
   - Both stories delete adjacent lines of the four-line `HOLDS` array (`namespace import` and `top-level await` are lines 1 and 2). Git conflicts on adjacent line edits. The `import.meta` deletion in `import-meta.md:68` sits away from the other two.
   - Name the files and the adjacent-line rule, as `namespace-objects` already does for `module.rs`.

5. **note.** Set, order 5 (`module-namespace-objects.md:61`). All four stories append their functions to `module.rs` "after the last function of the linking story, in the order namespace, top-level `await`, `import.meta`, JSON".
   - **Why it conflicts:** four branches cut from one join start at one anchor, so every pair conflicts at that point. The named order cannot hold in parallel. `aep plan artifact waves` already puts each story in its own wave (waves 6 to 9).
   - **Remedies, without choosing:** (a) order edges with `module.rs` as the reason; (b) the records story makes `crates/swell/src/module/` a directory, and each order-5 story writes its own file (`namespace.rs`, `async_eval.rs`, `import_meta.rs`, `json.rs`).
   - The decomposer admits this; a person decides.

6. **note.** Set. `website/data/status.json` is written by `top-level-await.md:67`, `import-meta.md:81`, `json-modules.md:71` (order 5), `dynamic-import.md:107` and `module-determinism-and-emulated-loading.md:57` (order 6).
   - No Shared paragraph names it. The waves tool lists it as a collision for all of those pairs (`aep plan artifact waves`).
   - It is a generated whole file, so any two branches conflict on it. Name it and state that the coordinator writes it again after each join (`slot --local task site-data`) and the unit's copy is dropped on conflict.

7. **note.** `story:ess-target-module-commands` (`:47`, `:55`), `story:dynamic-import` (`:101`, `:109`), `story:module-linking-and-evaluation` (`:89`). All three name `story:module-determinism-faults-and-emulated-loading`, which is not an artifact. The real id is `story:module-determinism-and-emulated-loading` (`grep` over the stories found only this missing id). The sibling named in the Shared paragraphs of order 6 does not resolve.

8. **note, out of my lane (acceptance).** `story:slice-11-conformance-close`, `:70` check (6). The lists-partition check cannot exit 0 once a case comes in.
   - **Evidence:** `docs/plan/slice-11/lists.sh`, END block: every case counted in slice 11 that is on no list prints `missing <path>` and sets `bad=1`. The text says the cases that come in "are on no list" and that `<m>` is `<c>` less the lines of `close-into-11.tsv`.
   - **What would settle it:** the cases that come in go on a list the script reads (`incoming.tsv`, as its own header describes), or check (6) says it exits 1 for them and names the expected `missing` lines.

9. **note (precondition not recorded as an edge).** `story:module-records-loading-and-module-runs`, `:70`; `story:import-meta`, `:50` and `:86`.
   - The design paragraph "Modules" in `engine.md` before order 3, and HOST-224 in `host-interface.md` before `import-meta`, are conditions in prose only. No blocker or task artifact exists and no edge points at one, so `waves` starts the records story in wave 4 with nothing before it.
   - File the design change as an artifact and relate it with `blocks` or `depends_on`, so the condition is machine-visible.

```findings
- file: .engineering/planning/story/module-linking-and-evaluation.md
  line: 66
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the frame that runs a module body (the ScriptOrModule component of crates/swell/src/interp/frame.rs:175, its readers and host/jobs.rs:247) is scoped only in story:dynamic-import (dynamic-import.md:95, :107) while this story (ExecuteModule), story:top-level-await (top-level-await.md:56) and story:import-meta (import-meta.md:70) need it first, and no edge can order them without a cycle; the change moves into this story or the records story"
- file: .engineering/planning/story/top-level-await.md
  line: 55
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the files of the await check (crates/compiler/src/expression.rs:1233, statement.rs:866) and of the hold call (crates/syntax/src/parser/expression.rs:742, :771) are unnamed in Writes and Shared, and story:import-meta writes the same compiler and parser expression files (import-meta.md:81)"
- file: .engineering/planning/story/module-syntax-and-early-errors.md
  line: 83
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "crates/syntax/tests/sources.rs:309-315 fails for a line of HOLDS with no held source and parses held sources in the Script goal, so the three module-only holds of orders 1 and 2 fail the acceptance, and sources.rs and held/ are in no story's scope although orders 1, 2 and 5 all touch them"
- file: .engineering/planning/story/module-namespace-objects.md
  line: 61
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "shares crates/syntax/src/parser/module.rs, crates/syntax/tests/modules.rs and adjacent lines of HOLDS with story:top-level-await (cited, both Writes lines) and neither Shared paragraph names them"
- file: .engineering/planning/story/module-namespace-objects.md
  line: 61
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "four stories of order 5 append to crates/swell/src/module.rs after the same last function, so each pair conflicts at one anchor and the named order of appends cannot hold in parallel; the plan names no edge and no split of the file"
- file: .engineering/planning/story/import-meta.md
  line: 81
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "website/data/status.json is written by five stories of orders 5 and 6 and no Shared paragraph names it or says the coordinator writes it again after the join"
- file: .engineering/planning/story/ess-target-module-commands.md
  line: 47
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "names story:module-determinism-faults-and-emulated-loading, which does not exist (real id story:module-determinism-and-emulated-loading); the same id stands in dynamic-import.md:101, :109 and module-linking-and-evaluation.md:89"
- file: .engineering/planning/story/slice-11-conformance-close.md
  line: 70
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "out of lane (acceptance): check (6) says lists.sh exits 0 with m equal to c less the lines of close-into-11.tsv, but lists.sh prints missing and exits 1 for every case that comes in and is on no list"
- file: .engineering/planning/story/module-records-loading-and-module-runs.md
  line: 70
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the design paragraph Modules (and HOST-224 for story:import-meta, import-meta.md:50) is a precondition stated in prose with no artifact and no edge, so the waves tool starts this story with nothing before it"
```

**What I read:** all 13 stories, the report, `waves-section.md`, `check.sh`, `part.sh`, `run-part.sh`, `lists.sh`, `scan.awk`, the slice 8 and 9 parallel-safety records, and `aep plan artifact waves` output. I ran `lists.sh` on the base (`slice 1281 lists 1281 incoming 0`, rc 0). I recomputed all nine `check.sh` bounds against `runs.tsv` (987, 1137, 1178, 420, 834, 538, 6, 87 and 782 match; the compile story's 985 is the guard size). I counted the pulled-in cases by `./` closure (12, 3, 10 and 11 match; 0 for top-level await, import-meta and json-modules). I counted the 32 pending requirements (13, 11, 1, 2, 1, 3 and 1; they sum to 32) and the close sums (1,279 and 2; 1,833 less 44 is 1,789). The token closure of each list found no case in a story's list that needs a part from outside that story's dependency closure.
**Surfaces and reads:** 13 stories cited, 0 inferred-only, 0 unplaced (the surfaces of the linking, top-level-await and order-1 stories are incomplete: findings 1 to 3). 24 acceptance reads traced to a producer in the set; 22 recorded by an edge or a path of edges. The 2 not recorded are the module frame and the active module, read by `top-level-await`, `import-meta` and `module-linking-and-evaluation` and produced by `dynamic-import`.

**What I could not establish:**
- Where order 1 places the `await` hold (the story names no file).
- Whether the order-5 pool jobs share one `.cache/test262` checkout on the node (`run-part.sh` runs `task test262-fetch` in each).
- Out of lane: the acceptance critic should read finding 3 (the failing held-sources test), and finding 8 is an acceptance equation.

**Open questions (Q1 to Q10 of the decomposer's report):**
- Q1: A. It adds one `heap/mod.rs` and `collect.rs` hunk in order 3 only; B or C would reopen the object and script arenas that slice 12 also edits.
- Q2: A. B adds an instruction at the tail of `bytecode.rs` and `dispatch.rs`, next to the ones `import-meta` and `dynamic-import` append.
- Q3: A.
- Q4: A.
- Q5: A, but only once finding 1 is settled: the module frame's ScriptOrModule component belongs to the linking or records story.
- Q6: A.
- Q7: B.
- Q8: A. Slice 12's plan names no `HOST-22x`, so no number collides.
- Q9: A, with the design change filed as an artifact and an edge (finding 9).
- Q10: A. B puts two order-1 stories on the `Keyword::Import` arm of `statement.rs`, `parser/mod.rs` and the generated `tests/main.rs`.
