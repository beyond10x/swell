//! Class definitions (`#sec-class-definitions`): a ClassDeclaration where a declaration stands
//! (the top of a body, a block, a `case` clause; anywhere else it is a grammar error) and a
//! ClassExpression as a primary expression, parsed to the class tree of `docs/design/engine.md`,
//! section 3, "Classes", item 1 ([`Class`]).
//!
//! All parts of a class are strict mode code (`#sec-strict-mode-code`): the parser reads it as
//! strict code from `class` to its `}`, its name and heritage included. The BindingIdentifier,
//! the heritage (`extends LeftHandSideExpression`) and the computed keys have the [Yield] and
//! [Await] of the code around the class. An element is `;` or a MethodDefinition, `static` or
//! not: a method, a getter, a setter, a generator, an `async` or an async generator method, with
//! a literal, string, number or computed key, read by the helpers of the object literal
//! (`parser/expression.rs`). A plain method that is not `static` and whose key is the literal
//! `constructor` is the class's constructor (ConstructorMethod); when the body has none the
//! parser makes the default one, with no parameter and an empty body. Either is a function of
//! kind [`FunctionKind::ClassConstructor`], numbered at the class's `{` and spanning the class
//! (F's [[SourceText]]). A second such method, and a getter, a setter, a generator or an `async`
//! method named `constructor`, stays an element, which the early errors refuse (`early.rs`).
//!
//! A class is parsed whole; its heritage is held ([`Parser::hold`], [`super::HOLDS`]): `class
//! heritage` at `extends`, so the parse goes on and the early errors of the source come first. A field and a static block, which `story:class-element-syntax-and-early-errors`
//! parses, are refused at their first token; a private name keeps the refusal of the lexer.
//!
//! As in `expression.rs`, the functions on the recursion (each an entry of the depth counter)
//! hold ids and a span; the helpers that look at tokens and build the record are not inlined.

use super::{Parser, R};
use crate::ast::{
    Child, Class, ClassElement, ClassKey, Expr, ExprId, FunctionId, FunctionKind, Name,
    PropertyKey, Span, Stmt, StmtId,
};
use crate::token::{Keyword, Punct, TokenKind};
use alloc::boxed::Box;
use alloc::vec::Vec;

/// What precedes the name of a class element: `static`, and a getter, a setter, or the `async`
/// and `*` of a method.
#[derive(Clone, Copy)]
struct Modifiers {
    is_static: bool,
    /// [`FunctionKind::Method`], [`FunctionKind::Getter`] or [`FunctionKind::Setter`].
    kind: FunctionKind,
    generator: bool,
    asynchronous: bool,
}

impl Modifiers {
    /// Whether the element is a plain method or field so far: no `get`, `set`, `async` or `*`.
    fn plain(self) -> bool {
        self.kind == FunctionKind::Method && !self.generator && !self.asynchronous
    }
}

/// Whether `key` is the literal PropertyName whose StringValue is `name`.
fn named(key: &PropertyKey, name: &str) -> bool {
    match key {
        PropertyKey::Identifier(n) => n == name,
        PropertyKey::String(s) => s.value.iter().copied().eq(name.encode_utf16()),
        PropertyKey::Number(_) | PropertyKey::Computed(_) => false,
    }
}

impl<'a, 'm> Parser<'a, 'm> {
    /// `ClassDeclaration : class BindingIdentifier ClassTail` at the current `class`.
    pub(super) fn class_declaration_statement(&mut self) -> R<StmtId> {
        self.enter()?;
        let start = self.start();
        let (id, outer) = self.class_declaration_head()?;
        self.class_tail(Child::Stmt(id))?;
        self.strict = outer;
        self.class_close(Child::Stmt(id), start);
        let id = self.complete_stmt(id, start)?;
        self.leave();
        Ok(id)
    }

    /// `ClassExpression : class BindingIdentifier? ClassTail` at the current `class`.
    pub(super) fn class_expression_node(&mut self) -> R<ExprId> {
        self.enter()?;
        let start = self.start();
        let (id, outer) = self.class_expression_head()?;
        self.class_tail(Child::Expr(id))?;
        self.strict = outer;
        self.class_close(Child::Expr(id), start);
        let id = self.complete_expr(id, start)?;
        self.leave();
        Ok(id)
    }

    /// `class` and the BindingIdentifier of a declaration: the statement reserved, and the
    /// strictness of the code around the class.
    #[inline(never)]
    fn class_declaration_head(&mut self) -> R<(StmtId, bool)> {
        let outer = self.class_keyword()?;
        let name = self.binding_identifier()?;
        let class = self.new_class(Some(name))?;
        Ok((self.reserve_stmt(Stmt::Class(class)), outer))
    }

    /// `class` and the BindingIdentifier of an expression, if any: the expression reserved, and
    /// the strictness of the code around the class.
    #[inline(never)]
    fn class_expression_head(&mut self) -> R<(ExprId, bool)> {
        let outer = self.class_keyword()?;
        let name = if matches!(self.tok.kind, TokenKind::Name(_)) {
            Some(self.binding_identifier()?)
        } else {
            None
        };
        let class = self.new_class(name)?;
        Ok((self.reserve_expr(Expr::Class(class)), outer))
    }

    /// The current `class`: makes the code strict and consumes it; answers the strictness of the
    /// code around.
    #[inline(never)]
    fn class_keyword(&mut self) -> R<bool> {
        let outer = self.strict;
        self.strict = true;
        self.advance()?;
        Ok(outer)
    }

    /// A class record named `name`, boxed, its bytes charged first.
    #[inline(never)]
    fn new_class(&mut self, name: Option<Name>) -> R<Box<Class>> {
        let class = self.m.boxed(Class {
            name,
            heritage: None,
            constructor: FunctionId(u32::MAX),
            elements: Vec::new(),
            span: Span::default(),
        });
        self.charged(class)
    }

    /// The class record of the class node `target`.
    fn class_mut(&mut self, target: Child) -> Option<&mut Class> {
        match target {
            Child::Stmt(id) => match self.stmt_mut(id) {
                Some(Stmt::Class(class)) => Some(class),
                _ => None,
            },
            Child::Expr(id) => match self.expr_mut(id) {
                Some(Expr::Class(class)) => Some(class),
                _ => None,
            },
            Child::Function(_) => None,
        }
    }

    /// `ClassTail : ClassHeritage? { ClassBody? }` of the class `target`.
    fn class_tail(&mut self, target: Child) -> R<()> {
        self.enter()?;
        if self.is_keyword(Keyword::Extends) {
            self.class_heritage(target)?;
        }
        self.class_body_open(target)?;
        while self.class_goes_on()? {
            self.class_element(target)?;
        }
        self.leave();
        Ok(())
    }

    /// `ClassHeritage : extends LeftHandSideExpression` at the current `extends`, which holds
    /// the refusal `class heritage`.
    fn class_heritage(&mut self, target: Child) -> R<()> {
        self.enter()?;
        self.extends_keyword()?;
        let heritage = self.left_hand_side()?;
        if let Some(class) = self.class_mut(target) {
            class.heritage = Some(heritage);
        }
        self.leave();
        Ok(())
    }

    /// Holds the refusal `class heritage` at the current `extends` and consumes it.
    #[inline(never)]
    fn extends_keyword(&mut self) -> R<()> {
        let span = self.start();
        self.hold("class heritage", 8, span);
        self.advance()
    }

    /// The `{` of the class body of `target`: the default constructor is numbered here, a
    /// function of kind [`FunctionKind::ClassConstructor`] with no parameter and an empty body,
    /// spanning the `{` until the class closes ([`Parser::class_close`]); a written
    /// `constructor` takes its place ([`Parser::element_head`]).
    #[inline(never)]
    fn class_body_open(&mut self, target: Child) -> R<()> {
        let brace = self.start();
        self.expect(Punct::LBrace)?;
        let id = self.begin_function(FunctionKind::ClassConstructor, brace, None);
        if let Some(f) = self.ast.functions.get_mut(id.0 as usize) {
            f.strict = true;
        }
        if let Some(class) = self.class_mut(target) {
            class.constructor = id;
        }
        Ok(())
    }

    /// Consumes the `}` that closes a class body; whether an element follows instead.
    #[inline(never)]
    fn class_goes_on(&mut self) -> R<bool> {
        if self.eat(Punct::RBrace)? {
            return Ok(false);
        }
        if self.at_eof() {
            return self.unexpected();
        }
        Ok(true)
    }

    /// The span of the class `target`, from `start` to its `}`, and of its constructor, whose
    /// source text is the class's.
    #[inline(never)]
    fn class_close(&mut self, target: Child, start: Span) {
        let span = self.finish(start);
        let constructor = match self.class_mut(target) {
            Some(class) => {
                class.span = span;
                class.constructor
            }
            None => return,
        };
        if let Some(f) = self.ast.functions.get_mut(constructor.0 as usize) {
            f.span = span;
        }
    }

    /// One `ClassElement` of the class `target`: `;`, or a MethodDefinition, `static` or not,
    /// appended to its elements or made its constructor.
    fn class_element(&mut self, target: Child) -> R<()> {
        self.enter()?;
        let start = self.start();
        if let Some(modifiers) = self.element_modifiers()? {
            let computed = self.computed_name()?;
            let id = self.element_head(target, modifiers, start, computed)?;
            self.function_rest(id)?;
            self.class_element_end(target, id);
        }
        self.leave();
        Ok(())
    }

    /// What precedes the name of the element at the current token, consumed; `None` for an
    /// empty element, `;`. `static` is a modifier unless `(`, `=`, `;` or `}` follows it (a
    /// method or a field named `static`), and `static {` starts a static block, refused here.
    /// `get`, `set` and `async` are read as in an object literal ([`Parser::accessor_kind`]).
    #[inline(never)]
    fn element_modifiers(&mut self) -> R<Option<Modifiers>> {
        if self.eat(Punct::Semicolon)? {
            return Ok(None);
        }
        let mut is_static = false;
        if self.is_name("static") {
            let (named, block) = match &self.peek()?.kind {
                TokenKind::Punct(p) => (
                    matches!(
                        p,
                        Punct::LParen | Punct::Eq | Punct::Semicolon | Punct::RBrace
                    ),
                    *p == Punct::LBrace,
                ),
                _ => (false, false),
            };
            if block {
                return self.refuse("class static block", 8);
            }
            if !named {
                is_static = true;
                self.advance()?;
            }
        }
        let mut modifiers = Modifiers {
            is_static,
            kind: FunctionKind::Method,
            generator: false,
            asynchronous: false,
        };
        match self.accessor_kind()? {
            Some(FunctionKind::Method) => modifiers.asynchronous = true,
            Some(kind) => modifiers.kind = kind,
            None => {}
        }
        if modifiers.kind == FunctionKind::Method && self.eat(Punct::Star)? {
            modifiers.generator = true;
        }
        Ok(Some(modifiers))
    }

    /// The `ClassElementName` of the element that started at `start` (the ComputedPropertyName
    /// `computed` read before, or the PropertyName at the current token) and the `(` that
    /// follows it: numbers the element's function and appends the element to `target`, or,
    /// for the constructor, answers the function numbered at the `{`. A name not followed by
    /// `(` is a field, refused at the element's first token, when a field may end there (at
    /// `=`, `;`, `}` or a line terminator); otherwise it is a grammar error.
    #[inline(never)]
    fn element_head(
        &mut self,
        target: Child,
        modifiers: Modifiers,
        start: Span,
        computed: Option<ExprId>,
    ) -> R<FunctionId> {
        let key = self.key_or(computed)?;
        if !self.is(Punct::LParen) {
            self.m.release(key);
            let ends_field = self.is(Punct::Eq)
                || self.is(Punct::Semicolon)
                || self.is(Punct::RBrace)
                || self.tok.newline_before;
            if modifiers.plain() && ends_field {
                return self.refuse_at("class field", 8, start);
            }
            return self.unexpected();
        }
        if !modifiers.is_static && modifiers.plain() && named(&key, "constructor") {
            let constructor = self.class_mut(target).map(|class| class.constructor);
            if let Some(id) = constructor
                && self.is_default_constructor(id)
            {
                self.m.release(key);
                if let Some(f) = self.ast.functions.get_mut(id.0 as usize) {
                    f.span = start;
                }
                return Ok(id);
            }
        }
        let id = self.begin_function(modifiers.kind, start, None);
        if let Some(f) = self.ast.functions.get_mut(id.0 as usize) {
            f.generator = modifiers.generator;
            f.asynchronous = modifiers.asynchronous;
        }
        let key = match key {
            PropertyKey::Computed(e) => ClassKey::Computed(e),
            literal => ClassKey::Literal(literal),
        };
        let element = ClassElement::Method {
            is_static: modifiers.is_static,
            key,
            kind: modifiers.kind,
            function: id,
            span: start,
        };
        let m = self.m;
        let pushed = match self.class_mut(target) {
            Some(class) => m.push(&mut class.elements, element),
            None => {
                m.release(element);
                Ok(())
            }
        };
        self.charged(pushed)?;
        Ok(id)
    }

    /// Whether the constructor `id` is still the default one the `{` numbered: its span starts
    /// at the `{`, where no element starts.
    fn is_default_constructor(&self, id: FunctionId) -> bool {
        self.ast
            .function(id)
            .is_some_and(|f| self.src.get(f.span.start as usize) == Some(&u16::from(b'{')))
    }

    /// Sets the end of the element of `target` whose function is `id`, the last one, at the
    /// previous token; the constructor is no element.
    #[inline(never)]
    fn class_element_end(&mut self, target: Child, id: FunctionId) {
        let end = self.prev_end;
        if let Some(ClassElement::Method { function, span, .. }) = self
            .class_mut(target)
            .and_then(|class| class.elements.last_mut())
            && *function == id
        {
            span.end = end.max(span.start);
        }
    }
}
