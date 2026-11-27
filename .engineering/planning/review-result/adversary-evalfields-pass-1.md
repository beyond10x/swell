---
format: aep.planning-md/3
id: review-result:adversary-evalfields-pass-1
kind: review-result
status: active
title: Adversary pass 1 on the unit evalfields
relations:
- reviews: story:eval-in-class-field-initializers
revision: 2
---
Adversary pass 1 on the unit evalfields, branch impl/evalfields at 5451f25, 2026-11-27.
Verdict green: no failing case; three mutants survive the tests, one judgement note.

unit: evalfields (story:eval-in-class-field-initializers)
verdict: green
cases: executed 13→13, red 0
origin: introduced 4, pre-existing 0, undecided 0
wrote-outside-worktree: <scratch>/wave2/evalfields/adversary-1/
needs-coordinator: yes. Three single-line changes to the unit's code pass every test in `swell::main` and `swell-syntax::main` (1640 tests). The unit should add the three tests below.

Nothing broke on the unit's code. No probe gave a value where the spec requires a SyntaxError, and none hung or panicked. The tree is clean at `5451f25`: I left no case in it, because none fails on HEAD. My probe versions of the test files are `class_field_eval.rs.probes` and `sources.rs.probes` in the work directory.

## Findings

1. **mutant, warning, introduced by `17de75e`.** `crates/swell/src/dynamic_code.rs:373` changed to `in_class_field_initializer: in_initializer || in_method` passes the 13 tests and all 1640 tests of `swell::main` + `swell-syntax::main` (exit 0). No test runs `eval("arguments")` in a method. The removed test held `m() { return eval("3") }` and `static n()` on a class with fields; none of the 13 holds that statement.
   - Catching test (red under the mutant): `both_pass("class C { f = 1; m() { return eval(\"arguments.length\"); } static n() { return eval(\"arguments.length\"); } }\n var o = { m() { return eval(\"arguments.length\"); } };\n new C().m(1, 2) === 2 && C.n(1) === 1 && o.m(1, 2, 3) === 3;")`

2. **mutant, warning, introduced by `17de75e`.** `crates/syntax/src/early.rs:365` with `| FunctionKind::Declaration` added to the kinds looked into passes all 1640 tests (exit 0). No test has a function declaration that reads `arguments` inside eval code.
   - Catching test (red under the mutant): `both_pass("class C { x = eval(\"function f() { return arguments.length; } f(1, 2)\"); }\n new C().x === 2;")`

3. **mutant / contract, note, introduced by `17de75e`.** `crates/syntax/src/early.rs:382` with `m.credit(bytes)` removed passes the 13 tests. No test in `swell-syntax` sets `in_class_field_initializer`. Under the mutant the tree holds 1,024 bytes it does not own (charged 12,489, `capacity_bytes()` 11,465), which breaks the stated rule that after the parse the ledger holds exactly the tree. The engine credits `charged` when it drops the tree, so the heap does not leak.
   - Catching test in `sources.rs`, a ledger that counts charges and credits: `parse_eval_with` of `"0, "×120 + "0"` with the flag set, then `assert_eq!(held, script.charged); assert_eq!(script.charged, script.capacity_bytes());`

4. **judgement, note, introduced by `17de75e`.** `crates/syntax/src/lib.rs:251`: the walk runs before the early errors, so a held refusal is hidden.
   - `class D { static {} }; arguments` (also with `#p = 1;` or `#m() {}`) in an initializer's eval gives the Grammar error with refusals +0; the same source in a function's eval gives the refusal, +1.
   - This matches the parser's own rule (`parser/mod.rs:531-535`: a held refusal is answered only when there is no grammar or early error), and the spec lets the checks run in any order.
   - In eval code, a class's own `arguments` error ("arguments in a class field initializer" or "arguments in a class static block") is replaced by the eval's message. Both are SyntaxErrors.
   - Nothing in the 84 cases reaches this: their private names are in the outer class, which the outer parse refuses.

## What each attack found

1. **ContainsArguments.** 44 sources that must fail, each in four places (instance field, static field, arrow, arrow two deep), plus a control run in a plain function. 26 sources that must pass. All match the clause in both builds.
   - Every computed key fails as it should: object method, getter, generator and async method; class method, static method and getter; static field key; field key. So does the heritage.
   - `var arguments`, `let arguments`, `(arguments) => 1` and `function arguments(){}` give "arguments may not be bound in strict mode code". These are BindingIdentifiers, so ContainsArguments is false and the strict-mode rule gives the error. test262 has no such case for an initializer eval; its `declare-arguments` cases are sloppy functions in `language/eval-code/direct`.
   - `arguments: 1` (a label) answers 1.
   - The escaped forms `\u0061rguments` and `argument\u{73}` fail; `{ \u0061rguments: 1 }` as a key passes.

2. **Which evals get the flag.** 43 cases, all as the spec says, with identical output in the collect build.
   - An arrow kept past construction keeps the flag, also when called from a function or with `.call`.
   - An async arrow keeps the flag after `await` (SyntaxError), and `this`, `super` and `new.target` read correctly there.
   - Derived classes: the initializer runs when `super()` returns (log `pre,B:false,init,post:1`). With a base constructor that returns another object `o`, the eval's `this === o` and `eval('arguments')` is a SyntaxError.

3. **Stops inside the walk.**
   - Syntax level, each charge refused once: 262 points with `arguments` (261 `Stopped(OverBudget)` holding 0, 1 Errors). 264 points without it (263 Stopped, 1 Ok with charged = held = capacity = 11,465).
   - Interrupt at each poll of a 6,000-node source: 5 points (4 `Stopped(Interrupted)`, 1 Errors).
   - Engine, step budget: 480 points, 0 to 479 (477 StepBudget, 3 SyntaxError).
   - Engine, memory budget: 769 points in 8-byte steps over [407,386, 413,530] (742 MemoryBudget, 27 SyntaxError).
   - Never a value, and `executed` stayed false at every point.
   - Credit: heap 404,800 → 404,800 and live 294,033 → 294,033 after 1,000 failing and 1,000 passing evals and a collection (default build). The same probe in the collect build was killed at 60 s; that is my probe's cost, not the unit's.

4. **Order of errors.** With the flag set, the `arguments` error wins over a duplicate `let` in both orders (1:15 and 1:1) and over a regexp early error (1:6). A parse error stays the parse error: `arguments +` gives "unexpected end of input (1:12)". A SuperCall stays the SuperCall error. Refusals: finding 4.

5. **Nested eval and the limits.**

   | depth | result | steps | wall, load ~20 | wall, second run |
   |---|---|---|---|---|
   | 100 | RangeError 97 | 17,072 | 15 ms | 42 ms |
   | 300 | RangeError 297 | 141,173 | 52 ms | 115 ms |
   | 1,000 | RangeError 997 | 1,520,523 | 269 ms | 467 ms |

   - Steps per level² come to about 1.5. A name lookup charges one step per record (`docs/design/engine.md:358`), so each step stays a bounded amount of work.
   - Test limits (1,000,000 steps, `frame_depth` 10,000): the run ends in `Terminated(StepBudget)` at 1,000,000 steps, directly and through an arrow. This is why the unit raised the budget to 50,000,000: depth 1,000 needs 1.52M steps.
   - Memory budget of the realm + 64 KiB: `MemoryBudget` at level 31; + 256 KiB: at level 128. The next entry runs.
   - Native stack: four scripts (nested eval; `x = eval('new C()')` directly and through an arrow; a static field that makes a new class) on 12,838,560 bytes, seven eighths of `min_stack_bytes()` at re-entry 1,000 and dev profile. No overflow and no EngineFault.
   - I measured wall time in-process, not CPU seconds.

6. **Collector** (collect build): an eval that allocates between parse and run → 69; an arrow kept past construction, then evaluating → 7 and SyntaxError; nested classes with fields inside eval code → 6 and 3.

7. **Times** (CPU s, user + system, `--exact --test-threads 1`; `SWELL_FULL_COUNTS=1` gives the same numbers, since no test reads it). All within bounds.

   | build | max | other tests | load on 32 cores |
   |---|---|---|---|
   | default | nested 0.415 | 0.013–0.019 | 21.6–25.5 |
   | lane collect (opt-level 0) | interrupt 1.081 | nested 0.990, step budget 0.519, rest ≤0.265 | 16.8–18.1 |
   | gate collect (opt-level 1) | interrupt 0.093 | nested 0.087, rest ≤0.046 | 3.1 |

8. **Planted defects**, all red:
   - (a) `in_class_field_initializer: false`: 4 red. `eval_of_arguments…`, the static-field test, the arrow test, and the nested test (`ReferenceError 50` against `SyntaxError 50`). The unit's reading said 3.
   - (b) arrows not looked into: 2 red (`eval_of_arguments…`, the static-field test).
   - (c) every function looked into: 1 red (`arguments_in_a_function_inside_the_eval_is_allowed`).
   - (d) the walk run only when `early::check` found errors: 4 red, the same four as (a).
   - The three survivors are findings 1 to 3.

9. **The removed test.** Its indirect evals are held by `indirect_eval_of_arguments_reads_the_global_binding`. Its direct `eval("1")` in instance, static and arrow positions is held by several tests. Its method and static-method evals are not held (finding 1).

## Cases added

None left in the tree. Probes and catchers were in `crates/swell/tests/class_field_eval.rs` (module `adversary`) and `crates/syntax/tests/sources.rs` (module `adversary_contains_arguments`); I restored both with `git show HEAD:<path> > <path>`.

## Commands (all exit 0 unless noted)

- `cargo nextest run --locked -p swell-syntax --test main -E 'test(/adversary/)' --no-capture` → `1 test run: 1 passed, 79 skipped`
- `cargo nextest run --locked -p swell --test main -E 'test(/^class_field_eval::adversary::/)' --no-capture` → `10 tests run: 10 passed, 1562 skipped`
- the same subset with `--features collect-every-safe-point` → exit 100, `6 tests run: 5 passed, 1 timed out` (the credit probe)
- baseline `cargo nextest run --locked --no-fail-fast -p swell -p swell-syntax --test main -E '…'` → `17 tests run: 17 passed, 1639 skipped`
- mutants a to g, the same command (exit 100 each) → a `12 passed, 4 failed`; b `14/2`; c `14/2` (one of the two red tests is my catcher); d `12/4`; e `15/1`; f `15/1`; g `15/1` (the single reds in e, f and g are my catchers)
- mutants f and g with `-E 'not test(/adv_/)'` → `1640 tests run: 1640 passed, 16 skipped` each
- `slot --timeout 30m sh -c "<time-tests.sh>"` → exit 0, every run `rc 0`. The first try with the script path was refused by slot (a file under the scratch directory of the session).

## Not attacked

- test262: the 84-case part run, not repeated.
- The interaction with `privatemethods` after it merges.
- The stack lanes in the release and `stack-debug` profiles (dev profile only).
- CPU seconds for the depth sweep (wall time only).
- The heap-credit check in a collect build (killed at 60 s).

Logs, mutant files and scripts are in `<scratch>/wave2/evalfields/adversary-1/` (`probe1.log`, `probe2-*.log`, `syntax-probe.log`, `baseline.log`, `mutants.out`, `mutants-full.out`, `mutant-*.log`, `time.txt`, `mutants/`).

```findings
- file: crates/swell/src/dynamic_code.rs
  line: 373
  category: mutant
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: Setting the flag to in_initializer || in_method passes all 1640 tests of swell::main and swell-syntax::main because no test evaluates arguments in a method, and the removed test's method evals are held by none of the 13.
- file: crates/syntax/src/early.rs
  line: 365
  category: mutant
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: Looking into FunctionKind::Declaration passes all 1640 tests because no test has a function declaration reading arguments inside eval code in an initializer.
- file: crates/syntax/src/early.rs
  line: 382
  category: mutant
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: Removing the credit of the walk's stack passes the 13 tests since no swell-syntax test sets in_class_field_initializer, while the tree then holds 1024 bytes over its capacity_bytes.
- file: crates/syntax/src/lib.rs
  line: 251
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: The walk's Grammar error hides a held refusal (refusals +0 against +1 in a function's eval) and replaces a class's own arguments message in eval code, which the parser's hold rule and the spec's free order allow and no listed test262 case reaches.
```
