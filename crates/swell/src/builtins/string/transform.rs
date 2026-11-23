//! The transform and compare methods of `String.prototype` (ECMA-262 22.1.3,
//! `#sec-properties-of-the-string-prototype-object`), the group of
//! `story:string-transform-and-compare-methods`: `localeCompare`
//! (`#sec-string.prototype.localecompare`), `normalize` (`#sec-string.prototype.normalize`,
//! `story:normalization-tables-normalize-and-locale-compare`), `padEnd`
//! (`#sec-string.prototype.padend`), `padStart`
//! (`#sec-string.prototype.padstart`) with StringPaddingBuiltinsImpl
//! (`#sec-stringpaddingbuiltinsimpl`) and StringPad (`#sec-stringpad`), `repeat`
//! (`#sec-string.prototype.repeat`), `toLocaleLowerCase` (`#sec-string.prototype.tolocalelowercase`),
//! `toLocaleUpperCase` (`#sec-string.prototype.tolocaleuppercase`), `toLowerCase`
//! (`#sec-string.prototype.tolowercase`), `toUpperCase` (`#sec-string.prototype.touppercase`),
//! `trim` (`#sec-string.prototype.trim`) with TrimString (`#sec-trimstring`), `trimEnd`
//! (`#sec-string.prototype.trimend`) and `trimStart` (`#sec-string.prototype.trimstart`). The
//! group defines nothing on `%String%`.
//!
//! Without ECMA-402 (`docs/design/engine.md`, section 1) the two locale case methods answer as
//! `toLowerCase` and `toUpperCase`, and the arguments ECMA-402 gives a meaning (those of the
//! locale case methods, the second and third of `localeCompare`) are not read.
//!
//! The four case methods map every code point of a String by the case tables of
//! `swell-unicode` (the locale-insensitive mappings of `UnicodeData.txt` and `SpecialCasing.txt`,
//! with the context `Final_Sigma`). `normalize` makes the four forms of UAX #15 by the
//! normalization tables of `swell-unicode`, and `localeCompare` orders two Strings by the code
//! units of their NFC forms (`docs/design/engine.md`, decision 5, "The order `localeCompare`
//! gives"). No method of the group refuses an input: the refusals of slice 3 (section 12, "A part
//! before its slice") are gone.
//!
//! The normalization of `normalize` and `localeCompare` is made in buffers of the method, not
//! in Strings of the heap: the full decomposition as code points, and for `localeCompare` the
//! NFC form of each String as code units. Each buffer grows by doubling its room, at least 16,
//! and the bytes of the room are charged to the memory budget before it grows (a form can be
//! longer than its String, so nothing is charged ahead); the method credits every byte when it
//! returns, by an answer, a throw or a stop. `string_length` does not bound these buffers;
//! `normalize` writes its result through [`StringBuilder`], which it bounds.
//!
//! Limits: a result whose length is known before it is built (`repeat`, the pads) is asked of
//! `string_length` and of the memory budget as a whole before a unit is copied, so a result above
//! `string_length` is a `RangeError` at once (`docs/design/host-interface.md`, section 2,
//! `HOST-21`) and one the memory budget cannot hold ends the run in `Terminated(MemoryBudget)`.
//! A case method's result, whose length is known only as it is built, grows through
//! [`StringBuilder`]: past `string_length` a `RangeError`, and the memory budget asked before the
//! buffer grows. Steps: a step per code unit copied or written into a result
//! ([`StringBuilder`]); the trims, the case methods, `normalize` and `localeCompare` also a step
//! per code unit of each String they read, charged before the read; `normalize` and
//! `localeCompare` a step per code unit written into a buffer of the normalization, charged
//! before it is written (the comparison of two forms charges nothing more).

use swell_unicode::{
    Buffer, CaseMapping, Form, FormBuffer, Stop, decompose, final_sigma_lowercase,
    is_case_ignorable, is_cased, reorder_and_compose, to_lowercase, to_uppercase,
};

use crate::builtins::object::argument;
use crate::builtins::string::{StringBuilder, code_point_at, utf16_encode_code_point};
use crate::builtins::{Method, SymbolData};
use crate::host::agent::Agent;
use crate::host::outcome::{Abrupt, Completion};
use crate::host::{ErrorKind, Host, Termination};
use crate::ops::conversion::is_str_white_space;
use crate::value::{ObjectRef, StringRef, Value};

/// The builtins of this group, by their steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TransformBuiltin {
    /// `String.prototype.localeCompare ( that [ , reserved1 [ , reserved2 ] ] )` (22.1.3.12).
    LocaleCompare,
    /// `String.prototype.normalize ( [ form ] )` (22.1.3.15).
    Normalize,
    /// `String.prototype.padEnd ( maxLength [ , fillString ] )` (22.1.3.16).
    PadEnd,
    /// `String.prototype.padStart ( maxLength [ , fillString ] )` (22.1.3.17).
    PadStart,
    /// `String.prototype.repeat ( count )` (22.1.3.18).
    Repeat,
    /// `String.prototype.toLocaleLowerCase ( [ reserved1 [ , reserved2 ] ] )` (22.1.3.26).
    ToLocaleLowerCase,
    /// `String.prototype.toLocaleUpperCase ( [ reserved1 [ , reserved2 ] ] )` (22.1.3.27).
    ToLocaleUpperCase,
    /// `String.prototype.toLowerCase ( )` (22.1.3.28).
    ToLowerCase,
    /// `String.prototype.toUpperCase ( )` (22.1.3.30).
    ToUpperCase,
    /// `String.prototype.trim ( )` (22.1.3.32).
    Trim,
    /// `String.prototype.trimEnd ( )` (22.1.3.33).
    TrimEnd,
    /// `String.prototype.trimStart ( )` (22.1.3.34).
    TrimStart,
}

/// The methods this group defines on `%String%`, in the order of their clauses: none.
pub(crate) const CONSTRUCTOR_METHODS: &[Method<TransformBuiltin>] = &[];

/// The methods this group defines on `%String.prototype%`, in the order of their clauses, each
/// with the "length" its clause heading gives (the required parameters, clause 18).
pub(crate) const PROTOTYPE_METHODS: &[Method<TransformBuiltin>] = &[
    ("localeCompare", 1, TransformBuiltin::LocaleCompare),
    ("normalize", 0, TransformBuiltin::Normalize),
    ("padEnd", 1, TransformBuiltin::PadEnd),
    ("padStart", 1, TransformBuiltin::PadStart),
    ("repeat", 1, TransformBuiltin::Repeat),
    ("toLocaleLowerCase", 0, TransformBuiltin::ToLocaleLowerCase),
    ("toLocaleUpperCase", 0, TransformBuiltin::ToLocaleUpperCase),
    ("toLowerCase", 0, TransformBuiltin::ToLowerCase),
    ("toUpperCase", 0, TransformBuiltin::ToUpperCase),
    ("trim", 0, TransformBuiltin::Trim),
    ("trimEnd", 0, TransformBuiltin::TrimEnd),
    ("trimStart", 0, TransformBuiltin::TrimStart),
];

/// The data properties keyed by a well-known symbol this group defines on
/// `%String.prototype%`, after its methods: none.
pub(crate) fn prototype_symbol_data<H: Host>() -> [SymbolData<H>; 0] {
    []
}

/// The placement of StringPad (`#sec-stringpad`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Placement {
    Start,
    End,
}

/// The `where` of TrimString (`#sec-trimstring`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Trim {
    Start,
    End,
    StartEnd,
}

/// The case a case method maps to.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Case {
    Lower,
    Upper,
}

/// A number of code units or steps as a `u64`.
fn count(units: usize) -> u64 {
    u64::try_from(units).unwrap_or(u64::MAX)
}

/// Whether a cased code point follows the code unit at `from` of `units`, after any
/// case-ignorable code points: the context `Final_Sigma` holds after a code point when none
/// does (Unicode chapter 3, table 3-17, "After C"). A case-ignorable code point is passed over
/// whether it is cased or not. Asked for each U+03A3 a lowercase meets, it stops at the first
/// code point that is not case-ignorable, U+03A3 among them, so its calls on one String read
/// each code unit at most once more: the step per code unit read pays for them.
fn cased_after(units: &[u16], from: usize) -> bool {
    let mut position = from;
    while let Some(point) = code_point_at(units, position) {
        if !is_case_ignorable(point.code_point) {
            return is_cased(point.code_point);
        }
        position += point.code_unit_count;
    }
    false
}

/// The code units of `mapping`, by UTF16EncodeCodePoint (`#sec-utf16encodecodepoint`): at most
/// six for its at most three code points. `None` for a code point above U+10FFFF, which no
/// mapping holds.
fn mapping_units(mapping: &CaseMapping) -> Option<([u16; 6], usize)> {
    let mut units = [0; 6];
    let mut length = 0;
    for &point in mapping.as_slice() {
        for &unit in utf16_encode_code_point(point)?.units() {
            *units.get_mut(length)? = unit;
            length += 1;
        }
    }
    Some((units, length))
}

impl<H: Host> Agent<H> {
    /// The steps of the builtins of this group; not inlined, so that the frame of
    /// `run_string_builtin` does not grow with its arms. Each arm calls a function of its own.
    #[inline(never)]
    pub(crate) fn run_string_transform_builtin(
        &mut self,
        id: TransformBuiltin,
        _function: ObjectRef,
        this: Value,
        args: &[Value],
        _new_target: Option<ObjectRef>,
    ) -> Completion<Value> {
        match id {
            TransformBuiltin::LocaleCompare => self.string_locale_compare(this, args),
            TransformBuiltin::Normalize => self.string_normalize(this, args),
            TransformBuiltin::PadEnd => {
                self.string_padding_method(this, args, Placement::End, "String.prototype.padEnd")
            }
            TransformBuiltin::PadStart => self.string_padding_method(
                this,
                args,
                Placement::Start,
                "String.prototype.padStart",
            ),
            TransformBuiltin::Repeat => self.string_repeat(this, args),
            TransformBuiltin::ToLocaleLowerCase => {
                self.string_case_method(this, Case::Lower, "String.prototype.toLocaleLowerCase")
            }
            TransformBuiltin::ToLocaleUpperCase => {
                self.string_case_method(this, Case::Upper, "String.prototype.toLocaleUpperCase")
            }
            TransformBuiltin::ToLowerCase => {
                self.string_case_method(this, Case::Lower, "String.prototype.toLowerCase")
            }
            TransformBuiltin::ToUpperCase => {
                self.string_case_method(this, Case::Upper, "String.prototype.toUpperCase")
            }
            TransformBuiltin::Trim => {
                self.trim_string(this, Trim::StartEnd, "String.prototype.trim")
            }
            TransformBuiltin::TrimEnd => {
                self.trim_string(this, Trim::End, "String.prototype.trimEnd")
            }
            TransformBuiltin::TrimStart => {
                self.trim_string(this, Trim::Start, "String.prototype.trimStart")
            }
        }
    }

    /// An empty builder for a result of `length` code units, asked of both limits before a unit
    /// is copied: a length above `string_length` throws a `RangeError`; a String of that
    /// length that would not fit the memory budget ends the run in `Terminated(MemoryBudget)`,
    /// as [`Agent::append_units`] would at its first unit past the budget, so the copy never
    /// starts. `length` is a non-negative integer, possibly +∞.
    fn result_builder(&mut self, length: f64) -> Completion<StringBuilder> {
        let limit = self.limits.string_length;
        // A length above the limit as an f64 is above the limit; the cast saturates and the
        // comparison in u64 is exact for the rest.
        let units = length as u64;
        if length > limit as f64 || units > limit {
            return Err(self.throw_error(
                ErrorKind::RangeError,
                "the string would be longer than string_length",
            ));
        }
        let mut builder = StringBuilder::new();
        let reserved = usize::try_from(units)
            .ok()
            .filter(|_| self.heap.accounting.check(units.saturating_mul(2)).is_ok())
            .is_some_and(|units| builder.units.try_reserve_exact(units).is_ok());
        if !reserved {
            return Err(self.terminate(Termination::MemoryBudget));
        }
        Ok(builder)
    }

    /// `String.prototype.repeat ( count )` (`#sec-string.prototype.repeat`).
    fn string_repeat(&mut self, this: Value, args: &[Value]) -> Completion<Value> {
        // 1. Let O be the this value. 2. Perform ? RequireObjectCoercible(O). 3. Let S be ?
        // ToString(O).
        let string = self.this_string(this, "String.prototype.repeat")?;
        // 4. Let n be ? ToIntegerOrInfinity(count). It may run code; S is kept.
        let keep = self.keep(Value::String(string));
        let n = self.to_integer_or_infinity(argument(args, 0));
        self.release(keep);
        let n = n?;
        // 5. If n < 0 or n = +∞, throw a RangeError exception.
        if n < 0.0 || n == f64::INFINITY {
            return Err(self.throw_error(
                ErrorKind::RangeError,
                "the count of String.prototype.repeat is negative or infinite",
            ));
        }
        // 6. If n = 0, return the empty String. n copies of the empty String are the empty
        // String, S itself.
        let length = self.string_length(string)?;
        if length == 0 {
            return Ok(Value::String(string));
        }
        // 7. Return the String value that is made from n copies of S appended together. Its
        // length, n times that of S, is asked of the limits first. For n = 0 nothing of S is
        // read.
        let mut builder = self.result_builder(n * length as f64)?;
        // n is at most the length asked, which the limits admitted.
        let copies = n as u64;
        if copies > 0 {
            // The first copy is read from S, charged before; the others from the first.
            self.append_string(&mut builder, string)?;
            for _ in 1..copies {
                self.append_again(&mut builder, length)?;
            }
        }
        Ok(Value::String(self.finish_string(builder)?))
    }

    /// Append the first `units` code units of `builder` to it again, as
    /// [`Agent::append_units`] appends units: the `RangeError` past `string_length`, a step per
    /// code unit and the memory budget before the copy.
    fn append_again(&mut self, builder: &mut StringBuilder, units: usize) -> Completion<()> {
        if units > builder.units.len() {
            return Err(self.fault());
        }
        let length = builder.units.len().saturating_add(units);
        if count(length) > self.limits.string_length {
            return Err(self.throw_error(
                ErrorKind::RangeError,
                "the string would be longer than string_length",
            ));
        }
        self.charge_steps(count(units))?;
        if self
            .heap
            .accounting
            .check(count(length).saturating_mul(2))
            .is_err()
            || builder.units.try_reserve(units).is_err()
        {
            return Err(self.terminate(Termination::MemoryBudget));
        }
        builder.units.extend_from_within(..units);
        Ok(())
    }

    /// `String.prototype.padEnd` and `padStart` (`#sec-string.prototype.padend`,
    /// `#sec-string.prototype.padstart`) for `placement`, named `method` in a TypeError.
    fn string_padding_method(
        &mut self,
        this: Value,
        args: &[Value],
        placement: Placement,
        method: &str,
    ) -> Completion<Value> {
        // 1. Let O be the this value. 2. Perform ? RequireObjectCoercible(O). 3. Return ?
        // StringPaddingBuiltinsImpl(O, maxLength, fillString, placement); its step 1, Let S be
        // ? ToString(O), is done here with step 2.
        let string = self.this_string(this, method)?;
        let keep = self.keep(Value::String(string));
        let padded = self.string_padding_builtins_impl(string, args, placement);
        self.release(keep);
        padded.map(Value::String)
    }

    /// Steps 2 to 7 of StringPaddingBuiltinsImpl (`#sec-stringpaddingbuiltinsimpl`), with S
    /// kept by the caller.
    fn string_padding_builtins_impl(
        &mut self,
        string: StringRef,
        args: &[Value],
        placement: Placement,
    ) -> Completion<StringRef> {
        // 2. Let intMaxLength be ℝ(? ToLength(maxLength)).
        let max_length = self.to_length(argument(args, 0))?;
        // 3. Let stringLength be the length of S. 4. If intMaxLength ≤ stringLength, return S.
        let length = self.string_length(string)?;
        if max_length <= length as f64 {
            return Ok(string);
        }
        // 5. If fillString is undefined, set fillString to the String value consisting solely
        // of the code unit 0x0020 (SPACE). 6. Else, set fillString to ? ToString(fillString).
        // Of fillString only the units truncatedStringFiller takes are read: at most
        // intMaxLength - stringLength (StringPad, steps 4 and 5).
        let filler = match argument(args, 1) {
            Value::Undefined => vec![0x0020],
            fill_string => {
                let fill_string = self.to_string(fill_string)?;
                // A positive integer below 2^53: the cast keeps it.
                let fill_length =
                    usize::try_from((max_length - length as f64) as u64).unwrap_or(usize::MAX);
                let used = self.string_length(fill_string)?.min(fill_length);
                self.string_units_in(fill_string, 0, used)?
            }
        };
        // 7. Return StringPad(S, intMaxLength, fillString, placement).
        self.string_pad(string, max_length, &filler, placement)
    }

    /// StringPad ( string, maxLength, fillString, placement ) (`#sec-stringpad`), with
    /// `filler` the code units of fillString it uses. The length of the result, maxLength, is
    /// asked of the limits before a unit is copied, and string is appended by its steps before
    /// its units are read.
    fn string_pad(
        &mut self,
        string: StringRef,
        max_length: f64,
        filler: &[u16],
        placement: Placement,
    ) -> Completion<StringRef> {
        // 1. Let stringLength be the length of string. 2. If maxLength ≤ stringLength, return
        // string. 3. If fillString is the empty String, return string.
        let length = self.string_length(string)?;
        if max_length <= length as f64 || filler.is_empty() {
            return Ok(string);
        }
        let mut builder = self.result_builder(max_length)?;
        // 4. Let fillLength be maxLength - stringLength. maxLength was admitted by the limits,
        // so it is an integer the cast keeps.
        let mut left = usize::try_from(max_length as u64)
            .unwrap_or(usize::MAX)
            .saturating_sub(length);
        // 6. If placement is start, return the string-concatenation of truncatedStringFiller
        // and string. 7. Return the string-concatenation of string and truncatedStringFiller.
        if placement == Placement::End {
            self.append_string(&mut builder, string)?;
        }
        // 5. Let truncatedStringFiller be the String value consisting of repeated
        // concatenations of fillString truncated to length fillLength.
        while left > 0 {
            let take = left.min(filler.len());
            self.append_units(&mut builder, filler.get(..take).unwrap_or(&[]))?;
            left -= take;
        }
        if placement == Placement::Start {
            self.append_string(&mut builder, string)?;
        }
        self.finish_string(builder)
    }

    /// `String.prototype.trim`, `trimEnd` and `trimStart` (`#sec-string.prototype.trim`,
    /// `#sec-string.prototype.trimend`, `#sec-string.prototype.trimstart`): Return ?
    /// TrimString(this value, `trim`). `method` names the method in a TypeError.
    fn trim_string(&mut self, this: Value, trim: Trim, method: &str) -> Completion<Value> {
        // TrimString (`#sec-trimstring`): 1. Perform ? RequireObjectCoercible(string). 2. Let S
        // be ? ToString(string).
        let string = self.this_string(this, method)?;
        let units = self.charged_units(string)?;
        // 3, 4, 5. A copy of S with the leading white space, the trailing white space or both
        // removed: WhiteSpace and LineTerminator, whose code points are all below 0x10000, so
        // each is one code unit, and no unit of a surrogate pair is one.
        let start = if trim == Trim::End {
            0
        } else {
            units
                .iter()
                .position(|&unit| !is_str_white_space(unit))
                .unwrap_or(units.len())
        };
        let end = if trim == Trim::Start {
            units.len()
        } else {
            units
                .iter()
                .rposition(|&unit| !is_str_white_space(unit))
                .map_or(start, |last| last + 1)
        };
        // A copy with nothing removed is S.
        if start == 0 && end == units.len() {
            return Ok(Value::String(string));
        }
        let mut builder = StringBuilder::new();
        self.append_units(&mut builder, units.get(start..end).unwrap_or(&[]))?;
        // 6. Return T.
        Ok(Value::String(self.finish_string(builder)?))
    }

    /// `String.prototype.toLowerCase` and `toUpperCase` (`#sec-string.prototype.tolowercase`,
    /// `#sec-string.prototype.touppercase`) and the locale methods, which answer the same
    /// without ECMA-402 (`#sec-string.prototype.tolocalelowercase`,
    /// `#sec-string.prototype.tolocaleuppercase`), for `case`; `method` names the method in a
    /// TypeError. A step per code unit of S before S is read.
    fn string_case_method(&mut self, this: Value, case: Case, method: &str) -> Completion<Value> {
        // 1. Let O be the this value. 2. Perform ? RequireObjectCoercible(O). 3. Let S be ?
        // ToString(O).
        let string = self.this_string(this, method)?;
        let units = self.charged_units(string)?;
        let mut builder = StringBuilder::new();
        self.append_case_mapped(&mut builder, &units, case)?;
        // 7. Return L.
        Ok(Value::String(self.finish_string(builder)?))
    }

    /// Steps 4 to 6 of `String.prototype.toLowerCase` (`#sec-string.prototype.tolowercase`),
    /// and of `toUpperCase` for [`Case::Upper`], on the code units of S into `builder`. 4. Let
    /// sText be StringToCodePoints(S) (`#sec-stringtocodepoints`): each code point read by
    /// CodePointAt, a lone surrogate as itself. 5. Let lowerText be toLowercase(sText)
    /// (toUppercase), according to the Unicode Default Case Conversion algorithm: each code
    /// point by its full mapping in `swell-unicode`, and U+03A3 in lower case by its context
    /// `Final_Sigma` (Unicode chapter 3, table 3-17): before it a cased code point after any
    /// case-ignorable ones, after it none. 6. Let L be CodePointsToString(lowerText): the units
    /// of each mapping appended as it is made, through [`Agent::append_units`] (a step per unit
    /// written, a `RangeError` past `string_length`, the memory budget before the buffer grows).
    fn append_case_mapped(
        &mut self,
        builder: &mut StringBuilder,
        units: &[u16],
        case: Case,
    ) -> Completion<()> {
        let mut position = 0;
        // Whether the last code point before `position` that is not case-ignorable is cased:
        // "Before C" of `Final_Sigma`.
        let mut cased_before = false;
        while let Some(point) = code_point_at(units, position) {
            let c = point.code_point;
            let next = position + point.code_unit_count;
            let mapping = match case {
                Case::Lower => final_sigma_lowercase(c)
                    .filter(|_| cased_before && !cased_after(units, next))
                    .unwrap_or_else(|| to_lowercase(c)),
                Case::Upper => to_uppercase(c),
            };
            let Some((mapped, length)) = mapping_units(&mapping) else {
                return Err(self.fault());
            };
            self.append_units(builder, mapped.get(..length).unwrap_or(&[]))?;
            if case == Case::Lower && !is_case_ignorable(c) {
                cased_before = is_cased(c);
            }
            position = next;
        }
        Ok(())
    }

    /// `String.prototype.localeCompare ( that [ , reserved1 [ , reserved2 ] ] )`
    /// (`#sec-string.prototype.localecompare`): -1, +0 or 1 as the NFC form of S is ordered
    /// before, equal to or after that of thatValue by code units (`docs/design/engine.md`,
    /// decision 5). The buffers of the forms are credited on every way out.
    fn string_locale_compare(&mut self, this: Value, args: &[Value]) -> Completion<Value> {
        // 1. Let O be the this value. 2. Perform ? RequireObjectCoercible(O). 3. Let S be ?
        // ToString(O).
        let string = self.this_string(this, "String.prototype.localeCompare")?;
        // 4. Let thatValue be ? ToString(that). It may run code; S is kept.
        let keep = self.keep(Value::String(string));
        let that = self.to_string(argument(args, 0));
        self.release(keep);
        let that = that?;
        let units = self.charged_units(string)?;
        let that_units = self.charged_units(that)?;
        let mut charged = 0;
        let order = self.compare_nfc_forms(&units, &that_units, &mut charged);
        self.heap.accounting.credit(charged);
        // Negative when S is ordered before thatValue, positive when after, zero otherwise: the
        // order of the code units of the forms, a String before each longer String that
        // starts with it.
        Ok(Value::Number(match order? {
            std::cmp::Ordering::Less => -1.0,
            std::cmp::Ordering::Equal => 0.0,
            std::cmp::Ordering::Greater => 1.0,
        }))
    }

    /// The order of the NFC forms of `units` and `that_units` by code units, each form made in a
    /// buffer of the method; `charged` is raised by every byte the buffers are charged. Not
    /// inlined: its buffers are not in the frame of `localeCompare` while ToString of `that`
    /// runs code.
    #[inline(never)]
    fn compare_nfc_forms(
        &mut self,
        units: &[u16],
        that_units: &[u16],
        charged: &mut u64,
    ) -> Completion<std::cmp::Ordering> {
        // The decomposition of S, then of thatValue, in one buffer.
        let mut points = Vec::new();
        let mut form = Vec::new();
        let mut that_form = Vec::new();
        self.nfc_units(units, &mut points, &mut form, charged)?;
        self.nfc_units(that_units, &mut points, &mut that_form, charged)?;
        Ok(form.cmp(&that_form))
    }

    /// The NFC form of `units` into `form`, by its full decomposition in `points`: a step per
    /// code unit written into each, the room of each charged before it grows and added to
    /// `charged`.
    fn nfc_units(
        &mut self,
        units: &[u16],
        points: &mut Vec<u32>,
        form: &mut Vec<u16>,
        charged: &mut u64,
    ) -> Completion<()> {
        self.decompose_units(Form::Nfc, units, points, charged)?;
        let mut out = UnitBuffer {
            agent: self,
            units: form,
            charged,
        };
        let done = reorder_and_compose(Form::Nfc, points, &mut out);
        self.normalized(done)
    }

    /// `String.prototype.normalize ( [ form ] )` (`#sec-string.prototype.normalize`).
    fn string_normalize(&mut self, this: Value, args: &[Value]) -> Completion<Value> {
        // 1. Let thisValue be the this value. 2. Perform ? RequireObjectCoercible(thisValue).
        // 3. Let string be ? ToString(thisValue).
        let string = self.this_string(this, "String.prototype.normalize")?;
        // 4. Set form to ? ValidateStringOption(form, coerce, « "NFC", "NFD", "NFKC", "NFKD" »,
        // "NFC"). It may run code; string is kept.
        let keep = self.keep(Value::String(string));
        let form = self.normalization_form(argument(args, 0));
        self.release(keep);
        let form = form?;
        // 5. Let normal be the String value that is the result of normalizing string into the
        // normalization form named by form as specified in the latest Unicode Standard,
        // Normalization Forms. 6. Return normal.
        let units = self.charged_units(string)?;
        let mut charged = 0;
        let normal = self.normalized_string(form, &units, &mut charged);
        self.heap.accounting.credit(charged);
        Ok(Value::String(normal?))
    }

    /// ValidateStringOption ( value, coerce, « "NFC", "NFD", "NFKC", "NFKD" », "NFC" )
    /// (`#sec-validatestringoption`), the form of `normalize`: "NFC" for undefined, else ToString
    /// of `value`, which must be one of the four names, or a `RangeError` (coerce). A String
    /// of another length than 3 or 4 is no name and is not read; one of 3 or 4 is read without
    /// a step.
    fn normalization_form(&mut self, value: Value) -> Completion<Form> {
        // 1. If value is undefined, then return defaultValue.
        if matches!(value, Value::Undefined) {
            return Ok(Form::Nfc);
        }
        // 3. Set value to ? ToString(value).
        let name = self.to_string(value)?;
        let length = self.string_length(name)?;
        let units = if length == 3 || length == 4 {
            self.string_units_in(name, 0, length)?
        } else {
            Vec::new()
        };
        // 4. If allowedValues does not contain value, throw a RangeError exception (coerce).
        // 5. Return value.
        match units.as_slice() {
            [0x4E, 0x46, 0x43] => Ok(Form::Nfc),
            [0x4E, 0x46, 0x44] => Ok(Form::Nfd),
            [0x4E, 0x46, 0x4B, 0x43] => Ok(Form::Nfkc),
            [0x4E, 0x46, 0x4B, 0x44] => Ok(Form::Nfkd),
            _ => Err(self.throw_error(
                ErrorKind::RangeError,
                "the form of String.prototype.normalize is not \"NFC\", \"NFD\", \"NFKC\" or \"NFKD\"",
            )),
        }
    }

    /// `units` in `form` as a String of the heap: the decomposition in a buffer of the method,
    /// then the form written through [`StringBuilder`], a step per code unit and bounded by
    /// `string_length`; `charged` is raised by every byte the buffer is charged. Not inlined, as
    /// [`Agent::compare_nfc_forms`].
    #[inline(never)]
    fn normalized_string(
        &mut self,
        form: Form,
        units: &[u16],
        charged: &mut u64,
    ) -> Completion<StringRef> {
        let mut points = Vec::new();
        self.decompose_units(form, units, &mut points, charged)?;
        let mut builder = StringBuilder::new();
        let mut out = BuilderBuffer {
            agent: self,
            builder: &mut builder,
        };
        let done = reorder_and_compose(form, &points, &mut out);
        self.normalized(done)?;
        self.finish_string(builder)
    }

    /// The full decomposition of the code points of `units` (StringToCodePoints,
    /// `#sec-stringtocodepoints`: a lone surrogate as itself) for `form`, into `points`, emptied
    /// first: a step per code unit written and the room charged before it grows.
    fn decompose_units(
        &mut self,
        form: Form,
        units: &[u16],
        points: &mut Vec<u32>,
        charged: &mut u64,
    ) -> Completion<()> {
        points.clear();
        let mut position = 0;
        let input = std::iter::from_fn(|| {
            let point = code_point_at(units, position)?;
            position += point.code_unit_count;
            Some(point.code_point)
        });
        let mut out = PointBuffer {
            agent: self,
            points,
            charged,
        };
        let done = decompose(form, input, &mut out);
        self.normalized(done)
    }

    /// The completion of a pass of the normalization: a stop of a buffer is its abrupt
    /// completion; a decomposition deeper than the tables say is a broken invariant.
    fn normalized(&mut self, done: Result<(), Stop<Abrupt>>) -> Completion<()> {
        match done {
            Ok(()) => Ok(()),
            Err(Stop::Buffer(abrupt)) => Err(abrupt),
            Err(Stop::TooDeep) => Err(self.fault()),
        }
    }

    /// Room for one more item in `list`, a buffer of the normalization: when it is full, its
    /// room doubles, at least to [`MIN_ROOM`], the bytes charged to the memory budget before it
    /// is reserved and added to `charged`; over the budget the run ends in
    /// `Terminated(MemoryBudget)`.
    fn room_for_one<T>(&mut self, list: &mut Vec<T>, charged: &mut u64) -> Completion<()> {
        let capacity = list.capacity();
        if list.len() < capacity {
            return Ok(());
        }
        let wanted = capacity.saturating_mul(2).max(MIN_ROOM);
        let bytes = count(wanted - capacity).saturating_mul(count(size_of::<T>()));
        self.charge_bytes(bytes)?;
        *charged = charged.saturating_add(bytes);
        let length = list.len();
        if list.try_reserve_exact(wanted - length).is_err() {
            return Err(self.terminate(Termination::MemoryBudget));
        }
        Ok(())
    }
}

/// The least room a buffer of the normalization grows to, in items.
const MIN_ROOM: usize = 16;

/// The buffer of the full decomposition: code points, a step per code unit each stands for,
/// charged before it is written, and its room charged before it grows.
struct PointBuffer<'a, H: Host> {
    agent: &'a mut Agent<H>,
    points: &'a mut Vec<u32>,
    charged: &'a mut u64,
}

impl<H: Host> Buffer for PointBuffer<'_, H> {
    type Error = Abrupt;

    fn push(&mut self, point: u32) -> Completion<()> {
        let Some(units) = utf16_encode_code_point(point) else {
            return Err(self.agent.fault());
        };
        self.agent.charge_steps(count(units.units().len()))?;
        self.agent.room_for_one(self.points, self.charged)?;
        self.points.push(point);
        Ok(())
    }
}

/// The buffer of the NFC form of `localeCompare`: code units, a step per code unit charged
/// before it is written, and the room charged before it grows. Not a String: `string_length`
/// does not bound it.
struct UnitBuffer<'a, H: Host> {
    agent: &'a mut Agent<H>,
    units: &'a mut Vec<u16>,
    charged: &'a mut u64,
}

impl<H: Host> Buffer for UnitBuffer<'_, H> {
    type Error = Abrupt;

    fn push(&mut self, point: u32) -> Completion<()> {
        let Some(encoded) = utf16_encode_code_point(point) else {
            return Err(self.agent.fault());
        };
        self.agent.charge_steps(count(encoded.units().len()))?;
        for &unit in encoded.units() {
            self.agent.room_for_one(self.units, self.charged)?;
            self.units.push(unit);
        }
        Ok(())
    }
}

impl<H: Host> FormBuffer for UnitBuffer<'_, H> {
    fn position(&self) -> usize {
        self.units.len()
    }

    fn replace(&mut self, position: usize, old: u32, new: u32) -> Completion<()> {
        replace_units(self.agent, self.units, position, old, new)
    }
}

/// The buffer of the result of `normalize`: the builder of a String, through
/// [`Agent::append_units`] (a step per code unit, a `RangeError` past `string_length`, the memory
/// budget before it grows).
struct BuilderBuffer<'a, H: Host> {
    agent: &'a mut Agent<H>,
    builder: &'a mut StringBuilder,
}

impl<H: Host> Buffer for BuilderBuffer<'_, H> {
    type Error = Abrupt;

    fn push(&mut self, point: u32) -> Completion<()> {
        let Some(encoded) = utf16_encode_code_point(point) else {
            return Err(self.agent.fault());
        };
        self.agent.append_units(self.builder, encoded.units())
    }
}

impl<H: Host> FormBuffer for BuilderBuffer<'_, H> {
    fn position(&self) -> usize {
        self.builder.units.len()
    }

    fn replace(&mut self, position: usize, old: u32, new: u32) -> Completion<()> {
        replace_units(self.agent, &mut self.builder.units, position, old, new)
    }
}

/// Replace the code units of `old` at `position` of `units` by those of `new`, a composite of
/// as many code units (the tables hold to that); anything else is a broken invariant.
fn replace_units<H: Host>(
    agent: &mut Agent<H>,
    units: &mut [u16],
    position: usize,
    old: u32,
    new: u32,
) -> Completion<()> {
    let (Some(old), Some(new)) = (utf16_encode_code_point(old), utf16_encode_code_point(new))
    else {
        return Err(agent.fault());
    };
    let end = position.saturating_add(old.units().len());
    match units.get_mut(position..end) {
        Some(slot) if slot.len() == new.units().len() && slot == old.units() => {
            slot.copy_from_slice(new.units());
            Ok(())
        }
        _ => Err(agent.fault()),
    }
}

/// The tests of the copies the builtins of this group make, and the agent the tests of the
/// other builtins of `story:string-builtins-charge-before-they-copy` share: a script runs on
/// Strings of [`tests::LONG`] code units the host made with `new_string_utf16`, and the test
/// reads the longest copy of a String's code units its entry made (`object/string.rs`,
/// `copies`).
#[cfg(test)]
pub(crate) mod tests {
    use crate::host::agent::Agent;
    use crate::host::hooks::test_host::{NoHooks, agent_with_realm_and, limits};
    use crate::object::string::copies;
    use crate::{Outcome, Parsed, PropertyKey, Realm, Root, Source, Termination, Value};

    /// The code units of the long Strings.
    pub(crate) const LONG: usize = 1_000_000;

    /// What an entry answered: the text of a String or the debug form of another value, the
    /// `name` of a thrown object, or a termination.
    #[derive(Debug, PartialEq)]
    pub(crate) enum Answer {
        Normal(String),
        Throw(String),
        Terminated(Termination),
    }

    /// An agent and its realm, with globals the host set.
    pub(crate) struct Counting {
        agent: Agent<NoHooks>,
        realm: Realm,
    }

    impl Counting {
        /// An agent with `string_length`, steps for the entries that set up, and 256 MiB of
        /// memory: a realm and the long Strings (about 1 MB each, a copy 2 MB) take a few MB of
        /// it, and no test of this file reaches the memory budget.
        pub(crate) fn new(string_length: u64) -> Counting {
            let mut limits = limits();
            limits.memory_bytes = 1 << 28;
            limits.string_length = string_length;
            limits.steps_per_entry = 1 << 26;
            let (agent, realm) = agent_with_realm_and(limits);
            Counting { agent, realm }
        }

        /// The global `name`, a String of `units` the host makes with `new_string_utf16`.
        pub(crate) fn string(&mut self, name: &str, units: &[u16]) {
            let global = self.agent.global_object(&self.realm).unwrap();
            self.agent
                .with_cx(&self.realm, |cx| {
                    let string = cx.new_string_utf16(units).unwrap();
                    let Value::Object(global) = cx.get(&global).unwrap() else {
                        panic!("the global object is no object");
                    };
                    let key = PropertyKey::String(cx.new_string(name).unwrap());
                    let set = cx.set(global, key, Value::String(string), true).unwrap();
                    assert_eq!(set, Ok(true), "{name}");
                })
                .unwrap();
        }

        /// The answer of `source` with the steps of the agent's limits.
        pub(crate) fn eval(&mut self, source: &str) -> Answer {
            let script = self.parse(source);
            let outcome = self.agent.evaluate_script(&script).unwrap();
            self.describe(outcome)
        }

        /// The answer of `source` with `steps` for its entry, and the longest copy of a
        /// String's code units the entry made.
        pub(crate) fn counted(&mut self, steps: u64, source: &str) -> (Answer, usize) {
            let script = self.parse(source);
            let saved = self.agent.limits.steps_per_entry;
            self.agent.limits.steps_per_entry = steps;
            copies::reset();
            let outcome = self.agent.evaluate_script(&script).unwrap();
            let longest = copies::longest();
            self.agent.limits.steps_per_entry = saved;
            (self.describe(outcome), longest)
        }

        /// `source`, parsed.
        fn parse(&mut self, source: &str) -> crate::Script {
            match self
                .agent
                .parse_script(&self.realm, Source::Utf8(source), None)
            {
                Ok(Parsed::Parsed(script)) => script,
                other => panic!("{source} did not parse: {other:?}"),
            }
        }

        /// The answer of `outcome`.
        fn describe(&mut self, outcome: Outcome<Root>) -> Answer {
            let (root, thrown) = match outcome {
                Outcome::Normal(root) => (root, false),
                Outcome::Throw(root) => (root, true),
                Outcome::Terminated(termination) => return Answer::Terminated(termination),
            };
            self.agent
                .with_cx(&self.realm, |cx| {
                    let mut value = cx.get(&root).unwrap();
                    if let (true, Value::Object(error)) = (thrown, value) {
                        let key = PropertyKey::String(cx.new_string("name").unwrap());
                        value = cx.get_property(error, key).unwrap().unwrap();
                    }
                    let text = match value {
                        Value::String(string) => cx.to_rust_string_lossy(string).unwrap(),
                        other => format!("{other:?}"),
                    };
                    if thrown {
                        Answer::Throw(text)
                    } else {
                        Answer::Normal(text)
                    }
                })
                .unwrap()
        }
    }

    /// An agent with `string_length` 2^21 and the global `s`, [`LONG`] code units of `unit`.
    pub(crate) fn with_long(unit: u16) -> Counting {
        let mut counting = Counting::new(1 << 21);
        counting.string("s", &vec![unit; LONG]);
        counting
    }

    /// The steps a test leaves the builtin: far fewer than the units of the long String.
    pub(crate) const FEW_STEPS: u64 = 1_000;

    /// Each of `sources`, in an agent of its own with the long String `s` of `unit`, with
    /// [`FEW_STEPS`] for its entry, ends in `Terminated(StepBudget)` before a unit of `s` is
    /// copied.
    fn charged_before_the_copy(unit: u16, sources: &[&str]) {
        for source in sources {
            let mut counting = with_long(unit);
            assert_eq!(
                counting.counted(FEW_STEPS, source),
                (Answer::Terminated(Termination::StepBudget), 0),
                "{source}"
            );
        }
    }

    /// `#sec-string.prototype.repeat`, step 6: a count of 0 answers the empty String, and
    /// nothing of S is copied for it.
    #[test]
    fn repeat_zero_copies_nothing() {
        let mut counting = with_long(0x61);
        assert_eq!(
            counting.counted(FEW_STEPS, "s.repeat(0)"),
            (Answer::Normal(String::new()), 0)
        );
    }

    /// `#sec-string.prototype.repeat`, step 7: the steps of S's units are charged before they
    /// are copied, so a budget that cannot pay them ends the entry with no copy.
    #[test]
    fn repeat_is_charged_before_its_copy() {
        charged_before_the_copy(0x61, &["s.repeat(1)"]);
    }

    /// StringPaddingBuiltinsImpl (`#sec-stringpaddingbuiltinsimpl`) and StringPad
    /// (`#sec-stringpad`): of the fill String only the units truncatedStringFiller takes are
    /// read, here 2 of [`LONG`].
    #[test]
    fn pad_copies_only_the_fill_it_uses() {
        let mut counting = with_long(0x66);
        let (answer, longest) = counting.counted(FEW_STEPS, "\"x\".padStart(3, s)");
        assert_eq!(answer, Answer::Normal("ffx".to_string()));
        assert!(longest <= 2, "{longest}");
    }

    /// StringPad (`#sec-stringpad`): S is appended by its steps before its units are copied.
    #[test]
    fn pad_is_charged_before_it_copies_s() {
        charged_before_the_copy(0x61, &["s.padEnd(1000001)"]);
    }

    /// TrimString (`#sec-trimstring`): a step per code unit of S before S is read.
    #[test]
    fn trim_methods_are_charged_before_their_copy() {
        charged_before_the_copy(0x61, &["s.trim()", "s.trimStart()", "s.trimEnd()"]);
    }

    /// The four case methods (`#sec-string.prototype.tolowercase` and the others): a step per
    /// code unit of S before S is read.
    #[test]
    fn case_methods_are_charged_before_their_copy() {
        charged_before_the_copy(
            0x61,
            &[
                "s.toLowerCase()",
                "s.toUpperCase()",
                "s.toLocaleLowerCase()",
                "s.toLocaleUpperCase()",
            ],
        );
    }

    /// `#sec-string.prototype.localecompare`: a step per code unit of S before S is read.
    #[test]
    fn locale_compare_is_charged_before_its_copies() {
        charged_before_the_copy(0x61, &["s.localeCompare(\"a\")"]);
    }
}
