//! Strings and atoms (`docs/design/engine.md`, decision 5; ECMA-262 6.1.4,
//! `#sec-ecmascript-language-types-string-type`: "a sequence of zero or more 16-bit unsigned
//! integer values"). A String is stored in one of two forms chosen when it is made: Latin-1, one
//! byte per code unit when every unit is below 0x100, or 16-bit units. Both forms read as the
//! same code units.
//!
//! Property keys are interned as atoms: one String per sequence of code units, found through
//! the keyed map. The atom table holds its entries weakly: a sweep that frees an atom's String
//! drops its entry, so a script that makes keys without end cannot grow the table past what is
//! alive. (Integer keys for array indices are `story:object-model-and-realms`.)
//!
//! Kind site of the collector: [`HeapString::trace`] reports nothing, since a String holds no
//! handle.

use super::Heap;
use super::accounting::OverBudget;
use super::arena::{Arena, Handle};
use super::collect::Mark;
use super::map::KeyedMap;
use crate::value::StringRef;

/// A String record.
pub(crate) enum HeapString {
    /// Every code unit is below 0x100 and is stored as one byte.
    Latin1(Box<[u8]>),
    /// The code units as they are.
    Utf16(Box<[u16]>),
}

impl HeapString {
    /// Kind site of the collector: a String holds no handle.
    pub(crate) fn trace(&self, mark: &mut Mark) {
        mark.count("heap/string.rs HeapString::trace");
    }

    /// The code units, exactly (Web IDL's `DOMString`).
    pub(crate) fn to_utf16(&self) -> Vec<u16> {
        match self {
            HeapString::Latin1(bytes) => bytes.iter().map(|&b| u16::from(b)).collect(),
            HeapString::Utf16(units) => units.to_vec(),
        }
    }

    /// The code points, each lone surrogate replaced by U+FFFD (Web IDL's `USVString`).
    pub(crate) fn to_rust_string_lossy(&self) -> String {
        match self {
            HeapString::Latin1(bytes) => bytes.iter().map(|&b| char::from(b)).collect(),
            HeapString::Utf16(units) => char::decode_utf16(units.iter().copied())
                .map(|unit| unit.unwrap_or(char::REPLACEMENT_CHARACTER))
                .collect(),
        }
    }

    /// Whether the record holds exactly `units`.
    fn equals(&self, units: &[u16]) -> bool {
        match self {
            HeapString::Latin1(bytes) => {
                bytes.len() == units.len()
                    && bytes.iter().zip(units).all(|(&b, &u)| u16::from(b) == u)
            }
            HeapString::Utf16(own) => **own == *units,
        }
    }

    /// An empty String, for the tests of the collector.
    #[cfg(test)]
    pub(crate) fn for_test() -> HeapString {
        HeapString::Latin1(Box::new([]))
    }
}

/// The bytes a String of `units` code units takes in its form.
fn owned_bytes(units: usize, latin1: bool) -> u64 {
    let units = u64::try_from(units).unwrap_or(u64::MAX);
    if latin1 {
        units
    } else {
        units.saturating_mul(2)
    }
}

/// A vector of exactly `len` elements' room, or `OverBudget` when the allocator refuses.
fn room<T>(len: usize) -> Result<Vec<T>, OverBudget> {
    let mut vec = Vec::new();
    vec.try_reserve_exact(len).map_err(|_| OverBudget)?;
    Ok(vec)
}

impl Heap {
    /// A new String of the UTF-16 encoding of `text`. Its size is charged before its units are
    /// copied.
    pub(crate) fn new_string_from_str(&mut self, text: &str) -> Result<StringRef, OverBudget> {
        let latin1 = text.chars().all(|c| u32::from(c) < 0x100);
        let len = if latin1 {
            text.chars().count()
        } else {
            text.encode_utf16().count()
        };
        let bytes = owned_bytes(len, latin1);
        self.accounting.check(bytes)?;
        let built = if latin1 {
            room(len).map(|mut out: Vec<u8>| {
                out.extend(text.chars().filter_map(|c| u8::try_from(c).ok()));
                HeapString::Latin1(out.into_boxed_slice())
            })
        } else {
            room(len).map(|mut out: Vec<u16>| {
                out.extend(text.encode_utf16());
                HeapString::Utf16(out.into_boxed_slice())
            })
        };
        self.new_string_record(built?, bytes)
    }

    /// A new String of exactly `units`, lone surrogates included.
    pub(crate) fn new_string_from_units(&mut self, units: &[u16]) -> Result<StringRef, OverBudget> {
        let latin1 = units.iter().all(|&u| u < 0x100);
        let bytes = owned_bytes(units.len(), latin1);
        self.accounting.check(bytes)?;
        let built = if latin1 {
            room(units.len()).map(|mut out: Vec<u8>| {
                out.extend(units.iter().filter_map(|&u| u8::try_from(u).ok()));
                HeapString::Latin1(out.into_boxed_slice())
            })
        } else {
            room(units.len()).map(|mut out: Vec<u16>| {
                out.extend_from_slice(units);
                HeapString::Utf16(out.into_boxed_slice())
            })
        };
        self.new_string_record(built?, bytes)
    }

    /// The atom of `units`: the String every property key of these code units uses. The first
    /// call makes it and charges its String and its entry's key; when the table has no room,
    /// the table's growth is charged first and stays charged.
    pub(crate) fn intern(&mut self, units: &[u16]) -> Result<StringRef, OverBudget> {
        if let Some(&(handle, _)) = self.atoms.map.get(units)
            && self
                .strings
                .get(handle)
                .is_ok_and(|string| string.equals(units))
        {
            return Ok(StringRef { handle });
        }
        let entry_bytes = owned_bytes(units.len(), false);
        let growth = self.atoms.growth();
        let growth_bytes = growth.map_or(0, |to| self.atoms.growth_bytes(to));
        self.accounting
            .charge(growth_bytes.saturating_add(entry_bytes))?;
        if let Some(to) = growth
            && self.atoms.grow(to).is_err()
        {
            self.accounting
                .credit(growth_bytes.saturating_add(entry_bytes));
            return Err(OverBudget);
        }
        let made = room(units.len()).and_then(|mut key: Vec<u16>| {
            key.extend_from_slice(units);
            let string = self.new_string_from_units(units)?;
            let replaced = self
                .atoms
                .map
                .insert(key.into_boxed_slice(), (string.handle, entry_bytes))?;
            // An entry whose String is gone is replaced, and what it was charged given back.
            if let Some((_, old_bytes)) = replaced {
                self.accounting.credit(old_bytes);
            }
            Ok(string)
        });
        if made.is_err() {
            self.accounting.credit(entry_bytes);
        }
        made
    }
}

/// The fewest entries the atom table grows to.
const MIN_ATOMS: usize = 16;

/// The atom table: code units to the String that is their atom, with the bytes the entry's
/// key owns. The table is charged by capacity, like an arena (`engine.md`, "Memory budget"):
/// before it grows its room doubles and the table's growth is charged
/// ([`KeyedMap::table_bytes_for`]); those bytes stay charged while the agent lives. It is not
/// shrunk on sweep: shrinking a `HashMap` moves it to a new table, an allocation inside a
/// collection.
pub(crate) struct Atoms {
    map: KeyedMap<Box<[u16]>, (Handle, u64)>,
    /// The entries the table was charged room for.
    room: usize,
}

impl Atoms {
    /// An empty table keyed by `hash_key`.
    pub(crate) fn new(hash_key: &[u8; 16]) -> Atoms {
        Atoms {
            map: KeyedMap::new(hash_key),
            room: 0,
        }
    }

    /// The room the table grows to before it takes another entry, or `None` while it has room.
    fn growth(&self) -> Option<usize> {
        if self.map.len() < self.room {
            return None;
        }
        Some(self.room.saturating_mul(2).max(MIN_ATOMS))
    }

    /// The bytes of the table with room for `to` entries less those of the table now.
    fn growth_bytes(&self, to: usize) -> u64 {
        type Table = KeyedMap<Box<[u16]>, (Handle, u64)>;
        Table::table_bytes_for(to).saturating_sub(Table::table_bytes_for(self.room))
    }

    /// Reserve room for `to` entries (its bytes charged before, [`Atoms::growth_bytes`]).
    fn grow(&mut self, to: usize) -> Result<(), OverBudget> {
        self.map.reserve(to.saturating_sub(self.map.len()))?;
        self.room = to;
        Ok(())
    }

    /// The bytes of the table's buckets that hold no entry, which the live reading leaves out:
    /// the table charged less a bucket, its entry and its control byte, per entry.
    pub(crate) fn room_bytes(&self) -> u64 {
        type Entry = (Box<[u16]>, (Handle, u64));
        let table = KeyedMap::<Box<[u16]>, (Handle, u64)>::table_bytes_for(self.room);
        let entry = u64::try_from(size_of::<Entry>().saturating_add(1)).unwrap_or(u64::MAX);
        let entries = u64::try_from(self.map.len()).unwrap_or(u64::MAX);
        table.saturating_sub(entries.saturating_mul(entry))
    }

    /// The hash the table gives `units`.
    #[cfg(test)]
    pub(crate) fn hash_of(&self, units: &[u16]) -> u64 {
        self.map.hash_of(units)
    }

    /// Drop every entry whose String the sweep freed, and answer the bytes their keys owned;
    /// the table keeps its room. Pushes nothing and allocates nothing.
    pub(crate) fn sweep(&mut self, strings: &Arena<HeapString>) -> u64 {
        let mut credited: u64 = 0;
        self.map.retain(|_, &mut (handle, bytes)| {
            let alive = strings.get(handle).is_ok();
            if !alive {
                credited = credited.saturating_add(bytes);
            }
            alive
        });
        credited
    }

    /// The number of atoms.
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.map.len()
    }
}

#[cfg(test)]
mod tests {
    use super::HeapString;
    use crate::{Agent, Limits, Outcome, Seeds, Value};

    use crate::host::hooks::test_host::NoHooks;

    fn agent() -> Agent<NoHooks> {
        let limits = Limits {
            steps_per_entry: 1_000,
            memory_bytes: 1 << 20,
            frame_depth: 100,
            reentry_depth: 10,
            parser_nesting: 100,
            string_length: 1 << 20,
            bigint_bits: 1 << 10,
        };
        Agent::new(
            NoHooks,
            limits,
            Seeds {
                hash_key: [1; 16],
                random_seed: [2; 32],
            },
        )
        .unwrap()
    }

    #[test]
    fn the_form_is_chosen_by_the_units_and_reads_the_same() {
        let mut agent = agent();
        let heap = &mut agent.heap;
        let latin1 = heap.new_string_from_str("caf\u{E9}").unwrap();
        let wide = heap.new_string_from_str("\u{20AC}").unwrap();
        let units = heap.new_string_from_units(&[0x61, 0xFF]).unwrap();
        assert!(matches!(
            heap.strings.get(latin1.handle),
            Ok(HeapString::Latin1(_))
        ));
        assert!(matches!(
            heap.strings.get(wide.handle),
            Ok(HeapString::Utf16(_))
        ));
        assert!(matches!(
            heap.strings.get(units.handle),
            Ok(HeapString::Latin1(_))
        ));
        assert_eq!(
            heap.strings.get(latin1.handle).unwrap().to_utf16(),
            [0x63, 0x61, 0x66, 0xE9]
        );
        assert_eq!(
            heap.strings
                .get(units.handle)
                .unwrap()
                .to_rust_string_lossy(),
            "a\u{FF}"
        );
    }

    #[test]
    fn an_atom_is_one_string_per_units_and_is_held_weakly() {
        let mut agent = agent();
        let key: Vec<u16> = "length".encode_utf16().collect();
        let empty = agent.heap.bytes();
        let first = agent.heap.intern(&key).unwrap();
        // The root is made before any safe point: an atom held only weakly is gone at the next
        // one, which the feature `collect-every-safe-point` makes every entry.
        let root = agent.heap.roots.add(Value::String(first));
        let second = agent.heap.intern(&key).unwrap();
        assert_eq!(first, second);
        let other = agent.heap.intern(&[0xD800]).unwrap();
        assert_ne!(first, other);
        assert_eq!(agent.heap.atoms.len(), 2);

        // Held by a root, the atom survives; the other one, which nothing holds, is dropped.
        // Once nothing holds the first either, the sweep drops its entry and credits
        // everything it charged.
        assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
        assert_eq!(agent.heap.atoms.len(), 1);
        assert_eq!(agent.heap.intern(&key).unwrap(), first);
        drop(root);
        assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
        assert_eq!(agent.heap.atoms.len(), 0);
        assert_eq!(agent.heap.bytes(), empty);
    }

    /// The atom table is charged by its capacity: after 10,000 atoms are interned, and after a
    /// collection that frees them all, the accounting holds at least the bytes of the table's
    /// buckets (`KeyedMap::table_bytes`), which keeps its room.
    #[test]
    fn the_atom_table_is_charged_by_its_capacity() {
        let mut limits = crate::host::hooks::test_host::limits();
        limits.memory_bytes = 1 << 26;
        let mut agent = crate::host::hooks::test_host::agent_with(limits);
        for i in 0..10_000 {
            let key: Vec<u16> = format!("atom {i}").encode_utf16().collect();
            agent.heap.intern(&key).unwrap();
        }
        assert_eq!(agent.heap.atoms.len(), 10_000);
        let table = agent.heap.atoms.map.table_bytes();
        let used = agent.heap.accounting.used();
        assert!(
            used >= table,
            "before the collection: used {used}, table {table}"
        );

        assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
        assert_eq!(agent.heap.atoms.len(), 0);
        let table = agent.heap.atoms.map.table_bytes();
        let used = agent.heap.accounting.used();
        assert!(
            used >= table,
            "after the collection: used {used}, table {table}"
        );
    }
}
