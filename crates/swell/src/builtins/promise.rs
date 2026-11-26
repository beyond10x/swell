//! The Promise objects of slice 10 (`#sec-promise-objects`), written by
//! `story:promise-constructor-then-and-jobs`: `%Promise%` (`#sec-promise-constructor`, `Promise (
//! executor )`, `#sec-promise-executor`) with the properties of the constructor
//! (`#sec-properties-of-the-promise-constructor`) `reject` (`#sec-promise.reject`), `resolve`
//! (`#sec-promise.resolve`) and `get [%Symbol.species%]`
//! (`#sec-get-promise-%symbol.species%`); `%Promise.prototype%`
//! (`#sec-properties-of-the-promise-prototype-object`) with `catch`
//! (`#sec-promise.prototype.catch`), `constructor` (`#sec-promise.prototype.constructor`), `then`
//! (`#sec-promise.prototype.then`) and `[%Symbol.toStringTag%]`
//! (`#sec-promise.prototype-%symbol.tostringtag%`); and the steps of the built-in functions with
//! slots the abstract operations make (`ops/promise.rs`): resolveSteps and rejectSteps of
//! CreateResolvingFunctions (`#sec-createresolvingfunctions`, steps 2 and 4) and executorClosure
//! of NewPromiseCapability (`#sec-newpromisecapability`, step 4).
//!
//! The pinned draft lets both resolving functions capture one record promiseOrEmpty whose
//! [[Value]] becomes empty when either runs; the design keeps the promise in each function's
//! slots and shares one [[AlreadyResolved]] flag between the two, which answers the same: the
//! second call of either returns undefined.
//!
//! `story:promise-statics-finally-and-aggregate-error` adds `Promise.prototype.finally`
//! (`#sec-promise.prototype.finally`) with the steps of its closures thenFinally, catchFinally,
//! valueThunk and thrower, built-in functions with slots of their own kind
//! (`PromiseSlots::Finally` and `PromiseSlots::Thunk`, `object/builtin_function.rs`), and puts
//! `all`, `allSettled`, `any`, `race`, `try` and `withResolvers` (`builtins/promise_statics.rs`)
//! on `%Promise%` beside `reject` and `resolve`, in the order of the clauses.
//!
//! A job calls the resolve or reject function of its capability from Rust
//! ([`Agent::call_settling_function`]): when that function is one of the resolving functions
//! made here, its steps run at once, after the step of a call, in its realm, without an
//! activation of its own, so that a job whose handler threw at the re-entry bound still settles
//! its derived promise; any other function is called as every function is.

use alloc::rc::Rc;

use crate::builtins::promise_statics::PromiseStaticsBuiltin;
use crate::host::agent::Agent;
use crate::host::outcome::{Abrupt, Completion};
use crate::host::{ErrorKind, Host};
use crate::object::ObjectKind;
use crate::object::builtin_function::{Behaviour, BuiltinFunction, BuiltinId, PromiseSlots, Slots};
use crate::object::ordinary::OrdinaryData;
use crate::object::property::{Key, Property, Slot};
use crate::realm::intrinsics::{Intrinsic, Making};
use crate::value::{ObjectRef, Value};

/// The builtins of this module, by their steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PromiseBuiltin {
    /// `Promise ( executor )` (27.5.3.1).
    Constructor,
    /// `Promise.reject ( r )` (27.5.4.6).
    Reject,
    /// `Promise.resolve ( x )` (27.5.4.7).
    Resolve,
    /// `get Promise [ %Symbol.species% ]` (27.5.4.10).
    Species,
    /// `Promise.prototype.catch ( onRejected )` (27.5.5.1).
    Catch,
    /// `Promise.prototype.then ( onFulfilled, onRejected )` (27.5.5.4).
    Then,
    /// resolveSteps of CreateResolvingFunctions (27.5.1.3, step 2), with [[Promise]] and
    /// [[AlreadyResolved]].
    ResolveFunction,
    /// rejectSteps of CreateResolvingFunctions (27.5.1.3, step 4), with [[Promise]] and
    /// [[AlreadyResolved]].
    RejectFunction,
    /// executorClosure of NewPromiseCapability (27.5.1.5, step 4), with the record
    /// resolvingFuncs.
    CapabilityExecutor,
    /// `Promise.prototype.finally ( onFinally )` (27.5.5.3).
    Finally,
    /// thenFinallyClosure of `finally` (27.5.5.3, step 6.a), with onFinally and ctor.
    ThenFinally,
    /// catchFinallyClosure of `finally` (27.5.5.3, step 6.c), with onFinally and ctor.
    CatchFinally,
    /// returnValue of thenFinallyClosure (step 6.a.iii), with value.
    ValueThunk,
    /// throwReason of catchFinallyClosure (step 6.c.iii), with reason.
    Thrower,
}

impl<H: Host> Agent<H> {
    /// `%Promise%` and `%Promise.prototype%` of the realm being made.
    pub(crate) fn create_promise_constructor(
        &mut self,
        making: Making,
    ) -> Completion<(ObjectRef, ObjectRef)> {
        // 27.5.5: "is an ordinary object", "is not a Promise instance", whose [[Prototype]] is
        // %Object.prototype%.
        let prototype =
            self.ordinary_object_create(Some(making.object_prototype), OrdinaryData::Plain)?;
        // 27.5.4: "has a "length" property whose value is 1𝔽"; its "prototype" (27.5.4.5) and
        // the prototype's "constructor" (27.5.5.2).
        let constructor = self.create_builtin_constructor(
            making,
            "Promise",
            1,
            BuiltinId::Promise(PromiseBuiltin::Constructor),
            prototype,
        )?;
        // 27.5.4.1 to 27.5.4.9 but `prototype`, in the order of the clauses; `length` as each
        // clause's parameters give it.
        for (name, length, id) in [
            (
                "all",
                1,
                BuiltinId::PromiseStatics(PromiseStaticsBuiltin::All),
            ),
            (
                "allSettled",
                1,
                BuiltinId::PromiseStatics(PromiseStaticsBuiltin::AllSettled),
            ),
            (
                "any",
                1,
                BuiltinId::PromiseStatics(PromiseStaticsBuiltin::Any),
            ),
            (
                "race",
                1,
                BuiltinId::PromiseStatics(PromiseStaticsBuiltin::Race),
            ),
            ("reject", 1, BuiltinId::Promise(PromiseBuiltin::Reject)),
            ("resolve", 1, BuiltinId::Promise(PromiseBuiltin::Resolve)),
            (
                "try",
                1,
                BuiltinId::PromiseStatics(PromiseStaticsBuiltin::Try),
            ),
            (
                "withResolvers",
                0,
                BuiltinId::PromiseStatics(PromiseStaticsBuiltin::WithResolvers),
            ),
        ] {
            self.define_builtin_method(making, constructor, name, length, id)?;
        }
        // 27.5.4.10: an accessor whose set accessor function is undefined, named "get
        // [Symbol.species]".
        let species = self.create_builtin(
            making,
            "get [Symbol.species]",
            0,
            BuiltinId::Promise(PromiseBuiltin::Species),
        )?;
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
        )?;
        for (name, length, id) in [
            ("catch", 1, PromiseBuiltin::Catch),
            ("finally", 1, PromiseBuiltin::Finally),
            ("then", 2, PromiseBuiltin::Then),
        ] {
            self.define_builtin_method(making, prototype, name, length, BuiltinId::Promise(id))?;
        }
        // 27.5.5.5: "Promise", { [[Writable]]: false, [[Enumerable]]: false, [[Configurable]]:
        // true }.
        self.define_to_string_tag(making, prototype, "Promise")?;
        Ok((constructor, prototype))
    }

    /// The steps of the builtin `id` of this module.
    pub(crate) fn run_promise_builtin(
        &mut self,
        id: PromiseBuiltin,
        function: ObjectRef,
        this: Value,
        args: &[Value],
        new_target: Option<ObjectRef>,
    ) -> Completion<Value> {
        let arg = |index: usize| args.get(index).copied().unwrap_or(Value::Undefined);
        match id {
            PromiseBuiltin::Constructor => self.promise_constructor(arg(0), new_target),
            PromiseBuiltin::Reject => self.promise_reject(this, arg(0)),
            PromiseBuiltin::Resolve => self.promise_resolve_static(this, arg(0)),
            // 27.5.4.10: "1. Return the this value."
            PromiseBuiltin::Species => Ok(this),
            PromiseBuiltin::Catch => self.promise_catch(this, arg(0)),
            PromiseBuiltin::Then => self.promise_then(this, arg(0), arg(1)),
            PromiseBuiltin::ResolveFunction => self.resolve_function(function, arg(0)),
            PromiseBuiltin::RejectFunction => self.reject_function(function, arg(0)),
            PromiseBuiltin::CapabilityExecutor => {
                self.capability_executor(function, arg(0), arg(1))
            }
            PromiseBuiltin::Finally => self.promise_finally(this, arg(0)),
            PromiseBuiltin::ThenFinally | PromiseBuiltin::CatchFinally => {
                self.finally_closure(id, function, arg(0))
            }
            PromiseBuiltin::ValueThunk | PromiseBuiltin::Thrower => {
                let value = match self.promise_closure_slots(function)? {
                    PromiseSlots::Thunk { value } => *value,
                    PromiseSlots::Element { .. } | PromiseSlots::Finally { .. } => {
                        return Err(self.fault());
                    }
                };
                // returnValue: 1. Return NormalCompletion(value). throwReason: 1. Return
                // ThrowCompletion(reason).
                if id == PromiseBuiltin::ValueThunk {
                    Ok(value)
                } else {
                    Err(Abrupt::Throw(value))
                }
            }
        }
    }

    /// `Promise.prototype.finally ( onFinally )` (`#sec-promise.prototype.finally`).
    fn promise_finally(&mut self, this: Value, on_finally: Value) -> Completion<Value> {
        // 1. Let promise be the this value. 2. If promise is not an Object, throw a TypeError
        // exception.
        let Value::Object(promise) = this else {
            return Err(self.throw_error(
                ErrorKind::TypeError,
                "Promise.prototype.finally called on a value that is not an object",
            ));
        };
        // 3. Let ctor be ? SpeciesConstructor(promise, %Promise%). 4. Assert: IsConstructor(ctor)
        // is true.
        let default = self.current_intrinsic(Intrinsic::Promise)?;
        let ctor = self.species_constructor(promise, default)?;
        let mark = self.keep(Value::Object(ctor));
        let answer =
            self.finally_handlers(ctor, on_finally)
                .and_then(|(then_finally, catch_finally)| {
                    self.keep(then_finally);
                    self.keep(catch_finally);
                    // 7. Return ? Invoke(promise, "then", « thenFinally, catchFinally »).
                    let key = self.name_key("then")?;
                    self.invoke(this, key, &[then_finally, catch_finally])
                });
        self.release(mark);
        answer
    }

    /// Steps 5 and 6 of `finally`: onFinally twice when it is not callable, else thenFinally and
    /// catchFinally, CreateBuiltinFunction(closure, 1, "", « ») of the current realm, whose
    /// slots hold onFinally and ctor.
    fn finally_handlers(
        &mut self,
        ctor: ObjectRef,
        on_finally: Value,
    ) -> Completion<(Value, Value)> {
        // 5. If IsCallable(onFinally) is false, then a. Let thenFinally be onFinally. b. Let
        // catchFinally be onFinally.
        if !self.is_callable(on_finally)? {
            return Ok((on_finally, on_finally));
        }
        // 6. Else, a and b: thenFinally; c and d: catchFinally.
        let realm = self.current_realm()?;
        let slots = || PromiseSlots::Finally {
            on_finally,
            constructor: ctor,
        };
        let then_finally = self.create_promise_closure(
            BuiltinId::Promise(PromiseBuiltin::ThenFinally),
            slots(),
            1,
            realm,
        )?;
        let mark = self.keep(Value::Object(then_finally));
        let catch_finally = self.create_promise_closure(
            BuiltinId::Promise(PromiseBuiltin::CatchFinally),
            slots(),
            1,
            realm,
        );
        self.release(mark);
        Ok((Value::Object(then_finally), Value::Object(catch_finally?)))
    }

    /// thenFinallyClosure ( value ) or catchFinallyClosure ( reason ) of `finally`
    /// (`#sec-promise.prototype.finally`, steps 6.a and 6.c), the closure `function`.
    fn finally_closure(
        &mut self,
        id: PromiseBuiltin,
        function: ObjectRef,
        argument: Value,
    ) -> Completion<Value> {
        let (on_finally, ctor) = match self.promise_closure_slots(function)? {
            PromiseSlots::Finally {
                on_finally,
                constructor,
            } => (*on_finally, *constructor),
            PromiseSlots::Element { .. } | PromiseSlots::Thunk { .. } => {
                return Err(self.fault());
            }
        };
        // i. Let result be ? Call(onFinally, undefined).
        let result = self.call_value(on_finally, Value::Undefined, &[])?;
        // ii. Let p be ? PromiseResolve(ctor, result).
        let mark = self.keep(result);
        let p = self.promise_resolve(ctor, result);
        self.release(mark);
        let p = p?;
        let mark = self.keep(p);
        // iii and iv: valueThunk, CreateBuiltinFunction(returnValue, 0, "", « »), or thrower,
        // CreateBuiltinFunction(throwReason, 0, "", « »), capturing the argument.
        let thunk_id = if id == PromiseBuiltin::ThenFinally {
            PromiseBuiltin::ValueThunk
        } else {
            PromiseBuiltin::Thrower
        };
        let answer = self
            .current_realm()
            .and_then(|realm| {
                self.create_promise_closure(
                    BuiltinId::Promise(thunk_id),
                    PromiseSlots::Thunk { value: argument },
                    0,
                    realm,
                )
            })
            .and_then(|thunk| {
                self.keep(Value::Object(thunk));
                // v. Return ? Invoke(p, "then", « valueThunk ») or « thrower ».
                let key = self.name_key("then")?;
                self.invoke(p, key, &[Value::Object(thunk)])
            });
        self.release(mark);
        answer
    }

    /// `Promise ( executor )` (`#sec-promise-executor`).
    fn promise_constructor(
        &mut self,
        executor: Value,
        new_target: Option<ObjectRef>,
    ) -> Completion<Value> {
        // 1. If NewTarget is undefined, throw a TypeError exception.
        let Some(new_target) = new_target else {
            return Err(self.throw_error(ErrorKind::TypeError, "Promise must be called with new"));
        };
        // 2. If IsCallable(executor) is false, throw a TypeError exception.
        if !self.is_callable(executor)? {
            return Err(self.throw_error(
                ErrorKind::TypeError,
                "the executor of a Promise is not a function",
            ));
        }
        // 3. Let promise be ? OrdinaryCreateFromConstructor(NewTarget, "%Promise.prototype%", «
        // [[PromiseState]], [[PromiseResult]], [[PromiseFulfillReactions]],
        // [[PromiseRejectReactions]], [[PromiseIsHandled]] »). 4 to 8: pending, its result
        // empty, with no reaction, not handled.
        let prototype =
            self.get_prototype_from_constructor(new_target, Intrinsic::PromisePrototype)?;
        let promise = self.new_promise_object(prototype)?;
        let mark = self.keep(Value::Object(promise));
        let ran = self.run_executor(promise, executor);
        self.release(mark);
        ran?;
        // 12. Return promise.
        Ok(Value::Object(promise))
    }

    /// Steps 9 to 11 of `Promise ( executor )`: the resolving functions of `promise`, the call
    /// of `executor` with them, and the reject function called with what it threw. The draft's
    /// "!" of step 11.a holds for a script's throw; a termination or a fault of the agent ends
    /// the call as it ends every step.
    fn run_executor(&mut self, promise: ObjectRef, executor: Value) -> Completion<()> {
        // 9. Let resolvingFuncs be CreateResolvingFunctions(promise).
        let (resolve, reject) = self.create_resolving_functions(promise)?;
        let mark = self.keep(Value::Object(resolve));
        self.keep(Value::Object(reject));
        // 10. Let completion be Completion(Call(executor, undefined, « resolvingFuncs.[[Resolve]],
        // resolvingFuncs.[[Reject]] »)).
        let completion = self.call_value(
            executor,
            Value::Undefined,
            &[Value::Object(resolve), Value::Object(reject)],
        );
        // 11. If completion is an abrupt completion, then a. Perform !
        // Call(resolvingFuncs.[[Reject]], undefined, « completion.[[Value]] »).
        let ran = match completion {
            Ok(_) => Ok(()),
            Err(Abrupt::Throw(reason)) => self
                .call_settling_function(Value::Object(reject), reason)
                .map(|_| ()),
            Err(abrupt) => Err(abrupt),
        };
        self.release(mark);
        ran
    }

    /// `Promise.reject ( reason )` (`#sec-promise.reject`).
    fn promise_reject(&mut self, this: Value, reason: Value) -> Completion<Value> {
        // 1. Let ctor be the this value. 2. Let promiseCapability be ?
        // NewPromiseCapability(ctor).
        let capability = self.new_promise_capability(this)?;
        let mark = self.keep_capability(&capability);
        // 3. Perform ? Call(promiseCapability.[[Reject]], undefined, « reason »).
        let called = self.call_value(capability.reject, Value::Undefined, &[reason]);
        self.release(mark);
        called?;
        // 4. Return promiseCapability.[[Promise]].
        Ok(Value::Object(capability.promise))
    }

    /// `Promise.resolve ( resolution )` (`#sec-promise.resolve`).
    fn promise_resolve_static(&mut self, this: Value, resolution: Value) -> Completion<Value> {
        // 1. Let ctor be the this value. 2. If ctor is not an Object, throw a TypeError
        // exception.
        let Value::Object(ctor) = this else {
            return Err(self.throw_error(
                ErrorKind::TypeError,
                "Promise.resolve called on a value that is not an object",
            ));
        };
        // 3. Return ? PromiseResolve(ctor, resolution).
        self.promise_resolve(ctor, resolution)
    }

    /// `Promise.prototype.catch ( onRejected )` (`#sec-promise.prototype.catch`).
    fn promise_catch(&mut self, this: Value, on_rejected: Value) -> Completion<Value> {
        // 1. Let promise be the this value. 2. Return ? Invoke(promise, "then", « undefined,
        // onRejected »).
        let key = self.name_key("then")?;
        self.invoke(this, key, &[Value::Undefined, on_rejected])
    }

    /// `Promise.prototype.then ( onFulfilled, onRejected )` (`#sec-promise.prototype.then`).
    fn promise_then(
        &mut self,
        this: Value,
        on_fulfilled: Value,
        on_rejected: Value,
    ) -> Completion<Value> {
        // 1. Let promise be the this value. 2. If IsPromise(promise) is false, throw a TypeError
        // exception.
        let Value::Object(promise) = this else {
            return Err(self.not_a_promise());
        };
        if !self.is_promise(this)? {
            return Err(self.not_a_promise());
        }
        // 3. Let ctor be ? SpeciesConstructor(promise, %Promise%).
        let default = self.current_intrinsic(Intrinsic::Promise)?;
        let ctor = self.species_constructor(promise, default)?;
        // 4. Let resultCapability be ? NewPromiseCapability(ctor).
        let capability = self.new_promise_capability(Value::Object(ctor))?;
        let mark = self.keep_capability(&capability);
        // 5. Return PerformPromiseThen(promise, onFulfilled, onRejected, resultCapability).
        let answer =
            self.perform_promise_then(promise, on_fulfilled, on_rejected, Some(capability));
        self.release(mark);
        answer
    }

    /// The TypeError of `then` called on a value that is not a promise.
    fn not_a_promise(&mut self) -> Abrupt {
        self.throw_error(
            ErrorKind::TypeError,
            "Promise.prototype.then called on a value that is not a promise",
        )
    }

    /// [[Promise]] and [[AlreadyResolved]] of the resolving function `function`.
    fn resolving_slots(
        &mut self,
        function: ObjectRef,
    ) -> Completion<(ObjectRef, Rc<core::cell::Cell<bool>>)> {
        match self.closure_slots(function)? {
            Slots::Resolving {
                promise,
                already_resolved,
            } => Ok((*promise, Rc::clone(already_resolved))),
            Slots::Executor { .. } | Slots::Await { .. } | Slots::AsyncFromSync(_) => {
                Err(self.fault())
            }
        }
    }

    /// resolveSteps of CreateResolvingFunctions (`#sec-createresolvingfunctions`, step 2) called
    /// with `resolution`; promiseOrEmpty is the function's [[Promise]] while its
    /// [[AlreadyResolved]] is false (the module's documentation).
    pub(crate) fn resolve_function(
        &mut self,
        function: ObjectRef,
        resolution: Value,
    ) -> Completion<Value> {
        let (promise, already_resolved) = self.resolving_slots(function)?;
        // a. If promiseOrEmpty.[[Value]] is empty, return undefined.
        if already_resolved.get() {
            return Ok(Value::Undefined);
        }
        // b. Let promise be promiseOrEmpty.[[Value]]. c. Set promiseOrEmpty.[[Value]] to empty.
        already_resolved.set(true);
        // d. If SameValue(resolution, promise) is true, then
        if self.same_value(resolution, Value::Object(promise))? {
            // i. Let selfResolutionError be a newly created TypeError object.
            let error = match self.throw_error(
                ErrorKind::TypeError,
                "a promise cannot be resolved with itself",
            ) {
                Abrupt::Throw(error) => error,
                abrupt => return Err(abrupt),
            };
            // ii. Perform RejectPromise(promise, selfResolutionError). iii. Return undefined.
            self.reject_promise(promise, error)?;
            return Ok(Value::Undefined);
        }
        // e. If resolution is not an Object, then perform FulfillPromise(promise, resolution) and
        // return undefined.
        let Value::Object(object) = resolution else {
            self.fulfill_promise(promise, resolution)?;
            return Ok(Value::Undefined);
        };
        // f. Let then be Completion(Get(resolution, "then")).
        let key = self.name_key("then")?;
        let then = match self.get(object, key) {
            Ok(then) => then,
            // g. If then is an abrupt completion, then perform RejectPromise(promise,
            // then.[[Value]]) and return undefined.
            Err(Abrupt::Throw(reason)) => {
                self.reject_promise(promise, reason)?;
                return Ok(Value::Undefined);
            }
            Err(abrupt) => return Err(abrupt),
        };
        // h. Let thenAction be then.[[Value]]. i. If IsCallable(thenAction) is false, then
        // perform FulfillPromise(promise, resolution) and return undefined.
        let Value::Object(then_action) = then else {
            self.fulfill_promise(promise, resolution)?;
            return Ok(Value::Undefined);
        };
        if !self.is_callable(then)? {
            self.fulfill_promise(promise, resolution)?;
            return Ok(Value::Undefined);
        }
        // j. Let thenJobCallback be HostMakeJobCallback(thenAction).
        let then_job_callback = self.host_make_job_callback(then_action)?;
        // k. Let job be NewPromiseResolveThenableJob(promise, resolution, thenJobCallback).
        let (job, realm) =
            self.new_promise_resolve_thenable_job(promise, resolution, then_job_callback)?;
        // l. Perform HostEnqueuePromiseJob(job.[[Job]], job.[[Realm]]).
        self.enqueue_promise_job(job, realm)?;
        // m. Return undefined.
        Ok(Value::Undefined)
    }

    /// rejectSteps of CreateResolvingFunctions (`#sec-createresolvingfunctions`, step 4) called
    /// with `reason`.
    pub(crate) fn reject_function(
        &mut self,
        function: ObjectRef,
        reason: Value,
    ) -> Completion<Value> {
        let (promise, already_resolved) = self.resolving_slots(function)?;
        // a. If promiseOrEmpty.[[Value]] is empty, return undefined.
        if already_resolved.get() {
            return Ok(Value::Undefined);
        }
        // b. Let promise be promiseOrEmpty.[[Value]]. c. Set promiseOrEmpty.[[Value]] to empty.
        already_resolved.set(true);
        // d. Perform RejectPromise(promise, reason). e. Return undefined.
        self.reject_promise(promise, reason)?;
        Ok(Value::Undefined)
    }

    /// executorClosure of NewPromiseCapability (`#sec-newpromisecapability`, step 4) called with
    /// `resolve` and `reject`; resolvingFuncs is the function's slots.
    fn capability_executor(
        &mut self,
        function: ObjectRef,
        resolve: Value,
        reject: Value,
    ) -> Completion<Value> {
        let (resolve_slot, reject_slot) = match self.closure_slots(function)? {
            Slots::Executor { resolve, reject } => (*resolve, *reject),
            Slots::Resolving { .. } | Slots::Await { .. } | Slots::AsyncFromSync(_) => {
                return Err(self.fault());
            }
        };
        // a. If resolvingFuncs.[[Resolve]] is not undefined, throw a TypeError exception.
        // b. If resolvingFuncs.[[Reject]] is not undefined, throw a TypeError exception.
        if resolve_slot != Value::Undefined || reject_slot != Value::Undefined {
            return Err(self.throw_error(
                ErrorKind::TypeError,
                "the executor of a promise capability was called twice",
            ));
        }
        // c. Set resolvingFuncs.[[Resolve]] to resolve. d. Set resolvingFuncs.[[Reject]] to
        // reject.
        match self.closure_slots_mut(function)? {
            Slots::Executor {
                resolve: resolve_slot,
                reject: reject_slot,
            } => {
                *resolve_slot = resolve;
                *reject_slot = reject;
            }
            Slots::Resolving { .. } | Slots::Await { .. } | Slots::AsyncFromSync(_) => {
                return Err(self.fault());
            }
        }
        // e. Return NormalCompletion(undefined).
        Ok(Value::Undefined)
    }

    /// Call `function` with undefined and « argument », for a job or the constructor that settles
    /// a promise through a resolving function: one of the resolving functions of this module runs
    /// its steps at once, after the step of a call, with its [[Realm]] as the current realm and
    /// no activation of its own (the module's documentation); any other function is called with
    /// Call (`#sec-call`).
    pub(crate) fn call_settling_function(
        &mut self,
        function: Value,
        argument: Value,
    ) -> Completion<Value> {
        let found = match function {
            Value::Object(object) => match &self.engine_object(object)?.kind {
                ObjectKind::BuiltinFunction(BuiltinFunction {
                    realm,
                    behaviour:
                        Behaviour::Closure {
                            id: BuiltinId::Promise(id),
                            ..
                        },
                    ..
                }) if matches!(
                    id,
                    PromiseBuiltin::ResolveFunction | PromiseBuiltin::RejectFunction
                ) =>
                {
                    Some((object, *id, *realm))
                }
                _ => None,
            },
            _ => None,
        };
        let Some((object, id, realm)) = found else {
            return self.call_value(function, Value::Undefined, &[argument]);
        };
        self.running()?;
        self.charge_steps(1)?;
        let saved = self.realm_slot();
        self.set_realm_slot(realm);
        let answer = if id == PromiseBuiltin::ResolveFunction {
            self.resolve_function(object, argument)
        } else {
            self.reject_function(object, argument)
        };
        self.set_realm_slot(saved);
        answer
    }
}
