//! Arguments exotic objects (ECMA-262 10.4.4, `#sec-arguments-exotic-objects`): [[ParameterMap]],
//! CreateUnmappedArgumentsObject (`#sec-createunmappedargumentsobject`),
//! CreateMappedArgumentsObject (`#sec-createmappedargumentsobject`) and the internal methods that
//! differ from the ordinary ones. Written by `story:with-arguments-and-dynamic-names`.
//!
//! The parameter map of a mapped arguments object is not an object here: it is, for each index
//! below the number of arguments and of parameters, the slot of the function environment that
//! holds the parameter of that index (MakeArgGetter and MakeArgSetter,
//! `#sec-makearggetter`, `#sec-makeargsetter`, read and write that binding), or none once the
//! index is no longer mapped. Every parameter of a function that makes one lives in its function
//! environment (`swell-compiler`, `resolve.rs`). An unmapped arguments object has no map; it is
//! this kind too, since its [[ParameterMap]] slot is what `Object.prototype.toString` reads
//! (`#sec-object.prototype.tostring`, step 7).
//!
//! [[GetOwnProperty]] (10.4.4.1), [[DefineOwnProperty]] (10.4.4.2) and [[Delete]] (10.4.4.5) are
//! the exotic ones. [[Get]] (10.4.4.3) and [[Set]] (10.4.4.4) are OrdinaryGet and OrdinarySet: a
//! mapped index is a writable data property, so OrdinaryGet, which reads it through
//! [[GetOwnProperty]], answers the binding's value as map.[[Get]] does, and OrdinarySet with the
//! object as the receiver ends in [[DefineOwnProperty]] with a [[Value]], which writes the
//! binding as map.[[Set]] does; with another receiver the map is not written either way.
//! @@iterator, which both kinds define, comes with the iterators of slice 6.
//!
//! The map's slots are charged to the memory budget with the object and credited when the
//! collector frees it.

use swell_compiler::bytecode::Slot as ParamSlot;

use super::property::{Descriptor, Key, Property, Slot};
use super::{ObjectKind, ObjectRecord};
use crate::heap::arena::Handle;
use crate::heap::collect::{Item, Mark};
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::host::{Host, Termination};
use crate::realm::intrinsics::Intrinsic;
use crate::value::{ObjectRef, Value};

/// The [[ParameterMap]] of a mapped arguments object.
pub(crate) struct ParameterMap {
    /// The function environment of the call.
    env: Handle,
    /// For each mapped index, the slot of `env` its binding lives in.
    slots: Vec<Option<u32>>,
}

/// The slots of an arguments object: the parameter map of a mapped one.
pub(crate) struct ArgumentsObject {
    pub(crate) map: Option<ParameterMap>,
}

impl ArgumentsObject {
    /// Kind site of the collector: the function environment of the map.
    pub(crate) fn trace(&self, mark: &mut Mark) {
        mark.count("object/arguments.rs ArgumentsObject::trace");
        if let Some(map) = &self.map {
            mark.push(Item::Environment(map.env));
        }
    }

    /// An arguments object, for the tests of the collector.
    #[cfg(test)]
    pub(crate) fn for_test() -> ArgumentsObject {
        ArgumentsObject { map: None }
    }
}

/// The function environment and the parameters of a call that makes a mapped arguments object.
pub(crate) struct Mapping<'a> {
    pub(crate) env: Option<Handle>,
    pub(crate) params: &'a [ParamSlot],
}

impl<H: Host> Agent<H> {
    /// CreateMappedArgumentsObject (`mapping` given) or CreateUnmappedArgumentsObject of a call
    /// of `function` with `args`, with `%Object.prototype%` and `%ThrowTypeError%` of `realm`. A
    /// step per argument; runs no code.
    pub(crate) fn create_arguments_object(
        &mut self,
        realm: Handle,
        function: ObjectRef,
        args: &[Value],
        mapping: Option<Mapping<'_>>,
    ) -> Completion<ObjectRef> {
        let count = u32::try_from(args.len()).unwrap_or(u32::MAX);
        self.charge_steps(u64::from(count))?;
        let map = match &mapping {
            Some(mapping) => self.parameter_map(mapping, args.len())?,
            None => None,
        };
        let owned = map.as_ref().map_or(0, |map| {
            u64::try_from(
                map.slots
                    .capacity()
                    .saturating_mul(size_of::<Option<u32>>()),
            )
            .unwrap_or(u64::MAX)
        });
        let prototype = self.intrinsic(realm, Intrinsic::ObjectPrototype)?;
        let record = ObjectRecord::new(
            Some(prototype),
            ObjectKind::Arguments(ArgumentsObject { map }),
        );
        let object = match self.heap.new_object(record, owned) {
            Ok(object) => object,
            Err(_) => return Err(self.terminate(Termination::MemoryBudget)),
        };
        // The object is new and extensible: each CreateDataPropertyOrThrow and
        // DefinePropertyOrThrow adds the property.
        let length_key = self.name_key("length")?;
        let length = Property::data(Value::Number(f64::from(count)), true, false, true);
        if mapping.is_none() {
            self.add_property(object, length_key, length)?;
        }
        for (index, &value) in args.iter().enumerate() {
            let Ok(index) = u32::try_from(index) else {
                return Err(self.fault());
            };
            self.add_property(
                object,
                Key::Index(index),
                Property::data(value, true, true, true),
            )?;
        }
        let callee_key = self.name_key("callee")?;
        if mapping.is_some() {
            self.add_property(object, length_key, length)?;
            self.add_property(
                object,
                callee_key,
                Property::data(Value::Object(function), true, false, true),
            )?;
        } else {
            let thrower = self.intrinsic(realm, Intrinsic::ThrowTypeError)?;
            self.add_property(
                object,
                callee_key,
                Property {
                    slot: Slot::Accessor {
                        get: Some(thrower),
                        set: Some(thrower),
                    },
                    enumerable: false,
                    configurable: false,
                },
            )?;
        }
        Ok(object)
    }

    /// The parameter map of CreateMappedArgumentsObject, steps 15 to 19: from the last
    /// parameter to the first, each name not mapped yet and whose index is below the number of
    /// arguments is mapped; parameters of one name have one slot, so a name is mapped once
    /// whatever its index (a later parameter of the name past the arguments leaves the earlier
    /// ones unmapped). `None` when no index is mapped. A step per parameter.
    fn parameter_map(
        &mut self,
        mapping: &Mapping<'_>,
        arguments: usize,
    ) -> Completion<Option<ParameterMap>> {
        let mapped = arguments.min(mapping.params.len());
        if mapped == 0 {
            return Ok(None);
        }
        self.charge_steps(u64::try_from(mapping.params.len()).unwrap_or(u64::MAX))?;
        let Some(env) = mapping.env else {
            return Err(self.fault());
        };
        let mut width = 0usize;
        for param in mapping.params {
            let ParamSlot::Env(slot) = *param else {
                return Err(self.fault());
            };
            width = width.max(
                usize::try_from(slot)
                    .unwrap_or(usize::MAX)
                    .saturating_add(1),
            );
        }
        let bytes = mapped
            .saturating_mul(size_of::<Option<u32>>())
            .saturating_add(width.saturating_mul(size_of::<bool>()));
        if self
            .heap
            .accounting
            .check(u64::try_from(bytes).unwrap_or(u64::MAX))
            .is_err()
        {
            return Err(self.terminate(Termination::MemoryBudget));
        }
        let mut slots = Vec::new();
        let mut seen = Vec::new();
        if slots.try_reserve_exact(mapped).is_err() || seen.try_reserve_exact(width).is_err() {
            return Err(self.terminate(Termination::MemoryBudget));
        }
        slots.resize(mapped, None);
        seen.resize(width, false);
        for (index, param) in mapping.params.iter().enumerate().rev() {
            let ParamSlot::Env(slot) = *param else {
                return Err(self.fault());
            };
            let Some(taken) = usize::try_from(slot).ok().and_then(|s| seen.get_mut(s)) else {
                return Err(self.fault());
            };
            if *taken {
                continue;
            }
            *taken = true;
            if let Some(entry) = slots.get_mut(index) {
                *entry = Some(slot);
            }
        }
        Ok(Some(ParameterMap { env, slots }))
    }

    /// The binding `key` is mapped to in the parameter map of `object`, if any
    /// (HasOwnProperty(map, P)).
    fn mapped_binding(&mut self, object: ObjectRef, key: Key) -> Completion<Option<(Handle, u32)>> {
        let Key::Index(index) = key else {
            return Ok(None);
        };
        let ObjectKind::Arguments(arguments) = &self.engine_object(object)?.kind else {
            return Ok(None);
        };
        Ok(arguments.map.as_ref().and_then(|map| {
            let slot = (*map.slots.get(usize::try_from(index).ok()?)?)?;
            Some((map.env, slot))
        }))
    }

    /// map.[[Delete]](P): `key` is no longer mapped.
    fn unmap(&mut self, object: ObjectRef, key: Key) -> Completion<()> {
        let Key::Index(index) = key else {
            return Ok(());
        };
        if let ObjectKind::Arguments(ArgumentsObject { map: Some(map) }) =
            &mut self.engine_object_mut(object)?.kind
            && let Some(entry) = usize::try_from(index)
                .ok()
                .and_then(|index| map.slots.get_mut(index))
        {
            *entry = None;
        }
        Ok(())
    }

    /// [[GetOwnProperty]] (`#sec-arguments-exotic-objects-getownproperty-p`): a mapped index
    /// has the value of its binding.
    pub(crate) fn arguments_get_own_property(
        &mut self,
        object: ObjectRef,
        key: Key,
    ) -> Completion<Option<Property>> {
        let Some(mut property) = self.ordinary_get_own_property(object, key)? else {
            return Ok(None);
        };
        if let Some((env, slot)) = self.mapped_binding(object, key)? {
            let bound = self.get_slot(Some(env), 0, slot)?;
            if let Slot::Data { value, .. } = &mut property.slot {
                *value = bound;
            }
        }
        Ok(Some(property))
    }

    /// [[DefineOwnProperty]] (`#sec-arguments-exotic-objects-defineownproperty-p-desc`).
    pub(crate) fn arguments_define_own_property(
        &mut self,
        object: ObjectRef,
        key: Key,
        descriptor: Descriptor,
    ) -> Completion<bool> {
        let mapped = self.mapped_binding(object, key)?;
        let mut new_descriptor = descriptor;
        if let Some((env, slot)) = mapped
            && descriptor.is_data()
            && descriptor.value.is_none()
            && descriptor.writable == Some(false)
        {
            new_descriptor.value = Some(self.get_slot(Some(env), 0, slot)?);
        }
        if !self.ordinary_define_own_property(object, key, new_descriptor)? {
            return Ok(false);
        }
        if let Some((env, slot)) = mapped {
            if descriptor.is_accessor() {
                self.unmap(object, key)?;
            } else {
                if let Some(value) = descriptor.value {
                    self.set_slot(Some(env), 0, slot, value)?;
                }
                if descriptor.writable == Some(false) {
                    self.unmap(object, key)?;
                }
            }
        }
        Ok(true)
    }

    /// [[Delete]] (`#sec-arguments-exotic-objects-delete-p`).
    pub(crate) fn arguments_delete(&mut self, object: ObjectRef, key: Key) -> Completion<bool> {
        let mapped = self.mapped_binding(object, key)?.is_some();
        let deleted = self.ordinary_delete(object, key)?;
        if deleted && mapped {
            self.unmap(object, key)?;
        }
        Ok(deleted)
    }
}
