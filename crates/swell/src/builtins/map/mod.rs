//! The Map objects of slice 11 (ECMA-262 24.1, `#sec-map-objects`), written by
//! `story:map-and-set`: `%Map%` (`Map ( [ iterable ] )`, `#sec-map-iterable`) with
//! AddEntriesFromIterable (`#sec-add-entries-from-iterable`) and `get Map [ %Symbol.species% ]`
//! (`#sec-get-map-%symbol.species%`); `%Map.prototype%` (`#sec-properties-of-the-map-prototype-object`)
//! with `clear`, `constructor`, `delete`, `entries`, `forEach`, `get`, `has`, `keys`, `set`,
//! `get size`, `values`, `[%Symbol.iterator%]` (the same function object as `entries`) and
//! `[%Symbol.toStringTag%]`; and `%MapIteratorPrototype%` (`#sec-%mapiteratorprototype%-object`)
//! with `next` and its `[%Symbol.toStringTag%]`. The steps a Set shares with a Map (`clear`,
//! `delete`, `has`, `size`, `forEach` and the iterators) are written here once, by the kind of
//! the collection, and `builtins/set/mod.rs` calls them.
//!
//! [[MapData]] is the entry table of the object (`object/collection.rs`): the table keeps the
//! insertion order, canonicalizes `-0` to `+0` and compares keys by SameValueZero, and each of
//! its operations charges its steps before its work. A walk over the table is a cursor
//! registered with it: an entry added before the cursor reaches it is visited, one deleted
//! before it is not, and `clear` sends the cursor to the end. `forEach` opens a cursor with no
//! owner and closes it on every way out, normal or abrupt, before it answers; an iterator opens
//! a cursor owned by the iterator object when it is made, closes it when `next` answers done,
//! and the collector releases the cursor of an iterator the script dropped unfinished
//! (`heap/collect.rs`).
//!
//! The pinned draft states the iterators as generators made by CreateIteratorFromClosure whose
//! `next` is GeneratorResume with the brand "%MapIteratorPrototype%" or
//! "%SetIteratorPrototype%"; as the String Iterator does (`builtins/iterator.rs`), an iterator
//! holds the state the closure captures in slots of its own, and `next` does what a resumption
//! does: a `TypeError` for a this value that is not an iterator of its brand, an entry per call,
//! and done from the end on, for ever.
//!
//! AddEntriesFromIterable keeps [[Iterator]], [[NextMethod]], the adder, the entry and its key
//! and value on the kept stack until it returns, as `Object.fromEntries` does
//! (`builtins/object/from_entries.rs`), charges a step per iteration before its work, and
//! closes the iterator on an abrupt completion of its own steps (IfAbruptCloseIterator); a
//! throw of `next` ends it with no close, and a termination closes nothing.
//!
//! `getOrInsert`, `getOrInsertComputed` and `Map.groupBy` (`story:map-upsert-and-group-by`) are
//! the group `map/upsert.rs`: the variant [`MapBuiltin::Upsert`], the two methods in
//! [`PROTOTYPE_METHODS`] in the order of the clauses, and `groupBy` on `%Map%`.

pub(crate) mod upsert;

use crate::builtins::Method;
use crate::builtins::map::upsert::UpsertBuiltin;
use crate::builtins::object::argument;
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::host::{ErrorKind, Host};
use crate::object::ObjectKind;
use crate::object::builtin_function::BuiltinId;
use crate::object::collection::{CollectionIterator, CollectionKind, IterationKind};
use crate::object::ordinary::OrdinaryData;
use crate::object::property::{Key, Property, Slot};
use crate::realm::intrinsics::{Intrinsic, Making};
use crate::value::{ObjectRef, Value};

/// The builtins of this module, by their steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MapBuiltin {
    /// `Map ( [ iterable ] )` (24.1.1.1).
    Constructor,
    /// `get Map [ %Symbol.species% ]` (24.1.2.3).
    Species,
    /// `Map.prototype.clear ( )` (24.1.3.1).
    Clear,
    /// `Map.prototype.delete ( key )` (24.1.3.3).
    Delete,
    /// `Map.prototype.entries ( )` (24.1.3.4).
    Entries,
    /// `Map.prototype.forEach ( callbackfn [ , thisArg ] )` (24.1.3.5).
    ForEach,
    /// `Map.prototype.get ( key )` (24.1.3.6).
    Get,
    /// `Map.prototype.has ( key )` (24.1.3.7).
    Has,
    /// `Map.prototype.keys ( )` (24.1.3.8).
    Keys,
    /// `Map.prototype.set ( key, value )` (24.1.3.9).
    Set,
    /// `get Map.prototype.size` (24.1.3.10).
    Size,
    /// `Map.prototype.values ( )` (24.1.3.11).
    Values,
    /// `%MapIteratorPrototype%.next ( )` (24.1.5.2.1).
    IteratorNext,
    /// `Map.groupBy`, `Map.prototype.getOrInsert` and `getOrInsertComputed` (`map/upsert.rs`).
    Upsert(UpsertBuiltin),
}

/// The methods of `%Map.prototype%` after `clear` and `constructor`, in the order of the
/// clauses; `entries` is made apart, for `[%Symbol.iterator%]`, and `size` is an accessor.
const PROTOTYPE_METHODS: &[Method<MapBuiltin>] = &[
    ("delete", 1, MapBuiltin::Delete),
    ("entries", 0, MapBuiltin::Entries),
    ("forEach", 1, MapBuiltin::ForEach),
    ("get", 1, MapBuiltin::Get),
    (
        "getOrInsert",
        2,
        MapBuiltin::Upsert(UpsertBuiltin::GetOrInsert),
    ),
    (
        "getOrInsertComputed",
        2,
        MapBuiltin::Upsert(UpsertBuiltin::GetOrInsertComputed),
    ),
    ("has", 1, MapBuiltin::Has),
    ("keys", 0, MapBuiltin::Keys),
    ("set", 2, MapBuiltin::Set),
];

/// `%Map%`, `%Map.prototype%`, `%MapIteratorPrototype%`, `%Set%`, `%Set.prototype%` and
/// `%SetIteratorPrototype%` of a realm.
#[derive(Clone, Copy, Debug)]
pub(crate) struct CollectionIntrinsics {
    pub(crate) map: ObjectRef,
    pub(crate) map_prototype: ObjectRef,
    pub(crate) map_iterator_prototype: ObjectRef,
    pub(crate) set: ObjectRef,
    pub(crate) set_prototype: ObjectRef,
    pub(crate) set_iterator_prototype: ObjectRef,
}

impl<H: Host> Agent<H> {
    /// The Map and Set intrinsics of the realm being made; `iterator_prototype` is
    /// `%Iterator.prototype%`, the [[Prototype]] of both iterator prototypes (24.1.5.2,
    /// 24.2.6.2).
    pub(crate) fn create_collection_intrinsics(
        &mut self,
        making: Making,
        iterator_prototype: ObjectRef,
    ) -> Completion<CollectionIntrinsics> {
        let (map, map_prototype) = self.create_map_constructor(making)?;
        let map_iterator_prototype = self.create_collection_iterator_prototype(
            making,
            iterator_prototype,
            BuiltinId::Map(MapBuiltin::IteratorNext),
            "Map Iterator",
        )?;
        let (set, set_prototype) = self.create_set_constructor(making)?;
        let set_iterator_prototype = self.create_collection_iterator_prototype(
            making,
            iterator_prototype,
            BuiltinId::Set(crate::builtins::set::SetBuiltin::IteratorNext),
            "Set Iterator",
        )?;
        Ok(CollectionIntrinsics {
            map,
            map_prototype,
            map_iterator_prototype,
            set,
            set_prototype,
            set_iterator_prototype,
        })
    }

    /// `%Map%` and `%Map.prototype%` of the realm being made.
    fn create_map_constructor(&mut self, making: Making) -> Completion<(ObjectRef, ObjectRef)> {
        // 24.1.3: "is an ordinary object", "is not a Map", whose [[Prototype]] is
        // %Object.prototype%.
        let prototype =
            self.ordinary_object_create(Some(making.object_prototype), OrdinaryData::Plain)?;
        // 24.1.3.1 `clear` comes before 24.1.3.2 `constructor`.
        self.define_builtin_method(
            making,
            prototype,
            "clear",
            0,
            BuiltinId::Map(MapBuiltin::Clear),
        )?;
        // 24.1.2: "length" 0 (24.1.1.1 has one optional parameter); its "prototype" (24.1.2.2)
        // and the prototype's "constructor" (24.1.3.2).
        let constructor = self.create_builtin_constructor(
            making,
            "Map",
            0,
            BuiltinId::Map(MapBuiltin::Constructor),
            prototype,
        )?;
        // 24.1.2.1 `Map.groupBy ( items, callback )`, "length" 2.
        self.define_builtin_method(
            making,
            constructor,
            "groupBy",
            2,
            BuiltinId::Map(MapBuiltin::Upsert(UpsertBuiltin::GroupBy)),
        )?;
        self.define_species_getter(making, constructor, BuiltinId::Map(MapBuiltin::Species))?;
        let mut entries = None;
        for &(name, length, id) in PROTOTYPE_METHODS {
            let function =
                self.define_builtin_method(making, prototype, name, length, BuiltinId::Map(id))?;
            if id == MapBuiltin::Entries {
                entries = Some(function);
            }
        }
        // 24.1.3.10: an accessor whose set accessor function is undefined.
        self.define_size_getter(making, prototype, BuiltinId::Map(MapBuiltin::Size))?;
        self.define_builtin_method(
            making,
            prototype,
            "values",
            0,
            BuiltinId::Map(MapBuiltin::Values),
        )?;
        // 24.1.3.12: "The initial value of the %Symbol.iterator% property is
        // %Map.prototype.entries%".
        let Some(entries) = entries else {
            return Err(self.fault());
        };
        self.add_property(
            prototype,
            Key::Symbol(making.symbols.iterator),
            Property::data(Value::Object(entries), true, false, true),
        )?;
        // 24.1.3.13: "Map", { [[Writable]]: false, [[Enumerable]]: false, [[Configurable]]: true }.
        self.define_to_string_tag(making, prototype, "Map")?;
        Ok((constructor, prototype))
    }

    /// `get C [ %Symbol.species% ]` of `constructor` (24.1.2.3, 24.2.3.2): an accessor whose
    /// set accessor function is undefined, named "get [Symbol.species]", { [[Enumerable]]:
    /// false, [[Configurable]]: true }.
    pub(crate) fn define_species_getter(
        &mut self,
        making: Making,
        constructor: ObjectRef,
        id: BuiltinId,
    ) -> Completion<()> {
        let species = self.create_builtin(making, "get [Symbol.species]", 0, id)?;
        self.add_property(
            constructor,
            Key::Symbol(making.symbols.species),
            Property {
                slot: Slot::Accessor {
                    get: Some(species),
                    set: None,
                },
                enumerable: false,
                configurable: true,
            },
        )
    }

    /// `get C.prototype.size` on `prototype` (24.1.3.10, 24.2.4.14): an accessor whose set
    /// accessor function is undefined, named "get size", { [[Enumerable]]: false,
    /// [[Configurable]]: true }.
    pub(crate) fn define_size_getter(
        &mut self,
        making: Making,
        prototype: ObjectRef,
        id: BuiltinId,
    ) -> Completion<()> {
        let size = self.create_builtin(making, "get size", 0, id)?;
        let key = self.name_key("size")?;
        self.add_property(
            prototype,
            key,
            Property {
                slot: Slot::Accessor {
                    get: Some(size),
                    set: None,
                },
                enumerable: false,
                configurable: true,
            },
        )
    }

    /// `%MapIteratorPrototype%` or `%SetIteratorPrototype%` (24.1.5.2, 24.2.6.2): an ordinary
    /// object whose [[Prototype]] is `%Iterator.prototype%`, with "next" of "length" 0 whose
    /// steps are `next`, then its `[%Symbol.toStringTag%]` `tag`.
    fn create_collection_iterator_prototype(
        &mut self,
        making: Making,
        iterator_prototype: ObjectRef,
        next: BuiltinId,
        tag: &str,
    ) -> Completion<ObjectRef> {
        let prototype =
            self.ordinary_object_create(Some(iterator_prototype), OrdinaryData::Plain)?;
        self.define_builtin_method(making, prototype, "next", 0, next)?;
        self.define_to_string_tag(making, prototype, tag)?;
        Ok(prototype)
    }

    /// The steps of the builtin `id` of this module.
    pub(crate) fn run_map_builtin(
        &mut self,
        id: MapBuiltin,
        _function: ObjectRef,
        this: Value,
        args: &[Value],
        new_target: Option<ObjectRef>,
    ) -> Completion<Value> {
        let map = CollectionKind::Map;
        match id {
            MapBuiltin::Constructor => self.map_constructor(argument(args, 0), new_target),
            // 24.1.2.3: "1. Return the this value."
            MapBuiltin::Species => Ok(this),
            MapBuiltin::Clear => self.collection_clear(this, map, "Map.prototype.clear"),
            MapBuiltin::Delete => {
                self.collection_delete(this, map, argument(args, 0), "Map.prototype.delete")
            }
            MapBuiltin::Entries => self.collection_iterator_of(
                this,
                map,
                IterationKind::KeyValue,
                "Map.prototype.entries",
            ),
            MapBuiltin::ForEach => self.collection_for_each(
                this,
                map,
                argument(args, 0),
                argument(args, 1),
                "Map.prototype.forEach",
            ),
            MapBuiltin::Get => self.map_get(this, argument(args, 0)),
            MapBuiltin::Has => {
                self.collection_has(this, map, argument(args, 0), "Map.prototype.has")
            }
            MapBuiltin::Keys => {
                self.collection_iterator_of(this, map, IterationKind::Key, "Map.prototype.keys")
            }
            MapBuiltin::Set => self.map_set(this, argument(args, 0), argument(args, 1)),
            MapBuiltin::Size => self.collection_size(this, map, "get Map.prototype.size"),
            MapBuiltin::Values => {
                self.collection_iterator_of(this, map, IterationKind::Value, "Map.prototype.values")
            }
            MapBuiltin::IteratorNext => self.collection_iterator_next(this, map),
            MapBuiltin::Upsert(id) => self.run_upsert_builtin(id, this, args),
        }
    }

    /// `Map ( [ iterable ] )` (`#sec-map-iterable`).
    fn map_constructor(
        &mut self,
        iterable: Value,
        new_target: Option<ObjectRef>,
    ) -> Completion<Value> {
        // 1. If NewTarget is undefined, throw a TypeError exception.
        let Some(new_target) = new_target else {
            return Err(self.throw_error(ErrorKind::TypeError, "Constructor Map requires 'new'"));
        };
        // 2. Let map be ? OrdinaryCreateFromConstructor(NewTarget, "%Map.prototype%", «
        // [[MapData]] »). 3. Set map.[[MapData]] to a new empty List.
        let map = self.create_collection(new_target, CollectionKind::Map)?;
        // 4. If iterable is either undefined or null, return map.
        if matches!(iterable, Value::Undefined | Value::Null) {
            return Ok(Value::Object(map));
        }
        let mark = self.keep(Value::Object(map));
        let filled = self.fill_collection(map, iterable, "set");
        self.release(mark);
        filled?;
        Ok(Value::Object(map))
    }

    /// Steps 5 to 7 of `Map ( [ iterable ] )` (`adder_name` "set") and of `Set ( [ iterable ]
    /// )` (`adder_name` "add"): Get of the adder, the `TypeError` when it is not callable, and
    /// the loop, with `collection` kept by the caller, which releases what is kept here.
    pub(crate) fn fill_collection(
        &mut self,
        collection: ObjectRef,
        iterable: Value,
        adder_name: &str,
    ) -> Completion<()> {
        // 5. Let adder be ? Get(map, "set").
        let key = self.name_key(adder_name)?;
        let adder = self.get(collection, key)?;
        // 6. If IsCallable(adder) is false, throw a TypeError exception.
        if !self.is_callable(adder)? {
            let message =
                alloc::format!("the '{adder_name}' of the new collection is not a function");
            return Err(self.throw_error(ErrorKind::TypeError, &message));
        }
        self.keep(adder);
        if adder_name == "add" {
            self.add_values_from_iterable(collection, iterable, adder)
        } else {
            // 7. Return ? AddEntriesFromIterable(map, iterable, adder).
            self.add_entries_from_iterable_with(collection, iterable, adder)
        }
    }

    /// AddEntriesFromIterable ( target, iterable, adder ) (`#sec-add-entries-from-iterable`),
    /// with an adder that is a function the script can reach (the `set` a Map constructor
    /// read, a WeakMap's `set`); `iterable` and `adder` are held by the caller, which releases
    /// what is kept here.
    pub(crate) fn add_entries_from_iterable_with(
        &mut self,
        target: ObjectRef,
        iterable: Value,
        adder: Value,
    ) -> Completion<()> {
        // 1. Let iteratorRecord be ? GetIterator(iterable, sync).
        let mut record = self.get_iterator(iterable)?;
        self.keep(Value::Object(record.iterator));
        self.keep(record.next_method);
        // 2. Repeat,
        loop {
            self.charge_steps(1)?;
            // 2.a. Let next be ? IteratorStepValue(iteratorRecord). 2.b. If next is done,
            // return target.
            let Some(next) = self.iterator_step_value(&mut record)? else {
                return Ok(());
            };
            // 2.c. If next is not an Object, then let error be ThrowCompletion(a newly created
            // TypeError object) and return ? IteratorClose(iteratorRecord, error).
            let Value::Object(entry) = next else {
                let error = self.throw_error(
                    ErrorKind::TypeError,
                    "an entry of the iterable is not an object",
                );
                return Err(self.close_iterator_after(&record, error));
            };
            // 2.d to 2.i, IfAbruptCloseIterator on each abrupt completion.
            let mark = self.keep(next);
            let added = self.add_entry_with(target, entry, adder);
            self.release(mark);
            if let Err(abrupt) = added {
                return Err(self.close_iterator_after(&record, abrupt));
            }
        }
    }

    /// Steps 2.d to 2.h of AddEntriesFromIterable, the entry kept by the caller, which
    /// releases the key and the value kept here.
    fn add_entry_with(
        &mut self,
        target: ObjectRef,
        entry: ObjectRef,
        adder: Value,
    ) -> Completion<()> {
        // 2.d. Let key be Completion(Get(next, "0")).
        let key = self.get(entry, Key::Index(0))?;
        self.keep(key);
        // 2.f. Let value be Completion(Get(next, "1")).
        let value = self.get(entry, Key::Index(1))?;
        self.keep(value);
        // 2.h. Let status be Completion(Call(adder, target, « key, value »)).
        self.call_value(adder, Value::Object(target), &[key, value])?;
        Ok(())
    }

    /// `Map.prototype.get ( key )` (`#sec-map.prototype.get`).
    fn map_get(&mut self, this: Value, key: Value) -> Completion<Value> {
        // 1. Let M be the this value. 2. Perform ? RequireInternalSlot(M, [[MapData]]).
        let map = self.map_data(this, "Map.prototype.get")?;
        // 3. Set key to CanonicalizeKeyedCollectionKey(key). 4. For each Record { [[Key]],
        // [[Value]] } p of M.[[MapData]], if p.[[Key]] is not empty and SameValue(p.[[Key]],
        // key) is true, return p.[[Value]]. 5. Return undefined.
        self.with_entry_table(map, |agent, table| {
            let found = table.find(agent, key)?;
            Ok(found
                .and_then(|position| table.entry_at(position))
                .map_or(Value::Undefined, |(_, value)| value))
        })
    }

    /// `Map.prototype.set ( key, value )` (`#sec-map.prototype.set`).
    fn map_set(&mut self, this: Value, key: Value, value: Value) -> Completion<Value> {
        // 1. Let M be the this value. 2. Perform ? RequireInternalSlot(M, [[MapData]]).
        let map = self.map_data(this, "Map.prototype.set")?;
        // 3. Set key to CanonicalizeKeyedCollectionKey(key). 4. For each p of M.[[MapData]],
        // if SameValue(p.[[Key]], key), set p.[[Value]] to value and return M. 5. Let p be the
        // Record { [[Key]]: key, [[Value]]: value }. 6. Append p to M.[[MapData]].
        self.with_entry_table(map, |agent, table| table.insert(agent, key, value))?;
        // 7. Return M.
        Ok(this)
    }

    /// `Map.prototype.clear ( )` (`#sec-map.prototype.clear`) and `Set.prototype.clear ( )`
    /// (`#sec-set.prototype.clear`): every entry emptied; a walk that runs on visits only the
    /// entries added after.
    pub(crate) fn collection_clear(
        &mut self,
        this: Value,
        kind: CollectionKind,
        method: &str,
    ) -> Completion<Value> {
        let collection = self.collection_of(this, kind, method)?;
        self.with_entry_table(collection, |agent, table| table.clear(agent))?;
        Ok(Value::Undefined)
    }

    /// `Map.prototype.delete ( key )` (`#sec-map.prototype.delete`) and `Set.prototype.delete
    /// ( value )` (`#sec-set.prototype.delete`).
    pub(crate) fn collection_delete(
        &mut self,
        this: Value,
        kind: CollectionKind,
        key: Value,
        method: &str,
    ) -> Completion<Value> {
        let collection = self.collection_of(this, kind, method)?;
        let deleted = self.with_entry_table(collection, |agent, table| table.delete(agent, key))?;
        Ok(Value::Boolean(deleted))
    }

    /// `Map.prototype.has ( key )` (`#sec-map.prototype.has`) and `Set.prototype.has ( value
    /// )` (`#sec-set.prototype.has`).
    pub(crate) fn collection_has(
        &mut self,
        this: Value,
        kind: CollectionKind,
        key: Value,
        method: &str,
    ) -> Completion<Value> {
        let collection = self.collection_of(this, kind, method)?;
        let found = self.with_entry_table(collection, |agent, table| table.find(agent, key))?;
        Ok(Value::Boolean(found.is_some()))
    }

    /// `get Map.prototype.size` (`#sec-get-map.prototype.size`) and `get Set.prototype.size`
    /// (`#sec-get-set.prototype.size`): the live entries.
    pub(crate) fn collection_size(
        &mut self,
        this: Value,
        kind: CollectionKind,
        method: &str,
    ) -> Completion<Value> {
        let collection = self.collection_of(this, kind, method)?;
        let size = match &self.engine_object(collection)?.kind {
            ObjectKind::Collection(slots) => slots.table.len(),
            _ => return Err(self.fault()),
        };
        let Ok(size) = u32::try_from(size) else {
            return Err(self.fault());
        };
        Ok(Value::Number(f64::from(size)))
    }

    /// RequireInternalSlot ( this, [[MapData]] or [[SetData]] ) by `kind`.
    fn collection_of(
        &mut self,
        this: Value,
        kind: CollectionKind,
        method: &str,
    ) -> Completion<ObjectRef> {
        match kind {
            CollectionKind::Map => self.map_data(this, method),
            CollectionKind::Set => self.set_data(this, method),
        }
    }

    /// `Map.prototype.forEach ( callbackfn [ , thisArg ] )` (`#sec-map.prototype.foreach`) and
    /// `Set.prototype.forEach` (`#sec-set.prototype.foreach`): a cursor with no owner,
    /// registered for the loop and closed on every way out before the completion is answered.
    pub(crate) fn collection_for_each(
        &mut self,
        this: Value,
        kind: CollectionKind,
        callback: Value,
        this_arg: Value,
        method: &str,
    ) -> Completion<Value> {
        // 1. Let M be the this value. 2. Perform ? RequireInternalSlot(M, [[MapData]]).
        let collection = self.collection_of(this, kind, method)?;
        // 3. If IsCallable(callbackfn) is false, throw a TypeError exception.
        if !self.is_callable(callback)? {
            let message = alloc::format!("{method}: the callback is not a function");
            return Err(self.throw_error(ErrorKind::TypeError, &message));
        }
        let cursor =
            self.with_entry_table(collection, |agent, table| table.open_cursor(agent, None))?;
        let walked = self.for_each_steps(collection, kind, cursor, callback, this_arg);
        let closed = self.with_entry_table(collection, |agent, table| {
            table.close_cursor(agent, cursor);
            Ok(())
        });
        // 7. Return undefined; an abrupt completion of the loop is answered as it is.
        walked?;
        closed?;
        Ok(Value::Undefined)
    }

    /// Steps 4 to 6 of `forEach`: for each entry the cursor reaches, Call(callbackfn, thisArg,
    /// « e.[[Value]], e.[[Key]], M ») for a Map and « e, e, S » for a Set. The collection and
    /// the callback are held by the activation; key and value become the arguments of the call
    /// with no safe point between.
    fn for_each_steps(
        &mut self,
        collection: ObjectRef,
        kind: CollectionKind,
        cursor: crate::object::keyed_entries::CursorId,
        callback: Value,
        this_arg: Value,
    ) -> Completion<()> {
        loop {
            self.charge_steps(1)?;
            let entry =
                self.with_entry_table(collection, |agent, table| table.advance(agent, cursor))?;
            let Some((key, value)) = entry else {
                return Ok(());
            };
            let first = match kind {
                CollectionKind::Map => value,
                CollectionKind::Set => key,
            };
            self.call_value(callback, this_arg, &[first, key, Value::Object(collection)])?;
        }
    }

    /// `Map.prototype.entries`, `keys` and `values`, and `Set.prototype.entries` and `values`:
    /// RequireInternalSlot of the this value, then CreateMapIterator (`#sec-createmapiterator`)
    /// or CreateSetIterator (`#sec-createsetiterator`).
    pub(crate) fn collection_iterator_of(
        &mut self,
        this: Value,
        brand: CollectionKind,
        kind: IterationKind,
        method: &str,
    ) -> Completion<Value> {
        let collection = self.collection_of(this, brand, method)?;
        Ok(Value::Object(
            self.create_collection_iterator(collection, brand, kind)?,
        ))
    }

    /// CreateMapIterator or CreateSetIterator: an iterator object of `%MapIteratorPrototype%`
    /// or `%SetIteratorPrototype%` of the current realm that holds `collection`, with a cursor
    /// it owns registered with the collection's table. The iterator is made first, so the
    /// cursor names its owner; no safe point lies between the two.
    fn create_collection_iterator(
        &mut self,
        collection: ObjectRef,
        brand: CollectionKind,
        kind: IterationKind,
    ) -> Completion<ObjectRef> {
        let prototype = self.current_intrinsic(match brand {
            CollectionKind::Map => Intrinsic::MapIteratorPrototype,
            CollectionKind::Set => Intrinsic::SetIteratorPrototype,
        })?;
        let iterator = self.allocate_object(
            Some(prototype),
            ObjectKind::CollectionIterator(CollectionIterator {
                brand,
                collection: Some(collection),
                cursor: None,
                kind,
            }),
        )?;
        let cursor = self.with_entry_table(collection, |agent, table| {
            table.open_cursor(agent, Some(iterator.handle))
        })?;
        self.update_collection_iterator(iterator, Some(collection), Some(cursor))?;
        Ok(iterator)
    }

    /// `%MapIteratorPrototype%.next ( )` (`#sec-%mapiteratorprototype%.next`) and
    /// `%SetIteratorPrototype%.next ( )` (`#sec-%setiteratorprototype%.next`): GeneratorResume
    /// of the closure, over the slots that hold its state. No code runs: the collection is
    /// held by the iterator, the iterator by the activation.
    pub(crate) fn collection_iterator_next(
        &mut self,
        this: Value,
        brand: CollectionKind,
    ) -> Completion<Value> {
        // GeneratorValidate ( generator, brand ): a this value that is not an iterator of the
        // brand throws a TypeError.
        let slots = match this {
            Value::Object(iterator) => match &self.engine_object(iterator)?.kind {
                ObjectKind::CollectionIterator(slots) if slots.brand == brand => {
                    Some((iterator, slots.collection, slots.cursor, slots.kind))
                }
                _ => None,
            },
            _ => None,
        };
        let Some((iterator, collection, cursor, kind)) = slots else {
            let message = alloc::format!(
                "%{0}IteratorPrototype%.next requires that 'this' be a {0} Iterator",
                brand.name()
            );
            return Err(self.throw_error(ErrorKind::TypeError, &message));
        };
        // A completed generator answers CreateIteratorResultObject(undefined, true).
        let (Some(collection), Some(cursor)) = (collection, cursor) else {
            return Ok(Value::Object(
                self.create_iter_result_object(Value::Undefined, true)?,
            ));
        };
        // The entry this call visits is a step of its own, charged before the work, beside
        // the step of the call.
        self.charge_steps(1)?;
        let entry =
            self.with_entry_table(collection, |agent, table| table.advance(agent, cursor))?;
        let Some((key, value)) = entry else {
            // The closure returns: the cursor is closed, the collection dropped, and the
            // iterator is done for ever.
            self.with_entry_table(collection, |agent, table| {
                table.close_cursor(agent, cursor);
                Ok(())
            })?;
            self.update_collection_iterator(iterator, None, None)?;
            return Ok(Value::Object(
                self.create_iter_result_object(Value::Undefined, true)?,
            ));
        };
        // Map: key → e.[[Key]], value → e.[[Value]], key+value → CreateArrayFromList(«
        // e.[[Key]], e.[[Value]] »). Set: value → e, key+value → CreateArrayFromList(« e, e »).
        // No code runs while the Array is made, so the entry needs no root.
        let element = match brand {
            CollectionKind::Map => value,
            CollectionKind::Set => key,
        };
        let result = match kind {
            IterationKind::Key => key,
            IterationKind::Value => element,
            IterationKind::KeyValue => Value::Object(self.create_array_from_list(&[key, element])?),
        };
        // GeneratorYield(CreateIteratorResultObject(result, false)).
        Ok(Value::Object(
            self.create_iter_result_object(result, false)?,
        ))
    }

    /// Set the collection and the cursor of the iterator `iterator`; an object of another
    /// kind here is a broken invariant.
    fn update_collection_iterator(
        &mut self,
        iterator: ObjectRef,
        collection: Option<ObjectRef>,
        cursor: Option<crate::object::keyed_entries::CursorId>,
    ) -> Completion<()> {
        let changed = match &mut self.engine_object_mut(iterator)?.kind {
            ObjectKind::CollectionIterator(slots) => {
                slots.collection = collection;
                slots.cursor = cursor;
                true
            }
            _ => false,
        };
        if changed { Ok(()) } else { Err(self.fault()) }
    }
}
