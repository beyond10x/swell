---
format: aep.planning-md/3
id: review-result:adversary-weak-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on the unit weak: red, one failing case of 10 and four findings'
relations:
- reviews: story:weak-map-and-weak-set
revision: 1
---
Adversary pass 1 on `impl/weak` at `34c0e5d5` (wave big-2, slice 9, order 3), run on 2026-11-26.
The report below is the adversary's last message; paths of the workstation are replaced.

```
unit: weak (story:weak-map-and-weak-set), branch impl/weak at 34c0e5d5, plus my uncommitted cases in the worktree
verdict: red
cases: executed 768→788, red 1
origin: introduced 3, pre-existing 1, undecided 0
wrote-outside-worktree: 17 files in <scratch>/wave2/weak/adversary-1 (part 6)
needs-coordinator: yes. My red case stays red until `weak_insert` changes. The coordinator decides whether to keep or drop the 10 cases.
```

**1. What I changed in the tree (`git --no-pager diff --stat`)**
```
 crates/swell/src/heap/collect.rs                | 279 ++++++++++++++++++++++++
 crates/swell/src/object/weak_collection.rs      | 130 +++++++++++
 crates/swell/tests/builtins_weak_collections.rs |  77 +++++++
 3 files changed, 486 insertions(+)
```
`collect.rs` and `weak_collection.rs` are implementation files. Every hunk I added is inside their `mod tests` (`@@ -1036,0 +1037` with `mod tests` at line 414, and `@@ -549,0 +550` with `mod tests` at line 389), as the brief allows. No implementation line changed. I ran `cargo fmt --all` on my hunks, and `slot --timeout 20m task clippy` exits 0.

**2. Cases I added**

The red case:
- `object::weak_collection::tests::adversary_a_weak_map_used_as_a_cache_rehashes_in_place_within_its_budget` (`weak_collection.rs:656`). A WeakMap used as a cache: first filled to 896 entries (a table of 1,024 buckets), then from round 8 on it keeps at most 224 live keys. With a large budget the run shows the table's charge stays at 58,384 bytes the whole time, one rehash in place happens, and the highest `heap_bytes()` is 287,872. The same run with a budget of that highest reading plus 29,192 bytes needs no extra byte, but it ends at the budget. It is red in both modes. Output of the case run alone, default mode:
```
panicked at crates/swell/src/object/weak_collection.rs:671:9:
assertion `left == right` failed: the same churn with the budget 317064 (29192 more than its most heap_bytes() with a large budget, the table 58384 bytes) ended at (round, insertion); Churned { peak: 287872, in_place: 1, charged: (58384, 58384), ended: None }
  left: Some((74, 109))
 right: None
Summary [   0.072s] 1 test run: 0 passed, 1 failed, 267 skipped
```
Collect mode gave the same panic, `1 test run: 0 passed, 1 failed, 263 skipped`, exit 100.

Two earlier versions of this case failed for reasons other than the one claimed, so they are not findings. I rewrote both before the version above:
- A refill after clearing everything failed its precondition (`precondition: the refill rehashes the table in place`). A collection that clears every entry leaves empty buckets, not deleted ones, so no rehash happened.
- My first churn version checked the room as len + spare, which deleted buckets also lower. It failed on `rooms: (127, 896)`.

Green cases, each run alone in both modes before the suite:
- In `heap::collect::tests`:
  - `adversary_a_symbol_key_marked_after_its_map_was_traced_keeps_its_value` (:1041)
  - `adversary_one_key_pending_in_many_tables_releases_every_table` (:1082)
  - `adversary_a_cycle_of_ephemerons_across_tables_is_kept_or_cleared_whole` (:1137)
  - `adversary_one_table_chain_of_1000_entries` (:1209)
  - `adversary_one_table_chain_of_10000_entries` (:1215)
  - `adversary_a_slot_left_pending_is_clean_when_it_is_used_again` (:1224)
  - `adversary_a_symbol_reached_only_through_its_description_is_cleared` (:1277)
- In `builtins_weak_collections::`:
  - `adversary_weak_collections_at_the_edges_of_their_clauses` (:379)
  - `adversary_weak_maps_filled_and_dropped_leave_the_ledger_where_it_was` (:423)

Times: none over 5 s. Over 1 s are both integration cases in collect mode, in the parallel suite run: the edges case 2.417 s and the ledger case 1.259 s, wall time; I did not measure CPU time. The 2.417 s may be over the story's 2 CPU-second bound.

**3. Suite run (`suite.sh`, after the cases existed)**

Commands: the acceptance filter on `--test main`, and `--lib`, each in both modes.
- main, default: `128 tests run: 128 passed, 1191 skipped`, exit 0
- lib, default: `268 tests run: 267 passed, 1 failed`, exit 100
- main, collect: `128 tests run: 128 passed, 1191 skipped`, exit 0
- lib, collect: `264 tests run: 263 passed, 1 failed`, exit 100

The failing test in both lib runs is the red case above. Before my cases the four runs counted 126 + 260 + 126 + 256 = 768 (the implementor's report). After: 128 + 268 + 128 + 264 = 788.

**4. Findings**

- **The precharge stops a WeakMap at its budget when nothing grows.** Severity: warning. Verdict: NEEDS-CHANGE. Origin: introduced.
  - Where: `weak_collection.rs:348`. When `spare() == 0`, `weak_insert` charges `table_bytes_for(room + 1) - charged`, about one more table, before it inserts. `KeyedMap::reserve` then rehashes in place, because the entries are at most half the room. The charge is credited right after, but a budget with less headroom than that ends the run in `Terminated(MemoryBudget)`. The transient charge also counts toward `since_collection`.
  - What reaches it: any script with a WeakMap or WeakSet whose keys die over collections, which is the normal cache use. Deleted buckets left by `clear_unmarked` and `delete` use up `growth_left`. Measured: the rehash came at round 74 with 224 live entries.
  - Fix to name: charge for growth only when `len + 1 > room / 2`; otherwise charge 0.
- **The unit's tests do not cover a Symbol key marked after its map was traced.** Severity: warning. Verdict: CONFIRMED. Origin: introduced.
  - Where: `collect.rs:342`. I planted `release_pending` returning early for `Item::Symbol` in a scratch clone (`mutants.patch`). The unit's lib tests in both modes and `--test main` filtered to `builtins_weak_collections|builtins_map_and_set|collect` in collect mode stayed green (`46 tests run: 46 passed`). Only my `adversary_a_symbol_key_marked_after_its_map_was_traced_keeps_its_value` went red: `holder first true: the value of a live Symbol key was freed`.
  - What reaches it: any WeakMap keyed by a Symbol that only an object traced after the map reaches. Without the release, the value of a live key is freed.
- **The unit's tests do not cover a key pending in two or more tables.** Severity: warning. Verdict: CONFIRMED. Origin: introduced.
  - Where: `weak_collection.rs:193`. I planted `release` answering `Some(0)`, which cuts the list after its first table. The unit's tests stayed green in the same runs. Only `adversary_one_key_pending_in_many_tables_releases_every_table` went red: `2 tables, reached true: values left: 1 right: 2`.
  - What reaches it: one object used as a key in two WeakMaps (and, later, private names).
- **The no-allocation test cannot see a vector made in the ephemeron pass.** Severity: note. Verdict: CONFIRMED. Origin: pre-existing.
  - Where: `collect.rs:404`. `collect_allocations` counts only worklist overflows and free-list growths; base `collect.rs:308` counts the same way. So `the_ephemeron_pass_allocates_nothing` would stay green if `release_pending` collected into a `Vec`. Its doc comment says it "counts no vector made or grown".
  - What reaches it: nothing at run time; this is a test that cannot fail for the defect it names.

Tree covered: 34c0e5d5 plus my uncommitted cases.

**5. What I attacked and could not break**
- **Ephemeron rules:**
  - A cycle across three tables, including a map reached only as a value, is kept or cleared as a whole.
  - A value that reaches its own key, and the map as its own key and value, are handled correctly.
  - A Symbol reached only through its description String is cleared.
  - Head words are clean when a slot is reused. A planted "heads not zeroed" mutant is also caught by the unit's `the_ephemeron_pass_allocates_nothing`.
- **Linear time:** one-table chain of 1k / 10k / 100k entries took 0.008 / 0.045 / 0.433 s in default mode and 0.039 / 0.124 / 0.722 s in collect mode. The 100k probe ran once and was removed.
- **Ledger:** 10,000 rounds of maps and sets made and dropped, plus a long-lived map, read [2098151, 2098151, 2098151]. 300 rounds in collect mode read [384203 ×3].
- **Script-level edges:** Proxy and revoked Proxy keys, function keys, the map as its own key, brand checks through a Proxy and on the prototype, String iterables, a non-callable adder refused before the iterable opens, the iterator closed when the adder throws or an entry is not an object, re-entrant `getOrInsertComputed` with the same key, and rehashes inside the callback.
- **Planted switch:** it is a `#[cfg(test)]` field on each table, so it cannot leak between tests.
- **Code rules:** about 1,000 lines read. No `unwrap`, `expect` or indexing outside tests. slot + 1 cannot overflow, because `max_slots()` is `u32::MAX`.
- **Not attacked:** the stack profiles and `STACK_BYTES_PER_REENTRY`, a second `test-realm-sizes` run, cross-realm Symbols and objects, `fault-injection`, and the test262 part.

**6. Files I wrote outside the worktree**

All in `<scratch>/wave2/weak/adversary-1/`:
- scripts: `red-alone.sh`, `red-refill.sh`, `red-churn.sh`, `mutants.sh`, `suite.sh`
- logs: `alone.log`, `refill.log`, `churn.log`, `mutants.log`, `suite.log`, `fmt.log`, `clippy.log`
- markers: `alone.done`, `refill.done`, `mutants.done`, `suite.done`
- `mutants.patch`

The scratch clone `adversary-1/mutant` (1.8 GB, almost all `target/`) is deleted.

```findings
- file: crates/swell/src/object/weak_collection.rs
  line: 348
  category: boundary
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: weak_insert charges about one more table before a rehash in place that needs no byte, so a WeakMap used as a cache ends in Terminated(MemoryBudget) with headroom it never uses (red in both modes)
- file: crates/swell/src/heap/collect.rs
  line: 342
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: a planted release_pending that skips Symbol items passes every test of the unit; only the added case for a Symbol key marked after its map was traced catches the freed value
- file: crates/swell/src/object/weak_collection.rs
  line: 193
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: a planted release that cuts the pending list after its first table passes every test of the unit; only the added case with one key in 2 and 1,000 tables catches it
- file: crates/swell/src/heap/collect.rs
  line: 404
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: collect_allocations counts only worklist overflows and free-list growths, so the_ephemeron_pass_allocates_nothing cannot fail on a vector made in the ephemeron pass, though its doc comment says it counts every vector made or grown
```
