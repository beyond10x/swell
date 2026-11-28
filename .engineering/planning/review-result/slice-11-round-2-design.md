---
format: aep.planning-md/3
id: review-result:slice-11-round-2-design
kind: review-result
status: active
title: 'Slice 11, round 2, the design critic: needs-revision, 7 findings (1 blocker)'
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
Plan critic, design, round 2 on the fifteen draft stories of
`epic:slice-11-modules` (`plan/slice-11` at `e1eaf39a`), 2026-11-28.
Verdict: needs-revision, 7 findings, 1 of them a blocker (the built-in closures of a module with top-level await); revision 2 answers them. The critic's answer, as returned:

needs-revision

Round-1 findings of the design role:

1. Script-or-module handle: settled. `module-records-loading-and-module-runs.md:124` owns it at every site I grepped in `crates/swell/src` (`interp/frame.rs`, `object/ecmascript_function.rs`, `env/mod.rs` `SlotNames`, `host/jobs.rs`, the readers in `interp/`, `dynamic_code.rs`, `agent.rs`). `dynamic-import.md:96` no longer writes `frame.rs` or `jobs.rs`. The Q2 import slot kind has an owner in the linking story (`module-linking-and-evaluation.md:79`), but its file list is incomplete (new finding 5).
2. Append order in `module.rs`: settled. `crates/swell/src/module/` is a directory, one file per part (`module-records-loading-and-module-runs.md:123`, `:107`). The "after the functions of X" sentences are gone (grep finds none).
3. Unread record fields and clippy: settled. Option (a) at `module-records-loading-and-module-runs.md:107`: every later part's state is an empty type read by the record's trace. The only `expect(dead_code)` is on `LoadPayload::Dynamic`. The workspace lints (`crates/swell/Cargo.toml:26`) have no `unused_self`, so empty traces do not warn.
4. Joint cases: settled. `json-via-namespace.js` is on the dynamic-import list, `new-await-script-code.js` on the syntax list, and `close-joint.cases.txt` is gone. `lists.sh` over the base run prints `slice 1281 lists 1281 incoming 0`. The list sums are 3+152+193+122+70+240+6+10+485 = 1281. The bounds 988, 1137, 1178, 420, 843 (834 on `f962c324`), 538, 6, 87 and 794 (783 on `f962c324`) each equal B + F.
5. Namespace vs JSON claims: settled. `module-namespace-objects.md` no longer says JSON modules use `GetModuleNamespace`; `:42` gives it to dynamic-import.
6. JSON `GetExportedNames` and `ResolveExport`: settled. `json-modules.md` fills `exported_names`, `resolve_export`, `link` and `evaluate` in `json.rs`. The linking story adds the dispatch stubs (`module-linking-and-evaluation.md:77`).
7. `LoadRequest` payload: partly settled. Records declares `LoadPayload` as a sum (`module-records-loading-and-module-runs.md:109`). The finish step for a script or realm referrer is still unowned (new finding 3).
8. `FaultPoint::ParseModule`: settled. It is in records (`module-records-loading-and-module-runs.md:125`). `FaultPoint::Link` and the HOST-157 line are in linking (`module-linking-and-evaluation.md:80`).

New findings:

1. `story:top-level-await` — blocking. Nothing in the story says how a module body's completion reaches AsyncModuleExecutionFulfilled and Rejected, and the story excludes the file that would carry it.
   - Place: `top-level-await.md:59` and `:73`.
   - What is wrong: ExecuteAsyncModule is PerformPromiseThen with two built-in closures that hold the module. A promise reaction handler is only `Empty` or a `JobCallback` of a function object (`crates/swell/src/object/promise.rs:102`). A closure with captured state is a `BuiltinId` variant plus a `Slots` variant in `object/builtin_function.rs`: `BuiltinId` at line 57, `Slots` at line 251, exhaustive matches at lines 844, 902 and 954. The Writes line says "Not written: … `crates/swell/src/object/`".
   - What would settle it: either records declares one module built-in kind and its `Slots` variant once, in the form of the state files, with TLA and dynamic-import filling their functions; or `object/builtin_function.rs` goes into TLA's Writes with its hunks. The existing edge from dynamic-import to TLA orders the two stories on that file.

2. `story:top-level-await` — warning. The compile side names three compiler files, and the module's `Code` has no field to say it is asynchronous.
   - Place: `top-level-await.md:58` and `:73`.
   - Evidence:
     - The compile story's `ModuleCode` lists slots, requests, entries and `function_declarations`, with no HasTLA or async-slot field (`module-code-compiles.md`, Change).
     - The activation local slot of an async body is allocated in `crates/compiler/src/resolve.rs:667-672`.
     - `async_start` and `async_body` are private in `crates/compiler/src/function.rs:233-262`.
     - TLA's Writes are `lib`, `expression` and `statement` only.
   - What would settle it: name `bytecode.rs` (the `ModuleCode` field), `resolve.rs` (slot allocation for the module's top level) and `function.rs` (or a copy of the two helpers in `lib.rs`) with their hunks.

3. `story:dynamic-import` — warning. The story keeps loaded modules on script and realm records, but writes no code that stores into them or reads them.
   - Place: `dynamic-import.md:81` and `:96`.
   - What is wrong: the story gives `module/loading.rs` "one line" (the `expect`). The records story's finish only handles a module referrer (HOST-63, `module-records-loading-and-module-runs.md:125`) and puts script and realm referrers out of scope (`:151`). The test `two_imports_of_one_specifier_share_one_module` needs the arm.
   - What would settle it: add the Script and Realm arms of the finish to dynamic-import's hunks in `loading.rs`, or have records dispatch on the referrer kind with those arms answering an engine fault until this story fills them.

4. `story:dynamic-import` — warning. This is the same gap as finding 1.
   - Place: `dynamic-import.md:81`.
   - What is wrong: ContinueDynamicImport's onFulfilled and onRejected are built-in closures over the module and the promise capability. The story names no `builtin_function.rs` hunk.
   - What would settle it: the same single owner as finding 1.

5. `story:module-linking-and-evaluation` — warning. The Writes line omits files whose exhaustive matches the new `SlotKind::Import` and `SlotValue` variant break.
   - Place: `module-linking-and-evaluation.md:79` and `:93`.
   - Evidence, all matches without a wildcard:
     - `SlotValue` in `crates/swell/src/env/global.rs:262`, `interp/class.rs:516-519`, `:541-545` and `:564-566`, and `interp/private.rs:76-77`.
     - `SlotKind` in `crates/swell/src/dynamic_code.rs:853-863`.
   - What is wrong: the linking story has no "checked against the code" line for these two types, though the records story has one for `Item` and `Held`.
   - What would settle it: add the four files, with their lines, to the linking story's Writes.

6. `story:module-code-compiles` — warning. The Context names the wrong instruction.
   - Place: `module-code-compiles.md:69`.
   - What is wrong: it says the named-slot reads are "`GetNamed`, `bytecode.rs` line 183". `GetNamed(u32)` is a property read, `[o] -> [v]` (`crates/compiler/src/bytecode.rs:183`). The slot reads are `GetEnv`, `GetEnvChecked` and `SetEnv` (`bytecode.rs:116-125`; `dispatch.rs:426` and `:596`), which the linking story names correctly.
   - What would settle it: replace the name in the sentence.

7. `story:module-records-loading-and-module-runs` — warning. `[[TopLevelCapability]]` is placed with TLA, but linking's `evaluate_module` already uses a top-level capability.
   - Place: `module-records-loading-and-module-runs.md:107` against `module-linking-and-evaluation.md:65` and `:78`.
   - What is wrong: records puts `[[TopLevelCapability]]` in `AsyncState`, filled by TLA. Linking says the async values are "read from the `AsyncState` the records story made empty" and makes a capability per `evaluate_module`. A repeated `evaluate_module` of an Evaluated graph returns the stored promise in the spec (`#sec-moduleevaluation`). The text keeps no stored capability before TLA, so the second call answers a fresh promise.
   - What would settle it: move the field to `LinkState`, or state in linking that a later `evaluate_module` answers a new fulfilled promise until TLA.

On the decomposer's list, judged in the code:
- **Tool splitting orders 5 and 6:** the hunks do not overlap. `parser/expression.rs` is at lines 742/771 against 1222-1230. `compiler/src/expression.rs` is at line 342 against 1233. The `requirements.txt` lines 64, 68, 99, 158 and 224 are not adjacent. This is the trade-off of edges against hunk rules, not a defect.
- **`HOLDS` lines 1 and 2:** the merge rule is mechanical. Line 4 is separated from line 2 by an unchanged line 3, so it merges cleanly.
- **Scripts-or-module end-to-end test in linking:** accepted. No module code runs before linking, and `a_closure_made_in_module_code_runs_after_the_evaluation_ended` is there.
- **Size of the records story:** one thing. The arena, the record and the handle cannot be shown apart.
- **`json.rs` stubs and the runner expectation:** acceptable. The scaffolds are removed by `git grep` checks that must print nothing (`module-linking-and-evaluation.md:99`, `json-modules.md`).
- **`continue_dynamic_import` answering an engine fault:** acceptable, except the referrer arms in finding 3.

Read: 15 stories whole (`show`-equivalent, with the two new ones as fresh drafts), `validate` (`valid`), `relations` and `graph`. I walked 367 `depends_on` edges across the whole store, outside the set, through `tsort`: no cycle. I traced about 24 acceptance reads to producers (HOLDS greps, pending counts, `run::`, `modules::` and `faults::` counts, source-file counts, fixture trees, HOST-224); an edge or a path of edges records every one. I ran `lists.sh` over the base run and recomputed the sums and bounds. I read the base code at the sites named above.

What I could not establish: whether the `cli-module-loader` hook finds the referrer's file through `module_host_defined` or through its map. `LoadRequest::host_defined()` is the graph's `hostDefined`, not the referrer's, so the story should say which. This is an unease, not a finding. `HOLDS` needs `#[rustfmt::skip]` as slice 8 had it (`class-syntax-and-early-errors.md:136`), but the repository has no fmt gate, so I did not count it. Out of my lane: `cli-module-loader` states no exit status for an unreadable root file.

```findings
- file: .engineering/planning/story/top-level-await.md
  line: 73
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the story names no way for a module body's completion to reach AsyncModuleExecutionFulfilled and Rejected, which needs a built-in closure kind (a BuiltinId and a Slots variant in object/builtin_function.rs, exhaustive matches at lines 844, 902, 954; promise.rs:102 Handler is only Empty or JobCallback), and its Writes line says crates/swell/src/object/ is not written; give the kind one owner in the records story or name the file and hunks here"
- file: .engineering/planning/story/top-level-await.md
  line: 58
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the module's Code is said to carry HasTLA but ModuleCode of story:module-code-compiles has no such field, and the activation slot of an async body is allocated in crates/compiler/src/resolve.rs:667 and emitted by the private async_start of function.rs:233, none of which are in the Writes line (lib, expression, statement); name bytecode.rs, resolve.rs and function.rs with their hunks"
- file: .engineering/planning/story/dynamic-import.md
  line: 96
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "ContinueDynamicImport's onFulfilled and onRejected are built-in closures over the module and the capability and the Writes line names no object/builtin_function.rs hunk; the same single owner as for story:top-level-await settles it"
- file: .engineering/planning/story/dynamic-import.md
  line: 81
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "script and realm referrers keep their loaded modules but nothing writes the finish step that stores into and reads them: loading.rs is given one line (the expect) and the records story's finish handles a module referrer only; add the Script and Realm arms to this story's loading.rs hunks or have records dispatch on the referrer kind"
- file: .engineering/planning/story/module-linking-and-evaluation.md
  line: 93
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the new SlotKind::Import and SlotValue variant break exhaustive matches in env/global.rs:262, interp/class.rs:516-566, interp/private.rs:76 (SlotValue) and dynamic_code.rs:853-863 (SlotKind), none in the Writes line; add them with their lines"
- file: .engineering/planning/story/module-code-compiles.md
  line: 69
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the Context names GetNamed (bytecode.rs:183, a property read) as the named-slot read; the slot reads are GetEnv, GetEnvChecked and SetEnv (bytecode.rs:116, dispatch.rs:426 and :596)"
- file: .engineering/planning/story/module-records-loading-and-module-runs.md
  line: 107
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "[[TopLevelCapability]] sits in AsyncState, filled by story:top-level-await, while story:module-linking-and-evaluation (:65, :78) makes and settles a top-level capability per evaluate_module and says it reads async values from the empty AsyncState; a repeated evaluate_module of an Evaluated graph returns the stored promise in the spec, so move the field to LinkState or say linking answers a fresh fulfilled promise until TLA"
```
