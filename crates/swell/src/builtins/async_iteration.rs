//! The async iteration objects of slice 10, written by
//! `story:for-await-and-async-from-sync-iterator`: `%AsyncIteratorPrototype%`
//! (`#sec-asynciteratorprototype`) with its `[%Symbol.asyncIterator%]`
//! (`#sec-asynciteratorprototype-asynciterator`), and the Async-from-Sync Iterator objects
//! (`#sec-async-from-sync-iterator-objects`): CreateAsyncFromSyncIterator
//! (`#sec-createasyncfromsynciterator`), `%AsyncFromSyncIteratorPrototype%`
//! (`#sec-%asyncfromsynciteratorprototype%-object`) with `next`, `return` and `throw`
//! (`#sec-%asyncfromsynciteratorprototype%.next` and the two after it), and
//! AsyncFromSyncIteratorContinuation (`#sec-asyncfromsynciteratorcontinuation`) with its two
//! Abstract Closures, the unwrap function and the function that closes the sync iterator.
//!
//! The design is `docs/design/engine.md`, "The Async-from-Sync Iterator": no step of these
//! Awaits, so they use only built-in functions with slots (decision 4, "A built-in function
//! with slots"): the unwrap function holds `done` ([`AsyncFromSyncSlots::Unwrap`]), and the
//! function that closes the sync iterator holds the [[Iterator]] of its Iterator Record, the one
//! field IteratorClose reads ([`AsyncFromSyncSlots::Close`]). The object holds
//! [[SyncIteratorRecord]] in a box of its kind, `ObjectKind::AsyncFromSyncIterator`, charged
//! with the record as owned bytes when it is made (`Heap::new_object`), credited by the sweep
//! that frees it, and traced at its kind site ([`AsyncFromSyncIterator::trace`]). Such an
//! object is never handed to a script: only the loop of a `for await` holds it, as the
//! [[Iterator]] of its record. A call of one of the three methods spends the step of a builtin
//! call; the call of the sync iterator's method is a re-entry as any call from a builtin. The
//! capability of a method and the one of PromiseResolve are made without the call of an
//! executor, and settled without a call ([`Agent::intrinsic_capability`]), so a depth bound
//! refuses no step the standard marks with `!`.
//!
//! `[%Symbol.asyncDispose%]` of `%AsyncIteratorPrototype%`
//! (`#sec-%asynciteratorprototype%-%symbol.asyncdispose%`) is slice 15.

use crate::heap::collect::Mark;
use crate::host::agent::Agent;
use crate::host::outcome::{Abrupt, Completion};
use crate::host::{ErrorKind, Host, PromiseCapability, Termination};
use crate::object::builtin_function::{BuiltinId, Slots};
use crate::object::ordinary::OrdinaryData;
use crate::object::property::{Key, Property};
use crate::object::{ObjectKind, ObjectRecord};
use crate::ops::iteration::IteratorRecord;
use crate::realm::intrinsics::{Intrinsic, Making};
use crate::value::{ObjectRef, Value};
use alloc::boxed::Box;

/// The builtins of this module, by their steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AsyncIterationBuiltin {
    /// `%AsyncIteratorPrototype% [ %Symbol.asyncIterator% ] ( )` (27.1.4.2).
    AsyncIterator,
    /// `%AsyncFromSyncIteratorPrototype%.next ( [ value ] )` (27.1.5.2.1).
    Next,
    /// `%AsyncFromSyncIteratorPrototype%.return ( [ value ] )` (27.1.5.2.2).
    Return,
    /// `%AsyncFromSyncIteratorPrototype%.throw ( [ value ] )` (27.1.5.2.3).
    Throw,
    /// The unwrap function of AsyncFromSyncIteratorContinuation (27.1.5.4, steps 8 and 9), a
    /// built-in function with the slots [`AsyncFromSyncSlots::Unwrap`].
    Unwrap,
    /// The function that closes the sync iterator (27.1.5.4, step 12), a built-in function with
    /// the slots [`AsyncFromSyncSlots::Close`].
    Close,
}

/// The slots of the two functions of AsyncFromSyncIteratorContinuation: what their Abstract
/// Closures capture.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum AsyncFromSyncSlots {
    /// `unwrap` captures `done`.
    Unwrap { done: bool },
    /// `closeIterator` captures syncIteratorRecord; IteratorClose reads its [[Iterator]] alone.
    Close { iterator: ObjectRef },
}

impl AsyncFromSyncSlots {
    /// Every value the slots hold.
    pub(crate) fn trace(&self, mark: &mut Mark) {
        match self {
            AsyncFromSyncSlots::Unwrap { .. } => {}
            AsyncFromSyncSlots::Close { iterator } => mark.object(*iterator),
        }
    }
}

/// The slots of an Async-from-Sync Iterator object (27.1.5.3, Table 86): [[SyncIteratorRecord]],
/// in a box charged with the object.
pub(crate) struct AsyncFromSyncIterator {
    pub(crate) record: Box<IteratorRecord>,
}

impl AsyncFromSyncIterator {
    /// Kind site of the collector: [[Iterator]] and [[NextMethod]] of [[SyncIteratorRecord]].
    pub(crate) fn trace(&self, mark: &mut Mark) {
        mark.count("builtins/async_iteration.rs AsyncFromSyncIterator::trace");
        mark.object(self.record.iterator);
        mark.value(self.record.next_method);
    }

    /// An Async-from-Sync Iterator object's slots, for the tests of the collector.
    #[cfg(test)]
    pub(crate) fn for_test(iterator: ObjectRef) -> AsyncFromSyncIterator {
        AsyncFromSyncIterator {
            record: Box::new(IteratorRecord {
                iterator,
                next_method: Value::Undefined,
                done: false,
            }),
        }
    }
}

/// The bytes the box of [[SyncIteratorRecord]] is charged.
pub(crate) fn sync_record_bytes() -> u64 {
    u64::try_from(size_of::<IteratorRecord>()).unwrap_or(u64::MAX)
}

/// `%AsyncIteratorPrototype%` and `%AsyncFromSyncIteratorPrototype%` of a realm.
#[derive(Clone, Copy, Debug)]
pub(crate) struct AsyncIterationPrototypes {
    pub(crate) async_iterator: ObjectRef,
    pub(crate) async_from_sync_iterator: ObjectRef,
}

impl<H: Host> Agent<H> {
    /// `%AsyncIteratorPrototype%`, an ordinary object whose [[Prototype]] is `%Object.prototype%`
    /// (27.1.4), with its `[%Symbol.asyncIterator%]` named "[Symbol.asyncIterator]" of "length"
    /// 0; `%AsyncFromSyncIteratorPrototype%`, an ordinary object whose [[Prototype]] is
    /// `%AsyncIteratorPrototype%` (27.1.5.2), with "next", "return" and "throw" of "length" 0
    /// (their one parameter is optional, clause 18). The methods are { [[Writable]]: true,
    /// [[Enumerable]]: false, [[Configurable]]: true } (clause 18).
    pub(crate) fn create_async_iteration_prototypes(
        &mut self,
        making: Making,
    ) -> Completion<AsyncIterationPrototypes> {
        let async_iterator =
            self.ordinary_object_create(Some(making.object_prototype), OrdinaryData::Plain)?;
        let answer_this = self.create_builtin(
            making,
            "[Symbol.asyncIterator]",
            0,
            BuiltinId::AsyncIteration(AsyncIterationBuiltin::AsyncIterator),
        )?;
        self.add_property(
            async_iterator,
            Key::Symbol(making.symbols.async_iterator),
            Property::data(Value::Object(answer_this), true, false, true),
        )?;
        let async_from_sync_iterator =
            self.ordinary_object_create(Some(async_iterator), OrdinaryData::Plain)?;
        for (name, id) in [
            ("next", AsyncIterationBuiltin::Next),
            ("return", AsyncIterationBuiltin::Return),
            ("throw", AsyncIterationBuiltin::Throw),
        ] {
            self.define_builtin_method(
                making,
                async_from_sync_iterator,
                name,
                0,
                BuiltinId::AsyncIteration(id),
            )?;
        }
        Ok(AsyncIterationPrototypes {
            async_iterator,
            async_from_sync_iterator,
        })
    }

    /// CreateAsyncFromSyncIterator ( syncIteratorRecord ) (`#sec-createasyncfromsynciterator`).
    /// The caller holds nothing of `sync`: its [[Iterator]] and [[NextMethod]] are kept here
    /// until the new object holds them. The record answered is not kept.
    pub(crate) fn create_async_from_sync_iterator(
        &mut self,
        sync: IteratorRecord,
    ) -> Completion<IteratorRecord> {
        let mark = self.keep(Value::Object(sync.iterator));
        self.keep(sync.next_method);
        let made = self.async_from_sync_iterator_object(sync);
        self.release(mark);
        let iterator = made?;
        // 3. Let nextMethod be ! Get(asyncIterator, "next"): the data property of the
        // prototype, read with the object kept.
        let mark = self.keep(Value::Object(iterator));
        let next_method = self
            .name_key("next")
            .and_then(|key| self.get(iterator, key));
        self.release(mark);
        // 4. Let iteratorRecord be the Iterator Record { [[Iterator]]: asyncIterator,
        // [[NextMethod]]: nextMethod, [[Done]]: false }. 5. Return iteratorRecord.
        Ok(IteratorRecord {
            iterator,
            next_method: next_method?,
            done: false,
        })
    }

    /// Steps 1 and 2 of CreateAsyncFromSyncIterator: OrdinaryObjectCreate(
    /// `%AsyncFromSyncIteratorPrototype%`, « [[SyncIteratorRecord]] ») with the record set, its
    /// box charged as the object's owned bytes; over the budget the run ends in
    /// `Terminated(MemoryBudget)`.
    fn async_from_sync_iterator_object(&mut self, sync: IteratorRecord) -> Completion<ObjectRef> {
        let prototype = self.current_intrinsic(Intrinsic::AsyncFromSyncIteratorPrototype)?;
        let kind = ObjectKind::AsyncFromSyncIterator(AsyncFromSyncIterator {
            record: Box::new(sync),
        });
        match self.heap.new_object(
            ObjectRecord::new(Some(prototype), kind),
            sync_record_bytes(),
        ) {
            Ok(object) => Ok(object),
            Err(_) => Err(self.terminate(Termination::MemoryBudget)),
        }
    }

    /// [[SyncIteratorRecord]] of the Async-from-Sync Iterator object `this`; anything else is a
    /// broken invariant, since no script reaches the methods of
    /// `%AsyncFromSyncIteratorPrototype%` (27.1.5.2: "Assert: obj is an Object that has a
    /// [[SyncIteratorRecord]] internal slot").
    fn sync_iterator_record(&mut self, this: Value) -> Completion<IteratorRecord> {
        let Value::Object(object) = this else {
            return Err(self.fault());
        };
        match &self.engine_object(object)?.kind {
            ObjectKind::AsyncFromSyncIterator(iterator) => Ok(*iterator.record),
            _ => Err(self.fault()),
        }
    }

    /// The steps of the builtins of this module.
    pub(crate) fn run_async_iteration_builtin(
        &mut self,
        id: AsyncIterationBuiltin,
        function: ObjectRef,
        this: Value,
        args: &[Value],
    ) -> Completion<Value> {
        match id {
            // 27.1.4.2: 1. Return the this value.
            AsyncIterationBuiltin::AsyncIterator => Ok(this),
            AsyncIterationBuiltin::Next
            | AsyncIterationBuiltin::Return
            | AsyncIterationBuiltin::Throw => {
                // 3. Let promiseCapability be ! NewPromiseCapability(%Promise%).
                let capability = self.intrinsic_capability()?;
                let mark = self.keep_capability(&capability);
                let answered = self.async_from_sync_method(id, this, args, capability);
                self.release(mark);
                answered
            }
            AsyncIterationBuiltin::Unwrap => {
                let argument = args.first().copied().unwrap_or(Value::Undefined);
                self.async_from_sync_unwrap(function, argument)
            }
            AsyncIterationBuiltin::Close => {
                let argument = args.first().copied().unwrap_or(Value::Undefined);
                self.async_from_sync_close(function, argument)
            }
        }
    }

    /// `next`, `return` or `throw` of `%AsyncFromSyncIteratorPrototype%` from step 4 on, with
    /// the capability of step 3 kept: its promise, settled now or by the reaction of the
    /// continuation. `value` is present when the call has an argument.
    fn async_from_sync_method(
        &mut self,
        id: AsyncIterationBuiltin,
        this: Value,
        args: &[Value],
        capability: PromiseCapability,
    ) -> Completion<Value> {
        // 4. Let syncIteratorRecord be obj.[[SyncIteratorRecord]].
        let mut sync = self.sync_iterator_record(this)?;
        let value = args.first().copied();
        let result = match id {
            AsyncIterationBuiltin::Next => {
                // 5. IteratorNext(syncIteratorRecord[, value]). 6. IfAbruptRejectPromise.
                match self.iterator_next_with(&mut sync, value) {
                    Ok(result) => result,
                    Err(abrupt) => return self.reject_capability(capability, abrupt),
                }
            }
            AsyncIterationBuiltin::Return => match self.async_from_sync_return(&sync, value)? {
                Answer::Result(result) => result,
                Answer::Resolve(answer) => {
                    return self.resolve_capability(capability, answer);
                }
                Answer::Reject(abrupt) => return self.reject_capability(capability, abrupt),
            },
            AsyncIterationBuiltin::Throw => match self.async_from_sync_throw(&sync, value)? {
                Answer::Result(result) => result,
                Answer::Resolve(answer) => {
                    return self.resolve_capability(capability, answer);
                }
                Answer::Reject(abrupt) => return self.reject_capability(capability, abrupt),
            },
            AsyncIterationBuiltin::AsyncIterator
            | AsyncIterationBuiltin::Unwrap
            | AsyncIterationBuiltin::Close => return Err(self.fault()),
        };
        // 7 (`next`), 12 (`return`, `throw`). Return AsyncFromSyncIteratorContinuation(result,
        // promiseCapability, syncIteratorRecord, closeOnRejection): true for `next` and
        // `throw`, false for `return`.
        let close_on_rejection = id != AsyncIterationBuiltin::Return;
        let mark = self.keep(Value::Object(result));
        let continued =
            self.async_from_sync_continuation(result, capability, sync, close_on_rejection);
        self.release(mark);
        continued
    }

    /// Steps 5 to 11 of `%AsyncFromSyncIteratorPrototype%.return` (27.1.5.2.2).
    fn async_from_sync_return(
        &mut self,
        sync: &IteratorRecord,
        value: Option<Value>,
    ) -> Completion<Answer> {
        // 5. Let syncIterator be syncIteratorRecord.[[Iterator]]. 6. Let return be
        // Completion(GetMethod(syncIterator, "return")). 7. IfAbruptRejectPromise.
        let method = self
            .name_key("return")
            .and_then(|key| self.get_method(Value::Object(sync.iterator), key));
        let method = match method {
            Ok(method) => method,
            Err(abrupt) => return Ok(Answer::Reject(abrupt)),
        };
        // 8. If return is undefined, then a. Let iteratorResult be
        // CreateIteratorResultObject(value, true). b. Perform ! Call(promiseCapability.[[Resolve]],
        // undefined, « iteratorResult »).
        let Some(method) = method else {
            let result = self.create_iter_result_object(value.unwrap_or(Value::Undefined), true)?;
            return Ok(Answer::Resolve(Value::Object(result)));
        };
        // 9. Call(return, syncIterator[, « value »]). 10. IfAbruptRejectPromise.
        self.async_from_sync_call(method, sync.iterator, value)
    }

    /// Steps 5 to 11 of `%AsyncFromSyncIteratorPrototype%.throw` (27.1.5.2.3).
    fn async_from_sync_throw(
        &mut self,
        sync: &IteratorRecord,
        value: Option<Value>,
    ) -> Completion<Answer> {
        // 5. Let syncIterator be syncIteratorRecord.[[Iterator]]. 6. Let throw be
        // Completion(GetMethod(syncIterator, "throw")). 7. IfAbruptRejectPromise.
        let method = self
            .name_key("throw")
            .and_then(|key| self.get_method(Value::Object(sync.iterator), key));
        let method = match method {
            Ok(method) => method,
            Err(abrupt) => return Ok(Answer::Reject(abrupt)),
        };
        let Some(method) = method else {
            // 8.b. Let closeCompletion be NormalCompletion(empty). c. Let result be
            // Completion(IteratorClose(syncIteratorRecord, closeCompletion)). d.
            // IfAbruptRejectPromise(result, promiseCapability).
            if let Err(abrupt) = self.iterator_close(sync, Ok(Value::Undefined)) {
                return Ok(Answer::Reject(abrupt));
            }
            // g. Perform ! Call(promiseCapability.[[Reject]], undefined, « a newly created
            // TypeError object »).
            return Ok(Answer::Reject(self.throw_error(
                ErrorKind::TypeError,
                "the sync iterator has no throw method",
            )));
        };
        // 9. Call(throw, syncIterator[, « value »]). 10. IfAbruptRejectPromise.
        self.async_from_sync_call(method, sync.iterator, value)
    }

    /// The call of the sync iterator's `return` or `throw`, with the argument `value` when it is
    /// present, and its check (27.1.5.2.2 and 27.1.5.2.3, steps 9 to 11): a throw of it, and a
    /// `TypeError` for a result that is no Object, reject.
    fn async_from_sync_call(
        &mut self,
        method: Value,
        iterator: ObjectRef,
        value: Option<Value>,
    ) -> Completion<Answer> {
        let called = match value {
            Some(value) => self.call_value(method, Value::Object(iterator), &[value]),
            None => self.call_value(method, Value::Object(iterator), &[]),
        };
        match called {
            Ok(Value::Object(result)) => Ok(Answer::Result(result)),
            // 11. If result is not an Object, then perform ! Call(promiseCapability.[[Reject]],
            // undefined, « a newly created TypeError object »).
            Ok(_) => Ok(Answer::Reject(self.throw_error(
                ErrorKind::TypeError,
                "the result of the sync iterator is not an object",
            ))),
            Err(abrupt) => Ok(Answer::Reject(abrupt)),
        }
    }

    /// AsyncFromSyncIteratorContinuation ( result, promiseCapability, syncIteratorRecord,
    /// closeOnRejection ) (`#sec-asyncfromsynciteratorcontinuation`). The caller keeps
    /// `result` and the capability; `this`, which holds `sync`, is held by the activation of the
    /// method.
    fn async_from_sync_continuation(
        &mut self,
        result: ObjectRef,
        capability: PromiseCapability,
        sync: IteratorRecord,
        close_on_rejection: bool,
    ) -> Completion<Value> {
        // 1. Let done be Completion(IteratorComplete(result)). 2. IfAbruptRejectPromise.
        let done = match self.iterator_complete(result) {
            Ok(done) => done,
            Err(abrupt) => return self.reject_capability(capability, abrupt),
        };
        // 3. Let value be Completion(IteratorValue(result)). 4. IfAbruptRejectPromise.
        let value = match self.iterator_value(result) {
            Ok(value) => value,
            Err(abrupt) => return self.reject_capability(capability, abrupt),
        };
        let mark = self.keep(value);
        let continued =
            self.async_from_sync_then(value, done, capability, sync, close_on_rejection);
        self.release(mark);
        continued
    }

    /// Steps 5 to 14 of AsyncFromSyncIteratorContinuation, with `value` kept.
    fn async_from_sync_then(
        &mut self,
        value: Value,
        done: bool,
        capability: PromiseCapability,
        sync: IteratorRecord,
        close_on_rejection: bool,
    ) -> Completion<Value> {
        // 5. Let valueWrapper be Completion(PromiseResolve(%Promise%, value)).
        let wrapper = match self.intrinsic_promise_resolve(value) {
            // 6. If valueWrapper is an abrupt completion, done is false, and closeOnRejection
            // is true, then set valueWrapper to Completion(IteratorClose(syncIteratorRecord,
            // valueWrapper)).
            Err(Abrupt::Throw(thrown)) if !done && close_on_rejection => {
                self.iterator_close(&sync, Err(Abrupt::Throw(thrown)))
            }
            other => other,
        };
        // 7. IfAbruptRejectPromise(valueWrapper, promiseCapability).
        let wrapper = match wrapper {
            Ok(Value::Object(wrapper)) => wrapper,
            Ok(_) => return Err(self.fault()),
            Err(abrupt) => return self.reject_capability(capability, abrupt),
        };
        let mark = self.keep(Value::Object(wrapper));
        let made =
            self.async_from_sync_reactions(wrapper, done, capability, sync, close_on_rejection);
        self.release(mark);
        made?;
        // 14. Return promiseCapability.[[Promise]].
        Ok(Value::Object(capability.promise))
    }

    /// Steps 8 to 13 of AsyncFromSyncIteratorContinuation: onFulfilled, the unwrap function;
    /// onRejected, the function that closes the sync iterator, or undefined when `done` is true
    /// or `close_on_rejection` false; then PerformPromiseThen(valueWrapper, onFulfilled,
    /// onRejected, promiseCapability). The functions are kept until the reactions hold them.
    fn async_from_sync_reactions(
        &mut self,
        wrapper: ObjectRef,
        done: bool,
        capability: PromiseCapability,
        sync: IteratorRecord,
        close_on_rejection: bool,
    ) -> Completion<()> {
        let realm = self.current_realm()?;
        // 8 and 9. unwrap, CreateBuiltinFunction(unwrap, 1, "", « »).
        let on_fulfilled = self.create_builtin_closure(
            BuiltinId::AsyncIteration(AsyncIterationBuiltin::Unwrap),
            Slots::AsyncFromSync(AsyncFromSyncSlots::Unwrap { done }),
            1,
            realm,
        )?;
        let mark = self.keep(Value::Object(on_fulfilled));
        // 11. If done is true, or if closeOnRejection is false, then let onRejected be
        // undefined. 12. Else, closeIterator, CreateBuiltinFunction(closeIterator, 1, "", « »).
        let on_rejected = if done || !close_on_rejection {
            Ok(Value::Undefined)
        } else {
            self.create_builtin_closure(
                BuiltinId::AsyncIteration(AsyncIterationBuiltin::Close),
                Slots::AsyncFromSync(AsyncFromSyncSlots::Close {
                    iterator: sync.iterator,
                }),
                1,
                realm,
            )
            .map(Value::Object)
        };
        let then = on_rejected.and_then(|on_rejected| {
            self.keep(on_rejected);
            // 13. Perform PerformPromiseThen(valueWrapper, onFulfilled, onRejected,
            // promiseCapability).
            self.perform_promise_then(
                wrapper,
                Value::Object(on_fulfilled),
                on_rejected,
                Some(capability),
            )
        });
        self.release(mark);
        then.map(|_| ())
    }

    /// The unwrap function `function` called with `value` (27.1.5.4, step 8):
    /// CreateIteratorResultObject(value, done) with the `done` its slots hold.
    fn async_from_sync_unwrap(&mut self, function: ObjectRef, value: Value) -> Completion<Value> {
        let done = match self.closure_slots(function)? {
            Slots::AsyncFromSync(AsyncFromSyncSlots::Unwrap { done }) => *done,
            _ => return Err(self.fault()),
        };
        // a. Return CreateIteratorResultObject(value, done).
        Ok(Value::Object(self.create_iter_result_object(value, done)?))
    }

    /// The function `function` that closes the sync iterator, called with `error` (27.1.5.4,
    /// step 12.a): IteratorClose(syncIteratorRecord, ThrowCompletion(error)), whose answer is the
    /// throw of `error` unless the run ends.
    fn async_from_sync_close(&mut self, function: ObjectRef, error: Value) -> Completion<Value> {
        let iterator = match self.closure_slots(function)? {
            Slots::AsyncFromSync(AsyncFromSyncSlots::Close { iterator }) => *iterator,
            _ => return Err(self.fault()),
        };
        let record = IteratorRecord {
            iterator,
            next_method: Value::Undefined,
            done: false,
        };
        // i. Return ? IteratorClose(syncIteratorRecord, ThrowCompletion(error)).
        self.iterator_close(&record, Err(Abrupt::Throw(error)))
    }

    /// NewPromiseCapability(%Promise%) (`#sec-newpromisecapability`) made as Construct of
    /// `%Promise%` makes it, without the call of the executor: with the intrinsic `%Promise%` as
    /// the constructor and the new target, its "prototype" (non-writable, non-configurable) and
    /// the executor are not observable, so a new pending promise with the [[Prototype]]
    /// `%Promise.prototype%` and its resolving functions are the same record, as `AsyncStart`
    /// makes them. No call is made, so no depth bound refuses it: the `!` of the steps holds. With
    /// the resolving functions settled at once, the calls of the sync iterator's methods are the
    /// only calls of the method that a depth bound counts.
    fn intrinsic_capability(&mut self) -> Completion<PromiseCapability> {
        let prototype = self.current_intrinsic(Intrinsic::PromisePrototype)?;
        let promise = self.new_promise_object(prototype)?;
        let mark = self.keep(Value::Object(promise));
        let functions = self.create_resolving_functions(promise);
        self.release(mark);
        let (resolve, reject) = functions?;
        Ok(PromiseCapability {
            promise,
            resolve: Value::Object(resolve),
            reject: Value::Object(reject),
        })
    }

    /// PromiseResolve(%Promise%, value) (`#sec-promise-resolve`) with the capability of step 2
    /// made by [`Agent::intrinsic_capability`]; step 3, the call of its resolve function, runs
    /// its steps at once (`call_settling_function`), with no activation of its own.
    fn intrinsic_promise_resolve(&mut self, value: Value) -> Completion<Value> {
        // 1. If IsPromise(resolution) is true, then a. Let resolutionCtor be ? Get(resolution,
        // "constructor"). b. If SameValue(resolutionCtor, ctor) is true, return resolution.
        if let Value::Object(object) = value
            && self.is_promise(value)?
        {
            let constructor = self.current_intrinsic(Intrinsic::Promise)?;
            let key = self.name_key("constructor")?;
            let resolution_constructor = self.get(object, key)?;
            if self.same_value(resolution_constructor, Value::Object(constructor))? {
                return Ok(value);
            }
        }
        // 2. Let promiseCapability be ? NewPromiseCapability(ctor). 3. Perform ?
        // Call(promiseCapability.[[Resolve]], undefined, « resolution »). 4. Return
        // promiseCapability.[[Promise]].
        let capability = self.intrinsic_capability()?;
        let mark = self.keep_capability(&capability);
        let called = self.call_settling_function(capability.resolve, value);
        self.release(mark);
        called?;
        Ok(Value::Object(capability.promise))
    }

    /// IfAbruptRejectPromise (`#sec-ifabruptrejectpromise`) of `abrupt`: for a throw, the call
    /// of [[Reject]] of `capability` with its value, its steps run at once
    /// (`call_settling_function`), then its promise; a termination passes up.
    fn reject_capability(
        &mut self,
        capability: PromiseCapability,
        abrupt: Abrupt,
    ) -> Completion<Value> {
        let Abrupt::Throw(reason) = abrupt else {
            return Err(abrupt);
        };
        // A throw recorded inside the call is ended by `builtin_call` when the method answers
        // normally (decision 13).
        self.call_settling_function(capability.reject, reason)?;
        Ok(Value::Object(capability.promise))
    }

    /// The call of [[Resolve]] of `capability` with `value`, its steps run at once
    /// (`call_settling_function`), then its promise.
    fn resolve_capability(
        &mut self,
        capability: PromiseCapability,
        value: Value,
    ) -> Completion<Value> {
        let mark = self.keep(value);
        let called = self.call_settling_function(capability.resolve, value);
        self.release(mark);
        called?;
        Ok(Value::Object(capability.promise))
    }
}

/// What steps 5 to 11 of `return` and `throw` of `%AsyncFromSyncIteratorPrototype%` answer: the
/// result object for AsyncFromSyncIteratorContinuation, a value to resolve the capability with
/// at once, or an abrupt completion to reject it with.
enum Answer {
    Result(ObjectRef),
    Resolve(Value),
    Reject(Abrupt),
}

#[cfg(test)]
mod tests {
    use super::sync_record_bytes;
    use crate::host::hooks::test_host::{agent_with_realm, enter};
    use crate::object::property::Key;
    use crate::realm::intrinsics::Intrinsic;
    use crate::value::Value;

    /// The box of [[SyncIteratorRecord]] is charged its size, which the integration test of the
    /// charge (`tests/for_await.rs`, `SYNC_RECORD_BYTES`) states for this target.
    #[test]
    fn an_async_from_sync_iterator_is_charged_its_record() {
        assert_eq!(sync_record_bytes(), 32);
    }

    /// `%AsyncIteratorPrototype%` (27.1.4) has the [[Prototype]] `%Object.prototype%` and a
    /// `[%Symbol.asyncIterator%]` that answers its this value (27.1.4.2);
    /// `%AsyncFromSyncIteratorPrototype%` (27.1.5.2) has the [[Prototype]]
    /// `%AsyncIteratorPrototype%`, inherits that method, and has "next", "return" and "throw".
    /// No script reaches either before the async generators, so the test reads them here.
    #[test]
    fn the_async_from_sync_prototype_inherits_a_symbol_async_iterator_that_answers_this() {
        let (mut agent, realm) = agent_with_realm();
        enter(&mut agent, &realm);
        let handle = agent.heap.roots.get_realm(&realm.0).unwrap();
        let async_iterator = agent
            .intrinsic(handle, Intrinsic::AsyncIteratorPrototype)
            .unwrap();
        let from_sync = agent
            .intrinsic(handle, Intrinsic::AsyncFromSyncIteratorPrototype)
            .unwrap();
        let object_prototype = agent.intrinsic(handle, Intrinsic::ObjectPrototype).unwrap();
        assert_eq!(
            agent.get_prototype_of(async_iterator),
            Ok(Some(object_prototype))
        );
        assert_eq!(agent.get_prototype_of(from_sync), Ok(Some(async_iterator)));
        let symbols = agent.well_known().unwrap();
        let method = agent
            .get(from_sync, Key::Symbol(symbols.async_iterator))
            .unwrap();
        assert!(matches!(method, Value::Object(_)), "{method:?}");
        assert_eq!(
            agent.call_value(method, Value::Object(from_sync), &[]),
            Ok(Value::Object(from_sync))
        );
        assert_eq!(
            agent.call_value(method, Value::Number(1.0), &[]),
            Ok(Value::Number(1.0))
        );
        for name in ["next", "return", "throw"] {
            let key = agent.name_key(name).unwrap();
            let own = agent.get_own_property(from_sync, key).unwrap();
            assert!(own.is_some(), "{name}");
        }
    }
}
