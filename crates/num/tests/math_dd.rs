//! The double-double arithmetic of `swell_num::math::dd` and `swell_num::math::hypot` (design,
//! decision 6, "The functions of `Math` that IEEE 754 does not fix": the paragraph "Double-double"
//! and item 8; `Math.hypot`, `#sec-math.hypot`).
//!
//! Every check is against exact arithmetic ([`exact`]) or against the committed reference lines
//! of `data/math/reference-hypot.txt` (made once by GNU bc 1.08.2, run as `bc -l`, by the
//! procedure of the design's "The tool that makes the lines"; its header says how):
//!
//! - `two_sum` and `fast_two_sum` are exact for every pair of Numbers whose sum does not
//!   overflow, `two_product` for every product that is zero or of magnitude 2^-960 or more and
//!   does not overflow; outside that range only the high part is held (the correctly rounded
//!   `a + b` or `a * b`).
//! - `add`, `sub`, `mul`, `div` and `sqrt` are held to a bound of their relative error that this
//!   file measured, each a multiple of 2^-106, and each bound to the design's ceiling of 2^-100.
//!   A pair is in the range of an operation when its exact result is zero or of magnitude 2^-960
//!   to the largest Number (for `div` also the dividend; for `sqrt` the operand); a boundary pair
//!   outside it is only run, so that it shows it does not panic.
//! - `hypot` is held to "one of the two Numbers that enclose the exact result" (less than 1 ulp,
//!   the design's group A) by the reference lines and again by exact arithmetic.
//!
//! The drawn inputs come from Knuth's 64-bit linear congruential generator with seed 1, the
//! generator of the design's reference data (multiplier 6364136223846793005, increment
//! 1442695040888963407): 2,000 inputs per operation, beside the boundary inputs (±0, the smallest
//! and the largest subnormal, 2^-1022, 2^-960, 1, 1 + 2^-52, 1.5, 2^996, 2^1023 and the largest
//! Number, each with both signs).

#![cfg(test)]

#[path = "support/exact.rs"]
mod exact;

use std::cmp::Ordering;

use exact::{Rat, is_faithful_root, relative_error, sqrt_relative_error};
use swell_num::math::dd::{Dd, fast_two_sum, pair, two_product, two_sum};
use swell_num::math::hypot;

/// The design's ceiling for the relative error of every double-double operation, 2^-100.
const CEILING_LOG2: i64 = -100;

/// The bounds this file holds, in units of 2^-106: the largest relative error measured over the
/// inputs of each test, rounded up to the next whole unit. Measured on 2026-11-24 (in units of
/// 2^-106): add 1.2566, sub 1.2469, mul 1.6980, div 2.3975, sqrt 1.9243.
const ADD_BOUND: i64 = 2;
const MUL_BOUND: i64 = 2;
const DIV_BOUND: i64 = 3;
const SQRT_BOUND: i64 = 2;

/// The reference lines of `hypot` and their count: 18 boundary pairs and 200 drawn pairs.
const REFERENCE: &str = include_str!("../../../data/math/reference-hypot.txt");
const REFERENCE_LINES: usize = 218;

/// Knuth's 64-bit linear congruential generator, seed 1 (design, "The tool that makes the
/// lines", step 5).
struct Lcg(u64);

impl Lcg {
    fn new() -> Lcg {
        Lcg(1)
    }

    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0
    }

    /// A normal Number: the sign is bit 63 of a draw, the binary exponent is
    /// `low + ((draw mod 2^63) / 2^32 mod (high - low + 1))`, the 52 stored bits are the high 52
    /// bits of the next draw (the reference generator's rule).
    fn number(&mut self, low: i32, high: i32) -> f64 {
        let low = low.max(-1022);
        let high = high.min(1023);
        let draw = self.next();
        let span = (high - low + 1) as u64;
        let exponent = low + (((draw & (u64::MAX >> 1)) >> 32) % span) as i32;
        let stored = self.next() >> 12;
        f64::from_bits((draw & (1 << 63)) | (((exponent + 1023) as u64) << 52) | stored)
    }

    /// A Number whose exponent is within 60 of that of x.
    fn near(&mut self, x: f64) -> f64 {
        let e = exponent(x);
        self.number(e - 60, e + 60)
    }

    /// A low part for hi: an exponent 54 to 61 below that of hi, so that |lo| < ulp(hi) / 2.
    fn low_part(&mut self, hi: f64) -> f64 {
        let below = 54 + (self.next() >> 61) as i32;
        let e = exponent(hi) - below;
        self.number(e, e)
    }

    /// A normalized pair whose high part has an exponent in [low, high].
    fn pair(&mut self, low: i32, high: i32) -> Dd {
        let hi = self.number(low, high);
        Dd {
            hi,
            lo: self.low_part(hi),
        }
    }
}

/// The binary exponent of a normal Number.
fn exponent(x: f64) -> i32 {
    ((x.to_bits() >> 52) & 0x7FF) as i32 - 1023
}

fn pow2(k: i32) -> f64 {
    Rat::pow2(i64::from(k)).to_f64()
}

/// The boundary Numbers, each with both signs.
fn boundary_values() -> Vec<f64> {
    let positive = [
        f64::from_bits(1),
        f64::from_bits(0x000F_FFFF_FFFF_FFFF),
        f64::MIN_POSITIVE,
        pow2(-960),
        1.0,
        1.0 + f64::EPSILON,
        1.5,
        pow2(996),
        pow2(1023),
        f64::MAX,
    ];
    let mut values = vec![0.0, -0.0];
    for x in positive {
        values.push(x);
        values.push(-x);
    }
    values
}

fn boundary_pairs() -> Vec<(f64, f64)> {
    let values = boundary_values();
    let mut pairs = Vec::new();
    for &a in &values {
        for &b in &values {
            pairs.push((a, b));
        }
    }
    pairs
}

fn exact_of(x: Dd) -> Rat {
    Rat::from_dd(x.hi, x.lo)
}

/// Zero, or a magnitude from 2^-960 to the largest Number.
fn in_range(value: &Rat) -> bool {
    if value.is_zero() {
        return true;
    }
    let magnitude = value.abs();
    magnitude.compare(&Rat::pow2(-960)) != Ordering::Less
        && magnitude.compare(&Rat::from_f64(f64::MAX)) != Ordering::Greater
}

/// The largest relative error seen, and how many inputs were measured.
struct Worst {
    error: Rat,
    measured: usize,
    outside: usize,
}

impl Worst {
    fn new() -> Worst {
        Worst {
            error: Rat::zero(),
            measured: 0,
            outside: 0,
        }
    }

    fn see(&mut self, error: Rat) {
        self.measured += 1;
        if error.compare(&self.error) == Ordering::Greater {
            self.error = error;
        }
    }

    /// Prints the measured error and asserts it against the bound of this file and the bound
    /// against the design's ceiling.
    fn hold(&self, name: &str, bound_units: i64) {
        let bound = Rat::from_i64(bound_units).mul(&Rat::pow2(-106));
        let units = self.error.mul(&Rat::pow2(106)).to_f64();
        println!(
            "math_dd {name}: largest relative error {units:.4} x 2^-106 (2^{:.3}) over {} inputs, \
             {} boundary pair(s) outside the range; bound {bound_units} x 2^-106, ceiling 2^{CEILING_LOG2}",
            self.error.log2(),
            self.measured,
            self.outside,
        );
        assert!(
            bound.compare(&Rat::pow2(CEILING_LOG2)) != Ordering::Greater,
            "{name}: the bound {bound_units} x 2^-106 is above the design's ceiling 2^{CEILING_LOG2}"
        );
        assert!(
            self.error.compare(&bound) != Ordering::Greater,
            "{name}: measured {units} x 2^-106, above the bound {bound_units} x 2^-106"
        );
    }
}

/// The relative error of result against exact; a zero exact value needs a zero result.
fn error_of(result: Dd, exact: &Rat) -> Rat {
    let value = exact_of(result);
    if exact.is_zero() {
        assert!(value.is_zero(), "exact zero, result {result:?}");
        return Rat::zero();
    }
    relative_error(&value, exact)
}

#[test]
fn two_sum_and_fast_two_sum_are_exact() {
    let mut lcg = Lcg::new();
    let mut pairs = boundary_pairs();
    for i in 0..2_000 {
        let a = lcg.number(-1022, 1023);
        let b = if i % 2 == 0 {
            lcg.near(a)
        } else {
            lcg.number(-1022, 1023)
        };
        pairs.push((a, b));
    }
    let mut exact = 0;
    let mut overflow = 0;
    for &(a, b) in &pairs {
        let sum = a + b;
        let (big, small) = if a.abs() >= b.abs() { (a, b) } else { (b, a) };
        for (name, result) in [
            ("two_sum", two_sum(a, b)),
            ("fast_two_sum", fast_two_sum(big, small)),
        ] {
            assert_eq!(
                result.hi.to_bits(),
                sum.to_bits(),
                "{name}({a:e}, {b:e}): hi"
            );
            if sum.is_finite() {
                let want = Rat::from_f64(a).add(&Rat::from_f64(b));
                assert_eq!(
                    exact_of(result).compare(&want),
                    Ordering::Equal,
                    "{name}({a:e}, {b:e}) = {result:?} is not exact"
                );
            }
        }
        if sum.is_finite() {
            exact += 1;
        } else {
            overflow += 1;
        }
    }
    println!("math_dd two_sum: {exact} pair(s) exact, {overflow} overflowing (hi held only)");
    assert_eq!(exact + overflow, 2_000 + 22 * 22);
    assert!(overflow > 0 && exact > 2_000);
}

#[test]
fn two_product_is_exact_where_its_low_part_is_a_number() {
    let mut lcg = Lcg::new();
    let mut pairs = boundary_pairs();
    for i in 0..2_000 {
        if i % 2 == 0 {
            pairs.push((lcg.number(-480, 480), lcg.number(-480, 480)));
        } else {
            pairs.push((lcg.number(-1022, 1023), lcg.number(-1022, 1023)));
        }
    }
    let mut exact = 0;
    let mut outside = 0;
    for &(a, b) in &pairs {
        let product = two_product(a, b);
        assert_eq!(
            product.hi.to_bits(),
            (a * b).to_bits(),
            "two_product({a:e}, {b:e}): hi"
        );
        let want = Rat::from_f64(a).mul(&Rat::from_f64(b));
        if (a * b).is_finite() && in_range(&want) {
            assert_eq!(
                exact_of(product).compare(&want),
                Ordering::Equal,
                "two_product({a:e}, {b:e}) = {product:?} is not exact"
            );
            exact += 1;
        } else {
            outside += 1;
        }
    }
    println!(
        "math_dd two_product: {exact} pair(s) exact, {outside} outside the range (hi held only)"
    );
    assert!(exact > 1_000 && outside > 0);
}

#[test]
fn two_product_scales_a_factor_of_2_996_or_more() {
    // A factor at the scaling guard times factors from the smallest subnormal up, each product
    // finite: exact, and a finite low part.
    let large = [pow2(996), 1.5 * pow2(996), pow2(1023), f64::MAX];
    let others = [
        f64::from_bits(1),
        f64::from_bits(0x000F_FFFF_FFFF_FFFF),
        f64::MIN_POSITIVE,
        pow2(-960),
        1.25 * pow2(-1000),
        0.5,
        1.0 - f64::EPSILON / 2.0,
        1.0,
        134_217_729.0 * pow2(-27) - 1.0,
    ];
    let mut checked = 0;
    for &a in &large {
        for &b in &others {
            for (x, y) in [(a, b), (b, a), (-a, b), (a, -b)] {
                let product = two_product(x, y);
                if !(x * y).is_finite() {
                    continue;
                }
                assert!(
                    product.lo.is_finite(),
                    "two_product({x:e}, {y:e}) = {product:?}"
                );
                let want = Rat::from_f64(x).mul(&Rat::from_f64(y));
                assert_eq!(
                    exact_of(product).compare(&want),
                    Ordering::Equal,
                    "two_product({x:e}, {y:e}) = {product:?} is not exact"
                );
                checked += 1;
            }
        }
    }
    // A product near the largest Number of two factors below 2^996: the high halves of the split
    // round up and their product would overflow without the guard.
    let a = f64::MAX * pow2(-512);
    let b = pow2(512);
    let product = two_product(a, b);
    assert_eq!(product.hi, f64::MAX);
    assert_eq!(product.lo.to_bits(), 0);
    println!(
        "math_dd two_product scaling guard: {} pair(s) exact",
        checked + 1
    );
    assert_eq!(checked, large.len() * others.len() * 4);
}

#[test]
fn add_and_sub_stay_within_the_measured_bound() {
    let mut lcg = Lcg::new();
    let mut inputs = Vec::new();
    for i in 0..2_000 {
        let x = lcg.pair(-800, 1000);
        let y = match i % 4 {
            0 => {
                let hi = lcg.near(x.hi);
                Dd {
                    hi,
                    lo: lcg.low_part(hi),
                }
            }
            1 => Dd {
                hi: -x.hi,
                lo: lcg.low_part(x.hi),
            },
            2 => {
                let hi = -x.hi.next_up();
                Dd {
                    hi,
                    lo: lcg.low_part(hi),
                }
            }
            _ => lcg.pair(-800, 1000),
        };
        inputs.push((x, y));
    }
    let mut add = Worst::new();
    let mut sub = Worst::new();
    for &(x, y) in &inputs {
        let exact_x = exact_of(x);
        let exact_y = exact_of(y);
        add.see(error_of(x + y, &exact_x.add(&exact_y)));
        sub.see(error_of(x - y, &exact_x.sub(&exact_y)));
    }
    for (a, b) in boundary_pairs() {
        let (x, y) = (Dd::from_f64(a), Dd::from_f64(b));
        for (worst, result, want) in [
            (&mut add, x + y, Rat::from_f64(a).add(&Rat::from_f64(b))),
            (&mut sub, x - y, Rat::from_f64(a).sub(&Rat::from_f64(b))),
        ] {
            if result.hi.is_finite() && in_range(&want) {
                worst.see(error_of(result, &want));
            } else {
                worst.outside += 1;
            }
        }
    }
    add.hold("add", ADD_BOUND);
    sub.hold("sub", ADD_BOUND);
}

#[test]
fn mul_stays_within_the_measured_bound() {
    let mut lcg = Lcg::new();
    let mut worst = Worst::new();
    for _ in 0..2_000 {
        let x = lcg.pair(-450, 450);
        let y = lcg.pair(-450, 450);
        worst.see(error_of(x * y, &exact_of(x).mul(&exact_of(y))));
    }
    for (a, b) in boundary_pairs() {
        let result = Dd::from_f64(a) * Dd::from_f64(b);
        let want = Rat::from_f64(a).mul(&Rat::from_f64(b));
        if in_range(&want) {
            worst.see(error_of(result, &want));
        } else {
            worst.outside += 1;
        }
    }
    worst.hold("mul", MUL_BOUND);
}

#[test]
fn div_stays_within_the_measured_bound() {
    let mut lcg = Lcg::new();
    let mut worst = Worst::new();
    for _ in 0..2_000 {
        let x = lcg.pair(-450, 450);
        let y = lcg.pair(-450, 450);
        worst.see(error_of(x / y, &exact_of(x).div(&exact_of(y))));
    }
    for (a, b) in boundary_pairs() {
        let result = Dd::from_f64(a) / Dd::from_f64(b);
        if b == 0.0 {
            worst.outside += 1;
            continue;
        }
        let dividend = Rat::from_f64(a);
        let want = dividend.div(&Rat::from_f64(b));
        if in_range(&dividend) && in_range(&want) {
            assert!(
                result.hi.is_finite() && result.lo.is_finite(),
                "{a:e} / {b:e} = {result:?}"
            );
            worst.see(error_of(result, &want));
        } else {
            worst.outside += 1;
        }
    }
    worst.hold("div", DIV_BOUND);
}

#[test]
fn sqrt_stays_within_the_measured_bound() {
    let mut lcg = Lcg::new();
    let mut worst = Worst::new();
    for _ in 0..2_000 {
        let mut x = lcg.pair(-900, 1000);
        if x.hi < 0.0 {
            x = -x;
        }
        let root = x.sqrt();
        worst.see(sqrt_relative_error(&exact_of(root), &exact_of(x)));
    }
    for a in boundary_values() {
        let root = Dd::from_f64(a).sqrt();
        let value = Rat::from_f64(a);
        if a < 0.0 {
            assert!(root.hi.is_nan(), "sqrt({a:e}) = {root:?}");
            worst.outside += 1;
        } else if value.is_zero() {
            assert_eq!(root.hi.to_bits(), a.to_bits(), "sqrt({a:e}) = {root:?}");
            assert_eq!(root.lo, 0.0);
        } else if in_range(&value) {
            worst.see(sqrt_relative_error(&exact_of(root), &value));
        } else {
            worst.outside += 1;
        }
    }
    worst.hold("sqrt", SQRT_BOUND);
}

/// x × 2^k with one rounding to nearest, ties to even, from the bits alone: the reference that
/// `scale2` is compared with.
fn scaled_by_bits(x: f64, k: i32) -> f64 {
    if x == 0.0 || !x.is_finite() {
        return x;
    }
    let bits = x.to_bits();
    let sign = bits & (1 << 63);
    let biased = ((bits >> 52) & 0x7FF) as i64;
    let stored = bits & ((1 << 52) - 1);
    let (mut m, mut q) = if biased == 0 {
        (stored, -1074i64)
    } else {
        (stored | (1 << 52), biased - 1075)
    };
    // Normalize m to 53 bits.
    while m < (1 << 52) {
        m <<= 1;
        q -= 1;
    }
    q += i64::from(k);
    // The value is m × 2^q, m in [2^52, 2^53).
    let top = q + 52;
    if top > 1023 {
        return f64::from_bits(sign | 0x7FF0_0000_0000_0000);
    }
    if top >= -1022 {
        return f64::from_bits(sign | (((top + 1023) as u64) << 52) | (m - (1 << 52)));
    }
    // Subnormal: keep the bits at 2^-1074 and above.
    let drop = -1074 - q;
    if drop > 54 {
        return f64::from_bits(sign);
    }
    let kept = m >> drop;
    let dropped = m & ((1u64 << drop) - 1);
    let half = 1u64 << (drop - 1);
    let rounded = if dropped > half || (dropped == half && kept & 1 == 1) {
        kept + 1
    } else {
        kept
    };
    f64::from_bits(sign | rounded)
}

#[test]
fn scale2_is_exact_over_the_whole_exponent_range() {
    let values = [
        Dd { hi: 1.0, lo: 0.0 },
        Dd {
            hi: 1.5,
            lo: pow2(-60),
        },
        Dd {
            hi: -(1.0 + f64::EPSILON),
            lo: -pow2(-80),
        },
        Dd {
            hi: f64::MAX,
            lo: pow2(970) * 0.75,
        },
        Dd {
            hi: f64::MIN_POSITIVE,
            lo: 0.0,
        },
        Dd {
            hi: f64::from_bits(1),
            lo: 0.0,
        },
        Dd {
            hi: -f64::from_bits(0x000F_FFFF_FFFF_FFFF),
            lo: 0.0,
        },
        Dd {
            hi: 0.1,
            lo: 0.1 * pow2(-56),
        },
    ];
    let mut checked = 0;
    for k in -2_200..=2_200 {
        for x in values {
            // Above 2046 the two factors stop at 2^2046: exact for a high part of 2^-1022 or more
            // (whose result overflows either way), not for a subnormal one.
            if k > 2_046 && x.hi.abs() < f64::MIN_POSITIVE {
                continue;
            }
            let result = x.scale2(k);
            assert_eq!(
                result.hi.to_bits(),
                scaled_by_bits(x.hi, k).to_bits(),
                "{x:?}.scale2({k}): hi"
            );
            assert_eq!(
                result.lo.to_bits(),
                scaled_by_bits(x.lo, k).to_bits(),
                "{x:?}.scale2({k}): lo"
            );
            checked += 1;
        }
    }
    println!("math_dd scale2: {checked} scalings, k from -2200 to 2200");
    // 154 values of k above 2046, times the two subnormal high parts.
    assert_eq!(checked, 4_401 * 8 - 154 * 2);
}

#[test]
fn pair_from_f64_neg_and_to_f64() {
    let one = pair(0x3FF0_0000_0000_0000, 0x3C90_0000_0000_0000);
    assert_eq!(
        one,
        Dd {
            hi: 1.0,
            lo: pow2(-54)
        }
    );
    assert_eq!(one.to_f64(), 1.0);
    // hi + lo is the one rounding: 1 + 2^-53 is a tie and goes to the even 1.
    assert_eq!(
        Dd {
            hi: 1.0,
            lo: pow2(-53)
        }
        .to_f64(),
        1.0
    );
    assert_eq!(
        Dd {
            hi: 1.0 + f64::EPSILON,
            lo: pow2(-53)
        }
        .to_f64(),
        1.0 + 2.0 * f64::EPSILON
    );
    assert_eq!(Dd::from_f64(-2.5), Dd { hi: -2.5, lo: 0.0 });
    let negated = -one;
    assert_eq!((negated.hi, negated.lo), (-1.0, -pow2(-54)));
}

/// The reference lines: (x, y, lower bits, upper bits).
fn reference_lines() -> Vec<(f64, f64, u64, u64)> {
    let mut lines = Vec::new();
    for line in REFERENCE.lines() {
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split(' ').collect();
        assert_eq!(fields.len(), 5, "reference line {line:?}");
        assert_eq!(fields[0], "hypot", "reference line {line:?}");
        let bits = |text: &str| {
            assert_eq!(text.len(), 16, "reference line {line:?}");
            u64::from_str_radix(text, 16).unwrap()
        };
        lines.push((
            f64::from_bits(bits(fields[1])),
            f64::from_bits(bits(fields[2])),
            bits(fields[3]),
            bits(fields[4]),
        ));
    }
    assert_eq!(lines.len(), REFERENCE_LINES);
    lines
}

#[test]
fn hypot_is_one_of_the_two_numbers_of_each_reference_line() {
    let mut exact = 0;
    let mut lower = 0;
    for (x, y, low, high) in reference_lines() {
        let r = hypot(&[x, y]);
        assert!(
            r.to_bits() == low || r.to_bits() == high,
            "hypot({x:e}, {y:e}) = {r:e} ({:016X}), the reference gives {low:016X} {high:016X}",
            r.to_bits()
        );
        if low == high {
            exact += 1;
        } else if r.to_bits() == low {
            lower += 1;
        }
    }
    println!(
        "math_dd hypot reference: {REFERENCE_LINES} line(s), {exact} exact, {lower} at the lower Number, {} at the upper",
        REFERENCE_LINES - exact - lower
    );
}

#[test]
fn hypot_of_the_reference_pairs_is_faithful_to_exact_arithmetic() {
    let mut overflow = 0;
    for (x, y, low, high) in reference_lines() {
        let square = Rat::from_f64(x)
            .mul(&Rat::from_f64(x))
            .add(&Rat::from_f64(y).mul(&Rat::from_f64(y)));
        let r = hypot(&[x, y]);
        let (low, high) = (f64::from_bits(low), f64::from_bits(high));
        if high.is_infinite() {
            // Beyond the largest Number: the enclosure is (largest Number, +∞).
            assert!(Rat::from_f64(low).mul(&Rat::from_f64(low)).compare(&square) == Ordering::Less);
            assert!(
                r == f64::MAX || r == f64::INFINITY,
                "hypot({x:e}, {y:e}) = {r:e}"
            );
            overflow += 1;
            continue;
        }
        // The reference line itself, checked exactly: low² ≤ x² + y² ≤ high².
        let low_square = Rat::from_f64(low).mul(&Rat::from_f64(low));
        let high_square = Rat::from_f64(high).mul(&Rat::from_f64(high));
        assert!(
            low_square.compare(&square) != Ordering::Greater,
            "reference line of ({x:e}, {y:e}): lower"
        );
        assert!(
            high_square.compare(&square) != Ordering::Less,
            "reference line of ({x:e}, {y:e}): upper"
        );
        assert!(is_faithful_root(r, &square), "hypot({x:e}, {y:e}) = {r:e}");
    }
    println!(
        "math_dd hypot exact: {REFERENCE_LINES} pair(s), {overflow} beyond the largest Number"
    );
}

#[test]
fn hypot_of_1000_arguments_of_equal_size_is_faithful() {
    let mut lcg = Lcg::new();
    for e in [-1000, 0, 1000] {
        let values: Vec<f64> = (0..1_000).map(|_| lcg.number(e, e)).collect();
        let mut square = Rat::zero();
        for &v in &values {
            let exact = Rat::from_f64(v);
            square = square.add(&exact.mul(&exact));
        }
        let r = hypot(&values);
        assert!(
            is_faithful_root(r, &square),
            "hypot of 1,000 values of 2^{e}: {r:e}"
        );
        println!("math_dd hypot 1,000 values of exponent {e}: {r:e}");
    }
}

#[test]
fn hypot_special_values_and_exact_results() {
    assert_eq!(hypot(&[3.0, 4.0]), 5.0);
    assert_eq!(hypot(&[-3.0, 4.0]), 5.0);
    assert_eq!(hypot(&[]).to_bits(), 0);
    assert_eq!(hypot(&[-0.0]).to_bits(), 0);
    assert_eq!(hypot(&[-0.0, 0.0, -0.0]).to_bits(), 0);
    assert_eq!(hypot(&[f64::INFINITY, f64::NAN]), f64::INFINITY);
    assert_eq!(hypot(&[f64::NAN, f64::NEG_INFINITY]), f64::INFINITY);
    assert_eq!(hypot(&[f64::NEG_INFINITY]), f64::INFINITY);
    assert!(hypot(&[f64::NAN]).is_nan());
    assert!(hypot(&[1.0, f64::NAN]).is_nan());
    assert!(hypot(&[0.0, f64::NAN]).is_nan());
    assert_eq!(hypot(&[-5.0]), 5.0);
    assert_eq!(hypot(&[-f64::from_bits(1)]), f64::from_bits(1));
    assert_eq!(hypot(&[f64::MAX]), f64::MAX);
    assert_eq!(hypot(&[1.0, 2.0, 2.0]), 3.0);
}

#[test]
fn hypot_of_1e308_twice_is_finite() {
    let r = hypot(&[1e308, 1e308]);
    assert_eq!(r.to_bits(), 0x7FE9_2C80_954C_51F5);
    assert_eq!(format!("{r:e}"), "1.4142135623730951e308");
    // The literal one digit shorter is the Number one below.
    assert_eq!(
        1.414_213_562_373_095e308_f64.to_bits(),
        0x7FE9_2C80_954C_51F4
    );
    assert_ne!(r, 1.414_213_562_373_095e308);
}

#[test]
fn hypot_of_the_two_smallest_subnormals() {
    let tiny = f64::from_bits(1);
    let r = hypot(&[tiny, tiny]);
    let square = Rat::from_f64(tiny)
        .mul(&Rat::from_f64(tiny))
        .mul(&Rat::from_i64(2));
    assert!(
        is_faithful_root(r, &square),
        "hypot(5e-324, 5e-324) = {r:e}"
    );
    assert!(r == tiny || r == 2.0 * tiny);
    println!("math_dd hypot(5e-324, 5e-324) = {r:e}");
}
