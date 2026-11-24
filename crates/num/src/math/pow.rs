//! `Number::exponentiate` (`#sec-numeric-types-number-exponentiate`), which `Math.pow`
//! (`#sec-math.pow`) and the operator `**` share, and `Math.cbrt` (`#sec-math.cbrt`) (design,
//! decision 6, "The functions of `Math` that IEEE 754 does not fix", items 6 and 7).
//!
//! [`exponentiate`] answers the draft's cases before its last line as the draft writes them (a
//! NaN exponent, a zero exponent, a NaN base, the infinities and the zeros with the rule of the
//! odd integral exponent, an infinite exponent against |base| 1, which is NaN, and a negative
//! base with an exponent that is not integral, which is NaN). For a finite non-zero base and a
//! finite non-zero exponent, |base|^exponent = 2^t with t = exponent × log2|base|: log2|base| is
//! ln|base| ([`log_dd`]) times 1/ln 2 as pairs, relative error at most 2^-79; t is the product in
//! double-double; k is the nearest integer to t (`floor` of t's high part plus one half, moved by
//! one where t - k lies beyond ±1/2), r = (t - k) × ln 2 in double-double, and 2^t = 2^k × e^r
//! with the kernel of [`super::exp`], rounded once and scaled by [`dd::scale`]. t beyond ±1,100
//! answers ∞ or 0 directly. A negative base with an odd integral exponent gives the negative of
//! the magnitude (so -∞ and -0 by the same rule). |t| is below 2^11 for every result that is a
//! Number, so the absolute error of t is below 2^-68 and the relative error of the result below
//! 2^-67, far under half an ulp (2^-53): a result that is exactly a Number is returned exactly
//! (`pow(2, 10)` is 1024, `pow(10, 2)` is 100), and `pow(2, -2^31)` is +0 by underflow.
//!
//! [`cbrt`] writes |x| = 2^(3j) × w with w in [1, 8) (exact: a power of two, a subnormal x scaled
//! by 2^54 first), y0 = exp(log(w) / 3) in double-double ([`exp_dd`], [`log_dd`]), then one
//! Newton step y = y0 - (y0³ - w) / (3·y0²) in double-double, rounded once, times 2^j (exact) and
//! with the sign of x; a perfect cube gives its root exactly.
//!
//! The accuracy promised (design, "The accuracy the engine promises"): both are in group A, less
//! than 1 ulp. `crates/num/tests/math_exp_log.rs` holds them against
//! `data/math/reference-exp-log.txt`, the draft's table and the exact cases.
//!
//! The `f64` operations, and what fixes each result: `+ - * /` (IEEE 754-2019, 5.4.1: correctly
//! rounded), `floor`, `abs` and `copysign` (exact: 5.9, 5.5.1), `to_bits` and `from_bits`
//! (exact), comparisons and `is_nan`, `is_infinite`, `is_finite`, and `as i32` of an integral
//! `f64` after a range test (in [`super::exp`]). No loop depends on the argument.

use super::dd::{self, Dd};
use super::exp::{INV_LN2, LN2, exp_dd, expm1_kernel, small_integer};
use super::log::log_dd;

/// 2^53: from here on every Number is an even integer.
const TWO_53: f64 = 9_007_199_254_740_992.0;

/// 2^54, the scaling of a subnormal argument of `cbrt`.
const TWO_54: f64 = f64::from_bits(0x4350_0000_0000_0000);

/// Beyond this magnitude of t = exponent × log2|base|, 2^t is beyond the largest Number (t above
/// 1024) or below half the smallest subnormal (t below -1075), whatever the rounding of t.
const T_LIMIT: f64 = 1100.0;

/// Whether a Number is integral: finite and equal to its `floor`.
fn is_integral(x: f64) -> bool {
    x.is_finite() && x.floor() == x
}

/// Whether a Number is an odd integral Number: integral, below 2^53 in magnitude (above it every
/// Number is even), and x / 2 not integral (x / 2 is exact for an integer of magnitude at
/// least 1).
fn is_odd_integral(x: f64) -> bool {
    is_integral(x) && x.abs() < TWO_53 && (x * 0.5).floor() != x * 0.5
}

/// |base|^exponent for a positive finite base and a finite non-zero exponent, as 2^t with
/// t = exponent × log2(base) in double-double.
fn magnitude(base: f64, exponent: f64) -> f64 {
    let log2 = log_dd(base) * INV_LN2;
    let rough = exponent * log2.hi;
    if rough > T_LIMIT {
        return f64::INFINITY;
    }
    if rough < -T_LIMIT {
        return 0.0;
    }
    let t = Dd::from_f64(exponent) * log2;
    let mut k = (t.hi + 0.5).floor();
    let mut fraction = t - Dd::from_f64(k);
    if fraction.hi > 0.5 {
        k += 1.0;
        fraction = t - Dd::from_f64(k);
    } else if fraction.hi < -0.5 {
        k -= 1.0;
        fraction = t - Dd::from_f64(k);
    }
    let p = Dd::from_f64(1.0) + expm1_kernel(fraction * LN2);
    dd::scale(p.to_f64(), small_integer(k))
}

/// `Number::exponentiate ( base, exponent )` (`#sec-numeric-types-number-exponentiate`): the
/// draft's cases in its order, then base^exponent, less than 1 ulp from the exact value.
pub fn exponentiate(base: f64, exponent: f64) -> f64 {
    if exponent.is_nan() {
        return f64::NAN;
    }
    if exponent == 0.0 {
        return 1.0;
    }
    if base.is_nan() {
        return f64::NAN;
    }
    if base == f64::INFINITY {
        return if exponent > 0.0 { f64::INFINITY } else { 0.0 };
    }
    if base == f64::NEG_INFINITY {
        let odd = is_odd_integral(exponent);
        return match (exponent > 0.0, odd) {
            (true, true) => f64::NEG_INFINITY,
            (true, false) => f64::INFINITY,
            (false, true) => -0.0,
            (false, false) => 0.0,
        };
    }
    if base == 0.0 {
        if base.is_sign_positive() {
            return if exponent > 0.0 { 0.0 } else { f64::INFINITY };
        }
        let odd = is_odd_integral(exponent);
        return match (exponent > 0.0, odd) {
            (true, true) => -0.0,
            (true, false) => 0.0,
            (false, true) => f64::NEG_INFINITY,
            (false, false) => f64::INFINITY,
        };
    }
    if exponent.is_infinite() {
        let a = base.abs();
        return if a == 1.0 {
            f64::NAN
        } else if (a > 1.0) == (exponent > 0.0) {
            f64::INFINITY
        } else {
            0.0
        };
    }
    if base < 0.0 && !is_integral(exponent) {
        return f64::NAN;
    }
    let result = magnitude(base.abs(), exponent);
    if base < 0.0 && is_odd_integral(exponent) {
        -result
    } else {
        result
    }
}

/// `Math.cbrt ( x )` (`#sec-math.cbrt`) after `ToNumber`: NaN, ±∞ and ±0 are returned; the rest
/// is the cube root, less than 1 ulp from the exact value and exact for a perfect cube.
pub fn cbrt(x: f64) -> f64 {
    if !x.is_finite() || x == 0.0 {
        return x;
    }
    let a = x.abs();
    let e = if a < f64::MIN_POSITIVE {
        super::binary_exponent(a * TWO_54) - 54
    } else {
        super::binary_exponent(a)
    };
    let j = e.div_euclid(3);
    // a × 2^-3j, with 2^(e - 3j) ≤ w < 2^(e - 3j + 1) and e - 3j in {0, 1, 2}: a power of two
    // whose product is a normal Number in [1, 8), so exact.
    let w = Dd::from_f64(dd::scale(a, -3 * j));
    let three = Dd::from_f64(3.0);
    let logarithm = log_dd(w.hi) / three;
    let y0 = exp_dd(logarithm);
    let square = y0 * y0;
    let y = y0 - (square * y0 - w) / (three * square);
    dd::scale(y.to_f64(), j).copysign(x)
}
