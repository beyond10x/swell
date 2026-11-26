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
//! answered comes from the list. OrdinaryOwnPropertyKeys answers the indices ascending: the
//! list records whether its index keys were added ascending (the usual case), and then they are
//! read in place; otherwise [`sort_indices`] sorts them by a bottom-up merge sort whose caller
//! charges n steps before each pass (`object/mod.rs`,
//! `story:own-keys-charged-before-the-list-exists`).
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
use alloc::boxed::Box;
use alloc::vec::Vec;
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
impl core::hash::Hash for Key {
    fn hash<S: core::hash::Hasher>(&self, state: &mut S) {
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
/// when it is freed, and the entries a compaction visited. An object's properties and the
/// bindings eval code adds charge those as steps before they ask for room
/// ([`KeyedList::compaction_visits`]) and do not read `visited`.
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
    /// Whether every index key was added above the one added before it: one comparison at
    /// each addition of an index key; a removal or a compaction keeps it.
    ascending: bool,
    /// The index key added last.
    last_index: Option<u32>,
    /// The entries lookups visited, in the counting build of the tests.
    #[cfg(test)]
    visited: core::cell::Cell<u64>,
    /// The comparisons sorts of its index keys made, in the counting build of the tests.
    #[cfg(test)]
    compared: core::cell::Cell<u64>,
}

impl<V> Default for KeyedList<V> {
    fn default() -> Self {
        KeyedList {
            entries: Vec::new(),
            index: None,
            ascending: true,
            last_index: None,
            #[cfg(test)]
            visited: core::cell::Cell::new(0),
            #[cfg(test)]
            compared: core::cell::Cell::new(0),
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

    /// The comparisons sorts of the index keys made so far.
    #[cfg(test)]
    pub(crate) fn compared(&self) -> u64 {
        self.compared.get()
    }

    /// Count `n` comparisons a sort of the index keys made, in the counting build.
    #[cfg(test)]
    pub(crate) fn count_compared(&self, n: u64) {
        self.compared.set(self.compared.get().saturating_add(n));
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

    /// The first entry, if it holds one.
    pub(crate) fn first(&self) -> Option<&(Key, V)> {
        self.entries.first()?.as_ref()
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

    /// Whether [`KeyedList::make_room`] compacts the list in place: it is full and at least
    /// half of it is positions removals left empty.
    fn compacts(&self) -> bool {
        let length = self.entries.len();
        length >= self.entries.capacity()
            && self
                .index
                .as_ref()
                .is_some_and(|index| index.removed > 0 && index.removed.saturating_mul(2) >= length)
    }

    /// The entries [`KeyedList::make_room`] would visit if it were asked now: the list's length
    /// when it compacts, else 0. Read from the length, the room and the removed positions,
    /// without a walk, so that the caller charges them as steps before the compaction runs
    /// (`docs/design/engine.md`, "Memory budget": work is charged before it is done).
    pub(crate) fn compaction_visits(&self) -> u64 {
        if self.compacts() {
            to_u64(self.entries.len())
        } else {
            0
        }
    }

    /// Make room for one more entry: nothing while the list has room; a compaction in place
    /// when it is full and at least half of it is empty (`room.visited` entries visited, as
    /// [`KeyedList::compaction_visits`] answers before it runs); otherwise the list's room
    /// doubles (at least 4) and, once that passes [`SMALL`], the map gets as much. The growth
    /// of both is charged to `accounting` before it is made and added to `room.bytes`; over
    /// the budget nothing more is charged or made and the answer is `OverBudget`. What
    /// `room.bytes` holds was charged, also on a refusal, and the owner records it.
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
        if self.compacts() {
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
        #[expect(
            clippy::disallowed_methods,
            reason = "charged as `list_bytes` before it grows"
        )]
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
            // The map took more than its layout says: charged now.
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
        if let Key::Index(added) = key {
            if self.last_index.is_some_and(|last| added < last) {
                self.ascending = false;
            }
            self.last_index = Some(added);
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

/// Sort `indices` ascending by a bottom-up merge sort through `buffer`, as long
/// (`docs/design/engine.md`, "The step budget"): each pass merges runs of `width` from one
/// into the other, which then change places, so the sorted indices end in `indices`. Before
/// each pass `before_pass` is asked with the length, the most comparisons a pass makes; an
/// error ends the sort. `compared` counts the comparisons made.
pub(crate) fn sort_indices<E>(
    indices: &mut Vec<u32>,
    buffer: &mut Vec<u32>,
    compared: &mut u64,
    mut before_pass: impl FnMut(u64) -> Result<(), E>,
) -> Result<(), E> {
    let length = indices.len();
    let mut width: usize = 1;
    while width < length {
        before_pass(to_u64(length))?;
        let run = width.saturating_mul(2);
        for (pair, out) in indices.chunks(run).zip(buffer.chunks_mut(run)) {
            let (left, right) = pair.split_at(width.min(pair.len()));
            merge(left, right, out, compared);
        }
        core::mem::swap(indices, buffer);
        width = run;
    }
    Ok(())
}

/// Merge the ascending runs `left` and `right` into `out`, as long as both, counting each
/// comparison in `compared`.
fn merge(left: &[u32], right: &[u32], out: &mut [u32], compared: &mut u64) {
    let mut left = left.iter().copied().peekable();
    let mut right = right.iter().copied().peekable();
    for slot in out {
        let next = match (left.peek(), right.peek()) {
            (Some(&a), Some(&b)) => {
                *compared = compared.saturating_add(1);
                if b < a { right.next() } else { left.next() }
            }
            (Some(_), None) => left.next(),
            (None, _) => right.next(),
        };
        if let Some(value) = next {
            *slot = value;
        }
    }
}

/// The own properties of an object, in creation order.
pub(crate) type Properties = KeyedList<Property>;

impl KeyedList<Property> {
    /// Whether the list ever held an array index; false means it holds none now.
    pub(crate) fn may_hold_index(&self) -> bool {
        self.last_index.is_some()
    }

    /// The array indices of the list, in creation order.
    pub(crate) fn listed_indices(&self) -> impl Iterator<Item = u32> + '_ {
        self.iter().filter_map(|(key, _)| match key {
            Key::Index(index) => Some(*index),
            _ => None,
        })
    }

    /// The keys that are not array indices, in the order of OrdinaryOwnPropertyKeys
    /// (`#sec-ordinaryownpropertykeys`): the Strings, then the Symbols, each in creation order.
    pub(crate) fn names(&self) -> impl Iterator<Item = Key> + '_ {
        let strings = self
            .iter()
            .filter(|(key, _)| matches!(key, Key::String(_)))
            .map(|(key, _)| *key);
        let symbols = self
            .iter()
            .filter(|(key, _)| matches!(key, Key::Symbol(_)))
            .map(|(key, _)| *key);
        strings.chain(symbols)
    }

    /// `None` when the array indices of the list are ascending in creation order, as
    /// OrdinaryOwnPropertyKeys answers them: added ascending, which the list records, or found
    /// so by a walk of the list. Otherwise how many there are, for a sort.
    pub(crate) fn unsorted_index_count(&self) -> Option<usize> {
        if self.ascending {
            return None;
        }
        let (mut count, mut last, mut sorted) = (0, None, true);
        for index in self.listed_indices() {
            if last.is_some_and(|last| index < last) {
                sorted = false;
            }
            last = Some(index);
            count += 1;
        }
        (!sorted).then_some(count)
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
    use super::{Key, Properties, Property, SMALL, Slot, array_index};
    use crate::Realm;
    use crate::host::agent::Agent;
    use crate::host::hooks::test_host::{NoHooks, agent_with_realm_and, enter, limits};
    use crate::host::outcome::{Abrupt, Termination};
    use crate::object::ordinary::OrdinaryData;
    use crate::value::{ObjectRef, Value};
    use alloc::format;
    use alloc::vec;
    use alloc::vec::Vec;

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
            agent.own_property_keys(object).unwrap().keys,
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
            agent.own_property_keys(object).unwrap().keys,
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
            agent.own_property_keys(object).unwrap().keys,
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
        let keys = agent.own_property_keys(object).unwrap().keys;
        let expected: Vec<Key> = (1..16u32).step_by(2).chain([99]).map(Key::Index).collect();
        assert_eq!(keys, expected);
    }

    /// An object whose list is full at 65,536 index keys, with the `removed` lowest even ones
    /// removed.
    fn full_list(agent: &mut Agent<NoHooks>, removed: u32) -> ObjectRef {
        const N: u32 = 65_536;
        let object = plain(agent);
        for i in 0..N {
            assert_eq!(
                agent.create_data_property(object, Key::Index(i), Value::Null),
                Ok(true)
            );
        }
        for i in (0..N).step_by(2).take(removed as usize) {
            assert_eq!(agent.internal_delete(object, Key::Index(i)), Ok(true));
        }
        let list = &agent.engine_object(object).unwrap().properties;
        assert_eq!(
            (list.entries.len(), list.entries.capacity()),
            (65_536, 65_536)
        );
        object
    }

    /// The length of the list of `object` with its empty positions, and the positions removals
    /// left empty.
    fn list_state(agent: &mut Agent<NoHooks>, object: ObjectRef) -> (usize, Option<usize>) {
        let list = &agent.engine_object(object).unwrap().properties;
        (
            list.entries.len(),
            list.index.as_ref().map(|index| index.removed),
        )
    }

    /// `docs/design/engine.md`, "Memory budget" (work is charged before it is done): the
    /// entries a compaction would visit are read from the list's length, room and removals
    /// before it runs: 65,536 for a full list of 65,536 entries with 32,768 removed, and 0
    /// with one removed fewer than half.
    #[test]
    fn a_compaction_is_counted_before_it_runs() {
        let (mut agent, _realm) = agent();
        let object = full_list(&mut agent, 32_767);
        let visits = |agent: &mut Agent<NoHooks>| {
            agent
                .engine_object(object)
                .unwrap()
                .properties
                .compaction_visits()
        };
        assert_eq!(visits(&mut agent), 0);
        assert_eq!(agent.internal_delete(object, Key::Index(65_534)), Ok(true));
        assert_eq!(list_state(&mut agent, object), (65_536, Some(32_768)));
        assert_eq!(visits(&mut agent), 65_536);
    }

    /// `docs/design/engine.md`, "Memory budget" (work is charged before it is done): an
    /// object whose list is full at 65,536 entries with 32,768 removed, and one step left. The
    /// addition of a new key ends in `Terminated(StepBudget)` before the compaction runs: the
    /// list keeps its length and its removed positions.
    #[test]
    fn an_added_property_charges_its_compaction_before_it_runs() {
        let (mut agent, _realm) = agent();
        let object = full_list(&mut agent, 32_768);
        assert_eq!(list_state(&mut agent, object), (65_536, Some(32_768)));
        agent.steps = 0;
        agent.limits.steps_per_entry = 1;
        assert_eq!(
            agent.add_property(
                object,
                Key::Index(70_000),
                Property::data(Value::Null, true, true, true)
            ),
            Err(Abrupt::Terminated(Termination::StepBudget))
        );
        assert_eq!(list_state(&mut agent, object), (65_536, Some(32_768)));
    }

    /// OrdinaryOwnPropertyKeys (`#sec-ordinaryownpropertykeys`) answers the indices ascending:
    /// added ascending they are read as they are, a step per key; added in another order they
    /// are sorted, and each pass of the sort charges a step per key more.
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
        assert_eq!(agent.own_property_keys(ascending).unwrap().keys, expected);
        assert_eq!(agent.steps - steps, 20);
        let steps = agent.steps;
        assert_eq!(agent.own_property_keys(descending).unwrap().keys, expected);
        let sorted = agent.steps - steps;
        assert!(sorted > 20, "the sort was not charged: {sorted} steps");
    }

    /// `docs/design/engine.md`, "The step budget": a sort of n index keys added
    /// out of order charges n steps before each pass, so the step budget is read every n
    /// comparisons. n = 65,536 keys added in the order i × 40,503 mod 65,536 (a permutation,
    /// 40,503 being odd), with 2n + 10 steps left: the n steps of the keys and one pass fit, a
    /// second pass does not. At most n comparisons happen before `Terminated(StepBudget)`.
    #[test]
    fn index_keys_added_out_of_order_charge_n_steps_before_each_pass() {
        const N: u32 = 65_536;
        let (mut agent, _realm) = agent();
        let object = plain(&mut agent);
        for i in 0..N {
            let index = i.wrapping_mul(40_503) % N;
            assert_eq!(
                agent.create_data_property(object, Key::Index(index), Value::Null),
                Ok(true)
            );
        }
        let compared =
            |agent: &mut Agent<NoHooks>| agent.engine_object(object).unwrap().properties.compared();
        let before = compared(&mut agent);
        agent.steps = 0;
        agent.limits.steps_per_entry = 2 * u64::from(N) + 10;
        let answer = agent.own_property_keys(object).map(|keys| keys.len());
        let made = compared(&mut agent) - before;
        assert_eq!(answer, Err(Abrupt::Terminated(Termination::StepBudget)));
        assert!(
            made <= u64::from(N),
            "{made} comparisons before the step budget ended the run, n = {N}"
        );
    }

    /// OrdinaryOwnPropertyKeys (`#sec-ordinaryownpropertykeys`), whatever order the keys were
    /// added in: 64 fixed permutations of the index keys 0 to 999 (the first ascending, the
    /// second descending, the others from a generator seeded here), added with String and
    /// Symbol keys between them, come out as the indices ascending, then the Strings, then the
    /// Symbols, each in the order they were added.
    #[test]
    fn keys_in_any_order_come_out_in_the_order_of_the_clause() {
        const N: usize = 1_000;
        let (mut agent, _realm) = agent();
        let strings: Vec<Key> = (0..10)
            .map(|i| agent.name_key(&format!("s{i}")).unwrap())
            .collect();
        let symbols: Vec<Key> = (0..3)
            .map(|_| Key::Symbol(agent.heap.new_symbol(None).unwrap()))
            .collect();
        // xorshift64, seeded here: the same permutations on every run.
        let mut state: u64 = 0x2545_f491_4f6c_dd1d;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        for permutation in 0..64 {
            let mut indices: Vec<u32> = (0..N as u32).collect();
            match permutation {
                0 => {}
                1 => indices.reverse(),
                _ => {
                    for i in (1..N).rev() {
                        let j = usize::try_from(next() % (i as u64 + 1)).unwrap();
                        indices.swap(i, j);
                    }
                }
            }
            let object = plain(&mut agent);
            let (mut added_strings, mut added_symbols) = (Vec::new(), Vec::new());
            for (position, &index) in indices.iter().enumerate() {
                let mut keys = vec![Key::Index(index)];
                if position % 100 == 50 {
                    keys.push(strings[(position / 100 + permutation) % strings.len()]);
                }
                if position % 400 == 7 {
                    keys.push(symbols[position / 400]);
                }
                for key in keys {
                    if agent.get_own_property(object, key).unwrap().is_some() {
                        continue;
                    }
                    assert_eq!(
                        agent.create_data_property(object, key, Value::Null),
                        Ok(true)
                    );
                    match key {
                        Key::String(_) => added_strings.push(key),
                        Key::Symbol(_) => added_symbols.push(key),
                        Key::Index(_) => {}
                    }
                }
            }
            let expected: Vec<Key> = (0..N as u32)
                .map(Key::Index)
                .chain(added_strings)
                .chain(added_symbols)
                .collect();
            let keys = agent.own_property_keys(object).unwrap().to_vec();
            assert_eq!(keys, expected, "permutation {permutation}");
        }
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
