//! Outcomes and the result type of each entry point (`docs/design/host-interface.md`, decisions
//! 5, 6, 13 and 18): `Err(HostError)` for a call the host should not have made, `Ok` with a
//! result type of its own taken from the draft's return type. Written by
//! `story:heap-values-and-host-surface`.

use super::hooks::HostDefined;
use crate::host::ErrorKind;
use crate::value::{Root, Value};

/// Why a run ended without a completion (decision 2 and decision 10).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Termination {
    /// The entry spent `steps_per_entry`.
    StepBudget,
    /// An allocation would take the heap over `memory_bytes`.
    MemoryBudget,
    /// The interrupt handle was set.
    Interrupted,
    /// The engine found one of its own invariants broken; the agent is faulted for good.
    EngineFault,
}

/// The result of an entry point that runs code.
#[derive(Debug)]
pub enum Outcome<T> {
    Normal(T),
    Throw(Root),
    Terminated(Termination),
}

/// The result of a parse: ParseScript "returns a Script Record or a non-empty List of
/// SyntaxError objects" (`#sec-parse-script`).
#[derive(Debug)]
pub enum Parsed<T> {
    Parsed(T),
    Errors(Vec<Root>),
    Terminated(Termination),
}

/// An abrupt completion inside a scope: a thrown value, or the end of the run.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Abrupt {
    Throw(Value),
    Terminated(Termination),
}

/// The result of a call made inside a scope (`Cx`, `HookCx`, a host function).
pub type Completion<T> = Result<T, Abrupt>;

/// A call the host should not have made (decision 6). It changes nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HostError {
    /// The agent ended a run in `Terminated(EngineFault)`; it answers nothing but `state()`.
    Faulted,
    /// A handle whose record was freed, or a released root.
    StaleHandle,
    /// A handle or root of another agent.
    ForeignHandle,
    /// A value of the wrong kind where the interface needs a kind.
    WrongKind { expected: ValueKind },
    /// A module whose [[Status]] the call does not allow.
    ModuleStatus { found: ModuleStatus },
    /// A module whose graph holds a module cut by a termination (decision 10).
    ModuleTerminated,
    /// An Evaluate while another one runs in the agent.
    EvaluationInProgress,
}

/// The kind `HostError::WrongKind` expected.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ValueKind {
    Function,
    Constructor,
    Object,
    Promise,
    String,
}

/// A module's [[Status]] (`#sec-cyclic-module-records`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ModuleStatus {
    New,
    Unlinked,
    Linking,
    Linked,
    Evaluating,
    EvaluatingAsync,
    Evaluated,
}

/// Where the error an outermost entry answered arose (decision 13): the first error of a parse
/// that answered `Parsed::Errors`, or the instruction where the throw an entry answered arose.
/// `line` and `column` count from 1, a column in UTF-16 code units.
#[derive(Clone)]
pub struct ErrorSite {
    pub line: u32,
    pub column: u32,
    /// The position is in eval code or in the source text of a `Function` constructor,
    /// counted in that code's own text.
    pub dynamic: bool,
    /// The [[HostDefined]] of the script record whose code raised it.
    pub host_defined: Option<HostDefined>,
}

impl std::fmt::Debug for ErrorSite {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ErrorSite")
            .field("line", &self.line)
            .field("column", &self.column)
            .field("dynamic", &self.dynamic)
            .field("host_defined", &self.host_defined.is_some())
            .finish()
    }
}

/// What `error_summary` reads of an object with [[ErrorData]] (decision 13), without running
/// code: the kind the constructor that made it recorded, or the host's
/// `HostObjectSpec::error_kind`, never its prototype or `name`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ErrorSummary {
    pub kind: ErrorKind,
    /// The own "message" property's value when it is a data property holding a String.
    pub message: Option<Vec<u16>>,
}

/// What a collection did (decision 18): the bytes `heap_bytes()` read before it and after it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CollectReport {
    pub heap_bytes_before: u64,
    pub heap_bytes_after: u64,
}
