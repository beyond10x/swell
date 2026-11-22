//! The syntax tree of a script: the productions of clauses 13 (`#sec-ecmascript-language-expressions`),
//! 14 (`#sec-ecmascript-language-statements-and-declarations`), 15.2 (`#sec-function-definitions`)
//! and 15.4 (`#sec-method-definitions`: accessors and plain methods) that slices 1, 2 and 6 parse.
//!
//! The tree lives in three arenas, one per kind of node, and a node names its children by index.
//! So dropping a tree is no recursion at all, and a walk over it recurses once per level, a depth
//! the parser bounds by `parser_nesting` (every node records its `depth`, the length of the
//! longest path from it to a leaf, and the parser refuses a node deeper than the limit).
//! Every node records its source position.

use crate::ledger::HeapBytes;

/// The source position of a node or token: code unit offsets `start..end`, and the `line` and
/// `column` of `start`, both counted from 1 (a column counts UTF-16 code units).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Span {
    pub start: u32,
    pub end: u32,
    pub line: u32,
    pub column: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ExprId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StmtId(pub u32);

/// A function of the script, numbered in the order its first token appears; an arrow function
/// at its `=>`, after the functions in the initializers of its parameters.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FunctionId(pub u32);

/// The production of `NumericLiteral` a literal matched (`#sec-literals-numeric-literals`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumberForm {
    /// `DecimalLiteral` whose integer part, if any, is not a `NonOctalDecimalIntegerLiteral`.
    Decimal,
    /// A `DecimalLiteral` whose integer part is a `NonOctalDecimalIntegerLiteral`: `08`, `09.5`.
    NonOctalDecimal,
    Binary,
    Octal,
    Hex,
    /// `LegacyOctalIntegerLiteral`: `017`.
    LegacyOctal,
}

/// A numeric literal. Its text is kept; its value is the compiler's to compute
/// (`#sec-numericvalue`), with `swell-num`.
#[derive(Clone, Debug, PartialEq)]
pub struct NumberLiteral {
    pub text: String,
    pub form: NumberForm,
}

/// A string literal: its `SV` (`#sec-string-literals-static-semantics-sv`) as UTF-16 code units,
/// lone surrogates included, and whether it holds an escape strict mode code forbids
/// (`#sec-string-literals-early-errors`).
#[derive(Clone, Debug, PartialEq)]
pub struct StringLiteral {
    pub value: Vec<u16>,
    /// A `LegacyOctalEscapeSequence`: `\1`, `\07`, `\08`.
    pub legacy_octal_escape: bool,
    /// A `NonOctalDecimalEscapeSequence`: `\8`, `\9`.
    pub non_octal_decimal_escape: bool,
}

/// A regular expression literal (`#sec-literals-regular-expression-literals`): its `BodyText`
/// and `FlagText`. Before slice 5 its pattern is not parsed; evaluating it throws a
/// `SyntaxError` (a recorded omission of `sec-literals-regular-expression-literals`).
#[derive(Clone, Debug, PartialEq)]
pub struct RegExpLiteral {
    pub body: Vec<u16>,
    pub flags: Vec<u16>,
}

/// A `BindingIdentifier` with its position.
#[derive(Clone, Debug, PartialEq)]
pub struct Name {
    pub name: String,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnaryOp {
    Delete,
    Void,
    Typeof,
    Plus,
    Minus,
    BitNot,
    Not,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UpdateOp {
    Increment,
    Decrement,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryOp {
    Mul,
    Div,
    Rem,
    Add,
    Sub,
    Shl,
    Shr,
    UShr,
    Lt,
    Gt,
    LtEq,
    GtEq,
    Instanceof,
    In,
    Eq,
    NotEq,
    StrictEq,
    StrictNotEq,
    BitAnd,
    BitXor,
    BitOr,
}

/// `&&`, `||` (`#sec-binary-logical-operators`) and `??`, a `CoalesceExpression`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogicalOp {
    And,
    Or,
    Coalesce,
}

/// `=`, a compound assignment or a logical assignment (`&&=`, `||=`, `??=`) of
/// `#sec-assignment-operators`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssignOp {
    Assign,
    Compound(BinaryOp),
    Logical(LogicalOp),
}

#[derive(Clone, Debug, PartialEq)]
pub enum PropertyKey {
    /// An `IdentifierName`, reserved words included.
    Identifier(String),
    String(StringLiteral),
    Number(NumberLiteral),
    /// A ComputedPropertyName `[ AssignmentExpression ]` (`#sec-object-initializer`): the
    /// expression, evaluated and converted by ToPropertyKey (`#sec-topropertykey`) where the
    /// property is defined or read.
    Computed(ExprId),
}

#[derive(Clone, Debug, PartialEq)]
pub enum PropertyValue {
    /// `PropertyName : AssignmentExpression`.
    Init(ExprId),
    Get(FunctionId),
    Set(FunctionId),
    /// A plain `MethodDefinition`, `name(params) { body }` (`#sec-method-definitions`).
    Method(FunctionId),
    /// `PropertyDefinition : IdentifierReference`, the identifier expression of its value.
    Shorthand(ExprId),
    /// `PropertyDefinition : CoverInitializedName`, `name = init`: the identifier expression
    /// and the initializer. It stands only in an object literal that is read again as an
    /// ObjectAssignmentPattern or an ObjectBindingPattern; one left in an object literal is the
    /// early error of `#sec-object-initializer-static-semantics-early-errors`.
    CoverInitialized(ExprId, ExprId),
    /// `PropertyDefinition : ... AssignmentExpression`, copied by CopyDataProperties
    /// (`#sec-copydataproperties`). Its key is an empty `Identifier` that nothing reads. Its
    /// span ends after the `,` that follows it, when one does: read again as the rest property
    /// of a pattern, which no comma may follow (`#sec-destructuring-assignment`,
    /// `#sec-destructuring-binding-patterns`), that tells it from the last property.
    Spread(ExprId),
}

/// A `PropertyDefinition` of an object literal (`#sec-object-initializer`).
#[derive(Clone, Debug, PartialEq)]
pub struct Property {
    pub key: PropertyKey,
    pub value: PropertyValue,
    pub span: Span,
}

/// How the targets of an object pattern are bound.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PatternKind {
    /// An ObjectAssignmentPattern (`#sec-destructuring-assignment`): each target is a reference,
    /// evaluated before its value is read, then PutValue.
    Assignment,
    /// The ObjectBindingPattern of a `var` declaration or a for-in head with `var`:
    /// BindingInitialization with no environment, ResolveBinding before the value is read, then
    /// PutValue (`#sec-runtime-semantics-keyedbindinginitialization`).
    Var,
    /// The ObjectBindingPattern of a `let` or `const` declaration, a parameter or a `catch`
    /// parameter: InitializeReferencedBinding of each name.
    Initialize,
}

/// The target of one property of an object pattern.
#[derive(Clone, Debug, PartialEq)]
pub enum PatternTarget {
    /// A `BindingIdentifier` of a binding pattern.
    Binding(Name),
    /// An assignment target (an identifier, a property access, in parentheses or not) of an
    /// assignment pattern, or a nested object pattern of the same kind.
    Expr(ExprId),
}

/// A `BindingProperty` (`#sec-destructuring-binding-patterns`) or an `AssignmentProperty`
/// (`#sec-destructuring-assignment`): the key read, its target and its default.
#[derive(Clone, Debug, PartialEq)]
pub struct PatternProperty {
    pub key: PropertyKey,
    pub target: PatternTarget,
    /// The `Initializer`, evaluated when the property's value is `undefined`.
    pub default: Option<ExprId>,
    pub span: Span,
}

/// A `BindingElement` or an `AssignmentElement` of an array pattern
/// (`#sec-destructuring-binding-patterns`, `#sec-destructuring-assignment`), or its rest element
/// (`BindingRestElement`, `AssignmentRestElement`), which has no default: its target and its
/// default.
#[derive(Clone, Debug, PartialEq)]
pub struct PatternElement {
    pub target: PatternTarget,
    /// The `Initializer`, evaluated when the element's value is `undefined`.
    pub default: Option<ExprId>,
    pub span: Span,
}

/// A name a binding pattern binds: the name, and the pattern node and property index (of an
/// object pattern) or element index (of an array pattern, elisions counted) of its
/// `BindingIdentifier`.
#[derive(Clone, Debug, PartialEq)]
pub struct BoundName {
    pub name: Name,
    pub pattern: ExprId,
    pub property: u32,
}

/// An ObjectBindingPattern where a binding stands: the pattern node, its BoundNames in source
/// order (`#sec-static-semantics-boundnames`) and whether it holds a default
/// (ContainsExpression, `#sec-static-semantics-containsexpression`).
#[derive(Clone, Debug, PartialEq)]
pub struct Pattern {
    pub pattern: ExprId,
    pub names: Vec<BoundName>,
    pub expressions: bool,
    pub span: Span,
}

/// What a declarator, a for-in head, a `catch` clause or a parameter binds: a
/// `BindingIdentifier` or a `BindingPattern`.
#[derive(Clone, Debug, PartialEq)]
pub enum BindingTarget {
    Identifier(Name),
    Pattern(Pattern),
}

impl BindingTarget {
    /// The BoundNames, in source order.
    pub fn names(&self) -> impl Iterator<Item = &Name> {
        let (one, many) = match self {
            BindingTarget::Identifier(name) => (Some(name), &[][..]),
            BindingTarget::Pattern(p) => (None, p.names.as_slice()),
        };
        one.into_iter().chain(many.iter().map(|b| &b.name))
    }

    /// The `BindingIdentifier`, when it is one.
    pub fn identifier(&self) -> Option<&Name> {
        match self {
            BindingTarget::Identifier(name) => Some(name),
            BindingTarget::Pattern(_) => None,
        }
    }

    /// The pattern node, when it is a pattern.
    pub fn pattern(&self) -> Option<ExprId> {
        match self {
            BindingTarget::Identifier(_) => None,
            BindingTarget::Pattern(p) => Some(p.pattern),
        }
    }

    pub fn span(&self) -> Span {
        match self {
            BindingTarget::Identifier(name) => name.span,
            BindingTarget::Pattern(p) => p.span,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    This,
    /// An `IdentifierReference`.
    Identifier(String),
    Null,
    Boolean(bool),
    Number(NumberLiteral),
    String(StringLiteral),
    RegExp(RegExpLiteral),
    /// An array literal; `None` is an elision.
    Array(Vec<Option<ExprId>>),
    Object(Vec<Property>),
    Function(FunctionId),
    Unary {
        op: UnaryOp,
        operand: ExprId,
    },
    Update {
        op: UpdateOp,
        prefix: bool,
        operand: ExprId,
    },
    Binary {
        op: BinaryOp,
        left: ExprId,
        right: ExprId,
    },
    Logical {
        op: LogicalOp,
        left: ExprId,
        right: ExprId,
    },
    Conditional {
        test: ExprId,
        consequent: ExprId,
        alternate: ExprId,
    },
    Assign {
        op: AssignOp,
        target: ExprId,
        value: ExprId,
    },
    /// The comma operator, two or more expressions.
    Sequence(Vec<ExprId>),
    /// `object.property`.
    Member {
        object: ExprId,
        property: String,
    },
    /// `object[index]`.
    Index {
        object: ExprId,
        index: ExprId,
    },
    Call {
        callee: ExprId,
        arguments: Vec<ExprId>,
    },
    /// `new callee(arguments)`; `None` when the `Arguments` are absent (`new X`).
    New {
        callee: ExprId,
        arguments: Option<Vec<ExprId>>,
    },
    /// An `OptionalExpression` (`#sec-optional-chains`): the expression whose links from its
    /// first `?.` on are skipped, giving `undefined`, when the value before a `?.` is
    /// `undefined` or `null`. Its links are the `Optional...` nodes and ordinary member
    /// accesses and calls.
    OptionalChain(ExprId),
    /// `object?.property`.
    OptionalMember {
        object: ExprId,
        property: String,
    },
    /// `object?.[index]`.
    OptionalIndex {
        object: ExprId,
        index: ExprId,
    },
    /// `callee?.(arguments)`.
    OptionalCall {
        callee: ExprId,
        arguments: Vec<ExprId>,
    },
    /// A `TemplateLiteral` without a tag (`#sec-template-literals`): its template strings in
    /// source order, one more than its substitutions, each `Expression` of a `${ }`.
    Template {
        strings: Vec<TemplateString>,
        substitutions: Vec<ExprId>,
    },
    /// An ObjectBindingPattern or an ObjectAssignmentPattern (`#sec-destructuring-binding-patterns`,
    /// `#sec-destructuring-assignment`), its properties in source order. It destructures the
    /// value its context gives it; an assignment whose target is one has that value as its own.
    /// With `rest`, the last of `properties` is the BindingRestProperty or AssignmentRestProperty
    /// `... target`, whose key is an empty `Identifier` that nothing reads and which has no
    /// default.
    ObjectPattern {
        properties: Vec<PatternProperty>,
        kind: PatternKind,
        rest: bool,
    },
    /// An ArrayBindingPattern or an ArrayAssignmentPattern, its elements in source order, `None`
    /// an elision. It destructures the value its context gives it through the value's iterator,
    /// as an object pattern does by keys. With `rest`, the last of `elements` is the
    /// BindingRestElement or AssignmentRestElement `... target`.
    ArrayPattern {
        elements: Vec<Option<PatternElement>>,
        kind: PatternKind,
        rest: bool,
    },
    /// `... AssignmentExpression`: a SpreadElement of an array literal
    /// (`#sec-array-initializer`) or an argument of a call or `new` (`#sec-argument-lists`),
    /// where alone it stands. Its span ends after the `,` that follows it, when one does: read
    /// again as the rest element of an array pattern, which no comma may follow, that tells it
    /// from the last element. The parenthesized rest element of an arrow's parameters that is
    /// a pattern, `(...[a]) =>`, is one too, its span starting at the `...`.
    Spread(ExprId),
}

/// One template string of a `TemplateLiteral` (`#sec-templatestring`), as UTF-16 code units:
/// its TV (`#sec-static-semantics-tv`), the value a template without a tag concatenates, and its
/// TRV (`#sec-static-semantics-trv`), the source text between its delimiters with each CR LF
/// and CR made a LF. A string with a NotEscapeSequence, whose TV is undefined, is an early error
/// of a template without a tag (`#sec-static-semantics-template-early-errors`) and never in a
/// tree.
#[derive(Clone, Debug, PartialEq)]
pub struct TemplateString {
    pub cooked: Vec<u16>,
    pub raw: Vec<u16>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExprNode {
    pub expr: Expr,
    pub span: Span,
    /// The longest path from this node to a leaf, counted in nodes, through functions.
    pub depth: u32,
    /// Whether the expression stood in a `ParenthesizedExpression`.
    pub parenthesized: bool,
}

/// A `VariableDeclaration` or a `LexicalBinding`.
#[derive(Clone, Debug, PartialEq)]
pub struct VarDeclarator {
    pub target: BindingTarget,
    pub init: Option<ExprId>,
}

/// The keyword of a declaration: a `VariableStatement` (`#sec-variable-statement`) or a
/// `LexicalDeclaration` (`#sec-let-and-const-declarations`), and the same in the head of a `for`
/// or a `for`-`in`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeclKind {
    Var,
    Let,
    Const,
}

impl DeclKind {
    /// Whether the declaration is a `let` or a `const` one.
    pub fn is_lexical(self) -> bool {
        self != DeclKind::Var
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ForInit {
    Var(DeclKind, Vec<VarDeclarator>),
    Expression(ExprId),
}

/// The left side of `for (... in ...)`.
#[derive(Clone, Debug, PartialEq)]
pub enum ForInTarget {
    Var(DeclKind, BindingTarget),
    /// A `LeftHandSideExpression`: an assignment target or an ObjectAssignmentPattern.
    Expression(ExprId),
}

/// The iterationKind of ForIn/OfHeadEvaluation (`#sec-runtime-semantics-forinofheadevaluation`)
/// of a [`Stmt::ForIn`]: the keys of a for-in, the values of a for-of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IterationKind {
    /// `for (... in ...)`: EnumerateObjectProperties.
    Enumerate,
    /// `for (... of ...)`: GetIterator.
    Iterate,
}

/// A name a `let` or `const` declaration binds (the BoundNames of a `LexicalDeclaration`), or a
/// function declaration in a block or a `case` block ([`Ast::lexical_names`]), with the
/// statement that declares it.
#[derive(Clone, Debug, PartialEq)]
pub struct LexicalName {
    pub name: Name,
    /// Whether the declaration is `const` (IsConstantDeclaration).
    pub constant: bool,
    pub declaration: StmtId,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SwitchCase {
    /// `None` for `default`.
    pub test: Option<ExprId>,
    pub body: Vec<StmtId>,
    pub span: Span,
}

/// A `Catch`; `param` is `None` for `catch Block`, which binds nothing.
#[derive(Clone, Debug, PartialEq)]
pub struct CatchClause {
    pub param: Option<BindingTarget>,
    pub body: Vec<StmtId>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Stmt {
    Block(Vec<StmtId>),
    /// A `VariableStatement` or a `LexicalDeclaration`.
    Var(DeclKind, Vec<VarDeclarator>),
    Empty,
    Expression(ExprId),
    If {
        test: ExprId,
        consequent: StmtId,
        alternate: Option<StmtId>,
    },
    DoWhile {
        body: StmtId,
        test: ExprId,
    },
    While {
        test: ExprId,
        body: StmtId,
    },
    For {
        init: Option<ForInit>,
        test: Option<ExprId>,
        update: Option<ExprId>,
        body: StmtId,
    },
    /// `for (target in object) body` or `for (target of object) body`: one statement with its
    /// `iteration` kind, whose head declares, binds and assigns alike
    /// (`#sec-for-in-and-for-of-statements`).
    ForIn {
        target: ForInTarget,
        object: ExprId,
        body: StmtId,
        iteration: IterationKind,
    },
    /// `continue` with its `LabelIdentifier`, if any.
    Continue(Option<Name>),
    /// `break` with its `LabelIdentifier`, if any.
    Break(Option<Name>),
    Return(Option<ExprId>),
    Switch {
        discriminant: ExprId,
        cases: Vec<SwitchCase>,
    },
    Throw(ExprId),
    Try {
        block: Vec<StmtId>,
        handler: Option<CatchClause>,
        finalizer: Option<Vec<StmtId>>,
    },
    Debugger,
    FunctionDeclaration(FunctionId),
    /// `LabelIdentifier : LabelledItem` (`#sec-labelled-statements`); the item is a statement.
    Labelled {
        label: Name,
        body: StmtId,
    },
    /// `with ( Expression ) Statement` (`#sec-with-statement`), in non-strict code only.
    With {
        object: ExprId,
        body: StmtId,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct StmtNode {
    pub stmt: Stmt,
    pub span: Span,
    /// The longest path from this node to a leaf, counted in nodes, through functions.
    pub depth: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FunctionKind {
    Declaration,
    Expression,
    Getter,
    Setter,
    /// A plain method of an object literal (`#sec-method-definitions`): no own name binding,
    /// no [[Construct]].
    Method,
    /// An `ArrowFunction` (`#sec-arrow-function-definitions`): no name, no [[Construct]], no
    /// `this` and no `arguments` of its own ([[ThisMode]] lexical). Its parameters are the
    /// `ArrowParameters`, a `BindingIdentifier` or the `CoverParenthesizedExpressionAndArrowParameterList`
    /// read again as `ArrowFormalParameters`. A `ConciseBody` that is an `ExpressionBody` is
    /// held as a body of one `return` statement of that expression, whose span is the
    /// expression's: the value of the call is the expression's value
    /// (`#sec-arrow-function-definitions-runtime-semantics-evaluatebody`). Its span runs from
    /// the first token of its parameters to the end of its body.
    Arrow,
}

/// What scope analysis found for a function body or the script (`src/scope.rs`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Scope {
    /// `VarDeclaredNames` (`#sec-static-semantics-vardeclarednames`), each name once, in source
    /// order of its first declaration; function declarations of the body included.
    pub var_names: Vec<String>,
    /// The function declarations to instantiate, as GlobalDeclarationInstantiation and
    /// FunctionDeclarationInstantiation take them: the last declaration of each name, in source
    /// order.
    pub functions: Vec<FunctionId>,
    /// The names the `let` and `const` declarations at the top level of the body bind, in
    /// source order: the BoundNames of its LexicallyScopedDeclarations
    /// (`#sec-static-semantics-lexicallyscopeddeclarations`; a function declaration at the top
    /// level of a body is var scoped, and one in a block is a lexical name of that block,
    /// [`Ast::lexical_names`]).
    pub lexical: Vec<LexicalName>,
    /// The `IdentifierReference`s of the body itself (not of nested functions), each name once,
    /// in source order of its first reference.
    pub references: Vec<String>,
    /// Whether a function needs an arguments object (`#sec-functiondeclarationinstantiation`,
    /// steps 15 to 18; made only where its code can observe it, `docs/design/engine.md`,
    /// decision 3): its own code references `arguments` or holds a direct `eval` call (decision
    /// 3, "Names", item 4), or an arrow function in it does, at any depth of arrows, where no
    /// arrow between binds the name `arguments` itself ("`this` and `arguments` in an arrow
    /// function", item 2); and no parameter is named `arguments`, nor, when no parameter has an
    /// initializer, a function or a lexical declaration of the body (steps 17 and 18). Always
    /// false for the script and for an arrow function, which never has one (step 16).
    pub arguments_object: bool,
    /// Whether the body's own code (not a nested function's) holds a `with` statement, whose
    /// body's references are looked up by name (decision 3, "Names").
    pub contains_with: bool,
    /// Whether the body's own code (not a nested function's) holds a direct `eval` call: a call
    /// whose callee, its parentheses removed, is the IdentifierReference `eval`
    /// (`#sec-function-calls-runtime-semantics-evaluation`, step 6), not an optional call.
    /// The initializers of a function's parameters are its own code.
    pub contains_direct_eval: bool,
    /// Whether the initializers of a function's parameters hold a direct `eval` call
    /// themselves (not in a nested function): in non-strict code its `var` names go to a
    /// record outside the parameters' (`#sec-functiondeclarationinstantiation`, step 20).
    pub parameters_contain_direct_eval: bool,
}

/// A `FunctionDeclaration`, `FunctionExpression`, an accessor or method of an object literal, or
/// an `ArrowFunction`.
#[derive(Clone, Debug, PartialEq)]
pub struct Function {
    pub kind: FunctionKind,
    pub name: Option<Name>,
    /// The FormalParameters, one per parameter in order, each a `BindingIdentifier` or an
    /// ObjectBindingPattern, the rest parameter last (`#sec-function-definitions`).
    pub params: Vec<BindingTarget>,
    /// The Initializer of each parameter, beside its name in `params`: empty when no parameter
    /// has one, else as long as `params`, `None` for a parameter without one.
    pub initializers: Vec<Option<ExprId>>,
    /// Whether the last of `params` is a FunctionRestParameter.
    pub rest: bool,
    pub body: Vec<StmtId>,
    /// Whether the function's code is strict mode code (`#sec-strict-mode-code`): it is
    /// contained in strict code or its body starts with a Use Strict Directive.
    pub strict: bool,
    pub scope: Scope,
    pub span: Span,
    /// The longest path from this function to a leaf, counted in nodes.
    pub depth: u32,
}

impl Function {
    /// IsSimpleParameterList (`#sec-static-semantics-issimpleparameterlist`): no initializer,
    /// no rest parameter and no pattern.
    pub fn is_simple(&self) -> bool {
        self.initializers.is_empty()
            && !self.rest
            && self.params.iter().all(|p| p.identifier().is_some())
    }

    /// ContainsExpression of the FormalParameters
    /// (`#sec-static-semantics-containsexpression`): a parameter has an initializer, or a
    /// pattern has a default.
    pub fn has_parameter_expressions(&self) -> bool {
        !self.initializers.is_empty()
            || self
                .params
                .iter()
                .any(|p| matches!(p, BindingTarget::Pattern(p) if p.expressions))
    }

    /// The BoundNames of the FormalParameters, in source order.
    pub fn parameter_names(&self) -> impl Iterator<Item = &Name> {
        self.params.iter().flat_map(BindingTarget::names)
    }

    /// The Initializer of parameter `i`, if it has one.
    pub fn initializer(&self, i: usize) -> Option<ExprId> {
        self.initializers.get(i).copied().flatten()
    }

    /// ExpectedArgumentCount (`#sec-static-semantics-expectedargumentcount`): the parameters
    /// before the first one with an initializer or the rest parameter.
    pub fn expected_argument_count(&self) -> usize {
        let plain = if self.rest {
            self.params.len().saturating_sub(1)
        } else {
            self.params.len()
        };
        self.initializers
            .iter()
            .position(Option::is_some)
            .map_or(plain, |first| first.min(plain))
    }
}

/// The arenas of a tree.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Ast {
    pub(crate) exprs: Vec<ExprNode>,
    pub(crate) stmts: Vec<StmtNode>,
    pub(crate) functions: Vec<Function>,
}

/// A child of a node, for walks that go one level at a time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Child {
    Expr(ExprId),
    Stmt(StmtId),
    Function(FunctionId),
}

impl Ast {
    pub fn expr(&self, id: ExprId) -> Option<&ExprNode> {
        self.exprs.get(id.0 as usize)
    }

    pub fn stmt(&self, id: StmtId) -> Option<&StmtNode> {
        self.stmts.get(id.0 as usize)
    }

    pub fn function(&self, id: FunctionId) -> Option<&Function> {
        self.functions.get(id.0 as usize)
    }

    /// Every function, in source order.
    pub fn functions(&self) -> &[Function] {
        &self.functions
    }

    /// Whether a call (not an optional call) of `callee` is a direct `eval`, as far as the source
    /// tells (`#sec-function-calls-runtime-semantics-evaluation`, step 6): the callee, its
    /// parentheses removed, is the IdentifierReference `eval`. Whether the call then performs
    /// PerformEval depends on the value it calls, `%eval%` of the current realm, and is known
    /// only when it runs.
    pub fn is_direct_eval_callee(&self, callee: ExprId) -> bool {
        matches!(self.expr(callee).map(|n| &n.expr), Some(Expr::Identifier(name)) if name == "eval")
    }

    /// The number of children of the statement `id`: what a walk pushes for it.
    pub fn stmt_child_count(&self, id: StmtId) -> usize {
        let mut count = 0usize;
        if let Some(n) = self.stmt(id) {
            n.stmt.children(|_| count += 1);
        }
        count
    }

    /// The number of children of the expression `id`.
    pub fn expr_child_count(&self, id: ExprId) -> usize {
        let mut count = 0usize;
        if let Some(n) = self.expr(id) {
            n.expr.children(|_| count += 1);
        }
        count
    }

    /// [`Ast::push_children`] onto a stack whose growth is charged to `m` first: the children
    /// are counted, the room for them made, then they are pushed.
    pub(crate) fn push_children_charged<S>(
        &self,
        of: Child,
        stack: &mut Vec<S>,
        step: impl Fn(Child) -> S,
        m: &crate::ledger::Meter<'_>,
    ) -> Result<(), crate::ledger::Stop> {
        let mut count = 0usize;
        match of {
            Child::Expr(id) => {
                if let Some(n) = self.expr(id) {
                    n.expr.children(|_| count += 1);
                }
            }
            Child::Stmt(id) => {
                if let Some(n) = self.stmt(id) {
                    n.stmt.children(|_| count += 1);
                }
            }
            Child::Function(id) => {
                count = self.function(id).map_or(0, |f| {
                    f.initializers
                        .iter()
                        .flatten()
                        .count()
                        .saturating_add(f.params.iter().filter_map(BindingTarget::pattern).count())
                        .saturating_add(f.body.len())
                });
            }
        }
        m.grow_by(stack, count)?;
        self.push_children(of, stack, step);
        Ok(())
    }

    /// Pushes the children of the node `of` onto the work stack of a walk, each made a step by
    /// `step`, so that popping them gives them in source order. A walk that pops one step at a
    /// time keeps its work on the heap: its Rust stack does not grow with the depth of the tree.
    pub(crate) fn push_children<S>(
        &self,
        of: Child,
        stack: &mut Vec<S>,
        step: impl Fn(Child) -> S,
    ) {
        let mark = stack.len();
        match of {
            Child::Expr(id) => {
                if let Some(n) = self.expr(id) {
                    n.expr.children(|c| stack.push(step(c)));
                }
            }
            Child::Stmt(id) => {
                if let Some(n) = self.stmt(id) {
                    n.stmt.children(|c| stack.push(step(c)));
                }
            }
            Child::Function(id) => {
                if let Some(f) = self.function(id) {
                    for (i, p) in f.params.iter().enumerate() {
                        if let Some(e) = p.pattern() {
                            stack.push(step(Child::Expr(e)));
                        }
                        if let Some(e) = f.initializer(i) {
                            stack.push(step(Child::Expr(e)));
                        }
                    }
                    for s in &f.body {
                        stack.push(step(Child::Stmt(*s)));
                    }
                }
            }
        }
        if let Some(pushed) = stack.get_mut(mark..) {
            pushed.reverse();
        }
    }

    /// The depth of a child, 0 for an index that names nothing.
    pub(crate) fn depth(&self, child: Child) -> u32 {
        match child {
            Child::Expr(id) => self.expr(id).map_or(0, |n| n.depth),
            Child::Stmt(id) => self.stmt(id).map_or(0, |n| n.depth),
            Child::Function(id) => self.function(id).map_or(0, |f| f.depth),
        }
    }
}

// What each part of a tree owns on the heap, by capacity: the walk `Script::capacity_bytes`
// sums, and what a drop of a part credits.

impl HeapBytes for NumberLiteral {
    fn heap_bytes(&self) -> usize {
        self.text.heap_bytes()
    }
}

impl HeapBytes for StringLiteral {
    fn heap_bytes(&self) -> usize {
        self.value.heap_bytes()
    }
}

impl HeapBytes for RegExpLiteral {
    fn heap_bytes(&self) -> usize {
        self.body
            .heap_bytes()
            .saturating_add(self.flags.heap_bytes())
    }
}

impl HeapBytes for TemplateString {
    fn heap_bytes(&self) -> usize {
        self.cooked
            .heap_bytes()
            .saturating_add(self.raw.heap_bytes())
    }
}

impl HeapBytes for Name {
    fn heap_bytes(&self) -> usize {
        self.name.heap_bytes()
    }
}

impl HeapBytes for PropertyKey {
    fn heap_bytes(&self) -> usize {
        match self {
            PropertyKey::Identifier(name) => name.heap_bytes(),
            PropertyKey::String(s) => s.heap_bytes(),
            PropertyKey::Number(n) => n.heap_bytes(),
            PropertyKey::Computed(_) => 0,
        }
    }
}

impl HeapBytes for Property {
    fn heap_bytes(&self) -> usize {
        self.key.heap_bytes()
    }
}

impl HeapBytes for PatternTarget {
    fn heap_bytes(&self) -> usize {
        match self {
            PatternTarget::Binding(name) => name.heap_bytes(),
            PatternTarget::Expr(_) => 0,
        }
    }
}

impl HeapBytes for PatternProperty {
    fn heap_bytes(&self) -> usize {
        self.key
            .heap_bytes()
            .saturating_add(self.target.heap_bytes())
    }
}

impl HeapBytes for PatternElement {
    fn heap_bytes(&self) -> usize {
        self.target.heap_bytes()
    }
}

impl HeapBytes for BoundName {
    fn heap_bytes(&self) -> usize {
        self.name.heap_bytes()
    }
}

impl HeapBytes for BindingTarget {
    fn heap_bytes(&self) -> usize {
        match self {
            BindingTarget::Identifier(name) => name.heap_bytes(),
            BindingTarget::Pattern(p) => p.names.heap_bytes(),
        }
    }
}

impl HeapBytes for Expr {
    fn heap_bytes(&self) -> usize {
        match self {
            Expr::This
            | Expr::Null
            | Expr::Boolean(_)
            | Expr::Function(_)
            | Expr::Unary { .. }
            | Expr::Update { .. }
            | Expr::Binary { .. }
            | Expr::Logical { .. }
            | Expr::Conditional { .. }
            | Expr::Assign { .. }
            | Expr::Index { .. }
            | Expr::OptionalChain(_)
            | Expr::OptionalIndex { .. }
            | Expr::Spread(_) => 0,
            Expr::Identifier(name) => name.heap_bytes(),
            Expr::Number(n) => n.heap_bytes(),
            Expr::String(s) => s.heap_bytes(),
            Expr::RegExp(r) => r.heap_bytes(),
            Expr::Array(elements) => elements.heap_bytes(),
            Expr::Object(properties) => properties.heap_bytes(),
            Expr::Sequence(list) => list.heap_bytes(),
            Expr::Member { property, .. } | Expr::OptionalMember { property, .. } => {
                property.heap_bytes()
            }
            Expr::Call { arguments, .. } | Expr::OptionalCall { arguments, .. } => {
                arguments.heap_bytes()
            }
            Expr::New { arguments, .. } => arguments.heap_bytes(),
            Expr::Template {
                strings,
                substitutions,
            } => strings
                .heap_bytes()
                .saturating_add(substitutions.heap_bytes()),
            Expr::ObjectPattern { properties, .. } => properties.heap_bytes(),
            Expr::ArrayPattern { elements, .. } => elements.heap_bytes(),
        }
    }
}

impl HeapBytes for ExprNode {
    fn heap_bytes(&self) -> usize {
        self.expr.heap_bytes()
    }
}

impl HeapBytes for VarDeclarator {
    fn heap_bytes(&self) -> usize {
        self.target.heap_bytes()
    }
}

impl HeapBytes for ForInit {
    fn heap_bytes(&self) -> usize {
        match self {
            ForInit::Var(_, declarators) => declarators.heap_bytes(),
            ForInit::Expression(_) => 0,
        }
    }
}

impl HeapBytes for ForInTarget {
    fn heap_bytes(&self) -> usize {
        match self {
            ForInTarget::Var(_, target) => target.heap_bytes(),
            ForInTarget::Expression(_) => 0,
        }
    }
}

impl HeapBytes for SwitchCase {
    fn heap_bytes(&self) -> usize {
        self.body.heap_bytes()
    }
}

impl HeapBytes for CatchClause {
    fn heap_bytes(&self) -> usize {
        self.param
            .heap_bytes()
            .saturating_add(self.body.heap_bytes())
    }
}

impl HeapBytes for Stmt {
    fn heap_bytes(&self) -> usize {
        match self {
            Stmt::Empty
            | Stmt::Expression(_)
            | Stmt::If { .. }
            | Stmt::DoWhile { .. }
            | Stmt::While { .. }
            | Stmt::Return(_)
            | Stmt::Throw(_)
            | Stmt::Debugger
            | Stmt::FunctionDeclaration(_)
            | Stmt::With { .. } => 0,
            Stmt::Block(body) => body.heap_bytes(),
            Stmt::Var(_, declarators) => declarators.heap_bytes(),
            Stmt::For { init, .. } => init.heap_bytes(),
            Stmt::ForIn { target, .. } => target.heap_bytes(),
            Stmt::Continue(label) | Stmt::Break(label) => label.heap_bytes(),
            Stmt::Switch { cases, .. } => cases.heap_bytes(),
            Stmt::Try {
                block,
                handler,
                finalizer,
            } => block
                .heap_bytes()
                .saturating_add(handler.heap_bytes())
                .saturating_add(finalizer.heap_bytes()),
            Stmt::Labelled { label, .. } => label.heap_bytes(),
        }
    }
}

impl HeapBytes for StmtNode {
    fn heap_bytes(&self) -> usize {
        self.stmt.heap_bytes()
    }
}

impl HeapBytes for LexicalName {
    fn heap_bytes(&self) -> usize {
        self.name.heap_bytes()
    }
}

impl HeapBytes for Scope {
    fn heap_bytes(&self) -> usize {
        self.var_names
            .heap_bytes()
            .saturating_add(self.functions.heap_bytes())
            .saturating_add(self.lexical.heap_bytes())
            .saturating_add(self.references.heap_bytes())
    }
}

impl HeapBytes for Function {
    fn heap_bytes(&self) -> usize {
        self.name
            .heap_bytes()
            .saturating_add(self.params.heap_bytes())
            .saturating_add(self.initializers.heap_bytes())
            .saturating_add(self.body.heap_bytes())
            .saturating_add(self.scope.heap_bytes())
    }
}

impl HeapBytes for Ast {
    /// The three arenas by capacity and what their nodes own.
    fn heap_bytes(&self) -> usize {
        self.exprs
            .heap_bytes()
            .saturating_add(self.stmts.heap_bytes())
            .saturating_add(self.functions.heap_bytes())
    }
}

impl Expr {
    /// Calls `f` on each child of the expression, in source order.
    pub(crate) fn children(&self, mut f: impl FnMut(Child)) {
        match self {
            Expr::This
            | Expr::Identifier(_)
            | Expr::Null
            | Expr::Boolean(_)
            | Expr::Number(_)
            | Expr::String(_)
            | Expr::RegExp(_) => {}
            Expr::Array(elements) => {
                for e in elements.iter().flatten() {
                    f(Child::Expr(*e));
                }
            }
            Expr::Object(properties) => {
                for p in properties {
                    if let PropertyKey::Computed(key) = p.key {
                        f(Child::Expr(key));
                    }
                    match p.value {
                        PropertyValue::Init(e)
                        | PropertyValue::Shorthand(e)
                        | PropertyValue::Spread(e) => f(Child::Expr(e)),
                        PropertyValue::CoverInitialized(name, init) => {
                            f(Child::Expr(name));
                            f(Child::Expr(init));
                        }
                        PropertyValue::Get(id)
                        | PropertyValue::Set(id)
                        | PropertyValue::Method(id) => f(Child::Function(id)),
                    }
                }
            }
            Expr::ObjectPattern { properties, .. } => {
                for p in properties {
                    if let PropertyKey::Computed(key) = p.key {
                        f(Child::Expr(key));
                    }
                    if let PatternTarget::Expr(e) = p.target {
                        f(Child::Expr(e));
                    }
                    if let Some(e) = p.default {
                        f(Child::Expr(e));
                    }
                }
            }
            Expr::ArrayPattern { elements, .. } => {
                for p in elements.iter().flatten() {
                    if let PatternTarget::Expr(e) = p.target {
                        f(Child::Expr(e));
                    }
                    if let Some(e) = p.default {
                        f(Child::Expr(e));
                    }
                }
            }
            Expr::Spread(e) => f(Child::Expr(*e)),
            Expr::Function(id) => f(Child::Function(*id)),
            Expr::Unary { operand, .. } | Expr::Update { operand, .. } => f(Child::Expr(*operand)),
            Expr::Binary { left, right, .. } | Expr::Logical { left, right, .. } => {
                f(Child::Expr(*left));
                f(Child::Expr(*right));
            }
            Expr::Conditional {
                test,
                consequent,
                alternate,
            } => {
                f(Child::Expr(*test));
                f(Child::Expr(*consequent));
                f(Child::Expr(*alternate));
            }
            Expr::Assign { target, value, .. } => {
                f(Child::Expr(*target));
                f(Child::Expr(*value));
            }
            Expr::Sequence(list) => {
                for e in list {
                    f(Child::Expr(*e));
                }
            }
            Expr::Member { object, .. } | Expr::OptionalMember { object, .. } => {
                f(Child::Expr(*object))
            }
            Expr::OptionalChain(e) => f(Child::Expr(*e)),
            Expr::Index { object, index } | Expr::OptionalIndex { object, index } => {
                f(Child::Expr(*object));
                f(Child::Expr(*index));
            }
            Expr::Call { callee, arguments } | Expr::OptionalCall { callee, arguments } => {
                f(Child::Expr(*callee));
                for e in arguments {
                    f(Child::Expr(*e));
                }
            }
            Expr::New { callee, arguments } => {
                f(Child::Expr(*callee));
                for e in arguments.iter().flatten() {
                    f(Child::Expr(*e));
                }
            }
            Expr::Template { substitutions, .. } => {
                for e in substitutions {
                    f(Child::Expr(*e));
                }
            }
        }
    }
}

impl Stmt {
    /// Calls `f` on each child of the statement, in source order.
    pub(crate) fn children(&self, mut f: impl FnMut(Child)) {
        let list = |list: &[StmtId], f: &mut dyn FnMut(Child)| {
            for s in list {
                f(Child::Stmt(*s));
            }
        };
        match self {
            Stmt::Block(body) => list(body, &mut f),
            Stmt::Var(_, declarators) => {
                for d in declarators {
                    if let Some(e) = d.target.pattern() {
                        f(Child::Expr(e));
                    }
                    if let Some(e) = d.init {
                        f(Child::Expr(e));
                    }
                }
            }
            Stmt::Empty | Stmt::Continue(_) | Stmt::Break(_) | Stmt::Debugger => {}
            Stmt::Labelled { body, .. } => f(Child::Stmt(*body)),
            Stmt::Expression(e) | Stmt::Throw(e) => f(Child::Expr(*e)),
            Stmt::If {
                test,
                consequent,
                alternate,
            } => {
                f(Child::Expr(*test));
                f(Child::Stmt(*consequent));
                if let Some(s) = alternate {
                    f(Child::Stmt(*s));
                }
            }
            Stmt::DoWhile { body, test } => {
                f(Child::Stmt(*body));
                f(Child::Expr(*test));
            }
            Stmt::While { test, body } => {
                f(Child::Expr(*test));
                f(Child::Stmt(*body));
            }
            Stmt::For {
                init,
                test,
                update,
                body,
            } => {
                match init {
                    Some(ForInit::Var(_, declarators)) => {
                        for d in declarators {
                            if let Some(e) = d.target.pattern() {
                                f(Child::Expr(e));
                            }
                            if let Some(e) = d.init {
                                f(Child::Expr(e));
                            }
                        }
                    }
                    Some(ForInit::Expression(e)) => f(Child::Expr(*e)),
                    None => {}
                }
                if let Some(e) = test {
                    f(Child::Expr(*e));
                }
                if let Some(e) = update {
                    f(Child::Expr(*e));
                }
                f(Child::Stmt(*body));
            }
            Stmt::ForIn {
                target,
                object,
                body,
                ..
            } => {
                match target {
                    ForInTarget::Expression(e) => f(Child::Expr(*e)),
                    ForInTarget::Var(_, t) => {
                        if let Some(e) = t.pattern() {
                            f(Child::Expr(e));
                        }
                    }
                }
                f(Child::Expr(*object));
                f(Child::Stmt(*body));
            }
            Stmt::Return(e) => {
                if let Some(e) = e {
                    f(Child::Expr(*e));
                }
            }
            Stmt::Switch {
                discriminant,
                cases,
            } => {
                f(Child::Expr(*discriminant));
                for case in cases {
                    if let Some(e) = case.test {
                        f(Child::Expr(e));
                    }
                    list(&case.body, &mut f);
                }
            }
            Stmt::Try {
                block,
                handler,
                finalizer,
            } => {
                list(block, &mut f);
                if let Some(h) = handler {
                    if let Some(e) = h.param.as_ref().and_then(BindingTarget::pattern) {
                        f(Child::Expr(e));
                    }
                    list(&h.body, &mut f);
                }
                if let Some(body) = finalizer {
                    list(body, &mut f);
                }
            }
            Stmt::FunctionDeclaration(id) => f(Child::Function(*id)),
            Stmt::With { object, body } => {
                f(Child::Expr(*object));
                f(Child::Stmt(*body));
            }
        }
    }
}
