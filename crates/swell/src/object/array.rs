//! Array exotic objects (ECMA-262 10.4.2, `#sec-array-exotic-objects`). The element storage, the
//! internal methods and the trace are filled by `story:object-model-and-realms`.

use crate::heap::collect::Mark;

/// The slots of an Array. None before `story:object-model-and-realms`.
pub(crate) struct Array {}

impl Array {
    /// Kind site of the collector: reports nothing until the slots exist.
    pub(crate) fn trace(&self, mark: &mut Mark) {
        mark.count("object/array.rs Array::trace");
    }

    /// An Array, for the tests of the collector.
    #[cfg(test)]
    pub(crate) fn for_test() -> Array {
        Array {}
    }
}
