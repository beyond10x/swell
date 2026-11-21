//! ECMAScript function objects (ECMA-262 10.2, `#sec-ecmascript-function-objects`):
//! [[Environment]], [[Realm]], [[ScriptOrModule]], [[HomeObject]] and the compiled code. Created
//! by `story:object-model-and-realms`, filled by `story:interpreter-and-script-entry-points`.

use crate::heap::collect::Mark;

/// The slots of an ECMAScript function. None before
/// `story:interpreter-and-script-entry-points`.
pub(crate) struct EcmascriptFunction {}

impl EcmascriptFunction {
    /// Kind site of the collector: reports nothing until the environment, realm, script and
    /// home object exist.
    pub(crate) fn trace(&self, mark: &mut Mark) {
        mark.count("object/ecmascript_function.rs EcmascriptFunction::trace");
    }

    /// An ECMAScript function, for the tests of the collector.
    #[cfg(test)]
    pub(crate) fn for_test() -> EcmascriptFunction {
        EcmascriptFunction {}
    }
}
