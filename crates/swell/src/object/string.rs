//! String exotic objects (ECMA-262 10.4.3, `#sec-string-exotic-objects`): [[StringData]], the
//! internal methods and the trace, filled by `story:object-model-and-realms`.

use crate::heap::collect::Mark;

/// The slots of a String object. None before `story:object-model-and-realms`.
pub(crate) struct StringObject {}

impl StringObject {
    /// Kind site of the collector: reports nothing until [[StringData]] exists.
    pub(crate) fn trace(&self, mark: &mut Mark) {
        mark.count("object/string.rs StringObject::trace");
    }

    /// A String object, for the tests of the collector.
    #[cfg(test)]
    pub(crate) fn for_test() -> StringObject {
        StringObject {}
    }
}
