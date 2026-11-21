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

use std::fmt;

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
        SyntaxError::new(
            format!("{construct} is not supported before slice {slice}"),
            line,
            column,
        )
    }

    /// The error of passing the nesting limit `limit`.
    pub(crate) fn nesting(limit: u32, line: u32, column: u32) -> SyntaxError {
        SyntaxError::new(
            format!("nesting deeper than parser_nesting ({limit})"),
            line,
            column,
        )
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
