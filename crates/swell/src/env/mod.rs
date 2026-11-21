//! Environment Records (ECMA-262 9.1, `#sec-environment-records`): declarative, object, global
//! and function Environment Records. Filled by `story:interpreter-and-script-entry-points`.
//!
//! Kind site of the collector: [`EnvironmentRecord::trace`], which reports nothing until the
//! bindings exist.

pub(crate) mod declarative;
pub(crate) mod function;
pub(crate) mod global;
pub(crate) mod object;

use crate::heap::collect::Mark;

/// An Environment Record. None before `story:interpreter-and-script-entry-points`.
pub(crate) struct EnvironmentRecord {}

impl EnvironmentRecord {
    /// Kind site of the collector: the outer environment and the bindings once they exist.
    pub(crate) fn trace(&self, mark: &mut Mark) {
        mark.count("env/mod.rs EnvironmentRecord::trace");
    }

    /// An Environment Record, for the tests of the collector.
    #[cfg(test)]
    pub(crate) fn for_test() -> EnvironmentRecord {
        EnvironmentRecord {}
    }
}
