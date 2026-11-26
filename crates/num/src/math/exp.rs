//! The exponential family of `Math` (design, decision 6, "The functions of `Math` that IEEE 754
//! does not fix", item 2): [`exp`] (`#sec-math.exp`), [`expm1`] (`#sec-math.expm1`), [`sinh`]
//! (`#sec-math.sinh`), [`cosh`] (`#sec-math.cosh`) and [`tanh`] (`#sec-math.tanh`), and the
//! helper [`exp_dd`], each over a Number the caller has already coerced.
//!
//! The reduction: x = k·ln 2 + r with k the nearest integer to x / ln 2, so that |r| ≤ ln 2 / 2.
//! k is `floor` of the high part of the double-double product x × (1/ln 2) plus one half; the
//! remainder r = x - k·L1 - k·L2 - k·L3 is a pair, with ln 2 in the three parts [`LN2_PARTS`]
//! (32, 32 and 53 bits: k·L1 and k·L2 are exact for |k| below 2^11, k·L3 is exact by
//! [`dd::two_product`]); where the remainder's high part lies beyond ±(ln 2 / 2), k moves by one
//! and r is taken again, so the comparison of the remainder decides a near tie, never a
//! rounding to even. The kernel is the Maclaurin series of e^r - 1 through r^18 (the degree of
//! the design's cut: the first omitted term, r^19/19!, is below 2^-80 of the result on the whole
//! interval, and r^18/18! is not), evaluated by Horner's rule in double-double with each
//! coefficient 1/n! a pair ([`EXP_COEFFICIENTS`]) and its leading term r exact. The result is the
//! pair times 2^k, rounded once and scaled by [`dd::scale`] (two factors where 2^k alone is not a
//! normal Number: exact for a normal result, and one more rounding for a subnormal one, which
//! keeps it on one of the two Numbers that enclose the value).
//!
//! The accuracy promised (design, "The accuracy the engine promises"): `exp` is in group A, less
//! than 1 ulp; `expm1`, `sinh`, `cosh` and `tanh` are in group B, at most 2 ulp.
//! `crates/num/tests/math_exp_log.rs` holds both against `data/math/reference-exp-log.txt`.
//!
//! The `f64` operations, and what fixes each result: `+ - * /` (IEEE 754-2019, 5.4.1: correctly
//! rounded), `fmath::floor`, `abs` and `copysign` (exact: 5.9, 5.5.1), `to_bits` and `from_bits`
//! (exact), comparisons and `is_nan`, `is_infinite`, `is_finite`, and `as i32` of an integral
//! `f64` after the range test that the line names (the Rust Reference, "Numeric cast": exact for
//! an integral value in range). No loop depends on the argument.

use super::dd::{self, Dd, pair, two_product};
use crate::fmath;

/// ln 2 in three parts, L1 + L2 + L3: L1 = floor(ln 2 × 2^32) / 2^32 (32 bits), L2 =
/// floor((ln 2 - L1) × 2^64) / 2^64 (32 bits) and L3 the Number nearest to ln 2 - L1 - L2.
pub const LN2_PARTS: [f64; 3] = [
    f64::from_bits(0x3FE6_2E42_FEE0_0000),
    f64::from_bits(0x3DEA_39EF_3560_0000),
    f64::from_bits(0x3BE9_3C76_7300_7E5F),
];

/// ln 2 as a pair: the Number nearest to it and the Number nearest to the rest.
pub const LN2: Dd = pair(0x3FE6_2E42_FEFA_39EF, 0x3C7A_BC9E_3B39_803F);

/// 1/ln 2 as a pair.
pub const INV_LN2: Dd = pair(0x3FF7_1547_652B_82FE, 0x3C77_77D0_FFDA_0D24);

/// 1/n! for n from 2 to 18, each as a pair: the coefficients of e^r - 1 after its term r.
pub const EXP_COEFFICIENTS: [Dd; 17] = [
    pair(0x3FE0_0000_0000_0000, 0x0000_0000_0000_0000),
    pair(0x3FC5_5555_5555_5555, 0x3C65_5555_5555_5555),
    pair(0x3FA5_5555_5555_5555, 0x3C45_5555_5555_5555),
    pair(0x3F81_1111_1111_1111, 0x3C01_1111_1111_1111),
    pair(0x3F56_C16C_16C1_6C17, 0xBBEF_49F4_9F49_F49F),
    pair(0x3F2A_01A0_1A01_A01A, 0x3B6A_01A0_1A01_A01A),
    pair(0x3EFA_01A0_1A01_A01A, 0x3B3A_01A0_1A01_A01A),
    pair(0x3EC7_1DE3_A556_C734, 0xBB6C_154F_8DDC_6C00),
    pair(0x3E92_7E4F_B778_9F5C, 0x3B3C_BBC0_5B4F_A99A),
    pair(0x3E5A_E645_67F5_44E4, 0xBAFC_062E_06D1_F209),
    pair(0x3E21_EED8_EFF8_D898, 0xBAC2_AEC9_59E1_4C06),
    pair(0x3DE6_1246_13A8_6D09, 0x3A8F_28E0_CC74_8EBE),
    pair(0x3DA9_3974_A8C0_7C9D, 0x3A30_5D6F_8A2E_FD1F),
    pair(0x3D6A_E7F3_E733_B81F, 0x39E1_D865_6B0E_E8CB),
    pair(0x3D2A_E7F3_E733_B81F, 0x39A1_D865_6B0E_E8CB),
    pair(0x3CE9_52C7_7030_AD4A, 0x398A_C981_465D_DC6C),
    pair(0x3CA6_8278_63B9_7D97, 0x394E_EC01_221A_8B0B),
];

/// Above this, e^x is beyond the largest Number by more than half an ulp of it (ln of the largest
/// Number is 709.782712893384): +∞. At and below it the reduction is run (k ≤ 1024).
const OVERFLOW: f64 = 709.79;

/// Below this, e^x is below half the smallest subnormal (ln 2^-1075 is -745.1332...): +0.
const UNDERFLOW: f64 = -745.2;

/// Below this, e^x - 1 is -1 within half an ulp of 1 (e^-38.5 is about 1.9 × 10^-17, below
/// 2^-54): `expm1` answers -1.
const EXPM1_FLOOR: f64 = -38.5;

/// 2^-28: below it, `sinh(x)` and `tanh(x)` are x (their next term is x³/6 or x³/3, below 2^-57
/// of x, under half an ulp).
const TINY: f64 = f64::from_bits(0x3E30_0000_0000_0000);

/// From this magnitude on `tanh` is ±1: 1 - tanh(22) is 2/(e^44 + 1), about 1.6 × 10^-19, below
/// half an ulp under 1 (2^-54).
const TANH_ONE: f64 = 22.0;

/// From this magnitude on `sinh` and `cosh` take e^(|x|/2) squared, so that e^|x| itself is not
/// formed; above [`HYPERBOLIC_OVERFLOW`] they overflow.
const HYPERBOLIC_SQUARE: f64 = 709.0;

/// Above this, e^|x| / 2 is beyond the largest Number (ln of twice the largest Number is
/// 710.4758600739439): ±∞.
const HYPERBOLIC_OVERFLOW: f64 = 711.0;

/// An integral `f64` k as `i32`. The range test |k| < 2^11 makes the cast exact; the callers
/// keep k below 1,100 in magnitude (an argument of at most 1,100 / ln 2), and 0 answers a value
/// out of that range, which no caller passes.
pub(crate) fn small_integer(k: f64) -> i32 {
    if k.abs() < 2048.0 {
        // The range test above: |k| < 2^11, and k is integral, so the cast is exact.
        k as i32
    } else {
        0
    }
}

/// e^r - 1 for a pair r with |r| ≤ ln 2 / 2 (a little more is harmless): r + r² × P(r), P the
/// series 1/2! + r/3! + ... + r^16/18! by Horner's rule in double-double, so the leading term r
/// is exact.
pub(crate) fn expm1_kernel(r: Dd) -> Dd {
    let mut p = Dd::from_f64(0.0);
    for coefficient in EXP_COEFFICIENTS.iter().rev() {
        p = p * r + *coefficient;
    }
    r + (r * r) * p
}

/// x - k·ln 2 as a pair, with ln 2 in its three parts; k is integral and below 2^11.
fn remainder(x: Dd, k: f64) -> Dd {
    let [l1, l2, l3] = LN2_PARTS;
    x - Dd::from_f64(k * l1) - Dd::from_f64(k * l2) - two_product(k, l3)
}

/// The reduction x = k·ln 2 + r, |r| ≤ ln 2 / 2, for a pair x with |x| at most 1,100: k from the
/// high part of x × (1/ln 2) plus one half, cut by `floor`, then moved by one where the
/// remainder's high part lies beyond ±(ln 2 / 2).
fn reduce(x: Dd) -> (Dd, i32) {
    let quotient = (x * INV_LN2).hi;
    let mut k = fmath::floor(quotient + 0.5);
    let mut r = remainder(x, k);
    let half = LN2.hi * 0.5;
    if r.hi > half {
        k += 1.0;
        r = remainder(x, k);
    } else if r.hi < -half {
        k -= 1.0;
        r = remainder(x, k);
    }
    (r, small_integer(k))
}

/// e^x = 2^k × p for a pair x with |x| at most 1,100: (p, k), p = e^r in [0.70, 1.42] as a pair.
fn exp_parts(x: Dd) -> (Dd, i32) {
    let (r, k) = reduce(x);
    (Dd::from_f64(1.0) + expm1_kernel(r), k)
}

/// e^x for a pair x, as a pair (the helper of `pow` and `cbrt` in the design's sense: a reduced
/// argument already in double-double). NaN gives NaN, a high part above 709.79 gives +∞ and one
/// below -745.2 gives +0, each with a zero low part. The pair is within 2^-79 of e^x relative
/// (the series is cut at 2^-80 of its result), for results from 2^-960 to the largest Number;
/// below 2^-960 its low part is not a Number of full precision.
pub fn exp_dd(x: Dd) -> Dd {
    if x.hi.is_nan() {
        return Dd::from_f64(f64::NAN);
    }
    if x.hi > OVERFLOW {
        return Dd::from_f64(f64::INFINITY);
    }
    if x.hi < UNDERFLOW {
        return Dd::from_f64(0.0);
    }
    let (p, k) = exp_parts(x);
    p.scale2(k)
}

/// `Math.exp ( x )` (`#sec-math.exp`) after `ToNumber`: NaN and +∞ are returned, ±0 gives 1,
/// -∞ gives +0; the rest is e^x, less than 1 ulp from the exact value.
pub fn exp(x: f64) -> f64 {
    if x.is_nan() {
        return x;
    }
    if x == 0.0 {
        return 1.0;
    }
    if x > OVERFLOW {
        return f64::INFINITY;
    }
    if x < UNDERFLOW {
        return 0.0;
    }
    let (p, k) = exp_parts(Dd::from_f64(x));
    dd::scale(p.to_f64(), k)
}

/// e^x - 1 as a pair for x from -38.5 to 709.79: the kernel alone for |x| below ln 2 / 2, else
/// 2^k × p - 1 in double-double (+∞ where 2^k × p overflows).
fn expm1_dd(x: f64) -> Dd {
    if x.abs() < LN2.hi * 0.5 {
        return expm1_kernel(Dd::from_f64(x));
    }
    let (p, k) = exp_parts(Dd::from_f64(x));
    let power = p.scale2(k);
    if !power.hi.is_finite() {
        return Dd::from_f64(f64::INFINITY);
    }
    power - Dd::from_f64(1.0)
}

/// `Math.expm1 ( x )` (`#sec-math.expm1`) after `ToNumber`: NaN, ±0 and +∞ are returned, -∞
/// gives -1; the rest is e^x - 1, at most 2 ulp from the exact value: -1 below -38.5, +∞ above
/// 709.79.
pub fn expm1(x: f64) -> f64 {
    if x.is_nan() || x == 0.0 || x == f64::INFINITY {
        return x;
    }
    if x < EXPM1_FLOOR {
        return -1.0;
    }
    if x > OVERFLOW {
        return f64::INFINITY;
    }
    expm1_dd(x).to_f64()
}

/// e^a / 2 for a from 709 to 711, as (e^(a/2))² / 2: e^a itself overflows from 709.79 on while
/// e^a / 2 is a Number up to 710.47.
fn half_exp_by_squares(a: f64) -> f64 {
    let (p, k) = exp_parts(Dd::from_f64(a * 0.5));
    dd::scale((p * p).to_f64(), 2 * k - 1)
}

/// `Math.sinh ( x )` (`#sec-math.sinh`) after `ToNumber`: NaN, ±∞ and ±0 are returned; the rest
/// is (e^x - e^-x) / 2, at most 2 ulp from the exact value. With E = e^|x| - 1 in double-double
/// it is (E + E / (E + 1)) / 2, which keeps the relative accuracy near 0; x for |x| below
/// 2^-28; (e^(|x|/2))² / 2 from 709 on; ±∞ above 711. The sign is that of x.
pub fn sinh(x: f64) -> f64 {
    if !x.is_finite() || x == 0.0 {
        return x;
    }
    let a = x.abs();
    if a < TINY {
        return x;
    }
    let magnitude = if a > HYPERBOLIC_OVERFLOW {
        f64::INFINITY
    } else if a > HYPERBOLIC_SQUARE {
        half_exp_by_squares(a)
    } else {
        let e = expm1_dd(a);
        (e + e / (e + Dd::from_f64(1.0))).scale2(-1).to_f64()
    };
    magnitude.copysign(x)
}

/// `Math.cosh ( x )` (`#sec-math.cosh`) after `ToNumber`: NaN is returned, ±∞ gives +∞, ±0
/// gives 1; the rest is (e^x + e^-x) / 2, at most 2 ulp from the exact value: with E = e^|x| in
/// double-double, (E + 1/E) / 2; (e^(|x|/2))² / 2 from 709 on; +∞ above 711.
pub fn cosh(x: f64) -> f64 {
    if x.is_nan() {
        return x;
    }
    let a = x.abs();
    if a == 0.0 {
        return 1.0;
    }
    if a > HYPERBOLIC_OVERFLOW {
        return f64::INFINITY;
    }
    if a > HYPERBOLIC_SQUARE {
        return half_exp_by_squares(a);
    }
    let e = exp_dd(Dd::from_f64(a));
    (e + Dd::from_f64(1.0) / e).scale2(-1).to_f64()
}

/// `Math.tanh ( x )` (`#sec-math.tanh`) after `ToNumber`: NaN and ±0 are returned, ±∞ gives ±1;
/// the rest is (e^2x - 1) / (e^2x + 1), at most 2 ulp from the exact value: with E = e^(2|x|) - 1
/// in double-double, E / (E + 2); x for |x| below 2^-28; ±1 from 22 on. The sign is that of x.
pub fn tanh(x: f64) -> f64 {
    if x.is_nan() || x == 0.0 {
        return x;
    }
    let a = x.abs();
    if a >= TANH_ONE {
        return 1.0_f64.copysign(x);
    }
    if a < TINY {
        return x;
    }
    let e = expm1_dd(2.0 * a);
    (e / (e + Dd::from_f64(2.0))).to_f64().copysign(x)
}
