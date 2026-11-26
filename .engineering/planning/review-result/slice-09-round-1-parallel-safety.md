---
format: aep.planning-md/3
id: review-result:slice-09-round-1-parallel-safety
kind: review-result
status: active
title: 'Slice 9, round 1, the parallel-safety critic: needs-revision, 5 findings'
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
Plan critic, parallel safety, round 1 on the ten draft stories of
`epic:slice-09-collections-proxy-reflect` (`plan/slice-09` at `33330e14`), 2026-11-26.
Verdict: needs-revision, 5 findings. The critic's answer, as returned:

needs-revision

story:weak-map-and-weak-set — No `depends_on` edge orders this story against story:proxy-objects-and-access-traps, and its Writes line never names that story. Yet both write the same tails of ObjectKind, KindTag, `trace` and `tag` (object/mod.rs 215, 236, 274, 298), KIND_SITES and its length (heap/collect.rs 351, 368), BuiltinId and `run_builtin` (builtin_function.rs 126, 826), the Intrinsic sites and the global list after `"Date"` (global_object.rs 95). In ops/testing.rs, CanBeHeldWeakly goes "beside IsArray", next to the `is_array` body and module comment that proxy rewrites (lines 1-4, 46-51). The story's line 64 even counts 24 KIND_SITES "after" proxy without an edge to it. The fix is a `depends_on` edge to story:proxy-objects-and-access-traps with these files as its reason, or a named anchor per site that differs from proxy's and CanBeHeldWeakly placed away from `is_array` (for example after `same_value_zero`, line 107) — .engineering/planning/story/weak-map-and-weak-set.md:76, :64, :66; .engineering/planning/story/proxy-objects-and-access-traps.md:83 (both cited)

story:map-and-set — It calls its shared hunks with story:proxy-objects-and-access-traps "hunks of its own", but neither body names an anchor, only whole lists (ObjectKind and KindTag "beside" each other, BuiltinId lines 57-127 and 759, intrinsics lines 67/237/366/532, global_object 64-90, "a record" in the layout test). Both therefore append at the same tails and conflict at each one: object/mod.rs 215, 236, 274, 298; builtin_function.rs 126, 826; intrinsics.rs 211, 282, 345, 416, 667; global_object.rs 95; collect.rs 418 or 460. The fix is, for each site, an existing entry this story inserts after that differs from proxy's, or the same written hand-resolution the body already gives for KIND_SITES — .engineering/planning/story/map-and-set.md:80, :68, :70; .engineering/planning/story/proxy-objects-and-access-traps.md:83 (both cited)

story:set-methods — It claims object/builtin_function.rs for "seven BuiltinId variants". The group convention puts a group file's ids in one variant of the parent's id enum, in the parent's mod.rs: builtins/mod.rs 28-43, and `ArrayBuiltin::Iteration` at builtins/array/mod.rs 74-75. Here that is `SetBuiltin` in builtins/set/mod.rs. So the claim adds a file the story need not write, and with it collisions with story:weak-map-and-weak-set, story:proxy-objects-and-access-traps and story:map-upsert-and-group-by. The fix is to drop the file from Writes and scope and name the SetBuiltin variant and arm in set/mod.rs — .engineering/planning/story/set-methods.md:57, :47 (cited)

story:map-upsert-and-group-by — Same defect: "three BuiltinId variants" in object/builtin_function.rs, where the convention puts one variant of `MapBuiltin` in builtins/map/mod.rs. This creates collisions with story:set-methods and story:proxy-objects-and-access-traps, which have no edge to it and which its Writes line does not name. The fix is to drop the file and name the MapBuiltin variant and arm — .engineering/planning/story/map-upsert-and-group-by.md:63, :53 (cited)

story:keyed-entry-table — Its Writes line says every later writer of object/mod.rs "is ordered after this story or does not touch this hunk". That is false for story:proxy-objects-and-access-traps: no path of edges joins the two, and its `mod invariants;` goes where rustfmt's module order also puts `mod keyed_entries;`, between `mod generator;` (line 46) and `mod ordinary;` (line 47). The two conflict if proxy branches before this story merges. The fix is to name that shared insertion point, or add a `depends_on` edge from proxy-objects-and-access-traps to this story with that line as its reason — .engineering/planning/story/keyed-entry-table.md:49; .engineering/planning/story/proxy-objects-and-access-traps.md:83 (cited, checked against the tree)

**The four files the store sequences by** (checked against the tree at `33330e14`)
- **`crates/swell/tests/main.rs`**: merges cleanly, nothing to add. The file is generated, one `mod` line per test file in byte order, and each story run at the same time inserts at a different point:
  - `builtins_map_and_set` and `builtins_map_upsert` after line 24
  - `builtins_reflect` after line 32
  - `builtins_set_methods` after line 33
  - `builtins_weak_collections` after line 40
  - `host_overrides` after line 62
  - `proxy` and `proxy_shape_traps` after line 79

  The coordinator regenerates it on each join anyway. All main.rs collisions in the store are false positives.
- **`crates/swell/src/object/mod.rs`**:
  - The `mod` lines are separate except `keyed_entries` / `invariants` (finding 5).
  - The `use` lines are separate.
  - The kind tails (215, 236, 274, 298) are one shared point for map-and-set, proxy and weak (findings 1 and 2).
  - The dispatch arms (402-690) are written only by proxy, then by host after it through edges.
  - Host's comment line 15 and its Host arms do not touch weak's kind lines.
- **`crates/swell/src/heap/collect.rs`**: these hunks are not separate.
  - The array length on line 351 is changed by map-and-set, proxy and weak, so a conflict is certain. map-and-set:80 and proxy:83 state the hand resolution; weak:76 does not.
  - The layout-test record has no named anchor.
  - The sorted `use` lines (309-333) and weak's mark-phase and test hunks are separate.
  - Making `KIND_SITES` a slice (`&[&str]`, lines 351 and 497) would remove the length line from every story's change.
- **`crates/swell/src/object/builtin_function.rs`**: every story appends at 126 and 826, a conflict for map-and-set with proxy and for weak with proxy. set-methods and map-upsert should not be in this file at all (findings 3 and 4). reflect is ordered ahead of all of them. proxy's `Slots` hunk (line 255 and its `trace` at 268) is its alone.

**The merge order the coordinator needs** (5 rounds, the decomposer's 5 orders, once the findings are fixed):
1. keyed-entry-table and reflect-namespace, in any order. Merge keyed before proxy's branch is cut, or fix finding 5.
2. map-and-set and proxy-objects, in any order. The second fixes collect.rs:351 by hand, plus every tail without an anchor.
3. proxy-descriptor after proxy. set-methods after map-and-set; once narrowed, it shares only main.rs with the other two. weak after map-and-set and, until finding 2 is fixed, after proxy.
4. map-upsert after weak, host after proxy-descriptor, in any order between them. Whichever of host and map-and-set merges second regenerates `website/data/status.json`. Host's other hunks are separate from map-and-set's and weak's: `requirements.txt` lines 25/214 against 41/112-114/222, `determinism.rs` 499 against the end, `arena.rs` 501 against 69-260.
5. slice-09-conformance-close.

**What I read:** all 10 stories in full, the plan-page section, and the code behind each shared hunk. Commands: `aep plan artifact waves --kind story --status draft --format json` (collisions and unassessed), `aep plan artifact graph`, `grep` and `sed` on the tree. Surfaces: 10 cited, 0 inferred, 0 unplaced. Acceptance reads: 24 traced to a producer in the set. An edge or a path of edges records 23 of them. The one left: host-object-overrides' `pending slice 9` count reads what map-and-set writes, and holds through its base-relative wording.

**What I could not establish**
- Doc-comment tails that past stories extended: object/mod.rs 1-12, builtins/mod.rs 1-27, global_object.rs 56-73. No body says whether it writes them; if several do, those are more shared insertion points (inferred).
- Whether the realm byte reading HOST-187 checks still passes once several new intrinsics and weak's extra word per slot land together. This needs a build on the join.
- A story running beside host-object-overrides that adds a new `HostObjectSpec` literal would break the join's compile. Host's rule for that covers slice 8 only (host-object-overrides.md:119); no slice-9 body plans such a literal.
- Out of my lane:
  - reflect's "13 BuiltinId variants" goes against the group convention (design critic).
  - proxy-descriptor's edge to story:exponentiation-operator and the sharing with slice 8 are outside the set.

```findings
- file: .engineering/planning/story/weak-map-and-weak-set.md
  line: 76
  category: parallel-safety
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "no depends_on edge orders this story against story:proxy-objects-and-access-traps and its Writes line never names it, yet both append at the tails of ObjectKind, KindTag, trace and tag (object/mod.rs 215, 236, 274, 298), KIND_SITES and its length (heap/collect.rs 351, 368), BuiltinId and run_builtin (builtin_function.rs 126, 826), the Intrinsic sites and the global list after Date (global_object.rs 95), and CanBeHeldWeakly beside IsArray touches the is_array body and module comment proxy rewrites (ops/testing.rs 1-4, 46-51); add a depends_on edge with these files as its reason, or name a distinct anchor per site and place CanBeHeldWeakly away from is_array"
- file: .engineering/planning/story/map-and-set.md
  line: 80
  category: parallel-safety
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the hunks shared with story:proxy-objects-and-access-traps are called its own but neither body names an anchor, so both append at the same tails (object/mod.rs 215, 236, 274, 298; builtin_function.rs 126, 826; intrinsics.rs 211, 282, 345, 416, 667; global_object.rs 95; the layout-test record in heap/collect.rs) and conflict on each; name per site an existing entry this story inserts after that differs from proxy's, or state the hand resolution as the body already does for KIND_SITES"
- file: .engineering/planning/story/set-methods.md
  line: 57
  category: parallel-safety
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the story claims object/builtin_function.rs for seven BuiltinId variants although the group convention (builtins/mod.rs 28-43, ArrayBuiltin::Iteration at builtins/array/mod.rs 75) puts them in one SetBuiltin variant in builtins/set/mod.rs, which widens its surface into collisions with weak-map-and-weak-set, proxy-objects-and-access-traps and map-upsert-and-group-by; drop the file from Writes and scope and name the SetBuiltin variant and arm"
- file: .engineering/planning/story/map-upsert-and-group-by.md
  line: 63
  category: parallel-safety
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the story claims object/builtin_function.rs for three BuiltinId variants although the group convention puts them in one MapBuiltin variant in builtins/map/mod.rs, which creates collisions with set-methods and proxy-objects-and-access-traps that have no edge to it and that its Writes line does not name; drop the file and name the MapBuiltin variant and arm"
- file: .engineering/planning/story/keyed-entry-table.md
  line: 49
  category: parallel-safety
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the Writes line says every later writer of object/mod.rs is ordered after this story or does not touch its hunk, but story:proxy-objects-and-access-traps has no path of edges to it and adds mod invariants at the same place between mod generator (line 46) and mod ordinary (line 47) where mod keyed_entries goes; name that shared insertion point or add a depends_on edge from proxy-objects-and-access-traps to this story"
```
