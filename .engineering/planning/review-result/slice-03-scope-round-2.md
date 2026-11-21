---
format: aep.planning-md/3
id: review-result:slice-03-scope-round-2
kind: review-result
status: active
title: scope critic, slice 3 stories, round 2
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

- epic:slice-03-builtins-core — the epic still says the `omit` lines of the string-length `RangeError` are "added by the stories that deliver the methods", but `story:slice-03-conformance-close` now writes all four (`sections.txt`) and the eight delivering stories name it as the one writer. The epic's sentence has to say the close writes them, or the stories have to go back to writing their own. — .engineering/planning/epic/slice-03-builtins-core.md:71

What I read:
- 14 artifacts in full: the epic first, then the 13 stories, each with `aep plan artifact show`.
- The four round-1 results, plus `aep plan artifact waves --kind story --status draft` and `aep plan artifact validate`, which reports valid.
- Code spot-checks with `git grep` and `sed` for the `THROUGH_BUILTINS` practice and the recorded scope of `object/mod.rs`.

Promises:
- I extracted 19 promises from the epic: 10 outcomes, 9 acceptance items, plus the settled points and out-of-scope list as bounds.
- All 19 trace to an item. The counts, the coverage (below) and the unclaimed cases (below) were checked.

Round-1 scope findings:
- Both are met. The epic's rewrite carries the object model, with the `eval` binding scan, in an outcome bullet. It also carries the "paths read the same way" change in its last outcome bullet.
- The three round-1 findings of the other critics that touch coverage are met as well. `SetIntegrityLevel` and `TestIntegrityLevel` moved into `story:builtin-group-modules`. `Invoke` moved there too. The probe lists are in the property story.

Coverage and double claims:
- **Object:** 13 + 7 builtins, plus `fromEntries` and `groupBy` named as slice 6. That makes 19 of the missing constructor properties and 3 of the missing prototype ones.
- **Array:** the three stories hold 12, 11 (including `@@unscopables`) and 9 members. `from`, `keys`, `values`, `entries` and `fromAsync` are named as later slices.
- **String:** 3 + 13 + 11 builtins, with 7 for slice 5.
- **Host requirements:** `HOST-21`, `HOST-120` and `HOST-155` each have one test-writing story and one line-moving story (the close).
- **Double claims:** none. The deleted mark lines shared by two stories are the same lines, so they are a parallel-safety matter and not a double claim.

The wave-1 operations are complete and tested in `story:builtin-group-modules`:
- `SetIntegrityLevel`, `TestIntegrityLevel`, `Invoke`, `EnumerableOwnProperties` with its three kinds, and `CreateDataPropertyOrThrow` each have a named test.
- `CodePointAt` and `UTF16EncodeCodePoint` have two tests between them.
- No operation that two wave-2 stories need is missing. Each other shared operation exists on the base or has one caller.

Reach beyond the parent:
- The three things the reviser added beyond the brief are traceable to the epic, as drafted. The close writing the requirement lines and the four `omit` lines follows the epic's acceptance 9 and its settled `RangeError` point. The empty group modules serve the "each builtin in a module" outcome. The shared mark deletion is a mechanical choice.
- The two well-known symbols and `@@unscopables` are internal to the realm and cite slice 2's precedent. They do not reach the `Symbol` exclusion.

What I could not establish:
- Out of my lane, for parallel-safety: `story:builtin-group-modules` has no `crates/swell/src/object/mod.rs` among its recorded scope files, though its body and `Writes:` name it. `story:property-index-and-dense-elements` records it. `aep plan artifact waves` therefore lists no wave-1 collision between the two, while both stories say they share the file.
- Out of my lane, for design: between wave 2 and wave 3 the gate counts three requirements as pending, and the wave branch carries cases that pass or fail by an `omit` line not yet recorded. The set accepts this knowingly. I judged it by consequence only for scope, where nothing is dropped, because the close claims both.
- `docs/plan/slice-03/` is still untracked on this branch. The brief says it is committed with the plan, so I did not count it.
- Cases of `cross-story.tsv` whose failed run ends on a slice-3 part that no story owns have no named owner. The close sends such a case "back to the story that owns the part", which cannot be checked before the wave exists. This is an unease and not a finding.
- The ruling on the seven directories is not needed for my verdict. The close's acceptance holds either way, and the stories no longer assume it.

```findings
- file: .engineering/planning/epic/slice-03-builtins-core.md
  line: 71
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the epic still says the omit lines of the string-length RangeError are added by the stories that deliver the methods, but story:slice-03-conformance-close now writes all four and the eight delivering stories name it as the one writer; the epic's sentence has to say the close writes them, or the stories have to go back to writing their own"
```
