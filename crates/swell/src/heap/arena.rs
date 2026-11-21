//! Arenas of slots, one per kind of heap record, and the handles into them (`docs/design/engine.md`,
//! decision 4, "Heap"; `docs/design/host-interface.md`, decision 4).
//!
//! A handle is the key of the agent that made it, a slot index and the slot's generation. Freeing
//! a record moves its slot's generation, so a handle kept past its record's life reaches nothing:
//! every access compares the key and the generation and answers [`Access`] instead of reading
//! another record. Inside the engine that answer ends the run in `Terminated(EngineFault)`; from
//! the host it is `Err(HostError::StaleHandle)` or `Err(HostError::ForeignHandle)`. A slot whose
//! generation reaches `u32::MAX` is retired, so a generation is never used twice.

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
    /// What the record was charged, credited when it is freed.
    bytes: u64,
}

/// The bytes a slot of an arena of `T` costs, beside what its record owns.
pub(crate) fn slot_bytes<T>() -> u64 {
    u64::try_from(std::mem::size_of::<Slot<T>>()).unwrap_or(u64::MAX)
}

/// The records of one kind. Freed slots are used again, most recently freed first, so the same
/// calls give the same handles.
pub(crate) struct Arena<T> {
    key: u32,
    slots: Vec<Slot<T>>,
    free: Vec<u32>,
}

impl<T> Arena<T> {
    /// An empty arena of the agent `key`.
    pub(crate) fn new(key: u32) -> Arena<T> {
        Arena {
            key,
            slots: Vec::new(),
            free: Vec::new(),
        }
    }

    /// Store `value`, which was charged `bytes`, and answer its handle. A new slot is reserved
    /// with `Vec::try_reserve`, so a refusal of the allocator is an answer, not an abort.
    pub(crate) fn allocate(&mut self, value: T, bytes: u64) -> Result<Handle, Full> {
        if let Some(index) = self.free.pop()
            && let Some(slot) = usize::try_from(index)
                .ok()
                .and_then(|i| self.slots.get_mut(i))
        {
            slot.value = Some(value);
            slot.bytes = bytes;
            return Ok(Handle {
                key: self.key,
                index,
                generation: slot.generation,
            });
        }
        let index = u32::try_from(self.slots.len()).map_err(|_| Full)?;
        self.slots.try_reserve(1).map_err(|_| Full)?;
        self.slots.push(Slot {
            generation: 0,
            value: Some(value),
            bytes,
        });
        Ok(Handle {
            key: self.key,
            index,
            generation: 0,
        })
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
    /// it was charged, or `None` when the handle names no record.
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
        if let Some(generation) = slot.generation.checked_add(1) {
            slot.generation = generation;
            if generation < u32::MAX {
                self.free.push(index);
            }
        }
        drop(value);
        Some(bytes)
    }

    /// The number of slots, live or free: the length of a mark bitmap of this arena.
    pub(crate) fn slot_count(&self) -> usize {
        self.slots.len()
    }

    /// Free every live record whose slot `marked` does not mark, in slot order, and answer the
    /// bytes they were charged. Dropping a record drops what it owns (host data, decision 15).
    pub(crate) fn sweep(&mut self, marked: &[bool]) -> u64 {
        let mut credited: u64 = 0;
        for index in 0..self.slots.len() {
            let live = self
                .slots
                .get(index)
                .is_some_and(|slot| slot.value.is_some());
            let kept = marked.get(index).copied().unwrap_or(false);
            if live && !kept {
                let freed = u32::try_from(index).ok().and_then(|i| self.free_index(i));
                credited = credited.saturating_add(freed.unwrap_or(0));
            }
        }
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
}
