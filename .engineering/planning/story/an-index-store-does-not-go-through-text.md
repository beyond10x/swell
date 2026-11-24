---
format: aep.planning-md/3
id: story:an-index-store-does-not-go-through-text
kind: story
status: implemented
title: A store at an array index does not go through text
summary: A computed store at a Number index made a heap String of the index and parsed it back, the Array's [[Set]] looked its index up on the Array, its prototypes and the Array again and `length` up by a key built from text, every fixed name built a `Vec<u16>` per use, and every safe point took the release queue of the root table; a Number that is an array index is now its key, a fixed name is built on the stack, a safe point reads one flag, and a store into an Array that is its own receiver, at an element it holds, at a hole or at its end, writes in place with the steps OrdinarySet charges.
relations:
- decomposes: epic:slice-05-regexp
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T19:04:02Z", actor: "human:operator", revision: 2, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-23T19:04:03Z", actor: "human:operator", revision: 3, executor: "agent:coordinator"}
- {from: "active", to: "implemented", at: "2026-11-24T08:04:06Z", actor: "human:operator", revision: 4, decided_on: {"recorded":{"test_result":2}}, executor: "agent:coordinator"}
---

## Context

A step stands for a bounded amount of work, and storage is charged before it is allocated
(`docs/design/engine.md`, decisions 3 and 4, "The step budget" and "Memory budget"). The loop
`for (i = 0; i < n; i++) a[i] = i` (the probe p04 of the work report) cost 2.313 CPU seconds on
`3d5afeb` against 0.709 for the same loop without the store (p03), with 32,305,846 steps.

Rule: a store at an index costs no allocation that its steps and its bytes do not count, and
no work a step does not bound; the engine's fast paths change no step total and no observable
of `#sec-ordinaryset` and `#sec-array-exotic-objects-defineownproperty-p-desc`.

Four places broke the first half of it on `3d5afeb`:

1. `to_property_key` (`crates/swell/src/ops/conversion.rs`) turned a Number into its text with
   `number_to_string`, made a heap String of it and parsed it back with `key_from_string`: one
   String per computed store, read and delete at a Number (64 Strings for a loop of 64 stores).
2. `name_key` built a `Vec<u16>` of the name per call; 103 call sites of the engine pass a fixed
   name, `"length"` on every Array store among them.
3. `RootTable::drain`, at every safe point, took the release queue out of its `Cell` and put it
   back, also when no root was dropped (2,815 takes for a loop of 200 iterations).
4. OrdinarySet on an Array looked the index up on the Array, walked its prototypes, and then
   [[DefineOwnProperty]] looked the index up again and `length` up by `name_key("length")`.

`key_from_string` charges nothing for a key of up to 32 code units (`charge_code_units`
charges max(0, length − 32)); an index has at most 10, so item 1 changes no step.

## Change

1. ToPropertyKey of a Number that is a non-negative integer below 2^32 − 1 is `Key::Index` of
   it, −0 the index 0; every other Number goes through its text as before. The one function
   serves every caller: computed member read, store and delete, `in`, computed keys of object
   literals, `Object.defineProperty`, `Object.getOwnPropertyDescriptor`, `hasOwnProperty`,
   `propertyIsEnumerable`, `Object.hasOwn`, `Object.fromEntries`, the groupBy builtins, the
   builtins' `index_key` above 2^32 − 2 and the host's `Cx::to_property_key`.
2. `name_key` encodes a name of up to 64 code units into a buffer on the stack and builds a
   `Vec<u16>` only for a longer one. No call site changed; no key is held by the agent, so no
   byte reading moves.
3. The release queue is a struct with a flag a drop sets; `drain` reads the flag and takes the
   queue only when it is set. A dropped root is released at the same safe point or collection
   as before.
4. `array_set_index` (`crates/swell/src/object/array.rs`), called by `ordinary_set` when the key
   is an index and the receiver is the Array itself: an element the dense part holds is
   overwritten in place (its own data property, writable, found with no step, as before); a
   store at a hole or at the end of the dense part is made in place when the Array is
   extensible, its property list never held an index, its `length` (the first entry of the
   list) is writable, and every prototype is an ordinary object, a function, a host object, a
   RegExp or an Array with no elements, whose list never held an index. It charges the step per
   prototype hop that OrdinarySet charges, then the growth of the elements before it is made,
   and raises `length` in place. Every other case goes the general way.
5. Counters of the test build: `STRINGS_MADE` (Strings stored in the heap) and `QUEUE_TAKEN`
   (drains that took the queue), `#[cfg(test)]` thread-locals.

Measured (release build, CPU seconds per run, mean of 3 rounds back to back at a load of 70 to
80; step totals identical in every variant):

| variant | p01 | p03 | p04 | p09 | store (p04 − p03) |
|---|---|---|---|---|---|
| `3d5afeb` | 0.340 | 0.709 | 2.313 | 2.530 | 1.604 |
| item 1 | 0.352 | 0.732 | 1.427 | 1.661 | 0.696 |
| items 1-2 | 0.349 | 0.712 | 1.344 | 1.562 | 0.632 |
| items 1-3 | 0.296 | 0.627 | 1.201 | 1.397 | 0.575 |
| items 1-4 | 0.310 | 0.599 | 0.830 | 1.026 | 0.230 |

p04 runs in 35.9 % of its base time. The 20 cases of `built-ins/RegExp/property-escapes/generated`
(every 22nd): `test262-time 05 40 343735` on the base, `40 62986` after (18.3 %), the
`test262-steps` lines identical.

Writes: crates/swell/src/ops/conversion.rs, crates/swell/src/heap/mod.rs,
crates/swell/src/heap/roots.rs, crates/swell/src/value.rs, crates/swell/src/object/property.rs,
crates/swell/src/object/array.rs, crates/swell/src/object/ordinary.rs,
crates/swell/tests/index_stores.rs, crates/swell/tests/main.rs

## Acceptance

- Tests red on `3d5afeb`, green after:
  - `ops::conversion::tests::a_number_that_is_an_array_index_is_its_key_without_a_string`: 0,
    −0, 7 and 4294967294 give an index key with no String made (1 on the base); 4294967295,
    2^32, −1, 1.5, −0.5, 1e21, NaN and ±Infinity give the key of their text;
  - `ops::conversion::tests::a_store_loop_makes_no_string_per_store`: loops of 8 and 32 stores
    make 0 Strings (loops of 64 and 256 made 64 and 256 on the base);
  - `heap::roots::tests::a_safe_point_with_nothing_dropped_takes_no_queue`: a loop of 20
    iterations takes the queue 0 times, once after two roots are dropped (200 iterations took
    it 2,815 times on the base).
- Tests green on `3d5afeb` and after, each naming the defect that turns it red
  (`crates/swell/tests/index_stores.rs`): a setter on a prototype for an index the Array lacks;
  a read-only index on a prototype; frozen, sealed and non-extensible Arrays and a read-only
  `length`; a String object and an arguments object on the chain; holes and the end; the steps
  of seven stores, as recorded on the base: 8 at an element held, 10 at the end, 10 at a
  hole, 11 at the end below three prototypes, 8 with none, 436 for 20 stores at the end in a
  loop and 460 for the same after a truncation.
- `ops::conversion::tests::name_key_answers_alike_on_both_sides_of_its_buffer` and
  `object::array::tests::a_set_at_the_end_charges_the_growth_before_it_is_made`.
- `cargo nextest run --locked -p swell --test main`: 711 tests pass (705 before), the same
  with `--features collect-every-safe-point`; `--lib`: 176 (171 before), with the feature 174
  (169 before); `-p swell-test262 --test main`: 50.
- Each new test under 0.5 CPU seconds, its binary run directly: at most 0.020 without the
  feature and 0.219 with it.
- `HOST-187` reads as on the base: `BINDINGS_REALM_HEAP_BYTES 449821` (ceiling 506,112),
  `heap_bytes()` after the scripts 656009 (647429 with the feature).
- test262: the subset of `built-ins/{Array,Object,Reflect,String}`, the assignment,
  compound-assignment and object expressions, `for` and computed property names (10,413
  cases, 20,414 runs): every run has the status it had on the base (17,121 passed, 3,293
  failed). The full run (`task test262`): 287 lines equal the expected file, every one of
  85,556 runs has the status it had on the base (35,974 passed, 49,578 failed, 4 ended by a
  budget), 615 s in the pool (1,599 s on the base), `test262-time 05 5441 1303148`
  (3,988,526 ms on the base, 32.7 %).
- `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo fmt --all --check`,
  `sh taskfiles/test-main.sh check`, `host-surface-check` and `host-requirements-check` exit 0.
- Totals that moved: none.

## Out of scope

- `number_to_string` of `swell-num`, which this loop no longer calls.
- String concatenation (`concatenate` copies twice), `String.fromCodePoint`, ropes.
- A store into an Array that is not its own receiver, or whose list or a prototype's list has
  held an index, or below a prototype of another kind: the general path, unchanged.
- Read of an index (`get_value_computed`) beyond item 1: it still walks the chain as before.
- A new kind of object (a generator): `array_set_index` matches every kind of `ObjectKind`, so
  a new kind fails to compile until it is placed on one side.
