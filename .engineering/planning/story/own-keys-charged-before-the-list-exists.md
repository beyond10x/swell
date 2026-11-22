---
format: aep.planning-md/3
id: story:own-keys-charged-before-the-list-exists
kind: story
status: implemented
title: '[[OwnPropertyKeys]] charges its steps and the bytes of its list before it builds the list, and a sort of index keys is charged pass by pass'
summary: The three [[OwnPropertyKeys]] of the engine (String, Array, ordinary) charge the steps they charge today and the list's bytes before the list exists; every holder of a key list credits it when it drops it and gives the kept stack back the room it had; a sort of index keys added out of order charges n steps before each pass of a merge sort.
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T05:10:13Z", actor: "human:operator", revision: 2, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-22T05:10:13Z", actor: "human:operator", revision: 3, executor: "agent:coordinator"}
- {from: "active", to: "implemented", at: "2026-11-22T17:07:51Z", actor: "human:operator", revision: 4, decided_on: {"recorded":{"test_result":1}}, executor: "agent:coordinator"}
---
Waits for: nothing; starts on the base at once. Base: `wave/s3-1` with the gate unit
(`gate-test-steps-within-budget`) and the design text merged. Precondition: the design text of
decision D4 (taken: option A), which the base holds: `engine.md`, "The step budget" ("per pass
of a sort of n property keys, n before the pass", and the paragraph on lists of property keys).
Stories that wait for this one: `story:for-in-key-list-charged-while-the-loop-holds-it`,
`story:frontend-scratch-on-the-memory-ledger` (it reads the high-water mark this story adds).

## Context

Line numbers are of `wave/s3-1` at `c108c2d`.

- String objects: `string_own_property_keys` (`crates/swell/src/object/string.rs:183-202`) takes
  the list's keys (`properties.keys()`, `:186`, a new `Vec` of every own key with the indices
  sorted), charges the sort's comparisons (`:187-189`), then builds a `Vec` holding one `Key`
  per code unit of the String (`:190-195`) and the own keys after them (`:196-199`), and charges
  a step per key only then (`:200`). The `Vec` is `size_of::<Key>()` bytes per code unit and is
  charged to the memory budget nowhere, so a String of n code units, itself charged one or two
  bytes per unit, makes a list of `size_of::<Key>()` × n uncharged bytes before any step check.
- Ordinary objects: `ordinary_own_property_keys` (`crates/swell/src/object/ordinary.rs:387-392`)
  calls `Properties::keys` (`crates/swell/src/object/property.rs:498-513`), which collects the
  indices into a new `Vec<u32>` (`:516-523`), checks whether they are ascending and otherwise
  sorts them with `sort_unstable_by` (`sort_indices`, `:479-489`), then collects every key into
  a new `Vec<Key>`; the steps for the keys and the comparisons are charged after
  (`ordinary.rs:389-390`).
- Arrays: `array_own_property_keys` (`crates/swell/src/object/array.rs:351-382`) does the same
  over the list and the dense elements, the steps after (`:380-381`).
- Holders that charge the list once it exists: `held_own_keys` with `key_list_bytes`
  (`crates/swell/src/ops/objects.rs:189-210`, the integrity levels), `held_keys` with
  `list_bytes` and `charge_list` (`crates/swell/src/builtins/object/enumeration.rs:76-78`,
  `:115-136`, `Object.assign` and `Object.getOwnPropertyDescriptors`) and
  `hold_define_properties_keys` (`crates/swell/src/builtins/object/integrity.rs:240-266`,
  `define_properties_bytes` charged at `:247-251`). Holders that charge nothing:
  `enumerable_own_keys` (`ops/objects.rs:331-354`, JSON), `enumerable_own_properties`
  (`:365-382`, `Object.keys`, `values`, `entries`; its `results` reserved uncharged),
  `get_own_property_keys` (`crates/swell/src/builtins/object/mod.rs:378-389`) and
  EnumerateObjectProperties (`ops/objects.rs:560-580`).
- The kept stack: each holder reserves room on `Agent::kept` (`crates/swell/src/host/agent.rs:89`)
  for the keys it keeps (`ops/objects.rs:202`, `:372`, `enumeration.rs:128`, `integrity.rs:253`;
  `enumerable_own_keys` pushes without a reservation, `ops/objects.rs:349`). The reservation is
  charged nowhere, and `release` (`crates/swell/src/object/builtin_function.rs:168-171`)
  truncates the stack and keeps its capacity: `Object.keys(new String(s))` leaves
  `size_of::<Value>()` bytes of capacity per code unit of `s` behind it. The sort of
  `Array.prototype.sort` already does it right for its own list: `sort_list`, `reserve_kept` and
  `end_sort_list` (`crates/swell/src/builtins/array/mutation.rs:605-648`) charge the growth of
  the kept stack before it is made, and give the stack back the room it had and credit the
  growth when the sort ends.
- Property lookup is not part of this story: a lookup goes through the index map once a list's
  room passes `SMALL` (8, `property.rs:184`), for index, String and Symbol keys alike
  (`KeyedList::position`, `property.rs:248-263`).

## Change

Failing tests first (Acceptance). Then:

1. **Steps first.** Each [[OwnPropertyKeys]] charges, before it allocates anything, the steps it
   charges today, from lengths it holds without a walk: an ordinary object `Properties::len()`
   (`property.rs:291-294`); an Array the length of its dense storage plus `Properties::len()`
   (today: a step per key and per hole, the same total); a String object its length plus
   `Properties::len()` (a String object's list holds no index below its length, since its
   [[DefineOwnProperty]] answers those from the String). Through `Agent::charge_steps`
   (`agent.rs:559-570`), which also reads the interrupt.
2. **Bytes next, before the allocation.** The list's bytes (`size_of::<Key>()` per key) and
   those of the index buffer are charged to the memory budget before they are reserved with
   `try_reserve_exact`; over the budget the run ends in `Terminated(MemoryBudget)` and nothing
   stays charged. One helper computes those bytes for every list of keys (`key_list_bytes`
   moves to `crates/swell/src/object/mod.rs`; `list_bytes` of `enumeration.rs` and
   `define_properties_bytes` use it).
3. **The charge goes with the list.** [[OwnPropertyKeys]] answers the keys with the bytes they
   were charged (one type in `object/mod.rs`); the holder that drops them credits those bytes.
   `held_own_keys`, `held_keys` and `hold_define_properties_keys` take that charge over instead
   of charging a second time. `enumerable_own_keys`, `enumerable_own_properties` (its
   `results`), `get_own_property_keys` (its filtered list) charge what they build before they
   reserve it and credit it when they drop it, on every way out.
4. **The kept stack gives its room back.** The room a holder reserves on `Agent::kept` for its
   keys is charged with the list before it is reserved, and when the holder releases them the
   kept stack shrinks to the capacity it had before and the growth's charge is credited.
   `reserve_kept` and `end_sort_list` move from `builtins/array/mutation.rs` to
   `object/builtin_function.rs` beside `keep` and `release`, over a record of the capacity
   before and the bytes charged (the fields `capacity` and `charged` of `SortList`, which keeps
   them), and the sort and every key-list holder call them: one mechanism for the kept stack.
   The sort's behaviour does not change.
5. **A sort that can be stopped (decision D4).** `KeyedList` records whether its index keys were
   added ascending (one comparison at each addition of an index key; a removal or a compaction
   keeps it). A list added ascending sorts nothing, as today. Otherwise `sort_indices` becomes a
   bottom-up merge sort over a second buffer of n `u32`, charged with the list; before each of
   its passes it charges n steps (at most n comparisons happen in a pass), so the step budget
   and the interrupt are read every n comparisons.
6. **EnumerateObjectProperties** credits each list of own keys it has consumed, before it asks
   for the next object's. The list it answers and its `visited` set stay as they are in this
   story: `story:for-in-key-list-charged-while-the-loop-holds-it` charges both and hands the
   list's charge to the for-in enumerator. After this story alone nothing it charges is left
   uncredited.
7. **A high-water mark for tests.** `crates/swell/src/heap/accounting.rs` gains, under
   `#[cfg(test)]`, the high-water mark of `used` with a reset, and the length of the largest key
   list an [[OwnPropertyKeys]] allocated; per agent, read only by the lib tests.
8. **Stays as it is.** The order of OrdinaryOwnPropertyKeys (`sec-ordinaryownpropertykeys`) and
   of the String and Array keys; the step totals of every [[OwnPropertyKeys]] whose index keys
   were added ascending, which is every list today's `is_sorted` check passes; the lookups of
   `KeyedList`; the step counter of bytecode.

No requirement of `host-interface.md` changes; this story has no line in
`conformance/host/requirements.txt`.

Writes: `crates/swell/src/object/{mod,ordinary,string,array,property,builtin_function}.rs`,
`crates/swell/src/ops/objects.rs`, `crates/swell/src/builtins/object/{mod,enumeration,integrity}.rs`,
`crates/swell/src/builtins/json/{mod,parse}.rs` (the two callers of `enumerable_own_keys`, lines
542 and 950), `crates/swell/src/builtins/mod.rs` (its tests at lines 154 and 273),
`crates/swell/src/builtins/array/search.rs` (the test line 1078 only),
`crates/swell/src/builtins/array/mutation.rs` (the two functions that move, and their calls),
`crates/swell/src/heap/accounting.rs` (the test-only fields of point 7),
`crates/swell/tests/builtins_object_enumeration.rs`. Not written: `crates/swell/src/interp/`,
`crates/swell/src/env/mod.rs`. Running units: no file of `let-const-and-block-scopes` (it writes
`object/ecmascript_function.rs` only under `object/`, and nothing under `builtins/` or `ops/`) or
`template-literals`. Shared later: `ops/objects.rs` and `builtins_object_enumeration.rs` with
story 6, `heap/accounting.rs` with story 3; both wait for this one.

## Acceptance

Unit tests in the lib test binary of `swell`, reading the fields of Change point 7:

- `crates/swell/src/object/string.rs` (a new tests module):
  `own_keys_of_a_long_string_end_at_the_step_budget_before_the_list_exists`: a String object of
  1,000,000 Latin-1 code units, 1,000 steps left. Today `Terminated(StepBudget)` after a list
  of 1,000,000 keys was built (the largest key list reads 1,000,000). After: the same outcome,
  the largest key list reads 0.
  `own_keys_of_a_long_string_are_charged_before_they_are_built`: `memory_bytes` = `used()` + 1
  MiB, `steps_per_entry` 10,000,000 (above the 1,000,000 steps of the keys). Today `Ok` with
  1,000,000 keys and `used()` unchanged. After `Terminated(MemoryBudget)` and `used()` as
  before.
- `crates/swell/src/object/property.rs`:
  `index_keys_added_out_of_order_charge_n_steps_before_each_pass`: n = 65,536 index keys added
  in the order i × 40,503 mod 65,536 (a permutation: 40,503 is odd), `2n + 10` steps left (the
  n steps of the keys, one pass, and less than a second). Today `sort_unstable_by` makes all its
  comparisons before the charge (the counter of `sort_indices` reads more than n) and the charge
  then ends the run. After, at most n comparisons happen before `Terminated(StepBudget)`.
  `keys_in_any_order_come_out_in_the_order_of_the_clause` (a guard, green before and after: 64
  fixed permutations of 1,000 index keys mixed with String and Symbol keys, a generator seeded in
  the test).
- `crates/swell/src/ops/objects.rs`: `every_key_list_is_credited_when_it_drops`: for
  `Object.keys`, `values`, `entries`, `getOwnPropertyNames`, `assign`,
  `getOwnPropertyDescriptors`, `defineProperties`, `freeze`, `isFrozen`, `JSON.stringify`,
  `JSON.parse` with a reviver and `for-in`, over an object of 10,000 keys: during each call the
  high-water mark rises above `heap_bytes()` before the call by at least 10,000 ×
  `size_of::<Key>()`, and after `collect()` `heap_bytes()` reads the same after the second and
  the tenth call. Today the mark rises by less for `keys`, `values`, `entries`,
  `getOwnPropertyNames`, `JSON.stringify`, `JSON.parse` and `for-in`, whose lists are not charged
  (`ops/objects.rs:333`, `:371`, `builtins/object/mod.rs:384`, `ops/objects.rs:569`).
  `the_kept_stack_returns_to_its_capacity_after_a_key_list`: `Object.keys(new String(s))`,
  `s` of 100,000 code units: `kept.capacity()` after the call equals it before. Today it is at
  least 100,000 (`ops/objects.rs:372`).

In `crates/swell/tests/builtins_object_enumeration.rs`:

- `for_in_over_a_long_string_object_is_charged_before_its_keys_exist`: `steps_per_entry`
  10,000,000; the host defines a global `s` of 1,000,000 code units through `with_cx`;
  `memory_bytes` = `heap_bytes()` after that + 1 MiB; the script
  `for (var k in new String(s)) break; 1`. Today `Normal(1)` (the list of the String object's
  keys is not charged). After `Terminated(MemoryBudget)` at the list [[OwnPropertyKeys]] builds,
  and the next script `1` answers `Normal(1)`.
- `the_same_enumeration_twice_spends_the_same_steps` (a guard of determinism): `Object.keys` of
  an object of 5,000 index keys added in the order i × 3,001 mod 5,000, two agents: equal
  `last_entry_steps()` and `heap_bytes()`.

The tests of `crates/swell/tests/property_index.rs`, of `builtins_array_mutation.rs` (the sort
over the moved functions) and the test at `crates/swell/src/object/property.rs:576` pass
unchanged.

Commands, each one job of the build pool: `slot cargo nextest run --locked -p swell --profile
default --lib`, `slot cargo nextest run --locked -p swell --profile default --test <b>` for each
`<b>` of `builtins_object_enumeration`, `builtins_object_integrity`, `builtins_json`,
`builtins_json_parse`, `builtins_array_mutation` and `property_index`, the same with `--features
collect-every-safe-point`, `slot cargo clippy --workspace --all-targets --locked -- -D
warnings`, `slot task conform` and `slot task conform-engine` (the counts of the base), and at
the end, once, `slot task test262 TEST262_OUT=.out/test262-unit`, whose `runs.tsv` is compared
with that of `slot task test262 TEST262_OUT=.out/test262-base` run on the base: `awk -F'\t' '{
print $3, $4, $5 }'` over each, and `cmp` of the two outputs exits 0 (no pass lost in the
directories named here). Never `cargo test -p swell` whole, `slot task test-collect`, `slot task
test` or `slot task check`.

Directories: `test/built-ins/Object/{keys,values,entries,getOwnPropertyNames,getOwnPropertyDescriptors,assign,defineProperties,create,freeze,seal,isFrozen,isSealed}`,
`test/built-ins/JSON`, `test/built-ins/String`, `test/built-ins/Array` and
`test/language/statements/for-in`.

Rules this unit keeps. The failing test comes first: each test of the acceptance is written and
seen red before the change, except one named as a guard. A test is killed at 60 s and the
gate's test steps fail over 120 s; each test runs in a process of its own under cargo-nextest,
so no fixture and no counter is shared through a `static` between tests (a counting build
counts within one test's process). Each new test is timed, and one over 5 s is named in the
report with its time. A job of the build pool asks for one slot. Determinism: the same script
under the same budgets ends the same way at the same count on every run. Nothing is added to
the frame of `Agent::execute`; `STACK_BYTES_PER_REENTRY` and the stack tests are unchanged. No
third-party crate, no unsafe code.

## Cost

Bound: the CPU seconds (user + system) of `slot cargo nextest run --locked -p swell --profile
default --test builtins_object_enumeration -E 'not test(=for_in_over_a_long_string_object_is_charged_before_its_keys_exist)
and not test(=the_same_enumeration_twice_spends_the_same_steps)'` (the tests the base has),
built first with `--no-run` and measured with `/usr/bin/time -f '%U %S'` inside the pool job,
three runs on the base and three on the unit, the median of each, rise by at most 3 %. Why 3 %:
per key list the change adds one charge and one credit, and one of each for the kept stack's
room, against n `Key` writes and n steps; the merge sort runs only for index keys added out of
order, and then charges n steps per pass where today's sort counts a step per comparison. Past
the bound the unit reports the three numbers of each side and stops before merging.

## Report

The unit's report gives the new tests with their times, `size_of::<Key>()`, the Cost numbers,
and this sentence for the changelog: "Listing an object's own keys (`Object.keys`, `for`-`in`,
`JSON.stringify` and the others) now charges its steps and the list's bytes before the list
exists, credits them when the list drops, and gives the kept stack back its room; a sort of
index keys added out of order charges n steps before each pass of a merge sort, so the step
budget and the interrupt are read every n comparisons."
