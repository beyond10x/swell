//! `Number::toString ( x, radix )` for radix 10 (`#sec-numeric-types-number-tostring`).
//!
//! Step 5 asks for integers n, k and s such that k ≥ 1, 10^(k-1) ≤ s < 10^k, 𝔽(s × 10^(n-k)) is
//! x, and k is as small as possible. [`shortest_digits`] finds them exactly, with the free-format
//! algorithm of Steele and White as Burger and Dybvig give it ("Printing Floating-Point Numbers
//! Quickly and Accurately", 1996, figure 1 with the scaling of section 2.2), over the small
//! unsigned big integers of this module: no floating-point arithmetic decides a digit. Where two
//! values of s of that length round to x it takes the one closer to x, and of two equally close
//! the even one, as the clause's Note 2 recommends. Steps 6 to 12 then lay the digits out.
//!
//! The other radices, `toFixed`, `toExponential` and `toPrecision` are slice 4's (design,
//! section 12, "A part before its slice").

use std::cmp::Ordering;

/// `Number::toString(x, 10)`.
pub fn number_to_string(x: f64) -> String {
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
    let (digits, n) = shortest_digits(x);
    layout(&mut out, &digits, n);
    out
}

/// Steps 6 to 12: the digits of s (k of them) and n, as a decimal or in scientific E notation.
fn layout(out: &mut String, digits: &[u8], n: i32) {
    let k = i32::try_from(digits.len()).unwrap_or(i32::MAX);
    let text: String = digits.iter().map(|&d| char::from(b'0' + d)).collect();
    let zeros = |out: &mut String, count: i32| {
        for _ in 0..count {
            out.push('0');
        }
    };
    if (-5..=21).contains(&n) {
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

/// For a finite positive `x`: the decimal digits of s, most significant first, and n, of step 5
/// of `Number::toString`, so that x is the Number nearest to 0.d₁d₂…d_k × 10^n.
pub(crate) fn shortest_digits(x: f64) -> (Vec<u8>, i32) {
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

    // n is the least integer with (r + m⁺) / s < 10^n (≤ when the upper bound reads back as x is
    // excluded). An estimate from the binary exponent, floor(L × log10 2) + 1 with
    // floor(L × log10 2) = (L × 78913) >> 18 for |L| ≤ 1650, is corrected below.
    let top = e + 63 - f.leading_zeros() as i32;
    let mut n = ((top * 78913) >> 18) + 1;
    if n >= 0 {
        s.mul_pow10(n.unsigned_abs());
    } else {
        let scale = n.unsigned_abs();
        r.mul_pow10(scale);
        m_plus.mul_pow10(scale);
        m_minus.mul_pow10(scale);
    }
    let upper_reached = |r: &Big, m_plus: &Big, s: &Big| match Big::cmp_sum(r, m_plus, s) {
        Ordering::Greater => true,
        Ordering::Equal => even,
        Ordering::Less => false,
    };
    while upper_reached(&r, &m_plus, &s) {
        s.mul_small(10);
        n += 1;
    }
    loop {
        let mut high = r.add(&m_plus);
        high.mul_small(10);
        let n_too_large = match high.cmp(&s) {
            Ordering::Less => true,
            Ordering::Equal => !even,
            Ordering::Greater => false,
        };
        if !n_too_large {
            break;
        }
        r.mul_small(10);
        m_plus.mul_small(10);
        m_minus.mul_small(10);
        n -= 1;
    }

    // Digit generation.
    let mut digits = Vec::with_capacity(17);
    let mut product = Big::from_u64(0);
    loop {
        r.mul_small(10);
        m_plus.mul_small(10);
        m_minus.mul_small(10);
        // The digit is floor(r / s), below 10. Its estimate from the leading 128 bits of both is
        // off by at most one either way, and corrected.
        let shift = r.bit_len().saturating_sub(128);
        let estimate = r.bits_from(shift) / s.bits_from(shift).max(1);
        let mut digit = u8::try_from(estimate.min(9)).unwrap_or(9);
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
            (false, false) => digits.push(digit),
            (true, false) => {
                digits.push(digit);
                break;
            }
            (false, true) => {
                digits.push(digit + 1);
                break;
            }
            (true, true) => {
                // Both d and d + 1 read back as x: the closer, and of two equally close the even.
                let twice = r.clone().shl(1);
                let up = match twice.cmp(&s) {
                    Ordering::Less => false,
                    Ordering::Greater => true,
                    Ordering::Equal => digit % 2 == 1,
                };
                digits.push(if up { digit + 1 } else { digit });
                break;
            }
        }
    }
    // The algorithm never produces a digit 10 or a trailing zero; this keeps the result a valid
    // s if it did.
    while let Some(&last) = digits.last() {
        if last < 10 {
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

    fn mul_pow10(&mut self, mut exponent: u32) {
        while exponent >= 19 {
            self.mul_small(10_000_000_000_000_000_000);
            exponent -= 19;
        }
        if exponent > 0 {
            self.mul_small(10u64.pow(exponent));
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
