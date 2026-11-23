//! `RegExp.escape ( string )` (`#sec-regexp.escape`) with EncodeForRegExpEscape
//! (`#sec-encodeforregexpescape`), written by `story:regexp-match-all-and-escape`: a copy of a
//! String in which each code point that could be special in a Pattern is an escape sequence.
//!
//! The code units are read with a step each before the first is looked at, and the result is
//! built by a [`StringBuilder`], which charges a step per code unit written, throws a
//! `RangeError` before a result longer than `string_length` and ends the run in
//! `Terminated(MemoryBudget)` before its buffer grows past the memory budget. No code runs:
//! the argument must be a String already.

use crate::builtins::object::argument;
use crate::builtins::string::{StringBuilder, code_point_at, utf16_encode_code_point};
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::host::{ErrorKind, Host};
use crate::value::Value;

/// The code unit 0x005C (REVERSE SOLIDUS).
const BACKSLASH: u16 = 0x005C;
/// The code unit 0x0078 (LATIN SMALL LETTER X).
const LETTER_X: u16 = 0x0078;
/// The code unit 0x0075 (LATIN SMALL LETTER U).
const LETTER_U: u16 = 0x0075;

/// The code points of SyntaxCharacter (`#prod-SyntaxCharacter`) and U+002F (SOLIDUS), which
/// EncodeForRegExpEscape writes after a `\` (step 1).
const SYNTAX_CHARACTERS: [u32; 15] = [
    0x5E, 0x24, 0x5C, 0x2E, 0x2A, 0x2B, 0x3F, 0x28, 0x29, 0x5B, 0x5D, 0x7B, 0x7D, 0x7C, 0x2F,
];

/// The code points of `otherPunctuators`, ",-=<>#&!%:;@~'`" and U+0022 (QUOTATION MARK)
/// (step 3).
const OTHER_PUNCTUATORS: [u32; 16] = [
    0x2C, 0x2D, 0x3D, 0x3C, 0x3E, 0x23, 0x26, 0x21, 0x25, 0x3A, 0x3B, 0x40, 0x7E, 0x27, 0x60, 0x22,
];

/// The code units EncodeForRegExpEscape answers for one code point: at most two `\u` escapes of
/// six code units each.
struct Encoded {
    units: [u16; 12],
    count: usize,
}

impl Encoded {
    /// No code unit yet.
    fn new() -> Encoded {
        Encoded {
            units: [0; 12],
            count: 0,
        }
    }

    /// Append `unit`; past the room of twelve the unit is dropped, which no code point reaches.
    fn push(&mut self, unit: u16) {
        if let Some(slot) = self.units.get_mut(self.count) {
            *slot = unit;
            self.count += 1;
        }
    }

    /// The code units appended.
    fn units(&self) -> &[u16] {
        self.units.get(..self.count).unwrap_or_default()
    }

    /// The `digits` lowest hex digits of `value`, lowercase, most significant first: what
    /// Number::toString(𝔽(value), 16) writes, padded at the start with "0" to `digits` by
    /// StringPad (`#sec-stringpad`).
    fn push_hex(&mut self, value: u32, digits: u32) {
        for place in (0..digits).rev() {
            let digit = value.checked_shr(place * 4).unwrap_or(0) & 0xF;
            // A digit is below 16, so the conversion is exact.
            let digit = u16::try_from(digit).unwrap_or(0);
            self.push(if digit < 10 {
                0x30 + digit
            } else {
                0x61 + digit - 10
            });
        }
    }

    /// `\x` and two hex digits of `code_point` (at most 0xFF).
    fn push_hex_escape(&mut self, code_point: u32) {
        self.push(BACKSLASH);
        self.push(LETTER_X);
        self.push_hex(code_point, 2);
    }
}

/// Whether `code_point` is matched by DecimalDigit or AsciiLetter (`#prod-DecimalDigit`,
/// `#prod-AsciiLetter`).
fn is_ascii_alphanumeric(code_point: u32) -> bool {
    matches!(code_point, 0x30..=0x39 | 0x41..=0x5A | 0x61..=0x7A)
}

/// Whether `code_point` is matched by WhiteSpace (`#sec-white-space`: TAB, VT, FF, ZWNBSP and
/// USP, the code points of General_Category `Zs`) or by LineTerminator
/// (`#sec-line-terminators`: LF, CR, LS and PS).
fn is_white_space_or_line_terminator(code_point: u32) -> bool {
    matches!(
        code_point,
        0x09 | 0x0B | 0x0C | 0xFEFF | 0x0A | 0x0D | 0x2028 | 0x2029
    ) || swell_unicode::is_space_separator(code_point)
}

/// EncodeForRegExpEscape ( codePoint ) (`#sec-encodeforregexpescape`), with the escape of an
/// initial ASCII letter or digit of `RegExp.escape` (step 4.a) when `initial`.
fn encode_for_regexp_escape(code_point: u32, initial: bool) -> Encoded {
    let mut encoded = Encoded::new();
    // RegExp.escape, 4.a. If escaped is the empty String and codePoint is matched by either
    // DecimalDigit or AsciiLetter, then iv. the string-concatenation of the code unit 0x005C
    // (REVERSE SOLIDUS), "x", and hex.
    if initial && is_ascii_alphanumeric(code_point) {
        encoded.push_hex_escape(code_point);
        return encoded;
    }
    // 1. If codePoint is matched by SyntaxCharacter or codePoint is U+002F (SOLIDUS), then a.
    // Return the string-concatenation of 0x005C (REVERSE SOLIDUS) and
    // UTF16EncodeCodePoint(codePoint).
    if SYNTAX_CHARACTERS.contains(&code_point) {
        encoded.push(BACKSLASH);
        encoded.push(u16::try_from(code_point).unwrap_or(0));
        return encoded;
    }
    // 2. If codePoint is a code point listed in the "Code Point" column of Table 63, then a.
    // Return the string-concatenation of 0x005C (REVERSE SOLIDUS) and the string in the
    // "ControlEscape" column of the row whose "Code Point" column contains codePoint.
    let control = match code_point {
        0x09 => Some(b't'),
        0x0A => Some(b'n'),
        0x0B => Some(b'v'),
        0x0C => Some(b'f'),
        0x0D => Some(b'r'),
        _ => None,
    };
    if let Some(letter) = control {
        encoded.push(BACKSLASH);
        encoded.push(u16::from(letter));
        return encoded;
    }
    // 3, 4. otherPunctuators and toEscape. 5. If toEscape contains codePoint, codePoint is
    // matched by either WhiteSpace or LineTerminator, or codePoint has the same numeric value
    // as a leading surrogate or trailing surrogate, then
    if OTHER_PUNCTUATORS.contains(&code_point)
        || is_white_space_or_line_terminator(code_point)
        || (0xD800..=0xDFFF).contains(&code_point)
    {
        // b. If codePointNumber ≤ 0xFF, then i. Let hex be Number::toString(𝔽(codePointNumber),
        // 16). ii. Return the string-concatenation of the code unit 0x005C (REVERSE SOLIDUS),
        // "x", and StringPad(hex, 2, "0", start).
        if code_point <= 0xFF {
            encoded.push_hex_escape(code_point);
            return encoded;
        }
        // c. Let escaped be the empty String. d. Let codeUnits be
        // UTF16EncodeCodePoint(codePoint). e. For each code unit codeUnit of codeUnits, do i.
        // Set escaped to the string-concatenation of escaped and UnicodeEscape(codeUnit)
        // (`#sec-unicodeescape`: "\u" and four lowercase hex digits). f. Return escaped.
        if let Some(units) = utf16_encode_code_point(code_point) {
            for &unit in units.units() {
                encoded.push(BACKSLASH);
                encoded.push(LETTER_U);
                encoded.push_hex(u32::from(unit), 4);
            }
        }
        return encoded;
    }
    // 6. Return UTF16EncodeCodePoint(codePoint).
    if let Some(units) = utf16_encode_code_point(code_point) {
        for &unit in units.units() {
            encoded.push(unit);
        }
    }
    encoded
}

impl<H: Host> Agent<H> {
    /// `RegExp.escape ( string )` (`#sec-regexp.escape`).
    #[inline(never)]
    pub(super) fn regexp_escape(&mut self, args: &[Value]) -> Completion<Value> {
        // 1. If string is not a String, throw a TypeError exception.
        let Value::String(string) = argument(args, 0) else {
            return Err(self.throw_error(
                ErrorKind::TypeError,
                "RegExp.escape requires that its argument be a String",
            ));
        };
        // 3. Let codePointList be StringToCodePoints(string) (`#sec-stringtocodepoints`): the
        // code units, a step each, read by CodePointAt.
        let units = self.charged_units(string)?;
        // 2. Let escaped be the empty String.
        let mut escaped = StringBuilder::new();
        let mut position = 0;
        let mut initial = true;
        // 4. For each code point codePoint of codePointList, do
        while let Some(code_point) = code_point_at(&units, position) {
            // a. If escaped is the empty String and codePoint is matched by either DecimalDigit
            // or AsciiLetter, the escape of 4.a; b. Else, set escaped to the
            // string-concatenation of escaped and EncodeForRegExpEscape(codePoint). Every code
            // point writes at least one code unit, so escaped is empty only before the first.
            let encoded = encode_for_regexp_escape(code_point.code_point, initial);
            self.append_units(&mut escaped, encoded.units())?;
            initial = false;
            position = position.saturating_add(code_point.code_unit_count);
        }
        // 5. Return escaped.
        Ok(Value::String(self.finish_string(escaped)?))
    }
}
