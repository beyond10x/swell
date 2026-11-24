//! `Math.atan`, `Math.atan2`, `Math.asin` and `Math.acos` (`#sec-math.atan`, `#sec-math.atan2`,
//! `#sec-math.asin`, `#sec-math.acos`) over arguments the caller has already coerced (design,
//! decision 6, "The functions of `Math` that IEEE 754 does not fix", item 4).
//!
//! One kernel serves all four: the angle θ in [0, π/2] of the magnitudes y and x as pairs. Both
//! are scaled by 2^-e, e the binary exponent of the larger, so that the larger lies in [1, 2):
//! the quotient is unchanged and every pair operation stays in its range, also for two
//! subnormal arguments; the smaller is exact after the scaling unless it falls below 2^-1022,
//! which needs exponents more than 1,022 apart and leaves it below 2^-1022 of the larger (the
//! design's scaling "when the exponents differ by 60 or more" is this one, taken on every
//! call). A quotient q = y/x above 1 is taken the other way round and θ is π/2 minus the
//! result. c is the nearest of 0, 1/8, ..., 1 to q, atan(c) comes from [`ATAN_OF_EIGHTHS`],
//! u = (q - c)/(1 + q c) has |u| ≤ 1/16, and atan u = u - u^3/3 + ... through u^19 (the degree
//! the design's rule gives; `crates/num/tests/math_trig.rs` derives it), by Horner's rule in u²
//! in double-double.
//!
//! `atan2` answers the draft's table of special values first (NaN, the infinities, the zeros
//! with their signs), then θ of |y| and |x|, π - θ for a negative x, and the sign of y.
//! `atan(x)` is θ of |x| and 1 with the sign of x. `asin(x)` is the angle of |x| and
//! sqrt((1 - |x|)(1 + |x|)) with the sign of x, and `acos(x)` the angle of that root and |x|,
//! π minus it for a negative x: the product of the two exact pairs 1 - |x| and 1 + |x|
//! (`two_sum`) is the root's operand, so no cancellation loses the angle near ±1.
//!
//! The accuracy (design, "The accuracy the engine promises"): `atan` is in group A, less than
//! 1 ulp; `atan2`, `asin` and `acos` in group B, at most 2 ulp. The test file holds them against
//! `data/math/reference-trig.txt` and a denser walk of its own.
//!
//! The operations and what fixes each result: the pair operations of [`super::dd`] (among them
//! [`Dd::sqrt`], on the correctly rounded `f64::sqrt`), `abs` and `copysign` (exact, IEEE
//! 754-2019, 5.5.1), comparisons, `is_nan`, `is_infinite`, a multiplication by 2 or 1/2
//! (exact) and the one rounding `hi + lo` (5.4.1).

use super::binary_exponent;
use super::dd::{Dd, pair, two_sum};
use super::reduce::PI_OVER_2;

/// atan(k/8) for k from 0 to 8 as pairs: the Number nearest to the value and the Number nearest
/// to the rest (atan(1) is π/4).
pub const ATAN_OF_EIGHTHS: [Dd; 9] = [
    pair(0x0000_0000_0000_0000, 0x0000_0000_0000_0000),
    pair(0x3FBF_D5BA_9AAC_2F6E, 0xBC4C_D376_8676_0C17),
    pair(0x3FCF_5B75_F92C_80DD, 0x3C68_AB6E_3CF7_AFBD),
    pair(0x3FD6_F619_41E4_DEF1, 0xBC7C_63AA_E6F6_E918),
    pair(0x3FDD_AC67_0561_BB4F, 0x3C7A_2B7F_222F_65E2),
    pair(0x3FE1_E00B_ABDE_FEB4, 0xBC59_28DF_287A_668F),
    pair(0x3FE4_978F_A326_9EE1, 0x3C72_419A_87F2_A458),
    pair(0x3FE7_00A7_C578_4634, 0xBC78_C34D_25AA_DEF6),
    pair(0x3FE9_21FB_5444_2D18, 0x3C81_A626_3314_5C07),
];

/// The coefficients of atan u after u: -1/3, 1/5, ..., -1/19, as pairs.
pub const ATAN_COEFFICIENTS: [Dd; 9] = [
    pair(0xBFD5_5555_5555_5555, 0xBC75_5555_5555_5555),
    pair(0x3FC9_9999_9999_999A, 0xBC69_9999_9999_999A),
    pair(0xBFC2_4924_9249_2492, 0xBC62_4924_9249_2492),
    pair(0x3FBC_71C7_1C71_C71C, 0x3C5C_71C7_1C71_C71C),
    pair(0xBFB7_45D1_745D_1746, 0x3C47_45D1_745D_1746),
    pair(0x3FB3_B13B_13B1_3B14, 0xBC53_B13B_13B1_3B14),
    pair(0xBFB1_1111_1111_1111, 0xBC31_1111_1111_1111),
    pair(0x3FAE_1E1E_1E1E_1E1E, 0x3C2E_1E1E_1E1E_1E1E),
    pair(0xBFAA_F286_BCA1_AF28, 0xBC4A_F286_BCA1_AF28),
];

/// The eighths c = k/8 and the least quotient for which each is the nearest, (2k - 1)/16.
const EIGHTHS: [(f64, f64); 9] = [
    (0.0, 0.0),
    (0.125, 0.0625),
    (0.25, 0.1875),
    (0.375, 0.3125),
    (0.5, 0.4375),
    (0.625, 0.5625),
    (0.75, 0.6875),
    (0.875, 0.8125),
    (1.0, 0.9375),
];

const ONE: Dd = Dd::from_f64(1.0);
const ZERO: Dd = Dd::from_f64(0.0);

/// π as a pair, twice π/2 (exact).
fn pi() -> Dd {
    Dd {
        hi: PI_OVER_2.hi * 2.0,
        lo: PI_OVER_2.lo * 2.0,
    }
}

/// a > b for normalized pairs.
fn greater(a: Dd, b: Dd) -> bool {
    a.hi > b.hi || (a.hi == b.hi && a.lo > b.lo)
}

/// The angle in [0, π/2] whose tangent is y / x, for finite y ≥ 0 and x ≥ 0 not both zero.
fn angle(y: Dd, x: Dd) -> Dd {
    let swapped = greater(y, x);
    let (numerator, denominator) = if swapped { (x, y) } else { (y, x) };
    if denominator.hi <= 0.0 {
        return ZERO;
    }
    let e = binary_exponent(denominator.hi);
    let q = numerator.scale2(-e) / denominator.scale2(-e);
    let mut k = 0;
    for (index, &(_, least)) in EIGHTHS.iter().enumerate() {
        if q.hi >= least {
            k = index;
        }
    }
    let (c, atan_c) = match (EIGHTHS.get(k), ATAN_OF_EIGHTHS.get(k)) {
        (Some(&(c, _)), Some(&atan_c)) => (Dd::from_f64(c), atan_c),
        _ => (ZERO, ZERO),
    };
    let u = (q - c) / (ONE + q * c);
    let z = u * u;
    let mut sum = ZERO;
    for &coefficient in ATAN_COEFFICIENTS.iter().rev() {
        sum = sum * z + coefficient;
    }
    let theta = atan_c + (u + u * (z * sum));
    if swapped { PI_OVER_2 - theta } else { theta }
}

/// The pair with the sign of a negative argument.
fn signed(value: Dd, negative: bool) -> Dd {
    if negative { -value } else { value }
}

/// `Math.atan ( x )` (`#sec-math.atan`): NaN and ±0 give themselves, ±∞ gives ±π/2 (the Number
/// nearest to it); otherwise less than 1 ulp from atan x.
pub fn atan(x: f64) -> f64 {
    if x.is_nan() || x == 0.0 {
        return x;
    }
    if x.is_infinite() {
        return PI_OVER_2.to_f64().copysign(x);
    }
    signed(angle(Dd::from_f64(x.abs()), ONE), x < 0.0).to_f64()
}

/// `Math.atan2 ( y, x )` (`#sec-math.atan2`): the draft's special values, then at most 2 ulp from
/// the angle of (x, y) in [-π, π].
pub fn atan2(y: f64, x: f64) -> f64 {
    if y.is_nan() || x.is_nan() {
        return f64::NAN;
    }
    let negative = y.is_sign_negative();
    if y.is_infinite() {
        let value = if x == f64::INFINITY {
            Dd {
                hi: PI_OVER_2.hi * 0.5,
                lo: PI_OVER_2.lo * 0.5,
            }
        } else if x == f64::NEG_INFINITY {
            PI_OVER_2
                + Dd {
                    hi: PI_OVER_2.hi * 0.5,
                    lo: PI_OVER_2.lo * 0.5,
                }
        } else {
            PI_OVER_2
        };
        return signed(value, negative).to_f64();
    }
    if y == 0.0 {
        // +0 for x > 0 or x = +0, π for x < 0 or x = -0; the sign of y.
        let value = if x.is_sign_positive() { ZERO } else { pi() };
        return signed(value, negative).to_f64();
    }
    if x.is_infinite() {
        let value = if x > 0.0 { ZERO } else { pi() };
        return signed(value, negative).to_f64();
    }
    if x == 0.0 {
        return signed(PI_OVER_2, negative).to_f64();
    }
    let theta = angle(Dd::from_f64(y.abs()), Dd::from_f64(x.abs()));
    let theta = if x < 0.0 { pi() - theta } else { theta };
    signed(theta, negative).to_f64()
}

/// sqrt((1 - a)(1 + a)) for 0 ≤ a ≤ 1, from the exact pairs 1 - a and 1 + a.
fn cosine_of(a: f64) -> Dd {
    (two_sum(1.0, -a) * two_sum(1.0, a)).sqrt()
}

/// `Math.asin ( x )` (`#sec-math.asin`): NaN and ±0 give themselves, a magnitude above 1 NaN;
/// otherwise at most 2 ulp from asin x (`asin(±1)` is ±π/2, the Number nearest to it).
pub fn asin(x: f64) -> f64 {
    if x.is_nan() || x == 0.0 {
        return x;
    }
    let a = x.abs();
    if a > 1.0 {
        return f64::NAN;
    }
    signed(angle(Dd::from_f64(a), cosine_of(a)), x < 0.0).to_f64()
}

/// `Math.acos ( x )` (`#sec-math.acos`): NaN and a magnitude above 1 give NaN, 1 gives +0;
/// otherwise at most 2 ulp from acos x (`acos(-1)` is π, the Number nearest to it).
pub fn acos(x: f64) -> f64 {
    let a = x.abs();
    if x.is_nan() || a > 1.0 {
        return f64::NAN;
    }
    if x == 1.0 {
        return 0.0;
    }
    let theta = angle(cosine_of(a), Dd::from_f64(a));
    let theta = if x < 0.0 { pi() - theta } else { theta };
    theta.to_f64()
}
