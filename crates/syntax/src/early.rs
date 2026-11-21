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
//! - `#sec-break-statement-static-semantics-early-errors` and
//!   `#sec-continue-statement-static-semantics-early-errors`: `break` outside a loop or `switch`,
//!   `continue` outside a loop, neither crossing a function;
//! - `#sec-object-initializer-static-semantics-early-errors`: `__proto__` twice as a
//!   `PropertyName : AssignmentExpression`;
//! - `#sec-primary-expression-regular-expression-literals-static-semantics-early-errors`: the
//!   flags of IsValidRegularExpressionLiteral. Its pattern is not parsed before slice 5 (a
//!   recorded omission of `sec-literals-regular-expression-literals`).
//!
//! The walk recurses once per node level, so it is bounded by the node depth the parser bounds.

use std::collections::BTreeSet;

use crate::ast::{
    Ast, Child, Expr, ExprId, ForInTarget, ForInit, FunctionId, Name, NumberForm, NumberLiteral,
    PropertyKey, PropertyValue, RegExpLiteral, Span, Stmt, StmtId, StringLiteral, UnaryOp,
};
use crate::error::SyntaxError;

/// The message of a `with` statement in strict mode code.
pub(crate) const WITH_IN_STRICT: &str = "with statement in strict mode code";

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
pub(crate) fn check(ast: &Ast, body: &[StmtId], strict: bool, max: u32) -> Vec<SyntaxError> {
    let mut checker = Checker {
        ast,
        errors: Vec::new(),
        strict,
        max,
        depth: 0,
        loops: 0,
        breakables: 0,
    };
    for s in body {
        checker.stmt(*s);
    }
    checker.errors
}

struct Checker<'a> {
    ast: &'a Ast,
    errors: Vec<SyntaxError>,
    strict: bool,
    max: u32,
    depth: u32,
    /// Loops around the current position inside the current function.
    loops: u32,
    /// Loops and `switch` statements around the current position inside the current function.
    breakables: u32,
}

impl<'a> Checker<'a> {
    fn error(&mut self, message: impl Into<String>, span: Span) {
        self.errors
            .push(SyntaxError::new(message, span.line, span.column));
    }

    /// One level deeper; false when that passes the limit (the parser bounds node depth, so this
    /// only holds the walk to the same bound).
    fn enter(&mut self, span: Span) -> bool {
        self.depth += 1;
        if self.depth > self.max {
            self.errors
                .push(SyntaxError::nesting(self.max, span.line, span.column));
            self.depth -= 1;
            return false;
        }
        true
    }

    fn leave(&mut self) {
        self.depth = self.depth.saturating_sub(1);
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

    fn child(&mut self, c: Child) {
        match c {
            Child::Expr(e) => self.expr(e),
            Child::Stmt(s) => self.stmt(s),
            Child::Function(f) => self.function(f),
        }
    }

    /// Walks a statement. The checks of the node itself are in [`Checker::check_stmt`], so the
    /// frame that stays on the stack while the children are walked is small.
    fn stmt(&mut self, id: StmtId) {
        let Some(node) = self.ast.stmt(id) else {
            return;
        };
        if !self.enter(node.span) {
            return;
        }
        let (loops, breakables) = self.check_stmt(id);
        self.loops += loops;
        self.breakables += breakables;
        for c in self.ast.stmt_children(id) {
            self.child(c);
        }
        self.loops -= loops;
        self.breakables -= breakables;
        self.leave();
    }

    /// The early errors of the statement `id` itself; returns how many loops and how many
    /// breakable statements it opens for its children (the tests of a loop hold no `break` or
    /// `continue` that is not inside a function of their own).
    fn check_stmt(&mut self, id: StmtId) -> (u32, u32) {
        let Some(node) = self.ast.stmt(id) else {
            return (0, 0);
        };
        match &node.stmt {
            Stmt::Var(declarators)
            | Stmt::For {
                init: Some(ForInit::Var(declarators)),
                ..
            } => {
                for d in declarators {
                    self.binding(&d.name);
                }
            }
            Stmt::ForIn { target, .. } => match target {
                ForInTarget::Var(name) => self.binding(name),
                ForInTarget::Expression(e) => self.target(*e),
            },
            Stmt::Continue if self.loops == 0 => {
                self.error("continue outside of a loop", node.span);
            }
            Stmt::Break if self.breakables == 0 => {
                self.error("break outside of a loop or switch", node.span);
            }
            Stmt::Try {
                handler: Some(h), ..
            } => self.binding(&h.param),
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

    /// Walks an expression; its own checks are in [`Checker::check_expr`].
    fn expr(&mut self, id: ExprId) {
        let Some(node) = self.ast.expr(id) else {
            return;
        };
        if !self.enter(node.span) {
            return;
        }
        self.check_expr(id);
        for c in self.ast.expr_children(id) {
            self.child(c);
        }
        self.leave();
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

    /// Walks a function with its own strictness, outside every loop and `switch`.
    fn function(&mut self, id: FunctionId) {
        let Some(f) = self.ast.function(id) else {
            return;
        };
        if !self.enter(f.span) {
            return;
        }
        let saved = (self.strict, self.loops, self.breakables);
        self.strict = f.strict;
        self.loops = 0;
        self.breakables = 0;
        self.check_function(id);
        for s in &f.body {
            self.stmt(*s);
        }
        (self.strict, self.loops, self.breakables) = saved;
        self.leave();
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
            if !seen.insert(p.name.as_str()) && self.strict {
                self.error(
                    format!("duplicate parameter name {} in strict mode code", p.name),
                    p.span,
                );
            }
        }
    }
}
