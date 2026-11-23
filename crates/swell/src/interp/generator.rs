//! The instructions of a generator template (`swell_compiler::bytecode`, "generators"), on the
//! suspended frame of `interp/suspend.rs` (`docs/design/engine.md`, decision 3, "Suspended
//! frames", item 6). Written by `story:generator-functions-and-suspended-frames`.
//!
//! - `GeneratorStart`: EvaluateGeneratorBody after FunctionDeclarationInstantiation
//!   (`#sec-runtime-semantics-evaluategeneratorbody`): OrdinaryCreateFromConstructor of the
//!   running function with `%GeneratorPrototype%` (`#sec-ordinarycreatefromconstructor`), the
//!   object into the slot the code reserves, GeneratorStart (`#sec-generatorstart`): the state
//!   suspended-start, the frame suspended, the object handed over as the value of the call.
//! - `Yield`: Yield (`#sec-yield`): CreateIteratorResultObject(value, false)
//!   (`#sec-createiterresultobject`), then [`Agent::generator_yield`].
//! - `GeneratorReturn`: the generator completed (GeneratorStart, step 4), then
//!   CreateIteratorResultObject(value, true), returned as any return is: `return_from_frame`
//!   pops the frame and credits it.
//! - `GeneratorDone`: the generator completed, the block of the `finally` around the body,
//!   which a throw that leaves the body runs.
//! - `YieldReturn`: the entries of the expression around a `yield` that got a return
//!   completion, dropped, the iterator of each array pattern among them closed.
//!
//! Each is a step and runs in a function of its own, called from `Agent::execute`.

use super::dispatch::Flow;
use super::frame::{Entry, SlotValue};
use crate::host::Host;
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::object::ObjectKind;
use crate::object::generator::{Generator, GeneratorState};
use crate::realm::intrinsics::Intrinsic;
use crate::value::{ObjectRef, Value};

impl<H: Host> Agent<H> {
    /// The generator object in local slot `slot` of the running frame, which the code of a
    /// generator template guarantees.
    fn generator_in(&mut self, slot: u32) -> Completion<ObjectRef> {
        let position = usize::try_from(slot).unwrap_or(usize::MAX);
        let value = self.top_frame()?.locals.get(position).copied();
        match value {
            Some(SlotValue::Value(Value::Object(generator))) => Ok(generator),
            _ => Err(self.fault()),
        }
    }

    /// Set [[GeneratorState]] of `generator` to `state`.
    pub(crate) fn set_generator_state(
        &mut self,
        generator: ObjectRef,
        state: GeneratorState,
    ) -> Completion<()> {
        let set = match &mut self.engine_object_mut(generator)?.kind {
            ObjectKind::Generator(record) => {
                record.state = state;
                true
            }
            _ => false,
        };
        if set { Ok(()) } else { Err(self.fault()) }
    }

    /// Suspend the running frame into `generator` with the state `state` (suspended-start or
    /// suspended-yield): the state first, then the move ("Suspended frames", item 5).
    fn suspend_generator(&mut self, generator: ObjectRef, state: GeneratorState) -> Completion<()> {
        self.set_generator_state(generator, state)?;
        self.suspend_frame(|agent, suspended| {
            let stored = match &mut agent.engine_object_mut(generator)?.kind {
                ObjectKind::Generator(record) => {
                    record.context = Some(suspended);
                    true
                }
                _ => false,
            };
            if stored { Ok(()) } else { Err(agent.fault()) }
        })
    }

    /// `GeneratorStart(s)`.
    #[inline(never)]
    pub(super) fn generator_start(&mut self, slot: u32) -> Completion<Flow> {
        let Some(function) = self.top_frame()?.function else {
            return Err(self.fault());
        };
        let prototype =
            self.get_prototype_from_constructor(function, Intrinsic::GeneratorPrototype)?;
        let generator =
            self.allocate_object(Some(prototype), ObjectKind::Generator(Generator::new()))?;
        let position = usize::try_from(slot).unwrap_or(usize::MAX);
        let stored = self
            .top_frame()?
            .locals
            .get_mut(position)
            .map(|binding| *binding = SlotValue::Value(Value::Object(generator)));
        if stored.is_none() {
            return Err(self.fault());
        }
        self.suspend_generator(generator, GeneratorState::SuspendedStart)?;
        Ok(Flow::Suspend(Value::Object(generator)))
    }

    /// `Yield(s)`: the value stays on the operand stack while its result object is made.
    #[inline(never)]
    pub(super) fn yield_instruction(&mut self, slot: u32) -> Completion<Flow> {
        let value = self.peek_value(0)?;
        let result = self.create_iter_result_object(value, false)?;
        self.drop_entries(1)?;
        let generator = self.generator_in(slot)?;
        self.generator_yield(generator, Value::Object(result))
    }

    /// GeneratorYield (`#sec-generatoryield`) of the running frame, the code of `generator`,
    /// with `value` as it is: [[GeneratorState]] suspended-yield, the frame suspended into the
    /// generator, and `value` handed over as the value of the resumption that ran it.
    pub(crate) fn generator_yield(
        &mut self,
        generator: ObjectRef,
        value: Value,
    ) -> Completion<Flow> {
        self.suspend_generator(generator, GeneratorState::SuspendedYield)?;
        Ok(Flow::Suspend(value))
    }

    /// `GeneratorReturn(s)`: the value stays on the operand stack while its result object is
    /// made; the interrupt is checked as at every return.
    #[inline(never)]
    pub(super) fn generator_return(&mut self, slot: u32) -> Completion<Flow> {
        let generator = self.generator_in(slot)?;
        self.set_generator_state(generator, GeneratorState::Completed)?;
        let value = self.peek_value(0)?;
        let result = self.create_iter_result_object(value, true)?;
        self.drop_entries(1)?;
        self.check_interrupt()?;
        Ok(Flow::Return(Value::Object(result)))
    }

    /// `GeneratorDone(s)`.
    #[inline(never)]
    pub(super) fn generator_done(&mut self, slot: u32) -> Completion<Flow> {
        let generator = self.generator_in(slot)?;
        self.set_generator_state(generator, GeneratorState::Completed)?;
        Ok(Flow::Next)
    }

    /// `YieldReturn(n)`: the received value stays on top while the Iterator Records above the
    /// first `n` entries are closed, innermost first; each is marked done before its close, so
    /// a throw of a close leaves the records to the handlers of their patterns, which close
    /// the ones not done yet with the throw. Then every entry above `n` but the value is
    /// dropped (their records credited by `truncate_stack`).
    #[inline(never)]
    pub(super) fn yield_return(&mut self, items: u32) -> Completion<Flow> {
        let items = usize::try_from(items).unwrap_or(usize::MAX);
        let value = self.peek_value(0)?;
        let mut below = self.top_frame()?.stack.len().saturating_sub(1);
        loop {
            let found = {
                let frame = self.top_frame()?;
                let mut found = None;
                while below > items {
                    below -= 1;
                    if let Some(Entry::Iterator(record)) = frame.stack.get_mut(below)
                        && !record.done
                    {
                        record.done = true;
                        found = Some(**record);
                        break;
                    }
                }
                found
            };
            let Some(record) = found else { break };
            self.iterator_close(&record, Ok(value))?;
        }
        if self.top_frame()?.stack.len() <= items {
            return Err(self.fault());
        }
        self.truncate_stack(items)?;
        self.push_value(value)?;
        Ok(Flow::Next)
    }
}
