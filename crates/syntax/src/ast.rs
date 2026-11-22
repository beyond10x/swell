//! The syntax tree of a script: the productions of clauses 13 (`#sec-ecmascript-language-expressions`),
//! 14 (`#sec-ecmascript-language-statements-and-declarations`), 15.2 (`#sec-function-definitions`)
//! and 15.4 (`#sec-method-definitions`: accessors and plain methods) that slices 1 and 2 parse.
//!
//! The tree lives in three arenas, one per kind of node, and a node names its children by index.
//! So dropping a tree is no recursion at all, and a walk over it recurses once per level, a depth
//! the parser bounds by `parser_nesting` (every node records its `depth`, the length of the
//! longest path from it to a leaf, and the parser refuses a node deeper than the limit).
//! Every node records its source position.

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

/// A function of the script, numbered in the order its first token appears.
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
}

/// A `PropertyDefinition` of an object literal (`#sec-object-initializer`).
#[derive(Clone, Debug, PartialEq)]
pub struct Property {
    pub key: PropertyKey,
    pub value: PropertyValue,
    pub span: Span,
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

/// A `VariableDeclaration`.
#[derive(Clone, Debug, PartialEq)]
pub struct VarDeclarator {
    pub name: Name,
    pub init: Option<ExprId>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ForInit {
    Var(Vec<VarDeclarator>),
    Expression(ExprId),
}

/// The left side of `for (... in ...)`.
#[derive(Clone, Debug, PartialEq)]
pub enum ForInTarget {
    Var(Name),
    Expression(ExprId),
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
    pub param: Option<Name>,
    pub body: Vec<StmtId>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Stmt {
    Block(Vec<StmtId>),
    Var(Vec<VarDeclarator>),
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
    ForIn {
        target: ForInTarget,
        object: ExprId,
        body: StmtId,
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
    /// The `IdentifierReference`s of the body itself (not of nested functions), each name once,
    /// in source order of its first reference.
    pub references: Vec<String>,
    /// Whether a function needs an arguments object (`#sec-functiondeclarationinstantiation`,
    /// steps 15 to 18; made only where its code can observe it, `docs/design/engine.md`,
    /// decision 3): its own code references `arguments` or holds a direct `eval` call (decision
    /// 3, "Names", item 4), and neither a parameter nor a function it declares is named
    /// `arguments`. Always false for the script.
    pub arguments_object: bool,
    /// Whether the body's own code (not a nested function's) holds a `with` statement, whose
    /// body's references are looked up by name (decision 3, "Names").
    pub contains_with: bool,
    /// Whether the body's own code (not a nested function's) holds a direct `eval` call: a call
    /// whose callee, its parentheses removed, is the IdentifierReference `eval`
    /// (`#sec-function-calls-runtime-semantics-evaluation`, step 6), not an optional call.
    pub contains_direct_eval: bool,
}

/// A `FunctionDeclaration`, `FunctionExpression`, or an accessor or method of an object literal.
#[derive(Clone, Debug, PartialEq)]
pub struct Function {
    pub kind: FunctionKind,
    pub name: Option<Name>,
    pub params: Vec<Name>,
    pub body: Vec<StmtId>,
    /// Whether the function's code is strict mode code (`#sec-strict-mode-code`): it is
    /// contained in strict code or its body starts with a Use Strict Directive.
    pub strict: bool,
    pub scope: Scope,
    pub span: Span,
    /// The longest path from this function to a leaf, counted in nodes.
    pub depth: u32,
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
                    match p.value {
                        PropertyValue::Init(e) | PropertyValue::Shorthand(e) => f(Child::Expr(e)),
                        PropertyValue::Get(id)
                        | PropertyValue::Set(id)
                        | PropertyValue::Method(id) => f(Child::Function(id)),
                    }
                }
            }
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
            Stmt::Var(declarators) => {
                for d in declarators {
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
                    Some(ForInit::Var(declarators)) => {
                        for d in declarators {
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
            } => {
                if let ForInTarget::Expression(e) = target {
                    f(Child::Expr(*e));
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
