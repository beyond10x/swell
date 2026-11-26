//! The JSON object (ECMA-262 25.5, `#sec-json-object`): `JSON.stringify` (`#sec-json.stringify`)
//! with SerializeJSONProperty (`#sec-serializejsonproperty`), QuoteJSONString
//! (`#sec-quotejsonstring`), UnicodeEscape (`#sec-unicodeescape`), SerializeJSONObject
//! (`#sec-serializejsonobject`) and SerializeJSONArray (`#sec-serializejsonarray`), and
//! `JSON[@@toStringTag]` (`#sec-json-%symbol.tostringtag%`). Written by `story:harness-builtins`.
//! `story:json-parse-and-raw-json` completes the clause: `JSON.isRawJSON`, `JSON.parse` and
//! `JSON.rawJSON` are in `parse.rs`, defined after the keys of the base in the order of their
//! clauses, and SerializeJSONProperty writes an object with [[IsRawJSON]] as its text (step
//! 4.a).
//!
//! The serialization walks nested values on an explicit stack of frames, one per object or
//! array being serialized, so a nesting of any depth takes no Rust stack (decision 3, "Depth
//! bounds"); each frame is charged to the memory budget while it is open, and each member a
//! step. The result is written as it is made into one [`StringBuilder`], so it is bounded by
//! `string_length` as it grows: past it the `RangeError` comes before the String is allocated.
//! A member whose SerializeJSONProperty is undefined is known before anything of it is
//! written, so writing as it goes gives the Strings the clauses concatenate. What the walk
//! holds across code (the wrapper, the property list, each open frame's object and keys) is
//! on the agent's kept stack (decision 4).

use alloc::format;
use alloc::string::ToString;
use alloc::vec;
use alloc::vec::Vec;

mod parse;

use alloc::collections::BTreeSet;

use crate::builtins::object::argument;
use crate::builtins::string::{StringBuilder, units_of};
use crate::heap::accounting::ChargedVec;
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::host::{ErrorKind, Host, Termination};
use crate::object::ObjectKind;
use crate::object::builtin_function::{BuiltinId, KeptRoom};
use crate::object::ordinary::OrdinaryData;
use crate::object::property::Key;
use crate::realm::intrinsics::{Intrinsic, Making};
use crate::value::{ObjectRef, StringRef, Value};

/// The builtins of this module, by their steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum JsonBuiltin {
    /// `JSON.stringify ( value [ , replacer [ , space ] ] )` (25.5.4).
    Stringify,
    /// `JSON.isRawJSON ( obj )` (25.5.1).
    IsRawJson,
    /// `JSON.parse ( text [ , reviver ] )` (25.5.2).
    Parse,
    /// `JSON.rawJSON ( text )` (25.5.3).
    RawJson,
}

/// The most code units of a gap (`#sec-json.stringify`, steps 7 and 8).
const GAP_MAX: usize = 10;

/// The code units QuoteJSONString gathers before it appends them to the result.
const QUOTE_CHUNK: usize = 256;

/// The key of a property being serialized: a key of EnumerableOwnProperties, kept by its
/// frame; a String of the property list, kept for the whole call; or the index of an array
/// element, whose key is made again where it is used.
#[derive(Clone, Copy)]
enum JsonKey {
    Key(Key),
    String(StringRef),
    Index(f64),
}

/// The members of an open object or array.
enum Members {
    /// SerializeJSONObject: the keys (`None` for the property list) and the next one.
    Object { keys: Option<Vec<Key>>, next: usize },
    /// SerializeJSONArray: the length and the next index.
    Array { length: f64, next: f64 },
}

/// An object or array being serialized: an element of state.[[Stack]].
struct Frame {
    object: ObjectRef,
    members: Members,
    /// Whether a member was written.
    written: bool,
    /// stepBack: the length of state.[[Indent]] when the frame opened.
    step_back: usize,
    /// The kept stack's length when the frame opened.
    mark: usize,
    /// The bytes charged for the frame.
    charged: u64,
}

/// The JSON Serialization Record (`#sec-json-serialization-record`), with the frames that
/// stand for the nested calls of SerializeJSONProperty and the result being written.
struct State<'r> {
    replacer: Option<Value>,
    /// The property list, its Strings kept for the whole call.
    property_list: Option<ChargedVec<StringRef>>,
    gap: Vec<u16>,
    indent: ChargedVec<u16>,
    /// The handles of the objects on state.[[Stack]], for the cycle check.
    on_stack: BTreeSet<(u32, u32, u32)>,
    frames: ChargedVec<Frame>,
    out: StringBuilder,
    /// The bytes charged for the frames, their lists of keys and the property list, credited
    /// when they go.
    charged: u64,
    /// The room of the kept stack for the call (`Agent::with_kept_room`): the growth charged
    /// for the keys the frames keep and the items of the property list, given back when the
    /// call ends.
    room: &'r mut KeptRoom,
}

/// What SerializeJSONProperty of a value gives, before anything is written: undefined, a
/// String of JSON text, a String value to quote, the text of an object with [[IsRawJSON]], or
/// an object or array to open. A String is kept while it is read.
enum Serialized {
    Undefined,
    Text(Vec<u16>),
    Quoted(StringRef),
    Raw(StringRef),
    Open(ObjectRef),
}

/// The handle of `object` as an ordered key.
fn identity(object: ObjectRef) -> (u32, u32, u32) {
    (
        object.handle.key,
        object.handle.index,
        object.handle.generation,
    )
}

impl<H: Host> Agent<H> {
    /// `%JSON%` (25.5): an ordinary object whose [[Prototype]] is `%Object.prototype%`, with
    /// `stringify` and @@toStringTag "JSON" (25.5.5), then `isRawJSON`, `parse` and `rawJSON`
    /// (25.5.1 to 25.5.3).
    pub(crate) fn create_json_object(&mut self, making: Making) -> Completion<ObjectRef> {
        let json =
            self.ordinary_object_create(Some(making.object_prototype), OrdinaryData::Plain)?;
        self.define_builtin_method(
            making,
            json,
            "stringify",
            3,
            BuiltinId::Json(JsonBuiltin::Stringify),
        )?;
        self.define_to_string_tag(making, json, "JSON")?;
        for (name, length, id) in [
            ("isRawJSON", 1, JsonBuiltin::IsRawJson),
            ("parse", 2, JsonBuiltin::Parse),
            ("rawJSON", 1, JsonBuiltin::RawJson),
        ] {
            self.define_builtin_method(making, json, name, length, BuiltinId::Json(id))?;
        }
        Ok(json)
    }

    /// The steps of the builtins of this module.
    pub(crate) fn run_json_builtin(
        &mut self,
        id: JsonBuiltin,
        _function: ObjectRef,
        _this: Value,
        args: &[Value],
        _new_target: Option<ObjectRef>,
    ) -> Completion<Value> {
        match id {
            JsonBuiltin::Stringify => {
                // The room is ended on every way out, as `Agent::with_kept_room` ends it;
                // written out here so that no closure's frame stands between this call and a
                // re-entry of `toJSON` or the replacer.
                let mark = self.kept.len();
                let mut room = self.kept_room();
                let result = {
                    let mut state = State {
                        replacer: None,
                        property_list: None,
                        gap: Vec::new(),
                        indent: ChargedVec::new(),
                        on_stack: BTreeSet::new(),
                        frames: ChargedVec::new(),
                        out: StringBuilder::new(),
                        charged: 0,
                        room: &mut room,
                    };
                    let result = self.json_stringify(&mut state, args);
                    self.heap.accounting.credit(state.charged);
                    result
                };
                self.end_kept_room(mark, &room);
                result
            }
            JsonBuiltin::IsRawJson => self.json_is_raw_json(args),
            JsonBuiltin::Parse => self.json_parse(args),
            JsonBuiltin::RawJson => self.json_raw_json(args),
        }
    }

    /// Charge `bytes` of the walk to the memory budget.
    fn json_charge(&mut self, state: &mut State<'_>, bytes: u64) -> Completion<()> {
        if self.heap.accounting.charge(bytes).is_err() {
            return Err(self.terminate(Termination::MemoryBudget));
        }
        state.charged = state.charged.saturating_add(bytes);
        Ok(())
    }

    /// `JSON.stringify ( value [ , replacer [ , space ] ] )` (`#sec-json.stringify`). The
    /// arguments are held by this builtin's activation.
    fn json_stringify(&mut self, state: &mut State<'_>, args: &[Value]) -> Completion<Value> {
        let value = argument(args, 0);
        let replacer = argument(args, 1);
        // 5. If replacer is an Object, then
        if let Value::Object(replacer_object) = replacer {
            if self.is_callable(replacer)? {
                // 5.a. If IsCallable(replacer) is true, set replacerFunc to replacer.
                state.replacer = Some(replacer);
            } else if self.is_array(replacer)? {
                // 5.b. Else, if IsArray(replacer) is true: the property list.
                self.json_property_list(state, replacer_object)?;
            }
        }
        // 6 to 9. The gap.
        state.gap = self.json_gap(argument(args, 2))?;
        // 10. Let wrapper be OrdinaryObjectCreate(%Object.prototype%).
        let prototype = self.current_intrinsic(Intrinsic::ObjectPrototype)?;
        let wrapper = self.ordinary_object_create(Some(prototype), OrdinaryData::Plain)?;
        self.keep(Value::Object(wrapper));
        // 11. Perform ! CreateDataPropertyOrThrow(wrapper, the empty String, value).
        let empty = self.new_string_str("")?;
        self.keep(Value::String(empty));
        let key = self.key_from_string(empty)?;
        if !self.create_data_property(wrapper, key, value)? {
            return Err(self.fault());
        }
        // 12, 13. Return ? SerializeJSONProperty(state, the empty String, wrapper).
        match self.json_serialize_property(state, JsonKey::String(empty), wrapper)? {
            Serialized::Undefined => return Ok(Value::Undefined),
            serialized => self.json_write(state, serialized)?,
        }
        self.json_walk(state)?;
        let out = core::mem::take(&mut state.out);
        Ok(Value::String(self.finish_string(out)?))
    }

    /// Step 5.b of `JSON.stringify`: the property list of the array `replacer`, each item once,
    /// in order. A step per element and per comparison with an item already listed; the
    /// comparison is in place and charges a step per code unit past 32 of two items of the
    /// same length before it reads them ([`Agent::string_equals`]).
    fn json_property_list(&mut self, state: &mut State<'_>, replacer: ObjectRef) -> Completion<()> {
        // 5.b.i. Set propertyList to a new empty List.
        let mut list: ChargedVec<StringRef> = ChargedVec::new();
        // 5.b.ii. Let len be ? LengthOfArrayLike(replacer).
        let length = self.length_of_array_like(replacer)?;
        // 5.b.iii, iv. For each k below len.
        let mut index = 0.0;
        while index < length {
            self.charge_steps(1)?;
            // 5.b.iv.1, 2. Let prop be ! ToString(𝔽(k)); let v be ? Get(replacer, prop).
            let key = self.index_key(index)?;
            let element = self.get(replacer, key)?;
            // 5.b.iv.3 to 6. A String, a Number, or a String or Number object gives an item.
            let item = match element {
                Value::String(string) => Some(string),
                Value::Number(_) => Some(self.to_string(element)?),
                Value::Object(object) if self.json_is_string_or_number_object(object)? => {
                    Some(self.to_string(element)?)
                }
                _ => None,
            };
            // 5.b.iv.7. If item is not undefined and propertyList does not contain item,
            // append item to propertyList.
            if let Some(item) = item {
                let mut listed = false;
                for &other in &list {
                    self.charge_steps(1)?;
                    if self.string_equals(other, item)? {
                        listed = true;
                        break;
                    }
                }
                if !listed {
                    self.reserve_kept(state.room, 1)?;
                    self.keep(Value::String(item));
                    self.push_vec(&mut list, item)?;
                }
            }
            index += 1.0;
        }
        state.property_list = Some(list);
        Ok(())
    }

    /// Whether `object` has a [[StringData]] or a [[NumberData]] internal slot.
    fn json_is_string_or_number_object(&mut self, object: ObjectRef) -> Completion<bool> {
        Ok(match &self.engine_object(object)?.kind {
            ObjectKind::String(_) => true,
            ObjectKind::Ordinary(ordinary) => matches!(ordinary.data, OrdinaryData::Number(_)),
            _ => false,
        })
    }

    /// Steps 6 to 9 of `JSON.stringify`: the gap of `space`.
    fn json_gap(&mut self, space: Value) -> Completion<Vec<u16>> {
        // 6. If space is an Object: a Number object gives ? ToNumber(space), a String object
        // ? ToString(space).
        let mut space = space;
        if let Value::Object(object) = space {
            let data = match &self.engine_object(object)?.kind {
                ObjectKind::String(_) => Some(false),
                ObjectKind::Ordinary(ordinary) => {
                    matches!(ordinary.data, OrdinaryData::Number(_)).then_some(true)
                }
                _ => None,
            };
            space = match data {
                Some(true) => Value::Number(self.to_number(space)?),
                Some(false) => Value::String(self.to_string(space)?),
                None => space,
            };
        }
        Ok(match space {
            // 7. If space is a Number: min(10, ToIntegerOrInfinity(space)) spaces, none below 1.
            Value::Number(_) => {
                let count = self.to_integer_or_infinity(space)?.min(GAP_MAX as f64);
                if count < 1.0 {
                    Vec::new()
                } else {
                    // 1 ≤ count ≤ 10: an integer that fits.
                    vec![u16::from(b' '); count as usize]
                }
            }
            // 8. If space is a String: its first 10 code units, and no more of it is read.
            Value::String(string) => {
                let length = self.string_length(string)?.min(GAP_MAX);
                self.string_units_in(string, 0, length)?.into_vec()
            }
            // 9. Else, the empty String.
            _ => Vec::new(),
        })
    }

    /// The property key of `key`.
    fn json_key(&mut self, key: JsonKey) -> Completion<Key> {
        match key {
            JsonKey::Key(key) => Ok(key),
            JsonKey::String(string) => self.key_from_string(string),
            JsonKey::Index(index) => self.index_key(index),
        }
    }

    /// The String of `key`, as SerializeJSONProperty passes it to toJSON and the replacer.
    fn json_key_value(&mut self, key: JsonKey) -> Completion<Value> {
        match key {
            JsonKey::Key(key) => self.key_value(key),
            JsonKey::String(string) => Ok(Value::String(string)),
            JsonKey::Index(index) => {
                let key = self.index_key(index)?;
                self.key_value(key)
            }
        }
    }

    /// QuoteJSONString of `key` for a member of an object, appended to the result: an index by
    /// its digits, a String key read in ranges as a String value is.
    fn json_quote_key(&mut self, state: &mut State<'_>, key: JsonKey) -> Completion<()> {
        match key {
            JsonKey::Key(Key::Index(index)) => {
                self.json_quote_units(state, &units_of(&index.to_string()))
            }
            JsonKey::Key(Key::String(string)) | JsonKey::String(string) => {
                self.json_quote_string(state, string)
            }
            JsonKey::Key(Key::Symbol(_)) => Err(self.fault()),
            JsonKey::Index(index) => self.json_quote_units(
                state,
                &units_of(&swell_num::to_string::number_to_string(index)),
            ),
        }
    }

    /// SerializeJSONProperty (`#sec-serializejsonproperty`), steps 1 to 11: the value of `key`
    /// of `holder` after toJSON, the replacer and the unwrapping of a Boolean, Number or String
    /// object; an object that is not callable is opened by the caller (step 12). Each value is
    /// held across the code that follows by the activation it is passed to.
    fn json_serialize_property(
        &mut self,
        state: &State<'_>,
        key: JsonKey,
        holder: ObjectRef,
    ) -> Completion<Serialized> {
        // 1. Let value be ? Get(holder, key).
        let property_key = self.json_key(key)?;
        let mut value = self.get(holder, property_key)?;
        // 2. If value is an Object (no BigInt exists before slice 12), then
        if let Value::Object(_) = value {
            // 2.a. Let toJSON be ? GetV(value, "toJSON").
            let to_json_key = self.name_key("toJSON")?;
            let to_json = self.get_v(value, to_json_key)?;
            // 2.b. If IsCallable(toJSON) is true, set value to ? Call(toJSON, value, « key »).
            if self.is_callable(to_json)? {
                let key_value = self.json_key_value(key)?;
                value = self.call_value(to_json, value, &[key_value])?;
            }
        }
        // 3. If state.[[ReplacerFunction]] is not undefined, set value to
        // ? Call(state.[[ReplacerFunction]], holder, « key, value »).
        if let Some(replacer) = state.replacer {
            let key_value = self.json_key_value(key)?;
            value = self.call_value(replacer, Value::Object(holder), &[key_value, value])?;
        }
        // 4. If value is an Object: an object with [[IsRawJSON]] gives its "rawJSON" (4.a); a
        // Number object gives ? ToNumber(value), a String object ? ToString(value), a Boolean
        // object its [[BooleanData]]; no [[BigIntData]] before slice 12.
        if let Value::Object(object) = value {
            enum Wrapped {
                Raw,
                Number,
                String,
                Boolean(bool),
            }
            let wrapped = match &self.engine_object(object)?.kind {
                ObjectKind::String(_) => Some(Wrapped::String),
                ObjectKind::Ordinary(ordinary) => match ordinary.data {
                    OrdinaryData::RawJson => Some(Wrapped::Raw),
                    OrdinaryData::Number(_) => Some(Wrapped::Number),
                    OrdinaryData::Boolean(boolean) => Some(Wrapped::Boolean(boolean)),
                    _ => None,
                },
                _ => None,
            };
            value = match wrapped {
                Some(Wrapped::Raw) => return self.json_raw_text(object),
                Some(Wrapped::Number) => Value::Number(self.to_number(value)?),
                Some(Wrapped::String) => Value::String(self.to_string(value)?),
                Some(Wrapped::Boolean(boolean)) => Value::Boolean(boolean),
                None => value,
            };
        }
        Ok(match value {
            // 5, 6, 7. null, true and false.
            Value::Null => Serialized::Text(units_of("null")),
            Value::Boolean(true) => Serialized::Text(units_of("true")),
            Value::Boolean(false) => Serialized::Text(units_of("false")),
            // 8. If value is a String, return QuoteJSONString(value).
            Value::String(string) => Serialized::Quoted(string),
            // 9. A finite Number gives ! ToString(value), another "null".
            Value::Number(number) if number.is_finite() => {
                Serialized::Text(units_of(&swell_num::to_string::number_to_string(number)))
            }
            Value::Number(_) => Serialized::Text(units_of("null")),
            // 10. If value is a BigInt, throw a TypeError exception: none exists before slice
            // 12, so one here is a broken invariant.
            Value::BigInt(_) => return Err(self.fault()),
            // 11. If value is an Object and IsCallable(value) is false: an object or array.
            Value::Object(object) if !self.is_callable(value)? => Serialized::Open(object),
            // 12. Return undefined.
            Value::Undefined | Value::Symbol(_) | Value::Object(_) => Serialized::Undefined,
        })
    }

    /// Step 4.a of SerializeJSONProperty: the text of an object with [[IsRawJSON]], ! Get(value,
    /// "rawJSON"), which is a String (the property is frozen as `JSON.rawJSON` made it).
    fn json_raw_text(&mut self, object: ObjectRef) -> Completion<Serialized> {
        let key = self.name_key("rawJSON")?;
        match self.get(object, key)? {
            Value::String(text) => Ok(Serialized::Raw(text)),
            _ => Err(self.fault()),
        }
    }

    /// Write a serialized value that is not undefined: its text, its quoted String, the text
    /// of a raw JSON object, or the opening of its object or array. A String is kept while it
    /// is read: it may be one no object holds, made by toJSON or the replacer.
    fn json_write(&mut self, state: &mut State<'_>, serialized: Serialized) -> Completion<()> {
        match serialized {
            Serialized::Undefined => Err(self.fault()),
            Serialized::Text(units) => self.append_units(&mut state.out, &units),
            Serialized::Quoted(string) => {
                let keep = self.keep(Value::String(string));
                let quoted = self.json_quote_string(state, string);
                self.release(keep);
                quoted
            }
            Serialized::Raw(text) => {
                let keep = self.keep(Value::String(text));
                let appended = self.append_string(&mut state.out, text);
                self.release(keep);
                appended
            }
            Serialized::Open(object) => self.json_open(state, object),
        }
    }

    /// QuoteJSONString (`#sec-quotejsonstring`) of the code units `units`, appended to the
    /// result.
    fn json_quote_units(&mut self, state: &mut State<'_>, units: &[u16]) -> Completion<()> {
        // 1. Let product be the String value consisting solely of the code unit 0x0022.
        let mut chunk: Vec<u16> = vec![u16::from(b'"')];
        self.json_quote_range(state, &mut chunk, units)?;
        // 3. Append the code unit 0x0022 (QUOTATION MARK).
        chunk.push(u16::from(b'"'));
        self.append_units(&mut state.out, &chunk)
    }

    /// QuoteJSONString (`#sec-quotejsonstring`) of `string`, appended to the result. The String
    /// is read in ranges of at most [`QUOTE_CHUNK`] code units, each quoted before the next is
    /// read; a range that would end between the two units of a surrogate pair ends before the
    /// first, so each pair is read whole.
    fn json_quote_string(&mut self, state: &mut State<'_>, string: StringRef) -> Completion<()> {
        let length = self.string_length(string)?;
        // 1. Let product be the String value consisting solely of the code unit 0x0022.
        let mut chunk: Vec<u16> = vec![u16::from(b'"')];
        let mut from = 0;
        while from < length {
            let mut to = from.saturating_add(QUOTE_CHUNK).min(length);
            // A range of QUOTE_CHUNK units that ends before the String does has at least two.
            if to < length
                && matches!(self.code_unit_at(string, to - 1)?, Some(0xD800..=0xDBFF))
                && matches!(self.code_unit_at(string, to)?, Some(0xDC00..=0xDFFF))
            {
                to -= 1;
            }
            let units = self.string_units_in(string, from, to)?;
            self.json_quote_range(state, &mut chunk, &units)?;
            from = to;
        }
        // 3. Append the code unit 0x0022 (QUOTATION MARK).
        chunk.push(u16::from(b'"'));
        self.append_units(&mut state.out, &chunk)
    }

    /// Step 2 of QuoteJSONString for `units`, which end with no half of a surrogate pair whose
    /// other half follows: the code units are gathered in `chunk`, which is appended to the
    /// result and emptied each time it holds [`QUOTE_CHUNK`] units or more.
    fn json_quote_range(
        &mut self,
        state: &mut State<'_>,
        chunk: &mut Vec<u16>,
        units: &[u16],
    ) -> Completion<()> {
        // 2. For each code point of StringToCodePoints(value).
        let mut index = 0;
        while let Some(&unit) = units.get(index) {
            let is_leading = (0xD800..=0xDBFF).contains(&unit);
            let pair = is_leading
                && units
                    .get(index + 1)
                    .is_some_and(|next| (0xDC00..=0xDFFF).contains(next));
            match unit {
                // 2.a. The escapes of Table 78.
                0x08 => chunk.extend(units_of("\\b")),
                0x09 => chunk.extend(units_of("\\t")),
                0x0A => chunk.extend(units_of("\\n")),
                0x0C => chunk.extend(units_of("\\f")),
                0x0D => chunk.extend(units_of("\\r")),
                0x22 => chunk.extend(units_of("\\\"")),
                0x5C => chunk.extend(units_of("\\\\")),
                // 2.c. A surrogate pair is a code point: UTF16EncodeCodePoint gives it back.
                _ if pair => {
                    chunk.push(unit);
                    if let Some(&next) = units.get(index + 1) {
                        chunk.push(next);
                    }
                    index += 1;
                }
                // 2.b. Below 0x0020, or a lone surrogate: UnicodeEscape(unit)
                // (`#sec-unicodeescape`), \u and four lowercase hex digits.
                _ if unit < 0x20 || (0xD800..=0xDFFF).contains(&unit) => {
                    chunk.extend(units_of(&format!("\\u{unit:04x}")));
                }
                // 2.c. Any other code point is itself.
                _ => chunk.push(unit),
            }
            index += 1;
            if chunk.len() >= QUOTE_CHUNK {
                self.append_units(&mut state.out, chunk)?;
                chunk.clear();
            }
        }
        Ok(())
    }

    /// SerializeJSONObject (`#sec-serializejsonobject`) and SerializeJSONArray
    /// (`#sec-serializejsonarray`), steps 1 to 4 (to 6 of an array): the cycle check, the
    /// stack, the indent and the keys or length; the frame is charged and opened.
    fn json_open(&mut self, state: &mut State<'_>, object: ObjectRef) -> Completion<()> {
        // 1. If state.[[Stack]] contains value, throw a TypeError exception.
        if state.on_stack.contains(&identity(object)) {
            return Err(self.throw_error(
                ErrorKind::TypeError,
                "JSON.stringify cannot serialize a cyclic structure",
            ));
        }
        // 2. Append value to state.[[Stack]]; the object is kept while its frame is open.
        let mark = self.kept.len();
        self.keep(Value::Object(object));
        // 3, 4. Let stepBack be state.[[Indent]]; append state.[[Gap]] to state.[[Indent]].
        let step_back = state.indent.len();
        let (members, list_bytes) = if self.is_array(Value::Object(object))? {
            // SerializeJSONArray, 6. Let len be ? LengthOfArrayLike(value).
            let length = self.length_of_array_like(object)?;
            (Members::Array { length, next: 0.0 }, 0)
        } else if state.property_list.is_some() {
            // SerializeJSONObject, 5. Let K be state.[[PropertyList]].
            let keys = None;
            (Members::Object { keys, next: 0 }, 0)
        } else {
            // SerializeJSONObject, 6. Let K be ? EnumerableOwnProperties(value, key)
            // (`ops/objects.rs`): charged before it exists, each key kept in room charged to
            // the call until the frame's mark is released. The frame takes the list's charge
            // over.
            let list = self.enumerable_own_keys(object, state.room)?;
            state.charged = state.charged.saturating_add(list.bytes);
            let keys = Some(list.keys);
            (Members::Object { keys, next: 0 }, list.bytes)
        };
        // The frame's slot on the kept stack and four keys' bytes of `on_stack`; the frame
        // itself and its indent are in lists charged by their capacity.
        let bytes = size_of::<Value>().saturating_add(size_of::<(u32, u32, u32)>() * 4);
        let frame_bytes = u64::try_from(bytes).unwrap_or(u64::MAX);
        self.json_charge(state, frame_bytes)?;
        let charged = frame_bytes.saturating_add(list_bytes);
        self.reserve_vec(&mut state.frames, 1)?;
        self.reserve_vec(&mut state.indent, state.gap.len())?;
        if state.indent.extend_from_slice(&state.gap).is_err() {
            return Err(self.fault());
        }
        state.on_stack.insert(identity(object));
        let pushed = state.frames.push(Frame {
            object,
            members,
            written: false,
            step_back,
            mark,
            charged,
        });
        if pushed.is_err() {
            return Err(self.fault());
        }
        Ok(())
    }

    /// The members of the open frames, the innermost first, until none is open: a step per
    /// member.
    fn json_walk(&mut self, state: &mut State<'_>) -> Completion<()> {
        while let Some(frame) = state.frames.last_mut() {
            let object = frame.object;
            // The next member of the innermost frame, if any.
            let next = match &mut frame.members {
                Members::Object { keys, next } => {
                    let key = match keys {
                        Some(keys) => keys.get(*next).copied().map(JsonKey::Key),
                        None => state
                            .property_list
                            .as_ref()
                            .and_then(|list| list.get(*next))
                            .map(|string| JsonKey::String(*string)),
                    };
                    *next += 1;
                    key.map(|key| (key, false))
                }
                Members::Array { length, next } => {
                    let key = (*next < *length).then_some(JsonKey::Index(*next));
                    *next += 1.0;
                    key.map(|key| (key, true))
                }
            };
            self.charge_steps(1)?;
            let Some((key, is_array)) = next else {
                self.json_close(state)?;
                continue;
            };
            // SerializeJSONObject 8.a / SerializeJSONArray 8.a: ? SerializeJSONProperty.
            let serialized = self.json_serialize_property(state, key, object)?;
            if is_array {
                self.json_separator(state, b'[')?;
                // 8.b. If stringP is undefined, append "null".
                match serialized {
                    Serialized::Undefined => self.append_str(&mut state.out, "null")?,
                    serialized => self.json_write(state, serialized)?,
                }
            } else {
                // 8.b. If stringP is not undefined: the member is QuoteJSONString(P), ":", a
                // space if there is a gap, and stringP.
                if let Serialized::Undefined = serialized {
                    continue;
                }
                self.json_separator(state, b'{')?;
                self.json_quote_key(state, key)?;
                self.append_str(&mut state.out, ":")?;
                if !state.gap.is_empty() {
                    self.append_str(&mut state.out, " ")?;
                }
                self.json_write(state, serialized)?;
            }
        }
        Ok(())
    }

    /// What comes before a member of the innermost frame: `open` before the first, a comma
    /// before the others; with a gap, a line feed and state.[[Indent]] (steps 10.b of the
    /// clauses, the separator and the start of final).
    fn json_separator(&mut self, state: &mut State<'_>, open: u8) -> Completion<()> {
        let Some(frame) = state.frames.last_mut() else {
            return Err(self.fault());
        };
        let first = !frame.written;
        frame.written = true;
        self.append_units(
            &mut state.out,
            &[u16::from(if first { open } else { b',' })],
        )?;
        if !state.gap.is_empty() {
            self.append_str(&mut state.out, "\n")?;
            let indent = core::mem::take(&mut state.indent);
            let appended = self.append_units(&mut state.out, &indent);
            state.indent = indent;
            appended?;
        }
        Ok(())
    }

    /// The end of the innermost frame: "{}" or "[]" when it wrote no member; else, with a gap,
    /// a line feed and stepBack, and the closing bracket. Then the last element of
    /// state.[[Stack]] is removed and state.[[Indent]] is set to stepBack.
    fn json_close(&mut self, state: &mut State<'_>) -> Completion<()> {
        let Some(frame) = state.frames.pop() else {
            return Err(self.fault());
        };
        let close = match frame.members {
            Members::Object { .. } => b'}',
            Members::Array { .. } => b']',
        };
        state.indent.truncate(frame.step_back);
        if frame.written {
            if !state.gap.is_empty() {
                self.append_str(&mut state.out, "\n")?;
                let indent = core::mem::take(&mut state.indent);
                let appended = self.append_units(&mut state.out, &indent);
                state.indent = indent;
                appended?;
            }
            self.append_units(&mut state.out, &[u16::from(close)])?;
        } else {
            let open = if close == b'}' { b'{' } else { b'[' };
            self.append_units(&mut state.out, &[u16::from(open), u16::from(close)])?;
        }
        state.on_stack.remove(&identity(frame.object));
        self.release(frame.mark);
        self.heap.accounting.credit(frame.charged);
        state.charged = state.charged.saturating_sub(frame.charged);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::Termination;
    use crate::builtins::string::transform::tests::{Answer, FEW_STEPS, LONG, with_long};
    use alloc::string::ToString;
    use alloc::vec;

    /// `#sec-json.stringify`, step 8: a String space gives its first 10 code units, and no
    /// more of it is read.
    #[test]
    fn a_string_gap_copies_ten_units() {
        let mut counting = with_long(0x67);
        let (answer, longest) = counting.counted(FEW_STEPS, "JSON.stringify([1], null, s)");
        assert_eq!(answer, Answer::Normal("[\ngggggggggg1\n]".to_string()));
        assert!(longest <= 10, "{longest}");
    }

    /// QuoteJSONString (`#sec-quotejsonstring`) reads a String in ranges of at most 256 code
    /// units and appends each before it reads the next: with few steps left the entry ends at
    /// the step budget with no longer copy made.
    #[test]
    fn quoting_reads_a_string_in_ranges() {
        let mut counting = with_long(0x61);
        let (answer, longest) = counting.counted(FEW_STEPS, "JSON.stringify(s)");
        assert_eq!(answer, Answer::Terminated(Termination::StepBudget));
        assert!(longest <= 256, "{longest}");
    }

    /// SerializeJSONObject (`#sec-serializejsonobject`), step 8.b: the key of a member is
    /// quoted as a String value is, in ranges.
    #[test]
    fn quoting_a_key_reads_it_in_ranges() {
        let mut counting = with_long(0x61);
        assert_eq!(
            counting.eval("var o = {}; o[s] = 1; 0"),
            Answer::Normal("Number(0.0)".to_string())
        );
        let (answer, longest) = counting.counted(FEW_STEPS, "JSON.stringify(o)");
        assert_eq!(answer, Answer::Terminated(Termination::StepBudget));
        assert!(longest <= 256, "{longest}");
    }

    /// SerializeJSONProperty (`#sec-serializejsonproperty`), step 4.a: the text of an object
    /// with [[IsRawJSON]] is appended by its steps before it is copied.
    #[test]
    fn a_raw_text_is_charged_before_its_copy() {
        let mut counting = with_long(0x61);
        let mut literal = vec![0x61; LONG];
        literal[0] = 0x22;
        literal[LONG - 1] = 0x22;
        counting.string("t", &literal);
        assert_eq!(
            counting.eval("var r = JSON.rawJSON(t); 0"),
            Answer::Normal("Number(0.0)".to_string())
        );
        assert_eq!(
            counting.counted(FEW_STEPS, "JSON.stringify(r)"),
            (Answer::Terminated(Termination::StepBudget), 0)
        );
    }

    /// `docs/design/engine.md`, "Memory budget": the result builder of `JSON.stringify`
    /// (`#sec-json.stringify`), its frames, its indent and its property list are charged by
    /// their capacity while it calls `toJSON`. Four levels, each holding the quoted String of
    /// 65,536 code units, do not fit a room of three such copies and end in
    /// `Terminated(MemoryBudget)`; with a property list of 64 Strings and a `space` of 4 no
    /// capacity is uncovered at any entry into script.
    #[test]
    fn stringify_holds_charged_buffers_across_a_call_of_to_json() {
        use crate::object::string::across::Across;
        use crate::object::string::copies;
        let mut across = Across::with_steps(4_000_000);
        across.string("s", &vec![0x61; 65_536]);
        let script = across.parse(
            "var d = 0; var o = { toJSON: function () { if (++d < 4) JSON.stringify([s, o]); \
             return 1; } }; JSON.stringify([s, o]).length",
        );
        across.room(3 * 131_072);
        assert_eq!(
            across.evaluate(&script),
            Answer::Terminated(Termination::MemoryBudget)
        );

        let mut across = Across::with_steps(4_000_000);
        let answer = across.run(
            "var d = 0; var l = []; for (var i = 0; i < 64; i++) l.push(\"k\" + i); var o = { \
             k0: 1, toJSON: function () { if (++d < 4) JSON.stringify([\"a\", o], l, 4); \
             return { k1: 2 }; } }; JSON.stringify([\"a\", o], l, 4).length",
        );
        assert_eq!(answer, Answer::Normal("Number(40.0)".to_string()));
        assert_eq!(copies::uncovered_at_entry(), 0);
    }
}
