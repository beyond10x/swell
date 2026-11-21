//! Ordinary objects (ECMA-262 10.1, `#sec-ordinary-object-internal-methods-and-internal-slots`),
//! with the Boolean and Number wrappers ([[BooleanData]], [[NumberData]]). The slots, the
//! ordinary internal methods and the trace are filled by `story:object-model-and-realms`.

use crate::heap::collect::Mark;

/// The slots of an ordinary object beyond the common record. None before
/// `story:object-model-and-realms`.
pub(crate) struct Ordinary {}

impl Ordinary {
    /// Kind site of the collector: reports nothing until the slots exist.
    pub(crate) fn trace(&self, mark: &mut Mark) {
        mark.count("object/ordinary.rs Ordinary::trace");
    }

    /// An ordinary object, for the tests of the collector.
    #[cfg(test)]
    pub(crate) fn for_test() -> Ordinary {
        Ordinary {}
    }
}
