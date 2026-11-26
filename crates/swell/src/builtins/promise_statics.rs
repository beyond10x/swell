//! The statics of `%Promise%` that `story:promise-constructor-then-and-jobs` left out
//! (`#sec-properties-of-the-promise-constructor`), written by
//! `story:promise-statics-finally-and-aggregate-error`: `Promise.all` (`#sec-promise.all`),
//! `Promise.allSettled` (`#sec-promise.allsettled`), `Promise.any` (`#sec-promise.any`) and
//! `Promise.race` (`#sec-promise.race`), with GetPromiseResolve (`#sec-getpromiseresolve`),
//! PerformPromiseAll, PerformPromiseAllSettled, PerformPromiseAny and PerformPromiseRace
//! (`#sec-performpromiseall`, `#sec-performpromiseallsettled`, `#sec-performpromiseany`,
//! `#sec-performpromiserace`) and IfAbruptRejectPromise (`#sec-ifabruptrejectpromise`);
//! `Promise.try` (`#sec-promise.try`) and `Promise.withResolvers` (`#sec-promise.withResolvers`);
//! and the steps of the element functions (`#sec-promise.all-resolve-element-functions`,
//! `#sec-promise.allsettled-resolve-element-functions`,
//! `#sec-promise.allsettled-reject-element-functions`,
//! `#sec-promise.any-reject-element-functions`). `%Promise%` gets them in `builtins/promise.rs`.
//!
//! The element functions are built-in functions with slots ([`PromiseSlots::Element`],
//! `object/builtin_function.rs`): [[Index]], [[AlreadyCalled]], and the record the functions of
//! one call share in an `Rc` ([`ElementRecord`]). The list values or errors of the draft is an
//! Array of the current realm held by that record and by the call while it runs: it grows as
//! elements are read, each growth charged before it is made as an Array's is, and the sweep
//! that frees it credits it, so a combined promise over n elements traces its list once, not
//! once per element function. CreateArrayFromList answers that Array as it is: no script reaches
//! it before every element is in, so nothing tells it from a new Array made of the list. The
//! call charges the record's box while it holds the record, and each element function its own
//! count of it ([`PromiseSlots::bytes`]).
//!
//! The calls of the capability's [[Resolve]] and [[Reject]] that the statics and the element
//! functions make (IfAbruptRejectPromise and the last element) go through
//! [`Agent::call_settling_function`], as a job's do: when the capability is the one of
//! `%Promise%`, its resolving function runs at once, so that a combined promise whose call
//! threw at the re-entry bound is still rejected with that `RangeError`.

use alloc::rc::Rc;
use core::cell::Cell;

use crate::builtins::object::argument;
use crate::heap::arena::Handle;
use crate::host::agent::Agent;
use crate::host::outcome::{Abrupt, Completion};
use crate::host::{ErrorKind, Host, PromiseCapability};
use crate::object::builtin_function::{BuiltinId, ElementRecord, PromiseSlots};
use crate::object::ordinary::OrdinaryData;
use crate::object::property::Key;
use crate::ops::iteration::IteratorRecord;
use crate::realm::intrinsics::Intrinsic;
use crate::value::{ObjectRef, Value};

/// The builtins of this module, by their steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PromiseStaticsBuiltin {
    /// `Promise.all ( iterable )` (27.5.4.1).
    All,
    /// `Promise.allSettled ( iterable )` (27.5.4.2).
    AllSettled,
    /// `Promise.any ( iterable )` (27.5.4.3).
    Any,
    /// `Promise.race ( iterable )` (27.5.4.5).
    Race,
    /// `Promise.try ( callback, ...args )` (27.5.4.8).
    Try,
    /// `Promise.withResolvers ( )` (27.5.4.9).
    WithResolvers,
    /// fulfilledSteps of PerformPromiseAll (27.5.4.1.2).
    AllResolveElement,
    /// fulfilledSteps of PerformPromiseAllSettled (27.5.4.2.1).
    AllSettledResolveElement,
    /// rejectedSteps of PerformPromiseAllSettled (27.5.4.2.1).
    AllSettledRejectElement,
    /// rejectedSteps of PerformPromiseAny (27.5.4.3.1).
    AnyRejectElement,
}

/// Which of the four statics over an iterable runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Combinator {
    All,
    AllSettled,
    Any,
    Race,
}

impl<H: Host> Agent<H> {
    /// The steps of the builtin `id` of this module.
    pub(crate) fn run_promise_statics_builtin(
        &mut self,
        id: PromiseStaticsBuiltin,
        function: ObjectRef,
        this: Value,
        args: &[Value],
    ) -> Completion<Value> {
        let first = argument(args, 0);
        match id {
            PromiseStaticsBuiltin::All => self.promise_combinator(Combinator::All, this, first),
            PromiseStaticsBuiltin::AllSettled => {
                self.promise_combinator(Combinator::AllSettled, this, first)
            }
            PromiseStaticsBuiltin::Any => self.promise_combinator(Combinator::Any, this, first),
            PromiseStaticsBuiltin::Race => self.promise_combinator(Combinator::Race, this, first),
            PromiseStaticsBuiltin::Try => self.promise_try(this, args),
            PromiseStaticsBuiltin::WithResolvers => self.promise_with_resolvers(this),
            PromiseStaticsBuiltin::AllResolveElement
            | PromiseStaticsBuiltin::AllSettledResolveElement
            | PromiseStaticsBuiltin::AllSettledRejectElement
            | PromiseStaticsBuiltin::AnyRejectElement => self.element_function(id, function, first),
        }
    }

    /// `Promise.all`, `allSettled`, `any` or `race` ( iterable ), steps 1 and 2: ctor, the this
    /// value, and its capability, kept while the rest runs.
    fn promise_combinator(
        &mut self,
        kind: Combinator,
        ctor: Value,
        iterable: Value,
    ) -> Completion<Value> {
        // 1. Let ctor be the this value. 2. Let promiseCapability be ? NewPromiseCapability(ctor).
        let capability = self.new_promise_capability(ctor)?;
        let mark = self.keep_capability(&capability);
        let answer = self.promise_combinator_resolve(kind, ctor, iterable, capability);
        self.release(mark);
        answer
    }

    /// Steps 3 and 4: promiseResolve, kept while the rest runs.
    fn promise_combinator_resolve(
        &mut self,
        kind: Combinator,
        ctor: Value,
        iterable: Value,
        capability: PromiseCapability,
    ) -> Completion<Value> {
        // 3. Let promiseResolve be Completion(GetPromiseResolve(ctor)). 4.
        // IfAbruptRejectPromise(promiseResolve, promiseCapability).
        let resolve = match self.get_promise_resolve(ctor) {
            Ok(resolve) => resolve,
            Err(abrupt) => return self.if_abrupt_reject_promise(abrupt, capability),
        };
        let mark = self.keep(resolve);
        let answer = self.promise_combinator_iterate(kind, ctor, iterable, capability, resolve);
        self.release(mark);
        answer
    }

    /// Steps 5 to 9: the iterator, kept while the loop runs, the loop, and the close of the
    /// iterator when the loop's completion is a throw and the iterator is not done.
    fn promise_combinator_iterate(
        &mut self,
        kind: Combinator,
        ctor: Value,
        iterable: Value,
        capability: PromiseCapability,
        resolve: Value,
    ) -> Completion<Value> {
        // 5. Let iteratorRecord be Completion(GetIterator(iterable, sync)). 6.
        // IfAbruptRejectPromise(iteratorRecord, promiseCapability).
        let mut record = match self.get_iterator(iterable) {
            Ok(record) => record,
            Err(abrupt) => return self.if_abrupt_reject_promise(abrupt, capability),
        };
        let mark = self.keep(Value::Object(record.iterator));
        self.keep(record.next_method);
        // 7. Let result be Completion(PerformPromiseX(iteratorRecord, ctor, promiseCapability,
        // promiseResolve)).
        let mut result = if kind == Combinator::Race {
            self.perform_promise_race(&mut record, ctor, capability, resolve)
        } else {
            self.perform_promise_combinator(kind, &mut record, ctor, capability, resolve)
        };
        // 8. If result is an abrupt completion, then a. If iteratorRecord.[[Done]] is false, set
        // result to Completion(IteratorClose(iteratorRecord, result)). A termination ends the
        // call as it is (`docs/design/engine.md`, decision 3, item 4).
        if matches!(result, Err(Abrupt::Throw(_))) && !record.done {
            result = self.iterator_close(&record, result);
        }
        // b. IfAbruptRejectPromise(result, promiseCapability). 9. Return ! result.
        let answer = match result {
            Ok(value) => Ok(value),
            Err(abrupt) => self.if_abrupt_reject_promise(abrupt, capability),
        };
        self.release(mark);
        answer
    }

    /// GetPromiseResolve ( promiseCtor ) (`#sec-getpromiseresolve`).
    fn get_promise_resolve(&mut self, ctor: Value) -> Completion<Value> {
        let Value::Object(ctor) = ctor else {
            return Err(self.fault());
        };
        // 1. Let promiseResolve be ? Get(promiseCtor, "resolve").
        let key = self.name_key("resolve")?;
        let resolve = self.get(ctor, key)?;
        // 2. If IsCallable(promiseResolve) is false, throw a TypeError exception.
        if !self.is_callable(resolve)? {
            return Err(self.throw_error(
                ErrorKind::TypeError,
                "the resolve of the promise constructor is not a function",
            ));
        }
        // 3. Return promiseResolve.
        Ok(resolve)
    }

    /// IfAbruptRejectPromise ( value, capability ) (`#sec-ifabruptrejectpromise`) for the
    /// abrupt completion `abrupt`: a throw rejects the capability, which the caller keeps, with
    /// its value; a termination is answered as it is.
    fn if_abrupt_reject_promise(
        &mut self,
        abrupt: Abrupt,
        capability: PromiseCapability,
    ) -> Completion<Value> {
        let Abrupt::Throw(reason) = abrupt else {
            return Err(abrupt);
        };
        // a. Perform ? Call(capability.[[Reject]], undefined, « value.[[Value]] »).
        let mark = self.keep(reason);
        let called = self.call_settling_function(capability.reject, reason);
        self.release(mark);
        called?;
        // b. Return capability.[[Promise]].
        Ok(Value::Object(capability.promise))
    }

    /// PerformPromiseAll, PerformPromiseAllSettled or PerformPromiseAny, steps 1 to 3: the list
    /// of values or errors, kept, and the record the element functions share, whose box is
    /// charged while the call holds it.
    fn perform_promise_combinator(
        &mut self,
        kind: Combinator,
        record: &mut IteratorRecord,
        ctor: Value,
        capability: PromiseCapability,
        resolve: Value,
    ) -> Completion<Value> {
        // 1. Let values be a new empty List.
        let values = self.array_create(0.0, None)?;
        let mark = self.keep(Value::Object(values));
        // 2. Let remainingElementsCount be the Record { [[Value]]: 1 }.
        let bytes = ElementRecord::bytes();
        let answer = self.charge_bytes(bytes).and_then(|()| {
            let shared = Rc::new(ElementRecord {
                values,
                capability,
                remaining: Cell::new(1),
            });
            let answer = self.perform_combinator_loop(kind, record, ctor, &shared, resolve);
            self.heap.accounting.credit(bytes);
            answer
        });
        self.release(mark);
        answer
    }

    /// Steps 3 and 4 of PerformPromiseAll, PerformPromiseAllSettled and PerformPromiseAny: the
    /// loop over the iterator.
    fn perform_combinator_loop(
        &mut self,
        kind: Combinator,
        record: &mut IteratorRecord,
        ctor: Value,
        shared: &Rc<ElementRecord>,
        resolve: Value,
    ) -> Completion<Value> {
        // 3. Let index be 0.
        let mut index: u32 = 0;
        // 4. Repeat,
        loop {
            // a. Let next be ? IteratorStepValue(iteratorRecord).
            let Some(next) = self.iterator_step_value(record)? else {
                // b. If next is done, then i. Set remainingElementsCount.[[Value]] to
                // remainingElementsCount.[[Value]] - 1. ii. If remainingElementsCount.[[Value]]
                // = 0, then settle the combined promise.
                if count_down(shared) {
                    self.settle_combined(kind, shared)?;
                }
                // iii. Return resultCapability.[[Promise]].
                return Ok(Value::Object(shared.capability.promise));
            };
            // An Array holds at most 2^32 - 1 elements.
            if index == u32::MAX {
                return Err(self.throw_error(ErrorKind::RangeError, "Invalid array length"));
            }
            let mark = self.keep(next);
            let step = self.combinator_element(kind, ctor, shared, resolve, next, index);
            self.release(mark);
            step?;
            index += 1;
        }
    }

    /// Steps 4.c to 4.n for the element `next` at `index`: undefined appended to the list, the
    /// call of promiseResolve, the element functions, the count raised, and `then` invoked.
    fn combinator_element(
        &mut self,
        kind: Combinator,
        ctor: Value,
        shared: &Rc<ElementRecord>,
        resolve: Value,
        next: Value,
        index: u32,
    ) -> Completion<()> {
        // c. Append undefined to values.
        self.create_data_property_or_throw(shared.values, Key::Index(index), Value::Undefined)?;
        // d. Let nextPromise be ? Call(promiseResolve, ctor, « next »).
        let next_promise = self.call_value(resolve, ctor, &[next])?;
        let mark = self.keep(next_promise);
        let answer = self
            .element_functions(kind, shared, index)
            .and_then(|handlers| {
                self.keep(handlers.0);
                self.keep(handlers.1);
                // Set remainingElementsCount.[[Value]] to remainingElementsCount.[[Value]] + 1.
                shared
                    .remaining
                    .set(shared.remaining.get().saturating_add(1));
                // Perform ? Invoke(nextPromise, "then", « onFulfilled, onRejected »).
                let key = self.name_key("then")?;
                self.invoke(next_promise, key, &[handlers.0, handlers.1])
                    .map(|_| ())
            });
        self.release(mark);
        answer
    }

    /// The two handlers of the element at `index`: for `all` its resolve element function and
    /// the capability's [[Reject]]; for `allSettled` its resolve and reject element functions,
    /// which share one [[AlreadyCalled]]; for `any` the capability's [[Resolve]] and its reject
    /// element function. Each function is made in the current realm, of `length` 1, its `name`
    /// the empty String.
    fn element_functions(
        &mut self,
        kind: Combinator,
        shared: &Rc<ElementRecord>,
        index: u32,
    ) -> Completion<(Value, Value)> {
        let realm = self.current_realm()?;
        let capability = shared.capability;
        let already_called = Rc::new(Cell::new(false));
        match kind {
            Combinator::All => {
                let on_fulfilled = self.element_function_object(
                    PromiseStaticsBuiltin::AllResolveElement,
                    shared,
                    index,
                    already_called,
                    realm,
                )?;
                Ok((Value::Object(on_fulfilled), capability.reject))
            }
            Combinator::AllSettled => {
                let on_fulfilled = self.element_function_object(
                    PromiseStaticsBuiltin::AllSettledResolveElement,
                    shared,
                    index,
                    Rc::clone(&already_called),
                    realm,
                )?;
                let mark = self.keep(Value::Object(on_fulfilled));
                let on_rejected = self.element_function_object(
                    PromiseStaticsBuiltin::AllSettledRejectElement,
                    shared,
                    index,
                    already_called,
                    realm,
                );
                self.release(mark);
                Ok((Value::Object(on_fulfilled), Value::Object(on_rejected?)))
            }
            Combinator::Any => {
                let on_rejected = self.element_function_object(
                    PromiseStaticsBuiltin::AnyRejectElement,
                    shared,
                    index,
                    already_called,
                    realm,
                )?;
                Ok((capability.resolve, Value::Object(on_rejected)))
            }
            Combinator::Race => Err(self.fault()),
        }
    }

    /// CreateBuiltinFunction(steps, 1, "", « [[AlreadyCalled]], [[Index]] ») with its slots.
    fn element_function_object(
        &mut self,
        id: PromiseStaticsBuiltin,
        shared: &Rc<ElementRecord>,
        index: u32,
        already_called: Rc<Cell<bool>>,
        realm: Handle,
    ) -> Completion<ObjectRef> {
        self.create_promise_closure(
            BuiltinId::PromiseStatics(id),
            PromiseSlots::Element {
                index,
                already_called,
                record: Rc::clone(shared),
            },
            1,
            realm,
        )
    }

    /// The step that settles the combined promise once [[RemainingElements]] is 0: for `all`
    /// and `allSettled` the call of [[Resolve]] with CreateArrayFromList(values); for `any` the
    /// call of [[Reject]] with a new AggregateError of the current realm whose "errors" is
    /// CreateArrayFromList(errors).
    fn settle_combined(&mut self, kind: Combinator, shared: &ElementRecord) -> Completion<Value> {
        let capability = shared.capability;
        match kind {
            Combinator::All | Combinator::AllSettled => {
                // Let valuesArray be CreateArrayFromList(values). Perform ?
                // Call(resultCapability.[[Resolve]], undefined, « valuesArray »).
                self.call_settling_function(capability.resolve, Value::Object(shared.values))
            }
            Combinator::Any => {
                // Let aggregateError be a newly created AggregateError object. Perform !
                // DefinePropertyOrThrow(aggregateError, "errors", PropertyDescriptor {
                // [[Configurable]]: true, [[Enumerable]]: false, [[Writable]]: true, [[Value]]:
                // CreateArrayFromList(errors) }).
                let realm = self.current_realm()?;
                let error = self.create_aggregate_error(realm, None, shared.values)?;
                // Perform ? Call(resultCapability.[[Reject]], undefined, « aggregateError »).
                let mark = self.keep(Value::Object(error));
                let called = self.call_settling_function(capability.reject, Value::Object(error));
                self.release(mark);
                called
            }
            Combinator::Race => Err(self.fault()),
        }
    }

    /// The steps of an element function `function` called with `argument`: fulfilledSteps of
    /// PerformPromiseAll and PerformPromiseAllSettled, rejectedSteps of
    /// PerformPromiseAllSettled and PerformPromiseAny.
    fn element_function(
        &mut self,
        id: PromiseStaticsBuiltin,
        function: ObjectRef,
        argument: Value,
    ) -> Completion<Value> {
        let (index, already_called, shared) = match self.promise_closure_slots(function)? {
            PromiseSlots::Element {
                index,
                already_called,
                record,
            } => (*index, Rc::clone(already_called), Rc::clone(record)),
            PromiseSlots::Finally { .. } | PromiseSlots::Thunk { .. } => {
                return Err(self.fault());
            }
        };
        // 1. Let activeFunc be the active function object. 2. If
        // activeFunc.[[AlreadyCalled]] is true, return undefined.
        if already_called.get() {
            return Ok(Value::Undefined);
        }
        // 3. Set activeFunc.[[AlreadyCalled]] to true.
        already_called.set(true);
        // allSettled: Let obj be OrdinaryObjectCreate(%Object.prototype%), with "status" and
        // "value" or "reason".
        let (value, kind) = match id {
            PromiseStaticsBuiltin::AllResolveElement => (argument, Combinator::All),
            PromiseStaticsBuiltin::AnyRejectElement => (argument, Combinator::Any),
            PromiseStaticsBuiltin::AllSettledResolveElement => (
                self.settled_record("fulfilled", "value", argument)?,
                Combinator::AllSettled,
            ),
            PromiseStaticsBuiltin::AllSettledRejectElement => (
                self.settled_record("rejected", "reason", argument)?,
                Combinator::AllSettled,
            ),
            PromiseStaticsBuiltin::All
            | PromiseStaticsBuiltin::AllSettled
            | PromiseStaticsBuiltin::Any
            | PromiseStaticsBuiltin::Race
            | PromiseStaticsBuiltin::Try
            | PromiseStaticsBuiltin::WithResolvers => return Err(self.fault()),
        };
        // Let thisIndex be activeFunc.[[Index]]. Set values[thisIndex] to value. The callee,
        // whose slots hold the list, is in the activation; the record made for allSettled is
        // kept until it is in the list.
        let mark = self.keep(value);
        let set = self.create_data_property_or_throw(shared.values, Key::Index(index), value);
        self.release(mark);
        set?;
        // Set remainingElementsCount.[[Value]] to remainingElementsCount.[[Value]] - 1. If
        // remainingElementsCount.[[Value]] = 0, then return ? the settling call.
        if count_down(&shared) {
            return self.settle_combined(kind, &shared);
        }
        // Return undefined.
        Ok(Value::Undefined)
    }

    /// The object an element function of `allSettled` records: an ordinary object of the
    /// current realm with "status" `status` and `key` `value`.
    fn settled_record(&mut self, status: &str, key: &str, value: Value) -> Completion<Value> {
        let prototype = self.current_intrinsic(Intrinsic::ObjectPrototype)?;
        let object = self.ordinary_object_create(Some(prototype), OrdinaryData::Plain)?;
        let mark = self.keep(Value::Object(object));
        let made = self.new_string_str(status).and_then(|status| {
            // Perform ! CreateDataPropertyOrThrow(obj, "status", status).
            let status_key = self.name_key("status")?;
            self.create_data_property_or_throw(object, status_key, Value::String(status))?;
            // Perform ! CreateDataPropertyOrThrow(obj, key, value).
            let key = self.name_key(key)?;
            self.create_data_property_or_throw(object, key, value)
        });
        self.release(mark);
        made.map(|()| Value::Object(object))
    }

    /// PerformPromiseRace ( iteratorRecord, ctor, resultCapability, promiseResolve )
    /// (`#sec-performpromiserace`).
    fn perform_promise_race(
        &mut self,
        record: &mut IteratorRecord,
        ctor: Value,
        capability: PromiseCapability,
        resolve: Value,
    ) -> Completion<Value> {
        // 1. Repeat,
        loop {
            // a. Let next be ? IteratorStepValue(iteratorRecord).
            let Some(next) = self.iterator_step_value(record)? else {
                // b. If next is done, then return resultCapability.[[Promise]].
                return Ok(Value::Object(capability.promise));
            };
            // c. Let nextPromise be ? Call(promiseResolve, ctor, « next »).
            let next_promise = self.call_value(resolve, ctor, &[next])?;
            // d. Perform ? Invoke(nextPromise, "then", « resultCapability.[[Resolve]],
            // resultCapability.[[Reject]] »).
            let mark = self.keep(next_promise);
            let invoked = self.name_key("then").and_then(|key| {
                self.invoke(next_promise, key, &[capability.resolve, capability.reject])
            });
            self.release(mark);
            invoked?;
        }
    }

    /// `Promise.try ( callback, ...args )` (`#sec-promise.try`).
    fn promise_try(&mut self, ctor: Value, args: &[Value]) -> Completion<Value> {
        // 1. Let ctor be the this value. 2. If ctor is not an Object, throw a TypeError
        // exception.
        let Value::Object(constructor) = ctor else {
            return Err(self.throw_error(
                ErrorKind::TypeError,
                "Promise.try called on a value that is not an object",
            ));
        };
        // 3. Let status be Completion(Call(callback, undefined, args)).
        let status = self.call_value(
            argument(args, 0),
            Value::Undefined,
            args.get(1..).unwrap_or(&[]),
        );
        match status {
            // 4. If status is an abrupt completion, then
            Err(Abrupt::Throw(reason)) => {
                let mark = self.keep(reason);
                let answer = self.reject_with(ctor, reason);
                self.release(mark);
                answer
            }
            Err(abrupt) => Err(abrupt),
            // 5. Else, return ? PromiseResolve(ctor, ! status).
            Ok(value) => {
                let mark = self.keep(value);
                let answer = self.promise_resolve(constructor, value);
                self.release(mark);
                answer
            }
        }
    }

    /// Step 4 of `Promise.try` with `reason` kept: a. Let promiseCapability be ?
    /// NewPromiseCapability(ctor). b. Perform ? Call(promiseCapability.[[Reject]], undefined, «
    /// status.[[Value]] »). c. Return promiseCapability.[[Promise]].
    fn reject_with(&mut self, ctor: Value, reason: Value) -> Completion<Value> {
        let capability = self.new_promise_capability(ctor)?;
        let mark = self.keep_capability(&capability);
        let called = self.call_settling_function(capability.reject, reason);
        self.release(mark);
        called?;
        Ok(Value::Object(capability.promise))
    }

    /// `Promise.withResolvers ( )` (`#sec-promise.withResolvers`).
    fn promise_with_resolvers(&mut self, ctor: Value) -> Completion<Value> {
        // 1. Let ctor be the this value. 2. Let promiseCapability be ?
        // NewPromiseCapability(ctor).
        let capability = self.new_promise_capability(ctor)?;
        let mark = self.keep_capability(&capability);
        let made = self.resolvers_object(capability);
        self.release(mark);
        made.map(Value::Object)
    }

    /// Steps 3 to 7 of `Promise.withResolvers`, with the capability kept.
    fn resolvers_object(&mut self, capability: PromiseCapability) -> Completion<ObjectRef> {
        // 3. Let obj be OrdinaryObjectCreate(%Object.prototype%).
        let prototype = self.current_intrinsic(Intrinsic::ObjectPrototype)?;
        let object = self.ordinary_object_create(Some(prototype), OrdinaryData::Plain)?;
        let mark = self.keep(Value::Object(object));
        // 4 to 6. Perform ! CreateDataPropertyOrThrow(obj, "promise", ...), then "resolve" and
        // "reject".
        let made = [
            ("promise", Value::Object(capability.promise)),
            ("resolve", capability.resolve),
            ("reject", capability.reject),
        ]
        .into_iter()
        .try_for_each(|(name, value)| {
            let key = self.name_key(name)?;
            self.create_data_property_or_throw(object, key, value)
        });
        self.release(mark);
        // 7. Return obj.
        made.map(|()| object)
    }
}

/// Set [[RemainingElements]] of `shared` to itself - 1; whether it is now 0.
fn count_down(shared: &ElementRecord) -> bool {
    let remaining = shared.remaining.get().saturating_sub(1);
    shared.remaining.set(remaining);
    remaining == 0
}
