//! `Math.sin`, `Math.cos` and `Math.tan` (`#sec-math.sin`, `#sec-math.cos`, `#sec-math.tan`)
//! over an argument the caller has already coerced (design, decision 6, "The functions of `Math`
//! that IEEE 754 does not fix", item 1).
//!
//! The draft fixes the special values: NaN and ±∞ give NaN, `sin(±0)` and `tan(±0)` are ±0,
//! `cos(±0)` is 1. Every other argument is reduced by [`reduce`] to the quadrant k and r = x -
//! k π/2 with |r| ≤ π/4, and the kernels evaluate the Maclaurin series on r in double-double, by
//! Horner's rule in r², each coefficient a pair: `sin` through r^21 and `cos` through r^22 (the
//! degrees the design's rule gives, the first omitted term below 2^-80 of the result on the
//! whole interval; `crates/num/tests/math_trig.rs` derives them). The quadrant picks the kernel
//! and the sign: sin x is S, C, -S, -C and cos x is C, -S, -C, S for k = 0, 1, 2, 3. `tan` is
//! the quotient of the two kernels, S / C in the even quadrants and -C / S in the odd ones.
//! The pair is rounded once.
//!
//! The accuracy (design, "The accuracy the engine promises"): `sin` and `cos` are in group A,
//! less than 1 ulp; `tan` in group B, at most 2 ulp. The test file holds both against
//! `data/math/reference-trig.txt` and a denser walk of its own.
//!
//! The operations and what fixes each result: the pair operations of [`super::dd`], comparisons
//! and `is_nan`, `is_infinite` (exact), and the one rounding `hi + lo` (IEEE 754-2019, 5.4.1).

use super::dd::{Dd, pair};
use super::reduce::reduce;

/// The coefficients of `sin` after r: -1/3!, 1/5!, ..., 1/21!, each the Number nearest to the
/// fraction and the Number nearest to the rest.
pub const SIN_COEFFICIENTS: [Dd; 10] = [
    pair(0xBFC5_5555_5555_5555, 0xBC65_5555_5555_5555),
    pair(0x3F81_1111_1111_1111, 0x3C01_1111_1111_1111),
    pair(0xBF2A_01A0_1A01_A01A, 0xBB6A_01A0_1A01_A01A),
    pair(0x3EC7_1DE3_A556_C734, 0xBB6C_154F_8DDC_6C00),
    pair(0xBE5A_E645_67F5_44E4, 0x3AFC_062E_06D1_F209),
    pair(0x3DE6_1246_13A8_6D09, 0x3A8F_28E0_CC74_8EBE),
    pair(0xBD6A_E7F3_E733_B81F, 0xB9E1_D865_6B0E_E8CB),
    pair(0x3CE9_52C7_7030_AD4A, 0x398A_C981_465D_DC6C),
    pair(0xBC62_F49B_4681_4157, 0xB8F2_650F_61DB_DCB4),
    pair(0x3BD7_1B8E_F6DC_F572, 0xB87D_043A_E40C_4647),
];

/// The coefficients of `cos` after 1: -1/2!, 1/4!, ..., -1/22!, as pairs.
pub const COS_COEFFICIENTS: [Dd; 11] = [
    pair(0xBFE0_0000_0000_0000, 0x0000_0000_0000_0000),
    pair(0x3FA5_5555_5555_5555, 0x3C45_5555_5555_5555),
    pair(0xBF56_C16C_16C1_6C17, 0x3BEF_49F4_9F49_F49F),
    pair(0x3EFA_01A0_1A01_A01A, 0x3B3A_01A0_1A01_A01A),
    pair(0xBE92_7E4F_B778_9F5C, 0xBB3C_BBC0_5B4F_A99A),
    pair(0x3E21_EED8_EFF8_D898, 0xBAC2_AEC9_59E1_4C06),
    pair(0xBDA9_3974_A8C0_7C9D, 0xBA30_5D6F_8A2E_FD1F),
    pair(0x3D2A_E7F3_E733_B81F, 0x39A1_D865_6B0E_E8CB),
    pair(0xBCA6_8278_63B9_7D97, 0xB94E_EC01_221A_8B0B),
    pair(0x3C1E_542B_A402_0225, 0x387E_A72B_4AFE_3C2F),
    pair(0xBB90_CE39_6DB7_F853, 0x383A_EBCD_BD20_331C),
];

const ONE: Dd = Dd::from_f64(1.0);

/// Horner's rule in z over the coefficients, the last one first: c0 + z (c1 + z (c2 + ...)).
fn horner(coefficients: &[Dd], z: Dd) -> Dd {
    let mut sum = Dd::from_f64(0.0);
    for &coefficient in coefficients.iter().rev() {
        sum = sum * z + coefficient;
    }
    sum
}

/// sin r for |r| ≤ π/4: r + r × z × (-1/3! + z/5! - ...), z = r².
fn sin_kernel(r: Dd) -> Dd {
    let z = r * r;
    r + r * (z * horner(&SIN_COEFFICIENTS, z))
}

/// cos r for |r| ≤ π/4: 1 + z × (-1/2! + z/4! - ...), z = r².
fn cos_kernel(r: Dd) -> Dd {
    let z = r * r;
    ONE + z * horner(&COS_COEFFICIENTS, z)
}

/// `Math.sin ( x )` (`#sec-math.sin`): NaN and ±0 give themselves, ±∞ NaN; otherwise less than
/// 1 ulp from sin x.
pub fn sin(x: f64) -> f64 {
    if x.is_nan() || x == 0.0 {
        return x;
    }
    if x.is_infinite() {
        return f64::NAN;
    }
    let (quadrant, r) = reduce(x);
    let value = match quadrant {
        0 => sin_kernel(r),
        1 => cos_kernel(r),
        2 => -sin_kernel(r),
        _ => -cos_kernel(r),
    };
    value.to_f64()
}

/// `Math.cos ( x )` (`#sec-math.cos`): NaN and ±∞ give NaN, ±0 gives 1; otherwise less than 1 ulp
/// from cos x.
pub fn cos(x: f64) -> f64 {
    if !x.is_finite() {
        return f64::NAN;
    }
    if x == 0.0 {
        return 1.0;
    }
    let (quadrant, r) = reduce(x);
    let value = match quadrant {
        0 => cos_kernel(r),
        1 => -sin_kernel(r),
        2 => -cos_kernel(r),
        _ => sin_kernel(r),
    };
    value.to_f64()
}

/// `Math.tan ( x )` (`#sec-math.tan`): NaN and ±0 give themselves, ±∞ NaN; otherwise at most 2 ulp
/// from tan x.
pub fn tan(x: f64) -> f64 {
    if x.is_nan() || x == 0.0 {
        return x;
    }
    if x.is_infinite() {
        return f64::NAN;
    }
    let (quadrant, r) = reduce(x);
    let value = if quadrant.is_multiple_of(2) {
        sin_kernel(r) / cos_kernel(r)
    } else {
        -(cos_kernel(r) / sin_kernel(r))
    };
    value.to_f64()
}
