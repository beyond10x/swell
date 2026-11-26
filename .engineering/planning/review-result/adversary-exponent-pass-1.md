---
format: aep.planning-md/3
id: review-result:adversary-exponent-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on unit exponent of wave big-1: green, 1 note, 12 cases added'
relations:
- reviews: story:exponentiation-operator
revision: 1
---
Adversary pass 1 on unit `exponent` of wave big-1 (`story:exponentiation-operator`), the tree `impl/exponentiation` at `b7d34305`, 2026-11-26.
Verdict green: 12 cases added, none red, 1 finding (a note on a test of the unit). The report as the adversary returned it, with the paths of the workstation replaced:

I found no defect: the grammar, the evaluation order and the depth bounds all matched ECMA-262 and the engine's bounds. All 12 cases I added are green, and the one finding is a note about a weak test.

```
unit: exponent (story exponentiation-operator), branch impl/exponentiation at b7d34305 plus my uncommitted test files
verdict: green
cases: executed 179→190, red 0
origin: introduced 1, pre-existing 0, undecided 0
wrote-outside-worktree: 7 paths (part 6)
needs-coordinator: no
```

**1. What I changed in the worktree**

`git --no-pager diff --stat` shows only the three generated `tests/main.rs` files, each with one new `mod` line, written by `sh taskfiles/test-main.sh write`:
- `crates/compiler/tests/main.rs`
- `crates/swell/tests/main.rs`
- `crates/syntax/tests/main.rs`

The three new files are untracked. All six paths are test files; I changed no implementation file.

**2. The cases I added (all green now)**

- `crates/syntax/tests/exponentiation_attack.rs`, five tests:
  - 80 sources the grammar accepts all parse. Among them: `(-a) ** b`, `(await a) ** b`, `await ** 2` as a plain identifier, `a ** -b`, `a ** typeof b`, `a++ ** b`, `++a ** b`, `new a ** b`, template and optional-chain operands, `yield a ** b`, `async () => a ** b`, `a ?? b ** c`, `a ** b ?? c`, `in` and `instanceof` neighbours, and a regex after `**` and after `**=`.
  - 54 sources the grammar refuses all fail. Among them: every unary operator and `await` as the left operand, `-(a) ** b`, `a ** -b ** c`, `a * -b ** c`, an arrow function as the right operand, `a ** b = c`, `a?.b **= c`, patterns, strict `eval` and `arguments`, `yield ** 2` in a generator, and `(x = -a ** b) => x`.
  - `**=` is accepted or refused exactly where `*=` is, over 35 targets in sloppy and strict code.
  - 24 tree shapes are right.
  - A chain of 100,000 `**`, either way round, and of 5,000 `**=` fails with the one nesting error.
- `crates/compiler/tests/exponentiation_attack.rs`: the deepest chains the parser accepts at a nesting limit of 1,000 compile on a 256 KiB stack, with one `Exp` per operator. That is 997 `**`, 494 `**=`, and a chain of member operands.
- `crates/swell/tests/exponentiation_attack.rs`, five tests:
  - 42 values of `**` written as literals from the spec table, not compared with `Math.pow`.
  - Symbol operands throw TypeError on either side, the left operand's `valueOf` runs first, and `Symbol.toPrimitive` gets the hint `number`.
  - In `f(a) ** f(b) ** f(c)` the conversions run in the order b, c, a.
  - `**=` through `with` with a getter and setter, on a frozen object (sloppy and strict), on a `const`, on an undeclared name and on `null.p`.
  - A chain of 990 `**` runs to its value through a script, `eval` and `Function`.

Nothing went red against the implementation. My first run failed once, on my own bound: I had assumed more than 500 `**=` would fit, but each assignment costs two nesting levels. I lowered the bound to 400; that was my error, not a finding.

**3. The suite runs**

These ran after the cases existed: the whole `main` binaries of swell-syntax and swell-compiler, plus the swell filter for `exponentiation`, `operators`, `builtins_math`, `budgets`, `termination`, `interrupt`, `frontend_ledger`, `eval_stack` and `limits_scripts`.

| Run | Tests | Result | Exit |
|---|---|---|---|
| Default mode | 190 | all passed | 0 |
| Default mode without my files | 179 | all passed | 0 |
| `collect-every-safe-point` (swell filter only) | 118 | all passed | 0 |

Timing of my new tests: none passes 5 s. The slowest is `the_deepest_exponent_chain_runs` at 3.7 s wall time in collect mode, under parallel load.

**4. Finding**

`crates/swell/tests/exponentiation.rs:321`, `an_interrupt_between_two_operations_ends_the_loop`: the interrupt is set inside the host function `interruptNow`. The run then ends at the check made after every host function returns (`crates/swell/src/interp/budget.rs:10-14`, `check_interrupt_after`), before the next `**` and before any loop jump.
- So the test cannot fail if the interrupt check at the loop jump goes missing.
- Its iteration count does not matter: the 50 iterations in collect mode prove exactly what the 20,000 in default mode prove.
- Nothing reaches it beyond the test itself. Origin: introduced (the unit's test).

**5. What I attacked and could not break**

- **Grammar against `#sec-exp-operator`:** everything on your list, plus ASI and cover-grammar forms. The check in `exponent_base`, and the `precedence + 1` in `binary_step`, are right.
- **Equivalent mutant:** giving `Exp` precedence 10 instead of 11 behaves the same, because the `+1` bound never reduces a pending `*`. This is not a defect.
- **`**=`:** the targets match `*=`. The order is reference, then the right side, then `GetValue` once, then left before right, then `PutValue`, including `with` and getter/setter targets.
- **Evaluation:**
  - The spec table matches as literals.
  - Symbols throw TypeError.
  - A BigInt is unreachable: the lexer refuses BigInt literals, nothing in `crates/swell/src` builds a `Value::BigInt`, and no `BigInt` global exists. So the `self.fault()` path is never reached.
- **Step charge:** `Exp` costs the same as `Mul`, and the comparison test can fail. Comparing 1,000 iterations instead of 20,000 still shows equal steps per operator, because steps grow linearly with iterations.
- **Depth:**
  - The compiler and the resolver walk with explicit work lists, so right-deep trees do not grow the Rust stack.
  - The operand stack is charged and bounded by `parser_nesting`.
  - Collect mode at 50 iterations still spends more than 1,000 steps, so the step-budget case can fail.
- **Leftover text and panics:**
  - The stale-text grep finds nothing in `crates/syntax/src` or its tests.
  - The new parser code has no `unwrap`, indexing or `as` cast.
  - The operator code has no `std::` path or float method.

**6. Paths I wrote outside the worktree**

All in `<scratch>/wave/exponentiation/adversary-1/`:
- `suite.sh`
- `syntax-compiler-cases.log`
- `swell-cases.log`
- `suite-default.log`
- `suite-default-without.log`
- `suite-collect.log`
- `suite-status.txt`

**7. Findings block**

```findings
- file: crates/swell/tests/exponentiation.rs
  line: 321
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the interrupt is set in a host function and ends the run at the check after that function returns, so the test passes whether or not a loop jump or anything near the operator checks the interrupt, and its iteration count (50 or 20,000) changes nothing it proves
```
