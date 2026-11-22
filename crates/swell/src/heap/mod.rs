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

pub(crate) mod accounting;
pub(crate) mod arena;
pub(crate) mod collect;
pub(crate) mod map;
pub(crate) mod roots;
pub(crate) mod string;

use crate::env::EnvironmentRecord;
use crate::object::ObjectRecord;
use crate::realm::RealmRecord;
use crate::script::ScriptRecord;
use crate::value::{ObjectRef, StringRef, SymbolRef, Value};
use accounting::{Accounting, OverBudget};
use arena::{Access, Arena, Handle};
use collect::Mark;
use roots::RootTable;
use string::{Atoms, HeapString};

use swell_num::siphash::SipBuildHasher;

/// A Symbol record (6.1.5): its [[Description]], a String or undefined.
pub(crate) struct SymbolRecord {
    /// [[Description]].
    pub(crate) description: Option<StringRef>,
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
        SymbolRecord { description: None }
    }
}

/// The bytes allocated since the last collection that start one at the next safe point, when the
/// live reading is smaller than this. Above it, the threshold is the live reading after the last
/// collection ([`Heap::live_bytes`]), so a heap that grows is collected after it has doubled
/// (`engine.md`, decision 4: "a threshold that grows with the live size"), and garbage is
/// collected before the charged reading, which keeps the arenas' capacity, reaches the budget.
/// Triggers count bytes, never time.
const MIN_TRIGGER_BYTES: u64 = 1 << 20;

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
    /// and the atom table's room ([`Atoms::room_charge`]).
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
    /// every arena that hold no record with the workspace of every slot, and the atom table's
    /// buckets that hold no entry. Between entries that is what the live records own with
    /// their slots, property maps included, and what the atom table's entries own.
    pub(crate) fn live_bytes(&self) -> u64 {
        let room = [
            self.strings.room_bytes(),
            self.symbols.room_bytes(),
            self.objects.room_bytes(),
            self.realms.room_bytes(),
            self.environments.room_bytes(),
            self.scripts.room_bytes(),
            self.atoms.room_bytes(),
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

    /// Store a string record whose units take `owned_bytes`.
    pub(crate) fn new_string_record(
        &mut self,
        string: HeapString,
        owned_bytes: u64,
    ) -> Result<StringRef, OverBudget> {
        let handle = store(&mut self.accounting, &mut self.strings, string, owned_bytes)?;
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
            SymbolRecord { description },
            0,
        )?;
        Ok(SymbolRef { handle })
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
/// [`Accounting::charge`] are the only way into an arena.
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
