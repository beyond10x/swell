//! Array exotic objects (ECMA-262 10.4.2, `#sec-array-exotic-objects`): ArrayCreate, the
//! [[DefineOwnProperty]] of 10.4.2.1 and ArraySetLength (10.4.2.4), and the storage of the
//! elements. Written by `story:object-model-and-realms`; `story:property-index-and-dense-elements`
//! added the dense storage.
//!
//! `"length"` is an ordinary data property of the record, kept by the two methods here as the
//! clause says. The elements of an Array from index 0 on that are data properties with the
//! attributes an assignment makes ({ [[Writable]]: true, [[Enumerable]]: true,
//! [[Configurable]]: true }) live in the Array's dense storage, a vector with holes
//! (`docs/design/engine.md`, decision 4, "Objects"); an element with other attributes, an
//! accessor, and an index more than [`MAX_HOLES`] beyond the end of the dense part live in the
//! common list, the sparse fallback. An index lives in at most one of the two: a hole of the
//! dense part is either absent or in the list. [[GetOwnProperty]], [[DefineOwnProperty]],
//! [[Delete]] and [[OwnPropertyKeys]] answer over both as OrdinaryGetOwnProperty,
//! OrdinaryDefineOwnProperty, OrdinaryDelete and OrdinaryOwnPropertyKeys do over one map, so
//! nothing a script can observe depends on where an element lives. An element changed to other
//! attributes moves to the list and stays there.
//!
//! The dense storage grows by doubling (at least to the index written), charged to the memory
//! budget before it is made and credited when the Array is freed (`Grown`, `object/mod.rs`); a
//! walk over it (the keys, a truncation) charges a step per position it visits, holes included.

use std::cmp::Ordering;

use super::ordinary::applied_property;
use super::property::{Descriptor, Key, Property, SMALL, Slot};
use super::{KeyList, ListedIndices, ObjectKind};
use crate::heap::collect::Mark;
use crate::host::ErrorKind;
use crate::host::agent::Agent;
use crate::host::outcome::{Abrupt, Completion};
use crate::host::{Host, Termination};
use crate::realm::intrinsics::Intrinsic;
use crate::value::{ObjectRef, Value};

/// The holes a write may open past the end of the dense part: an index further beyond it lives
/// in the list, so that `a[4294967294] = 1` allocates no storage for the indices below it.
pub(crate) const MAX_HOLES: usize = 1024;

/// Push to `keys` the indices of the dense `elements` that hold an element and the index keys
/// `listed` answers ascending, merged ascending.
fn merge_indices(
    elements: &[Option<Value>],
    listed: impl Iterator<Item = u32>,
    keys: &mut Vec<Key>,
) {
    let mut sparse = listed.peekable();
    for (position, slot) in elements.iter().enumerate() {
        if slot.is_none() {
            continue;
        }
        let Ok(index) = u32::try_from(position) else {
            break;
        };
        while let Some(&other) = sparse.peek()
            && other < index
        {
            keys.push(Key::Index(other));
            sparse.next();
        }
        keys.push(Key::Index(index));
    }
    keys.extend(sparse.map(Key::Index));
}

/// The slots of an Array: its dense elements.
pub(crate) struct Array {
    /// The elements from index 0: `Some` an element with the attributes an assignment makes,
    /// `None` a hole.
    elements: Vec<Option<Value>>,
    /// Test only: every element lives in the list, for the tests that compare both storages.
    #[cfg(test)]
    sparse: bool,
}

impl Array {
    /// An Array with no element.
    fn new() -> Array {
        Array {
            elements: Vec::new(),
            #[cfg(test)]
            sparse: false,
        }
    }

    /// Kind site of the collector: the dense elements.
    pub(crate) fn trace(&self, mark: &mut Mark) {
        mark.count("object/array.rs Array::trace");
        for value in self.elements.iter().flatten() {
            mark.value(*value);
        }
    }

    /// An Array of the dense `elements`, for the tests of the collector.
    #[cfg(test)]
    pub(crate) fn for_test_with(elements: Vec<Option<Value>>) -> Array {
        Array {
            elements,
            sparse: false,
        }
    }
}

/// Whether `property` is an element the dense storage holds.
fn is_dense_element(property: &Property) -> bool {
    matches!(
        property,
        Property {
            slot: Slot::Data { writable: true, .. },
            enumerable: true,
            configurable: true,
        }
    )
}

/// 2^32 - 1, the largest length.
const MAX_LENGTH: f64 = 4_294_967_295.0;

/// The code units of `"length"`.
const LENGTH_UNITS: [u16; 6] = [0x6c, 0x65, 0x6e, 0x67, 0x74, 0x68];

impl<H: Host> Agent<H> {
    /// ArrayCreate (`#sec-arraycreate`), with `%Array.prototype%` of the current realm when
    /// `prototype` is `None`.
    pub(crate) fn array_create(
        &mut self,
        length: f64,
        prototype: Option<ObjectRef>,
    ) -> Completion<ObjectRef> {
        if length > MAX_LENGTH {
            return Err(self.throw_error(ErrorKind::RangeError, "Invalid array length"));
        }
        let prototype = match prototype {
            Some(prototype) => prototype,
            None => self.current_intrinsic(Intrinsic::ArrayPrototype)?,
        };
        let array = self.allocate_object(Some(prototype), ObjectKind::Array(Array::new()))?;
        let key = self.name_key("length")?;
        self.add_property(
            array,
            key,
            Property::data(Value::Number(length), true, false, false),
        )?;
        Ok(array)
    }

    /// An Array whose elements all live in the list, for the tests that compare both storages.
    #[cfg(test)]
    pub(crate) fn array_create_sparse(&mut self) -> Completion<ObjectRef> {
        let array = self.array_create(0.0, None)?;
        if let ObjectKind::Array(storage) = &mut self.engine_object_mut(array)?.kind {
            storage.sparse = true;
        }
        Ok(array)
    }

    /// The room of the dense storage of `array`.
    #[cfg(test)]
    pub(crate) fn dense_capacity(&mut self, array: ObjectRef) -> Completion<usize> {
        Ok(self.dense(array)?.elements.capacity())
    }

    /// The dense storage of `array`; an Array without one is an engine bug.
    fn dense(&mut self, array: ObjectRef) -> Completion<&Array> {
        if !matches!(self.engine_object(array)?.kind, ObjectKind::Array(_)) {
            return Err(self.fault());
        }
        match &self.engine_object(array)?.kind {
            ObjectKind::Array(storage) => Ok(storage),
            // The check above passed, so this is not reached.
            _ => Err(Abrupt::Terminated(Termination::EngineFault)),
        }
    }

    /// The dense storage of `array`, to change.
    fn dense_mut(&mut self, array: ObjectRef) -> Completion<&mut Array> {
        if !matches!(self.engine_object(array)?.kind, ObjectKind::Array(_)) {
            return Err(self.fault());
        }
        match &mut self.engine_object_mut(array)?.kind {
            ObjectKind::Array(storage) => Ok(storage),
            // The check above passed, so this is not reached.
            _ => Err(Abrupt::Terminated(Termination::EngineFault)),
        }
    }

    /// The dense element at `index`, if there is one.
    fn dense_element(&mut self, array: ObjectRef, index: u32) -> Completion<Option<Value>> {
        let elements = &self.dense(array)?.elements;
        Ok(usize::try_from(index)
            .ok()
            .and_then(|index| elements.get(index).copied().flatten()))
    }

    /// [[GetOwnProperty]] of an Array: OrdinaryGetOwnProperty
    /// (`#sec-ordinarygetownproperty`) over the dense storage and the list.
    pub(crate) fn array_get_own_property(
        &mut self,
        array: ObjectRef,
        key: Key,
    ) -> Completion<Option<Property>> {
        if let Key::Index(index) = key
            && let Some(value) = self.dense_element(array, index)?
        {
            return Ok(Some(Property::data(value, true, true, true)));
        }
        self.ordinary_get_own_property(array, key)
    }

    /// The value of the `"length"` property of `array` and whether it is writable.
    fn array_length(&mut self, array: ObjectRef, key: Key) -> Completion<(f64, bool)> {
        match self.ordinary_get_own_property(array, key)? {
            Some(Property {
                slot:
                    Slot::Data {
                        value: Value::Number(length),
                        writable,
                    },
                ..
            }) => Ok((length, writable)),
            _ => Err(self.fault()),
        }
    }

    /// [[DefineOwnProperty]] of an Array (`#sec-array-exotic-objects-defineownproperty-p-desc`).
    pub(crate) fn array_define_own_property(
        &mut self,
        array: ObjectRef,
        key: Key,
        descriptor: Descriptor,
    ) -> Completion<bool> {
        let length_key = self.name_key("length")?;
        if key == length_key {
            return self.array_set_length(array, length_key, descriptor);
        }
        let Key::Index(index) = key else {
            return self.ordinary_define_own_property(array, key, descriptor);
        };
        let (length, writable) = self.array_length(array, length_key)?;
        let index_number = f64::from(index);
        if index_number >= length && !writable {
            return Ok(false);
        }
        if !self.array_define_element(array, index, descriptor)? {
            return Ok(false);
        }
        if index_number >= length {
            let new_length = Descriptor {
                value: Some(Value::Number(index_number + 1.0)),
                ..Descriptor::default()
            };
            self.ordinary_define_own_property(array, length_key, new_length)?;
        }
        Ok(true)
    }

    /// The value and [[Writable]] of the `"length"` of `array`, read where ArrayCreate put it:
    /// the first entry of its list, which it stays, since it is not configurable (no deletion
    /// removes it) and a compaction keeps the order. `None` when that entry is not it.
    fn head_length(&self, array: ObjectRef) -> Option<(f64, bool)> {
        let record = self.heap.objects.get(array.handle).ok()?;
        let (Key::String(name), property) = record.properties.first()? else {
            return None;
        };
        let Slot::Data {
            value: Value::Number(length),
            writable,
        } = property.slot
        else {
            return None;
        };
        let name = self.heap.strings.get(name.handle).ok()?;
        (name.cmp_units(&LENGTH_UNITS) == Ordering::Equal).then_some((length, writable))
    }

    /// OrdinarySet (`#sec-ordinaryset`) of `value` at `index` on `array`, its own receiver,
    /// when the answer is known without the general path; `None` leaves the store to it.
    ///
    /// - An element of the dense part is a data property { [[Writable]]: true } of the Array
    ///   itself: OrdinarySet finds it first, charges no step, and the define of its [[Value]]
    ///   stores it in place.
    /// - At a hole or the end of the dense part of an extensible Array whose `"length"` is
    ///   writable and whose list never held an index: when no object of its chain can hold the
    ///   index (none is a String or an arguments exotic object, no list of one ever held an
    ///   index, an Array has no dense element; every other kind, a generator object included,
    ///   has ordinary methods), OrdinarySet passes to every prototype, a step
    ///   each, and ends in CreateDataProperty on the Array, which stores the element dense, its
    ///   growth charged first ([`Agent::store_dense`]), and raises the length past it.
    pub(crate) fn array_set_index(
        &mut self,
        array: ObjectRef,
        index: u32,
        value: Value,
    ) -> Completion<Option<bool>> {
        let Ok(position) = usize::try_from(index) else {
            return Ok(None);
        };
        let record = self.engine_object_mut(array)?;
        let ObjectKind::Array(storage) = &mut record.kind else {
            return Ok(None);
        };
        #[cfg(test)]
        if storage.sparse {
            return Ok(None);
        }
        if let Some(slot) = storage.elements.get_mut(position)
            && slot.is_some()
        {
            *slot = Some(value);
            return Ok(Some(true));
        }
        if position > storage.elements.len()
            || !record.extensible
            || record.properties.may_hold_index()
        {
            return Ok(None);
        }
        let Some((length, true)) = self.head_length(array) else {
            return Ok(None);
        };
        let mut hops: u64 = 0;
        let mut next = self.engine_object(array)?.prototype;
        while let Some(object) = next {
            let record = self.engine_object(object)?;
            let plain = match &record.kind {
                // Exotic [[GetOwnProperty]] for indices: the String's code units, the
                // arguments object's mapped parameters.
                ObjectKind::String(_) | ObjectKind::Arguments(_) => false,
                // Its dense elements are not in its list.
                ObjectKind::Array(storage) => storage.elements.is_empty(),
                // Ordinary [[GetOwnProperty]], [[DefineOwnProperty]] and [[Set]]: its list
                // holds every property it has. A generator object's methods are all ordinary
                // (`#sec-properties-of-generator-instances`; `object/generator.rs`).
                ObjectKind::Ordinary(_)
                | ObjectKind::BuiltinFunction(_)
                | ObjectKind::BoundFunction(_)
                | ObjectKind::EcmascriptFunction(_)
                | ObjectKind::Host(_)
                | ObjectKind::RegExp(_)
                | ObjectKind::Generator(_) => true,
            };
            if !plain || record.properties.may_hold_index() {
                return Ok(None);
            }
            hops += 1;
            next = record.prototype;
        }
        for _ in 0..hops {
            self.charge_steps(1)?;
        }
        self.store_dense(array, index, value)?;
        let raised = f64::from(index) + 1.0;
        if raised > length
            && let Some(Property {
                slot: Slot::Data { value, .. },
                ..
            }) = self.engine_object_mut(array)?.properties.at_mut(0)
        {
            *value = Value::Number(raised);
        }
        Ok(Some(true))
    }

    /// OrdinaryDefineOwnProperty (`#sec-ordinarydefineownproperty`) of the element `index` over
    /// both storages: ValidateAndApplyPropertyDescriptor against the element where it lives,
    /// then the property it leaves stored where it belongs: an element in the list stays
    /// there; a dense one stays dense while it keeps the attributes of an assignment and moves
    /// to the list otherwise; a new one is dense when it has them and its index is at most
    /// [`MAX_HOLES`] beyond the dense part.
    fn array_define_element(
        &mut self,
        array: ObjectRef,
        index: u32,
        descriptor: Descriptor,
    ) -> Completion<bool> {
        let key = Key::Index(index);
        let dense = self.dense_element(array, index)?;
        let current = match dense {
            Some(value) => Some(Property::data(value, true, true, true)),
            None => match self.ordinary_get_own_property(array, key)? {
                Some(_) => return self.ordinary_define_own_property(array, key, descriptor),
                None => None,
            },
        };
        let extensible = self.is_extensible(array)?;
        if !self
            .validate_and_apply_property_descriptor(None, key, extensible, descriptor, current)?
        {
            return Ok(false);
        }
        let property = applied_property(&descriptor, current);
        if let Slot::Data { value, .. } = property.slot
            && is_dense_element(&property)
            && (dense.is_some() || self.fits_dense(array, index)?)
        {
            self.store_dense(array, index, value)?;
            return Ok(true);
        }
        if dense.is_some() {
            self.clear_dense(array, index)?;
        }
        self.add_property(array, key, property)?;
        Ok(true)
    }

    /// Whether a new element at `index` belongs to the dense storage.
    fn fits_dense(&mut self, array: ObjectRef, index: u32) -> Completion<bool> {
        let storage = self.dense(array)?;
        #[cfg(test)]
        if storage.sparse {
            return Ok(false);
        }
        let end = storage.elements.len().saturating_add(MAX_HOLES);
        Ok(usize::try_from(index).is_ok_and(|index| index <= end))
    }

    /// Store `value` as the dense element `index`, growing the storage first when it has no
    /// room for it: to twice its room, at least 4 and at least `index + 1`, charged before it is
    /// made; over the budget the run ends in `Terminated(MemoryBudget)`.
    fn store_dense(&mut self, array: ObjectRef, index: u32, value: Value) -> Completion<()> {
        let Ok(position) = usize::try_from(index) else {
            return Err(self.fault());
        };
        let capacity = self.dense(array)?.elements.capacity();
        if position >= capacity {
            let wanted = capacity
                .saturating_mul(2)
                .max(4)
                .max(position.saturating_add(1));
            let more = u64::try_from(wanted - capacity).unwrap_or(u64::MAX);
            let element = u64::try_from(size_of::<Option<Value>>()).unwrap_or(u64::MAX);
            let bytes = more.saturating_mul(element);
            if self.heap.accounting.charge(bytes).is_err() {
                return Err(self.terminate(Termination::MemoryBudget));
            }
            let storage = self.dense_mut(array)?;
            let length = storage.elements.len();
            if storage
                .elements
                .try_reserve_exact(wanted.saturating_sub(length))
                .is_err()
            {
                self.heap.accounting.credit(bytes);
                return Err(self.terminate(Termination::MemoryBudget));
            }
            self.record_grown(array, bytes)?;
        }
        let elements = &mut self.dense_mut(array)?.elements;
        if position >= elements.len() {
            elements.resize(position.saturating_add(1), None);
        }
        match elements.get_mut(position) {
            Some(slot) => {
                *slot = Some(value);
                Ok(())
            }
            None => Err(self.fault()),
        }
    }

    /// Make the dense element `index` a hole.
    fn clear_dense(&mut self, array: ObjectRef, index: u32) -> Completion<()> {
        let elements = &mut self.dense_mut(array)?.elements;
        if let Some(slot) = usize::try_from(index)
            .ok()
            .and_then(|index| elements.get_mut(index))
        {
            *slot = None;
        }
        Ok(())
    }

    /// [[Delete]] of an Array: OrdinaryDelete (`#sec-ordinarydelete`) over both storages; a
    /// dense element is configurable and becomes a hole.
    pub(crate) fn array_delete(&mut self, array: ObjectRef, key: Key) -> Completion<bool> {
        if let Key::Index(index) = key
            && self.dense_element(array, index)?.is_some()
        {
            self.clear_dense(array, index)?;
            return Ok(true);
        }
        self.ordinary_delete(array, key)
    }

    /// [[OwnPropertyKeys]] of an Array: OrdinaryOwnPropertyKeys
    /// (`#sec-ordinaryownpropertykeys`) over both storages, the dense indices and those of the
    /// list merged ascending. A step per position of the dense storage, holes included, and
    /// per key of the list, counted from their lengths before anything is allocated; then the
    /// list's bytes, a key per element and per key of the list, before it is reserved. Index
    /// keys of the list not added ascending are sorted, n steps before each pass
    /// (`object/mod.rs`).
    pub(crate) fn array_own_property_keys(&mut self, array: ObjectRef) -> Completion<KeyList> {
        let record = self.engine_object(array)?;
        let ObjectKind::Array(storage) = &record.kind else {
            return Err(self.fault());
        };
        let (positions, listed) = (storage.elements.len(), record.properties.len());
        self.charge_steps(u64::try_from(positions.saturating_add(listed)).unwrap_or(u64::MAX))?;
        let elements = self.dense(array)?.elements.iter().flatten().count();
        self.own_key_list(elements.saturating_add(listed), |agent, keys| {
            agent.push_array_keys(array, keys)
        })
    }

    /// Push to `keys`, which has room for them, the keys of `array` in the order of
    /// OrdinaryOwnPropertyKeys: the dense indices and the index keys of the list merged
    /// ascending, then the list's String keys and Symbol keys in creation order.
    fn push_array_keys(&mut self, array: ObjectRef, keys: &mut Vec<Key>) -> Completion<()> {
        let listed = self.listed_indices(array)?;
        let record = match self.engine_object(array) {
            Ok(record) => record,
            Err(abrupt) => {
                self.heap.accounting.credit(listed.bytes());
                return Err(abrupt);
            }
        };
        let pushed = if let ObjectKind::Array(storage) = &record.kind {
            match &listed {
                ListedIndices::InPlace => {
                    merge_indices(&storage.elements, record.properties.listed_indices(), keys);
                }
                ListedIndices::Sorted { indices, .. } => {
                    merge_indices(&storage.elements, indices.iter().copied(), keys);
                }
            }
            keys.extend(record.properties.names());
            true
        } else {
            false
        };
        self.heap.accounting.credit(listed.bytes());
        if !pushed {
            return Err(self.fault());
        }
        Ok(())
    }

    /// ArraySetLength (`#sec-arraysetlength`).
    fn array_set_length(
        &mut self,
        array: ObjectRef,
        length_key: Key,
        descriptor: Descriptor,
    ) -> Completion<bool> {
        let Some(value) = descriptor.value else {
            return self.ordinary_define_own_property(array, length_key, descriptor);
        };
        let mut new_length_descriptor = descriptor;
        let new_length = self.to_uint32(value)?;
        let number_length = self.to_number(value)?;
        if f64::from(new_length) != number_length {
            return Err(self.throw_error(ErrorKind::RangeError, "Invalid array length"));
        }
        new_length_descriptor.value = Some(Value::Number(f64::from(new_length)));
        let (old_length, old_writable) = self.array_length(array, length_key)?;
        if f64::from(new_length) >= old_length {
            return self.ordinary_define_own_property(array, length_key, new_length_descriptor);
        }
        if !old_writable {
            return Ok(false);
        }
        let new_writable = new_length_descriptor.writable != Some(false);
        if !new_writable {
            new_length_descriptor.writable = Some(true);
        }
        if !self.ordinary_define_own_property(array, length_key, new_length_descriptor)? {
            return Ok(false);
        }
        // Step 17: delete each index from the old length down to the new one, the highest
        // first; the first that is not deleted stops it. A termination ends it before anything
        // is deleted, and the old length goes back, so that the length is never below an index
        // the Array still has (`docs/design/engine.md`, "Iterators held by a builtin").
        let stop = match self.truncate_elements(array, new_length) {
            Ok(stop) => stop,
            Err(Abrupt::Terminated(termination)) => {
                self.restore_length(array, length_key, old_length)?;
                return Err(Abrupt::Terminated(termination));
            }
            Err(abrupt) => return Err(abrupt),
        };
        if let Some(index) = stop {
            new_length_descriptor.value = Some(Value::Number(f64::from(index) + 1.0));
            if !new_writable {
                new_length_descriptor.writable = Some(false);
            }
            self.ordinary_define_own_property(array, length_key, new_length_descriptor)?;
            return Ok(false);
        }
        if !new_writable {
            let fixed = Descriptor {
                writable: Some(false),
                ..Descriptor::default()
            };
            self.ordinary_define_own_property(array, length_key, fixed)?;
        }
        Ok(true)
    }

    /// Put `old_length` back as the value of the length of `array` after a termination ended
    /// [`Agent::truncate_elements`], through the same define as step 15 of ArraySetLength. The
    /// property exists and its [[Writable]] was true, so only its value changes: no property is
    /// added, and no step and no byte is charged.
    fn restore_length(
        &mut self,
        array: ObjectRef,
        length_key: Key,
        old_length: f64,
    ) -> Completion<()> {
        let old = Descriptor {
            value: Some(Value::Number(old_length)),
            ..Descriptor::default()
        };
        self.ordinary_define_own_property(array, length_key, old)?;
        Ok(())
    }

    /// The deletions of step 17 of ArraySetLength: every element at `new_length` or above, the
    /// highest first, until one is not deleted, whose index is answered. Deleting an element
    /// runs no code and only a non-configurable one refuses, and every such one lives in the
    /// list, so the stop is the highest non-configurable index of the list at `new_length` or
    /// above, and everything above it goes. A step per element the loop deletes and for the one
    /// that stops it, as the loop of the clause makes, per hole of the dense part it passes,
    /// and, for a list above the small size, per entry of the list the search visits. The last
    /// is the list's length, charged before the search; the others depend on the stop the
    /// search finds and are charged after it, before anything is deleted. A list of at most
    /// [`SMALL`] entries is searched first and charged once. Every charge, of steps and of
    /// bytes, comes before the first change of the Array, so a termination leaves every element
    /// in place and [`Agent::array_set_length`] puts the old length back.
    ///
    /// The index keys the search finds at `new_length` or above are gathered for the deletions
    /// in a list with room for every entry of the property list, its bytes charged to the
    /// memory budget before the room is reserved (`docs/design/engine.md`, "Memory budget") and
    /// credited when it drops, on every way out.
    fn truncate_elements(&mut self, array: ObjectRef, new_length: u32) -> Completion<Option<u32>> {
        let list_entries = self.engine_object(array)?.properties.len();
        if list_entries > SMALL {
            self.charge_steps(u64::try_from(list_entries).unwrap_or(u64::MAX))?;
        }
        let bytes =
            u64::try_from(list_entries.saturating_mul(size_of::<u32>())).unwrap_or(u64::MAX);
        self.charge_bytes(bytes)?;
        let mut listed: Vec<u32> = Vec::new();
        if listed.try_reserve_exact(list_entries).is_err() {
            self.heap.accounting.credit(bytes);
            return Err(self.terminate(Termination::MemoryBudget));
        }
        let answer = self.truncate_listed(array, new_length, &mut listed);
        drop(listed);
        self.heap.accounting.credit(bytes);
        answer
    }

    /// The search, the steps and the deletions of [`Agent::truncate_elements`], the index keys
    /// at `new_length` or above gathered in `listed`, which has room for every entry of the
    /// property list.
    fn truncate_listed(
        &mut self,
        array: ObjectRef,
        new_length: u32,
        listed: &mut Vec<u32>,
    ) -> Completion<Option<u32>> {
        let record = self.engine_object(array)?;
        let ObjectKind::Array(storage) = &record.kind else {
            return Err(self.fault());
        };
        let mut stop: Option<u32> = None;
        for (key, property) in record.properties.iter() {
            #[cfg(test)]
            TRUNCATION_VISITS.with(|visits| visits.set(visits.get().saturating_add(1)));
            if let Key::Index(index) = *key
                && index >= new_length
            {
                listed.push(index);
                if !property.configurable && stop.is_none_or(|stop| index > stop) {
                    stop = Some(index);
                }
            }
        }
        let floor = stop.map_or(new_length, |stop| stop.saturating_add(1));
        let floor_position = usize::try_from(floor).unwrap_or(usize::MAX);
        let dense_visited = storage.elements.len().saturating_sub(floor_position);
        listed.retain(|&index| index >= floor);
        let steps = u64::try_from(dense_visited.saturating_add(listed.len()))
            .unwrap_or(u64::MAX)
            .saturating_add(u64::from(stop.is_some()));
        self.charge_steps(steps)?;
        let elements = &mut self.dense_mut(array)?.elements;
        if elements.len() > floor_position {
            elements.truncate(floor_position);
        }
        let record = self.engine_object_mut(array)?;
        for &index in listed.iter() {
            record.properties.remove(Key::Index(index));
        }
        Ok(stop)
    }
}

#[cfg(test)]
thread_local! {
    /// The entries of the list the walk of `truncate_elements` visited, in the counting build of
    /// the tests; a test resets it before it reads it.
    static TRUNCATION_VISITS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
mod tests {
    use super::TRUNCATION_VISITS;
    use crate::host::agent::Agent;
    use crate::host::hooks::test_host::{
        NoHooks, agent_with_realm, agent_with_realm_and, enter, limits,
    };
    use crate::host::outcome::{Abrupt, Termination};
    use crate::object::property::{Descriptor, Key, Property, SMALL};
    use crate::value::{ObjectRef, Value};
    use crate::{Outcome, Parsed, Realm, Source};

    fn number(i: u32) -> Value {
        Value::Number(f64::from(i))
    }

    fn set_length(agent: &mut Agent<NoHooks>, array: ObjectRef, value: u32) -> bool {
        let length = agent.name_key("length").unwrap();
        agent
            .define_own_property(
                array,
                length,
                Descriptor {
                    value: Some(number(value)),
                    ..Descriptor::default()
                },
            )
            .unwrap()
    }

    /// What a script can observe of `array`: its own keys and the descriptor of each, and the
    /// answer for each index below 64 it does not have.
    fn observed(agent: &mut Agent<NoHooks>, array: ObjectRef) -> Vec<(Key, Option<Property>)> {
        let mut seen: Vec<(Key, Option<Property>)> = agent
            .own_property_keys(array)
            .unwrap()
            .keys
            .into_iter()
            .map(|key| (key, agent.get_own_property(array, key).unwrap()))
            .collect();
        for index in 0..64 {
            let key = Key::Index(index);
            if !seen.iter().any(|(own, _)| *own == key) {
                seen.push((key, agent.get_own_property(array, key).unwrap()));
            }
        }
        seen
    }

    /// The Array [[DefineOwnProperty]] (`#sec-array-exotic-objects-defineownproperty-p-desc`),
    /// ArraySetLength (`#sec-arraysetlength`), [[Delete]] and [[OwnPropertyKeys]] answer alike
    /// over the dense storage and the list: one sequence of defines, writes, deletes and length
    /// changes on an Array kept dense and on one whose elements all live in the list.
    #[test]
    fn dense_and_sparse_elements_answer_alike() {
        let (mut agent, realm) = agent_with_realm();
        enter(&mut agent, &realm);
        let dense = agent.array_create(0.0, None).unwrap();
        let sparse = agent.array_create_sparse().unwrap();
        let name = agent.name_key("x").unwrap();
        let mut answers = Vec::new();
        for array in [dense, sparse] {
            let mut answer = Vec::new();
            for i in 0..20 {
                answer.push(agent.create_data_property(array, Key::Index(i), number(i)));
            }
            answer.push(agent.create_data_property(array, name, Value::Null));
            // An element that is no longer writable, one that is an accessor, a hole.
            answer.push(agent.define_own_property(
                array,
                Key::Index(5),
                Descriptor {
                    writable: Some(false),
                    ..Descriptor::default()
                },
            ));
            answer.push(agent.define_own_property(
                array,
                Key::Index(7),
                Descriptor {
                    get: Some(None),
                    set: Some(None),
                    ..Descriptor::default()
                },
            ));
            answer.push(agent.internal_delete(array, Key::Index(3)));
            answer.push(agent.set(array, Key::Index(5), Value::Null, false));
            answer.push(agent.set(array, Key::Index(2), Value::Boolean(true), false));
            answer.push(agent.set(array, Key::Index(40), number(40), false));
            answer.push(Ok(set_length(&mut agent, array, 15)));
            answer.push(agent.create_data_property(array, Key::Index(3), number(33)));
            // A non-configurable element stops a truncation above it.
            answer.push(agent.define_own_property(
                array,
                Key::Index(10),
                Descriptor {
                    configurable: Some(false),
                    ..Descriptor::default()
                },
            ));
            answer.push(Ok(set_length(&mut agent, array, 4)));
            answer.push(agent.internal_delete(array, Key::Index(10)));
            answer.push(agent.set(array, Key::Index(12), number(12), false));
            let length = agent.name_key("length").unwrap();
            answer.push(Ok(agent.get(array, length).unwrap() == number(13)));
            answers.push((answer, observed(&mut agent, array)));
        }
        assert_eq!(answers[0], answers[1]);
        assert_eq!(answers[0].0.iter().filter(|a| **a == Ok(false)).count(), 3);
    }

    /// An index far beyond the dense part lives in the list: `a[4294967294] = 1` under a budget
    /// of 1 MiB beyond what the agent holds answers, and the dense storage is not allocated.
    #[test]
    fn an_index_far_beyond_the_dense_part_does_not_allocate_it() {
        let (mut agent, realm) = agent_with_realm();
        enter(&mut agent, &realm);
        let array = agent.array_create(0.0, None).unwrap();
        let used = agent.heap.bytes();
        agent.heap.accounting.set_limit(used + (1 << 20));
        let last = Key::Index(4_294_967_294);
        assert_eq!(agent.set(array, last, number(1), true), Ok(true));
        assert_eq!(agent.get(array, last), Ok(number(1)));
        let length = agent.name_key("length").unwrap();
        assert_eq!(agent.get(array, length), Ok(Value::Number(4_294_967_295.0)));
        assert_eq!(agent.dense_capacity(array), Ok(0));
        assert!(agent.heap.bytes() < used + 4096);
    }

    /// The memory budget (decision 4): the dense storage grows as the list does, charged
    /// before it is made, refused over the budget with nothing charged, and credited when the
    /// Array is freed.
    #[test]
    fn the_dense_storage_is_charged_refused_and_credited() {
        let (mut agent, realm) = agent_with_realm();
        enter(&mut agent, &realm);
        let empty = agent.heap.bytes();
        let array = agent.array_create(0.0, None).unwrap();
        let made = agent.heap.bytes();
        agent.heap.accounting.set_limit(made);
        assert_eq!(
            agent.create_data_property(array, Key::Index(0), Value::Null),
            Err(Abrupt::Terminated(Termination::MemoryBudget))
        );
        assert_eq!(agent.heap.bytes(), made);
        agent.termination = None;
        agent.heap.accounting.set_limit(1 << 24);
        for i in 0..100 {
            assert_eq!(
                agent.create_data_property(array, Key::Index(i), number(i)),
                Ok(true)
            );
        }
        assert!(agent.dense_capacity(array).unwrap() >= 100);
        assert!(agent.heap.bytes() > made);
        assert!(agent.collect().is_ok());
        assert_eq!(agent.heap.bytes(), empty);
    }

    /// The memory budget (`docs/design/engine.md`, "Memory budget"): a Set at the end of the
    /// dense part, the Array its own receiver, charges the growth of the storage before it is
    /// made. An Array of 4 elements, which fill its first room, under a budget of what the
    /// agent holds: `a[4] = 4` ends in `Terminated(MemoryBudget)` with nothing charged, and the
    /// Array keeps its elements, its room and its length; with room the store is made. The
    /// defect it holds: a store at the end that grows the storage without its charge.
    #[test]
    fn a_set_at_the_end_charges_the_growth_before_it_is_made() {
        let (mut agent, realm) = agent_with_realm();
        enter(&mut agent, &realm);
        let array = agent.array_create(0.0, None).unwrap();
        let length = agent.name_key("length").unwrap();
        for i in 0..4 {
            assert_eq!(agent.set(array, Key::Index(i), number(i), true), Ok(true));
        }
        assert_eq!(agent.dense_capacity(array), Ok(4));
        let held = agent.heap.bytes();
        agent.heap.accounting.set_limit(held);
        assert_eq!(
            agent.set(array, Key::Index(4), number(4), true),
            Err(Abrupt::Terminated(Termination::MemoryBudget))
        );
        assert_eq!(agent.heap.bytes(), held);
        agent.termination = None;
        agent.heap.accounting.set_limit(held + (1 << 20));
        assert_eq!(agent.get(array, length), Ok(number(4)));
        assert_eq!(agent.get_own_property(array, Key::Index(4)), Ok(None));
        assert_eq!(agent.dense_capacity(array), Ok(4));
        assert_eq!(agent.set(array, Key::Index(4), number(4), true), Ok(true));
        assert_eq!(agent.get(array, length), Ok(number(5)));
        assert_eq!(agent.dense_capacity(array), Ok(8));
    }

    /// The state the integrity operations make (`#sec-setintegritylevel`): an Array made
    /// non-extensible, each element defined again with [[Writable]] and [[Configurable]] false,
    /// in dense and in sparse storage. A Set, a define, a delete and a change of `length` are
    /// refused alike, a new element too, and every element keeps its value.
    #[test]
    fn a_non_extensible_array_with_read_only_elements_refuses_every_change() {
        let (mut agent, realm) = agent_with_realm();
        enter(&mut agent, &realm);
        let dense = agent.array_create(0.0, None).unwrap();
        let sparse = agent.array_create_sparse().unwrap();
        let length = agent.name_key("length").unwrap();
        for array in [dense, sparse] {
            for i in 0..3 {
                assert_eq!(
                    agent.create_data_property(array, Key::Index(i), number(i)),
                    Ok(true)
                );
            }
            assert_eq!(agent.prevent_extensions(array), Ok(true));
            for i in 0..3 {
                let fixed = Descriptor {
                    writable: Some(false),
                    configurable: Some(false),
                    ..Descriptor::default()
                };
                assert_eq!(
                    agent.define_own_property(array, Key::Index(i), fixed),
                    Ok(true)
                );
            }
            assert_eq!(
                agent.set(array, Key::Index(1), Value::Null, false),
                Ok(false)
            );
            assert_eq!(
                agent.define_own_property(
                    array,
                    Key::Index(1),
                    Descriptor {
                        value: Some(Value::Null),
                        ..Descriptor::default()
                    }
                ),
                Ok(false)
            );
            assert_eq!(agent.internal_delete(array, Key::Index(1)), Ok(false));
            assert!(!set_length(&mut agent, array, 0));
            assert_eq!(
                agent.set(array, Key::Index(3), Value::Null, false),
                Ok(false)
            );
            assert_eq!(agent.get(array, length), Ok(number(3)));
            for i in 0..3 {
                assert_eq!(
                    agent.get_own_property(array, Key::Index(i)),
                    Ok(Some(Property::data(number(i), false, true, false)))
                );
            }
            assert_eq!(agent.get_own_property(array, Key::Index(3)), Ok(None));
        }
    }

    /// ArraySetLength (`#sec-arraysetlength`) and the Array [[DefineOwnProperty]]
    /// (`#sec-array-exotic-objects-defineownproperty-p-desc`): a smaller length deletes the
    /// indices above it from the highest down and stops at a non-configurable one; a
    /// non-writable length refuses a larger index and a larger length; a length that is not a
    /// uint32 throws a RangeError.
    #[test]
    fn array_length_truncates_and_refuses_non_writable() {
        let (mut agent, realm) = agent_with_realm();
        enter(&mut agent, &realm);
        let array = agent.array_create(0.0, None).unwrap();
        let length = agent.name_key("length").unwrap();
        for index in 0..5 {
            assert_eq!(
                agent.create_data_property(
                    array,
                    Key::Index(index),
                    Value::Number(f64::from(index))
                ),
                Ok(true)
            );
        }
        assert_eq!(agent.get(array, length), Ok(Value::Number(5.0)));

        let set_length = |agent: &mut crate::host::agent::Agent<_>, value: f64| {
            agent.define_own_property(
                array,
                length,
                Descriptor {
                    value: Some(Value::Number(value)),
                    ..Descriptor::default()
                },
            )
        };
        assert_eq!(set_length(&mut agent, 3.0), Ok(true));
        assert_eq!(agent.get(array, length), Ok(Value::Number(3.0)));
        assert_eq!(agent.has_property(array, Key::Index(3)), Ok(false));
        assert_eq!(agent.has_property(array, Key::Index(2)), Ok(true));

        // A non-configurable index stops the truncation above it.
        let fixed = Descriptor {
            configurable: Some(false),
            ..Descriptor::default()
        };
        assert_eq!(
            agent.define_own_property(array, Key::Index(1), fixed),
            Ok(true)
        );
        assert_eq!(set_length(&mut agent, 0.0), Ok(false));
        assert_eq!(agent.get(array, length), Ok(Value::Number(2.0)));
        assert_eq!(agent.has_property(array, Key::Index(1)), Ok(true));
        assert_eq!(agent.has_property(array, Key::Index(0)), Ok(true));

        // A length that is not a uint32 is a RangeError.
        assert!(matches!(set_length(&mut agent, 1.5), Err(Abrupt::Throw(_))));
        assert!(matches!(
            set_length(&mut agent, -1.0),
            Err(Abrupt::Throw(_))
        ));

        // A non-writable length refuses a larger index and a larger length.
        let frozen = Descriptor {
            writable: Some(false),
            ..Descriptor::default()
        };
        assert_eq!(agent.define_own_property(array, length, frozen), Ok(true));
        assert_eq!(
            agent.create_data_property(array, Key::Index(5), Value::Null),
            Ok(false)
        );
        assert_eq!(set_length(&mut agent, 9.0), Ok(false));
        assert_eq!(agent.get(array, length), Ok(Value::Number(2.0)));
        // An index below the length can still be written.
        assert_eq!(agent.set(array, Key::Index(0), Value::Null, true), Ok(true));
    }

    /// An agent with a realm and no step limit to reach, and a memory budget of what the realm
    /// holds plus 256 MiB.
    fn roomy() -> (Agent<NoHooks>, Realm) {
        let mut limits = limits();
        limits.steps_per_entry = u64::MAX;
        limits.memory_bytes = 1 << 30;
        let (mut agent, realm) = agent_with_realm_and(limits);
        let held = agent.heap.bytes();
        agent.heap.accounting.set_limit(held + (1 << 28));
        (agent, realm)
    }

    fn length_descriptor(value: u32) -> Descriptor {
        Descriptor {
            value: Some(number(value)),
            ..Descriptor::default()
        }
    }

    /// The entries the walks of `truncate_elements` visited since the count was reset.
    fn truncation_visits() -> u64 {
        TRUNCATION_VISITS.with(std::cell::Cell::get)
    }

    /// An Array with the dense elements 0 to 2 and `keys` index keys in its list from 2,000 up,
    /// beyond the holes a write may open; with `fixed` the lowest of them is non-configurable.
    fn listed_array(agent: &mut Agent<NoHooks>, keys: u32, fixed: bool) -> ObjectRef {
        let array = agent.array_create(0.0, None).unwrap();
        for i in 0..3 {
            assert_eq!(
                agent.create_data_property(array, Key::Index(i), number(i)),
                Ok(true)
            );
        }
        for i in 0..keys {
            assert_eq!(
                agent.create_data_property(array, Key::Index(2_000 + i), number(i)),
                Ok(true)
            );
        }
        if fixed {
            let fixed = Descriptor {
                configurable: Some(false),
                ..Descriptor::default()
            };
            assert_eq!(
                agent.define_own_property(array, Key::Index(2_000), fixed),
                Ok(true)
            );
        }
        let listed = agent.engine_object(array).unwrap().properties.len();
        assert_eq!(listed, keys as usize + 1, "the index keys and \"length\"");
        array
    }

    /// `docs/design/engine.md`, "Memory budget" (work is charged before it is done), and
    /// ArraySetLength (`#sec-arraysetlength`), step 17: an Array with 100,000 index keys in its
    /// list above its dense part and 10 steps left. `a.length = 0` ends in
    /// `Terminated(StepBudget)` before the walk of the list visits an entry, since the list's
    /// length is charged first.
    #[test]
    fn setting_a_length_charges_before_the_walk() {
        const N: u32 = 100_000;
        let (mut agent, realm) = roomy();
        enter(&mut agent, &realm);
        let array = listed_array(&mut agent, N, false);
        let length = agent.name_key("length").unwrap();
        agent.steps = 0;
        agent.limits.steps_per_entry = 10;
        TRUNCATION_VISITS.with(|visits| visits.set(0));
        assert_eq!(
            agent.define_own_property(array, length, length_descriptor(0)),
            Err(Abrupt::Terminated(Termination::StepBudget))
        );
        assert_eq!(
            truncation_visits(),
            0,
            "entries the walk visited before the step budget ended the run"
        );
    }

    /// A guard of ArraySetLength's steps (`#sec-arraysetlength`) on both sides of the small
    /// size: Arrays with 0, 1, 7, 8 and 9 index keys in their list (with `"length"`, lists of
    /// 1, 2, 8, 9 and 10 entries), with and without a non-configurable index at the lowest of
    /// them. The steps of `a.length = 1`, its answer and the entries the walk visited are those
    /// recorded on the base of the change that charges the list's length before the walk.
    #[test]
    fn truncating_a_small_list_charges_as_today() {
        assert_eq!(SMALL, 8);
        // (index keys, one non-configurable, steps, answer, entries visited)
        const RECORDED: [(u32, bool, u64, bool, u64); 9] = [
            (0, false, 2, true, 1),
            (1, false, 3, true, 2),
            (1, true, 1, false, 2),
            (7, false, 9, true, 8),
            (7, true, 7, false, 8),
            (8, false, 19, true, 9),
            (8, true, 17, false, 9),
            (9, false, 21, true, 10),
            (9, true, 19, false, 10),
        ];
        let (mut agent, realm) = roomy();
        enter(&mut agent, &realm);
        let length = agent.name_key("length").unwrap();
        let mut seen = Vec::new();
        for &(keys, fixed, ..) in &RECORDED {
            let array = listed_array(&mut agent, keys, fixed);
            TRUNCATION_VISITS.with(|visits| visits.set(0));
            let before = agent.steps;
            let answer = agent
                .define_own_property(array, length, length_descriptor(1))
                .unwrap();
            seen.push((
                keys,
                fixed,
                agent.steps - before,
                answer,
                truncation_visits(),
            ));
        }
        assert_eq!(seen, RECORDED.to_vec());
    }

    /// Evaluate `source` in `realm`, which must complete normally, and answer the steps its
    /// entry spent.
    fn entry_steps(agent: &mut Agent<NoHooks>, realm: &Realm, source: &str) -> u64 {
        let Ok(Parsed::Parsed(script)) = agent.parse_script(realm, Source::Utf8(source), None)
        else {
            panic!("{source} did not parse");
        };
        let outcome = agent.evaluate_script(&script);
        assert!(
            matches!(outcome, Ok(Outcome::Normal(_))),
            "{source}: {outcome:?}"
        );
        agent.last_entry_steps().unwrap()
    }

    /// A guard of the step totals of the three operations that charge before their work: the
    /// steps of each entry below, after a setup entry that builds Arrays with 0, 8, 9 and 1,000
    /// index keys in their list, an object whose list compacts at its next addition, a function
    /// whose eval code adds bindings until their list compacts, and bound functions of 0, 1 and
    /// 1,000 bound arguments per link through chains of 1 and 3 links. The totals are those
    /// recorded on the base of the change `c8f39a4`, but for `f()`: since the frontend charges,
    /// `eval` charges a step per code unit of its source before it parses
    /// (`dynamic_code.rs`, `parse_eval_code`), so `f()` carries the 156 code units of its ten
    /// `eval` sources (72, eight times `delete b<i>` with 75 in sum, and 9) on top of its 341
    /// there. `o.x = 1` compacts the list (16 entries visited) and `o.y = 1`, the same
    /// instructions, does not.
    #[test]
    fn step_totals_are_unchanged() {
        const SETUP: &str = "var a0 = [0, 1, 2]; \
            var a8 = [0, 1, 2]; a8.length = 1036; a8.fill(7, 1028); \
            var a9 = [0, 1, 2]; a9.length = 1037; a9.fill(7, 1028); \
            var a1000 = [0, 1, 2]; a1000.length = 2028; a1000.fill(7, 1028); \
            var o = {}; \
            for (var i = 0; i < 16; i++) o['k' + i] = i; \
            for (i = 0; i < 16; i += 2) delete o['k' + i]; \
            function f() { \
              eval('var b0, b1, b2, b3, b4, b5, b6, b7, b8, b9, b10, b11, b12, b13, b14, b15'); \
              for (var i = 0; i < 16; i += 2) eval('delete b' + i); \
              eval('var x = 1'); \
              return x; \
            } \
            function t() { return arguments.length; } \
            var bind = Function.prototype.bind; \
            var b0x1 = t.bind(null), b1x1 = t.bind(null, 1); \
            var b1000x1 = bind.apply(t, new Array(1001)); \
            var b0x3 = b0x1.bind(null).bind(null), b1x3 = b1x1.bind(null, 1).bind(null, 1); \
            var b1000x3 = bind.apply(bind.apply(b1000x1, new Array(1001)), new Array(1001)); \
            0";
        const RECORDED: [(&str, u64); 13] = [
            ("a0.length = 1", 9),
            ("a8.length = 1", 26),
            ("a9.length = 1", 28),
            ("a1000.length = 1", 2_010),
            ("o.x = 1", 24),
            ("o.y = 1", 8),
            ("f()", 497),
            ("b0x1()", 12),
            ("b1x1()", 14),
            ("b1000x1()", 2_012),
            ("b0x3()", 14),
            ("b1x3()", 20),
            ("b1000x3()", 6_014),
        ];
        let (mut agent, realm) = roomy();
        entry_steps(&mut agent, &realm, SETUP);
        let seen: Vec<(&str, u64)> = RECORDED
            .iter()
            .map(|&(source, _)| (source, entry_steps(&mut agent, &realm, source)))
            .collect();
        assert_eq!(seen, RECORDED.to_vec());
        let steps = |source: &str| seen.iter().find(|(s, _)| *s == source).map(|(_, n)| *n);
        assert_eq!(
            steps("o.x = 1").zip(steps("o.y = 1")).map(|(x, y)| x - y),
            Some(16)
        );
    }

    /// `source`, parsed in `realm`.
    fn parsed(agent: &mut Agent<NoHooks>, realm: &Realm, source: &str) -> crate::value::Script {
        let Ok(Parsed::Parsed(script)) = agent.parse_script(realm, Source::Utf8(source), None)
        else {
            panic!("{source} did not parse");
        };
        script
    }

    /// `script`, evaluated, which must not throw: the Number it answers, `None` for another
    /// value, or the termination that ended it. `realm` is entered again after it.
    fn run(
        agent: &mut Agent<NoHooks>,
        realm: &Realm,
        script: &crate::value::Script,
    ) -> Result<Option<f64>, Termination> {
        let outcome = match agent.evaluate_script(script) {
            Ok(Outcome::Normal(root)) => match agent.heap.roots.get(&root) {
                Ok(Value::Number(number)) => Ok(Some(number)),
                _ => Ok(None),
            },
            Ok(Outcome::Terminated(termination)) => Err(termination),
            other => panic!("{other:?}"),
        };
        enter(agent, realm);
        outcome
    }

    /// `docs/design/engine.md`, "Memory budget" (storage is charged before it is allocated, and
    /// a temporary structure is credited when it drops), and ArraySetLength
    /// (`#sec-arraysetlength`), step 17: the list of the index keys at the new length or above
    /// that `truncate_elements` gathers for the deletions. `a`, `b` and `c` hold 10,000 index
    /// keys in their lists (10,001 entries with `"length"`; a list of `u32` for all of them takes
    /// 40,004 bytes). Under a budget of the reading after the setup plus half of that,
    /// `a.length = 0` ends in `Terminated(MemoryBudget)` before the walk visits an entry; the
    /// next scripts `a.length` and `c.length` answer 12,000, and after a collection the reading
    /// is the one before. A step budget that ends `b.length = 0` after the list was charged
    /// leaves the charged reading where it was; with a roomy budget `c.length = 0` raises the
    /// high-water mark by at least the list and, done, leaves the reading where it was.
    #[test]
    fn the_list_of_deleted_keys_is_charged_before_it_is_made() {
        const N: u32 = 10_000;
        let (mut agent, realm) = roomy();
        enter(&mut agent, &realm);
        let global = agent.global_object(&realm).unwrap();
        let Ok(Value::Object(global)) = agent.heap.roots.get(&global) else {
            panic!("the global object");
        };
        let [_, b, c] = ["a", "b", "c"].map(|name| {
            let array = listed_array(&mut agent, N, false);
            let key = agent.name_key(name).unwrap();
            assert_eq!(
                agent.create_data_property(global, key, Value::Object(array)),
                Ok(true)
            );
            array
        });
        let shorten = parsed(&mut agent, &realm, "a.length = 0");
        let read_a = parsed(&mut agent, &realm, "a.length");
        let read = parsed(&mut agent, &realm, "c.length");
        let list = u64::from(N + 1) * size_of::<u32>() as u64;
        let roomy_limit = agent.heap.bytes() + (1 << 28);
        assert!(agent.collect().is_ok());
        let before = agent.heap_bytes().unwrap();
        agent.heap.accounting.set_limit(before + list / 2);
        TRUNCATION_VISITS.with(|visits| visits.set(0));
        assert_eq!(
            run(&mut agent, &realm, &shorten),
            Err(Termination::MemoryBudget)
        );
        assert_eq!(
            truncation_visits(),
            0,
            "entries the walk visited before the memory budget ended the run"
        );
        agent.heap.accounting.set_limit(roomy_limit);
        assert_eq!(
            run(&mut agent, &realm, &read_a),
            Ok(Some(f64::from(2_000 + N))),
            "the length of a after the memory budget"
        );
        assert_eq!(
            run(&mut agent, &realm, &read),
            Ok(Some(f64::from(2_000 + N)))
        );
        assert!(agent.collect().is_ok());
        assert_eq!(agent.heap_bytes().unwrap(), before, "after the next script");

        let length = agent.name_key("length").unwrap();
        let charged = agent.heap.accounting.used();
        agent.steps = 0;
        agent.limits.steps_per_entry = u64::from(N) + 100;
        assert_eq!(
            agent.define_own_property(b, length, length_descriptor(0)),
            Err(Abrupt::Terminated(Termination::StepBudget))
        );
        assert_eq!(agent.heap.accounting.used(), charged, "after a step budget");
        agent.steps = 0;
        agent.limits.steps_per_entry = u64::MAX;
        agent.termination = None;
        agent.heap.accounting.reset_high_water();
        assert_eq!(
            agent.define_own_property(c, length, length_descriptor(0)),
            Ok(true)
        );
        let rise = agent.heap.accounting.high_water() - charged;
        assert!(rise >= list, "the high-water mark rose by {rise} < {list}");
        assert_eq!(agent.heap.accounting.used(), charged, "after a normal end");
    }

    /// ArraySetLength (`#sec-arraysetlength`) and the invariant of an Array exotic object that
    /// every array index is below its `length` (`#sec-array-exotic-objects`): a termination that
    /// ends the deletions of step 17 puts the old length back (`docs/design/engine.md`,
    /// "Iterators held by a builtin"). `a`, `b`, `c` and `d` hold the dense elements 0 to 2 and
    /// 10,000 index keys from 2,000 up. The step budgets are a measured reading plus a stated
    /// room: `S`, the steps of `d.length = 0`; `L`, the list's length (10,001 entries with
    /// `"length"`), charged before the search; `R`, the steps charged after the search,
    /// 3 + 10,000 (the dense positions passed and the keys removed, no stop). `a.length = 0`
    /// with `S - R - L / 2` steps ends at the charge of the list's length, `b.length = 0` with
    /// `S - R / 2` steps at the charge after the search; each keeps its length 12,000 and its
    /// 10,004 own property names, each `"length"` or an index below the length (with
    /// `"length"` taken out, `Math.max` of the names is below it, and `NaN` for a name that is
    /// no number). With a roomy budget `c.length = 0` completes and leaves `"length"` alone.
    #[test]
    fn a_terminated_truncation_leaves_the_array_as_it_was() {
        const N: u32 = 10_000;
        let (mut agent, realm) = roomy();
        enter(&mut agent, &realm);
        let global = agent.global_object(&realm).unwrap();
        let Ok(Value::Object(global)) = agent.heap.roots.get(&global) else {
            panic!("the global object");
        };
        let [a, ..] = ["a", "b", "c", "d"].map(|name| {
            let array = listed_array(&mut agent, N, false);
            let key = agent.name_key(name).unwrap();
            assert_eq!(
                agent.create_data_property(global, key, Value::Object(array)),
                Ok(true)
            );
            array
        });
        let list_entries = agent.engine_object(a).unwrap().properties.len() as u64;
        assert_eq!(list_entries, u64::from(N) + 1, "L, the list's length");
        let after_search = 3 + u64::from(N);
        let whole = entry_steps(&mut agent, &realm, "d.length = 0");
        assert!(whole > after_search + list_entries, "S = {whole}");

        let shorten_a = parsed(&mut agent, &realm, "a.length = 0");
        let shorten_b = parsed(&mut agent, &realm, "b.length = 0");
        agent.limits.steps_per_entry = whole - after_search - list_entries / 2;
        assert_eq!(
            run(&mut agent, &realm, &shorten_a),
            Err(Termination::StepBudget),
            "at the charge of the list's length"
        );
        agent.limits.steps_per_entry = whole - after_search / 2;
        assert_eq!(
            run(&mut agent, &realm, &shorten_b),
            Err(Termination::StepBudget),
            "at the charge after the search"
        );

        agent.limits.steps_per_entry = u64::MAX;
        for name in ["a", "b"] {
            let answer = |agent: &mut Agent<NoHooks>, source: &str| {
                let script = parsed(agent, &realm, source);
                run(agent, &realm, &script)
            };
            assert_eq!(
                answer(&mut agent, &format!("{name}.length")),
                Ok(Some(f64::from(2_000 + N))),
                "the length of {name}"
            );
            assert_eq!(
                answer(
                    &mut agent,
                    &format!("Object.getOwnPropertyNames({name}).length")
                ),
                Ok(Some(f64::from(N + 4))),
                "the own property names of {name}"
            );
            // Without a loop of the script's own: under `collect-every-safe-point` each
            // instruction collects, and a loop over 10,004 names ran past 60 s.
            let below = format!(
                "var names = Object.getOwnPropertyNames({name}); \
                 names.splice(names.indexOf('length'), 1); \
                 Math.max.apply(null, names) < {name}.length ? 1 : 0"
            );
            assert_eq!(
                answer(&mut agent, &below),
                Ok(Some(1.0)),
                "every name of {name} is \"length\" or an index below the length"
            );
        }
        let shorten_c = parsed(&mut agent, &realm, "c.length = 0");
        assert_eq!(run(&mut agent, &realm, &shorten_c), Ok(Some(0.0)));
        let names_c = parsed(&mut agent, &realm, "Object.getOwnPropertyNames(c).length");
        assert_eq!(run(&mut agent, &realm, &names_c), Ok(Some(1.0)));
    }
}
