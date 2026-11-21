//! `Number::toString ( x, radix )` (`#sec-numeric-types-number-tostring`) for every radix from 2
//! to 36.
//!
//! Step 5 asks for integers n, k and s such that k ≥ 1, radix^(k-1) ≤ s < radix^k,
//! 𝔽(s × radix^(n-k)) is x, and k is as small as possible. [`shortest_digits`] finds them
//! exactly, with the free-format algorithm of Steele and White as Burger and Dybvig give it
//! ("Printing Floating-Point Numbers Quickly and Accurately", 1996, figure 1 with the scaling of
//! section 2.2; the algorithm is written for any output base), over the small unsigned big
//! integers of this module: no floating-point arithmetic decides a digit. Where two values of s
//! of that length round to x it takes the one closer to x, and of two equally close the even
//! one, as the clause's Note 2 recommends (in an odd radix s is even when the sum of its digits
//! is). Steps 6 to 12 then lay the digits out with the first `radix` code units of
//! "0123456789abcdefghijklmnopqrstuvwxyz": in radix 10 as a decimal or in scientific E notation,
//! in any other radix always as step 6 does, with no exponent.
//!
//! Slice 1 took radix 10 early and slice 2 the other radices (design, section 12, "A part before
//! its slice"); `toFixed`, `toExponential` and `toPrecision` are slice 4's.

use std::cmp::Ordering;

/// The digits of every radix, from the clause: "taken from the first r code units of
/// "0123456789abcdefghijklmnopqrstuvwxyz" in order".
const DIGITS: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";

/// `Number::toString(x, 10)`.
pub fn number_to_string(x: f64) -> String {
    to_string_in_radix(x, 10)
}

/// `Number::toString(x, radix)`, or `None` for a radix outside 2 to 36, which the clause does
/// not take.
pub fn number_to_string_radix(x: f64, radix: u32) -> Option<String> {
    if !(2..=36).contains(&radix) {
        return None;
    }
    Some(to_string_in_radix(x, radix))
}

/// `Number::toString(x, radix)` for a radix from 2 to 36.
fn to_string_in_radix(x: f64, radix: u32) -> String {
    // Steps 1 to 4.
    if x.is_nan() {
        return "NaN".to_string();
    }
    if x == 0.0 {
        return "0".to_string();
    }
    let mut out = String::new();
    if x < 0.0 {
        out.push('-');
    }
    let x = x.abs();
    if x.is_infinite() {
        out.push_str("Infinity");
        return out;
    }
    let (digits, n) = shortest_digits(x, radix);
    layout(&mut out, &digits, n, radix);
    out
}

/// Steps 6 to 12: the digits of s (k of them) and n, as a positional numeral, or in radix 10
/// outside n from -5 to 21 in scientific E notation.
fn layout(out: &mut String, digits: &[u8], n: i32, radix: u32) {
    let k = i32::try_from(digits.len()).unwrap_or(i32::MAX);
    let text: String = digits
        .iter()
        .map(|&d| char::from(DIGITS.get(usize::from(d)).copied().unwrap_or(b'0')))
        .collect();
    let zeros = |out: &mut String, count: i32| {
        for _ in 0..count {
            out.push('0');
        }
    };
    // Step 6: "If radix ≠ 10 or n is in the inclusive interval from -5 to 21".
    if radix != 10 || (-5..=21).contains(&n) {
        if n >= k {
            // Step 6.a.
            out.push_str(&text);
            zeros(out, n - k);
        } else if n > 0 {
            // Step 6.b.
            let split = usize::try_from(n).unwrap_or(0);
            let (int, frac) = text.split_at_checked(split).unwrap_or((&text, ""));
            out.push_str(int);
            out.push('.');
            out.push_str(frac);
        } else {
            // Steps 6.c and 6.d.
            out.push_str("0.");
            zeros(out, -n);
            out.push_str(&text);
        }
        return;
    }
    // Steps 7 to 12.
    let sign = if n < 0 { '-' } else { '+' };
    let exponent = (n - 1).unsigned_abs();
    let (first, rest) = text.split_at_checked(1).unwrap_or((&text, ""));
    out.push_str(first);
    if !rest.is_empty() {
        out.push('.');
        out.push_str(rest);
    }
    out.push('e');
    out.push(sign);
    out.push_str(&exponent.to_string());
}

/// For a finite positive `x` and a radix from 2 to 36: the digits of s in that radix, most
/// significant first, and n, of step 5 of `Number::toString`, so that x is the Number nearest to
/// 0.d₁d₂…d_k × radix^n.
pub(crate) fn shortest_digits(x: f64, radix: u32) -> (Vec<u8>, i32) {
    // The callers pass a radix from 2 to 36; the clamp keeps every loop below finite whatever
    // they pass.
    let base = u64::from(radix.clamp(2, 36));
    let bits = x.to_bits();
    let biased = ((bits >> 52) & 0x7ff) as i32;
    let fraction = bits & ((1u64 << 52) - 1);
    // x = f × 2^e.
    let (f, e) = if biased == 0 {
        (fraction, -1074)
    } else {
        (fraction | (1u64 << 52), biased - 1075)
    };
    // Round to nearest, ties to even: the bounds of the interval that reads back as x belong to
    // it when f is even.
    let even = f & 1 == 0;
    // A power of two above the smallest normal Number: the gap below x is half the gap above.
    let unequal_gaps = biased > 1 && fraction == 0;

    // x = r / s; the interval that rounds to x is (r - m⁻, r + m⁺) / s.
    let one = || Big::from_u64(1);
    let magnitude = e.unsigned_abs();
    let (mut r, mut s, mut m_plus, mut m_minus) = match (e >= 0, unequal_gaps) {
        (true, true) => (
            Big::from_u64(f).shl(magnitude + 2),
            Big::from_u64(4),
            one().shl(magnitude + 1),
            one().shl(magnitude),
        ),
        (true, false) => (
            Big::from_u64(f).shl(magnitude + 1),
            Big::from_u64(2),
            one().shl(magnitude),
            one().shl(magnitude),
        ),
        (false, true) => (
            Big::from_u64(f).shl(2),
            one().shl(magnitude + 2),
            Big::from_u64(2),
            one(),
        ),
        (false, false) => (
            Big::from_u64(f).shl(1),
            one().shl(magnitude + 1),
            one(),
            one(),
        ),
    };

    // n is the least integer with (r + m⁺) / s < radix^n (≤ when the upper bound reads back as x
    // is excluded). An estimate from the binary exponent L, floor(L / log2 radix) + 1, is
    // corrected by the two loops below. For radix 10 it is floor(L × log10 2) + 1 with
    // floor(L × log10 2) = (L × 78913) >> 18 for |L| ≤ 1650; for another radix it divides by
    // log2 radix in fixed point ([`log2_q16`]). Only the number of passes of the loops depends on
    // how close the estimate is, not the result.
    let top = e + 63 - f.leading_zeros() as i32;
    let mut n = if base == 10 {
        ((top * 78913) >> 18) + 1
    } else {
        let estimate = (i64::from(top) * 65536).div_euclid(log2_q16(base)) + 1;
        i32::try_from(estimate).unwrap_or(0)
    };
    if n >= 0 {
        s.mul_pow(base, n.unsigned_abs());
    } else {
        let scale = n.unsigned_abs();
        r.mul_pow(base, scale);
        m_plus.mul_pow(base, scale);
        m_minus.mul_pow(base, scale);
    }
    let upper_reached = |r: &Big, m_plus: &Big, s: &Big| match Big::cmp_sum(r, m_plus, s) {
        Ordering::Greater => true,
        Ordering::Equal => even,
        Ordering::Less => false,
    };
    while upper_reached(&r, &m_plus, &s) {
        s.mul_small(base);
        n += 1;
    }
    loop {
        let mut high = r.add(&m_plus);
        high.mul_small(base);
        let n_too_large = match high.cmp(&s) {
            Ordering::Less => true,
            Ordering::Equal => !even,
            Ordering::Greater => false,
        };
        if !n_too_large {
            break;
        }
        r.mul_small(base);
        m_plus.mul_small(base);
        m_minus.mul_small(base);
        n -= 1;
    }

    // Digit generation.
    let mut digits = Vec::with_capacity(17);
    // The sum of the digits so far: in an odd radix, s is even when it is.
    let mut digit_sum = 0u64;
    let mut product = Big::from_u64(0);
    let top_digit = u8::try_from(base - 1).unwrap_or(9);
    loop {
        r.mul_small(base);
        m_plus.mul_small(base);
        m_minus.mul_small(base);
        // The digit is floor(r / s), below the radix. Its estimate from the leading 128 bits of
        // both is off by at most one either way, and corrected.
        let shift = r.bit_len().saturating_sub(128);
        let estimate = r.bits_from(shift) / s.bits_from(shift).max(1);
        let mut digit = u8::try_from(estimate.min(u128::from(top_digit))).unwrap_or(top_digit);
        if digit > 0 {
            product.limbs.clear();
            product.limbs.extend_from_slice(&s.limbs);
            product.mul_small(u64::from(digit));
            while digit > 0 && product.cmp(&r) == Ordering::Greater {
                product.sub_assign(&s);
                digit -= 1;
            }
            r.sub_assign(&product);
        }
        while r.cmp(&s) != Ordering::Less {
            r.sub_assign(&s);
            digit += 1;
        }
        let low = match r.cmp(&m_minus) {
            Ordering::Less => true,
            Ordering::Equal => even,
            Ordering::Greater => false,
        };
        let high = upper_reached(&r, &m_plus, &s);
        match (low, high) {
            (false, false) => {
                digits.push(digit);
                digit_sum += u64::from(digit);
            }
            (true, false) => {
                digits.push(digit);
                break;
            }
            (false, true) => {
                digits.push(digit + 1);
                break;
            }
            (true, true) => {
                // Both d and d + 1 read back as x: the closer, and of two equally close the even
                // s. In an even radix s has the parity of its last digit; in an odd one, every
                // power of the radix being odd, the parity of the sum of its digits.
                let twice = r.clone().shl(1);
                let s_with_digit_odd = if base.is_multiple_of(2) {
                    digit % 2 == 1
                } else {
                    (digit_sum + u64::from(digit)) % 2 == 1
                };
                let up = match twice.cmp(&s) {
                    Ordering::Less => false,
                    Ordering::Greater => true,
                    Ordering::Equal => s_with_digit_odd,
                };
                digits.push(if up { digit + 1 } else { digit });
                break;
            }
        }
    }
    // The algorithm never produces a digit equal to the radix or a trailing zero; this keeps the
    // result a valid s if it did.
    while let Some(&last) = digits.last() {
        if u64::from(last) < base {
            break;
        }
        digits.pop();
        match digits.last_mut() {
            Some(previous) => *previous += 1,
            None => {
                digits.push(1);
                n += 1;
            }
        }
    }
    while digits.len() > 1 && digits.last() == Some(&0) {
        digits.pop();
    }
    (digits, n)
}

/// floor(log2(base) × 2^16) for a base of 2 or more, in integers: the integer part from the
/// highest set bit, then the 16 binary digits of the fraction of log2 of y = base / 2^integer,
/// one per squaring of y (a square of 2 or more gives a 1 and is halved), y held with 62
/// fractional bits. The truncation of each square can only lower the result; it steers the
/// estimate of n, which the loops of [`shortest_digits`] correct, and is exact for a power of
/// two.
fn log2_q16(base: u64) -> i64 {
    let base = base.max(2);
    let integer = 63 - base.leading_zeros();
    // y in [1, 2): y × 2^62 is below 2^63, so its square fits in 128 bits.
    let mut y = u128::from(base) << 62u32.saturating_sub(integer);
    let two = 2u128 << 62;
    let mut fraction = 0i64;
    for _ in 0..16 {
        y = (y * y) >> 62;
        fraction <<= 1;
        if y >= two {
            y >>= 1;
            fraction |= 1;
        }
    }
    (i64::from(integer) << 16) | fraction
}

/// An unsigned integer of any size: 64-bit limbs, least significant first, with no zero limb at
/// the top. Only what the digit generation needs.
#[derive(Clone)]
struct Big {
    limbs: Vec<u64>,
}

impl Big {
    fn from_u64(value: u64) -> Big {
        let mut limbs = Vec::with_capacity(20);
        if value != 0 {
            limbs.push(value);
        }
        Big { limbs }
    }

    fn trim(&mut self) {
        while self.limbs.last() == Some(&0) {
            self.limbs.pop();
        }
    }

    /// The number of bits up to the highest set one.
    fn bit_len(&self) -> u32 {
        let full = u32::try_from(self.limbs.len().saturating_sub(1)).unwrap_or(u32::MAX);
        match self.limbs.last() {
            Some(top) => full
                .saturating_mul(64)
                .saturating_add(64 - top.leading_zeros()),
            None => 0,
        }
    }

    /// The 128 bits from bit `shift` up: floor(self / 2^shift) modulo 2^128.
    fn bits_from(&self, shift: u32) -> u128 {
        let word = (shift / 64) as usize;
        let bits = shift % 64;
        let limb = |i: usize| u128::from(self.limbs.get(word + i).copied().unwrap_or(0));
        let low = (limb(0) | (limb(1) << 64)) >> bits;
        if bits == 0 {
            low
        } else {
            low | (limb(2) << (128 - bits))
        }
    }

    fn shl(mut self, bits: u32) -> Big {
        let words = (bits / 64) as usize;
        let bits = bits % 64;
        if bits != 0 {
            let mut carry = 0u64;
            for limb in &mut self.limbs {
                let value = *limb;
                *limb = (value << bits) | carry;
                carry = value >> (64 - bits);
            }
            if carry != 0 {
                self.limbs.push(carry);
            }
        }
        if words != 0 && !self.limbs.is_empty() {
            self.limbs.splice(0..0, std::iter::repeat_n(0, words));
        }
        self
    }

    fn mul_small(&mut self, factor: u64) {
        let mut carry = 0u128;
        for limb in &mut self.limbs {
            let product = u128::from(*limb) * u128::from(factor) + carry;
            *limb = product as u64;
            carry = product >> 64;
        }
        if carry != 0 {
            self.limbs.push(carry as u64);
        }
        self.trim();
    }

    /// self × base^exponent, for a base from 2 to 36: by the largest power of the base that fits
    /// in a limb (10^19 for radix 10), then by the rest.
    fn mul_pow(&mut self, base: u64, mut exponent: u32) {
        let base = base.max(2);
        let (mut chunk, mut factor) = (0u32, 1u64);
        while let Some(next) = factor.checked_mul(base) {
            factor = next;
            chunk += 1;
        }
        while exponent >= chunk {
            self.mul_small(factor);
            exponent -= chunk;
        }
        // base^exponent < factor, so it fits.
        let mut rest = 1u64;
        for _ in 0..exponent {
            rest *= base;
        }
        if rest > 1 {
            self.mul_small(rest);
        }
    }

    fn add(&self, other: &Big) -> Big {
        let (long, short) = if self.limbs.len() >= other.limbs.len() {
            (self, other)
        } else {
            (other, self)
        };
        let mut limbs = Vec::with_capacity(long.limbs.len() + 1);
        let mut carry = false;
        let mut short = short.limbs.iter();
        for &a in &long.limbs {
            let b = short.next().copied().unwrap_or(0);
            let (sum, c1) = a.overflowing_add(b);
            let (sum, c2) = sum.overflowing_add(u64::from(carry));
            limbs.push(sum);
            carry = c1 || c2;
        }
        if carry {
            limbs.push(1);
        }
        Big { limbs }
    }

    /// self - other, for other ≤ self.
    fn sub_assign(&mut self, other: &Big) {
        let mut borrow = false;
        let mut other = other.limbs.iter();
        for limb in &mut self.limbs {
            let b = other.next().copied().unwrap_or(0);
            let (difference, b1) = limb.overflowing_sub(b);
            let (difference, b2) = difference.overflowing_sub(u64::from(borrow));
            *limb = difference;
            borrow = b1 || b2;
        }
        self.trim();
    }

    fn cmp(&self, other: &Big) -> Ordering {
        self.limbs
            .len()
            .cmp(&other.limbs.len())
            .then_with(|| self.limbs.iter().rev().cmp(other.limbs.iter().rev()))
    }

    /// a + b compared with c, without forming the sum: the sign of a + b - c, from the borrows
    /// of c - a - b computed from the least significant limb up.
    fn cmp_sum(a: &Big, b: &Big, c: &Big) -> Ordering {
        let width = a.limbs.len().max(b.limbs.len()).max(c.limbs.len());
        let (mut a, mut b, mut c) = (a.limbs.iter(), b.limbs.iter(), c.limbs.iter());
        // c - a - b as limbs, with a borrow of 0, 1 or 2 into the next limb.
        let mut borrow = 0u64;
        let mut nonzero = false;
        for _ in 0..width {
            let x = c.next().copied().unwrap_or(0);
            let y = a.next().copied().unwrap_or(0);
            let z = b.next().copied().unwrap_or(0);
            let (d, b1) = x.overflowing_sub(y);
            let (d, b2) = d.overflowing_sub(z);
            let (d, b3) = d.overflowing_sub(borrow);
            borrow = u64::from(b1) + u64::from(b2) + u64::from(b3);
            nonzero |= d != 0;
        }
        if borrow != 0 {
            Ordering::Greater
        } else if nonzero {
            Ordering::Less
        } else {
            Ordering::Equal
        }
    }
}
