---
format: aep.planning-md/3
id: review-result:adversary-fields-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on the unit fields: red, one failing case of 14 and four findings'
relations:
- reviews: story:public-class-fields
revision: 1
---
Adversary pass 1 on `impl/fields` at `6a77e86` (wave big-2, slice 8, order 3), run on 2026-11-27.
The report below is the adversary's last message; paths of the workstation are replaced.

unit: fields
verdict: red
cases: executed 13→14, red 1
origin: introduced 4, pre-existing 0, undecided 0
wrote-outside-worktree: <scratch>/wave2/fields/adversary-1/ (scripts, logs, probe copies of the test file)
needs-coordinator: yes. The one red case tests a rule that `story:eval-in-class-field-initializers` owns. Decide whether it stays here or goes to that story.

## Findings

1. **A direct eval of `arguments` in an initializer gives the wrong error.** `eval("arguments")` in a field initializer throws `ReferenceError` ("arguments is not defined"). The spec (`#sec-performeval`, inClassFieldInitializer) says `SyntaxError`. `eval("() => arguments")` throws nothing: the arrow is made and only throws `ReferenceError` when called. Instance and static fields behave the same, and the outer function's `arguments` does not leak in.
   - Reached through any direct eval in an initializer, which works since `f264bd3` lifted the hold. This is a wrong answer, not a stated refusal.
   - Case: `crates/swell/tests/class_fields.rs:555`. Result `left: (Normal("false"), Normal("false"))`, exit 100 in both modes.
   - `eval("super.x")` and `eval("super()")` are stated refusals ("super in eval code is not supported before slice 8"). `eval("new.target")` gives `undefined`, `eval("this")` gives the instance, and `var` declared in the eval stays inside it: all correct.

2. **No test checks that a computed field key stays alive between the class definition and the constructions.** Plant A (in a copy): the collector skips the env slots named `#key …`.
   - All 13 tests stay green in both modes: `Summary 16 tests run: 16 passed` (13 tests plus 3 probes).
   - 9 of my probe scripts end in `Terminated(EngineFault)` in the collect build. They are the ones whose keys are made at run time (`'k'+i` in a loop, `n++`, a key computed by eval, Symbol keys). The 13 tests use only literal-backed keys or Symbols held in a `var`, which stay alive anyway.
   - The code itself keeps the keys correctly: the same probes are green without the plant, in both modes.

3. **A change to where field errors are reported would go unnoticed.** Mutant: `DefineField` / `DefineComputedField` emitted at the initializer function's position, or the key's `ToPropertyKey` at the start of the whole field instead of the key. Neither the 13 tests nor the 515 cases check error positions.
   - The code is correct today: `last_entry_error_site` gives (3,3) for the failing `b` in `a = 1;\n b = Object.preventExtensions(this) && 0;`, and (3,4) for a key whose ToPropertyKey throws.

4. **A change to `__proto__` fields would go unnoticed.** Mutant: a literal field `__proto__ = v` compiled like the object-literal `__proto__: v`, which sets the prototype instead of making an own property. None of the 13 tests or 515 cases has a `__proto__` field (grep of the case list: 0).
   - The code is correct today: it gives an own property, the prototype is unchanged, and a static `__proto__` field behaves the same.

## Cases added
- `crates/swell/tests/class_fields.rs`, `adversary_direct_eval_of_arguments_in_an_initializer_is_a_syntax_error` (line 555, uncommitted, red).
- Plant copies: `.out/adv-plant-a`, `.out/adv-plant-b` (ignored by git).

Note tests for the mutants (not added, because the code does not have these defects):
- **Finding 2:** both_pass of `var cs = []; for (let i = 0; i < 3; i++) cs.push(class { ['k' + i] = i; [Symbol('s' + i)] = i; static ['s' + i] = i; }); var junk = []; for (var j = 0; j < 20; j++) junk.push({ j: j }); cs.map(function (C, i) { var o = new C(); return Object.keys(o).join() + Object.getOwnPropertySymbols(o)[0].description + C['s' + i]; }).join() === 'k0s00,k1s11,k2s22';`
- **Finding 3:** evaluate `class C {\n  a = 1;\n  b = Object.preventExtensions(this) && 0;\n  c = 2;\n}\nnew C();`. Assert `Throw(TypeError)` and that `last_entry_error_site()` is (3,3), read before the next entry.
- **Finding 4:** both_pass of `class C { __proto__ = 5; } var c = new C(); Object.getPrototypeOf(c) === C.prototype && Object.getOwnPropertyDescriptor(c, '__proto__').value === 5;`

## What held (no finding)
- **Order and count of evaluation (item 1):** keys of fields, methods, accessors and static elements mixed run once, in source order, before the static runs. ToPropertyKey runs once per key, with the hint `string`; `-0` becomes `"0"`; integer keys come first in key order.
  - A computed `"constructor"` is an own property. A static computed `"prototype"` throws TypeError, and the fields before it stay defined.
  - A key that names the class itself throws ReferenceError. A key reading a binding that the initializer later shadows reads the outer value.
  - Each class has its own keys: one function called twice, a class in a `let` loop and in a `var` loop.
  - `yield` and `await` in keys work, also with two generators interleaved and with a collection at every safe point.
- **Initializer scope (item 2):** `arguments` in an initializer is an early error, also in an arrow, and allowed in a nested function. A field defined later reads as `undefined`. Constructor parameters and `var`s are not visible. A class declared later is in its TDZ (ReferenceError).
  - Script cannot reach the hidden `#key` binding: not through `with`, a global property, or eval in a method.
  - Nested classes (in a key or in an initializer) keep their own keys.
- **DefineField (item 3):** a field over a prototype accessor or a non-writable prototype property becomes an own data property, with no setter call. A sealed or non-extensible instance gives TypeError.
  - Fields are defined after OrdinaryCreateFromConstructor reads `prototype` and before the constructor's parameters are bound (`get prototype,field,param,body`). This holds through `Reflect.construct`, a bound class, and a Proxy with and without a `construct` trap.
  - Static fields `name` and `length` replace the function's properties (writable, enumerable, configurable afterwards). Duplicate field names: the last value wins.
- **Function names (item 4):** names are right for literal, string, Symbol (with and without description, and `Symbol('')` gives `[]`), numeric (`0x10` gives `16`, `1e3` gives `1000`) and object keys. This holds for arrow, function, generator, async and async generator functions, for a class with and without a static `name`, and through parentheses. A comma expression or a call result gets no name.
- **Budgets (item 5):** script `var C = class { a = 1; ['b'] = this.a + 1; ['c' + ''] = [this.b]; }; …` (3 fields, 2 computed keys).
  - Step budget: 0 to 71 steps end in StepBudget, 72 and more give `5`. No other outcome.
  - Memory budget: fresh agent (389,722) + 2,048 + k·64 bytes. k 0 to 226 end in MemoryBudget, from k=227 the result is `5`. No other outcome.
  - Leaks: 3 rounds of 1,000 constructions (an initializer that throws, recursion to the frame depth, re-entry through `Reflect.construct`, a recursing initializer). `heap_bytes` 742,027 after every round, `live_bytes` 302,319, the recursion-depth stand-in for `frame_depth` 198 every round.
  - All 103 probe outcomes are the same with `collect-every-safe-point` as in the default build.
- **Plant B (item 5):** the function `MakeMethod` just pushed is not traced until called. Default build: 0 of 13 red. Collect build: 11 of 13 red, all but `fields_with_and_without_initializers_in_order` and `constructions_of_a_class_with_100_fields_are_charged_and_freed`.
- **Stack (item 6):**
  - `class C { x = f(); }` re-entered through `Reflect.construct(C, [])` passes 1,000 re-entries on seven eighths of `min_stack_bytes()` (12,838,560 of 14,672,640).
  - Bisected need: 7,570 bytes per re-entry, against 10,946 through `toJSON` in the same build and 14,336 promised (`STACK_BYTES_PER_REENTRY`). Test profile, `debug = false`.
- **Resolver and compiler (item 7):** no `unwrap`, `expect` or unchecked index; arithmetic saturates; the walk uses a work list, not recursion; maps and strings are charged.
  - `git diff 8ab9456 6a77e86 --stat -- crates/compiler/tests/sources` lists only the two new files.
  - `fits_the_stack_bound` for the `class_field` kind (parts 6, 7 and 8, my index count): gate 0.41, 0.44, 0.62 CPU s; full count 1.20, 1.29, 1.59 CPU s.
- **Times (item 8):** CPU seconds per test run alone, maximum over the 13 tests:
  - Default gate: 0.43 (100-field test); others at most 0.23.
  - Default full count: 6.41 (100-field test, the exception named in the brief); others at most 0.23.
  - Gate collect build (profile `collect`): 0.12; at full count 0.18.
  - Lane collect build: 1.44; at full count 2.11 (100-field test). Under the bounds.

## Commands
- `cargo nextest run --locked -p swell --test main --no-capture -E 'test(=class_fields::adversary_probe_outcomes)'`: `PASS [0.818s]`, exit 0.
- Probes for eval, async, steps, memory, leaks and stack (default build): 6 passed, exit 0.
- The same probes with `--features collect-every-safe-point`: 3 passed, 1 timed out. The leak probe at 1,000 rounds is too heavy for the collect build; that is not a finding.
- `sh adversary-1/plants.sh`: plant A default and collect `16 passed`. Plant B default `16 passed`; plant B collect `4 passed, 12 failed` (11 of the 13 tests plus 1 probe).
- `slot --timeout 30m sh -c "<measure.sh>"` and `"<measure2.sh>"`: exit 0. Times above.
- `slot --timeout 30m task test-stack-profiles`:
  - `test-stack-profiles release: 1 test binaries: 4 passed, 0 failed, 5 skipped`
  - `test-stack-profiles stack-debug: 1 test binaries: 4 passed, 0 failed, 5 skipped`
- Final `cargo nextest run --locked -p swell --test main --no-fail-fast -E 'test(/^class_fields::/)'`: `14 tests run: 13 passed, 1 failed`, exit 100. The same with `--features collect-every-safe-point`: `14 tests run: 13 passed, 1 failed`, exit 100.

## Not attacked
- The 515 test262 cases were not run again.
- Lowering the step and memory budgets was done in the default build only, not the collect build.
- The leak run of 1,000 rounds was not done in the collect build (over 60 s).
- A generator whose key is suspended cannot resume the same point twice; I tested two interleaved generators instead.
- The mutant "walk the instance initializer function at every instance field" was not built.
- The stack need was not bisected in the `stack-debug` profile.
- A Proxy returned as the instance cannot be reached without derived classes.
- `super` in an arrow in an initializer is a stated refusal ("super in an arrow function is not supported before slice 8"), left to `story:derived-classes-and-super-call`.

```findings
- file: crates/swell/tests/class_fields.rs
  line: 555
  category: judgement
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: A direct eval of arguments in a field initializer throws ReferenceError (and an eval'd arrow naming arguments is made without error) where PerformEval requires a SyntaxError, a silent wrong answer reachable since the hold was lifted and owned by story eval-in-class-field-initializers.
- file: crates/swell/tests/class_fields.rs
  line: 73
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: With the collector planted to skip the hidden key slots all 13 tests stay green in both modes while nine probe scripts with keys made at run time end in EngineFault, so no test holds the root of a computed key between definition and construction.
- file: crates/compiler/src/class.rs
  line: 148
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: Emitting DefineField, DefineComputedField or the key's ToPropertyKey at the function's or the element's span instead of the field's or the key's would pass the 13 tests and the 515 cases, since none reads an error site; the code is correct today.
- file: crates/compiler/src/class.rs
  line: 143
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: Compiling a literal __proto__ field as the object-literal prototype set would pass the 13 tests and the 515 cases, which hold no __proto__ field; the code defines an own property today.
```
