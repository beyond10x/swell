---
format: aep.planning-md/3
id: story:a-call-charges-what-it-keeps-of-its-arguments-and-parameters
kind: story
status: active
title: A call charges the parameter map of its arguments object and the arguments it records before it makes them, and copies no parameter list
summary: The mapped arguments object's map and the activation's copy of a call's arguments are charged to the memory ledger before they are made and credited where their holder drops them; the copy a call makes of its function's parameter list goes. No step total changes.
relations:
- decomposes: epic:slice-06-bindings-and-iteration
- depends_on: story:bound-functions-and-builtin-calls-charge-before-they-copy
- depends_on: story:default-and-rest-parameters
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T13:48:06Z", actor: "human:operator", revision: 2, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-22T13:48:07Z", actor: "human:operator", revision: 3, executor: "agent:coordinator"}
---
Waits for: `story:bound-functions-and-builtin-calls-charge-before-they-copy` (it adds
`Heap::new_object_charged` and the test-only hook `COLLECTIONS` this story uses), and the slice-6 story `story:default-and-rest-parameters`, which
changes how a call sets up a function's parameters and its arguments object and writes
`crates/swell/src/object/ecmascript_function.rs` (`.engineering/planning/story/default-and-rest-parameters.md:92`);
`parameter_map`, `make_frame`, `call` and `construct` are read again after it has merged.
Nothing else. No requirement of `host-interface.md`. Consumer: none named. A gap of the rule of
`docs/design/engine.md`, "Memory budget": storage is charged before it is allocated, and the
code that holds a charged structure credits it when it drops it, on every way out.

## Context

Line numbers are of `wave/budget-1` at `a6815ca`; they are read again on the unit's base. The
accounting calls are those named in the Context of
`story:bound-functions-and-builtin-calls-charge-before-they-copy` (`charge` adds to
`since_collection`, `check` charges nothing, `credit` does not lower `since_collection`).

1. **`parameter_map`** (`crates/swell/src/object/arguments.rs:155-213`, called by
   `create_arguments_object`, `:77-148`, at `:87`; that is called by `instantiate_arguments`,
   `crates/swell/src/object/ecmascript_function.rs:353-375`, from `make_frame`). It allocates
   `slots` (`Vec<Option<u32>>`, `mapped` entries, the smaller of the number of arguments and of
   parameters, `:160`) and `seen` (`Vec<bool>`, `width` entries, one past the highest
   environment slot of a parameter, `:168-178`). The parameters come from the source, the
   arguments from the call (`f.apply` takes any array-like). Both are asked with `check`
   (`:179-189`), which charges nothing, and reserved (`:190-194`). `seen` is never charged and
   drops at the end of `parameter_map` (`:212`). `slots` is charged after it exists, as the
   record's owned bytes read from its capacity (`:90-97`), when `new_object` stores the record
   (`:103-106`), after the intrinsic lookup at `:98`.
2. **`make_frame`'s copy of the parameters** (`ecmascript_function.rs:243-347`, called by
   `call_function`, `:216`, and by `crates/swell/src/interp/dispatch.rs:1402` and `:1448`).
   `let params = template.params.clone();` (`:258`) copies the template's `Vec<Slot>`, one entry
   per formal parameter. It is neither checked nor charged, and it is held across
   `ordinary_call_bind_this` (`:266`), the function environment (`:274`), the frame's charge
   (`:280-285`) and the arguments object (`:311-327`). The copy is not needed: `callee` is a
   local, `template` borrows its `compiled` (an `Rc`, `:252`), `params` is last read by the
   `Mapping` (`:312-315`) passed to `instantiate_arguments` (`:316-322`), and `callee.compiled`
   moves into the `Frame` only after that (`:330`). `instantiate_arguments` is
   `#[inline(never)]` (`:353`) to keep the call's frame small; nothing is added to that frame.
3. **The arguments a call from Rust records.** `call` and `construct`
   (`ecmascript_function.rs:140-157`, `:163-180`) build an `Activation` with `args.to_vec()`
   (`:150`, `:173`) for `enter_code` (`crates/swell/src/interp/mod.rs:96-129`). `enter_code`
   pushes it only when a loop is running and no builtin is between that loop and this call
   (`depth == base`, `:114-118`, the case `Entered::Activation`); in the cases `Outermost` and
   `Reentry` it drops the copy at once. `leave_code` (`:132-136`) pops what was pushed. The
   kept case with an argument list a script sets: a bound function whose target is an
   ECMAScript function, called by a `Call` instruction (`dispatch.rs:1348`, then `walk`'s
   `internal_call`), which records the whole gathered list. The dropped case: `f.apply(null,
   list)`, whose builtin's activation lies between (`Reentry`), copies `list` and frees it. No
   copy is checked or charged.

Follow-ups, not in this story: the copies of `call_instruction` and `new_instruction`
(`crates/swell/src/interp/dispatch.rs:1336`, `:1415`), a story after slice 6 (every slice-6
story writes `dispatch.rs`, and spread calls make these copies as long as a script likes); and
`string_units`/`string_units_in` (`crates/swell/src/object/string.rs:48-84`, 33 callers in 17
files), named in the first story.

## Change

Failing tests first (Acceptance). Then:

1. **`parameter_map`.** `charge_bytes` of the bytes of `slots` and `seen` replaces the `check`
   (`:179-189`); a failed reservation (`:192-194`) and each fault in the loop (`:198-203`)
   credit them; at the end the bytes of `seen` are credited as it drops, and those of `slots`
   stay charged with the map. `create_arguments_object` sets the record's owned bytes to the
   bytes charged for `slots` (not read from its capacity), credits them when the intrinsic
   lookup (`:98`) ends abruptly, and stores the record with `new_object_charged`, crediting
   them on a refusal. The sweep that frees the record credits them, as today.
2. **`make_frame`.** `let params = &template.params;` in place of the copy at `:258`, and the
   `Mapping` takes it as a slice. No allocation, so nothing to charge and no helper. (Whether the
   borrow compiles was read, not built.)
3. **The activation's copy: `enter_code` decides, charges and copies.** `enter_code` takes
   `Option<(Activation, &[Value])>`: the activation with empty `args`, and the arguments as a
   slice. It decides as today (`:101-123`), and only in its push branch (`:114-118`) it charges
   `size_of_val(args)`, copies the arguments into the activation and pushes it; a refused charge
   ends the run in `Terminated(MemoryBudget)` with nothing pushed and `Entered::Activation` not
   answered. In the branches `Reentry` and `Outermost` nothing is copied or charged. `leave_code`
   (`:132-136`), for `Entered::Activation`, pops the activation and credits `size_of_val` of its
   `args`; it runs on every way out, the interrupt check of `enter_code` (`:124-127`) included.
   No path pushes an activation that `leave_code` pops without its charge: `Entered::Activation`
   is answered only by the push branch, and the two other pushes of `activations`
   (`builtin_call`, `builtin_function.rs:356`, charged by the first story; and
   `ensure_can_compile_strings`, `dynamic_code.rs:92`, one argument, `:406`) pop their own
   activation (`builtin_function.rs:372`, `dynamic_code.rs:98`) and never reach `leave_code`.
   There is no second method. `call` and `construct` of `ecmascript_function.rs` pass the
   activation and `args` instead of `args.to_vec()`; the callers that pass `None`
   (`script.rs:555`, `dynamic_code.rs:456`) compile unchanged and are not written. Chosen over an
   `Entered` that carries the charged bytes because it is the smaller diff by reading: the charge
   has to sit in `enter_code`'s push branch either way, and carrying bytes in `Entered` would
   also change that `Copy` enum (`:61-69`) and the comparison in `leave_code` (`:133`).
4. **Stays as it is.** Every result and its order; every step total; determinism.

Design: none changes. No requirement of `host-interface.md` changes.

Writes: `crates/swell/src/object/arguments.rs` (`create_arguments_object` and `parameter_map`,
lines 77-213; the hooks below; a new `#[cfg(test)]` module at the end),
`crates/swell/src/object/ecmascript_function.rs` (`call` and `construct`, lines 140-180; line
258 and the `Mapping` at lines 312-315 of `make_frame`; the hook `CALL_ARGUMENTS_COPIED`; a new
`#[cfg(test)]` module at the end), `crates/swell/src/interp/mod.rs` (two hunks: `enter_code`,
lines 96-129, its doc, signature and push branch; `leave_code`, lines 132-136; nothing else in
the file, whose `mod` lines 33-38 the slice-6 story this one waits for writes, and the hunks of
the other slice-6 stories in it are read again on the base), and
`crates/swell/tests/` only where a test's memory budget needs a new room: such a budget is
restated as a measured reading plus a room, and each one is named in the report.

The test-only hooks, each a thread-local under `#[cfg(test)]`, reset by the test that reads it:

- `MAP_READINGS` (`arguments.rs`, private): `accounting.used()` at the entry of
  `parameter_map` and once `slots` and `seen` are reserved.
- `MAPS_RESERVED` (`arguments.rs`, private): the parameter maps reserved.
- `CALL_ARGUMENTS_COPIED` (`ecmascript_function.rs`, `pub(crate)`): the argument values copied
  for the activation of a call from Rust. Added before the change at the two copies of today
  (`args.to_vec()`, `:150`, `:173`); after the change it is counted in the push branch of
  `enter_code` (`interp/mod.rs`), the one place that copies. It counts nothing of
  `builtin_call`'s copy, which the first story's `ACTIVATION_ARGUMENTS_COPIED` counts.
- `COLLECTIONS` (`heap/mod.rs`, `pub(crate)`, added by the first story).

## Acceptance

Unit tests in the lib test binary of `swell`. In each budget test the budget is a measured
reading plus a room: `heap_bytes()` read after a collection with the subject made and rooted
(the pattern of `realm_charge()` in `crates/swell/tests/determinism.rs` and of
`bound_arguments_are_not_copied_past_the_memory_budget`), or a reading a hook recorded on a
first run of the same script on an agent made the same way, plus the room the test states.
After each, with the budget raised, the same operation answers normally, and after a collection
the reading is the one before.

- `crates/swell/src/object/arguments.rs`: `the_parameter_map_is_charged_while_it_is_held`: a
  non-strict function of 1,000 parameters that reads `arguments`, its source made by the test,
  called with 1,000 arguments: the second reading of `MAP_READINGS` is at least the first plus
  the bytes of `slots` and `seen` (today the two are equal, since `check` charges nothing). A
  guard, `the_parameter_map_is_not_made_past_the_memory_budget`: the first reading of
  `MAP_READINGS`, recorded on a first run with a roomy budget, plus half the map's bytes: the
  call ends in `Terminated(MemoryBudget)` and `MAPS_RESERVED` reads 0 (it passes today through
  `check`; it holds the order).
- `crates/swell/src/object/ecmascript_function.rs`:
  `the_arguments_a_call_records_are_charged_before_they_are_copied`: `function f() {}` bound with
  100,000 arguments through `Function.prototype.bind.apply` and called as `b()` by a script, the
  case where `enter_code` keeps the activation; budget: the reading plus 100,000 times
  `size_of::<Value>()` (the gathered list, charged by the first story) plus half as much again.
  The script ends in `Terminated(MemoryBudget)` and `CALL_ARGUMENTS_COPIED` reads 0 (today the
  call answers normally and it reads 100,000).
  `a_call_that_records_nothing_copies_nothing`: `f.apply(null, list)` with 100,000 values and a
  roomy budget answers normally and `CALL_ARGUMENTS_COPIED` reads 0 (today 100,000, copied at
  `:150` and dropped by `enter_code` in the case `Reentry`).
  A guard (green before the change, since the base charges nothing for the copy):
  `a_loop_of_bound_calls_collects_within_its_bound` (default mode only,
  `#[cfg(not(feature = "collect-every-safe-point"))]`): `var b1 = f.bind(null, 1); for (var i =
  0; i < 100000; i++) b1();` with a roomy budget; `COLLECTIONS` reads at most the count recorded
  on the base plus ceil(100,000 times `size_of::<Value>()` / `MIN_TRIGGER_BYTES`).
- `make_frame`: `git grep -n 'params.clone()' crates/swell/src/object/ecmascript_function.rs`
  prints nothing. No test-only count can read a copy that no longer has a site; the calls of the
  whole-package run below hold the behaviour.
- Guards: `step_totals_are_unchanged` (`crates/swell/src/object/array.rs:1010-1059`, the entry
  `t()` reads `arguments` through bound calls) and `the_step_totals_are_unchanged`
  (`crates/swell/tests/builtins_string.rs:976`) pass as they are.

Each test runs under 5 s in both collection modes; the time of the 1,000-parameter function's
parse and call is measured and reported.

Commands. Each round: `cargo nextest run --locked -p swell --profile default --lib`, and
`cargo nextest run --locked -p swell --profile default --test <b>` for each `<b>` of `calls`,
`builtins_function` and `language` (its `arguments-*.js` cases, `crates/swell/tests/language/`), the same
with `--features collect-every-safe-point`. Once at the end, the whole package in both collection
modes: `cargo nextest run --locked -p swell --profile default --no-fail-fast` and the same with
`--features collect-every-safe-point` (655 tests per mode on `a6815ca`, a count this story did
not make; each mode under 60 s of run time). That run holds the stack tests (`eval_stack`,
`limits_scripts`, `budgets`), the re-entry-limit tests on the path of `call` and `enter_code`
(`arrow_functions`, `builtins_function_source`, `builtins_uri`, `builtins_json_parse`,
`builtins_array_search`, `builtins_string_transform`, `builtins_object_integrity`), and
`termination`, `determinism` and `collect`, whose budgets and collection points the new charges
can move. Then `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo fmt
--all --check`, `cargo nextest run --locked -p swell-test262`, and once
`slot --timeout 30m task test262 TEST262_OUT=.out/test262-unit`, whose `runs.tsv` is compared
whole with that of `slot --timeout 30m task test262 TEST262_OUT=.out/test262-base` run on the
base: `cmp .out/test262-base/runs.tsv .out/test262-unit/runs.tsv` exits 0 (all six fields,
`crates/test262/src/count.rs:309-315`). Never `cargo test -p swell` whole, `slot task
test-collect`, `slot task test` or `slot task check`.

Rules this unit keeps. The failing test comes first: each test of the acceptance is written and
seen red before the change, for the reason stated beside it, except those named as guards
(`the_parameter_map_is_not_made_past_the_memory_budget`,
`a_loop_of_bound_calls_collects_within_its_bound`, `step_totals_are_unchanged`,
`the_step_totals_are_unchanged`). A
test is killed at 60 s and the gate's test steps fail over 120 s in sum; each test runs in a
process of its own under cargo-nextest, so nothing is shared through a `static` between tests.
Each new test is timed, and one over 5 s is named in the report with its time. A job of the
build pool ends after 10 minutes unless it names its time, 30 minutes at most (`slot --timeout
30m`, or `SLOT_TIMEOUT=30m` for `cargo`); a `timeout` wrapped around a command extends nothing.
Determinism: the same script under the same budgets ends the same way at the same count on
every run. Nothing is added to the frame of `Agent::execute` or of `make_frame`;
`STACK_BYTES_PER_REENTRY` is unchanged. No third-party crate, no unsafe code, no Python.

## Cost

No step total changes. A call that keeps its activation now charges `size_of::<Value>()` per
argument, and a call that makes a mapped arguments object the bytes of `seen` (`slots` was
charged before, later); a credit does not lower `since_collection`, so collections come at most
one per `MIN_TRIGGER_BYTES` (1 MiB) of these charges earlier, fewer with a live heap above 1 MiB
(`heap/mod.rs:213`). A call no longer allocates a parameter list, and a call that records nothing
no longer copies its arguments. Measured: the collections of
`a_loop_of_bound_calls_collects_within_its_bound` against its bound; and the CPU of the test
binary `language`, seven runs alternating with the base in one pool slot, the median at most
1.03 times the base's.

## Report

The new tests with their times in both modes, the collection count of the loop on the base and
the unit, the CPU medians of `language`, the whole-package runs, every test of
`crates/swell/tests/` whose room changed, the test262 comparison, the follow-ups of Context, and
this line for the changelog's "Changed" heading (no name of the host interface changes): "The
parameter map of a mapped arguments object and the arguments a call records are charged to the
memory budget before they are made; a call no longer copies its function's parameter list."
