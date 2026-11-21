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

/// The Rust stack, in bytes, one re-entry into the interpreter may take: the frames from a
/// builtin's or a host function's call into a function, or from an instruction's call of a
/// getter, through [[Call]] and the run loop to the next such call (`interp/mod.rs`). Measured
/// on x86_64 in the test build without incremental compilation and without a compiler cache, as
/// the forge's gate builds, with the script of `min_stack_bytes_is_enough` (`tests/budgets.rs`):
/// the smallest thread stack it ends on is 25,512,840 bytes at `reentry_depth` 1,000 and
/// 5,212,717 at 200, 25,375 bytes per re-entry; this is that plus a fifth, for the frames of
/// another compiler version.
const STACK_BYTES_PER_REENTRY: usize = 30 * 1024;

/// The Rust stack, in bytes, an agent takes besides its re-entries and its parser: the host's
/// entry point and the outermost run loop, and the thread's own start. The measurement above
/// leaves nothing for it (the parser's stated need covers it with room), so it is a margin.
const STACK_BYTES_FIXED: usize = 64 * 1024;

impl Limits {
    /// The Rust stack, in bytes, a thread must have for an agent with these limits (decision 2):
    /// the parser's need at `parser_nesting` (`swell_syntax::ParseOptions::stack_bytes`), which
    /// a host function may reach at the deepest re-entry through `Cx::parse_script`, plus
    /// `reentry_depth + 1` times the stack one re-entry takes (the outermost run and every
    /// re-entry), plus a fixed part. HOST-17.
    pub const fn min_stack_bytes(&self) -> usize {
        let parser = swell_syntax::ParseOptions {
            max_nesting: self.parser_nesting,
        }
        .stack_bytes();
        let runs = (self.reentry_depth as usize).saturating_add(1);
        STACK_BYTES_FIXED
            .saturating_add(parser)
            .saturating_add(runs.saturating_mul(STACK_BYTES_PER_REENTRY))
    }
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
