//! `RegExp.prototype [ %Symbol.matchAll% ]` (`#sec-regexp-prototype-%symbol.matchall%`) and
//! the RegExp String Iterators (22.2.9, `#sec-regexp-string-iterator-objects`), written by
//! `story:regexp-match-all-and-escape`: CreateRegExpStringIterator
//! (`#sec-createregexpstringiterator`) and `%RegExpStringIteratorPrototype%`
//! (`#sec-%regexpstringiteratorprototype%-object`) with `next`
//! (`#sec-%regexpstringiteratorprototype%.next`) and `[%Symbol.toStringTag%]`
//! (`#sec-%regexpstringiteratorprototype%-%symbol.tostringtag%`).
//!
//! `@@matchAll` constructs its matcher by SpeciesConstructor (`#sec-speciesconstructor`) with
//! the String of Get of `flags`, and copies `lastIndex` by ToLength; the iterator holds the
//! matcher and the String in its slots ([[IteratingRegExp]], [[IteratedString]], with
//! [[Global]], [[Unicode]] and [[Done]]: `OrdinaryData::RegExpStringIterator`, which the
//! collector traces, `object/ordinary.rs`). Each `next` matches through RegExpExec
//! (`#sec-regexpexec`), so a user `exec` answers for an object of a script, and moves
//! `lastIndex` past an empty match by AdvanceStringIndex (`#sec-advancestringindex`).
//!
//! The budgets (`docs/design/engine.md`, decisions 3 and 7): the call path charges each call
//! of `next` its step, and `next` charges one of its own besides the steps of its match, as
//! each pass of the loops of `@@match` does (`regexp/symbol.rs`); the code units of `flags` are
//! read with a step each. The iterator object is charged when it is made, as every object is.
//! The species constructor, a getter of `flags` or `lastIndex`, the conversions of `lastIndex`
//! and of the match String and a user `exec` are calls of the agent's call path, so each is a
//! re-entry; every value the steps hold across one is kept.

use super::RegExpBuiltin;
use crate::builtins::object::argument;
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::host::{ErrorKind, Host};
use crate::object::ObjectKind;
use crate::object::builtin_function::BuiltinId;
use crate::object::ordinary::OrdinaryData;
use crate::object::property::Key;
use crate::realm::intrinsics::{Intrinsic, Making};
use crate::value::{ObjectRef, StringRef, Value};

/// The code unit 0x0067 (LATIN SMALL LETTER G).
const FLAG_G: u16 = 0x0067;
/// The code unit 0x0075 (LATIN SMALL LETTER U).
const FLAG_U: u16 = 0x0075;
/// The code unit 0x0076 (LATIN SMALL LETTER V).
const FLAG_V: u16 = 0x0076;

/// The slots of a RegExp String Iterator as `next` reads them (Table 69): [[IteratingRegExp]],
/// [[IteratedString]], [[Global]], [[Unicode]] and [[Done]].
#[derive(Clone, Copy)]
struct IteratorSlots {
    regexp: ObjectRef,
    string: StringRef,
    global: bool,
    unicode: bool,
    done: bool,
}

impl<H: Host> Agent<H> {
    /// `%RegExpStringIteratorPrototype%` (`#sec-%regexpstringiteratorprototype%-object`): an
    /// ordinary object whose [[Prototype]] is `iterator_prototype`, `%Iterator.prototype%`,
    /// with `next` of "length" 0 { [[Writable]]: true, [[Enumerable]]: false, [[Configurable]]:
    /// true } and `[%Symbol.toStringTag%]` "RegExp String Iterator" { [[Writable]]: false,
    /// [[Enumerable]]: false, [[Configurable]]: true }.
    pub(crate) fn create_regexp_string_iterator_prototype(
        &mut self,
        making: Making,
        iterator_prototype: ObjectRef,
    ) -> Completion<ObjectRef> {
        let prototype =
            self.ordinary_object_create(Some(iterator_prototype), OrdinaryData::Plain)?;
        self.define_builtin_method(
            making,
            prototype,
            "next",
            0,
            BuiltinId::RegExp(RegExpBuiltin::StringIteratorNext),
        )?;
        self.define_to_string_tag(making, prototype, "RegExp String Iterator")?;
        Ok(prototype)
    }

    /// `RegExp.prototype [ %Symbol.matchAll% ] ( string )`
    /// (`#sec-regexp-prototype-%symbol.matchall%`).
    #[inline(never)]
    pub(super) fn regexp_symbol_match_all(
        &mut self,
        this: Value,
        args: &[Value],
    ) -> Completion<Value> {
        // 1. Let regexp be the this value. 2. If regexp is not an Object, throw a TypeError
        // exception.
        let Value::Object(regexp) = this else {
            return Err(self.regexp_this_not_object("RegExp.prototype[Symbol.matchAll]"));
        };
        // 3. Set string to ? ToString(string).
        let string = self.to_string(argument(args, 0))?;
        let mark = self.keep(Value::String(string));
        let answer = self.match_all_steps(regexp, string);
        self.release(mark);
        answer.map(Value::Object)
    }

    /// Steps 4 to 11 of `@@matchAll`, with string kept: the matcher made with the species
    /// constructor kept, then kept itself while `lastIndex` is copied and the iterator made.
    fn match_all_steps(&mut self, regexp: ObjectRef, string: StringRef) -> Completion<ObjectRef> {
        // 4. Let speciesCtor be ? SpeciesConstructor(regexp, %RegExp%).
        let default = self.current_intrinsic(Intrinsic::RegExp)?;
        let constructor = self.species_constructor(regexp, default)?;
        let mark = self.keep(Value::Object(constructor));
        let made = self.match_all_construct(regexp, constructor);
        self.release(mark);
        let (matcher, global, full_unicode) = made?;
        let mark = self.keep(Value::Object(matcher));
        let answer = self
            .match_all_copy_last_index(regexp, matcher)
            .and_then(|()| {
                self.create_regexp_string_iterator(matcher, string, global, full_unicode)
            });
        self.release(mark);
        answer
    }

    /// Steps 5, 6, 9 and 10 of `@@matchAll`, with speciesCtor kept: "Let flags be ? ToString(?
    /// Get(regexp, "flags")). Let matcher be ? Construct(speciesCtor, « regexp, flags »)", and
    /// whether flags contains "g", and "u" or "v" (each code unit read with a step).
    fn match_all_construct(
        &mut self,
        regexp: ObjectRef,
        constructor: ObjectRef,
    ) -> Completion<(ObjectRef, bool, bool)> {
        let key = self.name_key("flags")?;
        let value = self.get(regexp, key)?;
        let mark = self.keep(value);
        let flags = self.to_string(value);
        self.release(mark);
        let flags = flags?;
        let length = self.string_length(flags)?;
        let units = self.read_units(flags, 0, length)?;
        let global = units.contains(&FLAG_G);
        let full_unicode = units.contains(&FLAG_U) || units.contains(&FLAG_V);
        let mark = self.keep(Value::String(flags));
        let made = self.construct_value(
            constructor,
            &[Value::Object(regexp), Value::String(flags)],
            None,
        );
        self.release(mark);
        match made? {
            Value::Object(matcher) => Ok((matcher, global, full_unicode)),
            // Construct answers an Object (`#sec-construct`, step 3).
            _ => Err(self.fault()),
        }
    }

    /// Steps 7 and 8 of `@@matchAll`, with matcher kept: "Let lastIndex be ? ToLength(?
    /// Get(regexp, "lastIndex")). Perform ? Set(matcher, "lastIndex", lastIndex, true)."
    fn match_all_copy_last_index(
        &mut self,
        regexp: ObjectRef,
        matcher: ObjectRef,
    ) -> Completion<()> {
        let key = self.name_key("lastIndex")?;
        let value = self.get(regexp, key)?;
        let mark = self.keep(value);
        let last_index = self.to_length(value);
        self.release(mark);
        self.set(matcher, key, Value::Number(last_index?), true)?;
        Ok(())
    }

    /// CreateRegExpStringIterator ( regexp, string, global, fullUnicode )
    /// (`#sec-createregexpstringiterator`): an ordinary object of
    /// `%RegExpStringIteratorPrototype%` of the current realm with [[IteratingRegExp]] `regexp`,
    /// [[IteratedString]] `string`, [[Global]] `global`, [[Unicode]] `unicode` and [[Done]]
    /// false. No code runs while it is made.
    fn create_regexp_string_iterator(
        &mut self,
        regexp: ObjectRef,
        string: StringRef,
        global: bool,
        unicode: bool,
    ) -> Completion<ObjectRef> {
        let prototype = self.current_intrinsic(Intrinsic::RegExpStringIteratorPrototype)?;
        self.ordinary_object_create(
            Some(prototype),
            OrdinaryData::RegExpStringIterator {
                regexp,
                string,
                global,
                unicode,
                done: false,
            },
        )
    }

    /// `%RegExpStringIteratorPrototype%.next ( )` (`#sec-%regexpstringiteratorprototype%.next`).
    /// The iterator is held by the activation as its this value and holds its RegExp and its
    /// String; both are kept besides while the match runs code.
    #[inline(never)]
    pub(super) fn regexp_string_iterator_next(&mut self, this: Value) -> Completion<Value> {
        // 1. Let iteratorObj be the this value. 2. If iteratorObj is not an Object, throw a
        // TypeError exception. 3. If iteratorObj does not have all of the internal slots of a
        // RegExp String Iterator Object Instance (22.2.9.3), throw a TypeError exception.
        let slots = match this {
            Value::Object(iterator) => match &self.engine_object(iterator)?.kind {
                ObjectKind::Ordinary(ordinary) => match ordinary.data {
                    OrdinaryData::RegExpStringIterator {
                        regexp,
                        string,
                        global,
                        unicode,
                        done,
                    } => Some((
                        iterator,
                        IteratorSlots {
                            regexp,
                            string,
                            global,
                            unicode,
                            done,
                        },
                    )),
                    _ => None,
                },
                _ => None,
            },
            _ => None,
        };
        let Some((iterator, slots)) = slots else {
            return Err(self.throw_error(
                ErrorKind::TypeError,
                "%RegExpStringIteratorPrototype%.next requires that 'this' be a RegExp String Iterator",
            ));
        };
        // 4. If iteratorObj.[[Done]] is true, then a. Return CreateIteratorResultObject(undefined,
        // true).
        if slots.done {
            return Ok(Value::Object(
                self.create_iter_result_object(Value::Undefined, true)?,
            ));
        }
        // The step of this `next`, besides those of its match.
        self.charge_steps(1)?;
        let mark = self.keep(Value::Object(slots.regexp));
        self.keep(Value::String(slots.string));
        let answer = self.regexp_string_iterator_match(iterator, slots);
        self.release(mark);
        answer
    }

    /// Steps 5 to 14 of `next`, with regexp and string kept.
    fn regexp_string_iterator_match(
        &mut self,
        iterator: ObjectRef,
        slots: IteratorSlots,
    ) -> Completion<Value> {
        // 5 to 8. regexp, string, global and fullUnicode from the slots. 9. Let match be ?
        // RegExpExec(regexp, string).
        let found = self.regexp_exec(slots.regexp, slots.string)?;
        // 10. If match is null, then a. Set iteratorObj.[[Done]] to true. b. Return
        // CreateIteratorResultObject(undefined, true).
        let Value::Object(found) = found else {
            self.finish_regexp_string_iterator(iterator)?;
            return Ok(Value::Object(
                self.create_iter_result_object(Value::Undefined, true)?,
            ));
        };
        // 11. If global is false, then a. Set iteratorObj.[[Done]] to true. b. Return
        // CreateIteratorResultObject(match, false). No code runs after the match is made.
        if !slots.global {
            self.finish_regexp_string_iterator(iterator)?;
            return Ok(Value::Object(
                self.create_iter_result_object(Value::Object(found), false)?,
            ));
        }
        let mark = self.keep(Value::Object(found));
        let answer = self
            .regexp_string_iterator_advance(slots, found)
            .and_then(|()| self.create_iter_result_object(Value::Object(found), false));
        self.release(mark);
        // 14. Return CreateIteratorResultObject(match, false).
        answer.map(Value::Object)
    }

    /// Steps 12 and 13 of `next`, with match kept: "Let matchStr be ? ToString(? Get(match,
    /// "0")). If matchStr is the empty String, then a. Let thisIndex be ℝ(? ToLength(?
    /// Get(regexp, "lastIndex"))). b. Let nextIndex be AdvanceStringIndex(string, thisIndex,
    /// fullUnicode). c. Perform ? Set(regexp, "lastIndex", 𝔽(nextIndex), true)."
    fn regexp_string_iterator_advance(
        &mut self,
        slots: IteratorSlots,
        found: ObjectRef,
    ) -> Completion<()> {
        let value = self.get(found, Key::Index(0))?;
        let mark = self.keep(value);
        let matched = self.to_string(value);
        self.release(mark);
        if self.string_length(matched?)? != 0 {
            return Ok(());
        }
        let key = self.name_key("lastIndex")?;
        let value = self.get(slots.regexp, key)?;
        let mark = self.keep(value);
        let this_index = self.to_length(value);
        self.release(mark);
        // ToLength answers an integer from 0 to 2^53 - 1, which the casts keep exactly.
        let next = self.advance_string_index(slots.string, this_index? as u64, slots.unicode)?;
        self.set(slots.regexp, key, Value::Number(next as f64), true)?;
        Ok(())
    }

    /// Set [[Done]] of the RegExp String Iterator `iterator` to true; an object without the
    /// slot here is a broken invariant.
    fn finish_regexp_string_iterator(&mut self, iterator: ObjectRef) -> Completion<()> {
        let changed = match &mut self.engine_object_mut(iterator)?.kind {
            ObjectKind::Ordinary(ordinary) => match &mut ordinary.data {
                OrdinaryData::RegExpStringIterator { done, .. } => {
                    *done = true;
                    true
                }
                _ => false,
            },
            _ => false,
        };
        if changed { Ok(()) } else { Err(self.fault()) }
    }
}
