//! The functions of `swell_num::fmath` (`story:core-builds-without-std`): `sqrt`, `floor`,
//! `ceil`, `trunc`, `fmod` and `rem_euclid`, computed in the crate in the place of the `std`
//! methods of the same names.
//!
//! One corpus holds their results: `tests/fixtures/fmath/corpus.txt`, one line per call,
//! "<function> <edge|drawn> <x> [<y>] <result>" with each Number as the 16 hex digits of its
//! bits. The edge inputs are the list [`EDGES`] and the pairs [`EDGE_PAIRS`] (±0, ±∞, NaN, the
//! subnormals, exact powers and squares, halves, the integers around 2^52 and 2^53, values near
//! multiples of π/2, large arguments); the drawn inputs come from Knuth's 64-bit linear
//! congruential generator below, 256 per function. The test makes every line again from the
//! inputs and compares the file byte for byte, so the inputs are those of this file and the
//! results are the crate's. To print the lines of the current code in place of the comparison,
//! run the test with `SWELL_FMATH_CORPUS_PRINT=1` and `--nocapture`; the lines are between
//! "fmath-corpus begin" and "fmath-corpus end".
//!
//! The results are held a second time without the file:
//!
//! - the values IEEE 754-2019 and ECMA-262 fix, written out in [`fixed_values`];
//! - for every input, by exact arithmetic ([`exact`]): `sqrt` is correctly rounded (the exact
//!   root lies strictly between the midpoints to the two neighbours of the result, or is it);
//!   `floor`, `ceil` and `trunc` are integers less than 1 from x on their side, with x's sign;
//!   `fmod` has x's sign and a magnitude below |y|, and `rem_euclid` lies from 0 to |y|.

#![cfg(test)]

// Each test file that reads exact arithmetic loads the one file as a module of its own.
#[allow(clippy::duplicate_mod)]
#[path = "support/exact.rs"]
mod exact;

use std::cmp::Ordering;
use std::f64::consts::{FRAC_PI_2, PI, TAU};

use exact::Rat;
use swell_num::fmath;

/// The committed corpus.
const CORPUS: &str = include_str!("fixtures/fmath/corpus.txt");

/// The drawn inputs per function.
const DRAWN: usize = 256;

/// The smallest subnormal, 2^-1074.
const TINY: f64 = 5e-324;

/// The single inputs of every one-argument function.
const EDGES: &[f64] = &[
    0.0,
    -0.0,
    TINY,
    -TINY,
    3.0 * TINY,
    4.0 * TINY,
    2.225_073_858_507_201e-308,
    -2.225_073_858_507_201e-308,
    f64::MIN_POSITIVE,
    -f64::MIN_POSITIVE,
    1e-310,
    0.25,
    0.49999999999999994,
    -0.49999999999999994,
    0.5,
    -0.5,
    0.5000000000000001,
    -0.5000000000000001,
    0.9999999999999999,
    -0.9999999999999999,
    1.0,
    -1.0,
    1.0000000000000002,
    -1.0000000000000002,
    1.5,
    -1.5,
    2.0,
    2.5,
    -2.5,
    3.0,
    4.0,
    9.0,
    10.0,
    16.0,
    1e22,
    FRAC_PI_2,
    -FRAC_PI_2,
    PI,
    -PI,
    4.71238898038469,
    TAU,
    -TAU,
    4_503_599_627_370_495.5,
    -4_503_599_627_370_495.5,
    4_503_599_627_370_496.0,
    -4_503_599_627_370_496.0,
    4_503_599_627_370_497.0,
    9_007_199_254_740_991.0,
    9_007_199_254_740_992.0,
    -9_007_199_254_740_992.0,
    1e300,
    -1e300,
    f64::MAX,
    -f64::MAX,
    f64::INFINITY,
    f64::NEG_INFINITY,
    f64::NAN,
];

/// The pairs of the two-argument functions.
const EDGE_PAIRS: &[(f64, f64)] = &[
    (5.5, 2.0),
    (-5.5, 2.0),
    (5.5, -2.0),
    (-5.5, -2.0),
    (10.0, 3.0),
    (-10.0, 3.0),
    (7.0, 7.0),
    (-7.0, 7.0),
    (1.0, 0.1),
    (0.0, 1.0),
    (-0.0, 1.0),
    (1.0, 0.0),
    (1.0, -0.0),
    (f64::INFINITY, 1.0),
    (f64::NEG_INFINITY, 1.0),
    (1.0, f64::INFINITY),
    (-1.0, f64::NEG_INFINITY),
    (f64::NAN, 1.0),
    (1.0, f64::NAN),
    (-1e-300, 1.0),
    (f64::MAX, TINY),
    (TINY, f64::MAX),
    (3.0 * TINY, 2.0 * TINY),
    (f64::MIN_POSITIVE, 3.0 * TINY),
    (1e308, 1e-308),
    (f64::MAX, 1.0),
    (1e22, FRAC_PI_2),
    (TAU, PI),
    (4.71238898038469, FRAC_PI_2),
    (9_007_199_254_740_993.0, 3.0),
    (4_294_967_296.0 * 3.5, 4_294_967_296.0),
    (-4_294_967_297.0, 4_294_967_296.0),
    (-65_537.0, 65_536.0),
];

/// Knuth's 64-bit linear congruential generator (multiplier 6364136223846793005, increment
/// 1442695040888963407), the generator of the design's reference data.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0
    }

    /// A Number: every other draw any bits at all (every exponent, NaN and ±∞ among them), the
    /// others with a magnitude from 2^-60 to 2^60, where the integer functions have fraction
    /// bits to clear.
    fn number(&mut self, wide: bool) -> f64 {
        let bits = self.next();
        if wide {
            return f64::from_bits(bits);
        }
        let exponent = 1023 - 60 + (self.next() % 121);
        f64::from_bits((bits & ((1 << 63) | ((1 << 52) - 1))) | (exponent << 52))
    }
}

/// A function of the corpus with its name and the seed of its drawn inputs.
type Unary = (&'static str, fn(f64) -> f64, u64);
type Binary = (&'static str, fn(f64, f64) -> f64, u64);

/// The functions of the corpus.
const UNARY: &[Unary] = &[
    ("sqrt", fmath::sqrt, 1),
    ("floor", fmath::floor, 2),
    ("ceil", fmath::ceil, 3),
    ("trunc", fmath::trunc, 4),
];
const BINARY: &[Binary] = &[
    ("fmod", fmath::fmod, 5),
    ("rem_euclid", fmath::rem_euclid, 6),
];

fn hex(x: f64) -> String {
    format!("{:016x}", x.to_bits())
}

/// One call of the corpus.
enum Call {
    Unary(&'static str, fn(f64) -> f64, f64),
    Binary(&'static str, fn(f64, f64) -> f64, f64, f64),
}

/// Every call of the corpus with its kind, in the order of the file.
fn calls() -> Vec<(Call, &'static str)> {
    let mut calls = Vec::new();
    for &(name, f, seed) in UNARY {
        for &x in EDGES {
            calls.push((Call::Unary(name, f, x), "edge"));
        }
        let mut draw = Lcg(seed);
        for i in 0..DRAWN {
            calls.push((Call::Unary(name, f, draw.number(i % 2 == 0)), "drawn"));
        }
    }
    for &(name, f, seed) in BINARY {
        for &(x, y) in EDGE_PAIRS {
            calls.push((Call::Binary(name, f, x, y), "edge"));
        }
        let mut draw = Lcg(seed);
        for i in 0..DRAWN {
            let wide = i % 2 == 0;
            let (x, y) = (draw.number(wide), draw.number(wide));
            calls.push((Call::Binary(name, f, x, y), "drawn"));
        }
    }
    calls
}

fn line(call: &Call, kind: &str) -> String {
    match *call {
        Call::Unary(name, f, x) => format!("{name} {kind} {} {}", hex(x), hex(f(x))),
        Call::Binary(name, f, x, y) => {
            format!("{name} {kind} {} {} {}", hex(x), hex(y), hex(f(x, y)))
        }
    }
}

/// Every line of the corpus file equals the line the current code makes for the same input, and
/// the file has no other line.
#[test]
fn the_corpus_holds_every_result() {
    let made: Vec<String> = calls()
        .iter()
        .map(|(call, kind)| line(call, kind))
        .collect();
    if std::env::var_os("SWELL_FMATH_CORPUS_PRINT").is_some() {
        println!("fmath-corpus begin");
        for line in &made {
            println!("{line}");
        }
        println!("fmath-corpus end");
        return;
    }
    let held: Vec<&str> = CORPUS
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
        .collect();
    let differing: Vec<(usize, &String, &str)> = made
        .iter()
        .zip(&held)
        .enumerate()
        .filter(|(_, (m, h))| m.as_str() != **h)
        .map(|(i, (m, h))| (i, m, *h))
        .collect();
    for (i, made, held) in differing.iter().take(20) {
        println!("line {i}: made \"{made}\", the corpus has \"{held}\"");
    }
    assert!(differing.is_empty(), "{} line(s) differ", differing.len());
    assert_eq!(made.len(), held.len(), "lines made and lines in the corpus");
}

/// The values the standards fix, written out.
#[test]
fn fixed_values() {
    let same = |a: f64, b: f64| a.to_bits() == b.to_bits() || (a.is_nan() && b.is_nan());
    let cases: &[(&str, f64, f64)] = &[
        ("sqrt(+0)", fmath::sqrt(0.0), 0.0),
        ("sqrt(-0)", fmath::sqrt(-0.0), -0.0),
        ("sqrt(-1)", fmath::sqrt(-1.0), f64::NAN),
        ("sqrt(-∞)", fmath::sqrt(f64::NEG_INFINITY), f64::NAN),
        ("sqrt(+∞)", fmath::sqrt(f64::INFINITY), f64::INFINITY),
        ("sqrt(NaN)", fmath::sqrt(f64::NAN), f64::NAN),
        ("sqrt(4)", fmath::sqrt(4.0), 2.0),
        (
            "sqrt(2)",
            fmath::sqrt(2.0),
            f64::from_bits(0x3ff6_a09e_667f_3bcd),
        ),
        (
            "sqrt(2^-1074)",
            fmath::sqrt(TINY),
            f64::from_bits(0x1e60_0000_0000_0000),
        ),
        (
            "sqrt(4 × 2^-1074)",
            fmath::sqrt(4.0 * TINY),
            2.0 * f64::from_bits(0x1e60_0000_0000_0000),
        ),
        (
            "sqrt(MAX)",
            fmath::sqrt(f64::MAX),
            f64::from_bits(0x5fef_ffff_ffff_ffff),
        ),
        ("floor(-0)", fmath::floor(-0.0), -0.0),
        ("floor(0.5)", fmath::floor(0.5), 0.0),
        ("floor(-0.5)", fmath::floor(-0.5), -1.0),
        ("floor(-2^-1074)", fmath::floor(-TINY), -1.0),
        ("floor(-1.5)", fmath::floor(-1.5), -2.0),
        (
            "floor(2^52 - 0.5)",
            fmath::floor(4_503_599_627_370_495.5),
            4_503_599_627_370_495.0,
        ),
        (
            "floor(-(2^52 - 0.5))",
            fmath::floor(-4_503_599_627_370_495.5),
            -4_503_599_627_370_496.0,
        ),
        (
            "floor(-∞)",
            fmath::floor(f64::NEG_INFINITY),
            f64::NEG_INFINITY,
        ),
        ("ceil(-0.5)", fmath::ceil(-0.5), -0.0),
        ("ceil(2^-1074)", fmath::ceil(TINY), 1.0),
        ("ceil(-0)", fmath::ceil(-0.0), -0.0),
        ("ceil(1.5)", fmath::ceil(1.5), 2.0),
        (
            "ceil(0.9999999999999999)",
            fmath::ceil(0.9999999999999999),
            1.0,
        ),
        ("trunc(-0.7)", fmath::trunc(-0.7), -0.0),
        ("trunc(2.5)", fmath::trunc(2.5), 2.0),
        ("trunc(-2.5)", fmath::trunc(-2.5), -2.0),
        ("trunc(NaN)", fmath::trunc(f64::NAN), f64::NAN),
        ("fmod(5.5, 2)", fmath::fmod(5.5, 2.0), 1.5),
        ("fmod(-5.5, 2)", fmath::fmod(-5.5, 2.0), -1.5),
        ("fmod(5.5, -2)", fmath::fmod(5.5, -2.0), 1.5),
        ("fmod(-7, 7)", fmath::fmod(-7.0, 7.0), -0.0),
        ("fmod(-0, 1)", fmath::fmod(-0.0, 1.0), -0.0),
        ("fmod(1, 0)", fmath::fmod(1.0, 0.0), f64::NAN),
        ("fmod(∞, 1)", fmath::fmod(f64::INFINITY, 1.0), f64::NAN),
        ("fmod(1, ∞)", fmath::fmod(1.0, f64::INFINITY), 1.0),
        ("fmod(-1, -∞)", fmath::fmod(-1.0, f64::NEG_INFINITY), -1.0),
        (
            "fmod(3 × 2^-1074, 2 × 2^-1074)",
            fmath::fmod(3.0 * TINY, 2.0 * TINY),
            TINY,
        ),
        ("fmod(MAX, 2^-1074)", fmath::fmod(f64::MAX, TINY), 0.0),
        (
            "fmod(2^53 + 2, 3)",
            fmath::fmod(9_007_199_254_740_994.0, 3.0),
            1.0,
        ),
        ("rem_euclid(-1, 4)", fmath::rem_euclid(-1.0, 4.0), 3.0),
        ("rem_euclid(-1, -4)", fmath::rem_euclid(-1.0, -4.0), 3.0),
        ("rem_euclid(-0, 4)", fmath::rem_euclid(-0.0, 4.0), -0.0),
        (
            "rem_euclid(-1e-300, 1)",
            fmath::rem_euclid(-1e-300, 1.0),
            1.0,
        ),
        (
            "rem_euclid(-2^32 - 1, 2^32)",
            fmath::rem_euclid(-4_294_967_297.0, 4_294_967_296.0),
            4_294_967_295.0,
        ),
    ];
    let wrong: Vec<String> = cases
        .iter()
        .filter(|(_, got, want)| !same(*got, *want))
        .map(|(name, got, want)| format!("{name}: {got:e} ({}), not {want:e}", hex(*got)))
        .collect();
    assert!(wrong.is_empty(), "{wrong:#?}");
}

/// The Number after x toward +∞, for a finite positive x.
fn next_up(x: f64) -> f64 {
    f64::from_bits(x.to_bits() + 1)
}

/// The Number before x, for a finite positive x.
fn next_down(x: f64) -> f64 {
    f64::from_bits(x.to_bits() - 1)
}

/// Whether a finite x is an integer: no fraction bit below its binary point.
fn is_integer(x: f64) -> bool {
    let bits = x.to_bits() & !(1 << 63);
    let exponent = (bits >> 52) as i64 - 1023;
    if bits == 0 {
        return true;
    }
    if exponent < 0 {
        return false;
    }
    exponent >= 52 || bits & (((1u64 << 52) - 1) >> exponent) == 0
}

/// `sqrt(x)` is correctly rounded: r² = x, or (r⁻ + r)² / 4 < x < (r + r⁺)² / 4 for the
/// neighbours r⁻ and r⁺ of r.
fn sqrt_is_correctly_rounded(x: f64, r: f64) -> Result<(), String> {
    if !(x.is_finite() && x > 0.0) {
        return Ok(());
    }
    let exact = Rat::from_f64(x);
    let root = Rat::from_f64(r);
    if root.mul(&root).compare(&exact) == Ordering::Equal {
        return Ok(());
    }
    let quarter = Rat::pow2(-2);
    let below = root.add(&Rat::from_f64(next_down(r)));
    let above = root.add(&Rat::from_f64(next_up(r)));
    let low = below.mul(&below).mul(&quarter);
    let high = above.mul(&above).mul(&quarter);
    if low.compare(&exact) == Ordering::Less && exact.compare(&high) == Ordering::Less {
        Ok(())
    } else {
        Err(format!("sqrt({}) = {} is not the nearest", hex(x), hex(r)))
    }
}

/// `floor`, `ceil` or `trunc` of x is an integer less than 1 from x on the side of its
/// direction, with the sign of x.
fn integer_is_on_its_side(name: &str, x: f64, r: f64) -> Result<(), String> {
    if !x.is_finite() {
        return if r.to_bits() == x.to_bits() || (x.is_nan() && r.is_nan()) {
            Ok(())
        } else {
            Err(format!("{name}({}) = {}, not x", hex(x), hex(r)))
        };
    }
    let fail = |why: &str| Err(format!("{name}({}) = {}: {why}", hex(x), hex(r)));
    if !is_integer(r) {
        return fail("not an integer");
    }
    if r.is_sign_negative() != x.is_sign_negative() {
        return fail("not the sign of x");
    }
    let gap = Rat::from_f64(x).sub(&Rat::from_f64(r));
    let one = Rat::from_i64(1);
    let below_one = gap.abs().compare(&one) == Ordering::Less;
    let side = match name {
        "floor" => !gap.is_negative() || gap.is_zero(),
        "ceil" => gap.is_negative() || gap.is_zero(),
        _ => gap.is_zero() || gap.is_negative() == x.is_sign_negative(),
    };
    if below_one && side {
        Ok(())
    } else {
        fail("not within 1 on its side")
    }
}

/// `fmod(x, y)` has the sign of x and a magnitude below |y|; `rem_euclid(x, y)` lies from 0 to
/// |y|. NaN for a NaN, a zero y or an infinite x.
fn remainder_is_bounded(name: &str, x: f64, y: f64, r: f64) -> Result<(), String> {
    let fail = |why: &str| {
        Err(format!(
            "{name}({}, {}) = {}: {why}",
            hex(x),
            hex(y),
            hex(r)
        ))
    };
    if x.is_nan() || y.is_nan() || y == 0.0 || x.is_infinite() {
        return if r.is_nan() { Ok(()) } else { fail("not NaN") };
    }
    if r.is_nan() {
        return fail("NaN");
    }
    let (magnitude, bound) = (r.abs(), y.abs());
    let ok = if name == "fmod" {
        magnitude < bound && r.is_sign_negative() == x.is_sign_negative()
    } else {
        r >= 0.0 && r <= bound
    };
    if ok { Ok(()) } else { fail("out of its range") }
}

/// Every call of the corpus against exact arithmetic.
#[test]
fn every_result_is_exact() {
    let mut wrong = Vec::new();
    for (call, _) in calls() {
        let verdict = match call {
            Call::Unary("sqrt", f, x) => sqrt_is_correctly_rounded(x, f(x)),
            Call::Unary(name, f, x) => integer_is_on_its_side(name, x, f(x)),
            Call::Binary(name, f, x, y) => remainder_is_bounded(name, x, y, f(x, y)),
        };
        if let Err(why) = verdict {
            wrong.push(why);
        }
    }
    assert!(wrong.is_empty(), "{} wrong: {wrong:#?}", wrong.len());
}
