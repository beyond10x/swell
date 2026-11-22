//! String exotic objects (ECMA-262 10.4.3, `#sec-string-exotic-objects`): [[StringData]],
//! StringCreate, StringGetOwnProperty and the three internal methods that differ from the
//! ordinary ones, and the reads of a String's code units the engine uses. Written by
//! `story:object-model-and-realms`.

use super::property::{Descriptor, Key, Property};
use super::{KeyList, ObjectKind};
use crate::heap::collect::Mark;
use crate::heap::string::HeapString;
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::host::{Host, Termination};
use crate::value::{ObjectRef, StringRef, Value};

/// The slots of a String object.
pub(crate) struct StringObject {
    /// [[StringData]].
    pub(crate) data: StringRef,
}

impl StringObject {
    /// Kind site of the collector: [[StringData]].
    pub(crate) fn trace(&self, mark: &mut Mark) {
        mark.count("object/string.rs StringObject::trace");
        mark.value(Value::String(self.data));
    }

    /// A String object of the empty String, for the tests of the collector.
    #[cfg(test)]
    pub(crate) fn for_test(data: StringRef) -> StringObject {
        StringObject { data }
    }
}

impl<H: Host> Agent<H> {
    /// The record of `string`; a stale one inside the engine faults the agent.
    fn engine_string(&mut self, string: StringRef) -> Completion<&HeapString> {
        if self.heap.strings.get(string.handle).is_err() {
            return Err(self.fault());
        }
        self.heap
            .strings
            .get(string.handle)
            .map_err(|_| crate::host::Abrupt::Terminated(Termination::EngineFault))
    }

    /// The code units of `string`.
    pub(crate) fn string_units(&mut self, string: StringRef) -> Completion<Vec<u16>> {
        let units = self.engine_string(string)?.to_utf16();
        #[cfg(test)]
        copies::note(units.len());
        Ok(units)
    }

    /// The code units of `string`, a step per code unit charged before they are copied
    /// (`docs/design/engine.md`, "The step budget").
    pub(crate) fn charged_units(&mut self, string: StringRef) -> Completion<Vec<u16>> {
        let length = self.string_length(string)?;
        self.charge_steps(u64::try_from(length).unwrap_or(u64::MAX))?;
        self.string_units(string)
    }

    /// The code units of `string` from `from` to `to`, with no step: for a caller that charges
    /// them as it appends them. A range that is not inside the String is a broken invariant.
    pub(crate) fn string_units_in(
        &mut self,
        string: StringRef,
        from: usize,
        to: usize,
    ) -> Completion<Vec<u16>> {
        let units = match self.engine_string(string)? {
            HeapString::Latin1(bytes) => bytes
                .get(from..to)
                .map(|bytes| bytes.iter().map(|&byte| u16::from(byte)).collect()),
            HeapString::Utf16(units) => units.get(from..to).map(<[u16]>::to_vec),
        };
        let Some(units) = units else {
            return Err(self.fault());
        };
        #[cfg(test)]
        copies::note(units.len());
        Ok(units)
    }

    /// The length of `string` in code units.
    pub(crate) fn string_length(&mut self, string: StringRef) -> Completion<usize> {
        Ok(match self.engine_string(string)? {
            HeapString::Latin1(bytes) => bytes.len(),
            HeapString::Utf16(units) => units.len(),
        })
    }

    /// The code unit of `string` at `index`, if there is one.
    pub(crate) fn code_unit_at(
        &mut self,
        string: StringRef,
        index: usize,
    ) -> Completion<Option<u16>> {
        Ok(match self.engine_string(string)? {
            HeapString::Latin1(bytes) => bytes.get(index).map(|&b| u16::from(b)),
            HeapString::Utf16(units) => units.get(index).copied(),
        })
    }

    /// A new String of `units`; over the budget the run ends in `Terminated(MemoryBudget)`.
    pub(crate) fn new_string_value(&mut self, units: &[u16]) -> Completion<StringRef> {
        match self.heap.new_string_from_units(units) {
            Ok(string) => Ok(string),
            Err(_) => Err(self.terminate(Termination::MemoryBudget)),
        }
    }

    /// A new String of `text`; over the budget the run ends in `Terminated(MemoryBudget)`.
    pub(crate) fn new_string_str(&mut self, text: &str) -> Completion<StringRef> {
        match self.heap.new_string_from_str(text) {
            Ok(string) => Ok(string),
            Err(_) => Err(self.terminate(Termination::MemoryBudget)),
        }
    }

    /// StringCreate (`#sec-stringcreate`).
    pub(crate) fn string_create(
        &mut self,
        value: StringRef,
        prototype: ObjectRef,
    ) -> Completion<ObjectRef> {
        let length = self.string_length(value)?;
        let object = self.allocate_object(
            Some(prototype),
            ObjectKind::String(StringObject { data: value }),
        )?;
        let key = self.name_key("length")?;
        // A length is below 2^53, so the conversion is exact.
        let length = length as f64;
        self.add_property(
            object,
            key,
            Property::data(Value::Number(length), false, false, false),
        )?;
        Ok(object)
    }

    /// [[StringData]] of a String object.
    fn string_data(&mut self, object: ObjectRef) -> Completion<StringRef> {
        match &self.engine_object(object)?.kind {
            ObjectKind::String(string) => Ok(string.data),
            _ => Err(self.fault()),
        }
    }

    /// StringGetOwnProperty (`#sec-stringgetownproperty`). Only an array index can name a code
    /// unit: CanonicalNumericIndexString answers an integral, non-negative value below the
    /// length for nothing else.
    fn string_get_own_property(
        &mut self,
        object: ObjectRef,
        key: Key,
    ) -> Completion<Option<Property>> {
        let Key::Index(index) = key else {
            return Ok(None);
        };
        let data = self.string_data(object)?;
        let Ok(index) = usize::try_from(index) else {
            return Ok(None);
        };
        let Some(unit) = self.code_unit_at(data, index)? else {
            return Ok(None);
        };
        let result = self.new_string_value(&[unit])?;
        Ok(Some(Property::data(
            Value::String(result),
            false,
            true,
            false,
        )))
    }

    /// [[GetOwnProperty]] of a String object
    /// (`#sec-string-exotic-objects-getownproperty-p`).
    pub(crate) fn string_get_own_property_method(
        &mut self,
        object: ObjectRef,
        key: Key,
    ) -> Completion<Option<Property>> {
        if let Some(property) = self.ordinary_get_own_property(object, key)? {
            return Ok(Some(property));
        }
        self.string_get_own_property(object, key)
    }

    /// [[DefineOwnProperty]] of a String object
    /// (`#sec-string-exotic-objects-defineownproperty-p-desc`).
    pub(crate) fn string_define_own_property(
        &mut self,
        object: ObjectRef,
        key: Key,
        descriptor: Descriptor,
    ) -> Completion<bool> {
        if let Some(current) = self.string_get_own_property(object, key)? {
            let extensible = self.is_extensible(object)?;
            // IsCompatiblePropertyDescriptor (`#sec-iscompatiblepropertydescriptor`).
            return self.validate_and_apply_property_descriptor(
                None,
                key,
                extensible,
                descriptor,
                Some(current),
            );
        }
        self.ordinary_define_own_property(object, key, descriptor)
    }

    /// [[OwnPropertyKeys]] of a String object
    /// (`#sec-string-exotic-objects-ownpropertykeys`): a step per key, counted from the
    /// String's length and the length of the property list before anything is allocated (the
    /// list holds no index below the length, which [[DefineOwnProperty]] answers from the
    /// String), then the list's bytes before it is reserved; index keys of the list not added
    /// ascending are sorted, n steps before each pass (`object/mod.rs`).
    pub(crate) fn string_own_property_keys(&mut self, object: ObjectRef) -> Completion<KeyList> {
        let data = self.string_data(object)?;
        let length = self.string_length(data)?;
        let listed = self.engine_object(object)?.properties.len();
        let count = length.saturating_add(listed);
        self.charge_steps(u64::try_from(count).unwrap_or(u64::MAX))?;
        // 5. For each integer i such that 0 ≤ i < length, in ascending order, append
        // ! ToString(𝔽(i)); 6 to 8, the keys of the list, its array indices from length on.
        let from = u32::try_from(length).unwrap_or(u32::MAX);
        self.own_key_list(count, |agent, keys| {
            keys.extend((0..from).map(Key::Index));
            agent.push_own_keys(object, from, keys)
        })
    }
}

/// The counting build: the longest copy [`Agent::string_units`] (or [`Agent::charged_units`],
/// [`Agent::string_units_in`]) made on this thread since the
/// last reset, a high-water mark in code units. Each test runs in a process of its own under
/// cargo-nextest and resets it first; nothing else reads it.
#[cfg(test)]
pub(crate) mod copies {
    use std::cell::Cell;

    thread_local! {
        static LONGEST: Cell<usize> = const { Cell::new(0) };
    }

    /// Count a copy of `units` code units.
    pub(crate) fn note(units: usize) {
        LONGEST.with(|longest| longest.set(longest.get().max(units)));
    }

    /// The longest copy since the last reset.
    pub(crate) fn longest() -> usize {
        LONGEST.with(Cell::get)
    }

    /// Start counting again from 0.
    pub(crate) fn reset() {
        LONGEST.with(|longest| longest.set(0));
    }
}

#[cfg(test)]
mod tests {
    use crate::Realm;
    use crate::host::agent::Agent;
    use crate::host::hooks::test_host::{NoHooks, agent_with_realm_and, enter, limits};
    use crate::host::outcome::{Abrupt, Termination};
    use crate::realm::intrinsics::Intrinsic;
    use crate::value::{ObjectRef, Value};

    /// The code units of the long String.
    const UNITS: usize = 1_000_000;

    /// An agent with a budget of 256 MiB and a String object of [`UNITS`] Latin-1 code units,
    /// both kept.
    fn long_string_object() -> (Agent<NoHooks>, Realm, ObjectRef) {
        let mut limits = limits();
        limits.memory_bytes = 1 << 28;
        let (mut agent, realm) = agent_with_realm_and(limits);
        enter(&mut agent, &realm);
        let prototype = agent.current_intrinsic(Intrinsic::StringPrototype).unwrap();
        let data = agent.new_string_str(&"a".repeat(UNITS)).unwrap();
        agent.keep(Value::String(data));
        let object = agent.string_create(data, prototype).unwrap();
        agent.keep(Value::Object(object));
        (agent, realm, object)
    }

    /// `docs/design/engine.md`, "The step budget": [[OwnPropertyKeys]] of a String object
    /// (`#sec-string-exotic-objects-ownpropertykeys`) charges a step per key it will answer,
    /// counted from the String's length, before the list exists. With 1,000 steps left the run
    /// ends in `Terminated(StepBudget)` and no list was allocated.
    #[test]
    fn own_keys_of_a_long_string_end_at_the_step_budget_before_the_list_exists() {
        let (mut agent, _realm, object) = long_string_object();
        agent.limits.steps_per_entry = agent.steps + 1_000;
        let answer = agent.own_property_keys(object).map(|keys| keys.len());
        assert_eq!(answer, Err(Abrupt::Terminated(Termination::StepBudget)));
        assert_eq!(agent.heap.accounting.largest_key_list(), 0);
    }

    /// "Memory budget": the list of a String object's keys, `size_of::<Key>()` bytes per code
    /// unit, is charged before it is allocated. With 1 MiB left and steps enough for every key,
    /// the run ends in `Terminated(MemoryBudget)` and nothing stays charged.
    #[test]
    fn own_keys_of_a_long_string_are_charged_before_they_are_built() {
        let (mut agent, _realm, object) = long_string_object();
        agent.limits.steps_per_entry = 10_000_000;
        agent.steps = 0;
        let used = agent.heap.bytes();
        agent.heap.accounting.set_limit(used + (1 << 20));
        let answer = agent.own_property_keys(object).map(|keys| keys.len());
        assert_eq!(answer, Err(Abrupt::Terminated(Termination::MemoryBudget)));
        assert_eq!(agent.heap.bytes(), used);
    }
}
