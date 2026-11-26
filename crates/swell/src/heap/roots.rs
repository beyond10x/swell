//! The root table (`docs/design/host-interface.md`, decision 4, `Root`; `docs/design/engine.md`,
//! decision 4, "Host-held values"): the values the host keeps across calls, and the realm
//! records its `Realm`s and the script records its `Script`s hold. A [`Root`] is an index and
//! a generation into this table; dropping it pushes it on a release queue that the agent drains
//! at its next safe point and at every
//! collection, so dropping needs no access to the agent. The queue is an `Rc`, which keeps
//! `Root` and with it `Agent` from being `Send` (decision 1).
//!
//! The table is a root site of the collector: [`trace_roots`] reports every value, realm
//! record and script record it holds.

use alloc::rc::Rc;
use alloc::vec::Vec;
use core::cell::Cell;

use super::arena::Handle;
use super::collect::{Item, Mark};
use crate::host::agent::Agent;
use crate::host::{Host, HostError};
use crate::value::{Root, Value};

/// The queue of released roots, as (index, generation) pairs, and whether a root was pushed on
/// it since the last drain: a drop pushes and sets the flag, and a drain reads the flag before
/// it takes the queue, so that a safe point after no drop does no more than that read.
#[derive(Default)]
pub(crate) struct ReleaseQueue {
    entries: Cell<Vec<(u32, u32)>>,
    pending: Cell<bool>,
}

impl ReleaseQueue {
    /// Queue the release of the root at `index` with `generation`.
    pub(crate) fn push(&self, index: u32, generation: u32) {
        let mut entries = self.entries.take();
        entries.push((index, generation));
        self.entries.set(entries);
        self.pending.set(true);
    }
}

/// What a root holds: a value, the realm record of a `Realm`, or the script record of a
/// `Script`.
#[derive(Clone, Copy)]
enum Held {
    Value(Value),
    Realm(Handle),
    Script(Handle),
}

struct RootSlot {
    generation: u32,
    held: Option<Held>,
}

/// The root table of one agent.
pub(crate) struct RootTable {
    key: u32,
    slots: Vec<RootSlot>,
    free: Vec<u32>,
    released: Rc<ReleaseQueue>,
    /// The counting build: the drains that took the release queue; a test resets it before
    /// it reads it.
    #[cfg(test)]
    pub(crate) queue_taken: u64,
}

impl RootTable {
    /// An empty table of the agent `key`.
    pub(crate) fn new(key: u32) -> RootTable {
        RootTable {
            key,
            slots: Vec::new(),
            free: Vec::new(),
            released: Rc::new(ReleaseQueue::default()),
            #[cfg(test)]
            queue_taken: 0,
        }
    }

    /// A new root holding `held`.
    fn hold(&mut self, held: Held) -> Root {
        let reused = self.free.pop().and_then(|index| {
            let slot = usize::try_from(index)
                .ok()
                .and_then(|i| self.slots.get_mut(i))?;
            slot.held = Some(held);
            Some((index, slot.generation))
        });
        let (index, generation) = match reused {
            Some(found) => found,
            None => {
                let index = u32::try_from(self.slots.len()).unwrap_or(u32::MAX);
                self.slots.push(RootSlot {
                    generation: 0,
                    held: Some(held),
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

    /// A new root holding `value`. The caller has checked that `value` is live.
    pub(crate) fn add(&mut self, value: Value) -> Root {
        self.hold(Held::Value(value))
    }

    /// A new root holding the realm record `realm`, for a `Realm`.
    pub(crate) fn add_realm(&mut self, realm: Handle) -> Root {
        self.hold(Held::Realm(realm))
    }

    /// A new root holding the script record `script`, for a `Script`.
    pub(crate) fn add_script(&mut self, script: Handle) -> Root {
        self.hold(Held::Script(script))
    }

    /// What `root` holds: `ForeignHandle` for a root of another agent, `StaleHandle` for a
    /// released one.
    fn lookup(&self, root: &Root) -> Result<Held, HostError> {
        if root.key != self.key {
            return Err(HostError::ForeignHandle);
        }
        match usize::try_from(root.index)
            .ok()
            .and_then(|i| self.slots.get(i))
        {
            Some(RootSlot {
                generation,
                held: Some(held),
            }) if *generation == root.generation => Ok(*held),
            _ => Err(HostError::StaleHandle),
        }
    }

    /// The value `root` holds: `ForeignHandle` for a root of another agent, `StaleHandle` for a
    /// released one. The root of a `Realm` holds no value; the host cannot reach it, since a
    /// `Realm` does not give its root out.
    pub(crate) fn get(&self, root: &Root) -> Result<Value, HostError> {
        match self.lookup(root)? {
            Held::Value(value) => Ok(value),
            Held::Realm(_) | Held::Script(_) => Err(HostError::StaleHandle),
        }
    }

    /// The realm record the root of a `Realm` holds, with the answers of [`RootTable::get`].
    pub(crate) fn get_realm(&self, root: &Root) -> Result<Handle, HostError> {
        match self.lookup(root)? {
            Held::Realm(realm) => Ok(realm),
            Held::Value(_) | Held::Script(_) => Err(HostError::StaleHandle),
        }
    }

    /// The script record the root of a `Script` holds, with the answers of [`RootTable::get`].
    pub(crate) fn get_script(&self, root: &Root) -> Result<Handle, HostError> {
        match self.lookup(root)? {
            Held::Script(script) => Ok(script),
            Held::Value(_) | Held::Realm(_) => Err(HostError::StaleHandle),
        }
    }

    /// Release every root the host dropped since the last drain. With none dropped, one read of
    /// the queue's flag: the queue is not taken.
    #[inline]
    pub(crate) fn drain(&mut self) {
        if self.released.pending.get() {
            self.drain_queue();
        }
    }

    /// The work of [`RootTable::drain`] after a drop: the flag cleared and every queued root
    /// released.
    #[inline(never)]
    fn drain_queue(&mut self) {
        self.released.pending.set(false);
        #[cfg(test)]
        {
            self.queue_taken = self.queue_taken.saturating_add(1);
        }
        for (index, generation) in self.released.entries.take() {
            let Some(slot) = usize::try_from(index)
                .ok()
                .and_then(|i| self.slots.get_mut(i))
            else {
                continue;
            };
            if slot.generation != generation || slot.held.is_none() {
                continue;
            }
            slot.held = None;
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
        self.slots.iter().filter(|slot| slot.held.is_some()).count()
    }
}

/// Root site of the collector: every value, realm record and script record of the root table.
pub(crate) fn trace_roots<H: Host>(agent: &Agent<H>, mark: &mut Mark) {
    mark.count("heap/roots.rs trace_roots");
    for slot in &agent.heap.roots.slots {
        match slot.held {
            Some(Held::Value(value)) => mark.value(value),
            Some(Held::Realm(realm)) => mark.push(Item::Realm(realm)),
            Some(Held::Script(script)) => mark.push(Item::Script(script)),
            None => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::RootTable;
    use crate::heap::arena::Handle;
    use crate::host::hooks::test_host::agent_with_realm;
    use crate::{HostError, Outcome, Parsed, Source, Value};

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

    /// The root of a `Realm` holds a realm record: `get_realm` reads it, `get` does not, and a
    /// value root is no realm.
    #[test]
    fn a_realm_root_holds_a_realm_record() {
        let mut table = RootTable::new(5);
        let realm = Handle {
            key: 5,
            index: 0,
            generation: 0,
        };
        let root = table.add_realm(realm);
        let value = table.add(Value::Null);
        assert_eq!(table.get_realm(&root), Ok(realm));
        assert_eq!(table.get(&root), Err(HostError::StaleHandle));
        assert_eq!(table.get_realm(&value), Err(HostError::StaleHandle));
        drop(root);
        table.drain();
        assert_eq!(table.held(), 1);
    }

    /// A safe point with no root dropped since the last drain does no work for the release
    /// queue (`story:an-index-store-does-not-go-through-text`): a script whose loop runs 20
    /// times, every instruction a safe point, takes the queue 0 times (2,815 times for 200 on
    /// its base); after two roots are dropped, the next run of it takes the queue once, and
    /// both are released by then, as they were at the first safe point after their drop
    /// before.
    #[test]
    fn a_safe_point_with_nothing_dropped_takes_no_queue() {
        let (mut agent, realm) = agent_with_realm();
        let Ok(Parsed::Parsed(script)) = agent.parse_script(
            &realm,
            Source::Utf8("for (var i = 0; i < 20; i++) {} i"),
            None,
        ) else {
            panic!("the script did not parse");
        };
        let global = agent.global_object(&realm).unwrap();
        // Whatever the setup dropped is released here.
        assert!(agent.collect().is_ok());
        agent.heap.roots.queue_taken = 0;
        let Ok(Outcome::Normal(first)) = agent.evaluate_script(&script) else {
            panic!("the first run did not complete");
        };
        assert_eq!(
            agent.heap.roots.queue_taken, 0,
            "drains that took the queue, nothing dropped"
        );
        let held = agent.heap.roots.held();
        drop(global);
        drop(first);
        assert_eq!(
            agent.heap.roots.held(),
            held,
            "dropping only queues the release"
        );
        let Ok(Outcome::Normal(second)) = agent.evaluate_script(&script) else {
            panic!("the second run did not complete");
        };
        assert_eq!(
            agent.heap.roots.queue_taken, 1,
            "drains that took the queue, two roots dropped"
        );
        assert_eq!(
            agent.heap.roots.held(),
            held - 1,
            "two released, the answer held"
        );
        drop(second);
    }
}
