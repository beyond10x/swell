//! The budget `swell-regexp` charges its work through (`docs/design/engine.md`, decision 7, "How
//! `swell-regexp` charges its work", item 1). The crate names no type of `swell-syntax` or `swell`,
//! so its caller hands it a [`Budget`] of its own: `swell-syntax` over the meter of a parse,
//! `swell` over the agent's accounting, step counter and interrupt flag. [`Unlimited`] charges
//! nothing and never stops, for tests and tools.
//!
//! A refusal of the budget is a [`Stop`] of the crate. After a stop the crate credits everything
//! it holds and answers the stop, never an error of the pattern.
//!
//! - **By capacity.** A vector grows by the crate's own doubling, to at least 4 elements, with
//!   `try_reserve_exact`, as `Meter::grow` of `swell-syntax` does, so a charge does not depend on
//!   the growth policy of the standard library; a refusal of the allocator is a refused charge.
//! - **Steps.** The parser charges no step: RegExpInitialize charges one per code unit of the
//!   pattern before ParsePattern ("What one step of the matcher is"). Grants and returns are for
//!   the matcher.
//! - **Polls.** The parser reads the interrupt at least once per [`POLL_INTERVAL`] code units it
//!   reads and per [`POLL_INTERVAL`] nodes it makes.

use std::collections::BTreeMap;

/// Why the crate stopped: the budget refused, or the interrupt was read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stop {
    /// A charge was refused: the memory budget, or the allocator.
    MemoryBudget,
    /// A grant was refused: no step is left.
    StepBudget,
    /// The host's interrupt was read.
    Interrupted,
}

/// The most steps one grant asks for, and the most code units read or nodes made between two
/// polls.
pub const MAX_GRANT: u32 = 4096;

/// The most code units the parser reads, and the most nodes it makes, between two polls.
pub const POLL_INTERVAL: usize = 4096;

/// What the crate charges its work through.
pub trait Budget {
    /// Charge `bytes` before the growth they pay for, or refuse and charge nothing.
    fn charge(&mut self, bytes: usize) -> Result<(), Stop>;

    /// Give back `bytes` charged before.
    fn credit(&mut self, bytes: usize);

    /// Grant steps, `asked` from 1 to [`MAX_GRANT`]: read the interrupt first, then spend and
    /// answer the smaller of `asked` and the steps left, at least 1, or refuse with
    /// [`Stop::StepBudget`] when no step is left.
    fn grant(&mut self, asked: u32) -> Result<u32, Stop>;

    /// Return `steps` granted and not run.
    fn refund(&mut self, steps: u32);

    /// Read the interrupt: `Err(Stop::Interrupted)` when it is set.
    fn poll(&mut self) -> Result<(), Stop>;
}

/// A budget that charges nothing and never stops: tests and tools.
#[derive(Clone, Copy, Debug, Default)]
pub struct Unlimited;

impl Budget for Unlimited {
    fn charge(&mut self, _bytes: usize) -> Result<(), Stop> {
        Ok(())
    }

    fn credit(&mut self, _bytes: usize) {}

    fn grant(&mut self, asked: u32) -> Result<u32, Stop> {
        Ok(asked.clamp(1, MAX_GRANT))
    }

    fn refund(&mut self, _steps: u32) {}

    fn poll(&mut self) -> Result<(), Stop> {
        Ok(())
    }
}

/// The fewest elements a vector grows to.
const MIN_ELEMENTS: usize = 4;

/// The bytes one entry of a `BTreeMap<K, V>` is charged at its insertion: its key and value bytes
/// plus a share of a node, the bound `swell-syntax` derives for its maps (`map_entry_bytes` in
/// `crates/syntax/src/ledger.rs`): a leaf and an internal node for every 5 entries.
pub(crate) const fn map_entry_bytes<K, V>() -> usize {
    let entry = size_of::<K>() + size_of::<V>();
    let leaf = 16 + 11 * entry;
    let internal = leaf + 96;
    entry + (leaf + internal).div_ceil(5)
}

/// The bytes of the buffer of `v` by its capacity.
pub(crate) fn capacity_bytes<T>(v: &Vec<T>) -> usize {
    v.capacity().saturating_mul(size_of::<T>())
}

/// The budget of one parse: what it holds charged, the first stop it met, and the counts between
/// two polls. Every growth goes through it, so that a stop can credit everything.
pub(crate) struct Meter<'b> {
    budget: &'b mut dyn Budget,
    held: usize,
    stop: Option<Stop>,
    reads: usize,
    visits: usize,
}

impl<'b> Meter<'b> {
    pub(crate) fn new(budget: &'b mut dyn Budget) -> Meter<'b> {
        Meter {
            budget,
            held: 0,
            stop: None,
            reads: 0,
            visits: 0,
        }
    }

    /// What the parse holds charged.
    pub(crate) fn held(&self) -> usize {
        self.held
    }

    /// The first stop the parse met, if any.
    pub(crate) fn stopped(&self) -> Option<Stop> {
        self.stop
    }

    fn halt(&mut self, stop: Stop) -> Stop {
        *self.stop.get_or_insert(stop)
    }

    /// Charge `bytes`; refused once the parse has stopped.
    pub(crate) fn charge(&mut self, bytes: usize) -> Result<(), Stop> {
        if let Some(stop) = self.stop {
            return Err(stop);
        }
        if bytes == 0 {
            return Ok(());
        }
        match self.budget.charge(bytes) {
            Ok(()) => {
                self.held = self.held.saturating_add(bytes);
                Ok(())
            }
            Err(stop) => Err(self.halt(stop)),
        }
    }

    /// Give back `bytes` the parse held.
    pub(crate) fn credit(&mut self, bytes: usize) {
        let bytes = bytes.min(self.held);
        if bytes == 0 {
            return;
        }
        self.held = self.held.saturating_sub(bytes);
        self.budget.credit(bytes);
    }

    /// Credit everything the parse holds.
    pub(crate) fn credit_all(&mut self) {
        self.credit(self.held);
    }

    /// Read the interrupt.
    pub(crate) fn poll(&mut self) -> Result<(), Stop> {
        if let Some(stop) = self.stop {
            return Err(stop);
        }
        match self.budget.poll() {
            Ok(()) => Ok(()),
            Err(stop) => Err(self.halt(stop)),
        }
    }

    /// `units` code units read: a poll once [`POLL_INTERVAL`] of them have been read since the
    /// last.
    pub(crate) fn read(&mut self, units: usize) -> Result<(), Stop> {
        self.reads = self.reads.saturating_add(units);
        if self.reads >= POLL_INTERVAL {
            self.reads = 0;
            return self.poll();
        }
        Ok(())
    }

    /// One node made or visited: a poll every [`POLL_INTERVAL`] of them.
    pub(crate) fn visit(&mut self) -> Result<(), Stop> {
        self.visits = self.visits.saturating_add(1);
        if self.visits >= POLL_INTERVAL {
            self.visits = 0;
            return self.poll();
        }
        Ok(())
    }

    /// Room for one more element in `v`: when it is full, its capacity doubles (at least 4),
    /// charged before the allocation.
    pub(crate) fn grow<T>(&mut self, v: &mut Vec<T>) -> Result<(), Stop> {
        if v.len() < v.capacity() {
            return Ok(());
        }
        let wanted = v.capacity().saturating_mul(2).max(MIN_ELEMENTS);
        self.reserve_to(v, wanted)
    }

    /// Room for `additional` more elements in `v`, its capacity doubled (at least 4) until they
    /// fit, charged before the allocation.
    pub(crate) fn grow_by<T>(&mut self, v: &mut Vec<T>, additional: usize) -> Result<(), Stop> {
        let needed = v.len().saturating_add(additional);
        if needed <= v.capacity() {
            return Ok(());
        }
        let mut wanted = v.capacity().max(MIN_ELEMENTS);
        while wanted < needed {
            wanted = wanted.saturating_mul(2);
        }
        self.reserve_to(v, wanted)
    }

    /// The capacity of `v` raised to `wanted`, the growth charged first.
    fn reserve_to<T>(&mut self, v: &mut Vec<T>, wanted: usize) -> Result<(), Stop> {
        let had = v.capacity();
        if wanted <= had {
            return Ok(());
        }
        let bytes = wanted.saturating_sub(had).saturating_mul(size_of::<T>());
        self.charge(bytes)?;
        if v.try_reserve_exact(wanted.saturating_sub(v.len())).is_err() {
            self.credit(bytes);
            return Err(self.halt(Stop::MemoryBudget));
        }
        let got = v.capacity();
        if got > wanted {
            // The allocator gave more room than asked: the rest is charged too. When that is
            // refused the parse has stopped, answers the stop at its next charge or poll, and
            // credits only what was charged, since a credit never passes what is held.
            let extra = got.saturating_sub(wanted).saturating_mul(size_of::<T>());
            let _ = self.charge(extra);
        }
        Ok(())
    }

    /// Push `item` onto `v`, grown first.
    pub(crate) fn push<T>(&mut self, v: &mut Vec<T>, item: T) -> Result<(), Stop> {
        self.grow(v)?;
        v.push(item);
        Ok(())
    }

    /// Drop `v` and credit its capacity.
    pub(crate) fn release<T>(&mut self, v: Vec<T>) {
        let bytes = capacity_bytes(&v);
        drop(v);
        self.credit(bytes);
    }

    /// Insert `key` with `value` into `map` when the key is new, an entry and the key's
    /// capacity charged first; the value already there otherwise, and `key` is dropped with
    /// its capacity credited.
    pub(crate) fn insert<V: Copy>(
        &mut self,
        map: &mut BTreeMap<Vec<u8>, V>,
        key: Vec<u8>,
        value: V,
    ) -> Result<Option<V>, Stop> {
        if let Some(old) = map.get(&key) {
            let old = *old;
            self.release(key);
            return Ok(Some(old));
        }
        if let Err(stop) = self.charge(map_entry_bytes::<Vec<u8>, V>()) {
            self.release(key);
            return Err(stop);
        }
        map.insert(key, value);
        Ok(None)
    }

    /// A copy of `bytes` with exactly their number as capacity, charged first.
    pub(crate) fn copy(&mut self, bytes: &[u8]) -> Result<Vec<u8>, Stop> {
        let mut v = Vec::new();
        if bytes.is_empty() {
            return Ok(v);
        }
        self.charge(bytes.len())?;
        if v.try_reserve_exact(bytes.len()).is_err() {
            self.credit(bytes.len());
            return Err(self.halt(Stop::MemoryBudget));
        }
        let got = v.capacity();
        if got > bytes.len() {
            let _ = self.charge(got.saturating_sub(bytes.len()));
        }
        v.extend_from_slice(bytes);
        Ok(v)
    }

    /// Drop `map` and credit its entries and keys.
    pub(crate) fn release_map<V>(&mut self, map: BTreeMap<Vec<u8>, V>) {
        let bytes = map.iter().fold(
            map.len().saturating_mul(map_entry_bytes::<Vec<u8>, V>()),
            |sum, (k, _)| sum.saturating_add(k.capacity()),
        );
        drop(map);
        self.credit(bytes);
    }
}
