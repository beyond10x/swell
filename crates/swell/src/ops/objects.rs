//! Operations on objects (ECMA-262 7.3, `#sec-operations-on-objects`): Get, GetV, GetMethod,
//! Set, CreateDataProperty, DefinePropertyOrThrow, DeletePropertyOrThrow, HasProperty (the
//! internal method of `object/mod.rs`), HasOwnProperty, Call, Construct, LengthOfArrayLike,
//! GetFunctionRealm, OrdinaryHasInstance and InstanceofOperator (13.10.2), and
//! EnumerateObjectProperties (`#sec-enumerate-object-properties`). Written by
//! `story:object-model-and-realms`.

use std::collections::BTreeSet;

use crate::heap::arena::Handle;
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::host::{ErrorKind, Host};
use crate::object::property::{Descriptor, Key};
use crate::object::{KindTag, ObjectKind};
use crate::value::{ObjectRef, Value};

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
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "Object.prototype.hasOwnProperty: story:harness-builtins"
        )
    )]
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

    /// LengthOfArrayLike (`#sec-lengthofarraylike`).
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "Function.prototype.apply: story:harness-builtins")
    )]
    pub(crate) fn length_of_array_like(&mut self, object: ObjectRef) -> Completion<f64> {
        let key = self.name_key("length")?;
        let length = self.get(object, key)?;
        self.to_length(length)
    }

    /// GetFunctionRealm (`#sec-getfunctionrealm`): a built-in function's [[Realm]], else the
    /// current realm. Bound functions and proxies come with their stories.
    pub(crate) fn get_function_realm(&mut self, object: ObjectRef) -> Completion<Handle> {
        if let ObjectKind::BuiltinFunction(function) = &self.engine_object(object)?.kind
            && let Some(realm) = function.realm
        {
            return Ok(realm);
        }
        self.current_realm()
    }

    /// OrdinaryHasInstance (`#sec-ordinaryhasinstance`). A step per object of the chain.
    pub(crate) fn ordinary_has_instance(
        &mut self,
        constructor: Value,
        value: Value,
    ) -> Completion<bool> {
        if !self.is_callable(constructor)? {
            return Ok(false);
        }
        let Value::Object(constructor) = constructor else {
            return Ok(false);
        };
        if self.kind_of(constructor)? == KindTag::BoundFunction {
            // Step 2 needs [[BoundTargetFunction]], which comes with story:harness-builtins;
            // no bound function exists before it.
            return Err(self.fault());
        }
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
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "instanceof: story:interpreter-and-script-entry-points"
        )
    )]
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
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "for-in: story:interpreter-and-script-entry-points"
        )
    )]
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
    use crate::host::Host;
    use crate::host::agent::Agent;
    use crate::host::hooks::test_host::{NoHooks, agent_with_realm, enter};
    use crate::host::outcome::{Abrupt, Completion};
    use crate::host::{Cx, HostCall};
    use crate::object::ordinary::OrdinaryData;
    use crate::object::property::{Descriptor, Key};
    use crate::value::{ObjectRef, Value};

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
    /// (`#sec-ordinaryhasinstance`): no @@hasInstance method exists in this part, so the
    /// prototype chain of the value is searched for the constructor's "prototype"; a
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
}
