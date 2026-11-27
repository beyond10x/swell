//! The activation of a built-in async function (`docs/design/engine.md`, decision 3, "A
//! built-in async function", items 1 to 3), written by `story:array-from-async`: a built-in
//! function whose [[Async]] is true runs its steps as an `async` function runs its body
//! (BuiltinCallOrConstruct, `#sec-builtincallorconstruct`, which calls AsyncFunctionStart with a
//! closure of the steps): the call answers a promise once the steps reach their first Await or
//! their end, and each Await (`#sec-await`) suspends the steps until a job of the awaited
//! promise continues them. `Array.fromAsync` (`builtins/array/from_async.rs`) is the one such
//! builtin of slice 10.
//!
//! The activation is an internal kind of object, `ObjectKind::AsyncBuiltinActivation`, with a
//! null [[Prototype]], which no instruction and no read hands to a script or a host. Its kind
//! holds the state (running, suspended at an Await with the number of that suspension,
//! completed), the capability of the call (its promise and resolving functions, made as
//! `AsyncStart` makes them, without a call of the constructor), and a box with the builtin's own
//! state ([`AsyncBuiltinSteps`]: one variant of fixed size per builtin, that of
//! `Array.fromAsync`), charged with the record as owned bytes when it is made
//! (`Heap::new_object`) and credited by the sweep that frees the record. Its kind site,
//! [`AsyncBuiltinActivation::trace`], traces the capability and every value of the box; no value
//! of it is a root. While its steps run the activation is on the kept stack (`Agent::keep`);
//! while it is suspended the two functions of its Await hold it in their slots (`Slots::Await`
//! of `object/builtin_function.rs`, with `BuiltinId::AsyncBuiltinAwait`), the reactions of the
//! awaited promise hold the functions, and the promise, or the queue once it settled, holds the
//! reactions.
//!
//! An Await of the steps ([`Agent::async_builtin_await`]) makes what an `await` of a script
//! makes (item 3 of "`async` functions on suspended frames"): PromiseResolve(%Promise%, value)
//! with a capability of `%Promise%` made without a call of the constructor and settled at once,
//! as the Async-from-Sync Iterator makes it (`builtins/async_iteration.rs`), two built-in
//! functions with slots, and PerformPromiseThen without a capability, which calls
//! `host_make_job_callback` for each. A function of an Await
//! ([`Agent::async_builtin_await_function`]) called while its activation is suspended at its
//! suspension spends one step (the continuation's, which reads the interrupt), sets the state to
//! running and runs the builtin's steps from that Await to the next or to their end; called at
//! any other time it does nothing. Either way it returns `undefined`. So a continuation costs 3
//! steps before the next of its steps runs (the job taken off, the call of the function, the
//! continuation), takes no frame and no re-entry of its own, and each call it makes into code is
//! a re-entry of that code (item 2). The steps answer a [`Reached`]; their end resolves the
//! promise of the call and a throw completion of them rejects it, the resolving functions run at
//! once; a termination runs none of that and leaves the state running, so the promise stays
//! pending for good (item 3).

use crate::builtins::array::from_async::FromAsync;
use crate::heap::collect::Mark;
use crate::host::agent::Agent;
use crate::host::outcome::{Abrupt, Completion};
use crate::host::{Host, Termination};
use crate::object::builtin_function::{BuiltinId, Slots};
use crate::object::promise::ReactionType;
use crate::object::{ObjectKind, ObjectRecord};
use crate::realm::intrinsics::Intrinsic;
use crate::value::{ObjectRef, Value};
use alloc::boxed::Box;

/// The state of an activation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AsyncBuiltinState {
    /// Its steps run, or a termination ended them while they ran.
    Running,
    /// Its steps wait at the Await whose number is the activation's `suspension`.
    Suspended,
    /// Its steps ended and settled the promise of the call.
    Completed,
}

/// The builtin's own state, one variant of fixed size per built-in async function.
pub(crate) enum AsyncBuiltinSteps {
    /// `Array.fromAsync` (`#sec-array.fromasync`, `builtins/array/from_async.rs`).
    FromAsync(FromAsync),
}

impl AsyncBuiltinSteps {
    /// Every value the builtin's state holds.
    fn trace(&self, mark: &mut Mark) {
        match self {
            AsyncBuiltinSteps::FromAsync(state) => state.trace(mark),
        }
    }
}

/// The bytes the box of the builtin's state is charged, as owned bytes of the activation.
pub(crate) fn steps_box_bytes() -> u64 {
    u64::try_from(size_of::<AsyncBuiltinSteps>()).unwrap_or(u64::MAX)
}

/// The slots of an activation of a built-in async function.
pub(crate) struct AsyncBuiltinActivation {
    pub(crate) state: AsyncBuiltinState,
    /// The number of the last suspension: 0 before the first Await, raised by each.
    pub(crate) suspension: u64,
    /// The promise of the call, and its resolve and reject functions.
    pub(crate) promise: ObjectRef,
    pub(crate) resolve: ObjectRef,
    pub(crate) reject: ObjectRef,
    /// The builtin's own state, in a box: the kind holds one pointer for it, so the record of
    /// every object keeps its size.
    pub(crate) steps: Box<AsyncBuiltinSteps>,
}

impl AsyncBuiltinActivation {
    /// Whether the activation is suspended at the suspension numbered `suspension`.
    pub(crate) fn suspended_at(&self, suspension: u64) -> bool {
        self.state == AsyncBuiltinState::Suspended && self.suspension == suspension
    }

    /// Kind site of the collector: the promise of the call, its resolving functions, and every
    /// value of the builtin's state.
    pub(crate) fn trace(&self, mark: &mut Mark) {
        mark.count("object/async_builtin.rs AsyncBuiltinActivation::trace");
        mark.object(self.promise);
        mark.object(self.resolve);
        mark.object(self.reject);
        self.steps.trace(mark);
    }

    /// An activation whose values are `object`, for the tests of the collector.
    #[cfg(test)]
    pub(crate) fn for_test(object: ObjectRef) -> AsyncBuiltinActivation {
        AsyncBuiltinActivation {
            state: AsyncBuiltinState::Suspended,
            suspension: 1,
            promise: object,
            resolve: object,
            reject: object,
            steps: Box::new(AsyncBuiltinSteps::FromAsync(FromAsync::for_test(object))),
        }
    }
}

/// What a run of a builtin's steps reached: an Await, at which the activation is suspended now,
/// or their end with the value that fulfils the promise of the call. A throw completion of the
/// steps is their `Err(Abrupt::Throw)`.
pub(crate) enum Reached {
    Await,
    Return(Value),
}

impl<H: Host> Agent<H> {
    /// The record of the activation `activation`, to change.
    pub(crate) fn async_builtin_mut(
        &mut self,
        activation: ObjectRef,
    ) -> Completion<&mut AsyncBuiltinActivation> {
        if !matches!(
            self.engine_object(activation)?.kind,
            ObjectKind::AsyncBuiltinActivation(_)
        ) {
            return Err(self.fault());
        }
        match &mut self.engine_object_mut(activation)?.kind {
            ObjectKind::AsyncBuiltinActivation(record) => Ok(record),
            _ => Err(Abrupt::Terminated(Termination::EngineFault)),
        }
    }

    /// The call of a built-in async function (BuiltinCallOrConstruct with AsyncFunctionStart):
    /// NewPromiseCapability(%Promise%) made as `AsyncStart` makes it, the activation holding it
    /// and `steps`, its box charged with the record; then `run` with the activation kept, to
    /// the first Await or the end; then the promise of the call. A termination ends the call
    /// with the activation running.
    pub(crate) fn start_async_builtin(
        &mut self,
        steps: AsyncBuiltinSteps,
        run: impl FnOnce(&mut Self, ObjectRef) -> Completion<Reached>,
    ) -> Completion<Value> {
        let prototype = self.current_intrinsic(Intrinsic::PromisePrototype)?;
        let promise = self.new_promise_object(prototype)?;
        let mark = self.keep(Value::Object(promise));
        let made = self
            .create_resolving_functions(promise)
            .and_then(|(resolve, reject)| {
                self.keep(Value::Object(resolve));
                self.keep(Value::Object(reject));
                let kind = ObjectKind::AsyncBuiltinActivation(AsyncBuiltinActivation {
                    state: AsyncBuiltinState::Running,
                    suspension: 0,
                    promise,
                    resolve,
                    reject,
                    steps: Box::new(steps),
                });
                match self
                    .heap
                    .new_object(ObjectRecord::new(None, kind), steps_box_bytes())
                {
                    Ok(activation) => Ok(activation),
                    Err(_) => Err(self.terminate(Termination::MemoryBudget)),
                }
            });
        self.release(mark);
        let activation = made?;
        let mark = self.keep(Value::Object(activation));
        let reached = run(self, activation);
        let settled = self.async_builtin_settle(activation, reached);
        self.release(mark);
        settled?;
        Ok(Value::Object(promise))
    }

    /// What the steps of `activation` reached, settled: at an Await nothing more; at their end
    /// the promise of the call resolved with the value, and for a throw completion rejected
    /// with the reason, the activation completed and the resolving function run at once
    /// (AsyncBlockStart, `#sec-asyncblockstart`, steps 2.g to 2.i); a termination passes up.
    fn async_builtin_settle(
        &mut self,
        activation: ObjectRef,
        reached: Completion<Reached>,
    ) -> Completion<()> {
        let (value, kind) = match reached {
            Ok(Reached::Await) => return Ok(()),
            Ok(Reached::Return(value)) => (value, ReactionType::Fulfill),
            Err(Abrupt::Throw(reason)) => (reason, ReactionType::Reject),
            Err(abrupt) => return Err(abrupt),
        };
        let settle = {
            let record = self.async_builtin_mut(activation)?;
            record.state = AsyncBuiltinState::Completed;
            match kind {
                ReactionType::Fulfill => record.resolve,
                ReactionType::Reject => record.reject,
            }
        };
        let mark = self.keep(value);
        let settled = match kind {
            ReactionType::Fulfill => self.resolve_function(settle, value),
            ReactionType::Reject => self.reject_function(settle, value),
        };
        self.release(mark);
        settled.map(|_| ())
    }

    /// Await ( value ) (`#sec-await`) of the steps of `activation`: PromiseResolve(%Promise%,
    /// value) (step 2), whose Get of `constructor` may throw, which the caller handles as the
    /// steps' throw completion; onFulfilled and onRejected, built-in functions with the slots
    /// `Slots::Await` that hold the activation and the number of this suspension (steps 3 to
    /// 6); PerformPromiseThen without a capability (step 7); then the activation suspended at
    /// that number. `value` is kept while the promise is made.
    pub(crate) fn async_builtin_await(
        &mut self,
        activation: ObjectRef,
        value: Value,
    ) -> Completion<()> {
        let mark = self.keep(value);
        let promise = self.async_builtin_promise_resolve(value);
        self.release(mark);
        let promise = promise?;
        let mark = self.keep(Value::Object(promise));
        let suspension = self
            .async_builtin_mut(activation)?
            .suspension
            .saturating_add(1);
        let made = self.current_realm().and_then(|realm| {
            let on_fulfilled = self.create_builtin_closure(
                BuiltinId::AsyncBuiltinAwait(ReactionType::Fulfill),
                Slots::Await {
                    activation,
                    suspension,
                },
                1,
                realm,
            )?;
            self.keep(Value::Object(on_fulfilled));
            let on_rejected = self.create_builtin_closure(
                BuiltinId::AsyncBuiltinAwait(ReactionType::Reject),
                Slots::Await {
                    activation,
                    suspension,
                },
                1,
                realm,
            )?;
            self.keep(Value::Object(on_rejected));
            self.perform_promise_then(
                promise,
                Value::Object(on_fulfilled),
                Value::Object(on_rejected),
                None,
            )
        });
        self.release(mark);
        made?;
        // 8. Remove asyncContext from the execution context stack: the steps return to the code
        // that called them, the caller of the builtin or the job.
        let record = self.async_builtin_mut(activation)?;
        record.state = AsyncBuiltinState::Suspended;
        record.suspension = suspension;
        Ok(())
    }

    /// PromiseResolve(%Promise%, value) (`#sec-promise-resolve`) with the capability of step 2
    /// made as `AsyncStart` makes it (a new pending promise of `%Promise.prototype%` of the
    /// current realm and its resolving functions; with the intrinsic `%Promise%` as the
    /// constructor nothing a script sees differs) and step 3 run at once by the resolve function's
    /// steps, so no call of a constructor or executor counts against a depth bound.
    fn async_builtin_promise_resolve(&mut self, value: Value) -> Completion<ObjectRef> {
        // 1. If IsPromise(resolution) is true, then a. Let resolutionCtor be ? Get(resolution,
        // "constructor"). b. If SameValue(resolutionCtor, ctor) is true, return resolution.
        if let Value::Object(object) = value
            && self.is_promise(value)?
        {
            let constructor = self.current_intrinsic(Intrinsic::Promise)?;
            let key = self.name_key("constructor")?;
            let resolution_constructor = self.get(object, key)?;
            if self.same_value(resolution_constructor, Value::Object(constructor))? {
                return Ok(object);
            }
        }
        // 2. Let promiseCapability be ? NewPromiseCapability(ctor). 3. Perform ?
        // Call(promiseCapability.[[Resolve]], undefined, « resolution »). 4. Return
        // promiseCapability.[[Promise]].
        let prototype = self.current_intrinsic(Intrinsic::PromisePrototype)?;
        let promise = self.new_promise_object(prototype)?;
        let mark = self.keep(Value::Object(promise));
        let resolved = self
            .create_resolving_functions(promise)
            .and_then(|(resolve, reject)| {
                self.keep(Value::Object(resolve));
                self.keep(Value::Object(reject));
                self.resolve_function(resolve, value)
            });
        self.release(mark);
        resolved?;
        Ok(promise)
    }

    /// The behaviour of onFulfilled (`kind` `Fulfill`) and onRejected (`Reject`) of an Await of
    /// a built-in async function, the function `function` called with `args`, whose first is the
    /// value or the reason (`#sec-await`,
    /// steps 3 and 5): while its activation is suspended at its suspension, the continuation's
    /// step (which reads the interrupt; a refusal leaves the activation suspended), the state
    /// running, and the builtin's steps from that Await with the value or the reason, settled
    /// as [`Agent::start_async_builtin`] settles them; otherwise nothing. Either way `undefined`.
    pub(crate) fn async_builtin_await_function(
        &mut self,
        kind: ReactionType,
        function: ObjectRef,
        args: &[Value],
    ) -> Completion<Value> {
        // Read here, not in the dispatch of `run_builtin`, whose frame every re-entry through a
        // builtin takes (`STACK_BYTES_PER_REENTRY`, `host/limits.rs`).
        let argument = args.first().copied().unwrap_or(Value::Undefined);
        let (activation, suspension) = match self.closure_slots(function)? {
            Slots::Await {
                activation,
                suspension,
            } => (*activation, *suspension),
            Slots::Resolving { .. }
            | Slots::Executor { .. }
            | Slots::AsyncFromSync(_)
            | Slots::Revocable { .. } => {
                return Err(self.fault());
            }
        };
        if !self.async_builtin_mut(activation)?.suspended_at(suspension) {
            return Ok(Value::Undefined);
        }
        self.charge_steps(1)?;
        let builtin = {
            let record = self.async_builtin_mut(activation)?;
            record.state = AsyncBuiltinState::Running;
            match record.steps.as_ref() {
                AsyncBuiltinSteps::FromAsync(_) => Builtin::FromAsync,
            }
        };
        let mark = self.keep(Value::Object(activation));
        let reached = match builtin {
            Builtin::FromAsync => self.array_from_async_continue(activation, kind, argument),
        };
        let settled = self.async_builtin_settle(activation, reached);
        self.release(mark);
        settled?;
        Ok(Value::Undefined)
    }
}

/// Which built-in async function an activation runs the steps of.
#[derive(Clone, Copy)]
enum Builtin {
    FromAsync,
}

#[cfg(test)]
mod tests {
    use super::steps_box_bytes;

    /// The bytes of the box of an activation's state on this target, which the integration
    /// test `builtins_array_from_async::a_pending_from_async_is_charged_until_its_activation_is_freed`
    /// states as `ACTIVATION_BOX_BYTES`.
    const STEPS_BOX_BYTES: u64 = 112;

    /// "A built-in async function", item 1: the builtin's state is a box of one fixed size,
    /// what the activation is charged as its owned bytes.
    #[test]
    fn the_box_of_an_activation_keeps_its_size() {
        assert_eq!(steps_box_bytes(), STEPS_BOX_BYTES);
    }
}
