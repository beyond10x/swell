//! Type conversion (ECMA-262 7.1, `#sec-type-conversion`): ToPrimitive, OrdinaryToPrimitive,
//! ToBoolean, ToNumber with StringToNumber, ToNumeric, ToIntegerOrInfinity, ToInt32, ToUint32,
//! ToString, ToObject, ToPropertyKey, ToLength and CanonicalNumericIndexString, and the property
//! keys of the engine made from Strings. Written by `story:object-model-and-realms`.
//! `story:string-access-and-search-methods` adds ToUint16 (`#sec-touint16`) with
//! ToFixedSizeInteger (`#sec-tofixedsizeinteger`), ToAbsoluteIndex (`#sec-toabsoluteindex`) and
//! ToClampedIndex (`#sec-toclampedindex`).
//!
//! StringToNumber trims `StrWhiteSpaceChar` (`#sec-tonumber-applied-to-the-string-type`: the
//! white space of 12.2, with every `Zs` code point from `swell-unicode`, and the line
//! terminators of 12.3) and parses the rest with `swell-num`; Number::toString is `swell-num`'s.
//! The trim charges a step per code unit it reads.

use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::host::{ErrorKind, Host, PreferredType};
use crate::object::ordinary::OrdinaryData;
use crate::object::property::{Key, array_index};
use crate::realm::intrinsics::Intrinsic;
use crate::value::{ObjectRef, StringRef, Value};

/// Whether `unit` is a `StrWhiteSpaceChar` (`#sec-tonumber-applied-to-the-string-type`):
/// `WhiteSpace` (12.2: TAB, VT, FF, ZWNBSP and every `Zs` code point) or `LineTerminator` (12.3:
/// LF, CR, LS, PS).
pub(crate) fn is_str_white_space(unit: u16) -> bool {
    matches!(
        unit,
        0x0009 | 0x000B | 0x000C | 0xFEFF | 0x000A | 0x000D | 0x2028 | 0x2029
    ) || swell_unicode::is_space_separator(u32::from(unit))
}

/// ToInt32 (`#sec-toint32`) of a Number.
pub(crate) fn number_to_int32(number: f64) -> i32 {
    let unsigned = i64::from(number_to_uint32(number));
    let signed = if unsigned >= 1 << 31 {
        unsigned - (1 << 32)
    } else {
        unsigned
    };
    i32::try_from(signed).unwrap_or(0)
}

/// ToUint32 (`#sec-touint32`) of a Number.
pub(crate) fn number_to_uint32(number: f64) -> u32 {
    if !number.is_finite() || number == 0.0 {
        return 0;
    }
    // rem_euclid answers an integer from 0 to 2^32 - 1, which the cast keeps exactly.
    number.trunc().rem_euclid(4_294_967_296.0) as u32
}

/// ToFixedSizeInteger(int, unsigned, 16) (`#sec-tofixedsizeinteger`) of what
/// ToIntegerOrInfinity answers: step 2 of ToUint16 (`#sec-touint16`).
pub(crate) fn integer_to_uint16(int: f64) -> u16 {
    // 1. If int = +∞ or int = -∞, return 0. (NaN is no integer; it is 0 as well.)
    if !int.is_finite() {
        return 0;
    }
    // 2. Let fixedInt be int modulo 2^16. rem_euclid of an integer answers an integer from 0 to
    // 2^16 - 1, which the cast keeps exactly. 4. Return fixedInt.
    int.trunc().rem_euclid(65_536.0) as u16
}

/// The result of clamping the integer, +∞ or -∞ `int` between 0 and `upper` (5.2.6,
/// "clamping"), as an index.
pub(crate) fn clamp_index(int: f64, upper: usize) -> usize {
    if int <= 0.0 || int.is_nan() {
        0
    } else if int >= upper as f64 {
        upper
    } else {
        // 0 < int < upper: an integer that fits.
        int as usize
    }
}

#[expect(
    clippy::wrong_self_convention,
    reason = "the names are ECMA-262's; each operation allocates, charges steps or calls into the agent, so it needs `&mut self`"
)]
impl<H: Host> Agent<H> {
    /// ToPrimitive (`#sec-toprimitive`).
    pub(crate) fn to_primitive(
        &mut self,
        input: Value,
        preferred: Option<PreferredType>,
    ) -> Completion<Value> {
        let Value::Object(object) = input else {
            return Ok(input);
        };
        let exotic = match self.well_known() {
            Some(symbols) => self.get_method(input, Key::Symbol(symbols.to_primitive))?,
            None => None,
        };
        if let Some(exotic) = exotic {
            let hint = match preferred {
                None => "default",
                Some(PreferredType::String) => "string",
                Some(PreferredType::Number) => "number",
            };
            let hint = self.new_string_str(hint)?;
            let result = self.call_value(exotic, input, &[Value::String(hint)])?;
            if matches!(result, Value::Object(_)) {
                return Err(self.throw_error(
                    ErrorKind::TypeError,
                    "Cannot convert object to primitive value",
                ));
            }
            return Ok(result);
        }
        self.ordinary_to_primitive(object, preferred.unwrap_or(PreferredType::Number))
    }

    /// OrdinaryToPrimitive (`#sec-ordinarytoprimitive`).
    pub(crate) fn ordinary_to_primitive(
        &mut self,
        object: ObjectRef,
        hint: PreferredType,
    ) -> Completion<Value> {
        let names = match hint {
            PreferredType::String => ["toString", "valueOf"],
            PreferredType::Number => ["valueOf", "toString"],
        };
        for name in names {
            let key = self.name_key(name)?;
            let method = self.get(object, key)?;
            if self.is_callable(method)? {
                let result = self.call_value(method, Value::Object(object), &[])?;
                if !matches!(result, Value::Object(_)) {
                    return Ok(result);
                }
            }
        }
        Err(self.throw_error(
            ErrorKind::TypeError,
            "Cannot convert object to primitive value",
        ))
    }

    /// ToBoolean (`#sec-toboolean`).
    pub(crate) fn to_boolean(&mut self, value: Value) -> Completion<bool> {
        Ok(match value {
            Value::Undefined | Value::Null => false,
            Value::Boolean(boolean) => boolean,
            Value::Number(number) => !(number == 0.0 || number.is_nan()),
            Value::String(string) => self.string_length(string)? != 0,
            Value::Symbol(_) | Value::Object(_) => true,
            Value::BigInt(_) => return Err(self.fault()),
        })
    }

    /// ToNumber (`#sec-tonumber`).
    pub(crate) fn to_number(&mut self, value: Value) -> Completion<f64> {
        match value {
            Value::Undefined => Ok(f64::NAN),
            Value::Null => Ok(0.0),
            Value::Boolean(boolean) => Ok(if boolean { 1.0 } else { 0.0 }),
            Value::Number(number) => Ok(number),
            Value::String(string) => self.string_to_number(string),
            Value::Symbol(_) => Err(self.throw_error(
                ErrorKind::TypeError,
                "Cannot convert a Symbol value to a number",
            )),
            Value::BigInt(_) => Err(self.throw_error(
                ErrorKind::TypeError,
                "Cannot convert a BigInt value to a number",
            )),
            Value::Object(_) => {
                let primitive = self.to_primitive(value, Some(PreferredType::Number))?;
                self.to_number(primitive)
            }
        }
    }

    /// StringToNumber (`#sec-stringtonumber`): the `StrWhiteSpace` at either end trimmed, the
    /// rest parsed by `swell-num`. A step per code unit, charged before the units are copied.
    pub(crate) fn string_to_number(&mut self, string: StringRef) -> Completion<f64> {
        let length = self.string_length(string)?;
        self.charge_steps(u64::try_from(length).unwrap_or(u64::MAX))?;
        let units = self.string_units(string)?;
        let start = units
            .iter()
            .position(|&unit| !is_str_white_space(unit))
            .unwrap_or(units.len());
        let end = units
            .iter()
            .rposition(|&unit| !is_str_white_space(unit))
            .map_or(start, |last| last + 1);
        let trimmed = units.get(start..end).unwrap_or(&[]);
        Ok(swell_num::literal::string_to_number_trimmed(trimmed))
    }

    /// ToNumeric (`#sec-tonumeric`). No BigInt exists before slice 12.
    pub(crate) fn to_numeric(&mut self, value: Value) -> Completion<Value> {
        let primitive = self.to_primitive(value, Some(PreferredType::Number))?;
        if let Value::BigInt(_) = primitive {
            return Ok(primitive);
        }
        Ok(Value::Number(self.to_number(primitive)?))
    }

    /// ToIntegerOrInfinity (`#sec-tointegerorinfinity`).
    pub(crate) fn to_integer_or_infinity(&mut self, value: Value) -> Completion<f64> {
        let number = self.to_number(value)?;
        if number.is_nan() || number == 0.0 {
            return Ok(0.0);
        }
        if number.is_infinite() {
            return Ok(number);
        }
        // Truncation keeps the sign of a value above -1, so -0.5 would give -0: step 4 answers
        // +0 for it.
        let integer = number.trunc();
        Ok(if integer == 0.0 { 0.0 } else { integer })
    }

    /// ToInt32 (`#sec-toint32`).
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "the bitwise operators: story:interpreter-and-script-entry-points"
        )
    )]
    pub(crate) fn to_int32(&mut self, value: Value) -> Completion<i32> {
        Ok(number_to_int32(self.to_number(value)?))
    }

    /// ToUint32 (`#sec-touint32`).
    pub(crate) fn to_uint32(&mut self, value: Value) -> Completion<u32> {
        Ok(number_to_uint32(self.to_number(value)?))
    }

    /// ToUint16 (`#sec-touint16`).
    pub(crate) fn to_uint16(&mut self, value: Value) -> Completion<u16> {
        // 1. Let int be ? ToIntegerOrInfinity(arg).
        let int = self.to_integer_or_infinity(value)?;
        // 2. Return 𝔽(ToFixedSizeInteger(int, unsigned, 16)).
        Ok(integer_to_uint16(int))
    }

    /// ToAbsoluteIndex (`#sec-toabsoluteindex`): an integer, +∞ or -∞. `length` is a Number: an
    /// array-like's length goes up to 2^53 - 1, which a `usize` does not hold on every target.
    pub(crate) fn to_absolute_index(&mut self, value: Value, length: f64) -> Completion<f64> {
        // 1. Let int be ? ToIntegerOrInfinity(value).
        let int = self.to_integer_or_infinity(value)?;
        // 2. If int is finite and int < 0, set int to length + int. 3. Return int.
        Ok(if int.is_finite() && int < 0.0 {
            length + int
        } else {
            int
        })
    }

    /// ToClampedIndex (`#sec-toclampedindex`), for the length of an array-like: an integer from
    /// 0 to `length`.
    pub(crate) fn to_clamped_index(&mut self, value: Value, length: f64) -> Completion<f64> {
        // 1. Let index be ? ToAbsoluteIndex(value, length).
        let index = self.to_absolute_index(value, length)?;
        // 2. Return the result of clamping index between 0 and length.
        Ok(index.clamp(0.0, length))
    }

    /// ToClampedIndex (`#sec-toclampedindex`) for a String's length, as a `usize`: a thin wrapper
    /// over the Number form, because String methods index units and Array methods compare Numbers.
    pub(crate) fn to_clamped_string_index(
        &mut self,
        value: Value,
        length: usize,
    ) -> Completion<usize> {
        // 1. Let index be ? ToAbsoluteIndex(value, length). A String's length is below 2^53.
        let index = self.to_absolute_index(value, length as f64)?;
        // 2. Return the result of clamping index between 0 and length.
        Ok(clamp_index(index, length))
    }

    /// ToString (`#sec-tostring`).
    pub(crate) fn to_string(&mut self, value: Value) -> Completion<StringRef> {
        match value {
            Value::Undefined => self.new_string_str("undefined"),
            Value::Null => self.new_string_str("null"),
            Value::Boolean(true) => self.new_string_str("true"),
            Value::Boolean(false) => self.new_string_str("false"),
            Value::Number(number) => {
                self.new_string_str(&swell_num::to_string::number_to_string(number))
            }
            Value::String(string) => Ok(string),
            Value::Symbol(_) => Err(self.throw_error(
                ErrorKind::TypeError,
                "Cannot convert a Symbol value to a string",
            )),
            Value::BigInt(_) => Err(self.fault()),
            Value::Object(_) => {
                let primitive = self.to_primitive(value, Some(PreferredType::String))?;
                self.to_string(primitive)
            }
        }
    }

    /// ToObject (`#sec-toobject`), with the prototypes of the current realm. A Symbol makes a
    /// Symbol object whose [[SymbolData]] is the Symbol (`#sec-properties-of-symbol-instances`,
    /// `story:symbol-constructor-and-well-known-symbols`). No BigInt exists before slice 12, so
    /// one here is a broken invariant.
    pub(crate) fn to_object(&mut self, value: Value) -> Completion<ObjectRef> {
        match value {
            Value::Undefined | Value::Null => Err(self.throw_error(
                ErrorKind::TypeError,
                "Cannot convert undefined or null to object",
            )),
            Value::Boolean(boolean) => {
                let prototype = self.current_intrinsic(Intrinsic::BooleanPrototype)?;
                self.ordinary_object_create(Some(prototype), OrdinaryData::Boolean(boolean))
            }
            Value::Number(number) => {
                let prototype = self.current_intrinsic(Intrinsic::NumberPrototype)?;
                self.ordinary_object_create(Some(prototype), OrdinaryData::Number(number))
            }
            Value::String(string) => {
                let prototype = self.current_intrinsic(Intrinsic::StringPrototype)?;
                self.string_create(string, prototype)
            }
            // "Return a new Symbol object whose [[SymbolData]] internal slot is set to argument."
            Value::Symbol(symbol) => {
                let prototype = self.current_intrinsic(Intrinsic::SymbolPrototype)?;
                self.ordinary_object_create(Some(prototype), OrdinaryData::Symbol(symbol))
            }
            Value::BigInt(_) => Err(self.fault()),
            Value::Object(object) => Ok(object),
        }
    }

    /// ToPropertyKey (`#sec-topropertykey`).
    pub(crate) fn to_property_key(&mut self, value: Value) -> Completion<Key> {
        let key = self.to_primitive(value, Some(PreferredType::String))?;
        if let Value::Symbol(symbol) = key {
            return Ok(Key::Symbol(symbol));
        }
        let string = self.to_string(key)?;
        self.key_from_string(string)
    }

    /// The key of the String `string`: its array index, or its atom. A step per code unit past
    /// [`FREE_CODE_UNITS`](crate::ops::testing::FREE_CODE_UNITS) is charged before a unit is
    /// read (`docs/design/engine.md`, "The step budget"); the String is hashed and compared in
    /// place, and its units are copied only into a new atom.
    pub(crate) fn key_from_string(&mut self, string: StringRef) -> Completion<Key> {
        let length = self.string_length(string)?;
        self.charge_code_units(length)?;
        // An array index has at most 10 code units (`array_index`).
        let mut short = [0u16; 10];
        if length <= short.len() {
            let copied = match self.heap.strings.get(string.handle) {
                Ok(record) => record.units().copy_to(&mut short),
                Err(_) => return Err(self.fault()),
            };
            if let Some(index) = array_index(short.get(..copied).unwrap_or(&[])) {
                return Ok(Key::Index(index));
            }
        }
        match self.heap.intern_string(string) {
            Ok(Ok(atom)) => Ok(Key::String(atom)),
            Ok(Err(_)) => Err(self.terminate(crate::host::Termination::MemoryBudget)),
            Err(_) => Err(self.fault()),
        }
    }

    /// The key of `units`: its array index, or its atom.
    fn key_from_units(&mut self, units: &[u16]) -> Completion<Key> {
        if let Some(index) = array_index(units) {
            return Ok(Key::Index(index));
        }
        match self.heap.intern(units) {
            Ok(atom) => Ok(Key::String(atom)),
            Err(_) => Err(self.terminate(crate::host::Termination::MemoryBudget)),
        }
    }

    /// The key of the property name `name`.
    pub(crate) fn name_key(&mut self, name: &str) -> Completion<Key> {
        let units: Vec<u16> = name.encode_utf16().collect();
        self.key_from_units(&units)
    }

    /// The value of a key: a String or a Symbol. An array index is made a String again.
    pub(crate) fn key_value(&mut self, key: Key) -> Completion<Value> {
        Ok(match key {
            Key::Index(index) => Value::String(self.new_string_str(&index.to_string())?),
            Key::String(string) => Value::String(string),
            Key::Symbol(symbol) => Value::Symbol(symbol),
        })
    }

    /// ToLength (`#sec-tolength`).
    pub(crate) fn to_length(&mut self, value: Value) -> Completion<f64> {
        let length = self.to_integer_or_infinity(value)?;
        if length <= 0.0 {
            return Ok(0.0);
        }
        Ok(length.min(9_007_199_254_740_991.0))
    }

    /// CanonicalNumericIndexString (`#sec-canonicalnumericindexstring`).
    #[cfg_attr(not(test), expect(dead_code, reason = "typed arrays: slice 13"))]
    pub(crate) fn canonical_numeric_index_string(
        &mut self,
        string: StringRef,
    ) -> Completion<Option<f64>> {
        let units = self.string_units(string)?;
        if units == [u16::from(b'-'), u16::from(b'0')] {
            return Ok(Some(-0.0));
        }
        let number = self.string_to_number(string)?;
        let canonical: Vec<u16> = swell_num::to_string::number_to_string(number)
            .encode_utf16()
            .collect();
        Ok((canonical == units).then_some(number))
    }
}

#[cfg(test)]
mod tests {
    use super::{clamp_index, number_to_int32, number_to_uint32};
    use crate::host::hooks::test_host::{agent_with_realm, enter};
    use crate::value::Value;

    /// ToUint16 (`#sec-touint16`) with ToFixedSizeInteger (`#sec-tofixedsizeinteger`):
    /// ToIntegerOrInfinity, then modulo 2^16; NaN and the infinities are 0.
    #[test]
    fn to_uint16_wraps_modulo_2_16() {
        let (mut agent, realm) = agent_with_realm();
        enter(&mut agent, &realm);
        for (number, uint16) in [
            (0.0, 0),
            (-0.0, 0),
            (f64::NAN, 0),
            (f64::INFINITY, 0),
            (f64::NEG_INFINITY, 0),
            (65.9, 65),
            (-0.9, 0),
            (-1.0, 65_535),
            (-65_535.0, 1),
            (65_535.0, 65_535),
            (65_536.0, 0),
            (65_537.5, 1),
            (4_294_967_361.0, 65),
            (-4_294_967_361.0, 65_471),
            (9_007_199_254_740_993.0, 0),
        ] {
            assert_eq!(
                agent.to_uint16(Value::Number(number)),
                Ok(uint16),
                "{number}"
            );
        }
        assert_eq!(agent.to_uint16(Value::Boolean(true)), Ok(1));
        assert_eq!(agent.to_uint16(Value::Undefined), Ok(0));
    }

    /// ToAbsoluteIndex and ToClampedIndex (`#sec-toabsoluteindex`, `#sec-toclampedindex`): a
    /// negative index counts from the length, the infinities stay, and the clamped index lies
    /// from 0 to the length.
    #[test]
    fn to_absolute_index_and_to_clamped_index_count_from_the_end() {
        let (mut agent, realm) = agent_with_realm();
        enter(&mut agent, &realm);
        for (number, absolute, clamped) in [
            (0.0, 0.0, 0),
            (2.9, 2.0, 2),
            (-1.0, 4.0, 4),
            (-5.0, 0.0, 0),
            (-6.5, -1.0, 0),
            (7.0, 7.0, 5),
            (f64::NAN, 0.0, 0),
            (f64::INFINITY, f64::INFINITY, 5),
            (f64::NEG_INFINITY, f64::NEG_INFINITY, 0),
        ] {
            assert_eq!(
                agent.to_absolute_index(Value::Number(number), 5.0),
                Ok(absolute),
                "{number}"
            );
            assert_eq!(
                agent.to_clamped_string_index(Value::Number(number), 5),
                Ok(clamped),
                "{number}"
            );
        }
        assert_eq!(clamp_index(3.0, 0), 0);
        assert_eq!(clamp_index(f64::NAN, 3), 0);
    }

    /// ToInt32 and ToUint32 (`#sec-toint32`, `#sec-touint32`): truncation modulo 2^32.
    #[test]
    fn to_int32_and_to_uint32_wrap_modulo_2_32() {
        for (number, int32, uint32) in [
            (0.0, 0, 0),
            (-0.0, 0, 0),
            (f64::NAN, 0, 0),
            (f64::INFINITY, 0, 0),
            (1.9, 1, 1),
            (-1.9, -1, 4_294_967_295),
            (2_147_483_648.0, -2_147_483_648, 2_147_483_648),
            (4_294_967_296.0, 0, 0),
            (4_294_967_297.5, 1, 1),
            (-4_294_967_297.0, -1, 4_294_967_295),
        ] {
            assert_eq!(number_to_int32(number), int32, "{number}");
            assert_eq!(number_to_uint32(number), uint32, "{number}");
        }
    }

    /// ToLength, ToIntegerOrInfinity and CanonicalNumericIndexString
    /// (`#sec-tolength`, `#sec-tointegerorinfinity`, `#sec-canonicalnumericindexstring`).
    #[test]
    fn to_length_and_canonical_numeric_index_string() {
        let (mut agent, realm) = agent_with_realm();
        enter(&mut agent, &realm);
        assert_eq!(agent.to_length(Value::Number(-3.0)), Ok(0.0));
        assert_eq!(agent.to_int32(Value::Number(-1.5)), Ok(-1));
        assert_eq!(agent.to_length(Value::Number(2.7)), Ok(2.0));
        assert_eq!(
            agent.to_length(Value::Number(f64::INFINITY)),
            Ok(9_007_199_254_740_991.0)
        );
        let negative_half = agent.to_integer_or_infinity(Value::Number(-0.5)).unwrap();
        assert!(negative_half == 0.0 && negative_half.is_sign_positive());
        for (text, expected) in [
            ("-0", Some(-0.0)),
            ("1", Some(1.0)),
            ("1.5", Some(1.5)),
            ("-1", Some(-1.0)),
            ("01", None),
            ("1.0", None),
            ("Infinity", Some(f64::INFINITY)),
            ("NaN", Some(f64::NAN)),
            ("x", None),
        ] {
            let string = agent.new_string_str(text).unwrap();
            let found = agent.canonical_numeric_index_string(string).unwrap();
            match (found, expected) {
                (Some(found), Some(expected)) if expected.is_nan() => assert!(found.is_nan()),
                (Some(found), Some(expected)) => {
                    assert!(
                        found == expected
                            && found.is_sign_negative() == expected.is_sign_negative(),
                        "{text}"
                    )
                }
                (None, None) => {}
                other => panic!("{text}: {other:?}"),
            }
        }
    }
}
