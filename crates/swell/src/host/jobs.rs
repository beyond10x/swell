//! The microtask queue of the agent and its checkpoint (`docs/design/host-interface.md`,
//! decision 11, "A host microtask's realm", "The checkpoint is one entry", "Steps, the interrupt
//! and what a checkpoint leaves" and "What a queued job costs"; section 10, "The microtask
//! checkpoint"; `docs/design/engine.md`, decision 3, "The microtask checkpoint is one entry",
//! decision 4, "Queued jobs", and decision 9, "Jobs"; `html:perform-a-microtask-checkpoint`).
//! Written by `story:job-queue-and-microtask-checkpoint`.
//!
//! One FIFO queue in the agent (`Agent::microtasks`, of [`Entry`]) holds every entry in the
//! order it was queued. Before `Promise` the host's microtasks are its only entries: each holds
//! the realm it runs in and the host's closure ([`Microtask`]). A host microtask is the host's
//! own storage, bounded by the host's calls, and is not charged to the memory budget; the
//! queue's spare capacity is not charged either, and a checkpoint that leaves the queue empty
//! shrinks a buffer of more than [`SPARE_ENTRIES`] entries to that many.
//!
//! The checkpoint ([`Agent::checkpoint`]) runs every entry under the step budget of the entry
//! it runs in: `Agent::perform_microtask_checkpoint` is an outermost entry of its own, and
//! `Cx::perform_microtask_checkpoint` runs inside the running entry, or is an outermost entry
//! of its own from `with_cx`. While it runs, the agent's flag `checkpointing` is set, so that
//! every `Cx` call of an entry is a nested entry that spends from the checkpoint (`Agent::begin`)
//! and a checkpoint asked for inside it answers at once (HOST-81). Its loop is bounded by the
//! step budget: taking an entry off costs one step, charged while the entry is still first in
//! the queue, so one checkpoint takes at most `steps_per_entry` entries off, whatever its
//! entries queue.
//!
//! An entry is destructured with `let`, never matched with a wildcard: the entry of a promise
//! job (`story:promise-constructor-then-and-jobs`) makes each such place a compile error until
//! it handles that entry.
//!
//! Root site of the collector: [`trace_roots`], called from the root site of `host/agent.rs`,
//! reports the realm of every queued host microtask.

use super::agent::Agent;
use super::{Completion, Cx, Host, HostError};
use crate::heap::arena::Handle;
use crate::heap::collect::{Item, Mark};

/// A host microtask (decision 11): the host's code, run once by a checkpoint with a `Cx` whose
/// current realm is the realm it was queued with. What it needs of the engine it holds in
/// `Root`s; a dropped one is dropped as a Rust value, its closure never called.
pub type Microtask<H> = Box<dyn FnOnce(&mut Cx<'_, H>)>;

/// What a checkpoint that emptied the queue ran: the promise jobs and the host microtasks it
/// took off the queue and ran to their end (decision 11).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct CheckpointReport {
    pub promise_jobs: u64,
    pub host_microtasks: u64,
}

/// The entries a checkpoint that leaves the queue empty keeps room for; a larger buffer is
/// shrunk to this (`engine.md`, decision 4, "Queued jobs").
pub(crate) const SPARE_ENTRIES: usize = 64;

/// An entry of the microtask queue.
pub(crate) enum Entry<H: Host> {
    /// A host microtask with the realm it runs in, which the queue holds while it is queued.
    HostMicrotask { realm: Handle, task: Microtask<H> },
}

/// Root site of the collector: the realm of every queued host microtask, so that a realm the
/// host dropped lives until its microtask is taken off.
pub(crate) fn trace_roots<H: Host>(agent: &Agent<H>, mark: &mut Mark) {
    for entry in &agent.microtasks {
        let Entry::HostMicrotask { realm, .. } = entry;
        mark.push(Item::Realm(*realm));
    }
}

impl<H: Host> Agent<H> {
    /// Queue `task` to run in `realm`, behind every entry queued. It runs no code, spends no step
    /// and calls no hook. A full buffer grows to twice its capacity, at least 4 entries, with
    /// `try_reserve_exact`; when the allocator refuses that, the agent cannot hold the entry, a
    /// broken invariant of the engine: the agent is faulted and nothing is queued.
    pub(crate) fn enqueue_microtask(
        &mut self,
        realm: Handle,
        task: Microtask<H>,
    ) -> Result<(), HostError> {
        if self.microtasks.len() == self.microtasks.capacity() {
            let more = self.microtasks.capacity().max(4);
            if self.microtasks.try_reserve_exact(more).is_err() {
                self.fault();
                return Err(HostError::Faulted);
            }
        }
        self.microtasks
            .push_back(Entry::HostMicrotask { realm, task });
        Ok(())
    }

    /// Drop every queued entry unrun and answer how many: a host microtask is dropped as a Rust
    /// value, its closure never called. It runs no script, calls no hook and spends no step; a
    /// host microtask was not charged, so nothing is credited for it (the promise jobs of
    /// `story:promise-constructor-then-and-jobs` are).
    pub(crate) fn drop_microtasks(&mut self) -> usize {
        let dropped = std::mem::take(&mut self.microtasks);
        let count = dropped.len();
        for entry in dropped {
            let Entry::HostMicrotask { task, .. } = entry;
            drop(task);
        }
        count
    }

    /// The loop of a checkpoint, inside the entry the caller began: the flag `checkpointing` is
    /// set until it returns, on every path.
    pub(crate) fn checkpoint(&mut self) -> Completion<CheckpointReport> {
        self.checkpointing = true;
        let ran = self.run_microtasks();
        self.checkpointing = false;
        ran
    }

    /// While the queue is not empty: the step of taking the first entry off, charged with
    /// `charge_steps` (which reads the interrupt first) while the entry is still first, so that
    /// a refused step leaves it there, unrun and whole; then the entry taken off and run, the
    /// scope and the scoped scripts truncated after it, on every way out, to what they held
    /// before it (one budget, a scope per job: HOST-196); and a safe point before the next
    /// entry, where a collection may run. A termination ends the loop: the entry that was
    /// running is not run again, and every entry behind it stays queued in its order. Each turn
    /// spends at least one step, so the step budget bounds the turns.
    fn run_microtasks(&mut self) -> Completion<CheckpointReport> {
        let mut report = CheckpointReport::default();
        while !self.microtasks.is_empty() {
            self.charge_steps(1)?;
            let Some(entry) = self.microtasks.pop_front() else {
                break;
            };
            let scope = self.scope.len();
            let scripts = self.scoped_scripts.len();
            let ran = self.run_entry(entry, &mut report);
            self.scope.truncate(scope);
            self.scoped_scripts.truncate(scripts);
            ran?;
            if self.safe_point().is_err() {
                return Err(self.fault());
            }
        }
        // ClearKeptObjects (`#sec-clear-kept-objects`, `html:perform-a-microtask-checkpoint`):
        // the agent keeps no object for a WeakRef before slice 16, so there is nothing to clear.
        if self.microtasks.capacity() > SPARE_ENTRIES {
            self.microtasks.shrink_to(SPARE_ENTRIES);
        }
        Ok(report)
    }

    /// Run one entry taken off the queue, and count it in `report` when it ran to its end. A
    /// host microtask runs with its realm as the current realm (the realm of the entry, or of
    /// the running builtin when the checkpoint runs inside a host function), restored after it;
    /// a termination of a call it made is the run's, sticky after it (decision 10).
    fn run_entry(&mut self, entry: Entry<H>, report: &mut CheckpointReport) -> Completion<()> {
        let Entry::HostMicrotask { realm, task } = entry;
        // The queue held the realm, so it is alive: anything else is a broken invariant.
        if self.heap.realms.get(realm).is_err() {
            return Err(self.fault());
        }
        let saved = self.realm_slot();
        self.set_realm_slot(Some(realm));
        task(&mut Cx { agent: self });
        self.set_realm_slot(saved);
        self.running()?;
        report.host_microtasks = report.host_microtasks.saturating_add(1);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::host::hooks::test_host::{NoHooks, agent_with_realm};
    use crate::value::{Realm, Root};
    use crate::{Cx, HostError};

    /// HOST-192's released realm, which the interface cannot make (a `Realm` is released only
    /// by its drop): a `Realm` whose root names its slot with a later generation, as a slot
    /// released and taken again does, answers `Err(StaleHandle)` and queues nothing. Its drop
    /// releases a slot of another generation, which the root table ignores.
    #[test]
    fn queue_microtask_with_a_released_realm_answers_stale_handle() {
        let (mut agent, realm) = agent_with_realm();
        let released = Realm(Root {
            key: realm.0.key,
            index: realm.0.index,
            generation: realm.0.generation.wrapping_add(1),
            released: std::rc::Rc::clone(&realm.0.released),
        });
        let answer = agent.queue_microtask(&released, Box::new(|_cx: &mut Cx<'_, NoHooks>| {}));
        assert_eq!(answer, Err(HostError::StaleHandle));
        assert_eq!(agent.pending_microtasks(), Ok(0));
    }
}
