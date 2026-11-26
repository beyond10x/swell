---
format: aep.planning-md/3
id: review-result:adversary-classsyntax-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on the unit classsyntax: no defect, 5 mutants the tests miss, 3 notes'
relations:
- reviews: story:class-syntax-and-early-errors
revision: 1
---
Adversary, pass 1 on the unit classsyntax of wave big-2 (impl/classsyntax at 1e671094), 2026-11-26.
Verdict: no defect found. 8 findings (2 warnings as mutants, 6 notes; 7 introduced, 1 pre-existing); 399 sources answered as expected; no failing case.

unit: classsyntax
verdict: green
cases: executed 75→75, red 0
origin: introduced 7, pre-existing 1, undecided 0
wrote-outside-worktree: <scratch>/wave2/classsyntax/adversary-1/
needs-coordinator: yes

No defect found in `classsyntax` at `1e671094`. The findings are five code changes the unit's tests miss, one unclear test message, and two notes for the coordinator.

**What was run**
- **Items 1, 2, 4, 5, 6:** 399 sources through the script, eval and `Function`-body entries, each with its expected answer. All 399 matched; eval matched script every time.
- **Item 2:** an early error anywhere beats the hold. A field, static block, private name or `super` is refused at its token and wins even over an earlier early error, as the story says.
- **Item 3:** through `Agent::evaluate_script`, a script with a class answers `Errors(["class is not supported before slice 8 (1:1)"])`. `eval`, `Function` and the three other function constructors throw a catchable SyntaxError; nothing of a class source runs.
- **Item 7:** nine extra nesting kinds through all three entries fit at limit 10 and 2,500 (slowest 64 ms). 1,000,000-deep text ends in the nesting error in under 1 ms. Class bodies of 25k to 200k elements parse in linear time (0.19 to 1.44 s). `class.rs` (384 lines read) has no `unwrap`, `expect`, unchecked index or overflowing arithmetic.
- **Item 8:** four helpers made `pub(super)`, bodies unchanged. `Parser::parameters` gives the new kinds the right rules.
- **Planted changes, on a copy under `.out/adv-mut/`:**
  - Deleting the `("class", 8)` line of `HOLDS` turns 7 syntax tests and 1 compiler test red.
  - Deleting it with the hold call (a simulated lift) turns only `compile::held_sources_compile_once_nothing_in_them_is_held` red (`compile.rs:196`), as intended.

**Findings**
1. **M2 (mutant):** with `self.push_step(&mut stack, back)` at `crates/syntax/src/early.rs:832` replaced by nothing, all 75 tests pass. The strict state then leaks past a class. The code is correct. Test that catches it, in `sources.rs`: `class A {} with (a) {}` and `x = class {}; with (a) {}` each answer exactly `class is not supported before slice 8`.
2. **M3 (mutant):** with `self.property_key(key, *span)` at `early.rs:1345` removed, all 75 pass. No listed test262 case has a legacy-octal class key. The code is correct. Fixture: `early-errors/class-method-name-octal.js`, header `// sec-numeric-literals-early-errors: SyntaxError "legacy octal literal in strict mode code"`, source `class A { 010() {} }`.
3. **M5 (mutant):** with `PropertyKey::String(_) => false` in `named` at `crates/syntax/src/parser/class.rs:58`, all 75 pass. A single `'constructor'() {}` would then be a false "only one constructor" error. Fixture: `held/class-string-constructor.js`, header `// sec-class-definitions: held "class"`, source `class A { 'constructor'() {} }`.
4. **M1 (mutant):** with both `self.strict = outer;` lines deleted (`class.rs:70` and `:83`), all 75 pass. `class A {} if (a) function f() {}` must answer `function declaration as an if clause is not supported before slice 18` (it does today); it belongs in the test of finding 1.
5. **M4 (mutant):** with `self.hold("class heritage", 8, span)` at `class.rs:185` removed, all 75 tests and the test262 part pass. A hold after the first `class` cannot be seen while `class` is held. Only the story's acceptance `grep` (count 2) catches the deletion; a misplaced hold would pass it.
6. **Unclear message:** with a hold call kept and its `HOLDS` line deleted, five stack-bound parts fail with `Script class_heritage 177: an error other than the nesting error` (`crates/syntax/tests/sources.rs:966`), which does not name the refusal. The held test (`sources.rs:284`) does name it. Origin: the message is in existing helper code.
7. **For the coordinator:** class methods bypass `method_head`.
   - The merge with `story:super-property-and-super-syntax` must set its super flag at `class.rs:244`, between `element_head` and `function_rest`, doing what `method_head` does.
   - That one site covers methods, getters, setters, generator and async methods, and the written constructor (which returns at `:323-333` before `begin_function`). Computed keys and the heritage keep the outer flag.
   - No test of either unit shows the gap. After the merge, a fixture `held/class-super-property.js` (`super.x` in a constructor, method, getter, setter, static and generator method; header `held "class"`) would.
   - The unit's code does not make that story impossible.
8. **Report mismatch:** the unit reports three helpers made `pub(super)`; the diff makes four (`crates/syntax/src/parser/expression.rs:806` `left_hand_side` too).

```findings
- file: crates/syntax/src/early.rs
  line: 832
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: Dropping the Restore step after a class leaks strict mode to the code after it and passes all 75 tests of syntax and compiler; no test parses sloppy-only code after a class.
- file: crates/syntax/src/early.rs
  line: 1345
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: Removing the check of literal class element keys (legacy octal 010 or '\07' as a method name) passes all 75 tests and no listed test262 case has such a key.
- file: crates/syntax/src/parser/class.rs
  line: 58
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: Making a string-keyed 'constructor'() not the constructor passes all 75 tests; no held fixture has a string-keyed constructor.
- file: crates/syntax/src/parser/class.rs
  line: 70
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: Deleting both restores of the parser's strictness after a class (lines 70 and 83) passes all 75 tests; class A {} if (a) function f() {} would answer an early error instead of the slice-18 refusal.
- file: crates/syntax/src/parser/class.rs
  line: 185
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: Removing the class heritage hold passes all 75 tests and the test262 part; every hold after the first is unobservable while class is held, and only the story's grep acceptance catches the deletion.
- file: crates/syntax/tests/sources.rs
  line: 966
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: With a hold call left and its HOLDS line deleted, five stack-bound parts fail with "an error other than the nesting error" without naming the refusal.
- file: crates/syntax/src/parser/class.rs
  line: 244
  category: contract-drift
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: Class methods bypass method_head, so the merge with story super-property-and-super-syntax must set its super flag at class.rs:244 or super.x in class methods stays an early error, and no test of either unit shows it.
- file: crates/syntax/src/parser/expression.rs
  line: 806
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: The unit reports three helpers made pub(super) but the diff makes four (left_hand_side too), bodies unchanged.
```

**Not checked**

The pass reached all ten items. These parts were not done:
- **Items 3 and 10:** no test262 part run (`part.sh` plus `count`) with the planted changes. The claim that no listed case catches them rests on a grep of the 505 part cases that mention `class`, in the local checkout.
- **Item 7:** the extra nesting kinds were not run at `SWELL_FULL_COUNTS=1` (limit 7,500); only at 10 and 2,500.
- **Item 8:** I did not measure the per-node cost of `class_early_errors` against the base myself. I rely on the unit's `timing.txt` and `timing-base.txt`: parts without class kinds are unchanged (part 8: 1.102→1.062 s, part 16: 1.076→1.067 s CPU, default mode).
- **Item 10:** the tests that catch the five changes are given as text only (findings 1–5), not added: the code has none of these defects.
- clippy, fmt and `collect-every-safe-point` were not run.

**Uncommitted cases**

None. The worktree is clean at `1e671094` (`git status --short` prints nothing), and my temporary probe tests are removed. The only file of mine in the tree is the ignored copy `.out/adv-mut/` (38 MB) used for the planted changes. Nothing is committed or pushed. My logs, probe sources and `mutants.sh` are in `<scratch>/wave2/classsyntax/adversary-1/`.
