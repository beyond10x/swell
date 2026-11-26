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
//! Two Strings are compared, and a String is looked up in the atom table, in place: in either
//! form, unit by unit, with no copy (`engine.md`, "The step budget", "Strings"). A String's units
//! are copied into an atom only when the table has none of them.
//!
//! Kind site of the collector: [`HeapString::trace`] reports nothing, since a String holds no
//! handle.

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use core::borrow::Borrow;
use core::cmp::Ordering;
use core::hash::{Hash, Hasher};

use super::Heap;
use super::accounting::{Accounting, OverBudget};
use super::arena::{Access, Arena, Handle};
use super::collect::Mark;
use super::map::KeyedMap;
use crate::value::StringRef;

/// The code units of a String in the form they are stored in, read in place.
#[derive(Clone, Copy)]
pub(crate) enum CodeUnits<'a> {
    /// Every code unit is below 0x100, one byte each.
    Latin1(&'a [u8]),
    /// The code units as they are.
    Utf16(&'a [u16]),
}

impl<'a> From<&'a [u16]> for CodeUnits<'a> {
    fn from(units: &'a [u16]) -> CodeUnits<'a> {
        CodeUnits::Utf16(units)
    }
}

impl CodeUnits<'_> {
    /// The number of code units.
    fn len(self) -> usize {
        match self {
            CodeUnits::Latin1(bytes) => bytes.len(),
            CodeUnits::Utf16(units) => units.len(),
        }
    }

    /// Whether both hold the same code units. Allocates nothing.
    fn equals(self, other: CodeUnits<'_>) -> bool {
        match (self, other) {
            (CodeUnits::Latin1(x), CodeUnits::Latin1(y)) => x == y,
            (CodeUnits::Utf16(x), CodeUnits::Utf16(y)) => x == y,
            (CodeUnits::Latin1(bytes), CodeUnits::Utf16(units))
            | (CodeUnits::Utf16(units), CodeUnits::Latin1(bytes)) => {
                bytes.len() == units.len()
                    && bytes.iter().zip(units).all(|(&b, &u)| u16::from(b) == u)
            }
        }
    }

    /// The order of the code units, compared one by one, a prefix before the longer sequence
    /// (`#sec-islessthan`, step 3). Allocates nothing.
    fn order(self, other: CodeUnits<'_>) -> Ordering {
        match (self, other) {
            (CodeUnits::Latin1(x), CodeUnits::Latin1(y)) => x.cmp(y),
            (CodeUnits::Utf16(x), CodeUnits::Utf16(y)) => x.cmp(y),
            (CodeUnits::Latin1(x), CodeUnits::Utf16(y)) => {
                x.iter().map(|&b| u16::from(b)).cmp(y.iter().copied())
            }
            (CodeUnits::Utf16(x), CodeUnits::Latin1(y)) => {
                x.iter().copied().cmp(y.iter().map(|&b| u16::from(b)))
            }
        }
    }

    /// Copy the first code units into `out`, as many as it holds, and answer how many.
    pub(crate) fn copy_to(self, out: &mut [u16]) -> usize {
        match self {
            CodeUnits::Latin1(bytes) => {
                let mut copied = 0;
                for (slot, &b) in out.iter_mut().zip(bytes) {
                    *slot = u16::from(b);
                    copied += 1;
                }
                copied
            }
            CodeUnits::Utf16(units) => {
                let mut copied = 0;
                for (slot, &u) in out.iter_mut().zip(units) {
                    *slot = u;
                    copied += 1;
                }
                copied
            }
        }
    }

    /// Append the code units to `out`, which has room for them.
    fn extend_into(self, out: &mut Vec<u16>) {
        match self {
            CodeUnits::Latin1(bytes) => out.extend(bytes.iter().map(|&b| u16::from(b))),
            CodeUnits::Utf16(units) => out.extend_from_slice(units),
        }
    }

    /// Feed `state` the number of code units, then each unit as two little-endian bytes, in
    /// pieces of at most [`HASH_PIECE`] units: the same bytes for either form.
    fn hash_into<H: Hasher>(self, state: &mut H) {
        state.write_usize(self.len());
        let mut buffer = [[0u8; 2]; HASH_PIECE];
        match self {
            CodeUnits::Latin1(bytes) => {
                for piece in bytes.chunks(HASH_PIECE) {
                    for (pair, &b) in buffer.iter_mut().zip(piece) {
                        *pair = [b, 0];
                    }
                    let filled = buffer.get(..piece.len()).unwrap_or(&[]);
                    state.write(filled.as_flattened());
                }
            }
            CodeUnits::Utf16(units) => {
                for piece in units.chunks(HASH_PIECE) {
                    for (pair, &u) in buffer.iter_mut().zip(piece) {
                        *pair = u.to_le_bytes();
                    }
                    let filled = buffer.get(..piece.len()).unwrap_or(&[]);
                    state.write(filled.as_flattened());
                }
            }
        }
    }
}

/// The code units hashed in one write.
const HASH_PIECE: usize = 32;

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

    /// The number of code units, read from the storage's length.
    pub(crate) fn len(&self) -> usize {
        match self {
            HeapString::Latin1(bytes) => bytes.len(),
            HeapString::Utf16(units) => units.len(),
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

    /// The code units, read in place.
    pub(crate) fn units(&self) -> CodeUnits<'_> {
        match self {
            HeapString::Latin1(bytes) => CodeUnits::Latin1(bytes),
            HeapString::Utf16(units) => CodeUnits::Utf16(units),
        }
    }

    /// Whether the record holds exactly `units`, compared in place: for the tests of this file;
    /// the engine compares two records with [`HeapString::equals_string`].
    #[cfg(test)]
    pub(crate) fn equals(&self, units: &[u16]) -> bool {
        self.units().equals(CodeUnits::Utf16(units))
    }

    /// Whether two records hold the same code units, compared in place, in either form.
    pub(crate) fn equals_string(&self, other: &HeapString) -> bool {
        self.units().equals(other.units())
    }

    /// The order of the record's code units against `units` (`#sec-islessthan`, step 3),
    /// compared in place.
    pub(crate) fn cmp_units(&self, units: &[u16]) -> Ordering {
        self.units().order(CodeUnits::Utf16(units))
    }

    /// The order of two records' code units (`#sec-islessthan`, step 3), compared in place, in
    /// either form.
    pub(crate) fn cmp_string(&self, other: &HeapString) -> Ordering {
        self.units().order(other.units())
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

    /// The atom of `units` if the table has one, or `None`: hashed and compared in place, in
    /// either form. Makes nothing, charges nothing and allocates nothing.
    pub(crate) fn atom_of<'a>(&self, units: impl Into<CodeUnits<'a>>) -> Option<StringRef> {
        let units = units.into();
        let handle = self.atoms.find(units)?;
        self.strings
            .get(handle)
            .is_ok_and(|string| string.units().equals(units))
            .then_some(StringRef { handle })
    }

    /// The atom of `units`: the String every property key of these code units uses. The first
    /// call makes it ([`Heap::insert_atom`]); a later one finds it ([`Heap::atom_of`]).
    pub(crate) fn intern(&mut self, units: &[u16]) -> Result<StringRef, OverBudget> {
        if let Some(atom) = self.atom_of(units) {
            return Ok(atom);
        }
        let key = new_atom_key(
            &mut self.accounting,
            &mut self.atoms,
            CodeUnits::Utf16(units),
        )?;
        self.insert_atom(key)
    }

    /// The atom of the code units of `string`, as [`Heap::intern`] answers it: looked up in
    /// place, and on a miss its units copied once, into the new entry's key, which the new atom
    /// is made from. `Err` for a stale handle.
    pub(crate) fn intern_string(
        &mut self,
        string: StringRef,
    ) -> Result<Result<StringRef, OverBudget>, Access> {
        let units = self.strings.get(string.handle)?.units();
        if let Some(atom) = self.atom_of(units) {
            return Ok(Ok(atom));
        }
        let key = match new_atom_key(&mut self.accounting, &mut self.atoms, units) {
            Ok(key) => key,
            Err(over) => return Ok(Err(over)),
        };
        Ok(self.insert_atom(key))
    }

    /// The insertion half of [`Heap::intern`]: the atom made from the key [`new_atom_key`]
    /// charged and filled, and the entry stored. Over the budget the key's charge is given
    /// back; the table's growth stays charged.
    fn insert_atom(
        &mut self,
        (key, entry_bytes): (Box<[u16]>, u64),
    ) -> Result<StringRef, OverBudget> {
        let made = match self.new_string_from_units(&key) {
            Ok(string) => self
                .atoms
                .map
                .insert(AtomKey(key), (string.handle, entry_bytes))
                .map(|replaced| {
                    // An entry whose String is gone is replaced, and what it was charged given
                    // back.
                    if let Some((_, old_bytes)) = replaced {
                        self.accounting.credit(old_bytes);
                    }
                    string
                }),
            Err(over) => Err(over),
        };
        if made.is_err() {
            self.accounting.credit(entry_bytes);
        }
        made
    }
}

/// The key of a new entry of the atom table for `units`, with the bytes it owns: when the
/// table has no room its growth is charged first and stays charged, then the key's bytes are
/// charged and the units copied into it. Over the budget nothing more is charged.
fn new_atom_key(
    accounting: &mut Accounting,
    atoms: &mut Atoms,
    units: CodeUnits<'_>,
) -> Result<(Box<[u16]>, u64), OverBudget> {
    let entry_bytes = owned_bytes(units.len(), false);
    let growth = atoms.growth();
    let growth_bytes = growth.map_or(0, |to| atoms.growth_bytes(to));
    accounting.charge(growth_bytes.saturating_add(entry_bytes))?;
    if let Some(to) = growth
        && atoms.grow(to).is_err()
    {
        accounting.credit(growth_bytes.saturating_add(entry_bytes));
        return Err(OverBudget);
    }
    match room(units.len()) {
        Ok(mut key) => {
            units.extend_into(&mut key);
            Ok((key.into_boxed_slice(), entry_bytes))
        }
        Err(over) => {
            accounting.credit(entry_bytes);
            Err(over)
        }
    }
}

/// The key of an entry of the atom table: the atom's code units, which the entry owns.
struct AtomKey(Box<[u16]>);

/// Code units the atom table hashes and compares, whatever their form: the key of an entry,
/// and the units a lookup asks for, read in place ([`CodeUnits::hash_into`] gives both forms
/// the same hash).
trait AtomText {
    fn code_units(&self) -> CodeUnits<'_>;
}

impl AtomText for AtomKey {
    fn code_units(&self) -> CodeUnits<'_> {
        CodeUnits::Utf16(&self.0)
    }
}

impl AtomText for CodeUnits<'_> {
    fn code_units(&self) -> CodeUnits<'_> {
        *self
    }
}

impl Hash for dyn AtomText + '_ {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.code_units().hash_into(state);
    }
}

impl PartialEq for dyn AtomText + '_ {
    fn eq(&self, other: &Self) -> bool {
        self.code_units().equals(other.code_units())
    }
}

impl Eq for dyn AtomText + '_ {}

impl Hash for AtomKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.code_units().hash_into(state);
    }
}

impl PartialEq for AtomKey {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for AtomKey {}

impl<'a> Borrow<dyn AtomText + 'a> for AtomKey {
    fn borrow(&self) -> &(dyn AtomText + 'a) {
        self
    }
}

/// The fewest entries the atom table grows to.
const MIN_ATOMS: usize = 16;

/// The map of the atom table.
type Table = KeyedMap<AtomKey, (Handle, u64)>;

/// The atom table: code units to the String that is their atom, with the bytes the entry's
/// key owns. The table is charged by capacity, like an arena (`engine.md`, "Memory budget"):
/// before it grows its room doubles and the table's growth is charged
/// ([`KeyedMap::table_bytes_for`]); those bytes stay charged while the agent lives. It is not
/// shrunk on sweep: shrinking a `KeyedMap` would move it to a new table, an allocation inside a
/// collection.
pub(crate) struct Atoms {
    map: Table,
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
        if self.map.len() < self.room && !self.rehash_grows() {
            return None;
        }
        Some(self.room.saturating_mul(2).max(MIN_ATOMS))
    }

    /// Whether the next insertion would move the table to a larger one by itself: a sweep
    /// leaves buckets marked deleted, which the map counts against its room
    /// ([`KeyedMap::spare`]), so it can grow before it holds the entries it was charged for.
    /// With no spare room it rehashes in place, without an allocation, while its entries and
    /// the new one fill at most half of what the table holds ([`KeyedMap::full_capacity_for`]),
    /// and otherwise grows, to the table of twice the room, which is then charged first.
    fn rehash_grows(&self) -> bool {
        self.map.spare() == 0 && self.map.len() >= Table::full_capacity_for(self.room) / 2
    }

    /// The bytes of the table with room for `to` entries less those of the table now.
    fn growth_bytes(&self, to: usize) -> u64 {
        Table::table_bytes_for(to).saturating_sub(Table::table_bytes_for(self.room))
    }

    /// The counting build: the bytes the table's room was charged.
    #[cfg(test)]
    pub(crate) fn room_charge(&self) -> u64 {
        Table::table_bytes_for(self.room)
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
        type Entry = (AtomKey, (Handle, u64));
        let table = Table::table_bytes_for(self.room);
        let entry = u64::try_from(size_of::<Entry>().saturating_add(1)).unwrap_or(u64::MAX);
        let entries = u64::try_from(self.map.len()).unwrap_or(u64::MAX);
        table.saturating_sub(entries.saturating_mul(entry))
    }

    /// The entry of `units`: the handle of its String, found in place.
    fn find(&self, units: CodeUnits<'_>) -> Option<Handle> {
        let text: &dyn AtomText = &units;
        self.map.get(text).map(|&(handle, _)| handle)
    }

    /// The hash the table gives `units`.
    #[cfg(test)]
    pub(crate) fn hash_of(&self, units: &[u16]) -> u64 {
        let text: &dyn AtomText = &CodeUnits::Utf16(units);
        self.map.hash_of(text)
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
    use alloc::format;
    use alloc::vec::Vec;

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

    /// An atom is one String per sequence of code units, and the table holds it weakly: once
    /// nothing holds it, a collection drops its entry and the live reading reads again what it
    /// read before the atoms were made. The charged reading keeps the capacity charge: the
    /// arenas' slots with their workspace and the atom table's room (`Heap::capacity_charge`).
    #[test]
    fn an_atom_is_one_string_per_units_and_is_held_weakly() {
        let mut agent = agent();
        let key: Vec<u16> = "length".encode_utf16().collect();
        let empty = agent.heap.live_bytes();
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
        // Once nothing holds the first either, the sweep drops its entry and credits what its
        // String and its key owned.
        assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
        assert_eq!(agent.heap.atoms.len(), 1);
        assert_eq!(agent.heap.intern(&key).unwrap(), first);
        drop(root);
        assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
        assert_eq!(agent.heap.atoms.len(), 0);
        assert_eq!(agent.heap.live_bytes(), empty);
        assert_eq!(agent.heap.bytes(), agent.heap.capacity_charge());
    }

    /// The atom table never grows past the room it was charged for. A collection that drops
    /// entries leaves buckets marked deleted, which the map counts against its room, so
    /// with fewer entries than the room charged an insertion can find no spare room. Here the
    /// table is charged room for 32 entries (64 buckets); 28 atoms whose hash starts at bucket
    /// 0 fill buckets 0 to 27, and their collection leaves 28 deleted buckets; 31 atoms whose
    /// hash starts at bucket 32 then take the 28 buckets left before the map's room is gone, at
    /// 28 entries, more than half of the 56 a table of 64 buckets holds. The next insertion
    /// grows the table, to 128 buckets, and that growth is charged first. After every insertion
    /// the bytes of the table the map has (`KeyedMap::table_bytes`, which reads a table with
    /// deleted buckets as smaller) stay within those its room was charged.
    #[test]
    fn the_atom_table_never_grows_past_its_charged_room() {
        let mut agent = agent();
        let mut next = 0u32;
        let mut keys = |start: u64, count: usize, agent: &Agent<NoHooks>| {
            let mut found = Vec::new();
            while found.len() < count {
                let key: Vec<u16> = format!("atom {next}").encode_utf16().collect();
                next += 1;
                if agent.heap.atoms.hash_of(&key) & 63 == start {
                    found.push(key);
                }
            }
            found
        };
        let first = keys(0, 28, &agent);
        let second = keys(32, 31, &agent);
        let check = |agent: &mut Agent<NoHooks>, key: &[u16], step: &str| {
            agent.heap.intern(key).unwrap();
            let atoms = &agent.heap.atoms;
            let table = atoms.map.table_bytes();
            let charged = atoms.room_charge();
            assert!(
                table <= charged,
                "{step}, {} atoms: the table takes {table} bytes, its room was charged {charged}",
                atoms.len()
            );
        };
        for key in &first {
            check(&mut agent, key, "first round");
        }
        assert_eq!(agent.heap.atoms.room, 32);
        assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
        assert_eq!(agent.heap.atoms.len(), 0);
        for key in &second {
            check(&mut agent, key, "second round");
        }
        assert_eq!(agent.heap.atoms.len(), 31);
        assert!(agent.heap.bytes() >= agent.heap.capacity_charge());
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

    /// A generator of the tests, seeded in the test: xorshift64.
    struct Generator(u64);

    impl Generator {
        fn below(&mut self, bound: usize) -> usize {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            (self.0 % bound as u64) as usize
        }
    }

    /// A record of `units` in the storage kind asked: Latin-1 (every unit is below 0x100) or
    /// 16-bit units.
    fn record(units: &[u16], latin1: bool) -> HeapString {
        if latin1 {
            let bytes: Vec<u8> = units.iter().map(|&u| u8::try_from(u).unwrap()).collect();
            HeapString::Latin1(bytes.into_boxed_slice())
        } else {
            HeapString::Utf16(units.to_vec().into_boxed_slice())
        }
    }

    /// For each of the four pairs of storage kinds (Latin-1 or 16-bit units on each side), 500
    /// pairs of records: equal, a prefix, a difference in the first, a middle or the last unit,
    /// and a unit above 0xFF on a side of 16-bit units. The units below 0x100 include 0xFF, the
    /// largest a Latin-1 record holds.
    fn pairs_of_every_storage_kind() -> Vec<(HeapString, HeapString)> {
        const NARROW: [u16; 4] = [0x61, 0x62, 0x63, 0xFF];
        let mut generator = Generator(0x7377_656c_6c5f_6432);
        let mut pairs = Vec::new();
        for (x_latin1, y_latin1) in [(true, true), (true, false), (false, true), (false, false)] {
            for i in 0..500 {
                let len = generator.below(41);
                let mut x: Vec<u16> = (0..len).map(|_| NARROW[generator.below(4)]).collect();
                let mut y = x.clone();
                let other = |unit: u16, generator: &mut Generator| {
                    let mut next = NARROW[generator.below(4)];
                    if next == unit {
                        next = if unit == 0x61 { 0x62 } else { 0x61 };
                    }
                    next
                };
                let at = match i % 6 {
                    2 => (len > 0).then_some(0),
                    3 => (len > 0).then_some(len / 2),
                    4 | 5 => len.checked_sub(1),
                    _ => None,
                };
                match (i % 6, at) {
                    (1, _) => y.truncate(generator.below(len + 1)),
                    (5, Some(at)) if !y_latin1 => y[at] = 0x100 + NARROW[generator.below(4)],
                    (5, Some(at)) if !x_latin1 => x[at] = 0x100 + NARROW[generator.below(4)],
                    (2..=5, Some(at)) => y[at] = other(y[at], &mut generator),
                    _ => {}
                }
                let (x, y) = if generator.below(2) == 0 && x_latin1 == y_latin1 {
                    (y, x)
                } else {
                    (x, y)
                };
                pairs.push((record(&x, x_latin1), record(&y, y_latin1)));
            }
        }
        pairs
    }

    /// The equality of two records in place, and of a record with a slice of units, answers
    /// `==` of the two `to_utf16` vectors, for every pair of storage kinds.
    #[test]
    fn equality_in_place_matches_to_utf16_for_every_pair_of_storage_kinds() {
        let pairs = pairs_of_every_storage_kind();
        assert_eq!(pairs.len(), 2_000);
        let mut equal = 0;
        for (x, y) in &pairs {
            let (xs, ys) = (x.to_utf16(), y.to_utf16());
            assert_eq!(x.equals_string(y), xs == ys, "{xs:x?} and {ys:x?}");
            assert_eq!(x.equals(&ys), xs == ys, "{xs:x?} and the units {ys:x?}");
            equal += usize::from(xs == ys);
        }
        assert!(equal > 0 && equal < pairs.len(), "{equal} pairs equal");
    }

    /// The order of two records in place, and of a record and a slice of units, answers `cmp`
    /// of the two `to_utf16` vectors, for every pair of storage kinds.
    #[test]
    fn order_in_place_matches_to_utf16_for_every_pair_of_storage_kinds() {
        let pairs = pairs_of_every_storage_kind();
        let mut seen = [0usize; 3];
        for (x, y) in &pairs {
            let (xs, ys) = (x.to_utf16(), y.to_utf16());
            let ordering = xs.cmp(&ys);
            assert_eq!(x.cmp_string(y), ordering, "{xs:x?} and {ys:x?}");
            assert_eq!(x.cmp_units(&ys), ordering, "{xs:x?} and the units {ys:x?}");
            seen[usize::try_from(ordering as i8 + 1).unwrap()] += 1;
        }
        assert!(seen.iter().all(|&count| count > 0), "{seen:?}");
    }

    /// `Heap::atom_of` looks an atom up and makes nothing: for 1,000 slices of units no atom
    /// has it answers none, and the bytes charged and the table's length stay as they were; for
    /// interned units it answers their atom, from the slice and from a Latin-1 record of the
    /// same units.
    #[test]
    fn atom_of_makes_nothing() {
        let mut agent = agent();
        let interned: Vec<u16> = "interned".encode_utf16().collect();
        let atom = agent.heap.intern(&interned).unwrap();
        let used = agent.heap.accounting.used();
        let atoms = agent.heap.atoms.len();
        for i in 0..1_000 {
            let units: Vec<u16> = format!("absent {i}").encode_utf16().collect();
            assert_eq!(agent.heap.atom_of(&units[..]), None, "absent {i}");
        }
        assert_eq!(agent.heap.accounting.used(), used);
        assert_eq!(agent.heap.atoms.len(), atoms);
        assert_eq!(agent.heap.atom_of(&interned[..]), Some(atom));
        let same = agent.heap.new_string_from_str("interned").unwrap();
        let record = agent.heap.strings.get(same.handle).unwrap();
        assert!(matches!(record, HeapString::Latin1(_)));
        assert_eq!(agent.heap.atom_of(record.units()), Some(atom));
    }
}
