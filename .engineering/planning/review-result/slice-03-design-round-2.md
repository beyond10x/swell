---
format: aep.planning-md/3
id: review-result:slice-03-design-round-2
kind: review-result
status: active
title: design critic, slice 3 stories, round 2
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
approve

**What I read:** 13 stories, the epic and the four round-1 results, with `aep plan artifact show` for each. I also ran `aep plan artifact relations`, `graph`, `validate` (valid) and `waves --kind story --status draft`. I read the code under the stories' `Writes:` lines (`object/mod.rs`, `builtin_function.rs`, `realm/mod.rs`, `sections.rs`, `host-requirements.yml`). I walked 54 edges across the whole graph, outside the set too. They hold 26 `depends_on`, 13 `decomposes` and no cycle. The close story's ten edges are the only chain, and each records a real need. I traced about 40 acceptance reads to a producer in the set, and an edge records every one. The reads are the case lists, `cross-story.tsv`, `move-estimate.tsv` and the probes. They sit in the repository, or in the story's own files, or at the base commit.

**Round 1, my finding:** met. `SetIntegrityLevel`, `TestIntegrityLevel`, `Invoke`, `EnumerableOwnProperties`, `CreateDataPropertyOrThrow`, `CodePointAt`, `UTF16EncodeCodePoint` and the this-string steps are now in `story:builtin-group-modules`, with their own tests. `story:json-parse-and-raw-json` and `story:object-integrity-and-prototype-methods` call them and write none.

**Round 1, the others' findings, as far as they touch shape:** met.
- The epic now carries the object model and the path agreement of the task file.
- The case lists are in `docs/plan/slice-03/`, and the probes are files of the story.
- The group-module key test now pins the base keys followed by the group lists.
- `ordinary.rs` has a single writer.
- No wave 2 test calls another wave 2 story's builtin. I checked each story's list of what it uses instead.

**The three beyond-brief choices:**
- **`requirements.txt` and `sections.txt` moved to the close:** no defect. The runner reads `omit` lines as metadata only (`crates/test262/src/sections.rs`). `host-requirements-check` allows `pending` for a slice not yet delivered (`taskfiles/host-requirements.yml:254`). Between wave 2 and wave 3 nothing in the gate fails. The close depends on all ten stories, and each story checks its own line on a copy.
- **Empty group modules:** no seam. A group file holds its id type, its list and its run function, and `mod.rs` builds ids from the list.
- **Marks deleted by two stories:** each of `SetIntegrityLevel`, `Invoke` and the two code-point functions is deleted by two stories. Each deletion sits on a hunk of its own, so a merge takes the same change from both sides. Every caller outside tests is in the story that removes the mark.

**Completeness of wave 1:** I read what each of the ten wave 2 stories calls against the base. The named properties of `Object`, `Array` and `String` that are missing add up to the stories' lists, and no operation two stories need is missing. `ToUint16`, `FindViaPredicate`, `FlattenIntoArray`, `SortIndexedProperties`, `Encode`, `Decode` and the rest each have one caller, in that caller's file.

**What I could not establish:**
- Whether a mark on an item whose only caller is itself under `expect(dead_code)` stays quiet under clippy. I think the compiler treats a callee of an `expect`-marked item as live. It is a compile-time fact the unit's own check shows, not a shape defect.
- Whether `integrity_levels_charge_a_step_per_key` stays green once the property-index story changes what a key list charges. A lower bound would; an exact count would not.
- **Out of my lane (parallel safety):**
  - `crates/swell/src/object/mod.rs` is written by both wave 1 stories, and `waves` records no collision between them. `story:builtin-group-modules` does not list it in its scope, although its body says it removes four lines there.
  - The wave 2 stories' mark deletions in `ops/objects.rs`, `ops/testing.rs`, `object/mod.rs` and `builtins/string/mod.rs` are in no story's scope.
- **Out of my lane (scope):** the epic's "Settled" list still says the `omit` lines are "added by the stories that deliver the methods". The close story now writes them.

```findings
[]
```
