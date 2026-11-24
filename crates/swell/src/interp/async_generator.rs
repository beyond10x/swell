//! The instructions of an async generator template (`swell_compiler::bytecode`, "async
//! generators") and the `await` of an async generator, on the suspended frame of
//! `interp/suspend.rs` (`docs/design/engine.md`, decision 3, "Async generators on suspended
//! frames", items 1 to 5). Written by `story:async-generators-and-their-request-queue`.
//!
//! - `AsyncGeneratorStart`: EvaluateAsyncGeneratorBody after FunctionDeclarationInstantiation
//!   (`#sec-runtime-semantics-evaluateasyncgeneratorbody`): OrdinaryCreateFromConstructor of the
//!   running function with `%AsyncGeneratorPrototype%` (`#sec-ordinarycreatefromconstructor`),
//!   the box of its queue charged with it, the object into the slot the code reserves,
//!   AsyncGeneratorStart (`#sec-asyncgeneratorstart`): the state suspended-start, the queue
//!   empty, the frame suspended, the object handed over as the value of the call.
//! - `AsyncGeneratorYield`: AsyncGeneratorYield (`#sec-asyncgeneratoryield`) of the awaited
//!   operand: the request at the head settled with `{ value, done: false }`; then the completion
//!   of the next request at once when one is queued (step 9), or the state suspended-yield and
//!   the frame suspended, with `undefined` handed over to the code that resumed it.
//! - `AsyncGeneratorReturn` and `AsyncGeneratorReject`: AsyncGeneratorStart, steps 4.f to 4.j:
//!   the state draining-queue, the request at the head settled with `{ value, done: true }` or
//!   rejected, AsyncGeneratorDrainQueue, then `undefined` returned by the frame
//!   (`return_from_frame` pops it and credits it).
//! - The `await` of an async generator ([`Agent::async_generator_await`], from
//!   `Agent::await_instruction` when its holder is an async generator): as an `async`
//!   function's, with the generator as the holder and the number of its wait; its state stays
//!   executing while its frame is suspended there, and the value handed over is `undefined`.
//!   [`Agent::async_generator_await_function`] is the behaviour of the two functions of such an
//!   `await`.
//!
//! Each instruction is a step and runs in a function of its own, called from `Agent::execute`.
//! Settling a request runs the steps of its resolving function at once, with no step of its
//! own; the drain spends one step for each request it settles (item 3). A termination enters
//! no handler (item 5).

use swell_compiler::bytecode::{TAG_NORMAL, TAG_THROW};

use super::dispatch::Flow;
use super::frame::{Entry, SlotValue};
use crate::host::Host;
use crate::host::agent::Agent;
use crate::host::outcome::{Completion, Termination};
use crate::object::async_generator::{AsyncGenerator, AsyncGeneratorState, queue_box_bytes};
use crate::object::builtin_function::{BuiltinId, Slots};
use crate::object::promise::ReactionType;
use crate::object::{ObjectKind, ObjectRecord};
use crate::realm::intrinsics::Intrinsic;
use crate::value::{ObjectRef, Value};

impl<H: Host> Agent<H> {
    /// The async generator object in local slot `slot` of the running frame, which the code of
    /// an async generator template guarantees after its start.
    fn async_generator_in(&mut self, slot: u32) -> Completion<ObjectRef> {
        let position = usize::try_from(slot).unwrap_or(usize::MAX);
        let value = self.top_frame()?.locals.get(position).copied();
        match value {
            Some(SlotValue::Value(Value::Object(generator))) => Ok(generator),
            _ => Err(self.fault()),
        }
    }

    /// Suspend the running frame into `generator`, whose state the caller set.
    fn suspend_async_generator(&mut self, generator: ObjectRef) -> Completion<()> {
        self.suspend_frame(|agent, suspended| {
            agent.async_generator_mut(generator)?.frame = Some(suspended);
            Ok(())
        })
    }

    /// `AsyncGeneratorStart(s)`.
    #[inline(never)]
    pub(super) fn async_generator_start(&mut self, slot: u32) -> Completion<Flow> {
        let Some(function) = self.top_frame()?.function else {
            return Err(self.fault());
        };
        let prototype =
            self.get_prototype_from_constructor(function, Intrinsic::AsyncGeneratorPrototype)?;
        let record = ObjectRecord::new(
            Some(prototype),
            ObjectKind::AsyncGenerator(AsyncGenerator::new()),
        );
        let generator = match self.heap.new_object(record, queue_box_bytes()) {
            Ok(generator) => generator,
            Err(_) => return Err(self.terminate(Termination::MemoryBudget)),
        };
        let position = usize::try_from(slot).unwrap_or(usize::MAX);
        let stored = self
            .top_frame()?
            .locals
            .get_mut(position)
            .map(|binding| *binding = SlotValue::Value(Value::Object(generator)));
        if stored.is_none() {
            return Err(self.fault());
        }
        self.suspend_async_generator(generator)?;
        Ok(Flow::Suspend(Value::Object(generator)))
    }

    /// `AsyncGeneratorYield(s)`: the awaited value stays on the operand stack while the request
    /// at the head is settled.
    #[inline(never)]
    pub(super) fn async_generator_yield(&mut self, slot: u32) -> Completion<Flow> {
        let generator = self.async_generator_in(slot)?;
        let value = self.peek_value(0)?;
        // 9. Perform AsyncGeneratorCompleteStep(generator, completion, false, previousRealm).
        self.async_generator_complete_step(generator, TAG_NORMAL, value, false)?;
        self.drop_entries(1)?;
        // 10. Let queue be generator.[[AsyncGeneratorQueue]]. 11. If queue is not empty, then
        // a. NOTE: Execution continues without suspending the generator. b. Let toYield be the
        // first element of queue. c. Let resumptionValue be Completion(toYield.[[Completion]]).
        // d. Return ? AsyncGeneratorUnwrapYieldResumption(resumptionValue) (the code's).
        let head = self.async_generator_mut(generator)?.queue.front().copied();
        if let Some(head) = head {
            self.push_value(head.value)?;
            self.push_entry(Entry::Tag(head.tag))?;
            return Ok(Flow::Next);
        }
        // 12. Set generator.[[AsyncGeneratorState]] to suspended-yield. 13. Let resumptionValue
        // be Completion(RunCallerContext(undefined)).
        self.set_async_generator_state(generator, AsyncGeneratorState::SuspendedYield)?;
        self.suspend_async_generator(generator)?;
        Ok(Flow::Suspend(Value::Undefined))
    }

    /// `AsyncGeneratorReturn(s)`: the value on the operand stack settles the head.
    #[inline(never)]
    pub(super) fn async_generator_return(&mut self, slot: u32) -> Completion<Flow> {
        self.async_generator_end(slot, TAG_NORMAL)
    }

    /// `AsyncGeneratorReject(s)`, with the thrown value on the operand stack.
    #[inline(never)]
    pub(super) fn async_generator_reject(&mut self, slot: u32) -> Completion<Flow> {
        self.async_generator_end(slot, TAG_THROW)
    }

    /// The end of the body (AsyncGeneratorStart, steps 4.f to 4.j): draining-queue, the head
    /// settled with the value on top (`TAG_NORMAL`, done true) or rejected with it
    /// (`TAG_THROW`), the value dropped, the drain, and `undefined` returned; the interrupt is
    /// checked as at every return.
    fn async_generator_end(&mut self, slot: u32, tag: u32) -> Completion<Flow> {
        let generator = self.async_generator_in(slot)?;
        let value = self.peek_value(0)?;
        // g. Set acGenerator.[[AsyncGeneratorState]] to draining-queue.
        self.set_async_generator_state(generator, AsyncGeneratorState::DrainingQueue)?;
        // h. and i. The completion: a normal or a return completion is a normal one of its
        // value. j. Perform AsyncGeneratorCompleteStep(acGenerator, result, true).
        self.async_generator_complete_step(generator, tag, value, true)?;
        self.drop_entries(1)?;
        // k. Perform AsyncGeneratorDrainQueue(acGenerator).
        self.async_generator_drain_queue(generator)?;
        self.check_interrupt()?;
        // l. Return undefined.
        Ok(Flow::Return(Value::Undefined))
    }

    /// `Await(s)` when slot `s` holds the async generator `generator` (`#sec-await`): as the
    /// `await` of an `async` function, with the generator as the holder of its two functions
    /// and of its frame, and the number of its wait raised; the state stays executing.
    #[inline(never)]
    pub(super) fn async_generator_await(&mut self, generator: ObjectRef) -> Completion<Flow> {
        let value = self.peek_value(0)?;
        // 2. Let promise be ? PromiseResolve(%Promise%, value).
        let constructor = self.current_intrinsic(Intrinsic::Promise)?;
        let Value::Object(promise) = self.promise_resolve(constructor, value)? else {
            return Err(self.fault());
        };
        let mark = self.keep(Value::Object(promise));
        let wait = self.async_generator_mut(generator)?.wait.saturating_add(1);
        let made = self.current_realm().and_then(|realm| {
            // 3. to 6. onFulfilled and onRejected: CreateBuiltinFunction(closure, 1, "", « »).
            let on_fulfilled = self.create_builtin_closure(
                BuiltinId::Await(ReactionType::Fulfill),
                Slots::Await {
                    activation: generator,
                    suspension: wait,
                },
                1,
                realm,
            )?;
            self.keep(Value::Object(on_fulfilled));
            let on_rejected = self.create_builtin_closure(
                BuiltinId::Await(ReactionType::Reject),
                Slots::Await {
                    activation: generator,
                    suspension: wait,
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
        // undefined: the code that resumed the generator answers its own value.
        self.drop_entries(1)?;
        {
            let record = self.async_generator_mut(generator)?;
            record.waiting = true;
            record.wait = wait;
        }
        self.suspend_async_generator(generator)?;
        Ok(Flow::Suspend(Value::Undefined))
    }

    /// The behaviour of onFulfilled (`tag` `TAG_NORMAL`) and onRejected (`TAG_THROW`) of an
    /// `await` of the async generator `generator`, made for its wait `wait`, called with
    /// `argument`: while the generator waits there, the resumption with `argument` and the tag
    /// (a refusal of its checks throws from here and leaves the generator suspended);
    /// otherwise nothing. Either way `undefined`.
    pub(crate) fn async_generator_await_function(
        &mut self,
        generator: ObjectRef,
        wait: u64,
        argument: Value,
        tag: u32,
    ) -> Completion<Value> {
        if !self.async_generator_mut(generator)?.waits_at(wait) {
            return Ok(Value::Undefined);
        }
        let resumption = self.resume_frame(
            |agent| {
                let record = agent.async_generator_mut(generator)?;
                record.waiting = false;
                match record.frame.take() {
                    Some(frame) => Ok(frame),
                    None => Err(agent.fault()),
                }
            },
            argument,
            tag,
        )?;
        self.run_resumed(resumption)?;
        Ok(Value::Undefined)
    }
}
