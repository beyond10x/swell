//! Bound function exotic objects (ECMA-262 10.4.1, `#sec-bound-function-exotic-objects`):
//! [[BoundTargetFunction]], [[BoundThis]] and [[BoundArguments]]. Created with its call arms by
//! `story:object-model-and-realms`, filled by `story:harness-builtins`.

use crate::heap::collect::Mark;

/// The slots of a bound function. None before `story:harness-builtins`.
pub(crate) struct BoundFunction {}

impl BoundFunction {
    /// Kind site of the collector: reports nothing until the target, `this` and the arguments
    /// exist.
    pub(crate) fn trace(&self, mark: &mut Mark) {
        mark.count("object/bound_function.rs BoundFunction::trace");
    }

    /// A bound function, for the tests of the collector.
    #[cfg(test)]
    pub(crate) fn for_test() -> BoundFunction {
        BoundFunction {}
    }
}
