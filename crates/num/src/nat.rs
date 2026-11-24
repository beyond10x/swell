//! `Nat`, an unsigned integer of any length (`docs/design/engine.md`, decision 6, "Big integers
//! in slice 4"): 64-bit limbs, least significant first, `u128` intermediates, no zero limb at
//! the top. It is the former `Big` of `to_string.rs` and keeps its behaviour there; slice 4
//! adds what `toFixed`, `toExponential` and `toPrecision` need (`format.rs`): the product by a
//! `Nat`, a power of a `u64`, the quotient and remainder by a `u64`, the difference, the right
//! shift and the decimal digits.
//!
//! No operand of these uses is longer than 2,048 bits (32 limbs; the design's bound), so every
//! loop here runs over at most 32 limbs: `swell-num` takes no budget, and the builtins charge
//! the temporaries of a call as one temporary of 2,048 bytes. The lib tests read the longest
//! operand made, in bits (`longest_bits`).

use std::cmp::Ordering;

#[cfg(test)]
thread_local! {
    /// The longest operand made on this thread, in bits, in the lib tests.
    static LONGEST: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

/// The longest operand made on this thread since `reset_longest`, in bits.
#[cfg(test)]
pub(crate) fn longest_bits() -> u32 {
    LONGEST.with(std::cell::Cell::get)
}

/// Count the longest operand from 0 again.
#[cfg(test)]
pub(crate) fn reset_longest() {
    LONGEST.with(|longest| longest.set(0));
}

/// An unsigned integer of any size: 64-bit limbs, least significant first, with no zero limb at
/// the top.
#[derive(Clone)]
pub(crate) struct Nat {
    pub(crate) limbs: Vec<u64>,
}

impl Nat {
    /// In the lib tests, record the length of a result that may have grown.
    fn note(&self) {
        #[cfg(test)]
        LONGEST.with(|longest| longest.set(longest.get().max(self.bit_len())));
    }

    /// base^exponent: 1 for the exponent 0, 0 for the base 0 and any other exponent.
    pub(crate) fn pow_u64(base: u64, exponent: u32) -> Nat {
        let mut power = Nat::from_u64(1);
        match base {
            0 if exponent > 0 => power.limbs.clear(),
            0 | 1 => {}
            _ => power.mul_pow(base, exponent),
        }
        power
    }

    /// self × other, the schoolbook product: each limb of one by each limb of the other, with
    /// `u128` intermediates. a × b + c + carry stays below 2^128 for limbs a, b, c and a carry
    /// below 2^64.
    pub(crate) fn mul(&self, other: &Nat) -> Nat {
        if self.limbs.is_empty() || other.limbs.is_empty() {
            return Nat::from_u64(0);
        }
        let mut limbs = vec![0u64; self.limbs.len() + other.limbs.len()];
        for (i, &a) in self.limbs.iter().enumerate() {
            let mut carry = 0u128;
            for (j, &b) in other.limbs.iter().enumerate() {
                if let Some(slot) = limbs.get_mut(i + j) {
                    let sum = u128::from(a) * u128::from(b) + u128::from(*slot) + carry;
                    *slot = sum as u64;
                    carry = sum >> 64;
                }
            }
            // The rows before this one wrote no limb above i + other's length - 1.
            if let Some(slot) = limbs.get_mut(i + other.limbs.len()) {
                *slot = carry as u64;
            }
        }
        let mut product = Nat { limbs };
        product.trim();
        product.note();
        product
    }

    /// self / divisor in place, answering self modulo divisor. A divisor of 0 leaves self as it
    /// is and answers 0. The remainder is below the divisor, so remainder × 2^64 + limb fits a
    /// `u128` and its quotient by the divisor fits a limb.
    pub(crate) fn div_rem_u64(&mut self, divisor: u64) -> u64 {
        if divisor == 0 {
            return 0;
        }
        let divisor = u128::from(divisor);
        let mut remainder = 0u128;
        for limb in self.limbs.iter_mut().rev() {
            let current = (remainder << 64) | u128::from(*limb);
            *limb = (current / divisor) as u64;
            remainder = current % divisor;
        }
        self.trim();
        remainder as u64
    }

    /// self - other, the larger minus the smaller: the caller compares the two first.
    pub(crate) fn sub(&self, other: &Nat) -> Nat {
        let mut difference = self.clone();
        difference.sub_assign(other);
        difference
    }

    /// floor(self / 2^bits).
    pub(crate) fn shr(mut self, bits: u32) -> Nat {
        let words = usize::try_from(bits / 64).unwrap_or(usize::MAX);
        let bits = bits % 64;
        if words >= self.limbs.len() {
            self.limbs.clear();
            return self;
        }
        self.limbs.drain(..words);
        if bits != 0 {
            let mut carry = 0u64;
            for limb in self.limbs.iter_mut().rev() {
                let value = *limb;
                *limb = (value >> bits) | carry;
                carry = value << (64 - bits);
            }
        }
        self.trim();
        self
    }

    /// The decimal digits, most significant first, each from 0 to 9; one 0 for 0. The value is
    /// cut into groups of 19 digits by divisions by 10^19, the largest power of ten in a limb.
    pub(crate) fn to_decimal_digits(&self) -> Vec<u8> {
        const GROUP: u64 = 10_000_000_000_000_000_000;
        let mut rest = self.clone();
        let mut groups = Vec::new();
        while !rest.limbs.is_empty() {
            groups.push(rest.div_rem_u64(GROUP));
        }
        let mut digits = Vec::with_capacity(groups.len() * 19 + 1);
        for group in groups.iter().rev() {
            let mut text = [0u8; 19];
            let mut value = *group;
            for digit in text.iter_mut().rev() {
                *digit = (value % 10) as u8;
                value /= 10;
            }
            if digits.is_empty() {
                // No leading zero: the first group from its first digit that is not 0.
                let first = text.iter().position(|&digit| digit != 0).unwrap_or(18);
                digits.extend_from_slice(text.get(first..).unwrap_or_default());
            } else {
                digits.extend_from_slice(&text);
            }
        }
        if digits.is_empty() {
            digits.push(0);
        }
        digits
    }

    pub(crate) fn from_u64(value: u64) -> Nat {
        let mut limbs = Vec::with_capacity(20);
        if value != 0 {
            limbs.push(value);
        }
        Nat { limbs }
    }

    fn trim(&mut self) {
        while self.limbs.last() == Some(&0) {
            self.limbs.pop();
        }
    }

    /// The number of bits up to the highest set one.
    pub(crate) fn bit_len(&self) -> u32 {
        let full = u32::try_from(self.limbs.len().saturating_sub(1)).unwrap_or(u32::MAX);
        match self.limbs.last() {
            Some(top) => full
                .saturating_mul(64)
                .saturating_add(64 - top.leading_zeros()),
            None => 0,
        }
    }

    /// The 128 bits from bit `shift` up: floor(self / 2^shift) modulo 2^128.
    pub(crate) fn bits_from(&self, shift: u32) -> u128 {
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

    pub(crate) fn shl(mut self, bits: u32) -> Nat {
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
        self.note();
        self
    }

    pub(crate) fn mul_small(&mut self, factor: u64) {
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
        self.note();
    }

    /// self × base^exponent, for a base from 2 to 36: by the largest power of the base that fits
    /// in a limb (10^19 for radix 10), then by the rest.
    pub(crate) fn mul_pow(&mut self, base: u64, mut exponent: u32) {
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

    pub(crate) fn add(&self, other: &Nat) -> Nat {
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
        let sum = Nat { limbs };
        sum.note();
        sum
    }

    /// self - other, for other ≤ self.
    pub(crate) fn sub_assign(&mut self, other: &Nat) {
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

    pub(crate) fn cmp(&self, other: &Nat) -> Ordering {
        self.limbs
            .len()
            .cmp(&other.limbs.len())
            .then_with(|| self.limbs.iter().rev().cmp(other.limbs.iter().rev()))
    }

    /// a + b compared with c, without forming the sum: the sign of a + b - c, from the borrows
    /// of c - a - b computed from the least significant limb up.
    pub(crate) fn cmp_sum(a: &Nat, b: &Nat, c: &Nat) -> Ordering {
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

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;

    use super::{Nat, longest_bits, reset_longest};

    /// Knuth's 64-bit linear congruential generator, the design's (decision 6, the arguments of
    /// the reference data): multiplier 6364136223846793005, increment 1442695040888963407,
    /// modulus 2^64.
    struct Lcg(u64);

    impl Lcg {
        fn next(&mut self) -> u64 {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            self.0
        }

        fn wide(&mut self) -> u128 {
            (u128::from(self.next()) << 64) | u128::from(self.next())
        }
    }

    fn nat(value: u128) -> Nat {
        let mut limbs = vec![value as u64, (value >> 64) as u64];
        while limbs.last() == Some(&0) {
            limbs.pop();
        }
        Nat { limbs }
    }

    fn value(nat: &Nat) -> u128 {
        assert!(nat.limbs.len() <= 2, "{} limbs", nat.limbs.len());
        assert_ne!(nat.limbs.last(), Some(&0), "a zero limb at the top");
        nat.limbs
            .iter()
            .rev()
            .fold(0u128, |sum, &limb| (sum << 64) | u128::from(limb))
    }

    fn decimal(nat: &Nat) -> String {
        nat.to_decimal_digits()
            .iter()
            .map(|&d| char::from(b'0' + d))
            .collect()
    }

    /// Every operation of the type against the same operation on `u128`, for 2,000 pairs of
    /// operands drawn with the seeds 1 and 2: products of two limbs, sums, differences in the
    /// order the caller checks, comparisons, quotients and remainders by a limb, shifts both
    /// ways, the decimal digits and powers.
    #[test]
    fn arithmetic_agrees_with_u128_for_2000_pairs() {
        let mut draw = Lcg(1);
        let mut other = Lcg(2);
        for _ in 0..2_000 {
            let (a, b) = (draw.next(), other.next());
            let (wide_a, wide_b) = (draw.wide(), other.wide());
            let product = u128::from(a) * u128::from(b);
            let mut small = nat(u128::from(a));
            small.mul_small(b);
            assert_eq!(value(&small), product, "{a} × {b}");
            assert_eq!(
                value(&nat(u128::from(a)).mul(&nat(u128::from(b)))),
                product,
                "{a} × {b}"
            );
            let (half_a, half_b) = (wide_a >> 1, wide_b >> 1);
            assert_eq!(value(&nat(half_a).add(&nat(half_b))), half_a + half_b);
            assert_eq!(nat(wide_a).cmp(&nat(wide_b)), wide_a.cmp(&wide_b));
            let (large, small) = if wide_a >= wide_b {
                (wide_a, wide_b)
            } else {
                (wide_b, wide_a)
            };
            assert_eq!(value(&nat(large).sub(&nat(small))), large - small);
            let divisor = b | 1;
            let mut quotient = nat(wide_a);
            let remainder = quotient.div_rem_u64(divisor);
            assert_eq!(value(&quotient), wide_a / u128::from(divisor));
            assert_eq!(u128::from(remainder), wide_a % u128::from(divisor));
            let shift = (a % 128) as u32;
            assert_eq!(value(&nat(wide_a).shr(shift)), wide_a >> shift);
            let shift = (b % 64) as u32;
            assert_eq!(
                value(&nat(u128::from(a)).shl(shift)),
                u128::from(a) << shift
            );
            assert_eq!(decimal(&nat(wide_a)), wide_a.to_string());
            let exponent = (a % 39) as u32;
            assert_eq!(value(&Nat::pow_u64(10, exponent)), 10u128.pow(exponent));
            let base = b % 1_000;
            let exponent = (a % 5) as u32;
            assert_eq!(
                value(&Nat::pow_u64(base, exponent)),
                u128::from(base).pow(exponent),
                "{base}^{exponent}"
            );
        }
    }

    /// The carries and borrows at the edges of a limb: all-ones limbs, a sum and a product that
    /// open a limb, a difference that closes one, and shifts by 0, 1, 63, 64 and 65 bits.
    #[test]
    fn carry_edges() {
        let ones = u128::MAX;
        assert_eq!(nat(ones).add(&nat(1)).limbs, vec![0, 0, 1]);
        assert_eq!(
            value(
                &Nat {
                    limbs: vec![0, 0, 1]
                }
                .sub(&nat(1))
            ),
            ones
        );
        assert_eq!(value(&nat(1u128 << 64).sub(&nat(1))), u128::from(u64::MAX));
        assert_eq!(
            value(&nat(u128::from(u64::MAX)).mul(&nat(u128::from(u64::MAX)))),
            u128::from(u64::MAX) * u128::from(u64::MAX)
        );
        assert_eq!(
            nat(ones).mul(&nat(ones)).limbs,
            vec![1, 0, u64::MAX - 1, u64::MAX]
        );
        let mut sum = nat(ones);
        sum.mul_small(u64::MAX);
        assert_eq!(sum.limbs, vec![1, u64::MAX, u64::MAX - 1]);
        for shift in [0u32, 1, 63, 64, 65] {
            let wide = Nat {
                limbs: vec![u64::MAX, u64::MAX, u64::MAX],
            };
            let back = wide.clone().shl(shift).shr(shift);
            assert_eq!(back.limbs, wide.limbs, "shl and shr by {shift}");
            if shift < 128 {
                assert_eq!(value(&nat(ones).shr(shift)), ones >> shift, "shr {shift}");
            }
            if shift < 64 {
                let top = u128::from(u64::MAX);
                assert_eq!(value(&nat(top).shl(shift)), top << shift, "shl {shift}");
            }
        }
        assert!(nat(ones).shr(128).limbs.is_empty());
        assert!(nat(ones).shr(4_000).limbs.is_empty());
        let mut quotient = nat(ones);
        assert_eq!(quotient.div_rem_u64(u64::MAX), 0);
        assert_eq!(value(&quotient), ones / u128::from(u64::MAX));
        // A divisor of 0 leaves the value as it is and answers 0: no panic.
        let mut kept = nat(12_345);
        assert_eq!(kept.div_rem_u64(0), 0);
        assert_eq!(value(&kept), 12_345);
        assert_eq!(decimal(&Nat::from_u64(0)), "0");
        assert_eq!(decimal(&nat(10u128.pow(19))), "10000000000000000000");
        assert!(Nat::pow_u64(0, 3).limbs.is_empty());
        assert_eq!(value(&Nat::pow_u64(0, 0)), 1);
        assert_eq!(value(&Nat::pow_u64(1, 4_000)), 1);
        assert_eq!(value(&Nat::pow_u64(u64::MAX, 1)), u128::from(u64::MAX));
    }

    /// The difference is the larger operand minus the smaller: the caller compares first. A
    /// difference of equal values is zero, with no limb.
    #[test]
    fn sub_takes_the_larger_first() {
        let mut draw = Lcg(3);
        for _ in 0..200 {
            let (a, b) = (nat(draw.wide()), nat(draw.wide()));
            let (large, small) = if a.cmp(&b) == Ordering::Less {
                (b, a)
            } else {
                (a, b)
            };
            let difference = large.sub(&small);
            assert_eq!(difference.add(&small).cmp(&large), Ordering::Equal);
        }
        let same = nat(u128::MAX);
        assert!(same.sub(&same).limbs.is_empty());
    }

    /// Products and powers past two limbs: 10^424 has 1,409 bits, and its decimal digits are a
    /// one and 424 zeros; the product of two powers is the power of the sum; the counter of the
    /// longest operand reads at least the longest one made.
    #[test]
    fn long_products_and_the_counter() {
        reset_longest();
        let power = Nat::pow_u64(10, 424);
        assert_eq!(power.bit_len(), 1_409);
        let text = decimal(&power);
        assert_eq!(text.len(), 425);
        assert!(text.starts_with('1') && text[1..].bytes().all(|b| b == b'0'));
        let product = Nat::pow_u64(10, 200).mul(&Nat::pow_u64(10, 224));
        assert_eq!(product.cmp(&power), Ordering::Equal);
        let mut quotient = power.clone();
        for _ in 0..22 {
            assert_eq!(quotient.div_rem_u64(10_000_000_000_000_000_000), 0);
        }
        assert_eq!(decimal(&quotient), "1000000");
        assert!(longest_bits() >= 1_409, "longest {}", longest_bits());
    }
}
