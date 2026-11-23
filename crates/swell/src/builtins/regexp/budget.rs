//! The budget of `swell-regexp` over the agent (`docs/design/engine.md`, decision 7, "How
//! `swell-regexp` charges its work", item 2), written by `story:regexp-objects-exec-and-literals`.
//!
//! [`AgentBudget`] borrows the agent's accounting, its step counter with the bound
//! `steps_per_entry` and its interrupt flag, apart from the arenas, so that the matcher reads its
//! subject in place in the arena of Strings while it charges. Bytes go to the accounting before
//! each growth; a grant of steps never gives more than the steps left, and refuses when none is
//! left, so the refused grant is the step that would pass the budget, as for `Agent::step`; a
//! return lowers the counter by the steps granted and not run. Every grant and every poll reads
//! the interrupt flag and clears it when it is set.
//!
//! A stop of the crate ends the entry ([`Agent::regexp_stopped`]): `Terminated(MemoryBudget)`,
//! `Terminated(StepBudget)` or `Terminated(Interrupted)`, where no `catch` or `finally` runs
//! (decision 3). The crate credits everything it charged before it answers a stop.

use swell_regexp::{Budget, Stop};

use crate::heap::accounting::Accounting;
use crate::host::agent::Agent;
use crate::host::outcome::Abrupt;
use crate::host::{Host, InterruptHandle, Termination};

/// The budget of one parse, compile step or search of `swell-regexp`, over the agent.
pub(crate) struct AgentBudget<'a> {
    accounting: &'a mut Accounting,
    steps: &'a mut u64,
    limit: u64,
    interrupt: &'a InterruptHandle,
}

impl<'a> AgentBudget<'a> {
    /// The budget over `accounting`, the step counter `steps` bounded by `limit`, and
    /// `interrupt`.
    pub(crate) fn new(
        accounting: &'a mut Accounting,
        steps: &'a mut u64,
        limit: u64,
        interrupt: &'a InterruptHandle,
    ) -> AgentBudget<'a> {
        AgentBudget {
            accounting,
            steps,
            limit,
            interrupt,
        }
    }

    /// `Err(Stop::Interrupted)` when the flag is set; the flag is cleared.
    fn read_interrupt(&mut self) -> Result<(), Stop> {
        if self.interrupt.is_set() {
            self.interrupt.clear();
            return Err(Stop::Interrupted);
        }
        Ok(())
    }
}

impl Budget for AgentBudget<'_> {
    fn charge(&mut self, bytes: usize) -> Result<(), Stop> {
        let bytes = u64::try_from(bytes).unwrap_or(u64::MAX);
        self.accounting
            .charge(bytes)
            .map_err(|_| Stop::MemoryBudget)
    }

    fn credit(&mut self, bytes: usize) {
        self.accounting
            .credit(u64::try_from(bytes).unwrap_or(u64::MAX));
    }

    fn grant(&mut self, asked: u32) -> Result<u32, Stop> {
        self.read_interrupt()?;
        let left = self.limit.saturating_sub(*self.steps);
        if left == 0 {
            return Err(Stop::StepBudget);
        }
        let given = u64::from(asked.max(1)).min(left);
        *self.steps = self.steps.saturating_add(given);
        Ok(u32::try_from(given).unwrap_or(1))
    }

    fn refund(&mut self, steps: u32) {
        *self.steps = self.steps.saturating_sub(u64::from(steps));
    }

    fn poll(&mut self) -> Result<(), Stop> {
        self.read_interrupt()
    }
}

impl<H: Host> Agent<H> {
    /// The budget of `swell-regexp` over this agent, for a parse or a compile step, which read
    /// no arena.
    pub(crate) fn regexp_budget(&mut self) -> AgentBudget<'_> {
        AgentBudget::new(
            &mut self.heap.accounting,
            &mut self.steps,
            self.limits.steps_per_entry,
            &self.interrupt,
        )
    }

    /// End the entry on the stop of the crate.
    pub(crate) fn regexp_stopped(&mut self, stop: Stop) -> Abrupt {
        self.terminate(match stop {
            Stop::MemoryBudget => Termination::MemoryBudget,
            Stop::StepBudget => Termination::StepBudget,
            Stop::Interrupted => Termination::Interrupted,
        })
    }
}
