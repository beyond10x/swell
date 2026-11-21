//! What an agent is created with (`docs/design/host-interface.md`, decisions 2 and 3): its
//! limits, its seeds, and the one way creating it can fail. Written by
//! `story:heap-values-and-host-surface`; `Limits::min_stack_bytes` is
//! `story:interpreter-and-script-entry-points`.

use std::fmt;

/// The limits of an agent (decision 2). Each has a unit, a meaning at 0 and one outcome when it
/// is reached; 0 is a limit of zero, never "no limit", and a host that wants no practical limit
/// gives the type's maximum.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    /// Steps of one outermost entry, with every nested entry: `Terminated(StepBudget)`.
    pub steps_per_entry: u64,
    /// Bytes charged to the agent's heap over its life, a sweep crediting:
    /// `Terminated(MemoryBudget)`.
    pub memory_bytes: u64,
    /// ECMAScript execution contexts on the engine's frame stack: a `RangeError`.
    pub frame_depth: u32,
    /// Calls from Rust into code nested beyond the outermost entry: a `RangeError`.
    pub reentry_depth: u32,
    /// Nesting levels of the parser: a `SyntaxError` whose message names `parser_nesting`.
    pub parser_nesting: u32,
    /// UTF-16 code units of a String created at run time from other values: a `RangeError`.
    pub string_length: u64,
    /// Bits of a BigInt's magnitude computed at run time: a `RangeError`.
    pub bigint_bits: u64,
}

/// The two seeds of an agent (decision 3), and the only randomness the engine has.
/// `Agent::new` is the only function of the interface that takes them (HOST-146).
#[derive(Clone, Copy)]
pub struct Seeds {
    /// Keys the SipHash of the engine's hash maps (HOST-145).
    pub hash_key: [u8; 16],
    /// Seeds `Math.random`, each realm's generator taking it with the realm's creation index as
    /// its stream (slice 4).
    pub random_seed: [u8; 32],
}

impl fmt::Debug for Seeds {
    /// Never prints the seeds.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Seeds { .. }")
    }
}

/// Why `Agent::new` made no agent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CreateError {
    /// Every handle carries its agent's key, a `u32` from a process-wide counter; this process
    /// has created 2^32 - 1 agents.
    AgentKeysExhausted,
}
