//! The keyed hash map of the engine (`docs/design/engine.md`, decision 4, "Objects";
//! `docs/design/host-interface.md`, decision 3, HOST-145): `std`'s `HashMap` with the SipHash of
//! `swell-num` keyed by `Seeds::hash_key` and by nothing else, never `std`'s default hasher,
//! which reads the operating system's random source. The key decides only which keys collide;
//! no order the script can observe comes from a map's iteration.

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
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.map.len()
    }
}

#[cfg(test)]
mod tests {
    use std::hash::BuildHasher;

    use swell_num::siphash::SipBuildHasher;

    use super::KeyedMap;
    use crate::{Agent, Host, Limits, Seeds};

    struct NoHooks;

    impl Host for NoHooks {}

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
}
