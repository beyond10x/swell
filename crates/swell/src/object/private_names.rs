//! [[PrivateElements]] of an object (`#sec-privateelement-specification-type`;
//! `docs/design/engine.md`, "Private names", item 2), written by
//! `story:private-names-and-private-methods`: the one add function of every private element,
//! PrivateElementFind (`#sec-privateelementfind`), PrivateGet (`#sec-privateget`) and PrivateSet
//! (`#sec-privateset`).
//!
//! The elements of an object are a list in the box `ObjectRecord::private_elements`, made at the
//! first add, so an object without one pays the box's 8 bytes only. Each entry is a Private Name
//! (`heap/mod.rs`, `PrivateNameRef`) with a method (its function) or an accessor (an optional
//! getter and an optional setter); `story:private-fields-and-private-in` adds fields through the
//! same add. The box and every growth of its list are charged to the memory budget before they
//! are made and recorded with the object (`Agent::record_grown`), so the sweep that frees it
//! credits them; the list never shrinks. Elements are no properties: no internal method, no
//! trap of a `Proxy` and no listing of a host reads them.
//!
//! PrivateElementFind compares the entries in order and spends a step per comparison past
//! [`SMALL`] (`object/property.rs`), so a list of n elements costs at most n − `SMALL` steps per
//! lookup, counted before the step is charged.
//!
//! The collector traces each entry's Private Name and its functions at the object record's site
//! ([`PrivateElements::trace`], called by `ObjectRecord::trace`).

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
/// slots): a method's function, or an accessor's getter and setter, either of which may be
/// missing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum PrivateValue {
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
    /// Name and its functions.
    pub(crate) fn trace(&self, mark: &mut Mark) {
        for entry in &self.entries {
            mark.push(Item::PrivateName(entry.key.handle));
            match entry.value {
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
}

/// The fewest entries a list grows to.
const MIN_ENTRIES: usize = 4;

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

    /// PrivateGet(`object`, `key`) (`#sec-privateget`): a method's function, or its getter
    /// called with `object` as `this`; a `TypeError` when `object` has no element of `key` (step
    /// 2) or the accessor has no getter (step 5.b).
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

    /// PrivateSet(`object`, `key`, `value`) (`#sec-privateset`): the setter of an accessor
    /// called with `object` as `this` and `value`; a `TypeError` when `object` has no element of
    /// `key` (step 2), for a method (step 5) and for an accessor with no setter (step 6.b).
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
    pub(crate) fn private_method_or_accessor_add(
        &mut self,
        object: ObjectRef,
        key: PrivateNameRef,
        function: ObjectRef,
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
        self.add_private_element(object, PrivateElement { key, value })
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

    /// The one add of every private element ("Private names", item 2): an element of its key
    /// already on `object` is a `TypeError`, unless the new one is the missing half of its
    /// accessor; otherwise the element is appended, the box and the growth of the list charged
    /// first. The add does not read [[Extensible]].
    pub(crate) fn add_private_element(
        &mut self,
        object: ObjectRef,
        element: PrivateElement,
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
        self.private_room(object)?;
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

    /// Room for one more element of `object`: the box at the first add, and the list doubled
    /// (at least [`MIN_ENTRIES`]) when it is full, each charged before it is made and recorded
    /// with the object; over the budget the run ends in `Terminated(MemoryBudget)`.
    fn private_room(&mut self, object: ObjectRef) -> Completion<()> {
        let (boxed, length, capacity) = match &self.engine_object(object)?.private_elements {
            Some(elements) => (true, elements.entries.len(), elements.entries.capacity()),
            None => (false, 0, 0),
        };
        if boxed && length < capacity {
            return Ok(());
        }
        let wanted = capacity.saturating_mul(2).max(MIN_ENTRIES);
        let box_bytes = if boxed {
            0
        } else {
            size_of::<PrivateElements>()
        };
        let bytes = wanted
            .saturating_sub(capacity)
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
        let reserved = elements
            .entries
            .try_reserve_exact(wanted.saturating_sub(length));
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

    /// A list that grows past its room keeps every element, in the order they were added, and
    /// each find answers the position of its element: no growth drops or reorders an entry; a key
    /// never added is not found.
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
            agent.add_private_element(object, element).unwrap();
        }
        for (i, &key) in keys.iter().enumerate() {
            assert_eq!(agent.private_element_find(object, key).unwrap(), Some(i));
        }
        let record = agent.heap.objects.get(object.handle).unwrap();
        assert_eq!(record.private_elements.as_ref().unwrap().len(), keys.len());
        let other = agent.heap.new_private_name(description).unwrap();
        assert_eq!(agent.private_element_find(object, other).unwrap(), None);
    }
}
