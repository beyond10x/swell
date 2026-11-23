//! The delegation of `yield*` (YieldExpression : `yield *` AssignmentExpression,
//! `#sec-generator-function-definitions-runtime-semantics-evaluation`, step 8), as instructions
//! of the delegating frame (`swell_compiler::bytecode`, "generators": `DelegateStep`,
//! `JumpIfDelegateDone` and `DelegateYield`), each in a function of its own that one arm of
//! `interp/dispatch.rs` calls. Written by `story:yield-star-delegation`.
//!
//! The design is `docs/design/engine.md`, decision 3, "Suspended frames", item 6, its paragraph
//! on `yield*`: the Iterator Record of the operand lies on the operand stack of the delegating
//! frame as [`Entry::Iterator`] for the whole loop. When the method a step calls is
//! `%GeneratorPrototype%.next`, `.throw` or `.return` of any realm of the agent and the iterator
//! is a generator object, the step does what that builtin does, with no call: GeneratorValidate
//! with the `TypeError` of the builtin's realm, GeneratorResume or GeneratorResumeAbrupt up to
//! the run (`builtins/generator.rs`), then the checks and the move of a resumption
//! ([`Agent::resume_frame`], `interp/suspend.rs`); the generator's frame then runs above the
//! delegating one on the same run loop, and its suspension or its return hands the inner result
//! to the delegating frame as a return does. So a chain of n generators each delegating to the
//! next holds n frames on the frame stack, counted against `frame_depth`, and takes no Rust
//! frame and no re-entry per level. Any other method is called (`Agent::call_value`), a
//! re-entry as the step of a `for`-`of` is.
//!
//! Per `next` a level spends five steps: the step of its resumption and the four instructions
//! of one turn of its loop (`LoopJump`, `DelegateStep`, `JumpIfDelegateDone`, `DelegateYield`),
//! with the Get of the method for `return` and `throw`; it reads one property ("done") and
//! moves one frame, and nothing walks the chain.

use swell_compiler::bytecode::{TAG_NORMAL, TAG_RETURN, TAG_THROW};

use super::dispatch::Flow;
use super::frame::{Entry, SlotValue};
use crate::builtins::generator::{GeneratorBuiltin, Resume};
use crate::heap::arena::Handle;
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::host::{ErrorKind, Host};
use crate::object::ObjectKind;
use crate::object::builtin_function::{Behaviour, BuiltinId};
use crate::ops::iteration::IteratorRecord;
use crate::value::{ObjectRef, Value};

impl<H: Host> Agent<H> {
    /// `DelegateStep(t)`: one step of the loop with the received completion, its value `v` and
    /// its tag on top of the record `i`.
    ///
    /// - A normal completion (step 8.a): the call of [[NextMethod]] with `v`.
    /// - A throw completion (step 8.b): GetMethod(iterator, "throw"); its call with `v`, or, when
    ///   it is `undefined`, IteratorClose with a normal completion and a `TypeError`.
    /// - A return completion (step 8.c): GetMethod(iterator, "return"); its call with `v`, or,
    ///   when it is `undefined`, the return of `v`: `[v, TAG_RETURN]` and the jump to `t`.
    ///
    /// A call leaves `[i, k, r]`: `r` its result, `k` the tag the `yield*` completes with when
    /// `r` is done, [`TAG_RETURN`] after a return completion and [`TAG_NORMAL`] otherwise. A
    /// generator resumed in the loop leaves `[i, k]`, and its frame, pushed above, hands `r`
    /// over when it suspends or returns.
    #[inline(never)]
    pub(super) fn delegate_step(&mut self, target: u32) -> Completion<Flow> {
        let tag = match self.top_frame()?.stack.last() {
            Some(Entry::Tag(tag)) => Some(*tag),
            _ => None,
        };
        let value = self.peek_value(1)?;
        let record = self.delegate_record(2)?;
        match tag {
            Some(TAG_NORMAL) => self.delegate_call(
                record.next_method,
                record.iterator,
                value,
                GeneratorBuiltin::Next,
                TAG_NORMAL,
            ),
            Some(TAG_THROW) => match self.delegate_method(record.iterator, "throw")? {
                Some(method) => self.delegate_call(
                    method,
                    record.iterator,
                    value,
                    GeneratorBuiltin::Throw,
                    TAG_NORMAL,
                ),
                // 8.b.iii: the record leaves with the throw, by the handler around the loop,
                // which does not close it again.
                None => {
                    self.iterator_close(&record, Ok(Value::Undefined))?;
                    Err(self.throw_error(
                        ErrorKind::TypeError,
                        "the iterator of yield* has no throw method",
                    ))
                }
            },
            Some(TAG_RETURN) => match self.delegate_method(record.iterator, "return")? {
                Some(method) => self.delegate_call(
                    method,
                    record.iterator,
                    value,
                    GeneratorBuiltin::Return,
                    TAG_RETURN,
                ),
                // 8.c.iii: ReturnCompletion(received.[[Value]]).
                None => {
                    self.drop_entries(3)?;
                    self.push_value(value)?;
                    self.push_entry(Entry::Tag(TAG_RETURN))?;
                    self.top_frame()?.pc = usize::try_from(target).unwrap_or(usize::MAX);
                    Ok(Flow::Next)
                }
            },
            _ => Err(self.fault()),
        }
    }

    /// `JumpIfDelegateDone(t)`: `r`, on top, is the inner result; a `TypeError` when it is no
    /// Object (steps 8.a.iii, 8.b.ii.4, 8.c.vi). When IteratorComplete(r) is true, IteratorValue(r)
    /// replaces the three entries with the tag `k` above it, `[v, k]`, the record's box credited,
    /// and the run continues at `t`; otherwise nothing.
    #[inline(never)]
    pub(super) fn jump_if_delegate_done(&mut self, target: u32) -> Completion<Flow> {
        let Value::Object(result) = self.peek_value(0)? else {
            return Err(self.throw_error(
                ErrorKind::TypeError,
                "the result of the iterator of yield* is not an object",
            ));
        };
        if !self.iterator_complete(result)? {
            return Ok(Flow::Next);
        }
        let key = self.name_key("value")?;
        let value = self.get(result, key)?;
        let kind = {
            let frame = self.top_frame()?;
            let below = frame.stack.len().checked_sub(2);
            match below.and_then(|index| frame.stack.get(index)) {
                Some(Entry::Tag(kind)) => Some(*kind),
                _ => None,
            }
        };
        let Some(kind) = kind else {
            return Err(self.fault());
        };
        self.drop_entries(3)?;
        self.push_value(value)?;
        self.push_entry(Entry::Tag(kind))?;
        self.top_frame()?.pc = usize::try_from(target).unwrap_or(usize::MAX);
        Ok(Flow::Next)
    }

    /// `DelegateYield(s)`: GeneratorYield(r) (`#sec-generatoryield`) of the generator in local
    /// slot `s`, with the inner result `r` as it is (step 8.a.vii); `r` and the tag under it
    /// leave the stack first, so the frame suspends with the record on top, and the resumption
    /// puts the received value and its tag above it.
    #[inline(never)]
    pub(super) fn delegate_yield(&mut self, slot: u32) -> Completion<Flow> {
        let result = self.peek_value(0)?;
        self.drop_entries(2)?;
        let position = usize::try_from(slot).unwrap_or(usize::MAX);
        let generator = match self.top_frame()?.locals.get(position).copied() {
            Some(SlotValue::Value(Value::Object(generator))) => generator,
            _ => return Err(self.fault()),
        };
        self.generator_yield(generator, result)
    }

    /// The Iterator Record `depth` entries below the top of the operand stack (0 is the top).
    fn delegate_record(&mut self, depth: usize) -> Completion<IteratorRecord> {
        let frame = self.top_frame()?;
        let found = frame
            .stack
            .len()
            .checked_sub(depth.saturating_add(1))
            .and_then(|index| frame.stack.get(index));
        match found {
            Some(Entry::Iterator(record)) => Ok(**record),
            _ => Err(self.fault()),
        }
    }

    /// GetMethod(iterator, name) (`#sec-getmethod`); the iterator is on the operand stack.
    fn delegate_method(&mut self, iterator: ObjectRef, name: &str) -> Completion<Option<Value>> {
        let key = self.name_key(name)?;
        self.get_method(Value::Object(iterator), key)
    }

    /// The call of `method` with the iterator as the this value and the received value `value`
    /// (steps 8.a.i, 8.b.ii.1, 8.c.iv), or, for `%GeneratorPrototype%.<which>` of any realm and
    /// a generator object, the resumption in the loop ([`Agent::delegate_in_loop`]). `[v, tag]`
    /// on top is replaced by `[k, r]` after a call.
    fn delegate_call(
        &mut self,
        method: Value,
        iterator: ObjectRef,
        value: Value,
        which: GeneratorBuiltin,
        kind: u32,
    ) -> Completion<Flow> {
        if let Some(realm) = self.generator_method_realm(method, which)?
            && matches!(self.engine_object(iterator)?.kind, ObjectKind::Generator(_))
        {
            return self.delegate_in_loop(realm, iterator, value, which, kind);
        }
        let result = self.call_value(method, Value::Object(iterator), &[value])?;
        self.drop_entries(2)?;
        self.push_entry(Entry::Tag(kind))?;
        self.push_value(result)?;
        Ok(Flow::Next)
    }

    /// The [[Realm]] of `method` when it is `%GeneratorPrototype%.<which>` of a realm of the
    /// agent (`Behaviour::Builtin`, `object/builtin_function.rs`); `None` for any other value.
    fn generator_method_realm(
        &mut self,
        method: Value,
        which: GeneratorBuiltin,
    ) -> Completion<Option<Handle>> {
        let Value::Object(function) = method else {
            return Ok(None);
        };
        Ok(match &self.engine_object(function)?.kind {
            ObjectKind::BuiltinFunction(builtin)
                if matches!(
                    builtin.behaviour,
                    Behaviour::Builtin(BuiltinId::Generator(id)) if id == which
                ) =>
            {
                builtin.realm
            }
            _ => None,
        })
    }

    /// What `%GeneratorPrototype%.<which>` of `realm` does with the generator `generator` and
    /// `value`, in the loop: `[v, tag]` on top is replaced by the tag `k` first; then, with
    /// `realm` the current realm as in the builtin, GeneratorValidate and GeneratorResume or
    /// GeneratorResumeAbrupt up to the run. An answer with no run (a completed generator) is
    /// pushed as `r`; a resumption pushes the generator's frame with `value` and its tag
    /// ([`Agent::resume_frame`]: `frame_depth`, the step, `reentry_depth` and the interrupt,
    /// then the move), and the run loop goes on in that frame. `value` is kept while it is in no
    /// frame. Every way out but the pushed frame makes the delegating frame's realm the current
    /// realm again; the frame pushed hands it back as it suspends, returns or throws.
    fn delegate_in_loop(
        &mut self,
        realm: Handle,
        generator: ObjectRef,
        value: Value,
        which: GeneratorBuiltin,
        kind: u32,
    ) -> Completion<Flow> {
        let own = self.top_frame()?.realm;
        self.drop_entries(2)?;
        self.push_entry(Entry::Tag(kind))?;
        let mark = self.keep(value);
        self.set_realm_slot(Some(realm));
        let resumed = self.delegate_resume(generator, value, which);
        self.release(mark);
        if !matches!(resumed, Ok(true)) {
            self.set_realm_slot(Some(own));
        }
        resumed.map(|_| Flow::Next)
    }

    /// GeneratorResume or GeneratorResumeAbrupt of `generator` with `value`, then the answer
    /// pushed (`false`) or the resumption's frame pushed (`true`).
    fn delegate_resume(
        &mut self,
        generator: ObjectRef,
        value: Value,
        which: GeneratorBuiltin,
    ) -> Completion<bool> {
        let generator = Value::Object(generator);
        let resume = match which {
            GeneratorBuiltin::Next => self.generator_resume(generator, value)?,
            GeneratorBuiltin::Return => {
                self.generator_resume_abrupt(generator, value, TAG_RETURN)?
            }
            GeneratorBuiltin::Throw => self.generator_resume_abrupt(generator, value, TAG_THROW)?,
        };
        match resume {
            Resume::Answer(answer) => {
                self.push_value(answer)?;
                Ok(false)
            }
            Resume::Run {
                generator,
                value,
                tag,
            } => {
                self.resume_frame(|agent| agent.take_generator_frame(generator), value, tag)?;
                Ok(true)
            }
        }
    }
}
