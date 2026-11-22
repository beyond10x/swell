//! Arenas of slots, one per kind of heap record, and the handles into them (`docs/design/engine.md`,
//! decision 4, "Heap"; `docs/design/host-interface.md`, decision 4).
//!
//! A handle is the key of the agent that made it, a slot index and the slot's generation. Freeing
//! a record moves its slot's generation, so a handle kept past its record's life reaches nothing:
//! every access compares the key and the generation and answers [`Access`] instead of reading
//! another record. Inside the engine that answer ends the run in `Terminated(EngineFault)`; from
//! the host it is `Err(HostError::StaleHandle)` or `Err(HostError::ForeignHandle)`. A slot whose
//! generation reaches `u32::MAX` is retired, so a generation is never used twice.
//!
//! An arena is charged by capacity (`engine.md`, "Memory budget"): it grows by its own doubling,
//! at least [`MIN_SLOTS`] slots, with `try_reserve_exact`, and before it grows `store`
//! (`heap/mod.rs`) charges each new slot with its workspace ([`slot_bytes`]): an entry of the
//! free list, a mark word and room for one entry on the worklist ("Collector"). Those bytes stay
//! charged while the agent lives; an arena does not shrink, and a freed slot is used again. A
//! record is charged what it owns beside its slot, and the sweep credits that.

/// A handle into an arena: the key of the agent that made it, the slot index and the slot's
/// generation when the record was stored.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Handle {
    pub(crate) key: u32,
    pub(crate) index: u32,
    pub(crate) generation: u32,
}

/// Why a handle reaches no record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Access {
    /// The handle was made by another agent.
    Foreign,
    /// The record was freed, or the handle never named one of this arena.
    Stale,
}

/// The arena cannot take another record: the allocator refused to grow it, or its index space
/// is used up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Full;

struct Slot<T> {
    generation: u32,
    value: Option<T>,
    /// What the record owns beside its slot, credited when it is freed.
    bytes: u64,
}

/// The fewest slots an arena grows to.
const MIN_SLOTS: usize = 16;

/// The most slots an arena holds: a slot index is a `u32`.
fn max_slots() -> usize {
    usize::try_from(u32::MAX).unwrap_or(usize::MAX)
}

/// The bytes one slot of an arena of `T` is charged when the arena grows: the slot, one `u32` of
/// the free list, one `u32` mark word and one `u32` of room on the worklist.
pub(crate) fn slot_bytes<T>() -> u64 {
    let bytes = size_of::<Slot<T>>().saturating_add(3 * size_of::<u32>());
    u64::try_from(bytes).unwrap_or(u64::MAX)
}

/// The bytes of a slot alone, which a live record holds in the live reading.
fn record_slot_bytes<T>() -> u64 {
    u64::try_from(size_of::<Slot<T>>()).unwrap_or(u64::MAX)
}

/// The collector's workspace of one arena, grown with its capacity: a mark word per slot (0
/// unmarked, otherwise the generation marked plus one) and room on the worklist for one entry
/// per slot. The collector takes it out of the arena for a mark phase and gives it back.
#[derive(Default)]
pub(crate) struct Workspace {
    marks: Vec<u32>,
    work: Vec<u32>,
}

/// What marking a slot found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Marked {
    /// Marked now and put on the worklist.
    New,
    /// Marked before by the same generation.
    Before,
    /// Marked by another generation, or no slot of this arena: a stale handle.
    Stale,
    /// The worklist has no room: never, since a slot goes on it at most once.
    Full,
}

impl Workspace {
    /// Mark the slot `index` for the record of `generation`; the first time, put it on the
    /// worklist. Allocates nothing.
    pub(crate) fn mark(&mut self, index: u32, generation: u32) -> Marked {
        let Some(word) = generation.checked_add(1) else {
            return Marked::Stale;
        };
        let Some(mark) = usize::try_from(index)
            .ok()
            .and_then(|i| self.marks.get_mut(i))
        else {
            return Marked::Stale;
        };
        if *mark == word {
            return Marked::Before;
        }
        if *mark != 0 {
            return Marked::Stale;
        }
        if self.work.len() == self.work.capacity() {
            return Marked::Full;
        }
        *mark = word;
        self.work.push(index);
        Marked::New
    }

    /// The next slot on the worklist, with the generation it was marked for.
    pub(crate) fn pop(&mut self) -> Option<(u32, u32)> {
        let index = self.work.pop()?;
        let generation = usize::try_from(index)
            .ok()
            .and_then(|i| self.marks.get(i))
            .and_then(|word| word.checked_sub(1))
            .unwrap_or(u32::MAX);
        Some((index, generation))
    }

    /// The entries on the worklist.
    #[cfg(test)]
    pub(crate) fn held(&self) -> usize {
        self.work.len()
    }

    /// Whether the slot `index` is marked for the record of `generation`.
    #[cfg(test)]
    pub(crate) fn is_marked(&self, index: u32, generation: u32) -> bool {
        usize::try_from(index)
            .ok()
            .and_then(|i| self.marks.get(i))
            .is_some_and(|&word| generation.checked_add(1) == Some(word))
    }

    /// Whether the slot `index` is marked.
    fn marked(&self, index: usize) -> bool {
        self.marks.get(index).is_some_and(|&word| word != 0)
    }
}

/// The records of one kind. Freed slots are used again, most recently freed first, so the same
/// calls give the same handles.
pub(crate) struct Arena<T> {
    key: u32,
    slots: Vec<Slot<T>>,
    free: Vec<u32>,
    workspace: Workspace,
    /// The slots the budget was charged for.
    capacity: usize,
    /// The slots that hold a record.
    live: usize,
    /// The counting build: what the live records own beside their slots.
    #[cfg(test)]
    owned: u64,
    /// The counting build: pushes onto the free list past its capacity.
    #[cfg(test)]
    pub(crate) growths: u64,
}

impl<T> Arena<T> {
    /// An empty arena of the agent `key`.
    pub(crate) fn new(key: u32) -> Arena<T> {
        Arena {
            key,
            slots: Vec::new(),
            free: Vec::new(),
            workspace: Workspace::default(),
            capacity: 0,
            live: 0,
            #[cfg(test)]
            owned: 0,
            #[cfg(test)]
            growths: 0,
        }
    }

    /// The counting build: the bytes of the arena's vectors by their capacity (the slots, the
    /// free list, the mark words and the worklist), plus what the live records own beside their
    /// slots.
    #[cfg(test)]
    pub(crate) fn ledger_floor(&self) -> u64 {
        let bytes = |count: usize, size: usize| {
            u64::try_from(count.saturating_mul(size)).unwrap_or(u64::MAX)
        };
        bytes(self.slots.capacity(), size_of::<Slot<T>>())
            .saturating_add(bytes(self.free.capacity(), size_of::<u32>()))
            .saturating_add(bytes(self.workspace.marks.capacity(), size_of::<u32>()))
            .saturating_add(bytes(self.workspace.work.capacity(), size_of::<u32>()))
            .saturating_add(self.owned)
    }

    /// The capacity the arena grows to before it takes another record, or `None` when a free
    /// slot or a slot of its capacity takes it.
    fn growth(&self) -> Option<usize> {
        if !self.free.is_empty() || self.slots.len() < self.capacity {
            return None;
        }
        Some(
            self.capacity
                .saturating_mul(2)
                .max(MIN_SLOTS)
                .min(max_slots()),
        )
    }

    /// The bytes the next record's slot costs before what the record owns: the growth of the
    /// arena, each new slot with its workspace, or 0 when the arena has room.
    pub(crate) fn growth_bytes(&self) -> u64 {
        self.growth().map_or(0, |to| {
            let slots = u64::try_from(to.saturating_sub(self.capacity)).unwrap_or(u64::MAX);
            slots.saturating_mul(slot_bytes::<T>())
        })
    }

    /// Grow the slots, the free list, the mark words and the worklist to `to` slots, each with
    /// `try_reserve_exact`, so a refusal of the allocator is an answer, not an abort.
    fn grow(&mut self, to: usize) -> Result<(), Full> {
        if to <= self.capacity {
            return Err(Full);
        }
        self.slots
            .try_reserve_exact(to.saturating_sub(self.slots.len()))
            .map_err(|_| Full)?;
        self.free
            .try_reserve_exact(to.saturating_sub(self.free.len()))
            .map_err(|_| Full)?;
        let marks = &mut self.workspace.marks;
        marks
            .try_reserve_exact(to.saturating_sub(marks.len()))
            .map_err(|_| Full)?;
        marks.resize(to, 0);
        let work = &mut self.workspace.work;
        work.try_reserve_exact(to.saturating_sub(work.len()))
            .map_err(|_| Full)?;
        self.capacity = to;
        Ok(())
    }

    /// Store `value`, which owns `bytes` beside its slot, and answer its handle. A freed slot is
    /// used first; otherwise the arena grows when it has no room (`store` charged the growth
    /// before, [`Arena::growth_bytes`]).
    pub(crate) fn allocate(&mut self, value: T, bytes: u64) -> Result<Handle, Full> {
        if let Some(to) = self.growth() {
            self.grow(to)?;
        }
        let handle = if let Some(index) = self.free.pop()
            && let Some(slot) = usize::try_from(index)
                .ok()
                .and_then(|i| self.slots.get_mut(i))
        {
            slot.value = Some(value);
            slot.bytes = bytes;
            Handle {
                key: self.key,
                index,
                generation: slot.generation,
            }
        } else {
            let index = u32::try_from(self.slots.len()).map_err(|_| Full)?;
            // The room was reserved when the arena grew; a push never allocates.
            if self.slots.len() == self.slots.capacity() {
                return Err(Full);
            }
            self.slots.push(Slot {
                generation: 0,
                value: Some(value),
                bytes,
            });
            Handle {
                key: self.key,
                index,
                generation: 0,
            }
        };
        self.live = self.live.saturating_add(1);
        #[cfg(test)]
        {
            self.owned = self.owned.saturating_add(bytes);
        }
        Ok(handle)
    }

    fn slot(&self, handle: Handle) -> Result<&Slot<T>, Access> {
        if handle.key != self.key {
            return Err(Access::Foreign);
        }
        match usize::try_from(handle.index)
            .ok()
            .and_then(|i| self.slots.get(i))
        {
            Some(slot) if slot.generation == handle.generation && slot.value.is_some() => Ok(slot),
            _ => Err(Access::Stale),
        }
    }

    /// The record `handle` names.
    pub(crate) fn get(&self, handle: Handle) -> Result<&T, Access> {
        self.slot(handle)?.value.as_ref().ok_or(Access::Stale)
    }

    /// The record `handle` names, to change.
    pub(crate) fn get_mut(&mut self, handle: Handle) -> Result<&mut T, Access> {
        self.slot(handle)?;
        let slot = usize::try_from(handle.index)
            .ok()
            .and_then(|i| self.slots.get_mut(i));
        slot.and_then(|slot| slot.value.as_mut())
            .ok_or(Access::Stale)
    }

    /// Free the record `handle` names: drop it and move its slot's generation. Answers the bytes
    /// it owned, or `None` when the handle names no record.
    #[cfg(test)]
    pub(crate) fn free(&mut self, handle: Handle) -> Option<u64> {
        self.get(handle).ok()?;
        self.free_index(handle.index)
    }

    fn free_index(&mut self, index: u32) -> Option<u64> {
        let slot = usize::try_from(index)
            .ok()
            .and_then(|i| self.slots.get_mut(i))?;
        let value = slot.value.take()?;
        let bytes = std::mem::take(&mut slot.bytes);
        self.live = self.live.saturating_sub(1);
        #[cfg(test)]
        {
            self.owned = self.owned.saturating_sub(bytes);
        }
        if let Some(generation) = slot.generation.checked_add(1) {
            slot.generation = generation;
            // The free list was reserved to the capacity when the arena grew, so it has room
            // for every slot and this push never allocates.
            let room = self.free.len() < self.free.capacity();
            #[cfg(test)]
            if !room {
                self.growths = self.growths.saturating_add(1);
            }
            if generation < u32::MAX && room {
                self.free.push(index);
            }
        }
        drop(value);
        Some(bytes)
    }

    /// The number of slots, live or free, that have held a record.
    #[cfg(test)]
    pub(crate) fn slot_count(&self) -> usize {
        self.slots.len()
    }

    /// The bytes of the arena's room, which the live reading leaves out: the slots that hold no
    /// record, free or never used, and the workspace of every slot.
    pub(crate) fn room_bytes(&self) -> u64 {
        let capacity = u64::try_from(self.capacity).unwrap_or(u64::MAX);
        let live = u64::try_from(self.live).unwrap_or(u64::MAX);
        capacity
            .saturating_mul(slot_bytes::<T>())
            .saturating_sub(live.saturating_mul(record_slot_bytes::<T>()))
    }

    /// The counting build: the bytes the arena's capacity was charged, each slot with its
    /// workspace.
    #[cfg(test)]
    pub(crate) fn capacity_bytes(&self) -> u64 {
        u64::try_from(self.capacity)
            .unwrap_or(u64::MAX)
            .saturating_mul(slot_bytes::<T>())
    }

    /// Take the collector's workspace out of the arena for a mark phase; moves it, allocates
    /// nothing.
    pub(crate) fn take_workspace(&mut self) -> Workspace {
        std::mem::take(&mut self.workspace)
    }

    /// Give the workspace back after a mark phase, its worklist empty (the room stays); the
    /// mark words stay for the sweep.
    pub(crate) fn give_back_workspace(&mut self, mut workspace: Workspace) {
        workspace.work.clear();
        self.workspace = workspace;
    }

    /// Clear every mark word, in place.
    pub(crate) fn clear_marks(&mut self) {
        self.workspace.marks.fill(0);
    }

    /// Free every live record whose slot is not marked, in slot order, answer the bytes they
    /// owned, and clear the mark words. Dropping a record drops what it owns (host data,
    /// decision 15).
    pub(crate) fn sweep(&mut self) -> u64 {
        let mut credited: u64 = 0;
        for index in 0..self.slots.len() {
            let live = self
                .slots
                .get(index)
                .is_some_and(|slot| slot.value.is_some());
            if live && !self.workspace.marked(index) {
                let freed = u32::try_from(index).ok().and_then(|i| self.free_index(i));
                credited = credited.saturating_add(freed.unwrap_or(0));
            }
        }
        self.clear_marks();
        credited
    }
}

#[cfg(test)]
mod tests {
    use super::{Access, Arena, Handle};
    use crate::{
        Abrupt, Agent, AgentState, Brand, HostError, HostObjectSpec, Limits, Prototype, Seeds,
        Termination, Value,
    };

    use crate::host::hooks::test_host::{NoHooks, new_realm};

    fn agent() -> Agent<NoHooks> {
        let limits = Limits {
            steps_per_entry: 1_000,
            memory_bytes: 1 << 20,
            frame_depth: 100,
            reentry_depth: 10,
            parser_nesting: 100,
            string_length: 1 << 20,
            bigint_bits: 1 << 10,
        };
        let seeds = Seeds {
            hash_key: [1; 16],
            random_seed: [2; 32],
        };
        Agent::new(NoHooks, limits, seeds).unwrap()
    }

    /// The epic's item 11: a handle whose generation does not match its slot ends the run with
    /// `Terminated(EngineFault)`, and every later call on that agent answers
    /// `Err(HostError::Faulted)`.
    #[test]
    fn stale_generation_faults_the_agent() {
        // The arena: freeing a slot moves its generation, so the old handle reaches nothing,
        // also after the slot is used again; a handle of another key reaches nothing either.
        let mut arena = Arena::new(9);
        let first = arena.allocate("a", 1).unwrap();
        assert_eq!(arena.get(first), Ok(&"a"));
        assert_eq!(arena.free(first), Some(1));
        assert_eq!(arena.get(first), Err(Access::Stale));
        assert_eq!(arena.free(first), None);
        let second = arena.allocate("b", 1).unwrap();
        assert_eq!(second.index, first.index);
        assert_ne!(second.generation, first.generation);
        assert_eq!(arena.get(first), Err(Access::Stale));
        assert_eq!(arena.get(second), Ok(&"b"));
        assert_eq!(
            arena.get(Handle { key: 10, ..second }),
            Err(Access::Foreign)
        );
        assert_eq!(arena.get(Handle { index: 5, ..second }), Err(Access::Stale));

        // Inside an entry: the engine meets a stale handle of its own.
        let mut agent = agent();
        let realm = new_realm(&mut agent);
        let (outcome, later_call, later_read) = agent
            .with_cx(&realm, |cx| {
                let object = cx
                    .new_host_object(HostObjectSpec {
                        prototype: Prototype::Null,
                        brand: Brand(1),
                        data: None,
                    })
                    .unwrap()
                    .unwrap();
                let string = cx.new_string("s").unwrap();
                cx.agent.heap.objects.free(object.handle);
                let outcome = cx.agent.engine_object(object).map(|_| ());
                (outcome, cx.new_string("t").map(|_| ()), cx.to_utf16(string))
            })
            .unwrap();
        assert_eq!(outcome, Err(Abrupt::Terminated(Termination::EngineFault)));
        assert_eq!(
            later_call,
            Err(Abrupt::Terminated(Termination::EngineFault))
        );
        assert_eq!(later_read, Err(HostError::Faulted));

        assert_eq!(agent.state(), AgentState::Faulted);
        assert!(matches!(
            agent.with_cx(&realm, |_| ()),
            Err(HostError::Faulted)
        ));
        assert!(matches!(agent.collect(), Err(HostError::Faulted)));
        assert_eq!(agent.heap_bytes(), Err(HostError::Faulted));

        // A root made before the fault cannot be duplicated after it.
        let mut other = self::agent();
        let other_realm = new_realm(&mut other);
        let root = other
            .with_cx(&other_realm, |cx| cx.root(Value::Null).unwrap())
            .unwrap();
        assert!(matches!(agent.duplicate(&root), Err(HostError::Faulted)));
    }

    /// The memory budget covers the arenas: ten rounds of 10,000 objects allocated with none
    /// kept, each followed by a collection. After every allocation and every collection the
    /// accounting holds at least the counting build's floor, the capacities of every arena's
    /// slots, free list, mark words and worklist times their element sizes plus what the live
    /// records own, and the floor's high-water mark stays within `memory_bytes`. Prints the
    /// bytes of one slot of each arena.
    #[test]
    fn the_ledger_covers_the_arenas_over_repeated_allocation_and_collection() {
        use super::Slot;
        use crate::Outcome;
        use crate::env::EnvironmentRecord;
        use crate::heap::SymbolRecord;
        use crate::heap::string::HeapString;
        use crate::host::hooks::test_host;
        use crate::object::ordinary::Ordinary;
        use crate::object::{ObjectKind, ObjectRecord};
        use crate::realm::RealmRecord;
        use crate::script::ScriptRecord;

        println!(
            "one slot: strings {} symbols {} objects {} realms {} environments {} scripts {}",
            size_of::<Slot<HeapString>>(),
            size_of::<Slot<SymbolRecord>>(),
            size_of::<Slot<ObjectRecord>>(),
            size_of::<Slot<RealmRecord>>(),
            size_of::<Slot<EnvironmentRecord>>(),
            size_of::<Slot<ScriptRecord>>(),
        );
        let mut limits = test_host::limits();
        limits.memory_bytes = 1 << 26;
        let mut agent = test_host::agent_with(limits);
        let mut high: u64 = 0;
        let mut check = |agent: &Agent<NoHooks>, round: usize, at: usize| {
            let floor = agent.heap.ledger_floor();
            let used = agent.heap.accounting.used();
            assert!(
                used >= floor,
                "round {round}, step {at}: used {used} is below the arenas' floor {floor}"
            );
            high = high.max(floor);
        };
        for round in 0..10 {
            for at in 0..10_000 {
                let record = ObjectRecord::new(None, ObjectKind::Ordinary(Ordinary::for_test()));
                agent.heap.new_object(record, 0).unwrap();
                check(&agent, round, at);
            }
            assert!(matches!(agent.collect(), Ok(Outcome::Normal(_))));
            check(&agent, round, 10_000);
        }
        assert!(high <= limits.memory_bytes, "high-water mark {high}");
    }
}
