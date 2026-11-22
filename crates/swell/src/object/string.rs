//! String exotic objects (ECMA-262 10.4.3, `#sec-string-exotic-objects`): [[StringData]],
//! StringCreate, StringGetOwnProperty and the three internal methods that differ from the
//! ordinary ones, and the reads of a String's code units the engine uses. Written by
//! `story:object-model-and-realms`.

use super::ObjectKind;
use super::property::{Descriptor, Key, Property};
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
        Ok(self.engine_string(string)?.to_utf16())
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
    /// (`#sec-string-exotic-objects-ownpropertykeys`): a step per key, and a step per comparison
    /// of a sort of the indices of the list (`object/property.rs`).
    pub(crate) fn string_own_property_keys(&mut self, object: ObjectRef) -> Completion<Vec<Key>> {
        let data = self.string_data(object)?;
        let length = self.string_length(data)?;
        let (own, comparisons) = self.engine_object(object)?.properties.keys();
        if comparisons > 0 {
            self.charge_steps(comparisons)?;
        }
        let mut keys = Vec::new();
        keys.extend(
            (0..length)
                .filter_map(|index| u32::try_from(index).ok())
                .map(Key::Index),
        );
        keys.extend(own.into_iter().filter(|key| match key {
            Key::Index(index) => usize::try_from(*index).is_ok_and(|index| index >= length),
            _ => true,
        }));
        self.charge_steps(u64::try_from(keys.len()).unwrap_or(u64::MAX))?;
        Ok(keys)
    }
}
