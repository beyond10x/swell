//! Function definitions (15.2, `#sec-function-definitions`) and the accessors of object literals
//! (`#sec-method-definitions`, the `get` and `set` forms ES5 has).
//!
//! The parameters are a list of `BindingIdentifier`s; a default value, a rest parameter and a
//! binding pattern are refused (slice 6), as are generators (7) and a trailing comma (2). A
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
            FunctionKind::Declaration | FunctionKind::Expression => {
                while !self.is(Punct::RParen) {
                    params.push(self.parameter()?);
                    if self.eat(Punct::Comma)? {
                        if self.is(Punct::RParen) {
                            return self
                                .refuse("trailing comma in an argument or parameter list", 2);
                        }
                    } else {
                        break;
                    }
                }
            }
        }
        self.expect(Punct::RParen)?;
        Ok(params)
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
