//! Double-double arithmetic (design, decision 6, "The functions of `Math` that IEEE 754 does not
//! fix", the paragraph "Double-double"): a value held as the unevaluated sum of two Numbers,
//! [`Dd`] `{ hi, lo }`, with `hi + lo` the value and `|lo|` at most half an ulp of `hi`. It
//! carries about 106 bits, and every kernel of [`crate::math`] is written in it.
//!
//! The building blocks are exact: [`two_sum`] (Knuth, six additions) and [`fast_two_sum`]
//! (Dekker, for `|a| ≥ |b|`) give a sum and its rounding error; [`two_product`] (Dekker, with
//! Veltkamp's split by 2^27 + 1) gives a product and its rounding error. The fused
//! multiply-add of `std` is not used: where a target has no fused instruction it calls the
//! platform's library. The operations on pairs are those of Joldes, Muller and Popescu, "Tight
//! and Rigorous Error Bounds for Basic Building Blocks of Double-Word Arithmetic" (2017): the
//! accurate sum of two pairs, the product of two pairs without a fused operation, the product
//! of a pair and a Number; the quotient is that of the high parts corrected once by the
//! residual in double-double, the root one Newton step on `sqrt` of the high part. A pair has
//! the operators `+`, `-`, `*`, `/` and unary `-` (the methods `add`, `sub`, `mul`, `div` and
//! `neg` of `core::ops`), and [`Dd::sqrt`]. The design puts each operation's relative error at
//! most 2^-100; `crates/num/tests/math_dd.rs` measures it against exact arithmetic and holds
//! the measured bound.
//!
//! Each operation is a short fixed sequence of `+ - * /` with no loop. The only branches on a
//! value are the scaling guards of [`two_product`] and of the quotient and, in [`Dd::sqrt`],
//! an operand that is zero, negative or not finite, whose root is that of the high part.
//!
//! What fixes each result on every platform: `+ - * /` are correctly rounded to nearest (IEEE
//! 754-2019, 5.4.1); `crate::math` refuses to compile where `f64` arithmetic carries extended
//! precision, and rustc does not contract `a * b + c` into one fused operation. `fmath::sqrt` is
//! correctly rounded (5.4.1). `abs`, `to_bits` and `from_bits` are exact, and so are the
//! comparisons. A multiplication by a power of two is exact when its result is a normal
//! Number, and is the one rounding of [`Dd::scale2`] otherwise.
//!
//! The range in which the operations hold their bound: the exact result is zero or of
//! magnitude from 2^-960 to the largest Number (below 2^-960 the low part of a product falls
//! under the smallest subnormal and is not a Number), and for a quotient and a root the
//! dividend and the operand are in that range too. Outside it the high part is still the
//! Number the operation's first step computes, and nothing panics.

use core::ops::{Add, Div, Mul, Neg, Sub};

use crate::fmath;

/// 2^27 + 1: Veltkamp's splitter for a 53-bit significand.
const SPLITTER: f64 = 134_217_729.0;

/// 2^996: a factor of at least this magnitude is scaled before it is split, so that
/// `SPLITTER * a` cannot overflow.
const SPLIT_LIMIT: f64 = f64::from_bits(0x7E30_0000_0000_0000);

/// 2^1022: a product of at least this magnitude is computed scaled, so that the product of the
/// high halves of the split, which may each be rounded up, cannot overflow.
const PRODUCT_LIMIT: f64 = f64::from_bits(0x7FD0_0000_0000_0000);

/// 2^-60 and 2^60, the scaling of [`two_product`].
const SCALE_DOWN: f64 = f64::from_bits(0x3C30_0000_0000_0000);
const SCALE_UP: f64 = f64::from_bits(0x43B0_0000_0000_0000);

/// A double-double: the value `hi + lo`, with `|lo|` at most half an ulp of `hi` for a
/// normalized pair (every operation returns one).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Dd {
    pub hi: f64,
    pub lo: f64,
}

/// The pair of the two Numbers with these bits: the form of a constant, whose bits a test
/// recomputes from its definition.
pub const fn pair(hi_bits: u64, lo_bits: u64) -> Dd {
    Dd {
        hi: f64::from_bits(hi_bits),
        lo: f64::from_bits(lo_bits),
    }
}

/// `a + b` and its rounding error (Knuth's TwoSum, six additions): `hi` is the correctly rounded
/// sum and `hi + lo` is `a + b` exactly, for every pair whose sum does not overflow.
pub fn two_sum(a: f64, b: f64) -> Dd {
    let hi = a + b;
    let b_virtual = hi - a;
    let a_virtual = hi - b_virtual;
    let lo = (a - a_virtual) + (b - b_virtual);
    Dd { hi, lo }
}

/// `a + b` and its rounding error for `|a| ≥ |b|` (or `a` zero), in three additions (Dekker's
/// Fast2Sum): exact as [`two_sum`] is.
pub fn fast_two_sum(a: f64, b: f64) -> Dd {
    let hi = a + b;
    let lo = b - (hi - a);
    Dd { hi, lo }
}

/// Veltkamp's split: two halves of at most 26 and 27 significant bits whose sum is `a`, for
/// `|a|` below 2^996.
fn split(a: f64) -> (f64, f64) {
    let c = SPLITTER * a;
    let high = c - (c - a);
    (high, a - high)
}

/// The rounding error of `p = a * b` by Dekker's product of the halves.
fn product_error(a: f64, b: f64, p: f64) -> f64 {
    let (a_high, a_low) = split(a);
    let (b_high, b_low) = split(b);
    ((a_high * b_high - p) + a_high * b_low + a_low * b_high) + a_low * b_low
}

/// `a * b` and its rounding error (Dekker's TwoProduct with Veltkamp's split): `hi` is the
/// correctly rounded product, and `hi + lo` is `a * b` exactly for a product that is zero or of
/// magnitude at least 2^-960 and that does not overflow.
///
/// The scaling guard: when a factor has magnitude 2^996 or more, or the product 2^1022 or more,
/// the larger factor is multiplied by 2^-60 before the split and both parts of the result by
/// 2^60 after it. Each of those is exact: the larger factor is then at least 2^511, the scaled
/// product at least 2^-138. Scaling the larger factor alone keeps a subnormal factor whole.
/// A product that is not finite is returned with a zero low part.
pub fn two_product(a: f64, b: f64) -> Dd {
    let p = a * b;
    if !p.is_finite() {
        return Dd { hi: p, lo: 0.0 };
    }
    let (large, small) = if a.abs() >= b.abs() { (a, b) } else { (b, a) };
    if large.abs() >= SPLIT_LIMIT || p.abs() >= PRODUCT_LIMIT {
        let scaled = large * SCALE_DOWN;
        let q = scaled * small;
        let error = product_error(scaled, small, q);
        return Dd {
            hi: q * SCALE_UP,
            lo: error * SCALE_UP,
        };
    }
    Dd {
        hi: p,
        lo: product_error(a, b, p),
    }
}

/// 2^k as a Number, for k from -1074 to 1023.
fn power_of_two(k: i32) -> f64 {
    let k = k.clamp(-1074, 1023);
    if k >= -1022 {
        // k + 1023 is from 1 to 2046: the biased exponent of a normal Number.
        let biased = u64::try_from(k + 1023).unwrap_or(1);
        f64::from_bits(biased << 52)
    } else {
        // k + 1074 is from 0 to 51: the one bit of a subnormal Number.
        let bit = u32::try_from(k + 1074).unwrap_or(0);
        f64::from_bits(1u64 << bit)
    }
}

/// `x` times 2^k with at most two factors, each a Number, so that no intermediate overflows or
/// flushes, and the result rounded once: for k from -1074 to 1023 one factor; above 1023 the
/// factors 2^1023 and 2^(k - 1023), the first exact unless the result overflows anyway; below
/// -1074 the factors 2^(k + 1074) and 2^-1074, the first exact whenever the result is not zero.
/// k is taken in [-2148, 2046]: below, every finite x gives zero, as the exact product rounds;
/// above, the result is exact for |x| at least 2^-1022 (it overflows either way) and not for a
/// subnormal x.
pub(crate) fn scale(x: f64, k: i32) -> f64 {
    let k = k.clamp(-2148, 2046);
    if k > 1023 {
        x * power_of_two(1023) * power_of_two(k - 1023)
    } else if k < -1074 {
        x * power_of_two(k + 1074) * power_of_two(-1074)
    } else {
        x * power_of_two(k)
    }
}

impl Dd {
    /// The pair of one Number, `(x, 0)`.
    pub const fn from_f64(x: f64) -> Dd {
        Dd { hi: x, lo: 0.0 }
    }

    /// The square root: `x = sqrt(hi)`, then one Newton step `x + (self - x²) / (2x)`, with `x²`
    /// exact by [`two_product`] and the difference in double-double. A zero, a negative or a
    /// non-finite high part gives `(sqrt(hi), 0)`: ±0, NaN or +∞.
    pub fn sqrt(self) -> Dd {
        let x = fmath::sqrt(self.hi);
        if self.hi <= 0.0 || !self.hi.is_finite() {
            return Dd { hi: x, lo: 0.0 };
        }
        let residual = sum(self, negated(two_product(x, x)));
        fast_two_sum(x, residual.hi / (2.0 * x))
    }

    /// `self` times 2^k, each part by the scaling described at [`scale`] (rounded once each).
    pub fn scale2(self, k: i32) -> Dd {
        Dd {
            hi: scale(self.hi, k),
            lo: scale(self.lo, k),
        }
    }

    /// The Number nearest to the pair: `hi + lo`, the one rounding.
    pub fn to_f64(self) -> f64 {
        self.hi + self.lo
    }
}

/// `a + b`: the accurate sum of two pairs (Joldes, Muller and Popescu, Algorithm 6), two
/// [`two_sum`]s of the parts and two renormalizations; its relative error holds under
/// cancellation too.
fn sum(a: Dd, b: Dd) -> Dd {
    let s = two_sum(a.hi, b.hi);
    let t = two_sum(a.lo, b.lo);
    let c = s.lo + t.hi;
    let v = fast_two_sum(s.hi, c);
    let w = t.lo + v.lo;
    fast_two_sum(v.hi, w)
}

/// `-a`, exact.
fn negated(a: Dd) -> Dd {
    Dd {
        hi: -a.hi,
        lo: -a.lo,
    }
}

/// `a * b`: the exact product of the high parts by [`two_product`], the two cross products
/// added to its low part, renormalized (Joldes, Muller and Popescu, Algorithm 10, the form
/// without a fused operation).
fn product(a: Dd, b: Dd) -> Dd {
    let c = two_product(a.hi, b.hi);
    let cross = a.hi * b.lo + a.lo * b.hi;
    fast_two_sum(c.hi, c.lo + cross)
}

/// `a * b` for a Number `b` (Joldes, Muller and Popescu, Algorithm 7).
fn product_f64(a: Dd, b: f64) -> Dd {
    let c = two_product(a.hi, b);
    let t = fast_two_sum(c.hi, a.lo * b);
    fast_two_sum(t.hi, t.lo + c.lo)
}

/// `a / b`: the quotient of the high parts, then one correction by the residual `a - b × q`,
/// computed in double-double and divided by the high part of `b`.
///
/// The scaling guard: a dividend of magnitude 2^1022 or more is divided as a quarter of itself
/// and both parts of the quotient are multiplied by 4 (each exact, the result being at most the
/// largest Number), since `b × q` exceeds the dividend by up to an ulp of it and would overflow
/// next to the largest Number (`f64::MAX / 1.5`).
fn quotient(a: Dd, b: Dd) -> Dd {
    let (dividend, factor) = if a.hi.abs() >= PRODUCT_LIMIT {
        (
            Dd {
                hi: a.hi * 0.25,
                lo: a.lo * 0.25,
            },
            4.0,
        )
    } else {
        (a, 1.0)
    };
    let q = dividend.hi / b.hi;
    let residual = sum(dividend, negated(product_f64(b, q)));
    let corrected = fast_two_sum(q, residual.hi / b.hi);
    Dd {
        hi: corrected.hi * factor,
        lo: corrected.lo * factor,
    }
}

/// The sum of two pairs, [`sum`].
impl Add for Dd {
    type Output = Dd;

    fn add(self, other: Dd) -> Dd {
        sum(self, other)
    }
}

/// The difference of two pairs: the sum with the negated pair.
impl Sub for Dd {
    type Output = Dd;

    fn sub(self, other: Dd) -> Dd {
        sum(self, negated(other))
    }
}

/// The negated pair, exact.
impl Neg for Dd {
    type Output = Dd;

    fn neg(self) -> Dd {
        negated(self)
    }
}

/// The product of two pairs, [`product`].
impl Mul for Dd {
    type Output = Dd;

    fn mul(self, other: Dd) -> Dd {
        product(self, other)
    }
}

/// The quotient of two pairs, [`quotient`].
impl Div for Dd {
    type Output = Dd;

    fn div(self, other: Dd) -> Dd {
        quotient(self, other)
    }
}
