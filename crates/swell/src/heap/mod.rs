//! The heap of one agent (`docs/design/engine.md`, decision 4; `docs/design/host-interface.md`,
//! decisions 4 and 18): arenas of slots per kind of record with `(index, generation)` handles
//! that carry the agent's key (`arena`), the memory budget charged through one accounting
//! function (`accounting`), the precise mark and sweep collector (`collect`), the host's root
//! table and its release queue (`roots`), strings and atoms (`string`), and the keyed hash map
//! (`map`). Written by `story:heap-values-and-host-surface`.
//!
//! A later kind of record adds its arena here and its two sites (`heap/collect.rs`). Symbols
//! (6.1.5, `#sec-ecmascript-language-types-symbol-type`) have their arena and their record here,
//! [`SymbolRecord`]; the first ones are the well-known symbols of
//! `story:object-model-and-realms`.
//!
//! The agent's GlobalSymbolRegistry (`#sec-globalsymbolregistry-records`; `docs/design/engine.md`,
//! decision 4, "Symbols", items 1 to 3) is here too, beside the atom table:
//! [`SymbolRegistry`], written by `story:symbol-constructor-and-well-known-symbols`. It is a root
//! of the collector ([`trace_registry`]), charged to the memory budget as it grows, and never
//! gives an entry back.

pub(crate) mod accounting;
pub(crate) mod arena;
pub(crate) mod collect;
pub(crate) mod map;
pub(crate) mod roots;
pub(crate) mod string;

use std::borrow::Borrow;
use std::hash::{Hash, Hasher};

use crate::env::EnvironmentRecord;
use crate::host::Host;
use crate::host::agent::Agent;
use crate::object::ObjectRecord;
use crate::realm::RealmRecord;
use crate::script::ScriptRecord;
use crate::value::{ObjectRef, StringRef, SymbolRef, Value};
use accounting::{Accounting, OverBudget};
use arena::{Access, Arena, Handle};
use collect::{Item, Mark};
use map::KeyedMap;
use roots::RootTable;
use string::{Atoms, CodeUnits, HeapString};

use swell_num::siphash::SipBuildHasher;

/// A Symbol record (6.1.5): its [[Description]], a String or undefined.
pub(crate) struct SymbolRecord {
    /// [[Description]].
    pub(crate) description: Option<StringRef>,
    /// `Symbol.for` made it: the GlobalSymbolRegistry holds it, and its [[Description]] is its
    /// [[Key]], which KeyForSymbol answers (`#sec-keyforsymbol`).
    pub(crate) registered: bool,
}

impl SymbolRecord {
    /// Kind site of the collector: the description.
    pub(crate) fn trace(&self, mark: &mut Mark) {
        mark.count("heap/mod.rs SymbolRecord::trace");
        if let Some(description) = self.description {
            mark.value(Value::String(description));
        }
    }

    /// A Symbol with no description, for the tests of the collector.
    #[cfg(test)]
    pub(crate) fn for_test() -> SymbolRecord {
        SymbolRecord {
            description: None,
            registered: false,
        }
    }
}

/// The number of code units.
fn units_len(units: CodeUnits<'_>) -> usize {
    match units {
        CodeUnits::Latin1(bytes) => bytes.len(),
        CodeUnits::Utf16(units) => units.len(),
    }
}

/// The code units hashed in one write.
const REGISTRY_HASH_PIECE: usize = 32;

/// Feed `state` the number of code units, then each unit as two little-endian bytes, in pieces
/// of at most [`REGISTRY_HASH_PIECE`] units: the same bytes for either form, read in place.
fn hash_units<H: Hasher>(units: CodeUnits<'_>, state: &mut H) {
    state.write_usize(units_len(units));
    let mut buffer = [[0u8; 2]; REGISTRY_HASH_PIECE];
    match units {
        CodeUnits::Latin1(bytes) => {
            for piece in bytes.chunks(REGISTRY_HASH_PIECE) {
                for (pair, &b) in buffer.iter_mut().zip(piece) {
                    *pair = [b, 0];
                }
                state.write(buffer.get(..piece.len()).unwrap_or(&[]).as_flattened());
            }
        }
        CodeUnits::Utf16(units) => {
            for piece in units.chunks(REGISTRY_HASH_PIECE) {
                for (pair, &u) in buffer.iter_mut().zip(piece) {
                    *pair = u.to_le_bytes();
                }
                state.write(buffer.get(..piece.len()).unwrap_or(&[]).as_flattened());
            }
        }
    }
}

/// Whether both hold the same code units, compared in place, in either form.
fn units_equal(x: CodeUnits<'_>, y: CodeUnits<'_>) -> bool {
    match (x, y) {
        (CodeUnits::Latin1(x), CodeUnits::Latin1(y)) => x == y,
        (CodeUnits::Utf16(x), CodeUnits::Utf16(y)) => x == y,
        (CodeUnits::Latin1(bytes), CodeUnits::Utf16(units))
        | (CodeUnits::Utf16(units), CodeUnits::Latin1(bytes)) => {
            bytes.len() == units.len() && bytes.iter().zip(units).all(|(&b, &u)| u16::from(b) == u)
        }
    }
}

/// The key of an entry of the registry: the code units of its [[Key]], which the entry owns.
struct RegistryKey(Box<[u16]>);

/// Code units the registry hashes and compares, whatever their form: the key of an entry, and
/// the units of a String `Symbol.for` asks for, read in place.
trait RegistryText {
    fn code_units(&self) -> CodeUnits<'_>;
}

impl RegistryText for RegistryKey {
    fn code_units(&self) -> CodeUnits<'_> {
        CodeUnits::Utf16(&self.0)
    }
}

impl RegistryText for CodeUnits<'_> {
    fn code_units(&self) -> CodeUnits<'_> {
        *self
    }
}

impl Hash for dyn RegistryText + '_ {
    fn hash<S: Hasher>(&self, state: &mut S) {
        hash_units(self.code_units(), state);
    }
}

impl PartialEq for dyn RegistryText + '_ {
    fn eq(&self, other: &Self) -> bool {
        units_equal(self.code_units(), other.code_units())
    }
}

impl Eq for dyn RegistryText + '_ {}

impl Hash for RegistryKey {
    fn hash<S: Hasher>(&self, state: &mut S) {
        hash_units(self.code_units(), state);
    }
}

impl PartialEq for RegistryKey {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for RegistryKey {}

impl<'a> Borrow<dyn RegistryText + 'a> for RegistryKey {
    fn borrow(&self) -> &(dyn RegistryText + 'a) {
        self
    }
}

/// The fewest entries the registry's table grows to.
const MIN_REGISTRY: usize = 16;

/// The map of the registry.
type RegistryTable = KeyedMap<RegistryKey, Handle>;

/// The GlobalSymbolRegistry of the agent (`#sec-globalsymbolregistry-records`; `engine.md`,
/// decision 4, "Symbols", item 2): the code units of each [[Key]] to its [[Symbol]], whose
/// [[Description]] is the key's String. Keyed by the heap's `hash_key` alone (HOST-145), as the
/// atom table is. Every realm of the agent reads it. An entry is never removed, so the table
/// never holds a deleted bucket, and it never grows past its room: before it takes an entry past
/// its room, the room doubles and the table's growth is charged first
/// ([`KeyedMap::table_bytes_for`]); the copy of the key's units is charged with it. No sweep
/// credits either.
pub(crate) struct SymbolRegistry {
    map: RegistryTable,
    /// The entries the table was charged room for.
    room: usize,
}

impl SymbolRegistry {
    /// An empty registry keyed by `hash_key`.
    fn new(hash_key: &[u8; 16]) -> SymbolRegistry {
        SymbolRegistry {
            map: KeyedMap::new(hash_key),
            room: 0,
        }
    }

    /// The Symbol registered for `units`, found in place.
    fn find(&self, units: CodeUnits<'_>) -> Option<Handle> {
        let text: &dyn RegistryText = &units;
        self.map.get(text).copied()
    }

    /// The room the table grows to before it takes another entry, or `None` while it has room.
    fn growth(&self) -> Option<usize> {
        (self.map.len() >= self.room).then(|| self.room.saturating_mul(2).max(MIN_REGISTRY))
    }

    /// The bytes of the table with room for `to` entries less those of the table now.
    fn growth_bytes(&self, to: usize) -> u64 {
        RegistryTable::table_bytes_for(to).saturating_sub(RegistryTable::table_bytes_for(self.room))
    }

    /// Reserve room for `to` entries (its bytes charged before, [`SymbolRegistry::growth_bytes`]).
    fn grow(&mut self, to: usize) -> Result<(), OverBudget> {
        self.map.reserve(to.saturating_sub(self.map.len()))?;
        self.room = to;
        Ok(())
    }

    /// The bytes of the table's buckets that hold no entry, which the live reading leaves out:
    /// the table charged less a bucket, its entry and its control byte, per entry.
    fn room_bytes(&self) -> u64 {
        let table = RegistryTable::table_bytes_for(self.room);
        let entry =
            u64::try_from(size_of::<(RegistryKey, Handle)>().saturating_add(1)).unwrap_or(u64::MAX);
        let entries = u64::try_from(self.map.len()).unwrap_or(u64::MAX);
        table.saturating_sub(entries.saturating_mul(entry))
    }

    /// The counting build: the bytes the table's room was charged.
    #[cfg(test)]
    pub(crate) fn room_charge(&self) -> u64 {
        RegistryTable::table_bytes_for(self.room)
    }

    /// The number of entries.
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.map.len()
    }

    /// Report every registered Symbol; each one's trace reports its key, the [[Description]].
    fn trace(&self, mark: &mut Mark) {
        for &handle in self.map.values() {
            mark.push(Item::Symbol(handle));
        }
    }
}

/// Root site of the collector: the GlobalSymbolRegistry (`engine.md`, decision 4, "Collector"
/// and "Symbols", item 2). Every registered Symbol, and its key with it, lives as long as the
/// agent.
pub(crate) fn trace_registry<H: Host>(agent: &Agent<H>, mark: &mut Mark) {
    mark.count("heap/mod.rs trace_registry");
    agent.heap.registry.trace(mark);
}

/// The bytes allocated since the last collection that start one at the next safe point, when the
/// live reading is smaller than this. Above it, the threshold is the live reading after the last
/// collection ([`Heap::live_bytes`]), so a heap that grows is collected after it has doubled
/// (`engine.md`, decision 4: "a threshold that grows with the live size"), and garbage is
/// collected before the charged reading, which keeps the arenas' capacity, reaches the budget.
/// Triggers count bytes, never time.
pub(crate) const MIN_TRIGGER_BYTES: u64 = 1 << 20;

#[cfg(test)]
thread_local! {
    /// The collections run, counted in [`Heap::collected`], in the counting build of the tests;
    /// a test resets it before it reads it.
    pub(crate) static COLLECTIONS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    /// The String records stored, counted in [`Heap::new_string_record`], in the counting build
    /// of the tests; a test resets it before it reads it.
    pub(crate) static STRINGS_MADE: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// The heap of one agent.
pub(crate) struct Heap {
    /// The agent's key, carried by every handle into this heap.
    pub(crate) key: u32,
    pub(crate) accounting: Accounting,
    pub(crate) strings: Arena<HeapString>,
    pub(crate) symbols: Arena<SymbolRecord>,
    pub(crate) objects: Arena<ObjectRecord>,
    pub(crate) realms: Arena<RealmRecord>,
    pub(crate) environments: Arena<EnvironmentRecord>,
    pub(crate) scripts: Arena<ScriptRecord>,
    pub(crate) roots: RootTable,
    pub(crate) atoms: Atoms,
    /// The GlobalSymbolRegistry ([`SymbolRegistry`]).
    pub(crate) registry: SymbolRegistry,
    /// The hasher of every other keyed map of this heap (`map::KeyedMap`, HOST-145): keyed by
    /// `hash_key` and by nothing else, as the atom table's is. Its `Debug` never prints the key.
    pub(crate) hasher: SipBuildHasher,
    /// The bytes allocated since the last collection at which a safe point collects.
    pub(crate) trigger_bytes: u64,
    /// The counting build: the vectors the last collection made and its pushes past a
    /// vector's capacity.
    #[cfg(test)]
    pub(crate) collect_allocations: u64,
    /// The counting build: the most entries the last collection's worklists held at once.
    #[cfg(test)]
    pub(crate) collect_peak_work: usize,
}

impl Heap {
    /// An empty heap for the agent `key`, with the budget `memory_bytes` and its hash maps keyed
    /// by `hash_key`.
    pub(crate) fn new(key: u32, memory_bytes: u64, hash_key: &[u8; 16]) -> Heap {
        Heap {
            key,
            accounting: Accounting::new(memory_bytes),
            strings: Arena::new(key),
            symbols: Arena::new(key),
            objects: Arena::new(key),
            realms: Arena::new(key),
            environments: Arena::new(key),
            scripts: Arena::new(key),
            roots: RootTable::new(key),
            atoms: Atoms::new(hash_key),
            registry: SymbolRegistry::new(hash_key),
            hasher: SipBuildHasher::new(hash_key),
            trigger_bytes: MIN_TRIGGER_BYTES,
            #[cfg(test)]
            collect_allocations: 0,
            #[cfg(test)]
            collect_peak_work: 0,
        }
    }

    /// The counting build: the floor of the six arenas ([`Arena::ledger_floor`]).
    #[cfg(test)]
    pub(crate) fn ledger_floor(&self) -> u64 {
        [
            self.strings.ledger_floor(),
            self.symbols.ledger_floor(),
            self.objects.ledger_floor(),
            self.realms.ledger_floor(),
            self.environments.ledger_floor(),
            self.scripts.ledger_floor(),
        ]
        .iter()
        .fold(0u64, |sum, bytes| sum.saturating_add(*bytes))
    }

    /// The counting build: the capacity charge, the bytes that stay charged while the agent
    /// lives: the six arenas' capacity, each slot with its workspace ([`Arena::capacity_bytes`]),
    /// the atom table's room ([`Atoms::room_charge`]) and the registry's
    /// ([`SymbolRegistry::room_charge`]).
    #[cfg(test)]
    pub(crate) fn capacity_charge(&self) -> u64 {
        [
            self.strings.capacity_bytes(),
            self.symbols.capacity_bytes(),
            self.objects.capacity_bytes(),
            self.realms.capacity_bytes(),
            self.environments.capacity_bytes(),
            self.scripts.capacity_bytes(),
            self.atoms.room_charge(),
            self.registry.room_charge(),
        ]
        .iter()
        .fold(0u64, |sum, bytes| sum.saturating_add(*bytes))
    }

    /// The counting build: the slots of the six arenas, live or free.
    #[cfg(test)]
    pub(crate) fn slot_total(&self) -> usize {
        self.strings.slot_count()
            + self.symbols.slot_count()
            + self.objects.slot_count()
            + self.realms.slot_count()
            + self.environments.slot_count()
            + self.scripts.slot_count()
    }

    /// The counting build: the pushes past capacity onto the six free lists so far.
    #[cfg(test)]
    pub(crate) fn free_list_growths(&self) -> u64 {
        self.strings.growths
            + self.symbols.growths
            + self.objects.growths
            + self.realms.growths
            + self.environments.growths
            + self.scripts.growths
    }

    /// The bytes the heap is charged now (`heap_bytes()`, the reading `memory_bytes` bounds).
    pub(crate) fn bytes(&self) -> u64 {
        self.accounting.used()
    }

    /// The live reading (`live_bytes()`): the charged reading less the room, the slots of
    /// every arena that hold no record with the workspace of every slot, and the buckets of the
    /// atom table and of the registry that hold no entry. Between entries that is what the live
    /// records own with their slots, property maps included, and what the entries of the atom
    /// table and of the registry own.
    pub(crate) fn live_bytes(&self) -> u64 {
        let room = [
            self.strings.room_bytes(),
            self.symbols.room_bytes(),
            self.objects.room_bytes(),
            self.realms.room_bytes(),
            self.environments.room_bytes(),
            self.scripts.room_bytes(),
            self.atoms.room_bytes(),
            self.registry.room_bytes(),
        ]
        .iter()
        .fold(0u64, |sum, bytes| sum.saturating_add(*bytes));
        self.accounting.used().saturating_sub(room)
    }

    /// Clear the mark words of every arena, after a mark phase that found a stale handle.
    pub(crate) fn clear_marks(&mut self) {
        self.strings.clear_marks();
        self.symbols.clear_marks();
        self.objects.clear_marks();
        self.realms.clear_marks();
        self.environments.clear_marks();
        self.scripts.clear_marks();
    }

    /// Whether a safe point should collect: the bytes allocated since the last collection
    /// passed the threshold.
    pub(crate) fn collection_due(&self) -> bool {
        self.accounting.since_collection() >= self.trigger_bytes
    }

    /// After a collection and every credit it made: count again from zero, against a threshold
    /// of the live reading, which counts neither what was freed nor the arenas' room.
    pub(crate) fn collected(&mut self) {
        #[cfg(test)]
        COLLECTIONS.with(|collections| collections.set(collections.get().saturating_add(1)));
        self.accounting.collected();
        self.trigger_bytes = self.live_bytes().max(MIN_TRIGGER_BYTES);
    }

    /// Store an object record that owns `owned_bytes` beside its slot.
    pub(crate) fn new_object(
        &mut self,
        record: ObjectRecord,
        owned_bytes: u64,
    ) -> Result<ObjectRef, OverBudget> {
        let handle = store(&mut self.accounting, &mut self.objects, record, owned_bytes)?;
        Ok(ObjectRef { handle })
    }

    /// [`Heap::new_object`] of a record whose `owned_bytes` the caller charged already
    /// ([`store_charged`]): over the budget, or when the arena cannot grow, nothing is stored
    /// and the caller credits them; stored, the sweep that frees the record credits them.
    pub(crate) fn new_object_charged(
        &mut self,
        record: ObjectRecord,
        owned_bytes: u64,
    ) -> Result<ObjectRef, OverBudget> {
        let handle = store_charged(&mut self.accounting, &mut self.objects, record, owned_bytes)?;
        Ok(ObjectRef { handle })
    }

    /// Store a string record whose units take `owned_bytes`.
    pub(crate) fn new_string_record(
        &mut self,
        string: HeapString,
        owned_bytes: u64,
    ) -> Result<StringRef, OverBudget> {
        let handle = store(&mut self.accounting, &mut self.strings, string, owned_bytes)?;
        #[cfg(test)]
        STRINGS_MADE.with(|made| made.set(made.get().saturating_add(1)));
        Ok(StringRef { handle })
    }

    /// Store a Symbol record with the description `description`.
    pub(crate) fn new_symbol(
        &mut self,
        description: Option<StringRef>,
    ) -> Result<SymbolRef, OverBudget> {
        let handle = store(
            &mut self.accounting,
            &mut self.symbols,
            SymbolRecord {
                description,
                registered: false,
            },
            0,
        )?;
        Ok(SymbolRef { handle })
    }

    /// The Symbol of the GlobalSymbolRegistry whose [[Key]] has the code units of `key`
    /// (`#sec-symbol.for`, steps 2 to 7; `engine.md`, decision 4, "Symbols", items 2 and 3): found
    /// in place, or made, registered and stored, with `key` as its [[Description]]. A new entry
    /// charges first the growth of the table when it has no room, which stays charged, then
    /// the copy of the key's units, then the Symbol's record (`store`); none of them is ever
    /// credited, since no entry is removed. Over the budget nothing is registered. `Err` for a
    /// stale `key`.
    pub(crate) fn symbol_for(
        &mut self,
        key: StringRef,
    ) -> Result<Result<SymbolRef, OverBudget>, Access> {
        let units = self.strings.get(key.handle)?.units();
        if let Some(handle) = self.registry.find(units) {
            return Ok(Ok(SymbolRef { handle }));
        }
        let length = units_len(units);
        let entry_bytes = u64::try_from(length.saturating_mul(2)).unwrap_or(u64::MAX);
        let growth = self.registry.growth();
        let growth_bytes = growth.map_or(0, |to| self.registry.growth_bytes(to));
        if self
            .accounting
            .charge(growth_bytes.saturating_add(entry_bytes))
            .is_err()
        {
            return Ok(Err(OverBudget));
        }
        if let Some(to) = growth
            && self.registry.grow(to).is_err()
        {
            self.accounting
                .credit(growth_bytes.saturating_add(entry_bytes));
            return Ok(Err(OverBudget));
        }
        let mut copy: Vec<u16> = Vec::new();
        if copy.try_reserve_exact(length).is_err() {
            self.accounting.credit(entry_bytes);
            return Ok(Err(OverBudget));
        }
        match units {
            CodeUnits::Latin1(bytes) => copy.extend(bytes.iter().map(|&b| u16::from(b))),
            CodeUnits::Utf16(units) => copy.extend_from_slice(units),
        }
        let record = SymbolRecord {
            description: Some(key),
            registered: true,
        };
        let handle = match store(&mut self.accounting, &mut self.symbols, record, 0) {
            Ok(handle) => handle,
            Err(over) => {
                self.accounting.credit(entry_bytes);
                return Ok(Err(over));
            }
        };
        // The table has room for the entry, so the insertion allocates nothing.
        if let Err(over) = self
            .registry
            .map
            .insert(RegistryKey(copy.into_boxed_slice()), handle)
        {
            self.accounting.credit(entry_bytes);
            return Ok(Err(over));
        }
        Ok(Ok(SymbolRef { handle }))
    }

    /// Whether `value` names a live record of this heap. A primitive always does; a BigInt
    /// never does yet, since no arena holds them before slice 12.
    pub(crate) fn check_value(&self, value: Value) -> Result<(), Access> {
        match value {
            Value::Undefined | Value::Null | Value::Boolean(_) | Value::Number(_) => Ok(()),
            Value::String(string) => self.strings.get(string.handle).map(|_| ()),
            Value::Object(object) => self.objects.get(object.handle).map(|_| ()),
            Value::Symbol(symbol) => self.symbols.get(symbol.handle).map(|_| ()),
            Value::BigInt(bigint) => Err(self.no_arena(bigint.handle)),
        }
    }

    /// The answer for a handle of a kind that has no arena yet.
    fn no_arena(&self, handle: Handle) -> Access {
        if handle.key == self.key {
            Access::Stale
        } else {
            Access::Foreign
        }
    }
}

/// Charge `owned_bytes` to the budget, and the growth of `arena` when it has no room (each new
/// slot with its workspace, `arena::slot_bytes`), then store `value` in `arena`. Over the
/// budget, or when the arena cannot grow, nothing is charged or stored. A record in a freed
/// slot or a slot of the arena's capacity is charged what it owns only. This and
/// [`store_charged`], for a value whose owned bytes are charged already, are the only way into an arena.
pub(crate) fn store<T>(
    accounting: &mut Accounting,
    arena: &mut Arena<T>,
    value: T,
    owned_bytes: u64,
) -> Result<Handle, OverBudget> {
    let bytes = arena.growth_bytes().saturating_add(owned_bytes);
    accounting.charge(bytes)?;
    match arena.allocate(value, owned_bytes) {
        Ok(handle) => Ok(handle),
        Err(_) => {
            accounting.credit(bytes);
            Err(OverBudget)
        }
    }
}

/// [`store`] of a `value` whose `owned_bytes` are charged already (a script record, whose code
/// and source copy were charged as they grew): only the growth of `arena` is charged here. Over
/// the budget, or when the arena cannot grow, nothing more is charged, nothing is stored, and the
/// caller credits `owned_bytes`; stored, the slot owns them and the sweep that frees it credits
/// them.
pub(crate) fn store_charged<T>(
    accounting: &mut Accounting,
    arena: &mut Arena<T>,
    value: T,
    owned_bytes: u64,
) -> Result<Handle, OverBudget> {
    let growth = arena.growth_bytes();
    accounting.charge(growth)?;
    match arena.allocate(value, owned_bytes) {
        Ok(handle) => Ok(handle),
        Err(_) => {
            accounting.credit(growth);
            Err(OverBudget)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::host::hooks::test_host;
    use crate::value::StringRef;
    use crate::{Outcome, Value};

    /// The GlobalSymbolRegistry (`engine.md`, decision 4, "Symbols", items 2 and 3): 1,000 keys
    /// registered through `Heap::symbol_for`, held by nothing else, one entry each; after every
    /// registration the table takes at most the bytes its room was charged
    /// (`KeyedMap::table_bytes`), and the charged reading holds the capacity charge with the
    /// registry's room. A collection keeps every entry, every Symbol and its key: its
    /// [[Description]] is the String it was made with, and `symbol_for` of a new String of the
    /// same units answers it (the entry holds 16-bit units, the String is read in place in its
    /// Latin-1 form); a key of 16-bit units is found the same way.
    #[test]
    fn the_registry_is_charged_by_its_room_and_is_a_root() {
        let mut limits = test_host::limits();
        limits.memory_bytes = 1 << 26;
        let mut agent = test_host::agent_with(limits);
        let mut made = Vec::new();
        for i in 0..1_000 {
            let key = agent.heap.new_string_from_str(&format!("key {i}")).unwrap();
            let symbol = agent.heap.symbol_for(key).unwrap().unwrap();
            assert_eq!(agent.heap.symbol_for(key).unwrap(), Ok(symbol));
            let registry = &agent.heap.registry;
            assert!(registry.map.table_bytes() <= registry.room_charge(), "{i}");
            made.push((key, symbol));
        }
        assert_eq!(agent.heap.registry.len(), 1_000);
        assert!(agent.heap.bytes() >= agent.heap.capacity_charge());

        assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
        assert_eq!(agent.heap.registry.len(), 1_000);
        for (i, &(key, symbol)) in made.iter().enumerate() {
            let record = agent.heap.symbols.get(symbol.handle).unwrap();
            assert!(record.registered);
            assert_eq!(record.description, Some(key));
            assert!(agent.heap.check_value(Value::String(key)).is_ok());
            let units: Vec<u16> = format!("key {i}").encode_utf16().collect();
            let latin1 = agent.heap.new_string_from_units(&units).unwrap();
            assert_eq!(agent.heap.symbol_for(latin1).unwrap(), Ok(symbol));
        }
        // A key of 16-bit units finds the entry of the same units.
        let wide: Vec<u16> = "\u{20AC} key".encode_utf16().collect();
        let first = agent.heap.new_string_from_units(&wide).unwrap();
        let symbol = agent.heap.symbol_for(first).unwrap().unwrap();
        let second: StringRef = agent.heap.new_string_from_units(&wide).unwrap();
        assert_eq!(agent.heap.symbol_for(second).unwrap(), Ok(symbol));
        assert_eq!(agent.heap.registry.len(), 1_001);
    }
}
