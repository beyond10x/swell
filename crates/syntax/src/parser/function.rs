//! Function definitions (15.2, `#sec-function-definitions`), arrow functions (15.3,
//! `#sec-arrow-function-definitions`) and the accessors and plain methods of object literals
//! (`#sec-method-definitions`).
//!
//! The parameters are a list of `BindingIdentifier`s, with a trailing comma allowed
//! (`#sec-parameter-lists`); a default value, a rest parameter and a binding pattern are refused
//! (slice 6), as are generators (7) and `async` arrows (10). A
//! function's code is strict mode code when the code around it is or when its body starts with
//! a Use Strict Directive (`#sec-strict-mode-code`); the parser tracks that while it reads the
//! body, and the early errors that depend on it are checked afterwards (`src/early.rs`).

use super::{Parser, R};
use crate::ast::{
    AssignOp, Child, Expr, ExprId, Function, FunctionId, FunctionKind, Name, Scope, Span, Stmt,
    StmtId,
};
use crate::token::{Keyword, Punct};

impl<'a, 'm> Parser<'a, 'm> {
    /// `FunctionDeclaration` at the current `function`.
    pub(super) fn function_declaration(&mut self) -> R<FunctionId> {
        self.enter()?;
        let id = self.function_head(FunctionKind::Declaration)?;
        self.function_rest(id)?;
        self.leave();
        Ok(id)
    }

    /// `FunctionExpression` at the current `function`.
    pub(super) fn function_expression(&mut self) -> R<FunctionId> {
        self.enter()?;
        let id = self.function_head(FunctionKind::Expression)?;
        self.function_rest(id)?;
        self.leave();
        Ok(id)
    }

    /// `function` and the name: numbers the function, so functions are numbered in source order.
    fn function_head(&mut self, kind: FunctionKind) -> R<FunctionId> {
        let span = self.start();
        self.expect_keyword(Keyword::Function)?;
        if self.is(Punct::Star) {
            return self.refuse("generator function", 7);
        }
        let name = if kind == FunctionKind::Expression && self.is(Punct::LParen) {
            None
        } else {
            Some(self.binding_identifier()?)
        };
        Ok(self.begin_function(kind, span, name))
    }

    /// Numbers a function whose first token is at `span`.
    pub(super) fn begin_function(
        &mut self,
        kind: FunctionKind,
        span: Span,
        name: Option<Name>,
    ) -> FunctionId {
        self.reserve_function(Function {
            kind,
            name,
            params: Vec::new(),
            body: Vec::new(),
            strict: false,
            scope: Scope::default(),
            span,
            depth: 1,
        })
    }

    /// The parameters and body of the function `id`, from its `(`.
    pub(super) fn function_rest(&mut self, id: FunctionId) -> R<()> {
        self.enter()?;
        let outer = self.function_open(id)?;
        let (body, use_strict) = self.body(false)?;
        self.function_close(id, outer, body, use_strict)?;
        self.leave();
        Ok(())
    }

    /// Reads the parameters and the `{` of function `id`; returns the strictness of the code
    /// around it.
    fn function_open(&mut self, id: FunctionId) -> R<bool> {
        let kind = self
            .ast
            .function(id)
            .map_or(FunctionKind::Expression, |f| f.kind);
        let params = self.parameters(kind)?;
        if let Some(f) = self.ast.functions.get_mut(id.0 as usize) {
            f.params = params;
        }
        self.expect(Punct::LBrace)?;
        self.functions += 1;
        Ok(self.strict)
    }

    /// Reads the `}` of function `id` and records its body, strictness, span and depth.
    fn function_close(
        &mut self,
        id: FunctionId,
        outer: bool,
        body: Vec<StmtId>,
        use_strict: bool,
    ) -> R<()> {
        self.expect(Punct::RBrace)?;
        self.functions -= 1;
        self.function_end(id, outer, body, use_strict)
    }

    /// Records the body of function `id`, which ends at the previous token, with its
    /// strictness, span and depth; the code after it has the strictness `outer` again.
    #[inline(never)]
    fn function_end(
        &mut self,
        id: FunctionId,
        outer: bool,
        body: Vec<StmtId>,
        use_strict: bool,
    ) -> R<()> {
        self.strict = outer;
        let mut deepest = 0;
        for s in &body {
            deepest = deepest.max(self.ast.depth(Child::Stmt(*s)));
        }
        let depth = deepest.saturating_add(1);
        let start = self.ast.function(id).map(|f| f.span).unwrap_or_default();
        let span = self.finish(start);
        self.check_depth(depth, span)?;
        if let Some(f) = self.ast.functions.get_mut(id.0 as usize) {
            f.body = body;
            f.strict = outer || use_strict;
            f.span = span;
            f.depth = depth;
        }
        Ok(())
    }

    /// `( FormalParameters )`; for a getter `()`, for a setter one `BindingIdentifier`. An arrow
    /// function's `ArrowFormalParameters` have the same grammar, but its parser reads them as
    /// the parenthesized expression they cover ([`Parser::arrow_function`]).
    fn parameters(&mut self, kind: FunctionKind) -> R<Vec<Name>> {
        self.expect(Punct::LParen)?;
        let mut params = Vec::new();
        match kind {
            FunctionKind::Getter => {}
            FunctionKind::Setter => self.push_parameter(&mut params)?,
            FunctionKind::Declaration
            | FunctionKind::Expression
            | FunctionKind::Method
            | FunctionKind::Arrow => {
                while !self.is(Punct::RParen) {
                    self.push_parameter(&mut params)?;
                    if !self.eat(Punct::Comma)? {
                        break;
                    }
                }
            }
        }
        self.expect(Punct::RParen)?;
        Ok(params)
    }

    /// The parameter text of CreateDynamicFunction (`#sec-createdynamicfunction`, step 16) alone,
    /// the whole source as `FormalParameters[~Yield, ~Await]`: the parameters, a trailing comma
    /// allowed, to the end of the input. An empty text is the empty list; any other text enters
    /// the nesting bound once, as a script does.
    pub(super) fn formal_parameters_text(&mut self) -> R<Vec<Name>> {
        if self.src.is_empty() {
            return Ok(Vec::new());
        }
        self.enter()?;
        self.advance()?;
        let mut params = Vec::new();
        while !self.at_eof() {
            self.push_parameter(&mut params)?;
            if !self.eat(Punct::Comma)? {
                break;
            }
        }
        if !self.at_eof() {
            return self.unexpected();
        }
        self.leave();
        Ok(params)
    }

    /// The body text of CreateDynamicFunction (step 17) alone, the whole source as
    /// `FunctionBody[~Yield, ~Await]`: the statements of a function body to the end of the input,
    /// where `return` may stand and a `}` may not.
    pub(super) fn function_body_text(&mut self) -> R<()> {
        self.enter()?;
        self.advance()?;
        self.functions += 1;
        self.body(true)?;
        self.functions -= 1;
        if !self.at_eof() {
            return self.unexpected();
        }
        self.leave();
        Ok(())
    }

    /// The source text of CreateDynamicFunction (step 19) as a `FunctionExpression` and nothing
    /// after it: the function, its own name taken off (the name "anonymous" is given, not
    /// bound), then an expression statement holding it, made without a depth check, so that the
    /// function is admitted exactly when the same expression standing alone would be.
    pub(super) fn dynamic_function_text(&mut self) -> R<StmtId> {
        self.enter()?;
        self.advance()?;
        let start = self.start();
        let id = self.function_expression()?;
        if !self.at_eof() {
            return self.unexpected();
        }
        self.leave();
        let span = self.finish(start);
        let m = self.m;
        let depth = match self.ast.functions.get_mut(id.0 as usize) {
            Some(f) => {
                if let Some(name) = f.name.take() {
                    m.release(name);
                }
                f.depth
            }
            None => return self.fail_here("parse failed"),
        };
        self.wrap_in_statement(id, span, depth)
    }

    /// One parameter, appended to `params`, the growth of the list charged first.
    #[inline(never)]
    fn push_parameter(&mut self, params: &mut Vec<Name>) -> R<()> {
        let parameter = self.parameter()?;
        let m = self.m;
        let pushed = m.push(params, parameter);
        self.charged(pushed)
    }

    /// One `FormalParameter`, a `BindingIdentifier` without an initializer.
    fn parameter(&mut self) -> R<Name> {
        if self.is(Punct::Ellipsis) {
            return self.refuse("rest parameter", 6);
        }
        let name = self.binding_identifier()?;
        if self.is(Punct::Eq) {
            return self.refuse("default parameter", 6);
        }
        Ok(name)
    }

    // ---- arrow functions -----------------------------------------------------------------------

    /// `ArrowFunction : ArrowParameters [no LineTerminator here] => ConciseBody`
    /// (`#sec-arrow-function-definitions`) at the current `=>`, after the operand `left` of an
    /// `AssignmentExpression` that started at `start`: the operand read again as the
    /// parameters, then the body. The function is numbered here: no function stands in valid
    /// parameters, so it still comes after every function before its first token.
    pub(super) fn arrow_function(&mut self, left: ExprId, start: Span, no_in: bool) -> R<ExprId> {
        self.enter()?;
        let id = self.arrow_head(left, start)?;
        self.arrow_body(id, no_in)?;
        let e = self.make(Expr::Function(id), start)?;
        self.leave();
        Ok(e)
    }

    /// The parameters of the arrow function whose `=>` is the current token, from the operand
    /// `left`; consumes the `=>` and numbers the function, its span starting at `start`.
    #[inline(never)]
    fn arrow_head(&mut self, left: ExprId, start: Span) -> R<FunctionId> {
        if self.tok.newline_before {
            return self.fail_here(LINE_TERMINATOR_BEFORE_ARROW);
        }
        let params = self.arrow_parameters(left, start)?;
        self.advance()?;
        let id = self.begin_function(FunctionKind::Arrow, start, None);
        match self.ast.functions.get_mut(id.0 as usize) {
            Some(f) => f.params = params,
            None => self.m.release(params),
        }
        Ok(id)
    }

    /// `ArrowParameters` from the operand `left` before `=>`: a `BindingIdentifier` (an
    /// identifier not in parentheses), or the parenthesized expression, in one pair of
    /// parentheses ([`Parser::parenthesized`]), read again as `ArrowFormalParameters`
    /// (`#sec-arrow-function-definitions-static-semantics-early-errors`, the cover's
    /// refinement): an identifier, a comma expression of identifiers not in parentheses, or the
    /// empty list of `()`. An initializer is a default parameter and an object or array literal
    /// a pattern, both refused; `async` before parentheses is an async arrow, refused. The
    /// names move from the expression's nodes into the list, the nodes of the first reading are
    /// dropped and what they own credited; the list's growth is charged first.
    #[inline(never)]
    fn arrow_parameters(&mut self, left: ExprId, start: Span) -> R<Vec<Name>> {
        let (parenthesized, elements) = match self.expr_node(left) {
            Some(n) => match &n.expr {
                Expr::Identifier(_) => (n.parenthesized, Some(1)),
                Expr::Sequence(list) if n.parenthesized => (true, Some(list.len())),
                _ => (n.parenthesized, None),
            },
            None => return self.unexpected(),
        };
        let Some(elements) = elements else {
            return self.arrow_refusal(left, parenthesized, start);
        };
        let mut mark = left;
        for i in 0..elements {
            let e = self.arrow_element(left, i);
            mark = mark.min(e);
            // `left` itself is in no parentheses or in the one pair of the cover; an operand of
            // the cover's comma expression is in none.
            let (identifier, bare) = self.expr_node(e).map_or((false, false), |n| {
                (matches!(n.expr, Expr::Identifier(_)), !n.parenthesized)
            });
            if !identifier || !(bare || e == left) {
                return self.arrow_refusal(e, bare, start);
            }
        }
        let made = self.m.with_capacity(elements);
        let mut params = self.charged(made)?;
        for i in 0..elements {
            let e = self.arrow_element(left, i);
            let span = self.expr_span(e);
            if let Some(Expr::Identifier(name)) = self.expr_mut(e) {
                let name = std::mem::take(name);
                params.push(Name { name, span });
            }
        }
        self.drop_cover(mark);
        Ok(params)
    }

    /// The `i`th element of the operand `left` read as parameters: `left` itself, or the `i`th
    /// operand of the comma expression it is.
    fn arrow_element(&self, left: ExprId, i: usize) -> ExprId {
        match self.expr_node(left).map(|n| &n.expr) {
            Some(Expr::Sequence(list)) => list.get(i).copied().unwrap_or(left),
            _ => left,
        }
    }

    /// The error of the element `e` of an operand before `=>` that is no `ArrowParameters`:
    /// the refusal a later story lifts (`default parameter` and `destructuring pattern` for an
    /// element that stands as a parameter, `plain`: the cover's own expression or an operand of
    /// its comma expression in no parentheses of its own; `async function` for a call of
    /// `async`), or the `=>` unexpected.
    #[inline(never)]
    fn arrow_refusal<T>(&mut self, e: ExprId, plain: bool, start: Span) -> R<T> {
        let Some(node) = self.expr_node(e) else {
            return self.unexpected();
        };
        let at = node.span;
        match &node.expr {
            Expr::Assign {
                op: AssignOp::Assign,
                target,
                ..
            } if plain
                && self
                    .expr_node(*target)
                    .is_some_and(|t| !t.parenthesized && matches!(t.expr, Expr::Identifier(_))) =>
            {
                self.refuse_at("default parameter", 6, at)
            }
            Expr::Object(_) | Expr::Array(_) if plain => {
                self.refuse_at("destructuring pattern", 6, at)
            }
            Expr::Call { callee, .. }
                if !node.parenthesized
                    && self.expr_node(*callee).is_some_and(|c| {
                        !c.parenthesized && matches!(&c.expr, Expr::Identifier(n) if n == "async")
                    }) =>
            {
                self.refuse_at("async function", 10, start)
            }
            _ => self.unexpected(),
        }
    }

    /// Drops the nodes from `mark` on, the first reading of an arrow's parameters as an
    /// expression, which no node refers to any more; what they own is credited, and their
    /// places in the arena are taken by the nodes that come next.
    #[inline(never)]
    fn drop_cover(&mut self, mark: ExprId) {
        let m = self.m;
        while self.ast.exprs.len() > mark.0 as usize {
            if let Some(node) = self.ast.exprs.pop() {
                m.release(node.expr);
            }
        }
    }

    /// The `ConciseBody` of the arrow function `id` (`#sec-arrow-function-definitions`): a
    /// `FunctionBody` in braces, whose directive prologue may make the function strict, or an
    /// `ExpressionBody`, an `AssignmentExpression` with `in` as the context allows, held as one
    /// `return` statement.
    fn arrow_body(&mut self, id: FunctionId, no_in: bool) -> R<()> {
        self.enter()?;
        let outer = self.strict;
        if self.is(Punct::LBrace) {
            self.advance()?;
            self.functions += 1;
            let (body, use_strict) = self.body(false)?;
            self.function_close(id, outer, body, use_strict)?;
        } else {
            let e = self.assignment(no_in)?;
            let body = self.concise_body(e)?;
            self.function_end(id, outer, body, false)?;
        }
        self.leave();
        Ok(())
    }

    /// The body of an `ExpressionBody` `e`: one `return` statement of it, with its span.
    #[inline(never)]
    fn concise_body(&mut self, e: ExprId) -> R<Vec<StmtId>> {
        let span = self.expr_span(e);
        let stmt = self.push_stmt(Stmt::Return(Some(e)), span)?;
        let mut body = Vec::new();
        self.push_item(&mut body, stmt)?;
        Ok(body)
    }
}

/// A line terminator between `ArrowParameters` and `=>` (`#sec-arrow-function-definitions`:
/// `[no LineTerminator here]`).
const LINE_TERMINATOR_BEFORE_ARROW: &str = "line terminator before =>";
