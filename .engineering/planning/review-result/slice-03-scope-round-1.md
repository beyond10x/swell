---
format: aep.planning-md/3
id: review-result:slice-03-scope-round-1
kind: review-result
status: active
title: scope critic, slice 3 stories, round 1
relations:
- reviews: story:builtin-group-modules
- reviews: story:property-index-and-dense-elements
- reviews: story:object-integrity-and-prototype-methods
- reviews: story:object-enumeration-and-copy-methods
- reviews: story:function-apply-and-source-text
- reviews: story:global-this-uri-functions-and-is-error
- reviews: story:json-parse-and-raw-json
- reviews: story:array-iteration-methods
- reviews: story:array-search-and-copy-methods
- reviews: story:array-mutation-and-sort-methods
- reviews: story:string-access-and-search-methods
- reviews: story:string-transform-and-compare-methods
- reviews: story:slice-03-conformance-close
revision: 1
---
needs-revision

- epic:slice-03-builtins-core — no outcome, constraint or acceptance of the epic names the property index map, the dense Array elements or the `eval`-binding scan, yet `story:property-index-and-dense-elements` claims all three. The only source for it is `docs/plan/waves.md:177` ("The slice that builds the map (slice 3 at the latest) covers both, or charges the scan") and the design's decision 4, which the epic cites only for other things. Either the epic carries the outcome in its rewrite or the story is dropped from the set. — `.engineering/planning/epic/slice-03-builtins-core.md:21`, `docs/plan/waves.md:177`
- story:slice-03-conformance-close — the story changes how `taskfiles/test262.yml` reads its paths when a variable is exported and also passed on the command line, and adds the plant `tests/gate/test262-paths-agree-when-exported-and-passed.sh`. Nothing in the epic asks for this: it comes from the "next change of `taskfiles/test262.yml`" entry in `docs/plan/waves.md`, not from any promise of the epic. Only the `to > from` check traces to epic acceptance 4. Drop the path-agreement part or have the epic claim it. — `.engineering/planning/story/slice-03-conformance-close.md:39`, `docs/plan/waves.md:173`

What I read: 14 artifacts, each read in full. The epic I read first and listed 16 promises (8 outcomes, 8 acceptance items) plus 5 open points. I ran `aep plan artifact show` on the epic and on all thirteen stories. I also read the decomposer's `s3-need.tsv`, `s3-create-realm.txt` and `move-estimate.tsv`, `docs/design/engine.md` ("A part before its slice") and `docs/plan/waves.md` ("What later slices take over"), and `conformance/host/requirements.txt` and the `[slice 3]` HOST lines. I traced 13 of the 16 promises to an item. The other three are the `Function` nesting-bound `RangeError` (outcome 7 and acceptance 8) and the half of acceptance 3 that says `AGENTS.md` and `README.md` carry the gate's numbers. The first two were delivered in slice 2 as a `SyntaxError` and the coordinator rewrites them. The third is named as the coordinator's in the close story's "Out of scope". The 5 open points are settled in the design.

Coverage, both directions:
- **Properties:** the named properties of `Object`, `Array` and `String` that were missing add up exactly to the stories' lists: 19 of `Object`, 34 of `Array.prototype` and 31 of `String.prototype`.
- **Deferred properties:** `Object.fromEntries`, `Object.groupBy`, `Array.from` and the Array iterator methods are deferred in the open and named; `normalize` and `Array.fromAsync` already have later slices by `dir` lines.
- **Failing cases:** the ten wave stories' lists sum to 4,050 of the 4,873 failing cases. The rest are 203 cross-story cases and the 620 later-slice moves, and the close story claims both.
- **HOST requirements:** HOST-21, HOST-120 and HOST-155 are each claimed once.
- **Double claims:** I found none. `toLocaleString` and `at` appear in two stories but name different receivers.

What I could not establish:
- Stated as settled before the ruling in your point 3: `story:array-search-and-copy-methods` and `story:object-enumeration-and-copy-methods` say their iterator directories "go to slice 6 at the close", passing cases included, as if the exception were already ruled. Only the close story conditions it ("Without that decision the 9 stay"). This is a wording unease, not a finding, because neither story's own acceptance depends on it.
- Out of my lane, for the other critics:
  - **Shared files:** `conformance/test262/sections.txt` gets three `omit` lines from three stories (array-search, string-transform and json); `requirements.txt` gets three one-line moves; the `mod.rs` mark lines and `object/ordinary.rs` are shared. These are parallel-safety questions.
  - **Dependencies:** `story:string-transform-and-compare-methods` and `story:function-apply-and-source-text` depend on design changes made before the wave. They say so in their bodies; whether the dependency is also an edge between artifacts is for design.
- The twelve cross-realm cases (`s3-create-realm.txt`) wait on "a way for a host function to create a realm" (`docs/plan/waves.md:153`). The global story names it as out of scope and the drafter reported it. I did not check whether the runner's own realms are enough for the cases that still fail.

```findings
- file: "aep plan artifact show epic:slice-03-builtins-core"
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "no outcome, constraint or acceptance of the epic names the property index map, the dense Array elements or the eval-binding scan that story:property-index-and-dense-elements claims; the only source is docs/plan/waves.md:177, so the epic should carry the outcome or the story leaves the set"
- file: ".engineering/planning/story/slice-03-conformance-close.md"
  line: 39
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the story changes how taskfiles/test262.yml reads its paths when a variable is exported and also passed on the command line, with a new plant script, and nothing in the epic asks for it (it comes from docs/plan/waves.md:173); only the to-greater-than-from check traces to epic acceptance 4"
```
