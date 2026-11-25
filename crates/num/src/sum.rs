//! The exact sum of `Math.sumPrecise ( items )` (ECMA-262 21.3.2.34, `#sec-math.sumprecise`), by
//! the design's rule (`docs/design/engine.md`, decision 6, "`Math.sumPrecise`"): the draft sums in
//! ℝ and rounds once, so [`Sum`] keeps the sum exactly and rounds it in [`Sum::finish`].
//!
//! A finite Number is an integer of at most 53 bits times a power of two no lower than 2^-1074,
//! so it is a whole count of units of 2^-1074 below 2^2,098. [`Sum`] holds two unsigned integers of
//! 34 limbs of 64 bits (2,176 bits), one for the positive items and one for the negative items,
//! counting those units (a fixed-size superaccumulator, of the family the draft's note names:
//! Neal, "Fast exact summation using small and large superaccumulators"). Adding an item adds its
//! mantissa shifted by its exponent, which touches two adjacent limbs and carries upward as a
//! binary counter does. Fewer than 2^53 items are summed ([`MAX_ITEMS`], the draft's count limit),
//! so each accumulator stays below 2^2,151 and no carry leaves the top limb.
//!
//! The draft's state (not-a-number, plus-infinity, minus-infinity, minus-zero, finite) is kept as
//! the clause writes it and decides NaN, the infinities and -0; the accumulators decide a finite
//! sum. The `f64` operations are `to_bits`, `from_bits`, `is_nan`, `is_infinite`,
//! `is_sign_negative` and comparisons, each fixed by IEEE 754 or the Rust reference; the result
//! is built by `from_bits` from the rounded mantissa and exponent, so nothing is multiplied by a
//! power of two. No `f64` is cast to an integer.

/// The draft's limit on the count of items: step 7.b.i of `#sec-math.sumprecise` throws a
/// `RangeError` when `count` ≥ 2^53 - 1 and another item comes.
pub const MAX_ITEMS: u64 = (1 << 53) - 1;

/// The limbs of each accumulator: 34 × 64 = 2,176 bits, above the 2,151 bits that 2^53 items
/// below 2^2,098 units each can reach.
const LIMBS: usize = 34;

/// The bits of a limb.
const LIMB_BITS: u32 = 64;

/// The stored bits of a binary64 significand.
const STORED_BITS: u32 = 52;

/// The mask of the stored bits of a significand.
const STORED_MASK: u64 = (1 << STORED_BITS) - 1;

/// The implicit bit of a normal Number's significand, 2^52.
const IMPLICIT_BIT: u64 = 1 << STORED_BITS;

/// The biased exponent field of the infinities, 2,047.
const INFINITE_EXPONENT: u64 = 0x7ff;

/// The sign bit of a binary64.
const SIGN_BIT: u64 = 1 << 63;

/// The state of the draft (`#sec-math.sumprecise`, steps 3 and 7.b.iv).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    /// not-a-number: a NaN, or both infinities, was seen.
    NotANumber,
    /// plus-infinity.
    PlusInfinity,
    /// minus-infinity.
    MinusInfinity,
    /// minus-zero: every item so far was -0, or there was none.
    MinusZero,
    /// finite: an item that was not -0 was seen, and no NaN and no infinity.
    Finite,
}

/// The exact running sum of `Math.sumPrecise`: two accumulators of [`LIMBS`] limbs, positive and
/// negative items apart, in units of 2^-1074, and the draft's state. It lives on the frame of the
/// call that sums; its size does not depend on the count of items.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sum {
    /// The sum of the magnitudes of the positive items, least significant limb first.
    positive: [u64; LIMBS],
    /// The sum of the magnitudes of the negative items, least significant limb first.
    negative: [u64; LIMBS],
    /// The draft's state.
    state: State,
}

impl Default for Sum {
    fn default() -> Self {
        Self::new()
    }
}

impl Sum {
    /// The sum of no item: the state minus-zero (step 3, "Let state be minus-zero") and
    /// accumulators of zero (step 4, "Let sum be 0").
    #[must_use]
    pub const fn new() -> Self {
        Sum {
            positive: [0; LIMBS],
            negative: [0; LIMBS],
            state: State::MinusZero,
        }
    }

    /// Step 7.b.iv of `#sec-math.sumprecise` for the Number `x`:
    ///
    /// - in the state not-a-number nothing changes;
    /// - NaN sets not-a-number;
    /// - +∞ sets not-a-number after minus-infinity and plus-infinity otherwise, -∞ the other way;
    /// - any other `x` that is not -0, while the state is minus-zero or finite, sets finite and is
    ///   added to the sum. So +0 takes the state out of minus-zero, and -0, or a finite `x` after
    ///   an infinity, changes nothing.
    ///
    /// At most [`MAX_ITEMS`] items are added to one `Sum` (the caller counts them, step 7.b.i);
    /// past that a carry could leave the top limb and be lost.
    pub fn add(&mut self, x: f64) {
        self.state = match self.state {
            State::NotANumber => State::NotANumber,
            _ if x.is_nan() => State::NotANumber,
            State::MinusInfinity if x.is_infinite() && !x.is_sign_negative() => State::NotANumber,
            State::PlusInfinity if x.is_infinite() && x.is_sign_negative() => State::NotANumber,
            _ if x.is_infinite() && x.is_sign_negative() => State::MinusInfinity,
            _ if x.is_infinite() => State::PlusInfinity,
            State::MinusZero | State::Finite if x == 0.0 && x.is_sign_negative() => self.state,
            State::MinusZero | State::Finite => {
                self.add_finite(x.to_bits());
                State::Finite
            }
            state @ (State::PlusInfinity | State::MinusInfinity) => state,
        };
    }

    /// Adds the finite Number whose bits are `bits` to the accumulator of its sign: its
    /// significand, the stored bits with the implicit bit of a normal Number, shifted left by its
    /// exponent field less one (none for a subnormal, whose field is 0), counts units of 2^-1074.
    fn add_finite(&mut self, bits: u64) {
        let field = (bits >> STORED_BITS) & INFINITE_EXPONENT;
        let stored = bits & STORED_MASK;
        let (significand, shift) = if field == 0 {
            (stored, 0)
        } else {
            (stored | IMPLICIT_BIT, field - 1)
        };
        let limbs = if bits & SIGN_BIT == 0 {
            &mut self.positive
        } else {
            &mut self.negative
        };
        // The shift is at most 2,045 (field 2,046), so the limb is at most 31 and the next one
        // exists.
        let limb = usize::try_from(shift / u64::from(LIMB_BITS)).unwrap_or(LIMBS);
        let offset = (shift % u64::from(LIMB_BITS)) as u32;
        let low = significand << offset;
        // The bits shifted out of the low limb: none at offset 0, where a shift by 64 would not be
        // defined.
        let high = if offset == 0 {
            0
        } else {
            significand >> (LIMB_BITS - offset)
        };
        add_at(limbs, limb, low);
        add_at(limbs, limb.saturating_add(1), high);
    }

    /// Steps 8 to 12 of `#sec-math.sumprecise`: NaN, +∞, -∞ or -0 by the state, or, for the state
    /// finite, 𝔽(sum): the larger accumulator less the smaller, with the sign of the larger,
    /// rounded to the nearest Number with ties to even (the 53 bits from the highest set bit, the
    /// next bit, and a sticky bit of all below), ±∞ when it rounds to 2^1,024 or more. An exact
    /// zero is +0 (𝔽(0)).
    #[must_use]
    pub fn finish(&self) -> f64 {
        match self.state {
            State::NotANumber => f64::NAN,
            State::PlusInfinity => f64::INFINITY,
            State::MinusInfinity => f64::NEG_INFINITY,
            State::MinusZero => -0.0,
            State::Finite => {
                let (magnitude, sign) = if less(&self.positive, &self.negative) {
                    (difference(&self.negative, &self.positive), SIGN_BIT)
                } else {
                    (difference(&self.positive, &self.negative), 0)
                };
                f64::from_bits(sign | round(&magnitude))
            }
        }
    }
}

/// Adds `value` to `limbs` at limb `at` and carries upward. A carry out of the top limb, which
/// [`MAX_ITEMS`] rules out, is dropped.
fn add_at(limbs: &mut [u64; LIMBS], at: usize, value: u64) {
    let mut carry = value;
    for limb in limbs.iter_mut().skip(at) {
        if carry == 0 {
            return;
        }
        let (sum, over) = limb.overflowing_add(carry);
        *limb = sum;
        carry = u64::from(over);
    }
}

/// Whether the integer `a` is below the integer `b`, compared from the top limb.
fn less(a: &[u64; LIMBS], b: &[u64; LIMBS]) -> bool {
    for (x, y) in a.iter().rev().zip(b.iter().rev()) {
        if x != y {
            return x < y;
        }
    }
    false
}

/// `a - b` for `a` ≥ `b`: 34 limb subtractions with borrow.
fn difference(a: &[u64; LIMBS], b: &[u64; LIMBS]) -> [u64; LIMBS] {
    let mut out = [0; LIMBS];
    let mut borrow = 0;
    for ((out, x), y) in out.iter_mut().zip(a).zip(b) {
        let (d, under) = x.overflowing_sub(*y);
        let (d, under_borrow) = d.overflowing_sub(borrow);
        *out = d;
        borrow = u64::from(under || under_borrow);
    }
    out
}

/// The bit `index` of `limbs` (0 the least significant), 0 past the top.
fn bit(limbs: &[u64; LIMBS], index: u64) -> u64 {
    let limb = usize::try_from(index / u64::from(LIMB_BITS)).unwrap_or(LIMBS);
    limbs
        .get(limb)
        .map_or(0, |value| (value >> (index % u64::from(LIMB_BITS))) & 1)
}

/// The 64 bits of `limbs` from bit `index` up (bits past the top read 0).
fn bits_from(limbs: &[u64; LIMBS], index: u64) -> u64 {
    let limb = usize::try_from(index / u64::from(LIMB_BITS)).unwrap_or(LIMBS);
    let offset = (index % u64::from(LIMB_BITS)) as u32;
    let low = limbs.get(limb).copied().unwrap_or(0) >> offset;
    let high = if offset == 0 {
        0
    } else {
        limbs.get(limb.saturating_add(1)).copied().unwrap_or(0) << (LIMB_BITS - offset)
    };
    low | high
}

/// Whether any bit of `limbs` below bit `index` is set.
fn any_below(limbs: &[u64; LIMBS], index: u64) -> bool {
    let whole = usize::try_from(index / u64::from(LIMB_BITS)).unwrap_or(LIMBS);
    let offset = (index % u64::from(LIMB_BITS)) as u32;
    if limbs.iter().take(whole).any(|&limb| limb != 0) {
        return true;
    }
    // The bits below `offset` of the limb that holds bit `index`; none when `offset` is 0.
    let partial = limbs.get(whole).copied().unwrap_or(0);
    offset != 0 && partial << (LIMB_BITS - offset) != 0
}

/// The bits of the Number nearest the non-negative integer `magnitude` of units of 2^-1074, ties
/// to even, without the sign: below 2^53 units the Number is exact and its bits are the integer
/// itself (a subnormal below 2^52, a Number of exponent field 1 from 2^52); above, the 53 bits
/// from the highest set bit `h` are the significand, rounded up when the next bit is set and
/// either a lower bit is set or the significand is odd, and the exponent field is `h - 51`
/// (`h - 52` more after a rounding that carries to 2^53); a field of 2,047 or more is ∞.
fn round(magnitude: &[u64; LIMBS]) -> u64 {
    let Some(highest) = highest_bit(magnitude) else {
        // 𝔽(0) is +0.
        return 0;
    };
    let significand_bits = u64::from(STORED_BITS) + 1;
    if highest < significand_bits {
        return bits_from(magnitude, 0);
    }
    // The shift that leaves 53 bits: at least 1.
    let mut shift = highest - u64::from(STORED_BITS);
    let mut significand = bits_from(magnitude, shift) & (IMPLICIT_BIT | STORED_MASK);
    let half = bit(magnitude, shift - 1) == 1;
    let sticky = any_below(magnitude, shift - 1);
    if half && (sticky || significand & 1 == 1) {
        significand += 1;
        if significand == IMPLICIT_BIT << 1 {
            significand = IMPLICIT_BIT;
            shift += 1;
        }
    }
    // The value is significand × 2^shift units, (2^52 + stored) × 2^(shift - 1074), whose
    // exponent field is shift + 1.
    let field = shift + 1;
    if field >= INFINITE_EXPONENT {
        return INFINITE_EXPONENT << STORED_BITS;
    }
    (field << STORED_BITS) | (significand & STORED_MASK)
}

/// The index of the highest set bit of `limbs`, or `None` for zero.
fn highest_bit(limbs: &[u64; LIMBS]) -> Option<u64> {
    let (index, limb) = limbs
        .iter()
        .enumerate()
        .rev()
        .find(|(_, limb)| **limb != 0)?;
    let index = u64::try_from(index).ok()?;
    Some(index * u64::from(LIMB_BITS) + u64::from(LIMB_BITS - 1 - limb.leading_zeros()))
}
