---
format: aep.planning-md/3
id: review-result:slice-11-round-2-scope
kind: review-result
status: active
title: 'Slice 11, round 2, the scope critic: needs-revision, 4 findings (2 blockers)'
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
Plan critic, scope, round 2 on the fifteen draft stories of
`epic:slice-11-modules` (`plan/slice-11` at `e1eaf39a`), 2026-11-28.
Verdict: needs-revision, 4 findings, 2 of them blockers (both in the story of the command-line host's loader); revision 2 answers them. The critic's answer, as returned:

needs-revision

**Round-1 findings (scope review)**

1. Bounds of namespace and dynamic-import: settled.
   - Places: `module-namespace-objects.md:76` and `dynamic-import.md:104`.
   - Both now give the formula (`B` counts pulled-in cases whose owner has landed) and the join's numbers: `bound 843 B 773` and `bound 794 B 309`.
   - I rebuilt the dynamic-import part with `part.sh` into scratch. It holds 794 files: 485 on the list, 298 guard, 11 extras. That gives 309 + 485 = 794, so the report's 794 is right. My round-1 figure of 793 predates the move of `json-via-namespace.js` to this list.
   - The namespace part is 844 = 70 + 764 + 10; 773 + 70 = 843.
2. Command-line host loader: open.
   - `story:cli-module-loader` exists and the three "not in this slice" mentions now point to it.
   - The epic sentence that excludes it is still there and no rewrite is carried (new finding 1).
3. Wrong story id: settled. `grep -rn determinism-faults .engineering docs | wc -l` prints `0`.
4. The 39 features question: settled at `slice-11-conformance-close.md:41`, in the form of slice 9 (`globalThis` 11 cases, `hashbang` 1 case).
5. Lines 245 to 247: settled. `statement.rs` has the refusals at 245, 246 and 247; the import-call story says "245 and 246" (line 73) and the syntax story uses the same range.
6. The four events: settled at `ess-target-module-commands.md:65`. One passed scenario per command and per event is named in the report.

**New findings**

1. `story:cli-module-loader` and `epic:slice-11-modules` — blocking.
   - Place: `.engineering/planning/epic/slice-11-modules.md:80` ("Out of scope"); close story `slice-11-conformance-close.md:64`.
   - The epic's out-of-scope bullet reads "A real module resolver (URLs, the network, files): the host's; the engine only calls the hook (decision 9). The command-line host and the runner provide their own loaders." The new story builds a file loader in the command-line host.
   - The close story's Context (line 39) says the sentence "is claimed by `story:cli-module-loader`". The close's coordinator list (line 64) rewrites the epic's counts and does not rewrite this bullet. After the close the epic would still exclude what a story of the slice built.
   - Slice 9 round 1 had exactly this defect as its only finding (an epic out-of-scope line contradicting its stories).
   - Q7 A, taken by the coordinator, is a decision and I do not dispute it.
   - What would settle it: add to the close's coordinator list the rewrite of that bullet. For example: "the command-line host reads module files beside the referring file (`story:cli-module-loader`); `import()` and JSON modules in it are not in this slice". The close's Context sentence then cites the rewrite.
2. `story:cli-module-loader` — blocking. Its Context says `import()` "from a script or a module of the shell" is out of scope, and its Change keeps only some requests pending.
   - Place: `cli-module-loader.md:30` and `:49`.
   - Context says `import()` from a script or a module is out of scope and stays pending.
   - The Change keeps pending only "a request whose referrer is a script or a realm, or with an attribute". A dynamic request whose referrer is a Module satisfies none of these. The hook would resolve, read and finish it.
   - That finish carries the `Dynamic` payload. The story depends on neither `story:dynamic-import` nor `story:module-determinism-and-emulated-loading`. The decomposer's report says `continue_dynamic_import` answers an engine fault until `dynamic-import` lands.
   - So a module of the shell that calls `import()` either ends in exit 70 or loads, depending on merge order. The story neither claims nor excludes that behaviour.
   - What would settle it: add `request.is_dynamic()` to the keep condition (keep the request, as today). Add a test line or one fixture sentence saying `import()` in a module file leaves its promise pending. Or, the other way, depend on `story:dynamic-import` and claim `import()` from a module. The first is the smaller scope the coordinator asked for.
3. `story:cli-module-loader` — note. The story does not say how the hook learns the referring module's file.
   - Place: `cli-module-loader.md:49`.
   - The story builds a map from canonical path to `Module`. The hook resolves a specifier "against the directory of the referring module's file", and the map points the wrong way for that lookup.
   - The interface carries it: `parse_module(realm, source, host_defined)` and `module_host_defined(module)` (`host-interface.md:928`, `:937`). The story does not name it.
   - What would settle it: one clause saying the file's path is the `host_defined` given to `parse_module`, read back with `module_host_defined` of the referrer.
4. `story:slice-11-conformance-close` and `story:cli-module-loader` — note. The deferred part is claimed by an entry that names no later slice.
   - Place: `slice-11-conformance-close.md:64`, `cli-module-loader.md:30`.
   - The `waves.md` entries under "What later slices take over" begin with the slice that takes the item ("Slice 4:", "Slice 5:", "Slices 2, 9 and 11:"). The proposed entry begins "Slice 11:", the slice just closed.
   - The story's summary says "named for a later slice", and no slice is named.
   - What would settle it: name the slice, or write "a later slice, decided at its decomposition".

**Checked and found right**

- `modules-design-text`:
  - The anchors match: `**Private names.**` at 1999, `**Template objects.**` at 2419, the section 11 "Modules" bullet at 3561, HOST-68 at 975, HOST-223 at `requirements.txt:223`.
  - The base counts match: the four greps print `0`, and `pending slice 11$` prints `32`.
  - The 33 sum holds: records 13, linking 11, json 1, tla 1, import-meta 3 (with HOST-224), dynamic-import 3, determinism 1.
  - `depends_on` is present on compiles, records, linking, top-level-await and import-meta.
  - No other story writes `docs/design/` before the close.
  - The epic does not name the paragraphs. Their basis is coordinator ruling Q9, the epic's decisions 3, 9, 10 and 11, and the `waves.md` entry "Slices 2, 9 and 11" for HOST-224. I raise no reach finding.
- `cli-module-loader`:
  - Section 9 line 3495 does promise "scripts and modules from files through the command-line host".
  - The shell anchors match: `ShellHost` at 38 to 44, `host_load_imported_module` at 59, `evaluate` at 128.
  - The test count is 16 on the base and 22 after (16 + 6; the help test is edited, not new). The fixtures are 7 files.
  - `lists.sh` over the base run prints `slice 1281 lists 1281 incoming 0`.
  - `aep plan artifact validate` prints `valid`.

Out of my lane: the acceptance commands of `modules-design-text` (`host-requirements-check` builds every test of the workspace) and of the close carry no `slot --timeout`. This is for the acceptance and parallel-safety critics.

```findings
- file: .engineering/planning/epic/slice-11-modules.md
  line: 80
  category: scope
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the epic excludes 'A real module resolver (URLs, the network, files)' and says 'The command-line host and the runner provide their own loaders'; story:cli-module-loader builds a file loader in the command-line host, and the close's coordinator list (slice-11-conformance-close.md:64) carries no rewrite of that bullet; add the rewrite so the epic and its stories agree"
- file: .engineering/planning/story/cli-module-loader.md
  line: 49
  category: scope
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "Context puts import() from a script or a module out of scope, but the hook keeps pending only a request whose referrer is a script or a realm or that has an attribute; a dynamic request with a Module referrer is loaded and finished with the Dynamic payload before story:dynamic-import lands (an engine fault); add is_dynamic to the keep condition with a fixture sentence, or depend on dynamic-import and claim it"
- file: .engineering/planning/story/cli-module-loader.md
  line: 49
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the hook resolves against the directory of the referring module's file but the story keeps only a path-to-Module map; say the path is the host_defined given to parse_module, read with module_host_defined of the referrer"
- file: .engineering/planning/story/slice-11-conformance-close.md
  line: 64
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the waves.md entry for import() and JSON modules in the command-line host begins 'Slice 11:' (the slice just closed) and names no later slice; entries of 'What later slices take over' begin with the taking slice; name it or say it is decided at that slice's decomposition"
```
