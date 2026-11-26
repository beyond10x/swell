//! The functions of `Math` that IEEE 754 does not fix (design, decision 6, "The functions of
//! `Math` that IEEE 754 does not fix"), in plain `f64` arithmetic, so that a script prints the
//! same bytes on every platform (HOST-135). The draft fixes their special values and says "an
//! implementation-approximated Number value" for the rest; `std`'s functions of the same names
//! are of unspecified precision and are not called (`libm-check` holds that).
//!
//! - [`dd`]: double-double arithmetic, in which every kernel is written.
//! - [`hypot`]: `Math.hypot` (`#sec-math.hypot`) over arguments the caller has already coerced.
//!
//! The accuracy promised (design, "The accuracy the engine promises"): `hypot` is in group A,
//! less than 1 ulp of the exact result (the result is one of the two Numbers that enclose it,
//! or the exact value when that is a Number). `crates/num/tests/math_dd.rs` holds it against
//! `data/math/reference-hypot.txt` and against exact arithmetic.
//!
//! The `f64` operations of this module and of [`dd`], and what fixes each result on every
//! platform: `+ - * /` (IEEE 754-2019, 5.4.1: correctly rounded to nearest; the
//! `compile_error!` below refuses a target whose `f64` arithmetic carries extended precision,
//! and rustc does not contract `a * b + c` into one fused operation), `fmath::sqrt` (correctly
//! rounded, 5.4.1), `abs`, `to_bits` and `from_bits` (exact), comparisons and the tests
//! `is_nan`, `is_infinite` and `is_finite`, and a multiplication by a power of two (exact when
//! the result is a normal Number). No fused multiply-add and no transcendental method of `std`.
//!
//! `swell-num` takes no budget: a loop over the arguments is the calling builtin's to charge.

#[cfg(all(target_arch = "x86", not(target_feature = "sse2")))]
compile_error!("swell-num needs SSE2 or a target without extended-precision f64 arithmetic");

pub mod atan;
pub mod dd;
pub mod exp;
pub mod log;
pub mod pow;
pub mod reduce;
pub mod trig;

pub use atan::{acos, asin, atan, atan2};
pub use exp::{cosh, exp, exp_dd, expm1, sinh, tanh};
pub use log::{acosh, asinh, atanh, log, log_dd, log1p, log2, log10};
pub use pow::{cbrt, exponentiate};
pub use trig::{cos, sin, tan};

/// The binary exponent e of a finite non-zero Number: 2^e ≤ |x| < 2^(e+1), from -1074 to 1023.
fn binary_exponent(x: f64) -> i32 {
    let bits = x.to_bits() & 0x7FFF_FFFF_FFFF_FFFF;
    let biased = bits >> 52;
    if biased == 0 {
        // A subnormal: its highest set bit, at position 0 to 51, is 2^(position - 1074).
        let position = 63u32.saturating_sub(bits.leading_zeros());
        i32::try_from(position).unwrap_or(0) - 1074
    } else {
        // 1 to 2046 for a finite Number.
        i32::try_from(biased).unwrap_or(1) - 1023
    }
}

/// `Math.hypot ( ...args )` (`#sec-math.hypot`) after step 1, which coerces every argument:
/// an infinite argument gives +∞ (a NaN beside it too), else a NaN gives NaN, else arguments
/// that are all zero give +0 (so does no argument). One argument gives its absolute value.
///
/// The rest is "the square root of the sum of squares", less than 1 ulp from the exact value:
/// every argument is scaled by 2^-e, e the binary exponent of the largest magnitude, so that
/// the largest lies in [1, 2) and no square overflows or underflows (a power of two: exact for
/// every argument whose scaled value is a normal Number; one that falls below that is under
/// 2^-1022 and its square is below the last bit of the sum); each square is exact by
/// [`dd::two_product`]; the squares are summed by Neumaier's compensated sum, each addition by
/// [`dd::two_sum`] with its error and the low part of the square added to the compensation;
/// the pair of sum and compensation takes [`dd::Dd::sqrt`], is scaled back by 2^e and rounded
/// once.
pub fn hypot(values: &[f64]) -> f64 {
    if values.iter().any(|v| v.is_infinite()) {
        return f64::INFINITY;
    }
    if values.iter().any(|v| v.is_nan()) {
        return f64::NAN;
    }
    let mut largest = 0.0_f64;
    for &v in values {
        let magnitude = v.abs();
        if magnitude > largest {
            largest = magnitude;
        }
    }
    if largest == 0.0 {
        return 0.0;
    }
    if let [only] = values {
        return only.abs();
    }
    let e = binary_exponent(largest);
    let mut sum = 0.0;
    let mut compensation = 0.0;
    for &v in values {
        let scaled = dd::scale(v, -e);
        let square = dd::two_product(scaled, scaled);
        let step = dd::two_sum(sum, square.hi);
        sum = step.hi;
        compensation += step.lo + square.lo;
    }
    dd::fast_two_sum(sum, compensation)
        .sqrt()
        .scale2(e)
        .to_f64()
}
