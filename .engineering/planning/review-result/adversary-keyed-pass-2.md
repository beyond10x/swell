---
format: aep.planning-md/3
id: review-result:adversary-keyed-pass-2
kind: review-result
status: active
title: 'Adversary pass 2 on the unit keyed: no red case, 7 findings, 4 green cases to keep'
relations:
- reviews: story:keyed-entry-table
revision: 1
---
Adversary, pass 2 (the last) on the unit keyed of wave big-2 (impl/keyed at 61a171bf), 2026-11-26.
Verdict: needs-change. 7 findings (1 warning, 6 notes), all introduced; 4 cases added, none red.

unit: keyed (story:keyed-entry-table), branch impl/keyed at 61a171bf plus 329 uncommitted test lines
verdict: NEEDS-CHANGE
cases: executed 35→39, red 0
origin: introduced 7 / pre-existing 0 / undecided 0
wrote-outside-worktree: 5 paths (part 6)
needs-coordinator: yes. (a) The u32 limits are a gap of the whole engine, not of this unit (finding 7). (b) Which termination should an exhausted u64 counter give: `MemoryBudget` or `EngineFault` (finding 2)?

**1. `git --no-pager diff --stat`**
```
 crates/swell/src/object/keyed_entries.rs | 329 +++++++++++++++++++++++++++++++
 1 file changed, 329 insertions(+)
```
Every added line is inside `mod tests`, after the last test. No implementation line changed and no mutant was planted.

**2. Cases added** (`crates/swell/src/object/keyed_entries.rs`). I wrote all of them before anything ran. The first run was these cases alone, default mode: `4 tests run: 4 passed, 247 skipped`, exit 0. None was red.
- `:2587` `adversary_a_compaction_at_every_cursor_position_and_every_budget_matches_the_model`:
  - Tables: all 255 hole patterns of an 8-position table without an index map, plus 260 patterns of 16 and 32 positions with one.
  - A cursor at every position including holes and the end, and 2 free places in the cursor list.
  - Every budget below the charge: `StepBudget`, nothing spent, table, map and cursors unchanged. At the charge it spends exactly `moved + places`.
  - After it: `find` agrees with the model, and each cursor visits the model's entries plus an entry added afterwards. Green, 0.21 s.
- `:2749` `adversary_the_generation_counter_ends_the_run_at_its_last_value`: the counter is set to `u64::MAX - 1`. The open after that ends in `MemoryBudget` on both paths (push and free list), with everything unchanged. Green.
- `:2799` `adversary_an_id_stays_stale_through_reuse_compaction_clear_and_a_remade_list`: one place reused twice gets two ids. Old ids stay stale across a compaction, `clear` and a remade list. Green.
- `:2847` `adversary_clear_charges_its_holes_by_64`: 1 live entry and 999 holes; `clear` spends 16. Green.
- `:2869` `adversary_measure_clear_against_insert`: `#[ignore]`, a measurement only (part 5). 4.5 s.

**3. Suite runs (after the cases existed)**
- `SLOT_TIMEOUT=20m cargo nextest run --locked -p swell --lib --no-fail-fast -E 'test(/^object::keyed_entries::/)'`: `39 tests run: 39 passed, 212 skipped`, exit 0.
- The same with `--features collect-every-safe-point`: `39 tests run: 39 passed, 208 skipped`, exit 0.
- Slowest: the model test at 0.52 s, my case at 0.21 s. Pool jobs used: 4.

**4. Findings** (they cover 61a171bf plus the test diff)
1. **The generation counter's end has no test.** At `keyed_entries.rs:705`, `AGENTS.md:123` says a new limit gets a test that reaches it, and the test module can set `next_generation` directly. Nothing in the 35 tests writes that field, so a `wrapping_add` mutant would stay green; `:2749` catches it. What reaches it: 2^64 opens, so nobody does. The fix is to keep `:2749`. NEEDS-CHANGE, warning.
2. **The counter's end answers `MemoryBudget`** (`:706`). The engine's precedent for an unreachable u64 counter, `create_realm` (`host/agent.rs:440-442`), answers `EngineFault`. `MemoryBudget` also sets `collect_pending`, and a collection cannot help here. What reaches it: nothing. Note.
3. **Mutant, by reading:** remove `self.next_generation = next_generation;` on the free-list path (`:724`). Two opens at the same place would then hand out the same id. `a_stale_cursor_id_reaches_nothing` never reads an id after a second open at its place. `:2799` (`assert_ne!(t, u)`) catches it. Note.
4. **Mutant, by reading:** `entries.len()` changed to `live` in `clear` (`:641`). The unit's formula test (`:2451`, 990 live, 10 holes) reads 16 steps either way. `:2847` reads 16 against 1. Note.
5. **Stale wording in the story files.** The Bounds line of `keyed-entry-table.md:44` still says "one per cursor it moves / resets". The test text of `map-and-set.md:80` says "one per cursor the compaction moves". The numbers in both still hold, because the list is freed there and has 0 places. Only the coordinator can edit these files. Note.
6. **An id from table A acts on table B** when place and generation match. Each table's counter starts at 0, and `open_at` (`:761`) compares only those two. What reaches it: nothing. A `CollectionIterator` holds its own collection and id (`map-and-set.md:75`). INFEASIBLE, note.
7. **The u32 limits (`:404`, `:731`) still have no test.** No limit of this kind in the engine has one:
   - `max_slots` (`heap/arena.rs:64`)
   - generation retirement (`arena.rs:374-384`, `roots.rs:196`)
   - agent keys (`agent.rs:69-78`)
   - the realm counter (`agent.rs:440`)

   I accept filing this once for the engine. INFEASIBLE, note.

**5. Attacked and not broken**
- **Compaction:**
  - The charge is taken in full before any work, so no budget can stop it halfway (`:2587`, every budget).
  - Cursors on a live entry, on a hole in the middle, on a hole at the start and at the end all land right. Nothing can stand past the end: the position never exceeds `len`.
- **Stale ids:** stale across `clear`, `release_cursors`, a compaction and a remade list.
- **Stale `advance` costs no step, but no script loop is free:** the interpreter charges a step per instruction and per call (`interp/budget.rs:5`).
- **`clear` cost:**
  - Per position, `clear` only frees two allocations of `Copy` data and does not touch atoms.
  - Measured: 1,000,000 entries cleared in 10.9 ms for 15,625 steps (697 ns per step), against 2,816 ns per step for an insert. 100,000 String keys: 6.5 µs for 1,563 steps.
  - No other site in the engine charges per 64; the 64 comes from the story.
- **Bytes:** no constant 40 is left; `:737` reads `size_of::<CursorSlot>()`. The agent's count returns to its start in all three cases asked: 1,000 open/close, `clear` with cursors open, and the drop.
  - In the original 16 tests no assertion changed. Only a doc comment changed (`:1502-1505`: it spends 600 and 500, within the story's 601 and 501).
  - Their byte checks compare readings with each other, not literal values, so the new sizes do not touch them.
- **Code rules:** I read 892 lines outside the tests, including the 140 the correction added. No `unwrap`, `expect` or indexing. Arithmetic is checked or saturating; `:519` and `:524` are guarded.

**6. Paths written outside the worktree** (under `<scratch>/wave2/keyed/`)
- `adversary-2/` (the directory)
- `adversary-2/alone-default.log`
- `adversary-2/suite-default.log`
- `adversary-2/suite-collect.log`
- `adversary-2/measure.log`

```findings
- file: crates/swell/src/object/keyed_entries.rs
  line: 705
  category: boundary
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: The generation counter's end is a new limit with no test, although a test can set next_generation and reach it (the adversary's green case at keyed_entries.rs:2749), so a wrapping counter would stay green.
- file: crates/swell/src/object/keyed_entries.rs
  line: 706
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: The exhausted generation counter answers MemoryBudget, while the engine's unreachable u64 realm counter answers EngineFault (host/agent.rs:440), and the collection that MemoryBudget asks for cannot help.
- file: crates/swell/src/object/keyed_entries.rs
  line: 724
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: By reading, no test of the 35 reads an id after a second open at its place, so dropping the counter's advance on the free-list path stays green; keyed_entries.rs:2799 catches it.
- file: crates/swell/src/object/keyed_entries.rs
  line: 641
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: The unit's clear formula test reads 16 steps for both live and live plus holes, so counting live entries only stays green; keyed_entries.rs:2847 catches it.
- file: .engineering/planning/story/keyed-entry-table.md
  line: 44
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: The story's Bounds and map-and-set.md:80 still charge per cursor moved or reset, while the code now charges every place of the cursor list.
- file: crates/swell/src/object/keyed_entries.rs
  line: 761
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: An id of one table acts on another table's cursor with the same place and generation, since every counter starts at 0 and no table id is carried; no caller passes an id across tables.
- file: crates/swell/src/object/keyed_entries.rs
  line: 404
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: The u32 limits on entry positions and cursor places still have no test, and no limit of this kind in the engine has one (arena max_slots, generation retirement, agent keys, realm counter), so it is one engine-wide gap.
```
