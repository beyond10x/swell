//! Operations on objects (ECMA-262 7.3, `#sec-operations-on-objects`): Get, GetV, GetMethod,
//! Set, CreateDataProperty, DefinePropertyOrThrow, DeletePropertyOrThrow, HasProperty (the
//! internal method of `object/mod.rs`), HasOwnProperty, Call, Construct, LengthOfArrayLike,
//! GetFunctionRealm, OrdinaryHasInstance and InstanceofOperator (13.10.2), and
//! EnumerateObjectProperties (`#sec-enumerate-object-properties`). Written by
//! `story:object-model-and-realms`. `story:builtin-group-modules` adds what the builtins of
//! slice 3 share: CreateDataPropertyOrThrow (`#sec-createdatapropertyorthrow`, from the
//! interpreter), SetIntegrityLevel (`#sec-setintegritylevel`), TestIntegrityLevel
//! (`#sec-testintegritylevel`), Invoke (`#sec-invoke`) and EnumerableOwnProperties
//! (`#sec-enumerableownproperties`, from `builtins/json.rs`).

use std::collections::BTreeSet;

use crate::heap::arena::Handle;
use crate::host::agent::Agent;
use crate::host::outcome::{Completion, Termination};
use crate::host::{ErrorKind, Host};
use crate::object::ObjectKind;
use crate::object::property::{Descriptor, Key, Property, Slot};
use crate::value::{ObjectRef, Value};

/// The level of SetIntegrityLevel and TestIntegrityLevel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum IntegrityLevel {
    Sealed,
    Frozen,
}

/// The kind of EnumerableOwnProperties.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "Object.keys, Object.values, Object.entries: story:object-enumeration-and-copy-methods"
    )
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PropertyKind {
    Key,
    Value,
    KeyValue,
}

/// An order on keys for the visited set of EnumerateObjectProperties: the handles' fields.
fn key_order(key: Key) -> (u8, u32, u32, u32) {
    match key {
        Key::Index(index) => (0, index, 0, 0),
        Key::String(string) => (
            1,
            string.handle.key,
            string.handle.index,
            string.handle.generation,
        ),
        Key::Symbol(symbol) => (
            2,
            symbol.handle.key,
            symbol.handle.index,
            symbol.handle.generation,
        ),
    }
}

impl<H: Host> Agent<H> {
    /// Get (`#sec-get-o-p`).
    pub(crate) fn get(&mut self, object: ObjectRef, key: Key) -> Completion<Value> {
        self.internal_get(object, key, Value::Object(object))
    }

    /// GetV (`#sec-getv`).
    pub(crate) fn get_v(&mut self, value: Value, key: Key) -> Completion<Value> {
        let object = self.to_object(value)?;
        self.internal_get(object, key, value)
    }

    /// GetMethod (`#sec-getmethod`): `None` for undefined or null.
    pub(crate) fn get_method(&mut self, value: Value, key: Key) -> Completion<Option<Value>> {
        let function = self.get_v(value, key)?;
        if matches!(function, Value::Undefined | Value::Null) {
            return Ok(None);
        }
        if !self.is_callable(function)? {
            return Err(self.throw_error(ErrorKind::TypeError, "the method is not a function"));
        }
        Ok(Some(function))
    }

    /// Set (`#sec-set-o-p-v-throw`): whether the property was set; with `throw`, a refusal is a
    /// TypeError.
    pub(crate) fn set(
        &mut self,
        object: ObjectRef,
        key: Key,
        value: Value,
        throw: bool,
    ) -> Completion<bool> {
        let done = self.internal_set(object, key, value, Value::Object(object))?;
        if !done && throw {
            return Err(self.throw_error(ErrorKind::TypeError, "Cannot assign to the property"));
        }
        Ok(done)
    }

    /// CreateDataProperty (`#sec-createdataproperty`).
    pub(crate) fn create_data_property(
        &mut self,
        object: ObjectRef,
        key: Key,
        value: Value,
    ) -> Completion<bool> {
        self.define_own_property(object, key, Descriptor::data(value, true, true, true))
    }

    /// CreateDataPropertyOrThrow (`#sec-createdatapropertyorthrow`).
    pub(crate) fn create_data_property_or_throw(
        &mut self,
        object: ObjectRef,
        key: Key,
        value: Value,
    ) -> Completion<()> {
        // 1. Let success be ? CreateDataProperty(O, P, V).
        // 2. If success is false, throw a TypeError exception.
        if !self.create_data_property(object, key, value)? {
            return Err(self.throw_error(ErrorKind::TypeError, "Cannot define the property"));
        }
        Ok(())
    }

    /// DefinePropertyOrThrow (`#sec-definepropertyorthrow`).
    pub(crate) fn define_property_or_throw(
        &mut self,
        object: ObjectRef,
        key: Key,
        descriptor: Descriptor,
    ) -> Completion<()> {
        if !self.define_own_property(object, key, descriptor)? {
            return Err(self.throw_error(ErrorKind::TypeError, "Cannot define the property"));
        }
        Ok(())
    }

    /// DeletePropertyOrThrow (`#sec-deletepropertyorthrow`).
    pub(crate) fn delete_property_or_throw(
        &mut self,
        object: ObjectRef,
        key: Key,
    ) -> Completion<()> {
        if !self.internal_delete(object, key)? {
            return Err(self.throw_error(ErrorKind::TypeError, "Cannot delete the property"));
        }
        Ok(())
    }

    /// HasOwnProperty (`#sec-hasownproperty`).
    pub(crate) fn has_own_property(&mut self, object: ObjectRef, key: Key) -> Completion<bool> {
        Ok(self.get_own_property(object, key)?.is_some())
    }

    /// Call (`#sec-call`).
    pub(crate) fn call_value(
        &mut self,
        function: Value,
        this: Value,
        args: &[Value],
    ) -> Completion<Value> {
        let Value::Object(object) = function else {
            return Err(self.throw_error(ErrorKind::TypeError, "the value is not a function"));
        };
        if !self.has_call(object)? {
            return Err(self.throw_error(ErrorKind::TypeError, "the object is not a function"));
        }
        self.internal_call(object, this, args)
    }

    /// Construct (`#sec-construct`): `new_target` is `function` when `None`. The caller has
    /// checked that both are constructors.
    pub(crate) fn construct_value(
        &mut self,
        function: ObjectRef,
        args: &[Value],
        new_target: Option<ObjectRef>,
    ) -> Completion<Value> {
        self.internal_construct(function, args, new_target.unwrap_or(function))
    }

    /// Invoke ( V, P [ , argumentsList ] ) (`#sec-invoke`): the method `key` of `value`, a
    /// primitive included, called with `value` as the this value.
    pub(crate) fn invoke(&mut self, value: Value, key: Key, args: &[Value]) -> Completion<Value> {
        // 2. Let func be ? GetV(V, P). Nothing runs between it and the Call, which holds func,
        // V and the arguments in its activation.
        let function = self.get_v(value, key)?;
        // 3. Return ? Call(func, V, argumentsList).
        self.call_value(function, value, args)
    }

    /// The bytes of a list of `count` keys, as the integrity levels charge it.
    fn key_list_bytes(count: usize) -> u64 {
        u64::try_from(count.saturating_mul(size_of::<Key>())).unwrap_or(u64::MAX)
    }

    /// The own keys of `object` by [[OwnPropertyKeys]], charged to the memory budget and kept
    /// on the kept stack until the caller credits `bytes` and releases `mark`: answers the
    /// keys, the bytes charged and the mark.
    fn held_own_keys(&mut self, object: ObjectRef) -> Completion<(Vec<Key>, u64, usize)> {
        let keys = self.own_property_keys(object)?;
        let bytes = Self::key_list_bytes(keys.len());
        if self.heap.accounting.charge(bytes).is_err() {
            return Err(self.terminate(Termination::MemoryBudget));
        }
        let mark = self.kept.len();
        if self.kept.try_reserve(keys.len()).is_err() {
            self.heap.accounting.credit(bytes);
            return Err(self.terminate(Termination::MemoryBudget));
        }
        for &key in &keys {
            self.keep_key(key);
        }
        Ok((keys, bytes, mark))
    }

    /// SetIntegrityLevel ( O, level ) (`#sec-setintegritylevel`), for every object the base has
    /// (ordinary, Array, String and arguments exotic objects, functions): a step per key, the
    /// list of keys charged to the memory budget and kept while it runs.
    pub(crate) fn set_integrity_level(
        &mut self,
        object: ObjectRef,
        level: IntegrityLevel,
    ) -> Completion<bool> {
        // 1. Let status be ? O.[[PreventExtensions]](). 2. If status is false, return false.
        if !self.prevent_extensions(object)? {
            return Ok(false);
        }
        // 3. Let keys be ? O.[[OwnPropertyKeys]]().
        let (keys, bytes, mark) = self.held_own_keys(object)?;
        let done = self.set_integrity_level_keys(object, level, &keys);
        self.release(mark);
        self.heap.accounting.credit(bytes);
        done?;
        // 6. Return true.
        Ok(true)
    }

    /// Steps 4 and 5 of SetIntegrityLevel, over `keys`.
    fn set_integrity_level_keys(
        &mut self,
        object: ObjectRef,
        level: IntegrityLevel,
        keys: &[Key],
    ) -> Completion<()> {
        for &key in keys {
            self.charge_steps(1)?;
            let descriptor = match level {
                // 4. If level is sealed: for each key, ? DefinePropertyOrThrow(O, k,
                // PropertyDescriptor { [[Configurable]]: false }).
                IntegrityLevel::Sealed => Descriptor {
                    configurable: Some(false),
                    ..Descriptor::default()
                },
                // 5.b. For each key: let currentDesc be ? O.[[GetOwnProperty]](k); if it is not
                // undefined, { [[Configurable]]: false } for an accessor, else
                // { [[Configurable]]: false, [[Writable]]: false }.
                IntegrityLevel::Frozen => match self.get_own_property(object, key)? {
                    None => continue,
                    Some(Property {
                        slot: Slot::Accessor { .. },
                        ..
                    }) => Descriptor {
                        configurable: Some(false),
                        ..Descriptor::default()
                    },
                    Some(_) => Descriptor {
                        configurable: Some(false),
                        writable: Some(false),
                        ..Descriptor::default()
                    },
                },
            };
            // Perform ? DefinePropertyOrThrow(O, k, desc).
            self.define_property_or_throw(object, key, descriptor)?;
        }
        Ok(())
    }

    /// TestIntegrityLevel ( O, level ) (`#sec-testintegritylevel`), for every object the base
    /// has: a step per key, the list of keys charged to the memory budget and kept while it
    /// runs.
    pub(crate) fn test_integrity_level(
        &mut self,
        object: ObjectRef,
        level: IntegrityLevel,
    ) -> Completion<bool> {
        // 1. Let extensible be ? IsExtensible(O). 2. If extensible is true, return false.
        if self.is_extensible(object)? {
            return Ok(false);
        }
        // 4. Let keys be ? O.[[OwnPropertyKeys]]().
        let (keys, bytes, mark) = self.held_own_keys(object)?;
        let answer = self.test_integrity_level_keys(object, level, &keys);
        self.release(mark);
        self.heap.accounting.credit(bytes);
        answer
    }

    /// Steps 5 and 6 of TestIntegrityLevel, over `keys`.
    fn test_integrity_level_keys(
        &mut self,
        object: ObjectRef,
        level: IntegrityLevel,
        keys: &[Key],
    ) -> Completion<bool> {
        // 5. For each element k of keys.
        for &key in keys {
            self.charge_steps(1)?;
            // a. Let currentDesc be ? O.[[GetOwnProperty]](k). b. If it is not undefined:
            let Some(current) = self.get_own_property(object, key)? else {
                continue;
            };
            // i. If currentDesc.[[Configurable]] is true, return false.
            if current.configurable {
                return Ok(false);
            }
            // ii. If level is frozen and IsDataDescriptor(currentDesc) is true: if
            // currentDesc.[[Writable]] is true, return false.
            if level == IntegrityLevel::Frozen
                && let Slot::Data { writable: true, .. } = current.slot
            {
                return Ok(false);
            }
        }
        // 6. Return true.
        Ok(true)
    }

    /// EnumerableOwnProperties ( O, kind ) (`#sec-enumerableownproperties`) of kind key, the
    /// keys as the property keys they are (a String key, an array index as `Key::Index`): the
    /// String keys of `object` whose own property is enumerable, in the order of
    /// [[OwnPropertyKeys]]. A step per key; each key answered is kept on the kept stack, which
    /// the caller releases to the mark it took before the call. No code runs.
    pub(crate) fn enumerable_own_keys(&mut self, object: ObjectRef) -> Completion<Vec<Key>> {
        // 1. Let ownKeys be ? O.[[OwnPropertyKeys]]().
        let own_keys = self.own_property_keys(object)?;
        // 2. Let results be a new empty List.
        let mut keys = Vec::new();
        if keys.try_reserve(own_keys.len()).is_err() {
            return Err(self.terminate(Termination::MemoryBudget));
        }
        // 3. For each element key of ownKeys: a. If key is a String.
        for key in own_keys {
            self.charge_steps(1)?;
            if matches!(key, Key::Symbol(_)) {
                continue;
            }
            // 3.a.i, ii. Let desc be ? O.[[GetOwnProperty]](key); if it is not undefined and
            // desc.[[Enumerable]] is true, append key.
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

    /// EnumerableOwnProperties ( O, kind ) (`#sec-enumerableownproperties`) as the List of
    /// ECMAScript values the clause answers: of kind key the keys as Strings, of kind value the
    /// values, of kind key+value an Array of each key and value. Each key is read again by
    /// [[GetOwnProperty]] when its turn comes, so a property that a getter of an earlier one
    /// deleted is skipped. A step per key; the own keys and the results are kept on the kept
    /// stack across each Get, and stay kept until the caller releases to the mark it took
    /// before the call.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "Object.keys, Object.values, Object.entries: story:object-enumeration-and-copy-methods"
        )
    )]
    pub(crate) fn enumerable_own_properties(
        &mut self,
        object: ObjectRef,
        kind: PropertyKind,
    ) -> Completion<Vec<Value>> {
        // 1. Let ownKeys be ? O.[[OwnPropertyKeys]](). Kept: a getter may collect an atom.
        let own_keys = self.own_property_keys(object)?;
        if self.kept.try_reserve(own_keys.len()).is_err() {
            return Err(self.terminate(Termination::MemoryBudget));
        }
        for &key in &own_keys {
            self.keep_key(key);
        }
        // 2. Let results be a new empty List.
        let mut results = Vec::new();
        if results.try_reserve(own_keys.len()).is_err() {
            return Err(self.terminate(Termination::MemoryBudget));
        }
        // 3. For each element key of ownKeys: a. If key is a String.
        for key in own_keys {
            self.charge_steps(1)?;
            if matches!(key, Key::Symbol(_)) {
                continue;
            }
            // 3.a.i. Let desc be ? O.[[GetOwnProperty]](key). ii. If desc is not undefined and
            // desc.[[Enumerable]] is true:
            if !self
                .get_own_property(object, key)?
                .is_some_and(|property| property.enumerable)
            {
                continue;
            }
            let result = match kind {
                // 1. If kind is key, append key to results.
                PropertyKind::Key => self.key_value(key)?,
                // 2.a. Let value be ? Get(O, key). b. If kind is value, append value.
                PropertyKind::Value => self.get(object, key)?,
                // 2.c. Else: let entry be CreateArrayFromList(« key, value »); append entry.
                PropertyKind::KeyValue => {
                    let value = self.get(object, key)?;
                    let mark = self.keep(value);
                    let name = self.key_value(key);
                    let entry = name.and_then(|name| {
                        self.keep(name);
                        self.create_array_from_list(&[name, value])
                    });
                    self.release(mark);
                    Value::Object(entry?)
                }
            };
            self.keep(result);
            results.push(result);
        }
        Ok(results)
    }

    /// LengthOfArrayLike (`#sec-lengthofarraylike`).
    pub(crate) fn length_of_array_like(&mut self, object: ObjectRef) -> Completion<f64> {
        let key = self.name_key("length")?;
        let length = self.get(object, key)?;
        self.to_length(length)
    }

    /// The [[BoundTargetFunction]] of `object` when it is a bound function exotic object.
    fn bound_target(&mut self, object: ObjectRef) -> Completion<Option<ObjectRef>> {
        let target = match &self.engine_object(object)?.kind {
            ObjectKind::BoundFunction(bound) => Some(bound.target),
            _ => None,
        };
        match target {
            Some(Some(target)) => Ok(Some(target)),
            // Only the collector's fixture has no target; reached here it is a broken invariant.
            Some(None) => Err(self.fault()),
            None => Ok(None),
        }
    }

    /// GetFunctionRealm (`#sec-getfunctionrealm`): step 1, the [[Realm]] of a built-in or an
    /// ECMAScript function object; step 2, for a bound function exotic object its target's
    /// ("Return ? GetFunctionRealm(boundTargetFunc)"), as a loop with a step per bound function,
    /// since a chain of bound functions is as long as a script makes it; step 4, the current
    /// realm. Proxies (step 3) come with their story.
    pub(crate) fn get_function_realm(&mut self, object: ObjectRef) -> Completion<Handle> {
        let mut function = object;
        loop {
            // 1. If obj has a [[Realm]] internal slot, return obj.[[Realm]].
            let realm = match &self.engine_object(function)?.kind {
                ObjectKind::BuiltinFunction(builtin) => builtin.realm,
                ObjectKind::EcmascriptFunction(function) => function.realm,
                _ => None,
            };
            if let Some(realm) = realm {
                return Ok(realm);
            }
            match self.bound_target(function)? {
                Some(target) => {
                    self.charge_steps(1)?;
                    function = target;
                }
                None => return self.current_realm(),
            }
        }
    }

    /// OrdinaryHasInstance (`#sec-ordinaryhasinstance`). Step 2 for a bound function, "Return ?
    /// InstanceofOperator(O, BC)", runs InstanceofOperator's steps (`#sec-instanceofoperator`)
    /// on the target here and, when they reach OrdinaryHasInstance again, goes round the loop:
    /// a step per bound function, no Rust recursion. A step per object of the prototype chain.
    pub(crate) fn ordinary_has_instance(
        &mut self,
        constructor: Value,
        value: Value,
    ) -> Completion<bool> {
        let mut constructor = constructor;
        let constructor = loop {
            // 1. If IsCallable(C) is false, return false.
            if !self.is_callable(constructor)? {
                return Ok(false);
            }
            let Value::Object(object) = constructor else {
                return Ok(false);
            };
            // 2. If C has a [[BoundTargetFunction]] internal slot: InstanceofOperator(O, BC).
            let Some(target) = self.bound_target(object)? else {
                break object;
            };
            self.charge_steps(1)?;
            let target = Value::Object(target);
            // InstanceofOperator, steps 2 and 3: BC is an Object; its @@hasInstance method.
            let handler = match self.well_known() {
                Some(symbols) => self.get_method(target, Key::Symbol(symbols.has_instance))?,
                None => None,
            };
            if let Some(handler) = handler {
                let result = self.call_value(handler, target, &[value])?;
                return self.to_boolean(result);
            }
            // Steps 4 and 5: not callable throws; else OrdinaryHasInstance(BC, O).
            if !self.is_callable(target)? {
                return Err(self.throw_error(
                    ErrorKind::TypeError,
                    "the right-hand side of instanceof is not callable",
                ));
            }
            constructor = target;
        };
        let Value::Object(mut object) = value else {
            return Ok(false);
        };
        let key = self.name_key("prototype")?;
        let Value::Object(prototype) = self.get(constructor, key)? else {
            return Err(self.throw_error(
                ErrorKind::TypeError,
                "the prototype of the right-hand side is not an object",
            ));
        };
        loop {
            self.charge_steps(1)?;
            match self.get_prototype_of(object)? {
                None => return Ok(false),
                Some(parent) if parent == prototype => return Ok(true),
                Some(parent) => object = parent,
            }
        }
    }

    /// InstanceofOperator (`#sec-instanceofoperator`).
    pub(crate) fn instanceof_operator(&mut self, value: Value, target: Value) -> Completion<bool> {
        if !matches!(target, Value::Object(_)) {
            return Err(self.throw_error(
                ErrorKind::TypeError,
                "the right-hand side of instanceof is not an object",
            ));
        }
        let handler = match self.well_known() {
            Some(symbols) => self.get_method(target, Key::Symbol(symbols.has_instance))?,
            None => None,
        };
        if let Some(handler) = handler {
            let result = self.call_value(handler, target, &[value])?;
            return self.to_boolean(result);
        }
        if !self.is_callable(target)? {
            return Err(self.throw_error(
                ErrorKind::TypeError,
                "the right-hand side of instanceof is not callable",
            ));
        }
        self.ordinary_has_instance(target, value)
    }

    /// EnumerateObjectProperties (`#sec-enumerate-object-properties`), as the informative
    /// definition there does it: the enumerable String keys of the object and then of each
    /// prototype, each name once, a name shadowed by a non-enumerable property not visited.
    /// A step per key and per object of the chain.
    pub(crate) fn enumerate_object_properties(
        &mut self,
        object: ObjectRef,
    ) -> Completion<Vec<Key>> {
        let mut visited = BTreeSet::new();
        let mut found = Vec::new();
        let mut current = Some(object);
        while let Some(o) = current {
            self.charge_steps(1)?;
            for key in self.own_property_keys(o)? {
                if let Key::Symbol(_) = key {
                    continue;
                }
                let Some(property) = self.get_own_property(o, key)? else {
                    continue;
                };
                if visited.insert(key_order(key)) && property.enumerable {
                    found.push(key);
                }
            }
            current = self.get_prototype_of(o)?;
        }
        Ok(found)
    }
}

#[cfg(test)]
mod tests {
    use super::{IntegrityLevel, PropertyKind};
    use crate::host::agent::Agent;
    use crate::host::hooks::test_host::{NoHooks, agent_with_realm, enter};
    use crate::host::outcome::{Abrupt, Completion, Outcome, Termination};
    use crate::host::{Cx, ErrorKind, Host, HostCall, Parsed, Source};
    use crate::object::ordinary::OrdinaryData;
    use crate::object::property::{Descriptor, Key, Property, Slot};
    use crate::realm::intrinsics::Intrinsic;
    use crate::value::{ObjectRef, Realm, Value};

    fn constructor(_cx: &mut Cx<'_, NoHooks>, _call: HostCall<'_>) -> Completion<Value> {
        Ok(Value::Undefined)
    }

    fn function<H: Host>(
        agent: &mut Agent<H>,
        realm: &crate::value::Realm,
        steps: crate::host::HostFn<H>,
    ) -> ObjectRef {
        let handle = agent.heap.roots.get_realm(&realm.0).unwrap();
        agent
            .create_host_function(handle, "F", 0, steps, true, None)
            .unwrap()
    }

    /// InstanceofOperator (`#sec-instanceofoperator`) with OrdinaryHasInstance
    /// (`#sec-ordinaryhasinstance`): the @@hasInstance method of a function is
    /// `%Function.prototype%[@@hasInstance]`, so the prototype chain of the value is searched
    /// for the constructor's "prototype"; a
    /// non-object or non-callable right-hand side and a non-object "prototype" throw a
    /// TypeError; a primitive left-hand side is not an instance.
    #[test]
    fn instanceof_uses_ordinary_has_instance() {
        let (mut agent, realm) = agent_with_realm();
        enter(&mut agent, &realm);
        let f = function(&mut agent, &realm, constructor);
        let prototype = agent
            .ordinary_object_create(None, OrdinaryData::Plain)
            .unwrap();
        let key = agent.name_key("prototype").unwrap();
        assert_eq!(
            agent.create_data_property(f, key, Value::Object(prototype)),
            Ok(true)
        );
        let middle = agent
            .ordinary_object_create(Some(prototype), OrdinaryData::Plain)
            .unwrap();
        let instance = agent
            .ordinary_object_create(Some(middle), OrdinaryData::Plain)
            .unwrap();
        let other = agent
            .ordinary_object_create(None, OrdinaryData::Plain)
            .unwrap();
        // `%Function.prototype%[@@hasInstance]` is called for each instanceof, and a call is a
        // safe point that may collect: the objects held only by these locals are kept.
        for object in [f, prototype, middle, instance, other] {
            agent.keep(Value::Object(object));
        }
        let f = Value::Object(f);
        assert_eq!(
            agent.instanceof_operator(Value::Object(instance), f),
            Ok(true)
        );
        assert_eq!(
            agent.instanceof_operator(Value::Object(middle), f),
            Ok(true)
        );
        assert_eq!(
            agent.instanceof_operator(Value::Object(prototype), f),
            Ok(false)
        );
        assert_eq!(
            agent.instanceof_operator(Value::Object(other), f),
            Ok(false)
        );
        assert_eq!(agent.instanceof_operator(Value::Number(1.0), f), Ok(false));

        for target in [Value::Number(1.0), Value::Object(other)] {
            assert!(matches!(
                agent.instanceof_operator(Value::Object(instance), target),
                Err(Abrupt::Throw(_))
            ));
        }
        let Value::Object(f_object) = f else {
            unreachable!()
        };
        assert_eq!(
            agent.create_data_property(f_object, key, Value::Number(3.0)),
            Ok(true)
        );
        assert!(matches!(
            agent.instanceof_operator(Value::Object(instance), f),
            Err(Abrupt::Throw(_))
        ));
    }

    /// EnumerateObjectProperties (`#sec-enumerate-object-properties`): own enumerable String
    /// keys, then the prototype's not shadowed, never a Symbol; with HasOwnProperty and
    /// LengthOfArrayLike.
    #[test]
    fn enumerate_object_properties_skips_shadowed_and_symbols() {
        let (mut agent, realm) = agent_with_realm();
        enter(&mut agent, &realm);
        let parent = agent
            .ordinary_object_create(None, OrdinaryData::Plain)
            .unwrap();
        let child = agent
            .ordinary_object_create(Some(parent), OrdinaryData::Plain)
            .unwrap();
        let [a, b, c, length] = ["a", "b", "c", "length"].map(|name| agent.name_key(name).unwrap());
        let symbol = Key::Symbol(agent.heap.new_symbol(None).unwrap());
        for key in [a, b, c] {
            agent
                .create_data_property(parent, key, Value::Null)
                .unwrap();
        }
        agent
            .define_own_property(child, b, Descriptor::data(Value::Null, true, false, true))
            .unwrap();
        agent
            .create_data_property(child, symbol, Value::Null)
            .unwrap();
        agent
            .create_data_property(child, Key::Index(0), Value::Null)
            .unwrap();
        agent
            .create_data_property(child, length, Value::Number(2.5))
            .unwrap();
        assert_eq!(
            agent.enumerate_object_properties(child),
            Ok(vec![Key::Index(0), length, a, c])
        );
        assert_eq!(agent.has_own_property(child, b), Ok(true));
        assert_eq!(agent.has_own_property(child, a), Ok(false));
        assert_eq!(agent.length_of_array_like(child), Ok(2.0));
    }

    /// Whether `completion` is a throw of a TypeError of the current realm.
    fn is_type_error<T: std::fmt::Debug>(
        agent: &mut Agent<NoHooks>,
        completion: Completion<T>,
    ) -> bool {
        let Err(Abrupt::Throw(Value::Object(error))) = completion else {
            return false;
        };
        let prototype = agent
            .current_intrinsic(Intrinsic::ErrorPrototype(ErrorKind::TypeError))
            .unwrap();
        agent.get_prototype_of(error) == Ok(Some(prototype))
    }

    /// A getter that answers 5.
    fn five(_cx: &mut Cx<'_, NoHooks>, _call: HostCall<'_>) -> Completion<Value> {
        Ok(Value::Number(5.0))
    }

    /// A plain object of the current realm.
    fn plain(agent: &mut Agent<NoHooks>) -> ObjectRef {
        let prototype = agent.current_intrinsic(Intrinsic::ObjectPrototype).unwrap();
        let object = agent
            .ordinary_object_create(Some(prototype), OrdinaryData::Plain)
            .unwrap();
        agent.keep(Value::Object(object));
        object
    }

    /// The object a script answers, evaluated in `realm`, which is entered again after it.
    fn script_object(agent: &mut Agent<NoHooks>, realm: &Realm, source: &str) -> ObjectRef {
        let Ok(Parsed::Parsed(script)) = agent.parse_script(realm, Source::Utf8(source), None)
        else {
            panic!("{source} did not parse");
        };
        let Ok(Outcome::Normal(root)) = agent.evaluate_script(&script) else {
            panic!("{source} did not answer");
        };
        enter(agent, realm);
        let Ok(Value::Object(object)) = agent.heap.roots.get(&root) else {
            panic!("{source} answered no object");
        };
        agent.keep(Value::Object(object));
        object
    }

    /// Every own property of `object` with the attributes of `level`, its value or its
    /// functions as `before` has them.
    fn check_level(
        agent: &mut Agent<NoHooks>,
        object: ObjectRef,
        level: IntegrityLevel,
        before: &[(Key, Property)],
    ) {
        assert_eq!(agent.is_extensible(object), Ok(false));
        let keys = agent.own_property_keys(object).unwrap();
        assert_eq!(keys.len(), before.len());
        for &(key, was) in before {
            let now = agent.get_own_property(object, key).unwrap().unwrap();
            assert!(!now.configurable, "{key:?}");
            assert_eq!(now.enumerable, was.enumerable, "{key:?}");
            match (was.slot, now.slot) {
                (
                    Slot::Data { value, writable },
                    Slot::Data {
                        value: now_value,
                        writable: now_writable,
                    },
                ) => {
                    assert_eq!(now_value, value, "{key:?}");
                    let expected = writable && level == IntegrityLevel::Sealed;
                    assert_eq!(now_writable, expected, "{key:?}");
                }
                (accessor @ Slot::Accessor { .. }, now @ Slot::Accessor { .. }) => {
                    assert_eq!(now, accessor, "{key:?}");
                }
                (was, now) => panic!("{key:?}: {was:?} became {now:?}"),
            }
        }
    }

    /// The own properties of `object`, in the order of its keys.
    fn own_properties(agent: &mut Agent<NoHooks>, object: ObjectRef) -> Vec<(Key, Property)> {
        let keys = agent.own_property_keys(object).unwrap();
        keys.into_iter()
            .map(|key| (key, agent.get_own_property(object, key).unwrap().unwrap()))
            .collect()
    }

    /// SetIntegrityLevel (`#sec-setintegritylevel`) of each level, of an ordinary object with
    /// a data, an index, a Symbol and an accessor property, of an Array, of an unmapped
    /// arguments object (whose "callee" is an accessor of %ThrowTypeError%) and of a mapped
    /// one made by a script: it answers true; after it [[IsExtensible]] is false, every own
    /// property is non-configurable, a data property is non-writable when the level is frozen
    /// and keeps its value, and an accessor keeps its getter and setter.
    #[test]
    fn set_integrity_level_seals_and_freezes_an_object_an_array_and_arguments() {
        let (mut agent, realm) = agent_with_realm();
        enter(&mut agent, &realm);
        let handle = agent.heap.roots.get_realm(&realm.0).unwrap();
        let getter = agent
            .create_host_function(handle, "five", 0, five, false, None)
            .unwrap();
        agent.keep(Value::Object(getter));
        for level in [IntegrityLevel::Sealed, IntegrityLevel::Frozen] {
            let object = plain(&mut agent);
            let a = agent.name_key("a").unwrap();
            agent.keep_key(a);
            let accessor = agent.name_key("accessor").unwrap();
            agent.keep_key(accessor);
            let symbol = Key::Symbol(agent.heap.new_symbol(None).unwrap());
            agent.keep_key(symbol);
            for key in [a, Key::Index(3), symbol] {
                agent
                    .create_data_property(object, key, Value::Number(1.0))
                    .unwrap();
            }
            agent
                .define_own_property(
                    object,
                    accessor,
                    Descriptor {
                        get: Some(Some(getter)),
                        set: Some(Some(getter)),
                        enumerable: Some(true),
                        configurable: Some(true),
                        ..Descriptor::default()
                    },
                )
                .unwrap();

            let array = agent.array_create(0.0, None).unwrap();
            agent.keep(Value::Object(array));
            for index in 0..3 {
                agent
                    .create_data_property(array, Key::Index(index), Value::Number(f64::from(index)))
                    .unwrap();
            }

            let unmapped = agent
                .create_arguments_object(
                    handle,
                    getter,
                    &[Value::Number(1.0), Value::Number(2.0)],
                    None,
                )
                .unwrap();
            agent.keep(Value::Object(unmapped));
            let mapped = script_object(
                &mut agent,
                &realm,
                "(function (p, q) { return arguments; })(1, 2);",
            );

            for subject in [object, array, unmapped, mapped] {
                let before = own_properties(&mut agent, subject);
                assert_eq!(agent.set_integrity_level(subject, level), Ok(true));
                check_level(&mut agent, subject, level, &before);
            }
            // The mapped arguments object still reads its arguments.
            assert_eq!(agent.get(mapped, Key::Index(0)), Ok(Value::Number(1.0)));
            assert_eq!(agent.get(mapped, Key::Index(1)), Ok(Value::Number(2.0)));
        }
    }

    /// `#sec-setintegritylevel` with the Array exotic [[DefineOwnProperty]]
    /// (`#sec-array-exotic-objects-defineownproperty-p-desc`): a frozen Array refuses a Set of
    /// an element and of "length" (a TypeError when Set throws), the definition of a new
    /// element and the deletion of an element, and keeps its elements and its length.
    #[test]
    fn a_frozen_array_refuses_a_set_a_define_and_a_delete_and_keeps_its_elements() {
        let (mut agent, realm) = agent_with_realm();
        enter(&mut agent, &realm);
        let array = agent.array_create(0.0, None).unwrap();
        agent.keep(Value::Object(array));
        for (index, value) in [(0, 10.0), (1, 20.0)] {
            agent
                .create_data_property(array, Key::Index(index), Value::Number(value))
                .unwrap();
        }
        assert_eq!(
            agent.set_integrity_level(array, IntegrityLevel::Frozen),
            Ok(true)
        );
        let length = agent.name_key("length").unwrap();
        assert_eq!(
            agent.set(array, Key::Index(0), Value::Number(99.0), false),
            Ok(false)
        );
        let thrown = agent.set(array, Key::Index(0), Value::Number(99.0), true);
        assert!(is_type_error(&mut agent, thrown));
        assert_eq!(
            agent.set(array, length, Value::Number(0.0), false),
            Ok(false)
        );
        assert_eq!(
            agent.define_own_property(
                array,
                Key::Index(2),
                Descriptor::data(Value::Number(30.0), true, true, true)
            ),
            Ok(false)
        );
        assert_eq!(
            agent.create_data_property(array, Key::Index(0), Value::Null),
            Ok(false)
        );
        assert_eq!(agent.internal_delete(array, Key::Index(1)), Ok(false));
        assert_eq!(agent.get(array, Key::Index(0)), Ok(Value::Number(10.0)));
        assert_eq!(agent.get(array, Key::Index(1)), Ok(Value::Number(20.0)));
        assert_eq!(agent.has_property(array, Key::Index(2)), Ok(false));
        assert_eq!(agent.length_of_array_like(array), Ok(2.0));
    }

    /// TestIntegrityLevel (`#sec-testintegritylevel`): an extensible object is neither sealed
    /// nor frozen, even when it has no property; an empty non-extensible object is both; one
    /// with a configurable property is neither; sealed with a writable data property it is
    /// sealed and not frozen; sealed with only an accessor it is both; frozen it is both.
    #[test]
    fn test_integrity_level_answers_for_each_level_and_for_an_empty_non_extensible_object() {
        let (mut agent, realm) = agent_with_realm();
        enter(&mut agent, &realm);
        let handle = agent.heap.roots.get_realm(&realm.0).unwrap();
        let levels = |agent: &mut Agent<NoHooks>, object| {
            (
                agent.test_integrity_level(object, IntegrityLevel::Sealed),
                agent.test_integrity_level(object, IntegrityLevel::Frozen),
            )
        };
        let empty = plain(&mut agent);
        assert_eq!(levels(&mut agent, empty), (Ok(false), Ok(false)));
        assert_eq!(agent.prevent_extensions(empty), Ok(true));
        assert_eq!(levels(&mut agent, empty), (Ok(true), Ok(true)));

        let data = plain(&mut agent);
        let a = agent.name_key("a").unwrap();
        agent.keep_key(a);
        agent.create_data_property(data, a, Value::Null).unwrap();
        assert_eq!(levels(&mut agent, data), (Ok(false), Ok(false)));
        assert_eq!(agent.prevent_extensions(data), Ok(true));
        assert_eq!(levels(&mut agent, data), (Ok(false), Ok(false)));
        assert_eq!(
            agent.set_integrity_level(data, IntegrityLevel::Sealed),
            Ok(true)
        );
        assert_eq!(levels(&mut agent, data), (Ok(true), Ok(false)));
        assert_eq!(
            agent.set_integrity_level(data, IntegrityLevel::Frozen),
            Ok(true)
        );
        assert_eq!(levels(&mut agent, data), (Ok(true), Ok(true)));

        let getter = agent
            .create_host_function(handle, "five", 0, five, false, None)
            .unwrap();
        agent.keep(Value::Object(getter));
        let accessor = plain(&mut agent);
        agent
            .define_own_property(
                accessor,
                a,
                Descriptor {
                    get: Some(Some(getter)),
                    set: Some(None),
                    enumerable: Some(true),
                    configurable: Some(true),
                    ..Descriptor::default()
                },
            )
            .unwrap();
        assert_eq!(
            agent.set_integrity_level(accessor, IntegrityLevel::Sealed),
            Ok(true)
        );
        assert_eq!(levels(&mut agent, accessor), (Ok(true), Ok(true)));
    }

    /// An ordinary object of the current realm with `count` data properties "k0", "k1", ...
    fn with_keys(agent: &mut Agent<NoHooks>, count: u32) -> ObjectRef {
        let object = plain(agent);
        for index in 0..count {
            let key = agent.name_key(&format!("k{index}")).unwrap();
            agent.keep_key(key);
            agent
                .create_data_property(object, key, Value::Number(f64::from(index)))
                .unwrap();
        }
        object
    }

    /// The steps `run` spends.
    fn steps_of(agent: &mut Agent<NoHooks>, run: impl FnOnce(&mut Agent<NoHooks>)) -> u64 {
        let before = agent.last_entry_steps().unwrap();
        run(agent);
        agent.last_entry_steps().unwrap() - before
    }

    /// SetIntegrityLevel and TestIntegrityLevel charge a step per key (decision 2), beside the
    /// step per key [[OwnPropertyKeys]] charges (`object/ordinary.rs`): of an object with `n`
    /// keys each spends 2n steps, read from `last_entry_steps()`, for n of 0, 4 and 12, and
    /// TestIntegrityLevel of a frozen object reads every key.
    #[test]
    fn integrity_levels_charge_a_step_per_key() {
        let (mut agent, realm) = agent_with_realm();
        enter(&mut agent, &realm);
        for count in [0_u32, 4, 12] {
            for level in [IntegrityLevel::Sealed, IntegrityLevel::Frozen] {
                let object = with_keys(&mut agent, count);
                let set = steps_of(&mut agent, |agent| {
                    assert_eq!(agent.set_integrity_level(object, level), Ok(true));
                });
                assert_eq!(set, 2 * u64::from(count), "set {level:?} of {count}");
                let tested = steps_of(&mut agent, |agent| {
                    assert_eq!(agent.test_integrity_level(object, level), Ok(true));
                });
                assert_eq!(tested, 2 * u64::from(count), "test {level:?} of {count}");
            }
        }
    }

    /// The step budget ends a freeze of more keys than it has steps for: in an entry with
    /// `steps_per_entry` 45, a freeze of 30 keys (30 steps for [[OwnPropertyKeys]], then one
    /// per key) ends in `Terminated(StepBudget)` at the 16th key, with the counter at the
    /// budget and not past it; so does TestIntegrityLevel of a frozen object of 30 keys.
    #[test]
    fn step_budget_ends_a_freeze_of_many_keys() {
        for test in [false, true] {
            let (mut agent, realm) = agent_with_realm();
            enter(&mut agent, &realm);
            let object = with_keys(&mut agent, 30);
            if test {
                assert_eq!(
                    agent.set_integrity_level(object, IntegrityLevel::Frozen),
                    Ok(true)
                );
            }
            // A new entry with a budget of 45 steps.
            agent.limits.steps_per_entry = 45;
            agent.steps = 0;
            let ended = if test {
                agent.test_integrity_level(object, IntegrityLevel::Frozen)
            } else {
                agent.set_integrity_level(object, IntegrityLevel::Frozen)
            };
            assert_eq!(ended, Err(Abrupt::Terminated(Termination::StepBudget)));
            assert_eq!(agent.last_entry_steps(), Ok(45));
        }
    }

    /// A function that answers its this value when its arguments are 1 and 2, else undefined.
    fn this_of(_cx: &mut Cx<'_, NoHooks>, call: HostCall<'_>) -> Completion<Value> {
        if call.args == [Value::Number(1.0), Value::Number(2.0)] {
            return Ok(call.this);
        }
        Ok(Value::Undefined)
    }

    /// Invoke (`#sec-invoke`): the function is found by GetV, through the prototype of a
    /// Number and of a String, and is called with the primitive itself as the this value
    /// (not its wrapper) and with the arguments; of an object, with the object.
    #[test]
    fn invoke_gets_from_a_primitive_and_calls_with_it_as_this() {
        let (mut agent, realm) = agent_with_realm();
        enter(&mut agent, &realm);
        let handle = agent.heap.roots.get_realm(&realm.0).unwrap();
        let function = agent
            .create_host_function(handle, "thisOf", 0, this_of, false, None)
            .unwrap();
        agent.keep(Value::Object(function));
        let key = agent.name_key("thisOf").unwrap();
        agent.keep_key(key);
        for prototype in [
            Intrinsic::NumberPrototype,
            Intrinsic::StringPrototype,
            Intrinsic::ObjectPrototype,
        ] {
            let prototype = agent.intrinsic(handle, prototype).unwrap();
            agent
                .create_data_property(prototype, key, Value::Object(function))
                .unwrap();
        }
        let string = Value::String(agent.new_string_str("s").unwrap());
        agent.keep(string);
        let object = Value::Object(plain(&mut agent));
        let args = [Value::Number(1.0), Value::Number(2.0)];
        for value in [Value::Number(7.0), string, object] {
            assert_eq!(agent.invoke(value, key, &args), Ok(value));
        }
        assert_eq!(
            agent.invoke(Value::Number(7.0), key, &[]),
            Ok(Value::Undefined)
        );
    }

    /// Invoke (`#sec-invoke`) of a property that is not callable (absent, or a Number) throws
    /// a TypeError at Call; of undefined and null, at GetV's ToObject.
    #[test]
    fn invoke_of_what_is_not_callable_throws_type_error() {
        let (mut agent, realm) = agent_with_realm();
        enter(&mut agent, &realm);
        let object = plain(&mut agent);
        let [missing, number] = ["missing", "number"].map(|name| agent.name_key(name).unwrap());
        agent.keep_key(missing);
        agent.keep_key(number);
        agent
            .create_data_property(object, number, Value::Number(3.0))
            .unwrap();
        for (value, key) in [
            (Value::Object(object), missing),
            (Value::Object(object), number),
            (Value::Number(1.0), missing),
            (Value::Undefined, missing),
            (Value::Null, number),
        ] {
            let thrown = agent.invoke(value, key, &[]);
            assert!(is_type_error(&mut agent, thrown), "{value:?} {key:?}");
        }
    }

    /// The text of a String value.
    fn text(agent: &mut Agent<NoHooks>, value: Value) -> String {
        let Value::String(string) = value else {
            panic!("{value:?} is not a String");
        };
        String::from_utf16_lossy(&agent.string_units(string).unwrap())
    }

    /// The elements of an Array of two, an entry of key+value.
    fn entry(agent: &mut Agent<NoHooks>, value: Value) -> (String, Value) {
        let Value::Object(array) = value else {
            panic!("{value:?} is not an entry");
        };
        assert_eq!(agent.is_array(value), Ok(true));
        assert_eq!(agent.length_of_array_like(array), Ok(2.0));
        let key = agent.get(array, Key::Index(0)).unwrap();
        let value = agent.get(array, Key::Index(1)).unwrap();
        (text(agent, key), value)
    }

    /// EnumerableOwnProperties (`#sec-enumerableownproperties`) of each kind: of an object with
    /// an index key, an enumerable data and accessor property, a non-enumerable one and an
    /// enumerable Symbol-keyed one, the String keys whose property is enumerable, in the order
    /// of [[OwnPropertyKeys]]; kind key answers them as property keys and as Strings, kind value
    /// their values (a getter's answer for the accessor), kind key+value an Array of each key
    /// and value. Of a String object, its indices.
    #[test]
    fn enumerable_own_properties_of_each_kind_skip_symbols_and_what_is_not_enumerable() {
        let (mut agent, realm) = agent_with_realm();
        enter(&mut agent, &realm);
        let handle = agent.heap.roots.get_realm(&realm.0).unwrap();
        let getter = agent
            .create_host_function(handle, "five", 0, five, false, None)
            .unwrap();
        agent.keep(Value::Object(getter));
        let object = plain(&mut agent);
        let [a, hidden, c, d] = ["a", "hidden", "c", "d"].map(|name| agent.name_key(name).unwrap());
        let symbol = Key::Symbol(agent.heap.new_symbol(None).unwrap());
        for key in [a, hidden, c, d, symbol] {
            agent.keep_key(key);
        }
        agent
            .create_data_property(object, a, Value::Number(1.0))
            .unwrap();
        agent
            .define_own_property(
                object,
                hidden,
                Descriptor::data(Value::Number(2.0), true, false, true),
            )
            .unwrap();
        agent
            .create_data_property(object, symbol, Value::Number(3.0))
            .unwrap();
        agent
            .define_own_property(
                object,
                c,
                Descriptor {
                    get: Some(Some(getter)),
                    set: Some(None),
                    enumerable: Some(true),
                    configurable: Some(true),
                    ..Descriptor::default()
                },
            )
            .unwrap();
        agent
            .create_data_property(object, d, Value::Number(4.0))
            .unwrap();
        agent
            .create_data_property(object, Key::Index(1), Value::Number(0.5))
            .unwrap();

        let mark = agent.kept.len();
        assert_eq!(
            agent.enumerable_own_keys(object),
            Ok(vec![Key::Index(1), a, c, d])
        );
        let keys = agent
            .enumerable_own_properties(object, PropertyKind::Key)
            .unwrap();
        let keys: Vec<String> = keys.into_iter().map(|key| text(&mut agent, key)).collect();
        assert_eq!(keys, ["1", "a", "c", "d"]);
        assert_eq!(
            agent.enumerable_own_properties(object, PropertyKind::Value),
            Ok(vec![
                Value::Number(0.5),
                Value::Number(1.0),
                Value::Number(5.0),
                Value::Number(4.0)
            ])
        );
        let entries = agent
            .enumerable_own_properties(object, PropertyKind::KeyValue)
            .unwrap();
        let entries: Vec<(String, Value)> = entries
            .into_iter()
            .map(|value| entry(&mut agent, value))
            .collect();
        assert_eq!(
            entries,
            [
                ("1".to_string(), Value::Number(0.5)),
                ("a".to_string(), Value::Number(1.0)),
                ("c".to_string(), Value::Number(5.0)),
                ("d".to_string(), Value::Number(4.0)),
            ]
        );
        agent.release(mark);

        let prototype = agent.current_intrinsic(Intrinsic::StringPrototype).unwrap();
        let ab = agent.new_string_str("ab").unwrap();
        let string = agent.string_create(ab, prototype).unwrap();
        agent.keep(Value::Object(string));
        assert_eq!(
            agent.enumerable_own_keys(string),
            Ok(vec![Key::Index(0), Key::Index(1)])
        );
    }

    /// A getter that deletes "b" of its this value and answers 1.
    fn delete_b(cx: &mut Cx<'_, NoHooks>, call: HostCall<'_>) -> Completion<Value> {
        let agent = &mut *cx.agent;
        let Value::Object(this) = call.this else {
            return Ok(Value::Undefined);
        };
        let key = agent.name_key("b")?;
        agent.internal_delete(this, key)?;
        Ok(Value::Number(1.0))
    }

    /// `#sec-enumerableownproperties`, step 3.a.i: each key is read again by [[GetOwnProperty]]
    /// when its turn comes, so with kinds value and key+value a property that the getter of an
    /// earlier one deleted is skipped; the keys and the results are kept across the getter's
    /// call (run also with the feature `collect-every-safe-point`).
    #[test]
    fn enumerable_own_properties_skip_a_key_a_getter_deleted() {
        let (mut agent, realm) = agent_with_realm();
        enter(&mut agent, &realm);
        let handle = agent.heap.roots.get_realm(&realm.0).unwrap();
        let getter = agent
            .create_host_function(handle, "deleteB", 0, delete_b, false, None)
            .unwrap();
        agent.keep(Value::Object(getter));
        for kind in [PropertyKind::Value, PropertyKind::KeyValue] {
            let object = plain(&mut agent);
            let a = agent.name_key("a").unwrap();
            agent
                .define_own_property(
                    object,
                    a,
                    Descriptor {
                        get: Some(Some(getter)),
                        set: Some(None),
                        enumerable: Some(true),
                        configurable: Some(true),
                        ..Descriptor::default()
                    },
                )
                .unwrap();
            for (name, value) in [("b", 2.0), ("c", 3.0)] {
                let key = agent.name_key(name).unwrap();
                agent
                    .create_data_property(object, key, Value::Number(value))
                    .unwrap();
            }
            let mark = agent.kept.len();
            let results = agent.enumerable_own_properties(object, kind).unwrap();
            match kind {
                PropertyKind::Value => {
                    assert_eq!(results, [Value::Number(1.0), Value::Number(3.0)]);
                }
                _ => {
                    let entries: Vec<(String, Value)> = results
                        .into_iter()
                        .map(|value| entry(&mut agent, value))
                        .collect();
                    assert_eq!(
                        entries,
                        [
                            ("a".to_string(), Value::Number(1.0)),
                            ("c".to_string(), Value::Number(3.0)),
                        ]
                    );
                }
            }
            agent.release(mark);
            let b = agent.name_key("b").unwrap();
            assert_eq!(agent.has_own_property(object, b), Ok(false));
        }
    }

    /// CreateDataPropertyOrThrow (`#sec-createdatapropertyorthrow`): on an extensible object it
    /// defines a writable, enumerable, configurable data property; on a non-extensible object,
    /// and over a non-configurable property, it throws a TypeError and defines nothing.
    #[test]
    fn create_data_property_or_throw_throws_on_a_non_extensible_object() {
        let (mut agent, realm) = agent_with_realm();
        enter(&mut agent, &realm);
        let object = plain(&mut agent);
        let [a, b] = ["a", "b"].map(|name| agent.name_key(name).unwrap());
        agent.keep_key(a);
        agent.keep_key(b);
        assert_eq!(
            agent.create_data_property_or_throw(object, a, Value::Number(1.0)),
            Ok(())
        );
        assert_eq!(
            agent.get_own_property(object, a),
            Ok(Some(Property::data(Value::Number(1.0), true, true, true)))
        );
        agent
            .define_own_property(object, a, Descriptor::data(Value::Null, false, true, false))
            .unwrap();
        let thrown = agent.create_data_property_or_throw(object, a, Value::Number(2.0));
        assert!(is_type_error(&mut agent, thrown));
        assert_eq!(agent.get(object, a), Ok(Value::Null));
        assert_eq!(agent.prevent_extensions(object), Ok(true));
        let thrown = agent.create_data_property_or_throw(object, b, Value::Number(2.0));
        assert!(is_type_error(&mut agent, thrown));
        assert_eq!(agent.has_own_property(object, b), Ok(false));
    }
}
