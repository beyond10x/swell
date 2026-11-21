#![forbid(unsafe_code)]
//! `swell-syntax`: the lexer, parser, early errors and scope analysis of swell (design,
//! decision 2).
//!
//! Slice 1 takes the ES5 grammar with strict mode (`docs/design/engine.md`, section 12): `var`
//! and function declarations, every ES5 statement but `with` and labelled statements, every ES5
//! expression form, the directive prologue and the strict early errors. Every construct of a
//! later slice is refused with a `SyntaxError` that names the slice: `<construct> is not
//! supported before slice <n>` ("A part before its slice"). Two of those refusals fall inside
//! sections whose cases are counted in slice 2 and are recorded as omissions in
//! `conformance/test262/sections.txt`: an identifier with a code point outside ASCII or a `\u`
//! escape (`sec-names-and-keywords`), and a regular expression literal, which is lexed to its
//! end and kept as a node, its pattern unparsed, so that evaluating it throws a `SyntaxError`
//! (`sec-literals-regular-expression-literals`).
//!
//! The parser's depth is bounded by [`ParseOptions::max_nesting`], the host's `parser_nesting`
//! (`docs/design/host-interface.md`); every walk over the tree is bounded by the same depth.
//!
//! Modules: [`error`], `token` and `lexer` (clause 12), [`ast`], `parser` (clauses 13 to 15),
//! `early` (the early errors) and `scope` (directives, strictness, declared and referenced names).

pub mod ast;
mod early;
pub mod error;
mod lexer;
mod parser;
mod scope;
mod token;

pub use ast::Scope;
pub use error::SyntaxError;

use ast::{Ast, StmtId};

/// The options of a parse.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParseOptions {
    /// The host's `parser_nesting`: how deep the parser may recurse and how deep a node of the
    /// tree may be. Passing it is the `SyntaxError` `nesting deeper than parser_nesting
    /// (<limit>)`; at 0 every non-empty source fails.
    pub max_nesting: u32,
}

/// A parsed `Script` (`#sec-scripts`) with its analysis.
#[derive(Clone, Debug, PartialEq)]
pub struct Script {
    /// The statements of the `ScriptBody`.
    pub body: Vec<StmtId>,
    /// Whether the script is strict mode code: its directive prologue holds a Use Strict
    /// Directive (`#sec-strict-mode-code`).
    pub strict: bool,
    /// The script's var declared names, function declarations to instantiate and references.
    pub scope: Scope,
    pub ast: Ast,
}

/// Parses `source`, UTF-16 code units (a UTF-8 source is converted by the caller), as a
/// `Script`: the tree, or every `SyntaxError` the parse found, at least one. A grammar error or a
/// refusal stops the parse and is the only error; the early errors of a source that parses are
/// all reported, in source order.
pub fn parse_script(source: &[u16], options: ParseOptions) -> Result<Script, Vec<SyntaxError>> {
    let max = options.max_nesting;
    let parsed = parser::parse(source, max).map_err(|e| vec![e])?;
    let mut ast = parsed.ast;
    let errors = early::check(&ast, &parsed.body, parsed.strict, max);
    if !errors.is_empty() {
        return Err(errors);
    }
    let scope = scope::analyse(&mut ast, &parsed.body, max).map_err(|e| vec![e])?;
    Ok(Script {
        body: parsed.body,
        strict: parsed.strict,
        scope,
        ast,
    })
}
