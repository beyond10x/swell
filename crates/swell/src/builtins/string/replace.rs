//! The replace and split methods of `String.prototype` for a String pattern (ECMA-262 22.1.3,
//! `#sec-properties-of-the-string-prototype-object`), the group of
//! `story:string-replace-replace-all-and-split`: `replace` (`#sec-string.prototype.replace`)
//! with GetSubstitution (`#sec-getsubstitution`), `replaceAll`
//! (`#sec-string.prototype.replaceall`) and `split` (`#sec-string.prototype.split`). The group
//! defines nothing on `%String%`.
//!
//! An Object given as the pattern (`searchValue`, `separator`) is handed to its method keyed
//! by `%Symbol.replace%` or `%Symbol.split%` when it has one, as the first steps of the clauses
//! say; a primitive is never looked up, so a method on its prototype is not read.
//! `%RegExp.prototype%` has both methods (`builtins/regexp/symbol.rs`,
//! `story:regexp-symbol-methods-and-string-match-search`), and a method of a script's own object
//! answers the same way. `replaceAll` checks the `flags`
//! of an Object that IsRegExp (`#sec-isregexp`) takes for a RegExp before it looks up the
//! method.
//!
//! The budgets (`docs/design/engine.md`, decisions 3 and 5): the code units of the String, of
//! the pattern and of a replacement template are read with [`Agent::read_units`], a step per
//! code unit before the read; the search is StringIndexOf ([`Agent::string_index_of`]), a step
//! per code unit compared at each start it tries; every String a method makes is built by
//! [`StringBuilder`], so it is bounded by `string_length` (a `RangeError`) and the memory
//! budget before it grows, and its copy is charged a step per code unit. The list of the match
//! positions of `replaceAll` is charged to the memory budget before it grows and credited when
//! the method ends, a step per position. The Array of `split` grows through the dense elements
//! of the engine, charged before each growth, and each element is charged a step. A functional
//! replacer and a method keyed by a well-known symbol are calls of the agent's call path, so
//! each is a re-entry. Every String a method holds across code that may run (a conversion, a
//! call) is on the kept stack.

use crate::builtins::object::argument;
use crate::builtins::string::StringBuilder;
use crate::builtins::{Method, SymbolData};
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::host::{ErrorKind, Host, Termination};
use crate::object::property::Key;
use crate::value::{ObjectRef, StringRef, Value};
use alloc::format;
use alloc::vec::Vec;

/// The builtins of this group, by their steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ReplaceBuiltin {
    /// `String.prototype.replace ( searchValue, replaceValue )` (22.1.3.19).
    Replace,
    /// `String.prototype.replaceAll ( searchValue, replaceValue )` (22.1.3.20).
    ReplaceAll,
    /// `String.prototype.split ( separator, substringLimit )` (22.1.3.23).
    Split,
}

/// The methods this group defines on `%String%`, in the order of their clauses: none.
pub(crate) const CONSTRUCTOR_METHODS: &[Method<ReplaceBuiltin>] = &[];

/// The methods this group defines on `%String.prototype%`, in the order of their clauses, each
/// with the "length" its clause heading gives (the required parameters, clause 18).
pub(crate) const PROTOTYPE_METHODS: &[Method<ReplaceBuiltin>] = &[
    ("replace", 2, ReplaceBuiltin::Replace),
    ("replaceAll", 2, ReplaceBuiltin::ReplaceAll),
    ("split", 2, ReplaceBuiltin::Split),
];

/// The data properties keyed by a well-known symbol this group defines on
/// `%String.prototype%`, after its methods: none.
pub(crate) fn prototype_symbol_data<H: Host>() -> [SymbolData<H>; 0] {
    []
}

/// The code unit 0x0024 (DOLLAR SIGN), which starts a replacement pattern of GetSubstitution.
const DOLLAR: u16 = 0x0024;

/// The code unit 0x003E (GREATER-THAN SIGN), which ends a `$<name>` pattern.
const GREATER_THAN: u16 = 0x003E;

/// The code unit 0x0067 (LATIN SMALL LETTER G), the flag `replaceAll` asks of a RegExp.
const FLAG_G: u16 = 0x0067;

/// The value of a decimal digit of GetSubstitution, a code unit from 0x0030 (DIGIT ZERO) to
/// 0x0039 (DIGIT NINE).
fn decimal_digit(unit: Option<&u16>) -> Option<usize> {
    let unit = *unit?;
    (0x0030..=0x0039)
        .contains(&unit)
        .then(|| usize::from(unit - 0x0030))
}

/// The well-known symbol whose method an Object pattern is handed to.
#[derive(Clone, Copy)]
enum Hook {
    /// `%Symbol.replace%`, of `replace` and `replaceAll`.
    Replace,
    /// `%Symbol.split%`, of `split`.
    Split,
}

/// The replacement of `replace` and `replaceAll` once it is read: a function (functionalReplace
/// is true) or the String template, as its code units.
enum Replacement {
    Function(Value),
    Template(crate::object::string::Units),
}

/// The arguments of GetSubstitution (`#sec-getsubstitution`) but the template: the matched
/// String, the String, the position of the match, the captures (each a String or undefined)
/// and namedCaptures (an Object or undefined).
pub(crate) struct Substitution<'a> {
    pub(crate) matched: StringRef,
    pub(crate) string: StringRef,
    pub(crate) position: usize,
    pub(crate) captures: &'a [Option<StringRef>],
    pub(crate) named_captures: Option<ObjectRef>,
}

/// The positions of the matches of `replaceAll` (matchPositions): a list whose room is charged
/// to the memory budget before it is reserved; the method credits [`Positions::charged`] when
/// it ends, whatever its completion.
#[derive(Default)]
struct Positions {
    list: Vec<usize>,
    charged: u64,
}

/// A number of code units, steps or bytes as a `u64`.
fn count(units: usize) -> u64 {
    u64::try_from(units).unwrap_or(u64::MAX)
}

impl<H: Host> Agent<H> {
    /// The steps of the builtins of this group; not inlined, so that the frame of
    /// `run_string_builtin` does not grow with its arms. Each builtin is a function of its own,
    /// not inlined either, so that a re-entry through one of them pays its frame alone.
    #[inline(never)]
    pub(crate) fn run_string_replace_builtin(
        &mut self,
        id: ReplaceBuiltin,
        _function: ObjectRef,
        this: Value,
        args: &[Value],
        _new_target: Option<ObjectRef>,
    ) -> Completion<Value> {
        match id {
            ReplaceBuiltin::Replace => self.string_replace(this, args),
            ReplaceBuiltin::ReplaceAll => self.string_replace_all(this, args),
            ReplaceBuiltin::Split => self.string_split(this, args),
        }
    }

    /// "Perform ? RequireObjectCoercible(thisValue)" (`#sec-requireobjectcoercible`), the second
    /// step of each method of this group, for the method `method`.
    fn require_coercible_this(&mut self, this: Value, method: &str) -> Completion<()> {
        if matches!(this, Value::Undefined | Value::Null) {
            return Err(self.throw_error(
                ErrorKind::TypeError,
                &format!("{method} called on undefined or null"),
            ));
        }
        Ok(())
    }

    /// The step of each clause of this group that hands an Object pattern to its method: "If
    /// `value` is an Object, then: Let method be ? GetMethod(value, `hook`). If method is not
    /// undefined, then return ? Call(method, value, « thisValue, `second` »)". `None` when the
    /// clause goes on. The well-known symbols live while a realm does; without them no object
    /// has the key.
    fn hand_to_symbol_method(
        &mut self,
        value: Value,
        hook: Hook,
        this: Value,
        second: Value,
    ) -> Completion<Option<Value>> {
        if !matches!(value, Value::Object(_)) {
            return Ok(None);
        }
        let Some(symbols) = self.well_known() else {
            return Ok(None);
        };
        let symbol = match hook {
            Hook::Replace => symbols.replace,
            Hook::Split => symbols.split,
        };
        match self.get_method(value, Key::Symbol(symbol))? {
            Some(method) => self.call_value(method, value, &[this, second]).map(Some),
            None => Ok(None),
        }
    }

    /// `String.prototype.replace ( searchValue, replaceValue )`
    /// (`#sec-string.prototype.replace`).
    #[inline(never)]
    fn string_replace(&mut self, this: Value, args: &[Value]) -> Completion<Value> {
        let (search_value, replace_value) = (argument(args, 0), argument(args, 1));
        // 1. Let thisValue be the this value. 2. Perform ? RequireObjectCoercible(thisValue).
        self.require_coercible_this(this, "String.prototype.replace")?;
        // 3. If searchValue is an Object, then: a. Let replacer be ? GetMethod(searchValue,
        // %Symbol.replace%). b. If replacer is not undefined, return ? Call(replacer,
        // searchValue, « thisValue, replaceValue »).
        if let Some(answer) =
            self.hand_to_symbol_method(search_value, Hook::Replace, this, replace_value)?
        {
            return Ok(answer);
        }
        // 4. Let string be ? ToString(thisValue).
        let string = self.to_string(this)?;
        let keep = self.keep(Value::String(string));
        let replaced = self.replace_steps(string, search_value, replace_value, false);
        self.release(keep);
        replaced.map(Value::String)
    }

    /// `String.prototype.replaceAll ( searchValue, replaceValue )`
    /// (`#sec-string.prototype.replaceall`).
    #[inline(never)]
    fn string_replace_all(&mut self, this: Value, args: &[Value]) -> Completion<Value> {
        let (search_value, replace_value) = (argument(args, 0), argument(args, 1));
        // 1. Let thisValue be the this value. 2. Perform ? RequireObjectCoercible(thisValue).
        self.require_coercible_this(this, "String.prototype.replaceAll")?;
        // 3. If searchValue is an Object, then: a. Let isRegExp be ? IsRegExp(searchValue).
        // b. If isRegExp is true, then: i. Let flags be ? Get(searchValue, "flags"). ii.
        // Perform ? RequireObjectCoercible(flags). iii. If ? ToString(flags) does not contain
        // "g", throw a TypeError exception.
        if let Value::Object(object) = search_value
            && self.is_reg_exp(search_value)?
        {
            self.require_global_flags(object)?;
        }
        // c. Let replacer be ? GetMethod(searchValue, %Symbol.replace%). d. If replacer is not
        // undefined, return ? Call(replacer, searchValue, « thisValue, replaceValue »).
        if let Some(answer) =
            self.hand_to_symbol_method(search_value, Hook::Replace, this, replace_value)?
        {
            return Ok(answer);
        }
        // 4. Let string be ? ToString(thisValue).
        let string = self.to_string(this)?;
        let keep = self.keep(Value::String(string));
        let replaced = self.replace_steps(string, search_value, replace_value, true);
        self.release(keep);
        replaced.map(Value::String)
    }

    /// Step 3.b of `replaceAll` for the Object `object` that IsRegExp takes for a RegExp: its
    /// `flags`, which must be neither undefined nor null and whose String must contain "g".
    fn require_global_flags(&mut self, object: ObjectRef) -> Completion<()> {
        // i. Let flags be ? Get(searchValue, "flags").
        let key = self.name_key("flags")?;
        let flags = self.get(object, key)?;
        // ii. Perform ? RequireObjectCoercible(flags).
        if matches!(flags, Value::Undefined | Value::Null) {
            return Err(self.throw_error(
                ErrorKind::TypeError,
                "the flags of the argument of String.prototype.replaceAll are undefined or null",
            ));
        }
        // iii. If ? ToString(flags) does not contain "g", throw a TypeError exception. ToString
        // may run code; flags is kept.
        let keep = self.keep(flags);
        let flags = self.to_string(flags);
        self.release(keep);
        let flags = flags?;
        let length = self.string_length(flags)?;
        let units = self.read_units(flags, 0, length)?;
        if !units.contains(&FLAG_G) {
            return Err(self.throw_error(
                ErrorKind::TypeError,
                "String.prototype.replaceAll must be called with a global RegExp",
            ));
        }
        Ok(())
    }

    /// The steps of `replace` (5 to 8) and `replaceAll` (5 to 8) after string, with string
    /// kept: "Let searchString be ? ToString(searchValue). Let functionalReplace be
    /// IsCallable(replaceValue). If functionalReplace is false, then set replaceValue to ?
    /// ToString(replaceValue)." Then the first match, or every match when `all`.
    fn replace_steps(
        &mut self,
        string: StringRef,
        search_value: Value,
        replace_value: Value,
        all: bool,
    ) -> Completion<StringRef> {
        let search = self.to_string(search_value)?;
        let keep = self.keep(Value::String(search));
        let replaced = self.replace_with_search(string, search, replace_value, all);
        self.release(keep);
        replaced
    }

    /// The steps of [`Agent::replace_steps`] after searchString, with string and searchString
    /// kept.
    fn replace_with_search(
        &mut self,
        string: StringRef,
        search: StringRef,
        replace_value: Value,
        all: bool,
    ) -> Completion<StringRef> {
        let replacement = if self.is_callable(replace_value)? {
            Replacement::Function(replace_value)
        } else {
            let template = self.to_string(replace_value)?;
            let length = self.string_length(template)?;
            Replacement::Template(self.read_units(template, 0, length)?)
        };
        // The units of string and of searchString, each read once, a step per code unit.
        let length = self.string_length(string)?;
        let units = self.read_units(string, 0, length)?;
        let search_length = self.string_length(search)?;
        let search_units = self.read_units(search, 0, search_length)?;
        let matched = Matched {
            string,
            search,
            units: &units,
            search_units: &search_units,
        };
        if all {
            self.replace_every_match(&matched, &replacement)
        } else {
            self.replace_first_match(&matched, &replacement)
        }
    }

    /// Steps 9 to 15 of `replace`: the first match of searchString in string replaced.
    fn replace_first_match(
        &mut self,
        matched: &Matched<'_>,
        replacement: &Replacement,
    ) -> Completion<StringRef> {
        // 9. Let searchLength be the length of searchString. 10. Let position be
        // StringIndexOf(string, searchString, 0). 11. If position is not-found, return string.
        let Some(position) = self.string_index_of(matched.units, matched.search_units, 0)? else {
            return Ok(matched.string);
        };
        let end = position + matched.search_units.len();
        // 12. Let preceding be the substring of string from 0 to position. 13. Let following be
        // the substring of string from position + searchLength.
        let preceding = matched.units.get(..position).unwrap_or_default();
        let following = matched.units.get(end..).unwrap_or_default();
        let mut result = StringBuilder::new();
        match replacement {
            // 14. If functionalReplace is true, then let replacement be ? ToString(? Call(
            // replaceValue, undefined, « searchString, 𝔽(position), string »)).
            Replacement::Function(function) => {
                let replaced = self.call_replacer(*function, matched, position)?;
                // 15. Return the string-concatenation of preceding, replacement, and
                // following.
                self.append_units(&mut result, preceding)?;
                self.append_string(&mut result, replaced)?;
            }
            // 14. Else, let replacement be ! GetSubstitution(searchString, string, position,
            // « », undefined, replaceValue).
            Replacement::Template(template) => {
                self.append_units(&mut result, preceding)?;
                let substitution = matched.substitution(position);
                self.append_substitution(&mut result, &substitution, template)?;
            }
        }
        self.append_units(&mut result, following)?;
        self.finish_string(result)
    }

    /// ToString(? Call(replaceValue, undefined, « searchString, 𝔽(position), string »)): the
    /// replacement of a functional replacer for the match at `position`. The value it returns
    /// is kept while ToString may run code.
    fn call_replacer(
        &mut self,
        function: Value,
        matched: &Matched<'_>,
        position: usize,
    ) -> Completion<StringRef> {
        // An index of a String is below 2^53, so the conversion is exact.
        let answer = self.call_value(
            function,
            Value::Undefined,
            &[
                Value::String(matched.search),
                Value::Number(position as f64),
                Value::String(matched.string),
            ],
        )?;
        let keep = self.keep(answer);
        let replaced = self.to_string(answer);
        self.release(keep);
        replaced
    }

    /// Steps 9 to 16 of `replaceAll`: every match of searchString in string replaced. The list
    /// of the positions is credited at the end, whatever the completion.
    fn replace_every_match(
        &mut self,
        matched: &Matched<'_>,
        replacement: &Replacement,
    ) -> Completion<StringRef> {
        let mut positions = Positions::default();
        let replaced = match self.match_positions(matched, &mut positions) {
            Ok(()) => self.replace_at_positions(matched, replacement, &positions.list),
            Err(abrupt) => Err(abrupt),
        };
        self.heap.accounting.credit(positions.charged);
        replaced
    }

    /// Steps 9 to 13 of `replaceAll`: "Let searchLength be the length of searchString. Let
    /// advanceBy be max(1, searchLength). Let matchPositions be a new empty List. Let position
    /// be StringIndexOf(string, searchString, 0). Repeat, while position is not not-found: a.
    /// Append position to matchPositions. b. Set position to StringIndexOf(string,
    /// searchString, position + advanceBy)." A step per position appended.
    fn match_positions(
        &mut self,
        matched: &Matched<'_>,
        positions: &mut Positions,
    ) -> Completion<()> {
        let advance_by = matched.search_units.len().max(1);
        let mut position = self.string_index_of(matched.units, matched.search_units, 0)?;
        while let Some(found) = position {
            self.charge_steps(1)?;
            self.push_position(positions, found)?;
            position = self.string_index_of(
                matched.units,
                matched.search_units,
                found.saturating_add(advance_by),
            )?;
        }
        Ok(())
    }

    /// Append `position` to `positions`: when the list has no room, its room grows to twice
    /// what it was, at least 4, the bytes charged to the memory budget before it is reserved;
    /// over the budget the run ends in `Terminated(MemoryBudget)`.
    fn push_position(&mut self, positions: &mut Positions, position: usize) -> Completion<()> {
        let capacity = positions.list.capacity();
        if positions.list.len() == capacity {
            let wanted = capacity.saturating_mul(2).max(4);
            let bytes = count(wanted - capacity).saturating_mul(count(size_of::<usize>()));
            self.charge_bytes(bytes)?;
            positions.charged = positions.charged.saturating_add(bytes);
            let length = positions.list.len();
            if positions.list.try_reserve_exact(wanted - length).is_err() {
                return Err(self.terminate(Termination::MemoryBudget));
            }
        }
        positions.list.push(position);
        Ok(())
    }

    /// Steps 14 to 16 of `replaceAll`: "Let endOfLastMatch be 0. Let result be the empty
    /// String. For each element matchPosition of matchPositions: a. Let preserved be the
    /// substring of string from endOfLastMatch to matchPosition. b. The replacement, by Call
    /// and ToString or by GetSubstitution. c. Set result to the string-concatenation of
    /// result, preserved, and replacement. d. Set endOfLastMatch to matchPosition +
    /// searchLength. If endOfLastMatch < the length of string, then set result to the
    /// string-concatenation of result and the substring of string from endOfLastMatch. Return
    /// result."
    fn replace_at_positions(
        &mut self,
        matched: &Matched<'_>,
        replacement: &Replacement,
        positions: &[usize],
    ) -> Completion<StringRef> {
        let search_length = matched.search_units.len();
        let mut end_of_last_match = 0;
        let mut result = StringBuilder::new();
        for &position in positions {
            let preserved = matched
                .units
                .get(end_of_last_match..position)
                .unwrap_or_default();
            match replacement {
                Replacement::Function(function) => {
                    let replaced = self.call_replacer(*function, matched, position)?;
                    self.append_units(&mut result, preserved)?;
                    self.append_string(&mut result, replaced)?;
                }
                Replacement::Template(template) => {
                    self.append_units(&mut result, preserved)?;
                    let substitution = matched.substitution(position);
                    self.append_substitution(&mut result, &substitution, template)?;
                }
            }
            end_of_last_match = position + search_length;
        }
        if end_of_last_match < matched.units.len() {
            let rest = matched.units.get(end_of_last_match..).unwrap_or_default();
            self.append_units(&mut result, rest)?;
        }
        self.finish_string(result)
    }

    /// GetSubstitution ( matched, string, position, captures, namedCaptures,
    /// replacementTemplate ) (`#sec-getsubstitution`): the String the template stands for at a
    /// match. Every String and Object of `substitution` and the template are kept while it
    /// runs: a Get of namedCaptures and ToString of what it answers may run code. The String
    /// paths of this file append its steps to their result ([`Agent::append_substitution`]);
    /// `RegExp.prototype[@@replace]` calls this.
    pub(crate) fn get_substitution(
        &mut self,
        substitution: &Substitution<'_>,
        template: StringRef,
    ) -> Completion<StringRef> {
        let keep = self.keep(Value::String(substitution.matched));
        self.keep(Value::String(substitution.string));
        self.keep(Value::String(template));
        for capture in substitution.captures.iter().flatten() {
            self.keep(Value::String(*capture));
        }
        if let Some(named_captures) = substitution.named_captures {
            self.keep(Value::Object(named_captures));
        }
        let made = self.get_substitution_steps(substitution, template);
        self.release(keep);
        made
    }

    /// The steps of [`Agent::get_substitution`] with its arguments kept.
    fn get_substitution_steps(
        &mut self,
        substitution: &Substitution<'_>,
        template: StringRef,
    ) -> Completion<StringRef> {
        let length = self.string_length(template)?;
        let template = self.read_units(template, 0, length)?;
        let mut result = StringBuilder::new();
        self.append_substitution(&mut result, substitution, &template)?;
        self.finish_string(result)
    }

    /// Steps 1 to 6 of GetSubstitution (`#sec-getsubstitution`), with replacementTemplate as
    /// its code units `template`, appending the result to `result`. The caller keeps the
    /// Strings and the Object of `substitution`.
    fn append_substitution(
        &mut self,
        result: &mut StringBuilder,
        substitution: &Substitution<'_>,
        template: &[u16],
    ) -> Completion<()> {
        // 1. Let stringLength be the length of string. 2. Assert: position ≤ stringLength.
        let string_length = self.string_length(substitution.string)?;
        if substitution.position > string_length {
            return Err(self.fault());
        }
        // 3. Let result be the empty String. 4. Let templateRemainder be replacementTemplate.
        let mut at = 0;
        // 5. Repeat, while templateRemainder is not the empty String,
        while at < template.len() {
            let remainder = template.get(at..).unwrap_or_default();
            // The code units up to the next "$" are each a ref of one code unit that stands
            // for itself (step 5.h): they are appended at once.
            let literal = remainder
                .iter()
                .position(|&unit| unit == DOLLAR)
                .unwrap_or(remainder.len());
            if literal > 0 {
                self.append_units(result, remainder.get(..literal).unwrap_or_default())?;
                at += literal;
                continue;
            }
            // a. NOTE: The following steps isolate ref (a prefix of templateRemainder),
            // determine refReplacement (its replacement), and then append that replacement to
            // result. The length of ref is answered and the replacement appended.
            at += self.append_reference(result, substitution, remainder, string_length)?;
        }
        // 6. Return result.
        Ok(())
    }

    /// Steps 5.b to 5.k of GetSubstitution for a `remainder` that starts with "$": the
    /// replacement of its ref appended to `result`; answers refLength.
    fn append_reference(
        &mut self,
        result: &mut StringBuilder,
        substitution: &Substitution<'_>,
        remainder: &[u16],
        string_length: usize,
    ) -> Completion<usize> {
        let next = remainder.get(1).copied();
        match next {
            // b. If templateRemainder starts with "$$", then let ref be "$$" and refReplacement
            // be "$".
            Some(DOLLAR) => {
                self.append_units(result, &[DOLLAR])?;
                Ok(2)
            }
            // c. Else if templateRemainder starts with "$`", then let ref be "$`" and
            // refReplacement be the substring of string from 0 to position.
            Some(0x0060) => {
                self.append_substring(result, substitution.string, 0, substitution.position)?;
                Ok(2)
            }
            // d. Else if templateRemainder starts with "$&", then let ref be "$&" and
            // refReplacement be matched.
            Some(0x0026) => {
                self.append_string(result, substitution.matched)?;
                Ok(2)
            }
            // e. Else if templateRemainder starts with "$'", then: let ref be "$'"; let
            // matchLength be the length of matched; let tailPosition be position +
            // matchLength; let refReplacement be the substring of string from
            // min(tailPosition, stringLength).
            Some(0x0027) => {
                let match_length = self.string_length(substitution.matched)?;
                let tail = substitution
                    .position
                    .saturating_add(match_length)
                    .min(string_length);
                self.append_substring(result, substitution.string, tail, string_length)?;
                Ok(2)
            }
            // f. Else if templateRemainder starts with "$" followed by 1 or more decimal
            // digits, then ...
            Some(_) if decimal_digit(remainder.get(1)).is_some() => {
                self.append_capture_reference(result, substitution, remainder)
            }
            // g. Else if templateRemainder starts with "$<", then ...
            Some(0x003C) => self.append_named_reference(result, substitution, remainder),
            // h. Else, let ref be the substring of templateRemainder from 0 to 1 and
            // refReplacement be ref.
            _ => {
                self.append_units(result, &[DOLLAR])?;
                Ok(1)
            }
        }
    }

    /// Step 5.f of GetSubstitution, `$n` and `$nn`, for a `remainder` that starts with "$" and
    /// a decimal digit; answers refLength.
    fn append_capture_reference(
        &mut self,
        result: &mut StringBuilder,
        substitution: &Substitution<'_>,
        remainder: &[u16],
    ) -> Completion<usize> {
        let Some(first) = decimal_digit(remainder.get(1)) else {
            return Err(self.fault());
        };
        let second = decimal_digit(remainder.get(2));
        // i. If templateRemainder starts with "$" followed by 2 or more decimal digits, let
        // digitCount be 2; else let digitCount be 1. ii. Let digits be the substring of
        // templateRemainder from 1 to 1 + digitCount. iii. Let index be ℝ(StringToNumber(
        // digits)). iv. Assert: 0 ≤ index ≤ 99. v. Let captureLength be the number of elements
        // in captures.
        let (mut digit_count, mut index) = match second {
            Some(second) => (2, first * 10 + second),
            None => (1, first),
        };
        let capture_length = substitution.captures.len();
        // vi. If index > captureLength and digitCount = 2, then: NOTE: When a two-digit
        // replacement pattern specifies an index exceeding the count of capturing groups, it is
        // treated as a one-digit replacement pattern followed by a literal digit. Set
        // digitCount to 1. Set digits to the substring of digits from 0 to 1. Set index to
        // ℝ(StringToNumber(digits)).
        if index > capture_length && digit_count == 2 {
            digit_count = 1;
            index = first;
        }
        // vii. Let ref be the substring of templateRemainder from 0 to 1 + digitCount.
        let reference_length = 1 + digit_count;
        // viii. If 1 ≤ index ≤ captureLength, then: 1. Let capture be captures[index - 1]. 2.
        // If capture is undefined, let refReplacement be the empty String. 3. Else, let
        // refReplacement be capture.
        if (1..=capture_length).contains(&index) {
            if let Some(Some(capture)) = substitution.captures.get(index - 1) {
                self.append_string(result, *capture)?;
            }
        } else {
            // ix. Else, let refReplacement be ref.
            let reference = remainder.get(..reference_length).unwrap_or_default();
            self.append_units(result, reference)?;
        }
        Ok(reference_length)
    }

    /// Step 5.g of GetSubstitution, `$<groupName>`, for a `remainder` that starts with "$<";
    /// answers refLength. The Get of namedCaptures and ToString of its answer may run code;
    /// the caller keeps what they need.
    fn append_named_reference(
        &mut self,
        result: &mut StringBuilder,
        substitution: &Substitution<'_>,
        remainder: &[u16],
    ) -> Completion<usize> {
        // i. Let gtPosition be StringIndexOf(templateRemainder, ">", 0).
        let greater_than = self.string_index_of(remainder, &[GREATER_THAN], 0)?;
        // ii. If gtPosition is not-found or namedCaptures is undefined, then: 1. Let ref be
        // "$<". 2. Let refReplacement be ref.
        let (Some(greater_than), Some(named_captures)) =
            (greater_than, substitution.named_captures)
        else {
            self.append_units(result, remainder.get(..2).unwrap_or_default())?;
            return Ok(2);
        };
        // iii. Else: 1. Let ref be the substring of templateRemainder from 0 to gtPosition +
        // 1. 2. Let groupName be the substring of templateRemainder from 2 to gtPosition. 3.
        // Assert: namedCaptures is an Object. 4. Let capture be ? Get(namedCaptures,
        // groupName).
        let group_name = remainder.get(2..greater_than).unwrap_or_default();
        let mut name = StringBuilder::new();
        self.append_units(&mut name, group_name)?;
        let name = self.finish_string(name)?;
        let key = self.key_from_string(name)?;
        let capture = self.get(named_captures, key)?;
        // 5. If capture is undefined, then let refReplacement be the empty String. 6. Else,
        // let refReplacement be ? ToString(capture).
        if !matches!(capture, Value::Undefined) {
            let keep = self.keep(capture);
            let replaced = self.to_string(capture);
            self.release(keep);
            let replaced = replaced?;
            self.append_string(result, replaced)?;
        }
        Ok(greater_than + 1)
    }

    /// Append the substring of `string` from `from` to `to` to `result`: its code units read,
    /// a step per code unit before the read, then appended.
    fn append_substring(
        &mut self,
        result: &mut StringBuilder,
        string: StringRef,
        from: usize,
        to: usize,
    ) -> Completion<()> {
        let units = self.read_units(string, from, to)?;
        self.append_units(result, &units)
    }

    /// `String.prototype.split ( separator, substringLimit )` (`#sec-string.prototype.split`).
    #[inline(never)]
    fn string_split(&mut self, this: Value, args: &[Value]) -> Completion<Value> {
        let (separator, limit) = (argument(args, 0), argument(args, 1));
        // 1. Let thisValue be the this value. 2. Perform ? RequireObjectCoercible(thisValue).
        self.require_coercible_this(this, "String.prototype.split")?;
        // 3. If separator is an Object, then: a. Let splitter be ? GetMethod(separator,
        // %Symbol.split%). b. If splitter is not undefined, return ? Call(splitter, separator,
        // « thisValue, substringLimit »).
        if let Some(answer) = self.hand_to_symbol_method(separator, Hook::Split, this, limit)? {
            return Ok(answer);
        }
        // 4. Let string be ? ToString(thisValue).
        let string = self.to_string(this)?;
        let keep = self.keep(Value::String(string));
        let parts = self.split_steps(string, separator, limit);
        self.release(keep);
        parts.map(Value::Object)
    }

    /// Steps 5 to 18 of `split`, with string kept.
    fn split_steps(
        &mut self,
        string: StringRef,
        separator: Value,
        limit: Value,
    ) -> Completion<ObjectRef> {
        // 5. If substringLimit is undefined, let substringLimitInt be 2^32 - 1; else let
        // substringLimitInt be ℝ(? ToUint32(substringLimit)).
        let limit = match limit {
            Value::Undefined => u32::MAX,
            limit => self.to_uint32(limit)?,
        };
        // 6. Let separatorString be ? ToString(separator).
        let separator_string = self.to_string(separator)?;
        // 7. If substringLimitInt = 0, then return CreateArrayFromList(« »).
        if limit == 0 {
            return self.create_array_from_list(&[]);
        }
        // 8. If separator is undefined, then return CreateArrayFromList(« string »).
        if matches!(separator, Value::Undefined) {
            return self.create_array_from_list(&[Value::String(string)]);
        }
        let keep = self.keep(Value::String(separator_string));
        let parts = self.split_by(string, separator_string, limit);
        self.release(keep);
        parts
    }

    /// Steps 9 to 18 of `split`, with string and separatorString kept: the Array is made first
    /// and kept, and each part is appended to it as it is made, as CreateArrayFromList of the
    /// list of the parts would define it (no code runs in between).
    fn split_by(
        &mut self,
        string: StringRef,
        separator: StringRef,
        limit: u32,
    ) -> Completion<ObjectRef> {
        // 9. Let separatorLength be the length of separatorString.
        let separator_length = self.string_length(separator)?;
        let string_length = self.string_length(string)?;
        // 11. If string is the empty String, return CreateArrayFromList(« string »).
        if separator_length > 0 && string_length == 0 {
            return self.create_array_from_list(&[Value::String(string)]);
        }
        let array = self.array_create(0.0, None)?;
        let keep = self.keep(Value::Object(array));
        let filled = if separator_length == 0 {
            self.split_into_code_units(array, string, string_length, limit)
        } else {
            self.split_at_separators(array, string, string_length, separator, limit)
        };
        self.release(keep);
        filled.map(|()| array)
    }

    /// Step 10 of `split`: "If separatorLength = 0, then: a. Let stringLength be the length of
    /// string. b. Let outLength be the result of clamping substringLimitInt between 0 and
    /// stringLength. c. Let head be the substring of string from 0 to outLength. d. Let
    /// codeUnits be a List consisting of the sequence of code units that are the elements of
    /// head. e. Return CreateArrayFromList(codeUnits)." Only the units of head are read.
    fn split_into_code_units(
        &mut self,
        array: ObjectRef,
        string: StringRef,
        string_length: usize,
        limit: u32,
    ) -> Completion<()> {
        let out_length = usize::try_from(limit)
            .unwrap_or(usize::MAX)
            .min(string_length);
        let head = self.read_units(string, 0, out_length)?;
        for (index, unit) in head.iter().enumerate() {
            self.push_part(array, index, core::slice::from_ref(unit))?;
        }
        Ok(())
    }

    /// Steps 12 to 18 of `split`: the substrings of string between the occurrences of
    /// separatorString, at most substringLimitInt of them.
    fn split_at_separators(
        &mut self,
        array: ObjectRef,
        string: StringRef,
        string_length: usize,
        separator: StringRef,
        limit: u32,
    ) -> Completion<()> {
        let units = self.read_units(string, 0, string_length)?;
        let separator_length = self.string_length(separator)?;
        let separator = self.read_units(separator, 0, separator_length)?;
        let limit = usize::try_from(limit).unwrap_or(usize::MAX);
        // 12. Let substrings be a new empty List. 13. Let searchStart be 0. 14. Let matchIndex
        // be StringIndexOf(string, separatorString, 0).
        let mut parts = 0;
        let mut search_start = 0;
        let mut match_index = self.string_index_of(&units, &separator, 0)?;
        // 15. Repeat, while matchIndex is not not-found,
        while let Some(index) = match_index {
            // a. Let substring be the substring of string from searchStart to matchIndex. b.
            // Append substring to substrings.
            let part = units.get(search_start..index).unwrap_or_default();
            self.push_part(array, parts, part)?;
            parts += 1;
            // c. If the number of elements in substrings is substringLimitInt, return
            // CreateArrayFromList(substrings).
            if parts == limit {
                return Ok(());
            }
            // d. Set searchStart to matchIndex + separatorLength. e. Set matchIndex to
            // StringIndexOf(string, separatorString, searchStart).
            search_start = index + separator_length;
            match_index = self.string_index_of(&units, &separator, search_start)?;
        }
        // 16. Let substring be the substring of string from searchStart. 17. Append substring
        // to substrings. 18. Return CreateArrayFromList(substrings).
        let part = units.get(search_start..).unwrap_or_default();
        self.push_part(array, parts, part)
    }

    /// The element `index` of the Array of `split`, a new String of `units`: a step for the
    /// element, the String made by a [`StringBuilder`], then stored, the dense elements charged
    /// before they grow. An index is below the limit, 2^32 - 1.
    fn push_part(&mut self, array: ObjectRef, index: usize, units: &[u16]) -> Completion<()> {
        self.charge_steps(1)?;
        let Ok(index) = u32::try_from(index) else {
            return Err(self.fault());
        };
        let mut part = StringBuilder::new();
        self.append_units(&mut part, units)?;
        let part = self.finish_string(part)?;
        if !self.create_data_property(array, Key::Index(index), Value::String(part))? {
            return Err(self.fault());
        }
        Ok(())
    }
}

/// What `replace` and `replaceAll` search and in what: string and searchString, and their code
/// units, each read once.
struct Matched<'a> {
    string: StringRef,
    search: StringRef,
    units: &'a [u16],
    search_units: &'a [u16],
}

impl Matched<'_> {
    /// The arguments of GetSubstitution for the match at `position` of a String pattern:
    /// matched is searchString, no captures, namedCaptures undefined.
    fn substitution(&self, position: usize) -> Substitution<'static> {
        Substitution {
            matched: self.search,
            string: self.string,
            position,
            captures: &[],
            named_captures: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Substitution;
    use crate::host::agent::Agent;
    use crate::host::hooks::test_host::{NoHooks, agent_with_realm, enter};
    use crate::object::ordinary::OrdinaryData;
    use crate::realm::intrinsics::Intrinsic;
    use crate::value::{StringRef, Value};
    use alloc::format;
    use alloc::string::String;
    use alloc::string::ToString;
    use alloc::vec;
    use alloc::vec::Vec;

    /// A new String of `text`, kept for the rest of the test.
    fn string(agent: &mut Agent<NoHooks>, text: &str) -> StringRef {
        let made = agent.new_string_str(text).unwrap();
        agent.keep(Value::String(made));
        made
    }

    /// The text of GetSubstitution of `template` with `substitution`.
    fn substituted(
        agent: &mut Agent<NoHooks>,
        substitution: &Substitution<'_>,
        template: &str,
    ) -> String {
        let template = string(agent, template);
        let made = agent.get_substitution(substitution, template).unwrap();
        String::from_utf16_lossy(&agent.string_units_for_tests(made).unwrap())
    }

    /// GetSubstitution (`#sec-getsubstitution`), each row of the table of its steps: `$$` a
    /// dollar sign, `` $` `` the String before the match, `$&` the match, `$'` the String after
    /// it (empty when the match passes the end of the String), `$n` and `$nn` the capture of
    /// that index (empty for an undefined capture; a two-digit index past the count of captures
    /// a one-digit reference and a literal digit; an index of 0 or past the count the reference
    /// itself), `$<name>` the ToString of the Get of the name on namedCaptures (empty for
    /// undefined), and `$<` itself without a `>` or without namedCaptures; every other code
    /// unit, a `$` before any other unit and at the end included, stands for itself.
    #[test]
    fn get_substitution_reads_every_pattern() {
        let (mut agent, realm) = agent_with_realm();
        enter(&mut agent, &realm);
        let handle = agent.heap.roots.get_realm(&realm.0).unwrap();
        let object_prototype = agent.intrinsic(handle, Intrinsic::ObjectPrototype).unwrap();
        let named = agent
            .ordinary_object_create(Some(object_prototype), OrdinaryData::Plain)
            .unwrap();
        agent.keep(Value::Object(named));
        let x = string(&mut agent, "X!");
        for (name, value) in [
            ("x", Value::String(x)),
            ("n", Value::Number(5.0)),
            ("u", Value::Undefined),
        ] {
            let key = agent.name_key(name).unwrap();
            agent
                .create_data_property_or_throw(named, key, value)
                .unwrap();
        }
        let subject = string(&mut agent, "abcdef");
        let matched = string(&mut agent, "cd");
        let c1 = string(&mut agent, "C1");
        let c3 = string(&mut agent, "C3");
        let captures = [Some(c1), None, Some(c3)];
        let three = Substitution {
            matched,
            string: subject,
            position: 2,
            captures: &captures,
            named_captures: Some(named),
        };
        for (template, expected) in [
            ("", ""),
            ("plain", "plain"),
            ("$$", "$"),
            ("$$1", "$1"),
            ("$`", "ab"),
            ("$&", "cd"),
            ("$'", "ef"),
            ("[$`|$&|$']", "[ab|cd|ef]"),
            ("$1", "C1"),
            ("$2", ""),
            ("$3", "C3"),
            ("$4", "$4"),
            ("$0", "$0"),
            ("$01", "C1"),
            ("$03", "C3"),
            ("$00", "$00"),
            ("$04", "$04"),
            ("$10", "C10"),
            ("$31", "C31"),
            ("$99", "$99"),
            ("$1$2$3", "C1C3"),
            ("$<x>", "X!"),
            ("$<n>", "5"),
            ("$<u>", ""),
            ("$<missing>", ""),
            ("$<x", "$<x"),
            ("$<", "$<"),
            ("a$<x>b$<n>", "aX!b5"),
            ("$", "$"),
            ("a$b", "a$b"),
            ("$a$", "$a$"),
        ] {
            assert_eq!(
                substituted(&mut agent, &three, template),
                expected,
                "{template}"
            );
        }
        // namedCaptures undefined: `$<` stands for itself.
        let unnamed = Substitution {
            named_captures: None,
            ..three
        };
        assert_eq!(substituted(&mut agent, &unnamed, "$<x>"), "$<x>");
        // No captures: every `$n` and `$nn` is the reference itself.
        let none = Substitution {
            captures: &[],
            ..unnamed
        };
        for template in ["$1", "$10", "$01", "$99"] {
            assert_eq!(substituted(&mut agent, &none, template), template);
        }
        // Twelve captures: `$12` is the twelfth, `$13` the first and a literal "3".
        let twelve: Vec<Option<StringRef>> = (1..=12)
            .map(|i| Some(string(&mut agent, &format!("c{i}"))))
            .collect();
        let many = Substitution {
            captures: &twelve,
            ..none
        };
        for (template, expected) in [("$12", "c12"), ("$13", "c13"), ("$011", "c11")] {
            assert_eq!(
                substituted(&mut agent, &many, template),
                expected,
                "{template}"
            );
        }
        // A match that passes the end of the String: `$'` is the empty String.
        let long = string(&mut agent, "cdefgh");
        let past = Substitution {
            matched: long,
            ..many
        };
        assert_eq!(substituted(&mut agent, &past, "[$']"), "[]");
    }

    /// `docs/design/engine.md`, "Memory budget": the copies of the String and of searchString
    /// that `replace` (`#sec-string.prototype.replace`) holds while its replacer runs are
    /// charged. Four levels, each holding a copy of 65,536 code units, raise the most charged
    /// by at least four such copies, do not fit a room of three, leave no capacity uncovered at
    /// any entry into script, and take the steps they took before.
    #[test]
    fn replace_holds_charged_copies_across_a_call_of_its_replacer() {
        use crate::builtins::string::transform::tests::Answer;
        use crate::host::Termination;
        use crate::object::string::across::Across;
        use crate::object::string::copies;
        let mut units = vec![0x61; 65_535];
        units.push(0x78);
        let source = "var d = 0; function f() { if (++d < 4) s.replace(\"x\", f); return \"y\"; } \
                      s.replace(\"x\", f).length";
        let mut across = Across::with_steps(4_000_000);
        across.string("s", &units);
        let script = across.parse(source);
        across.agent.heap.accounting.reset_high_water();
        let before = across.agent.heap.accounting.high_water();
        assert_eq!(
            across.evaluate(&script),
            Answer::Normal("Number(65536.0)".to_string())
        );
        let risen = across.agent.heap.accounting.high_water() - before;
        assert!(risen >= 4 * 131_072, "{risen}");
        assert_eq!(copies::uncovered_at_entry(), 0);
        assert_eq!(across.steps(), REPLACE_ACROSS_STEPS);

        let mut across = Across::with_steps(4_000_000);
        across.string("s", &units);
        let script = across.parse(source);
        across.room(3 * 131_072);
        assert_eq!(
            across.evaluate(&script),
            Answer::Terminated(Termination::MemoryBudget)
        );
    }

    /// The steps of the script of
    /// [`replace_holds_charged_copies_across_a_call_of_its_replacer`] on the base.
    const REPLACE_ACROSS_STEPS: u64 = 786_530;
}
