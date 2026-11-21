//! Scripts (ECMA-262 16.1, `#sec-scripts`): the Script Record (`#sec-script-records`: its realm
//! and its compiled code with their constants), ParseScript and ScriptEvaluation. Filled by
//! `story:interpreter-and-script-entry-points`.
//!
//! Kind site of the collector: [`ScriptRecord::trace`], which reports nothing until the record
//! has its realm and constants.

use crate::heap::collect::Mark;

/// A Script Record. Empty before `story:interpreter-and-script-entry-points`.
pub(crate) struct ScriptRecord {}

impl ScriptRecord {
    /// Kind site of the collector: [[Realm]] and the code's constants once they exist.
    pub(crate) fn trace(&self, mark: &mut Mark) {
        mark.count("script.rs ScriptRecord::trace");
    }

    /// A Script Record, for the tests of the collector.
    #[cfg(test)]
    pub(crate) fn for_test() -> ScriptRecord {
        ScriptRecord {}
    }
}
