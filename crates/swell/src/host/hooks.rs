//! The hook trait (`docs/design/host-interface.md`, decision 16; ECMA-262 Annex D,
//! `#sec-host-hooks-summary`). The agent owns the host value; a hook is an associated function
//! that receives a context, not `&mut self` (decision 1). The trait has no method until
//! `story:object-model-and-realms` adds the hooks and their defaults; the job and load types it
//! adds here (`Job`, `LoadRequest`) are public through the glob of `host/mod.rs`.

/// The host of an agent: the value that answers the hooks.
pub trait Host: Sized + 'static {}
