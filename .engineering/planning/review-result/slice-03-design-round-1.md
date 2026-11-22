---
format: aep.planning-md/3
id: review-result:slice-03-design-round-1
kind: review-result
status: active
title: design critic, slice 3 stories, round 1
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

story:json-parse-and-raw-json — `JSON.rawJSON` must return a frozen object, but the code has no SetIntegrityLevel, and the only story that adds one is `story:object-integrity-and-prototype-methods` (`builtins/object/integrity.rs`). Ten stories are meant to run side by side in one wave, and no `depends_on` edge records this. The mark on `prevent_extensions` (`object/mod.rs:282`) is a second sign of the same gap. Integrity's Change removes that mark, and JSON's own freeze would need that function too. The fix is a `depends_on: story:object-integrity-and-prototype-methods` edge, which ties the JSON unit to integrity and makes the wave less parallel. The alternative is to move SetIntegrityLevel and TestIntegrityLevel into `story:builtin-group-modules` (which already writes `ops/objects.rs`) so that both stories use them. — `.engineering/planning/story/json-parse-and-raw-json.md:30`, `.engineering/planning/story/object-integrity-and-prototype-methods.md:26`

**What you read**
- I read 13 stories and the epic with `aep plan artifact show`, and ran `aep plan artifact relations`, `graph` and `validate`; `validate` reports valid.
- I walked 39 edges, 26 of them `depends_on`, and outside the set as well (the epic's edge to slice 2, slice 4's edge to this epic). I found no cycle. The only chain is the close story's ten edges, and each of those records a real need.
- I traced about 22 acceptance reads to a producer in the set and 21 have an edge. The one without is the JSON-to-integrity read above.
- I also read `AGENTS.md`, `docs/design/engine.md` section 12 ("A part before its slice"), `taskfiles/test262.yml`, `conformance/test262/expected.txt`, `crates/swell/src/object/builtin_function.rs`, the `expect(dead_code)` marks in `crates/swell/src`, and the base `runs.tsv`.
- I checked that the 13 stories cover every missing `Object`, `Array` and `String` builtin, or name where it goes. `Array.from`, `keys`, `values`, `entries`, `Object.fromEntries`, `Object.groupBy` and `normalize` go to later slices.

**What you could not establish**
- **The three re-entry tests in the Array and Object stories:** `array-mutation-and-sort-methods` has `a_frozen_receiver_throws_and_keeps_its_elements`, and `array-search-and-copy-methods` has `with_to_reversed_and_to_spliced_leave_the_receiver_unchanged`. Neither names how it freezes or prevents extension. If either uses `Object.freeze` or `Object.preventExtensions` from script, that is a second hidden dependency on the integrity story. If it uses the Rust internal methods, it is fine.
- **The two design preconditions:** the source text kept alive and charged, and the counted refusal outside ASCII before slice 5. The stories `function-apply-and-source-text` and `string-transform-and-compare-methods` state both. They have no edge because a design change is not an artifact, so I did not count it as a finding.
- **Your point 3 (the 79 cases moved whole by `dir` lines):** the exception looks harmless. Directory lines in `expected.txt` count cases across all slices, so moving a passing case between slices changes no directory line. The close story's final comparison against `main` still holds. If the 9 stayed, they would keep passing, and they would run against the real method once slice 6 delivers it. I would rule for the move, because their pass proves nothing. The repository has no hard check against it: `AGENTS.md` only says a case that fails on its own slice is never moved.
- **Out of my lane (parallel safety and acceptance):**
  - Ten units write `builtins/*/mod.rs` mark lines, `realm/mod.rs` marks and `sections.txt`.
  - `object/ordinary.rs` is open to the JSON and integrity stories at once.
  - The own-property regexes in the acceptances use bare words such as `at`, `of`, `is` and `with`.
  - The "no run is Budget" check in the global story cannot be settled on its own branch for the 48 URI cases that call `String.fromCharCode`.
- **Two stories bundle independent small items:** `function-apply-and-source-text` joins `apply` and `toString`, and `global-this-uri-functions-and-is-error` joins `globalThis`, the URI functions and `Error.isError`. I judged neither a seam defect, because each part can be shown working alone and they share no abstraction.

```findings
- file: .engineering/planning/story/json-parse-and-raw-json.md
  line: 30
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "JSON.rawJSON must return a frozen object but the code has no SetIntegrityLevel and only story:object-integrity-and-prototype-methods adds it (integrity.rs, line 26 of that story), so the JSON unit depends on its internals with no depends_on edge; add depends_on story:object-integrity-and-prototype-methods, or move SetIntegrityLevel and TestIntegrityLevel into story:builtin-group-modules (ops/objects.rs) so both stories use them"
```
