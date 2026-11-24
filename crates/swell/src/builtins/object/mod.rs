//! Object objects (ECMA-262 20.1, `#sec-object-objects`): the `Object` constructor
//! (`#sec-object-value`), `Object.defineProperty` (`#sec-object.defineproperty`),
//! `Object.getOwnPropertyDescriptor` (`#sec-object.getownpropertydescriptor`),
//! `Object.getOwnPropertyNames` (`#sec-object.getownpropertynames`),
//! `Object.getOwnPropertySymbols` (`#sec-object.getownpropertysymbols`, written by
//! `story:symbol-constructor-and-well-known-symbols`), `Object.getPrototypeOf`
//! (`#sec-object.getprototypeof`), `Object.fromEntries` (`#sec-object.fromentries`) and
//! `Object.groupBy` (`#sec-object.groupby`), whose steps are in `from_entries.rs`
//! (`story:array-from-and-object-from-entries`), and `Object.prototype.constructor`
//! (`#sec-object.prototype.constructor`), `.hasOwnProperty`
//! (`#sec-object.prototype.hasownproperty`), `.propertyIsEnumerable`
//! (`#sec-object.prototype.propertyisenumerable`) and `.toString`
//! (`#sec-object.prototype.tostring`); with ToPropertyDescriptor (`#sec-topropertydescriptor`),
//! FromPropertyDescriptor (`#sec-frompropertydescriptor`), GetOwnPropertyKeys
//! (`#sec-getownpropertykeys`) and CreateArrayFromList (`#sec-createarrayfromlist`). Written by
//! `story:harness-builtins`; the other members are slice 3, in the group modules beside this
//! file (`story:builtin-group-modules`): `integrity.rs`
//! (`story:object-integrity-and-prototype-methods`) and `enumeration.rs`
//! (`story:object-enumeration-and-copy-methods`). Their keys follow the keys of this file on
//! `%Object%` and `%Object.prototype%`, in that order of the groups.
//!
//! A value a builtin holds across a step that may run code (a getter, a toString) is kept on
//! the agent's kept stack, which the root site of `object/builtin_function.rs` reports
//! (decision 4); a key's atom is kept the same way, since atoms are held weakly.

pub(crate) mod enumeration;
pub(crate) mod from_entries;
pub(crate) mod integrity;

use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::host::{ErrorKind, Host, Termination};
use crate::object::builtin_function::BuiltinId;
use crate::object::ordinary::OrdinaryData;
use crate::object::property::{Descriptor, Key, Property, Slot};
use crate::object::{KeyList, ObjectKind};
use crate::realm::intrinsics::{Intrinsic, Making};
use crate::value::{ObjectRef, Value};

/// The builtins of this module, by their steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ObjectBuiltin {
    /// `Object ( value )` (20.1.1.1).
    Constructor,
    /// `Object.defineProperty ( O, P, Attributes )` (20.1.2.4).
    DefineProperty,
    /// `Object.getOwnPropertyDescriptor ( O, P )` (20.1.2.8).
    GetOwnPropertyDescriptor,
    /// `Object.getOwnPropertyNames ( O )` (20.1.2.10).
    GetOwnPropertyNames,
    /// `Object.getOwnPropertySymbols ( O )` (20.1.2.11).
    GetOwnPropertySymbols,
    /// `Object.getPrototypeOf ( O )` (20.1.2.12).
    GetPrototypeOf,
    /// `Object.fromEntries ( iterable )` (20.1.2.7) and `Object.groupBy ( items, callback )`
    /// (20.1.2.13), their steps in `from_entries.rs` (`story:array-from-and-object-from-entries`).
    FromEntries,
    GroupBy,
    /// `Object.prototype.hasOwnProperty ( V )` (20.1.3.2).
    HasOwnProperty,
    /// `Object.prototype.propertyIsEnumerable ( V )` (20.1.3.4).
    PropertyIsEnumerable,
    /// `Object.prototype.toString ( )` (20.1.3.6).
    ToString,
    /// A builtin of the group `integrity.rs`.
    Integrity(integrity::IntegrityBuiltin),
    /// A builtin of the group `enumeration.rs`.
    Enumeration(enumeration::EnumerationBuiltin),
}

/// The argument at `index`, undefined when it is not present (clause 18: "If a built-in
/// function is called with fewer arguments ... the missing arguments [are] the undefined value").
pub(crate) fn argument(args: &[Value], index: usize) -> Value {
    args.get(index).copied().unwrap_or(Value::Undefined)
}

impl<H: Host> Agent<H> {
    /// `%Object%` (20.1.1) with its functions and the methods of `%Object.prototype%`.
    pub(crate) fn create_object_constructor(&mut self, making: Making) -> Completion<ObjectRef> {
        let constructor = self.create_builtin_constructor(
            making,
            "Object",
            1,
            BuiltinId::Object(ObjectBuiltin::Constructor),
            making.object_prototype,
        )?;
        for (name, length, id) in [
            ("defineProperty", 3, ObjectBuiltin::DefineProperty),
            (
                "getOwnPropertyDescriptor",
                2,
                ObjectBuiltin::GetOwnPropertyDescriptor,
            ),
            ("getOwnPropertyNames", 1, ObjectBuiltin::GetOwnPropertyNames),
            (
                "getOwnPropertySymbols",
                1,
                ObjectBuiltin::GetOwnPropertySymbols,
            ),
            ("getPrototypeOf", 1, ObjectBuiltin::GetPrototypeOf),
            ("fromEntries", 1, ObjectBuiltin::FromEntries),
            ("groupBy", 2, ObjectBuiltin::GroupBy),
        ] {
            self.define_builtin_method(making, constructor, name, length, BuiltinId::Object(id))?;
        }
        for (name, length, id) in [
            ("hasOwnProperty", 1, ObjectBuiltin::HasOwnProperty),
            (
                "propertyIsEnumerable",
                1,
                ObjectBuiltin::PropertyIsEnumerable,
            ),
            ("toString", 0, ObjectBuiltin::ToString),
        ] {
            self.define_builtin_method(
                making,
                making.object_prototype,
                name,
                length,
                BuiltinId::Object(id),
            )?;
        }
        for (object, on_prototype) in [(constructor, false), (making.object_prototype, true)] {
            self.define_group_methods(
                making,
                object,
                if on_prototype {
                    integrity::PROTOTYPE_METHODS
                } else {
                    integrity::CONSTRUCTOR_METHODS
                },
                |id| BuiltinId::Object(ObjectBuiltin::Integrity(id)),
            )?;
            self.define_group_methods(
                making,
                object,
                if on_prototype {
                    enumeration::PROTOTYPE_METHODS
                } else {
                    enumeration::CONSTRUCTOR_METHODS
                },
                |id| BuiltinId::Object(ObjectBuiltin::Enumeration(id)),
            )?;
        }
        let prototype = making.object_prototype;
        self.define_symbol_data(making, prototype, &integrity::prototype_symbol_data())?;
        self.define_symbol_data(making, prototype, &enumeration::prototype_symbol_data())?;
        Ok(constructor)
    }

    /// The steps of the builtins of this module.
    pub(crate) fn run_object_builtin(
        &mut self,
        id: ObjectBuiltin,
        function: ObjectRef,
        this: Value,
        args: &[Value],
        new_target: Option<ObjectRef>,
    ) -> Completion<Value> {
        match id {
            ObjectBuiltin::Constructor => self.object_constructor(function, args, new_target),
            ObjectBuiltin::DefineProperty => self.object_define_property(args),
            ObjectBuiltin::GetOwnPropertyDescriptor => {
                self.object_get_own_property_descriptor(args)
            }
            ObjectBuiltin::GetOwnPropertyNames => self.object_get_own_property_names(args),
            ObjectBuiltin::GetOwnPropertySymbols => self.object_get_own_property_symbols(args),
            ObjectBuiltin::GetPrototypeOf => self.object_get_prototype_of(args),
            ObjectBuiltin::FromEntries => self.object_from_entries(args),
            ObjectBuiltin::GroupBy => self.object_group_by(args),
            ObjectBuiltin::HasOwnProperty => self.object_has_own_property(this, args),
            ObjectBuiltin::PropertyIsEnumerable => self.object_property_is_enumerable(this, args),
            ObjectBuiltin::ToString => self.object_to_string(this),
            ObjectBuiltin::Integrity(id) => {
                self.run_object_integrity_builtin(id, function, this, args, new_target)
            }
            ObjectBuiltin::Enumeration(id) => {
                self.run_object_enumeration_builtin(id, function, this, args, new_target)
            }
        }
    }

    /// Keep the handle a key holds, if any, until [`Agent::release`] of the mark this answers.
    pub(crate) fn keep_key(&mut self, key: Key) -> usize {
        match key {
            Key::Index(_) => self.kept.len(),
            Key::String(string) => self.keep(Value::String(string)),
            Key::Symbol(symbol) => self.keep(Value::Symbol(symbol)),
        }
    }

    /// `Object ( value )` (`#sec-object-value`).
    fn object_constructor(
        &mut self,
        function: ObjectRef,
        args: &[Value],
        new_target: Option<ObjectRef>,
    ) -> Completion<Value> {
        // 1. If NewTarget is neither undefined nor the active function object, then return ?
        // OrdinaryCreateFromConstructor(NewTarget, "%Object.prototype%").
        if let Some(new_target) = new_target
            && new_target != function
        {
            let made = self.ordinary_create_from_constructor(
                new_target,
                Intrinsic::ObjectPrototype,
                OrdinaryData::Plain,
            )?;
            return Ok(Value::Object(made));
        }
        // 2. If value is either undefined or null, return
        // OrdinaryObjectCreate(%Object.prototype%).
        let value = argument(args, 0);
        if matches!(value, Value::Undefined | Value::Null) {
            let prototype = self.current_intrinsic(Intrinsic::ObjectPrototype)?;
            let made = self.ordinary_object_create(Some(prototype), OrdinaryData::Plain)?;
            return Ok(Value::Object(made));
        }
        // 3. Return ! ToObject(value).
        Ok(Value::Object(self.to_object(value)?))
    }

    /// `Object.defineProperty ( O, P, Attributes )` (`#sec-object.defineproperty`).
    fn object_define_property(&mut self, args: &[Value]) -> Completion<Value> {
        // 1. If O is not an Object, throw a TypeError exception.
        let Value::Object(object) = argument(args, 0) else {
            return Err(self.throw_error(
                ErrorKind::TypeError,
                "Object.defineProperty called on a non-object",
            ));
        };
        // 2. Let key be ? ToPropertyKey(P).
        let key = self.to_property_key(argument(args, 1))?;
        // 3. Let desc be ? ToPropertyDescriptor(Attributes): it may run code, so the key's
        // atom is kept.
        let keep = self.keep_key(key);
        let descriptor = self.to_property_descriptor(argument(args, 2));
        let defined = descriptor.and_then(|descriptor| {
            // 4. Perform ? DefinePropertyOrThrow(O, key, desc).
            self.define_property_or_throw(object, key, descriptor)
        });
        self.release(keep);
        defined?;
        // 5. Return O.
        Ok(Value::Object(object))
    }

    /// ToPropertyDescriptor (`#sec-topropertydescriptor`). Each field is read by HasProperty
    /// and Get, which may run code; the values read are kept until the descriptor is made.
    #[expect(
        clippy::wrong_self_convention,
        reason = "the name is ECMA-262's; the operation calls into the agent, so it needs `&mut self`"
    )]
    pub(crate) fn to_property_descriptor(&mut self, value: Value) -> Completion<Descriptor> {
        // 1. If Obj is not an Object, throw a TypeError exception.
        let Value::Object(object) = value else {
            return Err(self.throw_error(
                ErrorKind::TypeError,
                "a property descriptor must be an object",
            ));
        };
        let mark = self.kept.len();
        let read = self.to_property_descriptor_fields(object);
        self.release(mark);
        read
    }

    /// The field `name` of the descriptor object `object`: `Some` of its value when
    /// HasProperty answers true, kept.
    fn descriptor_field(&mut self, object: ObjectRef, name: &str) -> Completion<Option<Value>> {
        let key = self.name_key(name)?;
        if !self.has_property(object, key)? {
            return Ok(None);
        }
        let key = self.name_key(name)?;
        let value = self.get(object, key)?;
        self.keep(value);
        Ok(Some(value))
    }

    /// Steps 2 to 16 of ToPropertyDescriptor.
    #[expect(
        clippy::wrong_self_convention,
        reason = "the name is ECMA-262's; the operation calls into the agent, so it needs `&mut self`"
    )]
    fn to_property_descriptor_fields(&mut self, object: ObjectRef) -> Completion<Descriptor> {
        let mut descriptor = Descriptor::default();
        // 3, 4. "enumerable".
        if let Some(value) = self.descriptor_field(object, "enumerable")? {
            descriptor.enumerable = Some(self.to_boolean(value)?);
        }
        // 5, 6. "configurable".
        if let Some(value) = self.descriptor_field(object, "configurable")? {
            descriptor.configurable = Some(self.to_boolean(value)?);
        }
        // 7, 8. "value".
        descriptor.value = self.descriptor_field(object, "value")?;
        // 9, 10. "writable".
        if let Some(value) = self.descriptor_field(object, "writable")? {
            descriptor.writable = Some(self.to_boolean(value)?);
        }
        // 11, 12. "get": callable or undefined, else a TypeError.
        if let Some(getter) = self.descriptor_field(object, "get")? {
            descriptor.get = Some(self.accessor_field(getter, "a getter")?);
        }
        // 13, 14. "set".
        if let Some(setter) = self.descriptor_field(object, "set")? {
            descriptor.set = Some(self.accessor_field(setter, "a setter")?);
        }
        // 15. A descriptor with [[Get]] or [[Set]] and [[Value]] or [[Writable]] is a TypeError.
        if descriptor.is_accessor() && descriptor.is_data() {
            return Err(self.throw_error(
                ErrorKind::TypeError,
                "a property descriptor cannot be both accessor and data",
            ));
        }
        Ok(descriptor)
    }

    /// A getter or setter field: "If IsCallable(getter) is false and getter is not
    /// undefined, throw a TypeError exception".
    fn accessor_field(&mut self, value: Value, what: &str) -> Completion<Option<ObjectRef>> {
        match value {
            Value::Undefined => Ok(None),
            Value::Object(function) if self.is_callable(value)? => Ok(Some(function)),
            _ => Err(self.throw_error(
                ErrorKind::TypeError,
                &format!("{what} must be a function or undefined"),
            )),
        }
    }

    /// FromPropertyDescriptor (`#sec-frompropertydescriptor`) of a property, complete as every
    /// property is: "value", "writable" or "get", "set", then "enumerable", "configurable".
    /// The new object is ordinary and extensible, so each CreateDataPropertyOrThrow succeeds
    /// and runs no code.
    #[expect(
        clippy::wrong_self_convention,
        reason = "the name is ECMA-262's; the operation allocates in the agent, so it needs `&mut self`"
    )]
    pub(crate) fn from_property_descriptor(
        &mut self,
        property: Option<Property>,
    ) -> Completion<Value> {
        // 1. If Desc is undefined, return undefined.
        let Some(property) = property else {
            return Ok(Value::Undefined);
        };
        // 2. Let obj be OrdinaryObjectCreate(%Object.prototype%).
        let prototype = self.current_intrinsic(Intrinsic::ObjectPrototype)?;
        let object = self.ordinary_object_create(Some(prototype), OrdinaryData::Plain)?;
        let accessor =
            |function: Option<ObjectRef>| function.map_or(Value::Undefined, Value::Object);
        let fields: [(&str, Value); 2] = match property.slot {
            Slot::Data { value, writable } => {
                [("value", value), ("writable", Value::Boolean(writable))]
            }
            Slot::Accessor { get, set } => [("get", accessor(get)), ("set", accessor(set))],
        };
        for (name, value) in fields.into_iter().chain([
            ("enumerable", Value::Boolean(property.enumerable)),
            ("configurable", Value::Boolean(property.configurable)),
        ]) {
            let key = self.name_key(name)?;
            if !self.create_data_property(object, key, value)? {
                return Err(self.fault());
            }
        }
        Ok(Value::Object(object))
    }

    /// `Object.getOwnPropertyDescriptor ( O, P )` (`#sec-object.getownpropertydescriptor`).
    fn object_get_own_property_descriptor(&mut self, args: &[Value]) -> Completion<Value> {
        // 1. Let obj be ? ToObject(O).
        let object = self.to_object(argument(args, 0))?;
        // 2. Let key be ? ToPropertyKey(P): it may run code, so obj, which may be new, is kept.
        let keep = self.keep(Value::Object(object));
        let key = self.to_property_key(argument(args, 1));
        self.release(keep);
        let key = key?;
        // 3. Let desc be ? obj.[[GetOwnProperty]](key). 4. Return FromPropertyDescriptor(desc).
        let property = self.get_own_property(object, key)?;
        self.from_property_descriptor(property)
    }

    /// `Object.getOwnPropertyNames ( O )` (`#sec-object.getownpropertynames`): CreateArrayFromList
    /// of GetOwnPropertyKeys(O, string).
    fn object_get_own_property_names(&mut self, args: &[Value]) -> Completion<Value> {
        let keys = self.get_own_property_keys(argument(args, 0), false)?;
        let names = self.array_of_names(&keys);
        self.drop_key_list(keys);
        names
    }

    /// `Object.getOwnPropertySymbols ( O )` (`#sec-object.getownpropertysymbols`):
    /// CreateArrayFromList of GetOwnPropertyKeys(O, symbol), through the same charged path as
    /// `getOwnPropertyNames` (`story:symbol-constructor-and-well-known-symbols`).
    fn object_get_own_property_symbols(&mut self, args: &[Value]) -> Completion<Value> {
        let keys = self.get_own_property_keys(argument(args, 0), true)?;
        let symbols = self.array_of_names(&keys);
        self.drop_key_list(keys);
        symbols
    }

    /// CreateArrayFromList of the values of `keys` (Strings, or Symbols for
    /// `getOwnPropertySymbols`): the list of the values is charged to the memory budget before
    /// it is reserved and credited when the array is made.
    fn array_of_names(&mut self, keys: &[Key]) -> Completion<Value> {
        let bytes =
            u64::try_from(keys.len().saturating_mul(size_of::<Value>())).unwrap_or(u64::MAX);
        self.charge_bytes(bytes)?;
        let mut names = Vec::new();
        if names.try_reserve_exact(keys.len()).is_err() {
            self.heap.accounting.credit(bytes);
            return Err(self.terminate(Termination::MemoryBudget));
        }
        let mut made = Ok(());
        for &key in keys {
            match self.key_value(key) {
                Ok(name) => names.push(name),
                Err(abrupt) => {
                    made = Err(abrupt);
                    break;
                }
            }
        }
        let array = made.and_then(|()| self.create_array_from_list(&names));
        self.heap.accounting.credit(bytes);
        Ok(Value::Object(array?))
    }

    /// GetOwnPropertyKeys (`#sec-getownpropertykeys`): the String keys of ToObject(O), or its
    /// Symbol keys when `symbols`. [[OwnPropertyKeys]] charges a step per key and its list;
    /// the list answered is charged before it is reserved, and the caller credits it when it
    /// drops it. The list of own keys is credited here.
    pub(crate) fn get_own_property_keys(
        &mut self,
        value: Value,
        symbols: bool,
    ) -> Completion<KeyList> {
        let object = self.to_object(value)?;
        let own_keys = self.own_property_keys(object)?;
        let wanted = |key: &Key| matches!(key, Key::Symbol(_)) == symbols;
        let count = own_keys.iter().filter(|key| wanted(key)).count();
        let answer = self.new_key_list(count).map(|mut list| {
            list.keys
                .extend(own_keys.iter().copied().filter(|key| wanted(key)));
            list
        });
        self.drop_key_list(own_keys);
        answer
    }

    /// CreateArrayFromList (`#sec-createarrayfromlist`): an Array of the current realm. A step
    /// per element; no code runs, so the elements need no root.
    pub(crate) fn create_array_from_list(&mut self, elements: &[Value]) -> Completion<ObjectRef> {
        let array = self.array_create(0.0, None)?;
        for (index, &element) in elements.iter().enumerate() {
            self.charge_steps(1)?;
            let Ok(index) = u32::try_from(index) else {
                return Err(self.fault());
            };
            if !self.create_data_property(array, Key::Index(index), element)? {
                return Err(self.fault());
            }
        }
        Ok(array)
    }

    /// `Object.getPrototypeOf ( O )` (`#sec-object.getprototypeof`).
    fn object_get_prototype_of(&mut self, args: &[Value]) -> Completion<Value> {
        let object = self.to_object(argument(args, 0))?;
        Ok(self
            .get_prototype_of(object)?
            .map_or(Value::Null, Value::Object))
    }

    /// Steps 1 and 2 of `hasOwnProperty` and `propertyIsEnumerable`: ToPropertyKey of V, then
    /// ToObject of the this value. Neither ToObject nor the caller's step 3 runs code, so the
    /// key's atom needs no keep.
    fn key_and_this_object(&mut self, this: Value, args: &[Value]) -> Completion<(Key, ObjectRef)> {
        let key = self.to_property_key(argument(args, 0))?;
        let object = self.to_object(this)?;
        Ok((key, object))
    }

    /// `Object.prototype.hasOwnProperty ( V )` (`#sec-object.prototype.hasownproperty`).
    fn object_has_own_property(&mut self, this: Value, args: &[Value]) -> Completion<Value> {
        let (key, object) = self.key_and_this_object(this, args)?;
        // 3. Return ? HasOwnProperty(O, P).
        Ok(Value::Boolean(self.has_own_property(object, key)?))
    }

    /// `Object.prototype.propertyIsEnumerable ( V )`
    /// (`#sec-object.prototype.propertyisenumerable`).
    fn object_property_is_enumerable(&mut self, this: Value, args: &[Value]) -> Completion<Value> {
        let (key, object) = self.key_and_this_object(this, args)?;
        // 3. Let desc be ? O.[[GetOwnProperty]](P). 4. If desc is undefined, return false.
        // 5. Return desc.[[Enumerable]].
        Ok(Value::Boolean(
            self.get_own_property(object, key)?
                .is_some_and(|property| property.enumerable),
        ))
    }

    /// `Object.prototype.toString ( )` (`#sec-object.prototype.tostring`).
    fn object_to_string(&mut self, this: Value) -> Completion<Value> {
        // 1, 2. undefined and null.
        let object = match this {
            Value::Undefined => {
                return Ok(Value::String(self.new_string_str("[object Undefined]")?));
            }
            Value::Null => return Ok(Value::String(self.new_string_str("[object Null]")?)),
            // 3. Let O be ! ToObject(this value).
            _ => self.to_object(this)?,
        };
        let builtin_tag = self.builtin_tag(object)?;
        // 15. Let tag be ? Get(O, @@toStringTag): it may run code, so O, which may be new, is
        // kept.
        let keep = self.keep(Value::Object(object));
        let tag = match self.well_known() {
            Some(symbols) => self.get(object, Key::Symbol(symbols.to_string_tag)),
            None => Err(self.fault()),
        };
        self.release(keep);
        // 16. If tag is not a String, set tag to builtinTag.
        let mut builder = crate::builtins::string::StringBuilder::new();
        self.append_str(&mut builder, "[object ")?;
        match tag? {
            Value::String(tag) => self.append_string(&mut builder, tag)?,
            _ => self.append_str(&mut builder, builtin_tag)?,
        }
        self.append_str(&mut builder, "]")?;
        // 17. Return the string-concatenation of "[object ", tag, and "]".
        Ok(Value::String(self.finish_string(builder)?))
    }

    /// Steps 4 to 14 of `Object.prototype.toString`: the builtinTag of the kinds this slice
    /// has, [[DateValue]] with them (`story:date-object-clock-and-time-value`); an ordinary
    /// object with a slot the steps do not name, and every other kind, is step 14's "Object".
    fn builtin_tag(&mut self, object: ObjectRef) -> Completion<&'static str> {
        // 4, 5. If isArray is true, let builtinTag be "Array".
        if self.is_array(Value::Object(object))? {
            return Ok("Array");
        }
        // 7. [[Call]]: "Function".
        if self.has_call(object)? {
            return Ok("Function");
        }
        Ok(match &self.engine_object(object)?.kind {
            // 6. [[ParameterMap]]: "Arguments".
            ObjectKind::Arguments(_) => "Arguments",
            // 8. [[ErrorData]] of a host object made with an `error_kind`
            // (`docs/design/host-interface.md`, decision 15): "Error".
            ObjectKind::Host(host) if host.error_kind.is_some() => "Error",
            ObjectKind::Ordinary(ordinary) => match ordinary.data {
                // 8. [[ErrorData]]: "Error".
                OrdinaryData::Error(_) => "Error",
                // 9. [[BooleanData]]: "Boolean".
                OrdinaryData::Boolean(_) => "Boolean",
                // 10. [[NumberData]]: "Number".
                OrdinaryData::Number(_) => "Number",
                // 12. [[DateValue]]: "Date".
                OrdinaryData::Date(_) => "Date",
                // 14. Else, let builtinTag be "Object".
                _ => "Object",
            },
            // 11. [[StringData]]: "String".
            ObjectKind::String(_) => "String",
            // 13. [[RegExpMatcher]]: "RegExp".
            ObjectKind::RegExp(_) => "RegExp",
            // 14. Else, "Object".
            _ => "Object",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{ObjectBuiltin, enumeration, integrity};
    use crate::builtins::group_testing::{GroupMethod, check_group_keys, group_methods};
    use crate::host::agent::Agent;
    use crate::host::hooks::test_host::{NoHooks, agent_with_realm, enter};
    use crate::object::builtin_function::BuiltinId;
    use crate::object::ordinary::OrdinaryData;
    use crate::object::property::Key;
    use crate::realm::intrinsics::Intrinsic;
    use crate::value::{ObjectRef, Value};

    /// The order of keys of `Object` and `Object.prototype` (`story:builtin-group-modules`):
    /// the keys of the base in the base's order, then the keys of the groups `integrity` and
    /// `enumeration`, and nothing else, each a method of its group that runs through its
    /// function object; `Object.prototype` has no Symbol key on the base.
    #[test]
    fn every_group_module_installs_and_runs_through_its_constructor() {
        let (mut agent, realm) = agent_with_realm();
        enter(&mut agent, &realm);
        let handle = agent.heap.roots.get_realm(&realm.0).unwrap();
        let constructor = agent.intrinsic(handle, Intrinsic::Object).unwrap();
        let prototype = agent.intrinsic(handle, Intrinsic::ObjectPrototype).unwrap();
        let constructor_groups: Vec<GroupMethod> = [
            group_methods(integrity::CONSTRUCTOR_METHODS, |i| {
                BuiltinId::Object(ObjectBuiltin::Integrity(i))
            }),
            group_methods(enumeration::CONSTRUCTOR_METHODS, |i| {
                BuiltinId::Object(ObjectBuiltin::Enumeration(i))
            }),
        ]
        .concat();
        let prototype_groups: Vec<GroupMethod> = [
            group_methods(integrity::PROTOTYPE_METHODS, |i| {
                BuiltinId::Object(ObjectBuiltin::Integrity(i))
            }),
            group_methods(enumeration::PROTOTYPE_METHODS, |i| {
                BuiltinId::Object(ObjectBuiltin::Enumeration(i))
            }),
        ]
        .concat();
        check_group_keys(
            &mut agent,
            constructor,
            &[
                "length",
                "name",
                "prototype",
                "defineProperty",
                "getOwnPropertyDescriptor",
                "getOwnPropertyNames",
                "getOwnPropertySymbols",
                "getPrototypeOf",
                "fromEntries",
                "groupBy",
            ],
            &constructor_groups,
            &[],
        );
        check_group_keys(
            &mut agent,
            prototype,
            &[
                "constructor",
                "hasOwnProperty",
                "propertyIsEnumerable",
                "toString",
            ],
            &prototype_groups,
            &[],
        );
    }

    /// `#sec-object.prototype.tostring`, steps 15 and 16: an own @@toStringTag that is a String
    /// replaces the builtinTag, of an ordinary object and of an Array; one that is not a String
    /// does not. A well-known symbol is needed, which the host interface does not hand out.
    #[test]
    fn to_string_tag_of_an_object_replaces_the_builtin_tag() {
        let (mut agent, realm) = agent_with_realm();
        enter(&mut agent, &realm);
        let handle = agent.heap.roots.get_realm(&realm.0).unwrap();
        let object_prototype = agent.intrinsic(handle, Intrinsic::ObjectPrototype).unwrap();
        let to_string_key = agent.name_key("toString").unwrap();
        let to_string = agent.get(object_prototype, to_string_key).unwrap();
        let tag_key = Key::Symbol(agent.well_known().unwrap().to_string_tag);
        let plain = |agent: &mut Agent<NoHooks>| -> ObjectRef {
            let prototype = agent.current_intrinsic(Intrinsic::ObjectPrototype).unwrap();
            agent
                .ordinary_object_create(Some(prototype), OrdinaryData::Plain)
                .unwrap()
        };
        let array = |agent: &mut Agent<NoHooks>| agent.array_create(0.0, None).unwrap();
        type Make = fn(&mut Agent<NoHooks>) -> ObjectRef;
        let cases: [(Make, Option<&str>, &str); 5] = [
            (plain, None, "[object Object]"),
            (plain, Some("Custom"), "[object Custom]"),
            (array, Some("Custom"), "[object Custom]"),
            (plain, Some(""), "[object ]"),
            (array, None, "[object Array]"),
        ];
        for (make, tag, expected) in cases {
            let object = make(&mut agent);
            let tag = match tag {
                Some(text) => Value::String(agent.new_string_str(text).unwrap()),
                None => Value::Number(1.0),
            };
            agent.create_data_property(object, tag_key, tag).unwrap();
            let Ok(Value::String(text)) = agent.call_value(to_string, Value::Object(object), &[])
            else {
                panic!("toString answered no String");
            };
            let text = agent.string_units_for_tests(text).unwrap();
            assert_eq!(String::from_utf16_lossy(&text), expected);
        }
    }
}
