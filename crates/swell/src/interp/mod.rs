//! The interpreter (`docs/design/engine.md`, decision 3; ECMA-262 9.4, `#sec-execution-contexts`):
//! a stack-based bytecode interpreter with every ECMAScript frame on the engine's own frame
//! stack, the step budget and the depth bounds. Filled by
//! `story:interpreter-and-script-entry-points`.
//!
//! The frame stack ([`Interp::frames`]) holds the execution contexts of ECMAScript code; the
//! activations of `object/builtin_function.rs` hold those of builtins and host functions. A
//! call from ECMAScript code to an ECMAScript function pushes a frame and the same loop goes on
//! (`interp/dispatch.rs`), with no Rust recursion. Code reached from Rust (a builtin or host
//! function calling a function, a getter, setter, `valueOf` or `toString` an operation of an
//! instruction calls, `evaluate_script`) runs in a loop of its own, [`Agent::run_frame`], which
//! returns when the frame it pushed returns: a re-entry.
//!
//! Re-entries are counted against `reentry_depth` with the count `object/builtin_function.rs`
//! keeps, the number of activations (decision 2): code reached from a builtin or host function
//! is checked against the activations already there, the call into that builtin having counted
//! one; code reached from an operation of an instruction, with no builtin between, is a
//! re-entry too (a run loop nested in the running one) and pushes an activation of its own,
//! which counts it: it is checked against the activations already there plus one. So the
//! activations are never more than `reentry_depth` while code runs, the code at the deepest
//! re-entry may still call a builtin or host function directly, and at most `reentry_depth`
//! run loops are nested in the outermost one (`Limits::min_stack_bytes`). The outermost run,
//! which no code is running around, is the entry and not a re-entry.
//!
//! The current realm is the Realm of the running execution context: the run loop writes the
//! running frame's realm where `realm/mod.rs` reads the current realm (the innermost activation,
//! or the realm of the entry) whenever another frame starts to run, and restores what was there
//! when the loop returns.
//!
//! Root site of the collector: [`trace_roots`] (the frame stack: operand stacks, locals,
//! environments, function and realm of each frame). A suspended frame (`interp/suspend.rs`) is
//! off the frame stack and no root: the kind site of the object that holds it traces it.

mod array_patterns;
mod arrow;
mod async_function;
mod async_generator;
pub(crate) mod budget;
pub(crate) mod dispatch;
mod for_of;
pub(crate) mod frame;
mod generator;
mod lexical;
mod object_literal;
mod parameters;
mod patterns;
mod regexp;
mod spread;
pub(crate) mod suspend;
mod template;
mod templates;
mod yield_star;

use frame::{Entry, Frame, entry_bytes};

use crate::heap::arena::Handle;
use crate::heap::collect::Mark;
use crate::host::agent::Agent;
use crate::host::outcome::{Abrupt, Completion, Termination};
use crate::host::{ErrorKind, Host};
use crate::object::builtin_function::Activation;
use crate::value::{ObjectRef, Value};

/// The interpreter's part of an agent.
#[derive(Default)]
pub(crate) struct Interp {
    /// The ECMAScript execution contexts, outermost first.
    pub(crate) frames: Vec<Frame>,
    /// The number of activations when the innermost running loop started; `None` while no loop
    /// runs.
    pub(crate) running: Option<usize>,
}

/// How code reached from Rust was entered, for [`Agent::leave_code`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Entered {
    /// No code was running: the entry itself.
    Outermost,
    /// A re-entry from a builtin or host function, counted by its activation.
    Reentry,
    /// A re-entry from an operation of an instruction, which pushed an activation.
    Activation,
}

/// Root site of the collector: the frame stack.
pub(crate) fn trace_roots<H: Host>(agent: &Agent<H>, mark: &mut Mark) {
    mark.count("interp/mod.rs trace_roots");
    for frame in &agent.interp.frames {
        frame.trace(mark);
    }
}

impl<H: Host> Agent<H> {
    /// Where the current realm is read: the realm of the innermost activation, or the realm of
    /// the entry.
    pub(crate) fn realm_slot(&self) -> Option<Handle> {
        match self.activations.last() {
            Some(activation) => activation.realm,
            None => self.entry_realm,
        }
    }

    /// Make `realm` the current realm, where [`Agent::realm_slot`] reads it.
    pub(crate) fn set_realm_slot(&mut self, realm: Option<Handle>) {
        match self.activations.last_mut() {
            Some(activation) => activation.realm = realm,
            None => self.entry_realm = realm,
        }
    }

    /// The check of code reached from Rust (the module's documentation): `reentry_depth`, then
    /// the interrupt. `activation` is what the code is called with, its `args` empty, and the
    /// arguments of the call: when no builtin is between the running loop and this call, the
    /// arguments are charged to the memory budget, copied into the activation and it is pushed;
    /// otherwise nothing is copied. A refused charge ends the run in `Terminated(MemoryBudget)`
    /// with nothing pushed.
    pub(crate) fn enter_code(
        &mut self,
        activation: Option<(Activation, &[Value])>,
    ) -> Completion<Entered> {
        let depth = self.activations.len();
        let base = self.interp.running.unwrap_or(0);
        let entered = if depth > base || self.interp.running.is_some() {
            // The re-entries after this one: every activation below made one, the innermost
            // builtin's included (it makes this one); with no builtin between, this call
            // makes one more, which its own activation counts.
            let reentries = match activation {
                Some(_) if depth == base => depth.saturating_add(1),
                _ => depth,
            };
            if u32::try_from(reentries).unwrap_or(u32::MAX) > self.limits.reentry_depth {
                return Err(self.throw_error(ErrorKind::RangeError, "reentry_depth exceeded"));
            }
            match activation {
                Some((mut activation, args)) if depth == base => {
                    // The activation's copy of the arguments lives until `leave_code` pops it:
                    // charged before it is made, credited there.
                    let bytes = u64::try_from(size_of_val(args)).unwrap_or(u64::MAX);
                    self.charge_bytes(bytes)?;
                    let mut kept = Vec::new();
                    if kept.try_reserve_exact(args.len()).is_err() {
                        self.heap.accounting.credit(bytes);
                        return Err(self.terminate(Termination::MemoryBudget));
                    }
                    kept.extend_from_slice(args);
                    #[cfg(test)]
                    crate::object::ecmascript_function::CALL_ARGUMENTS_COPIED.with(|copied| {
                        let count = u64::try_from(args.len()).unwrap_or(u64::MAX);
                        copied.set(copied.get().saturating_add(count));
                    });
                    activation.args = kept;
                    self.activations.push(activation);
                    Entered::Activation
                }
                _ => Entered::Reentry,
            }
        } else {
            Entered::Outermost
        };
        if let Err(abrupt) = self.check_interrupt() {
            self.leave_code(entered);
            return Err(abrupt);
        }
        Ok(entered)
    }

    /// Undo what [`Agent::enter_code`] pushed: the activation, whose arguments are credited.
    pub(crate) fn leave_code(&mut self, entered: Entered) {
        if entered == Entered::Activation
            && let Some(activation) = self.activations.pop()
        {
            let bytes = u64::try_from(size_of_val(activation.args.as_slice())).unwrap_or(u64::MAX);
            self.heap.accounting.credit(bytes);
        }
    }

    /// Push `frame`, run `prepare` in it, then run the frame stack until `frame` returns: its
    /// completion. The current realm and the running loop are restored after, and the frames a
    /// throw or a termination leaves are dropped.
    pub(crate) fn run_frame(
        &mut self,
        frame: Frame,
        prepare: impl FnOnce(&mut Self) -> Completion<()>,
    ) -> Completion<Value> {
        let saved_realm = self.realm_slot();
        let saved_running = self.interp.running.replace(self.activations.len());
        let base = self.interp.frames.len();
        self.push_frame(frame);
        let result = match prepare(self) {
            Ok(()) => self.run(base),
            Err(abrupt) => {
                self.drop_frames(base);
                Err(abrupt)
            }
        };
        self.interp.running = saved_running;
        self.set_realm_slot(saved_realm);
        result
    }

    /// Push a frame, whose realm becomes the current realm.
    pub(crate) fn push_frame(&mut self, frame: Frame) {
        self.set_realm_slot(Some(frame.realm));
        self.interp.frames.push(frame);
    }

    /// Pop the running frame and credit what it was charged.
    pub(crate) fn pop_frame(&mut self) -> Completion<Frame> {
        match self.interp.frames.pop() {
            Some(frame) => {
                self.heap.accounting.credit(frame.charged);
                Ok(frame)
            }
            None => Err(self.fault()),
        }
    }

    /// Drop the frames above `base`, as a termination or an uncaught throw leaves them.
    pub(crate) fn drop_frames(&mut self, base: usize) {
        while self.interp.frames.len() > base {
            if let Some(frame) = self.interp.frames.pop() {
                self.heap.accounting.credit(frame.charged);
            }
        }
    }

    /// The running frame.
    pub(crate) fn top_frame(&mut self) -> Completion<&mut Frame> {
        if self.interp.frames.is_empty() {
            return Err(self.fault());
        }
        self.interp
            .frames
            .last_mut()
            .ok_or(Abrupt::Terminated(Termination::EngineFault))
    }

    /// Push `entry` on the running frame's operand stack, charging its growth.
    pub(crate) fn push_entry(&mut self, entry: Entry) -> Completion<()> {
        let frame = self.top_frame()?;
        if frame.stack.len() == frame.stack.capacity() {
            let capacity = frame.stack.capacity();
            let wanted = capacity.saturating_mul(2).max(8);
            let added = u64::try_from(wanted.saturating_sub(capacity)).unwrap_or(u64::MAX);
            let bytes = added.saturating_mul(entry_bytes());
            let reserved = frame
                .stack
                .try_reserve_exact(wanted.saturating_sub(frame.stack.len()))
                .is_ok();
            if !reserved || self.heap.accounting.charge(bytes).is_err() {
                return Err(self.terminate(Termination::MemoryBudget));
            }
            let frame = self.top_frame()?;
            frame.charged = frame.charged.saturating_add(bytes);
            frame.stack.push(entry);
            return Ok(());
        }
        frame.stack.push(entry);
        Ok(())
    }

    /// Push a value on the running frame's operand stack.
    pub(crate) fn push_value(&mut self, value: Value) -> Completion<()> {
        self.push_entry(Entry::Value(value))
    }

    /// Pop the top entry of the running frame's operand stack.
    pub(crate) fn pop_entry(&mut self) -> Completion<Entry> {
        match self.top_frame()?.stack.pop() {
            Some(entry) => Ok(entry),
            None => Err(self.fault()),
        }
    }

    /// Pop the top value of the running frame's operand stack.
    pub(crate) fn pop_value(&mut self) -> Completion<Value> {
        match self.pop_entry()? {
            Entry::Value(value) => Ok(value),
            _ => Err(self.fault()),
        }
    }

    /// The value `depth` entries below the top of the operand stack (0 is the top), left there.
    pub(crate) fn peek_value(&mut self, depth: usize) -> Completion<Value> {
        let frame = self.top_frame()?;
        let found = frame
            .stack
            .len()
            .checked_sub(depth.saturating_add(1))
            .and_then(|index| frame.stack.get(index));
        match found {
            Some(Entry::Value(value)) => Ok(*value),
            _ => Err(self.fault()),
        }
    }

    /// Drop `count` entries from the top of the operand stack.
    pub(crate) fn drop_entries(&mut self, count: usize) -> Completion<()> {
        let length = self.top_frame()?.stack.len();
        match length.checked_sub(count) {
            Some(len) => self.truncate_stack(len),
            None => Err(self.fault()),
        }
    }

    /// Cut the running frame's operand stack back to its first `len` entries: how `Pop`,
    /// [`Agent::drop_entries`] and the `items` of a handler drop entries without their frame
    /// (an entry [`Agent::pop_entry`] takes is one an instruction uses, never an enumerator or
    /// an Iterator Record in the bytecode). The key list an enumerator among the removed
    /// entries holds, and the box of an Iterator Record among them, leave the frame's `charged`
    /// and are credited, so a `for`-`in` or `for`-`of` loop that ends, normally, by a jump out
    /// of it or by a throw caught outside it, credits them at once.
    pub(crate) fn truncate_stack(&mut self, len: usize) -> Completion<()> {
        let frame = self.top_frame()?;
        let Some(removed) = frame.stack.get(len..) else {
            return Err(self.fault());
        };
        let held = removed.iter().fold(0u64, |held, entry| match entry {
            Entry::Enumerator(enumerator) => held.saturating_add(enumerator.bytes),
            Entry::Iterator(_) => held.saturating_add(for_of::record_bytes()),
            _ => held,
        });
        frame.stack.truncate(len);
        frame.charged = frame.charged.saturating_sub(held);
        self.heap.accounting.credit(held);
        Ok(())
    }

    /// The `count` values on top of the operand stack, bottom first, left there.
    pub(crate) fn peek_values(&mut self, count: usize) -> Completion<Vec<Value>> {
        let frame = self.top_frame()?;
        let start = frame.stack.len().checked_sub(count);
        let values: Option<Vec<Value>> = start.and_then(|start| {
            frame.stack.get(start..).map(|entries| {
                entries
                    .iter()
                    .filter_map(|entry| match entry {
                        Entry::Value(value) => Some(*value),
                        _ => None,
                    })
                    .collect()
            })
        });
        match values {
            Some(values) if values.len() == count => Ok(values),
            _ => Err(self.fault()),
        }
    }

    /// The object `value` holds, which the bytecode guarantees.
    pub(crate) fn expect_object(&mut self, value: Value) -> Completion<ObjectRef> {
        match value {
            Value::Object(object) => Ok(object),
            _ => Err(self.fault()),
        }
    }
}
