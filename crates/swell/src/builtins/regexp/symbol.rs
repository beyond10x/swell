//! The methods of `%RegExp.prototype%` keyed by well-known symbols (ECMA-262 22.2.6), written
//! by `story:regexp-symbol-methods-and-string-match-search`: `RegExp.prototype [
//! %Symbol.match% ]` (`#sec-regexp.prototype-%symbol.match%`), `[ %Symbol.replace% ]`
//! (`#sec-regexp.prototype-%symbol.replace%`), `[ %Symbol.search% ]`
//! (`#sec-regexp.prototype-%symbol.search%`) and `[ %Symbol.split% ]`
//! (`#sec-regexp.prototype-%symbol.split%`). Each matches through RegExpExec
//! (`#sec-regexpexec`, `regexp/exec.rs`), so a user `exec` answers for an object of a script,
//! and reads `flags` by Get; `@@split` constructs its splitter by SpeciesConstructor
//! (`#sec-speciesconstructor`, `ops/objects.rs`), and `@@replace` substitutes through
//! GetSubstitution (`#sec-getsubstitution`, `builtins/string/replace.rs`).
//!
//! The budgets (`docs/design/engine.md`, decisions 3 and 7): the steps of each match are those
//! of the matcher and of the operations around it; each pass of a loop of `@@match`,
//! `@@replace` and `@@split` charges a step besides, as does each capture `@@replace` and
//! `@@split` read. The Arrays `@@match` and `@@split` answer grow through the dense elements of
//! the engine, charged before each growth. The results `@@replace` collects stay on the kept
//! stack until the method ends, its room charged before it grows, so a collection while a
//! replacer or a user `exec` runs keeps them; the captures of a result and the arguments of a
//! replacer are lists charged before they grow and credited when the result is done. Every
//! String a method builds goes through [`StringBuilder`], bounded by `string_length`. A
//! functional replacer, a user `exec`, a getter of `flags` or `lastIndex` and the species
//! constructor are calls of the agent's call path, so each is a re-entry; every value a method
//! holds across one is kept.

use crate::builtins::object::argument;
use crate::builtins::string::StringBuilder;
use crate::builtins::string::replace::Substitution;
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::host::{Host, Termination};
use crate::object::builtin_function::KeptRoom;
use crate::object::property::Key;
use crate::ops::conversion::clamp_index;
use crate::realm::intrinsics::Intrinsic;
use crate::value::{ObjectRef, StringRef, Value};

/// The code unit 0x0067 (LATIN SMALL LETTER G).
const FLAG_G: u16 = 0x0067;
/// The code unit 0x0075 (LATIN SMALL LETTER U).
const FLAG_U: u16 = 0x0075;
/// The code unit 0x0076 (LATIN SMALL LETTER V).
const FLAG_V: u16 = 0x0076;
/// The code unit 0x0079 (LATIN SMALL LETTER Y).
const FLAG_Y: u16 = 0x0079;

/// What the methods read of `? ToString(? Get(regexp, "flags"))`: the String, and whether it
/// contains "g", "u" or "v", and "y".
#[derive(Clone, Copy)]
struct FlagsRead {
    string: StringRef,
    global: bool,
    full_unicode: bool,
    sticky: bool,
}

/// The replacement of `@@replace` once it is read: a function (functionalReplace is true) or
/// the String replaceValue.
#[derive(Clone, Copy)]
enum Replacement {
    Function(Value),
    Template(StringRef),
}

/// A list whose room is charged to the memory budget before it is reserved; its owner credits
/// [`Charged::bytes`] when it drops the list, whatever the completion.
struct Charged<T> {
    list: Vec<T>,
    bytes: u64,
}

impl<T> Charged<T> {
    /// An empty list, nothing charged.
    fn new() -> Charged<T> {
        Charged {
            list: Vec::new(),
            bytes: 0,
        }
    }
}

/// One result of `@@replace` read: its position clamped to the String, the length of its
/// matched String, and its replacement String.
struct Piece {
    position: usize,
    match_length: usize,
    replacement: StringRef,
}

/// A number of code units, steps or elements as a `u64`.
fn count(units: usize) -> u64 {
    u64::try_from(units).unwrap_or(u64::MAX)
}

impl<H: Host> Agent<H> {
    /// Append `item` to `list`: when the list has no room, its room grows to twice what it
    /// was, at least 4, the bytes charged to the memory budget before it is reserved; over the
    /// budget the run ends in `Terminated(MemoryBudget)`.
    fn push_charged<T>(&mut self, list: &mut Charged<T>, item: T) -> Completion<()> {
        let capacity = list.list.capacity();
        if list.list.len() == capacity {
            let wanted = capacity.saturating_mul(2).max(4);
            let bytes = count(wanted - capacity).saturating_mul(count(size_of::<T>()));
            self.charge_bytes(bytes)?;
            list.bytes = list.bytes.saturating_add(bytes);
            let length = list.list.len();
            if list.list.try_reserve_exact(wanted - length).is_err() {
                return Err(self.terminate(Termination::MemoryBudget));
            }
        }
        list.list.push(item);
        Ok(())
    }

    /// Keep `value` on the kept stack in room charged to `room` (`Agent::reserve_kept`).
    fn keep_in(&mut self, room: &mut KeptRoom, value: Value) -> Completion<()> {
        self.reserve_kept(room, 1)?;
        self.keep(value);
        Ok(())
    }

    /// `? ToString(value)` with `value` kept while the conversion may run code.
    fn string_of_kept(&mut self, value: Value) -> Completion<StringRef> {
        let mark = self.keep(value);
        let string = self.to_string(value);
        self.release(mark);
        string
    }

    /// `? ToString(? Get(object, key))`.
    fn get_string(&mut self, object: ObjectRef, key: Key) -> Completion<StringRef> {
        let value = self.get(object, key)?;
        self.string_of_kept(value)
    }

    /// `Let flags be ? ToString(? Get(regexp, "flags"))` and what the methods test of it, each
    /// code unit read with a step.
    fn regexp_flags_read(&mut self, regexp: ObjectRef) -> Completion<FlagsRead> {
        let key = self.name_key("flags")?;
        let string = self.get_string(regexp, key)?;
        let length = self.string_length(string)?;
        let units = self.read_units(string, 0, length)?;
        Ok(FlagsRead {
            string,
            global: units.contains(&FLAG_G),
            full_unicode: units.contains(&FLAG_U) || units.contains(&FLAG_V),
            sticky: units.contains(&FLAG_Y),
        })
    }

    /// `Perform ? Set(regexp, "lastIndex", value, true)`.
    fn set_last_index(&mut self, regexp: ObjectRef, value: Value) -> Completion<()> {
        let key = self.name_key("lastIndex")?;
        self.set(regexp, key, value, true)?;
        Ok(())
    }

    /// `ℝ(? ToLength(? Get(regexp, "lastIndex")))`.
    fn last_index_length(&mut self, regexp: ObjectRef) -> Completion<u64> {
        let key = self.name_key("lastIndex")?;
        let value = self.get(regexp, key)?;
        let mark = self.keep(value);
        let length = self.to_length(value);
        self.release(mark);
        // ToLength answers an integer from 0 to 2^53 - 1, which the cast keeps exactly.
        Ok(length? as u64)
    }

    /// The steps of `@@match` and `@@replace` after an empty match: "Let thisIndex be ℝ(?
    /// ToLength(? Get(regexp, "lastIndex"))). Let nextIndex be AdvanceStringIndex(string,
    /// thisIndex, fullUnicode). Perform ? Set(regexp, "lastIndex", 𝔽(nextIndex), true)."
    fn advance_last_index(
        &mut self,
        regexp: ObjectRef,
        string: StringRef,
        full_unicode: bool,
    ) -> Completion<()> {
        let this_index = self.last_index_length(regexp)?;
        let next = self.advance_string_index(string, this_index, full_unicode)?;
        self.set_last_index(regexp, Value::Number(next as f64))
    }

    /// The TypeError of `method` called on a `this` that is not an Object; else the Object.
    fn regexp_this_object(&mut self, this: Value, method: &str) -> Completion<ObjectRef> {
        match this {
            Value::Object(object) => Ok(object),
            _ => Err(self.regexp_this_not_object(method)),
        }
    }

    /// `RegExp.prototype [ %Symbol.match% ] ( string )` (`#sec-regexp.prototype-%symbol.match%`).
    #[inline(never)]
    pub(super) fn regexp_symbol_match(&mut self, this: Value, args: &[Value]) -> Completion<Value> {
        // 1. Let regexp be the this value. 2. If regexp is not an Object, throw a TypeError
        // exception.
        let regexp = self.regexp_this_object(this, "RegExp.prototype[Symbol.match]")?;
        // 3. Set string to ? ToString(string).
        let string = self.to_string(argument(args, 0))?;
        let mark = self.keep(Value::String(string));
        let answer = self.regexp_match_steps(regexp, string);
        self.release(mark);
        answer
    }

    /// Steps 4 to 6 of `@@match`, with string kept.
    fn regexp_match_steps(&mut self, regexp: ObjectRef, string: StringRef) -> Completion<Value> {
        // 4. Let flags be ? ToString(? Get(regexp, "flags")).
        let flags = self.regexp_flags_read(regexp)?;
        // 5. If flags does not contain "g", then return ? RegExpExec(regexp, string).
        if !flags.global {
            return self.regexp_exec(regexp, string);
        }
        // 6.a, b. fullUnicode; 6.c. Perform ? Set(regexp, "lastIndex", +0𝔽, true).
        self.set_last_index(regexp, Value::Number(0.0))?;
        // 6.d. Let A be ! ArrayCreate(0).
        let array = self.array_create(0.0, None)?;
        let mark = self.keep(Value::Object(array));
        let answer = self.regexp_match_every(regexp, string, array, flags.full_unicode);
        self.release(mark);
        answer
    }

    /// Steps 6.e and 6.f of `@@match`, with string and A kept: every match collected into A, a
    /// step per pass.
    fn regexp_match_every(
        &mut self,
        regexp: ObjectRef,
        string: StringRef,
        array: ObjectRef,
        full_unicode: bool,
    ) -> Completion<Value> {
        // 6.e. Let n be 0. 6.f. Repeat,
        let mut n = 0.0;
        loop {
            self.charge_steps(1)?;
            // i. Let result be ? RegExpExec(regexp, string).
            let result = self.regexp_exec(regexp, string)?;
            // ii. If result is null, then 1. If n = 0, return null. 2. Return A.
            let Value::Object(result) = result else {
                return Ok(if n == 0.0 {
                    Value::Null
                } else {
                    Value::Object(array)
                });
            };
            let mark = self.keep(Value::Object(result));
            let stored = self.regexp_match_store(regexp, string, array, result, n, full_unicode);
            self.release(mark);
            stored?;
            // 6.f.iii.5. Set n to n + 1.
            n += 1.0;
        }
    }

    /// Steps 6.f.iii.1 to 4 of `@@match`, with result kept: its String stored at n of A, and
    /// `lastIndex` advanced past an empty match.
    fn regexp_match_store(
        &mut self,
        regexp: ObjectRef,
        string: StringRef,
        array: ObjectRef,
        result: ObjectRef,
        n: f64,
        full_unicode: bool,
    ) -> Completion<()> {
        // 1. Let matchStr be ? ToString(? Get(result, "0")).
        let matched = self.get_string(result, Key::Index(0))?;
        // 2. Perform ! CreateDataPropertyOrThrow(A, ! ToString(𝔽(n)), matchStr). No code runs
        // on the new Array.
        let key = self.index_key(n)?;
        self.create_data_property_or_throw(array, key, Value::String(matched))?;
        // 3. If matchStr is the empty String, then advance lastIndex.
        if self.string_length(matched)? == 0 {
            self.advance_last_index(regexp, string, full_unicode)?;
        }
        Ok(())
    }

    /// `RegExp.prototype [ %Symbol.replace% ] ( string, replaceValue )`
    /// (`#sec-regexp.prototype-%symbol.replace%`).
    #[inline(never)]
    pub(super) fn regexp_symbol_replace(
        &mut self,
        this: Value,
        args: &[Value],
    ) -> Completion<Value> {
        // 1. Let rx be the this value. 2. If rx is not an Object, throw a TypeError exception.
        let regexp = self.regexp_this_object(this, "RegExp.prototype[Symbol.replace]")?;
        // 3. Let S be ? ToString(string).
        let string = self.to_string(argument(args, 0))?;
        let mark = self.keep(Value::String(string));
        let replaced = self.regexp_replace_steps(regexp, string, argument(args, 1));
        self.release(mark);
        replaced.map(Value::String)
    }

    /// Steps 4 to 15 of `@@replace`, with string kept: the replacement read, the results
    /// collected on the kept stack, then replaced; the room of the kept stack is given back
    /// whatever the completion.
    fn regexp_replace_steps(
        &mut self,
        regexp: ObjectRef,
        string: StringRef,
        replace_value: Value,
    ) -> Completion<StringRef> {
        // 5. Let functionalReplace be IsCallable(replaceValue). 6. If functionalReplace is
        // false, then a. Set replaceValue to ? ToString(replaceValue).
        let replacement = if self.is_callable(replace_value)? {
            Replacement::Function(replace_value)
        } else {
            Replacement::Template(self.to_string(replace_value)?)
        };
        let mut room = self.kept_room();
        let mark = self.kept.len();
        if let Replacement::Template(template) = replacement {
            self.keep(Value::String(template));
        }
        let replaced = self.regexp_replace_kept(regexp, string, replacement, &mut room);
        self.end_kept_room(mark, &room);
        replaced
    }

    /// Steps 7 to 15 of `@@replace`, with string and the replacement kept.
    fn regexp_replace_kept(
        &mut self,
        regexp: ObjectRef,
        string: StringRef,
        replacement: Replacement,
        room: &mut KeptRoom,
    ) -> Completion<StringRef> {
        // 7. Let flags be ? ToString(? Get(rx, "flags")). 8. Let global be whether flags
        // contains "g". 9. If global is true, then a. Perform ? Set(rx, "lastIndex", +0𝔽,
        // true).
        let flags = self.regexp_flags_read(regexp)?;
        if flags.global {
            self.set_last_index(regexp, Value::Number(0.0))?;
        }
        // 10. Let results be a new empty List: the results are kept from here on.
        let first = self.kept.len();
        let found = self.regexp_replace_collect(regexp, string, flags, room)?;
        self.regexp_replace_results(string, first, found, replacement, room)
    }

    /// Steps 11 and 12 of `@@replace`: "Let done be false. Repeat, while done is false," each
    /// result of RegExpExec kept in `room`, a step per pass; answers the number of results.
    fn regexp_replace_collect(
        &mut self,
        regexp: ObjectRef,
        string: StringRef,
        flags: FlagsRead,
        room: &mut KeptRoom,
    ) -> Completion<usize> {
        let mut found = 0;
        loop {
            self.charge_steps(1)?;
            // a. Let result be ? RegExpExec(rx, S). b. If result is null, set done to true.
            let Value::Object(result) = self.regexp_exec(regexp, string)? else {
                return Ok(found);
            };
            // c. Else, i. Append result to results.
            self.keep_in(room, Value::Object(result))?;
            found += 1;
            // ii. If global is false, set done to true.
            if !flags.global {
                return Ok(found);
            }
            // iii. Else, 1. Let matchStr be ? ToString(? Get(result, "0")). 2. If matchStr is
            // the empty String, then advance lastIndex by AdvanceStringIndex.
            let matched = self.get_string(result, Key::Index(0))?;
            if self.string_length(matched)? == 0 {
                self.advance_last_index(regexp, string, flags.full_unicode)?;
            }
        }
    }

    /// Steps 13 to 15 of `@@replace`: each of the `found` results kept from `first` on read and
    /// replaced in order, a step per pass, the pieces joined by a [`StringBuilder`]; a result
    /// whose position lies before nextSourcePosition is skipped.
    fn regexp_replace_results(
        &mut self,
        string: StringRef,
        first: usize,
        found: usize,
        replacement: Replacement,
        room: &mut KeptRoom,
    ) -> Completion<StringRef> {
        let length = self.string_length(string)?;
        // 13. Let accumulatedResult be the empty String. 14. Let nextSourcePosition be 0.
        let mut accumulated = StringBuilder::new();
        let mut next_source_position = 0;
        // 15. For each element result of results, do
        for index in first..first.saturating_add(found) {
            self.charge_steps(1)?;
            let Some(&Value::Object(result)) = self.kept.get(index) else {
                return Err(self.fault());
            };
            let mark = self.kept.len();
            let piece = self.regexp_replace_piece(result, string, length, replacement, room);
            let appended = match piece {
                // p. If position ≥ nextSourcePosition, then i. NOTE. ii. Set accumulatedResult
                // to the string-concatenation of accumulatedResult, the substring of S from
                // nextSourcePosition to position, and replacementString. iii. Set
                // nextSourcePosition to position + matchLength.
                Ok(piece) if piece.position >= next_source_position => {
                    let appended = self
                        .read_units(string, next_source_position, piece.position)
                        .and_then(|units| self.append_units(&mut accumulated, &units))
                        .and_then(|()| self.append_string(&mut accumulated, piece.replacement));
                    next_source_position = piece.position.saturating_add(piece.match_length);
                    appended
                }
                Ok(_) => Ok(()),
                Err(abrupt) => Err(abrupt),
            };
            self.release(mark);
            appended?;
        }
        // 16. If nextSourcePosition ≥ lengthS, return accumulatedResult. 17. Return the
        // string-concatenation of accumulatedResult and the substring of S from
        // nextSourcePosition.
        if next_source_position < length {
            let rest = self.read_units(string, next_source_position, length)?;
            self.append_units(&mut accumulated, &rest)?;
        }
        self.finish_string(accumulated)
    }

    /// Steps 15.a to 15.o of `@@replace` for one result, kept: its length, matched, position,
    /// captures and groups read in the order of the steps, and the replacement String made by
    /// a call of the replacer or by GetSubstitution. What it makes stays kept in `room` until
    /// the caller releases it; the list of the captures is credited here.
    fn regexp_replace_piece(
        &mut self,
        result: ObjectRef,
        string: StringRef,
        length: usize,
        replacement: Replacement,
        room: &mut KeptRoom,
    ) -> Completion<Piece> {
        // a. Let resultLength be ? LengthOfArrayLike(result). b. Let nCaptures be
        // max(resultLength - 1, 0).
        let result_length = self.length_of_array_like(result)?;
        let captures_count = (result_length - 1.0).max(0.0);
        // c. Let matched be ? ToString(? Get(result, "0")). d. Let matchLength be the length of
        // matched.
        let matched = self.get_string(result, Key::Index(0))?;
        self.keep_in(room, Value::String(matched))?;
        let match_length = self.string_length(matched)?;
        // e. Let position be ? ToIntegerOrInfinity(? Get(result, "index")). f. Set position to
        // the result of clamping position between 0 and lengthS.
        let key = self.name_key("index")?;
        let position = self.get(result, key)?;
        let mark = self.keep(position);
        let position = self.to_integer_or_infinity(position);
        self.release(mark);
        let position = clamp_index(position?, length);
        let mut captures = Charged::new();
        let made = self
            .regexp_replace_captures(result, captures_count, &mut captures, room)
            .and_then(|()| {
                let substitution = Substitution {
                    matched,
                    string,
                    position,
                    captures: &captures.list,
                    named_captures: None,
                };
                self.regexp_replacement_string(result, &substitution, replacement, room)
            });
        self.heap.accounting.credit(captures.bytes);
        Ok(Piece {
            position,
            match_length,
            replacement: made?,
        })
    }

    /// Steps 15.g to 15.i of `@@replace`: captures 1 to nCaptures read from `result`, each
    /// converted by ToString unless undefined and kept in `room`, a step each.
    fn regexp_replace_captures(
        &mut self,
        result: ObjectRef,
        captures_count: f64,
        captures: &mut Charged<Option<StringRef>>,
        room: &mut KeptRoom,
    ) -> Completion<()> {
        // g. Let captures be a new empty List. h. Let n be 1. i. Repeat, while n ≤ nCaptures,
        let mut n = 1.0;
        while n <= captures_count {
            self.charge_steps(1)?;
            // i. Let capN be ? Get(result, ! ToString(𝔽(n))).
            let key = self.index_key(n)?;
            let capture = self.get(result, key)?;
            // ii. If capN is not undefined, then 1. Set capN to ? ToString(capN). iii. Append
            // capN to captures.
            let capture = match capture {
                Value::Undefined => None,
                capture => {
                    let capture = self.string_of_kept(capture)?;
                    self.keep_in(room, Value::String(capture))?;
                    Some(capture)
                }
            };
            self.push_charged(captures, capture)?;
            // v. Set n to n + 1.
            n += 1.0;
        }
        Ok(())
    }

    /// Steps 15.j to 15.l of `@@replace`: namedCaptures read from `result`, then
    /// replacementString by a call of the replacer with (matched, the captures, position, S
    /// and namedCaptures when it is not undefined) and ToString of its answer, or by
    /// GetSubstitution with namedCaptures converted by ToObject. The String made is kept in
    /// `room`.
    fn regexp_replacement_string(
        &mut self,
        result: ObjectRef,
        substitution: &Substitution<'_>,
        replacement: Replacement,
        room: &mut KeptRoom,
    ) -> Completion<StringRef> {
        // j. Let namedCaptures be ? Get(result, "groups").
        let key = self.name_key("groups")?;
        let named_captures = self.get(result, key)?;
        self.keep_in(room, named_captures)?;
        let made = match replacement {
            // k. If functionalReplace is true, then
            Replacement::Function(function) => {
                self.call_replacer_of_result(function, substitution, named_captures)?
            }
            // l. Else, i. If namedCaptures is not undefined, then 1. Set namedCaptures to ?
            // ToObject(namedCaptures). ii. Let replacementString be ? GetSubstitution(matched,
            // S, position, captures, namedCaptures, replaceValue).
            Replacement::Template(template) => {
                let named_captures = match named_captures {
                    Value::Undefined => None,
                    value => {
                        let object = self.to_object(value)?;
                        self.keep_in(room, Value::Object(object))?;
                        Some(object)
                    }
                };
                let substitution = Substitution {
                    named_captures,
                    ..*substitution
                };
                self.get_substitution(&substitution, template)?
            }
        };
        self.keep_in(room, Value::String(made))?;
        Ok(made)
    }

    /// Step 15.k of `@@replace`: "Let replacerArgs be the list-concatenation of « matched »,
    /// captures, and « 𝔽(position), S ». If namedCaptures is not undefined, then append
    /// namedCaptures to replacerArgs. Let replValue be ? Call(replaceValue, undefined,
    /// replacerArgs). Let replacementString be ? ToString(replValue)." The list is charged
    /// before it grows and credited after the call.
    fn call_replacer_of_result(
        &mut self,
        function: Value,
        substitution: &Substitution<'_>,
        named_captures: Value,
    ) -> Completion<StringRef> {
        let mut replacer_args = Charged::new();
        let built = self.replacer_args(&mut replacer_args, substitution, named_captures);
        let answer = match built {
            // The arguments are kept by the caller and by the call's activation.
            Ok(()) => self.call_value(function, Value::Undefined, &replacer_args.list),
            Err(abrupt) => Err(abrupt),
        };
        self.heap.accounting.credit(replacer_args.bytes);
        self.string_of_kept(answer?)
    }

    /// The list replacerArgs of step 15.k.i and ii of `@@replace`, in `list`.
    fn replacer_args(
        &mut self,
        list: &mut Charged<Value>,
        substitution: &Substitution<'_>,
        named_captures: Value,
    ) -> Completion<()> {
        self.push_charged(list, Value::String(substitution.matched))?;
        for capture in substitution.captures {
            let value = capture.map_or(Value::Undefined, Value::String);
            self.push_charged(list, value)?;
        }
        // An index of a String is below 2^53, so the conversion is exact.
        self.push_charged(list, Value::Number(substitution.position as f64))?;
        self.push_charged(list, Value::String(substitution.string))?;
        if !matches!(named_captures, Value::Undefined) {
            self.push_charged(list, named_captures)?;
        }
        Ok(())
    }

    /// `RegExp.prototype [ %Symbol.search% ] ( string )`
    /// (`#sec-regexp.prototype-%symbol.search%`).
    #[inline(never)]
    pub(super) fn regexp_symbol_search(
        &mut self,
        this: Value,
        args: &[Value],
    ) -> Completion<Value> {
        // 1. Let rx be the this value. 2. If rx is not an Object, throw a TypeError exception.
        let regexp = self.regexp_this_object(this, "RegExp.prototype[Symbol.search]")?;
        // 3. Let S be ? ToString(string).
        let string = self.to_string(argument(args, 0))?;
        let mark = self.keep(Value::String(string));
        let answer = self.regexp_search_steps(regexp, string);
        self.release(mark);
        answer
    }

    /// Steps 4 to 9 of `@@search`, with S kept.
    fn regexp_search_steps(&mut self, regexp: ObjectRef, string: StringRef) -> Completion<Value> {
        // 4. Let previousLastIndex be ? Get(rx, "lastIndex").
        let key = self.name_key("lastIndex")?;
        let previous = self.get(regexp, key)?;
        let mark = self.keep(previous);
        let answer = self.regexp_search_kept(regexp, string, previous);
        self.release(mark);
        answer
    }

    /// Steps 5 to 9 of `@@search`, with previousLastIndex kept.
    fn regexp_search_kept(
        &mut self,
        regexp: ObjectRef,
        string: StringRef,
        previous: Value,
    ) -> Completion<Value> {
        // 5. If previousLastIndex is not +0𝔽, then a. Perform ? Set(rx, "lastIndex", +0𝔽,
        // true).
        if !self.same_value(previous, Value::Number(0.0))? {
            self.set_last_index(regexp, Value::Number(0.0))?;
        }
        // 6. Let result be ? RegExpExec(rx, S).
        let result = self.regexp_exec(regexp, string)?;
        let mark = self.keep(result);
        let answer = self.regexp_search_restore(regexp, previous, result);
        self.release(mark);
        answer
    }

    /// Steps 7 to 9 of `@@search`, with result kept.
    fn regexp_search_restore(
        &mut self,
        regexp: ObjectRef,
        previous: Value,
        result: Value,
    ) -> Completion<Value> {
        // 7. Let currentLastIndex be ? Get(rx, "lastIndex").
        let key = self.name_key("lastIndex")?;
        let current = self.get(regexp, key)?;
        // 8. If SameValue(currentLastIndex, previousLastIndex) is false, then a. Perform ?
        // Set(rx, "lastIndex", previousLastIndex, true).
        if !self.same_value(current, previous)? {
            self.set_last_index(regexp, previous)?;
        }
        // 9. If result is null, return -1𝔽. 10. Return ? Get(result, "index").
        match result {
            Value::Object(result) => {
                let key = self.name_key("index")?;
                self.get(result, key)
            }
            _ => Ok(Value::Number(-1.0)),
        }
    }

    /// `RegExp.prototype [ %Symbol.split% ] ( string, limit )`
    /// (`#sec-regexp.prototype-%symbol.split%`).
    #[inline(never)]
    pub(super) fn regexp_symbol_split(&mut self, this: Value, args: &[Value]) -> Completion<Value> {
        // 1. Let rx be the this value. 2. If rx is not an Object, throw a TypeError exception.
        let regexp = self.regexp_this_object(this, "RegExp.prototype[Symbol.split]")?;
        // 3. Let S be ? ToString(string).
        let string = self.to_string(argument(args, 0))?;
        let mark = self.keep(Value::String(string));
        let answer = self.regexp_split_splitter(regexp, string, argument(args, 1));
        self.release(mark);
        answer.map(Value::Object)
    }

    /// Steps 4 to 10 of `@@split`, with S kept: the splitter made by Construct of the species
    /// constructor, kept, then the parts.
    fn regexp_split_splitter(
        &mut self,
        regexp: ObjectRef,
        string: StringRef,
        limit: Value,
    ) -> Completion<ObjectRef> {
        // 4. Let C be ? SpeciesConstructor(rx, %RegExp%).
        let default = self.current_intrinsic(Intrinsic::RegExp)?;
        let constructor = self.species_constructor(regexp, default)?;
        let mark = self.keep(Value::Object(constructor));
        let made = self.regexp_split_construct(regexp, constructor);
        self.release(mark);
        let (splitter, unicode_matching) = made?;
        let mark = self.keep(Value::Object(splitter));
        let parts = self.regexp_split_parts(splitter, string, limit, unicode_matching);
        self.release(mark);
        parts
    }

    /// Steps 5 to 10 of `@@split`, with C kept: "Let flags be ? ToString(? Get(rx, "flags")).
    /// If flags contains "u" or flags contains "v", let unicodeMatching be true; else false. If
    /// flags contains "y", let newFlags be flags; else let newFlags be the string-concatenation
    /// of flags and "y". Let splitter be ? Construct(C, « rx, newFlags »)."
    fn regexp_split_construct(
        &mut self,
        regexp: ObjectRef,
        constructor: ObjectRef,
    ) -> Completion<(ObjectRef, bool)> {
        let flags = self.regexp_flags_read(regexp)?;
        let new_flags = if flags.sticky {
            flags.string
        } else {
            let mut built = StringBuilder::new();
            self.append_string(&mut built, flags.string)?;
            self.append_units(&mut built, &[FLAG_Y])?;
            self.finish_string(built)?
        };
        let mark = self.keep(Value::String(new_flags));
        let made = self.construct_value(
            constructor,
            &[Value::Object(regexp), Value::String(new_flags)],
            None,
        );
        self.release(mark);
        match made? {
            Value::Object(splitter) => Ok((splitter, flags.full_unicode)),
            _ => Err(self.fault()),
        }
    }

    /// Steps 11 to 25 of `@@split`, with S and the splitter kept: A made and kept, then
    /// filled.
    fn regexp_split_parts(
        &mut self,
        splitter: ObjectRef,
        string: StringRef,
        limit: Value,
        unicode_matching: bool,
    ) -> Completion<ObjectRef> {
        // 11. Let A be ! ArrayCreate(0). 12. Let lengthA be 0.
        let array = self.array_create(0.0, None)?;
        let mark = self.keep(Value::Object(array));
        let filled = self.regexp_split_fill(splitter, string, limit, unicode_matching, array);
        self.release(mark);
        filled.map(|()| array)
    }

    /// Steps 13 to 25 of `@@split`, with A kept.
    fn regexp_split_fill(
        &mut self,
        splitter: ObjectRef,
        string: StringRef,
        limit: Value,
        unicode_matching: bool,
        array: ObjectRef,
    ) -> Completion<()> {
        // 13. If limit is undefined, let lim be 2^32 - 1; else let lim be ℝ(? ToUint32(limit)).
        let limit = match limit {
            Value::Undefined => u32::MAX,
            limit => self.to_uint32(limit)?,
        };
        // 14. If lim = 0, return A.
        if limit == 0 {
            return Ok(());
        }
        let size = self.string_length(string)?;
        // 15. If S is the empty String, then a. Let z be ? RegExpExec(splitter, S). b. If z is
        // not null, return A. c. Perform ! CreateDataPropertyOrThrow(A, "0", S). d. Return A.
        if size == 0 {
            if matches!(self.regexp_exec(splitter, string)?, Value::Null) {
                self.create_data_property_or_throw(array, Key::Index(0), Value::String(string))?;
            }
            return Ok(());
        }
        let mut parts = SplitParts {
            array,
            length: 0.0,
            limit: f64::from(limit),
        };
        // 16. Let size be the length of S. 17. Let p be 0. 18. Let q be p.
        let mut p = 0;
        let mut q = 0;
        // 19. Repeat, while q < size,
        while q < size {
            self.charge_steps(1)?;
            // a. Perform ? Set(splitter, "lastIndex", 𝔽(q), true). b. Let z be ?
            // RegExpExec(splitter, S).
            self.set_last_index(splitter, Value::Number(q as f64))?;
            let found = self.regexp_exec(splitter, string)?;
            // c. If z is null, set q to AdvanceStringIndex(S, q, unicodeMatching).
            let Value::Object(found) = found else {
                q = self.advance_index(string, q, unicode_matching)?;
                continue;
            };
            // d. Else, with z kept.
            let mark = self.keep(Value::Object(found));
            let step = self.split_at_match(&mut parts, splitter, string, found, (p, q, size));
            self.release(mark);
            match step? {
                // iii. If e = p, set q to AdvanceStringIndex(S, q, unicodeMatching).
                SplitStep::Advance => q = self.advance_index(string, q, unicode_matching)?,
                SplitStep::Done => return Ok(()),
                // iv.5. Set p to e. iv.12. Set q to p.
                SplitStep::Moved(end) => {
                    p = end;
                    q = p;
                }
            }
        }
        // 20. Let T be the substring of S from p to size. 21. Perform !
        // CreateDataPropertyOrThrow(A, ! ToString(𝔽(lengthA)), T). 22. Return A.
        self.split_push_substring(&mut parts, string, p, size)?;
        Ok(())
    }

    /// Step 19.d of `@@split` for the match `found`, kept, at the positions (p, q, size): "i.
    /// Let e be ℝ(? ToLength(? Get(splitter, "lastIndex"))). ii. Set e to min(e, size). iii.
    /// If e = p, advance q. iv. Else, 1. Let T be the substring of S from p to q. 2. Perform !
    /// CreateDataPropertyOrThrow(A, ! ToString(𝔽(lengthA)), T). 3. Set lengthA to lengthA + 1.
    /// 4. If lengthA = lim, return A." Then the captures of z; p moves to e.
    fn split_at_match(
        &mut self,
        parts: &mut SplitParts,
        splitter: ObjectRef,
        string: StringRef,
        found: ObjectRef,
        (p, q, size): (usize, usize, usize),
    ) -> Completion<SplitStep> {
        let end = self.last_index_length(splitter)?;
        let end = usize::try_from(end).unwrap_or(usize::MAX).min(size);
        if end == p {
            return Ok(SplitStep::Advance);
        }
        if self.split_push_substring(parts, string, p, q)?
            || self.split_push_captures(parts, found)?
        {
            return Ok(SplitStep::Done);
        }
        Ok(SplitStep::Moved(end))
    }

    /// AdvanceStringIndex of a position of a String, as an index.
    fn advance_index(
        &mut self,
        string: StringRef,
        index: usize,
        unicode: bool,
    ) -> Completion<usize> {
        let next = self.advance_string_index(string, count(index), unicode)?;
        Ok(usize::try_from(next).unwrap_or(usize::MAX))
    }

    /// Steps 19.d.iv.6 to 11 of `@@split`, with z kept: "Let numberOfCaptures be ?
    /// LengthOfArrayLike(z). Set numberOfCaptures to max(numberOfCaptures - 1, 0). Let i be 1.
    /// Repeat, while i ≤ numberOfCaptures, a. Let nextCapture be ? Get(z, ! ToString(𝔽(i))).
    /// b. Perform ! CreateDataPropertyOrThrow(A, ! ToString(𝔽(lengthA)), nextCapture). c. Set
    /// i to i + 1. d. Set lengthA to lengthA + 1. e. If lengthA = lim, return A." A step per
    /// capture; whether A is done.
    fn split_push_captures(
        &mut self,
        parts: &mut SplitParts,
        found: ObjectRef,
    ) -> Completion<bool> {
        let captures = (self.length_of_array_like(found)? - 1.0).max(0.0);
        let mut i = 1.0;
        while i <= captures {
            self.charge_steps(1)?;
            let key = self.index_key(i)?;
            let capture = self.get(found, key)?;
            if self.split_push(parts, capture)? {
                return Ok(true);
            }
            i += 1.0;
        }
        Ok(false)
    }

    /// The substring of `string` from `from` to `to`, made by a [`StringBuilder`], pushed to
    /// A; whether A is done.
    fn split_push_substring(
        &mut self,
        parts: &mut SplitParts,
        string: StringRef,
        from: usize,
        to: usize,
    ) -> Completion<bool> {
        let units = self.read_units(string, from, to)?;
        let mut part = StringBuilder::new();
        self.append_units(&mut part, &units)?;
        let part = self.finish_string(part)?;
        self.split_push(parts, Value::String(part))
    }

    /// "Perform ! CreateDataPropertyOrThrow(A, ! ToString(𝔽(lengthA)), value). Set lengthA to
    /// lengthA + 1. If lengthA = lim, return A": whether A is done. No code runs on the new
    /// Array, so `value` needs no root.
    fn split_push(&mut self, parts: &mut SplitParts, value: Value) -> Completion<bool> {
        let key = self.index_key(parts.length)?;
        self.create_data_property_or_throw(parts.array, key, value)?;
        parts.length += 1.0;
        Ok(parts.length == parts.limit)
    }
}

/// What step 19.d of `@@split` did with a match: q is to advance (e = p), A is done (lengthA =
/// lim), or p moved to e.
enum SplitStep {
    Advance,
    Done,
    Moved(usize),
}

/// The Array of `@@split` as it is filled: A, lengthA and lim.
struct SplitParts {
    array: ObjectRef,
    length: f64,
    limit: f64,
}
