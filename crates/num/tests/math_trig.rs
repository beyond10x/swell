//! The trigonometric functions of `swell_num::math`: `sin`, `cos`, `tan` (`#sec-math.sin`,
//! `#sec-math.cos`, `#sec-math.tan`) over Payne and Hanek's reduction, and `atan`, `atan2`,
//! `asin`, `acos` (`#sec-math.atan`, `#sec-math.atan2`, `#sec-math.asin`, `#sec-math.acos`)
//! (design, decision 6, "The functions of `Math` that IEEE 754 does not fix", items 1 and 4).
//!
//! What this file holds, each against exact integer arithmetic or the committed reference lines
//! of `data/math/reference-trig.txt` (made once by GNU bc 1.08.2, run as `bc -l`, by the
//! procedure of the design's "The tool that makes the lines"; its header says how):
//!
//! - Every constant is recomputed from its definition and compared bit for bit: π by Machin's
//!   formula to 1,400 bits with a bound of its error, the 1,280 bits of 2/π as the quotient of a
//!   power of two by it, π/2 as a pair, the 9 values of atan(k/8) by their series (atan(1) as
//!   π/4), the series coefficients as the exact fractions; and the degree of each series by the
//!   design's rule (the first omitted term below 2^-80 of the result on the whole interval).
//! - `reduce` against the exact reduction: its quadrant, and r within 2^-100 of x - k π/2
//!   relative, for every argument of `sin`, `cos` and `tan` in the reference lines, among them
//!   the 40 that the search found nearest to a multiple of π/2 and 6381956970095103 × 2^797.
//! - The reference lines: group A (`sin`, `cos`, `atan`) gives one of the two Numbers of the
//!   line, group B (`tan`, `asin`, `acos`, `atan2`) a Number at most 2 ulp from the nearer.
//! - A denser walk of its own, against a reference of about 127 bits computed here in integers
//!   ([`Real`]): the largest error in ulp of each function, held to its group.
//! - The values the draft fixes, and NaN, ±∞, ±0 and a subnormal for every function.

#![cfg(test)]

// The shared exact arithmetic, included as `tests/math_dd.rs` includes it; a second include of
// one file is what `clippy::duplicate_mod` names, and the crate has no `tests/support/mod.rs`.
#[allow(clippy::duplicate_mod)]
#[path = "support/exact.rs"]
mod exact;

use std::cmp::Ordering;
use std::time::Instant;

use exact::{Big, Rat, relative_error};
use swell_num::math::atan::{ATAN_COEFFICIENTS, ATAN_OF_EIGHTHS};
use swell_num::math::dd::Dd;
use swell_num::math::reduce::{PI_OVER_2, TWO_OVER_PI, reduce};
use swell_num::math::trig::{COS_COEFFICIENTS, SIN_COEFFICIENTS};
use swell_num::math::{acos, asin, atan, atan2, cos, sin, tan};

/// The reference lines and their count per function: for `sin`, `cos` and `tan` 20 special
/// arguments, 200 drawn and the 40 nearest to a multiple of π/2; for `atan` 18 special and 200
/// drawn, for `asin` and `acos` 14 and 200, for `atan2` 16 pairs and 200.
const REFERENCE: &str = include_str!("../../../data/math/reference-trig.txt");
const REFERENCE_COUNTS: [(&str, usize); 7] = [
    ("sin", 260),
    ("cos", 260),
    ("tan", 260),
    ("atan", 218),
    ("asin", 214),
    ("acos", 214),
    ("atan2", 216),
];

/// The fractional bits of every transcendental constant this file computes (the design: "each to
/// 1,400 bits").
const BITS: u64 = 1_400;

/// The largest Number not above π/4: no reduction up to it.
const PI_OVER_4_BELOW: f64 = f64::from_bits(0x3FE9_21FB_5444_2D18);

// ---------------------------------------------------------------------------------------------
// Constants by integers.

/// 2^bits atan(1/n) as Σ (-1)^k floor(2^bits / (n^(2k+1) (2k+1))), and a bound of its error in
/// units. Each term is that floor exactly (floor(floor(a / b) / c) = floor(a / (b c)) for
/// integers), short of the exact term by less than one unit; the sum stops at the first power
/// that is zero, and the alternating tail after it is below that power's exact value, under one
/// unit. So the error is below the count of terms plus one.
fn atan_of_inverse(n: u64, bits: u64) -> (Big, u64) {
    let mut power = Big::from_u64(1).shl(bits).div_small(n);
    let mut plus = Big::zero();
    let mut minus = Big::zero();
    let mut k = 0u64;
    while !power.is_zero() {
        let term = power.div_small(2 * k + 1);
        if k.is_multiple_of(2) {
            plus = plus.add(&term);
        } else {
            minus = minus.add(&term);
        }
        power = power.div_small(n * n);
        k += 1;
    }
    (plus.sub(&minus), k + 1)
}

/// π × 2^bits by Machin's formula, π = 16 atan(1/5) - 4 atan(1/239), with a bound of its error.
struct Pi {
    value: Big,
    error: u64,
    bits: u64,
}

impl Pi {
    fn new(bits: u64) -> Pi {
        let (a5, e5) = atan_of_inverse(5, bits);
        let (a239, e239) = atan_of_inverse(239, bits);
        Pi {
            value: a5.mul(&Big::from_u64(16)).sub(&a239.mul(&Big::from_u64(4))),
            error: 16 * e5 + 4 * e239,
            bits,
        }
    }

    fn low(&self) -> Big {
        self.value.sub(&Big::from_u64(self.error))
    }

    fn high(&self) -> Big {
        self.value.add(&Big::from_u64(self.error))
    }

    /// π / 2^shift as an interval of two rationals.
    fn over_power_of_two(&self, shift: u64) -> (Rat, Rat) {
        let den = Big::from_u64(1).shl(self.bits + shift);
        (
            Rat::new(false, self.low(), den.clone()),
            Rat::new(false, self.high(), den),
        )
    }

    /// floor(2^w × 2/π) from each end of the interval: (from the upper π, from the lower π).
    fn two_over_pi(&self, w: u64) -> (Big, Big) {
        let numerator = Big::from_u64(1).shl(w + 1 + self.bits);
        (
            numerator.div_floor(&self.high()),
            numerator.div_floor(&self.low()),
        )
    }
}

/// 2^bits atan(k/8) for k from 1 to 7 by its series Σ (-1)^j (k/8)^(2j+1) / (2j+1), with a
/// bound of its error in units. The power p_j ≈ 2^bits (k/8)^(2j+1) is p_(j+1) = floor(p_j k^2
/// / 64), short of its exact value by less than 64 / (64 - k^2) ≤ 64/15 < 5 units (each step
/// scales the shortfall by k^2/64 and adds under one); a term floor(p_j / (2j+1)) is short by
/// less than 6. The tail after the first zero power is below 5 units.
fn atan_of_eighth(k: u64, bits: u64) -> (Big, u64) {
    let mut power = Big::from_u64(k).shl(bits - 3);
    let square = Big::from_u64(k * k);
    let mut plus = Big::zero();
    let mut minus = Big::zero();
    let mut j = 0u64;
    while !power.is_zero() {
        let term = power.div_small(2 * j + 1);
        if j.is_multiple_of(2) {
            plus = plus.add(&term);
        } else {
            minus = minus.add(&term);
        }
        power = power.mul(&square).shr(6);
        j += 1;
    }
    (plus.sub(&minus), 6 * j + 5)
}

/// The pair of a rational: the Number nearest to it and the Number nearest to the rest, as bits.
fn pair_of(value: &Rat) -> (u64, u64) {
    let hi = value.to_f64();
    let lo = value.sub(&Rat::from_f64(hi)).to_f64();
    (hi.to_bits(), lo.to_bits())
}

/// The pair of every value in [low, high]: the pairs of both ends, which must agree.
fn pair_of_interval(name: &str, low: &Rat, high: &Rat) -> (u64, u64) {
    let pair = pair_of(low);
    assert_eq!(
        pair,
        pair_of(high),
        "{name}: the interval is too wide to fix the pair"
    );
    pair
}

fn bits_of(pair: Dd) -> (u64, u64) {
    (pair.hi.to_bits(), pair.lo.to_bits())
}

fn power_of(x: &Rat, n: u64) -> Rat {
    let mut result = Rat::from_i64(1);
    for _ in 0..n {
        result = result.mul(x);
    }
    result
}

fn factorial(n: u64) -> Big {
    let mut result = Big::from_u64(1);
    for k in 2..=n {
        result = result.mul(&Big::from_u64(k));
    }
    result
}

fn fraction(numerator: u64, denominator: u64) -> Rat {
    Rat::new(false, Big::from_u64(numerator), Big::from_u64(denominator))
}

#[test]
fn two_over_pi_and_pi_over_2_are_recomputed_by_machin_s_formula() {
    let start = Instant::now();
    let pi = Pi::new(BITS);
    // floor(2^1280 × 2/π) from both ends of π's interval: one integer, the table's 20 words.
    let (from_high, from_low) = pi.two_over_pi(1_280);
    assert_eq!(
        from_high, from_low,
        "π to {BITS} bits does not fix 1,280 bits of 2/π"
    );
    let mut table = Big::zero();
    for word in TWO_OVER_PI {
        table = table.shl(64).add(&Big::from_u64(word));
    }
    assert_eq!(table, from_low, "TWO_OVER_PI is not floor(2^1280 × 2/π)");
    // 2/π lies in [1/2, 1): its first fractional bit is set, so the table holds 1,280 bits.
    assert_eq!(table.bit_len(), 1_280);
    let (low, high) = pi.over_power_of_two(1);
    assert_eq!(
        bits_of(PI_OVER_2),
        pair_of_interval("π/2", &low, &high),
        "PI_OVER_2"
    );
    // The window `reduce` reads: bits e - 1 to e + 254 of 2/π for the binary exponent e of the
    // last bit of the mantissa, at most 971 (the largest Number): up to bit 1,225 of 1,280.
    println!(
        "math_trig constants: π to {BITS} bits within {} unit(s); 2/π, 1,280 bits, and π/2 \
         as a pair bit for bit; reduce reads up to bit {} of 2/π; {:?}",
        pi.error,
        971 + 254,
        start.elapsed()
    );
}

#[test]
fn atan_of_the_eighths_are_their_series() {
    let start = Instant::now();
    let pi = Pi::new(BITS);
    let den = Big::from_u64(1).shl(BITS);
    for (k, pair) in ATAN_OF_EIGHTHS.iter().enumerate() {
        let k = k as u64;
        let (low, high) = match k {
            0 => (Rat::zero(), Rat::zero()),
            8 => pi.over_power_of_two(2),
            _ => {
                let (value, error) = atan_of_eighth(k, BITS);
                (
                    Rat::new(false, value.sub(&Big::from_u64(error)), den.clone()),
                    Rat::new(false, value.add(&Big::from_u64(error)), den.clone()),
                )
            }
        };
        assert_eq!(
            bits_of(*pair),
            pair_of_interval(&format!("atan({k}/8)"), &low, &high),
            "ATAN_OF_EIGHTHS[{k}]"
        );
    }
    println!(
        "math_trig constants: the 9 values atan(k/8) bit for bit, by their series to {BITS} bits \
         (atan(1) as π/4); {:?}",
        start.elapsed()
    );
}

#[test]
fn series_coefficients_are_their_fractions_and_the_degrees_follow_the_rule() {
    // The coefficients: sin -1/3!, 1/5!, ..., 1/21!; cos -1/2!, 1/4!, ..., -1/22!; atan -1/3,
    // 1/5, ..., -1/19.
    for (i, pair) in SIN_COEFFICIENTS.iter().enumerate() {
        let n = 2 * i as u64 + 3;
        let value = Rat::new(i.is_multiple_of(2), Big::from_u64(1), factorial(n));
        assert_eq!(bits_of(*pair), pair_of(&value), "sin coefficient 1/{n}!");
    }
    for (i, pair) in COS_COEFFICIENTS.iter().enumerate() {
        let n = 2 * i as u64 + 2;
        let value = Rat::new(i.is_multiple_of(2), Big::from_u64(1), factorial(n));
        assert_eq!(bits_of(*pair), pair_of(&value), "cos coefficient 1/{n}!");
    }
    for (i, pair) in ATAN_COEFFICIENTS.iter().enumerate() {
        let n = 2 * i as u64 + 3;
        let value = Rat::new(i.is_multiple_of(2), Big::from_u64(1), Big::from_u64(n));
        assert_eq!(bits_of(*pair), pair_of(&value), "atan coefficient 1/{n}");
    }

    // The degrees by the design's rule: a series is cut where its first omitted term is below
    // 2^-80 of the result on the whole interval. For sin and cos on [0, π/4] the ratio of a
    // term r^n/n! to the result grows with r, so it is bounded at π/4: by R = 0.7854 ≥ π/4 for
    // the term and 0.7071 ≤ √2/2 = sin(π/4) = cos(π/4) for the result. That the degree before
    // is not enough is shown at R' = 0.7853 ≤ π/4 with sin(π/4) ≤ 0.7072 and cos ≤ 1.
    let pi = Pi::new(BITS);
    let (pi_4_low, pi_4_high) = pi.over_power_of_two(2);
    let r_up = fraction(7_854, 10_000);
    let r_down = fraction(7_853, 10_000);
    assert_eq!(r_up.compare(&pi_4_high), Ordering::Greater);
    assert_eq!(r_down.compare(&pi_4_low), Ordering::Less);
    let root_low = fraction(7_071, 10_000);
    let root_high = fraction(7_072, 10_000);
    let half = fraction(1, 2);
    assert_eq!(root_low.mul(&root_low).compare(&half), Ordering::Less);
    assert_eq!(root_high.mul(&root_high).compare(&half), Ordering::Greater);
    let limit = Rat::pow2(-80);
    let term =
        |r: &Rat, n: u64| power_of(r, n).div(&Rat::new(false, factorial(n), Big::from_u64(1)));
    let below = |t: Rat, result: &Rat| t.compare(&limit.mul(result)) == Ordering::Less;

    // sin: odd degrees.
    let mut sin_degree = 1;
    while !below(term(&r_up, sin_degree + 2), &root_low) {
        sin_degree += 2;
    }
    assert!(
        !below(term(&r_down, sin_degree), &root_high),
        "sin: degree {} would do",
        sin_degree - 2
    );
    // cos: even degrees.
    let mut cos_degree = 0;
    while !below(term(&r_up, cos_degree + 2), &root_low) {
        cos_degree += 2;
    }
    assert!(
        !below(term(&r_down, cos_degree), &Rat::from_i64(1)),
        "cos: degree {} would do",
        cos_degree - 2
    );
    // atan on |u| ≤ 1/16 (and a margin of 2^-40 for the rounding of the quotient): the term
    // u^n/n against atan(u) ≥ u - u^3/3, the ratio growing with u. The degree before fails at
    // u = 1/16 with atan(u) ≤ u.
    let u_up = fraction(1, 16).add(&Rat::pow2(-40));
    let u_down = fraction(1, 16);
    let atan_low = u_up.sub(&power_of(&u_up, 3).div(&Rat::from_i64(3)));
    let atan_term = |u: &Rat, n: u64| power_of(u, n).div(&Rat::from_i64(n as i64));
    let mut atan_degree = 1;
    while !below(atan_term(&u_up, atan_degree + 2), &atan_low) {
        atan_degree += 2;
    }
    assert!(
        !below(atan_term(&u_down, atan_degree), &u_down),
        "atan: degree {} would do",
        atan_degree - 2
    );
    println!(
        "math_trig degrees by the rule: sin r^{sin_degree} (design r^21), cos r^{cos_degree} \
         (design r^22), atan u^{atan_degree} (design u^19)"
    );
    assert_eq!(sin_degree, 2 * SIN_COEFFICIENTS.len() as u64 + 1);
    assert_eq!(cos_degree, 2 * COS_COEFFICIENTS.len() as u64);
    assert_eq!(atan_degree, 2 * ATAN_COEFFICIENTS.len() as u64 + 1);
}

// ---------------------------------------------------------------------------------------------
// The reference lines.

/// A line: the function, its arguments (the second only for atan2) and the two Numbers.
struct Line {
    function: String,
    x: f64,
    y: f64,
    lower: f64,
    upper: f64,
}

fn reference_lines() -> Vec<Line> {
    let mut lines = Vec::new();
    for text in REFERENCE.lines() {
        if text.starts_with('#') || text.trim().is_empty() {
            continue;
        }
        let fields: Vec<&str> = text.split(' ').collect();
        let bits = |field: &str| {
            assert_eq!(field.len(), 16, "reference line {text:?}");
            f64::from_bits(u64::from_str_radix(field, 16).unwrap())
        };
        let line = match fields.as_slice() {
            ["atan2", y, x, lower, upper] => Line {
                function: "atan2".to_string(),
                x: bits(y),
                y: bits(x),
                lower: bits(lower),
                upper: bits(upper),
            },
            [function, x, lower, upper] => Line {
                function: (*function).to_string(),
                x: bits(x),
                y: 0.0,
                lower: bits(lower),
                upper: bits(upper),
            },
            _ => panic!("reference line {text:?}"),
        };
        lines.push(line);
    }
    for (function, count) in REFERENCE_COUNTS {
        assert_eq!(
            lines.iter().filter(|l| l.function == function).count(),
            count,
            "reference lines of {function}"
        );
    }
    assert_eq!(
        lines.len(),
        REFERENCE_COUNTS.iter().map(|(_, n)| n).sum::<usize>()
    );
    lines
}

/// The function of a line; for atan2 x is the first argument (y of the function), y the second.
fn evaluate(function: &str, x: f64, y: f64) -> f64 {
    match function {
        "sin" => sin(x),
        "cos" => cos(x),
        "tan" => tan(x),
        "atan" => atan(x),
        "asin" => asin(x),
        "acos" => acos(x),
        "atan2" => atan2(x, y),
        _ => panic!("no function {function}"),
    }
}

fn group_a(function: &str) -> bool {
    matches!(function, "sin" | "cos" | "atan")
}

#[test]
fn each_reference_line_holds_its_function_to_its_group() {
    let start = Instant::now();
    let reference = Reference::new();
    let lines = reference_lines();
    for (function, count) in REFERENCE_COUNTS {
        let mut at_lower = 0;
        let mut exact = 0;
        let mut worst = 0.0f64;
        let mut worst_line = String::new();
        for line in lines.iter().filter(|l| l.function == function) {
            let (x, y) = (line.x, line.y);
            let r = evaluate(function, x, y);
            // The line agrees with the reference of this file: the exact value lies between
            // its two Numbers.
            let want = reference.evaluate(function, x, y);
            let slack = 1.0 + 1e-15;
            assert!(
                ulp_error(line.lower, want) <= slack && ulp_error(line.upper, want) <= slack,
                "{function}({x:e}, {y:e}): {:e} {:e} do not enclose the reference",
                line.lower,
                line.upper
            );
            if group_a(function) {
                assert!(
                    r.to_bits() == line.lower.to_bits() || r.to_bits() == line.upper.to_bits(),
                    "{function}({x:e}, {y:e}) = {r:e} ({:016X}), the line gives {:016X} {:016X}",
                    r.to_bits(),
                    line.lower.to_bits(),
                    line.upper.to_bits()
                );
            } else {
                let gap = if line.lower == line.upper {
                    line.lower.abs().next_up() - line.lower.abs()
                } else {
                    (line.upper - line.lower).abs()
                };
                let distance = (r - line.lower).abs().min((r - line.upper).abs());
                assert!(
                    distance <= 2.0 * gap,
                    "{function}({x:e}, {y:e}) = {r:e}, more than 2 ulp from {:e} {:e}",
                    line.lower,
                    line.upper
                );
            }
            if line.lower == line.upper {
                exact += 1;
            } else if r.to_bits() == line.lower.to_bits() {
                at_lower += 1;
            }
            let error = ulp_error(r, want);
            if error > worst {
                worst = error;
                worst_line = format!("{x:e} {y:e}");
            }
        }
        let bound = if group_a(function) { "< 1" } else { "≤ 2" };
        println!(
            "math_trig reference {function}: {count} line(s), {exact} exact, {at_lower} at the \
             Number nearer zero; largest error {worst:.6} ulp at ({worst_line}), bound {bound} ulp"
        );
        if group_a(function) {
            assert!(worst < 1.0, "{function}: {worst} ulp");
        } else {
            assert!(worst <= 2.0, "{function}: {worst} ulp");
        }
    }
    // The largest arguments of sin, cos and tan are in the lines.
    for function in ["sin", "cos", "tan"] {
        for x in [f64::from_bits(0x7FE0_0000_0000_0000), f64::MAX, -f64::MAX] {
            assert!(
                lines
                    .iter()
                    .any(|l| l.function == function && l.x.to_bits() == x.to_bits()),
                "{function}({x:e}) is not in the reference lines"
            );
        }
    }
    println!(
        "math_trig reference: {} line(s) in {:?}",
        lines.len(),
        start.elapsed()
    );
}

// ---------------------------------------------------------------------------------------------
// The reduction against exact arithmetic.

/// The exact reduction of a finite x with |x| above π/4: the quadrant k mod 4 and x - k π/2 for
/// the nearest integer k, as an interval from π's. `t` is floor(2^BITS × 2/π).
fn exact_reduction(x: f64, pi: &Pi, t: &Big) -> (u8, Rat, Rat) {
    let bits = x.abs().to_bits();
    let biased = (bits >> 52) as i64;
    let mantissa = (bits & ((1 << 52) - 1)) | (1 << 52);
    let q = biased - 1075;
    // |x| × 2/π × 2^BITS ≈ mantissa × t × 2^q; k = floor(that / 2^(BITS - q) + 1/2).
    let shift = (BITS as i64 - q) as u64;
    let product = Big::from_u64(mantissa).mul(t);
    let k = product.add(&Big::from_u64(1).shl(shift - 1)).shr(shift);
    let quadrant = (k.low_u64() & 3) as u8;
    let magnitude = Rat::from_f64(x.abs());
    let den = Big::from_u64(1).shl(BITS + 1);
    let near = magnitude.sub(&Rat::new(false, k.mul(&pi.high()), den.clone()));
    let far = magnitude.sub(&Rat::new(false, k.mul(&pi.low()), den));
    if x < 0.0 {
        (quadrant.wrapping_neg() & 3, near.neg(), far.neg())
    } else {
        (quadrant, near, far)
    }
}

#[test]
fn reduce_is_within_2_to_the_minus_100_of_the_exact_reduction() {
    let start = Instant::now();
    let pi = Pi::new(BITS);
    let (_, t) = pi.two_over_pi(BITS);
    let lines = reference_lines();
    // The lines of sin end with the 40 arguments of the search (the header, step 5).
    let searched: Vec<u64> = lines
        .iter()
        .filter(|l| l.function == "sin")
        .skip(220)
        .map(|l| l.x.to_bits())
        .collect();
    assert_eq!(searched.len(), 40);
    let mut arguments: Vec<f64> = lines
        .iter()
        .filter(|l| matches!(l.function.as_str(), "sin" | "cos" | "tan"))
        .map(|l| l.x)
        .collect();
    arguments.sort_by_key(|x| x.to_bits());
    arguments.dedup_by_key(|x| x.to_bits());
    let limit = Rat::pow2(-100);
    let mut worst = Rat::zero();
    let mut worst_searched = Rat::zero();
    let mut closest: Option<(Rat, f64)> = None;
    let mut closest_searched: Option<(Rat, f64)> = None;
    let mut reduced = 0;
    let mut near = 0;
    for &x in &arguments {
        let (quadrant, r) = reduce(x);
        if x.abs() <= PI_OVER_4_BELOW {
            assert_eq!((quadrant, r.hi.to_bits(), r.lo), (0, x.to_bits(), 0.0));
            continue;
        }
        reduced += 1;
        let (k, low, high) = exact_reduction(x, &pi, &t);
        assert_eq!(quadrant, k, "reduce({x:e}): quadrant");
        let value = Rat::from_dd(r.hi, r.lo);
        let mut error = relative_error(&value, &low);
        let other = relative_error(&value, &high);
        if other.compare(&error) == Ordering::Greater {
            error = other;
        }
        assert!(
            error.compare(&limit) != Ordering::Greater,
            "reduce({x:e}) = {r:?}: relative error 2^{:.2}",
            error.log2()
        );
        if error.compare(&worst) == Ordering::Greater {
            worst = error.clone();
        }
        let magnitude = low.abs();
        let nearer = |best: &Option<(Rat, f64)>| {
            best.as_ref()
                .is_none_or(|(c, _)| magnitude.compare(c) == Ordering::Less)
        };
        if searched.contains(&x.to_bits()) {
            near += 1;
            if error.compare(&worst_searched) == Ordering::Greater {
                worst_searched = error;
            }
            if nearer(&closest_searched) {
                closest_searched = Some((magnitude.clone(), x));
            }
        }
        if nearer(&closest) {
            closest = Some((magnitude, x));
        }
    }
    let (closest, at) = closest.unwrap();
    let (closest_searched, at_searched) = closest_searched.unwrap();
    println!(
        "math_trig reduce: {} argument(s), {reduced} reduced; largest relative error of r 2^{:.2} \
         (design at most 2^-100), over the {near} searched arguments 2^{:.2}; closest approach \
         |x - k π/2| of the search 2^{:.2} at x = {at_searched:e} ({:016X}), of all 2^{:.2} at \
         x = {at:e} ({:016X}) (design about 2^-61); {:?}",
        arguments.len(),
        worst.log2(),
        worst_searched.log2(),
        closest_searched.log2(),
        at_searched.to_bits(),
        closest.log2(),
        at.to_bits(),
        start.elapsed()
    );
    assert_eq!(near, 40);
}

// ---------------------------------------------------------------------------------------------
// A reference of about 127 bits, in integers.

/// A real number (-1)^negative × m × 2^e, m normalized (bit 127 set), or zero (m = 0). Each
/// operation truncates to 128 bits: a relative error below 2^-126.
#[derive(Clone, Copy, Debug)]
struct Real {
    negative: bool,
    m: u128,
    e: i64,
}

const LOW_64: u128 = (1 << 64) - 1;

impl Real {
    const ZERO: Real = Real {
        negative: false,
        m: 0,
        e: 0,
    };
    const ONE: Real = Real {
        negative: false,
        m: 1 << 127,
        e: -127,
    };

    fn normalized(negative: bool, m: u128, e: i64) -> Real {
        if m == 0 {
            return Real::ZERO;
        }
        let shift = m.leading_zeros();
        Real {
            negative,
            m: m << shift,
            e: e - i64::from(shift),
        }
    }

    fn from_f64(x: f64) -> Real {
        let bits = x.to_bits();
        let biased = ((bits >> 52) & 0x7FF) as i64;
        let stored = bits & ((1 << 52) - 1);
        let (m, e) = if biased == 0 {
            (stored, -1074)
        } else {
            (stored | (1 << 52), biased - 1075)
        };
        Real::normalized(x.is_sign_negative(), u128::from(m), e)
    }

    /// value × 2^-fraction_bits, its top 128 bits.
    fn from_big(value: &Big, fraction_bits: u64) -> Real {
        let length = value.bit_len();
        assert!(length >= 128);
        let drop = length - 128;
        let high = value.shr(drop + 64).low_u64();
        let low = value.shr(drop).low_u64();
        Real::normalized(
            false,
            (u128::from(high) << 64) | u128::from(low),
            drop as i64 - fraction_bits as i64,
        )
    }

    fn negated(self) -> Real {
        Real {
            negative: !self.negative && self.m != 0,
            ..self
        }
    }

    fn abs(self) -> Real {
        Real {
            negative: false,
            ..self
        }
    }

    fn times(self, other: Real) -> Real {
        if self.m == 0 || other.m == 0 {
            return Real::ZERO;
        }
        let (a1, a0) = (self.m >> 64, self.m & LOW_64);
        let (b1, b0) = (other.m >> 64, other.m & LOW_64);
        let (hh, hl, lh, ll) = (a1 * b1, a1 * b0, a0 * b1, a0 * b0);
        let middle = (hl & LOW_64) + (lh & LOW_64) + (ll >> 64);
        let high = hh + (hl >> 64) + (lh >> 64) + (middle >> 64);
        let next = (middle & LOW_64) << 64;
        let negative = self.negative != other.negative;
        if high >> 127 == 1 {
            Real {
                negative,
                m: high,
                e: self.e + other.e + 128,
            }
        } else {
            Real {
                negative,
                m: (high << 1) | (next >> 127),
                e: self.e + other.e + 127,
            }
        }
    }

    fn plus(self, other: Real) -> Real {
        if self.m == 0 {
            return other;
        }
        if other.m == 0 {
            return self;
        }
        let (big, small) = if (self.e, self.m) >= (other.e, other.m) {
            (self, other)
        } else {
            (other, self)
        };
        let shift = big.e - small.e;
        let aligned = if shift >= 128 { 0 } else { small.m >> shift };
        if big.negative == small.negative {
            let (sum, carry) = big.m.overflowing_add(aligned);
            if carry {
                Real {
                    negative: big.negative,
                    m: (sum >> 1) | (1 << 127),
                    e: big.e + 1,
                }
            } else {
                Real {
                    negative: big.negative,
                    m: sum,
                    e: big.e,
                }
            }
        } else {
            Real::normalized(big.negative, big.m - aligned, big.e)
        }
    }

    fn minus(self, other: Real) -> Real {
        self.plus(other.negated())
    }

    /// The quotient: floor(self.m × 2^127 / other.m) by long division.
    fn over(self, other: Real) -> Real {
        assert!(other.m != 0, "Real::over zero");
        if self.m == 0 {
            return Real::ZERO;
        }
        let divisor = other.m;
        let mut rest = self.m;
        let mut carry = false;
        let mut quotient = 0u128;
        for _ in 0..128 {
            quotient <<= 1;
            if carry || rest >= divisor {
                rest = rest.wrapping_sub(divisor);
                quotient |= 1;
            }
            carry = rest >> 127 == 1;
            rest <<= 1;
        }
        Real::normalized(
            self.negative != other.negative,
            quotient,
            self.e - other.e - 127,
        )
    }

    fn over_small(self, n: u64) -> Real {
        Real::normalized(self.negative, self.m / u128::from(n), self.e)
    }

    fn half(self) -> Real {
        Real {
            e: self.e - 1,
            ..self
        }
    }

    /// The root of a positive value: three Newton steps from the root of its nearest Number.
    fn sqrt(self) -> Real {
        if self.m == 0 {
            return Real::ZERO;
        }
        let mut root = Real::from_f64(self.approx().sqrt());
        for _ in 0..3 {
            root = root.plus(self.over(root)).half();
        }
        root
    }

    fn greater_than_one(self) -> bool {
        !self.negative && (self.e, self.m) > (Real::ONE.e, Real::ONE.m)
    }

    /// The value as a Number, by its top 53 bits (for printing and for choosing an eighth).
    fn approx(self) -> f64 {
        if self.m == 0 {
            return 0.0;
        }
        let top = (self.m >> 75) as u64 as f64;
        let value = top * power_of_two(self.e + 75);
        if self.negative { -value } else { value }
    }
}

/// 2^k as a Number, 0 below 2^-1074 and +∞ above 2^1023.
fn power_of_two(k: i64) -> f64 {
    if k > 1023 {
        f64::INFINITY
    } else if k >= -1022 {
        f64::from_bits(((k + 1023) as u64) << 52)
    } else if k >= -1074 {
        f64::from_bits(1 << (k + 1074))
    } else {
        0.0
    }
}

/// |result - exact| in ulp of the exact value (the gap between the two Numbers that enclose
/// it, 2^-1074 below 2^-1022); +∞ for a result that is not finite.
fn ulp_error(result: f64, exact: Real) -> f64 {
    if !result.is_finite() {
        return f64::INFINITY;
    }
    let difference = Real::from_f64(result).minus(exact).abs();
    if difference.m == 0 {
        return 0.0;
    }
    let leading = (exact.e + 127).max(-1022);
    Real {
        e: difference.e - (leading - 52),
        ..difference
    }
    .approx()
}

/// sin r for |r| ≤ π/4: r × Σ (-1)^j r^(2j) / (2j+1)!, to terms below 2^-160.
fn sin_series(r: Real) -> Real {
    let z = r.times(r);
    let mut term = Real::ONE;
    let mut sum = Real::ONE;
    let mut j = 1u64;
    loop {
        term = term.times(z).over_small((2 * j) * (2 * j + 1));
        if term.m == 0 || term.e + 127 < -160 {
            break;
        }
        sum = if j % 2 == 1 {
            sum.minus(term)
        } else {
            sum.plus(term)
        };
        j += 1;
    }
    r.times(sum)
}

/// cos r for |r| ≤ π/4: Σ (-1)^j r^(2j) / (2j)!.
fn cos_series(r: Real) -> Real {
    let z = r.times(r);
    let mut term = Real::ONE;
    let mut sum = Real::ONE;
    let mut j = 1u64;
    loop {
        term = term.times(z).over_small((2 * j - 1) * (2 * j));
        if term.m == 0 || term.e + 127 < -160 {
            break;
        }
        sum = if j % 2 == 1 {
            sum.minus(term)
        } else {
            sum.plus(term)
        };
        j += 1;
    }
    sum
}

/// atan u for |u| ≤ 1/16 or so: u × Σ (-1)^j u^(2j) / (2j+1).
fn atan_series(u: Real) -> Real {
    let z = u.times(u);
    let mut power = Real::ONE;
    let mut sum = Real::ONE;
    let mut j = 1u64;
    loop {
        power = power.times(z);
        if power.m == 0 || power.e + 127 < -160 {
            break;
        }
        let term = power.over_small(2 * j + 1);
        sum = if j % 2 == 1 {
            sum.minus(term)
        } else {
            sum.plus(term)
        };
        j += 1;
    }
    u.times(sum)
}

/// The reference values: π/2 and π, floor(2^BITS × 2/π) for the reduction, and atan(k/8).
struct Reference {
    pi_over_2: Real,
    pi: Real,
    two_over_pi: Big,
    eighths: Vec<Real>,
}

impl Reference {
    fn new() -> Reference {
        let pi = Pi::new(BITS);
        let (_, two_over_pi) = pi.two_over_pi(BITS);
        let mut eighths = vec![Real::ZERO];
        for k in 1..8 {
            eighths.push(Real::from_big(&atan_of_eighth(k, 256).0, 256));
        }
        eighths.push(Real::from_big(&pi.value, BITS + 2));
        Reference {
            pi_over_2: Real::from_big(&pi.value, BITS + 1),
            pi: Real::from_big(&pi.value, BITS),
            two_over_pi,
            eighths,
        }
    }

    /// sin x and cos x for a finite x.
    fn sin_cos(&self, x: f64) -> (Real, Real) {
        let magnitude = x.abs();
        let (k, r) = if magnitude <= PI_OVER_4_BELOW {
            (0, Real::from_f64(magnitude))
        } else {
            self.reduced(magnitude)
        };
        let (s, c) = (sin_series(r), cos_series(r));
        let (s, c) = match k {
            0 => (s, c),
            1 => (c, s.negated()),
            2 => (s.negated(), c.negated()),
            _ => (c.negated(), s),
        };
        if x < 0.0 { (s.negated(), c) } else { (s, c) }
    }

    /// x - k π/2 for a positive x above π/4, with the quadrant k mod 4, by the exact product of
    /// the mantissa and 1,400 bits of 2/π.
    fn reduced(&self, x: f64) -> (u64, Real) {
        let bits = x.to_bits();
        let mantissa = (bits & ((1 << 52) - 1)) | (1 << 52);
        let q = (bits >> 52) as i64 - 1075;
        let shift = (BITS as i64 - q) as u64;
        let product = Big::from_u64(mantissa).mul(&self.two_over_pi);
        let integer = product.shr(shift);
        let rest = product.sub(&integer.shl(shift));
        let half = Big::from_u64(1).shl(shift - 1);
        let (k, fraction, negative) = if rest.compare(&half) == Ordering::Less {
            (integer.low_u64(), rest, false)
        } else {
            (
                integer.low_u64().wrapping_add(1),
                Big::from_u64(1).shl(shift).sub(&rest),
                true,
            )
        };
        let mut r = Real::from_big(&fraction.shl(256), shift + 256).times(self.pi_over_2);
        if negative {
            r = r.negated();
        }
        (k & 3, r)
    }

    /// atan t for t > 0.
    fn atan_positive(&self, t: Real) -> Real {
        if t.greater_than_one() {
            return self.pi_over_2.minus(self.atan_positive(Real::ONE.over(t)));
        }
        let k = (t.approx() * 8.0 + 0.5).floor() as usize;
        let c = Real::from_f64(k as f64 / 8.0);
        let u = t.minus(c).over(Real::ONE.plus(t.times(c)));
        self.eighths[k].plus(atan_series(u))
    }

    /// atan2 of the magnitudes |y|, |x| (not both zero) with the quadrant of the signs.
    fn atan2(&self, y: f64, x: f64) -> Real {
        let theta = if x == 0.0 {
            self.pi_over_2
        } else if y == 0.0 {
            Real::ZERO
        } else {
            self.atan_positive(Real::from_f64(y.abs()).over(Real::from_f64(x.abs())))
        };
        let theta = if x < 0.0 { self.pi.minus(theta) } else { theta };
        if y < 0.0 { theta.negated() } else { theta }
    }

    /// sqrt((1 - |x|)(1 + |x|)).
    fn cosine_of(&self, x: f64) -> Real {
        let a = Real::from_f64(x.abs());
        Real::ONE.minus(a).times(Real::ONE.plus(a)).sqrt()
    }

    fn evaluate(&self, function: &str, x: f64, y: f64) -> Real {
        match function {
            "sin" => self.sin_cos(x).0,
            "cos" => self.sin_cos(x).1,
            "tan" => {
                let (s, c) = self.sin_cos(x);
                s.over(c)
            }
            "atan" => {
                let theta = self.atan_positive(Real::from_f64(x.abs()));
                if x < 0.0 { theta.negated() } else { theta }
            }
            "asin" => {
                let root = self.cosine_of(x);
                let theta = if root.m == 0 {
                    self.pi_over_2
                } else {
                    self.atan_positive(Real::from_f64(x.abs()).over(root))
                };
                if x < 0.0 { theta.negated() } else { theta }
            }
            "acos" => {
                let root = self.cosine_of(x);
                let theta = if x == 0.0 {
                    self.pi_over_2
                } else if root.m == 0 {
                    Real::ZERO
                } else {
                    self.atan_positive(root.over(Real::from_f64(x.abs())))
                };
                if x < 0.0 { self.pi.minus(theta) } else { theta }
            }
            "atan2" => self.atan2(x, y),
            _ => panic!("no function {function}"),
        }
    }
}

// ---------------------------------------------------------------------------------------------
// The denser walk.

/// Knuth's 64-bit linear congruential generator (design, "The tool that makes the lines", step
/// 5), from a seed other than the reference lines' 1.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0
    }

    /// A normal Number: the sign is bit 63 of a draw, the binary exponent `low + ((draw mod
    /// 2^63) / 2^32 mod (high - low + 1))`, the 52 stored bits the high bits of the next draw.
    fn number(&mut self, low: i32, high: i32) -> f64 {
        let draw = self.next();
        let span = (high - low + 1) as u64;
        let exponent = low + (((draw & (u64::MAX >> 1)) >> 32) % span) as i32;
        let stored = self.next() >> 12;
        f64::from_bits((draw & (1 << 63)) | (((exponent + 1023) as u64) << 52) | stored)
    }
}

/// The largest error in ulp of each function over its arguments, held to its group.
fn walk(reference: &Reference, function: &str, arguments: &[(f64, f64)]) {
    let mut worst = 0.0f64;
    let mut at = (0.0, 0.0);
    for &(x, y) in arguments {
        let error = ulp_error(evaluate(function, x, y), reference.evaluate(function, x, y));
        assert!(
            if group_a(function) {
                error < 1.0
            } else {
                error <= 2.0
            },
            "{function}({x:e}, {y:e}): {error} ulp"
        );
        if error > worst {
            worst = error;
            at = (x, y);
        }
    }
    let bound = if group_a(function) { "< 1" } else { "≤ 2" };
    println!(
        "math_trig walk {function}: {} argument(s), largest error {worst:.6} ulp at ({:e}, {:e}), \
         group bound {bound} ulp",
        arguments.len(),
        at.0,
        at.1
    );
}

/// Half the arguments with an exponent in the dense range, half in the wide one.
fn arguments(lcg: &mut Lcg, count: usize, dense: (i32, i32), wide: (i32, i32)) -> Vec<(f64, f64)> {
    (0..count)
        .map(|i| {
            let (low, high) = if i.is_multiple_of(2) { dense } else { wide };
            (lcg.number(low, high), 0.0)
        })
        .collect()
}

const WALK: usize = 10_000;

#[test]
fn a_denser_walk_holds_sin_cos_and_tan_to_their_groups() {
    let start = Instant::now();
    let reference = Reference::new();
    let mut lcg = Lcg(2);
    for function in ["sin", "cos", "tan"] {
        let arguments = arguments(&mut lcg, WALK, (-3, 8), (-28, 1023));
        walk(&reference, function, &arguments);
    }
    println!("math_trig walk sin cos tan: {:?}", start.elapsed());
}

#[test]
fn a_denser_walk_holds_atan_and_atan2_to_their_groups() {
    let start = Instant::now();
    let reference = Reference::new();
    let mut lcg = Lcg(3);
    let arguments_of_atan = arguments(&mut lcg, WALK, (-4, 4), (-30, 70));
    walk(&reference, "atan", &arguments_of_atan);
    let pairs: Vec<(f64, f64)> = (0..WALK)
        .map(|i| {
            if i.is_multiple_of(2) {
                (lcg.number(-3, 3), lcg.number(-3, 3))
            } else {
                let y = lcg.number(-200, 200);
                let e = ((y.to_bits() >> 52) & 0x7FF) as i32 - 1023;
                (y, lcg.number(e - 80, e + 80))
            }
        })
        .collect();
    walk(&reference, "atan2", &pairs);
    println!("math_trig walk atan atan2: {:?}", start.elapsed());
}

#[test]
fn a_denser_walk_holds_asin_and_acos_to_their_groups() {
    let start = Instant::now();
    let reference = Reference::new();
    let mut lcg = Lcg(4);
    for function in ["asin", "acos"] {
        let arguments = arguments(&mut lcg, WALK, (-5, -1), (-40, -1));
        walk(&reference, function, &arguments);
    }
    println!("math_trig walk asin acos: {:?}", start.elapsed());
}

// ---------------------------------------------------------------------------------------------
// The values the draft fixes.

/// The Number nearest to π × numerator / 4 from both ends of π's interval.
fn pi_number(pi: &Pi, quarters: i64) -> f64 {
    let (low, high) = pi.over_power_of_two(2);
    let factor = Rat::from_i64(quarters);
    let (a, b) = (low.mul(&factor).to_f64(), high.mul(&factor).to_f64());
    assert_eq!(a.to_bits(), b.to_bits());
    a
}

fn same(name: &str, got: f64, want: f64) {
    assert!(
        got.to_bits() == want.to_bits() || (got.is_nan() && want.is_nan()),
        "{name}: {got:e} ({:016X}), want {want:e} ({:016X})",
        got.to_bits(),
        want.to_bits()
    );
}

#[test]
fn the_values_the_draft_fixes_and_every_function_at_nan_infinity_zero_and_a_subnormal() {
    let pi = Pi::new(256);
    let half_pi = pi_number(&pi, 2);
    let quarter_pi = pi_number(&pi, 1);
    let three_quarters_pi = pi_number(&pi, 3);
    let whole_pi = pi_number(&pi, 4);
    assert_eq!(half_pi, std::f64::consts::FRAC_PI_2);
    assert_eq!(whole_pi, std::f64::consts::PI);
    assert_eq!(half_pi.to_bits(), 0x3FF9_21FB_5444_2D18);
    assert_eq!(format!("{half_pi}"), "1.5707963267948966");

    let nan = f64::NAN;
    let inf = f64::INFINITY;
    let tiny = f64::from_bits(1);
    // One table: every function at NaN, ±∞, ±0 and the smallest subnormal with both signs.
    type Unary = fn(f64) -> f64;
    let table: [(&str, Unary, [f64; 7]); 6] = [
        ("sin", sin, [nan, nan, nan, 0.0, -0.0, tiny, -tiny]),
        ("cos", cos, [nan, nan, nan, 1.0, 1.0, 1.0, 1.0]),
        ("tan", tan, [nan, nan, nan, 0.0, -0.0, tiny, -tiny]),
        ("asin", asin, [nan, nan, nan, 0.0, -0.0, tiny, -tiny]),
        (
            "acos",
            acos,
            [nan, nan, nan, half_pi, half_pi, half_pi, half_pi],
        ),
        (
            "atan",
            atan,
            [nan, half_pi, -half_pi, 0.0, -0.0, tiny, -tiny],
        ),
    ];
    let inputs = [nan, inf, -inf, 0.0, -0.0, tiny, -tiny];
    let mut checked = 0;
    for (name, function, wants) in table {
        for (x, want) in inputs.into_iter().zip(wants) {
            same(&format!("{name}({x:e})"), function(x), want);
            checked += 1;
        }
    }
    // atan2 with x = 1 over the same inputs for y.
    for (y, want) in inputs
        .into_iter()
        .zip([nan, half_pi, -half_pi, 0.0, -0.0, tiny, -tiny])
    {
        same(&format!("atan2({y:e}, 1)"), atan2(y, 1.0), want);
        checked += 1;
    }

    // The other values the draft fixes.
    same("asin(1)", asin(1.0), half_pi);
    same("asin(-1)", asin(-1.0), -half_pi);
    for x in [2.0, -2.0, 1.0f64.next_up(), (-1.0f64).next_down()] {
        same(&format!("asin({x:e})"), asin(x), nan);
        same(&format!("acos({x:e})"), acos(x), nan);
        checked += 2;
    }
    same("acos(1)", acos(1.0), 0.0);
    same("acos(-1)", acos(-1.0), whole_pi);
    checked += 4;
    println!("math_trig fixed values of sin cos tan asin acos atan: {checked} checked");

    // atan2, entry by entry of the draft's table (`#sec-math.atan2`).
    let finite_x = [1.0, -1.0, 0.0, -0.0];
    let mut entries = 0;
    let mut entry = |name: String, got: f64, want: f64| {
        same(&name, got, want);
        entries += 1;
    };
    // 3. NaN.
    for (y, x) in [(nan, 1.0), (1.0, nan), (nan, nan), (inf, nan), (nan, -0.0)] {
        entry(format!("atan2({y}, {x})"), atan2(y, x), nan);
    }
    // 4. y = +∞.
    entry("atan2(+∞, +∞)".into(), atan2(inf, inf), quarter_pi);
    entry("atan2(+∞, -∞)".into(), atan2(inf, -inf), three_quarters_pi);
    for x in finite_x {
        entry(format!("atan2(+∞, {x:e})"), atan2(inf, x), half_pi);
    }
    // 5. y = -∞.
    entry("atan2(-∞, +∞)".into(), atan2(-inf, inf), -quarter_pi);
    entry(
        "atan2(-∞, -∞)".into(),
        atan2(-inf, -inf),
        -three_quarters_pi,
    );
    for x in finite_x {
        entry(format!("atan2(-∞, {x:e})"), atan2(-inf, x), -half_pi);
    }
    // 6. y = +0, 7. y = -0.
    for x in [1.0, inf, 0.0, tiny] {
        entry(format!("atan2(+0, {x:e})"), atan2(0.0, x), 0.0);
        entry(format!("atan2(-0, {x:e})"), atan2(-0.0, x), -0.0);
    }
    for x in [-1.0, -inf, -0.0, -tiny] {
        entry(format!("atan2(+0, {x:e})"), atan2(0.0, x), whole_pi);
        entry(format!("atan2(-0, {x:e})"), atan2(-0.0, x), -whole_pi);
    }
    // 8, 9. y finite and positive, or negative.
    for y in [1.0, tiny, f64::MAX] {
        entry(format!("atan2({y:e}, +∞)"), atan2(y, inf), 0.0);
        entry(format!("atan2({y:e}, -∞)"), atan2(y, -inf), whole_pi);
        entry(format!("atan2({y:e}, +0)"), atan2(y, 0.0), half_pi);
        entry(format!("atan2({y:e}, -0)"), atan2(y, -0.0), half_pi);
        entry(format!("atan2({:e}, +∞)", -y), atan2(-y, inf), -0.0);
        entry(format!("atan2({:e}, -∞)", -y), atan2(-y, -inf), -whole_pi);
        entry(format!("atan2({:e}, +0)", -y), atan2(-y, 0.0), -half_pi);
        entry(format!("atan2({:e}, -0)", -y), atan2(-y, -0.0), -half_pi);
    }
    println!("math_trig atan2: {entries} entries of the draft's table checked");
}
