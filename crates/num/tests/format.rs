//! `toFixed`, `toExponential` and `toPrecision` of `swell_num::format`
//! (`#sec-number.prototype.tofixed`, `#sec-number.prototype.toexponential`,
//! `#sec-number.prototype.toprecision`) against an oracle of this file: the exact decimal
//! expansion of x that the Rust standard library writes with 1,100 digits after the point
//! (`{:.1100}` and `{:.1100e}`; every finite Number has at most 1,074, so nothing is rounded
//! there), cut to the digits the method asks for and rounded half up by string arithmetic, the
//! larger of two equally near as the clauses say.
//!
//! The values are random bit patterns drawn by Knuth's 64-bit linear congruential generator of
//! the design (decision 6) with fixed seeds, and the edges: zeros, ties, the smallest
//! subnormal, the largest Number below 10^21 and, for the exponent forms, the largest Number.

#![cfg(test)]

use swell_num::format::{to_exponential, to_fixed, to_precision};
use swell_num::to_string::number_to_string;

/// Whether the tests of drawn values run their full counts. The lane `slow` sets
/// `SWELL_FULL_COUNTS=1`.
fn full_counts() -> bool {
    std::env::var("SWELL_FULL_COUNTS").is_ok_and(|value| value == "1")
}

/// Knuth's 64-bit linear congruential generator: multiplier 6364136223846793005, increment
/// 1442695040888963407, modulus 2^64.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0
    }

    /// The next finite Number whose magnitude is below `bound`, from random bits.
    fn finite_below(&mut self, bound: f64) -> f64 {
        loop {
            let x = f64::from_bits(self.next());
            if x.is_finite() && x.abs() < bound {
                return x;
            }
        }
    }
}

/// The digit string plus one in its last place, and whether it grew by a digit.
fn increment(digits: &mut Vec<u8>) -> bool {
    for digit in digits.iter_mut().rev() {
        if *digit == b'9' {
            *digit = b'0';
        } else {
            *digit += 1;
            return false;
        }
    }
    digits.insert(0, b'1');
    true
}

/// The oracle of `toFixed(f)` for a finite x with abs(x) < 10^21.
fn fixed_oracle(x: f64, f: usize) -> String {
    let exact = format!("{:.1100}", x.abs());
    let (int, frac) = exact.split_once('.').unwrap();
    let mut digits: Vec<u8> = int.bytes().chain(frac.bytes().take(f)).collect();
    if frac.as_bytes()[f] >= b'5' {
        increment(&mut digits);
    }
    let split = digits.len() - f;
    let mut out = String::new();
    if x < 0.0 {
        out.push('-');
    }
    out.push_str(std::str::from_utf8(&digits[..split]).unwrap());
    if f > 0 {
        out.push('.');
        out.push_str(std::str::from_utf8(&digits[split..]).unwrap());
    }
    out
}

/// The p digits of x, rounded half up, and the exponent of the first, from `{:.1100e}`.
fn significant_oracle(x: f64, p: usize) -> (Vec<u8>, i32) {
    let exact = format!("{:.1100e}", x.abs());
    let (mantissa, exponent) = exact.split_once('e').unwrap();
    let mut exponent: i32 = exponent.parse().unwrap();
    let all: Vec<u8> = mantissa.bytes().filter(|&b| b != b'.').collect();
    let mut digits = all[..p].to_vec();
    if all[p] >= b'5' && increment(&mut digits) {
        digits.pop();
        exponent += 1;
    }
    (digits, exponent)
}

fn exponential_text(x: f64, digits: &[u8], exponent: i32) -> String {
    let mut out = String::new();
    if x < 0.0 {
        out.push('-');
    }
    out.push(char::from(digits[0]));
    if digits.len() > 1 {
        out.push('.');
        out.push_str(std::str::from_utf8(&digits[1..]).unwrap());
    }
    out.push('e');
    out.push(if exponent < 0 { '-' } else { '+' });
    out.push_str(&exponent.unsigned_abs().to_string());
    out
}

/// The oracle of `toExponential(f)` for a finite x.
fn exponential_oracle(x: f64, f: usize) -> String {
    let (digits, exponent) = significant_oracle(x, f + 1);
    exponential_text(x, &digits, exponent)
}

/// The oracle of `toPrecision(p)` for a finite x: the layout of steps 10 to 13 over the
/// digits of the oracle.
fn precision_oracle(x: f64, p: usize) -> String {
    let (digits, e) = significant_oracle(x, p);
    let p = p as i32;
    if e < -6 || e >= p {
        return exponential_text(x, &digits, e);
    }
    let text = std::str::from_utf8(&digits).unwrap();
    let sign = if x < 0.0 { "-" } else { "" };
    if e == p - 1 {
        format!("{sign}{text}")
    } else if e >= 0 {
        let (int, frac) = text.split_at((e + 1) as usize);
        format!("{sign}{int}.{frac}")
    } else {
        format!("{sign}0.{}{text}", "0".repeat((-(e + 1)) as usize))
    }
}

/// The values of the oracle tests: `count` drawn below `bound` with the seed `seed`, after the
/// edges.
fn values(seed: u64, count: usize, bound: f64, edges: &[f64]) -> Vec<f64> {
    let mut draw = Lcg(seed);
    let mut values = edges.to_vec();
    while values.len() < count {
        values.push(draw.finite_below(bound));
    }
    values
}

/// The largest Number below 10^21.
fn below_1e21() -> f64 {
    f64::from_bits(1e21f64.to_bits() - 1)
}

/// The edges of `toFixed`: zeros, ties at 0 digits, 1.005 (below its tie: 1.00499...), the
/// largest Number below 10^21, the smallest subnormal and their negatives.
fn fixed_edges() -> Vec<f64> {
    let mut edges = vec![0.0, -0.0, 0.5, 1.5, 2.5, 1.005, 1.25, below_1e21(), 5e-324];
    for x in [0.5, 1.5, 2.5, 1.005, below_1e21(), 5e-324] {
        edges.push(-x);
    }
    edges
}

/// The count of values the oracle tests draw: 3,000 at the full count, a tenth in the gate.
fn oracle_count() -> usize {
    if full_counts() { 3_000 } else { 300 }
}

/// `to_fixed` equals the oracle for 3,000 values below 10^21 (a tenth in the gate) times 0, 1,
/// 2, 20 and 100 fraction digits.
#[test]
fn to_fixed_equals_the_exact_expansion_rounded_half_up() {
    for x in values(1, oracle_count(), 1e21, &fixed_edges()) {
        for f in [0usize, 1, 2, 20, 100] {
            assert_eq!(
                to_fixed(x, f as u32),
                fixed_oracle(x, f),
                "{x:e} ({:#018x}) with {f} digits",
                x.to_bits()
            );
        }
    }
}

/// `to_exponential` with a count of digits equals the oracle for 3,000 finite values (a tenth
/// in the gate) times 0, 1, 2, 20 and 100 fraction digits, with the largest Number and the
/// smallest subnormal among them.
#[test]
fn to_exponential_equals_the_exact_expansion_rounded_half_up() {
    let mut edges = fixed_edges();
    edges.extend([f64::MAX, -f64::MAX, f64::MIN_POSITIVE, 1e21, 9.5, 0.95]);
    for x in values(2, oracle_count(), f64::INFINITY, &edges) {
        for f in [0usize, 1, 2, 20, 100] {
            assert_eq!(
                to_exponential(x, Some(f as u32)),
                exponential_oracle(x, f),
                "{x:e} ({:#018x}) with {f} digits",
                x.to_bits()
            );
        }
    }
}

/// `to_precision` equals the oracle for 3,000 finite values (a tenth in the gate) times 1, 2,
/// 3, 21 and 100 digits.
#[test]
fn to_precision_equals_the_exact_expansion_rounded_half_up() {
    let mut edges = fixed_edges();
    edges.extend([
        f64::MAX,
        -f64::MAX,
        1e21,
        1e-7,
        1e-6,
        9.5e-7,
        123_456.0,
        0.000_001,
    ]);
    for x in values(3, oracle_count(), f64::INFINITY, &edges) {
        for p in [1usize, 2, 3, 21, 100] {
            assert_eq!(
                to_precision(x, p as u32),
                precision_oracle(x, p),
                "{x:e} ({:#018x}) with {p} digits",
                x.to_bits()
            );
        }
    }
}

/// Exact ties go to the larger magnitude: 0.5, 2.5 and 1.25 at one digit fewer than they have,
/// with either sign; the cases of the standard's notes and of test262.
#[test]
fn ties_round_to_the_larger_and_the_notes() {
    let cases: [(&str, String, &str); 22] = [
        ("(0.5).toFixed(0)", to_fixed(0.5, 0), "1"),
        ("(2.5).toFixed(0)", to_fixed(2.5, 0), "3"),
        ("(-2.5).toFixed(0)", to_fixed(-2.5, 0), "-3"),
        ("(1.25).toFixed(1)", to_fixed(1.25, 1), "1.3"),
        ("(-1.5).toFixed(0)", to_fixed(-1.5, 0), "-2"),
        ("(-0).toFixed(2)", to_fixed(-0.0, 2), "0.00"),
        ("(1.005).toFixed(2)", to_fixed(1.005, 2), "1.00"),
        (
            "(2.5).toExponential(0)",
            to_exponential(2.5, Some(0)),
            "3e+0",
        ),
        (
            "(1.25).toExponential(1)",
            to_exponential(1.25, Some(1)),
            "1.3e+0",
        ),
        (
            "(25).toExponential(0)",
            to_exponential(25.0, Some(0)),
            "3e+1",
        ),
        (
            "(12345).toExponential(3)",
            to_exponential(12_345.0, Some(3)),
            "1.235e+4",
        ),
        ("(0.5).toPrecision(1)", to_precision(0.5, 1), "0.5"),
        ("(2.5).toPrecision(1)", to_precision(2.5, 1), "3"),
        ("(1.25).toPrecision(2)", to_precision(1.25, 2), "1.3"),
        (
            "(0.000001).toPrecision(2)",
            to_precision(0.000_001, 2),
            "0.0000010",
        ),
        (
            "(1000000000000000128).toFixed(0)",
            to_fixed(1_000_000_000_000_000_128.0, 0),
            "1000000000000000128",
        ),
        (
            "(123.456).toExponential()",
            to_exponential(123.456, None),
            "1.23456e+2",
        ),
        ("(0).toExponential()", to_exponential(0.0, None), "0e+0"),
        (
            "(0).toExponential(2)",
            to_exponential(0.0, Some(2)),
            "0.00e+0",
        ),
        ("(0).toPrecision(3)", to_precision(0.0, 3), "0.00"),
        ("(-0).toPrecision(1)", to_precision(-0.0, 1), "0"),
        ("(-1e-7).toFixed(2)", to_fixed(-1e-7, 2), "-0.00"),
    ];
    for (call, ours, expected) in cases {
        assert_eq!(ours, expected, "{call}");
    }
}

/// NaN, the infinities and every x with abs(x) ≥ 10^21 answer `Number::toString(x)`.
#[test]
fn not_finite_and_large_answer_to_string() {
    for x in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(to_fixed(x, 2), number_to_string(x));
        assert_eq!(to_exponential(x, Some(2)), number_to_string(x));
        assert_eq!(to_exponential(x, None), number_to_string(x));
        assert_eq!(to_precision(x, 2), number_to_string(x));
    }
    for x in [1e21, -1e21, 1.5e300, f64::MAX] {
        assert_eq!(to_fixed(x, 2), number_to_string(x));
    }
}

/// The extremes of the design's bound: 101 digits of 2^-1074 (10^424 against 2^1,074) and of
/// the largest Number (2^1,024 against 10^208), and 100 fraction digits of 10^20 and of 2^-1074.
#[test]
fn the_extremes_of_the_bound() {
    let tiny = 5e-324;
    let exponential = to_exponential(tiny, Some(100));
    assert_eq!(exponential, exponential_oracle(tiny, 100));
    assert!(exponential.ends_with("e-324"));
    assert_eq!(exponential.len(), "4.".len() + 100 + "e-324".len());
    let precision = to_precision(tiny, 100);
    assert_eq!(precision, precision_oracle(tiny, 100));
    assert_eq!(precision.len(), "4.".len() + 99 + "e-324".len());
    let largest = to_exponential(f64::MAX, Some(100));
    assert_eq!(largest, exponential_oracle(f64::MAX, 100));
    assert!(largest.ends_with("e+308"));
    assert_eq!(to_precision(f64::MAX, 100), precision_oracle(f64::MAX, 100));
    assert_eq!(
        to_fixed(1e20, 100),
        format!("100000000000000000000.{}", "0".repeat(100))
    );
    assert_eq!(to_fixed(tiny, 100), format!("0.{}", "0".repeat(100)));
    assert_eq!(to_fixed(-tiny, 100), format!("-0.{}", "0".repeat(100)));
}
