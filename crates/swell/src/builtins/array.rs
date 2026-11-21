//! Array objects (ECMA-262 23.1, `#sec-array-objects`): the `Array` constructor
//! (`#sec-array`), `Array.isArray` (`#sec-array.isarray`), `get Array[@@species]`
//! (`#sec-get-array-%symbol.species%`), `Array.prototype.constructor`
//! (`#sec-array.prototype.constructor`), `.join` (`#sec-array.prototype.join`) with
//! JoinIndexedProperties (`#sec-joinindexedproperties`), `.push` (`#sec-array.prototype.push`)
//! and `.map` (`#sec-array.prototype.map`) with ArraySpeciesCreate (`#sec-arrayspeciescreate`).
//! Written by `story:harness-builtins`; the other members are slice 3.
//!
//! Every loop over indices charges a step per index (decision 2), so a length of 2^32 - 1 or
//! 2^53 - 1 ends in `Terminated(StepBudget)`; `join` builds its String with the builder of
//! `builtins/string.rs`, bounded by `string_length`. What `map` holds across its callback (the
//! object it reads and the array it fills) is kept on the agent's kept stack, which the root
//! site of `object/builtin_function.rs` reports (decision 4).

use crate::builtins::object::argument;
use crate::builtins::string::{StringBuilder, units_of};
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::host::{ErrorKind, Host};
use crate::object::builtin_function::BuiltinId;
use crate::object::property::{Key, Property, Slot};
use crate::ops::conversion::number_to_uint32;
use crate::realm::intrinsics::{Intrinsic, Making};
use crate::value::{ObjectRef, Value};

/// The builtins of this module, by their steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ArrayBuiltin {
    /// `Array ( ...values )` (23.1.1.1).
    Constructor,
    /// `Array.isArray ( arg )` (23.1.2.3).
    IsArray,
    /// `get Array [ @@species ]` (23.1.2.6).
    Species,
    /// `Array.prototype.join ( separator )` (23.1.3.18).
    Join,
    /// `Array.prototype.map ( callback [ , thisArg ] )` (23.1.3.21).
    Map,
    /// `Array.prototype.push ( ...items )` (23.1.3.23).
    Push,
}

/// 2^53 - 1, the largest length (`#sec-tolength`).
const MAX_SAFE_LENGTH: f64 = 9_007_199_254_740_991.0;

/// 2^32 - 1: indices below it are array indices.
const ARRAY_INDEX_END: f64 = 4_294_967_295.0;

impl<H: Host> Agent<H> {
    /// `%Array%` (23.1.1) with `isArray`, `[@@species]`, and the methods of
    /// `%Array.prototype%` this part has.
    pub(crate) fn create_array_constructor(
        &mut self,
        making: Making,
        array_prototype: ObjectRef,
    ) -> Completion<ObjectRef> {
        let constructor = self.create_builtin_constructor(
            making,
            "Array",
            1,
            BuiltinId::Array(ArrayBuiltin::Constructor),
            array_prototype,
        )?;
        self.define_builtin_method(
            making,
            constructor,
            "isArray",
            1,
            BuiltinId::Array(ArrayBuiltin::IsArray),
        )?;
        // 23.1.2.6: an accessor whose set accessor function is undefined; the getter is named
        // "get [Symbol.species]"; the property is { [[Enumerable]]: false, [[Configurable]]:
        // true } (clause 18).
        let species = self.create_builtin(
            making,
            "get [Symbol.species]",
            0,
            BuiltinId::Array(ArrayBuiltin::Species),
        )?;
        self.add_property(
            constructor,
            Key::Symbol(making.symbols.species),
            Property {
                slot: Slot::Accessor {
                    get: Some(species),
                    set: None,
                },
                enumerable: false,
                configurable: true,
            },
        )?;
        for (name, length, id) in [
            ("join", 1, ArrayBuiltin::Join),
            ("map", 1, ArrayBuiltin::Map),
            ("push", 1, ArrayBuiltin::Push),
        ] {
            self.define_builtin_method(
                making,
                array_prototype,
                name,
                length,
                BuiltinId::Array(id),
            )?;
        }
        Ok(constructor)
    }

    /// The steps of the builtins of this module.
    pub(crate) fn run_array_builtin(
        &mut self,
        id: ArrayBuiltin,
        function: ObjectRef,
        this: Value,
        args: &[Value],
        new_target: Option<ObjectRef>,
    ) -> Completion<Value> {
        match id {
            ArrayBuiltin::Constructor => self.array_constructor(function, args, new_target),
            // "Return ? IsArray(arg)."
            ArrayBuiltin::IsArray => Ok(Value::Boolean(self.is_array(argument(args, 0))?)),
            // "Return the this value."
            ArrayBuiltin::Species => Ok(this),
            ArrayBuiltin::Join => self.array_join(this, args),
            ArrayBuiltin::Map => self.array_map(this, args),
            ArrayBuiltin::Push => self.array_push(this, args),
        }
    }

    /// The key of the index `index`, a non-negative integer below 2^53: `! ToString(𝔽(index))`
    /// as a property key, an array index below 2^32 - 1 and an atom above.
    pub(crate) fn index_key(&mut self, index: f64) -> Completion<Key> {
        if index < ARRAY_INDEX_END {
            // A non-negative integer below 2^32 - 1, which the cast keeps exactly.
            return Ok(Key::Index(index as u32));
        }
        self.to_property_key(Value::Number(index))
    }

    /// `Array ( ...values )` (`#sec-array`).
    fn array_constructor(
        &mut self,
        function: ObjectRef,
        args: &[Value],
        new_target: Option<ObjectRef>,
    ) -> Completion<Value> {
        // 1. If NewTarget is undefined, let newTarget be the active function object.
        let new_target = new_target.unwrap_or(function);
        // 2. Let proto be ? GetPrototypeFromConstructor(newTarget, "%Array.prototype%").
        let prototype =
            self.get_prototype_from_constructor(new_target, Intrinsic::ArrayPrototype)?;
        match args {
            // 4. If numberOfArgs = 0, return ! ArrayCreate(0, proto).
            [] => Ok(Value::Object(self.array_create(0.0, Some(prototype))?)),
            // 5. numberOfArgs = 1.
            [length] => {
                let array = self.array_create(0.0, Some(prototype))?;
                let int_length = match *length {
                    // 5.c. If len is not a Number: ! CreateDataPropertyOrThrow(array, "0", len);
                    // intLen is 1.
                    Value::Number(number) => {
                        // 5.d.i. Let intLen be ! ToUint32(len). 5.d.ii. If SameValueZero(intLen,
                        // len) is false, throw a RangeError exception.
                        let int_length = f64::from(number_to_uint32(number));
                        if int_length != number {
                            return Err(
                                self.throw_error(ErrorKind::RangeError, "Invalid array length")
                            );
                        }
                        int_length
                    }
                    element => {
                        if !self.create_data_property(array, Key::Index(0), element)? {
                            return Err(self.fault());
                        }
                        1.0
                    }
                };
                // 5.e. Perform ! Set(array, "length", intLen, true).
                let key = self.name_key("length")?;
                self.set(array, key, Value::Number(int_length), true)?;
                Ok(Value::Object(array))
            }
            // 6. numberOfArgs ≥ 2: ArrayCreate(numberOfArgs, proto), then each element.
            elements => {
                let array = self.array_create(elements.len() as f64, Some(prototype))?;
                for (index, &element) in elements.iter().enumerate() {
                    self.charge_steps(1)?;
                    let Ok(index) = u32::try_from(index) else {
                        return Err(self.fault());
                    };
                    if !self.create_data_property(array, Key::Index(index), element)? {
                        return Err(self.fault());
                    }
                }
                Ok(Value::Object(array))
            }
        }
    }

    /// `Array.prototype.join ( separator )` (`#sec-array.prototype.join`).
    fn array_join(&mut self, this: Value, args: &[Value]) -> Completion<Value> {
        // 1. Let O be ? ToObject(this value). It may be new, and the steps after run code: kept.
        let object = self.to_object(this)?;
        let keep = self.keep(Value::Object(object));
        let joined = self.array_join_steps(object, argument(args, 0));
        self.release(keep);
        joined
    }

    /// Steps 2 to 5 of `join`, with O kept.
    fn array_join_steps(&mut self, object: ObjectRef, separator: Value) -> Completion<Value> {
        // 2. Let len be ? LengthOfArrayLike(O).
        let length = self.length_of_array_like(object)?;
        // 3, 4. If separator is undefined, let sep be ","; else let sep be ? ToString(separator).
        let separator = match separator {
            Value::Undefined => units_of(","),
            separator => {
                let separator = self.to_string(separator)?;
                self.string_units(separator)?
            }
        };
        // 5. Return ? JoinIndexedProperties(O, len, sep).
        let joined = self.join_indexed_properties(object, length, &separator)?;
        Ok(Value::String(joined))
    }

    /// JoinIndexedProperties (`#sec-joinindexedproperties`) without a method name: a step per
    /// index; the result is bounded by `string_length` before it is allocated.
    fn join_indexed_properties(
        &mut self,
        object: ObjectRef,
        length: f64,
        separator: &[u16],
    ) -> Completion<crate::value::StringRef> {
        let mut result = StringBuilder::new();
        let mut index = 0.0;
        while index < length {
            self.charge_steps(1)?;
            // a. If k > 0, set R to the string-concatenation of R and sep.
            if index > 0.0 {
                self.append_units(&mut result, separator)?;
            }
            // b. Let element be ? Get(O, ! ToString(𝔽(k))).
            let key = self.index_key(index)?;
            let element = self.get(object, key)?;
            // c. If element is neither undefined nor null, append ? ToString(element).
            if !matches!(element, Value::Undefined | Value::Null) {
                let string = self.to_string(element)?;
                self.append_string(&mut result, string)?;
            }
            index += 1.0;
        }
        self.finish_string(result)
    }

    /// `Array.prototype.push ( ...items )` (`#sec-array.prototype.push`).
    fn array_push(&mut self, this: Value, args: &[Value]) -> Completion<Value> {
        // 1. Let O be ? ToObject(this value). It may be new, and Set may run code: kept.
        let object = self.to_object(this)?;
        let keep = self.keep(Value::Object(object));
        let pushed = self.array_push_steps(object, args);
        self.release(keep);
        pushed
    }

    /// Steps 2 to 7 of `push`, with O kept. The items are held by this builtin's activation.
    fn array_push_steps(&mut self, object: ObjectRef, items: &[Value]) -> Completion<Value> {
        // 2. Let len be ? LengthOfArrayLike(O).
        let mut length = self.length_of_array_like(object)?;
        // 3, 4. If len + argCount > 2^53 - 1, throw a TypeError exception.
        if length + items.len() as f64 > MAX_SAFE_LENGTH {
            return Err(self.throw_error(
                ErrorKind::TypeError,
                "Array.prototype.push would make a length above 2^53 - 1",
            ));
        }
        // 5. For each element E of items: ? Set(O, ! ToString(𝔽(len)), E, true); len + 1.
        for &item in items {
            self.charge_steps(1)?;
            let key = self.index_key(length)?;
            self.set(object, key, item, true)?;
            length += 1.0;
        }
        // 6. Perform ? Set(O, "length", 𝔽(len), true).
        let key = self.name_key("length")?;
        self.set(object, key, Value::Number(length), true)?;
        // 7. Return 𝔽(len).
        Ok(Value::Number(length))
    }

    /// `Array.prototype.map ( callback [ , thisArg ] )` (`#sec-array.prototype.map`).
    fn array_map(&mut self, this: Value, args: &[Value]) -> Completion<Value> {
        // 1. Let O be ? ToObject(this value). It may be new and the callback runs code: kept.
        let object = self.to_object(this)?;
        let mark = self.keep(Value::Object(object));
        let mapped = self.array_map_steps(object, argument(args, 0), argument(args, 1));
        self.release(mark);
        mapped
    }

    /// Steps 2 to 7 of `map`, with O kept. The callback and thisArg are held by this builtin's
    /// activation; the array A is kept from its creation to the end.
    fn array_map_steps(
        &mut self,
        object: ObjectRef,
        callback: Value,
        this_arg: Value,
    ) -> Completion<Value> {
        // 2. Let len be ? LengthOfArrayLike(O).
        let length = self.length_of_array_like(object)?;
        // 3. If IsCallable(callbackfn) is false, throw a TypeError exception.
        if !self.is_callable(callback)? {
            return Err(self.throw_error(
                ErrorKind::TypeError,
                "Array.prototype.map: the callback is not a function",
            ));
        }
        // 4. Let A be ? ArraySpeciesCreate(O, len).
        let array = self.array_species_create(object, length)?;
        self.keep(Value::Object(array));
        // 5, 6. For each k below len.
        let mut index = 0.0;
        while index < length {
            self.charge_steps(1)?;
            // 6.a, 6.b. Let Pk be ! ToString(𝔽(k)); let kPresent be ? HasProperty(O, Pk).
            let key = self.index_key(index)?;
            if self.has_property(object, key)? {
                // 6.c.i. Let kValue be ? Get(O, Pk).
                let key = self.index_key(index)?;
                let value = self.get(object, key)?;
                // 6.c.ii. Let mappedValue be ? Call(callbackfn, thisArg, « kValue, 𝔽(k), O »).
                let mapped = self.call_value(
                    callback,
                    this_arg,
                    &[value, Value::Number(index), Value::Object(object)],
                )?;
                // 6.c.iii. Perform ? CreateDataPropertyOrThrow(A, Pk, mappedValue). The key is
                // made again: the call may have collected the atom of an index above 2^32 - 2.
                let key = self.index_key(index)?;
                if !self.create_data_property(array, key, mapped)? {
                    return Err(self.throw_error(
                        ErrorKind::TypeError,
                        "Array.prototype.map cannot define an element of its result",
                    ));
                }
            }
            index += 1.0;
        }
        // 7. Return A.
        Ok(Value::Object(array))
    }

    /// ArraySpeciesCreate (`#sec-arrayspeciescreate`).
    pub(crate) fn array_species_create(
        &mut self,
        original: ObjectRef,
        length: f64,
    ) -> Completion<ObjectRef> {
        // 1, 2. If IsArray(originalArray) is false, return ? ArrayCreate(length).
        if !self.is_array(Value::Object(original))? {
            return self.array_create(length, None);
        }
        // 3. Let C be ? Get(originalArray, "constructor").
        let key = self.name_key("constructor")?;
        let mut constructor = self.get(original, key)?;
        // 4. If IsConstructor(C) is true, then
        if let Value::Object(object) = constructor
            && self.has_construct(object)?
        {
            // 4.a, 4.b. thisRealm, and realmC = ? GetFunctionRealm(C).
            let this_realm = self.current_realm()?;
            let constructor_realm = self.get_function_realm(object)?;
            // 4.c. If they differ and C is realmC's %Array%, set C to undefined.
            if this_realm != constructor_realm
                && self.intrinsic(constructor_realm, Intrinsic::Array)? == object
            {
                constructor = Value::Undefined;
            }
        }
        // 5. If C is an Object, set C to ? Get(C, @@species); null becomes undefined.
        if let Value::Object(object) = constructor {
            let Some(symbols) = self.well_known() else {
                return Err(self.fault());
            };
            constructor = match self.get(object, Key::Symbol(symbols.species))? {
                Value::Null => Value::Undefined,
                species => species,
            };
        }
        // 6. If C is undefined, return ? ArrayCreate(length).
        if constructor == Value::Undefined {
            return self.array_create(length, None);
        }
        // 7. If IsConstructor(C) is false, throw a TypeError exception.
        let Value::Object(object) = constructor else {
            return Err(self.throw_error(ErrorKind::TypeError, "the species is not a constructor"));
        };
        if !self.has_construct(object)? {
            return Err(self.throw_error(ErrorKind::TypeError, "the species is not a constructor"));
        }
        // 8. Return ? Construct(C, « 𝔽(length) »).
        match self.construct_value(object, &[Value::Number(length)], None)? {
            Value::Object(made) => Ok(made),
            _ => Err(self.fault()),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::host::hooks::test_host::{NoHooks, agent_with_realm, enter};
    use crate::host::outcome::{Abrupt, Completion};
    use crate::host::{Cx, HostCall};
    use crate::object::property::{Key, Slot};
    use crate::realm::intrinsics::Intrinsic;
    use crate::value::Value;

    /// A species constructor that records the length it is given in the host's place, the
    /// callee's "seen" property, and answers a new ordinary object.
    fn species(cx: &mut Cx<'_, NoHooks>, call: HostCall<'_>) -> Completion<Value> {
        let agent = &mut *cx.agent;
        let key = agent.name_key("seen")?;
        let length = call.args.first().copied().unwrap_or(Value::Undefined);
        agent.create_data_property(call.callee, key, length)?;
        let prototype = agent.current_intrinsic(Intrinsic::ObjectPrototype)?;
        let made = agent.ordinary_object_create(
            Some(prototype),
            crate::object::ordinary::OrdinaryData::Plain,
        )?;
        Ok(Value::Object(made))
    }

    /// A callback that answers its value.
    fn identity(_cx: &mut Cx<'_, NoHooks>, call: HostCall<'_>) -> Completion<Value> {
        Ok(call.args.first().copied().unwrap_or(Value::Undefined))
    }

    /// `#sec-arrayspeciescreate`, step 5, the part that needs @@species, which the host
    /// interface does not hand out: an Array whose constructor has an own @@species
    /// constructor gets its result from that constructor, called with the length, and `map`
    /// fills it, skipping holes; a @@species of null makes an Array; a @@species that is not a
    /// constructor throws a TypeError. `Array[@@species]` (`#sec-get-array-%symbol.species%`) is
    /// an accessor whose getter answers its this value and whose setter is undefined.
    #[test]
    fn map_uses_species_and_skips_holes() {
        let (mut agent, realm) = agent_with_realm();
        enter(&mut agent, &realm);
        let handle = agent.heap.roots.get_realm(&realm.0).unwrap();
        let array_constructor = agent.intrinsic(handle, Intrinsic::Array).unwrap();
        let array_prototype = agent.intrinsic(handle, Intrinsic::ArrayPrototype).unwrap();
        let species_key = Key::Symbol(agent.well_known().unwrap().species);
        let property = agent
            .get_own_property(array_constructor, species_key)
            .unwrap()
            .unwrap();
        let Slot::Accessor {
            get: Some(getter),
            set: None,
        } = property.slot
        else {
            panic!("{property:?}");
        };
        assert!(!property.enumerable && property.configurable);
        assert_eq!(
            agent.call_value(Value::Object(getter), Value::Number(3.0), &[]),
            Ok(Value::Number(3.0))
        );

        let constructor = agent
            .create_host_function(handle, "Species", 0, species, true, None)
            .unwrap();
        let callback = agent
            .create_host_function(handle, "identity", 1, identity, false, None)
            .unwrap();
        let owner = agent
            .create_host_function(handle, "Owner", 0, species, true, None)
            .unwrap();
        let source = agent.array_create(3.0, None).unwrap();
        for object in [constructor, callback, owner, source] {
            agent.keep(Value::Object(object));
        }
        let a = Value::String(agent.new_string_str("a").unwrap());
        agent
            .create_data_property(source, Key::Index(0), a)
            .unwrap();
        agent
            .create_data_property(source, Key::Index(2), Value::Number(2.0))
            .unwrap();
        let constructor_key = agent.name_key("constructor").unwrap();
        agent
            .create_data_property(source, constructor_key, Value::Object(owner))
            .unwrap();
        agent
            .create_data_property(owner, species_key, Value::Object(constructor))
            .unwrap();
        let map_key = agent.name_key("map").unwrap();
        let map = agent.get(array_prototype, map_key).unwrap();
        let Ok(Value::Object(mapped)) =
            agent.call_value(map, Value::Object(source), &[Value::Object(callback)])
        else {
            panic!("map answered no object");
        };
        let seen_key = agent.name_key("seen").unwrap();
        assert_eq!(agent.get(constructor, seen_key), Ok(Value::Number(3.0)));
        assert_eq!(agent.is_array(Value::Object(mapped)), Ok(false));
        assert_eq!(agent.get(mapped, Key::Index(0)), Ok(a));
        assert_eq!(agent.has_property(mapped, Key::Index(1)), Ok(false));
        assert_eq!(agent.get(mapped, Key::Index(2)), Ok(Value::Number(2.0)));

        // A @@species of null makes an Array; one that is not a constructor throws.
        let species_property =
            |value| crate::object::property::Descriptor::data(value, true, true, true);
        agent
            .define_own_property(owner, species_key, species_property(Value::Null))
            .unwrap();
        let Ok(Value::Object(mapped)) =
            agent.call_value(map, Value::Object(source), &[Value::Object(callback)])
        else {
            panic!("map answered no object");
        };
        assert_eq!(agent.is_array(Value::Object(mapped)), Ok(true));
        agent
            .define_own_property(
                owner,
                species_key,
                species_property(Value::Object(callback)),
            )
            .unwrap();
        assert!(matches!(
            agent.call_value(map, Value::Object(source), &[Value::Object(callback)]),
            Err(Abrupt::Throw(_))
        ));
    }
}
