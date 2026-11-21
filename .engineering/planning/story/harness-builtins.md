---
format: aep.planning-md/3
id: story:harness-builtins
kind: story
status: active
title: The builtins the harness files call, complete for every argument
summary: Object, Function.prototype, Array, String, Math and JSON members the epic lists and the measurement adds, the bound function object, registered in the realm; tested from Rust beside the interpreter.
relations:
- decomposes: epic:slice-01-harness
- depends_on: story:object-model-and-realms
scope:
- confidence: cited
  path: crates/swell/src/builtins
- confidence: cited
  path: crates/swell/src/object/bound_function.rs
- confidence: cited
  path: crates/swell/src/object/builtin_function.rs
- confidence: cited
  path: crates/swell/src/object/mod.rs
- confidence: cited
  path: crates/swell/src/ops/objects.rs
- confidence: cited
  path: crates/swell/src/ops/testing.rs
- confidence: cited
  path: crates/swell/src/realm/global_object.rs
- confidence: cited
  path: crates/swell/src/realm/intrinsics.rs
revision: 10
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T04:49:59Z", actor: "human:operator", revision: 5, decided_on: {"recorded":{"review_outcome":2}}, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-21T04:49:59Z", actor: "human:operator", revision: 6, decided_on: {"recorded":{"review_outcome":2}}, executor: "agent:coordinator"}
---
## Context

The builtins the harness files call (`docs/design/engine.md`, section 12, item 1; the epic,
Outcome, which lists `Function.prototype.call` and `.bind`, `Object.getPrototypeOf`,
`.getOwnPropertyDescriptor`, `.getOwnPropertyNames`, `.defineProperty`,
`Object.prototype.hasOwnProperty`, `.propertyIsEnumerable`, `.toString`, `Array.prototype.join`,
`.push`, `.map`, `Array.isArray`, `Math.max` and `JSON.stringify`). The epic leaves open how much
of `Object`, `Array`, `Function` and `JSON` slice 1 takes; the measurement settles it.

Measured on the 31 cases that stay in slice 1 and the harness files they include, with comments
removed and string contents blanked: they call `Object.prototype.toString.call`,
`Array.prototype.map.call(arrayLike, String).join(...)` (`compareArray.js`), `JSON.stringify`
(`assert.js`, `assert._toString`, when `JSON` exists), `String(...)` as a function,
`String.prototype.indexOf` (`assert-samevalue-zeros`), `instanceof` and the Error constructors
(`story:object-model-and-realms`). The other methods of the epic list are reached by
`propertyHelper.js`, whose cases move to slice 3 (`story:test262-run-path`); they stay in this
story because the epic asks for them, and are tested here from Rust.

What the list needs besides itself, each from the algorithm of the listed method: `Object`,
`Array` and `String` constructors (`Array.prototype.map` reaches `ArraySpeciesCreate`, which
reads `constructor` and `Array[@@species]`; `String` is called as a function by
`compareArray.js`); `Function.prototype[@@hasInstance]` (`InstanceofOperator` reads it first,
`sec-instanceofoperator`); the bound function exotic object for `bind` (10.4.1); `Math` and
`JSON` as objects with their `@@toStringTag`.

Every method that exists is complete for every argument (design, "A part before its slice"):
`JSON.stringify` with replacer function and array, `space`, `toJSON`, cycles, and lone
surrogates (`sec-quotejsonstring`); `bind` with its `name` and `length`;
`Object.defineProperty` with every descriptor case. Loops over elements charge steps through
the step counter of `story:object-model-and-realms`; `JSON.stringify` walks nested values with
an explicit stack, not Rust recursion, and each `toJSON` or replacer call is a re-entry bounded
by `reentry_depth` (decision 3, "Depth bounds").

`Array.prototype.join` and `JSON.stringify` build a String at run time from other values, so
each checks `string_length` (`docs/design/host-interface.md`, section 2, the limits table): a
result that would pass it throws a `RangeError` before it is allocated, as the concatenation of
`story:object-model-and-realms` does, and each has a test that reaches the bound.

This story runs no script: it tests through `with_cx`, `Agent::call`, and host functions as the
callbacks of `map`, `bind`, `call`, `toJSON` and the replacer, so it runs beside
`story:interpreter-and-script-entry-points`. The same builtins with ECMAScript functions as
callbacks are tested by `story:host-interface-hardening`, which depends on both stories; the
scripts of the 31 cases reach them in `story:test262-run-path`.

## Additions after wave 2 of slice 1 (2026-11-21)

The design changed after this story was accepted (`docs/design/host-interface.md`, decision 5,
"A method that takes a handle returns `Result<_, HostError>`"; see
`docs/plan/slice-01-wave-2.md`). No method this story writes changes. Its tests call the
engine through `cx`, and those calls now return `Result<Completion<T>, HostError>`: a test
reads the outer `Result` first.

- The current realm (2026-11-21, `docs/design/host-interface.md` decision 7): `with_cx`,
  `Agent::call` and `Agent::construct` take a `&Realm`; this story's tests pass one.

- `Writes:` is extended (2026-11-21, found by this story's unit before it wrote anything: a bound
  function cannot be created under `-D warnings` while three `expect(dead_code)` marks name
  this story, and two operations lack their bound-function step):
  `crates/swell/src/object/mod.rs` (the mark on the bound-function kind, nothing else; the
  sentence of this story that the file is not edited is withdrawn for that one line),
  `crates/swell/src/ops/testing.rs` (the mark on `is_array`),
  `crates/swell/src/ops/objects.rs` (the marks on `has_own_property` and
  `length_of_array_like`; step 2 of GetFunctionRealm and of OrdinaryHasInstance for a bound
  function, each as a loop that charges one step per bound function, never by recursion in
  Rust), and `crates/swell/src/object/builtin_function.rs` (six `BuiltinId` variants, one per
  builtin module of this story, each module holding its own id enum, so that an engine builtin
  does not go through the host-function path).
- The `[[Call]]` and `[[Construct]]` of a bound function gather the bound arguments in a loop
  over the chain, one step charged per link, and do not recurse in Rust.
- No public method returns a well-known symbol, and none is added in this slice. The two tests
  that need one (`has_instance_of_a_bound_function`, and the custom `@@species` part of
  `map_uses_species_and_skips_holes`) are unit tests inside `builtins/function.rs` and
  `builtins/array.rs`; the integration test of `map` keeps what the interface reaches (the
  `%Array%` of another realm, a constructor without `@@species`, a non-constructor that throws
  a `TypeError`, holes). The gate runs them in both collection modes (`test` and
  `test-collect`).
- `story:interpreter-and-script-entry-points` runs at the same time and will change
  `ops/objects.rs` (GetFunctionRealm for an ECMAScript function, `is_constructor`,
  `instanceof_operator`) and `object/mod.rs` (its own mark). This story touches only its own
  hunks there; the unit that lands second takes the other one's in.

## Change

Failing tests first: the test files below are written before the code and fail to build.

- `builtins/object.rs` (20.1): the `Object` constructor (`sec-object-value`),
  `Object.getPrototypeOf`, `Object.getOwnPropertyDescriptor` (`sec-frompropertydescriptor`),
  `Object.getOwnPropertyNames`, `Object.defineProperty` (`sec-topropertydescriptor`),
  `Object.prototype.constructor`, `.hasOwnProperty`, `.propertyIsEnumerable`, `.toString`
  (`sec-object.prototype.tostring`, with `@@toStringTag`).
- `builtins/function.rs` (20.2): `Function.prototype.call`, `.bind`,
  `Function.prototype[@@hasInstance]`; `object/bound_function.rs` (10.4.1,
  `sec-boundfunctioncreate`): the kind data and the bodies of `[[Call]]` and `[[Construct]]` that
  `story:object-model-and-realms` left as an unreachable `Terminated(EngineFault)` behind its
  dispatch arm; and the trace function of the kind, which reports the target, the bound `this`
  and every bound argument (the trace site `story:heap-values-and-host-surface` lists for this
  file). `object/mod.rs` and `heap/collect.rs` are not edited. Not the global `Function`, which
  needs `CreateDynamicFunction` (slice 3).
- `builtins/array.rs` (23.1): the `Array` constructor (`sec-array-constructor-array`),
  `Array.isArray`, `Array[@@species]`, `Array.prototype.constructor`, `.join`, `.push`, `.map`
  (with `ArraySpeciesCreate`).
- `builtins/string.rs` (22.1): the `String` constructor (`sec-string-constructor-string-value`),
  `String.prototype.constructor`, `.indexOf`.
- `builtins/math.rs` (21.3): the `Math` object, `Math.max`, `Math[@@toStringTag]`.
- `builtins/json.rs` (25.5): the `JSON` object, `JSON.stringify` (`sec-serializejsonproperty`,
  `sec-quotejsonstring`), `JSON[@@toStringTag]`.
- `realm/intrinsics.rs` and `realm/global_object.rs`: the new intrinsics and the global
  properties `Object`, `Array`, `String`, `Math`, `JSON`.
- Values a builtin keeps across a call into code (the array `map` builds, the holder and the
  stack of `JSON.stringify`, the arguments `call` passes on) are kept in the slots of its
  activation, the root site of `object/builtin_function.rs` (`story:object-model-and-realms`),
  never in a Rust local (decision 4).
- Tests, each citing its clause, in `crates/swell/tests/`:
  `builtins_object.rs` (`define_property_every_descriptor_case`,
  `get_own_property_descriptor_round_trips`, `own_property_names_order`,
  `to_string_tags_of_each_kind`, `has_own_and_property_is_enumerable`),
  `builtins_function.rs` (`call_passes_this_and_arguments`,
  `bind_name_length_and_construct`, `has_instance_of_a_bound_function`,
  `bound_target_this_and_arguments_survive_collect`),
  `builtins_array.rs` (`join_of_holes_and_nullish`, `push_past_2_53_minus_1_throws`,
  `map_uses_species_and_skips_holes`, `map_result_survives_a_collection_at_each_callback`,
  `join_of_length_2_32_minus_1_ends_in_step_budget`,
  `join_past_string_length_throws_range_error`),
  `builtins_string.rs` (`string_called_and_constructed`, `index_of_positions`),
  `builtins_math.rs` (`max_of_nan_and_signed_zeros`), `builtins_json.rs`
  (`stringify_values_and_nesting`, `stringify_replacer_and_space`,
  `stringify_cycle_throws_type_error`, `stringify_lone_surrogates_are_escaped`,
  `stringify_of_100000_nested_arrays_needs_no_rust_stack`,
  `stringify_past_string_length_throws_range_error`).
  `bound_target_this_and_arguments_survive_collect` (a bound function rooted alone, a
  collection, then a call shows the target receiving the bound `this` and arguments) and
  `map_result_survives_a_collection_at_each_callback` (under `collect-every-safe-point` every
  callback call is a re-entry and collects; the result array reads all its elements) fail when
  the trace function of the kind or the activation slots report nothing.

No crate and no dependency is added, so `Cargo.lock` does not change.

Writes: `crates/swell/src/builtins/{object,function,array,string,math,json}.rs`,
`crates/swell/src/object/bound_function.rs`, `crates/swell/src/realm/intrinsics.rs`,
`crates/swell/src/realm/global_object.rs`,
`crates/swell/tests/{builtins_object,builtins_function,builtins_array,builtins_string,
builtins_math,builtins_json}.rs`.

## Acceptance

`cargo test --locked -p swell --test builtins_object --test builtins_function --test
builtins_array --test builtins_string --test builtins_math --test builtins_json` and `cargo test
--locked -p swell --features collect-every-safe-point --test builtins_function --test
builtins_array --test builtins_json` exit 0 and run the twenty-four tests named in Change.

## Out of scope

Every other method of these objects, the global `Function`, `eval`, `Error.isError`,
`Function.prototype.toString`, `Boolean`, `Symbol` (slices 3, 4 and 6); `$262` and `print`
(`story:test262-run-path`); running these builtins from script (`story:test262-run-path` and
`story:host-interface-hardening`).
