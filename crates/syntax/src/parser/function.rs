//! Function definitions (15.2, `#sec-function-definitions`), arrow functions (15.3,
//! `#sec-arrow-function-definitions`), the accessors and plain methods of object literals
//! (`#sec-method-definitions`) and generator declarations and expressions (15.5,
//! `#sec-generator-function-definitions`).
//!
//! The parameters are a list of `BindingIdentifier`s and binding patterns
//! (`#sec-destructuring-binding-patterns`), each with an optional `Initializer`, the last
//! possibly a `FunctionRestParameter` of either, with a trailing comma allowed after a
//! parameter that is no rest parameter (`#sec-parameter-lists`); `async` arrows (10) are
//! refused. A generator is parsed whole, its parameters and body with [Yield] set, and the
//! refusal of slice 7 at its `*` is held back to the end of the parse ([`Parser::held`]); the
//! parameters and body of every other function clear [Yield], but the parameters of an arrow
//! function, which keep the [Yield] around them. A
//! function's code is strict mode code when the code around it is or when its body starts with
//! a Use Strict Directive (`#sec-strict-mode-code`); the parser tracks that while it reads the
//! body, and the early errors that depend on it are checked afterwards (`src/early.rs`).

use super::{Parser, R};
use crate::ast::{
    AssignOp, BindingTarget, Child, Expr, ExprId, ExprNode, Function, FunctionId, FunctionKind,
    Name, PatternKind, Scope, Span, Stmt, StmtId,
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

    /// `function`, the `*` of a generator and the name: numbers the function, so functions are
    /// numbered in source order. The name is read with the [Yield] around it; that it is no
    /// `yield` where [Yield] is set, or in a GeneratorExpression (`BindingIdentifier[+Yield]`),
    /// is an early error (`src/early.rs`).
    #[inline(never)]
    fn function_head(&mut self, kind: FunctionKind) -> R<FunctionId> {
        let span = self.start();
        self.expect_keyword(Keyword::Function)?;
        let generator = self.is(Punct::Star);
        if generator {
            self.advance()?;
        }
        let name = if kind == FunctionKind::Expression && self.is(Punct::LParen) {
            None
        } else {
            Some(self.binding_identifier()?)
        };
        let id = self.begin_function(kind, span, name);
        if let Some(f) = self.ast.functions.get_mut(id.0 as usize) {
            f.generator = generator;
        }
        Ok(id)
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
            generator: false,
            params: Vec::new(),
            initializers: Vec::new(),
            rest: false,
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
        self.open_yield(id)?;
        self.parameters(id, kind)?;
        self.expect(Punct::LBrace)?;
        self.functions += 1;
        Ok(self.strict)
    }

    /// Saves the [Yield] around the function `id` and sets its own for its parameters and body:
    /// set for a generator, cleared for every other function that reads its parameters here
    /// (`#sec-generator-function-definitions`, `#sec-function-definitions`,
    /// `#sec-method-definitions`). [`Parser::function_end`] puts the saved one back.
    #[inline(never)]
    fn open_yield(&mut self, id: FunctionId) -> R<()> {
        let generator = self.ast.function(id).is_some_and(|f| f.generator);
        self.save_yield(generator)
    }

    /// Saves the current [Yield] on the parser's list and sets `yields`, the growth of the list
    /// charged first.
    #[inline(never)]
    fn save_yield(&mut self, yields: bool) -> R<()> {
        let m = self.m;
        let pushed = m.push(&mut self.outer_yields, self.yields);
        self.charged(pushed)?;
        self.yields = yields;
        Ok(())
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
    /// strictness, span and depth; the code after it has the strictness `outer` again, and the
    /// [Yield] saved as the function opened.
    #[inline(never)]
    fn function_end(
        &mut self,
        id: FunctionId,
        outer: bool,
        body: Vec<StmtId>,
        use_strict: bool,
    ) -> R<()> {
        self.strict = outer;
        if let Some(yields) = self.outer_yields.pop() {
            self.yields = yields;
        }
        let mut deepest = 0;
        for s in &body {
            deepest = deepest.max(self.ast.depth(Child::Stmt(*s)));
        }
        let (start, simple) = self
            .ast
            .function(id)
            .map_or((Span::default(), true), |f| (f.span, f.is_simple()));
        if let Some(f) = self.ast.function(id) {
            for e in f.initializers.iter().flatten() {
                deepest = deepest.max(self.ast.depth(Child::Expr(*e)));
            }
            for e in f.params.iter().filter_map(BindingTarget::pattern) {
                deepest = deepest.max(self.ast.depth(Child::Expr(e)));
            }
        }
        let depth = deepest.saturating_add(1);
        // `#sec-function-definitions-static-semantics-early-errors`,
        // `#sec-arrow-function-definitions-static-semantics-early-errors` and
        // `#sec-method-definitions-static-semantics-early-errors`: FunctionBodyContainsUseStrict
        // with a parameter list that is not simple.
        if use_strict && !simple {
            return self.fail_at(USE_STRICT_WITH_PARAMETER_LIST, start);
        }
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

    /// `( FormalParameters )` of function `id`, into its `params`, `initializers` and `rest`;
    /// for a getter `()`, for a setter one `FormalParameter`, which may have an initializer and
    /// is no rest parameter (`#sec-method-definitions-static-semantics-early-errors`). An arrow
    /// function's `ArrowFormalParameters` have the same grammar, but its parser reads them as
    /// the parenthesized expression they cover ([`Parser::arrow_function`]).
    fn parameters(&mut self, id: FunctionId, kind: FunctionKind) -> R<()> {
        self.expect(Punct::LParen)?;
        match kind {
            FunctionKind::Getter => {}
            FunctionKind::Setter => {
                if self.is(Punct::Ellipsis) {
                    return self.fail_here(SETTER_REST);
                }
                self.parameter(id)?;
            }
            FunctionKind::Declaration
            | FunctionKind::Expression
            | FunctionKind::Method
            | FunctionKind::Arrow => self.parameter_list(id, Some(Punct::RParen))?,
        }
        self.expect(Punct::RParen)
    }

    /// `FormalParameters` of function `id` up to `end` (a `)`, or the end of the input when
    /// `end` is `None`): `FormalParameter`s separated by commas, a trailing comma allowed after
    /// one, and a `FunctionRestParameter` only last, with no comma after it
    /// (`#sec-function-definitions`).
    fn parameter_list(&mut self, id: FunctionId, end: Option<Punct>) -> R<()> {
        let at_end = |p: &Self| match end {
            Some(end) => p.is(end),
            None => p.at_eof(),
        };
        while !at_end(self) {
            if self.parameter(id)? {
                if !at_end(self) {
                    return self.fail_here(REST_NOT_LAST);
                }
                break;
            }
            if !self.eat(Punct::Comma)? {
                break;
            }
        }
        Ok(())
    }

    /// The parameter text of CreateDynamicFunction (`#sec-createdynamicfunction`, step 16) alone,
    /// the whole source as `FormalParameters[~Yield, ~Await]`: the parameters, a trailing comma
    /// allowed, to the end of the input, read into a function of this parse that the answer
    /// drops. An empty text is the empty list; any other text enters the nesting bound once, as
    /// a script does.
    pub(super) fn formal_parameters_text(&mut self) -> R<()> {
        if self.src.is_empty() {
            return Ok(());
        }
        self.enter()?;
        self.advance()?;
        let start = self.start();
        let id = self.begin_function(FunctionKind::Expression, start, None);
        self.parameter_list(id, None)?;
        if !self.at_eof() {
            return self.unexpected();
        }
        self.leave();
        Ok(())
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

    /// One `FormalParameter` of function `id`, a `BindingIdentifier` or a `BindingPattern` with
    /// an optional `Initializer`, or a `FunctionRestParameter`, `... BindingIdentifier` or
    /// `... BindingPattern`; whether it was the rest parameter.
    fn parameter(&mut self, id: FunctionId) -> R<bool> {
        let rest = self.eat(Punct::Ellipsis)?;
        // A pattern is read here, so that the frame on the stack while it is read holds ids.
        let pattern = self.pattern_literal()?;
        self.parameter_head(id, rest, pattern)?;
        if !rest && self.is(Punct::Eq) {
            self.parameter_initializer(id)?;
        }
        Ok(rest)
    }

    /// The pattern `pattern` read, or the `BindingIdentifier` at the current token, of a
    /// parameter of function `id` (after `...` when `rest`), appended to its `params` (and a
    /// `None` to its `initializers` when they are kept), the growth of each list charged first.
    #[inline(never)]
    fn parameter_head(&mut self, id: FunctionId, rest: bool, pattern: Option<ExprId>) -> R<()> {
        let name = self.binding_target(pattern, PatternKind::Initialize)?;
        let m = self.m;
        let pushed = match self.ast.functions.get_mut(id.0 as usize) {
            Some(f) => {
                f.rest = rest;
                let pushed = m.push(&mut f.params, name);
                if pushed.is_ok() && !f.initializers.is_empty() {
                    m.push(&mut f.initializers, None)
                } else {
                    pushed
                }
            }
            None => {
                m.release(name);
                Ok(())
            }
        };
        self.charged(pushed)
    }

    /// The `Initializer` of the last parameter of function `id`, at its `=`: an
    /// `AssignmentExpression` with `in` allowed. The first initializer of the list makes its
    /// `initializers`, a `None` for each parameter before it, charged first.
    #[inline(never)]
    fn parameter_initializer(&mut self, id: FunctionId) -> R<()> {
        self.enter()?;
        self.advance()?;
        let e = self.assignment(false)?;
        let m = self.m;
        let set = match self.ast.functions.get_mut(id.0 as usize) {
            Some(f) => {
                let made = if f.initializers.is_empty() {
                    let n = f.params.len();
                    m.grow_by(&mut f.initializers, n)
                        .map(|()| f.initializers.resize(n, None))
                } else {
                    Ok(())
                };
                if let Some(last) = f.initializers.last_mut() {
                    *last = Some(e);
                }
                made
            }
            None => Ok(()),
        };
        self.charged(set)?;
        self.leave();
        Ok(())
    }

    // ---- arrow functions -----------------------------------------------------------------------

    /// `ArrowFunction : ArrowParameters [no LineTerminator here] => ConciseBody`
    /// (`#sec-arrow-function-definitions`) at the current `=>`, after the operand `left` of an
    /// `AssignmentExpression` that started at `start`: the operand read again as the
    /// parameters, then the body. The function is numbered here, after every function before
    /// its first token and every function in the initializers of its parameters.
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
        let (params, initializers, rest) = self.arrow_parameters(left, start)?;
        self.advance()?;
        let id = self.begin_function(FunctionKind::Arrow, start, None);
        match self.ast.functions.get_mut(id.0 as usize) {
            Some(f) => {
                f.params = params;
                f.initializers = initializers;
                f.rest = rest;
            }
            None => {
                self.m.release(params);
                self.m.release(initializers);
            }
        }
        Ok(id)
    }

    /// `ArrowParameters` from the operand `left` before `=>`: a `BindingIdentifier` (an
    /// identifier not in parentheses), or the parenthesized expression, in one pair of
    /// parentheses ([`Parser::parenthesized`]), read again as `ArrowFormalParameters`
    /// (`#sec-arrow-function-definitions-static-semantics-early-errors`, the cover's
    /// refinement): an identifier, an object or array literal read again as a binding pattern,
    /// or an assignment with `=` to either (a parameter with its initializer), or a comma
    /// expression of those, none in parentheses of its own, the last one possibly the rest
    /// element the cover read ([`Parser::cover_rest`]); or the empty list of `()`. `async`
    /// before parentheses is an async arrow, refused.
    /// The names move from the expression's nodes into the list; with no initializer and no
    /// pattern the nodes of the first reading are dropped and what they own credited, otherwise
    /// they stay in the arena, unreferenced but by the initializers' and the patterns' own, and
    /// the list of the comma expression is credited. Answers the names, the
    /// initializers (empty when there is none) and whether the last name is a rest parameter;
    /// the growth of each list is charged first.
    #[inline(never)]
    fn arrow_parameters(&mut self, left: ExprId, start: Span) -> R<ArrowParameters> {
        let elements = match self.expr_node(left) {
            Some(n) => match &n.expr {
                Expr::Identifier(_) => Some(1),
                Expr::Assign {
                    op: AssignOp::Assign,
                    ..
                } if n.parenthesized => Some(1),
                Expr::Sequence(list) if n.parenthesized => Some(list.len()),
                Expr::Object(_) | Expr::Array(_) | Expr::Spread(_) if n.parenthesized => Some(1),
                _ => None,
            },
            None => return self.unexpected(),
        };
        let Some(elements) = elements else {
            return self.arrow_refusal(left, start);
        };
        let mut mark = left;
        let mut defaults = false;
        let mut patterns = false;
        for i in 0..elements {
            let e = self.arrow_element(left, i);
            mark = mark.min(e);
            // `left` itself is in no parentheses or in the one pair of the cover; an operand of
            // the cover's comma expression is in none.
            let (element, bare) = self.expr_node(e).map_or((Element::Other, false), |n| {
                (self.element(n), !n.parenthesized)
            });
            if element == Element::Other || !(bare || e == left) {
                return self.arrow_refusal(e, start);
            }
            defaults |= matches!(element, Element::Default | Element::PatternDefault);
            patterns |= matches!(element, Element::Pattern | Element::PatternDefault);
        }
        let rest = elements > 0 && self.is_cover_rest(self.arrow_element(left, elements - 1));
        let made = self.m.with_capacity(elements);
        let mut params = self.charged(made)?;
        let made = self.m.with_capacity(if defaults { elements } else { 0 });
        let mut initializers = self.charged(made)?;
        for i in 0..elements {
            let e = self.arrow_element(left, i);
            let (target, value) = match self.expr_node(e).map(|n| &n.expr) {
                Some(Expr::Assign { target, value, .. }) => (*target, Some(*value)),
                Some(Expr::Spread(pattern)) => (*pattern, None),
                _ => (e, None),
            };
            let span = self.expr_span(target);
            match self.expr_mut(target) {
                Some(Expr::Identifier(name)) => {
                    let name = std::mem::take(name);
                    params.push(BindingTarget::Identifier(Name { name, span }));
                }
                Some(
                    Expr::Object(_)
                    | Expr::ObjectPattern { .. }
                    | Expr::Array(_)
                    | Expr::ArrayPattern { .. },
                ) => {
                    let pattern = self
                        .reread_as_pattern(target, PatternKind::Initialize)
                        .and_then(|()| self.bound_names(target));
                    match pattern {
                        Ok(pattern) => params.push(BindingTarget::Pattern(pattern)),
                        Err(stop) => {
                            self.m.release(params);
                            self.m.release(initializers);
                            return Err(stop);
                        }
                    }
                }
                _ => {}
            }
            if defaults {
                initializers.push(value);
            }
        }
        if defaults || patterns {
            let m = self.m;
            if let Some(Expr::Sequence(list)) = self.expr_mut(left) {
                m.release(std::mem::take(list));
            }
        } else {
            self.drop_cover(mark);
        }
        Ok((params, initializers, rest))
    }

    /// What the element node `n` of an operand before `=>` is as a parameter.
    fn element(&self, n: &ExprNode) -> Element {
        match &n.expr {
            Expr::Identifier(_) => Element::Name,
            // A spread is the cover's rest element of a pattern ([`Parser::cover_rest`]).
            Expr::Object(_) | Expr::Array(_) | Expr::Spread(_) => Element::Pattern,
            Expr::Assign {
                op: AssignOp::Assign,
                target,
                ..
            } => match self.expr_node(*target) {
                Some(t) if !t.parenthesized && matches!(t.expr, Expr::Identifier(_)) => {
                    Element::Default
                }
                Some(t)
                    if !t.parenthesized
                        && matches!(
                            t.expr,
                            Expr::ObjectPattern { .. } | Expr::ArrayPattern { .. }
                        ) =>
                {
                    Element::PatternDefault
                }
                _ => Element::Other,
            },
            _ => Element::Other,
        }
    }

    /// Whether the element `e` is the rest element the cover read: the identifier whose span
    /// starts at the `...` before it, or the spread of a pattern ([`Parser::cover_rest`]); no
    /// identifier's own text starts with a `.`.
    fn is_cover_rest(&self, e: ExprId) -> bool {
        self.expr_node(e).is_some_and(|n| {
            matches!(n.expr, Expr::Spread(_))
                || matches!(n.expr, Expr::Identifier(_))
                    && self.src.get(n.span.start as usize) == Some(&u16::from(b'.'))
        })
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
    /// the refusal a later story lifts (`async function` for a call of `async`), or the `=>`
    /// unexpected.
    #[inline(never)]
    fn arrow_refusal<T>(&mut self, e: ExprId, start: Span) -> R<T> {
        let Some(node) = self.expr_node(e) else {
            return self.unexpected();
        };
        match &node.expr {
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
    /// `return` statement. Either has [Yield] cleared (`ConciseBody`, `ExpressionBody[?In,
    /// ~Await]` of `AssignmentExpression[?In, ~Yield, ?Await]`, `FunctionBody[~Yield, ~Await]`).
    fn arrow_body(&mut self, id: FunctionId, no_in: bool) -> R<()> {
        self.enter()?;
        self.save_yield(false)?;
        let outer = self.strict;
        if self.is(Punct::LBrace) {
            self.advance()?;
            self.functions += 1;
            let (body, use_strict) = self.body(false)?;
            self.function_close(id, outer, body, use_strict)?;
        } else {
            let e = self.assignment(no_in)?;
            self.concise_body(id, outer, e)?;
        }
        self.leave();
        Ok(())
    }

    /// The body of the arrow function `id` from its `ExpressionBody` `e`: one `return`
    /// statement of it, with its span, recorded as [`Parser::function_end`] does. Not inlined,
    /// so that the list stays out of the frame of [`Parser::arrow_body`], which is on the
    /// recursion of nested arrows.
    #[inline(never)]
    fn concise_body(&mut self, id: FunctionId, outer: bool, e: ExprId) -> R<()> {
        let span = self.expr_span(e);
        let stmt = self.push_stmt(Stmt::Return(Some(e)), span)?;
        let mut body = Vec::new();
        self.push_item(&mut body, stmt)?;
        self.function_end(id, outer, body, false)
    }
}

/// A line terminator between `ArrowParameters` and `=>` (`#sec-arrow-function-definitions`:
/// `[no LineTerminator here]`).
const LINE_TERMINATOR_BEFORE_ARROW: &str = "line terminator before =>";

/// A `FunctionRestParameter` or an arrow's rest element with a parameter or a comma after it
/// (`#sec-function-definitions`, `#sec-arrow-function-definitions`).
pub(super) const REST_NOT_LAST: &str = "a rest parameter must be the last parameter";

/// The rest element of a parenthesized list that no `=>` follows: only
/// `ArrowFormalParameters` take one (`#sec-arrow-function-definitions`).
pub(super) const REST_WITHOUT_ARROW: &str = "expected => after a rest parameter";

/// A setter's parameter that is a rest parameter
/// (`#sec-method-definitions-static-semantics-early-errors`).
const SETTER_REST: &str = "a setter may not have a rest parameter";

/// A Use Strict Directive in the body of a function whose parameter list is not simple
/// (`#sec-function-definitions-static-semantics-early-errors`).
const USE_STRICT_WITH_PARAMETER_LIST: &str =
    "use strict directive in a function whose parameter list is not simple";

/// What an arrow's parameters are: the names and patterns, the initializers beside them (empty
/// when there is none) and whether the last one is a rest parameter.
type ArrowParameters = (Vec<BindingTarget>, Vec<Option<ExprId>>, bool);

/// What an element of the operand before `=>` is as a parameter.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Element {
    /// An identifier.
    Name,
    /// An assignment with `=` to an identifier not in parentheses: a name and its initializer.
    Default,
    /// An object literal: an ObjectBindingPattern.
    Pattern,
    /// An assignment with `=` to an object literal not in parentheses, which the assignment
    /// read as an ObjectAssignmentPattern: a pattern and its initializer.
    PatternDefault,
    /// Anything else, no parameter.
    Other,
}
