//! The memory budget (`docs/design/engine.md`, decision 4, "Memory budget";
//! `docs/design/host-interface.md`, decision 2, `memory_bytes`): every allocation and every growth
//! of the heap is charged through [`Accounting::charge`], the one accounting function, and a
//! sweep credits what it frees. Over the budget the allocation is refused and nothing is
//! charged; the caller ends the run in `Terminated(MemoryBudget)`. The lib tests read the
//! high-water mark of the bytes charged and the length of the largest list of property keys
//! an [[OwnPropertyKeys]] allocated (`story:own-keys-charged-before-the-list-exists`).

/// An allocation would take the heap over its budget.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct OverBudget;

/// The bytes the heap is charged, its budget, and the bytes charged since the last collection
/// (the collector's trigger).
pub(crate) struct Accounting {
    used: u64,
    limit: u64,
    since_collection: u64,
    /// The most `used` has read since the agent was made or the last reset, in the lib tests.
    #[cfg(test)]
    high_water: u64,
    /// The length of the largest list of keys an [[OwnPropertyKeys]] allocated, in the lib
    /// tests.
    #[cfg(test)]
    largest_key_list: usize,
}

impl Accounting {
    /// Nothing charged, with the budget `limit` (0 is a budget of zero, never "no limit").
    pub(crate) fn new(limit: u64) -> Accounting {
        Accounting {
            used: 0,
            limit,
            since_collection: 0,
            #[cfg(test)]
            high_water: 0,
            #[cfg(test)]
            largest_key_list: 0,
        }
    }

    /// The one accounting function: charge `bytes`, or refuse and charge nothing when the heap
    /// would then hold more than the budget.
    pub(crate) fn charge(&mut self, bytes: u64) -> Result<(), OverBudget> {
        let used = self.used.checked_add(bytes).ok_or(OverBudget)?;
        if used > self.limit {
            return Err(OverBudget);
        }
        self.used = used;
        self.since_collection = self.since_collection.saturating_add(bytes);
        #[cfg(test)]
        {
            self.high_water = self.high_water.max(used);
        }
        Ok(())
    }

    /// The most the heap was charged since the agent was made or the last
    /// [`Accounting::reset_high_water`].
    #[cfg(test)]
    pub(crate) fn high_water(&self) -> u64 {
        self.high_water
    }

    /// Count the high-water mark again from the bytes charged now.
    #[cfg(test)]
    pub(crate) fn reset_high_water(&mut self) {
        self.high_water = self.used;
    }

    /// An [[OwnPropertyKeys]] allocated a list of `length` keys.
    #[cfg(test)]
    pub(crate) fn note_key_list(&mut self, length: usize) {
        self.largest_key_list = self.largest_key_list.max(length);
    }

    /// The length of the largest list of keys an [[OwnPropertyKeys]] allocated.
    #[cfg(test)]
    pub(crate) fn largest_key_list(&self) -> usize {
        self.largest_key_list
    }

    /// Whether `bytes` more would fit the budget, charging nothing: asked before a large copy,
    /// so that an allocation over the budget is refused before memory is taken for it.
    pub(crate) fn check(&self, bytes: u64) -> Result<(), OverBudget> {
        match self.used.checked_add(bytes) {
            Some(used) if used <= self.limit => Ok(()),
            _ => Err(OverBudget),
        }
    }

    /// Give back `bytes` a sweep freed, or an allocation that charged and then failed.
    pub(crate) fn credit(&mut self, bytes: u64) {
        self.used = self.used.saturating_sub(bytes);
    }

    /// A test sets the budget to reach it with a small subject.
    #[cfg(test)]
    pub(crate) fn set_limit(&mut self, limit: u64) {
        self.limit = limit;
    }

    /// The bytes charged now.
    pub(crate) fn used(&self) -> u64 {
        self.used
    }

    /// The bytes charged since the last collection.
    pub(crate) fn since_collection(&self) -> u64 {
        self.since_collection
    }

    /// A collection ran: count the trigger from zero.
    pub(crate) fn collected(&mut self) {
        self.since_collection = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::{Accounting, OverBudget};

    #[test]
    fn the_budget_is_a_bound_and_a_refusal_charges_nothing() {
        let mut accounting = Accounting::new(100);
        assert_eq!(accounting.charge(60), Ok(()));
        assert_eq!(accounting.charge(41), Err(OverBudget));
        assert_eq!(accounting.used(), 60);
        assert_eq!(accounting.charge(40), Ok(()));
        assert_eq!(accounting.used(), 100);
        assert_eq!(accounting.charge(u64::MAX), Err(OverBudget));
        accounting.credit(30);
        assert_eq!(accounting.used(), 70);
        assert_eq!(accounting.since_collection(), 100);
        accounting.collected();
        assert_eq!(accounting.since_collection(), 0);

        let mut zero = Accounting::new(0);
        assert_eq!(zero.charge(0), Ok(()));
        assert_eq!(zero.charge(1), Err(OverBudget));
    }
}
