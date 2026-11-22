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
//! Both kinds define an own @@iterator, `%Array.prototype.values%` of the function's realm
//! (`story:iterator-protocol-and-array-string-iterators`).
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
        // The map's slots are charged when the map is made; the record owns those bytes once it
        // is stored, and the sweep that frees it credits them.
        let (map, owned) = match &mapping {
            Some(mapping) => match self.parameter_map(mapping, args.len())? {
                Some((map, owned)) => (Some(map), owned),
                None => (None, 0),
            },
            None => (None, 0),
        };
        let prototype = match self.intrinsic(realm, Intrinsic::ObjectPrototype) {
            Ok(prototype) => prototype,
            Err(abrupt) => {
                self.heap.accounting.credit(owned);
                return Err(abrupt);
            }
        };
        let record = ObjectRecord::new(
            Some(prototype),
            ObjectKind::Arguments(ArgumentsObject { map }),
        );
        let object = match self.heap.new_object_charged(record, owned) {
            Ok(object) => object,
            Err(_) => {
                self.heap.accounting.credit(owned);
                return Err(self.terminate(Termination::MemoryBudget));
            }
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
        if mapping.is_some() {
            self.add_property(object, length_key, length)?;
        }
        // Both kinds: ! DefinePropertyOrThrow(obj, %Symbol.iterator%, PropertyDescriptor {
        // [[Value]]: %Array.prototype.values%, [[Writable]]: true, [[Enumerable]]: false,
        // [[Configurable]]: true }), before "callee".
        let Some(symbols) = self.well_known() else {
            return Err(self.fault());
        };
        let values = self.intrinsic(realm, Intrinsic::ArrayPrototypeValues)?;
        self.add_property(
            object,
            Key::Symbol(symbols.iterator),
            Property::data(Value::Object(values), true, false, true),
        )?;
        let callee_key = self.name_key("callee")?;
        if mapping.is_some() {
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
    /// ones unmapped). `None` when no index is mapped. A step per parameter. The map's `slots`
    /// and the `seen` list of the walk are charged before they are reserved; `seen` is credited
    /// when it drops, and the map answers with the bytes of `slots`, which stay charged.
    fn parameter_map(
        &mut self,
        mapping: &Mapping<'_>,
        arguments: usize,
    ) -> Completion<Option<(ParameterMap, u64)>> {
        #[cfg(test)]
        MAP_READINGS.with(|readings| readings.set([self.heap.accounting.used(), 0]));
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
        let slot_bytes =
            u64::try_from(mapped.saturating_mul(size_of::<Option<u32>>())).unwrap_or(u64::MAX);
        let seen_bytes = u64::try_from(width.saturating_mul(size_of::<bool>())).unwrap_or(u64::MAX);
        let bytes = slot_bytes.saturating_add(seen_bytes);
        self.charge_bytes(bytes)?;
        let mut slots = Vec::new();
        let mut seen = Vec::new();
        if slots.try_reserve_exact(mapped).is_err() || seen.try_reserve_exact(width).is_err() {
            self.heap.accounting.credit(bytes);
            return Err(self.terminate(Termination::MemoryBudget));
        }
        #[cfg(test)]
        {
            let used = self.heap.accounting.used();
            MAP_READINGS.with(|readings| readings.set([readings.get()[0], used]));
            MAPS_RESERVED.with(|reserved| reserved.set(reserved.get().saturating_add(1)));
        }
        slots.resize(mapped, None);
        seen.resize(width, false);
        for (index, param) in mapping.params.iter().enumerate().rev() {
            let ParamSlot::Env(slot) = *param else {
                self.heap.accounting.credit(bytes);
                return Err(self.fault());
            };
            let Some(taken) = usize::try_from(slot).ok().and_then(|s| seen.get_mut(s)) else {
                self.heap.accounting.credit(bytes);
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
        drop(seen);
        self.heap.accounting.credit(seen_bytes);
        Ok(Some((ParameterMap { env, slots }, slot_bytes)))
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

#[cfg(test)]
thread_local! {
    /// `accounting.used()` at the entry of the last `parameter_map` and, once its `slots` and
    /// `seen` are reserved, after that (0 until then), in the counting build of the tests; a
    /// test resets it before it reads it.
    static MAP_READINGS: std::cell::Cell<[u64; 2]> = const { std::cell::Cell::new([0, 0]) };
    /// The parameter maps reserved, in the counting build of the tests; a test resets it before
    /// it reads it.
    static MAPS_RESERVED: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
mod tests {
    use super::{MAP_READINGS, MAPS_RESERVED};
    use crate::host::agent::Agent;
    use crate::host::hooks::test_host::{NoHooks, agent_with_realm_and, limits};
    use crate::host::outcome::HostError;
    use crate::value::{Root, Value};
    use crate::{Outcome, Parsed, Realm, Source, Termination};

    /// The number of parameters of `f` and of arguments of its call.
    const PARAMETERS: u32 = 1_000;

    /// The bytes of the parameter map of the call of `f` at least: `slots`, one entry per
    /// mapped index, and `seen`, one entry per environment slot up to the highest a parameter
    /// has, at least one per parameter.
    fn map_bytes() -> u64 {
        u64::from(PARAMETERS) * (size_of::<Option<u32>>() + size_of::<bool>()) as u64
    }

    /// An agent with no step budget to speak of and a memory budget of what its realm holds
    /// plus 256 MiB, in which a script declared the non-strict function `f` of 1,000
    /// parameters that answers `arguments.length`, with the roots of a call of `f` from Rust
    /// with 1,000 Numbers.
    struct Prepared {
        agent: Agent<NoHooks>,
        realm: Realm,
        function: Root,
        this: Root,
        args: Vec<Root>,
        roomy_limit: u64,
    }

    impl Prepared {
        /// The agent, with the call run once and the heap collected after it.
        fn new() -> Prepared {
            let names: Vec<String> = (0..PARAMETERS).map(|i| format!("p{i}")).collect();
            let source = format!(
                "function f({}) {{ return arguments.length; }} f",
                names.join(", ")
            );
            let mut limits = limits();
            limits.steps_per_entry = u64::MAX;
            limits.memory_bytes = 1 << 30;
            let (mut agent, realm) = agent_with_realm_and(limits);
            let roomy_limit = agent.heap.bytes() + (1 << 28);
            agent.heap.accounting.set_limit(roomy_limit);
            let script = match agent.parse_script(&realm, Source::Utf8(&source), None) {
                Ok(Parsed::Parsed(script)) => script,
                other => panic!("the declaration did not parse: {other:?}"),
            };
            let function = match agent.evaluate_script(&script) {
                Ok(Outcome::Normal(function)) => function,
                other => panic!("the declaration answered {other:?}"),
            };
            let this = agent.heap.roots.add(Value::Undefined);
            let args = (0..PARAMETERS)
                .map(|i| agent.heap.roots.add(Value::Number(f64::from(i))))
                .collect();
            let mut prepared = Prepared {
                agent,
                realm,
                function,
                this,
                args,
                roomy_limit,
            };
            prepared.answers_the_count();
            assert!(prepared.agent.collect().is_ok());
            prepared
        }

        /// `f` called with the 1,000 Numbers.
        fn call(&mut self) -> Result<Outcome<Root>, HostError> {
            let args: Vec<&Root> = self.args.iter().collect();
            self.agent
                .call(&self.realm, &self.function, &self.this, &args)
        }

        /// The call answers 1,000.
        fn answers_the_count(&mut self) {
            let outcome = self.call();
            let Ok(Outcome::Normal(answer)) = outcome else {
                panic!("{outcome:?}");
            };
            assert_eq!(
                self.agent.heap.roots.get(&answer),
                Ok(Value::Number(f64::from(PARAMETERS)))
            );
        }
    }

    /// `docs/design/engine.md`, "Memory budget" (storage is charged before it is allocated),
    /// and CreateMappedArgumentsObject (`#sec-createmappedargumentsobject`), steps 15 to 19, the
    /// parameter map: the call of [`Prepared`] reads the ledger at the entry of the map's
    /// making and once its two lists are reserved, and the second reading holds their bytes
    /// on top of the first. After a collection the reading is the one before the call.
    #[test]
    fn the_parameter_map_is_charged_while_it_is_held() {
        let mut prepared = Prepared::new();
        let before = prepared.agent.heap_bytes().unwrap();
        MAP_READINGS.with(|readings| readings.set([0, 0]));
        prepared.answers_the_count();
        let [first, second] = MAP_READINGS.with(std::cell::Cell::get);
        assert!(
            second >= first + map_bytes(),
            "{second} once the map is reserved, {first} before, {} the map's bytes at least",
            map_bytes()
        );
        assert!(prepared.agent.collect().is_ok());
        assert_eq!(
            prepared.agent.heap_bytes().unwrap(),
            before,
            "after the call"
        );
    }

    /// `docs/design/engine.md`, "Memory budget" (storage is charged before it is allocated),
    /// and CreateMappedArgumentsObject: the call of [`Prepared`] under a budget of the reading
    /// recorded at the entry of the map's making on a first run, on an agent made the same
    /// way, plus half the map's bytes, ends in `Terminated(MemoryBudget)` at that entry with no
    /// map reserved. With the budget raised the call answers 1,000 and reserves one map, and
    /// after a collection the reading is the one before.
    #[test]
    fn the_parameter_map_is_not_made_past_the_memory_budget() {
        let mut first_run = Prepared::new();
        MAP_READINGS.with(|readings| readings.set([0, 0]));
        first_run.answers_the_count();
        let [entry, _] = MAP_READINGS.with(std::cell::Cell::get);
        drop(first_run);
        let mut prepared = Prepared::new();
        let before = prepared.agent.heap_bytes().unwrap();
        prepared
            .agent
            .heap
            .accounting
            .set_limit(entry + map_bytes() / 2);
        MAP_READINGS.with(|readings| readings.set([0, 0]));
        MAPS_RESERVED.with(|reserved| reserved.set(0));
        let outcome = prepared.call();
        assert!(
            matches!(outcome, Ok(Outcome::Terminated(Termination::MemoryBudget))),
            "{outcome:?}"
        );
        assert_eq!(
            MAP_READINGS.with(std::cell::Cell::get)[0],
            entry,
            "the map's making was entered at another reading"
        );
        assert_eq!(
            MAPS_RESERVED.with(std::cell::Cell::get),
            0,
            "a map reserved past the memory budget"
        );
        drop(outcome);
        let roomy_limit = prepared.roomy_limit;
        prepared.agent.heap.accounting.set_limit(roomy_limit);
        prepared.answers_the_count();
        assert_eq!(MAPS_RESERVED.with(std::cell::Cell::get), 1);
        assert!(prepared.agent.collect().is_ok());
        assert_eq!(
            prepared.agent.heap_bytes().unwrap(),
            before,
            "after the call"
        );
    }
}
