//! The keyed hash map of the engine (`docs/design/engine.md`, decision 4, "Objects";
//! `docs/design/host-interface.md`, decision 3, HOST-145): a hash map written here, with the
//! SipHash of `swell-num` keyed by `Seeds::hash_key` and by nothing else, never a hasher that
//! reads the operating system's random source. The key decides only which keys collide; no
//! order the script can observe comes from a map's iteration. The atom table, the
//! GlobalSymbolRegistry (`heap/mod.rs`, `SymbolRegistry`) and the index
//! maps of property lists and of the bindings eval code adds (`object/property.rs`) are such
//! maps; the latter take the heap's hasher (`Heap::hasher`) and charge their table by
//! [`KeyedMap::table_bytes_for`] before it is made.
//!
//! The table is open addressing with a control byte per bucket, in groups of 16 buckets probed
//! triangularly: the layout and the policy of growth, deletion and rehashing of the map the
//! standard library had here before (a SwissTable, with the group of 16 of x86-64), so that a
//! table grows at the same insertion, is rehashed in place at the same point, and has the same
//! room ([`KeyedMap::spare`]) and charge ([`KeyedMap::table_bytes`]) after every operation. A
//! control byte is `EMPTY`, `DELETED` (a removal inside a run of full groups, which a probe must
//! walk past) or the top 7 bits of the hash of the entry it holds; the 16 bytes after the last
//! bucket repeat the first 16, so a group read near the end needs no wrap. A group is read as
//! two 64-bit words and matched with word arithmetic: the crate has no `unsafe` and calls no
//! `core::arch` intrinsic (decision 6).

use alloc::vec::Vec;
use core::borrow::Borrow;
use core::hash::{BuildHasher, Hash};

use swell_num::siphash::SipBuildHasher;

use super::accounting::OverBudget;

/// The buckets of a group: one probe reads the control bytes of 16 buckets.
const WIDTH: usize = 16;
/// The control byte of a bucket that never held an entry since the table was made or cleared.
const EMPTY: u8 = 0xFF;
/// The control byte of a bucket whose entry was removed while a probe may need to walk past it.
const DELETED: u8 = 0x80;

/// The control byte of a full bucket: the top 7 bits of its entry's hash.
fn h2(hash: u64) -> u8 {
    // The shift leaves 7 bits.
    u8::try_from(hash >> 57).unwrap_or(0)
}

/// The bucket where the probe of `hash` starts, before the mask.
fn h1(hash: u64) -> usize {
    // The low bits of the hash: on a 32-bit target the high half is dropped.
    hash as usize
}

/// The entries a table with the mask `bucket_mask` holds before it grows: seven eighths of its
/// buckets, or its buckets less one below 8.
fn bucket_mask_to_capacity(bucket_mask: usize) -> usize {
    if bucket_mask < 8 {
        bucket_mask
    } else {
        bucket_mask.saturating_add(1) / 8 * 7
    }
}

/// The 16 control bytes of a group as two words, byte `i` of a word in its bits `8 i` to
/// `8 i + 7`: buckets 0 to 7 in `low`, 8 to 15 in `high`.
#[derive(Clone, Copy)]
struct Group {
    low: u64,
    high: u64,
}

/// The byte 0x80 in every byte of a word: the top bits.
const TOPS: u64 = 0x8080_8080_8080_8080;
/// The byte 0x01 in every byte of a word.
const ONES: u64 = 0x0101_0101_0101_0101;

/// The buckets of a group a set marks, as two words with the top bit of each member's byte
/// set, taken lowest first.
struct Members {
    low: u64,
    high: u64,
}

impl Iterator for Members {
    type Item = usize;

    #[inline]
    fn next(&mut self) -> Option<usize> {
        if self.low != 0 {
            let offset = (self.low.trailing_zeros() / 8) as usize;
            self.low &= self.low - 1;
            Some(offset)
        } else if self.high != 0 {
            let offset = (self.high.trailing_zeros() / 8) as usize;
            self.high &= self.high - 1;
            Some(offset + 8)
        } else {
            None
        }
    }
}

impl Group {
    /// The buckets whose control byte may be `byte`: each that is, and rarely one that is not
    /// (a byte one above a matching byte can borrow), so the caller compares the key of each.
    #[inline]
    fn candidates(self, byte: u8) -> Members {
        let splat = ONES.wrapping_mul(u64::from(byte));
        let low = self.low ^ splat;
        let high = self.high ^ splat;
        Members {
            low: low.wrapping_sub(ONES) & !low & TOPS,
            high: high.wrapping_sub(ONES) & !high & TOPS,
        }
    }

    /// The empty buckets: of the three kinds of byte only `EMPTY` has bits 6 and 7 both set.
    #[inline]
    fn empty(self) -> Members {
        Members {
            low: self.low & (self.low << 1) & TOPS,
            high: self.high & (self.high << 1) & TOPS,
        }
    }

    /// Whether a bucket is empty.
    #[inline]
    fn has_empty(self) -> bool {
        let empty = self.empty();
        empty.low | empty.high != 0
    }

    /// The first bucket that is empty or deleted: the bytes with bit 7 set.
    #[inline]
    fn first_empty_or_deleted(self) -> Option<usize> {
        Members {
            low: self.low & TOPS,
            high: self.high & TOPS,
        }
        .next()
    }

    /// The full or deleted buckets at the start of the group, before its first empty one; 16
    /// for none.
    fn taken_at_start(self) -> u32 {
        let empty = self.empty();
        if empty.low != 0 {
            empty.low.trailing_zeros() / 8
        } else {
            8 + empty.high.trailing_zeros() / 8
        }
    }

    /// The full or deleted buckets at the end of the group, after its last empty one; 16 for
    /// none.
    fn taken_at_end(self) -> u32 {
        let empty = self.empty();
        if empty.high != 0 {
            empty.high.leading_zeros() / 8
        } else {
            8 + empty.low.leading_zeros() / 8
        }
    }
}

/// The sequence of groups a probe reads: from the bucket of the hash, then 16, 32, 48 and so
/// on buckets further, wrapping, which reaches every group of a table of a power of two.
struct Probe {
    pos: usize,
    stride: usize,
}

impl Probe {
    fn next(&mut self, bucket_mask: usize) {
        self.stride = self.stride.wrapping_add(WIDTH);
        self.pos = self.pos.wrapping_add(self.stride) & bucket_mask;
    }
}

/// A hash map whose hasher is keyed by the agent's `hash_key`.
pub(crate) struct KeyedMap<K, V> {
    hasher: SipBuildHasher,
    /// A control byte per bucket and 16 more, or none while the map has no table.
    ctrl: Vec<u8>,
    /// The entry of each bucket; `Some` exactly where the control byte is a full one.
    slots: Vec<Option<(K, V)>>,
    /// The buckets less one; 0 while the map has no table.
    bucket_mask: usize,
    /// The entries.
    items: usize,
    /// The entries the table takes before it is rehashed: its capacity less the entries and
    /// the deleted buckets.
    growth_left: usize,
}

impl<K: Hash + Eq, V> KeyedMap<K, V> {
    /// An empty map keyed by `hash_key`.
    pub(crate) fn new(hash_key: &[u8; 16]) -> KeyedMap<K, V> {
        KeyedMap::with_hasher(SipBuildHasher::new(hash_key))
    }

    /// The hash this map gives `key`.
    #[cfg(test)]
    pub(crate) fn hash_of<Q: Hash + ?Sized>(&self, key: &Q) -> u64 {
        self.hasher.hash_one(key)
    }

    /// The value stored under `key`.
    #[inline]
    pub(crate) fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let (_, (_, value)) = self.find(self.hasher.hash_one(key), key)?;
        Some(value)
    }

    /// Store `value` under `key` and answer the value it replaces; a key already stored keeps
    /// the key it was stored with. The table grows first when it has no room for one more
    /// entry, through `try_reserve`, so a refusal of the allocator is `OverBudget`, not an
    /// abort.
    #[inline]
    pub(crate) fn insert(&mut self, key: K, value: V) -> Result<Option<V>, OverBudget> {
        self.reserve(1)?;
        let hash = self.hasher.hash_one(&key);
        let h2 = h2(hash);
        let mask = self.bucket_mask;
        let mut probe = self.probe(hash);
        let mut insert_slot = None;
        for _ in 0..=mask {
            let group = self.group(probe.pos);
            for offset in group.candidates(h2) {
                let index = probe.pos.wrapping_add(offset) & mask;
                if let Some(Some((stored, old))) = self.slots.get_mut(index)
                    && *stored == key
                {
                    return Ok(Some(core::mem::replace(old, value)));
                }
            }
            if insert_slot.is_none()
                && let Some(offset) = group.first_empty_or_deleted()
            {
                insert_slot = Some(probe.pos.wrapping_add(offset) & mask);
            }
            if group.has_empty() {
                break;
            }
            probe.next(mask);
        }
        let index = self.fix_insert_slot(insert_slot.ok_or(OverBudget)?);
        if self.ctrl_at(index) == EMPTY {
            self.growth_left = self.growth_left.saturating_sub(1);
        }
        let slot = self.slots.get_mut(index).ok_or(OverBudget)?;
        *slot = Some((key, value));
        self.set_ctrl(index, h2);
        self.items = self.items.saturating_add(1);
        Ok(None)
    }

    /// Keep the entries for which `keep` answers true.
    pub(crate) fn retain(&mut self, mut keep: impl FnMut(&K, &mut V) -> bool) {
        for index in 0..self.slots.len() {
            let Some(Some((key, value))) = self.slots.get_mut(index) else {
                continue;
            };
            if !keep(key, value) {
                self.erase(index);
            }
        }
    }

    /// The number of entries.
    pub(crate) fn len(&self) -> usize {
        self.items
    }

    /// The values, in no order a script can observe: for a tracer, which reports each one and
    /// whose result does not depend on the order (the GlobalSymbolRegistry, `heap/mod.rs`).
    /// Allocates nothing.
    pub(crate) fn values(&self) -> impl Iterator<Item = &V> {
        self.slots
            .iter()
            .filter_map(|slot| slot.as_ref().map(|(_, value)| value))
    }

    /// An empty map whose hasher is `hasher`, the one the heap keeps (`Heap::hasher`, keyed by
    /// `hash_key`). It has no table until it is given room.
    pub(crate) fn with_hasher(hasher: SipBuildHasher) -> KeyedMap<K, V> {
        KeyedMap {
            hasher,
            ctrl: Vec::new(),
            slots: Vec::new(),
            bucket_mask: 0,
            items: 0,
            growth_left: 0,
        }
    }

    /// Room for `additional` more entries, through `try_reserve`: a refusal of the allocator
    /// is `OverBudget`, not an abort. The caller charged the table's growth first. With less
    /// room than that the table is rehashed: in place while its entries and the new ones fill
    /// at most half of what it holds, else into a table for at least one more entry than it
    /// holds.
    #[inline]
    pub(crate) fn reserve(&mut self, additional: usize) -> Result<(), OverBudget> {
        if additional <= self.growth_left {
            return Ok(());
        }
        let new_items = self.items.checked_add(additional).ok_or(OverBudget)?;
        let full_capacity = bucket_mask_to_capacity(self.bucket_mask);
        if new_items <= full_capacity / 2 {
            self.rehash_in_place();
            Ok(())
        } else {
            self.resize(new_items.max(full_capacity.saturating_add(1)))
        }
    }

    /// Remove the entry of `key` and answer its value.
    #[inline]
    pub(crate) fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let (index, _) = self.find(self.hasher.hash_one(key), key)?;
        self.erase(index).map(|(_, value)| value)
    }

    /// Remove every entry; the table keeps its room. A map with no entry is left as it is,
    /// deleted buckets included.
    pub(crate) fn clear(&mut self) {
        if self.items == 0 {
            return;
        }
        for slot in &mut self.slots {
            *slot = None;
        }
        self.ctrl.fill(EMPTY);
        self.items = 0;
        self.growth_left = bucket_mask_to_capacity(self.bucket_mask);
    }

    /// The bytes of the table of a map with room for `capacity` entries, charged before the
    /// table is made: a slot of an entry and a control byte per bucket and a group of 16
    /// control bytes more, with a power of two of buckets at most seven eighths full (four
    /// buckets below 4 entries, eight below 8).
    pub(crate) fn table_bytes_for(capacity: usize) -> u64 {
        Self::bytes_of_buckets(Self::buckets_for(capacity))
    }

    /// The entries the table [`KeyedMap::table_bytes_for`] charges for `capacity` holds before
    /// the map grows it: seven eighths of its buckets, or its buckets less one below 8.
    pub(crate) fn full_capacity_for(capacity: usize) -> usize {
        let buckets = Self::buckets_for(capacity);
        if buckets < 8 {
            buckets.saturating_sub(1)
        } else {
            buckets / 8 * 7
        }
    }

    /// The entries the map takes before its next insertion rehashes its table: its capacity
    /// less the entries, where a bucket a removal marked as deleted counts against the room
    /// until the table is rehashed. At 0 the next insertion rehashes the table, in place while
    /// its entries and the new one fill at most half of what it holds, else into a table of
    /// twice the buckets.
    pub(crate) fn spare(&self) -> usize {
        self.growth_left
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

    /// The bytes of the table the map has now, by the same layout, read from its room (its
    /// entries and [`KeyedMap::spare`]) as the charges have always read it: the room answers
    /// the buckets (the buckets less one below 8, seven eighths of them above).
    pub(crate) fn table_bytes(&self) -> u64 {
        let capacity = self.items.saturating_add(self.growth_left);
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

    /// The probe of `hash` in this table.
    #[inline]
    fn probe(&self, hash: u64) -> Probe {
        Probe {
            pos: h1(hash) & self.bucket_mask,
            stride: 0,
        }
    }

    /// The control byte of bucket `index` (or of the repeated group after the last bucket).
    #[inline]
    fn ctrl_at(&self, index: usize) -> u8 {
        self.ctrl.get(index).copied().unwrap_or(EMPTY)
    }

    /// The 16 control bytes from bucket `pos`.
    #[inline]
    fn group(&self, pos: usize) -> Group {
        let word = |bytes: Option<&[u8; 8]>| bytes.copied().map_or(u64::MAX, u64::from_le_bytes);
        let bytes = self.ctrl.get(pos..pos.wrapping_add(WIDTH));
        Group {
            low: word(bytes.and_then(<[u8]>::first_chunk)),
            high: word(bytes.and_then(<[u8]>::last_chunk)),
        }
    }

    /// Set the control byte of bucket `index` and its copy after the last bucket: for a bucket
    /// of the first group, the byte `WIDTH` past the end of the buckets; for any other, itself.
    #[inline]
    fn set_ctrl(&mut self, index: usize, byte: u8) {
        let mirror = (index.wrapping_sub(WIDTH) & self.bucket_mask).wrapping_add(WIDTH);
        for at in [index, mirror] {
            if let Some(slot) = self.ctrl.get_mut(at) {
                *slot = byte;
            }
        }
    }

    /// The bucket that holds `key`, whose hash is `hash`, and its entry.
    #[inline]
    fn find<Q>(&self, hash: u64, key: &Q) -> Option<(usize, &(K, V))>
    where
        K: Borrow<Q>,
        Q: Eq + ?Sized,
    {
        let h2 = h2(hash);
        let mask = self.bucket_mask;
        let mut probe = self.probe(hash);
        for _ in 0..=mask {
            let group = self.group(probe.pos);
            for offset in group.candidates(h2) {
                let index = probe.pos.wrapping_add(offset) & mask;
                if let Some(Some(entry)) = self.slots.get(index)
                    && entry.0.borrow() == key
                {
                    return Some((index, entry));
                }
            }
            if group.has_empty() {
                return None;
            }
            probe.next(mask);
        }
        None
    }

    /// The first empty or deleted bucket on the probe of `hash`.
    #[inline]
    fn find_insert_slot(&self, hash: u64) -> usize {
        let mut probe = self.probe(hash);
        for _ in 0..=self.bucket_mask {
            if let Some(offset) = self.group(probe.pos).first_empty_or_deleted() {
                return self.fix_insert_slot(probe.pos.wrapping_add(offset) & self.bucket_mask);
            }
            probe.next(self.bucket_mask);
        }
        0
    }

    /// In a table of fewer buckets than a group, a group read past the last bucket sees empty
    /// bytes that are not buckets, which the mask maps onto a full bucket: the first empty or
    /// deleted bucket of the first group is taken instead.
    #[inline]
    fn fix_insert_slot(&self, index: usize) -> usize {
        if self.ctrl_at(index) & 0x80 == 0 {
            self.group(0).first_empty_or_deleted().unwrap_or(index)
        } else {
            index
        }
    }

    /// Remove the entry of bucket `index`. Its bucket becomes empty when no probe can have
    /// walked past it, that is when fewer than 16 full or deleted buckets run through it;
    /// else it is marked deleted and its room comes back at the next rehash.
    fn erase(&mut self, index: usize) -> Option<(K, V)> {
        let entry = self.slots.get_mut(index)?.take()?;
        let index_before = index.wrapping_sub(WIDTH) & self.bucket_mask;
        let taken_before = self.group(index_before).taken_at_end();
        let taken_after = self.group(index).taken_at_start();
        let byte = if taken_before + taken_after >= WIDTH as u32 {
            DELETED
        } else {
            self.growth_left = self.growth_left.saturating_add(1);
            EMPTY
        };
        self.set_ctrl(index, byte);
        self.items = self.items.saturating_sub(1);
        Some(entry)
    }

    /// The hash of the entry of bucket `index`.
    fn hash_at(&self, index: usize) -> Option<u64> {
        let (key, _) = self.slots.get(index)?.as_ref()?;
        Some(self.hasher.hash_one(key))
    }

    /// Rehash the table into its own buckets, which drops every deleted mark: each full bucket
    /// is marked deleted and each deleted one empty, then each entry so marked moves to the
    /// first free bucket of its probe, unless that is in the group its probe reached it by.
    fn rehash_in_place(&mut self) {
        let buckets = self.slots.len();
        for byte in self.ctrl.iter_mut().take(buckets.max(WIDTH)) {
            *byte = if *byte & 0x80 != 0 { EMPTY } else { DELETED };
        }
        let (from, to, count) = if buckets < WIDTH {
            (0, WIDTH, buckets)
        } else {
            (0, buckets, WIDTH)
        };
        let head: Vec<u8> = self.ctrl.iter().skip(from).take(count).copied().collect();
        for (byte, value) in self.ctrl.iter_mut().skip(to).zip(head) {
            *byte = value;
        }
        for index in 0..buckets {
            if self.ctrl_at(index) != DELETED {
                continue;
            }
            // Each pass places one entry for good or swaps the bucket's entry with a deleted
            // one, which is placed at a later pass; a table has fewer entries than buckets.
            for _ in 0..=buckets {
                let Some(hash) = self.hash_at(index) else {
                    self.set_ctrl(index, EMPTY);
                    break;
                };
                let new_index = self.find_insert_slot(hash);
                let start = self.probe(hash).pos;
                let bucket_mask = self.bucket_mask;
                let probe_index = |pos: usize| (pos.wrapping_sub(start) & bucket_mask) / WIDTH;
                if probe_index(index) == probe_index(new_index) {
                    self.set_ctrl(index, h2(hash));
                    break;
                }
                let previous = self.ctrl_at(new_index);
                self.set_ctrl(new_index, h2(hash));
                self.slots.swap(index, new_index);
                if previous == EMPTY {
                    self.set_ctrl(index, EMPTY);
                    break;
                }
            }
        }
        self.growth_left = bucket_mask_to_capacity(self.bucket_mask).saturating_sub(self.items);
    }

    /// Move every entry into a new table with room for `capacity` entries, taken in the order
    /// of their buckets. The new table is allocated first, through `try_reserve`: a refusal
    /// leaves the map as it was.
    fn resize(&mut self, capacity: usize) -> Result<(), OverBudget> {
        let buckets = if capacity < 8 {
            Self::buckets_for(capacity)
        } else {
            (capacity.checked_mul(8).ok_or(OverBudget)? / 7)
                .checked_next_power_of_two()
                .ok_or(OverBudget)?
        };
        let mut ctrl = Vec::new();
        let ctrl_bytes = buckets.checked_add(WIDTH).ok_or(OverBudget)?;
        #[expect(
            clippy::disallowed_methods,
            reason = "the control bytes of the new table, charged by the map's owner by `KeyedMap::table_bytes_for` before it grows"
        )]
        ctrl.try_reserve_exact(ctrl_bytes).map_err(|_| OverBudget)?;
        ctrl.resize(ctrl_bytes, EMPTY);
        let mut slots = Vec::new();
        #[expect(
            clippy::disallowed_methods,
            reason = "the buckets of the new table, charged by the map's owner by `KeyedMap::table_bytes_for` before it grows"
        )]
        slots.try_reserve_exact(buckets).map_err(|_| OverBudget)?;
        slots.resize_with(buckets, || None);
        let old = core::mem::replace(&mut self.slots, slots);
        self.ctrl = ctrl;
        self.bucket_mask = buckets.saturating_sub(1);
        for (key, value) in old.into_iter().flatten() {
            let hash = self.hasher.hash_one(&key);
            let index = self.find_insert_slot(hash);
            if let Some(slot) = self.slots.get_mut(index) {
                *slot = Some((key, value));
            }
            self.set_ctrl(index, h2(hash));
        }
        self.growth_left = bucket_mask_to_capacity(self.bucket_mask).saturating_sub(self.items);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use alloc::format;
    use alloc::vec::Vec;
    use core::hash::BuildHasher;
    use std::collections::HashMap;

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
        assert_eq!(
            map.get(&"key 18".encode_utf16().collect::<Vec<u16>>()),
            Some(&18)
        );
        assert_eq!(map.values().count(), 500);
        assert!(map.values().all(|value| value % 2 == 0));
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

    /// A step of the comparison with the standard library's map.
    #[derive(Clone, Copy, Debug)]
    enum Step {
        Insert(u32),
        Remove(u32),
        Reserve(usize),
        RetainOdd,
        Clear,
    }

    /// The map answers as the standard library's map with the same hasher does, over 20,000
    /// steps of insertions, replacements, removals, reservations, `retain` and `clear` drawn
    /// from a fixed generator over a few hundred keys: every answer, the entries, and, on
    /// x86-64, whose standard library map has groups of 16 as this one, the room after each
    /// step, which is what the charges read ([`KeyedMap::spare`], [`KeyedMap::table_bytes`]).
    #[test]
    fn the_map_answers_as_the_standard_library_map_did() {
        let hasher = SipBuildHasher::new(&[7; 16]);
        let mut ours: KeyedMap<u32, u64> = KeyedMap::with_hasher(hasher);
        let mut std_map: HashMap<u32, u64, SipBuildHasher> = HashMap::with_hasher(hasher);
        let mut state = 0x2545_f491_4f6c_dd1d_u64;
        let mut draw = |bound: u64| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state % bound
        };
        for step_number in 0..20_000u64 {
            let range = if step_number % 5_000 < 2_500 { 64 } else { 600 };
            let step = match draw(100) {
                0..=54 => Step::Insert(u32::try_from(draw(range)).unwrap()),
                55..=94 => Step::Remove(u32::try_from(draw(range)).unwrap()),
                95..=97 => Step::Reserve(usize::try_from(draw(40)).unwrap()),
                98 => Step::RetainOdd,
                _ => Step::Clear,
            };
            match step {
                Step::Insert(key) => {
                    assert_eq!(
                        ours.insert(key, step_number),
                        Ok(std_map.insert(key, step_number)),
                        "{step:?}"
                    );
                }
                Step::Remove(key) => assert_eq!(ours.remove(&key), std_map.remove(&key)),
                Step::Reserve(additional) => {
                    ours.reserve(additional).unwrap();
                    std_map.try_reserve(additional).unwrap();
                }
                Step::RetainOdd => {
                    ours.retain(|key, _| key % 2 == 1);
                    std_map.retain(|key, _| key % 2 == 1);
                }
                Step::Clear => {
                    ours.clear();
                    std_map.clear();
                }
            }
            assert_eq!(ours.len(), std_map.len(), "{step_number} {step:?}");
            #[cfg(target_arch = "x86_64")]
            assert_eq!(
                ours.spare(),
                std_map.capacity() - std_map.len(),
                "{step_number} {step:?}"
            );
        }
        for key in 0..600 {
            assert_eq!(ours.get(&key), std_map.get(&key), "{key}");
        }
        let mut values: Vec<u64> = ours.values().copied().collect();
        let mut expected: Vec<u64> = std_map.values().copied().collect();
        values.sort_unstable();
        expected.sort_unstable();
        assert_eq!(values, expected);
    }
}
