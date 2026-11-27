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
//! A class is parsed whole. The written constructor of a class with a heritage is read with a
//! SuperCall admitted ([`SuperProperty::DerivedConstructor`]), and an arrow function in it keeps
//! that; every other function of the class is read with a SuperProperty only.
//!
//! The other elements (`#sec-class-definitions`, ClassElement), each held at its first token:
//! a field (FieldDefinition, `class field`, or with a private name `private field`), a private
//! method or accessor (`private method`) and a static block (ClassStaticBlock, `class static
//! block`). A field ends at `;`, before `}`, or where a line terminator lets a semicolon be
//! inserted (`#sec-rules-of-automatic-semicolon-insertion`); `static`, `get`, `set` and `async`
//! name a field where no other element can start with them. The functions of item 1 of
//! "Classes" are made here: one of kind [`FunctionKind::FieldInitializer`] per run of
//! consecutive static fields, numbered at its first field, and one for the instance elements
//! ([`Class::instance_elements`]), numbered at the first field or private method that is not
//! static; each initializer is an expression statement of its function's body. A static block
//! is a function of kind [`FunctionKind::StaticBlock`], numbered at `static`. An initializer
//! and a static block are read with `new.target` and a SuperProperty admitted, and a static
//! block with no `return`. A static block is read with [Yield] and [Await] cleared (`await` is
//! an identifier there, which the early errors refuse, `early.rs`). An initializer keeps the
//! [Yield] and [Await] of the class body, as the grammar's `Initializer[+In, ?Yield, ?Await]`
//! does, but its code (a class nested in it included, the functions in it not) holds no
//! YieldExpression and no AwaitExpression: each is refused at its first token
//! ([`YIELD_IN_FIELD`], [`AWAIT_IN_FIELD`]), a stated difference from the pinned draft, which
//! has no early error against them. The initializer is the body of a function of its own that
//! is neither a generator nor an `async` function, so nothing could run one there.
//!
//! As in `expression.rs`, the functions on the recursion (each an entry of the depth counter)
//! hold ids and a span; the helpers that look at tokens and build the record are not inlined.

use super::{Parser, R, SuperProperty, copy_name};
use crate::ast::{
    Child, Class, ClassElement, ClassKey, Expr, ExprId, FunctionId, FunctionKind, Name,
    PropertyKey, Span, Stmt, StmtId,
};
use crate::token::{Keyword, Punct, TokenKind};
use alloc::boxed::Box;
use alloc::vec::Vec;

/// The message of a YieldExpression in the code of a field initializer, at its `yield`.
pub(super) const YIELD_IN_FIELD: &str = "yield expression in a class field initializer";

/// The message of an AwaitExpression in the code of a field initializer, at its `await`.
pub(super) const AWAIT_IN_FIELD: &str = "await expression in a class field initializer";

/// What a class element starts with, its modifiers consumed.
#[derive(Clone, Copy)]
enum Start {
    /// `;`.
    Empty,
    /// `static {`, at the `{`.
    StaticBlock,
    /// A method or a field, at its name.
    Named(Modifiers),
}

/// What the name of a class element starts.
#[derive(Clone, Copy)]
enum Head {
    /// A method, at its `(`, with its function.
    Method(FunctionId),
    /// A field, after its name, with the function of its initializer.
    Field(FunctionId),
}

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
            default_constructor: true,
            elements: Vec::new(),
            instance_elements: None,
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

    /// `ClassHeritage : extends LeftHandSideExpression` at the current `extends`.
    fn class_heritage(&mut self, target: Child) -> R<()> {
        self.enter()?;
        self.advance()?;
        let heritage = self.left_hand_side()?;
        if let Some(class) = self.class_mut(target) {
            class.heritage = Some(heritage);
        }
        self.leave();
        Ok(())
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

    /// One `ClassElement` of the class `target`: `;`, a MethodDefinition, a FieldDefinition or
    /// a ClassStaticBlock, `static` or not, appended to its elements or made its constructor.
    fn class_element(&mut self, target: Child) -> R<()> {
        self.enter()?;
        let start = self.start();
        match self.element_modifiers()? {
            Start::Empty => {}
            Start::StaticBlock => self.static_block(target, start)?,
            Start::Named(modifiers) => {
                let computed = self.computed_name()?;
                match self.element_head(target, modifiers, start, computed)? {
                    Head::Method(id) => {
                        self.function_rest(id)?;
                        self.class_element_end(target, id);
                    }
                    Head::Field(id) => self.field(target, id)?,
                }
            }
        }
        self.leave();
        Ok(())
    }

    /// What precedes the name of the element at the current token, consumed. `static` is a
    /// modifier unless `(`, `=`, `;` or `}` follows it (a method or a field named `static`), and
    /// `static {` starts a static block. `get`, `set` and `async` are read as in an object
    /// literal ([`Parser::accessor_kind`]), a private name counting as a name after them.
    #[inline(never)]
    fn element_modifiers(&mut self) -> R<Start> {
        if self.eat(Punct::Semicolon)? {
            return Ok(Start::Empty);
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
                self.advance()?;
                return Ok(Start::StaticBlock);
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
        Ok(Start::Named(modifiers))
    }

    /// The `ClassElementName` of the element that started at `start` (the ComputedPropertyName
    /// `computed` read before, or the PropertyName or PrivateIdentifier at the current token)
    /// and what follows it. Before `(` it is a method: numbers the element's function and
    /// appends the element to `target`, or, for the constructor, answers the function numbered
    /// at the `{`; a private method holds `private method`. A plain name (no `get`, `set`,
    /// `async` or `*` before it) followed by `=`, `;`, `}` or a line terminator is a field
    /// ([`Parser::field_head`]). Anything else is a grammar error.
    #[inline(never)]
    fn element_head(
        &mut self,
        target: Child,
        modifiers: Modifiers,
        start: Span,
        computed: Option<ExprId>,
    ) -> R<Head> {
        let key = self.class_key(computed)?;
        if !self.is(Punct::LParen) {
            let ends_field = self.is(Punct::Eq)
                || self.is(Punct::Semicolon)
                || self.is(Punct::RBrace)
                || self.tok.newline_before;
            if modifiers.plain() && ends_field {
                return self
                    .field_head(target, modifiers.is_static, key, start)
                    .map(Head::Field);
            }
            self.m.release(key);
            return self.unexpected();
        }
        if let ClassKey::Literal(literal) = &key
            && !modifiers.is_static
            && modifiers.plain()
            && named(literal, "constructor")
        {
            let constructor = self
                .class_mut(target)
                .map(|class| (class.constructor, class.heritage.is_some()));
            if let Some((id, derived)) = constructor
                && self.is_default_constructor(id)
            {
                self.m.release(key);
                if let Some(f) = self.ast.functions.get_mut(id.0 as usize) {
                    f.span = start;
                }
                self.written_constructor(target, id, derived);
                return Ok(Head::Method(id));
            }
        }
        if matches!(key, ClassKey::Private(_)) {
            self.hold("private method", 8, start);
            if !modifiers.is_static {
                self.instance_elements(target, start);
            }
        }
        let id = self.begin_function(modifiers.kind, start, None);
        if let Some(f) = self.ast.functions.get_mut(id.0 as usize) {
            f.generator = modifiers.generator;
            f.asynchronous = modifiers.asynchronous;
        }
        let element = ClassElement::Method {
            is_static: modifiers.is_static,
            key,
            kind: modifiers.kind,
            function: id,
            span: start,
        };
        self.push_element(target, element)?;
        Ok(Head::Method(id))
    }

    /// The name of a class element: the ComputedPropertyName `computed` read before, or the
    /// PrivateIdentifier or the PropertyName at the current token, consumed.
    #[inline(never)]
    fn class_key(&mut self, computed: Option<ExprId>) -> R<ClassKey> {
        if let Some(e) = computed {
            return Ok(ClassKey::Computed(e));
        }
        let span = self.start();
        let private = match &self.tok.kind {
            TokenKind::PrivateName(n) => Some(copy_name(self.m, n)),
            _ => None,
        };
        if let Some(name) = private {
            let name = self.charged(name)?;
            self.advance()?;
            return Ok(ClassKey::Private(Name { name, span }));
        }
        Ok(match self.key_or(None)? {
            PropertyKey::Computed(e) => ClassKey::Computed(e),
            literal => ClassKey::Literal(literal),
        })
    }

    /// Appends `element` to the elements of `target`, the growth charged first.
    #[inline(never)]
    fn push_element(&mut self, target: Child, element: ClassElement) -> R<()> {
        let m = self.m;
        let pushed = match self.class_mut(target) {
            Some(class) => m.push(&mut class.elements, element),
            None => {
                m.release(element);
                Ok(())
            }
        };
        self.charged(pushed)
    }

    /// A function the class body makes for its elements, of kind `kind`, numbered at `start`:
    /// strict, as all class code is.
    fn element_function(&mut self, kind: FunctionKind, start: Span) -> FunctionId {
        let id = self.begin_function(kind, start, None);
        if let Some(f) = self.ast.functions.get_mut(id.0 as usize) {
            f.strict = true;
        }
        id
    }

    /// The function of the instance elements of `target` ([`Class::instance_elements`]),
    /// numbered at `start` when this is the first such element.
    fn instance_elements(&mut self, target: Child, start: Span) -> FunctionId {
        if let Some(id) = self
            .class_mut(target)
            .and_then(|class| class.instance_elements)
        {
            return id;
        }
        let id = self.element_function(FunctionKind::FieldInitializer, start);
        if let Some(class) = self.class_mut(target) {
            class.instance_elements = Some(id);
        }
        id
    }

    /// The FieldDefinition named `key` of `target`, which started at `start`, appended to its
    /// elements; holds `class field`, or `private field` for a private name, at `start`.
    /// Answers the function its initializer belongs to: for a static field the function of the
    /// run of static fields the element before it began, or a new one; for any other field the
    /// function of the instance elements.
    #[inline(never)]
    fn field_head(
        &mut self,
        target: Child,
        is_static: bool,
        key: ClassKey,
        start: Span,
    ) -> R<FunctionId> {
        if matches!(key, ClassKey::Private(_)) {
            self.hold("private field", 8, start);
        } else {
            self.hold("class field", 8, start);
        }
        let function = if is_static {
            let run = self
                .class_mut(target)
                .and_then(|class| match class.elements.last() {
                    Some(ClassElement::Field {
                        is_static: true,
                        function,
                        ..
                    }) => Some(*function),
                    _ => None,
                });
            match run {
                Some(id) => id,
                None => self.element_function(FunctionKind::FieldInitializer, start),
            }
        } else {
            self.instance_elements(target, start)
        };
        let element = ClassElement::Field {
            is_static,
            key,
            initializer: None,
            function,
            span: start,
        };
        self.push_element(target, element)?;
        Ok(function)
    }

    /// The rest of the field just appended to `target`, after its name: its Initializer, if
    /// any, read as code of `function`, and its end: `;`, consumed, or a `}` or a line
    /// terminator before the current token, where a semicolon is inserted.
    fn field(&mut self, target: Child, function: FunctionId) -> R<()> {
        if self.is(Punct::Eq) {
            self.enter()?;
            self.advance()?;
            self.element_context_open(true)?;
            let e = self.assignment(false)?;
            self.element_context_close();
            self.field_initializer(target, function, e)?;
            self.leave();
        }
        self.field_end(target, function)
    }

    /// Saves the [Yield] and [Await], `new.target`, the SuperProperty flag and whether the code
    /// is a field initializer's, of the class body, and sets those of a field initializer
    /// (`field`) or a static block: `new.target` and a SuperProperty admitted
    /// (`#sec-class-definitions-static-semantics-early-errors`: both are code of a function with
    /// a [[HomeObject]]). A field initializer keeps the [Yield] and [Await] of the class body
    /// (`Initializer[+In, ?Yield, ?Await]`) and refuses the YieldExpression and the
    /// AwaitExpression they admit ([`YIELD_IN_FIELD`], [`AWAIT_IN_FIELD`]); a static block
    /// clears both. As `function.rs` saves them for a function, on the same lists, the growth
    /// charged first.
    #[inline(never)]
    fn element_context_open(&mut self, field: bool) -> R<()> {
        let m = self.m;
        let pushed = m.push(&mut self.outer_yields, self.yields);
        self.charged(pushed)?;
        let pushed = m.push(&mut self.outer_awaits, self.awaits);
        self.charged(pushed)?;
        let pushed = m.push(&mut self.outer_new_target, self.new_target);
        self.charged(pushed)?;
        let pushed = m.push(&mut self.outer_super_properties, self.super_property);
        self.charged(pushed)?;
        let pushed = m.push(&mut self.outer_field_code, self.field_code);
        self.charged(pushed)?;
        if !field {
            self.yields = false;
            self.awaits = false;
        }
        self.field_code = field;
        self.new_target = true;
        self.super_property = SuperProperty::Method;
        Ok(())
    }

    /// Puts back what [`Parser::element_context_open`] saved.
    #[inline(never)]
    fn element_context_close(&mut self) {
        if let Some(yields) = self.outer_yields.pop() {
            self.yields = yields;
        }
        if let Some(awaits) = self.outer_awaits.pop() {
            self.awaits = awaits;
        }
        if let Some(new_target) = self.outer_new_target.pop() {
            self.new_target = new_target;
        }
        if let Some(super_property) = self.outer_super_properties.pop() {
            self.super_property = super_property;
        }
        if let Some(field_code) = self.outer_field_code.pop() {
            self.field_code = field_code;
        }
    }

    /// The initializer `e` of the field just appended to `target`: the field's, and an
    /// expression statement appended to the body of `function`, whose depth grows to hold it.
    #[inline(never)]
    fn field_initializer(&mut self, target: Child, function: FunctionId, e: ExprId) -> R<()> {
        let span = self.expr_span(e);
        let stmt = self.push_stmt(Stmt::Expression(e), span)?;
        let depth = self.ast.depth(Child::Stmt(stmt)).saturating_add(1);
        self.check_depth(depth, span)?;
        let m = self.m;
        let pushed = match self.ast.functions.get_mut(function.0 as usize) {
            Some(f) => {
                f.depth = f.depth.max(depth);
                m.push(&mut f.body, stmt)
            }
            None => Ok(()),
        };
        self.charged(pushed)?;
        if let Some(ClassElement::Field { initializer, .. }) = self
            .class_mut(target)
            .and_then(|class| class.elements.last_mut())
        {
            *initializer = Some(e);
        }
        Ok(())
    }

    /// The end of the field just appended to `target`, whose initializer belongs to
    /// `function`: the field and the function end at the previous token, and a `;` is
    /// consumed; without one, a `}` or a line terminator must come next.
    #[inline(never)]
    fn field_end(&mut self, target: Child, function: FunctionId) -> R<()> {
        let end = self.prev_end;
        if let Some(ClassElement::Field { span, .. }) = self
            .class_mut(target)
            .and_then(|class| class.elements.last_mut())
        {
            span.end = end.max(span.start);
        }
        if let Some(f) = self.ast.functions.get_mut(function.0 as usize) {
            f.span.end = end.max(f.span.start);
        }
        if self.eat(Punct::Semicolon)? || self.is(Punct::RBrace) || self.tok.newline_before {
            return Ok(());
        }
        self.unexpected()
    }

    /// `ClassStaticBlock : static { ClassStaticBlockBody }` of `target`, which started at
    /// `start`, at its `{`.
    fn static_block(&mut self, target: Child, start: Span) -> R<()> {
        self.enter()?;
        let (id, functions) = self.static_block_open(target, start)?;
        let (body, _) = self.body(false)?;
        self.static_block_close(target, id, functions, body)?;
        self.leave();
        Ok(())
    }

    /// The `{` of the static block that started at `start`: holds `class static block` there,
    /// numbers its function, appends the element to `target` and opens its context, in which
    /// no `return` stands. Answers the function and the count of enclosing functions to put back.
    #[inline(never)]
    fn static_block_open(&mut self, target: Child, start: Span) -> R<(FunctionId, u32)> {
        self.expect(Punct::LBrace)?;
        self.hold("class static block", 8, start);
        let id = self.element_function(FunctionKind::StaticBlock, start);
        self.push_element(
            target,
            ClassElement::StaticBlock {
                function: id,
                span: start,
            },
        )?;
        self.element_context_open(false)?;
        Ok((id, core::mem::take(&mut self.functions)))
    }

    /// The `}` of the static block `id` of `target`: its body, span and depth recorded as a
    /// function's, the context and the count of enclosing `functions` put back.
    #[inline(never)]
    fn static_block_close(
        &mut self,
        target: Child,
        id: FunctionId,
        functions: u32,
        body: Vec<StmtId>,
    ) -> R<()> {
        self.expect(Punct::RBrace)?;
        self.functions = functions;
        self.element_context_close();
        let mut deepest = 0;
        for s in &body {
            deepest = deepest.max(self.ast.depth(Child::Stmt(*s)));
        }
        let depth = deepest.saturating_add(1);
        let start = self.ast.function(id).map_or(Span::default(), |f| f.span);
        let span = self.finish(start);
        self.check_depth(depth, span)?;
        if let Some(f) = self.ast.functions.get_mut(id.0 as usize) {
            f.body = body;
            f.span = span;
            f.depth = depth;
        }
        if let Some(ClassElement::StaticBlock { span: element, .. }) = self
            .class_mut(target)
            .and_then(|class| class.elements.last_mut())
        {
            element.end = span.end;
        }
        Ok(())
    }

    /// The constructor `id` of `target` is written, no longer the default one
    /// ([`Class::default_constructor`]); of a class with a heritage (`derived`) its parameters
    /// and body admit a SuperCall ([`Parser::open_context`] reads `derived_constructor`).
    #[inline(never)]
    fn written_constructor(&mut self, target: Child, id: FunctionId, derived: bool) {
        if let Some(class) = self.class_mut(target) {
            class.default_constructor = false;
        }
        if derived {
            self.derived_constructor = Some(id);
        }
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

#[cfg(test)]
mod tests {
    use std::vec::Vec;

    use crate::ast::{ClassElement, Expr, ExprId, FunctionId, FunctionKind, Stmt, StmtId};
    use crate::ledger::{Counting, HeapBytes, Meter};

    /// The tree of the class elements (`docs/design/engine.md`, section 3, "Classes", item 1):
    /// the functions in the order of their first tokens, one `FieldInitializer` per run of
    /// static fields and one for the instance elements, each initializer an expression
    /// statement of its function's body, a `StaticBlock` with its statements; `#x in` with its
    /// right operand; the class held; and what the ledger holds after the parse is the tree's
    /// capacity, so every name, list and body the element parse made is charged and nothing it
    /// dropped stays charged.
    #[test]
    fn the_elements_of_a_class_are_functions_of_the_tree() {
        let text = "class C {\n\
                    static a = 1; static b = () => 2;\n\
                    m() {}\n\
                    static c;\n\
                    x = 3;\n\
                    #p() { return #q in this && this.#q; }\n\
                    #q = this?.#p;\n\
                    static { var v = 1; }\n\
                    }";
        let source: Vec<u16> = text.encode_utf16().collect();
        let mut ledger = Counting::default();
        let meter = Meter::new(&mut ledger);
        let parsed = match super::super::parse(&source, 1_000, &meter) {
            Ok(parsed) => parsed,
            Err(error) => panic!("{error:?}"),
        };
        let kinds: Vec<FunctionKind> = parsed.ast.functions().iter().map(|f| f.kind).collect();
        assert_eq!(
            kinds,
            [
                FunctionKind::ClassConstructor,
                FunctionKind::FieldInitializer,
                FunctionKind::Arrow,
                FunctionKind::Method,
                FunctionKind::FieldInitializer,
                FunctionKind::FieldInitializer,
                FunctionKind::Method,
                FunctionKind::StaticBlock,
            ]
        );
        let bodies: Vec<usize> = parsed
            .ast
            .functions()
            .iter()
            .map(|f| f.body.len())
            .collect();
        assert_eq!(bodies, [0, 2, 1, 0, 0, 2, 1, 1]);
        assert!(parsed.ast.functions().iter().all(|f| f.strict));
        let Some(Stmt::Class(class)) = parsed
            .body
            .first()
            .and_then(|s| parsed.ast.stmt(*s))
            .map(|n| &n.stmt)
        else {
            panic!("no class declaration");
        };
        assert_eq!(class.instance_elements, Some(FunctionId(5)));
        let runs: Vec<(bool, u32, bool)> = class
            .elements
            .iter()
            .filter_map(|e| match e {
                ClassElement::Field {
                    is_static,
                    function,
                    initializer,
                    ..
                } => Some((*is_static, function.0, initializer.is_some())),
                _ => None,
            })
            .collect();
        assert_eq!(
            runs,
            [
                (true, 1, true),
                (true, 1, true),
                (true, 4, false),
                (false, 5, true),
                (false, 5, true)
            ]
        );
        let statement = |f: usize, i: usize| {
            parsed
                .ast
                .functions()
                .get(f)
                .and_then(|f| f.body.get(i))
                .and_then(|s: &StmtId| parsed.ast.stmt(*s))
                .map(|n| matches!(n.stmt, Stmt::Expression(_)))
        };
        assert_eq!(statement(5, 0), Some(true));
        assert_eq!(statement(5, 1), Some(true));
        let private_in = (0..)
            .map_while(|i| parsed.ast.expr(ExprId(i)))
            .find_map(|n| match &n.expr {
                Expr::PrivateIn { name, right } => Some((name.name.as_str(), *right)),
                _ => None,
            });
        let Some(("#q", right)) = private_in else {
            panic!("no #q in: {private_in:?}");
        };
        assert!(matches!(
            parsed.ast.expr(right).map(|n| &n.expr),
            Some(Expr::This)
        ));
        let members = (0..)
            .map_while(|i| parsed.ast.expr(ExprId(i)))
            .filter_map(|n| match &n.expr {
                Expr::PrivateMember { name, optional, .. } => Some((name.name.as_str(), *optional)),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(members, [("#q", false), ("#p", true)]);
        assert!(parsed.held.is_some(), "the class is held");
        let tree = parsed
            .ast
            .heap_bytes()
            .saturating_add(parsed.body.heap_bytes());
        assert_eq!(meter.held(), tree, "held against the tree");
    }

    /// The function of the instance elements comes with the first private method that is not
    /// static and with no static one (`docs/design/engine.md`, section 3, "Classes", item 1):
    /// a class with a static private method alone has none, and one with both has it once.
    #[test]
    fn a_static_private_method_makes_no_instance_elements() {
        use FunctionKind::{ClassConstructor, FieldInitializer, Method};
        let cases: [(&str, Option<FunctionId>, &[FunctionKind]); 3] = [
            (
                "class C { static #s() {} }",
                None,
                &[ClassConstructor, Method],
            ),
            (
                "class C { #m() {} }",
                Some(FunctionId(1)),
                &[ClassConstructor, FieldInitializer, Method],
            ),
            (
                "class C { static #s() {} #m() {} }",
                Some(FunctionId(2)),
                &[ClassConstructor, Method, FieldInitializer, Method],
            ),
        ];
        let mut wrong = Vec::new();
        for (text, instance, expected) in cases {
            let source: Vec<u16> = text.encode_utf16().collect();
            let mut ledger = Counting::default();
            let meter = Meter::new(&mut ledger);
            let parsed = match super::super::parse(&source, 1_000, &meter) {
                Ok(parsed) => parsed,
                Err(error) => panic!("{text}: {error:?}"),
            };
            let kinds: Vec<FunctionKind> = parsed.ast.functions().iter().map(|f| f.kind).collect();
            let found = match parsed
                .body
                .first()
                .and_then(|s| parsed.ast.stmt(*s))
                .map(|n| &n.stmt)
            {
                Some(Stmt::Class(class)) => class.instance_elements,
                _ => panic!("{text}: no class declaration"),
            };
            if found != instance || kinds != expected {
                wrong.push(std::format!(
                    "{text}: instance elements {found:?}, functions {kinds:?}"
                ));
            }
        }
        assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    }
}
