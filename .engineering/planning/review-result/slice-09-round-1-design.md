---
format: aep.planning-md/3
id: review-result:slice-09-round-1-design
kind: review-result
status: active
title: 'Slice 9, round 1, the design critic: needs-revision, 8 findings'
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
Plan critic, design, round 1 on the ten draft stories of
`epic:slice-09-collections-proxy-reflect` (`plan/slice-09` at `33330e14`), 2026-11-26.
Verdict: needs-revision, 8 findings. The critic's answer, as returned:

needs-revision

story:keyed-entry-table — Nothing in the body releases the cursor of an iterator that is never finished. Map and Set iterators have no `return`, so even `for (k of m) break;` leaves a cursor in the table's list for as long as the Map lives. That list's growth is never charged, and compaction moves every listed cursor while charging only per entry, so the design cannot stay within the step and memory budgets. The body must name who releases the cursor of an abandoned iterator, or of a `forEach` that ends abruptly (a weak list needs the clearing pass of story:weak-map-and-weak-set, which comes later; the alternative is a release in the sweep), and charge both costs — .engineering/planning/story/keyed-entry-table.md:39, .engineering/planning/story/map-and-set.md:62, .engineering/planning/story/map-and-set.md:69

story:keyed-entry-table — The body says String keys are interned "with the interning of ops/conversion.rs lines 344 to 390". Those lines are `key_from_string`, which turns an array-index String such as "0" into `Key::Index`, with no atom, so the rule "a String with no atom is in no table" breaks for "0". `find` is charged one step, but hashing and interning a long String key costs work per code unit. The body must name `Heap::intern_string` and `Heap::atom_of` and charge code units as `key_from_string` does — .engineering/planning/story/keyed-entry-table.md:38, .engineering/planning/story/keyed-entry-table.md:40, crates/swell/src/ops/conversion.rs:363

story:proxy-objects-and-access-traps — It describes the six checks in `object/invariants.rs` as reads of the target's [[GetOwnProperty]] and [[IsExtensible]]. story:host-object-overrides calls these same checks with recorded facts in place of a target, and may not write `invariants.rs`. Only story:proxy-descriptor-prototype-and-extensibility-traps promises "functions of (target facts, trap result)", and only for its seven traps. The body must give its six checks that same shape — .engineering/planning/story/proxy-objects-and-access-traps.md:70, .engineering/planning/story/host-object-overrides.md:107, .engineering/planning/story/host-object-overrides.md:119, .engineering/planning/story/proxy-descriptor-prototype-and-extensibility-traps.md:35

story:host-object-overrides — The fast path `array_set_index` walks `record.prototype` directly and treats `ObjectKind::Host(_)` as plain. A host object with a `set`, `get_own_property` or `define_own_property` override in an Array's prototype chain is therefore skipped. The match already covers `Host`, so the compiler will not flag it. The body's Writes must add `object/array.rs` and say which overrides make that arm false — crates/swell/src/object/array.rs:340, .engineering/planning/story/host-object-overrides.md:106

story:host-object-overrides — The facts box records values, keys and a prototype, but the body never has the kind site `HostObject::trace` list them. The precise collector can then free a recorded value or atom and reuse its handle, and later checks compare against a stale handle. The body must have the kind site trace the facts — .engineering/planning/story/host-object-overrides.md:107, crates/swell/src/host/host_object.rs:157

story:host-object-overrides — Its acceptance line says `grep -c 'pending slice 9$'` "prints 0 once story:map-and-set has moved HOST-25 and HOST-214". That reads state story:map-and-set produces, and no `depends_on` edge runs from this story to story:map-and-set. The line should either state only "base count less 5" or get that edge — .engineering/planning/story/host-object-overrides.md:125, .engineering/planning/story/map-and-set.md:84

story:map-upsert-and-group-by — `Map.groupBy` collects its groups in a bare `EntryTable` and builds the Map afterwards. A Rust-held table is reached by no root site, and `keep` takes one Value at a time, so a collection inside the callback frees the keys and group Arrays. The body must make the result Map first and keep it while it fills, as `Object.groupBy` does with its object, or keep each key and group — .engineering/planning/story/map-upsert-and-group-by.md:51, .engineering/planning/story/map-upsert-and-group-by.md:52, crates/swell/src/builtins/object/from_entries.rs:137

epic:slice-09-collections-proxy-reflect — Its edge `depends_on: epic:slice-08-classes` says slice 8 comes first. The drafted set announces two edges the other way (slice 8's private-names story on story:host-object-overrides, and slice-08-conformance-close on story:slice-09-conformance-close) and says slice 9 is delivered first. Once those edges are recorded, each slice needs the other first. The epic edge must be dropped or turned around before slice 8 is decomposed — .engineering/planning/epic/slice-09-collections-proxy-reflect.md:9, .engineering/planning/story/host-object-overrides.md:94, .engineering/planning/story/slice-09-conformance-close.md:69

What I read: all 10 stories in full, the epic, the plan-page section, and `docs/plan/slice-09/scan.awk`. Commands, all read-only: `aep plan artifact relations`, `validate` (it reports only 7 scope warnings on active stories outside the set) and `graph --format json`. I walked all 284 `depends_on` edges of the whole store through `tsort`, which found no cycle; that includes the edges leaving the set, to `exponentiation-operator`, `slice-04-conformance-close` and the epics. I traced 22 acceptance reads to a producer in the set; edges cover 21 of them, directly or transitively, and the one without an edge is the finding above. I checked the designs against `object/mod.rs` (the dispatch is `Agent<H>`-generic and returns `Completion`, so Proxy and Host arms can be added without `unsafe` or a panic path), `object/ordinary.rs`, `object/array.rs`, `heap/map.rs` (no `iter`/`get_mut`, but `values()` and `Cell` links make the ephemeron pass buildable without touching `map.rs`), `heap/collect.rs`, `heap/string.rs`, `host/host_object.rs`, the design's section 15, and `from_entries.rs`.

What I could not establish:
- Whether `ObjectRecord` stays at 200 bytes (an exact `assert_eq`) once `HostObject` gains a table pointer and a facts box, and once Proxy and CollectionIterator are added unboxed. No build was allowed; each story's acceptance runs that test.
- Out of my lane, for the parallel-safety critic: three stories write the `KIND_SITES` length, `object/mod.rs` and `heap/arena.rs`.
- Out of my lane, for the acceptance critic: the extra collector-workspace word per slot from story:weak-map-and-weak-set moves the HOST-187 realm reading that sibling stories report before and after.

```findings
- file: .engineering/planning/story/keyed-entry-table.md
  line: 39
  category: design
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Nothing releases the cursor of an iterator that is never finished (Map and Set iterators have no return, so even for-of with break leaves one); the cursor list grows without a charge and compaction moves every listed cursor without charging for it, so the body must name the release (a weak list or the sweep) and charge both costs."
- file: .engineering/planning/story/keyed-entry-table.md
  line: 38
  category: design
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "The String key is interned with key_from_string (ops/conversion.rs 344 to 390), which makes an array-index String a Key::Index with no atom, and find is charged one step while hashing a String key costs work per code unit; the body must name Heap::intern_string and Heap::atom_of and charge code units."
- file: .engineering/planning/story/proxy-objects-and-access-traps.md
  line: 70
  category: design
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "The six access-trap checks of object/invariants.rs are written as reads of the target's methods, but story:host-object-overrides calls them with recorded facts and may not write that file, so the body must state them as functions of (target facts, trap result) as the shape-trap story does."
- file: crates/swell/src/object/array.rs
  line: 340
  category: design
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "The fast path array_set_index treats every host object as plain and reads record.prototype directly, so a host parent with a set or define override is skipped; story:host-object-overrides must add object/array.rs to its Writes and make that arm false for such objects."
- file: .engineering/planning/story/host-object-overrides.md
  line: 107
  category: design
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "The facts box holds values, keys and a prototype, but the body does not have the kind site HostObject::trace list them, so the collector can free a recorded value or atom and leave a stale handle behind."
- file: .engineering/planning/story/host-object-overrides.md
  line: 125
  category: design
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "The acceptance line that greps pending slice 9 reads the HOST-25 and HOST-214 lines that story:map-and-set moves, and no depends_on edge records that, so the line must state only the base count less 5 or get that edge."
- file: .engineering/planning/story/map-upsert-and-group-by.md
  line: 52
  category: design
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Map.groupBy collects its groups in a bare EntryTable that no root site reaches while the callback runs, so the body must make the result Map first and keep it, as Object.groupBy keeps its object, or keep each key and group."
- file: .engineering/planning/epic/slice-09-collections-proxy-reflect.md
  line: 9
  category: design
  severity: warning
  verdict: NEEDS-CHANGE
  origin: pre-existing
  message: "The epic depends_on epic:slice-08-classes while the drafted set announces slice 8 stories that depend on host-object-overrides and on slice-09-conformance-close, so the epic edge must be dropped or turned around before those edges are recorded, or slice 8 and slice 9 each need the other first."
```
