---
format: aep.planning-md/3
id: review-result:slice-12-round-2-acceptance
kind: review-result
status: active
title: 'Slice 12, round 2, the acceptance critic: approve, 5 notes'
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
Plan critic, acceptance, round 2 on the eight draft stories of
`epic:slice-12-bigint` (`plan/slice-12` at `82dde80a`), 2026-11-28.
Verdict: approve, 5 notes; revision 2 answers them. The findings block is brought into the store's form by the coordinator; its text is the critic's. The critic's answer, as returned:

approve

Round-1 findings of my role:
1. settled: the close writes the two `omit` lines, with `sections.txt` in its scope and Writes (`slice-12-conformance-close.md:59`, `:64`), and greps each for `1` (`:68`). Core's sentence names the same two lines (`swell-num-bigint-core.md:41`). The form matches the file: line 14 is `path sec-bigint-objects test/built-ins/BigInt/` and line 95 is the `string-length-limit` line.
2. settled: `bigint_operation` charges in the order limit, steps, memory (`bigint-arithmetic-and-bitwise-operators.md:47`). That is the order of `Agent::concatenate`, `operators.rs:133-145`, quoted correctly. The test names `steps_per_entry` 2^40 and shows the numbers (`:49`). I checked `pow_bits` (16,777,217, 33,554,432), 2^38 = 274,877,906,944 and 262,145 limbs × 8 = 2,097,160 bytes. Finding 1 below concerns one figure.
3. settled: `bigint-comparison-and-loose-equality.md:37` says "at least 9,999", with the reason.
4. settled: `pow_cost` and `pow_bits` are constants for bases 0, 1 and −1 (`swell-num-bigint-powers-shifts-and-bits.md:42-43`). The new test is at `:48`, and the arith test reads fewer than 1,000 steps (`bigint-arithmetic-and-bitwise-operators.md:49`).
5. settled: core names `schoolbook_mul_count` and `karatsuba_mul_count` (`swell-num-bigint-core.md:60`). The test `the_count_functions_equal_the_counter` ties them to the counter (`:66`). The greps are at `:79`. Both greps hold on the base: the threshold grep gives 0, and the sentence grep gives 1 at `engine.md:2561`. Finding 4 below concerns the line number.
6. settled: `a_reservation_that_cannot_be_met_is_err_alloc` (`swell-num-bigint-core.md:66`), `a_left_shift_longer_than_64_bits_is_err_alloc` (powers `:48`) and `a_reservation_the_allocator_refuses_ends_in_memory_budget` (arith `:49`). Check of the numbers: 2^61 limbs = 2^64 bytes is a capacity overflow, and a shift of 2^63 gives 2^57 + 1 limbs = 2^60 + 8 bytes. Finding 3 below concerns the grep.
7. settled: `a_long_multiplication_is_charged_per_limb_operation` has a floor of 100,000 steps. 2000^(log2 3) = 170,612, so the floor holds, and the 1 by 1 product costs at most 2·1·1 + 64 = 66.
8. settled: six `cstm` cases (`bigint-constructor-and-prototype.md:41`) and 35 cases (`bigint-json-number-and-joined-cases.md:28`: 33 + 2 = 35, and 6 + 1 + 1 = 8, with 35 + 8 = 43).
9. settled: every story's Acceptance asks its report for the sum of its new tests' CPU seconds. The close adds no test.
10. settled: the awk of the close (`:68`). On `f962c324` against itself it prints 0 lines. On a copy with one directory's passed count lowered by one it prints that line. With a `close-into-12.tsv` line naming that slice it also prints it, because that line sits in a directory, not a slice line.
11. settled as ruled: each of the five test262 stories says the unit names the part run's wall seconds, and keeps the split into halves.

New findings:

1. `story:bigint-arithmetic-and-bitwise-operators`, `:49` (the memory test). `note`.
   - What is wrong: the figure "183,254,036,694" is reproducible only with L = 1 for squarings 1 to 5. The stated rule "L ≤ 2^k/64 + 1" gives 183,254,036,698.54 for k = 1..24 (`bc`), and with integer ceilings 183,254,036,724.
   - A second gap: core says costs are "a function of operand lengths alone" (`swell-num-bigint-core.md:59`), so `pow_cost` may also count one multiplication per exponent bit. That adds about 10^6 steps to the figure, and the body says "at most" it.
   - The conclusion is unaffected, since it is below 2^38 with a margin of 1.5 times. The stated equation is still not reproducible.
   - What would settle it: say "L = 1 for k < 6" and name the multiplications of a length-only cost, or state "below 1.84 × 10^11".

2. `story:swell-num-bigint-powers-shifts-and-bits`, `:43` and `:48`. `note`.
   - What is wrong, part one: the formula for the upper bound of `pow_bits`, `base_bits × e`, gives 0 for e = 0, but `x ** 0n` is `1n` with 1 bit. A unit that follows the text fails its own test, since test 1 covers exponents from 0.
   - What is wrong, part two: `bit_bounds_enclose_the_result` is named with no pairs, while its sibling in core says which pairs. Which operations and pairs it covers is not observable.
   - What would settle it: add "and 1 for e = 0" to the upper bound, and name the pairs of the test (the pow pairs of test 1, the shifts of test 4, the bitwise pairs of test 6, `as_n` for bits 0 to 130).

3. `story:swell-num-bigint-core`, `:55` and `:61`, with the grep at `:79`. `note`.
   - What is wrong: the acceptance wants `try_reserve_exact` once, in `limbs_with_capacity` (`-> Vec<u64>`), and no `vec!`, `Vec::with_capacity` or `.push(` in `bigint.rs` and `bigint/`.
   - `to_radix(..) -> Vec<u8>` is a buffer whose length an operand sets, and its signature has no `Result`. It cannot be made by a `Vec<u64>` function, nor without `push`, `with_capacity` or an infallible `resize` or `extend`. As worded, the rule and the acceptance cannot both hold.
   - What would settle it: state how the byte buffer is made, for example a generic reservation function with the same single `try_reserve_exact` and `to_radix` returning `Result<Vec<u8>, Alloc>`, or a caller-supplied buffer.

4. `story:swell-num-bigint-core`, `:79` and `:65`. `note`.
   - What is wrong: the sentence the grep counts is on line 2561 of `engine.md`; line 2560 holds only "advance to exceed ... (whether". The count of 1 on the base is right, but the cited line is off by one.
   - What would settle it: cite 2561, or "2560 to 2561".

5. `story:bigint-comparison-and-loose-equality`, `:39`, and `story:bigint-constructor-and-prototype`, `:59`. `note`.
   - What is wrong: Bounds states that a String of 400,000 decimal digits against `1n` under a small `bigint_bits` throws a `RangeError`, and that NumberToBigInt and StringToBigInt are limited. No test in either story shows it. The constructor's only limit test is `asUintN`.
   - What would settle it: add one assertion each, for example inside `relational_operators_with_strings` and `the_constructor_throws_where_the_specification_says`, with `bigint_bits` set.

Checked and right:
- `lists.sh` over the base run prints `slice 381 lists 381 incoming 0` (exit 0).
- The part cases and runs are 392/768, 1,275/2,373, 1,279/2,381, 398/794 and 341/681.
- `check.sh` over the restricted base run gives 213, 84, 92, 199 and 86 `failing` lines with the stated `passed … B F R` lines.
- The `(b)` counts are 213 and 50.
- 682 non-passing runs in slice 12 and 8 runs of the 4 slice-11 cases, as stated.
- The 16 `before slice 12` lines in `crates` are all owned (3 arith, 3 joint, 1 constructor, 9 values).
- The HOST lines 23, 24 and 219, the 349 lines and 0 BigInt lines of `realm_places.txt`, the `omit` form and `validate` ending in `valid`.
- The waves put the three order-3 stories in one wave (5 waves).
- Test counts per story: 16, 11, 10 + 1, 6, 11, 13 + 2 and 6, each matching its list.
- `limits.rs`/`last_entry_steps` do not forbid `u64::MAX` limits.

Out of my lane: the missing edge to `story:slice-11-conformance-close` (the coordinator adds it), the collision between the constructor and joint stories on `host/limits.rs` (an edge joins them), and values writing about 24 files, 14 of them shared with slice 11 (scope and parallel-safety).

```findings
[{"file": ".engineering/planning/story/bigint-arithmetic-and-bitwise-operators.md", "line": 49, "category": "acceptance", "severity": "note", "verdict": "approve", "origin": "introduced", "message": "the figure 183,254,036,694 in the memory test is not reproducible from the stated rule L <= 2^k/64+1 (gives 698.54, or 724 with ceilings) and ignores possible multiplications of a length-only pow_cost; conclusion below 2^38 unaffected"},
 {"file": ".engineering/planning/story/swell-num-bigint-powers-shifts-and-bits.md", "line": 43, "category": "acceptance", "severity": "note", "verdict": "approve", "origin": "introduced", "message": "pow_bits upper bound base_bits x e is 0 at e = 0 but the result 1n has 1 bit, so a unit following the text fails test 1's range; bit_bounds_enclose_the_result names no pairs or operations"},
 {"file": ".engineering/planning/story/swell-num-bigint-core.md", "line": 61, "category": "acceptance", "severity": "note", "verdict": "approve", "origin": "introduced", "message": "the acceptance (one try_reserve_exact in limbs_with_capacity -> Vec<u64>, no push/with_capacity/vec!) cannot hold together with to_radix -> Vec<u8>, an operand-sized buffer with no Result in its signature"},
 {"file": ".engineering/planning/story/swell-num-bigint-core.md", "line": 79, "category": "acceptance", "severity": "note", "verdict": "approve", "origin": "introduced", "message": "the sentence the grep counts is on engine.md line 2561 (the sentence wraps), the body cites 2560"},
 {"file": ".engineering/planning/story/bigint-comparison-and-loose-equality.md", "line": 39, "category": "acceptance", "severity": "note", "verdict": "approve", "origin": "introduced", "message": "Bounds states a RangeError for a 400,000-digit String under a small bigint_bits (and the constructor states StringToBigInt and NumberToBigInt are limited) but no test shows it"}]
```
