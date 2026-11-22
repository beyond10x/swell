//! The instructions of an array pattern (`swell_compiler::bytecode`, "array patterns":
//! `PatternStep`, `PatternSkip` and `PatternRest`), each in a function of its own that one arm
//! of `interp/dispatch.rs` calls, so the frame of the instruction loop stays as it was; and the
//! cut of the operand stack a [`HandlerKind::Pattern`] handler makes. Written by
//! `story:array-destructuring-and-spread`.
//!
//! The design is `docs/design/engine.md`, decision 3, "The iterator of `for`-`of`, of an array
//! pattern and of a spread", item 5: an ArrayBindingPattern
//! (`#sec-runtime-semantics-iteratorbindinginitialization`) and an ArrayAssignmentPattern
//! (`#sec-runtime-semantics-iteratordestructuringassignmentevaluation`) hold the Iterator Record
//! of their value on the operand stack as [`Entry::Iterator`], as a for-of does, above the value
//! and under the references of the element being read; its [[Done]] says whether the pattern's
//! end closes the iterator. Each step sets [[Done]] when `next` throws, answers no object, says
//! it is done, or a read of "done" or "value" throws, so the close the compiler puts after the
//! elements, and the handler around them, skip such an iterator. A termination enters no
//! handler, so it calls no `return`.
//!
//! [`HandlerKind::Pattern`]: swell_compiler::bytecode::HandlerKind::Pattern

use super::dispatch::Flow;
use super::frame::Entry;
use crate::host::Host;
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::ops::iteration::IteratorRecord;
use crate::value::Value;

impl<H: Host> Agent<H> {
    /// `PatternStep(n)`: the next value of the record `n` entries under the top, `undefined`
    /// once it is done, pushed.
    #[inline(never)]
    pub(super) fn pattern_step(&mut self, depth: u32) -> Completion<Flow> {
        let depth = usize::try_from(depth).unwrap_or(usize::MAX);
        let mut record = self.pattern_record(depth)?;
        let mut value = Value::Undefined;
        if !record.done {
            let step = self.iterator_step_value(&mut record);
            self.set_pattern_done(depth, record.done)?;
            value = step?.unwrap_or(Value::Undefined);
        }
        self.push_value(value)?;
        Ok(Flow::Next)
    }

    /// `PatternSkip`: an elision, IteratorStep (`#sec-iteratorstep`) of the record on top unless
    /// it is done: `next` called, "done" read, "value" not.
    #[inline(never)]
    pub(super) fn pattern_skip(&mut self) -> Completion<Flow> {
        let mut record = self.pattern_record(0)?;
        if record.done {
            return Ok(Flow::Next);
        }
        let step = self.elision_step(&mut record);
        self.set_pattern_done(0, record.done)?;
        step?;
        Ok(Flow::Next)
    }

    /// IteratorStep of `record` with its result dropped: IteratorNext (`#sec-iteratornext`),
    /// then IteratorComplete (`#sec-iteratorcomplete`), [[Done]] set by a throw of either, a
    /// result that is no object, or done.
    fn elision_step(&mut self, record: &mut IteratorRecord) -> Completion<()> {
        let result = self.call_value(record.next_method, Value::Object(record.iterator), &[]);
        let result = match result {
            Ok(Value::Object(result)) => result,
            Ok(_) => {
                record.done = true;
                return Err(self.throw_error(
                    crate::host::ErrorKind::TypeError,
                    "the iterator result is not an object",
                ));
            }
            Err(abrupt) => {
                record.done = true;
                return Err(abrupt);
            }
        };
        let mark = self.keep(Value::Object(result));
        let done = self
            .name_key("done")
            .and_then(|key| self.get(result, key))
            .and_then(|done| self.to_boolean(done));
        self.release(mark);
        match done {
            Ok(false) => Ok(()),
            Ok(true) => {
                record.done = true;
                Ok(())
            }
            Err(abrupt) => {
                record.done = true;
                Err(abrupt)
            }
        }
    }

    /// `PatternRest(n)`: a new Array (`#sec-arraycreate`) pushed, then each value of the record
    /// `n + 1` entries under the top, from IteratorStepValue until it is done, defined at the
    /// next index, one step charged before each call of `next`; nothing is read once the record
    /// is done.
    #[inline(never)]
    pub(super) fn pattern_rest(&mut self, depth: u32) -> Completion<Flow> {
        let depth = usize::try_from(depth).unwrap_or(usize::MAX);
        let array = self.array_create(0.0, None)?;
        self.push_value(Value::Object(array))?;
        let below = depth.saturating_add(1);
        let mut record = self.pattern_record(below)?;
        let mut index = 0.0;
        while !record.done {
            self.charge_steps(1)?;
            let step = self.iterator_step_value(&mut record);
            self.set_pattern_done(below, record.done)?;
            let Some(value) = step? else {
                break;
            };
            let mark = self.keep(value);
            let defined = self
                .index_key(index)
                .and_then(|key| self.create_data_property_or_throw(array, key, value));
            self.release(mark);
            defined?;
            index += 1.0;
        }
        self.set_pattern_done(below, true)?;
        Ok(Flow::Next)
    }

    /// The length the operand stack of the running frame is cut to when a throw enters a
    /// [`HandlerKind::Pattern`] handler whose `items` is `items`: just above the topmost
    /// Iterator Record at `items` or above, the record of the pattern whose elements the throw
    /// left. A pattern nested in it, whose handler ran first, has dropped its own.
    ///
    /// [`HandlerKind::Pattern`]: swell_compiler::bytecode::HandlerKind::Pattern
    pub(super) fn pattern_handler_items(&mut self, items: usize) -> Completion<usize> {
        let frame = self.top_frame()?;
        let found = frame
            .stack
            .iter()
            .enumerate()
            .skip(items)
            .rev()
            .find(|(_, entry)| matches!(entry, Entry::Iterator(_)))
            .map(|(index, _)| index.saturating_add(1));
        match found {
            Some(length) => Ok(length),
            None => Err(self.fault()),
        }
    }

    /// The Iterator Record `depth` entries below the top of the operand stack (0 is the top).
    fn pattern_record(&mut self, depth: usize) -> Completion<IteratorRecord> {
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
    fn set_pattern_done(&mut self, depth: usize, done: bool) -> Completion<()> {
        let frame = self.top_frame()?;
        let index = frame.stack.len().checked_sub(depth.saturating_add(1));
        match index.and_then(|index| frame.stack.get_mut(index)) {
            Some(Entry::Iterator(record)) => {
                record.done = done;
                Ok(())
            }
            _ => Err(self.fault()),
        }
    }
}
