#![forbid(unsafe_code)]
//! `swell-regexp`, the regular expressions of swell (`docs/design/engine.md`, decision 7;
//! `#sec-regexp-regular-expression-objects`): the pattern parser with its early errors, the
//! compiler of a pattern to a program, and the backtracking matcher over UTF-16.
//!
//! The crate depends on `swell-unicode` and names no type of `swell-syntax` or `swell` (decision
//! 2): its caller hands it a [`Budget`] (decision 7, "How `swell-regexp` charges its work").
//!
//! - [`Flags::parse`]: the flags of a pattern from their code units (`#sec-regexpinitialize`).
//! - [`parse_pattern`]: ParsePattern (`#sec-parsepattern`) of a pattern's code units with its
//!   flags and a nesting limit: the [`Pattern`] tree, one [`PatternError`] with its offset, or
//!   the [`Stop`] of the budget with everything credited.
//!
//! - [`compile_pattern`]: CompilePattern (`#sec-compilepattern`) of the tree to a [`Program`],
//!   or the stop of the budget ([`CompileError`]).
//! - [`canonicalize`]: Canonicalize (`#sec-runtime-semantics-canonicalize-ch`) of a character
//!   under the flag `i` or a modifier `i`: the simple case folding of `CaseFolding.txt` under
//!   `u` or `v`, the uppercase mapping of the code unit under neither.
//! - [`match_at`] and [`search`]: the matcher of 22.2.2 from one start index, and the search of
//!   RegExpBuiltinExec over the start indices, against a [`Subject`] read in place; the
//!   [`Captures`] in code unit indices, or no match, or the stop of the budget.
//!
//! A property escape `\p{...}` or `\P{...}` under `u` names a property of `swell-unicode`
//! ([`ClassEscape::Property`]); a name or a value ECMA-262 does not admit is an early error.
//!
//! Refused until the later stories of slice 5, each with the message `<construct> is not
//! supported before slice 5`: the flag `v` ([`Refusal`]), which ParsePattern refuses.
//! CompilePattern refuses nothing ([`CompileRefusal`] has no value).

mod ast;
mod budget;
mod canonicalize;
mod compile;
mod early;
mod error;
mod flags;
mod matcher;
mod parser;
mod program;

pub use ast::{
    Assertion, ClassEscape, ClassItem, List, Look, Modifiers, Name, Node, NodeId, Pattern,
    Quantifier,
};
pub use budget::{Budget, MAX_GRANT, POLL_INTERVAL, Stop, Unlimited};
pub use canonicalize::canonicalize;
pub use compile::{CompileError, CompileRefusal, compile_pattern};
pub use error::{ErrorKind, ParseError, PatternError, Refusal};
pub use flags::Flags;
pub use matcher::{Captures, Found, Subject, match_at, search, search_slot_visits};
pub use parser::{parse_name_visits, parse_pattern};
pub use program::Program;
