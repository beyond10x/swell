//! The instructions of computed property names, of spread properties and of rest properties
//! (`swell_compiler::bytecode`, "literals" and "object patterns"), each in a function of its own
//! that one arm of `interp/dispatch.rs` calls, so the frame of the instruction loop stays as it
//! was. Written by `story:computed-names-and-object-spread`.
//!
//! A ComputedPropertyName is converted by ToPropertyKey (`#sec-topropertykey`) at once after its
//! expression; the property defined under it, and an anonymous function definition that is its
//! value, take their name from the key at run time (SetFunctionName, `#sec-setfunctionname`). A
//! spread property and a rest property copy the own enumerable properties of a value with
//! CopyDataProperties (`#sec-copydataproperties`): the keys come from [[OwnPropertyKeys]],
//! charged and held as `Object.assign` holds them ([`Agent::held_own_keys`]), and each key is
//! charged a step, one more per excluded name it is compared with. The excluded names of a rest
//! property are the keys its pattern read before it, charged as a list of keys while it copies.

use super::dispatch::Flow;
use crate::builtins::string::StringBuilder;
use crate::host::Host;
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::object::KeyList;
use crate::object::ordinary::OrdinaryData;
use crate::object::property::{Descriptor, Key};
use crate::realm::intrinsics::Intrinsic;
use crate::value::{ObjectRef, Value};

/// A count of the operand stack, as an index.
fn count(n: u32) -> usize {
    usize::try_from(n).unwrap_or(usize::MAX)
}

impl<H: Host> Agent<H> {
    /// `ToPropertyKey`: the value on top, rooted there while its `toString` or `valueOf` runs,
    /// replaced by its property key as a String or a Symbol.
    #[inline(never)]
    pub(super) fn computed_key_instruction(&mut self) -> Completion<Flow> {
        let value = self.peek_value(0)?;
        let key = self.to_property_key(value)?;
        let key = self.key_value(key)?;
        self.drop_entries(1)?;
        self.push_value(key)?;
        Ok(Flow::Next)
    }

    /// The key of the property key value `depth` entries under the top, which
    /// [`Agent::computed_key_instruction`] made: no code runs.
    fn key_at(&mut self, depth: usize) -> Completion<Key> {
        let value = self.peek_value(depth)?;
        self.to_property_key(value)
    }

    /// `DefineComputedField`: CreateDataPropertyOrThrow of the literal with the key and the value
    /// under it.
    #[inline(never)]
    pub(super) fn define_computed_field(&mut self) -> Completion<Flow> {
        let object = self.peek_value(2)?;
        let object = self.expect_object(object)?;
        let key = self.key_at(1)?;
        let value = self.peek_value(0)?;
        self.create_data_property_or_throw(object, key, value)?;
        self.drop_entries(2)?;
        Ok(Flow::Next)
    }

    /// `DefineComputedGetter` (`getter` true), `DefineComputedSetter` and `DefineComputedMethod`
    /// (`method` true): SetFunctionName with the key (and the prefix of an accessor), MakeMethod,
    /// then the property, enumerable and configurable (`#sec-runtime-semantics-definemethod`,
    /// `#sec-definemethodproperty`, `#sec-runtime-semantics-methoddefinitionevaluation`).
    #[inline(never)]
    pub(super) fn define_computed_function(
        &mut self,
        getter: bool,
        method: bool,
    ) -> Completion<Flow> {
        let object = self.peek_value(2)?;
        let object = self.expect_object(object)?;
        let key = self.key_at(1)?;
        let function = self.peek_value(0)?;
        let function = self.expect_object(function)?;
        let prefix = match (method, getter) {
            (true, _) => None,
            (false, true) => Some("get"),
            (false, false) => Some("set"),
        };
        self.set_function_name(function, key, prefix)?;
        self.make_method(function, object)?;
        let descriptor = match prefix {
            None => Descriptor {
                value: Some(Value::Object(function)),
                writable: Some(true),
                enumerable: Some(true),
                configurable: Some(true),
                ..Descriptor::default()
            },
            Some(_) if getter => Descriptor {
                get: Some(Some(function)),
                enumerable: Some(true),
                configurable: Some(true),
                ..Descriptor::default()
            },
            Some(_) => Descriptor {
                set: Some(Some(function)),
                enumerable: Some(true),
                configurable: Some(true),
                ..Descriptor::default()
            },
        };
        self.define_property_or_throw(object, key, descriptor)?;
        self.drop_entries(2)?;
        Ok(Flow::Next)
    }

    /// `SetFunctionName`: the function on top named after the key under it; both stay.
    #[inline(never)]
    pub(super) fn set_function_name_instruction(&mut self) -> Completion<Flow> {
        let function = self.peek_value(0)?;
        let function = self.expect_object(function)?;
        let key = self.key_at(1)?;
        self.set_function_name(function, key, None)?;
        Ok(Flow::Next)
    }

    /// SetFunctionName ( F, name [ , prefix ] ) (`#sec-setfunctionname`): the name of a Symbol
    /// key is its description in brackets, or the empty String when it has none; a prefix comes
    /// before it with a space. `function`, made by its closure, has its `name` property already,
    /// configurable, which is defined again with the name.
    pub(super) fn set_function_name(
        &mut self,
        function: ObjectRef,
        key: Key,
        prefix: Option<&str>,
    ) -> Completion<()> {
        let name = match (key, prefix) {
            (Key::String(string), None) => string,
            (Key::Index(_), None) => match self.key_value(key)? {
                Value::String(string) => string,
                _ => return Err(self.fault()),
            },
            _ => {
                let mut builder = StringBuilder::new();
                if let Some(prefix) = prefix {
                    self.append_str(&mut builder, prefix)?;
                    self.append_str(&mut builder, " ")?;
                }
                self.append_key_name(&mut builder, key)?;
                self.finish_string(builder)?
            }
        };
        let name_key = self.name_key("name")?;
        let descriptor = Descriptor {
            value: Some(Value::String(name)),
            writable: Some(false),
            enumerable: Some(false),
            configurable: Some(true),
            ..Descriptor::default()
        };
        self.define_property_or_throw(function, name_key, descriptor)
    }

    /// Steps 2 and 3 of SetFunctionName: the name `key` gives, appended to `builder`.
    fn append_key_name(&mut self, builder: &mut StringBuilder, key: Key) -> Completion<()> {
        let Key::Symbol(symbol) = key else {
            return match self.key_value(key)? {
                Value::String(string) => self.append_string(builder, string),
                _ => Err(self.fault()),
            };
        };
        let description = match self.heap.symbols.get(symbol.handle) {
            Ok(record) => record.description,
            Err(_) => return Err(self.fault()),
        };
        if let Some(description) = description {
            self.append_str(builder, "[")?;
            self.append_string(builder, description)?;
            self.append_str(builder, "]")?;
        }
        Ok(())
    }

    /// `CopyDataProperties`: the own enumerable properties of the value on top copied into the
    /// literal under it; the value is dropped.
    #[inline(never)]
    pub(super) fn copy_data_properties_instruction(&mut self) -> Completion<Flow> {
        let target = self.peek_value(1)?;
        let target = self.expect_object(target)?;
        let source = self.peek_value(0)?;
        self.copy_data_properties(target, source, &[])?;
        self.drop_entries(1)?;
        Ok(Flow::Next)
    }

    /// `PatternGetKey { depth, kept }`: GetV of the value `depth + 1 + kept` entries under the
    /// top with the key `depth` entries under it, pushed.
    #[inline(never)]
    pub(super) fn pattern_get_key(&mut self, depth: u32, kept: u32) -> Completion<Flow> {
        let depth = count(depth);
        let key = self.key_at(depth)?;
        let base = depth.saturating_add(1).saturating_add(count(kept));
        let base = self.peek_value(base)?;
        let value = self.get_value(base, key)?;
        self.push_value(value)?;
        Ok(Flow::Next)
    }

    /// `ObjectRest { depth, kept }`: the `kept` keys `depth` entries under the top and above the
    /// value, made a list of excluded names, charged, their handles held; then a new ordinary
    /// object, pushed, which CopyDataProperties fills from the value
    /// (`#sec-destructuring-binding-patterns-runtime-semantics-restbindinginitialization`,
    /// `#sec-runtime-semantics-restdestructuringassignmentevaluation`).
    #[inline(never)]
    pub(super) fn object_rest(&mut self, depth: u32, kept: u32) -> Completion<Flow> {
        let (depth, kept) = (count(depth), count(kept));
        let mark = self.kept.len();
        let mut room = self.kept_room();
        let done = match self.excluded_keys(depth, kept, &mut room) {
            Ok(excluded) => {
                let made = self.make_rest(depth.saturating_add(kept), &excluded);
                self.drop_key_list(excluded);
                made
            }
            Err(abrupt) => Err(abrupt),
        };
        self.end_kept_room(mark, &room);
        done.map(|()| Flow::Next)
    }

    /// The list of the `kept` keys `depth` entries under the top, charged first; the handles
    /// its keys hold are kept in room charged to `room`.
    fn excluded_keys(
        &mut self,
        depth: usize,
        kept: usize,
        room: &mut crate::object::builtin_function::KeptRoom,
    ) -> Completion<KeyList> {
        let mut list = self.new_key_list(kept)?;
        let filled = self.reserve_kept(room, kept).and_then(|()| {
            for at in 0..kept {
                let key = self.key_at(depth.saturating_add(at))?;
                self.keep_key(key);
                list.keys.push(key);
            }
            Ok(())
        });
        match filled {
            Ok(()) => Ok(list),
            Err(abrupt) => {
                self.drop_key_list(list);
                Err(abrupt)
            }
        }
    }

    /// The rest object of the value `depth` entries under the top, pushed, then filled.
    fn make_rest(&mut self, depth: usize, excluded: &[Key]) -> Completion<()> {
        let source = self.peek_value(depth)?;
        let prototype = self.current_intrinsic(Intrinsic::ObjectPrototype)?;
        let rest = self.ordinary_object_create(Some(prototype), OrdinaryData::Plain)?;
        self.push_value(Value::Object(rest))?;
        self.copy_data_properties(rest, source, excluded)
    }

    /// CopyDataProperties ( target, source, excludedItems ) (`#sec-copydataproperties`):
    /// nothing for `undefined` and `null`, and nothing for a Symbol or a BigInt, whose wrapper
    /// objects have no own property; otherwise the keys of ToObject of `source` by
    /// [[OwnPropertyKeys]], charged and held, each charged a step and one per excluded name.
    fn copy_data_properties(
        &mut self,
        target: ObjectRef,
        source: Value,
        excluded: &[Key],
    ) -> Completion<()> {
        // 1. If source is either undefined or null, return unused.
        if matches!(
            source,
            Value::Undefined | Value::Null | Value::Symbol(_) | Value::BigInt(_)
        ) {
            return Ok(());
        }
        // 2. Let from be ! ToObject(source).
        let from = self.to_object(source)?;
        let mark = self.keep(Value::Object(from));
        // 3. Let keys be ? from.[[OwnPropertyKeys]]().
        let mut room = self.kept_room();
        let copied = match self.held_own_keys(from, &mut room) {
            Ok(keys) => {
                let copied = self.copy_keys(target, from, &keys, excluded);
                self.drop_key_list(keys);
                copied
            }
            Err(abrupt) => Err(abrupt),
        };
        self.end_kept_room(mark, &room);
        copied
    }

    /// Step 4 of CopyDataProperties: for each element nextKey of keys.
    fn copy_keys(
        &mut self,
        target: ObjectRef,
        from: ObjectRef,
        keys: &[Key],
        excluded: &[Key],
    ) -> Completion<()> {
        let steps = u64::try_from(excluded.len())
            .unwrap_or(u64::MAX)
            .saturating_add(1);
        for &next_key in keys {
            self.charge_steps(steps)?;
            // a, b. If excluded is false: SameValue of nextKey and an element of excludedItems.
            if excluded.contains(&next_key) {
                continue;
            }
            // i. Let desc be ? from.[[GetOwnProperty]](nextKey).
            // ii. If desc is not undefined and desc.[[Enumerable]] is true, then
            if !self
                .get_own_property(from, next_key)?
                .is_some_and(|property| property.enumerable)
            {
                continue;
            }
            // 1. Let propValue be ? Get(from, nextKey).
            let value = self.get(from, next_key)?;
            // 2. Perform ! CreateDataPropertyOrThrow(target, nextKey, propValue): target is an
            // ordinary object no code holds yet, so no code runs.
            self.create_data_property_or_throw(target, next_key, value)?;
        }
        Ok(())
    }
}
