//! The step budget and the depth bounds (`docs/design/engine.md`, decision 3;
//! `docs/design/host-interface.md`, decision 2), and the interrupt checks that go with them
//! (decision 12). Filled by `story:interpreter-and-script-entry-points`.
//!
//! - One step per instruction and one per call into an ECMAScript function, spent by
//!   [`Agent::step`]: the step that would pass `steps_per_entry` is not spent, and the run ends
//!   in `Terminated(StepBudget)`, so `last_entry_steps()` never reads more than the budget. The
//!   steps builtins charge go through `Agent::charge_steps` (`host/agent.rs`) with the same
//!   rule.
//! - The interrupt flag is read by [`Agent::check_interrupt`] at every `LoopJump`, every call
//!   (the frame a call pushes, and right after a builtin or host function returns), every
//!   return and every re-entry (`interp/mod.rs`), and by `Agent::charge_steps`; a set flag ends
//!   the run in `Terminated(Interrupted)` and is cleared.
//! - `frame_depth` bounds the frames on the frame stack, a script's own frame included
//!   ([`Agent::check_frame_depth`]): the push that would make one more throws a `RangeError` in
//!   the current realm, which the script can catch.

use crate::host::agent::Agent;
use crate::host::outcome::{Completion, Termination};
use crate::host::{ErrorKind, Host};

impl<H: Host> Agent<H> {
    /// Spend one step of the running outermost entry.
    pub(crate) fn step(&mut self) -> Completion<()> {
        if self.steps >= self.limits.steps_per_entry {
            return Err(self.terminate(Termination::StepBudget));
        }
        self.steps = self.steps.saturating_add(1);
        Ok(())
    }

    /// The interrupt check: a set flag ends the run and is cleared.
    pub(crate) fn check_interrupt(&mut self) -> Completion<()> {
        if self.interrupt.is_set() {
            self.interrupt.clear();
            return Err(self.terminate(Termination::Interrupted));
        }
        Ok(())
    }

    /// The `frame_depth` check before a frame is pushed.
    pub(crate) fn check_frame_depth(&mut self) -> Completion<()> {
        let depth = u32::try_from(self.interp.frames.len()).unwrap_or(u32::MAX);
        if depth >= self.limits.frame_depth {
            return Err(self.throw_error(ErrorKind::RangeError, "frame_depth exceeded"));
        }
        Ok(())
    }
}
