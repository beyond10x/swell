//! Ordinary objects (ECMA-262 10.1, `#sec-ordinary-object-internal-methods-and-internal-slots`):
//! every ordinary internal method, ValidateAndApplyPropertyDescriptor, OrdinaryObjectCreate and
//! OrdinaryCreateFromConstructor, with the slots of the ordinary objects of this part: the
//! Boolean and Number objects ([[BooleanData]], [[NumberData]]), the Error objects
//! ([[ErrorData]]) and `%Object.prototype%`, an immutable prototype exotic object (10.4.7) whose
//! other methods are ordinary. Written by `story:object-model-and-realms`.
//!
//! A walk up the prototype chain is a loop, never a Rust recursion, and charges one step per
//! object it passes (decision 2): no proxy exists before slice 9, so every [[Get]],
//! [[HasProperty]] and [[Set]] of the chain is the ordinary one.

use super::ObjectKind;
use super::property::{Descriptor, Key, Property, Slot};
use crate::heap::collect::Mark;
use crate::host::Host;
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::realm::intrinsics::Intrinsic;
use crate::value::{ObjectRef, Value};

/// What an ordinary object holds beyond the common record.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum OrdinaryData {
    Plain,
    /// `%Object.prototype%` (10.4.7).
    ImmutablePrototype,
    /// [[BooleanData]].
    Boolean(bool),
    /// [[NumberData]].
    Number(f64),
    /// [[ErrorData]].
    Error,
}

/// The slots of an ordinary object beyond the common record.
pub(crate) struct Ordinary {
    pub(crate) data: OrdinaryData,
}

impl Ordinary {
    /// An ordinary object with `data`.
    pub(crate) fn new(data: OrdinaryData) -> Ordinary {
        Ordinary { data }
    }

    /// Kind site of the collector: the slots of this part hold no handle.
    pub(crate) fn trace(&self, mark: &mut Mark) {
        mark.count("object/ordinary.rs Ordinary::trace");
    }

    /// Whether this is `%Object.prototype%`.
    pub(crate) fn has_immutable_prototype(&self) -> bool {
        self.data == OrdinaryData::ImmutablePrototype
    }

    /// An ordinary object, for the tests of the collector.
    #[cfg(test)]
    pub(crate) fn for_test() -> Ordinary {
        Ordinary::new(OrdinaryData::Plain)
    }
}

/// The property ValidateAndApplyPropertyDescriptor (`#sec-validateandapplypropertydescriptor`)
/// leaves once `descriptor` is valid: step 2.c of a new one (`current` absent; every absent
/// field its default) or step 6 of a change of `current` (every absent field kept, a change of
/// kind keeping only the two attributes).
pub(crate) fn applied_property(descriptor: &Descriptor, current: Option<Property>) -> Property {
    let Some(current) = current else {
        let slot = if descriptor.is_accessor() {
            Slot::Accessor {
                get: descriptor.get.flatten(),
                set: descriptor.set.flatten(),
            }
        } else {
            Slot::Data {
                value: descriptor.value.unwrap_or(Value::Undefined),
                writable: descriptor.writable.unwrap_or(false),
            }
        };
        return Property {
            slot,
            enumerable: descriptor.enumerable.unwrap_or(false),
            configurable: descriptor.configurable.unwrap_or(false),
        };
    };
    let slot = match current.slot {
        Slot::Data { .. } if descriptor.is_accessor() => Slot::Accessor {
            get: descriptor.get.flatten(),
            set: descriptor.set.flatten(),
        },
        Slot::Accessor { .. } if descriptor.is_data() => Slot::Data {
            value: descriptor.value.unwrap_or(Value::Undefined),
            writable: descriptor.writable.unwrap_or(false),
        },
        Slot::Data { value, writable } => Slot::Data {
            value: descriptor.value.unwrap_or(value),
            writable: descriptor.writable.unwrap_or(writable),
        },
        Slot::Accessor { get, set } => Slot::Accessor {
            get: descriptor.get.unwrap_or(get),
            set: descriptor.set.unwrap_or(set),
        },
    };
    Property {
        slot,
        enumerable: descriptor.enumerable.unwrap_or(current.enumerable),
        configurable: descriptor.configurable.unwrap_or(current.configurable),
    }
}

impl<H: Host> Agent<H> {
    /// OrdinaryGetPrototypeOf (`#sec-ordinarygetprototypeof`).
    pub(crate) fn ordinary_get_prototype_of(
        &mut self,
        object: ObjectRef,
    ) -> Completion<Option<ObjectRef>> {
        Ok(self.engine_object(object)?.prototype)
    }

    /// OrdinarySetPrototypeOf (`#sec-ordinarysetprototypeof`). The loop of step 7 charges a step
    /// per object.
    pub(crate) fn ordinary_set_prototype_of(
        &mut self,
        object: ObjectRef,
        prototype: Option<ObjectRef>,
    ) -> Completion<bool> {
        let (current, extensible) = {
            let record = self.engine_object(object)?;
            (record.prototype, record.extensible)
        };
        if prototype == current {
            return Ok(true);
        }
        if !extensible {
            return Ok(false);
        }
        let mut p = prototype;
        while let Some(next) = p {
            if next == object {
                return Ok(false);
            }
            self.charge_steps(1)?;
            p = self.engine_object(next)?.prototype;
        }
        self.engine_object_mut(object)?.prototype = prototype;
        Ok(true)
    }

    /// OrdinaryIsExtensible (`#sec-ordinaryisextensible`).
    pub(crate) fn ordinary_is_extensible(&mut self, object: ObjectRef) -> Completion<bool> {
        Ok(self.engine_object(object)?.extensible)
    }

    /// OrdinaryPreventExtensions (`#sec-ordinarypreventextensions`).
    pub(crate) fn ordinary_prevent_extensions(&mut self, object: ObjectRef) -> Completion<bool> {
        self.engine_object_mut(object)?.extensible = false;
        Ok(true)
    }

    /// OrdinaryGetOwnProperty (`#sec-ordinarygetownproperty`).
    pub(crate) fn ordinary_get_own_property(
        &mut self,
        object: ObjectRef,
        key: Key,
    ) -> Completion<Option<Property>> {
        Ok(self.engine_object(object)?.properties.get(key).copied())
    }

    /// OrdinaryDefineOwnProperty (`#sec-ordinarydefineownproperty`).
    pub(crate) fn ordinary_define_own_property(
        &mut self,
        object: ObjectRef,
        key: Key,
        descriptor: Descriptor,
    ) -> Completion<bool> {
        let current = self.get_own_property(object, key)?;
        let extensible = self.is_extensible(object)?;
        self.validate_and_apply_property_descriptor(
            Some(object),
            key,
            extensible,
            descriptor,
            current,
        )
    }

    /// ValidateAndApplyPropertyDescriptor (`#sec-validateandapplypropertydescriptor`).
    pub(crate) fn validate_and_apply_property_descriptor(
        &mut self,
        object: Option<ObjectRef>,
        key: Key,
        extensible: bool,
        descriptor: Descriptor,
        current: Option<Property>,
    ) -> Completion<bool> {
        // 2. If current is undefined, then
        let Some(current) = current else {
            if !extensible {
                return Ok(false);
            }
            let Some(object) = object else {
                return Ok(true);
            };
            self.add_property(object, key, applied_property(&descriptor, None))?;
            return Ok(true);
        };
        // 4. If Desc does not have any fields, return true.
        if descriptor == Descriptor::default() {
            return Ok(true);
        }
        // 5. If current.[[Configurable]] is false, then
        if !current.configurable {
            if descriptor.configurable == Some(true) {
                return Ok(false);
            }
            if descriptor
                .enumerable
                .is_some_and(|enumerable| enumerable != current.enumerable)
            {
                return Ok(false);
            }
            let current_is_accessor = matches!(current.slot, Slot::Accessor { .. });
            if !descriptor.is_generic() && descriptor.is_accessor() != current_is_accessor {
                return Ok(false);
            }
            match current.slot {
                Slot::Accessor { get, set } => {
                    if descriptor.get.is_some_and(|new| new != get)
                        || descriptor.set.is_some_and(|new| new != set)
                    {
                        return Ok(false);
                    }
                }
                Slot::Data {
                    value,
                    writable: false,
                } => {
                    if descriptor.writable == Some(true) {
                        return Ok(false);
                    }
                    if let Some(new) = descriptor.value
                        && !self.same_value(new, value)?
                    {
                        return Ok(false);
                    }
                }
                Slot::Data { .. } => {}
            }
        }
        // 6. If O is not undefined, then
        if let Some(object) = object {
            let applied = applied_property(&descriptor, Some(current));
            let record = self.engine_object_mut(object)?;
            if let Some(property) = record.properties.get_mut(key) {
                *property = applied;
            }
        }
        Ok(true)
    }

    /// OrdinaryHasProperty (`#sec-ordinaryhasproperty`), as a loop over the chain.
    pub(crate) fn ordinary_has_property(
        &mut self,
        object: ObjectRef,
        key: Key,
    ) -> Completion<bool> {
        let mut o = object;
        loop {
            if self.get_own_property(o, key)?.is_some() {
                return Ok(true);
            }
            match self.get_prototype_of(o)? {
                Some(parent) => {
                    self.charge_steps(1)?;
                    o = parent;
                }
                None => return Ok(false),
            }
        }
    }

    /// OrdinaryGet (`#sec-ordinaryget`), as a loop over the chain.
    pub(crate) fn ordinary_get(
        &mut self,
        object: ObjectRef,
        key: Key,
        receiver: Value,
    ) -> Completion<Value> {
        let mut o = object;
        loop {
            match self.get_own_property(o, key)? {
                None => match self.get_prototype_of(o)? {
                    Some(parent) => {
                        self.charge_steps(1)?;
                        o = parent;
                    }
                    None => return Ok(Value::Undefined),
                },
                Some(property) => {
                    return match property.slot {
                        Slot::Data { value, .. } => Ok(value),
                        Slot::Accessor { get: None, .. } => Ok(Value::Undefined),
                        Slot::Accessor {
                            get: Some(getter), ..
                        } => self.call_value(Value::Object(getter), receiver, &[]),
                    };
                }
            }
        }
    }

    /// OrdinarySet (`#sec-ordinaryset`) with OrdinarySetWithOwnDescriptor
    /// (`#sec-ordinarysetwithowndescriptor`), as a loop over the chain.
    pub(crate) fn ordinary_set(
        &mut self,
        object: ObjectRef,
        key: Key,
        value: Value,
        receiver: Value,
    ) -> Completion<bool> {
        let mut o = object;
        let own = loop {
            match self.get_own_property(o, key)? {
                Some(property) => break property,
                None => match self.get_prototype_of(o)? {
                    Some(parent) => {
                        self.charge_steps(1)?;
                        o = parent;
                    }
                    None => break Property::data(Value::Undefined, true, true, true),
                },
            }
        };
        match own.slot {
            Slot::Data { writable, .. } => {
                if !writable {
                    return Ok(false);
                }
                let Value::Object(receiver) = receiver else {
                    return Ok(false);
                };
                match self.get_own_property(receiver, key)? {
                    Some(existing) => {
                        let Slot::Data { writable, .. } = existing.slot else {
                            return Ok(false);
                        };
                        if !writable {
                            return Ok(false);
                        }
                        let value_only = Descriptor {
                            value: Some(value),
                            ..Descriptor::default()
                        };
                        self.define_own_property(receiver, key, value_only)
                    }
                    None => self.create_data_property(receiver, key, value),
                }
            }
            Slot::Accessor { set: None, .. } => Ok(false),
            Slot::Accessor {
                set: Some(setter), ..
            } => {
                self.call_value(Value::Object(setter), receiver, &[value])?;
                Ok(true)
            }
        }
    }

    /// OrdinaryDelete (`#sec-ordinarydelete`).
    pub(crate) fn ordinary_delete(&mut self, object: ObjectRef, key: Key) -> Completion<bool> {
        match self.get_own_property(object, key)? {
            None => Ok(true),
            Some(property) if property.configurable => {
                self.engine_object_mut(object)?.properties.remove(key);
                Ok(true)
            }
            Some(_) => Ok(false),
        }
    }

    /// OrdinaryOwnPropertyKeys (`#sec-ordinaryownpropertykeys`): a step per key, and a step per
    /// comparison when the indices were not added ascending and are sorted.
    pub(crate) fn ordinary_own_property_keys(&mut self, object: ObjectRef) -> Completion<Vec<Key>> {
        let (keys, comparisons) = self.engine_object(object)?.properties.keys();
        let steps = u64::try_from(keys.len()).unwrap_or(u64::MAX);
        self.charge_steps(steps.saturating_add(comparisons))?;
        Ok(keys)
    }

    /// OrdinaryObjectCreate (`#sec-ordinaryobjectcreate`) of an ordinary object with `data`.
    pub(crate) fn ordinary_object_create(
        &mut self,
        prototype: Option<ObjectRef>,
        data: OrdinaryData,
    ) -> Completion<ObjectRef> {
        self.allocate_object(prototype, ObjectKind::Ordinary(Ordinary::new(data)))
    }

    /// OrdinaryCreateFromConstructor (`#sec-ordinarycreatefromconstructor`).
    pub(crate) fn ordinary_create_from_constructor(
        &mut self,
        constructor: ObjectRef,
        default: Intrinsic,
        data: OrdinaryData,
    ) -> Completion<ObjectRef> {
        let prototype = self.get_prototype_from_constructor(constructor, default)?;
        self.ordinary_object_create(Some(prototype), data)
    }

    /// GetPrototypeFromConstructor (`#sec-getprototypefromconstructor`).
    pub(crate) fn get_prototype_from_constructor(
        &mut self,
        constructor: ObjectRef,
        default: Intrinsic,
    ) -> Completion<ObjectRef> {
        let key = self.name_key("prototype")?;
        match self.get(constructor, key)? {
            Value::Object(prototype) => Ok(prototype),
            _ => {
                let realm = self.get_function_realm(constructor)?;
                self.intrinsic(realm, default)
            }
        }
    }

    /// Whether `object` is `%Object.prototype%`-like, for the dispatch's tests.
    #[cfg(test)]
    pub(crate) fn is_immutable_prototype(&mut self, object: ObjectRef) -> Completion<bool> {
        Ok(self.kind_of(object)? == super::KindTag::ImmutablePrototype)
    }
}

#[cfg(test)]
mod tests {
    use crate::host::hooks::test_host::{agent_with_realm, enter};
    use crate::object::ordinary::OrdinaryData;
    use crate::object::property::{Descriptor, Key};
    use crate::value::Value;

    /// OrdinaryOwnPropertyKeys (`#sec-ordinaryownpropertykeys`): the array indices ascending,
    /// then the String keys in creation order, then the Symbol keys in creation order. A
    /// numeric String that is not an array index (`"4294967295"`, `"-1"`) is a String key.
    #[test]
    fn own_property_keys_order() {
        let (mut agent, realm) = agent_with_realm();
        enter(&mut agent, &realm);
        let object = agent
            .ordinary_object_create(None, OrdinaryData::Plain)
            .unwrap();
        let symbol = agent.heap.new_symbol(None).unwrap();
        let mut keys = Vec::new();
        for name in ["b", "1", "#", "a", "0", "4294967295", "-1"] {
            let key = if name == "#" {
                Key::Symbol(symbol)
            } else {
                agent.name_key(name).unwrap()
            };
            keys.push(key);
            assert_eq!(
                agent.create_data_property(object, key, Value::Null),
                Ok(true)
            );
        }
        let names = |agent: &mut crate::host::agent::Agent<_>, keys: Vec<Key>| -> Vec<String> {
            keys.into_iter()
                .map(|key| match key {
                    Key::Index(index) => index.to_string(),
                    Key::String(string) => agent
                        .heap
                        .strings
                        .get(string.handle)
                        .unwrap()
                        .to_rust_string_lossy(),
                    Key::Symbol(_) => "#".to_string(),
                })
                .collect()
        };
        let own = agent.own_property_keys(object).unwrap();
        assert_eq!(
            names(&mut agent, own),
            ["0", "1", "b", "a", "4294967295", "-1", "#"]
        );
    }

    /// `#sec-validateandapplypropertydescriptor` step 2.a: a new property on a non-extensible
    /// object is refused, while an existing one can still be changed; step 5 for the
    /// non-configurable cases is `tests/objects.rs`.
    #[test]
    fn define_on_a_non_extensible_object_fails() {
        let (mut agent, realm) = agent_with_realm();
        enter(&mut agent, &realm);
        let object = agent
            .ordinary_object_create(None, OrdinaryData::Plain)
            .unwrap();
        let old = agent.name_key("old").unwrap();
        let new = agent.name_key("new").unwrap();
        assert_eq!(
            agent.create_data_property(object, old, Value::Null),
            Ok(true)
        );
        assert_eq!(agent.prevent_extensions(object), Ok(true));
        assert_eq!(agent.is_extensible(object), Ok(false));
        assert_eq!(
            agent.define_own_property(object, new, Descriptor::data(Value::Null, true, true, true)),
            Ok(false)
        );
        assert_eq!(agent.has_property(object, new), Ok(false));
        assert_eq!(
            agent.define_own_property(
                object,
                old,
                Descriptor::data(Value::Number(2.0), false, false, false)
            ),
            Ok(true)
        );
        assert_eq!(agent.get(object, old), Ok(Value::Number(2.0)));
    }

    /// OrdinarySetPrototypeOf (`#sec-ordinarysetprototypeof`) refuses a cycle, and
    /// `%Object.prototype%` keeps its [[Prototype]] (10.4.7.1).
    #[test]
    fn set_prototype_of_refuses_cycles_and_the_immutable_prototype() {
        let (mut agent, realm) = agent_with_realm();
        enter(&mut agent, &realm);
        let a = agent
            .ordinary_object_create(None, OrdinaryData::Plain)
            .unwrap();
        let b = agent
            .ordinary_object_create(Some(a), OrdinaryData::Plain)
            .unwrap();
        assert_eq!(agent.set_prototype_of(a, Some(b)), Ok(false));
        assert_eq!(agent.set_prototype_of(b, None), Ok(true));
        assert_eq!(agent.set_prototype_of(a, Some(b)), Ok(true));
        let handle = agent.heap.roots.get_realm(&realm.0).unwrap();
        let object_prototype = agent
            .intrinsic(handle, crate::realm::intrinsics::Intrinsic::ObjectPrototype)
            .unwrap();
        assert_eq!(agent.is_immutable_prototype(object_prototype), Ok(true));
        assert_eq!(agent.set_prototype_of(object_prototype, Some(a)), Ok(false));
        assert_eq!(agent.set_prototype_of(object_prototype, None), Ok(true));
    }
}
