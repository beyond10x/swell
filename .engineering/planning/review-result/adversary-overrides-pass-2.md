---
format: aep.planning-md/3
id: review-result:adversary-overrides-pass-2
kind: review-result
status: active
title: Adversary pass 2 on the unit overrides (impl/overrides at 08ad2f4)
relations:
- reviews: story:host-object-overrides
revision: 2
---
Adversary pass 2 (the last) on the unit overrides of wave big-2, impl/overrides at 08ad2f4, 2026-11-27.
Red: 5 failing cases, four warnings and five notes. The adversary's own report follows.

unit: overrides, correction `046bfb6` on `impl/overrides` (head `08ad2f4`, plus my uncommitted test cases)
verdict: CONFIRMED (warning): 4 invariant breaks when the record answers for a missing override, plus 1 design question
cases: executed 36→41, red 5
origin: introduced 9 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 directory tree (`<scratch>/wave2/overrides/adversary-2/`, 2.9 GB)
needs-coordinator: F-D: is a `has_property` false a fact that the key does not exist? F-F: can anything reach the keep at `ordinary.rs:506`?

**1. `git --no-pager diff --stat`**
```
 crates/swell/tests/host_overrides.rs | 343 +++++++++++++++++++++++++++++++++++
 1 file changed, 343 insertions(+)
```
Only appended lines: helpers that build a host object whose 11 overrides ask a script handler (the way a Proxy asks its handler), and 5 tests. No other path changed. `.out/` is ignored: it holds the test262 part run and its build logs.

**2. Cases added** (`crates/swell/tests/host_overrides.rs`; all red now; each is red in both builds). Output below is from each case run alone, before the suite.
- `adversary_set_prototype_of_from_the_record_after_a_non_extensible_report_faults` (:3158). Table without `set_prototype_of`. The object says it is not extensible and its prototype is `P1`. Then `Reflect.setPrototypeOf(o, P2)` answers true; the test expects a fault.
  `left: (Normal("\"false,true,true\""), Alive)` / `right: (Terminated(EngineFault), Faulted)`
- `adversary_prevent_extensions_true_from_the_record_makes_the_object_non_extensible` (:3179). Table without `prevent_extensions`. Two sequences, both expected to fault:
  - the record answers PreventExtensions true, then `is_extensible` answers true;
  - absent, then present after that true.

  `left: ((Normal("\"true,true\""), Alive), (Normal("\"true,,1\""), Alive))` / `right: ((Terminated(EngineFault), Faulted), (Terminated(EngineFault), Faulted))`
- `adversary_the_record_key_list_must_hold_an_observed_non_configurable_key` (:3207). Table without `own_property_keys`. `x` is reported non-configurable. The record's key list does not hold `x`; the test expects a fault.
  `left: (Normal("\"false,0\""), Alive)` / `right: (Terminated(EngineFault), Faulted)`
- `adversary_delete_true_from_the_record_on_a_non_extensible_object_makes_the_key_absent` (:3228). The same script runs twice: once with the full table, once with no `delete`. The full table faults; the record's OrdinaryDelete true lets `x` come back.
  `left: ((Terminated(EngineFault), Faulted), (Normal("\"false,true,1\""), Alive))` / `right: ((Terminated(EngineFault), Faulted), (Terminated(EngineFault), Faulted))`
- `adversary_has_property_false_is_not_an_observation_of_a_non_existent_key` (:3247). The object says it is not extensible, then `"x" in o` answers false, then `x` is described as configurable. The test expects the script to run.
  `left: (Terminated(EngineFault), Faulted)` / `right: (Normal("\"false,false,1\""), Alive)`

CPU seconds alone, 3 runs each: default 0.008–0.015, collect 0.006–0.013.

**3. Suite run** (after the cases existed). `cargo nextest run --locked -p swell --test main --no-fail-fast --success-output immediate [--features collect-every-safe-point] -E 'test(/^host_overrides::./)'`
```
default: FAIL adversary_set_prototype_of_from_the_record_after_a_non_extensible_report_faults
         FAIL adversary_has_property_false_is_not_an_observation_of_a_non_existent_key
         FAIL adversary_prevent_extensions_true_from_the_record_makes_the_object_non_extensible
         FAIL adversary_delete_true_from_the_record_on_a_non_extensible_object_makes_the_key_absent
         FAIL adversary_the_record_key_list_must_hold_an_observed_non_configurable_key
     Summary [   0.224s] 41 tests run: 36 passed, 5 failed, 1563 skipped   exit 100
collect: the same 5 FAIL
     Summary [   0.606s] 41 tests run: 36 passed, 5 failed, 1542 skipped   exit 100
```
No compiler warnings. The red suite is the expected result.

Other runs:
- `slot --timeout 30m task test-stack-profiles`, with `host_overrides` and its 3 stack tests added to the lists:
  - release: 7 passed, 0 failed, 0 debug sections;
  - stack-debug: 7 passed, 0 failed;
  - exit 0.
- test262 `built-ins/Reflect` and `built-ins/Proxy` as a part:
  - head: 912 Passed, 1 Failed. Base `runs.tsv`: 912 Passed, 1 Failed.
  - `diff` of path, mode and result: empty.
  - The only failure on both: `Proxy/preventExtensions/trap-is-undefined-target-is-proxy.js` module ("a module run is not made before slice 11").
  - Totals: Proxy 311 (310 passed), Reflect 153 (153 passed).
- Item 6: each of the 36 tests run alone, CPU seconds (user + system), 3 runs per build, on the build node (32 cores, load 4.96 at the start, 1.41 during the runs):
  - default build: highest is `the_facts_end_at_the_memory_budget_at_every_size_and_leave_no_byte` at 0.197–0.199.
  - collect build: highest is `a_key_held_by_a_walk_across_a_get_prototype_of_override_survives_a_collection` at 0.052.
  - No reading is near the unit's 0.603 bound, and none of the 3 runs differ.

**4. Findings** (all in `046bfb6`)
- **F-A, warning, introduced.** `host_overrides.rs:836` (and `known_extensible` :632). A table with `is_extensible` but no `prevent_extensions`: the record's PreventExtensions true is never recorded (`facts.sealed` is not set). A later `is_extensible` true is accepted, and so is a key absent and then present. Reached through `Reflect.preventExtensions` and `Object.preventExtensions`.
- **F-B, warning, introduced.** `host_overrides.rs:753`. A table without `set_prototype_of`: OrdinarySetPrototypeOf on the record answers true for another prototype after the object said it is not extensible and named its prototype. Reached through `Reflect.setPrototypeOf`.
- **F-C, warning, introduced.** `host_overrides.rs:1240`. A table without `own_property_keys`: the record's key list leaves out a key the `get_own_property` override reported non-configurable, and nothing faults. The module's own doc says "every key list holds it". Reached through `Reflect.ownKeys`, `Object.keys` and for-in.
- **F-E, warning, introduced.** `host_overrides.rs:1192`. A table without `delete`: OrdinaryDelete true on a configurable key of a non-extensible object is not recorded as absent. A later present answer runs, while the same answers through the `delete` override fault. These are the same answers, so the two paths contradict each other.
- **F-D, note, introduced.** `host_overrides.rs:1083`. `has_property` false for a key with no recorded fact goes to `answered_absent`, which records the key as absent on a non-extensible object. Clause 6.1.7.3 binds [[HasProperty]] only for a non-configurable key. The Proxy `has` check (10.5.7 step 9.b.ii) is equally strict. Either the doc names this choice or the record is dropped; the coordinator decides which.
- **F-F, note, introduced.** `ordinary.rs:506`. With this keep removed (plant A), all 36 tests and all my probes stay green.
  - Every caller keeps the key, value and receiver before OrdinarySet runs on a target with a Proxy receiver: `proxy_set_steps` (`proxy.rs:962–985`), `reflect.rs`, and each `Cx` method.
  - The interpreter keeps values in registers.
  - So nothing showed that the "older defect" in the commit message can be reached.
- **F-G, note (test gap), introduced.** `host_overrides.rs:980`. Removing the keep of `current`'s value (plant B, collect build) leaves the 36 tests green. The catching test is in the note below.
- **F-H, note (test gap), introduced.** `host_overrides.rs:1356`. Removing the `retain` on a later key list (plant C) leaves the 36 tests green.
- **F-I, note (test gap), introduced.** `host_overrides.rs:572`. Removing the `keys.remove` for a listed key that is later reported absent (plant D) leaves the 36 tests green.

The code is right in F-G to F-I, so the catching tests are given here as text and not added as failing cases. Each was green on head and red with its plant. Shortened versions, using the helpers in my block:
```rust
// plant C
let r = js_run(&JS_EVERY, "base({ isExtensible: function () { return false; }, ownKeys: seq([[\"a\", \"b\"], [\"a\"], [\"a\", \"b\"]]) })",
    "Object.isExtensible(o); Reflect.ownKeys(o); Reflect.ownKeys(o); Reflect.ownKeys(o).length;");
assert_eq!(r, (Outcome::Terminated(Termination::EngineFault), AgentState::Faulted));
// plant D
let r = js_run(&JS_EVERY, "base({ isExtensible: function () { return false; }, ownKeys: function () { return [\"a\"]; }, getOwnPropertyDescriptor: seq([undefined, D(1, true, true, true)]) })",
    "Object.isExtensible(o); Reflect.ownKeys(o); Object.getOwnPropertyDescriptor(o, \"a\"); Object.getOwnPropertyDescriptor(o, \"a\");");
assert_eq!(r, (Outcome::Terminated(Termination::EngineFault), AgentState::Faulted));
```
Plant B is caught by `adversary_the_current_value_held_across_an_is_extensible_override_survives` (collect build) in `.../adversary-2/host_overrides-with-probes.rs:4172–4230`:
- a `get_own_property` override answers `x` with a new object made by a script function `mk` and held by nothing else;
- `is_extensible` runs a hook that collects;
- `Reflect.defineProperty(o, "x", { enumerable: false })` must stay true, and the agent must stay alive after `collect()`.

**5. Attacked and not broken**
- **Held values (item 1).** The hook ran a script that collects before each answer. Its outcome equaled the outcome without the hook in both builds, with no termination:
  - 21 scripts on a host object: `with`; strict and sloppy code; the array builtins on array-likes and through a host prototype; `Object.assign`, `Object.entries`, `Object.defineProperties` and descriptors; JSON stringify and parse with a reviver; spread and rest; for-in; `instanceof` and `Symbol.hasInstance`; `isPrototypeOf`; `super` get and set; bound functions; `class extends` a bound function; `Reflect.set` with a host receiver; a fresh getter an override returns; a Proxy receiver.
  - 14 scripts on a host global object: direct and indirect `eval` with var and function, `new Function`, typeof, sloppy and strict assignment, `delete`, `let` against `eval("var")`.
- **Cost of `hold_for_walk`.** Three scripts with no host object on a 50-object chain, 1,000 iterations each, `9b01ef1` against head:
  - deep read: 72,197 steps, heap 389,722 → 395,649 on both;
  - setter on the chain: 78,204 steps, 389,722 → 396,487 on both;
  - `in` on a missing key: 72,195 steps, 389,722 → 395,510 on both.

  No difference.
- **Invariants on the full 11-override table (item 2).** All 57 rows matched the clause. Forbidden sequences fault and allowed ones run:

| bullet | forbidden faults | allowed runs |
|---|---|---|
| GetPrototypeOf same after not extensible | yes | yes |
| SetPrototypeOf false unless observed prototype | yes | yes |
| IsExtensible false stays false | yes | yes |
| PreventExtensions true, then IsExtensible false | yes | yes |
| GetOwnProperty fully populated | yes | yes |
| non-writable value SameValue (1→2, +0→−0 / NaN→NaN) | yes, yes | yes |
| writable non-configurable value may change | – | yes |
| non-configurable enumerable, getter, setter fixed | yes, yes, yes | getter: yes |
| configurable → non-configurable / then absent | yes (absent) | yes |
| writable false→true / true→false | yes | yes |
| non-existent stays absent (absent before the report binds nothing) | yes | yes |
| deleted then present: non-extensible / extensible | yes | yes |
| DefineOwnProperty: fixed changed / writable changed, all SameValue | yes | yes, yes |
| DefineOwnProperty on non-extensible: non-existent / existence unknown | yes | yes |
| HasProperty non-configurable true / configurable false (also non-extensible) | yes | yes, yes |
| Get fixed value (1→2, +0→−0 / NaN); accessor without getter undefined | yes, yes, yes | yes, yes |
| Set non-writable SameValue only; accessor without setter false | yes, yes | yes, yes |
| Delete non-configurable false / configurable true | yes | yes |
| OwnPropertyKeys: no duplicate / Symbol and String of one description | yes | yes |
| every observed non-configurable key; non-extensible lists only own keys | yes, yes | yes, yes |
| first list after the report (a list before it binds nothing) | yes | yes |
| later list superset / subset; dropped key returns; listed key absent then present | yes, yes, yes | yes |
| method absent, record answers: `set_prototype_of` / `prevent_extensions` / `own_property_keys` / `delete` | **no** (F-B, F-A, F-C, F-E) | yes |
| method absent: `get_prototype_of` / `get_own_property` / `is_extensible` | yes / yes / yes (faults at the PreventExtensions true) | yes |
| method absent: `define_own_property` / `has_property` / `get` / `set` | no forbidden sequence (the record answers from the override's descriptor) | yes ("false,true,false", "true,false", "1,1", "false,false,1") |

- **Absent table (item 3).**
  - Growth measured after a collection: +132 at key 4, +264 at 8, +528 at 15, +1,056 at 29, 0 at every other key from 2 to 30. That matches buckets 4/8/16/32/64.
  - A host object that never says it is not extensible pays nothing: 563,468 = 563,468 bytes after 2,200 absent answers (collect build: 391,436 = 391,436).
  - 1,000,000 distinct keys:
    - strings stop at the memory budget (heap 4,583,627 of a 4,584,071 budget);
    - symbols also stop at the budget (heap 4,129,017);
    - the agent stays alive, with no fault, and the table still holds its keys afterwards.
  - 1,000 objects freed: `heap_bytes` is equal across rounds after one warm-up round. Arena capacity stays charged, so it does not drop back to the first reading.
  - Budget lowered 8 bytes at a time from the peak + 16 down to the peak − 392 (peak 399,931 default, 399,922 collect):
    - the table answers as before down to the peak − 136 (default) or − 128 (collect);
    - below that, every run ends at the memory budget;
    - never a fault.
- **Cycle loop (item 4).** Chains of 100,000 objects:
  - a host object without `get_prototype_of`: the loop goes on, `false`, 100,013 steps;
  - a host object with the override: stops, `true`, 100,012 steps;
  - a Proxy: stops, `true`, 100,012 steps;
  - about 18 ms each.

  A cycle through an ordinary object and an override, walked by `instanceof`, `in`, a read and `isPrototypeOf` on 7/8 of `min_stack_bytes()`: each ends at the step budget with the agent alive.
- **Not attacked:** the website status data, and test262 directories other than Reflect and Proxy.

Probe times, over the bound: the probes were removed from the tree and kept in scratch.
- collect build: the extensible-object probe 25.4 s, the two batteries 3.1 and 3.6 s, the invariant table 3.4 s;
- default build: the 100,000 chain 3.0 s.

**6. Paths written outside the worktree** (all under `<scratch>/wave2/overrides/adversary-2/`)
- **Trees, with their `target/` build output** (most of the 2.9 GB): `base-tree/` (`9b01ef1`), `head-tree/` (`08ad2f4`), `plant/tree/`.
- **Case and probe sources:** `cases-final.rs`, `cases-part1.rs`, `cases-part2.rs`, `cases-part3.rs`, `record-red.rs`, `record-rest.rs`, `record-set.rs`, `green-record-rows.rs`, `adversary_walk_cost.rs`.
- **Whole test-file copies:** `host_overrides-final.rs`, `host_overrides-with-probes.rs`.
- **Scripts:** `run-adv.sh`, `run-adv-out.sh`, `plants.sh`, `walk-cost.sh`, `measure.sh`, `times.sh`, `part.sh`, `compare.sh`.
- **`logs/`** (all `.log` files and the `.exits` and `.offon` files).

Inside the worktree, ignored: `.out/adv2-part/`, `.out/adv2-cases.txt`, `.out/adv2-build-*.txt`.

**7. Findings block**
```findings
- file: crates/swell/src/object/host_overrides.rs
  line: 836
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: without a prevent_extensions override the record's PreventExtensions true is not recorded, so a later is_extensible true and a key absent then present run without a fault
- file: crates/swell/src/object/host_overrides.rs
  line: 753
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: without a set_prototype_of override the record answers true for another prototype after the object reported itself not extensible and its prototype
- file: crates/swell/src/object/host_overrides.rs
  line: 1240
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: without an own_property_keys override the record's key list leaves out a key the get_own_property override reported non-configurable, with no fault
- file: crates/swell/src/object/host_overrides.rs
  line: 1192
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: without a delete override the record's OrdinaryDelete true on a non-extensible object is not recorded as absent, while the same answer from the override is, so a later present answer runs
- file: crates/swell/src/object/host_overrides.rs
  line: 1083
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: has_property false on a non-extensible object is recorded as non-existent, stricter than the HasProperty bullet of 6.1.7.3 and equal to the Proxy has check; the choice is not written down
- file: crates/swell/src/object/ordinary.rs
  line: 506
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: no script or host call was found that reaches OrdinarySet with a Proxy receiver and an unkept key or value, so removing this keep stays green and the older defect named in the commit is unproven
- file: crates/swell/src/object/host_overrides.rs
  line: 980
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: removing the keep of the current value in held_define_steps leaves the 36 tests green; a fresh value from get_own_property across a collecting is_extensible catches it
- file: crates/swell/src/object/host_overrides.rs
  line: 1356
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: removing the retain of the keys a later list leaves out leaves the 36 tests green; a key dropped by a later list and listed again catches it
- file: crates/swell/src/object/host_overrides.rs
  line: 572
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: removing the keys.remove for a listed key reported absent leaves the 36 tests green; a listed key reported absent and then present catches it
```
