#![forbid(unsafe_code)]
//! `swell-regexp`, the regular expressions of swell (`docs/design/engine.md`, decision 7;
//! `#sec-regexp-regular-expression-objects`): the pattern parser with its early errors, and,
//! from the later stories of slice 5, the compiler and the backtracking matcher over UTF-16.
//!
//! The crate depends on `swell-unicode` and names no type of `swell-syntax` or `swell` (decision
//! 2): its caller hands it a [`Budget`] (decision 7, "How `swell-regexp` charges its work").
//!
//! - [`Flags::parse`]: the flags of a pattern from their code units (`#sec-regexpinitialize`).
//! - [`parse_pattern`]: ParsePattern (`#sec-parsepattern`) of a pattern's code units with its
//!   flags and a nesting limit: the [`Pattern`] tree, one [`PatternError`] with its offset, or
//!   the [`Stop`] of the budget with everything credited.
//!
//! Refused until the later stories of slice 5, each with the message `<construct> is not
//! supported before slice 5`: the flag `v`, a property escape `\p{...}` or `\P{...}`, and a
//! group name with a code point outside ASCII ([`Refusal`]).

mod ast;
mod budget;
mod early;
mod error;
mod flags;
mod parser;

pub use ast::{
    Assertion, ClassEscape, ClassItem, List, Look, Modifiers, Name, Node, NodeId, Pattern,
    Quantifier,
};
pub use budget::{Budget, MAX_GRANT, POLL_INTERVAL, Stop, Unlimited};
pub use error::{ErrorKind, ParseError, PatternError, Refusal};
pub use flags::Flags;
pub use parser::parse_pattern;
