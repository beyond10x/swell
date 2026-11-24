//! Reading a string in a radix: the steps of `parseFloat` (ECMA-262 19.2.4,
//! `#sec-parsefloat-string`) after ToString, and of `parseInt` (19.2.5,
//! `#sec-parseint-string-radix`) after ToString and ToInt32, over the code units of a String,
//! Latin-1 (`u8`) or UTF-16 (`u16`), read in place.
//!
//! - [`parse_float`]: the longest prefix after the leading `StrWhiteSpace` that is a
//!   `StrDecimalLiteral`, and its `StringNumericValue`; NaN when there is none.
//! - [`parse_int`]: the radix rule, the sign and the longest run of digits of the radix, and the
//!   integer they write; NaN when there is none or the radix is outside 2 to 36.
//! - [`is_str_white_space`]: `StrWhiteSpaceChar` (`#sec-tonumber-applied-to-the-string-type`).
//!
//! Rounding (`docs/design/engine.md`, decision 6, "Parsing" and "parseInt for a radix other than
//! 10"). Decimal digits are copied to an ASCII buffer and converted by `str::parse::<f64>`,
//! which rounds to nearest, ties to even (`dec2flt`); the scanner here has checked the grammar
//! first, so `parse` sees only digits, a point, an exponent and signs, never Rust's own `inf`
//! or `nan` (`Infinity` is read here). The draft's allowance to replace the digits after the
//! 20th by 0 is not taken. In radix 2, 4, 8, 16 and 32 the run is converted exactly and rounded
//! once to nearest, ties to even: the leading 128 bits of the value in a `u128`, the lower bits
//! as one sticky bit and their count as the exponent; the 53 bits are made exact by hand before
//! the one `as f64` of a `u128`, and the power of two is one exact multiplication. In every
//! other radix the value is `acc * radix + digit` accumulated in `f64`, exact while `acc` is
//! below 2^53 and an approximation after, as 19.2.5 step 16 allows; IEEE 754 multiplication and
//! addition give it the same on every platform.
//!
//! Work: one pass over the code units read, and for decimal digits one buffer of at most the
//! length of the text, reserved before the digits are copied. This crate takes no budget: the
//! engine charges a step per code unit and the bytes of the buffer before it calls.

/// Whether the code unit `unit` is a `StrWhiteSpaceChar`
/// (`#sec-tonumber-applied-to-the-string-type`): `WhiteSpace` (12.2: TAB, VT, FF, ZWNBSP and
/// every code point of General_Category `Zs`) or `LineTerminator` (12.3: LF, CR, LS, PS). `Zs`
/// is the 17 code points of Unicode 17.0.0 (`data/unicode/17.0.0`, the table `zs.rs` of
/// `swell-unicode`), all in the Basic Multilingual Plane; this crate depends on no other, so it
/// writes them out, and a test of `swell` holds the two sets equal over every code unit.
pub fn is_str_white_space(unit: u32) -> bool {
    // TAB, LF, VT, FF and CR are 0x09 to 0x0D.
    const SINGLE: [u32; 9] = [
        0x0020, 0x00A0, 0x1680, 0x2028, 0x2029, 0x202F, 0x205F, 0x3000, 0xFEFF,
    ];
    (0x0009..=0x000D).contains(&unit) || (0x2000..=0x200A).contains(&unit) || SINGLE.contains(&unit)
}

/// The code unit at `index` of `text` as an ASCII byte; `None` past the end or for a unit
/// above 0x7F, which no character of either grammar is.
fn ascii_at<U: Copy + Into<u32>>(text: &[U], index: usize) -> Option<u8> {
    let unit: u32 = (*text.get(index)?).into();
    u8::try_from(unit).ok().filter(u8::is_ascii)
}

/// `text` from its first code unit that is not `StrWhiteSpace`: TrimString(string, start)
/// (`#sec-trimstring`).
fn trim_start<U: Copy + Into<u32>>(text: &[U]) -> &[U] {
    let start = text
        .iter()
        .position(|&unit| !is_str_white_space(unit.into()))
        .unwrap_or(text.len());
    text.get(start..).unwrap_or_default()
}

/// The count of decimal digits at `from` in `text`.
fn decimal_run<U: Copy + Into<u32>>(text: &[U], from: usize) -> usize {
    text.get(from..)
        .unwrap_or_default()
        .iter()
        .take_while(|&&unit| {
            let unit: u32 = unit.into();
            (u32::from(b'0')..=u32::from(b'9')).contains(&unit)
        })
        .count()
}

/// Whether `text` starts with the ASCII `word` at `from`.
fn has_word<U: Copy + Into<u32>>(text: &[U], from: usize, word: &[u8]) -> bool {
    word.iter()
        .enumerate()
        .all(|(i, &byte)| from.checked_add(i).and_then(|at| ascii_at(text, at)) == Some(byte))
}

/// `parseFloat` from step 2 (`#sec-parsefloat-string`): the Number of the longest prefix of
/// `text`, after its leading `StrWhiteSpace`, that satisfies the syntax of a
/// `StrDecimalLiteral`, or NaN when no prefix does. -0 keeps its sign.
///
/// `StrDecimalLiteral` (`#sec-tonumber-applied-to-the-string-type`) is an optional sign, then
/// `Infinity`, or `DecimalDigits`, `.` and optional `DecimalDigits`, or `.` and
/// `DecimalDigits`, or `DecimalDigits`, each of the last three with an optional `ExponentPart`
/// (`e` or `E`, an optional sign, `DecimalDigits`); no separator `_`, no hex, binary or octal
/// form. An `e` with no digit after it is not part of the prefix.
pub fn parse_float<U: Copy + Into<u32>>(text: &[U]) -> f64 {
    // 2, 3. trimmedString, trimmed: the code units from the first that is not white space. A
    // code point outside the BMP is no character of the grammar, so its units end the prefix
    // as the code point would.
    let text = trim_start(text);
    // 4. trimmedPrefix, the longest prefix that is a StrDecimalLiteral.
    let mut end = 0;
    let negative = match ascii_at(text, 0) {
        Some(b'-') => {
            end = 1;
            true
        }
        Some(b'+') => {
            end = 1;
            false
        }
        _ => false,
    };
    if has_word(text, end, b"Infinity") {
        // StrUnsignedDecimalLiteral ::: Infinity, whose value is +∞ (-∞ with the sign).
        return if negative {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        };
    }
    let integer = decimal_run(text, end);
    end += integer;
    let mut fraction = 0;
    if ascii_at(text, end) == Some(b'.') {
        fraction = decimal_run(text, end + 1);
        // `.` belongs to the prefix when a digit stands on either side of it.
        if integer > 0 || fraction > 0 {
            end += 1 + fraction;
        }
    }
    // 4. "If there is no such prefix, return NaN."
    if integer == 0 && fraction == 0 {
        return f64::NAN;
    }
    if matches!(ascii_at(text, end), Some(b'e' | b'E')) {
        let mut after = end + 1;
        if matches!(ascii_at(text, after), Some(b'+' | b'-')) {
            after += 1;
        }
        let exponent = decimal_run(text, after);
        if exponent > 0 {
            end = after + exponent;
        }
    }
    // 5, 6, 7. The StringNumericValue of the prefix, rounded by `str::parse`.
    decimal_value(text.get(..end).unwrap_or_default())
}

/// The value of the checked ASCII prefix `prefix`, copied to a buffer of its length, reserved
/// first, and rounded to nearest, ties to even, by `str::parse::<f64>`.
fn decimal_value<U: Copy + Into<u32>>(prefix: &[U]) -> f64 {
    let mut digits = String::with_capacity(prefix.len());
    for index in 0..prefix.len() {
        let Some(byte) = ascii_at(prefix, index) else {
            return f64::NAN;
        };
        digits.push(char::from(byte));
    }
    digits.parse::<f64>().unwrap_or(f64::NAN)
}

/// The value of the code unit `unit` as a digit of radix `radix` (2 to 36): `0` to `9`, then the
/// letters `A` to `Z` and `a` to `z` for 10 to 35 (19.2.5, step 16); `None` for any other unit
/// or a value not below `radix`.
fn digit_value(unit: u32, radix: u32) -> Option<u32> {
    let byte = u8::try_from(unit).ok()?;
    let value = match byte {
        b'0'..=b'9' => u32::from(byte - b'0'),
        b'a'..=b'z' => u32::from(byte - b'a') + 10,
        b'A'..=b'Z' => u32::from(byte - b'A') + 10,
        _ => return None,
    };
    (value < radix).then_some(value)
}

/// `parseInt` from step 3 (`#sec-parseint-string-radix`), with `radix` the value of ToInt32 of
/// the radix argument: the integer that the longest run of digits of the radix writes, after
/// the leading `StrWhiteSpace`, a sign and, for radix 0 or 16, a `0x` or `0X`; NaN when the
/// radix is not 0 and outside 2 to 36, or when the run is empty; -0 for a negative sign and a
/// zero value.
pub fn parse_int<U: Copy + Into<u32>>(text: &[U], radix: i32) -> f64 {
    // 3. If radixMV ≠ 0 and radixMV is not in the inclusive interval from 2 to 36, return NaN.
    let Ok(mut radix) = u32::try_from(radix) else {
        return f64::NAN;
    };
    if radix != 0 && !(2..=36).contains(&radix) {
        return f64::NAN;
    }
    // 4. Let trimmedString be ! TrimString(inputString, start). 5. If it is empty, NaN (the
    // run below is empty then).
    let mut text = trim_start(text);
    // 6, 7. The sign.
    let mut negative = false;
    match ascii_at(text, 0) {
        Some(b'-') => {
            negative = true;
            text = text.get(1..).unwrap_or_default();
        }
        Some(b'+') => text = text.get(1..).unwrap_or_default(),
        _ => {}
    }
    // 8. If radixMV = 0 or radixMV = 16, a leading "0x" or "0X" is stripped and the radix is 16.
    if (radix == 0 || radix == 16)
        && ascii_at(text, 0) == Some(b'0')
        && matches!(ascii_at(text, 1), Some(b'x' | b'X'))
    {
        text = text.get(2..).unwrap_or_default();
        radix = 16;
    }
    // 9. If radixMV = 0, set radixMV to 10.
    if radix == 0 {
        radix = 10;
    }
    // 10, 11. numberString, up to the first code unit that is not a digit of the radix.
    let end = text
        .iter()
        .position(|&unit| digit_value(unit.into(), radix).is_none())
        .unwrap_or(text.len());
    let digits = text.get(..end).unwrap_or_default();
    // 12. If numberString is empty, return NaN.
    if digits.is_empty() {
        return f64::NAN;
    }
    // 13. mathInt, by the method of the radix.
    let magnitude = match radix {
        10 => decimal_value(digits),
        2 | 4 | 8 | 16 | 32 => exact_value(digits, radix),
        _ => accumulated_value(digits, radix),
    };
    // 14. If sign = -1 and mathInt = 0, return -0𝔽. 15. Return 𝔽(sign × mathInt).
    if negative { -magnitude } else { magnitude }
}

/// The value of `digits` in `radix`, 2, 4, 8, 16 or 32, read bit by bit and rounded once.
fn exact_value<U: Copy + Into<u32>>(digits: &[U], radix: u32) -> f64 {
    // The bits per digit: 1 to 5, the power of two the radix is.
    let bits = radix.trailing_zeros();
    let mut value = LeadingBits::default();
    for &unit in digits {
        value.push(digit_value(unit.into(), radix).unwrap_or(0), bits);
    }
    value.round()
}

/// The value of `digits` in `radix` as `acc * radix + digit` in `f64`, from the first digit.
fn accumulated_value<U: Copy + Into<u32>>(digits: &[U], radix: u32) -> f64 {
    let radix_value = f64::from(radix);
    digits.iter().fold(0.0, |acc, &unit| {
        acc * radix_value + f64::from(digit_value(unit.into(), radix).unwrap_or(0))
    })
}

/// An integer read bit by bit, most significant first: its leading 128 bits (from its first 1),
/// how many bits followed them, and whether one of those was 1.
#[derive(Default)]
struct LeadingBits {
    leading: u128,
    dropped: u64,
    sticky: bool,
}

impl LeadingBits {
    /// Appends the low `bits` bits of `digit`, most significant first.
    fn push(&mut self, digit: u32, bits: u32) {
        for i in (0..bits).rev() {
            let bit = u128::from((digit >> i) & 1);
            if self.leading >> 127 == 0 {
                self.leading = (self.leading << 1) | bit;
            } else {
                self.dropped = self.dropped.saturating_add(1);
                self.sticky |= bit != 0;
            }
        }
    }

    /// The Number nearest to the integer, ties to even; +∞ when it rounds to 2^1024 or more
    /// (`#sec-ecmascript-language-types-number-type`).
    fn round(&self) -> f64 {
        let m = self.leading;
        if m == 0 {
            return 0.0;
        }
        // The place of the leading 1, from 0 to 127.
        let top = 127 - m.leading_zeros();
        if top <= 52 {
            // Fewer than 54 bits, so no bit was dropped: the integer is below 2^53, which the
            // cast keeps exactly.
            return m as f64;
        }
        // Keep 53 bits; the `shift` bits below them and the dropped ones decide the rounding.
        let shift = top - 52;
        let mut mantissa = m >> shift;
        let rest = m & ((1u128 << shift) - 1);
        let half = 1u128 << (shift - 1);
        if rest > half || (rest == half && (self.sticky || mantissa & 1 == 1)) {
            mantissa += 1;
        }
        // The value is mantissa × 2^exponent.
        let mut exponent = u64::from(shift).saturating_add(self.dropped);
        if mantissa == 1u128 << 53 {
            mantissa >>= 1;
            exponent = exponent.saturating_add(1);
        }
        // mantissa is from 2^52 to 2^53 - 1, so its leading 1 is at 52 + exponent, and the
        // largest finite Number has it at 1023.
        if exponent > 1023 - 52 {
            return f64::INFINITY;
        }
        // mantissa is below 2^53, which the cast keeps exactly; 2^exponent, exponent at most
        // 971, is a normal Number written by its bits, and the product of the two is exact.
        let power = f64::from_bits((exponent + 1023) << 52);
        (mantissa as f64) * power
    }
}
