//! Array exotic objects (ECMA-262 10.4.2, `#sec-array-exotic-objects`): ArrayCreate, the
//! [[DefineOwnProperty]] of 10.4.2.1 and ArraySetLength (10.4.2.4). Written by
//! `story:object-model-and-realms`.
//!
//! `"length"` is an ordinary data property of the record, kept by the two methods here as the
//! clause says; the elements are the array index properties of the common map. An Array holds
//! no slot of its own, so its trace reports nothing.

use super::ObjectKind;
use super::property::{Descriptor, Key, Property, Slot};
use crate::heap::collect::Mark;
use crate::host::ErrorKind;
use crate::host::Host;
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::realm::intrinsics::Intrinsic;
use crate::value::{ObjectRef, Value};

/// The slots of an Array: none beyond the common record.
pub(crate) struct Array {}

impl Array {
    /// Kind site of the collector: an Array holds nothing beyond its properties.
    pub(crate) fn trace(&self, mark: &mut Mark) {
        mark.count("object/array.rs Array::trace");
    }

    /// An Array, for the tests of the collector.
    #[cfg(test)]
    pub(crate) fn for_test() -> Array {
        Array {}
    }
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
        let array = self.allocate_object(Some(prototype), ObjectKind::Array(Array {}))?;
        let key = self.name_key("length")?;
        self.add_property(
            array,
            key,
            Property::data(Value::Number(length), true, false, false),
        )?;
        Ok(array)
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
        let index = f64::from(index);
        if index >= length && !writable {
            return Ok(false);
        }
        if !self.ordinary_define_own_property(array, key, descriptor)? {
            return Ok(false);
        }
        if index >= length {
            let new_length = Descriptor {
                value: Some(Value::Number(index + 1.0)),
                ..Descriptor::default()
            };
            self.ordinary_define_own_property(array, length_key, new_length)?;
        }
        Ok(true)
    }

    /// ArraySetLength (`#sec-arraysetlength`). The deletion loop charges a step per index.
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
        let mut indices: Vec<u32> = self
            .engine_object(array)?
            .properties
            .indices()
            .into_iter()
            .filter(|&index| index >= new_length)
            .collect();
        indices.sort_unstable_by(|a, b| b.cmp(a));
        for index in indices {
            self.charge_steps(1)?;
            if !self.internal_delete(array, Key::Index(index))? {
                new_length_descriptor.value = Some(Value::Number(f64::from(index) + 1.0));
                if !new_writable {
                    new_length_descriptor.writable = Some(false);
                }
                self.ordinary_define_own_property(array, length_key, new_length_descriptor)?;
                return Ok(false);
            }
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
}

#[cfg(test)]
mod tests {
    use crate::host::hooks::test_host::{agent_with_realm, enter};
    use crate::host::outcome::Abrupt;
    use crate::object::property::{Descriptor, Key};
    use crate::value::Value;

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
