---
format: aep.planning-md/3
id: review-result:slice-12-round-2-design
kind: review-result
status: active
title: 'Slice 12, round 2, the design critic: needs-revision, 3 findings (1 blocker)'
relations:
- reviews: story:swell-num-bigint-core
- reviews: story:swell-num-bigint-powers-shifts-and-bits
- reviews: story:bigint-values-and-literals
- reviews: story:bigint-comparison-and-loose-equality
- reviews: story:bigint-arithmetic-and-bitwise-operators
- reviews: story:bigint-constructor-and-prototype
- reviews: story:bigint-json-number-and-joined-cases
- reviews: story:slice-12-conformance-close
revision: 1
---
Plan critic, design, round 2 on the eight draft stories of
`epic:slice-12-bigint` (`plan/slice-12` at `82dde80a`), 2026-11-28.
Verdict: needs-revision, 3 findings, 1 of them a blocker (the bounds of `pow_bits` at an exponent of 0); revision 2 answers them. The critic's answer, as returned:

needs-revision

**Round-1 findings of the design review**

1. `story:bigint-constructor-and-prototype`, the hidden dependency on arith: settled. Test `as_int_n_and_as_uint_n` at `bigint-constructor-and-prototype.md:114` writes "every expectation a literal". The constructor has no edge to arith, and the other constructor tests use no binary operator.
2. `pow_cost` against a base of ±1: settled in text. See new finding 1 for a bound that is still wrong.
   - Powers `:125` gives constants for the bases 0, 1 and −1, and `:126` keeps `u64::MAX` for an absolute base of 2 or more.
   - The new powers test `pow_bits_and_pow_cost_of_bases_0_1_and_minus_1_ignore_the_exponent_length` uses a 65-bit and a 1,000-bit exponent.
   - Arith `exponentiation_rules` reads fewer than 1,000 steps.
3. The memory and step order in `bigint_operation`: settled, and the order now follows the engine. Arith `:161` quotes `Agent::concatenate`.
   - I checked `crates/swell/src/ops/operators.rs:133-145` on the base. It checks `string_length`, then `charge_steps(length)`, then `reserve_vec`. That is limit, steps, memory.
   - I recomputed the numbers of the acceptance-6 test:
     - `pow_bits` is (16,777,217, 33,554,432), under 2^30.
     - The sum over k = 1..24 of 2·(⌊2^k/64⌋+1)²+64 is 183,254,036,694, under 2^38 and under 2^40.
     - The memory charge is at least 262,145 × 8 = 2,097,160 bytes, against 1,048,576 left.
     - The outcome is `Terminated(MemoryBudget)`.
4. `story:bigint-values-and-literals`, dead code: settled.
   - Every function the values story adds has a caller in it: `negate` calls `bigint_check_bits`, `bigint_charge` and `new_bigint(…, true)`. ToString and the keys call the reader, `Instruction::BigInt` calls `new_bigint(…, false)`.
   - StringToBigInt moved to compare (`ops/testing.rs`) and the constructor (`builtins/bigint.rs`), each a file that story writes anyway.
   - `waves` shows no collision between the three order-3 stories.
5. `story:slice-12-conformance-close`, the omission lines: settled.
   - `sections.txt` is in the close's scope, Change and Writes (`:125`, `:130`), and the acceptance greps both lines.
   - The anchors exist: `sections.txt:14` (`path sec-bigint-objects …`) and `:95` (the `string-length-limit` line). Core's sentence at `swell-num-bigint-core.md:41` names the same two lines.
6. The generated `tests/main.rs`, as a trade-off: settled by the coordinator's ruling.
   - Neither `main.rs` is in any scope, and every body carries the "take either side and regenerate" sentence.
   - `aep plan artifact waves --status draft` prints 5 waves, with the three order-3 stories in wave 3.
7. The generator of the 2,000 pairs: settled. `crates/num/tests/support/pairs.rs` is in core's scope, Change and Writes. Powers loads it and does not change it.
8. The edge to the slice-11 close: kept as ruled. The coordinator adds it on the join, and the close has the sentence (`slice-12-conformance-close.md:130`).

**New findings**

1. `story:swell-num-bigint-powers-shifts-and-bits` — `.engineering/planning/story/swell-num-bigint-powers-shifts-and-bits.md:126` — `blocking`.
   - The upper bound of `pow_bits` for an absolute base of 2 or more, `base_bits × e`, is below the lower bound and below the true length when `e` is 0.
   - Example: base 5 has 3 bits and e = 0. The bounds are `((3−1)×0+1, 3×0)`, which is (1, 0), and the result `1n` has 1 bit.
   - The first powers test runs "exponents 0 to 70", so `bit_bounds_enclose_the_result` (lower ≤ `bit_len()` ≤ upper) fails on the formula as written.
   - The cost, built from the same upper bound, also sees an operand of length 0 at e = 0.
   - Settle by writing the upper bound as `max(1, base_bits × e)`, or by stating e = 0 as the constant (1, 1). Add exponent 0 and 1 for a base of 2 or more to the enclosure test.
2. `story:swell-num-bigint-core` — `swell-num-bigint-core.md:60` and `:66` — `note`.
   - Two tests need Karatsuba at lengths where `mul` uses schoolbook.
     - `the_count_functions_equal_the_counter` runs "for each method, lengths 1 to 4 × the threshold".
     - `the_karatsuba_threshold_is_where_its_count_falls_below_schoolbook` needs `karatsuba_mul_count(n, n)` for n below the threshold.
   - `mul` takes the method by length, and the body names no entry point that forces one method.
   - Settle by naming a `#[doc(hidden)] pub` function that runs one method at any length (it takes the threshold or the method as an argument). The counter equality is then tested on it.
3. `story:swell-num-bigint-core` — `swell-num-bigint-core.md:65` against `:79` — `note`.
   - Change says "the line of 'Not checked' on the allowance replaced by that finding". The acceptance says `awk '/^## Not checked/,/^## How the counts/' … | grep -c 'BigInt'` prints `0`.
   - A replacement that names BigInt inside "Not checked" fails the grep.
   - Settle by saying the line is deleted and the finding lives only in decision 6.

**What I checked and found right**
- **Graph and reads.** I walked 26 declared edges of the eight stories plus the outgoing edges of `slice-08-conformance-close` and the epic edges (`epic:slice-11-modules`, `epic:slice-13-binary-data`). There is no cycle and no chain. I traced about 14 acceptance reads to a producer in the set, and an edge records each, directly or transitively. `aep plan artifact validate` ends `valid`.
- **Counts.** I recomputed from the base run file: the five parts are 392/768, 1,275/2,373, 1,279/2,381, 398/794 and 341/681 cases and runs, with 213, 84, 92, 199 and 86 failing runs. The sums are 338 + 4 = 342 failing cases and 674 + 8 = 682 failing runs. The test counts are 16, 11, 10 + 1, 6, 13 + 2, 11 and 6. The 16 `before slice 12` lines each have an owner story. The design greps give `1`, `0` and `1` on the base, as the bodies say.

```findings
- file: .engineering/planning/story/swell-num-bigint-powers-shifts-and-bits.md
  line: 126
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "pow_bits upper bound base_bits x e is 0 for e = 0 while the result 1n has 1 bit and the lower bound is 1, so bit_bounds_enclose_the_result fails for the exponents 0 to 70 of the first test; write max(1, base_bits x e) and add exponents 0 and 1 to the enclosure test"
- file: .engineering/planning/story/swell-num-bigint-core.md
  line: 60
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the counter-equality test and the threshold test need Karatsuba at lengths below the threshold where mul takes schoolbook, and no entry point that forces one method is named; name a doc(hidden) pub function that runs one method at any length"
- file: .engineering/planning/story/swell-num-bigint-core.md
  line: 65
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Change says the Not-checked line is replaced by the finding while the acceptance expects 0 BigInt lines in that section; say the line is deleted and the finding lives only in decision 6"
```
