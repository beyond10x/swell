//! Bound function exotic objects (ECMA-262 10.4.1, `#sec-bound-function-exotic-objects`):
//! [[BoundTargetFunction]], [[BoundThis]] and [[BoundArguments]]. `story:object-model-and-realms`
//! writes the dispatch arms of `object/mod.rs` and the final signatures of [[Call]] (10.4.1.1)
//! and [[Construct]] (10.4.1.2) here; `story:harness-builtins` replaces the struct, the bodies
//! and the trace function.
//!
//! No code can create a bound function before that story: the struct has no constructor, so
//! reaching a body here is a broken engine invariant, and the run ends in
//! `Terminated(EngineFault)`.

use crate::heap::collect::Mark;
use crate::host::Host;
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::value::{ObjectRef, Value};

/// The slots of a bound function. None before `story:harness-builtins`.
pub(crate) struct BoundFunction {}

impl BoundFunction {
    /// Kind site of the collector: reports nothing until the target, `this` and the arguments
    /// exist.
    pub(crate) fn trace(&self, mark: &mut Mark) {
        mark.count("object/bound_function.rs BoundFunction::trace");
    }

    /// A bound function exotic object has a [[Call]].
    pub(crate) fn has_call(&self) -> bool {
        true
    }

    /// Whether it has a [[Construct]]: its target's, which does not exist yet.
    pub(crate) fn has_construct(&self) -> bool {
        false
    }

    /// A bound function, for the tests of the collector.
    #[cfg(test)]
    pub(crate) fn for_test() -> BoundFunction {
        BoundFunction {}
    }
}

/// [[Call]] (`#sec-bound-function-exotic-objects-call-thisargument-argumentslist`).
pub(crate) fn call<H: Host>(
    agent: &mut Agent<H>,
    _function: ObjectRef,
    _this: Value,
    _args: &[Value],
) -> Completion<Value> {
    Err(agent.fault())
}

/// [[Construct]] (`#sec-bound-function-exotic-objects-construct-argumentslist-newtarget`).
pub(crate) fn construct<H: Host>(
    agent: &mut Agent<H>,
    _function: ObjectRef,
    _args: &[Value],
    _new_target: ObjectRef,
) -> Completion<Value> {
    Err(agent.fault())
}
