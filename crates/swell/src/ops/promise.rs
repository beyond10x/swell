//! The abstract operations of promises (ECMA-262 27.5.1, `#sec-promise-abstract-operations`) and
//! the promise jobs (27.5.2, `#sec-promise-jobs`), written by
//! `story:promise-constructor-then-and-jobs`: CreateResolvingFunctions, FulfillPromise,
//! NewPromiseCapability, IsPromise, RejectPromise, TriggerPromiseReactions, PromiseResolve,
//! PerformPromiseThen, NewPromiseReactionJob and NewPromiseResolveThenableJob, with
//! HostMakeJobCallback (`#sec-hostmakejobcallback`). Each is `pub(crate)` with the name the
//! draft gives it, for the `await` of `story:async-functions-and-await`.
//!
//! RejectPromise and PerformPromiseThen call HostPromiseRejectionTracker for a promise not yet
//! handled (`object/promise.rs`, `Agent::track_unhandled_rejection`; HOST-121), written by
//! `story:promise-rejection-tracker-and-promise-reads`.
//!
//! A job is a [`PromiseJob`] record with its realm or none, queued by HostEnqueuePromiseJob
//! (`host/jobs.rs`, `Agent::enqueue_promise_job`), which charges one step and the bytes of its
//! entry before it stores it (`docs/design/engine.md`, decision 3, "The microtask checkpoint is
//! one entry", and decision 4, "Queued jobs"). The checkpoint runs it with
//! [`Agent::run_promise_job`].

use std::cell::Cell;
use std::rc::Rc;

use crate::builtins::promise::PromiseBuiltin;
use crate::heap::arena::Handle;
use crate::host::agent::Agent;
use crate::host::host_object::HostObject;
use crate::host::outcome::{Abrupt, Completion};
use crate::host::{ErrorKind, HookCx, Host, PromiseCapability};
use crate::object::ObjectKind;
use crate::object::builtin_function::{BuiltinId, Slots};
use crate::object::promise::{Handler, JobCallback, Promise, PromiseState, Reaction, ReactionType};
use crate::realm::intrinsics::Intrinsic;
use crate::value::{ObjectRef, Value};

/// The Job Abstract Closure of a promise job, with what it captured.
pub(crate) enum PromiseJob {
    /// NewPromiseReactionJob (`#sec-newpromisereactionjob`): the reaction and the argument.
    Reaction { reaction: Reaction, argument: Value },
    /// NewPromiseResolveThenableJob (`#sec-newpromiseresolvethenablejob`): the promise to resolve,
    /// the thenable and its `then`.
    Thenable {
        promise: ObjectRef,
        thenable: Value,
        then: JobCallback,
    },
}

impl PromiseJob {
    /// The values of the job, for the kept stack while it runs.
    pub(crate) fn values(&self) -> [Value; 5] {
        match self {
            PromiseJob::Reaction { reaction, argument } => {
                let [promise, resolve, reject, callback] = reaction.values();
                [promise, resolve, reject, callback, *argument]
            }
            PromiseJob::Thenable {
                promise,
                thenable,
                then,
            } => [
                Value::Object(*promise),
                *thenable,
                Value::Object(then.callback),
                Value::Undefined,
                Value::Undefined,
            ],
        }
    }

    /// A second holder of the [[HostDefined]] of its JobCallback, as host data the collector
    /// traces, for the agent to root while the job runs (`host/jobs.rs`).
    pub(crate) fn host_defined_object(&self) -> Option<HostObject> {
        match self {
            PromiseJob::Reaction { reaction, .. } => reaction.host_defined_object(),
            PromiseJob::Thenable { then, .. } => then.host_defined_object(),
        }
    }

    /// Every value of the job, and what the [[HostDefined]] of its JobCallback lists.
    pub(crate) fn trace(&self, mark: &mut crate::heap::collect::Mark) {
        match self {
            PromiseJob::Reaction { reaction, argument } => {
                reaction.trace(mark);
                mark.value(*argument);
            }
            PromiseJob::Thenable {
                promise,
                thenable,
                then,
            } => {
                mark.object(*promise);
                mark.value(*thenable);
                then.trace(mark);
            }
        }
    }
}

impl<H: Host> Agent<H> {
    /// IsPromise ( x ) (`#sec-ispromise`): whether `value` is an object with a [[PromiseState]].
    pub(crate) fn is_promise(&mut self, value: Value) -> Completion<bool> {
        match value {
            Value::Object(object) => Ok(matches!(
                self.engine_object(object)?.kind,
                ObjectKind::Promise(_)
            )),
            _ => Ok(false),
        }
    }

    /// A new pending promise whose [[Prototype]] is `prototype` (`#sec-promise-executor`, steps 3
    /// to 8).
    pub(crate) fn new_promise_object(&mut self, prototype: ObjectRef) -> Completion<ObjectRef> {
        self.allocate_object(Some(prototype), ObjectKind::Promise(Promise::new()))
    }

    /// CreateResolvingFunctions ( toResolve ) (`#sec-createresolvingfunctions`): the resolve and
    /// reject functions of `promise`. The draft's record promiseOrEmpty is the [[Promise]] of
    /// each function with one [[AlreadyResolved]] flag the two share (`builtins/promise.rs`).
    /// Each is a built-in function with slots of `length` 1 and the empty `name`, in the current
    /// realm.
    pub(crate) fn create_resolving_functions(
        &mut self,
        promise: ObjectRef,
    ) -> Completion<(ObjectRef, ObjectRef)> {
        // 1. Let promiseOrEmpty be the Record { [[Value]]: toResolve }.
        let already_resolved = Rc::new(Cell::new(false));
        let realm = self.current_realm()?;
        // 2 and 3. resolve, CreateBuiltinFunction(resolveSteps, 1, "", « »).
        let resolve = self.create_builtin_closure(
            BuiltinId::Promise(PromiseBuiltin::ResolveFunction),
            Slots::Resolving {
                promise,
                already_resolved: Rc::clone(&already_resolved),
            },
            1,
            realm,
        )?;
        let mark = self.keep(Value::Object(resolve));
        // 4 and 5. reject, CreateBuiltinFunction(rejectSteps, 1, "", « »).
        let reject = self.create_builtin_closure(
            BuiltinId::Promise(PromiseBuiltin::RejectFunction),
            Slots::Resolving {
                promise,
                already_resolved,
            },
            1,
            realm,
        );
        self.release(mark);
        // 6. Return the Record { [[Resolve]]: resolve, [[Reject]]: reject }.
        Ok((resolve, reject?))
    }

    /// FulfillPromise ( promise, value ) (`#sec-fulfillpromise`).
    pub(crate) fn fulfill_promise(&mut self, promise: ObjectRef, value: Value) -> Completion<()> {
        // 1. Assert: promise.[[PromiseState]] is pending.
        if self.promise_slots(promise)?.state != PromiseState::Pending {
            return Err(self.fault());
        }
        // 2. Let reactions be promise.[[PromiseFulfillReactions]]. 4 to 5: both lists set to
        // undefined, and credited.
        let reactions = self.take_reactions(promise, ReactionType::Fulfill)?;
        // 3. Set promise.[[PromiseResult]] to value. 6. Set promise.[[PromiseState]] to
        // fulfilled.
        self.promise_slots(promise)?.state = PromiseState::Fulfilled(value);
        // 7. Perform TriggerPromiseReactions(reactions, value).
        self.trigger_promise_reactions(reactions, value)
    }

    /// RejectPromise ( promise, reason ) (`#sec-rejectpromise`).
    pub(crate) fn reject_promise(&mut self, promise: ObjectRef, reason: Value) -> Completion<()> {
        // 1. Assert: promise.[[PromiseState]] is pending.
        if self.promise_slots(promise)?.state != PromiseState::Pending {
            return Err(self.fault());
        }
        // 2. Let reactions be promise.[[PromiseRejectReactions]]. 4 to 5: both lists set to
        // undefined, and credited.
        let reactions = self.take_reactions(promise, ReactionType::Reject)?;
        // 3. Set promise.[[PromiseResult]] to reason. 6. Set promise.[[PromiseState]] to
        // rejected.
        self.promise_slots(promise)?.state = PromiseState::Rejected(reason);
        // 7. If promise.[[PromiseIsHandled]] is false, perform
        // HostPromiseRejectionTracker(promise, "reject").
        self.track_unhandled_rejection(promise, crate::host::RejectionOperation::Reject)?;
        // 8. Perform TriggerPromiseReactions(reactions, reason).
        self.trigger_promise_reactions(reactions, reason)
    }

    /// TriggerPromiseReactions ( reactions, argument ) (`#sec-triggerpromisereactions`): a job
    /// for each reaction, in order, each queued with one step (`host/jobs.rs`). A termination
    /// drops the reactions not yet queued with it.
    pub(crate) fn trigger_promise_reactions(
        &mut self,
        reactions: Vec<Reaction>,
        argument: Value,
    ) -> Completion<()> {
        // 1. For each element reaction of reactions, do
        for reaction in reactions {
            // a. Let job be NewPromiseReactionJob(reaction, argument).
            let (job, realm) = self.new_promise_reaction_job(reaction, argument)?;
            // b. Perform HostEnqueuePromiseJob(job.[[Job]], job.[[Realm]]).
            self.enqueue_promise_job(job, realm)?;
        }
        Ok(())
    }

    /// NewPromiseCapability ( ctor ) (`#sec-newpromisecapability`).
    pub(crate) fn new_promise_capability(&mut self, ctor: Value) -> Completion<PromiseCapability> {
        // 1. If IsConstructor(ctor) is false, throw a TypeError exception.
        let constructor = match ctor {
            Value::Object(object) if self.has_construct(object)? => object,
            _ => {
                return Err(self.throw_error(
                    ErrorKind::TypeError,
                    "the promise capability needs a constructor",
                ));
            }
        };
        // 3. Let resolvingFuncs be the Record { [[Resolve]]: undefined, [[Reject]]: undefined }.
        // 4 and 5. executor, CreateBuiltinFunction(executorClosure, 2, "", « »), with
        // resolvingFuncs as its slots.
        let realm = self.current_realm()?;
        let executor = self.create_builtin_closure(
            BuiltinId::Promise(PromiseBuiltin::CapabilityExecutor),
            Slots::Executor {
                resolve: Value::Undefined,
                reject: Value::Undefined,
            },
            2,
            realm,
        )?;
        let mark = self.keep(Value::Object(executor));
        // 6. Let promise be ? Construct(ctor, « executor »).
        let constructed = self.construct_value(constructor, &[Value::Object(executor)], None);
        let slots = self.executor_slots(executor);
        self.release(mark);
        let promise = constructed?;
        let (resolve, reject) = slots?;
        // 7. If IsCallable(resolvingFuncs.[[Resolve]]) is false, throw a TypeError exception.
        // 8. If IsCallable(resolvingFuncs.[[Reject]]) is false, throw a TypeError exception.
        if !self.is_callable(resolve)? || !self.is_callable(reject)? {
            return Err(self.throw_error(
                ErrorKind::TypeError,
                "the promise capability's executor was not given two functions",
            ));
        }
        let Value::Object(promise) = promise else {
            return Err(self.fault());
        };
        // 9. Return the PromiseCapability Record { [[Promise]]: promise, [[Resolve]]:
        // resolvingFuncs.[[Resolve]], [[Reject]]: resolvingFuncs.[[Reject]] }.
        Ok(PromiseCapability {
            promise,
            resolve,
            reject,
        })
    }

    /// resolvingFuncs of the executor `executor`: its [[Resolve]] and [[Reject]].
    fn executor_slots(&mut self, executor: ObjectRef) -> Completion<(Value, Value)> {
        match self.closure_slots(executor)? {
            Slots::Executor { resolve, reject } => Ok((*resolve, *reject)),
            Slots::Resolving { .. } => Err(self.fault()),
        }
    }

    /// NewPromiseCapability(%Promise%) of the current realm, for `Cx::new_promise_capability`.
    pub(crate) fn new_intrinsic_promise_capability(&mut self) -> Completion<PromiseCapability> {
        let constructor = self.current_intrinsic(Intrinsic::Promise)?;
        self.new_promise_capability(Value::Object(constructor))
    }

    /// PromiseResolve ( ctor, resolution ) (`#sec-promise-resolve`).
    pub(crate) fn promise_resolve(
        &mut self,
        ctor: ObjectRef,
        resolution: Value,
    ) -> Completion<Value> {
        // 1. If IsPromise(resolution) is true, then
        if let Value::Object(object) = resolution
            && self.is_promise(resolution)?
        {
            // a. Let resolutionCtor be ? Get(resolution, "constructor").
            let key = self.name_key("constructor")?;
            let resolution_ctor = self.get(object, key)?;
            // b. If SameValue(resolutionCtor, ctor) is true, return resolution.
            if self.same_value(resolution_ctor, Value::Object(ctor))? {
                return Ok(resolution);
            }
        }
        // 2. Let promiseCapability be ? NewPromiseCapability(ctor).
        let capability = self.new_promise_capability(Value::Object(ctor))?;
        let mark = self.keep_capability(&capability);
        // 3. Perform ? Call(promiseCapability.[[Resolve]], undefined, « resolution »).
        let called = self.call_value(capability.resolve, Value::Undefined, &[resolution]);
        self.release(mark);
        called?;
        // 4. Return promiseCapability.[[Promise]].
        Ok(Value::Object(capability.promise))
    }

    /// Keep the values of `capability` on the kept stack; answers the mark to release.
    pub(crate) fn keep_capability(&mut self, capability: &PromiseCapability) -> usize {
        let mark = self.keep(Value::Object(capability.promise));
        self.keep(capability.resolve);
        self.keep(capability.reject);
        mark
    }

    /// HostMakeJobCallback ( callback ) (`#sec-hostmakejobcallback`): the JobCallback Record of
    /// `callback` with the [[HostDefined]] the host's hook answers, through a `HookCx`, which
    /// runs no code. A termination of the run while the hook ran is the answer.
    pub(crate) fn host_make_job_callback(
        &mut self,
        callback: ObjectRef,
    ) -> Completion<JobCallback> {
        let start = self.scope.len();
        let host_defined = H::host_make_job_callback(&mut HookCx { agent: self }, callback);
        self.scope.truncate(start);
        self.running()?;
        Ok(JobCallback::new(callback, host_defined))
    }

    /// The handler of a reaction for `value` (`#sec-performpromisethen`, steps 3 to 6): empty
    /// when it is not callable, otherwise HostMakeJobCallback of it.
    fn reaction_handler(&mut self, value: Value) -> Completion<Handler> {
        match value {
            Value::Object(callback) if self.is_callable(value)? => {
                Ok(Handler::JobCallback(self.host_make_job_callback(callback)?))
            }
            _ => Ok(Handler::Empty),
        }
    }

    /// PerformPromiseThen ( promise, onFulfilled, onRejected [ , resultCapability ] )
    /// (`#sec-performpromisethen`). Answers the promise of `capability`, or undefined without
    /// one.
    pub(crate) fn perform_promise_then(
        &mut self,
        promise: ObjectRef,
        on_fulfilled: Value,
        on_rejected: Value,
        capability: Option<PromiseCapability>,
    ) -> Completion<Value> {
        // 3 to 6. The two handlers, HostMakeJobCallback for each callable one.
        let fulfill_handler = self.reaction_handler(on_fulfilled)?;
        let reject_handler = self.reaction_handler(on_rejected)?;
        // 7. Let fulfillReaction be the PromiseReaction Record { [[Capability]]:
        // resultCapability, [[Type]]: fulfill, [[Handler]]: onFulfilledJobCallback }.
        let fulfill = Reaction {
            capability,
            kind: ReactionType::Fulfill,
            handler: fulfill_handler,
        };
        // 8. Let rejectReaction be the PromiseReaction Record { [[Capability]]:
        // resultCapability, [[Type]]: reject, [[Handler]]: onRejectedJobCallback }.
        let reject = Reaction {
            capability,
            kind: ReactionType::Reject,
            handler: reject_handler,
        };
        match self.promise_slots(promise)?.state {
            // 9. If promise.[[PromiseState]] is pending, then append both.
            PromiseState::Pending => self.add_reactions(promise, fulfill, reject)?,
            // 10. Else if promise.[[PromiseState]] is fulfilled, then
            PromiseState::Fulfilled(value) => {
                // b. Let fulfillJob be NewPromiseReactionJob(fulfillReaction, value).
                let (job, realm) = self.new_promise_reaction_job(fulfill, value)?;
                // c. Perform HostEnqueuePromiseJob(fulfillJob.[[Job]], fulfillJob.[[Realm]]).
                self.enqueue_promise_job(job, realm)?;
            }
            // 11. Else, promise.[[PromiseState]] is rejected.
            PromiseState::Rejected(reason) => {
                // c. If promise.[[PromiseIsHandled]] is false, perform
                // HostPromiseRejectionTracker(promise, "handle").
                self.track_unhandled_rejection(promise, crate::host::RejectionOperation::Handle)?;
                // d. Let rejectJob be NewPromiseReactionJob(rejectReaction, reason).
                let (job, realm) = self.new_promise_reaction_job(reject, reason)?;
                // e. Perform HostEnqueuePromiseJob(rejectJob.[[Job]], rejectJob.[[Realm]]).
                self.enqueue_promise_job(job, realm)?;
            }
        }
        // 12. Set promise.[[PromiseIsHandled]] to true.
        self.promise_slots(promise)?.is_handled = true;
        // 13. If resultCapability is undefined, return undefined. 14. Else, return
        // resultCapability.[[Promise]].
        Ok(capability.map_or(Value::Undefined, |capability| {
            Value::Object(capability.promise)
        }))
    }

    /// NewPromiseReactionJob ( reaction, arg ) (`#sec-newpromisereactionjob`): the job and its
    /// realm, GetFunctionRealm of the handler (the current realm when that throws), or none for
    /// an empty handler.
    pub(crate) fn new_promise_reaction_job(
        &mut self,
        reaction: Reaction,
        argument: Value,
    ) -> Completion<(PromiseJob, Option<Handle>)> {
        // 2 and 3. handlerRealm.
        let realm = self.handler_realm(&reaction.handler)?;
        // 1 and 5. The Job Abstract Closure and the Record { [[Job]], [[Realm]] }.
        Ok((PromiseJob::Reaction { reaction, argument }, realm))
    }

    /// NewPromiseResolveThenableJob ( promiseToResolve, thenable, then )
    /// (`#sec-newpromiseresolvethenablejob`): the job and its realm, GetFunctionRealm of
    /// then.[[Callback]], or the current realm when that throws.
    pub(crate) fn new_promise_resolve_thenable_job(
        &mut self,
        promise: ObjectRef,
        thenable: Value,
        then: JobCallback,
    ) -> Completion<(PromiseJob, Option<Handle>)> {
        // 2 to 4. thenRealm.
        let realm = match self.get_function_realm(then.callback) {
            Ok(realm) => realm,
            Err(Abrupt::Throw(_)) => self.current_realm()?,
            Err(abrupt) => return Err(abrupt),
        };
        // 1 and 6. The Job Abstract Closure and the Record { [[Job]], [[Realm]] }.
        Ok((
            PromiseJob::Thenable {
                promise,
                thenable,
                then,
            },
            Some(realm),
        ))
    }

    /// The Job Abstract Closure of `job`, run by the checkpoint with its values kept: its
    /// completion.
    pub(crate) fn run_promise_job(&mut self, job: &PromiseJob) -> Completion<Value> {
        match job {
            PromiseJob::Reaction { reaction, argument } => {
                // 1.a to 1.e. handlerResult.
                let result = self.run_handler(&reaction.handler, reaction.kind, *argument);
                let result = match result {
                    Err(Abrupt::Terminated(termination)) => {
                        return Err(Abrupt::Terminated(termination));
                    }
                    other => other,
                };
                // 1.f. If promiseCapability is undefined, then: i. Assert: handlerResult is not an
                // abrupt completion. ii. Return empty.
                let Some(capability) = reaction.capability else {
                    return result;
                };
                match result {
                    // 1.h. If handlerResult is an abrupt completion, then return ?
                    // Call(promiseCapability.[[Reject]], undefined, « handlerResult.[[Value]] »).
                    Err(Abrupt::Throw(reason)) => {
                        self.call_settling_function(capability.reject, reason)
                    }
                    // 1.i. Return ? Call(promiseCapability.[[Resolve]], undefined, «
                    // handlerResult.[[Value]] »).
                    Ok(value) => self.call_settling_function(capability.resolve, value),
                    Err(abrupt) => Err(abrupt),
                }
            }
            PromiseJob::Thenable {
                promise,
                thenable,
                then,
            } => {
                // 1.a. Let resolvingFuncs be CreateResolvingFunctions(promiseToResolve).
                let (resolve, reject) = self.create_resolving_functions(*promise)?;
                let mark = self.keep(Value::Object(resolve));
                self.keep(Value::Object(reject));
                // 1.b. Let thenCallResult be Completion(HostCallJobCallback(then, thenable, «
                // resolvingFuncs.[[Resolve]], resolvingFuncs.[[Reject]] »)).
                let result = self.host_call_job_callback(
                    then,
                    *thenable,
                    &[Value::Object(resolve), Value::Object(reject)],
                );
                let completion = match result {
                    // 1.c. If thenCallResult is an abrupt completion, then return !
                    // Call(resolvingFuncs.[[Reject]], undefined, « thenCallResult.[[Value]] »);
                    // a termination or a fault of the agent ends it as it ends every step.
                    Err(Abrupt::Throw(reason)) => {
                        self.call_settling_function(Value::Object(reject), reason)
                    }
                    // 1.d. Return ! thenCallResult.
                    other => other,
                };
                self.release(mark);
                completion
            }
        }
    }
}
