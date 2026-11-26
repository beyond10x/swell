//! Suspended frames (`docs/design/engine.md`, decision 3, "Suspended frames", items 1 to 6):
//! the move of the running frame off the frame stack into the object that holds it, and back.
//! Written by `story:generator-functions-and-suspended-frames` for the generators of slice 7 and
//! for the `async` functions of slice 10, which use it as it is: nothing here names a
//! generator.
//!
//! - What moves (item 1): the whole [`Frame`], by value, in a box, with its `charged`; the
//!   handler table stays with the code it belongs to.
//! - Where it lives (item 2): in its holder's kind, a [`SuspendedFrame`], traced at the
//!   holder's kind site with [`SuspendedFrame::trace`] and never as a root.
//! - Its charge (item 3): beside the box a [`CreditGuard`] of the agent's credit queue holds
//!   the frame's `charged`; dropping the holder's record queues it, and the agent credits the
//!   queue after the sweep of the same collection. Taking the frame out takes the charge with
//!   it and leaves nothing to queue, so the frame is credited once: by `pop_frame` or
//!   `drop_frames` once it runs again, or after the sweep that frees its holder.
//! - Suspension (items 5 and 6): [`Agent::suspend_frame`] moves the running frame into its
//!   holder without crediting it; the instruction then answers `Flow::Suspend`, and the run
//!   loop hands the value over ([`Agent::hand_over`]) as a return hands its value over.
//! - Resumption (items 4, 5 and 6): [`Agent::resume_frame`] checks `frame_depth`, spends the
//!   step of a call, checks `reentry_depth` and the interrupt, and only then has the holder
//!   give the frame up, pushes it with the received value and its tag; [`Agent::run_resumed`]
//!   runs it in a run loop of its own, one re-entry. A caller with a run loop of its own (the
//!   instruction of `yield*`) calls the first and goes on in its own loop.

use alloc::boxed::Box;
use alloc::rc::Rc;
use core::cell::Cell;

use super::frame::{Entry, Frame};
use crate::heap::arena::Handle;
use crate::heap::collect::Mark;
use crate::host::Host;
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::value::Value;

/// The charge of a suspended frame, in the form of `Grown` (`object/mod.rs`): its drop adds
/// the bytes to the agent's credit queue, which the agent credits after the sweep
/// (`Agent::collect_now`).
pub(crate) struct CreditGuard {
    bytes: u64,
    queue: Rc<Cell<u64>>,
}

impl Drop for CreditGuard {
    fn drop(&mut self) {
        self.queue.set(self.queue.get().saturating_add(self.bytes));
    }
}

/// A frame moved off the frame stack, in a box, with the guard of its charge beside it: the
/// slot a holder's kind has for one suspended frame.
pub(crate) struct SuspendedFrame {
    frame: Box<Frame>,
    guard: CreditGuard,
}

impl SuspendedFrame {
    /// What the frame holds, traced at its holder's kind site (item 2).
    pub(crate) fn trace(&self, mark: &mut Mark) {
        self.frame.trace(mark);
    }

    /// The frame, with its charge: the guard is left holding nothing.
    fn into_frame(mut self) -> Box<Frame> {
        self.guard.bytes = 0;
        self.frame
    }
}

/// What a resumption restores when its frame stops running: the current realm of the code
/// that resumed it.
pub(crate) struct Resumption {
    saved_realm: Option<Handle>,
}

impl<H: Host> Agent<H> {
    /// Suspend the running frame (items 3 and 5): move it off the frame stack, with its charge
    /// and nothing credited, and give it to `store`, which puts it into its holder. The caller
    /// sets the holder's state first and hands the value over after, by answering
    /// `Flow::Suspend`. A frame `store` refuses is dropped with its guard, which queues its
    /// charge.
    pub(crate) fn suspend_frame(
        &mut self,
        store: impl FnOnce(&mut Self, SuspendedFrame) -> Completion<()>,
    ) -> Completion<()> {
        let Some(frame) = self.interp.frames.pop() else {
            return Err(self.fault());
        };
        let guard = CreditGuard {
            bytes: frame.charged,
            queue: Rc::clone(&self.credit),
        };
        let suspended = SuspendedFrame {
            frame: Box::new(frame),
            guard,
        };
        store(self, suspended)
    }

    /// Drop a suspended frame that will never run again, its charge credited at once (item 3:
    /// an abrupt completion given to a generator before its start completes it).
    pub(crate) fn release_suspended_frame(&mut self, suspended: SuspendedFrame) {
        let frame = suspended.into_frame();
        self.heap.accounting.credit(frame.charged);
    }

    /// Hand the value of a suspension over (item 6), as a return does: the answer of the run
    /// loop when the suspended frame was its base frame (`Some`); otherwise the value goes on
    /// the operand stack of the frame below, whose realm becomes the current realm, and the
    /// interrupt is checked.
    #[inline(never)]
    pub(super) fn hand_over(&mut self, base: usize, value: Value) -> Completion<Option<Value>> {
        if self.interp.frames.len() <= base {
            return Ok(Some(value));
        }
        let realm = self.top_frame()?.realm;
        self.set_realm_slot(Some(realm));
        self.push_value(value)?;
        self.check_interrupt()?;
        Ok(None)
    }

    /// The checks and the move of a resumption, in the order of item 5: `frame_depth` (a
    /// `RangeError`, item 4), the step of a call, `reentry_depth` and the interrupt; each
    /// refusal leaves the holder as it was. Then `take` has the holder give its frame up (and
    /// set its state), the frame is pushed, and the received `value` and its `tag` go on its
    /// operand stack: a growth refused there ends the run with the frame dropped and credited.
    /// The caller then runs the frame: [`Agent::run_resumed`] from Rust, or its own run loop.
    #[inline(never)]
    pub(crate) fn resume_frame(
        &mut self,
        take: impl FnOnce(&mut Self) -> Completion<SuspendedFrame>,
        value: Value,
        tag: u32,
    ) -> Completion<Resumption> {
        self.check_frame_depth()?;
        self.step()?;
        // No activation is pushed (`None`), so there is nothing for `leave_code` to undo.
        self.enter_code(None)?;
        let saved_realm = self.realm_slot();
        let frame = take(self)?.into_frame();
        let base = self.interp.frames.len();
        self.push_frame(*frame);
        let pushed = match self.push_value(value) {
            Ok(()) => self.push_entry(Entry::Tag(tag)),
            Err(abrupt) => Err(abrupt),
        };
        if let Err(abrupt) = pushed {
            self.drop_frames(base);
            self.set_realm_slot(saved_realm);
            return Err(abrupt);
        }
        Ok(Resumption { saved_realm })
    }

    /// Run the frame [`Agent::resume_frame`] pushed in a run loop of its own (item 6,
    /// "Re-entry"), until it suspends, returns or throws: its value, a throw, or a
    /// termination. The running loop and the current realm are restored after.
    pub(crate) fn run_resumed(&mut self, resumption: Resumption) -> Completion<Value> {
        let saved_running = self.interp.running.replace(self.activations.len());
        let base = self.interp.frames.len().saturating_sub(1);
        let result = self.run(base);
        self.interp.running = saved_running;
        self.set_realm_slot(resumption.saved_realm);
        result
    }
}
