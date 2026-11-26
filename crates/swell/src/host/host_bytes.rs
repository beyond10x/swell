//! The host's own bytes on the memory budget (`docs/design/host-interface.md`, section 14, "The
//! host's own work and bytes"; decision 2, `memory_bytes`; HOST-175, HOST-176): [`HostBytes`],
//! what `charge_host_bytes` answers, and the agent's side of `charge_host_bytes` and
//! `resize_host_bytes`. Written by `story:the-host-charges-its-own-bytes-and-steps`.
//!
//! A charge goes through the one accounting of the memory budget (`heap/accounting.rs`); there
//! is no second budget. A dropped `HostBytes` adds its bytes to a queue of its agent, an
//! `Rc<Cell<u64>>` it holds a clone of, as a freed property map's `Grown` does
//! (`object/mod.rs`): it touches nothing else, so a drop in a sweep runs no code of the agent and
//! allocates nothing, and a drop after its agent faulted or was dropped is a store nobody reads.
//! The agent credits the queue after each sweep (`Agent::collect_now`) and before each charge
//! here, so a credit is always the bytes of one charge, as resized, given back once.

use alloc::rc::Rc;
use core::cell::Cell;
use core::cmp::Ordering;
use core::fmt;

use super::Host;
use super::agent::Agent;
use super::outcome::{Completion, HostError, Termination};

/// Bytes the host charged for the agent with `charge_host_bytes`. Neither `Clone` nor `Copy`,
/// no public method; dropping it gives its bytes back to the agent that charged them, after
/// that agent's next sweep or at its next charge of host bytes, and `Agent::credit_host_bytes`
/// gives them back at once. One kept for good keeps its bytes charged for the agent's life.
pub struct HostBytes {
    /// The bytes this handle holds charged.
    pub(crate) bytes: u64,
    /// The key of the agent that charged them.
    pub(crate) key: u32,
    /// That agent's queue of host credits (`Agent::host_credit`).
    queue: Rc<Cell<u64>>,
}

impl fmt::Debug for HostBytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HostBytes")
            .field("bytes", &self.bytes)
            .finish_non_exhaustive()
    }
}

impl Drop for HostBytes {
    /// Add the bytes to the agent's queue, with a saturating add: no panic and no allocation.
    fn drop(&mut self) {
        self.queue.set(self.queue.get().saturating_add(self.bytes));
    }
}

impl<H: Host> Agent<H> {
    /// Credit the host's charges whose `HostBytes` were dropped since the last credit.
    pub(crate) fn credit_dropped_host_bytes(&mut self) {
        let dropped = self.host_credit.replace(0);
        self.heap.accounting.credit(dropped);
    }

    /// `charge_host_bytes` of `Cx` and `HookCx`: the sticky termination, the dropped charges
    /// credited, then `bytes` charged. Over the budget nothing is charged and the run ends in
    /// `Terminated(MemoryBudget)`. Not a safe point.
    pub(crate) fn scoped_charge_host_bytes(&mut self, bytes: u64) -> Completion<HostBytes> {
        self.running()?;
        self.credit_dropped_host_bytes();
        if self.heap.accounting.charge(bytes).is_err() {
            return Err(self.terminate(Termination::MemoryBudget));
        }
        Ok(HostBytes {
            bytes,
            key: self.heap.key,
            queue: Rc::clone(&self.host_credit),
        })
    }

    /// `resize_host_bytes` of `Cx` and `HookCx`: a handle of another agent answers
    /// `Err(ForeignHandle)` before the sticky termination (decision 5). Growing credits the
    /// dropped charges and charges the difference; over the budget nothing is charged, the
    /// handle keeps its bytes and the run ends in `Terminated(MemoryBudget)`. Shrinking credits
    /// the difference at once. Not a safe point.
    pub(crate) fn scoped_resize_host_bytes(
        &mut self,
        charge: &mut HostBytes,
        bytes: u64,
    ) -> Result<Completion<()>, HostError> {
        self.alive()?;
        if charge.key != self.heap.key {
            return Err(HostError::ForeignHandle);
        }
        Ok(self.running().and_then(|()| {
            match bytes.cmp(&charge.bytes) {
                Ordering::Greater => {
                    self.credit_dropped_host_bytes();
                    let more = bytes.saturating_sub(charge.bytes);
                    if self.heap.accounting.charge(more).is_err() {
                        return Err(self.terminate(Termination::MemoryBudget));
                    }
                }
                Ordering::Less => {
                    let less = charge.bytes.saturating_sub(bytes);
                    self.heap.accounting.credit(less);
                }
                Ordering::Equal => {}
            }
            charge.bytes = bytes;
            Ok(())
        }))
    }
}
