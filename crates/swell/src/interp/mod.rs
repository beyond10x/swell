//! The interpreter (`docs/design/engine.md`, decision 3; ECMA-262 9.4, `#sec-execution-contexts`):
//! a stack-based bytecode interpreter with every ECMAScript frame on the engine's own frame
//! stack, the step budget and the depth bounds. Filled by
//! `story:interpreter-and-script-entry-points`.
//!
//! Root site of the collector: [`trace_roots`] (the frame stack: operand stacks, locals,
//! environments, function and realm of each frame), which reports nothing until frames exist.

pub(crate) mod budget;
pub(crate) mod dispatch;
pub(crate) mod frame;

use crate::heap::collect::Mark;
use crate::host::Host;
use crate::host::agent::Agent;

/// Root site of the collector: the frame stack. None before
/// `story:interpreter-and-script-entry-points`.
pub(crate) fn trace_roots<H: Host>(_agent: &Agent<H>, mark: &mut Mark) {
    mark.count("interp/mod.rs trace_roots");
}
