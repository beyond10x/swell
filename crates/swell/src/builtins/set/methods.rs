//! The set methods of `%Set.prototype%` (ECMA-262 24.2.4, `story:set-methods`): `difference`
//! (`#sec-set.prototype.difference`), `intersection` (`#sec-set.prototype.intersection`),
//! `isDisjointFrom` (`#sec-set.prototype.isdisjointfrom`), `isSubsetOf`
//! (`#sec-set.prototype.issubsetof`), `isSupersetOf` (`#sec-set.prototype.issupersetof`),
//! `symmetricDifference` (`#sec-set.prototype.symmetricdifference`) and `union`
//! (`#sec-set.prototype.union`), with GetSetRecord (`#sec-getsetrecord`) over a set-like and
//! SetDataHas and SetDataIndex (`#sec-setdatahas`, `#sec-setdataindex`) as lookups in the entry
//! table (`object/keyed_entries.rs`), which canonicalizes `-0` to `+0` and compares by
//! SameValueZero.
//!
//! The steps keep the order of the clauses. Two readings of the clauses' Lists:
//!
//! - resultSetData is the table of the result `Set` itself, made at the step where the clause
//!   makes resultSetData (OrdinaryObjectCreate of the realm's `%Set.prototype%` reads no property,
//!   calls no code and reads no species, so making it early moves only an allocation) and kept on
//!   the kept stack until the call returns, so a root reaches every key it holds while the
//!   set-like's code runs; "a copy of set.[[SetData]]" is the live entries of `this` inserted in
//!   their order. The clause's last steps then answer that object.
//! - An index walk over a List (`this` in `intersection`, `isSubsetOf` and `isDisjointFrom`, the
//!   copy in `difference`) is a cursor of its table with no owner, which the collector keeps
//!   (`story:keyed-entry-table`, "Who releases a cursor"): an entry deleted before the walk
//!   reaches it is not visited, one added is, as the walk by index sees them. The walk runs in a
//!   function of its own; its cursor is closed on every way out before the completion is
//!   answered, as `forEach` closes its own (`builtins/map/mod.rs`).
//!
//! Bounds (`docs/design/engine.md`, decisions 3 and 4): the call is a builtin call, its step;
//! one step per entry of `this` visited (by a walk or by the copy) and per key read from the
//! set-like's iterator, each before the work; the table of the result is charged as every
//! table is; every call of `size`'s getter, `has`, `keys` and `next` goes through the agent's
//! call path, which counts it against the depth bounds.

use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::host::{ErrorKind, Host, Termination};
use crate::object::collection::{Collection, CollectionKind};
use crate::object::keyed_entries::CursorId;
use crate::object::{ObjectKind, ObjectRecord};
use crate::ops::iteration::IteratorRecord;
use crate::realm::intrinsics::Intrinsic;
use crate::value::{ObjectRef, Value};
use alloc::boxed::Box;

/// The builtins of this module, by their steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SetMethodsBuiltin {
    /// `Set.prototype.difference ( other )` (24.2.4.5).
    Difference,
    /// `Set.prototype.intersection ( other )` (24.2.4.9).
    Intersection,
    /// `Set.prototype.isDisjointFrom ( other )` (24.2.4.10).
    IsDisjointFrom,
    /// `Set.prototype.isSubsetOf ( other )` (24.2.4.11).
    IsSubsetOf,
    /// `Set.prototype.isSupersetOf ( other )` (24.2.4.12).
    IsSupersetOf,
    /// `Set.prototype.symmetricDifference ( other )` (24.2.4.15).
    SymmetricDifference,
    /// `Set.prototype.union ( other )` (24.2.4.16).
    Union,
}

impl SetMethodsBuiltin {
    /// The method's name, for the messages of its errors.
    fn name(self) -> &'static str {
        match self {
            SetMethodsBuiltin::Difference => "Set.prototype.difference",
            SetMethodsBuiltin::Intersection => "Set.prototype.intersection",
            SetMethodsBuiltin::IsDisjointFrom => "Set.prototype.isDisjointFrom",
            SetMethodsBuiltin::IsSubsetOf => "Set.prototype.isSubsetOf",
            SetMethodsBuiltin::IsSupersetOf => "Set.prototype.isSupersetOf",
            SetMethodsBuiltin::SymmetricDifference => "Set.prototype.symmetricDifference",
            SetMethodsBuiltin::Union => "Set.prototype.union",
        }
    }
}

/// A Set Record (`#sec-set-records`, Table 72). Its holder keeps [[Has]] and [[Keys]] on the
/// kept stack; [[SetObject]] is the argument, which the activation holds.
struct SetRecord {
    /// [[SetObject]].
    object: ObjectRef,
    /// [[Size]]: a non-negative integer or +∞.
    size: f64,
    /// [[Has]].
    has: Value,
    /// [[Keys]].
    keys: Value,
}

/// What a walk over a List by index does with each entry and the answer of [[Has]].
#[derive(Clone, Copy)]
enum Walk {
    /// `intersection`, step 5.c: append an entry the other has to the result.
    Intersection(ObjectRef),
    /// `difference`, step 5.c: empty an entry of the result (the walked copy) the other has.
    Difference(ObjectRef),
    /// `isSubsetOf`, step 7: false at an entry the other lacks.
    IsSubsetOf,
    /// `isDisjointFrom`, step 4: false at an entry the other has.
    IsDisjointFrom,
}

/// What the loop over the keys of the set-like does with each key.
#[derive(Clone, Copy)]
enum Read {
    /// `union`, step 7: append a key the result lacks.
    Union(ObjectRef),
    /// `intersection`, step 6: append a key `this` has and the result lacks.
    Intersection(ObjectRef),
    /// `difference`, step 6: empty the result's entry of the key.
    Difference(ObjectRef),
    /// `symmetricDifference`, step 7.
    SymmetricDifference(ObjectRef),
    /// `isSupersetOf`, step 7: false, after IteratorClose, at a key `this` lacks.
    IsSupersetOf,
    /// `isDisjointFrom`, step 5: false, after IteratorClose, at a key `this` has.
    IsDisjointFrom,
}

impl<H: Host> Agent<H> {
    /// The steps of the builtin `id` of this module on the this value `this` and the argument
    /// `other`: what the steps keep on the kept stack is released on every way out.
    pub(crate) fn run_set_methods_builtin(
        &mut self,
        id: SetMethodsBuiltin,
        this: Value,
        other: Value,
    ) -> Completion<Value> {
        // 1. Let set be the this value. 2. Perform ? RequireInternalSlot(set, [[SetData]]).
        let set = self.set_data(this, id.name())?;
        let mark = self.keep(this);
        let answer = self.set_method_steps(id, set, other);
        self.release(mark);
        answer
    }

    /// Steps 3 and after of the method `id`, on the Set `set`.
    fn set_method_steps(
        &mut self,
        id: SetMethodsBuiltin,
        set: ObjectRef,
        other: Value,
    ) -> Completion<Value> {
        // 3. Let otherRecord be ? GetSetRecord(other).
        let record = self.get_set_record(other, id.name())?;
        match id {
            SetMethodsBuiltin::Union => {
                // 4. Let keysIterator be ? GetIteratorFromMethod(otherRecord.[[SetObject]],
                // otherRecord.[[Keys]]). 5. Let resultSetData be a copy of set.[[SetData]].
                let mut keys = self.open_keys(&record)?;
                let result = self.new_result_set(Some(set))?;
                // 6. and 7.
                self.read_keys(&mut keys, set, Read::Union(result))?;
                // 8. Let result be OrdinaryObjectCreate(%Set.prototype%, « [[SetData]] »). 9.
                // Set result.[[SetData]] to resultSetData. 10. Return result.
                Ok(Value::Object(result))
            }
            SetMethodsBuiltin::Intersection => {
                // 4. Let resultSetData be a new empty List.
                let result = self.new_result_set(None)?;
                // 5. If SetDataSize(set.[[SetData]]) ≤ otherRecord.[[Size]], walk set.[[SetData]].
                if self.set_data_size(set)? <= record.size {
                    self.walk_with_has(set, &record, Walk::Intersection(result))?;
                } else {
                    // 6. Else, iterate the keys of the other.
                    let mut keys = self.open_keys(&record)?;
                    self.read_keys(&mut keys, set, Read::Intersection(result))?;
                }
                // 7. to 9.
                Ok(Value::Object(result))
            }
            SetMethodsBuiltin::Difference => {
                // 4. Let resultSetData be a copy of set.[[SetData]].
                let result = self.new_result_set(Some(set))?;
                // 5. If SetDataSize(set.[[SetData]]) ≤ otherRecord.[[Size]], walk
                // resultSetData by index (step 5.c reads resultSetData[index]).
                if self.set_data_size(set)? <= record.size {
                    self.walk_with_has(result, &record, Walk::Difference(result))?;
                } else {
                    // 6. Else, iterate the keys of the other.
                    let mut keys = self.open_keys(&record)?;
                    self.read_keys(&mut keys, set, Read::Difference(result))?;
                }
                // 7. to 9.
                Ok(Value::Object(result))
            }
            SetMethodsBuiltin::SymmetricDifference => {
                // 4. Let keysIterator be ? GetIteratorFromMethod(otherRecord.[[SetObject]],
                // otherRecord.[[Keys]]). 5. Let resultSetData be a copy of set.[[SetData]].
                let mut keys = self.open_keys(&record)?;
                let result = self.new_result_set(Some(set))?;
                // 6. and 7.
                self.read_keys(&mut keys, set, Read::SymmetricDifference(result))?;
                // 8. to 10.
                Ok(Value::Object(result))
            }
            SetMethodsBuiltin::IsSubsetOf => {
                // 4. If SetDataSize(set.[[SetData]]) > otherRecord.[[Size]], return false.
                if self.set_data_size(set)? > record.size {
                    return Ok(Value::Boolean(false));
                }
                // 5. to 8.
                let answer = self.walk_with_has(set, &record, Walk::IsSubsetOf)?;
                Ok(Value::Boolean(answer))
            }
            SetMethodsBuiltin::IsSupersetOf => {
                // 4. If SetDataSize(set.[[SetData]]) < otherRecord.[[Size]], return false.
                if self.set_data_size(set)? < record.size {
                    return Ok(Value::Boolean(false));
                }
                // 5. Let keysIterator be ? GetIteratorFromMethod(otherRecord.[[SetObject]],
                // otherRecord.[[Keys]]). 6. to 8.
                let mut keys = self.open_keys(&record)?;
                let answer = self.read_keys(&mut keys, set, Read::IsSupersetOf)?;
                Ok(Value::Boolean(answer))
            }
            SetMethodsBuiltin::IsDisjointFrom => {
                // 4. If SetDataSize(set.[[SetData]]) ≤ otherRecord.[[Size]], walk
                // set.[[SetData]].
                let answer = if self.set_data_size(set)? <= record.size {
                    self.walk_with_has(set, &record, Walk::IsDisjointFrom)?
                } else {
                    // 5. Else, iterate the keys of the other.
                    let mut keys = self.open_keys(&record)?;
                    self.read_keys(&mut keys, set, Read::IsDisjointFrom)?
                };
                // 6. Return true.
                Ok(Value::Boolean(answer))
            }
        }
    }

    /// GetSetRecord ( obj ) (`#sec-getsetrecord`); `method` names the caller in the messages.
    /// [[Has]] and [[Keys]] are kept on the kept stack for the caller, which releases them.
    fn get_set_record(&mut self, other: Value, method: &str) -> Completion<SetRecord> {
        // 1. If obj is not an Object, throw a TypeError exception.
        let Value::Object(object) = other else {
            let message = alloc::format!("{method}: the argument is not an object");
            return Err(self.throw_error(ErrorKind::TypeError, &message));
        };
        // 2. Let rawSize be ? Get(obj, "size").
        let key = self.name_key("size")?;
        let raw_size = self.get(object, key)?;
        // 3. Let numberSize be ? ToNumber(rawSize). rawSize is kept while its conversion runs
        // code.
        let mark = self.keep(raw_size);
        let number_size = self.to_number(raw_size);
        self.release(mark);
        let number_size = number_size?;
        // 4. NOTE: If rawSize is undefined, then numberSize will be NaN. 5. If numberSize is
        // NaN, throw a TypeError exception.
        if number_size.is_nan() {
            let message = alloc::format!("{method}: the size of the argument is not a number");
            return Err(self.throw_error(ErrorKind::TypeError, &message));
        }
        // 6. Let intSize be ! ToIntegerOrInfinity(numberSize).
        let size = self.to_integer_or_infinity(Value::Number(number_size))?;
        // 7. If intSize < 0, throw a RangeError exception.
        if size < 0.0 {
            let message = alloc::format!("{method}: the size of the argument is negative");
            return Err(self.throw_error(ErrorKind::RangeError, &message));
        }
        // 8. Let has be ? Get(obj, "has"). 9. If IsCallable(has) is false, throw a TypeError
        // exception.
        let has = self.callable_property(object, "has", method)?;
        self.keep(has);
        // 10. Let keys be ? Get(obj, "keys"). 11. If IsCallable(keys) is false, throw a
        // TypeError exception.
        let keys = self.callable_property(object, "keys", method)?;
        self.keep(keys);
        // 12. Return a new Set Record { [[SetObject]]: obj, [[Size]]: intSize, [[Has]]: has,
        // [[Keys]]: keys }.
        Ok(SetRecord {
            object,
            size,
            has,
            keys,
        })
    }

    /// Get(obj, `name`), or the `TypeError` of GetSetRecord when the value is not callable.
    fn callable_property(
        &mut self,
        object: ObjectRef,
        name: &str,
        method: &str,
    ) -> Completion<Value> {
        let key = self.name_key(name)?;
        let value = self.get(object, key)?;
        if !self.is_callable(value)? {
            let message =
                alloc::format!("{method}: the '{name}' of the argument is not a function");
            return Err(self.throw_error(ErrorKind::TypeError, &message));
        }
        Ok(value)
    }

    /// SetDataSize ( set.[[SetData]] ) (`#sec-setdatasize`) as a Number, for the comparisons
    /// with [[Size]].
    fn set_data_size(&mut self, set: ObjectRef) -> Completion<f64> {
        let size = match &self.engine_object(set)?.kind {
            ObjectKind::Collection(slots) => slots.table.len(),
            _ => return Err(self.fault()),
        };
        match u32::try_from(size) {
            Ok(size) => Ok(f64::from(size)),
            Err(_) => Err(self.fault()),
        }
    }

    /// OrdinaryObjectCreate(%Set.prototype%, « [[SetData]] ») of the current realm, made at the
    /// step that makes resultSetData and kept on the kept stack for the caller, which releases
    /// it: its table is resultSetData, empty, or with `copy_of` a copy of `copy_of`'s
    /// [[SetData]], each live entry inserted in its order, one step per entry before its
    /// insertion. Over the memory budget the run ends in `Terminated(MemoryBudget)`.
    fn new_result_set(&mut self, copy_of: Option<ObjectRef>) -> Completion<ObjectRef> {
        let prototype = self.current_intrinsic(Intrinsic::SetPrototype)?;
        let record = ObjectRecord::new(
            Some(prototype),
            ObjectKind::Collection(Box::new(Collection::new(CollectionKind::Set))),
        );
        let result = match self.heap.new_object(record, Collection::box_bytes()) {
            Ok(result) => result,
            Err(_) => return Err(self.terminate(Termination::MemoryBudget)),
        };
        self.keep(Value::Object(result));
        if let Some(set) = copy_of {
            // No operation of either table runs code or passes a safe point.
            self.with_entry_table(set, |agent, source| {
                agent.with_entry_table(result, |agent, target| {
                    let mut position = 0;
                    while let Some(at) = source.next_live(agent, position)? {
                        agent.charge_steps(1)?;
                        if let Some((key, _)) = source.entry_at(at) {
                            target.insert(agent, key, Value::Undefined)?;
                        }
                        position = at.saturating_add(1);
                    }
                    Ok(())
                })
            })?;
        }
        Ok(result)
    }

    /// GetIteratorFromMethod ( otherRecord.[[SetObject]], otherRecord.[[Keys]] )
    /// (`#sec-getiteratorfrommethod`), with [[Iterator]] and [[NextMethod]] (read once) kept on
    /// the kept stack for the caller, which releases them.
    fn open_keys(&mut self, record: &SetRecord) -> Completion<IteratorRecord> {
        let keys = self.get_iterator_from_method(Value::Object(record.object), record.keys)?;
        self.keep(Value::Object(keys.iterator));
        self.keep(keys.next_method);
        Ok(keys)
    }

    /// The loop "Repeat, while next is not done" over the keys of the set-like: one step per
    /// key, before IteratorStepValue reads it (`#sec-iteratorstepvalue`), then what `read`
    /// does with it. Answers false where `isSupersetOf` or `isDisjointFrom` answers false, after
    /// IteratorClose ( keysIterator, NormalCompletion(unused) ) (`#sec-iteratorclose`), and true
    /// at the end of the keys. No other way out closes the iterator: a throw of `next` or of
    /// reading its result is answered as it is, and a termination closes nothing.
    fn read_keys(
        &mut self,
        keys: &mut IteratorRecord,
        set: ObjectRef,
        read: Read,
    ) -> Completion<bool> {
        loop {
            self.charge_steps(1)?;
            // Set next to ? IteratorStepValue(keysIterator).
            let Some(next) = self.iterator_step_value(keys)? else {
                return Ok(true);
            };
            // Set next to CanonicalizeKeyedCollectionKey(next): the table canonicalizes every
            // key it reads or stores. From here to the next call no code runs.
            match read {
                Read::Union(result) => {
                    // If SetDataHas(resultSetData, next) is false, append next to resultSetData.
                    self.with_entry_table(result, |agent, table| {
                        table.insert(agent, next, Value::Undefined)
                    })?;
                }
                Read::Intersection(result) => {
                    // Let inThis be SetDataHas(set.[[SetData]], next). If inThis is true and
                    // SetDataHas(resultSetData, next) is false, append next to resultSetData.
                    if self.set_data_has(set, next)? {
                        self.with_entry_table(result, |agent, table| {
                            table.insert(agent, next, Value::Undefined)
                        })?;
                    }
                }
                Read::Difference(result) => {
                    // Let valueIndex be SetDataIndex(resultSetData, next). If valueIndex is not
                    // not-found, set resultSetData[valueIndex] to empty.
                    self.with_entry_table(result, |agent, table| table.delete(agent, next))?;
                }
                Read::SymmetricDifference(result) => {
                    // Let resultIndex be SetDataIndex(resultSetData, next); alreadyInResult.
                    let already_in_result = self.set_data_has(result, next)?;
                    if self.set_data_has(set, next)? {
                        // If SetDataHas(set.[[SetData]], next) is true and alreadyInResult is
                        // true, set resultSetData[resultIndex] to empty.
                        if already_in_result {
                            self.with_entry_table(result, |agent, table| {
                                table.delete(agent, next)
                            })?;
                        }
                    } else if !already_in_result {
                        // Else, if alreadyInResult is false, append next to resultSetData.
                        self.with_entry_table(result, |agent, table| {
                            table.insert(agent, next, Value::Undefined)
                        })?;
                    }
                }
                Read::IsSupersetOf => {
                    // If SetDataHas(set.[[SetData]], next) is false, then perform ?
                    // IteratorClose(keysIterator, NormalCompletion(unused)) and return false.
                    if !self.set_data_has(set, next)? {
                        self.iterator_close(keys, Ok(Value::Undefined))?;
                        return Ok(false);
                    }
                }
                Read::IsDisjointFrom => {
                    // If SetDataHas(set.[[SetData]], next) is true, then perform ?
                    // IteratorClose(keysIterator, NormalCompletion(unused)) and return false.
                    if self.set_data_has(set, next)? {
                        self.iterator_close(keys, Ok(Value::Undefined))?;
                        return Ok(false);
                    }
                }
            }
        }
    }

    /// SetDataHas ( set.[[SetData]], value ) (`#sec-setdatahas`) of the Set `set`.
    fn set_data_has(&mut self, set: ObjectRef, value: Value) -> Completion<bool> {
        let found = self.with_entry_table(set, |agent, table| table.find(agent, value))?;
        Ok(found.is_some())
    }

    /// A walk by index over the List of the Set `walked`, calling [[Has]] of the set-like with
    /// each entry: a cursor of its table with no owner, opened before the walk and closed on
    /// every way out before the completion is answered. Answers what [`Walk`] answers early,
    /// or true at the end of the List.
    fn walk_with_has(
        &mut self,
        walked: ObjectRef,
        record: &SetRecord,
        walk: Walk,
    ) -> Completion<bool> {
        let cursor =
            self.with_entry_table(walked, |agent, table| table.open_cursor(agent, None))?;
        let walked_answer = self.walk_steps(walked, cursor, record, walk);
        let closed = self.with_entry_table(walked, |agent, table| {
            table.close_cursor(agent, cursor);
            Ok(())
        });
        let answer = walked_answer?;
        closed?;
        Ok(answer)
    }

    /// The loop of [`Agent::walk_with_has`]: "Repeat, while index < thisSize", one step per
    /// entry before its work. The cursor visits the entries a walk by index visits: an entry
    /// [[Has]] deletes before the walk reaches it is skipped, one it adds is visited, and the
    /// walk's own deletions in the copy of `difference` do not move it past an entry.
    fn walk_steps(
        &mut self,
        walked: ObjectRef,
        cursor: CursorId,
        record: &SetRecord,
        walk: Walk,
    ) -> Completion<bool> {
        loop {
            self.charge_steps(1)?;
            // Let entry be the element at index; set index to index + 1; skip an empty one.
            let entry =
                self.with_entry_table(walked, |agent, table| table.advance(agent, cursor))?;
            let Some((entry, _)) = entry else {
                return Ok(true);
            };
            // Let inOther be ToBoolean(? Call(otherRecord.[[Has]], otherRecord.[[SetObject]],
            // « entry »)). The call holds entry; after it no code runs until the next call.
            let answer = self.call_value(record.has, Value::Object(record.object), &[entry])?;
            let in_other = self.to_boolean(answer)?;
            match walk {
                Walk::Intersection(result) => {
                    // If inOther is true and SetDataHas(resultSetData, entry) is false, append
                    // entry to resultSetData.
                    if in_other {
                        self.with_entry_table(result, |agent, table| {
                            table.insert(agent, entry, Value::Undefined)
                        })?;
                    }
                }
                Walk::Difference(result) => {
                    // If inOther is true, set resultSetData[index] to empty.
                    if in_other {
                        self.with_entry_table(result, |agent, table| table.delete(agent, entry))?;
                    }
                }
                Walk::IsSubsetOf => {
                    // If inOther is false, return false.
                    if !in_other {
                        return Ok(false);
                    }
                }
                Walk::IsDisjointFrom => {
                    // If inOther is true, return false.
                    if in_other {
                        return Ok(false);
                    }
                }
            }
        }
    }
}
