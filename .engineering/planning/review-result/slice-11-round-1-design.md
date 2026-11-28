---
format: aep.planning-md/3
id: review-result:slice-11-round-1-design
kind: review-result
status: active
title: 'Slice 11, round 1, the design critic: needs-revision, 8 findings'
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
Plan critic, design, round 1 on the thirteen draft stories of
`epic:slice-11-modules` (`plan/slice-11` at `c3021d72`), 2026-11-28.
Verdict: needs-revision, 8 findings; revision 1 answers them. The critic's answer, as returned:

needs-revision

Findings (blocking unless marked note):

1. **`story:module-linking-and-evaluation` (with `story:import-meta`, `story:top-level-await`, `story:dynamic-import`) — blocking.** The ScriptOrModule component of frames, functions and environments is built in order 6 but read in orders 4 and 5.
   - **Where.** `dynamic-import.md:95` and `:107` say a frame's `script: Handle` becomes a script-or-module handle, "and the files that make a frame". The producer is therefore order 6.
   - **The readers.**
     - `module-linking-and-evaluation.md:66`, `:77`: ExecuteModule runs the body "as an entry of its own frame" and instantiates the module's functions. `env/` is not in its Writes.
     - `import-meta.md:70` reads "the active module's `import.meta` object".
     - `top-level-await.md:56` gives the module frame "its active module the module's".
   - **Evidence on the base.**
     - `crates/swell/src/interp/frame.rs:175` has `script: Handle`, marked as `Item::Script` at line 284.
     - `crates/swell/src/object/ecmascript_function.rs:66` has `script: Option<Handle>`, marked at line 124 as `Item::Script`.
     - `crates/swell/src/env/mod.rs:94` has `SlotNames.script`.
     - `crates/swell/src/host/jobs.rs:92` has the queued job's `script`.
     - The same handle is read in `interp/{parameters,arrow,class,lexical,dispatch,templates}.rs`.
     - A module record is not a script record, so none of these can hold a module before the generalisation lands.
   - **What settles it.** One owner at or before linking for the script-or-module handle at all those sites, with the files named in its Writes. Records is the natural owner (it adds `Item::Module`). Then `dynamic-import.md:95` only reads it.
   - **Same gap, Q2 default.** The default says an import slot "points to the target binding" and the named-slot instructions "read through it". That needs a new `SlotKind` and `SlotValue` in `env/mod.rs` (`SlotKind` is at line 101, `EnvironmentRecord.slots` at line 269), which no story scopes.

2. **`story:module-namespace-objects`, `story:top-level-await`, `story:import-meta`, `story:json-modules` — blocking.** The bodies prescribe an append order that no edge records, and `waves` prints a different order.
   - **Prescribed order.** `module-namespace-objects.md:61` says "in the order namespace, top-level `await`, `import.meta`, JSON". `import-meta.md:70` and `:75` say import-meta goes "after those of top-level `await`". `json-modules.md:59` and `:65` say JSON goes "after those of `import.meta`". `top-level-await.md:61` says its functions are "added after the namespace's".
   - **What `waves` prints.** Run from this worktree, `aep plan artifact waves` places import-meta in wave 36, json-modules in 38, module-namespace-objects in 39 and top-level-await in 43.
   - **Why it matters.** Import-meta runs first, and its text names functions that do not exist yet. No `depends_on` runs between the four stories.
   - **What settles it, as a trade-off on `crates/swell/src/module.rs`.**
     - (a) Record the chain with edges: ns, then TLA, then import-meta, then json. This costs no wave, since `waves` already serialises them.
     - (b) Put each part's functions in its own file or submodule, and drop the "after X" sentences.

3. **`story:module-records-loading-and-module-runs` — blocking.** The split between the record's fields and their readers fails the story's own clippy line.
   - **Where.** `module-records-loading-and-module-runs.md:74` and `:87` declare "every field" of the record "so that no story of order 5 adds a field".
   - **Evidence.** The gate runs `cargo clippy --workspace --all-targets --locked -- -D warnings` (`Taskfile.yml:1229`). The base already needs `#[expect(dead_code, reason = "modules: slice 11")]` on an unread field (`crates/swell/src/value.rs:102`).
   - **Why it fails.** `trace` reads only handle-bearing fields. The following are never read until orders 4 to 5: `has_tla`, `async_evaluation_order`, `pending_async_dependencies`, `async_parent_modules`, `top_level_capability`, `dfs_index`, `dfs_ancestor_index`, and the synthetic kind's default export. Records' acceptance line `slot task clippy exits 0` cannot hold as drafted.
   - **What settles it.** Either each unread field carries `#[expect(dead_code, reason = "story:<owner>")]`, named in the records story and deleted by its owner (an unfulfilled `expect` is itself a warning). Or each story adds the fields it reads.

4. **`story:slice-11-conformance-close` (also `docs/plan/slice-11/scan.awk`) — blocking.** `json-via-namespace.js` is owned by the one story that writes no engine code.
   - **Where.** The case is on `close-joint.cases.txt` (`scan.tsv:677`, parts `ns,json`). `scan.awk:163` sets `key = "joint"` when the highest-ranked part's story lacks another part.
   - **Why it is wrong.** The header (`scan.awk:2`) promises "the first story, in the order of the edges, to hold every part". `story:dynamic-import` holds both `ns` and `json` (`holds["dimp"]`) and is not offered.
   - **Consequence.** If the case fails, `slice-11-conformance-close.md:58` says a failure "goes back to the story whose list holds it". No list holds it, and the close writes no engine file.
   - **Second joint case.** `new-await-script-code.js` ("other", passes on the base) is checked only at the close. A regression from the Module goal or the `await` change is found after every story has landed.
   - **What settles it.**
     - Move `json-via-namespace.js` onto `story-dynamic-import.cases.txt`; its bound becomes 783 (485 + 298) and `close-joint` shrinks.
     - Put `new-await-script-code.js` on `story:module-syntax-and-early-errors`, or on a guard.
     - Fix the scan rule to pick the first story that holds all parts.

5. **`story:module-namespace-objects` vs `story:json-modules` — note.** The two bodies contradict each other on the JSON namespace.
   - `module-namespace-objects.md:44` says `story:json-modules` uses `GetModuleNamespace`. `:79` calls a JSON module's namespace json-modules' own work.
   - `json-modules.md:83` and `:42` give that case to the close, and json-modules has no edge to the namespace story.
   - A unit reading either body will look for `get_module_namespace` that does not exist yet in its order.
   - Settle it by deleting the claim in `module-namespace-objects.md:44` and `:79`. Finding 4 gives the case to `dynamic-import`.

6. **`story:json-modules` — note.** Nothing writes `GetExportedNames` and `ResolveExport` for the synthetic kind.
   - `json-modules.md:59` lists the synthetic kind's link and evaluation only. The linking story's `ResolveExport` is written for source-text records (`module-linking-and-evaluation.md:66`).
   - All 8 passing cases import `default` from a JSON module, and `json-named-bindings.js` (a `resolution` negative) needs `null` for any other name. Both go through `ResolveExport` of the synthetic record.
   - The Change line should add both functions for the synthetic kind.

7. **`story:module-records-loading-and-module-runs` vs `story:dynamic-import` — note.** The payload of a `LoadRequest` is split in half.
   - Records builds `LoadRequest` holding the roots of its referrer and "its payload (the GraphLoadingState Record)" (`module-records-loading-and-module-runs.md:88`). Its `finish_loading_imported_module` has one branch.
   - Dynamic-import's payload is a promise capability, and its finish branch is ContinueDynamicImport (`dynamic-import.md:95`). Its Writes (`:107`) omit `host/hooks.rs`, where `LoadRequest` lives (`crates/swell/src/host/hooks.rs:70`), and `host/agent.rs`.
   - Settle it by having records declare the payload as a sum of the two kinds and the finish dispatch on it. Or add both files to dynamic-import's Writes.

8. **`story:module-linking-and-evaluation` — note.** The fault point of `parse_module` is added in the wrong story.
   - `FaultPoint::ParseModule` and HOST-157 sit at `module-linking-and-evaluation.md:67`, but `parse_module` is the records story's entry point (`module-records-loading-and-module-runs.md:88`).
   - The records story cannot show its own fault path.
   - Either move `ParseModule` to records with HOST-157 split, or say the single requirement lands with linking.

Checked and found right:
- Walked all 355 `depends_on` edges of the whole store through `tsort`, including the slice-10 and slice-12 epic edges: no cycle. `validate` prints `valid`.
- Traced about 20 acceptance reads to producers: pending counts 32, 19, 8, 0 and 13+11+1+2+1+3+1=32; the `HOLDS` and refusal grep counts (6, 4, 0); `modules::` test names; instruction order after `ImportMeta`. Edges record every one except the ones in findings 1 and 2.
- Counted every number the acceptances state. All are right:
  - The lists sum to 1,279 cases and 1,830 runs, plus 2 cases and 3 runs, giving 1,281 and 1,833.
  - All bounds equal list plus guard: 987, 1137, 985, 1178, 420, 834, 538, 6, 87, 782.
  - The guards are 985, 298, 764 and 77, each fully passing on the base.
  - 23 cases pass with 44 runs, so 1789 failing runs.
  - `lists.sh` prints `slice 1281 lists 1281 incoming 0`.
- Transitive fixture closure: no case on the linking or records lists reaches `import * as`, `import.meta`, `import(` or `.json`. The only `await` hits are the export name in `instn-named-id-name.js` and `instn-star-id-name.js`.
- Every story carries the 60 s, 120 s, 10-minute, no-Python and no-`unsafe` bounds.

Q1 to Q10, my view of each:
- Q1: A, a new `modules` arena.
- Q2: B, a new `GetImport` instruction. The compiler already knows which names are imports, so scripts' `GetNamed` and the env slot types stay untouched. A needs unscoped `env/` changes (finding 1).
- Q3: A. Q4: A. Q5: A. Q6: A.
- Q7: A, or the epic's out-of-scope text amended to name who delivers it. `engine.md:3495` lists "scripts and modules from files through the command-line host" among what swell proves without surf, and nothing else in the set carries it.
- Q8: A.
- Q9: A. B means rewriting stories if the paragraph differs.
- Q10: A. B collides on `parser/mod.rs` and `statement.rs`.

What I could not establish:
- Whether the ESS story's measured floor and maximum depend on module scenarios that use namespace, JSON or `import.meta`. The synthesized suite is not in the tree, and the story has an edge only to `story:top-level-await`.
- The base-line figures in the syntax and import-call stories are off by one, out of my lane. `statement.rs` has `Dot` at line 245, `LParen` at line 246 and `_ =>` at line 247; the stories say 244, 245 and 246.
- Out of my lane (acceptance): `slice-11-conformance-close.md:70` item (6) expects `lists.sh` to exit 0 with cases in `close-into-11.tsv`. The script prints `missing` for any slice-11 case on no list and exits 1.
- No build or test run, so none of the code claims is run-tested.

```findings
- file: .engineering/planning/story/dynamic-import.md
  line: 95
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the script-or-module handle of frames, function objects, environment slot names and queued jobs is generalised only here in order 6, but story:module-linking-and-evaluation (module frame and functions), story:top-level-await and story:import-meta (active module) need it in orders 4 and 5; give it one owner at or before linking and name the files (frame.rs:175, ecmascript_function.rs:66, env/mod.rs:94, jobs.rs:92)"
- file: .engineering/planning/story/import-meta.md
  line: 70
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the four order-5 stories prescribe an append order in module.rs (namespace, top-level await, import.meta, JSON) that no depends_on records and that aep plan artifact waves contradicts (import-meta wave 36, json 38, namespace 39, top-level await 43); record the chain with edges or give each part its own file"
- file: .engineering/planning/story/module-records-loading-and-module-runs.md
  line: 74
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the record is declared with every field of orders 4 and 5 but nothing reads them until then, so slot task clippy (-D warnings, Taskfile.yml:1229) fails on dead_code; name an expect(dead_code) per unread field that its owner deletes, or let each story add the fields it reads"
- file: .engineering/planning/story/slice-11-conformance-close.md
  line: 38
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "json-via-namespace.js is owned by the close, which writes no engine file, although story:dynamic-import holds both parts it needs (scan.awk:163 sends it to joint); move it to the dynamic-import list and put new-await-script-code.js on an early story or a guard"
- file: .engineering/planning/story/module-namespace-objects.md
  line: 44
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "it says story:json-modules uses GetModuleNamespace (and at line 79 that a JSON module's namespace is json-modules'), while json-modules.md:83 gives that case to the close and has no edge to this story; delete the claim"
- file: .engineering/planning/story/json-modules.md
  line: 59
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the Change names link and evaluation of the synthetic kind but not GetExportedNames and ResolveExport for it, which all 8 passing cases and json-named-bindings.js need"
- file: .engineering/planning/story/dynamic-import.md
  line: 107
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the payload of LoadRequest (GraphLoadingState in the records story, a promise capability here) and the finish dispatch are split between the two stories, and Writes omits host/hooks.rs and host/agent.rs; have records declare the payload as a sum of both kinds or add the files"
- file: .engineering/planning/story/module-linking-and-evaluation.md
  line: 67
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "FaultPoint::ParseModule is added here although parse_module is the records story's entry point, so the records story cannot show its own fault path; move it with HOST-157 split or state the single requirement lands with linking"
```
