//! The instructions of an `async` template (`swell_compiler::bytecode`, "async functions") and
//! the two functions of an `await`, on the suspended frame of `interp/suspend.rs`
//! (`docs/design/engine.md`, decision 3, "`async` functions on suspended frames", items 1 to 5).
//! Written by `story:async-functions-and-await`.
//!
//! - `AsyncStart`: EvaluateAsyncFunctionBody (`#sec-runtime-semantics-evaluateasyncfunctionbody`),
//!   step 1, NewPromiseCapability(`%Promise%`), before FunctionDeclarationInstantiation: a
//!   promise of `%Promise.prototype%` of the current realm and its resolving functions, made
//!   directly (the capability of `%Promise%` is these three, and nothing a script sees differs),
//!   held by a new activation (`object/async_function.rs`) in the slot the code reserves.
//! - `Await`: Await (`#sec-await`): PromiseResolve(`%Promise%`, value), whose Get of
//!   `constructor` may throw at the `await`; onFulfilled and onRejected, the two functions with
//!   slots of `object/builtin_function.rs` (`BuiltinId::Await`, `Slots::Await`), which hold the
//!   activation and the number of this suspension; PerformPromiseThen with no capability, which
//!   calls `host_make_job_callback` for each; then the suspension number raised, the frame moved
//!   into the activation and the promise of the call handed over (`Flow::Suspend`): to the
//!   caller at the first suspension of a call, as a return hands its value over, and at a later
//!   one to the function the job called, whose run it ends.
//! - `AsyncReturn`: AsyncBlockStart (`#sec-asyncblockstart`), step 2.g.ii and 2.h: resolve the
//!   promise with the value, then return the promise as any return does (`return_from_frame`
//!   pops the frame and credits it).
//! - `AsyncReject`: AsyncBlockStart, step 2.i, and EvaluateAsyncFunctionBody, step 3: reject the
//!   promise with the thrown value, then return the promise.
//!
//! The start, the return and the rejection are one step each: the resolving functions run
//! their steps at once, as `call_settling_function` runs them, without the step of a call
//! (`builtins/promise.rs`). A termination enters no handler: the activation of a frame on the
//! stack stays executing, its frame dropped and credited by `drop_frames`, and its promise
//! pending for good (item 5).
//!
//! [`Agent::await_function`] is the behaviour of the two functions of an `await`: called while
//! their activation is suspended at their suspension they resume it, with the value and
//! `TAG_NORMAL` or the reason and `TAG_THROW` ([`Agent::resume_frame`], then
//! [`Agent::run_resumed`]: one re-entry, which ends before the job does); called at any other
//! time they do nothing. Either way they return `undefined`.
//!
//! The slot of the holder takes an async generator object too
//! (`story:async-generators-and-their-request-queue`): `Await` of an async generator and the two
//! functions of its `await` go to `interp/async_generator.rs`, whose generator holds its frame and
//! the number of its wait, and hands `undefined` over at each suspension.

use swell_compiler::bytecode::{TAG_NORMAL, TAG_THROW};

use super::dispatch::Flow;
use super::frame::SlotValue;
use super::suspend::SuspendedFrame;
use crate::host::Host;
use crate::host::agent::Agent;
use crate::host::outcome::{Abrupt, Completion, Termination};
use crate::object::ObjectKind;
use crate::object::async_function::{ActivationState, AsyncActivation};
use crate::object::builtin_function::{BuiltinId, Slots};
use crate::object::promise::ReactionType;
use crate::realm::intrinsics::Intrinsic;
use crate::value::{ObjectRef, Value};

impl<H: Host> Agent<H> {
    /// The activation in local slot `slot` of the running frame, which the code of an `async`
    /// template guarantees after its start.
    fn activation_in(&mut self, slot: u32) -> Completion<ObjectRef> {
        let position = usize::try_from(slot).unwrap_or(usize::MAX);
        let value = self.top_frame()?.locals.get(position).copied();
        match value {
            Some(SlotValue::Value(Value::Object(activation))) => Ok(activation),
            _ => Err(self.fault()),
        }
    }

    /// The activation record of `activation`, to change.
    fn activation_mut(&mut self, activation: ObjectRef) -> Completion<&mut AsyncActivation> {
        if !matches!(
            self.engine_object(activation)?.kind,
            ObjectKind::AsyncActivation(_)
        ) {
            return Err(self.fault());
        }
        match &mut self.engine_object_mut(activation)?.kind {
            ObjectKind::AsyncActivation(record) => Ok(record),
            _ => Err(Abrupt::Terminated(Termination::EngineFault)),
        }
    }

    /// `AsyncStart(s)`.
    #[inline(never)]
    pub(super) fn async_start(&mut self, slot: u32) -> Completion<Flow> {
        let prototype = self.current_intrinsic(Intrinsic::PromisePrototype)?;
        let promise = self.new_promise_object(prototype)?;
        let mark = self.keep(Value::Object(promise));
        let made = self
            .create_resolving_functions(promise)
            .and_then(|(resolve, reject)| {
                self.keep(Value::Object(resolve));
                self.keep(Value::Object(reject));
                self.allocate_object(
                    None,
                    ObjectKind::AsyncActivation(AsyncActivation::new(promise, resolve, reject)),
                )
            });
        self.release(mark);
        let activation = made?;
        let position = usize::try_from(slot).unwrap_or(usize::MAX);
        let stored = self
            .top_frame()?
            .locals
            .get_mut(position)
            .map(|binding| *binding = SlotValue::Value(Value::Object(activation)));
        if stored.is_none() {
            return Err(self.fault());
        }
        Ok(Flow::Next)
    }

    /// `Await(s)`: the value stays on the operand stack while PromiseResolve, the two functions
    /// and the reactions are made, so a throw of PromiseResolve leaves it to the handlers.
    #[inline(never)]
    pub(super) fn await_instruction(&mut self, slot: u32) -> Completion<Flow> {
        let activation = self.activation_in(slot)?;
        if self.is_async_generator(activation)? {
            return self.async_generator_await(activation);
        }
        let value = self.peek_value(0)?;
        // 2. Let promise be ? PromiseResolve(%Promise%, value).
        let constructor = self.current_intrinsic(Intrinsic::Promise)?;
        let Value::Object(promise) = self.promise_resolve(constructor, value)? else {
            return Err(self.fault());
        };
        let mark = self.keep(Value::Object(promise));
        let suspension = self
            .activation_mut(activation)?
            .suspension
            .saturating_add(1);
        let realm = self.current_realm();
        let made = realm.and_then(|realm| {
            // 3. to 6. onFulfilled and onRejected: CreateBuiltinFunction(closure, 1, "", « »).
            let on_fulfilled = self.create_builtin_closure(
                BuiltinId::Await(ReactionType::Fulfill),
                Slots::Await {
                    activation,
                    suspension,
                },
                1,
                realm,
            )?;
            self.keep(Value::Object(on_fulfilled));
            let on_rejected = self.create_builtin_closure(
                BuiltinId::Await(ReactionType::Reject),
                Slots::Await {
                    activation,
                    suspension,
                },
                1,
                realm,
            )?;
            self.keep(Value::Object(on_rejected));
            // 7. Perform PerformPromiseThen(promise, onFulfilled, onRejected).
            self.perform_promise_then(
                promise,
                Value::Object(on_fulfilled),
                Value::Object(on_rejected),
                None,
            )
        });
        self.release(mark);
        made?;
        // 8. Remove asyncContext from the execution context stack. 9. Resume the caller with
        // the promise of the call (`Flow::Suspend` hands it over).
        self.drop_entries(1)?;
        let call_promise = {
            let record = self.activation_mut(activation)?;
            record.state = ActivationState::Suspended;
            record.suspension = suspension;
            record.promise
        };
        self.suspend_frame(|agent, suspended| {
            agent.activation_mut(activation)?.frame = Some(suspended);
            Ok(())
        })?;
        Ok(Flow::Suspend(Value::Object(call_promise)))
    }

    /// `AsyncReturn(s)`: the value stays on the operand stack while the promise is resolved;
    /// the interrupt is checked as at every return.
    #[inline(never)]
    pub(super) fn async_return(&mut self, slot: u32) -> Completion<Flow> {
        self.async_settle(slot, ReactionType::Fulfill)
    }

    /// `AsyncReject(s)`, with the thrown value on the operand stack.
    #[inline(never)]
    pub(super) fn async_reject(&mut self, slot: u32) -> Completion<Flow> {
        self.async_settle(slot, ReactionType::Reject)
    }

    /// The return or the rejection: the activation completed, the promise of the call resolved
    /// or rejected with the value on top, the value dropped and the promise returned.
    fn async_settle(&mut self, slot: u32, kind: ReactionType) -> Completion<Flow> {
        let activation = self.activation_in(slot)?;
        let value = self.peek_value(0)?;
        let (promise, settle) = {
            let record = self.activation_mut(activation)?;
            record.state = ActivationState::Completed;
            let settle = match kind {
                ReactionType::Fulfill => record.resolve,
                ReactionType::Reject => record.reject,
            };
            (record.promise, settle)
        };
        match kind {
            ReactionType::Fulfill => self.resolve_function(settle, value)?,
            ReactionType::Reject => self.reject_function(settle, value)?,
        };
        self.drop_entries(1)?;
        self.check_interrupt()?;
        Ok(Flow::Return(Value::Object(promise)))
    }

    /// The behaviour of onFulfilled (`kind` `Fulfill`) and onRejected (`Reject`) of an `await`
    /// (`#sec-await`, steps 3 and 5), the function `function` called with `argument`: while its
    /// activation is suspended at its suspension, the resumption with `argument` and
    /// `TAG_NORMAL` or `TAG_THROW` (steps 3.b to 3.f; item 4: a refusal of the resumption
    /// throws from here and leaves the activation suspended); otherwise nothing. Either way
    /// `undefined` (step 3.g).
    pub(crate) fn await_function(
        &mut self,
        kind: ReactionType,
        function: ObjectRef,
        argument: Value,
    ) -> Completion<Value> {
        let (activation, suspension) = match self.closure_slots(function)? {
            Slots::Await {
                activation,
                suspension,
            } => (*activation, *suspension),
            Slots::Resolving { .. } | Slots::Executor { .. } | Slots::AsyncFromSync(_) => {
                return Err(self.fault());
            }
        };
        let tag = match kind {
            ReactionType::Fulfill => TAG_NORMAL,
            ReactionType::Reject => TAG_THROW,
        };
        if self.is_async_generator(activation)? {
            return self.async_generator_await_function(activation, suspension, argument, tag);
        }
        if !self.activation_mut(activation)?.suspended_at(suspension) {
            return Ok(Value::Undefined);
        }
        let resumption = self.resume_frame(
            |agent| agent.take_activation_frame(activation),
            argument,
            tag,
        )?;
        self.run_resumed(resumption)?;
        Ok(Value::Undefined)
    }

    /// The move of a resumption after its checks: the activation executing, and its frame taken
    /// out for `Agent::resume_frame` to push.
    fn take_activation_frame(&mut self, activation: ObjectRef) -> Completion<SuspendedFrame> {
        let record = self.activation_mut(activation)?;
        record.state = ActivationState::Executing;
        match record.frame.take() {
            Some(frame) => Ok(frame),
            None => Err(self.fault()),
        }
    }
}
