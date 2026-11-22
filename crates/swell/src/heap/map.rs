//! The keyed hash map of the engine (`docs/design/engine.md`, decision 4, "Objects";
//! `docs/design/host-interface.md`, decision 3, HOST-145): `std`'s `HashMap` with the SipHash of
//! `swell-num` keyed by `Seeds::hash_key` and by nothing else, never `std`'s default hasher,
//! which reads the operating system's random source. The key decides only which keys collide;
//! no order the script can observe comes from a map's iteration. The atom table, the
//! GlobalSymbolRegistry (`heap/mod.rs`, `SymbolRegistry`) and the index
//! maps of property lists and of the bindings eval code adds (`object/property.rs`) are such
//! maps; the latter take the heap's hasher (`Heap::hasher`) and charge their table by
//! [`KeyedMap::table_bytes_for`] before it is made.

use std::borrow::Borrow;
use std::collections::HashMap;
use std::hash::Hash;

use swell_num::siphash::SipBuildHasher;

use super::accounting::OverBudget;

/// A hash map whose hasher is keyed by the agent's `hash_key`.
pub(crate) struct KeyedMap<K, V> {
    map: HashMap<K, V, SipBuildHasher>,
}

impl<K: Hash + Eq, V> KeyedMap<K, V> {
    /// An empty map keyed by `hash_key`.
    pub(crate) fn new(hash_key: &[u8; 16]) -> KeyedMap<K, V> {
        KeyedMap {
            map: HashMap::with_hasher(SipBuildHasher::new(hash_key)),
        }
    }

    /// The hash this map gives `key`.
    #[cfg(test)]
    pub(crate) fn hash_of<Q: Hash + ?Sized>(&self, key: &Q) -> u64 {
        std::hash::BuildHasher::hash_one(self.map.hasher(), key)
    }

    /// The value stored under `key`.
    pub(crate) fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.map.get(key)
    }

    /// Store `value` under `key` and answer the value it replaces. The table grows through
    /// `try_reserve`, so a refusal of the allocator is `OverBudget`, not an abort.
    pub(crate) fn insert(&mut self, key: K, value: V) -> Result<Option<V>, OverBudget> {
        self.map.try_reserve(1).map_err(|_| OverBudget)?;
        Ok(self.map.insert(key, value))
    }

    /// Keep the entries for which `keep` answers true.
    pub(crate) fn retain(&mut self, keep: impl FnMut(&K, &mut V) -> bool) {
        self.map.retain(keep);
    }

    /// The number of entries.
    pub(crate) fn len(&self) -> usize {
        self.map.len()
    }

    /// The values, in no order a script can observe: for a tracer, which reports each one and
    /// whose result does not depend on the order (the GlobalSymbolRegistry, `heap/mod.rs`).
    /// Allocates nothing.
    pub(crate) fn values(&self) -> impl Iterator<Item = &V> {
        self.map.values()
    }

    /// An empty map whose hasher is `hasher`, the one the heap keeps (`Heap::hasher`, keyed by
    /// `hash_key`).
    pub(crate) fn with_hasher(hasher: SipBuildHasher) -> KeyedMap<K, V> {
        KeyedMap {
            map: HashMap::with_hasher(hasher),
        }
    }

    /// Room for `additional` more entries, through `try_reserve`: a refusal of the allocator
    /// is `OverBudget`, not an abort. The caller charged the table's growth first.
    pub(crate) fn reserve(&mut self, additional: usize) -> Result<(), OverBudget> {
        self.map.try_reserve(additional).map_err(|_| OverBudget)
    }

    /// Remove the entry of `key` and answer its value.
    pub(crate) fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.map.remove(key)
    }

    /// Remove every entry; the table keeps its room.
    pub(crate) fn clear(&mut self) {
        self.map.clear();
    }

    /// The bytes of the table of a map with room for `capacity` entries, charged before the
    /// table is made: the layout of `std`'s map, a slot of an entry and a control byte per
    /// bucket and a group of 16 control bytes more, with a power of two of buckets at most
    /// seven eighths full (four buckets below 4 entries, eight below 8).
    pub(crate) fn table_bytes_for(capacity: usize) -> u64 {
        Self::bytes_of_buckets(Self::buckets_for(capacity))
    }

    /// The entries the table [`KeyedMap::table_bytes_for`] charges for `capacity` holds before
    /// `std`'s map grows it: seven eighths of its buckets, or its buckets less one below 8.
    pub(crate) fn full_capacity_for(capacity: usize) -> usize {
        let buckets = Self::buckets_for(capacity);
        if buckets < 8 {
            buckets.saturating_sub(1)
        } else {
            buckets / 8 * 7
        }
    }

    /// The entries the map takes before its next insertion rehashes its table: `std`'s
    /// `capacity` less the entries, where a bucket a removal marked as deleted counts against
    /// the room until the table is rehashed. At 0 the next insertion rehashes the table, in
    /// place while its entries and the new one fill at most half of what it holds, else into a
    /// table of twice the buckets.
    pub(crate) fn spare(&self) -> usize {
        self.map.capacity().saturating_sub(self.map.len())
    }

    /// The buckets of the table of a map with room for `capacity` entries.
    fn buckets_for(capacity: usize) -> usize {
        if capacity == 0 {
            0
        } else if capacity < 4 {
            4
        } else if capacity < 8 {
            8
        } else {
            (capacity.saturating_mul(8) / 7).next_power_of_two()
        }
    }

    /// The bytes of the table the map has now, by the same layout: its room answers its
    /// buckets (`capacity` is the buckets less one below 8, seven eighths of them above).
    pub(crate) fn table_bytes(&self) -> u64 {
        let capacity = self.map.capacity();
        let buckets = match capacity {
            0 => 0,
            1..8 => capacity.saturating_add(1),
            _ => (capacity / 7).saturating_mul(8),
        };
        Self::bytes_of_buckets(buckets)
    }

    /// The bytes of a table of `buckets` buckets.
    fn bytes_of_buckets(buckets: usize) -> u64 {
        if buckets == 0 {
            return 0;
        }
        let slots = buckets
            .saturating_mul(size_of::<(K, V)>())
            .next_multiple_of(16);
        let bytes = slots.saturating_add(buckets).saturating_add(16);
        u64::try_from(bytes).unwrap_or(u64::MAX)
    }
}

#[cfg(test)]
mod tests {
    use std::hash::BuildHasher;

    use swell_num::siphash::SipBuildHasher;

    use super::KeyedMap;
    use crate::{Agent, Limits, Seeds};

    use crate::host::hooks::test_host::NoHooks;

    fn keys() -> Vec<Vec<u16>> {
        (0..1_000)
            .map(|i| format!("key {i}").encode_utf16().collect())
            .collect()
    }

    /// HOST-145: the hasher of the engine's hash maps is keyed by `hash_key` alone: equal
    /// hashes for each of 1,000 keys under two equal `hash_key`s, and a different hash for at
    /// least one of them under two different ones.
    #[test]
    fn host_145_hash_is_keyed_by_hash_key_alone() {
        let keys = keys();
        let mut other_key = [3; 16];
        other_key[15] = 4;
        let a: KeyedMap<Vec<u16>, u32> = KeyedMap::new(&[3; 16]);
        let b: KeyedMap<Vec<u16>, u32> = KeyedMap::new(&[3; 16]);
        let c: KeyedMap<Vec<u16>, u32> = KeyedMap::new(&other_key);
        for key in &keys {
            assert_eq!(a.hash_of(key), b.hash_of(key));
            // The hash is the SipHash of `swell-num` under that key.
            assert_eq!(a.hash_of(key), SipBuildHasher::new(&[3; 16]).hash_one(key));
        }
        assert!(keys.iter().any(|key| a.hash_of(key) != c.hash_of(key)));

        // The maps of two agents that differ in everything but `hash_key` hash alike.
        let limits = |memory_bytes| Limits {
            steps_per_entry: 1_000,
            memory_bytes,
            frame_depth: 100,
            reentry_depth: 10,
            parser_nesting: 100,
            string_length: 1 << 20,
            bigint_bits: 1 << 10,
        };
        let one = Agent::new(
            NoHooks,
            limits(1 << 20),
            Seeds {
                hash_key: [5; 16],
                random_seed: [1; 32],
            },
        )
        .unwrap();
        let two = Agent::new(
            NoHooks,
            limits(1 << 21),
            Seeds {
                hash_key: [5; 16],
                random_seed: [9; 32],
            },
        )
        .unwrap();
        let three = Agent::new(
            NoHooks,
            limits(1 << 20),
            Seeds {
                hash_key: [6; 16],
                random_seed: [1; 32],
            },
        )
        .unwrap();
        for key in &keys {
            assert_eq!(one.heap.atoms.hash_of(key), two.heap.atoms.hash_of(key));
        }
        assert!(
            keys.iter()
                .any(|key| one.heap.atoms.hash_of(key) != three.heap.atoms.hash_of(key))
        );
    }

    /// The map keeps what is inserted and finds it by an equal key.
    #[test]
    fn a_keyed_map_maps() {
        let mut map: KeyedMap<Vec<u16>, usize> = KeyedMap::new(&[0; 16]);
        for (i, key) in keys().into_iter().enumerate() {
            assert_eq!(map.insert(key, i), Ok(None));
        }
        assert_eq!(map.len(), 1_000);
        assert_eq!(
            map.get(&"key 17".encode_utf16().collect::<Vec<u16>>()),
            Some(&17)
        );
        map.retain(|_, value| *value % 2 == 0);
        assert_eq!(map.len(), 500);
        assert_eq!(
            map.get(&"key 17".encode_utf16().collect::<Vec<u16>>()),
            None
        );
    }

    /// The bytes charged for a table before it is made are those of the table the map then
    /// has, for every room the lists of the engine ask (4 doubling to 2^17), and a removal
    /// keeps the others and the table's room.
    #[test]
    fn the_table_charged_is_the_table_made() {
        for shift in 2..18 {
            let wanted = 1usize << shift;
            let mut map: KeyedMap<u32, u32> = KeyedMap::new(&[0; 16]);
            assert_eq!(map.table_bytes(), 0);
            map.reserve(wanted).unwrap();
            assert_eq!(
                map.table_bytes(),
                KeyedMap::<u32, u32>::table_bytes_for(wanted),
                "room {wanted}"
            );
        }
        let mut map: KeyedMap<u32, u32> = KeyedMap::new(&[0; 16]);
        map.reserve(16).unwrap();
        let bytes = map.table_bytes();
        for i in 0..16 {
            assert_eq!(map.insert(i, i), Ok(None));
        }
        assert_eq!(map.remove(&3), Some(3));
        assert_eq!(map.remove(&3), None);
        assert_eq!(map.len(), 15);
        assert_eq!(map.get(&4), Some(&4));
        map.clear();
        assert_eq!(map.len(), 0);
        assert_eq!(map.table_bytes(), bytes);
    }
}
