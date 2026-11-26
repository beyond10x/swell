//! The Number value of a numeric literal and of a string.
//!
//! - [`numeric_literal`]: the grammar of `NumericLiteral` (`#sec-literals-numeric-literals`)
//!   without its BigInt forms, and its value, `NumericValue` (`#sec-numericvalue`), with the form
//!   the parser needs for the early error of `#sec-literals-numeric-literals-early-errors`.
//! - [`string_to_number_trimmed`]: `StringToNumber` (`#sec-stringtonumber`) of a string whose
//!   leading and trailing `StrWhiteSpace` the caller has removed: the grammar of
//!   `StringNumericLiteral` (`#sec-tonumber-applied-to-the-string-type`) and its value,
//!   `StringNumericValue` (`#sec-runtime-semantics-stringnumericvalue`).
//!
//! The mathematical value is rounded to a Number as `#sec-ecmascript-language-types-number-type`
//! says, to nearest with ties to even. `RoundMVResult` (`#sec-roundmvresult`) also allows replacing
//! the digits after the 20th significant one by zeros; this crate does not, so a decimal literal
//! gets the correctly rounded value. The decimal digits are converted by `str::parse::<f64>`, which
//! rounds that way (design, decision 6); the grammar is checked here first, so `parse` sees only
//! digits, a point, an exponent and a sign. A binary, octal or hexadecimal literal is rounded here,
//! exactly, from its bits.
//!
//! Both functions take the code units of the text, Latin-1 (`u8`) or UTF-16 (`u16`); every
//! character of either grammar is ASCII, so a unit above 0x7F makes the text no literal.

use alloc::string::String;
use alloc::vec::Vec;

/// Which production of `NumericLiteral` a literal is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LiteralForm {
    /// `DecimalLiteral` whose `DecimalIntegerLiteral`, if any, is not a
    /// `NonOctalDecimalIntegerLiteral`: `0`, `12_3.5e-1`, `.5`.
    Decimal,
    /// `DecimalLiteral` whose `DecimalIntegerLiteral` is a `NonOctalDecimalIntegerLiteral`, a
    /// zero followed by digits one of which is 8 or 9: `08`, `019.5`.
    NonOctalDecimal,
    /// `BinaryIntegerLiteral`: `0b101`.
    Binary,
    /// `OctalIntegerLiteral`: `0o17`.
    Octal,
    /// `HexIntegerLiteral`: `0xff`.
    Hex,
    /// `LegacyOctalIntegerLiteral`, a zero followed by octal digits: `017`.
    LegacyOctal,
}

impl LiteralForm {
    /// Whether the literal is a Syntax Error in strict code
    /// (`#sec-literals-numeric-literals-early-errors`: `LegacyOctalIntegerLiteral` and
    /// `NonOctalDecimalIntegerLiteral`).
    pub fn is_legacy(self) -> bool {
        matches!(
            self,
            LiteralForm::LegacyOctal | LiteralForm::NonOctalDecimal
        )
    }
}

/// A numeric literal: its form and its `NumericValue`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NumericLiteral {
    pub form: LiteralForm,
    pub value: f64,
}

/// The form and value of `text` when the whole of it is a `NumericLiteral` that is not a BigInt
/// literal; `None` otherwise.
///
/// A literal with the `BigIntLiteralSuffix` `n` is `None`: `BigInt` is slice 12's. Whether the
/// character after a literal is allowed (`#sec-literals-numeric-literals`, "The SourceCharacter
/// immediately following a NumericLiteral must not be an IdentifierStart or DecimalDigit") is the
/// lexer's to check; it passes the literal's text alone.
pub fn numeric_literal<U: Copy + Into<u32>>(text: &[U]) -> Option<NumericLiteral> {
    let s = ascii(text)?;
    let s = s.as_slice();
    if let Some((form, bits, digits)) = non_decimal_prefix(s) {
        let value = integer_value(digits, bits, true)?;
        return Some(NumericLiteral { form, value });
    }
    let (form, integer_end) = match s {
        [b'0', next, ..] if next.is_ascii_digit() => {
            let run = s.iter().skip(1).take_while(|b| b.is_ascii_digit()).count() + 1;
            let digits = s.get(..run)?;
            if digits.iter().all(|b| is_octal(*b)) {
                // `LegacyOctalIntegerLiteral`: nothing may follow it in the same literal.
                if run != s.len() {
                    return None;
                }
                let value = integer_value(digits, 3, false)?;
                return Some(NumericLiteral {
                    form: LiteralForm::LegacyOctal,
                    value,
                });
            }
            (LiteralForm::NonOctalDecimal, run)
        }
        [b'0', ..] => (LiteralForm::Decimal, 1),
        _ => (LiteralForm::Decimal, digits_end(s, is_decimal, true)),
    };
    if !decimal_tail_is_valid(s, integer_end, true) {
        return None;
    }
    let value = decimal_value(None, s)?;
    Some(NumericLiteral { form, value })
}

/// `StringToNumber` of a string whose leading and trailing `StrWhiteSpace` (white space and line
/// terminators, `#sec-tonumber-applied-to-the-string-type`) is already removed: the
/// `StringNumericValue` of the text when it is a `StrNumericLiteral`, +0 when it is empty, NaN
/// otherwise.
///
/// White space left inside the text, or at either end, makes it NaN: the caller trims.
pub fn string_to_number_trimmed<U: Copy + Into<u32>>(text: &[U]) -> f64 {
    let Some(s) = ascii(text) else {
        return f64::NAN;
    };
    let s = s.as_slice();
    if s.is_empty() {
        return 0.0;
    }
    if let Some((_, bits, digits)) = non_decimal_prefix(s) {
        return integer_value(digits, bits, false).unwrap_or(f64::NAN);
    }
    let (sign, unsigned) = match s {
        [b'+', rest @ ..] => (Some(b'+'), rest),
        [b'-', rest @ ..] => (Some(b'-'), rest),
        _ => (None, s),
    };
    if unsigned == b"Infinity" {
        return if sign == Some(b'-') {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        };
    }
    let integer_end = digits_end(unsigned, is_decimal, false);
    if !decimal_tail_is_valid(unsigned, integer_end, false) {
        return f64::NAN;
    }
    decimal_value(sign, unsigned).unwrap_or(f64::NAN)
}

/// The units of `text` as ASCII bytes, or `None` when one of them is not ASCII.
fn ascii<U: Copy + Into<u32>>(text: &[U]) -> Option<Vec<u8>> {
    text.iter()
        .map(|&unit| {
            let unit: u32 = unit.into();
            u8::try_from(unit).ok().filter(u8::is_ascii)
        })
        .collect()
}

fn is_decimal(b: u8) -> bool {
    b.is_ascii_digit()
}

fn is_octal(b: u8) -> bool {
    (b'0'..=b'7').contains(&b)
}

fn is_binary(b: u8) -> bool {
    b == b'0' || b == b'1'
}

fn is_hex(b: u8) -> bool {
    b.is_ascii_hexdigit()
}

/// For a `NonDecimalIntegerLiteral` prefix (`0b`, `0o`, `0x`, either case): the form, the bits
/// per digit and the text after the prefix.
fn non_decimal_prefix(s: &[u8]) -> Option<(LiteralForm, u32, &[u8])> {
    match s {
        [b'0', b'b' | b'B', rest @ ..] => Some((LiteralForm::Binary, 1, rest)),
        [b'0', b'o' | b'O', rest @ ..] => Some((LiteralForm::Octal, 3, rest)),
        [b'0', b'x' | b'X', rest @ ..] => Some((LiteralForm::Hex, 4, rest)),
        _ => None,
    }
}

/// The end of the digits at the start of `s`, 0 when there is none: digits that `is_digit`
/// accepts, and with `separators` single `_` between two of them (`DecimalDigits[+Sep]` and its
/// kin). A `_` that is not followed by a digit is not taken, so it is left over.
fn digits_end(s: &[u8], is_digit: fn(u8) -> bool, separators: bool) -> usize {
    let mut end = 0;
    let mut after_digit = false;
    for (i, &b) in s.iter().enumerate() {
        if is_digit(b) {
            end = i + 1;
            after_digit = true;
        } else if b == b'_' && separators && after_digit {
            after_digit = false;
        } else {
            break;
        }
    }
    end
}

/// Whether `s`, whose integer digits end at `integer_end` (0: none), ends as a decimal literal
/// does: an optional `.` with optional digits (required when there are no integer digits), then
/// an optional `ExponentPart`, then nothing.
fn decimal_tail_is_valid(s: &[u8], integer_end: usize, separators: bool) -> bool {
    let Some(mut rest) = s.get(integer_end..) else {
        return false;
    };
    let mut has_digits = integer_end > 0;
    if let [b'.', after @ ..] = rest {
        let end = digits_end(after, is_decimal, separators);
        has_digits |= end > 0;
        rest = after.get(end..).unwrap_or_default();
    }
    if !has_digits {
        return false;
    }
    if let [b'e' | b'E', after @ ..] = rest {
        let after = match after {
            [b'+' | b'-', signed @ ..] => signed,
            _ => after,
        };
        let end = digits_end(after, is_decimal, separators);
        if end == 0 {
            return false;
        }
        rest = after.get(end..).unwrap_or_default();
    }
    rest.is_empty()
}

/// The value of a checked decimal literal `s` with an optional sign, separators removed,
/// rounded to nearest, ties to even, by `str::parse`.
fn decimal_value(sign: Option<u8>, s: &[u8]) -> Option<f64> {
    let mut text = String::with_capacity(s.len() + 1);
    text.extend(sign.map(char::from));
    text.extend(s.iter().filter(|&&b| b != b'_').map(|&b| char::from(b)));
    text.parse::<f64>().ok()
}

/// The value of the digits `s` in the radix 2^`bits` (1, 3 or 4), with `_` between digits when
/// `separators`; `None` when `s` is not such digits.
fn integer_value(s: &[u8], bits: u32, separators: bool) -> Option<f64> {
    let is_digit: fn(u8) -> bool = match bits {
        1 => is_binary,
        3 => is_octal,
        _ => is_hex,
    };
    let end = digits_end(s, is_digit, separators);
    if end == 0 || end != s.len() {
        return None;
    }
    let mut value = BinaryValue::default();
    for &b in s.iter().filter(|&&b| b != b'_') {
        let digit = char::from(b).to_digit(16)?;
        value.push(digit, bits);
    }
    Some(value.round())
}

/// An integer read bit by bit: its leading 64 bits, how many bits followed them, and whether one
/// of those was set.
#[derive(Default)]
struct BinaryValue {
    leading: u64,
    dropped: u64,
    sticky: bool,
}

impl BinaryValue {
    /// Appends the low `bits` bits of `digit`, most significant first.
    fn push(&mut self, digit: u32, bits: u32) {
        for i in (0..bits).rev() {
            let bit = u64::from((digit >> i) & 1);
            if self.leading >> 63 == 0 {
                self.leading = (self.leading << 1) | bit;
            } else {
                self.dropped = self.dropped.saturating_add(1);
                self.sticky |= bit != 0;
            }
        }
    }

    /// The Number nearest to the integer, ties to even; +∞ beyond the largest finite Number
    /// (`#sec-ecmascript-language-types-number-type`).
    fn round(&self) -> f64 {
        let m = self.leading;
        if m == 0 {
            return 0.0;
        }
        let top = u64::from(63 - m.leading_zeros());
        let (mut mantissa, mut exponent) = if top <= 52 {
            (m << (52 - top), top)
        } else {
            let shift = top - 52;
            let mut mantissa = m >> shift;
            let rest = m & ((1u64 << shift) - 1);
            let half = 1u64 << (shift - 1);
            let up = rest > half || (rest == half && (self.sticky || mantissa & 1 == 1));
            if up {
                mantissa += 1;
            }
            (mantissa, top)
        };
        if mantissa == 1u64 << 53 {
            mantissa >>= 1;
            exponent += 1;
        }
        let exponent = exponent.saturating_add(self.dropped);
        if exponent > 1023 {
            return f64::INFINITY;
        }
        f64::from_bits(((exponent + 1023) << 52) | (mantissa & ((1u64 << 52) - 1)))
    }
}
