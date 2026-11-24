//! The microtask queue of the agent and its checkpoint (`docs/design/host-interface.md`,
//! decision 11, "A host microtask's realm", "The checkpoint is one entry", "Steps, the interrupt
//! and what a checkpoint leaves" and "What a queued job costs"; section 10, "The microtask
//! checkpoint"; `docs/design/engine.md`, decision 3, "The microtask checkpoint is one entry",
//! decision 4, "Queued jobs", and decision 9, "Jobs"; `html:perform-a-microtask-checkpoint`).
//! Written by `story:job-queue-and-microtask-checkpoint`.
//!
//! One FIFO queue in the agent (`Agent::microtasks`, of [`Entry`]) holds every entry in the
//! order it was queued: the host's microtasks, each with the realm it runs in and the host's
//! closure ([`Microtask`]), and the promise jobs (`story:promise-constructor-then-and-jobs`).
//! A host microtask's closure is the host's own storage, bounded by the host's calls, and is not
//! charged to the memory budget. The queue's capacity is: a `Charge` of the agent holds its
//! bytes, grown before the buffer grows; a refused growth queues nothing and answers
//! `Err(HostError::MemoryBudget)` to the host, or ends the running entry in
//! `Terminated(MemoryBudget)` for a promise job. A checkpoint that leaves the queue empty shrinks
//! a buffer of more than [`SPARE_ENTRIES`] entries to that many and gives back the charge above
//! it; dropping the queue gives back all of it.
//!
//! A promise job (HostEnqueuePromiseJob, `#sec-hostenqueuepromisejob`; `engine.md`, decision 4,
//! "Queued jobs") holds the Job Abstract Closure with what it captured (`ops/promise.rs`,
//! `PromiseJob`), its realm or none, the realm current when it was queued and the active script
//! or module then. Its enqueue spends one step before the entry is made, by the checkpoint when
//! a job queues it and by the running entry otherwise, then charges the bytes of the entry
//! ([`promise_job_bytes`]) and calls `host_enqueue_promise_job` with the job's realm (HOST-83)
//! once the entry is queued. The bytes are credited when the checkpoint takes the entry off,
//! whether the job then runs to its end or a termination ends it, or when
//! `Agent::clear_microtasks` drops it; an entry a terminated checkpoint left queued stays
//! charged. The script or module is recorded and traced; no step of this part reads the active
//! script or module, which `import()` of a later slice does.
//!
//! A promise job runs with its values on the kept stack and the [[HostDefined]] of its
//! JobCallback in `Agent::running_host_defined`, so a collection inside it keeps them; with its
//! realm current, or the realm current at its enqueue when its realm is null; between
//! `host_prepare_to_run_job` and `host_clean_up_after_job` when its realm is not null (HOST-82),
//! the second with `Err(value)` for a throw completion and `Ok(())` otherwise, after a
//! termination too, so the two stay paired.
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
//! An entry is matched with an arm for each variant, never with a wildcard.
//!
//! Root site of the collector: [`trace_roots`], called from the root site of `host/agent.rs`,
//! reports the realm of every queued host microtask, every value, realm and script of every
//! queued promise job with the [[HostDefined]] of its JobCallbacks, and the [[HostDefined]] of
//! the promise jobs that run.

use super::agent::Agent;
use super::outcome::Abrupt;
use super::{Completion, Cx, HookCx, Host, HostError, Termination};
use crate::heap::arena::Handle;
use crate::heap::collect::{Item, Mark};
use crate::object::promise::HOST_DEFINED_BYTES;
use crate::ops::promise::PromiseJob;
use crate::value::RealmRef;

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
    /// A promise job with its realm or none, the realm current when it was queued and the
    /// active script or module then, which the queue holds while it is queued.
    PromiseJob {
        job: PromiseJob,
        realm: Option<Handle>,
        queued_realm: Option<Handle>,
        script: Option<Handle>,
    },
}

/// The bytes of one entry of the queue's capacity.
fn entry_bytes<H: Host>() -> u64 {
    u64::try_from(size_of::<Entry<H>>()).unwrap_or(u64::MAX)
}

/// The bytes a promise job is charged while it is queued: its entry of the queue, and the box
/// of a [[HostDefined]] its JobCallback may hold.
pub(crate) fn promise_job_bytes<H: Host>() -> u64 {
    entry_bytes::<H>().saturating_add(HOST_DEFINED_BYTES)
}

/// Root site of the collector: the realm of every queued host microtask, so that a realm the
/// host dropped lives until its microtask is taken off; every value, realm and script of every
/// queued promise job and the [[HostDefined]] of its JobCallbacks, through `HostData::trace`;
/// and the [[HostDefined]] of the promise jobs that run.
pub(crate) fn trace_roots<H: Host>(agent: &Agent<H>, mark: &mut Mark) {
    for entry in &agent.microtasks {
        match entry {
            Entry::HostMicrotask { realm, .. } => mark.push(Item::Realm(*realm)),
            Entry::PromiseJob {
                job,
                realm,
                queued_realm,
                script,
            } => {
                job.trace(mark);
                for realm in [realm, queued_realm].into_iter().flatten() {
                    mark.push(Item::Realm(*realm));
                }
                if let Some(script) = script {
                    mark.push(Item::Script(*script));
                }
            }
        }
    }
    for host_defined in &agent.running_host_defined {
        host_defined.trace(mark);
    }
}

/// Why the queue has no room for one more entry.
enum NoRoom {
    /// The memory budget refused the growth; nothing changed.
    Budget,
    /// The allocator refused the growth; the agent is faulted.
    Faulted,
}

impl<H: Host> Agent<H> {
    /// Room for one more entry: a full buffer grows to twice its capacity, at least 4 entries,
    /// with no smaller fallback, the growth charged to the queue's [`Charge`] before
    /// `try_reserve_exact`. When the budget refuses the growth nothing changes. When the
    /// allocator refuses it, the growth's charge is given back and the agent cannot hold the
    /// entry, a broken invariant of the engine: the agent is faulted.
    ///
    /// [`Charge`]: crate::heap::accounting::Charge
    fn reserve_entry(&mut self) -> Result<(), NoRoom> {
        if self.microtasks.len() == self.microtasks.capacity() {
            let more = self.microtasks.capacity().max(4);
            let growth = u64::try_from(more)
                .unwrap_or(u64::MAX)
                .saturating_mul(entry_bytes::<H>());
            if self
                .queue_charge
                .grow(&mut self.heap.accounting, growth)
                .is_err()
            {
                return Err(NoRoom::Budget);
            }
            if self.microtasks.try_reserve_exact(more).is_err() {
                self.queue_charge.release(growth);
                self.fault();
                return Err(NoRoom::Faulted);
            }
            self.note_queue();
        }
        Ok(())
    }

    /// Give back the queue's charge above its capacity now.
    fn release_queue_room(&mut self) {
        let held = u64::try_from(self.microtasks.capacity())
            .unwrap_or(u64::MAX)
            .saturating_mul(entry_bytes::<H>());
        let above = self.queue_charge.bytes().saturating_sub(held);
        self.queue_charge.release(above);
        self.note_queue();
    }

    /// The counting build: report the queue's capacity and its charge to the tallies of
    /// `copies` (`object/string.rs`).
    fn note_queue(&mut self) {
        #[cfg(test)]
        {
            let capacity = u64::try_from(self.microtasks.capacity())
                .unwrap_or(u64::MAX)
                .saturating_mul(entry_bytes::<H>());
            self.queue_tally.set(capacity, self.queue_charge.bytes());
        }
    }

    /// Queue `task` to run in `realm`, behind every entry queued. It runs no code, spends no step
    /// and calls no hook. When the memory budget refuses the queue's growth
    /// ([`Agent::reserve_entry`]) nothing is queued and it answers `Err(MemoryBudget)`; when the
    /// allocator refuses it the agent is faulted and nothing is queued.
    pub(crate) fn enqueue_microtask(
        &mut self,
        realm: Handle,
        task: Microtask<H>,
    ) -> Result<(), HostError> {
        match self.reserve_entry() {
            Ok(()) => {}
            Err(NoRoom::Budget) => return Err(HostError::MemoryBudget),
            Err(NoRoom::Faulted) => return Err(HostError::Faulted),
        }
        self.microtasks
            .push_back(Entry::HostMicrotask { realm, task });
        Ok(())
    }

    /// HostEnqueuePromiseJob ( job, realm ) (`#sec-hostenqueuepromisejob`): one step, charged
    /// before the entry is made; the bytes of the entry; the entry queued behind every entry with
    /// the realm current now and the active script or module (the script of the innermost frame,
    /// none outside every script); then `host_enqueue_promise_job` with `realm` (HOST-83), in a
    /// hook scope. A refused step, entry charge or growth of the queue's capacity ends the run
    /// with nothing queued and the entry's bytes credited; a queue the allocator cannot grow
    /// faults the agent.
    pub(crate) fn enqueue_promise_job(
        &mut self,
        job: PromiseJob,
        realm: Option<Handle>,
    ) -> Completion<()> {
        self.charge_steps(1)?;
        let bytes = promise_job_bytes::<H>();
        self.charge_bytes(bytes)?;
        match self.reserve_entry() {
            Ok(()) => {}
            Err(NoRoom::Budget) => {
                self.heap.accounting.credit(bytes);
                return Err(self.terminate(Termination::MemoryBudget));
            }
            Err(NoRoom::Faulted) => {
                self.heap.accounting.credit(bytes);
                return Err(self.fault());
            }
        }
        let queued_realm = self.realm_slot();
        let script = self.interp.frames.last().map(|frame| frame.script);
        self.microtasks.push_back(Entry::PromiseJob {
            job,
            realm,
            queued_realm,
            script,
        });
        let start = self.scope.len();
        H::host_enqueue_promise_job(
            &mut HookCx { agent: self },
            realm.map(|handle| RealmRef { handle }),
        );
        self.scope.truncate(start);
        self.running()
    }

    /// Drop every queued entry unrun and answer how many: a host microtask is dropped as a Rust
    /// value, its closure never called; a promise job is dropped with the bytes of its entry
    /// credited. The queue's capacity goes with them and its charge is given back. It runs no
    /// script, calls no hook and spends no step; a host microtask has no entry charge, so
    /// nothing more is credited for it.
    pub(crate) fn drop_microtasks(&mut self) -> usize {
        let dropped = std::mem::take(&mut self.microtasks);
        self.release_queue_room();
        let count = dropped.len();
        for entry in dropped {
            match entry {
                Entry::HostMicrotask { task, .. } => drop(task),
                Entry::PromiseJob { .. } => self.heap.accounting.credit(promise_job_bytes::<H>()),
            }
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
            self.release_queue_room();
        }
        Ok(report)
    }

    /// Run one entry taken off the queue, and count it in `report` when it ran to its end. A
    /// host microtask runs with its realm as the current realm (the realm of the entry, or of
    /// the running builtin when the checkpoint runs inside a host function), restored after it;
    /// a termination of a call it made is the run's, sticky after it (decision 10). A promise
    /// job runs as [`Agent::run_promise_entry`] says.
    fn run_entry(&mut self, entry: Entry<H>, report: &mut CheckpointReport) -> Completion<()> {
        match entry {
            Entry::HostMicrotask { realm, task } => {
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
            Entry::PromiseJob {
                job,
                realm,
                queued_realm,
                script: _,
            } => {
                self.heap.accounting.credit(promise_job_bytes::<H>());
                self.run_promise_entry(&job, realm, queued_realm)?;
                report.promise_jobs = report.promise_jobs.saturating_add(1);
                Ok(())
            }
        }
    }

    /// Run the promise job `job` taken off the queue, whose entry was credited: its values on
    /// the kept stack and the [[HostDefined]] of its JobCallback in
    /// `Agent::running_host_defined` while it runs; `realm`, or `queued_realm` when `realm` is
    /// null, as the current realm, restored after it; `host_prepare_to_run_job` before it and
    /// `host_clean_up_after_job` after it when `realm` is not null (HOST-82), the second with
    /// `Err(value)` for a throw completion and `Ok(())` otherwise, after a termination too. A
    /// job that ends in a throw completion ran to its end; a termination is the run's.
    fn run_promise_entry(
        &mut self,
        job: &PromiseJob,
        realm: Option<Handle>,
        queued_realm: Option<Handle>,
    ) -> Completion<()> {
        // The queue held both realms, so they are alive: anything else is a broken invariant.
        for held in [realm, queued_realm].into_iter().flatten() {
            if self.heap.realms.get(held).is_err() {
                return Err(self.fault());
            }
        }
        let mark = self.kept.len();
        for value in job.values() {
            self.keep(value);
        }
        let running = self.running_host_defined.len();
        if let Some(host_defined) = job.host_defined_object() {
            self.running_host_defined.push(host_defined);
        }
        let saved = self.realm_slot();
        self.set_realm_slot(realm.or(queued_realm));
        if let Some(realm) = realm {
            let start = self.scope.len();
            H::host_prepare_to_run_job(&mut HookCx { agent: self }, RealmRef { handle: realm });
            self.scope.truncate(start);
        }
        let completion = match self.running() {
            Ok(()) => self.run_promise_job(job),
            Err(abrupt) => Err(abrupt),
        };
        if let Some(realm) = realm {
            let result = match completion {
                Err(Abrupt::Throw(value)) => Err(value),
                Ok(_) | Err(Abrupt::Terminated(_)) => Ok(()),
            };
            let start = self.scope.len();
            H::host_clean_up_after_job(
                &mut HookCx { agent: self },
                Some(RealmRef { handle: realm }),
                result,
            );
            self.scope.truncate(start);
        }
        self.set_realm_slot(saved);
        self.running_host_defined.truncate(running);
        self.release(mark);
        self.running()?;
        match completion {
            Ok(_) | Err(Abrupt::Throw(_)) => Ok(()),
            Err(Abrupt::Terminated(termination)) => Err(Abrupt::Terminated(termination)),
        }
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

    /// The bytes a queued promise job is charged, which `tests/promises.rs` states as
    /// `ENTRY_BYTES`: the entry of the queue and the box of a [[HostDefined]].
    #[test]
    fn a_promise_job_entry_is_charged_its_size() {
        let entry = size_of::<super::Entry<NoHooks>>();
        assert_eq!(
            super::promise_job_bytes::<NoHooks>(),
            176,
            "the entry is {entry} bytes"
        );
    }

    /// The bytes of one entry of the queue's capacity.
    fn entry_bytes() -> u64 {
        u64::try_from(size_of::<super::Entry<NoHooks>>()).unwrap()
    }

    /// A host microtask that calls the global function `f` of the realm of `global`, or does
    /// nothing when `global` is `None`.
    fn calling_f(global: Option<Root>) -> super::Microtask<NoHooks> {
        Box::new(move |cx: &mut Cx<'_, NoHooks>| {
            let Some(global) = global else {
                return;
            };
            let crate::Value::Object(object) = cx.get(&global).unwrap() else {
                panic!("the global object is no object");
            };
            let key = crate::PropertyKey::String(cx.new_string("f").unwrap());
            let f = cx.get_property(object, key).unwrap().unwrap();
            cx.call(f, crate::Value::Undefined, &[]).unwrap().unwrap();
        })
    }

    /// `docs/design/engine.md`, "Queued jobs": the microtask queue's capacity is charged
    /// before it grows. 100 host microtasks raise `used()` by the capacity of 128 entries; a
    /// checkpoint whose first microtask calls into script finds no capacity uncovered there;
    /// after it the queue keeps 64 entries of room, charged; 100 more and a
    /// `clear_microtasks` give back all of it.
    #[test]
    fn the_queue_is_charged_by_its_capacity_across_a_call() {
        use crate::object::string::across::Across;
        use crate::object::string::copies;
        let mut across = Across::with_steps(1_000_000);
        across.run("function f() {}");
        let global = across.agent.global_object(&across.realm).unwrap();
        let first = across.agent.duplicate(&global).unwrap();
        across
            .agent
            .queue_microtask(&across.realm, calling_f(Some(first)))
            .unwrap();
        across.agent.perform_microtask_checkpoint().unwrap();
        across.agent.clear_microtasks().unwrap();
        across.agent.collect().unwrap();
        let before = across.used();
        for index in 0..100 {
            let global = (index == 0).then(|| across.agent.duplicate(&global).unwrap());
            across
                .agent
                .queue_microtask(&across.realm, calling_f(global))
                .unwrap();
        }
        assert_eq!(across.agent.microtasks.capacity(), 128);
        assert_eq!(across.used(), before + 128 * entry_bytes());
        copies::reset();
        let ran = across.agent.perform_microtask_checkpoint().unwrap();
        assert!(matches!(ran, crate::Outcome::Normal(_)), "{ran:?}");
        assert_eq!(copies::uncovered_at_entry(), 0);
        across.agent.collect().unwrap();
        assert_eq!(across.used(), before + 64 * entry_bytes());
        for _ in 0..100 {
            across
                .agent
                .queue_microtask(&across.realm, calling_f(None))
                .unwrap();
        }
        assert_eq!(across.agent.clear_microtasks(), Ok(100));
        assert_eq!(across.used(), before);
    }

    /// `docs/design/engine.md`, "Queued jobs": a promise job whose queue would have to grow
    /// past the memory budget ends the running entry in `Terminated(MemoryBudget)` with nothing
    /// queued; the entry's bytes are credited and no capacity is charged. The room is the most
    /// the fourth `then` charged at once, so a `then` that does not grow the queue fits it. With
    /// a collection at every safe point the readings of `used()` after the fourth and the fifth
    /// differ by what the collections in each freed, so that reading is compared in the default
    /// mode only; the queue's charge is read in both.
    #[test]
    fn a_promise_job_past_the_queue_room_ends_across_a_call() {
        use crate::builtins::string::transform::tests::Answer;
        use crate::object::string::across::Across;
        let mut across = Across::with_steps(1_000_000);
        across.run("var p = Promise.resolve(); function f() {}");
        let then = across.parse("p.then(f)");
        for _ in 0..8 {
            across.evaluate(&then);
        }
        across.agent.clear_microtasks().unwrap();
        across.agent.collect().unwrap();
        for _ in 0..3 {
            assert!(matches!(across.evaluate(&then), Answer::Normal(_)));
        }
        let before = across.used();
        across.agent.heap.accounting.reset_high_water();
        assert!(matches!(across.evaluate(&then), Answer::Normal(_)));
        let rise = across.used() - before;
        let peak = across.agent.heap.accounting.high_water() - before;
        assert_eq!(across.agent.pending_microtasks(), Ok(4));
        assert_eq!(across.agent.microtasks.capacity(), 4);
        let before = across.used();
        across.room(peak + entry_bytes());
        assert_eq!(
            across.evaluate(&then),
            Answer::Terminated(crate::Termination::MemoryBudget)
        );
        if !cfg!(feature = "collect-every-safe-point") {
            assert_eq!(
                across.used(),
                before + rise - super::promise_job_bytes::<NoHooks>()
            );
        }
        assert_eq!(across.agent.queue_charge.bytes(), 4 * entry_bytes());
        assert_eq!(across.agent.microtasks.capacity(), 4);
        assert_eq!(across.agent.pending_microtasks(), Ok(4));
    }

    /// `docs/design/host-interface.md`, decision 11, "What a queued job costs": a host
    /// microtask whose queue would have to grow past the memory budget is refused with
    /// `Err(HostError::MemoryBudget)`; nothing is queued and nothing is charged.
    #[test]
    fn a_host_microtask_past_the_queue_room_is_refused_across_a_call() {
        let (mut agent, realm) = agent_with_realm();
        for _ in 0..4 {
            agent.queue_microtask(&realm, calling_f(None)).unwrap();
        }
        let used = agent.heap.accounting.used();
        agent.heap.accounting.set_limit(used + entry_bytes());
        assert_eq!(
            agent.queue_microtask(&realm, calling_f(None)),
            Err(HostError::MemoryBudget)
        );
        assert_eq!(agent.pending_microtasks(), Ok(4));
        assert_eq!(agent.heap.accounting.used(), used);
    }
}
