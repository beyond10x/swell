//! Global Environment Records (ECMA-262 9.1.1.4, `#sec-global-environment-records`), and the one
//! a realm holds as [[GlobalEnv]] ([`RealmGlobalEnv`]: its constructor and its trace). Filled by
//! `story:interpreter-and-script-entry-points`; the realm record of
//! `story:object-model-and-realms` holds the field and calls both.

use crate::heap::collect::Mark;

/// The global environment a realm holds. Empty before
/// `story:interpreter-and-script-entry-points`.
pub(crate) struct RealmGlobalEnv {}

impl RealmGlobalEnv {
    /// Site of the collector, called by the realm record's trace: the object record and the
    /// declarative record of the global environment once they exist.
    pub(crate) fn trace(&self, mark: &mut Mark) {
        mark.count("env/global.rs RealmGlobalEnv::trace");
    }

    /// A global environment, for the tests of the collector.
    #[cfg(test)]
    pub(crate) fn for_test() -> RealmGlobalEnv {
        RealmGlobalEnv {}
    }
}
