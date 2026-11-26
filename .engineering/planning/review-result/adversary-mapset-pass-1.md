---
format: aep.planning-md/3
id: review-result:adversary-mapset-pass-1
kind: review-result
status: active
title: Adversary pass 1 on the unit mapset of wave big-2
relations:
- reviews: story:map-and-set
revision: 1
---
Adversary, pass 1 on the unit mapset of wave big-2 (impl/mapset at 9cae087a), 2026-11-26.
Verdict: no failing case. 2 findings, both introduced warnings (HOST-25 and HOST-214 run 1,000 keys where the requirement states 10,000; the per-entry step charges are held by no test of the unit); 12 cases added, 0 red; task test-realm-sizes green at all 7 sizes in both modes.

unit: mapset (`impl/mapset` at `9cae087a`, plus my uncommitted cases)
verdict: green
cases: executed 1173→1185, red 0
origin: introduced 2, pre-existing 0, undecided 0
wrote-outside-worktree: <scratch>/wave2/mapset/adversary-1/
needs-coordinator: yes

No case is red on `9cae087a`. I found two warnings, neither one a code defect. You need to decide one thing: HOST-25 and HOST-214 run 1,000 keys while the requirement says 10,000.

**1. What I changed**
```
 crates/swell/tests/builtins_map_and_set.rs | 602 +++++++++++++++++++++++++++++
 1 file changed, 602 insertions(+)
```
Only the test file. No implementation file. Nothing is committed.

**2. My cases** (in `crates/swell/tests/builtins_map_and_set.rs`, from line 690)

All 12 are green now in both modes. The slowest takes 4.1 s wall time, in collect mode.
- `adv_random_operations_match_the_model_list_of_the_clauses`: 2,000 random operations (12 in collect mode) with a fixed seed. Each answer is compared with the spec's List model. It covers attacks 1 and 2: every key kind, deletes, re-adds, `clear`, five iterators with some abandoned, `forEach` that changes the Map, compactions, `size` and what `delete` answers.
- `adv_a_compaction_inside_for_each_keeps_five_cursors_on_their_entries`: four iterators at the first hole, in the deleted run, past it and at the end, plus the `forEach` cursor. Run with and without the index map.
- `adv_the_constructors_read_and_call_in_the_order_of_the_clauses`: a log of every observable read in order, in nine cases. These include a non-callable adder, an entry that is not an object, and an adder that throws while `return` or its getter throws too.
- `adv_the_surface_of_map_set_and_their_iterators`: own keys in clause order, every descriptor, the shared functions, the prototype chains, and 11 TypeErrors for each method, getter and `next`.
- Also:
  - `adv_minus_zero_values_and_keys_past_the_index_map`
  - `adv_a_map_of_another_realm`
  - `adv_the_collector_keeps_what_iterators_and_generators_hold`
  - `adv_churn_without_live_entries_stays_bounded`
  - `adv_the_memory_budget_twice_in_one_agent_credits_both_runs`
  - `adv_an_interrupt_between_next_calls_and_in_the_adder`
  - `adv_loops_that_add_an_entry_per_turn_end_at_the_step_budget`
  - `adv_each_entry_of_next_for_each_and_the_constructor_is_a_step`

One case was red on its first run, and that was my mistake. `adv_churn…` failed with "379100 after the churn, 376720 before": I allowed 2,048 bytes and the reading taken without a collection rose 2,380 bytes. That rise is the script's own overhead; after a collection the rise was 128 bytes. I widened only the reading without a collection to 8,192 bytes.

**3. Suite runs** (after my cases existed)
- `--test main`: 1185 passed, 2 skipped (6.6 s).
- `--test main` with `--features collect-every-safe-point`: 1185 passed (33.0 s).
- `--lib`: 250 passed (1.3 s).
- `--lib` with the collect feature: 246 passed (14.6 s).
- `slot --timeout 30m task test-realm-sizes`, exit 0. My 12 cases are included.
  - Builds: `test` profile 20.63 s, `collect` profile 4m 04s.
  - Each of the 14 runs (sizes 4, 11, 17, 38, 64, 128, 200, each in both modes) is green: `default: 1435 passed, 0 failed, 1 ignored` and `collect: 1431 passed, 0 failed, 1 ignored`.
  - Last line: `realm-sizes: 0 run(s) red`.

**Planted defects (attacks 8 and 11)**

I planted each defect in a copy under `.out/adv-mutants-<mode>` in one pool job. Some builds combine defects that target different tests.
- **Hash-dependent order:** iteration follows the index map's order.
  - `host_25` (1,000 / 12 keys) is red in both modes: "the two transcripts differ".
  - `host_214` stays green, as designed (same Seeds).
- **Cursors not released:**
  - Collector pass returns 0 → `abandoned…` steps [16847, 14848]; collect mode [573, 544].
  - `forEach` does not close on a throw → `a_throw…` 264744→297512; collect 264744→266792.
  - `next` keeps its cursor when done → `finished_loops…` [15848, 14847]; collect [554, 543].
  - All red in both modes.
- **Charges not credited** (the table's drop credit and the cursor-release credit): `a_map_filled…memory…`, `a_set_of_10000…`, `abandoned…` (live_bytes) and my memory case are all red in both modes. For example, the Map test reads 544721 after `collect()` against 376673 before.
- Every lowered count still goes red on the defect it exists for.
- HOST-25 at 10,000 keys (default mode only): passes, 4.556 s wall time; HOST-214 passes, 2.414 s.

**4. Findings**
- **F1 (contract drift).** `crates/swell/tests/determinism.rs:827` (`ordered_keys`) uses 1,000 keys (12 in collect mode).
  - `docs/design/host-interface.md:376` (HOST-25) and `:397` (HOST-214) require 10,000. `conformance/host/requirements.txt:25,214` record both as held.
  - At 10,000 keys both tests pass well under the 60 s limit. The unit lowered the count for the story's limit of 1.5 CPU seconds per test, which 10,000 keys likely exceeds; I measured wall time only.
  - What reaches it: the host-requirements check, which reads `requirements.txt`. Introduced by this unit.
  - The story contradicts itself here. You decide whether the requirement text changes or the count goes back up.
- **F2 (mutant).** I removed all four one-step charges per entry:
  - `next`: `crates/swell/src/builtins/map/mod.rs:656`
  - `forEach`: `map/mod.rs:561`
  - Map constructor loop: `map/mod.rs:372`
  - Set constructor loop: `set/mod.rs:194`
  
  The 28 other tests of the module stayed green, the unit's 17 included. Only my pin test went red: `left: [0, 1000, 3000] right: [1000, 2000, 4000]`. test262 has no step budget, so the 376 listed cases cannot see this; I did not run test262.
  - These are the three mutants for attack 11. The test that catches them is `adv_each_entry_of_next_for_each_and_the_constructor_is_a_step` (line 1248). It is green on the code, which has no defect.
  - Its assertion: `assert_eq!([next_live - next_done, for_each_full - for_each_empty, set_full - set_empty], [n, 2 * n, 4 * n])`.
  - What reaches it: the per-entry steps the story promises under "Bounds", which no test of the unit checks. Introduced.

**5. What I attacked and could not break**
- Keys and order: `-0`/`+0`, NaN, Strings built two ways, Symbols, `14` and `"14"`. A delete followed by a re-add moves the key to the end; `clear` then adds.
- Every iteration mode while the collection changes, and compaction with five live cursors.
- The constructors' read order and how they close the iterator; `Reflect.construct` fallback across realms.
- The whole surface and the brand checks.
- The collector:
  - a Map that holds itself;
  - a Map held only through a suspended generator's iterator;
  - iterators dropped unfinished.
- Churn stays bounded. With `+0`-only churn the reading after a collection rises 128 bytes. A Map of 1,000 keys doubles its storage once (460800→544768), and a second churn leaves it unchanged.
- The memory budget twice in one agent: 376417 before, and [376417, 376417] after each run and `collect()`, in both modes.
- Interrupts between two `next` calls and inside the adder (`return` is not called).
- Step-budget ends for loops that add an entry per turn.
- Code rules: I read 1,166 lines (`map/mod.rs` 707, `set/mod.rs` 225, `collection.rs` 234) and the diffs to `arena.rs` and `collect.rs`. I found no `unwrap`, `expect`, unchecked index, overflowing arithmetic or uncharged growth.
- The collector's new pass charges no steps. It is bounded by the object arena's slots and the charged bytes of each cursor list.

What I did not check:
- the test262 part;
- clippy on my cases (they built with `-D warnings`);
- CPU time, since I measured wall time only;
- HOST-25 and HOST-214 at 10,000 keys in collect mode;
- tombstone growth in `KeyedMap` (unit `keyed`, unchanged here);
- Proxy receivers (not on the base);
- I took no worktree lease of my own.

**6. Paths written outside the tracked files**
- Scratch: `<scratch>/wave2/mapset/adversary-1/`
  - `cases-default-1.log`, `cases-collect-1.log`
  - `mutants.sh`, `mutants-default.log`, `mutants-collect.log`
  - `suites.sh`, `suites.out`, `suite--test-main-.log`, `suite--test-main---features-collect-every-safe-point.log`, `suite--lib-.log`, `suite--lib---features-collect-every-safe-point.log`
  - `realm-sizes.log`
- Ignored paths inside the worktree:
  - `.out/adv-mutants-default/` and `.out/adv-mutants-collect/` (39 MB each)
  - `.out/adv-*.log` (8 files)
  - `.out/realm-sizes-*`, written by the task

```findings
- file: crates/swell/tests/determinism.rs
  line: 827
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: HOST-25 and HOST-214 are recorded as held by tests of 1,000 keys (12 in collect mode) while the requirements state 10,000, which pass in 4.6 s and 2.4 s wall time.
- file: crates/swell/src/builtins/map/mod.rs
  line: 656
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: Removing the per-entry step charge of next, forEach and both constructor loops leaves every test of the unit green; only the added pin test fails.
```
