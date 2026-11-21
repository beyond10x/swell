//! The JSON object (ECMA-262 25.5, `#sec-json-object`): `JSON.stringify` (`#sec-json.stringify`)
//! with SerializeJSONProperty (`#sec-serializejsonproperty`), QuoteJSONString
//! (`#sec-quotejsonstring`), UnicodeEscape (`#sec-unicodeescape`), SerializeJSONObject
//! (`#sec-serializejsonobject`) and SerializeJSONArray (`#sec-serializejsonarray`), and
//! `JSON[@@toStringTag]` (`#sec-json-%symbol.tostringtag%`). Written by `story:harness-builtins`;
//! `JSON.parse` is slice 3.
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

use std::collections::BTreeSet;

use crate::builtins::object::argument;
use crate::builtins::string::{StringBuilder, units_of};
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::host::{ErrorKind, Host, Termination};
use crate::object::ObjectKind;
use crate::object::builtin_function::BuiltinId;
use crate::object::ordinary::OrdinaryData;
use crate::object::property::Key;
use crate::realm::intrinsics::{Intrinsic, Making};
use crate::value::{ObjectRef, StringRef, Value};

/// The builtins of this module, by their steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum JsonBuiltin {
    /// `JSON.stringify ( value [ , replacer [ , space ] ] )` (25.5.4).
    Stringify,
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
struct State {
    replacer: Option<Value>,
    /// The property list, each String with its code units.
    property_list: Option<Vec<(StringRef, Vec<u16>)>>,
    gap: Vec<u16>,
    indent: Vec<u16>,
    /// The handles of the objects on state.[[Stack]], for the cycle check.
    on_stack: BTreeSet<(u32, u32, u32)>,
    frames: Vec<Frame>,
    out: StringBuilder,
    /// The bytes charged for the frames and the property list, credited when they go.
    charged: u64,
}

/// What SerializeJSONProperty of a value gives, before anything is written: undefined, a
/// String of JSON text, or an object or array to open.
enum Serialized {
    Undefined,
    Text(Vec<u16>),
    Quoted(Vec<u16>),
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
    /// `stringify` and @@toStringTag "JSON" (25.5.5).
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
                let mark = self.kept.len();
                let mut state = State {
                    replacer: None,
                    property_list: None,
                    gap: Vec::new(),
                    indent: Vec::new(),
                    on_stack: BTreeSet::new(),
                    frames: Vec::new(),
                    out: StringBuilder::new(),
                    charged: 0,
                };
                let result = self.json_stringify(&mut state, args);
                self.heap.accounting.credit(state.charged);
                self.release(mark);
                result
            }
        }
    }

    /// Charge `bytes` of the walk to the memory budget.
    fn json_charge(&mut self, state: &mut State, bytes: u64) -> Completion<()> {
        if self.heap.accounting.charge(bytes).is_err() {
            return Err(self.terminate(Termination::MemoryBudget));
        }
        state.charged = state.charged.saturating_add(bytes);
        Ok(())
    }

    /// `JSON.stringify ( value [ , replacer [ , space ] ] )` (`#sec-json.stringify`). The
    /// arguments are held by this builtin's activation.
    fn json_stringify(&mut self, state: &mut State, args: &[Value]) -> Completion<Value> {
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
        let out = std::mem::take(&mut state.out);
        Ok(Value::String(self.finish_string(out)?))
    }

    /// Step 5.b of `JSON.stringify`: the property list of the array `replacer`, each item once,
    /// in order. A step per element and per comparison with an item already listed.
    fn json_property_list(&mut self, state: &mut State, replacer: ObjectRef) -> Completion<()> {
        // 5.b.i. Set propertyList to a new empty List.
        let mut list: Vec<(StringRef, Vec<u16>)> = Vec::new();
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
                let units = self.string_units(item)?;
                let mut listed = false;
                for (_, other) in &list {
                    self.charge_steps(1)?;
                    if *other == units {
                        listed = true;
                        break;
                    }
                }
                if !listed {
                    let bytes = size_of::<(StringRef, Vec<u16>)>()
                        .saturating_add(units.len().saturating_mul(2))
                        .saturating_add(size_of::<Value>());
                    self.json_charge(state, u64::try_from(bytes).unwrap_or(u64::MAX))?;
                    if list.try_reserve(1).is_err() {
                        return Err(self.terminate(Termination::MemoryBudget));
                    }
                    self.keep(Value::String(item));
                    list.push((item, units));
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
            // 8. If space is a String: its first 10 code units.
            Value::String(string) => {
                let mut units = self.string_units(string)?;
                units.truncate(GAP_MAX);
                units
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

    /// The code units of `key`, which QuoteJSONString quotes for a member of an object.
    fn json_key_units(&mut self, key: JsonKey) -> Completion<Vec<u16>> {
        match key {
            JsonKey::Key(Key::Index(index)) => Ok(units_of(&index.to_string())),
            JsonKey::Key(Key::String(string)) | JsonKey::String(string) => {
                self.string_units(string)
            }
            JsonKey::Key(Key::Symbol(_)) => Err(self.fault()),
            JsonKey::Index(index) => Ok(units_of(&swell_num::to_string::number_to_string(index))),
        }
    }

    /// SerializeJSONProperty (`#sec-serializejsonproperty`), steps 1 to 11: the value of `key`
    /// of `holder` after toJSON, the replacer and the unwrapping of a Boolean, Number or String
    /// object; an object that is not callable is opened by the caller (step 12). Each value is
    /// held across the code that follows by the activation it is passed to.
    fn json_serialize_property(
        &mut self,
        state: &State,
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
        // 4. If value is an Object: no [[IsRawJSON]] before `JSON.rawJSON`; a Number object
        // gives ? ToNumber(value), a String object ? ToString(value), a Boolean object its
        // [[BooleanData]]; no [[BigIntData]] before slice 12.
        if let Value::Object(object) = value {
            enum Wrapped {
                Number,
                String,
                Boolean(bool),
            }
            let wrapped = match &self.engine_object(object)?.kind {
                ObjectKind::String(_) => Some(Wrapped::String),
                ObjectKind::Ordinary(ordinary) => match ordinary.data {
                    OrdinaryData::Number(_) => Some(Wrapped::Number),
                    OrdinaryData::Boolean(boolean) => Some(Wrapped::Boolean(boolean)),
                    _ => None,
                },
                _ => None,
            };
            value = match wrapped {
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
            Value::String(string) => Serialized::Quoted(self.string_units(string)?),
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

    /// Write a serialized value that is not undefined: its text, its quoted String, or the
    /// opening of its object or array.
    fn json_write(&mut self, state: &mut State, serialized: Serialized) -> Completion<()> {
        match serialized {
            Serialized::Undefined => Err(self.fault()),
            Serialized::Text(units) => self.append_units(&mut state.out, &units),
            Serialized::Quoted(units) => self.json_quote(state, &units),
            Serialized::Open(object) => self.json_open(state, object),
        }
    }

    /// QuoteJSONString (`#sec-quotejsonstring`) of `units`, appended to the result.
    fn json_quote(&mut self, state: &mut State, units: &[u16]) -> Completion<()> {
        let mut chunk: Vec<u16> = Vec::new();
        // 1. Let product be the String value consisting solely of the code unit 0x0022.
        chunk.push(u16::from(b'"'));
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
                self.append_units(&mut state.out, &chunk)?;
                chunk.clear();
            }
        }
        // 3. Append the code unit 0x0022 (QUOTATION MARK).
        chunk.push(u16::from(b'"'));
        self.append_units(&mut state.out, &chunk)
    }

    /// SerializeJSONObject (`#sec-serializejsonobject`) and SerializeJSONArray
    /// (`#sec-serializejsonarray`), steps 1 to 4 (to 6 of an array): the cycle check, the
    /// stack, the indent and the keys or length; the frame is charged and opened.
    fn json_open(&mut self, state: &mut State, object: ObjectRef) -> Completion<()> {
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
        let members = if self.is_array(Value::Object(object))? {
            // SerializeJSONArray, 6. Let len be ? LengthOfArrayLike(value).
            let length = self.length_of_array_like(object)?;
            Members::Array { length, next: 0.0 }
        } else if state.property_list.is_some() {
            // SerializeJSONObject, 5. Let K be state.[[PropertyList]].
            Members::Object {
                keys: None,
                next: 0,
            }
        } else {
            // SerializeJSONObject, 6. Let K be ? EnumerableOwnProperties(value, key).
            let keys = self.json_enumerable_own_keys(object)?;
            Members::Object {
                keys: Some(keys),
                next: 0,
            }
        };
        let key_count = match &members {
            Members::Object {
                keys: Some(keys), ..
            } => keys.len(),
            _ => 0,
        };
        let bytes = size_of::<Frame>()
            .saturating_add(key_count.saturating_mul(size_of::<Key>() + size_of::<Value>()))
            .saturating_add(size_of::<Value>())
            .saturating_add(size_of::<(u32, u32, u32)>() * 4)
            .saturating_add(state.gap.len() * 2);
        let charged = u64::try_from(bytes).unwrap_or(u64::MAX);
        self.json_charge(state, charged)?;
        if state.frames.try_reserve(1).is_err()
            || state.indent.try_reserve(state.gap.len()).is_err()
        {
            return Err(self.terminate(Termination::MemoryBudget));
        }
        state.indent.extend_from_slice(&state.gap);
        state.on_stack.insert(identity(object));
        state.frames.push(Frame {
            object,
            members,
            written: false,
            step_back,
            mark,
            charged,
        });
        Ok(())
    }

    /// EnumerableOwnProperties (`#sec-enumerableownproperties`) of kind key: the String keys
    /// of `object` whose own property is enumerable, each kept. A step per key.
    fn json_enumerable_own_keys(&mut self, object: ObjectRef) -> Completion<Vec<Key>> {
        // 1. Let ownKeys be ? obj.[[OwnPropertyKeys]]().
        let own_keys = self.own_property_keys(object)?;
        let mut keys = Vec::new();
        if keys.try_reserve(own_keys.len()).is_err() {
            return Err(self.terminate(Termination::MemoryBudget));
        }
        // 3. For each element key of ownKeys that is a String.
        for key in own_keys {
            self.charge_steps(1)?;
            if matches!(key, Key::Symbol(_)) {
                continue;
            }
            // 3.a.i, ii. Let desc be ? obj.[[GetOwnProperty]](key); keep the key if it is
            // enumerable.
            if self
                .get_own_property(object, key)?
                .is_some_and(|property| property.enumerable)
            {
                self.keep_key(key);
                keys.push(key);
            }
        }
        Ok(keys)
    }

    /// The members of the open frames, the innermost first, until none is open: a step per
    /// member.
    fn json_walk(&mut self, state: &mut State) -> Completion<()> {
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
                            .map(|(string, _)| JsonKey::String(*string)),
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
                let units = self.json_key_units(key)?;
                self.json_quote(state, &units)?;
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
    fn json_separator(&mut self, state: &mut State, open: u8) -> Completion<()> {
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
            let indent = std::mem::take(&mut state.indent);
            let appended = self.append_units(&mut state.out, &indent);
            state.indent = indent;
            appended?;
        }
        Ok(())
    }

    /// The end of the innermost frame: "{}" or "[]" when it wrote no member; else, with a gap,
    /// a line feed and stepBack, and the closing bracket. Then the last element of
    /// state.[[Stack]] is removed and state.[[Indent]] is set to stepBack.
    fn json_close(&mut self, state: &mut State) -> Completion<()> {
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
                let indent = std::mem::take(&mut state.indent);
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
