//! The access and search methods of `String` and `String.prototype` (ECMA-262 22.1.2 and
//! 22.1.3, `#sec-properties-of-the-string-constructor`,
//! `#sec-properties-of-the-string-prototype-object`), the group of
//! `story:string-access-and-search-methods`: `String.fromCharCode` (`#sec-string.fromcharcode`)
//! with ToUint16, `String.fromCodePoint` (`#sec-string.fromcodepoint`) and `String.raw`
//! (`#sec-string.raw`); of the prototype `at` (`#sec-string.prototype.at`), `charAt`
//! (`#sec-string.prototype.charat`), `charCodeAt` (`#sec-string.prototype.charcodeat`),
//! `codePointAt` (`#sec-string.prototype.codepointat`), `concat`
//! (`#sec-string.prototype.concat`), `endsWith` (`#sec-string.prototype.endswith`), `includes`
//! (`#sec-string.prototype.includes`), `isWellFormed` (`#sec-string.prototype.iswellformed`),
//! `lastIndexOf` (`#sec-string.prototype.lastindexof`) with StringLastIndexOf
//! (`#sec-stringlastindexof`), `slice` (`#sec-string.prototype.slice`), `startsWith`
//! (`#sec-string.prototype.startswith`), `substring` (`#sec-string.prototype.substring`) and
//! `toWellFormed` (`#sec-string.prototype.towellformed`); and the operations IsRegExp
//! (`#sec-isregexp`, whose step 4 reads the kind of a RegExp object, `object/regexp.rs`) and
//! IsStringWellFormedUnicode (`#sec-isstringwellformedunicode`), which no other file of the base
//! has. The keys follow those of `mod.rs`, in the order of the clauses.
//!
//! The budgets (`docs/design/engine.md`, decisions 3 and 5): every String a method makes is
//! built by [`StringBuilder`], so it is bounded by `string_length` and its copy is charged a
//! step per code unit; every code unit a method reads from a String is charged a step before it
//! is copied ([`Agent::read_units`]), so no method does work in proportion to a String's length
//! that the step budget does not see; a search charges a step per code unit it compares at each
//! start it tries, as StringIndexOf does. A String a method holds across the conversion of an
//! argument, which may run a `toString` or a `valueOf`, is on the kept stack.

use crate::builtins::object::argument;
use crate::builtins::string::{StringBuilder, code_point_at, utf16_encode_code_point};
use crate::builtins::{Method, SymbolData};
use crate::heap::accounting::ChargedVec;
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::host::{ErrorKind, Host};
use crate::object::property::Key;
use crate::object::string::Units;
use crate::ops::conversion::clamp_index;
use crate::value::{ObjectRef, StringRef, Value};

/// The builtins of this group, by their steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AccessBuiltin {
    /// `String.fromCharCode ( ...codeUnits )` (22.1.2.1).
    FromCharCode,
    /// `String.fromCodePoint ( ...codePoints )` (22.1.2.2).
    FromCodePoint,
    /// `String.raw ( template, ...substitutions )` (22.1.2.4).
    Raw,
    /// `String.prototype.at ( index )` (22.1.3.1).
    At,
    /// `String.prototype.charAt ( position )` (22.1.3.2).
    CharAt,
    /// `String.prototype.charCodeAt ( position )` (22.1.3.3).
    CharCodeAt,
    /// `String.prototype.codePointAt ( position )` (22.1.3.4).
    CodePointAt,
    /// `String.prototype.concat ( ...args )` (22.1.3.5).
    Concat,
    /// `String.prototype.endsWith ( searchString [ , endPosition ] )` (22.1.3.7).
    EndsWith,
    /// `String.prototype.includes ( searchString [ , position ] )` (22.1.3.8).
    Includes,
    /// `String.prototype.isWellFormed ( )` (22.1.3.10).
    IsWellFormed,
    /// `String.prototype.lastIndexOf ( searchString [ , position ] )` (22.1.3.11).
    LastIndexOf,
    /// `String.prototype.slice ( start, end )` (22.1.3.22).
    Slice,
    /// `String.prototype.startsWith ( searchString [ , position ] )` (22.1.3.24).
    StartsWith,
    /// `String.prototype.substring ( start, end )` (22.1.3.25).
    Substring,
    /// `String.prototype.toWellFormed ( )` (22.1.3.31).
    ToWellFormed,
}

/// The methods this group defines on `%String%`, in the order of their clauses.
pub(crate) const CONSTRUCTOR_METHODS: &[Method<AccessBuiltin>] = &[
    ("fromCharCode", 1, AccessBuiltin::FromCharCode),
    ("fromCodePoint", 1, AccessBuiltin::FromCodePoint),
    ("raw", 1, AccessBuiltin::Raw),
];

/// The methods this group defines on `%String.prototype%`, in the order of their clauses.
pub(crate) const PROTOTYPE_METHODS: &[Method<AccessBuiltin>] = &[
    ("at", 1, AccessBuiltin::At),
    ("charAt", 1, AccessBuiltin::CharAt),
    ("charCodeAt", 1, AccessBuiltin::CharCodeAt),
    ("codePointAt", 1, AccessBuiltin::CodePointAt),
    ("concat", 1, AccessBuiltin::Concat),
    ("endsWith", 1, AccessBuiltin::EndsWith),
    ("includes", 1, AccessBuiltin::Includes),
    ("isWellFormed", 0, AccessBuiltin::IsWellFormed),
    ("lastIndexOf", 1, AccessBuiltin::LastIndexOf),
    ("slice", 2, AccessBuiltin::Slice),
    ("startsWith", 1, AccessBuiltin::StartsWith),
    ("substring", 2, AccessBuiltin::Substring),
    ("toWellFormed", 0, AccessBuiltin::ToWellFormed),
];

/// The data properties keyed by a well-known symbol this group defines on
/// `%String.prototype%`, after its methods: none (`String.prototype[@@iterator]` is defined with
/// the base's keys, `builtins/string/mod.rs`).
pub(crate) fn prototype_symbol_data<H: Host>() -> [SymbolData<H>; 0] {
    []
}

/// The code unit U+FFFD (REPLACEMENT CHARACTER), which `toWellFormed` puts for a lone surrogate.
const REPLACEMENT_CHARACTER: u16 = 0xFFFD;

/// IsStringWellFormedUnicode ( string ) (`#sec-isstringwellformedunicode`), over the code units
/// of a String. Touches no heap; the caller charged the read of each code unit.
pub(crate) fn is_string_well_formed_unicode(string: &[u16]) -> bool {
    // 1. Let length be the length of string. 2. Let k be 0.
    let mut k = 0;
    // 3. Repeat, while k < length,
    while k < string.len() {
        // a. Let codePoint be CodePointAt(string, k).
        let Some(code_point) = code_point_at(string, k) else {
            return false;
        };
        // b. If codePoint.[[IsUnpairedSurrogate]] is true, return false.
        if code_point.is_unpaired_surrogate {
            return false;
        }
        // c. Set k to k + codePoint.[[CodeUnitCount]].
        k += code_point.code_unit_count;
    }
    // 4. Return true.
    true
}

/// The steps a String method is charged per start it tries in a search: a step per code unit of
/// the search String, and one for the empty String.
fn steps_per_candidate(search: &[u16]) -> u64 {
    u64::try_from(search.len()).unwrap_or(u64::MAX).max(1)
}

impl<H: Host> Agent<H> {
    /// The steps of the builtins of this group; not inlined, so that the frame of
    /// `run_string_builtin` does not grow with its arms. Each builtin is a function of its own,
    /// not inlined either, so that a re-entry through one of them pays its frame alone.
    #[inline(never)]
    pub(crate) fn run_string_access_builtin(
        &mut self,
        id: AccessBuiltin,
        _function: ObjectRef,
        this: Value,
        args: &[Value],
        _new_target: Option<ObjectRef>,
    ) -> Completion<Value> {
        match id {
            AccessBuiltin::FromCharCode => self.string_from_char_code(args),
            AccessBuiltin::FromCodePoint => self.string_from_code_point(args),
            AccessBuiltin::Raw => self.string_raw(args),
            AccessBuiltin::At => self.string_at(this, args),
            AccessBuiltin::CharAt => self.string_char_at(this, args),
            AccessBuiltin::CharCodeAt => self.string_char_code_at(this, args),
            AccessBuiltin::CodePointAt => self.string_code_point_at(this, args),
            AccessBuiltin::Concat => self.string_concat(this, args),
            AccessBuiltin::EndsWith => self.string_ends_with(this, args),
            AccessBuiltin::Includes => self.string_includes(this, args),
            AccessBuiltin::IsWellFormed => self.string_is_well_formed(this),
            AccessBuiltin::LastIndexOf => self.string_last_index_of_method(this, args),
            AccessBuiltin::Slice => self.string_slice(this, args),
            AccessBuiltin::StartsWith => self.string_starts_with(this, args),
            AccessBuiltin::Substring => self.string_substring(this, args),
            AccessBuiltin::ToWellFormed => self.string_to_well_formed(this),
        }
    }

    /// Run `steps` with `string` on the kept stack: it stays alive through any code they run.
    fn with_string_kept<T>(
        &mut self,
        string: StringRef,
        steps: impl FnOnce(&mut Self) -> Completion<T>,
    ) -> Completion<T> {
        let keep = self.keep(Value::String(string));
        let result = steps(self);
        self.release(keep);
        result
    }

    /// The code units of `string` from `from` to `to`, a step per code unit charged before they
    /// are copied, then their bytes held charged to the memory budget while the copy lives
    /// (`docs/design/engine.md`, "Memory budget"). A range past the end of the String reads
    /// nothing; the callers' ranges lie inside it.
    pub(crate) fn read_units(
        &mut self,
        string: StringRef,
        from: usize,
        to: usize,
    ) -> Completion<Units> {
        self.charge_steps(u64::try_from(to.saturating_sub(from)).unwrap_or(u64::MAX))?;
        let length = self.string_length(string)?;
        if from > to || to > length {
            return Ok(ChargedVec::new());
        }
        let mut units = self.new_charged_vec(to - from)?;
        self.extend_units(&mut units, string, from, to)?;
        Ok(units)
    }

    /// "The substring of string from from to to" (6.1.4), made by a [`StringBuilder`].
    fn substring_of(&mut self, string: StringRef, from: usize, to: usize) -> Completion<Value> {
        let units = self.read_units(string, from, to)?;
        let mut result = StringBuilder::new();
        self.append_units(&mut result, &units)?;
        Ok(Value::String(self.finish_string(result)?))
    }

    /// The empty String, made by a [`StringBuilder`].
    fn empty_string(&mut self) -> Completion<Value> {
        Ok(Value::String(self.finish_string(StringBuilder::new())?))
    }

    /// IsRegExp ( arg ) (`#sec-isregexp`).
    pub(crate) fn is_reg_exp(&mut self, value: Value) -> Completion<bool> {
        // 1. If arg is not an Object, return false.
        let Value::Object(object) = value else {
            return Ok(false);
        };
        // 2. Let matcher be ? Get(arg, %Symbol.match%). The well-known symbols live while a
        // realm does; without them no object has the key.
        let matcher = match self.well_known() {
            Some(symbols) => self.get(object, Key::Symbol(symbols.r#match))?,
            None => Value::Undefined,
        };
        // 3. If matcher is not undefined, return ToBoolean(matcher).
        if !matches!(matcher, Value::Undefined) {
            return self.to_boolean(matcher);
        }
        // 4. If arg has a [[RegExpMatcher]] internal slot, return true. 5. Return false.
        Ok(matches!(
            self.engine_object(object)?.kind,
            crate::object::ObjectKind::RegExp(_)
        ))
    }

    /// Steps 4 to 6 of `endsWith`, `includes` and `startsWith`, for the method `method`: "Let
    /// isRegexp be ? IsRegExp(searchString). If isRegexp is true, throw a TypeError exception.
    /// Set searchString to ? ToString(searchString)." The caller keeps S.
    fn search_string_not_reg_exp(&mut self, search: Value, method: &str) -> Completion<StringRef> {
        if self.is_reg_exp(search)? {
            return Err(self.throw_error(
                ErrorKind::TypeError,
                &format!("the first argument of {method} must not be a regular expression"),
            ));
        }
        self.to_string(search)
    }

    /// `String.fromCharCode ( ...codeUnits )` (`#sec-string.fromcharcode`).
    #[inline(never)]
    fn string_from_char_code(&mut self, args: &[Value]) -> Completion<Value> {
        // 1. Let result be the empty String.
        let mut result = StringBuilder::new();
        // 2. For each element next of codeUnits, do
        for &next in args {
            // a. Let nextCU be the code unit whose numeric value is ℝ(? ToUint16(next)).
            let unit = self.to_uint16(next)?;
            // b. Set result to the string-concatenation of result and nextCU.
            self.append_units(&mut result, &[unit])?;
        }
        // 3. Return result.
        Ok(Value::String(self.finish_string(result)?))
    }

    /// `String.fromCodePoint ( ...codePoints )` (`#sec-string.fromcodepoint`).
    #[inline(never)]
    fn string_from_code_point(&mut self, args: &[Value]) -> Completion<Value> {
        // 1. Let result be the empty String.
        let mut result = StringBuilder::new();
        // 2. For each element next of codePoints, do
        for &next in args {
            // a. Let nextCP be ? ToNumber(next).
            let number = self.to_number(next)?;
            // b. If nextCP is not an integral Number, throw a RangeError exception.
            // c. If ℝ(nextCP) < 0 or ℝ(nextCP) > 0x10FFFF, throw a RangeError exception.
            if !number.is_finite()
                || number.trunc() != number
                || !(0.0..=1_114_111.0).contains(&number)
            {
                return Err(self.throw_error(
                    ErrorKind::RangeError,
                    "String.fromCodePoint: the argument is not a code point",
                ));
            }
            // d. Set result to the string-concatenation of result and
            // UTF16EncodeCodePoint(ℝ(nextCP)). An integer from 0 to 0x10FFFF (-0 is 0) fits.
            let Some(encoded) = utf16_encode_code_point(number as u32) else {
                return Err(self.fault());
            };
            self.append_units(&mut result, encoded.units())?;
        }
        // 3. Assert: If codePoints is empty, then result is the empty String. 4. Return result.
        Ok(Value::String(self.finish_string(result)?))
    }

    /// `String.raw ( template, ...substitutions )` (`#sec-string.raw`).
    #[inline(never)]
    fn string_raw(&mut self, args: &[Value]) -> Completion<Value> {
        let substitutions = args.get(1..).unwrap_or_default();
        // 1. Let substitutionCount be the number of elements in substitutions.
        let substitution_count = substitutions.len();
        // 2. Let cooked be ? ToObject(template).
        let cooked = self.to_object(argument(args, 0))?;
        // 3. Let literals be ? ToObject(? Get(cooked, "raw")). A getter of "raw" runs with
        // cooked as its this value, which holds it.
        let raw_key = self.name_key("raw")?;
        let raw = self.get(cooked, raw_key)?;
        let literals = self.to_object(raw)?;
        let keep = self.keep(Value::Object(literals));
        let result = self.string_raw_steps(literals, substitutions, substitution_count);
        self.release(keep);
        result
    }

    /// Steps 4 to 8 of `String.raw`, with literals kept.
    fn string_raw_steps(
        &mut self,
        literals: ObjectRef,
        substitutions: &[Value],
        substitution_count: usize,
    ) -> Completion<Value> {
        // 4. Let literalCount be ? LengthOfArrayLike(literals).
        let literal_count = self.length_of_array_like(literals)?;
        // 5. If literalCount ≤ 0, return the empty String.
        if literal_count <= 0.0 {
            return self.empty_string();
        }
        // 6. Let result be the empty String. 7. Let nextIndex be 0.
        let mut result = StringBuilder::new();
        let mut next_index = 0_usize;
        // 8. Repeat, a step per literal: a literal that is the empty String copies nothing.
        loop {
            self.charge_steps(1)?;
            // a. Let nextLiteralValue be ? Get(literals, ! ToString(𝔽(nextIndex))).
            let key = self.index_key(next_index as f64)?;
            let value = self.get(literals, key)?;
            // b. Let nextLiteral be ? ToString(nextLiteralValue).
            let literal = self.to_string(value)?;
            // c. Set result to the string-concatenation of result and nextLiteral.
            self.append_string(&mut result, literal)?;
            // d. If nextIndex + 1 = literalCount, return result. literalCount is an integer
            // below 2^53, which next_index reaches exactly.
            if (next_index + 1) as f64 == literal_count {
                return Ok(Value::String(self.finish_string(result)?));
            }
            // e. If nextIndex < substitutionCount, then
            if next_index < substitution_count {
                // i. Let nextSubValue be substitutions[nextIndex].
                let value = substitutions
                    .get(next_index)
                    .copied()
                    .unwrap_or(Value::Undefined);
                // ii. Let nextSub be ? ToString(nextSubValue).
                let substitution = self.to_string(value)?;
                // iii. Set result to the string-concatenation of result and nextSub.
                self.append_string(&mut result, substitution)?;
            }
            // f. Set nextIndex to nextIndex + 1.
            next_index += 1;
        }
    }

    /// `String.prototype.at ( index )` (`#sec-string.prototype.at`).
    #[inline(never)]
    fn string_at(&mut self, this: Value, args: &[Value]) -> Completion<Value> {
        // 1, 2, 3. Let O be the this value; ? RequireObjectCoercible(O); let S be ? ToString(O).
        let string = self.this_string(this, "String.prototype.at")?;
        // 4. Let len be the length of S.
        let length = self.string_length(string)?;
        // 5. Let k be ? ToAbsoluteIndex(index, len).
        let k = self.with_string_kept(string, |agent| {
            agent.to_absolute_index(argument(args, 0), length as f64)
        })?;
        // 6. If k < 0 or k ≥ len, return undefined.
        if k < 0.0 || k >= length as f64 {
            return Ok(Value::Undefined);
        }
        // 7. Return the substring of S from k to k + 1. 0 ≤ k < len: an integer that fits.
        let k = k as usize;
        self.substring_of(string, k, k + 1)
    }

    /// Steps 1 to 6 of `charAt`, `charCodeAt` and `codePointAt`: S, and the position when it
    /// lies inside S.
    fn string_and_position(
        &mut self,
        this: Value,
        args: &[Value],
        method: &str,
    ) -> Completion<(StringRef, Option<usize>)> {
        // 1, 2, 3. Let O be the this value; ? RequireObjectCoercible(O); let S be ? ToString(O).
        let string = self.this_string(this, method)?;
        // 4. Set position to ? ToIntegerOrInfinity(position).
        let position = self.with_string_kept(string, |agent| {
            agent.to_integer_or_infinity(argument(args, 0))
        })?;
        // 5. Let size be the length of S. 6. If position < 0 or position ≥ size, the method
        // answers its value for no code unit.
        let size = self.string_length(string)?;
        if position < 0.0 || position >= size as f64 {
            return Ok((string, None));
        }
        // 0 ≤ position < size: an integer that fits.
        Ok((string, Some(position as usize)))
    }

    /// `String.prototype.charAt ( position )` (`#sec-string.prototype.charat`).
    #[inline(never)]
    fn string_char_at(&mut self, this: Value, args: &[Value]) -> Completion<Value> {
        match self.string_and_position(this, args, "String.prototype.charAt")? {
            // 6. If position < 0 or position ≥ size, return the empty String.
            (_, None) => self.empty_string(),
            // 7. Return the substring of S from position to position + 1.
            (string, Some(position)) => self.substring_of(string, position, position + 1),
        }
    }

    /// `String.prototype.charCodeAt ( position )` (`#sec-string.prototype.charcodeat`).
    #[inline(never)]
    fn string_char_code_at(&mut self, this: Value, args: &[Value]) -> Completion<Value> {
        match self.string_and_position(this, args, "String.prototype.charCodeAt")? {
            // 6. If position < 0 or position ≥ size, return NaN.
            (_, None) => Ok(Value::Number(f64::NAN)),
            // 7. Return the Number value for the numeric value of the code unit at index
            // position within the String S.
            (string, Some(position)) => {
                let units = self.read_units(string, position, position + 1)?;
                match units.first() {
                    Some(&unit) => Ok(Value::Number(f64::from(unit))),
                    None => Err(self.fault()),
                }
            }
        }
    }

    /// `String.prototype.codePointAt ( position )` (`#sec-string.prototype.codepointat`).
    #[inline(never)]
    fn string_code_point_at(&mut self, this: Value, args: &[Value]) -> Completion<Value> {
        match self.string_and_position(this, args, "String.prototype.codePointAt")? {
            // 6. If position < 0 or position ≥ size, return undefined.
            (_, None) => Ok(Value::Undefined),
            (string, Some(position)) => {
                // 7. Let cp be CodePointAt(S, position): it reads at most two code units.
                let size = self.string_length(string)?;
                let units = self.read_units(string, position, size.min(position + 2))?;
                let Some(code_point) = code_point_at(&units, 0) else {
                    return Err(self.fault());
                };
                // 8. Return 𝔽(cp.[[CodePoint]]).
                Ok(Value::Number(f64::from(code_point.code_point)))
            }
        }
    }

    /// `String.prototype.concat ( ...args )` (`#sec-string.prototype.concat`).
    #[inline(never)]
    fn string_concat(&mut self, this: Value, args: &[Value]) -> Completion<Value> {
        // 1, 2, 3. Let O be the this value; ? RequireObjectCoercible(O); let S be ? ToString(O).
        let string = self.this_string(this, "String.prototype.concat")?;
        // 4. Let R be S. Its code units are copied before any code runs, so S needs no root.
        let mut result = StringBuilder::new();
        self.append_string(&mut result, string)?;
        // 5. For each element next of args, do
        for &next in args {
            // a. Let nextString be ? ToString(next).
            let next_string = self.to_string(next)?;
            // b. Set R to the string-concatenation of R and nextString.
            self.append_string(&mut result, next_string)?;
        }
        // 6. Return R.
        Ok(Value::String(self.finish_string(result)?))
    }

    /// `String.prototype.endsWith ( searchString [ , endPosition ] )`
    /// (`#sec-string.prototype.endswith`).
    #[inline(never)]
    fn string_ends_with(&mut self, this: Value, args: &[Value]) -> Completion<Value> {
        // 1, 2, 3. Let O be the this value; ? RequireObjectCoercible(O); let S be ? ToString(O).
        let string = self.this_string(this, "String.prototype.endsWith")?;
        self.with_string_kept(string, |agent| agent.ends_with_steps(string, args))
    }

    /// Steps 4 to 16 of `endsWith`, with S kept.
    fn ends_with_steps(&mut self, string: StringRef, args: &[Value]) -> Completion<Value> {
        // 4, 5, 6. IsRegExp, then searchString ToString.
        let search =
            self.search_string_not_reg_exp(argument(args, 0), "String.prototype.endsWith")?;
        // 7. Let len be the length of S.
        let length = self.string_length(string)?;
        // 8. If endPosition is undefined, let pos be len; else let pos be ?
        // ToIntegerOrInfinity(endPosition). 9. Let end be the result of clamping pos between 0
        // and len.
        let end = match argument(args, 1) {
            Value::Undefined => length,
            position => clamp_index(
                self.with_string_kept(search, |agent| agent.to_integer_or_infinity(position))?,
                length,
            ),
        };
        // 10. Let searchLength be the length of searchStr. 11. If searchLength = 0, return true.
        let search_length = self.string_length(search)?;
        if search_length == 0 {
            return Ok(Value::Boolean(true));
        }
        // 12. Let start be end - searchLength. 13. If start < 0, return false.
        let Some(start) = end.checked_sub(search_length) else {
            return Ok(Value::Boolean(false));
        };
        // 14. Let substring be the substring of S from start to end. 15. If substring is
        // searchStr, return true. 16. Return false.
        let candidate = self.read_units(string, start, end)?;
        let search = self.read_units(search, 0, search_length)?;
        Ok(Value::Boolean(candidate[..] == search[..]))
    }

    /// `String.prototype.includes ( searchString [ , position ] )`
    /// (`#sec-string.prototype.includes`).
    #[inline(never)]
    fn string_includes(&mut self, this: Value, args: &[Value]) -> Completion<Value> {
        // 1, 2, 3. Let O be the this value; ? RequireObjectCoercible(O); let S be ? ToString(O).
        let string = self.this_string(this, "String.prototype.includes")?;
        self.with_string_kept(string, |agent| agent.includes_steps(string, args))
    }

    /// Steps 4 to 12 of `includes`, with S kept.
    fn includes_steps(&mut self, string: StringRef, args: &[Value]) -> Completion<Value> {
        // 4, 5, 6. IsRegExp, then searchString ToString.
        let search =
            self.search_string_not_reg_exp(argument(args, 0), "String.prototype.includes")?;
        // 7. Let len be the length of S.
        let length = self.string_length(string)?;
        // 8. Let pos be ? ToIntegerOrInfinity(position). 9. Assert: If position is undefined,
        // then pos is 0. 10. Let start be the result of clamping pos between 0 and len.
        let position = self.with_string_kept(search, |agent| {
            agent.to_integer_or_infinity(argument(args, 1))
        })?;
        let start = clamp_index(position, length);
        // 11. Let index be StringIndexOf(S, searchStr, start): the search of the code units
        // from start on, from their first. 12. If index is not-found, return false. 13. Return
        // true.
        let search_length = self.string_length(search)?;
        let units = self.read_units(string, start, length)?;
        let search = self.read_units(search, 0, search_length)?;
        Ok(Value::Boolean(
            self.string_index_of(&units, &search, 0)?.is_some(),
        ))
    }

    /// `String.prototype.isWellFormed ( )` (`#sec-string.prototype.iswellformed`).
    #[inline(never)]
    fn string_is_well_formed(&mut self, this: Value) -> Completion<Value> {
        // 1, 2, 3. Let O be the this value; ? RequireObjectCoercible(O); let S be ? ToString(O).
        let string = self.this_string(this, "String.prototype.isWellFormed")?;
        // 4. Return IsStringWellFormedUnicode(S).
        let length = self.string_length(string)?;
        let units = self.read_units(string, 0, length)?;
        Ok(Value::Boolean(is_string_well_formed_unicode(&units)))
    }

    /// `String.prototype.lastIndexOf ( searchString [ , position ] )`
    /// (`#sec-string.prototype.lastindexof`).
    #[inline(never)]
    fn string_last_index_of_method(&mut self, this: Value, args: &[Value]) -> Completion<Value> {
        // 1, 2, 3. Let O be the this value; ? RequireObjectCoercible(O); let S be ? ToString(O).
        let string = self.this_string(this, "String.prototype.lastIndexOf")?;
        self.with_string_kept(string, |agent| agent.last_index_of_steps(string, args))
    }

    /// Steps 4 to 14 of `lastIndexOf`, with S kept.
    fn last_index_of_steps(&mut self, string: StringRef, args: &[Value]) -> Completion<Value> {
        // 4. Let searchStr be ? ToString(searchString).
        let search = self.to_string(argument(args, 0))?;
        // 5. Let numPos be ? ToNumber(position).
        let number_position =
            self.with_string_kept(search, |agent| agent.to_number(argument(args, 1)))?;
        // 6. Let len be the length of S. 7. Let searchLen be the length of searchStr.
        let length = self.string_length(string)?;
        let search_length = self.string_length(search)?;
        // 8. Let maxStart be len - searchLen. 9. If maxStart < 0, return -1𝔽.
        let Some(max_start) = length.checked_sub(search_length) else {
            return Ok(Value::Number(-1.0));
        };
        // 10. If numPos is NaN, let start be maxStart; else let start be the result of clamping
        // ! ToIntegerOrInfinity(numPos) between 0 and maxStart. 11. Assert: If position is
        // undefined, then start is maxStart.
        let start = if number_position.is_nan() {
            max_start
        } else {
            clamp_index(
                self.to_integer_or_infinity(Value::Number(number_position))?,
                max_start,
            )
        };
        // 12. Let result be StringLastIndexOf(S, searchStr, start): it reads no code unit at or
        // after start + searchLen. 13. If result is not-found, return -1𝔽. 14. Return
        // 𝔽(result).
        let units = self.read_units(string, 0, start + search_length)?;
        let search = self.read_units(search, 0, search_length)?;
        Ok(Value::Number(
            match self.string_last_index_of(&units, &search, start)? {
                // An index of a String is below 2^53, so the conversion is exact.
                Some(index) => index as f64,
                None => -1.0,
            },
        ))
    }

    /// StringLastIndexOf ( string, searchValue, fromIndex ) (`#sec-stringlastindexof`): the
    /// last index at or before `from` where `search` occurs in `string`. A step per candidate
    /// index and per code unit compared.
    pub(crate) fn string_last_index_of(
        &mut self,
        string: &[u16],
        search: &[u16],
        from: usize,
    ) -> Completion<Option<usize>> {
        // 1. Let len be the length of string. 2. Let searchLen be the length of searchValue.
        // 3. Assert: fromIndex + searchLen ≤ len: a start past it is not tried.
        let per_candidate = steps_per_candidate(search);
        // 4. For each integer i such that 0 ≤ i ≤ fromIndex, in descending order, do
        for index in (0..=from).rev() {
            self.charge_steps(per_candidate)?;
            // a. Let candidate be the substring of string from i to i + searchLen. b. If
            // candidate is searchValue, return i.
            if string.get(index..index + search.len()) == Some(search) {
                return Ok(Some(index));
            }
        }
        // 5. Return not-found.
        Ok(None)
    }

    /// `String.prototype.slice ( start, end )` (`#sec-string.prototype.slice`).
    #[inline(never)]
    fn string_slice(&mut self, this: Value, args: &[Value]) -> Completion<Value> {
        // 1, 2, 3. Let O be the this value; ? RequireObjectCoercible(O); let S be ? ToString(O).
        let string = self.this_string(this, "String.prototype.slice")?;
        // 4. Let len be the length of S.
        let length = self.string_length(string)?;
        let (from, to) = self.with_string_kept(string, |agent| {
            // 5. Let from be ? ToClampedIndex(start, len).
            let from = agent.to_clamped_string_index(argument(args, 0), length)?;
            // 6. If end is undefined, let to be len; else let to be ? ToClampedIndex(end, len).
            let to = match argument(args, 1) {
                Value::Undefined => length,
                end => agent.to_clamped_string_index(end, length)?,
            };
            Ok((from, to))
        })?;
        // 7. If from ≥ to, return the empty String.
        if from >= to {
            return self.empty_string();
        }
        // 8. Return the substring of S from from to to.
        self.substring_of(string, from, to)
    }

    /// `String.prototype.startsWith ( searchString [ , position ] )`
    /// (`#sec-string.prototype.startswith`).
    #[inline(never)]
    fn string_starts_with(&mut self, this: Value, args: &[Value]) -> Completion<Value> {
        // 1, 2, 3. Let O be the this value; ? RequireObjectCoercible(O); let S be ? ToString(O).
        let string = self.this_string(this, "String.prototype.startsWith")?;
        self.with_string_kept(string, |agent| agent.starts_with_steps(string, args))
    }

    /// Steps 4 to 16 of `startsWith`, with S kept.
    fn starts_with_steps(&mut self, string: StringRef, args: &[Value]) -> Completion<Value> {
        // 4, 5, 6. IsRegExp, then searchString ToString.
        let search =
            self.search_string_not_reg_exp(argument(args, 0), "String.prototype.startsWith")?;
        // 7. Let len be the length of S.
        let length = self.string_length(string)?;
        // 8. Let pos be ? ToIntegerOrInfinity(position). 9. Assert: If position is undefined,
        // then pos is 0. 10. Let start be the result of clamping pos between 0 and len.
        let position = self.with_string_kept(search, |agent| {
            agent.to_integer_or_infinity(argument(args, 1))
        })?;
        let start = clamp_index(position, length);
        // 11. Let searchLength be the length of searchStr. 12. If searchLength = 0, return
        // true.
        let search_length = self.string_length(search)?;
        if search_length == 0 {
            return Ok(Value::Boolean(true));
        }
        // 13. Let end be start + searchLength. 14. If end > len, return false.
        let end = start + search_length;
        if end > length {
            return Ok(Value::Boolean(false));
        }
        // 15. Let substring be the substring of S from start to end. 16. If substring is
        // searchStr, return true. 17. Return false.
        let candidate = self.read_units(string, start, end)?;
        let search = self.read_units(search, 0, search_length)?;
        Ok(Value::Boolean(candidate[..] == search[..]))
    }

    /// `String.prototype.substring ( start, end )` (`#sec-string.prototype.substring`).
    #[inline(never)]
    fn string_substring(&mut self, this: Value, args: &[Value]) -> Completion<Value> {
        // 1, 2, 3. Let O be the this value; ? RequireObjectCoercible(O); let S be ? ToString(O).
        let string = self.this_string(this, "String.prototype.substring")?;
        // 4. Let len be the length of S.
        let length = self.string_length(string)?;
        let (final_start, final_end) = self.with_string_kept(string, |agent| {
            // 5. Let intStart be ? ToIntegerOrInfinity(start).
            let start = agent.to_integer_or_infinity(argument(args, 0))?;
            // 6. If end is undefined, let intEnd be len; else let intEnd be ?
            // ToIntegerOrInfinity(end).
            let end = match argument(args, 1) {
                Value::Undefined => length,
                end => clamp_index(agent.to_integer_or_infinity(end)?, length),
            };
            // 7. Let finalStart be the result of clamping intStart between 0 and len. 8. Let
            // finalEnd be the result of clamping intEnd between 0 and len.
            Ok((clamp_index(start, length), end))
        })?;
        // 9. Let from be min(finalStart, finalEnd). 10. Let to be max(finalStart, finalEnd).
        // 11. Return the substring of S from from to to.
        self.substring_of(
            string,
            final_start.min(final_end),
            final_start.max(final_end),
        )
    }

    /// `String.prototype.toWellFormed ( )` (`#sec-string.prototype.towellformed`).
    #[inline(never)]
    fn string_to_well_formed(&mut self, this: Value) -> Completion<Value> {
        // 1, 2, 3. Let O be the this value; ? RequireObjectCoercible(O); let S be ? ToString(O).
        let string = self.this_string(this, "String.prototype.toWellFormed")?;
        // 4. Let strLen be the length of S.
        let length = self.string_length(string)?;
        let units = self.read_units(string, 0, length)?;
        // 5. Let k be 0. 6. Let result be the empty String.
        let mut k = 0;
        let mut result = StringBuilder::new();
        // 7. Repeat, while k < strLen,
        while k < units.len() {
            // a. Let cp be CodePointAt(S, k).
            let Some(code_point) = code_point_at(&units, k) else {
                return Err(self.fault());
            };
            if code_point.is_unpaired_surrogate {
                // b. If cp.[[IsUnpairedSurrogate]] is true, then set result to the
                // string-concatenation of result and 0xFFFD (REPLACEMENT CHARACTER).
                self.append_units(&mut result, &[REPLACEMENT_CHARACTER])?;
            } else {
                // c. Else, set result to the string-concatenation of result and
                // UTF16EncodeCodePoint(cp.[[CodePoint]]).
                let Some(encoded) = utf16_encode_code_point(code_point.code_point) else {
                    return Err(self.fault());
                };
                self.append_units(&mut result, encoded.units())?;
            }
            // d. Set k to k + cp.[[CodeUnitCount]].
            k += code_point.code_unit_count;
        }
        // 8. Return result.
        Ok(Value::String(self.finish_string(result)?))
    }
}

#[cfg(test)]
mod tests {
    use super::is_string_well_formed_unicode;
    use crate::host::Termination;
    use crate::host::agent::Agent;
    use crate::host::hooks::test_host::{
        NoHooks, agent_with_realm, agent_with_realm_and, enter, limits,
    };
    use crate::host::outcome::Abrupt;
    use crate::object::ordinary::OrdinaryData;
    use crate::object::property::Key;
    use crate::realm::intrinsics::Intrinsic;
    use crate::value::{ObjectRef, Value};

    /// The text of a String value.
    fn text(agent: &mut Agent<NoHooks>, value: Value) -> String {
        let Value::String(string) = value else {
            panic!("{value:?} is not a String");
        };
        String::from_utf16_lossy(&agent.string_units_for_tests(string).unwrap())
    }

    /// A new ordinary object whose [[Prototype]] is `%Object.prototype%`, kept, with the
    /// property `@@match` of value `matcher` when there is one.
    fn object_matching(
        agent: &mut Agent<NoHooks>,
        prototype: ObjectRef,
        matcher: Option<Value>,
    ) -> ObjectRef {
        let object = agent
            .ordinary_object_create(Some(prototype), OrdinaryData::Plain)
            .unwrap();
        agent.keep(Value::Object(object));
        if let Some(matcher) = matcher {
            let symbols = agent.well_known().unwrap();
            agent
                .create_data_property_or_throw(object, Key::Symbol(symbols.r#match), matcher)
                .unwrap();
        }
        object
    }

    /// `#sec-isregexp` in `includes`, `startsWith` and `endsWith` (step 5 of each): an object
    /// whose `@@match` is truthy (true, a String, a String object) is taken for a RegExp and
    /// the method throws a `TypeError` before the search String is converted; a falsy one (false,
    /// 0, the empty String) is not, and the object is converted by ToString ("[object
    /// Object]"); IsRegExp of a value that is no object is false without a Get.
    #[test]
    fn includes_starts_with_and_ends_with_refuse_what_is_reg_exp_by_match() {
        let (mut agent, realm) = agent_with_realm();
        enter(&mut agent, &realm);
        let handle = agent.heap.roots.get_realm(&realm.0).unwrap();
        let prototype = agent.intrinsic(handle, Intrinsic::StringPrototype).unwrap();
        let object_prototype = agent.intrinsic(handle, Intrinsic::ObjectPrototype).unwrap();
        let subject = agent.new_string_str("[object Object]").unwrap();
        agent.keep(Value::String(subject));
        let truthy_text = agent.new_string_str("x").unwrap();
        let empty = agent.new_string_str("").unwrap();
        agent.keep(Value::String(truthy_text));
        agent.keep(Value::String(empty));
        let wrapper = agent.to_object(Value::String(empty)).unwrap();
        agent.keep(Value::Object(wrapper));
        for name in ["includes", "startsWith", "endsWith"] {
            let key = agent.name_key(name).unwrap();
            let method = agent.get(prototype, key).unwrap();
            agent.keep(method);
            for matcher in [
                Value::Boolean(true),
                Value::String(truthy_text),
                Value::Number(1.0),
                Value::Object(wrapper),
            ] {
                let search = object_matching(&mut agent, object_prototype, Some(matcher));
                let thrown =
                    agent.call_value(method, Value::String(subject), &[Value::Object(search)]);
                let Err(Abrupt::Throw(error)) = thrown else {
                    panic!("{name} with @@match {matcher:?}: {thrown:?}");
                };
                let Value::Object(error) = error else {
                    panic!("{name}: {error:?}");
                };
                let name_key = agent.name_key("name").unwrap();
                let error_name = agent.get(error, name_key).unwrap();
                assert_eq!(
                    text(&mut agent, error_name),
                    "TypeError",
                    "{name} with {matcher:?}"
                );
            }
            for matcher in [
                None,
                Some(Value::Undefined),
                Some(Value::Boolean(false)),
                Some(Value::Number(0.0)),
                Some(Value::String(empty)),
            ] {
                let search = object_matching(&mut agent, object_prototype, matcher);
                assert_eq!(
                    agent.call_value(method, Value::String(subject), &[Value::Object(search)]),
                    Ok(Value::Boolean(true)),
                    "{name} with @@match {matcher:?}"
                );
            }
        }
        for value in [
            Value::Undefined,
            Value::Null,
            Value::Boolean(true),
            Value::Number(1.0),
            Value::String(truthy_text),
        ] {
            assert_eq!(agent.is_reg_exp(value), Ok(false), "{value:?}");
        }
    }

    /// StringLastIndexOf (`#sec-stringlastindexof`): the last start at or before the index at
    /// which the search String occurs; the empty String is found at the index; a step per start
    /// tried and per code unit of the search String.
    #[test]
    fn string_last_index_of_searches_down_from_the_index() {
        let (mut agent, realm) = agent_with_realm();
        enter(&mut agent, &realm);
        let string = [0x61, 0x62, 0x61, 0x62, 0x63];
        assert_eq!(
            agent.string_last_index_of(&string, &[0x61, 0x62], 3),
            Ok(Some(2))
        );
        assert_eq!(
            agent.string_last_index_of(&string, &[0x61, 0x62], 1),
            Ok(Some(0))
        );
        assert_eq!(agent.string_last_index_of(&string, &[0x63], 4), Ok(Some(4)));
        assert_eq!(agent.string_last_index_of(&string, &[0x63], 3), Ok(None));
        assert_eq!(agent.string_last_index_of(&string, &[], 5), Ok(Some(5)));
        assert_eq!(agent.string_last_index_of(&string, &[], 0), Ok(Some(0)));
        assert_eq!(agent.string_last_index_of(&[], &[], 0), Ok(Some(0)));
        // Four starts of a search String of two units that is not there are eight steps, within
        // a budget of nine; five starts, ten steps, pass it.
        let mut limits = limits();
        limits.steps_per_entry = 9;
        let (mut agent, realm) = agent_with_realm_and(limits);
        enter(&mut agent, &realm);
        agent.steps = 0;
        assert_eq!(
            agent.string_last_index_of(&[0x61; 5], &[0x62, 0x62], 3),
            Ok(None)
        );
        assert_eq!(agent.steps, 8);
        agent.steps = 0;
        assert_eq!(
            agent.string_last_index_of(&[0x61; 6], &[0x62, 0x62], 4),
            Err(Abrupt::Terminated(Termination::StepBudget))
        );
    }

    /// IsStringWellFormedUnicode (`#sec-isstringwellformedunicode`): a String with a lone
    /// leading or trailing surrogate is not well formed; pairs and other code units are.
    #[test]
    fn is_string_well_formed_unicode_finds_a_lone_surrogate() {
        assert!(is_string_well_formed_unicode(&[]));
        assert!(is_string_well_formed_unicode(&[
            0x61, 0xD83D, 0xDE00, 0xFFFF
        ]));
        assert!(!is_string_well_formed_unicode(&[0xD800]));
        assert!(!is_string_well_formed_unicode(&[0xDC00, 0xD800]));
        assert!(!is_string_well_formed_unicode(&[0x61, 0xD800, 0x62]));
        assert!(!is_string_well_formed_unicode(&[0xD800, 0xD800, 0xDC00]));
    }

    /// `docs/design/engine.md`, "Memory budget": the builder of `concat`
    /// (`#sec-string.prototype.concat`) is charged while it lives, so four levels of `concat`,
    /// each holding a String of 65,536 code units while ToString of its argument runs code, do
    /// not fit a room of three such copies and end in `Terminated(MemoryBudget)`; the builder of
    /// `String.fromCharCode` (`#sec-string.fromcharcode`), 64 code units when `valueOf` of its
    /// last argument runs code, leaves no capacity uncovered at any entry into script.
    #[test]
    fn a_builder_is_charged_across_a_call_of_to_string_and_value_of() {
        use crate::builtins::string::transform::tests::Answer;
        use crate::object::string::across::Across;
        use crate::object::string::copies;
        let mut across = Across::with_steps(4_000_000);
        across.string("s", &vec![0x61; 65_536]);
        let script = across.parse(
            "var d = 0; var o = { toString: function () { if (++d < 4) s.concat(o); return \
             \"e\"; } }; s.concat(o).length",
        );
        across.room(3 * 131_072);
        assert_eq!(
            across.evaluate(&script),
            Answer::Terminated(Termination::MemoryBudget)
        );

        let mut across = Across::with_steps(4_000_000);
        let codes = vec!["65"; 64].join(", ");
        let answer = across.run(&format!(
            "var d = 0; var v = {{ valueOf: function () {{ if (++d < 4) \
             String.fromCharCode({codes}, v); return 66; }} }}; String.fromCharCode({codes}, \
             v).length"
        ));
        assert_eq!(answer, Answer::Normal("Number(65.0)".to_string()));
        assert_eq!(copies::uncovered_at_entry(), 0);
    }
}
