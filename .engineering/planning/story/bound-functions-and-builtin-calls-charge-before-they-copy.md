---
format: aep.planning-md/3
id: story:bound-functions-and-builtin-calls-charge-before-they-copy
kind: story
status: implemented
title: A bound function's arguments, the argument list of a bound call and the arguments a builtin call records are charged before they are copied
summary: Three copies whose size a script sets are charged to the memory ledger before they are made and credited at one point where their holder drops them; four documentation lines are corrected. No step total changes.
relations:
- decomposes: epic:slice-06-bindings-and-iteration
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T13:48:05Z", actor: "human:operator", revision: 2, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-22T13:48:05Z", actor: "human:operator", revision: 3, executor: "agent:coordinator"}
- {from: "active", to: "implemented", at: "2026-11-22T17:07:49Z", actor: "human:operator", revision: 4, decided_on: {"recorded":{"test_result":1}}, executor: "agent:coordinator"}
---
Waits for: nothing. It starts beside `story:default-and-rest-parameters`, which writes
`crates/swell/src/object/ecmascript_function.rs`, `crates/swell/src/interp/frame.rs` and
`crates/swell/src/dynamic_code.rs` (its Writes line,
`.engineering/planning/story/default-and-rest-parameters.md:92`); this story writes none of them.
`story:a-call-charges-what-it-keeps-of-its-arguments-and-parameters` waits for this one (it uses
`Heap::new_object_charged` and the test-only hooks named below). No requirement of
`host-interface.md`. Consumer: none named. A gap of the rule of `docs/design/engine.md`,
"Memory budget": storage is charged before it is allocated, and the code that holds a charged
structure credits it when it drops it, on every way out. The commit `a34ea7c` named these sites
in its last paragraph.

## Context

Line numbers are of `wave/budget-1` at `a6815ca`; they are read again on the unit's base.

The accounting calls: `Accounting::charge` (`crates/swell/src/heap/accounting.rs:75-87`)
charges or refuses and charges nothing, and adds what it charges to `since_collection` (`:81`);
`Accounting::check` (`:116-121`) charges nothing; `Accounting::credit` (`:124-126`) does not
lower `since_collection`. `heap::store` (`crates/swell/src/heap/mod.rs:277-291`) charges a
record's owned bytes and the arena's growth before it stores; `heap::store_charged` (`:299-313`)
charges only the growth, for owned bytes charged already; `Heap::new_object` (`:217-224`) calls
`store`. A safe point collects when `since_collection` reaches `trigger_bytes`
(`heap/mod.rs:205-207`, `host/agent.rs:847-857`), which is the live reading after the last
collection and at least `MIN_TRIGGER_BYTES`, 1 MiB (`heap/mod.rs:61`, `:211-214`).

1. **`bound_function_create`** (`crates/swell/src/object/bound_function.rs:76-105`, called by
   `function_bind`, `crates/swell/src/builtins/function.rs:359`) copies the bound arguments into
   `kept` (`:87-91`), as many as `bind` got past its first (any number through
   `Function.prototype.bind.apply`). The copy is neither checked nor charged before it is
   reserved and filled; its bytes (`owned`, `:86`) are charged after it, by `new_object`
   (`:101-104`), which ends the run in `Terminated(MemoryBudget)` only once the copy is made.
2. **`walk`'s `gathered`** (`bound_function.rs:186-235`, called by `call`, `:239-247`, and
   `construct`, `:253-264`) is the argument list of the innermost call: the bound arguments of
   every link and the call's own (`count`, `:192-206`). Since `a34ea7c` its bytes are asked with
   `check` and it is reserved (`:209-213`); it is never charged. It lives until `internal_call`
   (`:246`) or `internal_construct` (`:263`) returns, so the target runs, and allocates, against
   a ledger that leaves the list out. `construct` has one more way out after `walk`: the fault
   on a missing new target (`:260-262`).
3. **The arguments `builtin_call` records** (`crates/swell/src/object/builtin_function.rs:338-379`):
   after `enter_function` (`:355`, defined at `:326-334`) it pushes an `Activation` with
   `args: args.to_vec()` (`:356-362`) and pops it at `:372`; no way out lies between the push and
   the pop. The copy has as many values as the call has arguments (`f.apply`, `Reflect.apply`,
   spread). It is neither checked nor charged, and it lives while the builtin runs.

Follow-ups, not in this story:

- `string_units` and `string_units_in` (`crates/swell/src/object/string.rs:48-53`, `:65-84`)
  copy a String's code units (bounded by the host's `string_length`) with no memory charge or
  check; `charged_units` (`:57-61`) charges their steps. The copy is held by its caller: 33
  calls in 17 files, all but one outside `crates/swell/src/object/`, which hold the credit
  points. A story of its own.
- `call_instruction` and `new_instruction` copy the call's argument list
  (`crates/swell/src/interp/dispatch.rs:1336`, `:1415`) uncharged. Every slice-6 story writes
  `dispatch.rs`, and `story:array-destructuring-and-spread` adds spread calls, which make these
  copies as long as a script likes: a story after slice 6.

## Change

Failing tests first (Acceptance). Then:

1. **`Heap::new_object_charged`** (`crates/swell/src/heap/mod.rs`, beside `new_object`, `:217`):
   `new_object` over `store_charged`, for a record whose owned bytes the caller charged; on a
   refusal the caller credits them.
2. **`bound_function_create`.** It charges `owned` (`:86`) before `kept` is reserved, credits
   it when the reservation fails, and stores the record with `new_object_charged`, crediting
   `owned` on a refusal. The sweep that frees the record credits it, as today.
3. **The gathered list, one credit point.** The form of `truncate_elements`
   (`crates/swell/src/object/array.rs:493-510`): `call` and `construct` go through one function
   that runs the first walk (the steps and the count, `:192-206`, as today), charges the list's
   bytes (`:209`) instead of asking `check`, calls one helper that holds the rest of the work
   (the reservation, the copies, the fault checks of `:216-218` and `:231-233`, and the call:
   `internal_call`, or for a construction the check of the new target, `:260-262`, and
   `internal_construct`), and credits the bytes once after the helper returns, whatever its
   completion. A refused charge credits nothing.
4. **`builtin_call`.** It charges `size_of_val(args)` after `enter_function` and before the copy
   at `:359`, and credits it after the pop at `:372`. A refused charge ends the run in
   `Terminated(MemoryBudget)` with nothing pushed.
5. **Stays as it is.** Every result and its order; every step total; determinism.

Also in this story, no behaviour change:

- (a) The doc of `heap::store` (`crates/swell/src/heap/mod.rs:272-276`) ends "This and
  [`Accounting::charge`] are the only way into an arena", while `store_charged` (`:293-313`) is
  the other caller of `Arena::allocate` outside the tests (`:285`, `:307`). The sentence
  becomes: "This and [`store_charged`], for a value whose owned bytes are charged already, are
  the only way into an arena."
- (b) `HeapString::equals` (`crates/swell/src/heap/string.rs:183-193`) carries
  `cfg_attr(not(test), expect(dead_code, ...))` naming
  `story:string-builtins-charge-before-they-copy`. Its one caller is the test
  `equality_in_place_matches_to_utf16_for_every_pair_of_storage_kinds` (`:788-800`, the call at
  `:796`). It becomes `#[cfg(test)]` in place of the attribute, with the doc "Whether the record
  holds exactly `units`, compared in place: for the tests of this file; the engine compares two
  records with [`HeapString::equals_string`]." The function's body and the test's assertion stay
  as they are; a build without `test` no longer has the function, so no `dead_code` attribute
  is needed.
- (c) `STEP_CHUNK` (`crates/swell/src/builtins/json/parse.rs:41-42`) is documented as "The code
  units read between two charges of the step budget." Since `24704ac` the text's steps are
  charged by `charged_units` before the copy (`:269`), and `unpaid` counts one step per value
  made (`json_node`, `:495`), paid by `json_pay_chunk` (`:288-293`). The comment becomes "The
  values made between two charges of the step budget." The module doc (`:12-14`), "A step is
  charged per code unit read and per value made", becomes "A step is charged per code unit of
  the text before the text is copied, and per value made, in batches of `STEP_CHUNK`."
- (d) `docs/design/host-interface.md:1823` cites `host.yaml:867` for the marker "UNMAPPED: a
  run of Evaluate that ends with Terminated". `grep -n "UNMAPPED: a run of Evaluate"
  spec/domains/host.yaml` prints `932:  # UNMAPPED: a run of Evaluate that ends with
  Terminated. The modules that were evaluating stay` (its "Carried by" line is `:938`). The list
  cites each marker by its `UNMAPPED:` line (its first entry, `host.yaml:40`, is that line), so
  the citation becomes `host.yaml:932`. Found and not changed: the entries of that list from
  `host.yaml:43` on cite lines that moved too (for example `host.yaml:812` for the marker now at
  `:872`), and the marker's "Carried by" line names HOST-48 where line 1823 names HOST-188 to
  HOST-190; nothing under `spec/` changes here.

Design: none changes. No requirement of `host-interface.md` changes.

Writes: `crates/swell/src/heap/mod.rs` (`new_object_charged` beside `new_object`, line 217; the
doc at lines 272-276; the hook `COLLECTIONS` in `Heap::collected`, lines 211-214),
`crates/swell/src/object/bound_function.rs` (`bound_function_create`, `walk`, `call`,
`construct`, lines 76-264; the hooks `KEPT_COPIED` and `GATHERED_READING` beside
`ARGUMENTS_CLONED`, lines 266-271; tests in its `#[cfg(test)]` module, line 273),
`crates/swell/src/object/builtin_function.rs` (`builtin_call`, lines 338-379; the hook
`ACTIVATION_ARGUMENTS_COPIED`; a new `#[cfg(test)]` module at the end),
`crates/swell/src/heap/string.rs` (lines 183-193), `crates/swell/src/builtins/json/parse.rs`
(lines 12-14 and 41), `docs/design/host-interface.md` (line 1823), and `crates/swell/tests/`
only where a test's memory budget needs a new room: such a budget is restated as a measured
reading plus a room, and each one is named in the report.

The test-only hooks, each a thread-local under `#[cfg(test)]`, reset by the test that reads it:

- `KEPT_COPIED` (`bound_function.rs`, private): the values `bound_function_create` copied into
  `kept`.
- `GATHERED_READING` (`bound_function.rs`, private): `accounting.used()` read just before the
  helper calls `internal_call`.
- `ACTIVATION_ARGUMENTS_COPIED` (`builtin_function.rs`, `pub(crate)`): the values the
  activations copied; `story:a-call-charges-what-it-keeps-of-its-arguments-and-parameters` adds
  to it from `interp/mod.rs`.
- `COLLECTIONS` (`heap/mod.rs`, `pub(crate)`): the collections run, counted in
  `Heap::collected`.
- `ARGUMENTS_CLONED` (`bound_function.rs:266-271`) is read as it is.

## Acceptance

Unit tests in the lib test binary of `swell`. In each budget test the budget is a measured
reading plus a room: `heap_bytes()` read after a collection with the subject made and rooted
(the pattern of `realm_charge()` in `crates/swell/tests/determinism.rs` and of
`bound_arguments_are_not_copied_past_the_memory_budget`), plus half of the bytes of the copy the
test names, computed with `size_of` in the test. After each, with the budget raised, the same
operation answers normally, and after a collection the reading is the one before.

- `crates/swell/src/object/bound_function.rs`:
  `bound_arguments_are_charged_before_they_are_kept`: `bound_function_create` called directly
  with 100,000 Numbers under a budget of the reading plus half their bytes ends in
  `Terminated(MemoryBudget)`, and `KEPT_COPIED` reads 0 (today 100,000: the copy is made and
  `new_object` refuses after it). `the_gathered_list_is_charged_while_the_call_runs`: the
  function of `bound_with(100_000)` (`:284-328`) called with a roomy budget: `GATHERED_READING`
  is at least the reading before the call plus 100,000 times `size_of::<Value>()` (today equal
  to it, since `check` charges nothing), and after the call `accounting.used()` is the reading
  before it. `a_construction_that_faults_credits_the_gathered_list`: the helper reached with a
  missing new target (a test-only entry to it) ends in `Terminated(EngineFault)` and
  `accounting.used()` is the reading before (today nothing is charged, so this is a guard of the
  one credit point). `bound_arguments_are_not_copied_past_the_memory_budget` (`:358-385`) passes
  as it is.
- `crates/swell/src/object/builtin_function.rs`:
  `the_arguments_of_a_builtin_call_are_charged_before_they_are_recorded`: `builtin_call` of
  `Function.prototype` with 100,000 Numbers under a budget of the reading plus half their bytes
  ends in `Terminated(MemoryBudget)` and `ACTIVATION_ARGUMENTS_COPIED` reads 0 (today the call
  answers normally and the count reads 100,000).
  A guard (green before the change, since the base charges nothing per builtin call):
  `a_loop_of_builtin_calls_collects_within_its_bound` (default mode only,
  `#[cfg(not(feature = "collect-every-safe-point"))]`, where every safe point collects): the
  script `for (var i = 0; i < 100000; i++) Math.abs(i);` with a roomy budget; `COLLECTIONS`
  reads at most the count recorded on the base (the hook added first) plus
  ceil(100,000 times `size_of::<Value>()` / `MIN_TRIGGER_BYTES`).
- Guards: `step_totals_are_unchanged` (`crates/swell/src/object/array.rs:1010-1059`, its bound
  calls `b0x1()` to `b1000x3()`) and `the_step_totals_are_unchanged`
  (`crates/swell/tests/builtins_string.rs:976`) pass as they are.
- (a): `git grep -n 'are the only way into an arena' crates/swell/src/heap/mod.rs` prints one
  line, which names `store_charged`. (c): `git grep -n 'The values made between two charges'
  crates/swell/src/builtins/json/parse.rs` prints one line. (d): `git grep -n 'host.yaml:932'
  docs/design/host-interface.md` prints one line, and `git grep -n 'host.yaml:867'
  docs/design/host-interface.md` prints none. (b): `cargo clippy` below.

Each test runs under 5 s in both collection modes.

Commands. Each round: `cargo nextest run --locked -p swell --profile default --lib`, and
`cargo nextest run --locked -p swell --profile default --test <b>` for each `<b>` of
`builtins_function` and `builtins_string`, the same with `--features collect-every-safe-point`.
Once at the end, the whole package in both collection modes: `cargo nextest run --locked -p
swell --profile default --no-fail-fast` and the same with `--features
collect-every-safe-point` (655 tests per mode on `a6815ca`, a count this story did not make;
each mode under 60 s of run time). That run holds the stack tests (`eval_stack`,
`limits_scripts`, `budgets`), the re-entry-limit tests on the path of `builtin_call`
(`arrow_functions`, `builtins_function_source`, `builtins_uri`, `builtins_json_parse`,
`builtins_array_search`, `builtins_string_transform`, `builtins_object_integrity`), and
`termination`, `determinism` and `collect`, whose budgets and collection points the new charges
can move. Then `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo fmt
--all --check`, `cargo nextest run --locked -p swell-test262`, and once
`slot --timeout 30m task test262 TEST262_OUT=.out/test262-unit`, whose `runs.tsv` is compared
whole with that of `slot --timeout 30m task test262 TEST262_OUT=.out/test262-base` run on the
base: `cmp .out/test262-base/runs.tsv .out/test262-unit/runs.tsv` exits 0 (all six fields,
`crates/test262/src/count.rs:309-315`; the sixth, the run's detail, comes from the engine's
outcome, `crates/test262/src/run.rs:323-355`, with no time in it). Never `cargo test -p swell`
whole, `slot task test-collect`, `slot task test` or `slot task check`.

Rules this unit keeps. The failing test comes first: each test of the acceptance is written and
seen red before the change, for the reason stated beside it, except those named as guards
(`a_construction_that_faults_credits_the_gathered_list`,
`bound_arguments_are_not_copied_past_the_memory_budget`,
`a_loop_of_builtin_calls_collects_within_its_bound`, `step_totals_are_unchanged`,
`the_step_totals_are_unchanged`). A test is killed at 60 s and the gate's test steps fail over 120 s in sum; each test runs in a
process of its own under cargo-nextest, so nothing is shared through a `static` between tests.
Each new test is timed, and one over 5 s is named in the report with its time. A job of the
build pool ends after 10 minutes unless it names its time, 30 minutes at most (`slot --timeout
30m`, or `SLOT_TIMEOUT=30m` for `cargo`); a `timeout` wrapped around a command extends nothing.
Determinism: the same script under the same budgets ends the same way at the same count on
every run. Nothing is added to the frame of `Agent::execute`; `STACK_BYTES_PER_REENTRY` is
unchanged. No third-party crate, no unsafe code, no Python.

## Cost

No step total changes. Every builtin call now charges `size_of::<Value>()` per argument and every
bound call its gathered list; a credit does not lower `since_collection`, so these charges bring
collections forward: at most one more collection per `MIN_TRIGGER_BYTES` (1 MiB) of such
charges, fewer with a live heap above 1 MiB (`heap/mod.rs:213`). Transient charges already count
the same way (a frame, `ecmascript_function.rs:280-285`; a key list, `object/mod.rs:464-470`),
so no other kind of charge is added. Measured: the collections of
`a_loop_of_builtin_calls_collects_within_its_bound` against its bound; and the CPU of the test
binary `language`, seven runs alternating with the base in one pool slot, the median at most
1.03 times the base's.

## Report

The new tests with their times in both modes, the collection count of the loop on the base and
the unit, the CPU medians of `language`, the whole-package runs, every test of
`crates/swell/tests/` whose room changed, the test262 comparison, the two follow-ups of Context,
and this line for the changelog's "Changed" heading (no name of the host interface changes):
"A bound function's arguments, the argument list of a bound call and the arguments a builtin
call records are charged to the memory budget before they are copied and stay charged while
they are held."
