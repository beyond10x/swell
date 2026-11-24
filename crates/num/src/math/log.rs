//! The logarithmic family of `Math` (design, decision 6, "The functions of `Math` that IEEE 754
//! does not fix", items 3 and 5): [`log`] (`#sec-math.log`), [`log2`] (`#sec-math.log2`),
//! [`log10`] (`#sec-math.log10`), [`log1p`] (`#sec-math.log1p`), [`asinh`] (`#sec-math.asinh`),
//! [`acosh`] (`#sec-math.acosh`) and [`atanh`] (`#sec-math.atanh`), and the helper [`log_dd`],
//! each over a Number the caller has already coerced.
//!
//! The reduction: x = 2^k × m with m in [√2/2, √2), read from the bits (`to_bits` and
//! `from_bits`; a subnormal x is scaled by 2^54 first, which is exact); a mantissa of
//! [`SQRT2_UPPER`] or more is halved. f = m - 1 is exact (m is within a factor 2 of 1), s =
//! f / (2 + f) in double-double, and log m = 2·atanh(s) = 2·(s + s³/3 + s^5/5 + ... + s^29/29)
//! (|s| ≤ 3 - 2√2 = 0.1716; the degree of the design's cut: the first omitted term, s^31/31, is
//! below 2^-80 of the result, and s^29/29 is not), by Horner's rule in s² with each coefficient
//! 1/(2j + 1) a pair ([`LOG_COEFFICIENTS`]) and the leading term s kept apart. The result is
//! k·ln 2 + log m in double-double, k·ln 2 from the three parts of ln 2 (k·L1 and k·L2 exact,
//! k·L3 by [`two_product`]). `log2` and `log10` multiply that pair by 1/ln 2 and 1/ln 10 as
//! pairs: the pair is within 2^-79 of its value relative (the series is cut at 2^-80), and for a
//! power of two, which takes no series, within about 2^-100, so `log2(2^k)` is k for every k from
//! -1074 to 1023 and `log10(10^n)` is n for n from 0 to 22. `log1p(x)` takes f = x where 1 + x
//! lies in [√2/2, √2), and otherwise reduces the exact pair 1 + x ([`two_sum`]) the same way, its
//! low part scaled with it.
//!
//! The inverse hyperbolic functions (item 5) are identities over that kernel and `sqrt` in
//! double-double: `asinh(x) = sign × log1p(|x| + x² / (1 + sqrt(1 + x²)))`, x itself for |x|
//! below 2^-28 and log|x| + ln 2 above 2^28; `acosh(x) = log1p(t + sqrt(2t + t²))` with t = x - 1
//! exact for x below 2, log(x + sqrt(x² - 1)) up to 2^28 and log(x) + ln 2 above;
//! `atanh(x) = sign × log1p(2|x| / (1 - |x|)) / 2`, x itself for |x| below 2^-28.
//!
//! The accuracy promised (design, "The accuracy the engine promises"): `log`, `log2` and `log10`
//! are in group A, less than 1 ulp; `log1p`, `asinh`, `acosh` and `atanh` in group B, at most 2
//! ulp. `crates/num/tests/math_exp_log.rs` holds both against `data/math/reference-exp-log.txt`.
//!
//! The `f64` operations, and what fixes each result: `+ - * /` and `sqrt` (IEEE 754-2019, 5.4.1:
//! correctly rounded), `abs` and `copysign` (exact: 5.5.1), `to_bits` and `from_bits` (exact),
//! comparisons and `is_nan`, `is_finite`, and `f64::from` of an `i32` (exact: every `i32` is a
//! Number). No loop depends on the argument.

use super::dd::{self, Dd, pair, two_product, two_sum};
use super::exp::{LN2, LN2_PARTS};

/// 1/ln 10 as a pair: the Number nearest to it and the Number nearest to the rest.
pub const INV_LN10: Dd = pair(0x3FDB_CB7B_1526_E50E, 0x3C69_5355_BAAA_FAD3);

/// 1/(2j + 1) for j from 1 to 14 (1/3 to 1/29), each as a pair: the coefficients of
/// atanh(s) / s = 1 + s²/3 + s^4/5 + ... + s^28/29.
pub const LOG_COEFFICIENTS: [Dd; 14] = [
    pair(0x3FD5_5555_5555_5555, 0x3C75_5555_5555_5555),
    pair(0x3FC9_9999_9999_999A, 0xBC69_9999_9999_999A),
    pair(0x3FC2_4924_9249_2492, 0x3C62_4924_9249_2492),
    pair(0x3FBC_71C7_1C71_C71C, 0x3C5C_71C7_1C71_C71C),
    pair(0x3FB7_45D1_745D_1746, 0xBC47_45D1_745D_1746),
    pair(0x3FB3_B13B_13B1_3B14, 0xBC53_B13B_13B1_3B14),
    pair(0x3FB1_1111_1111_1111, 0x3C31_1111_1111_1111),
    pair(0x3FAE_1E1E_1E1E_1E1E, 0x3C2E_1E1E_1E1E_1E1E),
    pair(0x3FAA_F286_BCA1_AF28, 0x3C4A_F286_BCA1_AF28),
    pair(0x3FA8_6186_1861_8618, 0x3C48_6186_1861_8618),
    pair(0x3FA6_42C8_590B_2164, 0x3C36_42C8_590B_2164),
    pair(0x3FA4_7AE1_47AE_147B, 0xBC2E_B851_EB85_1EB8),
    pair(0x3FA2_F684_BDA1_2F68, 0x3C42_F684_BDA1_2F68),
    pair(0x3FA1_A7B9_611A_7B96, 0x3C21_A7B9_611A_7B96),
];

/// The least Number above √2 (1.4142135623730951): a mantissa of at least this is halved, so the
/// reduced m lies in [SQRT2_UPPER / 2, SQRT2_UPPER), within (√2/2, √2).
pub const SQRT2_UPPER: f64 = f64::from_bits(0x3FF6_A09E_667F_3BCD);

/// 2^54, the scaling of a subnormal argument.
const TWO_54: f64 = f64::from_bits(0x4350_0000_0000_0000);

/// 2^-28 and 2^28, the bounds of the short forms of `asinh`, `acosh` and `atanh`.
const TINY: f64 = f64::from_bits(0x3E30_0000_0000_0000);
const HUGE: f64 = f64::from_bits(0x41B0_0000_0000_0000);

/// 2^-54: below it, `log1p(x)` is x (the next term, x²/2, is below 2^-55 of x, under half an
/// ulp), and the quotient of the kernel would leave the range of double-double.
const LOG1P_TINY: f64 = f64::from_bits(0x3C90_0000_0000_0000);

/// The stored bits of a Number and the bits of 1.0.
const MANTISSA: u64 = 0x000F_FFFF_FFFF_FFFF;
const ONE_BITS: u64 = 0x3FF0_0000_0000_0000;

/// k·ln 2 as a pair, k from -1130 to 1030: k·L1 and k·L2 are exact (32-bit parts, k below
/// 2^11), k·L3 is exact by [`two_product`].
fn k_ln2(k: i32) -> Dd {
    let k = f64::from(k);
    let [l1, l2, l3] = LN2_PARTS;
    (Dd::from_f64(k * l1) + Dd::from_f64(k * l2)) + two_product(k, l3)
}

/// log(1 + f) for a pair f with 1 + f in [√2/2, √2]: s = f / (2 + f), then
/// 2·(s + s × z × Q(z)) with z = s² and Q(z) = 1/3 + z/5 + ... + z^13/29.
fn log1p_kernel(f: Dd) -> Dd {
    let s = f / (Dd::from_f64(2.0) + f);
    let z = s * s;
    let mut q = Dd::from_f64(0.0);
    for coefficient in LOG_COEFFICIENTS.iter().rev() {
        q = q * z + *coefficient;
    }
    (s + (s * z) * q).scale2(1)
}

/// The natural logarithm of a pair u with a positive normal finite high part, plus `extra`·ln 2:
/// u = 2^k × (m + rest) with m from the bits of the high part, halved where it is at least
/// [`SQRT2_UPPER`]; f = (m - 1) + rest as a pair (m - 1 exact).
fn log_reduced(u: Dd, extra: i32) -> Dd {
    let bits = u.hi.to_bits();
    // A positive normal Number: its biased exponent is from 1 to 2046.
    let biased = i32::try_from(bits >> 52).unwrap_or(1023);
    let mut k = biased - 1023;
    let mut m = f64::from_bits((bits & MANTISSA) | ONE_BITS);
    if m >= SQRT2_UPPER {
        m *= 0.5;
        k += 1;
    }
    let rest = dd::scale(u.lo, -k);
    let f = two_sum(m - 1.0, rest);
    k_ln2(k + extra) + log1p_kernel(f)
}

/// The natural logarithm of a positive finite Number x as a pair (the helper of `pow` and
/// `cbrt`), with a relative error of at most 2^-79 (the series is cut at 2^-80 of its result): a
/// subnormal x is scaled by 2^54 first. NaN and a negative x give NaN, ±0 gives -∞ and +∞ gives
/// +∞, each with a zero low part.
pub fn log_dd(x: f64) -> Dd {
    if x.is_nan() || x < 0.0 {
        return Dd::from_f64(f64::NAN);
    }
    if x == 0.0 {
        return Dd::from_f64(f64::NEG_INFINITY);
    }
    if x == f64::INFINITY {
        return Dd::from_f64(x);
    }
    if x < f64::MIN_POSITIVE {
        log_reduced(Dd::from_f64(x * TWO_54), -54)
    } else {
        log_reduced(Dd::from_f64(x), 0)
    }
}

/// The special values that `log`, `log2` and `log10` share (`#sec-math.log` and its siblings):
/// NaN and +∞ are returned, 1 gives +0, ±0 gives -∞, a negative x gives NaN.
fn log_special(x: f64) -> Option<f64> {
    if x.is_nan() || x == f64::INFINITY {
        Some(x)
    } else if x == 1.0 {
        Some(0.0)
    } else if x == 0.0 {
        Some(f64::NEG_INFINITY)
    } else if x < 0.0 {
        Some(f64::NAN)
    } else {
        None
    }
}

/// `Math.log ( x )` (`#sec-math.log`) after `ToNumber`: the special values of the draft, then
/// ln x, less than 1 ulp from the exact value.
pub fn log(x: f64) -> f64 {
    match log_special(x) {
        Some(answer) => answer,
        None => log_dd(x).to_f64(),
    }
}

/// `Math.log2 ( x )` (`#sec-math.log2`) after `ToNumber`: the special values of the draft, then
/// ln x × (1/ln 2) in double-double, less than 1 ulp from the exact value and exact for every
/// power of two.
pub fn log2(x: f64) -> f64 {
    match log_special(x) {
        Some(answer) => answer,
        None => (log_dd(x) * super::exp::INV_LN2).to_f64(),
    }
}

/// `Math.log10 ( x )` (`#sec-math.log10`) after `ToNumber`: the special values of the draft, then
/// ln x × (1/ln 10) in double-double, less than 1 ulp from the exact value and exact for 10^n,
/// n from 0 to 22.
pub fn log10(x: f64) -> f64 {
    match log_special(x) {
        Some(answer) => answer,
        None => (log_dd(x) * INV_LN10).to_f64(),
    }
}

/// log(1 + t) for a pair t above -1: the kernel alone where 1 + t lies in [√2/2, √2), else the
/// reduction of the pair 1 + t.
fn log1p_pair(t: Dd) -> Dd {
    if t.hi >= SQRT2_UPPER * 0.5 - 1.0 && t.hi < SQRT2_UPPER - 1.0 {
        log1p_kernel(t)
    } else {
        log_reduced(Dd::from_f64(1.0) + t, 0)
    }
}

/// `Math.log1p ( x )` (`#sec-math.log1p`) after `ToNumber`: NaN, ±0 and +∞ are returned, -1
/// gives -∞, below -1 NaN; the rest is ln(1 + x), at most 2 ulp from the exact value: x itself
/// below 2^-54 in magnitude, else with 1 + x held exactly as the pair of [`two_sum`].
pub fn log1p(x: f64) -> f64 {
    if x.is_nan() || x == 0.0 || x == f64::INFINITY {
        return x;
    }
    if x == -1.0 {
        return f64::NEG_INFINITY;
    }
    if x < -1.0 {
        return f64::NAN;
    }
    if x.abs() < LOG1P_TINY {
        return x;
    }
    log1p_pair(Dd::from_f64(x)).to_f64()
}

/// `Math.asinh ( x )` (`#sec-math.asinh`) after `ToNumber`: NaN, ±∞ and ±0 are returned; the rest
/// is the inverse hyperbolic sine, at most 2 ulp from the exact value: x below 2^-28 in
/// magnitude, log1p(|x| + x² / (1 + sqrt(1 + x²))) up to 2^28 (x² exact by [`two_product`]) and
/// log|x| + ln 2 above it (the omitted 1/(4x²) is below 2^-58), with the sign of x.
pub fn asinh(x: f64) -> f64 {
    if !x.is_finite() || x == 0.0 {
        return x;
    }
    let a = x.abs();
    if a < TINY {
        return x;
    }
    let magnitude = if a > HUGE {
        (log_dd(a) + LN2).to_f64()
    } else {
        let one = Dd::from_f64(1.0);
        let square = two_product(a, a);
        let root = (one + square).sqrt();
        log1p_pair(Dd::from_f64(a) + square / (one + root)).to_f64()
    };
    magnitude.copysign(x)
}

/// `Math.acosh ( x )` (`#sec-math.acosh`) after `ToNumber`: NaN and +∞ are returned, 1 gives +0,
/// below 1 NaN; the rest is the inverse hyperbolic cosine, at most 2 ulp from the exact value:
/// log1p(t + sqrt(2t + t²)) with t = x - 1 (exact) below 2, log(x + sqrt(x² - 1)) up to 2^28,
/// and log x + ln 2 above it (the omitted 1/(4x²) is below 2^-58).
pub fn acosh(x: f64) -> f64 {
    if x.is_nan() || x == f64::INFINITY {
        return x;
    }
    if x == 1.0 {
        return 0.0;
    }
    if x < 1.0 {
        return f64::NAN;
    }
    if x < 2.0 {
        let t = x - 1.0;
        let inner = two_product(t, t) + Dd::from_f64(2.0 * t);
        return log1p_pair(Dd::from_f64(t) + inner.sqrt()).to_f64();
    }
    if x <= HUGE {
        let square = two_product(x, x);
        let root = (square - Dd::from_f64(1.0)).sqrt();
        return log_reduced(Dd::from_f64(x) + root, 0).to_f64();
    }
    (log_dd(x) + LN2).to_f64()
}

/// `Math.atanh ( x )` (`#sec-math.atanh`) after `ToNumber`: NaN and ±0 are returned, beyond ±1
/// NaN, ±1 gives ±∞; the rest is the inverse hyperbolic tangent, at most 2 ulp from the exact
/// value: x below 2^-28 in magnitude, else log1p(2|x| / (1 - |x|)) / 2 with 1 - |x| as the exact
/// pair of [`two_sum`], with the sign of x.
pub fn atanh(x: f64) -> f64 {
    if x.is_nan() || x == 0.0 {
        return x;
    }
    let a = x.abs();
    if a > 1.0 {
        return f64::NAN;
    }
    if a == 1.0 {
        return f64::INFINITY.copysign(x);
    }
    if a < TINY {
        return x;
    }
    let quotient = Dd::from_f64(2.0 * a) / two_sum(1.0, -a);
    log1p_pair(quotient).scale2(-1).to_f64().copysign(x)
}
