//! WeakMap and WeakSet instances (ECMA-262 24.3.4, `#sec-properties-of-weakmap-instances`;
//! 24.4.4, `#sec-properties-of-weakset-instances`), written by `story:weak-map-and-weak-set`.
//!
//! A WeakMap or a WeakSet is the kind [`WeakCollection`]: which of the two it is, and its
//! [[WeakMapData]] or [[WeakSetData]], a keyed hash map (`heap/map.rs`, the heap's hasher) from
//! the key's handle, an object or a Symbol that CanBeHeldWeakly (`#sec-canbeheldweakly`), to the
//! entry, whose value is `undefined` in a WeakSet. The order of the entries is never observable,
//! so the map's order is free. The kind lies in a box, charged to the memory budget with the
//! object before either exists (the slot's owned bytes, which the sweep that frees the object
//! credits), so the record of every object keeps its size (`object/generator.rs`,
//! `RECORD_BYTES`). Each growth of the map is charged before it, by
//! [`KeyedMap::table_bytes_for`] of its room for one more entry, and the table's bytes are
//! recorded with the object (`Agent::record_grown`), so the sweep that frees it credits them.
//! The map never shrinks: an entry deleted or cleared leaves its room charged.
//!
//! The collector holds the entries weakly (`heap/collect.rs`): [`WeakCollection::trace`] reports
//! no key; for a WeakMap it reports the value of each entry whose key is marked and puts each
//! other entry on the pending list of its key's slot (linear ephemeron marking), through the
//! entry's link; when the key is marked, [`WeakCollection::release`] reports the value and
//! answers the next link. After marking, [`WeakCollection::clear_unmarked`] drops every entry
//! whose key is not marked. A WeakSet's entries have no value; they are only cleared.
//!
//! Both kinds take every internal method by its ordinary arm (`object/mod.rs`).

use alloc::boxed::Box;
use core::cell::Cell;

use crate::heap::arena::Handle;
use crate::heap::collect::{Item, Mark};
use crate::heap::map::KeyedMap;
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::host::{ErrorKind, Host, Termination};
use crate::object::{ObjectKind, ObjectRecord};
use crate::realm::intrinsics::Intrinsic;
use crate::value::{ObjectRef, Value};
use swell_num::siphash::SipBuildHasher;

/// Which weak collection a [`WeakCollection`] is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WeakKind {
    /// A WeakMap, with [[WeakMapData]].
    Map,
    /// A WeakSet, with [[WeakSetData]].
    Set,
}

impl WeakKind {
    /// The constructor's name, for the messages of its `TypeError`s.
    pub(crate) fn name(self) -> &'static str {
        match self {
            WeakKind::Map => "WeakMap",
            WeakKind::Set => "WeakSet",
        }
    }
}

/// The key of an entry: the handle of a value that CanBeHeldWeakly, compared by identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum WeakKey {
    /// An object.
    Object(Handle),
    /// A Symbol not in the GlobalSymbolRegistry.
    Symbol(Handle),
}

impl WeakKey {
    /// The key of `value`, an object or a Symbol; `None` for any other value. Whether a
    /// Symbol can be held weakly is the caller's check (`Agent::can_be_held_weakly`).
    pub(crate) fn of(value: Value) -> Option<WeakKey> {
        match value {
            Value::Object(object) => Some(WeakKey::Object(object.handle)),
            Value::Symbol(symbol) => Some(WeakKey::Symbol(symbol.handle)),
            _ => None,
        }
    }

    /// The key of a marked item, an object or a Symbol; `None` for any other item.
    pub(crate) fn of_item(item: Item) -> Option<WeakKey> {
        match item {
            Item::Object(handle) => Some(WeakKey::Object(handle)),
            Item::Symbol(handle) => Some(WeakKey::Symbol(handle)),
            _ => None,
        }
    }

    /// The key as the collector's item.
    pub(crate) fn item(self) -> Item {
        match self {
            WeakKey::Object(handle) => Item::Object(handle),
            WeakKey::Symbol(handle) => Item::Symbol(handle),
        }
    }
}

/// An entry: its key, its value (set in place, so an overwrite does not touch the map) and its
/// link on the pending list of its key's slot during a mark phase.
pub(crate) struct WeakEntry {
    /// The key, as the map holds it.
    key: WeakKey,
    /// The value; `undefined` in a WeakSet.
    value: Cell<Value>,
    /// The slot of the objects arena plus one of the next weak table on the same pending list,
    /// 0 for the end; read only while the entry is on a list.
    next: Cell<u32>,
}

/// The slots of a WeakMap or a WeakSet.
pub(crate) struct WeakCollection {
    /// Whether it is a WeakMap or a WeakSet.
    pub(crate) kind: WeakKind,
    /// [[WeakMapData]] or [[WeakSetData]].
    table: KeyedMap<WeakKey, WeakEntry>,
    /// The entries the map's table holds before it grows (its capacity when it was last
    /// made), 0 before it has one.
    room: usize,
    /// The bytes charged for the map's table, recorded with the object.
    charged: u64,
    /// The planted fault of the tests of the collector: keep every entry, its key and its
    /// value, as a strong table would.
    #[cfg(test)]
    keeps_every_entry: bool,
}

impl WeakCollection {
    /// An empty WeakMap or WeakSet whose map hashes with `hasher`, the heap's.
    pub(crate) fn new(kind: WeakKind, hasher: SipBuildHasher) -> WeakCollection {
        WeakCollection {
            kind,
            table: KeyedMap::with_hasher(hasher),
            room: 0,
            charged: 0,
            #[cfg(test)]
            keeps_every_entry: false,
        }
    }

    /// The bytes of the box of the kind, charged with the object.
    pub(crate) fn box_bytes() -> u64 {
        u64::try_from(size_of::<WeakCollection>()).unwrap_or(u64::MAX)
    }

    /// The entries, for the tests of the collector.
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.table.len()
    }

    /// The value of the entry of `key`, if there is one.
    pub(crate) fn get(&self, key: WeakKey) -> Option<Value> {
        self.table.get(&key).map(|entry| entry.value.get())
    }

    /// Kind site of the collector: no key; for a WeakMap, the value of each entry whose key is
    /// marked, and each other entry whose value holds a handle put on the pending list of its
    /// key's slot ([`Mark::pend`]); a WeakSet reports nothing. Allocates nothing.
    pub(crate) fn trace(&self, mark: &mut Mark) {
        mark.count("object/weak_collection.rs WeakCollection::trace");
        #[cfg(test)]
        if self.keeps_every_entry {
            for entry in self.table.values() {
                mark.push(entry.key.item());
                mark.value(entry.value.get());
            }
            return;
        }
        if self.kind == WeakKind::Set {
            return;
        }
        for entry in self.table.values() {
            let value = entry.value.get();
            if matches!(
                value,
                Value::Undefined | Value::Null | Value::Boolean(_) | Value::Number(_)
            ) {
                continue;
            }
            let key = entry.key.item();
            if mark.is_marked(key) {
                mark.value(value);
            } else {
                entry.next.set(mark.pend(key));
            }
        }
    }

    /// For the mark phase, when `key` is marked and this table is on its pending list: report
    /// the value of the entry of `key` and answer the entry's link, the next table on the list;
    /// `None` when the table has no entry of `key`, a broken invariant.
    pub(crate) fn release(&self, key: WeakKey, mark: &mut Mark) -> Option<u32> {
        let entry = self.table.get(&key)?;
        mark.value(entry.value.get());
        Some(entry.next.get())
    }

    /// For the collector's pass after the mark phase: drop every entry whose key is not
    /// marked, an object key by `objects` and a Symbol key by `symbols`, and answer the bytes
    /// given back for the collector to credit: 0, since the map keeps its room, charged until
    /// the object is freed. Allocates nothing.
    pub(crate) fn clear_unmarked(
        &mut self,
        objects: &dyn Fn(Handle) -> bool,
        symbols: &dyn Fn(Handle) -> bool,
    ) -> u64 {
        #[cfg(test)]
        if self.keeps_every_entry {
            return 0;
        }
        self.table.retain(|key, _| match *key {
            WeakKey::Object(handle) => objects(handle),
            WeakKey::Symbol(handle) => symbols(handle),
        });
        0
    }

    /// A WeakMap whose one entry has `object` as its key and its value, for the layout test of
    /// the collector.
    #[cfg(test)]
    pub(crate) fn for_test(object: ObjectRef) -> WeakCollection {
        let mut table = WeakCollection::new(WeakKind::Map, SipBuildHasher::new(&[0; 16]));
        table.insert_for_test(WeakKey::Object(object.handle), Value::Object(object));
        table
    }

    /// Store `value` under `key` with no charge, for the tests of the collector.
    #[cfg(test)]
    pub(crate) fn insert_for_test(&mut self, key: WeakKey, value: Value) {
        let inserted = self.table.insert(key, WeakEntry::new(key, value));
        assert!(inserted.is_ok());
    }
}

impl WeakEntry {
    /// A new entry, on no pending list.
    fn new(key: WeakKey, value: Value) -> WeakEntry {
        WeakEntry {
            key,
            value: Cell::new(value),
            next: Cell::new(0),
        }
    }
}

impl<H: Host> Agent<H> {
    /// OrdinaryCreateFromConstructor ( constructor, "%WeakMap.prototype%" or
    /// "%WeakSet.prototype%", « [[WeakMapData]] » or « [[WeakSetData]] » ) with the slot set to
    /// a new empty List (`#sec-weakmap-iterable` and `#sec-weakset-iterable`, steps 2 and 3):
    /// the box of the kind is charged with the object before either exists; over the budget the
    /// run ends in `Terminated(MemoryBudget)`.
    pub(crate) fn create_weak_collection(
        &mut self,
        constructor: ObjectRef,
        kind: WeakKind,
    ) -> Completion<ObjectRef> {
        let default = match kind {
            WeakKind::Map => Intrinsic::WeakMapPrototype,
            WeakKind::Set => Intrinsic::WeakSetPrototype,
        };
        let prototype = self.get_prototype_from_constructor(constructor, default)?;
        let table = WeakCollection::new(kind, self.heap.hasher);
        let record =
            ObjectRecord::new(Some(prototype), ObjectKind::WeakCollection(Box::new(table)));
        match self.heap.new_object(record, WeakCollection::box_bytes()) {
            Ok(object) => Ok(object),
            Err(_) => Err(self.terminate(Termination::MemoryBudget)),
        }
    }

    /// The weak collection of `kind` that `value` is, or the `TypeError` of
    /// RequireInternalSlot ( value, [[WeakMapData]] or [[WeakSetData]] )
    /// (`#sec-requireinternalslot`); `method` names the caller in the message.
    pub(crate) fn weak_collection_of(
        &mut self,
        value: Value,
        kind: WeakKind,
        method: &str,
    ) -> Completion<ObjectRef> {
        if let Value::Object(object) = value
            && matches!(
                &self.engine_object(object)?.kind,
                ObjectKind::WeakCollection(table) if table.kind == kind
            )
        {
            return Ok(object);
        }
        let message = alloc::format!("{method} requires that 'this' be a {}", kind.name());
        Err(self.throw_error(ErrorKind::TypeError, &message))
    }

    /// The key of `value` if CanBeHeldWeakly ( value ) is true, else `None`.
    pub(crate) fn weak_key(&mut self, value: Value) -> Completion<Option<WeakKey>> {
        if self.can_be_held_weakly(value)? {
            Ok(WeakKey::of(value))
        } else {
            Ok(None)
        }
    }

    /// Run `operation` on the table of the weak collection `collection`, checked by the caller;
    /// any other object is a broken invariant. The operation runs no code.
    fn with_weak_table<T>(
        &mut self,
        collection: ObjectRef,
        operation: impl FnOnce(&mut WeakCollection) -> T,
    ) -> Completion<T> {
        let answer = match self.heap.objects.get_mut(collection.handle) {
            Ok(ObjectRecord {
                kind: ObjectKind::WeakCollection(table),
                ..
            }) => Some(operation(table)),
            _ => None,
        };
        answer.ok_or_else(|| self.fault())
    }

    /// The value of the entry of `key` in `collection`, if there is one.
    pub(crate) fn weak_get(
        &mut self,
        collection: ObjectRef,
        key: WeakKey,
    ) -> Completion<Option<Value>> {
        self.with_weak_table(collection, |table| table.get(key))
    }

    /// Remove the entry of `key` from `collection`; whether there was one. The map keeps its
    /// room.
    pub(crate) fn weak_delete(&mut self, collection: ObjectRef, key: WeakKey) -> Completion<bool> {
        self.with_weak_table(collection, |table| table.table.remove(&key).is_some())
    }

    /// Set the value of the entry of `key` in `collection` to `value`, or append the entry. A
    /// map with no spare room grows: the bytes of its table for one more entry than its room
    /// ([`KeyedMap::table_bytes_for`]), less what it holds charged, are charged before it is
    /// made; once it is made, what its room then says is kept charged, the rest credited, and
    /// what was kept is recorded with the object for the sweep that frees it. Over the budget
    /// the run ends in `Terminated(MemoryBudget)` and nothing stays charged.
    pub(crate) fn weak_insert(
        &mut self,
        collection: ObjectRef,
        key: WeakKey,
        value: Value,
    ) -> Completion<()> {
        let precharged = self.with_weak_table(collection, |table| {
            if let Some(entry) = table.table.get(&key) {
                entry.value.set(value);
                return None;
            }
            Some(if table.table.spare() == 0 {
                KeyedMap::<WeakKey, WeakEntry>::table_bytes_for(table.room.saturating_add(1))
                    .saturating_sub(table.charged)
            } else {
                0
            })
        })?;
        let Some(precharged) = precharged else {
            return Ok(());
        };
        self.charge_bytes(precharged)?;
        let grown = self.with_weak_table(collection, |table| {
            table.table.insert(key, WeakEntry::new(key, value)).ok()?;
            let room = table.table.len().saturating_add(table.table.spare());
            table.room = table.room.max(room);
            let bytes = KeyedMap::<WeakKey, WeakEntry>::table_bytes_for(table.room);
            let grown = bytes.saturating_sub(table.charged);
            table.charged = table.charged.saturating_add(grown);
            Some(grown)
        })?;
        let Some(grown) = grown else {
            self.heap.accounting.credit(precharged);
            return Err(self.terminate(Termination::MemoryBudget));
        };
        if grown > precharged {
            // The map took more than its layout says: charged now.
            if self.heap.accounting.charge(grown - precharged).is_err() {
                self.record_grown(collection, precharged)?;
                return Err(self.terminate(Termination::MemoryBudget));
            }
        } else {
            self.heap.accounting.credit(precharged - grown);
        }
        if grown > 0 {
            self.record_grown(collection, grown)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{WeakCollection, WeakEntry, WeakKey, WeakKind};
    use crate::heap::map::KeyedMap;
    use crate::host::hooks::test_host;
    use crate::host::hooks::test_host::NoHooks;
    use crate::object::ordinary::Ordinary;
    use crate::object::{ObjectKind, ObjectRecord};
    use crate::realm::intrinsics::Intrinsic;
    use crate::value::{ObjectRef, Value};
    use crate::{Agent, Outcome, Parsed, Source};
    use alloc::boxed::Box;
    use alloc::format;
    use alloc::vec::Vec;

    /// A new ordinary object with no prototype.
    fn ordinary(agent: &mut Agent<NoHooks>) -> ObjectRef {
        let record = ObjectRecord::new(None, ObjectKind::Ordinary(Ordinary::for_test()));
        agent.heap.new_object(record, 0).unwrap()
    }

    /// A new empty weak table of `kind` with no prototype, charged as the constructor charges
    /// it.
    fn new_table(agent: &mut Agent<NoHooks>, kind: WeakKind) -> ObjectRef {
        let table = WeakCollection::new(kind, agent.heap.hasher);
        let record = ObjectRecord::new(None, ObjectKind::WeakCollection(Box::new(table)));
        agent
            .heap
            .new_object(record, WeakCollection::box_bytes())
            .unwrap()
    }

    /// The entries of the weak table `table`.
    fn entries_of(agent: &Agent<NoHooks>, table: ObjectRef) -> usize {
        match &agent.heap.objects.get(table.handle).unwrap().kind {
            ObjectKind::WeakCollection(table) => table.len(),
            _ => panic!("{table:?} is no weak table"),
        }
    }

    /// Plant the fault "keep every entry" in the weak table `table`.
    fn plant_keep_every_entry(agent: &mut Agent<NoHooks>, table: ObjectRef) {
        match &mut agent.heap.objects.get_mut(table.handle).unwrap().kind {
            ObjectKind::WeakCollection(table) => table.keeps_every_entry = true,
            _ => panic!("{table:?} is no weak table"),
        }
    }

    /// The test of the clearing holds the collector to it: a WeakMap and a WeakSet, each with
    /// an entry whose key nothing else reaches, lose it in one collection, and the WeakMap's
    /// value is freed; the same tables with the planted fault "keep every entry" keep both
    /// entries, the key and the value, so the check that the entry is cleared fails on them.
    #[test]
    fn a_planted_keep_every_entry_fails_the_check_of_the_clearing() {
        for planted in [false, true] {
            let mut agent = test_host::agent_with(test_host::limits());
            let key = ordinary(&mut agent);
            let value = ordinary(&mut agent);
            let map = new_table(&mut agent, WeakKind::Map);
            let set = new_table(&mut agent, WeakKind::Set);
            for (table, value) in [(map, Value::Object(value)), (set, Value::Undefined)] {
                agent
                    .weak_insert(table, WeakKey::Object(key.handle), value)
                    .unwrap();
                if planted {
                    plant_keep_every_entry(&mut agent, table);
                }
            }
            let _roots = [
                agent.heap.roots.add(Value::Object(map)),
                agent.heap.roots.add(Value::Object(set)),
            ];
            assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
            let cleared = entries_of(&agent, map) == 0
                && entries_of(&agent, set) == 0
                && agent.heap.objects.get(key.handle).is_err()
                && agent.heap.objects.get(value.handle).is_err();
            assert_eq!(cleared, !planted, "planted {planted}");
        }
    }

    /// The table's bytes: appending entries to an empty WeakMap raises `heap_bytes()` by
    /// exactly the bytes of the table its room says ([`KeyedMap::table_bytes_for`]), each growth
    /// charged; setting the value of a key already there charges nothing; deleting keeps the
    /// room charged, and appending again into it charges nothing; once the map is unreachable,
    /// one collection credits the table and the box. The keys are made and rooted before the
    /// first reading.
    #[test]
    fn a_weak_map_charges_its_table_and_its_sweep_credits_it() {
        const KEYS: usize = 40;
        let mut agent = test_host::agent_with(test_host::limits());
        let keys: Vec<ObjectRef> = (0..KEYS).map(|_| ordinary(&mut agent)).collect();
        let _key_roots: Vec<_> = keys
            .iter()
            .map(|&key| agent.heap.roots.add(Value::Object(key)))
            .collect();
        let map = new_table(&mut agent, WeakKind::Map);
        let map_root = agent.heap.roots.add(Value::Object(map));
        let before = agent.heap_bytes().unwrap();
        for &key in &keys {
            agent
                .weak_insert(map, WeakKey::Object(key.handle), Value::Number(1.0))
                .unwrap();
        }
        let filled = agent.heap_bytes().unwrap();
        let table = KeyedMap::<WeakKey, WeakEntry>::table_bytes_for(
            KeyedMap::<WeakKey, WeakEntry>::full_capacity_for(KEYS),
        );
        assert_eq!(filled - before, table, "{KEYS} entries");
        for &key in &keys {
            agent
                .weak_insert(map, WeakKey::Object(key.handle), Value::Number(2.0))
                .unwrap();
        }
        assert_eq!(agent.heap_bytes().unwrap(), filled, "every value set again");
        for &key in &keys[..KEYS / 2] {
            assert!(agent.weak_delete(map, WeakKey::Object(key.handle)).unwrap());
        }
        for &key in &keys[..KEYS / 2] {
            agent
                .weak_insert(map, WeakKey::Object(key.handle), Value::Number(3.0))
                .unwrap();
        }
        assert_eq!(entries_of(&agent, map), KEYS);
        assert_eq!(
            agent.heap_bytes().unwrap(),
            filled,
            "deleted and appended again"
        );
        drop(map_root);
        assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
        let after = agent.heap_bytes().unwrap();
        assert_eq!(
            filled - after,
            table + WeakCollection::box_bytes(),
            "the table and the box credited"
        );
    }

    /// The box of the kind: `create_weak_collection` charges [`WeakCollection::box_bytes`] with
    /// the object, for a WeakMap and a WeakSet, each made into a slot of the objects arena that
    /// a collection freed before the first reading, so no growth of the arena is in the
    /// difference.
    #[test]
    fn a_new_weak_collection_charges_its_box() {
        let (mut agent, realm) = test_host::agent_with_realm();
        test_host::enter(&mut agent, &realm);
        let handle = agent.heap.roots.get_realm(&realm.0).unwrap();
        for (kind, which) in [
            (WeakKind::Map, Intrinsic::WeakMap),
            (WeakKind::Set, Intrinsic::WeakSet),
        ] {
            let constructor = agent.intrinsic(handle, which).unwrap();
            ordinary(&mut agent);
            assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
            let before = agent.heap_bytes().unwrap();
            agent.create_weak_collection(constructor, kind).unwrap();
            let after = agent.heap_bytes().unwrap();
            assert_eq!(after - before, WeakCollection::box_bytes(), "{kind:?}");
        }
    }

    /// Evaluate `source` in a fresh realm's agent after `setup`; the steps of the entry of
    /// `source`.
    fn steps_of(setup: &str, source: &str) -> u64 {
        let (mut agent, realm) = test_host::agent_with_realm();
        for script in [setup, source] {
            let Ok(Parsed::Parsed(script)) = agent.parse_script(&realm, Source::Utf8(script), None)
            else {
                panic!("{script} did not parse");
            };
            let outcome = agent.evaluate_script(&script);
            assert!(matches!(outcome, Ok(Outcome::Normal(_))), "{outcome:?}");
        }
        agent.last_entry_steps().unwrap()
    }

    /// The step of the constructor per entry: `new WeakSet(a)` and `new WeakMap(a)` over an
    /// Array of `N` entries take, beyond the same call over an empty Array, exactly `N` times
    /// the steps of one entry, measured here: the loop's own step, the iterator's `next` and
    /// the adder's call. Without the loop's step the difference is `N` less.
    #[test]
    fn the_constructors_charge_a_step_per_entry() {
        const N: usize = 50;
        for (constructor, entry, per_entry) in [("WeakSet", "{}", 3), ("WeakMap", "[{}, 1]", 3)] {
            let fill =
                |n: usize| format!("var a = []; for (var i = 0; i < {n}; i++) a.push({entry}); 1");
            let call = format!("new {constructor}(a); 1");
            let empty = steps_of(&fill(0), &call);
            let full = steps_of(&fill(N), &call);
            assert_eq!(
                full - empty,
                (N as u64) * per_entry,
                "{constructor}: {empty} steps empty, {full} with {N} entries"
            );
        }
    }
}
