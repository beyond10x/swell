//! Function definitions (15.2, `#sec-function-definitions`) and the accessors and plain methods
//! of object literals (`#sec-method-definitions`).
//!
//! The parameters are a list of `BindingIdentifier`s, with a trailing comma allowed
//! (`#sec-parameter-lists`); a default value, a rest parameter and a binding pattern are refused
//! (slice 6), as are generators (7). A
//! function's code is strict mode code when the code around it is or when its body starts with
//! a Use Strict Directive (`#sec-strict-mode-code`); the parser tracks that while it reads the
//! body, and the early errors that depend on it are checked afterwards (`src/early.rs`).

use super::{Parser, R};
use crate::ast::{Child, Function, FunctionId, FunctionKind, Name, Scope, Span, StmtId};
use crate::token::{Keyword, Punct};

impl<'a> Parser<'a> {
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

    /// `( FormalParameters )`; for a getter `()`, for a setter one `BindingIdentifier`.
    fn parameters(&mut self, kind: FunctionKind) -> R<Vec<Name>> {
        self.expect(Punct::LParen)?;
        let mut params = Vec::new();
        match kind {
            FunctionKind::Getter => {}
            FunctionKind::Setter => params.push(self.parameter()?),
            FunctionKind::Declaration | FunctionKind::Expression | FunctionKind::Method => {
                while !self.is(Punct::RParen) {
                    params.push(self.parameter()?);
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
            params.push(self.parameter()?);
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
        let depth = match self.ast.functions.get_mut(id.0 as usize) {
            Some(f) => {
                f.name = None;
                f.depth
            }
            None => return self.fail_here("parse failed"),
        };
        Ok(self.wrap_in_statement(id, span, depth))
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
}
