//! Map and Set instances (ECMA-262 24.1.4, `#sec-properties-of-map-instances`; 24.2.5,
//! `#sec-properties-of-set-instances`) and the Map and Set Iterators (24.1.5,
//! `#sec-map-iterator-objects`; 24.2.6, `#sec-set-iterator-objects`), written by
//! `story:map-and-set`.
//!
//! A Map or a Set is the kind [`Collection`]: which of the two it is, and its [[MapData]] or
//! [[SetData]], the entry table of `story:keyed-entry-table` (`object/keyed_entries.rs`), a
//! Set's entries with the value `undefined`. The kind lies in a box, charged to the memory
//! budget with the object before either exists (the slot's owned bytes, which the sweep that
//! frees the object credits), so the record of every object keeps its size
//! (`object/generator.rs`, `RECORD_BYTES`); the table charges its own storage as it grows.
//!
//! An iterator is the kind [`CollectionIterator`]: the closure of CreateMapIterator
//! (`#sec-createmapiterator`) and CreateSetIterator (`#sec-createsetiterator`) held as slots,
//! as the String Iterator holds its state (`builtins/iterator.rs`): the brand of its prototype,
//! the collection it iterates until it is done, its cursor registered with the collection's
//! table, and its kind. Its cursor is closed when `next` answers done; an iterator the script
//! drops unfinished has its cursor released by the collector, whose pass between the mark
//! phase and the sweep (`heap/collect.rs`, `Arena::visit_marked_mut`) calls
//! [`Collection::release_cursors`] on every marked collection with the mark of each cursor's
//! owner.
//!
//! The table is used through [`Agent::with_entry_table`], which takes it out of the record for
//! an operation that needs the agent and puts it back on every way out. No operation of the
//! table runs code or passes a safe point, so no collection can run while the table is out.
//!
//! Both kinds take every internal method by its ordinary arm (`object/mod.rs`): their
//! [[GetOwnProperty]], [[DefineOwnProperty]] and [[Set]] are ordinary.

use alloc::boxed::Box;

use crate::heap::arena::Handle;
use crate::heap::collect::Mark;
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::host::{ErrorKind, Host, Termination};
use crate::object::keyed_entries::{CursorId, EntryTable};
use crate::object::{ObjectKind, ObjectRecord};
use crate::realm::intrinsics::Intrinsic;
use crate::value::{ObjectRef, Value};

/// Which collection a [`Collection`] is, and which prototype brands an iterator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CollectionKind {
    /// A Map, with [[MapData]].
    Map,
    /// A Set, with [[SetData]].
    Set,
}

impl CollectionKind {
    /// The constructor's name, for the messages of its `TypeError`s.
    pub(crate) fn name(self) -> &'static str {
        match self {
            CollectionKind::Map => "Map",
            CollectionKind::Set => "Set",
        }
    }
}

/// The kind of an iteration (`#sec-createmapiterator`, `#sec-createsetiterator`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum IterationKind {
    /// key: the key.
    Key,
    /// value: the value (a Set's element).
    Value,
    /// key+value: an Array of the key and the value (a Set's element twice).
    KeyValue,
}

/// The slots of a Map or a Set.
pub(crate) struct Collection {
    /// Whether it is a Map or a Set.
    pub(crate) kind: CollectionKind,
    /// [[MapData]] or [[SetData]].
    pub(crate) table: EntryTable,
}

impl Collection {
    /// An empty Map or Set.
    pub(crate) fn new(kind: CollectionKind) -> Collection {
        Collection {
            kind,
            table: EntryTable::new(),
        }
    }

    /// The bytes of the box of the kind, charged with the object.
    pub(crate) fn box_bytes() -> u64 {
        u64::try_from(size_of::<Collection>()).unwrap_or(u64::MAX)
    }

    /// Kind site of the collector: every live key (a String key as its atom) and value of the
    /// table, each once; allocates nothing.
    pub(crate) fn trace(&self, mark: &mut Mark) {
        mark.count("object/collection.rs Collection::trace");
        for value in self.table.traced() {
            mark.value(value);
        }
    }

    /// For the collector's pass after the mark phase: release the cursors whose owner
    /// `is_live` answers false for (an iterator the script dropped), and answer the bytes the
    /// table gave back, for the collector to credit.
    pub(crate) fn release_cursors(&mut self, is_live: &dyn Fn(Handle) -> bool) -> u64 {
        self.table.release_cursors(is_live)
    }

    /// A Map, for the tests of the collector.
    #[cfg(test)]
    pub(crate) fn for_test() -> Collection {
        Collection::new(CollectionKind::Map)
    }
}

/// The slots of a Map or Set Iterator: the state the closure of CreateMapIterator or
/// CreateSetIterator captures.
pub(crate) struct CollectionIterator {
    /// The [[GeneratorBrand]]: "%MapIteratorPrototype%" for `Map`, "%SetIteratorPrototype%"
    /// for `Set`.
    pub(crate) brand: CollectionKind,
    /// The collection, until `next` answers done; then none, and done for ever.
    pub(crate) collection: Option<ObjectRef>,
    /// The cursor registered with the collection's table, while it is open.
    pub(crate) cursor: Option<CursorId>,
    /// The kind of the iteration.
    pub(crate) kind: IterationKind,
}

impl CollectionIterator {
    /// Kind site of the collector: the collection it iterates, until it is done.
    pub(crate) fn trace(&self, mark: &mut Mark) {
        mark.count("object/collection.rs CollectionIterator::trace");
        if let Some(collection) = self.collection {
            mark.object(collection);
        }
    }

    /// An iterator of `collection`, for the tests of the collector.
    #[cfg(test)]
    pub(crate) fn for_test(collection: ObjectRef) -> CollectionIterator {
        CollectionIterator {
            brand: CollectionKind::Map,
            collection: Some(collection),
            cursor: None,
            kind: IterationKind::KeyValue,
        }
    }
}

impl<H: Host> Agent<H> {
    /// OrdinaryCreateFromConstructor ( constructor, "%Map.prototype%" or "%Set.prototype%", «
    /// [[MapData]] » or « [[SetData]] » ) with the slot set to a new empty List
    /// (`#sec-map-iterable` and `#sec-set-iterable`, steps 2 and 3): the box of the kind is
    /// charged with the object before either exists; over the budget the run ends in
    /// `Terminated(MemoryBudget)`.
    pub(crate) fn create_collection(
        &mut self,
        constructor: ObjectRef,
        kind: CollectionKind,
    ) -> Completion<ObjectRef> {
        let default = match kind {
            CollectionKind::Map => Intrinsic::MapPrototype,
            CollectionKind::Set => Intrinsic::SetPrototype,
        };
        let prototype = self.get_prototype_from_constructor(constructor, default)?;
        let record = ObjectRecord::new(
            Some(prototype),
            ObjectKind::Collection(Box::new(Collection::new(kind))),
        );
        match self.heap.new_object(record, Collection::box_bytes()) {
            Ok(object) => Ok(object),
            Err(_) => Err(self.terminate(Termination::MemoryBudget)),
        }
    }

    /// The Map `value` is, or the `TypeError` of RequireInternalSlot ( value, [[MapData]] )
    /// (`#sec-requireinternalslot`) for any other value; `method` names the caller in the
    /// message.
    pub(crate) fn map_data(&mut self, value: Value, method: &str) -> Completion<ObjectRef> {
        self.collection_data(value, CollectionKind::Map, method)
    }

    /// The Set `value` is, or the `TypeError` of RequireInternalSlot ( value, [[SetData]] ).
    pub(crate) fn set_data(&mut self, value: Value, method: &str) -> Completion<ObjectRef> {
        self.collection_data(value, CollectionKind::Set, method)
    }

    /// The collection of `kind` that `value` is, or the `TypeError` of RequireInternalSlot.
    fn collection_data(
        &mut self,
        value: Value,
        kind: CollectionKind,
        method: &str,
    ) -> Completion<ObjectRef> {
        if let Value::Object(object) = value
            && matches!(
                &self.engine_object(object)?.kind,
                ObjectKind::Collection(collection) if collection.kind == kind
            )
        {
            return Ok(object);
        }
        let message = alloc::format!("{method} requires that 'this' be a {}", kind.name());
        Err(self.throw_error(ErrorKind::TypeError, &message))
    }

    /// Run `operation` on the table of the collection `collection` (a Map or a Set, checked by
    /// the caller), taken out of the record while it runs and put back on every way out. The
    /// operation runs no code and passes no safe point, so no collection sees the record
    /// without its table; any other object here is a broken invariant.
    pub(crate) fn with_entry_table<T>(
        &mut self,
        collection: ObjectRef,
        operation: impl FnOnce(&mut Self, &mut EntryTable) -> Completion<T>,
    ) -> Completion<T> {
        let mut table = match &mut self.engine_object_mut(collection)?.kind {
            ObjectKind::Collection(slots) => core::mem::take(&mut slots.table),
            _ => return Err(self.fault()),
        };
        let answer = operation(self, &mut table);
        match self.heap.objects.get_mut(collection.handle) {
            Ok(ObjectRecord {
                kind: ObjectKind::Collection(slots),
                ..
            }) => {
                slots.table = table;
                answer
            }
            _ => Err(self.fault()),
        }
    }
}
