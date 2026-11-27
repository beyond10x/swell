//! `Object.fromEntries ( iterable )` (ECMA-262 20.1.2.7, `#sec-object.fromentries`) through
//! AddEntriesFromIterable (`#sec-add-entries-from-iterable`), and `Object.groupBy ( items,
//! callback )` (20.1.2.13, `#sec-object.groupby`) through GroupBy (`#sec-groupby`) with the
//! property keys coercion and AddValueToKeyedGroup (`#sec-addvaluetokeyedgroup`). Written by
//! `story:array-from-and-object-from-entries`.
//!
//! The design is `docs/design/engine.md`, decision 3, "Iterators held by a builtin": the
//! Iterator Record's [[Iterator]] and [[NextMethod]], the object being built, the entry while
//! its `0` and `1` are read and its key and value while the key is made, and the value being
//! grouped while the callback and ToPropertyKey run, are kept on the agent's kept stack until
//! the builtin returns, on every way out (item 1). Each loop charges one step per iteration
//! through `charge_steps` before its work, beside the steps the call path charges for `next`
//! and the callback (items 2 and 3). An abrupt completion of the builtin's own work between two
//! steps closes the iterator (IfAbruptCloseIterator, `#sec-ifabruptcloseiterator`); a throw of
//! `next`, `done` or `value` ends it with no close, and a termination closes nothing (item 4).
//!
//! The adder of `Object.fromEntries` (steps 4 and 5, an abstract closure made a built-in function
//! that "is never directly accessible to ECMAScript code") is its two steps, run in place:
//! ToPropertyKey of the key and CreateDataPropertyOrThrow on the new object. The groups of
//! `Object.groupBy` are kept as the object the builtin answers, built while the iterator runs:
//! the object (prototype null) holds one Array per key, each made when its key is first seen
//! and appended to after, so the groups are in the order of their keys' first appearance, as
//! GroupBy's List is, and each is found by its key through the object's property index. The
//! object and its Arrays are charged to the memory budget as they grow, as every object is,
//! and are garbage when the builtin ends abruptly. No step between GroupBy's return and the
//! object's last property runs code or can fail but at the memory budget, so the answer is the
//! one the clause's steps 2 to 4 make from the List.
//!
//! GroupBy takes its keyCoercion ([`KeyCoercion`], `story:map-upsert-and-group-by`): property
//! for `Object.groupBy`, as above, and collection for `Map.groupBy` (`#sec-map.groupby`,
//! `builtins/map/upsert.rs`), whose groups are kept the same way in the table of the result Map,
//! made and kept by the caller before the first element is read: each key after
//! CanonicalizeKeyedCollectionKey (the table makes `-0` `+0` and compares by SameValueZero,
//! `object/keyed_entries.rs`), its Array made when the key is first seen and stored at once as
//! the key's value, so every key and every group is reachable from the kept Map while the
//! callback and `next` run. Both coercions run the one loop below, with its step per element,
//! its IteratorClose and its charges.

use super::argument;
use crate::builtins::array::MAX_SAFE_LENGTH;
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::host::{ErrorKind, Host};
use crate::object::ordinary::OrdinaryData;
use crate::object::property::{Key, Property, Slot};
use crate::realm::intrinsics::Intrinsic;
use crate::value::{ObjectRef, Value};

/// The keyCoercion of GroupBy (`#sec-groupby`), which also says what kind of object keeps the
/// groups.
#[derive(Clone, Copy, Debug)]
pub(crate) enum KeyCoercion {
    /// property: ToPropertyKey of each key; the groups are the properties of an ordinary object
    /// (`Object.groupBy`).
    Property,
    /// collection: CanonicalizeKeyedCollectionKey of each key; the groups are the entries of a
    /// Map's table (`Map.groupBy`).
    Collection,
}

impl<H: Host> Agent<H> {
    /// `Object.fromEntries ( iterable )` (`#sec-object.fromentries`).
    pub(crate) fn object_from_entries(&mut self, args: &[Value]) -> Completion<Value> {
        let iterable = argument(args, 0);
        // 1. Perform ? RequireObjectCoercible(iterable) (`#sec-requireobjectcoercible`).
        if matches!(iterable, Value::Undefined | Value::Null) {
            return Err(self.throw_error(
                ErrorKind::TypeError,
                "Object.fromEntries called on undefined or null",
            ));
        }
        // 2. Let obj be OrdinaryObjectCreate(%Object.prototype%).
        let prototype = self.current_intrinsic(Intrinsic::ObjectPrototype)?;
        let object = self.ordinary_object_create(Some(prototype), OrdinaryData::Plain)?;
        // 6. Return ? AddEntriesFromIterable(obj, iterable, adder), with obj and what the steps
        // keep released on every way out.
        let mark = self.keep(Value::Object(object));
        let added = self.add_entries_from_iterable(object, iterable);
        self.release(mark);
        added?;
        Ok(Value::Object(object))
    }

    /// AddEntriesFromIterable ( target, iterable, adder ) (`#sec-add-entries-from-iterable`)
    /// with the adder of `Object.fromEntries`; the caller releases what it keeps.
    fn add_entries_from_iterable(&mut self, target: ObjectRef, iterable: Value) -> Completion<()> {
        // 1. Let iteratorRecord be ? GetIterator(iterable, sync). `iterable` is held by the
        // activation of `Object.fromEntries`.
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
            // 2.c. If next is not an Object, return ? IteratorClose(iteratorRecord, a
            // TypeError).
            let Value::Object(entry) = next else {
                let error = self.throw_error(
                    ErrorKind::TypeError,
                    "Object.fromEntries: an entry is not an object",
                );
                return Err(self.close_iterator_after(&record, error));
            };
            // 2.d to 2.i, each abrupt completion closing the iterator.
            let mark = self.keep(next);
            let added = self.add_entry(target, entry);
            self.release(mark);
            if let Err(abrupt) = added {
                return Err(self.close_iterator_after(&record, abrupt));
            }
        }
    }

    /// Steps 2.d to 2.i of AddEntriesFromIterable with the adder of `Object.fromEntries`
    /// (`#sec-object.fromentries`, step 4), the entry kept by the caller: Get of its `0` and
    /// `1`, each a Get that may run code, then ToPropertyKey of the key, which may too, and the
    /// definition. The caller releases the key and the value kept here.
    fn add_entry(&mut self, target: ObjectRef, entry: ObjectRef) -> Completion<()> {
        // 2.d. Let key be Completion(Get(next, "0")).
        let key = self.get(entry, Key::Index(0))?;
        self.keep(key);
        // 2.f. Let value be Completion(Get(next, "1")).
        let value = self.get(entry, Key::Index(1))?;
        self.keep(value);
        // 2.h. Let status be Completion(Call(adder, target, « key, value »)): 4.a. Let
        // propertyKey be ? ToPropertyKey(key). 4.b. Perform ! CreateDataPropertyOrThrow(obj,
        // propertyKey, value): obj is an extensible ordinary object, so the definition is
        // made, or the run ends at the memory budget.
        let key = self.to_property_key(key)?;
        if !self.create_data_property(target, key, value)? {
            return Err(self.fault());
        }
        Ok(())
    }

    /// `Object.groupBy ( items, callback )` (`#sec-object.groupby`).
    pub(crate) fn object_group_by(&mut self, args: &[Value]) -> Completion<Value> {
        let items = argument(args, 0);
        let callback = argument(args, 1);
        self.group_by_arguments(items, callback, "Object.groupBy")?;
        // 2. Let obj be OrdinaryObjectCreate(null): made first, the groups' object (the module
        // comment).
        let object = self.ordinary_object_create(None, OrdinaryData::Plain)?;
        let mark = self.keep(Value::Object(object));
        let grouped = self.group_by(
            object,
            items,
            callback,
            KeyCoercion::Property,
            "Object.groupBy",
        );
        self.release(mark);
        grouped?;
        // 4. Return obj.
        Ok(Value::Object(object))
    }

    /// Steps 1 and 2 of GroupBy (`#sec-groupby`), before the caller makes the object that keeps
    /// the groups: RequireObjectCoercible of `items` and IsCallable of `callback`; `method`
    /// names the caller in the messages.
    pub(crate) fn group_by_arguments(
        &mut self,
        items: Value,
        callback: Value,
        method: &str,
    ) -> Completion<()> {
        // 1. Perform ? RequireObjectCoercible(items).
        if matches!(items, Value::Undefined | Value::Null) {
            let message = alloc::format!("{method} called on undefined or null");
            return Err(self.throw_error(ErrorKind::TypeError, &message));
        }
        // 2. If IsCallable(callback) is false, throw a TypeError exception.
        if !self.is_callable(callback)? {
            let message = alloc::format!("{method}: the callback is not a function");
            return Err(self.throw_error(ErrorKind::TypeError, &message));
        }
        Ok(())
    }

    /// Steps 3 to 6 of GroupBy ( items, callback, keyCoercion ) (`#sec-groupby`), each group
    /// added to `groups` (the object of `Object.groupBy` or the Map of `Map.groupBy`, by
    /// `coercion`) as it grows; the caller keeps `groups` and releases what is kept here.
    /// `items` and `callback` are held by the activation of the caller.
    pub(crate) fn group_by(
        &mut self,
        groups: ObjectRef,
        items: Value,
        callback: Value,
        coercion: KeyCoercion,
        method: &str,
    ) -> Completion<()> {
        // 3. Let groups be a new empty List. 4. Let iteratorRecord be ? GetIterator(items,
        // sync).
        let mut record = self.get_iterator(items)?;
        self.keep(Value::Object(record.iterator));
        self.keep(record.next_method);
        // 5. Let k be 0. 6. Repeat,
        let mut index = 0.0;
        loop {
            self.charge_steps(1)?;
            // 6.a. If k ≥ 2^53 - 1, return ? IteratorClose(iteratorRecord, a TypeError).
            if index >= MAX_SAFE_LENGTH {
                let message = alloc::format!("{method} would count an index above 2^53 - 1");
                let error = self.throw_error(ErrorKind::TypeError, &message);
                return Err(self.close_iterator_after(&record, error));
            }
            // 6.b. Let next be ? IteratorStepValue(iteratorRecord). 6.c. If next is done,
            // return groups.
            let Some(value) = self.iterator_step_value(&mut record)? else {
                return Ok(());
            };
            // 6.d to 6.i, each abrupt completion closing the iterator; the value is kept
            // while the callback and ToPropertyKey run.
            let mark = self.keep(value);
            let added = self.group_value(groups, callback, value, index, coercion);
            self.release(mark);
            if let Err(abrupt) = added {
                return Err(self.close_iterator_after(&record, abrupt));
            }
            // 6.j. Set k to k + 1.
            index += 1.0;
        }
    }

    /// Steps 6.d to 6.i of GroupBy with `coercion`: the key of `value` at `index`, then
    /// AddValueToKeyedGroup.
    fn group_value(
        &mut self,
        groups: ObjectRef,
        callback: Value,
        value: Value,
        index: f64,
        coercion: KeyCoercion,
    ) -> Completion<()> {
        // 6.d. Let value be next. 6.e. Let key be Completion(Call(callback, undefined, « value,
        // 𝔽(k) »)).
        let key = self.call_value(callback, Value::Undefined, &[value, Value::Number(index)])?;
        match coercion {
            KeyCoercion::Property => {
                // 6.g.i. Set key to Completion(ToPropertyKey(key)). ToPropertyKey holds its
                // argument while it runs code; nothing runs after it, so its atom needs no keep.
                let key = self.to_property_key(key)?;
                // 6.i. Perform AddValueToKeyedGroup(groups, key, value).
                self.add_value_to_keyed_group(groups, key, value)
            }
            // 6.h.ii. Set key to CanonicalizeKeyedCollectionKey(key): the table's key. 6.i.
            // Perform AddValueToKeyedGroup(groups, key, value). No code runs after the callback.
            KeyCoercion::Collection => self.add_value_to_collection_group(groups, key, value),
        }
    }

    /// AddValueToKeyedGroup ( groups, key, value ) (`#sec-addvaluetokeyedgroup`) on the table of
    /// the Map `groups`: the Array that is the value of `key` (by SameValueZero, `-0` as `+0`)
    /// gets `value` at its end, or a new Array of `value` alone is appended to the table under
    /// `key`. The lookup and the insertion are a step each, and the table charges its growth
    /// before it (`object/keyed_entries.rs`). No code runs and no safe point lies between the
    /// callback's answer and its insertion, so the key needs no keep: the Array is reachable
    /// from the Map as soon as it is stored.
    fn add_value_to_collection_group(
        &mut self,
        groups: ObjectRef,
        key: Value,
        value: Value,
    ) -> Completion<()> {
        // 1. For each Record group of groups: if SameValue(group.[[Key]], key) is true, append
        // value to group.[[Elements]] and return.
        match self.map_entry_value(groups, key)? {
            Some(Value::Object(group)) => self.append_to_group(group, value),
            // Every value of the result's table is a group's Array.
            Some(_) => Err(self.fault()),
            // 2. Let group be the Record { [[Key]]: key, [[Elements]]: « value » }. 3. Append
            // group to groups. With `#sec-map.groupby`, step 3.a, CreateArrayFromList.
            None => {
                let group = self.create_array_from_list(&[value])?;
                self.with_entry_table(groups, |agent, table| {
                    table.insert(agent, key, Value::Object(group))
                })
            }
        }
    }

    /// Append `value` to the group's Array `group`, which the builtin made: CreateDataProperty
    /// at its length. No code runs: the Array is new and the builtin's own.
    fn append_to_group(&mut self, group: ObjectRef, value: Value) -> Completion<()> {
        let length = self.length_of_array_like(group)?;
        let index = self.index_key(length)?;
        if !self.create_data_property(group, index, value)? {
            return Err(self.fault());
        }
        Ok(())
    }

    /// AddValueToKeyedGroup ( groups, key, value ) (`#sec-addvaluetokeyedgroup`) on the groups'
    /// object: the Array of `key` gets `value` at its end, or a new Array of `value` alone is
    /// defined under `key` after the groups there are. The group's key is a property key, so
    /// SameValue of two keys is their equality. No code runs: each object is new and the
    /// builtin's own.
    fn add_value_to_keyed_group(
        &mut self,
        groups: ObjectRef,
        key: Key,
        value: Value,
    ) -> Completion<()> {
        // 1. For each Record group of groups: if SameValue(group.[[Key]], key) is true, append
        // value to group.[[Elements]] and return.
        match self.get_own_property(groups, key)? {
            Some(Property {
                slot:
                    Slot::Data {
                        value: Value::Object(group),
                        ..
                    },
                ..
            }) => self.append_to_group(group, value),
            // Every property of the groups' object is a group's Array.
            Some(_) => Err(self.fault()),
            // 2. Let group be the Record { [[Key]]: key, [[Elements]]: « value » }. 3. Append
            // group to groups. With the clause's step 3.a, CreateArrayFromList.
            None => {
                let group = self.create_array_from_list(&[value])?;
                if !self.create_data_property(groups, key, Value::Object(group))? {
                    return Err(self.fault());
                }
                Ok(())
            }
        }
    }
}
