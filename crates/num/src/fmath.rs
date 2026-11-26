//! The `f64` functions that `std` has and `core` does not, computed here in integer arithmetic on
//! the bits of the operands (`story:core-builds-without-std`): [`sqrt`], [`floor`], [`ceil`],
//! [`trunc`], [`fmod`] (the remainder the operator `%` of two `f64` gives, which the compiler
//! otherwise lowers to a call into the C math library) and [`rem_euclid`]. Each is named and
//! behaves like the `std` method of the name (`fmod(x, y)` like `x % y`), and each result is the
//! one IEEE 754-2019 fixes: `sqrt` is correctly rounded to nearest (5.4.1), `floor`, `ceil` and
//! `trunc` are `roundToIntegralTowardNegative`, `roundToIntegralTowardPositive` and
//! `roundToIntegralTowardZero` (5.9), and `fmod` is exact. So a result is the same bits on every
//! platform. A NaN operand is answered with itself; an invalid operation (the square root of a
//! negative Number, `fmod` with a zero divisor or an infinite dividend) with [`f64::NAN`].
//!
//! The transcendental functions are not here: they are `swell_num::math` (design, decision 6).
//!
//! The algorithms are those of fdlibm 5.3 as the msun library of FreeBSD carries them, ported to
//! safe Rust with the two 32-bit words of a double held as one `u64`: the root bit by bit of
//! `e_sqrt.c` (the lib tests' oracle of [`sqrt`], which corrects a fast estimate in integers);
//! `floor`, `ceil` and `trunc` from `s_floor.c`, `s_ceil.c` and `s_trunc.c` (the fraction bits
//! cleared, one unit added toward the rounding direction first); and `fmod` from `e_fmod.c`
//! (shift and subtract, fixed point), after an `i64` path for two integers below 2^53. The
//! parts of the C code that raise the inexact flag of the floating-point environment are left
//! out: Rust reads no flag. The notices those files carry, which their terms ask to keep:
//!
//! ```text
//! e_sqrt.c and e_fmod.c:
//! ====================================================
//! Copyright (C) 1993 by Sun Microsystems, Inc. All rights reserved.
//!
//! Developed at SunSoft, a Sun Microsystems, Inc. business.
//! Permission to use, copy, modify, and distribute this
//! software is freely granted, provided that this notice
//! is preserved.
//! ====================================================
//!
//! s_floor.c, s_ceil.c and s_trunc.c:
//! ====================================================
//! Copyright (C) 1993 by Sun Microsystems, Inc. All rights reserved.
//!
//! Developed at SunPro, a Sun Microsystems, Inc. business.
//! Permission to use, copy, modify, and distribute this
//! software is freely granted, provided that this notice
//! is preserved.
//! ====================================================
//! ```
//!
//! `NOTICE` at the root of the repository lists them as third-party material.

/// The sign bit of a double.
const SIGN: u64 = 1 << 63;
/// The 52 fraction bits of a double.
const FRACTION: u64 = (1 << 52) - 1;
/// The implicit leading bit of a normal double's significand.
const IMPLICIT: u64 = 1 << 52;

/// The unbiased exponent of `bits`: -1023 for a zero or a subnormal, 1024 for an infinity or a
/// NaN.
fn exponent(bits: u64) -> i32 {
    // The biased exponent has 11 bits, so it fits an `i32`.
    ((bits >> 52) & 0x7ff) as i32 - 1023
}

/// The significand and exponent of a finite non-zero |x| given as `bits` without the sign: `(m,
/// e)` with 2^52 ≤ m < 2^53 and |x| = m × 2^(e - 52). A subnormal is shifted up until its leading
/// bit is the implicit one.
fn normalized(magnitude: u64) -> (u64, i32) {
    let biased = magnitude >> 52;
    if biased == 0 {
        // At least 12 leading zeros: the shift is from 1 to 52.
        let shift = magnitude.leading_zeros() - 11;
        (magnitude << shift, -1022 - shift as i32)
    } else {
        ((magnitude & FRACTION) | IMPLICIT, biased as i32 - 1023)
    }
}

/// The square root of `x`, correctly rounded to nearest (IEEE 754-2019, 5.4.1; `f64::sqrt`):
/// ±0 for ±0, +∞ for +∞, NaN for a NaN (x itself) and [`f64::NAN`] for every x below zero.
///
/// x = m × 2^(e - 52) with e made even (m doubled when e is odd), so that sqrt(x) =
/// sqrt(t) × 2^(e/2 - 52) with t = m × 2^52 and sqrt(t) from 2^52 to 2^53: the result's
/// significand is the integer q nearest to sqrt(t). An estimate of q comes from `f64` arithmetic
/// alone (a first reciprocal square root from the bits, three Newton steps, one Heron step); it
/// is within a few units of q, and it is then corrected in `u128` integers until
/// (2q - 1)² < 4t < (2q + 1)², which is exactly "q is the nearest" (the root of an integer that
/// is not a square is irrational, so it never lies on a half). The estimate decides only how
/// many corrections run, never the result, and no instruction of a processor is asked for a
/// root: the crate forbids `unsafe`, which the processors' square-root intrinsics need on the
/// pinned compiler (1.99.1: calling them is unsafe even where SSE2 is in the target's
/// baseline), and `libm-check` refuses `core::arch`. About 4 times faster than the bit-by-bit
/// root of `e_sqrt.c`, which the lib tests keep as the oracle of this one.
pub fn sqrt(x: f64) -> f64 {
    let bits = x.to_bits();
    if x.is_nan() || x == 0.0 || bits == f64::INFINITY.to_bits() {
        return x;
    }
    if bits & SIGN != 0 {
        return f64::NAN;
    }
    let (mut m, mut e) = normalized(bits);
    if e & 1 != 0 {
        m += m;
    }
    e >>= 1;
    // t = m × 2^52 < 2^106, and 4t and (2q + 1)² for q ≤ 2^53 stay below 2^128.
    let four_t = u128::from(m) << 54;
    // The estimate: m below 2^54 and even when it has 54 bits, so m as f64 is exact.
    let v = (m as f64) * TWO_52;
    let mut r = f64::from_bits(0x5fe6_eb50_c7b5_37a9 - (v.to_bits() >> 1));
    for _ in 0..3 {
        r *= 1.5 - 0.5 * v * r * r;
    }
    let y = v * r;
    let y = 0.5 * (y + v / y);
    // Clamped to [2^52, 2^53], where both corrections below stop: (2^53 - 1)² < 2^106 ≤ 4t and
    // (2^54 + 1)² > 2^108 > 4t.
    let mut q = ((y + 0.5) as u64).clamp(IMPLICIT, IMPLICIT << 1);
    // The estimate is within one of q (measured over the lib tests' inputs): one correction each
    // way without a branch, then the loops, which then run no pass and cost no mispredicted
    // branch.
    q += u64::from(square(2 * q + 1) <= four_t);
    q -= u64::from(square(2 * q - 1) > four_t);
    while square(2 * q + 1) <= four_t {
        q += 1;
    }
    while square(2 * q - 1) > four_t {
        q -= 1;
    }
    // q × 2^(e - 52): the biased exponent e + 1023 is added one below, so that the leading bit of
    // q (2^52, or 2^53 after rounding up, which moves the exponent) adds the last one.
    f64::from_bits((((i64::from(e) + 1022) as u64) << 52).wrapping_add(q))
}

/// 2^52.
const TWO_52: f64 = 4_503_599_627_370_496.0;
/// 2^53.
const TWO_53: f64 = 9_007_199_254_740_992.0;

/// n², for n below 2^55.
fn square(n: u64) -> u128 {
    u128::from(n) * u128::from(n)
}

/// The bit-by-bit square root of `e_sqrt.c`, the oracle of [`sqrt`] in the lib tests: ±0 for
/// ±0, +∞ for +∞, x for a NaN and [`f64::NAN`] for every x below zero.
///
/// x = m × 2^(e - 52) with e made even (m doubled when it is odd), so that
/// sqrt(x) = sqrt(m × 2^-52) × 2^(e/2); the 54 bits of q = floor(sqrt(2m × 2^54) / 2^27), one
/// more than the result has, are made one at a time from the top, each the larger bit for which
/// q² stays at or below the operand, with the running remainder kept exactly. The last bit
/// rounds: with a remainder left the exact root lies strictly between two 54-bit values, never
/// on a half of the 53-bit ones, so adding the last bit to q before it is dropped rounds to
/// nearest.
#[cfg(test)]
fn sqrt_bit_by_bit(x: f64) -> f64 {
    let bits = x.to_bits();
    if x.is_nan() || x == 0.0 || bits == f64::INFINITY.to_bits() {
        return x;
    }
    if bits & SIGN != 0 {
        return f64::NAN;
    }
    let (mut ix, mut m) = normalized(bits);
    if m & 1 != 0 {
        ix += ix;
    }
    m >>= 1;
    // Generate sqrt(x) bit by bit; s is twice the root made so far, at the scale of the remainder.
    ix += ix;
    let (mut q, mut s) = (0u64, 0u64);
    let mut r = 1u64 << 53;
    while r != 0 {
        let t = s + r;
        if t <= ix {
            s = t + r;
            ix -= t;
            q += r;
        }
        ix += ix;
        r >>= 1;
    }
    // Round to nearest: the remainder is not zero only when the root is inexact.
    if ix != 0 {
        q += q & 1;
    }
    // q / 2 has its leading bit at 2^52 (2^53 after a carry, which moves the exponent): the
    // biased exponent 1023 + m is added one below, as e_sqrt.c adds 0x3fe00000.
    let biased = (0x3fe + i64::from(m)) as u64;
    f64::from_bits((q >> 1).wrapping_add(biased << 52))
}

/// The greatest integer at or below `x` (`roundToIntegralTowardNegative`; `f64::floor`). A NaN,
/// an infinity, a zero and every |x| ≥ 2^52 answer x; -1 for -1 < x < 0, +0 for 0 < x < 1.
pub fn floor(x: f64) -> f64 {
    let bits = x.to_bits();
    let j0 = exponent(bits);
    if j0 > 51 {
        return x;
    }
    if j0 < 0 {
        return if bits & SIGN == 0 {
            0.0
        } else if bits & !SIGN != 0 {
            -1.0
        } else {
            x
        };
    }
    let fraction = FRACTION >> j0;
    if bits & fraction == 0 {
        return x;
    }
    // A negative x moves one unit down before the fraction is cleared; a carry out of the
    // significand moves the exponent up, as the value needs.
    let up = if bits & SIGN != 0 { IMPLICIT >> j0 } else { 0 };
    f64::from_bits((bits + up) & !fraction)
}

/// The least integer at or above `x` (`roundToIntegralTowardPositive`; `f64::ceil`). A NaN, an
/// infinity, a zero and every |x| ≥ 2^52 answer x; -0 for -1 < x < 0, +1 for 0 < x < 1.
pub fn ceil(x: f64) -> f64 {
    let bits = x.to_bits();
    let j0 = exponent(bits);
    if j0 > 51 {
        return x;
    }
    if j0 < 0 {
        return if bits & SIGN != 0 {
            -0.0
        } else if bits != 0 {
            1.0
        } else {
            x
        };
    }
    let fraction = FRACTION >> j0;
    if bits & fraction == 0 {
        return x;
    }
    let up = if bits & SIGN == 0 { IMPLICIT >> j0 } else { 0 };
    f64::from_bits((bits + up) & !fraction)
}

/// The integer toward zero from `x` (`roundToIntegralTowardZero`; `f64::trunc`), with the sign
/// of x: ±0 for -1 < x < 1. A NaN, an infinity and every |x| ≥ 2^52 answer x.
pub fn trunc(x: f64) -> f64 {
    let bits = x.to_bits();
    let j0 = exponent(bits);
    if j0 > 51 {
        return x;
    }
    if j0 < 0 {
        return f64::from_bits(bits & SIGN);
    }
    f64::from_bits(bits & !(FRACTION >> j0))
}

/// The remainder of `x` divided by `y` with the sign of x, x - n × y for the integer n toward
/// zero from x / y, exactly (the `%` of two `f64`, C's `fmod`). NaN when either is NaN, when y
/// is ±0 and when x is infinite; x itself when |x| < |y| or y is infinite; ±0 with the sign of
/// x when y divides x.
///
/// `e_fmod.c`: both significands are made 53-bit integers mx and my with exponents ix ≥ iy;
/// then ix - iy times my is taken from mx when it fits and mx is doubled, which leaves
/// mx × 2^iy ≡ x modulo y with mx < 2my; one last subtraction and a shift back to a normal (or
/// a subnormal) significand give the result, which is always a double.
///
/// Before it, the common case of a script: x and y integers of magnitude below 2^53 and y not
/// zero. Both are exact as `i64`, and the `i64` remainder, which has the sign of the dividend,
/// is the result, exact as `f64`; a zero remainder is +0 or -0 with the sign of x.
pub fn fmod(x: f64, y: f64) -> f64 {
    if x.abs() < TWO_53 && y.abs() < TWO_53 && y != 0.0 {
        let (a, b) = (x as i64, y as i64);
        if a as f64 == x && b as f64 == y {
            let r = a % b;
            return if r == 0 {
                f64::from_bits(x.to_bits() & SIGN)
            } else {
                r as f64
            };
        }
    }
    shift_and_subtract(x, y)
}

/// [`fmod`] by `e_fmod.c` for every pair: the part after its `i64` path, and in the lib tests
/// the oracle of that path.
fn shift_and_subtract(x: f64, y: f64) -> f64 {
    if x.is_nan() {
        return x;
    }
    if y.is_nan() {
        return y;
    }
    let (ux, uy) = (x.to_bits(), y.to_bits());
    let sx = ux & SIGN;
    let (hx, hy) = (ux & !SIGN, uy & !SIGN);
    if hy == 0 || hx >= f64::INFINITY.to_bits() {
        return f64::NAN;
    }
    if hx <= hy {
        // |x| < |y| (y infinite included) answers x; |x| = |y| answers 0 with the sign of x.
        return if hx < hy { x } else { f64::from_bits(sx) };
    }
    let (mut mx, ix) = normalized(hx);
    let (my, mut iy) = normalized(hy);
    // Fixed-point remainder.
    for _ in 0..(ix - iy) {
        if mx >= my {
            mx -= my;
            if mx == 0 {
                return f64::from_bits(sx);
            }
        }
        mx += mx;
    }
    if mx >= my {
        mx -= my;
    }
    if mx == 0 {
        return f64::from_bits(sx);
    }
    // Normalize: the leading bit back to 2^52, the exponent down with it.
    let shift = mx.leading_zeros() - 11;
    mx <<= shift;
    iy -= shift as i32;
    let magnitude = if iy >= -1022 {
        (mx - IMPLICIT) | (((iy + 1023) as u64) << 52)
    } else {
        // A subnormal result: exact, the bits shifted out are zero (|r| < |y| and y's
        // significand ends at 2^-1074 or above).
        mx >> (-1022 - iy)
    };
    f64::from_bits(magnitude | sx)
}

/// The least non-negative remainder of `x` modulo |`y`| as `f64::rem_euclid` computes it:
/// `r = fmod(x, y)`, and `r + |y|` when r < 0 (that sum is rounded, so a tiny negative r gives
/// |y|). NaN as [`fmod`].
pub fn rem_euclid(x: f64, y: f64) -> f64 {
    let r = fmod(x, y);
    if r < 0.0 { r + y.abs() } else { r }
}

/// The fast paths against the ports they stand for, with equal bits, NaN results included:
/// both roots answer x for a NaN and `f64::NAN` below zero, and the `i64` path of `fmod` takes
/// no NaN.
#[cfg(test)]
mod tests {
    use super::{fmod, shift_and_subtract, sqrt, sqrt_bit_by_bit};

    /// The corpus of `tests/fmath.rs`.
    const CORPUS: &str = include_str!("../tests/fixtures/fmath/corpus.txt");

    /// Knuth's 64-bit linear congruential generator, the design's.
    struct Lcg(u64);

    impl Lcg {
        fn next(&mut self) -> u64 {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            self.0
        }
    }

    fn number(hex: &str) -> f64 {
        f64::from_bits(u64::from_str_radix(hex, 16).unwrap_or(0))
    }

    /// Both roots over every `sqrt` input of the corpus and 100,000 drawn bit patterns (every
    /// exponent, NaN, ±∞ and the subnormals among them), and over the 2^16 bit patterns on each side
    /// of 1, 2, 2^-1022 and 2^-1074, where a carry or a short significand is likeliest to show.
    #[test]
    fn the_fast_root_equals_the_root_bit_by_bit() {
        let mut inputs: alloc::vec::Vec<f64> = CORPUS
            .lines()
            .filter_map(|line| line.strip_prefix("sqrt "))
            .filter_map(|rest| rest.split(' ').nth(1).map(number))
            .collect();
        assert!(
            inputs.len() > 300,
            "{} sqrt lines in the corpus",
            inputs.len()
        );
        let mut draw = Lcg(11);
        inputs.extend((0..100_000).map(|_| f64::from_bits(draw.next())));
        for start in [1.0f64, 2.0, f64::MIN_POSITIVE, 5e-324] {
            inputs.extend((0..1u64 << 16).map(|i| f64::from_bits(start.to_bits() + i)));
            inputs.extend((1..1u64 << 16).map(|i| f64::from_bits(start.to_bits().wrapping_sub(i))));
        }
        for x in inputs {
            let (fast, oracle) = (sqrt(x), sqrt_bit_by_bit(x));
            assert_eq!(
                fast.to_bits(),
                oracle.to_bits(),
                "sqrt of {:016x}",
                x.to_bits()
            );
        }
    }

    /// The `i64` path of `fmod` against the port for integer pairs below 2^53: drawn pairs of
    /// every magnitude up to 2^53 - 1, both signs, the divisors ±1 and ±2, zero dividends of
    /// both signs, dividends smaller than the divisor, and remainders of zero.
    #[test]
    fn the_integer_remainder_equals_the_port() {
        let mut draw = Lcg(12);
        let mut pairs = alloc::vec::Vec::new();
        for _ in 0..100_000 {
            let (a, b) = (draw.next(), draw.next());
            let x = ((a >> (11 + a % 53)) as i64) as f64;
            let y = ((b >> (11 + b % 53)) as i64).max(1) as f64;
            let signs = draw.next();
            let x = if signs & 1 == 0 { x } else { -x };
            let y = if signs & 2 == 0 { y } else { -y };
            pairs.extend([(x, y), (y, x), (x * y, y)]);
        }
        let largest = 9_007_199_254_740_991.0;
        for x in [
            0.0, -0.0, 1.0, -1.0, 3.0, -3.0, 6.0, -6.0, largest, -largest,
        ] {
            for y in [1.0, -1.0, 2.0, -2.0, 3.0, -3.0, 7.0, largest, -largest] {
                pairs.push((x, y));
            }
        }
        for (x, y) in pairs {
            if y == 0.0 || x.abs() > largest || y.abs() > largest {
                continue;
            }
            let (fast, port) = (fmod(x, y), shift_and_subtract(x, y));
            assert_eq!(
                fast.to_bits(),
                port.to_bits(),
                "fmod({:016x}, {:016x})",
                x.to_bits(),
                y.to_bits()
            );
        }
    }
}
