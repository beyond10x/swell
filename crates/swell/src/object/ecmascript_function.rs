//! ECMAScript function objects (ECMA-262 10.2, `#sec-ecmascript-function-objects`):
//! [[Environment]], [[Realm]], [[ScriptOrModule]], [[HomeObject]] and the compiled code.
//! `story:object-model-and-realms` writes the dispatch arms of `object/mod.rs` and the final
//! signatures of [[Call]] (10.2.1) and [[Construct]] (10.2.2) here;
//! `story:interpreter-and-script-entry-points` replaces the struct, the bodies and the trace
//! function.
//!
//! No code can create an ECMAScript function before that story: the struct has no constructor,
//! so reaching a body here is a broken engine invariant, and the run ends in
//! `Terminated(EngineFault)`.

use crate::heap::collect::Mark;
use crate::host::Host;
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::value::{ObjectRef, Value};

/// The slots of an ECMAScript function. None before
/// `story:interpreter-and-script-entry-points`.
pub(crate) struct EcmascriptFunction {}

impl EcmascriptFunction {
    /// Kind site of the collector: reports nothing until the environment, realm, script and
    /// home object exist.
    pub(crate) fn trace(&self, mark: &mut Mark) {
        mark.count("object/ecmascript_function.rs EcmascriptFunction::trace");
    }

    /// An ECMAScript function object has a [[Call]].
    pub(crate) fn has_call(&self) -> bool {
        true
    }

    /// Whether it has a [[Construct]]: decided by its kind, which does not exist yet.
    pub(crate) fn has_construct(&self) -> bool {
        false
    }

    /// An ECMAScript function, for the tests of the collector.
    #[cfg(test)]
    pub(crate) fn for_test() -> EcmascriptFunction {
        EcmascriptFunction {}
    }
}

/// [[Call]] (`#sec-ecmascript-function-objects-call-thisargument-argumentslist`).
pub(crate) fn call<H: Host>(
    agent: &mut Agent<H>,
    _function: ObjectRef,
    _this: Value,
    _args: &[Value],
) -> Completion<Value> {
    Err(agent.fault())
}

/// [[Construct]] (`#sec-ecmascript-function-objects-construct-argumentslist-newtarget`).
pub(crate) fn construct<H: Host>(
    agent: &mut Agent<H>,
    _function: ObjectRef,
    _args: &[Value],
    _new_target: ObjectRef,
) -> Completion<Value> {
    Err(agent.fault())
}
