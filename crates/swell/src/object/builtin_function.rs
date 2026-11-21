//! Built-in function objects (ECMA-262 10.3, `#sec-built-in-function-objects`): [[Realm]],
//! [[InitialName]], [[Call]] and [[Construct]] of builtins and host functions, and the slots in
//! which a running builtin keeps the values it needs across a call into code. Filled by
//! `story:object-model-and-realms`.
//!
//! Two sites of the collector: the kind site [`BuiltinFunction::trace`] and the root site
//! [`trace_roots`] (the slots of running builtins). Both report nothing until their owner lands.

use crate::heap::collect::Mark;
use crate::host::Host;
use crate::host::agent::Agent;

/// The slots of a built-in function object. None before `story:object-model-and-realms`.
pub(crate) struct BuiltinFunction {}

impl BuiltinFunction {
    /// Kind site of the collector: reports nothing until [[Realm]] and [[InitialName]] exist.
    pub(crate) fn trace(&self, mark: &mut Mark) {
        mark.count("object/builtin_function.rs BuiltinFunction::trace");
    }

    /// A built-in function object, for the tests of the collector.
    #[cfg(test)]
    pub(crate) fn for_test() -> BuiltinFunction {
        BuiltinFunction {}
    }
}

/// Root site of the collector: the values running builtins keep across a call into code. None
/// before `story:object-model-and-realms`.
pub(crate) fn trace_roots<H: Host>(_agent: &Agent<H>, mark: &mut Mark) {
    mark.count("object/builtin_function.rs trace_roots");
}
