//! The parser: recursive descent over the tokens of [`crate::lexer`], for the syntactic grammar
//! of `#sec-scripts` that slice 1 takes (clause 13 in `expression.rs`, 14 in `statement.rs`, 15.2
//! in `function.rs`).
//!
//! Two bounds hold its depth (`docs/design/engine.md`, decision 3; `docs/design/host-interface.md`,
//! `parser_nesting`), and passing either is the `SyntaxError` `nesting deeper than parser_nesting
//! (<limit>)`:
//!
//! - every recursive entry of the parser, each function on a path that can come back to itself,
//!   goes through one depth counter, and a script that is not empty enters it once; so the Rust
//!   stack the parser uses is a function of the limit (`ParseOptions::stack_bytes`), and at limit
//!   0 any non-empty source is refused;
//! - every node records its depth, the longest path from it to a leaf, and a node deeper than the
//!   limit is refused; so a chain built by a loop (`a + a + ...`, `a.b.c...`) is bounded too.
//!
//! A frame that stays on the stack while the parser recurses holds little: nodes are reserved and
//! filled in by leaf helpers, and every helper a recursive function calls that is not on a
//! recursive path itself (tokens, nodes, errors) is `#[inline(never)]`, so an optimised build
//! does not move the helper's locals into the recursive frame. `crate::STACK_BYTES_PER_LEVEL`
//! is the bound those frames hold.
//!
//! The parser stops at its first error: a parse that fails has exactly one error.

mod expression;
mod function;
mod statement;

use crate::ast::{
    Ast, Child, Expr, ExprId, ExprNode, Function, FunctionId, NumberLiteral, RegExpLiteral, Span,
    Stmt, StmtId, StmtNode, StringLiteral, TemplateString,
};
use crate::error::SyntaxError;
use crate::ledger::{HeapBytes, Meter, Stop as Halt};
use crate::lexer::Lexer;
use crate::token::{Keyword, Punct, Token, TokenKind};

/// The parse stopped; its error is in [`Parser::error`].
pub(crate) struct Stop;

pub(crate) type R<T> = Result<T, Stop>;

/// What a parse produced: the tree, the statements of the script and whether its code is strict
/// (its directive prologue holds a Use Strict Directive, or, for eval code, the caller is
/// strict), and the refusal the parse held back to its end ([`Parser::held`]), or none.
pub(crate) struct Parsed {
    pub ast: Ast,
    pub body: Vec<StmtId>,
    pub strict: bool,
    pub held: Option<SyntaxError>,
}

pub(crate) struct Parser<'a, 'm> {
    /// The ledger of the parse, shared with the lexer, the walks after the parser and the
    /// compile step: in the parser's state, never in a recursive frame.
    m: &'a Meter<'m>,
    pub(crate) lexer: Lexer<'a, 'm>,
    src: &'a [u16],
    tok: Token,
    peeked: Option<Token>,
    /// The end offset of the token before `tok`.
    prev_end: u32,
    ast: Ast,
    max: u32,
    depth: u32,
    error: Option<SyntaxError>,
    /// Whether the code being parsed is strict mode code, as far as the directive prologues read
    /// so far tell.
    strict: bool,
    /// How many function bodies enclose the current position.
    functions: u32,
    /// The [Yield] parameter of the grammar at the current position: set in the parameters and
    /// the body of a generator, cleared in those of every other function but in the parameters
    /// of an arrow function, which keep the one around them (`#sec-generator-function-definitions`,
    /// `#sec-arrow-function-definitions`).
    yields: bool,
    /// The [Yield] of the code around each function whose parameters or body the parser is in,
    /// innermost last: saved as a function opens and put back as it closes, here and never in a
    /// recursive frame; the growth charged first.
    outer_yields: Vec<bool>,
    /// The [Await] parameter of the grammar at the current position: set in the parameters and
    /// the body of an `async` function or arrow function, cleared in those of every other
    /// function but in the parameters of an arrow function, which keep the one around them
    /// (`#sec-async-function-definitions`, `#sec-async-arrow-function-definitions`). Kept beside
    /// [Yield] in the same way.
    awaits: bool,
    /// The [Await] of the code around each function whose parameters or body the parser is in,
    /// beside `outer_yields`, saved and put back with it; the growth charged first.
    outer_awaits: Vec<bool>,
    /// The refusal of a construct whose parse goes on, held back to the end of the parse, at the
    /// first token of the first such construct of the source: that of an async generator,
    /// `async generator function` (slice 10), at its `async` (`parser/function.rs`). `yield*`
    /// (slice 7) holds none: it parses and runs. The parse answers it only when it ends without
    /// a grammar error and without an early error (`lib.rs`, `analyse_parsed`).
    held: Option<SyntaxError>,
    /// The operator stack of the binary expression parser, shared by its nested calls.
    operators: Vec<expression::Pending>,
    /// The prefix operators of the unary expression parser waiting for their operand, shared by
    /// its nested calls, with where each starts.
    prefixes: Vec<(expression::Prefix, Span)>,
}

/// Parses `source` as a `Script` with nesting limit `max`, charged to `m`.
pub(crate) fn parse(source: &[u16], max: u32, m: &Meter<'_>) -> Result<Parsed, SyntaxError> {
    parse_eval(source, max, false, m)
}

/// Parses `source` as a `Script` with nesting limit `max` for PerformEval (`#sec-performeval`,
/// step 9, `ParseText(x, Script)`): the code is strict from its start when `strict` (a strict
/// caller), and from its directive prologue on when that holds a Use Strict Directive.
/// [`Parsed::strict`] is whether the eval code is strict. Charged to `m`; after a stop of the
/// meter the answer is an error the caller does not read.
pub(crate) fn parse_eval(
    source: &[u16],
    max: u32,
    strict: bool,
    m: &Meter<'_>,
) -> Result<Parsed, SyntaxError> {
    let mut parser = Parser::new(source, max, m);
    parser.strict = strict;
    match parser.script() {
        Ok((body, use_strict)) => Ok(Parsed {
            ast: parser.finish_parse(),
            body,
            strict: strict || use_strict,
            held: parser.held.take(),
        }),
        Err(Stop) => Err(parser.into_error()),
    }
}

/// The kind of a dynamic function (`#sec-createdynamicfunction`, steps 2 to 5): the prefix of
/// its source text and the [Yield] and [Await] of its parameters and body. The kind
/// `async-generator` is slice 10's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DynamicKind {
    /// `normal`, the `Function` constructor: prefix `function`, `FormalParameters[~Yield,
    /// ~Await]`, `FunctionBody[~Yield, ~Await]`, a `FunctionExpression`.
    Normal,
    /// `generator`, `%GeneratorFunction%`: prefix `function*`, `FormalParameters[+Yield,
    /// ~Await]`, `GeneratorBody`, a `GeneratorExpression`.
    Generator,
    /// `async`, `%AsyncFunction%`: prefix `async function`, `FormalParameters[~Yield, +Await]`,
    /// `AsyncFunctionBody`, an `AsyncFunctionExpression`.
    Async,
}

impl DynamicKind {
    /// The source text up to the parameter text: the prefix, then " anonymous(" (step 15).
    fn head(self) -> &'static str {
        match self {
            DynamicKind::Normal => "function anonymous(",
            DynamicKind::Generator => "function* anonymous(",
            DynamicKind::Async => "async function anonymous(",
        }
    }

    /// The [Yield] of the parameters and the body.
    fn yields(self) -> bool {
        self == DynamicKind::Generator
    }

    /// The [Await] of the parameters and the body.
    fn awaits(self) -> bool {
        self == DynamicKind::Async
    }
}

/// The three parses of CreateDynamicFunction (`#sec-createdynamicfunction`, steps 16 to 20) for
/// `kind`, each with nesting limit `max`: `parameters` alone as `FormalParameters` with the
/// [Yield] and [Await] of the kind, the body text (a line feed before and after `body`) alone as
/// the `FunctionBody` of the kind, then the whole source text
/// `<prefix> anonymous(<parameters>\n) {\n<body>\n}` as the function expression of the kind
/// (`function` and a `FunctionExpression`, `function*` and a `GeneratorExpression`, or
/// `async function` and an `AsyncFunctionExpression`). The answer is the tree of the last parse
/// as a script of one expression statement, the function expression, whose own name is taken
/// off: SetFunctionName gives the function the name "anonymous" (step 35), but the name is not
/// bound inside it. The refusal the last parse holds
/// back to its end ([`Parsed::held`]) is answered with it, as a script's is; the first two
/// parses drop theirs, since the last one reads the same text again.
pub(crate) fn parse_dynamic_function(
    kind: DynamicKind,
    parameters: &[u16],
    body: &[u16],
    max: u32,
    m: &Meter<'_>,
) -> Result<Parsed, SyntaxError> {
    const LF: u16 = 0x000A;
    const MIDDLE: &str = "\n) {";
    let head = kind.head();
    // The first two parses keep nothing: what they charged is credited when each ends.
    let mark = m.held();
    let mut alone = Parser::new(parameters, max, m);
    alone.yields = kind.yields();
    alone.awaits = kind.awaits();
    if alone.formal_parameters_text().is_err() {
        return Err(alone.into_error());
    }
    drop(alone);
    m.credit(m.held().saturating_sub(mark));
    let unit_bytes = size_of::<u16>();
    let mut body_text = m
        .with_capacity::<u16>(body.len().saturating_add(2))
        .map_err(|_| stopped())?;
    body_text.push(LF);
    body_text.extend_from_slice(body);
    body_text.push(LF);
    let text_bytes = body_text.capacity().saturating_mul(unit_bytes);
    let mark = m.held();
    let mut alone = Parser::new(&body_text, max, m);
    alone.yields = kind.yields();
    alone.awaits = kind.awaits();
    if alone.function_body_text().is_err() {
        return Err(alone.into_error());
    }
    drop(alone);
    m.credit(m.held().saturating_sub(mark));
    let length = head
        .len()
        .saturating_add(parameters.len())
        .saturating_add(MIDDLE.len())
        .saturating_add(body_text.len())
        .saturating_add(1);
    let mut source = m.with_capacity::<u16>(length).map_err(|_| stopped())?;
    source.extend(head.encode_utf16());
    source.extend_from_slice(parameters);
    source.extend(MIDDLE.encode_utf16());
    source.extend_from_slice(&body_text);
    source.push(u16::from(b'}'));
    let source_bytes = source.capacity().saturating_mul(unit_bytes);
    drop(body_text);
    m.credit(text_bytes);
    let answer = {
        let mut whole = Parser::new(&source, max, m);
        match whole.dynamic_function_text() {
            Ok(statement) => match m.with_capacity(1) {
                Ok(mut body) => {
                    body.push(statement);
                    Ok(Parsed {
                        ast: whole.finish_parse(),
                        body,
                        strict: false,
                        held: whole.held.take(),
                    })
                }
                Err(_) => Err(stopped()),
            },
            Err(Stop) => Err(whole.into_error()),
        }
    };
    drop(source);
    m.credit(source_bytes);
    answer
}

/// The error a parse answers once its meter has stopped; the entry answers the stop instead.
fn stopped() -> SyntaxError {
    SyntaxError::stopped()
}

/// A copy of the name `n` for the tree, charged to `m`.
fn copy_name(m: &Meter<'_>, n: &str) -> Result<String, Halt> {
    m.string(n)
}

/// A copy of the number literal `n` for the tree, charged to `m`.
fn copy_number(m: &Meter<'_>, n: &NumberLiteral) -> Result<NumberLiteral, Halt> {
    Ok(NumberLiteral {
        text: m.string(&n.text)?,
        form: n.form,
    })
}

/// A copy of the string literal `s` for the tree, charged to `m`.
fn copy_string(m: &Meter<'_>, s: &StringLiteral) -> Result<StringLiteral, Halt> {
    Ok(StringLiteral {
        value: m.copy(&s.value)?,
        legacy_octal_escape: s.legacy_octal_escape,
        non_octal_decimal_escape: s.non_octal_decimal_escape,
    })
}

/// A copy of the regular expression literal `r` for the tree, charged to `m`.
fn copy_regexp(m: &Meter<'_>, r: &RegExpLiteral) -> Result<RegExpLiteral, Halt> {
    let body = m.copy(&r.body)?;
    match m.copy(&r.flags) {
        Ok(flags) => Ok(RegExpLiteral { body, flags }),
        Err(stop) => {
            m.release(body);
            Err(stop)
        }
    }
}

/// A copy of the template string `t` for the tree, charged to `m`.
fn copy_template(m: &Meter<'_>, t: &TemplateString) -> Result<TemplateString, Halt> {
    let cooked = m.copy(&t.cooked)?;
    match m.copy(&t.raw) {
        Ok(raw) => Ok(TemplateString { cooked, raw }),
        Err(stop) => {
            m.release(cooked);
            Err(stop)
        }
    }
}

impl<'a, 'm> Parser<'a, 'm> {
    /// A parser at the start of `source`, with nesting limit `max`, outside any function, in
    /// non-strict code, charged to `m`.
    pub(crate) fn new(source: &'a [u16], max: u32, m: &'a Meter<'m>) -> Parser<'a, 'm> {
        Parser {
            m,
            lexer: Lexer::new(source, m),
            src: source,
            tok: Token {
                kind: TokenKind::Eof,
                start: 0,
                end: 0,
                line: 1,
                column: 1,
                newline_before: false,
                escaped: false,
            },
            peeked: None,
            prev_end: 0,
            ast: Ast::default(),
            max,
            depth: 0,
            error: None,
            strict: false,
            functions: 0,
            yields: false,
            outer_yields: Vec::new(),
            awaits: false,
            outer_awaits: Vec::new(),
            held: None,
            operators: Vec::new(),
            prefixes: Vec::new(),
        }
    }

    /// The error a stopped parse found.
    fn into_error(self) -> SyntaxError {
        self.error
            .unwrap_or_else(|| SyntaxError::new("parse failed", 1, 1))
    }

    /// The tree of a parse that ended, its scratch credited: the operator stacks and the tokens
    /// the parser still holds.
    fn finish_parse(&mut self) -> Ast {
        let m = self.m;
        m.release(std::mem::take(&mut self.operators));
        m.release(std::mem::take(&mut self.prefixes));
        m.release(std::mem::take(&mut self.outer_yields));
        m.release(std::mem::take(&mut self.outer_awaits));
        m.credit(self.tok.kind.heap_bytes());
        self.tok.kind = TokenKind::Eof;
        if let Some(peeked) = self.peeked.take() {
            m.credit(peeked.kind.heap_bytes());
        }
        std::mem::take(&mut self.ast)
    }

    /// `Script : ScriptBody?` (`#sec-scripts`).
    pub(crate) fn script(&mut self) -> R<(Vec<StmtId>, bool)> {
        if self.src.is_empty() {
            return Ok((Vec::new(), false));
        }
        self.enter()?;
        self.advance()?;
        let (body, strict) = self.body(true)?;
        if !self.at_eof() {
            return self.unexpected();
        }
        self.leave();
        Ok((body, strict))
    }

    // ---- errors -------------------------------------------------------------------------------

    #[inline(never)]
    fn fail<T>(&mut self, error: SyntaxError) -> R<T> {
        if self.error.is_none() {
            self.error = Some(error);
        }
        Err(Stop)
    }

    #[inline(never)]
    fn fail_here<T>(&mut self, message: impl Into<String>) -> R<T> {
        let e = SyntaxError::new(message, self.tok.line, self.tok.column);
        self.fail(e)
    }

    #[inline(never)]
    fn fail_at<T>(&mut self, message: impl Into<String>, span: Span) -> R<T> {
        self.fail(SyntaxError::new(message, span.line, span.column))
    }

    /// Refuses the construct at the current token, which slice `slice` brings.
    #[inline(never)]
    fn refuse<T>(&mut self, construct: &str, slice: u32) -> R<T> {
        let e = SyntaxError::refused(construct, slice, self.tok.line, self.tok.column);
        self.fail(e)
    }

    #[inline(never)]
    fn refuse_at<T>(&mut self, construct: &str, slice: u32, span: Span) -> R<T> {
        self.fail(SyntaxError::refused(
            construct,
            slice,
            span.line,
            span.column,
        ))
    }

    #[inline(never)]
    fn unexpected<T>(&mut self) -> R<T> {
        let message = match &self.tok.kind {
            TokenKind::Eof => "unexpected end of input".to_string(),
            TokenKind::Name(n) => format!("unexpected token {n}"),
            TokenKind::Keyword(k) => format!("unexpected token {}", k.as_str()),
            TokenKind::Punct(p) => format!("unexpected token {}", p.as_str()),
            TokenKind::Number(n) => format!("unexpected number {}", n.text),
            TokenKind::String(_) => "unexpected string literal".to_string(),
            TokenKind::RegExp(_) => "unexpected regular expression literal".to_string(),
            TokenKind::NoSubstitutionTemplate(_)
            | TokenKind::TemplateHead(_)
            | TokenKind::TemplateMiddle(_)
            | TokenKind::TemplateTail(_) => "unexpected template literal".to_string(),
        };
        self.fail_here(message)
    }

    // ---- the nesting bound ---------------------------------------------------------------------

    /// One recursive entry: refused when it passes the limit.
    #[inline(never)]
    fn enter(&mut self) -> R<()> {
        self.depth += 1;
        if self.depth > self.max {
            let e = SyntaxError::nesting(self.max, self.tok.line, self.tok.column);
            return self.fail(e);
        }
        Ok(())
    }

    #[inline(never)]
    fn leave(&mut self) {
        self.depth = self.depth.saturating_sub(1);
    }

    /// Checks a node depth against the limit.
    #[inline(never)]
    fn check_depth(&mut self, depth: u32, span: Span) -> R<()> {
        if depth > self.max {
            return self.fail(SyntaxError::nesting(self.max, span.line, span.column));
        }
        Ok(())
    }

    // ---- tokens --------------------------------------------------------------------------------

    /// The answer of a charge as the parser's: a stop ends the parse.
    #[inline(never)]
    fn charged<T>(&mut self, answer: Result<T, Halt>) -> R<T> {
        match answer {
            Ok(value) => Ok(value),
            Err(_) => self.fail(stopped()),
        }
    }

    /// Push `item` onto `list`, the growth of the list charged first.
    #[inline(never)]
    fn push_item<T>(&mut self, list: &mut Vec<T>, item: T) -> R<()> {
        let pushed = self.m.push(list, item);
        self.charged(pushed)
    }

    /// Make `next` the current token; what the token it replaces still owns is credited (a
    /// payload the tree keeps was copied with its own charge).
    #[inline(never)]
    fn replace_token(&mut self, next: Token) {
        let old = std::mem::replace(&mut self.tok, next);
        self.m.release(old.kind);
    }

    #[inline(never)]
    fn advance(&mut self) -> R<()> {
        self.prev_end = self.tok.end;
        let next = match self.peeked.take() {
            Some(t) => t,
            None => match self.lexer.next_token() {
                Ok(t) => t,
                Err(e) => return self.fail(e),
            },
        };
        self.replace_token(next);
        Ok(())
    }

    /// The token after the current one, lexed in the `InputElementDiv` goal.
    #[inline(never)]
    fn peek(&mut self) -> R<&Token> {
        if self.peeked.is_none() {
            match self.lexer.next_token() {
                Ok(t) => self.peeked = Some(t),
                Err(e) => return self.fail(e),
            }
        }
        match &self.peeked {
            Some(t) => Ok(t),
            None => Err(Stop),
        }
    }

    /// Lexes the current `/` or `/=` again as a regular expression literal.
    #[inline(never)]
    fn rescan_regexp(&mut self) -> R<()> {
        match self.lexer.rescan_regexp(&self.tok) {
            Ok(t) => {
                self.replace_token(t);
                Ok(())
            }
            Err(e) => self.fail(e),
        }
    }

    /// Lexes the current `}`, the end of a substitution, again as a `TemplateMiddle` or
    /// `TemplateTail`; a token the parser peeked at after it is dropped, as it was lexed in the
    /// wrong goal.
    #[inline(never)]
    fn rescan_template(&mut self) -> R<()> {
        if let Some(peeked) = self.peeked.take() {
            self.m.release(peeked.kind);
        }
        match self.lexer.rescan_template(&self.tok) {
            Ok(t) => {
                self.replace_token(t);
                Ok(())
            }
            Err(e) => self.fail(e),
        }
    }

    fn at_eof(&self) -> bool {
        self.tok.kind == TokenKind::Eof
    }

    fn is(&self, p: Punct) -> bool {
        self.tok.kind == TokenKind::Punct(p)
    }

    fn is_keyword(&self, k: Keyword) -> bool {
        self.tok.kind == TokenKind::Keyword(k)
    }

    /// Whether the current token is the contextual keyword `name` (`let`, `of`, `get`, ...):
    /// the name written without an escape (`#sec-keywords-and-reserved-words`, Note 1).
    fn is_name(&self, name: &str) -> bool {
        !self.tok.escaped && matches!(&self.tok.kind, TokenKind::Name(n) if n == name)
    }

    /// An `Identifier` at the current token, a name written with an escape, may not have the
    /// StringValue of a reserved word (`#sec-identifiers-static-semantics-early-errors`);
    /// `await` and `yield` are left to their own rules.
    #[inline(never)]
    fn check_escaped_keyword(&mut self) -> R<()> {
        if let TokenKind::Name(n) = &self.tok.kind
            && self.tok.escaped
            && Keyword::from_name(n).is_some()
        {
            let message = crate::early::escaped_keyword(n);
            return self.fail_here(message);
        }
        Ok(())
    }

    #[inline(never)]
    fn eat(&mut self, p: Punct) -> R<bool> {
        if self.is(p) {
            self.advance()?;
            return Ok(true);
        }
        Ok(false)
    }

    #[inline(never)]
    fn expect(&mut self, p: Punct) -> R<()> {
        if self.is(p) {
            return self.advance();
        }
        self.unexpected()
    }

    #[inline(never)]
    fn expect_keyword(&mut self, k: Keyword) -> R<()> {
        if self.is_keyword(k) {
            return self.advance();
        }
        self.unexpected()
    }

    /// The `;` that ends a statement, or the one `#sec-rules-of-automatic-semicolon-insertion`
    /// inserts: before a `}`, at the end of the input, or before a token on a new line (the
    /// offending token, as the grammar wants a `;` here). The parser asks for it only where a
    /// statement ends, so an inserted `;` is never an empty statement and never one of the two in
    /// the head of a `for`, which read their `;` themselves.
    #[inline(never)]
    fn semicolon(&mut self) -> R<()> {
        if self.eat(Punct::Semicolon)? {
            return Ok(());
        }
        if self.is(Punct::RBrace) || self.at_eof() || self.tok.newline_before {
            return Ok(());
        }
        self.unexpected()
    }

    /// Whether `#sec-rules-of-automatic-semicolon-insertion` inserts a `;` before the current
    /// token after a restricted production's keyword: a `}`, the end of the input or a line
    /// terminator before it (`return`, `break`, `continue`).
    fn ends_statement_here(&self) -> bool {
        self.is(Punct::Semicolon)
            || self.is(Punct::RBrace)
            || self.at_eof()
            || self.tok.newline_before
    }

    // ---- positions and nodes -------------------------------------------------------------------

    /// A span starting at the current token; [`Parser::finish`] sets its end.
    fn start(&self) -> Span {
        Span {
            start: self.tok.start,
            end: self.tok.end,
            line: self.tok.line,
            column: self.tok.column,
        }
    }

    fn finish(&self, mut span: Span) -> Span {
        span.end = self.prev_end.max(span.start);
        span
    }

    #[inline(never)]
    fn expr_span(&self, id: ExprId) -> Span {
        self.ast.expr(id).map(|n| n.span).unwrap_or_default()
    }

    #[inline(never)]
    fn push_expr(&mut self, expr: Expr, span: Span) -> R<ExprId> {
        let mut deepest = 0;
        expr.children(|c: Child| deepest = deepest.max(self.ast.depth(c)));
        let depth = deepest.saturating_add(1);
        self.check_depth(depth, span)?;
        let room = self.m.grow(&mut self.ast.exprs);
        self.charged(room)?;
        let id = ExprId(u32::try_from(self.ast.exprs.len()).unwrap_or(u32::MAX));
        self.ast.exprs.push(ExprNode {
            expr,
            span,
            depth,
            parenthesized: false,
        });
        Ok(id)
    }

    #[inline(never)]
    fn push_stmt(&mut self, stmt: Stmt, span: Span) -> R<StmtId> {
        let mut deepest = 0;
        stmt.children(|c: Child| deepest = deepest.max(self.ast.depth(c)));
        let depth = deepest.saturating_add(1);
        self.check_depth(depth, span)?;
        let room = self.m.grow(&mut self.ast.stmts);
        self.charged(room)?;
        let id = StmtId(u32::try_from(self.ast.stmts.len()).unwrap_or(u32::MAX));
        self.ast.stmts.push(StmtNode { stmt, span, depth });
        Ok(id)
    }

    /// Pushes `stmt` as a node whose parts are filled in while its children are parsed; its
    /// depth and span are set by [`Parser::complete_stmt`]. A frame that stays on the stack while
    /// the children are parsed then holds only the id.
    ///
    /// When the growth of the arena is refused, nothing is pushed and the id names no node: the
    /// meter has stopped, every later charge is refused, and the parse ends at its next token
    /// (the lexer checks the meter before each), so the recursive callers keep their frames.
    #[inline(never)]
    fn reserve_stmt(&mut self, stmt: Stmt) -> StmtId {
        let id = StmtId(u32::try_from(self.ast.stmts.len()).unwrap_or(u32::MAX));
        if self.m.grow(&mut self.ast.stmts).is_ok() {
            self.ast.stmts.push(StmtNode {
                stmt,
                span: Span::default(),
                depth: 0,
            });
        }
        id
    }

    fn stmt_mut(&mut self, id: StmtId) -> Option<&mut Stmt> {
        self.ast.stmts.get_mut(id.0 as usize).map(|n| &mut n.stmt)
    }

    /// Sets the span (from `start` to the previous token) and the depth of a reserved statement.
    #[inline(never)]
    fn complete_stmt(&mut self, id: StmtId, start: Span) -> R<StmtId> {
        let span = self.finish(start);
        let mut deepest = 0;
        if let Some(n) = self.ast.stmt(id) {
            n.stmt
                .children(|c: Child| deepest = deepest.max(self.ast.depth(c)));
        }
        let depth = deepest.saturating_add(1);
        self.check_depth(depth, span)?;
        if let Some(n) = self.ast.stmts.get_mut(id.0 as usize) {
            n.span = span;
            n.depth = depth;
        }
        Ok(id)
    }

    /// As [`Parser::reserve_stmt`], for an expression.
    #[inline(never)]
    fn reserve_expr(&mut self, expr: Expr) -> ExprId {
        let id = ExprId(u32::try_from(self.ast.exprs.len()).unwrap_or(u32::MAX));
        if self.m.grow(&mut self.ast.exprs).is_ok() {
            self.ast.exprs.push(ExprNode {
                expr,
                span: Span::default(),
                depth: 0,
                parenthesized: false,
            });
        }
        id
    }

    fn expr_mut(&mut self, id: ExprId) -> Option<&mut Expr> {
        self.ast.exprs.get_mut(id.0 as usize).map(|n| &mut n.expr)
    }

    /// As [`Parser::complete_stmt`], for an expression.
    #[inline(never)]
    fn complete_expr(&mut self, id: ExprId, start: Span) -> R<ExprId> {
        let span = self.finish(start);
        let mut deepest = 0;
        if let Some(n) = self.ast.expr(id) {
            n.expr
                .children(|c: Child| deepest = deepest.max(self.ast.depth(c)));
        }
        let depth = deepest.saturating_add(1);
        self.check_depth(depth, span)?;
        if let Some(n) = self.ast.exprs.get_mut(id.0 as usize) {
            n.span = span;
            n.depth = depth;
        }
        Ok(id)
    }

    /// Reserves the next function number, so functions are numbered in source order.
    #[inline(never)]
    fn reserve_function(&mut self, function: Function) -> FunctionId {
        let id = FunctionId(u32::try_from(self.ast.functions.len()).unwrap_or(u32::MAX));
        // As `reserve_stmt`: a refused growth pushes nothing and the parse ends at its next
        // token.
        if self.m.grow(&mut self.ast.functions).is_ok() {
            self.ast.functions.push(function);
        } else {
            self.m.release(function);
        }
        id
    }

    fn expr_node(&self, id: ExprId) -> Option<&ExprNode> {
        self.ast.expr(id)
    }

    /// An expression statement holding the function expression `function`, whose depth is
    /// `depth`, with no depth check: the statement and the expression are the wrapping of a
    /// dynamic function's tree, not source nesting.
    #[inline(never)]
    fn wrap_in_statement(&mut self, function: FunctionId, span: Span, depth: u32) -> R<StmtId> {
        let room = self.m.grow(&mut self.ast.exprs);
        self.charged(room)?;
        let expr = ExprId(u32::try_from(self.ast.exprs.len()).unwrap_or(u32::MAX));
        self.ast.exprs.push(ExprNode {
            expr: Expr::Function(function),
            span,
            depth: depth.saturating_add(1),
            parenthesized: false,
        });
        let room = self.m.grow(&mut self.ast.stmts);
        self.charged(room)?;
        let stmt = StmtId(u32::try_from(self.ast.stmts.len()).unwrap_or(u32::MAX));
        self.ast.stmts.push(StmtNode {
            stmt: Stmt::Expression(expr),
            span,
            depth: depth.saturating_add(2),
        });
        Ok(stmt)
    }
}
