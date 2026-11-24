//! Promise instances (ECMA-262 27.5.6, `#sec-properties-of-promise-instances`), the PromiseReaction
//! Records (`#sec-promisereaction-records`) and the JobCallback Records
//! (`#sec-jobcallback-records`) of their reactions, written by
//! `story:promise-constructor-then-and-jobs`.
//!
//! The kind holds [[PromiseState]] with [[PromiseResult]] in one enum ([`PromiseState`]),
//! [[PromiseIsHandled]], and [[PromiseFulfillReactions]] with [[PromiseRejectReactions]] in one box
//! ([`Reactions`]), so that the record of every object keeps its size (`object/generator.rs`,
//! `RECORD_BYTES`). The box exists while the promise holds a reaction: its bytes and the
//! capacity of its two lists are charged to the memory budget as the lists grow
//! ([`Agent::add_reactions`]), and credited when the promise settles and takes the lists out
//! ([`Agent::take_reactions`]); a promise freed while pending queues them for the agent to credit
//! after the sweep, as `Grown` does (`docs/design/engine.md`, decision 4, "Queued jobs").
//!
//! The handler of a reaction is a [`Handler`], empty or a JobCallback Record of the callback and
//! the [[HostDefined]] `host_make_job_callback` answered. Every match over it is in this file:
//! [`Agent::handler_realm`], the realm of the job of a reaction (`#sec-newpromisereactionjob`,
//! step 3), and [`Agent::run_handler`], the run of a handler (step 1 of its Job Abstract
//! Closure), which `ops/promise.rs` and the checkpoint of `host/jobs.rs` call. An `await`
//! (`story:async-functions-and-await`) hands PerformPromiseThen two built-in functions with slots,
//! so the handler needs no kind of its own for it.
//!
//! The kind takes every internal method by its ordinary arm (`object/mod.rs`). Its kind site of
//! the collector, [`Promise::trace`], traces the result and every value of every reaction, and the
//! [[HostDefined]] of each JobCallback through `HostData::trace`.

use std::cell::Cell;
use std::rc::Rc;

use crate::heap::arena::Handle;
use crate::heap::collect::Mark;
use crate::host::agent::Agent;
use crate::host::host_object::{Brand, HostData, HostObject, Tracer};
use crate::host::outcome::{Abrupt, Completion};
use crate::host::{Cx, Host, HostDefined, PromiseCapability, Termination};
use crate::object::ObjectKind;
use crate::value::{ObjectRef, Value};

/// [[PromiseState]] with [[PromiseResult]].
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum PromiseState {
    Pending,
    Fulfilled(Value),
    Rejected(Value),
}

/// The slots of a promise.
pub(crate) struct Promise {
    /// [[PromiseState]] and [[PromiseResult]].
    pub(crate) state: PromiseState,
    /// [[PromiseIsHandled]]: set by PerformPromiseThen; read by the rejection tracker of a later
    /// story of slice 10.
    pub(crate) is_handled: bool,
    /// [[PromiseFulfillReactions]] and [[PromiseRejectReactions]], while the promise holds a
    /// reaction.
    pub(crate) reactions: Option<Box<Reactions>>,
}

/// The two lists of reactions of a pending promise, with the bytes they were charged: the box
/// and the capacity of both lists. Its drop queues those bytes for the agent to credit after the
/// sweep that freed the promise; a settlement takes the lists out and credits them at once.
pub(crate) struct Reactions {
    pub(crate) fulfill: Vec<Reaction>,
    pub(crate) reject: Vec<Reaction>,
    charged: u64,
    queue: Rc<Cell<u64>>,
}

impl Drop for Reactions {
    fn drop(&mut self) {
        self.queue
            .set(self.queue.get().saturating_add(self.charged));
    }
}

/// [[Type]] of a PromiseReaction Record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ReactionType {
    Fulfill,
    Reject,
}

/// A PromiseReaction Record (`#sec-promisereaction-records`).
pub(crate) struct Reaction {
    /// [[Capability]]: the capability of the derived promise, or undefined.
    pub(crate) capability: Option<PromiseCapability>,
    /// [[Type]].
    pub(crate) kind: ReactionType,
    /// [[Handler]].
    pub(crate) handler: Handler,
}

/// [[Handler]] of a PromiseReaction Record: empty, or a JobCallback Record. Matched without a
/// wildcard, only in this file.
pub(crate) enum Handler {
    Empty,
    JobCallback(JobCallback),
}

/// A JobCallback Record (`#sec-jobcallback-records`): [[Callback]] and the [[HostDefined]]
/// `host_make_job_callback` answered, kept as the data of a [`HostObject`] so that its `trace` is
/// called the one way host data is.
pub(crate) struct JobCallback {
    pub(crate) callback: ObjectRef,
    host_defined: Option<HostObject>,
}

/// The [[HostDefined]] of a JobCallback, as host data.
struct JobHostDefined(HostDefined);

impl HostData for JobHostDefined {
    fn trace(&self, tracer: &mut Tracer<'_>) {
        self.0.trace(tracer);
    }
}

/// `data` as host data with no brand, which the collector traces through `HostData::trace`.
fn wrap(data: HostDefined) -> HostObject {
    HostObject {
        brand: Brand(0),
        data: Some(Box::new(JobHostDefined(data))),
        error_kind: None,
    }
}

/// The bytes of one reaction in the lists of a pending promise: the record, and the box of a
/// [[HostDefined]] its handler may hold.
pub(crate) const REACTION_BYTES: u64 = size_of::<Reaction>() as u64 + HOST_DEFINED_BYTES;

/// The bytes of the box of a [[HostDefined]] a JobCallback holds.
pub(crate) const HOST_DEFINED_BYTES: u64 = size_of::<JobHostDefined>() as u64;

/// The bytes of the box of the lists.
const BOX_BYTES: u64 = size_of::<Reactions>() as u64;

impl JobCallback {
    /// The record of `callback` with the [[HostDefined]] `host_defined`.
    pub(crate) fn new(callback: ObjectRef, host_defined: Option<HostDefined>) -> JobCallback {
        JobCallback {
            callback,
            host_defined: host_defined.map(wrap),
        }
    }

    /// The [[HostDefined]], shared.
    pub(crate) fn host_defined(&self) -> Option<HostDefined> {
        let data: &dyn std::any::Any = self.host_defined.as_ref()?.data.as_deref()?;
        data.downcast_ref::<JobHostDefined>()
            .map(|held| HostDefined::clone(&held.0))
    }

    /// A second holder of the [[HostDefined]] as host data the collector traces, for the agent
    /// to root while the job of the record runs (`host/jobs.rs`).
    pub(crate) fn host_defined_object(&self) -> Option<HostObject> {
        self.host_defined().map(wrap)
    }

    /// [[Callback]] and what the [[HostDefined]] lists.
    pub(crate) fn trace(&self, mark: &mut Mark) {
        mark.object(self.callback);
        if let Some(host_defined) = &self.host_defined {
            host_defined.trace(mark);
        }
    }
}

impl Reaction {
    /// Every value the reaction holds, and what the [[HostDefined]] of its handler lists.
    pub(crate) fn trace(&self, mark: &mut Mark) {
        if let Some(capability) = &self.capability {
            mark.object(capability.promise);
            mark.value(capability.resolve);
            mark.value(capability.reject);
        }
        match &self.handler {
            Handler::Empty => {}
            Handler::JobCallback(callback) => callback.trace(mark),
        }
    }

    /// The values of the reaction, for the kept stack while its job runs.
    pub(crate) fn values(&self) -> [Value; 4] {
        let (promise, resolve, reject) = match &self.capability {
            Some(capability) => (
                Value::Object(capability.promise),
                capability.resolve,
                capability.reject,
            ),
            None => (Value::Undefined, Value::Undefined, Value::Undefined),
        };
        let callback = match &self.handler {
            Handler::Empty => Value::Undefined,
            Handler::JobCallback(callback) => Value::Object(callback.callback),
        };
        [promise, resolve, reject, callback]
    }

    /// A second holder of the [[HostDefined]] of its handler, as host data the collector
    /// traces ([`JobCallback::host_defined_object`]).
    pub(crate) fn host_defined_object(&self) -> Option<HostObject> {
        match &self.handler {
            Handler::Empty => None,
            Handler::JobCallback(callback) => callback.host_defined_object(),
        }
    }
}

impl Promise {
    /// A pending promise with no reaction, not handled (`#sec-promise-executor`, steps 4 to 8).
    pub(crate) fn new() -> Promise {
        Promise {
            state: PromiseState::Pending,
            is_handled: false,
            reactions: None,
        }
    }

    /// Kind site of the collector: [[PromiseResult]] and every reaction.
    pub(crate) fn trace(&self, mark: &mut Mark) {
        mark.count("object/promise.rs Promise::trace");
        match self.state {
            PromiseState::Pending => {}
            PromiseState::Fulfilled(value) | PromiseState::Rejected(value) => mark.value(value),
        }
        if let Some(reactions) = &self.reactions {
            for reaction in reactions.fulfill.iter().chain(&reactions.reject) {
                reaction.trace(mark);
            }
        }
    }

    /// A pending promise with one reaction of an empty handler and no capability, for the tests
    /// of the collector.
    #[cfg(test)]
    pub(crate) fn for_test() -> Promise {
        let reaction = Reaction {
            capability: None,
            kind: ReactionType::Fulfill,
            handler: Handler::Empty,
        };
        Promise {
            state: PromiseState::Pending,
            is_handled: false,
            reactions: Some(Box::new(Reactions {
                fulfill: vec![reaction],
                reject: Vec::new(),
                charged: 0,
                queue: Rc::new(Cell::new(0)),
            })),
        }
    }
}

/// The bytes of `capacity` reactions.
fn list_bytes(capacity: usize) -> u64 {
    u64::try_from(capacity)
        .unwrap_or(u64::MAX)
        .saturating_mul(REACTION_BYTES)
}

/// The capacity a list of `length` reactions with `capacity` grows to for one more: twice what
/// it was, at least 2.
fn grown_capacity(length: usize, capacity: usize) -> usize {
    if length < capacity {
        capacity
    } else {
        capacity.saturating_mul(2).max(2)
    }
}

impl<H: Host> Agent<H> {
    /// The slots of the promise `promise`; anything but a promise is a broken invariant.
    pub(crate) fn promise_slots(&mut self, promise: ObjectRef) -> Completion<&mut Promise> {
        if !matches!(self.engine_object(promise)?.kind, ObjectKind::Promise(_)) {
            return Err(self.fault());
        }
        match &mut self.engine_object_mut(promise)?.kind {
            ObjectKind::Promise(slots) => Ok(slots),
            _ => Err(Abrupt::Terminated(Termination::EngineFault)),
        }
    }

    /// Append `fulfill` and `reject` to the reaction lists of the pending promise `promise`
    /// (`#sec-performpromisethen`, step 9). The growth of the lists, and the box when the promise
    /// holds none, is charged to the memory budget before it is made; over the budget the run
    /// ends in `Terminated(MemoryBudget)` and nothing is appended.
    pub(crate) fn add_reactions(
        &mut self,
        promise: ObjectRef,
        fulfill: Reaction,
        reject: Reaction,
    ) -> Completion<()> {
        let (fulfill_had, fulfill_wanted, reject_had, reject_wanted, box_bytes) =
            match &self.promise_slots(promise)?.reactions {
                Some(reactions) => (
                    reactions.fulfill.capacity(),
                    grown_capacity(reactions.fulfill.len(), reactions.fulfill.capacity()),
                    reactions.reject.capacity(),
                    grown_capacity(reactions.reject.len(), reactions.reject.capacity()),
                    0,
                ),
                None => (0, 2, 0, 2, BOX_BYTES),
            };
        let growth = list_bytes(fulfill_wanted.saturating_sub(fulfill_had))
            .saturating_add(list_bytes(reject_wanted.saturating_sub(reject_had)))
            .saturating_add(box_bytes);
        self.charge_bytes(growth)?;
        let queue = Rc::clone(&self.credit);
        let slots = self.promise_slots(promise)?;
        let reactions = slots.reactions.get_or_insert_with(|| {
            Box::new(Reactions {
                fulfill: Vec::new(),
                reject: Vec::new(),
                charged: 0,
                queue,
            })
        });
        let reserved = reactions
            .fulfill
            .try_reserve_exact(fulfill_wanted.saturating_sub(reactions.fulfill.len()))
            .and_then(|()| {
                reactions
                    .reject
                    .try_reserve_exact(reject_wanted.saturating_sub(reactions.reject.len()))
            });
        // What the lists hold now is what stays charged: the capacity the allocator gave, which
        // `try_reserve_exact` makes the capacity asked for.
        reactions.charged = reactions.charged.saturating_add(growth);
        if reserved.is_err() {
            return Err(self.terminate(Termination::MemoryBudget));
        }
        reactions.fulfill.push(fulfill);
        reactions.reject.push(reject);
        Ok(())
    }

    /// Take the reaction lists out of the promise `promise` that settles
    /// (`#sec-fulfillpromise` and `#sec-rejectpromise`, steps 2 and 4 to 5: the lists are set to
    /// undefined) and credit what they were charged; answers the reactions of `kind`.
    pub(crate) fn take_reactions(
        &mut self,
        promise: ObjectRef,
        kind: ReactionType,
    ) -> Completion<Vec<Reaction>> {
        let Some(mut reactions) = self.promise_slots(promise)?.reactions.take() else {
            return Ok(Vec::new());
        };
        // Credited here, so the drop of the box queues nothing.
        let charged = std::mem::take(&mut reactions.charged);
        self.heap.accounting.credit(charged);
        Ok(match kind {
            ReactionType::Fulfill => std::mem::take(&mut reactions.fulfill),
            ReactionType::Reject => std::mem::take(&mut reactions.reject),
        })
    }

    /// The realm of the job of a reaction whose handler is `handler`
    /// (`#sec-newpromisereactionjob`, step 3): GetFunctionRealm of its callback, or the current
    /// realm when that throws; `None` for an empty handler.
    pub(crate) fn handler_realm(&mut self, handler: &Handler) -> Completion<Option<Handle>> {
        match handler {
            Handler::Empty => Ok(None),
            Handler::JobCallback(callback) => match self.get_function_realm(callback.callback) {
                Ok(realm) => Ok(Some(realm)),
                Err(Abrupt::Throw(_)) => Ok(Some(self.current_realm()?)),
                Err(abrupt) => Err(abrupt),
            },
        }
    }

    /// The run of the handler of a reaction of `kind` with `argument`
    /// (`#sec-newpromisereactionjob`, steps 1.d and 1.e): for an empty handler the identity of a
    /// fulfil reaction or the thrower of a reject one, otherwise HostCallJobCallback of the
    /// handler with undefined and « argument ».
    pub(crate) fn run_handler(
        &mut self,
        handler: &Handler,
        kind: ReactionType,
        argument: Value,
    ) -> Completion<Value> {
        match (handler, kind) {
            (Handler::Empty, ReactionType::Fulfill) => Ok(argument),
            (Handler::Empty, ReactionType::Reject) => Err(Abrupt::Throw(argument)),
            (Handler::JobCallback(callback), _) => {
                self.host_call_job_callback(callback, Value::Undefined, &[argument])
            }
        }
    }

    /// HostCallJobCallback (`#sec-hostcalljobcallback`) of `callback` with `this` and `args`:
    /// the host's hook with a `Cx`, given the [[HostDefined]] made for the callback; what it
    /// answers is checked as a host function's answer is (decision 6). A termination of the run
    /// while the hook ran is the answer.
    pub(crate) fn host_call_job_callback(
        &mut self,
        callback: &JobCallback,
        this: Value,
        args: &[Value],
    ) -> Completion<Value> {
        let host_defined = callback.host_defined();
        let start = self.scope.len();
        let answer = H::host_call_job_callback(
            &mut Cx { agent: self },
            callback.callback,
            host_defined.as_ref(),
            this,
            args,
        );
        self.scope.truncate(start);
        self.running()?;
        match answer {
            Ok(value) => {
                if self.heap.check_value(value).is_err() {
                    return Err(self.fault());
                }
                Ok(value)
            }
            Err(Abrupt::Throw(value)) => {
                if self.heap.check_value(value).is_err() {
                    return Err(self.fault());
                }
                Err(Abrupt::Throw(value))
            }
            Err(Abrupt::Terminated(termination)) => Err(self.terminate(termination)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{REACTION_BYTES, Reaction};

    /// The bytes one reaction is charged, which `tests/promises.rs` states as `REACTION_BYTES`:
    /// the record and the box of a [[HostDefined]].
    #[test]
    fn a_reaction_is_charged_its_size() {
        assert_eq!(REACTION_BYTES, 112, "{} bytes", size_of::<Reaction>());
    }
}
