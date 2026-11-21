//! The interrupt handle (`docs/design/host-interface.md`, decision 12): a store into an
//! `Arc<AtomicBool>` that any thread may make. The engine reads the flag where it checks the
//! step budget and at the start of every outermost entry, and clears it when it ends a run with
//! `Terminated(Interrupted)`; those checks are `story:object-model-and-realms` and
//! `story:interpreter-and-script-entry-points`. Setting the flag of a faulted agent, or of one
//! that was dropped, is a store nobody reads. Written by `story:heap-values-and-host-surface`.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// The interrupt flag of one agent. `Send` and `Sync`; every clone shares the flag.
#[derive(Clone, Debug, Default)]
pub struct InterruptHandle {
    flag: Arc<AtomicBool>,
}

impl InterruptHandle {
    /// Ask the running entry, or the next one, to end with `Terminated(Interrupted)`.
    pub fn interrupt(&self) {
        self.flag.store(true, Ordering::SeqCst);
    }

    /// Withdraw an interrupt not yet used.
    pub fn clear(&self) {
        self.flag.store(false, Ordering::SeqCst);
    }

    /// Whether an interrupt is set and not yet used.
    pub fn is_set(&self) -> bool {
        self.flag.load(Ordering::SeqCst)
    }
}
