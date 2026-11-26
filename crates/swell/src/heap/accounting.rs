//! The memory budget (`docs/design/engine.md`, decision 4, "Memory budget";
//! `docs/design/host-interface.md`, decision 2, `memory_bytes`): every allocation and every growth
//! of the heap is charged through [`Accounting::charge`], the one accounting function, and a
//! sweep credits what it frees. Over the budget the allocation is refused and nothing is
//! charged; the caller ends the run in `Terminated(MemoryBudget)`. The lib tests read the
//! high-water mark of the bytes charged and the length of the largest list of property keys
//! an [[OwnPropertyKeys]] allocated (`story:own-keys-charged-before-the-list-exists`).
//!
//! A buffer of the engine outside its arenas whose size a script controls is charged by its
//! capacity before it is made or grows, and credited when it drops (`docs/design/engine.md`,
//! "Memory budget"; `story:copies-held-across-a-call-into-script-are-charged`): it is held as a
//! [`ChargedVec`], grown only by [`Agent::reserve_vec`], or as a [`Charge`], whose drop gives
//! its bytes back at once through a cell of the accounting that every reading subtracts.

use alloc::rc::Rc;
use alloc::vec::Vec;
use core::cell::Cell;
use core::ops::{Deref, DerefMut};

use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::host::{Host, Termination};

/// An allocation would take the heap over its budget.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct OverBudget;

/// A [`ChargedVec`] has no room left for what was pushed: its owner did not reserve it with
/// [`Agent::reserve_vec`], a broken invariant of the engine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Full;

/// Bytes held charged to the memory budget, given back when the `Charge` drops: the drop adds
/// them to a cell of the accounting, which every reading subtracts at once and the next charge
/// folds in. Neither `Clone` nor `Copy`. An empty one holds no cell until it first grows.
#[derive(Default)]
pub(crate) struct Charge {
    bytes: u64,
    released: Option<Rc<Cell<u64>>>,
}

impl Charge {
    /// The bytes held.
    pub(crate) fn bytes(&self) -> u64 {
        self.bytes
    }

    /// Charge `bytes` more into this charge; over the budget nothing is charged.
    pub(crate) fn grow(
        &mut self,
        accounting: &mut Accounting,
        bytes: u64,
    ) -> Result<(), OverBudget> {
        accounting.charge_held(bytes)?;
        self.bytes = self.bytes.saturating_add(bytes);
        if self.released.is_none() {
            self.released = Some(Rc::clone(&accounting.released));
        }
        Ok(())
    }

    /// Give back `bytes` of this charge at once, at most what it holds.
    pub(crate) fn release(&mut self, bytes: u64) {
        let bytes = bytes.min(self.bytes);
        self.bytes -= bytes;
        if let Some(released) = &self.released {
            released.set(released.get().saturating_add(bytes));
        }
    }
}

impl Drop for Charge {
    /// Add the bytes to the accounting's cell, with a saturating add: no panic, no allocation.
    fn drop(&mut self) {
        let bytes = self.bytes;
        self.release(bytes);
    }
}

/// A list charged to the memory budget by its capacity before it is made or grows, and
/// credited when it drops. It grows only through [`Agent::reserve_vec`] (or
/// [`Agent::new_charged_vec`]); its pushes take room already reserved.
pub(crate) struct ChargedVec<T> {
    list: Vec<T>,
    charge: Charge,
    /// The counting build: what this list adds to the tallies of `copies`.
    #[cfg(test)]
    tally: crate::object::string::copies::Tally,
}

impl<T> Default for ChargedVec<T> {
    fn default() -> ChargedVec<T> {
        ChargedVec::new()
    }
}

impl<T> Deref for ChargedVec<T> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        &self.list
    }
}

impl<T> DerefMut for ChargedVec<T> {
    fn deref_mut(&mut self) -> &mut [T] {
        &mut self.list
    }
}

impl<'a, T> IntoIterator for &'a ChargedVec<T> {
    type Item = &'a T;
    type IntoIter = core::slice::Iter<'a, T>;

    fn into_iter(self) -> core::slice::Iter<'a, T> {
        self.list.iter()
    }
}

impl<T> ChargedVec<T> {
    /// An empty list with no room, nothing charged.
    pub(crate) fn new() -> ChargedVec<T> {
        ChargedVec {
            list: Vec::new(),
            charge: Charge::default(),
            #[cfg(test)]
            tally: crate::object::string::copies::Tally::default(),
        }
    }

    /// The elements it can hold without growing.
    #[cfg(test)]
    pub(crate) fn capacity(&self) -> usize {
        self.list.capacity()
    }

    /// Append `item` in room reserved before; `Err(Full)` when there is none.
    pub(crate) fn push(&mut self, item: T) -> Result<(), Full> {
        if self.list.len() == self.list.capacity() {
            return Err(Full);
        }
        self.list.push(item);
        Ok(())
    }

    /// Remove and answer the last element.
    pub(crate) fn pop(&mut self) -> Option<T> {
        self.list.pop()
    }

    /// Keep the first `length` elements; the room and the charge stay.
    pub(crate) fn truncate(&mut self, length: usize) {
        self.list.truncate(length);
    }

    /// Append `items` in room reserved before; `Err(Full)`, with nothing appended, when there
    /// is not room for all of them.
    pub(crate) fn extend_from_slice(&mut self, items: &[T]) -> Result<(), Full>
    where
        T: Clone,
    {
        if self.list.capacity() - self.list.len() < items.len() {
            return Err(Full);
        }
        self.list.extend_from_slice(items);
        Ok(())
    }

    /// Append the elements `range` of the list itself in room reserved before.
    pub(crate) fn extend_from_within(&mut self, range: core::ops::Range<usize>) -> Result<(), Full>
    where
        T: Clone,
    {
        let fits = range.start <= range.end
            && range.end <= self.list.len()
            && self.list.capacity() - self.list.len() >= range.end - range.start;
        if !fits {
            return Err(Full);
        }
        self.list.extend_from_within(range);
        Ok(())
    }

    /// Append the elements of `items` in room reserved before for `count` of them; `Err(Full)`
    /// when there is not, or when `items` holds more.
    pub(crate) fn extend_counted(
        &mut self,
        count: usize,
        items: impl IntoIterator<Item = T>,
    ) -> Result<(), Full> {
        if self.list.capacity() - self.list.len() < count {
            return Err(Full);
        }
        let end = self.list.len() + count;
        for item in items {
            if self.list.len() == end {
                return Err(Full);
            }
            self.list.push(item);
        }
        Ok(())
    }

    /// The elements as a plain list; the charge drops with this call, so it is credited at
    /// once. For an owner that hands the elements to a charged record right away.
    pub(crate) fn into_vec(mut self) -> Vec<T> {
        core::mem::take(&mut self.list)
    }

    /// The counting build: report the capacity and the charge to the tallies of `copies`.
    #[cfg(test)]
    fn note(&mut self) {
        let size = u64::try_from(size_of::<T>()).unwrap_or(u64::MAX);
        let capacity = u64::try_from(self.list.capacity()).unwrap_or(u64::MAX);
        self.tally
            .set(capacity.saturating_mul(size), self.charge.bytes());
    }

    #[cfg(not(test))]
    fn note(&mut self) {}
}

/// The bytes the heap is charged, its budget, and the bytes charged since the last collection
/// (the collector's trigger).
pub(crate) struct Accounting {
    used: u64,
    limit: u64,
    since_collection: u64,
    /// The bytes of dropped or released [`Charge`]s not yet folded into `used`: every reading
    /// subtracts them, and the next charge or credit folds them.
    released: Rc<Cell<u64>>,
    /// The most `used` has read since the agent was made or the last reset, in the lib tests.
    #[cfg(test)]
    high_water: u64,
    /// The length of the largest list of keys an [[OwnPropertyKeys]] allocated, in the lib
    /// tests.
    #[cfg(test)]
    largest_key_list: usize,
    /// The polls of the frontend's ledger left before it sets the interrupt flag itself, in the
    /// lib tests (`script.rs`, `AgentLedger::poll`): `Some(0)` once it has.
    #[cfg(test)]
    poll_countdown: Option<u64>,
}

impl Accounting {
    /// Nothing charged, with the budget `limit` (0 is a budget of zero, never "no limit").
    pub(crate) fn new(limit: u64) -> Accounting {
        Accounting {
            used: 0,
            limit,
            since_collection: 0,
            released: Rc::new(Cell::new(0)),
            #[cfg(test)]
            high_water: 0,
            #[cfg(test)]
            largest_key_list: 0,
            #[cfg(test)]
            poll_countdown: None,
        }
    }

    /// The frontend's ledger sets the interrupt flag at its `polls`-th poll from now.
    #[cfg(test)]
    pub(crate) fn set_poll_countdown(&mut self, polls: u64) {
        self.poll_countdown = Some(polls);
    }

    /// The polls left before the ledger sets the flag; `Some(0)` once it has.
    #[cfg(test)]
    pub(crate) fn poll_countdown(&self) -> Option<u64> {
        self.poll_countdown
    }

    /// One poll of the frontend's ledger: whether the countdown reaches 0 with it, so that the
    /// ledger sets the flag before it reads it.
    #[cfg(test)]
    pub(crate) fn count_down_poll(&mut self) -> bool {
        match self.poll_countdown {
            Some(left) if left > 0 => {
                self.poll_countdown = Some(left - 1);
                left == 1
            }
            _ => false,
        }
    }

    /// The one accounting function: charge `bytes`, or refuse and charge nothing when the heap
    /// would then hold more than the budget.
    pub(crate) fn charge(&mut self, bytes: u64) -> Result<(), OverBudget> {
        self.charge_held(bytes)?;
        self.since_collection = self.since_collection.saturating_add(bytes);
        Ok(())
    }

    /// Fold the bytes of dropped and released charges into `used`.
    fn fold(&mut self) {
        let released = self.released.replace(0);
        self.used = self.used.saturating_sub(released);
    }

    /// Charge `bytes` that a [`Charge`] holds: as [`Accounting::charge`], but not counted
    /// toward the collector's trigger, since a collection frees none of them.
    fn charge_held(&mut self, bytes: u64) -> Result<(), OverBudget> {
        self.fold();
        let used = self.used.checked_add(bytes).ok_or(OverBudget)?;
        if used > self.limit {
            return Err(OverBudget);
        }
        self.used = used;
        #[cfg(test)]
        {
            self.high_water = self.high_water.max(used);
        }
        Ok(())
    }

    /// Charge `bytes` held by the [`Charge`] this answers, whose drop gives them back; over
    /// the budget nothing is charged.
    pub(crate) fn hold(&mut self, bytes: u64) -> Result<Charge, OverBudget> {
        let mut charge = Charge::default();
        charge.grow(self, bytes)?;
        Ok(charge)
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
        self.high_water = self.used();
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
        match self.used().checked_add(bytes) {
            Some(used) if used <= self.limit => Ok(()),
            _ => Err(OverBudget),
        }
    }

    /// Give back `bytes` a sweep freed, or an allocation that charged and then failed.
    pub(crate) fn credit(&mut self, bytes: u64) {
        self.fold();
        self.used = self.used.saturating_sub(bytes);
    }

    /// A test sets the budget to reach it with a small subject.
    #[cfg(test)]
    pub(crate) fn set_limit(&mut self, limit: u64) {
        self.limit = limit;
    }

    /// The bytes charged now.
    pub(crate) fn used(&self) -> u64 {
        self.used.saturating_sub(self.released.get())
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

/// The bytes of `count` elements of `T`.
fn list_bytes<T>(count: usize) -> u64 {
    u64::try_from(count)
        .unwrap_or(u64::MAX)
        .saturating_mul(u64::try_from(size_of::<T>()).unwrap_or(u64::MAX))
}

impl<H: Host> Agent<H> {
    /// Room in `list` for `more` elements beyond its length: when it has none, its capacity
    /// grows to twice what it was, at least 16 and at least what is needed, that growth
    /// charged to the list's [`Charge`] before it is reserved; when the budget refuses the
    /// doubled growth, the exact need is charged instead, and only when that is refused too
    /// does the run end in `Terminated(MemoryBudget)`, with nothing grown.
    pub(crate) fn reserve_vec<T>(
        &mut self,
        list: &mut ChargedVec<T>,
        more: usize,
    ) -> Completion<()> {
        self.reserve_vec_within(list, more, usize::MAX)
    }

    /// [`Agent::reserve_vec`] with the doubled capacity capped at `most` elements (never below
    /// the need).
    pub(crate) fn reserve_vec_within<T>(
        &mut self,
        list: &mut ChargedVec<T>,
        more: usize,
        most: usize,
    ) -> Completion<()> {
        let length = list.list.len();
        let needed = length.saturating_add(more);
        let capacity = list.list.capacity();
        if needed <= capacity {
            return Ok(());
        }
        let doubled = capacity.saturating_mul(2).max(16).min(most).max(needed);
        let held = list.charge.bytes();
        let mut wanted = doubled;
        let mut growth = list_bytes::<T>(wanted).saturating_sub(held);
        if list.charge.grow(&mut self.heap.accounting, growth).is_err() {
            wanted = needed;
            growth = list_bytes::<T>(wanted).saturating_sub(held);
            if list.charge.grow(&mut self.heap.accounting, growth).is_err() {
                return Err(self.terminate(Termination::MemoryBudget));
            }
        }
        #[expect(
            clippy::disallowed_methods,
            reason = "the room of a `ChargedVec`, charged by `Charge::grow` before it grows"
        )]
        if list.list.try_reserve_exact(wanted - length).is_err() {
            list.charge.release(growth);
            return Err(self.terminate(Termination::MemoryBudget));
        }
        list.note();
        Ok(())
    }

    /// Append `item` to `list`, its room grown first by [`Agent::reserve_vec`].
    pub(crate) fn push_vec<T>(&mut self, list: &mut ChargedVec<T>, item: T) -> Completion<()> {
        self.reserve_vec(list, 1)?;
        list.push(item).map_err(|Full| self.fault())
    }

    /// A list with room for exactly `count` elements, its bytes held before the room is
    /// reserved; over the budget the run ends in `Terminated(MemoryBudget)` and nothing stays
    /// charged.
    pub(crate) fn new_charged_vec<T>(&mut self, count: usize) -> Completion<ChargedVec<T>> {
        let Ok(charge) = self.heap.accounting.hold(list_bytes::<T>(count)) else {
            return Err(self.terminate(Termination::MemoryBudget));
        };
        let mut list = ChargedVec {
            list: Vec::new(),
            charge,
            #[cfg(test)]
            tally: crate::object::string::copies::Tally::default(),
        };
        #[expect(
            clippy::disallowed_methods,
            reason = "the room of a new `ChargedVec`, charged by `Accounting::hold` before it is reserved"
        )]
        if list.list.try_reserve_exact(count).is_err() {
            return Err(self.terminate(Termination::MemoryBudget));
        }
        list.note();
        Ok(list)
    }
}

#[cfg(test)]
mod tests {
    use super::{Accounting, ChargedVec, OverBudget};
    use crate::host::hooks::test_host::{agent_with, limits};
    use crate::value::Value;

    /// `docs/design/engine.md`, "Memory budget": a held charge raises `used()` at once and its
    /// drop gives the bytes back at once, before any fold, so `check` sees the credit; a release
    /// gives back part of it; a `ChargedVec` grows by doubling from 16 with each growth charged
    /// before it is made, and falls back to the exact need when the budget refuses the
    /// doubling; a refused hold charges nothing.
    #[test]
    fn a_charge_held_across_a_call_is_given_back_on_drop() {
        let mut agent = agent_with(limits());
        let accounting = &mut agent.heap.accounting;
        let before = accounting.used();
        let charge = accounting.hold(100).unwrap();
        assert_eq!(accounting.used(), before + 100);
        accounting.set_limit(before + 100);
        assert_eq!(accounting.check(1), Err(OverBudget));
        drop(charge);
        assert_eq!(accounting.used(), before);
        assert_eq!(accounting.check(100), Ok(()));
        accounting.set_limit(1 << 30);
        let mut charge = accounting.hold(100).unwrap();
        charge.release(40);
        assert_eq!(accounting.used(), before + 60);
        drop(charge);
        assert_eq!(accounting.used(), before);
        // A refused hold charges nothing.
        accounting.set_limit(before);
        assert!(accounting.hold(1).is_err());
        assert_eq!(accounting.used(), before);
        accounting.set_limit(1 << 30);

        let value = u64::try_from(size_of::<Value>()).unwrap();
        let mut list: ChargedVec<Value> = ChargedVec::new();
        for capacity in [16_usize, 32, 64] {
            let more = capacity - list.len();
            agent.reserve_vec(&mut list, 1).unwrap();
            assert_eq!(list.capacity(), capacity);
            assert_eq!(
                agent.heap.accounting.used(),
                before + u64::try_from(capacity).unwrap() * value
            );
            for _ in 0..more {
                list.push(Value::Undefined).unwrap();
            }
            assert!(list.push(Value::Undefined).is_err());
        }
        // Room for the need (one more element) but not for the doubled growth: the exact need.
        let used = agent.heap.accounting.used();
        agent.heap.accounting.set_limit(used + value);
        assert_eq!(agent.reserve_vec(&mut list, 1), Ok(()));
        assert_eq!(list.capacity(), 65);
        assert_eq!(agent.heap.accounting.used(), used + value);
        drop(list);
        assert_eq!(agent.heap.accounting.used(), before);
    }

    /// `docs/design/engine.md`, "Memory budget": the three buffers bounded by a limit and not
    /// charged have the sizes the design text names: 257 activations at `reentry_depth` 256,
    /// the frame stack's spare capacity at `frame_depth` 10,000, and the kept stack's growth by
    /// `keep` outside a room, at most `KEPT_OUTSIDE_A_ROOM` values per running builtin.
    #[test]
    fn the_buffers_bounded_by_a_limit_have_the_sizes_the_design_text_names() {
        /// The most values one running builtin keeps outside a room, read from the code.
        const KEPT_OUTSIDE_A_ROOM: usize = 10;
        let activations = 257 * size_of::<crate::object::builtin_function::Activation>();
        let frames = 10_000 * size_of::<crate::interp::frame::Frame>();
        let kept = 257 * KEPT_OUTSIDE_A_ROOM * size_of::<Value>();
        assert_eq!((activations, frames, kept), (22_616, 2_080_000, 41_120));
    }

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
