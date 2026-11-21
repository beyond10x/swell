//! Scope analysis.
//!
//! - The Use Strict Directive (`#sec-directive-prologues-and-the-use-strict-directive`): the
//!   exact code units `"use strict"` or `'use strict'`, with no escape and no line continuation.
//!   The parser finds the directive prologue of each body and asks [`is_use_strict`] of each of
//!   its string literals; a function's code is strict when the code around it is or when its own
//!   prologue holds the directive (`#sec-strict-mode-code`).
//! - For the script and each function: `VarDeclaredNames`
//!   (`#sec-scripts-static-semantics-vardeclarednames`,
//!   `#sec-function-definitions-static-semantics-vardeclarednames`), the function declarations to
//!   instantiate (the last one of each name, in source order, as GlobalDeclarationInstantiation
//!   and FunctionDeclarationInstantiation take them from the var scoped declarations), and the
//!   identifiers its own code references.
//!
//! The walk recurses once per node level, so it is bounded by the node depth the parser bounds.

use std::collections::BTreeSet;

use crate::ast::{
    Ast, Child, Expr, ExprId, ForInTarget, ForInit, FunctionId, Scope, Span, Stmt, StmtId,
};
use crate::error::SyntaxError;

/// Whether `raw`, the source text of a string literal in a directive prologue, is a Use Strict
/// Directive.
pub(crate) fn is_use_strict(raw: &[u16]) -> bool {
    let double = "\"use strict\"".encode_utf16();
    let single = "'use strict'".encode_utf16();
    raw.iter().copied().eq(double) || raw.iter().copied().eq(single)
}

/// Analyses the script `body` and every function of `ast`; stores each function's scope in it
/// and returns the script's.
pub(crate) fn analyse(ast: &mut Ast, body: &[StmtId], max: u32) -> Result<Scope, SyntaxError> {
    let mut analyser = Analyser {
        ast: &*ast,
        scopes: Vec::new(),
        max,
        depth: 0,
        error: None,
    };
    let script = analyser.body(body);
    let Analyser { scopes, error, .. } = analyser;
    if let Some(e) = error {
        return Err(e);
    }
    for (id, scope) in scopes {
        if let Some(f) = ast.functions.get_mut(id.0 as usize) {
            f.scope = scope;
        }
    }
    Ok(script)
}

/// The names one body collects.
#[derive(Default)]
struct Collect {
    var_names: Vec<String>,
    var_seen: BTreeSet<String>,
    declarations: Vec<(String, FunctionId)>,
    references: Vec<String>,
    reference_seen: BTreeSet<String>,
}

impl Collect {
    fn var(&mut self, name: &str) {
        if self.var_seen.insert(name.to_string()) {
            self.var_names.push(name.to_string());
        }
    }

    fn reference(&mut self, name: &str) {
        if self.reference_seen.insert(name.to_string()) {
            self.references.push(name.to_string());
        }
    }
}

struct Analyser<'a> {
    ast: &'a Ast,
    scopes: Vec<(FunctionId, Scope)>,
    max: u32,
    depth: u32,
    error: Option<SyntaxError>,
}

impl<'a> Analyser<'a> {
    fn enter(&mut self, span: Span) -> bool {
        self.depth += 1;
        if self.depth > self.max {
            if self.error.is_none() {
                self.error = Some(SyntaxError::nesting(self.max, span.line, span.column));
            }
            self.depth -= 1;
            return false;
        }
        true
    }

    fn leave(&mut self) {
        self.depth = self.depth.saturating_sub(1);
    }

    /// The scope of a script or function body.
    fn body(&mut self, body: &[StmtId]) -> Scope {
        let mut c = Collect::default();
        for s in body {
            // A function declaration at the top level of the body is var scoped
            // (`TopLevelVarDeclaredNames`).
            if let Some(Stmt::FunctionDeclaration(f)) = self.ast.stmt(*s).map(|n| &n.stmt)
                && let Some(name) = self.ast.function(*f).and_then(|x| x.name.as_ref())
            {
                c.var(&name.name);
                c.declarations.push((name.name.clone(), *f));
            }
            self.stmt(*s, &mut c);
        }
        // The last declaration of each name, in source order.
        let mut taken = BTreeSet::new();
        let mut functions = Vec::new();
        for (name, id) in c.declarations.iter().rev() {
            if taken.insert(name.as_str()) {
                functions.push(*id);
            }
        }
        functions.reverse();
        Scope {
            var_names: c.var_names,
            functions,
            references: c.references,
        }
    }

    fn child(&mut self, x: Child, c: &mut Collect) {
        match x {
            Child::Expr(e) => self.expr(e, c),
            Child::Stmt(s) => self.stmt(s, c),
            Child::Function(f) => self.function(f),
        }
    }

    /// Walks a statement; the names it declares itself are collected by
    /// [`Analyser::collect_stmt`].
    fn stmt(&mut self, id: StmtId, c: &mut Collect) {
        let Some(node) = self.ast.stmt(id) else {
            return;
        };
        if !self.enter(node.span) {
            return;
        }
        self.collect_stmt(id, c);
        for x in self.ast.stmt_children(id) {
            self.child(x, c);
        }
        self.leave();
    }

    /// The `var` names the statement `id` declares itself (`VarDeclaredNames` of a variable
    /// statement, a `for` with `var` and a for-in with `var`).
    fn collect_stmt(&mut self, id: StmtId, c: &mut Collect) {
        match self.ast.stmt(id).map(|n| &n.stmt) {
            Some(
                Stmt::Var(declarators)
                | Stmt::For {
                    init: Some(ForInit::Var(declarators)),
                    ..
                },
            ) => {
                for d in declarators {
                    c.var(&d.name.name);
                }
            }
            Some(Stmt::ForIn {
                target: ForInTarget::Var(name),
                ..
            }) => c.var(&name.name),
            _ => {}
        }
    }

    /// Walks an expression; an identifier is a reference of the body being analysed.
    fn expr(&mut self, id: ExprId, c: &mut Collect) {
        let Some(node) = self.ast.expr(id) else {
            return;
        };
        if !self.enter(node.span) {
            return;
        }
        if let Expr::Identifier(n) = &node.expr {
            c.reference(n);
        }
        for x in self.ast.expr_children(id) {
            self.child(x, c);
        }
        self.leave();
    }

    /// Analyses a function on its own; its scope is stored when the walk ends.
    fn function(&mut self, id: FunctionId) {
        let Some(f) = self.ast.function(id) else {
            return;
        };
        if !self.enter(f.span) {
            return;
        }
        let scope = self.body(&f.body);
        self.scopes.push((id, scope));
        self.leave();
    }
}
