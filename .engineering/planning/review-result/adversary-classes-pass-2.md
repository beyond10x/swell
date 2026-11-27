---
format: aep.planning-md/3
id: review-result:adversary-classes-pass-2
kind: review-result
status: active
title: 'Adversary pass 2 on the unit classes: green, two mutants caught by its new cases'
relations:
- reviews: story:class-definition-evaluation
revision: 1
---
Adversary pass 2 (the last) on `impl/classes` at `e249bfd` (wave big-2, slice 8, order 8.2), on correction 1, run on 2026-11-27.
The report below is the adversary's last message; paths of the workstation are replaced.

unit: classes
verdict: green
cases: executed 24→28, red 0
origin: introduced 2, pre-existing 0, undecided 0
wrote-outside-worktree: <scratch>/wave2/classes/adversary-2/ (probes, mutant and plant files, scripts, logs; the copy's `plant/target` was deleted)
needs-coordinator: no

The correction holds. Every case I tried for items 1 to 4 gave the answer the spec gives, in both modes, and everything I planted was caught. The two findings are notes: two changes that the unit's own 24 tests do not catch. My new cases catch both.

**Findings**

1. **Mutant survives: the return path clears strict code** (note, introduced in `3e013e7`).
   - The change: in `exit_return` (`crates/compiler/src/lib.rs:893`), `if self.strict { SetStrict(false) }` replaces `if self.strict != self.code_strict { SetStrict(self.code_strict) }`.
   - The unit's 24 tests pass with it, in both modes.
   - My case catches it: `classes.rs:1015`, a `"use strict"` generator with `yield` in a key, `return()`, and a `finally` that assigns an undeclared name. It answers `32 none`; exit 100 in both modes.
   - Reaches: strict generators and async generators.
2. **Mutant survives: the resolver names a class under `__proto__`** (note, introduced in `3e013e7`).
   - The change: `(computed || is_proto(&p.key))` at `resolve.rs:1285`.
   - The unit's tests pass with it.
   - My case catches it: `classes.rs:1050` and `:1052` answer `2 proto [object Object]` and `3 proto string [object Object]`, because `SetClassName` reads the object as the key. Exit 100 in both modes.

**What held**
- **Items 1 and 2:** 38 ways into and out of class code in non-strict code, plus 24 more in a second test. Each puts a check before, inside and after the class, in one script. They cover:
  - the mapped `arguments`, `arguments.callee` and `this` of the function around a key, `eval("this")`, and an arrow's `this`;
  - arrows and object methods in a key (strict), and an indirect `eval` and `Function` in a key (non-strict);
  - a class in a parameter default, followed by another default and by the body;
  - `with` (including a binding deleted in between and a read through an unscopables getter), a `switch` case falling through and a case test, a tagged template, a labelled block;
  - `break` and `continue outer` taken in a `finally` after `return()` or `throw()` into a key;
  - a key that re-enters its own function;
  - eval code and `Function` code with a class, direct and indirect;
  - strict code staying strict: a strict function, a method, and a strict catch;
  - three nested classes, `for-of`, `do-while` with `continue`, `??`, and a class in a `for` update;
  - `super.x =` to a frozen prototype;
  - `async` functions and arrows that reject or resume in a key;
  - 12 strict-only syntax errors in a key, and the unmapped `arguments` of a function in a key.
- **Step budget:** a run ended by the step budget inside a key in a non-strict frame. The next call of the same function object, and the next script, run non-strict.
- **Item 3:** 21 NamedEvaluation positions, run as written and as strict:
  - the key's `toString` runs once and before the class's own keys;
  - a Symbol returned by `toString` gives `[d]`; the numbers `1e21`, `-0` and `0.5` give their strings;
  - `__proto__:` written as an identifier or a string names nothing; `["__proto__"]` names;
  - a parenthesised class is named and a comma expression is not;
  - a named class keeps its name;
  - `=`, `??=`, `||=`, pattern defaults, `let` and `const` name the class;
  - a static `name` method or getter wins everywhere.
- **Item 4:** the handler records of `strict-code-of-a-class.disasm` hold. A handler's strictness is fixed where its task runs. In statements that is outside every class; in a key it is inside. The two cases where class code and handler cross:
  - a pattern range that contains a class records non-strict, and its target is outside the class;
  - a handler that starts inside a key is a pattern or `yield*` handler, and both throw again.

  So an equivalent mutant there (writing the code's strictness instead of the place's) cannot be told apart, and I did not count it as a finding.
- **Item 5:** a source with a syntax error next to a held element still reports the syntax error. `HOLDS` refuses only once the source parsed with no early error, and verdict `E` rejects a refusal.
  - Plant: the error "a class field may not be named constructor" removed (`early.rs:1658`).
  - Result: `90 tests run: 86 passed, 4 failed`. Red: `early_error_sources_fail_with_their_message`, `element_edges_answer_the_standard`, `element_names_and_duplicates_answer_the_standard`, `message_kind_tells_refusals_from_early_errors`. Each got `class field is not supported before slice 8` where the early error was expected.
  - The other three merge resolutions: `parser/class.rs` and `parser/expression.rs` differ from the wave only in comments, except that the `hold("class")` call is gone, which is this unit's change. `tests/main.rs` passed `test-main.sh check` in the correction.
- **Item 6, second plants:**
  - Last class element dropped (`class.rs:136`): `a_class_with_1000_methods` is red in both modes; collect gives `(Throw("TypeError"), Throw("TypeError"))`.
  - `check_frame_depth` removed from `push_call_frame` (`dispatch.rs:1577`): `a_method_recursion_stops_at_frame_depth` is red with `Terminated(StepBudget)`, 0.435 s default and 1.234 s collect.
- **Item 7, the other two mutants:**
  - `exit_return` writes `SetStrict(self.strict)`: red in 2 tests, including the unit's `a_frame_suspended_in_a_key_resumes_strict`.
  - `enter_handler` writes only when `handler.strict` is true: red in 5 tests, 3 of them the unit's.
- **Item 8, CPU seconds** of each test alone, one thread, build node, 28 tests: all exit 0 and none is over its bound.
  - Default: highest is `a_class_made_in_a_loop_is_charged_and_freed`, 0.361 (bound 1.5).
  - Lane collect: highest is `every_strict_operation_in_a_computed_key_is_strict`, 1.744 (bound 2). It is the nearest to its bound.
  - My four tests: 0.009 to 0.020 default, 0.119 to 0.795 collect.

**Cases added (uncommitted)** in `<trees>/swell-impl-classes/crates/swell/tests/classes.rs`, formatted with `cargo fmt -p swell`:
- helper `adversary_answer`
- `adversary_every_way_into_and_out_of_class_code`
- `adversary_every_named_evaluation_of_an_anonymous_class`
- `adversary_a_step_budget_in_a_key_leaves_no_strictness_behind`
- `adversary_strict_semantics_in_keys_of_every_kind`

**Commands** (one pool job at a time; plants and mutants in the copy `adversary-2/plant`, a `git archive e249bfd` export)
- Probes (`classes::adversary2_probe*`, 4 tests, removed afterwards): every result `true`; the budget probe gave `Normal("0") Terminated(StepBudget) Normal("\"number\"") Normal("\"number\"")`.
- `cargo nextest run --locked -p swell --test main -E 'test(/^classes::/)'` → `28 tests run: 28 passed, 1323 skipped`, exit 0.
- The same with `--features collect-every-safe-point` → `28 tests run: 28 passed, 1323 skipped`, exit 0. Both runs repeated after formatting, same result.
- Mutants, each with `classes::` in both modes:

| Mutant | Default | Collect |
|---|---|---|
| m1 | `27 passed, 1 failed` | `27 passed, 1 failed` |
| m2 | `26 passed, 2 failed` | `26 passed, 2 failed` |
| m3 | `23 passed, 5 failed` | `23 passed, 5 failed` |
| m4 | `27 passed, 1 failed` | `27 passed, 1 failed` |

  Each run exits 100.
- Plant 5: `cargo nextest run --locked -p swell-syntax` → `90 tests run: 86 passed, 4 failed`, exit 100.
- Plant 6a: `1 test run: 0 passed, 1 failed` in both modes, exit 100. Plant 6b: the same.
- Timing: `slot --timeout 30m sh -c "$(cat time-node.sh)"` → exit 0, all 56 runs `rc 0`.
- `cargo fmt -p swell -- --check` → exit 0, after `cargo fmt -p swell` changed only the layout of my cases.

**Not attacked**
- Heritage, because the compiler refuses it (`class.rs:106`).
- Fields, static blocks and private names, which are still held.
- A generator suspended in a key and then collected. It is a frame of its own and nothing in it can show strictness.
- The two collect-profile builds the correction also timed, `SWELL_FULL_COUNTS=1`, test262, and the stack check.
- A mutant of `set_class_name`'s stack index. The unit's own tests already fail on a wrong key.

```findings
- file: crates/compiler/src/lib.rs
  line: 893
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: Changing exit_return to write SetStrict(false) whenever the code being emitted is strict passes all 24 unit tests, although a strict generator's return() from a yield in a key then runs its finally as non-strict code; the case adversary_every_way_into_and_out_of_class_code (classes.rs:1015) catches it.
- file: crates/compiler/src/resolve.rs
  line: 1285
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: Marking a class under a __proto__ key as named by key passes all 24 unit tests, although it then names the prototype class "[object Object]"; the case adversary_every_named_evaluation_of_an_anonymous_class (classes.rs:1050) catches it.
```
