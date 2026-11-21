//! Properties (ECMA-262 6.1.7.1, `#sec-property-attributes`): the property map of an object, in
//! the order `OrdinaryOwnPropertyKeys` requires (`docs/design/engine.md`, decision 4, "Objects"),
//! the property keys of the engine, and the Property Descriptor record (6.2.6,
//! `#sec-property-descriptor-specification-type`). Written by `story:object-model-and-realms`.
//!
//! A key is an array index (`#sec-object-type`: "An array index is an integer index n such that
//! n is in the inclusive interval from +0𝔽 to 𝔽(2^32 - 2)"), an atom (`heap/string.rs`) or a
//! Symbol, so two keys are the same key exactly when they are equal. The map is a vector in
//! creation order; [`Properties::keys`] sorts the indices first, as 10.1.11.1 asks. Its growth
//! is charged to the memory budget by the object that owns it (`object/mod.rs`).

use crate::heap::collect::Mark;
use crate::value::{ObjectRef, StringRef, SymbolRef, Value};

/// A property key inside the engine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Key {
    /// An array index, 0 to 2^32 - 2.
    Index(u32),
    /// Any other String key, as its atom.
    String(StringRef),
    Symbol(SymbolRef),
}

impl Key {
    /// The value of the key: an index is read back as its String by the caller.
    pub(crate) fn trace(&self, mark: &mut Mark) {
        match *self {
            Key::Index(_) => {}
            Key::String(string) => mark.value(Value::String(string)),
            Key::Symbol(symbol) => mark.value(Value::Symbol(symbol)),
        }
    }
}

/// The array index `units` names, if it is one: the canonical decimal form of an integer from 0
/// to 2^32 - 2 (`#sec-canonicalnumericindexstring` restricted to array indices).
pub(crate) fn array_index(units: &[u16]) -> Option<u32> {
    let (&first, rest) = units.split_first()?;
    if first == u16::from(b'0') {
        return rest.is_empty().then_some(0);
    }
    if units.len() > 10 {
        return None;
    }
    let mut value: u64 = 0;
    for &unit in units {
        let digit = unit.checked_sub(u16::from(b'0')).filter(|d| *d <= 9)?;
        value = value * 10 + u64::from(digit);
    }
    u32::try_from(value).ok().filter(|&index| index != u32::MAX)
}

/// The value part of a property (6.1.7.1, Table 3 and Table 4).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Slot {
    Data {
        value: Value,
        writable: bool,
    },
    Accessor {
        get: Option<ObjectRef>,
        set: Option<ObjectRef>,
    },
}

/// A property: its value part and the two attributes every property has.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Property {
    pub(crate) slot: Slot,
    pub(crate) enumerable: bool,
    pub(crate) configurable: bool,
}

impl Property {
    /// A data property.
    pub(crate) fn data(value: Value, writable: bool, enumerable: bool, configurable: bool) -> Self {
        Property {
            slot: Slot::Data { value, writable },
            enumerable,
            configurable,
        }
    }

    /// What the property holds.
    pub(crate) fn trace(&self, mark: &mut Mark) {
        match self.slot {
            Slot::Data { value, .. } => mark.value(value),
            Slot::Accessor { get, set } => {
                for function in [get, set].into_iter().flatten() {
                    mark.object(function);
                }
            }
        }
    }
}

/// A Property Descriptor (6.2.6): each field present or absent. An accessor field holds
/// `Some(None)` for undefined.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Descriptor {
    pub(crate) value: Option<Value>,
    pub(crate) writable: Option<bool>,
    pub(crate) get: Option<Option<ObjectRef>>,
    pub(crate) set: Option<Option<ObjectRef>>,
    pub(crate) enumerable: Option<bool>,
    pub(crate) configurable: Option<bool>,
}

impl Descriptor {
    /// A data descriptor with every field.
    pub(crate) fn data(value: Value, writable: bool, enumerable: bool, configurable: bool) -> Self {
        Descriptor {
            value: Some(value),
            writable: Some(writable),
            enumerable: Some(enumerable),
            configurable: Some(configurable),
            ..Descriptor::default()
        }
    }

    /// IsAccessorDescriptor (`#sec-isaccessordescriptor`).
    pub(crate) fn is_accessor(&self) -> bool {
        self.get.is_some() || self.set.is_some()
    }

    /// IsDataDescriptor (`#sec-isdatadescriptor`).
    pub(crate) fn is_data(&self) -> bool {
        self.value.is_some() || self.writable.is_some()
    }

    /// IsGenericDescriptor (`#sec-isgenericdescriptor`).
    pub(crate) fn is_generic(&self) -> bool {
        !self.is_accessor() && !self.is_data()
    }
}

/// The own properties of an object, in creation order.
#[derive(Default)]
pub(crate) struct Properties {
    entries: Vec<(Key, Property)>,
}

impl Properties {
    /// The property `key`, if the object has it.
    pub(crate) fn get(&self, key: Key) -> Option<&Property> {
        self.entries
            .iter()
            .find(|(own, _)| *own == key)
            .map(|(_, property)| property)
    }

    /// The property `key` to change, if the object has it.
    pub(crate) fn get_mut(&mut self, key: Key) -> Option<&mut Property> {
        self.entries
            .iter_mut()
            .find(|(own, _)| *own == key)
            .map(|(_, property)| property)
    }

    /// Whether one more property needs more room than the map has.
    pub(crate) fn is_full(&self) -> bool {
        self.entries.len() == self.entries.capacity()
    }

    /// The bytes one property takes in the map.
    pub(crate) fn entry_bytes() -> u64 {
        u64::try_from(size_of::<(Key, Property)>()).unwrap_or(u64::MAX)
    }

    /// The room for properties the map has.
    pub(crate) fn capacity(&self) -> usize {
        self.entries.capacity()
    }

    /// Room for `additional` more properties, or `Err` when the allocator refuses.
    pub(crate) fn reserve(&mut self, additional: usize) -> Result<(), ()> {
        self.entries.try_reserve_exact(additional).map_err(|_| ())
    }

    /// Add a property the map does not have. The caller made room first.
    pub(crate) fn push(&mut self, key: Key, property: Property) {
        self.entries.push((key, property));
    }

    /// Remove the property `key`; the others keep their order.
    pub(crate) fn remove(&mut self, key: Key) {
        self.entries.retain(|(own, _)| *own != key);
    }

    /// The number of properties.
    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }

    /// The keys in the order of OrdinaryOwnPropertyKeys (`#sec-ordinaryownpropertykeys`): the
    /// array indices ascending, then the Strings, then the Symbols, each in creation order.
    pub(crate) fn keys(&self) -> Vec<Key> {
        let mut indices: Vec<u32> = self
            .entries
            .iter()
            .filter_map(|(key, _)| match key {
                Key::Index(index) => Some(*index),
                _ => None,
            })
            .collect();
        indices.sort_unstable();
        let mut keys: Vec<Key> = indices.into_iter().map(Key::Index).collect();
        keys.extend(
            self.entries
                .iter()
                .filter(|(key, _)| matches!(key, Key::String(_)))
                .map(|(key, _)| *key),
        );
        keys.extend(
            self.entries
                .iter()
                .filter(|(key, _)| matches!(key, Key::Symbol(_)))
                .map(|(key, _)| *key),
        );
        keys
    }

    /// The array indices of the map, unsorted.
    pub(crate) fn indices(&self) -> Vec<u32> {
        self.entries
            .iter()
            .filter_map(|(key, _)| match key {
                Key::Index(index) => Some(*index),
                _ => None,
            })
            .collect()
    }

    /// Every key and value the map holds.
    pub(crate) fn trace(&self, mark: &mut Mark) {
        for (key, property) in &self.entries {
            key.trace(mark);
            property.trace(mark);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::array_index;

    fn units(text: &str) -> Vec<u16> {
        text.encode_utf16().collect()
    }

    /// `#sec-object-type`: an array index is a canonical integer from 0 to 2^32 - 2.
    #[test]
    fn array_indices_are_canonical_and_below_2_32_minus_1() {
        assert_eq!(array_index(&units("0")), Some(0));
        assert_eq!(array_index(&units("17")), Some(17));
        assert_eq!(array_index(&units("4294967294")), Some(4_294_967_294));
        for not_one in [
            "",
            "00",
            "01",
            "-1",
            "1.0",
            "4294967295",
            "99999999999",
            "1e3",
            " 1",
        ] {
            assert_eq!(array_index(&units(not_one)), None, "{not_one:?}");
        }
    }
}
