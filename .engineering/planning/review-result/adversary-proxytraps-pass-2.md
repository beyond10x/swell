---
format: aep.planning-md/3
id: review-result:adversary-proxytraps-pass-2
kind: review-result
status: active
title: 'Adversary pass 2 on the unit proxytraps: red, for-in drops keys when a getPrototypeOf trap answers anew'
relations:
- reviews: story:proxy-descriptor-prototype-and-extensibility-traps
revision: 1
---
Adversary pass 2 (the last) on `impl/proxytraps` at `61b14ed2` (wave big-2, slice 9, order 3), on correction 1, run on 2026-11-27.
The report below is the adversary's last message; paths of the workstation are replaced.

**Verdict: red.** One case I added fails in both modes: when a Proxy's `getPrototypeOf` trap gives a different answer on its second call, `for`-`in` drops a key it should visit. That was introduced by the correction. I ran all 12 mutants and plants. Two mutants survive every test, mine included, because no public reading can see them. The unit's own tests are all within the time limits.

```
unit: proxytraps
verdict: red
cases: executed 218→232, red 1
origin: introduced 6, pre-existing 2, undecided 1
wrote-outside-worktree: <scratch>/wave2/proxytraps/adversary-2/
needs-coordinator: no
```

The case counts are per mode: default 119→127, collect 99→105.

## Findings

1. **`for`-`in` drops a prototype's keys (warning, introduced in `2b7317b`).** The walk follows the prototype the trap answers first, `{a:1}`, and finds `a`.
   - The filter at `ops/objects.rs:824-848` (called from `interp/dispatch.rs:1692`) starts again from the receiver and asks the trap a second time. It gets `{b:2}`, so `a` is dropped.
   - `a` is never deleted. The spec clause only lets a key be skipped if it was deleted before its turn.
   - Red case `adv2_for_in_visits_the_keys_of_the_prototype_the_trap_answered_first`: expected `"a"`, got `""`. Pass-1 finding 4 is only partly fixed.
2. **Two hold mutants pass every unit test (warning, introduced).**
   - m1: `keep_visited` removed (`objects.rs:784`).
   - m2: the keys of ordinary objects after the first Proxy not held (`objects.rs:789`).
   - Both end in `Terminated(EngineFault)` in my new hold cases, in both modes. The unit's cases miss them: their keys appear as literals in the script, so the code holds their atoms anyway.
3. **The filter's release mutant passes every unit test (warning, introduced).** With `release(mark)` removed at `objects.rs:818`, only my new case `adv2_for_in_in_a_loop_gives_back_what_each_loop_held` goes red, in both modes.
4. **Two mutants survive everything (note).** No public reading (`heap_bytes`, `live_bytes`) shows the kept stack's capacity, so I could not write a test that catches them.
   - m3: `reserve_kept(room, n-1)` at `objects.rs:235` under-charges one value, and only when a growth lands exactly at the need.
   - `end_kept_room` removed at `objects.rs:756`: `for_in_start` releases the values anyway, so only the grown capacity stays, uncharged.
   - Catching them needs a check inside the crate, which is implementation code I may not edit.
5. **The filter makes extra trap calls (note, pre-existing).**
   - Engine log: `p.ownKeys p.gopd:a p.gopd:b p.getPrototypeOf q.ownKeys q.gopd:c q.getPrototypeOf p.gopd:a body:a p.gopd:b body:b p.gopd:c p.getPrototypeOf q.gopd:c body:c`.
   - The clause's informative order is lazy and per object: `p.ownKeys, p.gopd:a, body:a, p.gopd:b, body:b, p.getPrototypeOf, q.ownKeys, q.gopd:c, body:c, q.getPrototypeOf`.
   - That is 5 extra calls. With the old `has` filter planted back there were 4 extra `has` calls.
6. **A cycle of `getPrototypeOf` answers is only ended by the step budget (note, pre-existing).** `for (k in cyc)` ends in `Terminated(StepBudget)` and the script cannot catch it.
7. **Values kept outside a room (note, undecided).** `docs/design/engine.md:2112` allows at most 10 values kept outside a room per running builtin. I count about 13 in `proxy_define_own_property_steps` (`object/proxy.rs:574`) and about 11 in `proxy_get_own_property_steps`. I did not confirm these counts by a run.

## Brief items

1. **The sweep's 105 sites:** I read about 50 in code: every direct call of the seven Proxy methods at head, plus the wrapper sites. The other ~55 I took from `sweep.md` without reading. No unheld value found.
2. **Memory budget at every 64 bytes** (default mode):
   - for-in over 3 Proxies × 20 keys: 467 runs ended inside it, 44 answered. Rooms 3,761–13,883 bytes, widest step 64.
   - `JSON.stringify` of the same: 356 ended inside it, 26 answered. Rooms 3,761–10,983 bytes, widest step 64.
   - In every run the outcome was the answer or `MemoryBudget`, and `live_bytes`/`heap_bytes` returned to their readings.
3. **Step charge:** with the `c8e54d7f` filter (`has_property`) planted back in a copy, the seven ordinary shapes spend exactly the same steps (244, 317, 157, 71, 149, 117, 69), in both modes. No ordinary for-in moved.
4. **Chain tests:**
   - Same share per level: 12,544 bytes per level at depth 1000 and at 500. The fixed slack is spread over fewer levels at 500, so each level gets a few tens of bytes more.
   - Planted frame of 2 KiB: 1 of 23 routes red (`the_own_keys_trap_reaches_the_reentry_limit_on_the_promised_stack`).
   - 4 KiB: 11 of 23 red.
   - 16 KiB: 23 of 23 red, all stack overflows. No route is green at 16 KiB.
   - The collect-mode combined test's 21 routes are the same as the 21 `deep_chain!` routes of `c8e54d7f`. The old tests also tried a 10,000 chain with a loose depth; the new ones try the exact depth and depth + 1. At depth 4 the test can see an off-by-one of the bound but not stack size.
5. **Second defect in two collect counts:**
   - Re-entry bound + 1: red in all 3 target tests (`a_throw_caught…`, `nesting_is_back_to_zero…`, `chains_of_every_route…`).
   - `is_array` charging one step short: red in `is_array_and_get_function_realm_charge_a_step_per_proxy`.
6. **Mutants:** m4 (the filter's Proxy parent not kept) is caught by the unit's `fresh_answers_are_held_by_for_in`. For the others, see findings 2 to 4.
7. **CPU times** (each test alone, one thread; all 232 measured):
   - None of the unit's tests is over 1.5 s default or 2 s collect.
   - Slowest unit tests: `fresh_answers_are_held_by_for_in` 1.23 s default, `walks_of_the_chain_through_a_proxy` 1.77 s collect.
   - My own loop case takes 2.09 s default (1.64 s collect). That is over the 1.5 s mark but under 5 s.
   - Totals: 16.8 s default, 49.6 s collect.

## Cases added
All uncommitted, in `crates/swell/tests/proxy_shape_traps.rs` (only lines added):
- `adv2_for_in_visits_the_keys_of_the_prototype_the_trap_answered_first` (red)
- `adv2_for_in_holds_the_visited_keys_a_trap_deletes_from_an_object_before_it`
- `adv2_for_in_holds_the_keys_of_an_ordinary_object_after_the_first_proxy`
- `adv2_memory_budget_at_every_64_bytes_through_for_in_over_three_proxies` (default mode only; helper `memory_sweep`)
- `adv2_memory_budget_at_every_64_bytes_through_json_stringify_of_three_proxies` (default mode only)
- `adv2_for_in_in_a_loop_gives_back_what_each_loop_held`
- `adv2_for_in_ends_at_a_revoked_proxy_and_at_a_cycle_of_trap_answers_and_gives_back_its_bytes`
- `adv2_for_in_over_ordinary_shapes_spends_the_steps_it_spent`

## Commands and result lines
- `cd <worktree> && SLOT_TIMEOUT=30m cargo nextest run --locked -p swell --test main --no-fail-fast -E 'test(/^proxy_shape_traps::/) | test(/^proxy::/)'` → `127 tests run: 126 passed, 1 failed`
- Same with `--features=collect-every-safe-point` → `105 tests run: 104 passed, 1 failed`
- Mutants and plants ran in the scratch clone `adversary-2/mut`.
- CPU times: `slot --timeout 30m --fetch .out/adv2-measure.txt sh -c "$(cat measure.sh)"` → exit 0, 232 tests timed.
- Deleted `<scratch>/wave2/proxytraps/adversary-2/mut/target`.

## Slip
Once, before this part of the session, I edited the test file while the collect-mode job of run a2 was still running on the tree. That breaks the rule against editing the tree during a pool job.

## Not attacked
- Symbol keys, duplicate `ownKeys` answers and shadowed keys: no case of mine; I relied on the unit's cases.
- Rooms below 3,761 bytes: the sweep's entries need that much room to start.
- About 55 of the 105 sweep sites were not read in code.
- test262 was not run, as the brief requires.

```findings
- file: crates/swell/src/ops/objects.rs
  line: 824
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: The for-in filter walks again from the receiver and asks a getPrototypeOf trap again, so it drops keys of the prototype the walk enumerated that were never deleted.
- file: crates/swell/src/ops/objects.rs
  line: 784
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: Removing keep_visited passes every unit test and is caught only by adv2_for_in_holds_the_visited_keys_a_trap_deletes_from_an_object_before_it.
- file: crates/swell/src/ops/objects.rs
  line: 789
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: Holding only Proxy own keys from the first Proxy on passes every unit test and is caught only by adv2_for_in_holds_the_keys_of_an_ordinary_object_after_the_first_proxy.
- file: crates/swell/src/ops/objects.rs
  line: 818
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: Dropping the release after the filter's walk passes every unit test and is caught only by adv2_for_in_in_a_loop_gives_back_what_each_loop_held.
- file: crates/swell/src/ops/objects.rs
  line: 235
  category: mutant
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: Reserving one kept slot short survives all tests because no public reading shows the kept stack's capacity.
- file: crates/swell/src/ops/objects.rs
  line: 756
  category: mutant
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: Skipping end_kept_room after the walk survives all tests because for_in_start releases the values and the uncharged capacity is not visible through a public reading.
- file: crates/swell/src/interp/dispatch.rs
  line: 1692
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: The eager walk plus a filter from the receiver makes five more trap calls than the informative lazy order of the clause for a Proxy with a Proxy prototype.
- file: crates/swell/src/ops/objects.rs
  line: 780
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: A cycle of getPrototypeOf answers in for-in is ended only by the step budget, which the script cannot catch.
- file: crates/swell/src/object/proxy.rs
  line: 574
  category: contract-drift
  severity: note
  verdict: NEEDS-CHANGE
  origin: undecided
  message: proxy_define_own_property_steps keeps about 13 values outside a room where docs/design/engine.md allows at most 10 per running builtin.
```
