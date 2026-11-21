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
use arena::{Access, Arena, Handle, slot_bytes};
use collect::Mark;
use roots::RootTable;
use string::{Atoms, HeapString};

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
/// heap is smaller than this. Above it, the threshold is the live size after the last collection,
/// so a heap that grows is collected after it has doubled (`engine.md`, decision 4: "a threshold
/// that grows with the live size"). Triggers count bytes, never time.
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
    /// The bytes allocated since the last collection at which a safe point collects.
    pub(crate) trigger_bytes: u64,
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
            trigger_bytes: MIN_TRIGGER_BYTES,
        }
    }

    /// The bytes the heap is charged now (`heap_bytes()`).
    pub(crate) fn bytes(&self) -> u64 {
        self.accounting.used()
    }

    /// Whether a safe point should collect: the bytes allocated since the last collection
    /// passed the threshold.
    pub(crate) fn collection_due(&self) -> bool {
        self.accounting.since_collection() >= self.trigger_bytes
    }

    /// After a collection: count again from zero, against a threshold of the live size.
    pub(crate) fn collected(&mut self) {
        self.accounting.collected();
        self.trigger_bytes = self.accounting.used().max(MIN_TRIGGER_BYTES);
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

/// Charge the slot and `owned_bytes` to the budget, then store `value` in `arena`. Over the
/// budget, or when the arena cannot grow, nothing is charged or stored. This and
/// [`Accounting::charge`] are the only way into an arena.
pub(crate) fn store<T>(
    accounting: &mut Accounting,
    arena: &mut Arena<T>,
    value: T,
    owned_bytes: u64,
) -> Result<Handle, OverBudget> {
    let bytes = slot_bytes::<T>().saturating_add(owned_bytes);
    accounting.charge(bytes)?;
    match arena.allocate(value, bytes) {
        Ok(handle) => Ok(handle),
        Err(_) => {
            accounting.credit(bytes);
            Err(OverBudget)
        }
    }
}
