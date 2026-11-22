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

use super::ObjectKind;
use super::ordinary::applied_property;
use super::property::{Descriptor, Key, Property, SMALL, Slot};
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
    /// list merged ascending. A step per key, per hole the walk passes and per comparison of a
    /// sort of the list's indices.
    pub(crate) fn array_own_property_keys(&mut self, array: ObjectRef) -> Completion<Vec<Key>> {
        let record = self.engine_object(array)?;
        let ObjectKind::Array(storage) = &record.kind else {
            return Err(self.fault());
        };
        let (listed, comparisons) = record.properties.keys();
        let holes = storage
            .elements
            .iter()
            .filter(|slot| slot.is_none())
            .count();
        let mut keys = Vec::new();
        let mut sparse = listed.iter().copied().peekable();
        for (position, slot) in storage.elements.iter().enumerate() {
            if slot.is_none() {
                continue;
            }
            let Ok(index) = u32::try_from(position) else {
                break;
            };
            while let Some(&Key::Index(other)) = sparse.peek()
                && other < index
            {
                keys.push(Key::Index(other));
                sparse.next();
            }
            keys.push(Key::Index(index));
        }
        keys.extend(sparse);
        let steps = u64::try_from(keys.len().saturating_add(holes)).unwrap_or(u64::MAX);
        self.charge_steps(steps.saturating_add(comparisons))?;
        Ok(keys)
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
        // first; the first that is not deleted stops it.
        let stop = self.truncate_elements(array, new_length)?;
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

    /// The deletions of step 17 of ArraySetLength: every element at `new_length` or above, the
    /// highest first, until one is not deleted, whose index is answered. Deleting an element
    /// runs no code and only a non-configurable one refuses, and every such one lives in the
    /// list, so the stop is the highest non-configurable index of the list at `new_length` or
    /// above, and everything above it goes. A step per element the loop deletes and for the one
    /// that stops it, as the loop of the clause makes, per hole of the dense part it passes,
    /// and, for a list above the small size, per entry of the list the search visits.
    fn truncate_elements(&mut self, array: ObjectRef, new_length: u32) -> Completion<Option<u32>> {
        let record = self.engine_object(array)?;
        let ObjectKind::Array(storage) = &record.kind else {
            return Err(self.fault());
        };
        let list_entries = record.properties.len();
        let mut stop: Option<u32> = None;
        let mut listed: Vec<u32> = Vec::new();
        for (key, property) in record.properties.iter() {
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
        let mut steps = u64::try_from(dense_visited.saturating_add(listed.len()))
            .unwrap_or(u64::MAX)
            .saturating_add(u64::from(stop.is_some()));
        if list_entries > SMALL {
            steps = steps.saturating_add(u64::try_from(list_entries).unwrap_or(u64::MAX));
        }
        self.charge_steps(steps)?;
        let elements = &mut self.dense_mut(array)?.elements;
        if elements.len() > floor_position {
            elements.truncate(floor_position);
        }
        let record = self.engine_object_mut(array)?;
        for index in listed {
            record.properties.remove(Key::Index(index));
        }
        Ok(stop)
    }
}

#[cfg(test)]
mod tests {
    use crate::host::agent::Agent;
    use crate::host::hooks::test_host::{NoHooks, agent_with_realm, enter};
    use crate::host::outcome::{Abrupt, Termination};
    use crate::object::property::{Descriptor, Key, Property};
    use crate::value::{ObjectRef, Value};

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
}
