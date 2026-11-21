---
format: aep.planning-md/3
id: story:object-model-and-realms
kind: story
status: implemented
title: Objects, abstract operations, realms, Error objects and the Host trait, tested from Rust
summary: Ordinary, Array, String and built-in function objects, clause 7 operations and operators, realms and intrinsics, 19.1 and 20.5, every hook with its default, Cx and Agent call paths, the step counter; 9 HOST lines to test.
relations:
- decomposes: epic:slice-01-harness
- depends_on: story:heap-values-and-host-surface
scope:
- confidence: cited
  path: conformance/host/requirements.txt
- confidence: cited
  path: crates/swell/Cargo.toml
- confidence: cited
  path: crates/swell/src/builtins
- confidence: cited
  path: crates/swell/src/compile_fail.rs
- confidence: cited
  path: crates/swell/src/heap
- confidence: cited
  path: crates/swell/src/host
- confidence: cited
  path: crates/swell/src/host/host_object.rs
- confidence: cited
  path: crates/swell/src/ops
- confidence: cited
  path: crates/swell/src/realm
- confidence: cited
  path: crates/swell/src/value.rs
- confidence: cited
  path: crates/swell/tests
- confidence: cited
  path: crates/swell/tests/budgets.rs
- confidence: cited
  path: crates/swell/tests/collect.rs
- confidence: cited
  path: crates/swell/tests/host_objects.rs
- confidence: cited
  path: crates/swell/tests/interrupt.rs
- confidence: cited
  path: crates/swell/tests/support/host.rs
revision: 13
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T04:49:59Z", actor: "human:operator", revision: 6, decided_on: {"recorded":{"review_outcome":2}}, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-21T04:49:59Z", actor: "human:operator", revision: 7, decided_on: {"recorded":{"review_outcome":2}}, executor: "agent:coordinator"}
- {from: "active", to: "implemented", at: "2026-11-21T08:09:48Z", actor: "human:operator", revision: 13, decided_on: {"recorded":{"test_result":1,"review_outcome":2,"verification":1}}, executor: "agent:coordinator"}
---
## Context

On the heap of `story:heap-values-and-host-surface`, this story builds the object model, the
abstract operations, realms and the global object, the Error objects, the whole `Host` trait
with its provided defaults, and the `Cx` and `Agent` operations that need objects but no
ECMAScript code. Everything here is tested from Rust through `with_cx`, host functions and
`Agent::call`, so it does not wait for the interpreter.

What the 31 cases of slice 1 and their harness files reach in this part (measured on the
harness files with comments and string contents removed; the list per builtin is in
`story:harness-builtins`): ordinary objects with accessors, Array exotic objects (`length`),
String exotic objects (`String({})` goes through `ToObject` and `Object.prototype.toString`),
`instanceof` through `OrdinaryHasInstance`, `typeof`, every ES5 operator, `for-in` through
`EnumerateObjectProperties`, the global value properties `NaN`, `Infinity` and `undefined`
(19.1), and the seven Error constructors (`assert.throws` compares constructors; every
`assert-throws-*` case throws or expects one; `Test262Error` is defined by `sta.js`).
`globalThis` is not taken: none of the 31 cases or their harness files names it (`grep -c
globalThis` over them, comments removed, prints 0), and `HOST-155` gives it to slice 3.
`ToObject` of a Boolean or a Number needs `%Boolean.prototype%` and `%Number.prototype%`; they
exist here as their clauses define them, without their methods (slices 3 and 4).

`ToNumber` of a String trims `StrWhiteSpaceChar` with the `Zs` table of `swell-unicode` and
parses the rest with `swell-num` (`story:num-and-unicode-early-parts`, `sec-stringtonumber`).
`ToString` of a Number uses `Number::toString` radix 10 of `swell-num`.

The `Host` trait of `docs/design/host-interface.md` section 16 names types of later slices in
its required methods (`Job`, `LoadRequest`, `ModuleRef`). `Job` and `LoadRequest` are defined
here in `host/hooks.rs` as the handle types sections 9 and 11 give them (`ModuleRef` is in
`value.rs`, `story:heap-values-and-host-surface`), with no engine path that creates one until
slices 10 and 11; `HOST-153` (slice 11) and `HOST-154` (slice 14) give their foreign-handle
answers.

Two object kinds with their own `[[Call]]` and `[[Construct]]` arrive in wave 4 side by side:
the bound function exotic object (10.4.1, `story:harness-builtins`) and the ECMAScript function
object (10.2, `story:interpreter-and-script-entry-points`). So that neither edits
`object/mod.rs`, this story writes the internal-method dispatch with an arm for each of the two
kinds for `[[Call]]`, `[[Construct]]` and marking, calling functions in
`object/bound_function.rs` and `object/ecmascript_function.rs`. Here those functions have their
final signatures and the kind data is an empty struct with no constructor; the call bodies end
the run in `Terminated(EngineFault)` (no code can create an object of either kind before its
story lands, so reaching the arm means a broken invariant, and the fault is the defined answer
for that, never a wrong value), and the trace function reports nothing, which is right for the
same reason. The two later stories replace the struct, the call bodies and the trace function in
their own file. The marking follows the trace and root sites of
`story:heap-values-and-host-surface`: this story fills the ones that table gives to it.

## Requirements

Lines of `conformance/host/requirements.txt` moved from `pending slice 1` to `test`:

- `HOST-4` `swell::doc::compile_fail::host_4_hook_cx_cannot_call_construct_or_evaluate_a_script`
  (three `compile_fail` blocks: a call, a construct, an evaluation of a script)
- `HOST-5` `swell::doc::compile_fail::host_5_agent_call_inside_a_host_function`
- `HOST-12` `swell::realms::host_12_create_realm_with_zero_memory_is_memory_budget`
- `HOST-15` `swell::calls::host_15_reentry_depth_bounds_cx_call`
- `HOST-47` `swell::calls::host_47_call_and_construct_of_the_wrong_kind_throw_type_error`
- `HOST-51` `swell::realms::host_51_default_global_is_ordinary_with_object_prototype`
- `HOST-52` `swell::realms::host_52_dropped_realm_returns_heap_bytes`
- `HOST-106` `swell::calls::host_106_host_function_name_and_length`
- `HOST-124` `swell::hooks::host_124_*`: one test per provided item of the section 16 table but
  `host_get_import_meta_properties` and `host_finalize_import_meta` (their own requirement, slice 11), 17
  tests: `host_124_host_ensure_can_compile_strings_default`,
  `host_124_host_get_code_for_eval_default`, `host_124_host_ensure_can_add_private_element_default`,
  `host_124_host_has_source_text_available_default`, `host_124_host_make_job_callback_default`,
  `host_124_host_call_job_callback_default`, `host_124_host_enqueue_promise_job_default`,
  `host_124_host_prepare_to_run_job_default`, `host_124_host_clean_up_after_job_default`,
  `host_124_host_promise_rejection_tracker_default`,
  `host_124_supported_import_attributes_default` (the associated constant, `&[]`),
  `host_124_host_resize_array_buffer_default`, `host_124_host_grow_shared_array_buffer_default`,
  `host_124_system_time_zone_identifier_default`,
  `host_124_get_named_time_zone_offset_nanoseconds_default`,
  `host_124_get_named_time_zone_epoch_nanoseconds_default`,
  `host_124_available_named_time_zone_identifiers_default`. The rule: one per row of the table
  whose default is not `required`, but the two import-meta rows, the HTML row counting its two methods and the time zone row
  its four (section 17); the five required ones (`host_enqueue_generic_job`,
  `host_enqueue_timeout_job`, `host_enqueue_finalization_registry_cleanup_job`,
  `host_load_imported_module`, `host_system_utc_epoch_nanoseconds`) have no default to test.

## Additions after wave 2 of slice 1 (2026-11-21)

The design changed after this story was accepted (`docs/design/host-interface.md`, decision 5,
"A method that takes a handle returns `Result<_, HostError>`", and decision 6, "A handle the
host returns"; found by the unit of `story:heap-values-and-host-surface`, see
`docs/plan/slice-01-wave-2.md`). Where this story's text gives another result type, the
design governs.

- The methods of this story on `Cx` return `Result<Completion<T>, HostError>`: `new_error`,
  `new_host_function`, `call`, `construct`, `get`, `set`, `define_property_or_throw`,
  `has_property`, `delete_property_or_throw`, `to_primitive`, `to_number`, `to_numeric`,
  `to_string`, `to_object`, `to_property_key` (and `new_error` on `HookCx`). Each checks every
  handle among its arguments before anything else, before the sticky termination of decision
  10 too, and answers `Err(StaleHandle)` or `Err(ForeignHandle)` with nothing changed and the
  agent `Alive`.
- `Cx::new_host_object` and `HookCx::new_host_object`, landed in wave 2 with the result type
  `Completion<ObjectRef>`, change to `Result<Completion<ObjectRef>, HostError>` in this story:
  the prototype of the spec is checked first and a stale or foreign one answers `Err`, in
  place of the fault the code has now. The tests of `crates/swell/tests/host_objects.rs` and
  the support code follow the new type.
- A handle the host returns: the `[[Call]]` and `[[Construct]]` of a host function check every
  handle in its `Ok` value and in its `Abrupt::Throw` value before using it, and
  `InitializeHostDefinedRealm` does the same for the `GlobalObjects` of the setup closure. A
  stale or foreign handle there faults the agent (`Terminated(EngineFault)`). The requirement
  that holds this, `HOST-160`, is closed by `story:host-interface-hardening`; this story
  writes the check, with a unit test of its own for each of the two sites.
- The counts of Acceptance are measured on the base of the wave: with `HOST-160` the list has
  161 lines (`HOST-161` was added on the same day).
- `Writes:` is extended (2026-11-21, found by this story's unit before it wrote anything: with a
  required hook and the corrected `new_host_object` the crate does not compile unless these
  change too): `crates/swell/src/value.rs` (the field of `Realm`, so that `create_realm`,
  `global_object` and `realm_ref` can be written), `crates/swell/src/heap/mod.rs`, `arena.rs`,
  `collect.rs`, `map.rs`, `roots.rs`, `string.rs` (a root slot that holds a realm record; an
  arena for Symbols with its trace function, so that a collection that reaches a well-known
  symbol does not fault the agent; the `expect(dead_code)` marks of items this story now
  uses; the test hosts inside the crate, which gain the required hooks),
  `crates/swell/src/host/host_object.rs` (its test host), and
  `crates/swell/tests/host_objects.rs`, `collect.rs`, `interrupt.rs`, `budgets.rs` (callers of
  `new_host_object` and test hosts). In these files the unit changes what the list names and
  nothing else; the tests of `story:heap-values-and-host-surface` keep their names and what
  they assert.

- The current realm (2026-11-21, `docs/design/host-interface.md` decision 7, `HOST-161`): `with_cx`,
  `Agent::call` and `Agent::construct` take a `&Realm` that is current for the whole call;
  `current_realm` is the running activation's realm, else that one, and never depends on a
  collection or on creation order. This story closes `HOST-161` as its tenth requirement
  (27 `test` lines after it), with the collector run at every safe point too. `Writes:` gains
  `crates/swell/tests/values.rs` and the callers of the three entry points.

## Change

Failing tests first: the tests of Requirements and the files below under `crates/swell/tests/`
are written before the code and fail to build.

- `object/property.rs`, `object/ordinary.rs` (10.1, every ordinary internal method),
  `object/array.rs` (10.4.2, `ArrayCreate`, `ArraySetLength`), `object/string.rs` (10.4.3),
  `object/builtin_function.rs` (10.3, `CreateBuiltinFunction`, host functions of section 14 with
  `this`, arguments and `new_target`), `object/mod.rs` (dispatch of the internal methods, with
  the two arms of Context), `object/bound_function.rs` and `object/ecmascript_function.rs` (the
  empty kind data and the `[[Call]]` and `[[Construct]]` signatures, as Context says).
- `ops/conversion.rs` (7.1: `ToPrimitive`, `OrdinaryToPrimitive`, `ToBoolean`, `ToNumber`,
  `ToNumeric`, `ToIntegerOrInfinity`, `ToUint32`, `ToInt32`, `ToString`, `ToObject`,
  `ToPropertyKey`, `ToLength`, `CanonicalNumericIndexString`), `ops/testing.rs` (7.2:
  `IsCallable`, `IsConstructor`, `IsArray`, `SameValue`, `SameValueZero`, `IsLooselyEqual`,
  `IsStrictlyEqual`, `IsLessThan`), `ops/objects.rs` (7.3: `Get`, `Set`, `CreateDataProperty`,
  `DefinePropertyOrThrow`, `DeletePropertyOrThrow`, `HasProperty`, `HasOwnProperty`, `Call`,
  `Construct`, `LengthOfArrayLike`, `OrdinaryHasInstance`, `InstanceofOperator`,
  `EnumerateObjectProperties` of `sec-enumerate-object-properties`), `ops/operators.rs`
  (`sec-applystringornumericbinaryoperator`, the Number operations of 6.1.6.1, `typeof`
  results, string concatenation with the `string_length` bound: a result longer than
  `string_length` throws a `RangeError` before it is allocated).
- `realm/mod.rs` (`sec-createrealm`, `sec-initializehostdefinedrealm` with the setup closure of
  section 7 and the `this` value it may return), `realm/intrinsics.rs` (`%Object.prototype%`,
  `%Function.prototype%`, `%Array.prototype%`, `%String.prototype%`, `%Boolean.prototype%`,
  `%Number.prototype%`, `%ThrowTypeError%`, the Error intrinsics; the well-known symbols
  `@@hasInstance`, `@@toPrimitive`, `@@toStringTag`, `@@species` as agent values with no
  `Symbol` global, which is slice 6), `realm/global_object.rs` (`SetDefaultGlobalBindings` for
  the properties that exist).
- `builtins/global.rs` (19.1: `Infinity`, `NaN`, `undefined`), `builtins/error.rs`
  (20.5: `Error`, `Error.prototype.message`, `.name`, `.toString`, `InstallErrorCause`, and the
  six NativeError constructors and prototypes of `sec-native-error-types-used-in-this-standard`),
  `builtins/boolean.rs` and `builtins/number.rs` (the prototype objects with their
  `[[BooleanData]]` and `[[NumberData]]` only).
- `host/hooks.rs`: every hook of section 16, provided methods with the defaults of its table (the
  time zone defaults use `swell_num::date::get_utc_epoch_nanoseconds`), required methods without
  a body, `Job` and `LoadRequest`; `host/mod.rs` and `host/agent.rs`: `create_realm`,
  `global_object`, `realm_host_defined`, `new_host_function`, `new_error`, the `Cx` operations
  `call`, `construct`, `get`, `set`, `define_property_or_throw`, `has_property`,
  `delete_property_or_throw`, `to_primitive`, `to_number`, `to_numeric`, `to_string`,
  `to_object`, `to_property_key`, and `Agent::call`, `Agent::construct`; the entry bookkeeping:
  the outermost entry, its step counter with one function that charges n steps and checks the
  counter and the interrupt flag (ending the run in `Terminated(StepBudget)` or
  `Terminated(Interrupted)`), which the interpreter and every builtin loop call, and the
  re-entry count of `reentry_depth` on every call from Rust into a function.
- The trace and root sites that `story:heap-values-and-host-surface` created for this story, each
  in a file this story already writes: in `object/mod.rs` the property values and accessors of
  the common record and one marking arm per object kind, beside the call arms, calling the
  trace function in the kind file (`object/ordinary.rs` with the Boolean and Number wrappers,
  `object/array.rs`, `object/string.rs` with `[[StringData]]`, `object/builtin_function.rs` with
  `[[Realm]]` and `[[InitialName]]`; in `object/bound_function.rs` and
  `object/ecmascript_function.rs` a trace function that reports nothing, as Context says); in
  `realm/mod.rs` the trace of the realm record (every intrinsic, the global object, the global
  `this`, and the `RealmGlobalEnv` field, whose trace and constructor live in `env/global.rs` and
  report nothing and hold no environment until `story:interpreter-and-script-entry-points` fills
  them) and the root site for the agent-wide well-known symbols; in
  `object/builtin_function.rs` the root site for the slots of running builtins: a builtin that
  keeps a value across a call into code puts it in a slot of its activation there, never in a
  Rust local (decision 4). `Cx::call` and every other re-entry is a safe point, where the
  feature `collect-every-safe-point` collects. This story writes neither `heap/collect.rs` nor
  `env/global.rs`.
- `crates/swell/tests/support/host.rs`: the test host implements the required methods.
- Tests besides those of Requirements, each citing its clause: `objects.rs`
  (`array_length_truncates_and_refuses_non_writable`, `string_exotic_indices_are_read_only`,
  `define_own_property_validates_every_case_of_10_1_6_3`, `own_property_keys_order`),
  `conversion.rs` (`to_number_of_strings` with every `Zs` code point as padding,
  `to_string_of_numbers`, `to_primitive_calls_valueof_and_tostring_in_hint_order`,
  `to_object_of_primitives`), `operators.rs` (`loose_equality_table`, `addition_and_relational`,
  `concatenation_past_string_length_is_range_error`), `errors.rs`
  (`error_constructors_set_message_and_cause`, `native_errors_inherit_from_error`,
  `error_prototype_to_string`), `realms.rs` (`global_value_properties`,
  `instanceof_uses_ordinary_has_instance`,
  `intrinsic_reachable_only_through_the_realm_survives_collect`: with only a `Realm` root kept, a
  collection, then `%Object.prototype%` (the global object prototype) and `%Error.prototype%` and
  `%RangeError.prototype%` (read through the global `Error` and `RangeError`) of the realm global
  object are the same objects with their properties). In `objects.rs` also
  `string_data_and_property_values_survive_collect` (a String wrapper and an object whose only
  reference to a value is a data property or an accessor function, rooted only through the
  object, still read the same string and value after a collection). Both fail when the trace
  functions of this story report nothing.

`Cargo.lock`: the unit regenerates it in its own tree (the new path dependency on
`swell-unicode`), because its build needs it, and runs its acceptance with `--locked` against
that lock; the coordinator does not take the unit lock into the wave branch but regenerates it
there after the merges of the wave.

Writes: `crates/swell/Cargo.toml` (dependency on `swell-unicode`), `crates/swell/src/object/{mod,
property,ordinary,array,string,builtin_function,bound_function,ecmascript_function}.rs`,
`crates/swell/src/ops/{mod,conversion,testing,objects,operators}.rs`,
`crates/swell/src/realm/{mod,intrinsics,global_object}.rs`,
`crates/swell/src/builtins/{mod,global,error,boolean,number}.rs`,
`crates/swell/src/host/{mod,agent,hooks}.rs`, `crates/swell/src/compile_fail.rs`,
`crates/swell/tests/{objects,conversion,operators,errors,realms,calls,hooks}.rs`,
`crates/swell/tests/support/host.rs`, `conformance/host/requirements.txt` (the nine lines),
`Cargo.lock` (regenerated in the unit tree; the wave branch lock is regenerated by the
coordinator).

## Acceptance

`cargo test --locked -p swell` and `cargo test --locked -p swell --features
collect-every-safe-point --test objects --test realms --test calls` exit 0 and run the tests of
Requirements and the tests named in Change (`intrinsic_reachable_only_through_the_realm_survives_collect`
and `string_data_and_property_values_survive_collect` among them);
`cargo test --locked -p swell --test hooks -- --list` prints 17 lines starting
`host_124_`, the 17 names of Requirements; and `slot task -d . -t taskfiles/host-requirements.yml
host-requirements-check` exits 0 and prints `host-requirements: 161 requirement(s), 161
line(s): 27 test, 134 pending, 0 omitted; 50 UNMAPPED marker(s), each naming a HOST-<n> or ESS
limit` and `host-requirements: pending by slice: 1:43 2:5 3:5 4:9 5:2 6:1 7:1 8:1 9:5 10:14
11:32 12:2 13:2 14:6 16:5 18:1`.

## Out of scope

Running code: `parse_script`, `evaluate_script`, `Cx::evaluate_script`, the behaviour of
ECMAScript function objects, environments, the places the interpreter charges steps and checks
them (`story:interpreter-and-script-entry-points`); the constructors `Object`, `Array`,
`String`, the bound function behaviour, the `Function.prototype` methods and the other builtins
of the harness files (`story:harness-builtins`); `globalThis` (slice 3, `HOST-155`); `Boolean`,
`Number` and their methods (slices 3 and 4); `Symbol` (slice 6); `Overrides` (slice 9); jobs and
modules beyond their types (slices 10 and 11). The root `Cargo.toml` and `Taskfile.yml` (the
coordinator).

## Domain relations

- Agent to Realm, one-to-many, the agent owns its realms: inferable (`spec/domains/host.yaml`,
  entity `swell.host.Agent`, relation `realms`, `owns`, `many`, via `agent_id`). `create_realm`
  adds one; `HOST-52` holds that a realm no root reaches is freed.
- Realm to global object and global `this`, one each, fixed at creation: inferable
  (`docs/design/host-interface.md`, section 7, the setup closure; `sec-initializehostdefinedrealm`).
- Realm lifetime, held by `Realm` roots and by objects of the realm: inferable
  (`docs/design/host-interface.md`, section 7 and `HOST-52`).
