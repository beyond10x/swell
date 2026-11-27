//! The upsert methods of `%Map.prototype%` and `Map.groupBy` (ECMA-262 24.1,
//! `story:map-upsert-and-group-by`): `Map.groupBy` (`#sec-map.groupby`) through GroupBy
//! (`#sec-groupby`) with the keyCoercion collection, `Map.prototype.getOrInsert`
//! (`#sec-map.prototype.getorinsert`) and `Map.prototype.getOrInsertComputed`
//! (`#sec-map.prototype.getorinsertcomputed`). The group's variant is `MapBuiltin::Upsert`
//! (`builtins/map/mod.rs`), which defines the two methods in the list of `%Map.prototype%` in the
//! order of the clauses and `groupBy` on `%Map%`.
//!
//! [[MapData]] is the entry table of the Map (`object/keyed_entries.rs`): it canonicalizes `-0`
//! to `+0` (CanonicalizeKeyedCollectionKey) and compares keys by SameValueZero, and a lookup and
//! an insertion are a step each, charged before the work; an insertion of a key the table holds
//! overwrites the value in place. So `getOrInsertComputed`'s search after the callback
//! (steps 7 to 10) is one insertion: a key the callback set keeps its place and takes the
//! callback's answer, and no entry is appended twice.
//!
//! `Map.groupBy` makes its result Map first (`! Construct(%Map%)`, which reads no property a
//! script can change and calls no code, so making it before GroupBy moves only an allocation),
//! keeps it until the call returns, and runs the loop of GroupBy that `Object.groupBy` runs
//! (`builtins/object/from_entries.rs`), which writes each group into the Map's table as its Array
//! is made: every key and every group is reachable from the kept Map while the callback and the
//! iterator's `next` run.
//!
//! Bounds (`docs/design/engine.md`, decisions 3 and 4): each method is a builtin call, its step;
//! GroupBy charges one step per element before its work, beside the steps of `next`, the
//! callback and the table; the table and each group's Array are charged to the memory budget
//! before they grow. The callbacks and `next` re-enter through the agent's call path, counted
//! against `reentry_depth` and checked against `frame_depth`.

use crate::builtins::object::argument;
use crate::builtins::object::from_entries::KeyCoercion;
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::host::{ErrorKind, Host};
use crate::object::collection::CollectionKind;
use crate::realm::intrinsics::Intrinsic;
use crate::value::{ObjectRef, Value};

/// The builtins of this module, by their steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum UpsertBuiltin {
    /// `Map.groupBy ( items, callback )` (24.1.2.1).
    GroupBy,
    /// `Map.prototype.getOrInsert ( key, value )` (24.1.3.7).
    GetOrInsert,
    /// `Map.prototype.getOrInsertComputed ( key, callback )` (24.1.3.8).
    GetOrInsertComputed,
}

/// CanonicalizeKeyedCollectionKey ( key ) (`#sec-canonicalizekeyedcollectionkey`): `-0` is
/// `+0`, every other value is itself. A Number plus `+0` is that Number, but `-0 + +0` is
/// `+0` (`#sec-numeric-types-number-add`, rounding to nearest), and a NaN stays a NaN.
fn canonical_key(key: Value) -> Value {
    match key {
        Value::Number(number) => Value::Number(number + 0.0),
        other => other,
    }
}

impl<H: Host> Agent<H> {
    /// The steps of the builtin `id` of this module.
    pub(crate) fn run_upsert_builtin(
        &mut self,
        id: UpsertBuiltin,
        this: Value,
        args: &[Value],
    ) -> Completion<Value> {
        let first = argument(args, 0);
        let second = argument(args, 1);
        match id {
            UpsertBuiltin::GroupBy => self.map_group_by(first, second),
            UpsertBuiltin::GetOrInsert => self.map_get_or_insert(this, first, second),
            UpsertBuiltin::GetOrInsertComputed => {
                self.map_get_or_insert_computed(this, first, second)
            }
        }
    }

    /// `Map.groupBy ( items, callback )` (`#sec-map.groupby`).
    fn map_group_by(&mut self, items: Value, callback: Value) -> Completion<Value> {
        // 1. Let groups be ? GroupBy(items, callback, collection): its steps 1 and 2 first.
        self.group_by_arguments(items, callback, "Map.groupBy")?;
        // 2. Let map be ! Construct(%Map%): made first, the groups' Map (the module comment).
        let constructor = self.current_intrinsic(Intrinsic::Map)?;
        let map = self.create_collection(constructor, CollectionKind::Map)?;
        let mark = self.keep(Value::Object(map));
        // GroupBy, steps 3 to 6, and 3. For each Record { [[Key]], [[Elements]] } group of
        // groups, the entry appended to map.[[MapData]] as its group is made.
        let grouped = self.group_by(map, items, callback, KeyCoercion::Collection, "Map.groupBy");
        self.release(mark);
        grouped?;
        // 4. Return map.
        Ok(Value::Object(map))
    }

    /// The value of the entry of `key` in the Map `map`, if it holds one: one step, and a
    /// String key's code units, charged by the table's lookup. `-0` is found as `+0`.
    pub(crate) fn map_entry_value(
        &mut self,
        map: ObjectRef,
        key: Value,
    ) -> Completion<Option<Value>> {
        self.with_entry_table(map, |agent, table| match table.find(agent, key)? {
            Some(position) => match table.entry_at(position) {
                Some((_, value)) => Ok(Some(value)),
                None => Err(agent.fault()),
            },
            None => Ok(None),
        })
    }

    /// `Map.prototype.getOrInsert ( key, value )` (`#sec-map.prototype.getorinsert`).
    fn map_get_or_insert(&mut self, this: Value, key: Value, value: Value) -> Completion<Value> {
        // 1. Let map be the this value. 2. Perform ? RequireInternalSlot(map, [[MapData]]).
        let map = self.map_data(this, "Map.prototype.getOrInsert")?;
        // 3. Set key to CanonicalizeKeyedCollectionKey(key). 4. For each Record { [[Key]],
        // [[Value]] } entry of map.[[MapData]], if entry.[[Key]] is not empty and
        // SameValue(entry.[[Key]], key) is true, return entry.[[Value]].
        let key = canonical_key(key);
        if let Some(found) = self.map_entry_value(map, key)? {
            return Ok(found);
        }
        // 5. Let entry be the Record { [[Key]]: key, [[Value]]: value }. 6. Append entry to
        // map.[[MapData]].
        self.with_entry_table(map, |agent, table| table.insert(agent, key, value))?;
        // 7. Return value.
        Ok(value)
    }

    /// `Map.prototype.getOrInsertComputed ( key, callback )`
    /// (`#sec-map.prototype.getorinsertcomputed`). The callback re-enters, counted against
    /// `reentry_depth` by the call; `this` and `key` are arguments of the running builtin, so
    /// the collector keeps them while it runs, and no code runs between the callback's answer
    /// and its insertion.
    fn map_get_or_insert_computed(
        &mut self,
        this: Value,
        key: Value,
        callback: Value,
    ) -> Completion<Value> {
        let method = "Map.prototype.getOrInsertComputed";
        // 1. Let map be the this value. 2. Perform ? RequireInternalSlot(map, [[MapData]]).
        let map = self.map_data(this, method)?;
        // 3. If IsCallable(callback) is false, throw a TypeError exception.
        if !self.is_callable(callback)? {
            let message = alloc::format!("{method}: the callback is not a function");
            return Err(self.throw_error(ErrorKind::TypeError, &message));
        }
        // 4. Set key to CanonicalizeKeyedCollectionKey(key). 5. For each Record { [[Key]],
        // [[Value]] } entry of map.[[MapData]], if entry.[[Key]] is not empty and
        // SameValue(entry.[[Key]], key) is true, return entry.[[Value]].
        let key = canonical_key(key);
        if let Some(found) = self.map_entry_value(map, key)? {
            return Ok(found);
        }
        // 6. Let value be ? Call(callback, undefined, « key »).
        let value = self.call_value(callback, Value::Undefined, &[key])?;
        // 7. NOTE: The Map may have been modified during execution of callback. 8. For each
        // Record { [[Key]], [[Value]] } entry of map.[[MapData]], if entry.[[Key]] is not empty
        // and SameValue(entry.[[Key]], key) is true, set entry.[[Value]] to value and return
        // value. 9. Let entry be the Record { [[Key]]: key, [[Value]]: value }. 10. Append
        // entry to map.[[MapData]]: the table's insertion, which overwrites in place or appends.
        self.with_entry_table(map, |agent, table| table.insert(agent, key, value))?;
        // 11. Return value.
        Ok(value)
    }
}
