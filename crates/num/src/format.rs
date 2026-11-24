//! The digits of `Number.prototype.toFixed` (`#sec-number.prototype.tofixed`),
//! `Number.prototype.toExponential` (`#sec-number.prototype.toexponential`) and
//! `Number.prototype.toPrecision` (`#sec-number.prototype.toprecision`), from the step that
//! chooses an integer to the String the method returns.
//!
//! Each clause asks for an integer n "for which" a product of n and a power of ten "is as close
//! to zero as possible" against the Number, the larger n of two. Here that choice is exact
//! (`docs/design/engine.md`, decision 6, "Big integers in slice 4"): x is read from its bits as
//! f × 2^e with a 53-bit f, the scaled value f × 2^e × 10^s is a quotient N / D of two
//! integers of `nat.rs` (`Nat`), its floor is made by divisions by a `u64` and a right shift,
//! and the half is decided by comparing twice the remainder with D, a tie going up. No `f64`
//! operation decides a digit: the `f64` operations of this module are `to_bits`,
//! `is_sign_negative`, `abs` and one comparison with the literal 1e21, which is exact.
//!
//! No operand is longer than 2,048 bits for any finite x and any count of digits up to 100,
//! the bound of the design: the longest is 10^424, 1,409 bits, against 2^1,074 for
//! `toExponential(100)` of 2^-1074 (the lib tests read it). The counts the methods take are
//! clamped to their ranges here (0 to 100 fraction digits, 1 to 100 digits of precision), so
//! that the bound holds whatever a caller passes; the builtins have already thrown the
//! RangeError of a count outside them.

use std::cmp::Ordering;

use crate::nat::Nat;
use crate::to_string::{number_to_string, shortest_digits};

/// The most fraction digits `toFixed` and `toExponential` take, and the most digits
/// `toPrecision` takes.
const MOST_DIGITS: u32 = 100;

/// A finite Number read from its bits: the sign, and the magnitude f × 2^e with f below 2^53
/// (f is 0 for a zero).
struct Parts {
    negative: bool,
    f: u64,
    e: i32,
}

/// x read from its bits, or `None` for NaN and the infinities.
fn parts(x: f64) -> Option<Parts> {
    let bits = x.to_bits();
    let biased = (bits >> 52) & 0x7ff;
    let fraction = bits & ((1u64 << 52) - 1);
    if biased == 0x7ff {
        return None;
    }
    // biased is below 2^11, so it fits an `i32`.
    let biased = i32::try_from(biased).unwrap_or(0);
    let (f, e) = if biased == 0 {
        (fraction, -1074)
    } else {
        (fraction | (1u64 << 52), biased - 1075)
    };
    // "If x < 0": the sign of a zero is not written.
    Some(Parts {
        negative: x.is_sign_negative() && f != 0,
        f,
        e,
    })
}

/// The integer nearest to f × 2^e × 10^s, the larger of two equally near: N / D with
/// N = f × 10^max(s, 0) × 2^max(e, 0) and D = 10^max(-s, 0) × 2^max(-e, 0). The quotient q is
/// the floor of N / D, made by divisions by `u64` powers of ten and a right shift; the
/// remainder is N - q × D, and q + 1 is taken when twice the remainder is at least D.
fn nearest_integer(f: u64, e: i32, s: i32) -> Nat {
    let mut numerator = Nat::from_u64(f);
    if s > 0 {
        numerator.mul_pow(10, s.unsigned_abs());
    }
    if e > 0 {
        numerator = numerator.shl(e.unsigned_abs());
    }
    let tens = if s < 0 { s.unsigned_abs() } else { 0 };
    let twos = if e < 0 { e.unsigned_abs() } else { 0 };
    if tens == 0 && twos == 0 {
        return numerator;
    }
    let mut quotient = numerator.clone();
    let mut left = tens;
    while left > 0 {
        let chunk = left.min(19);
        let divisor = 10u64.checked_pow(chunk).unwrap_or(1);
        quotient.div_rem_u64(divisor);
        left -= chunk;
    }
    let quotient = quotient.shr(twos);
    let divisor = Nat::pow_u64(10, tens).shl(twos);
    let remainder = numerator.sub(&quotient.mul(&divisor));
    if remainder.shl(1).cmp(&divisor) == Ordering::Less {
        quotient
    } else {
        quotient.add(&Nat::from_u64(1))
    }
}

/// An estimate of floor(log10(f × 2^e)) for f > 0, from the position of the highest set bit:
/// floor(top × log10 2) with log10 2 as 78913 / 2^18, which is that floor for every top a
/// Number has; the callers correct it in any case.
fn decimal_exponent_estimate(f: u64, e: i32) -> i32 {
    let top = e + 63 - i32::try_from(f.leading_zeros()).unwrap_or(0);
    (top * 78913) >> 18
}

/// For f × 2^e > 0 and p digits: the exponent and the p decimal digits of the integer n, with
/// 10^(p-1) ≤ n < 10^p, for which n × 10^(exponent - p + 1) - x is as close to zero as
/// possible, the larger of two (the step of `toExponential` with p = fractionDigits + 1 and of
/// `toPrecision`). The exponent starts at the estimate of floor(log10 x), which is that floor
/// or one less, and moves while n has a digit too many or too few; a rounding up to 10^p moves
/// it once more, to n = 10^(p-1).
fn rounded_digits(f: u64, e: i32, p: u32) -> (Vec<u8>, i32) {
    let wanted = usize::try_from(p).unwrap_or(1);
    let digits_minus_one = i32::try_from(p).unwrap_or(1) - 1;
    let mut exponent = decimal_exponent_estimate(f, e);
    let mut digits = Vec::new();
    // The estimate is at most one too small; each pass moves the exponent toward the answer,
    // so four passes always end on it.
    for _ in 0..4 {
        digits = nearest_integer(f, e, digits_minus_one - exponent).to_decimal_digits();
        match digits.len().cmp(&wanted) {
            Ordering::Greater => exponent += 1,
            Ordering::Less => exponent -= 1,
            Ordering::Equal => break,
        }
    }
    (digits, exponent)
}

/// The digits as text.
fn push_digits(out: &mut String, digits: &[u8]) {
    for &digit in digits {
        out.push(char::from(b'0' + digit.min(9)));
    }
}

/// "e", the sign of the exponent and its digits.
fn push_exponent(out: &mut String, exponent: i32) {
    out.push('e');
    out.push(if exponent < 0 { '-' } else { '+' });
    out.push_str(&exponent.unsigned_abs().to_string());
}

/// The first digit, a "." and the others when there are others, then the exponent.
fn push_exponential(out: &mut String, digits: &[u8], exponent: i32) {
    let (first, rest) = digits.split_at_checked(1).unwrap_or((digits, &[]));
    push_digits(out, first);
    if !rest.is_empty() {
        out.push('.');
        push_digits(out, rest);
    }
    push_exponent(out, exponent);
}

/// `Number.prototype.toFixed` from step 7 on (`#sec-number.prototype.tofixed`): x with
/// `fraction_digits` digits after the decimal point. NaN, the infinities and every x with
/// abs(x) ≥ 10^21 answer `Number::toString(x)`, as the clause does; `fraction_digits` above
/// 100 is taken as 100.
pub fn to_fixed(x: f64, fraction_digits: u32) -> String {
    let Some(parts) = parts(x) else {
        return number_to_string(x);
    };
    // Step 10: "If number ≥ 10^21, then let digitString be ! ToString(𝔽(number))", with the sign
    // of step 9 before it.
    if x.abs() >= 1e21 {
        return number_to_string(x);
    }
    let fraction_digits = fraction_digits.min(MOST_DIGITS);
    let mut out = String::new();
    if parts.negative {
        out.push('-');
    }
    // Step 11.a: n, an integer for which n / 10^f - x is as close to zero as possible, the
    // larger n of two. 11.b: its digits, "0" for 0.
    let s = i32::try_from(fraction_digits).unwrap_or(0);
    let mut digits = nearest_integer(parts.f, parts.e, s).to_decimal_digits();
    // Step 11.c: with f digits after the point, at least one before it.
    let wanted = usize::try_from(fraction_digits).unwrap_or(0);
    if wanted == 0 {
        push_digits(&mut out, &digits);
        return out;
    }
    if digits.len() <= wanted {
        let mut padded = vec![0u8; wanted + 1 - digits.len()];
        padded.append(&mut digits);
        digits = padded;
    }
    let (int, frac) = digits
        .split_at_checked(digits.len() - wanted)
        .unwrap_or((&digits, &[]));
    push_digits(&mut out, int);
    out.push('.');
    push_digits(&mut out, frac);
    out
}

/// `Number.prototype.toExponential` from step 7 on (`#sec-number.prototype.toexponential`): x
/// in exponential notation with `fraction_digits` digits after the point of the significand, or
/// with as many as it takes to tell x from every other Number for `None` (the shortest digits
/// of `Number::toString`). NaN and the infinities answer `Number::toString(x)`;
/// `fraction_digits` above 100 is taken as 100.
pub fn to_exponential(x: f64, fraction_digits: Option<u32>) -> String {
    let Some(parts) = parts(x) else {
        return number_to_string(x);
    };
    let mut out = String::new();
    if parts.negative {
        out.push('-');
    }
    let (digits, exponent) = match (parts.f, fraction_digits) {
        // Step 10: x = 0, f + 1 zeros and the exponent 0.
        (0, fraction_digits) => {
            let count = fraction_digits.unwrap_or(0).min(MOST_DIGITS);
            (vec![0u8; usize::try_from(count).unwrap_or(0) + 1], 0)
        }
        // Step 11.b: the shortest digits that read back as x.
        (_, None) => {
            let (digits, n) = shortest_digits(x.abs(), 10);
            (digits, n - 1)
        }
        // Step 11.a: f + 1 digits, the nearest, the larger of two.
        (_, Some(fraction_digits)) => {
            rounded_digits(parts.f, parts.e, fraction_digits.min(MOST_DIGITS) + 1)
        }
    };
    push_exponential(&mut out, &digits, exponent);
    out
}

/// `Number.prototype.toPrecision` from step 7 on (`#sec-number.prototype.toprecision`): x with
/// `precision` significant digits, in exponential notation when its exponent is below -6 or
/// not below `precision`, in fixed notation otherwise. NaN and the infinities answer
/// `Number::toString(x)`; `precision` is taken in the range from 1 to 100.
pub fn to_precision(x: f64, precision: u32) -> String {
    let Some(parts) = parts(x) else {
        return number_to_string(x);
    };
    let precision = precision.clamp(1, MOST_DIGITS);
    let p = i32::try_from(precision).unwrap_or(1);
    let mut out = String::new();
    if parts.negative {
        out.push('-');
    }
    let (digits, exponent) = if parts.f == 0 {
        // Step 10: x = 0, p zeros and the exponent 0.
        (vec![0u8; usize::try_from(precision).unwrap_or(1)], 0)
    } else {
        // Step 11.a: p digits, the nearest, the larger of two.
        rounded_digits(parts.f, parts.e, precision)
    };
    // Step 11.c: the exponential form.
    if exponent < -6 || exponent >= p {
        push_exponential(&mut out, &digits, exponent);
        return out;
    }
    // Step 12: e = p - 1, the digits alone.
    if exponent == p - 1 {
        push_digits(&mut out, &digits);
        return out;
    }
    if exponent >= 0 {
        // Step 13: e + 1 digits, ".", the other p - (e + 1).
        let split = usize::try_from(exponent + 1).unwrap_or(0);
        let (int, frac) = digits.split_at_checked(split).unwrap_or((&digits, &[]));
        push_digits(&mut out, int);
        out.push('.');
        push_digits(&mut out, frac);
    } else {
        // Step 14: "0.", -(e + 1) zeros and the digits.
        out.push_str("0.");
        for _ in 0..(-(exponent + 1)) {
            out.push('0');
        }
        push_digits(&mut out, &digits);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{to_exponential, to_fixed, to_precision};
    use crate::nat::{longest_bits, reset_longest};

    /// A call of the six extremes and its name.
    type Extreme = (&'static str, fn() -> String);

    /// The design's bound: after the six extremes (101 digits of 2^-1074 and of the largest
    /// Number in both exponent forms, 100 fraction digits of 10^20 and of 2^-1074) the longest
    /// operand is at most 2,048 bits, and at least the 1,409 of 10^424. Each reading is printed.
    #[test]
    fn no_operand_passes_2048_bits_at_the_extremes() {
        let extremes: [Extreme; 6] = [
            ("toExponential(100) of 2^-1074", || {
                to_exponential(5e-324, Some(100))
            }),
            ("toPrecision(100) of 2^-1074", || to_precision(5e-324, 100)),
            ("toExponential(100) of the largest Number", || {
                to_exponential(f64::MAX, Some(100))
            }),
            ("toPrecision(100) of the largest Number", || {
                to_precision(f64::MAX, 100)
            }),
            ("toFixed(100) of 10^20", || to_fixed(1e20, 100)),
            ("toFixed(100) of 2^-1074", || to_fixed(5e-324, 100)),
        ];
        let mut longest = 0;
        for (name, call) in extremes {
            reset_longest();
            let text = call();
            println!(
                "{name}: longest operand {} bits, {} code units",
                longest_bits(),
                text.len()
            );
            longest = longest.max(longest_bits());
        }
        println!("longest operand of the six: {longest} bits, the bound 2048");
        assert!(longest <= 2_048, "{longest}");
        assert!(longest >= 1_409, "{longest}");
    }

    /// Every binary exponent of a Number, with the mantissa all ones and the smallest one: no
    /// operand passes 2,048 bits for 100 digits of `toPrecision` and `toExponential`, and for
    /// 100 fraction digits of `toFixed` below 10^21.
    #[test]
    fn no_operand_passes_2048_bits_for_any_exponent() {
        reset_longest();
        for biased in 0u64..0x7ff {
            for fraction in [0u64, (1 << 52) - 1, 1] {
                let x = f64::from_bits((biased << 52) | fraction);
                let _ = to_precision(x, 100);
                let _ = to_exponential(x, Some(100));
                if x < 1e21 {
                    let _ = to_fixed(x, 100);
                }
            }
        }
        println!(
            "longest operand over every exponent: {} bits",
            longest_bits()
        );
        assert!(longest_bits() <= 2_048, "{}", longest_bits());
    }
}
