//! The errors of a parse: a `SyntaxError` with its message and the position of the code unit it
//! names (`#sec-parse-script`: ParseScript "returns a Script Record or a non-empty List of
//! SyntaxError objects").
//!
//! Three kinds of message:
//!
//! - a grammar or early error of ECMA-262 (`#sec-static-semantics-early-errors`), worded here;
//! - a refusal of a construct a later slice brings (`docs/design/engine.md`, section 12, "A part
//!   before its slice"): `<construct> is not supported before slice <n>`;
//! - the parser's implementation limit (`docs/design/host-interface.md`, `parser_nesting`):
//!   `nesting deeper than parser_nesting (<limit>)`.
//!
//! [`message_kind`] tells them apart from the message alone, so that a host that holds only the
//! `message` of a `SyntaxError` object (the conformance runner) can tell an error ECMA-262 asks
//! for from a construct this parser declines or a limit it reaches.

use std::fmt;

/// What stands between the construct and the slice in a refusal.
const REFUSED: &str = " is not supported before slice ";

/// The message of the `SyntaxError` a regular expression literal throws when it is evaluated
/// before slice 5 (`docs/design/engine.md`, "A part before its slice";
/// `sec-literals-regular-expression-literals` records the omission): a refusal of slice 5 in the
/// form [`message_kind`] reads.
pub const REGEXP_REFUSAL: &str = "regular expression literal is not supported before slice 5";

/// What stands before the limit, which a `)` follows, in the error of the nesting limit.
const NESTING: &str = "nesting deeper than parser_nesting (";

/// The kind of a `SyntaxError` message of this crate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MessageKind {
    /// A grammar or early error of ECMA-262: any message that is not one of the two below.
    Grammar,
    /// The refusal of a construct slice `slice` brings: `<construct> is not supported before
    /// slice <slice>`.
    Refusal { slice: u32 },
    /// The parser's implementation limit: `nesting deeper than parser_nesting (<limit>)`.
    Nesting { limit: u32 },
}

/// The kind of `message`, the message of a [`SyntaxError`] of this crate as it is, without the
/// position a host may add to it. A refusal needs a non-empty construct and a slice of decimal
/// digits only; the nesting error a limit of decimal digits only.
pub fn message_kind(message: &str) -> MessageKind {
    if let Some(limit) = message
        .strip_prefix(NESTING)
        .and_then(|rest| rest.strip_suffix(')'))
        .and_then(decimal)
    {
        return MessageKind::Nesting { limit };
    }
    if let Some((construct, slice)) = message.rsplit_once(REFUSED)
        && !construct.is_empty()
        && let Some(slice) = decimal(slice)
    {
        return MessageKind::Refusal { slice };
    }
    MessageKind::Grammar
}

/// `text` as a `u32` when it is one or more ASCII digits and nothing else.
fn decimal(text: &str) -> Option<u32> {
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

/// One `SyntaxError` of a parse. `line` and `column` count from 1; a column counts UTF-16 code
/// units from the start of the line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SyntaxError {
    pub message: String,
    pub line: u32,
    pub column: u32,
}

impl SyntaxError {
    pub(crate) fn new(message: impl Into<String>, line: u32, column: u32) -> SyntaxError {
        SyntaxError {
            message: message.into(),
            line,
            column,
        }
    }

    /// The refusal of `construct`, which slice `slice` brings.
    pub(crate) fn refused(construct: &str, slice: u32, line: u32, column: u32) -> SyntaxError {
        SyntaxError::new(format!("{construct}{REFUSED}{slice}"), line, column)
    }

    /// The error of passing the nesting limit `limit`.
    pub(crate) fn nesting(limit: u32, line: u32, column: u32) -> SyntaxError {
        SyntaxError::new(format!("{NESTING}{limit})"), line, column)
    }
}

impl fmt::Display for SyntaxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}: SyntaxError: {}",
            self.line, self.column, self.message
        )
    }
}

impl std::error::Error for SyntaxError {}
