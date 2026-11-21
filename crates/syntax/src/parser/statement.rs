//! Statements and declarations (clause 14, `#sec-ecmascript-language-statements-and-declarations`),
//! and the bodies of scripts and functions with their directive prologues
//! (`#sec-directive-prologues-and-the-use-strict-directive`).
//!
//! Every ES5 statement is parsed but `with` in non-strict code, which is refused (slice 2, with
//! its run time), as are the declarations of later slices: a function declaration inside a
//! block, `let` and `const` (6), `class` (8), `async function` (10), `import` and `export` (11),
//! and the Annex B forms `if (x) function f() {}`, `L: function f() {}` and
//! `for (var a = 0 in o)` (18). Labelled statements and labelled `break` and `continue`
//! (`#sec-labelled-statements`), a `catch` without a parameter (`#sec-try-statement`) and
//! automatic semicolon insertion (`#sec-rules-of-automatic-semicolon-insertion`) are slice 2's.
//!
//! As in `expression.rs`, the functions that stay on the Rust stack while the parser recurses are
//! kept small; [`Parser::statement`] picks the parser of a statement through one function
//! pointer.

use super::{Parser, R};
use crate::ast::{
    CatchClause, Expr, ExprId, ForInTarget, ForInit, Name, Span, Stmt, StmtId, SwitchCase,
    VarDeclarator,
};
use crate::early;
use crate::scope;
use crate::token::{Keyword, Punct, TokenKind};

/// Where a statement stands, which decides what a `function` there is.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Place {
    /// The top level of a script or function body: a `FunctionDeclaration` is a
    /// `HoistableDeclaration` of the body.
    Body,
    /// A `StatementList` inside a block, a `case` clause or a `try`, `catch` or `finally` block,
    /// where a function declaration is lexically scoped (slice 6).
    Block,
    /// The `Statement` of an `if` clause, where Annex B allows a function declaration in
    /// non-strict code (`#sec-functiondeclarations-in-ifstatement-statement-clauses`).
    IfClause,
    /// The `Statement` of a loop, where no declaration stands.
    Statement,
    /// The `LabelledItem` of a labelled statement at the top of a body or in a block, where
    /// Annex B allows a function declaration in non-strict code
    /// (`#sec-labelled-function-declarations`).
    Labelled,
    /// The `LabelledItem` of a labelled statement that is itself the statement of a loop or an
    /// `if` clause, where IsLabelledFunction rules a function declaration out in any code
    /// (`#sec-labelled-function-declarations`, Note).
    LabelledInStatement,
}

type StmtParser<'a> = fn(&mut Parser<'a>) -> R<StmtId>;

impl<'a> Parser<'a> {
    /// The statements of a script (`top` true) or a function body, up to the end of the input or
    /// the `}` that closes the body; and whether its directive prologue holds a Use Strict
    /// Directive, which makes the rest of the body strict mode code.
    pub(super) fn body(&mut self, top: bool) -> R<(Vec<StmtId>, bool)> {
        self.enter()?;
        let mut body = Vec::new();
        let mut prologue = true;
        let mut use_strict = false;
        while !self.at_eof() && (top || !self.is(Punct::RBrace)) {
            let directive = self.directive_candidate(prologue);
            let stmt = self.statement(Place::Body)?;
            body.push(stmt);
            prologue = self.directive_end(stmt, directive, &mut use_strict);
        }
        self.leave();
        Ok((body, use_strict))
    }

    /// The offsets of the string literal at the current token, while in the prologue.
    #[inline(never)]
    fn directive_candidate(&self, prologue: bool) -> Option<(u32, u32)> {
        match &self.tok.kind {
            TokenKind::String(_) if prologue => Some((self.tok.start, self.tok.end)),
            _ => None,
        }
    }

    /// After a statement of the prologue: whether the prologue goes on. A Use Strict Directive
    /// makes the code from here on strict.
    #[inline(never)]
    fn directive_end(
        &mut self,
        stmt: StmtId,
        directive: Option<(u32, u32)>,
        use_strict: &mut bool,
    ) -> bool {
        let Some((start, end)) = directive else {
            return false;
        };
        if !self.is_directive(stmt, start, end) {
            return false;
        }
        let raw = self.src.get(start as usize..end as usize).unwrap_or(&[]);
        if scope::is_use_strict(raw) {
            *use_strict = true;
            self.strict = true;
        }
        true
    }

    /// Whether `stmt` is an `ExpressionStatement` that consists entirely of the string literal
    /// token at `start..end`.
    #[inline(never)]
    fn is_directive(&self, stmt: StmtId, start: u32, end: u32) -> bool {
        let Some(Stmt::Expression(e)) = self.ast.stmt(stmt).map(|n| &n.stmt) else {
            return false;
        };
        self.expr_node(*e).is_some_and(|n| {
            matches!(n.expr, Expr::String(_))
                && !n.parenthesized
                && n.span.start == start
                && n.span.end == end
        })
    }

    /// A `Statement` or `Declaration` standing at `place`.
    pub(super) fn statement(&mut self, place: Place) -> R<StmtId> {
        self.enter()?;
        let parse = self.statement_kind(place)?;
        let s = parse(self)?;
        self.leave();
        Ok(s)
    }

    /// Which parser takes the statement at the current token; refuses the statements of later
    /// slices and reports the ones that cannot stand at `place`.
    #[inline(never)]
    fn statement_kind(&mut self, place: Place) -> R<StmtParser<'a>> {
        Ok(match &self.tok.kind {
            TokenKind::Punct(Punct::LBrace) => Parser::block_statement,
            TokenKind::Punct(Punct::Semicolon) => Parser::empty_statement,
            TokenKind::Keyword(k) => match *k {
                Keyword::Var => Parser::var_statement,
                Keyword::If => Parser::if_statement,
                Keyword::Do => Parser::do_while,
                Keyword::While => Parser::while_statement,
                Keyword::For => Parser::for_statement,
                Keyword::Continue | Keyword::Break => Parser::break_or_continue,
                Keyword::Return => Parser::return_statement,
                Keyword::With => {
                    if self.strict {
                        return self.fail_here(early::WITH_IN_STRICT);
                    }
                    return self.refuse("with statement in non-strict code", 2);
                }
                Keyword::Switch => Parser::switch_statement,
                Keyword::Throw => Parser::throw_statement,
                Keyword::Try => Parser::try_statement,
                Keyword::Debugger => Parser::debugger_statement,
                Keyword::Function => match place {
                    Place::Body => Parser::function_declaration_statement,
                    Place::Block => {
                        return self.refuse("function declaration inside a block", 6);
                    }
                    Place::IfClause if !self.strict => {
                        return self.refuse("function declaration as an if clause", 18);
                    }
                    Place::IfClause | Place::Statement => {
                        return self.fail_here("function declaration as the body of a statement");
                    }
                    Place::Labelled if !self.strict => {
                        return self.refuse("labelled function declaration", 18);
                    }
                    Place::Labelled => {
                        return self.fail_here("labelled function declaration in strict mode code");
                    }
                    Place::LabelledInStatement => {
                        return self
                            .fail_here("labelled function declaration as the body of a statement");
                    }
                },
                Keyword::Class => return self.refuse("class", 8),
                Keyword::Const => return self.refuse("const declaration", 6),
                Keyword::Export => return self.refuse("export declaration", 11),
                Keyword::Import => {
                    let next = self.peek()?.kind.clone();
                    return match next {
                        TokenKind::Punct(Punct::Dot) => self.refuse("import.meta", 11),
                        TokenKind::Punct(Punct::LParen) => self.refuse("dynamic import", 11),
                        _ => self.refuse("import declaration", 11),
                    };
                }
                _ => Parser::expression_statement,
            },
            TokenKind::Name(name) => {
                let plain = !self.tok.escaped;
                let (is_let, is_async) = (plain && name == "let", plain && name == "async");
                let next = self.peek()?;
                if next.kind == TokenKind::Punct(Punct::Colon) {
                    return Ok(match place {
                        Place::Statement | Place::IfClause | Place::LabelledInStatement => {
                            Parser::labelled_statement_in_statement
                        }
                        Place::Body | Place::Block | Place::Labelled => Parser::labelled_statement,
                    });
                }
                if is_let
                    && matches!(
                        next.kind,
                        TokenKind::Name(_)
                            | TokenKind::Punct(Punct::LBracket)
                            | TokenKind::Punct(Punct::LBrace)
                    )
                {
                    return self.refuse("let declaration", 6);
                }
                if is_async
                    && next.kind == TokenKind::Keyword(Keyword::Function)
                    && !next.newline_before
                {
                    return self.refuse("async function", 10);
                }
                Parser::expression_statement
            }
            _ => Parser::expression_statement,
        })
    }

    /// Pushes `stmt`, which started at `start` and ends at the previous token.
    #[inline(never)]
    fn make_stmt(&mut self, stmt: Stmt, start: Span) -> R<StmtId> {
        let span = self.finish(start);
        self.push_stmt(stmt, span)
    }

    fn empty_statement(&mut self) -> R<StmtId> {
        let start = self.start();
        self.advance()?;
        self.make_stmt(Stmt::Empty, start)
    }

    fn debugger_statement(&mut self) -> R<StmtId> {
        let start = self.start();
        self.advance()?;
        self.semicolon()?;
        self.make_stmt(Stmt::Debugger, start)
    }

    /// `continue LabelIdentifier? ;` and `break LabelIdentifier? ;`: a restricted production, so a
    /// label on the next line is a statement of its own.
    fn break_or_continue(&mut self) -> R<StmtId> {
        let start = self.start();
        let is_break = self.is_keyword(Keyword::Break);
        self.advance()?;
        let label = if matches!(self.tok.kind, TokenKind::Name(_)) && !self.ends_statement_here() {
            Some(self.binding_identifier()?)
        } else {
            None
        };
        self.semicolon()?;
        let stmt = if is_break {
            Stmt::Break(label)
        } else {
            Stmt::Continue(label)
        };
        self.make_stmt(stmt, start)
    }

    /// `LabelIdentifier : LabelledItem` (`#sec-labelled-statements`) at the top of a body or in a
    /// block.
    fn labelled_statement(&mut self) -> R<StmtId> {
        self.labelled(Place::Labelled)
    }

    /// A labelled statement that is the statement of a loop or an `if` clause.
    fn labelled_statement_in_statement(&mut self) -> R<StmtId> {
        self.labelled(Place::LabelledInStatement)
    }

    fn labelled(&mut self, item: Place) -> R<StmtId> {
        self.enter()?;
        let start = self.start();
        let id = self.label_head()?;
        let body = self.statement(item)?;
        if let Some(Stmt::Labelled { body: b, .. }) = self.stmt_mut(id) {
            *b = body;
        }
        let id = self.complete_stmt(id, start)?;
        self.leave();
        Ok(id)
    }

    /// The `LabelIdentifier :` of a labelled statement: reserves its node.
    #[inline(never)]
    fn label_head(&mut self) -> R<StmtId> {
        let label = self.binding_identifier()?;
        self.expect(Punct::Colon)?;
        Ok(self.reserve_stmt(Stmt::Labelled {
            label,
            body: StmtId(u32::MAX),
        }))
    }

    fn expression_statement(&mut self) -> R<StmtId> {
        self.enter()?;
        let start = self.start();
        let e = self.expression(false)?;
        self.semicolon()?;
        let s = self.make_stmt(Stmt::Expression(e), start)?;
        self.leave();
        Ok(s)
    }

    fn function_declaration_statement(&mut self) -> R<StmtId> {
        self.enter()?;
        let start = self.start();
        let id = self.function_declaration()?;
        let s = self.make_stmt(Stmt::FunctionDeclaration(id), start)?;
        self.leave();
        Ok(s)
    }

    fn block_statement(&mut self) -> R<StmtId> {
        self.enter()?;
        let start = self.start();
        let id = self.reserve_stmt(Stmt::Block(Vec::new()));
        self.statement_list(id, List::Block)?;
        let id = self.complete_stmt(id, start)?;
        self.leave();
        Ok(id)
    }

    /// `{ StatementList }`, each statement appended to the list `list` of the statement `id`.
    fn statement_list(&mut self, id: StmtId, list: List) -> R<()> {
        self.enter()?;
        self.expect(Punct::LBrace)?;
        while self.block_goes_on()? {
            let s = self.statement(Place::Block)?;
            self.append(id, list, s);
        }
        self.leave();
        Ok(())
    }

    /// Appends `s` to the list `list` of the statement `id`.
    #[inline(never)]
    fn append(&mut self, id: StmtId, list: List, s: StmtId) {
        let target = match (self.stmt_mut(id), list) {
            (Some(Stmt::Block(body)), List::Block) => body,
            (Some(Stmt::Try { block, .. }), List::Try) => block,
            (
                Some(Stmt::Try {
                    handler: Some(h), ..
                }),
                List::Catch,
            ) => &mut h.body,
            (
                Some(Stmt::Try {
                    finalizer: Some(f), ..
                }),
                List::Finally,
            ) => f,
            (Some(Stmt::Switch { cases, .. }), List::Case) => match cases.last_mut() {
                Some(c) => &mut c.body,
                None => return,
            },
            _ => return,
        };
        target.push(s);
    }

    /// Consumes the `}` that closes a block; whether a statement follows instead.
    fn block_goes_on(&mut self) -> R<bool> {
        if self.eat(Punct::RBrace)? {
            return Ok(false);
        }
        if self.at_eof() {
            return self.unexpected();
        }
        Ok(true)
    }

    /// A `BindingIdentifier` or a `LabelIdentifier`; a name written with an escape may not spell a
    /// reserved word. A binding pattern is refused (slice 6).
    #[inline(never)]
    pub(super) fn binding_identifier(&mut self) -> R<Name> {
        self.check_escaped_keyword()?;
        let span = self.start();
        match &self.tok.kind {
            TokenKind::Name(n) => {
                let name = n.clone();
                self.advance()?;
                Ok(Name {
                    name,
                    span: self.finish(span),
                })
            }
            TokenKind::Punct(Punct::LBracket) | TokenKind::Punct(Punct::LBrace) => {
                self.refuse("destructuring pattern", 6)
            }
            _ => self.unexpected(),
        }
    }

    fn var_statement(&mut self) -> R<StmtId> {
        self.enter()?;
        let start = self.start();
        let id = self.reserve_stmt(Stmt::Var(Vec::new()));
        self.advance()?;
        self.var_declarations(id, false)?;
        self.semicolon()?;
        let id = self.complete_stmt(id, start)?;
        self.leave();
        Ok(id)
    }

    /// A `VariableDeclarationList`, appended to the declarators of `id` (a `var` statement or a
    /// `for` whose head is one); `no_in` in the head of a `for`.
    fn var_declarations(&mut self, id: StmtId, no_in: bool) -> R<()> {
        self.enter()?;
        loop {
            self.declarator_name(id)?;
            if self.eat(Punct::Eq)? {
                let e = self.assignment(no_in)?;
                self.declarator_init(id, e);
            }
            if !self.eat(Punct::Comma)? {
                break;
            }
        }
        self.leave();
        Ok(())
    }

    fn declarators(&mut self, id: StmtId) -> Option<&mut Vec<VarDeclarator>> {
        match self.stmt_mut(id) {
            Some(Stmt::Var(list)) => Some(list),
            Some(Stmt::For {
                init: Some(ForInit::Var(list)),
                ..
            }) => Some(list),
            _ => None,
        }
    }

    /// The `BindingIdentifier` of a declarator, appended to the declarators of `id`.
    #[inline(never)]
    fn declarator_name(&mut self, id: StmtId) -> R<()> {
        let name = self.binding_identifier()?;
        if let Some(list) = self.declarators(id) {
            list.push(VarDeclarator { name, init: None });
        }
        Ok(())
    }

    #[inline(never)]
    fn declarator_init(&mut self, id: StmtId, e: ExprId) {
        if let Some(d) = self.declarators(id).and_then(|l| l.last_mut()) {
            d.init = Some(e);
        }
    }

    /// `( Expression )` after `if`, `while` and `switch`, with the keyword before it.
    fn condition(&mut self) -> R<ExprId> {
        self.enter()?;
        self.advance()?;
        self.expect(Punct::LParen)?;
        let e = self.expression(false)?;
        self.expect(Punct::RParen)?;
        self.leave();
        Ok(e)
    }

    fn if_statement(&mut self) -> R<StmtId> {
        self.enter()?;
        let start = self.start();
        let test = self.condition()?;
        let consequent = self.statement(Place::IfClause)?;
        let alternate = self.else_clause()?;
        let s = self.make_stmt(
            Stmt::If {
                test,
                consequent,
                alternate,
            },
            start,
        )?;
        self.leave();
        Ok(s)
    }

    fn else_clause(&mut self) -> R<Option<StmtId>> {
        if !self.is_keyword(Keyword::Else) {
            return Ok(None);
        }
        self.enter()?;
        self.advance()?;
        let s = self.statement(Place::IfClause)?;
        self.leave();
        Ok(Some(s))
    }

    fn do_while(&mut self) -> R<StmtId> {
        self.enter()?;
        let start = self.start();
        self.advance()?;
        let body = self.statement(Place::Statement)?;
        let test = self.do_while_end()?;
        let s = self.make_stmt(Stmt::DoWhile { body, test }, start)?;
        self.leave();
        Ok(s)
    }

    /// `while ( Expression ) ;` after the body of a do-while. A missing `;` after its `)` is
    /// inserted whatever follows (`#sec-rules-of-automatic-semicolon-insertion`, rule 1, third
    /// case).
    fn do_while_end(&mut self) -> R<ExprId> {
        if !self.is_keyword(Keyword::While) {
            return self.unexpected();
        }
        let test = self.condition()?;
        self.eat(Punct::Semicolon)?;
        Ok(test)
    }

    fn while_statement(&mut self) -> R<StmtId> {
        self.enter()?;
        let start = self.start();
        let test = self.condition()?;
        let body = self.statement(Place::Statement)?;
        let s = self.make_stmt(Stmt::While { test, body }, start)?;
        self.leave();
        Ok(s)
    }

    /// `for (;;)` and `for-in` (`#sec-for-statement`, `#sec-for-in-and-for-of-statements`);
    /// `for-of`, `for await` and lexical declarations in the head are refused.
    fn for_statement(&mut self) -> R<StmtId> {
        self.enter()?;
        let start = self.start();
        let id = self.reserve_stmt(Stmt::For {
            init: None,
            test: None,
            update: None,
            body: StmtId(u32::MAX),
        });
        self.for_head(id)?;
        if self.is_keyword(Keyword::In) {
            self.for_in_head(id, start)?;
            self.for_in_rest(id)?;
        } else {
            self.for_rest(id)?;
        }
        let id = self.complete_stmt(id, start)?;
        self.leave();
        Ok(id)
    }

    /// `for (` and what stands before the first `;` or `in`, recorded as the `init` of `id`.
    fn for_head(&mut self, id: StmtId) -> R<()> {
        self.enter()?;
        self.advance()?;
        if self.is_name("await") {
            return self.refuse("await expression", 10);
        }
        self.expect(Punct::LParen)?;
        if self.is_keyword(Keyword::Var) {
            self.advance()?;
            self.set_for_init(id, None);
            self.var_declarations(id, true)?;
        } else if !self.is(Punct::Semicolon) {
            self.for_head_refusals()?;
            let e = self.expression(true)?;
            self.set_for_init(id, Some(e));
        }
        if self.is_name("of") && self.has_for_init(id) {
            return self.refuse("for-of statement", 6);
        }
        self.leave();
        Ok(())
    }

    /// Sets the `init` of the `for` statement `id`: an expression, or an empty `var` list.
    #[inline(never)]
    fn set_for_init(&mut self, id: StmtId, e: Option<ExprId>) {
        if let Some(Stmt::For { init, .. }) = self.stmt_mut(id) {
            *init = Some(match e {
                Some(e) => ForInit::Expression(e),
                None => ForInit::Var(Vec::new()),
            });
        }
    }

    fn has_for_init(&self, id: StmtId) -> bool {
        matches!(
            self.ast.stmt(id).map(|n| &n.stmt),
            Some(Stmt::For { init: Some(_), .. })
        )
    }

    /// Refuses `const` and `let` declarations in the head of a `for`.
    #[inline(never)]
    fn for_head_refusals(&mut self) -> R<()> {
        if self.is_keyword(Keyword::Const) {
            return self.refuse("const declaration", 6);
        }
        if self.is_name("let") {
            let next = self.peek()?;
            if matches!(
                next.kind,
                TokenKind::Name(_)
                    | TokenKind::Punct(Punct::LBracket)
                    | TokenKind::Punct(Punct::LBrace)
            ) {
                return self.refuse("let declaration", 6);
            }
        }
        Ok(())
    }

    /// Sets a part of the loop `id` once it is parsed.
    #[inline(never)]
    fn set_loop_part(&mut self, id: StmtId, part: LoopPart) {
        match (self.stmt_mut(id), part) {
            (Some(Stmt::For { test, .. }), LoopPart::Test(e)) => *test = e,
            (Some(Stmt::For { update, .. }), LoopPart::Update(e)) => *update = e,
            (Some(Stmt::ForIn { object, .. }), LoopPart::Object(e)) => *object = e,
            (Some(Stmt::For { body, .. } | Stmt::ForIn { body, .. }), LoopPart::Body(s)) => {
                *body = s;
            }
            _ => {}
        }
    }

    /// The rest of `for ( head ; test ; update ) Statement` from the first `;`.
    fn for_rest(&mut self, id: StmtId) -> R<()> {
        self.enter()?;
        self.expect(Punct::Semicolon)?;
        let test = self.optional_expression(Punct::Semicolon)?;
        self.set_loop_part(id, LoopPart::Test(test));
        self.expect(Punct::Semicolon)?;
        let update = self.optional_expression(Punct::RParen)?;
        self.set_loop_part(id, LoopPart::Update(update));
        self.expect(Punct::RParen)?;
        let body = self.statement(Place::Statement)?;
        self.set_loop_part(id, LoopPart::Body(body));
        self.leave();
        Ok(())
    }

    /// An `Expression` unless the token `end` stands at the current position.
    fn optional_expression(&mut self, end: Punct) -> R<Option<ExprId>> {
        if self.is(end) {
            return Ok(None);
        }
        Ok(Some(self.expression(false)?))
    }

    /// Turns the `for` statement `id`, whose head is followed by `in`, into a for-in statement:
    /// one declarator without an initializer, or an expression that is no literal pattern.
    fn for_in_head(&mut self, id: StmtId, start: Span) -> R<()> {
        let init = match self.stmt_mut(id) {
            Some(Stmt::For { init, .. }) => init.take(),
            _ => None,
        };
        let target = match init {
            Some(ForInit::Var(mut declarators)) if declarators.len() == 1 => {
                let Some(d) = declarators.pop() else {
                    return self.unexpected();
                };
                if d.init.is_some() {
                    if self.strict {
                        return self.fail_at("initializer in a for-in head", start);
                    }
                    return self.refuse_at("initializer in a for-in head", 18, start);
                }
                ForInTarget::Var(d.name)
            }
            Some(ForInit::Expression(e)) => {
                if self.is_literal_pattern(e) {
                    let at = self.expr_span(e);
                    return self.refuse_at("destructuring pattern", 6, at);
                }
                ForInTarget::Expression(e)
            }
            _ => return self.unexpected(),
        };
        if let Some(s) = self.stmt_mut(id) {
            *s = Stmt::ForIn {
                target,
                object: ExprId(u32::MAX),
                body: StmtId(u32::MAX),
            };
        }
        Ok(())
    }

    /// The rest of `for ( target in Expression ) Statement` from `in`.
    fn for_in_rest(&mut self, id: StmtId) -> R<()> {
        self.enter()?;
        self.advance()?;
        let object = self.expression(false)?;
        self.set_loop_part(id, LoopPart::Object(object));
        self.expect(Punct::RParen)?;
        let body = self.statement(Place::Statement)?;
        self.set_loop_part(id, LoopPart::Body(body));
        self.leave();
        Ok(())
    }

    fn return_statement(&mut self) -> R<StmtId> {
        self.enter()?;
        let start = self.start();
        let value = self.return_value()?;
        let s = self.make_stmt(Stmt::Return(value), start)?;
        self.leave();
        Ok(s)
    }

    /// The operand of `return` and the `;` after it: a restricted production, so an operand on the
    /// next line is a statement of its own.
    fn return_value(&mut self) -> R<Option<ExprId>> {
        if self.functions == 0 {
            return self.fail_here("return outside of a function");
        }
        self.advance()?;
        if self.ends_statement_here() {
            self.semicolon()?;
            return Ok(None);
        }
        let e = self.expression(false)?;
        self.semicolon()?;
        Ok(Some(e))
    }

    fn throw_statement(&mut self) -> R<StmtId> {
        self.enter()?;
        let start = self.start();
        self.advance()?;
        if self.tok.newline_before {
            return self.fail_here("line terminator after throw");
        }
        let e = self.expression(false)?;
        self.semicolon()?;
        let s = self.make_stmt(Stmt::Throw(e), start)?;
        self.leave();
        Ok(s)
    }

    fn switch_statement(&mut self) -> R<StmtId> {
        self.enter()?;
        let start = self.start();
        let id = self.reserve_stmt(Stmt::Switch {
            discriminant: ExprId(u32::MAX),
            cases: Vec::new(),
        });
        let d = self.condition()?;
        if let Some(Stmt::Switch { discriminant, .. }) = self.stmt_mut(id) {
            *discriminant = d;
        }
        self.expect(Punct::LBrace)?;
        while !self.eat(Punct::RBrace)? {
            self.case_clause(id)?;
        }
        let id = self.complete_stmt(id, start)?;
        self.leave();
        Ok(id)
    }

    /// One `case` or `default` clause of the switch `id`.
    fn case_clause(&mut self, id: StmtId) -> R<()> {
        self.enter()?;
        self.case_head(id)?;
        while self.case_goes_on()? {
            let s = self.statement(Place::Block)?;
            self.append(id, List::Case, s);
        }
        let end = self.prev_end;
        if let Some(Stmt::Switch { cases, .. }) = self.stmt_mut(id)
            && let Some(c) = cases.last_mut()
        {
            c.span.end = end.max(c.span.start);
        }
        self.leave();
        Ok(())
    }

    /// `case Expression :` or `default :`, appended to the cases of `id`; a second `default` is
    /// an error.
    fn case_head(&mut self, id: StmtId) -> R<()> {
        self.enter()?;
        let span = self.start();
        let test = if self.is_keyword(Keyword::Case) {
            self.advance()?;
            Some(self.expression(false)?)
        } else if self.is_keyword(Keyword::Default) {
            let twice = matches!(
                self.ast.stmt(id).map(|n| &n.stmt),
                Some(Stmt::Switch { cases, .. }) if cases.iter().any(|c| c.test.is_none())
            );
            if twice {
                return self.fail_here("more than one default clause in a switch");
            }
            self.advance()?;
            None
        } else {
            return self.unexpected();
        };
        self.expect(Punct::Colon)?;
        if let Some(Stmt::Switch { cases, .. }) = self.stmt_mut(id) {
            cases.push(SwitchCase {
                test,
                body: Vec::new(),
                span,
            });
        }
        self.leave();
        Ok(())
    }

    /// Whether a statement of the current clause follows.
    fn case_goes_on(&mut self) -> R<bool> {
        if self.is(Punct::RBrace)
            || self.is_keyword(Keyword::Case)
            || self.is_keyword(Keyword::Default)
        {
            return Ok(false);
        }
        if self.at_eof() {
            return self.unexpected();
        }
        Ok(true)
    }

    /// `try Block Catch`, `try Block Finally` and `try Block Catch Finally`. The node is built and
    /// checked by leaf helpers, so the frame that stays on the stack while the blocks are parsed
    /// holds little more than the id.
    fn try_statement(&mut self) -> R<StmtId> {
        self.enter()?;
        let start = self.start();
        let id = self.reserve_try()?;
        self.statement_list(id, List::Try)?;
        if self.is_keyword(Keyword::Catch) {
            self.catch_parameter(id)?;
            self.statement_list(id, List::Catch)?;
        }
        if self.finally_keyword(id)? {
            self.statement_list(id, List::Finally)?;
        }
        let id = self.complete_try(id, start)?;
        self.leave();
        Ok(id)
    }

    /// Reserves the node of the `try` at the current token and consumes the keyword.
    #[inline(never)]
    fn reserve_try(&mut self) -> R<StmtId> {
        let id = self.reserve_stmt(Stmt::Try {
            block: Vec::new(),
            handler: None,
            finalizer: None,
        });
        self.advance()?;
        Ok(id)
    }

    /// Consumes `finally` and opens the finalizer of `id`; false when the current token is not
    /// `finally`.
    #[inline(never)]
    fn finally_keyword(&mut self, id: StmtId) -> R<bool> {
        if !self.is_keyword(Keyword::Finally) {
            return Ok(false);
        }
        self.advance()?;
        if let Some(Stmt::Try { finalizer, .. }) = self.stmt_mut(id) {
            *finalizer = Some(Vec::new());
        }
        Ok(true)
    }

    /// Completes the `try` statement `id`, which needs a `catch` or a `finally`.
    #[inline(never)]
    fn complete_try(&mut self, id: StmtId, start: Span) -> R<StmtId> {
        let complete = matches!(
            self.ast.stmt(id).map(|n| &n.stmt),
            Some(
                Stmt::Try {
                    handler: Some(_),
                    ..
                } | Stmt::Try {
                    finalizer: Some(_),
                    ..
                }
            )
        );
        if !complete {
            return self.unexpected();
        }
        self.complete_stmt(id, start)
    }

    /// `catch ( BindingIdentifier )` or `catch` alone (`#sec-try-statement`), recorded as the
    /// handler of `id`.
    #[inline(never)]
    fn catch_parameter(&mut self, id: StmtId) -> R<()> {
        self.advance()?;
        let param = if self.is(Punct::LBrace) {
            None
        } else {
            self.expect(Punct::LParen)?;
            let param = self.binding_identifier()?;
            self.expect(Punct::RParen)?;
            Some(param)
        };
        if let Some(Stmt::Try { handler, .. }) = self.stmt_mut(id) {
            *handler = Some(CatchClause {
                param,
                body: Vec::new(),
            });
        }
        Ok(())
    }
}

/// Which statement list of a node [`Parser::append`] adds to.
#[derive(Clone, Copy)]
enum List {
    Block,
    Try,
    Catch,
    Finally,
    Case,
}

/// A part of a loop, set once parsed.
#[derive(Clone, Copy)]
enum LoopPart {
    Test(Option<ExprId>),
    Update(Option<ExprId>),
    Object(ExprId),
    Body(StmtId),
}
