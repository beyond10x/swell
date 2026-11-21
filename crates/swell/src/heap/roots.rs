//! The root table (`docs/design/host-interface.md`, decision 4, `Root`; `docs/design/engine.md`,
//! decision 4, "Host-held values"): the values the host keeps across calls. A [`Root`] is an
//! index and a generation into this table; dropping it pushes it on a release queue that the
//! agent drains at its next safe point and at every collection, so dropping needs no access to
//! the agent. The queue is an `Rc`, which keeps `Root` and with it `Agent` from being `Send`
//! (decision 1).
//!
//! The table is a root site of the collector: [`trace_roots`] reports every value it holds.

use std::cell::Cell;
use std::rc::Rc;

use super::collect::Mark;
use crate::host::agent::Agent;
use crate::host::{Host, HostError};
use crate::value::{Root, Value};

/// The queue of released roots, as (index, generation) pairs.
pub(crate) type ReleaseQueue = Rc<Cell<Vec<(u32, u32)>>>;

struct RootSlot {
    generation: u32,
    value: Option<Value>,
}

/// The root table of one agent.
pub(crate) struct RootTable {
    key: u32,
    slots: Vec<RootSlot>,
    free: Vec<u32>,
    released: ReleaseQueue,
}

impl RootTable {
    /// An empty table of the agent `key`.
    pub(crate) fn new(key: u32) -> RootTable {
        RootTable {
            key,
            slots: Vec::new(),
            free: Vec::new(),
            released: Rc::new(Cell::new(Vec::new())),
        }
    }

    /// A new root holding `value`. The caller has checked that `value` is live.
    pub(crate) fn add(&mut self, value: Value) -> Root {
        let reused = self.free.pop().and_then(|index| {
            let slot = usize::try_from(index)
                .ok()
                .and_then(|i| self.slots.get_mut(i))?;
            slot.value = Some(value);
            Some((index, slot.generation))
        });
        let (index, generation) = match reused {
            Some(found) => found,
            None => {
                let index = u32::try_from(self.slots.len()).unwrap_or(u32::MAX);
                self.slots.push(RootSlot {
                    generation: 0,
                    value: Some(value),
                });
                (index, 0)
            }
        };
        Root {
            key: self.key,
            index,
            generation,
            released: Rc::clone(&self.released),
        }
    }

    /// The value `root` holds: `ForeignHandle` for a root of another agent, `StaleHandle` for a
    /// released one.
    pub(crate) fn get(&self, root: &Root) -> Result<Value, HostError> {
        if root.key != self.key {
            return Err(HostError::ForeignHandle);
        }
        match usize::try_from(root.index)
            .ok()
            .and_then(|i| self.slots.get(i))
        {
            Some(RootSlot {
                generation,
                value: Some(value),
            }) if *generation == root.generation => Ok(*value),
            _ => Err(HostError::StaleHandle),
        }
    }

    /// Release every root the host dropped since the last drain.
    pub(crate) fn drain(&mut self) {
        for (index, generation) in self.released.take() {
            let Some(slot) = usize::try_from(index)
                .ok()
                .and_then(|i| self.slots.get_mut(i))
            else {
                continue;
            };
            if slot.generation != generation || slot.value.is_none() {
                continue;
            }
            slot.value = None;
            if let Some(next) = slot.generation.checked_add(1) {
                slot.generation = next;
                if next < u32::MAX {
                    self.free.push(index);
                }
            }
        }
    }

    /// The number of roots held.
    #[cfg(test)]
    pub(crate) fn held(&self) -> usize {
        self.slots
            .iter()
            .filter(|slot| slot.value.is_some())
            .count()
    }
}

/// Root site of the collector: every value of the root table.
pub(crate) fn trace_roots<H: Host>(agent: &Agent<H>, mark: &mut Mark) {
    mark.count("heap/roots.rs trace_roots");
    for slot in &agent.heap.roots.slots {
        if let Some(value) = slot.value {
            mark.value(value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::RootTable;
    use crate::{HostError, Value};

    #[test]
    fn a_dropped_root_is_released_at_the_drain_and_its_slot_reused() {
        let mut table = RootTable::new(3);
        let first = table.add(Value::Number(1.0));
        let second = table.add(Value::Null);
        assert_eq!(table.get(&first), Ok(Value::Number(1.0)));
        assert_eq!(table.held(), 2);

        let (index, generation) = (first.index, first.generation);
        drop(first);
        assert_eq!(table.held(), 2, "dropping only queues the release");
        table.drain();
        assert_eq!(table.held(), 1);

        let third = table.add(Value::Boolean(true));
        assert_eq!(third.index, index);
        assert_ne!(third.generation, generation);
        assert_eq!(table.get(&third), Ok(Value::Boolean(true)));
        assert_eq!(table.get(&second), Ok(Value::Null));

        let mut other = RootTable::new(4);
        let foreign = other.add(Value::Null);
        assert_eq!(table.get(&foreign), Err(HostError::ForeignHandle));
        drop(foreign);
        other.drain();
        assert_eq!(other.held(), 0);
    }
}
