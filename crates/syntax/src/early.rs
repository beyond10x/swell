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
//! The walk does not recurse: its pending work is a stack on the heap, so the Rust stack it uses
//! is the same at any depth of the tree.

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
pub(crate) fn check(ast: &Ast, body: &[StmtId], strict: bool) -> Vec<SyntaxError> {
    let mut checker = Checker {
        ast,
        errors: Vec::new(),
        strict,
        loops: 0,
        breakables: 0,
    };
    checker.walk(body);
    checker.errors
}

/// One piece of the walk's pending work.
enum Step {
    Node(Child),
    /// Puts back the context a statement or a function changed, once its children are walked.
    Restore {
        strict: bool,
        loops: u32,
        breakables: u32,
    },
}

struct Checker<'a> {
    ast: &'a Ast,
    errors: Vec<SyntaxError>,
    strict: bool,
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
                } => {
                    (self.strict, self.loops, self.breakables) = (strict, loops, breakables);
                    continue;
                }
            };
            let restore = Step::Restore {
                strict: self.strict,
                loops: self.loops,
                breakables: self.breakables,
            };
            match node {
                Child::Stmt(id) => {
                    let (loops, breakables) = self.check_stmt(id);
                    if loops + breakables > 0 {
                        stack.push(restore);
                        self.loops += loops;
                        self.breakables += breakables;
                    }
                }
                Child::Expr(id) => self.check_expr(id),
                // A function is walked with its own strictness, outside every loop and `switch`.
                Child::Function(id) => {
                    let Some(f) = self.ast.function(id) else {
                        continue;
                    };
                    stack.push(restore);
                    self.strict = f.strict;
                    self.loops = 0;
                    self.breakables = 0;
                    self.check_function(id);
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
            if !seen.insert(p.name.as_str()) && self.strict {
                self.error(
                    format!("duplicate parameter name {} in strict mode code", p.name),
                    p.span,
                );
            }
        }
    }
}
