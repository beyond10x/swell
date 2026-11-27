//! The WeakMap objects (ECMA-262 24.3, `#sec-weakmap-objects`), written by
//! `story:weak-map-and-weak-set`: `%WeakMap%` (`WeakMap ( [ iterable ] )`,
//! `#sec-weakmap-iterable`, with AddEntriesFromIterable and the adder `set`, the loop of
//! `builtins/map/mod.rs`) and `%WeakMap.prototype%` (`#sec-properties-of-the-weakmap-prototype-object`)
//! with `constructor`, `delete`, `get`, `getOrInsert`, `getOrInsertComputed`, `has`, `set` and
//! `[%Symbol.toStringTag%]`. A key is an object or a Symbol that CanBeHeldWeakly
//! (`#sec-canbeheldweakly`, `ops/testing.rs`); [[WeakMapData]] is the table of
//! `object/weak_collection.rs`, whose entries the collector clears once their key is unreachable.

use crate::builtins::Method;
use crate::builtins::object::argument;
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::host::{ErrorKind, Host};
use crate::object::builtin_function::BuiltinId;
use crate::object::ordinary::OrdinaryData;
use crate::object::weak_collection::WeakKind;
use crate::realm::intrinsics::Making;
use crate::value::{ObjectRef, Value};

/// The builtins of this module, by their steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WeakMapBuiltin {
    /// `WeakMap ( [ iterable ] )` (24.3.1.1).
    Constructor,
    /// `WeakMap.prototype.delete ( key )` (24.3.3.2).
    Delete,
    /// `WeakMap.prototype.get ( key )` (24.3.3.3).
    Get,
    /// `WeakMap.prototype.getOrInsert ( key, value )` (24.3.3.4).
    GetOrInsert,
    /// `WeakMap.prototype.getOrInsertComputed ( key, callback )` (24.3.3.5).
    GetOrInsertComputed,
    /// `WeakMap.prototype.has ( key )` (24.3.3.6).
    Has,
    /// `WeakMap.prototype.set ( key, value )` (24.3.3.7).
    Set,
}

/// The methods of `%WeakMap.prototype%` after `constructor`, in the order of the clauses.
const PROTOTYPE_METHODS: &[Method<WeakMapBuiltin>] = &[
    ("delete", 1, WeakMapBuiltin::Delete),
    ("get", 1, WeakMapBuiltin::Get),
    ("getOrInsert", 2, WeakMapBuiltin::GetOrInsert),
    (
        "getOrInsertComputed",
        2,
        WeakMapBuiltin::GetOrInsertComputed,
    ),
    ("has", 1, WeakMapBuiltin::Has),
    ("set", 2, WeakMapBuiltin::Set),
];

/// The message of the `TypeError` for a key that cannot be held weakly.
const NOT_WEAK: &str = "a WeakMap key must be an object or a Symbol not in the registry";

impl<H: Host> Agent<H> {
    /// `%WeakMap%` and `%WeakMap.prototype%` of the realm being made.
    pub(crate) fn create_weak_map_constructor(
        &mut self,
        making: Making,
    ) -> Completion<(ObjectRef, ObjectRef)> {
        // 24.3.3: "is an ordinary object", whose [[Prototype]] is %Object.prototype%.
        let prototype =
            self.ordinary_object_create(Some(making.object_prototype), OrdinaryData::Plain)?;
        // 24.3.2: "length" 0 (24.3.1.1 has one optional parameter); its "prototype" (24.3.2.1)
        // and the prototype's "constructor" (24.3.3.1).
        let constructor = self.create_builtin_constructor(
            making,
            "WeakMap",
            0,
            BuiltinId::WeakMap(WeakMapBuiltin::Constructor),
            prototype,
        )?;
        self.define_group_methods(making, prototype, PROTOTYPE_METHODS, BuiltinId::WeakMap)?;
        // 24.3.3.8: "WeakMap", { [[Writable]]: false, [[Enumerable]]: false,
        // [[Configurable]]: true }.
        self.define_to_string_tag(making, prototype, "WeakMap")?;
        Ok((constructor, prototype))
    }

    /// The steps of the builtin `id` of this module.
    pub(crate) fn run_weak_map_builtin(
        &mut self,
        id: WeakMapBuiltin,
        this: Value,
        args: &[Value],
        new_target: Option<ObjectRef>,
    ) -> Completion<Value> {
        let key = argument(args, 0);
        match id {
            WeakMapBuiltin::Constructor => self.weak_map_constructor(key, new_target),
            WeakMapBuiltin::Delete => self.weak_map_delete(this, key),
            WeakMapBuiltin::Get => self.weak_map_get(this, key),
            WeakMapBuiltin::GetOrInsert => {
                self.weak_map_get_or_insert(this, key, argument(args, 1))
            }
            WeakMapBuiltin::GetOrInsertComputed => {
                self.weak_map_get_or_insert_computed(this, key, argument(args, 1))
            }
            WeakMapBuiltin::Has => self.weak_map_has(this, key),
            WeakMapBuiltin::Set => self.weak_map_set(this, key, argument(args, 1)),
        }
    }

    /// `WeakMap ( [ iterable ] )` (`#sec-weakmap-iterable`).
    fn weak_map_constructor(
        &mut self,
        iterable: Value,
        new_target: Option<ObjectRef>,
    ) -> Completion<Value> {
        // 1. If NewTarget is undefined, throw a TypeError exception.
        let Some(new_target) = new_target else {
            return Err(
                self.throw_error(ErrorKind::TypeError, "Constructor WeakMap requires 'new'")
            );
        };
        // 2. Let map be ? OrdinaryCreateFromConstructor(NewTarget, "%WeakMap.prototype%", «
        // [[WeakMapData]] »). 3. Set map.[[WeakMapData]] to a new empty List.
        let map = self.create_weak_collection(new_target, WeakKind::Map)?;
        // 4. If iterable is either undefined or null, return map.
        if matches!(iterable, Value::Undefined | Value::Null) {
            return Ok(Value::Object(map));
        }
        // 5. Let adder be ? Get(map, "set"). 6. If IsCallable(adder) is false, throw a
        // TypeError exception. 7. Return ? AddEntriesFromIterable(map, iterable, adder).
        let mark = self.keep(Value::Object(map));
        let filled = self.fill_collection(map, iterable, "set");
        self.release(mark);
        filled?;
        Ok(Value::Object(map))
    }

    /// `WeakMap.prototype.delete ( key )` (`#sec-weakmap.prototype.delete`).
    fn weak_map_delete(&mut self, this: Value, key: Value) -> Completion<Value> {
        // 1. Let weakMap be the this value. 2. Perform ? RequireInternalSlot(weakMap,
        // [[WeakMapData]]).
        let map = self.weak_collection_of(this, WeakKind::Map, "WeakMap.prototype.delete")?;
        // 3. If CanBeHeldWeakly(key) is false, return false.
        let Some(key) = self.weak_key(key)? else {
            return Ok(Value::Boolean(false));
        };
        // 4. For each entry, if SameValue(entry.[[Key]], key) is true, empty it and return
        // true. 5. Return false.
        Ok(Value::Boolean(self.weak_delete(map, key)?))
    }

    /// `WeakMap.prototype.get ( key )` (`#sec-weakmap.prototype.get`).
    fn weak_map_get(&mut self, this: Value, key: Value) -> Completion<Value> {
        let map = self.weak_collection_of(this, WeakKind::Map, "WeakMap.prototype.get")?;
        // 3. If CanBeHeldWeakly(key) is false, return undefined.
        let Some(key) = self.weak_key(key)? else {
            return Ok(Value::Undefined);
        };
        // 4. Return the value of the entry of key. 5. Return undefined.
        Ok(self.weak_get(map, key)?.unwrap_or(Value::Undefined))
    }

    /// `WeakMap.prototype.getOrInsert ( key, value )` (`#sec-weakmap.prototype.getorinsert`).
    fn weak_map_get_or_insert(
        &mut self,
        this: Value,
        key: Value,
        value: Value,
    ) -> Completion<Value> {
        let map = self.weak_collection_of(this, WeakKind::Map, "WeakMap.prototype.getOrInsert")?;
        // 3. If CanBeHeldWeakly(key) is false, throw a TypeError exception.
        let Some(key) = self.weak_key(key)? else {
            return Err(self.throw_error(ErrorKind::TypeError, NOT_WEAK));
        };
        // 4. If an entry of key exists, return its value.
        if let Some(found) = self.weak_get(map, key)? {
            return Ok(found);
        }
        // 5. Let entry be the Record { [[Key]]: key, [[Value]]: value }. 6. Append entry to
        // weakMap.[[WeakMapData]]. 7. Return value.
        self.weak_insert(map, key, value)?;
        Ok(value)
    }

    /// `WeakMap.prototype.getOrInsertComputed ( key, callback )`
    /// (`#sec-weakmap.prototype.getorinsertcomputed`). The callback re-enters, counted against
    /// `reentry_depth` by the call; `this` and `key` are arguments of the running builtin, so
    /// the collector keeps them while it runs.
    fn weak_map_get_or_insert_computed(
        &mut self,
        this: Value,
        key: Value,
        callback: Value,
    ) -> Completion<Value> {
        let method = "WeakMap.prototype.getOrInsertComputed";
        let map = self.weak_collection_of(this, WeakKind::Map, method)?;
        // 3. If CanBeHeldWeakly(key) is false, throw a TypeError exception.
        let Some(weak) = self.weak_key(key)? else {
            return Err(self.throw_error(ErrorKind::TypeError, NOT_WEAK));
        };
        // 4. If IsCallable(callback) is false, throw a TypeError exception.
        if !self.is_callable(callback)? {
            let message = alloc::format!("{method}: the callback is not a function");
            return Err(self.throw_error(ErrorKind::TypeError, &message));
        }
        // 5. If an entry of key exists, return its value.
        if let Some(found) = self.weak_get(map, weak)? {
            return Ok(found);
        }
        // 6. Let value be ? Call(callback, undefined, « key »).
        let value = self.call_value(callback, Value::Undefined, &[key])?;
        // 7. NOTE: The WeakMap may have been modified during execution of callback. 8. If an
        // entry of key exists now, set its value to value and return value. 9. Let entry be
        // the Record { [[Key]]: key, [[Value]]: value }. 10. Append entry to
        // weakMap.[[WeakMapData]]. 11. Return value.
        let mark = self.keep(value);
        let inserted = self.weak_insert(map, weak, value);
        self.release(mark);
        inserted?;
        Ok(value)
    }

    /// `WeakMap.prototype.has ( key )` (`#sec-weakmap.prototype.has`).
    fn weak_map_has(&mut self, this: Value, key: Value) -> Completion<Value> {
        let map = self.weak_collection_of(this, WeakKind::Map, "WeakMap.prototype.has")?;
        // 3. If CanBeHeldWeakly(key) is false, return false.
        let Some(key) = self.weak_key(key)? else {
            return Ok(Value::Boolean(false));
        };
        Ok(Value::Boolean(self.weak_get(map, key)?.is_some()))
    }

    /// `WeakMap.prototype.set ( key, value )` (`#sec-weakmap.prototype.set`).
    fn weak_map_set(&mut self, this: Value, key: Value, value: Value) -> Completion<Value> {
        let map = self.weak_collection_of(this, WeakKind::Map, "WeakMap.prototype.set")?;
        // 3. If CanBeHeldWeakly(key) is false, throw a TypeError exception.
        let Some(key) = self.weak_key(key)? else {
            return Err(self.throw_error(ErrorKind::TypeError, NOT_WEAK));
        };
        // 4. If an entry of key exists, set its value to value and return weakMap. 5. Let entry
        // be the Record { [[Key]]: key, [[Value]]: value }. 6. Append entry to
        // weakMap.[[WeakMapData]]. 7. Return weakMap.
        self.weak_insert(map, key, value)?;
        Ok(this)
    }
}
