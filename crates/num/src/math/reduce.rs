//! The argument reduction of `sin`, `cos` and `tan` (design, decision 6, "The functions of
//! `Math` that IEEE 754 does not fix", item 1): x = k π/2 + r with k an integer and |r| ≤ π/4,
//! by Payne and Hanek's method for every |x| above π/4 (Muller, Handbook of Floating-Point
//! Arithmetic, 11.2). One method for every size: no medium-size path with constants of its own.
//!
//! x is m × 2^e with m the 53-bit mantissa. Bit i of 2/π (weight 2^-i) adds m × 2^(e - i) to
//! x × 2/π, a multiple of 4 for i ≤ e - 2, so only the bits from e - 1 on reach k mod 4 and the
//! fraction. [`reduce`] multiplies m by a window of 256 of those bits, read from
//! [`TWO_OVER_PI`] at the position the exponent selects, by `u128` partial products, and keeps
//! of the product the two integer bits and the first 192 bits of the fraction (the design's 192
//! kept bits). The bits of 2/π beyond the window add less than m × 2^(e - (e - 1 + 255)) =
//! m × 2^-256 < 2^-202, and the cut at 192 bits less than 2^-192; the fraction of a Number is
//! never nearer an integer than about 2^-62 (`crates/num/tests/math_trig.rs` measures the
//! closest approach), so the fraction keeps a relative error below 2^-128. The fraction is
//! rounded to the nearest integer (k, and |f| ≤ 1/2), turned into a pair from its first 128
//! significant bits, and multiplied by π/2 as a pair: r with relative error at most 2^-100,
//! which the test file holds against exact arithmetic.
//!
//! The bits read go up to position 971 + 254 = 1,225 of the table's 1,280 for the largest
//! Number (e = 971); every loop runs over a fixed number of words, none over the argument.
//!
//! The operations and what fixes each result: `u64` and `u128` arithmetic (the product of two
//! `u64` fits a `u128`; a carry is added to a product below 2^117; shifts by less than the
//! width), `to_bits` and `from_bits` (exact), `abs` (exact), `as f64` of a `u64` below 2^53
//! (exact, the Rust Reference, "Numeric cast"), the pair operations of [`super::dd`] and one
//! `f64` addition (correctly rounded, IEEE 754-2019, 5.4.1).

use super::dd::{self, Dd, fast_two_sum, pair};

/// The first 1,280 fractional bits of 2/π, most significant word first: floor(2^1280 × 2/π).
/// `crates/num/tests/math_trig.rs` recomputes them from π by Machin's formula to 1,400 bits.
pub const TWO_OVER_PI: [u64; 20] = [
    0xA2F9_836E_4E44_1529,
    0xFC27_57D1_F534_DDC0,
    0xDB62_9599_3C43_9041,
    0xFE51_63AB_DEBB_C561,
    0xB724_6E3A_424D_D2E0,
    0x0649_2EEA_09D1_921C,
    0xFE1D_EB1C_B129_A73E,
    0xE882_35F5_2EBB_4484,
    0xE99C_7026_B45F_7E41,
    0x3991_D639_8353_39F4,
    0x9C84_5F8B_BDF9_283B,
    0x1FF8_97FF_DE05_980F,
    0xEF2F_118B_5A0A_6D1F,
    0x6D36_7ECF_27CB_09B7,
    0x4F46_3F66_9E5F_EA2D,
    0x7527_BAC7_EBE5_F17B,
    0x3D07_39F7_8A52_92EA,
    0x6BFB_5FB1_1F8D_5D08,
    0x5603_3046_FC7B_6BAB,
    0xF0CF_BC20_9AF4_361D,
];

/// π/2 as a pair: the Number nearest to π/2 and the Number nearest to the rest.
pub const PI_OVER_2: Dd = pair(0x3FF9_21FB_5444_2D18, 0x3C91_A626_3314_5C07);

/// The largest Number not above π/4: an argument up to it is not reduced.
const PI_OVER_4_BELOW: f64 = f64::from_bits(0x3FE9_21FB_5444_2D18);

/// The 52 stored bits of a Number.
const STORED: u64 = 0x000F_FFFF_FFFF_FFFF;

/// The quadrant k mod 4 and r = x - k π/2 as a pair, |r| ≤ π/4 (k the integer nearest to
/// x × 2/π): `(0, x)` for |x| at most π/4 and for NaN; `(0, NaN)` for ±∞. A negative x is
/// reduced as -x with the quadrant and r negated.
pub fn reduce(x: f64) -> (u8, Dd) {
    let magnitude = x.abs();
    if magnitude.is_nan() || magnitude <= PI_OVER_4_BELOW {
        return (0, Dd::from_f64(x));
    }
    if magnitude.is_infinite() {
        return (0, Dd::from_f64(f64::NAN));
    }
    let (quadrant, r) = reduce_above_pi_over_4(magnitude);
    if x < 0.0 {
        (quadrant.wrapping_neg() & 3, -r)
    } else {
        (quadrant, r)
    }
}

/// Payne and Hanek for a finite x above π/4.
fn reduce_above_pi_over_4(x: f64) -> (u8, Dd) {
    let bits = x.to_bits();
    let mantissa = (bits & STORED) | (1 << 52);
    // x = mantissa × 2^e; x above π/4 has a biased exponent of 1022 or more, so e is -53 to 971.
    let e = i32::try_from(bits >> 52).unwrap_or(0) - 1075;
    // The first bit of 2/π that reaches k mod 4: position e - 1, and 1 (the first fractional
    // bit) for e below 2. Its offset in the table is 0 to 969.
    let first = (e - 1).max(1);
    let window = window_of(u32::try_from(first - 1).unwrap_or(0));
    // The product mantissa × window: 309 bits at most, in five words, least significant first.
    let mut product = [0u64; 5];
    let mut carry = 0u128;
    for (slot, word) in product.iter_mut().zip(window.iter().rev()) {
        // At most (2^53 - 1)(2^64 - 1) + 2^53 - 1 < 2^117.
        let partial = u128::from(mantissa) * u128::from(*word) + carry;
        *slot = partial as u64;
        carry = partial >> 64;
    }
    if let Some(top) = product.last_mut() {
        *top = carry as u64;
    }
    // x × 2/π ≡ product × 2^-s (mod 4), s = first + 255 - e fraction bits: 254 to 309.
    let s = u32::try_from(first + 255 - e).unwrap_or(254);
    let integer = bits_at(&product, s) & 3;
    let mut fraction = [
        bits_at(&product, s - 64),
        bits_at(&product, s - 128),
        bits_at(&product, s - 192),
    ];
    // Round to the nearest integer: a fraction of 1/2 or more counts one more quadrant and
    // leaves 1 - fraction, negated.
    let [high, middle, low] = fraction;
    let negative = high >> 63 == 1;
    let quadrant = if negative {
        let (low, borrow_low) = (!low).overflowing_add(1);
        let (middle, borrow_middle) = (!middle).overflowing_add(u64::from(borrow_low));
        let high = (!high).wrapping_add(u64::from(borrow_middle));
        fraction = [high, middle, low];
        (integer + 1) & 3
    } else {
        integer
    };
    let r = fraction_to_pair(fraction) * PI_OVER_2;
    let quadrant = u8::try_from(quadrant).unwrap_or(0);
    if negative {
        (quadrant, -r)
    } else {
        (quadrant, r)
    }
}

/// 256 bits of the table from the bit at `offset` (0 for the first fractional bit), most
/// significant word first; bits past the table are zero.
fn window_of(offset: u32) -> [u64; 4] {
    let word = usize::try_from(offset / 64).unwrap_or(0);
    let shift = offset % 64;
    let mut window = [0u64; 4];
    for (k, slot) in window.iter_mut().enumerate() {
        let high = TWO_OVER_PI.get(word + k).copied().unwrap_or(0);
        let low = TWO_OVER_PI.get(word + k + 1).copied().unwrap_or(0);
        *slot = if shift == 0 {
            high
        } else {
            (high << shift) | (low >> (64 - shift))
        };
    }
    window
}

/// The 64 bits of a number held in words least significant first, from bit `start` up; bits
/// past the words are zero.
fn bits_at(words: &[u64; 5], start: u32) -> u64 {
    let word = usize::try_from(start / 64).unwrap_or(0);
    let shift = start % 64;
    let low = words.get(word).copied().unwrap_or(0);
    let high = words.get(word + 1).copied().unwrap_or(0);
    if shift == 0 {
        low
    } else {
        (low >> shift) | (high << (64 - shift))
    }
}

/// The fraction f = (high, middle, low) × 2^-192, 0 ≤ f ≤ 1/2, as a pair: its first 128
/// significant bits, the high part their first 53 (exact), the low part the other 75 in one
/// rounding.
fn fraction_to_pair(fraction: [u64; 3]) -> Dd {
    let [high, middle, low] = fraction;
    let zeros = if high != 0 {
        high.leading_zeros()
    } else if middle != 0 {
        64 + middle.leading_zeros()
    } else if low != 0 {
        128 + low.leading_zeros()
    } else {
        return Dd::from_f64(0.0);
    };
    // The fraction shifted left by `zeros`: its first word has its top bit set.
    let words = usize::try_from(zeros / 64).unwrap_or(0);
    let shift = zeros % 64;
    let mut normalized = [0u64; 2];
    for (k, slot) in normalized.iter_mut().enumerate() {
        let upper = fraction.get(k + words).copied().unwrap_or(0);
        let lower = fraction.get(k + words + 1).copied().unwrap_or(0);
        *slot = if shift == 0 {
            upper
        } else {
            (upper << shift) | (lower >> (64 - shift))
        };
    }
    let [first, second] = normalized;
    let top = (u128::from(first) << 64) | u128::from(second);
    // top × 2^-(128 + zeros) is the fraction, cut after 128 bits; zeros is at most 191.
    let exponent = -128 - i32::try_from(zeros).unwrap_or(0);
    let head = (top >> 75) as u64;
    let next = ((top >> 22) as u64) & ((1 << 53) - 1);
    let last = (top as u64) & ((1 << 22) - 1);
    let hi = dd::scale(head as f64, exponent + 75);
    let lo = dd::scale(next as f64, exponent + 22) + dd::scale(last as f64, exponent);
    fast_two_sum(hi, lo)
}
