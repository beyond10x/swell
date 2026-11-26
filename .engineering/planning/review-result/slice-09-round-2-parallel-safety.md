---
format: aep.planning-md/3
id: review-result:slice-09-round-2-parallel-safety
kind: review-result
status: active
title: 'Slice 9, round 2, the parallel-safety critic: approve, no finding'
relations:
- reviews: story:keyed-entry-table
- reviews: story:reflect-namespace
- reviews: story:map-and-set
- reviews: story:proxy-objects-and-access-traps
- reviews: story:weak-map-and-weak-set
- reviews: story:set-methods
- reviews: story:proxy-descriptor-prototype-and-extensibility-traps
- reviews: story:map-upsert-and-group-by
- reviews: story:host-object-overrides
- reviews: story:slice-09-conformance-close
revision: 1
---
Plan critic, parallel-safety, round 2 on the ten draft stories of
`epic:slice-09-collections-proxy-reflect` (`plan/slice-09` at `4aca865a`), 2026-11-26.
Verdict: approve, no finding; the 5 findings of round 1 are closed. The critic's answer, as returned:

approve

**Round-1 findings: all five closed**
1. `weak-map-and-weak-set` against proxy: closed. The edge is at `.engineering/planning/story/weak-map-and-weak-set.md:11` and the files are its reason at `:79`. CanBeHeldWeakly now goes after `same_value_zero` (`:69`). In the code that is `ops/testing.rs` lines 106–112, away from `is_array` (45–51) and the module comment (1–4) that proxy rewrites.
2. `map-and-set` beside proxy: closed. Each shared site now names the existing entry each story inserts after, and the two differ: `map-and-set.md:70, :73, :83` and `proxy-objects-and-access-traps.md:73, :75, :87`. I checked every pair against the code; the two inserts are always separated by unchanged lines:
   - `object/mod.rs`: 202/215, 232/236, 270/274, 294/298; `mod` lines after 44, against after 46 and 48.
   - `array.rs`: 343 against 329.
   - `collect.rs`: `KIND_SITES` 364/368; layout test 418 plus after 443, against after 458; the test `use` lines 321/326.
   - `builtin_function.rs`: 76/786 against 94/796.
   - `intrinsics.rs`: 168/277/334/409/603/662 against 150/273/329/404/595/658.
   - `global_object.rs`: 90 against 87.
   - `builtins/mod.rs`: `map` after `math` and `set` after `regexp`, against `proxy` after `promise_statics`.
3. `set-methods`: closed. `builtin_function.rs` is dropped and `SetBuiltin::Methods` is named (`set-methods.md:45, :55`).
4. `map-upsert-and-group-by`: closed. The file is dropped and `MapBuiltin::Upsert` is named (`map-upsert-and-group-by.md:51, :61`).
5. `keyed-entry-table` and proxy `mod` lines: closed. The shared point between `mod generator;` (line 46) and `mod ordinary;` (line 47) is named, with a sorted hand resolution, in `keyed-entry-table.md:50`, `proxy-objects-and-access-traps.md:87` and `host-object-overrides.md:121`.

**What the revision added, checked against `crates/`**
- **Edge weak→proxy:** it holds. Both stories add at the tails of `ObjectKind`, `KindTag`, `trace` and `tag`, and weak goes after proxy's entries in `KIND_SITES` and in the layout test. The edge adds no order. One small error in the stated reason: weak's text says the two share "the plain arm" of `array.rs`. They don't: proxy adds its own arm after line 329, and weak edits line 347. The other shared sites still justify the edge, so this is not a finding.
- **`KIND_SITES` as a slice** (`reflect-namespace.md:55`): line 351 becomes `&[&str]` and line 497 becomes `.chain(KIND_SITES)`. That type-checks against `ROOT_SITES.iter()`, and line 504 only calls `.len()`. Nothing else uses the constant, so no later story changes a length line.
- **Exhaustive matches:** the only exhaustive matches over `ObjectKind` and `KindTag` are the four lists in `object/mod.rs` and `array.rs` lines 326–348. The dispatch uses default arms, so there is no hidden site no story names.
- **`host-object-overrides` against stories it has no edge with:**
  - `array.rs`: it removes line 340 and adds an arm after line 331; `map-and-set` inserts after 343 and weak after 347.
  - `arena.rs`: its literal is at line 501; weak's workspace is lines 69–260.
  - `requirements.txt`: HOST-222 goes after line 221; `map-and-set` changes line 214.
  - `determinism.rs`: its literal is at line 499; `map-and-set` appends after the end (820).
  - No two of these touch.
- **Orders 3 and 4:** besides the generated `tests/main.rs`, the stories of order 3 share only conditional `builtins_symbol_values.rs` rows, and `map-upsert-and-group-by` shares nothing else with `host-object-overrides`.
- **Other additions:** `intern_string` (`heap/string.rs` line 315) and `atom_of` (line 289) exist and are only read. The invariant checks as functions, the facts box traced in `host_object.rs`, `Map.groupBy`, and `close-later.tsv` (written only by the coordinator) add no shared writer.

**What I read:** all 10 stories (head `4aca865a`), my round-1 record, `git diff 66f11d1b..4aca865a`, `revision-1.md`, `waves-section.md`, and the `check.sh`/`lists.sh` diff. Commands: `aep plan artifact waves --kind story --status draft --format json`, `grep`/`sed` on the tree. Surfaces: 10 cited, 0 inferred, 0 unplaced. Acceptance reads: 24 traced to a producer in the set, 22 of them recorded by an edge or a path of edges. The other 2 are in `host-object-overrides` (the `determinism::` filter and the `pending slice 9` count). Both read whatever `map-and-set` wrote, but they compare with the story's own base, so they pass with or without it.

**What I could not establish**
- **`builtins_symbol_values.rs`:** order 2 (`map-and-set` and proxy) and order 3 (weak and `set-methods`) write "rows only where a clause throws before a Symbol is read" and name no site in the file. Where a row lands follows the realm's traversal order. Every new clause I checked throws a TypeError, which needs no row, so I expect none. Not a finding.
- **`STACK_BYTES_PER_REENTRY`** (`limits.rs` line 46): every story may change it "if the measured stack moves". Two stories running at once could both do so. That is speculative and needs a build.
- **Out of my lane:** whether the cursor release and the weak clearing can read mark bits after `mark.give_back` is a design question.

```findings
[]
```
