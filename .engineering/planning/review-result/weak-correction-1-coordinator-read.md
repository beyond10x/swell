---
format: aep.planning-md/3
id: review-result:weak-correction-1-coordinator-read
kind: review-result
status: active
title: 'Coordinator''s read of correction 1 of the unit weak: no defect'
relations:
- reviews: story:weak-map-and-weak-set
revision: 1
---
Coordinator's read of correction 1 of the unit weak (impl/weak, 34c0e5d5 to 7d6e94c5),
2026-11-26. Verdict: no defect.

The correction answers review-result:adversary-weak-pass-1 (one red case of 10, four
findings). It is two commits.

Commit 17f4ef5d (4 files, 584 lines added, 22 removed; 476 of the added lines under
`crates/swell/src`, of them tests all but the lines read below):

- Finding 1: `weak_insert` charged the bytes of a larger table whenever the map had no spare
  room, also when `KeyedMap::reserve` then rehashed in place and took no byte. The rule "grow
  or rehash in place" now exists once: `KeyedMap::growth_for(additional)`
  (`heap/map.rs:320`) answers the room of the new table, or `None` when the map has the room
  or rehashes in place, and `reserve` (`:302`) takes its decision from it. Read: `reserve`
  returns early for `additional <= growth_left` as before, then calls `growth_for`, whose
  first test is the same one, so the two arms that follow are the two of the old body, with
  the same arithmetic (`checked_add`, `full_capacity / 2`, `new_items.max(full_capacity + 1)`
  saturating). `weak_insert` (`object/weak_collection.rs:350`) charges
  `table_bytes_for(room) - charged` for `Some(room)`, 0 for `None`, and `u64::MAX` when the
  count overflows, which ends the run in `Terminated(MemoryBudget)` like every charge over
  the budget. The adversary's red case, body unchanged, was red on the unit's head (`left:
  Some((74, 109)) right: None`) and is green; a second test,
  `a_rehash_in_place_counts_no_byte_toward_the_next_collection`, was red before the change
  (`left: 58368 right: 0`).
- The other owners that charge before `KeyedMap::reserve` or `insert` were read by the
  correction (the index of Map and Set, the property index, `Atoms`, `SymbolRegistry`, the
  visited set of for-in): none charges a rehash in place, so no backlog story follows.
  `Atoms` keeps its own copy of the rule (`heap/string.rs:490`).
- Findings 2 and 3: the adversary's cases
  `a_symbol_key_marked_after_its_map_was_traced_keeps_its_value` and
  `one_key_pending_in_many_tables_releases_every_table` stay; in a copy, each planted change
  turns its case red (`release_pending` returning early for a Symbol; `release` answering
  `Some(0)`).
- Finding 4: the doc comment of `the_ephemeron_pass_allocates_nothing` says what
  `collect_allocations` counts (pushes a worklist had no room for, pushes past capacity onto
  the sweep's free lists) and that a vector made in the ephemeron pass is not counted; what
  holds that is the code, which links each pending list through the arenas' head words and
  the `next` link of each entry.
- Added: `get_or_insert_computed_reentered_on_one_map_ends_at_the_reentry_limit` (limits 10
  and 20: a RangeError, the callback ran that many times, the map holds no entry of the
  key). Green at once; the behaviour existed.
- The adversary's 9 green cases stay under the unit's names.

Commit 7d6e94c5: the merge of `wave/big-2` at 91dc00d9. No conflict; no count or byte reading
of a test moved.

The story lists `heap/map.rs` under "Not written" and its acceptance runs `git diff --quiet`
over it; that check now exits 1 for `KeyedMap::growth_for`. The coordinator states this in the
story's "As delivered" at the wave's close.

Times, by the coordinator on 7d6e94c5 in the lane's collect build (test profile, opt-level
0, feature collect-every-safe-point, one thread; 23 tests whose name holds `weak` or
`ephemeron`): the slowest 1.231 s (`weak_collections_at_the_edges_of_their_clauses`), 1.175
s at the full counts; none over the 2 s and 2.5 s bounds. The correction measured all 29 of
the unit's tests alone in the default build (slowest 0.539 s) and in the gate's collect
profile (slowest 0.101 s).

On the merged head the implementor ran `task test-realm-sizes` (14 runs, 0 red) and the
story's 247 cases of test262 with the guard (`passed 403 bound 403 B 156 F 247 R 0`).
