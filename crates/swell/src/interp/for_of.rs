//! The instructions of the for-of statement (`swell_compiler::bytecode`, `GetIterator`,
//! `IteratorStep` and `IteratorClose`), each in a function of its own that one arm of
//! `interp/dispatch.rs` calls, so the frame of the instruction loop stays as it was. Written by
//! `story:for-of-statement`; the instructions of the for-await-of statement
//! (`GetAsyncIterator`, `AsyncIteratorNext`, `AsyncIteratorStep`, `AsyncIteratorClose` and
//! `AsyncIteratorClosed`, around the `Await` of `interp/async_function.rs`) by
//! `story:for-await-and-async-from-sync-iterator`.
//!
//! The design is `docs/design/engine.md`, decision 3, "The iterator of `for`-`of`, of an array
//! pattern and of a spread": the Iterator Record (`#sec-iterator-records`) lives on the operand
//! stack as [`Entry::Iterator`] for the whole loop, beside the enumerator of a for-in; the
//! frame's trace holds its [[Iterator]] and [[NextMethod]], and its bytes are charged to the
//! memory budget with the frame's `charged` and credited when the entry leaves the stack
//! ([`Agent::truncate_stack`]). Each call of `next` and of `return` goes through the agent's
//! call path (`ops/iteration.rs`). IteratorClose is compiled like a `finally` block over the
//! body, entered with a completion record; a termination enters no handler, so it calls no
//! `return`.

use swell_compiler::bytecode::TAG_THROW;

use super::dispatch::Flow;
use super::frame::Entry;
use crate::host::agent::Agent;
use crate::host::outcome::{Abrupt, Completion};
use crate::host::{ErrorKind, Host};
use crate::ops::iteration::IteratorRecord;
use crate::value::Value;

/// The bytes an Iterator Record on the operand stack holds besides its entry: its box.
pub(crate) fn record_bytes() -> u64 {
    u64::try_from(size_of::<IteratorRecord>()).unwrap_or(u64::MAX)
}

impl<H: Host> Agent<H> {
    /// `GetIterator`: GetIterator(v, sync) (`#sec-getiterator`) of the value on top, which stays
    /// there, rooted, while `[Symbol.iterator]` and the getter of "next" run; the record replaces
    /// it once its bytes are charged (ForIn/OfHeadEvaluation, step 7.b,
    /// `#sec-runtime-semantics-forinofheadevaluation`).
    #[inline(never)]
    pub(super) fn get_iterator_instruction(&mut self) -> Completion<Flow> {
        let value = self.peek_value(0)?;
        let record = self.get_iterator(value)?;
        self.replace_with_record(record)
    }

    /// `IteratorStep(t)`: IteratorStepValue (`#sec-iteratorstepvalue`) of the record on top,
    /// whose [[Done]] it sets when `next` throws, answers no object, or says it is done
    /// (ForIn/OfBodyEvaluation, steps 6.a to 6.c,
    /// `#sec-runtime-semantics-forin-div-ofbodyevaluation-lhs-stmt-iterator-lhskind-labelset`);
    /// the value is pushed, or the run continues at `t` when the iterator is done.
    #[inline(never)]
    pub(super) fn iterator_step_instruction(&mut self, target: u32) -> Completion<Flow> {
        let mut record = self.record_at(0)?;
        let step = self.iterator_step_value(&mut record);
        self.set_done(0, record.done)?;
        match step? {
            Some(value) => {
                self.push_value(value)?;
                Ok(Flow::Next)
            }
            None => {
                self.top_frame()?.pc = usize::try_from(target).unwrap_or(usize::MAX);
                Ok(Flow::Next)
            }
        }
    }

    /// `IteratorClose`: the completion record on top, the record under its value. A record
    /// whose [[Done]] is set is not closed: `next` threw or said it was done, or the record was
    /// closed already. Otherwise [[Done]] is set and IteratorClose (`#sec-iteratorclose`) is
    /// called with a throw completion of the value for [`TAG_THROW`] and a normal completion
    /// for every other tag (ForIn/OfBodyEvaluation, steps 6.i.iii, 6.k and 6.l.iii, and
    /// LoopContinues, `#sec-loopcontinues`, for a break or continue that leaves the loop). For
    /// a throw the answer is the throw of the body, which the end of the block throws again
    /// with the site it had; for every other tag a throw of `return`, or a result of it that is
    /// no object, replaces the completion.
    #[inline(never)]
    pub(super) fn iterator_close_instruction(&mut self) -> Completion<Flow> {
        let tag = match self.top_frame()?.stack.last() {
            Some(Entry::Tag(tag)) => Some(*tag),
            _ => None,
        };
        let Some(tag) = tag else {
            return Err(self.fault());
        };
        let value = self.peek_value(1)?;
        let record = self.record_at(2)?;
        if record.done {
            return Ok(Flow::Next);
        }
        self.set_done(2, true)?;
        if tag != TAG_THROW {
            self.iterator_close(&record, Ok(value))?;
            return Ok(Flow::Next);
        }
        // The site of the body's throw is put aside while `return` runs, which may throw, and
        // catch, values of its own (decision 13).
        let pending = self.throw_site.take();
        match self.iterator_close(&record, Err(Abrupt::Throw(value))) {
            Err(Abrupt::Terminated(termination)) => Err(Abrupt::Terminated(termination)),
            Ok(_) | Err(Abrupt::Throw(_)) => {
                self.throw_site = pending;
                Ok(Flow::Next)
            }
        }
    }

    /// `GetAsyncIterator`: GetIterator(v, async) (`#sec-getiterator`) of the value on top, which
    /// stays there, rooted, while the methods run; the record replaces it once its bytes are
    /// charged (ForIn/OfHeadEvaluation, step 7.a, `#sec-runtime-semantics-forinofheadevaluation`),
    /// as for `GetIterator`.
    #[inline(never)]
    pub(super) fn get_async_iterator_instruction(&mut self) -> Completion<Flow> {
        let value = self.peek_value(0)?;
        let record = self.get_async_iterator(value)?;
        self.replace_with_record(record)
    }

    /// The value on top replaced by `record`, its bytes charged with the frame's. Nothing runs
    /// from here on: the record's handles need no root until the trace of the frame reaches them
    /// on the stack.
    fn replace_with_record(&mut self, record: IteratorRecord) -> Completion<Flow> {
        let bytes = record_bytes();
        self.charge_bytes(bytes)?;
        let frame = self.top_frame()?;
        frame.charged = frame.charged.saturating_add(bytes);
        let replaced = match frame.stack.last_mut() {
            Some(top @ Entry::Value(_)) => {
                *top = Entry::Iterator(Box::new(record));
                true
            }
            _ => false,
        };
        if !replaced {
            return Err(self.fault());
        }
        Ok(Flow::Next)
    }

    /// `AsyncIteratorNext`: the call of [[NextMethod]] of the record on top with [[Iterator]]
    /// and no argument (ForIn/OfBodyEvaluation, step 6.a,
    /// `#sec-runtime-semantics-forin-div-ofbodyevaluation-lhs-stmt-iterator-lhskind-labelset`),
    /// a call checked against `frame_depth` and `reentry_depth` as every call is; its result is
    /// pushed for the `await` after it.
    #[inline(never)]
    pub(super) fn async_iterator_next_instruction(&mut self) -> Completion<Flow> {
        let record = self.record_at(0)?;
        let result = self.call_value(record.next_method, Value::Object(record.iterator), &[])?;
        self.push_value(result)?;
        Ok(Flow::Next)
    }

    /// `AsyncIteratorStep(t)`: the awaited result of `next` on top, the record under it
    /// (ForIn/OfBodyEvaluation, steps 6.c to 6.f): a `TypeError` when it is not an Object; when
    /// IteratorComplete of it is true, the record's [[Done]] is set, the result dropped and the
    /// run continues at `t`; otherwise IteratorValue of it replaces it. The result stays on the
    /// stack while "done" and "value" are read.
    #[inline(never)]
    pub(super) fn async_iterator_step_instruction(&mut self, target: u32) -> Completion<Flow> {
        let Value::Object(result) = self.peek_value(0)? else {
            return Err(
                self.throw_error(ErrorKind::TypeError, "the iterator result is not an object")
            );
        };
        if self.iterator_complete(result)? {
            self.set_done(1, true)?;
            self.drop_entries(1)?;
            self.top_frame()?.pc = usize::try_from(target).unwrap_or(usize::MAX);
            return Ok(Flow::Next);
        }
        let value = self.iterator_value(result)?;
        self.drop_entries(1)?;
        self.push_value(value)?;
        Ok(Flow::Next)
    }

    /// `AsyncIteratorClose(t)`: the completion record on top, the record under its value
    /// (AsyncIteratorClose, `#sec-asynciteratorclose`, steps 3 and 4). A record whose [[Done]]
    /// is set is not closed (the jump); otherwise [[Done]] is set, and GetMethod(iterator,
    /// "return") is read: `undefined` jumps to `t`, a method is called with no argument and its
    /// result pushed for the `await` after it. For [`TAG_THROW`] a throw of either is dropped
    /// with the jump (step 5: the completion stands), the site of the body's throw put aside
    /// while they run, as `IteratorClose` does; for every other tag it is thrown (step 6).
    #[inline(never)]
    pub(super) fn async_iterator_close_instruction(&mut self, target: u32) -> Completion<Flow> {
        let tag = match self.top_frame()?.stack.last() {
            Some(Entry::Tag(tag)) => Some(*tag),
            _ => None,
        };
        let Some(tag) = tag else {
            return Err(self.fault());
        };
        let record = self.record_at(2)?;
        let jump = usize::try_from(target).unwrap_or(usize::MAX);
        if record.done {
            self.top_frame()?.pc = jump;
            return Ok(Flow::Next);
        }
        self.set_done(2, true)?;
        let pending = (tag == TAG_THROW).then(|| self.throw_site.take());
        let iterator = Value::Object(record.iterator);
        let called = self
            .name_key("return")
            .and_then(|key| self.get_method(iterator, key))
            .and_then(|method| match method {
                Some(method) => self.call_value(method, iterator, &[]).map(Some),
                None => Ok(None),
            });
        match (called, pending) {
            (Err(Abrupt::Terminated(termination)), _) => Err(Abrupt::Terminated(termination)),
            (Ok(Some(result)), pending) => {
                if let Some(pending) = pending {
                    self.throw_site = pending;
                }
                self.push_value(result)?;
                Ok(Flow::Next)
            }
            (Ok(None) | Err(Abrupt::Throw(_)), Some(pending)) => {
                self.throw_site = pending;
                self.top_frame()?.pc = jump;
                Ok(Flow::Next)
            }
            (Ok(None), None) => {
                self.top_frame()?.pc = jump;
                Ok(Flow::Next)
            }
            (Err(abrupt), None) => Err(abrupt),
        }
    }

    /// `AsyncIteratorClosed`: the awaited result of `return` and its tag on top, the completion
    /// record under them (AsyncIteratorClose, steps 5 to 8): both dropped; for a completion of
    /// [`TAG_THROW`] nothing more, otherwise a rejection throws its value and a result that is
    /// not an Object a `TypeError`.
    #[inline(never)]
    pub(super) fn async_iterator_closed_instruction(&mut self) -> Completion<Flow> {
        let tags = {
            let stack = &self.top_frame()?.stack;
            let at = |depth: usize| {
                stack
                    .len()
                    .checked_sub(depth.saturating_add(1))
                    .and_then(|index| stack.get(index))
            };
            match (at(0), at(2)) {
                (Some(Entry::Tag(inner)), Some(Entry::Tag(outer))) => Some((*inner, *outer)),
                _ => None,
            }
        };
        let Some((inner, outer)) = tags else {
            return Err(self.fault());
        };
        let result = self.peek_value(1)?;
        self.drop_entries(2)?;
        // 5. If completion is a throw completion, return ? completion.
        if outer == TAG_THROW {
            return Ok(Flow::Next);
        }
        // 6. If innerResult is a throw completion, return ? innerResult.
        if inner == TAG_THROW {
            return Err(Abrupt::Throw(result));
        }
        // 7. If innerResult.[[Value]] is not an Object, throw a TypeError exception.
        if !matches!(result, Value::Object(_)) {
            return Err(self.throw_error(
                ErrorKind::TypeError,
                "the result of the iterator's return is not an object",
            ));
        }
        Ok(Flow::Next)
    }

    /// The Iterator Record `depth` entries below the top of the operand stack (0 is the top).
    fn record_at(&mut self, depth: usize) -> Completion<IteratorRecord> {
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

    /// Sets [[Done]] of the Iterator Record `depth` entries below the top of the operand stack.
    fn set_done(&mut self, depth: usize, done: bool) -> Completion<()> {
        let frame = self.top_frame()?;
        let index = frame.stack.len().checked_sub(depth.saturating_add(1));
        let found = index.and_then(|index| frame.stack.get_mut(index));
        match found {
            Some(Entry::Iterator(record)) => {
                record.done = done;
                Ok(())
            }
            _ => Err(self.fault()),
        }
    }
}
