---
format: aep.planning-md/3
id: story:map-upsert-and-group-by
kind: story
status: draft
title: Map.prototype.getOrInsert, getOrInsertComputed and Map.groupBy
summary: builtins/map/upsert.rs and GroupBy with the zero coercion shared with Object.groupBy; order 4; 46 cases.
relations:
- decomposes: epic:slice-09-collections-proxy-reflect
- depends_on: story:map-and-set
- depends_on: story:weak-map-and-weak-set
scope:
- confidence: inferred
  path: crates/swell/src/builtins/map/mod.rs
- confidence: inferred
  path: crates/swell/src/builtins/map/upsert.rs
- confidence: cited
  path: crates/swell/src/builtins/object/from_entries.rs
- confidence: cited
  path: crates/swell/src/object/builtin_function.rs
- confidence: inferred
  path: crates/swell/tests/builtins_map_upsert.rs
- confidence: cited
  path: crates/swell/tests/builtins_symbol_values.rs
- confidence: cited
  path: crates/swell/tests/main.rs
revision: 3
---
## Context

Slice 9 brings `Map`, `Set`, `WeakMap`, `WeakSet`, `Proxy` and `Reflect` (`epic:slice-09-collections-proxy-reflect`; `docs/design/engine.md`, section 12, item 9), with the feature `upsert` (the epic's Outcome). This story brings `Map.prototype.getOrInsert` and `Map.prototype.getOrInsertComputed` (`#sec-map.prototype.getorinsert`, `#sec-map.prototype.getorinsertcomputed`) and `Map.groupBy` (`#sec-map.groupby`, GroupBy with the key coercion `zero`). `WeakMap.prototype.getOrInsert*` are `story:weak-map-and-weak-set`'s. It is in order 4 and depends on `story:map-and-set` (the `Map` kind, its table, `%Map%` and `%Map.prototype%`) and on `story:weak-map-and-weak-set` (2 cases of its list construct a `WeakMap`; the key order of `docs/plan/slice-09/scan.awk`).

`Object.groupBy` exists (`crates/swell/src/builtins/object/from_entries.rs`: `object_group_by` line 120, `group_by_property` line 151, `group_value` line 194, `add_value_to_keyed_group` line 216), with GroupBy written for the key coercion `property` only. This story makes GroupBy take the coercion (`property` or `zero`), so `Map.groupBy` and `Object.groupBy` share the loop over the iterable, its steps, its closing of the iterator and its charges, and `Object.groupBy` keeps its behaviour (its cases in the guard).

Measured by the decomposition on `62c7cf59` with the full run on the code of wave big-1 before the close of slice 4 (2026-11-26; its line `test262-slice 09 1785 3541 0 0 1785 267 1518 0` equals `conformance/test262/expected.txt` at `62c7cf59`): the list of this story, `docs/plan/slice-09/story-map-upsert.cases.txt`, holds the 46 cases of the key `map-upsert` of `docs/plan/slice-09/scan.tsv` (the command is in the head of `docs/plan/slice-09/scan.awk`), all failing (91 runs: 75 on `Map is not defined`, 2 on `WeakMap is not defined`, the rest on a `Test262Error` that expects a `TypeError`): `built-ins/Map/prototype` 32 (`getOrInsert` and `getOrInsertComputed`) and `built-ins/Map/groupBy` 14. The cases of these directories that use a BigInt (slice 12) are on `docs/plan/slice-09/close-later.tsv`. A case of the list that ends on a part outside this story is reported with that part, and the coordinator takes it off the list by a commit of the list file on the wave branch whose body names that part; the acceptance reads the list as committed there when the wave closes.

Design it builds on, by name: `docs/design/engine.md` decision 3 ("The step budget", "Depth bounds") and decision 4 ("Memory budget"). No sentence of the design is missing.

## What later stories reuse

GroupBy with both key coercions, which nothing later in the slice calls.

## Domain relations

- A call of `Map.groupBy` → its groups: one to many, one group per key by SameValueZero, each group an Array of the values in order — inferable (inferred from ECMA-262 `#sec-groupby` and `#sec-map.groupby`; no ess/1 document declares it: the host model `spec/domains/host.yaml` holds no engine value, and ESS does not restate the slots of ECMA-262, `AGENTS.md`, "Rules").

## Change

Failing tests first: the tests below are written before the code and fail on the base of this story (`Map.groupBy` and `Map.prototype.getOrInsert` are `undefined`).

- `crates/swell/src/builtins/map/upsert.rs` (new; its `mod` line in `crates/swell/src/builtins/map/mod.rs`): `getOrInsert(key, value)` (the brand check, CanonicalizeKeyedCollectionKey, the entry's value if present, else the entry appended), `getOrInsertComputed(key, callbackfn)` (IsCallable first, the key canonicalized, the entry if present, else the callback called with the key, then the table searched again, since the callback may have set the key: an entry found then is overwritten in place, not appended twice), `Map.groupBy(items, callback)` (GroupBy with `zero`, then a new `Map` of `%Map.prototype%` with one entry per group, each value CreateArrayFromList of the group in order). `length` and `name` of each.
- `crates/swell/src/builtins/object/from_entries.rs`: GroupBy takes the coercion; `property` keeps the path of `group_by_property` (ToPropertyKey, groups keyed by property key), `zero` coerces `-0` to `+0` and groups by SameValueZero in an `EntryTable` (`object/keyed_entries.rs` of `story:keyed-entry-table`); the loop over the iterable, its step per element, its IteratorClose on an abrupt callback and its charges stay one loop. `Object.groupBy` (line 120) is unchanged in behaviour.
- Wiring: the three `BuiltinId` variants and arms (`crates/swell/src/object/builtin_function.rs`); the two prototype methods in the list of `%Map.prototype%` and `groupBy` on `%Map%` (`crates/swell/src/builtins/map/mod.rs`).
- Bounds. Steps: each method is a builtin call (its step); GroupBy one step per element before the callback; the table's own steps. Memory: each group and the table charged before they grow; an endless iterable ends at the memory budget or the step budget. Depth: the callbacks and the iterator's `next` re-enter, counted against `reentry_depth` and checked against `frame_depth`, a refusal the `RangeError` of "Depth bounds" (GroupBy closes the iterator after it); `STACK_BYTES_PER_REENTRY` (`crates/swell/src/host/limits.rs` line 46) does not change and the stack tests pass on seven eighths of `min_stack_bytes()`, or the unit gives the measured change and the constant follows it. Run loop: no instruction is added. Termination: a termination in a callback ends the call as section 10 of `docs/design/host-interface.md` states.
- Tests, each in both collection modes, in `crates/swell/tests/builtins_map_upsert.rs` (new): `get_or_insert_answers_the_present_value_or_inserts`; `get_or_insert_computed_calls_its_callback_only_for_a_missing_key`; `a_callback_that_sets_the_key_is_overwritten_not_appended` (the size after the call and the order of `keys()`); `map_group_by_groups_by_same_value_zero_in_order` (`-0` and `+0` one group, `NaN` one group, objects by identity); `object_group_by_still_groups_by_property_key` (`Object.groupBy` with `-0` and `"0"` one group, as before); `group_by_closes_the_iterator_when_the_callback_throws`; `group_by_over_an_endless_iterable_ends_at_the_step_budget` (`Terminated(StepBudget)`, `last_entry_steps()` at most N); `group_by_over_an_endless_iterable_ends_at_the_memory_budget` (`Terminated(MemoryBudget)`; after it, once the host released the script's values, one `collect()` brings `heap_bytes()` back within the realm's reading before the script; the test prints the readings, which the report quotes); `get_or_insert_computed_reaches_the_reentry_limit_on_the_promised_stack` (`reach_through("new Map().getOrInsertComputed(0, f)", BUILTINS_REENTRY)` with `crates/swell/tests/support/stack.rs` as it is); `map_group_by_reaches_the_reentry_limit_on_the_promised_stack` (`reach_through("Map.groupBy([0], f)", BUILTINS_REENTRY)`). `builtins_object_from_entries::` runs and passes as it is. The sweep of every function with a Symbol (`crates/swell/tests/builtins_symbol_values.rs`) reaches the three functions: the unit runs it and adds a row only where a clause throws before a Symbol is read. `crates/swell/tests/main.rs` generated again (`sh taskfiles/test-main.sh write`).
- Time ("Tests stay fast", the `AGENTS.md` of the folder that holds the repository, and `AGENTS.md`, "Commands and the gate"): a test is terminated at 60 s and counts as failed, and the gate fails when the wall time of its test steps passes 120 s. A test stays under 1.5 CPU seconds alone in the default mode, under 2 with a collection at every safe point and under 2.5 at its full count (`SWELL_FULL_COUNTS=1`), measured with the test binary run directly with `--exact <name> --test-threads 1` under `time` of the shell (user plus system); the unit times each test before it adds it and names each one over 1 CPU second with its time; no loop of a test runs more than 10,000 iterations. A product several tests share is built once inside one test function or is a committed file, never in a `static`; no test makes a network request or a model call; the unit runs the test binaries of its own change, never the whole gate and never a whole-workspace test run. Builds and tests go through the pool: `cargo` is the shim of the pool, anything else that compiles or tests is prefixed `slot`; a pool job ends after 10 minutes unless it names its time (`slot --timeout 30m` or `SLOT_TIMEOUT=30m`, the most); a script that measures, probes or waits runs without the prefix. No `unsafe` (the crate forbids it), no third-party crate, no panic, `unwrap`, `expect` or unchecked indexing; no Python in any tool, test or command; intermediate files in the scratch directory.
- Size: about 600 lines (the methods 160, GroupBy 90, the tests 350): one session.

## Requirements

None.

Writes: `crates/swell/src/builtins/map/upsert.rs` (new), `crates/swell/src/builtins/map/mod.rs` (its `mod` line, two prototype entries and `groupBy` on the constructor), `crates/swell/src/builtins/object/from_entries.rs` (GroupBy with the coercion, lines 120 to 240 on `62c7cf59`), `crates/swell/src/object/builtin_function.rs` (three `BuiltinId` variants and arms), `crates/swell/tests/builtins_map_upsert.rs` (new), `crates/swell/tests/builtins_symbol_values.rs` (rows only where Change says), `crates/swell/tests/main.rs` (generated). Order 4, built at the same time as `story:host-object-overrides`, which writes `crates/swell/tests/main.rs` (generated; the coordinator generates it again on the join) and test files of its own, and none of the files above but `tests/main.rs`. Shared in wave big-2 with the stories of slice 8 (`epic:slice-08-classes`), which no edge orders against this one: `object/builtin_function.rs` and `tests/main.rs`, each story its own lines. Not written: `crates/swell/src/object/collection.rs`, `crates/swell/src/object/keyed_entries.rs`, `crates/swell/src/builtins/weak_map.rs` (used as they are), `crates/swell/src/realm/` (no intrinsic is added), `crates/swell/src/host/limits.rs` (unless the measured stack moves, Change), `crates/test262`, `docs/design/`, `conformance/`, `website/`. A unit that meets a file outside this line that its change cannot compile or pass without makes the smallest change there and reports it.

## Acceptance

The tests named in Change pass in both collection modes, run by the binaries of this change and not as a whole step: `slot cargo nextest run --locked -p swell --test main -E 'test(/^builtins_map_upsert::/) | test(/^builtins_map_and_set::/) | test(/^builtins_object_from_entries::/) | test(/^builtins_symbol_values::/) | test(/^eval_stack::/) | test(/^limits_scripts::/) | test(/^budgets::/) | test(/^termination::/) | test(/^interrupt::/) | test(/^collect::/)'` and the same with `--features collect-every-safe-point`; `slot cargo nextest run --locked -p swell --lib` and the same with `--features collect-every-safe-point` (`object::keyed_entries`, `object::builtin_function::tests::a_loop_of_builtin_calls_collects_within_its_bound` and `heap::collect` among them); each exits 0. `slot task clippy` exits 0; `slot task forbid-unsafe-check` exits 0; `git diff --stat "$BASE" -- Cargo.lock "*/Cargo.toml"` prints nothing (`BASE` the join the unit starts from); `sh taskfiles/test-main.sh check` exits 0; `git diff --quiet "$BASE" -- crates/swell/src/object/collection.rs crates/swell/src/object/keyed_entries.rs crates/swell/src/realm` exits 0. Time: every test this story adds stays under 1.5 CPU seconds alone in the default mode and under 2 with `collect-every-safe-point`, as the unit measured them (Change, "Time"); the report lists each measured time and the sum, and a time over its bound fails this acceptance.

test262. The full run is not the unit's: the unit runs the runner on a part of the checkout. With the list as committed on the wave branch when the wave closes, `git show "$WAVE:docs/plan/slice-09/story-map-upsert.cases.txt" > .out/story-map-upsert.cases.txt` (`WAVE` the wave branch the unit merges into), and the guard `docs/plan/slice-09/guard-collections.cases.txt` (156 cases, `Object.groupBy` and `Object.fromEntries` among them; its command is in the head of `docs/plan/slice-09/scan.awk`), `cat .out/story-map-upsert.cases.txt docs/plan/slice-09/guard-collections.cases.txt | LC_ALL=C sort -u > .out/part.cases.txt` and `sh docs/plan/slice-09/part.sh "$CHECKOUT" .out/test262-part .out/part.cases.txt` (`CHECKOUT` `.cache/test262/<commit of conformance/test262/PIN>`; 202 cases, 389 runs over the base run), then, in the background, `slot --timeout 30m cargo run --locked --release -q -p swell-test262 -- count --test262 .out/test262-part --design docs/design/engine.md --features conformance/test262/features.txt --runs .out/test262-part/runs.tsv > .out/test262-part/lines.txt`, and the base run the coordinator made on the join the unit starts from copied to `.out/test262-base/runs.tsv`: `sh docs/plan/slice-09/check.sh .out/story-map-upsert.cases.txt .out/test262-base/runs.tsv .out/test262-part/runs.tsv` exits 0 (over the base run named in Context, restricted to the same cases, it exits 1, prints 91 lines `failing` and `passed 156 bound 156 B 156 F 46 R 46`): every run of the list passes, no run that passed on the base fails, and the part has at least B + F − R passing cases. On the join, the coordinator's full run, made in parts, passes the same check over the whole run.

## Out of scope

`WeakMap.prototype.getOrInsert*` (`story:weak-map-and-weak-set`); the rest of `Map` (`story:map-and-set`); the cases of these methods that use a BigInt (`docs/plan/slice-09/close-later.tsv`, the conformance close).
