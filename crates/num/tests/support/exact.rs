//! Exact arithmetic for the tests of `swell_num::math`: an unsigned big integer of 64-bit limbs
//! ([`Big`]) and a signed rational as two of them ([`Rat`]). Every Number is a rational, so a sum,
//! a product or a quotient of Numbers is held here without rounding, and a result of the crate is
//! compared with the exact value instead of with a second implementation (design, decision 6,
//! "How a test holds the bound with no second implementation").
//!
//! Written for these tests. `tests/to_string.rs` has its own big integer of 32-bit limbs; nothing
//! is copied from it. The tests of the other `Math` stories reuse this file.

#![cfg(test)]
#![allow(dead_code)]

use std::cmp::Ordering;

/// An unsigned integer of any size: 64-bit limbs, least significant first, no zero limb at the
/// top (zero has no limb).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Big(Vec<u64>);

impl Big {
    pub fn zero() -> Big {
        Big(Vec::new())
    }

    pub fn from_u64(value: u64) -> Big {
        let mut big = Big(vec![value]);
        big.trim();
        big
    }

    fn trim(&mut self) {
        while self.0.last() == Some(&0) {
            self.0.pop();
        }
    }

    pub fn is_zero(&self) -> bool {
        self.0.is_empty()
    }

    /// The number of bits up to the highest set one (0 for zero).
    pub fn bit_len(&self) -> u64 {
        match self.0.last() {
            None => 0,
            Some(top) => (self.0.len() as u64 - 1) * 64 + u64::from(64 - top.leading_zeros()),
        }
    }

    /// The number of zero bits below the lowest set one (0 for zero).
    pub fn trailing_zeros(&self) -> u64 {
        for (i, &limb) in self.0.iter().enumerate() {
            if limb != 0 {
                return i as u64 * 64 + u64::from(limb.trailing_zeros());
            }
        }
        0
    }

    /// The low 64 bits.
    pub fn low_u64(&self) -> u64 {
        self.0.first().copied().unwrap_or(0)
    }

    pub fn add(&self, other: &Big) -> Big {
        let len = self.0.len().max(other.0.len());
        let mut limbs = Vec::with_capacity(len + 1);
        let mut carry = 0u128;
        for i in 0..len {
            let sum = u128::from(self.0.get(i).copied().unwrap_or(0))
                + u128::from(other.0.get(i).copied().unwrap_or(0))
                + carry;
            limbs.push(sum as u64);
            carry = sum >> 64;
        }
        limbs.push(carry as u64);
        let mut sum = Big(limbs);
        sum.trim();
        sum
    }

    /// self - other; panics when other is larger.
    pub fn sub(&self, other: &Big) -> Big {
        assert!(self.compare(other) != Ordering::Less, "Big::sub below zero");
        let mut limbs = Vec::with_capacity(self.0.len());
        let mut borrow = 0u64;
        for (i, &limb) in self.0.iter().enumerate() {
            let (d1, b1) = limb.overflowing_sub(other.0.get(i).copied().unwrap_or(0));
            let (d2, b2) = d1.overflowing_sub(borrow);
            limbs.push(d2);
            borrow = u64::from(b1 || b2);
        }
        let mut difference = Big(limbs);
        difference.trim();
        difference
    }

    pub fn mul(&self, other: &Big) -> Big {
        if self.is_zero() || other.is_zero() {
            return Big::zero();
        }
        let mut limbs = vec![0u64; self.0.len() + other.0.len()];
        for (i, &a) in self.0.iter().enumerate() {
            let mut carry = 0u128;
            for (j, &b) in other.0.iter().enumerate() {
                let current = u128::from(limbs[i + j]) + u128::from(a) * u128::from(b) + carry;
                limbs[i + j] = current as u64;
                carry = current >> 64;
            }
            limbs[i + other.0.len()] = carry as u64;
        }
        let mut product = Big(limbs);
        product.trim();
        product
    }

    pub fn shl(&self, bits: u64) -> Big {
        if self.is_zero() {
            return Big::zero();
        }
        let words = (bits / 64) as usize;
        let shift = (bits % 64) as u32;
        let mut limbs = vec![0u64; words];
        let mut carry = 0u64;
        for &limb in &self.0 {
            if shift == 0 {
                limbs.push(limb);
            } else {
                limbs.push((limb << shift) | carry);
                carry = limb >> (64 - shift);
            }
        }
        limbs.push(carry);
        let mut shifted = Big(limbs);
        shifted.trim();
        shifted
    }

    /// floor(self / 2^bits).
    pub fn shr(&self, bits: u64) -> Big {
        let words = (bits / 64) as usize;
        if words >= self.0.len() {
            return Big::zero();
        }
        let shift = (bits % 64) as u32;
        let rest = &self.0[words..];
        let mut limbs = Vec::with_capacity(rest.len());
        for (i, &limb) in rest.iter().enumerate() {
            if shift == 0 {
                limbs.push(limb);
            } else {
                let high = rest.get(i + 1).copied().unwrap_or(0);
                limbs.push((limb >> shift) | (high << (64 - shift)));
            }
        }
        let mut shifted = Big(limbs);
        shifted.trim();
        shifted
    }

    pub fn compare(&self, other: &Big) -> Ordering {
        self.0
            .len()
            .cmp(&other.0.len())
            .then_with(|| self.0.iter().rev().cmp(other.0.iter().rev()))
    }
}

/// A rational number: sign, numerator and a positive denominator, with no common factor of two
/// (a Number's rational has a power of two as its denominator). Zero is positive.
#[derive(Clone, Debug)]
pub struct Rat {
    negative: bool,
    num: Big,
    den: Big,
}

impl Rat {
    /// negative × num / den, den non-zero.
    pub fn new(negative: bool, num: Big, den: Big) -> Rat {
        assert!(!den.is_zero(), "Rat with a zero denominator");
        if num.is_zero() {
            return Rat::zero();
        }
        let common = num.trailing_zeros().min(den.trailing_zeros());
        Rat {
            negative,
            num: num.shr(common),
            den: den.shr(common),
        }
    }

    pub fn zero() -> Rat {
        Rat {
            negative: false,
            num: Big::zero(),
            den: Big::from_u64(1),
        }
    }

    pub fn from_i64(value: i64) -> Rat {
        Rat::new(
            value < 0,
            Big::from_u64(value.unsigned_abs()),
            Big::from_u64(1),
        )
    }

    /// 2^k.
    pub fn pow2(k: i64) -> Rat {
        let one = Big::from_u64(1);
        if k >= 0 {
            Rat::new(false, one.shl(k as u64), Big::from_u64(1))
        } else {
            Rat::new(false, one.clone(), one.shl(k.unsigned_abs()))
        }
    }

    /// The exact value of a finite Number; panics on NaN and ±∞.
    pub fn from_f64(x: f64) -> Rat {
        assert!(x.is_finite(), "Rat::from_f64 of {x}");
        let bits = x.to_bits();
        let biased = ((bits >> 52) & 0x7FF) as i64;
        let stored = bits & ((1u64 << 52) - 1);
        let (mantissa, exponent) = if biased == 0 {
            (stored, -1074)
        } else {
            (stored | (1u64 << 52), biased - 1075)
        };
        Rat::new(
            x.is_sign_negative(),
            Big::from_u64(mantissa),
            Big::from_u64(1),
        )
        .mul(&Rat::pow2(exponent))
    }

    /// hi + lo, exactly.
    pub fn from_dd(hi: f64, lo: f64) -> Rat {
        Rat::from_f64(hi).add(&Rat::from_f64(lo))
    }

    pub fn is_zero(&self) -> bool {
        self.num.is_zero()
    }

    pub fn is_negative(&self) -> bool {
        self.negative
    }

    pub fn neg(&self) -> Rat {
        Rat::new(!self.negative, self.num.clone(), self.den.clone())
    }

    pub fn abs(&self) -> Rat {
        Rat::new(false, self.num.clone(), self.den.clone())
    }

    pub fn add(&self, other: &Rat) -> Rat {
        let left = self.num.mul(&other.den);
        let right = other.num.mul(&self.den);
        let den = self.den.mul(&other.den);
        if self.negative == other.negative {
            return Rat::new(self.negative, left.add(&right), den);
        }
        match left.compare(&right) {
            Ordering::Less => Rat::new(other.negative, right.sub(&left), den),
            _ => Rat::new(self.negative, left.sub(&right), den),
        }
    }

    pub fn sub(&self, other: &Rat) -> Rat {
        self.add(&other.neg())
    }

    pub fn mul(&self, other: &Rat) -> Rat {
        Rat::new(
            self.negative != other.negative,
            self.num.mul(&other.num),
            self.den.mul(&other.den),
        )
    }

    /// self / other; panics on a zero divisor.
    pub fn div(&self, other: &Rat) -> Rat {
        assert!(!other.is_zero(), "Rat::div by zero");
        Rat::new(
            self.negative != other.negative,
            self.num.mul(&other.den),
            self.den.mul(&other.num),
        )
    }

    pub fn compare(&self, other: &Rat) -> Ordering {
        match (self.is_zero(), other.is_zero()) {
            (true, true) => return Ordering::Equal,
            (true, false) => {
                return if other.negative {
                    Ordering::Greater
                } else {
                    Ordering::Less
                };
            }
            (false, true) => {
                return if self.negative {
                    Ordering::Less
                } else {
                    Ordering::Greater
                };
            }
            (false, false) => {}
        }
        if self.negative != other.negative {
            return if self.negative {
                Ordering::Less
            } else {
                Ordering::Greater
            };
        }
        let magnitude = self.num.mul(&other.den).compare(&other.num.mul(&self.den));
        if self.negative {
            magnitude.reverse()
        } else {
            magnitude
        }
    }

    /// The Number nearest to the value, a tie to the even one: ±∞ beyond the largest Number by
    /// half an ulp of it or more, ±0 below half the smallest subnormal (a tie at it gives ±0).
    pub fn to_f64(&self) -> f64 {
        let sign = if self.negative { 1u64 << 63 } else { 0 };
        if self.is_zero() {
            return 0.0;
        }
        // q = floor(num × 2^s / den) has 55 or 56 bits; sticky says whether the floor dropped a
        // non-zero rest.
        let s = 55 - (self.num.bit_len() as i64 - self.den.bit_len() as i64);
        let (mut rest, den) = if s >= 0 {
            (self.num.shl(s as u64), self.den.clone())
        } else {
            (self.num.clone(), self.den.shl(s.unsigned_abs()))
        };
        let mut q = 0u64;
        for bit in (0..57u64).rev() {
            let part = den.shl(bit);
            if part.compare(&rest) != Ordering::Greater {
                rest = rest.sub(&part);
                q |= 1u64 << bit;
            }
        }
        let sticky = !rest.is_zero();
        // The value is (q + sticky) × 2^-s, with its leading bit at 2^e.
        let e = i64::from(63 - q.leading_zeros() as i32) - s;
        if e > 1023 {
            return f64::from_bits(sign | 0x7FF0_0000_0000_0000);
        }
        let lsb = if e >= -1022 { e - 52 } else { -1074 };
        let drop = lsb + s;
        let (mut m, round_up) = if drop > 60 {
            (0u64, false)
        } else {
            let drop = drop as u32;
            let kept = q >> drop;
            let dropped = q & ((1u64 << drop) - 1);
            let half = 1u64 << (drop - 1);
            let up = dropped > half || (dropped == half && (sticky || kept & 1 == 1));
            (kept, up)
        };
        if round_up {
            m += 1;
        }
        let mut lsb = lsb;
        if m == 1u64 << 53 {
            m = 1u64 << 52;
            lsb += 1;
        }
        if m >= 1u64 << 52 {
            let biased = lsb + 52 + 1023;
            if biased >= 2047 {
                return f64::from_bits(sign | 0x7FF0_0000_0000_0000);
            }
            f64::from_bits(sign | ((biased as u64) << 52) | (m - (1u64 << 52)))
        } else {
            f64::from_bits(sign | m)
        }
    }

    /// log2 of the magnitude, approximately (for printing a measured error).
    pub fn log2(&self) -> f64 {
        if self.is_zero() {
            return f64::NEG_INFINITY;
        }
        let shift = self.num.bit_len() as i64 - self.den.bit_len() as i64;
        let scaled = self.abs().mul(&Rat::pow2(-shift));
        scaled.to_f64().log2() + shift as f64
    }
}

/// |approx - exact| / |exact|, for a non-zero exact value.
pub fn relative_error(approx: &Rat, exact: &Rat) -> Rat {
    approx.sub(exact).abs().div(&exact.abs())
}

/// A bound of the relative error of root as the square root of value (both positive):
/// |root - √value| / √value is at most |root² - value| / (2 · value) when root² ≥ value, and
/// below |root² - value| / (value + root²) when root² < value. It is |t - 1| / (1 + √t) with
/// t = root² / value, and √t ≥ 1 in the first case, √t > t in the second.
pub fn sqrt_relative_error(root: &Rat, value: &Rat) -> Rat {
    let square = root.mul(root);
    let difference = square.sub(value).abs();
    if square.compare(value) != Ordering::Less {
        difference.div(&value.add(value))
    } else {
        difference.div(&value.add(&square))
    }
}

/// Whether the non-negative Number r is one of the two Numbers that enclose √square (or √square
/// itself): next_down(r)² < square < next_up(r)², the side of r that square lies on decided by
/// r² against square.
pub fn is_faithful_root(r: f64, square: &Rat) -> bool {
    if !r.is_finite() || r < 0.0 {
        return false;
    }
    let exact = Rat::from_f64(r);
    let r_square = exact.mul(&exact);
    match r_square.compare(square) {
        Ordering::Equal => true,
        Ordering::Less => {
            let up = r.next_up();
            if !up.is_finite() {
                return true;
            }
            let up = Rat::from_f64(up);
            up.mul(&up).compare(square) == Ordering::Greater
        }
        Ordering::Greater => {
            let down = Rat::from_f64(r.next_down());
            down.mul(&down).compare(square) == Ordering::Less
        }
    }
}

/// Integer division, for the constants of `tests/math_trig.rs` (2/π as the quotient of a power
/// of two by π, the terms of a series).
impl Big {
    /// floor(self / divisor), limb by limb; panics on a zero divisor.
    pub fn div_small(&self, divisor: u64) -> Big {
        assert!(divisor != 0, "Big::div_small by zero");
        let divisor = u128::from(divisor);
        let mut limbs = vec![0u64; self.0.len()];
        let mut rest = 0u128;
        for (slot, &limb) in limbs.iter_mut().zip(&self.0).rev() {
            let current = (rest << 64) | u128::from(limb);
            *slot = (current / divisor) as u64;
            rest = current % divisor;
        }
        let mut quotient = Big(limbs);
        quotient.trim();
        quotient
    }

    /// floor(self / divisor), bit by bit (binary long division); panics on a zero divisor.
    pub fn div_floor(&self, divisor: &Big) -> Big {
        assert!(!divisor.is_zero(), "Big::div_floor by zero");
        let one = Big::from_u64(1);
        let mut limbs = vec![0u64; self.0.len()];
        let mut rest = Big::zero();
        for bit in (0..self.bit_len()).rev() {
            let word = (bit / 64) as usize;
            rest = rest.shl(1);
            if (self.0[word] >> (bit % 64)) & 1 == 1 {
                rest = rest.add(&one);
            }
            if rest.compare(divisor) != Ordering::Less {
                rest = rest.sub(divisor);
                limbs[word] |= 1 << (bit % 64);
            }
        }
        let mut quotient = Big(limbs);
        quotient.trim();
        quotient
    }
}
