//! Realms (ECMA-262 9.3, `#sec-code-realms`): the Realm Record (its intrinsics, global object,
//! global `this` value and global environment), `create_realm` with InitializeHostDefinedRealm
//! (`#sec-initializehostdefinedrealm`), and the agent-wide values (the well-known symbols,
//! `#sec-well-known-symbols`). Filled by `story:object-model-and-realms`.
//!
//! Two sites of the collector: the kind site [`RealmRecord::trace`] and the root site
//! [`trace_roots`] (the agent-wide values). Both report nothing until their owner lands, but the
//! global environment the realm record holds (`env/global.rs`).

pub(crate) mod global_object;
pub(crate) mod intrinsics;

use crate::env::global::RealmGlobalEnv;
use crate::heap::collect::Mark;
use crate::host::Host;
use crate::host::agent::Agent;

/// A Realm Record. Only the global environment before `story:object-model-and-realms`.
pub(crate) struct RealmRecord {
    /// [[GlobalEnv]].
    pub(crate) global_env: RealmGlobalEnv,
}

impl RealmRecord {
    /// Kind site of the collector: the global environment; the intrinsics, the global object and
    /// the global `this` value once they exist.
    pub(crate) fn trace(&self, mark: &mut Mark) {
        mark.count("realm/mod.rs RealmRecord::trace");
        self.global_env.trace(mark);
    }

    /// A realm record, for the tests of the collector.
    #[cfg(test)]
    pub(crate) fn for_test() -> RealmRecord {
        RealmRecord {
            global_env: RealmGlobalEnv::for_test(),
        }
    }
}

/// Root site of the collector: the agent-wide values. None before
/// `story:object-model-and-realms`.
pub(crate) fn trace_roots<H: Host>(_agent: &Agent<H>, mark: &mut Mark) {
    mark.count("realm/mod.rs trace_roots");
}
