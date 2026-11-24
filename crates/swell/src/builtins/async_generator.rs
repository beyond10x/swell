//! The async generator objects of slice 10 (`#sec-asyncgenerator-objects`), written by
//! `story:async-generators-and-their-request-queue`: `%AsyncGeneratorFunction.prototype%`
//! (`#sec-properties-of-asyncgeneratorfunction-prototype`) with its `prototype`
//! (`#sec-asyncgeneratorfunction-prototype-prototype`) and `[%Symbol.toStringTag%]`
//! (`#sec-asyncgeneratorfunction-prototype-tostringtag`); `%AsyncGeneratorPrototype%`
//! (`#sec-properties-of-asyncgenerator-prototype`) with its `constructor`
//! (`#sec-asyncgenerator-prototype-constructor`), `next` (`#sec-asyncgenerator-prototype-next`),
//! `return` (`#sec-asyncgenerator-prototype-return`), `throw`
//! (`#sec-asyncgenerator-prototype-throw`) and `[%Symbol.toStringTag%]`
//! (`#sec-asyncgenerator-prototype-tostringtag`); and the operations of the request queue:
//! AsyncGeneratorValidate (`#sec-asyncgeneratorvalidate`), AsyncGeneratorEnqueue
//! (`#sec-asyncgeneratorenqueue`), AsyncGeneratorCompleteStep
//! (`#sec-asyncgeneratorcompletestep`), AsyncGeneratorAwaitReturn
//! (`#sec-asyncgeneratorawaitreturn`) with its two functions, and AsyncGeneratorDrainQueue
//! (`#sec-asyncgeneratordrainqueue`), on the rules of `docs/design/engine.md`, decision 3, "Async
//! generators on suspended frames", items 1 to 5.
//!
//! - The capability of a call (step 2 of each method, NewPromiseCapability(`%Promise%`)) is made
//!   as `AsyncStart` makes it: a promise of `%Promise.prototype%` of the current realm and its
//!   resolving functions, with no call of the constructor, so the call of `next`, `return` or
//!   `throw` is the only call a depth bound counts (item 4). Each settling of a request runs the
//!   steps of its resolving function at once, with no step of its own: it is bounded work within
//!   the call, the `yield`, the end of the body or the drain that settles it (item 3).
//! - A resumption (AsyncGeneratorResume) is checked before the request is queued and before the
//!   state changes (item 4): [`Agent::resume_frame`] makes its checks, then has the generator
//!   queue the request, set its state executing and give its frame up; a `RangeError` of the
//!   checks rejects the promise of the call, with nothing queued.
//! - The queue grows by doubling, charged before it grows and recorded in the object's `Grown`
//!   (item 2); a growth the budget refuses ends the run in `Terminated(MemoryBudget)` with
//!   nothing queued.
//! - The drain spends one step for each request it settles, before it takes that request off
//!   (item 3); the wait of a `return` it meets is an `await` with two functions with slots
//!   ([`Slots::Await`], the generator and the number of its wait), which do nothing when called
//!   at another time (item 1).
//!
//! The iterator results are made in the current realm: AsyncGeneratorCompleteStep's realm of the
//! `yield` is the realm of the context that resumed the generator, which is always the function
//! of an `await` made by the generator's own code (every `yield` awaits its operand first).
//!
//! `%AsyncGeneratorFunction%` (`#sec-asyncgeneratorfunction-constructor`,
//! `#sec-properties-of-asyncgeneratorfunction`) with its `prototype`
//! (`#sec-asyncgeneratorfunction-prototype`), and the `constructor` of
//! `%AsyncGeneratorFunction.prototype%` (`#sec-asyncgeneratorfunction-prototype-constructor`),
//! written by `story:async-generator-function-constructor`: called or constructed,
//! `AsyncGeneratorFunction ( ...paramArgs, bodyArg )` (`#sec-asyncgeneratorfunction`) is
//! CreateDynamicFunction of kind `async-generator` (`dynamic_code.rs`). It is no property of the
//! global object: a script reaches it as the `constructor` of the [[Prototype]] of an async
//! generator function.

use swell_compiler::bytecode::{TAG_NORMAL, TAG_RETURN, TAG_THROW};

use crate::dynamic_code::DynamicFunctionKind;
use crate::host::agent::Agent;
use crate::host::outcome::{Abrupt, Completion, Termination};
use crate::host::{ErrorKind, Host};
use crate::interp::suspend::{Resumption, SuspendedFrame};
use crate::object::ObjectKind;
use crate::object::async_generator::{AsyncGenerator, AsyncGeneratorState, Request, request_bytes};
use crate::object::builtin_function::{Behaviour, BuiltinId, Slots};
use crate::object::ordinary::OrdinaryData;
use crate::object::property::Property;
use crate::realm::intrinsics::{Intrinsic, Making};
use crate::value::{ObjectRef, Value};

/// The builtins of this module, by their steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AsyncGeneratorBuiltin {
    /// `%AsyncGeneratorPrototype%.next ( value )` (27.6.1.2).
    Next,
    /// `%AsyncGeneratorPrototype%.return ( value )` (27.6.1.3).
    Return,
    /// `%AsyncGeneratorPrototype%.throw ( exception )` (27.6.1.4).
    Throw,
    /// fulfilledClosure of AsyncGeneratorAwaitReturn (27.6.3.9), with the slots
    /// [`Slots::Await`].
    AwaitReturnFulfilled,
    /// rejectedClosure of AsyncGeneratorAwaitReturn (27.6.3.9), with the slots
    /// [`Slots::Await`].
    AwaitReturnRejected,
}

/// `%AsyncGeneratorFunction.prototype%` and `%AsyncGeneratorPrototype%` of a realm.
#[derive(Clone, Copy, Debug)]
pub(crate) struct AsyncGeneratorPrototypes {
    pub(crate) function_prototype: ObjectRef,
    pub(crate) prototype: ObjectRef,
}

/// The capability of a request: a promise of `%Promise%` and its resolving functions.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Capability {
    pub(crate) promise: ObjectRef,
    pub(crate) resolve: ObjectRef,
    pub(crate) reject: ObjectRef,
}

impl<H: Host> Agent<H> {
    /// `%AsyncGeneratorFunction.prototype%`, an ordinary object whose [[Prototype]] is
    /// `%Function.prototype%`, with `prototype` holding `%AsyncGeneratorPrototype%` and
    /// `[%Symbol.toStringTag%]` "AsyncGeneratorFunction", each { [[Writable]]: false,
    /// [[Enumerable]]: false, [[Configurable]]: true }; and `%AsyncGeneratorPrototype%`, an
    /// ordinary object whose [[Prototype]] is `async_iterator`, `%AsyncIteratorPrototype%`, with
    /// `constructor` holding `%AsyncGeneratorFunction.prototype%` (the same attributes), the
    /// methods `next`, `return` and `throw` of "length" 1 (clause 18: { [[Writable]]: true,
    /// [[Enumerable]]: false, [[Configurable]]: true }) and `[%Symbol.toStringTag%]`
    /// "AsyncGenerator".
    pub(crate) fn create_async_generator_prototypes(
        &mut self,
        making: Making,
        async_iterator: ObjectRef,
    ) -> Completion<AsyncGeneratorPrototypes> {
        let function_prototype =
            self.ordinary_object_create(Some(making.function_prototype), OrdinaryData::Plain)?;
        let prototype = self.ordinary_object_create(Some(async_iterator), OrdinaryData::Plain)?;
        let prototype_key = self.name_key("prototype")?;
        self.add_property(
            function_prototype,
            prototype_key,
            Property::data(Value::Object(prototype), false, false, true),
        )?;
        self.define_to_string_tag(making, function_prototype, "AsyncGeneratorFunction")?;
        let constructor_key = self.name_key("constructor")?;
        self.add_property(
            prototype,
            constructor_key,
            Property::data(Value::Object(function_prototype), false, false, true),
        )?;
        for (name, id) in [
            ("next", AsyncGeneratorBuiltin::Next),
            ("return", AsyncGeneratorBuiltin::Return),
            ("throw", AsyncGeneratorBuiltin::Throw),
        ] {
            self.define_builtin_method(making, prototype, name, 1, BuiltinId::AsyncGenerator(id))?;
        }
        self.define_to_string_tag(making, prototype, "AsyncGenerator")?;
        Ok(AsyncGeneratorPrototypes {
            function_prototype,
            prototype,
        })
    }

    /// `%AsyncGeneratorFunction%`, a constructor whose [[Prototype]] is `function`, `%Function%`,
    /// with "length" 1, "name" "AsyncGeneratorFunction" and `prototype` holding
    /// `function_prototype`, `%AsyncGeneratorFunction.prototype%`, { [[Writable]]: false,
    /// [[Enumerable]]: false, [[Configurable]]: false }; and the `constructor` of
    /// `%AsyncGeneratorFunction.prototype%`, holding it, { [[Writable]]: false, [[Enumerable]]:
    /// false, [[Configurable]]: true }.
    pub(crate) fn create_async_generator_function_constructor(
        &mut self,
        making: Making,
        function: ObjectRef,
        function_prototype: ObjectRef,
    ) -> Completion<ObjectRef> {
        let constructor = self.create_builtin_function(
            Behaviour::Builtin(BuiltinId::AsyncGeneratorFunction),
            1,
            "AsyncGeneratorFunction",
            making.realm,
            Some(function),
            true,
        )?;
        let prototype_key = self.name_key("prototype")?;
        self.add_property(
            constructor,
            prototype_key,
            Property::data(Value::Object(function_prototype), false, false, false),
        )?;
        let constructor_key = self.name_key("constructor")?;
        self.add_property(
            function_prototype,
            constructor_key,
            Property::data(Value::Object(constructor), false, false, true),
        )?;
        Ok(constructor)
    }

    /// `AsyncGeneratorFunction ( ...paramArgs, bodyArg )` (`#sec-asyncgeneratorfunction`),
    /// called or constructed: the active function `function`, `bodyArg` the last argument or the
    /// empty String, then CreateDynamicFunction(function, NewTarget, async-generator, paramArgs,
    /// bodyArg).
    pub(crate) fn async_generator_function(
        &mut self,
        function: ObjectRef,
        args: &[Value],
        new_target: Option<ObjectRef>,
    ) -> Completion<Value> {
        // 1. Let activeFunc be the active function object. 2. If bodyArg is not present, set
        // bodyArg to the empty String.
        let (params, body) = match args.split_last() {
            Some((body, params)) => (params, Some(*body)),
            None => (args, None),
        };
        // 3. Return ? CreateDynamicFunction(activeFunc, NewTarget, async-generator, paramArgs,
        // bodyArg).
        let new_target = new_target.unwrap_or(function);
        let made = self.create_dynamic_function(
            new_target,
            DynamicFunctionKind::AsyncGenerator,
            params,
            body,
        )?;
        Ok(Value::Object(made))
    }

    /// The steps of the builtins of this module, called with `this` and the first argument.
    pub(crate) fn run_async_generator_builtin(
        &mut self,
        id: AsyncGeneratorBuiltin,
        function: ObjectRef,
        this: Value,
        args: &[Value],
    ) -> Completion<Value> {
        let value = args.first().copied().unwrap_or(Value::Undefined);
        match id {
            AsyncGeneratorBuiltin::Next => self.async_generator_method(this, value, TAG_NORMAL),
            AsyncGeneratorBuiltin::Return => self.async_generator_method(this, value, TAG_RETURN),
            AsyncGeneratorBuiltin::Throw => self.async_generator_method(this, value, TAG_THROW),
            AsyncGeneratorBuiltin::AwaitReturnFulfilled => {
                self.await_return_function(function, value, TAG_NORMAL)
            }
            AsyncGeneratorBuiltin::AwaitReturnRejected => {
                self.await_return_function(function, value, TAG_THROW)
            }
        }
    }

    /// The record of the async generator `generator`, to change.
    pub(crate) fn async_generator_mut(
        &mut self,
        generator: ObjectRef,
    ) -> Completion<&mut AsyncGenerator> {
        if !matches!(
            self.engine_object(generator)?.kind,
            ObjectKind::AsyncGenerator(_)
        ) {
            return Err(self.fault());
        }
        match &mut self.engine_object_mut(generator)?.kind {
            ObjectKind::AsyncGenerator(record) => Ok(record),
            _ => Err(Abrupt::Terminated(Termination::EngineFault)),
        }
    }

    /// Whether `object` is an async generator object.
    pub(crate) fn is_async_generator(&mut self, object: ObjectRef) -> Completion<bool> {
        Ok(matches!(
            self.engine_object(object)?.kind,
            ObjectKind::AsyncGenerator(_)
        ))
    }

    /// Set [[AsyncGeneratorState]] of `generator` to `state`.
    pub(crate) fn set_async_generator_state(
        &mut self,
        generator: ObjectRef,
        state: AsyncGeneratorState,
    ) -> Completion<()> {
        self.async_generator_mut(generator)?.state = state;
        Ok(())
    }

    /// NewPromiseCapability(%Promise%) (`#sec-newpromisecapability`) as `AsyncStart` makes it
    /// (the module's documentation): a new pending promise whose [[Prototype]] is
    /// `%Promise.prototype%` of the current realm, and its resolving functions.
    pub(crate) fn async_generator_capability(&mut self) -> Completion<Capability> {
        let prototype = self.current_intrinsic(Intrinsic::PromisePrototype)?;
        let promise = self.new_promise_object(prototype)?;
        let mark = self.keep(Value::Object(promise));
        let functions = self.create_resolving_functions(promise);
        self.release(mark);
        let (resolve, reject) = functions?;
        Ok(Capability {
            promise,
            resolve,
            reject,
        })
    }

    /// `next` (`tag` `TAG_NORMAL`), `return` (`TAG_RETURN`) and `throw` (`TAG_THROW`) with the
    /// this value `this` and the argument `value`: the capability kept while the steps run, its
    /// promise the answer.
    fn async_generator_method(&mut self, this: Value, value: Value, tag: u32) -> Completion<Value> {
        // 1. Let generator be the this value. 2. Let promiseCapability be !
        // NewPromiseCapability(%Promise%).
        let capability = self.async_generator_capability()?;
        let mark = self.keep(Value::Object(capability.promise));
        self.keep(Value::Object(capability.resolve));
        self.keep(Value::Object(capability.reject));
        self.keep(value);
        let done = self
            .async_generator_steps(this, value, tag, capability)
            .and_then(|resumption| match resumption {
                Some(resumption) => self.run_resumed(resumption).map(|_| ()),
                None => Ok(()),
            });
        self.release(mark);
        done?;
        Ok(Value::Object(capability.promise))
    }

    /// The steps of `next`, `return` and `throw` after step 2, with the capability's values kept
    /// by the caller. A resumption's frame is pushed and not run: the builtin runs it
    /// ([`Agent::run_resumed`]), and the instruction of `yield*` in an async generator
    /// (`interp/yield_star.rs`) goes on in its own run loop, in which it runs above the frame
    /// that resumed it.
    pub(crate) fn async_generator_steps(
        &mut self,
        this: Value,
        value: Value,
        tag: u32,
        capability: Capability,
    ) -> Completion<Option<Resumption>> {
        // 3. Let result be Completion(AsyncGeneratorValidate(generator, empty)). 4.
        // IfAbruptRejectPromise(result, promiseCapability).
        let Some((generator, mut state)) = self.async_generator_validate(this)? else {
            let error = self.throw_error(ErrorKind::TypeError, "not an async generator object");
            return self
                .reject_request_capability(capability, error)
                .map(|()| None);
        };
        // 5. Let state be generator.[[AsyncGeneratorState]].
        match tag {
            TAG_NORMAL => {
                // 6. If state is completed, then a. Let iteratorResult be
                // CreateIteratorResultObject(undefined, true). b. Perform !
                // Call(promiseCapability.[[Resolve]], undefined, « iteratorResult »). c. Return
                // promiseCapability.[[Promise]].
                if state == AsyncGeneratorState::Completed {
                    let result = self.create_iter_result_object(Value::Undefined, true)?;
                    let mark = self.keep(Value::Object(result));
                    let settled = self.resolve_function(capability.resolve, Value::Object(result));
                    self.release(mark);
                    return settled.map(|_| None);
                }
            }
            TAG_THROW => {
                // 6. If state is suspended-start, then a. Set generator.[[AsyncGeneratorState]] to
                // completed. b. Set state to completed.
                if state == AsyncGeneratorState::SuspendedStart {
                    self.complete_unstarted(generator, AsyncGeneratorState::Completed)?;
                    state = AsyncGeneratorState::Completed;
                }
                // 7. If state is completed, then a. Perform ! Call(promiseCapability.[[Reject]],
                // undefined, « exception »). b. Return promiseCapability.[[Promise]].
                if state == AsyncGeneratorState::Completed {
                    return self.reject_function(capability.reject, value).map(|_| None);
                }
            }
            _ => {}
        }
        let request = Request {
            tag,
            value,
            promise: capability.promise,
            resolve: capability.resolve,
            reject: capability.reject,
        };
        let resumes = match tag {
            TAG_NORMAL => matches!(
                state,
                AsyncGeneratorState::SuspendedStart | AsyncGeneratorState::SuspendedYield
            ),
            _ => state == AsyncGeneratorState::SuspendedYield,
        };
        if resumes {
            // AsyncGeneratorEnqueue, then AsyncGeneratorResume(generator, completion): the checks
            // of the resumption first (item 4), then the request queued, the state executing and
            // the frame taken, in the move of `resume_frame`.
            let resumption = match self.resume_frame(
                |agent| agent.async_generator_take_frame(generator, request),
                value,
                tag,
            ) {
                Ok(resumption) => resumption,
                Err(abrupt) => {
                    return self
                        .reject_request_capability(capability, abrupt)
                        .map(|()| None);
                }
            };
            return Ok(Some(resumption));
        }
        // Perform AsyncGeneratorEnqueue(generator, completion, promiseCapability).
        self.async_generator_enqueue(generator, request)?;
        // `return`: If state is either suspended-start or completed, then i. Set
        // generator.[[AsyncGeneratorState]] to draining-queue. ii. Perform
        // AsyncGeneratorAwaitReturn(generator).
        if tag == TAG_RETURN
            && matches!(
                state,
                AsyncGeneratorState::SuspendedStart | AsyncGeneratorState::Completed
            )
        {
            if state == AsyncGeneratorState::SuspendedStart {
                self.complete_unstarted(generator, AsyncGeneratorState::DrainingQueue)?;
            } else {
                self.set_async_generator_state(generator, AsyncGeneratorState::DrainingQueue)?;
            }
            if self.async_generator_await_return(generator, false)? {
                self.async_generator_drain_queue(generator)?;
            }
        }
        // Else, Assert: state is either executing or draining-queue.
        Ok(None)
    }

    /// IfAbruptRejectPromise of a request's capability (`#sec-ifabruptrejectpromise`): a throw
    /// rejects its promise; a termination ends the call.
    fn reject_request_capability(
        &mut self,
        capability: Capability,
        abrupt: Abrupt,
    ) -> Completion<()> {
        let Abrupt::Throw(reason) = abrupt else {
            return Err(abrupt);
        };
        let mark = self.keep(reason);
        let settled = self.reject_function(capability.reject, reason);
        self.release(mark);
        settled.map(|_| ())
    }

    /// AsyncGeneratorValidate ( generator, empty ) (`#sec-asyncgeneratorvalidate`): the
    /// generator and its [[AsyncGeneratorState]], or `None` for a value that is no async
    /// generator object (step 1 to 3 throw a `TypeError` there, which the caller makes).
    fn async_generator_validate(
        &mut self,
        generator: Value,
    ) -> Completion<Option<(ObjectRef, AsyncGeneratorState)>> {
        let Value::Object(object) = generator else {
            return Ok(None);
        };
        Ok(match &self.engine_object(object)?.kind {
            ObjectKind::AsyncGenerator(record) => Some((object, record.state)),
            _ => None,
        })
    }

    /// An async generator suspended at its start that will never run: its state `state`
    /// (completed for `throw`, draining-queue for `return`), and its frame dropped and its
    /// charge credited at once (item 3 of "Suspended frames").
    fn complete_unstarted(
        &mut self,
        generator: ObjectRef,
        state: AsyncGeneratorState,
    ) -> Completion<()> {
        let frame = {
            let record = self.async_generator_mut(generator)?;
            record.state = state;
            record.frame.take()
        };
        if let Some(frame) = frame {
            self.release_suspended_frame(frame);
        }
        Ok(())
    }

    /// The move of a resumption by `next`, `return` or `throw` after its checks: `request`
    /// queued (AsyncGeneratorEnqueue), the state executing (AsyncGeneratorResume, step 3) and
    /// the frame taken out for `Agent::resume_frame` to push.
    fn async_generator_take_frame(
        &mut self,
        generator: ObjectRef,
        request: Request,
    ) -> Completion<SuspendedFrame> {
        self.async_generator_enqueue(generator, request)?;
        let record = self.async_generator_mut(generator)?;
        record.state = AsyncGeneratorState::Executing;
        match record.frame.take() {
            Some(frame) => Ok(frame),
            None => Err(self.fault()),
        }
    }

    /// AsyncGeneratorEnqueue ( generator, completion, promiseCapability )
    /// (`#sec-asyncgeneratorenqueue`): `request` appended to the queue. A full queue grows first
    /// to twice its room, at least 4, charged before it grows and recorded in the object's
    /// `Grown` (item 2); over the budget the run ends in `Terminated(MemoryBudget)` with nothing
    /// queued.
    fn async_generator_enqueue(
        &mut self,
        generator: ObjectRef,
        request: Request,
    ) -> Completion<()> {
        let (length, capacity) = {
            let record = self.async_generator_mut(generator)?;
            (record.queue.len(), record.queue.capacity())
        };
        if length >= capacity {
            let wanted = capacity.saturating_mul(2).max(4);
            let more = u64::try_from(wanted - capacity).unwrap_or(u64::MAX);
            let bytes = more.saturating_mul(request_bytes());
            if self.heap.accounting.charge(bytes).is_err() {
                return Err(self.terminate(Termination::MemoryBudget));
            }
            let reserved = self
                .async_generator_mut(generator)?
                .queue
                .try_reserve_exact(wanted.saturating_sub(length));
            if reserved.is_err() {
                self.heap.accounting.credit(bytes);
                return Err(self.terminate(Termination::MemoryBudget));
            }
            self.record_grown(generator, bytes)?;
        }
        self.async_generator_mut(generator)?
            .queue
            .push_back(request);
        Ok(())
    }

    /// AsyncGeneratorCompleteStep ( generator, completion, done ) (`#sec-asyncgeneratorcompletestep`)
    /// with the completion of `tag` and `value` (`TAG_THROW` for a throw completion, any other
    /// tag for a normal one): the request at the head taken off, then its promise rejected with
    /// `value`, or resolved with CreateIteratorResultObject(value, done) made in the current
    /// realm; the resolving function's steps run at once. Its values are kept while it settles.
    pub(crate) fn async_generator_complete_step(
        &mut self,
        generator: ObjectRef,
        tag: u32,
        value: Value,
        done: bool,
    ) -> Completion<()> {
        // 1. Assert: generator.[[AsyncGeneratorQueue]] is not empty. 2. Let next be the first
        // element of generator.[[AsyncGeneratorQueue]]. 3. Remove the first element from
        // generator.[[AsyncGeneratorQueue]].
        let Some(next) = self.async_generator_mut(generator)?.queue.pop_front() else {
            return Err(self.fault());
        };
        // 4. Let promiseCapability be next.[[Capability]]. 5. Let value be
        // completion.[[Value]].
        let mark = self.keep(Value::Object(next.promise));
        self.keep(Value::Object(next.resolve));
        self.keep(Value::Object(next.reject));
        self.keep(value);
        let settled = if tag == TAG_THROW {
            // 6. If completion is a throw completion, then a. Perform !
            // Call(promiseCapability.[[Reject]], undefined, « value »).
            self.reject_function(next.reject, value)
        } else {
            // 7. Else, ... Let iteratorResult be CreateIteratorResultObject(value, done). d.
            // Perform ! Call(promiseCapability.[[Resolve]], undefined, « iteratorResult »).
            self.create_iter_result_object(value, done)
                .and_then(|result| {
                    self.keep(Value::Object(result));
                    self.resolve_function(next.resolve, Value::Object(result))
                })
        };
        self.release(mark);
        settled.map(|_| ())
    }

    /// AsyncGeneratorDrainQueue ( generator ) (`#sec-asyncgeneratordrainqueue`), with the state
    /// draining-queue: each request at the head settled with done true, one step spent before it
    /// is taken off (item 3), until the queue is empty (the state then completed) or its head is
    /// a return completion, whose value AsyncGeneratorAwaitReturn awaits. A termination leaves
    /// the state draining-queue and the requests not settled queued (item 5).
    pub(crate) fn async_generator_drain_queue(&mut self, generator: ObjectRef) -> Completion<()> {
        let mark = self.keep(Value::Object(generator));
        let drained = self.async_generator_drain_loop(generator);
        self.release(mark);
        drained
    }

    /// The loop of [`Agent::async_generator_drain_queue`].
    fn async_generator_drain_loop(&mut self, generator: ObjectRef) -> Completion<()> {
        // 1. Assert: generator.[[AsyncGeneratorState]] is draining-queue. 2. Let queue be
        // generator.[[AsyncGeneratorQueue]]. 3. If queue is empty, then a. Set
        // generator.[[AsyncGeneratorState]] to completed. b. Return unused. 4. Let done be
        // false. 5. Repeat, while done is false,
        loop {
            // a. Let next be the first element of queue. b. Let completion be
            // Completion(next.[[Completion]]).
            let head = self.async_generator_mut(generator)?.queue.front().copied();
            let Some(head) = head else {
                // e. If queue is empty, then i. Set generator.[[AsyncGeneratorState]] to
                // completed. ii. Set done to true.
                return self.set_async_generator_state(generator, AsyncGeneratorState::Completed);
            };
            if head.tag == TAG_RETURN {
                // c. If completion is a return completion, then i. Perform
                // AsyncGeneratorAwaitReturn(generator). ii. Set done to true. A throw of its
                // PromiseResolve settles the request at once, and the drain goes on.
                if self.async_generator_await_return(generator, true)? {
                    continue;
                }
                return Ok(());
            }
            // d. Else, i. If completion is a normal completion, then 1. Set completion to
            // NormalCompletion(undefined). ii. Perform AsyncGeneratorCompleteStep(generator,
            // completion, true).
            self.charge_steps(1)?;
            let value = if head.tag == TAG_THROW {
                head.value
            } else {
                Value::Undefined
            };
            self.async_generator_complete_step(generator, head.tag, value, true)?;
        }
    }

    /// AsyncGeneratorAwaitReturn ( generator ) (`#sec-asyncgeneratorawaitreturn`), the head a
    /// return completion: PromiseResolve(%Promise%, its value), then onFulfilled and onRejected,
    /// two functions with the slots [`Slots::Await`] that hold the generator and the number of
    /// this wait, given to PerformPromiseThen; the generator waits at that number, with no frame.
    /// Answers `true` when PromiseResolve threw: the request is then settled with the throw at
    /// once (steps 6 and 7; with `charged`, after the drain's step for it), and the caller
    /// drains on.
    fn async_generator_await_return(
        &mut self,
        generator: ObjectRef,
        charged: bool,
    ) -> Completion<bool> {
        // 1. Assert: generator.[[AsyncGeneratorState]] is draining-queue. 2. Let queue be
        // generator.[[AsyncGeneratorQueue]]. 3. Assert: queue is not empty. 4. Let next be the
        // first element of queue. 5. Let completion be Completion(next.[[Completion]]).
        let head = self.async_generator_mut(generator)?.queue.front().copied();
        let Some(head) = head else {
            return Err(self.fault());
        };
        // 6. Let promiseCompletion be Completion(PromiseResolve(%Promise%,
        // completion.[[Value]])).
        let constructor = self.current_intrinsic(Intrinsic::Promise)?;
        let promise = match self.promise_resolve(constructor, head.value) {
            Ok(Value::Object(promise)) => promise,
            Ok(_) => return Err(self.fault()),
            // 7. If promiseCompletion is an abrupt completion, then a. Perform
            // AsyncGeneratorCompleteStep(generator, promiseCompletion, true). b. Perform
            // AsyncGeneratorDrainQueue(generator) (the caller's). c. Return unused.
            Err(Abrupt::Throw(reason)) => {
                let mark = self.keep(reason);
                let settled = if charged {
                    self.charge_steps(1)
                } else {
                    Ok(())
                }
                .and_then(|()| {
                    self.async_generator_complete_step(generator, TAG_THROW, reason, true)
                });
                self.release(mark);
                settled?;
                return Ok(true);
            }
            Err(abrupt) => return Err(abrupt),
        };
        // 8. Assert: promiseCompletion is a normal completion. 9. Let promise be
        // promiseCompletion.[[Value]].
        let mark = self.keep(Value::Object(promise));
        let wait = self.async_generator_mut(generator)?.wait.saturating_add(1);
        let made = self.current_realm().and_then(|realm| {
            // 10. to 13. onFulfilled and onRejected: CreateBuiltinFunction(closure, 1, "", « »).
            let on_fulfilled = self.create_builtin_closure(
                BuiltinId::AsyncGenerator(AsyncGeneratorBuiltin::AwaitReturnFulfilled),
                Slots::Await {
                    activation: generator,
                    suspension: wait,
                },
                1,
                realm,
            )?;
            self.keep(Value::Object(on_fulfilled));
            let on_rejected = self.create_builtin_closure(
                BuiltinId::AsyncGenerator(AsyncGeneratorBuiltin::AwaitReturnRejected),
                Slots::Await {
                    activation: generator,
                    suspension: wait,
                },
                1,
                realm,
            )?;
            self.keep(Value::Object(on_rejected));
            // 14. Perform PerformPromiseThen(promise, onFulfilled, onRejected).
            self.perform_promise_then(
                promise,
                Value::Object(on_fulfilled),
                Value::Object(on_rejected),
                None,
            )
        });
        self.release(mark);
        made?;
        let record = self.async_generator_mut(generator)?;
        record.waiting = true;
        record.wait = wait;
        // 15. Return unused.
        Ok(false)
    }

    /// fulfilledClosure (`tag` `TAG_NORMAL`) and rejectedClosure (`TAG_THROW`) of
    /// AsyncGeneratorAwaitReturn, the function `function` called with `value`: while its
    /// generator waits at the number of its wait, AsyncGeneratorCompleteStep(generator, the
    /// completion, true), then AsyncGeneratorDrainQueue(generator); otherwise nothing. Either
    /// way `undefined`.
    fn await_return_function(
        &mut self,
        function: ObjectRef,
        value: Value,
        tag: u32,
    ) -> Completion<Value> {
        let (generator, wait) = match self.closure_slots(function)? {
            Slots::Await {
                activation,
                suspension,
            } => (*activation, *suspension),
            Slots::Resolving { .. } | Slots::Executor { .. } | Slots::AsyncFromSync(_) => {
                return Err(self.fault());
            }
        };
        let record = self.async_generator_mut(generator)?;
        if !record.waits_at(wait) {
            return Ok(Value::Undefined);
        }
        record.waiting = false;
        // a. Assert: generator.[[AsyncGeneratorState]] is draining-queue. b. Let result be
        // NormalCompletion(value) (ThrowCompletion(reason)). c. Perform
        // AsyncGeneratorCompleteStep(generator, result, true). d. Perform
        // AsyncGeneratorDrainQueue(generator). e. Return NormalCompletion(undefined).
        let mark = self.keep(Value::Object(generator));
        let settled = self
            .async_generator_complete_step(generator, tag, value, true)
            .and_then(|()| self.async_generator_drain_queue(generator));
        self.release(mark);
        settled?;
        Ok(Value::Undefined)
    }
}
