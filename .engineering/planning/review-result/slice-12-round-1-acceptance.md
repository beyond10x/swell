---
format: aep.planning-md/3
id: review-result:slice-12-round-1-acceptance
kind: review-result
status: active
title: 'Slice 12, round 1, the acceptance critic: needs-revision, 11 findings'
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
Plan critic, acceptance, round 1 on the eight draft stories of
`epic:slice-12-bigint` (`plan/slice-12` at `e4a59dde`), 2026-11-28.
Verdict: needs-revision, 11 findings; revision 1 answers them. The critic's answer, as returned:

needs-revision

1. **`story:swell-num-bigint-core`, `story:slice-12-conformance-close` (blocking).** The BigInt limit is a deviation of swell's own. The plan gives its omission line no owner, and no acceptance checks one.
   - Place: `swell-num-bigint-core.md:39` and `slice-12-conformance-close.md:61`.
   - Core says that if no allowance exists, the limit is "listed as an omission of the sections it touches in `conformance/test262/sections.txt` by `story:slice-12-conformance-close`".
   - The close says: "Not written: `conformance/test262/sections.txt` ... no line is expected to change".
   - `docs/design/host-interface.md:216-218` already says the BigInt `RangeError`s "are deviations of swell's own, to be listed as omissions of the sections they touch". `AGENTS.md` and the epic's Constraints require an omission to be recorded where the gate reads it.
   - On the base, `sections.txt` has `omit sec-string-objects string-length-limit decision-3` and `omit sec-applystringornumericbinaryoperator string-length-limit decision-3` (line 95). It has no BigInt line.
   - The file is an input of `expected.digest`, so the line has to be written in the close's commit.
   - To settle it:
     - Give the close `sections.txt` in its Writes.
     - Name the lines (for example `omit sec-bigint-objects bigint-bits-limit decision-6` and the same for `sec-applystringornumericbinaryoperator`).
     - Add a grep to the close's acceptance.
     - Or have core state that a found allowance makes the omission unnecessary.

2. **`story:bigint-arithmetic-and-bitwise-operators`, test `a_result_within_bigint_bits_but_over_the_memory_budget_ends_in_memory_budget` (blocking).** The test is the epic's acceptance item 6. As written it can end in `StepBudget`.
   - Place: `bigint-arithmetic-and-bitwise-operators.md:49` and Change item (3).
   - The test names `bigint_bits` and `memory_bytes`, but not `steps_per_entry`.
   - Item (3) charges "the steps before the work" and then the memory. The steps come from `pow_cost` for `2n ** (2n ** 24n)`, which is squarings of up to 131,072 limbs. That is far above any ordinary test step budget (`script.rs:850` uses 10,000,000).
   - Under a finite budget the entry ends in `Terminated(StepBudget)`, not `MemoryBudget`.
   - To settle it, name `steps_per_entry = u64::MAX` in the test, or charge the memory before the steps.

3. **`story:bigint-comparison-and-loose-equality`, test `a_string_compared_with_a_bigint_charges_per_code_unit` (blocking).** The stated difference is off by one.
   - Place: `bigint-comparison-and-loose-equality.md:37`. The text is "a String of 10,000 digits against `1n` ... against a String of 1 digit: the steps differ by at least 10,000".
   - The only charge the story states for StringToBigInt is a step per code unit, so the difference is 10,000 − 1 = 9,999.
   - Whether the result's limb comparison or the conversion adds a step is not stated. Its sibling test for limbs correctly says 9,999.
   - To settle it, state at least 9,999. Or state the further charge that makes 10,000 provable (for example the `from_radix_digits_cost` charge, or limbs read in the comparison).

4. **`story:swell-num-bigint-powers-shifts-and-bits` and `story:bigint-arithmetic-and-bitwise-operators` (blocking).** The cost rule for a long exponent contradicts a required test.
   - Place: `swell-num-bigint-powers-shifts-and-bits.md:41` and `bigint-arithmetic-and-bitwise-operators.md:49`.
   - The powers story says "an exponent longer than 64 bits has a cost of `u64::MAX`, so the engine refuses it by its size before any cost is charged". It does not exempt bases 0, 1 and −1.
   - The arith test `exponentiation_rules` requires `(-1n) ** (2n ** 64n)` to be `1n` "with few steps". The exponent is 65 bits, so the engine would refuse it.
   - The powers test `pow_agrees_with_repeated_multiplication` at the `swell-num` level uses `(−1n) ** (2n ** 70n)`, but the engine's precharge is what the arith test exercises.
   - To settle it, say that `pow_cost` and `pow_bits` give a small cost and size for 0, 1 and −1 whatever the exponent. The `u64::MAX` rule then holds for |base| ≥ 2 only.

5. **`story:swell-num-bigint-core`, Acceptance (note).** Decision 6 is not checked by any command, and the threshold test is not tied to the counter.
   - Place: `swell-num-bigint-core.md:62` and `:72`.
   - The only check on the design text is `awk '/^## Not checked/,/^## How the counts/' ... | grep -c 'BigInt'` printing 0. Deleting the line satisfies it; writing the finding does not have to.
   - Nothing greps decision 6 for the threshold constant, the derivation or the finding. The old "(whether ECMA-262 words this allowance is not checked)" at `engine.md:2560` is not checked either.
   - `the_karatsuba_threshold_is_where_its_count_falls_below_schoolbook` uses "the two count functions", which Change never names.
   - `costs_bound_the_counted_limb_operations` only ties the counter to `limb_ops` within a factor of 2 plus 64. A threshold derived from formulas can therefore differ from the one the code's counter shows.
   - To settle it, name the two count functions and require them to equal the counter. Add greps for the threshold line in decision 6 and for `2560`.

6. **`story:swell-num-bigint-core` and `story:swell-num-bigint-powers-shifts-and-bits` (note).** "Every reservation fallible" is shown only by a grep.
   - Place: both Acceptance sections.
   - The grep is `git grep -nE 'vec!|Vec::with_capacity|\.push\('`. No test shows that `Err(Alloc)` comes back or that the engine maps it to `Terminated(MemoryBudget)`.
   - To settle it, add one deterministic test. For example a `shl` by about 2^62 bits makes `try_reserve_exact` fail with capacity overflow, with no allocation.

7. **`story:bigint-arithmetic-and-bitwise-operators`, test `a_long_multiplication_is_charged_per_limb_operation` (note).** The check is circular.
   - Place: `bigint-arithmetic-and-bitwise-operators.md:49`.
   - The test asserts a difference "at least the Karatsuba count of 2,000 by 2,000 limbs less the count of 1 by 1, which the unit reads from `mul_cost` and names". It can pass with any `mul_cost`, including a wrong one.
   - To settle it, add an absolute floor such as at least 2,000 steps.

8. **Counts that are wrong (note).**
   - `bigint-constructor-and-prototype.md:43` says "seven `cstm-*-on-bigint-primitive` cases". The list has six. `grep String/prototype story-builtin.cases.txt` gives `match`, `matchAll`, `replace`, `replaceAll`, `search` and `split`. With `indexOf` 2, `isWellFormed` 1 and `toWellFormed` 1 the directory is 10, as stated; seven would make 11.
   - `bigint-json-number-and-joined-cases.md:30` says "The 37 cases that are not JSON or Number". The list has 6 `JSON/stringify` + 1 `rawJSON` + 1 `Number` = 8 of them, so 35. The other two are `BigInt/wrapper-object-ordinary-toprimitive.js` and `BigInt/prototype/toString/a-z.js`.

9. **`set` (note).** Only per-test bounds are given for 71 new tests; no step or report sums them.
   - The slice adds 23 tests in `crates/num` and 48 in `crates/swell` (11 + 6 + 14 + 11 + 6). Each is bounded at 1.5 CPU seconds in the default mode and 2 in collect mode.
   - `AGENTS.md:693` records the gate's test steps at 117.6 s of 120 on the forge.
   - Every unit is told never to run the gate, so none of them can show the 120 s or the `gate-cpu` line.
   - To settle it, have each report give the sum of its new tests' CPU seconds, and have the wave's one `slot task check` read `gate-time test-steps`.

10. **`story:slice-12-conformance-close`, Acceptance (note).** One check is not a command.
    - Place: `slice-12-conformance-close.md:65`. The text is "`git diff "$BASE" -- conformance/test262/expected.txt` shows no line whose passed field fell (the report quotes the diff)".
    - To settle it, give an awk over `git show "$BASE:conformance/test262/expected.txt"` and the new file. It should print nothing when no passed field fell.

11. **Part runs (note, not established).** Each part's release `count` as one pool job of at most 30 minutes is not measured. No run was timed, and the stories give a split into halves as the fallback.

**Out of my lane:**
- `story:bigint-values-and-literals` touches about 24 files in 3 crates (scope).
- Several order-3 stories share `crates/swell/tests/main.rs` (parallel-safety).
- D3's upper-bound precharge overcharges (design).
- The close's missing edge to the slice-11 close, and whether slice 11's own lists exclude the 4 cases of `close-slice-11.cases.txt`.

**Checked and right:**
- Counts: `lists.sh` over the base run prints `slice 381 lists 381 incoming 0` and exits 0. Every part has the stated cases and runs (392/768, 1,275/2,373, 1,279/2,381, 398/794, 341/681). `check.sh` prints the stated `passed … B F R` line and failing-line count for all five stories (213, 84, 92, 199, 86). I rebuilt the three guards from the base run and they match byte for byte. 682 + 78 = 760, and the 8 runs of the 4 slice-11 cases are present.
- Failure messages and numbers: the failure messages (127/50/22 and 84/2) and the directory sums for values, arith, compare, builtin and joint match.
- Test counts: every test count named in a Change matches its list (14, 9, 10 + 1, 6, 12 + 2, 11, 6).
- Repository facts: the baselines the acceptances name are in the tree. These are the 16 `before slice 12` lines and the three each in arith and joint, `HOST-23`, `HOST-24` and `HOST-219` at lines 23, 24 and 219, the 349 lines of `realm_places.txt`, the lexer refusal at line 570 and the `Not checked` BigInt count of 1.
- Test-body arithmetic: HOST-23, HOST-24, the 64-bit exact-length test, the 6,250-limb and 10,000-limb figures, the `to_f64` ties and the `asIntN` examples are right.

**Decomposer's open questions:**
- D1: A (compile time). It is observable by the memory test, and under B a run-time conversion would be charged on each execution.
- D2: A. A count-derived threshold can be tested; one set by CPU measurement cannot be re-read the same way twice. Finding 5 still applies.
- D3: A. Only a precharge can be shown "before computing" (HOST-23, the step-budget test). Fix finding 2 first.
- D4: A.
- D5: A. With B, `1n + 1n` at `bigint_bits` 0 would fail at the literal, and HOST-24 would no longer test the addition's lower bound.
