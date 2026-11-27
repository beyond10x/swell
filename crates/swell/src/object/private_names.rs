//! [[PrivateElements]] of an object (`#sec-privateelement-specification-type`;
//! `docs/design/engine.md`, "Private names", item 2), written by
//! `story:private-names-and-private-methods`: the one add function of every private element,
//! PrivateElementFind (`#sec-privateelementfind`), PrivateGet (`#sec-privateget`) and PrivateSet
//! (`#sec-privateset`).
//!
//! The elements of an object are a list in the box `ObjectRecord::private_elements`, made at the
//! first add, so an object without one pays the box's 8 bytes only. Each entry is a Private Name
//! (`heap/mod.rs`, `PrivateNameRef`) with a field (its value), a method (its function) or an
//! accessor (an optional getter and an optional setter); `story:private-fields-and-private-in`
//! added the fields (PrivateFieldAdd, `#sec-privatefieldadd`) through the same add. The list
//! grows only when it is full, by exactly the elements of the class level whose add found it
//! full (the `room` of `AddPrivateMethod` and `AddPrivateField`, its methods, accessors and
//! fields counted together, fixed when the class is compiled), so a construction reserves once
//! per level and an instance holds no spare entry. An add to an object that is no instance of
//! the class (a base constructor that returned another object) finds its list full like any
//! other and reserves the level's room, charged before it is reserved. The box and
//! every growth of its list are charged to the memory budget before they are made and recorded
//! with the object (`Agent::record_grown`), so the sweep that frees it credits them; the list
//! never shrinks. Elements are no properties: no internal method, no
//! trap of a `Proxy` and no listing of a host reads them.
//!
//! PrivateElementFind compares the entries in order and spends a step per comparison past
//! [`SMALL`] (`object/property.rs`), so a list of n elements costs at most n − `SMALL` steps per
//! lookup, counted before the step is charged.
//!
//! The collector traces each entry's Private Name and its values (a field's value, a method's or
//! an accessor's functions) at the object record's site ([`PrivateElements::trace`], called by
//! `ObjectRecord::trace`). A field's value is reached through its object alone: no private
//! record holds it, as one holds a method's function.

use alloc::boxed::Box;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use crate::heap::PrivateNameRef;
use crate::heap::collect::{Item, Mark};
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::host::{ErrorKind, Host, Termination};
use crate::object::ObjectKind;
use crate::object::property::SMALL;
use crate::value::{ObjectRef, Value};
use swell_syntax::ast::FunctionKind;

/// What a private element holds (`#sec-privateelement-specification-type`, [[Kind]] and its
/// slots): a field's value, a method's function, or an accessor's getter and setter, either of
/// which may be missing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum PrivateValue {
    Field(Value),
    Method(ObjectRef),
    Accessor {
        get: Option<ObjectRef>,
        set: Option<ObjectRef>,
    },
}

/// One private element: its [[Key]] and what it holds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PrivateElement {
    pub(crate) key: PrivateNameRef,
    pub(crate) value: PrivateValue,
}

/// The [[PrivateElements]] list of one object, in the order the elements were added.
#[derive(Default)]
pub(crate) struct PrivateElements {
    entries: Vec<PrivateElement>,
}

impl PrivateElements {
    /// The part of the object record's site that reports the elements: each entry's Private
    /// Name and its values.
    pub(crate) fn trace(&self, mark: &mut Mark) {
        for entry in &self.entries {
            mark.push(Item::PrivateName(entry.key.handle));
            match entry.value {
                PrivateValue::Field(value) => mark.value(value),
                PrivateValue::Method(function) => mark.object(function),
                PrivateValue::Accessor { get, set } => {
                    for function in get.into_iter().chain(set) {
                        mark.object(function);
                    }
                }
            }
        }
    }

    /// The position of the element of `key` and how many entries were compared to find it, or
    /// to find that there is none.
    fn find(&self, key: PrivateNameRef) -> (Option<usize>, usize) {
        match self.entries.iter().position(|entry| entry.key == key) {
            Some(position) => (Some(position), position.saturating_add(1)),
            None => (None, self.entries.len()),
        }
    }

    /// The number of elements.
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }

    /// The number of elements the list has room for.
    #[cfg(test)]
    pub(crate) fn capacity(&self) -> usize {
        self.entries.capacity()
    }
}

/// What a function added by `AddPrivateMethod` is, by the kind of its template.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MethodKind {
    Method,
    Getter,
    Setter,
}

impl<H: Host> Agent<H> {
    /// PrivateElementFind(`object`, `key`) (`#sec-privateelementfind`): the position of the
    /// element, `None` when `object` has none. The comparisons past [`SMALL`] are charged a step
    /// each, once the scan counted them.
    pub(crate) fn private_element_find(
        &mut self,
        object: ObjectRef,
        key: PrivateNameRef,
    ) -> Completion<Option<usize>> {
        let (position, compared) = match &self.engine_object(object)?.private_elements {
            Some(elements) => elements.find(key),
            None => (None, 0),
        };
        let past = compared.saturating_sub(SMALL);
        if past > 0 {
            self.charge_steps(u64::try_from(past).unwrap_or(u64::MAX))?;
        }
        Ok(position)
    }

    /// The element at `position` of `object`, which [`Agent::private_element_find`] answered.
    fn private_element(&mut self, object: ObjectRef, position: usize) -> Completion<PrivateValue> {
        let found = self
            .engine_object(object)?
            .private_elements
            .as_ref()
            .and_then(|elements| elements.entries.get(position))
            .map(|entry| entry.value);
        match found {
            Some(value) => Ok(value),
            None => Err(self.fault()),
        }
    }

    /// The [[Description]] of `key` as text, for a message.
    pub(crate) fn private_name_text(&mut self, key: PrivateNameRef) -> String {
        self.heap
            .private_names
            .get(key.handle)
            .ok()
            .and_then(|record| self.heap.strings.get(record.description.handle).ok())
            .map(|string| string.to_rust_string_lossy())
            .unwrap_or_default()
    }

    /// PrivateGet(`object`, `key`) (`#sec-privateget`): a field's value (step 3), a method's
    /// function, or its getter called with `object` as `this`; a `TypeError` when `object` has
    /// no element of `key` (step 2) or the accessor has no getter (step 5.b).
    pub(crate) fn private_get(
        &mut self,
        object: ObjectRef,
        key: PrivateNameRef,
    ) -> Completion<Value> {
        let Some(position) = self.private_element_find(object, key)? else {
            let name = self.private_name_text(key);
            return Err(self.throw_error(
                ErrorKind::TypeError,
                &format!(
                    "cannot read the private member {name} of an object whose class did not add it"
                ),
            ));
        };
        match self.private_element(object, position)? {
            PrivateValue::Field(value) => Ok(value),
            PrivateValue::Method(function) => Ok(Value::Object(function)),
            PrivateValue::Accessor {
                get: Some(getter), ..
            } => self.call_value(Value::Object(getter), Value::Object(object), &[]),
            PrivateValue::Accessor { get: None, .. } => {
                let name = self.private_name_text(key);
                Err(self.throw_error(
                    ErrorKind::TypeError,
                    &format!("the private accessor {name} has no getter"),
                ))
            }
        }
    }

    /// PrivateSet(`object`, `key`, `value`) (`#sec-privateset`): a field takes `value` (step 4),
    /// the setter of an accessor is called with `object` as `this` and `value`; a `TypeError`
    /// when `object` has no element of `key` (step 2), for a method (step 5) and for an accessor
    /// with no setter (step 6.b).
    pub(crate) fn private_set(
        &mut self,
        object: ObjectRef,
        key: PrivateNameRef,
        value: Value,
    ) -> Completion<()> {
        let Some(position) = self.private_element_find(object, key)? else {
            let name = self.private_name_text(key);
            return Err(self.throw_error(
                ErrorKind::TypeError,
                &format!(
                    "cannot write the private member {name} of an object whose class did not add it"
                ),
            ));
        };
        let message = match self.private_element(object, position)? {
            PrivateValue::Field(_) => {
                let entry = self
                    .engine_object_mut(object)?
                    .private_elements
                    .as_mut()
                    .and_then(|elements| elements.entries.get_mut(position));
                return match entry {
                    Some(entry) => {
                        entry.value = PrivateValue::Field(value);
                        Ok(())
                    }
                    None => Err(self.fault()),
                };
            }
            PrivateValue::Accessor {
                set: Some(setter), ..
            } => {
                self.call_value(Value::Object(setter), Value::Object(object), &[value])?;
                return Ok(());
            }
            PrivateValue::Method(_) => "the private method {name} is not writable",
            PrivateValue::Accessor { set: None, .. } => "the private accessor {name} has no setter",
        };
        let name = self.private_name_text(key);
        Err(self.throw_error(ErrorKind::TypeError, &message.replace("{name}", &name)))
    }

    /// PrivateMethodOrAccessorAdd(`object`, the method `function` keyed by `key`)
    /// (`#sec-privatemethodoraccessoradd`), through the one add of every private element
    /// ([`Agent::add_private_element`]). A getter or a setter joins the accessor of `key` that
    /// lacks that half: the class's code adds the two halves of an accessor pair one after the
    /// other, which together are the one element ClassDefinitionEvaluation combines (step
    /// 27.e); an element of `key` that has that half already is there, and is a `TypeError`.
    /// `room` is the number of elements the adds of this class level make.
    pub(crate) fn private_method_or_accessor_add(
        &mut self,
        object: ObjectRef,
        key: PrivateNameRef,
        function: ObjectRef,
        room: usize,
    ) -> Completion<()> {
        let value = match self.method_kind(function)? {
            MethodKind::Method => PrivateValue::Method(function),
            MethodKind::Getter => PrivateValue::Accessor {
                get: Some(function),
                set: None,
            },
            MethodKind::Setter => PrivateValue::Accessor {
                get: None,
                set: Some(function),
            },
        };
        self.add_private_element(object, PrivateElement { key, value }, room)
    }

    /// PrivateFieldAdd(`object`, `key`, `value`) (`#sec-privatefieldadd`), through the one add
    /// of every private element ([`Agent::add_private_element`]): a `TypeError` when `object` has
    /// an element of `key` already (step 3), otherwise the field is appended (step 4); the add
    /// does not read [[Extensible]]. `room` is the number of elements the adds of this class
    /// level make, its methods, accessors and fields together.
    pub(crate) fn private_field_add(
        &mut self,
        object: ObjectRef,
        key: PrivateNameRef,
        value: Value,
        room: usize,
    ) -> Completion<()> {
        let value = PrivateValue::Field(value);
        self.add_private_element(object, PrivateElement { key, value }, room)
    }

    /// The kind of the template of the ECMAScript function `function`.
    fn method_kind(&mut self, function: ObjectRef) -> Completion<MethodKind> {
        let kind = match &self.engine_object(function)?.kind {
            ObjectKind::EcmascriptFunction(f) => f.compiled.as_ref().and_then(|compiled| {
                let template = usize::try_from(f.template).ok()?;
                compiled.code.functions.get(template).map(|t| t.kind)
            }),
            _ => None,
        };
        match kind {
            Some(FunctionKind::Getter) => Ok(MethodKind::Getter),
            Some(FunctionKind::Setter) => Ok(MethodKind::Setter),
            Some(_) => Ok(MethodKind::Method),
            None => Err(self.fault()),
        }
    }

    /// The one add of every private element ("Private names", item 2), a field, a method or an
    /// accessor: an element of its key already on `object` is a `TypeError`, unless the new one
    /// is the missing half of its accessor; otherwise the element is appended, the box and the
    /// growth of the list charged first. The add does not read [[Extensible]].
    pub(crate) fn add_private_element(
        &mut self,
        object: ObjectRef,
        element: PrivateElement,
        room: usize,
    ) -> Completion<()> {
        if let Some(position) = self.private_element_find(object, element.key)? {
            let existing = self.private_element(object, position)?;
            let joined = match (existing, element.value) {
                (
                    PrivateValue::Accessor { get: None, set },
                    PrivateValue::Accessor {
                        get: Some(get),
                        set: None,
                    },
                )
                | (
                    PrivateValue::Accessor {
                        get: Some(get),
                        set: None,
                    },
                    PrivateValue::Accessor { get: None, set },
                ) if set.is_some() => Some(PrivateValue::Accessor {
                    get: Some(get),
                    set,
                }),
                _ => None,
            };
            let Some(joined) = joined else {
                let name = self.private_name_text(element.key);
                return Err(self.throw_error(
                    ErrorKind::TypeError,
                    &format!("the object already has the private element {name}"),
                ));
            };
            let entry = self
                .engine_object_mut(object)?
                .private_elements
                .as_mut()
                .and_then(|elements| elements.entries.get_mut(position));
            return match entry {
                Some(entry) => {
                    entry.value = joined;
                    Ok(())
                }
                None => Err(self.fault()),
            };
        }
        self.private_room(object, room)?;
        let pushed = self
            .engine_object_mut(object)?
            .private_elements
            .as_mut()
            .map(|elements| elements.entries.push(element));
        match pushed {
            Some(()) => Ok(()),
            None => Err(self.fault()),
        }
    }

    /// Room for one more element of `object`: when its list is full (or there is none), the box
    /// at the first add and room for exactly `room` more elements (at least one), the elements
    /// of the class level whose add runs, so each level of a construction reserves once and the
    /// list holds no spare entry; charged before it is made and recorded with the object, over
    /// the budget the run ends in `Terminated(MemoryBudget)`.
    fn private_room(&mut self, object: ObjectRef, room: usize) -> Completion<()> {
        let (boxed, length, capacity) = match &self.engine_object(object)?.private_elements {
            Some(elements) => (true, elements.entries.len(), elements.entries.capacity()),
            None => (false, 0, 0),
        };
        if boxed && length < capacity {
            return Ok(());
        }
        let more = room.max(1);
        let box_bytes = if boxed {
            0
        } else {
            size_of::<PrivateElements>()
        };
        let bytes = more
            .saturating_mul(size_of::<PrivateElement>())
            .saturating_add(box_bytes);
        let bytes = u64::try_from(bytes).unwrap_or(u64::MAX);
        self.charge_bytes(bytes)?;
        let record = self.engine_object_mut(object)?;
        let elements = record
            .private_elements
            .get_or_insert_with(|| Box::new(PrivateElements::default()));
        #[expect(
            clippy::disallowed_methods,
            reason = "the list's growth, charged by `charge_bytes` as `bytes` before it is reserved and recorded with the object by `record_grown`"
        )]
        let reserved = elements.entries.try_reserve_exact(more);
        if reserved.is_err() {
            self.heap.accounting.credit(bytes);
            return Err(self.terminate(Termination::MemoryBudget));
        }
        self.record_grown(object, bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::{PrivateElement, PrivateValue};
    use crate::heap::PrivateNameRef;
    use crate::host::hooks::test_host;
    use crate::object::ObjectKind;
    use crate::object::ordinary::Ordinary;

    /// A list that grows past its room (four levels of ten) keeps every element, in the order
    /// they were added, and each find answers the position of its element: no growth drops or
    /// reorders an entry; a key never added is not found.
    #[test]
    fn every_element_added_is_found_at_its_position() {
        let mut agent = test_host::agent_with(test_host::limits());
        let object = agent
            .allocate_object(None, ObjectKind::Ordinary(Ordinary::for_test()))
            .unwrap();
        let function = agent
            .allocate_object(None, ObjectKind::Ordinary(Ordinary::for_test()))
            .unwrap();
        let description = agent.heap.new_string_from_str("#m").unwrap();
        let keys: alloc::vec::Vec<PrivateNameRef> = (0..40)
            .map(|_| agent.heap.new_private_name(description).unwrap())
            .collect();
        for &key in &keys {
            let element = PrivateElement {
                key,
                value: PrivateValue::Method(function),
            };
            agent.add_private_element(object, element, 10).unwrap();
        }
        for (i, &key) in keys.iter().enumerate() {
            assert_eq!(agent.private_element_find(object, key).unwrap(), Some(i));
        }
        let record = agent.heap.objects.get(object.handle).unwrap();
        assert_eq!(record.private_elements.as_ref().unwrap().len(), keys.len());
        let other = agent.heap.new_private_name(description).unwrap();
        assert_eq!(agent.private_element_find(object, other).unwrap(), None);
    }

    /// The room of a list is reserved once per class level and charged exactly: the first add
    /// of a level of three makes the box and room for three entries, charged as that many bytes;
    /// the next two adds charge nothing; the first add of a level of two, on the full list,
    /// reserves and charges two entries more; a getter and a setter of one name are one entry, the
    /// setter's add joins it and charges nothing. The list never has a spare entry.
    #[test]
    fn the_room_is_reserved_once_per_level_and_charged_exactly() {
        use super::PrivateElements;
        let mut agent = test_host::agent_with(test_host::limits());
        let object = agent
            .allocate_object(None, ObjectKind::Ordinary(Ordinary::for_test()))
            .unwrap();
        let function = agent
            .allocate_object(None, ObjectKind::Ordinary(Ordinary::for_test()))
            .unwrap();
        let description = agent.heap.new_string_from_str("#m").unwrap();
        let keys: alloc::vec::Vec<PrivateNameRef> = (0..6)
            .map(|_| agent.heap.new_private_name(description).unwrap())
            .collect();
        let entry = size_of::<PrivateElement>() as u64;
        let list = |agent: &crate::host::agent::Agent<test_host::NoHooks>| {
            let record = agent.heap.objects.get(object.handle).unwrap();
            let elements = record.private_elements.as_ref().unwrap();
            (elements.len(), elements.capacity())
        };
        let add = |agent: &mut crate::host::agent::Agent<test_host::NoHooks>,
                   key: PrivateNameRef,
                   value: PrivateValue,
                   room: usize| {
            let before = agent.heap.bytes();
            agent
                .add_private_element(object, PrivateElement { key, value }, room)
                .unwrap();
            agent.heap.bytes() - before
        };
        let method = PrivateValue::Method(function);
        let first = add(&mut agent, keys[0], method, 3);
        assert_eq!(first, size_of::<PrivateElements>() as u64 + 3 * entry);
        assert_eq!(list(&agent), (1, 3));
        assert_eq!(add(&mut agent, keys[1], method, 3), 0);
        assert_eq!(add(&mut agent, keys[2], method, 3), 0);
        assert_eq!(list(&agent), (3, 3));
        assert_eq!(add(&mut agent, keys[3], method, 2), 2 * entry);
        assert_eq!(add(&mut agent, keys[4], method, 2), 0);
        assert_eq!(list(&agent), (5, 5));
        let getter = PrivateValue::Accessor {
            get: Some(function),
            set: None,
        };
        let setter = PrivateValue::Accessor {
            get: None,
            set: Some(function),
        };
        assert_eq!(add(&mut agent, keys[5], getter, 1), entry);
        assert_eq!(add(&mut agent, keys[5], setter, 1), 0);
        assert_eq!(list(&agent), (6, 6));
    }

    /// A level of methods and fields reserves its room once (`story:private-fields-and-private-in`):
    /// the adds of one class level carry one room, its methods, accessors and fields counted
    /// together, so the first add of a level of two methods and three fields makes the box and
    /// room for five entries, charged as that many bytes, and the other four adds charge
    /// nothing, whichever kind comes first; a level of two fields added to the full list (a
    /// second class's stamp) reserves and charges two entries more. A field read and written by
    /// PrivateGet and PrivateSet answers its value; a second add of a field's key fails (the
    /// `TypeError`) and appends nothing. The entry of a field is the size of every
    /// entry: the box is 24 bytes and an entry 48 (44 before fields: a `Value` aligns the entry to
    /// 8 bytes), the figures `tests/private_fields.rs` reads per instance.
    #[test]
    fn a_level_of_methods_and_fields_reserves_its_room_once() {
        use super::PrivateElements;
        use crate::value::Value;
        assert_eq!(size_of::<PrivateElements>(), 24);
        assert_eq!(size_of::<PrivateElement>(), 48);
        let mut agent = test_host::agent_with(test_host::limits());
        let object = agent
            .allocate_object(None, ObjectKind::Ordinary(Ordinary::for_test()))
            .unwrap();
        let function = agent
            .allocate_object(None, ObjectKind::Ordinary(Ordinary::for_test()))
            .unwrap();
        let description = agent.heap.new_string_from_str("#f").unwrap();
        let keys: alloc::vec::Vec<PrivateNameRef> = (0..7)
            .map(|_| agent.heap.new_private_name(description).unwrap())
            .collect();
        let entry = size_of::<PrivateElement>() as u64;
        let capacity = |agent: &crate::host::agent::Agent<test_host::NoHooks>| {
            let record = agent.heap.objects.get(object.handle).unwrap();
            let elements = record.private_elements.as_ref().unwrap();
            (elements.len(), elements.capacity())
        };
        let add = |agent: &mut crate::host::agent::Agent<test_host::NoHooks>,
                   key: PrivateNameRef,
                   value: PrivateValue,
                   room: usize| {
            let before = agent.heap.bytes();
            let added = agent.add_private_element(object, PrivateElement { key, value }, room);
            (added.is_ok(), agent.heap.bytes() - before)
        };
        let field = |n: f64| PrivateValue::Field(Value::Number(n));
        let method = PrivateValue::Method(function);
        let first = add(&mut agent, keys[0], field(0.0), 5);
        assert_eq!(
            first,
            (true, size_of::<PrivateElements>() as u64 + 5 * entry)
        );
        assert_eq!(add(&mut agent, keys[1], method, 5), (true, 0));
        assert_eq!(add(&mut agent, keys[2], field(2.0), 5), (true, 0));
        assert_eq!(add(&mut agent, keys[3], method, 5), (true, 0));
        assert_eq!(add(&mut agent, keys[4], field(4.0), 5), (true, 0));
        assert_eq!(capacity(&agent), (5, 5));
        assert_eq!(add(&mut agent, keys[5], field(5.0), 2), (true, 2 * entry));
        assert_eq!(add(&mut agent, keys[6], field(6.0), 2), (true, 0));
        assert_eq!(capacity(&agent), (7, 7));
        assert!(!add(&mut agent, keys[2], field(9.0), 1).0);
        assert_eq!(capacity(&agent), (7, 7));
        assert_eq!(
            agent.private_get(object, keys[2]).unwrap(),
            Value::Number(2.0)
        );
        agent
            .private_set(object, keys[2], Value::Number(3.0))
            .unwrap();
        assert_eq!(
            agent.private_get(object, keys[2]).unwrap(),
            Value::Number(3.0)
        );
        assert_eq!(
            agent.private_get(object, keys[6]).unwrap(),
            Value::Number(6.0)
        );
    }

    /// The collector reaches an object's private elements from the object alone
    /// (`ObjectRecord::trace`, [`super::PrivateElements::trace`]): with nothing else holding
    /// them, a mark that starts at the object marks each element's Private Name, a field's value,
    /// the method's function and both halves of an accessor, and not an object no element names.
    /// Script cannot see this for a method while the class's private record holds the same names
    /// and functions, so no test of `tests/private_methods.rs` goes red when the list is not
    /// traced; this one does. A field's value no record holds
    /// (`private_fields::a_private_field_survives_a_collection_between_add_and_read`).
    #[test]
    fn a_mark_from_the_object_reaches_every_element() {
        use crate::heap::collect::{Item, Mark, mark_phase};
        let mut agent = test_host::agent_with(test_host::limits());
        let mut new_object = || {
            agent
                .allocate_object(None, ObjectKind::Ordinary(Ordinary::for_test()))
                .unwrap()
        };
        let (object, method, getter, setter, other, held) = (
            new_object(),
            new_object(),
            new_object(),
            new_object(),
            new_object(),
            new_object(),
        );
        let description = agent.heap.new_string_from_str("#m").unwrap();
        let m = agent.heap.new_private_name(description).unwrap();
        let a = agent.heap.new_private_name(description).unwrap();
        let f = agent.heap.new_private_name(description).unwrap();
        let unused = agent.heap.new_private_name(description).unwrap();
        let elements = [
            PrivateElement {
                key: f,
                value: PrivateValue::Field(crate::value::Value::Object(held)),
            },
            PrivateElement {
                key: m,
                value: PrivateValue::Method(method),
            },
            PrivateElement {
                key: a,
                value: PrivateValue::Accessor {
                    get: Some(getter),
                    set: Some(setter),
                },
            },
        ];
        for element in elements {
            agent.add_private_element(object, element, 3).unwrap();
        }
        let mut mark = Mark::take(&mut agent.heap);
        mark.push(Item::Object(object.handle));
        mark_phase(&agent, &mut mark).unwrap();
        for item in [
            Item::PrivateName(f.handle),
            Item::Object(held.handle),
            Item::PrivateName(m.handle),
            Item::PrivateName(a.handle),
            Item::Object(method.handle),
            Item::Object(getter.handle),
            Item::Object(setter.handle),
        ] {
            assert!(mark.is_marked(item), "{item:?} is not marked");
        }
        assert!(
            !mark.is_marked(Item::Object(other.handle)),
            "an unrelated object"
        );
        assert!(
            !mark.is_marked(Item::PrivateName(unused.handle)),
            "an unused name"
        );
    }
}
