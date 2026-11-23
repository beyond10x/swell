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
//!   or the refusal of the flag `i` ([`CompileError`]), or the stop of the budget.
//! - [`match_at`] and [`search`]: the matcher of 22.2.2 from one start index, and the search of
//!   RegExpBuiltinExec over the start indices, against a [`Subject`] read in place; the
//!   [`Captures`] in code unit indices, or no match, or the stop of the budget.
//!
//! Refused until the later stories of slice 5, each with the message `<construct> is not
//! supported before slice 5`: the flag `v`, a property escape `\p{...}` or `\P{...}`, and a
//! group name with a code point outside ASCII ([`Refusal`]), which ParsePattern refuses; and
//! the flag `i` or a modifier `i` ([`CompileRefusal`]), which CompilePattern refuses.

mod ast;
mod budget;
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
pub use compile::{CompileError, CompileRefusal, compile_pattern};
pub use error::{ErrorKind, ParseError, PatternError, Refusal};
pub use flags::Flags;
pub use matcher::{Captures, Found, Subject, match_at, search};
pub use parser::parse_pattern;
pub use program::Program;
