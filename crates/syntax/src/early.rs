//! The early errors of a parsed script (`#sec-scripts-static-semantics-early-errors` and the
//! early errors of the productions in clauses 12 to 15 that slice 1 parses), strict ones
//! included, checked in one walk over the tree once every function's strictness is known:
//!
//! - `#sec-identifiers-static-semantics-early-errors`: in strict mode code `eval` and `arguments`
//!   are no `BindingIdentifier`, and `implements`, `interface`, `let`, `package`, `private`,
//!   `protected`, `public`, `static` and `yield` are no identifier;
//! - `#sec-static-semantics-assignmenttargettype` with `#sec-assignment-operators-static-semantics-early-errors`,
//!   `#sec-update-expressions-static-semantics-early-errors` and
//!   `#sec-for-in-and-for-of-statements-static-semantics-early-errors`: an assignment target is
//!   an identifier (not `eval` or `arguments` in strict mode code) or a property access. A call
//!   is not: swell does not take the normative optional "Runtime Errors for Function Call
//!   Assignment Targets";
//! - `#sec-delete-operator-static-semantics-early-errors`: `delete` of an identifier, in
//!   parentheses or not, in strict mode code;
//! - `#sec-parameter-lists-static-semantics-early-errors` through
//!   `#sec-function-definitions-static-semantics-early-errors`: duplicate parameters of a strict
//!   function, and the function's own name and parameters checked with its own strictness;
//! - `#sec-numeric-literals-early-errors` and `#sec-string-literals-early-errors`: legacy octal
//!   literals, `NonOctalDecimalIntegerLiteral`s, legacy octal escapes and `\8` and `\9` in strict
//!   mode code;
//! - `#sec-with-statement-static-semantics-early-errors`: `with` in strict mode code (the parser
//!   reports it, with [`WITH_IN_STRICT`], as it reads the statement);
//! - `#sec-break-statement-static-semantics-early-errors`,
//!   `#sec-continue-statement-static-semantics-early-errors` and
//!   `#sec-labelled-statements-static-semantics-early-errors`: `break` outside a loop or `switch`,
//!   `continue` outside a loop, a label that no enclosing statement has (ContainsUndefinedBreakTarget,
//!   ContainsUndefinedContinueTarget), a `continue` whose label is on no iteration statement, a
//!   label inside a statement with the same label (ContainsDuplicateLabels), none crossing a
//!   function; in strict mode code a label is no reserved word either;
//! - `#sec-object-initializer-static-semantics-early-errors`: `__proto__` twice as a
//!   `PropertyName : AssignmentExpression`;
//! - `#sec-method-definitions-static-semantics-early-errors`: the `UniqueFormalParameters` of a
//!   method, in any code;
//! - `#sec-primary-expression-regular-expression-literals-static-semantics-early-errors`: the
//!   flags of IsValidRegularExpressionLiteral. Its pattern is not parsed before slice 5 (a
//!   recorded omission of `sec-literals-regular-expression-literals`);
//! - `#sec-let-and-const-declarations-static-semantics-early-errors`: a `let` or `const`
//!   declaration binds no name `let` and no name twice, and a `const` one has an initializer
//!   outside a for-in head;
//! - the LexicallyDeclaredNames of a script, a function body, a block and a `case` block, and
//!   the names a `for` or for-in head declares with `let` or `const`, hold no duplicate and no
//!   name of the VarDeclaredNames around them (`#sec-scripts-static-semantics-early-errors`,
//!   `#sec-function-definitions-static-semantics-early-errors`,
//!   `#sec-block-static-semantics-early-errors`, `#sec-switch-statement-static-semantics-early-errors`,
//!   `#sec-for-statement-static-semantics-early-errors`,
//!   `#sec-for-in-and-for-of-statements-static-semantics-early-errors`); those of a function body
//!   no parameter name, those of a `catch` block not its parameter
//!   (`#sec-try-statement-static-semantics-early-errors`).
//!
//! The lexical names in scope are a stack of frames, each made as the walk enters the statement
//! list (or the head) that declares them; a `var` or a top-level function declaration is checked
//! against the frames of its own function. The errors are sorted into source order at the end.
//!
//! The parser reports the early errors it meets as it reads: a name written with an escape that
//! spells a reserved word ([`escaped_keyword`], `#sec-identifiers-static-semantics-early-errors`)
//! and `??` mixed with `&&` or `||` ([`COALESCE_MIXED`]).
//!
//! The walk does not recurse: its pending work is a stack on the heap, so the Rust stack it uses
//! is the same at any depth of the tree.

use std::collections::BTreeSet;

use crate::ast::{
    Ast, Child, DeclKind, Expr, ExprId, ForInTarget, ForInit, FunctionId, FunctionKind, Name,
    NumberForm, NumberLiteral, PropertyKey, PropertyValue, RegExpLiteral, Span, Stmt, StmtId,
    StringLiteral, UnaryOp,
};
use crate::error::SyntaxError;

/// The message of a `with` statement in strict mode code.
pub(crate) const WITH_IN_STRICT: &str = "with statement in strict mode code";

/// The message of a `??` with an `&&` or `||` operand, or the other way round, outside
/// parentheses (`#sec-binary-logical-operators`).
pub(crate) const COALESCE_MIXED: &str = "?? mixed with && or || without parentheses";

/// The message of an `IdentifierName` written with an escape whose StringValue is the reserved
/// word `name`, used as an identifier or where the keyword stands
/// (`#sec-identifiers-static-semantics-early-errors`, `#sec-keywords-and-reserved-words`).
pub(crate) fn escaped_keyword(name: &str) -> String {
    format!("keyword {name} must not contain escaped characters")
}

/// The identifiers strict mode code reserves (`#sec-identifiers-static-semantics-early-errors`).
const STRICT_RESERVED: [&str; 9] = [
    "implements",
    "interface",
    "let",
    "package",
    "private",
    "protected",
    "public",
    "static",
    "yield",
];

/// The early errors of the script `body` of `ast`, strict when `strict`, in source order.
pub(crate) fn check(ast: &Ast, body: &[StmtId], strict: bool) -> Vec<SyntaxError> {
    let mut checker = Checker {
        ast,
        errors: Vec::new(),
        strict,
        loops: 0,
        breakables: 0,
        labels: Vec::new(),
        label_base: 0,
        lexical: Vec::new(),
        lexical_base: 0,
    };
    let frame = checker.frame(lexical_of(ast, body), &[], "");
    checker.lexical.push(frame);
    checker.walk(body);
    let mut errors = checker.errors;
    errors.sort_by_key(|e| (e.line, e.column));
    errors
}

/// The names the `let` and `const` declarations of the statement list `list` bind.
fn lexical_of(ast: &Ast, list: &[StmtId]) -> Vec<Name> {
    ast.lexical_names(list)
        .into_iter()
        .map(|l| l.name)
        .collect()
}

/// Pushes the statements `list` with the frame of lexical names around them, so that popping
/// gives the frame's entry, the statements in source order and the frame's end.
fn push_list(stack: &mut Vec<Step>, list: &[StmtId], frame: BTreeSet<String>) {
    stack.push(Step::PopLexical);
    stack.extend(list.iter().rev().map(|s| Step::Node(Child::Stmt(*s))));
    stack.push(Step::Enter(frame));
}

/// One piece of the walk's pending work.
enum Step {
    Node(Child),
    /// Puts back the context a statement or a function changed, once its children are walked.
    Restore {
        strict: bool,
        loops: u32,
        breakables: u32,
        label_base: usize,
        lexical_base: usize,
    },
    /// Drops the label of a labelled statement, once its body is walked.
    PopLabel,
    /// Enters the lexical names of a `catch` or `finally` block, made as the `try` was entered.
    Enter(BTreeSet<String>),
    /// Drops the innermost frame of lexical names, once the statements it covers are walked.
    PopLexical,
}

struct Checker<'a> {
    ast: &'a Ast,
    errors: Vec<SyntaxError>,
    strict: bool,
    /// Loops around the current position inside the current function.
    loops: u32,
    /// Loops and `switch` statements around the current position inside the current function.
    breakables: u32,
    /// The labels around the current position, each with whether it labels an iteration
    /// statement (through other labels); those from `label_base` on are the current function's.
    labels: Vec<(String, bool)>,
    label_base: usize,
    /// The frames of lexical names around the current position, innermost last; those from
    /// `lexical_base` on are the current function's.
    lexical: Vec<BTreeSet<String>>,
    lexical_base: usize,
}

impl<'a> Checker<'a> {
    /// A frame of the lexical names `names`, in source order: a name twice is an error, and so
    /// is one of `others` (the parameters or the catch parameter, named by `what`).
    fn frame(&mut self, names: Vec<Name>, others: &[Name], what: &str) -> BTreeSet<String> {
        let mut frame = BTreeSet::new();
        for n in names {
            if others.iter().any(|o| o.name == n.name) {
                self.error(
                    format!("lexical declaration of {} conflicts with {what}", n.name),
                    n.span,
                );
            }
            if !frame.insert(n.name.clone()) {
                self.error(
                    format!("duplicate lexical declaration of {}", n.name),
                    n.span,
                );
            }
        }
        frame
    }

    /// A name a `var` or a top-level function declaration binds: no lexical declaration of the
    /// current function around it may bind it too.
    fn var_name(&mut self, name: &Name) {
        let clash = self
            .lexical
            .get(self.lexical_base..)
            .unwrap_or(&[])
            .iter()
            .any(|f| f.contains(&name.name));
        if clash {
            self.error(
                format!(
                    "lexical declaration of {} conflicts with a var or function declaration",
                    name.name
                ),
                name.span,
            );
        }
    }

    /// A name a declaration of kind `kind` binds.
    fn declared(&mut self, kind: DeclKind, name: &Name) {
        if kind.is_lexical() && name.name == "let" {
            self.error("let is not a valid lexically bound name", name.span);
            return;
        }
        self.binding(name);
        if kind == DeclKind::Var {
            self.var_name(name);
        }
    }

    /// Enters the lexical names the statement `id` declares for its children: those of a block,
    /// a `case` block, or a `for` or for-in head. A `try` pushes its children itself, a frame
    /// around each of its blocks; then this answers true.
    fn lexical_scope(&mut self, id: StmtId, stack: &mut Vec<Step>) -> bool {
        let ast = self.ast;
        let Some(node) = ast.stmt(id) else {
            return false;
        };
        let names = match &node.stmt {
            Stmt::Block(body) => lexical_of(ast, body),
            Stmt::Switch { cases, .. } => cases
                .iter()
                .flat_map(|c| lexical_of(ast, &c.body))
                .collect(),
            Stmt::For {
                init: Some(ForInit::Var(kind, declarators)),
                ..
            } if kind.is_lexical() => declarators.iter().map(|d| d.name.clone()).collect(),
            Stmt::ForIn {
                target: ForInTarget::Var(kind, name),
                ..
            } if kind.is_lexical() => vec![name.clone()],
            Stmt::Try {
                block,
                handler,
                finalizer,
            } => {
                let block_frame = self.frame(lexical_of(ast, block), &[], "");
                if let Some(list) = finalizer {
                    let frame = self.frame(lexical_of(ast, list), &[], "");
                    push_list(stack, list, frame);
                }
                if let Some(h) = handler {
                    let param: &[Name] = h.param.as_slice();
                    let frame = self.frame(lexical_of(ast, &h.body), param, "the catch parameter");
                    push_list(stack, &h.body, frame);
                }
                stack.push(Step::PopLexical);
                stack.extend(block.iter().rev().map(|s| Step::Node(Child::Stmt(*s))));
                self.lexical.push(block_frame);
                return true;
            }
            _ => return false,
        };
        let frame = self.frame(names, &[], "");
        stack.push(Step::PopLexical);
        self.lexical.push(frame);
        false
    }

    fn error(&mut self, message: impl Into<String>, span: Span) {
        self.errors
            .push(SyntaxError::new(message, span.line, span.column));
    }

    /// A `BindingIdentifier` in code of the current strictness.
    fn binding(&mut self, name: &Name) {
        if !self.strict {
            return;
        }
        if name.name == "eval" || name.name == "arguments" {
            self.error(
                format!("{} may not be bound in strict mode code", name.name),
                name.span,
            );
        } else if STRICT_RESERVED.contains(&name.name.as_str()) {
            self.reserved(&name.name, name.span);
        }
    }

    fn reserved(&mut self, name: &str, span: Span) {
        self.error(
            format!("{name} is a reserved word in strict mode code"),
            span,
        );
    }

    /// The target of an assignment, an update or a for-in head
    /// (`#sec-static-semantics-assignmenttargettype`).
    fn target(&mut self, e: ExprId) {
        let Some(node) = self.ast.expr(e) else {
            return;
        };
        match &node.expr {
            Expr::Identifier(n) => {
                if self.strict && (n == "eval" || n == "arguments") {
                    self.error(
                        format!("{n} may not be assigned in strict mode code"),
                        node.span,
                    );
                }
            }
            Expr::Member { .. } | Expr::Index { .. } => {}
            _ => self.error("invalid assignment target", node.span),
        }
    }

    fn number(&mut self, n: &NumberLiteral, span: Span) {
        if !self.strict {
            return;
        }
        match n.form {
            NumberForm::LegacyOctal => self.error("legacy octal literal in strict mode code", span),
            NumberForm::NonOctalDecimal => {
                self.error(
                    "decimal literal with a leading zero in strict mode code",
                    span,
                );
            }
            _ => {}
        }
    }

    fn string(&mut self, s: &StringLiteral, span: Span) {
        if !self.strict {
            return;
        }
        if s.legacy_octal_escape {
            self.error("legacy octal escape sequence in strict mode code", span);
        } else if s.non_octal_decimal_escape {
            self.error("\\8 or \\9 escape sequence in strict mode code", span);
        }
    }

    /// The flags of IsValidRegularExpressionLiteral
    /// (`#sec-primary-expression-regular-expression-literals-static-semantics-early-errors`):
    /// only `d g i m s u v y`, each once, and not `u` with `v` (ParsePattern refuses that pair).
    fn regexp(&mut self, r: &RegExpLiteral, span: Span) {
        let mut seen = Vec::new();
        let mut valid = true;
        for &f in &r.flags {
            if !b"dgimsuvy".iter().any(|&c| u16::from(c) == f) || seen.contains(&f) {
                valid = false;
            }
            seen.push(f);
        }
        if seen.contains(&u16::from(b'u')) && seen.contains(&u16::from(b'v')) {
            valid = false;
        }
        if !valid {
            self.error("invalid regular expression flags", span);
        }
    }

    fn property_key(&mut self, key: &PropertyKey, span: Span) {
        match key {
            PropertyKey::Identifier(_) => {}
            PropertyKey::String(s) => self.string(s, span),
            PropertyKey::Number(n) => self.number(n, span),
        }
    }

    fn is_proto(key: &PropertyKey) -> bool {
        match key {
            PropertyKey::Identifier(n) => n == "__proto__",
            PropertyKey::String(s) => s.value.iter().copied().eq("__proto__".encode_utf16()),
            PropertyKey::Number(_) => false,
        }
    }

    /// Walks the statements `body` and everything in them, each node before its children and the
    /// children in source order, so the errors come in source order.
    fn walk(&mut self, body: &[StmtId]) {
        let mut stack: Vec<Step> = body
            .iter()
            .rev()
            .map(|s| Step::Node(Child::Stmt(*s)))
            .collect();
        while let Some(step) = stack.pop() {
            let node = match step {
                Step::Node(node) => node,
                Step::Restore {
                    strict,
                    loops,
                    breakables,
                    label_base,
                    lexical_base,
                } => {
                    (self.strict, self.loops, self.breakables) = (strict, loops, breakables);
                    (self.label_base, self.lexical_base) = (label_base, lexical_base);
                    continue;
                }
                Step::PopLabel => {
                    self.labels.pop();
                    continue;
                }
                Step::Enter(frame) => {
                    self.lexical.push(frame);
                    continue;
                }
                Step::PopLexical => {
                    self.lexical.pop();
                    continue;
                }
            };
            let restore = Step::Restore {
                strict: self.strict,
                loops: self.loops,
                breakables: self.breakables,
                label_base: self.label_base,
                lexical_base: self.lexical_base,
            };
            match node {
                Child::Stmt(id) => {
                    let (loops, breakables) = self.check_stmt(id);
                    if loops + breakables > 0 {
                        stack.push(restore);
                        self.loops += loops;
                        self.breakables += breakables;
                    }
                    if self.label(id) {
                        stack.push(Step::PopLabel);
                    }
                    if self.lexical_scope(id, &mut stack) {
                        continue;
                    }
                }
                Child::Expr(id) => self.check_expr(id),
                // A function is walked with its own strictness, outside every loop, `switch` and
                // label, with the lexical names of its body as its only frame.
                Child::Function(id) => {
                    let Some(f) = self.ast.function(id) else {
                        continue;
                    };
                    stack.push(restore);
                    self.strict = f.strict;
                    self.loops = 0;
                    self.breakables = 0;
                    self.label_base = self.labels.len();
                    self.check_function(id);
                    self.lexical_base = self.lexical.len();
                    let frame = self.frame(lexical_of(self.ast, &f.body), &f.params, "a parameter");
                    stack.push(Step::PopLexical);
                    self.lexical.push(frame);
                }
            }
            self.ast.push_children(node, &mut stack, Step::Node);
        }
    }

    /// The early errors of the statement `id` itself; returns how many loops and how many
    /// breakable statements it opens for its children (the tests of a loop hold no `break` or
    /// `continue` that is not inside a function of their own).
    fn check_stmt(&mut self, id: StmtId) -> (u32, u32) {
        let Some(node) = self.ast.stmt(id) else {
            return (0, 0);
        };
        match &node.stmt {
            Stmt::Var(kind, declarators)
            | Stmt::For {
                init: Some(ForInit::Var(kind, declarators)),
                ..
            } => {
                for d in declarators {
                    self.declared(*kind, &d.name);
                    if *kind == DeclKind::Const && d.init.is_none() {
                        self.error("missing initializer in a const declaration", d.name.span);
                    }
                }
            }
            Stmt::ForIn { target, .. } => match target {
                ForInTarget::Var(kind, name) => self.declared(*kind, name),
                ForInTarget::Expression(e) => self.target(*e),
            },
            // A function declaration reaches the walk only at the top level of a body, where it
            // is var scoped.
            Stmt::FunctionDeclaration(f) => {
                if let Some(name) = self.ast.function(*f).and_then(|x| x.name.as_ref()) {
                    self.var_name(name);
                }
            }
            Stmt::Continue(None) if self.loops == 0 => {
                self.error("continue outside of a loop", node.span);
            }
            Stmt::Break(None) if self.breakables == 0 => {
                self.error("break outside of a loop or switch", node.span);
            }
            Stmt::Continue(Some(label)) => match self.find_label(&label.name) {
                None => self.error(
                    format!("continue to undefined label {}", label.name),
                    label.span,
                ),
                Some(false) => self.error(
                    format!(
                        "continue to label {}, which labels no iteration statement",
                        label.name
                    ),
                    label.span,
                ),
                Some(true) => {}
            },
            Stmt::Break(Some(label)) => {
                if self.find_label(&label.name).is_none() {
                    self.error(
                        format!("break to undefined label {}", label.name),
                        label.span,
                    );
                }
            }
            Stmt::Try {
                handler: Some(h), ..
            } => {
                if let Some(param) = &h.param {
                    self.binding(param);
                }
            }
            _ => {}
        }
        match node.stmt {
            Stmt::DoWhile { .. } | Stmt::While { .. } | Stmt::For { .. } | Stmt::ForIn { .. } => {
                (1, 1)
            }
            Stmt::Switch { .. } => (0, 1),
            _ => (0, 0),
        }
    }

    /// Whether the label `name` is around the current position in the current function, and if
    /// so whether it labels an iteration statement.
    fn find_label(&self, name: &str) -> Option<bool> {
        self.labels
            .get(self.label_base..)
            .unwrap_or(&[])
            .iter()
            .rev()
            .find(|(l, _)| l == name)
            .map(|(_, is_loop)| *is_loop)
    }

    /// When `id` is a labelled statement: checks its label and pushes it for the statement's
    /// body; returns whether it pushed one.
    fn label(&mut self, id: StmtId) -> bool {
        let Some(Stmt::Labelled { label, body }) = self.ast.stmt(id).map(|n| &n.stmt) else {
            return false;
        };
        if self.strict && STRICT_RESERVED.contains(&label.name.as_str()) {
            self.reserved(&label.name, label.span);
        }
        if self.find_label(&label.name).is_some() {
            self.error(format!("duplicate label {}", label.name), label.span);
        }
        let mut item = *body;
        while let Some(Stmt::Labelled { body, .. }) = self.ast.stmt(item).map(|n| &n.stmt) {
            item = *body;
        }
        let is_loop = matches!(
            self.ast.stmt(item).map(|n| &n.stmt),
            Some(Stmt::DoWhile { .. } | Stmt::While { .. } | Stmt::For { .. } | Stmt::ForIn { .. })
        );
        self.labels.push((label.name.clone(), is_loop));
        true
    }

    /// The early errors of the expression `id` itself.
    fn check_expr(&mut self, id: ExprId) {
        let Some(node) = self.ast.expr(id) else {
            return;
        };
        let span = node.span;
        match &node.expr {
            Expr::Identifier(n) => {
                if self.strict && STRICT_RESERVED.contains(&n.as_str()) {
                    self.reserved(n, span);
                }
            }
            Expr::Number(n) => self.number(n, span),
            Expr::String(s) => self.string(s, span),
            Expr::RegExp(r) => self.regexp(r, span),
            Expr::Object(properties) => {
                let mut protos = 0;
                for p in properties {
                    self.property_key(&p.key, p.span);
                    if matches!(p.value, PropertyValue::Init(_)) && Checker::is_proto(&p.key) {
                        protos += 1;
                        if protos == 2 {
                            self.error("duplicate __proto__ property in an object literal", p.span);
                        }
                    }
                }
            }
            Expr::Unary {
                op: UnaryOp::Delete,
                operand,
            } => {
                if self.strict
                    && matches!(
                        self.ast.expr(*operand).map(|n| &n.expr),
                        Some(Expr::Identifier(_))
                    )
                {
                    self.error(
                        "delete of an unqualified identifier in strict mode code",
                        span,
                    );
                }
            }
            Expr::Update { operand, .. } => self.target(*operand),
            Expr::Assign { target, .. } => self.target(*target),
            _ => {}
        }
    }

    /// The function's name and parameters, checked with its own strictness.
    fn check_function(&mut self, id: FunctionId) {
        let Some(f) = self.ast.function(id) else {
            return;
        };
        if let Some(name) = &f.name {
            self.binding(name);
        }
        let mut seen = BTreeSet::new();
        for p in &f.params {
            self.binding(p);
            if seen.insert(p.name.as_str()) {
                continue;
            }
            if f.kind == FunctionKind::Method {
                self.error(
                    format!("duplicate parameter name {} in a method", p.name),
                    p.span,
                );
            } else if self.strict {
                self.error(
                    format!("duplicate parameter name {} in strict mode code", p.name),
                    p.span,
                );
            }
        }
    }
}
