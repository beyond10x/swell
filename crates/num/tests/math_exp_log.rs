//! The exponential and logarithmic functions of `swell_num::math` and `Number::exponentiate`
//! (design, decision 6, "The functions of `Math` that IEEE 754 does not fix", items 2, 3, 5, 6
//! and 7; `#sec-math.exp` and its siblings, `#sec-numeric-types-number-exponentiate`).
//!
//! Every check is against exact arithmetic ([`exact`]), against the committed reference lines
//! of `data/math/reference-exp-log.txt` (made once by GNU bc 1.08.2, run as `bc -l`, by the
//! procedure of the design's "The tool that makes the lines"; its header says how), or against
//! a value the draft fixes:
//!
//! - every constant (ln 2 in three parts and as a pair, 1/ln 2, 1/ln 10, the coefficients of
//!   both series, the bound √2 of the reduction) is recomputed from its definition with exact
//!   integer arithmetic and compared bit for bit; ln 2 and ln 10 come from the series of
//!   2·atanh of a rational, to 1,400 bits;
//! - the degrees of the series follow the design's cut (the first omitted term below 2^-80 of
//!   the result on the whole reduced interval, the last kept one not);
//! - group A (`exp log log2 log10 pow cbrt`) is held to "one of the two Numbers that enclose
//!   the exact result" and group B (`expm1 sinh cosh tanh log1p asinh acosh atanh`) to at most 2
//!   Numbers from the nearer of the two, over every reference line;
//! - the exact cases and the special values are written here by hand.

#![cfg(test)]

// The exact arithmetic is included as `math_dd.rs` includes it; there is no `tests/support/mod.rs`
// to share it through, so the file is a module of each of the two.
#[allow(clippy::duplicate_mod)]
#[path = "support/exact.rs"]
mod exact;

use std::cmp::Ordering;

use exact::{Big, Rat, relative_error};
use swell_num::math::dd::Dd;
use swell_num::math::exp::{EXP_COEFFICIENTS, INV_LN2, LN2, LN2_PARTS};
use swell_num::math::log::{INV_LN10, LOG_COEFFICIENTS, SQRT2_UPPER};
use swell_num::math::{
    acosh, asinh, atanh, cbrt, cosh, exp, exp_dd, expm1, exponentiate, log, log_dd, log1p, log2,
    log10, sinh, tanh,
};

/// The reference lines and their count by function (the header of the file gives them).
const REFERENCE: &str = include_str!("../../../data/math/reference-exp-log.txt");
const REFERENCE_COUNTS: [(&str, usize); 14] = [
    ("exp", 223),
    ("expm1", 222),
    ("sinh", 217),
    ("cosh", 217),
    ("tanh", 215),
    ("log", 218),
    ("log2", 213),
    ("log10", 211),
    ("log1p", 216),
    ("asinh", 214),
    ("acosh", 210),
    ("atanh", 211),
    ("cbrt", 216),
    ("pow", 228),
];

/// The functions of group A (design, "The accuracy the engine promises"): less than 1 ulp.
const GROUP_A: [&str; 6] = ["exp", "log", "log2", "log10", "pow", "cbrt"];

/// The bits of the fixed-point values of ln 2 and ln 10: the design's 1,400.
const PRECISION: u64 = 1400;

fn bits(x: f64) -> u64 {
    x.to_bits()
}

fn pow2(k: i32) -> f64 {
    Rat::pow2(i64::from(k)).to_f64()
}

/// One function of a reference line, by its name.
fn evaluate(name: &str, x: f64, y: f64) -> f64 {
    match name {
        "exp" => exp(x),
        "expm1" => expm1(x),
        "sinh" => sinh(x),
        "cosh" => cosh(x),
        "tanh" => tanh(x),
        "log" => log(x),
        "log2" => log2(x),
        "log10" => log10(x),
        "log1p" => log1p(x),
        "asinh" => asinh(x),
        "acosh" => acosh(x),
        "atanh" => atanh(x),
        "cbrt" => cbrt(x),
        "pow" => exponentiate(x, y),
        _ => panic!("no function {name}"),
    }
}

/// A reference line: the function, the arguments and the bits of the two enclosing Numbers.
struct Line {
    name: String,
    x: f64,
    y: f64,
    lower: u64,
    upper: u64,
}

fn reference_lines() -> Vec<Line> {
    let mut lines = Vec::new();
    for text in REFERENCE.lines() {
        if text.starts_with('#') || text.trim().is_empty() {
            continue;
        }
        let fields: Vec<&str> = text.split(' ').collect();
        let hex = |field: &str| {
            assert_eq!(field.len(), 16, "reference line {text:?}");
            u64::from_str_radix(field, 16).unwrap()
        };
        let (x, y, rest) = match fields.as_slice() {
            ["pow", x, y, rest @ ..] => (hex(x), hex(y), rest),
            [_, x, rest @ ..] => (hex(x), 0, rest),
            _ => panic!("reference line {text:?}"),
        };
        assert_eq!(rest.len(), 2, "reference line {text:?}");
        lines.push(Line {
            name: fields[0].to_string(),
            x: f64::from_bits(x),
            y: f64::from_bits(y),
            lower: hex(rest[0]),
            upper: hex(rest[1]),
        });
    }
    lines
}

/// The position of a Number on the line of all Numbers: adjacent Numbers differ by 1, and +0
/// and -0 both sit at 0.
fn ordinal(bits: u64) -> i64 {
    let magnitude = (bits & 0x7FFF_FFFF_FFFF_FFFF) as i64;
    if bits >> 63 == 1 {
        -magnitude
    } else {
        magnitude
    }
}

/// How many Numbers the result lies from the nearer of the two that enclose the exact value:
/// 0 when it is one of them.
fn steps_from_enclosure(result: f64, lower: u64, upper: u64) -> i64 {
    let r = ordinal(bits(result));
    let (a, b) = (ordinal(lower), ordinal(upper));
    let (low, high) = if a <= b { (a, b) } else { (b, a) };
    if r < low {
        low - r
    } else if r > high {
        r - high
    } else if r == low || r == high {
        0
    } else {
        // Strictly between two adjacent Numbers cannot happen; count it as far.
        i64::MAX
    }
}

/// The enclosure of the reference line of a function at one argument.
fn enclosure_of(name: &str, x: f64) -> (u64, u64) {
    let line = reference_lines()
        .into_iter()
        .find(|line| line.name == name && bits(line.x) == bits(x))
        .unwrap_or_else(|| panic!("no reference line {name} {:016X}", bits(x)));
    (line.lower, line.upper)
}

#[test]
fn reference_lines_hold_group_a_and_group_b() {
    let lines = reference_lines();
    let mut total = 0;
    for (name, count) in REFERENCE_COUNTS {
        let group_a = GROUP_A.contains(&name);
        let mut seen = 0;
        let mut exact = 0;
        let mut worst = 0;
        let mut worst_line = String::new();
        for line in lines.iter().filter(|line| line.name == name) {
            seen += 1;
            if line.lower == line.upper {
                exact += 1;
            }
            let result = evaluate(name, line.x, line.y);
            let steps = steps_from_enclosure(result, line.lower, line.upper);
            // Group A: the result is one of the two Numbers, bit for bit (so the sign of a zero
            // counts); group B: at most 2 Numbers from the nearer.
            let within = if group_a {
                bits(result) == line.lower || bits(result) == line.upper
            } else {
                steps <= 2
            };
            assert!(
                within,
                "{name}({:016X}{}) = {:016X}, {steps} Number(s) from the enclosure {:016X} {:016X}",
                bits(line.x),
                if name == "pow" {
                    format!(", {:016X}", bits(line.y))
                } else {
                    String::new()
                },
                bits(result),
                line.lower,
                line.upper
            );
            if line.lower == line.upper {
                assert_eq!(bits(result), line.lower, "{name} exact line");
            }
            if steps > worst {
                worst = steps;
                worst_line = format!(
                    " (at {:016X} {:016X}: {:016X})",
                    bits(line.x),
                    bits(line.y),
                    bits(result)
                );
            }
        }
        assert_eq!(seen, count, "{name}: reference lines");
        total += seen;
        println!(
            "math_exp_log reference {name}: {seen} line(s), {exact} exact, group {}, at most {worst} Number(s) from the enclosure{worst_line}",
            if group_a { "A" } else { "B" }
        );
    }
    assert_eq!(total, lines.len(), "reference lines of no known function");
    println!("math_exp_log reference: {total} line(s)");
}

/// floor(2^PRECISION × 2·atanh(1/n)) up to the floors of the terms: Σ 2 / ((2j + 1) n^(2j+1)),
/// each term cut to an integer, so the sum is below the value by less than its term count.
/// Returns the sum and that count.
fn two_atanh_inverse(n: u64) -> (Big, u64) {
    let mut power = Big::from_u64(1).shl(PRECISION + 1).div_u64(n);
    let mut sum = Big::zero();
    let mut terms = 0;
    while !power.is_zero() {
        sum = sum.add(&power.div_u64(2 * terms + 1));
        power = power.div_u64(n * n);
        terms += 1;
    }
    // Each term and each power were cut once: the sum is low by at most 2 per term.
    (sum, 2 * terms)
}

/// ln 2 and ln 10 as integers times 2^-PRECISION: each value lies in [low, low + slack].
struct Logs {
    ln2: Big,
    ln10: Big,
    slack: Big,
}

fn logs() -> Logs {
    // ln 2 = 2·atanh(1/3); ln 10 = 3·ln 2 + ln(5/4) = 3·ln 2 + 2·atanh(1/9).
    let (ln2, slack2) = two_atanh_inverse(3);
    let (ln5_4, slack5_4) = two_atanh_inverse(9);
    let ln10 = ln2.add(&ln2).add(&ln2).add(&ln5_4);
    Logs {
        ln2,
        ln10,
        slack: Big::from_u64(3 * slack2 + slack5_4),
    }
}

fn fixed(value: &Big) -> Rat {
    Rat::new(false, value.clone(), Big::from_u64(1).shl(PRECISION))
}

/// The pair of a value: the Number nearest to it, and the Number nearest to the rest.
fn pair_of(value: &Rat) -> (u64, u64) {
    let hi = value.to_f64();
    let lo = value.sub(&Rat::from_f64(hi)).to_f64();
    (bits(hi), bits(lo))
}

/// The pair of a value known to lie between two rationals: both ends give the same pair, or
/// the test cannot decide it and fails.
fn pair_between(low: &Rat, high: &Rat, name: &str) -> (u64, u64) {
    let a = pair_of(low);
    let b = pair_of(high);
    assert_eq!(a, b, "{name}: the pair is not decided at {PRECISION} bits");
    a
}

fn pair_bits(x: Dd) -> (u64, u64) {
    (bits(x.hi), bits(x.lo))
}

#[test]
fn every_constant_is_recomputed_bit_for_bit() {
    let logs = logs();
    let one = Rat::from_i64(1);
    let ln2_high = logs.ln2.add(&logs.slack);
    let ln10_high = logs.ln10.add(&logs.slack);

    // ln 2 in three parts: L1 = floor(ln 2 × 2^32) / 2^32, L2 = floor((ln 2 - L1) × 2^64) / 2^64,
    // L3 = the Number nearest to ln 2 - L1 - L2.
    let parts = |value: &Big| {
        let l1 = value.shr(PRECISION - 32);
        let rest = value.sub(&l1.shl(PRECISION - 32));
        let l2 = rest.shr(PRECISION - 64);
        let rest = rest.sub(&l2.shl(PRECISION - 64));
        (
            bits(Rat::new(false, l1, Big::from_u64(1).shl(32)).to_f64()),
            bits(Rat::new(false, l2, Big::from_u64(1).shl(64)).to_f64()),
            bits(fixed(&rest).to_f64()),
        )
    };
    let low_parts = parts(&logs.ln2);
    assert_eq!(low_parts, parts(&ln2_high), "ln 2 parts not decided");
    assert_eq!(
        low_parts,
        (bits(LN2_PARTS[0]), bits(LN2_PARTS[1]), bits(LN2_PARTS[2])),
        "LN2_PARTS"
    );
    // L1 and L2 carry at most 32 significant bits each, so k × L1 and k × L2 are exact for
    // |k| below 2^11 (the 21 low stored bits are zero).
    assert_eq!(
        bits(LN2_PARTS[0]) & 0x1F_FFFF,
        0,
        "L1 has more than 32 bits"
    );
    assert_eq!(
        bits(LN2_PARTS[1]) & 0x1F_FFFF,
        0,
        "L2 has more than 32 bits"
    );

    let ln2 = pair_between(&fixed(&logs.ln2), &fixed(&ln2_high), "ln 2");
    assert_eq!(ln2, pair_bits(LN2), "LN2");
    let inv_ln2 = pair_between(
        &one.div(&fixed(&ln2_high)),
        &one.div(&fixed(&logs.ln2)),
        "1/ln 2",
    );
    assert_eq!(inv_ln2, pair_bits(INV_LN2), "INV_LN2");
    let inv_ln10 = pair_between(
        &one.div(&fixed(&ln10_high)),
        &one.div(&fixed(&logs.ln10)),
        "1/ln 10",
    );
    assert_eq!(inv_ln10, pair_bits(INV_LN10), "INV_LN10");

    // 1/n! for n from 2 to 18 (the series of exp).
    let mut factorial = 1u64;
    for (i, coefficient) in EXP_COEFFICIENTS.iter().enumerate() {
        let n = i as u64 + 2;
        factorial *= if i == 0 { 2 } else { n };
        let value = Rat::new(false, Big::from_u64(1), Big::from_u64(factorial));
        assert_eq!(pair_of(&value), pair_bits(*coefficient), "1/{n}!");
    }
    assert_eq!(EXP_COEFFICIENTS.len(), 17, "exp through r^18");

    // 1/(2j + 1) for j from 1 to 14 (the series of log: 3 to 29).
    for (j, coefficient) in LOG_COEFFICIENTS.iter().enumerate() {
        let d = 2 * j as u64 + 3;
        let value = Rat::new(false, Big::from_u64(1), Big::from_u64(d));
        assert_eq!(pair_of(&value), pair_bits(*coefficient), "1/{d}");
    }
    assert_eq!(LOG_COEFFICIENTS.len(), 14, "log through s^29");

    // SQRT2_UPPER is the least Number above √2: its square is above 2, its predecessor's below.
    let two = Rat::from_i64(2);
    let upper = Rat::from_f64(SQRT2_UPPER);
    let below = Rat::from_f64(SQRT2_UPPER.next_down());
    assert_eq!(upper.mul(&upper).compare(&two), Ordering::Greater);
    assert_eq!(below.mul(&below).compare(&two), Ordering::Less);
    println!(
        "math_exp_log constants: ln 2 and ln 10 to {PRECISION} bits, 3 parts of ln 2, 3 pairs, 17 + 14 coefficients, 1 bound"
    );
}

fn rational(numerator: i64, denominator: i64) -> Rat {
    Rat::from_i64(numerator).div(&Rat::from_i64(denominator))
}

fn power(base: &Rat, n: u32) -> Rat {
    let mut result = Rat::from_i64(1);
    for _ in 0..n {
        result = result.mul(base);
    }
    result
}

fn factorial(n: u32) -> Rat {
    let mut result = Rat::from_i64(1);
    for k in 2..=n {
        result = result.mul(&Rat::from_i64(i64::from(k)));
    }
    result
}

#[test]
fn series_degrees_follow_the_cut_at_2_to_the_minus_80() {
    let cut = Rat::pow2(-80);
    let below = |value: &Rat| value.compare(&cut) == Ordering::Less;

    // The reduced interval of exp: |r| ≤ ln 2 / 2 = 0.3465735..., which 0.34657 and 0.34658
    // enclose; the code's bound is half the high part of LN2.
    let r_low = rational(34_657, 100_000);
    let r_high = rational(34_658, 100_000);
    let half_ln2 = Rat::from_f64(LN2.hi * 0.5);
    assert_eq!(half_ln2.compare(&r_low), Ordering::Greater);
    assert_eq!(half_ln2.compare(&r_high), Ordering::Less);
    // exp: the result is at least e^(-ln 2 / 2) = 1/√2, between 0.7071 and 0.70711.
    let exp_low = rational(7_071, 10_000);
    let exp_high = rational(70_711, 100_000);
    let term = |r: &Rat, n: u32| power(r, n).div(&factorial(n));
    assert!(below(&term(&r_high, 19).div(&exp_low)), "exp: r^19 is kept");
    assert!(
        !below(&term(&r_low, 18).div(&exp_high)),
        "exp: r^18 can be cut"
    );
    // expm1 near 0 (the same series without its first term): |expm1(r) / r| is at least
    // (1 - 1/√2) / (ln 2 / 2) = 0.84511..., at r = -ln 2 / 2.
    let ratio_low = rational(845, 1_000);
    let ratio_high = rational(8_452, 10_000);
    assert!(
        below(&term(&r_high, 19).div(&r_low).div(&ratio_low)),
        "expm1: r^19 is kept"
    );
    assert!(
        !below(&term(&r_low, 18).div(&r_high).div(&ratio_high)),
        "expm1: r^18 can be cut"
    );

    // log: m in [SQRT2_UPPER / 2, SQRT2_UPPER), s = (m - 1)/(m + 1), |s| below 3 - 2√2 =
    // 0.1715728..., which 0.171572 and 0.171573 enclose; log m = 2·atanh(s) is at least 2|s|
    // and at most 2|s| × 1.0102.
    let s_low = rational(171_572, 1_000_000);
    let s_high = rational(171_573, 1_000_000);
    let one = Rat::from_i64(1);
    let m_high = Rat::from_f64(SQRT2_UPPER.next_down());
    let m_low = Rat::from_f64(SQRT2_UPPER * 0.5);
    let s_top = m_high.sub(&one).div(&m_high.add(&one));
    let s_bottom = one.sub(&m_low).div(&one.add(&m_low));
    assert_eq!(s_top.compare(&s_high), Ordering::Less);
    assert_eq!(s_bottom.compare(&s_high), Ordering::Less);
    assert_eq!(s_top.compare(&s_low), Ordering::Greater);
    // The term 2 s^n / n over the result 2·atanh|s|.
    let log_term = |s: &Rat, n: u32| power(s, n - 1).div(&Rat::from_i64(i64::from(n)));
    assert!(below(&log_term(&s_high, 31)), "log: s^31 is kept");
    assert!(
        !below(&log_term(&s_low, 29).div(&rational(10_102, 10_000))),
        "log: s^29 can be cut"
    );
    println!("math_exp_log degrees: exp and expm1 through r^18, log through s^29 (the design's)");
}

/// exp_dd and log_dd hold the design's 2^-79 (item 6: log2|x| "with relative error at most
/// 2^-79"; each series is cut at 2^-80 of its result, so the pairs are not 2^-100 but about
/// 2^-80): exp_dd(log_dd(x)) returns x within 2^-79 × (|e| + 2), e the binary exponent of x
/// (the absolute error of log_dd(x) is at most 2^-79 × |ln x|, below 2^-79 × (|e| + 1), and
/// exp_dd adds its own 2^-79); log_dd(2), which takes no series, is ln 2 within 2^-100.
#[test]
fn exp_dd_and_log_dd_hold_2_to_the_minus_79() {
    let ln2 = Rat::from_dd(LN2.hi, LN2.lo);
    let log_two = log_dd(2.0);
    let error = relative_error(&Rat::from_dd(log_two.hi, log_two.lo), &ln2);
    assert_eq!(error.compare(&Rat::pow2(-100)), Ordering::Less, "log_dd(2)");
    let mut worst = Rat::zero();
    let mut count = 0;
    for x in [
        0.5,
        0.75,
        1.0 + f64::EPSILON,
        2.0,
        3.0,
        10.0,
        1e-250,
        1e300,
        12345.678,
        pow2(-900),
        f64::MAX / 4.0,
    ] {
        let back = exp_dd(log_dd(x));
        let error = relative_error(&Rat::from_dd(back.hi, back.lo), &Rat::from_f64(x));
        let e = ((x.to_bits() >> 52) as i64 - 1023).abs();
        let bound = Rat::pow2(-79).mul(&Rat::from_i64(e + 2));
        println!(
            "math_exp_log pair exp_dd(log_dd({x:e})): relative error 2^{:.2}, bound 2^{:.2}",
            error.log2(),
            bound.log2()
        );
        assert_eq!(
            error.compare(&bound),
            Ordering::Less,
            "exp_dd(log_dd({x:e}))"
        );
        if error.compare(&worst) == Ordering::Greater {
            worst = error;
        }
        count += 1;
    }
    let zero = exp_dd(Dd::from_f64(0.0));
    assert_eq!((zero.hi, zero.lo), (1.0, 0.0), "exp_dd(0)");
    println!(
        "math_exp_log pairs: {count} round trips, largest relative error 2^{:.2}",
        worst.log2()
    );
}

#[test]
fn exact_cases_of_the_story() {
    let mut count = 0;
    let mut same = |actual: f64, expected: f64, what: &str| {
        assert_eq!(
            bits(actual),
            bits(expected),
            "{what}: {actual:e} is not {expected:e}"
        );
        count += 1;
    };
    same(exp(0.0), 1.0, "exp(+0)");
    same(exp(-0.0), 1.0, "exp(-0)");
    same(cosh(0.0), 1.0, "cosh(+0)");
    same(cosh(-0.0), 1.0, "cosh(-0)");
    same(log10(10.0), 1.0, "log10(10)");
    same(log10(100.0), 2.0, "log10(100)");
    same(log10(1000.0), 3.0, "log10(1000)");
    same(log2(2.0), 1.0, "log2(2)");
    same(log2(4.0), 2.0, "log2(4)");
    same(log2(8.0), 3.0, "log2(8)");
    same(exponentiate(2.0, 10.0), 1024.0, "pow(2, 10)");
    same(exponentiate(10.0, 2.0), 100.0, "pow(10, 2)");
    same(exponentiate(2.0, -2_147_483_648.0), 0.0, "pow(2, -2^31)");
    same(exponentiate(1.0, -2_147_483_648.0), 1.0, "pow(1, -2^31)");
    same(exponentiate(2.0, -1074.0), pow2(-1074), "pow(2, -1074)");
    same(exponentiate(-2.0, 3.0), -8.0, "pow(-2, 3)");
    same(exponentiate(-8.0, 1.0 / 3.0), f64::NAN, "pow(-8, 1/3)");
    same(tanh(f64::INFINITY), 1.0, "tanh(+inf)");
    same(tanh(f64::NEG_INFINITY), -1.0, "tanh(-inf)");
    same(exp(710.0), f64::INFINITY, "exp(710)");
    same(exp(-746.0), 0.0, "exp(-746)");
    same(expm1(-40.0), -1.0, "expm1(-40)");
    same(log(-1.0), f64::NAN, "log(-1)");
    same(log(0.0), f64::NEG_INFINITY, "log(0)");
    same(log1p(-1.0), f64::NEG_INFINITY, "log1p(-1)");
    same(atanh(1.0), f64::INFINITY, "atanh(1)");
    same(atanh(-1.0), f64::NEG_INFINITY, "atanh(-1)");
    same(acosh(0.5), f64::NAN, "acosh(0.5)");
    same(exponentiate(1.0, f64::INFINITY), f64::NAN, "pow(1, +inf)");
    same(
        exponentiate(1.0, f64::NEG_INFINITY),
        f64::NAN,
        "pow(1, -inf)",
    );
    println!("math_exp_log exact cases: {count}");
}

#[test]
fn log2_of_every_power_of_two_is_exact() {
    let mut count = 0;
    for k in -1074..=1023 {
        let x = pow2(k);
        assert_eq!(log2(x), f64::from(k), "log2(2^{k})");
        count += 1;
    }
    assert_eq!(count, 2098);
    println!("math_exp_log log2(2^k): {count} values of k, -1074 to 1023");
}

#[test]
fn log10_of_every_power_of_ten_that_is_a_number_is_exact() {
    let mut x = 1.0;
    for n in 0..=22 {
        // 10^n is a Number up to 10^22 (5^22 < 2^53), and each product is exact.
        assert_eq!(log10(x), f64::from(n), "log10(10^{n})");
        x *= 10.0;
    }
    println!("math_exp_log log10(10^n): 23 values of n, 0 to 22");
}

#[test]
fn cbrt_of_every_perfect_cube_is_exact() {
    let mut count = 0;
    for n in -10..=10i32 {
        let cube = f64::from(n * n * n);
        let root = cbrt(cube);
        assert_eq!(bits(root), bits(f64::from(n)), "cbrt({cube})");
        count += 1;
    }
    // The cubes of powers of two over the whole range, 2^-1074 = (2^-358)^3 to 2^1023.
    for j in -358..=341 {
        let cube = pow2(3 * j);
        assert_eq!(cbrt(cube), pow2(j), "cbrt(2^{})", 3 * j);
        assert_eq!(cbrt(-cube), -pow2(j), "cbrt(-2^{})", 3 * j);
        count += 2;
    }
    // Cubes of other integers and of dyadic fractions.
    for (root, cube) in [
        (1.5, 3.375),
        (12345.0, 1_881_365_963_625.0),
        (0.75, 0.421875),
    ] {
        assert_eq!(cbrt(cube), root, "cbrt({cube})");
        count += 1;
    }
    println!("math_exp_log cbrt of perfect cubes: {count}");
}

/// The draft's table of `Number::exponentiate` (`#sec-numeric-types-number-exponentiate`), entry
/// by entry: each step that returns before the last line, with its cases.
#[test]
fn exponentiate_follows_the_table_of_the_draft() {
    let nan = f64::NAN;
    let inf = f64::INFINITY;
    let odd = [1.0, 3.0, -1.0, -3.0, 9_007_199_254_740_991.0];
    let even = [2.0, -2.0, 0.5, -0.5, 1e300, -1e300, 9_007_199_254_740_992.0];
    let mut entries = 0;
    let mut cases = 0;
    let mut check = |base: f64, exponent: f64, expected: f64| {
        let actual = exponentiate(base, exponent);
        assert_eq!(
            bits(actual),
            bits(expected),
            "pow({base:e}, {exponent:e}) = {actual:e}, the draft says {expected:e}"
        );
        cases += 1;
    };
    // 1. The exponent is NaN.
    for base in [nan, 0.0, 1.0, -1.0, inf] {
        check(base, nan, nan);
    }
    entries += 1;
    // 2. The exponent is ±0.
    for base in [nan, 0.0, -0.0, 1.0, -2.5, inf, -inf] {
        check(base, 0.0, 1.0);
        check(base, -0.0, 1.0);
    }
    entries += 1;
    // 3. The base is NaN.
    for exponent in [1.0, -1.0, inf, 0.5] {
        check(nan, exponent, nan);
    }
    entries += 1;
    // 4 and 5. The base is +∞.
    for exponent in [0.5, 1.0, 3.0, inf] {
        check(inf, exponent, inf);
    }
    for exponent in [-0.5, -1.0, -3.0, -inf] {
        check(inf, exponent, 0.0);
    }
    entries += 2;
    // 6 to 9. The base is -∞.
    for exponent in odd.iter().filter(|e| **e > 0.0) {
        check(-inf, *exponent, -inf);
    }
    for exponent in even.iter().filter(|e| **e > 0.0).chain(&[inf]) {
        check(-inf, *exponent, inf);
    }
    for exponent in odd.iter().filter(|e| **e < 0.0) {
        check(-inf, *exponent, -0.0);
    }
    for exponent in even.iter().filter(|e| **e < 0.0).chain(&[-inf]) {
        check(-inf, *exponent, 0.0);
    }
    entries += 4;
    // 10 and 11. The base is +0.
    for exponent in [0.5, 1.0, 3.0, inf] {
        check(0.0, exponent, 0.0);
    }
    for exponent in [-0.5, -1.0, -3.0, -inf] {
        check(0.0, exponent, inf);
    }
    entries += 2;
    // 12 to 15. The base is -0.
    for exponent in odd.iter().filter(|e| **e > 0.0) {
        check(-0.0, *exponent, -0.0);
    }
    for exponent in even.iter().filter(|e| **e > 0.0).chain(&[inf]) {
        check(-0.0, *exponent, 0.0);
    }
    for exponent in odd.iter().filter(|e| **e < 0.0) {
        check(-0.0, *exponent, -inf);
    }
    for exponent in even.iter().filter(|e| **e < 0.0).chain(&[-inf]) {
        check(-0.0, *exponent, inf);
    }
    entries += 4;
    // 16 to 18. The exponent is +∞.
    for base in [1.5, -1.5, f64::MAX, -f64::MAX] {
        check(base, inf, inf);
    }
    check(1.0, inf, nan);
    check(-1.0, inf, nan);
    for base in [
        0.5,
        -0.5,
        pow2(-1074),
        -pow2(-1074),
        1.0 - f64::EPSILON / 2.0,
    ] {
        check(base, inf, 0.0);
    }
    entries += 3;
    // 19 to 21. The exponent is -∞.
    for base in [1.5, -1.5, f64::MAX, -f64::MAX] {
        check(base, -inf, 0.0);
    }
    check(1.0, -inf, nan);
    check(-1.0, -inf, nan);
    for base in [
        0.5,
        -0.5,
        pow2(-1074),
        -pow2(-1074),
        1.0 - f64::EPSILON / 2.0,
    ] {
        check(base, -inf, inf);
    }
    entries += 3;
    // 22. A negative base with an exponent that is not an integral Number.
    for (base, exponent) in [
        (-8.0, 1.0 / 3.0),
        (-1.0, 0.5),
        (-2.0, -0.5),
        (-f64::MAX, 1.5),
        (-pow2(-1074), 2.5),
        (-2.0, 4_503_599_627_370_495.5),
    ] {
        check(base, exponent, nan);
    }
    entries += 1;
    assert_eq!(entries, 22);
    println!("math_exp_log exponentiate table: {entries} entries of the draft, {cases} cases");
}

/// A row of the table of special values: the name, the function, and its results for NaN, +∞,
/// -∞, +0, -0, the smallest subnormal and its negative (None: the reference line decides).
type SpecialRow = (&'static str, fn(f64) -> f64, [Option<f64>; 7]);

/// NaN, ±∞, ±0 and the smallest subnormal into every function, with what the draft fixes (and,
/// for the subnormal, the exact case or the reference line).
#[test]
fn special_values_of_every_function() {
    let nan = f64::NAN;
    let inf = f64::INFINITY;
    let tiny = pow2(-1074);
    // name, f(NaN), f(+∞), f(-∞), f(+0), f(-0), f(tiny), f(-tiny); None: the reference line.
    let table: [SpecialRow; 13] = [
        (
            "exp",
            exp,
            [
                Some(nan),
                Some(inf),
                Some(0.0),
                Some(1.0),
                Some(1.0),
                Some(1.0),
                None,
            ],
        ),
        (
            "expm1",
            expm1,
            [
                Some(nan),
                Some(inf),
                Some(-1.0),
                Some(0.0),
                Some(-0.0),
                Some(tiny),
                Some(-tiny),
            ],
        ),
        (
            "sinh",
            sinh,
            [
                Some(nan),
                Some(inf),
                Some(-inf),
                Some(0.0),
                Some(-0.0),
                Some(tiny),
                Some(-tiny),
            ],
        ),
        (
            "cosh",
            cosh,
            [
                Some(nan),
                Some(inf),
                Some(inf),
                Some(1.0),
                Some(1.0),
                Some(1.0),
                Some(1.0),
            ],
        ),
        (
            "tanh",
            tanh,
            [
                Some(nan),
                Some(1.0),
                Some(-1.0),
                Some(0.0),
                Some(-0.0),
                Some(tiny),
                Some(-tiny),
            ],
        ),
        (
            "log",
            log,
            [
                Some(nan),
                Some(inf),
                Some(nan),
                Some(-inf),
                Some(-inf),
                None,
                Some(nan),
            ],
        ),
        (
            "log2",
            log2,
            [
                Some(nan),
                Some(inf),
                Some(nan),
                Some(-inf),
                Some(-inf),
                Some(-1074.0),
                Some(nan),
            ],
        ),
        (
            "log10",
            log10,
            [
                Some(nan),
                Some(inf),
                Some(nan),
                Some(-inf),
                Some(-inf),
                None,
                Some(nan),
            ],
        ),
        (
            "log1p",
            log1p,
            [
                Some(nan),
                Some(inf),
                Some(nan),
                Some(0.0),
                Some(-0.0),
                Some(tiny),
                Some(-tiny),
            ],
        ),
        (
            "asinh",
            asinh,
            [
                Some(nan),
                Some(inf),
                Some(-inf),
                Some(0.0),
                Some(-0.0),
                Some(tiny),
                Some(-tiny),
            ],
        ),
        (
            "acosh",
            acosh,
            [
                Some(nan),
                Some(inf),
                Some(nan),
                Some(nan),
                Some(nan),
                Some(nan),
                Some(nan),
            ],
        ),
        (
            "atanh",
            atanh,
            [
                Some(nan),
                Some(nan),
                Some(nan),
                Some(0.0),
                Some(-0.0),
                Some(tiny),
                Some(-tiny),
            ],
        ),
        (
            "cbrt",
            cbrt,
            [
                Some(nan),
                Some(inf),
                Some(-inf),
                Some(0.0),
                Some(-0.0),
                Some(pow2(-358)),
                Some(-pow2(-358)),
            ],
        ),
    ];
    let arguments = [nan, inf, -inf, 0.0, -0.0, tiny, -tiny];
    let mut count = 0;
    for (name, function, expected) in table {
        for (argument, expected) in arguments.iter().zip(expected) {
            let actual = function(*argument);
            match expected {
                Some(expected) => assert_eq!(
                    bits(actual),
                    bits(expected),
                    "{name}({argument:e}) = {actual:e}, expected {expected:e}"
                ),
                None => {
                    let (lower, upper) = enclosure_of(name, *argument);
                    assert!(
                        bits(actual) == lower || bits(actual) == upper,
                        "{name}({argument:e}) = {:016X}, not {lower:016X} or {upper:016X}",
                        bits(actual)
                    );
                }
            }
            count += 1;
        }
    }
    // The draft's single values: log(1), log2(1), log10(1) and acosh(1) are +0.
    for (name, actual) in [
        ("log", log(1.0)),
        ("log2", log2(1.0)),
        ("log10", log10(1.0)),
        ("acosh", acosh(1.0)),
    ] {
        assert_eq!(bits(actual), 0, "{name}(1)");
        count += 1;
    }
    println!("math_exp_log special values: {count}");
}
