//! The test host and the limits and seeds the tests share.

use swell::{Agent, CreateError, Host, Limits, Seeds};

/// A host that records every hook call, in order, as the hook's name and its arguments.
///
/// The `Host` trait has no hook until `story:object-model-and-realms`, so the record stays empty
/// here; a test that reads it shows that no hook was called.
#[derive(Debug, Default)]
pub struct TestHost {
    pub calls: Vec<String>,
}

impl Host for TestHost {}

/// The limits the tests use unless a test sets one of its own: large enough that no test of
/// this suite reaches one by accident.
pub fn limits() -> Limits {
    Limits {
        steps_per_entry: 1_000_000,
        memory_bytes: 64 * 1024 * 1024,
        frame_depth: 10_000,
        reentry_depth: 256,
        parser_nesting: 1_000,
        string_length: 1 << 30,
        bigint_bits: 1 << 20,
    }
}

/// The seeds the tests use: fixed, so every run is the same.
pub fn seeds() -> Seeds {
    Seeds {
        hash_key: [7; 16],
        random_seed: [11; 32],
    }
}

/// An agent with the test host, [`limits`] and [`seeds`].
pub fn agent() -> Result<Agent<TestHost>, CreateError> {
    Agent::new(TestHost::default(), limits(), seeds())
}

/// An agent with the test host, the given limits and [`seeds`].
pub fn agent_with(limits: Limits) -> Result<Agent<TestHost>, CreateError> {
    Agent::new(TestHost::default(), limits, seeds())
}
