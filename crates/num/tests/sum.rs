//! The exact accumulator of `Math.sumPrecise` (`swell_num::sum`; ECMA-262 21.3.2.34,
//! `#sec-math.sumprecise`; design, decision 6, "`Math.sumPrecise`").
//!
//! - Drawn lists against an oracle written here: the exact running partials of Shewchuk's
//!   algorithm, each step a two-sum, and the correctly rounded total of the partials, as
//!   `math.fsum` states them. The 2,000 lists come from Knuth's 64-bit linear congruential
//!   generator of the design's reference data (multiplier 6364136223846793005, increment
//!   1442695040888963407), seed 1: lengths 0 to 40, items of every exponent field from 0 (the
//!   subnormals) to 2,040, zeros of both signs, small integers, items near 1 and items that
//!   cancel an earlier one. The oracle's own sums pass no Number on the way, so no item is above
//!   2^1,018 and 40 of them stay below the largest Number; a sum that passes 2^1,024 on the way
//!   is held by the cases written by hand. Each list, its reverse and a fixed shuffle of it give
//!   the same bits.
//! - Cases written by hand: the signed zeros, ties to even, the sticky bit, overflow to ∞ at the
//!   tie below 2^1,024 and no overflow just under it, a sum that passes 2^1,025 on the way, the
//!   subnormals, NaN and the infinities, 100,000 items of 2^-1074, and 10,000 items of the
//!   largest Number and as many of its negation (a carry through every limb it reaches).
#![cfg(test)]

use swell_num::sum::{MAX_ITEMS, Sum};

/// Knuth's 64-bit linear congruential generator.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0
    }

    /// A draw below `n`, from the high bits.
    fn below(&mut self, n: u64) -> u64 {
        (self.next() >> 16) % n
    }

    /// A finite Number of exponent field `field` (0 for a subnormal), random stored bits and a
    /// random sign.
    fn number(&mut self, field: u64) -> f64 {
        let sign = self.next() & (1 << 63);
        let stored = self.next() >> 12;
        f64::from_bits(sign | (field << 52) | stored)
    }
}

/// The largest exponent field of a drawn item: 2,040, so an item is below 2^1,018 and 40 of them
/// below 2^1,024, where the oracle's two-sums stay exact.
const MAX_DRAWN_FIELD: u64 = 2_040;

/// A list of 0 to 40 items drawn by `lcg`.
fn draw_list(lcg: &mut Lcg) -> Vec<f64> {
    let length = lcg.below(41);
    let mut items: Vec<f64> = Vec::new();
    for _ in 0..length {
        let item = match lcg.below(8) {
            // Any exponent field, the subnormals' included.
            0..=2 => {
                let field = lcg.below(MAX_DRAWN_FIELD + 1);
                lcg.number(field)
            }
            // A subnormal.
            3 => lcg.number(0),
            // A zero of either sign.
            4 => {
                if lcg.next() & (1 << 63) == 0 {
                    0.0
                } else {
                    -0.0
                }
            }
            // A small integer, many of whose sums tie.
            5 => (lcg.below(2_001) as f64) - 1_000.0,
            // Near 1: exponent fields 1,023 ± 60.
            6 => {
                let field = 963 + lcg.below(121);
                lcg.number(field)
            }
            // The negation of an earlier item, or a fresh one when there is none.
            _ => match items.len() {
                0 => lcg.number(1_023),
                n => -items[lcg.below(n as u64) as usize],
            },
        };
        items.push(item);
    }
    items
}

/// The oracle's sum of `items`: Shewchuk's partials (each step the two-sum of the running value
/// and a partial, the larger magnitude first; a zero low part is dropped), then the correctly
/// rounded sum of the partials from the largest down, with the half-way correction of the last
/// step, as `math.fsum` has it. Every intermediate must stay finite.
fn oracle(items: &[f64]) -> f64 {
    let mut partials: Vec<f64> = Vec::new();
    for &item in items {
        let mut x = item;
        let mut kept = 0;
        for j in 0..partials.len() {
            let mut y = partials[j];
            if x.abs() < y.abs() {
                std::mem::swap(&mut x, &mut y);
            }
            let hi = x + y;
            let lo = y - (hi - x);
            if lo != 0.0 {
                partials[kept] = lo;
                kept += 1;
            }
            x = hi;
        }
        partials.truncate(kept);
        partials.push(x);
    }
    let mut n = partials.len();
    if n == 0 {
        return 0.0;
    }
    n -= 1;
    let mut hi = partials[n];
    let mut lo = 0.0;
    while n > 0 {
        let x = hi;
        n -= 1;
        let y = partials[n];
        hi = x + y;
        let y_rounded = hi - x;
        lo = y - y_rounded;
        if lo != 0.0 {
            break;
        }
    }
    if n > 0 && ((lo < 0.0 && partials[n - 1] < 0.0) || (lo > 0.0 && partials[n - 1] > 0.0)) {
        let y = lo * 2.0;
        let x = hi + y;
        let y_rounded = x - hi;
        if y == y_rounded {
            hi = x;
        }
    }
    assert!(hi.is_finite(), "the oracle passed a Number: {items:?}");
    hi
}

/// What `Math.sumPrecise` answers for the finite `items` by the oracle: its sum, but -0 for a
/// list whose items are all -0 (the state minus-zero) and +0 for any other zero sum.
fn expected(items: &[f64]) -> f64 {
    let sum = oracle(items);
    if sum != 0.0 {
        sum
    } else if items.iter().all(|x| x.to_bits() == (-0.0f64).to_bits()) {
        -0.0
    } else {
        0.0
    }
}

/// The sum of `items` by `Sum`.
fn sum(items: &[f64]) -> f64 {
    let mut sum = Sum::new();
    for &item in items {
        sum.add(item);
    }
    sum.finish()
}

/// `sum(items)` has the bits of `want`.
fn assert_sum(items: &[f64], want: f64, what: &str) {
    let got = sum(items);
    assert_eq!(
        got.to_bits(),
        want.to_bits(),
        "{what}: got {got:e} ({:#018x}), want {want:e} ({:#018x})",
        got.to_bits(),
        want.to_bits()
    );
}

/// 2^`n` for `n` from -1,074 to 1,023, built from its bits.
fn pow2(n: i32) -> f64 {
    if n < -1022 {
        f64::from_bits(1 << (n + 1074))
    } else {
        f64::from_bits(((n + 1023) as u64) << 52)
    }
}

/// 2,000 drawn lists agree with the oracle bit for bit, and so do their reverse and a fixed
/// shuffle of each (a Fisher-Yates shuffle by the same generator).
#[test]
fn drawn_lists_agree_with_the_oracle_in_any_order() {
    let mut lcg = Lcg(1);
    let mut items_seen = 0;
    for list in 0..2_000 {
        let items = draw_list(&mut lcg);
        items_seen += items.len();
        let want = expected(&items);
        assert_sum(&items, want, &format!("list {list}: {items:?}"));
        let mut reversed = items.clone();
        reversed.reverse();
        assert_sum(&reversed, want, &format!("list {list} reversed"));
        let mut shuffled = items.clone();
        for i in (1..shuffled.len()).rev() {
            let j = lcg.below(i as u64 + 1) as usize;
            shuffled.swap(i, j);
        }
        assert_sum(&shuffled, want, &format!("list {list} shuffled"));
    }
    println!("2000 lists, {items_seen} items");
}

/// `#sec-math.sumprecise`, steps 3 and 7.b.iv.4, and step 12 (𝔽(0) is +0): no item and only -0
/// items give -0; a +0, or items whose sum is zero, give +0.
#[test]
fn the_signed_zeros() {
    assert_sum(&[], -0.0, "[]");
    assert_sum(&[-0.0], -0.0, "[-0]");
    assert_sum(&[-0.0, -0.0], -0.0, "[-0, -0]");
    assert_sum(&[-0.0, 0.0], 0.0, "[-0, 0]");
    assert_sum(&[0.0, -0.0], 0.0, "[0, -0]");
    assert_sum(&[0.0], 0.0, "[0]");
    assert_sum(&[1.0, -1.0], 0.0, "[1, -1]");
    assert_sum(&[-1.0, 1.0], 0.0, "[-1, 1]");
    assert_sum(&[pow2(-1074), -pow2(-1074)], 0.0, "[2^-1074, -2^-1074]");
}

/// Rounding to nearest, ties to even, with the sticky bit of the bits below the next one.
#[test]
fn ties_to_even_and_the_sticky_bit() {
    let two53 = pow2(53);
    assert_sum(&[two53, 1.0], two53, "[2^53, 1]");
    assert_sum(
        &[two53, 1.0, pow2(-1074)],
        two53 + 2.0,
        "[2^53, 1, 2^-1074]",
    );
    assert_sum(&[two53 + 2.0, 1.0], two53 + 4.0, "[2^53 + 2, 1]");
    assert_sum(&[-two53, -1.0], -two53, "[-2^53, -1]");
    assert_sum(&[-(two53 + 2.0), -1.0], -(two53 + 4.0), "[-(2^53 + 2), -1]");
    assert_sum(&[two53, 1.0, -pow2(-1074)], two53, "[2^53, 1, -2^-1074]");
}

/// The largest Number plus 2^970 is the tie between it and 2^1,024 and rounds to even, up, to
/// ∞; three items that fall short of that tie by 2^967 give the largest Number; a sum that
/// passes 2^1,024 on the way and comes back is exact.
#[test]
fn overflow_at_the_tie_below_2_to_the_1024() {
    let max = f64::MAX;
    assert_sum(&[max, pow2(970)], f64::INFINITY, "[MAX, 2^970]");
    assert_sum(&[-max, -pow2(970)], f64::NEG_INFINITY, "[-MAX, -2^970]");
    assert_sum(
        &[max, pow2(969), pow2(968), pow2(967)],
        max,
        "[MAX, 2^969, 2^968, 2^967]",
    );
    assert_sum(&[max, max, -max], max, "[MAX, MAX, -MAX]");
    assert_sum(&[pow2(1023), pow2(1023)], f64::INFINITY, "[2^1023, 2^1023]");
    assert_sum(
        &[1e308, 1e308, 0.1, 0.1, 0.1, -1e308, -1e308],
        0.30000000000000004,
        "the design's example",
    );
}

/// The subnormals are counted exactly: three items of 2^-1074 are 3 × 2^-1074, and the largest
/// subnormal plus 2^-1074 is 2^-1022.
#[test]
fn subnormals() {
    let tiny = pow2(-1074);
    assert_sum(&[tiny, tiny, tiny], f64::from_bits(3), "three 2^-1074");
    assert_sum(
        &[f64::from_bits((1 << 52) - 1), tiny],
        pow2(-1022),
        "the largest subnormal and 2^-1074",
    );
    assert_sum(
        &[pow2(-1022), -tiny],
        f64::from_bits((1 << 52) - 1),
        "2^-1022 less 2^-1074",
    );
}

/// `#sec-math.sumprecise`, step 7.b.iv: a NaN anywhere gives NaN, after infinities too; both
/// infinities give NaN; one infinity gives itself whatever finite items come with it.
#[test]
fn nan_and_the_infinities() {
    let nan = f64::NAN;
    let inf = f64::INFINITY;
    for items in [
        vec![nan],
        vec![1.0, nan],
        vec![inf, nan],
        vec![nan, inf],
        vec![-inf, nan, 1.0],
        vec![inf, -inf],
        vec![-inf, inf],
        vec![inf, 1.0, -inf, 2.0],
    ] {
        assert!(sum(&items).is_nan(), "{items:?}");
    }
    assert_sum(&[inf], inf, "[inf]");
    assert_sum(&[inf, inf, -f64::MAX], inf, "[inf, inf, -MAX]");
    assert_sum(&[-0.0, -inf, -0.0], -inf, "[-0, -inf, -0]");
    assert_sum(&[f64::MAX, f64::MAX, -inf], -inf, "[MAX, MAX, -inf]");
}

/// 100,000 items of 2^-1074 give 100,000 × 2^-1074.
#[test]
fn a_hundred_thousand_of_the_smallest_subnormal() {
    let items = vec![pow2(-1074); 100_000];
    assert_sum(&items, f64::from_bits(100_000), "100,000 × 2^-1074");
}

/// 10,000 items of the largest Number, about 2^1,037.3 in all, then 10,000 of its negation give
/// +0: the carries reach limb 17, and the two accumulators are equal.
#[test]
fn ten_thousand_of_the_largest_number_and_of_its_negation() {
    let mut items = vec![f64::MAX; 10_000];
    items.extend(std::iter::repeat_n(-f64::MAX, 10_000));
    assert_sum(&items, 0.0, "10,000 × MAX, 10,000 × -MAX");
    let mut over = vec![f64::MAX; 10_000];
    over.extend(std::iter::repeat_n(-f64::MAX, 9_999));
    assert_sum(&over, f64::MAX, "10,000 × MAX, 9,999 × -MAX");
}

/// The draft's count limit, 2^53 - 1, and the size of `Sum`: the 544 bytes of its two
/// accumulators, the state and its padding.
#[test]
fn the_count_limit_and_the_size() {
    assert_eq!(MAX_ITEMS, 9_007_199_254_740_991);
    let size = size_of::<Sum>();
    println!("size_of::<Sum>() = {size}");
    assert_eq!(size, 2 * 34 * 8 + 8);
}
