//! The global object's own properties of clause 19 that are not constructors or namespaces.
//!
//! The value properties (19.1, `#sec-value-properties-of-the-global-object`): `Infinity`
//! (`#sec-value-properties-of-the-global-object-infinity`), `NaN`
//! (`#sec-value-properties-of-the-global-object-nan`) and `undefined`
//! (`#sec-undefined`), written by `story:object-model-and-realms`; `globalThis`
//! (`#sec-globalthis`, `HOST-155`), the realm's global `this` value, which SetDefaultGlobalBindings
//! reads from the realm record (`realm/global_object.rs`,
//! `story:global-this-uri-functions-and-is-error`).
//!
//! The function properties (19.2, `#sec-function-properties-of-the-global-object`) that slice 2
//! takes early (`docs/design/engine.md`, section 12, "A part before its slice";
//! `story:number-boolean-and-global-functions-early`): `isFinite` (19.2.2,
//! `#sec-isfinite-number`) and `isNaN` (19.2.3, `#sec-isnan-number`), the intrinsics
//! `%isFinite%` and `%isNaN%`; and `eval` (19.2.1, `#sec-eval-x`), slice 2's own, the intrinsic
//! `%eval%`, whose steps are `dynamic_code.rs` (`story:eval-and-function-constructor`).
//!
//! Slice 4 adds `parseFloat` (19.2.4, `#sec-parsefloat-string`) and `parseInt` (19.2.5,
//! `#sec-parseint-string-radix`), the intrinsics `%parseFloat%` and `%parseInt%`, which are also
//! `Number.parseFloat` and `Number.parseInt` (`story:number-functions`). ToString of the
//! argument, and for `parseInt` ToInt32 of the radix, may run code; then the interrupt is read,
//! a step is charged per code unit of the String past 32 before one is read, and the code units
//! are read in place by `swell-num` (`parse.rs`). Decimal digits go to an ASCII buffer of at
//! most the String's length there, which is charged to the memory budget at that length before
//! `swell-num` is called and credited when it returns.
//!
//! The URI handling functions (19.2.6, `#sec-uri-handling-functions`), slice 3
//! (`story:global-this-uri-functions-and-is-error`): `decodeURI` (`#sec-decodeuri-encodeduri`),
//! `decodeURIComponent` (`#sec-decodeuricomponent-encodeduricomponent`), `encodeURI`
//! (`#sec-encodeuri-uri`) and `encodeURIComponent` (`#sec-encodeuricomponent-uricomponent`),
//! the intrinsics `%decodeURI%`, `%decodeURIComponent%`, `%encodeURI%` and
//! `%encodeURIComponent%`, with Encode (`#sec-encode`), Decode (`#sec-decode`) and
//! ParseHexOctet (`#sec-parsehexoctet`). Encode reads the String by CodePointAt and Decode
//! writes a code point by UTF16EncodeCodePoint (`builtins/string/mod.rs`). Both charge a step
//! per code unit they read, and the [`StringBuilder`] of the result a step per code unit it
//! writes; the result is bounded by `string_length` and asks the memory budget before it
//! grows. The loops call no code: only ToString of the argument does, before them.
//!
//! Each is defined on each global object by SetDefaultGlobalBindings
//! (`realm/global_object.rs`).

use crate::builtins::object::argument;
use crate::builtins::string::{StringBuilder, code_point_at, utf16_encode_code_point};
use crate::heap::string::HeapString;
use crate::host::agent::Agent;
use crate::host::outcome::{Abrupt, Completion, Termination};
use crate::host::{ErrorKind, Host};
use crate::object::builtin_function::BuiltinId;
use crate::ops::conversion::number_to_int32;
use crate::realm::intrinsics::Making;
use crate::value::{ObjectRef, StringRef, Value};

/// The value properties of 19.1 this part defines, by name; `globalThis` is the realm's and is
/// defined by SetDefaultGlobalBindings from the realm record.
pub(crate) fn value_properties() -> [(&'static str, Value); 3] {
    [
        ("Infinity", Value::Number(f64::INFINITY)),
        ("NaN", Value::Number(f64::NAN)),
        ("undefined", Value::Undefined),
    ]
}

/// The builtins of this module, by their steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GlobalBuiltin {
    /// `isFinite ( value )` (19.2.2).
    IsFinite,
    /// `isNaN ( value )` (19.2.3).
    IsNaN,
    /// `eval ( x )` (19.2.1), slice 2's own.
    Eval,
    /// `parseFloat ( string )` (19.2.4).
    ParseFloat,
    /// `parseInt ( string, radix )` (19.2.5).
    ParseInt,
    /// `decodeURI ( encodedURI )` (19.2.6.1).
    DecodeUri,
    /// `decodeURIComponent ( encodedURIComponent )` (19.2.6.2).
    DecodeUriComponent,
    /// `encodeURI ( uri )` (19.2.6.3).
    EncodeUri,
    /// `encodeURIComponent ( uriComponent )` (19.2.6.4).
    EncodeUriComponent,
}

/// `%decodeURI%`, `%decodeURIComponent%`, `%encodeURI%` and `%encodeURIComponent%`.
#[derive(Clone, Copy, Debug)]
pub(crate) struct UriFunctions {
    pub(crate) decode_uri: ObjectRef,
    pub(crate) decode_uri_component: ObjectRef,
    pub(crate) encode_uri: ObjectRef,
    pub(crate) encode_uri_component: ObjectRef,
}

/// The reserved set of `encodeURI` and `decodeURI`: extraUnescaped of `#sec-encodeuri-uri`,
/// step 2, and preserveEscapeSet of `#sec-decodeuri-encodeduri`, step 2.
const URI_RESERVED: &[u8] = b";/?:@&=+$,#";

/// "-.!~*'()", which Encode's alwaysUnescaped adds to the ASCII word characters
/// (`#sec-encode`, step 3).
const URI_MARKS: &[u8] = b"-.!~*'()";

/// The code unit of "%" (U+0025 PERCENT SIGN).
const PERCENT: u16 = 0x25;

/// The uppercase hexadecimal digits, by value.
const HEX_DIGITS: &[u8; 16] = b"0123456789ABCDEF";

/// Whether `code_unit` is in the unescapedSet of Encode (`#sec-encode`, steps 3 and 4): the
/// ASCII word characters (`#ASCII-word-characters`: the letters and digits of Basic Latin
/// and U+005F LOW LINE), "-.!~*'()" and `extra_unescaped`.
fn is_unescaped(code_unit: u16, extra_unescaped: &[u8]) -> bool {
    let Ok(byte) = u8::try_from(code_unit) else {
        return false;
    };
    byte.is_ascii_alphanumeric()
        || byte == b'_'
        || URI_MARKS.contains(&byte)
        || extra_unescaped.contains(&byte)
}

/// The value of the code unit `unit` as a HexDigit (`#prod-HexDigit`): 0-9, a-f, A-F.
fn hex_digit_value(unit: u16) -> Option<u8> {
    let byte = u8::try_from(unit).ok()?;
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// `parseFloat` of the code units `text` when `radix` is `None`, otherwise `parseInt` with that
/// value of ToInt32 (`swell-num`, `parse.rs`).
fn parse_units<U: Copy + Into<u32>>(text: &[U], radix: Option<i32>) -> f64 {
    match radix {
        None => swell_num::parse::parse_float(text),
        Some(radix) => swell_num::parse::parse_int(text, radix),
    }
}

/// ParseHexOctet ( string, position ) (`#sec-parsehexoctet`): the octet the two code units of
/// `string` at `position` and `position + 1` write as HexDigits[~Sep], or `None` when they do
/// not parse (the List of SyntaxError objects, which Decode turns into a URIError) or are not
/// both there (step 2's assertion, which Decode's callers check first).
fn parse_hex_octet(string: &[u16], position: usize) -> Option<u8> {
    // 3. Let hexDigits be the substring of string from position to position + 2.
    let high = hex_digit_value(*string.get(position)?)?;
    let low = hex_digit_value(*string.get(position.checked_add(1)?)?)?;
    // 4, 5, 6. Let n be the MV of parseResult; it is in the interval from 0 to 255.
    Some(high * 16 + low)
}

/// The escape sequence "%XX" of `octet`, its two hex digits uppercase (`#sec-encode`, step
/// 6.b.v.1 and 2: "the String representation of octet, formatted as an uppercase hexadecimal
/// number", padded to two digits by StringPad).
fn escape_octet(octet: u8) -> [u16; 3] {
    let digit = |value: u8| {
        HEX_DIGITS
            .get(usize::from(value & 0xF))
            .map_or(0, |&digit| u16::from(digit))
    };
    [PERCENT, digit(octet >> 4), digit(octet)]
}

impl<H: Host> Agent<H> {
    /// `%isFinite%`, `%isNaN%` and `%parseFloat%`, built-in functions of length 1, and
    /// `%parseInt%`, of length 2 (clause 18 and 19.2), not constructors.
    pub(crate) fn create_global_functions(
        &mut self,
        making: Making,
    ) -> Completion<(ObjectRef, ObjectRef, ObjectRef, ObjectRef)> {
        let is_finite = self.create_builtin(
            making,
            "isFinite",
            1,
            BuiltinId::Global(GlobalBuiltin::IsFinite),
        )?;
        let is_nan =
            self.create_builtin(making, "isNaN", 1, BuiltinId::Global(GlobalBuiltin::IsNaN))?;
        let parse_float = self.create_builtin(
            making,
            "parseFloat",
            1,
            BuiltinId::Global(GlobalBuiltin::ParseFloat),
        )?;
        let parse_int = self.create_builtin(
            making,
            "parseInt",
            2,
            BuiltinId::Global(GlobalBuiltin::ParseInt),
        )?;
        Ok((is_finite, is_nan, parse_float, parse_int))
    }

    /// `%eval%` (19.2.1): a built-in function of length 1 (clause 18), not a constructor.
    pub(crate) fn create_eval_function(&mut self, making: Making) -> Completion<ObjectRef> {
        self.create_builtin(making, "eval", 1, BuiltinId::Global(GlobalBuiltin::Eval))
    }

    /// `%decodeURI%`, `%decodeURIComponent%`, `%encodeURI%` and `%encodeURIComponent%`
    /// (19.2.6.1 to 19.2.6.4): built-in functions of length 1 (clause 18), not constructors.
    pub(crate) fn create_uri_functions(&mut self, making: Making) -> Completion<UriFunctions> {
        let decode_uri = self.create_builtin(
            making,
            "decodeURI",
            1,
            BuiltinId::Global(GlobalBuiltin::DecodeUri),
        )?;
        let decode_uri_component = self.create_builtin(
            making,
            "decodeURIComponent",
            1,
            BuiltinId::Global(GlobalBuiltin::DecodeUriComponent),
        )?;
        let encode_uri = self.create_builtin(
            making,
            "encodeURI",
            1,
            BuiltinId::Global(GlobalBuiltin::EncodeUri),
        )?;
        let encode_uri_component = self.create_builtin(
            making,
            "encodeURIComponent",
            1,
            BuiltinId::Global(GlobalBuiltin::EncodeUriComponent),
        )?;
        Ok(UriFunctions {
            decode_uri,
            decode_uri_component,
            encode_uri,
            encode_uri_component,
        })
    }

    /// The steps of the builtins of this module.
    pub(crate) fn run_global_builtin(
        &mut self,
        id: GlobalBuiltin,
        _function: ObjectRef,
        _this: Value,
        args: &[Value],
        _new_target: Option<ObjectRef>,
    ) -> Completion<Value> {
        match id {
            // `#sec-eval-x`: 1. Return ? PerformEval(x, false, indirect-eval)
            // (`dynamic_code.rs`).
            GlobalBuiltin::Eval => self.indirect_eval(args),
            GlobalBuiltin::IsFinite => self.number_test(true, args),
            GlobalBuiltin::IsNaN => self.number_test(false, args),
            GlobalBuiltin::ParseFloat => self.parse_float(args),
            GlobalBuiltin::ParseInt => self.parse_int(args),
            // `#sec-decodeuri-encodeduri`: 2. Let preserveEscapeSet be ";/?:@&=+$,#".
            GlobalBuiltin::DecodeUri => self.uri_function(false, URI_RESERVED, args),
            // `#sec-decodeuricomponent-encodeduricomponent`: 2. Let preserveEscapeSet be the
            // empty String.
            GlobalBuiltin::DecodeUriComponent => self.uri_function(false, b"", args),
            // `#sec-encodeuri-uri`: 2. Let extraUnescaped be ";/?:@&=+$,#".
            GlobalBuiltin::EncodeUri => self.uri_function(true, URI_RESERVED, args),
            // `#sec-encodeuricomponent-uricomponent`: 2. Let extraUnescaped be the empty
            // String.
            GlobalBuiltin::EncodeUriComponent => self.uri_function(true, b"", args),
        }
    }

    /// `isFinite` when `finite`, otherwise `isNaN`.
    fn number_test(&mut self, finite: bool, args: &[Value]) -> Completion<Value> {
        // 1. Let number be ? ToNumber(value).
        let number = self.to_number(argument(args, 0))?;
        Ok(Value::Boolean(if finite {
            // `#sec-isfinite-number`: 2. If number is a finite Number, return true. 3. Return
            // false.
            number.is_finite()
        } else {
            // `#sec-isnan-number`: 2. If number is NaN, return true. 3. Return false.
            number.is_nan()
        }))
    }

    /// `parseFloat ( string )` (`#sec-parsefloat-string`).
    #[inline(never)]
    fn parse_float(&mut self, args: &[Value]) -> Completion<Value> {
        // 1. Let inputString be ? ToString(string).
        let string = self.to_string(argument(args, 0))?;
        // 2 to 7, in `swell-num`.
        self.parse_text(string, None)
    }

    /// `parseInt ( string, radix )` (`#sec-parseint-string-radix`).
    #[inline(never)]
    fn parse_int(&mut self, args: &[Value]) -> Completion<Value> {
        // 1. Let inputString be ? ToString(string).
        let string = self.to_string(argument(args, 0))?;
        // 2. Let radixMV be ℝ(? ToInt32(radix)) (`#sec-toint32`: ToNumber, then the integer
        // modulo 2^32). ToNumber may run code, so the String is kept while it runs.
        let keep = self.keep(Value::String(string));
        let radix = self.to_number(argument(args, 1));
        self.release(keep);
        let radix = number_to_int32(radix?);
        // 3 to 16, in `swell-num`.
        self.parse_text(string, Some(radix))
    }

    /// The steps of `parseFloat` after ToString (`radix` `None`) or of `parseInt` after ToInt32
    /// (`radix` the value of ToInt32) over the code units of `string`. No code runs from here
    /// on. The interrupt is read first, since the conversions before may have run code that
    /// raised it and no instruction has read it since. Then a step is charged per code unit past
    /// 32 before one is read; the code units are read in place, in the String's own storage;
    /// and when the digits may be decimal (`parseFloat`, or a radix of 0 or 10) the ASCII buffer
    /// `swell-num` copies them to, at most one byte per code unit, is charged at the String's
    /// length before the call (over the budget the run ends in `Terminated(MemoryBudget)` and
    /// nothing is read) and credited when it returns.
    fn parse_text(&mut self, string: StringRef, radix: Option<i32>) -> Completion<Value> {
        self.check_interrupt()?;
        let length = self.string_length(string)?;
        self.charge_code_units(length)?;
        let buffer = match radix {
            None | Some(0 | 10) => u64::try_from(length).unwrap_or(u64::MAX),
            Some(_) => 0,
        };
        if self.heap.accounting.charge(buffer).is_err() {
            return Err(self.terminate(Termination::MemoryBudget));
        }
        let value = match self.heap.strings.get(string.handle) {
            Ok(HeapString::Latin1(bytes)) => Some(parse_units(bytes, radix)),
            Ok(HeapString::Utf16(units)) => Some(parse_units(units, radix)),
            Err(_) => None,
        };
        self.heap.accounting.credit(buffer);
        match value {
            Some(value) => Ok(Value::Number(value)),
            None => Err(self.fault()),
        }
    }

    /// The four URI functions: Encode with extraUnescaped `set` when `encode`, otherwise Decode
    /// with preserveEscapeSet `set`.
    fn uri_function(&mut self, encode: bool, set: &[u8], args: &[Value]) -> Completion<Value> {
        // 1. Let uriString be ? ToString(uri) (and the like in the other three).
        let string = self.to_string(argument(args, 0))?;
        // The code units are copied out: neither loop below calls code or allocates on the
        // heap before the result, so nothing has to be kept.
        let length = self.string_length(string)?;
        let units = self.string_units_in(string, 0, length)?;
        // 3. Return ? Encode(uriString, extraUnescaped) / ? Decode(uriString,
        // preserveEscapeSet).
        let result = if encode {
            self.encode(&units, set)?
        } else {
            self.decode(&units, set)?
        };
        Ok(Value::String(result))
    }

    /// A new URIError of the current realm, as a throw completion.
    fn uri_error(&mut self, message: &str) -> Abrupt {
        self.throw_error(ErrorKind::UriError, message)
    }

    /// Encode ( string, extraUnescaped ) (`#sec-encode`). A step per code unit read; the
    /// builder charges a step per code unit written.
    fn encode(&mut self, string: &[u16], extra_unescaped: &[u8]) -> Completion<StringRef> {
        // 1. Let length be the length of string. 2. Let result be the empty String.
        let length = string.len();
        let mut result = StringBuilder::new();
        // 5. Let k be 0. 6. Repeat, while k < length.
        let mut k = 0;
        while k < length {
            // a. Let codeUnit be the code unit at index k within string.
            let Some(&code_unit) = string.get(k) else {
                return Err(self.fault());
            };
            // b. If unescapedSet contains codeUnit, then
            if is_unescaped(code_unit, extra_unescaped) {
                self.charge_steps(1)?;
                // i. Set k to k + 1. ii. Set result to the string-concatenation of result and
                // codeUnit.
                k += 1;
                self.append_units(&mut result, &[code_unit])?;
                continue;
            }
            // c. Else, i. Let codePoint be CodePointAt(string, k).
            let Some(code_point) = code_point_at(string, k) else {
                return Err(self.fault());
            };
            self.charge_steps(u64::try_from(code_point.code_unit_count).unwrap_or(u64::MAX))?;
            // ii. If codePoint.[[IsUnpairedSurrogate]] is true, throw a URIError exception.
            if code_point.is_unpaired_surrogate {
                return Err(self.uri_error("URI malformed: a lone surrogate cannot be encoded"));
            }
            // iii. Set k to k + codePoint.[[CodeUnitCount]].
            k += code_point.code_unit_count;
            // iv. Let octets be the List of octets resulting by applying the UTF-8
            // transformation to codePoint.[[CodePoint]]. A code point that is no surrogate is a
            // scalar value, which `char` holds.
            let Some(scalar) = char::from_u32(code_point.code_point) else {
                return Err(self.fault());
            };
            let mut buffer = [0_u8; 4];
            let octets = scalar.encode_utf8(&mut buffer).as_bytes();
            // v. For each element octet of octets: the string-concatenation of result, "%" and
            // StringPad(hex, 2, "0", start).
            let mut escaped = [0_u16; 12];
            for (slot, &octet) in escaped.as_chunks_mut::<3>().0.iter_mut().zip(octets) {
                *slot = escape_octet(octet);
            }
            let written = escaped.get(..octets.len() * 3).unwrap_or(&[]);
            self.append_units(&mut result, written)?;
        }
        // 7. Return result.
        self.finish_string(result)
    }

    /// Decode ( string, preserveEscapeSet ) (`#sec-decode`). A step per code unit read; the
    /// builder charges a step per code unit written.
    fn decode(&mut self, string: &[u16], preserve_escape_set: &[u8]) -> Completion<StringRef> {
        // 1. Let length be the length of string. 2. Let result be the empty String.
        let length = string.len();
        let mut result = StringBuilder::new();
        // 3. Let k be 0. 4. Repeat, while k < length.
        let mut k = 0;
        while k < length {
            // a. Let codeUnit be the code unit at index k within string.
            let Some(&code_unit) = string.get(k) else {
                return Err(self.fault());
            };
            // b. Let segment be codeUnit. c. If codeUnit is not "%", segment stays codeUnit;
            // e, f. it is appended and k moves on by one.
            if code_unit != PERCENT {
                self.charge_steps(1)?;
                self.append_units(&mut result, &[code_unit])?;
                k += 1;
                continue;
            }
            // c. i. If k + 3 > length, throw a URIError exception.
            if k + 3 > length {
                return Err(self.uri_error("URI malformed: an escape needs two hex digits"));
            }
            self.charge_steps(3)?;
            // ii. Let escape be the substring of string from k to k + 3. iii. Let firstOctet be
            // ParseHexOctet(string, k + 1). iv. If firstOctet is not an integer, throw.
            let Some(first_octet) = parse_hex_octet(string, k + 1) else {
                return Err(self.uri_error("URI malformed: an escape needs two hex digits"));
            };
            let escape = string.get(k..k + 3).unwrap_or(&[]);
            // v. Set k to k + 2 (and f, + 1: past the escape).
            k += 3;
            // vi. Let n be the number of leading 1 bits in firstOctet.
            let n = first_octet.leading_ones();
            // vii. If n = 0, then
            if n == 0 {
                // 1. Let asciiChar be the code unit whose numeric value is firstOctet. 2. If
                // preserveEscapeSet contains asciiChar, set segment to escape; else set segment
                // to asciiChar.
                if preserve_escape_set.contains(&first_octet) {
                    self.append_units(&mut result, escape)?;
                } else {
                    self.append_units(&mut result, &[u16::from(first_octet)])?;
                }
                continue;
            }
            // viii. 1. If n = 1 or n > 4, throw a URIError exception.
            if n == 1 || n > 4 {
                return Err(self.uri_error("URI malformed: not the first octet of UTF-8"));
            }
            // 2. Let octets be « firstOctet ». 3. Let j be 1. 4. Repeat, while j < n.
            let mut octets = [first_octet, 0, 0, 0];
            for slot in octets.iter_mut().take(n as usize).skip(1) {
                // a. Set k to k + 1 (done above). b. If k + 3 > length, throw a URIError
                // exception.
                if k + 3 > length {
                    return Err(self.uri_error("URI malformed: a UTF-8 sequence is cut short"));
                }
                // c. If the code unit at index k within string is not "%", throw.
                if string.get(k) != Some(&PERCENT) {
                    return Err(self.uri_error("URI malformed: a UTF-8 sequence is cut short"));
                }
                self.charge_steps(3)?;
                // d. Let continuationByte be ParseHexOctet(string, k + 1). e. If it is not an
                // integer, throw a URIError exception.
                let Some(continuation) = parse_hex_octet(string, k + 1) else {
                    return Err(self.uri_error("URI malformed: an escape needs two hex digits"));
                };
                // f. Append continuationByte to octets. g. Set k to k + 2 (and a, + 1).
                *slot = continuation;
                k += 3;
            }
            // 6. If octets does not contain a valid UTF-8 encoding of a Unicode code point,
            // throw a URIError exception (RFC 3629: no overlong form, no surrogate, nothing
            // above U+10FFFF, every continuation 10xxxxxx; the first octet's n leading 1 bits
            // make the n octets one code point or none).
            let decoded = octets
                .get(..n as usize)
                .and_then(|octets| std::str::from_utf8(octets).ok())
                .and_then(|text| text.chars().next());
            let Some(scalar) = decoded else {
                return Err(self.uri_error("URI malformed: not the UTF-8 of a code point"));
            };
            // 7. Let codePoint be the code point obtained by applying the UTF-8 transformation
            // to octets. 8. Set segment to UTF16EncodeCodePoint(codePoint).
            let Some(segment) = utf16_encode_code_point(u32::from(scalar)) else {
                return Err(self.fault());
            };
            self.append_units(&mut result, segment.units())?;
        }
        // 5. Return result.
        self.finish_string(result)
    }
}

#[cfg(test)]
mod tests {
    use super::{escape_octet, hex_digit_value, is_unescaped, parse_hex_octet};

    /// The white space `parseFloat` and `parseInt` skip in `swell-num` is the engine's
    /// `StrWhiteSpaceChar` (`ops/conversion.rs`, with `Zs` from `swell-unicode`), over every
    /// code unit: 25 of them.
    #[test]
    fn the_white_space_of_swell_num_is_the_engines() {
        let mut count = 0;
        for unit in 0..=u16::MAX {
            let engine = crate::ops::conversion::is_str_white_space(unit);
            assert_eq!(
                swell_num::parse::is_str_white_space(u32::from(unit)),
                engine,
                "{unit:#06x}"
            );
            count += usize::from(engine);
        }
        assert_eq!(count, 25);
    }

    /// ParseHexOctet (`#sec-parsehexoctet`) reads two HexDigits of either case and nothing else
    /// (no sign, no space, no digit outside Basic Latin), and needs both code units.
    #[test]
    fn parse_hex_octet_reads_two_hex_digits() {
        let units = |text: &str| text.encode_utf16().collect::<Vec<u16>>();
        assert_eq!(parse_hex_octet(&units("%41"), 1), Some(0x41));
        assert_eq!(parse_hex_octet(&units("ff"), 0), Some(0xFF));
        assert_eq!(parse_hex_octet(&units("aB"), 0), Some(0xAB));
        assert_eq!(parse_hex_octet(&units("00"), 0), Some(0));
        for bad in ["+1", "-1", " 1", "1 ", "g0", "0G", "\u{663}1", "1"] {
            assert_eq!(parse_hex_octet(&units(bad), 0), None, "{bad:?}");
        }
        assert_eq!(parse_hex_octet(&units("41"), 1), None);
        assert_eq!(hex_digit_value(0x0130), None);
        assert_eq!(escape_octet(0x0A), [0x25, 0x30, 0x41]);
        assert_eq!(escape_octet(0xF0), [0x25, 0x46, 0x30]);
    }

    /// The unescapedSet of Encode (`#sec-encode`, steps 3 and 4) over every code unit of Basic
    /// Latin and the first one past it.
    #[test]
    fn unescaped_set_is_word_characters_marks_and_extra() {
        let kept: Vec<u16> = (0..=0x80_u16)
            .filter(|&unit| is_unescaped(unit, b""))
            .collect();
        let expected: Vec<u16> =
            "!'()*-.0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ_abcdefghijklmnopqrstuvwxyz~"
                .encode_utf16()
                .collect();
        assert_eq!(kept, expected);
        assert!(is_unescaped(u16::from(b'#'), b";/?:@&=+$,#"));
        assert!(!is_unescaped(u16::from(b'#'), b""));
        assert!(!is_unescaped(u16::from(b'%'), b";/?:@&=+$,#"));
    }
}
