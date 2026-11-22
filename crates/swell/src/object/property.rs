//! Properties (ECMA-262 6.1.7.1, `#sec-property-attributes`): the property map of an object, in
//! the order `OrdinaryOwnPropertyKeys` requires (`docs/design/engine.md`, decision 4, "Objects"),
//! the property keys of the engine, and the Property Descriptor record (6.2.6,
//! `#sec-property-descriptor-specification-type`). Written by `story:object-model-and-realms`;
//! `story:property-index-and-dense-elements` added the index map.
//!
//! A key is an array index (`#sec-object-type`: "An array index is an integer index n such that
//! n is in the inclusive interval from +0𝔽 to 𝔽(2^32 - 2)"), an atom (`heap/string.rs`) or a
//! Symbol, so two keys are the same key exactly when they are equal.
//!
//! The map is a [`KeyedList`]: a vector in creation order and, once its room passes
//! [`SMALL`] entries, an index map from key to position (`heap/map.rs`, keyed by the agent's
//! `hash_key` alone, HOST-145), kept in step by every addition and removal, so that a lookup
//! visits a bounded number of entries whatever the size of the object. Up to that size a lookup
//! scans the list: at most `SMALL` comparisons, a bound the step of the operation that asks
//! covers. A removal from an indexed list leaves its position empty, so the others keep their
//! order and their positions; when the list is full and at least half of it is empty, the next
//! addition compacts it in place, one step per entry it visits (an amortized constant per
//! removal), instead of growing it. The map's iteration order is never read: every order
//! answered comes from the list. [`Properties::keys`] answers the indices ascending: added
//! ascending (the usual case) they are read in place, otherwise they are sorted and each
//! comparison of the sort is answered for the caller to charge as a step.
//!
//! `SMALL` is 8, measured with the release runner on scripts that make 100,000 objects of 8
//! properties and 50,000 of 16 and read every property once (the report of
//! `story:property-index-and-dense-elements`): with `SMALL` 8 and 16 the times were the same
//! within the noise of the shared machine (708 and 734 ms, 536 and 576 ms, the smallest of five
//! runs, 675 and 473 ms on the base, which scans), so a scan of 16 entries costs what a hash of
//! the key does. 8 keeps a miss, which a walk up the prototype chain makes on each object it
//! passes, at 8 comparisons; most objects of a script, and the list of every Array, hold fewer.
//!
//! The growth of a list and of its map is charged to the memory budget before it is made, by
//! [`KeyedList::make_room`], and recorded by the owner, which credits it when it is freed
//! (`object/mod.rs`, `Grown`; `env/mod.rs`, `Added`).

use crate::heap::accounting::{Accounting, OverBudget};
use crate::heap::collect::Mark;
use crate::heap::map::KeyedMap;
use crate::value::{ObjectRef, StringRef, SymbolRef, Value};
use swell_num::siphash::SipBuildHasher;

/// A property key inside the engine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Key {
    /// An array index, 0 to 2^32 - 2.
    Index(u32),
    /// Any other String key, as its atom.
    String(StringRef),
    Symbol(SymbolRef),
}

/// The hash of a key is one 64-bit word through the keyed SipHash of the map (`heap/map.rs`,
/// HOST-145): its kind in the top two bits, then the index, or the slot and the low 30 bits of
/// the generation of the atom's or the Symbol's handle. Keys that are equal hash alike; two
/// that differ in the bits left out only collide, which the map's equality tells apart. One
/// word is one block of the hash, where the derived hash wrote up to four pieces.
impl std::hash::Hash for Key {
    fn hash<S: std::hash::Hasher>(&self, state: &mut S) {
        let (kind, high, low): (u64, u32, u32) = match *self {
            Key::Index(index) => (0, 0, index),
            Key::String(string) => (1, string.handle.generation, string.handle.index),
            Key::Symbol(symbol) => (2, symbol.handle.generation, symbol.handle.index),
        };
        let word = (kind << 62) | (u64::from(high & 0x3fff_ffff) << 32) | u64::from(low);
        state.write_u64(word);
    }
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

/// The room of a list above which it keeps an index map; a list without one has at most this
/// many entries, so a scan of it makes at most this many comparisons.
pub(crate) const SMALL: usize = 8;

/// The index map of a list whose room passed [`SMALL`]: the position of each key the list
/// holds, the positions removals left empty, and the bytes the map was charged.
struct Index {
    map: KeyedMap<Key, u32>,
    removed: usize,
    charged: u64,
}

/// What [`KeyedList::make_room`] did: the bytes it charged, which the owner records to credit
/// when it is freed, and the entries a compaction visited, which the caller charges as steps.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Room {
    pub(crate) bytes: u64,
    pub(crate) visited: u64,
}

/// Entries of a key and a value in creation order, found through an index map above the small
/// size: the own properties of an object ([`Properties`]) and the bindings eval code adds to a
/// declarative record (`env/mod.rs`).
pub(crate) struct KeyedList<V> {
    /// `None` is a position a removal left empty; only an indexed list has one.
    entries: Vec<Option<(Key, V)>>,
    index: Option<Box<Index>>,
    /// The entries lookups visited, in the counting build of the tests.
    #[cfg(test)]
    visited: std::cell::Cell<u64>,
}

impl<V> Default for KeyedList<V> {
    fn default() -> Self {
        KeyedList {
            entries: Vec::new(),
            index: None,
            #[cfg(test)]
            visited: std::cell::Cell::new(0),
        }
    }
}

/// `n` as a count of bytes or steps.
fn to_u64(n: usize) -> u64 {
    u64::try_from(n).unwrap_or(u64::MAX)
}

impl<V> KeyedList<V> {
    /// The entries lookups visited so far.
    #[cfg(test)]
    pub(crate) fn visited(&self) -> u64 {
        self.visited.get()
    }

    /// Count `n` entries a lookup visited, in the counting build.
    fn count(&self, n: usize) {
        #[cfg(test)]
        self.visited
            .set(self.visited.get().saturating_add(to_u64(n)));
        #[cfg(not(test))]
        let _ = n;
    }

    /// The position of `key`, if the list holds it: through the map, or a scan of at most
    /// [`SMALL`] entries.
    pub(crate) fn position(&self, key: Key) -> Option<usize> {
        if let Some(index) = &self.index {
            self.count(1);
            return index
                .map
                .get(&key)
                .and_then(|&position| usize::try_from(position).ok());
        }
        let mut visited = 0;
        let found = self.entries.iter().position(|entry| {
            visited += 1;
            matches!(entry, Some((own, _)) if *own == key)
        });
        self.count(visited);
        found
    }

    /// The value of `key`, if the list holds it.
    pub(crate) fn get(&self, key: Key) -> Option<&V> {
        let position = self.position(key)?;
        self.at(position)
    }

    /// The value of `key` to change, if the list holds it.
    pub(crate) fn get_mut(&mut self, key: Key) -> Option<&mut V> {
        let position = self.position(key)?;
        self.at_mut(position)
    }

    /// The value at `position`, which [`KeyedList::position`] answered.
    pub(crate) fn at(&self, position: usize) -> Option<&V> {
        self.entries.get(position)?.as_ref().map(|(_, value)| value)
    }

    /// The value at `position` to change.
    pub(crate) fn at_mut(&mut self, position: usize) -> Option<&mut V> {
        self.entries
            .get_mut(position)?
            .as_mut()
            .map(|(_, value)| value)
    }

    /// The number of entries.
    pub(crate) fn len(&self) -> usize {
        let removed = self.index.as_ref().map_or(0, |index| index.removed);
        self.entries.len().saturating_sub(removed)
    }

    /// The entries in creation order.
    pub(crate) fn iter(&self) -> impl Iterator<Item = &(Key, V)> {
        self.entries.iter().flatten()
    }

    /// The bytes one entry takes in the list.
    pub(crate) fn entry_bytes() -> u64 {
        to_u64(size_of::<Option<(Key, V)>>())
    }

    /// Make room for one more entry: nothing while the list has room; a compaction in place
    /// when it is full and at least half of it is empty (`room.visited` entries visited);
    /// otherwise the list's room doubles (at least 4) and, once that passes [`SMALL`], the map
    /// gets as much. The growth of both is charged to `accounting` before it is made and added
    /// to `room.bytes`; over the budget nothing more is charged or made and the answer is
    /// `OverBudget`. What `room.bytes` holds was charged, also on a refusal, and the owner
    /// records it.
    pub(crate) fn make_room(
        &mut self,
        accounting: &mut Accounting,
        hasher: &SipBuildHasher,
        room: &mut Room,
    ) -> Result<(), OverBudget> {
        let length = self.entries.len();
        let capacity = self.entries.capacity();
        if length < capacity {
            return Ok(());
        }
        if let Some(index) = &self.index
            && index.removed > 0
            && index.removed.saturating_mul(2) >= length
        {
            room.visited = room.visited.saturating_add(to_u64(length));
            return self.compact();
        }
        let wanted = capacity.saturating_mul(2).max(4);
        let list_bytes =
            to_u64(wanted.saturating_sub(capacity)).saturating_mul(Self::entry_bytes());
        let indexed = wanted > SMALL;
        let had = self.index.as_ref().map_or(0, |index| index.charged);
        let map_bytes = if indexed {
            index_bytes(wanted).saturating_sub(had)
        } else {
            0
        };
        accounting.charge(list_bytes.saturating_add(map_bytes))?;
        if indexed
            && let Err(over) = self.reserve_index(wanted, hasher, accounting, map_bytes, room)
        {
            accounting.credit(list_bytes);
            return Err(over);
        }
        if self
            .entries
            .try_reserve_exact(wanted.saturating_sub(length))
            .is_err()
        {
            accounting.credit(list_bytes);
            return Err(OverBudget);
        }
        room.bytes = room.bytes.saturating_add(list_bytes);
        Ok(())
    }

    /// Give the map room for `wanted` keys, building it from the list when the list has none.
    /// `charged` bytes were charged for it before; the difference to the table the map then
    /// has is charged or credited, and the map's net charge is added to `room.bytes`. A
    /// refusal leaves the list as it was: without a map if it had none, with its map if it had.
    fn reserve_index(
        &mut self,
        wanted: usize,
        hasher: &SipBuildHasher,
        accounting: &mut Accounting,
        charged: u64,
        room: &mut Room,
    ) -> Result<(), OverBudget> {
        let built = self.index.is_none();
        let index = self.index.get_or_insert_with(|| {
            Box::new(Index {
                map: KeyedMap::with_hasher(*hasher),
                removed: 0,
                charged: 0,
            })
        });
        if index
            .map
            .reserve(wanted.saturating_sub(index.map.len()))
            .is_err()
        {
            accounting.credit(charged);
            if built {
                self.index = None;
            }
            return Err(OverBudget);
        }
        if built {
            // A list without a map has no empty position and at most `SMALL` entries.
            for (position, entry) in self.entries.iter().enumerate() {
                if let Some((key, _)) = entry {
                    let position = u32::try_from(position).map_err(|_| OverBudget)?;
                    index.map.insert(*key, position)?;
                }
            }
        }
        let before = index.charged;
        let expected = before.saturating_add(charged);
        let table = to_u64(size_of::<Index>()).saturating_add(index.map.table_bytes());
        let answer = if table > expected {
            // The standard library's map took more than its layout says: charged now.
            match accounting.charge(table - expected) {
                Ok(()) => {
                    index.charged = table;
                    Ok(())
                }
                Err(over) => {
                    index.charged = expected;
                    Err(over)
                }
            }
        } else {
            accounting.credit(expected - table);
            index.charged = table;
            Ok(())
        };
        room.bytes = room
            .bytes
            .saturating_add(index.charged.saturating_sub(before));
        answer
    }

    /// Drop the positions removals left empty and number the entries again, without
    /// allocating: the list keeps its room and the map, cleared, keeps its own.
    fn compact(&mut self) -> Result<(), OverBudget> {
        self.entries.retain(Option::is_some);
        if let Some(index) = &mut self.index {
            index.map.clear();
            index.removed = 0;
            for (position, (key, _)) in self.entries.iter().flatten().enumerate() {
                let position = u32::try_from(position).map_err(|_| OverBudget)?;
                index.map.insert(*key, position)?;
            }
        }
        Ok(())
    }

    /// Add an entry for a key the list does not hold. The caller made room first
    /// ([`KeyedList::make_room`]), so neither the list nor the map allocates.
    pub(crate) fn push(&mut self, key: Key, value: V) -> Result<(), OverBudget> {
        let position = u32::try_from(self.entries.len()).map_err(|_| OverBudget)?;
        if let Some(index) = &mut self.index {
            index.map.insert(key, position)?;
        }
        self.entries.push(Some((key, value)));
        Ok(())
    }

    /// Remove the entry of `key` and answer its value; the others keep their order.
    pub(crate) fn remove(&mut self, key: Key) -> Option<V> {
        let position = self.position(key)?;
        match &mut self.index {
            Some(index) => {
                index.map.remove(&key);
                index.removed = index.removed.saturating_add(1);
                self.entries
                    .get_mut(position)?
                    .take()
                    .map(|(_, value)| value)
            }
            None if position < self.entries.len() => {
                self.entries.remove(position).map(|(_, value)| value)
            }
            None => None,
        }
    }
}

/// The bytes of the map of a list with room for `capacity` entries, its record included.
fn index_bytes(capacity: usize) -> u64 {
    to_u64(size_of::<Index>()).saturating_add(KeyedMap::<Key, u32>::table_bytes_for(capacity))
}

/// Sort `indices` ascending and answer the comparisons the sort made: none when they are
/// ascending already, as indices added in order are.
pub(crate) fn sort_indices(indices: &mut [u32]) -> u64 {
    if indices.is_sorted() {
        return 0;
    }
    let mut comparisons: u64 = 0;
    indices.sort_unstable_by(|a, b| {
        comparisons = comparisons.saturating_add(1);
        a.cmp(b)
    });
    comparisons
}

/// The own properties of an object, in creation order.
pub(crate) type Properties = KeyedList<Property>;

impl KeyedList<Property> {
    /// The keys in the order of OrdinaryOwnPropertyKeys (`#sec-ordinaryownpropertykeys`): the
    /// array indices ascending, then the Strings, then the Symbols, each in creation order;
    /// with the comparisons the sort of the indices made, which the caller charges as steps.
    pub(crate) fn keys(&self) -> (Vec<Key>, u64) {
        let mut indices = self.indices();
        let comparisons = sort_indices(&mut indices);
        let mut keys: Vec<Key> = indices.into_iter().map(Key::Index).collect();
        keys.extend(
            self.iter()
                .filter(|(key, _)| matches!(key, Key::String(_)))
                .map(|(key, _)| *key),
        );
        keys.extend(
            self.iter()
                .filter(|(key, _)| matches!(key, Key::Symbol(_)))
                .map(|(key, _)| *key),
        );
        (keys, comparisons)
    }

    /// The array indices of the list, in creation order.
    pub(crate) fn indices(&self) -> Vec<u32> {
        self.iter()
            .filter_map(|(key, _)| match key {
                Key::Index(index) => Some(*index),
                _ => None,
            })
            .collect()
    }

    /// Every key and value the list holds.
    pub(crate) fn trace(&self, mark: &mut Mark) {
        for (key, property) in self.iter() {
            key.trace(mark);
            property.trace(mark);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Key, Properties, SMALL, Slot, array_index};
    use crate::Realm;
    use crate::host::agent::Agent;
    use crate::host::hooks::test_host::{NoHooks, agent_with_realm_and, enter, limits};
    use crate::host::outcome::{Abrupt, Termination};
    use crate::object::ordinary::OrdinaryData;
    use crate::value::{ObjectRef, Value};

    fn units(text: &str) -> Vec<u16> {
        text.encode_utf16().collect()
    }

    /// An agent in an entry of a realm, with a budget of 256 MiB and no step limit to reach.
    fn agent() -> (Agent<NoHooks>, Realm) {
        let mut limits = limits();
        limits.memory_bytes = 1 << 28;
        limits.steps_per_entry = u64::MAX;
        let (mut agent, realm) = agent_with_realm_and(limits);
        enter(&mut agent, &realm);
        (agent, realm)
    }

    fn plain(agent: &mut Agent<NoHooks>) -> ObjectRef {
        agent
            .ordinary_object_create(None, OrdinaryData::Plain)
            .unwrap()
    }

    fn visited(agent: &mut Agent<NoHooks>, object: ObjectRef) -> u64 {
        agent.engine_object(object).unwrap().properties.visited()
    }

    fn number(i: u32) -> Value {
        Value::Number(f64::from(i))
    }

    /// Decision 4, "Objects": above the small size a lookup goes through the index map. 10,000
    /// properties are added and each is looked up once; every addition (which looks the key up
    /// first) and every lookup visits at most `SMALL` entries.
    #[test]
    fn a_lookup_above_the_small_size_visits_a_bounded_number_of_entries() {
        const N: u32 = 10_000;
        let (mut agent, _realm) = agent();
        let object = plain(&mut agent);
        for i in 0..N {
            let before = visited(&mut agent, object);
            assert_eq!(
                agent.create_data_property(object, Key::Index(i), number(i)),
                Ok(true)
            );
            let spent = visited(&mut agent, object) - before;
            assert!(spent <= SMALL as u64, "adding property {i} visited {spent}");
        }
        let before = visited(&mut agent, object);
        for i in 0..N {
            let property = agent.get_own_property(object, Key::Index(i)).unwrap();
            assert_eq!(
                property.map(|property| property.slot),
                Some(Slot::Data {
                    value: number(i),
                    writable: true
                })
            );
        }
        let spent = visited(&mut agent, object) - before;
        assert!(
            spent < u64::from(N) * SMALL as u64,
            "{N} lookups visited {spent}"
        );
    }

    /// OrdinaryDelete (`#sec-ordinarydelete`) and OrdinaryOwnPropertyKeys
    /// (`#sec-ordinaryownpropertykeys`): removals on both sides of the small size keep the
    /// order of the others, every key left is found and no removed one is; additions after
    /// them (which reuse the room of the removed ones) come last in their group.
    #[test]
    fn removal_keeps_the_order_and_the_map() {
        let (mut agent, _realm) = agent();
        let object = plain(&mut agent);
        let symbols: Vec<Key> = (0..3)
            .map(|_| Key::Symbol(agent.heap.new_symbol(None).unwrap()))
            .collect();
        let mut strings = Vec::new();
        let mut indices = Vec::new();
        for i in 0..30u32 {
            let string = agent.name_key(&format!("k{i}")).unwrap();
            strings.push(string);
            indices.push(29 - i);
            for key in [string, Key::Index(29 - i)] {
                assert_eq!(agent.create_data_property(object, key, number(i)), Ok(true));
            }
            if i % 10 == 0
                && let Some(&symbol) = symbols.get(usize::try_from(i).unwrap() / 10)
            {
                assert_eq!(
                    agent.create_data_property(object, symbol, number(i)),
                    Ok(true)
                );
            }
        }
        let expected = |indices: &[u32], strings: &[Key]| -> Vec<Key> {
            let mut sorted = indices.to_vec();
            sorted.sort_unstable();
            sorted
                .into_iter()
                .map(Key::Index)
                .chain(strings.iter().copied())
                .chain(symbols.iter().copied())
                .collect()
        };
        assert_eq!(
            agent.own_property_keys(object).unwrap(),
            expected(&indices, &strings)
        );

        // Remove every third key of each group, the first ones while the object was small.
        let mut removed = Vec::new();
        for position in (0..30).step_by(3).rev() {
            removed.push(strings.remove(position));
            removed.push(Key::Index(indices.remove(position)));
        }
        for &key in &removed {
            assert_eq!(agent.internal_delete(object, key), Ok(true));
        }
        assert_eq!(
            agent.own_property_keys(object).unwrap(),
            expected(&indices, &strings)
        );
        for &key in &removed {
            assert_eq!(agent.get_own_property(object, key), Ok(None));
        }
        let before = visited(&mut agent, object);
        for &key in strings.iter().chain(&symbols) {
            assert!(agent.get_own_property(object, key).unwrap().is_some());
        }
        for &index in &indices {
            assert!(
                agent
                    .get_own_property(object, Key::Index(index))
                    .unwrap()
                    .is_some()
            );
        }
        let lookups = (strings.len() + symbols.len() + indices.len()) as u64;
        assert!(visited(&mut agent, object) - before <= lookups);

        // Add as many again: the removed room is used first, and the order holds.
        for i in 0..30u32 {
            let string = agent.name_key(&format!("n{i}")).unwrap();
            strings.push(string);
            indices.push(100 + i);
            for key in [string, Key::Index(100 + i)] {
                assert_eq!(agent.create_data_property(object, key, number(i)), Ok(true));
            }
        }
        assert_eq!(
            agent.own_property_keys(object).unwrap(),
            expected(&indices, &strings)
        );
        for &key in &removed {
            assert_eq!(agent.get_own_property(object, key), Ok(None));
        }
    }

    /// The memory budget (decision 4): the index map an object gets above the small size is
    /// charged with the growth of its list, before it is made; over the budget the run ends in
    /// `Terminated(MemoryBudget)` and nothing is charged; and what it was charged is credited
    /// when the object is freed.
    #[test]
    fn the_room_of_the_index_map_is_charged_and_credited() {
        let (mut agent, _realm) = agent();
        let empty = agent.heap.bytes();
        let object = plain(&mut agent);
        for i in 0..SMALL as u32 {
            assert_eq!(
                agent.create_data_property(object, Key::Index(i), number(i)),
                Ok(true)
            );
        }
        let small = agent.heap.bytes();
        let list_growth = SMALL as u64 * Properties::entry_bytes();

        // Room for the list's growth alone is not enough.
        agent.heap.accounting.set_limit(small + list_growth);
        assert_eq!(
            agent.create_data_property(object, Key::Index(SMALL as u32), Value::Null),
            Err(Abrupt::Terminated(Termination::MemoryBudget))
        );
        assert_eq!(agent.heap.bytes(), small);
        agent.termination = None;

        agent.heap.accounting.set_limit(1 << 28);
        assert_eq!(
            agent.create_data_property(object, Key::Index(SMALL as u32), Value::Null),
            Ok(true)
        );
        let grown = agent.heap.bytes();
        assert!(
            grown > small + list_growth,
            "the map was not charged: {small} -> {grown}"
        );

        // Nothing holds the object: a collection frees it and credits all it was charged.
        assert!(agent.collect().is_ok());
        assert_eq!(agent.heap.bytes(), empty);
    }

    /// A scan that stays charges a step per entry it visits: when the list is full and at least
    /// half of it is room removed properties left, the next addition compacts it in place
    /// instead of growing it, one step per entry, and charges no memory.
    #[test]
    fn the_compaction_of_the_list_charges_a_step_per_entry() {
        let (mut agent, _realm) = agent();
        let object = plain(&mut agent);
        for i in 0..16u32 {
            assert_eq!(
                agent.create_data_property(object, Key::Index(i), number(i)),
                Ok(true)
            );
        }
        for i in (0..16u32).step_by(2) {
            assert_eq!(agent.internal_delete(object, Key::Index(i)), Ok(true));
        }
        let (steps, bytes) = (agent.steps, agent.heap.bytes());
        assert_eq!(
            agent.create_data_property(object, Key::Index(99), Value::Null),
            Ok(true)
        );
        assert_eq!(agent.steps - steps, 16, "the compaction visits 16 entries");
        assert_eq!(agent.heap.bytes(), bytes);
        let keys = agent.own_property_keys(object).unwrap();
        let expected: Vec<Key> = (1..16u32).step_by(2).chain([99]).map(Key::Index).collect();
        assert_eq!(keys, expected);
    }

    /// OrdinaryOwnPropertyKeys (`#sec-ordinaryownpropertykeys`) answers the indices ascending:
    /// added ascending they are read as they are, a step per key; added in another order they
    /// are sorted, and each comparison of the sort is a step more.
    #[test]
    fn a_sort_of_index_keys_charges_its_comparisons() {
        let (mut agent, _realm) = agent();
        let ascending = plain(&mut agent);
        let descending = plain(&mut agent);
        for i in 0..20u32 {
            for (object, index) in [(ascending, i), (descending, 19 - i)] {
                assert_eq!(
                    agent.create_data_property(object, Key::Index(index), Value::Null),
                    Ok(true)
                );
            }
        }
        let expected: Vec<Key> = (0..20u32).map(Key::Index).collect();
        let steps = agent.steps;
        assert_eq!(agent.own_property_keys(ascending).unwrap(), expected);
        assert_eq!(agent.steps - steps, 20);
        let steps = agent.steps;
        assert_eq!(agent.own_property_keys(descending).unwrap(), expected);
        let sorted = agent.steps - steps;
        assert!(sorted > 20, "the sort was not charged: {sorted} steps");
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
