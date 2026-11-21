---
format: aep.planning-md/3
id: story:number-boolean-and-global-functions-early
kind: story
status: implemented
title: Number, Boolean, isNaN, isFinite and the String wrapper methods before their slices, complete for what they cover
summary: 'Parts taken early under the design rule: the Number and Boolean constructors, the Number value properties, valueOf and toString of the wrappers (Number.prototype.toString in every radix, swell-num), isNaN and isFinite; 512 cases of slice 2 need nothing later.'
relations:
- decomposes: epic:slice-02-language-core
scope:
- confidence: cited
  path: crates/num/src/lib.rs
- confidence: cited
  path: crates/num/src/to_string.rs
- confidence: cited
  path: crates/num/tests/to_string.rs
- confidence: cited
  path: crates/swell/src/builtins/boolean.rs
- confidence: cited
  path: crates/swell/src/builtins/global.rs
- confidence: cited
  path: crates/swell/src/builtins/mod.rs
- confidence: cited
  path: crates/swell/src/builtins/number.rs
- confidence: cited
  path: crates/swell/src/builtins/string.rs
- confidence: cited
  path: crates/swell/src/object/builtin_function.rs
- confidence: cited
  path: crates/swell/src/realm/global_object.rs
- confidence: cited
  path: crates/swell/src/realm/intrinsics.rs
- confidence: cited
  path: crates/swell/tests/builtins_boolean.rs
- confidence: cited
  path: crates/swell/tests/builtins_global.rs
- confidence: cited
  path: crates/swell/tests/builtins_number.rs
- confidence: cited
  path: crates/swell/tests/builtins_string.rs
- confidence: cited
  path: crates/swell/tests/language/boolean-wrapper-to-primitive.js
- confidence: cited
  path: crates/swell/tests/language/isnan-global.js
- confidence: cited
  path: crates/swell/tests/language/number-wrapper-to-primitive.js
- confidence: cited
  path: crates/swell/tests/language/number-wrapper-to-string.js
- confidence: cited
  path: crates/swell/tests/language/string-wrapper-to-primitive.js
revision: 10
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T13:41:53Z", actor: "human:operator", revision: 7, executor: "agent:coordinator"}
- {from: "proposed", to: "active", at: "2026-11-21T13:41:53Z", actor: "human:operator", revision: 8, executor: "agent:coordinator"}
- {from: "active", to: "implemented", at: "2026-11-21T14:46:07Z", actor: "human:operator", revision: 10, decided_on: {"recorded":{"test_result":1,"verification":1}}, executor: "agent:coordinator"}
---
## Context

Measured on the base (`6319af2`) with the per-run causes of a throwaway runner: of the 3,120 failing cases of slice 2, the first failed run of 201 throws `ReferenceError: Number is not defined`, of 164 `Boolean is not defined`, of 85 `isNaN is not defined`, and 56 more fail on a wrong result of the String wrapper (`new String("1") * "1"` gives `NaN`: ToPrimitive of a String object finds no `valueOf` and no `toString` of its own and takes `Object.prototype.toString`). These are tests of the operators of slice 2 that use the wrappers and `isNaN` as tools. A static scan of each failing case and its includes (comments removed, string contents blanked) finds 512 cases whose only parts of a later slice are `Number`, `Boolean`, `isNaN` or `isFinite`; the members they use are `new Number(` (611 uses), `Number.MAX_VALUE` (335), `Number.MIN_VALUE` (261), `Number.POSITIVE_INFINITY` (243), `Number.NEGATIVE_INFINITY` (208), `Number.NaN` (188), `Number(` (111), `new Boolean(` (1,112), `Boolean(` (18), `isNaN(` (296), and a few `valueOf` and `constructor`.

Two ways meet the epic (every run of the slice passes): move those cases to slices 3 and 4 by about 570 rule lines, or take the parts early under the design rule "A part before its slice" (`docs/design/engine.md`, section 12: "A part taken early is built to the standard for what it covers, never as a stand-in"). The parts are small and complete for what they cover, and the cases test the language of slice 2, so they are taken here. What a member needs from a later slice stays out whole: `toFixed`, `toExponential`, `toPrecision`, `toLocaleString`, `Number.parseFloat` and `Number.parseInt` and the `Number` statics (`Number.isInteger` and the others) are slice 4.

`Number.prototype.toString` is taken complete, every radix. Without it, a Number object converted with the string hint (`String(new Number(1))`, a property key, `new Number(1).toString()`) finds `Object.prototype.toString` on the prototype chain and answers `[object Number]`, a wrong answer of a conversion that exists, which the rule of the slice forbids ("a method that exists is correct for every argument"). Decided with the code: `swell-num` prints radix 10 only (`crates/num/src/to_string.rs`, `number_to_string`, with `shortest_digits`, the free-format algorithm of Steele and White as Burger and Dybvig give it over the small big integers of the module, cited), and its module comment gives the other radices to slice 4; ECMA-262 states every radix normatively (`sec-numeric-types-number-tostring`, step 5: k as small as possible, for any radix from 2 to 36), and the design already says the operation "is written here for every radix 2 to 36, with an exact shortest-digits algorithm over `swell-num`'s big integers" (`docs/design/engine.md`, section 6). `shortest_digits` takes the radix as a parameter in place of its 10 (its scaling by `mul_pow10`, its digit loop by `mul_small(10)`, its estimate of n corrected by the loops that follow it), so the radix printing is taken from slice 4 here; reading a string in a radix (`parseInt`, `parseFloat`) stays slice 4. Measured on the base (throwaway runner, every run): 1 case of slice 2 (`test/language/expressions/property-accessors/S11.2.1_A3_T2.js`) and 37 of slice 4 (36 under `test/built-ins/Number/prototype/toString/`, which holds 90 cases, and 1 of `toFixed`) fail with `[object Number]` in their detail; of the 437 cases of slice 2 that name `Number`, 112 also name `toString`, `String(` or a template, an upper bound of those that may convert a Number object with the string hint once `Number` exists (`xargs grep -lE` over the cases).

`%Number.prototype%` and `%Boolean.prototype%` exist (`crates/swell/src/builtins/number.rs`, `boolean.rs`, `story:object-model-and-realms`) with their [[NumberData]] and [[BooleanData]]; their doc comments name slices 4 and 3 for the constructors, and change with this story. No `expect(dead_code)` mark names these parts.

`crates/swell/src/realm/intrinsics.rs` is written by this story and later by `story:with-arguments-and-dynamic-names` (the `expect(dead_code)` mark of `Intrinsic::ThrowTypeError`) and `story:eval-and-function-constructor`; both depend on this story, which comes first.

Design it builds on, on the branch since merge `63dff4d` (a precondition that holds before wave 1): "A part before its slice" in section 12 names, as parts of slice 2 taken early, the `Number` constructor with its value properties and `Number.prototype.constructor`, `valueOf` and `toString` (with `Number::toString` for every radix of `swell-num`, from slice 4), the `Boolean` constructor with `Boolean.prototype.constructor`, `toString` and `valueOf` (from slice 3), `isNaN` and `isFinite` (from slice 3), and `String.prototype.toString` and `valueOf` (from slice 3).

## Additions in wave 1 of slice 2 (2026-11-21)

Binding; where this section and the text below differ, this section governs.

- **Three fixture cases of the runner's own tests are this story's too.** The fixture
  `crates/test262/tests/fixtures/tree/test/built-ins/Number/prototype/plain.js` ran
  `Number.prototype;` and the runner's tests expect it to fail; with `Number` delivered it
  passes and 11 tests of `swell-test262` fail. A fixture that must fail does not depend on a
  builtin being absent: its body names an identifier no slice defines. The same change is made
  now in `Atomics/wait/agent.js` and `Atomics/wait/no-agent.js` of that tree, which would break
  the same way when `Atomics` is delivered. The expectations of the runner's tests do not
  change.
- **No omission line in `conformance/test262/sections.txt`.** The parts of `Number` and
  `String` that stay out are whole methods of later slices, not a part taken early that is
  incomplete; the design's omission rule does not apply to them.
- **`Array.prototype.toString` is not this story's.** An array still converts through
  `Object.prototype.toString`; the wave plan records it for a decision before wave 4.

Writes, added: `crates/test262/tests/fixtures/tree/test/built-ins/Number/prototype/plain.js`,
`crates/test262/tests/fixtures/tree/test/built-ins/Atomics/wait/agent.js`,
`crates/test262/tests/fixtures/tree/test/built-ins/Atomics/wait/no-agent.js`.

## Change

Failing tests first: the tests below are written before the code and fail (the globals are not defined; `swell_num` has no radix but 10).

- `crates/swell/src/builtins/number.rs`: the Number constructor (`sec-number-constructor-number-value`: ToNumeric, a Number object from `new` through OrdinaryCreateFromConstructor with `%Number.prototype%`; the BigInt branch cannot be reached before slice 12 and says so), the value properties of the constructor (`sec-properties-of-the-number-constructor`: `EPSILON`, `MAX_SAFE_INTEGER`, `MAX_VALUE`, `MIN_SAFE_INTEGER`, `MIN_VALUE`, `NaN`, `NEGATIVE_INFINITY`, `POSITIVE_INFINITY`, each `{ [[Writable]]: false, [[Enumerable]]: false, [[Configurable]]: false }`), `Number.prototype.constructor`, `Number.prototype.valueOf` (`sec-number.prototype.valueof`, ThisNumberValue) and `Number.prototype.toString` (`sec-number.prototype.tostring`: ThisNumberValue, radix `undefined` is 10, ToIntegerOrInfinity of the radix, a `RangeError` outside 2 to 36, then `Number::toString(x, radix)`).
- `crates/num/src/to_string.rs`: `number_to_string_radix(x, radix)` for every radix from 2 to 36 (`sec-numeric-types-number-tostring`, steps 1 to 12: shortest digits by `shortest_digits` with the radix as parameter, of two equally close candidates the even one as Note 2 recommends, digits `0` to `9` and `a` to `z`, no exponent for a radix other than 10); `number_to_string(x)` stays the radix 10 entry and answers as before; `crates/num/src/lib.rs`: the module comment (radix printing taken early here; `radix.rs` of slice 4 reads strings in a radix).
- `crates/swell/src/builtins/boolean.rs`: the Boolean constructor (`sec-boolean-constructor-boolean-value`), `Boolean.prototype.constructor`, `toString` (`sec-boolean.prototype.tostring`) and `valueOf` (`sec-boolean.prototype.valueof`), ThisBooleanValue.
- `crates/swell/src/builtins/global.rs`: `isNaN` (`sec-isnan-number`) and `isFinite` (`sec-isfinite-number`), the intrinsics `%isNaN%` and `%isFinite%`.
- `crates/swell/src/builtins/string.rs`: `String.prototype.toString` (`sec-string.prototype.tostring`) and `valueOf` (`sec-string.prototype.valueof`), ThisStringValue.
- `crates/swell/src/realm/intrinsics.rs`: `%Number%`, `%Boolean%`, `%isNaN%`, `%isFinite%` among the intrinsics, the `constructor` properties of the two prototypes; `crates/swell/src/realm/global_object.rs`: the global properties `Number`, `Boolean`, `isNaN`, `isFinite` (clause 19, `{ [[Writable]]: true, [[Enumerable]]: false, [[Configurable]]: true }`); `crates/swell/src/object/builtin_function.rs`: the `BuiltinId` variants of the new builtins, one per module as for slice 1; `crates/swell/src/builtins/mod.rs`: its doc comment.
- Tests, each in both collection modes (`test-collect` runs every test of the crate): in `crates/num/tests/to_string.rs` `radix_ten_entry_is_unchanged` (`number_to_string_radix(x, 10)` equals `number_to_string(x)` on the 200,000 bit patterns the file already draws), `power_of_two_radices_are_exact` (radices 2, 4, 8, 16 and 32 against the bits of x, written out in the test), `every_radix_reads_back` (for each radix from 2 to 36 and 10,000 drawn bit patterns, the digits read back to x by an exact big-integer reading in the test, and no shorter digit string does), `radix_layout_examples` (`255` in radix 16 is `ff`, `0.5` in radix 2 is `0.1`, `-255` in radix 36 is `-73`, `1e21` in radix 36 has no exponent); `crates/swell/tests/builtins_number.rs` (new; `number_called_and_constructed`, `number_value_properties_are_frozen_data`, `number_prototype_value_of_needs_a_number`, `number_prototype_to_string_radix`, with radix 1, 37, `Infinity` and `NaN` throwing a `RangeError` and a radix whose `valueOf` throws), `crates/swell/tests/builtins_boolean.rs` (new; `boolean_called_and_constructed`, `boolean_prototype_methods_need_a_boolean`), `crates/swell/tests/builtins_global.rs` (new; `is_nan_and_is_finite_convert_first`, including an argument whose `valueOf` throws), and in the existing `crates/swell/tests/builtins_string.rs` `string_prototype_to_string_and_value_of_need_a_string`; language scripts (new files) `crates/swell/tests/language/number-wrapper-to-primitive.js`, `number-wrapper-to-string.js` (`String(new Number(255))` is `"255"`, `(255).toString(16)` is `"ff"`, `new Number(1).toString()` is `"1"`), `boolean-wrapper-to-primitive.js`, `string-wrapper-to-primitive.js` (`new String("1") * "1"` is 1), `isnan-global.js`.

`conformance/test262/expected.txt`: the unit writes the lines of its run in its own tree; the coordinator writes the wave branch file from the gate run after the merges.

Writes: `crates/swell/src/builtins/{number,boolean,global,string,mod}.rs`, `crates/swell/src/realm/{intrinsics,global_object}.rs`, `crates/swell/src/object/builtin_function.rs`, `crates/num/src/to_string.rs`, `crates/num/src/lib.rs`, `crates/num/tests/to_string.rs`, `crates/swell/tests/{builtins_number,builtins_boolean,builtins_global}.rs` (new), `crates/swell/tests/builtins_string.rs`, `crates/swell/tests/language/` (the five new files listed, by name), `conformance/test262/expected.txt` (the unit tree; the wave branch file is the coordinator).

## Acceptance

`cargo test --locked -p swell-num -p swell` and `slot task test-collect` exit 0 and run the tests named in Change, and `slot task test262` exits 0 with the unit lines in `conformance/test262/expected.txt`, prints `test262-slice 02 5245 9173 0 0 5245 <p> <f> 0` with `<p>` at least 2,125 (a bound: the 512 cases measured above mostly also need automatic semicolon insertion, `eval` or another part of slice 2 that lands in other stories) and `test262-slice 04 1198 2396 0 0 1198 <p4> <f4> 0` with `<p4>` at least the passed count of that line on `main` (the 36 cases of `Number.prototype.toString` failing with `[object Number]` are a measure, not a bound: they may need a part of slice 4 besides), and after `git show main:conformance/test262/expected.txt > .out/expected-base.txt` the comparison `awk 'NR == FNR { if ($1 == "test262") p[$1 " " $2] = $7; if ($1 == "test262-slice") p[$1 " " $2] = $8; next } { k = $1 " " $2; v = ($1 == "test262-slice") ? $8 : $7 } ($1 == "test262" || $1 == "test262-slice") && (k in p) && v < p[k] { print; bad = 1 } END { exit bad }' .out/expected-base.txt conformance/test262/expected.txt` exits 0 (no directory line, not the total and no `test262-slice` line passes fewer cases than on `main`, the base of the wave).

## Out of scope

Every other member of `Number`, `Boolean`, `String` and the global object (slices 3 and 4: `toFixed`, `toExponential`, `toPrecision`, `toLocaleString`, the `Number` statics, `parseInt`, `parseFloat` and reading a string in a radix, `globalThis`, the URI functions); `Object.prototype.valueOf` and the other `Object` methods (slice 3). Cases that need them move by rule lines in `story:slice-02-conformance-close`. The design text "A part before its slice" (a design author, before this wave).
