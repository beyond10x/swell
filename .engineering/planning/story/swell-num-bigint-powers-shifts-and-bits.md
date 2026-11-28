---
format: aep.planning-md/3
id: story:swell-num-bigint-powers-shifts-and-bits
kind: story
status: draft
title: 'swell-num BigInt: exponentiation, shifts, two''s complement bitwise operations, asIntN and asUintN'
summary: pow.rs, shift.rs, bitwise.rs under crates/num/src/bigint/ with their cost and bit bounds; order 2; no case.
relations:
- decomposes: epic:slice-12-bigint
- depends_on: story:swell-num-bigint-core
scope:
- confidence: inferred
  path: crates/num/src/bigint.rs
- confidence: inferred
  path: crates/num/src/bigint/bitwise.rs
- confidence: inferred
  path: crates/num/src/bigint/pow.rs
- confidence: inferred
  path: crates/num/src/bigint/shift.rs
- confidence: inferred
  path: crates/num/tests/bigint_ops.rs
revision: 5
---
## Context

Slice 12 brings `BigInt` (`epic:slice-12-bigint`; `docs/design/engine.md`, decision 6, "`BigInt`"). This story completes the arithmetic of `swell-num` that the operators and `BigInt.asIntN` and `asUintN` need, on top of `story:swell-num-bigint-core` (the type `BigInt`, add, sub, mul with Karatsuba, Knuth's division, text both ways, the Numbers, the cost and size functions, the counter of limb operations, `limbs_with_capacity` and the generator `crates/num/tests/support/pairs.rs`). It has no case of test262. It is in order 2, built at the same time as `story:bigint-values-and-literals`, with which it shares no file (that story writes no file of `crates/num/`). `story:bigint-arithmetic-and-bitwise-operators` and `story:bigint-constructor-and-prototype` depend on it.

Line numbers in this body are those of `f962c324`; every anchor is found by its name.

The clauses: `BigInt::exponentiate` (6.1.6.2.3), `BigInt::leftShift` (6.1.6.2.9), `BigInt::signedRightShift` (6.1.6.2.10; `BigInt::unsignedRightShift` is a `TypeError`, the engine's), `BigInt::bitwiseAND`, `bitwiseXOR`, `bitwiseOR` (6.1.6.2.18 to .20, through BigIntBitwiseOp, 6.1.6.2.17: the infinite two's complement of each operand), `BigInt::bitwiseNOT` (6.1.6.2.2: −x − 1), and the arithmetic of `BigInt.asIntN` and `BigInt.asUintN` (21.2.2.1 and .2: the value modulo 2^bits, then for `asIntN` minus 2^bits when it is at least 2^(bits − 1)).

## Domain relations

None beyond `story:swell-num-bigint-core`'s: the operations take and give `BigInt` values and hold nothing.

## Change

Failing tests first: the tests below are written before the code and fail to compile on the base of this story (the functions do not exist).

- `crates/num/src/bigint/pow.rs`, `crates/num/src/bigint/shift.rs` and `crates/num/src/bigint/bitwise.rs` (new), each with its `mod` line in `crates/num/src/bigint.rs` appended after the `mod` lines of `story:swell-num-bigint-core`:
  - `pow(&BigInt, &BigInt) -> Result<BigInt, Power>` with `Power::NegativeExponent` (the engine's `RangeError`), `0n ** 0n` = `1n`, a base of 0, 1 or −1 answered without a loop whatever the exponent, and otherwise left-to-right binary exponentiation over the exponent's bits with the multiplication of `story:swell-num-bigint-core`. `pow_bits(base, exponent) -> (u64, u64)` and `pow_cost(base, exponent) -> Cost` read the base's bit length and sign and the exponent's length:
    - for a base of 0, 1 or −1 they are small constants whatever the exponent's length: `pow_bits` is `(0, 1)` and `pow_cost` at most one limb operation, one result limb and no scratch (the parity of the exponent is read for −1, one limb);
    - for an absolute base of 2 or more, `pow_bits` is `((base_bits − 1) × e + 1, base_bits × e)` saturating, with `e` the exponent as a `u64` or `u64::MAX` when it is longer than 64 bits, and `pow_cost` is the sum of the `mul_cost` of the squarings and multiplications binary exponentiation performs for the operand lengths these bounds give, saturating; an exponent longer than 64 bits gives `u64::MAX` for both, so the engine refuses the power by its size before any cost is charged;
  - `shl(&BigInt, shift: &BigInt)` and `shr` (a negative shift shifts the other way; `shr` of a negative value rounds toward −∞, as 6.1.6.2.10's floor says: `−5n >> 1n` is `−3n`; a right shift by more than the bit length answers `0n` or `−1n` with no loop over the shift; a left shift of a non-zero value by a shift longer than 64 bits answers `Err(Alloc)` before it reserves anything, since its result cannot be addressed), `shift_bits` and `shift_cost` (a left shift of a non-zero value by `s` has `bit_len + s` bits, the shift read as a `u64` or as unbounded);
  - `and`, `or`, `xor`, `not` on the two's complement form, written limb by limb with the borrow of each negative operand's negation carried through the loop (no full complemented copy of an operand), `bitwise_bits` and `bitwise_cost` (the result is at most one limb longer than the longer operand);
  - `as_int_n(bits: u64, &BigInt)` and `as_uint_n(bits: u64, &BigInt)`, with `as_n_bits` and `as_n_cost` (`as_uint_n` of a negative value has `bits` bits: `asUintN(200, −1n)` is 2^200 − 1, so its result length comes from `bits`, not from the operand);
  - every buffer made by `limbs_with_capacity` of `story:swell-num-bigint-core`, `Err(Alloc)` when it refuses, no `vec!`, `Vec::with_capacity` or growing `push`, no panic, as that story says; the counter of limb operations counts every loop here too.
- Tests, in `crates/num/tests/bigint_ops.rs` (new; a module of the crate's one test binary; it loads `support/pairs.rs` as `#[path = "support/pairs.rs"] mod pairs;` and does not change it), 11 tests, none on the base: `pow_agrees_with_repeated_multiplication` (bases −20 to 20 and three long ones, exponents 0 to 70; `0n ** 0n`, `(−1n) ** (2n ** 70n)` answered with no loop: the counter reads at most 4); `pow_bits_refuse_before_computing` (`2n ** (2n ** 40n)`: the lower bound is above 2^20, read before any call of `pow`, and a negative exponent is `Err`); `pow_bits_and_pow_cost_of_bases_0_1_and_minus_1_ignore_the_exponent_length` (an exponent of 65 bits, `2n ** 64n`, and one of 1,000 bits: for the bases 0, 1 and −1 `pow_bits` is `(0, 1)` and `pow_cost.limb_ops` at most 1; for the base 2 and the exponent of 65 bits both are `u64::MAX`); `shifts_agree_with_i128_for_both_signs` (values and shifts from −200 to 200, every result that fits compared with `i128`, the floor of negative right shifts); `a_left_shift_longer_than_64_bits_is_err_alloc` (`shl(1n, 2n ** 64n)` answers `Err(Alloc)` and the counter reads 0: nothing was reserved or computed); `bitwise_operations_agree_with_i128_twos_complement` (2,000 pairs of both signs from `support/pairs.rs`); `bitwise_operations_of_long_operands_of_mixed_sign` (operands of 1 to 40 limbs: each result compared bit by bit with a reference the test computes from the two's complement of each operand over the longer length plus one limb); `as_int_n_and_as_uint_n_agree_with_i128_for_bits_0_to_130`; `as_uint_n_of_a_negative_value_has_bits_bits` (`asUintN(200, −1n)` = 2^200 − 1, `asIntN(1, −1n)` = −1n, `asIntN(0, x)` = 0n); `costs_bound_the_counted_limb_operations` (as the test of the same name of `story:swell-num-bigint-core`, for the operations of this story: counted ≤ `limb_ops` ≤ 2 × counted + 64; and `pow_cost` equals the sum of the `mul_cost` its exponentiation performs, for the bases and exponents of the first test); `bit_bounds_enclose_the_result`.
- `crates/num/tests/main.rs` is generated and not in this story's scope: a unit that meets a conflict in it takes either side and runs `sh taskfiles/test-main.sh write`; the coordinator generates it again on the join.
- Time ("Tests stay fast", the `AGENTS.md` of the folder that holds the repository, and `AGENTS.md`, "Commands and the gate"): a test is terminated at 60 s and counts as failed, and the gate fails when the wall time of its test steps passes 120 s. Each test of this story stays under 1.5 CPU seconds alone, measured as `AGENTS.md` says (the test binary run with `--exact <name> --test-threads 1` in a pool job, user plus system). A product several tests share is made once inside one test or is a committed file, never in a `static`; no network request, no model call. Builds and tests go through the pool: `cargo` is the shim of the pool, anything else that compiles or tests is prefixed `slot`; a pool job ends after 10 minutes unless it names its time (`slot --timeout 30m` or `SLOT_TIMEOUT=30m`, at most 30 minutes); a script that only measures, probes or waits runs without the prefix. One integration-test binary per crate. No `unsafe`, no third-party crate, no panic, `unwrap`, `expect` or unchecked indexing; no Python in any tool, test or command; intermediate files in the scratch directory; `swell-num` stays `#![no_std]`.
- Size: about 950 lines (powers 170, shifts 150, the bitwise operations 250, `asIntN` and `asUintN` 100, tests 280): one session.

## Requirements

No host requirement.

Writes: `crates/num/src/bigint/pow.rs`, `crates/num/src/bigint/shift.rs`, `crates/num/src/bigint/bitwise.rs` (new), `crates/num/src/bigint.rs` (three `mod` lines appended after those of `story:swell-num-bigint-core`, and the re-exports of the new functions if the type's methods are where the module puts them), `crates/num/tests/bigint_ops.rs` (new). Order 2, built at the same time as `story:bigint-values-and-literals`, which writes no file of `crates/num/` (it calls `swell_num::bigint` of `story:swell-num-bigint-core` as that story leaves it); the two stories share no file, generated files included. Depends on `story:swell-num-bigint-core` (its type, its multiplication and `mul_cost`, its counter, `limbs_with_capacity`, its cost form and `support/pairs.rs`). Not written: every file outside `crates/num/`, and in `crates/num/` every file `story:swell-num-bigint-core` wrote but the `mod` lines named (`crates/num/tests/support/pairs.rs` included).

## Acceptance

The 11 tests named in Change pass, run by the binaries of this change and not as a whole step, and the first command counts 11 tests under `bigint_ops::` (none on the base: the module is new) and the 16 of `bigint::`: `cargo nextest run --locked -p swell-num --test main -E 'test(/^bigint_ops::/) | test(/^bigint::/)'` and `cargo nextest run --locked -p swell-num --lib`, each exits 0. `slot task clippy`, `slot task forbid-unsafe-check`, `slot task libm-check` and `slot task nostd-build` exit 0; `sh taskfiles/test-main.sh check` exits 0; the names printed by `git log --no-merges --first-parent --format= --name-only "$BASE"..HEAD -- Cargo.lock '*/Cargo.toml' crates/num/src/nat.rs crates/num/tests/support` are none (`BASE` the commit the unit starts from; the unit's own commits only, so a merge of the wave branch does not change the reading); `git grep -nE 'vec!|Vec::with_capacity|\.push\(|try_reserve' -- crates/num/src/bigint/pow.rs crates/num/src/bigint/shift.rs crates/num/src/bigint/bitwise.rs` prints nothing. The report gives each test's measured CPU seconds and the sum of the CPU seconds of the 11 new tests; a test over 1.5 fails this acceptance.

## Out of scope

The operators of the language and their charges (`story:bigint-arithmetic-and-bitwise-operators`); `BigInt.asIntN` and `asUintN` as builtins, with `ToIndex` and the limit (`story:bigint-constructor-and-prototype`); anything outside `crates/num/`.
