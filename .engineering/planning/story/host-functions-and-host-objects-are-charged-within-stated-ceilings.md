---
format: aep.planning-md/3
id: story:host-functions-and-host-objects-are-charged-within-stated-ceilings
kind: story
status: implemented
title: Every byte of a host function, a host object and a property definition is charged, each within a ceiling the engine states as a constant and the design quotes, and a realm of 300 host functions and 40 host objects is measured
summary: The boxes of a host function's steps and data are charged as the record's own bytes before they are made; per-item ceilings on live_bytes() and a realm ceiling on heap_bytes() are public constants the design quotes and a test compares; one test builds a realm the size of the browser's generated bindings and states the steps a host function's call spends.
relations:
- depends_on: story:arena-capacity-and-collector-workspace-on-the-ledger
- depends_on: story:own-keys-charged-before-the-list-exists
- depends_on: story:the-host-sets-the-prototype-of-an-object-it-made
- depends_on: story:the-host-names-a-closed-list-of-intrinsics-and-symbols
- depends_on: story:the-host-charges-its-own-bytes-and-steps
- depends_on: story:an-error-object-is-read-without-running-code
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T06:48:26Z", actor: "human:operator", revision: 2, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-22T06:48:27Z", actor: "human:operator", revision: 3, executor: "agent:coordinator"}
- {from: "active", to: "implemented", at: "2026-11-22T17:07:50Z", actor: "human:operator", revision: 4, decided_on: {"recorded":{"test_result":1}}, executor: "agent:coordinator"}
---
Waits for: the budget batch's `story:arena-capacity-and-collector-workspace-on-the-ledger`
(slots charged by capacity and `live_bytes()`, which the ceilings are stated on) and
`story:own-keys-charged-before-the-list-exists` (it moves `reserve_kept` and `end_sort_list`
into `crates/swell/src/object/builtin_function.rs`, other lines than this story's), and
`story:the-host-sets-the-prototype-of-an-object-it-made` and
`story:the-host-names-a-closed-list-of-intrinsics-and-symbols` (the test builds its interface
objects with them), `story:the-host-charges-its-own-bytes-and-steps` (the steps sentence of the
design names its `cx.charge_steps`) and `story:an-error-object-is-read-without-running-code`
(both write `crates/swell/tests/host_objects.rs`, and that story adds `error_kind` to every
`HostObjectSpec` literal: it lands first, and this story's literals carry `error_kind: None`).
Batch H3. Requirements HOST-186, HOST-187. Consumer: the browser,
requirement 10 of its list. Decision D10 (taken: option A, stated ceilings and a measuring
test).

## Context

Line numbers are of `wave/budget-1` at `ed56b05`.

- A host function is `create_builtin_function` with `Behaviour::Host { steps: Box<dyn Any>, data }`
  (`crates/swell/src/object/builtin_function.rs:219-243`): its record is stored by
  `allocate_object`, which passes owned bytes `0` to `Heap::new_object`
  (`crates/swell/src/object/mod.rs:198-208`, the `0` at `:204`; `new_object` takes them,
  `crates/swell/src/heap/mod.rs:119-126`), so `store` charges the slot only
  (`heap/mod.rs:177-192`). The box of the steps (`Box::new(steps)`, `builtin_function.rs:234`)
  and the box of `FunctionData` with the host's data inside it (`:228-231`) are allocated before
  any charge and charged nowhere. Its name String is charged (`new_string_str`, `:190`), its
  `length` and `name` properties by the growth of its property list (`add_property`,
  `crates/swell/src/object/mod.rs:214-250`).
- A host object charges its slot and `size_of_val` of its data's box
  (`crates/swell/src/host/mod.rs:238-246`), not what the data owns behind it (that is the host's
  to charge, `story:the-host-charges-its-own-bytes-and-steps`).
- A property definition through `cx.define_property_or_throw` (`crates/swell/src/host/mod.rs:627`)
  charges the growth of the list before it is made, doubling (`KeyedList::make_room`,
  `crates/swell/src/object/property.rs:313-350`), and the index map past `SMALL` = 8 (`:184`).
- The two readings once the arena story merged (`docs/design/host-interface.md:169-181`):
  `heap_bytes()` holds an arena's capacity, which doubles at growth and stays charged while the
  agent lives; `live_bytes()` holds what live records own, their slots included, and falls by
  everything a sweep frees. A per-item rise of `heap_bytes()` is therefore 0 for most items and
  a whole arena's growth for one; of `live_bytes()` it is the item's own bytes.
- Steps. The counter that `last_entry_steps()` reads is set to 0 by `create_realm`
  (`crates/swell/src/host/agent.rs:261`), by an outermost entry of a script (`:452`), and by
  `Agent::begin` (`:542-553`, the reset at `:545`), which a `Cx` call that may run code or spend
  steps calls first (`define_property_or_throw`, `crates/swell/src/host/mod.rs:639`, and the
  others of `:556-798`); `with_cx` does not reset it. `new_host_function` (`host/mod.rs:523-540`)
  and `new_host_object` (`:215-255`) call only `Agent::running` and charge no step, so they leave
  the counter as it was. A definition that does not compact charges none after its `begin`.
  `set_prototype_of` charges one step per object on the new prototype's chain, from the new
  prototype to the end (`crates/swell/src/object/ordinary.rs:141-148`, the step at `:146`), and
  none when the prototype is already the current one (`:135-137`). A call charges one step
  per instruction (`crates/swell/src/interp/dispatch.rs:45`). The call of a host function adds
  one step at entry (`enter_function`, `builtin_function.rs:247-250`); the call of a script
  function adds one step (`push_call_frame`, `dispatch.rs:1373`) and then runs its body: an
  empty body is the two instructions `Undefined` and `Return`
  (`crates/compiler/src/function.rs:88-89`). So the script `g()` of an empty script function
  `g` spends exactly 2 steps more than the script `f0()` of a host function, the two scripts
  compiling to the same instructions.
- No document states any of these numbers; the browser counts 137 function objects for the four
  interfaces of its table alone, about 300 with the others, and 40 host objects.

## Change

Failing tests first (Acceptance). Then:

1. **Charge the boxes before they exist, by the `Grown` route.** `create_host_function`
   computes the bytes of the steps' box (`size_of::<HostFn<H>>()`), of `FunctionData`'s box and
   of the host's data (`size_of_val` of the box the host made), charges them with
   `Accounting::charge` before it boxes anything (`builtin_function.rs:230`, `:234`); over the
   budget nothing is allocated and the run ends in `Terminated(MemoryBudget)`. When
   `create_builtin_function` then fails, it credits them back; when it answers the function, it
   records them with `record_grown` (`crates/swell/src/object/mod.rs:252-258`), so the sweep that
   frees the function queues their credit as it does for a property map (`Grown`, `:45-70`).
   `allocate_object` and its four other callers do not change.
2. **Ceilings, one place** (decision D10, option A). `crates/swell/src/host/limits.rs` gains four
   public constants, each the value the test measures plus an eighth, rounded up to a multiple
   of 64:
   - `HOST_FUNCTION_LIVE_BYTES`: the rise of `live_bytes()` for a host function with a name of
     32 code units and no data;
   - `HOST_OBJECT_LIVE_BYTES`: for a host object with no data;
   - `PROPERTY_LIVE_BYTES`: per definition of a data or accessor property, amortized over 300
     definitions on one object;
   - `BINDINGS_REALM_HEAP_BYTES`: the rise of `heap_bytes()` for the realm of the Acceptance
     (300 host functions, 40 host objects, 600 definitions), arena growth included.
   The design quotes each as "`NAME` = <value>"; the test
   `the_design_quotes_the_ceilings_of_the_engine` compares them (Acceptance), so the document
   cannot state another number.
3. **Steps, stated** (decision 14): making a host function or object spends no step and leaves
   the counter of the running entry unchanged; defining a property that does not compact spends
   none; `set_prototype_of` spends one per object on the new
   prototype's chain; a call of a host function spends one step at entry besides the
   instructions of the call, which is 2 fewer than a call of an empty script function, and what
   the host charges with `cx.charge_steps`.
4. **Stays as it is.** The interface's signatures; the costs of the engine's own builtins;
   determinism (the same calls charge the same bytes).

Design preconditions (the design author writes them before the unit starts):
`docs/design/host-interface.md` decision 14 (host functions, properties) and 15 (host objects)
gain "A host function costs at most `HOST_FUNCTION_LIVE_BYTES` = <value> bytes of
`live_bytes()` plus its data, a host object `HOST_OBJECT_LIVE_BYTES` = <value> plus its data, a
property definition `PROPERTY_LIVE_BYTES` = <value>; a realm of 300 host functions and 40 host
objects with 600 definitions raises `heap_bytes()` by at most `BINDINGS_REALM_HEAP_BYTES` =
<value>, arena growth included. A release that raises a ceiling names it under the changelog's
Host interface heading.", with the values the unit reports written in before its branch merges,
and the steps sentence of point 3. And the lines of HOST-186 and HOST-187 below, with their
pending lines in `conformance/host/requirements.txt`.

New requirements:

- **HOST-186** (swell:D14) After a realm's setup, `new_host_function` with a name of at most 32
  code units and data of a `T` raises `live_bytes()` by at most `HOST_FUNCTION_LIVE_BYTES` plus
  `size_of::<T>()`, and by the same amount for each further function of the same name length
  and type; `new_host_object` with data of a `T` by at most `HOST_OBJECT_LIVE_BYTES` plus
  `size_of::<T>()`; 300 definitions of new properties on one host object by at most 300 ×
  `PROPERTY_LIVE_BYTES`; once the host drops them and `collect` ran twice, `live_bytes()` reads
  its value from before them. [slice 6]
- **HOST-187** (swell:D14) A realm with 300 host functions and 40 host objects whose properties
  are defined in `with_cx` raises `heap_bytes()`, from its reading after `create_realm` and
  before the first `with_cx`, by at most `BINDINGS_REALM_HEAP_BYTES`; with the baseline
  `last_entry_steps()` read after a `with_cx` whose last call began an entry (a
  `define_property_or_throw`, which reads 0 after it), a `with_cx` of only `new_host_function`
  or only `new_host_object` calls leaves `last_entry_steps()` unchanged; `set_prototype_of` with
  a new prototype whose chain has k objects reads k; three of its host functions, bound after the
  `heap_bytes()` reading as the global data properties `f0`, `f150` and `f299`, make the scripts
  `f0()`, `f150()` and `f299()` spend the same `last_entry_steps()`, which is 2 less than that of
  the script `g()` of a script function `g` with an empty body; once the realm's roots are
  dropped and `collect` ran twice, `live_bytes()` reads its value from before `create_realm` (the
  three bindings are properties of the realm's global and go with it), and building the same
  realm again leaves `heap_bytes()` at most where the first build left it. [slice 6]

Writes: `crates/swell/src/object/builtin_function.rs` (`create_host_function`, lines 219-243,
nothing else), `crates/swell/src/host/limits.rs` (the four constants; nothing of `Limits`),
`crates/swell/tests/host_objects.rs`, and in `conformance/host/requirements.txt` the lines of
HOST-186 and HOST-187 only. The values in `host-interface.md` are the design author's.

## Acceptance

In `crates/swell/tests/host_objects.rs`, in both collection modes:

- `host_186_a_host_function_charges_every_byte`: a host function with data of 4,096 bytes:
  today `live_bytes()` rises by less than 4,096 (the data's box is charged nowhere); after, by at
  least 4,096 and at most `HOST_FUNCTION_LIVE_BYTES` + 4,096; dropping it and two `collect`s
  return `live_bytes()` to before. Then the other ceilings of HOST-186.
- `host_187_a_realm_of_300_host_functions_and_40_host_objects`: one test function builds the
  realm inside its own body and makes every assertion on it; no other test reads it and nothing
  goes through a `static` (each test runs in a process of its own under cargo-nextest, so a
  shared fixture would be built once per test anyway). The realm: a setup closure that makes a
  global host object; in one `with_cx` per kind of call, 13 interface objects (host functions,
  `constructor: true`, each but the first given its parent as [[Prototype]] with
  `cx.set_prototype_of`), 13 interface prototype objects and 27 other host objects (40 host
  objects), and 287 member functions (300 host functions with the 13), each defined on a
  prototype as a data property or as the getter of an accessor, names of 4 to 32 code units from
  a fixed list. After the `heap_bytes()` reading it defines on the realm's global object the
  data properties `f0`, `f150` and `f299` holding the member functions of those indexes, and
  `function g() {}` by a script; globals rather than a rooted object, because the
  `live_bytes()` return is measured after the whole realm is dropped, which frees its global
  with them. It asserts HOST-187, builds the realm a second time as HOST-187 says, and prints
  the measured rises for the four constants as one line the report quotes. Time bound: at most 5
  s in each collection mode in the dev profile, the realm built twice; the test prints its own
  time, and the scripts it runs call 3 host functions, not 300, so that the collect mode's
  collection at every safe point stays inside the bound.
- `set_prototype_of_spends_a_step_per_object_of_the_new_chain`: for k of 1, 2 and 10, a chain
  of k host objects (the first made with `Prototype::Null`, each next given the one before by
  `set_prototype_of`) and a fresh host object `t`; a `with_cx` whose last call is
  `cx.set_prototype_of(t, Some(last))` answering `Ok(Ok(true))` leaves `last_entry_steps()` at k;
  setting the same prototype again leaves it at 0. A guard: story 8 made the method, and this
  test holds the number the design's steps sentence quotes.
- `the_design_quotes_the_ceilings_of_the_engine`: reads the file `HOST_INTERFACE_DOC` names
  (default `docs/design/host-interface.md` under the workspace root, found from
  `CARGO_MANIFEST_DIR`) with `std::fs::read_to_string`, and asserts that it holds
  "`HOST_FUNCTION_LIVE_BYTES` = <value>" and the same for the other three, each `<value>` the
  constant's. Today it does not compile (no constants). The order: the unit measures the values
  and shows this test green on a copy, `.out/host-interface.md`, with the four values written in
  (`HOST_INTERFACE_DOC` set to its path); the report gives the design author the four values;
  the design author writes them into `docs/design/host-interface.md` on the unit's branch before
  it merges; the coordinator runs the test on the merged branch.

Commands, each one job of the build pool (one slot): `slot cargo nextest run --locked -p swell
--profile default --test <b>` for each `<b>` of `host_objects`, `calls` and `collect`, the same
with `--features collect-every-safe-point`, `slot cargo clippy --workspace --all-targets
--locked -- -D warnings`, `slot task conform-engine` (its counts of the base), and last, once, in
the background, `slot timeout 1800 task host-requirements-check` (it builds every test target of
the workspace). No test262 run: no builtin changes, and the test262 host makes 3 host functions per
realm (`crates/test262/src/host.rs`), whose charge grows by the boxes of three functions, far
inside its `memory_bytes` (the report quotes that setting). Never `cargo test -p swell` whole,
`slot task test-collect`, `slot task test` or `slot task check`.

Rules this unit keeps. The failing test comes first: each test of the acceptance is written and
seen red before the change (the first by its charge of the boxes, the second and the last
because the constants do not exist), except the one named as a guard. A test is killed at 60 s and the gate's test steps fail over 120
s; each test runs in a process of its own under cargo-nextest, so nothing is shared through a
`static` between tests, and the realm of 340 objects is built inside the one test that reads it.
Each new test is timed, and one over 5 s is named in the report with its time. A job of the
build pool asks for one slot; one over 10 minutes is killed, so `host-requirements-check` on a cold tree and the full test262 run are named `slot timeout 1800 <command>`. Determinism: the same calls charge the same bytes on every run.
Nothing is added to the frame of `Agent::execute`; `STACK_BYTES_PER_REENTRY` and the stack tests
are unchanged. No third-party crate, no unsafe code.

## Report

The measured rises; for the design author, the four values as they are to be written,
"`HOST_FUNCTION_LIVE_BYTES` = <value>", "`HOST_OBJECT_LIVE_BYTES` = <value>",
"`PROPERTY_LIVE_BYTES` = <value>", "`BINDINGS_REALM_HEAP_BYTES` = <value>"; the tests with
their times; and this line for the
changelog's "Host interface" heading: "A host function now charges its steps' and data's boxes;
`HOST_FUNCTION_LIVE_BYTES`, `HOST_OBJECT_LIVE_BYTES`, `PROPERTY_LIVE_BYTES` and
`BINDINGS_REALM_HEAP_BYTES` (new) state what a host function, a host object, a property
definition and a realm of 300 host functions and 40 host objects cost at most
(`host-interface.md`, decisions 14 and 15; `HOST-186`, `HOST-187`)."
