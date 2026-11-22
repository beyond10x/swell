//! The instructions of the for-of statement (`swell_compiler::bytecode`, `GetIterator`,
//! `IteratorStep` and `IteratorClose`), each in a function of its own that one arm of
//! `interp/dispatch.rs` calls, so the frame of the instruction loop stays as it was. Written by
//! `story:for-of-statement`.
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
use crate::host::Host;
use crate::host::agent::Agent;
use crate::host::outcome::{Abrupt, Completion};
use crate::ops::iteration::IteratorRecord;

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
        // Nothing runs from here on: the record's handles need no root until the trace of the
        // frame reaches them on the stack.
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
