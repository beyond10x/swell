---
format: aep.planning-md/3
id: story:builtin-group-modules
kind: story
status: implemented
title: The builtins of Array, Object and String in group modules of their own, the operations and stack-test helpers the next wave shares; no result changes
summary: 'A restructuring before the wave of ten: one file per group of builtins, with no line of mod.rs left to write; SetIntegrityLevel, TestIntegrityLevel, Invoke, EnumerableOwnProperties, CreateDataPropertyOrThrow, CodePointAt and UTF16EncodeCodePoint delivered with their tests; the first steps of the String methods opened, @@isConcatSpreadable and @@match made, the helpers of the stack tests in tests/support; every run line stays as it is.'
relations:
- decomposes: epic:slice-03-builtins-core
scope:
- confidence: cited
  path: crates/swell/src/builtins/array.rs
- confidence: cited
  path: crates/swell/src/builtins/array/iteration.rs
- confidence: cited
  path: crates/swell/src/builtins/array/mod.rs
- confidence: cited
  path: crates/swell/src/builtins/array/mutation.rs
- confidence: cited
  path: crates/swell/src/builtins/array/search.rs
- confidence: cited
  path: crates/swell/src/builtins/json.rs
- confidence: cited
  path: crates/swell/src/builtins/mod.rs
- confidence: cited
  path: crates/swell/src/builtins/object.rs
- confidence: cited
  path: crates/swell/src/builtins/object/enumeration.rs
- confidence: cited
  path: crates/swell/src/builtins/object/integrity.rs
- confidence: cited
  path: crates/swell/src/builtins/object/mod.rs
- confidence: cited
  path: crates/swell/src/builtins/string.rs
- confidence: cited
  path: crates/swell/src/builtins/string/access.rs
- confidence: cited
  path: crates/swell/src/builtins/string/mod.rs
- confidence: cited
  path: crates/swell/src/builtins/string/transform.rs
- confidence: cited
  path: crates/swell/src/interp/dispatch.rs
- confidence: cited
  path: crates/swell/src/object/mod.rs
- confidence: cited
  path: crates/swell/src/ops/objects.rs
- confidence: cited
  path: crates/swell/src/realm/mod.rs
- confidence: cited
  path: crates/swell/tests/builtins_array.rs
- confidence: cited
  path: crates/swell/tests/builtins_string.rs
- confidence: cited
  path: crates/swell/tests/limits_scripts.rs
- confidence: cited
  path: crates/swell/tests/support/mod.rs
- confidence: cited
  path: crates/swell/tests/support/stack.rs
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T21:30:43Z", actor: "human:operator", revision: 7, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-21T21:30:44Z", actor: "human:operator", revision: 8, executor: "agent:coordinator"}
- {from: "active", to: "implemented", at: "2026-11-22T11:09:51Z", actor: "human:operator", revision: 9, decided_on: {"recorded":{"test_result":1,"verification":1}}, executor: "agent:coordinator"}
---
## Context

Slice 3 (`epic:slice-03-builtins-core`) delivers about ninety builtins in one wave of ten stories side by side. Today a builtin is registered in its constructor's own module and nowhere else: a variant of the module's id type (`ArrayBuiltin` in `crates/swell/src/builtins/array.rs`), one tuple in the list of `create_array_constructor`, one arm of `run_array_builtin`; `BuiltinId` (`crates/swell/src/object/builtin_function.rs`, line 35) has one variant per module and `create_intrinsics` (`crates/swell/src/realm/intrinsics.rs`) one call per module. That is enough for `Function`, `JSON`, `Error` and the global functions, each of which one story of the next wave writes. It is not enough for three constructors: `Array` lacks 34 of the 39 named properties the pinned draft lists for its prototype and 3 of the 5 of its constructor, `Object` 19 of the 24 of its constructor and 3 of the 7 of its prototype outside Annex B, `String` 3 of 4 and 31 of 35 (measured on the base `309b850` with a probe case run by the built runner, which reads `Object.getOwnPropertyDescriptor` of every name the clause lists and reports what each object has and lacks). Three stories write the methods of `Array`, two those of `Object`, two those of `String`, and two stories of one wave write different files (`AGENTS.md`, "How the work is run"). This story restructures before that wave, opens what two stories of it would otherwise each write, and changes no result.

What the stories of the next wave would otherwise each have to open or would write twice, found by reading every clause the ten stories cite against `crates/swell/src/ops`, `object`, `builtins` and `interp` on the base:

- SetIntegrityLevel and TestIntegrityLevel (`sec-setintegritylevel`, `sec-testintegritylevel`) do not exist. `Object.freeze`, `seal`, `isFrozen` and `isSealed` call them (`story:object-integrity-and-prototype-methods`), `JSON.rawJSON` returns a frozen object (`story:json-parse-and-raw-json`), and a test of a third story needs a frozen array. [[PreventExtensions]] exists (`prevent_extensions`, `crates/swell/src/object/mod.rs`, line 284) under `expect(dead_code, reason = "Object.preventExtensions: story:harness-builtins")`; SetIntegrityLevel is its first caller.
- Invoke (`sec-invoke`) does not exist. `Object.prototype.toLocaleString` and `Array.prototype.toLocaleString` call it (`story:object-integrity-and-prototype-methods`, `story:array-search-and-copy-methods`).
- EnumerableOwnProperties (`sec-enumerableownproperties`) exists for the kind key only, as a private function of `builtins/json.rs` (`json_enumerable_own_keys`, line 545). `Object.keys`, `values` and `entries` need its three kinds (`story:object-enumeration-and-copy-methods`) and the reviver of `JSON.parse` the kind key (`story:json-parse-and-raw-json`).
- CodePointAt and UTF16EncodeCodePoint (`sec-codepointat`, `sec-utf16encodecodepoint`) do not exist in `swell` (the lexer of `crates/syntax` has its own). Encode and Decode of the URI functions call them (`story:global-this-uri-functions-and-is-error`), and so do `codePointAt`, `fromCodePoint`, `isWellFormed` and `toWellFormed` (`story:string-access-and-search-methods`).
- CreateDataPropertyOrThrow is a private function of the interpreter (`crates/swell/src/interp/dispatch.rs`, line 1252); `Array.prototype.map` works around it with `create_data_property` and a throw of its own (`builtins/array.rs`, step 6.c.iii). About twenty builtins of the next wave name the operation.
- The first two steps of every method of `String.prototype` but `toString` and `valueOf` (RequireObjectCoercible of the this value, then ToString) are written inside `string_index_of_method` (`builtins/string.rs`, line 227); 27 methods of two stories need them.
- The stack tests through builtins keep their helpers private in `crates/swell/tests/limits_scripts.rs` (`on_bounded_stack`, `stack_limits`, `stack_agent`, `deepest`, `reach_through`, `FULL_REENTRY`, the host functions `reenter` and `parse_deep`, and the list `THROUGH_BUILTINS`). Every story of the next wave that delivers a builtin which calls back into script owes a test at the re-entry limit on the promised stack (`docs/plan/unit-invariants.md`; the epic, acceptance 7); ten stories adding to one list in one file is the overlap this slice must not have.
- Two builtins of the next wave read a well-known symbol the realm does not have yet: IsConcatSpreadable reads `@@isConcatSpreadable` (`sec-isconcatspreadable`, for `Array.prototype.concat`) and IsRegExp reads `@@match` (`sec-isregexp`, for `includes`, `startsWith` and `endsWith` of `String.prototype`). `WellKnown` (`crates/swell/src/realm/mod.rs`, line 32) has five symbols; slice 2 added `@@unscopables` the same way before `Symbol` exists. No script can obtain either symbol before slice 6, so nothing is observable; the Get is in place when slice 6 exposes them.

Every other operation the ten stories cite either exists on the base as `pub(crate)` (Get, GetV, GetMethod, Set, CreateDataProperty, DefinePropertyOrThrow, DeletePropertyOrThrow, HasProperty, HasOwnProperty, Call, Construct, LengthOfArrayLike, the conversions, IsCallable, IsConstructor, IsArray, SameValue, SameValueZero, IsStrictlyEqual, IsLessThan, ArrayCreate, ArraySpeciesCreate, OrdinaryObjectCreate, CreateArrayFromList, ToPropertyDescriptor, FromPropertyDescriptor, StringIndexOf, the `StringBuilder`) or is called by one story alone, which writes it in its own file (ObjectDefineProperties, CreateListFromArrayLike, Encode, Decode, ParseHexOctet, the operations of `JSON.parse`, FindViaPredicate, FlattenIntoArray, IsConcatSpreadable, SortIndexedProperties, CompareArrayElements, ToUint16, StringLastIndexOf, IsRegExp, IsStringWellFormedUnicode, StringPad, TrimString).

## Domain relations

- Realm → well-known symbol, one-to-many, every realm record holds the symbols its code sees — inferable (inferred from `crates/swell/src/realm/mod.rs:66`, the field `symbols` of `RealmRecord`; no ess/1 document declares this relation).

## Change

Failing check first: the unit tests below fail to compile before the modules and the operations exist.

- `crates/swell/src/builtins/array.rs` becomes `crates/swell/src/builtins/array/mod.rs` (moved with its history; the constructor, `isArray`, `[@@species]`, `join`, `map`, `push`, `toString`, ArraySpeciesCreate, `index_key`, their tests), and gains three group modules, each with the id type of its builtins, the list that installs them on the constructor and the prototype, and a function that runs one: `array/iteration.rs` (`story:array-iteration-methods`), `array/search.rs` (`story:array-search-and-copy-methods`), `array/mutation.rs` (`story:array-mutation-and-sort-methods`). `ArrayBuiltin` gets one variant per group, `create_array_constructor` reads each group's list after its own, `run_array_builtin` has one arm per group, so that a builtin of a group is one variant, one entry of the list and one arm in the group's own file. A group has no builtin yet: its id type is an enum without a variant, its list is empty and its run function a `match` without an arm, and nothing else stands in for a builtin. `mod.rs` makes its ids from the group's list and calls the group's function, so nothing in `mod.rs` waits for a builtin: `mod.rs` carries no `expect(dead_code)` for a group, and a story of the next wave writes its group file and no line of `mod.rs`. A mark the compiler still asks for stands in the group's own file, with the reason `story:<the story named above>`. The run function of a group is a function of its own that is not inlined into `run_array_builtin` (`#[inline(never)]`), so that the frame of the shared dispatch does not grow with the arms the next wave adds. `MAX_SAFE_LENGTH` and `ARRAY_INDEX_END` become `pub(crate)`.
- `crates/swell/src/builtins/object.rs` becomes `builtins/object/mod.rs` with `object/integrity.rs` (`story:object-integrity-and-prototype-methods`) and `object/enumeration.rs` (`story:object-enumeration-and-copy-methods`), in the same form. The match of `builtin_tag` over the slots of an ordinary object (`sec-object.prototype.tostring`, steps 8 to 14) ends in the arm of step 14 ("Else, let builtinTag be "Object"") for every slot it does not name, so that the slot `story:json-parse-and-raw-json` adds needs no line there.
- `crates/swell/src/builtins/string.rs` becomes `builtins/string/mod.rs` (the constructor, `indexOf`, `toString`, `valueOf`, `StringBuilder` and its functions) with `string/access.rs` (`story:string-access-and-search-methods`) and `string/transform.rs` (`story:string-transform-and-compare-methods`), in the same form; the two steps named in Context become one `pub(crate)` function of `string/mod.rs` that `indexOf` calls. Beside `units_of` it gains CodePointAt (`sec-codepointat`: the code point at a position of a sequence of code units, the number of units it takes and whether it is an unpaired surrogate) and UTF16EncodeCodePoint (`sec-utf16encodecodepoint`), both functions of code units that touch no heap.
- The order of keys, so that ten units agree without meeting. On each object the keys the base defines stay first, in the order the base has them. The keys of a group follow, the groups in the order `iteration`, `search`, `mutation` on `Array` and `Array.prototype`, `integrity`, `enumeration` on `Object` and `Object.prototype`, `access`, `transform` on `String` and `String.prototype`; inside a group the order is that of the clauses of the pinned draft. A story of the next wave that adds keys to an object without groups (`Function.prototype`, `JSON`, `Error`, the global object) defines them after the keys of the base, in the order of the clauses.
- `crates/swell/src/builtins/mod.rs`: its module comment (the modules are found as before: `crate::builtins::array::ArrayBuiltin` and the two others keep their paths, so `object/builtin_function.rs` and `dynamic_code.rs` do not change).
- `crates/swell/src/ops/objects.rs`, each operation complete for every object the base has (ordinary, Array, String and arguments exotic objects, functions), as a `pub(crate)` method with its steps cited:
  - CreateDataPropertyOrThrow (`sec-createdatapropertyorthrow`), moved from `crates/swell/src/interp/dispatch.rs`, which calls it there; `Array.prototype.map` uses it.
  - SetIntegrityLevel (`sec-setintegritylevel`, levels sealed and frozen: [[PreventExtensions]], then every own key by DefinePropertyOrThrow, an accessor keeping its functions) and TestIntegrityLevel (`sec-testintegritylevel`). Each charges a step per key; the list of keys is charged to the memory budget and kept on the kept stack.
  - Invoke (`sec-invoke`: GetV, then Call with the value as the this value, a primitive included).
  - EnumerableOwnProperties (`sec-enumerableownproperties`) with its three kinds, moved from `crates/swell/src/builtins/json.rs`, whose serialization calls it with the kind key as before: a key is read again by [[GetOwnProperty]] when its turn comes, so a property a getter deleted meanwhile is skipped; a step per key; for the kinds value and key+value the Get of each value, the keys and the results kept across it.
- The marks of what the next wave calls first. An item with no caller outside the tests carries `cfg_attr(not(test), expect(dead_code, reason = "..."))`, and the reason, in the form of the base (`<what>: story:<name>`, with `, story:<name>` for a second), names every story of the next wave that calls it: SetIntegrityLevel `story:object-integrity-and-prototype-methods, story:json-parse-and-raw-json`; TestIntegrityLevel `story:object-integrity-and-prototype-methods`; Invoke `story:object-integrity-and-prototype-methods, story:array-search-and-copy-methods`; the kinds value and key+value of EnumerableOwnProperties `story:object-enumeration-and-copy-methods`; CodePointAt and UTF16EncodeCodePoint `story:global-this-uri-functions-and-is-error, story:string-access-and-search-methods`. A story removes the mark that names it when it calls the item and changes nothing else in that file (`AGENTS.md`, "How the work is run": "a story may always remove a mark whose reason names it, in any file"). Two stories that remove one mark delete the same lines, and a merge takes the same change from both sides without a conflict.
- `crates/swell/src/object/mod.rs`: the mark of `prevent_extensions` (the attribute, lines 280 to 283) is removed, since SetIntegrityLevel calls it; nothing else changes in that file, which `story:property-index-and-dense-elements` writes in the same wave and which leaves those four lines alone.
- `crates/swell/src/realm/mod.rs`: `@@isConcatSpreadable` and `@@match` among the well-known symbols (`sec-well-known-symbols`, descriptions `Symbol.isConcatSpreadable` and `Symbol.match`), made and traced as the five others. `WellKnown::all` reads every field for the tracer, so no field needs a mark and no story of the next wave writes this file.
- `crates/swell/tests/support/stack.rs` (new) and `crates/swell/tests/support/mod.rs` (`pub mod stack;`): the helpers of the stack tests named in Context, moved from `crates/swell/tests/limits_scripts.rs`, with `reach_through(through, reentry_depth)` taking the expression that re-enters. `limits_scripts.rs` uses them; the names of its tests do not change (`conformance/host/requirements.txt` names five of them) and `THROUGH_BUILTINS` keeps its four entries. `budgets.rs` and `eval_stack.rs` keep their own copies.
- `crates/swell/tests/builtins_array.rs` and `crates/swell/tests/builtins_string.rs`: the file names in their module comments.
- A group installs a data property keyed by a well-known symbol from its own file. Beside its list each group file has a function for the properties of the group that are not built-in functions (`Array.prototype[@@unscopables]` of `story:array-search-and-copy-methods` is the one the next wave has); the function that makes the constructor calls it after the lists, the groups in the order given above, with the constructor, the prototype and the well-known symbols of the realm. It defines nothing in this story. A key it defines comes after the Symbol keys of the base. Test, in `builtins/array/mod.rs`: `a_group_installs_a_property_keyed_by_a_well_known_symbol` (a data property defined the way the function of a group defines one, with `@@unscopables` as its key, is an own property with the value and the attributes given and comes after every String key among the own keys; with the functions of the seven groups as this story leaves them, no object of a new realm has a Symbol key the base lacks).
- Time of the tests ("Tests stay fast", the `AGENTS.md` of the folder that holds the repository, and the `AGENTS.md` of the repository once `story:gate-test-steps-within-budget` has landed): the unit times each test it adds before it adds it; none runs over 5 s without a reason given in the report, and none over 60 s; no `OnceLock` is shared between test functions (the test steps run a process per test, so it would be built once per test); the unit does not run the whole gate, which runs once on the wave branch.
- Tests, each in both collection modes (`test-collect`), all inside the crate, where they call the operations without a builtin:
  - in `builtins/array/mod.rs`, `every_group_module_installs_and_runs_through_its_constructor` pins what stays, not what is there: on a new realm the own String keys of `Array` and of `Array.prototype` are the keys of the base, written out in the test in the order of the base, followed by the keys of the three group lists in the order given above, and nothing else; the Symbol keys of the base are still own keys. The lists of the groups are empty in this story and grow in the next wave, and the test stays green in each of its trees and on the wave branch. The same test in `object/mod.rs` and `string/mod.rs`.
  - in `ops/objects.rs`: `set_integrity_level_seals_and_freezes_an_object_an_array_and_arguments` (after it [[IsExtensible]] is false and every own property has the attributes of the level; an accessor keeps its getter and setter), `a_frozen_array_refuses_a_set_a_define_and_a_delete_and_keeps_its_elements`, `test_integrity_level_answers_for_each_level_and_for_an_empty_non_extensible_object`, `integrity_levels_charge_a_step_per_key` (from `last_entry_steps()`), `step_budget_ends_a_freeze_of_many_keys`, `invoke_gets_from_a_primitive_and_calls_with_it_as_this`, `invoke_of_what_is_not_callable_throws_type_error`, `enumerable_own_properties_of_each_kind_skip_symbols_and_what_is_not_enumerable`, `enumerable_own_properties_skip_a_key_a_getter_deleted`, `create_data_property_or_throw_throws_on_a_non_extensible_object`.
  - in `builtins/string/mod.rs`: `code_point_at_reads_a_pair_and_a_lone_surrogate_of_each_kind`, `utf16_encode_code_point_and_code_point_at_round_trip_every_scalar_value`.
  - in `realm/mod.rs`: the two new symbols are distinct from the five others, alike in two realms of one agent and survive a collection.

Writes: `crates/swell/src/builtins/array.rs` (moved to `crates/swell/src/builtins/array/mod.rs`), `crates/swell/src/builtins/array/{iteration,search,mutation}.rs` (new), `crates/swell/src/builtins/object.rs` (moved to `crates/swell/src/builtins/object/mod.rs`), `crates/swell/src/builtins/object/{integrity,enumeration}.rs` (new), `crates/swell/src/builtins/string.rs` (moved to `crates/swell/src/builtins/string/mod.rs`), `crates/swell/src/builtins/string/{access,transform}.rs` (new), `crates/swell/src/builtins/mod.rs`, `crates/swell/src/builtins/json.rs`, `crates/swell/src/ops/objects.rs`, `crates/swell/src/interp/dispatch.rs`, `crates/swell/src/realm/mod.rs`, `crates/swell/src/object/mod.rs` (the four attribute lines of the mark of `prevent_extensions`), `crates/swell/tests/support/stack.rs` (new), `crates/swell/tests/support/mod.rs`, `crates/swell/tests/limits_scripts.rs`, `crates/swell/tests/builtins_array.rs`, `crates/swell/tests/builtins_string.rs`. Mark it removes: the one of `prevent_extensions` in `crates/swell/src/object/mod.rs`. Shared with the wave: that file with `story:property-index-and-dense-elements`, four attribute lines here and every other line there; no other file. `conformance/test262/expected.txt` does not change.

## Acceptance

`cargo test --locked -p swell`, `slot task test-collect` and `slot task clippy` exit 0 and run the tests named in Change and the three stack tests of `limits_scripts.rs`, `budgets.rs` and `eval_stack.rs` unchanged, on seven eighths of `min_stack_bytes()` with `STACK_BYTES_PER_REENTRY` unchanged; with `slot task test262 TEST262_OUT=.out/test262-base` run on the base before the change and `slot task test262 TEST262_OUT=.out/test262-unit` after it, both exit 0 and `cmp .out/test262-base/runs.tsv .out/test262-unit/runs.tsv` exits 0 (mode, outcome and detail of all 85,556 runs are the same: the story adds no builtin; `awk 'END { print NR }' .out/test262-unit/runs.tsv` prints `85556`); `git diff --stat main -- conformance/` prints nothing; `grep -rlE 'reason = "[^"]*story:' crates/swell/src | sort` prints `crates/swell/src/builtins/string/mod.rs`, `crates/swell/src/object/mod.rs`, `crates/swell/src/ops/conversion.rs`, `crates/swell/src/ops/objects.rs` and `crates/swell/src/ops/testing.rs`, and besides them only a group file whose mark the report quotes with the compiler's message (on the base it prints the three of these that carry a mark of an earlier story: `object/mod.rs`, `ops/conversion.rs`, `ops/testing.rs`; the pattern finds `story:<name>` anywhere in a reason, since a reason reads `<what>: story:<name>` on the base, `crates/swell/src/ops/testing.rs`, line 74); and `grep -c 'Object.preventExtensions: story:harness-builtins' crates/swell/src/object/mod.rs` prints `0` (`1` on the base).

## Out of scope

Any builtin (a group module that installs something is the next wave's); the slot [[IsRawJSON]] (`story:json-parse-and-raw-json`, the one writer of `object/ordinary.rs` in the next wave); `builtins/function.rs`, `error.rs`, `global.rs`, `math.rs`, `number.rs`, `boolean.rs` (one story each, or none); `realm/intrinsics.rs` and `realm/global_object.rs` (`story:global-this-uri-functions-and-is-error` alone writes them in the next wave); `STACK_BYTES_PER_REENTRY`; the property list and the elements of arrays (`story:property-index-and-dense-elements`); `Symbol` and every other well-known symbol (slice 6).
