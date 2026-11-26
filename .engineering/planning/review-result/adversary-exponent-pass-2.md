---
format: aep.planning-md/3
id: review-result:adversary-exponent-pass-2
kind: review-result
status: active
title: 'Wave big-1, unit exponent, adversary pass 2: green, 2 notes'
relations:
- reviews: story:exponentiation-operator
revision: 1
---
Adversary pass 2, the last, on unit exponent of wave big-1 (story:exponentiation-operator, impl/exponentiation at 3771d526), 2026-11-26.
Verdict: green; 8 mutants planted in a copy, 7 caught by committed tests, 1 survived and got a case; 2 notes. The report, as returned:

I found no defect in `exponent` at `3771d526`. The interrupt correction holds in both modes, and 7 of the 8 mutants I planted were caught by a committed test. Two findings, both notes: one mutant at the operator survived (I added a case that catches it), and one chain in the bounded-chain test is listed twice.

```
unit: exponent (story:exponentiation-operator), impl/exponentiation at 3771d526 plus one uncommitted test function
verdict: green
cases: executed 85→86, red 0
origin: introduced 2, pre-existing 0, undecided 0
wrote-outside-worktree: 1 directory with 23 files and the copy (part 6)
needs-coordinator: no
```

**1. Worktree diff**
`crates/swell/tests/exponentiation.rs | 18 ++++++++++++++++++`. This is a test file. I changed no implementation file and committed nothing.

**2. Case added**
`crates/swell/tests/exponentiation.rs:520`, `adversary_a_base_inside_minus_one_and_one_to_an_infinite_exponent`.
- It asserts that `(-0.5) ** Infinity`, `(-1e-300) ** Infinity` and `(-0.999) ** Infinity` are +0, gives a few neighbouring rows, and compares two of them with `Math.pow` using `Object.is`.
- It passes on the tree: 0.016 s in default mode, 0.099 s in collect mode.
- It fails against mutant M5. I ran it alone in the copy, exit 100. The output: `left: Normal("\"-0,-0,-0,0,Infinity,Infinity,false,true\"")`, `right: Normal("\"0,0,0,0,Infinity,Infinity,true,true\"")`.

**3. Suite runs** (in the worktree, after the case existed; all ran on the build node)

| Run | Tests | Exit |
|---|---|---|
| Default mode: `swell-syntax`, `swell-compiler` `main` binaries and `exponentiation::` | 86 passed | 0 |
| Default mode, same filter without `test(/adversary_/)` | 85 passed | 0 |
| Collect mode (`collect-every-safe-point`), `exponentiation::` | 14 passed | 0 |

**4. Findings**
- **F1, mutant at the operator survives.** Mutant M5 is in the copy's `crates/swell/src/ops/operators.rs:53`: the operator answers -0 for a base in (-1, 0) raised to +Infinity. No committed test failed: `exponentiation::` passed 13 of 13 (exit 0) and the lib `operators` tests passed 3 of 3 (exit 0). The 120 pairs have no base strictly between -1 and 1 apart from the zeros, and the table of literal values has no negative base with +Infinity.
  - What reaches it: nothing found. The operator is one call, and swell-num's own table (`crates/num/tests/math_exp_log.rs`) holds that row for the shared function. Origin: introduced (the unit's tests).
- **F2, one chain listed twice.** `crates/syntax/tests/sources.rs:1878-1879`: the two chains of 100,000 `**` that the doc comment calls "written either way round" build the same text byte for byte (`"x = a" + " ** a"*N + ";"` equals `"x = " + "a ** "*N + "a;"`). The second line tests nothing new and costs one more parse of 100,000 operators. Origin: introduced.

**Mutants**

| Mutant | Planted in the copy at | Committed test that failed (exit status) |
|---|---|---|
| M1: no interrupt check at the loop jump | `crates/swell/src/interp/dispatch.rs:979` | `exponentiation::an_interrupt_from_another_thread_ends_a_loop_of_the_operator`, default mode: StepBudget at 4,000,000 steps after 6.7 s, exit 100. Collect mode: StepBudget at 20,000 steps after 4.6 s, exit 100 |
| M2: `**` left associative | `crates/syntax/src/parser/expression.rs`, `binary_step` | syntax: `sources::exponentiation_is_right_associative_and_binds_tightest`, `sources::exponentiation_tree_shapes` (exit 100). swell: `right_associativity_and_precedence`, `the_conversions_of_a_chain_run_inner_operator_first`, `values_of_the_clause_written_as_literals` (exit 100) |
| M3: `**=` compiled as `a = a ** b` (target evaluated twice) | `crates/compiler/src/expression.rs`, `AssignOp::Compound` | `exponentiation::compound_assignment_targets` (exit 100). Syntax and compiler tests passed, as expected |
| M4a / M4b / M4c: no refusal of an `await` / `delete` / `typeof` base | `exponent_base` | each fails `sources::exponentiation_sources_the_grammar_refuses_fail`, `sources::early_error_sources_fail_with_their_message`, `sources::message_kind_tells_refusals_from_early_errors`. M4c also fails `exponentiation_is_right_associative_and_binds_tightest`. All exit 100 |
| M5: (-0.5) ** Infinity gives -0 | `crates/swell/src/ops/operators.rs:53` | no test failed (F1) |
| M6: a second step per `**` | `dispatch.rs`, `I::Exp` arm | `exponentiation::a_loop_of_the_operator_costs_the_steps_of_multiplication` (exit 100) |
| M8: a negative base with a non-integral exponent gives the negative real root | `operators.rs:53` | `right_associativity_and_precedence`, `values_of_the_clause_written_as_literals`, `each_pair_answers_as_math_pow` (exit 100) |
| BigInt mixed with Number; negative BigInt exponent | not planted | Nothing to mutate: no `Value::BigInt` is ever built, the lexer refuses BigInt literals, and the story puts the BigInt arm out of scope |

**5. What I attacked and could not break**
- **Interrupt budget before 100 ms:** it cannot run out first. The mutant runs measured about 600,000 steps/s in default mode and about 4,300 steps/s in collect mode. That leaves a margin of about 66x and 46x over 100 ms. Load slows the loop, which only widens the margin.
- **Interrupt test duration:** with the mutant planted it ends in 6.7 s (default) and 4.6 s (collect), well under 60 s. On the tree it takes 0.12 s in both modes.
- **Thread join:** the thread is joined on every path that does not panic. A panic before the join ends the test's own process (nextest runs one process per test), and the thread ends by itself after 100 ms.
- **Untested residue:** if the main thread stalled for 100 ms or more between `spawn` and the start of the entry, `begin()` would end the run with 0 steps, and the test would fail at `spent > 0` (`exponentiation.rs:338`). The work in that window is microseconds of CPU.
- **Item 2, the demanded errors:** chain 1 ends in the unary-base error at its second `**`. That is what the grammar says: `a ** -a` is valid, and the next `**` has the unparenthesized `-a` as its left operand, long before depth 1,000. Chains 2 to 4 are otherwise valid and end in the nesting error only. None of the demanded errors is an accident.
- **Item 3, moved cases:** the five swell cases match the saved `swell-cases.rs` assertion for assertion; only the names and doc comments changed. Syntax and compiler: 35 targets, 24 shapes, 54 refused sources and the `n > 400` bound all match the pass-1 record. The admitted list has 79 sources against the "80" in the pass-1 report. The pass-1 syntax file was not saved, so I cannot check which one, if any, was dropped.
- **Item 5, collect-mode reductions:** in the copy, the deepest chain through script, eval and Function passed in collect mode (2.3 s), and so did both loops at 500 iterations (3.7 s and 6.2 s), exit 0. Nothing shows up in the unreduced form that the reduced form misses.
- **Where the copy's jobs ran:** the pool built and ran them on the workstation, not on the build node (no sync line; the copy's `target/` was 1.2 GB). Its path under the home cache directory is the likely reason. I deleted that `target/`.

**6. Paths written outside the worktree**
In `<scratch>/wave/exponentiation/adversary-2/`:
- the clone `copy/`, at `3771d526` with a clean tree, its `target/` deleted
- scripts: `m1.sh`, `mutants.sh`, `reductions.sh`, `case-on-m5.sh`, `suite.sh`, `case.patch`
- logs: `status.txt`, `m1-default.log`, `m1-collect.log`, `m2-syntax.log`, `m2-swell.log`, `m3-syntax.log`, `m3-swell.log`, `m4a-syntax.log`, `m4b-syntax.log`, `m4c-syntax.log`, `m5-swell.log`, `m5-lib.log`, `m6-swell.log`, `m8-swell.log`, `reductions-collect.log`, `case-on-m5.log`, `suite-default.log`, `suite-default-without.log`, `suite-collect.log`

**7. Findings block**
```findings
- file: crates/swell/tests/exponentiation.rs
  line: 520
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: an operator that answers -0 for a base in (-1, 0) to +Infinity passes every committed test, since the pairs hold no base strictly inside (-1, 1) but the zeros and the literal table no negative base with +Infinity; the added case fails on that mutant
- file: crates/syntax/tests/sources.rs
  line: 1879
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the second chain of 100,000 ** builds the same text byte for byte as the first, so the "either way round" of the doc comment tests one shape twice and costs one more parse of 100,000 operators
```
