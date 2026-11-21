---
format: aep.planning-md/3
id: review-result:slice-03-parallel-safety-round-2
kind: review-result
status: active
title: parallel-safety critic, slice 3 stories, round 2
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

What I read: 13 stories, the epic and the four round-1 results, with `aep plan artifact show`, `aep plan artifact waves --kind story --status draft`, `git grep` and `rg` over `crates/swell/src`, `crates/swell/tests`, `taskfiles/` and `conformance/`. I checked the base for each operation and mark the stories name.

Round 1 (my six findings, all met):
- **Key-pinning test:** `builtin-group-modules` now pins "the base keys stay, in order, then the groups' keys". That test is green in every wave-2 tree.
- **`Invoke`:** it is now in `builtin-group-modules`, tested there, with a mark naming the two stories that call it.
- **`SetIntegrityLevel` and `prevent_extensions`:** `SetIntegrityLevel` and `TestIntegrityLevel` are now in wave 1, and the `prevent_extensions` mark is removed there. `json-parse-and-raw-json` and `array-mutation-and-sort-methods` no longer wait on the integrity story.
- **`object/ordinary.rs`:** it has one writer in wave 2, `json-parse-and-raw-json`. `object-integrity-and-prototype-methods` says so and may only fix `object/{array,arguments,string}.rs`. `property-index-and-dense-elements` writes all four files in wave 1, and every wave-2 writer of them depends on it.
- **Acceptance regexps:** they select runs by directory before they read the detail, so the unanchored `is`/`toLocaleString` match is gone.
- **Case lists:** the lists are now under `docs/plan/slice-03/`, and all twelve files exist in the working tree. They are untracked at this moment; the brief says they are committed with the plan.

Wave 2 (judged as drafted):
- **Files:** the ten stories each write their own group file, test files and `tests/language` scripts, with no name clash against the base. The exceptions are the files each says it shares: the mark lines in `ops/objects.rs`, `builtins/string/mod.rs`, `ops/testing.rs` and `object/mod.rs`, plus `object/{array,arguments,string}.rs` for fixes.
- **Operations in wave 1:** I found no operation that two wave-2 stories need and wave 1 lacks.
  - Present on the base: `to_integer_or_infinity`, `to_length`, `index_key`, `array_species_create`, `get_v`, `get_method`, `is_constructor`, `same_value_zero`, `create_array_from_list`, `to_property_descriptor`, `from_property_descriptor`, `get_own_property_keys`.
  - Single-story operations: `ToUint16` is `string-access-and-search-methods` alone, and `TrimString`, `StringPad`, `FindViaPredicate`, `CreateListFromArrayLike`, `Encode`, `Decode` and the other single-story operations each have one story.
- **Mark lines:** each shared mark names its consumers. Every story that removes one calls the item (the `expect` marks are `cfg_attr(not(test), …)`), and the removals are identical lines in separate functions, so the merges are clean. `message_kind` and `Agent::count_refusal` are generic, so the counted refusal needs no file outside `string/transform.rs`.
- **Omissions and requirements:** nothing in wave 2 reads `requirements.txt`, `sections.txt` or `later-parts.tsv` as written. The runner does not read `later-parts.tsv`, and the omissions only change the `test262-section` lines of `expected.txt`. `host-requirements-check` accepts a `pending slice 3` line while `conformance/slices-delivered.txt` lacks 3, so the pending interval between waves is harmless.

The three decisions beyond the brief hold: the close writing the three requirement lines and four omissions, the empty group modules, and the doubly-removed marks.

What I could not establish:
- **Out of lane, admitted overlap:** the recorded `scope` of `object-integrity-and-prototype-methods`, `json-parse-and-raw-json`, `object-enumeration-and-copy-methods`, `array-search-and-copy-methods`, `global-this-uri-functions-and-is-error` and `string-access-and-search-methods` omits the files whose mark lines they delete. Those are `ops/objects.rs`, `ops/testing.rs`, `object/mod.rs` and `builtins/string/mod.rs`. `aep plan artifact waves` therefore shows no collision inside wave 2, while the bodies name four stories on `ops/objects.rs` and two on `string/mod.rs`. I judged it admitted and safe; a person should know the tool does not show it.
- **Out of lane (design):** `array-search-and-copy-methods` installs `Array.prototype[@@unscopables]`, a Symbol-keyed object property that the group's list of name, length and id cannot hold. The story says no wave story writes `builtins/array/mod.rs`, but the install needs either that file or a hook in the group. No other wave-2 story writes the file, so nothing collides.
- **Out of lane (acceptance):** the close's check reads `.cache/inventory/slices.awk` and `meta.tsv`, which no story creates.
- **Not checkable now:** whether ten stories adding private methods to `impl Agent` in different files avoid duplicate method names. The wave branch's build shows it.
- **Not checkable now:** whether adjacent-line edits to `object/mod.rs` by `property-index-and-dense-elements` and `builtin-group-modules` merge cleanly. The code that decides it is not written.

Counts: 13 items with a cited surface, 0 inferred, 0 unplaceable. I traced 26 acceptance reads to a producer in the set (10 wave 2 to `builtin-group-modules`, 6 to `property-index-and-dense-elements`, 10 from the close to wave 2). `depends_on` records all 26. The other four wave-2 stories read nothing `property-index-and-dense-elements` produces.

```findings
[]
```
