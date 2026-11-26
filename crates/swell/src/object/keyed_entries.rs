//! The ordered entry table of `Map` and `Set` (`story:keyed-entry-table`): the List of
//! [[MapData]] and [[SetData]] (`#sec-map-objects`, `#sec-set-objects`), whose elements are
//! records of a key and a value in insertion order, a deleted one `empty`.
//!
//! An [`EntryTable`] keeps its entries in a vector in insertion order; a deletion leaves a hole
//! at the entry's position, so the others keep their positions. Above [`SMALL`] (the room of
//! the vector, the bound of `object/property.rs` for the same reason) it keeps an index map
//! (`heap/map.rs`, made with `Heap::hasher`, keyed by `hash_key` alone, HOST-145) from the
//! canonical key to the position, so a lookup is one hash of one word; below it a lookup scans
//! at most `SMALL` entries, which the operation's one step covers. No order the table answers
//! comes from the map: every order is the vector's.
//!
//! The canonical key ([`EntryKey`]) is SameValueZero (`#sec-samevaluezero`) after
//! CanonicalizeKeyedCollectionKey (`#sec-canonicalizekeyedcollectionkey`, `-0` becomes `+0`),
//! as one word: `undefined`, `null` and a Boolean by their tag, a Number by its bits with every
//! NaN one NaN, a Symbol or an object by its handle, a String by the handle of its atom. A
//! String key is interned once, at its insertion (`Heap::intern_string`), so two Strings with
//! the same code units are one atom; a lookup or a deletion finds the atom with
//! `Heap::atom_of`, which makes nothing, and a String with no atom is in no table. Every
//! String, `"0"` included, is an atom here: a collection keeps `"0"` and `0` apart, which
//! `key_from_string` (an array index for `"0"`) would not. The atom table holds an atom weakly;
//! the table's trace lists the atom of each live key, so it stays while its entry does.
//!
//! A cursor ([`CursorId`]) is a position registered with the table, the index of the next
//! entry to visit after any operation (`#sec-map.prototype.foreach`, note 1;
//! `#sec-%mapiteratorprototype%.next`): an entry added before a cursor reaches it is visited,
//! a deleted one is not. A compaction moves every cursor down by the holes before it; `clear`
//! sets every cursor to the end of the empty storage, so a cursor that runs on after it visits
//! only entries added after it. A cursor is closed by its walk ([`EntryTable::close_cursor`]:
//! an iterator that answers done, every exit of a `forEach`), or, for an iterator dropped
//! unfinished, by the collector ([`EntryTable::release_cursors`]), which drops the cursors whose
//! owner it did not mark and keeps those with no owner (the walks in Rust that hold one).
//!
//! Bounds (`docs/design/engine.md`, decisions 3 and 4). Every operation charges its steps
//! before its work through `Agent::charge_steps`: one for `find`, `insert` and `delete`, and a
//! String key's code units past `FREE_CODE_UNITS` (`charge_code_units`); a compaction one per
//! entry it moves (with an index map, every live entry, which it numbers again) and one per
//! cursor it moves; `clear` one per 64 entries and one per open cursor; `open_cursor` one;
//! `next_live` one per hole it passes. The entry vector, the index map and the cursor list grow
//! by doubling, each growth charged before it with `Agent::charge_bytes`; over the budget the
//! run ends in `Terminated(MemoryBudget)` and the table is as it was. The table keeps what it
//! was charged (the entries, the map and the cursor list each in a count of its own), credits a
//! part when it gives that storage back (`clear`, the last cursor closed), and its drop queues
//! what the counts still hold for the agent to credit after the sweep, as `Grown` does
//! (`object/mod.rs`). The trace allocates nothing and visits each entry once. No operation
//! calls script (SameValueZero runs no code).

#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "story:map-and-set builds Map, Set and their iterators on the table"
    )
)]

use alloc::boxed::Box;
use alloc::rc::Rc;
use alloc::vec::Vec;
use core::cell::Cell;
use core::hash::{Hash, Hasher};

use crate::heap::arena::Handle;
use crate::heap::collect::Mark;
use crate::heap::map::KeyedMap;
use crate::host::agent::Agent;
use crate::host::{Completion, Host, Termination};
use crate::object::property::SMALL;
use crate::value::{ObjectRef, StringRef, SymbolRef, Value};

/// The bits of the one NaN of the keys: every NaN is this key.
const CANONICAL_NAN: u64 = 0x7ff8_0000_0000_0000;

/// The entries `clear` empties per step.
const CLEARED_PER_STEP: usize = 64;

/// A key after CanonicalizeKeyedCollectionKey, compared by SameValueZero: two keys are the same
/// key exactly when they are equal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EntryKey {
    Undefined,
    Null,
    Boolean(bool),
    /// The bits of the Number: NaN as [`CANONICAL_NAN`], `-0` as `+0`.
    Number(u64),
    /// The atom of the String.
    String(StringRef),
    Symbol(SymbolRef),
    Object(ObjectRef),
}

/// The hash of a key is one 64-bit word through the keyed SipHash of the map (HOST-145): a
/// Number's bits, or a kind in the top three bits with the slot and the low 29 bits of the
/// generation of the handle. Keys that are equal hash alike; others that share a word only
/// collide, which the map's equality tells apart.
impl Hash for EntryKey {
    fn hash<S: Hasher>(&self, state: &mut S) {
        let handle = |kind: u64, handle: Handle| {
            (kind << 61)
                | (u64::from(handle.generation & 0x1fff_ffff) << 32)
                | u64::from(handle.index)
        };
        let word = match *self {
            EntryKey::Undefined => 1 << 61,
            EntryKey::Null => 2 << 61,
            EntryKey::Boolean(boolean) => (3 << 61) | u64::from(boolean),
            EntryKey::Number(bits) => bits,
            EntryKey::String(string) => handle(4, string.handle),
            EntryKey::Symbol(symbol) => handle(5, symbol.handle),
            EntryKey::Object(object) => handle(6, object.handle),
        };
        state.write_u64(word);
    }
}

impl EntryKey {
    /// The key of the Number `number`: NaN as one NaN, `-0` as `+0`.
    fn number(number: f64) -> EntryKey {
        if number.is_nan() {
            EntryKey::Number(CANONICAL_NAN)
        } else if number == 0.0 {
            EntryKey::Number(0)
        } else {
            EntryKey::Number(number.to_bits())
        }
    }

    /// The key of a value that is not a String, or `None` for a String or a BigInt.
    fn of_value(value: Value) -> Option<EntryKey> {
        Some(match value {
            Value::Undefined => EntryKey::Undefined,
            Value::Null => EntryKey::Null,
            Value::Boolean(boolean) => EntryKey::Boolean(boolean),
            Value::Number(number) => EntryKey::number(number),
            Value::Symbol(symbol) => EntryKey::Symbol(symbol),
            Value::Object(object) => EntryKey::Object(object),
            Value::String(_) | Value::BigInt(_) => return None,
        })
    }

    /// The key as a value: the canonical key the List holds.
    fn value(self) -> Value {
        match self {
            EntryKey::Undefined => Value::Undefined,
            EntryKey::Null => Value::Null,
            EntryKey::Boolean(boolean) => Value::Boolean(boolean),
            EntryKey::Number(bits) => Value::Number(f64::from_bits(bits)),
            EntryKey::String(string) => Value::String(string),
            EntryKey::Symbol(symbol) => Value::Symbol(symbol),
            EntryKey::Object(object) => Value::Object(object),
        }
    }
}

/// An entry of the List: its key and its value (`undefined` for a Set).
#[derive(Clone, Copy)]
struct Entry {
    key: EntryKey,
    value: Value,
}

/// A position of the entry vector.
#[derive(Clone, Copy)]
enum Slot {
    Live(Entry),
    /// A position a deletion left empty. During a compaction it holds the live entries before
    /// it, the position the next live entry moves to; otherwise 0.
    Hole(u32),
}

/// The index map of a table whose room passed [`SMALL`], with the bytes it was charged (the
/// box included).
struct Index {
    map: KeyedMap<EntryKey, u32>,
    charged: u64,
}

/// A cursor of a table, as [`EntryTable::open_cursor`] answers it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CursorId(u32);

/// A cursor: the position of the next entry to visit, and the handle of the iterator object
/// that holds it (not traced), or none for a walk in Rust.
#[derive(Clone, Copy)]
struct Cursor {
    position: usize,
    owner: Option<Handle>,
    /// During a compaction, the key of the live entry the cursor is at, whose new position the
    /// cursor takes.
    anchor: Option<EntryKey>,
}

/// A place of the cursor list: an open cursor, or a free place with the next free one.
#[derive(Clone, Copy)]
enum CursorSlot {
    Open(Cursor),
    Free(Option<u32>),
}

/// `n` as a count of bytes or steps.
fn to_u64(n: usize) -> u64 {
    u64::try_from(n).unwrap_or(u64::MAX)
}

/// The bytes of `count` elements of `size` bytes.
fn bytes_of(count: usize, size: usize) -> u64 {
    to_u64(count.saturating_mul(size))
}

/// The bytes of the index map of a table with room for `capacity` entries, its box included.
fn index_bytes(capacity: usize) -> u64 {
    to_u64(size_of::<Index>()).saturating_add(KeyedMap::<EntryKey, u32>::table_bytes_for(capacity))
}

/// The position of `key` in `entries`: through the map, or a scan of at most [`SMALL`]
/// entries.
fn position_in(entries: &[Slot], index: Option<&Index>, key: EntryKey) -> Option<usize> {
    if let Some(index) = index {
        return index
            .map
            .get(&key)
            .and_then(|&position| usize::try_from(position).ok());
    }
    entries
        .iter()
        .position(|slot| matches!(slot, Slot::Live(entry) if entry.key == key))
}

/// The key of `value` for an insertion: a String is interned, its code units past
/// `FREE_CODE_UNITS` charged before they are read.
fn key_to_insert<H: Host>(agent: &mut Agent<H>, value: Value) -> Completion<EntryKey> {
    let Value::String(string) = value else {
        return key_of_other(agent, value);
    };
    let length = agent.string_length(string)?;
    agent.charge_code_units(length)?;
    match agent.heap.intern_string(string) {
        Ok(Ok(atom)) => Ok(EntryKey::String(atom)),
        Ok(Err(_)) => Err(agent.terminate(Termination::MemoryBudget)),
        Err(_) => Err(agent.fault()),
    }
}

/// The key of `value` for a lookup or a deletion, or `None` for a String no atom has, which no
/// table holds: the atom is looked up in place, making nothing, its code units past
/// `FREE_CODE_UNITS` charged before they are read.
fn key_to_find<H: Host>(agent: &mut Agent<H>, value: Value) -> Completion<Option<EntryKey>> {
    let Value::String(string) = value else {
        return key_of_other(agent, value).map(Some);
    };
    let length = agent.string_length(string)?;
    agent.charge_code_units(length)?;
    let atom = match agent.heap.strings.get(string.handle) {
        Ok(record) => agent.heap.atom_of(record.units()),
        Err(_) => return Err(agent.fault()),
    };
    Ok(atom.map(EntryKey::String))
}

/// The key of a value that is not a String. No BigInt exists before slice 12, so one here is a
/// broken invariant of the engine.
fn key_of_other<H: Host>(agent: &mut Agent<H>, value: Value) -> Completion<EntryKey> {
    match EntryKey::of_value(value) {
        Some(key) => Ok(key),
        None => Err(agent.fault()),
    }
}

/// The List of [[MapData]] or [[SetData]] with its cursors.
#[derive(Default)]
pub(crate) struct EntryTable {
    /// The entries in insertion order, holes included.
    entries: Vec<Slot>,
    /// The index map, once the room of `entries` passed [`SMALL`].
    index: Option<Box<Index>>,
    /// The live entries.
    live: usize,
    /// The holes of `entries`.
    holes: usize,
    /// The first hole of `entries`, if it has one.
    first_hole: Option<usize>,
    /// The bytes charged for the room of `entries`.
    entry_bytes: u64,
    /// The cursor list.
    cursors: Vec<CursorSlot>,
    /// The first free place of the cursor list.
    free_cursor: Option<u32>,
    /// The open cursors.
    open_cursors: usize,
    /// The bytes charged for the room of the cursor list.
    cursor_bytes: u64,
    /// The agent's queue of bytes to credit after a sweep, set at the first charge.
    queue: Option<Rc<Cell<u64>>>,
}

impl Drop for EntryTable {
    /// Queue what the counts still hold for the agent to credit after the sweep: no panic, no
    /// allocation.
    fn drop(&mut self) {
        if let Some(queue) = &self.queue {
            let index = self.index.as_ref().map_or(0, |index| index.charged);
            let bytes = self
                .entry_bytes
                .saturating_add(index)
                .saturating_add(self.cursor_bytes);
            queue.set(queue.get().saturating_add(bytes));
        }
    }
}

impl EntryTable {
    /// An empty table: no storage, nothing charged.
    pub(crate) fn new() -> EntryTable {
        EntryTable::default()
    }

    /// The live entries.
    pub(crate) fn len(&self) -> usize {
        self.live
    }

    /// Keep the agent's queue, which the drop credits through, at the first charge.
    fn keep_queue<H: Host>(&mut self, agent: &Agent<H>) {
        if self.queue.is_none() {
            self.queue = Some(Rc::clone(&agent.credit));
        }
    }

    /// The position of the entry of `key`, if the table holds it: one step, and a String key's
    /// code units past `FREE_CODE_UNITS`, before the lookup.
    pub(crate) fn find<H: Host>(
        &self,
        agent: &mut Agent<H>,
        key: Value,
    ) -> Completion<Option<usize>> {
        agent.charge_steps(1)?;
        let Some(key) = key_to_find(agent, key)? else {
            return Ok(None);
        };
        Ok(self.position(key))
    }

    /// The position of `key`.
    fn position(&self, key: EntryKey) -> Option<usize> {
        position_in(&self.entries, self.index.as_deref(), key)
    }

    /// Store `value` under `key` (`#sec-map.prototype.set`, `#sec-set.prototype.add` with
    /// `undefined`): the value of an entry the table holds is overwritten in place, its
    /// position unchanged; otherwise the entry is appended, after a compaction or a growth of
    /// the storage when it is full. One step, and a String key's code units, before the work;
    /// over a budget the table is as it was.
    pub(crate) fn insert<H: Host>(
        &mut self,
        agent: &mut Agent<H>,
        key: Value,
        value: Value,
    ) -> Completion<()> {
        agent.charge_steps(1)?;
        let key = key_to_insert(agent, key)?;
        if let Some(position) = self.position(key) {
            return match self.entries.get_mut(position) {
                Some(Slot::Live(entry)) => {
                    entry.value = value;
                    Ok(())
                }
                _ => Err(agent.fault()),
            };
        }
        self.make_room(agent)?;
        let Ok(position) = u32::try_from(self.entries.len()) else {
            return Err(agent.terminate(Termination::MemoryBudget));
        };
        if let Some(index) = &mut self.index
            && index.map.insert(key, position).is_err()
        {
            // The map was given room for every position of the storage.
            return Err(agent.fault());
        }
        // The room was reserved by `make_room`: the push does not allocate.
        self.entries.push(Slot::Live(Entry { key, value }));
        self.live = self.live.saturating_add(1);
        Ok(())
    }

    /// Whether the storage is full and at least half of it is holes: the next insertion then
    /// compacts it in place instead of growing it (the `compacts` rule of `object/property.rs`).
    fn compacts(&self) -> bool {
        let length = self.entries.len();
        length >= self.entries.capacity()
            && self.holes > 0
            && self.holes.saturating_mul(2) >= length
    }

    /// Room for one more entry: nothing while the storage has room; a compaction when it
    /// [`compacts`](EntryTable::compacts); otherwise the storage doubles (at least 4) and,
    /// once that passes [`SMALL`], the index map gets room for as many keys. Both growths are
    /// charged with `Agent::charge_bytes` before they are made; over the budget the run ends
    /// in `Terminated(MemoryBudget)` with nothing charged and the table as it was.
    fn make_room<H: Host>(&mut self, agent: &mut Agent<H>) -> Completion<()> {
        let length = self.entries.len();
        let capacity = self.entries.capacity();
        if length < capacity {
            return Ok(());
        }
        if self.compacts() {
            return self.compact(agent);
        }
        let wanted = capacity.saturating_mul(2).max(4);
        let list_bytes = bytes_of(wanted.saturating_sub(capacity), size_of::<Slot>());
        let indexed = wanted > SMALL;
        let had = self.index.as_ref().map_or(0, |index| index.charged);
        let map_bytes = if indexed {
            index_bytes(wanted).saturating_sub(had)
        } else {
            0
        };
        agent.charge_bytes(list_bytes.saturating_add(map_bytes))?;
        self.keep_queue(agent);
        if indexed && let Err(abrupt) = self.reserve_index(agent, wanted, map_bytes) {
            agent.heap.accounting.credit(list_bytes);
            return Err(abrupt);
        }
        #[expect(
            clippy::disallowed_methods,
            reason = "charged by `Agent::charge_bytes` as `list_bytes` before it grows, kept in `entry_bytes`"
        )]
        let reserved = self
            .entries
            .try_reserve_exact(wanted.saturating_sub(length));
        if reserved.is_err() {
            agent.heap.accounting.credit(list_bytes);
            return Err(agent.terminate(Termination::MemoryBudget));
        }
        self.entry_bytes = self.entry_bytes.saturating_add(list_bytes);
        Ok(())
    }

    /// Give the index map room for `wanted` keys, building it from the storage when the table
    /// has none. `charged` bytes were charged for it before; the difference to the table the
    /// map then has is charged or credited and kept in the map's count. A refusal leaves the
    /// table without a map if it had none, and with its map if it had one.
    fn reserve_index<H: Host>(
        &mut self,
        agent: &mut Agent<H>,
        wanted: usize,
        charged: u64,
    ) -> Completion<()> {
        let built = self.index.is_none();
        let hasher = agent.heap.hasher;
        let index = self.index.get_or_insert_with(|| {
            Box::new(Index {
                map: KeyedMap::with_hasher(hasher),
                charged: 0,
            })
        });
        if index
            .map
            .reserve(wanted.saturating_sub(index.map.len()))
            .is_err()
        {
            agent.heap.accounting.credit(charged);
            if built {
                self.index = None;
            }
            return Err(agent.terminate(Termination::MemoryBudget));
        }
        if built {
            // A table without a map has at most `SMALL` positions.
            for (position, slot) in self.entries.iter().enumerate() {
                if let Slot::Live(entry) = slot {
                    let inserted = u32::try_from(position)
                        .ok()
                        .is_some_and(|position| index.map.insert(entry.key, position).is_ok());
                    if !inserted {
                        return Err(agent.fault());
                    }
                }
            }
        }
        let before = index.charged;
        let expected = before.saturating_add(charged);
        let table = to_u64(size_of::<Index>()).saturating_add(index.map.table_bytes());
        if table > expected {
            // The map took more than its layout says: charged now.
            if agent.heap.accounting.charge(table - expected).is_err() {
                index.charged = expected;
                return Err(agent.terminate(Termination::MemoryBudget));
            }
        } else {
            agent.heap.accounting.credit(expected - table);
        }
        index.charged = table;
        Ok(())
    }

    /// Drop the holes and number the entries again, without allocating: the storage keeps its
    /// room and the map, cleared, its own. Every open cursor past the first hole moves down by
    /// the holes before it, so it names the same next entry. One step per entry moved (with
    /// an index map, per live entry, which it numbers again) and one per cursor moved, before
    /// the work. A table without a hole is left as it is.
    pub(crate) fn compact<H: Host>(&mut self, agent: &mut Agent<H>) -> Completion<()> {
        let Some(first_hole) = self.first_hole else {
            return Ok(());
        };
        let length = self.entries.len();
        let moved_entries = if self.index.is_some() {
            self.live
        } else {
            length.saturating_sub(first_hole).saturating_sub(self.holes)
        };
        let moved_cursors = self
            .cursors
            .iter()
            .filter(|slot| matches!(slot, CursorSlot::Open(cursor) if cursor.position > first_hole))
            .count();
        agent.charge_steps(to_u64(moved_entries).saturating_add(to_u64(moved_cursors)))?;

        // Each hole records the live entries before it: the position the next live entry
        // takes.
        let mut live_before = first_hole;
        for slot in self.entries.iter_mut().skip(first_hole) {
            match slot {
                Slot::Live(_) => live_before = live_before.saturating_add(1),
                Slot::Hole(next) => *next = u32::try_from(live_before).unwrap_or(u32::MAX),
            }
        }
        // A cursor at a hole or past the end takes its new position now; one at a live entry
        // takes that entry's new position once the entries moved.
        let live = self.live;
        for slot in &mut self.cursors {
            let CursorSlot::Open(cursor) = slot else {
                continue;
            };
            if cursor.position <= first_hole {
                continue;
            }
            match self.entries.get(cursor.position) {
                None => cursor.position = live,
                Some(Slot::Hole(next)) => {
                    cursor.position = usize::try_from(*next).unwrap_or(live);
                }
                Some(Slot::Live(entry)) => cursor.anchor = Some(entry.key),
            }
        }
        // Moves the live entries down in place; allocates nothing.
        self.entries.retain(|slot| matches!(slot, Slot::Live(_)));
        self.holes = 0;
        self.first_hole = None;
        if let Some(index) = &mut self.index {
            index.map.clear();
            for (position, slot) in self.entries.iter().enumerate() {
                if let Slot::Live(entry) = slot {
                    let inserted = u32::try_from(position)
                        .ok()
                        .is_some_and(|position| index.map.insert(entry.key, position).is_ok());
                    if !inserted {
                        return Err(agent.fault());
                    }
                }
            }
        }
        for slot in &mut self.cursors {
            if let CursorSlot::Open(cursor) = slot
                && let Some(key) = cursor.anchor.take()
            {
                cursor.position =
                    position_in(&self.entries, self.index.as_deref(), key).unwrap_or(live);
            }
        }
        Ok(())
    }

    /// Delete the entry of `key` (`#sec-map.prototype.delete`): its position becomes a hole,
    /// its key leaves the index map. One step, and a String key's code units, before the
    /// lookup. Answers whether the table held the key.
    pub(crate) fn delete<H: Host>(&mut self, agent: &mut Agent<H>, key: Value) -> Completion<bool> {
        agent.charge_steps(1)?;
        let Some(key) = key_to_find(agent, key)? else {
            return Ok(false);
        };
        let Some(position) = self.position(key) else {
            return Ok(false);
        };
        let Some(slot) = self.entries.get_mut(position) else {
            return Err(agent.fault());
        };
        *slot = Slot::Hole(0);
        if let Some(index) = &mut self.index {
            index.map.remove(&key);
        }
        self.live = self.live.saturating_sub(1);
        self.holes = self.holes.saturating_add(1);
        self.first_hole = Some(
            self.first_hole
                .map_or(position, |first| first.min(position)),
        );
        Ok(true)
    }

    /// `#sec-map.prototype.clear`: every entry becomes empty, and the storage and the index map
    /// are given back and credited; every open cursor is set to the end of the empty storage,
    /// so it visits only entries added after. One step per 64 entries and one per open cursor,
    /// before the work.
    pub(crate) fn clear<H: Host>(&mut self, agent: &mut Agent<H>) -> Completion<()> {
        let steps = to_u64(self.entries.len().div_ceil(CLEARED_PER_STEP))
            .saturating_add(to_u64(self.open_cursors));
        agent.charge_steps(steps)?;
        self.entries = Vec::new();
        let index = self.index.take().map_or(0, |index| index.charged);
        agent
            .heap
            .accounting
            .credit(self.entry_bytes.saturating_add(index));
        self.entry_bytes = 0;
        self.live = 0;
        self.holes = 0;
        self.first_hole = None;
        for slot in &mut self.cursors {
            if let CursorSlot::Open(cursor) = slot {
                cursor.position = 0;
            }
        }
        Ok(())
    }

    /// The key and the value of the entry at `position`, or `None` for a hole or a position
    /// past the end.
    pub(crate) fn entry_at(&self, position: usize) -> Option<(Value, Value)> {
        match self.entries.get(position)? {
            Slot::Live(entry) => Some((entry.key.value(), entry.value)),
            Slot::Hole(_) => None,
        }
    }

    /// The first live position at or after `position`, or `None` past the last entry: one
    /// step per hole it passes, each before the next position is read. The entry it answers
    /// is the caller's step.
    pub(crate) fn next_live<H: Host>(
        &self,
        agent: &mut Agent<H>,
        position: usize,
    ) -> Completion<Option<usize>> {
        let mut at = position;
        loop {
            match self.entries.get(at) {
                None => return Ok(None),
                Some(Slot::Live(_)) => return Ok(Some(at)),
                Some(Slot::Hole(_)) => {
                    agent.charge_steps(1)?;
                    at = at.saturating_add(1);
                }
            }
        }
    }

    /// Register a cursor at the first position, held by the iterator object `owner` or, with
    /// none, by a walk in Rust that closes it: one step, and a growth of the cursor list
    /// (doubling, at least 4) charged with `Agent::charge_bytes` before it is made.
    pub(crate) fn open_cursor<H: Host>(
        &mut self,
        agent: &mut Agent<H>,
        owner: Option<Handle>,
    ) -> Completion<CursorId> {
        agent.charge_steps(1)?;
        let cursor = CursorSlot::Open(Cursor {
            position: 0,
            owner,
            anchor: None,
        });
        if let Some(free) = self.free_cursor {
            let place = usize::try_from(free).ok();
            let Some(slot) = place.and_then(|place| self.cursors.get_mut(place)) else {
                return Err(agent.fault());
            };
            let CursorSlot::Free(next) = *slot else {
                return Err(agent.fault());
            };
            *slot = cursor;
            self.free_cursor = next;
            self.open_cursors = self.open_cursors.saturating_add(1);
            return Ok(CursorId(free));
        }
        let length = self.cursors.len();
        let Ok(id) = u32::try_from(length) else {
            return Err(agent.terminate(Termination::MemoryBudget));
        };
        let capacity = self.cursors.capacity();
        if length >= capacity {
            let wanted = capacity.saturating_mul(2).max(4);
            let bytes = bytes_of(wanted.saturating_sub(capacity), size_of::<CursorSlot>());
            agent.charge_bytes(bytes)?;
            self.keep_queue(agent);
            #[expect(
                clippy::disallowed_methods,
                reason = "charged by `Agent::charge_bytes` as `bytes` before it grows, kept in `cursor_bytes`"
            )]
            let reserved = self
                .cursors
                .try_reserve_exact(wanted.saturating_sub(length));
            if reserved.is_err() {
                agent.heap.accounting.credit(bytes);
                return Err(agent.terminate(Termination::MemoryBudget));
            }
            self.cursor_bytes = self.cursor_bytes.saturating_add(bytes);
        }
        // The room was reserved above: the push does not allocate.
        self.cursors.push(cursor);
        self.open_cursors = self.open_cursors.saturating_add(1);
        Ok(CursorId(id))
    }

    /// The position of the next entry `cursor` visits, or `None` for a cursor that is not open.
    pub(crate) fn cursor(&self, cursor: CursorId) -> Option<usize> {
        let place = usize::try_from(cursor.0).ok()?;
        match self.cursors.get(place)? {
            CursorSlot::Open(open) => Some(open.position),
            CursorSlot::Free(_) => None,
        }
    }

    /// Set the position of an open cursor.
    fn set_cursor(&mut self, cursor: CursorId, position: usize) {
        let place = usize::try_from(cursor.0).ok();
        if let Some(CursorSlot::Open(open)) = place.and_then(|place| self.cursors.get_mut(place)) {
            open.position = position;
        }
    }

    /// The next entry of `cursor`, which then moves past it, or `None` at the end, where the
    /// cursor stays: [`EntryTable::next_live`] charges the holes it passes; the entry answered
    /// is the caller's step.
    pub(crate) fn advance<H: Host>(
        &mut self,
        agent: &mut Agent<H>,
        cursor: CursorId,
    ) -> Completion<Option<(Value, Value)>> {
        let Some(position) = self.cursor(cursor) else {
            return Err(agent.fault());
        };
        match self.next_live(agent, position)? {
            Some(at) => {
                self.set_cursor(cursor, at.saturating_add(1));
                Ok(self.entry_at(at))
            }
            None => {
                self.set_cursor(cursor, self.entries.len());
                Ok(None)
            }
        }
    }

    /// Close `cursor` (an iterator that answers done, every exit of a `forEach`). The place
    /// becomes free; the last open cursor closed frees the list and credits its bytes. A cursor
    /// that is not open is left as it is.
    pub(crate) fn close_cursor<H: Host>(&mut self, agent: &mut Agent<H>, cursor: CursorId) {
        let place = usize::try_from(cursor.0).ok();
        let Some(slot) = place.and_then(|place| self.cursors.get_mut(place)) else {
            return;
        };
        if !matches!(slot, CursorSlot::Open(_)) {
            return;
        }
        *slot = CursorSlot::Free(self.free_cursor);
        self.free_cursor = Some(cursor.0);
        self.open_cursors = self.open_cursors.saturating_sub(1);
        if self.open_cursors == 0 {
            self.cursors = Vec::new();
            self.free_cursor = None;
            agent.heap.accounting.credit(self.cursor_bytes);
            self.cursor_bytes = 0;
        }
    }

    /// For the collector, after the mark phase and before the sweep: drop every cursor whose
    /// owner `is_live` answers false for, in place, and keep every cursor with no owner (a walk
    /// in Rust holds it, and only its `close_cursor` releases it). When no cursor is left the
    /// list is freed (replaced by an empty vector, which allocates nothing) and the bytes it
    /// was charged are answered for the caller to credit; while one is left the list keeps its
    /// room and the answer is 0. Moves no storage and allocates nothing.
    pub(crate) fn release_cursors(&mut self, is_live: &dyn Fn(Handle) -> bool) -> u64 {
        let mut free = None;
        let mut open = 0usize;
        for (place, slot) in self.cursors.iter_mut().enumerate().rev() {
            if let CursorSlot::Open(cursor) = slot
                && cursor.owner.is_some_and(|owner| !is_live(owner))
            {
                *slot = CursorSlot::Free(None);
            }
            match slot {
                CursorSlot::Open(_) => open = open.saturating_add(1),
                CursorSlot::Free(next) => {
                    *next = free;
                    free = u32::try_from(place).ok();
                }
            }
        }
        self.open_cursors = open;
        self.free_cursor = free;
        if open > 0 || self.cursors.capacity() == 0 {
            return 0;
        }
        self.cursors = Vec::new();
        self.free_cursor = None;
        core::mem::take(&mut self.cursor_bytes)
    }

    /// The values the live entries hold, keys first, each entry once; allocates nothing.
    pub(crate) fn traced(&self) -> impl Iterator<Item = Value> + '_ {
        self.entries
            .iter()
            .filter_map(|slot| match slot {
                Slot::Live(entry) => Some([entry.key.value(), entry.value]),
                Slot::Hole(_) => None,
            })
            .flatten()
    }

    /// Every live key (a String key as its atom) and value, for the trace of the kind that owns
    /// the table: a worklist push per value, nothing allocated.
    #[cfg_attr(
        test,
        expect(
            dead_code,
            reason = "called from the trace of the kinds that own a table, story:map-and-set"
        )
    )]
    pub(crate) fn trace(&self, mark: &mut Mark) {
        for value in self.traced() {
            mark.value(value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CursorId, EntryKey, EntryTable};
    use crate::heap::arena::Handle;
    use crate::heap::map::KeyedMap;
    use crate::host::agent::Agent;
    use crate::host::hooks::test_host::{NoHooks, agent_with, limits};
    use crate::host::host_object::{Brand, HostData, HostObject, Tracer};
    use crate::host::outcome::{Abrupt, Termination};
    use crate::object::ordinary::Ordinary;
    use crate::object::property::SMALL;
    use crate::object::{ObjectKind, ObjectRecord};
    use crate::value::{ObjectRef, StringRef, Value};
    use crate::{Outcome, Seeds};
    use alloc::boxed::Box;
    use alloc::string::String;
    use alloc::vec::Vec;

    /// An agent of the library's tests with a budget of 256 MiB and no step limit to reach.
    fn agent() -> Agent<NoHooks> {
        let mut limits = limits();
        limits.memory_bytes = 1 << 28;
        limits.steps_per_entry = u64::MAX;
        agent_with(limits)
    }

    /// An agent as [`agent`] whose hash maps are keyed by `hash_key`.
    fn agent_keyed(hash_key: [u8; 16]) -> Agent<NoHooks> {
        let mut limits = limits();
        limits.memory_bytes = 1 << 28;
        limits.steps_per_entry = u64::MAX;
        Agent::new(
            NoHooks,
            limits,
            Seeds {
                hash_key,
                random_seed: [2; 32],
            },
        )
        .unwrap()
    }

    fn num(n: f64) -> Value {
        Value::Number(n)
    }

    fn units(text: &str) -> Vec<u16> {
        text.encode_utf16().collect()
    }

    /// A new String of `text`, which is not an atom.
    fn string(agent: &mut Agent<NoHooks>, text: &str) -> StringRef {
        agent.heap.new_string_from_str(text).unwrap()
    }

    fn object(agent: &mut Agent<NoHooks>) -> ObjectRef {
        let record = ObjectRecord::new(None, ObjectKind::Ordinary(Ordinary::for_test()));
        agent.heap.new_object(record, 0).unwrap()
    }

    /// A handle no record has, for a cursor's owner: `release_cursors` only compares it.
    fn owner(agent: &Agent<NoHooks>, index: u32) -> Handle {
        Handle {
            key: agent.heap.key,
            index,
            generation: 1,
        }
    }

    /// The value found under `key`.
    fn value_of(agent: &mut Agent<NoHooks>, table: &EntryTable, key: Value) -> Option<Value> {
        let position = table.find(agent, key).unwrap()?;
        table.entry_at(position).map(|(_, value)| value)
    }

    /// The entries in the order a new cursor visits them.
    fn walk(agent: &mut Agent<NoHooks>, table: &mut EntryTable) -> Vec<(Value, Value)> {
        let cursor = table.open_cursor(agent, None).unwrap();
        let mut entries = Vec::new();
        while let Some(entry) = table.advance(agent, cursor).unwrap() {
            entries.push(entry);
        }
        table.close_cursor(agent, cursor);
        entries
    }

    /// The keys a new cursor visits.
    fn keys(agent: &mut Agent<NoHooks>, table: &mut EntryTable) -> Vec<Value> {
        walk(agent, table).into_iter().map(|(key, _)| key).collect()
    }

    /// The key of the entry `cursor` visits next, if any.
    fn next_key(agent: &mut Agent<NoHooks>, table: &EntryTable, cursor: CursorId) -> Option<Value> {
        let position = table.cursor(cursor).unwrap();
        let next = table.next_live(agent, position).unwrap()?;
        table.entry_at(next).map(|(key, _)| key)
    }

    /// Move `cursor` past `count` entries.
    fn skip(agent: &mut Agent<NoHooks>, table: &mut EntryTable, cursor: CursorId, count: usize) {
        for _ in 0..count {
            assert!(table.advance(agent, cursor).unwrap().is_some());
        }
    }

    /// A host object's data that holds a table, so that a collection traces it as the kinds
    /// that own a table will.
    struct Holder(Option<EntryTable>);

    impl HostData for Holder {
        fn trace(&self, tracer: &mut Tracer<'_>) {
            if let Some(table) = &self.0 {
                for value in table.traced() {
                    tracer.value(value);
                }
            }
        }
    }

    /// Run a collection while `table` is held by a host object kept by a root, and answer the
    /// table.
    fn collect_holding(agent: &mut Agent<NoHooks>, table: EntryTable) -> EntryTable {
        let host = HostObject {
            brand: Brand(7),
            data: Some(Box::new(Holder(Some(table)))),
            error_kind: None,
        };
        let record = ObjectRecord::new(None, ObjectKind::Host(host));
        let holder = agent.heap.new_object(record, 0).unwrap();
        let root = agent.heap.roots.add(Value::Object(holder));
        assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
        let record = agent.heap.objects.get_mut(holder.handle).unwrap();
        let ObjectKind::Host(host) = &mut record.kind else {
            panic!("the holder is a host object");
        };
        let table = host.data_mut::<Holder>(Brand(7)).unwrap().0.take().unwrap();
        drop(root);
        table
    }

    /// SameValueZero (`#sec-samevaluezero`) with CanonicalizeKeyedCollectionKey
    /// (`#sec-canonicalizekeyedcollectionkey`): `-0` and `+0` are one key, kept as `+0`; two
    /// NaNs of different bit patterns are one key; two Strings made apart with the same code
    /// units are one key; two objects are two keys, one object one.
    #[test]
    fn keys_compare_by_same_value_zero() {
        let mut agent = agent();
        let mut table = EntryTable::new();
        table.insert(&mut agent, num(-0.0), num(1.0)).unwrap();
        table.insert(&mut agent, num(0.0), num(2.0)).unwrap();
        assert_eq!(table.len(), 1);
        let at = table.find(&mut agent, num(-0.0)).unwrap().unwrap();
        let (key, value) = table.entry_at(at).unwrap();
        let Value::Number(zero) = key else {
            panic!("the key is {key:?}");
        };
        assert!(
            zero == 0.0 && zero.is_sign_positive(),
            "the key is {zero:?}"
        );
        assert_eq!(value, num(2.0));

        let nan_a = f64::from_bits(0x7ff8_0000_0000_0001);
        let nan_b = f64::from_bits(0xfff0_0000_0000_0100);
        assert!(nan_a.is_nan() && nan_b.is_nan() && nan_a.to_bits() != nan_b.to_bits());
        table.insert(&mut agent, num(nan_a), num(3.0)).unwrap();
        table.insert(&mut agent, num(nan_b), num(4.0)).unwrap();
        assert_eq!(table.len(), 2);
        assert_eq!(value_of(&mut agent, &table, num(f64::NAN)), Some(num(4.0)));

        let first = string(&mut agent, "same units");
        let second = string(&mut agent, "same units");
        assert_ne!(first, second);
        table
            .insert(&mut agent, Value::String(first), num(5.0))
            .unwrap();
        table
            .insert(&mut agent, Value::String(second), num(6.0))
            .unwrap();
        assert_eq!(table.len(), 3);
        let third = string(&mut agent, "same units");
        assert_eq!(
            value_of(&mut agent, &table, Value::String(third)),
            Some(num(6.0))
        );

        let one = object(&mut agent);
        let other = object(&mut agent);
        table
            .insert(&mut agent, Value::Object(one), num(7.0))
            .unwrap();
        table
            .insert(&mut agent, Value::Object(other), num(8.0))
            .unwrap();
        table
            .insert(&mut agent, Value::Object(one), num(9.0))
            .unwrap();
        assert_eq!(table.len(), 5);
        assert_eq!(
            value_of(&mut agent, &table, Value::Object(one)),
            Some(num(9.0))
        );
        assert_eq!(
            value_of(&mut agent, &table, Value::Object(other)),
            Some(num(8.0))
        );
    }

    /// `#sec-map.prototype.set` and `#sec-map.prototype.delete`: an overwrite keeps the
    /// entry's position, a deletion keeps the order of the others, and a key added again after
    /// its deletion comes last; on both sides of the small size.
    #[test]
    fn insertion_order_survives_deletes_and_overwrites() {
        let mut agent = agent();
        let mut table = EntryTable::new();
        for i in 0..20u32 {
            let key = f64::from(i);
            table.insert(&mut agent, num(key), num(key * 10.0)).unwrap();
        }
        table.insert(&mut agent, num(5.0), num(999.0)).unwrap();
        assert!(table.delete(&mut agent, num(3.0)).unwrap());
        assert!(table.delete(&mut agent, num(17.0)).unwrap());
        assert!(!table.delete(&mut agent, num(17.0)).unwrap());
        table.insert(&mut agent, num(3.0), num(33.0)).unwrap();
        assert_eq!(table.len(), 19);
        let mut expected: Vec<(Value, Value)> = (0..20u32)
            .filter(|&i| i != 3 && i != 17)
            .map(|i| {
                let key = f64::from(i);
                let value = if i == 5 { 999.0 } else { key * 10.0 };
                (num(key), num(value))
            })
            .collect();
        expected.push((num(3.0), num(33.0)));
        assert_eq!(walk(&mut agent, &mut table), expected);

        let mut small = EntryTable::new();
        for i in 0..4u32 {
            small
                .insert(&mut agent, num(f64::from(i)), Value::Null)
                .unwrap();
        }
        assert!(small.delete(&mut agent, num(1.0)).unwrap());
        small
            .insert(&mut agent, num(2.0), Value::Undefined)
            .unwrap();
        small.insert(&mut agent, num(1.0), Value::Null).unwrap();
        assert_eq!(
            walk(&mut agent, &mut small),
            [
                (num(0.0), Value::Null),
                (num(2.0), Value::Undefined),
                (num(3.0), Value::Null),
                (num(1.0), Value::Null)
            ]
        );
    }

    /// `#sec-map.prototype.foreach` and `#sec-%mapiteratorprototype%.next`: a cursor visits
    /// an entry added before it reaches the entry's position, and not one deleted before it
    /// reaches it; a deletion behind it changes nothing it visits.
    #[test]
    fn a_cursor_visits_an_entry_added_before_it_and_skips_one_deleted_before_it() {
        let mut agent = agent();
        let mut table = EntryTable::new();
        for i in 1..=4u32 {
            table
                .insert(&mut agent, num(f64::from(i)), Value::Null)
                .unwrap();
        }
        let cursor = table.open_cursor(&mut agent, None).unwrap();
        let mut visited = Vec::new();
        visited.push(table.advance(&mut agent, cursor).unwrap().unwrap().0);
        assert!(table.delete(&mut agent, num(3.0)).unwrap());
        table.insert(&mut agent, num(5.0), Value::Null).unwrap();
        assert!(table.delete(&mut agent, num(1.0)).unwrap());
        while let Some((key, _)) = table.advance(&mut agent, cursor).unwrap() {
            visited.push(key);
        }
        table.close_cursor(&mut agent, cursor);
        assert_eq!(visited, [num(1.0), num(2.0), num(4.0), num(5.0)]);
    }

    /// The `compacts` rule of `object/property.rs` with cursors: 1,000 entries, every second
    /// deleted, two cursors at different positions, a compaction forced: each cursor's next
    /// entry is the same before and after, the holes are gone and the order holds.
    #[test]
    fn compaction_moves_every_cursor_to_the_same_next_entry() {
        let mut agent = agent();
        let mut table = EntryTable::new();
        for i in 0..1_000u32 {
            table
                .insert(&mut agent, num(f64::from(i)), Value::Null)
                .unwrap();
        }
        let early = table.open_cursor(&mut agent, None).unwrap();
        let late = table.open_cursor(&mut agent, None).unwrap();
        skip(&mut agent, &mut table, early, 101);
        skip(&mut agent, &mut table, late, 600);
        for i in (0..1_000u32).step_by(2) {
            assert!(table.delete(&mut agent, num(f64::from(i))).unwrap());
        }
        assert_eq!(next_key(&mut agent, &table, early), Some(num(101.0)));
        assert_eq!(next_key(&mut agent, &table, late), Some(num(601.0)));
        let positions = (table.cursor(early), table.cursor(late));

        table.compact(&mut agent).unwrap();
        assert_eq!(table.entries.len(), 500, "the holes are gone");
        assert_ne!((table.cursor(early), table.cursor(late)), positions);
        assert_eq!(next_key(&mut agent, &table, early), Some(num(101.0)));
        assert_eq!(next_key(&mut agent, &table, late), Some(num(601.0)));
        assert_eq!(table.len(), 500);
        let expected: Vec<Value> = (1..1_000u32)
            .step_by(2)
            .map(|i| num(f64::from(i)))
            .collect();
        assert_eq!(keys(&mut agent, &mut table), expected);
        for i in (1..1_000u32).step_by(2) {
            assert_eq!(
                value_of(&mut agent, &table, num(f64::from(i))),
                Some(Value::Null)
            );
        }
    }

    /// `#sec-map.prototype.clear` with `#sec-%mapiteratorprototype%.next`: `clear` ends every
    /// cursor, and a cursor that runs on after it visits the entries added after it, and only
    /// those.
    #[test]
    fn clear_ends_every_cursor_and_entries_added_after_are_visited() {
        let mut agent = agent();
        let mut table = EntryTable::new();
        for i in 0..10u32 {
            table
                .insert(&mut agent, num(f64::from(i)), Value::Null)
                .unwrap();
        }
        let halfway = table.open_cursor(&mut agent, None).unwrap();
        let fresh = table.open_cursor(&mut agent, None).unwrap();
        skip(&mut agent, &mut table, halfway, 5);
        table.clear(&mut agent).unwrap();
        assert_eq!(table.len(), 0);
        assert_eq!(table.advance(&mut agent, halfway).unwrap(), None);
        for i in 100..103u32 {
            table
                .insert(&mut agent, num(f64::from(i)), Value::Null)
                .unwrap();
        }
        for cursor in [halfway, fresh] {
            let mut visited = Vec::new();
            while let Some((key, _)) = table.advance(&mut agent, cursor).unwrap() {
                visited.push(key);
            }
            assert_eq!(visited, [num(100.0), num(101.0), num(102.0)]);
            table.close_cursor(&mut agent, cursor);
        }
    }

    /// Decision 4, "Objects" and "Memory budget": the table keeps an index map once its room
    /// passes `SMALL`, and the map is charged before it is made: at the `SMALL + 1`th
    /// insertion the charged reading grows by at least the table of a map of that many keys.
    #[test]
    fn the_index_map_is_made_above_small_and_charged() {
        let mut agent = agent();
        let mut table = EntryTable::new();
        for i in 0..SMALL {
            let key = f64::from(u32::try_from(i).unwrap());
            table.insert(&mut agent, num(key), Value::Null).unwrap();
        }
        assert!(table.index.is_none());
        let before = agent.heap_bytes().unwrap();
        let key = f64::from(u32::try_from(SMALL).unwrap());
        table.insert(&mut agent, num(key), Value::Null).unwrap();
        let grown = agent.heap_bytes().unwrap() - before;
        assert!(table.index.is_some());
        let map = KeyedMap::<EntryKey, u32>::table_bytes_for(SMALL + 1);
        assert!(
            grown >= map,
            "grew {grown}, a map of SMALL + 1 keys is {map}"
        );
        for i in 0..=SMALL {
            let key = f64::from(u32::try_from(i).unwrap());
            assert_eq!(value_of(&mut agent, &table, num(key)), Some(Value::Null));
        }
    }

    /// The memory budget (decision 4): a table filled until a growth does not fit ends the run
    /// with `Terminated(MemoryBudget)`; the refused insertion leaves the length, the charged
    /// reading and every entry as they were.
    #[test]
    fn a_growth_over_the_memory_budget_ends_the_run_and_leaves_the_table() {
        let mut agent = agent();
        let mut table = EntryTable::new();
        let limit = agent.heap_bytes().unwrap() + (64 << 10);
        agent.heap.accounting.set_limit(limit);
        let mut refused = None;
        for i in 0..10_000u32 {
            let (length, bytes) = (table.len(), agent.heap_bytes().unwrap());
            match table.insert(&mut agent, num(f64::from(i)), Value::Null) {
                Ok(()) => assert_eq!(table.len(), length + 1),
                Err(abrupt) => {
                    assert_eq!(abrupt, Abrupt::Terminated(Termination::MemoryBudget));
                    assert_eq!(table.len(), length);
                    assert_eq!(agent.heap_bytes().unwrap(), bytes);
                    refused = Some(i);
                    break;
                }
            }
        }
        let refused = refused.expect("the budget ended the insertions");
        agent.termination = None;
        assert_eq!(table.len(), usize::try_from(refused).unwrap());
        for i in 0..refused {
            assert_eq!(
                value_of(&mut agent, &table, num(f64::from(i))),
                Some(Value::Null)
            );
        }
        assert_eq!(
            table.find(&mut agent, num(f64::from(refused))).unwrap(),
            None
        );
    }

    /// The step budget (decision 3): each insertion charges its step before its work, so a loop
    /// of insertions under `steps_per_entry` N ends in `Terminated(StepBudget)` with
    /// `last_entry_steps()` at most N, one entry per step.
    #[test]
    fn a_loop_of_inserts_ends_at_the_step_budget() {
        const N: u64 = 500;
        let mut agent = agent();
        let mut table = EntryTable::new();
        agent.steps = 0;
        agent.limits.steps_per_entry = N;
        let mut ended = None;
        for i in 0..10_000u32 {
            if let Err(abrupt) = table.insert(&mut agent, num(f64::from(i)), Value::Null) {
                ended = Some(abrupt);
                break;
            }
        }
        assert_eq!(ended, Some(Abrupt::Terminated(Termination::StepBudget)));
        let spent = agent.last_entry_steps().unwrap();
        assert!(spent <= N, "{spent} steps of {N}");
        assert_eq!(u64::try_from(table.len()).unwrap(), N);
    }

    /// Decision 4, "Collector", and the atom table's weak entries (`heap/string.rs`): a String
    /// key made by concatenation, held by nothing but its entry, is kept as its atom by the
    /// table's trace; after a collection a String made apart with the same code units finds
    /// the entry, while the concatenation itself, which the entry does not hold, is freed.
    #[test]
    fn a_string_key_keeps_its_atom_alive_across_collect() {
        let mut agent = agent();
        let mut table = EntryTable::new();
        let left = string(&mut agent, "entry ");
        let right = string(&mut agent, "key");
        let joined = agent.concatenate(left, right).unwrap();
        assert_eq!(agent.heap.atom_of(&units("entry key")[..]), None);
        table
            .insert(&mut agent, Value::String(joined), num(42.0))
            .unwrap();
        let table = collect_holding(&mut agent, table);
        assert!(agent.heap.strings.get(joined.handle).is_err());
        let probe = string(&mut agent, "entry key");
        assert_eq!(
            value_of(&mut agent, &table, Value::String(probe)),
            Some(num(42.0))
        );
    }

    /// The trace lists live entries only: after the entry of a String key is deleted, a
    /// collection frees its atom's slot.
    #[test]
    fn a_deleted_string_key_is_freed_by_collect() {
        let mut agent = agent();
        let mut table = EntryTable::new();
        let left = string(&mut agent, "entry ");
        let right = string(&mut agent, "key");
        let joined = agent.concatenate(left, right).unwrap();
        table
            .insert(&mut agent, Value::String(joined), Value::Null)
            .unwrap();
        let Some((Value::String(atom), _)) = table.entry_at(0) else {
            panic!("the key is a String");
        };
        let mut table = collect_holding(&mut agent, table);
        assert!(
            agent.heap.strings.get(atom.handle).is_ok(),
            "kept by its entry"
        );
        let probe = string(&mut agent, "entry key");
        assert!(table.delete(&mut agent, Value::String(probe)).unwrap());
        let _table = collect_holding(&mut agent, table);
        assert!(agent.heap.strings.get(atom.handle).is_err());
        assert_eq!(agent.heap.atom_of(&units("entry key")[..]), None);
    }

    /// HOST-145 (`docs/design/host-interface.md`, decision 3): no order a script can observe
    /// comes from a hash map. Two agents with different `hash_key`, the same 10,000 insertions
    /// and 5,000 deletions: a cursor visits the same keys in the same order, the order of
    /// insertion of the keys left.
    #[test]
    fn the_order_does_not_depend_on_hash_key() {
        let key = |i: u32| num(f64::from(i.wrapping_mul(7_919) % 10_007));
        let mut orders = Vec::new();
        for hash_key in [[1; 16], [9; 16]] {
            let mut agent = agent_keyed(hash_key);
            let mut table = EntryTable::new();
            for i in 0..10_000u32 {
                table.insert(&mut agent, key(i), Value::Null).unwrap();
            }
            for i in (0..10_000u32).step_by(2) {
                assert!(table.delete(&mut agent, key(i)).unwrap());
            }
            orders.push(keys(&mut agent, &mut table));
        }
        let expected: Vec<Value> = (1..10_000u32).step_by(2).map(key).collect();
        assert_eq!(orders[0], expected);
        assert_eq!(orders[1], expected);
    }

    /// A collection keeps `"0"` and `0` apart (SameValueZero is false for them): two entries,
    /// and `"0"` found by a String made apart, its key a String.
    #[test]
    fn an_index_like_string_key_stays_a_string() {
        let mut agent = agent();
        let mut table = EntryTable::new();
        let zero = string(&mut agent, "0");
        table
            .insert(&mut agent, Value::String(zero), num(1.0))
            .unwrap();
        table.insert(&mut agent, num(0.0), num(2.0)).unwrap();
        assert_eq!(table.len(), 2);
        let probe = string(&mut agent, "0");
        let at = table
            .find(&mut agent, Value::String(probe))
            .unwrap()
            .unwrap();
        let (key, value) = table.entry_at(at).unwrap();
        assert!(matches!(key, Value::String(_)), "the key is {key:?}");
        assert_eq!(value, num(1.0));
        assert_eq!(value_of(&mut agent, &table, num(-0.0)), Some(num(2.0)));
    }

    /// "The step budget": a String key's code units past `FREE_CODE_UNITS` (32) are charged a
    /// step each before they are read: a key of 10,000 code units makes `insert` and `find`
    /// each spend at least 10,000 - 32 steps.
    #[test]
    fn a_long_string_key_charges_its_code_units() {
        let mut agent = agent();
        let mut table = EntryTable::new();
        let text: String = "x".repeat(10_000);
        let key = string(&mut agent, &text);
        let before = agent.last_entry_steps().unwrap();
        table
            .insert(&mut agent, Value::String(key), num(1.0))
            .unwrap();
        let spent = agent.last_entry_steps().unwrap() - before;
        assert!(spent >= 10_000 - 32, "insert spent {spent}");
        let probe = string(&mut agent, &text);
        let before = agent.last_entry_steps().unwrap();
        assert!(
            table
                .find(&mut agent, Value::String(probe))
                .unwrap()
                .is_some()
        );
        let spent = agent.last_entry_steps().unwrap() - before;
        assert!(spent >= 10_000 - 32, "find spent {spent}");
    }

    /// The collector's release of the cursors of iterators it found dead: 10,000 cursors with
    /// owners that `is_live` answers false for are all dropped, and the bytes `release_cursors`
    /// answers bring the charged reading back to where it was before the first cursor.
    #[test]
    fn released_cursors_give_their_bytes_back() {
        let mut agent = agent();
        let mut table = EntryTable::new();
        for i in 0..3u32 {
            table
                .insert(&mut agent, num(f64::from(i)), Value::Null)
                .unwrap();
        }
        let before = agent.heap_bytes().unwrap();
        let mut cursors = Vec::new();
        for i in 0..10_000u32 {
            let owner = owner(&agent, i);
            cursors.push(table.open_cursor(&mut agent, Some(owner)).unwrap());
        }
        assert!(agent.heap_bytes().unwrap() > before);
        let released = table.release_cursors(&|_| false);
        agent.heap.accounting.credit(released);
        assert_eq!(agent.heap_bytes().unwrap(), before);
        assert!(cursors.iter().all(|&cursor| table.cursor(cursor).is_none()));
    }

    /// A table of 1,000 entries with every even key, the entries at even positions, deleted.
    fn table_with_holes(agent: &mut Agent<NoHooks>) -> EntryTable {
        let mut table = EntryTable::new();
        for i in 0..1_000u32 {
            table.insert(agent, num(f64::from(i)), Value::Null).unwrap();
        }
        for i in (0..1_000u32).step_by(2) {
            assert!(table.delete(agent, num(f64::from(i))).unwrap());
        }
        table
    }

    /// "The step budget": a compaction charges a step per entry it moves and one per cursor it
    /// moves. 1,000 entries, 500 deleted, 100 live cursors past the first hole: at most
    /// 500 + 100 + 1 steps; with the 100 cursors released before it, at most 501.
    #[test]
    fn compaction_charges_a_step_per_cursor_it_moves() {
        let mut agent = agent();
        let mut table = table_with_holes(&mut agent);
        let mut cursors = Vec::new();
        for i in 0..100u32 {
            let owner = owner(&agent, i);
            let cursor = table.open_cursor(&mut agent, Some(owner)).unwrap();
            skip(
                &mut agent,
                &mut table,
                cursor,
                usize::try_from(i).unwrap() + 1,
            );
            cursors.push(cursor);
        }
        let next: Vec<Option<Value>> = cursors
            .iter()
            .map(|&cursor| next_key(&mut agent, &table, cursor))
            .collect();
        let before = agent.last_entry_steps().unwrap();
        table.compact(&mut agent).unwrap();
        let spent = agent.last_entry_steps().unwrap() - before;
        assert!(spent <= 500 + 100 + 1, "the compaction spent {spent}");
        assert!(
            spent >= 100,
            "the cursors it moved were not charged: {spent}"
        );
        let after: Vec<Option<Value>> = cursors
            .iter()
            .map(|&cursor| next_key(&mut agent, &table, cursor))
            .collect();
        assert_eq!(after, next);

        let mut table = table_with_holes(&mut agent);
        for i in 0..100u32 {
            let owner = owner(&agent, i);
            let cursor = table.open_cursor(&mut agent, Some(owner)).unwrap();
            skip(
                &mut agent,
                &mut table,
                cursor,
                usize::try_from(i).unwrap() + 1,
            );
        }
        let released = table.release_cursors(&|_| false);
        agent.heap.accounting.credit(released);
        let before = agent.last_entry_steps().unwrap();
        table.compact(&mut agent).unwrap();
        let spent = agent.last_entry_steps().unwrap() - before;
        assert!(spent <= 501, "the compaction spent {spent} with no cursor");
    }

    /// The cursor list is credited only when it is given back: closing cursors one at a time
    /// ends where it started; closing one of two credits nothing; `release_cursors` keeps a
    /// cursor with no owner, which only its walk closes, and answers 0 while one is left.
    #[test]
    fn the_cursor_list_credits_only_what_it_gives_back() {
        let mut agent = agent();
        let mut table = EntryTable::new();
        for i in 1..=3u32 {
            table
                .insert(&mut agent, num(f64::from(i)), Value::Null)
                .unwrap();
        }
        let start = agent.heap_bytes().unwrap();
        for _ in 0..1_000 {
            let cursor = table.open_cursor(&mut agent, None).unwrap();
            table.close_cursor(&mut agent, cursor);
        }
        assert_eq!(agent.heap_bytes().unwrap(), start);

        let first = table.open_cursor(&mut agent, None).unwrap();
        let second = table.open_cursor(&mut agent, None).unwrap();
        let both = agent.heap_bytes().unwrap();
        assert!(both > start);
        table.close_cursor(&mut agent, first);
        assert_eq!(agent.heap_bytes().unwrap(), both);
        table.close_cursor(&mut agent, second);
        assert_eq!(agent.heap_bytes().unwrap(), start);

        let held_by = owner(&agent, 1);
        let owned = table.open_cursor(&mut agent, Some(held_by)).unwrap();
        let walk = table.open_cursor(&mut agent, None).unwrap();
        skip(&mut agent, &mut table, walk, 1);
        assert_eq!(next_key(&mut agent, &table, walk), Some(num(2.0)));
        assert_eq!(table.release_cursors(&|_| false), 0);
        assert_eq!(table.cursor(owned), None);
        assert_eq!(table.cursor(walk), Some(1));
        assert_eq!(next_key(&mut agent, &table, walk), Some(num(2.0)));
        table.close_cursor(&mut agent, walk);
        assert_eq!(agent.heap_bytes().unwrap(), start);
    }
}
