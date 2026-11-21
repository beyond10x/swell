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
    Ast, Child, Expr, ExprId, ExprNode, Function, FunctionId, Span, Stmt, StmtId, StmtNode,
};
use crate::error::SyntaxError;
use crate::lexer::Lexer;
use crate::token::{Keyword, Punct, Token, TokenKind};

/// The parse stopped; its error is in [`Parser::error`].
pub(crate) struct Stop;

pub(crate) type R<T> = Result<T, Stop>;

/// What a parse produced: the tree, the statements of the script and whether its directive
/// prologue holds a Use Strict Directive.
pub(crate) struct Parsed {
    pub ast: Ast,
    pub body: Vec<StmtId>,
    pub strict: bool,
}

pub(crate) struct Parser<'a> {
    lexer: Lexer<'a>,
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
    /// The operator stack of the binary expression parser, shared by its nested calls.
    operators: Vec<expression::Pending>,
    /// The prefix operators of the unary expression parser waiting for their operand, shared by
    /// its nested calls, with where each starts.
    prefixes: Vec<(expression::Prefix, Span)>,
}

/// Parses `source` as a `Script` with nesting limit `max`.
pub(crate) fn parse(source: &[u16], max: u32) -> Result<Parsed, SyntaxError> {
    let mut parser = Parser {
        lexer: Lexer::new(source),
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
        operators: Vec::new(),
        prefixes: Vec::new(),
    };
    match parser.script() {
        Ok((body, strict)) => Ok(Parsed {
            ast: parser.ast,
            body,
            strict,
        }),
        Err(Stop) => Err(parser
            .error
            .unwrap_or_else(|| SyntaxError::new("parse failed", 1, 1))),
    }
}

impl<'a> Parser<'a> {
    /// `Script : ScriptBody?` (`#sec-scripts`).
    fn script(&mut self) -> R<(Vec<StmtId>, bool)> {
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
        self.tok = next;
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
                self.tok = t;
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
        let id = StmtId(u32::try_from(self.ast.stmts.len()).unwrap_or(u32::MAX));
        self.ast.stmts.push(StmtNode { stmt, span, depth });
        Ok(id)
    }

    /// Pushes `stmt` as a node whose parts are filled in while its children are parsed; its
    /// depth and span are set by [`Parser::complete_stmt`]. A frame that stays on the stack while
    /// the children are parsed then holds only the id.
    #[inline(never)]
    fn reserve_stmt(&mut self, stmt: Stmt) -> StmtId {
        let id = StmtId(u32::try_from(self.ast.stmts.len()).unwrap_or(u32::MAX));
        self.ast.stmts.push(StmtNode {
            stmt,
            span: Span::default(),
            depth: 0,
        });
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
        self.ast.exprs.push(ExprNode {
            expr,
            span: Span::default(),
            depth: 0,
            parenthesized: false,
        });
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
        self.ast.functions.push(function);
        id
    }

    fn expr_node(&self, id: ExprId) -> Option<&ExprNode> {
        self.ast.expr(id)
    }
}
