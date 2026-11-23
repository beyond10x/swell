//! The budget of `swell-regexp` over the meter of a parse (`docs/design/engine.md`, decision 7,
//! "How `swell-regexp` charges its work", item 2): the early errors check the pattern of each
//! regular expression literal with ParsePattern (`#sec-parsepattern`), and the crate charges
//! its tree through [`PatternBudget`].
//!
//! - A charge and a credit go to the [`Meter`] of the parse, so the tree of a pattern counts
//!   against the same ledger as the script's own tree, and a refused charge stops the parse as
//!   every other refused charge does.
//! - A poll reads the interrupt through the meter.
//! - A grant of steps is answered in full and counted nowhere, and a return is ignored: a parse
//!   spends no step (decision 3, "The step budget"), and ParsePattern asks for none.

use swell_regexp::{Budget, MAX_GRANT, Stop as PatternStop};

use crate::ledger::{Meter, Stop};

/// The [`Budget`] of ParsePattern for a literal: the meter of the parse it is checked in.
pub struct PatternBudget<'a, 'm> {
    meter: &'a Meter<'m>,
}

impl<'a, 'm> PatternBudget<'a, 'm> {
    /// The budget over `meter`, the meter of the parse.
    pub fn new(meter: &'a Meter<'m>) -> PatternBudget<'a, 'm> {
        PatternBudget { meter }
    }
}

/// The stop of the crate for a stop of the meter.
fn pattern_stop(stop: Stop) -> PatternStop {
    match stop {
        Stop::OverBudget => PatternStop::MemoryBudget,
        Stop::Interrupted => PatternStop::Interrupted,
    }
}

impl Budget for PatternBudget<'_, '_> {
    fn charge(&mut self, bytes: usize) -> Result<(), PatternStop> {
        self.meter.charge(bytes).map_err(pattern_stop)
    }

    fn credit(&mut self, bytes: usize) {
        self.meter.credit(bytes);
    }

    /// `asked` in full, between 1 and [`MAX_GRANT`], spent from no counter; refused only once
    /// the parse has stopped, with its stop. The interrupt is read by the polls.
    fn grant(&mut self, asked: u32) -> Result<u32, PatternStop> {
        self.meter.check().map_err(pattern_stop)?;
        Ok(asked.clamp(1, MAX_GRANT))
    }

    fn refund(&mut self, _steps: u32) {}

    fn poll(&mut self) -> Result<(), PatternStop> {
        self.meter.poll().map_err(pattern_stop)
    }
}
