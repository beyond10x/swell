---
format: aep.planning-md/3
id: review-result:adversary-upsert-pass-1
kind: review-result
status: active
title: Adversary pass 1 on the unit upsert
relations:
- reviews: story:map-upsert-and-group-by
revision: 1
---
Adversary pass 1 on the unit upsert of wave big-2, impl/upsert at 5458d2e, 2026-11-27.
Green: no failing case, two warnings and five notes. The adversary's own report follows.

unit: upsert (`story:map-upsert-and-group-by`, pass 1, head `5458d2e`)
verdict: green
cases: executed 11→11, red 0
origin: introduced 5, pre-existing 2, undecided 0
wrote-outside-worktree: `<scratch>/wave2/upsert/adversary-1/` (logs, scripts, `adv-tests.rs`, and the clone `plant/`, whose `target/` (3.2G) I deleted)
needs-coordinator: yes. Two missing tests (F1, F2) need a decision, and some plant jobs ran on the workstation pool (see Commands).

The code matched the clauses everywhere I checked. The findings are gaps in the unit's tests. My 9 cases all pass on the head, so none stays in the tree. The tree is clean: `git status --short` prints nothing. Their text is in `adversary-1/adv-tests.rs`.

**Findings**

- **F1, the key of getOrInsertComputed is not held by any test (warning).** I planted P3: the activation of `getOrInsertComputed` keeps no copy of its arguments (`object/builtin_function.rs:773`). All 11 tests stay green in both modes, and so does my `Reflect.apply` case, because `Reflect.apply` keeps its own list.
  - The case that goes red is `adv_get_or_insert_computed_keeps_a_key_only_its_arguments_hold`, in the collect mode only: `Terminated(EngineFault)`, expected `"7:8"`. It is called through `src.forEach(Map.prototype.getOrInsertComputed, m)`; the callback clears `src` and drops its own parameter.
  - It takes 0.015 s default and 1.000 s collect. The comment at `upsert.rs:131` claims this keep. Origin: `4e74c19`.
- **F2, the memory test never grows a group (warning).** The test's iterable gives every element its own key (`tests/builtins_map_upsert.rs:341`), so no group's Array grows past one element.
  - Plant P4 grows the dense storage of an Array without a charge (`object/array.rs:444`). The unit's memory test stays green in both modes.
  - `adv_one_growing_group_ends_at_the_memory_budget` catches it in the default mode: `Terminated(StepBudget) after 4000000 steps, 117646 elements`. The test takes 0.291 s and runs in the default mode only. Origin: `4e74c19`, kept by `5458d2e`.
- **F3, no test can see a charge made after the growth (note).** Plant P6 reserves the table's storage before `charge_bytes` (`object/keyed_entries.rs:455`). Every test stays green in both modes, mine included, because no reading can tell the order of the charge and the growth.
  - The unit claims "charged before they grow"; that claim holds only by reading the code. Pre-existing: the unit did not change `keyed_entries.rs`.
- **F4, the memory test lost its bound on how far the run gets (note, `5458d2e`).** The first form held `at_the_end > before + room/2`. The second holds `at_the_end > before`, `grouped >= 16` and `after <= before`. Neither form held an exact reading, a bound on bytes per element, or a charge made before growth.
  - Now a run that stops early passes, as long as it grouped 16 elements. That allows up to room/16 bytes per element: about 16 KiB in the default mode, 4 KiB in the collect mode.
  - What it still catches: P5 (a table growth whose charge is never credited, `keyed_entries.rs:472`) turns it red in both modes, `2151063 after collect(), 2134679 before` in the default mode and 418385 / 414289 in the collect mode. P4 and P6 pass it, as F2 and F3 say.
- **F5, a present key whose value is `undefined` is not tested (note, `4e74c19`).** Mutant at `upsert.rs:152`: treat `Some(undefined)` as a missing key. Neither the 11 tests nor the test262 cases catch it; test262 uses values 0, 1 and 2, and `undefined` only as a key. The code is right: my shell probe `goic present undefined` passed.
  - Test text: `var m = new Map([[1, undefined]]), c = 0; [String(m.getOrInsertComputed(1, function () { c++; return 2; })), c, m.size].join()` must give `undefined,0,1`.
- **F6, no test pins the messages of the shared steps (note, `4e74c19`).** Mutant at `from_entries.rs:174/179`: write a fixed `Object.groupBy` instead of `{method}`. Nothing catches it. Removing step 1 (RequireObjectCoercible) is visible only in the message, because GetIterator of null or undefined also throws a TypeError.
  - No message of `Object.groupBy` changed: before and after the unit they read `Object.groupBy called on undefined or null`, `Object.groupBy: the callback is not a function` and `Object.groupBy would count an index above 2^53 - 1`. No test pins any of them.
  - Test text: the messages of `Map.groupBy(null, String)` and `Map.groupBy([], 1)` must be `Map.groupBy called on undefined or null` and `Map.groupBy: the callback is not a function`.
- **F7, the story names the coercion differently from the draft (note, pre-existing).** The story calls it `zero` (lines 33 and 53). The pinned draft (`74c33d4`, 7.3.35) calls it `collection`, and the code follows the draft.

**Plants, which of the 11 tests go red**

| Plant | Default mode | Collect mode |
|---|---|---|
| P1, the result Map's keep removed (`upsert.rs:85`) | `group_by_over_an_endless_iterable_ends_at_the_step_budget` (agent Faulted) | `group_by_closes_the_iterator_when_the_callback_throws`, `..._ends_at_the_memory_budget`, `..._ends_at_the_step_budget`, `map_group_by_groups_by_same_value_zero_in_order`, `map_group_by_keeps_its_groups_across_a_collection` |
| P2, the groups kept in a Rust list that no root reaches (`from_entries.rs`) | the same test | the same 5 tests |
| P3 | none | none |
| P4 | none | none |
| P5 | `..._ends_at_the_memory_budget` | `..._ends_at_the_memory_budget` |
| P6 | none | none |

- P1 to P4 ran in the clone `plant/` on this workstation's pool, before the coordinator's change of the brief. The P5 run there was cancelled (job 71257).
- P3, P5 and P6 ran in the worktree: the file was saved, planted, tested and put back, and `git status --short` was shown after each (`tree-plants.log`).

**Mutant M3, the registration in `map/mod.rs:94-104`.** Swapping the two entries `getOrInsert` and `getOrInsertComputed` escapes the 11 tests and the 202 cases. By reading, `builtins_map_and_set::the_surface_of_map_set_and_their_iterators` and `builtins_symbol_values::the_realm_places_file_equals_the_walk` catch it. Moving `groupBy` after the species getter changes nothing that can be seen, because symbol keys are always listed after string keys.

**Items 1 to 3 and 6: nothing found**
- The shell probe of 62 cases checked every case the brief names under items 1 and 2. It printed 6 DIFF lines: 2 were mistakes in my own expectations, and 4 were deliberate prints of the own keys of other builtins. The engine matched the clauses on every case.
- The step budget, cut one step at a time, across `Map.groupBy` (420 steps) and across `getOrInsertComputed` (543 steps): every cut ended in `Terminated(StepBudget)`, nothing was left half made, and the reading after `collect()` equalled the reading after a completed run.
- The memory budget, 64 bytes at a time over the whole room of each call (279 and 128 runs): the same result.
- An interrupt from the callback ends the call after 1 `next`. Depth through `next` reaches the re-entry limit on the promised stack.
- The index bound at 2^53 - 1 is checked by reading only: the code throws its TypeError and closes the iterator at `index >= MAX_SAFE_LENGTH`, before `next`. The step budget ends any run long before that.
- The realm: own keys of `Map` are `length,name,prototype,groupBy,Symbol(Symbol.species)`, the same pattern as `Promise`. The order of `Map.prototype` follows the clauses. `heap_bytes()` after the realm is 394751 and `live_bytes` 282792, as the unit read. The walk printed `realm places: 347 lines of crates/swell/tests/fixtures/realm_places.txt equal the walk`, and the parts are `[183, 164]`.

**Times (item 7)**, CPU seconds, the higher of two runs of each test alone. All are within the bounds (1.5 s default, 2 s collect).

| Test | Default | Collect |
|---|---|---|
| `a_callback_that_sets_the_key_is_overwritten_not_appended` | 0.020 | 1.249 |
| `get_or_insert_answers_the_present_value_or_inserts` | 0.019 | 0.526 |
| `get_or_insert_computed_calls_its_callback_only_for_a_missing_key` | 0.018 | 0.456 |
| `get_or_insert_computed_reaches_the_reentry_limit_on_the_promised_stack` | 0.046 | 0.456 |
| `group_by_closes_the_iterator_when_the_callback_throws` | 0.018 | 0.460 |
| `group_by_over_an_endless_iterable_ends_at_the_memory_budget` | 0.163 | 1.199 |
| `group_by_over_an_endless_iterable_ends_at_the_step_budget` | 0.121 | 0.670 |
| `map_group_by_groups_by_same_value_zero_in_order` | 0.022 | 0.719 |
| `map_group_by_keeps_its_groups_across_a_collection` | 0.013 | 1.298 (the unit read 1.299) |
| `map_group_by_reaches_the_reentry_limit_on_the_promised_stack` | 0.076 | 0.523 |
| `object_group_by_still_groups_by_property_key` | 0.012 | 0.076 |

My sweep probes take 3.1 to 5.5 s in the collect mode, so they are not fit for the gate.

**test262 (item 8)**
- The list: 100 cases whose features name `upsert` or `array-grouping` under Map, WeakMap and `Object/groupBy`; 40 of them are beyond the unit's part.
- Head: `test262 total 100 0 0 100 99 1 0`, 196 of 198 runs passed. Base: 123 of 198 runs passed.
- `check.sh`: `passed 99 bound 99 B 62 F 38 R 1`, 0 `regressed`, 0 `missing`. It exits 1 only on the 2 `failing` lines of `Map/prototype/getOrInsertComputed/this-not-object-throw.js`: `parse: refused: SyntaxError: BigInt literal is not supported before slice 12`. That case is on `close-later.tsv`.
- `WeakMap.prototype.getOrInsert` and `getOrInsertComputed` work on this head (from `story:weak-map-and-weak-set`), and all 39 WeakMap cases pass.

**Commands**
- `cargo run -p swell-cli -- -e probe1.js`: `bad 6` (the 6 DIFF lines explained under items 1 to 3).
- My cases in the tree: default `8 tests run: 8 passed`, collect `18 tests run: 18 passed`.
- `slot --timeout 30m sh -c time.sh`: exit 0.
- `slot --timeout 30m task test-realm-sizes`: last line `realm-sizes: 0 run(s) red`.
- The test262 part job: exit 0. It wrote `.out/adv-part/` and `.out/adv-list.txt` in the worktree.

**Not attacked:** `Map.groupBy` called from another realm; the `WeakMap` upsert code beyond test262; the stack-profile lane; `--lib`.

```findings
- file: crates/swell/src/builtins/map/upsert.rs
  line: 131
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: With the activation of getOrInsertComputed holding no copy of its arguments all 11 tests stay green in both modes, and only a collect-mode case calling it through Map.prototype.forEach goes red with EngineFault.
- file: crates/swell/tests/builtins_map_upsert.rs
  line: 341
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: The memory test gives every element its own group, so a group Array that grows without a charge passes it in both modes, while one endless group into a single key ends at the step budget after 4000000 steps.
- file: crates/swell/src/object/keyed_entries.rs
  line: 455
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: pre-existing
  message: A table that reserves its storage before the charge passes every test in both modes, so the unit's claim that the table is charged before it grows is held only by reading the code.
- file: crates/swell/tests/builtins_map_upsert.rs
  line: 379
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: Commit 5458d2e replaced the bound of half the room with at least 16 grouped elements, so a run that stops early at the budget, up to room/16 bytes per element, now passes.
- file: crates/swell/src/builtins/map/upsert.rs
  line: 152
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: Treating a present key whose value is undefined as missing is caught neither by the 11 tests nor by the test262 cases, though the code is right.
- file: crates/swell/src/builtins/object/from_entries.rs
  line: 174
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: The messages of the shared GroupBy steps are pinned by no test, so writing Object.groupBy for Map.groupBy, or dropping RequireObjectCoercible, passes.
- file: .engineering/planning/story/map-upsert-and-group-by.md
  line: 33
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: The story names the key coercion zero where the pinned draft and the code name it collection.
```
